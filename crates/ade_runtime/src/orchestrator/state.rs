// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types
// - Explicit state transitions only
// - Canonical serialization for all persisted/hashed data

//! GREEN orchestrator state (PHASE4-N-K S2).
//!
//! Single canonical `ReceiveState` (one ledger, one chain_dep, one
//! pending-header cache) plus per-peer protocol bookkeeping
//! (chain-sync version + block-fetch version pair). The N2N server
//! side has its own per-peer map of `PerPeerN2nServerState`.
//!
//! No `HashMap`/`HashSet`: per-peer maps are `BTreeMap` keyed by
//! `PeerId` so iteration order is deterministic.

use std::collections::BTreeMap;

use ade_ledger::receive::ReceiveState;
use ade_network::codec::version::{BlockFetchVersion, ChainSyncVersion};
use ade_types::SlotNo;

use crate::network::n2n_server::PerPeerN2nServerState;
use crate::rollback::cadence::SnapshotCadence;

use super::event::PeerId;

/// Per-peer receive-side protocol bookkeeping. The shared
/// `ReceiveState` lives on `OrchestratorState`; this struct only
/// records the per-peer negotiated versions.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PerPeerReceiveVersions {
    pub chain_sync_version: ChainSyncVersion,
    pub block_fetch_version: BlockFetchVersion,
}

/// Pure orchestrator state. Holds the canonical ledger + chain_dep
/// (via `ReceiveState`), per-peer protocol bookkeeping, snapshot
/// cadence state, and a shutdown flag.
pub struct OrchestratorState {
    pub receive_state: ReceiveState,
    pub per_peer_receive: BTreeMap<PeerId, PerPeerReceiveVersions>,
    pub per_peer_server: BTreeMap<PeerId, PerPeerN2nServerState>,
    pub cadence: SnapshotCadence,
    pub last_persistent_snapshot_slot: Option<SlotNo>,
    pub last_observed_slot: Option<SlotNo>,
    pub shutdown_requested: bool,
}

impl OrchestratorState {
    pub fn new(receive_state: ReceiveState, cadence: SnapshotCadence) -> Self {
        Self {
            receive_state,
            per_peer_receive: BTreeMap::new(),
            per_peer_server: BTreeMap::new(),
            cadence,
            last_persistent_snapshot_slot: None,
            last_observed_slot: None,
            shutdown_requested: false,
        }
    }

    pub fn install_receive_peer(
        &mut self,
        peer_id: PeerId,
        versions: PerPeerReceiveVersions,
    ) {
        self.per_peer_receive.insert(peer_id, versions);
    }

    pub fn install_server_peer(
        &mut self,
        peer_id: PeerId,
        state: PerPeerN2nServerState,
    ) {
        self.per_peer_server.insert(peer_id, state);
    }

    pub fn remove_peer(&mut self, peer_id: PeerId) {
        self.per_peer_receive.remove(&peer_id);
        self.per_peer_server.remove(&peer_id);
    }

    pub fn is_shutdown_requested(&self) -> bool {
        self.shutdown_requested
    }
}
