use anchor_lang::prelude::*;
use anchor_spl::token_2022::{self, Token2022, MintTo};
use anchor_spl::token_interface::{Mint, TokenAccount};

use crate::state::*;
use crate::constants::*;
use crate::errors::MannaError;

/// Borrow USDsol against collateral in vault
#[derive(Accounts)]
pub struct Borrow<'info> {
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
    
    /// Owner's USDsol token account (receives minted tokens)
    #[account(
        mut,
        token::mint = usdsol_mint,
        token::authority = owner,
    )]
    pub owner_usdsol: InterfaceAccount<'info, TokenAccount>,
    
    /// Price feed account (Pyth or mock)
    /// CHECK: Price data validated in instruction handler
    pub price_feed: AccountInfo<'info>,
    
    /// Token-2022 program
    pub token_program: Program<'info, Token2022>,
}

pub fn handler(ctx: Context<Borrow>, borrow_amount: u64) -> Result<()> {
    require!(borrow_amount > 0, MannaError::ZeroAmount);
    require!(!ctx.accounts.global_state.is_paused, MannaError::ProtocolPaused);
    require!(ctx.accounts.vault.status == VaultStatus::Active, MannaError::VaultNotActive);
    
    let vault = &mut ctx.accounts.vault;
    let global_state = &mut ctx.accounts.global_state;
    let clock = Clock::get()?;
    
    // Get SOL price from price feed
    let sol_price = get_sol_price(&ctx.accounts.price_feed)?;
    
    // Check if system is in Recovery Mode
    let is_recovery = global_state.is_recovery_mode(sol_price);
    
    // Calculate borrowing fee
    let borrowing_fee = if is_recovery {
        0 // No fee in Recovery Mode to encourage recapitalization
    } else {
        global_state.calculate_borrowing_fee(borrow_amount)
    };
    
    // Total debt increase = borrow_amount + fee
    let total_debt_increase = borrow_amount
        .checked_add(borrowing_fee)
        .ok_or(MannaError::MathOverflow)?;
    
    // Set liquidation reserve if this is first borrow
    let liquidation_reserve_increase = if vault.debt == 0 {
        LIQUIDATION_RESERVE
    } else {
        0
    };
    
    // Calculate new debt
    let new_debt = vault.debt
        .checked_add(total_debt_increase)
        .ok_or(MannaError::MathOverflow)?
        .checked_add(liquidation_reserve_increase)
        .ok_or(MannaError::MathOverflow)?;
    
    // Check minimum debt
    require!(new_debt >= MIN_DEBT, MannaError::BelowMinimumDebt);
    
    // Calculate new CR and check it's above MCR
    let new_cr = calculate_cr(vault.collateral, new_debt, sol_price)
        .ok_or(MannaError::MathOverflow)?;
    
    let min_cr = if is_recovery { CCR } else { MCR };
    require!(new_cr >= min_cr, MannaError::BelowMinimumCollateralRatio);
    
    // In Recovery Mode, new borrowing must improve TCR
    if is_recovery {
        let new_total_debt = global_state.total_debt
            .checked_add(total_debt_increase)
            .ok_or(MannaError::MathOverflow)?;
        let new_tcr = calculate_tcr(global_state.total_collateral, new_total_debt, sol_price)
            .ok_or(MannaError::MathOverflow)?;
        let old_tcr = global_state.calculate_tcr(sol_price).unwrap_or(0);
        require!(new_tcr >= old_tcr, MannaError::RecoveryModeActive);
    }
    
    // Update vault
    vault.debt = new_debt;
    vault.liquidation_reserve = vault.liquidation_reserve
        .checked_add(liquidation_reserve_increase)
        .ok_or(MannaError::MathOverflow)?;
    vault.last_updated = clock.unix_timestamp;
    
    // Update global state
    global_state.total_debt = global_state.total_debt
        .checked_add(total_debt_increase)
        .ok_or(MannaError::MathOverflow)?;
    
    // Mint USDsol to borrower (only the borrow_amount, not the fee)
    let seeds = &[
        GLOBAL_STATE_SEED,
        &[global_state.bump],
    ];
    let signer_seeds = &[&seeds[..]];
    
    token_2022::mint_to(
        CpiContext::new_with_signer(
            ctx.accounts.token_program.to_account_info(),
            MintTo {
                mint: ctx.accounts.usdsol_mint.to_account_info(),
                to: ctx.accounts.owner_usdsol.to_account_info(),
                authority: global_state.to_account_info(),
            },
            signer_seeds,
        ),
        borrow_amount,
    )?;
    
    msg!(
        "Borrowed {} USDsol (fee: {}), new debt: {}, CR: {}%",
        borrow_amount,
        borrowing_fee,
        vault.debt,
        new_cr / 10_000_000_000_000_000 // Convert to percentage
    );
    
    Ok(())
}

/// Get SOL/USD price from price feed
/// For hackathon: Uses a simplified price feed format
/// Production would use Pyth SDK with full validation
fn get_sol_price(price_feed: &AccountInfo) -> Result<u64> {
    // For hackathon demo: Read price from account data
    // Format: 8 bytes for price (u64, 6 decimals), 8 bytes for timestamp
    // In production, use Pyth SDK with proper validation
    
    let data = price_feed.try_borrow_data()?;
    
    // Minimum account size check
    if data.len() < 16 {
        // Default to $200 SOL for testing if no price feed
        return Ok(200_000_000); // $200 with 6 decimals
    }
    
    // Read price (first 8 bytes)
    let price_bytes: [u8; 8] = data[0..8].try_into()
        .map_err(|_| MannaError::InvalidOraclePrice)?;
    let price = u64::from_le_bytes(price_bytes);
    
    // Read timestamp (next 8 bytes) and check staleness
    let timestamp_bytes: [u8; 8] = data[8..16].try_into()
        .map_err(|_| MannaError::InvalidOraclePrice)?;
    let timestamp = i64::from_le_bytes(timestamp_bytes);
    
    let clock = Clock::get()?;
    if clock.unix_timestamp - timestamp > 60 {
        // For demo purposes, allow stale prices but log warning
        msg!("Warning: Price feed may be stale");
    }
    
    if price == 0 {
        return Err(MannaError::InvalidOraclePrice.into());
    }
    
    Ok(price)
}

/// Calculate collateral ratio
fn calculate_cr(collateral: u64, debt: u64, sol_price: u64) -> Option<u64> {
    if debt == 0 {
        return Some(u64::MAX);
    }
    
    let collateral_value = (collateral as u128)
        .checked_mul(sol_price as u128)?;
    
    let cr = collateral_value
        .checked_mul(1_000_000_000_000_000_000)? // 1e18
        .checked_div(debt as u128)?
        .checked_div(1_000_000_000)?; // lamports adjustment
    
    Some(cr as u64)
}

/// Calculate total collateral ratio
fn calculate_tcr(total_collateral: u64, total_debt: u64, sol_price: u64) -> Option<u64> {
    calculate_cr(total_collateral, total_debt, sol_price)
}
