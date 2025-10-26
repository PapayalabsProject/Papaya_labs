use anchor_lang::prelude::*;
use anchor_lang::solana_program::pubkey::Pubkey;
use anchor_spl::token_interface::{ TokenInterface, Mint };
use anchor_spl::token::{ Token, Transfer, TokenAccount };
use anchor_spl::token::transfer;
use borsh::BorshDeserialize;
use std::convert::{ TryInto, TryFrom };
#[allow(unused_imports)]
use solana_security_txt::security_txt;

declare_id!("PAYaBWtWMvLTbK4KEdaM8d6swgURyt2XoM3PuM9nQ9j");

#[cfg(not(feature = "no-entrypoint"))]
security_txt! {
    name: "Papaya Labs",
    project_url: "https://papaya-labs.xyz/",
    contacts: "email:contact@papaya-labs.xyz, twitter:@papaya_labs, link:https://papaya-labs.xyz/",
    policy: "https://github.com/PapayalabsProject/Papaya_labs/blob/main/README.md",
    preferred_languages: "en",
    source_code: "https://github.com/PapayalabsProject/Papaya_labs"
}

fn iter_all_eq<T: PartialEq>(iter: impl IntoIterator<Item = T>) -> Option<T> {
    let mut iter = iter.into_iter();
    let first = iter.next()?;
    iter.all(|elem| elem == first).then(|| first)
}
fn calculate_reward(
    amount: u64,
    time_elapsed_ms: u64,
    base_hour: u64,
    base_rate: f32,
    total_vault_amount: u64,
    start_pool: u64
) -> Option<u64> {
    if total_vault_amount == 0 || start_pool == 0 || base_hour == 0 {
        return None;
    }

    let held_hours = (time_elapsed_ms / 3_600_000).min(24);
    if held_hours < base_hour {
        return Some(0);
    }

    let pool_percent = (total_vault_amount as f64) / (start_pool as f64);
    let mut current_amount = amount as f64;

    for hour in 1..=held_hours {
        if hour % base_hour == 0 {
            let step_multiplier = ((base_rate as f64) * pool_percent) / 100.0;
            current_amount *= 1.0 + step_multiplier;
        }
    }

    Some(current_amount.floor() as u64)
}

#[program]
pub mod papaya {
    use super::*;

    pub fn create_papaya_vault(
        ctx: Context<CreateVault>,
        amount: u64,
        base_rate: f32,
        base_hour: u32
    ) -> Result<()> {
        let vault = &mut ctx.accounts.vault;
        let creator_token_account = &ctx.accounts.creator_token_account;

        require!(amount > 0, PapayaError::InvalidAmount);
        require!(vault.total_amount() == 0, PapayaError::AlreadyExists);
        require!(creator_token_account.amount >= amount, PapayaError::InsufficientFunds);

        vault.token = ctx.accounts.mint.key();
        
        let amount_per_vault = amount / 10;
        let remainder = amount % 10;
        
        for i in 0..10 {
            vault.vault_amounts[i] = amount_per_vault + if i < remainder as usize { 1 } else { 0 };
            vault.vault_amounts_staked[i] = 0;
        }
        
        vault.start_pool = amount;
        vault.base_rate = base_rate;
        vault.base_hour = base_hour;
        vault.total_stakers = 0;
        vault.current_stakers = 0;

        Ok(())
    }

    pub fn create_vault_token_account(
        ctx: Context<CreateVaultTokenAccount>,
        vault_index: u8
    ) -> Result<()> {
        require!(vault_index < 10, PapayaError::OutOfRange);
        
        let vault = &ctx.accounts.vault;
        let creator_token_account = &ctx.accounts.creator_token_account;
        let vault_token_account = &ctx.accounts.vault_token_account;
        
        let transfer_amount = vault.vault_amounts[vault_index as usize];
        
        if transfer_amount > 0 {
            let cpi_accounts = Transfer {
                from: creator_token_account.to_account_info(),
                to: vault_token_account.to_account_info(),
                authority: ctx.accounts.creator.to_account_info(),
            };
            let cpi_program = ctx.accounts.token_program.to_account_info();

            transfer(CpiContext::new(cpi_program, cpi_accounts), transfer_amount)?;
        }

        Ok(())
    }

