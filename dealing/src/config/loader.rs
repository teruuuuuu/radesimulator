use std::io::prelude::*;
use tracing::{debug, error, info, instrument};
use yaml_rust::YamlLoader;

use super::*;

pub fn load_config(config_path: &str) -> Option<Config> {
    info!("config path[{}]", config_path);
    let path = std::path::Path::new(config_path);

    let file_opt = match std::fs::File::open(&path) {
        Err(why) => {
            error!("couldn't open {}: {}", path.display(), why);
            None
        },
        Ok(file) => Some(file),
    };

    let file_content_opt =file_opt.and_then(move |mut file|{
        let mut s = String::new();
        match file.read_to_string(&mut s) {
            Err(why) => {
                error!("couldn't read {}: {}", path.display(), why);
                None   
            },
            Ok(_) => {        
                Some(s)
            },
        }
    });
    file_content_opt.and_then(|file_content| {
        load_config_from_str(&file_content)
    })
}

fn load_config_from_str(file_content: &str) -> Option<Config> {
    let docs = YamlLoader::load_from_str(file_content).unwrap();
    
    match docs.len() > 0 {
        true => {
            let doc = &docs[0];
            

            let commodities_opt = doc["commodities"].as_vec();
            let commodities = commodities_opt.map(|commodities| commodities.into_iter().map(|commodity| {
                let code = commodity["code"].as_i64().unwrap() as i32;
                let symbol = commodity["symbol"].as_str().unwrap();
                let pip_size = commodity["pip_size"].as_f64().unwrap() as f32;
                Commodity::new(code, symbol.to_string(), pip_size)
            }).collect::<Vec<Commodity>>());

            let cp_list_opt = doc["cp_list"].as_vec();
            let cp_list = cp_list_opt.map(|cp_list| cp_list.into_iter().map(|cp| {
                let price_port = cp["price_port"].as_i64().unwrap();

                CpInfo::new(price_port as i32)
            }).collect::<Vec<CpInfo>>());

            if commodities.is_some() && cp_list.is_some() {
                Some(Config::new(
                    commodities.unwrap(), 
                    cp_list.unwrap())
                )
            } else {
                None
            }
            // None
        },
        false => None
    }
    
}

#[cfg(test)]
mod tests {
    use tracing_subscriber;
    use super::*;


    
    #[test]
    fn test_load_config() {
        tracing_subscriber::fmt().init();

        {
            let config_content = " 
commodities:
  - code: 1
    symbol: \"USD/JPY\"
    pip_size: 0.01
  - code: 2
    symbol: \"EUR/JPY\"
    pip_size: 0.01
  - code: 3
    symbol: \"GBP/JPY\"
    pip_size: 0.01
  - code: 4
    symbol: \"AUD/JPY\"
    pip_size: 0.01


cp_list:
  - price_port: 6379
";
                    let config_opt = load_config_from_str(config_content);
                    println!("{:?}", config_opt);
                    assert!(config_opt.is_some());
        }

        {
            let config_content = "";
                    let config_opt = load_config_from_str(config_content);
                    assert!(config_opt.is_none());
        }
    }
}
