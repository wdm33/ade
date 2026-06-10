# Invariant Cluster — PHASE4-N-AK — Recovered Anchor Tip Is the Live-Follow Start Authority

> NARROW post-N-AH regression-remediation (recovery/follow authority — **not** evidence emission).
> **N-AJ is paused until AK lands.** Confirmed by a live A/B (same venue/store/relay/flags, binary
> differs): the N-AH binary follows the frozen relay from a bare block-8 anchor
> (`caught_up_to_peer_tip`, `forge_base_block_no=13`, 29 forges); the current binary halts at
> `UnsupportedRollbackPoint`.
>
> **Option A (persist).** The recovered store does NOT carry the anchor `(slot, hash)` point today
> (the sidecar + WAL provenance carry only `anchor_fp`; the FirstRun arm gets the point from the CLI).
> N-AH "worked" only by re-syncing from genesis on `RollBackward(Origin)` — never using the anchor.
> AK **persists the bootstrap anchor point as additive, replayable recovery provenance** and resolves
> the live-follow start from it — store-derived, never CLI-re-supplied at restart.
>
> **Two slices (AK-S2 added after a live CE-AK-3 finding).** AK-S1 fixed the live-follow START
> (FindIntersect at the anchor, not Origin) — proven live (0 `UnsupportedRollbackPoint`). But the
> relay's standard post-`IntersectFound(anchor)` rewind `RollBackward(anchor)` then halts the
> single-producer follow (`run_node_sync` `UnexpectedRollback`, `node_sync.rs:471`) because a bare
> anchor is a recovery snapshot, not a stored servable block. **AK-S2 accepts that
> rollback-to-the-recovered-anchor as an idempotent boundary rewind**, after which the existing
> `pump_block` resumes catch-up (proven by a live probe — blocks 9–13 admit through the existing
> admit path with NO new forward-link code). **CE-AK-3 now spans BOTH slices**; the cluster is not
> closed until the recover→follow completes end-to-end.

## Primary invariant

**DC-NODE-31** (declared here, targeted **enforced** at close): *After recovery from a non-Origin
bootstrap anchor, the recovered store persists the bootstrap anchor point `(slot, hash)` as replayable
recovery provenance, bound to the recovered anchor fingerprint. On warm-start, `BootstrapState`
resolves the live-follow start tip from that persisted anchor point whenever ChainDb has no servable
post-anchor block; resolution = servable ChainDb tip → persisted recovered anchor point (non-Origin +
provenance-bound) → Origin/None only if truly Origin/cold-start. A non-Origin recovered store whose
anchor-point record is missing/malformed/fingerprint-mismatched fails closed. Does not change
`ChainDb::tip()` semantics, does not synthesize a servable block, does not weaken `RollBackward(Origin)`
fail-close. Replay-equivalent (extends T-REC-05). The persisted anchor point is the durable restart
authority — NOT CLI re-supply (CLI seed-point is first-run input only).*

**DC-NODE-32** (declared by AK-S2, targeted **enforced** at close): *On the single-producer
live-follow path (`run_node_sync`), a peer `RollBackward` whose target binds EXACTLY (slot AND hash)
to the persisted recovered anchor point (DC-NODE-31 / `BootstrapState.tip`) is accepted as an
idempotent no-op boundary rewind — no `commit_rollback`, no `WalEntry::RollBack`, no ChainDb /
ledger / chain_dep mutation, no cursor. `RollBackward(Origin)` still fails closed (AI-S4a unchanged);
every non-anchor, non-Origin rollback fails closed; the accepted point binds to the PERSISTED anchor
on slot AND hash, never peer-supplied alone. The recovered anchor consumed by `run_node_sync` is the
single authority (`BootstrapState.tip`, threaded in — never re-read from the store inside the loop).
The anchor is a recovery snapshot boundary, NEVER synthesized into a ChainDb block or served. The
first forward block after the anchor admits through the EXISTING sole `pump_block` path (no new
forward-link code — the OQ-AK-S2-2 live probe proved it). Replay-equivalent (extends T-REC-05 /
DC-NODE-31 to the single-producer follow).*

