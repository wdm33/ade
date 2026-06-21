# SLICE ECA-0a — Cardano-faithful pool lifecycle in the reduced window

Part of EPOCH-CONTINUITY-ACTIVATION (the final activation work). ECA-0a is the **prerequisite
lifecycle fidelity** slice: it makes the reduced-window cert-state advance reproduce cardano's
pool registration/retirement lifecycle exactly, so the next-epoch candidate's pool set + VRF keys
are byte-faithful to cardano's mark snapshot. ECA-0b (the leadership-complete `EpochConsensusView`)
freezes the resulting *effective* VRF mapping into the candidate and is built **after** this.

User directive (2026-06-21): correctness-first, no narrow shortcut. "A continuously operating Ade
node must remain correct through ordinary pool retirements, re-registrations, and repeated epoch
transitions." Do not activate a leadership view that is incorrect under re-registration/retirement
merely to reach the first boundary sooner.

## Ground truth (PROVED vs cardano-ledger SHA 226b002d / core 1.20.0.0 = cardano-node 11.0.1)

Two research passes (`af19d4fb`, `a5f9da76`) + a direct read of `Pool.hs`/`SnapShots.hs`:

- **EPOCH transition order: SNAP first, POOLREAP second** (Conway `Epoch.hs:292-297`, Shelley
  `Epoch.hs:158-163`). Both future-param adoption and retirement removal live inside POOLREAP.
- **RegPool** (`Pool.hs:266-310`): NEW pool → `psStakePools` immediately; RE-REGISTRATION → staged
  to `psFutureStakePoolParams` (old `psStakePools` VRF kept) **and `psRetiring` deleted** (re-reg
  cancels a pending retirement immediately).
- **RetirePool** (`Pool.hs:311-328`): schedules `psRetiring[id] = e` (validated `cEpoch < e`).
- **POOLREAP** (`PoolReap.hs:132-229`): adopt `psFutureStakePoolParams → psStakePools` then clear
  futures (L151-167); `retired = {k | psRetiring[k] == e}` removed from `psStakePools` + `psRetiring`
  (L173, L223-224); `e` = the epoch being ENTERED.
- **Mark snapshot** (`SnapShots.hs:418-430` `snapShotFromInstantStake`): reads **only `psStakePools`**
  (never futures, never applies retiring); `spssVrf = spsVrf` from `psStakePools`. Captured by SNAP
  **before** POOLREAP → the mark carries pre-adoption (OLD) VRF and **includes** a pool retiring at
  that very boundary.
- **Inclusion** (`calculatePoolDistr'` `SnapShots.hs:449-465`): a pool is in `PoolDistr` iff
  `spssNumDelegators > 0` (delegator COUNT, not stake amount); a delegation to an unregistered pool
  is silently dropped — never an error.

**Net authoritative rule (target leadership epoch T):** the pool registration/retirement state is
`psStakePools` as of the END of epoch T-2, captured **pre-POOLREAP**. A same-epoch re-registration's
new VRF does NOT count (old VRF governs T); a pool retiring at the T-2→T-1 boundary is still present;
a newly-registered pool is present (dropped from the distribution only if 0 delegators).

**Window-replay corollary:** per-block cert processing == pre-POOLREAP `psStakePools` at the SNAP
instant (so the FINAL mark capture is per-block-only), BUT POOLREAP (adopt futures + reap retired)
MUST be applied at every epoch boundary **crossed** during replay, or the start-of-epoch state is wrong.

## Ade gaps this fixes (code-verified)

- **G1 (VRF version):** `apply_pool_registration` (delegation.rs:247) overwrites `pool.pools[id]`
  immediately on re-registration; Ade has no `future_pools`. → wrong (new) VRF where cardano keeps
  the old → Ade would reject a VRF-rotating pool's valid blocks for ~2 epochs.
- **G2 (boundary reap in the window):** `drive_window_aggregate` advances per-block only; it never
  applies the epoch-boundary POOLREAP, so a window crossing a boundary keeps retired pools and never
  adopts staged re-registrations.

## Design (BLUE; EVIEW/window + tests path only — live follow/forge unaffected, track_utxo=false skips certs)

1. **`PoolState.future_pools: BTreeMap<PoolId, PoolParams>`** (sibling of `pools`/`retiring`).
2. **`apply_pool_registration`** — pool already in `pools` ⇒ stage to `future_pools` + `retiring.remove`
   (do NOT touch `pools`); else (new) ⇒ insert into `pools`. (Latest same-epoch re-reg overwrites the
   staged future entry, matching `Map.insert`.)
3. **`apply_pool_reap(cert: &mut CertState, entered_epoch)`** — (a) adopt: drain `future_pools` into
   `pools`, dropping an orphan future (cardano `Map.dropMissing`); (b) reap: the pools with
   `retiring[id] == entered_epoch`; (c) CLEAR `delegations` targeting those reaped pools (cardano
   `removeStakePoolDelegations (delegsToClear ...)`), preserving each credential's registration +
   rewards; (d) remove the reaped pools from `pools` + `retiring`. Adopt-then-reap (the two sets are
   disjoint because re-reg cancels retiring). Takes the whole `CertState` — delegation-clearing needs
   `DelegationState`.
