use anchor_lang::prelude::*;
use anchor_spl::token::{
    Mint, 
    Token, 
    TokenAccount,
    Transfer, 
    transfer
};
use anchor_spl::associated_token::AssociatedToken;

declare_id!("7hZaF7cQzn6Yej5f6N6kjLTeZuhB6UTqG8GMhtkoD5tk");


#[program]
pub mod yield_vault {
    use super::*;

    pub fn initialize(ctx: Context<Initialize>) -> Result<()> {
        let vault = &mut ctx.accounts.vault_account;
        let owner = ctx.accounts.owner.key();
        vault.owner = owner;
        vault.bump = ctx.bumps.vault_account;
        // vault.usdc_vault = ctx.accounts.usdc_vault.key();


        

        msg!("Vault initialized for owner: {}", owner.to_string());
        Ok(())
    }

    pub fn deposit_usdc(ctx: Context<TransferUsdc>, amount: u64) -> Result<()> {
        // Transfer USDC from owner's USDC token account to vault USDC token account

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

        Ok(())
    }

    pub fn withdraw_usdc(ctx: Context<TransferUsdc>, amount: u64) -> Result<()> {
        // Transfer USDC from vault USDC token account to owner's USDC token account
        let vault_withdraw_accounts = Transfer {
            from: ctx.accounts.usdc_vault.to_account_info(),
            to: ctx.accounts.owner_usdc_account.to_account_info(),
            authority: ctx.accounts.vault_account.to_account_info(),
        };
        let seeds = &[VAULT_SEED, ctx.accounts.owner.to_account_info().key.as_ref(), &[ctx.accounts.vault_account.bump]];

        let signer = &[&seeds[..]];

        let cpi_context = CpiContext::new_with_signer(
            ctx.accounts.token_program.to_account_info(),
            vault_withdraw_accounts,
            signer);
        transfer(cpi_context, amount)?;
        msg!("Withdrew {} USDC from vault {} of owner {}", amount, ctx.accounts.usdc_vault.key(), ctx.accounts.owner.key().to_string());

        Ok(())
    }
}


#[derive(Accounts)]
pub struct Initialize<'info>{
    #[account(mut)]
    pub owner: Signer<'info>,

    pub usdc_mint: Account<'info, Mint>,

    /// CHECK: provided account is a valid SPL Token Mint
    /// collateral_mint: B8V6WVjPxW1UGwVDfxH2d2r8SyT4cqn7dQRK6XneVa7D
    // pub kamino_collateral_mint: AccountInfo<'info>,

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
        // seeds = [USDC_VAULT_TOKEN_ACCOUNT_SEED, vault_account.key().as_ref()],
        // bump,
        associated_token::mint = usdc_mint,
        associated_token::authority = vault_account,
        associated_token::token_program = token_program,
    )]
    pub usdc_vault: Account<'info, TokenAccount>,

    pub system_program: Program<'info, System>,
    pub token_program: Program<'info, Token>,
    pub associated_token_program: Program<'info, AssociatedToken>,
    pub rent: Sysvar<'info, Rent>,
}

#[derive(Accounts)]
pub struct TransferUsdc<'info> {
    #[account(mut)]
    pub owner: Signer<'info>,
    #[account(mut)]
    pub owner_usdc_account: Account<'info, TokenAccount>,

    pub usdc_mint: Account<'info, Mint>,

    #[account(
        mut,
        associated_token::mint = usdc_mint,
        associated_token::authority = vault_account,
        associated_token::token_program = token_program,
    )]
    pub usdc_vault: Account<'info, TokenAccount>,

    #[account(
        seeds = [VAULT_SEED, owner.key().as_ref()],
        bump=vault_account.bump,
    )]
    pub vault_account: Account<'info, Vault>,

    pub associated_token_program: Program<'info, AssociatedToken>,
    pub system_program: Program<'info, System>,
    pub token_program: Program<'info, Token>,
}

#[account]
pub struct Vault {
    pub bump: u8,               // Bump for the vault
    pub owner: Pubkey,          // Owner of the vault
    // pub usdc_vault: Pubkey,     // Token Account for USDC
}

impl Vault {
    pub const LEN: usize = 
    8 + // discriminator
    1 + // bump
    32; // owner

}

pub const VAULT_SEED: &[u8] = b"vault";
pub const USDC_VAULT_TOKEN_ACCOUNT_SEED: &[u8] = b"usdc_vault";