// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types
// - Explicit state transitions only
// - Canonical serialization for all persisted/hashed data

//! GREEN BA-02 peer-acceptance evidence manifest + correlator
//! (PHASE4-N-F-C L6).
//!
//! The evidence surface for BA-02: "an Ade-forged block was accepted by a
//! Haskell cardano-node peer." This module is GREEN evidence
//! (`[[feedback-evidence-reducers-are-green-not-authority]]`): it COMPARES
//! two already-authoritative outputs — the BLUE-minted forged-block hash
//! (read from `ForgedBlockArtifact`, never recomputed) and the
//! operator-captured peer-accept signal — and emits a closed verdict. It
//! forges nothing, admits nothing, persists no node state. A
//! [`Ba02Manifest`] is a CLAIM ABOUT authority, not authority; it cannot
//! make a block valid or accepted.
//!
//! Honesty line (`[[feedback-shell-must-not-overstate-semantic-truth]]`):
//! wire/forge success != peer acceptance. The manifest is constructible
//! ONLY on an exact forged-hash <-> peer-accept match at the matching
//! chain point. Every weaker or mismatched signal is [`NoEvidence`]:
//!   - Ade self-accept is NOT evidence;
//!   - `ForgeSucceeded` alone is NOT evidence;
//!   - `block_received` alone is NOT evidence;
//!   - a lagging/diverged agreement verdict is NOT evidence;
//!   - conflicting peer signals at the forged context are NOT evidence.
//! These weaker signals are not even representable as a [`PeerAcceptEvent`]
//! — the parser's allow-list drops them (it never coerces a non-acceptance
//! line into acceptance).
//!
//! Two peer-accept signal forms, RANKED (L6 §9.0 M1):
//!   - [`PeerAcceptEvent::PeerServedBlock`] — STRONGEST: the peer served
//!     the forged block back on its own chain-serving path (it is carrying
//!     the block in the chain it serves to others).
//!   - [`PeerAcceptEvent::PeerChainTip`] — corroborating: the peer's tip
//!     names the forged hash. Valid only when it names the EXACT forged
//!     hash at the matching slot/block context.
//! If multiple signals are present they MUST all agree; `PeerServedBlock`
//! is primary when present. Any conflict yields [`NoEvidence`].
//!
//! Synthetic logs may exercise the parser/correlator mechanics but CANNOT
//! satisfy BA-02: a real manifest requires a real operator-captured peer
//! log (the live capture is operator-gated under RO-LIVE-01; the status
//! flip happens only after registry review at the appropriate close).
//!
//! The bounty evaluates Ade's node lifecycle; it does not define Ade's
//! architecture.

use ade_runtime::producer::coordinator::ForgedBlockArtifact;
use ade_types::Hash32;
use serde::{Deserialize, Serialize};

/// Versioned schema tag for the canonical [`Ba02Manifest`] (mirrors the
/// `RawMithrilManifest` closed-manifest precedent). Bump on any field
/// change.
pub const BA02_MANIFEST_SCHEMA_VERSION: u32 = 1;

/// The Ade side of the correlation: the forged block's identity + chain
/// point. Both fields are read DIRECTLY from the BLUE-minted
/// [`ForgedBlockArtifact`] — the hash is NOT recomputed (M2). The artifact
/// exposes exactly `slot`, `hash`, `bytes`; it carries NO block-number or
/// prev-hash, so this record carries only the correlation key the artifact
/// genuinely provides — the forged hash + slot. Deriving a block-number or
/// prev-hash here would be new BLUE work (forbidden); the chain-point match
/// is the slot, which the artifact provides.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AdeForgeRecord {
    pub forged_block_hash: Hash32,
    pub slot: u64,
    pub network_magic: u32,
}

impl AdeForgeRecord {
    /// Build from the forge artifact `ForgeSucceeded` carries. Reads the
    /// already-minted `hash` + `slot` directly (no recomputation, no new
    /// BLUE authority — M2). `network_magic` is node-config context, not
    /// part of the forge artifact.
    pub fn from_forge_artifact(artifact: &ForgedBlockArtifact, network_magic: u32) -> Self {
        Self {
            forged_block_hash: Hash32(artifact.hash),
            slot: artifact.slot,
            network_magic,
        }
    }
}

