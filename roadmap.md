# Bridge Relayer — Roadmap (Prioritized)

Use these prompts as starting points. Keep this list up to date as you deliver work. After completing a task, update `status.md` and adjust priorities here.

## P0 — Immediate

- Substrate → TON relay (outbound to TON)
  - Prompt: Implement a relay loop that reads outbound TON commitments from Substrate and sends BOCs to the TON channel.
  - Details: Mirror `evm/sub_messages.rs` but for TON. Fetch Substrate outbound nonce and commitments for `GenericNetworkId::TON`, build TON `SendInboundMessage` or app-specific cells, sign/send via `SignedTonClient` wallet, and advance until nonces match. Add backoff, idempotence, and logging.
  - DoD: `bridge relay sora ton` (or equivalent CLI path) submits outbound batches/messages to TON; `status.md` updated with “implemented”.

- Resilience/backoff for relayer loops
  - Prompt: Add retry/backoff and error classification to EVM, Parachain, Multisig, and TON relayers.
  - Details: Wrap network calls with exponential backoff, avoid tight loops on transient errors, and surface structured logs for failures.
  - DoD: Each loop handles transient RPC/network errors without crashing or spamming; logs indicate retries and next steps.

- Finish docs for CLI subcommands
  - Prompt: Add module-level and function docs across `relayer/src/cli/bridge/**` to explain arguments, flows, and examples.
  - DoD: All public CLI commands show `--help` with clear descriptions; `relayer/AGENTS.md` references key subcommands.

## P1 — Next

- Liberland: Decide BEEFY vs Multisig-only and implement
  - Prompt: Either implement Liberland BEEFY light client path or explicitly remove/guard BEEFY flows and polish multisig.
  - Details: If implementing BEEFY, fill `ReceiverConfig` for Liberland with BEEFY storages and calls; else update CLI and docs to reflect multisig-only.
  - DoD: No `unimplemented!()` for chosen path; `status.md` reflects decision.

- Parachain → EVM/TON support
  - Prompt: Implement `SenderConfig` storage addresses and flows for Parachain to EVM/TON (currently unimplemented!).
  - DoD: Relay commands exist to send Parachain outbound batches to EVM/TON; `status.md` updated.

- Clippy cleanup and lint gating
  - Prompt: Address `// TODO #167: fix clippy warnings` and enable `-D warnings` in CI for modified crates.
  - DoD: Clean clippy on relayer and gen crates (exclude generated code where necessary).

- Config files and profiles
  - Prompt: Add optional TOML config loading for endpoints/keys, plus environment overrides.
  - DoD: Users can run relayers without long CLI lines; docs updated.

- Observability/metrics
  - Prompt: Add optional Prometheus metrics and structured logging (JSON) toggles.
  - DoD: Expose counters (submitted batches, retries, errors) and timings; documented ports/flags.

## P2 — Later

- Integration test harness
  - Prompt: Compose a minimal local EVM (anvil) + SORA dev-node and scripted flows to validate relays E2E.
  - DoD: `make itest` (or script) spins up env and proves a message in both directions.

- Docker and release polish
  - Prompt: Multi-stage Docker, config via envs, and example compose files for dev/test deployments.
  - DoD: Smaller images and example deployment descriptors.

- Gas metrics aggregation and docs
  - Prompt: Aggregate `gas_metrics` into CSV/JSON with summaries; add guidance in docs.
  - DoD: One command to summarize gas estimates; docs reflect usage.

- Metadata/ABI refresh automation
  - Prompt: Add scripts to regenerate Subxt metadata bytes and pull EVM ABIs from sources.
  - DoD: `make refresh-bindings` refreshes metadata and ABIs consistently.

- Security & key management
  - Prompt: Support hardware wallets or keystore files for EVM/Substrate; redact keys in logs.
  - DoD: Optional key providers integrated; secrets never logged.

- Performance and batching
  - Prompt: Optimize batch sizes/parallelism; reduce RPC round-trips where safe.
  - DoD: Measurable reduction in latency or RPC usage; no correctness regressions.

