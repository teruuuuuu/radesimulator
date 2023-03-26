use std::thread;
use std::sync::Arc;

use std::io::prelude::*;
use std::sync::Mutex;
use std::thread::JoinHandle;
use std::time::Duration;
use std::net::TcpListener;
use std::net::TcpStream;

use crate::config::ListnerConfig;


pub struct Listener {
    is_end: Arc<Mutex<bool>>,
    port: u16,
}

impl Listener {
    pub fn new(is_end: Arc<Mutex<bool>>, listner_config_ref: &ListnerConfig) -> Self {

        Listener { 
            is_end,
            port: listner_config_ref.port,
        }
    }


    pub fn start(&mut self) -> JoinHandle<()>{
        fn blocking_listen_thread(port: u16, is_end: Arc<Mutex<bool>>) -> JoinHandle<()>{
            let tcp_listener = TcpListener::bind(format!("127.0.0.1:{}", port)).unwrap();
            tcp_listener.set_nonblocking(true).expect("Cannot set non-blocking");
            std::thread::spawn(move|| {

                for stream in tcp_listener.incoming() {
                    match stream {
                        Ok(s) => {
                            handle_connection(s, is_end.clone());
                        }
                        Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                            // wait until network socket is ready, typically implemented
                            // via platform-specific APIs such as epoll or IOCP
                            // wait_for_fd();
                        }
                        Err(e) => panic!("encountered IO error: {e}"),
                    }

                    if !(*is_end.lock().unwrap()) {
                        // set_nonblockingにしてポーリングし続けているのでsleepを入れておく
                        thread::sleep(Duration::from_secs(1));
                    } else {
                        break;
                    }
                }
            })     
        }
        blocking_listen_thread(self.port, self.is_end.clone())
    }
}

fn handle_connection(mut stream: TcpStream, is_end: Arc<Mutex<bool>>) {
    let mut buffer = [0; 1024];
    stream.read(&mut buffer).unwrap();

    let terminate = b"POST /terminate HTTP/1.1\r\n";

    if buffer.starts_with(terminate) {
        // 停止
        (*is_end.lock().unwrap()) = true;
    }

    let response = format!("HTTP/1.1 200 OK\r\n\r\n");
    stream.write(response.as_bytes()).unwrap();
    stream.flush().unwrap();
}