//! RED — follow-mode bridge between a peer's ChainSync stream and BLUE
//! fork-choice.
//!
//! # What follow mode is — and is NOT
//!
//! Follow mode asserts *tip-selection agreement* with a peer that has
//! **already validated** the chain. It runs BLUE fork-choice
//! (`select_best_chain`) and rollback (`apply_rollback`) ONLY. It does
//! **not** call `validate_and_apply_header`, builds **no** `LedgerView`,
//! and verifies **no** VRF / leader / nonce / KES. Those checks need the
//! ledger stake state and epoch nonce (workstream B); follow mode trusts
//! the peer for them.
//!
//! It answers exactly one question: given a real peer's roll-forward /
//! roll-backward stream, does Ade's selected tip track the peer's tip?
//!
//! # Why building `ValidatedHeaderSummary` here is acceptable
//!
//! `ValidatedHeaderSummary` is BLUE's "this header validated" token.
//! Follow mode constructs it from *decoded-but-not-validated* header
//! fields. That is normally a layering violation — a RED actor minting a
//! BLUE validity claim. It is acceptable here, and ONLY here, because:
//!
//! 1. this crate is RED and non-authoritative — nothing it produces is
//!    persisted, hashed into consensus state, or fed back into BLUE;
//! 2. the summary is used solely to drive the *selection* comparison,
//!    which is the agreement smoke test, not a validity decision;
//! 3. the peer has already validated the header, so the fields are
//!    trustworthy enough for a selection-agreement check.
//!
//! This minting must never leak into BLUE or any persisted path.

use ade_codec::cbor::{self, ContainerEncoding};
use ade_codec::shelley::block::decode_shelley_block_inner;
use ade_codec::CodecError;
use ade_core::consensus::candidate::{CandidateFragment, ChainSelectorState, TiebreakerView};
use ade_core::consensus::events::{BlockDistance, ChainEvent, Point, SecurityParam};
use ade_core::consensus::fork_choice::{select_best_chain, ForkChoiceError};
use ade_core::consensus::header_summary::ValidatedHeaderSummary;
use ade_core::consensus::praos_state::{Nonce, PraosChainDepState};
use ade_core::consensus::rollback::{apply_rollback, RollBackRequest};
use ade_crypto::blake2b::{blake2b_224, blake2b_256};
use ade_crypto::vrf::VrfOutput;
use ade_types::shelley::block::VrfData;
use ade_types::{BlockNo, Hash28, Hash32, SlotNo};

/// Errors raised while projecting or ingesting a peer header. Closed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FollowError {
    /// The era-tagged block / header bytes failed to decode.
    HeaderDecode(CodecError),
    /// The VRF leader cert did not parse to a 64-byte output.
    VrfOutputMalformed,
    /// Fork-choice returned no candidates — should never happen because
    /// follow mode always submits a single fragment.
    ForkChoice(ForkChoiceError),
    /// A roll-backward target was not found in the recent-point window,
    /// so the rollback depth cannot be derived deterministically.
    RollbackTargetUnknown { to_point: Point },
}

/// The fork-choice projection of one peer header — all fields a
/// selection comparison needs, plus the exact header CBOR that was
/// hashed to obtain the block hash. Reusable corpus row for workstream B.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectedHeader {
    pub block_no: BlockNo,
    pub slot: SlotNo,
    /// Block hash = blake2b256(header_cbor). Doubles as `Point.hash`.
    pub hash: Hash32,
    /// Pool id = blake2b224(issuer_vkey).
    pub issuer_pool: Hash28,
    pub op_cert_counter: u64,
    /// First 8 bytes of the leader VRF output (the lottery value).
    pub vrf_output_first_8: [u8; 8],
}

impl ProjectedHeader {
    pub fn point(&self) -> Point {
        Point {
            slot: self.slot,
            hash: self.hash.clone(),
        }
    }

    fn tiebreaker(&self) -> TiebreakerView {
        TiebreakerView {
            slot: self.slot,
            issuer_hash: self.issuer_pool.clone(),
            op_cert_counter: self.op_cert_counter,
            leader_vrf_output_first_8: self.vrf_output_first_8,
        }
    }

