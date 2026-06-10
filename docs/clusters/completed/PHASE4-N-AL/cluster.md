# Invariant Cluster — PHASE4-N-AL — Participant Recovered-Anchor Boundary (CE-AI-6 unblock)

> NARROW follow-on to PHASE4-N-AK: the participant-path **MIRROR** of `DC-NODE-32`. AK-S2 fixed the
> bare-anchor recover→follow rollback boundary for the SINGLE-PRODUCER path (`run_node_sync`) and
> explicitly deferred the PARTICIPANT path (`run_participant_sync`) as *"OQ #4 — a separate
> follow-on."* **CE-AI-6 runs the participant path**, so it is BLOCKED until this lands: a bare-anchor
> participant recover halts at the relay's standard post-`IntersectFound` `RollBackward(anchor)`
> (`get_block_by_hash(anchor)→None→UnexpectedRollback`, `node_lifecycle.rs:2560-2582`) **before
> admitting any block**.
>
> Confirmed by code read (HEAD `c3ec7466`): `run_node_sync` matches `state.recovered_anchor`
> (`node_sync.rs:479-486`, DC-NODE-32); `run_participant_sync` has no such branch. The anchor is
> ALREADY available on the participant path (`ForwardSyncState.recovered_anchor`, set at
> `node_lifecycle.rs:563` in the forge-ON arm, which a participant venue takes; threaded to both
> dispatches by `run_relay_loop_with_sched`) — only the USE is missing.
>
> **Single slice (AL-S1).** The cluster closes when the participant bare-anchor recover→follow reaches
> the first forward block cleanly (CE-AL-3-LIVE preflight). The downstream CE-AI-6 reorg/convergence
> pass is the CONSUMER, **not** this cluster's bar — and runs ONLY after this closes (the bright-red
> gate: *no CE-AI-6 run until participant bare-anchor recover→follow reaches the first forward block
> cleanly*).

## Primary invariant

**DC-NODE-33** (declared here, targeted **enforced** at close): *On the participant live-follow path
(`run_participant_sync`), a peer `RollBackward` whose target binds EXACTLY (slot AND hash) to the
persisted recovered anchor point (DC-NODE-31 / `BootstrapState.tip`, carried in
`ForwardSyncState.recovered_anchor`) is accepted as an idempotent no-op boundary rewind — no
`commit_rollback`, no `WalEntry::RollBack`, no ChainDb / ledger / chain_dep mutation, no cursor, no
`pending_reselection`. Evaluated BEFORE the existing DC-NODE-29 stored-block resolution:
`RollBackward(Origin)` still fails closed (AI-S4a); every non-anchor, non-Origin rollback still
resolves through the EXISTING DC-NODE-29 authority UNCHANGED; the accepted point binds to the
PERSISTED anchor on slot AND hash, never peer-supplied alone. The anchor consumed by
`run_participant_sync` is the single authority (`state.recovered_anchor`, set once at
`node_lifecycle.rs:563` — never re-read from the store inside the loop). The anchor is a recovery
snapshot boundary, NEVER synthesized into a ChainDb block or served. The first forward block after the
anchor admits through the EXISTING sole `pump_block` path (no new forward-link code). Replay-equivalent
(extends T-REC-05 / DC-NODE-31 / DC-NODE-32 to the participant follow). The participant MIRROR of
DC-NODE-32; DC-NODE-32 stays scoped to `run_node_sync`.*

**CN-CONS-03 untouched — stays `declared`.** AL restores the participant recover→follow so the CE-AI-6
operator pass becomes runnable; AL does **not** emit convergence evidence or flip CN-CONS-03.

**DC-NODE-32 NOT broadened — stays scoped to `run_node_sync`.** AL is a distinct **sibling** rule
(cross-ref, not strengthening): re-scoping DC-NODE-32 after its deliberate single-producer scoping
would muddy the audit trail.

## Normative anchors

- `docs/planning/phase4-n-al-participant-recovered-anchor-boundary-invariants.md` (AL-1..5, the OQ-AL
  resolution, prohibitions, the acceptance bar).
- DC-NODE-32 (AK-S2 — the single-producer twin this mirrors; `node_sync.rs:479-486`).
- DC-NODE-31 (AK-S1 — the persisted anchor + `BootstrapState.tip` this consumes via
  `state.recovered_anchor`).