    pub fn deposit_papaya(ctx: Context<Deposit>, amount: u64, index: u32, vault_index: u8) -> Result<()> {
        let clock = Clock::get()?;
        let user_counter = &mut ctx.accounts.user_interactions_counter;
        let vault = &mut ctx.accounts.vault;
        let depositor_token_account = &ctx.accounts.depositor_token_account;

        require!((100_000_000..=10_000_000_000_000).contains(&amount), PapayaError::InvalidAmount);
        require!(index <= 4, PapayaError::OutOfRange);
        require!(vault_index < 10, PapayaError::OutOfRange);
        require!(depositor_token_account.amount >= amount, PapayaError::InsufficientFunds);

        let expected_vault_token_account = get_token_account_pda(&ctx.accounts.mint.key(), vault_index, ctx.program_id).0;
        require!(ctx.accounts.vault_token_account.key() == expected_vault_token_account, PapayaError::InvalidVaultAccount);

        let index = usize::try_from(index).map_err(|_| PapayaError::OutOfRange)?;
        require!(user_counter.total_deposits[index] == 0, PapayaError::AlreadyStaked);

        if
            user_counter.total_deposits[0] == 0 &&
            !iter_all_eq(user_counter.total_deposits).is_none()
        {
            vault.total_stakers += 1;
            vault.current_stakers += 1;
        }

        let timestamp = clock.unix_timestamp as u64;
        user_counter.total_deposits[index] = amount;
        user_counter.time_deposits[index] = timestamp;
        user_counter.stake_deposits[index] = timestamp;
        user_counter.vault_indices[index] = vault_index;

        vault.vault_amounts_staked[vault_index as usize] += amount;

        let cpi_accounts = Transfer {
            from: depositor_token_account.to_account_info(),
            to: ctx.accounts.vault_token_account.to_account_info(),
            authority: ctx.accounts.depositor.to_account_info(),
        };
        let cpi_program = ctx.accounts.token_program.to_account_info();
        transfer(CpiContext::new(cpi_program, cpi_accounts), amount)?;

        Ok(())
    }

    pub fn withdraw_papaya(ctx: Context<Withdraw>, index: u32, reward_only: bool) -> Result<()> {
        let clock = Clock::get()?;
        let now: u64 = clock.unix_timestamp.try_into().map_err(|_| PapayaError::TimeConversionError)?;

        let user_counter = &mut ctx.accounts.user_interactions_counter;
        let vault = &mut ctx.accounts.vault;
        let vault_token_account = &ctx.accounts.vault_token_account;

        let index = usize::try_from(index).map_err(|_| PapayaError::OutOfRange)?;
        require!(index <= 4, PapayaError::OutOfRange);

        let amount = user_counter.total_deposits[index];
        require!(amount > 0, PapayaError::NoDeposits);

        let vault_index = user_counter.vault_indices[index] as usize;
        require!(vault_index < 10, PapayaError::OutOfRange);

        let expected_vault_token_account = get_token_account_pda(&ctx.accounts.mint.key(), vault_index as u8, ctx.program_id).0;
        require!(vault_token_account.key() == expected_vault_token_account, PapayaError::InvalidVaultAccount);

        let seed = ctx.accounts.mint.key();
        let vault_seed = get_token_vault_seed(vault_index as u8);
        let (_, bump_seed) = get_token_account_pda(&seed, vault_index as u8, ctx.program_id);
        let signer_seeds: &[&[&[u8]]] = &[&[&vault_seed, seed.as_ref(), &[bump_seed]]];

        let stake_time = user_counter.stake_deposits[index];
        let time_elapsed = now.saturating_sub(stake_time).saturating_mul(1_000);

        let mut withdraw_amount = amount;

        if
            let Some(reward) = calculate_reward(
                amount,
                time_elapsed,
                u64::from(vault.base_hour),
                vault.base_rate,
                vault.total_amount(),
                vault.start_pool
            )
        {
            if reward > 0 {
                require!(vault.vault_amounts[vault_index] >= reward, PapayaError::EmptyVault);
                let gain = reward.checked_sub(withdraw_amount).ok_or(PapayaError::MathOverflow)?;
                vault.vault_amounts[vault_index] = vault.vault_amounts[vault_index]
                    .checked_sub(gain)
                    .ok_or(PapayaError::MathOverflow)?;

                withdraw_amount = if reward_only {
                    gain
                } else {
                    reward
                };
            } else if reward_only {
                withdraw_amount = 0;
            }
        }

        if reward_only {
            user_counter.stake_deposits[index] = now;
        } else {
            user_counter.total_deposits[index] = 0;
            user_counter.time_deposits[index] = 0;
            user_counter.stake_deposits[index] = 0;
            user_counter.vault_indices[index] = 0;

            vault.vault_amounts_staked[vault_index] = vault.vault_amounts_staked[vault_index]
                .checked_sub(amount)
                .ok_or(PapayaError::MathOverflow)?;
        }

        if
            user_counter.total_deposits[0] == 0 &&
            !iter_all_eq(user_counter.total_deposits).is_none()
        {
            vault.current_stakers = vault.current_stakers.saturating_sub(1);
        }
        
        if withdraw_amount > 0 {
            let cpi_accounts = Transfer {
                from: vault_token_account.to_account_info(),
                to: ctx.accounts.withdrawer_token_account.to_account_info(),
                authority: vault_token_account.to_account_info(),
            };
            let cpi_program = ctx.accounts.token_program.to_account_info();

            transfer(
                CpiContext::new(cpi_program, cpi_accounts).with_signer(signer_seeds),
                withdraw_amount
            )?;
        }

        Ok(())
    }

