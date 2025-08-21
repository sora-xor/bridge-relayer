Bridge Relayer — Project Status

Summary
- Purpose: Relays headers and messages between SORA, parachains, Liberland, and EVM chains; provides registration utilities for bridge components.
- Tooling: Rust stable, ethers 2.x, Subxt 0.25, Substrate pallets pinned to polkadot-v0.9.38 forks.
- Build: Requires submodules; CI runs fmt+clippy; no integration tests in-repo.

EVM Bridge
- Light Client (Ethash): Implemented. Imports EVM headers into SORA with Ethash proofs.
  - Components: `relayer/src/relay/ethereum.rs`, `relayer/src/ethereum/proof_loader.rs`, `relayer/src/ethereum/ethashproof/*`.
  - Registration: `bridge register sora evm ethash` (sudo). Checks chain ID; uses a finalized block as the anchor.
  - Operation: `bridge relay sora evm --base-path <dir>` runs header import; caches DAG and merkle data under `<base>/cache` and `<base>/data`.
  - Notes: Import loop throttles when SORA best lags finalized; simple LRU for parent continuity; does not include explicit deep reorg handling beyond parent tracking.
- Channel Messages (EVM → SORA): Implemented.
  - Components: `relayer/src/relay/ethereum_messages.rs`.
  - Flow: Reads OutboundChannel `Message` and InboundChannel `BatchDispatched` events; builds receipt proofs; submits to SORA’s inbound channel pallet. Requires channels registered on SORA.
- Channel Messages (SORA → EVM): Implemented.
  - Components: `relayer/src/relay/substrate_messages.rs`.
  - Flow: Tracks SORA outbound nonce; waits for BEEFY leaf finality; submits batches into EVM `InboundChannel::submit` with simplified MMR proof; logs `BatchDispatched` on receipt.
- Registration Utilities (SORA side): Implemented.
  - Ethash light client, EVM channels, and apps (ERC20/Native/EthApp/Migration) under `bridge register sora evm ...` (sudo required).
- Maintenance Utilities (EVM side): Implemented.
  - Owner-only resets for BEEFY and channels under `bridge register evm ...`.
- Transfers: Example send flows exist (`bridge transfer-to-sora`, `bridge transfer-to-ethereum`).
- Known caveats/gaps:
  - ABIs submodule branch is set to `outdated` in `.gitmodules`; verify compatibility with live contracts before use.
  - No automatic re-deploy or migration tooling in this repo; legacy support in `old_bridge`.
  - No unit/integration tests; behavior verified at runtime only.

Parachain Bridge
- BEEFY-based relays between Substrate-like chains (SORA ↔ Parachain, Parachain ↔ Parachain): Implemented.
  - Components: `relayer/src/relay/parachain*.rs` and CLI under `bridge relay parachain ...`.
  - Modes: BEEFY proofs or “trusted” multisig signing for federated scenarios.
  - Requires: MMR/BEEFY RPC access and correct metadata/IDs in `*-gen` crates.

Liberland Bridge (Federated)
- Federated relayer flows are present under `bridge relay liberland ...`.
  - Trusted multisig relay pattern similar to other “trusted” flows.
  - Assumes Liberland metadata is bundled in `liberland-gen/bytes/metadata.scale`.

Old Bridge
- Legacy bridge flows supported under `old_bridge` (assets dump/register, relay, migration).
  - Use for migrations or interacting with legacy contracts; not under active feature development.

TON Bridge (Requested Status)
- Current status: Not implemented in this codebase.
  - No TON-specific modules, clients, CLI, or proofs exist in this repository.
  - No references to TON RPC/SDKs (e.g., lite-client, tonlibjson), TVM cells/BOC parsing, or TON contract ABIs.
  - No on-chain pallets for TON message verification present in the Subxt metadata bindings.
- Implications: The relayer cannot interact with TON. Attempts would require separate development.
- Suggested path forward:
  - Design: Define trust model (trusted multisig vs. light-client-based). For light clients, identify verifiable proof format on SORA and counterpart on TON.
  - TON integration: Choose client layer (lite-client, toncenter, or SDK), map message/event schemas, and implement proof extraction (BOC/cell parsing) if required.
  - SORA pallets: Add/extend pallets to verify TON proofs or accept federated signatures; update `substrate-gen` metadata.
  - E2E wiring: Add `cli/bridge/relay/ton/...` and `cli/bridge/register/ton/...`, with typed bindings and config.
  - Testing: Add integration scenarios (happy path, reorg/rollback equivalents, out-of-order messages).

Quality, CI, and Docs
- Docs: High-level repo docs in `README.md`, developer guides in `AGENTS.md` files; pervasive inline module docs added.
- CI: `fmt` and `clippy` with SARIF upload; no automated tests.
- Lints: Many modules have `#![allow(clippy::all)]` pending cleanup (see TODO #167 comments).
- Codegen: Subxt bindings pinned to pre-generated metadata; must be refreshed when runtimes change.

Open Risks and TODOs
- Substrate version pin: Uses `polkadot-v0.9.38` forks; upgrading to newer Substrate may require non-trivial changes.
- ABIs drift: EVM contracts may evolve faster than the pinned ABI submodule; confirm branch and update before deployments.
- Proof caches: Ethash DAG/cache directories must be provisioned; storage and IO characteristics should be monitored in production.
- Observability: Logging is present; metrics and health endpoints are not included in this repo.
- Tests: No unit/integration tests; consider adding smoke tests for header import and message submission against local devnets.

