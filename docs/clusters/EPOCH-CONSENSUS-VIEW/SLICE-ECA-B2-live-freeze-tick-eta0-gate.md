# SLICE ECA-B2 — live candidate-freeze (RSW) + boundary tick + the eta0(seed+2) gate

Part of EPOCH-CONSENSUS-VIEW / EPOCH-CONTINUITY-ACTIVATION, **Tier B**. The live half of
[[SLICE-ECA-B1]] (DC-EPOCH-16). B1 shipped the BLUE rolling-nonce machinery + per-block follow-path
wiring + the hermetic proof, with the candidate-freeze left INERT (no production `RSW` yet) and the
boundary tick deferred. B2 makes the freeze live-correct, wires the tick on the follow path, and
**proves the decisive live gate** — Ade's self-evolved `eta0(seed+2)` == the cardano-node `epochNonce`
— which flips **DC-EPOCH-16 `declared` → `enforced`**.

## What B1 left for B2 (verbatim, from the B1 slice doc + IDD review)

1. Supply the production `RSW = ceil(4k/f)` so the candidate freezes live-correct.
2. Wire the boundary tick on the follow path (today the per-block evolution runs live, but NO live
   `StreamInput::EpochBoundary` emitter exists — the boundary combine never runs live).
3. The review's residual WARN: add the **symmetric** coupling gate (no live tick emitter while
   `freeze_boundary` can resolve to `CANDIDATE_FREEZE_INERT`); broaden the production-view check to
   *any* non-test `impl LedgerView` overriding `randomness_stabilisation_window`.
4. Retire the vestigial `StreamInput::EpochBoundary.last_block_of_prev_epoch`.

## Decisive findings (investigation, 2026-06-26)

- **`k` (securityParam) is NOT on the live follow path.** `genesis_parser` parses
  `shelley.securityParam` and computes `safe_zone_slots = ceil(3k/f)` — but only on the from-genesis
  path. The live/import path builds the `EraSchedule` via `recovered_node_schedule` →
  `make_node_schedule(epoch_start_slot, epoch_no, epoch_length_slots)` from the **durable sidecar**,
  whose geometry authority persists only `epoch_length_slots` (`seed_consensus_inputs.rs`) — NOT `k`,
  `safe_zone`, or RSW. On that path `safe_zone_slots` is a PLACEHOLDER (`epoch_length` / `432_000`),
  not the real `ceil(3k/f)`.
- **The boundary-detection site is `maybe_activate_epoch_boundary`** (`node_lifecycle.rs:1610`, the
  DC-EPOCH-15 atomic Seed→Promoted + forecast extension), called at `~2165`. That is where the live
  nonce tick fires.
- **The gate is a single FirstRun run (no restart),** so RSW can be computed from the venue shelley
  genesis `k` at bootstrap and threaded IN-MEMORY — **no durable-format change, no
  getting-started-guide impact.** Durable warm-start RSW (a sidecar v5 carrying `k` or RSW so the
  freeze survives a restart-across-boundary) is **B4**, not B2.

## Design

### 1. RSW in the era geometry (in-memory, BLUE-read)

- Add `randomness_stabilisation_window_slots: u32` to `EraSummary` (`ade_core/consensus/era_schedule.rs`),
  the canonical era geometry. `header_validate` reads `freeze_boundary = firstSlotNextEpoch −
  era.randomness_stabilisation_window_slots` directly from the `EraSchedule` it already holds, and the
  **defaulted `LedgerView::randomness_stabilisation_window` is removed** (eliminating the B1
  fail-open path entirely — RSW now always present in the era geometry, no `None`/`MAX`).
