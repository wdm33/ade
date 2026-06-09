# Invariant Slice AI-S1 — Rollback WAL durability foundation

> Slice of cluster PHASE4-N-AI (`docs/clusters/PHASE4-N-AI/cluster.md`). The cluster's
> **first** slice — the constitutional guard (no live fork-choice wiring before this is
> proven). BLUE-only. Implements OQ-1 mechanism A.

## 2. Slice Header
- **Slice Name:** Rollback WAL durability foundation (`WalEntry::RollBack` + rollback-aware fp replay).
- **Cluster:** PHASE4-N-AI — live fork-choice wiring (rung-2, single-best-peer).
- **Status:** Proposed.
- **Cluster Exit Criteria Addressed:**
  - [ ] **CE-AI-1** (`DC-NODE-27` rollback replay-equivalence) — *mechanism half; production half is AI-S3.*
- **Slice Dependencies:** none (first slice; AI-S2…S5 depend on it).

## 4. Intent
Make a live rollback **durable and replay-equivalent**: a chain that underwent a
rollback+reselection must, on restart, recover the **selected** chain and **never** resurrect
the abandoned branch — by recording the rollback as an append-only `WalEntry::RollBack` marker
that *records* a rollback happened (it does not decide or materialize one), whose replay
re-anchors the fingerprint chain and re-invokes the **existing** `materialize_rolled_back_state`
+ lockstep authority. Closes the OQ-1 danger: today a live rollback would `ChainBreak` or
resurrect the abandoned branch.

## 5. Scope
- **Modules / crates:** BLUE `ade_ledger::wal` — `event.rs` (`WalEntry::RollBack` + `RollbackReason`
  + encode/decode), `replay.rs` (rollback-aware **fp-only** `replay_from_anchor`), `store_trait.rs`
  (`verify_chain` RollBack arm), `error.rs` (new fail-closed variant). Reused unchanged:
  `ade_ledger::rollback::{materialize_rolled_back_state, commit_rollback}`,
  `ade_core::consensus::rollback::apply_rollback`.
- **State machines affected:** none (the rollback *authority* is reused, not changed).
- **Persistence impact:** **one new WAL record shape** — `WalEntry::RollBack` at tag 4 (additive,
  version-gated; append-only).
- **Network-visible impact:** none.
- **Out of scope:** production recovery rewire + live RollBack *production* — **AI-S3** (RED);
  detector/loop/forge-gate — AI-S2/S4; evidence — AI-S5.

## 6. Execution Boundary (TCB color)
- **BLUE:** `ade_ledger::wal::{event, replay, store_trait, error}` (all changes). Reused-unchanged
  BLUE: `ade_ledger::rollback::*`, `ade_core::consensus::rollback`.
- **GREEN:** none.
- **RED:** none. *(The production recovery/produce wiring that would make this RED is AI-S3.)*

## 7. Invariants Preserved (registry IDs)
`CN-WAL-01` (append-only — no mutation method added), `DC-WAL-02`/`DC-WAL-03` (fp-chain integrity +
anchor+WAL replay-equiv), `T-REC-03`/`T-REC-05` (replay/recovery equivalence), `DC-CONS-05`/`DC-CONS-06`
(rollback ≤ k / byte-identical truncated replay — reused, not re-implemented), `CN-STORE-07` (single
materialize authority — reused), `DC-CONS-20` (lockstep — reused), `DC-CONS-22` (replay-forward),
`T-ENC-01`/`T-ENC-02`/`T-ENC-03` (canonical / non-canonical rejected / round-trip identity), `T-DET-01`
(determinism). **`DC-CONS-03` untouched** (fork-choice is AI-S4).

## 8. Invariants Strengthened / Introduced
- **Introduces** the canonical type `WalEntry::RollBack` (the cluster's only new BLUE authority
  surface) + `RollbackReason` (closed).
- **Strengthens toward enforced** `DC-NODE-27` (rollback+reselection replay-equivalence) — the
  **mechanism half**: the WAL can faithfully record a rollback and replay it byte-identically through
  the existing authority. *(Production half — live RollBack on a real restart — lands in AI-S3;
  `DC-NODE-27` flips `declared→enforced` at cluster close.)* One invariant family: WAL rollback
  durability / replay-equivalence.

