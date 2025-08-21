substrate-gen — Subxt Runtime Bindings (SORA)

Purpose
- Provides static Subxt bindings to the SORA runtime for typed storage and extrinsics used by the relayer.

How It Works
- `src/lib.rs` uses `#[subxt::subxt(runtime_metadata_path = "bytes/metadata.scale")]`.
- The bundled `bytes/metadata.scale` is generated from a live chain and committed for reproducible builds.
- Several upstream types are substituted to avoid bounded types and match local crates.

Refreshing Metadata
- Install Subxt CLI (if not already): `cargo install subxt-cli`
- Export current metadata from a node:
  - `subxt metadata --url ws://<sora-node> -f bytes > bytes/metadata.scale`
- Rebuild: `cargo build -p substrate-gen && cargo build -p relayer`

Tips
- Use `substrate_gen::runtime` to access typed storage (`runtime::storage()...`) and extrinsics (`runtime::tx()...`).
- See `relayer/src/substrate/mod.rs` for helper functions that wrap Subxt and logging.

