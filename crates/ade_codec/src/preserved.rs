// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types
// - Explicit state transitions only
// - Canonical serialization for all persisted/hashed data

use crate::error::CodecError;
use crate::traits::{AdeEncode, CodecContext};

/// Preserved CBOR wrapper that maintains wire-byte authority.
///
/// `PreservedCbor<T>` carries three surfaces:
/// - `.wire_bytes()` — original bytes as received. Used for all hash-critical paths.
/// - `.decoded()` — reference to the decoded value.
/// - `.canonical_bytes()` — project-canonical re-encoding. NOT for hash computation.
///
/// There is no public constructor — construction only through decode chokepoints
/// within `ade_codec`. This invariant ensures that `.wire_bytes()` always reflects
/// genuine external input, never fabricated bytes.
#[derive(Debug, Clone)]
pub struct PreservedCbor<T> {
    wire: Vec<u8>,
    decoded: T,
}

impl<T> PreservedCbor<T> {
    /// Create a new `PreservedCbor` from wire bytes and a decoded value.
    ///
    /// This is `pub(crate)` — only accessible within `ade_codec` to enforce
    /// the decode chokepoint invariant.
    #[allow(dead_code)]
    pub(crate) fn new(wire: Vec<u8>, decoded: T) -> Self {
        Self { wire, decoded }
    }

    /// Original wire bytes. Used for all hash-critical computation.
    pub fn wire_bytes(&self) -> &[u8] {
        &self.wire
    }

    /// Reference to the decoded value.
    pub fn decoded(&self) -> &T {
        &self.decoded
    }

    /// Mutable reference to the decoded value.
    pub fn decoded_mut(&mut self) -> &mut T {
        &mut self.decoded
    }

    /// Consume the wrapper and return the decoded value.
    pub fn into_decoded(self) -> T {
        self.decoded
    }
}

impl<T: AdeEncode> PreservedCbor<T> {
    /// Project-canonical re-encoding. NOT for hash computation.
    ///
    /// Deterministic: same decoded value always produces the same bytes.
    /// But these bytes may differ from `.wire_bytes()` due to non-canonical
    /// wire encodings in historical blocks.
    pub fn canonical_bytes(&self, ctx: &CodecContext) -> Result<Vec<u8>, CodecError> {
        let mut buf = Vec::new();
        self.decoded.ade_encode(&mut buf, ctx)?;
        Ok(buf)
    }
}

impl<T: PartialEq> PartialEq for PreservedCbor<T> {
    fn eq(&self, other: &Self) -> bool {
        self.wire == other.wire && self.decoded == other.decoded
    }
}

impl<T: Eq> Eq for PreservedCbor<T> {}

/// Opaque non-interpreting CBOR byte carrier.
///
/// `RawCbor` stores raw CBOR bytes without semantic interpretation.
/// Encode = return stored bytes. Decode = store input bytes.
/// This preserves exact wire identity for substructures not yet parsed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RawCbor(pub(crate) Vec<u8>);

impl RawCbor {
    /// View the raw bytes.
    pub fn as_bytes(&self) -> &[u8] {
        &self.0
    }

    /// Consume and return the raw bytes.
    pub fn into_bytes(self) -> Vec<u8> {
        self.0
    }

    /// Length of the raw bytes.
    pub fn len(&self) -> usize {
        self.0.len()
    }

    /// Whether the raw bytes are empty.
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use crate::traits::AdeDecode;

    // A simple test type that implements AdeEncode/AdeDecode
    #[derive(Debug, Clone, PartialEq, Eq)]
    struct TestValue(u64);

    impl AdeEncode for TestValue {
        fn ade_encode(&self, buf: &mut Vec<u8>, _ctx: &CodecContext) -> Result<(), CodecError> {
            crate::cbor::write_uint_canonical(buf, self.0);
            Ok(())
        }
    }

    impl AdeDecode for TestValue {
        fn ade_decode(
            data: &[u8],
            offset: &mut usize,
            _ctx: &CodecContext,
        ) -> Result<Self, CodecError> {
            let (val, _) = crate::cbor::read_uint(data, offset)?;
            Ok(TestValue(val))
        }
    }

    #[test]
    fn preserved_wire_bytes_returned_exactly() {
        let wire = vec![0x18, 0x2a]; // non-canonical encoding of 42
        let preserved = PreservedCbor::new(wire.clone(), TestValue(42));
        assert_eq!(preserved.wire_bytes(), &wire[..]);
    }

    #[test]
    fn preserved_decoded_value() {
        let preserved = PreservedCbor::new(vec![0x18, 0x2a], TestValue(42));
        assert_eq!(preserved.decoded(), &TestValue(42));
    }

    #[test]
    fn preserved_canonical_bytes_may_differ_from_wire() {
        // Wire uses non-canonical I8 encoding for value 5
        let wire = vec![0x18, 0x05];
        let preserved = PreservedCbor::new(wire.clone(), TestValue(5));

        let ctx = CodecContext {
            era: ade_types::CardanoEra::ByronEbb,
        };
        let canonical = preserved.canonical_bytes(&ctx).unwrap();

        // Canonical should be single byte (inline)
        assert_eq!(canonical, vec![0x05]);
        // Wire bytes unchanged
        assert_eq!(preserved.wire_bytes(), &[0x18, 0x05]);
    }

    #[test]
    fn preserved_into_decoded() {
        let preserved = PreservedCbor::new(vec![0x05], TestValue(5));
        let val = preserved.into_decoded();
        assert_eq!(val, TestValue(5));
    }

    #[test]
    fn raw_cbor_as_bytes() {
        let raw = RawCbor(vec![0xde, 0xad, 0xbe, 0xef]);
        assert_eq!(raw.as_bytes(), &[0xde, 0xad, 0xbe, 0xef]);
        assert_eq!(raw.len(), 4);
        assert!(!raw.is_empty());
    }

    #[test]
    fn raw_cbor_empty() {
        let raw = RawCbor(vec![]);
        assert!(raw.is_empty());
        assert_eq!(raw.len(), 0);
    }

    #[test]
    fn preserved_equality_requires_both_wire_and_decoded() {
        let a = PreservedCbor::new(vec![0x05], TestValue(5));
        let b = PreservedCbor::new(vec![0x05], TestValue(5));
        let c = PreservedCbor::new(vec![0x18, 0x05], TestValue(5)); // different wire
        assert_eq!(a, b);
        assert_ne!(a, c);
    }
}
