<div align="center">
  <img src="https://i.imgur.com/bo3czvl.png" alt="Papaya Labs Banner" width="100%">
</div>

<div align="center">
  <p><em>A yield-generating protocol inspired by nature's rhythm of papaya production, where token holders can incubate their assets to harvest more $PAPAYA.
</em></p>
  
  [![Twitter Follow](https://img.shields.io/twitter/follow/Papaya_Labs?style=for-the-badge&logo=twitter&color=1DA1F2)](https://x.com/Papaya_Labs)
  [![Website](https://img.shields.io/badge/Website-papaya--labs.xyz-green?style=for-the-badge&logo=web)](http://papaya-labs.xyz/)
  [![Solana](https://img.shields.io/badge/Built%20on-Solana-blueviolet?style=for-the-badge&logo=solana)](https://solana.com)
</div>

---

## What is Papaya Labs?

Papaya Labs introduces a groundbreaking approach to token distribution and staking on Solana. Unlike traditional token launches where developers hold large allocations in wallets they control, **20% of our total supply (200M $PAPAPA tokens) is permanently locked in an unbreakable smart contract vault**.

### Key Features

- **Provably Secure Vault**: 200M tokens locked in Program Derived Address with no private key
- **Dynamic Staking Rewards**: Rates adjust based on pool utilization - early stakers get higher rewards
- **Flexible Time Periods**: Stake for as little as 6 hours, earn compound rewards
- **No Lockup Periods**: Withdraw your initial stake anytime after minimum period
- **Transparent Economics**: All calculations happen on-chain, fully auditable

## How It Works

### Staking Mechanics

1. **Stake Your Tokens**: Deposit $PAPAPA into the staking contract
2. **Earn Dynamic Rewards**: Base rate of 5% per 6-hour period (20% daily)
3. **Pool Depletion Logic**: As more users claim rewards, future rates decrease automatically
4. **Compound Growth**: Reinvest rewards every 6 hours for maximum returns
5. **Withdraw Anytime**: No lockup - access your stake after minimum 6-hour period

### Dynamic Rate System

| Pool Level | Multiplier | Effective Rate (6h) | Daily Rate |
|------------|------------|-------------------|------------|
| 100% | 100.0% | 5.000% | 20.00% |
| 75% | 75.0% | 3.750% | 15.00% |
| 50% | 50.0% | 2.500% | 10.00% |
| 25% | 25.0% | 1.250% | 5.00% |
| 10% | 10.0% | 0.500% | 2.00% |

### Example Returns

**Staking 100,000 $PAPAPA at different pool levels:**

- **At 100% pool**: +21,550 $PAPAPA/day
- **At 75% pool**: +15,969 $PAPAPA/day  
- **At 50% pool**: +10,517 $PAPAPA/day
- **At 25% pool**: +5,195 $PAPAPA/day
- **At 10% pool**: +2,020 $PAPAPA/day

*Early participants capture the highest rewards as the system naturally balances over time.*

## Security Architecture

### Program Derived Address (PDA) Vault System

Our security model leverages Solana's Program Derived Address technology - the same architecture used by major protocols like Squads and the SPL Token Program.

#### How the Vault Works

```rust
// Vault's token account created with deterministic seeds - no private key exists
pub vault_token_account: Account<'info, TokenAccount>;
#[account(mut, seeds = [b"token_vault", mint.key().as_ref()], bump)]
// Only the program can sign on behalf of the token vault using PDA authority
let seeds = &[b"token_vault", mint.key().as_ref(), &[bump]];
let signer_seeds = &[&seeds[..]];
let cpi_ctx = CpiContext::new_with_signer(token_program, transfer_accounts, signer_seeds);
transfer(cpi_ctx, amount)?;
```

#### Withdrawal Security

```rust
// Vault's token account created with deterministic seeds - no private key exists
pub vault_token_account: Account<'info, TokenAccount>;
#[account(mut, seeds = [b"token_vault", mint.key().as_ref()], bump)]
// Only the program can sign on behalf of the token vault using PDA authority
let seeds = &[b"token_vault", mint.key().as_ref(), &[bump]];
let signer_seeds = &[&seeds[..]];
let cpi_ctx = CpiContext::new_with_signer(token_program, transfer_accounts, signer_seeds);
transfer(cpi_ctx, amount)?;
```

### Security Guarantees

#### What IS Possible
- **Legitimate Stakers**: Can withdraw deposits + earned rewards after time requirement
- **Automatic Distribution**: Program calculates and distributes fair rewards mathematically
- **Full Transparency**: All operations visible and verifiable on-chain

#### What is NOT Possible
- **Team/Developer Access**: Cannot withdraw any tokens from vault (no private key exists)
- **Emergency Drains**: No admin functions or backdoors to extract funds
- **Bypassing Requirements**: Cannot claim rewards without legitimate staking position
- **Double Spending**: Each position individually tracked with overflow protection

### Code-Level Security Features

1. **Overflow Protection**: All arithmetic uses `checked_add()`, `checked_sub()`, `checked_mul()`
2. **Access Control**: Array bounds checking with proper error handling
3. **PDA Authority**: Only program can authorize vault transfers using cryptographic signatures
4. **Time Enforcement**: Rewards calculated only after minimum staking period
5. **Position Validation**: Individual tracking prevents unauthorized access to others' stakes

## Verification & Transparency

### On-Chain Verification
- **Vault Balance**: Check token balance at vault PDA address
- **Program Code**: Verify deployed bytecode matches source code (Anchor verify enabled)  
- **Transaction History**: Observe that only legitimate staking rewards leave vault
- **PDA Derivation**: Mathematically confirm vault address has no corresponding private key

### Technical Specifications

```rust
// Program Constants
const VAULT_SEED: &[u8] = b"vault";
const TOKEN_VAULT_SEED: &[u8] = b"token_vault";
const INTERACTOR_SEED: &[u8] = b"interactor";

// Vault Structure
pub struct Vault {
    pub amount: u64,           // Current tokens in vault
    pub amount_staked: u64,    // Total tokens currently staked
    pub start_pool: u64,       // Initial vault size (200M)
    pub base_rate: f32,        // Base reward rate (5%)
    pub base_hour: u32,        // Minimum staking period (6 hours)
    pub total_stakers: u64,    // Lifetime staker count
    pub current_stakers: u64   // Active stakers
}
```

## Getting Started

### For Users
1. Visit [papaya-labs.xyz](http://papaya-labs.xyz/)
2. Connect your Solana wallet
3. Stake your $PAPAPA tokens
4. Track rewards in real-time
5. Claim or compound every 6 hours

### For Developers
```bash
# Clone repository
git clone https://github.com/PapayaLabs/papaya-labs

# Install dependencies
npm install

# Build program
anchor build

# Run tests
anchor test

# Deploy (with verification)
anchor deploy --verify
```

## Community

- **Twitter**: [@Papaya_Labs](https://x.com/Papaya_Labs)
- **Website**: [papaya-labs.xyz](http://papaya-labs.xyz/)

---

## Legal & Disclaimers

This protocol is experimental software. Users should understand the risks involved in DeFi participation. Past performance does not guarantee future results. The dynamic rate system means rewards will decrease over time as designed.

**Built with ❤️ on Solana**
