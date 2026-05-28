// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types
// - Explicit state transitions only
// - Canonical serialization for all persisted/hashed data
//
//! BLUE — the single workspace authority for the N2N tag-24
//! CBOR-in-CBOR byte envelope (`CN-WIRE-08`).
//!
//! Cardano's Ouroboros N2N protocols carry serialised blocks and
//! headers as *CBOR-in-CBOR*: the inner CBOR item is serialised, then
//! wrapped in a CBOR tag-24 (`#6.24`, "encoded CBOR data item") byte
//! string. tag(24) is the two bytes `0xd8 0x18`.
//!
//! This module owns the wrap/unwrap of that byte envelope and NOTHING
//! else — it has no protocol knowledge. Per-protocol composition (where
//! the era tag sits relative to the wrap) lives in the `ade_network`
//! codecs:
//!   - BlockFetch `MsgBlock` payload = `tag24(bytes([era, block]))`
//!     (the era is *inside* the wrapped bytes).
//!   - ChainSync `RollForward` header = `[era_tag, tag24(bytes(header))]`
//!     (the era tag sits *outside* the wrap).
//!
//! Every N2N tag-24 wrap and unwrap — serve-side and receive-side —
//! routes through `wrap_tag24` / `unwrap_tag24`. No hand-rolled tag-24
//! parsing may exist elsewhere (`ci_check_tag24_wire_authority.sh`).

use crate::cbor::{self, IntWidth};

/// tag(24) on the wire is the two bytes `0xd8 0x18`.
const TAG24_FIRST: u8 = 0xd8;
const TAG24_SECOND: u8 = 0x18;

/// Closed failure sum for `unwrap_tag24`. Carries only non-secret wire
/// primitives. `unwrap_tag24` fails closed on every malformed input —
/// there is no lenient or partial-accept path.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TagEnvelopeError {
    /// The bytes do not begin with the tag-24 marker (`0xd8 0x18`), or
    /// are shorter than the two marker bytes.
    NotTag24 { first_byte: Option<u8> },
    /// The item following the tag-24 marker is not a definite-length
    /// CBOR byte string.
    NotByteString { offset: usize },
    /// The declared inner byte-string length runs past the end of the
    /// buffer.
    Truncated { offset: usize, needed: usize },
    /// Bytes remain after the single wrapped item — a tag-24 envelope
    /// must consume its whole input exactly.
    TrailingBytes { consumed: usize, total: usize },
}

/// Wrap `inner` in a tag-24 CBOR-in-CBOR byte envelope:
/// `0xd8 0x18 ‖ bytes_header(len) ‖ inner`.
///
/// Pure deterministic transform over accepted in-memory byte slices;
/// allocation failure is not a semantic branch. The inner bytes are
/// copied verbatim — never re-encoded — so a hash-bearing payload's
/// bytes survive the wrap unchanged.
pub fn wrap_tag24(inner: &[u8]) -> Vec<u8> {
    let mut buf = Vec::with_capacity(inner.len() + 5);
    // tag value 24 (> 23) encodes canonically as the 1-byte form
    // `0xd8 0x18`.
    cbor::write_tag(&mut buf, 24, IntWidth::I8);
    // canonical definite-length byte string carrying `inner` verbatim.
    cbor::write_bytes_canonical(&mut buf, inner);
    buf
}

