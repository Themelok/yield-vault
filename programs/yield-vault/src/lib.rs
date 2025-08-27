use anchor_lang::prelude::*;
use anchor_spl::token::{
    Mint, 
    Token, 
    TokenAccount,
    Transfer, 
    transfer
};
use anchor_lang::solana_program::sysvar::instructions as sysvar_instructions;
use anchor_spl::associated_token::AssociatedToken;
use kamino_lend::cpi as kamino_cpi;
use kamino_lend::program::KaminoLending;

use marginfi_cpi_local::program::Marginfi;
use marginfi_cpi_local::cpi::accounts as mfi_accounts; 
use marginfi_cpi_local::cpi as mfi_cpi;              


declare_id!("5urWt3YZS2aXYPhr7LbkQxTHB9o9FDPevV8N1PEeYkYu");

// Most lending protocols (Kamino included) define:
// •	deposit_reserve_liquidity(amount) → amount in liquidity units (USDC)
// •	redeem_reserve_collateral(amount) → amount in collateral units (kUSDC)
#[program]
pub mod yield_vault {

    use super::*;

    pub fn initialize(ctx: Context<Initialize>) -> Result<()> {
        let vault = &mut ctx.accounts.vault_account;
        let owner = ctx.accounts.owner.key();
        vault.owner = owner;
        vault.bump = ctx.bumps.vault_account;
        // vault.usdc_vault = ctx.accounts.usdc_vault.key();



// marginfi_group
// marginfi_account // Signer // isMut
// authority // Signer
// fee_payer // Signer // isMut
// system_program

        let seeds = [
            VAULT_SEED,
            ctx.accounts.owner.to_account_info().key.as_ref(),
            &[ctx.accounts.vault_account.bump],
        ];
        let signer: &[&[&[u8]]] = &[&seeds];
        
        let cpi_accounts = mfi_accounts::MarginfiAccountInitialize {
            marginfi_group:   ctx.accounts.marginfi_group.to_account_info(),
            marginfi_account: ctx.accounts.marginfi_account.to_account_info(), // fresh Keypair (outer signer)
            authority:        ctx.accounts.vault_account.to_account_info(),    // PDA authority
            fee_payer:        ctx.accounts.owner.to_account_info(),
            system_program:   ctx.accounts.system_program.to_account_info(),
        };
        
        let cpi_ctx = CpiContext::new_with_signer(
            ctx.accounts.marginfi_program.to_account_info(),
            cpi_accounts,
            signer,
        );
        mfi_cpi::marginfi_account_initialize(cpi_ctx)?;

        ctx.accounts.vault_account.marginfi_account = ctx.accounts.marginfi_account.key();
     
        msg!("Vault initialized for owner: {}", owner.to_string());
        Ok(())
    }

    pub fn deposit_usdc(ctx: Context<TransferUsdc>,  amount: u64) -> Result<()> {
        // Step 1: Transfer USDC from user to our usdc vault ATA
        let vault_deposit_accounts = Transfer {
            from:ctx.accounts.owner_usdc_account.to_account_info(),
            to: ctx.accounts.usdc_vault.to_account_info(),
            authority: ctx.accounts.owner.to_account_info(),
        };

        let cpi_context = CpiContext::new(
            ctx.accounts.token_program.to_account_info(),
            vault_deposit_accounts,
        );
        transfer(cpi_context, amount)?;
        msg!("Deposited {} USDC to vault {} of owner {}", amount, ctx.accounts.usdc_vault.key(), ctx.accounts.owner.key().to_string());

        // Step 2: CPI to deposit from our vault into Kamino
        // TODO: make a choice between lending protocols here when marginfi will be supported
        let seeds = [VAULT_SEED, ctx.accounts.owner.to_account_info().key.as_ref(), &[ctx.accounts.vault_account.bump]];
        let signer: &[&[&[u8]]; 1] = &[&seeds[..]];

        let cpi_deposit_accounts = kamino_cpi::accounts::DepositReserveLiquidity {
            owner:                          ctx.accounts.vault_account.to_account_info(),
            reserve:                        ctx.accounts.kamino_reserve.to_account_info(),
            // Lending Market accounts
            lending_market:                 ctx.accounts.kamino_lending_market.to_account_info(),
            lending_market_authority:       ctx.accounts.kamino_lending_market_authority.to_account_info(),
            // Reserve accounts
            reserve_liquidity_mint:         ctx.accounts.usdc_mint.to_account_info(),
            reserve_liquidity_supply:       ctx.accounts.kamino_reserve_liquidity_supply.to_account_info(),
            reserve_collateral_mint:        ctx.accounts.kamino_usdc_collateral_mint.to_account_info(), // TODO: check if this is correct
            // User accounts
            user_source_liquidity:          ctx.accounts.usdc_vault.to_account_info(),
            user_destination_collateral:    ctx.accounts.kamino_usdc_collateral_vault.to_account_info(),
            // Token programs
            collateral_token_program:       ctx.accounts.token_program.to_account_info(),
            liquidity_token_program:        ctx.accounts.token_program.to_account_info(),
            instruction_sysvar_account:     ctx.accounts.instruction_sysvar_account.to_account_info(),
        };
        let cpi_ctx_kamino = CpiContext::new_with_signer(
            ctx.accounts.kamino_program.to_account_info(),
            cpi_deposit_accounts,
            signer,
        );
        kamino_cpi::deposit_reserve_liquidity(cpi_ctx_kamino, amount)?;


        // let marginfi_cpi_accounts = mfi_accounts::LendingAccountDeposit{

        // }

        Ok(())
    }

