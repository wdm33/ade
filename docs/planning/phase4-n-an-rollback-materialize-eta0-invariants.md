# PHASE4-N-AN — Rollback Materialization Preserves Recovered eta0 (invariants sketch)

> Invariants sketch (IDD Part I). Cluster **PHASE4-N-AN**. **Classification:** *replay-equivalence
> correctness fix* — a BLUE consensus bug in the recover→follow→rollback path. The rollback-materialize
> authority re-validates a block against a `chain_dep` whose `epoch_nonce` is the **persisted snapshot
> placeholder**, NOT the recovered **eta0** that WarmStart overlays onto the live-admit `chain_dep`
> (T-REC-04). A block that validates on live admit fails rollback-replay VRF — a replay-equivalence
> violation. **Unblocks CE-AI-6** (the reorg-follow dies here).

## The bug (confirmed LIVE + root-caused, 2026-06-11)

On the CE-AI-6 bridge venue, an induced peer reorg sent Ade a `RollBackward`. Ade's rollback-follow
died:

```
apply_chain_event: Materialize(ReplayFailedAt { slot: SlotNo(261), error: Header(VrfCert(VerificationFailed)) })
```

Decisive diagnosis:
- **eta0 is CORRECT** — the recovered `epoch_nonce_hex` == the venue's epoch-0 nonce == the genesis hash
  (`124dd67f…`). NOT an extraction bug.
- **The live admit runs the FULL header VRF** (`receive/reducer.rs:242` → `admit_via_block_validity` →
  `block_validity`) and the block @ slot 261 **passed** — so the live `chain_dep.epoch_nonce` = eta0 and
  the VRF verifies.
- **The rollback path re-validates against a DIFFERENT nonce.** `apply_chain_event`
  (`node_lifecycle.rs:2350`) calls `materialize_rolled_back_state`, which sources its `chain_dep` from
  `reader.nearest_le(target.slot)` — the **persisted snapshot's `chain_dep`**, whose `epoch_nonce` is the
  **placeholder/genesis** value (per the `ci_check_warmstart_eta0_overlay.sh` design: the snapshot is a
  placeholder; eta0 lives in the seed-epoch sidecar and is overlaid at WarmStart bootstrap onto the LIVE
  `chain_dep` ONLY). `materialize` never applies that overlay → it replays the same block against the
  placeholder nonce → `block_validity` → `VrfCert(VerificationFailed)`.

So the eta0 overlay (T-REC-04 / `ci_check_warmstart_eta0_overlay.sh` clause D) reaches the live-admit
`chain_dep` but NOT the rollback-materialize `chain_dep`. **Live admit and rollback-replay disagree on
the epoch nonce → not replay-equivalent.**

## Pure transformation?
Yes — entirely a BLUE pure-function bug. `materialize_rolled_back_state(target, reader, source,
schedule, view)` is deterministic, no I/O. The fix carries the recovered eta0 (a canonical input from
the recovered seed-epoch sidecar) into the replay `chain_dep`, the same explicit-canonical-input overlay
T-REC-04 already mandates for live admit. No wall-clock, no peer data, no CLI, no looser validation.

## 1. What must always be true
- **AN-1 (replay-equivalence of authoritative validation — TRUE).** Same recovered store + same ordered
  WAL/feed ⇒ same `chain_dep` inputs ⇒ same `block_validity` result. The live-admit path and the
  rollback-materialize path MUST agree on the `epoch_nonce`/eta0 basis for the same chain point. A block
  that validates during live admit MUST NOT fail rollback-materialize replay because materialization
  substituted a different nonce source.
- **AN-2 (rollback materialize carries recovered eta0 — DERIVED).** `materialize_rolled_back_state` (the
  SOLE rolled-back-state authority, CN-STORE-07) MUST reconstruct the replay `chain_dep` with the SAME
  recovered eta0 (epoch_nonce) overlay WarmStart bootstrap applies to the live `chain_dep` (T-REC-04,
  `praos_vrf_input(slot, eta0)` per DC-CINPUT-03). The persisted snapshot's placeholder/genesis
  `epoch_nonce` MUST NOT reach VRF verification on the rollback-replay path. Scope of THIS cluster: the
  recovered seed-epoch (no epoch-boundary crossing within the follow window — eta0 is the constant
  epoch nonce); a multi-epoch rollback's nonce-evolution is a named out-of-scope follow-on.
