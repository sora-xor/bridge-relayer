#!/bin/bash
set -e
build() {
    cargo b -r
}

test() {
    export RUSTFLAGS="-Cinstrument-coverage"
    export SKIP_WASM_BUILD=1    
    export LLVM_PROFILE_FILE="sora2-%p-%m.profraw"
    cargo test
}