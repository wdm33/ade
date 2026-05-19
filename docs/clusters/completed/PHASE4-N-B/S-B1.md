# S-B1 — `EraSchedule` canonical authority + slot/era/time translation

## Slice Header

**Slice Name**: `EraSchedule` canonical authority + slot/era/time translation
**Cluster**: PHASE4-N-B
**Status**: In Progress
**Cluster Exit Criteria Addressed**:
- [x] **CE-N-B-3** — `EraSchedule` matches oracle hard-fork slots
  exactly; slot↔era↔epoch translation and slot→time pure and
  replay-equivalent; anchored to `BootstrapAnchorHash`.

**Slice Dependencies**: none.

---

## 3. Implementation Instruction (AI)

Implement exactly what is specified. Do not invent behavior. The
`MAC` in §12 defines the only way this slice is complete.

Commits MUST follow this repo's local override (`CLAUDE.md` →
`Co-Authored-By: Claude <model+context> <noreply@anthropic.com>`).

---

## 4. Intent

Make it impossible for any BLUE consensus path to ask "what era is
slot S in?" or "what UTC instant is slot S?" without consulting a
typed, hash-anchored `EraSchedule`. No magic constants. No floats.
No wall-clock reads. Out-of-horizon queries fail-fast with a
structured `OutsideForecastRange`.

---

## 5. Scope

**Modules / crates**:
- `crates/ade_core/src/consensus/mod.rs` (new module tree)
- `crates/ade_core/src/consensus/era_schedule.rs` (BLUE — types +
  translation)
- `crates/ade_core/src/consensus/errors.rs` (BLUE — `SlotTimeError`,
  `HFCError`, `OutsideForecastRange`)
- `crates/ade_runtime/src/consensus/mod.rs` (new module)
- `crates/ade_runtime/src/consensus/genesis_parser.rs` (RED — parses
  Byron + Shelley + Alonzo + Conway genesis JSON, computes
  `BootstrapAnchorHash`, materializes `EraSchedule`)
- `crates/ade_testkit/src/consensus/mod.rs` (new module — GREEN corpus
  harness)
- `crates/ade_testkit/tests/hfc_schedule_corpus.rs` (NEW integration
  test — closes CE-N-B-3 via `cargo test -p ade_core --test
  hfc_schedule_corpus` style invocation; place the integration test
  under `ade_core/tests/` so the CE-name in cluster.md is exact)
- `crates/ade_core/tests/hfc_schedule_corpus.rs` (NEW integration test
  closing CE-N-B-3)
- `corpus/consensus/hfc_schedule/mainnet.json` (NEW corpus fixture —
  ground truth oracle data, see §11)
- `corpus/consensus/hfc_schedule/preprod.json` (NEW corpus fixture)
- `.idd-config.json` — add `crates/ade_core/src/consensus/` to
  `core_paths` (already covered by BLUE crate scoping, but add an
  explicit doc note)

**State machines affected**: none — this slice is pure types +
translation.

**Persistence impact**: none.

**Network-visible impact**: none.

**Out-of-scope**:
- `PraosChainDepState` (S-B2)
- Any VRF / nonce / op-cert / fork-choice / rollback logic
- Live `cardano-node` interop wiring (S-B10)
- Time-conversion *with* the wall clock (forbidden in BLUE)

---

## 6. Execution Boundary

**BLUE (deterministic, authoritative)**:
- `ade_core::consensus::era_schedule`
- `ade_core::consensus::errors`

**GREEN (deterministic glue, non-authoritative)**:
- `ade_testkit::consensus` corpus harness

**RED (nondeterministic shell)**:
- `ade_runtime::consensus::genesis_parser` — reads files, parses JSON,
  computes the `BootstrapAnchorHash` and materializes the immutable
  `EraSchedule` once. After construction the schedule is BLUE-consumed
  by-value.

Rules:
- No RED behavior in BLUE code.
- GREEN code does not affect authoritative outputs (used in tests
  only).

---

## 7. Invariants Preserved

- All existing ledger replay tests still pass (`ade_ledger::hfc`
  era-translation tests are unaffected — different domain).
