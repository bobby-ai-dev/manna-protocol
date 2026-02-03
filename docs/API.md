# Manna Protocol API Reference

## Instructions

### initialize

Initialize the Manna protocol. Can only be called once.

```rust
pub fn initialize(ctx: Context<Initialize>) -> Result<()>
```

**Accounts:**
| Name | Type | Description |
|------|------|-------------|
| authority | Signer | Initial protocol authority |
| global_state | Account | Global state PDA (created) |
| usdsol_mint | Account | USDsol token mint (created) |
| manna_mint | Account | MANNA token mint (created) |
| stability_pool | Account | Stability pool PDA (created) |
| price_feed | AccountInfo | Pyth SOL/USD price feed |
| token_program | Program | Token-2022 program |
| system_program | Program | System program |

---

### open_vault

Open a new vault and deposit initial collateral.

```rust
pub fn open_vault(ctx: Context<OpenVault>, collateral_amount: u64) -> Result<()>
```

**Arguments:**
| Name | Type | Description |
|------|------|-------------|
| collateral_amount | u64 | Amount of SOL in lamports |

**Accounts:**
| Name | Type | Description |
|------|------|-------------|
| owner | Signer | Vault owner |
| global_state | Account | Global state PDA |
| vault | Account | New vault PDA (created) |
| system_program | Program | System program |

---

### deposit_collateral

Deposit additional SOL collateral into an existing vault.

```rust
pub fn deposit_collateral(ctx: Context<DepositCollateral>, amount: u64) -> Result<()>
```

**Arguments:**
| Name | Type | Description |
|------|------|-------------|
| amount | u64 | Amount of SOL in lamports |

**Accounts:**
| Name | Type | Description |
|------|------|-------------|
| owner | Signer | Vault owner |
| global_state | Account | Global state PDA |
| vault | Account | Owner's vault PDA |
| system_program | Program | System program |

---

### borrow

Borrow USDsol against vault collateral. A one-time fee is added to debt.

```rust
pub fn borrow(ctx: Context<Borrow>, borrow_amount: u64) -> Result<()>
```

**Arguments:**
| Name | Type | Description |
|------|------|-------------|
| borrow_amount | u64 | USDsol to borrow (6 decimals) |

**Accounts:**
| Name | Type | Description |
|------|------|-------------|
| owner | Signer | Vault owner |
| global_state | Account | Global state PDA |
| vault | Account | Owner's vault PDA |
| usdsol_mint | Account | USDsol mint PDA |
| owner_usdsol | Account | Owner's USDsol token account |
| price_feed | Account | Pyth price feed |
| token_program | Program | Token-2022 program |

**Errors:**
- `BelowMinimumCollateralRatio` - Resulting CR would be < 110%
- `BelowMinimumDebt` - Debt would be < 200 USDsol
- `RecoveryModeActive` - Operation not allowed in Recovery Mode

---

### repay

Repay USDsol debt. Tokens are burned.

```rust
pub fn repay(ctx: Context<Repay>, repay_amount: u64) -> Result<()>
```

**Arguments:**
| Name | Type | Description |
|------|------|-------------|
| repay_amount | u64 | USDsol to repay (6 decimals) |

**Accounts:**
| Name | Type | Description |
|------|------|-------------|
| owner | Signer | Vault owner |
| global_state | Account | Global state PDA |
| vault | Account | Owner's vault PDA |
| usdsol_mint | Account | USDsol mint PDA |
| owner_usdsol | Account | Owner's USDsol token account |
| token_program | Program | Token-2022 program |

---

### withdraw_collateral

Withdraw SOL collateral from vault. Must maintain MCR.

```rust
pub fn withdraw_collateral(ctx: Context<WithdrawCollateral>, amount: u64) -> Result<()>
```

**Arguments:**
| Name | Type | Description |
|------|------|-------------|
| amount | u64 | SOL to withdraw in lamports |

**Accounts:**
| Name | Type | Description |
|------|------|-------------|
| owner | Signer | Vault owner |
| global_state | Account | Global state PDA |
| vault | Account | Owner's vault PDA |
| price_feed | Account | Pyth price feed |
| system_program | Program | System program |

**Errors:**
- `WithdrawalWouldBreachMCR` - Withdrawal would drop CR below 110%
- `RecoveryModeActive` - Withdrawal not improving TCR

---

### close_vault

Close a vault after repaying all debt. Returns collateral + liquidation reserve.

```rust
pub fn close_vault(ctx: Context<CloseVault>) -> Result<()>
```

**Accounts:**
| Name | Type | Description |
|------|------|-------------|
| owner | Signer | Vault owner |
| global_state | Account | Global state PDA |
| vault | Account | Owner's vault PDA (closed) |
| usdsol_mint | Account | USDsol mint PDA |
| owner_usdsol | Account | Owner's USDsol token account |
| token_program | Program | Token-2022 program |
| system_program | Program | System program |

