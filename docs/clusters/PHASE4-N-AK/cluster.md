# Invariant Cluster — PHASE4-N-AK — Recovered Anchor Tip Is the Live-Follow Start Authority

> NARROW post-N-AH regression-remediation (recovery/follow authority — **not** evidence emission).
> **N-AJ is paused until AK lands.** Confirmed by a live A/B (same venue/store/relay/flags, binary
> differs): the N-AH binary follows the frozen relay from a bare block-8 anchor
> (`caught_up_to_peer_tip`, `forge_base_block_no=13`, 29 forges); the current binary halts at
> `UnsupportedRollbackPoint`.

## Primary invariant

**DC-NODE-31** (declared here, targeted **enforced** at close): *After recovery from a non-Origin
bootstrap anchor, the live-follow start tip exposed by `BootstrapState` resolves to the recovered
anchor tip (slot + real hash) whenever ChainDb has no servable post-anchor block; resolution order =
servable ChainDb tip → recovered anchor (non-Origin) → Origin/None only if truly Origin. Does not
change `ChainDb::tip()` semantics and does not synthesize a servable block; AI-S4a
`RollBackward(Origin)` fail-close unchanged; replay-equivalent (extends T-REC-05 to the recovered tip
surface).*

**CN-CONS-03 untouched — stays `declared`.** AK restores the live recover→follow path so the CE-AI-6
operator pass (N-AJ follow-on) becomes runnable again; AK does **not** emit convergence evidence or
flip CN-CONS-03.

## Normative anchors

- `docs/planning/phase4-n-ak-recovered-anchor-tip-invariants.md` (AK-INV-1..6, prohibitions, the TCB
  call, the OQ steers, the acceptance bar).
- DC-NODE-23..29 (N-AI single-best-peer rollback-follow; AI-S4a Origin fail-close
  `crates/ade_runtime/src/admission/wire_pump.rs:447` — **preserved**).
- T-REC-05 (recovered ledger fp == WAL-tail post_fp — this cluster extends replay-equivalence to the
  recovered *tip* surface).
- CE-AH-6 close evidence (the live recover→follow this cluster restores).

## Entry Conditions (guaranteed by prior clusters)

- N-M-A: the bootstrap anchor `seed_point` (slot+hash) is minted from `seed_slot`/`seed_block_hash`
  (`crates/ade_runtime/src/mithril_bootstrap.rs`) — **the recovered anchor is already recorded.**
- N-AH (DC-NODE-20/22): warm-start recovery + `replayed_anchor_block_no` derivation (bare-anchor vs
  replayed-spine distinction).
- N-AI (DC-NODE-23..29): single-best-peer rollback-follow; AI-S4a Origin fail-close.
- `bootstrap_initial_state` (`crates/ade_runtime/src/bootstrap.rs`) is the single recovery authority;
  `ChainDb::tip()` returns `Some` only for servable post-anchor blocks.

## Exit Criteria (CI-verifiable — named checks, not intent)

- **CE-AK-1** (recovery resolution, hermetic): `ade_runtime` tests on
  `bootstrap_initial_state` / `resolve_live_follow_start` —
  POSITIVE `bootstrap_bare_anchor_recovery_surfaces_anchor_as_live_follow_tip` (snapshot @ non-Origin
  anchor, `admit_count==0` ⇒ live-follow tip == anchor slot+real-hash);
  NEGATIVE `bootstrap_true_origin_recovery_surfaces_none_tip` (cold-start, empty snapshot set ⇒ tip ==
  `None`);
  POST-ANCHOR `bootstrap_servable_chaindb_tip_wins_over_anchor` (ChainDb has servable post-anchor
  blocks ⇒ servable ChainDb tip wins). `cargo test -p ade_runtime` green.
- **CE-AK-2** (live-follow start point, hermetic): `ade_node` test
  `recovered_bare_anchor_findintersect_starts_at_anchor_not_origin` — a bare-anchor warm-start ⇒
  `spawn_live_wire_pump_source` start_point == the anchor `Block` point (NOT `Origin`) ⇒ the AI-S4a
  Origin fail-close is not reached. `cargo test -p ade_node` green.
