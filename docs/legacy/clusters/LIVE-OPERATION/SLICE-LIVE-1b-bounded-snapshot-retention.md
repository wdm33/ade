# LIVE-1b — bounded snapshot retention (fix the chain.db disk-fill)

> **Cluster:** LIVE-OPERATION. Operational / GREEN efficiency slice surfaced by LIVE-1: the live follow
> filled the disk (chain.db → 18G → ENOSPC, Ade correctly failed closed). **No BLUE / consensus change** —
> ledger rules, chain selection, VRF/nonce, frozen leadership, accumulator, reduced checkpoint, and the WAL
> admission authority are all UNTOUCHED. Only the snapshot **capture cadence** + **retention** change.

## The bug

`run_node_sync` captures the E4 recovery checkpoint — `PersistentSnapshotCache::capture(tip.slot,
&state.receive.ledger, &chain_dep)` → `encode_snapshot` (the FULL ledger, ~600 MB) →
`put_snapshot(SNAPSHOTS_BY_SLOT)` — on **every sync pass** (node_sync.rs ~L771), and **nothing ever prunes
it** (`delete_snapshot` exists but is never called — "eviction out of scope per PHASE4-N-K"). At the live
tip each pass admits ~1 block, so it writes a ~600 MB snapshot per block and never deletes → chain.db grows
~600 MB/block until ENOSPC. (The 2026-06-25 mithril-judge note flagged exactly this; still open.)

## The fix (two parts, both GREEN)

1. **Cadence-gate the E4 tip capture.** Capture only when `should_snapshot_after_block(tip.slot,
   last_block_no, SnapshotCadence, last_snapshot_slot)` (the existing cadence primitive), not every pass.
   This drops the tip-follow capture from per-block to per-N-blocks — also cutting the ~600 MB per-block
   `encode_snapshot` that is a large share of the tip CPU. The **boundary** capture (node_sync.rs ~L754)
   stays unconditional (once per epoch; the boundary chain-dep tick must be durable).
2. **Bounded retention.** After a capture, keep the **oldest** snapshot (the bootstrap anchor — an always-≤
   fallback for `nearest_le`) plus the **latest M**; `delete_snapshot` the rest. M·cadence ≥ k (the rollback
   bound) so any rollback within k (and any warm-start at tip) finds a kept snapshot ≤ its target.

## Why it is correct (the read-side contract is preserved)

`nearest_le(target)` returns the largest snapshot ≤ target, then `materialize_rolled_back_state` replays
forward `(snapshot, target]` over the durable ChainDb blocks — which are unchanged. So a sparser/pruned
snapshot set only lengthens the forward replay; it never changes the reconstructed state (T-REC-05: the
recovered fingerprint must still equal the WAL-tail fp — the WAL is the admission authority, the snapshot is
a replay accelerator). The R4c RSW fix (5e83aaaa) already makes the longer warm-start replay freeze the
candidate nonce correctly. Keeping the oldest + latest-M (M·cadence ≥ k) guarantees `nearest_le` never
returns `None` for any admissible target > the anchor, so rollback never regresses to `RollbackTooDeep`.

Chosen constants (preview k=432, cadence 100 blocks): M = 8 (latest-8 span 800 blocks ≥ k) → ~9 snapshots
retained (~5 GB, bounded) instead of unbounded. Venue-parametric where k is available; the bootstrap-anchor
fallback keeps deeper (mainnet k=2160) rollbacks correct (replay from the anchor) even if M under-covers.

## Acceptance

- **Disk bounded:** a sustained live follow holds a bounded snapshot count (chain.db stops growing
  unboundedly); no ENOSPC over the window.
- **Live tip still held**, keep-alive stable, no desync.
- **Warm restart still works** (recovers the durable tip via replay from the nearest retained snapshot).
- **Rollback still materializes** (nearest retained snapshot ≤ target; no `RollbackTooDeep` within k) —
  guarded by a unit test over the retention policy + the existing rollback/warm-start proofs unchanged in
  outcome (byte-identical authority; only the snapshot set differs).
- **Forbidden paths clean; no BLUE semantic change.**
- (Bonus) tip CPU drops materially from dropping the per-block encode (feeds LIVE-1a).
