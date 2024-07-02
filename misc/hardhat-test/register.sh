#!/bin/bash

pkill bridge-relayer

./target/release/bridge-relayer bridge register evm initialize-channels \
    --channel-address 0x5FbDB2315678afecb367f032d93F642f64180aa3 \
    --evm-url http://localhost:8545 \
    --evm-key ac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80 \
    --peers 0x029b310f045b1230ea723174e40fefe35cc4f514e8b25eae1d08b396664002fe02 \
    --peers 0x03d9bb1dd9e05d49046c5140478b8770bcd0bab0f079dc2a8f3a106873422a330a

./target/release/bridge-relayer bridge register sora evm channels \
    --channel-address 0x5FbDB2315678afecb367f032d93F642f64180aa3 \
    --evm-url http://localhost:8545 \
    --evm-key ac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80 \
    --substrate-url ws://localhost:9944 \
    --substrate-key "//Alice" \
    --peers 0x029b310f045b1230ea723174e40fefe35cc4f514e8b25eae1d08b396664002fe02 \
    --peers 0x03d9bb1dd9e05d49046c5140478b8770bcd0bab0f079dc2a8f3a106873422a330a

./target/release/bridge-relayer bridge register sora evm app fungible-predefined \
    --contract 0xDc64a140Aa3E981100a9becA4E685f962f0cF6C9 \
    --evm-url http://localhost:8545 \
    --evm-key ac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80 \
    --substrate-url ws://localhost:9944 \
    --substrate-key "//Alice"

