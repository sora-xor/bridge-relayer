ethereum-gen — EVM Contract Bindings

Purpose
- Provides statically generated Ethers.rs bindings for the bridge contracts used by the relayer.

Sources
- ABI JSONs live in the submodule: `ethereum-gen/sora2-federated-bridge-evm-contracts/abi/*.json`.
- Bindings are created via `ethers::contract::abigen!` in `src/lib.rs` at compile time.

Key Contracts
- `InboundChannel`, `OutboundChannel`, `BeefyLightClient`
- `ETHApp`, `ERC20App`, `SidechainApp`, `MigrationApp`
- `Bridge`, `Master`, `TestToken`, `IERC20Metadata`

Usage
- Instantiate a contract with an `ethers::providers::Provider` (unsigned) or a `SignerMiddleware` (signed):
  - `let app = ethereum_gen::ETHApp::new(address, provider.clone());`
- Query events via filters (e.g. `.message_filter()`, `.batch_dispatched_filter()`).
- Send transactions by building a call then `.send().await?`.

Updating ABIs
- Pull latest contract changes:
  - `git submodule update --remote --init --recursive`
- Verify new JSONs under `sora2-federated-bridge-evm-contracts/abi/` then `cargo build`.
- If contract names or function/event signatures change, update call sites in `relayer/` accordingly.

Notes
- Ethers version is 2.x; prefer WS endpoints for event queries.
- Owner-restricted calls (e.g., resets) require a signer with permissions on-chain.

