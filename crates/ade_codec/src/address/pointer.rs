// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types
// - Explicit state transitions only
// - Canonical serialization for all persisted/hashed data

//! EPOCH-CONSENSUS-VIEW S3a (DC-EVIEW-03) — the era-parameterized pointer-address
//! varint decoder, matching cardano-ledger EXACTLY.
//!
//! A pointer address (header type 4/5) is `header(1) ‖ payment(28) ‖ 3 base-128
//! big-endian varints (slot, txIx, certIx)`. cardano-ledger's stored shape is
//! `(u32 slot, u16 txIx, u16 certIx)`. CIP-19 is SILENT on canonicality → the
//! cardano-ledger implementation is the SOLE authority, and its strict check is a
//! bit-WIDTH check, NOT a minimal-form check. The rule is ERA-PARAMETERIZED on the
//! block's bound protocol-major (here a typed [`CardanoEra`], never inferred from
//! the bytes / config / clock):
//!
//! | Era (protocol major)   | over-width varint                       | bounded leading-zero alias | trailing bytes |
//! |------------------------|-----------------------------------------|----------------------------|----------------|
//! | Conway (9+)            | REJECT (bounded groups + width check)   | ACCEPT                     | REJECT         |
//! | Babbage (7-8)          | NORMALIZE: clamp whole 3-tuple to 0      | ACCEPT                     | REJECT         |
//! | <=Alonzo (2-6)         | NORMALIZE: clamp whole 3-tuple to 0      | ACCEPT                     | accept + crop  |
//!
//! Bounded non-minimal (leading-zero-group) encodings are ACCEPTED in every era;
//! reject-all-non-canonical would FALSE-REJECT txs cardano-node accepts. Ade must
//! NOT substitute a cleaner parser rule that diverges from network semantics.

use ade_types::CardanoEra;

/// cardano-ledger's stored pointer shape: `(u32 slot, u16 txIx, u16 certIx)`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Ptr {
    pub slot: u32,
    pub tx_index: u16,
    pub cert_index: u16,
}

/// Why a pointer address fails to decode (Conway/Babbage strictness). Distinct
/// reasons so a divergence can be pinpointed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PointerDecodeError {
    /// The header byte's type nibble is not a pointer (4 or 5).
    NotAPointerAddress,
    /// Fewer than header(1) + payment(28) = 29 bytes (no room for the pointer tail).
    TooShort,
    /// A varint's continuation bit is set at end-of-input.
    TruncatedVarint,
    /// Conway: a coordinate exceeds its field width, or a continuation runs past
    /// the maximum group count ("too many bytes" / "More than N bits").
    OverWidth,
    /// Conway / Babbage: bytes remain after the 3rd coordinate.
    TrailingBytes,
}

/// Decode a pointer ADDRESS's coordinates per the bound era's cardano-ledger rule.
/// Validates the header is a pointer type and that the tail exists, then decodes.
pub fn decode_pointer_address(addr_bytes: &[u8], era: CardanoEra) -> Result<Ptr, PointerDecodeError> {
    if addr_bytes.is_empty() {
        return Err(PointerDecodeError::TooShort);
    }
    let addr_type = addr_bytes[0] >> 4;
    if addr_type != 4 && addr_type != 5 {
        return Err(PointerDecodeError::NotAPointerAddress);
    }
    if addr_bytes.len() < 29 {
        return Err(PointerDecodeError::TooShort);
    }
    decode_pointer_tail(&addr_bytes[29..], era)
}

/// Decode the 3 base-128 big-endian varints of a pointer tail (the bytes after
/// `header(1) ‖ payment(28)`), era-gated.
pub fn decode_pointer_tail(tail: &[u8], era: CardanoEra) -> Result<Ptr, PointerDecodeError> {
    if era >= CardanoEra::Conway {
        decode_pointer_strict(tail)
    } else {
        decode_pointer_normalized(tail, era)
    }
}

