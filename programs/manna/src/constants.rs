// Protocol constants for Manna

/// Minimum collateral ratio (110% = 1.1e18)
pub const MCR: u64 = 1_100_000_000_000_000_000; // 110% in 18 decimals

/// Critical collateral ratio for Recovery Mode (150%)
pub const CCR: u64 = 1_500_000_000_000_000_000; // 150% in 18 decimals

/// Minimum debt amount (200 USDsol)
pub const MIN_DEBT: u64 = 200_000_000; // 200 USDsol (6 decimals)

/// Liquidation reserve (50 USDsol) - gas compensation for liquidators
pub const LIQUIDATION_RESERVE: u64 = 50_000_000; // 50 USDsol (6 decimals)

/// Base borrowing fee (0.5%)
pub const BORROWING_FEE_FLOOR: u64 = 5_000_000_000_000_000; // 0.5% in 18 decimals

/// Max borrowing fee (5%)
pub const BORROWING_FEE_CAP: u64 = 50_000_000_000_000_000; // 5% in 18 decimals

/// Base rate decay factor - halves every 12 hours
pub const BASE_RATE_DECAY_FACTOR: u64 = 999_991_434_000_000_000; // per-second decay

/// Seconds in 12 hours
pub const SECONDS_IN_12_HOURS: i64 = 43_200;

/// Precision for math operations
pub const DECIMAL_PRECISION: u64 = 1_000_000_000_000_000_000; // 1e18

/// Liquidation penalty (10%)
pub const LIQUIDATION_PENALTY: u64 = 100_000_000_000_000_000; // 10% in 18 decimals

/// USDsol decimals
pub const USDSOL_DECIMALS: u8 = 6;

/// SOL decimals (lamports)
pub const SOL_DECIMALS: u8 = 9;

/// Pyth SOL/USD price feed ID (mainnet)
pub const PYTH_SOL_USD_FEED: &str = "H6ARHf6YXhGYeQfUzQNGk6rDNnLBQKrenN712K4AQJEG";

/// Seeds for PDA derivation
pub const GLOBAL_STATE_SEED: &[u8] = b"global_state";
pub const VAULT_SEED: &[u8] = b"vault";
pub const STABILITY_POOL_SEED: &[u8] = b"stability_pool";
pub const USDSOL_MINT_SEED: &[u8] = b"usdsol_mint";
pub const MANNA_MINT_SEED: &[u8] = b"manna_mint";