    /// Mint the BLUE `ValidatedHeaderSummary` from peer-trusted fields.
    /// See the module docstring for why a RED actor may construct this.
    fn as_summary(&self) -> ValidatedHeaderSummary {
        let mut vrf_full = [0u8; 64];
        vrf_full[..8].copy_from_slice(&self.vrf_output_first_8);
        ValidatedHeaderSummary {
            slot: self.slot,
            block_no: self.block_no,
            body_hash: self.hash.clone(),
            issuer_pool: self.issuer_pool.clone(),
            op_cert_counter: self.op_cert_counter,
            vrf_leader_output: VrfOutput(vrf_full),
        }
    }
}

/// Project a single header from an era-tagged block envelope
/// (`[era_tag, era_block]`) — the on-disk corpus shape and the same
/// shape a RollForward header decodes to once the N2N wrapper is peeled.
///
/// The block hash is `blake2b256` over the exact header CBOR sub-slice,
/// matching the Cardano header-hash rule.
///
/// Trailing bytes after the first envelope are tolerated: this is RED
/// corpus-reading, and some on-disk corpus entries concatenate more than
/// one block. BLUE's strict `decode_block_envelope` (no trailing bytes)
/// is intentionally NOT used here so the strict guarantee stays in BLUE.
pub fn project_header_from_block(block_envelope: &[u8]) -> Result<ProjectedHeader, FollowError> {
    // Outer envelope: array(2) = [era_tag, era_block]. Skip the tag,
    // then the era block is the next item; ignore anything after it.
    let mut off = 0usize;
    match cbor::read_array_header(block_envelope, &mut off) {
        Ok(ContainerEncoding::Definite(2, _)) => {}
        Ok(_) => {
            return Err(FollowError::HeaderDecode(CodecError::InvalidCborStructure {
                offset: 0,
                detail: "block envelope must be definite-length array(2)",
            }))
        }
        Err(e) => return Err(FollowError::HeaderDecode(e)),
    }
    cbor::read_uint(block_envelope, &mut off).map_err(FollowError::HeaderDecode)?; // era tag
    let (inner_start, inner_end) =
        cbor::skip_item(block_envelope, &mut off).map_err(FollowError::HeaderDecode)?;
    let inner = &block_envelope[inner_start..inner_end];

    // Capture the header CBOR sub-slice: skip past the block array
    // header, then take the first item (the header array(2)) verbatim.
    let mut hoff = 0usize;
    cbor::read_array_header(inner, &mut hoff).map_err(FollowError::HeaderDecode)?;
    let (h_start, h_end) = cbor::skip_item(inner, &mut hoff).map_err(FollowError::HeaderDecode)?;
    let header_cbor = &inner[h_start..h_end];

    project_header_from_components(inner, header_cbor)
}

/// Project a header straight from the N2N RollForward `header` field.
///
/// The on-wire shape (see `ade_network::codec::chain_sync`) is
/// `[serialisationInfo: word, tag(24, bytes)]`, where the tagged bytes
/// are the CBOR `[era_tag, header_array2]` — an era-tagged *header*, not
/// a full block. We peel the word + tag-24 wrapper, then hash and project
/// the inner header CBOR directly.
///
/// This path is exercised by the operator's live evidence pass, not by
/// the offline CI gate (which projects from full block envelopes on
/// disk). It is kept here so the live driver and the offline test share
/// one projection definition.
pub fn project_header_from_n2n_rollforward(
    rollforward_header: &[u8],
) -> Result<ProjectedHeader, FollowError> {
    let mut off = 0usize;
    // N2N chain-sync wrapped header (hard-fork combinator):
    //   [ era_tag, #6.24(header_cbor) ]
    // SINGLE wrapper: for Shelley-based eras (era_tag 1..=7) the tag-24
    // payload IS the era-specific header CBOR directly (itself an array(2)
    // of [header_body, kes_signature]). era_tag 0 (Byron) nests a different
    // shape and is not decodable by the Praos header decoder.
    //
    // Verified against a real preprod N2N RollForward frame
    // (corpus/network/n2n/chain_sync). An earlier double-wrapper assumption
    // mis-parsed the tag-24 payload as [era_tag, header] again and is fixed
    // here — the synthetic round-trip test had baked in the same wrong shape.
    match cbor::read_array_header(rollforward_header, &mut off) {
        Ok(ContainerEncoding::Definite(2, _)) => {}
        Ok(_) => {
            return Err(FollowError::HeaderDecode(CodecError::InvalidCborStructure {
                offset: 0,
                detail: "N2N RollForward header must be array(2)",
            }))
        }
        Err(e) => return Err(FollowError::HeaderDecode(e)),
    }
    let (era_tag, _) =
        cbor::read_uint(rollforward_header, &mut off).map_err(FollowError::HeaderDecode)?;
    if era_tag == 0 {
        return Err(FollowError::HeaderDecode(CodecError::InvalidCborStructure {
            offset: off,
            detail: "Byron (era 0) headers are not supported by follow mode",
        }));
    }
    let tag = cbor::read_tag(rollforward_header, &mut off).map_err(FollowError::HeaderDecode)?;
    if tag.0 != 24 {
        return Err(FollowError::HeaderDecode(CodecError::InvalidCborStructure {
            offset: off,
            detail: "expected CBOR tag 24 (encoded-CBOR) around the header",
        }));
    }
    let (header_cbor, _) =
        cbor::read_bytes(rollforward_header, &mut off).map_err(FollowError::HeaderDecode)?;

    project_header_from_header_cbor(&header_cbor)
}

