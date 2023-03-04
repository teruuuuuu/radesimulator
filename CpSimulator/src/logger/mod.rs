use env_logger;
use env_logger::Builder;
use log::{error, warn, info, debug};
use log::LevelFilter;

use crate::config;

pub fn init_logger(log_config_ref: &config::LogConfig) {
    use std::io::Write;
    use chrono::Local;
    
    let path_str = log_config_ref.log_path.clone();
    let path = std::path::Path::new(&path_str);

    let target = if path.exists() {        
        Box::new(std::fs::OpenOptions::new().write(true).append(true).open(path).expect("Can't open file"))
    } else {
        Box::new(std::fs::OpenOptions::new().write(true).create(true).open(path).expect("Can't create file"))
    };


    Builder::new()
        .format(|buf, record| {
            writeln!(buf,
                "{} {} [{}-{}] {}",
                Local::now().format("%Y-%m-%d %H:%M:%S.%3f"),
                record.level(),
                record.file().unwrap_or(""),
                record.line().unwrap_or(0),
                record.args()
            )
        })
        .target(env_logger::Target::Pipe(target))
        .filter(None, LevelFilter::Info)
        .init();
}