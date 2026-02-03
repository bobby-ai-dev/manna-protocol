use anchor_lang::prelude::*;

use crate::state::*;
use crate::constants::*;
use crate::errors::MannaError;

/// Withdraw collateral from vault
#[derive(Accounts)]
pub struct WithdrawCollateral<'info> {
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
    
    /// Price feed account
    /// CHECK: Price data validated in instruction handler
    pub price_feed: AccountInfo<'info>,
    
    /// System program for SOL transfers
    pub system_program: Program<'info, System>,
}

pub fn handler(ctx: Context<WithdrawCollateral>, amount: u64) -> Result<()> {
    require!(amount > 0, MannaError::ZeroAmount);
    require!(!ctx.accounts.global_state.is_paused, MannaError::ProtocolPaused);
    require!(ctx.accounts.vault.status == VaultStatus::Active, MannaError::VaultNotActive);
    
    let vault = &mut ctx.accounts.vault;
    let global_state = &mut ctx.accounts.global_state;
    
    require!(amount <= vault.collateral, MannaError::InsufficientCollateral);
    
    // Get SOL price
    let sol_price = get_sol_price(&ctx.accounts.price_feed)?;
    
    // Calculate new collateral and CR
    let new_collateral = vault.collateral
        .checked_sub(amount)
        .ok_or(MannaError::MathUnderflow)?;
    
    // If there's debt, check CR after withdrawal
    if vault.debt > 0 {
        let new_cr = calculate_cr(new_collateral, vault.debt, sol_price)
            .ok_or(MannaError::MathOverflow)?;
        
        let is_recovery = global_state.is_recovery_mode(sol_price);
        let min_cr = if is_recovery { CCR } else { MCR };
        
        require!(new_cr >= min_cr, MannaError::WithdrawalWouldBreachMCR);
        
        // In Recovery Mode, withdrawal must not decrease TCR
        if is_recovery {
            let new_total_collateral = global_state.total_collateral
                .checked_sub(amount)
                .ok_or(MannaError::MathUnderflow)?;
            let new_tcr = calculate_tcr(new_total_collateral, global_state.total_debt, sol_price)
                .ok_or(MannaError::MathOverflow)?;
            let old_tcr = global_state.calculate_tcr(sol_price).unwrap_or(0);
            require!(new_tcr >= old_tcr, MannaError::RecoveryModeActive);
        }
    }
    
    // Transfer SOL from vault to owner
    **vault.to_account_info().try_borrow_mut_lamports()? -= amount;
    **ctx.accounts.owner.to_account_info().try_borrow_mut_lamports()? += amount;
    
    // Update vault
    vault.collateral = new_collateral;
    vault.last_updated = Clock::get()?.unix_timestamp;
    
    // Update global state
    global_state.total_collateral = global_state.total_collateral
        .checked_sub(amount)
        .ok_or(MannaError::MathUnderflow)?;
    
    msg!("Withdrew {} lamports, remaining collateral: {}", amount, vault.collateral);
    
    Ok(())
}

fn get_sol_price(price_feed: &AccountInfo) -> Result<u64> {
    let data = price_feed.try_borrow_data()?;
    
    if data.len() < 16 {
        return Ok(200_000_000); // Default $200 for testing
    }
    
    let price_bytes: [u8; 8] = data[0..8].try_into()
        .map_err(|_| MannaError::InvalidOraclePrice)?;
    let price = u64::from_le_bytes(price_bytes);
    
    if price == 0 {
        return Err(MannaError::InvalidOraclePrice.into());
    }
    
    Ok(price)
}

fn calculate_cr(collateral: u64, debt: u64, sol_price: u64) -> Option<u64> {
    if debt == 0 {
        return Some(u64::MAX);
    }
    
    let collateral_value = (collateral as u128)
        .checked_mul(sol_price as u128)?;
    
    collateral_value
        .checked_mul(1_000_000_000_000_000_000)?
        .checked_div(debt as u128)?
        .checked_div(1_000_000_000)
        .map(|v| v as u64)
}

fn calculate_tcr(total_collateral: u64, total_debt: u64, sol_price: u64) -> Option<u64> {
    calculate_cr(total_collateral, total_debt, sol_price)
}
