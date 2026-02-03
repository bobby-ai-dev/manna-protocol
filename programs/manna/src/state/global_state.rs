use anchor_lang::prelude::*;

/// Global protocol state - singleton PDA
#[account]
pub struct GlobalState {
    /// Authority that can perform admin operations (set to program for immutability)
    pub authority: Pubkey,
    
    /// USDsol mint address
    pub usdsol_mint: Pubkey,
    
    /// MANNA token mint address
    pub manna_mint: Pubkey,
    
    /// Pyth price feed account
    pub price_feed: Pubkey,
    
    /// Total SOL collateral across all vaults (in lamports)
    pub total_collateral: u64,
    
    /// Total USDsol debt across all vaults
    pub total_debt: u64,
    
    /// Current base rate for borrowing/redemption fees (18 decimals)
    pub base_rate: u64,
    
    /// Timestamp of last fee operation
    pub last_fee_operation_time: i64,
    
    /// Total vaults ever created
    pub total_vaults: u64,
    
    /// Active vaults count
    pub active_vaults: u64,
    
    /// Is protocol paused (emergency only)
    pub is_paused: bool,
    
    /// Bump seed for PDA
    pub bump: u8,
    
    /// Reserved for future use
    pub _reserved: [u8; 64],
}

impl GlobalState {
    pub const LEN: usize = 8 + // discriminator
        32 + // authority
        32 + // usdsol_mint
        32 + // manna_mint
        32 + // price_feed
        8 +  // total_collateral
        8 +  // total_debt
        8 +  // base_rate
        8 +  // last_fee_operation_time
        8 +  // total_vaults
        8 +  // active_vaults
        1 +  // is_paused
        1 +  // bump
        64;  // reserved
    
    /// Calculate the Total Collateral Ratio (TCR)
    /// TCR = (total_collateral * price) / total_debt
    /// Returns value in 18 decimal precision
    pub fn calculate_tcr(&self, sol_price: u64) -> Option<u64> {
        if self.total_debt == 0 {
            return Some(u64::MAX); // Infinite CR if no debt
        }
        
        // collateral_value = total_collateral (lamports) * price (6 decimals) / 1e9
        // TCR = collateral_value * 1e18 / total_debt (6 decimals)
        let collateral_value = (self.total_collateral as u128)
            .checked_mul(sol_price as u128)?;
        
        let tcr = collateral_value
            .checked_mul(1_000_000_000_000_000_000)? // 1e18
            .checked_div(self.total_debt as u128)?
            .checked_div(1_000_000_000)?; // Adjust for lamports
        
        Some(tcr as u64)
    }
    
    /// Check if system is in Recovery Mode (TCR < 150%)
    pub fn is_recovery_mode(&self, sol_price: u64) -> bool {
        match self.calculate_tcr(sol_price) {
            Some(tcr) => tcr < crate::constants::CCR,
            None => false,
        }
    }
    
    /// Update base rate (called on redemptions)
    pub fn update_base_rate(&mut self, redeemed_amount: u64, current_time: i64) {
        // Decay existing base rate
        self.decay_base_rate(current_time);
        
        // Increase based on redemption size relative to total supply
        if self.total_debt > 0 {
            let increase = ((redeemed_amount as u128)
                .saturating_mul(500_000_000_000_000_000) // 0.5 in 18 decimals
                .checked_div(self.total_debt as u128)
                .unwrap_or(0)) as u64;
            
            self.base_rate = self.base_rate.saturating_add(increase);
        }
        
        self.last_fee_operation_time = current_time;
    }
    
    /// Decay base rate based on time elapsed
    pub fn decay_base_rate(&mut self, current_time: i64) {
        let seconds_elapsed = current_time.saturating_sub(self.last_fee_operation_time);
        
        if seconds_elapsed <= 0 {
            return;
        }
        
        // Exponential decay: base_rate * decay_factor^seconds
        // Simplified: decay by 50% every 12 hours
        let decay_factor = crate::constants::BASE_RATE_DECAY_FACTOR;
        
        for _ in 0..seconds_elapsed.min(86400) {
            self.base_rate = ((self.base_rate as u128)
                .saturating_mul(decay_factor as u128)
                .checked_div(1_000_000_000_000_000_000)
                .unwrap_or(0)) as u64;
        }
    }
    
    /// Calculate borrowing fee rate
    pub fn get_borrowing_fee_rate(&self) -> u64 {
        let fee = self.base_rate.saturating_add(crate::constants::BORROWING_FEE_FLOOR);
        fee.min(crate::constants::BORROWING_FEE_CAP)
    }
    
    /// Calculate borrowing fee for an amount
    pub fn calculate_borrowing_fee(&self, debt_amount: u64) -> u64 {
        let fee_rate = self.get_borrowing_fee_rate();
        ((debt_amount as u128)
            .saturating_mul(fee_rate as u128)
            .checked_div(1_000_000_000_000_000_000)
            .unwrap_or(0)) as u64
    }
}
