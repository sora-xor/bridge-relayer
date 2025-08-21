# liberland-gen — Agents Guide

Purpose: Subxt-generated runtime API and types for the Liberland chain, consumed by the relayer for SORA ↔ Liberland bridging.

## Contents

- bytes/metadata.scale: SCALE metadata for Liberland runtime.
- src/lib.rs:
  - `#[subxt::subxt(runtime_metadata_path = "bytes/metadata.scale")]` defines `liberland_runtime` with calls/events/storage.
  - Type substitutions to bridge/common/sp-* crates used by the pallets.
  - `config::DefaultConfig` implements `subxt::Config` with `SubstrateExtrinsicParams` and `MultiSignature`.

## Usage

- Import `liberland_gen::liberland_runtime` in relayer substrate clients and CLI flows under `bridge/relay/register ... liberland`.

## Update Metadata

- Replace `bytes/metadata.scale` from a Liberland node export (subxt 0.25 format), then rebuild.

