# PHASE4-N-AE — invariants sketch (Slice A: Recover→Serve Continuity + Forge-on-Followed-Tip Gate)

> IDD gate 1 (`/invariants`) for **Slice A** scoped in
> `docs/planning/c2-local-discovered-gaps.md` (Gap 2). Closes the **2b serve-continuity**
> crux + adds the **2a forge-on-followed-tip** admission gate, so a Haskell relay can
> intersect, roll forward, validate, and adopt Ade's forged successor on a recovered/followed
> non-Origin tip. **Non-goal:** Gap 1 (multi-producer fork-choice) — Slice B, separate.

## Why (grounded from the 2026-06-06 recover-far-behind run)

- Ade recovers + forges (#1–#7 proven). The recover-far-behind run **eliminated** the
  receive-side `BlockNoOutOfOrder` (Ade caught up + forged `BlockNo 22 = followed-tip(21)+1`)
  but node2-relay **still rejected** Ade's *served* chain: `UnexpectedBlockNo (BlockNo 22)
  (BlockNo 0)` — the peer intersected only at an early point and saw the forged block
  presented as if it followed genesis.
- **Root cause (grounded):** `ade_node::admission::seed_to_snapshot` persists a *ledger
  **snapshot*** at the anchor slot (via `PersistentSnapshotCache::capture`), **not servable
  `StoredBlock` bytes**. `ChainDbServedSource` serves only `StoredBlock.bytes` (writers:
  `pump_block` / `bootstrap_initial_state`). So the recovered anchor is **not a
  peer-intersectable servable block**, and the served chain is not a continuous lineage from
  a shared point → followed blocks → forged successor.

## Invariants this slice must establish (candidate registry rules)

1. **Recover→serve servable lineage (TRUE).** After recover-from-non-Origin-tip + follow,
   the durable served chain is a **continuous, peer-intersectable servable lineage** — a
   Haskell peer can `FindIntersect` at the recovered/followed tip (a real shared block hash)
   and roll forward onto Ade's forged successor. The recovered anchor must be representable
   as a servable chain head (or the serve must project a lineage that intersects the peer's
   chain), not only a ledger snapshot. *(supersedes the snapshot-only recover for the serve
   surface; relates to DC-NODE-13 serve-as-projection, CN-CONS-07 serve provenance,
   DC-CONS-23 extend-only, DC-NODE-12 pump_block.)*

2. **Forge-on-followed-tip admissibility gate (TRUE / derived).** Forge is admissible **iff**
   `local_selected_tip.hash == followed_peer_tip.hash` **and**
   `local_selected_tip.block_no == followed_peer_tip.block_no` **and**
   `served_chain_can_intersect_at(local_selected_tip)`. Mechanically enforced — **not** by
   timing/window size. *(relates to CN-FORGE-*, DC-CONS-18; strengthens the forge path.)*

3. **Canonical forged-parent relation (TRUE).** The forged successor's **parent hash
   byte-equals** the followed peer tip hash, and `block_no == followed_tip.block_no + 1`.
   Parent identity is the **canonical hash**, never inferred from block number alone.

4. **Structured refusal when not caught up (TRUE / derived).** If invariant 2 is false, the
   producer returns a typed `ForgeRefused::NotCaughtUp { local_tip, peer_tip, reason }` and
   **does not forge** — fail closed. **No** log-string-only refusal.

5. **Replay determinism (TRUE).** Same recovered anchor + same followed canonical blocks →
   **byte-identical** served chain + same forged successor (no wall-clock / arrival-order /
   scheduler dependence). *(IDD Part IV; relates to T-REC-05 replay-equivalence.)*

## TCB boundary (from the user's Slice A spec)

- **BLUE:** selected-tip continuity rules; forge admissibility check; structured refusal
  when not caught up; canonical parent-hash / block-no relation.
- **GREEN:** correlation manifest; test harness; relay-adoption evidence parser.
- **RED:** socket serving; cardano-testnet venue; process control; key-loading/signing shell.

## Tier mapping

| Requirement | Tier |
|---|---|
| Same recovered/followed inputs → same served chain | true |
| Forge only from selected/followed canonical tip | true / derived |
| Haskell peer can intersect + roll forward onto Ade block | derived / bounty |
| Manifest proves forged parent == relay tip hash AND forged == adopted | bounty / release |
| Venue orchestration recipe | operational |

## Slice-entry proof obligations (verify before/at implementation — per proof discipline)

- **PO1 [GROUNDED]:** `seed_to_snapshot` writes a *snapshot*, not servable blocks → recovered
  anchor is not a servable head.
- **PO2:** Does `pump_block` during follow durably admit the followed blocks as **servable**,
  linked to the (non-servable) anchor? (Evidence: the served chain presented only the forged
  block, so the followed 9–21 were not served — determine why: link-to-snapshot-base failure,
  or admitted-but-not-projected.)
- **PO3:** Exact mechanism of the served block appearing at "slot 8 / BlockNo 22 / expected
  0" — the forge's slot assignment + the serve projection from a snapshot base.
- **PO4:** Whether the forge builds on the followed tip (BlockNo 22 ⇒ likely yes) while the
  served lineage is broken at the snapshot base (confirms invariants 1 vs 2 are distinct).

## Acceptance criteria (Slice A complete only when CI/harness shows)

1. Recover from non-Origin anchor T−k. 2. Follow Haskell relay to selected tip T.
3. Forge attempt before caught-up is **refused structurally** (typed, not a log line).
4. Once caught-up, Ade forges T+1. 5. **Forged block parent hash byte-equals relay tip hash T**
(recorded — this is the closure proof for the `UnexpectedBlockNo` class). 6. Ade serves an
**intersectable** chain from T through T+1. 7. node2/node3 **adopt** T+1. 8. Manifest records:
recovered anchor; followed tip hash/block; forged parent hash; forged block hash; adopted
block hash; **forged hash == adopted hash**.

## Hard prohibitions (do not merge if any hold)

forge from a stale recovered tip · served chain omits the followed peer tip · parent hash
inferred from block number only · Haskell adoption assumed from local forge success · refusal
only a log line · correctness depends on timing/window size · code relies on node2/node3
being stopped at a lucky instant.

## Next IDD steps

`/cluster-doc PHASE4-N-AE` → `/slice-doc PHASE4-N-AE.A` (commit the authority doc standalone
first) → `/implement-slice` (delegate to `slice-implementer`, clean context) → `/commit-slice`.
Then Slice B (Gap 1).
