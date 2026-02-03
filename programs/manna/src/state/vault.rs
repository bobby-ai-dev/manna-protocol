use anchor_lang::prelude::*;

/// Status of a vault
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, PartialEq, Eq, Default)]
pub enum VaultStatus {
    #[default]
    Inactive,
    Active,
    ClosedByOwner,
    Liquidated,
}

/// Individual user vault (position)
#[account]
#[derive(Default)]
pub struct Vault {
    /// Owner of the vault
    pub owner: Pubkey,
    
    /// SOL collateral amount (in lamports)
    pub collateral: u64,
    
    /// USDsol debt amount (6 decimals)
    pub debt: u64,
    
    /// Liquidation reserve (returned on close)
    pub liquidation_reserve: u64,
    
    /// Vault status
    pub status: VaultStatus,
    
    /// Timestamp when vault was opened
    pub opened_at: i64,
    
    /// Timestamp of last update
    pub last_updated: i64,
    
    /// Bump seed for PDA
    pub bump: u8,
    
    /// Reserved for future use
    pub _reserved: [u8; 32],
}

impl Vault {
    pub const LEN: usize = 8 + // discriminator
        32 + // owner
        8 +  // collateral
        8 +  // debt
        8 +  // liquidation_reserve
        1 +  // status
        8 +  // opened_at
        8 +  // last_updated
        1 +  // bump
        32;  // reserved
    
    /// Calculate the collateral ratio (CR) for this vault
    /// CR = (collateral * price) / debt
    /// Returns value in 18 decimal precision
    pub fn calculate_cr(&self, sol_price: u64) -> Option<u64> {
        if self.debt == 0 {
            return Some(u64::MAX); // Infinite CR if no debt
        }
        
        // collateral is in lamports (9 decimals)
        // price is in USD (6 decimals)
        // debt is in USDsol (6 decimals)
        // We want CR in 18 decimal precision
        
        // collateral_value = collateral * price / 1e9 (to get USD value)
        // CR = collateral_value * 1e18 / debt
        let collateral_value = (self.collateral as u128)
            .checked_mul(sol_price as u128)?;
        
        let cr = collateral_value
            .checked_mul(1_000_000_000_000_000_000)? // 1e18
            .checked_div(self.debt as u128)?
            .checked_div(1_000_000_000)?; // Adjust for lamports (1e9)
        
        Some(cr as u64)
    }
    
    /// Check if vault is liquidatable (CR < MCR)
    pub fn is_liquidatable(&self, sol_price: u64) -> bool {
        match self.calculate_cr(sol_price) {
            Some(cr) => cr < crate::constants::MCR,
            None => false,
        }
    }
    
    /// Check if vault is in ICR (Individual CR) danger zone for Recovery Mode
    /// In Recovery Mode, vaults with CR < CCR (150%) can be liquidated
    pub fn is_liquidatable_recovery_mode(&self, sol_price: u64) -> bool {
        match self.calculate_cr(sol_price) {
            Some(cr) => cr < crate::constants::CCR,
            None => false,
        }
    }
    
    /// Calculate maximum borrowable amount given current collateral and price
    pub fn max_borrowable(&self, sol_price: u64) -> u64 {
        // max_debt = collateral_value / MCR
        // collateral_value = collateral * price / 1e9
        let collateral_value = (self.collateral as u128)
            .saturating_mul(sol_price as u128)
            .checked_div(1_000_000_000) // lamports to SOL
            .unwrap_or(0);
        
        let max_debt = collateral_value
            .saturating_mul(1_000_000_000_000_000_000) // 1e18
            .checked_div(crate::constants::MCR as u128)
            .unwrap_or(0);
        
        max_debt.saturating_sub(self.debt as u128) as u64
    }
    
    /// Calculate required collateral for a given debt amount
    pub fn required_collateral_for_debt(debt: u64, sol_price: u64) -> u64 {
        if sol_price == 0 {
            return u64::MAX;
        }
        
        // required = debt * MCR / price * 1e9 (to get lamports)
        let required = (debt as u128)
            .saturating_mul(crate::constants::MCR as u128)
            .saturating_mul(1_000_000_000) // Convert to lamports
            .checked_div(sol_price as u128)
            .unwrap_or(u64::MAX as u128)
            .checked_div(1_000_000_000_000_000_000) // Divide by 1e18 (MCR precision)
            .unwrap_or(u64::MAX as u128);
        
        required as u64
    }
    
    /// Get total debt including liquidation reserve
    pub fn total_debt(&self) -> u64 {
        self.debt.saturating_add(self.liquidation_reserve)
    }
}
