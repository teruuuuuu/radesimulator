use std::sync::{Arc, Mutex};

mod controller;
mod listener;

use crate::config::Config;
use controller::Controller;
use listener::Listener;
use std::thread::JoinHandle;

pub struct Manager {
    is_end: Arc<Mutex<bool>>,
    controller: Controller,
    listener: Listener
}

impl Manager {
    pub fn new(config_ref: &Config) -> Self {
        let is_end = Arc::new(Mutex::new(false));
        let controller = Controller::new(&config_ref);
        let listener = Listener::new(is_end.clone(), &config_ref.listener_config);
        
        Manager { 
            is_end,
            controller,
            listener
         }
    }

    pub fn start(&mut self) {
        let mut join_handles:Vec<JoinHandle<()>> = Vec::new();
        join_handles.append(&mut self.controller.start());
        join_handles.push(self.listener.start());

        // スレッドの停止を待つ
        loop {
            if *self.is_end.lock().unwrap() {
                self.controller.stop();
                // listener側はis_endを参照するので何もしない
            }

            if join_handles.iter().filter(|handle| !handle.is_finished()).count() == 0 {
                // 待ちスレッドなくなったら終了
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(1000));
        }        
    }
}