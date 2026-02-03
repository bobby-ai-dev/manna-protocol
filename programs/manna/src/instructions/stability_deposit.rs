use anchor_lang::prelude::*;
use anchor_spl::token_2022::{self, Token2022, Transfer};
use anchor_spl::token_interface::{Mint, TokenAccount};

use crate::state::*;
use crate::constants::*;
use crate::errors::MannaError;

/// Deposit USDsol into the Stability Pool
#[derive(Accounts)]
pub struct StabilityDepositCtx<'info> {
    /// Depositor
    #[account(mut)]
    pub depositor: Signer<'info>,
    
    /// Global protocol state
    #[account(
        seeds = [GLOBAL_STATE_SEED],
        bump = global_state.bump
    )]
    pub global_state: Account<'info, GlobalState>,
    
    /// Stability Pool
    #[account(
        mut,
        seeds = [STABILITY_POOL_SEED],
        bump = stability_pool.bump
    )]
    pub stability_pool: Account<'info, StabilityPool>,
    
    /// Depositor's stability deposit record
    #[account(
        init_if_needed,
        payer = depositor,
        space = StabilityDeposit::LEN,
        seeds = [b"stability_deposit", depositor.key().as_ref()],
        bump
    )]
    pub deposit_record: Account<'info, StabilityDeposit>,
    
    /// USDsol mint
    #[account(
        seeds = [USDSOL_MINT_SEED],
        bump
    )]
    pub usdsol_mint: InterfaceAccount<'info, Mint>,
    
    /// Depositor's USDsol token account
    #[account(
        mut,
        token::mint = usdsol_mint,
        token::authority = depositor,
    )]
    pub depositor_usdsol: InterfaceAccount<'info, TokenAccount>,
    
    /// Stability Pool's USDsol token account
    #[account(
        mut,
        token::mint = usdsol_mint,
    )]
    pub pool_usdsol: InterfaceAccount<'info, TokenAccount>,
    
    /// Token-2022 program
    pub token_program: Program<'info, Token2022>,
    
    /// System program
    pub system_program: Program<'info, System>,
}

pub fn handler(ctx: Context<StabilityDepositCtx>, amount: u64) -> Result<()> {
    require!(amount > 0, MannaError::ZeroAmount);
    require!(!ctx.accounts.global_state.is_paused, MannaError::ProtocolPaused);
    
    let stability_pool = &mut ctx.accounts.stability_pool;
    let deposit_record = &mut ctx.accounts.deposit_record;
    let clock = Clock::get()?;
    
    // If existing deposit, first claim pending rewards
    if deposit_record.initial_deposit > 0 {
        let pending_collateral = deposit_record.get_pending_collateral_gains(stability_pool);
        let compounded_deposit = deposit_record.get_compounded_deposit(stability_pool);
        
        // Accumulate gains
        deposit_record.collateral_gains = deposit_record.collateral_gains
            .checked_add(pending_collateral)
            .ok_or(MannaError::MathOverflow)?;
        
        // Update initial deposit to compounded value
        deposit_record.initial_deposit = compounded_deposit;
    }
    
    // Transfer USDsol from depositor to pool
    token_2022::transfer(
        CpiContext::new(
            ctx.accounts.token_program.to_account_info(),
            Transfer {
                from: ctx.accounts.depositor_usdsol.to_account_info(),
                to: ctx.accounts.pool_usdsol.to_account_info(),
                authority: ctx.accounts.depositor.to_account_info(),
            },
        ),
        amount,
    )?;
    
    // Update deposit record
    deposit_record.owner = ctx.accounts.depositor.key();
    deposit_record.initial_deposit = deposit_record.initial_deposit
        .checked_add(amount)
        .ok_or(MannaError::MathOverflow)?;
    deposit_record.snapshot_p = stability_pool.p;
    deposit_record.snapshot_s = stability_pool.s;
    deposit_record.snapshot_epoch = stability_pool.current_epoch;
    deposit_record.deposited_at = clock.unix_timestamp;
    deposit_record.bump = ctx.bumps.deposit_record;
    
    // Update stability pool
    stability_pool.total_usdsol_deposits = stability_pool.total_usdsol_deposits
        .checked_add(amount)
        .ok_or(MannaError::MathOverflow)?;
    
    msg!("Deposited {} USDsol to Stability Pool, total deposit: {}", 
        amount, 
        deposit_record.initial_deposit
    );
    
    Ok(())
}
