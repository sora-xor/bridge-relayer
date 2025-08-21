# Bridge Relayer — Agents Guide

Note: Check `status.md` for current feature status and open items. When you change behavior or add flows, update `status.md` accordingly so future devs have an accurate picture. For planned work and prioritized tasks, see `roadmap.md` and update it after completing work.

This workspace contains a Rust implementation of a multi-chain bridge relayer used in the SORA ecosystem. It orchestrates message and proof relaying between Substrate-based chains (SORA mainnet, a parachain, Liberland) and external networks like EVM chains and TON.

## Workspace Map

- relayer: CLI binary and core relay logic. Talks to all chains.
- substrate-gen: Subxt-generated client/types for the SORA (framenode) runtime.
- parachain-gen: Subxt-generated client/types for a SORA parachain runtime.
- liberland-gen: Subxt-generated client/types for Liberland runtime.
- ethereum-gen: Ethers-rs Abigen wrappers for EVM contracts used by the bridge.

## High-Level Flow

- Substrate → external: Subscribe to finalized commitments (BEEFY/MMR), build proofs for messages (bridge pallets), then submit to target network (EVM/TON/etc.).
- External → Substrate: Read/log events on external chain, convert to message format, submit extrinsics to the bridge pallets on SORA/parachain/Liberland.
- Finality/validation: Uses Substrate BEEFY/MMR data and bridge-specific proofs (see `substrate-gen`, `relay/*`).

## Key Entry Points

- relayer/src/main.rs: CLI bootstrap; logging and global options.
- relayer/src/cli: Clap-based commands for register/relay/transfer across networks.
- relayer/src/relay: Core relaying flows (BEEFY syncer, EVM/TON/parachain message relays).
- relayer/src/substrate: Typed Substrate clients and helpers (events, storage, MMR/BEEFY RPC, assets RPC).
- relayer/src/ethereum: EVM client wrappers (HTTP/WS) + signing and gas metrics.
- relayer/src/ton: TON wallet/contracts bindings used by CLI/relay.
- substrate-gen|parachain-gen|liberland-gen: Subxt modules and type substitutions using chain metadata.
- ethereum-gen: Abigen bindings for `ChannelHandler`, `FAApp`, `TestToken` contracts.

## Build

- Prereqs: Rust toolchain (see `rust-toolchain.toml`).
- Commands:
  - git submodule update --init --recursive
  - cargo b -r

## Run Examples

- Federated SORA → Liberland relay:
  - RUST_LOG=bridge_relayer=debug,info ./target/release/bridge-relayer --liberland-url ws://{LIBERLAND} --liberland-key {URI} --substrate-url ws://{SORA} --substrate-key {URI} bridge relay sora liberland trusted --signer {SEED}
- Federated Liberland → SORA relay:
  - RUST_LOG=bridge_relayer=debug,info ./target/release/bridge-relayer --liberland-url ws://{LIBERLAND} --liberland-key {URI} --substrate-url ws://{SORA} --substrate-key {URI} bridge relay liberland sora trusted --signer {SEED}

See `README.md` for exact commands and examples.

## Useful Concepts

- BEEFY/MMR: Finality and Merkle Mountain Range proofs (sp-beefy, mmr-rpc) consumed to validate cross-chain messages.
- Subxt clients: Code-generated APIs from on-chain metadata (the `bytes/metadata.scale` files per chain).
- EVM bindings: Generated with `ethers::contract::abigen!` from JSON ABIs in `ethereum-gen/abi`.

## Updating Bindings

- Substrate metadata: Regenerate `bytes/metadata.scale` from a running node, then rebuild (subxt v0.25 format).
- EVM ABIs: Replace JSON in `ethereum-gen/abi` and rebuild; types and events auto-generate via Abigen.

## Add a New Network

- Add a `*-gen` crate with Subxt metadata or EVM ABIs.
- Implement CLI surface in `relayer/src/cli` and relay logic in `relayer/src/relay`.
- Integrate typed clients under `relayer/src/substrate` or `relayer/src/ethereum` as appropriate.
