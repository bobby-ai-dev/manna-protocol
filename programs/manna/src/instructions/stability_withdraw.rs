use anchor_lang::prelude::*;
use anchor_spl::token_2022::{self, Token2022, Transfer};
use anchor_spl::token_interface::{Mint, TokenAccount};

use crate::state::*;
use crate::constants::*;
use crate::errors::MannaError;

/// Withdraw USDsol and claim rewards from Stability Pool
#[derive(Accounts)]
pub struct StabilityWithdrawCtx<'info> {
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
        mut,
        seeds = [b"stability_deposit", depositor.key().as_ref()],
        bump = deposit_record.bump,
        has_one = owner @ MannaError::Unauthorized,
    )]
    pub deposit_record: Account<'info, StabilityDeposit>,
    
    /// CHECK: The owner stored in deposit_record
    pub owner: AccountInfo<'info>,
    
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

pub fn handler(ctx: Context<StabilityWithdrawCtx>, amount: u64) -> Result<()> {
    let stability_pool = &mut ctx.accounts.stability_pool;
    let deposit_record = &mut ctx.accounts.deposit_record;
    
    require!(deposit_record.initial_deposit > 0, MannaError::ZeroAmount);
    
    // Calculate current position
    let compounded_deposit = deposit_record.get_compounded_deposit(stability_pool);
    let pending_collateral = deposit_record.get_pending_collateral_gains(stability_pool);
    
    // Amount to withdraw (capped at available)
    let withdraw_amount = amount.min(compounded_deposit);
    
    // Calculate remaining deposit
    let remaining_deposit = compounded_deposit.saturating_sub(withdraw_amount);
    
    // Accumulate collateral gains
    let total_collateral_gains = deposit_record.collateral_gains
        .checked_add(pending_collateral)
        .ok_or(MannaError::MathOverflow)?;
    
    // Transfer USDsol back to depositor
    if withdraw_amount > 0 {
        let seeds = &[
            STABILITY_POOL_SEED,
            &[stability_pool.bump],
        ];
        let signer_seeds = &[&seeds[..]];
        
        token_2022::transfer(
            CpiContext::new_with_signer(
                ctx.accounts.token_program.to_account_info(),
                Transfer {
                    from: ctx.accounts.pool_usdsol.to_account_info(),
                    to: ctx.accounts.depositor_usdsol.to_account_info(),
                    authority: stability_pool.to_account_info(),
                },
                signer_seeds,
            ),
            withdraw_amount,
        )?;
        
        // Update stability pool
        stability_pool.total_usdsol_deposits = stability_pool.total_usdsol_deposits
            .checked_sub(withdraw_amount)
            .ok_or(MannaError::MathUnderflow)?;
    }
    
    // Transfer SOL collateral gains to depositor
    if total_collateral_gains > 0 {
        // Transfer from stability pool's SOL balance
        **stability_pool.to_account_info().try_borrow_mut_lamports()? -= total_collateral_gains;
        **ctx.accounts.depositor.to_account_info().try_borrow_mut_lamports()? += total_collateral_gains;
        
        stability_pool.total_collateral_gains = stability_pool.total_collateral_gains
            .checked_sub(total_collateral_gains)
            .ok_or(MannaError::MathUnderflow)?;
    }
    
    // Update deposit record
    deposit_record.initial_deposit = remaining_deposit;
    deposit_record.snapshot_p = stability_pool.p;
    deposit_record.snapshot_s = stability_pool.s;
    deposit_record.snapshot_epoch = stability_pool.current_epoch;
    deposit_record.collateral_gains = 0; // Reset after claiming
    
    msg!(
        "Withdrew {} USDsol and {} lamports collateral, remaining deposit: {}",
        withdraw_amount,
        total_collateral_gains,
        remaining_deposit
    );
    
    Ok(())
}
