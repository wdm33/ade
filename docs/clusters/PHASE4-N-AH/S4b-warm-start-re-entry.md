# Invariant Slice — Single-producer warm-start re-entry (DC-NODE-22, PHASE4-N-AH S4b)

## 2. Slice Header
**Slice Name:** Single-producer warm-start re-entry derives forge mode from the recovered local durable spine (DC-NODE-22)
**Cluster:** PHASE4-N-AH — local selected durable chain forge-base authority; **rung-1, single-producer only**
**Status:** Proposed
**Authority source:** registry `DC-NODE-22` (declared); the S4 run-2 partial (`docs/evidence/phase4-n-ah-live-run-2-partial.md`) found the gap. CE-AH-8 (new).

**Cluster Exit Criteria Addressed:**
- [ ] **CE-AH-8:** in a declared rung-1 single-producer venue, after warm-start recovery yields a durable local `ChainDb::tip` above the bootstrap anchor, the node re-enters `SingleProducerExtendOwnDurableSpine{current_tip = ChainDb::tip}` and **resumes forging on the recovered spine without a fresh follow-link catch-up** — proven hermetically and (run-3) live. Fenced; fails closed to `InitialCatchupRequired`.

Exit criteria not listed (CE-AH-1/2/5=S1; CE-AH-3=S2; CE-AH-4=S3; CE-AH-6=S4 live; CE-AH-7=close) are out of scope.

**Slice Dependencies:** S1 (`b0fb8817`) DC-NODE-20 forge base; S3 (`dad29b43`) warm-start byte-identity; S4a (`7049d813`) the transcript surface that *witnessed* this gap live.

## 3. Implementation Instruction (AI — INLINE)
In the warm-start arm of `run_node_lifecycle_inner` (`node_lifecycle.rs`), after `warm_start_recovery` + `declare_single_producer_venue`, derive `forge_mode = SingleProducerExtendOwnDurableSpine{adopted_root, current_tip}` from the recovered `ChainDb::tip` **iff** `venue_role == SingleProducer` AND the recovered tip is above the bootstrap anchor (own-forged continuation) AND the DC-NODE-22 fence holds; else fall back to `InitialCatchupRequired`. **No BLUE change; reuses the DC-NODE-20 fence + `ChainDb::tip` + `pump_block`** (sets the forge *mode*, admits nothing). `DC-NODE-22` stays `declared`. §12 is the completion proof. Commit carries the repo's model trailer. **No `cargo fmt -p ade_node`** (cluster.md §12).

## 4. Intent
Make warm-start a first-class part of the sustained-producer claim: a rung-1 single-producer node that restarts must **resume forging from its recovered durable spine**, not stall. S4a's live transcript proved run-2's post-restart node reset to `InitialCatchupRequired`, needed a fresh follow-link catch-up, and stalled in `NoTipAvailable` when the follow link EOF'd first — re-introducing through restart the exact follow-link dependency DC-NODE-20 retired. DC-NODE-22 is the warm-start analog of DC-NODE-20: the recovered own spine already makes `ChainDb::tip` the forge base, so extend-state re-entry is immediate.

## 5. Scope
- **`node_lifecycle.rs` warm-start arm:** derive the extend `forge_mode` from the recovered `ChainDb::tip` for a single-producer venue with a recovered own-spine tip above the anchor, fenced (the 9-condition DC-NODE-22 fence; fail-closed to `InitialCatchupRequired`).
- **The above-anchor threshold — RESOLVED (seam check done, option a′ approved):** the bootstrap anchor block_no is NOT independently available in the warm-start arm (`BootstrapState` had no anchor field — only `anchor_fp` + the authoritative `admit_count`). So `warm_start_recovery` adds an **audit-honest derived recovery-summary field** `replayed_anchor_block_no: Option<BlockNo>` = `recovered_tip.block_no − replayed_admit_count`, where `replayed_admit_count` is recovery's authoritative count of `AdmitBlock`s replayed from the verified `anchor_fp` lineage (it already backs the T-REC-05 fingerprint check) — **NOT** the rejected snapshot-fragile raw `wal.read_all()` count. The threshold `recovered_tip.block_no > replayed_anchor_block_no` is therefore **mathematically equivalent to `replayed_admit_count > 0`** — exactly "did warm-start recover only the anchor, or a local continuation spine above it?". The field is documented as a *derived recovery summary, not an independently persisted chain point*. An independently-persisted anchor tip (option b′) is a bigger storage/bootstrap change — **deferred to a later storage-hardening slice, NOT pulled into this node-lifecycle fix.**
- **Out of scope:** the live run-3 (operator pass); the harness counter fix (operator scratch); any BLUE / fork-choice / multi-producer / preprod change; flipping DC-NODE-22 (close).

