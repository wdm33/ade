// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types
// - Explicit state transitions only
// - Canonical serialization for all persisted/hashed data

//! RED durable-chain serve projection (PHASE4-N-U S3, DC-NODE-13).
//!
//! [`ChainDbServedSource`] projects the durable ChainDb through the BLUE
//! serve reducers' read seams (`ServedHeaderLookup` /
//! `ServedRangeLookup`), so the `--mode node` serve path serves the
//! DURABLE adopted chain rather than an in-memory accumulator. It is the
//! serve-as-projection that supersedes the PHASE4-N-F-G-R monotone
//! serve-gate workaround (DC-NODE-11): the durable chain is extend-only
//! (DC-CONS-23), so it holds exactly one block 0 and a follower fetches
//! coherent history A→B (never B without A) — and serving survives
//! restart, because the durable ChainDb is recovered (T-REC-05) whereas
//! the accumulator was not.
//!
//! Provenance (CN-CONS-07 serve clause): every byte this source yields is
//! `StoredBlock.bytes` read from the durable ChainDb, whose sole
//! production writers are `pump_block` (DC-NODE-12) and the validated
//! warm-start / genesis replay `bootstrap_initial_state`. Serving cannot
//! leak a byte that did not clear `block_validity`. This source:
//!   - serves `stored.bytes` VERBATIM (no re-encode — DC-CONS-17);
//!   - reuses the single `block_header_bytes` header-projection authority
//!     (DC-CONS-18) and `decode_block` — NO parallel splitter, NO
//!     `AcceptedBlock` reconstruction;
//!   - is READ-ONLY: it advances no tip, admits nothing, derives no
//!     verdict.
//!
//! On a `ChainDbError` a lookup yields `None` / empty (serve nothing this
//! round — availability, never wrong or partial bytes). The serve task is
//! best-effort availability over an authoritative store, not an authority.

use ade_ledger::block_validity::{block_header_bytes, decode_block};
use ade_network::block_fetch::server::ServedRangeLookup;
use ade_network::chain_sync::server::{HeaderProjection, ServedHeaderLookup};
use ade_network::codec::chain_sync::Point;
use ade_types::{Hash32, SlotNo};

use crate::chaindb::ChainDb;

/// RED read-only projection of the durable ChainDb into the producer-side
/// serve read seams. Holds a borrowed `&dyn ChainDb`; the serve task
/// constructs one per dispatched frame. Cheap to build (a single
/// reference); all reads go straight to the durable store.
pub struct ChainDbServedSource<'a> {
    chaindb: &'a dyn ChainDb,
}

impl<'a> ChainDbServedSource<'a> {
    /// Wrap a borrowed durable ChainDb as a serve source.
    pub fn new(chaindb: &'a dyn ChainDb) -> Self {
        Self { chaindb }
    }
}

impl<'a> ServedHeaderLookup for ChainDbServedSource<'a> {
    fn next_after(&self, cursor: Option<(SlotNo, Hash32)>) -> Option<HeaderProjection> {
        // Smallest durable key (slot, hash) strictly greater than the
        // cursor. The durable chain is slot-ascending and linear
        // (extend-only), so iterating from the cursor slot and taking the
        // first key past the cursor matches the BTreeMap `next_after`
        // semantics the accumulator source provided.
        let from = match &cursor {
            Some((s, _)) => *s,
            None => SlotNo(0),
        };
        let iter = self.chaindb.iter_from_slot(from).ok()?;
        for item in iter {
            // A read error mid-iteration: serve nothing this round (never
            // wrong/partial bytes).
            let stored = item.ok()?;
            let key = (stored.slot, stored.hash.clone());
            let past_cursor = match &cursor {
                Some(c) => key > *c,
                None => true,
            };
            if !past_cursor {
                continue;
            }
            // Reuse the single header-projection + decode authorities over
            // the raw durable bytes — no AcceptedBlock reconstruction.
            let decoded = decode_block(&stored.bytes).ok()?;
            let header = block_header_bytes(&stored.bytes).ok()?.to_vec();
            return Some(HeaderProjection {
                slot: stored.slot,
                hash: stored.hash,
                block_no: decoded.header_input.block_no.0,
                era: decoded.era,
                header_bytes: header,
            });
        }
        None
    }

    fn intersect(&self, points: &[Point]) -> Option<(SlotNo, Hash32)> {
        // First listed point that is on the durable chain. Origin is
        // resolved by the BLUE reducer (the universal ancestor), not here.
        for p in points {
            if let Point::Block { slot, hash } = p {
                if let Ok(Some(stored)) = self.chaindb.get_block_by_hash(hash) {
                    if stored.slot == *slot {
                        return Some((*slot, hash.clone()));
                    }
                }
            }
        }
        None
    }

    fn tip(&self) -> Option<(SlotNo, Hash32, u64)> {
        let tip = self.chaindb.tip().ok()??;
        // ChainTip carries no block_no — derive it from the stored block
        // via the single decode authority.
        let stored = self.chaindb.get_block_by_hash(&tip.hash).ok()??;
        let decoded = decode_block(&stored.bytes).ok()?;
        Some((tip.slot, tip.hash, decoded.header_input.block_no.0))
    }
}

