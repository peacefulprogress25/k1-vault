use anchor_lang::prelude::*;
use anchor_spl::token_interface::{TokenAccount, TokenInterface};
use pyth_solana_receiver_sdk::price_update::PriceUpdateV2;

use crate::{KaminoVaultError, VaultState};

pub fn refresh_strategy_nav(
    ctx: Context<RefreshStrategyNav>,
    strategy_id: Pubkey,
    max_price_age_sec: u64,
) -> Result<()> {
    let vault = &mut ctx.accounts.vault_state.load_mut()?;
    require_keys_eq!(ctx.accounts.signer.key(), vault.vault_admin_authority);

    let base_vault_authority = vault.base_vault_authority;
    let idx = vault
        .get_strategy_idx(&strategy_id)
        .ok_or(KaminoVaultError::StrategyNotFound)?;
    let strategy = &mut vault.strategies[idx];

    require!(strategy.enabled == 1, KaminoVaultError::StrategyDisabled);
    require_keys_eq!(strategy.oracle_price_feed, ctx.accounts.price_update.key());
    require_keys_eq!(strategy.strategy_token_mint, ctx.accounts.strategy_token_account.mint);
    require_keys_eq!(ctx.accounts.strategy_token_account.owner, base_vault_authority);

    let clock = Clock::get()?;
    let price = ctx
        .accounts
        .price_update
        .get_price_no_older_than(&clock, max_price_age_sec, &strategy.oracle_feed_id)
        .map_err(|_| error!(KaminoVaultError::OraclePriceInvalid))?;

    require_gte!(price.price, 0, KaminoVaultError::OraclePriceInvalid);
    let price_i = i128::from(price.price);
    let exponent = price.exponent;
    let abs_price = price.price.unsigned_abs();
    let conf_bps = if abs_price == 0 { u64::MAX } else { price.conf.saturating_mul(10_000) / abs_price };
    require_gte!(u64::from(strategy.max_oracle_conf_bps), conf_bps, KaminoVaultError::OracleConfidenceTooHigh);


    let strategy_token_balance = ctx.accounts.strategy_token_account.amount;
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
    #[account(token::token_program = token_program)]
    pub strategy_token_account: InterfaceAccount<'info, TokenAccount>,
    pub token_program: Interface<'info, TokenInterface>,
}