- **CE-AK-3** (live regression re-verification — operator-run, mechanically checked): the FIXED binary,
  on the **SAME** frozen venue/store/relay the A/B used, `--mode node --single-producer-venue`:
  - FindIntersect starts from the **recovered anchor, not Origin**;
  - relay catch-up **reaches the frozen relay tip**;
  - **`forge_base_block_no == frozen relay tip block_no`** (the strongest live signal — the
    recovered-anchor start point restored the exact follow path; not merely "it forges");
  - **0** `UnsupportedRollbackPoint` in the run log.

  Run at close (the N-AH worktree binary + the frozen `c2-relay` are already standing). Evidence kept
  outside-repo (scrubbed in-repo note only).
- **CE-AK-4** (no collateral): `cargo test --workspace` green; `warm_start_recovers_seed_epoch_consensus_inputs_byte_identical`
  + `warm_start_dispatch_succeeds_end_to_end` (`node_lifecycle.rs`) + the T-REC-05 tests stay green;
  the three `ci/ci_check_convergence_evidence_{vocabulary_closed,emit_only,schema}.sh` gates stay
  green; the `ChainDb::tip()` contract is unchanged.

## Expected Slice Types

- **AK-S1** (single slice — a narrow regression is not over-split) — the BLUE recovery-decision fix +
  its mechanical proof (CE-AK-1, CE-AK-2, CE-AK-3). Add deterministic
  `resolve_live_follow_start(servable_chaindb_tip, recovered_anchor) -> Option<ChainTip>`;
  `bootstrap_initial_state` exposes it as the `BootstrapState` live-follow start tip; thread the
  existing `BootstrapAnchor.seed_point` into `bootstrap_initial_state`. The wire-pump consumer
  (`spawn_live_wire_pump_source`) and AI-S4a are **unchanged**.

## TCB Color Map (FC/IS Partition)

- **BLUE** — `resolve_live_follow_start` + the `BootstrapState` live-follow start tip resolution
  (`crates/ade_runtime/src/bootstrap.rs`): the authoritative deterministic recovery decision (what
  point the node recovered to) — it governs the FindIntersect start point, live-follow behavior, and
  replay-equivalent recovery state. Hosted in a crate that also carries RED I/O; the *decision* is
  BLUE regardless of host.
- **Canonical input** — the recovered anchor `seed_point` (`BootstrapAnchor`, minted from
  `seed_slot`/`seed_block_hash`).
- **RED (unchanged)** — `spawn_live_wire_pump_source` / the wire pump (`node_lifecycle.rs`,
  `wire_pump.rs`): consumes the resolved tip; not modified.
- **Out of scope** — `ChainDb::tip()` (storage contract unchanged); ledger materialization
  (`bootstrap.rs:216` null-hash target, OQ-AK-2); admission orchestration (OQ-AK-3); N-AJ evidence
  emission.

## Forbidden during this cluster (slices inherit)

- Do NOT weaken AI-S4a — `RollBackward(Origin)` stays fail-closed.
- Do NOT modify peer/relay behavior.
- Do NOT special-case the venue harness.
- Do NOT make ChainDb invent/synthesize a servable block.
- Do NOT use WAL `admit_count` as the anchor-hash proxy.
- Do NOT touch N-AJ evidence emission.
- Do NOT alter ledger materialization (`bootstrap.rs:216` null-hash target) unless a test proves
  materialization currently depends on the placeholder (OQ-AK-2 — out of scope by default).
- Do NOT redesign admission orchestration — `--mode admission` is in scope ONLY if it consumes the
  same `resolve_live_follow_start` helper (OQ-AK-3), else diagnostic/out-of-scope.
- Do NOT flip CN-CONS-03.

## Registry declarations (this cluster-doc appends as `declared`)

- **DC-NODE-31** (family DC, derived, `introduced_in = PHASE4-N-AK`, status `declared`) — statement as
  the Primary invariant above (verbatim, including *"does not change `ChainDb::tip()` semantics and
  does not synthesize a servable block"*). `tests = []` (AK-S1 populates the four named tests);
  `ci_script = ""` (Rust-test-enforced; CE-AK-3 is the operator-run live verification).
- Strengthening note (do **not** flip now): T-REC-05 may gain the recovery-tip test in its `tests` at
  AK close (`strengthened_in += PHASE4-N-AK`).

## Close-record note (preserve verbatim at `/cluster-close`)

> **AK fixes a regression in the live-follow start surface. It does NOT make ChainDb serve the anchor
> as a block, does NOT weaken rollback-to-Origin rejection, and does NOT claim full ChainSel
> convergence.**
