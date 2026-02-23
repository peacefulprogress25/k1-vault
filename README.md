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


## Strategy NAV / Oracle updates

- Added `refresh_strategy_nav` instruction using Pyth `PriceUpdateV2` to reprice strategy NAV from token balance and oracle price.
- Added strategy-mode withdraw behavior that unwinds strategy accounting into deposit token balance before transfering to user.
- Added vault controls for NAV staleness checks (`max_nav_age_sec`) and strategy mode flag (`vault_mode`).


## Admin Dashboard (frontend)

A lightweight admin dashboard is available at `admin-dashboard/`.

### Usage

1. Serve the folder with any static file server (for example: `python3 -m http.server 4173` from repo root).
2. Open one of:
   - Admin page: `http://localhost:4173/admin-dashboard/index.html`
   - User page: `http://localhost:4173/admin-dashboard/user.html`
3. Connect Phantom wallet.
4. Upload your generated Anchor IDL JSON.
5. Set RPC URL, Program ID, and Vault State account.
6. Use forms to invoke admin instructions (`add_strategy`, `update_strategy`, `remove_strategy`, `update_strategy_oracle`, `rebalance_vault`, `refresh_strategy_nav`, etc.).


### End-user relevant instructions

User-facing flows currently exposed in the dashboard:

- `deposit(max_amount)` / `buy(max_amount)` for minting shares.
- `withdraw(shares_amount)` / `sell(shares_amount)` for redeeming deposit token.
- `withdraw_from_available(shares_amount)` for liquid-only redemption path.

Note: user instructions require more accounts than strategy-admin methods (vault/user token ATAs, share mint/accounts, authority/token programs, and reserve-related remaining accounts where applicable). The user page supports these via JSON account inputs in the user section.


## Phase 1/2 hardening updates

- `refresh_strategy_nav` now reads strategy token balance from a real token account in accounts context instead of trusting a caller-supplied amount.
- Added oracle confidence guard (`max_oracle_conf_bps`) and per-strategy token mint binding for NAV updates.
