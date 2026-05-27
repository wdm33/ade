// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types
// - Explicit state transitions only
// - Canonical serialization for all persisted/hashed data

//! Generic recursive `SumKes<D>` (PHASE4-N-P S2).
//!
//! Implements the Sum_n KES construction: each level is a binary tree
//! whose left and right sub-trees are each a `D`-typed KES. The
//! verification key at this level is `Blake2b256(vk_left || vk_right)`.
//!
//! Recurrence:
//! - `Sum_n::SEED_SIZE = D::SEED_SIZE = 32`
//! - `Sum_n::SIGNING_KEY_SIZE = D::SIGNING_KEY_SIZE + 96`
//!   (child sk + seed_right + vk_left + vk_right at this level)
//! - `Sum_n::SIGNATURE_SIZE = D::SIGNATURE_SIZE + 64`
//!   (sigma_d + vk_left + vk_right)
//! - `Sum_n::VERIFICATION_KEY_SIZE = 32` (Blake2b256 output)
//! - `Sum_n::total_periods() = 2 * D::total_periods()`
//!
//! Matches Haskell `cardano-base`'s `SumKES` and the upstream
//! `cardano-crypto` 1.0.8 `SumKes<D, H>` byte-for-byte. Cross-impl
//! agreement is validated under `#[cfg(test)]` (see `tests.rs`).

use core::marker::PhantomData;

use super::hash::{expand_seed, hash_concat_vk};
use super::KesAlgorithm;
use super::KesError;

// =========================================================================
// SumKes<D> — generic recursive Sum_n type
// =========================================================================

/// Sum_n KES — recursive binary tree built atop `D` (which is itself a
/// `KesAlgorithm`, recursively).
pub struct SumKes<D> {
    _phantom: PhantomData<D>,
}

/// Zeroizing wrapper around a 32-byte seed. Its `Drop` best-effort
/// overwrites the bytes; held inside `Option<ZeroizingSeed>` so
/// `Option::take` can consume it during `update_kes`.
///
/// We use a per-field wrapper instead of impl-Drop on
/// `SumSigningKey<D>` because the latter blocks destructuring
/// (Rust forbids partial moves out of `Drop` types). Each field
/// now self-zeroizes on drop, which is the equivalent runtime
/// guarantee without the type-system friction.
pub(super) struct ZeroizingSeed(pub(super) [u8; 32]);

impl Drop for ZeroizingSeed {
    fn drop(&mut self) {
        for b in self.0.iter_mut() {
            *b = 0;
        }
        core::hint::black_box(&mut self.0);
    }
}

impl core::fmt::Debug for ZeroizingSeed {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str("ZeroizingSeed(<redacted>)")
    }
}

/// Sum_n signing key:
/// - `sk_child`: the currently-active sub-tree's signing key (left
///   sub-tree when `r1_seed.is_some()`, right when `None`).
/// - `r1_seed`: seed for the OTHER (right) sub-tree, or `None` after
///   the level-`n` boundary has been crossed by `update_kes`.
///   Wrapped in `ZeroizingSeed` so the 32 bytes overwrite on drop.
/// - `vk0` / `vk1`: left / right sub-tree verification keys (32-byte
///   Blake2b256 outputs).
///
/// **No `Drop` impl on `SumSigningKey` itself** — that would block
/// destructuring in `update_kes`. Per-field `Drop` (via
/// `ZeroizingSeed`) carries the zeroize guarantee, and `sk_child`
/// recursively zeroizes via its own `Drop`.
pub struct SumSigningKey<D: KesAlgorithm> {
    pub(super) sk_child: D::SigningKey,
    pub(super) r1_seed: Option<ZeroizingSeed>,
    pub(super) vk0: [u8; 32],
    pub(super) vk1: [u8; 32],
}

impl<D: KesAlgorithm> core::fmt::Debug for SumSigningKey<D> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str("SumSigningKey(<redacted>)")
    }
}

