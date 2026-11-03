// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types
// - Explicit state transitions only
// - Canonical serialization for all persisted/hashed data

//! BLUE chain-sync intersection authority (SLICE LV-16, DC-NODE-48 / T-REC-07).
//!
//! A recovering node asks a peer for a common point with `FindIntersect`. Which
//! points it offers, and what the peer's answer means, are decided here, from a
//! validated durable ancestry. The peer's reply is a captured input: it names one
//! of the offered points or says none is on its chain. It is never rollback
//! authority. The rollback that follows a found intersection is resolved and
//! admitted by the existing authorities (DC-NODE-29, [`super::admit_rollback`]).
//!
//! The offsets are Cardano's own (`mkOffsets`, ouroboros-consensus
//! `MiniProtocol/ChainSync/Client.hs`): the tip, the Fibonacci depths below
//! `l = min(k, retained)`, and `l` itself. Offering the point at depth `l` makes
//! one round decisive: a peer that shares any block within depth `l` shares that
//! block's ancestors, so it has the deepest candidate.

use ade_types::shelley::block::PrevHash;
use ade_types::{BlockNo, Hash32, SlotNo};

/// A chain point as it goes on the wire: slot and header hash.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChainPoint {
    pub slot: SlotNo,
    pub hash: Hash32,
}

/// One block of the durable ancestry, as decoded from its stored bytes. `hash` is
/// the recomputed header hash, `prev` the header's parent link.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AncestorHeader {
    pub slot: SlotNo,
    pub block_no: BlockNo,
    pub hash: Hash32,
    pub prev: PrevHash,
}

/// Why the ancestry read stopped.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AncestryEnd {
    /// It holds the tip and `k` ancestors (k + 1 blocks).
    SecurityParamReached,
    /// The next parent is not a stored block (the bootstrap anchor, or genesis).
    ParentNotStored,
}

/// What limited the deepest candidate.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IntersectionBound {
    /// The deepest candidate is `k` blocks below the tip.
    SecurityParam,
    /// Fewer than `k` ancestors are stored; the deepest candidate is the oldest
    /// stored block.
    RetainedHistory,
    /// The tip is the recovered bootstrap anchor, which is not a stored block; it is
    /// offered alone.
    NoStoredAncestry,
}

/// Where a candidate came from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CandidateSource {
    /// A durable block (or the recovered anchor tip) at this depth below the
    /// durable tip.
    Durable { depth: u64 },
    /// The last block delivered to the consumer on a lost session, newer than the
    /// durable tip. A resume hint only: a rollback to it is still resolved against
    /// the durable chain.
    DeliveredHead,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IntersectionCandidate {
    pub point: ChainPoint,
    pub source: CandidateSource,
}

/// The ordered offer, newest first, and the bound that limited it. Built only by
/// [`select_intersection_candidates`], [`IntersectionCandidates::without_ancestry`] and
/// [`IntersectionCandidates::with_delivered_head`], so it is never empty.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IntersectionCandidates {
    candidates: Vec<IntersectionCandidate>,
    bound: IntersectionBound,
}

impl IntersectionCandidates {
    /// The recovered bootstrap anchor as the tip: offered alone (DC-NODE-48 (c)).
    pub fn without_ancestry(tip: ChainPoint) -> Self {
        IntersectionCandidates {
            candidates: vec![IntersectionCandidate {
                point: tip,
                source: CandidateSource::Durable { depth: 0 },
            }],
            bound: IntersectionBound::NoStoredAncestry,
        }
    }

    /// Put the last delivered block first when it is newer than the newest
    /// candidate. Otherwise the offer is unchanged: the durable chain already
    /// covers it.
    pub fn with_delivered_head(mut self, head: ChainPoint) -> Self {
        if head.slot > self.candidates[0].point.slot {
            self.candidates.insert(
                0,
                IntersectionCandidate {
                    point: head,
                    source: CandidateSource::DeliveredHead,
                },
            );
        }
        self
    }

    pub fn candidates(&self) -> &[IntersectionCandidate] {
        &self.candidates
    }

    pub fn bound(&self) -> IntersectionBound {
        self.bound
    }

    /// The deepest candidate offered.
    pub fn deepest(&self) -> &IntersectionCandidate {
        &self.candidates[self.candidates.len() - 1]
    }
}

/// The ancestry is not a contiguous, linked, newest-first chain.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IntersectionAncestryError {
    EmptyAncestry,
    /// More than `k + 1` blocks: depths beyond `k` are never offered.
    LongerThanSecurityParam { len: usize, k: u64 },
    /// The read claims it reached `k` but holds fewer than `k + 1` blocks.
    ShorterThanSecurityParam { len: usize, k: u64 },
    /// The block at `depth` does not name the block at `depth + 1` as its parent.
    BrokenParentLink { depth: u64 },
    /// The block at `depth` is not exactly one higher than the block at `depth + 1`.
    HeightGap { depth: u64 },
    /// The block at `depth` is not in a later slot than the block at `depth + 1`.
    SlotNotDecreasing { depth: u64 },
}

