# substrate-gen — Agents Guide

Purpose: Subxt-generated runtime API and types for the SORA (framenode) chain, used by `relayer` for typed storage/events/extrinsics. Uses on-chain metadata baked into `bytes/metadata.scale`.

## Contents

- bytes/metadata.scale: SCALE-encoded metadata exported from a running node.
- src/lib.rs:
  - `#[subxt::subxt(runtime_metadata_path = "bytes/metadata.scale")]` defines the `runtime` module and `runtime_types`.
  - Extensive `#[subxt(substitute_type = ...)]` mappings to external bridge types (`bridge_types`, `bridge_common`, `sp_beefy`, `common`, etc.).
  - `config` module exposes a `DefaultConfig` implementing `subxt::Config` with SORA-compatible types and `PolkadotExtrinsicParams`.

## Usage

- Import `substrate_gen::runtime` for calls, events, constants, and storage addresses.
- Import `substrate_gen::runtime::runtime_types` for type aliases (e.g., `framenode_runtime::...`).
- Used via `relayer/src/substrate` clients for JSON-RPC + Subxt operations.

## Update Metadata

- Export fresh metadata from a compatible node (subxt 0.25 format) and replace `bytes/metadata.scale`. Then rebuild.
- Ensure type substitutions in `src/lib.rs` still match upstream crates (bridge/common/sp-*) after updates.

