# Invariant Slice — PHASE4-N-AE.B: Recovered/Forge-Parent Intersectability (Option B, FindIntersect-only)

## §2 Slice Header
- **Slice Name:** recovered/forged-parent peer-intersectability (FindIntersect-only projection)
- **Cluster:** PHASE4-N-AE — closes **DC-NODE-14** (the *anchor/parent-intersectability clause*; AE.A enforced the followed-tip-lineage clause → `partial`, AE.B closes the umbrella rule → `enforced`)
- **Status:** Proposed
- **PROMOTED:** the 2026-06-07 CE-A5 live run proved AE.B is **the actual C2-LOCAL #8–#9 adoption closer**, not the secondary edge case it was scoped as in `cluster.md §7`. The live relay rejected with `HeaderEnvelopeError (UnexpectedBlockNo (BlockNo 19) (BlockNo 0))`: Ade forged a successor whose parent the relay already knows, but Ade's served chain could not represent that parent as an intersection point → relay fell back to Origin → saw block 19 where it expected block 0.
- **Cluster Exit Criteria Addressed:** **CE-B1** (DC-NODE-14 anchor clause — recovered/forged parent peer-intersectable), **CE-B2** (no synthetic bytes — strengthens CN-CONS-07 / DC-CONS-23), plus **CE-A5** (the live relay-adoption manifest, now unblocked).

## §3 Dependencies
**AE.A merged** (`5f2afc2a`: forge-on-followed-tip gate — Ade chooses the right parent). **AE.C merged** (`5425b23c`: recover→follow WAL prior-fp continuity — lineage survives restart/retry). The three compose: **AE.A picks the right parent · AE.B lets the peer intersect at that parent · AE.C makes the lineage survive restart.** All three are required for a reliable CE-A5 manifest. Root cause confirmed by the existing `#[ignore]`'d AE.B fixtures, which reproduce the live symptom hermetically.

## §4 Intent (invariant impact)
Close **DC-NODE-14**: a recovered or followed forge parent must be **peer-intersectable** — a real Haskell relay must be able to `FindIntersect` at the parent Ade claims to extend, then roll forward onto Ade's forged successor. Before AE.B, `seed_to_snapshot` persists a snapshot keyed by slot only (no servable `StoredBlock`), so `intersect(anchor) == None`; a peer falls back to Origin and rejects the forged successor's block-number jump. After AE.B the serve **projects the recovered anchor as a FindIntersect-only point** — acknowledging the shared point so the relay rolls forward to the real servable successor — **without ever synthesizing or serving bytes for it.**

**AE.B invariant (the constitutional statement):** *A recovered or followed forge parent may be advertised as an intersection point ONLY IF Ade can prove it is the parent of a real servable successor. It MUST NOT be served as block bytes unless original bytes are present.*

