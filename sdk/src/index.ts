/**
 * Manna Protocol SDK
 * 
 * A TypeScript SDK for interacting with the Manna Protocol on Solana.
 * Manna is a decentralized borrowing protocol that lets you mint USDsol
 * against SOL collateral.
 * 
 * @example
 * ```typescript
 * import { MannaSDK } from '@manna/sdk';
 * import { Connection, Keypair } from '@solana/web3.js';
 * 
 * const connection = new Connection('https://api.devnet.solana.com');
 * const wallet = Keypair.generate();
 * const manna = new MannaSDK(connection, wallet);
 * 
 * // Open a vault with 1 SOL
 * await manna.openVault(1_000_000_000);
 * 
 * // Borrow 100 USDsol
 * await manna.borrow(100_000_000);
 * ```
 */

import { 
  Connection, 
  PublicKey, 
  Keypair, 
  Transaction,
  TransactionInstruction,
  SystemProgram,
  LAMPORTS_PER_SOL,
} from '@solana/web3.js';
import { 
  TOKEN_2022_PROGRAM_ID,
  getAssociatedTokenAddressSync,
  createAssociatedTokenAccountInstruction,
} from '@solana/spl-token';
import * as anchor from '@coral-xyz/anchor';

// Program ID (deployed to devnet)
export const PROGRAM_ID = new PublicKey('6UjE17wtzEmaAhMaTVDSbrhwkqPKHcGBs7YzVgB46Sx9');

// Seeds
export const GLOBAL_STATE_SEED = Buffer.from('global_state');
export const VAULT_SEED = Buffer.from('vault');
export const STABILITY_POOL_SEED = Buffer.from('stability_pool');
export const USDSOL_MINT_SEED = Buffer.from('usdsol_mint');
export const MANNA_MINT_SEED = Buffer.from('manna_mint');

// Protocol constants
export const MCR = 1.1; // 110% minimum collateral ratio
export const CCR = 1.5; // 150% critical collateral ratio (recovery mode)
export const MIN_DEBT = 200_000_000; // 200 USDsol minimum debt
export const LIQUIDATION_RESERVE = 50_000_000; // 50 USDsol

export interface VaultInfo {
  owner: PublicKey;
  collateral: bigint; // lamports
  debt: bigint; // USDsol (6 decimals)
  liquidationReserve: bigint;
  status: 'inactive' | 'active' | 'closedByOwner' | 'liquidated';
  collateralRatio: number;
  openedAt: Date;
  lastUpdated: Date;
}

export interface GlobalState {
  usdsol_mint: PublicKey;
  manna_mint: PublicKey;
  totalCollateral: bigint;
  totalDebt: bigint;
  baseRate: number;
  tcr: number; // Total Collateral Ratio
  isRecoveryMode: boolean;
  activeVaults: number;
}

export interface StabilityPoolInfo {
  totalDeposits: bigint;
  totalCollateralGains: bigint;
  currentEpoch: number;
}

export class MannaSDK {
  private connection: Connection;
  private wallet: Keypair;
  private programId: PublicKey;

  constructor(
    connection: Connection,
    wallet: Keypair,
    programId: PublicKey = PROGRAM_ID
  ) {
    this.connection = connection;
    this.wallet = wallet;
    this.programId = programId;
  }

  // ============ PDA Derivation ============

  /**
   * Get the global state PDA
   */
  getGlobalStatePDA(): [PublicKey, number] {
    return PublicKey.findProgramAddressSync(
      [GLOBAL_STATE_SEED],
      this.programId
    );
  }

  /**
   * Get the USDsol mint PDA
   */
  getUSDsolMintPDA(): [PublicKey, number] {
    return PublicKey.findProgramAddressSync(
      [USDSOL_MINT_SEED],
      this.programId
    );
  }

  /**
   * Get the MANNA mint PDA
   */
  getMannaMintPDA(): [PublicKey, number] {
    return PublicKey.findProgramAddressSync(
      [MANNA_MINT_SEED],
      this.programId
    );
  }

  /**
   * Get the stability pool PDA
   */
  getStabilityPoolPDA(): [PublicKey, number] {
    return PublicKey.findProgramAddressSync(
      [STABILITY_POOL_SEED],
      this.programId
    );
  }

  /**
   * Get a vault PDA for a given owner
   */
  getVaultPDA(owner: PublicKey): [PublicKey, number] {
    return PublicKey.findProgramAddressSync(
      [VAULT_SEED, owner.toBuffer()],
      this.programId
    );
  }

  /**
   * Get stability deposit PDA for a given depositor
   */
  getStabilityDepositPDA(depositor: PublicKey): [PublicKey, number] {
    return PublicKey.findProgramAddressSync(
      [Buffer.from('stability_deposit'), depositor.toBuffer()],
      this.programId
    );
  }

  // ============ Read Operations ============

  /**
   * Get global protocol state
   */
  async getGlobalState(): Promise<GlobalState | null> {
    const [pda] = this.getGlobalStatePDA();
    const account = await this.connection.getAccountInfo(pda);
    
    if (!account) return null;
    
    // TODO: Deserialize using Anchor IDL
    // For now, return placeholder
    return {
      usdsol_mint: this.getUSDsolMintPDA()[0],
      manna_mint: this.getMannaMintPDA()[0],
      totalCollateral: BigInt(0),
      totalDebt: BigInt(0),
      baseRate: 0.005,
      tcr: 0,
      isRecoveryMode: false,
      activeVaults: 0,
    };
  }

