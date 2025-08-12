use crate::constants::{ORCA_WHIRLPOOL_ADDRESS, METEORA_POOL_ADDRESS, SOL_DECIMALS, USDC_DECIMALS, SOL_MINT, USDC_MINT};
use futures::{Stream, stream::StreamExt};
use log::{error, info, warn};
use solana_program::pubkey::Pubkey;
use orca_whirlpools_client::Whirlpool;
use tonic::Status;
use yellowstone_grpc_proto::geyser::SubscribeUpdate;
use yellowstone_grpc_proto::prelude::{subscribe_update::UpdateOneof, CommitmentLevel, SubscribeRequest, SubscribeRequestFilterAccounts};
use std::sync::Mutex;
use std::collections::HashMap;
use std::str::FromStr;

// Simple price tracking
static ORCA_LAST_PRICE: Mutex<f64> = Mutex::new(0.0);
static METEORA_LAST_PRICE: Mutex<f64> = Mutex::new(0.0);

// Meteora LB pool structure (simplified heuristic parser)
#[derive(Debug)]
struct MeteoraLBPool {
    pub reserve_x: u64,
    pub reserve_y: u64,
}

impl MeteoraLBPool {
    fn from_bytes(data: &[u8]) -> Result<Self, Box<dyn std::error::Error>> {
        if data.len() < 32 { return Err("Meteora pool data too short".into()); }

        // Heuristic: scan 8-byte aligned u64 pairs, derive price (USDC/SOL) and keep the pair
        // with a plausible price (1..1000) and highest geometric mean liquidity.
        let mut best: Option<(u64,u64,f64,f64,usize)> = None; // (x,y,price,geo_mean,offset)

        for i in (0..data.len().saturating_sub(15)).step_by(8) {
            if i + 15 >= data.len() { break; }
            let reserve_x = u64::from_le_bytes(data[i..i+8].try_into()?);
            let reserve_y = u64::from_le_bytes(data[i+8..i+16].try_into()?);
            if reserve_x == 0 || reserve_y == 0 { continue; }
            // Filter obviously huge numbers (likely not reserves)
            if reserve_x > 10_000_000_000_000 || reserve_y > 10_000_000_000_000 { continue; }

            let price = (reserve_y as f64 / 10f64.powi(USDC_DECIMALS as i32)) /
                        (reserve_x as f64 / 10f64.powi(SOL_DECIMALS as i32));
            if !(1.0..=1000.0).contains(&price) { continue; }
            let norm_x = reserve_x as f64 / 10f64.powi(SOL_DECIMALS as i32);
            let norm_y = reserve_y as f64 / 10f64.powi(USDC_DECIMALS as i32);
            let geo = (norm_x * norm_y).sqrt();
            match &best {
                Some((_,_,_,best_geo,_)) if geo <= *best_geo => {},
                _ => best = Some((reserve_x,reserve_y,price,geo,i)),
            }
        }

        if let Some((rx,ry,price,_geo,off)) = best {
            info!("[METEORA] Chosen reserves offset {} rx={} ry={} price={:.2}", off, rx, ry, price);
            Ok(MeteoraLBPool { reserve_x: rx, reserve_y: ry })
        } else {
            Err("No plausible reserves found".into())
        }
    }

    fn calculate_price(&self) -> f64 {
        if self.reserve_x == 0 || self.reserve_y == 0 { return 0.0; }
        (self.reserve_y as f64 / 10f64.powi(USDC_DECIMALS as i32)) /
        (self.reserve_x as f64 / 10f64.powi(SOL_DECIMALS as i32))
    }
}

fn format_price(p: f64) -> String { format!("{:.6}", p) }

fn log_arbitrage_opportunity() {
    let orca_price = *ORCA_LAST_PRICE.lock().unwrap();
    let meteora_price = *METEORA_LAST_PRICE.lock().unwrap();
    if orca_price > 0.0 && meteora_price > 0.0 {
        let price_diff = (meteora_price - orca_price).abs();
        let profit_usdc = price_diff * 1.0; // 1 SOL size
        info!("=== PRICE COMPARISON ===");
        info!("Orca USDC/SOL: {}", format_price(orca_price));
        info!("Meteora USDC/SOL: {}", format_price(meteora_price));
        info!("Price Difference: {:.6} USDC", price_diff);
        info!("Potential Profit (1 SOL): {:.6} USDC", profit_usdc);
        if profit_usdc > 1.0 { info!("Arb opportunity ( > 1 USDC )"); }
        info!("========================");
    }
}

