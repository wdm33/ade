# Invariant Slice — Replay-equivalence over the local-tip-derived chain (DC-NODE-20 ∩ T-REC, S3)

## 2. Slice Header
**Slice Name:** Replay-equivalence over the local-tip-derived post-self-admit chain (PHASE4-N-AH S3)
**Cluster:** PHASE4-N-AH — local selected durable chain forge-base authority; **rung-1, single-producer only**
**Status:** Proposed
**Authority source:** `docs/clusters/PHASE4-N-AH/cluster.md` (§4, CE-AH-4); registry `T-REC-03` + `T-REC-05` (replay family)

**Cluster Exit Criteria Addressed:**
- [ ] **CE-AH-4:** replay-equivalence over the local-tip-derived post-self-admit chain — two clean runs byte-identical (WAL + durable tip + ledger fingerprint + served chain) + kill/warm-start byte-identical + the cert/timing surface absent from the durable bytes. Hermetic; reuses the N-AG `s2_extend_lead` / `served_snapshot` harness; `T-REC-03`/`T-REC-05`.

Exit criteria not listed (CE-AH-1/2/5=S1; CE-AH-3=S2; CE-AH-6=S4; CE-AH-7=close) are out of scope.

**Slice Dependencies:** S1 (`b0fb8817`) — DC-NODE-20 local-tip forge base; S2 (`050237e9`) — DC-NODE-21 cert fully removed from the node.

## 3. Implementation Instruction (AI — INLINE)
**Hermetic + test-only (user-directed).** Add three `#[tokio::test]`/`#[test]` functions to the `node_sync` test module, reusing the existing `s2_extend_lead` / `local_spine_sustains_two_successors_no_cert` / `served_snapshot` harness. **Zero production code** — every byte-comparison seam already exists (`served_snapshot`, `ChainDbServedSource::tip`, `ade_ledger::fingerprint::fingerprint`, `wal.read_all`, `warm_start_recovery`); no seam is missing. `DC-NODE-20`/`DC-NODE-21` stay `declared`. §12 is the completion proof. Commit carries the repo's model trailer. Do **not** run `cargo fmt -p ade_node` (cluster.md §12 lesson).

## 4. Intent
Mechanically prove that the **DC-NODE-20 local-tip-derived successor chain** is replay-equivalent: the durable surface produced by forging N+1/N+2 on `ChainDb::tip` (no cert, no peer-tip equality) is a **pure function of the recovered state + canonical slot schedule** — byte-identical across two clean runs and across a kill/warm-start, with the adoption-certificate file and real wall-clock **absent from the replay surface**. S1/S2 made the cert irrelevant to forge *authority*; S3 proves it is equally irrelevant to the *replay surface*, and that the local-tip forge base introduced no nondeterminism.

## 5. Scope
- **Add three hermetic tests** (`node_sync` test module), modeled on `continue_past_eof_two_runs_byte_identical` / `continue_past_eof_kill_warm_start_recovers_byte_identical` but over the **no-cert, K≥2 local-spine** path (`local_spine_sustains_two_successors_no_cert` harness):
  1. `local_spine_two_runs_byte_identical`
  2. `local_spine_kill_warm_start_byte_identical`
  3. `local_spine_cert_file_absent_from_replay_surface`
- **Zero production change.** All seams exist. *(Were one missing, it would be named here as the sole production touch — none is.)*
- **Out of scope:** any production code; the live pass (S4); flipping DC-NODE-20/21 (close); the registry `tests`-array append for T-REC-03/05 (lands at CE-AH-7 close per the cluster bookkeeping, not here).

## 6. Execution Boundary (TCB color)
- **BLUE (UNCHANGED):** `ade_ledger` fingerprint, `ChainDb`/`pump_block`, `warm_start_recovery`'s forward-replay — exercised, not modified.
- **GREEN:** the DC-NODE-20 forge-base selection (exercised) + the test-only `served_snapshot` projection.
- **RED:** `run_relay_loop` (exercised) + the new test harness (deterministic clock, in-memory ended feed, TempDir).
- No new authority of any color; the new code is `#[cfg(test)]`.

## 7. Invariants Preserved (registry IDs)
`DC-NODE-20` (forge base = local durable tip — exercised, unchanged) · `DC-NODE-21` (cert evidence-only — test 3 proves the cert is absent from the replay surface even when its file is present) · `DC-NODE-05`/`DC-NODE-12` (`pump_block` durable admit) · `DC-NODE-15` · `DC-NODE-18` core / `DC-NODE-19` core (the N-AG `continue_past_eof_*` tests stay green) · `DC-CONS-03` · **determinism** (the durable/served surface is a pure function of recovered state + canonical slot schedule; no `SystemTime` in the durable path — the two-runs byte-identity is the mechanical proof that real wall-clock is absent).

## 8. Invariants Strengthened or Introduced
**Strengthens the replay family — `T-REC-03` (two-clean-runs byte-identity) + `T-REC-05` (kill/warm-start forward-replay recovery)** — by extending their mechanical enforcement to the **DC-NODE-20 local-tip-derived, cert-free, K≥2** successor chain (the prior tests cover the cert-present K=1 N-AG path and the recover-follow / forge-tip-successor paths). Exactly **one** family (replay). Per the cluster bookkeeping, the three test names are appended to `T-REC-03`/`T-REC-05`'s registry `tests` arrays and `strengthened_in += "PHASE4-N-AH"` at **CE-AH-7 close**, not in this slice's commit.

