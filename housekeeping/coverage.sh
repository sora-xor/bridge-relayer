#!/bin/sh

grcov . --binary-path ./target/debug -s . -t lcov --branch -o ./lcov_report --ignore-not-existing --ignore  "/opt/cargo/**" "target/debug" --log-level="ERROR" --llvm-path='/usr/lib/llvm-14/bin'
find . -type f -name '*.profraw' -delete
