# Invariant Slice — Replay-equivalence over a post-feed-end chain (DC-NODE-19 S3)

## 2. Slice Header
**Slice Name:** Replay-equivalence over a post-feed-end chain (DC-NODE-19, PHASE4-N-AG S3)
**Cluster:** PHASE4-N-AG — single-producer loop continuation after follow-link EOF; **rung-1 only, single-producer only**
**Status:** Proposed
**Authority source:** `docs/clusters/PHASE4-N-AG/cluster.md` (§5, CE-AG-4, §9); registry `DC-NODE-19` (declared — **not flipped** by this slice)

**Cluster Exit Criteria Addressed:**
- [ ] **CE-AG-4:** replay-equivalence over a chain that includes successors forged **after** a feed EOF — two clean runs byte-identical (T-REC-03) + kill/warm-start byte-identical (T-REC-05); the feed-end event is replay-neutral (appends nothing to the WAL).

Exit criteria not listed (CE-AG-1=S1; CE-AG-2/3=S2; CE-AG-5=S4; CE-AG-6=close) are out of scope.

**Slice Dependencies:** S1 (`b9ef6e69`) + S2 (`46098c8c`) — the planner `VenuePolicy`, the loop continuation, and the `s2_extend_lead` always-leader harness.

## 3. Implementation Instruction (AI)
**Read §§9–11 + the S2 impl + `s2_extend_lead` + `extend_own_spine_two_runs_byte_identical`'s warm-start phase first.** **TEST-ONLY** — introduce **no** production code. If a read seam turns out to be missing, **stop and surface it** (do not add production surface silently). Reuse `s2_extend_lead`, `run_relay_loop`, `wal.read_all()`, `ChainDbServedSource` (`ServedHeaderLookup`/`ServedRangeLookup`), `warm_start_recovery`, `ade_ledger::fingerprint`. Do **not** flip `DC-NODE-19`; do **not** touch the registry (the T-REC strengthenings are recorded at `/cluster-close`). §12 is the only completion proof. Commit carries the repo's model trailer.

## 4. Intent
Make the **post-feed-end forge path replay-equivalent**: prove that DC-NODE-19's continuation (S2) introduces **no nondeterminism** on the durable/replay surface — the successors forged after a structural feed EOF are byte-identical across clean runs and across kill/warm-start, and the feed-EOF event itself is replay-neutral. **Guard: feed EOF is loop-control input, not durable input** — it may affect *whether* later ForgeTicks occur under the injected schedule, but EOF itself must **not** persist, hash, mutate ledger state, or append the WAL. This is the determinism gate that must pass before DC-NODE-19 can be enforced.

## 5. Scope
- **Modules:** `ade_node::node_sync` (test module only — 3 new hermetic tests reusing `s2_extend_lead`). **No production module changed.**
- **State machines:** none.
- **Persistence:** none (asserts the WAL contents; changes nothing).
- **Network-visible:** none.
- **Out of scope:** any production change; the live run (S4); flipping `DC-NODE-19`; the registry strengthenings (recorded at close).

## 6. Execution Boundary (TCB color)
- **BLUE (UNCHANGED — zero diff):** `pump_block`, `ade_ledger` ledger/chain_dep/WAL/`fingerprint`, `bootstrap_initial_state` warm-start replay, chain selection — all reused read-only.
- **GREEN / RED:** none changed — the loop (`run_relay_loop_with_sched`), the planner, `venue_policy`, and the fence are reused **as-is** (S1/S2).
- **Tests:** `ade_node::node_sync` `#[cfg(test)]` — over existing BLUE/RED machinery. **No new code in any TCB color.**

## 7. Invariants Preserved (registry IDs)
`DC-NODE-19` (the S2 continuation — exercised, not changed) · `DC-NODE-05` (`pump_block` sole durable tip authority) · `DC-NODE-12` (own-forged durable admit chokepoint) · `DC-WAL-04` (forged-block WAL chain integrity / no-orphan) · `DC-NODE-16` (`pump_block` idempotency) · `DC-CONS-03` (untouched) · `T-DET-01` (same canonical inputs → same authoritative bytes). The injected-clock determinism boundary (DC-NODE-05) holds: the Idle timer paces but never changes outputs.

## 8. Invariants Strengthened or Introduced
Strengthens the **replay-equivalence family** — `T-REC-03` (loop-as-replay) and `T-REC-05` (replay/recovery equivalence incl. forged admits) — extended to a chain that includes **post-feed-end** forged successors, via the 3 new tests. Exactly **one** invariant family (replay / `T-REC-*`). This is CE-AG-4, the determinism precondition for enforcing `DC-NODE-19`. **The `strengthened_in += PHASE4-N-AG` appends on T-REC-03 / T-REC-05 (and the test-name appends) are recorded at `/cluster-close`, not in this slice; `DC-NODE-19` is NOT flipped here.**

