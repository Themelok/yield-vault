use anyhow::Result;
use axum::{Json, extract::State, routing::{get, post}, Router};
use axum::http::StatusCode;
use serde::{Serialize, Deserialize};
use tracing::info;
use tokio::net::TcpListener;
use anchor_lang::prelude::*;

use std::net::SocketAddr;
use std::sync::Arc;

use crate::config;
use crate::rpc::Rpc;


#[derive(Serialize)]
struct Health { 
    ok: bool,
    service: &'static str,
    bot_kp: String,
    program_id: String,
    strategy: String,
}


// request/response payloads
#[derive(Deserialize)]
struct DepositReq {
    user: String,   // base58 pubkey
    amount: u64,    // in USDC base units (6 decimals)
}

#[derive(Serialize)]
struct DepositResp {
    ok: bool,
    tx: String,
    user: String,
    vault: String,
    protocol: String,
    requested: u64,
}

#[derive(Deserialize)]
struct WithdrawReq {
    user: String,
}

#[derive(Serialize)]
struct WithdrawResp {
    ok: bool,
    tx: String,
    user: String,
}


async fn health(State(st): State<config::AppState>) -> Json<Health> {
    info!("Health Check");

    let state_clone = st.clone();
    let strat = state_clone.strategy.read().await;
    match *strat {
        config::Strategy::Kamino   => {
            Json(Health {
                ok: true,
                service: "keeper_kamino",
                bot_kp: st.bot_pubkey.to_string(),
                program_id: st.program_id.to_string(),
                strategy: format!("{:?}", *strat),
            })
        },

       config::Strategy::Marginfi =>  {
        Json(Health {
            ok: true,
            service: "keeper_marginfi",
            bot_kp: st.bot_pubkey.to_string(),
            program_id: st.program_id.to_string(),
            strategy: format!("{:?}", *strat),
        })
       }
    }
}

async fn withdraw(
    State(st): State<config::AppState>, 
    Json(req): Json<WithdrawReq>) -> Result<Json<WithdrawResp>, (StatusCode, String)>  {

    let strat = st.strategy.read().await;
    let user: Pubkey = req.user.parse()
        .map_err(|e| (StatusCode::BAD_REQUEST, format!("invalid user pubkey: {e}")))?;
    info!("Making Withdraw RPC call..");
    let sig = tokio::task::block_in_place(|| {
        match *strat {
            config::Strategy::Kamino   => st.rpc.withdraw_from_kamino(user),
            config::Strategy::Marginfi => st.rpc.withdraw_from_marginfi(user),
        }
    })
    .map_err(|e: anyhow::Error| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(Json(WithdrawResp { ok: true, tx: sig, user: user.to_string() }))
}

// POST /deposit handler
async fn deposit(
    State(st): State<config::AppState>, 
    Json(req): Json<DepositReq>) -> Result<Json<DepositResp>, (StatusCode, String)>  {
 // basic validation
    if req.amount == 0 {
        return Err((StatusCode::BAD_REQUEST, "amount must be > 0".into()));
    }
    let user: Pubkey = req.user.parse()
        .map_err(|e| (StatusCode::BAD_REQUEST, format!("invalid user pubkey: {e}")))?;
    
    info!("Making Deposit RPC call..");
    let strat = st.strategy.read().await;
    // http.rs (inside POST /deposit handler)
    let sig = tokio::task::block_in_place(|| {
        match *strat {
            config::Strategy::Kamino   => st.rpc.deposit_to_kamino(user, req.amount),
            config::Strategy::Marginfi => st.rpc.deposit_to_marginfi(user, req.amount),
        }
    })
    .map_err(|e: anyhow::Error| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let (vault, _) = crate::rpc::Rpc::vault_pda(&user);

    Ok(Json(DepositResp {
        ok: true,
        tx: sig,
        user: user.to_string(),
        vault: vault.to_string(),
        protocol: format!("{:?}", st.strategy),
        requested: req.amount,
    }))
}

pub async fn run_http(
    addr: SocketAddr, 
    app_state: config::AppState) -> Result<()> {

    let app = Router::new()
    .route("/health", get(health))
    .route("/deposit", post(deposit))
    .route("/withdraw", post(withdraw))
    .with_state(app_state);
    


    info!(%addr, "Starting HTTP server");
    let listener = TcpListener::bind(&addr).await?;
    axum::serve(listener, app.into_make_service())
        .with_graceful_shutdown(shutdown_signal())
        .await?;
    Ok(())
}

async fn shutdown_signal() {
    let _ = tokio::signal::ctrl_c().await;
    info!("Received Ctrl-C, shutting down HTTP server");
}