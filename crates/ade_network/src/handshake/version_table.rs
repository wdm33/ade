// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types
// - Explicit state transitions only
// - Canonical serialization for all persisted/hashed data
//
// cardano-node 10.6.2 supported version tables for the N2N and N2C
// handshakes. Compile-time constants — DC-PROTO-06 forbids reading
// these from a runtime global, but a `const` table is acceptable
// because it has no execution-time state.
//
// Mainnet network magic is `764824073` per `cardano-configurations`;
// the local supported tables advertise the values this node speaks.
// Tables are sorted ascending by version so selection's intersection
// rule has a single canonical ordering (DC-PROTO-05).
//
// Spec source: IOG `ouroboros-network` package, modules
// `Ouroboros.Network.NodeToNode.Version` (N2N V11..=V14) and
// `Ouroboros.Network.NodeToClient.Version` (N2C V15..=V20) at the
// cardano-node 10.6.2 tag. Re-pinning against captured frames is the
// S-A9 closure obligation (see S-A3 §17).

use crate::handshake::state::{N2cVersionData, PeerSharingFlag, VersionData};

/// Mainnet network magic per `cardano-configurations` mainnet config.
/// The N2N/N2C tables advertise this value; preprod/preview/private
/// tables can be derived from the same shape with a different magic.
pub const MAINNET_NETWORK_MAGIC: u32 = 764_824_073;

/// N2N supported versions (cardano-node 10.6.2 — Chang+).
///
/// V11 was the first version to carry the peer-sharing flag and the
/// `query` flag in the handshake-data payload. V14 is the highest
/// version cardano-node 10.6.2 advertises.
pub const N2N_SUPPORTED: &[(u16, VersionData)] = &[
    (
        11,
        VersionData {
            network_magic: MAINNET_NETWORK_MAGIC,
            initiator_only_diffusion: false,
            peer_sharing: PeerSharingFlag::NoPeerSharing,
            query: false,
        },
    ),
    (
        12,
        VersionData {
            network_magic: MAINNET_NETWORK_MAGIC,
            initiator_only_diffusion: false,
            peer_sharing: PeerSharingFlag::NoPeerSharing,
            query: false,
        },
    ),
    (
        13,
        VersionData {
            network_magic: MAINNET_NETWORK_MAGIC,
            initiator_only_diffusion: false,
            peer_sharing: PeerSharingFlag::NoPeerSharing,
            query: false,
        },
    ),
    (
        14,
        VersionData {
            network_magic: MAINNET_NETWORK_MAGIC,
            initiator_only_diffusion: false,
            peer_sharing: PeerSharingFlag::NoPeerSharing,
            query: false,
        },
    ),
];

/// N2C supported versions (cardano-node 10.6.2).
///
/// V15 first carried the `query` flag; V20 is the highest version
/// cardano-node 10.6.2 advertises.
pub const N2C_SUPPORTED: &[(u16, N2cVersionData)] = &[
    (15, N2cVersionData { network_magic: MAINNET_NETWORK_MAGIC, query: false }),
    (16, N2cVersionData { network_magic: MAINNET_NETWORK_MAGIC, query: false }),
    (17, N2cVersionData { network_magic: MAINNET_NETWORK_MAGIC, query: false }),
    (18, N2cVersionData { network_magic: MAINNET_NETWORK_MAGIC, query: false }),
    (19, N2cVersionData { network_magic: MAINNET_NETWORK_MAGIC, query: false }),
    (20, N2cVersionData { network_magic: MAINNET_NETWORK_MAGIC, query: false }),
];
