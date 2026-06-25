// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types
// - Explicit state transitions only
// - Canonical serialization for all persisted/hashed data

use ade_types::{BlockNo, Hash32, SlotNo};

/// Hard Fork Combinator schedule construction and query errors.
///
/// All variants are flat and value-typed; replay corpora compare reject
/// reasons byte-for-byte without traversing strings or trait objects.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HFCError {
    EmptyEraList,
    NonMonotonicEras {
        prev_start: SlotNo,
        next_start: SlotNo,
    },
    ZeroSlotLength {
        era_index: u8,
    },
    ZeroEpochLength {
        era_index: u8,
    },
    SlotBeforeSystemStart {
        slot: SlotNo,
        first_era_start: SlotNo,
    },
    SlotAfterLastEra {
        slot: SlotNo,
        last_era_end: SlotNo,
    },
}

/// Slot-to-time translation errors.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SlotTimeError {
    OutOfRange { slot: SlotNo },
    HFC(HFCError),
    Overflow,
}

/// Returned when a consensus query asks for a slot strictly past the
/// stable forecast horizon (`last_era.start_slot + last_era.safe_zone_slots`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OutsideForecastRange {
    pub requested: SlotNo,
    pub horizon: SlotNo,
}

/// Praos header validation rejection reasons. CLOSED — every variant is
/// structured flat data; no `String`, no `Box<dyn>`, no `#[non_exhaustive]`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HeaderValidationError {
    VrfCert(VrfCertError),
    OpCertCounter(OpCertCounterError),
    Nonce(NonceEvolutionError),
    SlotBeforeLastApplied { last: SlotNo, attempted: SlotNo },
    BlockNoOutOfOrder { last: BlockNo, attempted: BlockNo },
    BodyHashMismatch { expected: Hash32, actual: Hash32 },
    EraMismatch { schedule_era: u8, header_era: u8 },
    HFC(HFCError),
    OutsideForecastRange(OutsideForecastRange),
    /// A fixed-size header crypto field had the wrong length — fail-closed.
    MalformedField(FieldError),
    /// `blake2b_256(header.vrf_vkey)` did not match the pool's registered
    /// VRF keyhash. The header VRF key is not the one the snapshot bound.
    VrfKeyhashMismatch { expected: Hash32, actual: Hash32 },
    /// KES signature over the header body failed verification.
    KesInvalid,
    /// Operational-certificate cold-key signature failed verification.
    OpCertInvalid,
}

/// Wrong-length fixed-size header crypto field. Flat value data so reject
/// reasons compare byte-for-byte. Parallels
/// `ade_ledger::block_validity::verdict::FieldError` (ade_core cannot depend
/// on ade_ledger); ade_ledger converts at the composition boundary.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FieldError {
    pub field: FieldKind,
    pub expected: usize,
    pub actual: usize,
}

/// Closed set of fixed-size header fields whose length is checked fail-closed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FieldKind {
    VrfVkey,
    VrfProof,
    VrfOutput,
    KesVkey,
    KesSignature,
    OpCertSignature,
    IssuerVkey,
}

/// VRF certificate verification errors. `LeaderValueAboveThreshold`
/// carries the value and threshold as raw 8-byte big-endian fixed-point
/// scalars so the comparison and the reject reason are both byte-stable.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VrfCertError {
    MalformedKey,
    MalformedProof,
    VerificationFailed,
    LeaderValueAboveThreshold { value: [u8; 8], threshold: [u8; 8] },
}

/// Op-cert counter monotonicity errors.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OpCertCounterError {
    Regression { existing: u64, attempted: u64 },
}

/// Nonce evolution errors.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NonceEvolutionError {
    SlotBeforeLast { last: SlotNo, attempted: SlotNo },
    UninitialisedEpochNonce,
    /// The epoch-boundary combine was attempted with no
    /// `last_epoch_block_nonce` operand — a legacy `array(9)` chain-dep
    /// store or a pre-seed state. Fail closed; never fabricate the operand
    /// (DC-EPOCH-16).
    MissingLastEpochBlockNonce,
}

/// Leader-schedule query errors.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LeaderScheduleError {
    UnknownPool,
    OutsideForecastRange(OutsideForecastRange),
    HFC(HFCError),
}