    // TODO: Treat the parameter as desired USDC - need to convert it to collateral using the reserve exchange rate on-chain (requires reading reserve state/slot math)
    pub fn withdraw_usdc(ctx: Context<TransferUsdc>, collateral_amount: u64) -> Result<()> {
        let seeds = [VAULT_SEED, ctx.accounts.owner.to_account_info().key.as_ref(), &[ctx.accounts.vault_account.bump]];
        let signer: &[&[&[u8]]; 1] = &[&seeds[..]];
        // Step 1: CPI to withdraw from Kamino into our vault
        // TODO: Make a choice between lending protocols here when marginfi will be supported
        let pre_liquidity = ctx.accounts.usdc_vault.amount;

        let cpi_accounts_kamino = kamino_cpi::accounts::RedeemReserveCollateral {
            owner:                         ctx.accounts.vault_account.to_account_info(), // PDA
            
            reserve:                       ctx.accounts.kamino_reserve.to_account_info(),
            lending_market:                ctx.accounts.kamino_lending_market.to_account_info(),
            lending_market_authority:      ctx.accounts.kamino_lending_market_authority.to_account_info(),
            // Reserve accounts
            reserve_liquidity_mint:        ctx.accounts.usdc_mint.to_account_info(),
            reserve_liquidity_supply:      ctx.accounts.kamino_reserve_liquidity_supply.to_account_info(),
            reserve_collateral_mint:       ctx.accounts.kamino_usdc_collateral_mint.to_account_info(),
            // User accounts
            user_source_collateral:        ctx.accounts.kamino_usdc_collateral_vault.to_account_info(),
            user_destination_liquidity:    ctx.accounts.usdc_vault.to_account_info(),

            collateral_token_program:      ctx.accounts.token_program.to_account_info(),
            liquidity_token_program:       ctx.accounts.token_program.to_account_info(),
            instruction_sysvar_account:    ctx.accounts.instruction_sysvar_account.to_account_info(),
        };
        let cpi_ctx_kamino = CpiContext::new_with_signer(
            ctx.accounts.kamino_program.to_account_info(),
            cpi_accounts_kamino,
            signer,
        );
        kamino_cpi::redeem_reserve_collateral(cpi_ctx_kamino, collateral_amount)?;

        ctx.accounts.usdc_vault.reload()?;
        let post_liquidity = ctx.accounts.usdc_vault.amount;
        let received_liquidity = post_liquidity.checked_sub(pre_liquidity).ok_or(ProgramError::InvalidAccountData)?; 
        // Early exit if nothing arrived (shouldn’t happen, but safe)
        require!(received_liquidity > 0, ErrorCode::NothingRedeemed);

        // Step 2: Transfer USDC from our vault back to the user usdc token account
        let vault_withdraw_accounts = Transfer {
            from: ctx.accounts.usdc_vault.to_account_info(),
            to: ctx.accounts.owner_usdc_account.to_account_info(),
            authority: ctx.accounts.vault_account.to_account_info(),
        };


        let cpi_context = CpiContext::new_with_signer(
            ctx.accounts.token_program.to_account_info(),
            vault_withdraw_accounts,
            signer);
        transfer(cpi_context, received_liquidity)?;
        msg!("Withdrew {} USDC from vault {} of owner {}", received_liquidity, ctx.accounts.usdc_vault.key(), ctx.accounts.owner.key().to_string());

        Ok(())
    }
}


