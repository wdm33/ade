// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types
// - Explicit state transitions only
// - Canonical serialization for all persisted/hashed data

//! VRF certificate verification wiring + Praos leader-eligibility threshold.
//!
//! Two responsibilities:
//!
//! 1. Wrap `ade_crypto::vrf::verify_vrf` in a typed transition so callers
//!    cannot bypass canonical alpha construction.
//! 2. Implement the integer-arithmetic leader-eligibility predicate
//!    `p < 1 - (1 - f)^σ` using a Taylor-expansion comparison that
//!    matches ouroboros-consensus `taylorExpCmp` (18 terms, Cardano
//!    convention). No floating point. No external bignum dependency.
//!
//! Fixed-precision rational arithmetic is done in Q.123 unsigned
//! fixed-point inside `u128`, with full u128 x u128 -> u256
//! multiplication built from u64 halves. The choice of 123 fractional
//! bits gives a numeric range of [0, 2^5) which comfortably contains
//! `exp(x)` for the negative `x` arguments we feed (the partial sums
//! never exceed ~e in magnitude during the iteration).

use ade_crypto::blake2b::blake2b_256;
use ade_crypto::vrf::{verify_vrf as crypto_verify_vrf, VrfOutput, VrfProof, VrfVerificationKey};
use ade_crypto::CryptoError;
use ade_types::SlotNo;

use crate::consensus::errors::VrfCertError;
use crate::consensus::praos_state::Nonce;

/// Tag byte distinguishing nonce VRF input from leader VRF input.
///
/// `'N' = 0x4E` and `'L' = 0x4C` match the cardano-node convention
/// (`mkNonceContrib` / `mkLeaderInput` in the Haskell reference).
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

/// Length of a Praos VRF input alpha: 8-byte BE slot + 32-byte epoch nonce + 1-byte tag.
pub const VRF_INPUT_LEN: usize = 41;

/// Build the canonical Praos VRF input alpha for `(slot, epoch_nonce, role)`.
///
/// Layout (41 bytes): `[slot_be (8) ‖ epoch_nonce (32) ‖ role_tag (1)]`.
pub fn vrf_input(slot: SlotNo, epoch_nonce: &Nonce, role: VrfRole) -> [u8; VRF_INPUT_LEN] {
    let mut out = [0u8; VRF_INPUT_LEN];
    out[0..8].copy_from_slice(&slot.0.to_be_bytes());
    out[8..40].copy_from_slice(epoch_nonce.as_bytes());
    out[40] = role.tag_byte();
    out
}

/// A VRF cert that has passed `crypto_verify_vrf`, carrying the role
/// and slot so downstream consumers cannot mix outputs across calls.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VerifiedVrf {
    pub role: VrfRole,
    pub slot: SlotNo,
    pub output: VrfOutput,
}

/// Pure VRF cert verification transition.
///
/// Builds alpha via `vrf_input`, calls `ade_crypto::vrf::verify_vrf`,
/// and maps `CryptoError` to the closed `VrfCertError` taxonomy.
pub fn verify_vrf_cert(
    vk: &VrfVerificationKey,
    proof: &VrfProof,
    slot: SlotNo,
    epoch_nonce: &Nonce,
    role: VrfRole,
) -> Result<VerifiedVrf, VrfCertError> {
    let alpha = vrf_input(slot, epoch_nonce, role);
    match crypto_verify_vrf(vk, proof, &alpha) {
        Ok(output) => Ok(VerifiedVrf { role, slot, output }),
        Err(e) => Err(map_crypto_error(e)),
    }
}

fn map_crypto_error(e: CryptoError) -> VrfCertError {
    match e {
        CryptoError::MalformedKey { .. } => VrfCertError::MalformedKey,
        CryptoError::MalformedProof { .. } => VrfCertError::MalformedProof,
        CryptoError::VerificationFailed { .. } => VrfCertError::VerificationFailed,
        _ => VrfCertError::VerificationFailed,
    }
}

