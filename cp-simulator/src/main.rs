use log;

use cp_simulator::config;
use cp_simulator::logger::init_logger; 
use cp_simulator::manager::Manager;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let config_path = if args.len() >= 3 {
        args.get(2).unwrap()
    } else {
        "./config/application.yml"
    };
    println!("config_path: {}", config_path);

    let config_opt = config::loader::load_config(config_path);
    if config_opt.is_none() {
        println!("config init error: file_path:{}", config_path);
    }
    let config = config_opt.unwrap();
    init_logger(&config.log_config);
    log::info!("init logger");

    let mut manager = Manager::new(&config);
    manager.start();

}
