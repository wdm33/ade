// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types
// - Explicit state transitions only
// - Canonical serialization for all persisted/hashed data

//! EPOCH-CONSENSUS-VIEW Slice 2 (DC-EVIEW-02) — typed, era-gated stake-reference
//! classification.
//!
//! SCOPE (binding): DC-EVIEW-02 proves typed, era-aware stake-reference
//! classification ONLY. It does NOT prove any stake contribution, pool
//! attribution, snapshot, or leader-election semantics — those are Slice 3+.
//!
//! Given canonical address bytes and a canonical era/protocol-version context
//! (a TYPED `CardanoEra` already bound to the block being processed — never
//! inferred from the bytes, local config, wall-clock, or a caller flag),
//! [`classify_output_stake_ref`] returns ONE deterministic typed result:
//! [`StakeRefClass`] = `Base | Pointer | Null | Reject`.
//!
//! This is the per-output attribution PRIMITIVE: it extracts the stake reference
//! only. It resolves nothing, sums nothing, and **no result directly changes
//! stake totals** — aggregation (resolve pre-Conway pointers, sum per
//! registered+delegated pool, add reward balances) is Slice 3.
//!
//! Era gate (Conway spec §9.1.2): pointer-address stake is RETIRED at Conway
//! (protocol major 9+). Pre-Conway a pointer address yields `Pointer(coords)`
//! (a decoded, UNRESOLVED reference that implies no credential / contribution /
//! eligibility); at Conway+ it yields `Null` (spendable, contributes 0). The
//! whole Conway era is PV9+, so the gate keys on `era >= CardanoEra::Conway`.
//!
//! No fixed byte offset is the contract across variants/eras: classification
//! routes through the typed `decode_address` chokepoint + per-form structural
//! validation. A malformed / under-length / wrong-position address is
//! `Reject(..)`, kept DISTINCT from `Null` (`Null` = a valid non-staking form;
//! `Reject` = not a valid output stake reference).

use ade_codec::address::decode_address;
use ade_codec::error::CodecError;
use ade_types::address::Address;
use ade_types::era::CardanoEra;
use ade_types::shelley::cert::StakeCredential;
use ade_types::Hash28;

/// A decoded, UNRESOLVED chain pointer `(slot, txIx, certIx)` from a pointer
/// address. It carries ONLY the coordinates — never a credential, a stake
/// contribution, or any eligibility. Resolution against the pointer map (built
/// from registration certs) is Slice 3, and only pre-Conway.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PointerRef {
    pub slot: u64,
    pub tx_index: u64,
    pub cert_index: u64,
}

/// Why an address is NOT a valid output stake reference. DISTINCT from `Null`:
/// `Null` is the semantic result for a valid non-staking form (enterprise,
/// Byron, or a Conway-retired pointer); `Reject` is invalid input or an address
/// form that cannot validly occupy a TxOut payment position.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StakeRefReject {
    /// Empty address bytes.
    Empty,
    /// The header byte's type nibble is not a known address type.
    UnknownAddressType,
    /// A base address (type 0-3) whose length is not header(1)+payment(28)+stake(28)=57.
    MalformedBase,
    /// A pointer address (type 4-5) too short to hold header(1)+payment(28) before the pointer.
    MalformedPointer,
    /// A reward address (type 14-15) in a TxOut payment position. A reward address
    /// is a withdrawal target / staking-credential form, NOT a valid output payment
    /// address; it must never be treated as ordinary output stake. The full ledger
    /// rule (reward addresses cannot occur as outputs) is proven in Slice 3; here it
    /// is fail-closed and decoder-complete (the reward form is recognised, not summed).
    RewardAddressNotValidAsOutput,
}

/// The one deterministic typed result of classifying an output's address for
/// stake attribution. No variant directly changes stake totals.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StakeRefClass {
    /// A base address's explicit staking credential (key/script per header bit 5).
    Base(StakeCredential),
    /// A decoded, unresolved pointer reference (pre-Conway only).
    Pointer(PointerRef),
    /// A valid non-staking form: enterprise, Byron, or a Conway-retired pointer.
    Null,
    /// Not a valid output stake reference (distinct from `Null`).
    Reject(StakeRefReject),
}

