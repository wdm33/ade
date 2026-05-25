# Invariant Slice — PHASE4-N-C S2

## Slice Header
**Slice Name:** BLUE `opcert_validate` + counter monotonicity + closed-grammar opcert encoder authority
**Cluster:** PHASE4-N-C
**Status:** Proposed
**CEs addressed:** CE-N-C-2 (opcert validation + counter monotonicity + closed encoder)
**Registry flips on merge:** `DC-CONS-11`, `DC-CONS-12` → `enforced`
**Dependencies:** S1 merged (HEAD ≥ `ea9770e`). S1 ships nothing this slice consumes; the dependency is purely sequential per the cluster slice plan.

---

## Intent

Make BLUE the single authority for opcert validation and serialisation:

- `ade_core::consensus::opcert_validate(opcert, cold_vk, expected_period, prev_counter)` is the only sanctioned producer-side opcert acceptance path. It rejects counter regression, counter repetition, period mismatch, and cold-signature failure with closed structured errors.
- `ade_codec::shelley::opcert::{encode_opcert, decode_opcert}` is the single closed-grammar opcert encoder/decoder, lifted out of `ade_codec::shelley::block`'s inline header path. The header encoder now delegates to this single authority.
- A CI gate forbids parallel opcert encoders / decoders anywhere outside the canonical authority.

The producer (S3 forge) will consume `opcert_validate` to reject malformed opcerts at the `ProducerTick → forge_block` boundary; this slice ships the BLUE primitive, not the consumer.

---

## The change (atomic; compile green as one unit)

### 1. New module `crates/ade_codec/src/shelley/opcert.rs` (BLUE)

Lifts opcert serialisation out of `ade_codec::shelley::block` (lines 167–195, 269–272, 279–282) into a standalone closed authority.

```rust
//! Closed-grammar OpCert encoder/decoder — the single producer-side opcert
//! byte authority. Both header CBOR (`shelley::block`) and standalone opcert
//! CBOR (S2 fixture parity) delegate to this module.

use crate::cbor::{read_bytes, read_uint, write_bytes_canonical, write_uint_canonical};
use ade_types::shelley::block::OperationalCert;

/// Standalone opcert CBOR shape — the 4-element array that
/// `cardano-cli node issue-op-cert` writes as cborHex:
///   [ hot_vkey: bstr, sequence_number: uint, kes_period: uint, sigma: bstr ]
/// `sigma` is the cold-key Ed25519 signature over the canonical signable
/// (hot_vkey || seq# BE || kes_period BE) — the same format `verify_opcert`
/// in `ade_crypto::kes` consumes.
pub fn encode_opcert(opcert: &OperationalCert) -> Vec<u8> { /* ... */ }

pub fn decode_opcert(bytes: &[u8]) -> Result<OperationalCert, OpCertCodecError> { /* ... */ }

/// Header-embedded opcert fields path — used by `shelley::block::encode_header`.
/// Differs from `encode_opcert` only in that it does NOT emit the surrounding
/// 4-element-array CBOR header; the fields are written inline into the
/// caller's CBOR stream. This is the bytes-delegated path; the canonical 4-tuple
/// header is computed by emitting `0x84` then calling
/// `write_opcert_fields_into`.
pub fn write_opcert_fields_into(buf: &mut Vec<u8>, opcert: &OperationalCert);

pub fn read_opcert_fields_from(
    data: &[u8],
    offset: &mut usize,
) -> Result<OperationalCert, OpCertCodecError>;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OpCertCodecError {
    BadArrayHeader { expected: u8, found: u8 },
    BadFieldType { field: &'static str, detail: &'static str },
    WrongHotVkeyLength { found: usize, expected: usize },
    WrongSigmaLength { found: usize, expected: usize },
    SequenceNumberOverflow,
    KesPeriodOverflow,
    TrailingBytes { remaining: usize },
}
```

The existing `shelley::block` header encode/decode is refactored to call
`write_opcert_fields_into` / `read_opcert_fields_from`. The byte output of the
header CBOR is unchanged — verified by every existing `roundtrip_*` and
`*_determinism` test in `ade_testkit`.