/// Closed peer-accept event sum. ONLY these two forms count as a peer
/// accepting an Ade-forged block; the parser allow-list refuses to
/// construct any other signal as acceptance.
///
/// `block_hash` is the REQUIRED correlation key (hash-primary evidence).
/// `slot` is OPTIONAL context: a peer signal may omit it; when present it
/// must agree with the Ade forge record's slot (a present-but-contradicting
/// slot is a mismatch, never a silent match).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PeerAcceptEvent {
    /// STRONGEST: the peer served block `block_hash` back on its
    /// chain-serving path (`peer` is the source address/label). `slot` is
    /// optional context.
    PeerServedBlock {
        block_hash: Hash32,
        slot: Option<u64>,
        peer: String,
    },
    /// Corroborating: the peer's chain tip names `block_hash`. `slot` is
    /// optional context.
    PeerChainTip {
        block_hash: Hash32,
        slot: Option<u64>,
        peer: String,
    },
}

impl PeerAcceptEvent {
    fn block_hash(&self) -> &Hash32 {
        match self {
            Self::PeerServedBlock { block_hash, .. } | Self::PeerChainTip { block_hash, .. } => {
                block_hash
            }
        }
    }
    /// Optional peer-provided slot context.
    fn slot(&self) -> Option<u64> {
        match self {
            Self::PeerServedBlock { slot, .. } | Self::PeerChainTip { slot, .. } => *slot,
        }
    }
    fn peer(&self) -> &str {
        match self {
            Self::PeerServedBlock { peer, .. } | Self::PeerChainTip { peer, .. } => peer,
        }
    }
    fn is_served(&self) -> bool {
        matches!(self, Self::PeerServedBlock { .. })
    }
}

/// Typed provenance of the accepting signal(s) recorded in the manifest.
/// `PeerServedBlock` is primary when present.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PeerAcceptSource {
    /// Backed by a served-block signal only (strongest).
    ServedBlock,
    /// Backed by a chain-tip signal only.
    ChainTip,
    /// Backed by BOTH a served-block and a chain-tip, which agree;
    /// served-block is primary.
    ServedBlockAndChainTip,
}

/// Closed reason sum for [`BA02Outcome::NoEvidence`]. NoEvidence is the
/// DEFAULT; a manifest is the exception.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NoEvidenceReason {
    /// No peer-accept signal names the forged hash or the forged slot
    /// (includes the case where the log held only weaker/unknown signals
    /// the allow-list dropped, and the empty-log case).
    NoPeerAccept,
    /// A peer-accept signal at the forged slot names a DIFFERENT hash (the
    /// peer accepted some other block at that chain position).
    HashMismatch,
    /// A peer-accept signal names the forged hash but at a DIFFERENT slot
    /// (stale / wrong chain-point context).
    ChainPointMismatch,
    /// At the forged context, peer-accept signals DISAGREE (e.g. a
    /// served-block names the forged hash but a chain-tip at the same slot
    /// names a different hash). A tip must never paper over a served-block
    /// disagreement.
    ConflictingPeerSignals,
}

/// Canonical, versioned BA-02 evidence manifest. Hashes are lowercase hex
/// strings (the existing transcript convention) so the manifest is a
/// stable serde value independent of `Hash32`'s wire repr.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Ba02Manifest {
    pub schema_version: u32,
    pub forged_block_hash_hex: String,
    pub slot: u64,
    pub network_magic: u32,
    pub peer_accept_source: PeerAcceptSource,
    pub peer: String,
    /// Always equals `forged_block_hash_hex` AND every accepting signal's
    /// hash — recorded explicitly so the match is auditable in the artifact.
    pub matched_block_hash_hex: String,
}

impl Ba02Manifest {
    /// Deterministic canonical JSON (serde preserves struct field order;
    /// all fields are String/u32/u64/closed-enum, so two encodes are
    /// byte-identical).
    pub fn to_canonical_json(&self) -> String {
        serde_json::to_string(self).expect("Ba02Manifest is always serializable")
    }
}

/// Closed correlation outcome. `NoEvidence` is the default.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BA02Outcome {
    /// An exact forged-hash <-> peer-accept match at the matching chain
    /// point, with no conflicting signal. The ONLY constructor of a
    /// manifest is [`correlate`]'s exact-match arm.
    Ba02Manifest(Ba02Manifest),
    /// Anything weaker, mismatched, conflicting, or absent.
    NoEvidence { reason: NoEvidenceReason },
}

