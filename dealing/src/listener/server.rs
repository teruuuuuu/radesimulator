use bytes::{Buf, Bytes, BytesMut};
use std::collections::{HashMap, VecDeque};

use std::future::Future;
use std::io::Cursor;
use std::sync::{Arc,Mutex};
use tokio::sync::{broadcast, mpsc, Semaphore};

use tokio::io::{AsyncReadExt, AsyncWriteExt, BufWriter};
use tokio::net::{TcpListener, TcpStream};
use tokio::time::{self, Duration};
use tracing::{debug, error, info, instrument};

use shared_util::cp::CpPrice;

type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

#[derive(Debug)]
pub struct PriceCache {
    cache: Mutex<HashMap<String, VecDeque<CpPrice>>>
}

impl PriceCache {
    fn add(&self, price: CpPrice) {
        info!("receive price[{}", price);
    }
}

#[derive(Debug)]
struct CpListener {
    price_cache: Arc<PriceCache>,
    listener: TcpListener,
    limit_connections: Arc<Semaphore>,

}

#[derive(Debug)]
struct Handler {
    price_cache: Arc<PriceCache>,
    connection: Connection,
}

#[derive(Debug)]
pub struct Connection {
    stream: BufWriter<TcpStream>,
    buffer: BytesMut,
}

pub enum Frame {
    Data(String),
}

#[derive(Debug)]
pub enum FrameError {
    Incomplete,
}

impl std::fmt::Display for FrameError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "invalid frame")
    }
}

impl std::error::Error for FrameError {}

impl CpListener {

    async fn run(&mut self) -> Result<()> {
        loop {
            let permit = self
                .limit_connections
                .clone()
                .acquire_owned()
                .await
                .unwrap();

            let socket = self.accept().await?;

            let mut handler = Handler {
                price_cache: self.price_cache.clone(),
                connection: Connection::new(socket),
            };

            tokio::spawn(async move {
                if let Err(err) = handler.run().await {
                    // error!(cause = ?err, "connection error");
                }
                drop(permit);
            });
        }
    }


    async fn accept(&mut self) -> Result<TcpStream> {
        let mut backoff = 1;

        loop {
            match self.listener.accept().await {
                Ok((socket, _)) => return Ok(socket),
                Err(err) => {
                    if backoff > 64 {
                        return Err(err.into());
                    }
                }
            }
            time::sleep(Duration::from_secs(backoff)).await;
            backoff *= 2;
        }
    }
}


impl Handler {
    async fn run(&mut self) -> Result<()> {
        loop {
            let maybe_frame = self.connection.read_frame().await?;

            let frame = match maybe_frame {
                Some(frame) => frame,
                None => return Ok(()),
            };

            let cp_price = from_frame(frame)?;
            self.price_cache.add(cp_price);
        }

        Ok(())
    }
}


impl Connection {
    pub fn new(socket: TcpStream) -> Connection {
        Connection {
            stream: BufWriter::new(socket),
            buffer: BytesMut::with_capacity(4 * 1024),
        }
    }

    pub async fn read_frame(&mut self) -> Result<Option<Frame>> {
        loop {
            let mut buf = Cursor::new(&self.buffer[..]);
            if let Ok(frame) = Frame::parse(&mut buf) {
                let len = buf.position() as usize;
                self.buffer.advance(len);
                return Ok(Some(frame));
            }

            if 0 == self.stream.read_buf(&mut self.buffer).await? {
                if self.buffer.is_empty() {
                    return Ok(None);
                } else {
                    return Err("connection reset by peer".into());
                }
            }
        }
    }

}

impl Frame {
    pub fn parse(src: &mut Cursor<&[u8]>) -> Result<Frame > {
        let line = get_line(src)?;
        let frame = String::from_utf8(line.to_vec())?;
        Ok(Frame::Data(frame))
    }
}

pub fn from_frame(frame: Frame) -> Result<CpPrice> {
    match frame {
        Frame::Data(line) => serde_json::from_str::<CpPrice>(&line).map_err(|e| e.into())
    }
}    


fn get_line<'a>(src: &mut Cursor<&'a [u8]>) -> Result<&'a [u8]> {
    if src.get_ref().len() == 0 {
        return Err(FrameError::Incomplete.into());
    }

    let start = src.position() as usize;
    // Scan to the second to last byte
    let end = src.get_ref().len() - 1;

    for i in start..end {
        if src.get_ref()[i] == b'\r' && src.get_ref()[i + 1] == b'\n' {
            src.set_position((i + 2) as u64);
            return Ok(&src.get_ref()[start..i]);
        }
    }
    Err(FrameError::Incomplete.into())
}


pub async fn run(listener: TcpListener, shutdown: impl Future) {
    let price_cache = Arc::new(PriceCache {
        cache: Mutex::new(HashMap::new())
    });
    
    let mut server = CpListener {
        listener,
        price_cache,
        limit_connections: Arc::new(Semaphore::new(3)),
    };


    tokio::select! {
        res = server.run() => {
            if let Err(err) = res {
                error!(cause = %err, "failed to accept");
            }
        }
        _ = shutdown => {
            info!("shutting down");
        }
    }
}


