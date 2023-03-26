use clap::Parser;
#[cfg(feature = "otel")]
use tracing_subscriber::{
    fmt, layer::SubscriberExt, util::SubscriberInitExt, util::TryInitError, EnvFilter,
};
use tracing::{debug, error, info, instrument};


pub const DEFAULT_PORT: u16 = 6379;
pub const MAX_CONNECTIONS: usize = 3;


#[tokio::main]
async fn main() {
    tracing_subscriber::fmt().init();
    info!("start");

    let cli = Cli::parse();
    let port = cli.port.unwrap_or(DEFAULT_PORT);
    info!("listener port[{}]", port);
    
}

#[derive(Parser, Debug)]
#[clap(name = "dealing-simulator", version, author = "", about = "")]
struct Cli {
    #[clap(long)]
    port: Option<u16>,
}