/// Parse a peer-accept JSONL log into the closed [`PeerAcceptEvent`] set.
///
/// ALLOW-LIST: only the `peer_served_block` and `peer_chain_tip`
/// discriminators are recognized. Every other `event` (a weaker signal,
/// an unknown line, or malformed JSON) is DROPPED — never coerced into an
/// acceptance. A recognized line missing its `block_hash_hex`/`slot`, or
/// with a malformed hash, is also dropped (fail-closed: a malformed accept
/// is no accept).
pub fn parse_peer_accept_events(log: &str) -> Vec<PeerAcceptEvent> {
    let mut out = Vec::new();
    for line in log.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let v: serde_json::Value = match serde_json::from_str(line) {
            Ok(v) => v,
            Err(_) => continue,
        };
        let event = v.get("event").and_then(|e| e.as_str()).unwrap_or("");
        // Allow-list: only these two discriminators name an acceptance.
        let served = match event {
            "peer_served_block" => true,
            "peer_chain_tip" => false,
            _ => continue,
        };
        let block_hash = v
            .get("block_hash_hex")
            .and_then(|x| x.as_str())
            .and_then(parse_hash32_hex);
        // slot is OPTIONAL context. Absent => None (still a valid
        // hash-primary signal). Present-but-non-integer is treated as a
        // malformed line and dropped (a garbled slot must not silently
        // become "no slot").
        let slot_field = v.get("slot");
        let slot: Option<u64> = match slot_field {
            None => None,
            Some(s) => match s.as_u64() {
                Some(n) => Some(n),
                None => continue, // present but not a u64 => malformed, drop.
            },
        };
        let peer = v
            .get("peer")
            .and_then(|x| x.as_str())
            .map(String::from)
            .unwrap_or_default();
        // block_hash is the REQUIRED key; a missing/short hash drops the line.
        let Some(block_hash) = block_hash else {
            continue;
        };
        out.push(if served {
            PeerAcceptEvent::PeerServedBlock {
                block_hash,
                slot,
                peer,
            }
        } else {
            PeerAcceptEvent::PeerChainTip {
                block_hash,
                slot,
                peer,
            }
        });
    }
    out
}

/// Correlate the Ade forge record against the peer-accept signals.
///
/// Pure, deterministic, total. HASH-PRIMARY: the peer-accept `block_hash`
/// MUST equal `ade.forged_block_hash`. `slot` is optional context — when a
/// signal provides it, it must equal `ade.slot`; when omitted, the signal
/// still matches on hash (it must not CONTRADICT the forge record, but it
/// may be silent about slot). Returns [`BA02Outcome::Ba02Manifest`] ONLY on
/// a hash match with no contradicting context and no conflicting signal;
/// every other case is [`BA02Outcome::NoEvidence`] with a typed reason.
/// This function is the SOLE constructor of a [`Ba02Manifest`].
pub fn correlate(ade: &AdeForgeRecord, peer_log: &[PeerAcceptEvent]) -> BA02Outcome {
    let forged = &ade.forged_block_hash;

    let mut served_exact = false;
    let mut tip_exact = false;
    let mut served_peer: Option<String> = None;
    let mut tip_peer: Option<String> = None;
    // A signal naming the forged hash but at a CONTRADICTING slot.
    let mut hash_ok_slot_contradicts = false;
    // A signal at the forged slot naming a DIFFERENT hash.
    let mut slot_ok_hash_wrong = false;

    for ev in peer_log {
        let hash_ok = ev.block_hash() == forged;
        // slot context: matches if absent (silent) OR equal to the forge
        // slot; contradicts only if present AND different.
        let slot_present_and_diff = matches!(ev.slot(), Some(s) if s != ade.slot);
        let slot_compatible = !slot_present_and_diff;

        if hash_ok && slot_compatible {
            if ev.is_served() {
                served_exact = true;
                served_peer = Some(ev.peer().to_string());
            } else {
                tip_exact = true;
                tip_peer = Some(ev.peer().to_string());
            }
        } else if hash_ok && slot_present_and_diff {
            // Forged hash named, but the peer puts it at a different slot:
            // contradicting context.
            hash_ok_slot_contradicts = true;
        } else if !hash_ok && ev.slot() == Some(ade.slot) {
            // A different hash claimed at the forged slot.
            slot_ok_hash_wrong = true;
        }
        // else: unrelated signal (different block, no overlapping context)
        // — ignored.
    }

    let has_exact = served_exact || tip_exact;

    // Conflict: a matching accept AND a contradicting signal — either a
    // different hash at the forged slot, or the forged hash placed at a
    // different slot. A weaker/optional signal must never paper over a
    // disagreement.
    if has_exact && (slot_ok_hash_wrong || hash_ok_slot_contradicts) {
        return BA02Outcome::NoEvidence {
            reason: NoEvidenceReason::ConflictingPeerSignals,
        };
    }

    if has_exact {
        let source = match (served_exact, tip_exact) {
            (true, true) => PeerAcceptSource::ServedBlockAndChainTip,
            (true, false) => PeerAcceptSource::ServedBlock,
            (false, true) => PeerAcceptSource::ChainTip,
            (false, false) => unreachable!("has_exact implies one signal present"),
        };
        // Served-block is primary when present.
        let peer = served_peer.or(tip_peer).unwrap_or_default();
        let forged_hex = hex32(forged);
        return BA02Outcome::Ba02Manifest(Ba02Manifest {
            schema_version: BA02_MANIFEST_SCHEMA_VERSION,
            forged_block_hash_hex: forged_hex.clone(),
            slot: ade.slot,
            network_magic: ade.network_magic,
            peer_accept_source: source,
            peer,
            matched_block_hash_hex: forged_hex,
        });
    }

    // No matching accept — classify the strongest near-miss.
    if slot_ok_hash_wrong {
        return BA02Outcome::NoEvidence {
            reason: NoEvidenceReason::HashMismatch,
        };
    }
    if hash_ok_slot_contradicts {
        return BA02Outcome::NoEvidence {
            reason: NoEvidenceReason::ChainPointMismatch,
        };
    }
    BA02Outcome::NoEvidence {
        reason: NoEvidenceReason::NoPeerAccept,
    }
}

