use chrono::offset::Utc;
use chrono::DateTime;
use std::sync::Mutex;
use std::time::SystemTime;
// use log;

pub struct CommodityService {
    code: u16,
    id_count: Mutex<u32>
}

impl CommodityService {
    pub fn new(code: u16) -> Self {
        CommodityService { code, id_count: Mutex::new(0) }
    }

    pub fn create_price_id(&self) -> String {
        let create_time = SystemTime::now();
        let datetime: DateTime<Utc> = create_time.into();

        let mut id_count = self.id_count.lock().unwrap();
        (*id_count) += 1;
        if *id_count >= 10000 {
            (*id_count) = 1;
        }

        format!("P{0:>02}-{1}{2:>04}", self.code, datetime.format("%Y%m%d%H%M%S%3f"), id_count)
    }

}