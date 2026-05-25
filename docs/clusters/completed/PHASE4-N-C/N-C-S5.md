# Invariant Slice — PHASE4-N-C S5

## Slice Header
**Slice Name:** BLUE `self_accept` bridge — type-level gate on RED broadcast
**Cluster:** PHASE4-N-C
**Status:** Merged
**CEs addressed:** CE-N-C-5 (self-acceptance bridge — no broadcast without validator agreement)
**Registry flips on merge:** `CN-CONS-07` → `enforced`
**Dependencies:** S1, S2, S3, S4 merged. S3 produces `ForgedBlock { bytes, block }`. S4 unified the body-hash recipe; the validator's `block_validity` recomputes `body_hash` over the same encoder forge wrote, so the parity property holds by construction.

---

## Intent

Make it a **type-level impossibility** for the RED scheduler (S6) to
broadcast forged bytes that Ade's own validator would reject.

Forge (S3) produces raw block bytes; the validator (`ade_ledger::block_validity::transition::block_validity`)
is the single closed authority for "is this block acceptable." S5
wraps them into one function `self_accept` whose only success return
is a newtype `AcceptedBlock`. The newtype's constructor is
module-private to `self_accept`; the broadcast surface (S6) accepts
only `AcceptedBlock`. RED therefore cannot construct a broadcastable
value without passing through BLUE validation.

The invariant impact: producer/validator drift becomes a release-blocking
test failure, not a production incident. A forged block whose body_hash,
KES signature, leader claim, or body validity disagrees with Ade's own
validator halts the producer deterministically before any bytes leave
the host.

---

## The change (atomic; compile green as one unit)

### 1. New module `crates/ade_ledger/src/producer/self_accept.rs` (BLUE)

