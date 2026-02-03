use anchor_lang::prelude::*;

#[error_code]
pub enum MannaError {
    #[msg("Collateral ratio below minimum (110%)")]
    BelowMinimumCollateralRatio,

    #[msg("Debt amount below minimum (200 USDsol)")]
    BelowMinimumDebt,

    #[msg("Vault is not active")]
    VaultNotActive,

    #[msg("Vault already exists")]
    VaultAlreadyExists,

    #[msg("Insufficient collateral")]
    InsufficientCollateral,

    #[msg("Insufficient debt to repay")]
    InsufficientDebt,

    #[msg("Cannot liquidate healthy vault")]
    VaultNotLiquidatable,

    #[msg("Recovery mode active - operation restricted")]
    RecoveryModeActive,

    #[msg("Math overflow")]
    MathOverflow,

    #[msg("Math underflow")]
    MathUnderflow,

    #[msg("Invalid oracle price")]
    InvalidOraclePrice,

    #[msg("Oracle price is stale")]
    StalePriceData,

    #[msg("Stability pool has insufficient funds")]
    InsufficientStabilityPoolFunds,

    #[msg("Invalid redemption amount")]
    InvalidRedemptionAmount,

    #[msg("Cannot withdraw - would breach minimum CR")]
    WithdrawalWouldBreachMCR,

    #[msg("Protocol is paused")]
    ProtocolPaused,

    #[msg("Unauthorized")]
    Unauthorized,

    #[msg("Invalid parameter")]
    InvalidParameter,

    #[msg("Vault has outstanding debt")]
    VaultHasDebt,

    #[msg("Zero amount not allowed")]
    ZeroAmount,

    #[msg("Protocol already initialized")]
    AlreadyInitialized,
}