  /**
   * Get vault info for a user
   */
  async getVault(owner: PublicKey): Promise<VaultInfo | null> {
    const [pda] = this.getVaultPDA(owner);
    const account = await this.connection.getAccountInfo(pda);
    
    if (!account) return null;
    
    // TODO: Deserialize using Anchor IDL
    return null;
  }

  /**
   * Get stability pool info
   */
  async getStabilityPool(): Promise<StabilityPoolInfo | null> {
    const [pda] = this.getStabilityPoolPDA();
    const account = await this.connection.getAccountInfo(pda);
    
    if (!account) return null;
    
    // TODO: Deserialize
    return null;
  }

  /**
   * Calculate collateral ratio
   */
  calculateCR(collateralLamports: bigint, debtUsdsol: bigint, solPriceUSD: number): number {
    if (debtUsdsol === BigInt(0)) return Infinity;
    
    const collateralUSD = (Number(collateralLamports) / LAMPORTS_PER_SOL) * solPriceUSD;
    const debtUSD = Number(debtUsdsol) / 1_000_000; // 6 decimals
    
    return collateralUSD / debtUSD;
  }

  /**
   * Calculate max borrowable amount
   */
  calculateMaxBorrow(collateralLamports: bigint, solPriceUSD: number, existingDebt: bigint = BigInt(0)): bigint {
    const collateralUSD = (Number(collateralLamports) / LAMPORTS_PER_SOL) * solPriceUSD;
    const maxDebtUSD = collateralUSD / MCR;
    const maxDebtUsdsol = BigInt(Math.floor(maxDebtUSD * 1_000_000));
    
    return maxDebtUsdsol - existingDebt;
  }

  /**
   * Calculate borrowing fee
   */
  calculateBorrowingFee(amount: bigint, baseRate: number = 0.005): bigint {
    const feeRate = Math.max(0.005, Math.min(0.05, baseRate + 0.005)); // 0.5% to 5%
    return BigInt(Math.floor(Number(amount) * feeRate));
  }

  // ============ Write Operations ============

  /**
   * Open a new vault with initial collateral
   */
  async openVault(collateralLamports: number): Promise<string> {
    const [vaultPda] = this.getVaultPDA(this.wallet.publicKey);
    const [globalState] = this.getGlobalStatePDA();
    
    // TODO: Build instruction using Anchor
    const instruction = new TransactionInstruction({
      programId: this.programId,
      keys: [
        { pubkey: this.wallet.publicKey, isSigner: true, isWritable: true },
        { pubkey: globalState, isSigner: false, isWritable: true },
        { pubkey: vaultPda, isSigner: false, isWritable: true },
        { pubkey: SystemProgram.programId, isSigner: false, isWritable: false },
      ],
      data: Buffer.from([/* instruction discriminator + collateralLamports */]),
    });
    
    const tx = new Transaction().add(instruction);
    const signature = await this.connection.sendTransaction(tx, [this.wallet]);
    
    return signature;
  }

  /**
   * Deposit additional collateral
   */
  async depositCollateral(amountLamports: number): Promise<string> {
    const [vaultPda] = this.getVaultPDA(this.wallet.publicKey);
    const [globalState] = this.getGlobalStatePDA();
    
    // TODO: Build instruction
    throw new Error('Not implemented - use Anchor client');
  }

  /**
   * Borrow USDsol against collateral
   */
  async borrow(amountUsdsol: number): Promise<string> {
    // TODO: Build instruction
    throw new Error('Not implemented - use Anchor client');
  }

  /**
   * Repay USDsol debt
   */
  async repay(amountUsdsol: number): Promise<string> {
    // TODO: Build instruction
    throw new Error('Not implemented - use Anchor client');
  }

  /**
   * Withdraw collateral
   */
  async withdrawCollateral(amountLamports: number): Promise<string> {
    // TODO: Build instruction
    throw new Error('Not implemented - use Anchor client');
  }

  /**
   * Close vault (requires zero debt)
   */
  async closeVault(): Promise<string> {
    // TODO: Build instruction
    throw new Error('Not implemented - use Anchor client');
  }

  /**
   * Deposit USDsol to stability pool
   */
  async stabilityDeposit(amountUsdsol: number): Promise<string> {
    // TODO: Build instruction
    throw new Error('Not implemented - use Anchor client');
  }

  /**
   * Withdraw from stability pool and claim rewards
   */
  async stabilityWithdraw(amountUsdsol: number): Promise<string> {
    // TODO: Build instruction
    throw new Error('Not implemented - use Anchor client');
  }

  /**
   * Liquidate an undercollateralized vault
   */
  async liquidate(vaultOwner: PublicKey): Promise<string> {
    // TODO: Build instruction
    throw new Error('Not implemented - use Anchor client');
  }
}

// Export types
export type { Connection, PublicKey, Keypair } from '@solana/web3.js';
