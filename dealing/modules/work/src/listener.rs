
use bytes::{Buf, Bytes, BytesMut};
use clap::Parser;
use shared_util::cp::CpPrice;
use tokio::net::{TcpStream, TcpListener};

use std::fmt;
use std::sync::Arc;
use std::future::Future;
use std::num::TryFromIntError;
use std::string::FromUtf8Error;
use std::io::{self, Cursor};
use tokio::time::{self, Duration};
use tokio::signal;
use tokio::io::{AsyncReadExt, AsyncWriteExt, BufWriter};
use tokio::sync::{broadcast, mpsc, Semaphore};


#[cfg(feature = "otel")]
use tracing_subscriber::{
    fmt, layer::SubscriberExt, util::SubscriberInitExt, util::TryInitError, EnvFilter,
};
use tracing::{debug, error, info, instrument};

pub type Error = Box<dyn std::error::Error + Send + Sync>;
pub type MyResult<T> = std::result::Result<T, Error>;

pub const DEFAULT_PORT: u16 = 6379;
pub const MAX_CONNECTIONS: usize = 3;

#[tokio::main]
async fn main() -> MyResult<()>{
    tracing_subscriber::fmt::try_init();

    let cli = Cli::parse();
    let port = cli.port.unwrap_or(DEFAULT_PORT);
    let listener = TcpListener::bind(&format!("127.0.0.1:{}", port)).await?;
    run(listener, signal::ctrl_c()).await;

    Ok(())
}

pub async fn run(listener: TcpListener, shutdown: impl Future) {

    let (notify_shutdown, _) = broadcast::channel(1);
    let (shutdown_complete_tx, shutdown_complete_rx) = mpsc::channel(1);

    let mut server = Listener {
        listener,
        limit_connections: Arc::new(Semaphore::new(MAX_CONNECTIONS)),
        notify_shutdown,
        shutdown_complete_tx,
        shutdown_complete_rx,
    };

    tokio::select! {
        res = server.run() => {
            if let Err(err) = res {
                error!(cause = %err, "failed to accept");
            }
        }
        _ = shutdown => {
            // The shutdown signal has been received.
            info!("shutting down");
        }
    }


}

#[derive(Parser, Debug)]
#[clap(name = "dealing-simulator", version, author = "", about = "")]
struct Cli {
    #[clap(long)]
    port: Option<u16>,
}

#[derive(Debug)]
struct Listener {
    listener: TcpListener,
    limit_connections: Arc<Semaphore>,
    notify_shutdown: broadcast::Sender<()>,
    shutdown_complete_rx: mpsc::Receiver<()>,
    shutdown_complete_tx: mpsc::Sender<()>,

}


impl Listener {
    async fn run(&mut self) -> MyResult<()> {
        info!("accepting inbound connections");

        loop {
            let permit = self
                .limit_connections
                .clone()
                .acquire_owned()
                .await
                .unwrap();
            
            let socket = self.accept().await?;

            let mut handler = Handler {
                connection: Connection::new(socket),

                // Receive shutdown notifications.
                shutdown: Shutdown::new(self.notify_shutdown.subscribe()),

                // Notifies the receiver half once all clones are
                // dropped.
                _shutdown_complete: self.shutdown_complete_tx.clone(),
            };

            // Spawn a new task to process the connections. Tokio tasks are like
            // asynchronous green threads and are executed concurrently.
            tokio::spawn(async move {
                // Process the connection. If an error is encountered, log it.
                if let Err(err) = handler.run().await {
                    error!(cause = ?err, "connection error");
                    println!("err"); 
                }
                println!("drop");
                // Move the permit into the task and drop it after completion.
                // This returns the permit back to the semaphore.
                drop(permit);
            });
        }
    }

    async fn accept(&mut self) -> MyResult<TcpStream> {
        let mut backoff = 1;

        // Try to accept a few times
        loop {
            // Perform the accept operation. If a socket is successfully
            // accepted, return it. Otherwise, save the error.
            match self.listener.accept().await {
                Ok((socket, _)) => return Ok(socket),
                Err(err) => {
                    if backoff > 64 {
                        // Accept has failed too many times. Return the error.
                        return Err(err.into());
                    }
                }
            }

            // Pause execution until the back off period elapses.
            time::sleep(Duration::from_secs(backoff)).await;

            // Double the back off
            backoff *= 2;
        }
    }
}



