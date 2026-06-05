# Invariant Slice — PHASE4-N-U S1: own-forged durable admit through the pump

## §2 Slice Header
- **Slice Name:** own-forged durable admit through the pump
- **Cluster:** PHASE4-N-U (forged-block durability) — primary invariant **DC-NODE-12**
- **Status:** declared → in progress
- **Cluster Exit Criteria Addressed:** **CE-1** (DC-NODE-12 — forged block reaches the durable tip ONLY via the fenced driver → `pump_block`, durable-before-tip, no second tip-advance path), **CE-2** (I-10 — byte-identity; no re-encode; no new `WalEntry` variant), **CE-3** (DC-CONS-23 — stale-tip forge fails closed; no admit-time fork-choice), **CE-4** (DC-WAL-04 chaining — forged `AdmitBlock.prior_fp == current durable post_fp`). *(CE-5/6 recovery = S2; CE-7 serve = S3.)*

## §3 Dependencies
Cluster entry conditions only (N-Y `pump_block`/`AdmitPlan::durable`; N-F-D/E relay loop + forge tick; N-F-G-J/Q `forge_header_position`; BLUE `block_validity`/`wal`/`self_accept`). **No dependency on S2 or S3** — S1 is the foundation they build on.

## §4 Intent (invariant impact)
Introduce **DC-NODE-12**: a self-accepted forged block becomes durable ONLY by submission to the existing `pump_block`/`AdmitPlan::durable` chokepoint. Before S1 the forge advances no durable tip (DC-NODE-05 containment) and re-mints block 0 each slot; after S1 the forge's self-accepted output is admitted through the same durable-before-tip authority as received blocks, so the durable tip advances and the next forge builds N+1. The forge gains **no** direct tip-advance path (`pump_block` stays the sole durable tip authority — DC-NODE-12 supersedes DC-NODE-05's "local artifact only" clause via cross-ref, preserving its deeper invariant). Co-enforces **DC-CONS-23** (extend-only; stale-tip forge fails closed) and **DC-WAL-04** (chaining) — facets of the same admit.