/// Sum_n signature = (sigma_d, vk0, vk1) where sigma_d is the
/// inner Sum_(n-1) signature and vk0/vk1 are 32-byte verification
/// keys.
///
/// Manual `Clone` / `PartialEq` / `Eq` impls — `#[derive]` would
/// over-bound `D: Clone + Eq`, but the actual bounds we need are
/// `D::Signature: Clone + Eq`.
pub struct SumSignature<D: KesAlgorithm> {
    pub sigma: D::Signature,
    pub vk0: [u8; 32],
    pub vk1: [u8; 32],
}

impl<D: KesAlgorithm> Clone for SumSignature<D>
where
    D::Signature: Clone,
{
    fn clone(&self) -> Self {
        Self {
            sigma: self.sigma.clone(),
            vk0: self.vk0,
            vk1: self.vk1,
        }
    }
}

impl<D: KesAlgorithm> PartialEq for SumSignature<D>
where
    D::Signature: PartialEq,
{
    fn eq(&self, other: &Self) -> bool {
        self.sigma == other.sigma && self.vk0 == other.vk0 && self.vk1 == other.vk1
    }
}

impl<D: KesAlgorithm> Eq for SumSignature<D> where D::Signature: Eq {}

impl<D: KesAlgorithm> core::fmt::Debug for SumSignature<D>
where
    D::Signature: core::fmt::Debug,
{
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("SumSignature")
            .field("sigma", &self.sigma)
            .field("vk0", &"<VK>")
            .field("vk1", &"<VK>")
            .finish()
    }
}

// =========================================================================
// KesAlgorithm impl for SumKes<D>
// =========================================================================