    pub fn get_vault_info(ctx: Context<GetVaultInfo>) -> Result<VaultInfo> {
        let vault = &ctx.accounts.vault;
        
        Ok(VaultInfo {
            token: vault.token,
            vault_amounts: vault.vault_amounts,
            vault_amounts_staked: vault.vault_amounts_staked,
            total_amount: vault.total_amount(),
            total_staked: vault.total_staked(),
            start_pool: vault.start_pool,
            base_rate: vault.base_rate,
            base_hour: vault.base_hour,
            total_stakers: vault.total_stakers,
            current_stakers: vault.current_stakers,
        })
    }

    pub fn get_optimal_vault_for_deposit(ctx: Context<GetVaultInfo>) -> Result<u8> {
        let vault = &ctx.accounts.vault;
        let optimal_index = vault.get_vault_index_for_deposit();
        Ok(optimal_index as u8)
    }
}

#[derive(Accounts)]
pub struct CreateVault<'info> {
    #[account(init, payer = creator, space = 8 + 32 + (8 * 10) + (8 * 10) + 8 + 4 + 4 + 8 + 8, seeds = [b"vault", mint.key().as_ref()], bump)]
    pub vault: Account<'info, Vault>,
    #[account(mut)]
    pub creator: Signer<'info>,
    #[account(mut, token::authority = creator.key(), token::mint = mint.key())]
    pub creator_token_account: Account<'info, TokenAccount>,
    pub mint: InterfaceAccount<'info, Mint>,
    pub token_program: Interface<'info, TokenInterface>,
    pub system_program: Program<'info, System>,
    pub rent: Sysvar<'info, Rent>,
}

#[derive(Accounts)]
#[instruction(vault_index: u8)]
pub struct CreateVaultTokenAccount<'info> {
    #[account(seeds = [b"vault", mint.key().as_ref()], bump)]
    pub vault: Account<'info, Vault>,
    #[account(mut)]
    pub creator: Signer<'info>,
    #[account(
        init,
        payer = creator,
        token::mint = mint,
        token::authority = vault_token_account,
        token::token_program = token_program,
        seeds = [&get_token_vault_seed(vault_index), mint.key().as_ref()],
        bump
    )]
    pub vault_token_account: Account<'info, TokenAccount>,
    #[account(mut, token::authority = creator.key(), token::mint = mint.key())]
    pub creator_token_account: Account<'info, TokenAccount>,
    pub mint: InterfaceAccount<'info, Mint>,
    pub token_program: Interface<'info, TokenInterface>,
    pub system_program: Program<'info, System>,
    pub rent: Sysvar<'info, Rent>,
}