---

### liquidate

Liquidate an undercollateralized vault. Anyone can call.

```rust
pub fn liquidate(ctx: Context<Liquidate>) -> Result<()>
```

**Accounts:**
| Name | Type | Description |
|------|------|-------------|
| liquidator | Signer | Caller triggering liquidation |
| global_state | Account | Global state PDA |
| vault | Account | Vault to liquidate |
| vault_owner | AccountInfo | Vault owner (for PDA) |
| stability_pool | Account | Stability pool PDA |
| usdsol_mint | Account | USDsol mint PDA |
| liquidator_usdsol | Account | Liquidator's USDsol token account |
| price_feed | Account | Pyth price feed |
| token_program | Program | Token-2022 program |
| system_program | Program | System program |

**Rewards:**
- Liquidation reserve (50 USDsol) as gas compensation
- 0.5% of liquidated collateral as bonus

---

### stability_deposit

Deposit USDsol to the Stability Pool.

```rust
pub fn stability_deposit(ctx: Context<StabilityDepositCtx>, amount: u64) -> Result<()>
```

**Arguments:**
| Name | Type | Description |
|------|------|-------------|
| amount | u64 | USDsol to deposit (6 decimals) |

**Accounts:**
| Name | Type | Description |
|------|------|-------------|
| depositor | Signer | Depositor |
| global_state | Account | Global state PDA |
| stability_pool | Account | Stability pool PDA |
| deposit_record | Account | Depositor's record (created if needed) |
| usdsol_mint | Account | USDsol mint PDA |
| depositor_usdsol | Account | Depositor's USDsol token account |
| pool_usdsol | Account | Pool's USDsol token account |
| token_program | Program | Token-2022 program |
| system_program | Program | System program |

---

### stability_withdraw

Withdraw USDsol and claim rewards from Stability Pool.

```rust
pub fn stability_withdraw(ctx: Context<StabilityWithdrawCtx>, amount: u64) -> Result<()>
```

**Arguments:**
| Name | Type | Description |
|------|------|-------------|
| amount | u64 | USDsol to withdraw (6 decimals), 0 for max |

**Accounts:**
| Name | Type | Description |
|------|------|-------------|
| depositor | Signer | Depositor |
| global_state | Account | Global state PDA |
| stability_pool | Account | Stability pool PDA |
| deposit_record | Account | Depositor's record |
| owner | AccountInfo | Record owner |
| usdsol_mint | Account | USDsol mint PDA |
| depositor_usdsol | Account | Depositor's USDsol token account |
| pool_usdsol | Account | Pool's USDsol token account |
| token_program | Program | Token-2022 program |
| system_program | Program | System program |

---

## Error Codes

| Code | Name | Description |
|------|------|-------------|
| 6000 | BelowMinimumCollateralRatio | CR would be < 110% |
| 6001 | BelowMinimumDebt | Debt would be < 200 USDsol |
| 6002 | VaultNotActive | Vault is not active |
| 6003 | VaultAlreadyExists | Vault already exists |
| 6004 | InsufficientCollateral | Not enough collateral |
| 6005 | InsufficientDebt | Repaying more than debt |
| 6006 | VaultNotLiquidatable | Vault is healthy |
| 6007 | RecoveryModeActive | Operation restricted |
| 6008 | MathOverflow | Arithmetic overflow |
| 6009 | MathUnderflow | Arithmetic underflow |
| 6010 | InvalidOraclePrice | Bad price data |
| 6011 | StalePriceData | Price feed too old |
| 6012 | InsufficientStabilityPoolFunds | SP too small |
| 6013 | InvalidRedemptionAmount | Bad redemption |
| 6014 | WithdrawalWouldBreachMCR | Would drop below MCR |
| 6015 | ProtocolPaused | Protocol is paused |
| 6016 | Unauthorized | Not authorized |
| 6017 | InvalidParameter | Invalid parameter |
| 6018 | VaultHasDebt | Cannot close with debt |
| 6019 | ZeroAmount | Amount must be > 0 |

---

## TypeScript SDK

```typescript
import { MannaSDK } from '@manna/sdk';
import { Connection, Keypair } from '@solana/web3.js';

const connection = new Connection('https://api.devnet.solana.com');
const wallet = Keypair.fromSecretKey(/* ... */);
const manna = new MannaSDK(connection, wallet);

// Open vault with 10 SOL
const tx1 = await manna.openVault(10_000_000_000);

// Borrow 1000 USDsol
const tx2 = await manna.borrow(1_000_000_000);

// Check vault status
const vault = await manna.getVault(wallet.publicKey);
console.log(`CR: ${vault.collateralRatio}%`);

// Deposit to stability pool
const tx3 = await manna.stabilityDeposit(500_000_000);
```
