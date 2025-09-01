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

pub const KEEPER_PUBKEY: &str = "bot7F9sfkm5ztmMGL11St2PD9necoEY6fC84L1WKMDg";
pub fn keeper_pubkey() -> Pubkey {
    KEEPER_PUBKEY.parse().unwrap()
}

declare_id!("CeHNmAJaE8K2yBEo8RRoh5whacchiq1gpqzJVuL8Df97");

// Most lending protocols (Kamino included) define:
// •	deposit_reserve_liquidity(amount) → amount in liquidity units (USDC)
// •	redeem_reserve_collateral(amount) → amount in collateral units (kUSDC)
// Here’s why this new structure works perfectly for that goal:
// Separation of Concerns: The most important change is the separation between user actions and keeper actions.
//  - Users can only deposit to and withdraw from your vault's internal holding account. They have no direct control over which lending protocol is being used.
//  - The Keeper has exclusive permission to call the deploy_to_kamino, withdraw_from_kamino, deploy_to_marginfi, and withdraw_from_marginfi functions.
// The Rebalancing Flow: When your off-chain keeper finds a better APY on MarginFi while the funds are in Kamino, 
// it will execute the automatic swap by calling two instructions in sequence:
// Transaction 1: Call withdraw_from_kamino(...) to pull all the USDC and collateral out of Kamino and back into the vault's secure internal accounts.
// Transaction 2: Immediately after, call deploy_to_marginfi(...) to send that same USDC from the vault's accounts into the MarginFi lending pool.
#[program]
pub mod yield_vault {

    use super::*;

    pub fn initialize(ctx: Context<Initialize>) -> Result<()> {
        let user_vault = &mut ctx.accounts.user_vault_account;
        user_vault.owner = ctx.accounts.user.key();
        user_vault.bump = ctx.bumps.user_vault_account;
        user_vault.marginfi_account = ctx.accounts.marginfi_account.key();

        // Marginfi CPI: Initialize the marginfi account
        let cpi_accounts = mfi_accounts::MarginfiAccountInitialize {
            marginfi_group:   ctx.accounts.marginfi_group.to_account_info(),
            marginfi_account: ctx.accounts.marginfi_account.to_account_info(), // fresh Keypair (outer signer)
            authority:        user_vault.to_account_info(),    // PDA authority
            fee_payer:        ctx.accounts.user.to_account_info(),
            system_program:   ctx.accounts.system_program.to_account_info(),
        };
        let signer: &[&[&[u8]]] = &[&user_vault.seeds()];
        mfi_cpi::marginfi_account_initialize(CpiContext::new_with_signer(
            ctx.accounts.marginfi_program.to_account_info(), cpi_accounts, signer
        ))?;
        
        msg!("Marginfi Account initialized: '{}'", user_vault.marginfi_account.to_string());
        msg!("Vault initialized for owner: {}", user_vault.owner.to_string());
        Ok(())
    }

    pub fn withdraw(ctx: Context<TransferAssets>, amount: u64) -> Result<()> {
        require!(amount > 0, YieldVaultErrors::InvalidAmount);
        msg!("Withdrawing {} from USDC vault", amount);
        let vault_withdraw_accounts = Transfer {
            from: ctx.accounts.user_usdc_vault_ata.to_account_info(),
            to: ctx.accounts.user_usdc_ta.to_account_info(),
            authority: ctx.accounts.user_vault_account.to_account_info(),
        };


        let signer: &[&[&[u8]]] = &[&ctx.accounts.user_vault_account.seeds()];
        let cpi_context = CpiContext::new_with_signer(
            ctx.accounts.token_program.to_account_info(), 
            vault_withdraw_accounts,
            signer);
        transfer(cpi_context, amount)?;

        // ctx.accounts.user_vault_account.deposited_amount = ctx.accounts.user_vault_account.deposited_amount.checked_sub(amount).ok_or(ProgramError::InvalidAccountData)?;
        msg!("Withdrawn {} USDC from vault {} of owner {}", amount, ctx.accounts.user_vault_account.key(), ctx.accounts.user.key().to_string());
        Ok(())
    }