/// Cardano's `mkOffsets k maxOffset`: `0`, the Fibonacci numbers `1, 2, 3, 5, …`
/// below `l = min(k, retained)`, then `l`. When `l = 0` Cardano lists `0` twice,
/// which selects the same point twice; it is listed once here.
pub fn intersection_offsets(k: u64, retained: u64) -> Vec<u64> {
    let l = k.min(retained);
    let mut out = vec![0];
    let (mut a, mut b) = (1u64, 2u64);
    while a < l {
        out.push(a);
        let next = a.saturating_add(b);
        a = b;
        b = next;
    }
    if l > 0 {
        out.push(l);
    }
    out
}

/// BLUE, pure, total. Validate `ancestry` (newest first, starting at the durable
/// tip) and choose the candidates at Cardano's offsets.
pub fn select_intersection_candidates(
    ancestry: &[AncestorHeader],
    end: AncestryEnd,
    k: u64,
) -> Result<IntersectionCandidates, IntersectionAncestryError> {
    if ancestry.is_empty() {
        return Err(IntersectionAncestryError::EmptyAncestry);
    }
    let len = ancestry.len();
    if len as u64 > k.saturating_add(1) {
        return Err(IntersectionAncestryError::LongerThanSecurityParam { len, k });
    }
    if end == AncestryEnd::SecurityParamReached && len as u64 != k.saturating_add(1) {
        return Err(IntersectionAncestryError::ShorterThanSecurityParam { len, k });
    }
    for (depth, pair) in ancestry.windows(2).enumerate() {
        let (child, parent) = (&pair[0], &pair[1]);
        let depth = depth as u64;
        if child.prev != PrevHash::Block(parent.hash.clone()) {
            return Err(IntersectionAncestryError::BrokenParentLink { depth });
        }
        if child.block_no.0 != parent.block_no.0.saturating_add(1) {
            return Err(IntersectionAncestryError::HeightGap { depth });
        }
        if child.slot <= parent.slot {
            return Err(IntersectionAncestryError::SlotNotDecreasing { depth });
        }
    }
    let retained = (len - 1) as u64;
    let candidates = intersection_offsets(k, retained)
        .into_iter()
        .map(|depth| {
            let a = &ancestry[depth as usize];
            IntersectionCandidate {
                point: ChainPoint {
                    slot: a.slot,
                    hash: a.hash.clone(),
                },
                source: CandidateSource::Durable { depth },
            }
        })
        .collect();
    let bound = if retained >= k {
        IntersectionBound::SecurityParam
    } else {
        IntersectionBound::RetainedHistory
    };
    Ok(IntersectionCandidates { candidates, bound })
}

/// The peer's answer to `FindIntersect`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IntersectReply {
    Found(ChainPoint),
    NotFound,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IntersectionOutcome {
    /// The peer's newest common point among those offered.
    Found(IntersectionCandidate),
    /// The peer has none of the offered points: it shares no block within the
    /// bound. Terminal for that peer; nothing durable changes.
    NoCommonPoint {
        bound: IntersectionBound,
        deepest: ChainPoint,
    },
}

/// The reply broke the protocol.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IntersectionReplyFault {
    /// `IntersectFound` named a point that was not offered.
    PointNotOffered { point: ChainPoint },
}