- DC-NODE-23..29 (N-AI participant rollback-follow + AI-S6 stored-block authority + AI-S4a Origin
  fail-close `crates/ade_runtime/src/admission/wire_pump.rs:447` — all **preserved**; the new branch
  sits BEFORE the DC-NODE-29 resolution).
- DC-NODE-30 (N-AJ participant convergence evidence — untouched; the anchor no-op is a rollback, not a
  `Block`, emits nothing).
- T-REC-05 (replay-equivalence — extended to the participant follow).

## Entry Conditions (guaranteed by prior clusters)

- **N-AK (DC-NODE-31/32):** the persisted recovered anchor point + `BootstrapState.tip` + the
  `ForwardSyncState.recovered_anchor` field + the forge-ON-arm set at `node_lifecycle.rs:563`.
- **N-AI (DC-NODE-23..29):** the participant rollback-follow (`apply_chain_event` /
  `classify_receive` / `resolve_disposition`) + AI-S4a + AI-S6 stored-chain-point binding.
- **N-AJ (DC-NODE-30):** participant convergence evidence on the `Block` path.
- `run_relay_loop_with_sched` passes the SAME `ForwardSyncState` (`&mut fwd`, with `recovered_anchor`)
  to both `run_participant_sync` (1313) and `run_node_sync` (1326).

## Exit Criteria (CI-verifiable — named checks, not intent)

- **CE-AL-1** (anchor no-op, hermetic — `ade_node`): `participant_rollback_to_recovered_anchor_is_noop`
  — feed `[RollBack(anchor slot+hash)]` with `state.recovered_anchor = Some(anchor)`;
  `run_participant_sync` returns `Ok`, store byte-identical (no WAL append, no ChainDb/tip/chain_dep
  mutation, `pending_reselection` stays `false`).
- **CE-AL-2** (Origin fail-closed, hermetic): `participant_rollback_origin_fails_closed` —
  `RollBack(Origin)` ⇒ `UnexpectedRollback` even with `recovered_anchor = Some(..)`.
- **CE-AL-3** (binding strictness, hermetic): `participant_rollback_non_anchor_fails_closed` —
  different slot+hash, **slot-only match**, and **hash-only match** all ⇒ the existing resolution
  (`UnexpectedRollback` for an unknown hash), never the no-op (binds BOTH slot and hash).
- **CE-AL-4** (forward after no-op, hermetic):
  `participant_first_forward_after_anchor_noop_admits_via_pump_block` — feed `[RollBack(anchor),
  Block(valid_successor)]`; the anchor rollback no-ops and the successor admits through the existing
  `LinearExtend → pump_block` (the tip advances).
- **CE-AL-5** (stored-block path unchanged, hermetic): `participant_stored_block_rollback_still_applies`
  — a `RollBack` to an actually-stored block (`get_block_by_hash = Some`) still routes through
  `apply_chain_event` (DC-NODE-29) unchanged — the anchor branch did not capture it (proves DC-NODE-29
  preserved).
- **CE-AL-6** (no collateral): `cargo test -p ade_node` green; the existing `run_participant_sync` /
  fork-choice tests stay green; `recovered_anchor = None` callers behaviorally unchanged.
- **CE-AL-3-LIVE** (live preflight — operator-run at close; the **bright-red CE-AI-6 gate**): a FRESH
  recover at the current rung-2 venue tip, then `--mode node --participant-venue
  --convergence-evidence-path …` ⇒ FindIntersect at the anchor ⇒ the relay's `RollBackward(anchor)` is
  no-op'd ⇒ the FIRST forward block admits through `pump_block` ⇒ **0** `UnsupportedRollbackPoint` AND
  **0** `UnexpectedRollback` before the first admit. (The participant analog of CE-AK-3; it proves the
  recover→follow STARTS cleanly. The actual reorg/convergence capture is the downstream CE-AI-6 pass,
  gated on this.)

## Expected Slice Types

- **AL-S1** (participant rollback-resolution fix — the single slice; DC-NODE-33). In
  `run_participant_sync`'s `RollBack` handler (`node_lifecycle.rs:2560`), add — BEFORE the DC-NODE-29
  `get_block_by_hash` resolution — a branch: if the rollback point is `Point::Block { slot, hash }`
  and `Some(anchor) = &state.recovered_anchor` and `(slot, hash) == (anchor.slot, anchor.hash)` (slot
  AND hash), `continue` (idempotent no-op: no `commit_rollback`, no `WalEntry::RollBack`, no
  ChainDb/ledger/chain_dep/cursor mutation, `pending_reselection` NOT set); `Origin` and every other
  point fall through to the EXISTING resolution unchanged. Mirror `node_sync.rs:479-486`. **NO** new
  param (`state.recovered_anchor` already populated via `node_lifecycle.rs:563`); **NO** field add;
  **NO** forward-link code; **NO** change to `pump_block` / `block_validity` / `apply_chain_event` /
  `run_node_sync` / DC-NODE-32 / N-AJ evidence. Mechanical proof CE-AL-1..6 + CE-AL-3-LIVE.