    pub fn deposit(ctx: Context<TransferAssets>, amount: u64) -> Result<()> {
        require!(amount > 0, YieldVaultErrors::InvalidAmount);
        msg!("Depositing {} to USDC vault", amount);
        let vault_deposit_accounts = Transfer {
            from: ctx.accounts.user_usdc_ta.to_account_info(),
            to: ctx.accounts.user_usdc_vault_ata.to_account_info(),
            authority: ctx.accounts.user.to_account_info(),

        };
        let cpi_context = CpiContext::new(ctx.accounts.token_program.to_account_info(), vault_deposit_accounts);
        transfer(cpi_context, amount)?;

        ctx.accounts.user_vault_account.deposited_amount = ctx.accounts.user_vault_account.deposited_amount.checked_add(amount).ok_or(ProgramError::InvalidAccountData)?;
        msg!("Deposited {} USDC to vault {} of owner {}", amount, ctx.accounts.user_vault_account.key(), ctx.accounts.user.key().to_string());
        Ok(())
    }

    pub fn deposit_usdc_marginfi(ctx: Context<DepositUsdcMarginfi>, amount: u64) -> Result<()> {
        require!(amount > 0, YieldVaultErrors::InvalidAmount);
        msg!("Depositing {} to USDC vault", amount);
        let vault_deposit_accounts = Transfer {
            from: ctx.accounts.owner_usdc_account.to_account_info(),
            to: ctx.accounts.user_usdc_vault.to_account_info(),
            authority: ctx.accounts.owner.to_account_info(),
        };
        let cpi_context = CpiContext::new(ctx.accounts.token_program.to_account_info(), vault_deposit_accounts);
        transfer(cpi_context, amount)?;
        msg!("Deposited {} USDC to vault {} of owner {}", amount, ctx.accounts.user_usdc_vault.key(), ctx.accounts.owner.key().to_string());
        
        // Marginfi CPI: Deposit USDC into the marginfi account
        let user_vault = &mut ctx.accounts.user_vault_account; 
        let cpi_accounts = mfi_accounts::LendingAccountDeposit {
            group:                  ctx.accounts.marginfi_group.to_account_info(),
            marginfi_account:       ctx.accounts.marginfi_account.to_account_info(),
            authority:              user_vault.to_account_info(),
            bank:                   ctx.accounts.marginfi_bank.to_account_info(),
            signer_token_account:   ctx.accounts.user_usdc_vault.to_account_info(),
            liquidity_vault:        ctx.accounts.marginfi_bank_liquidity_vault.to_account_info(),
            token_program:          ctx.accounts.token_program.to_account_info(),
        };
    
        let signer: &[&[&[u8]]] = &[&user_vault.seeds()]; 
        let cpi_ctx = CpiContext::new_with_signer(
            ctx.accounts.marginfi_program.to_account_info(), cpi_accounts, signer
        );
    
        mfi_cpi::lending_account_deposit(cpi_ctx, amount, Some(true))?; 
        Ok(())
    }

    pub fn withdraw_usdc_marginfi(ctx: Context<WithdrawUsdcMarginfi>, amount: u64) -> Result<()> {
        require!(amount > 0, YieldVaultErrors::NothingRedeemed);
        let user_vault = &mut ctx.accounts.user_vault_account;  
        // Build CPI accounts
        let cpi_accounts = mfi_accounts::LendingAccountWithdraw {
            group:                       ctx.accounts.marginfi_group.to_account_info(),
            marginfi_account:            ctx.accounts.marginfi_account.to_account_info(),
            authority:                   user_vault.to_account_info(),
            bank:                        ctx.accounts.marginfi_bank.to_account_info(),
            destination_token_account:   ctx.accounts.user_usdc_vault.to_account_info(),
            bank_liquidity_vault_authority: ctx.accounts.marginfi_bank_liquidity_vault_authority.to_account_info(),
            liquidity_vault:                ctx.accounts.marginfi_bank_liquidity_vault.to_account_info(),
            token_program:                  ctx.accounts.token_program.to_account_info(),
        };
    
        // PDA seeds for the vault authority
        let signer: &[&[&[u8]]] = &[&user_vault.seeds()]; 
        let cpi_ctx = CpiContext::new_with_signer(
            ctx.accounts.marginfi_program.to_account_info(), cpi_accounts, signer
        );
    
        mfi_cpi::lending_account_withdraw(cpi_ctx, amount, Some(true))?;
        msg!("Withdrew {} USDC from vault {} of owner {}", amount, user_vault.key(), ctx.accounts.owner.key().to_string());
        Ok(())
    }

