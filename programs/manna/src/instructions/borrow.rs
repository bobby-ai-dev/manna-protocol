use anchor_lang::prelude::*;
use anchor_spl::token_2022::{self, Token2022, MintTo};
use anchor_spl::token_interface::{Mint, TokenAccount};
use pyth_solana_receiver_sdk::price_update::PriceUpdateV2;

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
    
    /// Pyth price feed
    pub price_feed: Account<'info, PriceUpdateV2>,
    
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
    
    // Get SOL price from Pyth
    let sol_price = get_sol_price(&ctx.accounts.price_feed, clock.unix_timestamp)?;
    
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

/// Get SOL/USD price from Pyth
fn get_sol_price(price_feed: &Account<PriceUpdateV2>, current_time: i64) -> Result<u64> {
    let price = price_feed.get_price_no_older_than(
        &Clock::get()?,
        60, // 60 second staleness threshold
        &pyth_solana_receiver_sdk::price_update::get_feed_id_from_hex(PYTH_SOL_USD_FEED)?,
    )?;
    
    // Convert price to 6 decimal format
    // Pyth prices have variable exponents
    let price_value = if price.exponent >= 0 {
        (price.price as u64)
            .checked_mul(10u64.pow(price.exponent as u32))
            .ok_or(MannaError::MathOverflow)?
    } else {
        (price.price as u64)
            .checked_div(10u64.pow((-price.exponent) as u32))
            .ok_or(MannaError::MathOverflow)?
    };
    
    // Scale to 6 decimals if needed
    let scaled_price = price_value
        .checked_mul(1_000_000)
        .ok_or(MannaError::MathOverflow)?
        .checked_div(10u64.pow((-price.exponent.min(0)) as u32))
        .ok_or(MannaError::MathOverflow)?;
    
    Ok(scaled_price)
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
