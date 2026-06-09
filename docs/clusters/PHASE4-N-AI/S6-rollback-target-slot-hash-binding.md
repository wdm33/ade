# Invariant Slice AI-S6 — Rollback-target slot/hash binding

> **Remediation slice** opened by the PHASE4-N-AI cluster-close security review (HIGH **H-1**).
> The close is HALTED until this lands. AI-S1…S5 merged. Tight scope: the H-1 fix + two cheap
> WARN fold-ins; no rollback-policy redesign.

## 2. Slice Header
- **Slice Name:** Rollback-target slot/hash canonical binding (H-1 remediation).
- **Cluster:** PHASE4-N-AI. **Status:** Proposed.
- **CE Addressed:** remediates **H-1** under **CE-AI-3** (the live rollback apply path);
  **introduces + enforces DC-NODE-29**. Hardens CE-AI-3's fail-closed bar.
- **Dependency:** the AI-S3/S4b-ii live rollback path (`run_participant_sync`).

## 4. Intent
A live rollback target must be **one coherent durable chain point** — resolved against the durable
ChainDb, with the stored slot the sole slot authority. A peer-supplied slot decoupled from the
verified hash (mixed peer/local authority) must **fail closed before any durable mutation**. Closes
the H-1 vector where a single crafted `RollBackward` truncates the on-disk chain and bricks the node.

## 5. Scope
- **Core fix (RED):** `crates/ade_node/src/node_lifecycle.rs` `run_participant_sync` RollBack arm —
  resolve the wire hash against the ChainDb, **use the stored slot**, **require
  `peer.slot == stored.slot`**, typed fail-closed on mismatch **before** `apply_chain_event` (i.e.
  before `commit_rollback` / `WalEntry::RollBack` / any ChainDb mutation).
- **Fold-in (Sec W-3):** replace the reachable `.expect("Participant venue implies forge activation
  present")` (SyncOnce Participant branch) with a typed `NodeLifecycleError` fail-closed.
- **Fold-in (IDD W-a):** fix the stale `wal_tail_slot` comment (`node_lifecycle.rs` +
  `ade_runtime/src/recovery/restart.rs`) — record that the ChainDb-trim + T-REC-05 fingerprint are
  the load-bearing recovery floor (comment-only; no scan change).
- **Out of scope (close-record notes only):** Sec W-1 (explicit k-floor) and IDD W-b (DC-NODE-28 is
  enforced structurally by sequential loop ordering; `pending_reselection` is forward-defense).

## 6. Execution Boundary (TCB color)
**RED** shell fix (`node_lifecycle::run_participant_sync`) composing existing **BLUE** authorities.
**No new BLUE.** No change to `materialize_rolled_back_state` / `commit_rollback` /
`apply_chain_event` / the reducer / `WalEntry` / `select_best_chain`.

## 7. Invariants Preserved (registry IDs)
`DC-NODE-25` (apply mechanism — unchanged), **`DC-NODE-26` (reconciliation — byte-unchanged;
explicitly NOT weakened)**, `CN-STORE-07` + `DC-CONS-20` (materialize/commit unchanged),
`DC-NODE-27` (replay-equivalence), `DC-NODE-23/24` (detector/resolver), `DC-NODE-20` (SingleProducer
fence). `[[feedback-fail-closed-validation]]`.

## 8. Invariants Introduced
- **DC-NODE-29 (NEW, introduced + enforced by AI-S6) — Live rollback target canonical binding.**
  *For a peer `RollBackward(point)` on the live Participant path, the rollback target MUST be
  resolved against the durable ChainDb and use the stored chain point (stored slot + hash) as the
  sole authority. The peer-supplied slot MUST equal the stored slot for that hash; on any mismatch
  (or unknown hash, or Origin) the path fails closed with a typed error BEFORE `commit_rollback`,
  BEFORE `WalEntry::RollBack`, BEFORE any ChainDb/ledger/chain_dep mutation. No rollback target may
  be built from mixed peer/local authority.* tests: `rollback_slot_hash_mismatch_fails_before_mutation`;
  ci: `ci_check_rollback_target_canonical_binding.sh`. One family: live rollback-target integrity.

