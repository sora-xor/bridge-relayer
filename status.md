# Bridge Relayer — Current Status

This document summarizes what the workspace supports today, what’s partial, and where to extend next. Keep this file updated when adding or changing features.

## Overview

- Purpose: Relay commitments and messages between Substrate-based chains (SORA mainnet, a parachain, Liberland) and external networks (EVM chains, TON).
- Binaries/Crates:
  - `relayer` (binary): CLI + relay logic for EVM/TON/Substrate.
  - `substrate-gen`, `parachain-gen`, `liberland-gen`: Subxt-based runtime bindings from pinned metadata.
  - `ethereum-gen`: Ethers Abigen bindings for EVM contracts used by the bridge.

## Supported Flows

- EVM ↔ Substrate
  - EVM → Substrate: Implemented.
    - File: `relayer/src/relay/evm/evm_messages.rs`
    - Watches EVM channel (`ChannelHandler`) for:
      - `MessageDispatched` → submits `InboundCommitment` to Substrate inbound channel.
      - Periodic base-fee updates.
    - Note: Status report submission from `BatchDispatched` events was removed; current bridge types no longer include that commitment.
    - Tracks and advances by finality; keeps `latest_channel_block` to bound queries.
  - Substrate → EVM: Implemented.
    - File: `relayer/src/relay/evm/sub_messages.rs`
    - Collects outbound commitments + approvals (DataSigner pallet) from Substrate, prepares batch, and calls EVM `ChannelHandler.submit(batch, v, r, s)`.
    - Uses ethers signer middleware; writes optional gas metrics.

- Substrate ↔ Parachain (SORA ↔ SORA parachain)
  - BEEFY signature relaying: Implemented.
    - File: `relayer/src/relay/parachain.rs`
    - Builds random bitfield via RPC, constructs validator proof and simplified MMR proof, submits `submit_signature_commitment` to the receiver.
  - Channel message relaying with MMR proofs: Implemented.
    - File: `relayer/src/relay/parachain_messages.rs`
    - Compares inbound/outbound nonces and submits `submit_messages_commitment` with simplified MMR proof.
  - Multisig-based relay (no BEEFY): Implemented.
    - File: `relayer/src/relay/multisig_messages.rs`
    - Builds digest from auxiliary logs, gathers signatures up to threshold, submits with multisig proof. Used for chains that lack BEEFY.

- TON ↔ Substrate
  - TON → Substrate: Implemented.
    - File: `relayer/src/relay/ton/ton_messages.rs`
    - Reads `outboundNonce` from TON channel, parses outbound msgs from TON transactions, and submits `InboundCommitment` (TON variant) to Substrate.
  - Substrate → TON: Implemented.
    - File: `relayer/src/relay/ton/sub_messages.rs`
    - Reads outbound TON commitments from Substrate, compares nonces with TON inbound channel, builds `SendInboundMessage` cells from payload bytes, and submits via the configured TON wallet with basic backoff. Per-message value is capped by `max_fee`.

## Chain-Specific Support Gaps

- Liberland
  - BEEFY bridge: Not implemented.
    - In `relayer/src/substrate/traits.rs`, Liberland `ReceiverConfig` uses `multisig_verifier::Proof` and stubs out BEEFY-related calls (unimplemented).
  - Bridges to EVM/TON from Liberland: Not implemented (explicit unimplemented markers).

- Parachain
  - Bridges from parachain to EVM/TON: Not implemented (explicit unimplemented markers in `SenderConfig` implementations).

## CLI Surface (high level)

- Global options for endpoints and keys per network: `--substrate-url/key`, `--parachain-url/key`, `--liberland-url/key`, `--evm-url/key`, `--ton-url/key`, `--ton-api-key`, `--gas-metrics-path`.
- Selected commands:
  - `subscribe-beefy`: Stream BEEFY justifications.
  - `bridge register ...`: Register channels/apps/contracts across networks (EVM/TON/SORA/Parachain/Liberland).
  - `bridge relay ...`: Run relayers by direction and trust mode (e.g., sora→evm, evm→sora, sora↔parachain beefy, sora↔parachain multisig, sora↔liberland trusted/multisig, ton→sora).
  - `bridge transfer ...`: Token transfer helpers.
  - Utility commands: `mint-test-token`, `copy-liquidity`.

## Key Implementation Details

- Substrate client (`relayer/src/substrate`)
  - Based on Subxt 0.25 and Jsonrpsee; wraps unsigned/signed clients with helpers for storage, constants, events, block/headers, BEEFY/MMR/Assets RPC.
  - Proofs: MMR leaf generation via `mmr-rpc`, converted to simplified proofs for submission.
  - Data Signer: Approvals fetched from `bridge_data_signer` pallet; threshold logic used by multisig and EVM submission flows.

- EVM client (`relayer/src/ethereum`)
  - `UniversalClient` selects WS/HTTP by URL; ethers `Provider` + signer middleware for transactions.
  - ABIs in `ethereum-gen/abi` generate `ChannelHandler`, `FAApp`, `TestToken` bindings.
  - Optional gas metrics writer.

- TON client (`relayer/src/ton`)
  - Simple HTTP client for public TON API (with optional `X-API-Key`).
  - Wallet abstraction supporting multiple versions; can construct and send BOC (used by CLI tasks today).

## Known TODOs and Unfinished Work

- General: Many modules carry `// TODO #167: fix clippy warnings`.
- Liberland: BEEFY light client path is unimplemented; current flow uses multisig proofs only.
- Parachain: Outbound to EVM/TON is unimplemented in `SenderConfig`.
- Error handling and backoff in relayer loops could be expanded; some unimplemented! guards remain for unsupported directions.

## Local Development Overrides

- For local development, the workspace patches `sora2-common` crates to a sibling checkout (see root `Cargo.toml` [patch] section) so all crates use the same local versions:
  - `beefy-light-client`, `bridge-common`, `bridge-types`, `leaf-provider-rpc` are patched to `../sora2-common/pallets/*`.
- This ensures the relayer builds against the version that contains TON outbound commitment types and related APIs.
- Action: When releasing, replace these path dependencies with pinned tags in this repository’s `Cargo.toml` (or bump tags in sora2-common) and update `Cargo.lock`.

## Resilience/Backoff

- Added simple exponential backoff in loops for:
  - EVM (Substrate → EVM): `relayer/src/relay/evm/sub_messages.rs`
  - TON (TON → Substrate): `relayer/src/relay/ton/ton_messages.rs`
  - TON (Substrate → TON): `relayer/src/relay/ton/sub_messages.rs`
  The loops now avoid tight retry on transient errors and log retry delays.

## How To Update This Document

- After adding/altering a relay flow, registration logic, or supported network direction, update:
  - This `status.md` to reflect the new capabilities/limitations.
  - The relevant `AGENTS.md` sections if the architecture or interfaces changed significantly.
