# S-B2 — `PraosChainDepState` canonical type + closed event/error taxonomies

## Slice Header

**Slice Name**: `PraosChainDepState` canonical type + closed event/error taxonomies
**Cluster**: PHASE4-N-B
**Status**: In Progress
**Cluster Exit Criteria Addressed**: none directly — substrate for S-B3..S-B10.

**Slice Dependencies**: S-B1 (EraSchedule + errors module).

---

## 3. Implementation Instruction (AI)

Implement exactly what is specified. Do not implement transitions —
this slice is types-only. Transitions land in S-B4 (nonce), S-B5
(op-cert), S-B7 (header-validate), S-B8 (fork-choice), S-B9
(rollback).

Commit with the `Co-Authored-By: Claude <model+context>
<noreply@anthropic.com>` trailer per CLAUDE.md.

---

## 4. Intent

Make it impossible for any later consensus slice to invent its own
chain-dep state shape, its own event taxonomy, or its own reject
reason. This slice locks in **closed enums** for every reject reason
the cluster will use, the **frozen shape** of `PraosChainDepState`,
and the **canonical encoding** that replay tests will compare bytes
against.

---

## 5. Scope

**Modules / crates**:
- `crates/ade_core/src/consensus/praos_state.rs` (NEW — `PraosChainDepState`, `OpCertCounterMap`, `Nonce`)
- `crates/ade_core/src/consensus/events.rs` (NEW — `ChainEvent`, `ChainSelectionReject`)
- `crates/ade_core/src/consensus/errors.rs` (EXTEND — add `HeaderValidationError`, `VrfCertError`, `OpCertCounterError`, `NonceEvolutionError`, `LeaderScheduleError`)
- `crates/ade_core/src/consensus/encoding.rs` (NEW — canonical CBOR encoding for `PraosChainDepState` and the closed-event enums)
- `crates/ade_core/src/consensus/mod.rs` (extend — re-exports)
- `crates/ade_core/tests/praos_state_canonical_roundtrip.rs` (NEW integration test)

**State machines affected**: none — types only. Transitions live in
later slices.

**Persistence impact**: `PraosChainDepState` will be persisted by
N-D and reloaded; canonical encoding is fixed here so replay can
byte-compare.

**Network-visible impact**: none.

**Out-of-scope**:
- Any transition function (`evolve_nonce`, `check_op_cert`, etc.)
- Wiring `PraosChainDepState` into ledger or chain-db
- `Point`, `ChainHash`, `ValidatedHeaderSummary`, `CandidateFragment`
  (S-B7/S-B8 introduce these as needed; this slice declares only
  the types referenced *by* events/state that don't yet exist —
  forward declarations are allowed via marker structs)

---

## 6. Execution Boundary

**BLUE**:
- `ade_core::consensus::praos_state`
- `ade_core::consensus::events`
- `ade_core::consensus::errors` (extension)
- `ade_core::consensus::encoding`

**GREEN**: none.
**RED**: none.

---

## 7. Invariants Preserved

- S-B1's `EraSchedule`, `HFCError`, `SlotTimeError`,
  `OutsideForecastRange` continue to work; this slice only adds.
- `ade_core` crate-level lints preserved.
- BLUE crate-only dependencies (`ade_types`, `ade_crypto`,
  `ade_codec`). No `ade_runtime`, no `tokio`, no `serde_json`.

---

## 8. Invariants Strengthened or Introduced

- **`DC-CONS-04` (NEW, type shape only — behavior lands in S-B4/S-B5/S-B7)**:
  `PraosChainDepState` shape is fixed; no later slice may add a
  field without explicit doctrine change.
- **`T-DET-01` strengthened**: canonical encoding for the
  consensus-state type is byte-identical across runs and across
  encoder/decoder symmetry.

---

## 9. Design Summary

### Nonce wrapper

```rust
// praos_state.rs

use ade_types::{Hash32, SlotNo};

/// A 32-byte Praos nonce. Distinct newtype so the type system stops
/// callers from mixing nonces and other Hash32 values.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Nonce(pub Hash32);

impl Nonce {
    pub const ZERO: Nonce = Nonce(Hash32([0u8; 32]));
    pub fn as_bytes(&self) -> &[u8; 32] { &self.0.0 }
}
```

### Op-cert counter map

