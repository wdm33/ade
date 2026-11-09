# Invariant Slice — PHASE4-N-F-G-R S1: stable served block 0 (monotone served block_no)

> **Status:** Planning artifact (non-normative). Normative authority is the registry + CI.

## §2 Slice Header
- **Slice:** PHASE4-N-F-G-R S1 — the node-level serve sibling pushes a self-accepted forge handoff to the
  `ServedChainView` only when its block_no strictly exceeds the highest already-served block_no. The first
  genesis-successor block 0 is served stably; subsequent block-0 re-forges (the hermetic forge re-mints one each
  winning slot) are skipped — so the follower sees a STABLE block 0 to fetch + adopt.
- **Cluster:** PHASE4-N-F-G-R — Served-chain stability for the genesis-successor rehearsal.
- **Status:** planned.
- **CE addressed:** CE-G-R-1 (the serve gate + regression + CI). [S2 = live C1, operator-gated.]

## §3 Dependencies
- Captured evidence: `ServedChainSnapshot.blocks` = append-only `BTreeMap<(SlotNo, Hash32), AcceptedBlock>`
  (`served_chain.rs:40`); `served_chain_admit` inserts each block (no block_no dedup); the hermetic forge
  (DC-NODE-05) re-mints block 0 each winning slot → the served view accumulates multiple block_no-0 blocks.
  Clean-run live (14:57Z): 7 forges, served tip 120465→120537 (block 0), 48× `UnexpectedBlockNo(1,0)`, 0 adoptions.
- The serve sibling: `node_lifecycle` On-arm `push_atomic` loop (the SOLE node-spine push site).
- `ServedChainHandle::push_atomic`, `decode_block` (block_no), `SelfAcceptedHandoff::into_accepted()`.

## §4 Intent (invariant impact)
Close the proven served-block-0 churn so the follower sees a stable block 0 to adopt. Enforces `DC-NODE-11`.
A node-level monotone-block_no serve gate — no forge change, no durable own-tip advance, no `served_chain_admit`
change.

## §5 Scope / What is built
1. **Serve gate** — the serve sibling tracks the highest served block_no; it `push_atomic`s a handoff only when
   the handoff's decoded block_no strictly exceeds that (first block 0 → served; later block-0 re-forges →
   skipped, not re-served). The skipped handoff is dropped (self-accepted but not re-served), never a partial
   serve.
2. **Pin tests:** (a) two block_no-0 handoffs through the gate → the served view holds exactly ONE block 0 (the
   first); (b) a strictly-higher block_no handoff IS served (the gate does not freeze the chain); (c) the gate
   never serves a lower/equal block_no (no churn).
3. **Registry + CI:** `DC-NODE-11` → enforced; a CI gate asserts the serve sibling gates by monotone block_no
   (does not blindly `push_atomic` every handoff) and the forge / `served_chain_admit` / durable tip are unchanged.

**Out of scope:** the live C1 confirmation (S2); full producer own-tip advance (OQ-R1, separate cluster);
served-chain completeness after a feed ingest (OQ-R2); any forge / durable-tip / `self_accept` change.

## §6 Execution Boundary (TCB color)
RED node serve wiring (`node_lifecycle` serve sibling). The BLUE `self_accept` / `served_chain_admit` / serve
reducers + the durable tip are unchanged.

## §11 Replay / Crash / Epoch Validation
The serve gate is a deterministic monotone filter (same handoff sequence ⇒ same served view). Covered by the S1
pins. No new authoritative transition; the durable tip is untouched.

## §12 Mechanical Acceptance Criteria
- [ ] Two block_no-0 handoffs through the gate → the served view holds exactly ONE block 0 (the first).
- [ ] A strictly-higher block_no handoff IS served (the gate does not freeze the chain).
- [ ] A lower/equal block_no handoff is skipped (not re-served) — no churn.
- [ ] No durable own-tip advance; no bypass of `self_accept`; no serve of unvalidated bytes.
- [ ] `DC-NODE-11` enforced; CI gate present.
- [ ] No regression: ade_node node_lifecycle / node_sync + ade_runtime served-chain suites pass.

## §14 Hard Prohibitions
- no durable own-tip advance (separate own-tip cluster, OQ-R1); no forged block 1+ claim; no synthetic numbering;
- no private-only flag; no bypass of `self_accept`; no serve of unvalidated bytes;
- no RO-LIVE flip; no acceptance claim without the follower log through `correlate`.

## §15 Explicit Non-Goals
The live C1 confirmation (S2, operator-gated); full producer own-tip advance (OQ-R1); served-chain completeness
after a feed ingest (OQ-R2); any forge / durable-tip / `self_accept` change.
