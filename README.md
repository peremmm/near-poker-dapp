# Trustless Texas Hold'em Poker dApp on NEAR

This project is a proof-of-concept decentralized application for **COSC473 — Decentralised Applications on the Web**.

The dApp implements a trust-reduced **1v1 Texas Hold'em-style poker game** on the NEAR Protocol. The main goal is to reduce the need for players to trust a central dealer, backend server, or platform operator. Instead, the smart contract enforces table creation, buy-in escrow, player order, card dealing, betting actions, pot updates, winner resolution, refunds, withdrawals, and next-round voting.

The project follows **Track B: Building a Trustless Protocol**.

---

## Project Structure

```text
near-poker-dapp/
  contract/      # NEAR smart contract written in Rust
  frontend/      # React + Vite + TypeScript frontend
  README.md
```

---

## Main Features

* NEAR smart contract written in Rust using `near-sdk`
* React frontend using Meteor Wallet only
* 1v1 poker table creation and joining
* Escrowed buy-in and internal player balances
* Small blind and big blind
* Turn-based Check, Call, Raise, and Fold actions
* Automatic stage progression:

  * PreFlop
  * Flop
  * Turn
  * River
  * Showdown
* Automatic poker hand evaluation at showdown
* Fold resolution
* Finished round reveals both players' cards
* Players can vote to play the next round on the same table
* Timeout refund for abandoned active games (10 minutes)
* Asynchronous withdrawal with callback validation
* Storage deposit accounting for table creation and joining

---

# 1. Frontend Setup

The frontend uses:

```text
React
Vite
TypeScript
Meteor Wallet
near-api-js
NEAR Wallet Selector
```

Go to the frontend folder:

```bash
cd frontend
npm install
```

Create your local environment file:

```bash
cp .env.example .env
```

Example `frontend/.env.example`:

```env
VITE_NETWORK_ID=testnet
VITE_CONTRACT_ID=your-account.testnet
VITE_RPC_URL=https://test.rpc.fastnear.com
```

Replace:

```text
your-account.testnet
```

with your deployed contract account.

For example:

```env
VITE_NETWORK_ID=testnet
VITE_CONTRACT_ID=account1.testnet
VITE_RPC_URL=https://test.rpc.fastnear.com
```


Run the frontend:

```bash
npm run dev
```

Open the local Vite URL, usually:

```text
http://localhost:5173
```

The frontend supports:

* Meteor Wallet connection
* Contract view methods
* Create table
* Join table
* Submit poker actions
* Claim timeout refund
* Withdraw
* Play next round


---

# 2. Contract Setup

The smart contract is located in:

```text
contract/
```

This project was developed with:

```text
Rust: 1.88.0
near-sdk: 5.27.0
cargo-near: 0.20.3
NEAR CLI: 0.26.1
```

Install Rust:

```bash
rustup install 1.88.0
```

From the `contract/` folder, set the Rust version for this project:

```bash
cd contract
rustup override set 1.88.0
rustup target add wasm32-unknown-unknown
```

Check Rust version:

```bash
rustc --version
```

Expected:

```text
rustc 1.88.0
```

Install `cargo-near`:

```bash
cargo install cargo-near
```

Check `cargo-near`:

```bash
cargo near --version
```

Install or check NEAR CLI:

```bash
near --version
```

You also need to import your testnet account key locally:

```bash
near account import-account using-web-wallet network-config testnet
```

Set the contract/admin account you want to use:

```bash
export ADMIN=your-account.testnet
```

Example:

```bash
export ADMIN=account1.testnet
```

The frontend contract ID should match the account used in `$ADMIN`.

For example, if you deploy with:

```bash
export ADMIN=account1.testnet
```

then `frontend/.env` should contain:

```env
VITE_CONTRACT_ID=account1.testnet
```

---

# 3. Test, Build, and Deploy Contract

## Run Tests

From the `contract/` folder:

```bash
cargo test
```

Expected:

```text
all tests pass
```

## Build Contract

From the `contract/` folder:

```bash
cargo near build non-reproducible-wasm --skip-rust-version-check
```

The WASM file should be generated at:

```text
contract/target/near/contract.wasm
```

## Deploy Contract
"Contract has been deployed under epe78.testnet and can skip instructions further if you want to skip deployment."

Set your admin/contract account:

```bash
export ADMIN=your-account.testnet
```

Deploy the already-built WASM:

```bash
near contract deploy $ADMIN use-file target/near/contract.wasm without-init-call network-config testnet sign-with-keychain send
```

## Initialize Contract

If this is the first deployment, initialize the contract:

```bash
near contract call-function as-transaction $ADMIN new json-args "{\"owner_id\":\"$ADMIN\",\"min_buy_in\":\"1000000000000000000000000\",\"max_buy_in\":\"10000000000000000000000000\"}" prepaid-gas '30.0 Tgas' attached-deposit '0 NEAR' sign-as $ADMIN network-config testnet sign-with-keychain send
```

The values mean:

```text
min_buy_in = 1 NEAR
max_buy_in = 10 NEAR
```

## Verify Initialization

Check buy-in range:

```bash
near contract call-function as-read-only $ADMIN get_buy_in_range json-args '{}' network-config testnet now
```

Expected output:

```json
{
  "min_buy_in": "1000000000000000000000000",
  "max_buy_in": "10000000000000000000000000"
}
```