/// Classify an output's address bytes for stake attribution, gated by the
/// block's bound `era`. Total, pure, deterministic. Reusable by the Slice-3
/// aggregation; it never resolves, sums, or mutates anything.
pub fn classify_output_stake_ref(addr_bytes: &[u8], era: CardanoEra) -> StakeRefClass {
    // Header classification routes through the single typed decode chokepoint.
    let addr = match decode_address(addr_bytes) {
        Ok(a) => a,
        Err(CodecError::UnexpectedEof { .. }) => {
            return StakeRefClass::Reject(StakeRefReject::Empty)
        }
        Err(_) => return StakeRefClass::Reject(StakeRefReject::UnknownAddressType),
    };

    match addr {
        // Base 0-3: header(1) + payment(28) + staking(28) = 57 bytes exactly.
        // The staking credential is the decoded form, NOT a blind [29..57] slice:
        // we first prove the structure, then read the staking part with key/script
        // discrimination from header bit 5.
        Address::Base(b) => {
            if b.len() != 57 {
                return StakeRefClass::Reject(StakeRefReject::MalformedBase);
            }
            let header = b[0];
            let stake_is_script = (header >> 5) & 1 == 1;
            let mut h = [0u8; 28];
            h.copy_from_slice(&b[29..57]);
            let cred = if stake_is_script {
                StakeCredential::ScriptHash(Hash28(h))
            } else {
                StakeCredential::KeyHash(Hash28(h))
            };
            StakeRefClass::Base(cred)
        }
        // Pointer 4-5: era-gated. Conway+ retires pointer stake -> Null (the address
        // is spendable, contributes 0). Pre-Conway it decodes to an UNRESOLVED
        // PointerRef. The byte layout is header(1) + payment(28) + 3 base-128 varints.
        Address::Pointer(b) => {
            if era >= CardanoEra::Conway {
                return StakeRefClass::Null;
            }
            if b.len() < 29 {
                return StakeRefClass::Reject(StakeRefReject::MalformedPointer);
            }
            match decode_pointer_coords(&b[29..]) {
                Some(p) => StakeRefClass::Pointer(p),
                None => StakeRefClass::Reject(StakeRefReject::MalformedPointer),
            }
        }
        // Enterprise 6-7 and Byron 8: valid non-staking forms in every era. They
        // carry no staking part and contribute nothing, so the result is the
        // semantic `Null` regardless of length.
        Address::Enterprise(_) => StakeRefClass::Null,
        Address::Byron(_) => StakeRefClass::Null,
        // Reward 14-15: recognised for decoder completeness, but a reward address is
        // not a valid output payment address -> fail-closed, never ordinary output
        // stake. (Slice 3 proves the ledger rule.)
        Address::Reward(_) => {
            StakeRefClass::Reject(StakeRefReject::RewardAddressNotValidAsOutput)
        }
    }
}

/// Decode exactly three base-128 big-endian varints (slot, txIx, certIx) from a
/// pointer address tail, consuming the whole slice. Each byte's high bit is the
/// continuation flag. `None` on a truncated varint, a u64 overflow, or leftover
/// bytes after the third coordinate (a pointer is exactly three coordinates).
fn decode_pointer_coords(tail: &[u8]) -> Option<PointerRef> {
    let mut pos = 0usize;
    let slot = decode_varint(tail, &mut pos)?;
    let tx_index = decode_varint(tail, &mut pos)?;
    let cert_index = decode_varint(tail, &mut pos)?;
    if pos != tail.len() {
        return None; // trailing bytes after the third coordinate
    }
    Some(PointerRef {
        slot,
        tx_index,
        cert_index,
    })
}