## 9. Design Summary
Three hermetic tests over the `s2_extend_lead` always-leader harness (durable block 0 + the DC-NODE-18 extend state), driving `run_relay_loop` with an **ended** in-memory feed + an injected `DeterministicClock` so the loop forges successors **past** the EOF:
- **Two clean runs:** run the identical setup (same recovered state, same ended feed, same clock schedule, same adoption-cert file, same shutdown) twice into two fresh ChainDb/WAL; assert byte-identical `wal.read_all()` images, durable tips, recovered ledger fingerprints, and served-chain bytes (`ChainDbServedSource` per block_no).
- **Kill / warm-start:** forge the post-EOF chain, drop (kill) the ChainDb/WAL/state, reopen + `warm_start_recovery`; assert the recovered durable tip + ledger fingerprint + served chain byte-equal the pre-kill values (no ChainBreak across the post-EOF forge seams) — extends `extend_own_spine_two_runs_byte_identical` / T-REC-05 to the *loop-continued* chain.
- **EOF replay-neutrality:** after a run that forges K successors past EOF, assert `wal.read_all()` holds exactly the seed-provenance entry + K `WalEntry::AdmitBlock` entries (one per forged successor) and **nothing** attributable to the EOF.

**Guard (load-bearing):** *Feed EOF is loop-control input, not durable input. It may affect whether later ForgeTicks occur under the injected schedule, but EOF itself must not persist, hash, mutate ledger state, or append the WAL.* The `LoopState::Ending` signal flows only through the GREEN planner + the RED Idle wait; it reaches no reducer, no `pump_block`, no WAL append.

## 10. Changes Introduced
- **Types:** none.
- **State transitions:** none.
- **Persistence:** none.
- **Removal/Refactors:** none. (Three test functions added to the `node_sync` test module; if `s2_extend_lead` needs a small test-only tweak to run twice / expose the served bytes, that is a test-module change, not production — surfaced if needed.)

## 11. Replay, Crash, and Epoch Validation
- **Replay:** `continue_past_eof_two_runs_byte_identical` — two identical runs → byte-identical WAL image + durable tip + ledger fingerprint + served chain over a post-EOF forged chain (T-REC-03 extended).
- **Crash/restart:** `continue_past_eof_kill_warm_start_recovers_byte_identical` — kill mid-post-EOF-chain → `warm_start_recovery` recovers the same durable tip + ledger fingerprint + served chain (T-REC-05 extended; no ChainBreak).
- **EOF-neutrality:** `feed_eof_appends_nothing_to_wal` — the WAL holds exactly the forged `AdmitBlock`s (+ seed provenance), nothing for the EOF.
- **Epoch boundary:** **not applicable** — within-epoch only.

## 12. Mechanical Acceptance Criteria
- [ ] `continue_past_eof_two_runs_byte_identical` (`ade_node::node_sync::tests`) — two clean runs → byte-identical `wal.read_all()`, durable tip, recovered ledger fingerprint, **and** served-chain bytes (all four surfaces, not a subset).
- [ ] `continue_past_eof_kill_warm_start_recovers_byte_identical` (`ade_node::node_sync::tests`) — kill/warm-start over a post-EOF chain recovers byte-identical durable tip + ledger fingerprint + served chain.
- [ ] `feed_eof_appends_nothing_to_wal` (`ade_node::node_sync::tests`) — after K post-EOF forged successors, `wal.read_all()` contains exactly the seed/provenance entry plus K `WalEntry::AdmitBlock` entries, and no WAL entry attributable to feed EOF.
- [ ] `cargo test -p ade_node` green (the 3 new tests + all existing).
- [ ] `ci/ci_check_node_run_loop_containment.sh` + `ci/ci_check_single_producer_loop_continuation.sh` stay green (no production change).
- [ ] **No production diff** (`git diff` over `crates/*/src` outside the `node_sync` `#[cfg(test)]` module is empty); **`DC-NODE-19` not flipped**; registry untouched.

## 13. Failure Modes
None introduced (test-only). A test **failure** here is load-bearing: it would mean the post-EOF continuation is **non-deterministic** (a real DC-NODE-19 correctness defect) — S3 is the gate that catches it before enforcement.

## 14. Hard Prohibitions
**Inherited (cluster §8):** no new durable tip path; no chain-selector reference (DC-CONS-03 untouched); zero BLUE change; no new CLI flag; no flip of `DC-NODE-19`.
**Slice-specific:**
- **No production code** — test-module additions only (if a seam is missing, stop and surface it).
- No registry edit (the T-REC strengthenings + test-name appends are recorded at `/cluster-close`).
- No weakening of the byte-identity assertions — compare WAL image **and** durable tip **and** ledger fingerprint **and** served chain (a tip-only or WAL-only replay test is insufficient).
- No reliance on wall-clock timing for the determinism claim (outputs are clock-schedule-determined; the Idle sleep only paces).

## 15. Explicit Non-Goals
The planner (S1) · the loop continuation (S2) · the operator-gated live run / CE-AF-6b (S4) · flipping `DC-NODE-19` or any registry strengthening (close) · any production-surface change.

## 16. Completion Checklist
- [ ] Two clean runs byte-identical across all four surfaces (test green).
- [ ] Kill/warm-start byte-identical over a post-EOF chain (test green).
- [ ] EOF appends nothing to the WAL (test green).
- [ ] `cargo test -p ade_node` green; the two named gates stay green.
- [ ] Zero production diff; `DC-NODE-19` still `declared`; registry untouched.
```