- **Preserved (unchanged):** the live-admit eta0 overlay (T-REC-04); `block_validity` validation strength
  (NO loosening — VRF stays verified on both paths); the `commit_rollback` lockstep (DC-CONS-20); the
  WAL rollback marker + re-anchor (DC-NODE-27); the materialize sole authority (CN-STORE-07); the
  snapshot persistence model (the snapshot stays a placeholder; eta0 stays in the sidecar — the fix
  OVERLAYS at materialize, it does NOT change what the snapshot persists).

## 2. What must never be possible
- Rollback-materialize replay validating a block against the snapshot placeholder nonce (it must use
  eta0).
- Bypassing / skipping / weakening VRF verification on the rollback path (it stays as strict as live
  admit).
- Sourcing the rollback eta0 from peer data, a wall-clock, a CLI restart, or re-querying — eta0 is the
  recovered canonical input (the seed-epoch sidecar), same as live admit.
- A "fix" that special-cases the reorg venue or accepts replay divergence.

## 3 / 4. Determinism / replay
This IS the replay law (AN-1). The fix makes rollback-materialize byte-replay-equivalent to live admit
on the eta0 basis. Pure, deterministic, no nondeterministic inputs.

## 5. State transitions in scope
`materialize_rolled_back_state` gains the recovered eta0 as an explicit input (threaded from the
recovered seed-epoch sidecar by the caller `apply_chain_event`, which already holds
`state.seed_epoch_consensus_inputs`), and overlays it onto the `chain_dep` returned by
`reader.nearest_le` BEFORE the replay-forward fold — the same overlay `bootstrap_initial_state` applies
at WarmStart. The replay then evolves `chain_dep` via `block_validity` unchanged (epoch_nonce constant
within the seed epoch). No other transition changes.

## 6. TCB color hypothesis
- **BLUE** — the fix lives in `ade_ledger::rollback::materialize` (BLUE, CN-STORE-07) + the recovered-eta0
  threading. The overlay is a pure canonical-input application. No RED/GREEN behavior.

## 7. Open questions
- Exact overlay site: inside `materialize_rolled_back_state` (overlay onto `nearest_le`'s `chain_dep`)
  vs. a shared `overlay_recovered_eta0(chain_dep, eta0)` helper reused by bootstrap + materialize (DRY —
  single eta0-overlay authority). Resolve at slice-doc (lean: a shared helper, so the overlay is the
  SAME code on both paths — replay-equivalence by construction).
- Multi-epoch rollback (the snapshot's epoch_nonce ≠ eta0 because a boundary was crossed) — OUT OF
  SCOPE here (the seed-epoch follow window crosses no boundary); named as a follow-on if a multi-epoch
  reorg venue is ever exercised.

## Registry (declared after the repro pins the source)
**T-REC-06** (proposed; tier TRUE; family T) — *rollback-materialization replay-equivalence: a block that
validates on live admit MUST NOT fail rollback-materialize replay; `materialize_rolled_back_state`
reconstructs the replay `chain_dep` with the SAME recovered eta0 overlay (T-REC-04) the live-admit path
uses; the snapshot placeholder nonce never reaches rollback VRF.* `status = declared`. **Strengthens /
cross-refs** T-REC-04 (extends the eta0 overlay to the rollback path), DC-CINPUT-03 (`praos_vrf_input`),
CN-STORE-07 (the materialize authority), DC-NODE-27 (rollback replay-equivalence), DC-CONS-20. Likely a
new `ci_check_rollback_materialize_eta0.sh` gate + the hermetic repro/fix tests. **Unblocks CE-AI-6.**

## Repro-first (the bright-red discipline — make the failure MECHANICAL before fixing)
- **AN-S1** — hermetic repro: a snapshot reader returning a `chain_dep` with a PLACEHOLDER epoch_nonce +
  the recovered eta0; a corpus block whose VRF verifies against eta0; assert (a) live `block_validity`
  with the eta0 `chain_dep` ⇒ Valid, (b) `materialize_rolled_back_state` (reading the placeholder
  snapshot) ⇒ `ReplayFailedAt VrfCert`, (c) `materialized_chain_dep.epoch_nonce == placeholder != eta0`.
  No docker / partition / log. The current code MUST fail this.
- **AN-S2** — fix: thread the recovered eta0 into `materialize_rolled_back_state` + overlay it (shared
  `overlay_recovered_eta0`); the AN-S1 repro now passes (materialized nonce == eta0, replay validates),
  deterministic, no VRF bypass, no peer/CLI/wall-clock nonce.
- **Then** rerun the preserved CE-AI-6 bridge venue → the reorg-follow's materialize succeeds → slot
  regression + final `agreed` + 0 diverged → sha-bound CE-AI-6 transcript.