- `ade_core` crate-level lints (`deny(clippy::float_arithmetic)`,
  `deny(unsafe_code)`, etc.) preserved.
- No new dependency on `ade_runtime` from `ade_core` (BLUE inward).

---

## 8. Invariants Strengthened or Introduced

NEW registry rules introduced (will be appended to
`docs/ade-invariant-registry.toml` at cluster close):
- **`DC-CONS-07`** — `EraSchedule` is a typed BLUE-consumed value
  anchored to `BootstrapAnchorHash`.
- **`DC-CONS-08`** — slot→time is pure of wall clock.
- **`DC-CONS-09`** — forecast horizon stops at the safe zone;
  out-of-range = `OutsideForecastRange`.

Existing rules strengthened:
- **`DC-EPOCH-02`** — HFC schedule consulted only through typed value.

---

## 9. Design Summary

### Canonical types

```rust
// ade_core::consensus::era_schedule

use ade_types::{CardanoEra, EpochNo, Hash32, SlotNo};

/// 32-byte anchor binding an EraSchedule to a particular genesis
/// configuration. Computed by ade_runtime::consensus::genesis_parser
/// as Blake2b-256 of the concatenated canonical-CBOR encodings of the
/// four genesis blobs (byron, shelley, alonzo, conway).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct BootstrapAnchorHash(pub Hash32);

/// One era's parameters within the HFC schedule.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EraSummary {
    pub era: CardanoEra,
    pub start_slot: SlotNo,
    pub start_epoch: EpochNo,
    /// Slot length in milliseconds. Byron = 20_000; Shelley+ = 1_000
    /// on mainnet. Captured per era to keep slot→time pure.
    pub slot_length_ms: u32,
    /// Epoch length in slots. Byron = 21_600; Shelley+ = 432_000 on
    /// mainnet.
    pub epoch_length_slots: u32,
    /// Safe-zone in slots past `start_slot` within which forecast is
    /// stable. Zero means "no forecast latitude beyond era end".
    pub safe_zone_slots: u32,
}

/// Typed BLUE-consumed HFC schedule.
///
/// Constructed once by ade_runtime genesis parser; never mutated.
/// Era ordering by `start_slot` strictly increasing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EraSchedule {
    pub anchor: BootstrapAnchorHash,
    pub system_start_unix_ms: u64,
    eras: Vec<EraSummary>,        // ordered ascending by start_slot
}

impl EraSchedule {
    pub fn new(
        anchor: BootstrapAnchorHash,
        system_start_unix_ms: u64,
        eras: Vec<EraSummary>,
    ) -> Result<Self, HFCError>;

    pub fn anchor(&self) -> BootstrapAnchorHash;
    pub fn eras(&self) -> &[EraSummary];

    /// Pure translation: which era / epoch / relative-slot is `slot`?
    pub fn locate(&self, slot: SlotNo) -> Result<EraLocation, HFCError>;

    /// Slot → UTC instant in milliseconds since unix epoch.
    pub fn slot_to_time_ms(
        &self,
        slot: SlotNo,
    ) -> Result<u64, SlotTimeError>;

    /// Returns OutsideForecastRange if slot is beyond
    /// `last_era.start_slot + last_era.safe_zone_slots`.
    pub fn check_forecast_horizon(
        &self,
        slot: SlotNo,
    ) -> Result<(), OutsideForecastRange>;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EraLocation {
    pub era_index: u8,
    pub era: CardanoEra,
    pub epoch: EpochNo,
    pub relative_slot_in_epoch: u32,
}
```

### Closed error taxonomies

```rust
// ade_core::consensus::errors

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HFCError {
    EmptyEraList,
    NonMonotonicEras { prev_start: SlotNo, next_start: SlotNo },
    ZeroSlotLength { era_index: u8 },
    ZeroEpochLength { era_index: u8 },
    SlotBeforeSystemStart { slot: SlotNo, first_era_start: SlotNo },
    SlotAfterLastEra { slot: SlotNo, last_era_end: SlotNo },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SlotTimeError {
    OutOfRange { slot: SlotNo },
    HFC(HFCError),
    Overflow,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OutsideForecastRange {
    pub requested: SlotNo,
    pub horizon: SlotNo,
}
```

