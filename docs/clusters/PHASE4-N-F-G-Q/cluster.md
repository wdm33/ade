# PHASE4-N-F-G-Q — Forge-successor tip/block_no fidelity (DC-NODE-10)

> **Grounded in conclusive captured evidence (FORGE-TIP-DIAG, reverted).** With G-P in, the feed validates
> Step 5 + Step 7 and INGESTS Ade's block 0 (slot 107405) → the node-spine tip advances. The forge then tries
> the SUCCESSOR and fails `relay run-loop sync step failed (RecoveredTipMissingBlockNo)`. The capture proved the
> desync exactly — tick 2 (post-ingest): `selected_tip=Some(107405)` (the durable ChainDb tip the feed
> advanced), `baseline_last_block_no=None` (what `forge_header_position` reads, node_sync.rs:586 =
> `recovered.chain_dep.last_block_no`), `spine_last_block_no=Some(BlockNo(0))` /
> `spine_last_slot=Some(SlotNo(107405))` (the EVOLVED node-spine `chain_dep` the feed actually advanced). The
> correct block_no ALREADY EXISTS in the evolved admitted spine; the forge-successor reads the wrong copy (the
> stale WarmStart baseline). This is NOT missing information.
> Grounding: `[[project_phase4_c1_genesis_rehearsal_live_state]]`.

## §1 Primary invariant (DC-NODE-10)
After the feed validation/admission advances the node spine (a block ingested → `state.receive` evolved), the
next forge MUST derive the successor header position — block_no (and the `chain_dep` / `ledger` it self-accepts
against) — from the **EVOLVED admitted node-spine state** (`state.receive`: the evolved `chain_dep` + `ledger`),
NOT the stale WarmStart baseline (`recovered.chain_dep` / `recovered.ledger`). The successor block_no = the
evolved `chain_dep.last_block_no` + 1; the prev_hash = the durable selected tip's hash. `RecoveredTipMissingBlockNo`
is reserved for a genuinely malformed recovered state (a tip with no height on a path that did NOT advance via
admission) — it MUST NOT fire for a feed-advanced tip (the feed sets the evolved block_no). No guessed
block_no, no `unwrap_or(1)`, no synthetic numbering. The genesis-successor cold-start (BOTH tips None → block 0
+ `PrevHash::Genesis`) is UNCHANGED (`DC-NODE-08` / G-J). The seed-epoch PoolDistr + eta0
(`DC-CINPUT-02b` / `DC-CINPUT-03`) are per-epoch and unchanged — valid for the IN-EPOCH successor; cross-epoch
is off-epoch fail-closed (`DC-EPOCH-03`), unchanged.

**Scope note (narrow, load-bearing):** G-Q selects the evolved admitted chain state for the forge-successor and
makes NO other forge-semantics change; NO durable-recovery / WAL change (the ChainBreak-on-restart is the
SEPARATE N-U durability slice); NO feed-validation change.

## §2 The defect (proven from captured evidence, not hypothesis)
`forge_one_from_recovered` (node_sync.rs) builds the forge base ENTIRELY from `recovered` (the WarmStart
baseline `BootstrapState`): `forge_header_position(selected_tip, recovered.chain_dep.last_block_no)` (:586),
`ctx.eta0 = &recovered.chain_dep.epoch_nonce`, `ctx.chain_dep_state = &recovered.chain_dep`,
`ctx.base_state = &recovered.ledger`. The relay loop derives `selected_tip` from the DURABLE ChainDb
(node_lifecycle.rs:1107 `ChainDb::tip`) — which the feed's `pump_block` advanced — but `ChainTip` =
`{hash, slot}` ONLY (chaindb/types.rs:29), so it carries NO block_no. So after the feed ingests block 0:
`selected_tip = Some(107405)` (durable, feed-advanced) but `recovered.chain_dep.last_block_no = None` (the
baseline, never advanced) → `forge_header_position(Some, None)` → `RecoveredTipMissingBlockNo`. Meanwhile the
relay loop's evolved `state.receive.chain_dep.last_block_no = Some(BlockNo(0))` (proven by FORGE-TIP-DIAG) —
the feed DID advance the spine chain_dep; the forge just reads the baseline copy.

## §3 The fix — forge-successor from the evolved admitted spine state
The relay loop threads the evolved `state.receive` (`chain_dep` + `ledger`) into the forge base, so
`forge_header_position` reads the evolved `last_block_no` (`Some(0)` → successor block_no 1) and
`run_real_forge`'s self-accept validates the successor against the evolved chain state (monotone slot/block_no
vs block 0). The recovered seed sidecar still supplies the per-epoch PoolDistr (`DC-CINPUT-02b`); eta0 is the
evolved `chain_dep.epoch_nonce` = the seed eta0 within the seed epoch (unchanged in-epoch). No `decode_block` /
VRF / Step-5/6/7 change; no durable-recovery / WAL change.

