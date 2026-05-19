// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types
// - Explicit state transitions only
// - Canonical serialization for all persisted/hashed data
//
// Version intersection rule for the N2N and N2C handshakes.
//
// Selection rule (DC-PROTO-05):
//   1. Intersect proposed.keys() with supported.keys().
//   2. Empty intersection => Refuse(VersionMismatch { proposed, supported }).
//   3. Else select max(intersection) — the highest version both sides
//      speak. Highest-common-version is the published Ouroboros rule.
//   4. Return (selected_version, supported[selected_version].version_data).
//
// Inputs are slices, never globals (DC-PROTO-06). The proposed table
// arrives as the peer-decoded codec `VersionTable`; the supported
// table is our `&'static [(u16, VersionData)]` constant.

use crate::codec::handshake::{HandshakeMessage, RefuseReason, VersionTable};
use crate::codec::n2c_handshake::{N2cHandshakeMessage, N2cRefuseReason, N2cVersionTable};
use crate::codec::version::{N2CVersion, N2NVersion};
use crate::handshake::state::{N2cVersionData, VersionData};

/// Outcome of `select_n2n_version` / `select_n2c_version`: either a
/// concrete (version, data) tuple to accept, or the structured refuse
/// reason to send back on empty intersection.
pub enum SelectionOutcome<V, D> {
    Selected(V, D),
    Mismatch(Vec<V>),
}

/// Intersect a peer-proposed N2N version table with the local
/// supported list. Pure function; no I/O, no globals.
pub fn select_n2n_version(
    proposed: &VersionTable,
    supported: &[(u16, VersionData)],
) -> SelectionOutcome<N2NVersion, VersionData> {
    let mut best: Option<(u16, VersionData)> = None;
    for (v_peer, _params) in &proposed.0 {
        let pv = v_peer.get();
        for (v_local, data_local) in supported {
            if *v_local == pv && best.map(|(b, _)| pv > b).unwrap_or(true) {
                best = Some((pv, *data_local));
            }
        }
    }
    match best {
        Some((v, d)) => SelectionOutcome::Selected(N2NVersion::new(v), d),
        None => {
            let supported_vs: Vec<N2NVersion> =
                supported.iter().map(|(v, _)| N2NVersion::new(*v)).collect();
            SelectionOutcome::Mismatch(supported_vs)
        }
    }
}

/// Intersect a peer-proposed N2C version table with the local
/// supported list. Same shape as `select_n2n_version`.
pub fn select_n2c_version(
    proposed: &N2cVersionTable,
    supported: &[(u16, N2cVersionData)],
) -> SelectionOutcome<N2CVersion, N2cVersionData> {
    let mut best: Option<(u16, N2cVersionData)> = None;
    for (v_peer, _params) in &proposed.0 {
        let pv = v_peer.get();
        for (v_local, data_local) in supported {
            if *v_local == pv && best.map(|(b, _)| pv > b).unwrap_or(true) {
                best = Some((pv, *data_local));
            }
        }
    }
    match best {
        Some((v, d)) => SelectionOutcome::Selected(N2CVersion::new(v), d),
        None => {
            let supported_vs: Vec<N2CVersion> =
                supported.iter().map(|(v, _)| N2CVersion::new(*v)).collect();
            SelectionOutcome::Mismatch(supported_vs)
        }
    }
}

/// Helper: project the proposed version numbers from a peer's
/// `VersionTable` as a Vec<u16> (used for diagnostics-only paths).
pub fn proposed_n2n_versions(proposed: &VersionTable) -> Vec<u16> {
    proposed.0.iter().map(|(v, _)| v.get()).collect()
}

/// Helper: project the proposed version numbers from a peer's
/// `N2cVersionTable`.
pub fn proposed_n2c_versions(proposed: &N2cVersionTable) -> Vec<u16> {
    proposed.0.iter().map(|(v, _)| v.get()).collect()
}

/// Helper: project the local supported version numbers as Vec<u16>.
pub fn supported_n2n_versions(supported: &[(u16, VersionData)]) -> Vec<u16> {
    supported.iter().map(|(v, _)| *v).collect()
}

/// Helper: project the local supported N2C version numbers.
pub fn supported_n2c_versions(supported: &[(u16, N2cVersionData)]) -> Vec<u16> {
    supported.iter().map(|(v, _)| *v).collect()
}

// Re-exports used by `transition` to build the right Reply / Refused
// payloads from selection outcomes. Keeping the constructors here
// keeps the codec types out of `transition`'s signature surface.
pub(crate) fn build_n2n_refuse(mismatch_vs: Vec<N2NVersion>) -> HandshakeMessage {
    HandshakeMessage::Refuse(RefuseReason::VersionMismatch(mismatch_vs))
}

pub(crate) fn build_n2c_refuse(mismatch_vs: Vec<N2CVersion>) -> N2cHandshakeMessage {
    N2cHandshakeMessage::Refuse(N2cRefuseReason::VersionMismatch(mismatch_vs))
}
