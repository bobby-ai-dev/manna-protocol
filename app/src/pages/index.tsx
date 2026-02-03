import { useState, useEffect } from 'react';
import { useConnection, useWallet } from '@solana/wallet-adapter-react';
import { WalletMultiButton } from '@solana/wallet-adapter-react-ui';
import { PublicKey, LAMPORTS_PER_SOL } from '@solana/web3.js';

// Program ID (deployed to devnet)
const PROGRAM_ID = new PublicKey('6UjE17wtzEmaAhMaTVDSbrhwkqPKHcGBs7YzVgB46Sx9');

// Seeds
const GLOBAL_STATE_SEED = Buffer.from('global_state');
const VAULT_SEED = Buffer.from('vault');

export default function Home() {
  const { connection } = useConnection();
  const { publicKey, connected } = useWallet();
  
  const [solBalance, setSolBalance] = useState<number>(0);
  const [vaultData, setVaultData] = useState<any>(null);
  const [globalData, setGlobalData] = useState<any>(null);
  
  // Form state
  const [depositAmount, setDepositAmount] = useState('');
  const [borrowAmount, setBorrowAmount] = useState('');
  const [repayAmount, setRepayAmount] = useState('');
  const [withdrawAmount, setWithdrawAmount] = useState('');

  // Load user data
  useEffect(() => {
    if (connected && publicKey) {
      loadUserData();
    }
  }, [connected, publicKey]);

  const loadUserData = async () => {
    if (!publicKey) return;
    
    try {
      // Get SOL balance
      const balance = await connection.getBalance(publicKey);
      setSolBalance(balance / LAMPORTS_PER_SOL);
      
      // Get vault PDA
      const [vaultPda] = PublicKey.findProgramAddressSync(
        [VAULT_SEED, publicKey.toBuffer()],
        PROGRAM_ID
      );
      
      // Try to fetch vault data (will fail if no vault exists)
      try {
        const vaultAccount = await connection.getAccountInfo(vaultPda);
        if (vaultAccount) {
          // Parse vault data (simplified)
          setVaultData({
            exists: true,
            collateral: 0, // Would parse from account data
            debt: 0,
          });
        }
      } catch (e) {
        setVaultData({ exists: false });
      }
      
      // Get global state
      const [globalPda] = PublicKey.findProgramAddressSync(
        [GLOBAL_STATE_SEED],
        PROGRAM_ID
      );
      
      try {
        const globalAccount = await connection.getAccountInfo(globalPda);
        if (globalAccount) {
          setGlobalData({
            initialized: true,
            totalCollateral: 0,
            totalDebt: 0,
          });
        }
      } catch (e) {
        setGlobalData({ initialized: false });
      }
    } catch (error) {
      console.error('Error loading data:', error);
    }
  };

  // Calculate CR
  const calculateCR = (collateral: number, debt: number, price: number = 200) => {
    if (debt === 0) return Infinity;
    return ((collateral * price) / debt) * 100;
  };

  return (
    <div className="min-h-screen bg-gradient-to-br from-purple-900 via-blue-900 to-black text-white">
      {/* Header */}
      <header className="p-6 flex justify-between items-center">
        <div className="flex items-center gap-3">
          <span className="text-3xl">🥖</span>
          <h1 className="text-2xl font-bold">Manna Protocol</h1>
          <span className="text-sm text-purple-300 bg-purple-900/50 px-2 py-1 rounded">
            Devnet
          </span>
        </div>
        <WalletMultiButton />
      </header>

      {/* Main Content */}
      <main className="max-w-6xl mx-auto p-6">
        {!connected ? (
          // Not Connected State
          <div className="text-center py-20">
            <h2 className="text-4xl font-bold mb-4">
              Decentralized Borrowing on Solana
            </h2>
            <p className="text-xl text-gray-300 mb-8 max-w-2xl mx-auto">
              Mint USDsol stablecoin against SOL collateral at 110% minimum ratio.
              No ongoing interest — just a one-time fee.
            </p>
            <div className="flex justify-center gap-4">
              <WalletMultiButton />
            </div>
            
            {/* Stats */}
            <div className="mt-16 grid grid-cols-3 gap-8 max-w-3xl mx-auto">
              <div className="bg-white/5 rounded-xl p-6">
                <p className="text-3xl font-bold text-green-400">110%</p>
                <p className="text-gray-400">Min Collateral Ratio</p>
              </div>
              <div className="bg-white/5 rounded-xl p-6">
                <p className="text-3xl font-bold text-blue-400">0.5%</p>
                <p className="text-gray-400">Min Borrowing Fee</p>
              </div>
              <div className="bg-white/5 rounded-xl p-6">
                <p className="text-3xl font-bold text-purple-400">$0</p>
                <p className="text-gray-400">Ongoing Interest</p>
              </div>
            </div>
          </div>
        ) : (
          // Connected State
          <div className="grid grid-cols-1 lg:grid-cols-2 gap-6">
            {/* Wallet Overview */}
            <div className="bg-white/5 rounded-xl p-6">
              <h3 className="text-xl font-semibold mb-4">Your Wallet</h3>
              <div className="space-y-3">
                <div className="flex justify-between">
                  <span className="text-gray-400">SOL Balance</span>
                  <span className="font-mono">{solBalance.toFixed(4)} SOL</span>
                </div>
                <div className="flex justify-between">
                  <span className="text-gray-400">USDsol Balance</span>
                  <span className="font-mono">0.00 USDsol</span>
                </div>
                <div className="flex justify-between">
                  <span className="text-gray-400">MANNA Balance</span>
                  <span className="font-mono">0.00 MANNA</span>
                </div>
              </div>
            </div>

            {/* Your Vault */}
            <div className="bg-white/5 rounded-xl p-6">
              <h3 className="text-xl font-semibold mb-4">Your Vault</h3>
              {vaultData?.exists ? (
                <div className="space-y-3">
                  <div className="flex justify-between">
                    <span className="text-gray-400">Collateral</span>
                    <span className="font-mono">{vaultData.collateral} SOL</span>
                  </div>
                  <div className="flex justify-between">
                    <span className="text-gray-400">Debt</span>
                    <span className="font-mono">{vaultData.debt} USDsol</span>
                  </div>
                  <div className="flex justify-between">
                    <span className="text-gray-400">Collateral Ratio</span>
                    <span className="font-mono text-green-400">
                      {calculateCR(vaultData.collateral, vaultData.debt).toFixed(1)}%
                    </span>
                  </div>
                </div>
              ) : (
                <p className="text-gray-400">No vault yet. Open one below!</p>
              )}
            </div>

            {/* Open Vault / Deposit */}
            <div className="bg-white/5 rounded-xl p-6">
              <h3 className="text-xl font-semibold mb-4">
                {vaultData?.exists ? 'Deposit Collateral' : 'Open Vault'}
              </h3>
              <div className="space-y-4">
                <div>
                  <label className="block text-sm text-gray-400 mb-2">
                    SOL Amount
                  </label>
                  <div className="flex gap-2">
                    <input
                      type="number"
                      value={depositAmount}
                      onChange={(e) => setDepositAmount(e.target.value)}
                      placeholder="0.0"
                      className="flex-1 bg-white/10 rounded-lg px-4 py-3 outline-none focus:ring-2 ring-purple-500"
                    />
                    <button className="bg-purple-600 hover:bg-purple-700 px-6 py-3 rounded-lg font-semibold transition">
                      {vaultData?.exists ? 'Deposit' : 'Open Vault'}
                    </button>
                  </div>
                </div>
              </div>
            </div>

            {/* Borrow */}
            <div className="bg-white/5 rounded-xl p-6">
              <h3 className="text-xl font-semibold mb-4">Borrow USDsol</h3>
              <div className="space-y-4">
                <div>
                  <label className="block text-sm text-gray-400 mb-2">
                    USDsol Amount
                  </label>
                  <div className="flex gap-2">
                    <input
                      type="number"
                      value={borrowAmount}
                      onChange={(e) => setBorrowAmount(e.target.value)}
                      placeholder="0.0"
                      className="flex-1 bg-white/10 rounded-lg px-4 py-3 outline-none focus:ring-2 ring-purple-500"
                    />
                    <button 
                      className="bg-blue-600 hover:bg-blue-700 px-6 py-3 rounded-lg font-semibold transition disabled:opacity-50"
                      disabled={!vaultData?.exists}
                    >
                      Borrow
                    </button>
                  </div>
                </div>
                <p className="text-sm text-gray-400">
                  Min CR: 110% • Fee: ~0.5%
                </p>
              </div>
            </div>

            {/* Repay */}
            <div className="bg-white/5 rounded-xl p-6">
              <h3 className="text-xl font-semibold mb-4">Repay Debt</h3>
              <div className="space-y-4">
                <div>
                  <label className="block text-sm text-gray-400 mb-2">
                    USDsol Amount
                  </label>
                  <div className="flex gap-2">
                    <input
                      type="number"
                      value={repayAmount}
                      onChange={(e) => setRepayAmount(e.target.value)}
                      placeholder="0.0"
                      className="flex-1 bg-white/10 rounded-lg px-4 py-3 outline-none focus:ring-2 ring-purple-500"
                    />
                    <button 
                      className="bg-green-600 hover:bg-green-700 px-6 py-3 rounded-lg font-semibold transition disabled:opacity-50"
                      disabled={!vaultData?.exists || vaultData?.debt === 0}
                    >
                      Repay
                    </button>
                  </div>
                </div>
              </div>
            </div>

            {/* Withdraw */}
            <div className="bg-white/5 rounded-xl p-6">
              <h3 className="text-xl font-semibold mb-4">Withdraw Collateral</h3>
              <div className="space-y-4">
                <div>
                  <label className="block text-sm text-gray-400 mb-2">
                    SOL Amount
                  </label>
                  <div className="flex gap-2">
                    <input
                      type="number"
                      value={withdrawAmount}
                      onChange={(e) => setWithdrawAmount(e.target.value)}
                      placeholder="0.0"
                      className="flex-1 bg-white/10 rounded-lg px-4 py-3 outline-none focus:ring-2 ring-purple-500"
                    />
                    <button 
                      className="bg-orange-600 hover:bg-orange-700 px-6 py-3 rounded-lg font-semibold transition disabled:opacity-50"
                      disabled={!vaultData?.exists}
                    >
                      Withdraw
                    </button>
                  </div>
                </div>
                <p className="text-sm text-gray-400">
                  Must maintain 110% CR after withdrawal
                </p>
              </div>
            </div>

            {/* Stability Pool */}
            <div className="lg:col-span-2 bg-gradient-to-r from-purple-900/50 to-blue-900/50 rounded-xl p-6">
              <h3 className="text-xl font-semibold mb-4">🏦 Stability Pool</h3>
              <p className="text-gray-300 mb-4">
                Deposit USDsol to earn liquidation gains and MANNA rewards.
                When vaults are liquidated, your USDsol is exchanged for SOL at ~10% discount.
              </p>
              <div className="grid grid-cols-3 gap-4 mb-6">
                <div className="bg-white/5 rounded-lg p-4">
                  <p className="text-2xl font-bold text-green-400">~12%</p>
                  <p className="text-sm text-gray-400">Est. APY</p>
                </div>
                <div className="bg-white/5 rounded-lg p-4">
                  <p className="text-2xl font-bold">0</p>
                  <p className="text-sm text-gray-400">Total Deposits</p>
                </div>
                <div className="bg-white/5 rounded-lg p-4">
                  <p className="text-2xl font-bold">0</p>
                  <p className="text-sm text-gray-400">Your Deposit</p>
                </div>
              </div>
              <div className="flex gap-4">
                <button className="flex-1 bg-purple-600 hover:bg-purple-700 py-3 rounded-lg font-semibold transition">
                  Deposit to Pool
                </button>
                <button className="flex-1 bg-white/10 hover:bg-white/20 py-3 rounded-lg font-semibold transition">
                  Withdraw & Claim
                </button>
              </div>
            </div>
          </div>
        )}
      </main>

      {/* Footer */}
      <footer className="p-6 text-center text-gray-500 mt-12">
        <p>
          Built by Bobby 🤖 for Colosseum Agent Hackathon 2026 •{' '}
          <a href="https://github.com/bobby-ai-dev/manna-protocol" className="text-purple-400 hover:underline">
            GitHub
          </a>
        </p>
      </footer>
    </div>
  );
}
