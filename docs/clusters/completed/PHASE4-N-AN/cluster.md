# Invariant Cluster — PHASE4-N-AN — Rollback Materialization Preserves Recovered eta0 (CE-AI-6 unblock)

> A BLUE **replay-equivalence** fix surfaced by the CE-AI-6 reorg pass. An induced peer reorg sent Ade a
> `RollBackward`; Ade's rollback-follow died at
> `apply_chain_event → materialize_rolled_back_state → ReplayFailedAt { slot 261, VrfCert
> VerificationFailed }`. Root-caused (2026-06-11): the recovered **eta0** (epoch nonce) is overlaid onto
> the **live-admit** `chain_dep` at WarmStart (T-REC-04 / `ci_check_warmstart_eta0_overlay.sh` clause D),
> but `materialize_rolled_back_state` sources its replay `chain_dep` from `reader.nearest_le` — the
> **persisted snapshot placeholder nonce** — and never applies that overlay. So a block that validates on
> live admit (against eta0) fails rollback-replay VRF (against the placeholder). **Live admit and
> rollback-replay disagree on the epoch nonce → not replay-equivalent.** This is the consensus-replay seam
> the project's replay-first architecture exists to catch.
>
> **Not operator tooling.** The CE-AI-6 bridge venue already proved the outer pieces: partition-able
> 2-pool venue, a real peer reorg, N-AM keep-alive holding the link through a ~12-min partition, and Ade
> RECEIVING the `RollBackward`. The remaining blocker is this BLUE bug.
>
> **Repro-first (bright-red discipline).** AN-S1 makes the failure MECHANICAL (a hermetic test, no docker)
> BEFORE any fix. AN-S2 fixes it in the recover/snapshot/materialize authority. Then the preserved bridge
> venue re-runs to capture the CE-AI-6 transcript.

## Primary invariant

**T-REC-06** (declared here, targeted **enforced** at close; tier **true**): *Rollback-materialization
replay-equivalence. A block that validates during live admit (against the eta0-overlaid `chain_dep`,
T-REC-04) MUST NOT fail rollback-materialize replay because materialization substituted a different nonce
source. `materialize_rolled_back_state` (the SOLE rolled-back-state authority, CN-STORE-07) MUST
reconstruct the replay `chain_dep` with the SAME recovered eta0 (epoch nonce) the live-admit path uses
(`praos_vrf_input(slot, eta0)`, DC-CINPUT-03); the persisted snapshot's placeholder/genesis `epoch_nonce`
MUST NOT reach VRF verification on the rollback-replay path. Same recovered store + same ordered WAL/feed
⇒ same `chain_dep` inputs ⇒ same validation result on the live-admit and rollback paths. eta0 is the
recovered canonical input (the seed-epoch sidecar) — never peer data, wall-clock, CLI re-supply, or a
re-query. VRF validation strength is UNCHANGED (no bypass; the rollback path stays as strict as live
admit). SCOPE: the recovered seed epoch (no epoch-boundary crossing within the follow window — eta0 is
the constant epoch nonce); a multi-epoch rollback's nonce-evolution is a named out-of-scope follow-on.*

## Normative anchors

- `docs/planning/phase4-n-an-rollback-materialize-eta0-invariants.md` (AN-1..2, the root cause, the
  repro-first slicing).
- **T-REC-04** (the WarmStart-recovered eta0 overlay onto the LIVE `chain_dep`; this cluster extends it to
  the rollback-materialize path) + `ci_check_warmstart_eta0_overlay.sh`.
- **DC-CINPUT-03** (`praos_vrf_input(slot, eta0)` = `blake2b256(slot_be8 ‖ eta0_32)`, the VRF leader/header
  input recipe — the nonce semantics).
- **CN-STORE-07** (the SOLE materialize authority returning `(LedgerState, PraosChainDepState)` —
  `materialize_rolled_back_state`; the fix lives here).
- **DC-NODE-27** (rollback+reselection replay-equivalence — the live receive-event replay law this
  extends), **DC-CONS-20** (the ChainDb-ledger-chain_dep lockstep on the rollback side).
- The seed-epoch sidecar authority: `SeedEpochConsensusInputs.epoch_nonce` (the persisted eta0 carrier).

## Entry Conditions (guaranteed by prior clusters)