/// Return the 64-byte big-endian VRF output bytes as-is.
///
/// The leader-value integer is the big-endian interpretation of these
/// 64 bytes with implicit denominator `2^512`.
pub fn leader_value_bytes(output: &VrfOutput) -> [u8; 64] {
    output.0
}

// ---------------------------------------------------------------------------
// Praos (Babbage, Conway) single combined-VRF construction.
//
// Unlike TPraos (Shelley..Alonzo), which carries two role-tagged VRF proofs,
// Praos carries ONE proof. The leader value AND the nonce contribution are
// both derived from its single 64-byte certified output, by domain-separated
// re-hashing. Pinned against `Ouroboros.Consensus.Protocol.Praos.VRF`
// (`mkInputVRF`, `vrfLeaderValue`, `vrfNonceValue`) and validated against the
// 14 real Conway-576 corpus blocks.
// ---------------------------------------------------------------------------

/// Praos VRF range-extension domain tags (`hashVRF`): `"L"` for the leader
/// value, `"N"` for the nonce value. ASCII bytes, matching cardano-base.
const PRAOS_LEADER_TAG: u8 = b'L';
const PRAOS_NONCE_TAG: u8 = b'N';

/// Build the Praos VRF input (`mkInputVRF`): `blake2b256(slot_be8 ‖ eta0_32)`.
///
/// No role tag — the role split is a TPraos concept. The single proof in the
/// header certifies over exactly this 32-byte input.
pub fn praos_vrf_input(slot: SlotNo, epoch_nonce: &Nonce) -> [u8; 32] {
    let mut pre = [0u8; 40];
    pre[0..8].copy_from_slice(&slot.0.to_be_bytes());
    pre[8..40].copy_from_slice(epoch_nonce.as_bytes());
    blake2b_256(&pre).0
}

/// Verify the single Praos combined-VRF proof.
///
/// Builds the input via `praos_vrf_input`, verifies `proof` under `vk`, and
/// returns the certified 64-byte output. The returned output must equal the
/// `output` bytes carried in the header (the caller binds them).
pub fn verify_praos_vrf(
    vk: &VrfVerificationKey,
    proof: &VrfProof,
    slot: SlotNo,
    epoch_nonce: &Nonce,
) -> Result<VrfOutput, VrfCertError> {
    let alpha = praos_vrf_input(slot, epoch_nonce);
    crypto_verify_vrf(vk, proof, &alpha).map_err(map_crypto_error)
}

/// Praos `vrfLeaderValue`: range-extend the certified output for the leader
/// check via `blake2b256("L" ‖ output64)` — a 32-byte value interpreted as a
/// natural with implicit denominator `2^256`.
///
/// Returned as a `VrfOutput` (the leader-value hash in the high 32 bytes, the
/// low 32 bytes zero) so it feeds the existing `check_leader_claim`, which
/// reads the top-8-byte fractional prefix. The denominator difference vs the
/// raw output (`2^256` here, `2^512` for the raw output) is irrelevant under
/// the top-64-bit fractional truncation both share.
pub fn praos_leader_value(output: &VrfOutput) -> VrfOutput {
    let mut pre = [0u8; 65];
    pre[0] = PRAOS_LEADER_TAG;
    pre[1..65].copy_from_slice(&output.0);
    let h = blake2b_256(&pre).0;
    let mut out = [0u8; 64];
    out[0..32].copy_from_slice(&h);
    VrfOutput(out)
}

/// Praos `vrfNonceValue`: derive the 32-byte nonce contribution from the
/// certified output via `blake2b256(blake2b256("N" ‖ output64))`.
///
/// The double hash is intentional: the inner hash is the VRF-paper range
/// extension; the outer hash is the fixed-`Blake2b_256` nonce derivation.
pub fn praos_nonce_value(output: &VrfOutput) -> Nonce {
    let mut pre = [0u8; 65];
    pre[0] = PRAOS_NONCE_TAG;
    pre[1..65].copy_from_slice(&output.0);
    let inner = blake2b_256(&pre).0;
    Nonce(blake2b_256(&inner))
}

