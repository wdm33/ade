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

/// Per-request serve range cap (PHASE4-N-AA, DC-SERVEMEM-01). A fixed, closed,
/// non-configurable defensive bound — symmetric with the receive-side
/// `MAX_WIRE_PUMP_LOOKAHEAD = 256` (DC-LIVEMEM-01). NOT a Cardano semantic
/// parameter; it may be tightened later, but no runtime option (CLI / env /
/// config) may disable it or set it unbounded.
const MAX_SERVE_RANGE_BLOCKS: usize = 256;

/// Closed internal outcome of a peer serve range read (DC-SERVEMEM-01). Every
/// non-`Served` variant encodes to the same wire `NoBlocks`, but the reason is
/// distinct for diagnostics + tests (cap-exceeded is NOT the same condition as a
/// genuinely empty window).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ServeRangeOutcome {
    /// In-range blocks (`<= MAX_SERVE_RANGE_BLOCKS`); the BLUE reducer's
    /// both-endpoints-present check then decides StartBatch/.../BatchDone.
    Served(Vec<(SlotNo, Hash32, Vec<u8>)>),
    /// No durable block lies in the requested `[from, to]` window.
    Empty,
    /// The request spans more than `MAX_SERVE_RANGE_BLOCKS` blocks — fail closed
    /// BEFORE any unbounded storage/CPU work (no decode, no serve).
    CapExceeded,
    /// A ChainDb read error, or a durable block that does not decode — serve
    /// nothing (never torn / partial / unauthenticated bytes).
    ReadError,
}

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

    /// Read the durable blocks in the `[from, to]` `(slot, hash)` window for a
    /// peer BlockFetch RequestRange, BOUNDED to `MAX_SERVE_RANGE_BLOCKS`
    /// (DC-SERVEMEM-01). The per-request cap is enforced via S1's hash-free
    /// `range_bytes_capped` BEFORE any block is decoded or served, so an
    /// oversized request fails closed (`CapExceeded`) with no unbounded
    /// storage/CPU work and no per-block `SLOT_BY_HASH` scan. The hash is then
    /// derived from each block's own bytes via the single BLUE `decode_block`
    /// authority (== the stored hash; no second hash authority). Returns the
    /// structured outcome; [`ServedRangeLookup::range_bytes`] maps every
    /// non-`Served` outcome to an empty `Vec` (→ reducer `NoBlocks`).
    pub fn serve_range(
        &self,
        from: (SlotNo, Hash32),
        to: (SlotNo, Hash32),
    ) -> ServeRangeOutcome {
        let capped = match self
            .chaindb
            .range_bytes_capped(from.0, to.0, MAX_SERVE_RANGE_BLOCKS)
        {
            Ok(c) => c,
            Err(_) => return ServeRangeOutcome::ReadError,
        };
        if capped.truncated {
            // The slot range holds more than the cap — fail closed before
            // decoding or serving anything.
            return ServeRangeOutcome::CapExceeded;
        }
        let mut out: Vec<(SlotNo, Hash32, Vec<u8>)> = Vec::new();
        for (slot, bytes) in capped.blocks {
            // Derive the hash from the bytes via the single BLUE decode
            // authority (no second hash authority, no SLOT_BY_HASH scan).
            // Undecodable bytes fail closed — the serve never emits a block it
            // cannot authenticate.
            let hash = match decode_block(&bytes) {
                Ok(d) => d.block_hash,
                Err(_) => return ServeRangeOutcome::ReadError,
            };
            let key = (slot, hash.clone());
            if key >= from && key <= to {
                out.push((slot, hash, bytes));
            }
        }
        if out.is_empty() {
            ServeRangeOutcome::Empty
        } else {
            ServeRangeOutcome::Served(out)
        }
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
        // Bounded read (DC-SERVEMEM-01): the durable chain is slot-ascending,
        // linear, extend-only (<= 1 block per slot), so the first key strictly
        // past the cursor is within the first 2 blocks from the cursor slot —
        // the block AT the cursor slot (skipped: key == cursor) and the next.
        // Read that 2-block window via the S1 primitive — NO iter_from_slot, NO
        // SLOT_BY_HASH scan.
        let capped = self
            .chaindb
            .range_bytes_capped(from, SlotNo(u64::MAX), 2)
            .ok()?;
        for (slot, bytes) in capped.blocks {
            // Reuse the single decode authority over the raw durable bytes —
            // derive the hash, no AcceptedBlock reconstruction.
            let decoded = decode_block(&bytes).ok()?;
            let hash = decoded.block_hash.clone();
            let key = (slot, hash.clone());
            let past_cursor = match &cursor {
                Some(c) => key > *c,
                None => true,
            };
            if !past_cursor {
                continue;
            }
            let header = block_header_bytes(&bytes).ok()?.to_vec();
            return Some(HeaderProjection {
                slot,
                hash,
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
        // Bounded tip (DC-SERVEMEM-01): the highest-slot block's bytes via the
        // S1 O(log N) primitive — NO chaindb.tip() O(N) iteration + hash scan,
        // NO get_block_by_hash. Derive the hash + block_no from the bytes via
        // the single decode authority.
        let (slot, bytes) = self.chaindb.last_block_bytes().ok()??;
        let decoded = decode_block(&bytes).ok()?;
        Some((slot, decoded.block_hash, decoded.header_input.block_no.0))
    }
}

impl<'a> ServedRangeLookup for ChainDbServedSource<'a> {
    fn range_bytes(
        &self,
        from: (SlotNo, Hash32),
        to: (SlotNo, Hash32),
    ) -> Vec<(SlotNo, Hash32, Vec<u8>)> {
        // Bounded + fail-closed (DC-SERVEMEM-01). `serve_range` reads at most
        // MAX_SERVE_RANGE_BLOCKS via the S1 hash-free primitive, derives the
        // hash from the bytes, and fails closed on an oversized range. Every
        // non-`Served` outcome maps to an empty Vec, so the BLUE reducer's
        // both-endpoints-present check (CN-SNAPSHOT-02) emits NoBlocks. Within
        // the cap the in-range `stored.bytes` are served verbatim (no re-encode
        // — DC-CONS-17), identical to the pre-cap projection.
        match self.serve_range(from, to) {
            ServeRangeOutcome::Served(v) => v,
            ServeRangeOutcome::Empty
            | ServeRangeOutcome::CapExceeded
            | ServeRangeOutcome::ReadError => Vec::new(),
        }
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

    // PHASE4-N-AA S2 — bounded serve range + fail-closed (DC-SERVEMEM-01).
    // These use synthetic StoredBlocks: the CAP outcome (range_bytes_capped
    // .truncated) and the EMPTY window are decided BEFORE any decode, and an
    // undecodable in-range block fails closed (ReadError) — so they pin the
    // bound + fail-closed reasons without a real corpus. Within-cap serving of
    // REAL decodable blocks (Served, byte-identical, derived hash == stored
    // hash) is covered end-to-end by the ade_node serve integration tests
    // (served_view_projects_durable_chain / follower_fetches_coherent_history),
    // which forge and durably admit real blocks. The pre-S2 synthetic
    // range-window unit tests were removed: range_bytes now derives the hash via
    // decode_block, so the in-window semantics are pinned by S1's
    // range_bytes_capped_* contract tests (no decode) + those integration tests.
    fn synth(slot: u64, tag: u8) -> StoredBlock {
        StoredBlock {
            slot: SlotNo(slot),
            hash: Hash32([tag; 32]),
            bytes: vec![tag; 4],
        }
    }

    #[test]
    fn serve_range_over_cap_fails_closed() {
        // A slot range holding more than MAX_SERVE_RANGE_BLOCKS blocks fails
        // closed (CapExceeded) — decided by the S1 bound BEFORE any decode, so
        // synthetic bytes suffice. range_bytes maps it to an empty Vec.
        let db = InMemoryChainDb::new();
        for s in 1..=300u64 {
            db.put_block(&synth(s, 0xAB)).unwrap();
        }
        let src = ChainDbServedSource::new(&db);
        let from = (SlotNo(1), Hash32([0xAB; 32]));
        let to = (SlotNo(300), Hash32([0xAB; 32]));
        assert_eq!(
            src.serve_range(from.clone(), to.clone()),
            ServeRangeOutcome::CapExceeded,
            "300 blocks > cap 256 -> CapExceeded (decided before decode)"
        );
        assert!(
            src.range_bytes(from, to).is_empty(),
            "oversized range fails closed -> empty -> reducer NoBlocks"
        );
    }

    #[test]
    fn serve_range_empty_window_is_empty_not_capexceeded() {
        // An out-of-chain window is Empty (a distinct internal reason from
        // CapExceeded), even though both encode to NoBlocks.
        let db = InMemoryChainDb::new();
        db.put_block(&synth(10, 0x10)).unwrap();
        db.put_block(&synth(20, 0x20)).unwrap();
        let src = ChainDbServedSource::new(&db);
        assert_eq!(
            src.serve_range(
                (SlotNo(100), Hash32([0u8; 32])),
                (SlotNo(200), Hash32([0xff; 32])),
            ),
            ServeRangeOutcome::Empty,
            "no durable block in [100,200] -> Empty, not CapExceeded"
        );
    }

    #[test]
    fn serve_range_undecodable_in_range_fails_closed() {
        // A durable block whose bytes do not decode fails closed (ReadError):
        // the serve never emits a block it cannot authenticate via decode_block
        // (no garbage, no stored-hash shortcut).
        let db = InMemoryChainDb::new();
        db.put_block(&synth(10, 0x10)).unwrap(); // vec![0x10; 4] does not decode
        let src = ChainDbServedSource::new(&db);
        let pt = (SlotNo(10), Hash32([0x10; 32]));
        assert_eq!(
            src.serve_range(pt.clone(), pt.clone()),
            ServeRangeOutcome::ReadError,
            "undecodable in-range block -> ReadError (fail closed)"
        );
        assert!(src.range_bytes(pt.clone(), pt).is_empty());
    }

    #[test]
    fn serve_range_inverted_range_fails_closed() {
        // A peer controls both endpoints; an inverted (slot) range fails closed
        // (Empty) on the serve path and NEVER panics (the InMemory primitive is
        // guarded against the BTreeMap::range start > end panic). DC-SERVEMEM-01.
        let db = InMemoryChainDb::new();
        db.put_block(&synth(10, 0x10)).unwrap();
        db.put_block(&synth(20, 0x20)).unwrap();
        let src = ChainDbServedSource::new(&db);
        let from = (SlotNo(20), Hash32([0x20; 32]));
        let to = (SlotNo(10), Hash32([0x10; 32]));
        assert_eq!(
            src.serve_range(from.clone(), to.clone()),
            ServeRangeOutcome::Empty,
            "inverted range -> Empty (fail closed, no panic)"
        );
        assert!(src.range_bytes(from, to).is_empty());
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