```rust
// praos_state.rs

use std::collections::BTreeMap;
use ade_types::Hash28;

/// (pool_id, kes_period) → highest observed op-cert issue counter.
///
/// BTreeMap — never HashMap. Insertion / iteration order is
/// deterministic because consumers must replay the same state from
/// the same sequence of headers.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct OpCertCounterMap {
    counters: BTreeMap<(Hash28, u64), u64>,
}

impl OpCertCounterMap {
    pub fn new() -> Self { Self::default() }
    pub fn get(&self, pool: &Hash28, kes_period: u64) -> Option<u64>;
    pub fn upsert_strict(
        &mut self,
        pool: Hash28,
        kes_period: u64,
        counter: u64,
    ) -> Result<(), OpCertCounterError>;
    pub fn len(&self) -> usize;
    pub fn is_empty(&self) -> bool;
    pub fn iter(&self) -> impl Iterator<Item = (&(Hash28, u64), &u64)>;
}
```

`upsert_strict` rejects regression: if a counter ≤ the existing
counter for `(pool, kes_period)`, return `OpCertCounterError::
Regression { existing, attempted }`. Equal-counter is regression
(strictly increasing).

### PraosChainDepState

```rust
// praos_state.rs

use ade_types::{BlockNo, EpochNo, SlotNo};

/// The complete Praos chain-dep state owned by N-B consensus.
///
/// Five named nonce slots per Ouroboros-consensus PraosChainDepState:
/// evolving / candidate / epoch / previous_epoch / lab.
///
/// `last_epoch_block` tracks the block at the previous epoch boundary
/// (used for nonce candidate-to-epoch promotion).
///
/// `last_slot` tracks the most recent applied header slot.
/// `last_block_no` tracks the corresponding block number.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PraosChainDepState {
    pub evolving_nonce:       Nonce,
    pub candidate_nonce:      Nonce,
    pub epoch_nonce:          Nonce,
    pub previous_epoch_nonce: Nonce,
    pub lab_nonce:            Nonce,
    pub last_epoch_block:     Option<EpochNo>,
    pub last_slot:            Option<SlotNo>,
    pub last_block_no:        Option<BlockNo>,
    pub op_cert_counters:     OpCertCounterMap,
}

impl PraosChainDepState {
    /// Genesis state: all nonces are the shelley_genesis_hash
    /// (the well-known initial nonce derived from the Shelley
    /// genesis CBOR). Caller supplies it because computing it is
    /// genesis-parser business, not BLUE business.
    pub fn genesis(initial_nonce: Nonce) -> Self;

    /// Empty state (all nonces = ZERO, no counters). Used for tests
    /// and for the type-default. NOT a valid runtime state.
    pub fn empty() -> Self;
}
```

### ChainEvent / ChainSelectionReject

```rust
// events.rs

use ade_types::{BlockNo, Hash32, SlotNo};

/// Forward-declared point identifier. Refined in S-B7/S-B8.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Point {
    pub slot:  SlotNo,
    pub hash:  Hash32,
}

/// Forward-declared chain-tip identifier.
pub type ChainHash = Hash32;

/// Distance between two points expressed in blocks (not slots).
/// Rollback depth is measured in blocks per DC-CONS-05.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct BlockDistance(pub u64);

/// Security parameter k (block-count rollback bound).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct SecurityParam(pub u64);

/// Reasons fork-choice / rollback reject a candidate.
/// CLOSED — every variant is exhaustive; no `Other` or `String`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ChainSelectionReject {
    ForkBeforeImmutableTip {
        immutable_tip:          Point,
        candidate_intersection: Point,
        rollback_depth:         BlockDistance,
        security_param:         SecurityParam,
    },
    ExceededRollback {
        requested: BlockDistance,
        max:       SecurityParam,
    },
    HeaderInvalid {
        at_point: Point,
        reason:   HeaderValidationError,
    },
    TiebreakerLossKeepCurrent {
        current_tip:   Point,
        candidate_tip: Point,
    },
}

/// Output of the fork-choice / rollback transitions.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ChainEvent {
    ChainExtended { new_tip: Point, block_no: BlockNo },
    RolledBack    { to_point: Point, depth: BlockDistance },
    RolledForward { from: Point, to: Point },
    ChainSelected { new_tip: Point, replaced_tip: Option<Point> },
    Rejected      { reason: ChainSelectionReject },
}
```

### Header / VRF / nonce / op-cert / leader-schedule error enums

