use anchor_lang::prelude::*;
use anchor_spl::token_2022::{self, Token2022, Burn};
use anchor_spl::token_interface::{Mint, TokenAccount};

use crate::state::*;
use crate::constants::*;
use crate::errors::MannaError;

/// Repay USDsol debt
#[derive(Accounts)]
pub struct Repay<'info> {
    /// Vault owner
    #[account(mut)]
    pub owner: Signer<'info>,
    
    /// Global protocol state
    #[account(
        mut,
        seeds = [GLOBAL_STATE_SEED],
        bump = global_state.bump
    )]
    pub global_state: Account<'info, GlobalState>,
    
    /// Owner's vault
    #[account(
        mut,
        seeds = [VAULT_SEED, owner.key().as_ref()],
        bump = vault.bump,
        has_one = owner @ MannaError::Unauthorized
    )]
    pub vault: Account<'info, Vault>,
    
    /// USDsol mint
    #[account(
        mut,
        seeds = [USDSOL_MINT_SEED],
        bump
    )]
    pub usdsol_mint: InterfaceAccount<'info, Mint>,
    
    /// Owner's USDsol token account (tokens will be burned from here)
    #[account(
        mut,
        token::mint = usdsol_mint,
        token::authority = owner,
    )]
    pub owner_usdsol: InterfaceAccount<'info, TokenAccount>,
    
    /// Token-2022 program
    pub token_program: Program<'info, Token2022>,
}

pub fn handler(ctx: Context<Repay>, repay_amount: u64) -> Result<()> {
    require!(repay_amount > 0, MannaError::ZeroAmount);
    require!(ctx.accounts.vault.status == VaultStatus::Active, MannaError::VaultNotActive);
    
    let vault = &mut ctx.accounts.vault;
    let global_state = &mut ctx.accounts.global_state;
    
    // Cannot repay more than debt (excluding liquidation reserve)
    let actual_repay = repay_amount.min(vault.debt.saturating_sub(vault.liquidation_reserve));
    require!(actual_repay > 0, MannaError::InsufficientDebt);
    
    // Burn the repaid USDsol
    token_2022::burn(
        CpiContext::new(
            ctx.accounts.token_program.to_account_info(),
            Burn {
                mint: ctx.accounts.usdsol_mint.to_account_info(),
                from: ctx.accounts.owner_usdsol.to_account_info(),
                authority: ctx.accounts.owner.to_account_info(),
            },
        ),
        actual_repay,
    )?;
    
    // Update vault debt
    vault.debt = vault.debt
        .checked_sub(actual_repay)
        .ok_or(MannaError::MathUnderflow)?;
    vault.last_updated = Clock::get()?.unix_timestamp;
    
    // Update global debt
    global_state.total_debt = global_state.total_debt
        .checked_sub(actual_repay)
        .ok_or(MannaError::MathUnderflow)?;
    
    msg!("Repaid {} USDsol, remaining debt: {}", actual_repay, vault.debt);
    
    Ok(())
}