Check owner:

```bash
near contract call-function as-read-only $ADMIN get_owner json-args '{}' network-config testnet now
```

Expected output:

```json
"your-account.testnet"
```

## Automated Test, Build, and Deploy Command

From the project root:

```bash
export ADMIN=your-account.testnet

cd contract && \
cargo test && \
cargo near build non-reproducible-wasm --skip-rust-version-check && \
near contract deploy $ADMIN use-file target/near/contract.wasm without-init-call network-config testnet sign-with-keychain send
```

## Development Table Reset

During development, the `Table` storage layout changed several times. Because NEAR stores contract state using Borsh serialization, old table records may fail to deserialize after schema changes.

An owner-only development reset helper is included:

```bash
near contract call-function as-transaction $ADMIN dev_reset_tables json-args '{}' prepaid-gas '30.0 Tgas' attached-deposit '0 NEAR' sign-as $ADMIN network-config testnet sign-with-keychain send
```

Verify reset:

```bash
near contract call-function as-read-only $ADMIN get_open_tables json-args '{}' network-config testnet now
```

Expected output:

```json
[]
```

`dev_reset_tables` is not part of normal gameplay. It is included only as a testnet/development helper.

---

# 4. Suggested Manual Demo / Testing Steps

Use two browser sessions:

```text
Browser 1: first testnet account, for example account1.testnet
Browser 2: second testnet account, for example account2.testnet
```

A private/incognito window or separate browser profile is recommended for the second account.

## Step 1 — Create a Table

In Browser 1, connect Meteor Wallet using the first account.

Create a table:

```text
Buy-in: 1
Extra storage deposit: 0.1
```

Expected:

```text
Table appears in Open Tables.
Table status is WaitingForPlayers.
```

## Step 2 — Join the Table

In Browser 2, connect Meteor Wallet using the second account.

Join the table:

```text
Table ID: 0
Buy-in: 1
Storage deposit: 0.1
```

Expected:

```text
Table becomes Active.
Stage is PreFlop.
Pot starts with blinds.
Each player sees only their own cards.
```

For 1 NEAR buy-in, expected starting values are approximately:

```text
Pot: 0.3 NEAR
Small blind player balance: 0.9 NEAR
Big blind player balance: 0.8 NEAR
```

## Step 3 — Test Betting Flow

At PreFlop, the small blind cannot check immediately because they are facing the big blind.

Expected flow:

```text
Small blind: Call
Big blind: Check
```

Expected:

```text
Flop is automatically dealt.
Community cards show 3 cards.
```

Then:

```text
Player 1: Check
Player 2: Check
```

Expected:

```text
Turn is automatically dealt.
Community cards show 4 cards.
```

Then:

```text
Player 1: Check
Player 2: Check
```

Expected:

```text
River is automatically dealt.
Community cards show 5 cards.
```

Then:

```text
Player 1: Check
Player 2: Check
```

Expected:

```text
Showdown evaluation runs automatically.
Table becomes Finished.
Pot becomes 0.
Winner or split pot is shown.
Both players' cards are revealed.
```

## Step 4 — Test Raise and Call

Create and join a fresh table, or use a new round.

Example flow:

```text
Small blind: Raise
Big blind: Call
```

Expected:

```text
Raise amount is deducted from the raiser's internal balance.
Pot increases.
Call matches the required amount or the player's remaining balance, whichever is lower.
After both players settle the betting round, the stage progresses automatically.
```

## Step 5 — Test Fold

Create and join a fresh table.

Current player selects:

```text
Fold
```

Expected:

```text
Opponent wins the pot.
Table becomes Finished.
Pot becomes 0.
Round result is shown.
Both players' cards are revealed.
```

## Step 6 — Test Play Next Round

After a table is Finished:

```text
Player 1 clicks Play Next Round.
Player 2 clicks Play Next Round.
```

Expected:

```text
Table becomes Active again.
Round number increases.
New cards are dealt.
Community cards are cleared.
Blinds rotate.
Pot starts with new blinds.
```

## Step 7 — Test Timeout Refund

Create and join a fresh table. Wait more than 10 seconds after the last action.

Click:

```text
Claim Timeout Refund
```

Expected:

```text
Table becomes Cancelled.
Pot is refunded into internal balances.
Players can withdraw.
```

## Step 8 — Test Withdraw

After a table is either Finished or Cancelled, click:

```text
Withdraw
```

Expected:

```text
The contract sends an asynchronous transfer.
The callback validates whether the transfer succeeded.
If successful, the pending withdrawal is cleared.
If failed, the internal balance is restored.
```

---

# 5. Known Limitations

* The opponent browser does not update automatically after every action. After each step, the other browser may need to click Refresh or reload the table manually.
* The frontend does not currently use live polling, WebSocket updates, or an indexer.
* Card privacy is view-level only. The frontend and public view methods hide opponent hole cards while the game is active, but the underlying on-chain storage is still public.
* The poker rules are simplified for a 1v1 proof-of-concept.
* Side pots and complete all-in edge cases are simplified.
* The project uses `env::random_seed()` for proof-of-concept randomness.
* `dev_reset_tables` is included as an owner-only testnet development helper for clearing incompatible table state after schema changes.
* The UI may show approximate NEAR formatting because balances are stored in yoctoNEAR.
* The project supports same-table next rounds for two players, but it does not currently support larger multiplayer tables.
