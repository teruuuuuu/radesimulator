use std::sync::{Arc, Mutex};
use std::time::Instant;
use std::thread::{self, JoinHandle};

use super::*;


pub struct IntervalWorker {
    interval: u64,
    runnable:  Arc<CommodityMaker>,
    is_runnning: Arc<Mutex<bool>>,
}

impl IntervalWorker {

    pub fn new(interval: u64, runnable: Arc<CommodityMaker>) -> Self {
        IntervalWorker { 
            interval, 
            runnable,
            is_runnning: Arc::new(Mutex::new(false))
        }
    }

    pub fn start(&mut self) -> std::thread::JoinHandle<()> { 
        fn make_loop_thread(interval: u64, runnable:  Arc<CommodityMaker>, is_runnning: Arc<Mutex<bool>>) -> JoinHandle<()> {
            thread::spawn(move || {
                (*is_runnning.lock().unwrap()) = true;
                let mut cur_time;
                let mut next_time = Instant::now().checked_add(std::time::Duration::from_millis(interval)).unwrap();
                loop {
                    runnable.run();
                    
                    if !(*is_runnning.lock().unwrap()) {
                        break;
                    } else {
                        cur_time = Instant::now();
                        let sleep_mili = std::cmp::max(next_time.duration_since(cur_time).as_millis() as u64, 10);
                        thread::sleep(std::time::Duration::from_millis(sleep_mili));
                        next_time = next_time.checked_add(std::time::Duration::from_millis(interval)).unwrap();
                    }
                }
            })
        }
        make_loop_thread(self.interval, self.runnable.clone(), self.is_runnning.clone())
    }

    pub fn stop(&mut self) {
        log::info!("stop worker");
        (*self.is_runnning.lock().unwrap()) = false;
    }
}
