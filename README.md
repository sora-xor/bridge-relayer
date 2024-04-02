<img alt="SORA logo" src="https://static.tildacdn.com/tild3664-3939-4236-b762-306663333564/sora_small.svg"/>

# Overview

Relayer for Sora2 bridges

### Build

```sh
git submodule update --init --recursive
cargo b -r
```
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