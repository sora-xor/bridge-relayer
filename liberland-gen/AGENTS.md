liberland-gen — Subxt Runtime Bindings (Liberland)

Purpose
- Static Subxt bindings for the Liberland chain runtime used by federated relays.

Refresh Metadata
- Install Subxt CLI: `cargo install subxt-cli`
- Export metadata from node:
  - `subxt metadata --url ws://<liberland-node> -f bytes > bytes/metadata.scale`
- Rebuild: `cargo build -p liberland-gen && cargo build -p relayer`

Notes
- Access types under `liberland_gen::runtime`.
- Used by CLI under `bridge/relay/liberland` and related registration flows.

