use log;
use std::collections::HashMap;
use std::io::prelude::*;
use yaml_rust::YamlLoader;


use super::*;

pub fn load_config(config_path: &str) -> Option<Config> {
    log::info!("config path[{}]", config_path);
    let path = std::path::Path::new(config_path);

    let file_opt = match std::fs::File::open(&path) {
        Err(why) => {
            log::error!("couldn't open {}: {}", path.display(), why);
            None
        },
        Ok(file) => Some(file),
    };

    let file_content_opt =file_opt.and_then(move |mut file|{
        let mut s = String::new();
        match file.read_to_string(&mut s) {
            Err(why) => {
                log::error!("couldn't read {}: {}", path.display(), why);
                None   
            },
            Ok(_) => {        
                Some(s)
            },
        }
    });
    log::info!("config \n {}", file_content_opt.clone().unwrap());

    file_content_opt.and_then(|file_content| {
        load_config_from_str(&file_content)
    })
}

fn load_config_from_str(file_content: &str) -> Option<Config> {
    let docs = YamlLoader::load_from_str(file_content).unwrap();
    // println!("{:?}", &docs[0]);
    
    match docs.len() > 0 {
        true => {
            let doc = &docs[0];
            
            let file_path_opt = doc["log"]["filePath"].as_str();
            let log_config_opt = file_path_opt.map(|file_path| LogConfig::new(String::from(file_path)));

            let port_opt = doc["listener"]["port"].as_i64().map(|port| port as u16);
            let listener_config_opt = port_opt.map(|port| ListnerConfig::new(port) );


            let price_vec_opt = doc["cp"]["prices"].as_vec();
            let price_config_opt = price_vec_opt.map(|price_vec| price_vec.into_iter().map(|price| {
                let symbol = price["symbol"].as_str().unwrap();
                let tick = price["tick"].as_f64().unwrap();
                let spread = price["spread"].as_f64().unwrap();
                let sigma = price["sigma"].as_f64().unwrap();
                let r = price["r"].as_f64().unwrap();
                (symbol.to_string(), PriceConfig::new(symbol.to_string(), tick, spread, sigma, r))
            }).collect::<HashMap<String, PriceConfig>>());
    
            if log_config_opt.is_some() && listener_config_opt.is_some() && price_config_opt.is_some() {
                Some(Config::new(
                    log_config_opt.unwrap(), 
                    listener_config_opt.unwrap(), 
                    price_config_opt.unwrap())
                )
            } else {
                None
            }
        },
        false => None
    }
    
}

#[cfg(test)]
mod tests {
    use super::*;

    use chrono::Local;
    use log::LevelFilter;

    fn init() {
        let _ = env_logger::builder().format(|buf, record| {
            writeln!(buf,
                "{} {} [{}-{}] {}",
                Local::now().format("%Y-%m-%d %H:%M:%S.%s"),
                record.level(),
                record.file().unwrap_or(""),
                record.line().unwrap_or(0),
                record.args()
            )
        })
        .target(env_logger::Target::Stdout)
        .filter(None, LevelFilter::Info)
        .try_init();
    }

    
    #[test]
    fn test_load_config() {
        init();

        {
            let config_content = " 
log:
  filePath: \"./log/tickmaker.log\"

listener:
  port: 2345

cp:
  prices:
    - symbol: \"USD/JPY\"
      tick: 136.54
      spread: 0.02
      sigma: 0.4
      r: 0.01
    - symbol: \"EUR/JPY\"
      tick: 144.870
      spread: 0.02
      sigma: 0.4
      r: 0.01
    - symbol: \"GBP/JPY\"
      tick: 163.4
      spread: 0.02
      sigma: 0.4
      r: 0.01
    - symbol: \"AUD/JPY\"
      tick: 92.09
      spread: 0.02
      sigma: 0.4
      r: 0.01
";
                    let config_opt = load_config_from_str(config_content);
                    assert!(config_opt.is_some());
        }

        {
            let config_content = "";
                    let config_opt = load_config_from_str(config_content);
                    assert!(config_opt.is_none());
        }
    }
}
