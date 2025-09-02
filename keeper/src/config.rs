use anchor_lang::prelude::*;
use anchor_client::solana_sdk::signature::Keypair;
use crate::rpc::Rpc;
use std::sync::Arc;
use tokio::sync::RwLock;

pub struct Config {
    pub program_id: Pubkey,
    pub bot_pubkey: Pubkey,
    pub strategy: Strategy,
    
}

#[derive(Debug, Clone)]
pub enum Strategy {
    Kamino,
    Marginfi,
}

#[derive(Clone)]
pub struct AppState {
    pub program_id: Pubkey,
    pub bot_pubkey: Pubkey,
    pub strategy: Arc<RwLock<Strategy>>,
    pub rpc: Arc<Rpc>,
}

// pub type SharedState = Arc<RwLock<AppState>>;