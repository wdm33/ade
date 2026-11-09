# SLICE ECA-0b — the leadership-complete, self-contained EpochConsensusView

Part of EPOCH-CONTINUITY-ACTIVATION. Built on ECA-0a (the cardano-faithful pool lifecycle, which makes
the window driver surface the correct window-end `{stake, pool_params}`). ECA-0b freezes the *effective*
VRF mapping + a protocol-parameter commitment into the candidate view, and makes the next-epoch
`PoolDistrView` derive EXCLUSIVELY from that sealed view — the production authority for cross-epoch leadership.

User directive (2026-06-21): the production authority is the bound EpochConsensusView =
stake distribution + exact pool VRF keys effective for that snapshot + nonce + source-chain bindings +
protocol-parameter commitment. No live CertState read at rebind; no unbound protocol-parameter read —
ASC reaches the projection ONLY through the bound commitment.

## Design

1. **`EpochConsensusView` gains two fields** (`reduced_epoch_view.rs`), folded into `canonical_hash`:
   - `pool_vrf_keyhashes: BTreeMap<PoolId, Hash32>` — payload (like `stake_by_pool`).
   - `protocol_params_commitment: Hash32` — a binding (added to `ViewBindings` + `matches`).
   Plus `is_leadership_complete()` (the key sets of `stake_by_pool` and `pool_vrf_keyhashes` are equal)
   and getters. `matches` additionally requires `is_leadership_complete()` + the param commitment, so an
   incomplete or wrong-params view is INERT.

2. **`protocol_params_commitment`** = `blake2b(genesis_hash ‖ protocol_params_hash ‖ asc_numer ‖ asc_denom)`
   — the FULL consensus profile (user correction 2026-06-21: bind a canonical consensus-parameter/profile
   commitment, NOT ASC-only). Reuses the two canonical hashes already computed at bootstrap (genesis +
   protocol-params) + the explicit ASC the projection consumes, so a valid stake/VRF view cannot be
   consumed under the wrong genesis or protocol configuration. BLUE-pure; `consensus_profile_commitment`
   is the single canonical constructor, used at BOTH derivation (compute + bind) and projection (verify).

3. **`derive_candidate`** (`epoch_candidate.rs`) switches to `drive_window_consensus_inputs` (ECA-0a) to
   get `{stake, pool_params}`, then builds the candidate by the **cardano-faithful intersection**:
   `kept = stake.pool_stakes.keys() ∩ pool_params.keys()` (delegated ∩ registered); `vrf = pool_params[p].vrf_hash`;
   the candidate stake = `stake` restricted to `kept`; `total` recomputed over `kept`. So
   `pool_vrf_keyhashes.keys() == stake_by_pool.keys()` BY CONSTRUCTION (DC-EVIEW-12). Computes the
   full-profile commitment ONCE from the canonical `CandidateProfile {slots_per_epoch, genesis_hash,
   protocol_params_hash, asc}` (threaded canonical, no filesystem/config/network read) and binds it.

4. **The projection** `EpochConsensusView::to_pool_distr_view(genesis_hash, protocol_params_hash, asc)
   -> Result<PoolDistrView, ProjectionError>` (DC-EPOCH-12): verify
   `consensus_profile_commitment(genesis_hash, protocol_params_hash, asc) == self.protocol_params_commitment`
   (else `ParamsCommitmentMismatch`, fail-closed — no unbound param), require `is_leadership_complete()`
   (else `NotLeadershipComplete`), then build `PoolDistrView{epoch, total, asc, pools:
   PoolEntry{active_stake: stake_by_pool[p], vrf_keyhash: pool_vrf_keyhashes[p]}}`. Derived EXCLUSIVELY
   from the view + the commitment-checked profile; no live CertState read.

## Cardano ground truth (own reads @ 226b002d)

- `calculatePoolDistr'` (SnapShots.hs:449-462): include pools with `spssNumDelegators > 0`; VRF =
  `spssVrf` from the snapshot's pool params. The intersection (delegated ∩ registered) reproduces the
  "registered + has a delegator" set; the VRF comes from ECA-0a's window-end pool_params (the mark).
- The mark's VRF is the pre-POOLREAP active key (ECA-0a) — so `pool_params` is exactly the effective set.

## Invariants

- **DC-EVIEW-12 (new):** the EpochConsensusView is leadership-complete — every included pool has BOTH
  active stake AND an era-correct VRF keyhash (equal key sets); the canonical_hash covers stake + VRF +
  the param commitment + all bindings; a non-complete or wrong-params view is INERT.
- **DC-EPOCH-12 (new):** the promoted-epoch PoolDistrView is derived EXCLUSIVELY from the promoted
  EpochConsensusView + the bound-commitment-checked ASC — no live CertState read, no unbound protocol-param read.
- DC-EVIEW-07 strengthened (canonical_hash now covers pool_vrf_keyhashes + protocol_params_commitment).
- DC-EVIEW-05 strengthened (aggregate_pool_stake now includes a pool with >=1 delegator even at 0 stake
  — cardano numDelegators>0, count-not-amount — so the derived pool SET matches cardano's PoolDistr).

## Cardano pool-inclusion fix (user correction 2026-06-21)

`aggregate_pool_stake` (DC-EVIEW-05) previously skipped zero-total credentials, dropping a pool whose
delegators sum to 0, whereas cardano keeps it (`numDelegators > 0`, count-not-amount) with relative
stake 0. Per user direction this is now FIXED, not documented as a divergence: the aggregate includes a
pool with ≥1 delegator even at 0 stake, so the derived pool SET matches cardano's PoolDistr exactly
(snapshot / oracle / hash equality), not merely the likely-leader outcome. DC-EVIEW-05 strengthened; the
proof is `delegated_zero_stake_pool_is_included_with_zero` in `reduced_aggregate.rs`.

## Tests + CI

- `bind` round-trips with VRF + param commitment; canonical_hash is sensitive to a VRF change + a param
  change; `is_leadership_complete` true for a complete view, false (INERT) for mismatched key sets.
- `derive_candidate` produces equal stake/VRF key sets (intersection), drops a delegated-but-unregistered
  pool, recomputes total; round-trips through the WAL record + recovery (Promoted).
- `to_pool_distr_view` builds a complete PoolDistrView from a view; rejects a wrong-ASC (commitment
  mismatch) fail-closed; the built view answers pool_active_stake + pool_vrf_keyhash for the target epoch.
- `ci/ci_check_eview_leadership_complete.sh`.

## Out of scope

The activation wiring (ECA-1..4: remove the gate, deterministic inputs, the atomic ActiveEpochAuthority
swap, warm-start) and the live proof (ECA-5). ECA-0b only makes the candidate view leadership-complete +
the projection exclusive.
