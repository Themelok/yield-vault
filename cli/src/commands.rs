use anyhow::{Result, anyhow};
use anchor_client::{
    solana_sdk::{
        signature::{read_keypair_file, Keypair}, 
        signer::Signer,
        system_program,
        sysvar,
        pubkey::Pubkey,
        commitment_config::CommitmentConfig,
    },
    Client, Cluster
};
use spl_associated_token_account::get_associated_token_address;
use anchor_lang::prelude::*;
use std::rc::Rc;
use crate::consts::*;

declare_program!(yield_vault);
use yield_vault::{client::accounts, client::args};

pub fn init(keypair_path: std::path::PathBuf) -> Result<()> {
    let kp = read_keypair_file(&keypair_path)
    .map_err(|e| anyhow!("could not read file `{}`: {}", keypair_path.display(), e))?;
    let public_key = kp.pubkey();
    println!("Init for Public key: {}", public_key.to_string());

    let marginfi_account =  Keypair::new();
    println!("Marginfi account: {}", marginfi_account.pubkey().to_string());

    let provider = Client::new_with_options(
        Cluster::Localnet,
        Rc::new(kp.insecure_clone()), 
        CommitmentConfig::confirmed());

    let user_vault_pda = user_vault_pda(kp.pubkey());
    let program = provider.program(yield_vault::ID).map_err(|e| anyhow!("Failed to get program: {}", e))?;

    let kaminio_usdc_colateral_vault = get_associated_token_address(
        &user_vault_pda, 
        &Pubkey::from_str_const(KLEND_COLLATERAL_MINT));
    let user_usdc_vault = get_associated_token_address(
        &user_vault_pda, 
        &Pubkey::from_str_const(USDC_MINT));

    // Build and send instructions
    let tx = program.request().accounts(
        accounts::Initialize {
            user: kp.pubkey(),
            usdc_mint: Pubkey::from_str_const(USDC_MINT),
            user_vault_account: user_vault_pda,
            user_usdc_vault: user_usdc_vault,
            kamino_usdc_collateral_vault: kaminio_usdc_colateral_vault,
            kamino_usdc_collateral_mint: Pubkey::from_str_const(KLEND_COLLATERAL_MINT),
            marginfi_account: marginfi_account.pubkey(),
            marginfi_group: Pubkey::from_str_const(MARGINFI_GROUP),
            marginfi_program: Pubkey::from_str_const(MARGINFI_PROGRAM),
            system_program: system_program::ID,
            token_program: spl_token::ID,
            associated_token_program: spl_associated_token_account::ID,
            rent: sysvar::rent::ID,
        })
        .args(args::Initialize)
        .instructions()?
        .remove(0);

    let signature = program.request().instruction(tx).signer(kp).signer(marginfi_account).send()?;
    println!("Init Transaction signature: {}", signature.to_string());
    Ok(())
}

fn user_vault_pda(user: Pubkey) -> Pubkey {
    let (user_vault_pda, _bump) = Pubkey::find_program_address(
        &[b"vault", user.as_ref()],
        &yield_vault::ID
    );
    user_vault_pda
}