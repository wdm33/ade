# Invariant Slice — PHASE4-N-F-G-Q S1: forge-successor position from the evolved admitted spine state

> **Status:** Planning artifact (non-normative). Normative authority is the registry + CI.

## §2 Slice Header
- **Slice:** PHASE4-N-F-G-Q S1 — after the feed advances the node spine, the forge-successor derives its header
  position (block_no) + the chain state it self-accepts against from the EVOLVED admitted spine state
  (`state.receive`: evolved `chain_dep` + `ledger`), not the stale WarmStart baseline (`recovered`). Successor
  block_no = evolved `last_block_no` + 1.
- **Cluster:** PHASE4-N-F-G-Q — Forge-successor tip/block_no fidelity.
- **Status:** planned.
- **CE addressed:** CE-G-Q-1 (the wiring + regression + CI). [S2 = live C1, operator-gated.]

## §3 Dependencies
- Captured evidence (G-Q, FORGE-TIP-DIAG, reverted): tick 2 (post-ingest) `selected_tip=Some(107405)`,
  `baseline_last_block_no=None`, `spine_last_block_no=Some(BlockNo(0))`, `spine_last_slot=Some(SlotNo(107405))`.
- `forge_one_from_recovered` (node_sync.rs) — reads `recovered.chain_dep` (block_no/eta0/self-accept) +
  `recovered.ledger` (base_state); `forge_header_position(selected_tip, recovered.chain_dep.last_block_no)` (:586).
- The relay loop (node_lifecycle) — owns the evolved `state.receive` (the feed mutates it) + calls
  `forge_one_from_recovered`; `selected_tip` from the durable `ChainDb::tip` (:1107), `ChainTip = {hash, slot}`
  (no block_no).
- G-P (`DC-CINPUT-04`, the feed validation view) + G-O (`CN-WIRE-12`, the feed decode) — the feed ingest that
  advances the spine.

## §4 Intent (invariant impact)
Close the proven forge-successor desync so the node continues to block 1+ after ingesting block 0. Enforces
`DC-NODE-10`. Selects the EVOLVED admitted spine state for the forge base — no new forge engine, no
`forge_header_position` / `run_real_forge` / VRF / Step-5/6/7 change, no durable-recovery change.

## §5 Scope / What is built
1. **Forge-base selection** — the relay loop threads the evolved `state.receive` (`chain_dep` + `ledger`) into
   `forge_one_from_recovered`, so `forge_header_position` reads the evolved `last_block_no` (`Some(0)` ⇒
   successor 1) and the self-accept validates against the evolved chain state. The recovered seed sidecar still
   supplies the per-epoch PoolDistr; eta0 is the evolved `chain_dep.epoch_nonce` (= seed eta0 in-epoch).
2. **Pin tests:** (a) post-ingest (evolved `chain_dep.last_block_no = Some(0)`, a selected tip present) the
   forge-successor computes block_no 1 + `PrevHash::Block(tip)`, NOT `RecoveredTipMissingBlockNo`; (b) pre-ingest
   (both tips None, baseline) still forges block 0 + `PrevHash::Genesis` (`DC-NODE-08` / G-J), unchanged; (c) the
   path has no fallback / guess / `unwrap_or` / synthetic numbering.
3. **Registry + CI:** `DC-NODE-10` → enforced; a CI gate asserts the forge-successor reads the evolved
   `state.receive` chain state (not the baseline) and that no successor block-number fallback/guess exists.

**Out of scope:** the live C1 confirmation (S2); the N-U durable-recovery ChainBreak (OQ-Q1); successor BODY
content / multi-block (OQ-Q2); any VRF / eta0 / Step-5/6/7 / feed-validation / durable-recovery change.

## §6 Execution Boundary (TCB color)
RED node-spine wiring (`node_lifecycle` relay loop selects the evolved admitted `state.receive` as the forge
base). The BLUE `forge_header_position` + `run_real_forge` + `validate_and_apply_header` self-accept are
unchanged — they consume the evolved authoritative state.

## §11 Replay / Crash / Epoch Validation
Deriving the successor position from the evolved admitted `state.receive` is deterministic (same ingested chain
state ⇒ same successor block_no + prev_hash). Covered by the S1 pins. No new authoritative transition. Off-epoch
remains fail-closed (`DC-EPOCH-03`, unchanged).

## §12 Mechanical Acceptance Criteria
- [ ] A regression proves the post-ingest forge-successor reads the evolved spine `block_no Some(0)` (not the baseline `None`).
- [ ] The successor forge computes block_no 1 + `PrevHash::Block(tip)`, NOT `RecoveredTipMissingBlockNo`.
- [ ] No fallback / guess / `unwrap_or` / synthetic block-numbering exists in the forge-successor path.
- [ ] The pre-ingest no-tip cold-start still forges block 0 + `PrevHash::Genesis` (`DC-NODE-08` / G-J), unchanged.
- [ ] `DC-NODE-10` enforced; CI gate present.
- [ ] No regression: ade_node node_sync / node_lifecycle + ade_runtime + ade_ledger suites pass.

## §14 Hard Prohibitions
- no guessed block_no; no `unwrap_or(1)`; no synthetic block numbering;
- no durable-recovery / WAL / ChainBreak fix (separate N-U slice); no feed-validation / admission bypass;
- no forge-semantics change beyond selecting the evolved admitted chain state;
- no RO-LIVE flip; no acceptance claim without the follower log through `correlate`.

## §15 Explicit Non-Goals
The live C1 confirmation (S2, operator-gated); the N-U durable-recovery ChainBreak (OQ-Q1); successor BODY
content / multi-block progression (OQ-Q2); any VRF / eta0 / Step-5/6/7 / feed-validation / durable-recovery change.