#[derive(Debug)]
struct Handler {
    connection: Connection,
    shutdown: Shutdown,
    _shutdown_complete: mpsc::Sender<()>,
}

impl Handler {

    #[instrument(skip(self))]
    async fn run(&mut self) -> MyResult<()> {
        // As long as the shutdown signal has not been received, try to read a
        // new request frame.
        while !self.shutdown.is_shutdown() {
            // While reading a request frame, also listen for the shutdown
            // signal.
            let maybe_frame = tokio::select! {
                res = self.connection.read_frame() => res?,
                _ = self.shutdown.recv() => {
                    // If a shutdown signal is received, return from `run`.
                    // This will result in the task terminating.
                    return Ok(());
                }
            };

            // If `None` is returned from `read_frame()` then the peer closed
            // the socket. There is no further work to do and the task can be
            // terminated.
            let frame = match maybe_frame {
                Some(frame) => frame,
                None => return Ok(()),
            };

            println!("frame:[{:?}]", frame);

            // // Convert the redis frame into a command struct. This returns an
            // // error if the frame is not a valid redis command or it is an
            // // unsupported command.
            // let cmd = Command::from_frame(frame)?;

            // // Logs the `cmd` object. The syntax here is a shorthand provided by
            // // the `tracing` crate. It can be thought of as similar to:
            // //
            // // ```
            // // debug!(cmd = format!("{:?}", cmd));
            // // ```
            // //
            // // `tracing` provides structured logging, so information is "logged"
            // // as key-value pairs.
            // debug!(?cmd);

            // // Perform the work needed to apply the command. This may mutate the
            // // database state as a result.
            // //
            // // The connection is passed into the apply function which allows the
            // // command to write response frames directly to the connection. In
            // // the case of pub/sub, multiple frames may be send back to the
            // // peer.
            // cmd.apply(&self.db, &mut self.connection, &mut self.shutdown)
            //     .await?;
        }

        Ok(())
    }
}


#[derive(Debug)]
pub struct Connection {
    stream: BufWriter<TcpStream>,
    buffer: BytesMut,
}

impl Connection {
    pub fn new(socket: TcpStream) -> Connection {
        Connection {
            stream: BufWriter::new(socket),
            buffer: BytesMut::with_capacity(4 * 1024),
        }
    }

    pub async fn read_frame(&mut self) -> MyResult<Option<Frame>> {
        loop {
            if let Some(frame) = self.parse_frame()? {
                return Ok(Some(frame));
            }

            // On success, the number of bytes is returned. `0` indicates "end
            // of stream".
            if 0 == self.stream.read_buf(&mut self.buffer).await? {
                // The remote closed the connection. For this to be a clean
                // shutdown, there should be no data in the read buffer. If
                // there is, this means that the peer closed the socket while
                // sending a frame.
                if self.buffer.is_empty() {
                    return Ok(None);
                } else {
                    println!("connection reset");
                    return Err("connection reset by peer".into());
                }
            }
        }
    }

    fn parse_frame(&mut self) -> MyResult<Option<Frame>> {

        let mut buf = Cursor::new(&self.buffer[..]);

        match Frame::check(&mut buf) {
            Ok(_) => {
                let frame = Frame::parse(&mut buf)?;
                let len = buf.position() as usize;
                // buf.set_position(0);
                self.buffer.advance(len);
                // Return the parsed frame to the caller.
                Ok(Some(frame))
                
                // match Frame::parse(&mut buf) {


                //     Ok(frame) => {
                //         let len = buf.position() as usize;
                //         buf.set_position(0);
                //         self.buffer.advance(len);
                //         // Return the parsed frame to the caller.
                //         Ok(Some(frame))
                //     },
                //     _ => Ok(None)
                // }

            }
            // There is not enough data present in the read buffer to parse a
            // single frame. We must wait for more data to be received from the
            // socket. Reading from the socket will be done in the statement
            // after this `match`.
            //
            // We do not want to return `Err` from here as this "error" is an
            // expected runtime condition.
            Err(FrameError::Incomplete) => Ok(None),
            // An error was encountered while parsing the frame. The connection
            // is now in an invalid state. Returning `Err` from here will result
            // in the connection being closed.
            Err(e) => {
                println!("err[{}]", e);
                Err(e.into())
            },
        }
    }

