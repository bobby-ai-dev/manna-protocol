use anchor_lang::prelude::*;
use anchor_lang::system_program;

use crate::state::*;
use crate::constants::*;
use crate::errors::MannaError;

/// Deposit additional collateral into an existing vault
#[derive(Accounts)]
pub struct DepositCollateral<'info> {
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
    
    /// System program for SOL transfers
    pub system_program: Program<'info, System>,
}

pub fn handler(ctx: Context<DepositCollateral>, amount: u64) -> Result<()> {
    require!(amount > 0, MannaError::ZeroAmount);
    require!(!ctx.accounts.global_state.is_paused, MannaError::ProtocolPaused);
    require!(ctx.accounts.vault.status == VaultStatus::Active, MannaError::VaultNotActive);
    
    let vault = &mut ctx.accounts.vault;
    let global_state = &mut ctx.accounts.global_state;
    
    // Transfer SOL from owner to vault
    system_program::transfer(
        CpiContext::new(
            ctx.accounts.system_program.to_account_info(),
            system_program::Transfer {
                from: ctx.accounts.owner.to_account_info(),
                to: vault.to_account_info(),
            },
        ),
        amount,
    )?;
    
    // Update vault collateral
    vault.collateral = vault.collateral
        .checked_add(amount)
        .ok_or(MannaError::MathOverflow)?;
    vault.last_updated = Clock::get()?.unix_timestamp;
    
    // Update global collateral
    global_state.total_collateral = global_state.total_collateral
        .checked_add(amount)
        .ok_or(MannaError::MathOverflow)?;
    
    msg!("Deposited {} lamports to vault, new collateral: {}", 
        amount, 
        vault.collateral
    );
    
    Ok(())
}