/// BLUE, pure, total. Read the peer's reply against the offer.
pub fn classify_intersect_reply(
    offer: &IntersectionCandidates,
    reply: &IntersectReply,
) -> Result<IntersectionOutcome, IntersectionReplyFault> {
    match reply {
        IntersectReply::Found(point) => offer
            .candidates
            .iter()
            .find(|c| &c.point == point)
            .map(|c| IntersectionOutcome::Found(c.clone()))
            .ok_or_else(|| IntersectionReplyFault::PointNotOffered {
                point: point.clone(),
            }),
        IntersectReply::NotFound => Ok(IntersectionOutcome::NoCommonPoint {
            bound: offer.bound,
            deepest: offer.deepest().point.clone(),
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn h(n: u64) -> Hash32 {
        let mut b = [0u8; 32];
        b[..8].copy_from_slice(&n.to_be_bytes());
        b[31] = 0xA5;
        Hash32(b)
    }

    /// A linked chain of `len` blocks, newest first, whose tip is at `tip_block_no`
    /// and slot `tip_slot`; each parent is one block and two slots lower.
    fn chain(len: u64, tip_block_no: u64, tip_slot: u64) -> Vec<AncestorHeader> {
        (0..len)
            .map(|d| {
                let block_no = tip_block_no - d;
                AncestorHeader {
                    slot: SlotNo(tip_slot - 2 * d),
                    block_no: BlockNo(block_no),
                    hash: h(block_no),
                    prev: PrevHash::Block(h(block_no - 1)),
                }
            })
            .collect()
    }

    fn depths(c: &IntersectionCandidates) -> Vec<u64> {
        c.candidates()
            .iter()
            .map(|c| match c.source {
                CandidateSource::Durable { depth } => depth,
                CandidateSource::DeliveredHead => u64::MAX,
            })
            .collect()
    }

    #[test]
    fn lv16_offsets_are_cardanos_mk_offsets() {
        assert_eq!(
            intersection_offsets(2160, 1_000_000),
            vec![0, 1, 2, 3, 5, 8, 13, 21, 34, 55, 89, 144, 233, 377, 610, 987, 1597, 2160]
        );
        assert_eq!(intersection_offsets(2160, 0), vec![0]);
        assert_eq!(intersection_offsets(2160, 1), vec![0, 1]);
        assert_eq!(intersection_offsets(2160, 2), vec![0, 1, 2]);
        assert_eq!(intersection_offsets(2160, 4), vec![0, 1, 2, 3, 4]);
        assert_eq!(
            intersection_offsets(2160, 100),
            vec![0, 1, 2, 3, 5, 8, 13, 21, 34, 55, 89, 100]
        );
        for retained in [0u64, 1, 2, 4, 100, 2159, 2160, 9999] {
            let o = intersection_offsets(2160, retained);
            assert_eq!(*o.last().unwrap(), retained.min(2160));
            assert!(o.windows(2).all(|w| w[0] < w[1]));
        }
    }

    #[test]
    fn lv16_selection_is_newest_first_and_always_offers_the_deepest() {
        let a = chain(2161, 5_369_626, 138_046_917);
        let c = select_intersection_candidates(&a, AncestryEnd::SecurityParamReached, 2160).unwrap();
        assert_eq!(depths(&c), intersection_offsets(2160, 2160));
        assert_eq!(c.bound(), IntersectionBound::SecurityParam);
        assert_eq!(c.deepest().point.hash, h(5_369_626 - 2160));
        assert!(c
            .candidates()
            .windows(2)
            .all(|w| w[0].point.slot > w[1].point.slot));

        let short = chain(40, 1_000, 5_000);
        let c = select_intersection_candidates(&short, AncestryEnd::ParentNotStored, 2160).unwrap();
        assert_eq!(c.bound(), IntersectionBound::RetainedHistory);
        assert_eq!(*depths(&c).last().unwrap(), 39);
        assert_eq!(c.deepest().point.hash, h(1_000 - 39));
    }

    /// The killed store: tip 138046917 (block 5369626) is the block the peer
    /// replaced; its parent 138046915 (block 5369625) is on the peer's chain.
    #[test]
    fn lv16_killed_store_shape_offers_the_parent_and_finds_it() {
        let mut a = chain(2161, 5_369_626, 138_046_917);
        a[1].slot = SlotNo(138_046_915);
        let c = select_intersection_candidates(&a, AncestryEnd::SecurityParamReached, 2160).unwrap();
        assert_eq!(c.candidates()[0].point.slot, SlotNo(138_046_917));
        assert_eq!(c.candidates()[1].point.slot, SlotNo(138_046_915));
        let parent = c.candidates()[1].point.clone();
        match classify_intersect_reply(&c, &IntersectReply::Found(parent.clone())).unwrap() {
            IntersectionOutcome::Found(found) => {
                assert_eq!(found.point, parent);
                assert_eq!(found.source, CandidateSource::Durable { depth: 1 });
            }
            other => panic!("expected Found, got {other:?}"),
        }
    }

    #[test]
    fn lv16_ancestry_faults_are_typed() {
        let k = 2160;
        assert_eq!(
            select_intersection_candidates(&[], AncestryEnd::ParentNotStored, k),
            Err(IntersectionAncestryError::EmptyAncestry)
        );
        let long = chain(2162, 10_000, 50_000);
        assert_eq!(
            select_intersection_candidates(&long, AncestryEnd::SecurityParamReached, k),
            Err(IntersectionAncestryError::LongerThanSecurityParam { len: 2162, k })
        );
        let short = chain(10, 10_000, 50_000);
        assert_eq!(
            select_intersection_candidates(&short, AncestryEnd::SecurityParamReached, k),
            Err(IntersectionAncestryError::ShorterThanSecurityParam { len: 10, k })
        );
        let mut broken = chain(10, 10_000, 50_000);
        broken[3].prev = PrevHash::Block(h(1));
        assert_eq!(
            select_intersection_candidates(&broken, AncestryEnd::ParentNotStored, k),
            Err(IntersectionAncestryError::BrokenParentLink { depth: 3 })
        );
        let mut genesis_link = chain(10, 10_000, 50_000);
        genesis_link[0].prev = PrevHash::Genesis;
        assert_eq!(
            select_intersection_candidates(&genesis_link, AncestryEnd::ParentNotStored, k),
            Err(IntersectionAncestryError::BrokenParentLink { depth: 0 })
        );
        let mut gap = chain(10, 10_000, 50_000);
        gap[5].block_no = BlockNo(gap[5].block_no.0 - 1);
        gap[4].prev = PrevHash::Block(gap[5].hash.clone());
        assert_eq!(
            select_intersection_candidates(&gap, AncestryEnd::ParentNotStored, k),
            Err(IntersectionAncestryError::HeightGap { depth: 4 })
        );
        let mut slot = chain(10, 10_000, 50_000);
        slot[2].slot = slot[3].slot;
        assert_eq!(
            select_intersection_candidates(&slot, AncestryEnd::ParentNotStored, k),
            Err(IntersectionAncestryError::SlotNotDecreasing { depth: 2 })
        );
    }

    #[test]
    fn lv16_reply_must_name_an_offered_point() {
        let a = chain(100, 1_000, 5_000);
        let c = select_intersection_candidates(&a, AncestryEnd::ParentNotStored, 2160).unwrap();
        // Depth 4 is not one of Cardano's offsets, so it was not offered.
        let unoffered = ChainPoint {
            slot: a[4].slot,
            hash: a[4].hash.clone(),
        };
        assert_eq!(
            classify_intersect_reply(&c, &IntersectReply::Found(unoffered.clone())),
            Err(IntersectionReplyFault::PointNotOffered { point: unoffered })
        );
        // A point with an offered slot but another hash is not offered either.
        let forged = ChainPoint {
            slot: a[3].slot,
            hash: h(77),
        };
        assert!(classify_intersect_reply(&c, &IntersectReply::Found(forged)).is_err());
    }

    #[test]
    fn lv16_not_found_is_no_common_point_with_the_bound() {
        let a = chain(100, 1_000, 5_000);
        let c = select_intersection_candidates(&a, AncestryEnd::ParentNotStored, 2160).unwrap();
        assert_eq!(
            classify_intersect_reply(&c, &IntersectReply::NotFound),
            Ok(IntersectionOutcome::NoCommonPoint {
                bound: IntersectionBound::RetainedHistory,
                deepest: ChainPoint {
                    slot: a[99].slot,
                    hash: a[99].hash.clone()
                },
            })
        );
        let full = chain(2161, 9_000, 90_000);
        let c = select_intersection_candidates(&full, AncestryEnd::SecurityParamReached, 2160).unwrap();
        assert!(matches!(
            classify_intersect_reply(&c, &IntersectReply::NotFound),
            Ok(IntersectionOutcome::NoCommonPoint {
                bound: IntersectionBound::SecurityParam,
                ..
            })
        ));
    }

    #[test]
    fn lv16_a_point_without_stored_ancestry_is_offered_alone() {
        let tip = ChainPoint {
            slot: SlotNo(129_813_427),
            hash: h(1),
        };
        let c = IntersectionCandidates::without_ancestry(tip.clone());
        assert_eq!(c.candidates().len(), 1);
        assert_eq!(c.bound(), IntersectionBound::NoStoredAncestry);
        assert_eq!(c.deepest().point, tip);
    }

    #[test]
    fn lv16_delivered_head_goes_first_only_when_newer() {
        let a = chain(10, 1_000, 5_000);
        let c = select_intersection_candidates(&a, AncestryEnd::ParentNotStored, 2160).unwrap();
        let newer = ChainPoint {
            slot: SlotNo(5_003),
            hash: h(1_002),
        };
        let with = c.clone().with_delivered_head(newer.clone());
        assert_eq!(with.candidates()[0].point, newer);
        assert_eq!(with.candidates()[0].source, CandidateSource::DeliveredHead);
        assert_eq!(&with.candidates()[1..], c.candidates());
        assert_eq!(with.bound(), c.bound());
        let same = ChainPoint {
            slot: a[0].slot,
            hash: a[0].hash.clone(),
        };
        assert_eq!(c.clone().with_delivered_head(same), c);
        let older = ChainPoint {
            slot: SlotNo(4_000),
            hash: h(3),
        };
        assert_eq!(c.clone().with_delivered_head(older), c);
    }
}