    pub fn deploy_usdc_kamino(ctx: Context<TransferUsdcKamino>,  amount: u64) -> Result<()> {
        require!(amount > 0, YieldVaultErrors::InvalidAmount);
        // CPI to deposit from our vault into Kamino
        let signer: &[&[&[u8]]] = &[&ctx.accounts.user_vault_account.seeds()];
        let cpi_deposit_accounts = kamino_cpi::accounts::DepositReserveLiquidity {
            owner:                          ctx.accounts.user_vault_account.to_account_info(),
            reserve:                        ctx.accounts.kamino_reserve.to_account_info(),
            // Lending Market accounts
            lending_market:                 ctx.accounts.kamino_lending_market.to_account_info(),
            lending_market_authority:       ctx.accounts.kamino_lending_market_authority.to_account_info(),
            // Reserve accounts
            reserve_liquidity_mint:         ctx.accounts.usdc_mint.to_account_info(),
            reserve_liquidity_supply:       ctx.accounts.kamino_reserve_liquidity_supply.to_account_info(),
            reserve_collateral_mint:        ctx.accounts.kamino_usdc_collateral_mint.to_account_info(), // TODO: check if this is correct
            // User accounts
            user_source_liquidity:          ctx.accounts.user_usdc_vault_ata.to_account_info(),
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
        msg!("Deposited {} USDC to KLend for user {}",
        amount, ctx.accounts.user.key().to_string());
        ctx.accounts.kamino_usdc_collateral_vault.reload()?;
        let collateral_amount =ctx.accounts.kamino_usdc_collateral_vault.amount;
        msg!("Reserved {} kUSDC at {} user collateral vault ATA", collateral_amount, ctx.accounts.kamino_usdc_collateral_vault.key().to_string());
        Ok(())
    }

    // TODO: Treat the parameter as desired USDC - need to convert it to collateral using the reserve exchange rate on-chain (requires reading reserve state/slot math)
    pub fn redeem_usdc_kaminio(ctx: Context<TransferUsdcKamino>) -> Result<()> {
        let signer: &[&[&[u8]]] = &[&ctx.accounts.user_vault_account.seeds()];
        // reload the collateral vault full balance
        ctx.accounts.kamino_usdc_collateral_vault.reload()?;
        let collateral_amount =ctx.accounts.kamino_usdc_collateral_vault.amount;
        require!(collateral_amount > 0, YieldVaultErrors::NothingRedeemed);
        let cpi_accounts_kamino = kamino_cpi::accounts::RedeemReserveCollateral {
            owner:                         ctx.accounts.user_vault_account.to_account_info(), // PDA
            reserve:                       ctx.accounts.kamino_reserve.to_account_info(),
            lending_market:                ctx.accounts.kamino_lending_market.to_account_info(),
            lending_market_authority:      ctx.accounts.kamino_lending_market_authority.to_account_info(),
            // Reserve accounts
            reserve_liquidity_mint:        ctx.accounts.usdc_mint.to_account_info(),
            reserve_liquidity_supply:      ctx.accounts.kamino_reserve_liquidity_supply.to_account_info(),
            reserve_collateral_mint:       ctx.accounts.kamino_usdc_collateral_mint.to_account_info(),
            // User accounts
            user_source_collateral:        ctx.accounts.kamino_usdc_collateral_vault.to_account_info(),
            user_destination_liquidity:    ctx.accounts.user_usdc_vault_ata.to_account_info(),

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
        msg!("Withdrew {} USDC from KLend for user {}", collateral_amount, ctx.accounts.user.key().to_string());
        Ok(())
    }
}



#[derive(Accounts)]
pub struct TransferAssets<'info> {
    #[account(mut)]
    pub user: Signer<'info>,
    pub usdc_mint: Account<'info, Mint>,
    #[account(
        mut,
        seeds = [VAULT_SEED, user.key().as_ref()],
        bump = user_vault_account.bump
    )]
    pub user_vault_account: Account<'info, UserVault>,

    #[account(
        mut, 
        constraint = user_usdc_ta.mint == usdc_mint.key(), 
        constraint = user_usdc_ta.owner == user.key())]
    pub user_usdc_ta: Account<'info, TokenAccount>,


    #[account(
        mut,
        associated_token::mint = usdc_mint,
        associated_token::authority = user_vault_account,
    )]
    pub user_usdc_vault_ata: Account<'info, TokenAccount>,

    pub token_program: Program<'info, Token>, 
    pub associated_token_program: Program<'info, AssociatedToken>,
    pub system_program: Program<'info, System>,

}
    
