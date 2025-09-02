use std::{sync::Arc, collections::HashSet};
use anyhow::{Result, anyhow};
use anchor_lang::prelude::*;
use anchor_client::solana_sdk::{
        signature::{read_keypair_file, Keypair}, signer::Signer,
    };
use std::net::SocketAddr;
use tracing_subscriber::{fmt, EnvFilter};



declare_program!(yield_vault);
use yield_vault::{client::accounts, client::args};
mod http;
mod config;
mod consts;
mod rpc;
mod tracker;
mod marginfi_apy;



#[tokio::main]
async fn main() -> Result<()> {
    let filter = EnvFilter::try_from_default_env()
    .unwrap_or_else(|_| EnvFilter::new("info"));
    fmt()
    .with_env_filter(filter)
    .with_writer(std::io::stdout)
    .compact()
    .init();
    let bot_kp = get_kp()?;
    let bot_pubkey = bot_kp.pubkey();

    let config = Box::leak(Box::new(config::AppState {
        program_id: yield_vault::ID,
        bot_pubkey: bot_pubkey,
        strategy: Arc::new(tokio::sync::RwLock::new(config::Strategy::Marginfi)),
        rpc:  Arc::new(rpc::Rpc::new(bot_kp)?),
        lenders: Arc::new(tokio::sync::RwLock::new(HashSet::new())), 
    }));

    tracing::info!(%bot_pubkey, program_id = %yield_vault::ID, "Keeper starting up");


    // 1) One-shot: compute APYs and set initial strategy at startup
    tracker::bootstrap_once(config.clone()).await;

    // 2) Background: hourly tracker loop
    tracker::run_tracker(config.clone());

    http::run_http(SocketAddr::from(([0, 0, 0, 0], 8080)), config.clone()).await?;

    Ok(())
}


fn get_kp() -> Result<Keypair> {
    let pattern = std::env::args().nth(1).unwrap_or_else(|| anyhow!("no keypair path given").to_string());
    let kp = read_keypair_file(&pattern)
        .map_err(|e| anyhow!("could not read file `{}`: {}", pattern, e))?;
    Ok(kp)
}
