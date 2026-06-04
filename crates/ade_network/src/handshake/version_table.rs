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

use crate::codec::handshake::VersionParams;
use crate::codec::primitives::{encode_array_header, encode_bool, encode_u64};
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

/// PHASE4-N-F-G-H S2b: the N2N supported-version table for an arbitrary
/// configured network magic. The closed version SET is exactly
/// `N2N_SUPPORTED`'s version numbers (V11..=V16 — the single source, no
/// duplication) and the `VersionData` shape is identical; ONLY `network_magic`
/// is derived from the configured network identity (DC-NODE-07). Live serve
/// listeners build this from the configured magic instead of the static
/// mainnet `N2N_SUPPORTED`, so a non-mainnet (preprod magic 1 / private magic
/// 42) follower's N2N handshake succeeds. Additive + pure + deterministic; no
/// version widening, no version-set change, no new canonical type.
pub fn n2n_supported_for_magic(network_magic: u32) -> Vec<(u16, VersionData)> {
    N2N_SUPPORTED
        .iter()
        .map(|(version, _)| (*version, n2n(network_magic)))
        .collect()
}

/// PHASE4-N-F-G-L (CN-WIRE-10): the SINGLE closed N2N `versionData` wire encoding, per
/// negotiated version. V11..=15 emit the 4-field `NodeToNodeVersionData`
/// `[networkMagic, initiatorAndResponderDiffusionMode, peerSharing, query]`; V16+ append
/// `perasSupport`. This is the SHARED authority used by BOTH the initiator
/// (`build_n2n_version_table`) and the serve RESPONDER (the session handshake driver), so the two
/// directions cannot diverge. The emitted shape is byte-pinned against the captured real
/// cardano-node V15 fixture (`corpus/network/n2n/handshake/*_v11_v16_propose_recv.cbor`:
/// `[1, 15, [magic, true, 0, false]]`). Values match the proven initiator path
/// (`initiatorAndResponderDiffusionMode = true`, `peerSharing = NoPeerSharing (0)`, `query = false`,
/// `perasSupport = false`). Closed grammar — never a placeholder / fallback.
pub fn encode_n2n_version_params(version: u16, network_magic: u32) -> VersionParams {
    let mut buf = Vec::new();
    let field_count: u64 = if version >= 16 { 5 } else { 4 };
    encode_array_header(&mut buf, field_count);
    encode_u64(&mut buf, network_magic as u64);
    encode_bool(&mut buf, true); // initiatorAndResponderDiffusionMode
    encode_u64(&mut buf, 0); // peerSharing = NoPeerSharing
    encode_bool(&mut buf, false); // query
    if version >= 16 {
        encode_bool(&mut buf, false); // perasSupport (V16+)
    }
    VersionParams(buf)
}

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

#[cfg(test)]
mod tests {
    use super::*;

    /// PHASE4-N-F-G-H S2b (CE-G-H-2b): the magic-aware N2N table parameterizes
    /// only `network_magic` over the unchanged closed version set — proven for
    /// preprod (1), C1 (42), and mainnet.
    #[test]
    fn n2n_supported_for_magic_produces_configured_magic() {
        let want_versions: Vec<u16> = N2N_SUPPORTED.iter().map(|(v, _)| *v).collect();
        for magic in [1u32, 42, MAINNET_NETWORK_MAGIC] {
            let table = n2n_supported_for_magic(magic);
            // Closed version SET unchanged (no widening, no version-set change).
            let got_versions: Vec<u16> = table.iter().map(|(v, _)| *v).collect();
            assert_eq!(got_versions, want_versions, "closed N2N version set unchanged");
            // Every advertised version carries the CONFIGURED magic.
            for (_, vd) in &table {
                assert_eq!(vd.network_magic, magic, "VersionData advertises the configured magic");
            }
        }
        // The mainnet specialization matches the static `N2N_SUPPORTED` table
        // (same version numbers + same advertised magic) — mainnet behavior
        // remains available when configured.
        for ((gv, gvd), (sv, svd)) in n2n_supported_for_magic(MAINNET_NETWORK_MAGIC)
            .iter()
            .zip(N2N_SUPPORTED.iter())
        {
            assert_eq!(gv, sv, "version numbers match the static table");
            assert_eq!(
                gvd.network_magic, svd.network_magic,
                "mainnet specialization advertises the static table's magic"
            );
        }
    }
}