```rust
//! Self-acceptance bridge: a forged block cannot be broadcast unless
//! Ade's own validator (header + body) accepts it under the same slot,
//! era, and context. The `AcceptedBlock` newtype is the type-level
//! broadcast token — its private constructor lives only here.

use ade_core::consensus::era_schedule::EraSchedule;
use ade_core::consensus::ledger_view::LedgerView;
use ade_core::consensus::praos_state::PraosChainDepState;

use crate::block_validity::transition::{block_validity, BlockValidityOutcome};
use crate::block_validity::verdict::BlockValidityError;
use crate::state::LedgerState;

/// The closed self-accept verdict.
#[derive(Debug, Clone, PartialEq)]
pub enum SelfAcceptError {
    /// The full block validator (header + body-hash bind + body apply)
    /// rejected the forged bytes. Carries the underlying validator
    /// error verbatim — same `BlockValidityError` surface the
    /// receive-side validator emits.
    Rejected(BlockValidityError),
}

/// The type-level broadcast token. RED `broadcast` consumes this; it
/// has no constructor outside this module, so the only way to obtain
/// one is via `self_accept` returning `Ok(...)`.
#[derive(Debug, Clone, PartialEq)]
pub struct AcceptedBlock {
    /// The forged block CBOR bytes. Exposed read-only via
    /// `as_bytes()`; no public field, no `pub fn from_*` constructor.
    bytes: Vec<u8>,
}

impl AcceptedBlock {
    /// Public read-only access for the broadcast layer.
    pub fn as_bytes(&self) -> &[u8] {
        &self.bytes
    }

    /// Convert to `Vec<u8>` for hand-off to the broadcast queue. Total,
    /// no observable nondeterminism.
    pub fn into_bytes(self) -> Vec<u8> {
        self.bytes
    }
}

/// Wrap forged bytes in `AcceptedBlock` IFF the validator accepts them
/// under the supplied context. Pure, total, deterministic.
///
/// Pipeline (matches the receive-side validator exactly):
/// 1. `block_validity(ledger, chain_dep, era_schedule, ledger_view,
///    forged_bytes)` runs the full validator chain — decode + header
///    validate + body-hash bind + body apply.
/// 2. If the verdict is `BlockValidityOutcome::Valid`, return
///    `Ok(AcceptedBlock { bytes: forged_bytes.to_vec() })`.
/// 3. If the verdict is `BlockValidityOutcome::Invalid`, return
///    `Err(SelfAcceptError::Rejected(error))`.
pub fn self_accept(
    forged_bytes: &[u8],
    ledger: &LedgerState,
    chain_dep: &PraosChainDepState,
    era_schedule: &EraSchedule,
    ledger_view: &dyn LedgerView,
) -> Result<AcceptedBlock, SelfAcceptError> {
    match block_validity(ledger, chain_dep, era_schedule, ledger_view, forged_bytes) {
        BlockValidityOutcome::Valid { .. } => Ok(AcceptedBlock {
            bytes: forged_bytes.to_vec(),
        }),
        BlockValidityOutcome::Invalid { error, .. } => {
            Err(SelfAcceptError::Rejected(error))
        }
    }
}
```

### 2. Wire `pub mod self_accept;` from `crates/ade_ledger/src/producer/mod.rs` and re-export the public surface

```rust
// crates/ade_ledger/src/producer/mod.rs (existing)
pub mod forge;
pub mod self_accept;
pub mod state;

pub use self_accept::{self_accept, AcceptedBlock, SelfAcceptError};
```

### 3. New unit tests (in `crates/ade_ledger/src/producer/self_accept.rs` `#[cfg(test)] mod tests`)

These use the existing S3 replay fixtures (re-exported through
`ade_testkit::producer::fixtures`) to obtain forged bytes plus
their producing context. The fixtures already drive the
`forged_body_hash_matches_validator_recomputation` test from S4, so
the positive-path validator inputs are reachable from BLUE test code.

If `ade_testkit` is awkward to consume from `ade_ledger`'s
`#[cfg(test)]` (cyclic dev-dep), inline a minimal happy-path fixture
constructor here that builds a forged block deterministically and
captures the matching ledger/chain_dep/era_schedule/ledger_view
context.

Positive:
- `self_accept_accepts_freshly_forged_block` — forge → self_accept
  pipeline over the `fixture_empty_mempool_leader` shape. Asserts
  `Ok(AcceptedBlock { .. })` and that
  `accepted.as_bytes() == forged.bytes`.

Adversarial (all assert `Err(SelfAcceptError::Rejected(_))`):
- `self_accept_rejects_corrupted_body_hash` — flip one byte of
  `header.body.body_hash` in the forged bytes (re-encode header) →
  body-hash bind step rejects.
- `self_accept_rejects_invalid_kes_signature` — flip one byte of the
  KES signature field in the forged bytes → KES verify rejects.
- `self_accept_rejects_unbalanced_tx_in_body` — replace one tx body
  with bytes that re-decode but violate UTxO balance → body validator
  rejects.

Type-level / runtime-witness:
- `broadcast_callable_only_with_accept_verdict` — a compile-time witness
  that the only constructor of `AcceptedBlock` is via `self_accept`.
  Implementation: a `static_assertions::assert_not_impl_any!` or
  manual proof. Concretely, since the `bytes` field is private and no
  `pub fn` other than `as_bytes` / `into_bytes` / `Debug` / `Clone` /
  `PartialEq` exists, no external caller can construct one. The test
  is a compile-time guard: a `#[cfg(test)]` proof that
  `AcceptedBlock::default()` does NOT exist (no `Default` impl, no
  `new`) by relying on `let _: AcceptedBlock = unimplemented!();`
  patterns is awkward. The cleanest mechanical proof is the CI grep
  gate (§4 guard 1). The test name remains here for the registry
  link; its assertion can be:
  ```rust
  // The only way to obtain an `AcceptedBlock` is via `self_accept`.
  // This test re-asserts the runtime accept and pins the token's
  // public API surface (compile error if anything changes).
  let block: AcceptedBlock = make_accepted_block_via_self_accept();
  let _bytes_ref: &[u8] = block.as_bytes();
  let _bytes_owned: Vec<u8> = block.into_bytes();
  ```
  Compile-time + the CI grep gate together close the gap.

### 4. New CI gate `ci/ci_check_self_accept_gate.sh`

Mechanical guards:

1. **`AcceptedBlock` has no public constructor outside `self_accept.rs`.**
   - `grep -rE 'AcceptedBlock\s*\{' crates/` must show matches ONLY in
     `crates/ade_ledger/src/producer/self_accept.rs` (the struct literal
     inside `self_accept` itself) and in `#[cfg(test)]` blocks (these
     blocks live in the same file, so the file-scope grep is sufficient).
   - `grep -rE 'pub fn .*-> AcceptedBlock' crates/` must return EXACTLY
     one match in `crates/ade_ledger/src/producer/self_accept.rs`
     (the `self_accept` function itself, which returns
     `Result<AcceptedBlock, SelfAcceptError>` — the regex matches the
     `AcceptedBlock` portion of the return).
   - No `impl Default for AcceptedBlock` / `impl From<.*> for
     AcceptedBlock` / `impl TryFrom<.*> for AcceptedBlock` anywhere
     in the workspace.
2. **`AcceptedBlock.bytes` field is private.**
   - Grep `crates/ade_ledger/src/producer/self_accept.rs` for
     `pub bytes:` — finding a match is a failure.
3. **`SelfAcceptError` is a closed sum (no `#[non_exhaustive]`).**
4. **`self_accept` calls the canonical `block_validity` validator.**
   - Grep `crates/ade_ledger/src/producer/self_accept.rs` for
     `block_validity(` — must appear.
5. **`self_accept` does NOT re-implement the validator pipeline.**
   - Grep `crates/ade_ledger/src/producer/self_accept.rs` for
     `validate_and_apply_header(` / `decode_block(` / `block_body_hash(` —
     finding any of these is a failure (the wrapper must delegate to
     `block_validity`, not duplicate sub-steps).
6. **No `pub fn` in `self_accept.rs` returns raw forged bytes other than
   through `AcceptedBlock`.**
   - Grep `pub fn .*-> Vec<u8>\|pub fn .*-> &\[u8\]` in
     `self_accept.rs` — matches OK only for `into_bytes`/`as_bytes`
     methods *on* `AcceptedBlock` (they decompose the token after the
     accept verdict, the broadcast-layer hand-off). The gate's regex
     is tightened to flag `pub fn (?!as_bytes|into_bytes)` returning
     byte-blob types.

### 5. Registry updates (same commit)

Flip `CN-CONS-07` to `enforced` with populated arrays:

- `CN-CONS-07` — `tests = ["self_accept_accepts_freshly_forged_block",
  "self_accept_rejects_corrupted_body_hash",
  "self_accept_rejects_invalid_kes_signature",
  "self_accept_rejects_unbalanced_tx_in_body",
  "broadcast_callable_only_with_accept_verdict"]`,
  `ci_script = "ci/ci_check_self_accept_gate.sh"`,
  `code_locus = "crates/ade_ledger/src/producer/self_accept.rs (self_accept, AcceptedBlock, SelfAcceptError); crates/ade_ledger/src/block_validity/transition.rs (block_validity — single closed validator authority self_accept wraps)"`,
  `status = "enforced"`.

CN-CONS-07 is a `tier = "release"` entry (set in the original
registry append). The constitution-coverage validator should accept
`code_locus` / `tests` / `ci_script` on release tier when status is
enforced — verify at gate run.

---

## Mechanical Acceptance Criteria

- **AC-1** — `cargo build --workspace` green.
- **AC-2** — `cargo test -p ade_ledger producer::self_accept` green
  (5 tests).
- **AC-3** — `cargo test --workspace` green except the pre-existing
  `boundary_fingerprint_matches_pins` failure. Every `roundtrip_*`,
  `*_determinism`, and `*_replay_*` test remains green.
- **AC-4** — `bash ci/ci_check_self_accept_gate.sh` returns `PASS`
  (6 guards).
- **AC-5** — `bash ci/ci_check_constitution_coverage.sh` returns
  `PASS` (CN-CONS-07 status flips to `"enforced"`).
- **AC-6** — `bash ci/ci_check_no_producer_body_encoder.sh` returns
  `PASS` (S4 unregressed).
- **AC-7** — `bash ci/ci_check_forge_purity.sh` returns `PASS`
  (S3 unregressed).
- **AC-8** — `bash ci/ci_check_no_private_keys_in_corpus.sh` returns
  `PASS` (S3 unregressed).
- **AC-9** — `bash ci/ci_check_private_key_custody.sh` returns `PASS`
  (S1 unregressed).
- **AC-10** — `bash ci/ci_check_opcert_closed.sh` returns `PASS`
  (S2 unregressed).
- **AC-11** — `grep -rE 'pub fn .*-> AcceptedBlock' crates/` returns
  exactly one match (sanity-mirrors guard 1 of the CI gate).
- **AC-12** — `grep -rE 'impl (Default|From|TryFrom).*for AcceptedBlock'
  crates/` returns no matches.

---

## Hard Prohibitions

Cluster-level prohibitions inherited. Slice-specific additions:

- No public constructor of `AcceptedBlock` outside
  `crates/ade_ledger/src/producer/self_accept.rs`.
- No `pub` on `AcceptedBlock.bytes`.
- No `impl Default | From<*> | TryFrom<*> for AcceptedBlock` anywhere.
- No `#[non_exhaustive]` on `SelfAcceptError`.
- No re-implementation of the validator pipeline inside
  `self_accept.rs` — must delegate to `block_validity`.
- No `String`-bearing variant on `SelfAcceptError` (delegated to
  `BlockValidityError` whose variants are already closed structured
  errors).
- No `std::time` / `rand` / `std::fs` / `std::env` / `HashMap`
  iteration / floats / `println!` / `dbg!` / `async fn` in
  `self_accept.rs`.
- No `unsafe`, no `transmute`, no `Box<dyn Any>` tricks to bypass the
  token's private field.
- No `Serialize` impl on `AcceptedBlock` that would let RED reconstruct
  one from disk. (`Debug` + `Clone` + `PartialEq` only.)

---

## Explicit Non-Goals

- RED signing primitives — S1.
- BLUE `opcert_validate` — S2.
- BLUE forge core — S3.
- BLUE body-hash unification — S4.
- RED `broadcast` / scheduler / tick-assembler — S6 (CE-N-C-6). S5
  defines the type-level gate; S6 builds the queue that consumes
  `AcceptedBlock`.
- Cross-impl adapter + live-evidence binary — S7.
- Flipping `T-DET-01.strengthened_in` / `T-ENC-01.strengthened_in` —
  cluster-close.
- Resolving the OP-OPS-04 schema gap — cluster-close.
- Adding a new validator entry-point. `block_validity` is the existing
  closed authority; S5 wraps it without modification.
- Promoting `BlockValidityError` to a new public surface. It's already
  pub; S5 surfaces it verbatim through `SelfAcceptError::Rejected`.

---

## Failure Modes

`SelfAcceptError::Rejected(BlockValidityError)` is the single failure
shape. Carries the existing closed structured validator error verbatim
— no information loss, no information added. Failures are
deterministic and fail-fast: a producer whose forged bytes don't
self-accept halts cleanly; RED cannot retry the same artifacts.

This slice introduces no consensus-affecting failure mode. The
validator path is unchanged; S5 only wraps it.

---

## Grounding (verified at HEAD `4fd714c`)

- `ade_ledger::block_validity::transition::block_validity` at
  `crates/ade_ledger/src/block_validity/transition.rs:43` with
  signature
  `fn(&LedgerState, &PraosChainDepState, &EraSchedule, &dyn LedgerView, &[u8]) -> BlockValidityOutcome`.
- `ade_ledger::block_validity::verdict::BlockValidityError` exists
  as a closed sum (used by `block_validity` for the `Invalid`
  variant).
- `ade_core::consensus::header_validate::validate_and_apply_header` at
  `crates/ade_core/src/consensus/header_validate.rs:71` is called by
  `block_validity` internally; S5's wrapper does NOT call it directly
  (delegation to `block_validity` is the rule).
- S4's `ade_ledger::block_body_hash::block_body_hash` is the body-hash
  authority `block_validity` uses for the body-hash bind step
  (`crates/ade_ledger/src/block_validity/header_input.rs` switched in
  S4).
- S3's `ade_ledger::producer::forge::ForgedBlock { bytes, block }` is
  the natural producer of forged bytes that S5 consumes. The wrapper
  is type-level: `ForgedBlock.bytes -> &[u8] -> self_accept(...) ->
  AcceptedBlock` is the only producer-side path to broadcast.
- The S3 replay fixtures
  (`ade_testkit::producer::fixtures::fixture_empty_mempool_leader`,
  etc.) already build the matching context values. S5's positive test
  reuses these via a thin dev-dep edge if `ade_ledger` `#[cfg(test)]`
  can depend on `ade_testkit` without cycles. If a cycle blocks this,
  the implementer pins a minimal happy-path fixture in
  `self_accept.rs` directly — surface the deviation if so.

---

## Notes on the type-level gate

The strength of the gate is the **module-private field +
no-foreign-constructor** property of `AcceptedBlock`. Once that is
in place:

- RED `broadcast` (S6) will have a signature like
  `fn broadcast(&self, block: AcceptedBlock) -> ...`.
- The compiler refuses any RED code that tries to build an
  `AcceptedBlock` from raw bytes — the only path is
  `self_accept(...) -> Result<AcceptedBlock, ...>`.
- A reviewer can verify this property by reading one file
  (`self_accept.rs`) and one CI gate (`ci_check_self_accept_gate.sh`).

S6's slice will pin the `broadcast` signature to consume
`AcceptedBlock`, making the gate end-to-end. S5 alone establishes the
type and the constructor discipline; S6 wires it to the network
hand-off.