## 6. Execution Boundary (TCB color)
- **BLUE (UNCHANGED):** `warm_start_recovery` forward-replay, `ChainDb`/`pump_block`, the durable tip — read, not modified.
- **GREEN:** the `ForgeMode` value derived for the warm-start arm (the DC-NODE-22 condition is a pure predicate over the recovered tip + anchor + venue facts).
- **RED:** `node_lifecycle.rs` warm-start arm (sets `act.forge_mode` post-recovery).
- No new authority of any color; `pump_block` stays the sole durable admit (this rule sets the forge *mode* / reads the recovered tip).

## 7. Invariants Preserved (registry IDs)
`DC-NODE-20` (the forge base is `ChainDb::tip` under the same fence — extended to the warm-start entry) · `DC-NODE-19` core · `DC-NODE-15` (the bare-anchor warm-start still uses the catch-up gate — the fence requires the recovered tip *above* the anchor) · `DC-NODE-05`/`DC-NODE-12` (`pump_block` sole durable admit — DC-NODE-22 admits nothing) · `T-REC-05` (warm-start recovery of the durable tip / served chain is unchanged; this only sets the post-recovery forge mode) · `CN-NODE-02` (the run-loop warm-start wiring) · `DC-CONS-03` (untouched; rung-2 fork-choice) · `DC-NODE-21` (no cert read on warm-start).

## 8. Invariants Strengthened or Introduced
**Introduces `DC-NODE-22`** (single-producer warm-start re-entry derives forge mode from the recovered local durable spine) as mechanically enforced. Exactly **one** new family (warm-start re-entry). At `/cluster-close` (CE-AH-7) the strengthenings are recorded by appending PHASE4-N-AH to `strengthened_in` of **DC-NODE-20** (forge base = ChainDb::tip on warm-start too), **DC-NODE-19** (continue-past-EOF survives restart), **T-REC-05** (recovery now *resumes forging*, not just recovers state), and **CN-NODE-02** (warm-start run-loop wiring). `DC-NODE-22` flips declared→enforced at close.

## 9. Design Summary
After `warm_start_recovery` returns the recovered state and `declare_single_producer_venue()` sets `venue_role = SingleProducer`, the warm-start arm computes the DC-NODE-22 predicate: `venue_role == SingleProducer` ∧ recovery succeeded ∧ `ChainDb::tip` present + contiguous/servable ∧ `tip.block_no > anchor.block_no` ∧ (the DC-NODE-20 observed-feed fence is not yet violated). If true, `act.forge_mode = SingleProducerExtendOwnDurableSpine{adopted_root = recovered_tip, current_tip = recovered_tip}`; the next `ForgeTick` then forges on `ChainDb::tip` via the existing DC-NODE-20 path (emitting `ForgeBaseSelected{forge_base_source=local_chaindb_tip}`). If false (bare anchor, non-single-producer, recovery error), `forge_mode` stays `InitialCatchupRequired` (the existing catch-up flow). Fail-closed.

## 10. Changes Introduced
- `ade_runtime::bootstrap::BootstrapState`: add `replayed_anchor_block_no: Option<BlockNo>` (the derived recovery-summary field, documented as NOT an independently persisted chain point); cold-start / first-run leave it `None`.
- `node_lifecycle::warm_start_recovery`: populate `replayed_anchor_block_no = Some(recovered_tip.block_no − admit_count)` (admit_count is the existing authoritative replay count, line ~1642).
- `node_lifecycle.rs` warm-start arm: set `act.forge_mode` from the recovered `ChainDb::tip` under the DC-NODE-22 fence (the threshold `recovered_tip.block_no > replayed_anchor_block_no` ≡ `admit_count > 0`); else `InitialCatchupRequired`.
- Hermetic tests (§11/§12).
- `ci/ci_check_warm_start_re_entry.sh` (new) — asserts the warm-start arm derives the extend mode for a single-producer above-anchor recovery, fenced + fail-closed, no cert / fork-choice / new BLUE.

