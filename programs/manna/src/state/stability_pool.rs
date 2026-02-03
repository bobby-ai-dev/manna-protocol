use anchor_lang::prelude::*;

/// Stability Pool - absorbs liquidated debt in exchange for collateral
#[account]
pub struct StabilityPool {
    /// Total USDsol deposited in the pool
    pub total_usdsol_deposits: u64,
    
    /// Total SOL collateral gained from liquidations
    pub total_collateral_gains: u64,
    
    /// Epoch for tracking deposit snapshots (for reward calculation)
    pub current_epoch: u64,
    
    /// Running product for deposit tracking (P in Liquity)
    pub p: u128,
    
    /// Running sum for collateral gains (S in Liquity)
    pub s: u128,
    
    /// Total MANNA rewards distributed
    pub total_manna_issued: u64,
    
    /// Bump seed for PDA
    pub bump: u8,
    
    /// Reserved for future use
    pub _reserved: [u8; 64],
}

impl StabilityPool {
    pub const LEN: usize = 8 + // discriminator
        8 +  // total_usdsol_deposits
        8 +  // total_collateral_gains
        8 +  // current_epoch
        16 + // p (u128)
        16 + // s (u128)
        8 +  // total_manna_issued
        1 +  // bump
        64;  // reserved
    
    /// Initialize the stability pool
    pub fn initialize(&mut self, bump: u8) {
        self.p = 1_000_000_000_000_000_000; // 1e18 (initial product)
        self.s = 0;
        self.current_epoch = 0;
        self.bump = bump;
    }
    
    /// Process a liquidation - pool absorbs debt, gains collateral
    /// Returns (debt_to_offset, collateral_to_gain)
    pub fn offset_debt(
        &mut self,
        debt_to_liquidate: u64,
        collateral_to_distribute: u64,
    ) -> Result<(u64, u64)> {
        if self.total_usdsol_deposits == 0 {
            return Ok((0, 0));
        }
        
        let debt_to_offset = debt_to_liquidate.min(self.total_usdsol_deposits);
        let collateral_ratio = if debt_to_liquidate > 0 {
            (collateral_to_distribute as u128)
                .saturating_mul(debt_to_offset as u128)
                .checked_div(debt_to_liquidate as u128)
                .unwrap_or(0) as u64
        } else {
            0
        };
        
        // Update pool state
        self.total_usdsol_deposits = self.total_usdsol_deposits.saturating_sub(debt_to_offset);
        self.total_collateral_gains = self.total_collateral_gains.saturating_add(collateral_ratio);
        
        // Update running product P (for deposit tracking)
        if self.total_usdsol_deposits > 0 {
            let new_p = self.p
                .saturating_mul(self.total_usdsol_deposits as u128)
                .checked_div((self.total_usdsol_deposits + debt_to_offset) as u128)
                .unwrap_or(1);
            
            // If P becomes too small, increment epoch
            if new_p < 1_000_000_000 {
                self.current_epoch += 1;
                self.p = 1_000_000_000_000_000_000;
            } else {
                self.p = new_p;
            }
        }
        
        // Update running sum S (for collateral gain tracking)
        if self.total_usdsol_deposits > 0 {
            let collateral_per_unit = (collateral_ratio as u128)
                .saturating_mul(1_000_000_000_000_000_000)
                .checked_div(self.total_usdsol_deposits as u128)
                .unwrap_or(0);
            self.s = self.s.saturating_add(collateral_per_unit);
        }
        
        Ok((debt_to_offset, collateral_ratio))
    }
}

/// Individual depositor's position in the Stability Pool
#[account]
#[derive(Default)]
pub struct StabilityDeposit {
    /// Owner of this deposit
    pub owner: Pubkey,
    
    /// Initial deposit amount
    pub initial_deposit: u64,
    
    /// Snapshot of P at deposit time
    pub snapshot_p: u128,
    
    /// Snapshot of S at deposit time
    pub snapshot_s: u128,
    
    /// Snapshot of epoch at deposit time
    pub snapshot_epoch: u64,
    
    /// Accumulated collateral gains (claimable)
    pub collateral_gains: u64,
    
    /// Accumulated MANNA rewards (claimable)
    pub manna_rewards: u64,
    
    /// Timestamp of deposit
    pub deposited_at: i64,
    
    /// Bump seed for PDA
    pub bump: u8,
    
    /// Reserved for future use
    pub _reserved: [u8; 32],
}

impl StabilityDeposit {
    pub const LEN: usize = 8 + // discriminator
        32 + // owner
        8 +  // initial_deposit
        16 + // snapshot_p
        16 + // snapshot_s
        8 +  // snapshot_epoch
        8 +  // collateral_gains
        8 +  // manna_rewards
        8 +  // deposited_at
        1 +  // bump
        32;  // reserved
    
    /// Calculate current compounded deposit value
    pub fn get_compounded_deposit(&self, pool: &StabilityPool) -> u64 {
        if self.snapshot_epoch < pool.current_epoch {
            return 0; // Deposit was zeroed out in a previous epoch
        }
        
        if self.snapshot_p == 0 {
            return 0;
        }
        
        // compounded = initial * (current_P / snapshot_P)
        let compounded = (self.initial_deposit as u128)
            .saturating_mul(pool.p)
            .checked_div(self.snapshot_p)
            .unwrap_or(0);
        
        compounded as u64
    }
    
    /// Calculate pending collateral gains
    pub fn get_pending_collateral_gains(&self, pool: &StabilityPool) -> u64 {
        if self.snapshot_epoch < pool.current_epoch {
            // If epoch changed, depositor gets their full share of that liquidation
            return self.initial_deposit; // Simplified - full collateral for zeroed deposit
        }
        
        // gains = initial_deposit * (current_S - snapshot_S) / 1e18
        let gain = (self.initial_deposit as u128)
            .saturating_mul(pool.s.saturating_sub(self.snapshot_s))
            .checked_div(1_000_000_000_000_000_000)
            .unwrap_or(0);
        
        gain as u64
    }
}
