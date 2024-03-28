<img alt="SORA logo" src="https://static.tildacdn.com/tild3664-3939-4236-b762-306663333564/sora_small.svg"/>

# Overview

Relayer for Sora2 bridges

### Build

```sh
cargo b -r
```
# Run Relayers

### Run Federated SORA -> Liberland Relayer

```sh
RUST_LOG=bridge_relayer=debug,info ./target/release/bridge-relayer --liberland-url ws://{LIBERLAND_ADDRESS} --liberland-key {KEY_POSTFIX} --substrate-url ws://{SORA_ADDRESS} --substrate-key {KEY_POSTFIX} bridge relay sora liberland trusted --signer {YOUR_SEED}
```

Example:

```sh
RUST_LOG=bridge_relayer=debug,info ./target/release/bridge-relayer --liberland-url ws://localhost:10999 --substrate-url ws://localhost:9944 bridge relay sora liberland trusted --signer "{some seed}"
```

### Run Federated Liberland -> SORA Relayer

```sh
RUST_LOG=bridge_relayer=debug,info ./target/release/bridge-relayer --liberland-url ws://{LIBERLAND_ADDRESS} --liberland-key {KEY_POSTFIX} --substrate-url ws://{SORA_ADDRESS}--substrate-key {KEY_POSTFIX} bridge relay liberland sora trusted --signer {YOUR_SEED}
```

Example:

```sh
RUST_LOG=bridge_relayer=debug,info ./target/release/bridge-relayer --liberland-url ws://localhost:10999 --substrate-url ws://localhost:9944 bridge relay liberland sora trusted --signer "{some seed}"
```