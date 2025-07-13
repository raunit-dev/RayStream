mod client;
mod subscription;
mod processing;
mod constants;

use crate::constants::RUST_LOG_LEVEL;
use tokio;
use log::{info};


#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let _ = env::set_var("RUST_LOG", RUST_LOG_LEVEL);
    env_logger::init();
    info!("Starting to monitor account: {}", constants::RAYDIUM_LAUNCHPAD_PROGRAM);

    let mut client = client::setup_client().await?;
    info!("Connected to gRPC endpoint");
    let (subscribe_tx, subscribe_rx) = client.subscribe().await?;
    
    subscription::send_subscription_request(subscribe_tx).await?;
    info!("Subscription request sent. Listening for updates...");
    
    processing::process_updates(subscribe_rx).await?;
    
    info!("Stream closed");
    Ok(())
}