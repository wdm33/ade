# SLICE ECA-B1 — Rolling Praos nonce evolution on the live follow path

Part of EPOCH-CONSENSUS-VIEW / EPOCH-CONTINUITY-ACTIVATION, **Tier B** (self-sustaining operation
across epoch boundaries). ECA-5 proved a native-Mithril follower survives its FIRST boundary
(seed→seed+1) via a precomputed bootstrap **bridge** eta0. The bridge is welded to the fixed
`seed_epoch`; from boundary 2 (seed+1→seed+2) Ade must derive `eta0` itself. B1 is the foundation:
make the chain-dependent Praos nonce **roll correctly on the live follow path**, so Ade owns the
next-epoch nonce instead of leaning on a one-time precomputed value.

The defect is not "a missing field." Ade's chain-dep transition models the wrong protocol shape for
sustained Praos: per-header it advances only the evolving nonce; the candidate is never tracked/frozen
live, `lab` is never maintained, and the boundary transition does `epoch_nonce = candidate` (no
combine) **and wrongly resets `evolving`**. All of this is invisible to ECA-5 because within-epoch
leader checks read `epoch_nonce`, never `evolving`/`candidate` — it only surfaces at boundary 2.

## User directive (2026-06-25) — locked

- **Enum reshape.** Fold the live per-header Praos update into ONE authoritative `HeaderContribution`;
  retire the dead `CandidateFreeze` split (its pieces could be called out of order or omitted on the
  follow path — an unsafe shape for a consensus transition). From the single per-header input BLUE
  computes `evolving'`, `lab'`, `candidate'`. The epoch tick computes the combine + rotation.
- **Backward-compatible chain-dep format.** Always WRITE `array(10)`; ACCEPT legacy `array(9)`.
- **Explicit operand, no fabrication.** `last_epoch_block_nonce` is an explicit optional. A legacy
  `array(9)` store decodes to an explicit *no-operand* form and keeps its already-promised
  within-epoch operation, but the rolling cross-boundary combine **fails closed** on an absent operand
  unless a valid bootstrap bridge or freshly-seeded B1 state supplies it. Never invent a nonce so a
  stale store can *appear* to support continuity merely because it decodes.
- **Bridge equivalence is a mandatory hermetic assertion** (see Acceptance).
- **Live gate unchanged:** self-evolved `eta0(seed+2)` == the live node's `epochNonce(seed+2)`.
- **Track this doc.** The protocol rule, compatibility obligations, and CI mapping are the durable
  design record for a consensus-critical correction. Only live venue commands, timing, paths, keys,
  and capture procedure live in the untracked runbook.

## Pinned canonical rule — Praos (NOT TPraos)

Preview/preprod/mainnet run **Praos** (Babbage+). Pinned from `ouroboros-consensus`
`Ouroboros/Consensus/Protocol/Praos.hs` (`reupdateChainDepState` + the epoch tick), cross-checked
against `cardano-ledger @ cb57dc730` (`Cardano/Protocol/TPraos/Rules/{Updn,Tickn}.hs`,
`Cardano/Ledger/BaseTypes.hs`, `eras/shelley/.../StabilityWindow.hs`). Praos diverges from TPraos at
the boundary — exactly the shape the ECA-5 empirical finding already showed.

```
nonce algebra:  a ⭒ b = Nonce(blake2b256( bytes(a) ‖ bytes(b) )) ;  NeutralNonce = identity

PER VALIDATED HEADER  (slot s, prev-block-hash ph, VRF nonce-output ν):
  evolving'  = evolving ⭒ nonceValue(ν)
  lab'       = prevHashToNonce(ph)              # Nonce(castHash(ph)) ; NeutralNonce at genesis
  candidate' = if s < freeze_boundary then evolving' else candidate
                 freeze_boundary = firstSlotNextEpoch − RSW
                 RSW = randomnessStabilisationWindow = ceil(4·k / f)   # NOT 8k/f, NOT 3k/f

EPOCH TICK  (first tick into the new epoch, before any new-epoch block):
  epoch_nonce'            = candidate ⭒ last_epoch_block_nonce     # Praos: NO extraEntropy operand
  previous_epoch_nonce'   = epoch_nonce
  last_epoch_block_nonce' = lab
  # evolving and candidate carry through UNCHANGED (NOT reset)
```

`last_epoch_block_nonce` in the combine is the lab of the last block of epoch **E−1** (set at the
previous tick) — the 2-epoch lag that makes Praos randomness ungrindable.

## Current Ade gap (first-hand + reconnaissance)