impl<'a> ServedRangeLookup for ChainDbServedSource<'a> {
    fn range_bytes(
        &self,
        from: (SlotNo, Hash32),
        to: (SlotNo, Hash32),
    ) -> Vec<(SlotNo, Hash32, Vec<u8>)> {
        // Durable blocks whose (slot, hash) key lies in [from, to]
        // (tuple-lexicographic), ascending — replicating the
        // ServedChainSnapshot BTreeMap-range semantics over the linear
        // durable chain. The BLUE reducer's both-endpoints-present check
        // (CN-SNAPSHOT-02) then decides StartBatch/.../BatchDone vs
        // NoBlocks; this source only supplies the in-range entries.
        // `stored.bytes` are served verbatim (no re-encode — DC-CONS-17).
        let iter = match self.chaindb.iter_from_slot(from.0) {
            Ok(it) => it,
            Err(_) => return Vec::new(),
        };
        let mut out: Vec<(SlotNo, Hash32, Vec<u8>)> = Vec::new();
        for item in iter {
            let stored = match item {
                Ok(s) => s,
                // A read error: serve nothing (never a torn/partial range).
                Err(_) => return Vec::new(),
            };
            if stored.slot > to.0 {
                break;
            }
            let key = (stored.slot, stored.hash.clone());
            if key >= from && key <= to {
                out.push((stored.slot, stored.hash, stored.bytes));
            }
        }
        out
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
#[allow(clippy::expect_used)]
#[allow(clippy::panic)]
mod tests {
    use super::*;
    use crate::chaindb::{InMemoryChainDb, StoredBlock};

    // The projection is exercised end-to-end (forge → durable admit →
    // serve) by the ade_node integration tests
    // (`served_view_projects_durable_chain`,
    // `follower_fetches_coherent_history_incl_ingested_predecessor`).
    // These unit tests pin the empty-store read paths + the range
    // boundary semantics with synthetic StoredBlocks; they do not need a
    // real corpus block because they assert the projection's structural
    // contract, not header/decode projection (covered in the integration
    // tests with real bytes).

    #[test]
    fn empty_chaindb_yields_no_tip_no_next_no_range() {
        let db = InMemoryChainDb::new();
        let src = ChainDbServedSource::new(&db);
        assert!(src.tip().is_none());
        assert!(src.next_after(None).is_none());
        assert!(src
            .range_bytes((SlotNo(0), Hash32([0u8; 32])), (SlotNo(100), Hash32([0xff; 32])))
            .is_empty());
        assert!(src.intersect(&[Point::Block { slot: SlotNo(1), hash: Hash32([1u8; 32]) }]).is_none());
    }

    #[test]
    fn range_bytes_collects_inclusive_window_in_slot_order() {
        // Synthetic stored blocks (raw bytes need not decode for the range
        // contract — range_bytes never decodes; it serves stored.bytes).
        let db = InMemoryChainDb::new();
        let mk = |slot: u64, tag: u8| StoredBlock {
            slot: SlotNo(slot),
            hash: Hash32([tag; 32]),
            bytes: vec![tag; 4],
        };
        db.put_block(&mk(10, 0x10)).unwrap();
        db.put_block(&mk(20, 0x20)).unwrap();
        db.put_block(&mk(30, 0x30)).unwrap();
        let src = ChainDbServedSource::new(&db);
        let got = src.range_bytes(
            (SlotNo(10), Hash32([0x10; 32])),
            (SlotNo(20), Hash32([0x20; 32])),
        );
        assert_eq!(got.len(), 2, "inclusive [10,20] window");
        assert_eq!(got[0].0, SlotNo(10));
        assert_eq!(got[1].0, SlotNo(20));
        assert_eq!(got[0].2, vec![0x10; 4], "serves stored.bytes verbatim");
    }

    #[test]
    fn range_bytes_excludes_out_of_window_and_stops_past_to() {
        let db = InMemoryChainDb::new();
        let mk = |slot: u64, tag: u8| StoredBlock {
            slot: SlotNo(slot),
            hash: Hash32([tag; 32]),
            bytes: vec![tag; 4],
        };
        db.put_block(&mk(10, 0x10)).unwrap();
        db.put_block(&mk(20, 0x20)).unwrap();
        db.put_block(&mk(30, 0x30)).unwrap();
        let src = ChainDbServedSource::new(&db);
        // Window [20,20] yields exactly the slot-20 block.
        let got = src.range_bytes(
            (SlotNo(20), Hash32([0x20; 32])),
            (SlotNo(20), Hash32([0x20; 32])),
        );
        assert_eq!(got.len(), 1);
        assert_eq!(got[0].0, SlotNo(20));
    }

    #[test]
    fn intersect_matches_only_a_durable_key() {
        let db = InMemoryChainDb::new();
        db.put_block(&StoredBlock {
            slot: SlotNo(20),
            hash: Hash32([0x20; 32]),
            bytes: vec![0x20; 4],
        })
        .unwrap();
        let src = ChainDbServedSource::new(&db);
        // Present key matches.
        assert_eq!(
            src.intersect(&[Point::Block { slot: SlotNo(20), hash: Hash32([0x20; 32]) }]),
            Some((SlotNo(20), Hash32([0x20; 32]))),
        );
        // Wrong hash at the right slot does not match (provenance-exact).
        assert!(src
            .intersect(&[Point::Block { slot: SlotNo(20), hash: Hash32([0x99; 32]) }])
            .is_none());
    }
}