## §6 TCB color
RED node-spine wiring (the relay loop selects the evolved admitted `state.receive` as the forge base instead of
the WarmStart baseline). The BLUE forge engine (`run_real_forge`) + self-accept (`validate_and_apply_header`) +
the position helper (`forge_header_position`) are UNCHANGED — they consume the evolved authoritative state. No
new BLUE type; no new authoritative transition.

## §7 Slices
| Slice | Scope | CE | Registry | Status |
|---|---|---|---|---|
| **S1** | The forge-successor derives (block_no, chain state) from the evolved `state.receive` (not the recovered baseline); successor block_no = evolved `last_block_no` + 1; regression pins (a) post-ingest the forge reads evolved block_no `Some(0)` → successor block_no 1 (not `RecoveredTipMissingBlockNo`), (b) pre-ingest no-tip still forges block 0 (`DC-NODE-08`/G-J), (c) no fallback/guess in the path | CE-G-Q-1 | DC-NODE-10 → enforced | planned |
| **S2** | Live C1 rerun: the node proceeds past `RecoveredTipMissingBlockNo` and attempts/produces block 1+ | CE-G-Q-2 | operator-gated | planned |

## §8 Cluster Exit Criteria
- **CE-G-Q-1 (mechanical):**
  1. A regression proves that, post-ingest, the forge-successor reads the evolved spine `block_no Some(0)` (not the baseline `None`).
  2. The successor forge computes block_no 1 (and `PrevHash::Block(tip)`), NOT `RecoveredTipMissingBlockNo`.
  3. No fallback / guess / `unwrap_or` / synthetic block-numbering exists in the forge-successor path.
  4. The pre-ingest no-tip cold-start path still forges block 0 + `PrevHash::Genesis` (`DC-NODE-08` / G-J), unchanged.
  5. (covered by S2) the C1 rerun proceeds past `RecoveredTipMissingBlockNo` and attempts/produces block 1+.
- **CE-G-Q-2 (operator-gated):** a C1 rerun shows the node past `RecoveredTipMissingBlockNo`, attempting/producing
  block 1+, the serve alive, and any follower adoption decided only by the follower log through `correlate`.
  `blocked_until_operator_c1_genesis_successor_rehearsal`; no RO-LIVE flip.

## §9 Replay obligations
Deriving the successor position from the evolved admitted `state.receive` is deterministic (same ingested chain
state ⇒ same successor block_no + prev_hash). No new authoritative transition; `forge_header_position` +
`run_real_forge` + the BLUE self-accept are unchanged.

## §10 Invariants
- **Adds:** `DC-NODE-10` (forge-successor position from the evolved admitted spine state), declared → enforced at S1.
- **Preserves / cross-ref:** `DC-NODE-08` (genesis-successor cold-start position — the sibling, unchanged),
  `DC-CINPUT-02b` (forge leadership PoolDistr from the seed sidecar) + `DC-CINPUT-03` / `T-REC-04` (forge eta0),
  `DC-NODE-05` (forge subordinate to sync), `DC-EPOCH-03` (single seed epoch), `CN-WIRE-12` (G-O feed decode),
  `DC-CINPUT-04` (G-P feed validation view), `RO-LIVE-01` (no flip).

## §11 Forbidden during this cluster (hard boundaries)
- **no guessed block_no; no `unwrap_or(1)`; no synthetic block numbering.**
- **no durable-recovery / WAL / ChainBreak fix** — that is the SEPARATE N-U durability slice (OQ-Q1).
- **no feed-validation / admission bypass.**
- **no forge-semantics change beyond selecting the evolved admitted chain state.**
- **no RO-LIVE flip; no acceptance claim** without the follower log through `correlate`.

## §12 Open questions
- **OQ-Q1 (→ separate N-U durability slice):** a store with the seed WAL + an admitted block-0 WAL entry fails
  WarmStart recovery with `ChainBreak` (entry 1's `prior_fp = Genesis(0000)` does not chain to the seed entry's
  fp `036111…`). DIFFERENT authority surface (restart recovery, not live successor forge); preserved as
  evidence at `ade-inputs/{snap,wal}.go-p-ingested-dirty`. NOT required for C1 live acceptance (continuing to
  block 1+ needs no restart). Invariant: *a store that admits block 0 after the seed lineage must recover/replay
  without ChainBreak.* Out of scope for G-Q.
- **OQ-Q2:** the evolved `ledger` (base_state) threaded for the successor body — block 1 in the C1 rehearsal is
  expected empty (no txs available on the feed); the evolved ledger is selected for correctness, but successor
  BODY content (txs) is the multi-block / N-U concern, not G-Q.
