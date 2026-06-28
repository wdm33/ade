# S1 — Authority transition contract + accumulator state + canonical persistence

**Rule:** DC-EPOCH-19 (declared by this slice). **Cluster:** LIVE-LEDGER-EPOCH-TRANSITION.
**Depends on:** DC-EPOCH-18 (the seed+2 byte-exact stake — the bootstrap-transient seed), DC-EVIEW-*
(the reduced-window cert/stake transitions to reuse), the MEM-OPT reduced-checkpoint architecture.
**Status:** Proposed.

> **This slice is a CONTRACT, not a struct.** The center is the single authoritative transition
> `apply_selected_block`. The accumulator type and its encoding exist to serve that contract. If the
> contract is under-specified, the cluster recreates the same piecemeal problem inside a new struct.

## The gap

The live node has no authoritative ledger transition (cluster §2): cert effects are *reconstructed*
ad-hoc by a seed-anchored window-replay, and rewards / block-production / fees / snapshot-rotation are
not produced at all. Before any wiring, the project must pin ONE total transition that says *exactly*
how every non-UTxO ledger fact evolves per block and per boundary — so S2–S6 are wiring a defined
contract, not discovering it boundary by boundary.

## The contract (the load-bearing artifact)

```
apply_selected_block(
    prior:        &EpochAccumulator,           // the authoritative non-UTxO facts before this block
    block_bytes:  &[u8],                        // canonical [era, block] of a DURABLE selected-chain block
    ctx:          &SelectedBlockCtx,            // era, era-schedule geometry, stake source (reduced checkpoint
                                                //   handle), prior block's (slot, epoch) — NEVER peer/CLI/wall-clock
) -> Result<EpochAccumulator, LedgerTransitionError>
```

**Total, deterministic, replay-equivalent.** Same `prior` + same `block_bytes` + same `ctx` ⇒ a
byte-identical `EpochAccumulator`. No wall-clock, rand, HashMap, float, or I/O inside (BLUE). The only
nondeterminism is the canonical block bytes.

### Order of effects (the protocol order — non-negotiable)

For a block in epoch `C` with `prior` at epoch `P`:

1. **Boundary transitions first, one per crossed boundary** `e = P+1 .. C` (empty epochs included): apply
   `epoch_transition(acc, e)` BEFORE any of this block's within-epoch effects. Within each boundary, in
   cardano-ledger order (NEWEPOCH):
   a. **apply the completed reward update (RUPD)** computed from the JUST-FINISHED epoch's accumulated
      facts (block_production + epoch_fees + reserves) over the correct go-snapshot — AFTER that epoch's
      withdrawals (which were applied per-block in step 2 of earlier blocks), so a within-epoch
      withdrawer receives its boundary reward, not zero (the B3c/DC-EPOCH-18 lesson, now native);
   b. **SNAP** — rotate stake snapshots mark→set→go; the new mark = the current stake distribution;
   c. **POOLREAP** — reap pools retiring at `e`, adopt staged future-pool re-registrations, clear their
      delegations (reuse the proven `apply_pool_reap`);
   d. **governance/protocol-state enactment** that takes effect at `e` (the param/pots changes that feed
      the NEXT RUPD/leadership); reset block_production + epoch_fees for the new epoch.
2. **Within-epoch effects of THIS block** (in tx/cert order):
   - **certificates** — stake/pool registration + de-registration + delegation changes → the delegation,
     pool-registration, and future-pool/retirement maps;
   - **withdrawals** — zero the named reward account (the withdrawn amount leaves the reward map);
   - **issuer block-production** — `block_production[header.issuer_pool] += 1` (the producer is the
     header issuer — already read by `header_validate`; NO leader-schedule lookup);
   - **epoch fees** — `epoch_fees += Σ(tx fee)` over the block's transactions;
   - **governance** — record proposals/votes/ratification state needed for (1d) at the next boundary.

`epoch_transition` reads the reduced-checkpoint stake (via `ctx`) for the snapshot/RUPD; it does NOT
require a full UTxO map.

### Rollback / re-materialization

The accumulator is a pure function of the durable selected chain: `materialize(acc0, [block…])` =
fold `apply_selected_block` over an ordered durable block range from a durable accumulator checkpoint.
A rollback to slot `s` re-materializes the accumulator from the nearest durable checkpoint at-or-before
`s` by re-applying the canonical blocks — the SAME path as restart recovery (S5). No special rollback
mutation; replay equivalence is the rollback mechanism.

### Errors (`LedgerTransitionError`, fail-closed)

A malformed block, an unknown cert/governance variant on the authority path, an arithmetic overflow in a
pot/reward/count, a missing required input (e.g. a non-Complete reward source mid-pulse), or a boundary
gap (`C < P`, or a stake source unavailable for the snapshot) is a TERMINAL structured error — never a
silent partial accumulator, never a fabricated default.

## The accumulator state (`EpochAccumulator`, BLUE)

