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
    tick_maker: TickMaker,
    commodity_service: CommodityService,
    spread: f64,
    price_queue: VecDeque<Price>,
    client: Arc<Mutex<TcpClient>>
}

impl CommodityMaker {
    fn new(symbol: String, code: u16, tick_maker: TickMaker, spread: f64, client: Arc<Mutex<TcpClient>>) -> Self {
        let commodity_service = CommodityService::new(code);
        let price_queue = VecDeque::with_capacity(10);
        CommodityMaker { symbol, tick_maker, commodity_service, spread, price_queue, client }
    }

    pub fn run(&mut self) {
        let mid = self.tick_maker.gen();
        let create_time = SystemTime::now();
        let datetime: DateTime<Utc> = create_time.into();
        let price_id = self.commodity_service.create_price_id();
        let create_time_str = datetime.format("%Y/%m/%d %H:%M:%S%.3f").to_string();

        let price = Price::new(self.symbol.to_string(), price_id, mid, mid - self.spread, mid + self.spread, create_time_str);
        log::info!("{} {}", &price, self.price_queue.len());
        while self.price_queue.len() >= 1 {
            self.price_queue.pop_front();
        }
        self.client.lock().as_mut().unwrap().send(format!("{}\n",&price));
        self.price_queue.push_back(price);
    }

    fn ref_latest_price(&self) -> Option<&Price> {
        self.price_queue.back()
    }
}

pub struct PriceManager {
    // TODO 更新参照でのロックの粒度がCommodityMakerになっているので、もっと狭い範囲になるようにする
    tick_makers: HashMap<String, Arc<Mutex<CommodityMaker>>>,
    interval_workers: Vec<Mutex<IntervalWorker>>,
    client: Arc<Mutex<TcpClient>>
}

impl PriceManager {
    pub fn new(price_config_ref: &HashMap<String, PriceConfig>, client: Arc<Mutex<TcpClient>>) -> Self {
        let mut tick_makers = HashMap::new();
        let mut interval_workers = Vec::new();
        for (symbol, config) in price_config_ref.iter() {
            
            
            tick_makers.insert(
                symbol.to_string(), 
                Arc::new(Mutex::new(
                    CommodityMaker::new(
                        config.symbol.to_string(), 
                        config.code,
                        TickMaker::Random(RandomWalk::new(config.base_tick, config.sigma, config.r)),
                        config.spread,
                        client.clone()
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
            client
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

// #[cfg(test)]
// mod tests {
//     use super::*;

//     use chrono::Local;
//     use log::LevelFilter;

//     use std::{io::prelude::*, ops::Deref};

//     fn init() {
//         let _ = env_logger::builder().format(|buf, record| {
//             writeln!(buf,
//                 "{} {} [{}-{}] {}",
//                 Local::now().format("%Y-%m-%d %H:%M:%S.%s"),
//                 record.level(),
//                 record.file().unwrap_or(""),
//                 record.line().unwrap_or(0),
//                 record.args()
//             )
//         })
//         .target(env_logger::Target::Stdout)
//         .filter(None, LevelFilter::Warn)
//         .try_init();
//     }

    
//     #[test]
//     fn test_load_config() {
//         init();
//         let tick_maker = TickMaker::Random(RandomWalk::new(136.54, 0.4, 0.01));


//         let mut commodity_maker = CommodityMaker::new("USD/JPY".to_owned(), 1, tick_maker, 0.02);
//         let commodity_maker_ref = &commodity_maker;

//         println!("start");

//         let h1 = std::thread::spawn(move || {
//             loop {
//                 // commodity_maker_ref.run();
//                 std::thread::sleep(std::time::Duration::from_millis(1));
//             }
//         });

//         // loop {
//         //     std::thread::sleep(std::time::Duration::from_millis(10));
//         //     log::error!("{:?}", commodity_maker.ref_latest_price());
//         // }


//         h1.join().unwrap();
//         println!("end");
//     }


//     #[test]
//     fn test_1() {
//         use std::sync::Arc;
//         use std::cell::RefCell;

//         init();
//         {
//             #[derive(Debug)]
//             struct Count(u32);
//             struct Service { count: u32, count_queue: VecDeque<Count>}
        
//             impl Service {
//                 pub fn new() -> Self {
//                     Service { count: 0, count_queue: VecDeque::new() }
//                 }
//                 pub fn count_up(&mut self) {
//                     // 時間がかかる場合を想定
//                     std::thread::sleep(std::time::Duration::from_millis(1000));
//                     if self.count >= 10000 {
//                         self.count = 0;
//                     }  else {
//                         self.count += 1;
//                     }
//                     while self.count_queue.len() >= 1 {
//                         self.count_queue.pop_front();
//                     }
//                     self.count_queue.push_back(Count(self.count))
//                 }
//                 pub fn get_latest_ref(&self) -> Option<&Count> {
//                     self.count_queue.back()
//                 }
//             }
//             let service = Service::new();
//             let service_ref = Arc::new(Mutex::new(service));

//             let service_ref1 = service_ref.clone();
//             let service_ref2 = service_ref.clone();

//             let t1: JoinHandle<()> = std::thread::spawn(move || {
//                 loop {
//                     service_ref1.lock().unwrap().count_up();
//                     std::thread::sleep(std::time::Duration::from_millis(1));
//                 }
//             });
//             let t2: JoinHandle<()> = std::thread::spawn(move || {
//                 loop {
//                     log::warn!("{:?}", service_ref2.lock().as_ref().unwrap().get_latest_ref());
//                     std::thread::sleep(std::time::Duration::from_millis(1));
//                 }
//             });

//             t1.join();
//             t2.join();
//         }
//     }

//     use tokio::task;

//     #[test]
//     fn test_2() {
//         use std::sync::Arc;
        


//         init();
//         {
//             #[derive(Debug)]
//             struct Count(u32);
//             struct Service { count: u32, count_queue: VecDeque<Count>}
        
//             impl Service {
//                 pub fn new() -> Self {
//                     Service { count: 0, count_queue: VecDeque::new() }
//                 }
//                 pub async fn count_up(&mut self) {
//                     // 時間がかかる場合を想定
//                     std::thread::sleep(std::time::Duration::from_millis(1000));
//                     if self.count >= 10000 {
//                         self.count = 0;
//                     }  else {
//                         self.count += 1;
//                     }
//                     while self.count_queue.len() >= 1 {
//                         self.count_queue.pop_front();
//                     }
//                     self.count_queue.push_back(Count(self.count))
//                 }
//                 pub fn get_latest_ref(&self) -> Option<&Count> {
//                     self.count_queue.back()
//                 }
//             }
//             let service = Service::new();
//             let service_ref = Arc::new(Mutex::new(service));

//             let service_ref1 = service_ref.clone();
//             let service_ref2 = service_ref.clone();

//             let t1: JoinHandle<()> = std::thread::spawn(move || {
//                 loop {
//                     let res = task::spawn_blocking(move || {
//                         // めっちゃ重い処理をここでやる
//                         "done computing"
//                     }).await?;
//                     async {
//                         service_ref1.lock().unwrap().count_up().await;
//                     };
//                     std::thread::sleep(std::time::Duration::from_millis(1));
//                 }
//             });
//             let t2: JoinHandle<()> = std::thread::spawn(move || {
//                 loop {
//                     log::warn!("{:?}", service_ref2.lock().as_ref().unwrap().get_latest_ref());
//                     std::thread::sleep(std::time::Duration::from_millis(10000));
//                 }
//             });

//             t1.join();
//             t2.join();
//         }
    
//     }
// }