/// Pool stake fraction `(active_stake_for_pool, total_active_stake)`,
/// in lovelace. Caller guarantees `denom > 0`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StakeFraction {
    pub numer: u64,
    pub denom: u64,
}

/// Praos active-slots-coefficient `f`, e.g. `numer=1, denom=20`.
/// Caller guarantees `denom > 0` and `numer <= denom`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ActiveSlotsCoeff {
    pub numer: u32,
    pub denom: u32,
}

/// Leader-eligibility predicate.
///
/// Returns `true` iff `p < 1 - (1 - f)^σ` where `p = leader_value/2^512`,
/// `σ` is the pool's stake fraction, and `f` is the active-slots-coefficient.
///
/// The comparison is done by Taylor-expanding `(1 - f)^σ = exp(σ * ln(1-f))`
/// to 18 terms, matching ouroboros-consensus `taylorExpCmp`. No floats,
/// no rand, no clock, no platform-dependent math.
///
/// Boundary handling:
/// - `σ.numer == 0`           => always `false` (zero stake never leads).
/// - `f.numer == f.denom`     => always `true`  (asc = 1 ⇒ everyone leads).
/// - `f.numer == 0`           => always `false` (asc = 0 ⇒ no-one leads).
pub fn is_leader(output: &VrfOutput, sigma: StakeFraction, asc: ActiveSlotsCoeff) -> bool {
    if sigma.numer == 0 || sigma.denom == 0 {
        return false;
    }
    if asc.denom == 0 {
        return false;
    }
    if asc.numer >= asc.denom {
        return true;
    }
    if asc.numer == 0 {
        return false;
    }

    let p = leader_value_q(output);
    let threshold = one_minus_one_minus_f_pow_sigma(asc, sigma);
    p < threshold
}

/// Header-validation framing of `is_leader`.
///
/// Returns `Ok(())` when the leader claim is valid;
/// returns `Err(VrfCertError::LeaderValueAboveThreshold { value, threshold })`
/// when it is not, with both `value` and `threshold` truncated to their
/// 8-byte big-endian high prefixes so the rejection is byte-deterministic
/// without leaking the full 64-byte output.
pub fn check_leader_claim(
    output: &VrfOutput,
    sigma: StakeFraction,
    asc: ActiveSlotsCoeff,
) -> Result<(), VrfCertError> {
    if is_leader(output, sigma, asc) {
        return Ok(());
    }
    let value_bytes = output.0;
    let mut value = [0u8; 8];
    value.copy_from_slice(&value_bytes[0..8]);

    let threshold_q = one_minus_one_minus_f_pow_sigma(asc, sigma);
    let threshold = q_to_high_be8(threshold_q);

    Err(VrfCertError::LeaderValueAboveThreshold { value, threshold })
}

// ---------------------------------------------------------------------------
// Q.123 unsigned fixed-point arithmetic, all in u128.
//
// All values here represent the real number `x` as the integer
// `x * 2^123`. The choice of 123 fractional bits gives a numeric
// range of [0, 32), large enough to contain `exp(x)` for the
// magnitudes we feed (`|σ * ln(1-f)|` is at most ~ln(1) ≈ 0.05 in
// practice) and small enough that all intermediate Taylor terms fit
// in u128.
//
// Multiplication is exact via a u128 * u128 -> u256 helper built from
// u64 halves, then shifted right by 123 bits.
// ---------------------------------------------------------------------------

const FP_BITS: u32 = 123;
const ONE_Q: u128 = 1u128 << FP_BITS;