/// One base-128 big-endian unsigned varint, advancing `pos`. `None` on a
/// truncated tail or a value that would overflow u64 (`checked_mul` rejects the
/// magnitude — a plain `<< 7` would silently drop the shifted-out high bits).
///
/// NB (Slice-3 obligation, grounded 2026-06-20 vs cardano-ledger `master`): this
/// accepts bounded leading-zero-group encodings (e.g. `[0x80,0x01]` == `[0x01]`),
/// which MATCHES cardano-ledger -- it accepts bounded non-minimal encodings; its
/// strict check is a WIDTH check, NOT a minimal-form check. Harmless here
/// (pre-Conway only, the coordinate feeds no stake total, deterministic). What
/// DIVERGES and Slice 3 must replace: this rejects a `> u64` magnitude, but
/// cardano-ledger is ERA-PARAMETERIZED -- at Conway+ reject OVER-WIDTH (u32 slot /
/// u16 txIx / u16 certIx, bounded group counts) + trailing bytes; at Babbage
/// NORMALIZE (clamp the whole 3-tuple to 0 if any coordinate overflows its width)
/// + reject trailing; at <=Alonzo normalize + crop trailing. Slice 3's resolver
/// must implement that exact rule -- NOT reject-all-non-canonical (which would
/// false-reject txs cardano-node accepts).
fn decode_varint(bytes: &[u8], pos: &mut usize) -> Option<u64> {
    let mut result: u64 = 0;
    loop {
        let byte = *bytes.get(*pos)?;
        *pos += 1;
        // base-128: result = result*128 + low7. checked_mul rejects a value whose
        // magnitude would exceed u64 (NOT a bare `<< 7`, which wraps silently).
        result = result.checked_mul(128)?.checked_add((byte & 0x7f) as u64)?;
        if byte & 0x80 == 0 {
            return Some(result);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Build a base address: header + 28 payment + 28 staking bytes (57 total).
    fn base_addr(addr_type: u8, payment_fill: u8, stake_fill: u8) -> Vec<u8> {
        let mut v = vec![addr_type << 4]; // network nibble 0 (testnet)
        v.extend(std::iter::repeat(payment_fill).take(28));
        v.extend(std::iter::repeat(stake_fill).take(28));
        v
    }

    fn stake28(fill: u8) -> Hash28 {
        Hash28([fill; 28])
    }

    // A pointer address: header(type 4/5) + 28 payment + the raw varint tail.
    fn pointer_addr(addr_type: u8, tail: &[u8]) -> Vec<u8> {
        let mut v = vec![addr_type << 4];
        v.extend(std::iter::repeat(0xaa).take(28));
        v.extend_from_slice(tail);
        v
    }

    // CIP-19 vector: base type 0 = payment key, stake key.
    #[test]
    fn base_type0_is_stake_key_hash() {
        let a = base_addr(0, 0x11, 0x22);
        assert_eq!(
            classify_output_stake_ref(&a, CardanoEra::Conway),
            StakeRefClass::Base(StakeCredential::KeyHash(stake28(0x22)))
        );
    }

    // base type 1 = payment script, stake KEY (bit 5 clear).
    #[test]
    fn base_type1_is_stake_key_hash() {
        let a = base_addr(1, 0x11, 0x22);
        assert_eq!(
            classify_output_stake_ref(&a, CardanoEra::Babbage),
            StakeRefClass::Base(StakeCredential::KeyHash(stake28(0x22)))
        );
    }

    // base type 2 = payment key, stake SCRIPT (bit 5 set).
    #[test]
    fn base_type2_is_stake_script_hash() {
        let a = base_addr(2, 0x11, 0x33);
        assert_eq!(
            classify_output_stake_ref(&a, CardanoEra::Conway),
            StakeRefClass::Base(StakeCredential::ScriptHash(stake28(0x33)))
        );
    }

    // base type 3 = payment script, stake SCRIPT.
    #[test]
    fn base_type3_is_stake_script_hash() {
        let a = base_addr(3, 0x11, 0x44);
        assert_eq!(
            classify_output_stake_ref(&a, CardanoEra::Conway),
            StakeRefClass::Base(StakeCredential::ScriptHash(stake28(0x44)))
        );
    }

    // The era gate (the deliberate fixture): the SAME pointer-address bytes
    // classify to Pointer(coords) pre-Conway and Null at Conway+. The gate keys
    // on the bound era context, not the bytes.
    #[test]
    fn pointer_is_decoded_pre_conway_and_retired_at_conway() {
        // single-byte varints slot=1, txIx=2, certIx=3.
        let a = pointer_addr(4, &[0x01, 0x02, 0x03]);
        assert_eq!(
            classify_output_stake_ref(&a, CardanoEra::Babbage),
            StakeRefClass::Pointer(PointerRef {
                slot: 1,
                tx_index: 2,
                cert_index: 3
            })
        );
        assert_eq!(
            classify_output_stake_ref(&a, CardanoEra::Conway),
            StakeRefClass::Null
        );
        // Alonzo (also pre-Conway) decodes the pointer too.
        assert_eq!(
            classify_output_stake_ref(&a, CardanoEra::Alonzo),
            StakeRefClass::Pointer(PointerRef {
                slot: 1,
                tx_index: 2,
                cert_index: 3
            })
        );
    }

    // A multi-byte base-128 varint (slot = 128 = [0x81,0x00]).
    #[test]
    fn pointer_multibyte_varint_pre_conway() {
        let a = pointer_addr(5, &[0x81, 0x00, 0x02, 0x03]);
        assert_eq!(
            classify_output_stake_ref(&a, CardanoEra::Babbage),
            StakeRefClass::Pointer(PointerRef {
                slot: 128,
                tx_index: 2,
                cert_index: 3
            })
        );
    }

    // A `Pointer` result carries ONLY coordinates — no credential / contribution.
    // (Value-level: the variant has no StakeCredential reachable.)
    #[test]
    fn pointer_result_exposes_no_credential() {
        let a = pointer_addr(4, &[0x05, 0x06, 0x07]);
        match classify_output_stake_ref(&a, CardanoEra::Mary) {
            StakeRefClass::Pointer(p) => {
                assert_eq!(
                    p,
                    PointerRef {
                        slot: 5,
                        tx_index: 6,
                        cert_index: 7
                    }
                );
            }
            other => panic!("expected Pointer, got {other:?}"),
        }
    }

    // Enterprise (6/7) and Byron (8) are Null in every era.
    #[test]
    fn enterprise_and_byron_are_null_all_eras() {
        let ent = {
            let mut v = vec![6u8 << 4];
            v.extend(std::iter::repeat(0x11).take(28));
            v
        };
        let byron = {
            let mut v = vec![8u8 << 4];
            v.extend(std::iter::repeat(0x99).take(20));
            v
        };
        for era in [CardanoEra::Shelley, CardanoEra::Babbage, CardanoEra::Conway] {
            assert_eq!(classify_output_stake_ref(&ent, era), StakeRefClass::Null);
            assert_eq!(classify_output_stake_ref(&byron, era), StakeRefClass::Null);
        }
    }

    // Reward (14/15) is fail-closed: never Base, never Null — Reject.
    #[test]
    fn reward_address_is_rejected_not_summed() {
        let mut v = vec![14u8 << 4];
        v.extend(std::iter::repeat(0x55).take(28));
        assert_eq!(
            classify_output_stake_ref(&v, CardanoEra::Conway),
            StakeRefClass::Reject(StakeRefReject::RewardAddressNotValidAsOutput)
        );
        // type 15 (stake script) reward also rejected.
        let mut w = vec![15u8 << 4];
        w.extend(std::iter::repeat(0x66).take(28));
        assert_eq!(
            classify_output_stake_ref(&w, CardanoEra::Conway),
            StakeRefClass::Reject(StakeRefReject::RewardAddressNotValidAsOutput)
        );
    }

    // Empty bytes -> Reject(Empty), never Null.
    #[test]
    fn empty_is_reject_not_null() {
        assert_eq!(
            classify_output_stake_ref(&[], CardanoEra::Conway),
            StakeRefClass::Reject(StakeRefReject::Empty)
        );
    }

    // Unknown address type (9-13) -> Reject(UnknownAddressType).
    #[test]
    fn unknown_type_is_reject() {
        let v = vec![9u8 << 4, 0x00, 0x00];
        assert_eq!(
            classify_output_stake_ref(&v, CardanoEra::Conway),
            StakeRefClass::Reject(StakeRefReject::UnknownAddressType)
        );
    }

    // THE load-bearing distinction: a malformed-but-PREFIX-VALID base address (a
    // recognised type-0 header but a truncated body) is Reject(MalformedBase),
    // NEVER silently Null. Null is for valid non-staking forms only.
    #[test]
    fn malformed_but_prefix_valid_base_is_reject_not_null() {
        // header says base (type 0) but only 10 bytes -> structurally invalid base.
        let mut v = vec![0u8 << 4];
        v.extend(std::iter::repeat(0x00).take(9));
        let got = classify_output_stake_ref(&v, CardanoEra::Conway);
        assert_eq!(got, StakeRefClass::Reject(StakeRefReject::MalformedBase));
        assert_ne!(got, StakeRefClass::Null, "malformed must stay distinct from Null");
    }

    // A pointer with a truncated final varint -> Reject(MalformedPointer), pre-Conway.
    #[test]
    fn pointer_truncated_varint_is_reject_pre_conway() {
        // certIx varint never terminates (high bit set, then EOF).
        let a = pointer_addr(4, &[0x01, 0x02, 0x81]);
        assert_eq!(
            classify_output_stake_ref(&a, CardanoEra::Babbage),
            StakeRefClass::Reject(StakeRefReject::MalformedPointer)
        );
    }

    // A pointer with trailing bytes after the third coordinate -> Reject, pre-Conway.
    #[test]
    fn pointer_trailing_bytes_is_reject_pre_conway() {
        let a = pointer_addr(4, &[0x01, 0x02, 0x03, 0x04]);
        assert_eq!(
            classify_output_stake_ref(&a, CardanoEra::Babbage),
            StakeRefClass::Reject(StakeRefReject::MalformedPointer)
        );
    }

    // A pointer varint whose magnitude exceeds u64 -> Reject (not a silently
    // wrapped value), pre-Conway. 11 max continuation groups overflow u64.
    #[test]
    fn pointer_overflow_varint_is_reject_pre_conway() {
        let mut tail = vec![0xFFu8; 11]; // 11 * 7 = 77 bits of continuation -> overflow
        tail.push(0x00); // terminator for the (already-overflowed) slot varint
        tail.extend_from_slice(&[0x00, 0x00]); // txIx, certIx (unreached)
        let a = pointer_addr(4, &tail);
        assert_eq!(
            classify_output_stake_ref(&a, CardanoEra::Babbage),
            StakeRefClass::Reject(StakeRefReject::MalformedPointer)
        );
    }

    // A pointer too short to even hold header+payment -> Reject, pre-Conway.
    #[test]
    fn pointer_too_short_is_reject_pre_conway() {
        let a = pointer_addr(4, &[]); // 29 bytes: header+payment, no varints
        // 29 bytes: b.len()==29, b[29..] is empty -> decode_varint EOF -> None -> Reject.
        assert_eq!(
            classify_output_stake_ref(&a, CardanoEra::Babbage),
            StakeRefClass::Reject(StakeRefReject::MalformedPointer)
        );
    }

    // Tiny hex helper for the real-address fixture.
    fn hx(s: &str) -> Vec<u8> {
        (0..s.len())
            .step_by(2)
            .map(|i| u8::from_str_radix(&s[i..i + 2], 16).unwrap())
            .collect()
    }

    // MAC #8 — REAL preview addresses (bech32-decoded from the live preview UTxO
    // dump) classify correctly with ZERO Reject. Synthetic vectors miss real
    // wire-format quirks; this is the real-interop smoke. The credentials are
    // asserted exactly for the base forms (proving the [29..57] staking-part
    // extraction is right on real data), and the distribution is sane (base +
    // enterprise, no Reject).
    #[test]
    fn real_preview_addresses_classify_without_reject() {
        // type 0 base (payment key, stake key): assert the EXACT staking key hash.
        let t0 = hx("001b9cdd4e5a6cb08501e46e10551b7a859d4a98251009bb91d69785f65691d68ad87582fc89b9ac43fd0227cfa4108efb791b9987b290a9ba");
        assert_eq!(
            classify_output_stake_ref(&t0, CardanoEra::Conway),
            StakeRefClass::Base(StakeCredential::KeyHash(Hash28(hx(
                "5691d68ad87582fc89b9ac43fd0227cfa4108efb791b9987b290a9ba"
            ).try_into().unwrap())))
        );
        // type 1 base (payment script, stake KEY).
        let t1 = hx("10cfad1914b599d18bffd14d2bbd696019c2899cbdd6a03325cdf680bcbeafa5e778050f2f14f8c9388ac290d5bf085ea2a25b9a8fc4e38f0f");
        assert!(matches!(
            classify_output_stake_ref(&t1, CardanoEra::Conway),
            StakeRefClass::Base(StakeCredential::KeyHash(_))
        ));
        // type 3 base (payment script, stake SCRIPT).
        let t3 = hx("30ff2cca17bc0efec555f62304a17043ace26c93dac7b66b0fab01521c3a888d65f16790950a72daee1f63aa05add6d268434107cfa5b67712");
        assert!(matches!(
            classify_output_stake_ref(&t3, CardanoEra::Conway),
            StakeRefClass::Base(StakeCredential::ScriptHash(_))
        ));
        // type 6 enterprise (payment key, no stake) -> Null.
        let t6 = hx("60986cdecfc4f555a8605d621505a4a82c25c574f59fd0b79e2acdaf02");
        assert_eq!(classify_output_stake_ref(&t6, CardanoEra::Conway), StakeRefClass::Null);
        // type 7 enterprise (payment script, no stake) -> Null.
        let t7 = hx("70bbaf8ca900440d2bf53afe3697d041392cb3cac4bc509ada17b8f8da");
        assert_eq!(classify_output_stake_ref(&t7, CardanoEra::Conway), StakeRefClass::Null);

        // None of the real samples are Reject.
        for a in [&t0, &t1, &t3, &t6, &t7] {
            assert!(
                !matches!(classify_output_stake_ref(a, CardanoEra::Conway), StakeRefClass::Reject(_)),
                "a real preview address must not be Rejected"
            );
        }
    }

    // Determinism: same (bytes, era) -> byte-identical result across calls.
    #[test]
    fn classification_is_deterministic() {
        let a = base_addr(2, 0x11, 0x33);
        let r1 = classify_output_stake_ref(&a, CardanoEra::Conway);
        let r2 = classify_output_stake_ref(&a, CardanoEra::Conway);
        assert_eq!(r1, r2);
    }
}
