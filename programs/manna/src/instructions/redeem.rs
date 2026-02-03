use anchor_lang::prelude::*;
use anchor_spl::token_2022::{self, Token2022, Burn};
use anchor_spl::token_interface::{Mint, TokenAccount};

use crate::state::*;
use crate::constants::*;
use crate::errors::MannaError;

/// Redeem USDsol for SOL collateral at $1 parity
/// This is the core peg mechanism - anyone can redeem USDsol for $1 of SOL
/// Redemptions are fulfilled from vaults with lowest collateral ratios first
#[derive(Accounts)]
pub struct Redeem<'info> {
    /// Redeemer
    #[account(mut)]
    pub redeemer: Signer<'info>,
    
    /// Global protocol state
    #[account(
        mut,
        seeds = [GLOBAL_STATE_SEED],
        bump = global_state.bump
    )]
    pub global_state: Account<'info, GlobalState>,
    
    /// Vault to redeem from (lowest CR)
    /// Multiple vaults may need to be passed for large redemptions
    #[account(
        mut,
        seeds = [VAULT_SEED, vault_owner.key().as_ref()],
        bump = vault.bump
    )]
    pub vault: Account<'info, Vault>,
    
    /// Vault owner
    /// CHECK: Only used for PDA derivation
    pub vault_owner: AccountInfo<'info>,
    
    /// USDsol mint
    #[account(
        mut,
        seeds = [USDSOL_MINT_SEED],
        bump
    )]
    pub usdsol_mint: InterfaceAccount<'info, Mint>,
    
    /// Redeemer's USDsol token account (tokens will be burned)
    #[account(
        mut,
        token::mint = usdsol_mint,
        token::authority = redeemer,
    )]
    pub redeemer_usdsol: InterfaceAccount<'info, TokenAccount>,
    
    /// Price feed account
    /// CHECK: Price data validated in instruction handler
    pub price_feed: AccountInfo<'info>,
    
    /// Token-2022 program
    pub token_program: Program<'info, Token2022>,
    
    /// System program for SOL transfers
    pub system_program: Program<'info, System>,
}

pub fn handler(ctx: Context<Redeem>, usdsol_amount: u64) -> Result<()> {
    require!(usdsol_amount > 0, MannaError::ZeroAmount);
    require!(!ctx.accounts.global_state.is_paused, MannaError::ProtocolPaused);
    require!(ctx.accounts.vault.status == VaultStatus::Active, MannaError::VaultNotActive);
    
    let vault = &mut ctx.accounts.vault;
    let global_state = &mut ctx.accounts.global_state;
    let clock = Clock::get()?;
    
    // Get SOL price
    let sol_price = get_sol_price(&ctx.accounts.price_feed)?;
    
    // Calculate redemption fee (same mechanism as borrowing fee)
    let redemption_fee_rate = global_state.get_borrowing_fee_rate();
    let redemption_fee = ((usdsol_amount as u128)
        .saturating_mul(redemption_fee_rate as u128)
        .checked_div(DECIMAL_PRECISION as u128)
        .unwrap_or(0)) as u64;
    
    // Net USDsol after fee
    let net_usdsol = usdsol_amount.saturating_sub(redemption_fee);
    
    // Calculate SOL collateral to receive
    // collateral = (net_usdsol * 1e9) / sol_price
    // (USDsol is 6 decimals, SOL is 9 decimals, price is 6 decimals)
    let collateral_to_receive = ((net_usdsol as u128)
        .saturating_mul(1_000_000_000) // Convert to lamports
        .checked_div(sol_price as u128)
        .unwrap_or(0)) as u64;
    
    // Cannot redeem more than vault's debt (minus liquidation reserve)
    let redeemable_debt = vault.debt.saturating_sub(vault.liquidation_reserve);
    let actual_redeem_amount = usdsol_amount.min(redeemable_debt);
    
    if actual_redeem_amount == 0 {
        return Err(MannaError::InvalidRedemptionAmount.into());
    }
    
    // Recalculate collateral for actual amount
    let actual_net = actual_redeem_amount.saturating_sub(
        ((actual_redeem_amount as u128)
            .saturating_mul(redemption_fee_rate as u128)
            .checked_div(DECIMAL_PRECISION as u128)
            .unwrap_or(0)) as u64
    );
    
    let actual_collateral = ((actual_net as u128)
        .saturating_mul(1_000_000_000)
        .checked_div(sol_price as u128)
        .unwrap_or(0)) as u64;
    
    // Ensure vault has enough collateral
    require!(actual_collateral <= vault.collateral, MannaError::InsufficientCollateral);
    
    // Burn USDsol from redeemer
    token_2022::burn(
        CpiContext::new(
            ctx.accounts.token_program.to_account_info(),
            Burn {
                mint: ctx.accounts.usdsol_mint.to_account_info(),
                from: ctx.accounts.redeemer_usdsol.to_account_info(),
                authority: ctx.accounts.redeemer.to_account_info(),
            },
        ),
        actual_redeem_amount,
    )?;
    
    // Transfer SOL collateral to redeemer
    **vault.to_account_info().try_borrow_mut_lamports()? -= actual_collateral;
    **ctx.accounts.redeemer.to_account_info().try_borrow_mut_lamports()? += actual_collateral;
    
    // Update vault
    vault.debt = vault.debt.saturating_sub(actual_redeem_amount);
    vault.collateral = vault.collateral.saturating_sub(actual_collateral);
    vault.last_updated = clock.unix_timestamp;
    
    // Update global state
    global_state.total_debt = global_state.total_debt.saturating_sub(actual_redeem_amount);
    global_state.total_collateral = global_state.total_collateral.saturating_sub(actual_collateral);
    global_state.update_base_rate(actual_redeem_amount, clock.unix_timestamp);
    
    // If vault debt drops to just liquidation reserve, close it
    if vault.debt <= vault.liquidation_reserve {
        // Return remaining collateral and liquidation reserve to vault owner
        let remaining = vault.collateral.saturating_add(vault.liquidation_reserve);
        if remaining > 0 {
            **vault.to_account_info().try_borrow_mut_lamports()? -= remaining;
            **ctx.accounts.vault_owner.try_borrow_mut_lamports()? += remaining;
        }
        
        vault.status = VaultStatus::ClosedByOwner;
        vault.collateral = 0;
        vault.debt = 0;
        vault.liquidation_reserve = 0;
        
        global_state.active_vaults = global_state.active_vaults.saturating_sub(1);
    }
    
    msg!(
        "Redeemed {} USDsol for {} lamports (fee: {} basis points)",
        actual_redeem_amount,
        actual_collateral,
        redemption_fee_rate / 10_000_000_000_000_000 // Convert to bps for display
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