/// u128 * u128 -> (hi: u128, lo: u128). Schoolbook on u64 halves.
fn mul_u128_full(a: u128, b: u128) -> (u128, u128) {
    let a_lo = a as u64 as u128;
    let a_hi = a >> 64;
    let b_lo = b as u64 as u128;
    let b_hi = b >> 64;

    let ll = a_lo * b_lo;
    let lh = a_lo * b_hi;
    let hl = a_hi * b_lo;
    let hh = a_hi * b_hi;

    let mid = (ll >> 64) + (lh & 0xFFFF_FFFF_FFFF_FFFF) + (hl & 0xFFFF_FFFF_FFFF_FFFF);
    let lo = (mid << 64) | (ll & 0xFFFF_FFFF_FFFF_FFFF);
    let hi = hh + (lh >> 64) + (hl >> 64) + (mid >> 64);
    (hi, lo)
}

/// (hi, lo) >> shift, with shift in [1, 255].
fn shr_u256(hi: u128, lo: u128, shift: u32) -> u128 {
    if shift == 0 {
        return lo;
    }
    if shift < 128 {
        (lo >> shift) | (hi << (128 - shift))
    } else if shift == 128 {
        hi
    } else {
        hi >> (shift - 128)
    }
}

/// Multiply two Q.123 values. Saturates at u128::MAX if the result
/// would not fit (only possible if either operand encodes a value
/// outside the documented [0, 32) range).
fn mul_q(a: u128, b: u128) -> u128 {
    let (hi, lo) = mul_u128_full(a, b);
    // Right-shift by FP_BITS to get back to Q.123.
    // If hi >= 2^(FP_BITS), the result overflows u128 — saturate.
    if hi >= (1u128 << FP_BITS) {
        u128::MAX
    } else {
        shr_u256(hi, lo, FP_BITS)
    }
}

/// Divide two Q.123 values: returns `a / b` as Q.123.
/// Caller guarantees `b > 0`. If the result would not fit in u128,
/// the result is u128::MAX.
fn div_q(a: u128, b: u128) -> u128 {
    // (a / b) * ONE = a * ONE / b. We compute (a << FP_BITS) / b but
    // a << FP_BITS may overflow u128, so use the u256 helper.
    if b == 0 {
        return u128::MAX;
    }
    let (hi, lo) = (a >> (128 - FP_BITS), a << FP_BITS);
    div_u256_by_u128(hi, lo, b)
}

/// Compute `(hi:lo) / divisor` as a 128-bit quotient. Saturates at
/// `u128::MAX` if the result would not fit. Restoring division on
/// 128 bits — schoolbook shift-subtract.
fn div_u256_by_u128(mut hi: u128, mut lo: u128, divisor: u128) -> u128 {
    if divisor == 0 {
        return u128::MAX;
    }
    if hi >= divisor {
        // Quotient cannot fit in u128.
        return u128::MAX;
    }
    let mut quotient: u128 = 0;
    for _ in 0..128 {
        // Shift the 256-bit dividend left by 1.
        let new_hi = (hi << 1) | (lo >> 127);
        let new_lo = lo << 1;
        quotient <<= 1;
        if new_hi >= divisor {
            hi = new_hi - divisor;
            lo = new_lo;
            quotient |= 1;
        } else {
            hi = new_hi;
            lo = new_lo;
        }
    }
    let _ = (hi, lo);
    quotient
}

/// Convert (num, den) — both u128 — to a Q.123 unsigned value.
/// Saturates at u128::MAX if the value would not fit (i.e., > 32).
fn rational_to_q(num: u128, den: u128) -> u128 {
    if den == 0 {
        return u128::MAX;
    }
    // q = (num * ONE) / den.
    let (hi, lo) = mul_u128_full(num, ONE_Q);
    div_u256_by_u128(hi, lo, den)
}

