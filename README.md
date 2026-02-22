# K1 Multi-Strategy Vault (Adapted from Kamino Kvault)

This repository extends the original Kamino vault codebase to support a generic multi-strategy model for USDC deposits, vault shares, and strategy-based AUM/NAV accounting.

## What is implemented

- Strategy registry embedded in `VaultState` with `StrategyEntry` records (`strategy_id`, type, weight, cap, invested amount, NAV).
- Admin strategy management instructions:
  - `add_strategy`
  - `update_strategy`
  - `remove_strategy`
  - `rebalance_vault`
- Vault-level NAV aggregation helper: `compute_total_nav()`.
- Generic strategy adapter interface template in `operations/strategy_adapter.rs`.

## Current architecture notes

- Legacy reserve-allocation flows from Kamino kvault are still present for backward compatibility.
- The new strategy controls are additive and intended as the migration path for arbitrary DeFi and RWA adapters.
- Rebalance currently executes accounting-level movement in state (adapter CPI wiring can be added per strategy implementation).

## Deployments

Program IDs from original kvault:

- Mainnet: `KvauGMspG5k6rtzrqqn7WNn3oZdyKqLKwK2XWQ8FLjd`
- Staging (Mainnet): `stKvQfwRsQiKnLtMNVLHKS3exFJmZFsgfzBPWHECUYK`
