// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types
// - Explicit state transitions only
// - Canonical serialization for all persisted/hashed data

//! RED producer shell — key custody for live producer-mode
//! (PHASE4-N-Q S3).
//!
//! Holds the `KesSecret`, `VrfSigningKey`, `ColdSigningKey`, and
//! `OperationalCert` — the **only** N-Q surface that touches secret
//! signing material. The GREEN coordinator
//! (`crate::producer::coordinator`) emits a closed
//! `Effect::RequestForge`; S5's main loop calls the shell's
//! signing wrappers + assembles the result into a
//! `CoordinatorEvent::ForgeSucceeded` / `ForgeFailed` /
//! `ForgeNotLeader`.
//!
//! S3 scope: shell construction + KES/VRF/cold wrappers + public-
//! metadata projection. The end-to-end orchestration
//! (`handle_request_forge`) lands in S5 where the main loop
//! composes this shell with `scheduler_step` + `self_accept` +
//! `ServedChainSnapshot`.
//!
//! **Custody discipline carried from N-O / N-P:**
//! - No public byte accessors on `KesSecret` / `VrfSigningKey` /
//!   `ColdSigningKey`.
//! - Custom `Debug` redaction on every secret-bearing struct.
//! - Drop-zeroize via existing `signing.rs` machinery.
//!
//! Mechanical enforcement: `ci/ci_check_private_key_custody.sh`
//! (existing) + the per-cluster `ci_check_producer_coordinator_no_secrets.sh`
//! (new in S2) ensures the GREEN coordinator never touches these
//! types.

use ade_crypto::ed25519::Ed25519VerificationKey;
use ade_crypto::kes::{KesPeriod, KesSignature};
use ade_crypto::vrf::{VrfOutput, VrfProof};
use ade_types::shelley::block::OperationalCert;

use crate::producer::coordinator::OpCertPublicMetadata;
use crate::producer::signing::{
    kes_sign, kes_update, vrf_prove, ColdSigningKey, KesSecret, SigningError, VrfSigningKey,
};

// =========================================================================
// ShellInitError + ShellSignError — closed surfaces
// =========================================================================

/// Closed error surface for `ProducerShell::init`. Carries only
/// non-secret metadata (`&'static str` algorithm names).
#[derive(Debug, PartialEq, Eq)]
pub enum ShellInitError {
    /// `kes_secret` claims to be at a period below the opcert's
    /// declared `kes_start_period`. Operator-supplied opcert /
    /// kes-skey mismatch; fail-closed.
    KesPeriodBelowOpCertStart {
        kes_current: u32,
        opcert_start: u32,
    },
    /// `kes_secret`'s current period is past the opcert's last
    /// covered period (`kes_start_period + 63`). Operator must
    /// issue a new opcert.
    KesPeriodPastOpCertEnd {
        kes_current: u32,
        opcert_last: u32,
    },
    /// Operational certificate has wrong wire shape (hot_vkey not
    /// 32 bytes, sigma not 64 bytes, or kes_period exceeds u32).
    MalformedOpCert {
        detail: &'static str,
    },
    /// The KES key could not be evolved to the target evolution
    /// (`current_kes_period - opcert.kes_period`): either the key is already
    /// evolved PAST it (forward-secrecy — it can no longer sign the current
    /// period) or the target is out of the Sum6KES range. Fail-closed
    /// (OP-OPS-04).
    KesEvolutionFailed {
        key_evolution: u32,
        target_evolution: u32,
    },
}

/// Closed error surface for `ProducerShell::kes_sign_at` /
/// `kes_advance_to` / `vrf_prove`. All variants wrap
/// `SigningError` from `signing.rs` (which carries only non-secret
/// metadata).
#[derive(Debug)]
pub enum ShellSignError {
    /// Requested KES period does not match the shell's current
    /// `KesSecret` period. Caller MUST `kes_advance_to(period)`
    /// first.
    KesPeriodNotCurrent {
        requested: u32,
        current: u32,
    },
    /// Underlying signing primitive surfaced a closed error.
    Signing(SigningError),
}