### 2. New module `crates/ade_core/src/consensus/opcert_validate.rs` (BLUE)

```rust
use ade_crypto::kes::{
    Ed25519VerificationKey, KesVerificationKey, OperationalCertData, verify_opcert,
};
use ade_types::shelley::block::OperationalCert;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OpCertError {
    /// Cold-key signature over the signable did not verify under cold_vk.
    BadColdSignature,
    /// opcert.kes_period != expected_period at this slot/anchor.
    PeriodMismatch { found: u64, expected: u64 },
    /// opcert.sequence_number == prev_counter (repeat).
    CounterRepeat { counter: u64 },
    /// opcert.sequence_number < prev_counter (regression).
    CounterRegression { found: u64, prev: u64 },
    /// hot_vkey length not 32 bytes (closed-grammar shape failure).
    BadHotVkeyLength { found: usize },
    /// sigma length not 64 bytes (closed-grammar shape failure).
    BadSigmaLength { found: usize },
    /// hot_vkey bytes failed Ed25519 point validity (defense-in-depth).
    MalformedColdVk,
}

/// Validate a producer-supplied opcert at the RED -> BLUE boundary.
///
/// - `opcert` is the producer-supplied `OperationalCert` (header-shaped).
/// - `cold_vk` is the operator's cold verification key.
/// - `expected_period` is the KES period the producer expects to forge at,
///   typically `(slot - anchor) / slots_per_kes_period`.
/// - `prev_counter`, if Some, is the durable per-(cold-key, node) record of
///   the last accepted opcert counter. None permitted only for the first
///   opcert this node has ever seen.
///
/// Forbidden states are all enumerated in `OpCertError`. Every rejection is
/// deterministic; success returns Ok(()).
pub fn opcert_validate(
    opcert: &OperationalCert,
    cold_vk: &Ed25519VerificationKey,
    expected_period: u64,
    prev_counter: Option<u64>,
) -> Result<(), OpCertError> { /* ... */ }
```

The implementation is straightforward: shape-checks first
(`hot_vkey.len() == 32`, `sigma.len() == 64`), then cold-signature verification
via the existing `verify_opcert` in `ade_crypto::kes` (which builds the canonical
signable and runs Ed25519 verify), then period match, then counter discipline.
Errors are total — every forbidden state has a dedicated variant.

### 3. Re-exports

In `crates/ade_codec/src/shelley/mod.rs`:
```rust
pub mod opcert;
```

In `crates/ade_core/src/consensus/mod.rs`:
```rust
pub mod opcert_validate;
pub use opcert_validate::{opcert_validate, OpCertError};
```

### 4. Unit tests

**In `crates/ade_codec/src/shelley/opcert.rs` `#[cfg(test)] mod tests` (BLUE):**

- `opcert_encoder_matches_cardano_cli_byte_identical` — pins one synthetic
  but canonical fixture: `hot_vkey = [0x01; 32]`, `sequence_number = 7`,
  `kes_period = 42`, `sigma = [0x02; 64]`. Expected cborHex hand-computed
  from the cardano-api `OperationalCertificate` CBOR schema (4-tuple,
  bstr/uint/uint/bstr). Test asserts `encode_opcert(&fixture) ==
  hex::decode(EXPECTED).unwrap()`.
