// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types
// - Explicit state transitions only
// - Canonical serialization for all persisted/hashed data

//! Reference vectors for the PHASE4-N-C S1 RED signing primitives
//! (PHASE4-N-P S5 migrated to the Ade-owned BLUE KES algorithm).
//!
//! Each vector pins a `(seed, …, expected_*)` triple. The expected bytes
//! are computed at materialization time from the Ade-owned
//! `ade_crypto::kes_sum::Sum6Kes` algorithm (VRF still uses upstream
//! `cardano-crypto`), then frozen by the calling test. The test in
//! `ade_runtime::producer::signing` re-derives the same bytes through
//! the RED wrapper and asserts byte equality — any drift between the
//! wrapper and the underlying algorithm (mis-padded buffers,
//! accidentally re-derived seeds, parameter-order mistakes) shows up
//! as a vector mismatch.
//!
//! **PHASE4-N-P S5 migration**: KES vectors were previously
//! materialized via `cardano_crypto::kes::Sum6Kes`. After S5 they use
//! our BLUE impl; because our impl uses the Haskell-correct
//! `expand_seed` prefix bytes (0x01 / 0x02) while cardano-crypto Rust
//! 1.0.8 uses (0x00 / 0x01), the reference signatures HAVE CHANGED at
//! the byte level vs the N-C-era reference set. That is intentional
//! and load-bearing: our impl now matches cardano-cli ground truth
//! (see `crates/ade_crypto/src/kes_sum/cardano_cli_corpus.rs`).
//!
//! The reference set deliberately exercises:
//! - VRF: distinct seeds + alphas (period-irrelevant for VRF).
//! - KES: period 0 (base case), period 1 (one update), period 32 (mid-
//!   tree), and period 63 (last legal period). Each entry stresses a
//!   different branch of the Sum6 recursion.
//! - KES update chain: fingerprints of the evolved tree at three
//!   representative periods.

use ade_crypto::blake2b_256;
use ade_crypto::kes_sum::{KesAlgorithm, Sum6Kes};

use cardano_crypto::vrf::VrfDraft03;

pub struct VrfReferenceVector {
    pub seed: [u8; 32],
    pub alpha: Vec<u8>,
    pub expected_proof: [u8; 80],
    pub expected_output: [u8; 64],
}

pub struct KesReferenceVector {
    pub seed: [u8; 32],
    pub period: u32,
    pub message: Vec<u8>,
    pub expected_signature: [u8; 448],
}

pub fn vrf_reference_set() -> Vec<VrfReferenceVector> {
    let seeds: [[u8; 32]; 3] = [[0x01; 32], [0xA5; 32], [0xFE; 32]];
    let alphas: [&[u8]; 3] = [
        b"epoch 0 slot 0",
        b"epoch 100 slot 432000",
        b"epoch 510 slot 220247424",
    ];
    seeds
        .iter()
        .zip(alphas.iter())
        .map(|(seed, alpha)| {
            let (sk, _vk) = VrfDraft03::keypair_from_seed(seed);
            let proof = VrfDraft03::prove(&sk, alpha).expect("vrf prove must succeed");
            let output = VrfDraft03::proof_to_hash(&proof).expect("vrf proof_to_hash must succeed");
            VrfReferenceVector {
                seed: *seed,
                alpha: alpha.to_vec(),
                expected_proof: proof,
                expected_output: output,
            }
        })
        .collect()
}

pub fn kes_reference_set() -> Vec<KesReferenceVector> {
    let entries: [([u8; 32], u32, &[u8]); 4] = [
        ([0x11; 32], 0, b"period 0 block"),
        ([0x22; 32], 1, b"period 1 block"),
        ([0x33; 32], 32, b"period 32 block"),
        ([0x44; 32], 63, b"period 63 block"),
    ];
    entries
        .iter()
        .map(|(seed, period, msg)| {
            let mut sk = Sum6Kes::gen_key_kes_from_seed_bytes(seed).expect("kes gen must succeed");
            let mut current = 0u32;
            while current < *period {
                sk = Sum6Kes::update_kes(sk, current)
                    .expect("kes update must succeed")
                    .expect("kes key must not expire during reference materialization");
                current += 1;
            }
            let raw = Sum6Kes::sign_kes(&sk, *period, msg).expect("kes sign must succeed");
            let bytes = Sum6Kes::raw_serialize_signature_kes(&raw);
            let mut arr = [0u8; 448];
            arr.copy_from_slice(&bytes);
            KesReferenceVector {
                seed: *seed,
                period: *period,
                message: msg.to_vec(),
                expected_signature: arr,
            }
        })
        .collect()
}

/// Reference chain for `kes_update`. Each tuple is
/// `(seed, period_after_n_updates, expected_signing_key_fingerprint)`.
/// The fingerprint is the blake2b-256 of the canonical KES signature
/// over a fixed probe message at the evolved period — it collapses the
/// whole evolved tree state into 32 bytes so a single equality check
/// rejects any drift in the tree.
pub fn kes_update_reference_chain() -> Vec<([u8; 32], u32, [u8; 32])> {
    let probe = b"kes-update-chain-probe";
    let entries: [([u8; 32], u32); 3] = [([0xA1; 32], 1), ([0xB2; 32], 17), ([0xC3; 32], 63)];
    entries
        .iter()
        .map(|(seed, target_period)| {
            let mut sk = Sum6Kes::gen_key_kes_from_seed_bytes(seed).expect("kes gen must succeed");
            let mut current = 0u32;
            while current < *target_period {
                sk = Sum6Kes::update_kes(sk, current)
                    .expect("kes update must succeed")
                    .expect("kes key must not expire during reference materialization");
                current += 1;
            }
            let raw = Sum6Kes::sign_kes(&sk, *target_period, probe).expect("kes sign must succeed");
            let sig_bytes = Sum6Kes::raw_serialize_signature_kes(&raw);
            let fp = blake2b_256(&sig_bytes).0;
            (*seed, *target_period, fp)
        })
        .collect()
}
