# S-B3 — VRF cert verification wiring + Praos VRF input + leader threshold

## Slice Header

**Slice Name**: VRF cert verification wiring + Praos VRF input + leader threshold
**Cluster**: PHASE4-N-B
**Status**: In Progress
**Cluster Exit Criteria Addressed**: substrate for **CE-N-B-4** (leader schedule, S-B6) and **CE-N-B-1** (fork choice, S-B8); no CE closes here.

**Slice Dependencies**: S-B1 (`EraSchedule` not directly needed but `errors` module is), S-B2 (`Nonce`, `VrfCertError`, error closed taxonomy).

---

## 3. Implementation Instruction (AI)

Implement exactly what is specified. Do not pre-implement leader
schedule (S-B6) or nonce evolution (S-B4). This slice gives later
slices a typed VRF transition + a deterministic, float-free threshold
check.

Commit with the `Co-Authored-By: Claude <model+context>
<noreply@anthropic.com>` trailer per CLAUDE.md.

---

## 4. Intent

Make it impossible for any later consensus path to:
1. Reach for `verify_vrf` from `ade_crypto` without going through a
   typed `VRFCertVerify` transition that requires the canonical
   `(slot, epoch_nonce)` alpha input.
2. Compute a leader-eligibility check using floating-point. The
   stake-fraction × active-slots-coefficient threshold must be done
   in **integer arithmetic** byte-identically to ouroboros-consensus.

---

## 5. Scope

**Modules / crates**:
- `crates/ade_core/src/consensus/vrf_cert.rs` (NEW)
- `crates/ade_core/src/consensus/mod.rs` (extend — re-exports)
- `crates/ade_core/tests/vrf_cert_threshold.rs` (NEW integration test)
- `crates/ade_core/Cargo.toml` (add `ade_crypto` as a dep if not
  already present — verify; minimal change)

**State machines affected**: introduces `VRFCertVerify` pure
transition. No persistent state.

**Persistence impact**: none.

**Network-visible impact**: none.

**Out-of-scope**:
- Nonce evolution / candidate-to-epoch promotion (S-B4)
- Op-cert counters (S-B5)
- Leader schedule per pool (S-B6)
- Stake distribution / `LedgerView` interaction (S-B6)
- KES key handling — Ade verifies headers, doesn't sign them in this
  slice (block production is S-N-C; out of N-B entirely)

---

## 6. Execution Boundary

**BLUE**: `ade_core::consensus::vrf_cert`.
**GREEN**: none.
**RED**: none.

---

## 7. Invariants Preserved

- All previous tests still pass.
- No new dep on `ade_runtime`.
- BLUE crate-level lints preserved (`deny(clippy::float_arithmetic)`
  is the critical one here).

---

## 8. Invariants Strengthened or Introduced

- **`DC-CRYPTO-01` strengthened**: VRF verification used by N-B is
  wrapped in a typed transition that fails fast on any decoding
  inconsistency.
- **`CN-CRYPTO-02` strengthened**: the (slot, epoch-nonce) VRF input
  is now formed by a single, audited function — no caller can pass
  the wrong alpha.
- **`T-CORE-02`** (no float in BLUE): threshold check is integer-only
  and proven equivalent to the reference rational formula by a
  property test over known vectors.

---

## 9. Design Summary

### VRF input

Praos uses two VRF outputs per header (nonce contribution and leader
value); both compute over the same family of alphas: `(slot ‖
epoch_nonce ‖ tag)` where `tag` is a 1-byte discriminator
(`NONCE_TAG = 0x4e`, `LEADER_TAG = 0x4c`).

```rust
// vrf_cert.rs

use ade_types::SlotNo;
use crate::consensus::praos_state::Nonce;

/// Tag byte distinguishing nonce VRF input from leader VRF input.
/// 'N' = 0x4E, 'L' = 0x4C — matches cardano-node convention.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VrfRole {
    NonceContribution,
    LeaderEligibility,
}

impl VrfRole {
    pub const fn tag_byte(self) -> u8 {
        match self {
            VrfRole::NonceContribution => 0x4E,
            VrfRole::LeaderEligibility => 0x4C,
        }
    }
}

/// VRF input alpha = big-endian slot (8 bytes) ‖ epoch_nonce (32 bytes) ‖ tag (1 byte) = 41 bytes.
pub fn vrf_input(slot: SlotNo, epoch_nonce: &Nonce, role: VrfRole) -> [u8; 41];
```

### Typed transition

```rust
use ade_crypto::vrf::{VrfOutput, VrfProof, VrfVerificationKey};
use crate::consensus::errors::VrfCertError;

/// One verified VRF cert for a header. After this transition succeeds,
/// the caller may use the contained VrfOutput to (a) contribute to the
/// next-evolving nonce (S-B4) and (b) decide leader eligibility (this
/// slice's threshold check). Two outputs per header => caller invokes
/// this twice with different VrfRole values.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VerifiedVrf {
    pub role:        VrfRole,
    pub slot:        SlotNo,
    pub output:      VrfOutput,
}

pub fn verify_vrf_cert(
    vk:          &VrfVerificationKey,
    proof:       &VrfProof,
    slot:        SlotNo,
    epoch_nonce: &Nonce,
    role:        VrfRole,
) -> Result<VerifiedVrf, VrfCertError>;
```

