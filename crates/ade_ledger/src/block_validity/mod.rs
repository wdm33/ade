// Closed verdict/error taxonomies for block validity + canonical CBOR for the
// replay/comparison surface. Types-only: no transition logic (B1-S4).

pub mod encoding;
pub mod verdict;

pub use encoding::{
    decode_verdict_surface, encode_verdict_surface, SurfaceDecodeError, VerdictSurface,
};
pub use verdict::{
    BlockRejectClass, BlockValidityError, BlockValidityVerdict, FieldError, FieldKind, MissingInput,
};
