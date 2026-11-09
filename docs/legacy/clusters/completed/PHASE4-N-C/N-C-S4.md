# Invariant Slice — PHASE4-N-C S4

## Slice Header
**Slice Name:** BLUE body-hash parity via single Cardano-compatible canonical body-hash authority
**Cluster:** PHASE4-N-C
**Status:** Merged
**CEs addressed:** CE-N-C-4 (body-hash parity via validator-shared encoder)
**Registry flips on merge:** `DC-CONS-16` → `enforced`
**Dependencies:** S1, S2, S3 merged. S3 produces forged blocks whose `body_hash` already byte-equals the validator's recomputation by construction (every `roundtrip_*` and `*_determinism` test stayed green at S3 close). S4 makes the equality mechanically *locked* rather than incidentally true.

---

## Intent

Lift the body-hash recipe to a single named authority and prove producer
and validator share it.

At S3 close, two private functions compute the body_hash by the same
recipe:
- `ade_ledger::block_validity::header_input::block_body_hash` (private,
  validator side, takes `&ShelleyBlock`)
- `ade_ledger::producer::forge::compute_body_hash` (private, producer
  side, takes 4 byte slices)

Both produce identical bytes for the same inputs (extractive
correctness already proven by S3's replay corpus + the full
`roundtrip_*` / `*_determinism` suite remaining green). S4 unifies
them: a single `pub fn block_body_hash_from_buckets(...) -> Hash32`
in a new `ade_ledger::block_body_hash` module is the only function
in the workspace that computes the body-hash recipe. Both the
validator wrapper and the producer's forge call it. A CI grep gate
forbids any second definition.

The invariant impact: the producer and validator cannot drift on
body_hash bytes by independent edits — they share a single function
by construction. Strengthens `T-ENC-01` for the producer surface.

---

## The change (atomic; compile green as one unit)

### 1. New module `crates/ade_ledger/src/block_body_hash.rs` (BLUE)

The single body-hash authority in the workspace.

```rust
//! Canonical body-hash authority. The only function in the workspace
//! that computes the Cardano block body-hash recipe; both
//! `block_validity::header_input::HeaderInput::computed_body_hash`
//! and `producer::forge::forge_block` call this.

use ade_crypto::blake2b_256;
use ade_types::Hash32;
use ade_types::shelley::block::ShelleyBlock;

/// Compute the Cardano block body hash from the four preserved CBOR
/// byte buckets. For Alonzo+ (Conway included) the recipe is:
///
///   body_hash = blake2b_256(
///       blake2b_256(tx_bodies)
///    || blake2b_256(witness_sets)
///    || blake2b_256(metadata)
///    || blake2b_256(invalid_txs OR empty bytes)
///   )
///
/// Each input slice is the PRESERVED CBOR for the corresponding bucket,
/// including its outer array/map header (as carried in
/// `ShelleyBlock.{tx_bodies, witness_sets, metadata, invalid_txs}`).
/// `invalid_txs == None` is hashed as the empty byte string.
pub fn block_body_hash_from_buckets(
    tx_bodies: &[u8],
    witness_sets: &[u8],
    metadata: &[u8],
    invalid_txs: Option<&[u8]>,
) -> Hash32 {
    let h_tx = blake2b_256(tx_bodies).0;
    let h_ws = blake2b_256(witness_sets).0;
    let h_md = blake2b_256(metadata).0;
    let h_iv = blake2b_256(invalid_txs.unwrap_or(&[])).0;
    let mut concat = [0u8; 128];
    concat[0..32].copy_from_slice(&h_tx);
    concat[32..64].copy_from_slice(&h_ws);
    concat[64..96].copy_from_slice(&h_md);
    concat[96..128].copy_from_slice(&h_iv);
    Hash32(blake2b_256(&concat).0)
}

/// Compute the body hash of a `ShelleyBlock` value. Thin wrapper over
/// `block_body_hash_from_buckets` that destructures the block's four
/// bucket fields.
pub fn block_body_hash(block: &ShelleyBlock) -> Hash32 {
    block_body_hash_from_buckets(
        &block.tx_bodies,
        &block.witness_sets,
        &block.metadata,
        block.invalid_txs.as_deref(),
    )
}
```

### 2. Wire `pub mod block_body_hash;` from `crates/ade_ledger/src/lib.rs`

### 3. Refactor `crates/ade_ledger/src/block_validity/header_input.rs`