/// Project from the inner block bytes plus the already-isolated header
/// CBOR. Split out so a live driver that has the header bytes directly
/// (from the N2N RollForward wrapper) can reuse the projection without
/// re-deriving the sub-slice.
fn project_header_from_components(
    inner_block: &[u8],
    header_cbor: &[u8],
) -> Result<ProjectedHeader, FollowError> {
    let mut off = 0usize;
    let block = decode_shelley_block_inner(inner_block, &mut off)
        .map_err(FollowError::HeaderDecode)?;
    let body = &block.header.body;

    let hash = blake2b_256(header_cbor);
    let issuer_pool = blake2b_224(&body.issuer_vkey);
    let leader_cert = match &body.vrf {
        VrfData::Split { leader_vrf, .. } => leader_vrf.as_slice(),
        VrfData::Combined { vrf_result } => vrf_result.as_slice(),
    };
    let vrf_output_first_8 = leader_vrf_output_first_8(leader_cert)?;

    Ok(ProjectedHeader {
        block_no: BlockNo(body.block_number),
        slot: SlotNo(body.slot),
        hash,
        issuer_pool,
        op_cert_counter: body.operational_cert.sequence_number,
        vrf_output_first_8,
    })
}

/// Project directly from a bare header CBOR (`[header_body, kes_sig]`),
/// the shape that arrives on the N2N RollForward path. Mirrors the
/// `ade_codec` header-body layout but reads only the fields the
/// selection projection needs; the block hash is `blake2b256` over the
/// full `header_cbor`.
///
/// RED, peer-trusted: this parses fields without validating them.
fn project_header_from_header_cbor(header_cbor: &[u8]) -> Result<ProjectedHeader, FollowError> {
    let decode = |e| FollowError::HeaderDecode(e);
    let mut off = 0usize;
    // header = array(2) [body, kes_sig]
    match cbor::read_array_header(header_cbor, &mut off).map_err(decode)? {
        ContainerEncoding::Definite(2, _) => {}
        _ => {
            return Err(FollowError::HeaderDecode(CodecError::InvalidCborStructure {
                offset: 0,
                detail: "header must be array(2)",
            }))
        }
    }
    // body = array(15) TPraos / array(10) Praos
    let hdr_len = match cbor::read_array_header(header_cbor, &mut off).map_err(decode)? {
        ContainerEncoding::Definite(n @ 15, _) | ContainerEncoding::Definite(n @ 10, _) => n,
        _ => {
            return Err(FollowError::HeaderDecode(CodecError::InvalidCborStructure {
                offset: off,
                detail: "header body must be array(15) or array(10)",
            }))
        }
    };
    let (block_number, _) = cbor::read_uint(header_cbor, &mut off).map_err(decode)?;
    let (slot, _) = cbor::read_uint(header_cbor, &mut off).map_err(decode)?;
    cbor::skip_item(header_cbor, &mut off).map_err(decode)?; // prev_hash
    let (issuer_vkey, _) = cbor::read_bytes(header_cbor, &mut off).map_err(decode)?;
    cbor::skip_item(header_cbor, &mut off).map_err(decode)?; // vrf_vkey

    // VRF: TPraos has [nonce_vrf, leader_vrf]; Praos has [vrf_result].
    let leader_cert_range = if hdr_len == 15 {
        cbor::skip_item(header_cbor, &mut off).map_err(decode)?; // nonce_vrf
        cbor::skip_item(header_cbor, &mut off).map_err(decode)? // leader_vrf
    } else {
        cbor::skip_item(header_cbor, &mut off).map_err(decode)? // vrf_result
    };
    let vrf_output_first_8 =
        leader_vrf_output_first_8(&header_cbor[leader_cert_range.0..leader_cert_range.1])?;

    cbor::skip_item(header_cbor, &mut off).map_err(decode)?; // body_size (uint, but skip is fine)
    cbor::skip_item(header_cbor, &mut off).map_err(decode)?; // body_hash

    // operational_cert: inlined (15) or nested array(4) (10). The
    // sequence_number is the 2nd field either way.
    let op_cert_counter = if hdr_len == 15 {
        cbor::skip_item(header_cbor, &mut off).map_err(decode)?; // hot_vkey
        let (seq, _) = cbor::read_uint(header_cbor, &mut off).map_err(decode)?;
        seq
    } else {
        match cbor::read_array_header(header_cbor, &mut off).map_err(decode)? {
            ContainerEncoding::Definite(4, _) => {}
            _ => {
                return Err(FollowError::HeaderDecode(CodecError::InvalidCborStructure {
                    offset: off,
                    detail: "operational cert must be array(4)",
                }))
            }
        }
        cbor::skip_item(header_cbor, &mut off).map_err(decode)?; // hot_vkey
        let (seq, _) = cbor::read_uint(header_cbor, &mut off).map_err(decode)?;
        seq
    };

    Ok(ProjectedHeader {
        block_no: BlockNo(block_number),
        slot: SlotNo(slot),
        hash: blake2b_256(header_cbor),
        issuer_pool: blake2b_224(&issuer_vkey),
        op_cert_counter,
        vrf_output_first_8,
    })
}

