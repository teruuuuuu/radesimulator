// pub mod price;


use chrono::offset::Utc;
use chrono::DateTime;
use std::collections::{HashMap,VecDeque};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;
use std::time::SystemTime;
use log;

mod commodity_service;
mod worker;
mod ticker;

use crate::config::PriceConfig;
use commodity_service::CommodityService;
use ticker::*;
use worker::*;
use super::client::TcpClient;


#[derive(Debug)]
pub struct Price {
    symbol: String,
    price_id: String,
    mid: f64,
    bid: f64,
    ask: f64,
    create_time: String // %Y/%m/%d %H:%M:%S%.3f
}

impl Price {
    pub fn new(symbol: String, price_id: String, mid: f64, bid: f64, ask: f64, create_time: String) -> Self {
        Price {symbol, price_id, mid, bid, ask, create_time}
    }

    pub fn copy(&self) -> Price {
        Price {
            symbol: self.symbol.clone(), 
            price_id: self.price_id.clone(), 
            mid: self.mid, bid: self.bid, ask: self.ask, 
            create_time: self.create_time.clone()}
    }
}

impl std::fmt::Display for Price {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        // Use `self.number` to refer to each positional data point.
        write!(f, "{{\"symbol\":\"{}\",\"price_id\":\"{}\",\"mid\":{},\"bid\":{},\"ask\":{},\"create_time\":\"{}\"}}", 
            self.symbol,self.price_id,self.mid,self.bid,self.ask,self.create_time)
    }
}

pub struct CommodityMaker {
    symbol: String,
    tick_maker: Mutex<TickMaker>,
    commodity_service: CommodityService,
    spread: f64,
    price_queue: Mutex<VecDeque<Price>>,
    client: Arc<Mutex<TcpClient>>
}

impl CommodityMaker {
    fn new(symbol: String, code: u16, tick_maker: Mutex<TickMaker>, spread: f64, client: Arc<Mutex<TcpClient>>) -> Self {
        let commodity_service = CommodityService::new(code);
        let price_queue = Mutex::new(VecDeque::with_capacity(10));
        CommodityMaker { symbol, tick_maker, commodity_service, spread, price_queue, client }
    }

    pub fn run(&self) {
        let mid = self.tick_maker.lock().unwrap().gen();
        let create_time = SystemTime::now();
        let datetime: DateTime<Utc> = create_time.into();
        let price_id = self.commodity_service.create_price_id();
        let create_time_str = datetime.format("%Y/%m/%d %H:%M:%S%.3f").to_string();

        let price = Price::new(self.symbol.to_string(), price_id, mid, mid - self.spread, mid + self.spread, create_time_str);
        {
            let mut price_queue = self.price_queue.lock().unwrap();
            log::info!("{} {}", &price, price_queue.len());
            while price_queue.len() >= 1 {
                price_queue.pop_front();
            }
            self.client.lock().as_mut().unwrap().send(format!("{}\n",&price));
            price_queue.push_back(price);
        }
    }

    pub fn ref_latest_price(&self) -> Option<Price> {
        self.price_queue.lock().unwrap().back().map(|price| price.copy())
    }
}

pub struct PriceManager {
    // TODO 更新参照でのロックの粒度がCommodityMakerになっているので、もっと狭い範囲になるようにする
    tick_makers: HashMap<String, Arc<CommodityMaker>>,
    interval_workers: Vec<IntervalWorker>,
    client: Arc<Mutex<TcpClient>>
}

impl PriceManager {
    pub fn new(price_config_ref: &HashMap<String, PriceConfig>, client: Arc<Mutex<TcpClient>>) -> Self {
        let mut tick_makers = HashMap::new();
        let mut interval_workers = Vec::new();
        for (symbol, config) in price_config_ref.iter() {
            
            
            tick_makers.insert(
                symbol.to_string(), 
                Arc::new(
                    CommodityMaker::new(
                        config.symbol.to_string(), 
                        config.code,
                        Mutex::new(TickMaker::Random(RandomWalk::new(config.base_tick, config.sigma, config.r))),
                        config.spread,
                        client.clone()
                    )
                )
            );
        }
        
        for (_symbol, tick_maker) in tick_makers.iter() {
            interval_workers.push(
                    IntervalWorker::new(1000, tick_maker.clone()));
                
        }

        PriceManager { 
            tick_makers,
            interval_workers,
            client
         }
    }


    pub fn start(&mut self) -> Vec<JoinHandle<()>>{
        let mut ret = Vec::new();
        for worker in self.interval_workers.iter_mut() {
            ret.push(worker.start());
        }
        ret
    }

    pub fn stop(&mut self) {
        log::info!("price_manager stop");
        for worker in self.interval_workers.iter_mut() {
            worker.stop()
        }   
    }

}

#[cfg(test)]
mod tests {
    use super::*;

    use chrono::Local;
    use log::LevelFilter;

    use std::{io::prelude::*, ops::Deref};

    fn init() {
        let _ = env_logger::builder().format(|buf, record| {
            writeln!(buf,
                "{} {} [{}-{}] {}",
                Local::now().format("%Y-%m-%d %H:%M:%S.%3f"),
                record.level(),
                record.file().unwrap_or(""),
                record.line().unwrap_or(0),
                record.args()
            )
        })
        .target(env_logger::Target::Stdout)
        .filter(None, LevelFilter::Warn)
        .try_init();
    }

    
    #[test]
    fn test_load_config() {
        init();

        let price_gen = Arc::new(CommodityMaker::new(
            "USD/JPY".to_string(), 
            1,
            Mutex::new(TickMaker::Random(RandomWalk::new(136., 0.4, 0.2))),
            0.02,
            Arc::new(Mutex::new(TcpClient::new("127.0.0.1".to_owned(), 1234)))
        ));

        let mut worker = IntervalWorker::new(1000, price_gen.clone());
        let handler = worker.start();

        let price_ref = price_gen.clone();
        for i in 0..10 {
            log::error!("i:[{}] {:?}", i, price_ref.ref_latest_price());

            std::thread::sleep(std::time::Duration::from_millis(1000));
        }


        // let tick_maker = TickMaker::Random(RandomWalk::new(136.54, 0.4, 0.01));


        // let mut commodity_maker = CommodityMaker::new("USD/JPY".to_owned(), 1, tick_maker, 0.02);
        // let commodity_maker_ref = &commodity_maker;

        // println!("start");

        // let h1 = std::thread::spawn(move || {
        //     loop {
        //         // commodity_maker_ref.run();
        //         std::thread::sleep(std::time::Duration::from_millis(1));
        //     }
        // });

        // loop {
        //     std::thread::sleep(std::time::Duration::from_millis(10));
        //     log::error!("{:?}", commodity_maker.ref_latest_price());
        // }


        // h1.join().unwrap();
        println!("end");
    }

}