- **Source of RSW:** at FirstRun the venue shelley genesis (resolved by `--network` /
  `--shelley-genesis-path`) is available; parse `k` (`genesis_parser`'s `securityParam`) and compute
  `RSW = ceil(4·k·denom / numer)` (`f = numer/denom = active_slots_coeff`), exactly mirroring
  `safe_zone_slots = ceil(3·k·denom/numer)`. Thread it into the `EraSummary` built for the live
  follow path (`make_node_schedule` / the import schedule build) — replacing the placeholder for the
  RSW field specifically (the placeholder `safe_zone_slots` for the forecast is a separate concern,
  out of scope).
- Churn: `EraSummary` gains one field → ~20 construction sites (test fixtures + the live builders)
  add it; scriptable like the B1 field churn. Test fixtures get a representative value; the live
  builders compute the real `ceil(4k/f)` from the venue `k`.

### 2. The boundary tick on the follow path

- At `maybe_activate_epoch_boundary` (the boundary-detection site, when the durable tip crosses
  N→N+1), apply `apply_nonce_input(&chain_dep, &NonceInput::EpochBoundary { new_epoch })` to advance
  the chain-dep `epoch_nonce` via the BLUE combine, then continue the relay loop.
- **Reconcile with the ECA-5 bridge-sync** (`node_sync.rs:~583`, which overlays the precomputed
  `eta0(seed+1)` at boundary 1): the general tick on a fully-seeded chain-dep reproduces the bridge
  value byte-identically (B1's `seeded_chain_dep_tick_reproduces_bridge_eta0`). So the tick REPLACES
  the bridge's *nonce* overlay on the general path; the bridge stays for the *stake* authority (the
  MARK-snapshot seed+1 leadership — a separate concern). A boundary-1 cross-check (tick result ==
  bridge `eta0`) is a cheap fail-closed assertion to keep.
- **Coupling:** RSW + the tick MUST land together — the B1 gate fails if a production view supplies
  RSW without a live tick emitter, and (new) the symmetric gate fails if a live tick emitter exists
  while `freeze_boundary` can be `CANDIDATE_FREEZE_INERT`.

### 3. Gates + cleanup

- Broaden + symmetrise `ci_check_praos_nonce_follow_evolution.sh` per the review WARN.
- Retire `StreamInput::EpochBoundary.last_block_of_prev_epoch` (now `_`-unused).

## Proof obligations

- **PO-B2-1:** the venue shelley genesis (with `k`) is reachable at FirstRun on the Mithril-import
  path (not only the from-genesis path). If not, fall back to threading `k` from the resolved
  `NetworkProfile` (extend it with `security_param`) — still in-memory, still no sidecar change.
- **PO-B2-2:** the tick at `maybe_activate_epoch_boundary` fires exactly once per crossed boundary,
  on the durable selected tip (not a peer-reported tip), and is replay-equivalent.
- **PO-B2-3:** confirm `RSW = ceil(4k/f)` exactly matches cardano-node's
  `randomnessStabilisationWindow` for the venue (the live `eta0(seed+2)` gate is the ground truth).

## Invariant

- **DC-EPOCH-16 strengthened → enforced.** The rolling nonce is now driven on the live follow path
  end-to-end (per-header evolve/lab/candidate-freeze with a finite RSW + the epoch tick), and the
  self-evolved `eta0(seed+2)` equals the live node's `epochNonce`. The B1 fail-open path is removed
  (RSW is era geometry, no `None`/`CANDIDATE_FREEZE_INERT` on the live path).

## Tests + CI

- Hermetic: `EraSummary` RSW round-trips through the schedule build; `header_validate` reads a finite
  `freeze_boundary` from the era geometry (no `CANDIDATE_FREEZE_INERT` reachable when RSW is present);
  the tick fires once per boundary + the boundary-1 cross-check (tick == bridge `eta0`).
- `ci_check_praos_nonce_follow_evolution.sh`: symmetric coupling (per the WARN).
- **Live gate (decisive):** FirstRun from a Mithril bootstrap at epoch N, follow across N→N+1→N+2,
  self-evolve `eta0(N+2)`, assert == `cardano-cli query protocol-state` `epochNonce` at N+2. Venue /
  slots / commands in the untracked runbook (ECA-5 pattern).

## Out of scope (later Tier B)

- **B4:** durable warm-start RSW (sidecar v5 carrying `k`/RSW) so the freeze survives a
  restart-across-boundary; live restart recovery.
- **B2b/B3:** generalize the activation seam past the fixed `seed_epoch` (fires at every boundary;
  replay-derived seed+2 stake authority) so Ade VALIDATES N+2 blocks — the eta0(N+2) gate proves the
  nonce is right; validating N+2 needs the N+2 leadership authority.
- **B5:** the full unattended forge-off crossing proof.
