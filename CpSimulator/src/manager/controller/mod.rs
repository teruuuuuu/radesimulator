use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;

mod client;
mod price;

use crate::config::Config;
use client::TcpClient;
use price::PriceManager;

pub enum Status {
    Stop,
    Start
}

pub struct Controller {
    status: Status,
    price_manager: PriceManager,
    client: Arc<Mutex<TcpClient>>
}

impl Controller {
    
    pub fn new(config_ref: &Config) -> Self {
        let client = Arc::new(Mutex::new(TcpClient::new(config_ref.client_config.host.to_string(), config_ref.client_config.port)));
        let price_manager = PriceManager::new(&config_ref.price_configs, client.clone());
        Controller { 
            status: Status::Stop,
            price_manager,
            client,
         }
    }

    pub fn start(&mut self) -> Vec<JoinHandle<()>>{
        self.status = Status::Start;
        
        let mut ret = Vec::new();
        ret.append(&mut self.price_manager.start());
        ret
    }

    pub fn stop(&mut self) {
        log::info!("controller stop");
        self.price_manager.stop();
    }
}




