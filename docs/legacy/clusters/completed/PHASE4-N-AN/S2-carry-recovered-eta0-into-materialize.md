# Slice AN-S2 — Carry Recovered eta0 Into Rollback Materialization (the fix; T-REC-06 → enforced)

## 1. Title
`materialize_rolled_back_state` overlays the recovered seed-epoch eta0 onto the nearest-snapshot
`chain_dep` BEFORE the replay-forward fold, so rollback replay validates each block's header VRF against
eta0 — the SAME nonce live admit used — not the snapshot `Nonce::ZERO` placeholder. Flips the AN-S1 repro
from VrfCert to Valid. Fixes the root cause of BOTH the CE-AI-6 reorg-follow failure (#1) AND the
warm-start replay failure (#4) — they were the same bug on two callers.

## 2. Slice Header
- **Cluster:** PHASE4-N-AN. **Status:** Merged (this commit). **CE-AN-LIVE pending** (the live reorg
  transcript — the cluster's live deliverable).
- **Exit Criteria Addressed:** CE-AN-2 (fix), CE-AN-3 (replay-equivalence), CE-AN-4 (no VRF bypass),
  CE-AN-5 (gate), CE-AN-6 (no collateral). CE-AN-LIVE is the follow-on live run.
- **Primary registry rule:** T-REC-06 (`declared` → **`enforced`**).

## 4. Intent (invariant impact)
Establish T-REC-06: a block that validates on live admit MUST NOT fail rollback-materialize replay. The
single eta0-overlay authority (`PraosChainDepState::overlay_recovered_eta0`, shared by WarmStart bootstrap
+ materialize) makes the rollback-replay `chain_dep` carry the SAME recovered eta0 the live-admit path
uses — replay-equivalence by construction.

## 6. Execution Boundary (TCB color)
- **BLUE** — `PraosChainDepState::overlay_recovered_eta0` (`ade_core`, the shared overlay authority);
  `materialize_rolled_back_state` (`ade_ledger`, CN-STORE-07) gains `recovered_eta0: Option<&Nonce>` and
  applies the overlay onto the `nearest_le` `chain_dep` before the degenerate-return + the replay fold.
  The replay-forward `block_validity` fold is UNCHANGED (VRF stays verified).
- **Canonical input** — the recovered eta0 (`SeedEpochConsensusInputs.epoch_nonce`).
- **RED (wiring, no new behavior)** — `ForwardSyncState.recovered_eta0` (set once at bootstrap from the
  recovered sidecar); `apply_chain_event` passes `fwd.recovered_eta0.as_ref()` (the live rollback-follow);
  `bootstrap_initial_state` restores the sidecar BEFORE materialize and passes its eta0 (fixing warm-start
  replay from a non-bare store); `RollbackContext.recovered_eta0` (the test-only BLUE receive-reducer path).

## 7. Invariants Preserved
- `block_validity` VRF strength (CE-AN-4: a WRONG eta0 still fails VRF — the overlay is not a bypass).
- The WarmStart live overlay (T-REC-04) + its gate (`ci_check_warmstart_eta0_overlay.sh` still OK — the
  literal post-materialize overlay is retained; the materialize overlay is idempotent before it at the
  seed epoch).
- The snapshot persistence model (the snapshot still carries the placeholder; the fix OVERLAYS, it does
  not change what is persisted). `commit_rollback` lockstep (DC-CONS-20), the WAL marker (DC-NODE-27),
  CN-STORE-07 (still the sole materialize authority — additive param).

## 8. Invariants Strengthened
- **T-REC-06** `declared` → **`enforced`** (the repro flips to Valid; the gate + 2 regression tests land).
- The recovered eta0 now reaches the rollback-replay AND the bootstrap WarmStart replay (not only the
  post-materialize live `chain_dep`) — the same explicit-canonical-input discipline T-REC-04 mandates,
  now covering the replay fold.

## 9. Design Summary
1. `PraosChainDepState::overlay_recovered_eta0(&mut self, eta0)` sets `epoch_nonce = evolving_nonce =
   eta0` — extracted verbatim from the WarmStart overlay so both paths apply the IDENTICAL transform.
2. `materialize_rolled_back_state(..., recovered_eta0: Option<&Nonce>)`: after `reader.nearest_le`, if
   `Some(eta0)`, overlay it onto `chain_dep` before the degenerate return AND the replay fold. `None`
   keeps the snapshot nonce as-is (cold-start / no sidecar; existing callers + tests).
3. Threading: `ForwardSyncState.recovered_eta0` (set at the recover arm from
   `state.seed_epoch_consensus_inputs`) → `apply_chain_event`. Bootstrap restores the sidecar before
   materialize + passes its eta0. The test-only receive reducer threads via `RollbackContext`.
4. **#1 + #4 unified:** the live rollback-follow (`apply_chain_event`, the CE-AI-6 reorg) and the
   WarmStart bootstrap recovery (`bootstrap_initial_state`, the warm-start replay) both call the SAME
   materialize authority. Overlaying eta0 there fixes both. The live reorg capture (CE-AN-LIVE) and a
   live warm-start re-run are the remaining live validations.

## 11. Replay / Crash / Epoch Validation
The fix IS replay-equivalence (CE-AN-3: the materialized nonce == the live-admit nonce basis, byte-equal).
SCOPE: the recovered seed epoch (eta0 constant — no boundary in the follow window); a multi-epoch
rollback's nonce-evolution is a named out-of-scope follow-on.

## 12. Mechanical Acceptance Criteria
- **CE-AN-2/3** `rollback_materialize_overlays_recovered_eta0_replay_equivalent` (ade_ledger): None ⇒
  VrfCert; Some(eta0) ⇒ Valid + materialized `epoch_nonce == eta0 ==` live-admit basis. PASS.
- **CE-AN-4** `rollback_materialize_does_not_bypass_vrf_on_wrong_eta0`: wrong eta0 ⇒ VrfCert. PASS.
- **CE-AN-5** `ci_check_rollback_materialize_eta0.sh`. PASS.
- **CE-AN-6** no collateral: ade_ledger materialize 7/7, ade_runtime bootstrap 36/36 +
  receive_rollback_integration 5/5, wal_rollback_ai_s1 13/13, ade_node 338/338,
  `ci_check_warmstart_eta0_overlay` still OK. PASS.
- **CE-AN-LIVE** (follow-on): the preserved CE-AI-6 bridge venue re-run.

## 14. Hard Prohibitions (observed)
No VRF bypass/skip (CE-AN-4) · eta0 from the recovered sidecar only (never peer/CLI/wall-clock) · the
snapshot persistence model unchanged · no looser-than-live-admit validation · repro-first (AN-S1 before
AN-S2).