All variants are flat (no `String`, no formatted text, no `Box<dyn
Error>`). They are `Clone + Eq` so corpus tests can compare reject
reasons byte-for-byte.

### Pure translation algorithm

`locate(slot)`:
1. Linear scan `eras` (vec; small ≤ 7 entries). For each pair
   `(curr, next)`, if `curr.start_slot ≤ slot < next.start_slot`,
   `slot` is in `curr`. Last era extends to `u64::MAX`.
2. Compute `slots_into_era = slot - curr.start_slot`.
3. `era_epoch_offset = slots_into_era / curr.epoch_length_slots`.
4. `epoch = curr.start_epoch + era_epoch_offset`.
5. `relative_slot_in_epoch = slots_into_era % curr.epoch_length_slots`
   (cast to u32 — fits because epoch_length_slots is u32).
6. Errors: `EmptyEraList`, `SlotBeforeSystemStart`.

`slot_to_time_ms(slot)`:
1. `locate(slot)?` → `EraLocation`.
2. Walk eras up to `era_index`. For each prior era, accumulate
   `prior_era.epoch_length_slots × prior_era.slot_length_ms ×
   slot_count_in_era`. (Actually simpler: for prior eras,
   `(next.start_slot - prior.start_slot) × prior.slot_length_ms`.)
3. For current era: `(slot - curr.start_slot) × curr.slot_length_ms`.
4. Add `system_start_unix_ms`. Use `u64::checked_mul` /
   `checked_add`; on overflow return `SlotTimeError::Overflow`.

`check_forecast_horizon(slot)`:
1. `last_era.start_slot.checked_add(last_era.safe_zone_slots as u64)`
   → horizon.
2. If `slot > horizon`, return `OutsideForecastRange { requested:
   slot, horizon }`.

All arithmetic uses `checked_*` integer operations; `clippy::
float_arithmetic` already denied at crate level — no
`f64`/`f32` anywhere.

### RED genesis parser (sketch)

```rust
// ade_runtime::consensus::genesis_parser

pub struct GenesisBundle<'a> {
    pub byron_json: &'a [u8],
    pub shelley_json: &'a [u8],
    pub alonzo_json: &'a [u8],
    pub conway_json: &'a [u8],
}

pub fn parse_genesis(
    bundle: GenesisBundle<'_>,
    network: NetworkMagic,
) -> Result<EraSchedule, GenesisParseError>;

pub fn compute_anchor_hash(
    bundle: GenesisBundle<'_>,
) -> BootstrapAnchorHash;
```

`compute_anchor_hash` = Blake2b-256 of `b"ade_bootstrap_v1" ‖
canonical_cbor(byron) ‖ canonical_cbor(shelley) ‖
canonical_cbor(alonzo) ‖ canonical_cbor(conway)`.

The parser:
- Reads `byron.protocolConsts.k` and `byron.startTime` to seed
  `system_start_unix_ms` (Byron startTime is unix seconds).
- Reads `byron.blockVersionData.slotDuration` for Byron slot length
  (default 20000 ms).
- Reads `shelley.epochLength`, `shelley.slotLength`, `shelley.
  systemStart`, `shelley.activeSlotsCoeff`, `shelley.securityParam`.
- Builds `EraSummary` per era using mainnet boundary slots
  (parameterised by network — mainnet, preprod, preview each have
  their own table baked into the corpus, not into BLUE code).
- `safe_zone_slots = 3 × k × (1 / activeSlotsCoeff)` rounded up using
  integer math (Praos safe-zone formula); since `activeSlotsCoeff` is
  given as a rational `numerator/denominator` in genesis (e.g.
  1/20 = 0.05), the math stays integer: `safe = ceil(3·k·denom /
  numer)`.

`GenesisParseError` is structured (no `String`):