| Aspect | Praos canonical | Ade today | B1 |
|---|---|---|---|
| per-header `evolving'` | `evolving ⭒ nonceValue(ν)` | `blake2b256(evolving ‖ praos_nonce_value(ν)[0..32])` | **confirm** (sibling `praos_leader_value` validated by 1388 ECA-5 checks; final proof = the eta0 gate) |
| per-header `lab'` | `prevHashToNonce(ph)` | not maintained on follow path | **add** |
| per-header `candidate'` | track until `s ≥ freeze_boundary`, then freeze | not maintained (CandidateFreeze dead on follow path) | **add** |
| tick `epoch_nonce'` | `candidate ⭒ last_epoch_block_nonce` | `= candidate` | **fix combine** |
| tick `evolving` | unchanged | **reset to candidate** | **fix (stop resetting)** |
| tick rotation | `last_epoch_block_nonce' = lab` | records `Option<EpochNo>` into `last_epoch_block` | **fix** |
| operand field | `Nonce` | absent (`lab_nonce` + `last_epoch_block: Option<EpochNo>` only) | **add `last_epoch_block_nonce: Option<Nonce>`** |
| live wiring | per-header + tick fire on the follow path | only the ECA-5 bridge-sync sets epoch+evolving at boundary 1 | **wire** |

## Design

### 1. State machine (`ade_core/src/consensus/nonce.rs`, BLUE)

`PraosChainDepState` gains `last_epoch_block_nonce: Option<Nonce>` (`None` = explicit *unset*; legacy
or pre-seed). The closed `NonceInput` enum is reshaped to two indivisible transitions:

```
HeaderContribution { slot, prev_block_hash, vrf_nonce_output, freeze_boundary }
    evolving'  = evolving ⭒ nonceValue(vrf_nonce_output)
    lab'       = prevHashToNonce(prev_block_hash)
    candidate' = if slot < freeze_boundary { evolving' } else { candidate }
    last_slot' = slot                          (monotonic guard preserved)

EpochBoundary { new_epoch }
    epoch_nonce'            = candidate ⭒ leb        where leb = last_epoch_block_nonce
                             ↳ last_epoch_block_nonce == None  ⇒  Err(MissingLastEpochBlockNonce)  (fail closed)
    previous_epoch_nonce'   = epoch_nonce
    last_epoch_block_nonce' = Some(lab)
    # evolving, candidate UNCHANGED
```

`CandidateFreeze` is removed. `freeze_boundary` is a canonical input (computed in the shell from the
era geometry + k/f), not derived inside BLUE — keeping the transition a pure function of its inputs.
The `EpochBoundary` operand `last_block_of_prev_epoch: Option<EpochNo>` is dropped (the bookkeeping
`last_epoch_block` field, if still wanted, is set elsewhere — it is NOT the combine operand).

### 2. Durable format (`ade_ledger/src/snapshot/chain_dep.rs`, BLUE) — backward-compatible

- **Encode:** always `array(10)`; 10th field `null | bytes(32)` = `last_epoch_block_nonce`
  (`None`→`null`, `Some(n)`→`bytes(32)`).
- **Decode `array(10)`:** recover the full B1 state (10th field → `Option<Nonce>`).
- **Decode `array(9)` (legacy):** recover the prior 9 fields and set `last_epoch_block_nonce = None`
  (explicit unset). The decoder accepts EXACTLY arity 9 or 10; any other arity is the existing
  `ArrayLengthMismatch`.
- A legacy store thus warm-starts and follows **within an epoch** byte-for-byte as before; its first
  rolling cross-boundary combine fails closed (`MissingLastEpochBlockNonce`) — it must re-bootstrap
  (snapshot) to obtain a seeded B1 state. No silent default, no deceptive continuity.

This preserves `docs/getting-started-preview.md` exactly (the 3 commands + the within-epoch
warm-start promise are unchanged). The guide's "cross-epoch continuity is in active development"
caveat (lines ~208-210) is lifted only when B5 proves the live crossing — that future edit is a
flagged change.

### 3. Live wiring

- **Seed all five nonces at FirstRun.** The bootstrap path seeds `PraosChainDepState` from
  `extract_praos_nonces_v2`'s proven mapping `[candidate, epoch, evolving, lab, last_epoch_block]`,
  including `last_epoch_block_nonce = Some(tail[4])`. (Today only `tail[0]`/`tail[4]` feed the bridge
  precompute; B1 makes them persistent chain-dep state.)