Closed record, all fields required at construction (no `Default`, no `#[non_exhaustive]`):

- `epoch: EpochNo`, `last_slot: SlotNo` — position;
- `cert_state` — delegations + reward accounts + pool registrations + future-pool/retirement maps
  (reuse the existing `CertState`/`DelegationState`);
- `block_production: BTreeMap<PoolId, u64>`, `epoch_fees: Coin` — the current epoch's accumulating
  reward inputs;
- `pots: { reserves, treasury }`;
- `snapshots: { mark, set, go }` — the stake distributions (the leadership + RUPD inputs);
- `params` — the consensus-relevant protocol params + the pending governance changes that affect the
  epoch transition (rho, tau, a0, k, d, …);
- `pending_reward_update` — the in-progress RUPD source for the next boundary (the native analog of the
  bootstrap `BootstrapRewardUpdate`).

The large UTxO/stake set is NOT here — it stays in the disk-backed reduced checkpoint.

## Canonical persistence format

A version-gated, byte-canonical encoding (the bootstrap_bridge / seed_consensus_inputs discipline:
fixed field order, `BTreeMap` key order, fail-closed decode, re-encode-equality backstop). Durability
cadence (resolved as a proof obligation, but the contract pins the *shape*): a full accumulator
checkpoint at each epoch boundary (the natural barrier) + within-epoch recovery by replaying the current
epoch's durable blocks over the last boundary checkpoint — so restart/rollback replay is bounded to ONE
epoch, never the full chain. The encoding is the durable replay authority for S5.

## Memory criteria carried into the contract (cluster §4)

- The contract returns a `next` accumulator but the IMPLEMENTATION must mutate-in-place / apply a compact
  delta — `apply_selected_block` is specified by value for determinism, but S2 wires it with an in-place
  reducer that does NOT clone the whole accumulator per block.
- No full UTxO map in the accumulator; stake comes from the reduced checkpoint via `ctx`.
- No full-chain replay per block; per-block work touches only affected entries.
- The boundary checkpoint write is bounded + disk-backed (the only naturally heavier step).

## Proof obligations — RESOLVED (cardano-ledger source + Ade surfaces, 2026-06-28)

The resolutions below are read off the cardano-ledger Haskell source (on disk at
`~/Documents/ade-planning/reference/cardano-ledger`) and the actual Ade `ade_ledger` surfaces. They are
load-bearing: the contract's rotation is built to them.

1. **Reward-input lag — RESOLVED: the RUPD consumes `nesBprev` over the `go` snapshot (a true 2-buffer
   model).**
   - `Rules/Tick.hs:261,276` — `bheadTransition` extracts `bprev` from `NewEpochState _ bprev _ es _ _ _`
     and runs `TRC (RupdEnv bprev es, nesRu nes1, slot)`. The reward update consumes **`nesBprev`**, not
     `nesBcur`.
   - `LedgerState/PulsingReward.hs` — `startStep ... es@(EpochState acnt ls ss nm) ...` reads
     `let SnapShot ... = ssStakeGo ss`. The stake is the **`go`** snapshot.
   - `Rules/NewEpoch.hs:151-198` — at boundary `e→e+1`: apply `nesRu` (`updateRewards`) FIRST, then
     (Shelley only) `MIR`, then `EPOCH`, then rotate `nesBprev := bcur` (the old `nesBcur`),
     `nesBcur := mempty`, `nesRu := SNothing`. Conway (`Conway/Rules/NewEpoch.hs:158-192`) is identical
     **minus MIR**, with RATIFY enactment inside EPOCH.
   - `Rules/Epoch.hs:143-184` — EPOCH order = **SNAP → POOLREAP → UPEC/RATIFY**.
   - `Rules/Snap.hs:95-103` — `ssStakeMark := <current instant stake>`, `ssStakeSet := old ssStakeMark`,
     `ssStakeGo := old ssStakeSet` (matches Ade `epoch::rotate_snapshots`).
   - **Net:** the reward applied at the boundary INTO epoch `X` consumes blocks of epoch `X-2` (the held
     `nesBprev`) over the held `go` snapshot; after the boundary `nesBprev := nesBcur(X-1)`,
     `nesBcur := ∅`. So a follower accumulating `nesBcur` per block needs a SEPARATE held `nesBprev`
     (and `prev_epoch_fees`) — the accumulator carries BOTH. The existing
     `rules::apply_epoch_boundary_with_registrations` reads `epoch_state.block_production` as the
     to-be-rewarded counts (the `nesBprev` convention — see `EpochState.block_production` doc + the
     `ledgerdb_nonutxo_hermetic` NES layout `[epoch, nesBprev, nesBcur, EpochState, ru, pd, stashed]`)
     and resets it; the contract therefore feeds it `prev_block_production`/`prev_epoch_fees` at the
     boundary, then rotates `prev := <just-finished nesBcur>`.
