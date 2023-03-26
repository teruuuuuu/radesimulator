use clap::Parser;
use tokio::net::TcpListener;
use tokio::signal;

#[cfg(feature = "otel")]
use tracing_subscriber::{
    fmt, layer::SubscriberExt, util::SubscriberInitExt, util::TryInitError, EnvFilter,
};
use tracing::{debug, error, info, instrument};

use dealing::config;
use dealing::listener::server::run;

pub const DEFAULT_PORT: u16 = 6379;
pub const MAX_CONNECTIONS: usize = 3;


#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt().init();
    info!("start");

    let cli = Cli::parse();
    let config_path = cli.config_path.unwrap_or("./dealing-simulator/config/application.yml".to_string());


    let config_opt = config::loader::load_config(&config_path);
    info!("{:?}", config_opt);

    // info!("listener port[{}]", port);
    // let listener = TcpListener::bind(&format!("127.0.0.1:{}", port)).await?;
    // run(listener, signal::ctrl_c()).await;
    
    Ok(())
}

#[derive(Parser, Debug)]
#[clap(name = "dealing-simulator", version, author = "", about = "")]
struct Cli {
    #[clap(long)]
    config_path: Option<String>,
}


