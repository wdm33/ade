# PHASE4-N-Q — Live producer-mode (cluster doc)

> **Status:** Planning. 6-slice cluster wiring `n2n_server` (N-G)
> into a real tokio TCP listener + adding `Mode::Produce` to
> `ade_node` that runs the slot loop and serves forged blocks to
> peers. Closes the engineering side of `CN-CONS-06.open_obligation`
> and `RO-LIVE-01.open_obligation`; the operator-action evidence
> step is folded in as S6.
>
> **Predecessor:** PHASE4-N-P (HEAD `037bad8`).
>
> **Successor:** none planned. Multi-peer / multi-listener /
> mlocked-memory / TLS are deferred to future clusters.
>
> **Inputs:** [`docs/planning/phase4-n-q-invariants.md`](../../planning/phase4-n-q-invariants.md)
> + [`docs/planning/phase4-n-q-cluster-slice-plan.md`](../../planning/phase4-n-q-cluster-slice-plan.md).

---

## §1 Primary invariant

> Ade can act as a Cardano block producer over a real N2N
> network: it accepts inbound peer connections (via a tokio
> TCP listener), completes the N2N handshake, runs the
> producer pipeline on a slot-driven loop, and serves forged
> blocks via the chain-sync + block-fetch server reducers
> shipped in PHASE4-N-G. Concretely:
>
> - The GREEN coordinator is a pure state machine over a
>   closed event/effect surface; it NEVER holds secret-key
>   material.
> - The RED producer shell owns `KesSecret`, `VrfSigningKey`,
>   `ColdSigningKey`, and the on-chain opcert; it handles
>   `RequestForge` effects and emits `ForgeSucceeded` /
>   `ForgeFailed` events back into the coordinator.
> - The RED listener is a thin tokio wrapper; per-peer state is
>   independent (carried from N-G's DC-PROTO-08).
> - Replay equivalence is over canonical slot-tick + forge-result
>   streams (DC-PROD-02), NOT real wall-clock time.
> - Evidence is a closed-vocabulary JSONL log (`ProducerLogEvent`)
>   with no free-form strings, no key material, and no socket
>   addresses in the replayable stream.
>
> After this cluster, an operator can run `ade_node --mode produce`
> against a private cardano-node testnet and capture cardano-node-
> accepts-block evidence — the bounty-facing artifact.

### Why this matters

PHASE4-N-G shipped the producer-side server reducers
(`producer_chain_sync_serve`, `producer_block_fetch_serve`) + a
pure `n2n_server` driver, but explicitly deferred the tokio
socket wiring to "operator-action; not yet wired in-tree". The
existing `live_block_production_session` binary is a stub that
logs `would submit via block-fetch server-side (N-A follow-on)`.

Without N-Q, an operator cannot actually run Ade as a
block-producing node. The bounty's "cardano-node accepts an
Ade-forged block" artifact is operationally unreachable.

N-Q closes this gap:
- **Engineering:** S2..S5 wire the pure components from N-G into
  a real tokio loop with closed effect/event boundaries.
- **Operator:** S6 documents the private-testnet runbook
  (Conway-era genesis, pool registration, opcert issuance,
  evidence capture) so a reader can reproduce the bounty proof.

### Doctrine: GREEN coordinator vs RED shell

A central design choice: **the coordinator is GREEN (pure state
machine), not RED**. Key custody stays in a separate RED
`producer_shell`. The coordinator emits `RequestForge`
effects; the shell signs and returns `ForgeSucceeded` /
`ForgeFailed` events. The coordinator is replayable against a
canonical event stream without ever touching secret bytes —
which preserves the project's true-tier key-custody boundary
and makes the slot/event reducer deterministically auditable.

This split mirrors the N-G `n2n_server` precedent (pure state
machine + thin RED tokio driver) and is the only configuration
that lets us claim DC-PROD-02 (slot-tick + forge-result replay
equivalence) honestly.

## §2 Scope

### In scope

- New module `crates/ade_runtime/src/producer/coordinator.rs`
  (GREEN) — pure state machine; `CoordinatorState` (no secret
  material), `CoordinatorEvent`, `CoordinatorEffect`,
  `CoordinatorError` closed surfaces.
- New module `crates/ade_runtime/src/producer/producer_shell.rs`
  (RED) — holds keys + opcert; handles `RequestForge`.
- New module `crates/ade_runtime/src/producer/producer_log.rs`
  (GREEN) — closed `ProducerLogEvent` enum + JSONL writer.
- New module `crates/ade_runtime/src/network/n2n_listener.rs`
  (RED) — tokio TCP listener; per-peer pump; mirror of
  `n2n_dialer`.
- `crates/ade_node/src/cli.rs` — `Mode::Produce` + flags.
- New module `crates/ade_node/src/produce_mode.rs` (RED) —
  main loop, `tokio::select!` over { slot tick, peer accept,
  peer event, forge result, shutdown }.
- Existing `crates/ade_core_interop/src/bin/live_block_production_session.rs`
  retired (or rewritten as a thin shim).
- Replay test: `crates/ade_runtime/tests/producer_coordinator_replay.rs`.
- Loopback test: `crates/ade_runtime/tests/n2n_listener_loopback.rs`.
- End-to-end smoke: `crates/ade_node/tests/produce_mode_smoke.rs`.
- Operator runbook: `docs/active/cn-cons-06-operator-runbook.md`.
- Operator procedure: `docs/clusters/PHASE4-N-Q/CE-N-Q-OPERATOR_PROCEDURE.md`.
- Optional helper scripts under `scripts/n-q/`.

### Reclassification (no code change)

- `ade_runtime::network::n2n_server` reclassified from RED (N-G
  label) to **GREEN** for N-Q's color-model accounting. The
  module is a pure state-machine driver with no socket I/O; per
  the IDD color model that's GREEN, not RED. This is a
  description-only fix in TRACEABILITY/CODEMAP; N-G's registry
  entries are not rewritten.

### Out of scope (explicit)

- Multi-peer concurrent load (bounty needs 1 peer).
- Multi-listener / multi-port.
- TLS over N2N (¬P-8 continues to defer).
- Mlocked secret memory (future operational cluster).
- Producer-side N2C / LocalTxSubmission (N-E successor).
- Multi-relay topology (future cluster).
- Re-implementing cardano-node (it's the peer we test against).

## §3 Slice index

| Slice | Purpose | Strengthens | Introduces |
|---|---|---|---|
| **S1** | Planning artifacts + 4 registry entries declared | none yet (declarative) | `CN-PROD-01`, `CN-PROD-02`, `DC-PROD-01`, `DC-PROD-02` (all `declared`) |
| **S2** | GREEN `coordinator` + `producer_log` + replay test | `DC-PROD-02`, `CN-PROD-02` (state-shape part) | — |
| **S3** | RED `producer_shell` + forge-effect handler | `CN-PROD-02` (key-custody part), I7 | — |
| **S4** | RED `n2n_listener` + loopback integration test | `CN-PROD-01`, I1 / I2 / I6 | — |
| **S5** | `ade_node Mode::Produce` + end-to-end smoke + retire stub binary | all N-Q invariants (composition) | — |
| **S6** | Operator runbook + private testnet helpers + cluster close + registry strengthenings | closes `CN-CONS-06.open_obligation`, `RO-LIVE-01.open_obligation`; flips `CN-PROD-01/02`, `DC-PROD-01/02` to `enforced` | — |

## §4 Exit criteria (cluster-level MACs)

1. `docs/planning/phase4-n-q-invariants.md` and
   `docs/planning/phase4-n-q-cluster-slice-plan.md` exist.
2. `crates/ade_runtime/src/producer/coordinator.rs` is GREEN
   (no `tokio`, `fs`, `net`, `Instant`, or signing-key types in
   its compilation closure). `CoordinatorState` carries no
   secret-key fields (mechanically grep-asserted by a new CI
   guard).
3. Replay test: fixed event stream → byte-identical broadcast
   effects + log events across two runs.
4. `crates/ade_runtime/src/producer/producer_shell.rs` is RED
   and is the only N-Q-introduced surface that imports
   `KesSecret`/`VrfSigningKey`/`ColdSigningKey`.
   `ci/ci_check_private_key_custody.sh` passes; new Guard 7
   asserts the producer_shell-only constraint.
5. `crates/ade_runtime/src/network/n2n_listener.rs` accepts a
   tokio peer, completes N2N handshake, runs `n2n_server` per
   peer.
6. Loopback integration test: `n2n_dialer` connects to the
   listener, fetches a forged block, asserts byte-equality vs
   `scheduler_step` output.
7. `ade_node --mode produce --listen 127.0.0.1:0 ...` starts
   without panic against a minimal Conway genesis fixture +
   synthetic keys.
8. End-to-end smoke test: ade_node produce + synthetic dialer
   peer → ≥ 1 forged block fetched by peer over 10 slots.
9. Evidence JSONL is closed-vocabulary; every line parses as a
   `ProducerLogEvent`; no socket addresses, no free-form
   strings, no key material.
10. Negative tests pass for: wrong network magic at handshake,
    wrong handshake version, mid-stream malformed envelope,
    `KesPeriodMismatch`, `BroadcastFull`, `SlotDrift`.
11. `crates/ade_core_interop/src/bin/live_block_production_session.rs`
    retired (or replaced with a thin shim calling
    `produce_mode::run_produce_mode`).
12. `docs/active/cn-cons-06-operator-runbook.md` exists; every
    step reproducible.
13. `docs/clusters/PHASE4-N-Q/CE-N-Q-OPERATOR_PROCEDURE.md`
    exists.
14. Registry on S6 close: `CN-PROD-01/02`, `DC-PROD-01/02`
    flipped to `enforced`; `CN-CONS-06`, `RO-LIVE-01`
    `strengthened_in += "PHASE4-N-Q"` + `open_obligation`
    cleared (or narrowed to
    `blocked_until_operator_runbook_executed` if operator pass
    hasn't run).
15. `cargo test --workspace --lib` clean.
16. `/cluster-close PHASE4-N-Q` per the standard discipline.

## §5 Hard prohibitions

- **N9.** No secret-key material in `CoordinatorState` or
  `CoordinatorEvent`. Mechanically enforced by a new CI guard
  in S2: grep `coordinator.rs` for `KesSecret`/`VrfSigningKey`/
  `ColdSigningKey` outside `#[cfg(test)]` blocks → fail.
- **N15.** No socket addresses inside `ProducerLogEvent` (the
  replayable stream). `PeerId` is opaque `u64`; socket addresses
  live in RED operational metadata, surfaced separately.
- **N16.** No real-time replay claim. `DC-PROD-02` is replay
  over canonical slot-tick + forge-result streams only.
- All N-O / N-P key-custody invariants carry forward
  unchanged.
- No new `cardano_crypto::kes` imports in production code
  (carried from N-P S5 + `ci/ci_check_kes_sum_compatibility.sh`).

## §6 Replay obligations preserved + strengthened

- **T-DET-01** — strengthened. The producer pipeline is now
  driven end-to-end with deterministic state transitions; the
  coordinator's replay test is a new fixed-corpus replay anchor.
- **T-KEY-01** — strengthened. The GREEN coordinator
  type-system-enforces the key-custody boundary; the RED shell
  is the only key-touching surface in producer mode.
- **DC-CONS-17 / DC-CONS-18** (N-G served-bytes + header-body
  wire coherence) — strengthened. Now exercised end-to-end via
  a real tokio socket through the loopback integration test.
- **CN-SESS-02** (N2N handshake closure) — strengthened. Now
  exercised in server-role direction.
- **DC-PROTO-08** (server-agency deterministic resolution, N-G)
  — strengthened.

## §7 References

- Predecessor: `037bad8` (PHASE4-N-P close).
- Invariants: [`docs/planning/phase4-n-q-invariants.md`](../../planning/phase4-n-q-invariants.md).
- Cluster plan: [`docs/planning/phase4-n-q-cluster-slice-plan.md`](../../planning/phase4-n-q-cluster-slice-plan.md).
- N-C handoff: `docs/clusters/completed/PHASE4-N-C/` — producer
  pipeline.
- N-G handoff: `docs/clusters/completed/PHASE4-N-G/` — server
  reducers + `n2n_server` pure driver.
- N-L precedent: `crates/ade_runtime/src/network/n2n_dialer.rs`
  — the client-side mirror this cluster's listener follows.
- Doctrine:
  - [[feedback-shell-must-not-overstate-semantic-truth]] — log
    is evidence, not authority.
  - [[feedback-bounded-smoke-slices]] — synthetic-peer test is
    bounded proof; operator runbook is the bounty artifact.
  - [[feedback-fail-closed-validation]] — BroadcastFull,
    SlotDrift, handshake mismatch all fail-closed.
- Bounty: [[project-bounty-requirements]] — "cardano-node
  accepts an Ade-forged block" gate.
