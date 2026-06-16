# Slice MEM-OPT-UTXO-DISK S2b-2c.0 — admission seam specification

> **Status:** Spec (doc + gate) — **the highest-risk step in S2b.** It composes three authorities that must NOT blur: RED storage I/O, BLUE validation, RED durable commit / WAL / checkpoint. This slice defines the seam + the atomicity/recovery rule BEFORE any live-path code (2c.1).
> **Prior:** S2b pre-resolve wiring (`96118302`).

## 1. The seam (the admission loop becomes)
1. Decode / receive candidate block.
2. **GREEN:** `collect_required_txins_block(block) -> BTreeSet<TxIn>`.
3. **RED:** `UtxoAnchor::resolve_required(required) -> BTreeMap<TxIn, TxOut>`.
4. **GREEN:** `WorkingSet::seed_required_from_anchor(resolved)`.
5. **BLUE:** validate/apply block against the `WorkingSet` (the resolved view only).
6. **GREEN:** produce delta — `spent_all`, `produced_all`, the new incremental fp, `post_fp`.
7. **RED:** durable admit (see §3).
8. Publish/admit visibility — ONLY after the durable commit succeeds.

**The load-bearing rule:** validation never calls redb; the redb commit never decides validation. (5) reads only in-memory resolved state; (7) writes only already-decided deltas.

## 2. Two durable substrates (why one physical transaction is impossible)
- **WAL** = `FileWalStore` — an append-only, fsync-durable **file** log; the **admit authority** (`AdmitBlock{prior_fp, block_hash, slot, verdict, post_fp}` — note: NO block bytes, NO UTxO delta).
- **chaindb redb** = block bytes (`BLOCKS_BY_SLOT`) + the **UTxO anchor table** + the **anchor-position** + `fp_version`. Each redb write-txn is atomic.

A file log and a redb database are different storage engines, so a single physical transaction cannot span both. The "prefer one transaction" option does not apply across WAL↔redb; **a deterministic recovery rule is required** (§3) — but each redb commit is itself one atomic txn.

## 3. Atomicity + recovery rule (NOT hand-waved)
### Ordering, per admitted block
- **7a.** Block bytes durable in redb (`put_block`) — **BEFORE** the WAL append. *(Invariant: bytes-before-admit, so recovery can re-validate a WAL-admitted block. 2c.1 verifies the existing node_lifecycle `put_block` already precedes admit; else 2c.1 adds it.)*
- **7b.** WAL append — the admit record (`AdmitBlock`).
- **7c.** Anchor commit — **ONE redb write-txn**: delete `spent_all` + insert `produced_all` + write `anchor_position{slot, block_hash, prior_fp, post_fp}` + `fp_version`. Atomic; no half-applied anchor.
- **8.** Publish visibility — only after 7c (UTxO durability).

### Recovery (deterministic roll-forward — reuses `replay_from_anchor`)
On restart the WAL is the authority. Compare the **anchor-position** (the block the anchor materialized up to + its `post_fp`) to the WAL tail:
- **anchor-position == WAL tail** and `post_fp` matches → consistent; proceed.
- **anchor-position BEHIND WAL tail** (crash after 7b, before/during 7c) → **roll forward**: `replay_from_anchor` the WAL entries beyond the anchor-position, using the durable block bytes (redb), re-deriving each block's UTxO delta + `post_fp`, committing each to the anchor (one redb txn per block). Verify each re-derived `post_fp` == the WAL entry's `post_fp`; mismatch → **fail closed** (corruption). Continue until anchor-position == WAL tail.
- **anchor-position AHEAD of WAL tail** → **IMPOSSIBLE** (7c commits only after 7b; the anchor never leads the WAL). If observed → **fail closed** (corruption).
- **`post_fp` mismatch** at a shared position → **fail closed** (corruption).

### Why no split-brain
A block is *admitted* ⟺ it is in the WAL. The anchor is a **derived materialization** of the WAL-replayed UTxO, committed strictly after the WAL append, so it is always ≤ the WAL and is rolled forward on restart. Neither "WAL admitted, anchor did not apply" (→ roll forward) nor "anchor applied, WAL did not admit" (→ impossible by order) can persist.

## 4. Required tests (before the owned-RSS re-measure)
1. valid block admitted through the resolved path == the old (full-UTxO) path,
2. missing resolved input fails closed BEFORE validation escapes to disk,
3. intra-block produced-then-spent remains valid,
4. torn commit cannot produce a half-admitted state (anchor txn atomicity + the recovery rule),
5. replay after restart matches a clean run (roll-forward == straight-through),
6. incremental fp and WAL `post_fp` match,
7. block-hash agreement unchanged,
8. no `UtxoAnchor` dependency in BLUE crates.

## 5. Hard prohibitions
- **No lazy disk lookup inside `UtxoStore`** — BLUE reads only the resolved `WorkingSet`.
- **No cache in this slice** unless strictly behind `resolve_required` and proven output-invariant (a separate later slice).
- **No admitting block visibility before UTxO anchor durability** (8 after 7c).
- **No `OP-MEM-02` claim** until after the post-integration owned-RSS run (2c.2).

## 6. Plan
- **2c.0 (this):** the seam spec + `ci_check_utxo_admission_seam.sh` (asserts the spec exists with the atomicity/recovery rule + the prohibitions). No live-path code.
- **2c.1a (DONE):** the storage-level recovery primitive — `AnchorPosition` (slot/block_hash/prior_fp/post_fp) stamped **atomically with the delta** in `commit_block` (one redb txn) + `read_position` + the pure `reconcile(position, wal)` decision (Consistent / RollForward / FailClosed: anchor-ahead-or-diverged). Provable in isolation; `ci_check_utxo_disk_anchor.sh`. No admission rewire.
- **2c.1b:** the admission rewire — decode → collect → resolve → WorkingSet → validate → `commit_block` → publish, with the roll-forward EXECUTION (reconcile → re-validate from durable bytes → commit) + the §4 live tests.
- **2c.2:** the owned-RSS re-measure (S0/S3 scenario) — the bounty win.
