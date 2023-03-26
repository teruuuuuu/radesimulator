use serde::{Serialize, Deserialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct CpPrice {
    symbol: String,
    price_id: String,
    mid: f64,
    bid: f64,
    ask: f64,
    create_time: DateTime<Local> // %Y/%m/%d %H:%M:%S%.3f
}

impl CpPrice {
    pub fn new(symbol: String, price_id: String, mid: f64, bid: f64, ask: f64, create_time: DateTime<Local>) -> Self {
        CpPrice {symbol, price_id, mid, bid, ask, create_time}
    }

    pub fn copy(&self) -> Self {
        CpPrice {
            symbol: self.symbol.clone(), 
            price_id: self.price_id.clone(), 
            mid: self.mid, bid: self.bid, ask: self.ask, 
            create_time: self.create_time.clone()}
    }
}

impl std::fmt::Display for CpPrice {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        // Use `self.number` to refer to each positional data point.
        write!(f, "{{\"symbol\":\"{}\",\"price_id\":\"{}\",\"mid\":{},\"bid\":{},\"ask\":{},\"create_time\":\"{}\"}}", 
            self.symbol,self.price_id,self.mid,self.bid,self.ask,self.create_time)
    }
}

use chrono::{Utc, Local, DateTime, Date};

#[derive(Debug, Serialize, Deserialize)]
struct ChronoTest {
    time: DateTime<Local>
}

#[test]
fn test() {
    



    
}