Implementation:
1. Build alpha via `vrf_input`.
2. Call `ade_crypto::vrf::verify_vrf`.
3. Map `CryptoError` to `VrfCertError`:
   - `MalformedKey` → `VrfCertError::MalformedKey`
   - `MalformedProof` → `VrfCertError::MalformedProof`
   - `VerificationFailed` → `VrfCertError::VerificationFailed`
   - any other variant → `VrfCertError::VerificationFailed` (closed
     mapping; never panic)

### Leader-eligibility threshold (integer arithmetic, float-free)

Praos leader eligibility: a pool with stake fraction σ (relative to
total active stake) leads slot S iff

```
leader_value(slot, vrf_output) / 2^512 < 1 − (1 − f)^σ
```

where `f` is the active-slots-coefficient (typically 1/20).

In ouroboros-consensus this is implemented in integer arithmetic via
the `taylorExpCmp` routine — a fixed-precision rational comparison
that is byte-identical across runtimes. We mirror the same approach
in Rust using `num-bigint` (workspace dep — verify; if not present
**do not add a new dep**; instead, use the existing rational arithmetic
already used in ledger reward computation — see
`crates/ade_ledger/src/rational.rs` which is the project's chosen
integer-rational primitive).

```rust
/// Convert a 64-byte VRF output into a 64-byte big-endian integer
/// numerator over the implicit denominator 2^512. Returns the first
/// 8 bytes for the bound-comparison helper (the comparison only ever
/// needs the high 8 bytes, the rest are fed lazily — but we expose
/// both forms).
pub fn leader_value_bytes(output: &VrfOutput) -> [u8; 64];

/// Stake fraction = (active_stake_for_pool, total_active_stake).
/// Both u64 satoshi-style integers (lovelace). Caller guarantees
/// total_active_stake > 0.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StakeFraction {
    pub numer: u64,
    pub denom: u64,
}

/// Active-slots-coefficient. Stored as numer / denom (e.g. 1/20).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ActiveSlotsCoeff {
    pub numer: u32,
    pub denom: u32,
}

/// Integer-arithmetic leader-eligibility check.
///
/// Returns Ok(true) iff the pool leads slot S given `output`.
/// Returns Ok(false) iff the pool does not lead.
/// Returns Err(VrfCertError::LeaderValueAboveThreshold {..}) is NOT
/// used here — that error is for the inverse interpretation (a
/// purported-leader header whose VRF value is above its threshold).
///
/// Determinism: byte-identical across runs; no f64/f32; no nondeterministic
/// rational reduction step.
pub fn is_leader(
    output:    &VrfOutput,
    sigma:     StakeFraction,
    asc:       ActiveSlotsCoeff,
) -> bool;

/// Same as `is_leader`, but reframed for header validation: returns
/// an error when a header *claims* leadership but the VRF value is
/// above its threshold. Returns Ok(()) when the leader claim is valid.
pub fn check_leader_claim(
    output:    &VrfOutput,
    sigma:     StakeFraction,
    asc:       ActiveSlotsCoeff,
) -> Result<(), VrfCertError>;
```

The integer-arithmetic comparison uses fixed-precision rational
truncation matching ouroboros-consensus `taylorExpCmp` to 18
expansion terms. (18 terms is the Cardano convention.) The
implementation lives in `vrf_cert.rs` as a private helper
`taylor_exp_cmp_le(numer, denom, x_numer, x_denom, terms)` returning
`bool` ("`numer/denom ≤ 1 − (1 − x)^terms`").

If the project does not already have a rational primitive suitable
(`ade_ledger::rational` may be reward-computation specific), implement
the helper using only `u128` and `u256`-style manual carry math. Do
NOT introduce `num-bigint` if it is not already a transitive dep.

### Closed mapping

`VrfCertError::LeaderValueAboveThreshold { value: [u8;8], threshold:
[u8;8] }` (from S-B2) is the typed reject when `check_leader_claim`
fails. The `value` and `threshold` fields are 8-byte big-endian
truncations of the relevant comparand and bound — enough to make the
error deterministic and debuggable without leaking the full 64-byte
output.

---

## 10. Changes Introduced

### Types
- New: `VrfRole`, `VerifiedVrf`, `StakeFraction`, `ActiveSlotsCoeff`.

### State Transitions
- New: `verify_vrf_cert`, `is_leader`, `check_leader_claim`.

### Persistence
- None.

### Removal / Refactors
- None.

---

## 11. Replay, Crash, and Epoch Validation

### Tests

