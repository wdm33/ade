// TCB: BLUE — deterministic, authoritative.
// - No wall-clock, no rand, no floating point, no I/O
// - Pure data + total functions over it
//! PREPROD-ENTRY-AUTHORITY P6 (P4-S2) — the AUTHORITY SEMANTICS version.
//!
//! This is **not** a byte-layout version. Every existing version constant in the tree
//! (`FROZEN_LEADERSHIP_SCHEMA_VERSION`, `SEED_CINPUT_SCHEMA_VERSION`,
//! `BOOTSTRAP_RUPD_SCHEMA_VERSION`, `RECOVERED_ANCHOR_POINT_SCHEMA_VERSION`, the ChainDb
//! `SCHEMA_VERSION`) answers one question:
//!
//! > *Can I parse these bytes?*
//!
//! This one answers a different question:
//!
//! > *Were these authoritative bytes PRODUCED by the same rules this binary now implements?*
//!
//! P4 (`e1de7a2e`) is the case that proves the two are not the same. P3 changed the authoritative
//! epoch-boundary rule and changed **no byte layout whatsoever** — every durable object still decoded
//! cleanly at its current schema version. The store was structurally valid and semantically stale, and
//! nothing in the system could express that state. The failure surfaced three epochs later as an
//! opaque recovery fingerprint mismatch.
//!
//! ## The rule (strict, no escape hatch)
//!
//! ```text
//! marker == STORE_SEMANTICS_VERSION -> the artifact may be used
//! marker absent                     -> typed terminal, re-bootstrap required
//! marker older                      -> typed terminal, re-bootstrap required
//! marker newer / unknown            -> typed terminal, re-bootstrap required
//! ```
//!
//! There is deliberately **no stamp tool and no override**. An operator asserting "this unmarked store
//! is fine" is exactly the judgement that failed in P4 — that store looked structurally perfect and was
//! three epochs stale. A future migration is permitted ONLY as a sealed migration proof (read an old
//! MARKED store, prove a deterministic `old_semantics -> new_semantics` transform, write a new store,
//! emit evidence) — never as a stamp.
//!
//! ## Encoding may migrate; semantics may not
//!
//! The ChainDb already encodes this asymmetry for its own two axes: `SCHEMA_VERSION` (layout) upgrades
//! forward on the next write, while `FINGERPRINT_VERSION` (meaning) hard-fails in either direction.
//! This version follows the `FINGERPRINT_VERSION` discipline, and is ORTHOGONAL to it: the two change
//! independently, and folding them together would force spurious re-bootstraps.

/// The authority-semantics version this binary produces and requires.
///
/// **Bump this whenever authoritative ledger/epoch/leadership PRODUCTION rules change**, i.e. whenever
/// a store written by the previous binary would no longer be replay-equivalent under this one. The bump
/// is not left to memory: `ci/ci_check_store_semantics_lock.sh` hashes the declared semantics-bearing
/// surface and fails until the change is reconciled with `ci/store-semantics-surface.lock`, forcing an
/// explicit choice between a bump and a recorded semantics-neutral declaration.
///
/// Version 1 is the first marked generation. Every store produced before P6 is unmarked and is
/// therefore rejected — see the module docs for why no stamp path exists.
///
/// Version 2 (PREPROD-NONCE-2, CE-N2-4): the seed+1 bootstrap-bridge boundary now commits the
/// BOUNDARY-TICK `eta0` into its durable activation record, not the bridge's seed-time projection. A
/// store written by an earlier binary from a seed that PRECEDED the candidate freeze holds a
/// `nonce_commitment` this binary will never reproduce — preprod 304→305 wrote `e3402a2b…` where the
/// real `epochNonce(305)` is `74f10bea…`. Without the bump that store surfaces as an opaque
/// `EpochViewPostPromotionMismatch` at warm-start; with it, the store is refused up front with a
/// typed re-bootstrap terminal, which is the whole reason this constant exists.
/// Version 3 (CRE-S7): Conway `ParameterChange` enactment now supports `minPoolCost` (key 16), so a
/// boundary that ratifies it ENACTS it instead of stalling. Replaying the same blocks under this binary
/// therefore produces different protocol params — and `minPoolCost` is fingerprinted — so a store
/// written by an earlier binary is not replay-equivalent. Proven on preprod: `e641ec80…#0` enacts
/// `minPoolCost 170_000_000 -> 75_000_000` at the 304→305 boundary.
pub const STORE_SEMANTICS_VERSION: u32 = 3;

/// A durable artifact that carries authoritative semantics and therefore must be version-marked.
///
/// Closed by construction: adding a new authority-bearing store is a compile-time obligation to say
/// which artifact it is, so it cannot quietly join the set unmarked.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AuthorityArtifact {
    /// `chain.db` — the ledger-snapshot / sidecar / anchor-point authority (and, by shared data-dir
    /// lineage, the WAL fingerprint chain written in lockstep with it).
    ChainDb,
    /// `epoch-accumulator.redb` — the epoch-nonce and frozen-leadership authority.
    EpochAccumulator,
    /// `reduced-checkpoint.redb` — the reduced-validation-plane UTxO authority.
    ReducedCheckpoint,
}