#[derive(Accounts)]
pub struct Initialize<'info>{
    #[account(mut)]
    pub user: Signer<'info>,
    pub usdc_mint: Account<'info, Mint>,

    #[account(
        init,
        payer = user,
        space = UserVault::LEN,
        seeds = [VAULT_SEED, user.key().as_ref()],
        bump,
    )]
    pub user_vault_account: Account<'info, UserVault>,

    #[account(
        init,
        payer = user,
        associated_token::mint = usdc_mint,
        associated_token::authority = user_vault_account,
    )]
    pub user_usdc_vault: Account<'info, TokenAccount>,

    // Kamino Specific Accounts:
    pub kamino_usdc_collateral_mint: Account<'info, Mint>,
    #[account(
        init,
        payer = user,
        associated_token::mint = kamino_usdc_collateral_mint,
        associated_token::authority = user_vault_account,
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
pub struct TransferUsdcKamino<'info> {
    #[account(mut, constraint = keeper.key() == keeper_pubkey())]
    pub keeper: Signer<'info>,
    pub usdc_mint: Account<'info, Mint>,

    /// CHECK: User account
    pub user: UncheckedAccount<'info>,

    #[account(
        mut,
        seeds = [VAULT_SEED, user.key().as_ref()],
        bump = user_vault_account.bump
    )]
    pub user_vault_account: Account<'info, UserVault>,
    
    #[account(
        mut,
        associated_token::mint = usdc_mint,
        associated_token::authority = user_vault_account,
    )]
    pub user_usdc_vault_ata: Account<'info, TokenAccount>,

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
         associated_token::authority = user_vault_account,
     )]
     pub kamino_usdc_collateral_vault: Account<'info, TokenAccount>,
     // -------- Kamino (Lend) specific: END --------

    // BUILT-IN ACCOUNTS:
    pub associated_token_program: Program<'info, AssociatedToken>,
    pub system_program: Program<'info, System>,
    pub token_program: Program<'info, Token>,
    pub rent: Sysvar<'info, Rent>,

    /// CHECK: Instruction Sysvar Account
    #[account(address = sysvar_instructions::ID)]
    pub instruction_sysvar_account: UncheckedAccount<'info>,
}

// #[derive(Accounts)]
// pub struct TransferUsdc<'info> {
//     pub usdc_mint: Account<'info, Mint>,
    
//     #[account(mut)]
//     pub owner: Signer<'info>,
//     #[account(mut, constraint = owner_usdc_account.mint == usdc_mint.key(), constraint = owner_usdc_account.owner == owner.key())]
//     pub owner_usdc_account: Account<'info, TokenAccount>,

