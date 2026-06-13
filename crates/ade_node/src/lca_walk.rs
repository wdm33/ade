// Core Contract:
// - Deterministic: same inputs => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types
// - No durable mutation; read-only ChainDb lookups only

//! Last-common-ancestor fork-anchor walk (PHASE4-N-AO S7, `DC-NODE-38`).
//!
//! The live-geometry gap CE-AO-6 surfaced: a live competing branch is multi-block,
//! so the competing block's IMMEDIATE parent is an intermediate block on the
//! competing branch Ade never stored. The fork anchor is the **last common
//! ancestor (LCA)** — a durable `ChainDb`-stored block — reached by walking the
//! competing branch's preserved parent links.
//!
//! **The cache is NOT authority** — only an indexed memory of received, preserved
//! headers. Each entry self-binds (its map key == its own re-derived block hash)
//! or the branch fails closed; the cache may not become a stringly map of peer
//! claims. The durable LCA is authority ONLY when `ChainDb` confirms **slot AND
//! hash** (`DC-NODE-29`). The walk is k-bounded by **block depth** (traversed
//! header count), never slot distance — empty slots do not affect eligibility.

use std::collections::BTreeMap;

use ade_core::consensus::header_summary::HeaderInput;
use ade_runtime::chaindb::ChainDb;
use ade_types::shelley::block::PrevHash;
use ade_types::{Hash32, SlotNo};

/// A received competing-branch header, preserved for the LCA walk. Built from a
/// `decode_block` output, so `block_hash` is the re-derived block hash (the map
/// key) — the self-binding the walk verifies.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CachedHeader {
    pub header: HeaderInput,
    pub prev_hash: PrevHash,
    /// `decode_block(..).block_hash` — the re-derived block hash. MUST equal the
    /// map key (self-binding); a mismatch fails the branch closed.
    pub block_hash: Hash32,
}

/// Closed failure surface for the LCA walk. Every variant => the competing branch
/// is not selectable; the caller keeps the current chain (no durable mutation).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LcaError {
    /// Walked the preserved links to genesis without finding a durable stored
    /// ancestor (within k).
    NoDurableAncestorWithinK,
    /// An intermediate header is absent from the cache — the branch is incomplete
    /// (a gap); a complete LCA+1..=tip chain is required.
    BranchGap,
    /// The walk exceeded k traversed headers (block depth) before reaching the LCA.
    ExceededK,
    /// A cache entry's map key != its own re-derived block hash (or it was looked
    /// up under a hash it does not claim) — the cache is evidence, not peer-claim
    /// authority.
    CacheSelfBindingViolation,
}

/// The discovered fork anchor (durable LCA) + the complete intermediate header
/// chain (LCA+1 ..= competing tip, in order) ready for S2 `build_candidate_fragment`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LcaResult {
    pub anchor_slot: SlotNo,
    pub anchor_hash: Hash32,
    pub headers: Vec<HeaderInput>,
}

