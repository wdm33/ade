# PHASE4-N-F-G-R — Served-chain stability for the genesis-successor rehearsal (DC-NODE-11)

> **Grounded in captured evidence (code + clean-follower live run).** With G-Q in and the follower RESET to
> genesis, Ade serves the right block_no (block 0) but the follower does NOT adopt (0 adoptions, 48×
> `UnexpectedBlockNo (BlockNo 1) (BlockNo 0)`; follower tip stays genesis). CAUSE: the hermetic forge
> (DC-NODE-05, advances no own-tip) re-mints a NEW genesis-successor block 0 at EACH winning slot (7× in the
> clean run, slots 120465/120537/…, distinct hashes), and `ServedChainSnapshot.blocks` is an APPEND-ONLY
> `BTreeMap<(SlotNo, Hash32), AcceptedBlock>` (`ade_ledger/src/producer/served_chain.rs:40`) whose
> `served_chain_admit` INSERTS each block with no block_no dedup. So the served view ACCUMULATES multiple
> block_no-0 blocks; the follower downloads one block 0, expects block 1, gets another block 0 → rejects. The
> served view never stabilizes on a single block 0 the follower can fetch + validate + adopt.
> Grounding: `[[project_phase4_c1_genesis_rehearsal_live_state]]`.

## §1 Primary invariant (DC-NODE-11)
Once `--mode node` has self-accepted and SERVED a genesis-successor block at block_no 0, it MUST NOT add or
replace the served view with another block_no-0 block during the same recovered NO-TIP episode (durable
ChainDb tip + recovered tip both None). The FIRST self-accepted block 0 WINS the served view; subsequent
block-0 re-forges are NOT re-served. Equivalently (the implemented form): the node-level serve admits a forge
handoff to the `ServedChainView` only if its block_no STRICTLY EXCEEDS the highest already-served block_no — so
the served chain is monotone in block_no and never churns at a height. The episode ends only when the chain
advances through a SEPARATELY-scoped own-tip-adoption path (a future cluster) or a feed-ingested tip.

**Scope note (narrow, load-bearing):** G-R is the SERVE-side stability gate ONLY. NO durable own-tip advance;
NO forged block 1+ claim; NO synthetic block numbering. The served block is still self-accepted (no bypass of
`self_accept`, no serve of unvalidated bytes). The forge itself is UNCHANGED (it still self-accepts + re-mints
internally, DC-NODE-05 intact); G-R only gates which self-accepted handoffs reach the served view.

## §2 The defect (captured, not hypothesis)
The serve sibling (`node_lifecycle` On-arm) pushes EVERY forge handoff to the `ServedChainView` via
`ServedChainHandle::push_atomic` → `served_chain_admit`. `ServedChainSnapshot.blocks` is an append-only
`BTreeMap<(SlotNo, Hash32), AcceptedBlock>`; `served_chain_admit` inserts by `(slot, hash)` with NO block_no
dedup. The hermetic forge (DC-NODE-05) re-mints a genesis-successor block 0 each winning slot (the durable tip
never advances on self-accept), so each forge appends ANOTHER block_no-0 entry. **Clean-run live confirmation
(2026-06-04 14:57Z, follower reset to genesis):** 7 succeeded forges, served tip churned slots 120465→120537
(both block_no 0), 48× `UnexpectedBlockNo (BlockNo 1) (BlockNo 0)`, 0 adoptions; the follower tip stayed
genesis. (The follower HAS adopted an Ade block 0 before — the archived 11:20:17 `52e3ae88` — in a lucky race
window; G-R makes adoption RELIABLE.)

## §3 The fix — first block 0 wins the served view
The node-level serve sibling (`node_lifecycle`, the `push_atomic` loop) tracks the highest served block_no and
pushes a forge handoff to the `ServedChainView` only when its block_no strictly exceeds that — so the first
block 0 is served stably and subsequent block-0 re-forges are skipped (not re-served). No change to the forge
(it still self-accepts + re-mints internally), no durable own-tip advance, no `served_chain_admit` change (the
BLUE index stays append-only; the node-level rule gates what is pushed).