2. **Bootstrap seam — `pending_reward_update` carries the DC-EPOCH-18 seed; applied once at its
   `target_epoch→+1` boundary, then cleared.** The native RUPD takes over at the first boundary whose
   entire input epoch was followed (`pending_reward_update == None`). For S1 the field + its one-shot
   application (`delegation::apply_bootstrap_reward_deltas`, the proven Option-B path) are defined +
   tested in isolation; the live seeding (`nesBcur` decode for the partial seed epoch) is S2, the exact
   native-vs-seed layering is verified by S3's live byte-exact gate.
3. **Governance scope — Conway is the LIVE era, so the accumulator carries the FULL `ConwayGovState` +
   `ProtocolParameters`** (both already have canonical codecs in `snapshot/`). The boundary reuses the
   Conway-aware `apply_epoch_boundary_with_registrations` (RATIFY enactment, no MIR, circulation-based
   `totalStake` for PV≥4). Pre-Conway is rejected by the codec (the `snapshot/` Conway-only discipline).
4. **Persistence cadence — a full accumulator checkpoint, version-framed + byte-canonical** (the
   `bootstrap_bridge`/`seed_consensus_inputs` discipline: version gate, definite containers,
   re-encode-equality backstop, trailing-byte + Conway-era gate). Composes the existing sub-codecs
   (`encode_epoch_state`/`encode_cert_state`/`encode_pparams`/`encode_gov_state`/
   `encode_conway_deposit_params`) + the `nesBprev` buffer + `pending_reward_update`. Within-epoch
   recovery replays the current epoch's durable blocks over the last boundary checkpoint (bounded to one
   epoch) — S5.

### Known S3 reconciliation items (recorded, not silently dropped)
- **POOLREAP completeness.** `apply_epoch_boundary_with_registrations` does an INLINE retirement (deposit
  return + remove from `pools`, `≤ epoch`) but omits future-pool adoption + delegation-clearing;
  `delegation::apply_pool_reap` does adoption + delegation-clear + `== epoch` reap but no deposits. S1
  reuses the boundary fn (validated reward path) and additionally calls `apply_pool_reap` for
  future-pool adoption (consensus-critical: the re-registered VRF must be active next epoch). The
  deposit-vs-clear ordering reconciliation is an S3 byte-exact item.
- **Withdrawal credential discriminant.** The boundary credits rewards under a `KeyHash` projection of the
  reward-account bytes (`rules.rs` known simplification); the contract's withdrawal extraction decodes the
  real key/script discriminant from the stake-address header. Consistency at script-hash reward accounts
  is an S3 byte-exact item; S1's order test uses key-hash accounts (the matching projection).

## Cluster Exit Criteria Addressed

- [x] **CE-1 (contract):** this slice defines + tests the total deterministic replay-equivalent transition.
- [ ] CE-2..CE-6: out of scope for S1 (S2–S6).

## Acceptance (S1 — hermetic, no live) — DONE

Landed in `crates/ade_ledger/src/epoch_accumulator.rs` (BLUE); `cargo test -p ade_ledger --lib` green
(707 tests, 16 new); clippy clean; the new module is rustfmt-clean.

- [x] `apply_selected_block` + `EpochAccumulator` + `SelectedBlockCtx` + `LedgerTransitionError` +
  `encode_/decode_epoch_accumulator` exist; the codec composes the existing `snapshot/` sub-codecs (no
  second encoding scheme).
- [x] **Determinism/replay:** `apply_selected_block_on_real_conway_block_is_deterministic` (same prior +
  block + ctx ⇒ byte-identical) and `replay_equivalence_via_durable_checkpoint_across_a_boundary`
  (folding from the persisted checkpoint == folding from the start, across a real boundary).
- [x] **Codec:** `codec_round_trips_byte_identical_populated` + fail-closed on unknown version /
  pre-Conway era / trailing bytes / non-canonical (`codec_rejects_*`).
- [x] **Order:** `within_epoch_withdrawal_then_boundary_pays_fresh_reward` — the post-withdrawal zero
  does NOT suppress the boundary's fresh reward (the protocol order, the B3c/DC-EPOCH-18 lesson native).
- [x] **Two-buffer rotation:** `boundary_rotates_block_production_two_buffer` proves `nesBprev := <finished
  nesBcur>`, `nesBcur := ∅` (the cardano model resolved in proof obligation #1).
- [x] **Fail-closed:** `missing_boundary_stake_is_fail_closed`, `boundary_gap_is_fail_closed`,
  `pending_reward_update_applied_once_then_cleared`.
- [x] DC-EPOCH-19 in the registry (`tests`/`ci_scripts` populated); `ci/ci_check_epoch_accumulator_no_utxo.sh`
  mechanically guards the field-ownership contract (NO UTxO field; the OWNED two-buffer + seed; the
  empty-UTxO `as_ledger_view`; the version-gated Conway-only re-encode-backstop codec).
