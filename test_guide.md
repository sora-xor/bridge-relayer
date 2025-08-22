# DevOps Guide — Testing TON ↔ SORA Bridge

This guide walks through an end‑to‑end validation of relaying between TON and SORA using the `bridge-relayer` CLI. It assumes you have a local/dev SORA node, a TON API endpoint, and the TON channel + app contracts deployed.

Note on dependencies: For local development, this relayer uses local path overrides to the sibling `sora2-common` repository to pick up the latest TON types. See `status.md` (Local Development Overrides). For CI/releases, replace these overrides with pinned tags.

## 0) Prerequisites

- Toolchain: Rust as specified by `rust-toolchain.toml`.
- Repos present side by side:
  - `bridge-relayer`: this repo
  - `sora2-common`: sibling (used by local path overrides)
- Build binary:
  - `cd ../bridge-relayer && git submodule update --init --recursive && cargo b -r`
- Endpoints:
  - SORA node (WebSocket): `ws://localhost:9944` (or your endpoint)
  - TON API base URL (compatible with `toner` client) and optional `X-API-Key`
- Keys:
  - Substrate ECDSA seed phrase for sudo and approvals
  - TON wallet secret (sufficient balance for message fees)
- Contracts deployed/funded on TON:
  - Bridge channel contract address
  - Bridge app (e.g., Jetton App) address

## 1) Environment Setup

Export these for convenience (example values shown; adjust for your setup):

```sh
export SORA_URL=ws://localhost:9944
export SORA_SUDO="<SUBSTRATE_SEED_PHRASE>"
export TON_URL=https://<ton-api-host>
export TON_API_KEY=<api-key-or-empty>
export TON_WALLET="<ton-wallet-secret>"
# TON contract addresses (base64 or raw format accepted by CLI)
export TON_CHANNEL=<TON_CHANNEL_ADDR>
export TON_APP=<TON_APP_ADDR>
```

## 2) Register TON App on SORA

Choose one:

- New asset mapping (creates asset in SORA bound to TON app):

```sh
./target/release/bridge-relayer \
  --substrate-url $SORA_URL --substrate-key "$SORA_SUDO" \
  bridge register sora ton app fungible-new \
  --network Mainnet \
  --contract "$TON_APP" \
  --name <ASSET_NAME> --symbol <ASSET_SYMBOL> --precision <DECIMALS>
```

- Existing asset mapping (binds existing SORA asset to TON app):

```sh
./target/release/bridge-relayer \
  --substrate-url $SORA_URL --substrate-key "$SORA_SUDO" \
  bridge register sora ton app fungible-existing \
  --network Mainnet \
  --contract "$TON_APP" \
  --asset-id <SORA_ASSET_ID> --precision <DECIMALS>
```

Verify registration: `jetton_app.app_info()` storage should be set.

## 3) Register TON Channel + Peers on SORA

Register the TON channel contract and bridge peers (ECDSA pubkeys in hex):

```sh
./target/release/bridge-relayer \
  --substrate-url $SORA_URL --substrate-key "$SORA_SUDO" \
  bridge register sora ton channels \
  --network Mainnet \
  --channel "$TON_CHANNEL" \
  --peers <ECDSA_HEX1> --peers <ECDSA_HEX2> [--peers ...]
```

This:
- Registers the channel in `bridge_inbound_channel.ton_channel_addresses`.
- Registers peers in `bridge_data_signer` and initializes `multisig_verifier`.

## 4) Start Relayers

Run both directions in separate terminals.

- TON → SORA:

```sh
RUST_LOG=bridge_relayer=debug,info \
./target/release/bridge-relayer \
  --ton-url $TON_URL --ton-api-key $TON_API_KEY \
  --substrate-url $SORA_URL \
  bridge relay ton sora \
  --signer "$SORA_SUDO"
```

- SORA → TON (attach 0.1 TON per message by default; override with `--value`):

```sh
RUST_LOG=bridge_relayer=debug,info \
./target/release/bridge-relayer \
  --ton-url $TON_URL --ton-api-key $TON_API_KEY \
  --substrate-url $SORA_URL \
  bridge relay sora ton \
  --value 100000000  # nanotons (capped by per-message max_fee)
```

Notes:
- `--no-bounce` disables bounce if your channel requires it.
- Loops use exponential backoff; on transient errors they pause and retry.

## 5) Exercise TON → SORA

Send a TON→SORA transfer through the TON app (helper command):

```sh
./target/release/bridge-relayer \
  --substrate-url $SORA_URL \
  --ton-url $TON_URL --ton-api-key $TON_API_KEY \
  bridge transfer ton sora \
  --account-id <SORA_ACCOUNT_ID> \
  --asset-id <SORA_ASSET_ID> \
  --amount <AMOUNT>
```

Expected:
- TON app emits an OutboundMessage.
- TON→SORA relayer parses it and submits an inbound commitment to SORA.
- SORA inbound channel nonce for TON increases.

## 6) Exercise SORA → TON

Enqueue a TON outbound message from SORA via your app pallet (e.g., Jetton App extrinsic that targets TON). Submit using Polkadot‑JS Apps or a Subxt script.

Expected:
- SORA→TON relayer detects a new TON commitment, builds `SendInboundMessage` cells and submits to the TON channel.
- TON channel `inboundNonce` increases; app‑level state reflects the message.

## 7) Verification

- Logs (relayer): look for lines showing nonces (TON vs SORA), submissions, and TON tx hashes.
- SORA storage (via RPC or a small script):
  - `jetton_app.app_info()` (app registered)
  - `bridge_inbound_channel.ton_channel_addresses(network)` (channel registered)
  - `bridge_inbound_channel.channel_nonces(TON)` (TON→SORA increases)
  - `bridge_outbound_channel.channel_nonces(TON)` (SORA outbound tracking)
- TON side:
  - Use explorer or API get‑method `inboundNonce` on the channel; should increase for SORA→TON.
  - Inspect transactions on channel/app.

## 8) Troubleshooting

- “Bridge app/channel not registered”: re‑run the register commands with correct sudo key; check storages above.
- TON API errors: ensure `--ton-api-key` is valid; wallet balance must cover `--value` per message.
- No SORA outbound messages: verify your app extrinsic enqueues to the outbound channel (watch for MessageAccepted events, outbound nonce increment).
- Backoff: loops pause on errors; logs indicate retry delays (up to ~30s by default).

## 9) Cleanup

- Stop relayers (Ctrl+C).
- Optional: deregister app/channel with corresponding sudo calls if you need to reset.

## Notes on Dependencies

- Local path overrides are in `relayer/Cargo.toml` pointing to `../../sora2-common/pallets/*` to pick up the latest TON commitment types.
- For CI/release: replace with pinned git tags and update `Cargo.lock`. See `status.md` and `roadmap.md` for the “Replace local path dependencies with released tags” task.