4. **Window driver** — apply `apply_pool_reap` at each epoch boundary crossed within the replayed block
   range (detected via the era schedule / epoch length), and capture the mark/aggregate at the window
   end **before** the final boundary's reap. Single-epoch window ⇒ no crossing ⇒ per-block-only (G3).
5. **cert-state codec** (`snapshot/cert_state.rs`) — add `future_pools` as a 6th map (array(6));
   update encode/decode + round-trip tests. Needed so the bootstrap cert-state artifact (and the
   manifest's `cert_state_hash`) commits to staged re-registrations. Artifact is regenerated; no live
   behavior change.

### DEFERRED to a track_utxo=true LIVE-LEDGER-APPLY slice (NOT in ECA-0a)
- **Ledger fingerprint** (`fingerprint.rs::fingerprint_cert`) — do NOT add `future_pools`. The durable
  fingerprint covers the live `track_utxo=false` state, whose cert state is always empty, so adding it
  would change every existing fingerprint (empty-map header) and break warm-start for no benefit. The
  EVIEW window uses the transient candidate hash (ECA-0b), not the durable fingerprint. A code comment
  marks this; LIVE-LEDGER-APPLY adds it under a fingerprint-version bump.
- **Live boundary** (`apply_epoch_boundary_with_registrations`, rules.rs) — do NOT adopt `future_pools`.
  The EVIEW window derives via the window driver (`apply_pool_reap`), not this live boundary; on the
  live path future_pools is always empty, so adoption here is inert + untested. Add it with
  track_utxo=true.

## Invariants

- **DC-EVIEW-13 (new):** Cardano-faithful pool lifecycle in the reduced window — re-registrations are
  staged (active VRF unchanged until adoption), retirements scheduled, POOLREAP (adopt futures + reap
  `==entered_epoch`) applied at each crossed boundary, and the mark captured pre-POOLREAP. Matches
  cardano `Pool.hs`/`PoolReap.hs`/`SnapShots.hs` @ 226b002d.
- **DC-EVIEW-10 (strengthen):** the window driver now applies boundary POOLREAP across crossed epochs.

## Tests (hermetic)

- `re_registration_stages_to_future_pools_keeps_old_vrf` — `pools[id]` (incl. VRF) unchanged; the new
  params land in `future_pools`; `retiring[id]` cleared.
- `new_registration_inserts_into_pools` — first reg goes straight to `pools`.
- `pool_reap_adopts_futures_then_reaps_matching_epoch` — futures adopted (new VRF in `pools`); pool with
  `retiring==e` removed; pool retiring at a later epoch kept.
- `window_drive_re_reg_then_boundary_adopts` — re-reg in epoch e: the end-of-e mark (pre e→e+1 reap)
  has the OLD VRF; after the e→e+1 boundary the new VRF is adopted.
- `window_drive_retirement_reaped_at_boundary` — pool retiring at e+1 is present in the end-of-e mark,
  reaped entering e+1.
- `cert_state_codec_round_trips_future_pools` — 6-field round-trip incl. populated `future_pools`.
- replay-equivalence (two drives byte-identical), crash (deterministic mid-replay), reorg
  (rollback re-materialize restores the exact pre-rollback lifecycle state).
- explicit re-registration + retirement fixtures over a synthetic 2-epoch block range.

## Out of scope (documented)

- Conway `psVRFKeyHashes` duplicate-VRF multiset — validation-only; the window replays already-validated
  blocks, so the duplicate check was enforced upstream.
- Deposit / treasury / reward accounting on reap — a stake-amount concern (DC-EVIEW-05 / CE-71), NOT
  pool-set/VRF. PROOF it does not affect the EpochConsensusView at the relevant snapshot phase: SNAP
  captures the mark BEFORE POOLREAP (`Epoch.hs:292-297`), so a boundary's deposit refund cannot affect
  THAT boundary's mark; and the live DC-EPOCH-08 window is single-epoch, so no intermediate POOLREAP
  fires within it. For sustained multi-epoch continuity the refund would affect later reward balances —
  that belongs with the rewards-over-window work (CE-71 live) + the model-A/B `-wire` decision, not ECA-0a.

**Delegation-clearing is IN ECA-0a (NOT deferred):** a stale delegation reattaching on pool-id
re-registration is a delayed divergence (user-directed), so `apply_pool_reap` clears delegations of
reaped pools, proven by `reaped_pool_delegation_cleared_no_silent_reattach_on_reregistration`.
- The leadership-complete `EpochConsensusView` shape (`pool_vrf_keyhashes`, `protocol_params_commitment`,
  the `PoolDistrView` projection) — that is **ECA-0b**, built on top of this.

## CI

`ci/ci_check_eview_pool_lifecycle.sh` — assert `future_pools` present, `apply_pool_registration` stages
re-regs (no unconditional `pools.insert` overwrite on an existing pool), `apply_pool_reap` exists +
is wired into the window driver, mark-captured-before-reap.
