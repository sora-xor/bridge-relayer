<img alt="SORA logo" src="https://static.tildacdn.com/tild3664-3939-4236-b762-306663333564/sora_small.svg"/>

# Overview

Relayer for Sora2 bridges

### Build

There are two supported build modes:

- Development (local overrides): Use a sibling checkout of `sora2-common` for rapid iteration.
- Release/CI (pinned tags): Use pinned tags/revisions from `sora2-common` for reproducible builds.

#### Dev build (local path overrides)

1) Clone `sora2-common` next to this repo:

```sh
cd .. && git clone https://github.com/sora-xor/sora2-common.git && cd -
```

2) Ensure the root `Cargo.toml` contains `[patch."https://github.com/sora-xor/sora2-common.git"]` entries pointing to relative paths (already configured):

```
[patch."https://github.com/sora-xor/sora2-common.git"]
beefy-light-client = { path = "../sora2-common/pallets/beefy-light-client" }
leaf-provider-rpc = { path = "../sora2-common/pallets/leaf-provider/rpc" }
leaf-provider-runtime-api = { path = "../sora2-common/pallets/leaf-provider/runtime-api" }
bridge-common = { path = "../sora2-common/pallets/bridge-common" }
bridge-types = { path = "../sora2-common/pallets/types" }
```

And `relayer/Cargo.toml` references the same crates via relative paths (already configured).

3) Build:

```sh
git submodule update --init --recursive
cargo build -p bridge-relayer -r
```

Notes:
- Do not mix the above path overrides with direct `git = "https://github.com/sora-xor/sora2-common.git"` dependencies for the same crates; Cargo will error. In dev, prefer path overrides.

#### Release/CI build (make it permanent)

Pin all `sora2-common` crates to a released tag or a specific revision. Recommended approach:

1) Remove the root path override block for `sora2-common` in `Cargo.toml`.

2) In each crate that depends on `sora2-common` (search for `sora2-common.git`), pin to the same tag or rev:

Files typically include:
- `relayer/Cargo.toml`
- `substrate-gen/Cargo.toml`
- `parachain-gen/Cargo.toml`
- `liberland-gen/Cargo.toml`

Example pin using a tag:

```
beefy-light-client = { git = "https://github.com/sora-xor/sora2-common.git", tag = "X.Y.Z" }
bridge-common      = { git = "https://github.com/sora-xor/sora2-common.git", tag = "X.Y.Z" }
bridge-types       = { git = "https://github.com/sora-xor/sora2-common.git", tag = "X.Y.Z" }
leaf-provider-rpc  = { git = "https://github.com/sora-xor/sora2-common.git", tag = "X.Y.Z" }
```

Or using a specific commit:

```
beefy-light-client = { git = "https://github.com/sora-xor/sora2-common.git", rev = "<commit>" }
...
```

3) Refresh the lockfile and build:

```sh
cargo update -p beefy-light-client -p bridge-common -p bridge-types -p leaf-provider-rpc
cargo build -p bridge-relayer -r
```

4) Commit the `Cargo.toml` changes and the updated `Cargo.lock` so CI and downstream users get a reproducible build.

Common pitfalls:
- Mixing a `[patch."https://github.com/sora-xor/sora2-common.git"]` block with direct `git` dependencies to the same source will fail with: “patch … points to the same source”. For release builds, use only `git` with `tag`/`rev` and remove the patch block entirely.
- Ensure ALL `sora2-common` crate references across the workspace are pinned to the same tag/rev, otherwise Cargo may pick mixed versions.

## Troubleshooting

- Digest match failures in message subscription
  - Symptom: relayer logs "Expected digest for commitment not found" in parachain/substrate message subscription path.
  - Context: As of this revision, digest matching requires BOTH the network id and the commitment hash to match the auxiliary digest item (previously either value matching was accepted). If you recently updated nodes or pallets and now see this, ensure the block contains the correct digest item for the target network and the commitment hash you’re querying.
  - Where: `relayer/src/relay/messages_subscription.rs::load_digest`
# Run Federated Relayers

Before running the bridge the accounts for relayers must be generated, for example

```bash
./substrate-node key generate --scheme ecdsa 

Secret phrase:       {secret phrase}
  Network ID:        substrate
  Secret seed:       0x43a6d7abf11a9aa47fe75c50ed6f0788bf8ac04d9dbe4b78227d80fcc6403e76
  Public key (hex):  0x02c7082c578a1b2c59acb577a4fb61bef37770643b303f32c92466c1dfdb96f676
  Account ID:        0xcbeaf95001a3290d8bd457ce9b59565979abeccdb129dd40c7b7a483725e474c
  Public key (SS58): KW7RchUxKMku8BsQMecQNtAxFxpQ9pxBue7Sjf28WpyiAFQjR
  SS58 Address:      5Gg5Ny9g1npyCyehdqRydYrPBBtX3iUo8Bq91YQ3GP4o4Xj3
```

Save the secret phrases. Then Public Keys should be used to initialise the BridgeDataSighner and MultisigVerifier pallets

### Run Federated SORA -> Liberland Relayer

```sh
RUST_LOG=bridge_relayer=debug,info ./target/release/bridge-relayer --liberland-url ws://{LIBERLAND_ADDRESS} --liberland-key {KEY_POSTFIX} --substrate-url ws://{SORA_ADDRESS} --substrate-key {KEY_POSTFIX} bridge relay sora liberland trusted --signer {YOUR_SEED}
```

Example:

```sh
RUST_LOG=bridge_relayer=debug,info ./target/release/bridge-relayer --liberland-url ws://localhost:10999 --substrate-url ws://localhost:9944 bridge relay sora liberland trusted --signer "{secret phrase}"
```

### Run Federated Liberland -> SORA Relayer

```sh
RUST_LOG=bridge_relayer=debug,info ./target/release/bridge-relayer --liberland-url ws://{LIBERLAND_ADDRESS} --liberland-key {KEY_POSTFIX} --substrate-url ws://{SORA_ADDRESS}--substrate-key {KEY_POSTFIX} bridge relay liberland sora trusted --signer {YOUR_SEED}
```

Example:

```sh
RUST_LOG=bridge_relayer=debug,info ./target/release/bridge-relayer --liberland-url ws://localhost:10999 --substrate-url ws://localhost:9944 bridge relay liberland sora trusted --signer "{secret phrase}"
```
