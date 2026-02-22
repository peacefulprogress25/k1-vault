use anchor_lang::prelude::*;
use pyth_solana_receiver_sdk::price_update::PriceUpdateV2;

use crate::{KaminoVaultError, VaultState};

pub fn refresh_strategy_nav(
    ctx: Context<RefreshStrategyNav>,
    strategy_id: Pubkey,
    strategy_token_balance: u64,
    max_price_age_sec: u64,
) -> Result<()> {
    let vault = &mut ctx.accounts.vault_state.load_mut()?;
    require_keys_eq!(ctx.accounts.signer.key(), vault.vault_admin_authority);

    let idx = vault
        .get_strategy_idx(&strategy_id)
        .ok_or(KaminoVaultError::StrategyNotFound)?;
    let strategy = &mut vault.strategies[idx];

    require!(strategy.enabled == 1, KaminoVaultError::StrategyDisabled);
    require_keys_eq!(strategy.oracle_price_feed, ctx.accounts.price_update.key());

    let clock = Clock::get()?;
    let price = ctx
        .accounts
        .price_update
        .get_price_no_older_than(&clock, max_price_age_sec, &strategy.oracle_feed_id)
        .map_err(|_| error!(KaminoVaultError::OraclePriceInvalid))?;

    require_gte!(price.price, 0, KaminoVaultError::OraclePriceInvalid);
    let price_i = i128::from(price.price);
    let exponent = price.exponent;

    // nav_usdc = token_balance * price * 10^(6 + exponent - token_decimals)
    let mut nav = i128::from(strategy_token_balance)
        .checked_mul(price_i)
        .ok_or(KaminoVaultError::MathOverflow)?;
    let shift = 6i32 + exponent - i32::from(strategy.token_decimals);
    if shift >= 0 {
        nav = nav
            .checked_mul(10i128.pow(shift as u32))
            .ok_or(KaminoVaultError::MathOverflow)?;
    } else {
        nav /= 10i128.pow((-shift) as u32);
    }

    require_gte!(nav, 0, KaminoVaultError::OraclePriceInvalid);
    strategy.last_nav = u128::try_from(nav).map_err(|_| error!(KaminoVaultError::MathOverflow))?;
    strategy.last_nav_timestamp = clock.unix_timestamp as u64;
    strategy.invested_amount = strategy_token_balance;

    vault.last_nav_refresh_ts = clock.unix_timestamp as u64;
    vault.prev_aum_sf = vault.compute_total_nav()?;

    Ok(())
}

#[derive(Accounts)]
pub struct RefreshStrategyNav<'info> {
    #[account(mut)]
    pub signer: Signer<'info>,
    #[account(mut)]
    pub vault_state: AccountLoader<'info, VaultState>,
    pub price_update: Account<'info, PriceUpdateV2>,
}
