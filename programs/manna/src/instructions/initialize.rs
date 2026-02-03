use anchor_lang::prelude::*;
use anchor_spl::token_2022::{Token2022, self};
use anchor_spl::token_interface::{Mint, TokenAccount};

use crate::state::*;
use crate::constants::*;

/// Initialize the Manna protocol
/// Creates global state, USDsol mint, MANNA mint, and stability pool
#[derive(Accounts)]
pub struct Initialize<'info> {
    /// The authority initializing the protocol (becomes frozen after init)
    #[account(mut)]
    pub authority: Signer<'info>,
    
    /// Global protocol state PDA
    #[account(
        init,
        payer = authority,
        space = GlobalState::LEN,
        seeds = [GLOBAL_STATE_SEED],
        bump
    )]
    pub global_state: Account<'info, GlobalState>,
    
    /// USDsol stablecoin mint (Token-2022)
    #[account(
        init,
        payer = authority,
        seeds = [USDSOL_MINT_SEED],
        bump,
        mint::decimals = USDSOL_DECIMALS,
        mint::authority = global_state,
        mint::token_program = token_program,
    )]
    pub usdsol_mint: InterfaceAccount<'info, Mint>,
    
    /// MANNA token mint (Token-2022)
    #[account(
        init,
        payer = authority,
        seeds = [MANNA_MINT_SEED],
        bump,
        mint::decimals = USDSOL_DECIMALS, // MANNA also uses 6 decimals
        mint::authority = global_state,
        mint::token_program = token_program,
    )]
    pub manna_mint: InterfaceAccount<'info, Mint>,
    
    /// Stability Pool PDA
    #[account(
        init,
        payer = authority,
        space = StabilityPool::LEN,
        seeds = [STABILITY_POOL_SEED],
        bump
    )]
    pub stability_pool: Account<'info, StabilityPool>,
    
    /// Pyth price feed for SOL/USD
    /// CHECK: Validated in instruction handler
    pub price_feed: AccountInfo<'info>,
    
    /// Token-2022 program
    pub token_program: Program<'info, Token2022>,
    
    /// System program
    pub system_program: Program<'info, System>,
    
    /// Rent sysvar
    pub rent: Sysvar<'info, Rent>,
}

pub fn handler(ctx: Context<Initialize>) -> Result<()> {
    let global_state = &mut ctx.accounts.global_state;
    let stability_pool = &mut ctx.accounts.stability_pool;
    
    // Initialize global state
    global_state.authority = ctx.accounts.global_state.key(); // Self-referential for immutability
    global_state.usdsol_mint = ctx.accounts.usdsol_mint.key();
    global_state.manna_mint = ctx.accounts.manna_mint.key();
    global_state.price_feed = ctx.accounts.price_feed.key();
    global_state.total_collateral = 0;
    global_state.total_debt = 0;
    global_state.base_rate = 0;
    global_state.last_fee_operation_time = Clock::get()?.unix_timestamp;
    global_state.total_vaults = 0;
    global_state.active_vaults = 0;
    global_state.is_paused = false;
    global_state.bump = ctx.bumps.global_state;
    
    // Initialize stability pool
    stability_pool.initialize(ctx.bumps.stability_pool);
    
    msg!("Manna Protocol initialized!");
    msg!("USDsol mint: {}", ctx.accounts.usdsol_mint.key());
    msg!("MANNA mint: {}", ctx.accounts.manna_mint.key());
    
    Ok(())
}