- `opcert_round_trip_byte_identical` — `decode_opcert(encode_opcert(&x))` is
  byte-equal to `x` for the canonical fixture set (5 fixtures: minimal,
  large seq#, max u64 seq#, period 0, period far-future).
- `opcert_decode_rejects_trailing_garbage` — `decode_opcert` over a valid
  prefix followed by a single trailing byte returns
  `OpCertCodecError::TrailingBytes { remaining: 1 }`.
- `opcert_decode_rejects_truncated` — truncated by 1 byte returns a
  shape-error variant (not a panic, not an `assert!` trip).
- `opcert_decode_rejects_wrong_array_header` — input starting `0x83`
  (3-array) returns `BadArrayHeader { expected: 0x84, found: 0x83 }`.
- `opcert_decode_rejects_short_hot_vkey` — `hot_vkey` field length 31
  returns `WrongHotVkeyLength { found: 31, expected: 32 }`.
- `opcert_decode_rejects_short_sigma` — `sigma` field length 63 returns
  `WrongSigmaLength { found: 63, expected: 64 }`.
- `header_encoder_uses_opcert_fields_path` — for a Shelley/Babbage/Conway
  header fixture, the header CBOR is byte-identical when produced via the
  refactored path (delegating to `write_opcert_fields_into`) versus a
  golden checked-in pre-refactor byte string. Pins the no-regression
  property for every existing `roundtrip_*` test.

**In `crates/ade_core/src/consensus/opcert_validate.rs` `#[cfg(test)] mod tests` (BLUE):**

- `opcert_validate_accepts_canonical_fixture` — using a deterministically
  generated cold keypair (seed `[0x42; 32]`) and a deterministically
  generated KES verification key (Sum6KES seed `[0x43; 32]`), construct a
  canonical opcert via the dev signing path (uses `ade_runtime::producer::signing`
  in `#[cfg(test)]` only; this is the GREEN test-harness pattern). Assert
  `opcert_validate(&opcert, &cold_vk, 42, None) == Ok(())`.
- `opcert_validate_rejects_counter_regression` — same opcert with
  `prev_counter = Some(8)`, `opcert.sequence_number = 7` → returns
  `OpCertError::CounterRegression { found: 7, prev: 8 }`.
- `opcert_validate_rejects_counter_repeat` — `prev_counter = Some(7)`,
  `opcert.sequence_number = 7` → returns `OpCertError::CounterRepeat
  { counter: 7 }`.
- `opcert_validate_rejects_period_mismatch` — opcert stamped at period 42,
  `expected_period = 43` → returns `OpCertError::PeriodMismatch
  { found: 42, expected: 43 }`.
- `opcert_validate_rejects_bad_signature_over_cold_key` — flip a single
  byte of `opcert.sigma` → returns `OpCertError::BadColdSignature`.
- `opcert_validate_rejects_short_hot_vkey` — opcert with
  `hot_vkey.len() == 31` → returns `OpCertError::BadHotVkeyLength
  { found: 31 }` (shape-check before signature check).
- `opcert_validate_first_opcert_no_prev_counter` — `prev_counter = None`
  with any non-zero `sequence_number` → returns `Ok(())` (operator's
  initial opcert may carry any starting counter).

### 5. New CI gate `ci/ci_check_opcert_closed.sh` (closure proof)

Mechanical guards:

1. **No parallel opcert encoders.** Grep across all BLUE crates for
   `pub fn .*opcert.*encode\|pub fn encode.*opcert\|pub fn .*write_opcert`
   — the only sanctioned definitions are in
   `crates/ade_codec/src/shelley/opcert.rs`. Header path's
   `encode_header` delegates via `write_opcert_fields_into`; the gate
   verifies the header file does not contain its own bstr/uint emit
   sequence for opcert fields (greps `cbor::write_bytes_canonical.*hot_vkey\|operational_cert\.hot_vkey` outside `opcert.rs`).
2. **No parallel opcert decoders.** Grep for `pub fn .*opcert.*decode\|pub fn decode.*opcert\|pub fn .*read_opcert` outside `crates/ade_codec/src/shelley/opcert.rs`.
3. **`OpCertError` is a closed sum (no `#[non_exhaustive]`).** Grep for
   `#[non_exhaustive]` immediately preceding `pub enum OpCertError` —
   forbidden.
4. **`OpCertCodecError` is a closed sum (no `#[non_exhaustive]`).** Same.
5. **No production call site of `opcert_validate` outside
   `crates/ade_core/src/consensus/` and `crates/ade_runtime/src/producer/`.**
   Grep for `opcert_validate(` across `crates/*/src/`; test callers under
   `crates/*/tests/` are whitelisted.
6. **`opcert_validate` is a free function, not a method on an opcert
   type.** Grep for `impl .*OperationalCert.*\bvalidate\b\|impl .*opcert.*\bvalidate\b` — finding any match in source is a failure (validate is RED→BLUE entry, not a method on a value).

### 6. Registry updates (same commit)

Flip these to `enforced` and populate `tests` + `ci_script`:

- `DC-CONS-11` — `tests = ["opcert_validate_accepts_canonical_fixture",
  "opcert_validate_rejects_period_mismatch",
  "opcert_validate_rejects_short_hot_vkey",
  "opcert_validate_first_opcert_no_prev_counter"]`,
  `ci_script = "ci/ci_check_opcert_closed.sh"`,
  `code_locus = "crates/ade_core/src/consensus/opcert_validate.rs (opcert_validate, OpCertError); crates/ade_codec/src/shelley/opcert.rs (encode_opcert, decode_opcert, OpCertCodecError)"`,
  `status = "enforced"`.
- `DC-CONS-12` — `tests = ["opcert_validate_rejects_counter_regression",
  "opcert_validate_rejects_counter_repeat",
  "opcert_validate_rejects_bad_signature_over_cold_key"]`,
  `ci_script = "ci/ci_check_opcert_closed.sh"`,
  `code_locus = "crates/ade_core/src/consensus/opcert_validate.rs (opcert_validate, OpCertError::{CounterRepeat, CounterRegression, BadColdSignature})"`,
  `status = "enforced"`.

`opcert_encoder_matches_cardano_cli_byte_identical` and the codec-side
round-trip / negative tests belong to the *encoder closed-grammar* claim
under `DC-CONS-11`'s code_locus footprint (the opcert encoder is the
single producer-side authority that `DC-CONS-11`'s period-stamp check
operates over).

