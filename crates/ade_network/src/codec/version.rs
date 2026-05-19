// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types
// - Explicit state transitions only
// - Canonical serialization for all persisted/hashed data
//
// Typed per-protocol version markers (locked decision §7 #1).
//
// Each mini-protocol carries its own non-interchangeable version
// newtype so the type system rejects mixing version markers across
// protocols. Loose `u16` would let a chain-sync version be passed
// into a block-fetch transition; closed newtypes prevent that at the
// compile boundary.
//
// All newtypes wrap `u16` since the on-the-wire encoding uses CBOR
// unsigned integers that fit in `u16` for every cardano-node 10.6.2
// supported version. Versions are intentionally `Copy` — they're
// value-typed identifiers, not resources.

macro_rules! version_newtype {
    ($name:ident, $doc:literal) => {
        #[doc = $doc]
        #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
        pub struct $name(pub u16);

        impl $name {
            pub const fn new(v: u16) -> Self {
                Self(v)
            }

            pub const fn get(self) -> u16 {
                self.0
            }
        }
    };
}

version_newtype!(N2NVersion, "Selected N2N protocol version (top-level handshake outcome).");
version_newtype!(N2CVersion, "Selected N2C protocol version (top-level handshake outcome).");
version_newtype!(ChainSyncVersion, "Selected chain-sync mini-protocol version.");
version_newtype!(BlockFetchVersion, "Selected block-fetch mini-protocol version.");
version_newtype!(TxSubmission2Version, "Selected tx-submission2 mini-protocol version.");
version_newtype!(KeepAliveVersion, "Selected keep-alive mini-protocol version.");
version_newtype!(PeerSharingVersion, "Selected peer-sharing mini-protocol version.");
version_newtype!(LocalChainSyncVersion, "Selected local-chain-sync mini-protocol version.");
version_newtype!(LocalTxSubmissionVersion, "Selected local-tx-submission mini-protocol version.");
version_newtype!(LocalStateQueryVersion, "Selected local-state-query mini-protocol version.");
version_newtype!(LocalTxMonitorVersion, "Selected local-tx-monitor mini-protocol version.");
