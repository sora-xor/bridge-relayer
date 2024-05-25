#!/bin/bash

pkill bridge-relayer

label() {
    while read -r l; do
        echo "$1: $l"
    done
}

RUST_LOG=info,bridge-relayer=debug ./target/release/bridge-relayer bridge relay sora evm \
    --evm-url http://localhost:8545 \
    --substrate-url ws://localhost:9944 \
    --signer "//Relayer-1" 2>&1 | label ES1 &

RUST_LOG=info,bridge-relayer=debug ./target/release/bridge-relayer bridge relay sora evm \
    --evm-url http://localhost:8545 \
    --evm-key 59c6995e998f97a5a0044966f0945389dc9e86dae88c7a8412f4603b6b78690d \
    --substrate-url ws://localhost:9944 \
    --signer "//Relayer-2" 2>&1 | label ES2 &

RUST_LOG=info,bridge-relayer=debug ./target/release/bridge-relayer bridge relay evm sora \
    --evm-url http://localhost:8545 \
    --substrate-url ws://localhost:9944 \
    --signer "//Relayer-1" 2>&1 | label SE1 &

RUST_LOG=info,bridge-relayer=debug ./target/release/bridge-relayer bridge relay evm sora \
    --evm-url http://localhost:8545 \
    --substrate-url ws://localhost:9944 \
    --signer "//Relayer-2" 2>&1 | label SE2 &

wait

echo Waited

sleep 100000