---

## Mechanical Acceptance Criteria

- **AC-1** — `cargo build --workspace` green.
- **AC-2** — `cargo test -p ade_codec shelley::opcert` green
  (8 unit tests pass).
- **AC-3** — `cargo test -p ade_core consensus::opcert_validate` green
  (7 unit tests pass).
- **AC-4** — `cargo test --workspace` green (no pre-existing tests
  regress; specifically every `*_determinism`, `roundtrip_*`, and
  `*_replay_*` test passes — header CBOR bytes unchanged).
- **AC-5** — `bash ci/ci_check_opcert_closed.sh` returns `PASS`
  (all 6 guards).
- **AC-6** — `bash ci/ci_check_constitution_coverage.sh` returns `PASS`
  (registry edits round-trip; DC-CONS-11/12 status fields parse as
  `"enforced"`).
- **AC-7** — `bash ci/ci_check_private_key_custody.sh` returns `PASS`
  (S1's gate still green; S2 introduces no BLUE private-key types).
- **AC-8** — `grep -rE 'cbor::write_bytes_canonical.*hot_vkey'
  crates/ade_codec/src/shelley/block.rs` returns no matches (header
  path delegates fully; sanity-mirrors guard 1 of the CI gate).

---

## Hard Prohibitions

Cluster-level prohibitions inherited (cluster.md §Forbidden during
this cluster). Slice-specific additions:

- No `pub fn .*encode.*opcert` outside
  `crates/ade_codec/src/shelley/opcert.rs`.
- No `pub fn .*decode.*opcert` outside
  `crates/ade_codec/src/shelley/opcert.rs`.
- No `#[non_exhaustive]` on `OpCertError` or `OpCertCodecError`.
- No `impl OperationalCert { fn validate(...) }` — validation is a free
  function at the RED→BLUE boundary.
- No alteration of `ade_crypto::kes::verify_opcert` /
  `build_opcert_signable` semantics — that path is the established
  cold-signature authority; S2 only wraps it.
- No alteration of `ade_crypto::kes::OperationalCertData` shape — that
  type is the *verifier* shape; S2's `opcert_validate` accepts the
  *header* shape `ade_types::shelley::block::OperationalCert` and
  converts internally.
- No introduction of `HashMap` / `HashSet` / wall-clock / RNG / float /
  `std::fs` / `std::env` / `String`-bearing errors in
  `opcert_validate.rs` or `opcert.rs`.
- No re-introduction of opcert byte emission inline in
  `crates/ade_codec/src/shelley/block.rs` — the header path may only
  delegate.
- No path-string-bearing variants in `OpCertCodecError` or
  `OpCertError` (no filesystem layout leaks).
- No re-encoding of header CBOR through the opcert path during VALIDATE
  (would break T-ENC-01 preserved-bytes discipline). `opcert_validate`
  consumes the typed `OperationalCert` value, never the header bytes.

---

## Explicit Non-Goals

- RED `vrf_prove` / `kes_sign` / `kes_update` — landed in S1.
- BLUE `forge_block` / `ProducerTick` — that's S3 (CE-N-C-3).
- Body-hash parity / validator-shared encoder — that's S4 (CE-N-C-4).
- Self-acceptance gate — that's S5 (CE-N-C-5).
- Scheduler / tick-assembler / broadcast — that's S6 (CE-N-C-6).
- Cross-impl adapter + live-evidence binary — that's S7 (CE-N-C-7/8).
- Opcert *issuance / renewal* (the cardano-cli workflow remains the
  minting authority per OQ-6). S2 ships acceptance + serialization
  parity only.
- Durable per-node opcert-counter store. S2's `prev_counter: Option<u64>`
  is a pure input on the validate call; the RED-side persistence of the
  counter lives in S6 (`ade_runtime::producer::keys` or a sibling).
- Promoting `OP-OPS-04` from `declared` to `enforced` — blocked by the
  schema gap recorded in `OP-OPS-04.open_obligation`. Resolution stays
  at cluster-close.

---

## Failure Modes

Every `OpCertError` and `OpCertCodecError` variant is deterministic and
contains no key bytes. Every variant is fail-fast at the RED→BLUE
boundary: an opcert that fails validate must NOT proceed to `forge_block`
(enforced by S3's typed input `ProducerTick`, which carries an
`OperationalCert` only via an `OpCertValidated` newtype that
`opcert_validate` returns).

Note: this slice ships only the function and its error sum. The newtype
gate at the `ProducerTick` boundary lands in S3. For S2, the contract is:
calling `opcert_validate` is the producer's only sanctioned acceptance
path; downstream consumers MUST hold an `Ok(())` verdict before
proceeding.

---

## Notes on grounding

Verified at HEAD `9727bd9`:

- `ade_types::shelley::block::OperationalCert` exists with fields
  `hot_vkey: Vec<u8>`, `sequence_number: u64`, `kes_period: u64`,
  `sigma: Vec<u8>` (`crates/ade_types/src/shelley/block.rs:74`).
- `ade_crypto::kes::OperationalCertData` exists with fields
  `hot_vkey: KesVerificationKey`, `sequence_number: u64`,
  `kes_period: u64`, `cold_signature: Ed25519Signature`
  (`crates/ade_crypto/src/kes.rs:184`).
- `ade_crypto::kes::verify_opcert` exists at `crates/ade_crypto/src/kes.rs:208`
  and verifies the cold-key Ed25519 signature over the canonical
  signable `hot_vkey || seq# BE || kes_period BE`.
- Existing inline opcert emit in `crates/ade_codec/src/shelley/block.rs:269-272`
  and `:279-282` writes the four fields directly into the header CBOR
  buffer; S2 refactors these into a delegating call to
  `opcert::write_opcert_fields_into`.
- No existing `*.opcert` fixtures in `corpus/` or
  `crates/ade_testkit/fixtures/`; the synthetic fixtures S2 introduces
  are the first.

The canonical cardano-cli `node issue-op-cert` CBOR shape is documented
in cardano-api source under `Cardano.Api.OperationalCertificate`; the
4-tuple `[bstr32, uint, uint, bstr64]` is the format S2 pins via
`opcert_encoder_matches_cardano_cli_byte_identical`. The byte equality
is independently re-confirmed end-to-end at S7's live-evidence step
(forged blocks with opcerts in headers must be accepted by cardano-node).
