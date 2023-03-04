use rand::Rng;
use rand_distr::StandardNormal;
use std::time::{Instant,SystemTime};

use log::{error, warn, info, debug};

trait Ticker {
    fn gen(&mut self) -> f64;
}

// 幾何ブラウン運動によるランダムプライス生成
// https://ja.wikipedia.org/wiki/%E5%B9%BE%E4%BD%95%E3%83%96%E3%83%A9%E3%82%A6%E3%83%B3%E9%81%8B%E5%8B%95
#[derive(Clone)]
pub struct RandomWalk {
    pub value: f64, // 現在のプライス
    pub time: Instant, // 時刻
    pub sigma: f64, // ボラティリティ
    pub r: f64, // 短期金利
    
}

impl RandomWalk {
    pub fn new(value: f64, sigma: f64, r: f64) -> Self {
        RandomWalk {
            value,
            time: Instant::now(),
            sigma,
            r,
        }
    }
}


impl Ticker for RandomWalk {
    fn gen(&mut self) -> f64 {
        let mut random = rand::thread_rng();
        let now = Instant::now();
        let mut dt = now.duration_since(self.time).as_secs_f64() / (12 * 4 * 5 * 24 * 60 * 60) as f64;
        dt *= 500.;

        self.value *= std::f64::consts::E.powf((self.r - 0.5 * self.sigma.powf(2.0)) * dt 
            + self.sigma * dt.sqrt() * random.sample::<f64, _>(StandardNormal));
        self.time = now;
        self.value
    }
}

pub enum TickMaker {
    Random(RandomWalk)
}

impl TickMaker {
    pub fn gen(&mut self) -> f64 {
        match self {
            TickMaker::Random(ticker) => ticker.gen()
        }
    }
}