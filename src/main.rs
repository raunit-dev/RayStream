mod client;
mod processing;
mod constants;

use tokio;
use log::info;
use rustls::crypto::{ring, CryptoProvider};
use std::sync::Once;

static INIT_CRYPTO: Once = Once::new();
fn install_crypto() {
    INIT_CRYPTO.call_once(|| {
        CryptoProvider::install_default(ring::default_provider()).expect("install ring provider");
    });
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    install_crypto();
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info"))
        .format_timestamp_secs()
        .init();
    info!(
    "Logger initialized. Monitoring Meteora pool: {} and Orca pool: {}",
    constants::METEORA_POOL_ADDRESS,
    constants::ORCA_WHIRLPOOL_ADDRESS
    );

    let mut client = client::setup_client().await?;
    info!("Connected to gRPC endpoint");
    let (mut subscribe_tx, subscribe_rx) = client.subscribe().await?;
    
    info!("Connected to gRPC stream. Listening for updates...");
    
    processing::process_updates(subscribe_rx, &mut subscribe_tx).await?;
    
    info!("Stream closed");
    Ok(())
}