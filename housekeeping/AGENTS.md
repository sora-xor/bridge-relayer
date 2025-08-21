housekeeping — Scripts

Scripts
- `build.sh`:
  - `build`: runs `cargo build --release`.
  - `test`: enables coverage flags, skips WASM build, runs `cargo test`.
- `coverage.sh`:
  - Runs `grcov` to produce `lcov_report` from `./target/release` binaries, then cleans `*.profraw`.

Usage
- `./housekeeping/build.sh build`
- `./housekeeping/build.sh test`
- `./housekeeping/coverage.sh` (requires `grcov` and LLVM tools; adjust `--llvm-path` as needed).

CI
- See `.github/workflows/static_analysis.yml` for fmt+clippy checks and SARIF upload.

