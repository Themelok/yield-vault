use anyhow::{Result, anyhow};
use anchor_client::{
    Program,
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

    let program: Program<Rc<Keypair>> = get_program(kp.insecure_clone())?;

    let user_vault_pda: Pubkey = get_user_vault_pda(kp.pubkey());

    let kaminio_usdc_colateral_vault = get_associated_token_address(
        &user_vault_pda,
        &Pubkey::from_str_const(KLEND_COLLATERAL_MINT));

    let user_usdc_vault_ata = get_associated_token_address(
        &user_vault_pda, 
        &Pubkey::from_str_const(USDC_MINT));

    // Build and send instructions
    let tx = program.request().accounts(
        accounts::Initialize {
            user: kp.pubkey(),
            usdc_mint: Pubkey::from_str_const(USDC_MINT),
            user_vault_account: user_vault_pda,
            user_usdc_vault: user_usdc_vault_ata,
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
    println!("✅ Init Transaction signature: {}", signature.to_string());
    Ok(())
}


pub fn deposit(keypair_path: std::path::PathBuf, amount: u64) -> Result<()> {
    let kp = read_keypair_file(&keypair_path)
    .map_err(|e| anyhow!("could not read file `{}`: {}", keypair_path.display(), e))?;
    let public_key = kp.pubkey();
    println!("Deposit for Public key: {}", public_key.to_string());


    let program: Program<Rc<Keypair>> = get_program(kp.insecure_clone())?;

    let user_vault_pda: Pubkey = get_user_vault_pda(kp.pubkey());

    let user_usdc_ta = get_associated_token_address(
        &public_key,
        &Pubkey::from_str_const(USDC_MINT));

    println!("User USDC TA: {}", user_usdc_ta.to_string());

    let user_usdc_vault_ata = get_associated_token_address(
        &user_vault_pda, 
        &Pubkey::from_str_const(USDC_MINT));

    // Build and send instructions
    let tx = program.request().accounts(
        accounts::Deposit {
            user: public_key,
            usdc_mint: Pubkey::from_str_const(USDC_MINT),
            user_vault_account: user_vault_pda,
            user_usdc_ta: user_usdc_ta,
            user_usdc_vault_ata: user_usdc_vault_ata,
            token_program: spl_token::ID,
            associated_token_program: spl_associated_token_account::ID,
            system_program: system_program::ID,
        })
        .args(args::Deposit{amount:amount})
        .instructions()?
        .remove(0);

    let signature = program.request().instruction(tx).signer(kp).send()?;
    println!("✅ Deposit Transaction signature: {}", signature.to_string());
    Ok(())
}


pub fn withdraw(keypair_path: std::path::PathBuf, amount: u64) -> Result<()> {
    let kp = read_keypair_file(&keypair_path)
    .map_err(|e| anyhow!("could not read file `{}`: {}", keypair_path.display(), e))?;
    let public_key = kp.pubkey();
    println!("Withdraw for Public key: {}", public_key.to_string());
    let program: Program<Rc<Keypair>> = get_program(kp.insecure_clone())?;
    let user_vault_pda: Pubkey = get_user_vault_pda(kp.pubkey());

    let user_usdc_ta = get_associated_token_address(
        &public_key,
        &Pubkey::from_str_const(USDC_MINT));

    println!("User USDC TA: {}", user_usdc_ta.to_string());

    let user_usdc_vault_ata = get_associated_token_address(
        &user_vault_pda, 
        &Pubkey::from_str_const(USDC_MINT));

    // Build and send instructions
    let tx = program.request().accounts(
        accounts::Withdraw {
            user: public_key,
            usdc_mint: Pubkey::from_str_const(USDC_MINT),
            user_vault_account: user_vault_pda,
            user_usdc_ta: user_usdc_ta,
            user_usdc_vault_ata: user_usdc_vault_ata,
            token_program: spl_token::ID,
            associated_token_program: spl_associated_token_account::ID,
            system_program: system_program::ID,
        })
        .args(args::Withdraw{amount:amount})
        .instructions()?
        .remove(0);

    let signature = program.request().instruction(tx).signer(kp).send()?;
    println!("✅ Withdraw Transaction signature: {}", signature.to_string());
    Ok(())
}


fn get_user_vault_pda(user: Pubkey) -> Pubkey {
    let (user_vault_pda, _bump) = Pubkey::find_program_address(
        &[b"vault", user.as_ref()],
        &yield_vault::ID
    );
    user_vault_pda
}

fn get_program(kp: Keypair) -> Result<Program<Rc<Keypair>>> {
    Client::new_with_options(
        Cluster::Localnet,
        Rc::new(kp), 
        CommitmentConfig::confirmed())
        .program(yield_vault::ID)
        .map_err(|e| anyhow!("Failed to get program: {}", e))
}

fn get_user_usdc_ta(user: Pubkey) -> Pubkey {
    let (user_usdc_ta, _bump) = Pubkey::find_program_address(
        &[b"usdc_ta", user.as_ref()],
        &yield_vault::ID
    );
    user_usdc_ta
}
