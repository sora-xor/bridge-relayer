# Relayer Crate — Agents Guide

The `bridge-relayer` binary orchestrates cross-chain actions via a Clap-based CLI. It connects to Substrate chains (SORA, Parachain, Liberland), EVM chains, and TON, then relays commitments and messages using typed clients and generated bindings.

## Structure

- main.rs: CLI bootstrap, logging, and `Cli::run()`.
- cli/: CLI commands and subcommands grouped by domain.
  - bridge/register: Configure bridge contracts/pallets for EVM/TON/SORA/Parachain/Liberland.
  - bridge/relay: Run relayers for specific directions; supports trusted flows and BEEFY-based flows.
  - bridge/transfer: Token transfer helpers across networks.
  - subscribe_beefy, mint_test_token, copy_liquidity: Utilities.
- relay/: Core relay logic.
  - beefy_syncer.rs: Tracks BEEFY commitments and MMR leaves.
  - evm/: EVM message relay logic.
  - ton/: TON message relay logic.
  - parachain.rs, parachain_messages.rs: Parachain message paths.
  - multisig_messages.rs, justification.rs, messages_subscription.rs: Message plumbing/utilities.
- substrate/: Substrate clients + helpers.
  - traits.rs: `ConfigExt` and chain-specific `*Config` traits (SORA, Parachain, Liberland).
  - types.rs: Common types/aliases and transaction parameter configs.
  - beefy_subscription.rs: BEEFY subscription handling.
- ethereum/: EVM clients + provider.
  - provider.rs: `UniversalClient` (WS/HTTP) for ethers-rs `Provider`.

## Key Clients

- Substrate
  - `UnsignedClient<T: ConfigExt>`: JSON-RPC client with typed access (Subxt) + BEEFY/MMR/Assets RPC.
  - `SignedClient<T>`: Adds signer and extrinsic submission.
  - Type sources: `substrate-gen`, `parachain-gen`, `liberland-gen` provide `runtime` modules and types.
- EVM (ethers-rs)
  - `UnsignedClient` / `SignedClient`: Provider + signer middleware with optional gas metrics writing.
  - ABIs in `ethereum-gen` (ChannelHandler, FAApp, TestToken) power type-safe calls and event decoding.
- TON
  - Minimal bindings used by CLI (wallet versions and contract stubs in `ton/`).

## CLI Overview (selected)

Global flags (most commands):
- `--substrate-url`, `--substrate-key` (or `--substrate-key-file`)
- `--parachain-url`, `--parachain-key` (or file)
- `--liberland-url`, `--liberland-key` (or file)
- `--evm-url`, `--evm-key` (or file)
- `--ton-url`, `--ton-key` (or file), `--ton-api-key`
- `--gas-metrics-path`

Top-level commands (examples):
- `subscribe-beefy`: Subscribe to new commitments.
- `bridge register ...`: Registration/config flows for bridge components across chains.
- `bridge relay ...`: Relaying flows (e.g., `sora liberland trusted`, `sora evm`, `parachain ...`).
- `bridge transfer ...`: Token transfer helpers per network.

See the directory tree under `cli/bridge` for full topology (sora/liberland/evm/ton/parachain variants, including `trusted` and `beefy` submodes).

## Typical Flow (SORA → EVM)

1) Subscribe/track BEEFY commitments and MMR leaves.
2) Build message proofs (bridge pallets, simplified proofs, etc.).
3) Submit proof and message to EVM Channel/FA contracts.
4) Track confirmations/finality as needed.

Other directions (EVM → SORA, SORA ↔ Parachain, SORA ↔ Liberland, TON) follow analogous stages with chain-specific proof formats.

## Extending

- New chain: add a `*-gen` crate if Substrate, or new ABI(s) under `ethereum-gen` if EVM.
- CLI surface: add a new subcommand under `cli/bridge/...` and wire to `relay/*` logic.
- Clients: add chain config in `substrate/traits.rs` and import its `runtime` module.

## Notes

- Logging via `RUST_LOG` (default `info` if unset).
- `SignedClient::save_gas_price` can persist EVM call gas estimates for tracking/costing.

