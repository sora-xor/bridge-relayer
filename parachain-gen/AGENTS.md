# parachain-gen — Agents Guide

Purpose: Subxt-generated runtime API and types for a SORA-related parachain, consumed by the relayer to interact with parachain bridge pallets and events.

## Contents

- bytes/metadata.scale: SCALE metadata for the parachain runtime.
- src/lib.rs:
  - `#[subxt::subxt(runtime_metadata_path = "bytes/metadata.scale")]` defines `parachain_runtime` with calls/events/storage.
  - Type substitutions wire parachain types to bridge/Beefy/common crates.
  - `config::DefaultConfig` implements `subxt::Config` with `PolkadotExtrinsicParams` and `MultiSignature`.

## Usage

- Import `parachain_gen::parachain_runtime` in relayer substrate clients and CLI flows under `bridge/relay/register ... parachain`.

## Update Metadata

- Replace `bytes/metadata.scale` with a new export from the target parachain (subxt 0.25 format), then rebuild.

