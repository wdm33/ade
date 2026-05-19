// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types
// - Explicit state transitions only
// - Canonical serialization for all persisted/hashed data

#![deny(unsafe_code)]
#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![deny(clippy::float_arithmetic)]

pub mod address;
pub mod allegra;
pub mod alonzo;
pub mod babbage;
pub mod byron;
pub mod cbor;
pub mod conway;
pub mod error;
pub mod mary;
pub mod preserved;
pub mod primitives;
pub mod shelley;
pub mod traits;

pub use error::CodecError;
pub use preserved::{PreservedCbor, RawCbor};
pub use traits::{AdeDecode, AdeEncode, CodecContext};

/// Re-exports of the CBOR primitive surface used by downstream BLUE
/// codecs (e.g. `ade_network::codec`). Centralising the surface here
/// keeps direct `minicbor::`-style imports out of every downstream
/// codec — `ci_check_ingress_chokepoints.sh` enforces that
/// constraint. The re-exports are intentionally limited to the read
/// and write helpers, the container/integer width metadata types,
/// and CBOR major-type constants.
pub mod cbor_primitives {
    pub use crate::cbor::{
        canonical_width, is_break, peek_major, read_any_int, read_array_header, read_bool,
        read_bytes, read_map_header, read_tag, read_text, read_uint, skip_item, write_array_header,
        write_bool, write_break, write_bytes, write_bytes_canonical, write_map_header, write_null,
        write_tag, write_text_canonical, write_uint, write_uint_canonical, ContainerEncoding,
        IntWidth, MAJOR_ARRAY, MAJOR_BYTES, MAJOR_MAP, MAJOR_NEGATIVE, MAJOR_SIMPLE, MAJOR_TAG,
        MAJOR_TEXT, MAJOR_UNSIGNED,
    };
}
