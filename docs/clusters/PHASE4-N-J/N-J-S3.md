# Invariant Slice — PHASE4-N-J S3

**Slice Name:** `CertState` encode/decode
**Cluster:** PHASE4-N-J
**CEs addressed:** CE-N-J-3
**Dependencies:** N-J-S1

Wire shape: array(5) of 5 BTreeMaps —
* registrations: `StakeCredential → Coin`
* delegations: `StakeCredential → PoolId`
* rewards: `StakeCredential → Coin`
* pools: `PoolId → PoolParams` (array(7))
* retiring: `PoolId → epoch`

`StakeCredential` as `array(2)[variant(0=Key,1=Script), bytes(28)]`.
`PoolId` as `bytes(28)`.
`PoolParams` as `array(7)[pool_id, vrf_hash(32), pledge, cost,
margin_array(2), reward_account, owners[bytes(28)*]]`.

Tests (4, all green):
- `cert_state_round_trip_empty`
- `cert_state_round_trip_populated`
- `cert_state_encode_deterministic_across_runs`
- `cert_state_pool_params_round_trip_with_empty_owners`

No registry flip; per-sub-state slice.
