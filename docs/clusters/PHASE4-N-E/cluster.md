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
- **CE-N-E-6 (live N2N evidence)** — Real `cardano-node` peer submits
  txs to Ade over N2N tx-submission2; verdicts byte-identical to direct
  corpus replay. Closure via operator-captured evidence log
  `docs/clusters/PHASE4-N-E/CE-N-E-6_<date>.log` (CE-N-B-6 pattern).
- **CE-N-E-7 (live N2C evidence)** — Real `cardano-cli transaction submit`
  to Ade over N2C local-tx-submission UDS; verdict matches N2N submission
  of the same bytes. Closure via operator evidence log
  `docs/clusters/PHASE4-N-E/CE-N-E-7_<date>.log`.

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
- **N-E-S4** — N2N tx-submission2 session driver + live evidence. Creates
  `crates/ade_runtime/src/tx_submission/n2n_session.rs`; documents the
  operator procedure for capturing `CE-N-E-6_<date>.log` against a real
  cardano-node peer. *(CE-N-E-6)*. **TCB: RED.**
- **N-E-S5** — N2C local-tx-submission UDS session driver + live evidence.
  Creates `crates/ade_runtime/src/tx_submission/n2c_session.rs`; documents
  the operator procedure for capturing `CE-N-E-7_<date>.log` against real
  cardano-cli. *(CE-N-E-7)*. **TCB: RED.**

## TCB Color Map

- **BLUE** — `ade_ledger::mempool::ingress` (new, S1);
  `ade_ledger::mempool::admit` (existing, untouched).
- **GREEN** — `ade_ledger::mempool::canonicalize` (new, S3);
  `ade_testkit::mempool::ingress_replay` (new, S2).
- **RED** — `ade_runtime::tx_submission::n2n_session` (new, S4);
  `ade_runtime::tx_submission::n2c_session` (new, S5).

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
