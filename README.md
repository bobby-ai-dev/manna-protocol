# Manna Protocol

> **Decentralized Borrowing on Solana** — Mint USDsol against SOL collateral with algorithmic rates and battle-tested mechanics from Liquity.

![Manna Protocol](https://img.shields.io/badge/Solana-Hackathon-purple)
![Status](https://img.shields.io/badge/Status-Building-yellow)

## 🎯 What is Manna?

Manna is a decentralized borrowing protocol that lets users mint **USDsol** (a USD-pegged stablecoin) by depositing SOL as collateral. It adapts the battle-tested Liquity V1 mechanisms for Solana:

- **110% minimum collateral ratio** — Capital efficient borrowing
- **One-time borrowing fee** — No ongoing interest, predictable costs
- **Stability Pool** — First-line liquidation defense, earns rewards
- **Redemptions** — Hard peg mechanism ensuring USDsol ≈ $1
- **Immutable** — No governance, no admin keys, pure code

## 🚀 Features

| Feature | Description |
|---------|-------------|
| **Open Vault** | Deposit SOL, mint USDsol at 110%+ collateral ratio |
| **Stability Pool** | Deposit USDsol, earn liquidation gains + MANNA rewards |
| **Liquidations** | Automatic liquidation of undercollateralized vaults |
| **Redemptions** | Swap USDsol for $1 of SOL anytime |
| **No Governance** | Algorithmic parameters, immutable contracts |

## 📐 Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                      MANNA PROTOCOL                         │
├─────────────────────────────────────────────────────────────┤
│                                                             │
│   ┌──────────┐    ┌──────────────┐    ┌──────────────┐     │
│   │  Vaults  │    │  Stability   │    │  Redemption  │     │
│   │ (Borrow) │    │    Pool      │    │    Engine    │     │
│   └────┬─────┘    └──────┬───────┘    └──────┬───────┘     │
│        │                 │                   │              │
│        └────────────┬────┴───────────────────┘              │
│                     │                                       │
│              ┌──────▼──────┐                                │
│              │   USDsol    │                                │
│              │  (Mint)     │                                │
│              └─────────────┘                                │
│                                                             │
│   Oracle: Pyth SOL/USD    Token: MANNA (rewards)           │
└─────────────────────────────────────────────────────────────┘
```

## 🛠️ Technical Stack

- **Framework**: Anchor (Solana smart contract framework)
- **Language**: Rust (on-chain) + TypeScript (SDK)
- **Oracle**: Pyth Network for SOL/USD prices
- **Token Standard**: SPL Token-2022

## 📦 Project Structure

```
manna-hackathon/
├── programs/
│   └── manna/
│       └── src/
│           ├── lib.rs           # Program entry point
│           ├── state/           # Account structures
│           ├── instructions/    # Instruction handlers
│           ├── errors.rs        # Error definitions
│           └── constants.rs     # Protocol constants
├── sdk/
│   └── src/
│       ├── index.ts            # SDK entry point
│       ├── instructions.ts     # Instruction builders
│       └── accounts.ts         # Account helpers
├── tests/
│   └── manna.ts               # Integration tests
└── app/                       # Frontend (coming soon)
```

## 🏃 Quick Start

```bash
# Install dependencies
npm install

# Build the program
anchor build

# Run tests
anchor test

# Deploy to devnet
anchor deploy --provider.cluster devnet
```

## 📊 Protocol Parameters

| Parameter | Value | Description |
|-----------|-------|-------------|
| Min Collateral Ratio | 110% | Minimum CR to avoid liquidation |
| Critical CR | 150% | Recovery Mode trigger |
| Min Debt | 200 USDsol | Minimum borrowing amount |
| Liquidation Reserve | 50 USDsol | Gas compensation for liquidators |
| Borrowing Fee | 0.5% - 5% | One-time fee based on system state |
| Redemption Fee | 0.5% - 5% | Fee for redeeming USDsol |

## 🔗 Links

- **Hackathon**: [Colosseum Agent Hackathon](https://colosseum.com/agent-hackathon)
- **Docs**: Coming soon
- **Discord**: Coming soon

## 🏆 Built for Colosseum Agent Hackathon

This project was built by Bobby (an AI agent) for the Colosseum Agent Hackathon, February 2026.

**Prize wallet**: `AnHxNt3622PsdqihhrGStBg8Kvfc78ssHCGV3bT9zGK2`

---

*Built with 🥖 on Solana*
