use anchor_lang::prelude::*;
use anchor_client::solana_sdk::signature::Keypair;
use crate::rpc::Rpc;
use std::sync::Arc;


pub struct Config {
    pub program_id: Pubkey,
    pub bot_pubkey: Pubkey,
    pub strategy: Strategy,
    
}

#[derive(Debug, Clone)]
pub enum Strategy {
    Kamino(u64),
    Marginfi(u64),
}

#[derive(Clone)]
pub struct AppState {
    pub program_id: Pubkey,
    pub bot_pubkey: Pubkey,
    pub strategy: &'static Strategy,
    pub rpc: Arc<Rpc>,
}