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

# build func
if [ "$(type -t $1)" = "function" ]; then
    "$1"
else
    echo "Func '$1' is not exists in this workflow. Skipped."
fi