    // /// Write a single `Frame` value to the underlying stream.
    // ///
    // /// The `Frame` value is written to the socket using the various `write_*`
    // /// functions provided by `AsyncWrite`. Calling these functions directly on
    // /// a `TcpStream` is **not** advised, as this will result in a large number of
    // /// syscalls. However, it is fine to call these functions on a *buffered*
    // /// write stream. The data will be written to the buffer. Once the buffer is
    // /// full, it is flushed to the underlying socket.
    // pub async fn write_frame(&mut self, frame: &Frame) -> io::Result<()> {
    //     // Arrays are encoded by encoding each entry. All other frame types are
    //     // considered literals. For now, mini-redis is not able to encode
    //     // recursive frame structures. See below for more details.
    //     match frame {
    //         Frame::Array(val) => {
    //             // Encode the frame type prefix. For an array, it is `*`.
    //             self.stream.write_u8(b'*').await?;

    //             // Encode the length of the array.
    //             self.write_decimal(val.len() as u64).await?;

    //             // Iterate and encode each entry in the array.
    //             for entry in &**val {
    //                 self.write_value(entry).await?;
    //             }
    //         }
    //         // The frame type is a literal. Encode the value directly.
    //         _ => self.write_value(frame).await?,
    //     }

    //     // Ensure the encoded frame is written to the socket. The calls above
    //     // are to the buffered stream and writes. Calling `flush` writes the
    //     // remaining contents of the buffer to the socket.
    //     self.stream.flush().await
    // }

    // /// Write a frame literal to the stream
    // async fn write_value(&mut self, frame: &Frame) -> io::Result<()> {
    //     match frame {
    //         Frame::Simple(val) => {
    //             self.stream.write_u8(b'+').await?;
    //             self.stream.write_all(val.as_bytes()).await?;
    //             self.stream.write_all(b"\r\n").await?;
    //         }
    //         Frame::Error(val) => {
    //             self.stream.write_u8(b'-').await?;
    //             self.stream.write_all(val.as_bytes()).await?;
    //             self.stream.write_all(b"\r\n").await?;
    //         }
    //         Frame::Integer(val) => {
    //             self.stream.write_u8(b':').await?;
    //             self.write_decimal(*val).await?;
    //         }
    //         Frame::Null => {
    //             self.stream.write_all(b"$-1\r\n").await?;
    //         }
    //         Frame::Bulk(val) => {
    //             let len = val.len();

    //             self.stream.write_u8(b'$').await?;
    //             self.write_decimal(len as u64).await?;
    //             self.stream.write_all(val).await?;
    //             self.stream.write_all(b"\r\n").await?;
    //         }
    //         // Encoding an `Array` from within a value cannot be done using a
    //         // recursive strategy. In general, async fns do not support
    //         // recursion. Mini-redis has not needed to encode nested arrays yet,
    //         // so for now it is skipped.
    //         Frame::Array(_val) => unreachable!(),
    //     }

    //     Ok(())
    // }

    // /// Write a decimal frame to the stream
    // async fn write_decimal(&mut self, val: u64) -> io::Result<()> {
    //     use std::io::Write;

    //     // Convert the value to a string
    //     let mut buf = [0u8; 20];
    //     let mut buf = Cursor::new(&mut buf[..]);
    //     write!(&mut buf, "{}", val)?;

    //     let pos = buf.position() as usize;
    //     self.stream.write_all(&buf.get_ref()[..pos]).await?;
    //     self.stream.write_all(b"\r\n").await?;

    //     Ok(())
    // }
}


#[derive(Debug)]
pub(crate) struct Shutdown {
    shutdown: bool,
    notify: broadcast::Receiver<()>,
}

impl Shutdown {
    pub(crate) fn new(notify: broadcast::Receiver<()>) -> Shutdown {
        Shutdown {
            shutdown: false,
            notify,
        }
    }

    pub(crate) fn is_shutdown(&self) -> bool {
        self.shutdown
    }

    pub(crate) async fn recv(&mut self) {
        if self.shutdown {
            return;
        }

        let _ = self.notify.recv().await;

        self.shutdown = true;
    }
}


#[derive(Clone, Debug)]
pub enum Frame {
    Simple(String),
    Error(String),
    Integer(u64),
    Null,
    Array(Vec<Frame>),
}