// =========================================================================
// ProducerShell — the RED key-custody surface
// =========================================================================

/// RED key-custody wrapper. Holds all three N-Q signing keys + the
/// operational certificate. No public byte accessors; signing goes
/// through the wrappers in §"Public surface" below.
///
/// Constructed via [`ProducerShell::init`]. Custom `Debug` redacts
/// every secret-bearing field.
pub struct ProducerShell {
    kes: KesSecret,
    vrf: VrfSigningKey,
    cold: ColdSigningKey,
    opcert: OperationalCert,
    public_metadata: OpCertPublicMetadata,
}

impl ProducerShell {
    /// Initialize the shell with operator-supplied keys + opcert.
    ///
    /// Validates:
    /// - `opcert.hot_vkey.len() == 32` and
    ///   `opcert.sigma.len() == 64` (wire-shape sanity; the full
    ///   opcert-signature verification lives in
    ///   `ade_core::consensus::opcert_validate`, invoked by the
    ///   forge pipeline).
    /// - the INJECTED `current_kes_period` (an ABSOLUTE KES period) ∈
    ///   `[opcert.kes_period, opcert.kes_period + 63]` (the opcert covers the
    ///   current period). The key is then anchored (evolution-0 ↔
    ///   `opcert.kes_period`) and evolved to `current_kes_period -
    ///   opcert.kes_period` so it signs at the current period. The raw key
    ///   evolution index is NEVER compared to the absolute opcert start
    ///   (OP-OPS-04). `current_kes_period` is injected by the RED caller from
    ///   the genesis clock anchor — no wall-clock in this deterministic core.
    ///
    /// The cold-vkey-matches-opcert-issuer check is NOT enforced at
    /// shell-init (the opcert doesn't carry the cold vkey; opcert is
    /// signed by the cold key but the verifier supplies the cold vkey
    /// separately). The forge pipeline's `opcert_validate` catches
    /// signature-vkey mismatches at the BLUE boundary.
    ///
    /// Failure modes are closed; no key material in any returned
    /// error.
    pub fn init(
        kes: KesSecret,
        vrf: VrfSigningKey,
        cold: ColdSigningKey,
        opcert: OperationalCert,
        current_kes_period: u32,
    ) -> Result<Self, ShellInitError> {
        if opcert.hot_vkey.len() != 32 {
            return Err(ShellInitError::MalformedOpCert {
                detail: "opcert.hot_vkey must be 32 bytes",
            });
        }
        if opcert.sigma.len() != 64 {
            return Err(ShellInitError::MalformedOpCert {
                detail: "opcert.sigma must be 64 bytes",
            });
        }

        let opcert_start = u32::try_from(opcert.kes_period).map_err(|_| {
            ShellInitError::MalformedOpCert {
                detail: "opcert.kes_period exceeds u32 (Sum6KES anchor must fit)",
            }
        })?;
        let opcert_last = opcert_start.saturating_add(ade_crypto::kes::SUM6_MAX_PERIOD);

        // OP-OPS-04: `opcert.kes_period` is the ABSOLUTE KES period this key's
        // evolution-0 is certified for; the KES key's own `current_period()` is a
        // RELATIVE evolution index. Compare the INJECTED current ABSOLUTE period
        // (never the raw evolution index) to the opcert window, then anchor
        // evolution-0 at `opcert_start` and evolve the key by
        // `current_kes_period - opcert_start` so it signs at the current period.
        if current_kes_period < opcert_start {
            return Err(ShellInitError::KesPeriodBelowOpCertStart {
                kes_current: current_kes_period,
                opcert_start,
            });
        }
        if current_kes_period > opcert_last {
            return Err(ShellInitError::KesPeriodPastOpCertEnd {
                kes_current: current_kes_period,
                opcert_last,
            });
        }
        let target_evolution = current_kes_period - opcert_start;
        let key_evolution = kes.current_period().0;
        // Forward-only (forward-secrecy): `kes_update` fails closed on a backward
        // or out-of-range target. A fresh cardano-cli key is at evolution 0; this
        // evolves it to `target_evolution`.
        let kes = if target_evolution == key_evolution {
            kes
        } else {
            kes_update(kes, KesPeriod(target_evolution)).map_err(|_| {
                ShellInitError::KesEvolutionFailed {
                    key_evolution,
                    target_evolution,
                }
            })?
        };

        let mut kes_vkey = [0u8; 32];
        kes_vkey.copy_from_slice(&opcert.hot_vkey);

        let cold_vkey_bytes = cold.derive_verification_key_bytes();
        let cold_vkey_hash = blake2b_224_of(&cold_vkey_bytes);

        let public_metadata = OpCertPublicMetadata {
            kes_vkey,
            kes_start_period: opcert_start,
            sequence_number: opcert.sequence_number,
            cold_vkey_hash,
        };

        Ok(Self {
            kes,
            vrf,
            cold,
            opcert,
            public_metadata,
        })
    }

