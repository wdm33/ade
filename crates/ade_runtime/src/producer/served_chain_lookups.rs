// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types
// - Explicit state transitions only
// - Canonical serialization for all persisted/hashed data
//
// GREEN trait impls (PHASE4-N-G S5).
//
// Bridges `ServedChainSnapshot` (BLUE, ade_ledger) to the
// `ServedHeaderLookup` / `ServedRangeLookup` trait seams the BLUE
// server reducers (`ade_network::chain_sync::server`,
// `ade_network::block_fetch::server`) consume. The orphan rule
// prevents these impls from living in either ade_ledger or
// ade_network alone; they live here in ade_runtime which depends on
// both.
//
// Pure deterministic projections. No I/O. Header bytes come from
// `accepted_block_header_bytes` — the single canonical authority
// (DC-CONS-16, DC-CONS-18). No parallel splitter.

use ade_ledger::block_validity::{accepted_block_header_bytes, decode_block};
use ade_ledger::producer::ServedChainSnapshot;
use ade_network::block_fetch::server::ServedRangeLookup;
use ade_network::chain_sync::server::{HeaderProjection, ServedHeaderLookup};
use ade_network::codec::chain_sync::Point;
use ade_types::{Hash32, SlotNo};

/// Lookup adapter wrapping a borrowed `ServedChainSnapshot`. The
/// orchestrator (S6) constructs one of these per reducer call.
pub struct ServedChainLookups<'a> {
    pub snap: &'a ServedChainSnapshot,
}

impl<'a> ServedHeaderLookup for ServedChainLookups<'a> {
    fn next_after(&self, cursor: Option<(SlotNo, Hash32)>) -> Option<HeaderProjection> {
        let next = self
            .snap
            .iter_accepted()
            .find(|(s, h, _)| match &cursor {
                Some((c_s, c_h)) => (*s, *h) > (*c_s, c_h),
                None => true,
            })?;
        let header_bytes = accepted_block_header_bytes(next.2)
            .expect("snapshot AcceptedBlock projects (admit invariant)")
            .to_vec();
        let decoded = decode_block(next.2.as_bytes())
            .expect("snapshot AcceptedBlock decodes (admit invariant)");
        Some(HeaderProjection {
            slot: next.0,
            hash: next.1.clone(),
            block_no: decoded.header_input.block_no.0,
            header_bytes,
        })
    }

    fn intersect(&self, points: &[Point]) -> Option<(SlotNo, Hash32)> {
        for p in points {
            if let Point::Block { slot, hash } = p {
                if self.snap.block_at(*slot, hash).is_some() {
                    return Some((*slot, hash.clone()));
                }
            }
        }
        None
    }

    fn tip(&self) -> Option<(SlotNo, Hash32, u64)> {
        let mut last: Option<(SlotNo, Hash32, &ade_ledger::producer::AcceptedBlock)> = None;
        for (s, h, b) in self.snap.iter_accepted() {
            last = Some((s, h.clone(), b));
        }
        let (s, h, b) = last?;
        let decoded = decode_block(b.as_bytes())
            .expect("snapshot AcceptedBlock decodes (admit invariant)");
        Some((s, h, decoded.header_input.block_no.0))
    }
}

impl<'a> ServedRangeLookup for ServedChainLookups<'a> {
    fn range_bytes(
        &self,
        from: (SlotNo, Hash32),
        to: (SlotNo, Hash32),
    ) -> Vec<(SlotNo, Hash32, Vec<u8>)> {
        self.snap
            .range_bytes(from, to)
            .map(|(s, h, b)| (s, h.clone(), b.to_vec()))
            .collect()
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
#[allow(clippy::expect_used)]
#[allow(clippy::panic)]
mod tests {
    use super::*;

    #[test]
    fn empty_snapshot_next_after_yields_none() {
        let snap = ServedChainSnapshot::new();
        let look = ServedChainLookups { snap: &snap };
        assert!(look.next_after(None).is_none());
    }

    #[test]
    fn empty_snapshot_intersect_yields_none() {
        let snap = ServedChainSnapshot::new();
        let look = ServedChainLookups { snap: &snap };
        assert!(look.intersect(&[]).is_none());
    }

    #[test]
    fn empty_snapshot_tip_yields_none() {
        let snap = ServedChainSnapshot::new();
        let look = ServedChainLookups { snap: &snap };
        assert!(look.tip().is_none());
    }
}