## 9. Design Summary
The RollBack arm becomes:
```
peer RollBackward(slot_wire, hash)
  -> stored = chaindb.get_block_by_hash(&hash)?      // None -> typed fail-closed (unknown point)
  -> if slot_wire != stored.slot -> typed fail-closed (RollbackPointSlotMismatch)  // BEFORE any mutation
  -> ChainEvent::RolledBack { to_point: Point { slot: stored.slot, hash }, depth: BlockDistance(0) }
  -> apply_chain_event(...)                            // unchanged; now fed a canonical target
```
A new typed error `NodeSyncError::RollbackPointSlotMismatch { peer_slot, stored_slot, hash }`
(closed enum). The peer-slot is never used to build `to_point`. `StoredBlock` carries
`{hash, slot, bytes}` (no `block_no`); the apply path keys on slot+hash and the depth stays
`BlockDistance(0)` as before. Reconciliation (DC-NODE-26) stays as the post-apply backstop,
byte-unchanged — but it is no longer the *only* line of defense for this vector.

## 10. Changes Introduced
`run_participant_sync` RollBack arm (bind to stored slot + pre-mutation fail-closed);
`NodeSyncError::RollbackPointSlotMismatch`; the W-3 typed `NodeLifecycleError`; the W-a comment
fixes; the gate; the test. No production authority change.

## 11. Replay / Crash / Epoch
The new test asserts **no WAL append on mismatch** → replay is clean (the node is not bricked on
restart). Existing replay-equivalence tests (`wal_rollback_ai_s1`, `apply_driver_ai_s3`) stay green
(the matching-slot path is byte-unchanged).

## 12. Mechanical Acceptance Criteria
- [ ] **`rollback_slot_hash_mismatch_fails_before_mutation`** — peer hash exists but
  `peer.slot != stored.slot` ⇒ **(1)** typed error; **(2)** NO `commit_rollback`; **(3)** NO
  `WalEntry::RollBack` append; **(4)** ChainDb tip unchanged; **(5)** ledger unchanged; **(6)**
  chain_dep unchanged; **(7)** replay after the attempt is clean (not bricked).
- [ ] `participant_rollback_applies_durably` (existing positive — matching slot+hash) still green.
- [ ] `participant_rollback_to_unknown_point_fails_closed` (existing unknown-hash) still green.
- [ ] New gate **`ci/ci_check_rollback_target_canonical_binding.sh`** (here-strings): the RollBack
  arm uses the stored slot, requires `peer.slot == stored.slot` before `apply_chain_event`, and
  never builds `to_point` from the wire slot alone.
- [ ] `cargo test -p ade_node` green; reused gates (`ci_check_live_fork_choice_wiring.sh`,
  `ci_check_live_fork_choice_apply.sh`) green.

## 13. Failure Modes
A mismatched/unknown/Origin target now fails closed with zero durable mutation (deterministic typed
error). The matching path is unchanged.

## 14. Hard Prohibitions
**Do NOT weaken reconciliation** (DC-NODE-26 byte-unchanged) — the fix is pre-commit canonical
binding, not a later check. No new BLUE; no change to `materialize`/`commit_rollback`/
`apply_chain_event`/`WalEntry`/`select_best_chain`. No rollback-policy redesign. The peer slot must
never construct `to_point`.

## 15. Explicit Non-Goals
Sec W-1 (explicit k-floor) and IDD W-b (DC-NODE-28 structural-vs-flag) are **close-record notes**,
not code here. Multi-peer ChainSel. Any change to the apply mechanism.

## 16. Completion Checklist
- [ ] Core fix + `rollback_slot_hash_mismatch_fails_before_mutation` (7 must-holds) green.
- [ ] W-3 typed error + W-a comment fixes in.
- [ ] `ci_check_rollback_target_canonical_binding.sh` green; reused gates green.
- [ ] `cargo test -p ade_node` green; DC-NODE-29 added to registry (declared+enforced); no
  reconciliation weakening.