/// Strip the tag-24 CBOR-in-CBOR envelope and return the inner bytes as
/// a zero-copy borrow of `wire`.
///
/// Total over byte slices. Fails closed (typed error, no panic) on:
/// a missing `0xd8 0x18` marker, a non-byte-string payload, a declared
/// length past the buffer end, or trailing bytes after the wrapped item.
/// The returned slice aliases `wire` — the inner bytes are never copied
/// or re-encoded.
pub fn unwrap_tag24(wire: &[u8]) -> Result<&[u8], TagEnvelopeError> {
    if wire.len() < 2 || wire[0] != TAG24_FIRST || wire[1] != TAG24_SECOND {
        return Err(TagEnvelopeError::NotTag24 {
            first_byte: wire.first().copied(),
        });
    }
    // Parse via the shared ade_codec primitives — no second CBOR parser.
    let mut offset = 0usize;
    // `read_tag` re-validates the marker and advances past it.
    let (tag, _w) = cbor::read_tag(wire, &mut offset).map_err(|_| TagEnvelopeError::NotTag24 {
        first_byte: wire.first().copied(),
    })?;
    if tag != 24 {
        return Err(TagEnvelopeError::NotTag24 {
            first_byte: wire.first().copied(),
        });
    }
    let bytes_start = offset;
    let (inner, _w) = cbor::read_bytes(wire, &mut offset).map_err(|e| match e {
        crate::error::CodecError::UnexpectedEof { offset, needed } => {
            TagEnvelopeError::Truncated { offset, needed }
        }
        _ => TagEnvelopeError::NotByteString { offset: bytes_start },
    })?;
    if offset != wire.len() {
        return Err(TagEnvelopeError::TrailingBytes {
            consumed: offset,
            total: wire.len(),
        });
    }
    // Reborrow the inner content as a zero-copy slice of `wire`:
    // `read_bytes` advanced `offset` to the end of the content, whose
    // length is `inner.len()`.
    let inner_len = inner.len();
    Ok(&wire[offset - inner_len..offset])
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
#[allow(clippy::expect_used)]
#[allow(clippy::panic)]
mod tests {
    use super::*;

    /// Inner-byte lengths straddling every CBOR length-class boundary,
    /// so the byte-string header width (inline / 1 / 2 / 4 bytes) is
    /// exercised end to end.
    fn boundary_lengths() -> Vec<usize> {
        vec![0, 1, 23, 24, 255, 256, 65535, 65536]
    }

    fn sample(len: usize) -> Vec<u8> {
        (0..len).map(|i| (i % 251) as u8).collect()
    }

    #[test]
    fn wrap_then_unwrap_is_identity_across_length_classes() {
        for len in boundary_lengths() {
            let inner = sample(len);
            let wire = wrap_tag24(&inner);
            let back = unwrap_tag24(&wire).expect("unwrap");
            assert_eq!(back, &inner[..], "round-trip mismatch at len {len}");
        }
    }

    #[test]
    fn wrap_emits_canonical_tag24_marker_and_length() {
        // 1006-byte inner (the captured preprod block inner length)
        // must encode as `d8 18 59 03 ee ...` — exactly the real
        // cardano-node 11.0.1 framing.
        let inner = sample(1006);
        let wire = wrap_tag24(&inner);
        assert_eq!(&wire[0..5], &[0xd8, 0x18, 0x59, 0x03, 0xee]);
        assert_eq!(&wire[5..], &inner[..]);
    }

    #[test]
    fn unwrap_returns_zero_copy_borrow_of_input() {
        let inner = sample(40);
        let wire = wrap_tag24(&inner);
        let back = unwrap_tag24(&wire).unwrap();
        // The returned slice must alias the wire buffer, not a copy.
        let wire_ptr = wire.as_ptr() as usize;
        let back_ptr = back.as_ptr() as usize;
        assert!(back_ptr > wire_ptr && back_ptr < wire_ptr + wire.len());
    }

    #[test]
    fn unwrap_rejects_missing_tag24_marker() {
        // A bare byte string (no tag) — `0x44 aa bb cc dd`.
        let bare = vec![0x44, 0xaa, 0xbb, 0xcc, 0xdd];
        assert!(matches!(
            unwrap_tag24(&bare),
            Err(TagEnvelopeError::NotTag24 { .. })
        ));
        // Empty input.
        assert!(matches!(
            unwrap_tag24(&[]),
            Err(TagEnvelopeError::NotTag24 { first_byte: None })
        ));
        // A different tag (tag 23 = `0xd7`).
        assert!(matches!(
            unwrap_tag24(&[0xd7, 0x41, 0x00]),
            Err(TagEnvelopeError::NotTag24 { .. })
        ));
    }

    #[test]
    fn unwrap_rejects_non_byte_string_payload() {
        // tag(24) wrapping an array, not a byte string: `d8 18 80`.
        assert!(matches!(
            unwrap_tag24(&[0xd8, 0x18, 0x80]),
            Err(TagEnvelopeError::NotByteString { .. })
        ));
    }

    #[test]
    fn unwrap_rejects_truncated_inner() {
        // tag(24), byte string declaring 4 bytes but only 2 present.
        assert!(matches!(
            unwrap_tag24(&[0xd8, 0x18, 0x44, 0xaa, 0xbb]),
            Err(TagEnvelopeError::Truncated { .. })
        ));
    }

    #[test]
    fn unwrap_rejects_huge_declared_length_without_panic() {
        // Adversarial wire input: a tag-24 byte string whose LENGTH
        // ARGUMENT (not inline) declares a gigantic size with no content.
        // Every CBOR length class must fail closed with a typed error —
        // never an integer-overflow / slice-bounds panic (a remote DoS
        // on untrusted peer input). Covers the 2-/4-/8-byte length args.
        let adversarial: &[&[u8]] = &[
            &[0xd8, 0x18, 0x59, 0xff, 0xff], // 0x59 = 2-byte len = 65535
            &[0xd8, 0x18, 0x5a, 0xff, 0xff, 0xff, 0xff], // 0x5a = 4-byte len = u32::MAX
            &[0xd8, 0x18, 0x5b, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff], // 0x5b = 8-byte len = u64::MAX
        ];
        for bytes in adversarial {
            assert!(
                matches!(unwrap_tag24(bytes), Err(TagEnvelopeError::Truncated { .. })),
                "must fail closed (no panic) on declared length {:02x?}",
                &bytes[2..3]
            );
        }
    }

    #[test]
    fn unwrap_rejects_trailing_bytes() {
        let inner = sample(3);
        let mut wire = wrap_tag24(&inner);
        wire.push(0xff); // one extra byte after the wrapped item
        assert!(matches!(
            unwrap_tag24(&wire),
            Err(TagEnvelopeError::TrailingBytes { .. })
        ));
    }

    #[test]
    fn inner_bytes_are_verbatim_not_reencoded() {
        // A non-canonically-encoded inner CBOR item must survive the
        // wrap/unwrap byte-for-byte (the authority never re-encodes the
        // payload). `0x18 0x05` is a non-minimal encoding of 5.
        let inner = vec![0x18, 0x05];
        let wire = wrap_tag24(&inner);
        let back = unwrap_tag24(&wire).unwrap();
        assert_eq!(back, &inner[..]);
    }
}