#[derive(Accounts)]
pub struct Initialize<'info>{
    #[account(mut)]
    pub owner: Signer<'info>,
    pub usdc_mint: Account<'info, Mint>,


   
    #[account(
        init,
        payer = owner,
        space = Vault::LEN,
        seeds = [VAULT_SEED, owner.key().as_ref()],
        bump,
    )]
    pub vault_account: Account<'info, Vault>,

    #[account(
        init,
        payer = owner,
        associated_token::mint = usdc_mint,
        associated_token::authority = vault_account,
    )]
    pub usdc_vault: Account<'info, TokenAccount>,


    // Kamino Specific Accounts:
    pub kamino_usdc_collateral_mint: Account<'info, Mint>,
    #[account(
        init,
        payer = owner,
        associated_token::mint = kamino_usdc_collateral_mint,
        associated_token::authority = vault_account,
    )]
    pub kamino_usdc_collateral_vault: Account<'info, TokenAccount>,


    // Marginfi Specific Accounts:
    // pub marginfi_program: Program<'info, Marginfi>,
    /// CHECK: Marginfi group account
   /// CHECK: marginfi group (owner checked)
    #[account(owner = Marginfi::id())]
    pub marginfi_group: UncheckedAccount<'info>,
    #[account(mut)]
    pub marginfi_account: Signer<'info>,
    pub marginfi_program: Program<'info, Marginfi>,


    // BUILT-IN ACCOUNTS:
    pub system_program: Program<'info, System>,
    pub token_program: Program<'info, Token>,
    pub associated_token_program: Program<'info, AssociatedToken>,
    pub rent: Sysvar<'info, Rent>,
}

#[derive(Accounts)]
pub struct TransferUsdc<'info> {
    pub usdc_mint: Account<'info, Mint>,
    
    #[account(mut)]
    pub owner: Signer<'info>,
    #[account(mut, constraint = owner_usdc_account.mint == usdc_mint.key(), constraint = owner_usdc_account.owner == owner.key())]
    pub owner_usdc_account: Account<'info, TokenAccount>,

    #[account(
        mut,
        associated_token::mint = usdc_mint,
        associated_token::authority = vault_account,
    )]
    pub usdc_vault: Account<'info, TokenAccount>,

    #[account(seeds = [VAULT_SEED, owner.key().as_ref()], bump = vault_account.bump)]
    pub vault_account: Account<'info, Vault>,

     // -------- Kamino (Lend) specific: BEGIN --------
     /// MNT: KLend2g3cP87fffoy8q1mQqGKjrxjC8boSyAYavgmjD
    pub kamino_program: Program<'info, KaminoLending>,
    /// CHECK: Kamino's lending market account
    /// MNT: 7u3HeHxYDLhnCoErrtycNokbQYbWGzLs6JSDqGAv5PfF
    pub kamino_lending_market: UncheckedAccount<'info>,
    /// CHECK: Kamino's lending market authority PDA
    /// MNT: ??????
    pub kamino_lending_market_authority: UncheckedAccount<'info>,

    /// CHECK: Kamino's reserve account for USDC
    /// MNT: D6q6wuQSrifJKZYpR1M8R4YawnLDtDsMmWM1NbBmgJ59
    #[account(mut)]
    pub kamino_reserve: UncheckedAccount<'info>,

    /// CHECK: USDC Supply Token Account for Kamino Reserve
    #[account(mut)]
    pub kamino_reserve_liquidity_supply: UncheckedAccount<'info>,

    /// MNT: B8V6WVjPxW1UGwVDfxH2d2r8SyT4cqn7dQRK6XneVa7D
    #[account(mut)]
    pub kamino_usdc_collateral_mint: Account<'info, Mint>,

    #[account(
        mut,
        associated_token::mint = kamino_usdc_collateral_mint,
        associated_token::authority = vault_account,
    )]
    pub kamino_usdc_collateral_vault: Account<'info, TokenAccount>,

    // -------- Kamino (Lend) specific: END --------

    // BUILT-IN ACCOUNTS:
    pub associated_token_program: Program<'info, AssociatedToken>,
    pub system_program: Program<'info, System>,
    pub token_program: Program<'info, Token>,
    pub rent: Sysvar<'info, Rent>,
    pub clock: Sysvar<'info, Clock>,

    /// CHECK: Instruction Sysvar Account
    #[account(address = sysvar_instructions::ID)]
    pub instruction_sysvar_account: UncheckedAccount<'info>,
}

#[account]
pub struct Vault {
    pub bump: u8,               // Bump for the vault
    pub owner: Pubkey,          // Owner of the vault
    // pub usdc_vault: Pubkey,     // Token Account for USDC
    pub marginfi_account: Pubkey, // Marginfi account
}

impl Vault {
    pub const LEN: usize = 
    8 + // discriminator
    1 + // bump
    32 + // owner
    32; // marginfi_account


}

pub const VAULT_SEED: &[u8] = b"vault";
// pub const USDC_VAULT_TOKEN_ACCOUNT_SEED: &[u8] = b"usdc_vault";


#[error_code]
pub enum ErrorCode {
    #[msg("No liquidity was redeemed")]
    NothingRedeemed,
}