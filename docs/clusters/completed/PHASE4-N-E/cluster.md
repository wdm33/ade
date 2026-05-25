# Cluster PHASE4-N-E — Wire-level mempool ingress (Tier 1 half)

> **Status:** Planning artifact (non-normative). Strengthens `DC-MEM-01`
> (no new constitutional rule) and introduces `DC-MEM-03` + `DC-MEM-04`
> (derived-Cardano family) on first-slice landing. Produced from
> `docs/planning/phase4-n-e-tier1-invariants.md` and
> `docs/planning/phase4-n-e-tier1-cluster-slice-plan.md`. If this doc
> conflicts with the registry/specs, those win.

---

## Primary invariant

> Mempool acceptance equals ledger/block validity for every tx, regardless of
> ingress source. `admit(state, tx)` is a function of `(state, tx_bytes)`
> alone, reached through a single BLUE chokepoint
> `mempool_ingress(IngressEvent) → (MempoolState, AdmitOutcome)`. The two
> RED ingress sources (N2N tx-submission2, N2C local-tx-submission) funnel
> through a deterministic GREEN canonicalizer into that one BLUE bridge.
> An `IngressEvent`'s `source` variant is evidence/policy/replay metadata
> only — it MUST NOT change the validity verdict. A tx whose `PreservedCbor`
> round-trip fails MUST NOT be admitted.

## Normative anchors

- `docs/ade-invariant-registry.toml` — `DC-MEM-01` (strengthened),
  `DC-MEM-03` + `DC-MEM-04` (new, appended in S1/S2), `CN-MEM-04`
  (constitutional cross-ref).
- Project constitution §3 (derived-Cardano determinism doctrine),
  §2 T-CORE-01 (closed semantic surfaces), §2 T-DET-01 (determinism).
- IDD `~/.claude/methodology/idd.md` Part I §§4 (determinism), §5 (replay),
  §9 (FC/IS partition).

## OQ resolutions (locked — see invariants sketch §OQ resolutions)

- **OQ-1** Include both N2N TxSubmission2 and N2C LocalTxSubmission.
- **OQ-2** Close `DC-MEM-01` here; hold `DC-MEM-02`/`CN-MEM-01`/`CN-MEM-03`
  unless a bound changes admission verdicts.
- **OQ-3** Wrap existing B-track adversarial corpus in synthetic
  `IngressEvent`s; do not alter the corpus.
- **OQ-4** One closed `IngressEvent`; `source` is metadata only.
- **OQ-5** End at admission; outbound propagation is a different
  authority surface (separate cluster).
- **OQ-6** BLUE bridge is single-step; GREEN canonicalizer may batch.

## Grounding (verified at HEAD `52642e5`)

- BLUE `admit` exists: `crates/ade_ledger/src/mempool/admit.rs:78`
  `pub fn admit(mempool: &MempoolState, tx_cbor: &[u8]) -> (MempoolState, AdmitOutcome)`.
  Returns `mempool.clone()` on Invalid (no-false-accept already proven for
  CE-B2-5).
- BLUE `tx_submission2_transition` exists:
  `crates/ade_network/src/tx_submission/transition.rs` (state machine emits
  `InventoryEvent` values; does not touch mempool).
- BLUE `tx_validity` exists: `crates/ade_ledger/src/tx_validity/` (closed
  via PHASE4-B1..B5).
- Existing GREEN mempool sub-module: `crates/ade_ledger/src/mempool/policy.rs`
  (DC-MEM-02 sits here; untouched in this cluster).
- Existing B-track adversarial corpus: `corpus/validity/adversarial/`
  (B1–B5 + B3F).
- Existing `DC-MEM-01` entry: `docs/ade-invariant-registry.toml:817-826`
  (enforced; tests pinned to `admit.rs`; `strengthened_in = ["PHASE4-B2"]`).

## Entry Conditions

- PHASE4-N-A closed (tx-submission2 + local-tx-submission codecs + state
  machines exist and are version-pinned).
- PHASE4-B1..B5 + PHASE4-B3F closed (`tx_validity` no-false-accept already
  proven; `admit` is a thin gate on `tx_validity`).