//     #[account(
//         mut,
//         associated_token::mint = usdc_mint,
//         associated_token::authority = vault_account,
//     )]
//     pub usdc_vault: Account<'info, TokenAccount>,

//     #[account(seeds = [VAULT_SEED, owner.key().as_ref()], bump = vault_account.bump)]
//     pub vault_account: Account<'info, UserVault>,

//      // -------- Kamino (Lend) specific: BEGIN --------
//      /// MNT: KLend2g3cP87fffoy8q1mQqGKjrxjC8boSyAYavgmjD
//     pub kamino_program: Program<'info, KaminoLending>,
//     /// CHECK: Kamino's lending market account
//     /// MNT: 7u3HeHxYDLhnCoErrtycNokbQYbWGzLs6JSDqGAv5PfF
//     pub kamino_lending_market: UncheckedAccount<'info>,
//     /// CHECK: Kamino's lending market authority PDA
//     /// MNT: ??????
//     pub kamino_lending_market_authority: UncheckedAccount<'info>,

//     /// CHECK: Kamino's reserve account for USDC
//     /// MNT: D6q6wuQSrifJKZYpR1M8R4YawnLDtDsMmWM1NbBmgJ59
//     #[account(mut)]
//     pub kamino_reserve: UncheckedAccount<'info>,

//     /// CHECK: USDC Supply Token Account for Kamino Reserve
//     #[account(mut)]
//     pub kamino_reserve_liquidity_supply: UncheckedAccount<'info>,

//     /// MNT: B8V6WVjPxW1UGwVDfxH2d2r8SyT4cqn7dQRK6XneVa7D
//     #[account(mut)]
//     pub kamino_usdc_collateral_mint: Account<'info, Mint>,

//     #[account(
//         mut,
//         associated_token::mint = kamino_usdc_collateral_mint,
//         associated_token::authority = vault_account,
//     )]
//     pub kamino_usdc_collateral_vault: Account<'info, TokenAccount>,

//     // -------- Kamino (Lend) specific: END --------

//     // BUILT-IN ACCOUNTS:
//     pub associated_token_program: Program<'info, AssociatedToken>,
//     pub system_program: Program<'info, System>,
//     pub token_program: Program<'info, Token>,
//     pub rent: Sysvar<'info, Rent>,


//     /// CHECK: Instruction Sysvar Account
//     #[account(address = sysvar_instructions::ID)]
//     pub instruction_sysvar_account: UncheckedAccount<'info>,
    
// }

#[derive(Accounts)]
pub struct DepositUsdcMarginfi<'info> {
    // user signs to move their USDC into the vault (same as your Kamino path)
    #[account(mut)]
    pub owner: Signer<'info>,

    pub usdc_mint: Account<'info, Mint>,

    // user's source ATA (to collect from, or skip if you already moved to vault ATA)
    #[account(
        mut,
        constraint = owner_usdc_account.mint == usdc_mint.key(),
        constraint = owner_usdc_account.owner == owner.key()
    )]
    pub owner_usdc_account: Account<'info, TokenAccount>,

    // vault state PDA (authority for CPIs)
    #[account(
        mut,
        seeds = [VAULT_SEED, owner.key().as_ref()],
        bump = user_vault_account.bump
    )]
    pub user_vault_account: Account<'info, UserVault>,

    // vault’s USDC ATA (the CPI will pull from here)
    #[account(
        mut,
        associated_token::mint = usdc_mint,
        associated_token::authority = user_vault_account,
    )]
    pub user_usdc_vault: Account<'info, TokenAccount>,

    // ---- Marginfi specific ----
    /// CHECK: owner-checked for safety
    #[account(owner = Marginfi::id())]
    pub marginfi_group: UncheckedAccount<'info>,

    /// The vault-owned marginfi account created in initialize()
    /// (store/read its pubkey from UserVault)
    /// CHECK: validated by marginfi program in CPI; must be mut
    #[account(mut, address = user_vault_account.marginfi_account )]
    pub marginfi_account: UncheckedAccount<'info>,

    /// CHECK: USDC bank
    #[account(mut, owner = Marginfi::id())]
    pub marginfi_bank: UncheckedAccount<'info>,

    /// CHECK: bank's liquidity vault (destination)
    #[account(mut)]
    pub marginfi_bank_liquidity_vault: UncheckedAccount<'info>,
    pub marginfi_program: Program<'info, Marginfi>,

    // SPL programs
    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,
    pub associated_token_program: Program<'info, AssociatedToken>,
 
}

