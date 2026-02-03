use anchor_lang::prelude::*;
use anchor_spl::token_2022::Token2022;
use anchor_spl::token_interface::Mint;

use crate::state::*;
use crate::constants::*;

/// Initialize the Manna protocol - Part 1
/// Creates global state and stability pool
#[derive(Accounts)]
pub struct InitializeState<'info> {
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
    pub global_state: Box<Account<'info, GlobalState>>,
    
    /// Stability Pool PDA
    #[account(
        init,
        payer = authority,
        space = StabilityPool::LEN,
        seeds = [STABILITY_POOL_SEED],
        bump
    )]
    pub stability_pool: Box<Account<'info, StabilityPool>>,
    
    /// Pyth price feed for SOL/USD
    /// CHECK: Validated in instruction handler
    pub price_feed: AccountInfo<'info>,
    
    /// System program
    pub system_program: Program<'info, System>,
}

/// Initialize the Manna protocol - Part 2
/// Creates the USDsol and MANNA mints
#[derive(Accounts)]
pub struct InitializeMints<'info> {
    /// The authority initializing the protocol
    #[account(mut)]
    pub authority: Signer<'info>,
    
    /// Global protocol state PDA (already initialized)
    #[account(
        mut,
        seeds = [GLOBAL_STATE_SEED],
        bump = global_state.bump,
        constraint = global_state.usdsol_mint == Pubkey::default() @ crate::errors::MannaError::AlreadyInitialized,
    )]
    pub global_state: Box<Account<'info, GlobalState>>,
    
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
    pub usdsol_mint: Box<InterfaceAccount<'info, Mint>>,
    
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
    pub manna_mint: Box<InterfaceAccount<'info, Mint>>,
    
    /// Token-2022 program
    pub token_program: Program<'info, Token2022>,
    
    /// System program
    pub system_program: Program<'info, System>,
}

pub fn handler_initialize_state(ctx: Context<InitializeState>) -> Result<()> {
    let global_state_key = ctx.accounts.global_state.key();
    let global_state = &mut ctx.accounts.global_state;
    let stability_pool = &mut ctx.accounts.stability_pool;
    
    // Initialize global state
    global_state.authority = global_state_key; // Self-referential for immutability
    global_state.usdsol_mint = Pubkey::default(); // Will be set in part 2
    global_state.manna_mint = Pubkey::default(); // Will be set in part 2
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
    
    msg!("Manna Protocol state initialized - Part 1 complete!");
    msg!("Call initialize_mints to complete setup");
    
    Ok(())
}

pub fn handler_initialize_mints(ctx: Context<InitializeMints>) -> Result<()> {
    let global_state = &mut ctx.accounts.global_state;
    
    // Set the mint addresses
    global_state.usdsol_mint = ctx.accounts.usdsol_mint.key();
    global_state.manna_mint = ctx.accounts.manna_mint.key();
    
    msg!("Manna Protocol initialized!");
    msg!("USDsol mint: {}", ctx.accounts.usdsol_mint.key());
    msg!("MANNA mint: {}", ctx.accounts.manna_mint.key());
    
    Ok(())
}
