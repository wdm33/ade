// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types
// - Explicit state transitions only
// - Canonical serialization for all persisted/hashed data
//
// Handshake state machine types — pure values, no I/O, no async.
//
// `VersionData` models the negotiated per-version handshake-data
// payload for cardano-node 10.6.2 N2N versions 11..=14. The shape
// follows the IOG `ouroboros-network` published spec; precise byte-
// level fidelity is re-pinned against captured frames in S-A9 (see
// §17 of S-A3 slice doc). S-A3 verifies the state-machine surface
// against synthetic vectors derived from the spec.

use crate::codec::handshake::{HandshakeMessage, RefuseReason};
use crate::codec::n2c_handshake::{N2cHandshakeMessage, N2cRefuseReason};
use crate::codec::version::{N2CVersion, N2NVersion};

/// Peer-sharing willingness flag for N2N versions 11+.
///
/// Encoded on the wire as `NoPeerSharing = 0`, `PeerSharingPublic = 1`
/// per the ouroboros-network handshake-data CDDL. Closed enum: the
/// historical `PeerSharingPrivate` discriminant was dropped before
/// cardano-node 10.6.2 and is not represented here.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PeerSharingFlag {
    NoPeerSharing,
    PeerSharingPublic,
}

/// Negotiated N2N handshake-data payload (cardano-node 10.6.2, V11+).
///
/// Closed struct — every field is required for V11..=V14. The struct
/// is `Copy` because all fields are value-typed; the handshake state
/// machine clones it freely without allocation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VersionData {
    pub network_magic: u32,
    pub initiator_only_diffusion: bool,
    pub peer_sharing: PeerSharingFlag,
    pub query: bool,
}

/// Negotiated N2C handshake-data payload (cardano-node 10.6.2, V15+).
///
/// N2C handshake-data is narrower than N2N: only network magic and the
/// query flag are negotiated.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct N2cVersionData {
    pub network_magic: u32,
    pub query: bool,
}

/// Closed handshake state. The handshake is a single round-trip so
/// the state space is small: Idle → Proposed → (Done | Refused).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HandshakeState {
    Idle,
    Proposed,
    Done,
    Refused,
}

/// Output emitted by a successful N2N transition.
///
/// `Reply` carries the next on-wire message *value* — encoding to
/// bytes is the S-A2 codec's job, not the state machine's.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum N2nHandshakeOutput {
    Reply(HandshakeMessage),
    Selected(N2NVersion, VersionData),
    Refused(RefuseReason),
    Done,
}

/// Output emitted by a successful N2C transition (analogue of
/// `N2nHandshakeOutput`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum N2cHandshakeOutput {
    Reply(N2cHandshakeMessage),
    Selected(N2CVersion, N2cVersionData),
    Refused(N2cRefuseReason),
    Done,
}

/// Structured handshake errors. No `String`, no `anyhow`. The
/// `VersionMismatch` sets are `Vec<u16>` (raw version numbers,
/// protocol-family agnostic) so the same error type serves N2N and
/// N2C without coupling them to specific newtypes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HandshakeError {
    /// A (state, message, agency) triple that the protocol grammar
    /// forbids — e.g. server sending `ProposeVersions`, or client
    /// receiving `ProposeVersions` after already proposing.
    IllegalTransition {
        state: HandshakeState,
        message_tag: &'static str,
        agency: &'static str,
    },
    /// Decoded version not present in the local supported table.
    UnknownVersion { version: u16 },
    /// Empty intersection between proposed and supported version sets.
    /// Carries both sides for deterministic diagnostics.
    VersionMismatch { proposed_set: Vec<u16>, supported_set: Vec<u16> },
    /// Structurally-valid message that fails protocol-grammar invariants
    /// the codec does not check (e.g. empty version table from peer).
    MalformedMessage { reason: &'static str },
}
