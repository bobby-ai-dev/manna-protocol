# Manna Protocol Architecture

## Overview

Manna Protocol is a decentralized borrowing protocol on Solana that allows users to mint USDsol (a USD-pegged stablecoin) against SOL collateral. The protocol implements battle-tested mechanics from Liquity V1, adapted for Solana's account model.

## Core Components

### 1. Global State

```
┌─────────────────────────────────────────────┐
│              GlobalState PDA                │
├─────────────────────────────────────────────┤
│ authority: Pubkey (self-referential)        │
│ usdsol_mint: Pubkey                         │
│ manna_mint: Pubkey                          │
│ price_feed: Pubkey (Pyth)                   │
│ total_collateral: u64                       │
│ total_debt: u64                             │
│ base_rate: u64 (18 decimals)                │
│ last_fee_operation_time: i64                │
│ total_vaults: u64                           │
│ active_vaults: u64                          │
│ is_paused: bool                             │
└─────────────────────────────────────────────┘
```

### 2. Vault (User Position)

Each user has one vault per collateral type. Vaults hold:
- SOL collateral (stored as lamports in the PDA itself)
- Debt (USDsol borrowed + fees)
- Liquidation reserve (returned when vault closes)

```
┌─────────────────────────────────────────────┐
│           Vault PDA [owner, "vault"]        │
├─────────────────────────────────────────────┤
│ owner: Pubkey                               │
│ collateral: u64 (lamports)                  │
│ debt: u64 (USDsol, 6 decimals)              │
│ liquidation_reserve: u64                    │
│ status: VaultStatus                         │
│ opened_at: i64                              │
│ last_updated: i64                           │
└─────────────────────────────────────────────┘
```

### 3. Stability Pool

The Stability Pool is the first line of defense for liquidations. Depositors provide USDsol liquidity and receive:
- Collateral from liquidations (at ~10% discount)
- MANNA token rewards

```
┌─────────────────────────────────────────────┐
│       StabilityPool PDA ["stability_pool"]  │
├─────────────────────────────────────────────┤
│ total_usdsol_deposits: u64                  │
│ total_collateral_gains: u64                 │
│ current_epoch: u64                          │
│ p: u128 (running product)                   │
│ s: u128 (running sum)                       │
│ total_manna_issued: u64                     │
└─────────────────────────────────────────────┘
```

## Protocol Flow

### Opening a Vault and Borrowing

```
User                    Manna Program              Global State
  │                          │                          │
  │── open_vault(SOL) ──────►│                          │
  │                          │── create Vault PDA ─────►│
  │                          │── transfer SOL ─────────►│(vault)
  │                          │── update totals ────────►│
  │                          │                          │
  │── borrow(USDsol) ───────►│                          │
  │                          │── get SOL price ────────►│(Pyth)
  │                          │── calc fee & CR ────────►│
  │                          │── mint USDsol ──────────►│(user)
  │                          │── update vault debt ────►│
  │◄── USDsol tokens ────────│                          │
```

### Liquidation Flow

```
Price Drop                  Liquidator              Stability Pool
  │                            │                         │
  │── CR < 110% ──────────────►│                         │
  │                            │── liquidate() ─────────►│
  │                            │                         │
  │                            │◄── offset debt ─────────│
  │                            │◄── receive collateral ──│
  │                            │                         │
  │                            │── gas compensation ────►│(liquidator)
  │                            │── bonus (0.5%) ────────►│(liquidator)
```

## Key Mechanisms

### 1. Collateral Ratio (CR)

```
CR = (Collateral Value in USD) / Debt

Example:
  Collateral: 10 SOL @ $200 = $2,000
  Debt: 1,500 USDsol
  CR = $2,000 / $1,500 = 133%
```

**Minimum CR (MCR):** 110%
**Critical CR (CCR):** 150% (Recovery Mode threshold)

### 2. Borrowing Fee

One-time fee added to debt at borrow time:

```
fee_rate = max(0.5%, min(5%, base_rate + 0.5%))
borrowing_fee = debt_amount × fee_rate
```

Base rate increases with redemptions, decays over time (50% per 12 hours).

### 3. Stability Pool Mechanics

**Deposit Tracking:**
- Uses running product `P` to track compounded deposits
- When liquidation occurs, deposits are reduced proportionally

**Collateral Gains:**
- Running sum `S` tracks accumulated collateral per unit deposited
- Depositors earn collateral proportional to their share

**Epoch System:**
- When `P` becomes too small, epoch increments
- Protects against precision loss in long-running pools

### 4. Recovery Mode

Triggers when Total Collateral Ratio (TCR) < 150%:

| Normal Mode | Recovery Mode |
|-------------|---------------|
| Liquidate at CR < 110% | Liquidate at CR < 150% |
| Borrowing fee: 0.5-5% | Borrowing fee: 0% |
| Free operations | Operations must improve TCR |

### 5. Redemptions (Future)

Anyone can redeem USDsol for $1 of SOL:
1. System selects lowest-CR vaults
2. Vault debt reduced, collateral transferred
3. Fee (0.5-5%) deducted from collateral
4. Creates hard price floor for USDsol

## Security Model

### Immutability

Core program is designed to be immutable after deployment:
- No upgrade authority
- No admin keys
- Parameters are algorithmic

### Oracle Reliability

- Primary: Pyth Network SOL/USD
- Staleness check: 60 seconds
- Fallback: Switchboard (future)
- Price deviation check for manipulation

### Economic Security

- 110% MCR provides 10% buffer for liquidations
- Stability Pool absorbs bad debt
- Recovery Mode protects system solvency
- Redemptions maintain peg floor

## Account Sizes

| Account | Size (bytes) | Rent (SOL) |
|---------|--------------|------------|
| GlobalState | 250 | ~0.002 |
| Vault | 170 | ~0.002 |
| StabilityPool | 170 | ~0.002 |
| StabilityDeposit | 160 | ~0.002 |

## Program IDs

| Program | Address |
|---------|---------|
| Manna Core | `MaNNa11111111111111111111111111111111111111` |
| USDsol Mint | PDA: `["usdsol_mint"]` |
| MANNA Mint | PDA: `["manna_mint"]` |

## Future Enhancements

1. **Multi-collateral:** jitoSOL, bNSOL branches
2. **Redemptions:** Full peg mechanism
3. **MANNA rewards:** Stability Pool emissions
4. **Frontend:** Web interface for users
5. **Liquidation bot:** Automated liquidator
