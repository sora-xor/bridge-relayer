#!/bin/bash

pkill bridge-relayer

RUST_LOG=info,bridge-relayer=debug ./target/release/bridge-relayer bridge transfer evm sora \
    --substrate-url ws://localhost:9944 \
    --evm-url http://localhost:8545 \
    --evm-key 5de4111afa1a4b94908f83103eb1f1706367c2e68ca870fc3fb9a804cdab365a \
    --asset-id 0x0200070000000000000000000000000000000000000000000000000000000000 \
    --account-id cnUiNGP9GodVEwZQwtfsWQg8HoYbt4CStUWEf9AQf53Lv7nRs \
    --amount 1000000000000000000