#[derive(Accounts)]
#[instruction(amount: u64, index: u32, vault_index: u8)]
pub struct Deposit<'info> {
    #[account(mut, seeds = [b"vault", mint.key().as_ref()], bump)]
    pub vault: Account<'info, Vault>,
    #[account(mut)]
    pub depositor: Signer<'info>,
    #[account(mut, token::authority = depositor.key(), token::mint = mint.key())]
    pub depositor_token_account: Account<'info, TokenAccount>,
    #[account(mut, token::mint = mint,
        token::authority = vault_token_account,
        token::token_program = token_program)]
    pub vault_token_account: Account<'info, TokenAccount>,
    #[account(mut)]
    pub mint: InterfaceAccount<'info, Mint>,
    #[account(
        init_if_needed,
        space = 8 + (8 * 5) + (8 * 5) + (8 * 5) + (1 * 5),
        seeds = [b"interactor", depositor.key().as_ref(), mint.key().as_ref()],
        bump,
        payer = depositor
    )]
    pub user_interactions_counter: Account<'info, UserInteractions>,
    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,
}
#[derive(Accounts)]
#[instruction(index: u32, reward_only: bool)]
pub struct Withdraw<'info> {
    #[account(mut, seeds = [b"vault", mint.key().as_ref()], bump)]
    pub vault: Account<'info, Vault>,
    #[account(mut)]
    pub withdrawer: Signer<'info>,
    #[account(mut, token::authority = withdrawer.key(), token::mint = mint.key())]
    pub withdrawer_token_account: Account<'info, TokenAccount>,
    #[account(mut)]
    pub vault_token_account: Account<'info, TokenAccount>,
    #[account(mut)]
    pub mint: InterfaceAccount<'info, Mint>,
    #[account(
        mut,
        seeds = [b"interactor", withdrawer.key().as_ref(), mint.key().as_ref()],
        bump,
    )]
    pub user_interactions_counter: Account<'info, UserInteractions>,
    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct GetVaultInfo<'info> {
    #[account(seeds = [b"vault", mint.key().as_ref()], bump)]
    pub vault: Account<'info, Vault>,
    pub mint: InterfaceAccount<'info, Mint>,
}

#[derive(AnchorSerialize, AnchorDeserialize)]
pub struct VaultInfo {
    pub token: Pubkey,
    pub vault_amounts: [u64; 10],
    pub vault_amounts_staked: [u64; 10], 
    pub total_amount: u64,
    pub total_staked: u64,
    pub start_pool: u64,
    pub base_rate: f32,
    pub base_hour: u32,
    pub total_stakers: u64,
    pub current_stakers: u64,
}

#[account]
pub struct Vault {
    pub token: Pubkey,
    pub vault_amounts: [u64; 10],      
    pub vault_amounts_staked: [u64; 10], 
    pub start_pool: u64,             
    pub base_rate: f32,
    pub base_hour: u32,
    pub total_stakers: u64,
    pub current_stakers: u64,
}

impl Vault {
    pub fn total_amount(&self) -> u64 {
        self.vault_amounts.iter().sum()
    }
    
    pub fn total_staked(&self) -> u64 {
        self.vault_amounts_staked.iter().sum()
    }
    
    pub fn get_vault_index_for_deposit(&self) -> usize {
        self.vault_amounts_staked
            .iter()
            .enumerate()
            .min_by_key(|(_, &amount)| amount)
            .map(|(index, _)| index)
            .unwrap_or(0)
    }
    
    pub fn get_vault_index_for_withdrawal(&self, required_amount: u64) -> Option<usize> {
        self.vault_amounts
            .iter()
            .enumerate()
            .find(|(_, &amount)| amount >= required_amount)
            .map(|(index, _)| index)
    }
}

pub fn get_token_vault_seed(vault_index: u8) -> Vec<u8> {
    format!("token_vault_{}", vault_index).into_bytes()
}

pub fn get_token_account_pda(mint: &Pubkey, vault_index: u8, program_id: &Pubkey) -> (Pubkey, u8) {
    let seed = get_token_vault_seed(vault_index);
    Pubkey::find_program_address(&[&seed, mint.as_ref()], program_id)
}

#[account]
pub struct UserInteractions {
    total_deposits: [u64; 5],
    time_deposits: [u64; 5],
    stake_deposits: [u64; 5],
    vault_indices: [u8; 5], 
}

#[error_code]
pub enum PapayaError {
    #[msg("No papaya staked")]
    NoDeposits,
    #[msg("Papaya amount out of range")]
    InvalidAmount,
    #[msg("Papaya stake index out of range")]
    OutOfRange,
    #[msg("Papaya vault already initialized")]
    AlreadyExists,
    #[msg("Not enough papaya to deposit")]
    InsufficientFunds,
    #[msg("Account already has an active stake")]
    AlreadyStaked,
    #[msg("Vault is empty")]
    EmptyVault,
    #[msg("Invalid timestamp")]
    TimeConversionError,
    #[msg("Math overflow")]
    MathOverflow,
    #[msg("Invalid vault token account")]
    InvalidVaultAccount,
}
