# Invariant Slice — PHASE4-N-K S8

## Slice Header

**Slice Name:** Replay-equivalence harness for the orchestrator
core under `DeterministicClock`; closes DC-NODE-03.
**Cluster:** PHASE4-N-K
**Status:** In Progress
**CEs addressed:** CE-N-K-4 (DC-NODE-03).
**Registry effects on merge:** DC-NODE-03 → `enforced` with
`code_locus`, `tests`, `ci_script =
"ci/ci_check_clock_seam.sh"`, `strengthened_in = ["PHASE4-N-K"]`.
**Dependencies:** S2 (orchestrator core), S3 (writer), S1
(clock).

---

## Intent

Prove that the orchestrator core under a `DeterministicClock` +
recorded `OrchestratorEvent` corpus produces byte-identical
`(LedgerFingerprint.combined, PraosChainDepState fingerprint,
ChainDb tip)` across two consecutive runs. Pin the orchestrator
core's "no wall-clock / no rand / no tokio::time" property via a
file-tree grep.

---

## Scope

- New corpus directory: `corpus/orchestrator/` containing
  `events_001.cbor` (the recorded event stream — produced by a
  fixture builder in the test harness).
- New integration test:
  `crates/ade_runtime/tests/orchestrator_replay_equivalence.rs`.
- New CI gate: `ci/ci_check_clock_seam.sh`.

---

## Execution Boundary

- **BLUE:** none.
- **GREEN:** unchanged.
- **RED:** none — the harness runs the GREEN core with a
  `DeterministicClock` and an in-memory `ChainDb` /
  `SnapshotStore`.

---

## Invariants Preserved

- DC-NODE-03 — clock injection + replay equivalence.
- T-DET-01 — strengthened across the orchestrator surface.

## Invariants Strengthened or Introduced

- DC-NODE-03 (this slice closes).
- T-DET-01 strengthened (recorded in cluster close).

---

## Design Summary

```rust
#[test]
fn replay_equivalence_under_deterministic_clock_holds() {
    let corpus = OrchestratorReplayCorpus::load("corpus/orchestrator/events_001.cbor");
    let run = || {
        let mut state = bootstrap_with_genesis(corpus.bootstrap_inputs());
        let clock = DeterministicClock::new(corpus.tick_vector());
        let mut chain_db = InMemoryChainDb::new();
        let mut snapshot_store = chain_db.clone();
        for event in corpus.events() {
            let effects = orchestrator::core::step(
                &mut state,
                event.clone(),
                &mut ChainDbWriter::new(&chain_db),
                &corpus.served_snapshot(),
                &corpus.era_schedule(),
                &corpus.ledger_view(),
            ).expect("step");
            apply_effects_to_writer(&mut state, &snapshot_store, &effects);
        }
        StateFingerprint {
            ledger_fp: fingerprint(&state.receive_state.ledger).combined,
            chain_dep_fp: hash32(canonical_bytes(&state.receive_state.chain_dep)),
            chain_tip: chain_db.tip().expect("tip"),
        }
    };
    let a = run();
    let b = run();
    assert_eq!(a, b, "orchestrator replay must be byte-identical");
}
```

Corpus contents (small but real):
- Bootstrap inputs: a genesis triple identical to N-I /
  N-J test fixtures (Conway 576 anchor).
- Tick vector: 10 deterministic ticks, 1000ms apart.
- Event vector: `PeerConnected` (peer A) → 3
  `PeerChainSyncFrame`s (one per cached header) →
  3 `PeerBlockFetchFrame`s (block bodies) →
  `SlotTick` interspersed → `Shutdown`.

`ci_check_clock_seam.sh` greps:
- `crates/ade_runtime/src/orchestrator/` recursively for
  `SystemTime::now`, `Instant::now`, `tokio::time::Instant`,
  `tokio::time::Sleep`, `rand::*`. Allowlists the
  `*_runner.rs` / `*_session.rs` / `*_pump.rs` (RED files,
  S4–S6) under explicit exemption.
- `crates/ade_runtime/src/orchestrator/{core,event,state,mod}.rs`
  must not contain any of the above patterns (strict).

---

## Replay, Crash, and Epoch Validation

- **Replay test:**
  `replay_equivalence_under_deterministic_clock_holds` (above).
- **Crash:** not exercised here (S7 covers shutdown-resume).
- **Epoch boundary:** corpus stays in Conway 576; out of scope
  for this slice.

## §12 Mechanical Acceptance Criteria

- [ ] `replay_equivalence_under_deterministic_clock_holds`
- [ ] `replay_corpus_is_present_and_decodable` —
  `corpus/orchestrator/events_001.cbor` exists and decodes via
  the harness reader without panic.
- [ ] `ci_check_clock_seam.sh` — passes on a clean tree.

---

## §14 Hard Prohibitions

- No `tokio::*` in the harness file other than what the
  reused production code already brings.
- No `unwrap()` / `expect()` in non-test code (the harness is
  test code; expects are fine).
- No regenerating the corpus inside the test (it's a frozen
  fixture; if the corpus must change, the slice that does so
  records a strengthening in the registry).

## §15 Explicit Non-Goals

- No live peer corpus (operator-action work).
- No cross-impl equivalence against cardano-node (Ade snapshot
  format is project-internal; Tier 5).
- No producer-side leadership corpus (CN-CONS-06 live half).

---

## §16 Completion Checklist

- [ ] All §12 items present.
- [ ] CI gate runs in `cargo test` job.
- [ ] Registry DC-NODE-03 flipped to `enforced`.
- [ ] `T-DET-01.strengthened_in += "PHASE4-N-K"` (cluster-close
  step).