/// The leader VRF cert is `[output: bytes(64), proof: bytes]`. The
/// lottery value is the first 8 bytes of the 64-byte output.
fn leader_vrf_output_first_8(cert_cbor: &[u8]) -> Result<[u8; 8], FollowError> {
    let mut off = 0usize;
    cbor::read_array_header(cert_cbor, &mut off).map_err(|_| FollowError::VrfOutputMalformed)?;
    let (output, _) =
        cbor::read_bytes(cert_cbor, &mut off).map_err(|_| FollowError::VrfOutputMalformed)?;
    if output.len() < 8 {
        return Err(FollowError::VrfOutputMalformed);
    }
    let mut first8 = [0u8; 8];
    first8.copy_from_slice(&output[..8]);
    Ok(first8)
}

/// Peer's claimed tip, carried on every ChainSync reply.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PeerTip {
    pub point: Point,
    pub block_no: BlockNo,
}

/// Tip-agreement verdict between Ade's selected tip and the peer's tip.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AgreementStatus {
    /// No peer tip has been observed yet.
    NoPeerTipYet,
    /// Ade's tip is behind the peer tip (still catching up).
    CatchingUp {
        ade_block_no: BlockNo,
        peer_block_no: BlockNo,
    },
    /// Ade's tip equals the peer tip — caught up and in agreement.
    Agreed { tip: Point, block_no: BlockNo },
    /// Ade reached the peer's block number but selected a different
    /// point. Any disagreement is a hard failure for the smoke test.
    Disagree {
        ade_tip: Point,
        peer_tip: Point,
        block_no: BlockNo,
    },
}

/// RED follow-mode state. Wraps the BLUE selector + chain-dep state and
/// the bookkeeping the bridge needs (recent points for rollback-depth
/// derivation, last observed peer tip, disagreement count).
#[derive(Debug, Clone)]
pub struct FollowState {
    pub selector: ChainSelectorState,
    pub chain_dep: PraosChainDepState,
    /// Recent applied points, oldest-first, for rollback-depth lookup.
    /// Bounded to the security parameter so it cannot grow unbounded on
    /// a long follow run.
    recent: Vec<(Point, BlockNo)>,
    peer_tip: Option<PeerTip>,
    disagreements: u64,
}