## 9. Design Summary — three explicit layers (no overclaim)
**Layer 1 — S1 fp-chain replay (`replay_from_anchor` / `verify_chain`): fingerprint-chain ONLY.**
Rollback-aware but **fp-only — it does NOT call materialize**. Track each `AdmitBlock`'s `post_fp` by
point during the walk. On `RollBack { to_point, .. }`: re-anchor `prev_post_fp` to the `post_fp` the
walk already recorded for the `AdmitBlock` at `to_point` (or `anchor_fp` if `to_point` is the anchor)
— an **existing in-chain fingerprint, not a recorded rollback fp**; mark `AdmitBlock`s above
`to_point` **superseded** (bytes not required; excluded from `tail_fp`). Post-RollBack `AdmitBlock`s
chain from the re-anchored fp (else `ChainBreak`). Fail-closed `RollbackTargetNotInChain` if
`to_point` ∉ {prior `AdmitBlock` points, anchor}.

**Layer 2 — S1 hermetic state-replay test: proves the authority-call shape.** A BLUE test explicitly
invokes `materialize_rolled_back_state(to_point)` (`CN-STORE-07`) + the lockstep reducer (`DC-CONS-20`)
+ forward-applies the effective post-rollback `AdmitBlock`s, and asserts the materialized fp ==
Layer-1's `tail_fp` == the selected tip, and the abandoned branch is never in the recovered state.
**This is where "replay re-invokes materialize" (hard line 4) is proven — hermetically, against the
existing authority, with no recorded fp trusted** (the entry carries no fp; the cross-check is the
materialize-recompute vs the in-chain `post_fp`).

**Layer 3 — AI-S3 (OUT of this slice): production recovery wiring.** AI-S3 wires production
restart/recovery (`node_lifecycle`/`bootstrap`, RED) to use that **same** authority path for
live-generated RollBack entries, and produces RollBack entries from the live apply driver. **S1 proves
the mechanism + the authority-call shape; S3 wires it into production. S1 touches no production recovery
and produces no RollBack entry at runtime.**

## 10. Changes Introduced
- **Types:** `WalEntry::RollBack { to_point, reason, prior_tip, selected_tip }` (each tip =
  `(slot, hash, block_no)`); `RollbackReason` closed enum (`ForkChoiceWin = 0`, `PeerRollBackward = 1`;
  `from_wire_code` fails closed on unknown); `WalError::RollbackTargetNotInChain` (reuses `Structural`
  for malformed payloads).
- **`selected_tip` (and `prior_tip`, `reason`) are AUDIT / RECONCILIATION fields ONLY.** Replay
  authority is the **rollback target (`to_point`) plus the subsequent `AdmitBlock` entries**. Replay
  MUST NOT set the durable tip from `selected_tip` without validating/applying the selected branch —
  this guards against header-only adoption via WAL metadata.
- **Persistence — new WAL record shape (pinned canonical CBOR):**
  `array(2)[ uint 4, array(10)[ to_slot:uint, to_hash:bytes(32), to_block_no:uint, reason_code:uint,
  prior_slot:uint, prior_hash:bytes(32), prior_block_no:uint, selected_slot:uint, selected_hash:bytes(32),
  selected_block_no:uint ] ]` — definite-length, canonical widths, via the existing
  `write_uint_canonical`/`write_bytes_canonical`/`write_array_header`; decode mirrors via
  `expect_definite_array(.., 10, "RollBack payload")` + `read_uint`/`read_hash32`. Tag 4 (after
  `AdmitBlock`=0, reserved 1/2, `Seed`=3); the unknown-tag fall-through (`Structural "unknown wal entry
  tag"`) is unchanged.
- **State transitions:** none new — `apply_rollback`/`materialize_rolled_back_state`/`commit_rollback`
  reused verbatim.
- **Walks:** `encode_wal_entry`, `decode_wal_entry`, `replay_from_anchor`, `verify_chain` each gain a
  `RollBack` arm (the exhaustive `match` makes this a compile error until done in all four).

## 11. Replay / Crash / Epoch Validation
- **fp-chain replay tests** (`ade_ledger::wal`): `replay_with_rollback_recovers_selected_not_abandoned`
  (build `[a1, a2(abandoned), RollBack(to a1), b1, b2]` → `tail_fp` = b2 chain; a2 superseded, a2 bytes
  **not required**); `replay_rollback_reanchors_to_existing_post_fp`; `replay_with_rollback_two_runs_byte_identical`
  (T-REC-03); `verify_chain_accepts_recorded_rollback`.
- **Hermetic state-replay test (Layer 2):** `rollback_state_materialize_reinvokes_authority_and_matches_fp`
  — `materialize_rolled_back_state(to_point)` + forward-apply b1,b2 → fp == fp-walk `tail_fp` == selected
  tip; abandoned a2 never in the recovered state; durable tip derived from `to_point`+applied AdmitBlocks,
  never from `selected_tip` metadata.
- **Crash/restart:** proven via the byte-identical two-run replay (the production recovery path is wired
  in AI-S3).
- **Epoch boundary:** not applicable (no nonce/epoch change here).