## 9. Design Summary
Each test stands up `s2_extend_lead()` (recovered block-0 spine), declares a single-producer venue, sets `ForgeMode::SingleProducerExtendOwnDurableSpine`, and drives `run_relay_loop` over an **ended** in-memory feed with a `DeterministicClock` so the dead-feed Idle timer forges successors on `ChainDb::tip`. The four durable surfaces are captured via the existing seams and compared with `assert_eq!`:
- **WAL image:** `wal.read_all()`
- **durable tip:** `ChainDbServedSource::new(&chaindb).tip()`
- **ledger fingerprint:** `ade_ledger::fingerprint::fingerprint(&state.receive.ledger)`
- **served chain:** `served_snapshot(&chaindb)` (ordered `HeaderProjection` + body bytes)

## 10. Changes Introduced
- **`local_spine_two_runs_byte_identical`** — two independent runs of the K=2 no-cert local-spine forge (`local_spine_sustains` shape); assert all four surfaces byte-identical across runs and `tip.block_no == 2`.
- **`local_spine_kill_warm_start_byte_identical`** — one K=2 run, capture pre-kill tip/fp/served, drop the handles (TempDir survives), reopen + `warm_start_recovery`, assert post-recovery tip/fp/served byte-identical (no ChainBreak across the local-spine seam).
- **`local_spine_cert_file_absent_from_replay_surface`** — run the local-spine forge **with** a cert file present (carrying a distinctive **bogus** adopted-tip hash) and **without** one; assert the two runs' WAL + tip + fp + served are byte-identical (the cert file does not change the replay surface), **and** the bogus cert hash never appears in any served block body — the cert never enters the replay surface even when on disk. *(A valid cert references the real parent block, whose hash is legitimately in the WAL; the byte-identity is the load-bearing proof, the bogus-hash containment the secondary check.)*
- **No production files touched.**

## 11. Replay, Crash, and Epoch Validation
- **Replay (two-runs):** `local_spine_two_runs_byte_identical` (new) — joins the existing `continue_past_eof_two_runs_byte_identical`, `extend_own_spine_two_runs_byte_identical`, `recover_follow_two_runs_byte_identical` under T-REC-03.
- **Crash/warm-start:** `local_spine_kill_warm_start_byte_identical` (new) — joins `continue_past_eof_kill_warm_start_recovers_byte_identical`, `forge_tip_successor_kill_then_warm_start_recovers_block_one` under T-REC-05.
- **Surface containment:** `local_spine_cert_file_absent_from_replay_surface` (new) — a cert file present (bogus hash) yields a byte-identical replay surface vs no-cert, and the bogus hash never enters a served body; complements `feed_eof_appends_nothing_to_wal`.
- **Epoch:** not applicable.

## 12. Mechanical Acceptance Criteria
- [ ] `cargo test -p ade_node local_spine_two_runs_byte_identical` green.
- [ ] `cargo test -p ade_node local_spine_kill_warm_start_byte_identical` green.
- [ ] `cargo test -p ade_node local_spine_cert_file_absent_from_replay_surface` green.
- [ ] `cargo test -p ade_node` green overall (all existing tests, incl. the N-AG `continue_past_eof_*` set, unchanged).
- [ ] `ci_check_local_durable_forge_base.sh` + `ci_check_cert_evidence_only.sh` + `ci_check_single_producer_extend_own_spine.sh` + `ci_check_single_producer_loop_continuation.sh` + `ci_check_node_run_loop_containment.sh` + `ci_check_node_path_fidelity.sh` stay green.
- [ ] `git diff --stat` touches only the `node_sync` test module (zero production lines).
- [ ] `DC-NODE-20` + `DC-NODE-21` still `declared`.

## 13. Failure Modes
A byte-mismatch in any of the three tests is a **real replay-equivalence violation** of the DC-NODE-20 path (the slice's whole value) — e.g., a hidden wall-clock or map-ordering dependency in the local-tip forge base. The tests fail closed (`assert_eq!`), surfacing it before close.

## 14. Hard Prohibitions
**Inherited (cluster §8):** no cert in the forge/replay path; no new authority of any color; no fork-choice.
**Slice-specific:**
- **No production code** — if a byte-comparison required a missing seam, stop and surface it; do not add silent production accessors. (None is missing.)
- **Do not** weaken or modify the existing N-AG `continue_past_eof_*` tests.
- **Do not** run `cargo fmt -p ade_node` (cluster.md §12 lesson — it churns the whole crate).
- **Do not** touch the pre-existing-stale `ci_check_forge_followed_tip_admission.sh` (cluster.md §12 / AH-FOLLOW).

## 15. Explicit Non-Goals
Forge-base authority (S1) · cert removal (S2) · the operator-gated live pass (S4) · flipping DC-NODE-20/21 + the registry `tests`-array appends (CE-AH-7 close) · the competing-block fence broadening (AH-FOLLOW-1).

## 16. Completion Checklist
- [ ] Three new hermetic replay tests added to the `node_sync` test module; zero production change.
- [ ] `cargo test -p ade_node` green; all six AH/path-fidelity gates green.
- [ ] `DC-NODE-20` + `DC-NODE-21` still `declared`.