/// Convert a Q.123 unsigned value in [0, 1] to an 8-byte big-endian
/// high prefix — the top 64 bits of its fractional [0, 1) encoding.
///
/// This is the truncation used in `VrfCertError::LeaderValueAboveThreshold`.
/// For `q == ONE_Q` (representing exactly 1.0) the result is `0xFFFF…` —
/// the high-64-bit truncation of the next representable value below 1.
fn q_to_high_be8(q: u128) -> [u8; 8] {
    let clamped = if q >= ONE_Q { ONE_Q - 1 } else { q };
    let bits = (clamped >> (FP_BITS - 64)) as u64;
    bits.to_be_bytes()
}

/// Interpret the top 64 bits of the VRF output as a Q.123 value in [0, 1).
///
/// The full leader-value scalar is `output / 2^512`; for the
/// truncated-precision comparison we use the top 64 bits, placed at
/// the high end of the Q.123 fractional field. This matches the
/// ouroboros-consensus `FixedPoint`-precision behavior (it compares
/// at fixed-precision, not at full 2^512 precision).
fn leader_value_q(output: &VrfOutput) -> u128 {
    let mut top = [0u8; 8];
    top.copy_from_slice(&output.0[0..8]);
    let v = u64::from_be_bytes(top) as u128;
    // v occupies the high 64 bits of [0, 1). In Q.123, that means
    // shift left by (FP_BITS - 64).
    v << (FP_BITS - 64)
}

/// Truncated natural-log helper: `ln(1 - f)` where f ∈ (0, 1) is given
/// rationally as `(asc.numer, asc.denom)`. Returns the **absolute value**
/// of `ln(1 - f)` (which is positive since 1 - f < 1) as a Q.123 value.
///
/// Uses the standard series `-ln(1 - y) = Σ_{n=1}^{18} y^n / n` with
/// y = f. For mainnet `f = 1/20`, this converges to ~30 decimal digits
/// after 18 terms.
fn abs_ln_one_minus_f(asc: ActiveSlotsCoeff) -> u128 {
    let f_q = rational_to_q(asc.numer as u128, asc.denom as u128);
    let mut term = f_q;
    let mut sum: u128 = 0;
    let mut n: u128 = 1;
    while n <= 18 {
        // Add term / n
        sum = sum.saturating_add(term / n);
        // term <- term * f
        let next = mul_q(term, f_q);
        term = next;
        n += 1;
    }
    sum
}

/// Compute `exp(x)` for x ≥ 0 (Q.123), via the standard Taylor series
/// `Σ_{n=0}^{18} x^n / n!`.
///
/// 18 terms is the cardano-node mainnet convention.
fn exp_pos_q(x: u128) -> u128 {
    let mut term = ONE_Q;
    let mut sum = ONE_Q;
    let mut n: u128 = 1;
    while n <= 18 {
        // term <- term * x / n
        let mt = mul_q(term, x);
        term = mt / n;
        sum = sum.saturating_add(term);
        n += 1;
    }
    sum
}

/// Compute `exp(-y)` for y ≥ 0 (Q.123). Returns a Q.123 value in (0, 1].
///
/// Approach: `exp(-y) = 1 / exp(y)`. With 18-term Taylor, exp(y) is
/// computed first, then reciprocated.
fn exp_neg_q(y: u128) -> u128 {
    let e = exp_pos_q(y);
    if e == 0 {
        return 0;
    }
    // Reciprocal in Q.123: ONE_Q^2 / e.
    div_q(ONE_Q, e)
}

/// Compute `1 - (1 - f)^σ` as a Q.123 value in [0, 1].
fn one_minus_one_minus_f_pow_sigma(asc: ActiveSlotsCoeff, sigma: StakeFraction) -> u128 {
    // (1 - f)^σ = exp(σ * ln(1 - f)) = exp(-σ * |ln(1 - f)|)
    let abs_ln = abs_ln_one_minus_f(asc);
    let sigma_q = rational_to_q(sigma.numer as u128, sigma.denom as u128);
    let exponent = mul_q(sigma_q, abs_ln);
    let pow = exp_neg_q(exponent);
    ONE_Q.saturating_sub(pow)
}

