use anchor_lang::prelude::*;
use anchor_lang::system_program;

use crate::state::*;
use crate::constants::*;
use crate::errors::MannaError;

/// Open a new vault and deposit initial collateral
#[derive(Accounts)]
pub struct OpenVault<'info> {
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
    
    /// New vault PDA for this owner
    #[account(
        init,
        payer = owner,
        space = Vault::LEN,
        seeds = [VAULT_SEED, owner.key().as_ref()],
        bump
    )]
    pub vault: Account<'info, Vault>,
    
    /// System program for SOL transfers
    pub system_program: Program<'info, System>,
}

pub fn handler(ctx: Context<OpenVault>, collateral_amount: u64) -> Result<()> {
    require!(collateral_amount > 0, MannaError::ZeroAmount);
    require!(!ctx.accounts.global_state.is_paused, MannaError::ProtocolPaused);
    
    let vault = &mut ctx.accounts.vault;
    let global_state = &mut ctx.accounts.global_state;
    let clock = Clock::get()?;
    
    // Transfer SOL collateral from owner to vault PDA
    system_program::transfer(
        CpiContext::new(
            ctx.accounts.system_program.to_account_info(),
            system_program::Transfer {
                from: ctx.accounts.owner.to_account_info(),
                to: vault.to_account_info(),
            },
        ),
        collateral_amount,
    )?;
    
    // Initialize vault
    vault.owner = ctx.accounts.owner.key();
    vault.collateral = collateral_amount;
    vault.debt = 0;
    vault.liquidation_reserve = 0;
    vault.status = VaultStatus::Active;
    vault.opened_at = clock.unix_timestamp;
    vault.last_updated = clock.unix_timestamp;
    vault.bump = ctx.bumps.vault;
    
    // Update global state
    global_state.total_collateral = global_state.total_collateral
        .checked_add(collateral_amount)
        .ok_or(MannaError::MathOverflow)?;
    global_state.total_vaults = global_state.total_vaults
        .checked_add(1)
        .ok_or(MannaError::MathOverflow)?;
    global_state.active_vaults = global_state.active_vaults
        .checked_add(1)
        .ok_or(MannaError::MathOverflow)?;
    
    msg!("Vault opened for {} with {} lamports collateral", 
        ctx.accounts.owner.key(), 
        collateral_amount
    );
    
    Ok(())
}