/// Conway (PV9+): width-bounded varints, reject over-width / over-long / trailing.
fn decode_pointer_strict(tail: &[u8]) -> Result<Ptr, PointerDecodeError> {
    let mut pos = 0usize;
    let slot = decode_width_bounded(tail, &mut pos, 32)?;
    let tx = decode_width_bounded(tail, &mut pos, 16)?;
    let cert = decode_width_bounded(tail, &mut pos, 16)?;
    if pos != tail.len() {
        return Err(PointerDecodeError::TrailingBytes);
    }
    Ok(Ptr {
        slot: slot as u32,
        tx_index: tx as u16,
        cert_index: cert as u16,
    })
}

/// A width-bounded base-128 big-endian varint (cardano-ledger `decodeVariableLengthWordN`):
/// at most `ceil(bits/7)` groups; a continuation past the max group count is
/// `OverWidth` ("too many bytes"); when the max group count is used, the
/// most-significant (first) group's surplus data bits above the field width must be
/// 0, else `OverWidth` ("More than N bits"). Bounded in-range non-minimal encodings
/// are ACCEPTED (it is a width check, not a minimal-form check).
fn decode_width_bounded(
    tail: &[u8],
    pos: &mut usize,
    bits: u32,
) -> Result<u64, PointerDecodeError> {
    let max_groups = (bits + 6) / 7; // ceil(bits/7): 32 -> 5, 16 -> 3
    let ms_bits = bits - 7 * (max_groups - 1); // data bits the MS group may use
    let allowed: u8 = ((1u16 << ms_bits) - 1) as u8; // low ms_bits set
    let surplus_mask: u8 = 0x7f & !allowed; // data-bit positions above the width
    let first_off = *pos;
    let mut result: u64 = 0;
    let mut groups = 0u32;
    loop {
        let byte = *tail.get(*pos).ok_or(PointerDecodeError::TruncatedVarint)?;
        *pos += 1;
        groups += 1;
        result = (result << 7) | (byte & 0x7f) as u64;
        if byte & 0x80 == 0 {
            // Terminated. When the full group count is used, the MS group (the first
            // byte, big-endian) may not set data bits above the field width.
            if groups == max_groups && (tail[first_off] & surplus_mask) != 0 {
                return Err(PointerDecodeError::OverWidth);
            }
            return Ok(result);
        }
        if groups == max_groups {
            // Continuation still set after the maximum group count.
            return Err(PointerDecodeError::OverWidth);
        }
    }
}

/// Babbage / <=Alonzo: decode each coordinate as a u64 with a WRAPPING shift (bits
/// past 64 silently dropped, matching cardano-ledger's `decodeVariableLengthWord64`),
/// then `mkPtrNormalized`: if ANY coordinate overflows its field width, clamp the
/// WHOLE 3-tuple to (0,0,0). Babbage rejects trailing bytes; <=Alonzo crops them.
fn decode_pointer_normalized(tail: &[u8], era: CardanoEra) -> Result<Ptr, PointerDecodeError> {
    let mut pos = 0usize;
    let slot = decode_u64_wrapping(tail, &mut pos)?;
    let tx = decode_u64_wrapping(tail, &mut pos)?;
    let cert = decode_u64_wrapping(tail, &mut pos)?;
    if pos != tail.len() && era >= CardanoEra::Babbage {
        return Err(PointerDecodeError::TrailingBytes);
    }
    Ok(normalize_ptr(slot, tx, cert))
}

/// base-128 big-endian varint into u64, WRAPPING (bits past 64 silently dropped),
/// unbounded group count -- matches cardano-ledger's `decodeVariableLengthWord64`.
fn decode_u64_wrapping(tail: &[u8], pos: &mut usize) -> Result<u64, PointerDecodeError> {
    let mut result: u64 = 0;
    loop {
        let byte = *tail.get(*pos).ok_or(PointerDecodeError::TruncatedVarint)?;
        *pos += 1;
        result = (result << 7) | (byte & 0x7f) as u64; // wrapping: drops bits past 64
        if byte & 0x80 == 0 {
            return Ok(result);
        }
    }
}