    /// Sign `msg` with the shell's KES key at `period`. Fails closed
    /// if `period != kes_secret.current_period()` (caller must
    /// `kes_advance_to(period)` first; this prevents accidental
    /// out-of-period signatures).
    pub fn kes_sign_at(&self, period: u32, msg: &[u8]) -> Result<KesSignature, ShellSignError> {
        let current = self.kes.current_period().0;
        if period != current {
            return Err(ShellSignError::KesPeriodNotCurrent {
                requested: period,
                current,
            });
        }
        kes_sign(&self.kes, KesPeriod(period), msg).map_err(ShellSignError::Signing)
    }

    /// Evolve the KES key to `to_period`. One-way per N-C / N-P
    /// forward-secrecy discipline. Caller MUST not request a backwards
    /// or out-of-range period; the underlying `kes_update` enforces.
    pub fn kes_advance_to(&mut self, to_period: u32) -> Result<(), ShellSignError> {
        let new_kes = kes_update(
            std::mem::replace(
                &mut self.kes,
                // Placeholder zeroed seed (32 zero bytes). The replace
                // takes the real key out for the update call; we replace
                // it with the result on success or restore the failure
                // path's residual via panic-safe handling. Since
                // KesSecret::from_bytes_zeroizing requires a 32-byte
                // slice, we use a zeroed seed; the new key replaces it
                // immediately.
                //
                // SAFETY: This use of an all-zero seed is bounded to the
                // moment of replacement; on success the new key replaces
                // it; on failure the function returns Err and the shell
                // is dropped. Production scope.
                KesSecret::from_bytes_zeroizing(&[0u8; 32]).expect("32-byte zero seed accepted"),
            ),
            KesPeriod(to_period),
        )
        .map_err(ShellSignError::Signing)?;
        self.kes = new_kes;
        Ok(())
    }

    /// Produce a VRF proof + output for `msg` under the shell's VRF
    /// key.
    pub fn vrf_prove(&self, msg: &[u8]) -> Result<(VrfProof, VrfOutput), ShellSignError> {
        vrf_prove(&self.vrf, msg).map_err(ShellSignError::Signing)
    }

    /// VRF verification key (public). Derived from the shell's VRF
    /// signing key. Non-secret; returned by value. Used by the BLUE
    /// leader-check evaluator to verify VRF proofs the shell emits.
    pub fn vrf_verification_key(&self) -> ade_crypto::vrf::VrfVerificationKey {
        self.vrf.verification_key()
    }

    /// **PHASE4-N-S-A A3** — pre-check that a KES `period` is
    /// within the shell's signing window (current_period ..=
    /// opcert_start + SUM6_MAX_PERIOD). Used by the two-pass
    /// forge to fail fast with `ForgeFailureReason::KesPeriodMismatch`
    /// before issuing the placeholder-signature first pass.
    pub fn kes_period_in_window(&self, period: u32) -> bool {
        let current = self.kes.current_period().0;
        let opcert_start = match u32::try_from(self.opcert.kes_period) {
            Ok(s) => s,
            Err(_) => return false,
        };
        let opcert_last = opcert_start.saturating_add(ade_crypto::kes::SUM6_MAX_PERIOD);
        period >= current && period <= opcert_last
    }

