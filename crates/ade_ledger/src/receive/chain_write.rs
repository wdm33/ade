// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types
// - Explicit state transitions only
// - Canonical serialization for all persisted/hashed data

//! BLUE narrow ChainDb-write trait for the receive bridge
//! (PHASE4-N-H S1).
//!
//! The reducer (S2) calls `ChainDbWrite::write_admitted` to persist
//! an [`AdmittedBlock`] into a chain database. The trait takes
//! `AdmittedBlock` BY VALUE, so the receive admission gate is
//! preserved across the trait surface: a caller cannot persist raw
//! bytes — the only way to obtain an `AdmittedBlock` is via
//! [`super::admitted::admit_via_block_validity`].
//!
//! Production impl lives in `ade_runtime::receive::in_memory_chain_write`
//! (S3, GREEN) and would have a parallel `persistent_chain_write`
//! impl when the persistent ChainDb is wired (out of S1 scope).

use ade_types::{Hash32, SlotNo};

use super::admitted::AdmittedBlock;

/// Closed error sum for chain-db writes from the receive bridge.
/// Wraps the underlying ChainDb error reason without leaking the
/// runtime crate's type into BLUE.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ChainWriteError {
    /// The underlying store reported a slot conflict (different
    /// bytes attempted at an existing slot).
    SlotConflict { slot: SlotNo, hash: Hash32 },
    /// Generic store I/O / invariant error from the underlying
    /// ChainDb, lifted to a stable shape. Carries a static error
    /// kind tag rather than a String so the trait stays BLUE.
    Underlying(ChainWriteErrorKind),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChainWriteErrorKind {
    Io,
    InvalidOperation,
    Other,
}

/// Narrow trait the receive reducer calls to persist an admitted
/// block. Single method; the trait is intentionally minimal.
pub trait ChainDbWrite {
    /// Persist `block` into the underlying chain store. After this
    /// returns `Ok`, the store observes the block at its
    /// `(slot, hash)` key.
    fn write_admitted(&mut self, block: AdmittedBlock) -> Result<(), ChainWriteError>;
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
#[allow(clippy::expect_used)]
#[allow(clippy::panic)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;

    /// Hand-rolled BLUE mock: a BTreeMap-backed store that records
    /// what was written. Used by the trait-presence test below.
    #[derive(Default)]
    struct MockChainWrite {
        written: BTreeMap<usize, Vec<u8>>,
        next_idx: usize,
    }

    impl ChainDbWrite for MockChainWrite {
        fn write_admitted(&mut self, block: AdmittedBlock) -> Result<(), ChainWriteError> {
            self.written.insert(self.next_idx, block.into_bytes());
            self.next_idx += 1;
            Ok(())
        }
    }

    #[test]
    fn chain_write_trait_admits_via_admitted_block() {
        // This test asserts the trait surface compiles and accepts an
        // AdmittedBlock by value. Constructing a real AdmittedBlock
        // requires going through admit_via_block_validity (private
        // constructor); the deeper corpus-driven test lives in
        // admitted.rs alongside the corpus scaffolding. Here we
        // assert the trait + mock impl shape compile end-to-end.
        let mock = MockChainWrite::default();
        // We can't construct an AdmittedBlock from outside its module
        // here, so the trait-shape assertion is the compilation of
        // `MockChainWrite: ChainDbWrite` above — already proven by
        // the file compiling.
        assert_eq!(mock.next_idx, 0);
        let _ = &mock as &dyn ChainDbWrite; // trait-object proof
    }
}