/// cardano-ledger `mkPtrNormalized`: if ANY of slot/txIx/certIx overflows its field
/// width, the WHOLE pointer becomes (0,0,0); else all three are kept unmodified.
/// NOT per-field masking, NOT wrapping.
fn normalize_ptr(slot: u64, tx: u64, cert: u64) -> Ptr {
    if slot <= u32::MAX as u64 && tx <= u16::MAX as u64 && cert <= u16::MAX as u64 {
        Ptr {
            slot: slot as u32,
            tx_index: tx as u16,
            cert_index: cert as u16,
        }
    } else {
        Ptr {
            slot: 0,
            tx_index: 0,
            cert_index: 0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // A pointer tail with single-byte coords.
    fn tail(slot: &[u8], tx: &[u8], cert: &[u8]) -> Vec<u8> {
        let mut v = Vec::new();
        v.extend_from_slice(slot);
        v.extend_from_slice(tx);
        v.extend_from_slice(cert);
        v
    }

    // === alias acceptance (all eras) — the no-canonicalization-override guard ===
    #[test]
    fn bounded_leading_zero_alias_accepted_all_eras() {
        // [0x80,0x01] is the leading-zero alias of [0x01] (value 1). cardano-ledger
        // ACCEPTS it (width check, not minimal-form). Reject would diverge.
        let t = tail(&[0x80, 0x01], &[0x02], &[0x03]);
        for era in [CardanoEra::Alonzo, CardanoEra::Babbage, CardanoEra::Conway] {
            assert_eq!(
                decode_pointer_tail(&t, era),
                Ok(Ptr { slot: 1, tx_index: 2, cert_index: 3 }),
                "era {era:?} must ACCEPT the bounded leading-zero alias"
            );
        }
    }

    // === Conway (strict) ===
    #[test]
    fn conway_decodes_in_range() {
        let t = tail(&[0x01], &[0x02], &[0x03]);
        assert_eq!(
            decode_pointer_tail(&t, CardanoEra::Conway),
            Ok(Ptr { slot: 1, tx_index: 2, cert_index: 3 })
        );
    }

    #[test]
    fn conway_rejects_txix_over_u16() {
        // txIx = 4 groups -> "too many bytes" (Word16 max 3 groups).
        let t = tail(&[0x01], &[0x81, 0x80, 0x80, 0x00], &[0x03]);
        assert_eq!(
            decode_pointer_tail(&t, CardanoEra::Conway),
            Err(PointerDecodeError::OverWidth)
        );
    }

    #[test]
    fn conway_rejects_width_overflow_within_max_groups() {
        // txIx = 3 groups but the MS group sets surplus data bits (> 16 bits).
        // first byte 0x84 -> 0x84 & 0b1111100 = 0b0000100 != 0 -> OverWidth.
        let t = tail(&[0x01], &[0x84, 0x80, 0x00], &[0x03]);
        assert_eq!(
            decode_pointer_tail(&t, CardanoEra::Conway),
            Err(PointerDecodeError::OverWidth)
        );
    }

    #[test]
    fn conway_rejects_slot_over_u32() {
        // slot = 6 groups -> too many for Word32 (max 5).
        let t = tail(&[0x81, 0x80, 0x80, 0x80, 0x80, 0x00], &[0x02], &[0x03]);
        assert_eq!(
            decode_pointer_tail(&t, CardanoEra::Conway),
            Err(PointerDecodeError::OverWidth)
        );
    }

    #[test]
    fn conway_rejects_trailing_bytes() {
        let t = tail(&[0x01], &[0x02], &[0x03, 0x04]);
        assert_eq!(
            decode_pointer_tail(&t, CardanoEra::Conway),
            Err(PointerDecodeError::TrailingBytes)
        );
    }

    #[test]
    fn conway_accepts_max_width_boundary() {
        // txIx = u16::MAX = 65535 = 3 groups [0x83,0xFF,0x7F]: 0b11 1111111 1111111.
        // first byte 0x83 & 0b1111100 = 0 -> accepted.
        let t = tail(&[0x01], &[0x83, 0xFF, 0x7F], &[0x03]);
        assert_eq!(
            decode_pointer_tail(&t, CardanoEra::Conway),
            Ok(Ptr { slot: 1, tx_index: 65535, cert_index: 3 })
        );
    }

    // === Babbage (normalize + reject trailing) ===
    #[test]
    fn babbage_normalizes_overflow_to_zero_tuple() {
        // txIx overflows u16 (5 groups, huge) -> the WHOLE tuple clamps to (0,0,0).
        let t = tail(&[0x09], &[0x81, 0x80, 0x80, 0x80, 0x00], &[0x07]);
        assert_eq!(
            decode_pointer_tail(&t, CardanoEra::Babbage),
            Ok(Ptr { slot: 0, tx_index: 0, cert_index: 0 })
        );
    }

    #[test]
    fn babbage_in_range_kept_unmodified() {
        let t = tail(&[0x09], &[0x02], &[0x07]);
        assert_eq!(
            decode_pointer_tail(&t, CardanoEra::Babbage),
            Ok(Ptr { slot: 9, tx_index: 2, cert_index: 7 })
        );
    }

    #[test]
    fn babbage_rejects_trailing_bytes() {
        let t = tail(&[0x01], &[0x02], &[0x03, 0xFF]);
        assert_eq!(
            decode_pointer_tail(&t, CardanoEra::Babbage),
            Err(PointerDecodeError::TrailingBytes)
        );
    }

    // === <=Alonzo (normalize + crop trailing) ===
    #[test]
    fn alonzo_normalizes_overflow_to_zero_tuple() {
        let t = tail(&[0x09], &[0x81, 0x80, 0x80, 0x80, 0x00], &[0x07]);
        assert_eq!(
            decode_pointer_tail(&t, CardanoEra::Alonzo),
            Ok(Ptr { slot: 0, tx_index: 0, cert_index: 0 })
        );
    }

    #[test]
    fn alonzo_crops_trailing_bytes() {
        // trailing bytes are ACCEPTED (cropped) pre-Babbage.
        let t = tail(&[0x01], &[0x02], &[0x03, 0xFF, 0xFF]);
        assert_eq!(
            decode_pointer_tail(&t, CardanoEra::Alonzo),
            Ok(Ptr { slot: 1, tx_index: 2, cert_index: 3 })
        );
    }

    // === truncation (all eras) ===
    #[test]
    fn truncated_varint_is_error() {
        let t = tail(&[0x01], &[0x02], &[0x81]); // certIx continuation then EOF
        assert_eq!(
            decode_pointer_tail(&t, CardanoEra::Conway),
            Err(PointerDecodeError::TruncatedVarint)
        );
        assert_eq!(
            decode_pointer_tail(&t, CardanoEra::Babbage),
            Err(PointerDecodeError::TruncatedVarint)
        );
    }

    // === the full address wrapper ===
    #[test]
    fn decode_pointer_address_validates_header_and_tail() {
        // header 0x40 (type 4) + 28 payment + tail.
        let mut a = vec![0x40u8];
        a.extend(std::iter::repeat(0xaa).take(28));
        a.extend_from_slice(&[0x01, 0x02, 0x03]);
        assert_eq!(
            decode_pointer_address(&a, CardanoEra::Conway),
            Ok(Ptr { slot: 1, tx_index: 2, cert_index: 3 })
        );
        // a base address (type 0) is not a pointer.
        let base = vec![0x00u8; 57];
        assert_eq!(
            decode_pointer_address(&base, CardanoEra::Conway),
            Err(PointerDecodeError::NotAPointerAddress)
        );
        // too short (header but no full payment).
        let short = vec![0x40u8, 0x00, 0x00];
        assert_eq!(
            decode_pointer_address(&short, CardanoEra::Conway),
            Err(PointerDecodeError::TooShort)
        );
    }

    // determinism
    #[test]
    fn decode_is_deterministic() {
        let t = tail(&[0x81, 0x00], &[0x02], &[0x03]);
        assert_eq!(
            decode_pointer_tail(&t, CardanoEra::Conway),
            decode_pointer_tail(&t, CardanoEra::Conway)
        );
        // multi-byte slot = 128.
        assert_eq!(
            decode_pointer_tail(&t, CardanoEra::Conway),
            Ok(Ptr { slot: 128, tx_index: 2, cert_index: 3 })
        );
    }
}
