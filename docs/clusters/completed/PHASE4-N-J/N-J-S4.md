# Invariant Slice — PHASE4-N-J S4

**Slice Name:** `EpochState` encode/decode (incl. SnapshotState)
**Cluster:** PHASE4-N-J
**CEs addressed:** CE-N-J-4
**Dependencies:** N-J-S1, N-J-S3 (PoolId helper conventions reused)

Wire shape: array(7)[epoch, slot, snapshot_state, reserves, treasury,
block_production_map, epoch_fees].

`snapshot_state` = array(3) of stake_snapshot (mark, set, go).
`stake_snapshot` = array(2)[delegations_map, pool_stakes_map].

Tests (3, all green): empty + populated round-trips + determinism.