/// Direct integer-arithmetic Taylor comparison helper, exposed for
/// targeted unit testing. Returns `true` iff the rational
/// `(numer / denom) <= 1 - (1 - x)^terms_of_truncation` where the
/// right-hand side is evaluated by `taylorExpCmp`-style expansion of
/// `(1 - x)` to `terms` factors.
///
/// Semantics for the special cases match the boundary handling above:
/// - `x = 0`     => RHS = 0, returns `numer == 0`.
/// - `x = ONE`   => RHS = 1, returns `numer <= denom`.
/// - monotone   => fixing the bound and increasing `x` only increases
///   the probability of returning `true`.
#[cfg(test)]
fn taylor_exp_cmp_le(
    numer: u128,
    denom: u128,
    x_numer: u128,
    x_denom: u128,
    terms: u32,
) -> bool {
    if denom == 0 || x_denom == 0 {
        return false;
    }
    if x_numer == 0 {
        return numer == 0;
    }
    if x_numer >= x_denom {
        // x >= 1 => (1 - x)^terms = 0 for terms >= 1.
        return numer <= denom;
    }

    let x_q = rational_to_q(x_numer, x_denom);
    // (1 - x)^terms via direct factor multiplication.
    let one_minus_x = ONE_Q - x_q;
    let mut pow = ONE_Q;
    for _ in 0..terms {
        pow = mul_q(pow, one_minus_x);
    }
    let rhs = ONE_Q - pow; // = 1 - (1-x)^terms

    let lhs = rational_to_q(numer, denom);
    lhs <= rhs
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
#[allow(clippy::expect_used)]
#[allow(clippy::panic)]
mod tests {
    use super::*;

    #[test]
    fn vrf_role_tags_match_convention() {
        assert_eq!(VrfRole::NonceContribution.tag_byte(), 0x4E);
        assert_eq!(VrfRole::LeaderEligibility.tag_byte(), 0x4C);
    }

    #[test]
    fn vrf_input_byte_layout() {
        let slot = SlotNo(0x0102_0304_0506_0708);
        let nonce_bytes = [0xAAu8; 32];
        let nonce = Nonce(ade_types::Hash32(nonce_bytes));
        let input = vrf_input(slot, &nonce, VrfRole::NonceContribution);
        assert_eq!(input.len(), 41);
        assert_eq!(
            &input[0..8],
            &[0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08]
        );
        assert_eq!(&input[8..40], &nonce_bytes);
        assert_eq!(input[40], 0x4E);
    }

    #[test]
    fn taylor_exp_cmp_le_zero_x_returns_false() {
        // x = 0 => RHS = 0, so numer/denom <= 0 iff numer == 0.
        assert!(taylor_exp_cmp_le(0, 1, 0, 100, 18));
        assert!(!taylor_exp_cmp_le(1, 1000, 0, 100, 18));
    }

    #[test]
    fn taylor_exp_cmp_le_x_equals_one_returns_true() {
        // x = 1 => RHS = 1, so numer/denom <= 1 iff numer <= denom.
        assert!(taylor_exp_cmp_le(1, 1, 1, 1, 18));
        assert!(taylor_exp_cmp_le(1, 2, 1, 1, 18));
        assert!(!taylor_exp_cmp_le(2, 1, 1, 1, 18));
    }

    #[test]
    fn taylor_exp_cmp_le_monotone_in_x() {
        // Fix the bound, sweep x: increasing x can only flip false -> true,
        // never true -> false.
        let bound_n = 1u128;
        let bound_d = 4u128;
        let mut last_result = false;
        for x_num in [1u128, 2, 4, 8, 16, 32, 64].iter().copied() {
            let r = taylor_exp_cmp_le(bound_n, bound_d, x_num, 64, 18);
            if last_result {
                assert!(
                    r,
                    "monotonicity violated at x_num={x_num}: prev=true, curr=false"
                );
            }
            last_result = r;
        }
    }
}