#[derive(Accounts)]
pub struct WithdrawUsdcMarginfi<'info> {
    // User receiving USDC
    #[account(mut)]
    pub owner: Signer<'info>,
    pub usdc_mint: Account<'info, Mint>,

    // Destination ATA (USDC back to user)
    #[account(
        mut,
        constraint = owner_usdc_account.mint == usdc_mint.key(),
        constraint = owner_usdc_account.owner == owner.key()
    )]
    pub owner_usdc_account: Account<'info, TokenAccount>,

    // Vault PDA (authority) that “signs” CPIs via seeds
    #[account(
        mut,
        seeds = [VAULT_SEED, owner.key().as_ref()],
        bump = user_vault_account.bump
    )]
    pub user_vault_account: Account<'info, UserVault>,
    
    #[account(
        mut,
        associated_token::mint = usdc_mint,
        associated_token::authority = user_vault_account,
    )]
    pub user_usdc_vault: Account<'info, TokenAccount>,

    // ---- Marginfi specific ----
    /// CHECK: group owned by Marginfi
    #[account(owner = Marginfi::id())]
    pub marginfi_group: UncheckedAccount<'info>,

    /// CHECK: the vault-owned marginfi account
    #[account(mut, address = user_vault_account.marginfi_account)]
    pub marginfi_account: UncheckedAccount<'info>,

    /// CHECK: USDC bank (must be mutable; state updates)
    #[account(mut, owner = Marginfi::id())]
    pub marginfi_bank: UncheckedAccount<'info>,

    /// CHECK: bank’s liquidity vault authority PDA
    pub marginfi_bank_liquidity_vault_authority: UncheckedAccount<'info>,

    /// CHECK: bank’s liquidity vault (source of USDC)
    #[account(mut)]
    pub marginfi_bank_liquidity_vault: UncheckedAccount<'info>,

    pub marginfi_program: Program<'info, Marginfi>,

    // SPL
    pub token_program: Program<'info, Token>,
}

#[account]
pub struct UserVault {
    pub bump: u8,               // Bump for the vault
    pub owner: Pubkey,          // Owner of the vault
    // pub usdc_vault: Pubkey,     // Token Account for USDC
    pub marginfi_account: Pubkey, // Marginfi account
    pub deposited_amount: u64,   // Amount of USDC deposited to the vault
}

impl UserVault {
    pub const LEN: usize = 
    8 + // discriminator
    1 + // bump
    32 + // owner
    32 + // marginfi_account
    8; // deposited_amount

    /// Returns the PDA seeds used to sign as this vault's PDA.
    pub fn seeds<'a>(&'a self) -> [&'a [u8]; 3] {
        [VAULT_SEED, self.owner.as_ref(), core::slice::from_ref(&self.bump)]
    }
}

pub const VAULT_SEED: &[u8] = b"vault";
// pub const USDC_VAULT_TOKEN_ACCOUNT_SEED: &[u8] = b"usdc_vault";

#[derive(AnchorSerialize, AnchorDeserialize, Clone, PartialEq, Eq, Copy)]
pub enum Protocol {
    None,
    Kamino,
    Marginfi,
}

#[error_code]
pub enum YieldVaultErrors {
    #[msg("No liquidity was redeemed")]
    NothingRedeemed,
    #[msg("Amount must be greater than 0")]
    InvalidAmount,
    #[msg("Unauthorized action")]
    Unauthorized,
    #[msg("Funds are already deployed to a protocol")]
    ProtocolAlreadyActive,
    #[msg("Funds are not in the specified protocol")]
    IncorrectProtocol,
}