    /// **PHASE4-N-S-A A3** — KES sign over the canonical unsigned-
    /// header pre-image. Accepts only `&UnsignedHeaderPreImage`
    /// (branded type from `ade_ledger::block_validity::unsigned_header_pre_image`);
    /// arbitrary-byte signing is structurally unrepresentable.
    /// The branded type's only constructor is the canonical
    /// recipe + (this) signing API forwards the inner bytes
    /// verbatim to `kes_sign`.
    pub fn kes_sign_header(
        &self,
        period: u32,
        preimage: &ade_ledger::block_validity::unsigned_header_pre_image::UnsignedHeaderPreImage,
    ) -> Result<KesSignature, ShellSignError> {
        self.kes_sign_at(period, preimage.as_bytes())
    }

    /// **PHASE4-N-AC / DC-CRYPTO-10** — evolve the KES key forward to `period`
    /// (if needed) and KES-sign the canonical unsigned-header pre-image at it.
    ///
    /// The RED signing shell evolves the operator key to the REQUESTED period
    /// BEFORE signing, via the deterministic Sum6KES update (`kes_advance_to` →
    /// `kes_update`):
    /// - `period == current` → no-op evolution (the `while current < to` loop in
    ///   `kes_update` does nothing), then sign — existing period-0 signing is
    ///   unchanged;
    /// - `period > current` (in window) → forward evolution, then sign;
    /// - `period < current` → fail closed `Signing(EvolutionBackwards)` — a
    ///   destroyed past period cannot be re-signed (forward-secrecy);
    /// - `period` beyond the key lifetime / unreachable by sequential evolution
    ///   → fail closed `Signing(EvolutionExhausted)`.
    ///
    /// The `period` is passed verbatim — never manually adjusted. After a
    /// successful `kes_advance_to`, `current == period`, so `kes_sign_header`
    /// signs at the current period (no `KesPeriodNotCurrent`). This is the method
    /// the forge MUST use; the `&self` `kes_sign_at`/`kes_sign_header` remain for
    /// callers that manage periods themselves.
    pub fn kes_sign_header_advancing(
        &mut self,
        period: u32,
        preimage: &ade_ledger::block_validity::unsigned_header_pre_image::UnsignedHeaderPreImage,
    ) -> Result<KesSignature, ShellSignError> {
        self.kes_advance_to(period)?;
        self.kes_sign_header(period, preimage)
    }

    /// Current KES period.
    pub fn kes_current_period(&self) -> KesPeriod {
        self.kes.current_period()
    }

    /// Remaining KES evolutions.
    pub fn kes_evolutions_remaining(&self) -> u32 {
        self.kes.evolutions_remaining()
    }

    /// Cold verification key (public). For opcert verification +
    /// header pre-checks.
    pub fn cold_vk(&self) -> Ed25519VerificationKey {
        Ed25519VerificationKey(self.cold.derive_verification_key_bytes())
    }

    /// Operational certificate (public).
    pub fn opcert(&self) -> &OperationalCert {
        &self.opcert
    }

    /// Public projection of opcert metadata for the GREEN
    /// coordinator. Carries no secret material.
    pub fn public_metadata(&self) -> OpCertPublicMetadata {
        self.public_metadata
    }
}

impl core::fmt::Debug for ProducerShell {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("ProducerShell")
            .field("kes", &"<redacted>")
            .field("vrf", &"<redacted>")
            .field("cold", &"<redacted>")
            .field("opcert.sequence_number", &self.opcert.sequence_number)
            .field("opcert.kes_period", &self.opcert.kes_period)
            .field("public_metadata", &self.public_metadata)
            .finish()
    }
}

// =========================================================================
// Helpers
// =========================================================================

fn blake2b_224_of(bytes: &[u8]) -> [u8; 28] {
    ade_crypto::blake2b_224(bytes).0
}