/// Lowercase hex of a 32-byte hash (mirrors the transcript convention).
fn hex32(h: &Hash32) -> String {
    let mut s = String::with_capacity(64);
    for b in &h.0 {
        s.push(char::from_digit((b >> 4) as u32, 16).unwrap_or('0'));
        s.push(char::from_digit((b & 0x0f) as u32, 16).unwrap_or('0'));
    }
    s
}

/// Parse a 64-hex-char string into a 32-byte hash. `None` on wrong length
/// or non-hex (mirrors `node_lifecycle::parse_hash32`).
fn parse_hash32_hex(hex: &str) -> Option<Hash32> {
    if hex.len() != 64 {
        return None;
    }
    let mut out = [0u8; 32];
    for i in 0..32 {
        let pair = hex.get(i * 2..i * 2 + 2)?;
        out[i] = u8::from_str_radix(pair, 16).ok()?;
    }
    Some(Hash32(out))
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
#[allow(clippy::expect_used)]
mod tests {
    use super::*;

    const FORGED: Hash32 = Hash32([0xD1; 32]);
    const OTHER: Hash32 = Hash32([0x99; 32]);
    const FORGED_SLOT: u64 = 124_140_368;

    fn forge() -> AdeForgeRecord {
        AdeForgeRecord {
            forged_block_hash: FORGED,
            slot: FORGED_SLOT,
            network_magic: 1,
        }
    }

    fn served(h: Hash32, slot: u64) -> PeerAcceptEvent {
        PeerAcceptEvent::PeerServedBlock {
            block_hash: h,
            slot: Some(slot),
            peer: "127.0.0.1:3001".to_string(),
        }
    }
    fn tip(h: Hash32, slot: u64) -> PeerAcceptEvent {
        PeerAcceptEvent::PeerChainTip {
            block_hash: h,
            slot: Some(slot),
            peer: "127.0.0.1:3001".to_string(),
        }
    }
    /// A served-block signal that OMITS slot context (hash-primary only).
    fn served_no_slot(h: Hash32) -> PeerAcceptEvent {
        PeerAcceptEvent::PeerServedBlock {
            block_hash: h,
            slot: None,
            peer: "127.0.0.1:3001".to_string(),
        }
    }

    fn manifest(out: BA02Outcome) -> Ba02Manifest {
        match out {
            BA02Outcome::Ba02Manifest(m) => m,
            other => panic!("expected Ba02Manifest, got {other:?}"),
        }
    }
    fn no_evidence(out: BA02Outcome) -> NoEvidenceReason {
        match out {
            BA02Outcome::NoEvidence { reason } => reason,
            other => panic!("expected NoEvidence, got {other:?}"),
        }
    }

    // ===== schema =====

    #[test]
    fn ba02_manifest_schema_round_trips() {
        let m = manifest(correlate(&forge(), &[served(FORGED, FORGED_SLOT)]));
        let json = m.to_canonical_json();
        let decoded: Ba02Manifest = serde_json::from_str(&json).unwrap();
        assert_eq!(decoded, m);
        // Encode -> decode -> encode is byte-identical.
        assert_eq!(decoded.to_canonical_json(), json);
        // Versioned tag present.
        assert_eq!(m.schema_version, BA02_MANIFEST_SCHEMA_VERSION);
    }

    // ===== positives (synthetic — prove mechanics, NOT a BA-02 claim) =====
    // THROWAWAY SYNTHETIC FIXTURES. The hashes below are fabricated; a real
    // BA-02 manifest requires a real operator-captured peer log. These
    // tests prove the correlation mechanics only.

    #[test]
    fn ba02_correlate_served_block_yields_manifest() {
        let m = manifest(correlate(&forge(), &[served(FORGED, FORGED_SLOT)]));
        assert_eq!(m.peer_accept_source, PeerAcceptSource::ServedBlock);
        assert_eq!(m.forged_block_hash_hex, m.matched_block_hash_hex);
        assert_eq!(m.slot, FORGED_SLOT);
    }

    #[test]
    fn ba02_correlate_chain_tip_only_yields_manifest() {
        let m = manifest(correlate(&forge(), &[tip(FORGED, FORGED_SLOT)]));
        assert_eq!(m.peer_accept_source, PeerAcceptSource::ChainTip);
    }

    #[test]
    fn ba02_correlate_served_block_without_slot_yields_manifest() {
        // Hash-primary: a served-block that OMITS slot context still matches
        // on the forged hash (it does not contradict the forge record).
        let m = manifest(correlate(&forge(), &[served_no_slot(FORGED)]));
        assert_eq!(m.peer_accept_source, PeerAcceptSource::ServedBlock);
        assert_eq!(m.forged_block_hash_hex, m.matched_block_hash_hex);
    }

    #[test]
    fn ba02_correlate_no_slot_wrong_hash_is_no_evidence() {
        // A slot-omitted signal naming a DIFFERENT hash is not acceptance —
        // hash is the required key.
        let r = correlate(&forge(), &[served_no_slot(OTHER)]);
        assert_eq!(no_evidence(r), NoEvidenceReason::NoPeerAccept);
    }

    #[test]
    fn ba02_correlate_both_signals_agree_records_served_primary() {
        let m = manifest(correlate(
            &forge(),
            &[served(FORGED, FORGED_SLOT), tip(FORGED, FORGED_SLOT)],
        ));
        assert_eq!(
            m.peer_accept_source,
            PeerAcceptSource::ServedBlockAndChainTip
        );
    }

    // ===== conflict + mismatch + stale -> NoEvidence =====

    #[test]
    fn ba02_correlate_conflicting_signals_is_no_evidence() {
        // served names the forged hash; tip at the same slot names a
        // different hash -> conflict, tip must not paper over it.
        let r = correlate(
            &forge(),
            &[served(FORGED, FORGED_SLOT), tip(OTHER, FORGED_SLOT)],
        );
        assert_eq!(no_evidence(r), NoEvidenceReason::ConflictingPeerSignals);
    }

    #[test]
    fn ba02_correlate_wrong_hash_is_no_evidence() {
        // Peer accepted a different block at the forged slot.
        let r = correlate(&forge(), &[served(OTHER, FORGED_SLOT)]);
        assert_eq!(no_evidence(r), NoEvidenceReason::HashMismatch);
    }

    #[test]
    fn ba02_correlate_chain_point_mismatch_is_no_evidence() {
        // Forged hash named, but at the wrong slot.
        let r = correlate(&forge(), &[served(FORGED, FORGED_SLOT - 1)]);
        assert_eq!(no_evidence(r), NoEvidenceReason::ChainPointMismatch);
    }

    #[test]
    fn ba02_correlate_stale_log_is_no_evidence() {
        // A stale peer-accept: the forged hash appears far earlier on the
        // peer's chain (wrong slot) -> ChainPointMismatch, not a manifest.
        let r = correlate(&forge(), &[tip(FORGED, FORGED_SLOT - 100)]);
        assert_eq!(no_evidence(r), NoEvidenceReason::ChainPointMismatch);
    }

    #[test]
    fn ba02_correlate_empty_peer_log_is_no_evidence() {
        let r = correlate(&forge(), &[]);
        assert_eq!(no_evidence(r), NoEvidenceReason::NoPeerAccept);
    }

    // ===== weaker signals are not representable as acceptance =====
    // The parser allow-list drops every non-acceptance line, so a log of
    // weaker signals yields ZERO PeerAcceptEvents -> correlate -> NoEvidence.

    #[test]
    fn ba02_self_accept_is_not_evidence() {
        let log = r#"{"event":"self_accept","slot":124140368,"block_hash_hex":"d1d1d1d1d1d1d1d1d1d1d1d1d1d1d1d1d1d1d1d1d1d1d1d1d1d1d1d1d1d1d1d1"}"#;
        let events = parse_peer_accept_events(log);
        assert!(events.is_empty(), "self_accept must not parse to acceptance");
        assert_eq!(
            no_evidence(correlate(&forge(), &events)),
            NoEvidenceReason::NoPeerAccept
        );
    }

    #[test]
    fn ba02_forge_succeeded_alone_is_not_evidence() {
        let log = r#"{"event":"forge_succeeded","slot":124140368,"block_hash_hex":"d1d1d1d1d1d1d1d1d1d1d1d1d1d1d1d1d1d1d1d1d1d1d1d1d1d1d1d1d1d1d1d1"}"#;
        let events = parse_peer_accept_events(log);
        assert!(events.is_empty());
        assert_eq!(
            no_evidence(correlate(&forge(), &events)),
            NoEvidenceReason::NoPeerAccept
        );
    }

    #[test]
    fn ba02_block_received_alone_is_not_evidence() {
        let log = r#"{"event":"block_received","peer":"127.0.0.1:3001","slot":124140368,"block_hash_hex":"d1d1d1d1d1d1d1d1d1d1d1d1d1d1d1d1d1d1d1d1d1d1d1d1d1d1d1d1d1d1d1d1"}"#;
        let events = parse_peer_accept_events(log);
        assert!(
            events.is_empty(),
            "block_received is a pre-admit signal, not acceptance"
        );
        assert_eq!(
            no_evidence(correlate(&forge(), &events)),
            NoEvidenceReason::NoPeerAccept
        );
    }

    #[test]
    fn ba02_agreement_verdict_is_not_evidence() {
        let log = r#"{"event":"agreement_verdict","kind":"agreed","slot":124140368,"our_hash_hex":"d1d1d1d1d1d1d1d1d1d1d1d1d1d1d1d1d1d1d1d1d1d1d1d1d1d1d1d1d1d1d1d1"}"#;
        let events = parse_peer_accept_events(log);
        assert!(
            events.is_empty(),
            "an Ade-vs-Ade agreement verdict is not peer acceptance of an Ade-forged block"
        );
        assert_eq!(
            no_evidence(correlate(&forge(), &events)),
            NoEvidenceReason::NoPeerAccept
        );
    }

    #[test]
    fn ba02_parser_drops_unknown_and_malformed_lines() {
        let log = concat!(
            "not json at all\n",
            r#"{"event":"unknown_event","slot":1}"#,
            "\n",
            r#"{"event":"peer_served_block","slot":124140368}"#, // missing hash
            "\n",
            r#"{"event":"peer_served_block","block_hash_hex":"tooshort","slot":124140368}"#,
            "\n",
        );
        assert!(parse_peer_accept_events(log).is_empty());
    }

    #[test]
    fn ba02_parser_accepts_well_formed_served_block() {
        let h = "d1".repeat(32);
        let log = format!(
            r#"{{"event":"peer_served_block","peer":"p","slot":124140368,"block_hash_hex":"{h}"}}"#
        );
        let events = parse_peer_accept_events(&log);
        assert_eq!(events.len(), 1);
        // And it correlates to a manifest against the matching forge.
        let m = manifest(correlate(&forge(), &events));
        assert_eq!(m.peer_accept_source, PeerAcceptSource::ServedBlock);
    }

    // ===== determinism =====

    #[test]
    fn ba02_correlate_two_runs_byte_identical() {
        let log = &[served(FORGED, FORGED_SLOT), tip(FORGED, FORGED_SLOT)];
        let r1 = correlate(&forge(), log);
        let r2 = correlate(&forge(), log);
        assert_eq!(r1, r2);
        assert_eq!(manifest(r1).to_canonical_json(), manifest(r2).to_canonical_json());
    }
}
