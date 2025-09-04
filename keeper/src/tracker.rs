use std::time::Duration;
use anyhow::{Result, Context, anyhow};
use serde::Deserialize;
use tracing::{info, warn, error};
use anchor_lang::prelude::*;
use reqwest::Client;
use crate::marginfi_apy;

use crate::config::{AppState, Strategy};

// Tick every hour
const ONE_HOUR: Duration = Duration::from_secs(3600);

pub async fn bootstrap_once(app: AppState) {
    match tick_once(&app).await {
        Ok(_) => info!("tracker bootstrap complete"),
        Err(e) => error!(error=?e, "tracker bootstrap failed"),
    }
}


pub fn run_tracker(app: AppState) {
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(ONE_HOUR);
        interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

        loop {
            interval.tick().await;
            if let Err(e) = tick_once(&app).await {
                error!(error=?e, "tracker tick failed");
            }
        }
    });
}

// ---------- APY fetchers ----------

#[derive(Deserialize)]
struct KaminoHistory { history: Vec<KaminoPoint> }
#[derive(Deserialize)]
struct KaminoPoint { metrics: KaminoMetrics }
#[derive(Deserialize)]
struct KaminoMetrics {
    #[serde(default)]
    supplyInterestAPY: f64, // decimal (e.g., 0.035 for 3.5%)
}

// Build the Kamino URL based on your consts and a small lookback window
fn kamino_url() -> Result<String> {
    use chrono::{Utc, Duration as ChDur, SecondsFormat};
    let end = Utc::now();
    let start = end - ChDur::hours(12);
    Ok(format!(
        "https://api.kamino.finance/kamino-market/{}/reserves/{}/metrics/history?start={}&end={}",
        crate::consts::KLEND_MAIN_LENDING_MARKET,
        crate::consts::KLEND_USDC_RESEVE,
        start.to_rfc3339_opts(SecondsFormat::Secs, true),
        end.to_rfc3339_opts(SecondsFormat::Secs, true),
    ))
}

async fn kamino_supply_apy(client: &Client) -> Result<f64> {
    let url = kamino_url()?;
    let resp = client.get(&url)
        .timeout(Duration::from_secs(10))
        .send().await.context("kamino http")?
        .error_for_status().context("kamino 2xx")?;
    let data: KaminoHistory = resp.json().await.context("kamino json")?;
    let last = data.history.last().ok_or_else(|| anyhow!("kamino: empty history"))?;
    Ok(last.metrics.supplyInterestAPY)
}

// For now: fixed 1% APY on Marginfi;
async fn marginfi_supply_apy(client: &Client) -> Result<f64> {
    let bank_pk = crate::consts::MARGINFI_BANK;
    marginfi_apy::fetch_marginfi_supply_apy(client, bank_pk)
        .await
        .context("marginfi apy fetch")
}


async fn tick_once(app: &AppState) -> Result<()> {
    // log start of attempt
    info!("tracker: fetching APYs…");

    let client = Client::new();
    let kam_apy = kamino_supply_apy(&client).await.context("kamino apy")?;

    let mfi_apy = marginfi_supply_apy(&client).await.context("marginfi apy")?;


    info!(kam_apy = ?kam_apy, mfi_apy = ?mfi_apy, "tracker: APYs fetched");

    // Decide desired strategy
    // TODO: add logic to decide based on APYs, risk tolerance, transactions fees etc.
    let desired = if kam_apy > mfi_apy { Strategy::Kamino } else { Strategy::Marginfi };

    // Compare to current and flip if needed
    let mut lock = app.strategy.write().await;
    let current = *lock;
    info!(?current, ?desired, "tracker: decision");

    if desired != current {
        info!(?current, ?desired, "tracker: flipping strategy and rebalancing");

        // (Optional): unwind & redeploy for all tracked users
        let users: Vec<Pubkey> = app.lenders.read().await.iter().cloned().collect();

        // Unwind from current
        for u in &users {
            let res = if current == Strategy::Kamino {
                tokio::task::block_in_place(|| app.rpc.withdraw_from_kamino(*u))
            } else {
                tokio::task::block_in_place(|| app.rpc.withdraw_from_marginfi(*u))
            };
            if let Err(e) = res {
                warn!(user=%u, error=?e, "tracker: unwind failed");
            } else {
                info!(user=%u, "tracker: unwind ok");
            }
        }

        // Redeploy into desired
        for u in &users {
            // Determine amount to redeploy = vault ATA balance
            let (vault_pda, _) = crate::rpc::Rpc::vault_pda(u);
            let ata = crate::rpc::Rpc::ata(&vault_pda, &app.rpc.usdc_mint);
            let amount = app.rpc.spl_balance(ata).unwrap_or(0);
            if amount == 0 {
                warn!(user=%u, "tracker: no balance to redeploy");
                continue;
            }
            let res = if desired == Strategy::Kamino {
                tokio::task::block_in_place(|| app.rpc.deposit_to_kamino(*u, amount))
            } else {
                tokio::task::block_in_place(|| app.rpc.deposit_to_marginfi(*u, amount))
            };
            if let Err(e) = res {
                warn!(user=%u, error=?e, "tracker: redeploy failed");
            } else {
                info!(user=%u, amount, ?desired, "tracker: redeploy ok");
            }
        }

        *lock = desired;
        info!(?desired, "tracker: strategy updated");
    } else {
        info!("tracker: strategy unchanged");
    }

    Ok(())
}