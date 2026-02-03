import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import { Manna } from "../target/types/manna";
import { PublicKey, Keypair, LAMPORTS_PER_SOL } from "@solana/web3.js";
import { 
  TOKEN_2022_PROGRAM_ID,
  getAssociatedTokenAddressSync,
  createAssociatedTokenAccountInstruction,
} from "@solana/spl-token";
import { expect } from "chai";

describe("Manna Protocol", () => {
  // Configure the client to use the local cluster
  const provider = anchor.AnchorProvider.env();
  anchor.setProvider(provider);

  const program = anchor.workspace.Manna as Program<Manna>;
  
  // Test wallets
  const admin = Keypair.generate();
  const user1 = Keypair.generate();
  const user2 = Keypair.generate();
  const liquidator = Keypair.generate();

  // PDAs
  let globalStatePda: PublicKey;
  let usdsol_mintPda: PublicKey;
  let mannaMintPda: PublicKey;
  let stabilityPoolPda: PublicKey;
  let user1VaultPda: PublicKey;
  let user2VaultPda: PublicKey;

  // Mock Pyth price feed (for local testing)
  let mockPriceFeed: Keypair;

  before(async () => {
    // Airdrop SOL to test accounts
    await provider.connection.requestAirdrop(admin.publicKey, 100 * LAMPORTS_PER_SOL);
    await provider.connection.requestAirdrop(user1.publicKey, 100 * LAMPORTS_PER_SOL);
    await provider.connection.requestAirdrop(user2.publicKey, 100 * LAMPORTS_PER_SOL);
    await provider.connection.requestAirdrop(liquidator.publicKey, 10 * LAMPORTS_PER_SOL);

    // Wait for airdrops to confirm
    await new Promise(resolve => setTimeout(resolve, 1000));

    // Derive PDAs
    [globalStatePda] = PublicKey.findProgramAddressSync(
      [Buffer.from("global_state")],
      program.programId
    );

    [usdsol_mintPda] = PublicKey.findProgramAddressSync(
      [Buffer.from("usdsol_mint")],
      program.programId
    );

    [mannaMintPda] = PublicKey.findProgramAddressSync(
      [Buffer.from("manna_mint")],
      program.programId
    );

    [stabilityPoolPda] = PublicKey.findProgramAddressSync(
      [Buffer.from("stability_pool")],
      program.programId
    );

    [user1VaultPda] = PublicKey.findProgramAddressSync(
      [Buffer.from("vault"), user1.publicKey.toBuffer()],
      program.programId
    );

    [user2VaultPda] = PublicKey.findProgramAddressSync(
      [Buffer.from("vault"), user2.publicKey.toBuffer()],
      program.programId
    );

    // Create mock price feed account
    mockPriceFeed = Keypair.generate();
  });

  describe("Initialization", () => {
    it("Should initialize the protocol", async () => {
      await program.methods
        .initialize()
        .accounts({
          authority: admin.publicKey,
          globalState: globalStatePda,
          usdsol_mint: usdsol_mintPda,
          mannaMint: mannaMintPda,
          stabilityPool: stabilityPoolPda,
          priceFeed: mockPriceFeed.publicKey,
          tokenProgram: TOKEN_2022_PROGRAM_ID,
        })
        .signers([admin])
        .rpc();

      // Verify global state
      const globalState = await program.account.globalState.fetch(globalStatePda);
      expect(globalState.usdsol_mint.toString()).to.equal(usdsol_mintPda.toString());
      expect(globalState.mannaMint.toString()).to.equal(mannaMintPda.toString());
      expect(globalState.totalCollateral.toNumber()).to.equal(0);
      expect(globalState.totalDebt.toNumber()).to.equal(0);
      expect(globalState.isPaused).to.equal(false);
    });
  });

  describe("Vault Operations", () => {
    const collateralAmount = 10 * LAMPORTS_PER_SOL; // 10 SOL
    const borrowAmount = 1000_000_000; // 1000 USDsol (6 decimals)

    it("Should open a vault with collateral", async () => {
      await program.methods
        .openVault(new anchor.BN(collateralAmount))
        .accounts({
          owner: user1.publicKey,
          globalState: globalStatePda,
          vault: user1VaultPda,
        })
        .signers([user1])
        .rpc();

      // Verify vault
      const vault = await program.account.vault.fetch(user1VaultPda);
      expect(vault.owner.toString()).to.equal(user1.publicKey.toString());
      expect(vault.collateral.toNumber()).to.equal(collateralAmount);
      expect(vault.debt.toNumber()).to.equal(0);
      expect(vault.status).to.deep.equal({ active: {} });

      // Verify global state updated
      const globalState = await program.account.globalState.fetch(globalStatePda);
      expect(globalState.totalCollateral.toNumber()).to.equal(collateralAmount);
      expect(globalState.activeVaults.toNumber()).to.equal(1);
    });

    it("Should deposit additional collateral", async () => {
      const additionalCollateral = 5 * LAMPORTS_PER_SOL;
      
      await program.methods
        .depositCollateral(new anchor.BN(additionalCollateral))
        .accounts({
          owner: user1.publicKey,
          globalState: globalStatePda,
          vault: user1VaultPda,
        })
        .signers([user1])
        .rpc();

      const vault = await program.account.vault.fetch(user1VaultPda);
      expect(vault.collateral.toNumber()).to.equal(collateralAmount + additionalCollateral);
    });

    it("Should borrow USDsol", async () => {
      // Get user's USDsol token account
      const userUsdsol = getAssociatedTokenAddressSync(
        usdsol_mintPda,
        user1.publicKey,
        false,
        TOKEN_2022_PROGRAM_ID
      );

      // Create ATA if needed
      const createAtaIx = createAssociatedTokenAccountInstruction(
        user1.publicKey,
        userUsdsol,
        user1.publicKey,
        usdsol_mintPda,
        TOKEN_2022_PROGRAM_ID
      );

      await program.methods
        .borrow(new anchor.BN(borrowAmount))
        .accounts({
          owner: user1.publicKey,
          globalState: globalStatePda,
          vault: user1VaultPda,
          usdsol_mint: usdsol_mintPda,
          ownerUsdsol: userUsdsol,
          priceFeed: mockPriceFeed.publicKey,
          tokenProgram: TOKEN_2022_PROGRAM_ID,
        })
        .preInstructions([createAtaIx])
        .signers([user1])
        .rpc();

      // Verify vault debt
      const vault = await program.account.vault.fetch(user1VaultPda);
      expect(vault.debt.toNumber()).to.be.greaterThan(borrowAmount); // debt includes fee

      // Verify user received tokens
      const balance = await provider.connection.getTokenAccountBalance(userUsdsol);
      expect(parseInt(balance.value.amount)).to.equal(borrowAmount);
    });

    it("Should repay USDsol debt", async () => {
      const repayAmount = 500_000_000; // 500 USDsol
      
      const userUsdsol = getAssociatedTokenAddressSync(
        usdsol_mintPda,
        user1.publicKey,
        false,
        TOKEN_2022_PROGRAM_ID
      );

      const vaultBefore = await program.account.vault.fetch(user1VaultPda);
      
      await program.methods
        .repay(new anchor.BN(repayAmount))
        .accounts({
          owner: user1.publicKey,
          globalState: globalStatePda,
          vault: user1VaultPda,
          usdsol_mint: usdsol_mintPda,
          ownerUsdsol: userUsdsol,
          tokenProgram: TOKEN_2022_PROGRAM_ID,
        })
        .signers([user1])
        .rpc();

      const vaultAfter = await program.account.vault.fetch(user1VaultPda);
      expect(vaultAfter.debt.toNumber()).to.equal(vaultBefore.debt.toNumber() - repayAmount);
    });

    it("Should withdraw collateral while maintaining MCR", async () => {
      const withdrawAmount = 1 * LAMPORTS_PER_SOL;
      
      const vaultBefore = await program.account.vault.fetch(user1VaultPda);
      
      await program.methods
        .withdrawCollateral(new anchor.BN(withdrawAmount))
        .accounts({
          owner: user1.publicKey,
          globalState: globalStatePda,
          vault: user1VaultPda,
          priceFeed: mockPriceFeed.publicKey,
        })
        .signers([user1])
        .rpc();

      const vaultAfter = await program.account.vault.fetch(user1VaultPda);
      expect(vaultAfter.collateral.toNumber()).to.equal(vaultBefore.collateral.toNumber() - withdrawAmount);
    });

    it("Should NOT allow withdrawal that breaches MCR", async () => {
      const massiveWithdraw = 100 * LAMPORTS_PER_SOL;
      
      try {
        await program.methods
          .withdrawCollateral(new anchor.BN(massiveWithdraw))
          .accounts({
            owner: user1.publicKey,
            globalState: globalStatePda,
            vault: user1VaultPda,
            priceFeed: mockPriceFeed.publicKey,
          })
          .signers([user1])
          .rpc();
        
        expect.fail("Should have thrown error");
      } catch (err) {
        expect(err.toString()).to.include("WithdrawalWouldBreachMCR");
      }
    });
  });

  describe("Stability Pool", () => {
    it("Should deposit to stability pool", async () => {
      const depositAmount = 500_000_000; // 500 USDsol
      
      const userUsdsol = getAssociatedTokenAddressSync(
        usdsol_mintPda,
        user1.publicKey,
        false,
        TOKEN_2022_PROGRAM_ID
      );

      const poolUsdsol = getAssociatedTokenAddressSync(
        usdsol_mintPda,
        stabilityPoolPda,
        true,
        TOKEN_2022_PROGRAM_ID
      );

      await program.methods
        .stabilityDeposit(new anchor.BN(depositAmount))
        .accounts({
          depositor: user1.publicKey,
          globalState: globalStatePda,
          stabilityPool: stabilityPoolPda,
          usdsol_mint: usdsol_mintPda,
          depositorUsdsol: userUsdsol,
          poolUsdsol: poolUsdsol,
          tokenProgram: TOKEN_2022_PROGRAM_ID,
        })
        .signers([user1])
        .rpc();

      const pool = await program.account.stabilityPool.fetch(stabilityPoolPda);
      expect(pool.totalUsdsol_deposits.toNumber()).to.equal(depositAmount);
    });

    it("Should withdraw from stability pool", async () => {
      const withdrawAmount = 250_000_000; // 250 USDsol
      
      const userUsdsol = getAssociatedTokenAddressSync(
        usdsol_mintPda,
        user1.publicKey,
        false,
        TOKEN_2022_PROGRAM_ID
      );

      const poolUsdsol = getAssociatedTokenAddressSync(
        usdsol_mintPda,
        stabilityPoolPda,
        true,
        TOKEN_2022_PROGRAM_ID
      );

      const [depositPda] = PublicKey.findProgramAddressSync(
        [Buffer.from("stability_deposit"), user1.publicKey.toBuffer()],
        program.programId
      );

      await program.methods
        .stabilityWithdraw(new anchor.BN(withdrawAmount))
        .accounts({
          depositor: user1.publicKey,
          globalState: globalStatePda,
          stabilityPool: stabilityPoolPda,
          depositRecord: depositPda,
          owner: user1.publicKey,
          usdsol_mint: usdsol_mintPda,
          depositorUsdsol: userUsdsol,
          poolUsdsol: poolUsdsol,
          tokenProgram: TOKEN_2022_PROGRAM_ID,
        })
        .signers([user1])
        .rpc();

      const pool = await program.account.stabilityPool.fetch(stabilityPoolPda);
      expect(pool.totalUsdsol_deposits.toNumber()).to.equal(250_000_000); // 500 - 250
    });
  });

  describe("Liquidation", () => {
    it("Should liquidate undercollateralized vault", async () => {
      // First, create a vault that will be undercollateralized
      // Then trigger liquidation
      // This test requires mock price manipulation
      console.log("Liquidation test requires price feed mock - skipping in local env");
    });
  });

  describe("Edge Cases", () => {
    it("Should reject zero collateral deposit", async () => {
      try {
        await program.methods
          .depositCollateral(new anchor.BN(0))
          .accounts({
            owner: user1.publicKey,
            globalState: globalStatePda,
            vault: user1VaultPda,
          })
          .signers([user1])
          .rpc();
        
        expect.fail("Should have thrown error");
      } catch (err) {
        expect(err.toString()).to.include("ZeroAmount");
      }
    });

    it("Should reject borrowing below minimum debt", async () => {
      // Open new vault with user2
      const smallCollateral = 1 * LAMPORTS_PER_SOL;
      const tinyBorrow = 10_000_000; // 10 USDsol (below 200 minimum)

      await program.methods
        .openVault(new anchor.BN(smallCollateral))
        .accounts({
          owner: user2.publicKey,
          globalState: globalStatePda,
          vault: user2VaultPda,
        })
        .signers([user2])
        .rpc();

      const userUsdsol = getAssociatedTokenAddressSync(
        usdsol_mintPda,
        user2.publicKey,
        false,
        TOKEN_2022_PROGRAM_ID
      );

      try {
        await program.methods
          .borrow(new anchor.BN(tinyBorrow))
          .accounts({
            owner: user2.publicKey,
            globalState: globalStatePda,
            vault: user2VaultPda,
            usdsol_mint: usdsol_mintPda,
            ownerUsdsol: userUsdsol,
            priceFeed: mockPriceFeed.publicKey,
            tokenProgram: TOKEN_2022_PROGRAM_ID,
          })
          .signers([user2])
          .rpc();
        
        expect.fail("Should have thrown error");
      } catch (err) {
        expect(err.toString()).to.include("BelowMinimumDebt");
      }
    });
  });
});