## §5 Scope / What is built
- **NEW fenced RED driver fn** in `ade_node::node_sync` (e.g. `admit_forged_block_durably`): takes a self-accepted `AcceptedBlock` (via `SelfAcceptedHandoff`), extracts `accepted.into_bytes()` (**no re-encode**), feeds them to `pump_block` — decode → extend-only `admit_via_block_validity` → StoreBlockBytes → AppendWal → AdvanceTip (durable-before-tip; `TipBeforeDurable` fail-closed). On a stale-tip forge (header-position/`prev_hash` mismatch vs the current durable tip), it **fails closed** (typed error; tip unchanged; next tick re-forges on the current durable tip).
- **WIRE the ForgeTick arm** (`node_lifecycle.rs`) to call this driver with the self-accepted handoff. `forge_header_position` now reads the durable-consistent tip (durable ChainDb + evolved spine advance together via the driver→pump). The G-R served-view `push_atomic` is **retained unchanged** (serve still works from the accumulator until S3).
- **NEW gate** `ci/ci_check_forged_durable_admit_via_pump.sh`.
- **EXTEND** `ci/ci_check_node_run_loop_containment.sh` allow-list for the one new driver call (the gate still forbids direct `pump_block(`/`put_block`/`AdvanceTip`/`rollback` in `run_relay_loop`'s body).

## §6 Execution Boundary (TCB color)
- **RED (new/changed):** `ade_node::node_sync` (new driver fn), `ade_node::node_lifecycle` (ForgeTick wiring).
- **RED (reused, unchanged):** `ade_runtime::forward_sync::pump`, `ade_runtime::{chaindb, wal}`.
- **GREEN (reused, unchanged):** `ade_runtime::forward_sync::reducer` (`AdmitPlan::durable`), `ade_runtime::producer::self_accepted_handoff`.
- **BLUE (reused, NOT edited — no new type):** `ade_ledger::{receive::admit_via_block_validity, block_validity (incl. header_position), wal, producer::self_accept}`, `ade_core::consensus::header_validate`.

## §7 Invariants Preserved
DC-SYNC-01 (same `AdmitPlan::durable` ordering), DC-SYNC-02 / CN-NODE-02 (`pump_block` stays sole tip authority; containment gate **extended, not relaxed**), DC-NODE-05 (forge advances no tip *directly* — preserved; the pump does), CN-FORGE-01 (driver consumes only a self-accepted `AcceptedBlock`; `self_accept` unchanged), T-REC-01/02 (same `prior_fp`/`post_fp` chain), DC-STORE-07 (rides existing cadence; no eager snapshot), T-REC-03 (relay loop stays deterministic), DC-NODE-08/10/11 (cold-start position / forge-successor / G-R serve gate retained).

## §8 Invariants Strengthened or Introduced
One invariant family — **own-forged durable admit**:
- **DC-NODE-12** — declared → **enforced** (the slice's primary invariant; gate + tests below).
- **DC-CONS-23** — declared → **enforced** (extend-only / stale-tip fail-closed; a facet of the same admit).
- **DC-WAL-04** — declared → **partial** at S1 (chaining clause tested: `forged_admit_wal_prior_fp_chains`); its **no-orphan-recovery clause flips it to enforced at S2**.

(These three are co-enforced because they are inseparable properties of routing the forged block through the one pump.)

## §11 Replay / Crash / Epoch Validation
- **Replay (in-run determinism):** the durable output (tip slot/hash/block_no, ledger fp, WAL `AdmitBlock`) after a forged admit is a deterministic function of (forged bytes, current durable state). Tests: `forge_tick_durable_admit_advances_tip`, `forge_successor_builds_block_1_from_durable_tip`.
- **Crash recovery:** OUT OF SCOPE (S2, T-REC-05). S1 makes the admit durable via the same path as received blocks (same pre-existing recovery semantics — not newly weakened); S2 proves kill-then-recover.
- **Epoch:** unchanged — the forge epoch guard (DC-EPOCH-03) is upstream of the admit.

## §12 Mechanical Acceptance Criteria
- `cargo test -p ade_node` green incl. NEW: `forge_tick_durable_admit_advances_tip`, `forge_successor_builds_block_1_from_durable_tip`, `forged_admit_bytes_byte_identical_to_self_accept`, `stale_tip_forge_fails_closed`, `forged_admit_wal_prior_fp_chains`.
- NEW `ci/ci_check_forged_durable_admit_via_pump.sh` green: (a) ForgeTick durable admit routes via the fenced driver → `pump_block`; (b) no direct `put_block`/`AdvanceTip`/`rollback` at the ForgeTick site; (c) driver feeds `accepted.into_bytes()` (no re-encode); (d) no new `WalEntry` variant; (e) no `select_best_chain`/`fork_choice` token in driver/`receive`/`forward_sync`.
- `ci/ci_check_node_run_loop_containment.sh` green (extended allow-list; direct tip-mutation in the loop body still forbidden).
- `ci/ci_check_node_sync_via_pump.sh` green (`run_node_sync` body unchanged).
- Relevant workspace/crate tests green; the C1 genesis-rehearsal reproduction remains the release regression target. Full `cargo test --workspace` is a cluster-close gate where environment timeouts must be reported honestly.

## §14 Hard Prohibitions
**Inherited (cluster §11):** no new BLUE authority/type; no second durable tip-advance path; no admit-time fork-choice; no re-encode; no new `WalEntry` variant; no bypass of `self_accept`; no eager per-tip snapshots; no RO-LIVE flip; no Mithril/bootstrap change.
**Slice-specific:** the driver must be a sibling fn (not inlined in `run_relay_loop`/`run_node_sync` bodies); consumes ONLY a `SelfAcceptedHandoff`/`AcceptedBlock`; no `NodeBlockSource` variant; retains the G-R serve push (serve correctness until S3); reuses `pump_block` (no copy/fork of the admit logic).

## §15 Explicit Non-Goals
Crash recovery of the forged tip (S2); serve-as-projection (S3; G-R accumulator stays); feed-ingest-predecessor serving / OQ-R2 (S3); any RO-LIVE flip or bounty claim.
