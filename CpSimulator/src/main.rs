use log::{error, warn, info, debug};

use cp_simulator::config;
use cp_simulator::logger::init_logger; 
use cp_simulator::manager::Manager;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let config_path = if args.len() > 1 {
        args.get(0).unwrap()
    } else {
        "./config/application.yml"
    };

    let config_opt = config::loader::load_config(config_path);
    if config_opt.is_none() {
        println!("config init error: file_path:{}", config_path);
    }
    let config = config_opt.unwrap();
    init_logger(&config.log_config);
    info!("init logger");

    let mut manager = Manager::new(&config);
    manager.start();

}
