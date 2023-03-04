use chrono::offset::Utc;
use chrono::DateTime;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;
use std::time::SystemTime;

use log::{error, warn, info, debug};

use crate::config::PriceConfig;
use super::ticker::*;
use super::worker::*;
pub struct PriceMaker {
    pub symbol: String,
    pub tick_maker: TickMaker,
    pub spread: f64,
}

impl PriceMaker {
    fn new(symbol: String, tick_maker: TickMaker, spread: f64) -> Self {
        PriceMaker { symbol, tick_maker, spread }
    }

    pub fn run(&mut self) {
        let mid = self.tick_maker.gen();
        let create_time = SystemTime::now();
        let datetime: DateTime<Utc> = create_time.into();


        info!("symbol[{}] mid[{}] create_time[{}]", self.symbol, mid, datetime.format("%Y/%m/%d %H:%M:%S%.3f"));
    }
}

pub struct PriceManager {
    tick_makers: HashMap<String, Arc<Mutex<PriceMaker>>>,
    interval_workers: Vec<Mutex<IntervalWorker>>
}

impl PriceManager {
    pub fn new(price_config_ref: &HashMap<String, PriceConfig>) -> Self {
        let mut tick_makers = HashMap::new();
        let mut interval_workers = Vec::new();
        for (symbol, config) in price_config_ref.iter() {
            tick_makers.insert(
                symbol.to_string(), 
                Arc::new(Mutex::new(
                    PriceMaker::new(
                        config.symbol.to_string(), 
                        TickMaker::Random(RandomWalk::new(config.tick, config.sigma, config.r)),
                        config.spread
                    )
                ))
            );
        }
        
        for (_symbol, tick_maker) in tick_makers.iter() {
            interval_workers.push(
                    Mutex::new(IntervalWorker::new(1000, tick_maker.clone())));
                
        }

        PriceManager { 
            tick_makers,
            interval_workers,
         }
    }


    pub fn start(&mut self) -> Vec<JoinHandle<()>>{
        let mut ret = Vec::new();
        for worker in self.interval_workers.iter_mut() {
            ret.push(worker.get_mut().unwrap().start());
        }
        ret
    }

    pub fn stop(&mut self) {
        log::info!("price_manager stop");
        for worker in self.interval_workers.iter_mut() {
            worker.get_mut().unwrap().stop()
        }   
    }

}