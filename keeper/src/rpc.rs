// keeper/src/rpc.rs
use anyhow::{anyhow, Context, Result};
use std::sync::Arc;

use anchor_client::{
    solana_sdk::{
     commitment_config::CommitmentConfig, pubkey::Pubkey, signature::Keypair, signer::Signer, system_program, sysvar
    }, Client, Cluster, Program
};
use anchor_lang::{prelude::*};
use tracing::info;
use spl_associated_token_account::get_associated_token_address;

use crate::{consts::*, yield_vault::accounts::UserVault};
declare_program!(yield_vault);
use yield_vault::{client::accounts, client::args};

// Convenient wrapper holding long-lived RPC + Anchor Program
pub struct Rpc {
    pub program: Program<Arc<Keypair>>,
    // parsed ids (from consts.rs)
    // Kamino
    pub usdc_mint: Pubkey,
    pub klend_program: Pubkey,
    pub klend_market: Pubkey,
    pub klend_market_auth: Pubkey,
    pub klend_reserve: Pubkey,
    pub klend_reserve_liq_supply: Pubkey,
    pub klend_collateral_mint: Pubkey,

    // Marginfi
    pub mfi_program: Pubkey,
    pub mfi_group: Pubkey,
    pub mfi_bank: Pubkey,
    pub mfi_bank_liq_vault: Pubkey,
    pub mfi_bank_liq_vault_auth: Pubkey,

    // Bot(Keeper) credentials
    pub bot_pubkey: Pubkey,
    pub bot_kp: Keypair,
}


impl Rpc {
    pub fn new(bot_kp: Keypair) -> Result<Self> {
        let bot_pubkey = bot_kp.pubkey();
        let program = Client::new_with_options(
            Cluster::Localnet,
            Arc::new(bot_kp.insecure_clone()),
            CommitmentConfig::confirmed()
        ).program(yield_vault::ID)?;

        let usdc_mint = Pubkey::from_str_const(USDC_MINT);
        let klend_program = Pubkey::from_str_const(KLEND_PROGRAM);
        let klend_market = Pubkey::from_str_const(KLEND_MAIN_LENDING_MARKET);
        let klend_market_auth = Pubkey::from_str_const(KLEND_LENDING_MARKET_AUTHORITY);
        let klend_reserve = Pubkey::from_str_const(KLEND_USDC_RESEVE);
        let klend_reserve_liq_supply = Pubkey::from_str_const(KLEND_RESERVE_LIQUIDITY_SUPPLY);
        let klend_collateral_mint = Pubkey::from_str_const(KLEND_COLLATERAL_MINT);
        
        let mfi_program = Pubkey::from_str_const(MARGINFI_PROGRAM);
        let mfi_group = Pubkey::from_str_const(MARGINFI_GROUP);
        let mfi_bank = Pubkey::from_str_const(MARGINFI_BANK);
        let mfi_bank_liq_vault = Pubkey::from_str_const(MARGINFI_BANK_USDC_LIQUIDITY_VAULT);
        let mfi_bank_liq_vault_auth = Pubkey::from_str_const(MARGINFI_BANK_USDC_LIQUIDITY_VAULT_AUTH);


        Ok(Self { 
            program,
            usdc_mint,
            klend_program,
            klend_market,
            klend_market_auth,
            klend_reserve,
            klend_reserve_liq_supply,
            klend_collateral_mint,
            mfi_program,
            mfi_group,
            mfi_bank,
            mfi_bank_liq_vault,
            mfi_bank_liq_vault_auth,
            bot_pubkey,
            bot_kp,
        })
    }
    pub fn vault_pda(user: &Pubkey) -> (Pubkey, u8) {
        Pubkey::find_program_address(&[b"vault", user.as_ref()], &yield_vault::ID)
    }

    pub fn ata(owner: &Pubkey, mint: &Pubkey) -> Pubkey {
        get_associated_token_address(owner, mint)
    }
    pub fn withdraw_from_marginfi(&self, user: Pubkey) -> Result<String> {
        info!(%user, "withdrawing from marginfi for");
        let (vault_pda, _bump) = Self::vault_pda(&user);
        let vault_usdc_ata = Self::ata(&vault_pda, &self.usdc_mint);

        let vault_pda_acc: UserVault = self.program.account(vault_pda)?;
        let marginfi_account = vault_pda_acc.marginfi_account;
        info!(%marginfi_account, %vault_pda, "marginfi account from vault");
        let accounts = accounts::RedeemUsdcMarginfi{
            keeper: self.bot_pubkey,
            user: user,
            usdc_mint: self.usdc_mint,
            user_vault_account: vault_pda,
            user_usdc_vault_ata: vault_usdc_ata,
            // marginfi CPI accounts
            marginfi_program: self.mfi_program,
            marginfi_group: self.mfi_group,
            marginfi_account: vault_pda_acc.marginfi_account, // or the stored marginfi_account pubkey from your vault
            marginfi_bank: self.mfi_bank,
            marginfi_bank_liquidity_vault_authority: self.mfi_bank_liq_vault_auth,
            marginfi_bank_liquidity_vault: self.mfi_bank_liq_vault,
            token_program: spl_token::id(),
        };
        let tx = self.program
            .request()
            .accounts(accounts)
            .args(args::RedeemUsdcMarginfi).instructions()?.remove(0);

        let signature = self.program
            .request()
            .instruction(tx)
            .signer(self.bot_kp.insecure_clone())
            .send()?;
        Ok(signature.to_string())
    }