```rust
pub enum GenesisParseError {
    MalformedJson { which: GenesisBlob },
    MissingField { which: GenesisBlob, field: &'static str },
    InvalidValue { which: GenesisBlob, field: &'static str },
    Hfc(HFCError),
}
pub enum GenesisBlob { Byron, Shelley, Alonzo, Conway }
```

---

## 10. Changes Introduced

### Types
- New: `BootstrapAnchorHash`, `EraSummary`, `EraSchedule`,
  `EraLocation`, `HFCError`, `SlotTimeError`, `OutsideForecastRange`,
  `GenesisBundle<'_>`, `GenesisParseError`, `GenesisBlob`,
  `NetworkMagic` (u32 wrapper if not already present in `ade_types`;
  verify; reuse if exists, otherwise add to `ade_types::primitives`).

### State Transitions
- None (this slice is pure types + translation).

### Persistence
- None.

### Removal / Refactors
- None.

---

## 11. Replay, Crash, and Epoch Validation

### Corpus

`corpus/consensus/hfc_schedule/mainnet.json` — JSON fixture (NOT CBOR
because human-readable corpus is acceptable here; the *canonical*
test is that the parser produces a bit-identical `EraSchedule` from
JSON):

```jsonc
{
  "network": "mainnet",
  "system_start_unix_ms": 1506203091000,
  "expected_anchor_hex": "<blake2b-256 over canonical-CBOR of the four genesis blobs>",
  "expected_eras": [
    { "era": "ByronRegular", "start_slot": 0,           "start_epoch": 0,   "slot_length_ms": 20000, "epoch_length_slots": 21600,  "safe_zone_slots": 129600 },
    { "era": "Shelley",      "start_slot": 4492800,     "start_epoch": 208, "slot_length_ms": 1000,  "epoch_length_slots": 432000, "safe_zone_slots": 129600 },
    { "era": "Allegra",      "start_slot": 16588800,    "start_epoch": 236, "slot_length_ms": 1000,  "epoch_length_slots": 432000, "safe_zone_slots": 129600 },
    { "era": "Mary",         "start_slot": 23068800,    "start_epoch": 251, "slot_length_ms": 1000,  "epoch_length_slots": 432000, "safe_zone_slots": 129600 },
    { "era": "Alonzo",       "start_slot": 39916800,    "start_epoch": 290, "slot_length_ms": 1000,  "epoch_length_slots": 432000, "safe_zone_slots": 129600 },
    { "era": "Babbage",      "start_slot": 72316796,    "start_epoch": 365, "slot_length_ms": 1000,  "epoch_length_slots": 432000, "safe_zone_slots": 129600 },
    { "era": "Conway",       "start_slot": 133660800,   "start_epoch": 507, "slot_length_ms": 1000,  "epoch_length_slots": 432000, "safe_zone_slots": 129600 }
  ],
  "probe_points": [
    { "slot": 0,           "era": "ByronRegular", "epoch": 0,   "relative": 0,      "time_ms": 1506203091000 },
    { "slot": 4492800,     "era": "Shelley",      "epoch": 208, "relative": 0,      "time_ms": 1596059091000 },
    { "slot": 16588800,    "era": "Allegra",      "epoch": 236, "relative": 0,      "time_ms": 1608155091000 },
    { "slot": 39916800,    "era": "Alonzo",       "epoch": 290, "relative": 0,      "time_ms": 1631483091000 },
    { "slot": 72316796,    "era": "Babbage",      "epoch": 365, "relative": 0,      "time_ms": 1663883087000 },
    { "slot": 133660800,   "era": "Conway",       "epoch": 507, "relative": 0,      "time_ms": 1725227091000 }
  ],
  "horizon_probe": { "slot": 999999999999, "expected_horizon_error": true }
}
```

> NOTE on `expected_anchor_hex`: the implementer must compute this by
> running `compute_anchor_hash` against the actual mainnet genesis
> blobs (vendor them under `corpus/consensus/hfc_schedule/genesis/`
> from publicly-known checksums in the cardano-node 10.6.2 release)
> and pasting the result. If the genesis blobs are too large to
> commit, use *byron-stripped* canonical CBOR digests pre-computed and
> store the digests instead — but the anchor must still be
> reproducible from inputs the test owns.