- PHASE4-N-D closed (chain DB exists; `LedgerState` baseline is reachable).
- Constitution-coverage gate PASSES at HEAD (`bash ci/ci_check_constitution_coverage.sh`).

## Exit Criteria (CI-Verifiable)

- **CE-N-E-1 (closure)** — `IngressEvent`/`IngressSource` is the closed
  sum-type entry; `mempool_ingress` is the only sanctioned production path
  into `admit`. CI gate `ci/ci_check_mempool_ingress_closure.sh` forbids
  `MempoolState.accumulating` mutation outside `admit` and forbids non-test
  callers of `admit` outside `mempool_ingress`. (DC-MEM-03.)
- **CE-N-E-2 (agreement)** — `mempool_ingress` ≡ direct `admit` ≡
  `tx_validity` on the B-track adversarial corpus (byte-identical verdicts
  + accumulating state). Test:
  `ingress_admit_equals_direct_admit_on_b_track_corpus`.
- **CE-N-E-3 (source-invariance)** — Same `(state, tx_bytes)` under
  `IngressSource::N2N` vs `IngressSource::N2C` produces byte-identical
  `(MempoolState, AdmitOutcome)`. Test:
  `ingress_source_does_not_change_admit_verdict` (corpus-wide).
- **CE-N-E-4 (replay)** — Replaying the same ordered `[IngressEvent]`
  against the same `base` produces byte-identical
  `(MempoolState, [AdmitOutcome])`. Includes a multi-peer interleaving
  case proving the GREEN canonicalizer's determinism. Tests:
  `ingress_trace_replay_byte_identical`,
  `multi_peer_interleaving_canonicalizes_identically`. (DC-MEM-04.)
- **CE-N-E-5 (adversarial reuse, no-false-accept)** — Full B1–B5 + B3F
  adversarial corpus wrapped in synthetic `IngressEvent`s reproduces
  exact `TxValidityError` rejection class for every reject case. Test:
  `b_track_adversarial_rejections_preserved_through_ingress` (corpus-wide).
- **CE-N-E-6 (live N2N evidence)** — Outbound-client tx-submission2
  probe against a real `cardano-node` peer: N2N handshake accepted,
  mini-protocol opened, every received message drives the BLUE
  `tx_submission2_transition` state machine without
  `IllegalTransition` / `MalformedMessage`; every codec round-trip
  succeeds. Closure via operator-captured evidence log
  `docs/clusters/PHASE4-N-E/CE-N-E-6_<date>.log` produced by
  `cargo run -p ade_core_interop --bin live_tx_submission_session
  -- --connect …`. Bulk tx_bytes ingestion in this direction is
  opportunistic — the outbound client doesn't naturally pull from
  the peer's mempool, so the load-bearing evidence is wire +
  codec + state machine, not tx delivery. Full tx-delivery
  evidence is deferred to CE-NODE-N2C-LTX in the future
  node-binary cluster (see "Deferred / cross-cluster obligation"
  below).
- **CE-N-E-7 (live N2C evidence)** — **DEFERRED** to the future
  node-binary cluster as cross-cluster obligation
  `CE-NODE-N2C-LTX` (see "Deferred / cross-cluster obligation"
  below). Rationale: an N2C local-tx-submission UDS server requires
  real server ownership by `ade_node`, not an operator-only interop
  binary; building one here would create parallel mini-node
  scaffolding before `ade_node` owns that authority surface, and
  drift-risk a future architecture pass.

## Expected Slice Types

- **N-E-S1** — `IngressEvent` + `mempool_ingress` + closure CI gate. Creates
  `crates/ade_ledger/src/mempool/ingress.rs`, appends `DC-MEM-03` to the
  registry, ships `ci/ci_check_mempool_ingress_closure.sh`. *(CE-N-E-1,
  CE-N-E-3 type-level half)*. **TCB: BLUE + CI.**
- **N-E-S2** — Adversarial corpus reuse + ingress-replay harness. Creates
  `crates/ade_testkit/src/mempool/ingress_replay.rs`, wraps B-track corpus
  in synthetic `IngressEvent`s, appends `DC-MEM-04`, pins `DC-MEM-01`
  strengthening. *(CE-N-E-2, CE-N-E-3 corpus half, CE-N-E-5)*. **TCB:
  GREEN harness over BLUE.**
