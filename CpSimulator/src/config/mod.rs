use std::fmt::Debug;
use std::collections::HashMap;

pub mod loader; 

#[derive(Debug)]
pub struct Config {
    pub log_config: LogConfig,
    pub listener_config: ListnerConfig,
    pub price_configs: HashMap<String, PriceConfig>
}

impl Config {
    pub fn new(log_config: LogConfig, 
        listener_config: ListnerConfig,
        price_configs: HashMap<String, PriceConfig>) -> Self {
        Config { log_config, listener_config, price_configs }
    }
}

#[derive(Debug)]
pub struct LogConfig {
    pub log_path: String,
}

impl LogConfig {
    pub fn new(log_path: String ) -> Self {
        LogConfig { log_path }
    }
}

#[derive(Debug)]
pub struct ListnerConfig {
    pub port: u16
}

impl ListnerConfig {
    pub fn new(port: u16) -> Self {
        ListnerConfig { port }
    }
}

#[derive(Debug)]
pub struct PriceConfig {
    pub symbol: String, // 銘柄 
    pub tick: f64,      // 初期プライス
    pub spread: f64,    // スプレッド
    pub sigma: f64,     // ボラティリティ
    pub r: f64          // 短期金利
}

impl PriceConfig {
    pub fn new(symbol: String, tick: f64, spread: f64, sigma: f64, r: f64 ) -> Self {
        PriceConfig { symbol, tick, spread, sigma, r }
    }
}