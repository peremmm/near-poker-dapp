# Trustless Texas Hold'em Poker dApp on NEAR

Track: Track B — Building a Trustless Protocol

This project is a proof-of-concept NEAR dApp for a trustless poker-style game.

The contract will enforce:
- table creation
- buy-in rules
- player order
- non-duplicate card dealing
- turn-based actions
- pot and balance updates
- withdrawal and refund rules

## Structure

```text
/project-root
  /contract
  /frontend
  /docs
  README.md
```

## Setup

Install cargo-near
```bash
cargo install cargo-near
```
version used: **cargo-near-near 0.20.3**

## DO this if Error encountered for cargo build

cd <project_path>

Use **1.88 or newer** for this project
```bash
rustup override set 1.88
rustup target add wasm32-unknown-unknown
```