- **N-E-S3** — Per-peer canonicalizer (deterministic multi-peer ordering).
  Creates `crates/ade_ledger/src/mempool/canonicalize.rs`; tests that two
  distinct interleavings of the same per-peer streams produce identical
  `IngressEvent` sequences. *(CE-N-E-4 multi-peer half)*. **TCB: GREEN.**
- **N-E-S4** — N2N tx-submission2 GREEN bridge in `ade_core_interop` +
  documented operator procedure. Adds
  `crates/ade_core_interop/src/tx_submission.rs` (GREEN — InventoryEvent
  → IngressEvent adapter, per-peer accumulator, `ingest_n2n_events`
  orchestrator). *(CE-N-E-6 mechanical adapter half)*.
  **TCB: GREEN.**
- **N-E-S5** — N2C local-tx-submission GREEN bridge in `ade_core_interop` +
  documented operator procedure. Adds
  `crates/ade_core_interop/src/local_tx_submission.rs` (GREEN — same
  shape as S4 over LocalTxSubmissionEvent). *(adapter mechanical evidence
  via `n2n_and_n2c_bridges_produce_identical_outcomes`; CE-N-E-7 itself
  deferred — see below)*. **TCB: GREEN.**
- **N-E-S6** — Live N2N tx-submission2 session binary. Adds
  `crates/ade_core_interop/src/bin/live_tx_submission_session.rs`
  (RED, modeled on `live_consensus_session`) that drives the BLUE
  `tx_submission2_transition` against a real cardano-node peer over
  a sustained window and writes `CE-N-E-6_<date>.log`. Closure-gate
  `#[ignore]` test asserts the binary builds and starts.
  *(CE-N-E-6 mechanical-binary half; live log requires operator run)*.
  **TCB: RED.**

## TCB Color Map

- **BLUE** — `ade_ledger::mempool::ingress` (new, S1);
  `ade_ledger::mempool::admit` (existing, untouched);
  `ade_ledger::mempool::canonicalize` (new, S3 — sync, deterministic,
  no I/O; classified GREEN by sub-module convention but lives in the
  BLUE crate prefix so the family-level CI keeps it honest).
- **GREEN** — `ade_testkit::mempool::ingress_replay` (new, S2);
  `ade_core_interop::tx_submission` (new, S4 — InventoryEvent →
  IngressEvent adapter + per-peer accumulator + ingest_n2n_events
  orchestrator);
  `ade_core_interop::local_tx_submission` (new, S5 — same shape over
  LocalTxSubmissionEvent).
- **RED** — `ade_core_interop::bin::live_tx_submission_session` (new,
  S6 — sustained-window N2N probe; closure-gate `#[ignore]` test +
  operator `--connect` pass; writes
  `docs/clusters/PHASE4-N-E/CE-N-E-6_<date>.log`).
- **DEFERRED RED** — the live N2C UDS server + operator pass for
  CE-N-E-7 is **NOT** built in this cluster. It moves to the future
  node-binary cluster as cross-cluster obligation
  `CE-NODE-N2C-LTX`; `CE-N-E-7_PROCEDURE.md` is retained as the
  procedure spec for that future closure.

The cluster doc's prior placement of S4/S5 session loops under
`ade_runtime::tx_submission::*` is corrected: the actual home is
`ade_core_interop` (the project's established RED live-interop
crate that already houses the PHASE4-N-B follow-mode bridge).

## Cluster status

**Tier-1 authority closed; CE-N-E-6 live N2N evidence captured; live
N2C evidence deferred to node-binary cluster as CE-NODE-N2C-LTX.**