#[derive(Debug)]
pub enum FrameError {
    /// Not enough data is available to parse a message
    Incomplete,

    /// Invalid message encoding
    Other(crate::Error),
}

impl Frame {
    /// Returns an empty array
    pub(crate) fn array() -> Frame {
        Frame::Array(vec![])
    }

    // /// Push a "bulk" frame into the array. `self` must be an Array frame.
    // ///
    // /// # Panics
    // ///
    // /// panics if `self` is not an array
    // pub(crate) fn push_bulk(&mut self, bytes: Bytes) {
    //     match self {
    //         Frame::Array(vec) => {
    //             vec.push(Frame::Bulk(bytes));
    //         }
    //         _ => panic!("not an array frame"),
    //     }
    // }

    // /// Push an "integer" frame into the array. `self` must be an Array frame.
    // ///
    // /// # Panics
    // ///
    // /// panics if `self` is not an array
    // pub(crate) fn push_int(&mut self, value: u64) {
    //     match self {
    //         Frame::Array(vec) => {
    //             vec.push(Frame::Integer(value));
    //         }
    //         _ => panic!("not an array frame"),
    //     }
    // }

    // Checks if an entire message can be decoded from `src`
    pub fn check(src: &mut Cursor<&[u8]>) -> Result<(), FrameError> {
        let ret = get_u8(src).map(|_u8| ());
        src.set_position(0);
        ret
    }

    /// The message has already been validated with `check`.
    pub fn parse(src: &mut Cursor<&[u8]>) -> Result<Frame, FrameError> {
        let line = get_line(src)?.to_vec();
        // Convert the line to a String
        let string = String::from_utf8(line).unwrap();
        Ok(Frame::Simple(string))
    }

    // /// Converts the frame to an "unexpected frame" error
    // pub(crate) fn to_error(&self) -> crate::Error {
    //     format!("unexpected frame: {}", self).into()
    // }
}

impl PartialEq<&str> for Frame {
    fn eq(&self, other: &&str) -> bool {
        match self {
            Frame::Simple(s) => s.eq(other),
            // Frame::Bulk(s) => s.eq(other),
            _ => false,
        }
    }
}

impl fmt::Display for Frame {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        use std::str;

        match self {
            Frame::Simple(response) => response.fmt(fmt),
            Frame::Error(msg) => write!(fmt, "error: {}", msg),
            Frame::Integer(num) => num.fmt(fmt),
            // Frame::Bulk(msg) => match str::from_utf8(msg) {
            //     Ok(string) => string.fmt(fmt),
            //     Err(_) => write!(fmt, "{:?}", msg),
            // },
            Frame::Null => "(nil)".fmt(fmt),
            Frame::Array(parts) => {
                for (i, part) in parts.iter().enumerate() {
                    if i > 0 {
                        write!(fmt, " ")?;
                        part.fmt(fmt)?;
                    }
                }

                Ok(())
            }
        }
    }
}

impl From<String> for FrameError {
    fn from(src: String) -> FrameError {
        FrameError::Other(src.into())
    }
}

impl From<&str> for FrameError {
    fn from(src: &str) -> FrameError {
        src.to_string().into()
    }
}

impl From<FromUtf8Error> for FrameError {
    fn from(_src: FromUtf8Error) -> FrameError {
        "protocol error; invalid frame format".into()
    }
}

impl From<TryFromIntError> for FrameError {
    fn from(_src: TryFromIntError) -> FrameError {
        "protocol error; invalid frame format".into()
    }
}

impl std::error::Error for FrameError {}

impl fmt::Display for FrameError {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        match self {
            FrameError::Incomplete => "stream ended early".fmt(fmt),
            FrameError::Other(err) => err.fmt(fmt),
        }
    }
}


/// Find a line
fn get_line<'a>(src: &mut Cursor<&'a [u8]>) -> Result<&'a [u8], FrameError> {
    println!("getline");
    // Scan the bytes directly
    let start = src.position() as usize;
    // if src.get_ref().len() == 0 {
    //     return Err(FrameError::Incomplete);
    // }

    // Scan to the second to last byte
    let end = src.get_ref().len() - 1;

    for i in start..end {
        if src.get_ref()[i] == b'\r' && src.get_ref()[i + 1] == b'\n' {
            // We found a line, update the position to be *after* the \n
            src.set_position((i + 2) as u64);

            // Return the line
            return Ok(&src.get_ref()[start..i]);
        }
    }
    println!("incomplete");

    Err(FrameError::Incomplete)
}