    pub fn deposit_to_marginfi(&self, user: Pubkey, amount: u64) -> Result<String> {
        info!(%user, "deposing to marginfi for");
        if amount == 0 {
            return Err(anyhow!("amount must be > 0"));
        }
        let (vault_pda, _bump) = Self::vault_pda(&user);
        let vault_usdc_ata = Self::ata(&vault_pda, &self.usdc_mint);
        let vault_pda_acc: UserVault = self.program.account(vault_pda)?;
        let marginfi_account = vault_pda_acc.marginfi_account;
        info!(%marginfi_account, %vault_pda, "marginfi account from vault");

        let accounts = accounts::DeployUsdcMarginfi{
            keeper: self.bot_pubkey,
            user: user,
            usdc_mint: self.usdc_mint,
            user_vault_account: vault_pda,
            user_usdc_vault_ata: vault_usdc_ata,
            // marginfi CPI accounts
            marginfi_program: self.mfi_program,
            marginfi_group: self.mfi_group,
            marginfi_account: vault_pda_acc.marginfi_account,
            marginfi_bank: self.mfi_bank,
            
            marginfi_bank_liquidity_vault: self.mfi_bank_liq_vault,
            token_program: spl_token::id(),
            system_program: system_program::ID,
            associated_token_program: spl_associated_token_account::id(),
        };

        let tx = self.program.
            request()
            .accounts(accounts)
            .args(args::DeployUsdcMarginfi{amount: amount})
            .instructions()?
            .remove(0);

        let signature = self.program
            .request()
            .instruction(tx)
            .signer(self.bot_kp.insecure_clone())
            .send()?;
            Ok(signature.to_string())

    }

    pub fn withdraw_from_kamino(&self, user: Pubkey) -> Result<String> {
        info!(%user, "withdrawing from KLend for");
        let (vault_pda, _bump) = Self::vault_pda(&user);
        let vault_usdc_ata = Self::ata(&vault_pda, &self.usdc_mint);
        let vault_k_collateral_ata = Self::ata(&vault_pda, &self.klend_collateral_mint);
        let accounts = accounts::RedeemUsdcKaminio {
            keeper: self.bot_pubkey,
            usdc_mint: self.usdc_mint,
            user,
            user_vault_account: vault_pda,
            user_usdc_vault_ata: vault_usdc_ata,
            // Kamino
            kamino_program: self.klend_program,
            kamino_lending_market: self.klend_market,
            kamino_lending_market_authority: self.klend_market_auth,
            kamino_reserve: self.klend_reserve,
            kamino_reserve_liquidity_supply: self.klend_reserve_liq_supply,
            kamino_usdc_collateral_mint: self.klend_collateral_mint,
            kamino_usdc_collateral_vault: vault_k_collateral_ata,
            // Built-ins
            associated_token_program: spl_associated_token_account::id(),
            system_program: system_program::ID,
            token_program: spl_token::id(),
            rent: sysvar::rent::ID,
            instruction_sysvar_account: sysvar::instructions::ID,
        };
        let tx = self.program.request().accounts(accounts).args(args::RedeemUsdcKaminio).instructions()?.remove(0);
        let signature = self.program
        .request()
        .instruction(tx)
        .signer(self.bot_kp.insecure_clone())
        .send()?;
        Ok(signature.to_string())
    }

    pub fn deposit_to_kamino(&self, user: Pubkey, amount: u64) -> Result<String> {
        info!(%user, "deposing to Klend for");
        if amount == 0 {
            return Err(anyhow!("amount must be > 0"));
        }
        let (vault_pda, _bump) = Self::vault_pda(&user);
        let vault_usdc_ata = Self::ata(&vault_pda, &self.usdc_mint);
        let vault_k_collateral_ata = Self::ata(&vault_pda, &self.klend_collateral_mint);

      // Build accounts matching your on-chain `TransferUsdcKamino` struct
      let accounts = accounts::DeployUsdcKamino {
        keeper: self.bot_pubkey,
        usdc_mint: self.usdc_mint,
        user,
        user_vault_account: vault_pda,
        user_usdc_vault_ata: vault_usdc_ata,
        // Kamino
        kamino_program: self.klend_program,
        kamino_lending_market: self.klend_market,
        kamino_lending_market_authority: self.klend_market_auth,
        kamino_reserve: self.klend_reserve,
        kamino_reserve_liquidity_supply: self.klend_reserve_liq_supply,
        kamino_usdc_collateral_mint: self.klend_collateral_mint,
        kamino_usdc_collateral_vault: vault_k_collateral_ata,
        // Built-ins
        associated_token_program: spl_associated_token_account::id(),
        system_program: system_program::ID,
        token_program: spl_token::id(),
        rent: sysvar::rent::ID,
        instruction_sysvar_account: sysvar::instructions::ID,
    };

    let tx = self.program.request().accounts(accounts).args(args::DeployUsdcKamino{amount: amount}).instructions()?.remove(0);

    let signature = self.program
    .request()
    .instruction(tx)
    .signer(self.bot_kp.insecure_clone())
    .send()?;
    Ok(signature.to_string())

    }
}