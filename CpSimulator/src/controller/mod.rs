use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;
use log::{error, warn, info, debug};

use crate::config::Config;
use crate::price::price::PriceManager;

pub enum Status {
    Stop,
    Start
}

pub struct Controller {
    status: Status,
    price_manager: PriceManager,
}

impl Controller {
    
    pub fn new(config_ref: &Config) -> Self {
        let price_manager = PriceManager::new(&config_ref.price_configs);
        Controller { 
            status: Status::Stop,
            price_manager,
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




