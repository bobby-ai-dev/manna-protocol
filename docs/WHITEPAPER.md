# Manna Protocol

> **Decentralized Borrowing on Solana** — Mint USDsol against SOL and liquid staking tokens with algorithmic interest rates, redemptions, and battle-tested mechanisms inspired by Liquity.

---

## Table of Contents

1. [Overview](#overview)
2. [How It Works](#how-it-works)
3. [Core Mechanisms](#core-mechanisms)
   - [Collateral Branches](#collateral-branches)
   - [Minting USDsol](#minting-usdsol)
   - [Interest Rates](#interest-rates)
   - [Redemptions](#redemptions)
   - [Stability Pool](#stability-pool)
   - [Liquidations](#liquidations)
   - [Recovery Mode](#recovery-mode)
4. [Tokenomics ($MANNA)](#tokenomics-manna)
5. [Technical Architecture](#technical-architecture)
6. [Oracle Design](#oracle-design)
7. [Security Considerations](#security-considerations)
8. [Comparison to Other Protocols](#comparison-to-other-protocols)
9. [Roadmap](#roadmap)

---

## Overview

Manna Protocol is a decentralized borrowing protocol built natively on Solana. It enables users to mint **USDsol**, a USD-pegged stablecoin, by depositing collateral including SOL and liquid staking tokens (LSTs) like jitoSOL and bNSOL.

### Vision

DeFi on Solana deserves a truly decentralized stablecoin with:
- **No governance** — Protocol parameters are algorithmic, not decided by committees
- **Immutable contracts** — Once deployed, the core protocol cannot be changed
- **Capital efficiency** — Borrow at 110% minimum collateral ratio
- **Real yield** — Stability Pool depositors and MANNA stakers earn sustainable protocol fees
- **Hard peg mechanics** — Redemptions guarantee USDsol never trades significantly below $1

Manna adapts the battle-tested mechanisms pioneered by Liquity V1 on Ethereum, redesigned for Solana's account model and enhanced with support for yield-bearing LST collateral.

### Key Features

| Feature | Description |
|---------|-------------|
| **Multi-collateral** | SOL, jitoSOL, bNSOL — expandable to new LSTs |
| **Collateral isolation** | Each collateral type has its own branch and risk parameters |
| **Algorithmic interest** | One-time borrowing fees adjust based on redemption activity |
| **Redemptions** | Any USDsol holder can redeem for $1 of collateral |
| **Stability Pool** | First-line defense for liquidations, earns MANNA + collateral |
| **Recovery Mode** | System-wide protection triggers at 150% TCR |
| **LST yield preservation** | Staking rewards stay with collateral depositors |
| **Immutable** | Core programs are non-upgradeable after deployment |

---

## How It Works

### For Borrowers

1. **Open a Position**: Deposit collateral (SOL, jitoSOL, or bNSOL) into a position (called a "Vault")
2. **Mint USDsol**: Borrow USDsol against your collateral at minimum 110% collateral ratio
3. **Pay a one-time fee**: Borrowing fee (0.5%–5%) is added to your debt based on system state
4. **Maintain your position**: Keep your collateral ratio above 110% to avoid liquidation
5. **Repay anytime**: No lock-up periods — repay your debt whenever you want
6. **Claim collateral**: After repaying, withdraw your collateral

### For Stability Providers

1. **Deposit USDsol**: Add USDsol to the Stability Pool for your chosen collateral branch
2. **Earn MANNA**: Receive continuous MANNA token emissions
3. **Earn liquidation gains**: When vaults are liquidated, your USDsol is exchanged for collateral at ~10% discount
4. **Withdraw anytime**: No lock-up (except during active liquidations)

### For MANNA Stakers

1. **Stake MANNA**: Lock your MANNA tokens in the staking contract
2. **Earn protocol fees**: Receive a share of all borrowing and redemption fees
3. **Participate in collateral voting**: Vote on which new collaterals to add (only expandability, no other governance)

### For Arbitrageurs

1. **Redemptions**: When USDsol trades below $1, redeem USDsol for $1 of collateral
2. **Liquidations**: Trigger liquidations on undercollateralized vaults for gas compensation rewards

---

## Core Mechanisms

### Collateral Branches

Manna implements **collateral isolation** through separate branches. Each supported collateral type operates in its own isolated environment:

```
┌─────────────────────────────────────────────────────────────┐
│                      MANNA PROTOCOL                         │
├──────────────────┬──────────────────┬──────────────────────┤
│   SOL Branch     │  jitoSOL Branch  │   bNSOL Branch       │
├──────────────────┼──────────────────┼──────────────────────┤
│ • Vaults         │ • Vaults         │ • Vaults             │
│ • Stability Pool │ • Stability Pool │ • Stability Pool     │
│ • Sorted List    │ • Sorted List    │ • Sorted List        │
│ • Interest Rate  │ • Interest Rate  │ • Interest Rate      │
└──────────────────┴──────────────────┴──────────────────────┘
                              │
                              ▼
                    ┌─────────────────┐
                    │     USDsol      │
                    │  (Unified Mint) │
                    └─────────────────┘
```

**Why isolation matters:**
- **Risk containment**: Issues with one collateral don't affect others
- **Separate parameters**: Each branch can have tuned liquidation ratios
- **Independent redemptions**: Redemptions pull from the branch with lowest-CR vaults
- **LST yield isolation**: jitoSOL/bNSOL appreciation stays within their branches

**Initial Collaterals:**

| Collateral | Type | Notes |
|------------|------|-------|
| SOL | Native | Base collateral, most liquid |
| jitoSOL | LST | Jito Network staking + MEV rewards |
| bNSOL | LST | Binance liquid staking token |

**Adding New Collaterals:**

The deployer retains authority to add new collateral branches via a separate "Collateral Registry" program. This is the **only** form of changeability — core lending logic remains immutable. New collateral additions require:
- Pyth price feed availability
- Sufficient on-chain liquidity
- Community signaling via MANNA voting

### Minting USDsol

USDsol is minted when borrowers draw debt against their collateral:

```
User deposits 10 SOL (@ $200 = $2,000)
├── Minimum debt: 200 USDsol (anti-dust)
├── Maximum debt: $2,000 / 1.10 = ~1,818 USDsol
├── User borrows: 1,500 USDsol
├── Borrowing fee: 0.5% × 1,500 = 7.5 USDsol
├── Liquidation reserve: 50 USDsol (returned on close)
└── Total debt: 1,557.5 USDsol
    Collateral ratio: $2,000 / $1,557.5 = 128.4%
```

**Minting Rules:**
- Minimum collateral ratio: **110%** (Normal Mode)
- Minimum debt: **200 USDsol** (prevents dust positions)
- Liquidation reserve: **50 USDsol** (covers gas costs for liquidators)
- Borrowing fee: **0.5% – 5%** (algorithmically determined)

### Interest Rates

Unlike traditional DeFi protocols with continuous interest rates, Manna uses Liquity V1's elegant **one-time borrowing fee** model:

**How it works:**
- When you borrow, a fee is calculated and added to your debt immediately
- No ongoing interest accrues — your debt stays constant until repayment
- The fee rate is algorithmically determined based on redemption activity

**Fee Algorithm:**

```
baseRate starts at 0%

On each redemption:
  baseRate += (redemptionAmount / totalUSDsolSupply) × 0.5

Over time:
  baseRate decays by 50% every 12 hours

borrowingFee = max(0.5%, min(5%, baseRate + 0.5%))
```

**Economic Logic:**
- High redemption activity → USDsol likely below peg → increase fee to discourage borrowing
- Low redemption activity → system healthy → fees decay toward minimum
- Creates automatic monetary policy without governance

**Comparison to other approaches:**

| Protocol | Interest Model | Pros | Cons |
|----------|---------------|------|------|
| Liquity V1 / Manna | One-time fee | Predictable, no ongoing costs | Can't adjust to long-term rate environments |
| Liquity V2 | User-set rates | Flexible, market-driven | Complex, requires active management |
| MakerDAO | Governance rates | Can respond to macro | Slow, political, centralized |
| Aave | Algorithmic utilization | Responds to demand | Volatile, unpredictable |

### Redemptions

Redemptions are **the core peg mechanism** for USDsol. Anyone can redeem USDsol for $1 worth of collateral at any time.

**How redemptions work:**

```
1. Redeemer submits 1,000 USDsol for redemption
2. System finds vault(s) with LOWEST collateral ratio
3. Vault's debt is reduced, collateral transferred to redeemer
4. Redemption fee (0.5% - 5%) is deducted from collateral

Example:
  Redeem: 1,000 USDsol
  Current redemption fee: 1%
  Collateral received: $1,000 × 0.99 = $990 worth of SOL
  Vault owner loses: 1,000 USDsol debt, ~$1,000 collateral
  Net effect on vault: No loss (debt reduced = collateral reduced)
```

**Why redemptions maintain the peg:**
- USDsol trading at $0.98? Buy it, redeem for $1 of collateral = instant profit
- This arbitrage creates a hard price floor around $1 (minus fees)
- Borrowers with low collateral ratios are redeemed first (incentivizes healthy ratios)

**Redemption fee:**
- Same `baseRate` mechanism as borrowing fees
- Fee goes to MANNA stakers
- Increases with redemption volume, decays over time

**Cross-branch redemptions:**

When USDsol is redeemed, the system selects collateral from the branch with the lowest-CR vault:

```
SOL Branch lowest vault CR: 115%
jitoSOL Branch lowest vault CR: 125%
bNSOL Branch lowest vault CR: 140%

→ Redemption pulls from SOL branch
```

### Stability Pool

The Stability Pool is the **first line of defense** against undercollateralization. It's a pool of USDsol that absorbs liquidated debt in exchange for discounted collateral.

**How it works:**

```
┌──────────────────────────────────────────────────────────┐
│                    STABILITY POOL                         │
│                   (per collateral branch)                 │
├──────────────────────────────────────────────────────────┤
│  Depositors provide USDsol ──────────► Pool Balance      │
│                                              │            │
│  When liquidation occurs:                    ▼            │
│  ┌─────────────────────────────────────────────────┐     │
│  │ Pool's USDsol pays off vault debt               │     │
│  │ Pool receives vault's collateral                │     │
│  │ Depositors share collateral pro-rata            │     │
│  └─────────────────────────────────────────────────┘     │
│                                                          │
│  Depositors earn:                                        │
│  • Liquidation gains (~10% discount on collateral)       │
│  • MANNA token emissions (continuous)                    │
└──────────────────────────────────────────────────────────┘
```

**Example liquidation:**

```
Vault being liquidated:
  Debt: 1,000 USDsol
  Collateral: 1.1 SOL (worth $1,100 at $1,000/SOL)
  CR: 110% (at liquidation threshold)

Stability Pool:
  Total USDsol: 100,000
  Your deposit: 10,000 (10% share)

After liquidation:
  Pool loses: 1,000 USDsol
  Pool gains: 1.1 SOL
  
  Your USDsol: 10,000 - 100 = 9,900 USDsol
  Your SOL gain: 0.11 SOL (worth $110)
  
  Net gain: $110 - $100 = $10 (10% profit)
```

**Branch-specific pools:**

Each collateral branch has its own Stability Pool:
- SOL Stability Pool → receives SOL from liquidations
- jitoSOL Stability Pool → receives jitoSOL from liquidations
- bNSOL Stability Pool → receives bNSOL from liquidations

Depositors choose which pool(s) to deposit into based on their risk/reward preferences.

### Liquidations

Vaults become liquidatable when their collateral ratio falls below 110% (Normal Mode) or 150% (Recovery Mode).

**Liquidation process:**

```
                    Vault CR < MCR?
                          │
                          ▼
              ┌───────────────────────┐
              │   Stability Pool      │
              │   has enough USDsol?  │
              └───────────────────────┘
                    │           │
                   Yes          No
                    │           │
                    ▼           ▼
            ┌──────────┐  ┌──────────────┐
            │  Pool    │  │Redistribution│
            │Liquidation│  │  to other   │
            │          │  │   vaults    │
            └──────────┘  └──────────────┘
```

**Pool Liquidation (primary):**
- Stability Pool USDsol absorbs debt
- Collateral distributed to pool depositors
- Clean, efficient, incentivized

**Redistribution (fallback):**
- If Stability Pool is empty, debt and collateral are redistributed
- Goes to all other vaults in the same branch, proportional to their collateral
- Increases everyone's debt and collateral slightly

**Liquidation incentives:**

Anyone can trigger a liquidation and receives:
- **Gas compensation**: 50 USDsol (from liquidation reserve)
- **Collateral bonus**: 0.5% of liquidated collateral

### Recovery Mode

Recovery Mode is a **system-wide emergency state** that activates when total collateral ratio falls below 150%.

**What is TCR (Total Collateral Ratio)?**

```
TCR = (Total Collateral Value across all vaults) / (Total Debt)

Example:
  Total SOL collateral: 10,000 SOL @ $200 = $2,000,000
  Total debt: $1,500,000 USDsol
  TCR = $2,000,000 / $1,500,000 = 133% ← Below 150%, Recovery Mode!
```

**What changes in Recovery Mode:**

| Aspect | Normal Mode | Recovery Mode |
|--------|-------------|---------------|
| Liquidation threshold | 110% CR | 150% CR |
| Borrowing fee | 0.5% - 5% | 0% |
| New borrowing | Allowed if CR ≥ 110% | Only if improves TCR |
| Collateral withdrawal | Allowed if CR ≥ 110% | Only if improves TCR |

**Purpose of Recovery Mode:**
- Incentivizes rapid system recapitalization
- Allows liquidation of riskier positions (110-150% CR)
- Zero fees encourage new collateral deposits
- Blocks actions that would worsen system health

**Liquidation in Recovery Mode:**

Vaults with CR between 110-150% can be liquidated, but:
- Liquidation is capped at 110% of debt value
- Remaining collateral above 110% is returned to vault owner
- Same 10% liquidation penalty applies

---

## Tokenomics ($MANNA)

$MANNA is the protocol token that captures fee revenue and provides early-adopter incentives.

### Token Details

| Property | Value |
|----------|-------|
| Name | Manna |
| Symbol | MANNA |
| Max Supply | 100,000,000 |
| Blockchain | Solana |
| Governance | None (except collateral voting) |

### Distribution

```
┌────────────────────────────────────────────────────────────┐
│                   MANNA Distribution                        │
├────────────────────────────────────────────────────────────┤
│                                                            │
│  Community Issuance (Stability Pool)     35%  ████████     │
│  Protocol Treasury / Ecosystem           25%  ██████       │
│  Team & Contributors                     20%  █████        │
│  Investors                               15%  ████         │
│  Liquidity Incentives                     5%  ██           │
│                                                            │
└────────────────────────────────────────────────────────────┘
```

### Community Issuance Schedule

MANNA is emitted to Stability Pool depositors following a **halving schedule**:

```
Year 1: 17,500,000 MANNA
Year 2:  8,750,000 MANNA
Year 3:  4,375,000 MANNA
Year 4:  2,187,500 MANNA
...
Formula: 35,000,000 × (1 - 0.5^year)
```

This front-loads rewards to early adopters while maintaining long-term incentives.

### MANNA Staking

Stakers lock MANNA to earn protocol revenue:

**Revenue sources:**
1. **Borrowing fees**: 0.5-5% of all USDsol minted
2. **Redemption fees**: 0.5-5% of all USDsol redeemed

**Distribution:**
- Fees are collected in USDsol and collateral tokens
- Distributed pro-rata to all stakers instantly
- No lock-up period for staking/unstaking

**Example:**
```
Total MANNA staked: 10,000,000
Your stake: 100,000 (1%)

Daily protocol fees collected:
  Borrowing: 5,000 USDsol
  Redemptions: 2,000 USDsol + 10 SOL

Your daily earnings:
  50 USDsol + 20 USDsol + 0.1 SOL = 70 USDsol + 0.1 SOL
```

### MANNA vs LQTY Comparison

| Aspect | LQTY (Liquity) | MANNA |
|--------|---------------|-------|
| Max supply | 100M | 100M |
| Governance | None | Collateral voting only |
| Staking rewards | LUSD + ETH fees | USDsol + collateral fees |
| Emission | To Stability Pool | To Stability Pool |
| Schedule | Yearly halving | Yearly halving |

---

## Technical Architecture

### Solana Program Design

Manna is built using the **Anchor framework** and follows Solana's account-based programming model.

#### Key Architectural Differences from EVM

| Aspect | Ethereum (Liquity) | Solana (Manna) |
|--------|-------------------|----------------|
| State storage | Contract storage slots | Separate account data |
| Execution | Sequential | Parallel (where possible) |
| Programs | Smart contracts with state | Stateless programs + accounts |
| Upgradeability | Proxy patterns | Upgrade authority |
| Fees | Global gas market | Local per-account fees |

#### Program Structure

```
manna-protocol/
├── programs/
│   ├── manna-core/           # Core lending logic
│   │   ├── vault.rs          # Vault (position) management
│   │   ├── stability-pool.rs # Stability Pool logic
│   │   ├── liquidation.rs    # Liquidation mechanics
│   │   ├── redemption.rs     # Redemption mechanics
│   │   └── fee-manager.rs    # Fee calculation
│   │
│   ├── manna-token/          # MANNA SPL token
│   │   ├── mint.rs           # Token minting (emissions)
│   │   └── staking.rs        # MANNA staking
│   │
│   ├── usdsol-token/         # USDsol SPL token
│   │   └── mint.rs           # USDsol mint/burn
│   │
│   └── collateral-registry/  # Collateral management
│       └── registry.rs       # Add/configure collaterals
│
├── accounts/
│   ├── GlobalState           # Protocol-wide parameters
│   ├── Branch                # Per-collateral branch config
│   ├── Vault                 # Individual user positions
│   ├── StabilityPool         # Per-branch stability pool
│   ├── SortedVaults          # Ordered list for redemptions
│   └── StakingPool           # MANNA staking state
│
└── tests/
```

#### Account Model

**Vault Account:**
```rust
#[account]
pub struct Vault {
    pub owner: Pubkey,           // Vault owner
    pub branch: Pubkey,          // Collateral branch
    pub collateral: u64,         // Collateral amount (lamports)
    pub debt: u64,               // USDsol debt (with decimals)
    pub stake: u64,              // Stability Pool stake
    pub status: VaultStatus,     // Active/Closed/Liquidated
    pub array_index: u64,        // Position in sorted list
    pub bump: u8,                // PDA bump
}
```

**Branch Account:**
```rust
#[account]
pub struct Branch {
    pub collateral_mint: Pubkey,  // SOL, jitoSOL, bNSOL mint
    pub price_feed: Pubkey,       // Pyth price feed account
    pub total_collateral: u64,    // Total collateral in branch
    pub total_debt: u64,          // Total debt in branch
    pub base_rate: u64,           // Current base rate (bps)
    pub last_fee_time: i64,       // Last fee operation timestamp
    pub stability_pool: Pubkey,   // Associated stability pool
    pub mcr: u64,                 // Minimum collateral ratio (bps)
    pub ccr: u64,                 // Critical collateral ratio (bps)
    pub bump: u8,
}
```

#### Sorted Vaults (Redemption Ordering)

For efficient redemptions, vaults are maintained in a sorted doubly-linked list ordered by collateral ratio:

```rust
#[account]
pub struct SortedVaults {
    pub branch: Pubkey,
    pub head: Pubkey,        // Lowest CR (first to redeem)
    pub tail: Pubkey,        // Highest CR
    pub size: u64,
    pub max_size: u64,
}

// Each vault contains:
pub struct VaultNode {
    pub next: Pubkey,        // Higher CR vault
    pub prev: Pubkey,        // Lower CR vault
}
```

This enables O(1) finding of the lowest-CR vault for redemptions.

### Immutability Model

**Core programs (IMMUTABLE):**
- manna-core
- manna-token
- usdsol-token

After deployment, upgrade authority is **revoked** using:
```bash
solana program set-upgrade-authority <PROGRAM_ID> --final
```

This makes the programs permanently immutable — no one, including the deployer, can modify them.

**Collateral Registry (EXPANDABLE):**
- Deployer retains upgrade authority
- Can only add new collateral branches
- Cannot modify existing branches or core logic
- New collaterals require:
  - Valid Pyth price feed
  - Community signaling via MANNA vote
  - Careful parameter selection

### Cross-Program Invocations (CPIs)

```
┌─────────────┐     ┌─────────────┐     ┌─────────────┐
│   User Tx   │────►│ manna-core  │────►│ usdsol-token│
└─────────────┘     └─────────────┘     └─────────────┘
                           │
                           │ CPI
                           ▼
                    ┌─────────────┐
                    │ token-2022  │
                    │ (SPL Token) │
                    └─────────────┘
```

### Program Derived Addresses (PDAs)

Key PDAs in Manna:

| PDA | Seeds | Purpose |
|-----|-------|---------|
| Vault | `[b"vault", owner, branch]` | User's position |
| Branch | `[b"branch", collateral_mint]` | Collateral config |
| StabilityPool | `[b"sp", branch]` | Branch's stability pool |
| GlobalState | `[b"global"]` | Protocol parameters |

---

## Oracle Design

### Primary: Pyth Network

Manna uses **Pyth Network** as the primary oracle for all price feeds.

**Why Pyth:**
- Native Solana integration
- Sub-second price updates
- High-fidelity data from first-party sources
- Wide asset coverage (SOL, LSTs)
- Pull-based model (lower costs)

**Integration:**

```rust
use pyth_solana_receiver_sdk::price_update::PriceUpdateV2;

pub fn get_price(price_update: &Account<PriceUpdateV2>) -> Result<u64> {
    let price = price_update.get_price_no_older_than(
        &Clock::get()?,
        STALENESS_THRESHOLD,  // 60 seconds
        &FEED_ID,
    )?;
    
    // Convert to standard decimals
    let price_u64 = (price.price as u64)
        .checked_mul(10u64.pow(PRICE_DECIMALS))
        .ok_or(MannnaError::MathOverflow)?
        .checked_div(10u64.pow((-price.exponent) as u32))
        .ok_or(MannaError::MathOverflow)?;
    
    Ok(price_u64)
}
```

**Price Feed IDs:**

| Asset | Pyth Feed ID |
|-------|-------------|
| SOL/USD | `0xef0d8b6fda2ceba41da15d4095d1da392a0d2f8ed0c6c7bc0f4cfac8c280b56d` |
| jitoSOL/USD | Derived from SOL + exchange rate |
| bNSOL/USD | Derived from SOL + exchange rate |

### Fallback: Switchboard

If Pyth fails (stale price, invalid response, >50% price jump), Manna falls back to **Switchboard Oracle**:

**Fallback conditions:**
1. Pyth price not updated for >60 seconds
2. Pyth returns invalid price or timestamp
3. Price change >50% between updates (manipulation check)

**Fallback flow:**
```
Pyth price requested
        │
        ▼
   Valid & fresh?
    │         │
   Yes        No
    │         │
    ▼         ▼
  Use Pyth  Check Switchboard
              │
              ▼
         Valid & fresh?
          │         │
         Yes        No
          │         │
          ▼         ▼
    Use Switchboard  PAUSE operations
```

### LST Price Calculation

For liquid staking tokens, the price is derived:

```
jitoSOL_price = SOL_price × jitoSOL_exchange_rate

Where:
  jitoSOL_exchange_rate = jitoSOL_supply / underlying_SOL
  (Fetched from Jito stake pool account)
```

This ensures LST prices accurately reflect their SOL backing plus accrued yield.

---

## Security Considerations

### Immutability Trade-offs

**Pros:**
- No admin keys that can be compromised
- No governance attacks possible
- Users can trust code won't change
- Censorship resistant

**Cons:**
- Bugs cannot be fixed (require migration)
- Parameters cannot be adjusted
- Must get it right from the start

**Mitigation:**
- Extensive audits before deployment
- Comprehensive test coverage
- Formal verification where possible
- Bug bounty program
- Gradual rollout with caps

### Oracle Risks

**Risk:** Oracle manipulation or failure
**Mitigation:**
- Dual oracle design (Pyth + Switchboard)
- Staleness checks (60 second threshold)
- Price deviation checks (>50% = suspicious)
- Pause mechanism on oracle failure

### LST-Specific Risks

**Risk:** LST depegs from underlying SOL
**Mitigation:**
- Collateral isolation (LST issues don't affect SOL branch)
- Conservative collateral ratios for LSTs
- LST exchange rate verification
- Ability to pause specific branches (emergency only)

### Smart Contract Risks

**Risk:** Bugs in lending logic
**Mitigation:**
- Anchor framework (safety guardrails)
- Multiple audits (planned: OtterSec, Neodyme)
- Formal verification of core invariants
- Economic simulations
- Gradual TVL caps during launch

### Economic Risks

**Risk:** Black swan causing mass liquidations
**Mitigation:**
- Recovery Mode triggers at 150% TCR
- Stability Pool absorbs liquidations
- Redistribution as fallback
- 110% minimum CR (10% buffer)

### Audit Plan

| Phase | Auditor | Focus |
|-------|---------|-------|
| Pre-launch | OtterSec | Core logic, math |
| Pre-launch | Neodyme | Solana-specific |
| Post-launch | Ongoing | Bug bounty (Immunefi) |

---

## Comparison to Other Protocols

### vs Liquity V1 (Ethereum)

| Aspect | Liquity V1 | Manna |
|--------|-----------|-------|
| Chain | Ethereum | Solana |
| Collateral | ETH only | SOL + LSTs |
| Stablecoin | LUSD | USDsol |
| Fees | One-time | One-time |
| Min CR | 110% | 110% |
| Governance | None | Collateral voting only |
| Immutable | Yes | Yes |
| Multi-collateral | No | Yes (isolated branches) |

Manna extends Liquity V1's proven model to Solana with multi-collateral support.

### vs Liquity V2 (Ethereum)

| Aspect | Liquity V2 | Manna |
|--------|-----------|-------|
| Interest rates | User-set (variable) | Algorithmic (one-time) |
| Complexity | Higher | Lower |
| Predictability | Variable | Fixed at borrow time |
| Redemption order | By interest rate | By collateral ratio |

Manna chose Liquity V1's simpler fee model for predictability and lower complexity.

### vs MakerDAO (Ethereum)

| Aspect | MakerDAO | Manna |
|--------|----------|-------|
| Governance | MKR holders | None |
| Interest rates | Governance-set | Algorithmic |
| Collateral | Many (including RWA) | SOL + LSTs |
| Min CR | 150%+ (varies) | 110% |
| Immutable | No (upgradeable) | Yes |
| Centralization | Moderate | Minimal |

Manna is more capital efficient and truly decentralized compared to MakerDAO.

### vs Hubble (Solana)

| Aspect | Hubble | Manna |
|--------|--------|-------|
| Stablecoin | USDH | USDsol |
| Collateral | Multiple | SOL + LSTs |
| Liquidations | Keeper-based | Stability Pool |
| Redemptions | Limited | Full (key peg mechanism) |
| Governance | Yes | Minimal |

Manna's stability pool and redemption model provides stronger peg guarantees.

---

## Roadmap

### Phase 1: Foundation (Q2 2026)
- [ ] Core protocol development
- [ ] SOL branch implementation
- [ ] Stability Pool implementation
- [ ] Basic liquidation mechanics
- [ ] Internal testing & simulations

### Phase 2: Audit & Refinement (Q3 2026)
- [ ] OtterSec audit
- [ ] Neodyme audit
- [ ] Bug fixes and optimizations
- [ ] Economic modeling validation
- [ ] Testnet deployment

### Phase 3: Mainnet Launch (Q4 2026)
- [ ] Mainnet deployment (SOL branch only)
- [ ] TVL caps during initial period
- [ ] Bug bounty program launch
- [ ] Community onboarding

### Phase 4: LST Expansion (Q1 2027)
- [ ] jitoSOL branch launch
- [ ] bNSOL branch launch
- [ ] LST-specific parameter tuning

### Phase 5: Ecosystem Growth (2027+)
- [ ] Additional LST support
- [ ] DEX integrations
- [ ] Lending protocol integrations
- [ ] Cross-chain considerations

---

## Resources

### Links (Placeholder)
- Website: TBD
- Documentation: TBD
- GitHub: TBD
- Discord: TBD
- Twitter: TBD

### References

- [Liquity V1 Documentation](https://docs.liquity.org/liquity-v1)
- [Liquity V2 Documentation](https://docs.liquity.org)
- [Pyth Network Documentation](https://docs.pyth.network)
- [Solana Developer Documentation](https://solana.com/docs)
- [Anchor Framework](https://www.anchor-lang.com)

---

## Disclaimer

Manna Protocol is experimental software. Use at your own risk. This documentation describes planned functionality and may change before launch. Smart contracts, even when audited, may contain bugs. Never deposit more than you can afford to lose.

---

*Built with 🥖 on Solana*