## §5 Scope / What is built — Option B (FindIntersect-only), strict — **self-derived forge parent**
The serve **self-derives the forge parent from the `prev_hash` of the earliest servable StoredBlock** — NO `recovered.tip` threading. This generalizes over the recovered anchor (zero-followed: the forged successor's parent IS the recovered anchor) AND the followed/forged tip (multi-followed: the forged successor's parent is whatever its `prev_hash` says), so it covers the live multi-followed case regardless of whether the followed lineage is stored.
- **`ChainDbServedSource::intersect(points)`** — existing StoredBlock match FIRST (DC-NODE-14 followed-tip clause). THEN, **proof-gated forge-parent match:** read the earliest servable StoredBlock, decode its `prev_hash`; if `prev_hash == PrevHash::Block(parent_hash)`, then for each offered point `p` with `p.hash == parent_hash`, return `(p.slot, p.hash)`. The proof IS the linkage: the offered point is the `prev` of a real servable successor (the earliest StoredBlock). Recover-only (NO StoredBlock) → no earliest block → no projection → `None` (fail-closed; never a "magic" anchor).
- **`next_after(cursor)`** — UNCHANGED. The existing slot-range logic already returns the successor for `cursor == forge parent` (the parent's slot < the successor's slot, so `range_bytes_capped(parent.slot, …)` yields the earliest StoredBlock). No special case needed.
- **`get_block_by_hash(parent)` stays `None`**; **`serve_range`/`range_bytes` for the parent stays empty** → BlockFetch of the projected point **refuses structurally** (the parent has no `StoredBlock`; no code serves bytes for it).
- **BLUE additive:** expose `prev_hash: PrevHash` on `ade_ledger::block_validity::DecodedBlock` (already parsed at `decode_block` for `check_header_position`; one construction site, no exhaustive matches). This is an additive field-exposure, NOT a new authority/type/decode-logic change.
- **NEW live-style follow→serve diagnostic** (AC #8): drive a recover→follow→forge through the real `run_node_sync` follow + the serve, and assert the **actual forged parent** is FindIntersect-able and rolls forward to the forged successor. Acceptance surface = **"a Haskell peer can FindIntersect at the forged parent, then roll forward to Ade's forged successor"** — not "ChainDb has the block."
- **Un-ignore + adjust the two AE.B fixtures:** `forged_successor_on_recovered_anchor_is_not_peer_adoptable` → green (a forged successor exists ⇒ its parent is the proof-gated projected point: `intersect(parent)==Some` + `next_after(parent)==successor`). `recovered_anchor_is_not_peer_intersectable` → adjusted to the invariant's **fail-closed** side: recover-ONLY (no successor) ⇒ `intersect(anchor)==None` (no projection without a proven successor — the user's "must not be magic" boundary); the positive case is covered by the forged-successor fixture + the live-style test.

## §6 Execution Boundary (TCB color)
- **RED (changed):** `ade_runtime::network::served_chain_projection` (`ChainDbServedSource::intersect` gains the proof-gated forge-parent match — pure read-only logic, no I/O/clock/rand/float; `next_after`/`serve_range`/`tip` unchanged). **No `serve_dispatch` or `node_lifecycle` change** — the serve self-derives the forge parent from the durable store (no `recovered.tip` threading).
- **BLUE (additive — `prev_hash` exposure only):** `ade_ledger::block_validity::DecodedBlock` gains an additive `prev_hash: PrevHash` field, set from the already-parsed `hb.prev_hash` at `decode_block` (one construction site; no exhaustive matches; no decode-logic / hashing / determinism change). `decode_block` / `block_header_bytes` are otherwise reused (the single decode authority — no new splitter).
- **No new BLUE authority or canonical type. No synthetic `StoredBlock`. No second durable path. No bytes served for the projected point.**

## §7 Invariants Preserved
DC-NODE-13 / CN-CONS-07 (serve-as-projection + serve provenance — the projection serves NO bytes for the anchor; every served byte is still a real `StoredBlock`), DC-CONS-17 (verbatim bytes — unchanged), DC-CONS-23 (extend-only + no-synthetic-bytes — strengthened), DC-NODE-15 / DC-CONS-24 (AE.A forge gate — unchanged; AE.A tests stay green), DC-WAL-02 / T-REC-05 (AE.C — unchanged), DC-SERVEMEM-01 (bounded serve — the projection adds O(1) reads, no unbounded work).

## §8 Invariants Strengthened or Introduced
One invariant family — **forge-parent peer-intersectability**:
- **DC-NODE-14** — `partial` → **`enforced`** (the anchor/parent clause closes the umbrella rule: every claimed forge parent is servable OR peer-intersectable in the durable served lineage).
- **Strengthens** (`strengthened_in += "PHASE4-N-AE"`): **CN-CONS-07** (serve provenance now also covers an intersect-point-without-bytes — proof-gated, never synthetic), **DC-CONS-23** (the no-synthetic-anchor-bytes fence is now mechanically enforced by the structural BlockFetch refusal), **DC-NODE-13** (the serve projection now also answers FindIntersect for a proven parent).