Delete the private `fn block_body_hash(block: &ShelleyBlock) -> Hash32`
(currently at line 151). Replace its call site at line 62 with
`crate::block_body_hash::block_body_hash(block)`.

The validator's behaviour is byte-identical to before — same recipe,
same input, same output. Existing tests prove this.

### 4. Refactor `crates/ade_ledger/src/producer/forge.rs`

Delete the private `fn compute_body_hash(...)` (currently at line 338).
Replace its call site at line 268 with
`crate::block_body_hash::block_body_hash_from_buckets(&tx_bodies, &witness_sets, &metadata, invalid_txs.as_deref())`.

Adjust the existing `use ade_crypto::blake2b_256;` import — if no
other call sites in `forge.rs` use `blake2b_256` directly, remove the
import. (S4 implementer verifies and prunes.)

### 5. New unit tests

In `crates/ade_ledger/src/block_body_hash.rs` `#[cfg(test)] mod tests`
(BLUE):

- `block_body_hash_pinned_recipe_byte_identical` — for a hand-pinned
  fixture (tx_bodies=`80` (empty array), witness_sets=`80`, metadata=`a0` (empty map), invalid_txs=`None`), the computed body_hash equals a
  hand-computed reference value. This is the canonical recipe lock.
- `block_body_hash_from_block_equals_from_buckets` — for a fixture
  `ShelleyBlock`, `block_body_hash(&block)` equals
  `block_body_hash_from_buckets(&block.tx_bodies, &block.witness_sets,
  &block.metadata, block.invalid_txs.as_deref())`.
- `block_body_hash_none_invalid_txs_equals_empty_bucket` — for two
  fixtures that differ ONLY in `invalid_txs: None` vs `Some(&[])`, the
  computed body_hash bytes are identical (since `unwrap_or(&[])` maps
  both to the same hash input).

