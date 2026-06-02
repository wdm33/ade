// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types
// - Explicit state transitions only
// - Canonical serialization for all persisted/hashed data

//! GREEN private-testnet rehearsal evidence envelope (PHASE4-N-F-G-D S2).
//!
//! A [`PrivateRehearsalManifest`] WRAPS a correlate-produced
//! [`crate::ba02_evidence::Ba02Manifest`] payload (the SAME proof the bounty
//! BA-02 path produces) in a structurally distinct, NON-PROMOTABLE rehearsal
//! envelope. It exists ONLY to record that the exact `--mode node`
//! accepted-block path was exercised end-to-end against a real Haskell peer on
//! a C1 private testnet, as a bounty DRY-RUN.
//!
//! Non-promotability (CN-REHEARSAL-FIDELITY-01 clause 2):
//!   - the SOLE constructor wraps a [`BA02Outcome::Ba02Manifest`] —
//!     `NoEvidence` yields `None` (nothing to wrap, nothing to write), so a
//!     rehearsal manifest is ALWAYS correlate-produced (no synthetic manifest,
//!     no alternate correlator);
//!   - [`PrivateRehearsalManifest::to_canonical_toml`] ALWAYS emits
//!     `is_rehearsal = true` + `not_bounty_evidence = true` as literals — the
//!     type cannot represent a non-rehearsal;
//!   - the manifest lives ONLY under the rehearsal home
//!     (`docs/evidence/phase4-n-f-g-d-private-rehearsal-*.toml`), never the
//!     bounty home / `CE-G-C-LIVE_*` (enforced by
//!     `ci/ci_check_rehearsal_manifest_schema.sh`);
//!   - it flips NO RO-LIVE rule. C1 acceptance != bounty completion.
//!
//! `[[feedback-shell-must-not-overstate-semantic-truth]]`: Ade self-accept /
//! served bytes / wire success are NOT acceptance; only the Haskell peer log
//! through `ba02_evidence::correlate` is (the allow-list is inherited verbatim).

use crate::ba02_evidence::{Ba02Manifest, BA02Outcome, PeerAcceptSource};

/// Schema version of the rehearsal envelope (distinct from the wrapped
/// `Ba02Manifest`'s `BA02_MANIFEST_SCHEMA_VERSION`). Bump on any envelope-field
/// change.
pub const REHEARSAL_MANIFEST_SCHEMA_VERSION: u32 = 1;

/// Closed rehearsal venue. The dry-run is a private testnet ONLY; this enum
/// makes a non-private venue unrepresentable (a rehearsal is never preprod /
/// preview — that is the bounty surface, captured separately, never here).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RehearsalVenue {
    /// A C1 private testnet where the operator controls the genesis stake.
    PrivateTestnetC1,
}

impl RehearsalVenue {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::PrivateTestnetC1 => "private-testnet-c1",
        }
    }
}

/// Operator-supplied rehearsal envelope metadata: the venue + the committed
/// peer-log filename + its sha256. The operator computes the sha256 of the
/// captured peer log; `ci/ci_check_rehearsal_manifest_schema.sh` re-verifies it
/// against the committed file (the no-synthetic binding).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RehearsalEnvelope {
    pub venue: RehearsalVenue,
    pub peer_log_file: String,
    pub peer_log_file_sha256: String,
}

/// A non-promotable private-testnet rehearsal manifest: a correlate-produced
/// [`Ba02Manifest`] payload wrapped in the rehearsal envelope.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PrivateRehearsalManifest {
    /// The correlate-produced proof payload, wrapped verbatim. The SOLE source
    /// is `ba02_evidence::correlate`, via [`Self::from_correlate_outcome`].
    pub ba02: Ba02Manifest,
    pub venue: RehearsalVenue,
    pub peer_log_file: String,
    pub peer_log_file_sha256: String,
}

impl PrivateRehearsalManifest {
    /// SOLE constructor. Wraps the correlate-produced payload; `NoEvidence`
    /// yields `None` (nothing to wrap, nothing to write). A rehearsal manifest
    /// is therefore ALWAYS correlate-produced — there is no path from raw
    /// operator input or a `NoEvidence` outcome to a manifest.
    pub fn from_correlate_outcome(
        outcome: &BA02Outcome,
        envelope: RehearsalEnvelope,
    ) -> Option<Self> {
        match outcome {
            BA02Outcome::Ba02Manifest(m) => Some(Self {
                ba02: m.clone(),
                venue: envelope.venue,
                peer_log_file: envelope.peer_log_file,
                peer_log_file_sha256: envelope.peer_log_file_sha256,
            }),
            BA02Outcome::NoEvidence { .. } => None,
        }
    }

    /// Deterministic canonical TOML — the committed
    /// `docs/evidence/phase4-n-f-g-d-private-rehearsal-*.toml`. `is_rehearsal` +
    /// `not_bounty_evidence` are literal `true`: the type cannot serialize a
    /// non-rehearsal. Flat key=value shape (gate-greppable).
    pub fn to_canonical_toml(&self) -> String {
        let source = match self.ba02.peer_accept_source {
            PeerAcceptSource::ServedBlock => "served_block",
            PeerAcceptSource::ChainTip => "chain_tip",
            PeerAcceptSource::ServedBlockAndChainTip => "served_block_and_chain_tip",
        };
        format!(
            "schema_version = {schema}\n\
             venue = \"{venue}\"\n\
             is_rehearsal = true\n\
             not_bounty_evidence = true\n\
             peer_log_file = \"{peer_log_file}\"\n\
             peer_log_file_sha256 = \"{sha}\"\n\
             forged_block_hash_hex = \"{forged}\"\n\
             slot = {slot}\n\
             network_magic = {magic}\n\
             peer_accept_source = \"{source}\"\n\
             peer = \"{peer}\"\n\
             matched_block_hash_hex = \"{matched}\"\n",
            schema = REHEARSAL_MANIFEST_SCHEMA_VERSION,
            venue = self.venue.as_str(),
            peer_log_file = toml_escape(&self.peer_log_file),
            sha = toml_escape(&self.peer_log_file_sha256),
            forged = toml_escape(&self.ba02.forged_block_hash_hex),
            slot = self.ba02.slot,
            magic = self.ba02.network_magic,
            source = source,
            peer = toml_escape(&self.ba02.peer),
            matched = toml_escape(&self.ba02.matched_block_hash_hex),
        )
    }
}

/// Minimal TOML basic-string escape (backslash + double-quote). Deterministic.
/// The hex / venue / source fields are already escape-free; `peer` and
/// `peer_log_file` are operator-supplied, so escape them.
fn toml_escape(s: &str) -> String {
    s.replace('\\', "\\\\").replace('"', "\\\"")
}
