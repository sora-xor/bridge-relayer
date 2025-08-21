Bridge Relayer — Agent Guide

This repo contains a Rust workspace for SORA2 bridging utilities and the main bridge relayer binary. These notes help contributors and AI agents quickly build, run, and extend the relayer.

Overview
- Purpose: Sync headers/proofs and relay messages between SORA, parachains, Liberland, and EVM chains.
- Binary: `bridge-relayer` in `relayer/` with a Clap-powered CLI.
- Contracts/ABIs: Ethers bindings in `ethereum-gen/` (ABI submodule).
- Substrate runtimes: Static Subxt metadata in `*-gen/bytes/metadata.scale`.

Repo Layout
- `relayer/`: Relayer binary, CLI commands, relay logic, Ethereum/Substrate clients.
- `ethereum-gen/`: Ethers rs bindings for EVM contracts (from ABI JSONs).
- `substrate-gen/`, `parachain-gen/`, `liberland-gen/`: Subxt codegen crates for respective runtimes.
- `housekeeping/`: Build/test/coverage helper scripts.
- `.github/workflows/`: Formatting and clippy CI.

Prerequisites
- Rust: stable toolchain (see `rust-toolchain.toml`), plus `wasm32-unknown-unknown` target.
- Git submodules: `git submodule update --init --recursive` (ABI submodule).
- Optional: `protoc` (follows CI environment), `grcov` for coverage.

Build
- Clean build: `cargo build --release`
- Quick update: `cargo b -r` (if you have the alias).

Run Basics
- Logging: set `RUST_LOG`, e.g. `RUST_LOG=bridge_relayer=debug,info`.
- Global flags (picked up by subcommands):
  - `--substrate-url`, `--substrate-key | --substrate-key-file`
  - `--parachain-url`, `--parachain-key | --parachain-key-file`
  - `--liberland-url`, `--liberland-key | --liberland-key-file`
  - `--ethereum-url`, `--ethereum-key | --ethereum-key-file`, `--gas-metrics-path`

Common Workflows
- Register EVM on SORA (sudo required on-chain):
  - Ethash light client: `bridge register sora evm ethash --descendants-until-final <N> [--mainnet|--sepolia|--goerli|--custom <path>]`
  - Channels: `bridge register sora evm channels --inbound-channel <H160> --outbound-channel <H160>`
  - Apps: `bridge register sora evm app (ERC20App|NativeApp|EthAppPredefined|EthAppNew|EthAppExisting) ...`
- Reset EVM contracts (owner key on EVM):
  - BEEFY reset: `bridge register evm beefy --eth-app <H160>`
  - Channels reset: `bridge register evm channels --eth-app <H160>`
- Relay EVM → SORA headers and messages:
  - `bridge relay sora evm --base-path <dir> --substrate-url <ws://...> --substrate-key <seed> --ethereum-url <ws(s)://...> [--disable-message-relay]`
  - `--base-path` stores Ethash DAG/data caches under `<dir>/cache` and `<dir>/data`.
- Parachain relays (BEEFY proofs or trusted):
  - Parachain → SORA: `bridge relay parachain sora beefy|trusted` (set both `--parachain-*` and `--substrate-*`).
  - Parachain ↔ Parachain: see `bridge relay parachain parachain beefy|trusted`.
- Liberland relays (federated): see examples in `README.md`.
- Utilities:
  - Subscribe BEEFY: `subscribe-beefy`
  - Fetch finalized Ethereum header: `fetch-ethereum-header --descendants-until-final <N>`
  - Mint test token, test transfers, copy liquidity: see corresponding CLI subcommands.

Updating Codegen
- EVM ABIs: update submodule `ethereum-gen/sora2-federated-bridge-evm-contracts`, then `cargo build` to pick up ABI changes via `abigen!`.
- Subxt metadata: refresh `bytes/metadata.scale` for each `*-gen` crate from a live node:
  - Example: `subxt metadata --url ws://<sora-node> -f bytes > substrate-gen/bytes/metadata.scale`
  - Parachain: update `parachain-gen/bytes/metadata.scale` from the target parachain.
  - Liberland: update `liberland-gen/bytes/metadata.scale` from the Liberland node.

Housekeeping
- Format: `cargo fmt --all`
- Clippy: `cargo clippy --all-targets --all-features -- -D warnings`
- Build script: `housekeeping/build.sh build`
- Test (if present): `housekeeping/build.sh test`
- Coverage: `housekeeping/coverage.sh` (requires `grcov`/LLVM toolchain installed).

Development Notes
- CLI structure: `relayer/src/cli` contains subcommands; add new commands by extending `Commands` enums and modules.
- Substrate client helpers: see `relayer/src/substrate/mod.rs` for `storage_fetch`, `submit_extrinsic`, MMR proof utilities, and typed storage/tx accessors in `substrate_gen`.
- Ethereum client: use `UnsignedClient` for reads and `SignedClient` for transactions; universal WS/HTTP provider is in `relayer/src/ethereum/provider.rs`.
- Proofs and caches: Ethash proofs and receipt proofs are built in `relayer/src/ethereum/proof_loader.rs` using `--base-path` for cache directories.
- Network config: SORA chain network settings come from `bridge_types::network_config`; adding new networks typically requires updating that upstream crate and the on-chain config.

Gotchas
- Sudo calls: Registration on SORA uses `sudo::sudo` extrinsics; run with an admin key.
- Chain IDs: The EVM node `chainid` must match the selected network config when registering Ethash.
- Endpoints: Use WebSocket URLs for streaming where supported (`ws://` or `wss://`).

Where To Look Next
- See `relayer/AGENTS.md` for CLI architecture and extension points.
- See `substrate-gen/AGENTS.md` and `ethereum-gen/AGENTS.md` for codegen details.