```rust
// errors.rs (extension)

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
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VrfCertError {
    MalformedKey,
    MalformedProof,
    VerificationFailed,
    LeaderValueAboveThreshold { value: [u8;8], threshold: [u8;8] },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OpCertCounterError {
    Regression { existing: u64, attempted: u64 },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NonceEvolutionError {
    SlotBeforeLast { last: SlotNo, attempted: SlotNo },
    UninitialisedEpochNonce,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LeaderScheduleError {
    UnknownPool,
    OutsideForecastRange(OutsideForecastRange),
    HFC(HFCError),
}
```

All variants `Clone + PartialEq + Eq`. No `String`. No
`Box<dyn Error>`. No `#[non_exhaustive]` — closed enums.

### Canonical encoding

`encoding.rs` provides byte-stable serialization for
`PraosChainDepState`, `ChainEvent`, and `ChainSelectionReject` using
`minicbor` (already a workspace dep). Definite-length CBOR arrays and
maps; no indefinite-length. Tag-free for state. Field order is
documented and fixed.

```rust
pub fn encode_chain_dep_state(s: &PraosChainDepState) -> Vec<u8>;
pub fn decode_chain_dep_state(bytes: &[u8]) -> Result<PraosChainDepState, DecodeError>;

pub fn encode_chain_event(e: &ChainEvent) -> Vec<u8>;
pub fn decode_chain_event(bytes: &[u8]) -> Result<ChainEvent, DecodeError>;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DecodeError {
    Cbor(&'static str),
    UnknownDiscriminant { for_enum: &'static str, found: u32 },
    FieldCountMismatch { expected: u32, actual: u32 },
    InvalidLength { field: &'static str, expected: usize, actual: usize },
}
```

CBOR layout (informative, will be re-verified by the round-trip
test):

- `PraosChainDepState` → CBOR array of length 9:
  `[evolving, candidate, epoch, prev_epoch, lab, last_epoch_block (uint or null), last_slot (uint or null), last_block_no (uint or null), op_cert_counters_array]`
- `OpCertCounterMap` → CBOR array of `[pool_hash28, kes_period_u64, counter_u64]` tuples, sorted by `(pool, kes_period)` ascending (BTreeMap iteration order).
- `Nonce` → CBOR bytes (32).
- `ChainEvent` → CBOR `[discriminant_u32, payload_array]`.
- `ChainSelectionReject` → CBOR `[discriminant_u32, payload_array]`.

The exact field order is fixed by the test
`praos_state_canonical_roundtrip::layout_is_stable` which encodes a
known-vector state and asserts the hex output.

---

## 10. Changes Introduced

### Types
- New: `Nonce`, `OpCertCounterMap`, `PraosChainDepState`,
  `BlockDistance`, `SecurityParam`, `Point`, `ChainHash`,
  `ChainSelectionReject`, `ChainEvent`, `HeaderValidationError`,
  `VrfCertError`, `OpCertCounterError`, `NonceEvolutionError`,
  `LeaderScheduleError`, `DecodeError`.

### State Transitions
- None.

### Persistence
- Canonical CBOR encoding for `PraosChainDepState` defined; N-D will
  use it in a later slice.

### Removal / Refactors
- None.

---

## 11. Replay, Crash, and Epoch Validation

### Tests

- `crates/ade_core/tests/praos_state_canonical_roundtrip.rs`:
  - `layout_is_stable` — encode a fixed test vector; assert the hex
    bytes match a hard-coded expected vector (the vector lives in the
    test file; if encoder bytes change, the test fails and the change
    must be documented as a deliberate format change).
  - `roundtrip_empty_state` — encode `PraosChainDepState::empty()`,
    decode, assert equality.
  - `roundtrip_genesis_state` — encode `PraosChainDepState::genesis(
    Nonce(some_hash))`, roundtrip.
  - `roundtrip_populated_state` — state with 3 nonce slots, 5 op-cert
    counters, last_slot/last_block_no set. Roundtrip + equality.
  - `roundtrip_chain_event_all_variants` — one of each `ChainEvent`
    variant. Roundtrip.
  - `roundtrip_chain_selection_reject_all_variants` — one of each
    `ChainSelectionReject` variant. Roundtrip.
  - `decode_rejects_unknown_discriminant` — feed CBOR with an unused
    discriminant; assert `DecodeError::UnknownDiscriminant`.
  - `decode_rejects_short_array` — feed a CBOR array of fewer than 9
    elements for the state; assert `FieldCountMismatch`.
  - `op_cert_counter_map_iteration_is_deterministic` — insert in
    random orders; iterate; assert sorted by `(pool, kes_period)`.

