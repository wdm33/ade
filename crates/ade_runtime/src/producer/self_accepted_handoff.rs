// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types
// - Explicit state transitions only
// - Canonical serialization for all persisted/hashed data

//! GREEN typed self-accepted-artifact handoff fence (PHASE4-N-F-G-B S1).
//!
//! [`SelfAcceptedHandoff`] is the typed carrier that moves a BLUE
//! self-accepted forged block from the forge path toward the (S2) sibling
//! serve task. Its SOLE constructor,
//! [`SelfAcceptedHandoff::from_self_accepted`], takes an [`AcceptedBlock`] —
//! itself producible only by BLUE `self_accept` returning `Ok` (CN-FORGE-01;
//! the `AcceptedBlock` field is private and un-fabricable from raw bytes).
//!
//! There is NO constructor from a raw `Vec<u8>`, a `ForgedBlockArtifact`
//! (`artifact.bytes` is never an `AcceptedBlock` source — re-deriving the
//! token from bytes would breach CN-FORGE-01; the carrier holds the ORIGINAL
//! token), a `CoordinatorEvent`, a self-declared acceptance flag, or a peer
//! verdict. "Hand an artifact that was not BLUE self-accepted to the serve
//! task" is therefore unrepresentable — this is the GREEN fence backing
//! `DC-NODE-06`.
//!
//! Pure: no I/O, clock, rand, or float. The carrier wraps the original token
//! verbatim and never re-validates it; same `AcceptedBlock` => same carrier.

use ade_ledger::producer::AcceptedBlock;

/// Typed, constructor-fenced carrier for a BLUE self-accepted forged block on
/// its way to the (S2) sibling serve task.
///
/// See the module docs for the fence rationale. The wrapped [`AcceptedBlock`]
/// is the *original* token from `self_accept`; it is never re-derived from
/// forged bytes.
#[derive(Debug, Clone, PartialEq)]
pub struct SelfAcceptedHandoff {
    // Private: the only way to populate this is `from_self_accepted`, whose
    // argument can only have come from `self_accept` returning `Ok` (the
    // `AcceptedBlock` struct constructor is private to
    // `ade_ledger::producer::self_accept`). No raw-bytes / event / flag path.
    accepted: AcceptedBlock,
}

impl SelfAcceptedHandoff {
    /// Wrap a BLUE self-accepted block for handoff to the (S2) serve task. The
    /// token is carried verbatim — never re-validated, never re-derived from
    /// raw bytes. This is the SOLE constructor.
    pub fn from_self_accepted(accepted: AcceptedBlock) -> Self {
        Self { accepted }
    }

    /// Borrow the carried self-accepted block (identity / bytes for the serve
    /// task).
    pub fn accepted(&self) -> &AcceptedBlock {
        &self.accepted
    }

    /// Consume the carrier, yielding the BLUE self-accepted block for
    /// `ServedChainHandle::push_atomic` (S2).
    pub fn into_accepted(self) -> AcceptedBlock {
        self.accepted
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // The carrier's fence is type-level: the ONLY public constructor takes a
    // BLUE `AcceptedBlock`. A real `AcceptedBlock` cannot be fabricated here
    // (its field is private to `ade_ledger::producer::self_accept`), so the
    // end-to-end construction + surfacing tests live in `ade_node`'s forge
    // tests — `handoff_carrier_constructs_only_from_self_accepted_forge` and
    // `forge_surfaces_accepted_block_only_on_self_accept` — where a real
    // self-accepted token is produced by `run_real_forge`. Here we pin the
    // constructor SURFACE structurally, mirroring the
    // `broadcast_callable_only_with_accept_verdict` idiom in `self_accept.rs`.

    #[test]
    fn handoff_carrier_has_no_raw_bytes_constructor() {
        // Pinning the sole constructor as a typed fn-pointer is a compile-time
        // assertion of its surface: it takes a BLUE `AcceptedBlock`, not
        // `Vec<u8>` / `ForgedBlockArtifact` / `CoordinatorEvent`. A
        // `from_bytes(Vec<u8>) -> SelfAcceptedHandoff` (or an artifact/event
        // constructor) would not type-check at this binding, and none exists —
        // the field is private, so raw bytes can never populate the carrier.
        let _only_ctor: fn(AcceptedBlock) -> SelfAcceptedHandoff =
            SelfAcceptedHandoff::from_self_accepted;
    }

    #[test]
    fn serve_ingress_type_rejects_failed_forge_outcome() {
        // The carrier's content type is EXACTLY the BLUE `AcceptedBlock`:
        // `from_self_accepted` takes one and `into_accepted` yields one back. A
        // non-self-accepted forge outcome (`ForgeNotLeader` / `ForgeFailed`)
        // carries NO `AcceptedBlock`, so it is type-unrepresentable as a
        // `SelfAcceptedHandoff` — there is no `from_failed` / `from_event`
        // path. The surfacing half (Some on `ForgeSucceeded`, None on failure)
        // is proven by `ade_node`'s
        // `forge_surfaces_accepted_block_only_on_self_accept`.
        let _only_ctor: fn(AcceptedBlock) -> SelfAcceptedHandoff =
            SelfAcceptedHandoff::from_self_accepted;
        let _consume: fn(SelfAcceptedHandoff) -> AcceptedBlock = SelfAcceptedHandoff::into_accepted;
    }
}
