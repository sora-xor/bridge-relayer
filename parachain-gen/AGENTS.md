parachain-gen — Subxt Runtime Bindings (Parachain)

Purpose
- Static Subxt bindings for the target parachain runtime used by the relayer.

Refresh Metadata
- Install Subxt CLI: `cargo install subxt-cli`
- Export metadata from parachain node:
  - `subxt metadata --url ws://<parachain-node> -f bytes > bytes/metadata.scale`
- Rebuild: `cargo build -p parachain-gen && cargo build -p relayer`

Notes
- API surface mirrors `substrate-gen`; access via `parachain_gen::parachain_runtime`.
- Keep metadata in sync with the parachain the relayer targets.