- Unit tests in `praos_state.rs`:
  - `op_cert_upsert_rejects_regression`
  - `op_cert_upsert_rejects_equal_counter`
  - `op_cert_upsert_accepts_strictly_increasing`
  - `genesis_state_is_well_formed`
  - `nonce_zero_constant_is_zero_bytes`

### Replay impact
- N-D persistence in a later cluster will read/write this encoding.
- `ChainEvent` stream is the canonical replay-comparison surface for
  CE-N-B-5 (S-B10).

---

## 12. Mechanical Acceptance Criteria

- [ ] `cargo build -p ade_core` PASS
- [ ] `cargo test -p ade_core --lib consensus::praos_state` PASS
- [ ] `cargo test -p ade_core --lib consensus::events` PASS (or
      `consensus::encoding` — wherever the unit tests live)
- [ ] `cargo test -p ade_core --test praos_state_canonical_roundtrip`
      PASS
- [ ] `cargo clippy -p ade_core --all-targets -- -D warnings` PASS
- [ ] No `HashMap` / `HashSet` in `praos_state.rs` / `events.rs` /
      `encoding.rs` (grep)
- [ ] No `String` in any of the new `*Error` enums (grep — values
      must be flat data, not formatted text)
- [ ] No `Box<dyn` anywhere in `consensus::`
- [ ] Layout test hex vector is committed (proves encoding is stable
      by *value*, not just by `roundtrip`)

---

## 13. Failure Modes

| Failure | Shape | Fail-fast? |
|---|---|---|
| Decode wrong discriminant | `DecodeError::UnknownDiscriminant` | yes |
| Decode wrong array length | `DecodeError::FieldCountMismatch` | yes |
| Decode invalid byte length | `DecodeError::InvalidLength` | yes |
| Decode invalid CBOR | `DecodeError::Cbor` | yes |
| Op-cert regression | `OpCertCounterError::Regression` | yes |

---

## 14. Hard Prohibitions

### Inherited (from cluster.md)
- BLUE receiving `&ChainDb`, `&Mux`, parsing genesis text
- Wall-clock reads in BLUE
- `HashMap` / `HashSet`
- Floating-point arithmetic
- TODO/placeholder error variants
- `async fn`, `.await`, `tokio` in BLUE

### Slice-specific
- No `String` in `*Error` enums.
- No `#[non_exhaustive]` — these enums are intentionally closed.
- No `serde` derives — encoding is hand-written via `minicbor`.
- No `Option<Vec<...>>` patterns where `Vec` already encodes
  emptiness; prefer `Vec`. Single exception: `Option<SlotNo>` /
  `Option<EpochNo>` / `Option<BlockNo>` for genesis-state nulls.
- No transition functions. This is a types-only slice.

---

## 15. Explicit Non-Goals

- Do NOT implement nonce evolution, op-cert checking, header
  validation, fork-choice, or rollback. Those are S-B4..S-B9.
- Do NOT wire `PraosChainDepState` into `ade_ledger` or `ade_node`.
- Do NOT define `ValidatedHeaderSummary` or `CandidateFragment`;
  those land in S-B7 / S-B8.
- Do NOT introduce a generic `ChainState<T>` parameterized over
  protocol; Praos is the only protocol N-B supports.

---

## 16. Completion Checklist

- [ ] All new state is canonically encoded (CBOR layout test passes)
- [ ] All failure modes are deterministic
- [ ] No TODOs in BLUE
- [ ] CI enforces the closed-enum rule (script
      `ci/ci_check_consensus_closed_enums.sh` ensures no
      `#[non_exhaustive]` and no `Other` / `Unknown` variants in
      `consensus::*` error enums)
- [ ] Replay-equivalence tests pass across runs

---

## 17. Review Notes

- The `Point` type is *forward-declared* here so events can
  reference it. S-B7 will refine it (or this declaration is the
  final one — re-check at S-B7 entry).
- `BlockDistance` is in blocks, not slots, because k = 2160 is in
  blocks (DC-CONS-05). Be careful in fork-choice to never mix.

---

## 18. Authority Reminder

Correctness rules live in `docs/ade-invariant-registry.toml`. If
this doc conflicts with the registry, the registry wins.