impl AuthorityArtifact {
    pub fn as_str(self) -> &'static str {
        match self {
            AuthorityArtifact::ChainDb => "chain.db",
            AuthorityArtifact::EpochAccumulator => "epoch-accumulator.redb",
            AuthorityArtifact::ReducedCheckpoint => "reduced-checkpoint.redb",
        }
    }
}

/// What was found on the artifact. `Absent` is a distinct state, not a sentinel number — a legacy
/// (pre-P6) store is rejected for a different reason than a version-skewed one, and the operator
/// message differs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FoundSemanticsVersion {
    /// No marker at all — a store produced before the gate existed.
    Absent,
    Version(u32),
}

/// The ONLY remediation this gate offers. There is no `Stamp` and no `ContinueAnyway` variant, and
/// that absence is the invariant: the enum cannot express "trust me".
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RemediationAction {
    RebootstrapRequired,
}

/// A durable authority artifact disagrees with this binary about the meaning of its bytes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StoreSemanticsVersionMismatch {
    pub artifact: AuthorityArtifact,
    pub found: FoundSemanticsVersion,
    pub required: u32,
    pub action: RemediationAction,
}

impl core::fmt::Display for StoreSemanticsVersionMismatch {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self.found {
            FoundSemanticsVersion::Absent => write!(
                f,
                "{} carries NO authority-semantics marker (pre-P6 store); this binary requires v{}. \
                 The bytes may parse cleanly and still have been produced by different rules -- \
                 re-bootstrap required (there is no stamp path, by design)",
                self.artifact.as_str(),
                self.required
            ),
            FoundSemanticsVersion::Version(found) => write!(
                f,
                "{} was produced under authority-semantics v{} but this binary implements v{} -- \
                 re-bootstrap required (no implicit migration; a migration must be a sealed proof)",
                self.artifact.as_str(),
                found,
                self.required
            ),
        }
    }
}

/// DC-STORE-10 — the gate. Total, pure, and strict in every direction.
///
/// `found == None` models an absent marker. Equality with [`STORE_SEMANTICS_VERSION`] is the ONLY
/// accepting case: older, newer, and absent all fail closed with the same remediation.
pub fn check_store_semantics_version(
    artifact: AuthorityArtifact,
    found: Option<u32>,
) -> Result<(), StoreSemanticsVersionMismatch> {
    match found {
        Some(v) if v == STORE_SEMANTICS_VERSION => Ok(()),
        other => Err(StoreSemanticsVersionMismatch {
            artifact,
            found: match other {
                None => FoundSemanticsVersion::Absent,
                Some(v) => FoundSemanticsVersion::Version(v),
            },
            required: STORE_SEMANTICS_VERSION,
            action: RemediationAction::RebootstrapRequired,
        }),
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
#[allow(clippy::expect_used)]
mod tests {
    use super::*;

    #[test]
    fn current_marker_is_accepted() {
        for a in [
            AuthorityArtifact::ChainDb,
            AuthorityArtifact::EpochAccumulator,
            AuthorityArtifact::ReducedCheckpoint,
        ] {
            assert!(check_store_semantics_version(a, Some(STORE_SEMANTICS_VERSION)).is_ok());
        }
    }

    /// CE-P6-3: a pre-P6 store has no marker and is rejected as a LEGACY store, distinctly from a
    /// version-skewed one.
    #[test]
    fn absent_marker_is_rejected_as_legacy() {
        let e = check_store_semantics_version(AuthorityArtifact::EpochAccumulator, None)
            .expect_err("an unmarked store must fail closed");
        assert_eq!(e.found, FoundSemanticsVersion::Absent);
        assert_eq!(e.action, RemediationAction::RebootstrapRequired);
        assert!(e.to_string().contains("NO authority-semantics marker"));
    }

    /// CE-P6-4: older marker -> re-bootstrap required.
    #[test]
    fn older_marker_is_rejected() {
        let e = check_store_semantics_version(AuthorityArtifact::ChainDb, Some(0))
            .expect_err("an older marker must fail closed");
        assert_eq!(e.found, FoundSemanticsVersion::Version(0));
        assert_eq!(e.action, RemediationAction::RebootstrapRequired);
    }

    /// CE-P6-5: a FUTURE marker fails closed too -- a newer binary's store is not readable by an
    /// older one, and guessing is exactly the failure mode this gate exists to remove.
    #[test]
    fn future_marker_is_rejected() {
        let e = check_store_semantics_version(
            AuthorityArtifact::ReducedCheckpoint,
            Some(STORE_SEMANTICS_VERSION + 1),
        )
        .expect_err("a future marker must fail closed");
        assert_eq!(
            e.found,
            FoundSemanticsVersion::Version(STORE_SEMANTICS_VERSION + 1)
        );
    }

    /// The remediation surface is closed: there is exactly one action, so the type system cannot
    /// express "stamp it" or "continue anyway".
    #[test]
    fn the_only_remediation_is_rebootstrap() {
        let e = check_store_semantics_version(AuthorityArtifact::ChainDb, None).unwrap_err();
        match e.action {
            RemediationAction::RebootstrapRequired => {}
        }
    }
}
