# PHASE4-N-U — Forged-block durability (full producer own-tip advance) (DC-NODE-12)

> **Grounded in committed planning + two read-only code investigations** (durable-admit/WAL/recovery + forge/self-accept/served). Invariants sketch: `docs/planning/phase4-n-u-forged-block-durability-invariants.md`; cluster plan: `docs/planning/phase4-n-u-cluster-slice-plan.md`. Regression target: `docs/evidence/c1-genesis-rehearsal-reproduction-README.md` (N-U must not break block-0 acceptance). This is **OQ-R1** from the PHASE4-N-F-G-R close, and declared seam **#4** ("N-U — forged-block durability") in `ade-SEAMS.md §7`.

## §1 Primary invariant (DC-NODE-12)
A self-accepted own-forged block becomes part of the durable chain ONLY by being submitted to the same `pump_block` / `forward_sync AdmitPlan::durable` chokepoint that received blocks use (StoreBlockBytes → AppendWal → AdvanceTip, durable-before-tip, behind the BLUE **extend-only** admit authority). The forge advances NO durable tip directly and has NO second tip-advance path; `pump_block` remains the sole durable tip-advance authority, now feeding both received and forged blocks. (DC-NODE-05 preserved: no direct forge-side tip mutation.)

## §2 The problem (proven from captured evidence, not hypothesis)
Today the forge is self-accept-only (DC-NODE-05): `forge_one_from_recovered → run_real_forge → self_accept → SelfAcceptedHandoff → ServedChainHandle::push_atomic` (`node_lifecycle.rs:1175-1221, 549-578`). It NEVER calls `pump_block`, never `put_block`s, never `AppendWal`s. Captured consequences:
- The forge **re-mints a new genesis-successor block 0 at each winning slot** (no durable tip to build on) — the churn G-R's monotone serve-gate (DC-NODE-11) papered over.
- A forged block is **non-durable**; it survives no restart. The README "ChainBreak at WarmStart" is the *received*-block re-staging hazard (`replay_from_anchor → BlockBytesMissing` when a stale WAL `AdmitBlock`'s ChainDb bytes were cleared on re-stage, `node_lifecycle.rs:1343`); the forge path produces no WAL `AdmitBlock` at all.
- Investigation confirmed the durable admit core (`receive_apply → admit_via_block_validity → block_validity`) is **extend-only** — `select_best_chain`/`fork_choice` are NOT called there (only in `ade_core_interop::follow` + `ade_runtime::consensus::chain_selector`).

## §3 The design — submit the self-accepted forged block to the existing pump
Route the self-accepted forged block (`accepted.into_bytes()` — already the canonical `[era, block]` bytes; no re-encode, I-10) through the EXISTING durable chokepoint via a **new fenced RED driver fn** called from the ForgeTick arm (with `pump_block` inside that fn — gate-compatible; the run-loop/sync containment gates fence their own bodies, not a sibling fn). The durable admit is **extend-only**: a stale-tip forge fails closed (header-position/`prev_hash`, `TipBeforeDurable`, or WAL `prior_fp` mismatch) and the next tick re-forges on the current durable tip (DC-CONS-23). **No `NodeBlockSource` variant** (avoids conflating forged with received provenance); the admit chokepoint, WAL, and `self_accept` stay BLUE and unchanged.

## §4 Normative anchors
- Invariant registry `docs/ade-invariant-registry.toml` — Adds DC-NODE-12, DC-WAL-04, T-REC-05, DC-CONS-23, DC-NODE-13 (declared, `introduced_in = PHASE4-N-U`).
- `docs/planning/phase4-n-u-forged-block-durability-invariants.md` (sketch, I-1…I-10); `docs/planning/phase4-n-u-cluster-slice-plan.md` (3-slice plan + OQ resolutions).
- `ade-SEAMS.md §7` candidate #4 (declared N-U seam); `PHASE4-N-F-G-R/cluster.md §12` (OQ-R1/OQ-R2 scope).

## §5 Entry conditions (what prior clusters guarantee)
- **N-Y** (DC-SYNC-01): `pump_block` / `AdmitPlan::durable` is the durable-before-tip chokepoint for received blocks (`TipBeforeDurable` fail-closed); `verify_chain` + `replay_from_anchor` (T-REC-01/02); snapshot cadence DC-STORE-07.
- **N-F-D/E** (CN-NODE-02 / DC-SYNC-02 / DC-NODE-05): the relay loop advances the tip ONLY via `run_node_sync → pump_block`; the hermetic forge tick is wired + replay-equivalent (T-REC-03).
- **N-F-G-J/Q** (DC-NODE-08 / DC-NODE-10): the forge derives `(block_no, prev_hash)` via `forge_header_position`; cold-start = block 0 + `PrevHash::Genesis`.
- **N-F-G-R** (DC-NODE-11): the served view is a monotone-gated accumulator (the workaround S3 supersedes).
- BLUE `ade_ledger::{block_validity, receive::admit_via_block_validity, wal, producer::{forge, self_accept}}` + `ade_core::consensus` validation core — reused unchanged.

## §6 TCB color map (FC/IS partition)
- **BLUE (reused, unchanged — no new type):** `ade_ledger::block_validity` (incl. `header_position`), `ade_ledger::receive::admit_via_block_validity`, `ade_ledger::wal` (`WalEntry`/`verify_chain`/`replay_from_anchor`), `ade_ledger::producer::{forge, self_accept}`, `ade_core::consensus::{header_validate, header_summary}`. `ade_core::consensus::fork_choice`/`select_best_chain` is **not** on the durable-admit path (stays the follow/`chain_selector` authority; untouched).
- **GREEN (reused):** `ade_runtime::forward_sync::reducer` (`AdmitPlan::durable`), `ade_runtime::producer::self_accepted_handoff`, the loop planner.
- **RED (new wiring + changes):** `ade_node::node_sync` (NEW fenced durable-forge-admit driver fn), `ade_node::node_lifecycle` (ForgeTick wiring; serve sibling, S3), `ade_runtime::forward_sync::pump` (reused), `ade_runtime::recovery::restart` + `node_lifecycle` warm_start (S2), `ade_runtime::chaindb` (reused), `ade_runtime::network::serve_dispatch` / serve sibling (S3).
- **No new BLUE authority or canonical type in this cluster.**

## §7 Slices
| Slice | Scope | CE | Registry → enforced | TCB |
|---|---|---|---|---|
| **S1** | Own-forged durable admit through the pump: new fenced RED driver feeds `accepted.into_bytes()` → `pump_block` from the ForgeTick arm (extend-only validate → StoreBlockBytes → AppendWal → AdvanceTip); forge-successor builds N+1 from the durable tip; stale-tip forge fails closed; byte-identity; rides DC-STORE-07 (no eager per-tip snapshot) | CE-1,2,3,4 | DC-NODE-12, DC-CONS-23, DC-WAL-04 (chaining) | RED |
| **S2** | Forged-tip crash recovery + replay-equivalence: wire WAL-tail reconciliation + forward-replay-from-sub-tip-snapshot into production `warm_start_recovery`; kill-then-recover + two-run byte-identical | CE-5,6 | T-REC-05, DC-WAL-04 (no-orphan) | RED |
| **S3** | Serve-as-durable-chain projection: replace the `ServedChainSnapshot` accumulator + `serve_gate_admits` with a projection over the durable ChainDb (`iter_from_slot`/`get_block_by_*`); coherent-history fetch; retire the G-R workaround | CE-7 | DC-NODE-13 | RED |

## §8 Cluster Exit Criteria
All mechanical/CI-verifiable; each names the gate + key tests the slice **adds** (none exist yet — declared).
- **CE-1 (S1, DC-NODE-12):** new gate `ci/ci_check_forged_durable_admit_via_pump.sh` — the ForgeTick-arm forged block reaches the durable tip ONLY via the fenced driver → `pump_block` (durable-before-tip), no direct `put_block`/`AdvanceTip`/`rollback`, no second tip-advance path; `ci_check_node_run_loop_containment.sh` allow-list extended for the one driver call. Tests: `forge_tick_durable_admit_advances_tip`, `forge_successor_builds_block_1_from_durable_tip`.
- **CE-2 (S1, I-10):** `forged_admit_bytes_byte_identical_to_self_accept` — bytes `put_block`'d + served == `accepted.as_bytes()` (no re-encode); `WalEntry` byte-unchanged (no new variant).
- **CE-3 (S1, DC-CONS-23):** `stale_tip_forge_fails_closed` — a forge built on tip N, after a feed block advanced the durable tip, is rejected at admit (header-position/`prev_hash` or `TipBeforeDurable`), never overrides; gate fences `fork_choice`/`select_best_chain` absent from `receive`/`forward_sync`/the new driver.
- **CE-4 (S1, DC-WAL-04 chaining):** `forged_admit_wal_prior_fp_chains` — forged `AdmitBlock.prior_fp == current durable post_fp` (anchor `initial_ledger_fingerprint` for block 0); a mis-chained forged entry is `ChainBreak` (authority-fatal).
- **CE-5 (S2, T-REC-05 + DC-WAL-04 no-orphan):** new gate `ci/ci_check_forged_tip_recovery.sh`; tests `forge_kill_then_warm_start_recovers_same_tip`, `torn_forge_admit_crash_drops_orphan`, `warm_start_forward_replay_recovers_forged_tip_above_snapshot`.
- **CE-6 (S2, T-REC-05 replay):** `forge_two_clean_runs_byte_identical` — two clean forge-runs over identical inputs → byte-identical durable outputs (tip, WAL image, checkpoints) incl. forged blocks.
- **CE-7 (S3, DC-NODE-13):** new gate `ci/ci_check_served_chain_projection.sh`; tests `served_view_projects_durable_chain`, `follower_fetches_coherent_history_incl_ingested_predecessor`, `served_view_retires_accumulator`.
- **Cluster-wide:** `cargo test --workspace` green; the C1 genesis-rehearsal reproduction re-runs without breaking block-0 acceptance (now a growing chain past block 0).

## §9 Replay obligations
Brings FORGED blocks into the existing durable ChainDb + WAL (same stores as received) — **no new canonical type** (reuse `AcceptedBlock` bytes + `WalEntry::AdmitBlock`). New replay corpus: forge → durable-admit → kill → warm-start (byte-identical, T-REC-05) + a two-clean-run byte-identical forge transcript. Durable-before-tip (DC-SYNC-01) + WAL fingerprint chain (T-REC-01/02) + DC-STORE-07 cadence extend to forged admits unchanged. Command: `cargo test -p ade_testkit` + the `ade_node` forge-recovery tests.

## §10 Invariants
- **Adds:** DC-NODE-12, DC-WAL-04, T-REC-05, DC-CONS-23, DC-NODE-13 — declared → enforced as slices land (S1: DC-NODE-12 + DC-CONS-23 + DC-WAL-04 chaining; S2: T-REC-05 + DC-WAL-04 no-orphan; S3: DC-NODE-13).
- **Strengthens** (`strengthened_in += "PHASE4-N-U"` at close): DC-NODE-05 + CN-NODE-02 (their "local artifact only" / "no forge tip-advance path" containment clauses superseded by DC-NODE-12 via cross-ref — the deeper "pump is the sole durable tip authority" invariant is PRESERVED + extended to forged blocks), DC-SYNC-01, DC-SYNC-02, DC-NODE-10, T-REC-01, T-REC-02, T-REC-03, DC-STORE-07.
- **Preserves / cross-ref (NOT strengthened):** **DC-CONS-03** (Praos fork-choice authority) — N-U does **not** touch or reinforce it; the durable admit is extend-only with no admit-time fork-choice, so DC-CONS-03 is referenced only as the boundary (it stays the follow/`chain_selector` authority). Also CN-FORGE-01 (self-accept token unchanged); the BLUE admit/WAL/validation authorities; relay-loop containment (allow-list extended, not relaxed).

## §11 Forbidden during this cluster (hard boundaries)
- No new BLUE authority or canonical type — reuse `block_validity`/`wal`/`self_accept`/`pump_block`/`AdmitPlan`.
- No second durable tip-advance path — go THROUGH `pump_block`; no forge-specific `put_block`/`AdvanceTip`/`rollback_to_slot`; containment gates stay green (allow-list extended for the one driver call only).
- No admit-time fork-choice — admit stays extend-only; `select_best_chain`/`fork_choice` not added to `receive`/`forward_sync`/the driver (DC-CONS-03 stays the follow/`chain_selector` authority).
- No re-encode — feed `accepted.into_bytes()` verbatim (I-10); no new `WalEntry` variant; no parallel serializer.
- No bypass of `self_accept`; no admit/serve of unvalidated bytes.
- No eager per-tip snapshots — ride DC-STORE-07; recovery proven via WAL replay.
- No RO-LIVE flip (durability ≠ peer acceptance; RO-LIVE-01 stays operator-gated); no Mithril/bootstrap change.

## §12 Open questions
OQ-b/c/d/f/g resolved at `/cluster-plan` (code investigation). Residual:
- **S2 sizing:** the production forward-replay-from-sub-tip-snapshot may be large; S2 may split into S2a (reconciliation) + S2b (forward-replay) at `/slice-doc` — both independently mergeable.
- **Pre-existing wording (noted, not touched):** DC-SYNC-01's statement says the received-admit chokepoint is "…→ block_validity → fork-choice"; the code is extend-only (that "fork-choice" is a linear-extend decision, not a `select_best_chain` call). Out of N-U scope — flagged for a future DC-SYNC-01 wording pass.

## §13 Close record
**CLOSED 2026-06-05.** 3 slices merged: S1 (`f35451f5`/`3fedabea`/`71e789db`), S2 (`232071f7`/`f7e38712`), S3 (`a49563bc` doc + `8e0dbe99` impl). NIT-hygiene `4e358e92`.

**Rules (registry):** DC-NODE-12, DC-CONS-23, DC-WAL-04, T-REC-05, DC-NODE-13 → **enforced** (328→333). Strengthened (`strengthened_in += "PHASE4-N-U"`): CN-CONS-07 (serve-provenance clause: in-memory-token-proof → durable-provenance), DC-NODE-11 (monotone serve-gate mechanism superseded by serve-as-projection; invariant preserved + strengthened — survives restart). DC-NODE-05 / DC-SYNC-01/02 / CN-NODE-02 / T-REC-01/02/03 / DC-STORE-07 / DC-NODE-10 carry the supersede-via-cross-ref relationship documented per-rule. DC-CONS-03 preserved (cross-ref only; durable admit is extend-only).

**CEs (mechanical):**
- CE-1/2/3/4 (S1): `ci/ci_check_forged_durable_admit_via_pump.sh` PASS; tests `forge_tick_durable_admit_advances_tip`, `forge_successor_builds_block_1_from_durable_tip`, `forged_admit_bytes_byte_identical_to_self_accept`, `stale_tip_forge_fails_closed`, `forged_admit_wal_prior_fp_chains` green.
- CE-5/6 (S2): T-REC-05 + DC-WAL-04(no-orphan) **test-enforced** via `forge_kill_then_warm_start_recovers_same_tip_via_forward_replay` + `warm_start_drops_orphan_block_above_wal_tail` (in `cargo test -p ade_node`). HONEST DRIFT: the CE-5 gate `ci_check_forged_tip_recovery.sh` and the CE-6 test `forge_two_clean_runs_byte_identical` named in §8 were **not created literally** — S2 enforced replay-equivalence via the kill-recover fingerprint-equality test (recovered fp == WAL-tail post_fp) + the registry records T-REC-05 `ci_script = ""` (test-enforced) with rationale. The invariants are enforced; the §8 CE artifact names drifted during S2.
- CE-7 (S3): `ci/ci_check_served_chain_projection.sh` PASS; tests `served_view_projects_durable_chain`, `follower_fetches_coherent_history_incl_ingested_predecessor`, `served_view_retires_accumulator` green.
- Cluster-wide: `cargo test --workspace --exclude ade_testkit` → 0 failed (ade_testkit excluded — pre-existing ~600s corpus-suite timeout, environmental). Full CI gate sweep: 0 S3-introduced regressions (baseline-diff verified); S3 net-**fixed** `ci_check_registry_code_locus_exists`; 12 pre-existing gate failures remain (gate-vs-code drift in files N-U never touched — out of scope). C1 genesis-rehearsal mechanical regression preserved: a follower still adopts the served block 0, now via the durable projection (`served_view_projects_durable_chain`); the LIVE C1 rerun stays operator-gated.

**Reviews:** IDD reviewer **PASS** (no BLOCK; one NIT — stale containment-gate comment, fixed in `4e358e92`). Cross-slice security reviewer **PASS** (no HIGH/CRITICAL). Central provenance confirmed: the durable store's only production writers are `pump_block` (validated admit; received + forged) + `bootstrap_initial_state` (trusted seed) — no unvalidated write path feeds the served store.

**Tracked follow-ons (non-blocking; before RO-LIVE-01 / live serve):**
- **[MEDIUM]** `ChainDb::iter_from_slot` (pre-existing, `chaindb/persistent.rs`) materializes the full range + O(N²) hash recovery, and the serve path has no per-request range cap → per-request availability amplification on a long chain. Add a streaming iterator + a max-blocks-per-range bound before any large-chain live serve. Hermetic/operator-gated/no-RO-LIVE-claim now, so not release-blocking.
- **[LOW]** >64 KB block bodies cannot be served (session encoder does not segment payloads > `MAX_PAYLOAD` 65 535 B → drops the peer, fail-closed); unbounded inbound accept in `run_node_serve_task` (pre-existing shared infra). Both reinforce the cluster's no-live-serve claim.

**No RO-LIVE flip; durability + coherent serve ≠ operator-witnessed peer acceptance (RO-LIVE-01 stays operator-gated).**
