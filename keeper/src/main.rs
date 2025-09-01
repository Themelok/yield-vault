use std::{fs, sync::Arc};
use anyhow::{Result, Context, anyhow};
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
    let config = Box::leak(Box::new(config::Config {
        program_id: yield_vault::ID,
        bot_pubkey: bot_pubkey,
        strategy: config::Strategy::Kamino(1000000000),
    }));

    let rpc = Arc::new(rpc::Rpc::new(bot_kp)?);

    tracing::info!(%bot_pubkey, program_id = %yield_vault::ID, "Keeper starting up");

    http::run_http(SocketAddr::from(([0, 0, 0, 0], 8080)), config, rpc).await?;

    Ok(())
}


fn get_kp() -> Result<Keypair> {
    let pattern = std::env::args().nth(1).unwrap_or_else(|| anyhow!("no keypair path given").to_string());
    let kp = read_keypair_file(&pattern)
        .map_err(|e| anyhow!("could not read file `{}`: {}", pattern, e))?;
    Ok(kp)
}