## 11. Replay, Crash, and Epoch Validation
- **Threshold (the decision) — `warm_start_reentry_requires_tip_above_recovered_anchor`:** a pure-function test over the new GREEN `warm_start_forge_mode` helper, all three (+1) cases — (i) `replayed_anchor == tip.block_no` (admit_count 0, **bare anchor**) → `InitialCatchupRequired`; (ii) `tip.block_no > replayed_anchor` (admit_count > 0) → `SingleProducerExtendOwnDurableSpine{current_tip = recovered tip}`; (iii) `replayed_anchor == None` (**missing summary**) → `InitialCatchupRequired`; (iv) non-single-producer → `InitialCatchupRequired`.
- **Forge resumption (the core) — `warm_start_single_producer_re_enters_extend_and_forges`:** stand up the local-spine forge (S3 harness), build the warm-start `ForgeActivation` with `forge_mode` from `warm_start_forge_mode`, drive one `ForgeTick` over an **ended** feed (no follow-link catch-up available) and assert it **forges a successor on `ChainDb::tip`** (the transcript emits `forge_base_selected` + a succeeded `forge_result`).
- **T-REC-05** byte-identity (S3) is unchanged.
- **Epoch:** not applicable.

## 12. Mechanical Acceptance Criteria
- [ ] `cargo test -p ade_node warm_start_reentry_requires_tip_above_recovered_anchor` green (the 3+1 threshold cases).
- [ ] `cargo test -p ade_node warm_start_single_producer_re_enters_extend_and_forges` green (forge resumption on `ChainDb::tip`).
- [ ] `ci/ci_check_warm_start_re_entry.sh` (new) green.
- [ ] `cargo test -p ade_node` green overall.
- [ ] `ci_check_local_durable_forge_base.sh` + `ci_check_cert_evidence_only.sh` + `ci_check_live_transcript_forge_base.sh` + `ci_check_node_run_loop_containment.sh` + `ci_check_node_path_fidelity.sh` + `ci_check_node_sched_events_emit_only.sh` stay green.
- [ ] `DC-NODE-22` still `declared`; DC-NODE-20/21 untouched.

## 13. Failure Modes
A bare-anchor or non-single-producer warm-start that incorrectly re-entered extend would risk forging off a peer-forged anchor without catch-up — so the fence is fail-closed (any unmet condition ⇒ `InitialCatchupRequired`). The negative test guards it. A `pump_block` reject on the resumed forge stays a fail-fast (DC-NODE-12).

## 14. Hard Prohibitions
**Inherited (cluster §8):** no cert in the forge path; no new authority; no fork-choice; `pump_block` sole durable admit.
**Slice-specific:**
- **Single-producer ONLY** — never a general restart rule for multi-producer or preprod (DC-CONS-03 untouched).
- **Fail closed to `InitialCatchupRequired`** on any unmet fence condition (esp. bare anchor / recovery error).
- **No BLUE change** — DC-NODE-22 sets the forge *mode* + reads the recovered tip; it admits nothing.
- **Do not** run `cargo fmt -p ade_node`; **do not** touch the pre-existing-stale `ci_check_forge_followed_tip_admission.sh`.

## 15. Explicit Non-Goals
The live run-3 (operator pass) · the harness counter fix (operator scratch) · multi-producer / preprod / fork-choice warm-start · flipping DC-NODE-22 + the strengthenings (CE-AH-7 close) · the competing-block fence broadening (AH-FOLLOW-1).

## 16. Completion Checklist
- [ ] Warm-start arm derives the extend forge mode for single-producer above-anchor recovery, fenced + fail-closed.
- [ ] Positive + negative hermetic tests + `ci_check_warm_start_re_entry.sh` green; all AH/sched/path-fidelity gates green; `cargo test -p ade_node` green.
- [ ] `DC-NODE-22` still `declared`; DC-NODE-20/21 untouched.
