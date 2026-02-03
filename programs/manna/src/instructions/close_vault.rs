use anchor_lang::prelude::*;
use anchor_spl::token_2022::{self, Token2022, Burn};
use anchor_spl::token_interface::{Mint, TokenAccount};

use crate::state::*;
use crate::constants::*;
use crate::errors::MannaError;

/// Close a vault after repaying all debt
#[derive(Accounts)]
pub struct CloseVault<'info> {
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
    
    /// Owner's vault (will be closed)
    #[account(
        mut,
        seeds = [VAULT_SEED, owner.key().as_ref()],
        bump = vault.bump,
        has_one = owner @ MannaError::Unauthorized,
        close = owner
    )]
    pub vault: Account<'info, Vault>,
    
    /// USDsol mint
    #[account(
        mut,
        seeds = [USDSOL_MINT_SEED],
        bump
    )]
    pub usdsol_mint: InterfaceAccount<'info, Mint>,
    
    /// Owner's USDsol token account
    #[account(
        mut,
        token::mint = usdsol_mint,
        token::authority = owner,
    )]
    pub owner_usdsol: InterfaceAccount<'info, TokenAccount>,
    
    /// Token-2022 program
    pub token_program: Program<'info, Token2022>,
    
    /// System program
    pub system_program: Program<'info, System>,
}

pub fn handler(ctx: Context<CloseVault>) -> Result<()> {
    let vault = &ctx.accounts.vault;
    let global_state = &mut ctx.accounts.global_state;
    
    require!(vault.status == VaultStatus::Active, MannaError::VaultNotActive);
    
    // Must repay ALL debt (including liquidation reserve) to close
    let remaining_debt = vault.debt.saturating_sub(vault.liquidation_reserve);
    
    if remaining_debt > 0 {
        // Burn remaining debt from owner's account
        token_2022::burn(
            CpiContext::new(
                ctx.accounts.token_program.to_account_info(),
                Burn {
                    mint: ctx.accounts.usdsol_mint.to_account_info(),
                    from: ctx.accounts.owner_usdsol.to_account_info(),
                    authority: ctx.accounts.owner.to_account_info(),
                },
            ),
            remaining_debt,
        )?;
        
        // Update global debt
        global_state.total_debt = global_state.total_debt
            .checked_sub(remaining_debt)
            .ok_or(MannaError::MathUnderflow)?;
    }
    
    // Update global state
    global_state.total_collateral = global_state.total_collateral
        .checked_sub(vault.collateral)
        .ok_or(MannaError::MathUnderflow)?;
    global_state.active_vaults = global_state.active_vaults
        .checked_sub(1)
        .ok_or(MannaError::MathUnderflow)?;
    
    // Transfer remaining collateral + liquidation reserve back to owner
    // (done automatically by Anchor's close constraint)
    
    msg!("Vault closed, returned {} lamports collateral + {} liquidation reserve",
        vault.collateral,
        vault.liquidation_reserve
    );
    
    Ok(())
}