**CN-CONS-03 untouched — stays `declared`.** AK restores the live recover→follow path so the CE-AI-6
operator pass (N-AJ follow-on) becomes runnable again; AK does **not** emit convergence evidence or
flip CN-CONS-03.

## Normative anchors

- `docs/planning/phase4-n-ak-recovered-anchor-tip-invariants.md` (AK-INV-1..6, the Option-A persist
  design, prohibitions, OQ steers, the acceptance bar).
- DC-NODE-23..29 (N-AI single-best-peer rollback-follow; AI-S4a Origin fail-close
  `crates/ade_runtime/src/admission/wire_pump.rs:447` — **preserved**).
- T-REC-05 (recovered ledger fp == WAL-tail post_fp — this cluster extends replay-equivalence to the
  recovered *tip* surface).
- DC-MITHRIL-02 (the anchor `seed_point` binding — the canonical-input source the persisted record
  carries).
- CE-AH-6 close evidence (the live recover→follow this cluster restores, replacing N-AH's
  re-sync-from-genesis with FindIntersect-at-the-anchor).

## Entry Conditions (guaranteed by prior clusters)

- N-M-A: the bootstrap anchor `seed_point` (slot+hash) is minted from `seed_slot`/`seed_block_hash`
  (`mithril_bootstrap.rs`) at recover — but **today it is NOT persisted loadably** (the sidecar
  `SeedEpochConsensusInputs` + the WAL `RecoveredBootstrapProvenance` carry only `anchor_fp`). AK adds
  that persistence.
- N-AH (DC-NODE-20/22): warm-start recovery + `replayed_anchor_block_no` derivation.
- N-AI (DC-NODE-23..29): single-best-peer rollback-follow; AI-S4a Origin fail-close.
- `bootstrap_initial_state` (`crates/ade_runtime/src/bootstrap.rs`) is the single recovery authority;
  `ChainDb::tip()` returns `Some` only for servable post-anchor blocks.

## Exit Criteria (CI-verifiable — named checks, not intent)

- **CE-AK-1** (persistence + resolution, hermetic — `ade_runtime`):
  - `bootstrap_recover_persists_anchor_point_sidecar` — seed/recover writes the anchor-point provenance
    record (bound to `anchor_fp`).
  - `warm_start_loads_persisted_anchor_point` — warm-start loads it and resolves it as the live-follow
    start tip for a bare-anchor recovery.
  - `warm_start_non_origin_anchor_missing_anchor_point_fails_closed` — a non-Origin recovered store
    with no anchor-point record ⇒ fail closed (no Origin fallback).
  - `warm_start_anchor_point_fingerprint_mismatch_fails_closed` — a record not bound to the recovered
    `anchor_fp` ⇒ fail closed.
  - `same_store_same_anchor_point_same_findintersect_start` — replay-equivalence of the tip surface.
  - `bootstrap_bare_anchor_recovery_surfaces_anchor_as_live_follow_tip` (bare anchor ⇒ tip == anchor
    slot+real-hash); `bootstrap_true_origin_recovery_surfaces_none_tip` (cold-start ⇒ tip == None);
    `bootstrap_servable_chaindb_tip_wins_over_anchor` (post-anchor ⇒ servable ChainDb tip wins);
    `resolve_live_follow_start_treats_zero_hash_anchor_as_origin` (pure-fn unit).
  - `cargo test -p ade_runtime` green.
- **CE-AK-2** (live-follow start point, hermetic — `ade_node`):
  `recovered_bare_anchor_findintersect_starts_at_anchor_not_origin` — a bare-anchor warm-start ⇒
  `spawn_live_wire_pump_source` start_point == the anchor `Block` point (NOT `Origin`) ⇒ the AI-S4a
  Origin fail-close is not reached. `cargo test -p ade_node` green.