## TCB Color Map (FC/IS Partition)

- **BLUE** — the participant rollback-resolution predicate in `run_participant_sync`
  (`node_lifecycle.rs`): pure `(peer_point, recovered_anchor) → {AnchorNoop, FallThrough}`. The
  persisted anchor is already-BLUE canonical state (DC-NODE-31).
- **Canonical input** — the recovered anchor point `state.recovered_anchor` (DC-NODE-31 /
  `BootstrapState.tip`).
- **RED (wiring)** — routing the `RollBack` item to the predicate (`node_lifecycle.rs`). No new
  field/param — reuses the AK-S2 `ForwardSyncState.recovered_anchor`.
- **RED / unchanged** — `pump_block`, `block_validity` (the forward admit), `apply_chain_event` (the
  DC-NODE-29 stored-block resolution), `spawn_live_wire_pump_source` / the wire pump, AI-S4a
  (`wire_pump.rs:447`), `ChainDb::tip()` / serve, `run_node_sync` (DC-NODE-32), N-AJ evidence.

## Forbidden during this cluster (slices inherit)

Broadening DC-NODE-32's scope (AL is the distinct sibling DC-NODE-33) · a blanket rollback no-op (the
ONLY accepted rollback is the recovered anchor) · accepting a rollback by slot ALONE or hash ALONE
(must bind BOTH to the persisted anchor) · synthesizing / serving the anchor as a ChainDb block ·
re-reading the anchor from the store inside `run_participant_sync` (consume `state.recovered_anchor`) ·
changing `pump_block` / `block_validity` / `apply_chain_event` / the DC-NODE-29 stored-block
resolution · changing `run_node_sync` · weakening AI-S4a (`RollBackward(Origin)` stays fail-closed) ·
touching N-AJ evidence emission · flipping CN-CONS-03 · **running the CE-AI-6 reorg/convergence pass
before CE-AL-3-LIVE proves the first clean forward admit (the bright-red gate).**

## Registry declarations (this cluster-doc appends as `declared`)

- **DC-NODE-33** (family DC, derived, `introduced_in = PHASE4-N-AL`, status `declared`) — statement as
  the Primary invariant above. `tests = []` (AL-S1 populates the named tests); `ci_script = ""`
  (Rust-test-enforced; CE-AL-3-LIVE is the operator-run live preflight). `cross_ref = [DC-NODE-32,
  DC-NODE-31, DC-NODE-29, DC-NODE-23, T-REC-05, CN-CONS-03]`. Appended to the registry (declared) after
  the OQ-AL investigation (359 rules, coherence gate green).
- **DC-NODE-32 explicitly NOT modified** (no `strengthened_in += PHASE4-N-AL`) — the participant
  parity is a NEW rule, not a re-scoping; DC-NODE-32 was deliberately scoped to `run_node_sync` after
  investigation, and re-scoping it after the fact muddies the audit trail.
- Strengthening note (do **not** flip now): T-REC-05 may gain the participant-tip replay test in its
  `tests` at AL close (`strengthened_in += PHASE4-N-AL`); DC-NODE-33 flips `declared` → `enforced` at
  close after CE-AL-1..6 + CE-AL-3-LIVE pass.

## Close-record note (preserve verbatim at `/cluster-close`)

> **AL adds the participant-path mirror of DC-NODE-32:** `run_participant_sync` accepts the relay's
> post-`IntersectFound` rollback-to-the-recovered-anchor as an idempotent boundary rewind (exact slot
> AND hash) so the participant recover→follow completes through the existing `pump_block`. It does NOT
> broaden DC-NODE-32, synthesize/serve the anchor as a block, weaken `RollBackward(Origin)` rejection,
> change the DC-NODE-29 stored-block rollback authority, touch N-AJ evidence, or flip CN-CONS-03. It
> **unblocks — but does NOT run** — the CE-AI-6 convergence pass.