## §11 Replay / Crash / Epoch Validation
- **Determinism:** the projection is a pure deterministic function of (durable ChainDb, projected anchor) — same inputs → same intersect/next_after. No clock/rand/float.
- **Crash/replay:** unchanged — the projected anchor is `recovered.tip` (recovered deterministically per AE.C/T-REC-05); the projection adds no durable state.
- **Live (CE-A5, operator-gated):** after AE.B, the non-producing-relay C2-LOCAL venue → `ba02_evidence::correlate` manifest with **forged hash == adopted hash** (`AddedToCurrentChain`). The live-style diagnostic (AC #8) is the hermetic proxy.

## §12 Mechanical Acceptance Criteria
1. `recovered_anchor_is_not_peer_intersectable` flips **green** as the invariant's **fail-closed** side: recover-ONLY (no servable successor) ⇒ `intersect(anchor) == None` (no projection without a proven successor — the "must not be magic" boundary). The positive case is AC #2 + AC #8.
2. `forged_successor_on_recovered_anchor_is_not_peer_adoptable` flips **green** (a forged successor on the recovered anchor is peer-adoptable: intersect(parent) + roll forward).
3. `intersect([parent]) == Some(parent)` for the projected parent (proof-gated: only when the earliest StoredBlock's `prev_hash == parent`).
4. `next_after(parent) == forged successor` (the relay rolls forward onto the real servable successor).
5. `get_block_by_hash(parent)` is **not required to succeed** (the projection does not make it a StoredBlock).
6. serving / BlockFetching the projected parent **refuses structurally** — a new test asserts `serve_range(parent, parent)` / `range_bytes` is empty and the dispatch emits `NoBlocks` (no bytes for the projected point).
7. **No synthetic `StoredBlock` bytes** — `ci/ci_check_recovered_anchor_intersectable.sh` greps that the projection never constructs/serves bytes for the anchor (intersect-point-only); the proof gate requires a real successor's `prev_hash`.
8. **Live-style follow→serve diagnostic** green — a real `run_node_sync` follow → forge → serve proves the *actual* forged parent is FindIntersect-able and rolls forward to the forged successor (resolves whether the live follow stores the lineage; if not, the projection covers the actual forged parent).
9. `cargo test --workspace` green (AE.A/AE.C tests + the un-ignored AE.B fixtures); containment gates (`ci_check_node_run_loop_containment.sh`, `ci_check_served_chain_projection.sh`, `ci_check_forge_followed_tip_admission.sh`, `ci_check_recover_follow_wal_lineage.sh`) not regressed. **CE-A5 relay-adoption manifest succeeds** (operator-gated, run after AE.B lands).

## §13 Failure Modes
- A point offered for FindIntersect that is neither a StoredBlock nor the proof-gated projected anchor → `intersect == None` (the relay falls back, as today — no false intersect). The proof gate (`earliest StoredBlock.prev_hash == anchor`) prevents advertising an anchor that is NOT the parent of a real servable successor — so no relay is ever rolled forward into a gap.

## §14 Hard Prohibitions
**Inherited (cluster §11):** no `recovered.tip` as a forge base; no parent-hash inference from block number; no peer-tip signal as a chain selector; no fork-choice; no new BLUE authority or canonical type; no containment regression; no RO-LIVE flip on hermetic evidence.
**Slice-specific (your boundaries):**
- **FindIntersect-only projection is allowed; BlockFetch/serve of the projected point is FORBIDDEN.** `intersect(anchor) == Some(anchor)`; `next_after(anchor) == forged successor`; `get_block_by_hash(anchor)` may be `None`; `serve/blockfetch(anchor)` must refuse structurally.
- **No synthetic CBOR / no fake `StoredBlock`.** The anchor is never materialized as bytes (that is Option A — explicitly NOT taken here).
- The projection is **proof-gated**: advertise the anchor as intersectable ONLY if the earliest servable StoredBlock's `prev_hash == anchor` (Ade proves it is the parent of a real servable successor). Never "any snapshot anchor is magically a served block."
- The projection is a **pure read-only function** (no I/O/clock/rand/float; no durable write).

## §15 Explicit Non-Goals
Option A (materialize the recovered anchor as a hash-verified `StoredBlock` via a sanctioned writer) — deferred; it adds an anchor-block-bytes extraction dependency not required to prove adoption once Option B is strict. Fork-choice / multi-producer intake (Gap 1). Any RO-LIVE flip on hermetic evidence. A follow-storage *fix* (if AC #8 reveals the live follow does not store the followed lineage, that is surfaced as a finding for a follow-on — AE.B's projection covers the actual forged parent regardless).