- **CE-AK-3** (live regression re-verification — operator-run at close): the FIXED binary, on the
  **SAME** frozen venue/store/relay the A/B used, `--mode node --single-producer-venue`:
  - FindIntersect starts from the **persisted recovered anchor, not Origin** (AK-S1);
  - the relay's post-intersection `RollBackward(anchor)` is **accepted as an idempotent boundary
    rewind**, after which `pump_block` resumes catch-up (AK-S2);
  - relay catch-up **reaches the frozen relay tip**;
  - **`forge_base_block_no == frozen relay tip block_no`** (the strongest live signal — the persisted
    anchor restored the exact follow path; not merely "it forges");
  - **0** `UnsupportedRollbackPoint` AND **0** `UnexpectedRollback`.

  **Spans AK-S1 + AK-S2.** The AK-S1-only live re-verification (2026-06-10, fixed binary `8bb1c402`)
  confirmed FindIntersect at the anchor with 0 `UnsupportedRollbackPoint`, then halted at
  `UnexpectedRollback` — exactly the AK-S2 seam; the full end-to-end pass is verified only after AK-S2
  lands. Run at close (the frozen `c2-relay` + `~/.cardano-ceai6` are standing). Evidence outside-repo
  (scrubbed in-repo note only). **NOTE:** the fixed binary must RE-RECOVER (to write the persisted
  anchor-point record into the store) before the `--mode node` follow — the pre-AK store lacks it.
- **CE-AK-4** (no collateral): `cargo test --workspace` green;
  `warm_start_recovers_seed_epoch_consensus_inputs_byte_identical` +
  `warm_start_dispatch_succeeds_end_to_end` + the T-REC-05 tests stay green; the three
  `ci/ci_check_convergence_evidence_*.sh` gates green; the `ChainDb::tip()` contract is unchanged.

## Expected Slice Types

