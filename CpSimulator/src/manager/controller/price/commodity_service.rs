use chrono::offset::Utc;
use chrono::DateTime;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;
use std::time::SystemTime;
use log;

pub struct CommodityService {
    code: u16,
    id_count: u32
}

impl CommodityService {
    pub fn new(code: u16) -> Self {
        CommodityService { code, id_count: 0 }
    }

    pub fn create_price_id(&mut self) -> String {
        let create_time = SystemTime::now();
        let datetime: DateTime<Utc> = create_time.into();
        self.id_count += 1;
        if self.id_count >= 10000 {
            self.id_count = 1;
        }

        format!("P{0:>02}-{1}{2:>04}", self.code, datetime.format("%Y%m%d%H%M%S%3f"), self.id_count)
    }

}