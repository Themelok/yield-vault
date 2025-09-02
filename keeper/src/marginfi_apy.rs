use anyhow::{Context, Result};
use reqwest::Client;
use rust_decimal::prelude::*;
use rust_decimal::Decimal;
use serde::Deserialize;


const SCALE: i128 = 1_i128 << 48;

#[derive(Deserialize)]
struct RawBank {
    data: BankData,
}
#[derive(Deserialize)]
struct BankData {
    #[serde(rename = "assetShareValue")]
    asset_share_value: I80,
    #[serde(rename = "liabilityShareValue")]
    liab_share_value: I80,
    #[serde(rename = "totalAssetShares")]
    total_asset_shares: I80,
    #[serde(rename = "totalLiabilityShares")]
    total_liab_shares: I80,
    config: Config,
}
#[derive(Deserialize)]
struct Config {
    #[serde(rename = "interestRateConfig")]
    ir: IRConfig,
}
#[derive(Deserialize)]
struct IRConfig {
    #[serde(rename = "optimalUtilizationRate")]
    u_opt: I80,
    #[serde(rename = "plateauInterestRate")]
    plateau_apr: I80,
    #[serde(rename = "maxInterestRate")]
    max_apr: I80,

    #[serde(rename = "protocolFixedFeeApr")]
    protocol_fixed_fee_apr: I80,
    #[serde(rename = "protocolIrFee")]
    protocol_ir_fee: I80,

    #[serde(default, rename = "insuranceIrFee")]
    insurance_ir_fee: Option<I80>,
    #[serde(default, rename = "insuranceFeeFixedApr")]
    insurance_fixed_fee_apr: Option<I80>,
}
#[derive(Deserialize)]
struct I80 { value: [u8; 16] }

fn i80f48_to_decimal(v: &I80) -> Decimal {
    let raw = i128::from_le_bytes(v.value);
    Decimal::from_i128_with_scale(raw, 0) / Decimal::from_i128_with_scale(SCALE, 0)
}

fn apr_to_apy(apr: Decimal, periods_per_year: i64) -> Decimal {
    if apr.is_zero() { return Decimal::ZERO; }
    let n = Decimal::from_i64(periods_per_year).unwrap();
    (Decimal::ONE + (apr / n)).powd(n) - Decimal::ONE
}

fn borrow_curve(u: Decimal, u_opt: Decimal, plateau: Decimal, maxr: Decimal) -> Decimal {
    if u <= Decimal::ZERO { return Decimal::ZERO; }
    if u <= u_opt {
        return plateau * (u / u_opt);
    }
    if u >= Decimal::ONE {
        return maxr;
    }
    let span = Decimal::ONE - u_opt;
    plateau + (maxr - plateau) * ((u - u_opt) / span)
}

async fn fetch_raw_bank(client: &Client, bank_addr: &str) -> Result<BankData> {
    // Public endpoint the Marginfi docs/apps use
    // Example shown in their docs & app surfaces “Bank” parameters and share values.
    let url = format!(
        "https://app.marginfi.com/api/bankData/rawBankData?addresses={}",
        bank_addr
    );
    // let client = Client::new();
    let resp = client.get(&url)
        .send().await.context("http send")?
        .error_for_status().context("http 2xx")?;

    // API can return `[ { data: BankData, ... } ]` or `{ data: BankData }`
    // Try list first, then object.
    if resp.headers().get("content-type").and_then(|v| v.to_str().ok())
        .unwrap_or("").starts_with("application/json")
    {
        let txt = resp.text().await?;
        if txt.trim_start().starts_with('[') {
            let mut arr: Vec<RawBank> = serde_json::from_str(&txt)?;
            Ok(arr.remove(0).data)
        } else {
            let obj: RawBank = serde_json::from_str(&txt)?;
            Ok(obj.data)
        }
    } else {
        anyhow::bail!("unexpected content-type");
    }
}

fn compute_supply_rates(bank: &BankData) -> (Decimal, Decimal, Decimal) {
    // decode
    let u_opt      = i80f48_to_decimal(&bank.config.ir.u_opt);
    let plateau    = i80f48_to_decimal(&bank.config.ir.plateau_apr);
    let maxr       = i80f48_to_decimal(&bank.config.ir.max_apr);

    let prot_fix   = i80f48_to_decimal(&bank.config.ir.protocol_fixed_fee_apr);
    let prot_rate  = i80f48_to_decimal(&bank.config.ir.protocol_ir_fee);
    let ins_rate   = bank.config.ir.insurance_ir_fee.as_ref().map(i80f48_to_decimal).unwrap_or(Decimal::ZERO);
    let ins_fix    = bank.config.ir.insurance_fixed_fee_apr.as_ref().map(i80f48_to_decimal).unwrap_or(Decimal::ZERO);

    let a_share    = i80f48_to_decimal(&bank.asset_share_value);
    let l_share    = i80f48_to_decimal(&bank.liab_share_value);
    let tot_a_sh   = i80f48_to_decimal(&bank.total_asset_shares);
    let tot_l_sh   = i80f48_to_decimal(&bank.total_liab_shares);

    // values (NOT raw shares)
    let total_assets = tot_a_sh * a_share;
    let total_liabs  = tot_l_sh * l_share;
    let u = if total_assets.is_zero() { Decimal::ZERO } else { total_liabs / total_assets };

    // base curve apr
    let base = borrow_curve(u, u_opt, plateau, maxr);

    // UI simple APRs per SDK parity
    let _borrow_apr = base * (Decimal::ONE + prot_rate + ins_rate) + (prot_fix + ins_fix);
    let supply_apr  = base * u;

    // For comparison only: daily-compounded APY
    let supply_apy = apr_to_apy(supply_apr, 365);

    (u, supply_apr, supply_apy)
}

pub async fn fetch_marginfi_supply_apy(client: &reqwest::Client, bank_addr: &str) -> Result<f64> {
    let bank = fetch_raw_bank(client, bank_addr).await?;
    let (u, _supply_apr, supply_apy) = compute_supply_rates(&bank);
    Ok(supply_apy.to_f64().unwrap_or(0.0))
}

#[tokio::test]
async fn prints_marginfi_supply_apy_standalone() -> Result<()> {
    let client = Client::new();
    // let bank = fetch_raw_bank(&client, "2s37akK2eyBbp8DZgCm7RtsaEz8eJP3Nxd4urLHQv7yB").await?;
    let supply_apy: f64 = fetch_marginfi_supply_apy(&client, "2s37akK2eyBbp8DZgCm7RtsaEz8eJP3Nxd4urLHQv7yB").await?;

    // println!("Utilization (U):         {}", u);
    // println!("Supply APR (simple):     {} %", supply_apr * dec!(100));
    println!("Supply APY (daily comp): {} %", supply_apy * 100.0);
    Ok(())
}