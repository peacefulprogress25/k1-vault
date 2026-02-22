use anchor_lang::prelude::*;

use crate::{KaminoVaultError, StrategyEntry, VaultState, MAX_STRATEGIES};

pub fn add_strategy(
    ctx: Context<ManageStrategy>,
    strategy_id: Pubkey,
    strategy_type: u8,
    weight: u64,
    cap: u64,
) -> Result<()> {
    let vault = &mut ctx.accounts.vault_state.load_mut()?;
    require_keys_eq!(ctx.accounts.signer.key(), vault.vault_admin_authority);

    require!(vault.get_strategy_idx(&strategy_id).is_none(), KaminoVaultError::StrategyAlreadyExists);

    let mut empty_idx = None;
    for (idx, strategy) in vault.strategies.iter().enumerate() {
        if strategy.strategy_id == Pubkey::default() {
            empty_idx = Some(idx);
            break;
        }
    }

    let idx = empty_idx.ok_or(KaminoVaultError::StrategySpaceExhausted)?;
    let new_total_weight = vault
        .total_strategy_weight()?
        .checked_add(weight)
        .ok_or(KaminoVaultError::MathOverflow)?;
    require_gte!(10_000u64, new_total_weight, KaminoVaultError::TotalStrategyWeightTooBig);

    vault.strategies[idx] = StrategyEntry {
        strategy_id,
        strategy_type,
        token_decimals: 6,
        withdraw_priority: 1000,
        enabled: 1,
        _padding: [0; 3],
        target_weight: weight,
        allocation_cap: cap,
        invested_amount: 0,
        last_nav: 0,
        last_nav_timestamp: 0,
        oracle_price_feed: Pubkey::default(),
        oracle_feed_id: [0; 32],
    };
    vault.strategy_count = vault.get_strategies_count() as u64;

    Ok(())
}

pub fn update_strategy(
    ctx: Context<ManageStrategy>,
    strategy_id: Pubkey,
    new_weight: u64,
    new_cap: u64,
) -> Result<()> {
    let vault = &mut ctx.accounts.vault_state.load_mut()?;
    require_keys_eq!(ctx.accounts.signer.key(), vault.vault_admin_authority);

    let idx = vault
        .get_strategy_idx(&strategy_id)
        .ok_or(KaminoVaultError::StrategyNotFound)?;

    let old_weight = vault.strategies[idx].target_weight;
    let mut total_weight = vault.total_strategy_weight()?;
    total_weight = total_weight
        .checked_sub(old_weight)
        .ok_or(KaminoVaultError::MathOverflow)?
        .checked_add(new_weight)
        .ok_or(KaminoVaultError::MathOverflow)?;
    require_gte!(10_000u64, total_weight, KaminoVaultError::TotalStrategyWeightTooBig);

    vault.strategies[idx].target_weight = new_weight;
    vault.strategies[idx].allocation_cap = new_cap;

    Ok(())
}


pub fn update_strategy_oracle(
    ctx: Context<ManageStrategy>,
    strategy_id: Pubkey,
    oracle_price_feed: Pubkey,
    oracle_feed_id: [u8; 32],
    token_decimals: u8,
    withdraw_priority: u16,
) -> Result<()> {
    let vault = &mut ctx.accounts.vault_state.load_mut()?;
    require_keys_eq!(ctx.accounts.signer.key(), vault.vault_admin_authority);

    let idx = vault
        .get_strategy_idx(&strategy_id)
        .ok_or(KaminoVaultError::StrategyNotFound)?;
    vault.strategies[idx].oracle_price_feed = oracle_price_feed;
    vault.strategies[idx].oracle_feed_id = oracle_feed_id;
    vault.strategies[idx].token_decimals = token_decimals;
    vault.strategies[idx].withdraw_priority = withdraw_priority;

    Ok(())
}

pub fn remove_strategy(ctx: Context<ManageStrategy>, strategy_id: Pubkey) -> Result<()> {
    let vault = &mut ctx.accounts.vault_state.load_mut()?;
    require_keys_eq!(ctx.accounts.signer.key(), vault.vault_admin_authority);

    let idx = vault
        .get_strategy_idx(&strategy_id)
        .ok_or(KaminoVaultError::StrategyNotFound)?;

    require_eq!(vault.strategies[idx].invested_amount, 0, KaminoVaultError::StrategyNotUnwound);

    vault.strategies[idx] = StrategyEntry::default();
    vault.strategy_count = vault.get_strategies_count() as u64;

    Ok(())
}

pub fn rebalance_vault(ctx: Context<ManageStrategy>) -> Result<()> {
    let vault = &mut ctx.accounts.vault_state.load_mut()?;
    require_keys_eq!(ctx.accounts.signer.key(), vault.vault_admin_authority);

    let total_weight = vault.total_strategy_weight()?;
    if total_weight == 0 {
        return Ok(());
    }

    let now_ts = Clock::get()?.unix_timestamp as u64;
    let aum = vault.compute_total_nav()?;

    let mut token_available = vault.token_available;
    for idx in 0..vault.strategies.len() {
        if vault.strategies[idx].strategy_id == Pubkey::default() {
            continue;
        }

        let target = (aum
            .checked_mul(u128::from(vault.strategies[idx].target_weight))
            .ok_or(KaminoVaultError::MathOverflow)?
            / u128::from(total_weight)) as u64;
        let capped_target = target.min(vault.strategies[idx].allocation_cap);

        if vault.strategies[idx].invested_amount < capped_target {
            let needed = capped_target - vault.strategies[idx].invested_amount;
            let alloc = needed.min(token_available);
            vault.strategies[idx].invested_amount = vault.strategies[idx]
                .invested_amount
                .checked_add(alloc)
                .ok_or(KaminoVaultError::MathOverflow)?;
            vault.strategies[idx].last_nav = u128::from(vault.strategies[idx].invested_amount);
            vault.strategies[idx].last_nav_timestamp = now_ts;
            vault.strategies[idx].last_nav_timestamp = now_ts;
            token_available = token_available
                .checked_sub(alloc)
                .ok_or(KaminoVaultError::MathOverflow)?;
        } else if vault.strategies[idx].invested_amount > capped_target {
            let excess = vault.strategies[idx].invested_amount - capped_target;
            vault.strategies[idx].invested_amount = vault.strategies[idx]
                .invested_amount
                .checked_sub(excess)
                .ok_or(KaminoVaultError::MathOverflow)?;
            vault.strategies[idx].last_nav = u128::from(vault.strategies[idx].invested_amount);
            vault.strategies[idx].last_nav_timestamp = now_ts;
            token_available = token_available
                .checked_add(excess)
                .ok_or(KaminoVaultError::MathOverflow)?;
        }
    }
    vault.token_available = token_available;

    vault.prev_aum_sf = vault.compute_total_nav()?;
    vault.last_nav_refresh_ts = now_ts;
    Ok(())
}

#[derive(Accounts)]
pub struct ManageStrategy<'info> {
    #[account(mut)]
    pub signer: Signer<'info>,
    #[account(mut)]
    pub vault_state: AccountLoader<'info, VaultState>,
}

const _: usize = MAX_STRATEGIES;
