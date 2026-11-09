# Slice AN-S1 — Reproduce Rollback-Materialization eta0 Divergence (repro-first, NO fix)

## 1. Title
A hermetic test that makes the bug MECHANICAL: `materialize_rolled_back_state` replays a block against the
persisted-snapshot **placeholder** epoch_nonce instead of the recovered **eta0**, so a block that
validates on live admit (against eta0) fails rollback-replay VRF. No docker, no partition, no logs. The
first slice of PHASE4-N-AN — proves the failure before AN-S2 fixes it.

## 2. Slice Header
- **Cluster:** PHASE4-N-AN. **Status:** Proposed (authority docs this commit; repro test next).
- **Cluster Exit Criteria Addressed:** CE-AN-1 (the bright-red repro).
- **Primary registry rule:** T-REC-06 (`declared`; AN-S1 adds the failing repro to its `tests`; AN-S2
  flips it `enforced`).

## 4. Intent (invariant impact)
Make T-REC-06's violation mechanical. The live-admit path (`block_validity` with the eta0-overlaid
`chain_dep`) and the rollback-materialize path (`block_validity` with the `nearest_le` snapshot
`chain_dep`) currently DISAGREE on the epoch nonce for the same block: live uses eta0 (Valid), rollback
uses the snapshot placeholder (`VrfCert VerificationFailed`). AN-S1 captures this divergence in a pure
hermetic test — no fix, no production change — so AN-S2's fix has a red→green target.

## 6. Execution Boundary (TCB color)
- **BLUE (under test, unchanged)** — `ade_ledger::block_validity::block_validity` (the VRF check),
  `ade_ledger::rollback::materialize::materialize_rolled_back_state` (CN-STORE-07), the `PraosChainDepState`
  nonce fields. AN-S1 adds NO production code — only a test.
- **Test** — a `#[cfg(test)]` repro in `crates/ade_ledger/src/rollback/materialize.rs` (or a sibling test
  module) using the existing Conway validity corpus (a real block + its eta0) + the existing in-memory
  `SnapshotReader` / `BlockSource` test doubles.

## 7. Invariants Preserved
- All — AN-S1 is a TEST ONLY (no production change). `block_validity`, `materialize`, the snapshot model,
  T-REC-04's live overlay — all untouched.

## 8. Invariants Strengthened
- **T-REC-06** `declared` — AN-S1 populates its `tests` with the named repro (CE-AN-1). The rule stays
  `declared` until AN-S2's fix flips it `enforced` (the repro is currently a documented known-failing
  proof of the bug, NOT a passing enforcement yet).

## 9. Design Summary
- **Reuse the corpus** (the proof discipline: a REAL block whose VRF actually verifies against a known
  eta0). The `ConwayValidityCorpus` (used by `admit_via_block_validity_accepts_corpus_block`,
  `forward_sync::pump` tests, `producer::self_accept` `state_with_eta0`) carries a real block + its
  `epoch_nonce`. The repro:
  1. `eta0 = corpus.epoch_nonce`; `placeholder = [0u8; 32]` (≠ eta0).
  2. `chain_dep_eta0` = `PraosChainDepState` with `epoch_nonce = evolving_nonce = eta0`; `chain_dep_ph`
     with `= placeholder`. Both at the corpus block's parent point.
  3. **Live admit (control):** `block_validity(&ledger, &chain_dep_eta0, &sched, &view, &corpus.block)` ⇒
     `BlockValidityVerdict::Valid` — the block's VRF verifies against eta0.
  4. **Rollback replay (the bug):** a `SnapshotReader` test-double returning `(snapshot_slot, ledger,
     chain_dep_ph)` (the PLACEHOLDER-nonce snapshot, mirroring the persisted snapshot) + a `BlockSource`
     yielding `corpus.block` in `(snapshot_slot, target.slot]`. `materialize_rolled_back_state(target,
     &reader, &source, &sched, &view)` ⇒ `Err(MaterializeError::ReplayFailedAt { error:
     Header(VrfCert(VerificationFailed)), .. })`.
  5. **Assert the divergence:** the placeholder snapshot's `chain_dep.epoch_nonce != eta0`, and the same
     block that was `Valid` under eta0 is `VrfCert`-rejected under the placeholder on the rollback path.
- **Known-failing discipline:** because AN-S1 ships NO fix, the repro asserts the CURRENT (buggy)
  behavior (`expect_err` VrfCert + nonce mismatch). It is a TRUE-passing test of a FALSE invariant — it
  documents the bug mechanically. AN-S2 rewrites it (or adds the green companion) to assert the FIXED
  behavior (Valid + nonce == eta0). Mark it clearly (`// AN-S1 repro: documents the T-REC-06 violation;
  AN-S2 flips the assertion to the fixed behavior`).
- **No production change.** AN-S1 is purely a test. The repro isolates the bug to the `chain_dep` nonce
  source — confirming the AN invariants-sketch root cause WITHOUT docker.

## 11. Replay / Crash / Epoch Validation
N/A for the test slice. The repro IS the replay-equivalence probe: it shows live-admit and
rollback-replay diverge on the nonce (the very thing T-REC-06 forbids).

## 12. Mechanical Acceptance Criteria
- **CE-AN-1** (`ade_ledger`, hermetic): `materialize_replays_against_placeholder_nonce_not_recovered_eta0`
  — asserts (a) `block_validity(corpus.block, chain_dep_eta0)` ⇒ Valid; (b) `materialize_rolled_back_state`
  with the placeholder snapshot ⇒ `ReplayFailedAt { VrfCert }`; (c) `placeholder != eta0`. GREEN as a
  documented repro of the buggy behavior.
- **CE-AN-6 (partial):** `cargo test -p ade_ledger` green (the repro + all existing tests).

## 14. Hard Prohibitions (inherit cluster Forbidden verbatim)
- NO production change in AN-S1 (test only).
- NO fix — do NOT overlay eta0, do NOT change materialize, do NOT weaken VRF. Just reproduce.
- Use a REAL corpus block (VRF actually verifies against eta0) — not a hand-faked VRF.
- Do NOT start AN-S2 until CE-AN-1 is mechanical (the repro committed).
