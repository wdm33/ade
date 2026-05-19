// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types
// - Explicit state transitions only
// - Canonical serialization for all persisted/hashed data
//
// cardano-node supported version tables for the N2N and N2C
// handshakes — aligned with cardano-node 11.0.1 (10.6.2 forward-
// compatible). Compile-time constants — DC-PROTO-06 forbids reading
// these from a runtime global, but a `const` table is acceptable
// because it has no execution-time state.
//
// Mainnet network magic is `764824073` per `cardano-configurations`;
// the local supported tables advertise the values this node speaks.
// Tables are sorted ascending by version so selection's intersection
// rule has a single canonical ordering (DC-PROTO-05).
//
// Spec source: IOG `ouroboros-network` package, modules
// `Cardano.Network.NodeToNode.Version` (N2N V14..=V16 in 11.0.1;
// V11..=V13 retained for backward-compat headroom) and
// `Cardano.Network.NodeToClient.Version` (N2C V16..=V23 in 11.0.1;
// V15 was dropped at cardano-node 10.2). Re-pinning against captured
// frames is the S-A9 closure obligation (see S-A3 §17).
//
// `peras_support` was introduced at NodeToNodeV_16 (cardano-node
// 11.0.1). We default it to `false` across all entries; Peras
// consensus integration is a future cluster.

use crate::handshake::state::{N2cVersionData, PeerSharingFlag, VersionData};

/// Mainnet network magic per `cardano-configurations` mainnet config.
/// The N2N/N2C tables advertise this value; preprod/preview/private
/// tables can be derived from the same shape with a different magic.
pub const MAINNET_NETWORK_MAGIC: u32 = 764_824_073;

const fn n2n(version_magic: u32) -> VersionData {
    VersionData {
        network_magic: version_magic,
        initiator_only_diffusion: false,
        peer_sharing: PeerSharingFlag::NoPeerSharing,
        query: false,
        peras_support: false,
    }
}

/// N2N supported versions — cardano-node 11.0.1 advertises V14..V16;
/// we retain V11..V13 for backward-compat headroom against older relays
/// that may still negotiate down. V16 is the first version to carry the
/// `perasSupport` field in NodeToNodeVersionData.
pub const N2N_SUPPORTED: &[(u16, VersionData)] = &[
    (11, n2n(MAINNET_NETWORK_MAGIC)),
    (12, n2n(MAINNET_NETWORK_MAGIC)),
    (13, n2n(MAINNET_NETWORK_MAGIC)),
    (14, n2n(MAINNET_NETWORK_MAGIC)),
    (15, n2n(MAINNET_NETWORK_MAGIC)),
    (16, n2n(MAINNET_NETWORK_MAGIC)),
];

const fn n2c(version_magic: u32) -> N2cVersionData {
    N2cVersionData {
        network_magic: version_magic,
        query: false,
    }
}

/// N2C supported versions — cardano-node 11.0.1 advertises V16..V23.
/// V15 was dropped at cardano-node 10.2 and is no longer in our
/// advertised set. V22 added LSQ SRV records support and V23 updated
/// LedgerPeerSnapshot query encoding — both opaque at this layer
/// (LSQ Query/Result payloads are passed verbatim per cluster TCB
/// rule on the n2c module).
pub const N2C_SUPPORTED: &[(u16, N2cVersionData)] = &[
    (16, n2c(MAINNET_NETWORK_MAGIC)),
    (17, n2c(MAINNET_NETWORK_MAGIC)),
    (18, n2c(MAINNET_NETWORK_MAGIC)),
    (19, n2c(MAINNET_NETWORK_MAGIC)),
    (20, n2c(MAINNET_NETWORK_MAGIC)),
    (21, n2c(MAINNET_NETWORK_MAGIC)),
    (22, n2c(MAINNET_NETWORK_MAGIC)),
    (23, n2c(MAINNET_NETWORK_MAGIC)),
];