## 12. Mechanical Acceptance Criteria
- [ ] `wal_rollback_entry_round_trips_canonical_cbor` — encode(decode(RollBack)) == bytes (T-ENC-03).
- [ ] `wal_decode_rejects_malformed_rollback_payload` — short/wrong-arity payload → `Structural`, fail-closed.
- [ ] `wal_decode_rejects_noncanonical_rollback` — non-canonical widths / indefinite → fail-closed (T-ENC-02).
- [ ] `wal_decode_rejects_unknown_tag` — tag 5 → `Structural "unknown wal entry tag"` (regression-guards the version gate).
- [ ] `replay_with_rollback_recovers_selected_not_abandoned` (CE-AI-1 core).
- [ ] `replay_rollback_target_not_in_chain_fails_closed` — `RollbackTargetNotInChain`.
- [ ] `rollback_state_materialize_reinvokes_authority_and_matches_fp` (Layer 2 / hard line 4).
- [ ] `rollback_exceeding_k_or_crossing_immutable_rejected` — via `apply_rollback`/`materialize` bounds →
  `ExceededRollback`/`ForkBeforeImmutableTip` (DC-CONS-05/06); replay never applies it.
- [ ] `replay_with_rollback_two_runs_byte_identical` (T-REC-03).
- [ ] New gate **`ci/ci_check_wal_rollback_replay_equiv.sh`** green (asserts: tag-4 variant in all four
  exhaustive matches; the Layer-2 test re-invokes `materialize_rolled_back_state` and does NOT
  re-implement rollback; no recorded-fp-trust; no new WAL mutation method; durable tip not set from
  `selected_tip`).
- [ ] `ci/ci_check_wal_append_only.sh` stays green; `cargo test -p ade_ledger` green.

## 13. Failure Modes (all deterministic, fail-fast)
- Unknown tag / malformed RollBack payload → `WalError::Structural` (decode, fail-closed).
- `to_point` not a prior `AdmitBlock`/anchor → `WalError::RollbackTargetNotInChain` (fail-closed; replay halts).
- Rollback exceeding k / crossing the immutable tip → `apply_rollback` returns `ExceededRollback` /
  `ForkBeforeImmutableTip`; replay halts, never applies (reused DC-CONS-05/06).
- Materialized fp ≠ fp-walk re-anchor → `WalTailFingerprintMismatch` (fail-fast — no silent divergence).
- *(All affect replay provenance → fail-fast per template §13.)*

## 14. Hard Prohibitions
**Inherits all eight cluster hard lines** (cluster doc §8) — esp. #2 (`WalEntry::RollBack` is the only
new BLUE), #3 (not a second rollback impl), #4 (replay re-invokes `materialize_rolled_back_state` +
lockstep). **Slice-specific:**
- No recorded post-rollback fp trusted without recompute+cross-check (the entry carries no fp).
- No re-implementation of rollback, materialize, or forward-replay (reuse only).
- `selected_tip`/`prior_tip` are audit/reconciliation only — replay never sets the durable tip from
  `selected_tip`; the durable tip comes only from `to_point` + applied subsequent `AdmitBlock`s (guards
  against header-only adoption via WAL metadata).
- `reason` is a closed uint-coded enum, never a free `String`.
- No WAL mutation method (append-only); no `String`/`anyhow`/float/`HashMap`/wall-clock in BLUE.
- **No live wiring** (S2–S5) in this slice; **no live receive-loop wiring**; `DC-CONS-03` untouched.

## 15. Explicit Non-Goals
No production recovery rewire (AI-S3); no live RollBack production / apply driver (AI-S3); no
detector/loop/forge-gate (AI-S2/S4); no convergence evidence (AI-S5); no multi-peer; no new
fork-choice/consensus; no performance work; no config/feature flags.

## 16. Completion Checklist
- [ ] `WalEntry::RollBack` (tag 4) + `RollbackReason` added; all four exhaustive matches updated.
- [ ] All new data canonically encoded; non-canonical + unknown-tag + malformed rejected deterministically.
- [ ] fp-chain replay rollback-aware (re-anchor to in-chain `post_fp`; superseded bytes not required); no materialize call inside the fp-walk.
- [ ] Layer-2 hermetic test proves replay re-invokes `materialize_rolled_back_state` + lockstep, recovers selected-not-abandoned, byte-identical.
- [ ] `selected_tip` never sets the durable tip.
- [ ] CN-WAL-01 append-only preserved (no mutation method); `DC-CONS-03` untouched.
- [ ] `ci/ci_check_wal_rollback_replay_equiv.sh` + `cargo test -p ade_ledger` green; `ci_check_wal_append_only.sh` stays green.