fn get_u8(src: &mut Cursor<&[u8]>) -> Result<u8, FrameError> {
    if !src.has_remaining() {
        return Err(FrameError::Incomplete);
    }

    Ok(src.get_u8())
}

// #[derive(Debug)]
// pub enum Command {
//     Get(Get),
//     Publish(Publish),
//     Set(Set),
//     Subscribe(Subscribe),
//     Unsubscribe(Unsubscribe),
//     Ping(Ping),
//     Unknown(Unknown),
// }

// impl Command {
//     /// Parse a command from a received frame.
//     ///
//     /// The `Frame` must represent a Redis command supported by `mini-redis` and
//     /// be the array variant.
//     ///
//     /// # Returns
//     ///
//     /// On success, the command value is returned, otherwise, `Err` is returned.
//     pub fn from_frame(frame: Frame) -> crate::Result<Command> {
//         // The frame  value is decorated with `Parse`. `Parse` provides a
//         // "cursor" like API which makes parsing the command easier.
//         //
//         // The frame value must be an array variant. Any other frame variants
//         // result in an error being returned.
//         let mut parse = Parse::new(frame)?;

//         // All redis commands begin with the command name as a string. The name
//         // is read and converted to lower cases in order to do case sensitive
//         // matching.
//         let command_name = parse.next_string()?.to_lowercase();

//         // Match the command name, delegating the rest of the parsing to the
//         // specific command.
//         let command = match &command_name[..] {
//             "get" => Command::Get(Get::parse_frames(&mut parse)?),
//             "publish" => Command::Publish(Publish::parse_frames(&mut parse)?),
//             "set" => Command::Set(Set::parse_frames(&mut parse)?),
//             "subscribe" => Command::Subscribe(Subscribe::parse_frames(&mut parse)?),
//             "unsubscribe" => Command::Unsubscribe(Unsubscribe::parse_frames(&mut parse)?),
//             "ping" => Command::Ping(Ping::parse_frames(&mut parse)?),
//             _ => {
//                 // The command is not recognized and an Unknown command is
//                 // returned.
//                 //
//                 // `return` is called here to skip the `finish()` call below. As
//                 // the command is not recognized, there is most likely
//                 // unconsumed fields remaining in the `Parse` instance.
//                 return Ok(Command::Unknown(Unknown::new(command_name)));
//             }
//         };

//         // Check if there is any remaining unconsumed fields in the `Parse`
//         // value. If fields remain, this indicates an unexpected frame format
//         // and an error is returned.
//         parse.finish()?;

//         // The command has been successfully parsed
//         Ok(command)
//     }

//     /// Apply the command to the specified `Db` instance.
//     ///
//     /// The response is written to `dst`. This is called by the server in order
//     /// to execute a received command.
//     pub(crate) async fn apply(
//         self,
//         db: &Db,
//         dst: &mut Connection,
//         shutdown: &mut Shutdown,
//     ) -> crate::Result<()> {
//         use Command::*;

//         match self {
//             Get(cmd) => cmd.apply(db, dst).await,
//             Publish(cmd) => cmd.apply(db, dst).await,
//             Set(cmd) => cmd.apply(db, dst).await,
//             Subscribe(cmd) => cmd.apply(db, dst, shutdown).await,
//             Ping(cmd) => cmd.apply(dst).await,
//             Unknown(cmd) => cmd.apply(dst).await,
//             // `Unsubscribe` cannot be applied. It may only be received from the
//             // context of a `Subscribe` command.
//             Unsubscribe(_) => Err("`Unsubscribe` is unsupported in this context".into()),
//         }
//     }

//     /// Returns the command name
//     pub(crate) fn get_name(&self) -> &str {
//         match self {
//             Command::Get(_) => "get",
//             Command::Publish(_) => "pub",
//             Command::Set(_) => "set",
//             Command::Subscribe(_) => "subscribe",
//             Command::Unsubscribe(_) => "unsubscribe",
//             Command::Ping(_) => "ping",
//             Command::Unknown(cmd) => cmd.get_name(),
//         }
//     }
// }