// Main stream processing loop
pub async fn process_updates<S, T>(mut stream: S, tx: &mut T) -> Result<(), Box<dyn std::error::Error>>
where
    S: Stream<Item = Result<SubscribeUpdate, Status>> + Unpin,
    T: futures::sink::SinkExt<SubscribeRequest> + Unpin,
    <T as futures::Sink<SubscribeRequest>>::Error: std::error::Error + 'static,
{
    let mut accounts_filter = HashMap::new();
    accounts_filter.insert(
        "pools".to_string(),
        SubscribeRequestFilterAccounts {
            account: vec![ORCA_WHIRLPOOL_ADDRESS.to_string(), METEORA_POOL_ADDRESS.to_string()],
            owner: vec![],
            filters: vec![],
            nonempty_txn_signature: None,
        },
    );

    tx.send(SubscribeRequest {
        accounts: accounts_filter,
        transactions: HashMap::new(),
        commitment: Some(CommitmentLevel::Processed as i32),
        ..Default::default()
    }).await?;

    info!("Subscribed to Orca and Meteora pools");

    while let Some(message) = stream.next().await {
        match message {
            Ok(msg) => { if let Err(e) = handle_message(msg) { error!("Error handling message: {:?}", e); } }
            Err(e) => { error!("Stream error: {:?}", e); break; }
        }
    }
    Ok(())
}

fn compute_orca_price(state: &Whirlpool) -> f64 {
    // sqrt_price is Q64.64; ratio = (sqrt_price^2 / 2^128).
    let sqrt_price = state.sqrt_price as f64;
    let ratio = (sqrt_price * sqrt_price) / 2f64.powi(128);
    // Determine which mint is SOL / USDC to orient price as USDC per SOL.
    let mint_a = bs58::encode(state.token_mint_a).into_string();
    let mint_b = bs58::encode(state.token_mint_b).into_string();
    let dec_a = if mint_a == SOL_MINT { SOL_DECIMALS } else if mint_a == USDC_MINT { USDC_DECIMALS } else { 9 };
    let dec_b = if mint_b == SOL_MINT { SOL_DECIMALS } else if mint_b == USDC_MINT { USDC_DECIMALS } else { 9 };
    let ratio_adjusted = ratio * 10f64.powi(dec_b as i32 - dec_a as i32);

    // ratio_adjusted = token_b per token_a
    // We want USDC per SOL.
    let mut candidates = vec![];
    if mint_a == SOL_MINT && mint_b == USDC_MINT { // SOL -> USDC directly
        candidates.push(ratio_adjusted);
    } else if mint_a == USDC_MINT && mint_b == SOL_MINT { // need inverse
        candidates.push(1.0 / ratio_adjusted);
    } else {
        // Unknown ordering: consider both orientations, pick plausible USD price
        candidates.push(ratio_adjusted);
        if ratio_adjusted > 0.0 { candidates.push(1.0 / ratio_adjusted); }
    }
    // Choose candidate in plausible USD price range 1..1000 closest to 200 else fallback first.
    candidates.sort_by(|a,b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let mut best = candidates[0];
    let mut best_diff = f64::INFINITY;
    for c in &candidates {
        if (1.0..=1000.0).contains(c) {
            let d = (c - 200.0).abs();
            if d < best_diff { best_diff = d; best = *c; }
        }
    }
    best
}

fn handle_message(msg: SubscribeUpdate) -> Result<(), Box<dyn std::error::Error>> {
    let update = match msg.update_oneof { Some(u) => u, None => return Ok(()), };
    if let UpdateOneof::Account(account_update) = update {
        let account = match account_update.account { Some(a) => a, None => return Ok(()), };
        let account_pubkey = Pubkey::new_from_array(account.pubkey[..32].try_into().unwrap());
        let account_pubkey_str = account_pubkey.to_string();
        let account_data = &account.data;

        if account_pubkey_str == ORCA_WHIRLPOOL_ADDRESS {
            match Whirlpool::from_bytes(account_data) {
                Ok(whirlpool_state) => {
                    let price = compute_orca_price(&whirlpool_state);
                    if price > 0.0 {
                        *ORCA_LAST_PRICE.lock().unwrap() = price;
                        info!("[ORCA] Price: {} USDC/SOL", format_price(price));
                        log_arbitrage_opportunity();
                    }
                }
                Err(e) => error!("[ORCA PARSE ERROR] {}", e),
            }
        } else if account_pubkey_str == METEORA_POOL_ADDRESS {
            match MeteoraLBPool::from_bytes(account_data) {
                Ok(lb_pool) => {
                    let price = lb_pool.calculate_price();
                    if price > 0.0 {
                        *METEORA_LAST_PRICE.lock().unwrap() = price;
                        info!("[METEORA] Price: {} USDC/SOL (Reserve X: {}, Reserve Y: {})", format_price(price), lb_pool.reserve_x, lb_pool.reserve_y);
                        log_arbitrage_opportunity();
                    }
                }
                Err(e) => warn!("[METEORA PARSE WARN] {}", e),
            }
        }
    }
    Ok(())
}