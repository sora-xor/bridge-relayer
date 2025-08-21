TON Bridge — Implementation Roadmap

Goals
- Deliver a production-grade, bi-directional bridge between SORA and TON.
- Provide an MVP (trusted/federated) quickly, then evolve toward a light-client-based design where feasible.
- Reuse existing relayer patterns (channels, commitments) and operational practices.

Assumptions
- This repo hosts the relayer and CLI. Chain pallets and TON contracts live in sibling repos.
- We can choose a trusted MVP while the light-client path is researched and validated.

Milestones (Priority • Outcome)
1) P0 — Architecture & Design (1–2 weeks)
   - Decide trust model(s): MVP trusted multisig; longer-term light-client verification.
   - Define message formats, asset mapping (Jetton ↔ SORA asset), fee model, finality rules, reorg handling.
   - Deliverables: ADR documents, API/interface spec, test plan.

2) P0 — TON Client + Watchers (2–3 weeks)
   - Evaluate and select client stack: tonlibjson (C API), lite-client (ADNL), or a maintained Rust SDK.
   - Implement `ton-client` module/crate: connect, fetch blocks/transactions, send messages, query state.
   - Implement BOC/cell parsing and message decoding utilities (or adopt a library) for:
     - Jetton transfers (wallet, master)
     - Bridge channel contracts (to be specified)
   - Deliverables: Rust client with retries, pagination, and basic unit tests on BOC parsing.

3) P0 — TON Bridge Contracts (2–4 weeks)
   - Specify inbound/outbound channel contracts and app contracts (Jetton/native TON) in FunC/Tact.
   - Implement nonce tracking, batching, replay protection, admin/owner roles, and fee escrow.
   - Provide scripts for deployment and configuration per network (testnet/mainnet).
   - Deliverables: Contracts, test suite (local VM + integration), deployment artifacts.

4) P0 — SORA Pallet Extensions (parallel, external repo)
   - Extend `bridge_types` with TON network identifiers and message types.
   - Add SORA pallets to accept trusted TON messages (multisig verification) and (later) verify TON proofs.
   - Deliverables: New/extended pallets, runtime integration, metadata updates.

5) P0 — MVP Trusted Relayer (2–3 weeks)
   - CLI: `bridge register ton ...` and `bridge relay ton ...` (trusted), mirroring existing federated flows.
   - Implement `relayer/src/ton/*`:
     - `client.rs`: TON client wrapper and codecs.
     - `messages.rs`: Watch TON channel events → submit to SORA; watch SORA commitments → submit to TON channel.
     - `mod.rs`: error types, config loading, helpers.
   - Integrate with existing multisig/federated relay builder patterns.
   - Deliverables: End-to-end transfers (TON↔SORA) in a testnet environment with a small signer set.

6) P1 — Ops, Safety, and UX (1–2 weeks)
   - Confirmation/finality parameters and reorg (masterchain/workchain) handling in watchers.
   - Idempotence/dedup (nonce tracking, cache), backfill on relayer restart.
   - Observability: structured logs, metrics, health endpoints; gas/fee metrics.
   - CLI polish: config files, multiple endpoints, dry-run, rate limiting.
   - Deliverables: Ops-ready relayer with runbooks and dashboards.

7) P2 — Light-Client Track (R&D 4–8+ weeks)
   - Research TON proof system suitable for on-chain verification on SORA (masterchain signatures, config proofs, shard proofs).
   - Implement TON light client pallet (SORA):
     - Verify masterchain headers and validator set updates.
     - Verify inclusion proofs for workchain transactions or channel contract state changes.
   - Implement proof builder in relayer: extract BOCs and assemble minimal proofs for on-chain verification.
   - Stretch: SORA light client on TON, if required for symmetric trust.
   - Deliverables: Pallet(s), proof builder, end-to-end demo with on-chain verification.

8) P1 — Asset & App Layer (Jettons) (1–2 weeks)
   - Map Jetton metadata/precision to SORA assets; registration flows.
   - Approvals/locks/mints/burns on both sides; fee vaulting.
   - Deliverables: CLI flows to register and transfer Jettons/native TON ↔ SORA assets.

9) P1 — Testing & QA (ongoing)
   - Local integration test environment (docker-compose): SORA node + TON localnet.
   - End-to-end tests: headers, messages, asset transfers, replay protection.
   - Fault injection: reorgs, node failover, rate limiting, partial failures.
   - Deliverables: Automated smoke tests, reproducible local test setup.

10) P2 — Hardening & Release
   - Security review of contracts and pallets; formalization of threat model.
   - Performance tuning (batch sizing, parallelism); resource footprint caps.
   - Packaging: container images, Helm chart, release documentation.
   - Deliverables: Production release candidate and runbooks.

Task Breakdown (Repo-scoped)
- P0: Introduce TON modules
  - [ ] `relayer/src/ton/mod.rs` with error and config types
  - [ ] `relayer/src/ton/client.rs` (TON RPC/SDK wrapper) + retry policy
  - [ ] `relayer/src/ton/codec.rs` (BOC/cell helpers) or integrate external crate
  - [ ] `relayer/src/ton/messages.rs` (watchers and submitters)
  - [ ] `relayer/src/cli/bridge/register/ton/*` (registration flows)
  - [ ] `relayer/src/cli/bridge/relay/ton/*` (trusted relay flows)
  - [ ] `ton-gen/` (optional) if contract bindings/codegen is used
  - [ ] Docs: `ton/AGENTS.md`, CLI examples, config templates

- P1: Observability & UX
  - [ ] Structured logging fields for TON specifics (workchain, shard, lt, hash)
  - [ ] Prometheus metrics (processed blocks, lag, messages sent, failures)
  - [ ] Health/ready endpoints or log-based health with liveness probes
  - [ ] Config files and env var support

- P2: Light client integration hooks
  - [ ] Add proof ingestion path in relayer when SORA pallet becomes available
  - [ ] Map proof artifacts (BOC segments) to SCALE types in `bridge_types`

Dependencies & External Work
- TON contracts (FunC/Tact) with tests and deployment.
- SORA pallets and `bridge_types` extension for TON network IDs and proofs.
- TON client library decision and packaging (static/dynamic linking or pure Rust).

Acceptance Criteria (MVP Trusted)
- Register and run: `bridge register ton ...` and `bridge relay ton ...` complete without manual steps.
- Successful end-to-end asset transfer (Jetton/native TON ↔ SORA asset) with nonce monotonicity.
- Relayer recovers from restarts without message loss or duplication.
- Observability sufficient for on-call operations.

Risks
- TON proof-verification complexity and evolving ecosystem libraries.
- Contract ABI/protocol drift; coordination across repos.
- Availability and reliability of public TON RPC endpoints (plan for redundancy).