/// Walk the competing branch's preserved parent links back to the durable
/// last-common-ancestor. PURE-deterministic over the cache + read-only `ChainDb`.
///
/// From `start_hash` (the competing tip, already in `cache`), follow `prev_hash`:
/// at each step the link is the **LCA** iff `ChainDb` stores a block at that hash
/// (slot+hash bound, `DC-NODE-29`); otherwise it must be a cached intermediate
/// header (else `BranchGap`). k-bounds the **traversed header count** (block
/// depth); each visited entry's self-binding (`block_hash` == its map key) is
/// verified. Returns the LCA + the headers LCA+1..=tip in order.
pub fn walk_to_durable_lca<D: ChainDb + ?Sized>(
    cache: &BTreeMap<Hash32, CachedHeader>,
    retention: &BTreeMap<Hash32, CachedHeader>,
    start_hash: &Hash32,
    chaindb: &D,
    k: u64,
) -> Result<LcaResult, LcaError> {
    let mut headers: Vec<HeaderInput> = Vec::new();
    let mut cur_hash = start_hash.clone();
    let mut depth: u64 = 0;
    loop {
        // PHASE4-N-AO S13 (DC-NODE-40): on a per-peer-cache miss, consult the
        // rollback-retention EVIDENCE -- the blocks Ade itself rolled back during a
        // ForkChoiceWin adoption (admitted LinearExtend, so never in the competing-
        // only branch cache). This lets the walk traverse non-durable intermediate
        // headers until it reaches a real durable ancestor. The retention is
        // evidence only: it provides intermediate HOPS; the LCA anchor is still the
        // ChainDb durable slot+hash (below), never a retained block. Self-binding +
        // the k bound apply identically to retained entries.
        let entry = cache
            .get(&cur_hash)
            .or_else(|| retention.get(&cur_hash))
            .ok_or(LcaError::BranchGap)?;
        // Self-binding: the map key MUST equal the entry's own re-derived hash.
        if entry.block_hash != cur_hash {
            return Err(LcaError::CacheSelfBindingViolation);
        }
        // Block-depth bound: never traverse more than k headers above the LCA.
        if depth >= k {
            return Err(LcaError::ExceededK);
        }
        headers.push(entry.header.clone());
        depth += 1;
        let prev = match &entry.prev_hash {
            PrevHash::Block(h) => h.clone(),
            // Reached genesis without a durable stored ancestor.
            PrevHash::Genesis => return Err(LcaError::NoDurableAncestorWithinK),
        };
        // Is `prev` a DURABLE stored block? -> it is the LCA (slot+hash authority).
        match chaindb.get_block_by_hash(&prev) {
            Ok(Some(stored)) => {
                headers.reverse(); // LCA+1 ..= competing tip, in order
                return Ok(LcaResult {
                    anchor_slot: stored.slot,
                    anchor_hash: prev,
                    headers,
                });
            }
            // Not durable -> must be a cached intermediate header; step to it.
            Ok(None) => {
                cur_hash = prev;
            }
            // A store error: fail closed (do not select on an unreadable store).
            Err(_) => return Err(LcaError::BranchGap),
        }
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
#[allow(clippy::expect_used)]
#[allow(clippy::panic)]
mod tests {
    use super::*;
    use ade_core::consensus::header_summary::HeaderVrf;
    use ade_crypto::vrf::{VrfOutput, VrfProof, VrfVerificationKey};
    use ade_runtime::chaindb::{ChainDb, InMemoryChainDb, StoredBlock};
    use ade_types::{BlockNo, Hash28};

    fn h(b: u8) -> Hash32 {
        Hash32([b; 32])
    }

    fn header(slot: u64, block_no: u64) -> HeaderInput {
        HeaderInput {
            slot: SlotNo(slot),
            block_no: BlockNo(block_no),
            body_hash: Hash32([0x55; 32]),
            issuer_pool: Hash28([0xAA; 28]),
            op_cert_kes_period: 0,
            op_cert_counter: 1,
            vrf_vk: VrfVerificationKey([0u8; 32]),
            vrf: HeaderVrf::Praos {
                proof: VrfProof([0u8; 80]),
                output: VrfOutput([0u8; 64]),
            },
            kes: None,
        }
    }

    fn cached(block_hash: u8, prev: PrevHash, slot: u64, block_no: u64) -> CachedHeader {
        CachedHeader {
            header: header(slot, block_no),
            prev_hash: prev,
            block_hash: h(block_hash),
        }
    }

    /// A ChainDb with a stored LCA block at (slot, hash). The bytes are irrelevant
    /// to the walk (it uses get_block_by_hash slot+hash only).
    fn db_with_lca(lca_hash: u8, lca_slot: u64) -> InMemoryChainDb {
        let db = InMemoryChainDb::new();
        db.put_block(&StoredBlock {
            hash: h(lca_hash),
            slot: SlotNo(lca_slot),
            bytes: vec![lca_hash; 8],
        })
        .unwrap();
        db
    }

    #[test]
    fn one_block_fork_walks_in_one_step() {
        // B (hash 0x34) forks directly off the durable LCA (0x33). One header.
        let db = db_with_lca(0x33, 669);
        let mut cache = BTreeMap::new();
        cache.insert(h(0x34), cached(0x34, PrevHash::Block(h(0x33)), 670, 34));
        let r = walk_to_durable_lca(&cache, &BTreeMap::new(),&h(0x34), &db, 5).expect("1-deep walks");
        assert_eq!(r.anchor_hash, h(0x33));
        assert_eq!(r.anchor_slot, SlotNo(669));
        assert_eq!(r.headers.len(), 1);
        assert_eq!(r.headers[0].block_no, BlockNo(34));
    }

    #[test]
    fn multi_block_branch_walks_to_durable_lca() {
        // B(0x36) -> 0x35 -> 0x34 -> LCA(0x33, durable). Three intermediate headers.
        let db = db_with_lca(0x33, 669);
        let mut cache = BTreeMap::new();
        cache.insert(h(0x34), cached(0x34, PrevHash::Block(h(0x33)), 670, 34));
        cache.insert(h(0x35), cached(0x35, PrevHash::Block(h(0x34)), 672, 35));
        cache.insert(h(0x36), cached(0x36, PrevHash::Block(h(0x35)), 674, 36));
        let r = walk_to_durable_lca(&cache, &BTreeMap::new(),&h(0x36), &db, 5).expect("multi-block walks");
        assert_eq!(r.anchor_hash, h(0x33), "anchor is the durable LCA, not the immediate parent");
        // headers in order LCA+1 ..= tip: 34, 35, 36.
        let nos: Vec<u64> = r.headers.iter().map(|x| x.block_no.0).collect();
        assert_eq!(nos, vec![34, 35, 36]);
    }

    #[test]
    fn missing_intermediate_header_fails_closed() {
        // B(0x36) -> 0x35 (absent from the cache) -> ... : a gap.
        let db = db_with_lca(0x33, 669);
        let mut cache = BTreeMap::new();
        cache.insert(h(0x36), cached(0x36, PrevHash::Block(h(0x35)), 674, 36));
        // 0x35 NOT cached and NOT durable -> BranchGap.
        assert_eq!(
            walk_to_durable_lca(&cache, &BTreeMap::new(),&h(0x36), &db, 5),
            Err(LcaError::BranchGap)
        );
    }

    #[test]
    fn ancestor_older_than_k_fails_closed_block_depth() {
        // A 6-deep branch with k=5: 6 traversed headers > k -> ExceededK. The LCA
        // (0x30) is durable but too deep by BLOCK COUNT (slots irrelevant).
        let db = db_with_lca(0x30, 600);
        let mut cache = BTreeMap::new();
        // 0x31..0x36 chain down to the durable 0x30.
        cache.insert(h(0x31), cached(0x31, PrevHash::Block(h(0x30)), 601, 31));
        cache.insert(h(0x32), cached(0x32, PrevHash::Block(h(0x31)), 602, 32));
        cache.insert(h(0x33), cached(0x33, PrevHash::Block(h(0x32)), 603, 33));
        cache.insert(h(0x34), cached(0x34, PrevHash::Block(h(0x33)), 604, 34));
        cache.insert(h(0x35), cached(0x35, PrevHash::Block(h(0x34)), 605, 35));
        cache.insert(h(0x36), cached(0x36, PrevHash::Block(h(0x35)), 606, 36));
        // 6 headers above the LCA, k=5 -> ExceededK.
        assert_eq!(
            walk_to_durable_lca(&cache, &BTreeMap::new(),&h(0x36), &db, 5),
            Err(LcaError::ExceededK)
        );
        // The same branch with k=6 succeeds (6 <= 6) -- proving the bound is exact.
        let r = walk_to_durable_lca(&cache, &BTreeMap::new(),&h(0x36), &db, 6).expect("6 <= k=6 walks");
        assert_eq!(r.anchor_hash, h(0x30));
        assert_eq!(r.headers.len(), 6);
    }

    #[test]
    fn lying_parent_link_to_genesis_fails_closed() {
        // A header whose prev_hash is Genesis (no durable ancestor) -> fail closed.
        let db = db_with_lca(0x33, 669);
        let mut cache = BTreeMap::new();
        cache.insert(h(0x34), cached(0x34, PrevHash::Genesis, 670, 34));
        assert_eq!(
            walk_to_durable_lca(&cache, &BTreeMap::new(),&h(0x34), &db, 5),
            Err(LcaError::NoDurableAncestorWithinK)
        );
    }

    #[test]
    fn cache_self_binding_violation_fails_closed() {
        // An entry stored under key 0x34 but whose own block_hash claims 0x99 --
        // a corrupted / peer-claim cache must fail closed, not be trusted.
        let db = db_with_lca(0x33, 669);
        let mut cache = BTreeMap::new();
        let mut bad = cached(0x99, PrevHash::Block(h(0x33)), 670, 34); // block_hash=0x99
        bad.block_hash = h(0x99);
        cache.insert(h(0x34), bad); // ... but keyed under 0x34
        assert_eq!(
            walk_to_durable_lca(&cache, &BTreeMap::new(),&h(0x34), &db, 5),
            Err(LcaError::CacheSelfBindingViolation)
        );
    }

    #[test]
    fn walk_is_deterministic() {
        let db = db_with_lca(0x33, 669);
        let mut cache = BTreeMap::new();
        cache.insert(h(0x34), cached(0x34, PrevHash::Block(h(0x33)), 670, 34));
        cache.insert(h(0x35), cached(0x35, PrevHash::Block(h(0x34)), 672, 35));
        assert_eq!(
            walk_to_durable_lca(&cache, &BTreeMap::new(),&h(0x35), &db, 5),
            walk_to_durable_lca(&cache, &BTreeMap::new(),&h(0x35), &db, 5)
        );
    }

    #[test]
    fn arrival_order_permutation_walks_identical() {
        // The branch cache is keyed by re-derived block hash (a BTreeMap), so the
        // walk result is INDEPENDENT of the order competing blocks were received /
        // inserted -- the arrival-order-independence the live dispatch needs
        // (CN-CONS-01 at the LCA-walk layer). Build the same 3-block branch in two
        // insertion orders and assert byte-identical LCA + headers.
        let db = db_with_lca(0x33, 669);
        let entries = [
            (0x34u8, cached(0x34, PrevHash::Block(h(0x33)), 670, 34)),
            (0x35u8, cached(0x35, PrevHash::Block(h(0x34)), 672, 35)),
            (0x36u8, cached(0x36, PrevHash::Block(h(0x35)), 674, 36)),
        ];
        let mut forward = BTreeMap::new();
        for (k, v) in entries.iter() {
            forward.insert(h(*k), v.clone());
        }
        let mut reverse = BTreeMap::new();
        for (k, v) in entries.iter().rev() {
            reverse.insert(h(*k), v.clone());
        }
        let a = walk_to_durable_lca(&forward, &BTreeMap::new(), &h(0x36), &db, 5).expect("forward");
        let b = walk_to_durable_lca(&reverse, &BTreeMap::new(), &h(0x36), &db, 5).expect("reverse");
        assert_eq!(a, b, "walk is arrival-order independent");
        assert_eq!(a.anchor_hash, h(0x33));
        assert_eq!(
            a.headers.iter().map(|x| x.block_no.0).collect::<Vec<_>>(),
            vec![34, 35, 36]
        );
    }

    // ---- PHASE4-N-AO S13 (DC-NODE-40): rolled-back branch evidence retention ----

    #[test]
    fn rollback_retains_removed_blocks_for_lca_walk() {
        // The competing tip c(0x36) is in the per-peer CACHE; its bridge b1(0x34) +
        // b2(0x35) -- the blocks Ade ROLLED BACK -- are in the RETENTION, not the
        // cache. The walk must traverse the retention to reach the durable LCA(0x33),
        // so the competing branch is evaluable (no false BranchGap over-fire).
        let db = db_with_lca(0x33, 669);
        let mut cache = BTreeMap::new();
        cache.insert(h(0x36), cached(0x36, PrevHash::Block(h(0x35)), 674, 36));
        let mut retention = BTreeMap::new();
        retention.insert(h(0x35), cached(0x35, PrevHash::Block(h(0x34)), 672, 35));
        retention.insert(h(0x34), cached(0x34, PrevHash::Block(h(0x33)), 670, 34));
        let r = walk_to_durable_lca(&cache, &retention, &h(0x36), &db, 5)
            .expect("retention bridges the rolled-back gap");
        assert_eq!(r.anchor_hash, h(0x33), "reaches the durable LCA via retained bridges");
        assert_eq!(
            r.headers.iter().map(|x| x.block_no.0).collect::<Vec<_>>(),
            vec![34, 35, 36]
        );
    }

    #[test]
    fn retained_blocks_are_not_anchors() {
        // b1(0x34) is in RETENTION (prev = the durable 0x33). The walk must NOT stop
        // at the retained b1 as the LCA -- the anchor is ChainDb-durable ONLY, so it
        // continues to 0x33.
        let db = db_with_lca(0x33, 669);
        let mut cache = BTreeMap::new();
        cache.insert(h(0x35), cached(0x35, PrevHash::Block(h(0x34)), 672, 35));
        let mut retention = BTreeMap::new();
        retention.insert(h(0x34), cached(0x34, PrevHash::Block(h(0x33)), 670, 34));
        let r = walk_to_durable_lca(&cache, &retention, &h(0x35), &db, 5).expect("walks");
        assert_eq!(r.anchor_hash, h(0x33), "anchor is the DURABLE block");
        assert_ne!(r.anchor_hash, h(0x34), "the retained b1 is NEVER the anchor");
    }

    #[test]
    fn retained_blocks_are_k_bounded() {
        // A bridge entirely in retention: 6 headers above the durable LCA(0x30), k=5
        // -> ExceededK. Retention does not let the walk exceed the block-depth bound.
        let db = db_with_lca(0x30, 600);
        let mut cache = BTreeMap::new();
        cache.insert(h(0x36), cached(0x36, PrevHash::Block(h(0x35)), 606, 36));
        let mut retention = BTreeMap::new();
        retention.insert(h(0x35), cached(0x35, PrevHash::Block(h(0x34)), 605, 35));
        retention.insert(h(0x34), cached(0x34, PrevHash::Block(h(0x33)), 604, 34));
        retention.insert(h(0x33), cached(0x33, PrevHash::Block(h(0x32)), 603, 33));
        retention.insert(h(0x32), cached(0x32, PrevHash::Block(h(0x31)), 602, 32));
        retention.insert(h(0x31), cached(0x31, PrevHash::Block(h(0x30)), 601, 31));
        assert_eq!(
            walk_to_durable_lca(&cache, &retention, &h(0x36), &db, 5),
            Err(LcaError::ExceededK)
        );
    }

    #[test]
    fn retained_block_hash_self_binds() {
        // A retention entry mis-keyed: stored under 0x34 but its block_hash is 0x99.
        // The walk's self-binding check rejects it (CacheSelfBindingViolation) -- a
        // retained block is evidence, not a peer-claim free pass.
        let db = db_with_lca(0x33, 669);
        let mut cache = BTreeMap::new();
        cache.insert(h(0x35), cached(0x35, PrevHash::Block(h(0x34)), 672, 35));
        let mut retention = BTreeMap::new();
        retention.insert(h(0x34), cached(0x99, PrevHash::Block(h(0x33)), 670, 34));
        assert_eq!(
            walk_to_durable_lca(&cache, &retention, &h(0x35), &db, 5),
            Err(LcaError::CacheSelfBindingViolation)
        );
    }

    #[test]
    fn genuine_gap_still_missing_bridge() {
        // The bridge 0x35 is in NEITHER cache NOR retention NOR durable -> BranchGap.
        // The DC-NODE-39 fail-closed BranchGap path is preserved: retention does NOT
        // paper over a genuine gap (the dispatch maps this LcaError to a structured
        // fail-closed hold -- the walk itself never names that surface).
        let db = db_with_lca(0x33, 669);
        let mut cache = BTreeMap::new();
        cache.insert(h(0x36), cached(0x36, PrevHash::Block(h(0x35)), 674, 36));
        let retention = BTreeMap::new(); // empty -- 0x35 is nowhere
        assert_eq!(
            walk_to_durable_lca(&cache, &retention, &h(0x36), &db, 5),
            Err(LcaError::BranchGap)
        );
    }
}