`corpus/consensus/hfc_schedule/preprod.json` — analogous for preprod
network (boundary slots differ; parser must produce a different
schedule for a different `NetworkMagic`).

### Tests

- `crates/ade_core/tests/hfc_schedule_corpus.rs` (integration test)
  - For each corpus file: load `expected_eras`; build an `EraSchedule`
    directly from those values (skipping the RED parser); assert that
    each probe point's `locate()` + `slot_to_time_ms()` answer
    matches.
  - `horizon_probe.slot` → assert `check_forecast_horizon` returns
    `OutsideForecastRange` with the expected horizon.
- `crates/ade_runtime/tests/genesis_parser_corpus.rs` (RED integration
  test)
  - Load the genesis blobs from `corpus/consensus/hfc_schedule/
    genesis/` (or the digest-only files if blobs are too large);
    parse; assert the produced `EraSchedule.anchor() ==
    expected_anchor_hex` and `produced.eras() == expected_eras`.
- Unit tests in `era_schedule.rs`:
  - `locate_first_slot_of_each_era`
  - `locate_last_slot_of_each_era`
  - `locate_before_system_start_errors`
  - `slot_to_time_monotone_increasing` (property: for any two slots
    s1 < s2, `slot_to_time_ms(s1) < slot_to_time_ms(s2)`)
  - `slot_to_time_overflow_returns_structured_error`
  - `forecast_horizon_boundary`
  - `eraschedule_constructor_rejects_non_monotonic`
  - `eraschedule_constructor_rejects_empty`
  - `bootstrap_anchor_hash_distinguishes_genesis_variants` (two
    different inputs produce different anchors)

### Crash/restart behavior
- `EraSchedule` is constructed once at startup; if construction
  fails, the node refuses to start. No partial state.

### Epoch boundary behavior
- `locate()` and `slot_to_time_ms()` are pure functions; epoch
  boundaries are mechanical results of integer division, not events.

---

## 12. Mechanical Acceptance Criteria

- [ ] `cargo build -p ade_core -p ade_runtime -p ade_testkit` PASS
- [ ] `cargo test -p ade_core --lib consensus::era_schedule` PASS
- [ ] `cargo test -p ade_core --test hfc_schedule_corpus` PASS
- [ ] `cargo test -p ade_runtime --test genesis_parser_corpus` PASS
- [ ] `cargo clippy --all-targets -- -D warnings` PASS
- [ ] No `f64` / `f32` in `ade_core::consensus::era_schedule` (grep:
      `grep -RnE '\bf(32|64)\b' crates/ade_core/src/consensus/` returns
      no hits)
- [ ] No `std::time` / `Instant` / `SystemTime` in
      `ade_core::consensus::era_schedule`
- [ ] No `HashMap` / `HashSet` in `ade_core::consensus::era_schedule`
- [ ] `EraSchedule::new` rejects empty era list, non-monotonic eras,
      zero slot length, zero epoch length
- [ ] `OutsideForecastRange` returned for slots beyond
      `last_era.start_slot + safe_zone_slots`
- [ ] Determinism: `slot_to_time_ms(s)` produces the same answer
      across two consecutive runs (asserted by a `for _ in 0..2`
      loop over probe points)
- [ ] `BootstrapAnchorHash` derivation is deterministic (same genesis
      blobs → same hash, asserted in tests)

---

## 13. Failure Modes

| Failure | Shape | Fail-fast? | Replay impact |
|---|---|---|---|
| Empty era list | `HFCError::EmptyEraList` | yes (constructor) | none — node refuses to start |
| Non-monotonic eras | `HFCError::NonMonotonicEras { prev_start, next_start }` | yes | none |
| Zero slot/epoch length | `HFCError::ZeroSlotLength` / `ZeroEpochLength` | yes | none |
| Slot before system start | `HFCError::SlotBeforeSystemStart` | yes (per query) | replay-equivalent |
| Slot past last era end | `HFCError::SlotAfterLastEra` | yes | replay-equivalent |
| Slot→time overflow | `SlotTimeError::Overflow` | yes | replay-equivalent (deterministic) |
| Slot beyond forecast | `OutsideForecastRange { requested, horizon }` | yes | replay-equivalent |
| Malformed genesis JSON | `GenesisParseError::MalformedJson` (RED) | yes (startup) | none (BLUE never sees) |
| Genesis missing field | `GenesisParseError::MissingField` (RED) | yes (startup) | none |
| Genesis anchor mismatch | RED parser surfaces; caller refuses startup | yes | none |