- **N-F-G-N (T-REC-04 / DC-CINPUT-03):** the recovered eta0 is carried in the persisted seed-epoch sidecar
  + overlaid onto the live `chain_dep` at WarmStart bootstrap. `ci_check_warmstart_eta0_overlay.sh`
  enforces the LIVE overlay.
- **N-I / N-AI (CN-STORE-07, DC-CONS-20, DC-NODE-23..29):** `materialize_rolled_back_state` (the sole
  rolled-back-state authority) + `commit_rollback` lockstep + the live rollback-follow wiring +
  `WalEntry::RollBack`.
- **N-AK/N-AL (DC-NODE-31..33):** the recovered-anchor recover→follow start + the anchor-rollback no-op
  (this cluster is the NON-anchor rollback that actually replays).

## Exit Criteria (CI-verifiable — named checks, not intent)

- **CE-AN-1** (repro, hermetic, **FAILS on current code** — `ade_ledger`):
  `materialize_replays_against_placeholder_nonce_not_recovered_eta0` — a snapshot reader returning a
  `chain_dep` with a PLACEHOLDER epoch_nonce + the recovered eta0; a corpus block whose VRF verifies
  against eta0; assert (a) `block_validity` with the eta0 `chain_dep` ⇒ Valid (live admit), (b)
  `materialize_rolled_back_state` (placeholder snapshot, current code) ⇒ `ReplayFailedAt VrfCert`, (c)
  `materialized_chain_dep.epoch_nonce == placeholder != eta0`. The bright-red repro.
- **CE-AN-2** (fix, hermetic): the AN-S1 repro flips — with the recovered eta0 threaded + overlaid,
  `materialize` replays the block to **Valid** and the materialized `chain_dep.epoch_nonce == eta0`.
- **CE-AN-3** (replay-equivalence, hermetic):
  `live_admit_and_rollback_materialize_agree_on_eta0_for_same_block` — the live-admit nonce and the
  materialized nonce are byte-equal for the same chain point.
- **CE-AN-4** (NO VRF loosening, hermetic): `materialize_still_fails_closed_on_genuinely_invalid_vrf` —
  a block whose VRF verifies against NEITHER eta0 nor the placeholder still ⇒ `VrfCert` on materialize
  (the fix overlays the correct nonce; it does NOT bypass VRF).
- **CE-AN-5** (gate): `ci_check_rollback_materialize_eta0.sh` — the eta0 overlay is applied on the
  materialize/rollback path (via the shared overlay authority); no VRF bypass / skip; eta0 sourced from
  the recovered sidecar, not peer/CLI/wall-clock.
- **CE-AN-6** (no collateral): `cargo test -p ade_ledger` + `cargo test -p ade_runtime` green; the
  existing `materialize` / `commit_rollback` / `block_validity` / recover tests stay green;
  `ci_check_warmstart_eta0_overlay.sh` still passes (the live overlay is unchanged).
- **CE-AN-LIVE** ✅ **PASSED** (2026-06-11, fresh hermetic 2-pool bridge venue, magic 42): bare-anchor
  recover @ slot 162 → follow cn2 → induced reorg (short partition + `docker network connect` re-peer;
  cn1 led a shallow round, depth 2 ≤ k=5 → cn2 reorged to cn1) → Ade FOLLOWED the `RollBackward`: strict
  slot **regression admit 371 → 361** → re-converged `agreement_verdict{agreed}` @ slot 383 with
  `our_hash_hex == peer_hash_hex` (5b65db35…, the reorged cn1 tip) → 16 admits, 2 agreed, **0 diverged**,
  Ade ALIVE, **0 VrfCert** (the eta0 overlay held through the live rollback — without AN-S2 Ade died here).
  Transcript OUTSIDE-REPO (`~/.cardano-ceai6/ceai6-capture/`, bridge-venue methodology internal; scrubbed
  note only), convergence sha256 = `f3553aaa948f43b08c55a859421dcb8b048d16680e97ca20b9f8acd27aa28b01`;
  `ci_check_convergence_evidence_schema.sh` OK.

## Expected Slice Types

- **AN-S1** (repro-first — the failing hermetic test; NO fix). Construct the placeholder-vs-eta0 snapshot
  + the corpus block + assert the live-Valid / materialize-VrfCert divergence + the nonce mismatch.
  Mechanical proof CE-AN-1. **Commit the failing test** (marked `#[ignore]` or as a documented
  known-failing repro that AN-S2 un-ignores) so the bug is mechanical before the fix.