## §6 TCB color
RED node serve wiring (the serve sibling gates which self-accepted handoffs reach the `ServedChainView`, by
monotone block_no). The BLUE `self_accept`, `served_chain_admit`, and the serve reducers are UNCHANGED; the
durable tip is untouched (DC-NODE-05 intact).

## §7 Slices
| Slice | Scope | CE | Registry | Status |
|---|---|---|---|---|
| **S1** | The serve sibling serves only block_no-advancing handoffs (first block 0 wins; later block-0 re-forges skipped); regression pins (a) two block-0 handoffs → the served view holds exactly ONE block 0 (the first); (b) a strictly-higher block_no handoff IS served | CE-G-R-1 | DC-NODE-11 → enforced | planned |
| **S2** | Live C1 rerun (follower at genesis): Ade forges block 0, serves it STABLY (no churn), the follower fetches/adopts → `AddedToCurrentChain` → `correlate` → `PrivateRehearsalManifest` | CE-G-R-2 | operator-gated | planned |

## §8 Cluster Exit Criteria
- **CE-G-R-1 (mechanical):**
  1. Two block_no-0 forge handoffs pushed through the serve gate → the served view holds exactly ONE block 0 (the first); the second is skipped (not re-served).
  2. A handoff whose block_no strictly exceeds the highest served IS served (the gate does not freeze the chain).
  3. No durable own-tip advance; no bypass of `self_accept`; no serve of unvalidated bytes.
- **CE-G-R-2 (operator-gated):** a C1 rerun (follower at genesis) shows the follower fetch + ADOPT a STABLE
  Ade-forged block 0 (`ChainDB.AddedToCurrentChain` / `ValidCandidate`), bound only by the follower log through
  `correlate` → `PrivateRehearsalManifest`. `blocked_until_operator_c1_genesis_successor_rehearsal`; no RO-LIVE flip.

## §9 Replay obligations
The serve gate is a deterministic monotone filter (same handoff sequence ⇒ same served view). No new
authoritative transition; `self_accept` + `served_chain_admit` + the serve reducers are unchanged.

## §10 Invariants
- **Adds:** `DC-NODE-11` (served-chain block_no monotonicity / stable genesis-successor serve), declared →
  enforced at S1.
- **Preserves / cross-ref:** `DC-NODE-05` (forge advances no durable tip — UNCHANGED; the forge re-mints
  internally, the SERVE gates it), `DC-NODE-07` (serve loopback), `DC-NODE-08` (genesis-successor position),
  `DC-NODE-10` (forge-successor from the evolved spine), `CN-FORGE-01` (self-accept token), `RO-LIVE-01`.

## §11 Forbidden during this cluster (hard boundaries)
- **no durable own-tip advance** (that is the separate own-tip-adoption cluster, OQ-R1);
- **no forged block 1+ claim; no synthetic block numbering; no private-only flag;**
- **no bypass of `self_accept`; no serve of unvalidated bytes;**
- **no RO-LIVE flip; no acceptance claim** without the follower log through `correlate`.

## §12 Open questions
- **OQ-R1 (→ separate cluster, N-U / next):** full producer own-tip advance — forge self-accepts → the node
  ADOPTS its own block as the durable tip → the next forge builds block 1, 2, … a real growing chain. Touches
  durable tip authority, replay, WAL, recovery, chain selection, and the DC-NODE-05 boundary; deserves its own
  dedicated cluster. G-R is the narrow stable-serve for the genesis-successor rehearsal only.
- **OQ-R2 (folded into OQ-R1's cluster):** served-chain COMPLETENESS after a FEED ingest — when Ade ingests
  block 0 and forges block 1 (G-Q), the served view holds block 1 without the ingested predecessor block 0
  (the dirty-follower `UnexpectedBlockNo(0,1)` case). G-R's monotone gate keeps block 0 stable but does not
  serve a feed-ingested predecessor; that belongs with the own-tip / ingest-serve cluster.
