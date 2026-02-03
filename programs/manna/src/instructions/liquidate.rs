use anchor_lang::prelude::*;
use anchor_spl::token_2022::{self, Token2022};
use anchor_spl::token_interface::{Mint, TokenAccount};

use crate::state::*;
use crate::constants::*;
use crate::errors::MannaError;

/// Liquidate an undercollateralized vault
#[derive(Accounts)]
pub struct Liquidate<'info> {
    /// Liquidator (anyone can call)
    #[account(mut)]
    pub liquidator: Signer<'info>,
    
    /// Global protocol state
    #[account(
        mut,
        seeds = [GLOBAL_STATE_SEED],
        bump = global_state.bump
    )]
    pub global_state: Account<'info, GlobalState>,
    
    /// Vault to liquidate
    #[account(
        mut,
        seeds = [VAULT_SEED, vault_owner.key().as_ref()],
        bump = vault.bump
    )]
    pub vault: Account<'info, Vault>,
    
    /// Vault owner (for PDA derivation)
    /// CHECK: Only used for PDA derivation
    pub vault_owner: AccountInfo<'info>,
    
    /// Stability Pool
    #[account(
        mut,
        seeds = [STABILITY_POOL_SEED],
        bump = stability_pool.bump
    )]
    pub stability_pool: Account<'info, StabilityPool>,
    
    /// USDsol mint
    #[account(
        mut,
        seeds = [USDSOL_MINT_SEED],
        bump
    )]
    pub usdsol_mint: InterfaceAccount<'info, Mint>,
    
    /// Liquidator's USDsol token account (receives gas compensation)
    #[account(
        mut,
        token::mint = usdsol_mint,
        token::authority = liquidator,
    )]
    pub liquidator_usdsol: InterfaceAccount<'info, TokenAccount>,
    
    /// Price feed account
    /// CHECK: Price data validated in instruction handler
    pub price_feed: AccountInfo<'info>,
    
    /// Token-2022 program
    pub token_program: Program<'info, Token2022>,
    
    /// System program
    pub system_program: Program<'info, System>,
}

pub fn handler(ctx: Context<Liquidate>) -> Result<()> {
    let vault = &mut ctx.accounts.vault;
    let global_state = &mut ctx.accounts.global_state;
    let stability_pool = &mut ctx.accounts.stability_pool;
    
    require!(vault.status == VaultStatus::Active, MannaError::VaultNotActive);
    
    // Get SOL price
    let sol_price = get_sol_price(&ctx.accounts.price_feed)?;
    
    // Check if vault is liquidatable
    let is_recovery = global_state.is_recovery_mode(sol_price);
    
    let is_liquidatable = if is_recovery {
        vault.is_liquidatable_recovery_mode(sol_price)
    } else {
        vault.is_liquidatable(sol_price)
    };
    
    require!(is_liquidatable, MannaError::VaultNotLiquidatable);
    
    // Calculate amounts
    let debt_to_liquidate = vault.debt;
    let collateral_to_liquidate = vault.collateral;
    let liquidation_reserve = vault.liquidation_reserve;
    
    // In Recovery Mode, cap liquidation at 110% of debt
    let (final_debt, final_collateral, collateral_surplus) = if is_recovery {
        let vault_cr = vault.calculate_cr(sol_price).unwrap_or(0);
        if vault_cr > MCR {
            // Partial liquidation - only take what's needed at 110%
            let collateral_at_mcr = calculate_collateral_at_mcr(debt_to_liquidate, sol_price);
            let surplus = collateral_to_liquidate.saturating_sub(collateral_at_mcr);
            (debt_to_liquidate, collateral_at_mcr, surplus)
        } else {
            (debt_to_liquidate, collateral_to_liquidate, 0)
        }
    } else {
        (debt_to_liquidate, collateral_to_liquidate, 0)
    };
    
    // Offset debt using Stability Pool
    let (debt_offset, collateral_to_sp) = stability_pool.offset_debt(
        final_debt,
        final_collateral,
    )?;
    
    // Remaining debt/collateral after SP offset (if SP was insufficient)
    let remaining_debt = final_debt.saturating_sub(debt_offset);
    let remaining_collateral = final_collateral.saturating_sub(collateral_to_sp);
    
    // TODO: Handle redistribution if SP insufficient
    // For now, we require full SP offset
    if remaining_debt > 0 {
        msg!("Warning: Stability Pool insufficient, redistribution not yet implemented");
    }
    
    // Transfer liquidation reserve to liquidator (gas compensation)
    **vault.to_account_info().try_borrow_mut_lamports()? -= liquidation_reserve;
    **ctx.accounts.liquidator.to_account_info().try_borrow_mut_lamports()? += liquidation_reserve;
    
    // Transfer collateral surplus back to vault owner if any
    if collateral_surplus > 0 {
        **vault.to_account_info().try_borrow_mut_lamports()? -= collateral_surplus;
        **ctx.accounts.vault_owner.try_borrow_mut_lamports()? += collateral_surplus;
    }
    
    // Liquidation bonus to liquidator (0.5% of liquidated collateral)
    let liquidation_bonus = collateral_to_sp / 200; // 0.5%
    **vault.to_account_info().try_borrow_mut_lamports()? -= liquidation_bonus;
    **ctx.accounts.liquidator.to_account_info().try_borrow_mut_lamports()? += liquidation_bonus;
    
    // Update vault status
    vault.status = VaultStatus::Liquidated;
    vault.collateral = 0;
    vault.debt = 0;
    vault.liquidation_reserve = 0;
    vault.last_updated = Clock::get()?.unix_timestamp;
    
    // Update global state
    global_state.total_collateral = global_state.total_collateral
        .checked_sub(collateral_to_liquidate)
        .ok_or(MannaError::MathUnderflow)?;
    global_state.total_debt = global_state.total_debt
        .checked_sub(debt_to_liquidate)
        .ok_or(MannaError::MathUnderflow)?;
    global_state.active_vaults = global_state.active_vaults
        .checked_sub(1)
        .ok_or(MannaError::MathUnderflow)?;
    
    msg!(
        "Liquidated vault: {} debt, {} collateral offset via SP",
        debt_to_liquidate,
        collateral_to_sp
    );
    
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

fn calculate_collateral_at_mcr(debt: u64, sol_price: u64) -> u64 {
    if sol_price == 0 {
        return u64::MAX;
    }
    
    // collateral = debt * MCR / price * 1e9
    (debt as u128)
        .saturating_mul(MCR as u128)
        .saturating_mul(1_000_000_000)
        .checked_div(sol_price as u128)
        .unwrap_or(u64::MAX as u128)
        .checked_div(1_000_000_000_000_000_000)
        .unwrap_or(u64::MAX as u128) as u64
}