impl FollowState {
    /// Start a follow session anchored at `anchor` / `anchor_block_no`
    /// (the intersection point established with the peer). `initial_nonce`
    /// seeds the carried chain-dep state; follow mode never reads it for
    /// selection — it is threaded through only so rollbacks return a
    /// well-formed `PraosChainDepState`.
    pub fn new(
        anchor: Point,
        anchor_block_no: BlockNo,
        anchor_tiebreaker: TiebreakerView,
        k: SecurityParam,
        initial_nonce: Nonce,
    ) -> Self {
        let selector = ChainSelectorState {
            current_tip: anchor.clone(),
            current_tip_block_no: anchor_block_no,
            current_tiebreaker: anchor_tiebreaker,
            immutable_tip: anchor.clone(),
            immutable_tip_block_no: anchor_block_no,
            security_param: k,
        };
        FollowState {
            selector,
            chain_dep: PraosChainDepState::genesis(initial_nonce),
            recent: vec![(anchor, anchor_block_no)],
            peer_tip: None,
            disagreements: 0,
        }
    }

    pub fn disagreements(&self) -> u64 {
        self.disagreements
    }

    pub fn current_tip(&self) -> &Point {
        &self.selector.current_tip
    }

    pub fn current_tip_block_no(&self) -> BlockNo {
        self.selector.current_tip_block_no
    }

    fn record_recent(&mut self, point: Point, block_no: BlockNo) {
        self.recent.push((point, block_no));
        // Keep only the last k+1 entries — a rollback deeper than k is
        // rejected by BLUE anyway, so older points are never needed.
        let cap = self.selector.security_param.0.saturating_add(1) as usize;
        if self.recent.len() > cap {
            let excess = self.recent.len() - cap;
            self.recent.drain(0..excess);
        }
    }

    /// Note the peer's claimed tip (carried on every ChainSync reply).
    pub fn observe_peer_tip(&mut self, tip: PeerTip) {
        self.peer_tip = Some(tip);
    }
}

/// Ingest a peer RollForward. Builds a single-header `CandidateFragment`
/// anchored at the current tip and runs BLUE `select_best_chain`. On a
/// `ChainSelected` event the follow state advances; the recent-point
/// window and tip move with it.
pub fn ingest_rollforward(
    mut state: FollowState,
    header: &ProjectedHeader,
    peer_tip: PeerTip,
) -> Result<(FollowState, ChainEvent), FollowError> {
    let fragment = CandidateFragment {
        anchor: state.selector.current_tip.clone(),
        anchor_block_no: state.selector.current_tip_block_no,
        headers: vec![header.as_summary()],
        select_view: header.tiebreaker(),
        rollback_depth: BlockDistance(0),
    };

    let (new_selector, event) = select_best_chain(&state.selector, &[fragment])
        .map_err(FollowError::ForkChoice)?;
    state.selector = new_selector;

    if let ChainEvent::ChainSelected { .. } = event {
        state.record_recent(header.point(), header.block_no);
    }

    state.observe_peer_tip(peer_tip);
    Ok((state, event))
}

/// Ingest a peer RollBackward. Derives the rollback depth from the
/// recent-point window (block-distance from current tip to the target),
/// then runs BLUE `apply_rollback`. The carried chain-dep state is reused
/// verbatim — follow mode does not reconstruct nonces.
pub fn ingest_rollbackward(
    mut state: FollowState,
    to_point: Point,
    peer_tip: PeerTip,
) -> Result<(FollowState, ChainEvent), FollowError> {
    let to_block_no = state
        .recent
        .iter()
        .find(|(p, _)| *p == to_point)
        .map(|(_, bn)| *bn)
        .ok_or_else(|| FollowError::RollbackTargetUnknown {
            to_point: to_point.clone(),
        })?;

    let depth = BlockDistance(
        state
            .selector
            .current_tip_block_no
            .0
            .saturating_sub(to_block_no.0),
    );
    let tiebreaker_at_target = state.selector.current_tiebreaker.clone();

    let request = RollBackRequest {
        to_point: to_point.clone(),
        to_block_no,
        depth,
    };

    let applied = apply_rollback(
        &state.selector,
        &state.chain_dep,
        &state.chain_dep,
        &tiebreaker_at_target,
        &request,
    );
    state.selector = applied.new_state;
    state.chain_dep = applied.new_chain_dep;

    if let ChainEvent::RolledBack { .. } = applied.event {
        // Drop every recent point above the rollback target.
        if let Some(idx) = state.recent.iter().position(|(p, _)| *p == to_point) {
            state.recent.truncate(idx + 1);
        }
    }

    state.observe_peer_tip(peer_tip);
    Ok((state, applied.event))
}