- **AK-S1** (start-tip authority — the first of two slices; CLOSED hermetically, committed `8bb1c402`)
  — the BLUE recovery-decision fix + the additive anchor-point persistence + their mechanical proof
  (CE-AK-1, CE-AK-2; CE-AK-3's FindIntersect-at-anchor leg). (1) Add an additive persisted anchor-point
  provenance record (`(slot, hash)` bound to `anchor_fp`); (2) write it at seed/recover; (3) load it at
  warm-start (fail closed on missing/malformed/mismatch for a non-Origin store); (4) add
  `resolve_live_follow_start` + extend `BootstrapInputs` with `recovered_anchor: Option<ChainTip>`,
  sourced from the loaded record, so `bootstrap_initial_state` exposes it as the live-follow start tip.
  The wire-pump consumer + AI-S4a are **unchanged**.
- **AK-S2** (follow-rollback boundary — the second of two slices; DC-NODE-32) — the BLUE
  single-producer rollback-resolution fix that lets the recover→follow COMPLETE. (1) Thread the
  already-loaded recovered anchor point (`BootstrapState.tip`) into `run_node_sync` (one new param —
  single authority, NO store re-read); (2) in `run_node_sync`'s `RollBack` handler, accept a
  `RollBackward` binding EXACTLY (slot AND hash) to the recovered anchor as an idempotent no-op
  (no `commit_rollback`, no `WalEntry::RollBack`, no ChainDb/ledger/chain_dep/cursor mutation);
  `RollBackward(Origin)` and every other point still fail closed. The first forward block then admits
  through the EXISTING `pump_block` path — **no forward-link code** (the OQ-AK-S2-2 live probe proved
  it). `pump_block`, `block_validity`, the participant path (`run_participant_sync`), `ChainDb::tip()`,
  serve, and AI-S4a are **unchanged**. Mechanical proof: CE-AK-S2-1..5 (hermetic) + CE-AK-3 end-to-end.

## TCB Color Map (FC/IS Partition)

- **BLUE** — `resolve_live_follow_start`, the persisted anchor-point provenance record (content + the
  `anchor_fp` binding), and the `BootstrapState` live-follow start tip resolution
  (`crates/ade_runtime/src/bootstrap.rs` + the anchor-point store surface): the authoritative,
  replay-equivalent recovery decision. The *write* at recover is RED I/O of a BLUE-authoritative
  record; the *load + bind + resolve* is BLUE.
- **Canonical input** — the recovered anchor `seed_point` (`BootstrapAnchor`).
- **RED (unchanged)** — `spawn_live_wire_pump_source` / the wire pump (`node_lifecycle.rs`,
  `wire_pump.rs`).
- **BLUE (AK-S2)** — the single-producer rollback-resolution predicate in `run_node_sync`
  (`node_sync.rs`): pure `(peer_point, recovered_anchor) → {AnchorNoop, FailClosed}`. The *threading*
  of `BootstrapState.tip` into `run_node_sync` (`node_lifecycle.rs`) is RED wiring of a BLUE input.
- **RED / unchanged (AK-S2)** — `pump_block`, `block_validity` (the forward admit — already enforces
  the first-forward link); `run_participant_sync` (the participant rollback-follow — a separate
  follow-on, NOT touched).
- **Out of scope** — `ChainDb::tip()` (storage contract); ledger materialization (`bootstrap.rs:216`,
  OQ-AK-2); N-AJ evidence. AK-S2 brings ONLY the single-producer `run_node_sync` recovered-anchor
  rollback case into scope — NOT general stored-block rollback-follow (OQ-AK-3 otherwise preserved).

## Forbidden during this cluster (slices inherit)

Weakening AI-S4a · modifying peer/relay behavior · special-casing the venue harness · making ChainDb
invent/synthesize a servable block · using WAL `admit_count` (or any guess) as the anchor point ·
**using CLI re-supply as the durable restart fix (warm-start must be store-derived; CLI seed-point is
first-run input only)** · touching N-AJ evidence emission · altering ledger materialization
(`bootstrap.rs:216`) unless a test proves dependence (OQ-AK-2) · redesigning admission orchestration
beyond AK-S2's narrow single-producer rollback case (OQ-AK-3) · flipping CN-CONS-03.

**AK-S2 additionally forbids:** a blanket rollback no-op (the ONLY accepted rollback is the recovered
anchor) · accepting a rollback by slot ALONE or hash ALONE (must bind BOTH to the persisted anchor) ·
synthesizing a ChainDb block for the anchor or marking it servable · re-reading the anchor from the
store inside `run_node_sync` (consume the already-loaded `BootstrapState.tip`) · changing `pump_block`
/ `block_validity` (the forward admit) · changing the participant path (`run_participant_sync`) ·
adding general stored-block rollback-follow on the single-producer path · weakening AI-S4a
(`RollBackward(Origin)` stays fail-closed).

## Registry declarations (this cluster-doc appends as `declared`)

- **DC-NODE-31** (family DC, derived, `introduced_in = PHASE4-N-AK`, status `declared`) — statement as
  the Primary invariant above (verbatim, incl. *"persisted as replayable recovery provenance"* and the
  storage-boundary phrases). `tests = []` (AK-S1 populates the named tests); `ci_script = ""`
  (Rust-test-enforced; CE-AK-3 is the operator-run live verification).
- **DC-NODE-32** (family DC, derived, `introduced_in = PHASE4-N-AK`, status `declared`) — statement as
  the second Primary invariant above (the single-producer recovered-anchor rollback-to-intersection
  no-op). The first-forward "link" is a **non-goal / proof note**, NOT a registry clause — the
  OQ-AK-S2-2 live probe proved the existing `pump_block` + recovered `chain_dep` already admits the
  first forward block. `tests = []` (AK-S2 populates the named tests); `ci_script = ""`
  (Rust-test-enforced; CE-AK-3 is the operator-run end-to-end verification). **Appended to the registry
  (declared) after the OQ-AK-S2 investigation** (358 rules, coherence gate green).
- Strengthening note (do **not** flip now): T-REC-05 may gain the recovered-tip replay test in its
  `tests` at AK close (`strengthened_in += PHASE4-N-AK`); DC-NODE-31 + DC-NODE-32 flip `declared` /
  `enforced_scaffolding` → `enforced` at close after CE-AK-3 passes end-to-end.

## Close-record note (preserve verbatim at `/cluster-close`)

> **AK persists the recovered anchor point as replayable recovery provenance and resolves the
> live-follow start from it (AK-S1), AND accepts the relay's post-intersection rollback-to-that-anchor
> as an idempotent boundary rewind so the recover→follow completes through the existing `pump_block`
> (AK-S2). It does NOT make ChainDb serve the anchor as a block, does NOT weaken rollback-to-Origin
> rejection, does NOT depend on CLI re-supply at restart, does NOT change `pump_block` / `block_validity`
> / the participant path, and does NOT claim full ChainSel convergence.**
