# bridge-relayer
Relayer for Sora2 bridge

### Build

```sh
cargo b -r
```
# Run Relayers

### Run Federated SORA -> Liberland Relayer

```sh
RUST_LOG=bridge_relayer=debug,info ./target/release/bridge-relayer --liberland-url ws://{LIBERLAND_ADDRESS} --liberland-key "//Relayer1" --substrate-url ws://{SORA_ADDRESS} --substrate-key "//Relayer1" bridge relay sora liberland trusted --signer {YOUR_SEED}
```

### Run Federated Liberland -> SORA Relayer

```sh
RUST_LOG=bridge_relayer=debug,info ./target/release/bridge-relayer --liberland-url ws://{LIBERLAND_ADDRESS} --liberland-key "//Relayer1" --substrate-url ws://{SORA_ADDRESS}--substrate-key "//Relayer1" bridge relay liberland sora trusted --signer {YOUR_SEED}
```
