Relayer — Architecture & Dev Notes

Purpose
- Implements the `bridge-relayer` CLI: register bridge components, sync headers/proofs, and relay messages between chains.

Key Modules
- `src/main.rs`: entrypoint; initializes logging and dispatches CLI.
- `src/cli/`: Clap-driven subcommands (bridge register/relay, utilities).
- `src/relay/`: Runtime relayers (Ethereum headers/messages, BEEFY, parachain, Liberland).
- `src/ethereum/`: EVM provider abstraction, Ethash/receipt proofs, header conversion.
- `src/substrate/`: Subxt client wrappers, storage/tx helpers, BEEFY/MMR utilities.

CLI Structure
- `src/cli/mod.rs` defines top-level `Commands`:
  - `Bridge` → `bridge/` (register, relay, transfers)
  - `SubscribeBeefy`, `FetchEthereumHeader`, `MintTestToken`, `CalcDagRoots`, `CopyLiquidity`, `OldBridge`
- Global client args are flattened and shared:
  - `SubstrateClient`, `ParachainClient`, `LiberlandClient`, `EthereumClient`
- Add a new command:
  1) Create a module under `src/cli/` or nested under `bridge/`.
  2) Extend the `Commands` enum and delegate to `cmd.run().await`.

Client Helpers
- Substrate (`src/cli/utils.rs`, `src/substrate/mod.rs`):
  - `get_unsigned_substrate()`/`get_signed_substrate()` to connect via WS.
  - Read storage: `storage_fetch`, `storage_fetch_or_default`, `constant_fetch_or_default`.
  - Submit extrinsics: `submit_extrinsic` (signed), `submit_unsigned_extrinsic` (unsigned).
  - Block helpers: `block_hash`, `header`, `block_number`, `finalized_head`.
  - MMR/BEEFY: `mmr_generate_proof`, `beefy_start_block`, BEEFY subscriptions.
- Ethereum (`src/ethereum/mod.rs`):
  - `UnsignedClient::new(Url)` for reads; `signed(key, gas_metrics)` for TXs.
  - `provider::UniversalClient` supports both WS and HTTP.
  - Gas metrics: `SignedClient::save_gas_price(call, note)` appends to `--gas-metrics-path`.

Relaying Flows
- Ethereum headers: `relay/ethereum.rs` imports headers with Ethash proofs into SORA.
- Ethereum messages: `relay/ethereum_messages.rs` submits outbound/inbound channel logs with receipt proofs.
- Parachain messages: `relay/parachain*.rs` build/verify BEEFY-based proofs and submit messages.
- Liberland flows: federated relays via `relay/multisig_messages.rs` and CLI under `bridge/relay/liberland`.

Proofs & Caches
- Ethash proofs: `src/ethereum/proof_loader.rs` generates per-epoch cache/DAG data.
  - CLI flag `--base-path` sets working dir; cache is `<base>/cache`, data is `<base>/data`.
- Receipt proofs: built per block and cached by `ProofLoader`.

Coding Conventions
- Use `crate::prelude::*` and `cli::prelude::*` to import common types and traits.
- Prefer typed storage/tx accessors from `substrate_gen::runtime` for safety.
- Follow existing module naming and Clap patterns for new commands.

Extending Networks
- Network parameters and IDs come from `bridge_types` and on-chain storage.
- Adding a new EVM network usually requires:
  - Adding/updating on-chain `NetworkConfig`.
  - Registering Ethash and channels via CLI (`bridge register sora evm ...`).
  - Ensuring the EVM contracts expose the expected ABIs used in `ethereum-gen`.

Troubleshooting
- Set `RUST_LOG=bridge_relayer=debug,info` for verbose logs.
- Head-of-line checks: header import waits until best block catches up by `MAX_HEADER_IMPORTS_WITHOUT_CHECK`.
- Watch for “Network is not registered” → register Ethash + channels first.

