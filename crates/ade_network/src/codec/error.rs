// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types
// - Explicit state transitions only
// - Canonical serialization for all persisted/hashed data
//
// Closed mini-protocol codec error taxonomy. Every variant carries
// only `&'static str` context — never `String`, never `anyhow`. The
// session layer can branch deterministically on the closed set of
// variants; identical inputs produce identical errors across hosts.

/// Mini-protocol identifier used in structured errors. Closed so the
/// session layer can deterministically branch on the failing protocol.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProtocolKind {
    Handshake,
    ChainSync,
    BlockFetch,
    TxSubmission2,
    KeepAlive,
    PeerSharing,
    N2cHandshake,
    LocalChainSync,
    LocalTxSubmission,
    LocalStateQuery,
    LocalTxMonitor,
}

/// Closed codec error taxonomy for `ade_network::codec`.
///
/// No `String`, no `anyhow`, no dynamic context. Context fields are
/// `&'static str` so the enum is `Eq` and equality-comparable across
/// replay runs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CodecError {
    /// Input slice ran out before the codec finished reading a required
    /// field. `needed` is the number of additional bytes the codec
    /// required at the failure point; `got` is what was available.
    Truncated { needed: usize, got: usize },

    /// CBOR variant tag is not in the closed enum for `protocol`. The
    /// `tag` is the raw discriminant integer seen on the wire.
    UnknownTag { protocol: ProtocolKind, tag: u64 },

    /// Non-UTF-8 bytes in a text field.
    InvalidUtf8 { protocol: ProtocolKind, field: &'static str },

    /// CBOR was well-formed but did not match the protocol's grammar
    /// (e.g. wrong array length, missing required field).
    InvalidProtocolMessage { protocol: ProtocolKind, reason: &'static str },

    /// Integer field value is outside the protocol-defined range.
    InvalidIntegerRange { protocol: ProtocolKind, field: &'static str, value: u64 },

    /// CBOR ingress chokepoint reported malformed CBOR while parsing a
    /// mini-protocol message. The wrapped variant preserves the
    /// underlying `ade_codec::CodecError` for the session layer.
    MalformedCbor { protocol: ProtocolKind, source: ade_codec::CodecError },
}

impl core::fmt::Display for CodecError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            CodecError::Truncated { needed, got } => {
                write!(f, "truncated: needed {needed} bytes, got {got}")
            }
            CodecError::UnknownTag { protocol, tag } => {
                write!(f, "unknown tag {tag} for protocol {protocol:?}")
            }
            CodecError::InvalidUtf8 { protocol, field } => {
                write!(f, "invalid utf-8 in {protocol:?} field '{field}'")
            }
            CodecError::InvalidProtocolMessage { protocol, reason } => {
                write!(f, "invalid {protocol:?} message: {reason}")
            }
            CodecError::InvalidIntegerRange { protocol, field, value } => {
                write!(
                    f,
                    "invalid integer range in {protocol:?} field '{field}': {value}"
                )
            }
            CodecError::MalformedCbor { protocol, source } => {
                write!(f, "malformed CBOR in {protocol:?}: {source}")
            }
        }
    }
}

impl std::error::Error for CodecError {}