- **AN-S2** (fix — carry recovered eta0 into rollback materialization; T-REC-06 → enforced). Add a shared
  `overlay_recovered_eta0(chain_dep, eta0)` (the SINGLE eta0-overlay authority reused by WarmStart
  bootstrap + materialize), thread the recovered eta0 into `materialize_rolled_back_state` (from
  `apply_chain_event`'s `state.seed_epoch_consensus_inputs`), overlay before the replay-forward fold.
  Mechanical proof CE-AN-2..6.
- **AN-S3** (live capture — CE-AN-LIVE; the CE-AI-6 transcript). Re-run the preserved bridge venue.

## TCB Color Map (FC/IS Partition)

- **BLUE** — `ade_ledger::rollback::materialize::materialize_rolled_back_state` (CN-STORE-07) + the shared
  `overlay_recovered_eta0` helper: pure `(snapshot_chain_dep, recovered_eta0) → eta0-overlaid chain_dep`,
  then the unchanged replay-forward fold over `block_validity`. The recovered eta0 is BLUE canonical
  state (the seed-epoch sidecar, T-REC-04).
- **Canonical input** — the recovered eta0 (`SeedEpochConsensusInputs.epoch_nonce`).
- **RED (wiring)** — `apply_chain_event` (`node_lifecycle.rs`) threads `state.seed_epoch_consensus_inputs`'
  eta0 into the materialize call. No new RED behavior.
- **RED / unchanged** — `commit_rollback`, the WAL marker, `block_validity` strength, the snapshot
  persistence model, the WarmStart live overlay, `pump_block`.

## Forbidden during this cluster (slices inherit)

Bypassing / skipping / weakening VRF on the rollback path (it stays as strict as live admit) · accepting
replay divergence as okay · sourcing the rollback eta0 from peer data / wall-clock / CLI re-supply /
re-query (eta0 is the recovered canonical sidecar input) · special-casing the reorg venue · making the
snapshot persist eta0 (the snapshot stays a placeholder; the fix OVERLAYS at materialize — preserving the
T-REC-04 persistence model + its gate) · mutating WAL semantics (unless the repro proves the WAL lacks a
required canonical input — it does not; eta0 is in the sidecar) · making rollback validation looser than
live admit · starting with the fix before the repro is mechanical (AN-S1 before AN-S2) · running CE-AN-LIVE
before AN-S2's hermetic CEs pass.

## Registry declarations (this cluster-doc appends as `declared`)

- **T-REC-06** (family T, tier **true**, `introduced_in = PHASE4-N-AN`, status `declared`) — statement as
  the Primary invariant above. `tests = []` (AN-S1/S2 populate); `ci_scripts = []` (AN-S2 adds
  `ci_check_rollback_materialize_eta0.sh`). `cross_ref = [T-REC-04, DC-CINPUT-03, CN-STORE-07, DC-NODE-27,
  DC-CONS-20]`. Appended `declared` after the repro pins the source (coherence gate green).
- **Strengthening note (at AN-S2 close):** `strengthened_in += PHASE4-N-AN` on **T-REC-04** (the eta0
  overlay now also covers the rollback-materialize path via the shared overlay authority) and possibly
  **DC-NODE-27** (rollback replay-equivalence now also covers the eta0 nonce basis). Do NOT flip now;
  T-REC-06 flips `declared → enforced` at close after CE-AN-1..6.

## Close-record note (preserve verbatim at `/cluster-close`)

> **AN extends the recovered-eta0 overlay (T-REC-04) to the rollback-materialize path (T-REC-06):**
> `materialize_rolled_back_state` now reconstructs the replay `chain_dep` with the same recovered eta0 the
> live-admit path uses, so a block that validates on live admit no longer fails rollback-replay VRF —
> closing a replay-equivalence gap the CE-AI-6 reorg surfaced. It does NOT bypass/weaken VRF, change the
> snapshot persistence model, use peer/CLI/wall-clock nonce, or touch the live overlay. Repro-first
> (AN-S1 made the failure mechanical before AN-S2 fixed it). It **unblocks** the CE-AI-6 reorg capture
> (CE-AN-LIVE).
