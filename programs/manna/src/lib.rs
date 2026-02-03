use anchor_lang::prelude::*;

pub mod constants;
pub mod errors;
pub mod instructions;
pub mod state;

use instructions::*;

declare_id!("MaNNa11111111111111111111111111111111111111");

/// Manna Protocol - Decentralized Borrowing on Solana
/// 
/// Manna enables users to mint USDsol (a USD-pegged stablecoin) by depositing
/// SOL as collateral. It implements Liquity-style mechanics including:
/// - 110% minimum collateral ratio
/// - One-time borrowing fees (no ongoing interest)
/// - Stability Pool for liquidations
/// - Redemption mechanism for peg stability
/// 
/// Built for the Colosseum Agent Hackathon, February 2026
#[program]
pub mod manna {
    use super::*;

    /// Initialize the Manna protocol
    /// Creates global state, USDsol mint, MANNA mint, and stability pool
    /// Can only be called once
    pub fn initialize(ctx: Context<Initialize>) -> Result<()> {
        instructions::initialize::handler(ctx)
    }

    /// Open a new vault and deposit initial SOL collateral
    /// 
    /// # Arguments
    /// * `collateral_amount` - Amount of SOL (in lamports) to deposit
    pub fn open_vault(ctx: Context<OpenVault>, collateral_amount: u64) -> Result<()> {
        instructions::open_vault::handler(ctx, collateral_amount)
    }

    /// Deposit additional SOL collateral into an existing vault
    /// 
    /// # Arguments
    /// * `amount` - Amount of SOL (in lamports) to deposit
    pub fn deposit_collateral(ctx: Context<DepositCollateral>, amount: u64) -> Result<()> {
        instructions::deposit_collateral::handler(ctx, amount)
    }

    /// Borrow USDsol against vault collateral
    /// A one-time borrowing fee is charged and added to debt
    /// 
    /// # Arguments
    /// * `borrow_amount` - Amount of USDsol to borrow (6 decimals)
    pub fn borrow(ctx: Context<Borrow>, borrow_amount: u64) -> Result<()> {
        instructions::borrow::handler(ctx, borrow_amount)
    }

    /// Repay USDsol debt
    /// 
    /// # Arguments
    /// * `repay_amount` - Amount of USDsol to repay (6 decimals)
    pub fn repay(ctx: Context<Repay>, repay_amount: u64) -> Result<()> {
        instructions::repay::handler(ctx, repay_amount)
    }

    /// Withdraw SOL collateral from vault
    /// Must maintain minimum collateral ratio after withdrawal
    /// 
    /// # Arguments
    /// * `amount` - Amount of SOL (in lamports) to withdraw
    pub fn withdraw_collateral(ctx: Context<WithdrawCollateral>, amount: u64) -> Result<()> {
        instructions::withdraw_collateral::handler(ctx, amount)
    }

    /// Close a vault after repaying all debt
    /// Returns all remaining collateral and liquidation reserve to owner
    pub fn close_vault(ctx: Context<CloseVault>) -> Result<()> {
        instructions::close_vault::handler(ctx)
    }

    /// Liquidate an undercollateralized vault
    /// Anyone can call this to liquidate a vault below MCR (110%) or CCR (150% in Recovery Mode)
    /// Liquidator receives gas compensation + 0.5% bonus
    pub fn liquidate(ctx: Context<Liquidate>) -> Result<()> {
        instructions::liquidate::handler(ctx)
    }

    /// Deposit USDsol into the Stability Pool
    /// Earns liquidation gains (collateral at discount) and MANNA rewards
    /// 
    /// # Arguments
    /// * `amount` - Amount of USDsol to deposit (6 decimals)
    pub fn stability_deposit(ctx: Context<StabilityDepositCtx>, amount: u64) -> Result<()> {
        instructions::stability_deposit::handler(ctx, amount)
    }

    /// Withdraw USDsol and claim rewards from Stability Pool
    /// 
    /// # Arguments
    /// * `amount` - Amount of USDsol to withdraw (6 decimals), 0 for max
    pub fn stability_withdraw(ctx: Context<StabilityWithdrawCtx>, amount: u64) -> Result<()> {
        instructions::stability_withdraw::handler(ctx, amount)
    }

    /// Redeem USDsol for SOL collateral at $1 parity
    /// This is the core peg mechanism - maintains USDsol ≈ $1
    /// Redemptions are fulfilled from vaults with lowest collateral ratios
    /// 
    /// # Arguments
    /// * `usdsol_amount` - Amount of USDsol to redeem (6 decimals)
    pub fn redeem(ctx: Context<Redeem>, usdsol_amount: u64) -> Result<()> {
        instructions::redeem::handler(ctx, usdsol_amount)
    }
}