In `crates/ade_testkit/src/producer/replay.rs` `#[cfg(test)] mod tests`
(extends S3's replay suite):

- `forged_body_hash_matches_validator_recomputation` — for every
  positive fixture in `producer_replay_fixtures()`, decode the forged
  block via `ade_codec`, then call
  `ade_ledger::block_body_hash::block_body_hash(&decoded_block)` and
  assert it byte-equals `decoded_block.header.body.body_hash`. Also
  assert it equals the original forged ProducerTick's expected hash
  derived from the same call chain (the producer/validator parity
  property).
- `body_encoder_is_single_authority` — a Rust compile-time + runtime
  check:
  ```rust
  // Compile-time: only one function symbol with the canonical signature.
  const _A: fn(&[u8], &[u8], &[u8], Option<&[u8]>) -> Hash32 =
      ade_ledger::block_body_hash::block_body_hash_from_buckets;
  const _B: fn(&ade_types::shelley::block::ShelleyBlock) -> Hash32 =
      ade_ledger::block_body_hash::block_body_hash;
  // Runtime: cross-call agreement on a fixture.
  let block = /* fixture */;
  assert_eq!(
      ade_ledger::block_body_hash::block_body_hash(&block),
      ade_ledger::block_body_hash::block_body_hash_from_buckets(
          &block.tx_bodies, &block.witness_sets, &block.metadata,
          block.invalid_txs.as_deref(),
      ),
  );
  ```
  This pins both signatures; if either gets renamed or a parallel one
  added, the test breaks.

### 6. New CI gate `ci/ci_check_no_producer_body_encoder.sh`

Mechanical guards:

1. **Single canonical body-hash function.** `grep -rE 'pub fn
   block_body_hash\b|pub fn block_body_hash_from_buckets\b'
   crates/ade_ledger/src/ crates/ade_core/src/ crates/ade_codec/src/
   crates/ade_types/src/ crates/ade_crypto/src/` must return EXACTLY
   two matches, both in
   `crates/ade_ledger/src/block_body_hash.rs`. Any other location is
   a failure.
2. **No private `fn` that re-implements the recipe.** Grep all BLUE
   crates for the pattern of "four `blake2b_256` calls followed by a
   128-byte concat followed by a final `blake2b_256`":
   `grep -rE 'fn (compute_body_hash|block_body_hash_inner|recompute_body_hash)'`
   anywhere in BLUE crates is a failure. Additionally, scan for
   `let mut concat = \[0u8; 128\];` outside
   `crates/ade_ledger/src/block_body_hash.rs` — any match is a failure.
3. **No `blake2b_256` import in `producer/forge.rs`.** If forge no
   longer needs the import (S4 refactor removes the direct blake2b
   recipe), the import line must be absent. Grep
   `crates/ade_ledger/src/producer/forge.rs` for
   `use ade_crypto::blake2b_256` — if matched, the gate fails.
   (Implementer verifies whether other uses of `blake2b_256` remain
   in forge.rs; if so, this guard relaxes to "no blake2b_256(... 128 ...)
   patterns in forge.rs" — but the cleanest outcome is full removal.)
4. **Forge calls the canonical function.** Grep
   `crates/ade_ledger/src/producer/forge.rs` for substring
   `block_body_hash_from_buckets(` — must appear at least once.
5. **Validator calls the canonical function.** Grep
   `crates/ade_ledger/src/block_validity/header_input.rs` for
   substring `block_body_hash::block_body_hash(` (or the equivalent
   re-imported call) — must appear at least once.

### 7. Registry updates (same commit)

Flip `DC-CONS-16` to `enforced` with populated arrays:

- `DC-CONS-16` — `tests = ["block_body_hash_pinned_recipe_byte_identical",
  "block_body_hash_from_block_equals_from_buckets",
  "block_body_hash_none_invalid_txs_equals_empty_bucket",
  "forged_body_hash_matches_validator_recomputation",
  "body_encoder_is_single_authority"]`,
  `ci_script = "ci/ci_check_no_producer_body_encoder.sh"`,
  `code_locus = "crates/ade_ledger/src/block_body_hash.rs (block_body_hash, block_body_hash_from_buckets — single canonical authority); crates/ade_ledger/src/producer/forge.rs (forge_block — consumer); crates/ade_ledger/src/block_validity/header_input.rs (computed_body_hash — consumer)"`,
  `status = "enforced"`.

`T-ENC-01.strengthened_in` gains `PHASE4-N-C` at cluster-close (not in
this slice).

---

## Mechanical Acceptance Criteria

- **AC-1** — `cargo build --workspace` green.
- **AC-2** — `cargo test -p ade_ledger block_body_hash` green (3 unit
  tests).
- **AC-3** — `cargo test -p ade_testkit producer::replay` green
  (existing 2 replay tests + new `forged_body_hash_matches_validator_recomputation`
  + `body_encoder_is_single_authority` = 4 total).
- **AC-4** — `cargo test --workspace` green. The pre-existing
  `boundary_fingerprint_matches_pins` failure is the only allowed
  pre-existing fail. Every `roundtrip_*` and `*_determinism` test must
  remain green (the refactor preserves validator bytes).
- **AC-5** — `bash ci/ci_check_no_producer_body_encoder.sh` returns
  `PASS` (all 5 guards).
- **AC-6** — `bash ci/ci_check_constitution_coverage.sh` returns
  `PASS` (DC-CONS-16 status flips to `"enforced"`).
- **AC-7** — `bash ci/ci_check_forge_purity.sh` returns `PASS`
  (S3's gate; forge still pure after refactor — purity is not
  affected because the canonical function is in the same crate's
  BLUE space).
- **AC-8** — `bash ci/ci_check_private_key_custody.sh` returns `PASS`
  (S1's gate unregressed).
- **AC-9** — `bash ci/ci_check_opcert_closed.sh` returns `PASS`
  (S2's gate unregressed).
- **AC-10** — `grep -c 'pub fn block_body_hash' crates/ade_ledger/src/block_body_hash.rs`
  returns exactly `2` (the two canonical entries: the by-block wrapper
  and the by-buckets function). Sanity-mirrors guard 1 of the CI gate.
- **AC-11** — `grep -rE 'fn compute_body_hash|fn recompute_body_hash'
  crates/` returns no matches (the private parallel implementations are
  fully removed).

---

## Hard Prohibitions

Cluster-level prohibitions inherited (cluster.md §Forbidden during this
cluster). Slice-specific additions:

- No `pub fn block_body_hash` or `pub fn block_body_hash_from_buckets`
  outside `crates/ade_ledger/src/block_body_hash.rs`.
- No private function in any BLUE crate that re-implements the recipe
  (four `blake2b_256` calls + 128-byte concat + final `blake2b_256`).
  Including `compute_body_hash`, `recompute_body_hash`,
  `block_body_hash_inner`, or any name pattern that matches the recipe
  shape.
- No `let mut concat = [0u8; 128];` outside
  `crates/ade_ledger/src/block_body_hash.rs`.
- No alteration to the recipe formula itself. The slice ships the
  EXISTING recipe, lifted; any change to the formula would be a
  hash-critical wire-protocol break.
- No `String` / `HashMap` / `std::time` / `std::fs` / `rand` in
  `block_body_hash.rs`.
- No `#[non_exhaustive]` on any new type (no new types are introduced,
  but defensive).
- No re-export of the canonical function from any *other* crate (only
  `ade_ledger` owns it; consumers `use ade_ledger::block_body_hash::*`).
- No `pub use` shortcut that exposes the canonical function under a
  second name (would create a parallel-name false positive on the CI
  gate; future readers would also assume two functions).

---

## Explicit Non-Goals

- RED signing primitives — S1 (landed).
- BLUE `opcert_validate` — S2 (landed).
- BLUE forge core — S3 (landed; this slice refactors a private
  helper but does not change forge's external surface).
- BLUE self-acceptance gate — S5 (CE-N-C-5).
- Scheduler / tick-assembler / broadcast — S6 (CE-N-C-6).
- Cross-impl adapter + live-evidence binary — S7 (CE-N-C-7/8).
- New tx-component splitter — S3 (landed).
- Lifting `decode_shelley_block_inner` or `ShelleyBlock::ade_encode`
  to be the "single block encoder." Those are *block-level* encoders
  (different from the body-hash recipe). The block-level encoder
  closure is already adequate for the cluster: a single
  `ShelleyBlock::ade_encode` exists in `ade_codec`, and forge uses it.
  S4 scopes only to the body-hash recipe authority.
- Promoting `T-ENC-01.strengthened_in` — cluster-close.
- Resolving the OP-OPS-04 schema gap — cluster-close.

---

## Failure Modes

S4 introduces no new failure modes. The refactor is byte-preserving:
the recipe and its inputs are unchanged. Existing failure modes in
forge (NotLeader, TxSetNotAdmissiblePrefix, etc.) and in the validator
(BlockValidityError variants over header / body mismatch) are
unaffected.

If the refactor accidentally changes the recipe bytes for any input,
every existing `roundtrip_*` / `*_determinism` test fails. That's a
release-blocker; AC-4 is the gate.

---

## Grounding (verified at HEAD `8312690`)

- `ade_ledger::block_validity::header_input::block_body_hash` (private
  fn) at `crates/ade_ledger/src/block_validity/header_input.rs:151`.
  Called once at line 62
  (`let computed_body_hash = Hash32(block_body_hash(block).0);`).
- `ade_ledger::producer::forge::compute_body_hash` (private fn) at
  `crates/ade_ledger/src/producer/forge.rs:338`. Called once at
  line 268.
- Both functions are byte-equivalent for the same inputs — proven by
  S3's `forge_block_replay_byte_identical` (the producer-side body_hash
  is correct because the test fixtures pass) and by every `roundtrip_*`
  test remaining green (the validator-side recomputation is byte-equal
  to what the producer wrote). S4 makes this equivalence mechanical by
  collapsing the two implementations into one.
- `ade_types::shelley::block::ShelleyBlock` field shape pinned:
  `{ header, tx_count, tx_bodies, witness_sets, metadata, invalid_txs }`
  (`crates/ade_types/src/shelley/block.rs:15`). `invalid_txs:
  Option<Vec<u8>>` per the existing type.
- `ade_crypto::blake2b::blake2b_256` is the canonical hash primitive
  used by both call sites today (`crates/ade_crypto/src/blake2b.rs`).
- `ade_ledger::block_body_hash` module does NOT yet exist — this slice
  creates it.

---

## Notes on the "encoder" vs "hash recipe" distinction

The slice doc above is careful with terminology. "Body encoder" is
not a separate function in this codebase — the producer ASSEMBLES the
four byte buckets in `forge.rs` (via `split_conway_tx_components` plus
inline CBOR header writes), and the validator READS them from
preserved bytes in `decode_shelley_block_inner`. There is no
parallel-encoder collapse to do at this layer.

What IS shared and now centralized is the **hash recipe** — the
function that computes `body_hash` from the four bucket byte strings.
That's what S4's `block_body_hash_from_buckets` ships, and what
`CE-N-C-4` and `DC-CONS-16` actually pin.

The cluster doc CE-N-C-4 phrasing "no encoder bifurcation" is
honored by ensuring exactly one function in the workspace computes
the recipe. Any future drift in HOW the producer assembles the
buckets is bounded by the validator's `block_body_hash`
recomputation: if the producer writes different bucket bytes than
the validator reads, the validator's `computed_body_hash` will not
match the producer's `header.body_hash`, every `*_determinism` test
fails, and the bug is caught.
