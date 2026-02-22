use anchor_lang::prelude::*;

/// Generic strategy adapter CPI interface used by K1 multi-strategy vault integrations.
///
/// Concrete adapters can map these calls to external protocol instructions.
pub trait StrategyAdapter {
    fn deposit(vault_usdc_ata: &AccountInfo, strategy_account: &AccountInfo, amount: u64)
        -> Result<()>;

    fn withdraw(
        vault_usdc_ata: &AccountInfo,
        strategy_account: &AccountInfo,
        amount: u64,
    ) -> Result<()>;

    fn get_nav(strategy_account: &AccountInfo) -> Result<u128>;
}