/// Compare Ade's selected tip to the last observed peer tip. Increments
/// the disagreement counter on a hard mismatch (same block number,
/// different point).
pub fn agreement_status(state: &mut FollowState) -> AgreementStatus {
    let peer = match &state.peer_tip {
        None => return AgreementStatus::NoPeerTipYet,
        Some(p) => p.clone(),
    };
    let ade_block_no = state.selector.current_tip_block_no;
    let ade_tip = state.selector.current_tip.clone();

    if ade_block_no.0 < peer.block_no.0 {
        return AgreementStatus::CatchingUp {
            ade_block_no,
            peer_block_no: peer.block_no,
        };
    }
    if ade_block_no == peer.block_no && ade_tip == peer.point {
        return AgreementStatus::Agreed {
            tip: ade_tip,
            block_no: ade_block_no,
        };
    }
    // Caught up (or past) but the points differ — hard disagreement.
    state.disagreements = state.disagreements.saturating_add(1);
    AgreementStatus::Disagree {
        ade_tip,
        peer_tip: peer.point,
        block_no: ade_block_no,
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
#[allow(clippy::expect_used)]
#[allow(clippy::panic)]
mod tests {
    use super::*;
    use ade_codec::cbor::{ContainerEncoding, IntWidth};

    // A real Babbage block envelope is needed to exercise both
    // projection paths; the offline integration test reads the corpus
    // from disk. Here we synthesise the N2N RollForward wrapper around a
    // header CBOR and assert the wrapper-peeling path agrees with a
    // direct header-CBOR projection on the same bytes.

    fn header_cbor_from_block(path: &str) -> Vec<u8> {
        let block = std::fs::read(path).unwrap();
        let mut off = 0usize;
        cbor::read_array_header(&block, &mut off).unwrap(); // envelope array(2)
        cbor::read_uint(&block, &mut off).unwrap(); // era tag
        let (is, ie) = cbor::skip_item(&block, &mut off).unwrap();
        let inner = &block[is..ie];
        let mut io = 0usize;
        cbor::read_array_header(inner, &mut io).unwrap(); // block array
        let (hs, he) = cbor::skip_item(inner, &mut io).unwrap();
        inner[hs..he].to_vec()
    }

    fn wrap_n2n_rollforward(era_tag: u64, header_cbor: &[u8]) -> Vec<u8> {
        // Real N2N hard-fork-combinator wrapper: [era_tag, #6.24(header_cbor)].
        // (Matches a real preprod RollForward frame; the tag-24 payload is the
        // header CBOR directly, NOT a re-nested [era, header].)
        let mut outer = Vec::new();
        cbor::write_array_header(&mut outer, ContainerEncoding::Definite(2, IntWidth::Inline));
        cbor::write_uint_canonical(&mut outer, era_tag);
        cbor::write_tag(&mut outer, 24, cbor::canonical_width(24));
        cbor::write_bytes_canonical(&mut outer, header_cbor);
        outer
    }

    #[test]
    fn n2n_rollforward_projection_matches_block_projection() {
        let block_path = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../corpus/boundary_blocks/babbage_epoch366/blk_chunk03368_idx00000_era6_slot72748820.cbor"
        );
        let block_bytes = std::fs::read(block_path).unwrap();
        let from_block = project_header_from_block(&block_bytes).unwrap();

        let header_cbor = header_cbor_from_block(block_path);
        let wrapped = wrap_n2n_rollforward(6, &header_cbor);
        let from_n2n = project_header_from_n2n_rollforward(&wrapped).unwrap();

        assert_eq!(from_block, from_n2n);
    }

    #[test]
    fn n2n_rollforward_projection_matches_block_projection_tpraos() {
        // Allegra (array(15) Split-VRF) — the inlined op-cert path.
        let block_path = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../corpus/boundary_blocks/allegra_epoch237/blk_chunk00788_idx00000_era3_slot17020800.cbor"
        );
        let block_bytes = std::fs::read(block_path).unwrap();
        let from_block = project_header_from_block(&block_bytes).unwrap();

        let header_cbor = header_cbor_from_block(block_path);
        let wrapped = wrap_n2n_rollforward(3, &header_cbor);
        let from_n2n = project_header_from_n2n_rollforward(&wrapped).unwrap();

        assert_eq!(from_block, from_n2n);
    }
}
