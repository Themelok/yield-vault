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
    - CLI invokes `initialize` to:
        - create vault account for user.
        - create USDC Associated Token Account (ATA) for user.
        - create and initialize marginfi account for user.
   - CLI invokes `deposit`: transfers USDC:
     - from user to vault ATA;
     - from vault ATA to Lending Protocol;
    - CLI invokes `withdraw`: transfers USDC:
     - from Lending Protocol to vault ATA;
     - from vault ATA to user;

2. **Rebalancing Flow**:
Keeper runs hourly APY tracker to:
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
- [install Surfpool](https://github.com/txtx/surfpool?tab=readme-ov-file#installation) local validator and:
    - generate `bot.json` and `user.json` for keeper and user with `solana-keygen new`;
    - put `bot.json` in `~/.config/solana/` for keeper;
    - put `user.json` in `~/.config/solana/` for user;
    - change `KEEPER_PUBKEY` in `programs/yield-vault/src/lib.rs` to the pubkey of the keeper;

### Running Locally

#### On-chain Program
```bash
cd programs/yield-vault
anchor build
surfpool start --no-tui > sp.log
```
surfpool dashboard will be available at http://127.0.0.1:18488

#### Off-chain Program
##### Keeper
```bash
cd keeper
RUST_LOG=info cargo run -- <PATH_TO_BOT_JSON_KEYPAIR>
```

##### CLI
In the separate terminal:
```bash
cd cli
cargo run -- init <PATH_TO_USER_JSON_KEYPAIR>
cargo run -- deposit -a 100000000 <PATH_TO_USER_JSON_KEYPAIR>
cargo run -- withdraw <PATH_TO_USER_JSON_KEYPAIR>
```