Mechanical:
  - CE-N-E-1, CE-N-E-2, CE-N-E-3, CE-N-E-4, CE-N-E-5 — CI-green.
  - CE-N-E-6 — adapter mechanical evidence CI-green
    (`tx_submission_ingress`, `local_tx_submission_ingress`,
    `n2n_and_n2c_bridges_produce_identical_outcomes`); live-wire
    evidence captured at
    `docs/clusters/PHASE4-N-E/CE-N-E-6_2026-05-25.log` via a
    sustained-window run of `live_tx_submission_session` against the
    preprod relay: N2N handshake accepted at v15, tx-submission2
    mini-protocol opened, peer-originated `RequestTxIds` decoded and
    drove the BLUE `tx_submission2_transition` (Idle → TxIds…)
    without `IllegalTransition` / `MalformedMessage`, 97s active
    session ending in peer-side connection reset (expected: we held
    the peer's blocking request open without txs to offer).
    `[bridge] tx_bytes=0` — bulk tx ingestion in this direction is
    opportunistic per the honest-scope framing and joins
    CE-NODE-N2C-LTX in the deferral.

Deferred:
  - CE-N-E-7 — see cross-cluster obligation below. The cluster
    documents do NOT claim CE-N-E-7 as closed.

Cluster directory **ready to archive** to `docs/clusters/completed/`
in the next grounding-refresh commit (CE-N-E-6 evidence log
committed; no further code work pending in this cluster).

## Deferred / cross-cluster obligation (CE-NODE-N2C-LTX)

CE-NODE-N2C-LTX is an entry-condition proof obligation on the future
node-binary cluster (`ade_node` becoming a real Cardano node). It
holds the following requirement on that cluster:

> When `ade_node` exposes its N2C local-tx-submission UDS endpoint
> as part of normal node operation, a real `cardano-cli transaction
> submit` to that endpoint MUST produce verdicts byte-identical to
> the N2N bridge submission of the same tx bytes. Closure via
> operator-captured evidence log per the procedure spec in
> `docs/clusters/PHASE4-N-E/CE-N-E-7_PROCEDURE.md`, committed under
> the future node-binary cluster's directory as
> `CE-NODE-N2C-LTX_<date>.log`.

The deferral is not a semantic waiver: bounty / N2C certification
remains blocked until the node-binary cluster discharges this
obligation. The CE-N-E-7 procedure document is retained in place
as the canonical spec for the future closure.

## Forbidden During This Cluster

- Mutating `MempoolState.accumulating` from outside
  `ade_ledger::mempool::admit` (the existing chokepoint).
- Calling `admit` directly from production (non-test) code outside
  `mempool_ingress` — the CI gate forbids this.
- Branching admission logic on `IngressEvent.source` — the variant is
  evidence/policy/replay metadata only; doing so violates N-E-N7.
- Decoding and re-encoding tx body bytes anywhere on the ingress path —
  `PreservedCbor<Tx>` MUST flow end-to-end.
- Wall-clock, randomness, `HashMap`/`HashSet` ordering, or floats anywhere
  in the BLUE bridge or in the GREEN canonicalizer.
- Adding a new adversarial-corpus tx bytes file — S2 REUSES the B-track
  corpus verbatim, wrapped in `IngressEvent` envelopes.
- Touching outbound mempool propagation (separate cluster).
- Touching mempool bounds / shedding policy unless a bound would change
  admission verdicts (in which case escalate before proceeding).

## Declared non-goals

- Outbound tx propagation (Ade serving txs to peers via tx-submission2).
- Mempool eviction policy, byte-size bounds, fee prioritization,
  observability surfaces.
- Tier-5 N-E surfaces (operator metrics, per-tx queue position queries,
  enter/exit reason history).
- Pre-Conway tx ingress (the cluster targets the Conway-and-later authority
  surface; `tx_validity` is Conway-only at HEAD).

## Environment-blocked / operator-action exit

- **CE-N-E-6 and CE-N-E-7 close via operator-captured evidence logs**,
  not CI (the CE-N-B-6 pattern). If the live-evidence logs cannot be
  captured in-session, the cluster lands at "code + harness complete,
  live evidence pending"; S4/S5 still ship the RED driver and the
  operator procedure.

## Follow-ups (NOT regressions — for the next cluster)

- **Outbound tx propagation (Tier 1 or Tier 5).** Ade as a tx source.
  Separate authority surface; flagged in invariants sketch §OQ-5.
- **Mempool bounds / deterministic shedding (Tier 5).** `CN-MEM-01`,
  `CN-MEM-03`, `DC-MEM-02` strengthening. Out of scope here.
- **Multi-peer canonicalizer hardening** — S3 ships the function; richer
  fairness / per-peer fairness policy is a Tier-5 follow-up.
