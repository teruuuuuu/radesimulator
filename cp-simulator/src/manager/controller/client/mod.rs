use std::net::TcpStream;
use std::io::{Write, stdout};

pub struct TcpClient {
    host: String,
    port: u16,
    tcp_stream_opt: Option<TcpStream>
}

impl TcpClient {
    pub fn new(host: String, port: u16) -> Self {
        let tcp_stream_opt = TcpClient::connect(host.to_string(), port);
        TcpClient { host, port, tcp_stream_opt }
    }

    fn connect(host: String, port: u16) -> Option<TcpStream> {
        let remote = format!("{}:{}", host, port).parse().unwrap();
        let tcp_stream_res = TcpStream::connect_timeout(&remote, std::time::Duration::from_millis(100)); 
        match tcp_stream_res {
            Result::Ok(stream) => Some(stream),
            Result::Err(_) => None
        }
    }

    pub fn send(&mut self, message: String) {
        stdout().flush().unwrap();
        let send_message = message.to_string() + "\r\n";
        if self.tcp_stream_opt.is_some() {
            match self.tcp_stream_opt.as_mut().unwrap().write(send_message.as_bytes()) {
                Result::Ok(_) => {},
                Result::Err(_) => {
                    log::error!("send failed[{}]", message);
                    self.tcp_stream_opt = None;
                }
            }
        } else {
            log::error!("connection not established[{}]", message);
            self.tcp_stream_opt = TcpClient::connect(self.host.to_string(), self.port);
        }
    }
}

impl Drop for TcpClient {
    fn drop(&mut self) {
        println!("Shutdown call");
        
        match &self.tcp_stream_opt {
            Some(tcp_stream) => {
                tcp_stream.shutdown(std::net::Shutdown::Both).unwrap();
            },
            _ => {}
        }
    }
   
}