All BLUE failure variants are `Clone + Eq` and contain no `String`
or `Box<dyn Error>`.

---

## 14. Hard Prohibitions

### Inherited (from `cluster.md` "Forbidden")
- BLUE receiving `&ChainDb`, `&Mux`, or parsing genesis text
- Wall-clock reads in BLUE
- `HashMap` / `HashSet` on authority paths
- Floating-point arithmetic
- TODO/placeholder error variants
- `async fn`, `.await`, `tokio` in BLUE

### Slice-specific
- No use of `chrono` / `time` crate in BLUE — `system_start_unix_ms`
  is a `u64` and all arithmetic is integer ms.
- No `String` or `Box<dyn Error>` in any `*Error` enum.
- No global mutable state — `EraSchedule` is owned, immutable,
  passed by `&self`.
- No `serde_json` or any JSON-handling crate in `ade_core` —
  JSON-handling lives in `ade_runtime::consensus::genesis_parser`.
- Mainnet boundary slots MUST come from the corpus fixture, NOT
  hard-coded into BLUE source.
- No `unwrap` / `expect` / `panic` in BLUE (already denied at crate
  level — preserve).
- `safe_zone_slots` MUST be derived from `(k, activeSlotsCoeff)` by
  the RED parser using integer math; do not hard-code 129600 in BLUE.

---

## 15. Explicit Non-Goals

- Do NOT implement `PraosChainDepState`, VRF, nonce, op-cert,
  fork-choice, rollback, or header-validate. Those are S-B2..S-B9.
- Do NOT add live-cardano-node wiring. That's S-B10.
- Do NOT introduce a generic "time service" trait. Slot→time is a
  pure function on `EraSchedule`, period.
- Do NOT optimize for parser performance. Genesis is read once.
- Do NOT add feature flags or `cfg(...)` semantic switches —
  `ci_check_no_semantic_cfg.sh` already enforces this.
- Do NOT introduce floating-point representations of
  `activeSlotsCoeff`; keep `(numer, denom)` integer pair.

---

## 16. Completion Checklist

- [ ] All new state is replay-derivable (no implicit state — schedule
      is the only state and it's value-typed)
- [ ] All new data is canonically encoded (the only `Hash32` derived
      from inputs is `BootstrapAnchorHash`, computed deterministically)
- [ ] All failure modes are deterministic (structured enums, no
      strings)
- [ ] No TODOs or placeholders in BLUE
- [ ] CI enforces the invariant strengthened (new check
      `ci/ci_check_no_float_in_consensus.sh` grep-asserts no float in
      `ade_core::consensus`; new check
      `ci/ci_check_no_chaindb_in_consensus_blue.sh` grep-asserts no
      `ChainDb` / `chain_db` symbol in BLUE consensus)
- [ ] Replay-equivalence tests pass across runs

---

## 17. Review Notes

- Open `a-residual` from invariants sketch §7 belongs to S-B8, not
  here. Genesis boundary slots are reproducible from oracle-pinned
  data; ouroboros-consensus revision pinning is a fork-choice
  concern.
- The `safe_zone_slots` formula is `3k / activeSlotsCoeff` rounded
  up. With `k = 2160` and `activeSlotsCoeff = 1/20`, this yields
  `3·2160·20 = 129600` slots. The corpus encodes this as a numeric
  value; BLUE code derives nothing from it.

---

## 18. Authority Reminder

Correctness rules live in `docs/ade-invariant-registry.toml` and the
cluster doc. If this slice doc conflicts with them, the registry
wins.