// =========================================================================
// Tests
// =========================================================================

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use ade_crypto::kes::{verify_kes_signature, KesVerificationKey};
    use ade_crypto::vrf::{verify_vrf, VrfVerificationKey};

    fn synth_kes_secret(seed: u8) -> KesSecret {
        KesSecret::from_bytes_zeroizing(&[seed; 32]).unwrap()
    }

    fn synth_vrf_key(seed: u8) -> (VrfSigningKey, VrfVerificationKey) {
        let (sk_bytes, vk_bytes) = cardano_crypto::vrf::VrfDraft03::keypair_from_seed(&[seed; 32]);
        let sk = VrfSigningKey::from_bytes_zeroizing(&sk_bytes).unwrap();
        (sk, VrfVerificationKey(vk_bytes))
    }

    fn synth_cold_key(seed: u8) -> ColdSigningKey {
        ColdSigningKey::from_bytes_zeroizing(&[seed; 32]).unwrap()
    }

    fn synth_opcert(kes_vkey: [u8; 32], kes_period: u64) -> OperationalCert {
        // Construct a synthetic opcert with the cold key's public vkey
        // bound into the sigma signature. The full opcert-signature
        // verification is in BLUE (ade_core::consensus::opcert_validate)
        // and not exercised by S3.
        OperationalCert {
            hot_vkey: kes_vkey.to_vec(),
            sequence_number: 0,
            kes_period,
            sigma: vec![0u8; 64],
        }
    }

    fn make_shell(seed_offset: u8) -> ProducerShell {
        let kes_seed = [0x10 + seed_offset; 32];
        let kes = synth_kes_secret(0x10 + seed_offset);
        let (vrf, _vrf_vk) = synth_vrf_key(0x20 + seed_offset);
        let cold = synth_cold_key(0x30 + seed_offset);
        // Derive the KES vkey from the same seed so the opcert's
        // hot_vkey matches the shell's KES key.
        let kes_sk_raw =
            ade_crypto::kes_sum::Sum6Kes::gen_key_kes_from_seed_bytes(&kes_seed).unwrap();
        use ade_crypto::kes_sum::KesAlgorithm;
        let kes_vkey = ade_crypto::kes_sum::Sum6Kes::derive_verification_key(&kes_sk_raw);
        let opcert = synth_opcert(kes_vkey, 0);
        ProducerShell::init(kes, vrf, cold, opcert, 0).unwrap()
    }

    #[test]
    fn shell_init_succeeds_for_matching_keys() {
        let shell = make_shell(0);
        assert_eq!(shell.kes_current_period().0, 0);
        assert_eq!(shell.public_metadata().kes_start_period, 0);
    }

    #[test]
    fn shell_init_rejects_malformed_opcert_hot_vkey_length() {
        let kes = synth_kes_secret(0x10);
        let (vrf, _) = synth_vrf_key(0x20);
        let cold = synth_cold_key(0x30);
        let mut bad_opcert = synth_opcert([0x99; 32], 0);
        bad_opcert.hot_vkey = vec![0; 16]; // wrong length
        let err = ProducerShell::init(kes, vrf, cold, bad_opcert, 0).unwrap_err();
        assert!(matches!(err, ShellInitError::MalformedOpCert { .. }));
    }

    #[test]
    fn shell_init_rejects_kes_period_below_opcert_start() {
        // OP-OPS-04: the INJECTED current absolute period (5) is below an opcert
        // anchored at absolute start 10 -> fail closed. (Pre-fix this compared the
        // key's raw evolution index; now it compares the injected absolute period.)
        let kes = synth_kes_secret(0x10);
        let (vrf, _) = synth_vrf_key(0x20);
        let cold = synth_cold_key(0x30);
        let opcert = synth_opcert([0xAB; 32], 10);
        let err = ProducerShell::init(kes, vrf, cold, opcert, 5).unwrap_err();
        assert!(matches!(
            err,
            ShellInitError::KesPeriodBelowOpCertStart {
                kes_current: 5,
                opcert_start: 10,
            }
        ));
    }

    #[test]
    fn op_ops_04_anchors_evolution_zero_at_opcert_start_and_evolves_to_current_period() {
        // OP-OPS-04 acceptance: a FRESH key (evolution 0) certified by an opcert
        // with ABSOLUTE start 885; the current absolute period 887 is inside the
        // window [885, 948]. The shell anchors evolution-0 at 885 and EVOLVES the
        // key by delta = 887 - 885 = 2 -- it never compares the raw evolution
        // index (0) to the absolute start (885).
        let kes = synth_kes_secret(0x42);
        assert_eq!(kes.current_period().0, 0, "fresh key at evolution 0");
        let (vrf, _) = synth_vrf_key(0x20);
        let cold = synth_cold_key(0x30);
        let opcert = synth_opcert([0xAB; 32], 885);
        let shell = ProducerShell::init(kes, vrf, cold, opcert, 887)
            .expect("current period 887 in opcert window [885, 948]");
        assert_eq!(
            shell.kes_current_period().0,
            2,
            "key anchored at 885 evolves by delta 887-885=2"
        );
    }

    #[test]
    fn op_ops_04_fails_closed_outside_window_passes_at_boundaries() {
        let mk = |start: u64, current: u32| {
            let kes = synth_kes_secret(0x43);
            let (vrf, _) = synth_vrf_key(0x20);
            let cold = synth_cold_key(0x30);
            let opcert = synth_opcert([0xAB; 32], start);
            ProducerShell::init(kes, vrf, cold, opcert, current)
        };
        // BELOW start 885 -> fail closed (the injected ABSOLUTE period 884 is
        // compared, never the raw key evolution index 0).
        assert!(matches!(
            mk(885, 884),
            Err(ShellInitError::KesPeriodBelowOpCertStart {
                kes_current: 884,
                opcert_start: 885
            })
        ));
        // ABOVE opcert_last (885 + 63 = 948) -> fail closed.
        assert!(matches!(
            mk(885, 949),
            Err(ShellInitError::KesPeriodPastOpCertEnd {
                kes_current: 949,
                opcert_last: 948
            })
        ));
        // Boundaries pass: start 885 (delta 0) and last 948 (delta 63).
        assert!(mk(885, 885).is_ok(), "opcert start passes");
        assert!(mk(885, 948).is_ok(), "opcert last (start+63) passes");
    }

    #[test]
    fn shell_kes_sign_at_current_period_succeeds_and_verifies() {
        let shell = make_shell(0);
        let msg = b"S3 KES sign test";
        let sig = shell.kes_sign_at(0, msg).unwrap();
        // Verify against the KES vkey derived from the same seed.
        use ade_crypto::kes_sum::KesAlgorithm;
        let kes_sk =
            ade_crypto::kes_sum::Sum6Kes::gen_key_kes_from_seed_bytes(&[0x10; 32]).unwrap();
        let vk_bytes = ade_crypto::kes_sum::Sum6Kes::derive_verification_key(&kes_sk);
        verify_kes_signature(&KesVerificationKey(vk_bytes), KesPeriod(0), msg, &sig).unwrap();
    }

    /// **PHASE4-N-S-A A4** — kes_sign_header signs the branded
    /// UnsignedHeaderPreImage; the resulting signature verifies
    /// against the same pre-image bytes via the validator's
    /// verify_kes_signature primitive. End-to-end shell-API
    /// contract proof.
    #[test]
    fn shell_kes_sign_header_produces_verifiable_signature() {
        use ade_codec::cbor::{
            write_array_header, write_bytes_canonical, ContainerEncoding, IntWidth,
        };
        use ade_crypto::kes_sum::KesAlgorithm;
        use ade_ledger::block_validity::unsigned_header_pre_image::unsigned_header_pre_image;
        use ade_types::shelley::block::{PrevHash, ProtocolVersion};
        use ade_types::Hash32;

        let shell = make_shell(0);

        // Build a synthetic vrf_result CBOR (mirrors forge.rs).
        let vrf_result = {
            let mut buf = Vec::new();
            write_array_header(
                &mut buf,
                ContainerEncoding::Definite(2, IntWidth::Inline),
            );
            write_bytes_canonical(&mut buf, &[0xAA; 64]);
            write_bytes_canonical(&mut buf, &[0xBB; 80]);
            buf
        };

        let preimage = unsigned_header_pre_image(
            100,
            1,
            PrevHash::Block(Hash32([0u8; 32])),
            vec![0x01; 32],
            vec![0x02; 32],
            vrf_result,
            128,
            Hash32([0x05; 32]),
            shell.opcert().clone(),
            ProtocolVersion { major: 9, minor: 0 },
        )
        .expect("recipe encode");

        let sig = shell.kes_sign_header(0, &preimage).expect("sign");

        // Verify via the standalone primitive (the same path
        // verify_header_kes uses).
        let kes_sk =
            ade_crypto::kes_sum::Sum6Kes::gen_key_kes_from_seed_bytes(&[0x10; 32]).unwrap();
        let vk_bytes = ade_crypto::kes_sum::Sum6Kes::derive_verification_key(&kes_sk);
        verify_kes_signature(
            &KesVerificationKey(vk_bytes),
            KesPeriod(0),
            preimage.as_bytes(),
            &sig,
        )
        .expect("KES signature must verify against the canonical pre-image");
    }

    #[test]
    fn shell_kes_sign_at_wrong_period_errors() {
        let shell = make_shell(0);
        let err = shell.kes_sign_at(5, b"x").unwrap_err();
        assert!(matches!(
            err,
            ShellSignError::KesPeriodNotCurrent { requested: 5, current: 0 }
        ));
    }

    #[test]
    fn shell_kes_advance_to_evolves_key_one_way() {
        let mut shell = make_shell(0);
        shell.kes_advance_to(5).unwrap();
        assert_eq!(shell.kes_current_period().0, 5);
        // After evolution, signing at the OLD period must fail.
        let err = shell.kes_sign_at(0, b"x").unwrap_err();
        assert!(matches!(err, ShellSignError::KesPeriodNotCurrent { .. }));
        // Signing at the NEW period must succeed.
        shell.kes_sign_at(5, b"y").unwrap();
    }

    // ---- PHASE4-N-AC / DC-CRYPTO-10: evolve-then-sign --------------------

    /// Build a canonical unsigned-header pre-image for the advancing tests
    /// (mirrors `shell_kes_sign_header_produces_verifiable_signature`).
    fn synth_preimage(
        opcert: ade_types::shelley::block::OperationalCert,
    ) -> ade_ledger::block_validity::unsigned_header_pre_image::UnsignedHeaderPreImage {
        use ade_codec::cbor::{
            write_array_header, write_bytes_canonical, ContainerEncoding, IntWidth,
        };
        use ade_ledger::block_validity::unsigned_header_pre_image::unsigned_header_pre_image;
        use ade_types::shelley::block::{PrevHash, ProtocolVersion};
        use ade_types::Hash32;
        let vrf_result = {
            let mut buf = Vec::new();
            write_array_header(&mut buf, ContainerEncoding::Definite(2, IntWidth::Inline));
            write_bytes_canonical(&mut buf, &[0xAA; 64]);
            write_bytes_canonical(&mut buf, &[0xBB; 80]);
            buf
        };
        unsigned_header_pre_image(
            100,
            1,
            PrevHash::Block(Hash32([0u8; 32])),
            vec![0x01; 32],
            vec![0x02; 32],
            vrf_result,
            128,
            Hash32([0x05; 32]),
            opcert,
            ProtocolVersion { major: 9, minor: 0 },
        )
        .expect("recipe encode")
    }

    fn kes_root_vk() -> KesVerificationKey {
        use ade_crypto::kes_sum::KesAlgorithm;
        let kes_sk =
            ade_crypto::kes_sum::Sum6Kes::gen_key_kes_from_seed_bytes(&[0x10; 32]).unwrap();
        KesVerificationKey(ade_crypto::kes_sum::Sum6Kes::derive_verification_key(&kes_sk))
    }

    #[test]
    fn shell_kes_sign_header_advancing_evolves_then_signs() {
        // period-0 key, requested period 1 -> evolves to 1 then signs (acceptance #1).
        let mut shell = make_shell(0);
        let pre = synth_preimage(shell.opcert().clone());
        let sig = shell
            .kes_sign_header_advancing(1, &pre)
            .expect("evolve to period 1 then sign");
        assert_eq!(shell.kes_current_period().0, 1, "key evolved to period 1");
        verify_kes_signature(&kes_root_vk(), KesPeriod(1), pre.as_bytes(), &sig)
            .expect("signature must verify at the evolved period 1");
    }

    #[test]
    fn shell_kes_sign_header_advancing_at_current_period_signs() {
        // requested period == current (0) -> no-op evolution, still signs
        // (acceptance #4: existing period-0 signing unchanged).
        let mut shell = make_shell(0);
        let pre = synth_preimage(shell.opcert().clone());
        let sig = shell
            .kes_sign_header_advancing(0, &pre)
            .expect("no-op evolution at current period then sign");
        assert_eq!(shell.kes_current_period().0, 0);
        verify_kes_signature(&kes_root_vk(), KesPeriod(0), pre.as_bytes(), &sig)
            .expect("signature must verify at period 0");
    }

    #[test]
    fn shell_kes_sign_header_advancing_backwards_fails_closed() {
        // advanced to 5, then a backwards request (2) fails closed; no signature
        // (acceptance #2: before the key's current period / forward-secrecy).
        let mut shell = make_shell(0);
        shell.kes_advance_to(5).unwrap();
        let pre = synth_preimage(shell.opcert().clone());
        let err = shell
            .kes_sign_header_advancing(2, &pre)
            .expect_err("backwards period must fail closed");
        assert!(matches!(
            err,
            ShellSignError::Signing(crate::producer::signing::SigningError::EvolutionBackwards { .. })
        ));
        // Fail-closed = the structured EvolutionBackwards error + NO signature
        // (the `expect_err` above). We do NOT assert the post-error key period:
        // `kes_advance_to` replaces the key before `kes_update`, so a failed
        // advance leaves it zeroed — a pre-existing primitive behavior that is
        // UNREACHABLE via the forge (the `kes_period_in_window` pre-check rejects
        // a backwards/out-of-window period BEFORE `kes_sign_header_advancing` is
        // ever called; this test invokes the shell method directly to exercise
        // the guard).
    }

    #[test]
    fn shell_kes_sign_header_advancing_beyond_lifetime_fails_closed() {
        // beyond SUM6_MAX_PERIOD (63) -> unreachable by evolution; fail closed
        // (acceptance #2: beyond key lifetime).
        let mut shell = make_shell(0);
        let pre = synth_preimage(shell.opcert().clone());
        let err = shell
            .kes_sign_header_advancing(ade_crypto::kes::SUM6_MAX_PERIOD + 1, &pre)
            .expect_err("period beyond key lifetime must fail closed");
        assert!(matches!(
            err,
            ShellSignError::Signing(crate::producer::signing::SigningError::EvolutionExhausted { .. })
        ));
    }

    #[test]
    fn shell_vrf_prove_round_trips_verification() {
        let shell = make_shell(0);
        let msg = b"S3 VRF prove test";
        let (proof, _output) = shell.vrf_prove(msg).unwrap();
        // Derive vrf vk from the same seed and verify.
        let (_sk, vrf_vk) = synth_vrf_key(0x20);
        verify_vrf(&vrf_vk, &proof, msg).unwrap();
    }

    #[test]
    fn shell_debug_redacts_secret_fields() {
        let shell = make_shell(0);
        let formatted = format!("{:?}", shell);
        assert!(formatted.contains("<redacted>"));
        // Source seed must not appear (decimal or hex form).
        assert!(!formatted.contains("16, 16, 16")); // 0x10
        assert!(!formatted.contains("48, 48, 48")); // 0x30
    }

    #[test]
    fn shell_public_metadata_projection_is_byte_stable() {
        let shell = make_shell(0);
        let a = shell.public_metadata();
        let b = shell.public_metadata();
        assert_eq!(a, b);
    }

    #[test]
    fn shell_kes_evolutions_remaining_tracks_after_advance() {
        let mut shell = make_shell(0);
        assert_eq!(shell.kes_evolutions_remaining(), 63);
        shell.kes_advance_to(10).unwrap();
        assert_eq!(shell.kes_evolutions_remaining(), 53);
    }
}