impl<D> KesAlgorithm for SumKes<D>
where
    D: KesAlgorithm + 'static,
{
    type SigningKey = SumSigningKey<D>;
    type Signature = SumSignature<D>;

    const ALGORITHM_NAME: &'static str = "SumKES";

    const SEED_SIZE: usize = D::SEED_SIZE;
    const SIGNING_KEY_SIZE: usize = D::SIGNING_KEY_SIZE + 96;
    const SIGNATURE_SIZE: usize = D::SIGNATURE_SIZE + 64;

    fn total_periods() -> u32 {
        2 * D::total_periods()
    }

    fn gen_key_kes_from_seed_bytes(seed: &[u8]) -> Result<Self::SigningKey, KesError> {
        if seed.len() != Self::SEED_SIZE {
            return Err(KesError::InvalidSeedLength {
                expected: Self::SEED_SIZE,
                actual: seed.len(),
            });
        }
        let mut seed_arr = [0u8; 32];
        seed_arr.copy_from_slice(seed);

        let (left_seed, right_seed) = expand_seed(&seed_arr);

        // Construct the left sub-tree (this is the active sk_child at
        // period 0).
        let sk_left = D::gen_key_kes_from_seed_bytes(&left_seed)?;
        let vk_left = D::derive_verification_key(&sk_left);

        // Construct the right sub-tree just to derive its vk; drop it
        // immediately (forget). Its seed is stored as `r1_seed` for
        // later `update_kes` reconstruction.
        let sk_right_tmp = D::gen_key_kes_from_seed_bytes(&right_seed)?;
        let vk_right = D::derive_verification_key(&sk_right_tmp);
        drop(sk_right_tmp);

        // Best-effort zeroize the local left_seed copy. The child
        // already made its own copy.
        let mut local_left = left_seed;
        for b in local_left.iter_mut() {
            *b = 0;
        }
        core::hint::black_box(&mut local_left);

        // Best-effort zeroize the outer seed copy.
        for b in seed_arr.iter_mut() {
            *b = 0;
        }
        core::hint::black_box(&mut seed_arr);

        Ok(SumSigningKey {
            sk_child: sk_left,
            r1_seed: Some(ZeroizingSeed(right_seed)),
            vk0: vk_left,
            vk1: vk_right,
        })
    }

    fn derive_verification_key(sk: &Self::SigningKey) -> [u8; 32] {
        hash_concat_vk(&sk.vk0, &sk.vk1)
    }

    fn sign_kes(
        sk: &Self::SigningKey,
        period: u32,
        msg: &[u8],
    ) -> Result<Self::Signature, KesError> {
        let t_half = D::total_periods();
        let total = 2 * t_half;
        if period >= total {
            return Err(KesError::PeriodOutOfRange {
                period,
                max_period: total - 1,
            });
        }
        // `sk.sk_child` is the active sub-tree's key. If period <
        // t_half, we're in the left sub-tree and the child's period
        // is the same. Otherwise (right sub-tree active), the child
        // is at `period - t_half`.
        let child_period = if period < t_half { period } else { period - t_half };
        let sigma = D::sign_kes(&sk.sk_child, child_period, msg)?;
        Ok(SumSignature {
            sigma,
            vk0: sk.vk0,
            vk1: sk.vk1,
        })
    }

    fn verify_kes(
        vk: &[u8; 32],
        period: u32,
        msg: &[u8],
        sig: &Self::Signature,
    ) -> Result<(), KesError> {
        let t_half = D::total_periods();
        let total = 2 * t_half;
        if period >= total {
            return Err(KesError::PeriodOutOfRange {
                period,
                max_period: total - 1,
            });
        }
        // 1. Recompute vk from (vk0 || vk1); compare against the
        //    expected verification key.
        let computed = hash_concat_vk(&sig.vk0, &sig.vk1);
        if &computed != vk {
            return Err(KesError::VerificationFailed);
        }
        // 2. Recurse into the appropriate sub-tree.
        let (child_vk, child_period) = if period < t_half {
            (&sig.vk0, period)
        } else {
            (&sig.vk1, period - t_half)
        };
        D::verify_kes(child_vk, child_period, msg, &sig.sigma)
    }

    fn update_kes(
        sk: Self::SigningKey,
        period: u32,
    ) -> Result<Option<Self::SigningKey>, KesError> {
        let t_half = D::total_periods();
        let total = 2 * t_half;
        if period >= total {
            return Err(KesError::PeriodOutOfRange {
                period,
                max_period: total - 1,
            });
        }

        // Destructure the input key.
        let SumSigningKey {
            sk_child,
            mut r1_seed,
            vk0,
            vk1,
        } = sk;

        let next_period = period + 1;

        // Three transition cases:
        // (a) next_period == total       => key expires.
        // (b) next_period == t_half      => transition into right sub-tree.
        // (c) next_period < t_half       => recurse in left sub-tree.
        // (d) next_period > t_half       => recurse in right sub-tree.

        if next_period == total {
            // Last period; key cannot evolve further.
            drop(sk_child);
            r1_seed.take();
            return Ok(None);
        }

        if next_period == t_half {
            // Cross the level-n boundary: replace sk_child (which
            // is the left sub-tree's last period's key) with a fresh
            // right-sub-tree key generated from r1_seed. Consume
            // the seed.
            let seed_wrapper = r1_seed.take().ok_or(KesError::KeyExpired)?;
            let sk_right = D::gen_key_kes_from_seed_bytes(&seed_wrapper.0)?;
            // seed_wrapper drops here, zeroizing the 32 bytes.
            drop(seed_wrapper);
            drop(sk_child);
            return Ok(Some(SumSigningKey {
                sk_child: sk_right,
                r1_seed: None,
                vk0,
                vk1,
            }));
        }

        if next_period < t_half {
            // Still inside the left sub-tree; recurse.
            match D::update_kes(sk_child, period)? {
                Some(updated) => Ok(Some(SumSigningKey {
                    sk_child: updated,
                    r1_seed,
                    vk0,
                    vk1,
                })),
                None => {
                    r1_seed.take();
                    Ok(None)
                }
            }
        } else {
            // Inside the right sub-tree; recurse with the period
            // offset into the right sub-tree.
            let adjusted = period - t_half;
            match D::update_kes(sk_child, adjusted)? {
                Some(updated) => Ok(Some(SumSigningKey {
                    sk_child: updated,
                    r1_seed: None,
                    vk0,
                    vk1,
                })),
                None => Ok(None),
            }
        }
    }
}
