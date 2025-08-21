# ethereum-gen — Agents Guide

Purpose: Type-safe EVM contract bindings via ethers-rs Abigen. Consumed by `relayer` for EVM-side interactions (sending messages, reading events, channel/app configuration).

## Contents

- src/lib.rs: Abigen definitions using `ethers::contract::abigen!`.
- abi/: JSON ABIs used for code generation.
  - `ChannelHandler.json`
  - `FAApp.json`
  - `TestToken.json`

## Usage

- Import generated types in the relayer (e.g., `ethereum_gen::FAApp`), then instantiate contracts with an ethers `Provider`/`Signer`.
- Events are derived with `serde::{Deserialize, Serialize}` for easy decoding and logging.

## Update/Extend

- To add/replace a contract, put its ABI under `abi/`, then add a clause to the `abigen!` macro in `src/lib.rs`.
- Rebuild the workspace; Abigen generates Rust types and methods at compile time.