- `crates/ade_core/tests/vrf_cert_threshold.rs`:
  - `vrf_input_layout_is_41_bytes_with_correct_tag` — assert layout
    of `vrf_input(slot, nonce, role)` byte-for-byte.
  - `verify_vrf_cert_accepts_valid_proof` — generate via
    `cardano_crypto::vrf::VrfDraft03`, verify, assert `VerifiedVrf
    { role: NonceContribution, slot, output }`.
  - `verify_vrf_cert_rejects_wrong_alpha` — same key, different slot
    → `VrfCertError::VerificationFailed`.
  - `verify_vrf_cert_rejects_malformed_proof` —
    `MalformedProof` mapping.
  - `is_leader_zero_stake_never_leads` — `σ = 0` → false always.
  - `is_leader_full_stake_always_leads` — `σ = 1` → true always.
  - `is_leader_determinism` — same inputs twice → same answer.
  - `is_leader_known_vector_matches_reference` — at least one
    known-vector pulled from cardano-node's test corpus (the test
    file embeds the vector with `(vrf_output_hex, sigma_num,
    sigma_den, asc_num, asc_den, expected_bool)`). If pinning a real
    cardano-node vector is infeasible without a live node, the test
    embeds a synthetic vector and notes in the docstring that S-B10's
    live-interop pass pins a real one.
  - `check_leader_claim_returns_typed_error_on_above_threshold` —
    constructed vector where the comparison fails.

- Unit tests in `vrf_cert.rs`:
  - `vrf_role_tags_match_convention` — `NonceContribution.tag_byte()
    == 0x4E`, `LeaderEligibility.tag_byte() == 0x4C`.
  - `vrf_input_byte_layout` — concrete byte vector.
  - `taylor_exp_cmp_le_zero_x_returns_false` — `x = 0` → comparison
    becomes `numer/denom ≤ 0`; only true if `numer == 0`.
  - `taylor_exp_cmp_le_x_equals_one_returns_true` — `x = 1` → `1 − 0
    = 1`; comparison is `numer/denom ≤ 1`, true unless `numer >
    denom`.
  - `taylor_exp_cmp_le_monotone_in_x` — for fixed bound, increasing
    x makes the comparison more likely to be true.

### Replay impact
- `verify_vrf_cert` is a pure function — replay-equivalent.
- `is_leader` is a pure function — replay-equivalent.

---

## 12. Mechanical Acceptance Criteria

- [ ] `cargo build -p ade_core` PASS
- [ ] `cargo test -p ade_core --lib consensus::vrf_cert` PASS
- [ ] `cargo test -p ade_core --test vrf_cert_threshold` PASS
- [ ] `cargo clippy -p ade_core --all-targets -- -D warnings` PASS
- [ ] No `f32` / `f64` anywhere in `vrf_cert.rs` (grep — already
      enforced by crate-level lint, but explicitly grepped)
- [ ] No `String` in any new public type
- [ ] No `unwrap` / `expect` / `panic` in production code (crate-level
      lint already enforces; verify the new file)
- [ ] `ci/ci_check_no_float_in_consensus.sh` still PASS

---

## 13. Failure Modes

| Failure | Shape | Fail-fast? |
|---|---|---|
| Malformed VRF key | `VrfCertError::MalformedKey` | yes |
| Malformed VRF proof | `VrfCertError::MalformedProof` | yes |
| VRF verification failed | `VrfCertError::VerificationFailed` | yes |
| Leader value above threshold | `VrfCertError::LeaderValueAboveThreshold { value, threshold }` | yes |

All BLUE-side mapping; no panic; no string.

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
- No `unsafe`.
- No `num-bigint` / `num-rational` dep — if not already transitively
  present, manual `u128` (and `u256` via `(u128, u128)` pairs)
  arithmetic only.
- No "stub leader threshold" — the comparison must match the
  Cardano-convention 18-term Taylor expansion (or be proven
  equivalent by the known-vector test).
- No exposing the full 64-byte VRF output in error variants — keep
  to 8-byte truncations for `LeaderValueAboveThreshold`.

---

## 15. Explicit Non-Goals

- Do NOT evolve any nonce (S-B4).
- Do NOT check op-cert counters (S-B5).
- Do NOT query a stake distribution (S-B6).
- Do NOT produce a `ValidatedHeaderSummary` (S-B7).
- Do NOT implement KES signing or rotation (out of N-B entirely).

---

## 16. Completion Checklist

- [ ] All transitions are pure functions
- [ ] All failure modes are deterministic typed enums
- [ ] No TODOs in BLUE
- [ ] Threshold-check is integer-only (no float anywhere on the
      authority path; the existing `ci_check_no_float_in_consensus.sh`
      still passes after the new file lands)
- [ ] Known-vector test for `is_leader` is committed

---

## 17. Review Notes

- The Taylor-expansion approach matches ouroboros-consensus.
  ouroboros-consensus uses 18 terms by convention; this is the same
  number cardano-node uses for mainnet.
- The integer-arithmetic comparison must be auditable against the
  Haskell reference. A pinned known-vector matters more than a clean
  abstraction.

---

## 18. Authority Reminder

Correctness rules live in `docs/ade-invariant-registry.toml`. If
this doc conflicts with the registry, the registry wins.
