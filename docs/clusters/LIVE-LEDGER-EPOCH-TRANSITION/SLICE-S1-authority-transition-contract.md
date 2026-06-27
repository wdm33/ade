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

## Proof obligations (resolve BEFORE coding — slice-entry, not footnotes)

1. **Reward-input lag, exactly:** confirm against cardano-ledger that the RUPD applied at the `e/e+1`
   boundary consumes epoch-`(e-1)` `block_production` + the go-snapshot active during `e-1`, and that the
   accumulator's per-epoch `block_production`/`epoch_fees` rotation (1d resets, 2 accumulates) lines those
   up. Pin the snapshot the RUPD reads (go) vs the snapshot leadership reads (also go, lag 2) — they are
   the same object but at different rotation ages; the accumulator must hold enough rotation history.
2. **Bootstrap seam:** the first self-derived RUPD needs a full prior-epoch's counts. Seed+1 (bridge) and
   seed+2 (DC-EPOCH-18 nesRu) stay as the bootstrap-transient seeds; define EXACTLY at which boundary the
   accumulator's native RUPD takes over (the first boundary whose entire input epoch was followed) and how
   the accumulator is seeded from the snapshot (decode `nesBcur` for the partial seed-epoch counts).
3. **Governance scope:** the minimal protocol/governance state that affects the epoch transition
   (params + enactment), not the full Conway governance machinery — bound S1's `params`/governance fields
   to only what the RUPD + leadership need.
4. **Persistence cadence:** boundary-only checkpoint vs periodic; confirm the within-epoch replay bound is
   acceptable and the encoding is forward/backward-versioned.

## Cluster Exit Criteria Addressed

- [x] **CE-1 (contract):** this slice defines + tests the total deterministic replay-equivalent transition.
- [ ] CE-2..CE-6: out of scope for S1 (S2–S6).

## Acceptance (S1 — hermetic, no live)

- `apply_selected_block` + `EpochAccumulator` + the canonical codec exist; `cargo test` green.
- **Determinism/replay:** a hermetic multi-block + ≥1-boundary fixture folded twice yields byte-identical
  accumulators; folding the same blocks from a mid-sequence checkpoint == folding from the start.
- **Codec:** round-trip byte-identical; fail-closed on unknown version / non-canonical / trailing bytes.
- **Order:** a unit test asserts a within-epoch withdrawal followed by a boundary yields the boundary
  reward (not zero) — the protocol order encoded in the contract.
- DC-EPOCH-19 declared in the registry; `ci/ci_check_*` placeholder for the contract's structural
  invariants (no full-clone-per-block; the single transition site).