- **Per header on the follow path** (`node_sync` relay loop → `header_validate` Step 9): the existing
  `HeaderContribution` call site is extended to pass `prev_block_hash` + `freeze_boundary`; the shell
  computes `freeze_boundary` from `EraSchedule` (firstSlotNextEpoch for the header's epoch) and
  `RSW = ceil(4k/f)`.
- **At the boundary tick** the relay loop applies `EpochBoundary { new_epoch }` to advance the
  chain-dep nonce, replacing the ECA-5 bridge-sync overlay on the general path (the bridge value and
  the B1 combine are equal — see Acceptance (a)).

## Proof obligations

- **PO-1 (per-header evolving).** Confirm `praos_nonce_value` == cardano `vrfNonceValue` for the
  evolving mix. Strong prior: its sibling `praos_leader_value` passed 1388 live ECA-5 leader checks
  with 0 VrfCert failures; same construction, different domain tag. DEFINITIVELY closed by the live
  `eta0(seed+2)` gate. If it diverges, B1 corrects `praos_nonce_value`.
- **PO-2 (window).** Pin `RSW = ceil(4k/f)` and locate `k` (securityParam) in the canonical profile
  (`f` = `active_slots_coeff`, already threaded). Compute `freeze_boundary = firstSlotNextEpoch − RSW`
  via `EraSchedule`. Edge `firstSlotNextEpoch < RSW` (early chain) is N/A on preview mid-chain.
- **PO-3 (prev-hash availability).** Confirm the followed header's previous-block hash is available at
  the `HeaderContribution` call site for `lab' = prevHashToNonce(ph)`.

## Invariant

- **DC-EPOCH-16 (new, declared):** rolling Praos chain-dep nonce evolution on the live follow path —
  the per-header single-input transition (`evolving'`/`lab'`/`candidate'` from
  `{slot, prev_block_hash, vrf_nonce_output, freeze_boundary}`) + the epoch-tick combine
  (`candidate ⭒ last_epoch_block_nonce`, rotation, `evolving`/`candidate` unchanged), with explicit
  operand presence (fail-closed on absence, never fabricated), a backward-compatible `array(10)`/legacy
  `array(9)` durable format, mandatory hermetic bridge equivalence, and the live `epochNonce` ground
  truth. `CandidateFreeze` as a separable transition is retired.
- **DC-EPOCH-15 cross-ref:** B1 supplies the self-derived `eta0` that DC-EPOCH-15's forecast/authority
  promotion will consume from boundary 2 (the bridge is boundary-1 only).

## Tests + CI

- **Hermetic — bridge equivalence (mandatory):** seed the seed-epoch snapshot nonces → apply the B1
  `EpochBoundary` tick → assert `epoch_nonce'` byte-equals the live-proven ECA-5 bridge `eta0(seed+1)`.
- **Hermetic — state machine:** per-header `evolving'/lab'/candidate'` vectors incl. the freeze at
  `slot == freeze_boundary`; tick combine + rotation + `evolving`/`candidate` unchanged; tick with
  `last_epoch_block_nonce == None` → `MissingLastEpochBlockNonce`; replay determinism.
- **Hermetic — format:** `array(10)` round-trip for `Some`/`None`; legacy `array(9)` → `None`;
  always-write-`array(10)`; deterministic re-encode; a B1 store that crossed a boundary recovers an
  identical chain-dep and the same next-boundary `eta0` (replay equivalence). **Regenerate** the
  synthetic `corpus/consensus/nonce_evolution/epoch_boundary.json` (it locked the pre-combine rule).
- **CI:** `ci/ci_check_praos_nonce_follow_evolution.sh` (single-input transition shape; combine; no
  evolving-reset; fail-closed on absent operand; `CandidateFreeze` retired) + extend the chain-dep
  snapshot gate for backward-compat.
- **Live gate:** Ade's self-evolved `eta0(seed+2)` == `cardano-cli query protocol-state` `epochNonce`
  at seed+2 (untracked runbook holds the venue/slot/commands).

## Acceptance — what B1 must prove, with the scope seam marked

```
bootstrap at N → imported bridge validates N+1                         [ECA-5, done]
follow N+1: per-header transition advances evolving/lab/candidate      [B1]
boundary N+1 → N+2:
  B1 epoch tick derives eta0(N+2)                                      [B1]   ← decisive gate
  replay-derived authority promotes / forecast extends                [B2 (seam) + B3 (replay authority)]
  first N+2 header validates                                          [B2+B3]
restart: same chain-dep state + same next-boundary result             [B1 hermetic round-trip; live = B4]
```

Two gates are **B1-decisive and standalone** (reachable riding ECA-5's bridge authority through N+1,
no seam generalization needed):

- **(a) hermetic bridge equivalence** — binds the generalized Praos logic to a real observed Preview
  boundary before it is relied upon for N+2.
- **(b) live `eta0(seed+2)` == node `epochNonce`** — the decisive evidence that Ade has escaped the
  fixed-bootstrap scaffolding and entered the rolling continuous Praos pipeline.

The "authority promotes / forecast extends / N+2 header validates" rungs additionally require **B2**
(generalize the activation seam past the fixed `seed_epoch`) and **B3** (wire the replay-derived
seed+2 authority); live restart-identical-across-a-boundary is **B4**; the full unattended crossing is
**B5**. B1 does not absorb them.

## Out of scope

B2 (seam fires at every boundary), B3 (replay-derived seed+2 stake authority), B4 (live restart
recovery across a boundary), B5 (live forge-off crossing proof). B1 makes the **nonce** roll correctly
and proves it equals ground truth; it does not generalize the authority/forecast machinery.
