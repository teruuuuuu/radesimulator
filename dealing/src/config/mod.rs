use std::fmt::Debug;

pub mod loader;


#[derive(Debug)]
pub struct Config {
    pub commodities: Vec<Commodity>,
    pub cp_list:Vec<CpInfo>
}

impl Config {
    pub fn new(commodities: Vec<Commodity>, cp_list:Vec<CpInfo>) -> Self {
        Config { commodities, cp_list }
    }
}


#[derive(Debug)]
pub struct Commodity {
    pub code: i32,
    pub symbol: String,
    pub pip_size: f32
}

impl Commodity {
    pub fn new(code: i32, symbol: String, pip_size: f32) -> Self {
        Commodity { code, symbol, pip_size }
    }
}

#[derive(Debug)]
pub struct CpInfo {
    pub price_port: i32
}

impl CpInfo {
    pub fn new(price_port: i32) -> Self {
        CpInfo { price_port }
    }
}