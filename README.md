# Solana Yield Vault Aggregator

A Solana-based yield aggregator built with Anchor. It manages USDC deposits and dynamically allocates funds to lending protocols (currently Kamino and Marginfi) based on APY, using an off-chain tracker and keeper service.

## Project Structure
```
├── programs/
│   └── yield-vault/           # Anchor on-chain program
├── cli/                       # Command-line tool to deposit/withdraw as a user
├── keeper/                    # Off-chain service managing rebalancing
```
---

##  Components Overview

### 1. On-chain Program (`programs/yield-vault`)

- Built with Anchor Rust.
- Maintains vault accounts per user.
- User-facing instructions:
  - `initialize` — sets up vault and Marginfi account.
  - `deposit` & `withdraw` — move USDC between user and vault's token account.
- Keeper-only instructions: `deploy_to_kamino`, `withdraw_from_kamino`, `deploy_to_marginfi`, `withdraw_from_marginfi`.
- Access control:
  - Users only control deposits/withdrawals to/from vault.
  - Only the authorized Keeper (hardcoded pubkey or PDA) can cause funds to move into or out of external lending protocols.
- Strategy state (`active_protocol`) is tracked on-chain per vault.

---

### 2. CLI (`cli`)

- Rust-based command-line tool for end-users.
- Commands:
  - `initialize` — sets up vault for a user.
  - `deposit` — sends USDC to the vault ATA.
  - `withdraw` — withdraws available USDC.
- Intended for testing and user interaction.
- Runs locally—users provide their keypair path as argument.

---

### 3. Keeper Service (`keeper`)

- A long-running Rust-based HTTP server and RPC client.
- Responsibilities:
  - Exposes HTTP endpoints (e.g., `/deposit`, `/withdraw`) for CLI to trigger protocol deploy/withdraw operations.
  - Contains a background **Tracker** that:
    - Periodically fetches APYs from Kamino API and Marginfi on-chain.
    - Decides which protocol to use (based on APY, for now ignoring fees).
    - Rebalances assets: withdraws from current protocol, then deploys to the more profitable one.
    - Logs each operation and updates shared state (`AppState.strategy`).
- Holds an in-memory `Vec<Pubkey>` of **lender users** to act upon during rebalance.

---

## How It Works Together

1. **User Flow**:
   - CLI invokes `deposit`: transfers USDC from user to vault.
   - Keeper endpoint (HTTP) may be triggered to deploy funds to preferred lending protocol.

2. **Rebalancing Flow**:
   - Tracker runs hourly.
   - Fetches Kamino and Marginfi supply APYs.
   - If a better APY is found:
     - Keeps track of all lenders.
     - Withdraws from current protocol for each user’s vault.
     - Deposits funds into higher APY protocol.
     - Updates the strategy state.

---

## Development & Testing

### Prerequisites

- Install Rust, Solana CLI, and Anchor.
- For local testing, use `anchor test` which auto-starts a local validator.

### Running Locally

#### On-chain Program
```bash
cd programs/yield-vault
anchor build
anchor deploy --provider.cluster localhost
```