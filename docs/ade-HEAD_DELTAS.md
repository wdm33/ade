# HEAD Deltas — Ade

> **Status:** Living architectural document. Regenerated; not hand-edited.
> Regenerate via `/head-deltas <baseline>`. Baseline is declared in
> `.idd-config.json` (`head_deltas_baseline`).

> Baseline: `d509f02` (Phase 3 handoff snapshot, 2026-04-15)
> HEAD: `75f75da` (feat(ledger): wire RollBackward → close DC-CONS-20 (PHASE4-N-I S6), 2026-05-26)
> 177 commits, 11,447 files changed, +207,847 / −7,233,633 lines

Headline numbers note: the massive negative line count is dominated by
the **corpus relayout** under `corpus/snapshots/` and the deletion of
the multi-MB credentialed-snapshot text files
(`*_registered_creds.txt`, ~7M lines combined). Source-tree deltas are
far smaller — the per-crate breakdown in §3 is the representative view.

> **Commit-hash note.** This regen runs against the current (rebased)
> history. Earlier HEAD_DELTAS regens referenced commit hashes from a
> history that has since been rewritten; all hashes below are verbatim
> from `git log d509f02..HEAD` at this HEAD.

> **PHASE4-N-I cluster close note (newest thread).** This regen is cut
> at HEAD `75f75da`. Since the prior grounding-doc refresh `f143984`
> (which closed PHASE4-N-H — archive cluster + refresh
> CODEMAP/SEAMS/HEAD_DELTAS/TRACEABILITY), **six new commits have
> landed** — the **PHASE4-N-I cluster** (S1 → S6) shipping the
> in-memory snapshot + replay-forward rollback infrastructure that
> closes the rollback-side `open_obligation` left dangling by N-H. N-I
> is the rollback half of N-H: where N-H's receive reducer returned
> `Err(ReceiveError::RollbackOutOfScope)` on every peer `RollBackward`
> per the Path A scope split, N-I introduces the BLUE rollback module
> tree (`ade_ledger::rollback` with `traits`, `error`, `materialize`,
> `commit`), the GREEN/RED runtime support
> (`ade_runtime::rollback` with `cadence`, `in_memory_cache`,
> `chaindb_block_source`, `snapshot_writer`), and the S6 wiring that
> flips `DC-CONS-20` from `declared` (admit-side only) to `enforced`
> (atomic admit-side + rollback-side over ChainDb + LedgerState +
> PraosChainDepState). Sequence: `0e7e9ee` (S1, **BLUE foundation** —
> new submodule `ade_ledger::rollback` with two new files:
> `traits.rs` (~88 LOC) declaring the narrow read-only `SnapshotReader`
> + `BlockSource` traits the materialization driver depends on
> (production impls live in `ade_runtime::rollback`; tests pass
> in-memory fakes through the same single composition);
> `error.rs` (~98 LOC) declaring closed sums `MaterializeError`
> (`RollbackTooDeep`, `BlockNotInRange`, `BlockValidityFailed`) +
> `CommitRollbackError` (`Materialize`, `ChainWriteFailed`). Module
> registered in `lib.rs`. No CI gate yet — the closure CI gates are
> introduced when the bodies land in S2/S4). `0efdce3` (S2, **BLUE
> materialization driver** — new file
> `ade_ledger::rollback::materialize` (~402 LOC) declaring the SOLE
> `pub fn` in the project returning the rolled-back
> `(LedgerState, PraosChainDepState)` tuple:
> `materialize_rolled_back_state(snapshot_reader, block_source, target,
> era_schedule, ledger_view) -> Result<(LedgerState, PraosChainDepState),
> MaterializeError>`. Single-authority discipline (CN-STORE-07):
> looks up nearest-LE snapshot via `SnapshotReader::nearest_le`,
> reads blocks in `(snapshot_slot, target_slot]` via
> `BlockSource::blocks_in_range`, replays via the existing
> `block_validity` authority (the same authority N-H's admit branch
> uses) wrapped in `apply_block_with_verdicts` (which inherits the
> unique epoch-boundary authority per `rules.rs:244-250` — no parallel
> epoch path). Pure replay-forward fold; no I/O. New CI gate
> `ci_check_rollback_materialize_closure.sh` (~80 LOC). **Registry**:
> `DC-CONS-22` (replay-forward correctness) + `CN-STORE-07` (single
> materialize authority) flipped to `enforced` at S2). `02b5e31`
> (S3, **BLUE atomic commit helper + ChainDbWrite trait extension** —
> new file `ade_ledger::rollback::commit` (~200 LOC):
> `commit_rollback<W: ChainDbWrite>(state, target_slot, new_ledger,
> new_chain_dep, chain_write) -> Result<(), CommitRollbackError>` —
> irreversible-step-first staged commit shape: calls
> `chain_write.rollback_to_slot(target_slot)` first (the only
> non-reversible step); if it fails, `state` is unchanged and the
> error is returned; if it succeeds, swaps in `(new_ledger,
> new_chain_dep)` and resets `state.pending_headers`. The
> `ChainDbWrite` trait (N-H S1) is extended with a second method
> `rollback_to_slot(slot) -> Result<(), ChainWriteError>` that
> discards all blocks at slots strictly greater than `slot` (rolling
> beyond the empty tip is `Ok(())` per the ChainDb contract). All
> existing GREEN `in_memory_chain_write` + test fakes updated. No new
> CI gate; closure is folded into the S2 gate + the existing
> N-H `ci_check_receive_reducer_closure.sh` after S6). `3a9bab8` (S4,
> **GREEN snapshot cadence + InMemorySnapshotCache + ChainDbBlockSource**
> — new submodule `ade_runtime::rollback` with four new files:
> `cadence.rs` (~143 LOC, GREEN — pure `SnapshotCadence` struct with a
> single BLUE-structural field `every_n_blocks: u32` + pure decision
> `should_snapshot_after_block(slot, block_no, params, last_snapshot)
> -> bool`; operator-tunable cadence is explicitly out of scope per
> DC-STORE-07), `in_memory_cache.rs` (~165 LOC, GREEN — BTreeMap-keyed
> `InMemorySnapshotCache` implementing `SnapshotReader` with
> `nearest_le` returning the largest key ≤ target; deterministic
> iteration; no HashMap), `chaindb_block_source.rs` (~113 LOC, GREEN —
> borrow-wrapper around `&dyn ChainDb` implementing the BLUE
> `BlockSource` trait via `ChainDb::iter_from_slot`; pure projection,
> no I/O), `snapshot_writer.rs` (~146 LOC, **placeholder for S5**).
> Module registered in `lib.rs`. New CI gate
> `ci_check_snapshot_cadence_purity.sh` (~77 LOC). **Registry**:
> `DC-STORE-07` (snapshot cadence determinism) flipped to `enforced`
> at S4). `e7add4d` (S5, **GREEN snapshot-write hook** —
> `ade_runtime::rollback::snapshot_writer::maybe_capture_snapshot`
> wires the cadence decision (S4) to the InMemorySnapshotCache (S4),
> giving the scheduler / receive orchestrator a single point at which
> to capture a snapshot of post-block-admit state. Pure: takes
> `(cache, cadence, slot, block_no, state)` and conditionally calls
> `cache.capture_from(slot, state)`. No new file beyond the S4
> placeholder; no new CI gate; closure covered by the S4 cadence-purity
> gate). `75f75da` (S6, **BLUE wiring — DC-CONS-20 closure** —
> `ade_ledger::receive::reducer` extended with a new public type
> `RollbackContext<'a>` (carrying `&'a dyn SnapshotReader` +
> `&'a dyn BlockSource`) + a new `Option<&RollbackContext>` parameter
> on `receive_apply`. When `Some`, the `RollBackward` arm calls the new
> private function `roll_backward(state, target_point, chain_write,
> era_schedule, ledger_view, ctx)`, which composes
> `materialize_rolled_back_state` + `commit_rollback` atomically:
> failure leaves state unchanged; success swaps ledger + chain_dep +
> ChainDb tip atomically + resets pending headers. When `None`, the
> arm retains the legacy `Err(RollbackOutOfScope)` behavior for
> callers that haven't wired the rollback context yet. New integration
> test `crates/ade_runtime/tests/receive_rollback_integration.rs`
> (~445 LOC) drives the full pipeline: admit blocks → snapshot →
> peer RollBackward → materialize + commit → continue admitting →
> assert resulting ledger fingerprint equals the straight-line admit
> fingerprint (`rollback_then_continue_admit_equals_straight_line_admit`
> — the canonical DC-CONS-22 evidence). `ade_runtime::receive::orchestrator`
> + `in_memory_chain_write` updated for the new trait method. **No new
> CI gate**; the existing `ci_check_receive_reducer_closure.sh` +
> `ci_check_rollback_materialize_closure.sh` cover the new wiring.
> **Registry**: `DC-CONS-20` flipped from `declared` (admit-side only)
> to `enforced` (admit-side + rollback-side, lockstep over ChainDb +
> LedgerState + PraosChainDepState); `cluster = "PHASE4-N-H + PHASE4-N-I"`;
> `strengthened_in = ["PHASE4-N-I"]`; `open_obligation` removed.
> **DC-CONS-21** (persistent snapshot encode/decode round-trip) remains
> `declared` with `open_obligation = "persistent_ledger_snapshot_encoding_follow_on_cluster"`
> per the explicit cluster scope decision: N-I ships an **in-memory**
> SnapshotReader; persistent on-disk encoding is carved out to a
> follow-on cluster because the full canonical LedgerState encoder
> (~1500-2000 LoC of field-walk code mirroring `ade_ledger::fingerprint`)
> is too large to ship in one slice. **One new BLUE submodule**
> (`ade_ledger::rollback` with `traits` + `error` + `materialize` +
> `commit`), **one new GREEN+RED submodule** (`ade_runtime::rollback`
> with `cadence` (GREEN) + `in_memory_cache` (GREEN) +
> `chaindb_block_source` (GREEN) + `snapshot_writer` (GREEN)), **one
> ChainDbWrite trait extension** (`rollback_to_slot` method added in
> S3 to the N-H BLUE trait), **4 new registry rules appended at
> sketch close, with effects on cluster close** (`DC-CONS-21`
> `declared` with persistent-encoding `open_obligation`; `DC-CONS-22`
> `enforced` at S2; `CN-STORE-07` `enforced` at S2; `DC-STORE-07`
> `enforced` at S4) + **1 N-H rule status flip** (`DC-CONS-20`
> `declared` → `enforced` at S6, removing its
> `open_obligation`), **2 new CI scripts** (the 53rd → 54th:
> `ci_check_rollback_materialize_closure.sh`,
> `ci_check_snapshot_cadence_purity.sh`), **no new cross-crate dep
> edges** — the N-C `ade_runtime → ade_ledger` production edge
> carries the new BLUE `rollback::*` types; the N-D ChainDb traits
> are reused by `ChainDbBlockSource`. **One `open_obligation` retained**:
> `DC-CONS-21` (persistent ledger snapshot encoding, blocked on
> follow-on cluster). **Cluster status at HEAD: closed mechanically;
> DC-CONS-21 persistent-encoding half deferred to a follow-on cluster
> per the explicit scope decision**, mirroring the N-H Path A scope
> split pattern. **Cluster directory NOT YET archived** to
> `docs/clusters/completed/PHASE4-N-I/` — still at active
> `docs/clusters/PHASE4-N-I/` (8 files: `cluster.md` + `N-I-S1.md`
> through `N-I-S6.md`; planning spillovers
> `docs/planning/phase4-n-i-cluster-slice-plan.md` +
> `docs/planning/ledger-snapshot-rollback-invariants.md`). **No
> CODEMAP/SEAMS/TRACEABILITY refresh yet** for the N-I cluster —
> those three docs are stale relative to this HEAD_DELTAS regen and
> must be regenerated in the grounding ripple immediately following.

> **PHASE4-N-H cluster close note (prior thread, carried forward).**
> Closed at HEAD `efe1fb9` and archived to
> `docs/clusters/completed/PHASE4-N-H/` (8 files) by `f143984`.
> Six slices S1 → S6 shipped the receive-side header→body bridge
> under the Path A scope split (admit-only; rollback deferred to N-I).
> One new BLUE submodule `ade_ledger::receive` (with `admitted` +
> `chain_write` + `events` + `pending_header_cache` + `reducer`), one
> new GREEN+RED submodule `ade_runtime::receive` (with `events_to_state`
> + `in_memory_chain_write` + `orchestrator`), one new RED binary
> `live_block_follow_session`. 6 new registry rules (`CN-CONS-08`,
> `DC-CONS-19`, `DC-CONS-20`, `DC-PROTO-09`, `CN-PROTO-07`,
> `RO-LIVE-02`); 1 strengthening (`DC-PROTO-06`); 5 new CI scripts;
> no new cross-crate dep edges. Two `open_obligation` entries
> recorded at N-H close: `DC-CONS-20`
> (`rollback_side_blocked_until_ledger_snapshot_cluster`) — **closed
> by PHASE4-N-I S6** — and `RO-LIVE-02`
> (`blocked_until_operator_peer_available`).

> **PHASE4-N-G cluster close note (prior thread, carried forward).**
> Closed at HEAD `a280954` and archived to
> `docs/clusters/completed/PHASE4-N-G/` (10 files) by `2adfb45`.
> Seven slices S1 → S7 shipped the producer-side block-fetch +
> chain-sync server response paths — the "engineering bridge"
> between N-C's broadcast-queue output and a real Haskell
> cardano-node peer's RequestRange / RequestNext. Three new BLUE
> submodules (`ade_ledger::producer::served_chain`,
> `ade_network::chain_sync::server`, `ade_network::block_fetch::server`),
> two new GREEN files (`ade_runtime::producer::broadcast_to_served`,
> `ade_runtime::producer::served_chain_lookups`), two new RED files
> (`ade_runtime::network::mod` + `ade_runtime::network::n2n_server`),
> one new RED binary (`live_block_fetch_session`), 6 new registry
> rules (`DC-CONS-17/18`, `DC-PROTO-07/08`, `CN-PROTO-06`,
> `RO-LIVE-01`), 2 strengthenings (`CN-CONS-07`, `DC-PROTO-06`), 7
> new CI scripts, one new non-dev Cargo dep edge `ade_runtime →
> ade_network` (S6) + four new dev-dep edges on `ade_network` (S3).
> One `open_obligation` recorded: `RO-LIVE-01`
> (`blocked_until_operator_peer_available`).

> **PHASE4-N-C cluster close note (prior thread, carried forward).**
> Closed at HEAD `694dd74` and archived (CODEMAP/SEAMS/HEAD_DELTAS/TRACEABILITY
> refresh) at `df56e2d`. Seven slices S1 → S7 + two follow-ups shipped
> the last Tier-1 bounty deliverable — block-production authority, the
> validation→producer leap. New BLUE submodule `ade_ledger::producer`
> (`forge` + `state` + `self_accept`), new BLUE module
> `ade_ledger::block_body_hash` (single canonical body-hash authority),
> new BLUE module pair `ade_core::consensus::opcert_validate` +
> `ade_codec::shelley::opcert`, new RED submodule
> `ade_runtime::producer` (`signing` + `keys` + `scheduler` +
> `broadcast`) + GREEN `tick_assembler`, new RED binary
> `live_block_production_session`. 14 new registry rules; 8 new CI
> scripts; one new Cargo dep edge `ade_runtime → ade_ledger`. CE-N-C-8
> open as `CN-CONS-06.open_obligation`
> (`blocked_until_operator_stake_available`).

> **PROPOSAL-PROCEDURES-DECODE cluster close note (prior thread,
> carried forward).** Closed at HEAD `928c2be` and archived to
> `docs/clusters/completed/PROPOSAL-PROCEDURES-DECODE/` by `96d043c`.
> Two slices PP-S1 + PP-S2 introduced the new BLUE module
> `ade_codec::conway::governance` + new closed type
> `ProposalProcedure` + body-codec rewire at key 20 + new CI gate
> `ci_check_proposal_procedures_closed.sh` + new rule `DC-LEDGER-11`
> + a GREEN canonical synthetic-corpus replay harness
> `ade_testkit::governance::proposal_procedures_replay`.

> **PHASE4-N-E cluster close note (prior thread, carried forward).**
> Closed at HEAD `caa5ce8` and archived to
> `docs/clusters/completed/PHASE4-N-E/` (10 files) by `3af9e2b`.
> Tier-1 authority closed; CE-N-E-6 live N2N evidence captured at
> `CE-N-E-6_2026-05-25.log`. CE-N-E-7 deferred as cross-cluster
> obligation `CE-NODE-N2C-LTX`. Six implementation commits shipped
> the BLUE `mempool::ingress` chokepoint, the GREEN
> `mempool::canonicalize` ordering function, the GREEN
> `testkit::mempool::ingress_replay` harness, the two GREEN bridges
> (`ade_core_interop::tx_submission`, `ade_core_interop::local_tx_submission`),
> and the RED `live_tx_submission_session` probe binary. Two
> registry rules (`DC-MEM-03`, `DC-MEM-04`); two CI scripts; one
> new Cargo dep edge (`ade_core_interop → ade_ledger`).

> **Testkit follow-up note (prior thread, carried forward).** Four
> GREEN-scope commits between WRITEBACK refresh `3d94c22` and refresh
> `52642e5` — bounded to `ade_testkit` / corpus tooling, no BLUE
> source change, no new rule, no new CI script. `DC-EPOCH-01` and
> `DC-LEDGER-10` each gained one oracle test.

> **ENACTMENT-COMMITTEE-WRITEBACK cluster note (prior thread, carried
> forward).** Three implementation commits + close-hardening +
> grounding refresh; live committee write-back without a new
> module/rule/CI script.

> **ENACTMENT-COMMITTEE-FIDELITY / DREP-VOTE-FIDELITY /
> COMMITTEE-CRED-FIDELITY / OQ5-CREDENTIAL-FIDELITY cluster notes
> (prior threads, carried forward).** All structural changes,
> fingerprint surfaces, and credential-discriminant ripples unchanged
> at this HEAD.

> **B5 / B4 / B3F / B3 / B2 / B1 / N-D / N-B / N-A / PHASE4-N-E /
> PROPOSAL-PROCEDURES-DECODE / PHASE4-N-C / PHASE4-N-G / PHASE4-N-H
> cluster notes (carried forward).** All closed and archived at
> `docs/clusters/completed/<NAME>/`.

The delta now covers twenty-nine threads of work. The newest thread —
the **PHASE4-N-I cluster** (`0e7e9ee` → `75f75da`, 6 commits) — sits
on the post-N-H grounding refresh `f143984`, which closed +
archived PHASE4-N-H. In rough proportion of the substantive change
budget:

0. **PHASE4-N-I (in-memory snapshot + replay-forward rollback — the
   rollback half of PHASE4-N-H, closing the
   `DC-CONS-20.open_obligation = "rollback_side_blocked_until_ledger_snapshot_cluster"`
   that N-H carried; in-memory scope only, persistent on-disk encoding
   deferred to a follow-on cluster per the explicit scope decision) —
   closed in 6 slices.** S1 (`0e7e9ee`, **BLUE foundation**)
   introduces the new submodule `ade_ledger::rollback` with two new
   files: `traits.rs` (narrow read-only `SnapshotReader` +
   `BlockSource` traits — production impls live in
   `ade_runtime::rollback` per S4; tests pass in-memory fakes through
   the same single composition) and `error.rs` (closed sums
   `MaterializeError::{RollbackTooDeep, BlockNotInRange,
   BlockValidityFailed}` + `CommitRollbackError::{Materialize,
   ChainWriteFailed}` — no `Other`/`Misc` catchalls). Module
   registered in `lib.rs`. No new CI gate yet; closure gates land
   when the bodies do in S2/S4. **Registry**: 4 rules appended at
   `declared` — `DC-CONS-21`, `DC-CONS-22`, `CN-STORE-07`,
   `DC-STORE-07` (202 → 206 entries at HEAD).
   S2 (`0efdce3`, **BLUE materialization driver**) declares the SOLE
   `pub fn` in the project returning the rolled-back
   `(LedgerState, PraosChainDepState)` tuple in new file
   `ade_ledger::rollback::materialize`:
   `materialize_rolled_back_state(snapshot_reader, block_source,
   target, era_schedule, ledger_view) -> Result<(LedgerState,
   PraosChainDepState), MaterializeError>`. Single-authority discipline
   (CN-STORE-07): one `SnapshotReader::nearest_le` lookup; one
   `BlockSource::blocks_in_range` read for the slot range
   `(snapshot_slot, target_slot]`; per-block replay via the existing
   `block_validity` authority (the same authority N-H's admit branch
   uses) wrapped in `apply_block_with_verdicts`, which inherits the
   unique epoch-boundary authority per `rules.rs:244-250` — no
   parallel epoch path. Pure replay-forward fold. New CI gate
   `ci_check_rollback_materialize_closure.sh`. Registry:
   `CN-STORE-07` + `DC-CONS-22` flipped to `enforced`.
   S3 (`02b5e31`, **BLUE atomic commit helper + ChainDbWrite trait
   extension**) introduces new file `ade_ledger::rollback::commit`:
   `commit_rollback<W: ChainDbWrite>(state, target_slot, new_ledger,
   new_chain_dep, chain_write) -> Result<(), CommitRollbackError>` —
   irreversible-step-first staged commit shape. The
   `ChainDbWrite::rollback_to_slot(slot)` call is the only
   non-reversible step; on failure, `state` is unchanged and the
   error returns; on success, ledger + chain_dep swap atomically and
   `state.pending_headers` resets. The narrow N-H BLUE `ChainDbWrite`
   trait is extended with a second method `rollback_to_slot(slot) ->
   Result<(), ChainWriteError>` discarding all blocks at slots
   strictly greater than `slot` (rolling beyond empty tip is `Ok(())`
   per the ChainDb contract). All existing GREEN
   `in_memory_chain_write` + test fakes updated for the new method.
   No new CI gate; closure folds into S2's gate + N-H's
   `ci_check_receive_reducer_closure.sh` after S6.
   S4 (`3a9bab8`, **GREEN snapshot cadence + InMemorySnapshotCache +
   ChainDbBlockSource**) introduces the new submodule
   `ade_runtime::rollback` with four new files: `cadence.rs`
   (`SnapshotCadence` struct — single BLUE-structural field
   `every_n_blocks: u32`; pure decision `should_snapshot_after_block`;
   operator-tunable cadence explicitly out of scope per DC-STORE-07
   until represented as anchored, replay-derivable runtime data),
   `in_memory_cache.rs` (BTreeMap-keyed `InMemorySnapshotCache`
   implementing the BLUE `SnapshotReader` trait via `nearest_le`
   returning the largest key ≤ target; deterministic iteration; no
   HashMap), `chaindb_block_source.rs` (borrow-wrapper around
   `&dyn ChainDb` implementing `BlockSource` via
   `ChainDb::iter_from_slot`; pure projection, no I/O),
   `snapshot_writer.rs` (placeholder filled in S5). Module registered
   in `lib.rs`. New CI gate `ci_check_snapshot_cadence_purity.sh`.
   Registry: `DC-STORE-07` flipped to `enforced`.
   S5 (`e7add4d`, **GREEN snapshot-write hook**) fills
   `ade_runtime::rollback::snapshot_writer` with
   `maybe_capture_snapshot(cache, cadence, slot, block_no, state)` —
   pure, conditionally calls `cache.capture_from(slot, state)` when
   the cadence decision returns true. Gives the scheduler / receive
   orchestrator a single point at which to capture a snapshot of
   post-block-admit state. No new file beyond the S4 placeholder; no
   new CI gate.
   S6 (`75f75da`, **BLUE wiring — DC-CONS-20 closure**) extends
   `ade_ledger::receive::reducer` with a new public type
   `RollbackContext<'a>` (carrying `&'a dyn SnapshotReader` +
   `&'a dyn BlockSource`) + a new `Option<&RollbackContext>` parameter
   on `receive_apply`. When `Some`, the `RollBackward` arm calls the
   new private function `roll_backward(state, target_point,
   chain_write, era_schedule, ledger_view, ctx)`, which composes
   `materialize_rolled_back_state` + `commit_rollback` atomically:
   failure leaves state unchanged; success swaps ledger + chain_dep +
   ChainDb tip + resets pending headers in one structural transition.
   When `None`, the arm retains the legacy `Err(RollbackOutOfScope)`
   behavior for callers that haven't wired the rollback context yet.
   New integration test
   `crates/ade_runtime/tests/receive_rollback_integration.rs`
   (~445 LOC) covers the rollback paths end-to-end:
   `rollback_branch_returns_rolled_back_on_in_memory_snapshot`,
   `rollback_branch_returns_rollback_too_deep_when_no_snapshot`,
   `rollback_branch_state_unchanged_on_materialize_failure`,
   `rollback_then_continue_admit_equals_straight_line_admit` (the
   canonical DC-CONS-22 evidence — admit → snapshot → rollback →
   continue admit yields a ledger fingerprint byte-identical to a
   straight-line admit). `ade_runtime::receive::orchestrator` +
   `in_memory_chain_write` updated for the new trait method. **No new
   CI gate**; existing N-H + N-I S2 gates cover the wiring.
   Registry: **`DC-CONS-20` flipped from `declared` to `enforced`**;
   `cluster = "PHASE4-N-H + PHASE4-N-I"`;
   `strengthened_in = ["PHASE4-N-I"]`; `open_obligation` removed.
1. **PHASE4-N-H (receive-side header→body bridge — admit-only under
   the Path A scope split; the rollback half is now closed by N-I) —
   closed at HEAD `efe1fb9`, archived at `f143984`.** One new BLUE
   submodule `ade_ledger::receive`, one new GREEN+RED submodule
   `ade_runtime::receive`, one new RED binary
   `live_block_follow_session`. 6 new registry rules; 1 strengthening
   (`DC-PROTO-06`); 5 new CI scripts; no new cross-crate dep edges.
2. **PHASE4-N-G (producer-side block-fetch + chain-sync server
   response paths) — closed at HEAD `a280954`, archived at
   `2adfb45`.** Three new BLUE submodules, two new GREEN files, two
   new RED files, one new RED binary, 6 new registry rules, 7 new CI
   scripts; one new non-dev dep edge.
3. **PHASE4-N-C (last Tier-1 bounty deliverable — block-production
   authority) — closed at HEAD `694dd74`, archived at `df56e2d`.**
   New BLUE submodule `ade_ledger::producer`, new BLUE module
   `ade_ledger::block_body_hash`, new BLUE module pair
   `ade_core::consensus::opcert_validate` +
   `ade_codec::shelley::opcert`, new RED submodule
   `ade_runtime::producer` + GREEN `tick_assembler`, new RED binary
   `live_block_production_session`. 14 new rules; 8 new CI scripts;
   one new dep edge (`ade_runtime → ade_ledger`).
4. **PROPOSAL-PROCEDURES-DECODE (last open governance-domain decode
   seam) — closed in 2 slices.**
5. **PHASE4-N-E S6 (live N2N tx-submission2 evidence binary) —
   cluster close.**
6. **PHASE4-N-E S1–S5 (wire-level mempool ingress, Tier 1).**
7. **Post-WRITEBACK testkit follow-ups (four commits, GREEN-scope).**
8. **ENACTMENT-COMMITTEE-WRITEBACK — closed.**
9. **ENACTMENT-COMMITTEE-FIDELITY — closed.**
10. **DREP-VOTE-FIDELITY — closed.**
11. **COMMITTEE-CRED-FIDELITY — closed.**
12. **OQ5-CREDENTIAL-FIDELITY — closed.**
13. **Phase 4 cluster B5 (Conway gov-cert accumulation) — closed.**
14. **Phase 4 cluster B4 (Conway cert-state accumulation,
    fail-closed) — closed.**
15. **Phase 4 cluster B3F (follow-up hardening) — committed.**
16. **Phase 4 cluster B3 (Conway value-conservation accounting) —
    closed.**
17. **Phase 4 cluster B2 (tx validity agreement) — closed.**
18. **Phase 4 cluster B1 (full block validity agreement) — closed.**
19. **Phase 4 cluster N-A (network mini-protocols) — closed.**
20. **Phase 4 cluster N-B (consensus runtime) — closed.**
21. **CE-N-B-6 follow-mode bridge.**
22. **Phase 4 cluster N-D (ChainDB persistence) — closed.**
23. **Phase 2C close-out / CE-73 reclassification.**
24. **IDD canonicalization.**
25. **Grounding-doc generation + ripple.** Successive refreshes,
    including `52642e5`, `350130e`, `3af9e2b`, `96d043c`, `df56e2d`,
    `2adfb45`, `f143984`.
26. **BLUE-list drift closure.** Six CI scripts extended to full
    BLUE scope.
27. **Corpus relayout.** Credentialed `*_registered_creds.txt`
    removed (~7M-line negative); `corpus/snapshots/` now
    `.gitignore`-d.

---

## 1. Commit Log

| Hash | Type | Summary |
|------|------|---------|
| `75f75da` | feat | feat(ledger): wire RollBackward → close DC-CONS-20 (PHASE4-N-I S6) |
| `e7add4d` | feat | feat(runtime): snapshot-write hook maybe_capture_snapshot (PHASE4-N-I S5) |
| `3a9bab8` | feat | feat(runtime): GREEN snapshot cadence + InMemorySnapshotCache + ChainDbBlockSource (PHASE4-N-I S4) |
| `02b5e31` | feat | feat(ledger): commit_rollback atomic helper + ChainDbWrite trait extension (PHASE4-N-I S3) |
| `0efdce3` | feat | feat(ledger): materialize_rolled_back_state driver (PHASE4-N-I S2) |
| `0e7e9ee` | feat | feat(ledger): rollback traits + closed error sums (PHASE4-N-I S1) |
| `f143984` | docs | docs(grounding): close PHASE4-N-H — archive cluster + refresh CODEMAP/SEAMS/HEAD_DELTAS/TRACEABILITY |
| `efe1fb9` | feat | feat(interop): live_block_follow_session + CE-N-H-6 procedure (PHASE4-N-H S6) |
| `3973261` | test | test(runtime): mechanical cross-impl receive pipeline drive (PHASE4-N-H S5) |
| `1d06089` | feat | feat(runtime): RED N2N receive orchestrator (PHASE4-N-H S4) |
| `c584691` | feat | feat(runtime): GREEN events_to_state + in_memory_chain_write + transcript replay (PHASE4-N-H S3) |
| `0ecf22f` | feat | feat(ledger): receive_apply reducer composing block_validity (PHASE4-N-H S2) |
| `b019ee3` | feat | feat(ledger): AdmittedBlock token + receive closed sums (PHASE4-N-H S1) |
| `2adfb45` | docs | docs(grounding): close PHASE4-N-G — archive cluster + refresh CODEMAP/SEAMS/HEAD_DELTAS/TRACEABILITY |
| `a280954` | feat | feat(interop): mechanical cross-impl + live_block_fetch_session (PHASE4-N-G S7) |
| `f773b1c` | feat | feat(runtime): RED N2N server session driver (PHASE4-N-G S6) |
| `1a1b8e0` | feat | feat(runtime): GREEN broadcast->served adapter + transcript replay (PHASE4-N-G S5) |
| `03d120f` | feat | feat(network): block-fetch server reducer (PHASE4-N-G S4) |
| `cc49b1d` | feat | feat(network): chain-sync server reducers (PHASE4-N-G S3) |
| `dc069cf` | feat | feat(ledger): ServedChainSnapshot + served_chain_admit (PHASE4-N-G S2) |
| `8cd17c9` | feat | feat(network): header projection + closed ServerReply wrappers (PHASE4-N-G S1) |
| `df56e2d` | docs | docs(grounding): close PHASE4-N-C — archive cluster + refresh CODEMAP/SEAMS/HEAD_DELTAS/TRACEABILITY |
| `694dd74` | feat | feat(producer): mechanical cross-impl adapter + live_block_production_session binary (PHASE4-N-C S7) |
| `52b77c5` | chore | chore(lock): record Cargo.lock changes from N-C-S6 ade_runtime -> ade_ledger dep |
| `58678af` | feat | feat(producer): RED scheduler + GREEN tick-assembler + RED broadcast queue (PHASE4-N-C S6) |
| `aa7a7dd` | feat | feat(producer): BLUE self_accept bridge + AcceptedBlock type-level broadcast gate (PHASE4-N-C S5) |
| `4fd714c` | refactor | refactor(ledger): unify body-hash recipe into single canonical authority (PHASE4-N-C S4) |
| `8312690` | feat | feat(producer): BLUE forge core + ProducerTick + tx-admissibility prefix (PHASE4-N-C S3) |
| `4cf4b65` | feat | feat(consensus): BLUE opcert_validate + closed-grammar opcert encoder authority (PHASE4-N-C N-C-S2) |
| `9727bd9` | docs | docs(registry): record OP-OPS-04 open obligations from N-C-S1 closure |
| `ea9770e` | feat | feat(producer): RED signing primitives + cardano-cli skey loader (PHASE4-N-C S1) |
| `96d043c` | docs | docs(grounding): close PROPOSAL-PROCEDURES-DECODE — archive cluster + refresh CODEMAP/SEAMS/HEAD_DELTAS/TRACEABILITY |
| `928c2be` | test | test(testkit): proposal_procedures canonical corpus + replay harness (PROPOSAL-PROCEDURES-DECODE PP-S2) |
| `70bc85b` | feat | feat(codec): close proposal_procedures opacity at the Conway tx-body boundary (PROPOSAL-PROCEDURES-DECODE PP-S1) |
| `3af9e2b` | docs | docs(grounding): close PHASE4-N-E — archive cluster + refresh CODEMAP/SEAMS/HEAD_DELTAS/TRACEABILITY |
| `caa5ce8` | fix | fix(interop): retry-on-timeout + elapsed-time logging; CE-N-E-6 live evidence (PHASE4-N-E) |
| `d1068b3` | feat | feat(interop): live N2N tx-submission2 session binary (PHASE4-N-E S6) |
| `350130e` | docs | docs(grounding): refresh CODEMAP/SEAMS/HEAD_DELTAS/TRACEABILITY for PHASE4-N-E (partial close) |
| `43fcc31` | feat | feat(interop): N2C local-tx-submission -> mempool_ingress bridge (PHASE4-N-E S5) |
| `ca3f23a` | feat | feat(interop): N2N tx-submission2 -> mempool_ingress bridge (PHASE4-N-E S4) |
| `509d714` | feat | feat(ledger): per-peer ingress canonicalizer (PHASE4-N-E S3) |
| `2d0c918` | test | test(testkit): mempool ingress-replay harness + B-track corpus reuse (PHASE4-N-E S2) |
| `32c1ee6` | feat | feat(ledger): IngressEvent + mempool_ingress closed chokepoint (PHASE4-N-E S1) |
| `52642e5` | docs | docs(grounding): refresh CODEMAP/SEAMS/HEAD_DELTAS/TRACEABILITY + archive 7 closed cluster dirs |
| `168ac02` | fix | fix(testkit): snapshot-loader follow-ups (tip slot + Conway UMElem) |
| `c78ec76` | test | test(corpus): add reward_provenance generator (re-runnable, ignored) |
| `396664a` | test | test(corpus): align previously-blocked ade_testkit tests + ade_plutus compile with regenerated corpus |
| `b9cfaf9` | test | test(ledger): real-chain committee oracle, mainnet 575->576 (strengthens DC-EPOCH-01 + DC-LEDGER-10) |
| `3d94c22` | docs | docs(grounding): refresh CODEMAP/SEAMS/HEAD_DELTAS/TRACEABILITY + strengthen DC-EPOCH-01/DC-LEDGER-10 for ENACTMENT-COMMITTEE-WRITEBACK close |
| `69e2d4b` | test | test(ledger): harden update_committee decode + extend credential gate (ENACTMENT-COMMITTEE-WRITEBACK close) |
| `3180e27` | feat | feat(ledger): wire committee enactment write-back (ENACTMENT-COMMITTEE-WRITEBACK-S2) |
| `f2f15f9` | feat | feat(ledger): structured UpdateCommittee gov action (ENACTMENT-COMMITTEE-WRITEBACK-S1) |
| `ea25dd9` | docs | docs(ledger): ENACTMENT-COMMITTEE-WRITEBACK plan (wire committee enactment) |
| `3706534` | docs | docs(grounding): refresh CODEMAP/SEAMS/HEAD_DELTAS/TRACEABILITY for ENACTMENT-COMMITTEE-FIDELITY close |
| `a6b8de7` | feat | feat(ledger): discriminate EnactmentEffects.committee_changes (ENACTMENT-COMMITTEE-FIDELITY-S1) |
| `5d64fee` | docs | docs(ledger): ENACTMENT-COMMITTEE-FIDELITY plan (strengthens DC-LEDGER-10) |
| `06f517f` | docs | docs(grounding): refresh CODEMAP/SEAMS/HEAD_DELTAS/TRACEABILITY for DREP-VOTE-FIDELITY close |
| `62c9020` | test | test(ledger): DRep cross-resolve negative + CI gate, strengthen DC-LEDGER-10 (DREP-VOTE-FIDELITY-S2) |
| `ba4ff37` | feat | feat(ledger): discriminate drep_votes; exact-variant DRep stake resolution (DREP-VOTE-FIDELITY-S1) |
| `ecb0b92` | docs | docs(ledger): DREP-VOTE-FIDELITY plan (strengthens DC-LEDGER-10) |
| `a157c92` | docs | docs(grounding): refresh CODEMAP/SEAMS/HEAD_DELTAS/TRACEABILITY for COMMITTEE-CRED-FIDELITY close |
| `2aeea16` | test | test(ledger): committee cross-resolve negative + CI gate, strengthen DC-LEDGER-10 (COMMITTEE-CRED-FIDELITY-S2) |
| `2303a60` | feat | feat(ledger): discriminate committee member + vote credentials (COMMITTEE-CRED-FIDELITY-S1) |
| `32d7a2e` | docs | docs(ledger): COMMITTEE-CRED-FIDELITY plan (strengthens DC-LEDGER-10) |
| `676af5a` | docs | docs(grounding): refresh CODEMAP/SEAMS/HEAD_DELTAS/TRACEABILITY for OQ5 close |
| `a3ee2da` | test | test(ledger): credential-fidelity corpus + CI gate, enforce DC-LEDGER-10 (OQ5-S2) |
| `4187330` | feat | feat(types): discriminated StakeCredential end-to-end — preserve key/script tag (OQ5-S1) |
| `007b0e8` | docs | docs(ledger): OQ5-CREDENTIAL-FIDELITY cluster plan + cluster doc |
| `959e16c` | docs | docs(ledger): OQ-5 credential-fidelity invariants + DC-LEDGER-10 (declared) |
| `f81f815` | docs | docs(grounding): refresh CODEMAP/SEAMS/HEAD_DELTAS/TRACEABILITY for PHASE4-B5 close |
| `651adc9` | fix | fix(ledger): checked DRep-expiry arithmetic, deterministic fail-closed on overflow (PHASE4-B5-S5) |
| `06385d0` | test | test(ledger): gov-state accumulation corpus + CI gate, enforce DC-LEDGER-09 (PHASE4-B5-S4) |
| `d63c700` | feat | feat(ledger): apply gov-cert accumulation in block path, carry gov_state forward (PHASE4-B5-S3) |
| `7a48727` | feat | feat(ledger): native Conway gov-cert apply model — apply_conway_gov_cert (PHASE4-B5-S2) |
| `9c8d118` | feat | feat(ledger): gov-cert env infrastructure — drep_activity + GovCertEnv fail-fast (PHASE4-B5-S1) |
| `fdb6601` | docs | docs(gov): PHASE4-B5 invariants + cluster plan + DC-LEDGER-09 (Conway gov-cert accumulation) |
| `644eb03` | docs | docs(grounding): refresh CODEMAP/SEAMS/HEAD_DELTAS/TRACEABILITY for PHASE4-B4 close |
| `ee35493` | test | test(ledger): Conway cert-state accumulation corpus (PHASE4-B4-S5) |
| `302d22c` | feat | feat(ledger): era-dispatched fail-closed cert-state accumulation (PHASE4-B4-S3/S4) |
| `da30706` | feat | feat(ledger): native owner-tagged Conway cert apply model (PHASE4-B4-S2) |
| `228415b` | feat | feat(codec): owner-complete Conway certificate decoder (PHASE4-B4-S1) |
| `ae1300a` | docs | docs(planning): PHASE4-B4 grounding — invariants, cluster plan, cluster doc, B4-S1 slice (DC-LEDGER-08) |
| `1d989de` | docs | docs(grounding): refresh CODEMAP/TRACEABILITY/SEAMS/HEAD_DELTAS for PHASE4-B3F |
| `193d2fc` | feat | feat(codec): Conway cert decoder strictness — reject trailing bytes, bound preallocation (PHASE4-B3F) |
| `d6c1993` | feat | feat(ci): DC-TXV-06 cert-classification closure gate — flip partial to enforced (PHASE4-B3F) |
| `d766eb0` | chore | Close PHASE4-B3 — full Conway tx value-conservation accounting |
| `7784bf8` | test | test(tx-validity): PHASE4-B3 conservation corpora — real epoch-576 positive + adversarial no-false-accept |
| `978c222` | feat | feat(tx-validity): PHASE4-B3 full Conway value-conservation accounting — remove the cert/withdrawal early-out |
| `3aebbe5` | docs | docs(phase4-b3): invariants, cluster/slice plan, and registry rules for Conway value-conservation accounting |
| `c1cba82` | chore | chore(phase-4): close PHASE4-B2 — tx-validity agreement + mempool admission, grounding-doc refresh |
| `85a50dc` | feat | feat(tx-validity): B2-S5 mempool admission gate (Tier-1) — closes CE-B2-5 |
| `617139f` | feat | feat(tx-validity): B2-S4 adversarial tx corpus — closes CE-B2-4 (no false accept) + fixes a value-conservation fail-open |
| `4cffc2c` | feat | feat(tx-validity): B2-S3 positive tx corpus + replay — closes CE-B2-3 |
| `b24b22c` | feat | feat(tx-validity): B2-S2 tx_validity composition + verdict taxonomy — closes CE-B2-2 |
| `3e24d0b` | feat | feat(tx-validity): B2-S1 Conway vkey-witness + required-signer closure — closes CE-B2-1 |
| `7263699` | docs | docs(phase-4): PHASE4-B2 cluster doc — tx validity agreement |
| `b32fef3` | docs | docs(phase-4): PHASE4-B2 cluster/slice plan — 5-slice tx-validity-agreement arc |
| `b79f632` | docs | docs(phase-4): open PHASE4-B2 — tx validity agreement invariant sketch + DC-TXV family |
| `e0af99d` | chore | chore: gitignore multi-GB ledger-state dumps (belong in S3, not git) |
| `3552bc2` | chore | chore: sync Cargo.lock for PHASE4-B1 dependency edges |
| `993f363` | chore | Close PHASE4-B1 — full block validity agreement (validation core of workstream B) |
| `2630267` | feat | feat(validity): B1-S7 adversarial corpus — closes CE-B1-4 (no false accept) |
| `e394a82` | feat | feat(validity): B1-S6 positive agreement corpus + replay — closes CE-B1-3 |
| `7b95ccd` | feat | feat(validity): B1-S4 block_validity composition — closes CE-B1-2 + CE-B1-5 |
| `500589b` | feat | feat(validity): B1-S5 Praos single-VRF + KES header validation — 14/14 real Conway headers validate |
| `440ac72` | feat | feat(validity): B1-S3 BlockValidity verdict/error taxonomies + canonical surface encoding |
| `97a27cc` | feat | feat(validity): B1-S2 production LedgerView projection — closes CE-B1-1 |
| `a134379` | feat | feat(validity): B1-S1 consensus-input extractor + Conway-576 corpus |
| `b63f554` | docs | docs(phase-4): PHASE4-B1 cluster doc — full block validity agreement |
| `cb8165a` | docs | docs(phase-4): PHASE4-B1 cluster/slice plan — 7-slice full-block-validity arc |
| `c0acd59` | docs | docs(phase-4): open PHASE4-B1 — full block validity agreement invariant sketch + DC-VAL registry family |
| `e5f1f64` | feat | feat(interop): CE-N-B-6 follow-mode bridge + live preprod tip-agreement evidence |
| `807bcb6` | docs | docs(consensus): retarget N-B live-interop pin to cardano-node 11.0.1 |
| `a0c73e1` | chore | Close PHASE4-N-B — consensus runtime (Praos) authority + replay equivalence |
| `ad4d6f6` | feat | feat(consensus): S-B10 stream replay + orchestrator + live interop — closes CE-N-B-5 + CE-N-B-6 |
| `4f5cd7f` | feat | feat(consensus): S-B9 rollback authority — closes CE-N-B-2 |
| `8e991b5` | feat | feat(consensus): S-B8 fork choice + CandidateFragment — closes CE-N-B-1 |
| `e059652` | feat | feat(consensus): S-B7 Praos header validation |
| `f4c8369` | feat | feat(consensus): S-B6 leader schedule — closes CE-N-B-4 |
| `39cc143` | feat | feat(consensus): S-B5 op-cert counter monotonicity |
| `116eb57` | feat | feat(consensus): S-B4 nonce evolution authority |
| `70f60d9` | feat | feat(consensus): S-B3 VRF cert verification wiring + Praos VRF input + leader threshold |
| `ff01fe3` | feat | feat(consensus): S-B2 PraosChainDepState canonical type + closed event/error taxonomies |
| `fe68bb7` | feat | feat(consensus): S-B1 EraSchedule canonical authority + slot/era/time translation |
| `744ef34` | chore | chore(phase-4): complete PHASE4-N-A close — DoS hardening + grounding doc refreshes |
| `d9f0426` | docs | docs(phase-4): PHASE4-N-B invariant sketch v2 + 8 new DC-CONS-* registry rules |
| `69a2862` | chore | Close PHASE4-N-A — Ouroboros mini-protocols (11) wire-grammar conformance + state-machine determinism + real-interop validation |
| `56bfa7b` | feat | feat(phase-4): close CE-N-A-5 — 4 N2C real captures + LSQ/LTS/TxSubmission2 wire-form fixes + condition 4 + 5 + S-A10 evidence script |
| `d977640` | docs | docs(registry): wire S-A9 real-capture tests into PHASE4-N-A invariants |
| `b7cd39d` | feat | feat(phase-4): S-A9 N2C handshake + N2N keep-alive + peer-sharing real captures (3 more protocols + N2C 0x8000 wire-flag fix) |
| `a1b47ec` | feat | feat(phase-4): S-A9 block-fetch real interop + flat-range wire-form fix |
| `ef38212` | feat | feat(phase-4): S-A9 block-fetch codec wrapping fix + capture binary |
| `84d3eab` | feat | feat(phase-4): S-A9 chain-sync real capture + ChainSync codec wrapped-header fix |
| `98d0abe` | feat | feat(phase-4): S-A9 partial — real-capture corpus + handshake against mainnet relays |
| `1ba2d95` | feat | feat(phase-4): S-A8c — version table alignment with cardano-node 11.0.1 |
| `679491f` | docs | docs(phase-4): S-A8c entry obligation discharge — version table alignment with cardano-node 11.0.1 |
| `b7fade3` | feat | feat(phase-4): S-A8b — LocalTxMonitor wire-grammar rework (corrects S-A2/S-A8 misimpl) |
| `affa624` | docs | docs(phase-4): S-A8b entry obligation discharge — LocalTxMonitor wire-grammar rework |
| `9b7b96d` | docs | docs(phase-4): S-A9 + S-A10 entry obligation discharge — corpus replay harness + live interop closure gate |
| `77a02dd` | feat | feat(phase-4): S-A8 — N2C transition authority (4 state machines; structural completion) |
| `20b3554` | docs | docs(phase-4): S-A8 entry obligation discharge — N2C transition authority (4 state machines) |
| `b16329b` | feat | feat(phase-4): S-A7 — keep-alive + peer-sharing transition authority (structural completion) |
| `2cb0e86` | docs | docs(phase-4): S-A7 entry obligation discharge — keep-alive + peer-sharing transition authority |
| `844ae95` | feat | feat(phase-4): S-A6 — tx-submission2 transition authority (closes CE-N-A-4 state-machine portion) |
| `10659d5` | docs | docs(phase-4): S-A6 entry obligation discharge — tx-submission2 transition authority |
| `d702772` | feat | feat(phase-4): S-A5 — block-fetch transition authority (closes CE-N-A-3 state-machine portion) |
| `7078b9b` | docs | docs(phase-4): S-A5 entry obligation discharge — block-fetch transition authority |
| `787da55` | feat | feat(phase-4): S-A4 — chain-sync transition authority (closes CE-N-A-2 state-machine portion) |
| `7fef3a4` | docs | docs(phase-4): S-A4 entry obligation discharge — chain-sync transition authority |
| `ba02f71` | feat | feat(phase-4): S-A3 — handshake version negotiation authority (closes CE-N-A-1 state-machine portion) |
| `6faacd0` | docs | docs(phase-4): S-A3 entry obligation discharge — handshake version negotiation authority |
| `d1d47e9` | feat | feat(phase-4): S-A2 — protocol message codec authority for all 11 mini-protocols |
| `a4aabb9` | docs | docs(phase-4): S-A2 entry obligation discharge — protocol codec authority for all 11 mini-protocols |
| `4fde3a7` | feat | feat(phase-4): S-A1 — ade_network substrate + DC-CORE-01 mechanical gate |
| `22023be` | docs | docs(phase-4): S-A1 entry obligation discharge — mux/framing + sync-only CI gate |
| `6942674` | docs | docs(phase-4): open PHASE4-N-A cluster doc — wire+semantic Tier 1, 10 slices |
| `6ca2ba8` | docs | docs(phase-4): ratify PHASE4-N-A cluster plan (10 slices, authority-aligned) |
| `ae9c473` | docs | docs(phase-4): close N-A invariants §7 decisions + add DC-PROTO-06 |
| `492de56` | docs | docs(phase-4): open PHASE4-N-A — invariant sketch + DC-CORE-01 sync-only rule |
| `436b1d7` | chore | Close PHASE4-N-D — chain DB persistence with crash-equivalent recovery |
| `a3a083a` | docs | docs(phase-4): CE-N-D-1 closure evidence — 1000/1000 stress kill iterations green |
| `27960fd` | docs | docs(phase-4): lock N-A scope decisions before cluster opens |
| `a2c7ac8` | chore | chore(idd): refresh CODEMAP + TRACEABILITY + HEAD_DELTAS after N-D CI closure |
| `78da6c9` | chore | chore(ci): close Phase 4 N-D CI gap — 3 new scripts, 9 rules enforced |
| `f0b0fd6` | chore | chore(idd): refresh HEAD_DELTAS + SEAMS to align with BLUE-scope closure |
| `c8fa37f` | chore | chore(idd): refresh CODEMAP + TRACEABILITY after BLUE-list drift closure |
| `5b70bee` | chore | chore(ci): close BLUE-list drift — extend 6 CI scripts to full BLUE scope |
| `a87c3a3` | chore | chore(idd): generate four grounding docs (CODEMAP, SEAMS, HEAD_DELTAS, TRACEABILITY) |
| `3eddcbb` | chore | chore(idd): add .idd-config.json — opt the repo into IDD enforcement |
| `76c1f64` | chore | chore(idd): move in-flight cluster N-D into canonical clusters layout |
| `39865f6` | chore | chore(idd): update active-doc + CI refs to canonical registry path |
| `2047c42` | chore | chore(idd): commit-msg hook + CLAUDE.md trailer-override note |
| `5eecc8a` | feat | feat(phase-4): snapshot + forward-replay recovery (S-36) |
| `e52fe9f` | feat | feat(phase-4): SnapshotStore trait + impls (S-35) |
| `fb4a5d4` | feat | feat(phase-4): persistent ChainDb backed by redb (S-34) |
| `994203b` | feat | feat(phase-4): begin cluster N-D — ChainDb trait + InMemoryChainDb (S-33) |
| `9b15378` | feat | feat(phase-2c): reclassify CE-73 — semantic enforced, bytes Tier 4 non-goal |

Verbatim from `git log d509f02..HEAD` (`--no-merges`; history is
linear, no merge commits in range). Aggregation is in §3 and §5.

---

## 2. New Modules

| Module | Color | Purpose | Key sub-paths | Added in (cluster/slice) |
|--------|-------|---------|---------------|--------------------------|
| `ade_ledger::rollback` (new submodule of an existing BLUE crate) | BLUE | **In-memory snapshot + replay-forward rollback authority.** The rollback half of PHASE4-N-H's receive bridge — closes N-H's `DC-CONS-20.open_obligation = "rollback_side_blocked_until_ledger_snapshot_cluster"`. `traits.rs` declares the narrow read-only `SnapshotReader` (`nearest_le`) + `BlockSource` (`blocks_in_range`) traits the materialization driver depends on (production impls live in `ade_runtime::rollback`; in-memory test fakes pass through the same single composition — CN-STORE-07 single-authority discipline). `error.rs` declares closed sums `MaterializeError::{RollbackTooDeep, BlockNotInRange, BlockValidityFailed}` + `CommitRollbackError::{Materialize, ChainWriteFailed}` — no catchalls. `materialize.rs` declares the SOLE `pub fn` in the project returning the rolled-back `(LedgerState, PraosChainDepState)` tuple: `materialize_rolled_back_state(snapshot_reader, block_source, target, era_schedule, ledger_view)` — single SnapshotReader lookup + BlockSource read + per-block `block_validity` (the same authority N-H's admit branch uses) wrapped in `apply_block_with_verdicts` (inherits the unique epoch-boundary authority per `rules.rs:244-250`; no parallel epoch path). `commit.rs` declares `commit_rollback<W: ChainDbWrite>(state, target_slot, new_ledger, new_chain_dep, chain_write)` — irreversible-step-first staged commit shape: `ChainDbWrite::rollback_to_slot` is the only non-reversible step; on failure, state is unchanged; on success, ledger + chain_dep swap atomically and `state.pending_headers` resets. **Symmetric to N-H's `ade_ledger::receive::admit_via_block_validity`** — both compose `block_validity` as the sole authority. Enforced by `ci_check_rollback_materialize_closure.sh`. | `rollback/mod.rs`, `rollback/traits.rs` (~88 LOC), `rollback/error.rs` (~98 LOC), `rollback/materialize.rs` (~402 LOC), `rollback/commit.rs` (~200 LOC) | PHASE4-N-I / S1 (`0e7e9ee`); S2 driver (`0efdce3`); S3 commit + ChainDbWrite extension (`02b5e31`) |
| `ade_runtime::rollback` (new submodule of an existing RED crate) | GREEN | **Imperative-shell composition for the in-memory rollback infrastructure.** `cadence.rs` defines `SnapshotCadence` (single BLUE-structural field `every_n_blocks: u32`) + pure decision `should_snapshot_after_block(slot, block_no, params, last_snapshot) -> bool`; operator-tunable cadence is explicitly out of scope per DC-STORE-07 until represented as anchored, replay-derivable runtime data. `in_memory_cache.rs` defines `InMemorySnapshotCache` (BTreeMap-keyed, deterministic iteration, no HashMap) implementing the BLUE `SnapshotReader` trait via `nearest_le` returning the largest key ≤ target. `chaindb_block_source.rs` defines `ChainDbBlockSource<'a, D: ChainDb>` (borrow-wrapper around `&dyn ChainDb` implementing the BLUE `BlockSource` trait via `ChainDb::iter_from_slot`; pure projection, no I/O). `snapshot_writer.rs` defines `maybe_capture_snapshot(cache, cadence, slot, block_no, state)` — single point at which the scheduler / receive orchestrator captures a post-block-admit snapshot. All four files are GREEN: pure, no I/O, no async, no wall-clock, no rand, no HashMap. Enforced by `ci_check_snapshot_cadence_purity.sh`. | `rollback/mod.rs`, `rollback/cadence.rs` (~143 LOC), `rollback/in_memory_cache.rs` (~165 LOC), `rollback/chaindb_block_source.rs` (~113 LOC), `rollback/snapshot_writer.rs` (~146 LOC) | PHASE4-N-I / S4 (`3a9bab8`); S5 snapshot-writer hook (`e7add4d`) |
| `ade_ledger::receive` (new submodule of an existing BLUE crate) | BLUE | **Receive-side header→body bridge.** Pure total transition consuming peer-originated `ReceiveEvent` values (`RollForward`, `RollBackward`, `BlockDelivered`), composing the existing B1 `block_validity` authority through the new type-level admission gate `AdmittedBlock` (private constructor reachable only from `admit_via_block_validity`), and persisting through the narrow BLUE `ChainDbWrite` trait. Locally-originated chain-sync/block-fetch outputs (RequestNext, RequestRange, ClientDone, FindIntersect, Done) are NOT constructible — the CN-PROTO-07 closure. **PHASE4-N-I S6 wired the rollback half**: `receive_apply` gained an `Option<&RollbackContext>` parameter; when `Some`, the `RollBackward` arm composes `materialize_rolled_back_state` + `commit_rollback` atomically; when `None`, retains the legacy `Err(RollbackOutOfScope)` for callers not yet wired. The `ChainDbWrite` trait gained a second method `rollback_to_slot(slot) -> Result<(), ChainWriteError>` (PHASE4-N-I S3) for the rollback path. **Symmetric receive-side counterpart to N-C's `ade_ledger::producer` (producer authority) + N-G's `ade_ledger::producer::served_chain` (producer-side served chain).** Enforced by `ci_check_admitted_block_closure.sh` + `ci_check_receive_reducer_closure.sh`. | `receive/mod.rs`, `receive/admitted.rs` (~257 LOC), `receive/chain_write.rs` (~104 LOC, +rollback_to_slot), `receive/events.rs` (~220 LOC), `receive/pending_header_cache.rs` (~169 LOC), `receive/reducer.rs` (~743 LOC, +RollbackContext + roll_backward) | PHASE4-N-H / S1 (`b019ee3`); S2 reducer (`0ecf22f`); PHASE4-N-I / S3 trait extension (`02b5e31`); S6 RollbackContext wiring (`75f75da`) |
| `ade_runtime::receive` (new submodule of an existing RED crate) | GREEN+RED mix | **Imperative-shell composition for the N2N receive bridge.** GREEN: `events_to_state.rs` lifts N-A `ForkChoiceSignal` + `BatchDeliveryEvent` values into the BLUE `ReceiveEvent` stream (pass-through discipline — bytes are NEVER decoded here; the BLUE reducer's `BlockDelivered` branch is the canonical decode site); `in_memory_chain_write.rs` wraps a borrowed `&dyn ChainDb` and exposes `ChainDbWrite::write_admitted` **+ (PHASE4-N-I S3) `ChainDbWrite::rollback_to_slot`**. RED: `orchestrator.rs` is the per-peer N2N receive orchestrator — pure state-driver decoding inbound chain-sync (client role) + block-fetch (client role) wire frames via the existing PHASE4-N-A codecs, lifting via S3 GREEN, calling BLUE `receive_apply`. Multi-peer determinism: per-peer state is independent; the only cross-peer coordination point is the single shared `&dyn ChainDb`. **Key-boundary doctrine**: `orchestrator.rs` MUST NOT import from `crate::producer::signing` / `producer::broadcast` / `producer::scheduler`. Enforced by `ci_check_receive_replay_purity.sh` + `ci_check_receive_orchestrator_no_producer_dep.sh`. | `receive/mod.rs`, `receive/events_to_state.rs` (~209 LOC, GREEN), `receive/in_memory_chain_write.rs` (~209 LOC, GREEN, +rollback_to_slot), `receive/orchestrator.rs` (~433 LOC, RED) | PHASE4-N-H / S3 (`c584691`); S4 orchestrator (`1d06089`); PHASE4-N-I / S3 + S6 (in_memory_chain_write + orchestrator updates) |
| `ade_core_interop` bin `live_block_follow_session` (new RED binary in an existing RED crate) | RED | **Operator-action live-evidence probe for CE-N-H-6 / RO-LIVE-02.** Modeled on `live_block_fetch_session` (N-G S7) and `live_block_production_session` (N-C S7). Hermetic default mode prints a readiness banner and exits 0 (no sockets, no operator material read); `--connect` mode prints the wiring stub for the tokio socket bridge driving the receive orchestrator (S4) against a real peer. Args: `--network`, `--magic`, `--target`. Captures `docs/clusters/PHASE4-N-H/CE-N-H-LIVE_<date>.log` (operator-recorded). **Conditional on private Haskell peer availability**: at HEAD, `RO-LIVE-02.status = "partial"` with `open_obligation = "blocked_until_operator_peer_available"`. Build-and-start test asserts hermetic-mode banner; the byte-shape claim is closed by S5's mechanical `receive_pipeline_corpus_drive` test against the Conway-576 corpus. | `src/bin/live_block_follow_session.rs` (~136 LOC); `[[bin]]` entry in `crates/ade_core_interop/Cargo.toml`; operator procedure at `docs/clusters/completed/PHASE4-N-H/CE-N-H-6_PROCEDURE.md` | PHASE4-N-H / S6 (`efe1fb9`) |
| `ade_ledger::producer::served_chain` (new file in an existing BLUE crate) | BLUE | **Single canonical append-only chain index** from which N-G's server reducers source wire bytes. `ServedChainSnapshot` (BTreeMap-backed, deterministic) + `served_chain_admit(snapshot, AcceptedBlock) -> Result<(ServedChainSnapshot, ServedAdmitOutcome), ServedChainError>` — key `(slot, blake2b_256(header))` is derived from the bytes via `decode_block`; there is no caller-supplied "asserted hash" parameter. The broadcast gate (CN-CONS-07) is preserved across the network seam: the only path bytes enter the served chain is via an `AcceptedBlock` token, which only `self_accept` returning `Ok` produces. Accessors: `block_bytes(slot, &hash)` (point lookup; DC-CONS-17 foundation), `range_bytes(from, to)` (inclusive BTreeMap range; S4's RequestRange source), `iter()` (BTreeMap order), `iter_accepted` / `block_at` (S5 extensions — expose `&AcceptedBlock` so the GREEN adapter can call `accepted_block_header_bytes` directly), `fingerprint()` (blake2b_256 over `(slot_be8 \|\| hash \|\| bytes_len \|\| bytes)` triples in BTreeMap order — admission-order-independent replay anchor). Closed `ServedChainError::{Decode, KeyByteConflict}`. Enforced by `ci_check_served_chain_closure.sh`. | `producer/served_chain.rs` (~171 LOC) | PHASE4-N-G / S2 (`dc069cf`); S5 extension (`1a1b8e0`) |
| `ade_network::chain_sync::server` (new file in an existing BLUE-scoped network submodule) | BLUE | **Pure chain-sync server-pump reducers.** `producer_chain_sync_serve(state, in_msg, &served, version)` processes one client-originated message; `producer_chain_sync_advance_tip(state, &served)` is polled by the orchestrator after broadcast-queue admission. Composes the PHASE4-N-A `chain_sync_transition` for grammar validation (no parallel state machine). Header bytes in any RollForward come from the canonical `accepted_block_header_bytes` (DC-CONS-18) via the `ServedHeaderLookup` trait. Deterministic-resolution discipline (DC-PROTO-08): every server-agency state returns one of legal RollForward / RollBackward / AwaitReply / structured close-or-error — no ambiguous silent wait. Closed `ServerReply<ChainSyncMessage>` whose private inner enum carries only server-agency variants (RollForward / RollBackward / AwaitReply / IntersectFound / IntersectNotFound). Closed `ProducerServerError`. Trait-bound seam keeps `ade_network → ade_ledger` out of production deps. Enforced by `ci_check_chain_sync_server_closure.sh` + `ci_check_no_parallel_header_splitter.sh`. | `chain_sync/server.rs` (~804 LOC); registered in `chain_sync/mod.rs` | PHASE4-N-G / S1 (`8cd17c9`, ServerReply wrapper); S3 (`cc49b1d`, reducer) |
| `ade_network::block_fetch::server` (new file in an existing BLUE-scoped network submodule) | BLUE | **Pure block-fetch server-pump reducer.** `producer_block_fetch_serve(state, in_msg, &served, version)` — RequestRange{Block,Block} → look up `served.range_bytes`; if non-empty emit [StartBatch, Block{bytes}*, BatchDone]; if empty emit [NoBlocks]. RequestRange covering genesis Origin → [NoBlocks] (the producer does not serve genesis). ClientDone → `BlockFetchServerStep::Done`. Server-originated message from client agency → grammar reject. **Every Block{bytes} payload sources from `served.range_bytes()`** — which returns AcceptedBlock-derived slices verbatim via `ServedChainSnapshot`. DC-CONS-17 enforcement foundation: the reducer never re-encodes; bytes flow verbatim from AcceptedBlock through the served-chain index out to the wire. Closed `ServerReply<BlockFetchMessage>` whose private inner enum carries only server-agency variants (StartBatch / NoBlocks / Block / BatchDone). Trait-bound seam (`ServedRangeLookup`) mirrors S3. Enforced by `ci_check_block_fetch_server_closure.sh`. | `block_fetch/server.rs` (~596 LOC); registered in `block_fetch/mod.rs` | PHASE4-N-G / S1 (`8cd17c9`, ServerReply wrapper); S4 (`03d120f`, reducer) |
| `ade_runtime::producer::broadcast_to_served` (new file in an existing RED crate) | GREEN | **Pure adapter draining a BroadcastQueue and admitting each AcceptedBlock into a ServedChainSnapshot.** `drain_and_admit(BroadcastQueue, ServedChainSnapshot) -> (ServedChainSnapshot, BroadcastQueue, Vec<AcceptedBlock>)` is pure — no I/O, no clock, no rand, observably deterministic over arrival sequences. Bridges the RED scheduler / broadcast outputs into the BLUE server-pump input shape. Enforced by `ci_check_broadcast_to_served_purity.sh`. | `producer/broadcast_to_served.rs` (~188 LOC) | PHASE4-N-G / S5 (`1a1b8e0`) |
| `ade_runtime::producer::served_chain_lookups` (new file in an existing RED crate) | GREEN | **Borrow-wrapper around `ServedChainSnapshot` implementing both `ServedHeaderLookup` (chain-sync) and `ServedRangeLookup` (block-fetch).** The header projection goes through the canonical `accepted_block_header_bytes` (DC-CONS-16 / DC-CONS-18 — no parallel splitter). Pure projection — no I/O. Enforced by `ci_check_broadcast_to_served_purity.sh` (positive grep on the canonical import) + `ci_check_no_parallel_header_splitter.sh`. | `producer/served_chain_lookups.rs` (~120 LOC) | PHASE4-N-G / S5 (`1a1b8e0`) |
| `ade_runtime::network::n2n_server` (new file in a new RED submodule) | RED | **Pure per-peer N2N server-role session driver composing S5 GREEN + S3/S4 BLUE.** Decodes inbound mini-protocol frames, runs the reducers, encodes outgoing frames. No socket I/O — S7's evidence binary plugs this into tokio. Surface: `PerPeerN2nServerState::new(cs_v, bf_v)` (independent per-peer state holding both reducer states + handshake-negotiated versions), `dispatch_chain_sync_frame(state, frame, &snap)`, `dispatch_block_fetch_frame(state, frame, &snap)` (returns `Vec<frame>` since RequestRange yields a multi-frame batch), `poll_chain_sync_advance(state, &snap)` (drains a deferred RollForward after broadcast-queue admission). Multi-peer determinism (OQ-4): per-peer state is independent; cross-peer coordination only via `&ServedChainSnapshot` (proven by `tests/n2n_server_two_peer_determinism.rs`). Key-boundary doctrine: MUST NOT import from `crate::producer::signing`. Enforced by `ci_check_n2n_server_no_signing_dep.sh`. | `network/mod.rs`, `network/n2n_server.rs` (~216 LOC) | PHASE4-N-G / S6 (`f773b1c`) |
| `ade_core_interop` bin `live_block_fetch_session` (new RED binary in an existing RED crate) | RED | **Operator-action live-evidence probe for CE-N-G-8 / RO-LIVE-01.** Mirrors `live_block_production_session` and `live_tx_submission_session`. Hermetic default mode prints a readiness banner and exits 0 (no sockets, no operator material read); `--connect` mode prints the wiring stub (the tokio socket bridge to `n2n_server` is operator-action work; the `n2n_server` module itself is the pure driver). Args: `--network`, `--magic`, `--target`, `--out`. Captures `docs/clusters/PHASE4-N-G/CE-N-G-LIVE_<date>.log` (operator-recorded). **Conditional on private Haskell peer availability**: at HEAD, `RO-LIVE-01.status = "partial"` with `open_obligation = "blocked_until_operator_peer_available"`. Build-and-start test asserts hermetic-mode banner; the byte-shape claim is closed by S7's mechanical `cross_impl_server_pipeline` test against the Conway-576 corpus. | `src/bin/live_block_fetch_session.rs` (~141 LOC); `[[bin]]` entry in `crates/ade_core_interop/Cargo.toml`; operator procedure at `docs/clusters/completed/PHASE4-N-G/CE-N-G-8_PROCEDURE.md` | PHASE4-N-G / S7 (`a280954`) |
| `ade_runtime::producer::signing` (new file in an existing RED crate) | RED | **Producer crypto-substrate — RED-confined private-key custody and signing.** `VrfSigningKey`, `KesSecret`, `ColdSigningKey` hold in-memory secret material with zeroize-on-drop. `vrf_prove`, `kes_sign`, `kes_update`, closed `SigningError`. No reads of wall-clock, env, fs. Private-key types do NOT appear in any public BLUE API surface. Enforced by `ci_check_private_key_custody.sh` and `DC-CRYPTO-03/04/05` + `OP-OPS-04`. | `producer/signing.rs` (~600 LOC) | PHASE4-N-C / S1 (`ea9770e`) |
| `ade_runtime::producer::keys` (new file in an existing RED crate) | RED | **cardano-cli `*.skey` text-envelope loader.** `load_{vrf,kes,cold}_signing_key_skey` + `VRF/KES/POOL_SIGNING_KEY_TYPE` constants + closed `KeyLoadError`. **Open obligation** (`OP-OPS-04.open_obligation`): real cardano-cli's 612-byte expanded-tree Sum6KES skey loading is the upstream-fork-or-document call. | `producer/keys.rs` (~376 LOC) | PHASE4-N-C / S1 (`ea9770e`) |
| `ade_core::consensus::opcert_validate` (new file in an existing BLUE crate) | BLUE | **BLUE op-cert validator** (counter monotonicity + period gate + cold-sig verify). Closed `OpCertError::{CounterRepeat, CounterRegression, PeriodMismatch, ShortHotVkey, BadColdSignature}`. Enforced by `ci_check_opcert_closed.sh` and `DC-CONS-11`/`DC-CONS-12`. | `consensus/opcert_validate.rs` (~234 LOC) | PHASE4-N-C / S2 (`4cf4b65`) |
| `ade_codec::shelley::opcert` (new file in an existing BLUE crate) | BLUE | **Closed-grammar op-cert encoder/decoder.** Cardano-cli byte-identical. Closed `OpCertCodecError`. Enforced by `ci_check_opcert_closed.sh`. | `shelley/opcert.rs` (~375 LOC); registered in `shelley/mod.rs` | PHASE4-N-C / S2 (`4cf4b65`) |
| `ade_ledger::producer` (new submodule of an existing BLUE crate) | BLUE | **Producer authority core — the validation→producer leap.** `state.rs` (`ProducerTick`), `forge.rs` (`forge_block`, `ForgedBlock`, `ForgeError`, `ForgeEffects`), `self_accept.rs` (`self_accept`, `AcceptedBlock` type-level broadcast gate, `SelfAcceptError`). **PHASE4-N-G S2 added `served_chain.rs` as a sibling file under this submodule** (the served-chain index that consumes AcceptedBlock tokens). Enforced by `ci_check_forge_purity.sh`, `ci_check_self_accept_gate.sh`, `ci_check_no_private_keys_in_corpus.sh`, and (S2) `ci_check_served_chain_closure.sh`. | `producer/mod.rs`, `producer/state.rs` (~74 LOC), `producer/forge.rs` (~534 LOC), `producer/self_accept.rs` (~601 LOC at HEAD), `producer/served_chain.rs` (~171 LOC, **N-G S2**) | PHASE4-N-C / S3 + S4 + S5 (`8312690`, `4fd714c`, `aa7a7dd`); PHASE4-N-G / S2 (`dc069cf`) |
| `ade_ledger::block_body_hash` (new file in an existing BLUE crate) | BLUE | **Single canonical body-hash authority** consumed by both `forge_block` (producer) and `block_validity::header_input::computed_body_hash` (validator). Enforced by `ci_check_no_producer_body_encoder.sh` and `DC-CONS-16`. | `block_body_hash.rs` (~147 LOC) | PHASE4-N-C / S4 (`4fd714c`) |
| `ade_runtime::producer::tick_assembler` (new file in an existing RED crate) | GREEN | **Composes canonical `ProducerTick` from captured RED outputs.** Pure — no I/O, no clock, no rand, no async. | `producer/tick_assembler.rs` (~211 LOC) | PHASE4-N-C / S6 (`58678af`) |
| `ade_runtime::producer::scheduler` (new file in an existing RED crate) | RED | **Slot-wakeup RED loop driving the producer pipeline.** Self-accept failure → deterministic halt. Enforced by `ci_check_scheduler_closure.sh`. | `producer/scheduler.rs` (~478 LOC) | PHASE4-N-C / S6 (`58678af`) |
| `ade_runtime::producer::broadcast` (new file in an existing RED crate) | RED | **Outbound queue handing self-accepted bytes to `ade_network`'s N2N server path.** Argument type `AcceptedBlock` cannot be constructed outside `self_accept` (type-level broadcast gate, CN-CONS-07). | `producer/broadcast.rs` (~265 LOC) | PHASE4-N-C / S6 (`58678af`) |
| `ade_testkit::producer` (new submodule of an existing crate) | GREEN | **In-code synthetic producer corpus + replay + cross-impl adapter.** `fixtures.rs`, `replay.rs`, `reference_vectors.rs`, `cross_impl_adapter.rs` (S7). All synthetic — canonical by construction. | `producer/mod.rs`, `producer/{fixtures,replay,reference_vectors,cross_impl_adapter}.rs` | PHASE4-N-C / S1 + S3 + S4 + S7 |
| `ade_core_interop` bin `live_block_production_session` (new RED binary in an existing RED crate) | RED | **Sustained-window operator-action live-evidence probe for CE-N-C-8.** Conditional on testnet SPO stake; status tracked as `CN-CONS-06.open_obligation`. | `src/bin/live_block_production_session.rs` (~247 LOC); operator procedure at `docs/clusters/PHASE4-N-C/CE-N-C-8_PROCEDURE.md` | PHASE4-N-C / S7 (`694dd74`) |
| `ade_codec::conway::governance` (new file in an existing BLUE crate) | BLUE | **Closed-grammar Conway proposal-procedures decoder + encoder.** Enforced by `ci_check_proposal_procedures_closed.sh` and `DC-LEDGER-11`. | `conway/governance.rs` (~856 lines) | PROPOSAL-PROCEDURES-DECODE / PP-S1 (`70bc85b`) |
| `ade_testkit::governance::proposal_procedures_replay` (new submodule + file in an existing crate) | GREEN | **Canonical synthetic replay harness for the closed `proposal_procedures` decoder.** | `governance/mod.rs`, `governance/proposal_procedures_replay.rs` (~232 lines) | PROPOSAL-PROCEDURES-DECODE / PP-S2 (`928c2be`) |
| `ade_core_interop` bin `live_tx_submission_session` (new RED binary in an existing RED crate) | RED | Sustained-window N2N tx-submission2 live-evidence probe. CE-N-E-6 closure-gate. | `src/bin/live_tx_submission_session.rs` (~552 LOC) | PHASE4-N-E / S6 (`d1068b3` + `caa5ce8`) |
| `ade_ledger::mempool::ingress` (new file in an existing BLUE crate) | BLUE | Single closed wire-level ingress chokepoint. | `mempool/ingress.rs` | PHASE4-N-E / S1 (`32c1ee6`) |
| `ade_ledger::mempool::canonicalize` (new file in an existing BLUE crate) | GREEN | Deterministic per-peer ingress canonicalizer. | `mempool/canonicalize.rs` | PHASE4-N-E / S3 (`509d714`) |
| `ade_testkit::mempool::ingress_replay` (new submodule of an existing crate) | GREEN | Single-step ingress-replay harness over B-track corpus. | `mempool/mod.rs`, `mempool/ingress_replay.rs` | PHASE4-N-E / S2 (`2d0c918`) |
| `ade_core_interop::tx_submission` (new file in an existing RED crate) | GREEN | N2N tx-submission2 → `mempool_ingress` bridge. | `src/tx_submission.rs` | PHASE4-N-E / S4 (`ca3f23a`) |
| `ade_core_interop::local_tx_submission` (new file in an existing RED crate) | GREEN | N2C local-tx-submission → `mempool_ingress` bridge. | `src/local_tx_submission.rs` | PHASE4-N-E / S5 (`43fcc31`) |
| `ade_codec::conway::cert` (new file in an existing BLUE crate) | BLUE | Conway-complete certificate decoder with a closed wire grammar. | `conway/cert.rs` | PHASE4-B3 / B3-S1, B3-S2; B3F-S2 |
| `ade_codec::conway::withdrawals` (new file in an existing BLUE crate) | BLUE | Conway withdrawals-map decoder. | `conway/withdrawals.rs` | PHASE4-B3 / B3-S3 |
| `ade_ledger::cert_classify` (new file in an existing BLUE crate) | BLUE | Closed cert-deposit classification. | `cert_classify.rs` | PHASE4-B3 / B3-S2; closure gate B3F / B3F-S1 |
| `ade_ledger::gov_cert` (new file in an existing BLUE crate) | BLUE | Native Conway governance-certificate accumulation. | `gov_cert.rs` | PHASE4-B5 |
| `ade_ledger::tx_validity` (new submodule of an existing BLUE crate) | BLUE | Per-transaction verdict authority. | `mod.rs`, `verdict.rs`, `required_signers.rs`, `witness.rs`, `phase1.rs`, `transition.rs`, `encoding.rs` | PHASE4-B2 |
| `ade_ledger::mempool` (new submodule of an existing BLUE crate) | BLUE/GREEN mix | Two-layer mempool. | `mod.rs`, `admit.rs`, `policy.rs`, `ingress.rs` (N-E S1), `canonicalize.rs` (N-E S3) | PHASE4-B2; PHASE4-N-E |
| `ade_testkit::tx_validity` (new submodule of an existing crate) | GREEN | Test-only tx-validity harness. | `tx_validity/mod.rs`, etc. | PHASE4-B2 |
| `ade_ledger::block_validity` (new submodule of an existing BLUE crate) | BLUE | Full-block verdict authority. | `mod.rs`, `verdict.rs`, `transition.rs`, `header_input.rs`, `encoding.rs` | PHASE4-B1 |
| `ade_ledger::consensus_view` (new file in an existing BLUE crate) | BLUE | Production `LedgerView` projection. | `consensus_view.rs` | PHASE4-B1 |
| `ade_ledger::consensus_input_extract` (new file in an existing BLUE crate) | RED | Tail-scan of snapshot `state` CBOR. | `consensus_input_extract.rs` | PHASE4-B1 |
| `ade_core::consensus::kes_check` (new file in an existing BLUE crate) | BLUE | Fail-closed wiring of `ade_crypto::kes` into Praos header validation. | `kes_check.rs` | PHASE4-B1 / B1-S5 |
| `ade_testkit::validity` (new submodule of an existing crate) | GREEN | Test-only block-validity harness. | `validity/mod.rs`, etc. | PHASE4-B1 |
| `ade_core_interop::follow` (new file in an existing RED crate) | RED | Follow-mode bridge. | `follow.rs` | CE-N-B-6 (`e5f1f64`) |
| `ade_network` (new workspace crate) | BLUE-majority (per-submodule scoped) | Ouroboros mini-protocol authority. **N-G S1 / S3 / S4 added the new server-side files** `chain_sync/server.rs` + `block_fetch/server.rs` + their ServerReply wrappers; the rest is unchanged. **N-H + N-I added no new file in this crate** — the receive orchestrator dispatches the existing PHASE4-N-A codecs. | `codec/`, `handshake/`, `chain_sync/`, `block_fetch/`, `tx_submission/`, `keep_alive/`, `peer_sharing/`, `n2c/`, `mux/frame.rs` (BLUE), `mux/transport.rs` (RED), `session/` (RED), `chain_sync/server.rs` (**N-G S1/S3**), `block_fetch/server.rs` (**N-G S1/S4**) | PHASE4-N-A; PHASE4-N-G |
| `ade_core::consensus` (new submodule of an existing BLUE crate) | BLUE | Praos consensus authority. | `mod.rs`, `era_schedule.rs`, `header_validate.rs`, `vrf_cert.rs`, `nonce.rs`, `op_cert.rs`, `leader_schedule.rs`, `fork_choice.rs`, `rollback.rs`, `kes_check.rs` (B1), `praos_state.rs`, `candidate.rs`, `events.rs`, `errors.rs`, `encoding.rs`, `ledger_view.rs`, `header_summary.rs`, `opcert_validate.rs` (N-C S2) | PHASE4-N-B; PHASE4-N-C / S2 |
| `ade_runtime::consensus` (new submodule of an existing RED crate) | GREEN/RED mix | Imperative-shell composition for consensus runtime. | `mod.rs`, `candidate_fragment.rs`, `chain_selector.rs`, `genesis_parser.rs` | PHASE4-N-B / S-B8, S-B10 |
| `ade_runtime::producer` (new submodule of an existing RED crate) | RED + GREEN mix | Imperative-shell composition for producer runtime. **N-G S5 added `broadcast_to_served.rs` + `served_chain_lookups.rs` as new GREEN files.** | `mod.rs`, `signing.rs`, `keys.rs`, `scheduler.rs`, `broadcast.rs`, `tick_assembler.rs`, `broadcast_to_served.rs` (**N-G S5**), `served_chain_lookups.rs` (**N-G S5**) | PHASE4-N-C; PHASE4-N-G / S5 |
| `ade_runtime::network` (new submodule of an existing RED crate) | RED | **Imperative-shell composition for the N2N server-role session driver** (N-G S6). | `network/mod.rs`, `network/n2n_server.rs` | PHASE4-N-G / S6 (`f773b1c`) |
| `ade_core_interop` (new workspace crate) | RED | Live cardano-node interop driver. **N-G S7 added `live_block_fetch_session.rs`; N-H S6 added `live_block_follow_session.rs`.** | `src/lib.rs`, `src/follow.rs`, `src/tx_submission.rs` (N-E S4), `src/local_tx_submission.rs` (N-E S5), `src/bin/live_consensus_session.rs`, `src/bin/live_tx_submission_session.rs` (N-E S6), `src/bin/live_block_production_session.rs` (N-C S7), `src/bin/live_block_fetch_session.rs` (N-G S7), `src/bin/live_block_follow_session.rs` (**N-H S6**), `tests/` | PHASE4-N-B; PHASE4-N-E; PHASE4-N-C; PHASE4-N-G; PHASE4-N-H |
| `ade_testkit::consensus` (new submodule of an existing crate) | GREEN | Test-only harness for consensus replay corpora. | `consensus/mod.rs`, `consensus/corpus.rs`, `consensus/ledger_view_stub.rs`, `consensus/stream_replay.rs` | PHASE4-N-B |
| `ade_runtime::chaindb` | RED | Block-store abstraction and impls. | `mod.rs`, `types.rs`, `error.rs`, `in_memory.rs`, `persistent.rs`, `contract.rs`, `snapshot_contract.rs`, `crash_safety.rs` | PHASE4-N-D |
| `ade_runtime::recovery` | RED | Composes ChainDb + SnapshotStore. | `recovery.rs` | PHASE4-N-D / S-36 |
| `ade_runtime` bin `chaindb_kill_target` | RED | Kill-target child process for the 1,000-kill-9 stress harness. | `src/bin/chaindb_kill_target.rs`, `tests/stress_kill_harness.rs` | PHASE4-N-D / S-37 |

Workspace-level membership grew by **two crates** across the full
delta: `ade_network` (PHASE4-N-A) and `ade_core_interop` (PHASE4-N-B).
Both are RED-or-mixed. **PHASE4-N-I added no new crate** — S1/S2/S3's
BLUE `ade_ledger::rollback` submodule and S4/S5's GREEN
`ade_runtime::rollback` submodule both live as new files / submodules
under the existing 8 workspace crates. **PHASE4-N-H added no new
crate** either. **PHASE4-N-G added no new crate** either.

Crate dependency shape at HEAD: **PHASE4-N-I added no new cross-crate
dep edges.** The PHASE4-N-C S6 production edge `ade_runtime →
ade_ledger` carries the new BLUE `rollback::*` types (`SnapshotReader`,
`BlockSource`, `MaterializeError`, `CommitRollbackError`,
`materialize_rolled_back_state`, `commit_rollback`); the N-D `ChainDb`
trait is reused by `ChainDbBlockSource` through the existing
`ade_runtime → ade_runtime::chaindb` internal path. **PHASE4-N-H added
no new cross-crate dep edge** either. **PHASE4-N-G S6** added one new
non-dev dep edge (`ade_runtime → ade_network`) and four new dev-dep
edges on `ade_network`. Carried forward: the **PHASE4-N-C S6** edge
`ade_runtime → ade_ledger` and the **PHASE4-N-E S4** edge
`ade_core_interop → ade_ledger`. No edge from a BLUE crate to a RED
crate was introduced. Dependency direction RED → BLUE is permitted by
`ci_check_dependency_boundary.sh`.

Corpora at HEAD: N-A capture corpus, N-B replay corpus, B1 validity
corpus, B3 conservation corpora, B4/B5 README-only synthetic notes,
the credential-fidelity corpus from OQ5-S2, the PPD in-code
synthetic-canonical corpus, the N-C in-code synthetic producer
corpus, and `corpus/snapshots/` under `.gitignore` (canonical home
`s3://ade-corpus-snapshots`). **PHASE4-N-I added no external corpus**
— the receive-rollback integration test
(`tests/receive_rollback_integration.rs`) drives against synthetic
admit-then-rollback sequences built in-test; the corpus-grounded
DC-CONS-22 evidence comes from the existing N-H
`receive_pipeline_corpus_drive` test against the Conway-576 corpus.
PHASE4-N-H likewise added no new on-disk corpus. PHASE4-N-G likewise
added no new on-disk corpus.

Cross-reference: **The `ade-CODEMAP.md` regenerated in parallel with
this HEAD_DELTAS will record the new BLUE submodule
`ade_ledger::rollback` (with `traits`, `error`, `materialize`,
`commit`) and the new GREEN submodule `ade_runtime::rollback` (with
`cadence`, `in_memory_cache`, `chaindb_block_source`,
`snapshot_writer`)** as rows under their respective crates' BLUE/GREEN
listings; the prior CODEMAP at `f143984` does NOT yet contain either.
SEAMS will pick up `materialize_rolled_back_state` as the single
canonical rollback-materialization seam (CN-STORE-07, symmetric to
N-H's `admit_via_block_validity` admission gate), `commit_rollback`
as the rollback atomic-commit seam, the narrow `SnapshotReader` +
`BlockSource` traits as the rollback-driver dependency seams, and the
extended `ChainDbWrite::rollback_to_slot` method as the rollback-side
persistence seam. TRACEABILITY will pick up the 4 new registry rules
(`DC-CONS-21`, `DC-CONS-22`, `CN-STORE-07`, `DC-STORE-07`) + the
`DC-CONS-20` status flip + the `DC-CONS-20.strengthened_in` update
with their 2 new `ci_script ↔ rule` edges; the prior TRACEABILITY at
`f143984` does NOT contain any of them. All three rewrites are in
flight in the grounding ripple immediately following this HEAD_DELTAS
regen; the four docs will be self-consistent at the next grounding-doc
commit.

---

## 3. Modules Modified

| Module | Scope | Key changes |
|--------|-------|-------------|
| `ade_ledger` | +81 source/test files over the full delta; **PHASE4-N-I touched 7 files**: `rollback/mod.rs` (new, ~31 LOC: register submodule), `rollback/traits.rs` (new, ~88 LOC, S1), `rollback/error.rs` (new, ~98 LOC, S1), `rollback/materialize.rs` (new, ~402 LOC, S2), `rollback/commit.rs` (new, ~200 LOC, S3), `receive/chain_write.rs` (+8 net LOC: trait extension `rollback_to_slot`, S3), `receive/reducer.rs` (+115 net LOC: `RollbackContext` + `roll_backward` private fn, S6), `lib.rs` (+1: register `rollback`). **PHASE4-N-H touched 6 files** (carried forward). | **PHASE4-N-I (S1 + S2 + S3 + S6):** new BLUE submodule `ade_ledger::rollback` — the in-memory snapshot + replay-forward rollback authority. Narrow read-only `SnapshotReader` + `BlockSource` traits (production impls in `ade_runtime::rollback`; in-memory test fakes pass through the same single composition — CN-STORE-07); closed sums `MaterializeError` (`RollbackTooDeep`, `BlockNotInRange`, `BlockValidityFailed`) + `CommitRollbackError` (`Materialize`, `ChainWriteFailed`); SOLE `pub fn` returning rolled-back `(LedgerState, PraosChainDepState)` is `materialize_rolled_back_state` — composes one `SnapshotReader::nearest_le` + one `BlockSource::blocks_in_range` + per-block `block_validity` (same authority N-H's admit branch uses) wrapped in `apply_block_with_verdicts` (inherits unique epoch-boundary authority per `rules.rs:244-250`); `commit_rollback` irreversible-step-first staged commit (calls `ChainDbWrite::rollback_to_slot` first; on failure state unchanged; on success ledger + chain_dep swap atomically + `pending_headers` resets). N-H `ChainDbWrite` trait extended with `rollback_to_slot(slot) -> Result<(), ChainWriteError>` (S3). S6 wires `receive_apply` with new `Option<&RollbackContext>` parameter — `RollBackward` arm composes `materialize_rolled_back_state` + `commit_rollback` when `Some`, retains legacy `Err(RollbackOutOfScope)` when `None`. **`DC-CONS-20` flipped to `enforced`** with `cluster = "PHASE4-N-H + PHASE4-N-I"` + `strengthened_in = ["PHASE4-N-I"]` + `open_obligation` removed (rollback-side now mechanically enforced lockstep with admit-side). **PHASE4-N-H (S1 + S2, carried forward):** new BLUE submodule `ade_ledger::receive`. **Carried forward:** PHASE4-N-G producer::served_chain; PHASE4-N-C producer submodule + `block_body_hash`; PHASE4-N-E mempool ingress chokepoint; B-series; OQ5/FIDELITY/WRITEBACK; PROPOSAL-PROCEDURES-DECODE. |
| `ade_runtime` | +28 files, +6,400+ lines from prior threads; **PHASE4-N-I added 5 source files + 1 integration test / +1,038 LOC**: `rollback/mod.rs` (~26 LOC, S4: register `cadence`, `chaindb_block_source`, `in_memory_cache`, `snapshot_writer`), `rollback/cadence.rs` (~143 LOC, S4 GREEN), `rollback/in_memory_cache.rs` (~165 LOC, S4 GREEN), `rollback/chaindb_block_source.rs` (~113 LOC, S4 GREEN), `rollback/snapshot_writer.rs` (~146 LOC, S5 GREEN), `lib.rs` (+1: register `rollback`); `receive/in_memory_chain_write.rs` (+10 LOC: impl new trait method `rollback_to_slot`, S3), `receive/orchestrator.rs` (+1 LOC: pass `None` for rollback_ctx in legacy call site, S6); + test `receive_rollback_integration.rs` (~445 LOC, S6 — the canonical DC-CONS-22 evidence). **PHASE4-N-H added 4 source files + 3 integration tests / +1,041 LOC** (carried forward). | **PHASE4-N-I (S4 + S5 + S3 + S6):** new GREEN submodule `ade_runtime::rollback` — pure imperative-shell composition for the in-memory rollback infrastructure. `cadence` defines `SnapshotCadence` (single BLUE-structural field `every_n_blocks: u32`) + pure decision `should_snapshot_after_block` (operator-tunable cadence explicitly out of scope per DC-STORE-07). `in_memory_cache` defines `InMemorySnapshotCache` (BTreeMap-keyed, deterministic, no HashMap) implementing the BLUE `SnapshotReader` trait. `chaindb_block_source` defines `ChainDbBlockSource` (borrow-wrapper around `&dyn ChainDb` implementing BLUE `BlockSource` via `ChainDb::iter_from_slot`; pure projection, no I/O). `snapshot_writer::maybe_capture_snapshot` is the single point at which the scheduler / receive orchestrator captures a snapshot of post-block-admit state. `receive::in_memory_chain_write` extended to impl the new `ChainDbWrite::rollback_to_slot` method (S3); `receive::orchestrator` updated to pass `None` for the new `Option<&RollbackContext>` parameter at the legacy call site (S6). The new integration test `receive_rollback_integration` covers all rollback paths end-to-end: in-memory snapshot rollback, no-snapshot `RollbackTooDeep`, state-unchanged on materialize failure, and the canonical DC-CONS-22 evidence `rollback_then_continue_admit_equals_straight_line_admit` (admit → snapshot → rollback → continue admit yields a ledger fingerprint byte-identical to a straight-line admit). **No new dep edge** — reuses the N-C `ade_runtime → ade_ledger` production edge for the BLUE `rollback::*` types; the N-D ChainDb trait is reused by `ChainDbBlockSource` through the internal `ade_runtime::chaindb` path. **Carried forward:** N-H `receive` submodule; N-G `producer::broadcast_to_served` + `producer::served_chain_lookups` + `network::n2n_server`; N-C producer submodule; N-B consensus runtime; N-D chaindb/recovery. |
| `ade_core_interop` | +1,990 across 11 files from prior threads; **no PHASE4-N-I change** — N-I's evidence is mechanical (the in-tree integration test); no new binary or live-evidence harness. | **Unchanged in PHASE4-N-I.** **Carried forward:** N-H S6 live-follow binary; N-G S7 live-fetch binary; N-C S7 producer-side live binary; N-E S4/S5/S6 tx-submission bridges; CE-N-B-6 follow-bridge. |
| `ade_network` | 100 files, +17,861 lines (full N-A); **PHASE4-N-G added 4 files / +1,402 LOC** (carried forward); **no PHASE4-N-I change**; **no PHASE4-N-H change** — the receive orchestrator dispatches the existing PHASE4-N-A codecs unchanged. | **Unchanged in PHASE4-N-I + PHASE4-N-H.** **Carried forward:** N-G server reducers + ServerReply wrappers; N-A wire-grammar work + DoS hardening. |
| `ade_ledger` (block_validity sub-area) | (counted above) | **No PHASE4-N-I change** — the N-I `materialize_rolled_back_state` driver composes `block_validity` unchanged via `apply_block_with_verdicts`. **Carried forward:** N-H admit branch composing `block_validity` via `admit_via_block_validity`; N-G S1 `accepted_block_header_bytes` lift. |
| `ade_core` | +30 source files + tests (N-B); +828 / −86 across 16 files (B1); +1 new file (N-C S2). **No PHASE4-N-I source change** — the N-I driver depends only on existing `ade_core::consensus::{era_schedule, ledger_view, praos_state}` types. **No PHASE4-N-H source change** either. | **Unchanged in PHASE4-N-I + PHASE4-N-H.** **Carried forward:** N-B consensus authority + N-C S2 opcert validator. |
| `ade_codec` | +14 source/test files over the full delta; **no PHASE4-N-I change**. | **Unchanged in PHASE4-N-I + PHASE4-N-H.** **Carried forward:** PPD PP-S1 `conway::governance`; N-C S2 `shelley::opcert`; N-C S3 `shelley::tx_components` producer assembly path; B3 / B4 / OQ5 era decoder work. |
| `ade_crypto` | 2 files: `kes.rs` (+122 / −81), `lib.rs` (+5). **No PHASE4-N-I change.** | **Unchanged in PHASE4-N-I + PHASE4-N-H.** **Carried forward:** N-C S1 `KesSignature` + `verify_kes_signature`. |
| `ade_testkit` | +33 files across the full delta; **no PHASE4-N-I change** — N-I's tests live as an integration test under `ade_runtime/tests/`, not under the testkit crate (same pattern as N-G + N-H). | **Unchanged in PHASE4-N-I + PHASE4-N-H.** **Carried forward:** N-C `producer/` harness; PPD PP-S2 `governance/proposal_procedures_replay`; N-E `mempool/ingress_replay`. |

No other crate had non-trivial source changes since baseline.
`ade_plutus` and `ade_node` were untouched by code commits.
**PHASE4-N-I touched 2 of 8 workspace crates** (`ade_ledger`,
`ade_runtime`). **No `.idd-config.json` change.** **No BLUE
authority-path semantics changed apart from the new rollback
surfaces** — the prior validator authorities
(`ade_core::consensus::*`, `ade_ledger::block_validity::*`,
`ade_ledger::block_body_hash`), producer authorities
(`ade_ledger::producer::*`), and N-H receive authorities
(`ade_ledger::receive::{admitted, reducer admit-side}`) were re-used
unchanged by the rollback driver. The materialize driver composes
`block_validity` through `apply_block_with_verdicts` without changing
what `block_validity` computes. The S6 wiring of the rollback context
into `receive_apply` adds a new code path (the `Some(ctx)` arm of
`RollBackward`) but does not change the admit-side `RollForward` /
`BlockDelivered` arms.

---

## 4. Feature Flags

No Cargo `[features]` tables exist at HEAD in any workspace crate, and
none existed at baseline. The project does not use Cargo feature flags
as a semantic surface — closed semantic surfaces are encoded in the
type system per the IDD core principles, and conditional compilation
is checked out of BLUE code via `ci/ci_check_no_semantic_cfg.sh`
(scoped over the full 6-crate BLUE set, covering all surfaces
introduced through the PHASE4-N-I rollback submodule, the PHASE4-N-H
receive submodule, the PHASE4-N-G server reducers, the served-chain
index, and the header-projection seam).

No `#[cfg(feature = ...)]` gates appear at either ref.
**PHASE4-N-I introduced no new Ade-side feature flag and no new
upstream-crate feature selection.**

**Status: unchanged — zero Ade feature flags at baseline, zero at HEAD.**

---

## 5. CI Checks

The CI surface is the shell-script set under `ci/` (no
`.github/workflows` in this repo). At baseline there were 15 scripts.
At HEAD there are **54 scripts plus one git hook**
(`ci/git-hooks/commit-msg`). Across the full delta: CE-73 added one,
N-D added three, N-A added two, N-B added four, B3 added one, B3F
added one, B5 added one, OQ5 added one, PHASE4-N-E S1/S2 added two,
PROPOSAL-PROCEDURES-DECODE PP-S1 added one, PHASE4-N-C added eight
(the 33rd → 40th), PHASE4-N-G added seven (the 41st → 47th),
PHASE4-N-H added five (the 48th → 52nd), and **PHASE4-N-I added two
(the 53rd → 54th)**:
`ci_check_rollback_materialize_closure.sh` (S2),
`ci_check_snapshot_cadence_purity.sh` (S4).
Grouped by cluster.

### CE-73 reclassification (Phase 2C close-out)

| Check | Status | What it checks |
|-------|--------|----------------|
| `ci/ci_check_hfc_translation.sh` | **New** (`9b15378`) | CE-73-semantic gate: runs the three HFC ledger-side translation proof surfaces. Authoritative test for invariant `DC-EPOCH-02`. |

### IDD canonicalization (post-Phase-4-N-D)

| Check | Status | What it checks |
|-------|--------|----------------|
| `ci/ci_check_constitution_coverage.sh` | Modified (`39865f6`, `aa7a7dd`) | Path-only edit (`39865f6`): registry path now `docs/ade-invariant-registry.toml`. **Extended** (`aa7a7dd`, N-C S5) to recognize new closed types under `producer/`. Continues to PASS at HEAD with the 4 new N-I rules' `ci_script` + `tests` arrays populated (`DC-CONS-22`, `CN-STORE-07`, `DC-STORE-07` populated; `DC-CONS-21` carries `ci_script = ""` + `open_obligation` per the follow-on-cluster scope decision). |
| `ci/git-hooks/commit-msg` | **New** (`2047c42`) | Local git hook: rejects commit messages lacking a `Co-Authored-By: Claude ...` trailer. |

### BLUE-list drift closure (`5b70bee`)

| Check | Status | What it checks |
|-------|--------|----------------|
| `ci/ci_check_module_headers.sh` | Modified — BLUE-scope (`5b70bee`) | `// Core Contract:` header on every `.rs` in BLUE crates. Continues to PASS at HEAD: the new N-I BLUE files (`rollback/traits.rs`, `rollback/materialize.rs`, etc.) all carry the canonical Core Contract header. |
| `ci/ci_check_no_semantic_cfg.sh` | Modified — BLUE-scope (`5b70bee`) | No semantic `#[cfg(...)]` in BLUE `src/`. |
| `ci/ci_check_no_signing_in_blue.sh` | Modified — BLUE-scope (`5b70bee`) | No signing primitives in BLUE crates. |
| `ci/ci_check_hash_uses_wire_bytes.sh` | Modified — BLUE-scope (`5b70bee`) | All BLUE hashing via wire-byte fingerprint surfaces. |
| `ci/ci_check_ingress_chokepoints.sh` | Modified — BLUE-scope + registry growth (`5b70bee`) | No raw CBOR decoding outside named chokepoints in BLUE. |
| `ci/ci_check_dependency_boundary.sh` | Modified — BLUE-scope (`5b70bee`) | BLUE crates must not depend on RED crates. Continues to PASS at HEAD: **N-I added no new dep edge**; the N-G S6 edge is RED → BLUE (`ade_runtime → ade_network`), permitted. |

### Phase 4 N-D CI gap closure (`78da6c9`)

| Check | Status | What it checks |
|-------|--------|----------------|
| `ci/ci_check_chaindb_contract.sh` | **New** (`78da6c9`) | `cargo test -p ade_runtime --lib chaindb::` — 8 contract tests. |
| `ci/ci_check_recovery_contract.sh` | **New** (`78da6c9`) | `cargo test -p ade_runtime --lib recovery::` — 6-test recovery bundle. |
| `ci/ci_check_chaindb_crash_safety.sh` | **New** (`78da6c9`) | Smoke variant of the subprocess-SIGKILL harness + integrity post-checks. |

### Phase 4 N-A wire + semantic enforcement (S-A1, S-A10)

| Check | Status | What it checks |
|-------|--------|----------------|
| `ci/ci_check_no_async_in_blue.sh` | **New** (`4fde3a7`, S-A1) | Enforces `DC-CORE-01` — BLUE code is sync-only. Continues to PASS at HEAD over the new N-I BLUE `rollback/` files (no async). |
| `ci/ci_check_ce_n_a_5_proof.sh` | **New** (`56bfa7b`, S-A10) | CE-N-A-5 closure-gate evidence over the real-cardano-node corpus. |

### Phase 4 N-B consensus authority enforcement — extended by B1, B2, N-C, N-G, N-H, N-I

| Check | Status | What it checks |
|-------|--------|----------------|
| `ci/ci_check_consensus_closed_enums.sh` | **New** (N-B); **Modified** (B1, B2, N-C, N-G, N-H); **Modified** (N-I, implicit via new closed enums in `rollback/error.rs` — `MaterializeError`, `CommitRollbackError`) | Closed-enum scan over `ade_core/src/consensus/`, `ade_ledger/src/block_validity/`, `ade_ledger/src/tx_validity/`, `ade_ledger/src/mempool/`, `ade_ledger/src/producer/`, `ade_network/src/chain_sync/server.rs` + `ade_network/src/block_fetch/server.rs`, `ade_ledger/src/receive/events.rs`, and (now) `ade_ledger/src/rollback/error.rs`. |
| `ci/ci_check_no_chaindb_in_consensus_blue.sh` | **New** (N-B / S-B1) | No `ChainDb`/`chain_db` token in `consensus/`. |
| `ci/ci_check_no_density_in_fork_choice.sh` | **New** (N-B / S-B8) | No `density` token in `fork_choice.rs` / `candidate.rs`. |
| `ci/ci_check_no_float_in_consensus.sh` | **New** (N-B / S-B1) | No `f32`/`f64` in `consensus/`. |

### Phase 4 B3 / B3F / B4 / B5 enforcement (carried forward)

| Check | Status | What it checks |
|-------|--------|----------------|
| `ci/ci_check_deposit_param_authority.sh` | **New** (`978c222`) | Enforces `DC-TXV-07` (canonical deposit-param authority). |
| `ci/ci_check_conway_cert_classification_closed.sh` | **New** (`d6c1993`, B3F-S1) | Enforces `DC-TXV-06` — flips `partial` → `enforced`. |
| `ci/ci_check_forbidden_patterns.sh` | **Modified** (`302d22c`, B4-S3/S4) | Enforces `DC-LEDGER-08` — no `non-fatal during replay` rationale; no `Err(_) =>` swallow arm in `accumulate_tx_certs`. |
| `ci/ci_check_gov_cert_accumulation_closed.sh` | **New** (`06385d0`, B5-S4) | Enforces `DC-LEDGER-09` — four-part grep-gate over `apply_conway_gov_cert` totality + arithmetic + removal + env wiring. |

### OQ5 / FIDELITY / WRITEBACK credential discriminant gate (carried forward)

| Check | Status | What it checks |
|-------|--------|----------------|
| `ci/ci_check_credential_discriminant_closed.sh` | **New** (`a3ee2da`, OQ5-S2) | Enforces `DC-LEDGER-10`. **Unmodified by PHASE4-N-I.** |

### PHASE4-N-E wire-level mempool ingress closure (carried forward)

| Check | Status | What it checks |
|-------|--------|----------------|
| `ci/ci_check_mempool_ingress_closure.sh` | **New** (`32c1ee6`, S1) | Enforces `DC-MEM-03`. |
| `ci/ci_check_mempool_ingress_replay.sh` | **New** (`2d0c918`, S2); **Modified** (`509d714`, S3) | Enforces `DC-MEM-04`. |

### PROPOSAL-PROCEDURES-DECODE closure enforcement (carried forward)

| Check | Status | What it checks |
|-------|--------|----------------|
| `ci/ci_check_proposal_procedures_closed.sh` | **New** (`70bc85b`, PP-S1) | Enforces `DC-LEDGER-11`. |

### PHASE4-N-C block-production closure (carried forward)

| Check | Status | What it checks |
|-------|--------|----------------|
| `ci/ci_check_private_key_custody.sh` | **New** (`ea9770e`, S1) | Enforces `DC-CRYPTO-03/04/05` + `OP-OPS-04`. |
| `ci/ci_check_opcert_closed.sh` | **New** (`4cf4b65`, S2) | Enforces `DC-CONS-11` + `DC-CONS-12`. |
| `ci/ci_check_forge_purity.sh` | **New** (`8312690`, S3) | Enforces `DC-CONS-13/14/15` + `DC-LEDGER-12`. |
| `ci/ci_check_no_private_keys_in_corpus.sh` | **New** (`8312690`, S3) | Enforces `DC-CONS-14` + `DC-CRYPTO-03`. |
| `ci/ci_check_no_producer_body_encoder.sh` | **New** (`4fd714c`, S4) | Enforces `DC-CONS-16` — single canonical body-hash authority. |
| `ci/ci_check_self_accept_gate.sh` | **New** (`aa7a7dd`, S5) | Enforces `CN-CONS-07` — type-level broadcast gate. |
| `ci/ci_check_scheduler_closure.sh` | **New** (`58678af`, S6) | Enforces `OP-OPS-05`. |
| `ci/ci_check_producer_corpus_present.sh` | **New** (`694dd74`, S7) | Enforces `CN-CONS-06` (mechanical half). |

### PHASE4-N-G producer-side server response paths closure (`8cd17c9` → `a280954`) — 7 new scripts (the 41st → 47th)

| Check | Status | What it checks |
|-------|--------|----------------|
| `ci/ci_check_no_parallel_header_splitter.sh` | **New** (`8cd17c9`, S1) — the **41st** script | Single-authority header projection. |
| `ci/ci_check_served_chain_closure.sh` | **New** (`dc069cf`, S2) — the **42nd** script | Closure of the BLUE served-chain index. |
| `ci/ci_check_chain_sync_server_closure.sh` | **New** (`cc49b1d`, S3) — the **43rd** script | Enforces `DC-PROTO-08` + `DC-PROTO-07` partial. |
| `ci/ci_check_block_fetch_server_closure.sh` | **New** (`03d120f`, S4) — the **44th** script | Enforces `DC-PROTO-07` + `DC-CONS-17` foundation. |
| `ci/ci_check_broadcast_to_served_purity.sh` | **New** (`1a1b8e0`, S5) — the **45th** script | Enforces `DC-CONS-17` + `DC-CONS-18` + `DC-PROTO-07` GREEN-glue closure. |
| `ci/ci_check_n2n_server_no_signing_dep.sh` | **New** (`f773b1c`, S6) — the **46th** script | Key-boundary doctrine: `ade_runtime/src/network/` MUST NOT import from `producer::signing`. |
| `ci/ci_check_server_paths_corpus_present.sh` | **New** (`a280954`, S7) — the **47th** script | Enforces `RO-LIVE-01` mechanical half. |

### PHASE4-N-H receive-side header→body bridge closure (`b019ee3` → `efe1fb9`) — 5 new scripts (the 48th → 52nd)

| Check | Status | What it checks |
|-------|--------|----------------|
| `ci/ci_check_admitted_block_closure.sh` | **New** (`b019ee3`, S1) — the **48th** script | Enforces `CN-CONS-08` + `CN-PROTO-07`: `AdmittedBlock` private inner field; sole construction site `admit_via_block_validity` composes `block_validity`; receive events closed sums; `AdmittedBlock` distinct from `AcceptedBlock`. |
| `ci/ci_check_receive_reducer_closure.sh` | **New** (`0ecf22f`, S2) — the **49th** script | Enforces `CN-CONS-08` + `DC-CONS-19` + `DC-PROTO-09` + (post-N-I) `DC-CONS-20` rollback-side: reducer purity; `RollForward` mutates only `state.pending_headers` (I-6); `BlockDelivered` composes `admit_via_block_validity` before commit; **post-N-I**: the `RollBackward` arm composes `materialize_rolled_back_state` + `commit_rollback` when `RollbackContext` is provided. |
| `ci/ci_check_receive_replay_purity.sh` | **New** (`c584691`, S3) — the **50th** script | Enforces `DC-PROTO-09` purity of the GREEN adapter + transcript replay. |
| `ci/ci_check_receive_orchestrator_no_producer_dep.sh` | **New** (`1d06089`, S4) — the **51st** script | Key-boundary doctrine: `crates/ade_runtime/src/receive/` MUST NOT import from `crate::producer::signing` / `broadcast` / `scheduler`. |
| `ci/ci_check_receive_paths_corpus_present.sh` | **New** (`efe1fb9`, S6) — the **52nd** script | Enforces `RO-LIVE-02` mechanical half. |

### PHASE4-N-I in-memory snapshot + replay-forward rollback closure (`0e7e9ee` → `75f75da`) — 2 new scripts (the 53rd → 54th)

| Check | Status | What it checks |
|-------|--------|----------------|
| `ci/ci_check_rollback_materialize_closure.sh` | **New** (`0efdce3`, S2) — the **53rd** script | Enforces `CN-STORE-07` + `DC-CONS-22` + `DC-CONS-20` rollback-side via 3 mechanical guards: (1) production code in `crates/ade_ledger/src/rollback/materialize.rs` may not import wall-clock, randomness, async runtime, or HashMap; (2) the SOLE `pub fn` in the `crates/ade_ledger/src/rollback/*` module tree returning `(LedgerState, PraosChainDepState)` is `materialize_rolled_back_state` (single-authority discipline — no parallel rolled-back-state computation path); (3) positive grep: the driver calls `block_validity` (the same authority N-H's receive admit branch uses) — no parallel validation path. |
| `ci/ci_check_snapshot_cadence_purity.sh` | **New** (`3a9bab8`, S4) — the **54th** script | Enforces `DC-STORE-07` snapshot cadence determinism via 3 mechanical guards: (1) `crates/ade_runtime/src/rollback/cadence.rs` production code has no `HashMap` / wall-clock / `tokio` / `rand`; (2) `SnapshotCadence` has exactly one field (`every_n_blocks`) — no operator-tunable runtime input; (3) `in_memory_cache.rs` and `chaindb_block_source.rs` are pure (no I/O, no async, no HashMap; BTreeMap only). |

TRACEABILITY cross-reference: every script listed above appears as a
`ci_script` for at least one rule in `docs/ade-invariant-registry.toml`,
re-traced via `ci/ci_check_constitution_coverage.sh`. **PHASE4-N-I
added 2 new `ci_script ↔ rule` edges** (`ci_check_rollback_materialize_closure.sh`
appears on `DC-CONS-22`, `CN-STORE-07`, and `DC-CONS-20`;
`ci_check_snapshot_cadence_purity.sh` appears on `DC-STORE-07`).
The constitution-coverage gate continues to PASS at HEAD.

---

## 6. Canonical Type Registry Delta

n/a — `.idd-config.json` `canonical_type_registry` is null.
Canonical-type rules live inline in the invariant registry under
family `T`.

**PHASE4-N-I introduced ~12 new closed types** in support of the
in-memory snapshot + replay-forward rollback infrastructure (BLUE +
GREEN combined): `SnapshotReader` (trait), `BlockSource` (trait),
`MaterializeError`, `CommitRollbackError`, `TargetPoint` (rollback's),
`SnapshotCadence`, `InMemorySnapshotCache`, `ChainDbBlockSource`,
`RollbackContext`, plus the extended `ChainDbWrite::rollback_to_slot`
trait method. The `materialize_rolled_back_state` function is the
load-bearing single-authority surface (CN-STORE-07): SOLE `pub fn` in
the rollback module tree returning the rolled-back `(LedgerState,
PraosChainDepState)` tuple; composes one `SnapshotReader::nearest_le`
+ one `BlockSource::blocks_in_range` + per-block `block_validity`
(the same authority N-H's admit branch uses). The `commit_rollback`
function is the rollback's atomic-commit surface: irreversible-step-
first staged commit shape; on failure state unchanged; on success
ledger + chain_dep swap atomically and `pending_headers` resets. The
rest are closed event/effect/error taxonomies + capability records.
Exact whole-project recount belongs to the TRACEABILITY regen that
follows.

**Removals: 0** (expected under append-only discipline).

---

## 7. Normative Rule Delta

The project's invariant registry tracks structured rules (TOML), not
prose normative-doc rules; this section reports on it.

- Rules at baseline (`d509f02:constitution_registry.toml`): **147**
- Rules at prior refresh (`f143984:docs/ade-invariant-registry.toml`): **202**
- Rules at HEAD (`75f75da:docs/ade-invariant-registry.toml`): **206**
- Net additions vs baseline: **+59** (PHASE4-N-A: 2; PHASE4-N-B: 8;
  PHASE4-B1: 6; PHASE4-B2: 5; PHASE4-B3: 2; PHASE4-B3F: 0; PHASE4-B4: 1;
  PHASE4-B5: 1; OQ5: 1; COMMITTEE-CRED / DREP-VOTE / FIDELITY /
  WRITEBACK / post-3d94c22 testkit: 0 each; PHASE4-N-E S1–S5: 2;
  PHASE4-N-E S6: 0; PROPOSAL-PROCEDURES-DECODE: 1; PHASE4-N-C: 14;
  PHASE4-N-G: 6; PHASE4-N-H: 6; **PHASE4-N-I: 4** — `DC-CONS-21`,
  `DC-CONS-22`, `CN-STORE-07`, `DC-STORE-07`, all introduced at
  `declared` in S1 (`0e7e9ee`); `CN-STORE-07` + `DC-CONS-22` flipped
  to `enforced` at S2 (`0efdce3`); `DC-STORE-07` flipped to
  `enforced` at S4 (`3a9bab8`); `DC-CONS-21` remains `declared` with
  `open_obligation = "persistent_ledger_snapshot_encoding_follow_on_cluster"`
  per the explicit scope split; **plus the `DC-CONS-20` admit-only →
  enforced flip at S6 (`75f75da`)**, with
  `cluster = "PHASE4-N-H + PHASE4-N-I"` +
  `strengthened_in = ["PHASE4-N-I"]` + `open_obligation` removed).
- Net additions vs prior refresh: **+4** — the full N-I 4-rule
  family. Plus 1 status flip on a carried-forward rule:
  `DC-CONS-20` `declared` → `enforced`.
- Removals: **0** (expected under append-only discipline; clean).

- **Strengthenings recorded by PHASE4-N-I:**
  - **`DC-CONS-20.strengthened_in += "PHASE4-N-I"`** — the
    ChainDb-ledger-chain_dep lockstep rule was admit-side only at
    N-H close (with `open_obligation =
    "rollback_side_blocked_until_ledger_snapshot_cluster"`); N-I
    closes the rollback-side half (via `materialize_rolled_back_state`
    + `commit_rollback` atomically), flipping the rule from
    `declared` to `enforced` and updating `cluster =
    "PHASE4-N-H + PHASE4-N-I"`. The `open_obligation` is removed.
- **Strengthenings carried forward unchanged**: `DC-PROTO-06`
  (PHASE4-N-A + PHASE4-N-G + PHASE4-N-H — session-replay-equivalence
  across client + server + receive-client transitions); `CN-CONS-07`
  (PHASE4-N-G — broadcast gate preserved across the network seam);
  `DC-MEM-01` (PHASE4-N-E); `DC-MEM-02` (B2); `DC-EPOCH-01`
  (WRITEBACK + oracle); `DC-LEDGER-10` (OQ5 → COMMITTEE-CRED →
  DREP-VOTE → ENACTMENT-COMMITTEE-FIDELITY → WRITEBACK → oracle →
  PPD cross_ref); `DC-LEDGER-08` (B5); `T-DET-01` / `T-ENC-03`
  (OQ5 + N-C cross_ref); `DC-TXV-06` (B3F); `DC-VAL-06` (B3F + B4);
  `T-CONSERV-01` / `CN-LEDGER-07` (B3); `DC-MEM-01,02` (B2);
  `DC-EPOCH-02` (CE-73); the N-D bundle; the N-A real-capture
  bundle; `T-CORE-02` (S-B1); `T-ENC-01` (N-C `block_body_hash`).

- **Open obligations status at HEAD:**
  - **`DC-CONS-20.open_obligation` REMOVED** by PHASE4-N-I S6 — the
    rollback-side half is now mechanically enforced (atomic
    `materialize_rolled_back_state` + `commit_rollback` over
    ChainDb + LedgerState + PraosChainDepState; covered by
    `ci_check_rollback_materialize_closure.sh` + the canonical
    `rollback_then_continue_admit_equals_straight_line_admit` test).
  - **`DC-CONS-21.open_obligation = "persistent_ledger_snapshot_encoding_follow_on_cluster"`**
    — the snapshot encode/decode round-trip rule is `declared`
    pending a follow-on cluster that ships the full canonical
    LedgerState encoder (~1500-2000 LoC of field-walk code mirroring
    `ade_ledger::fingerprint`'s structure; too large for a single
    slice per the explicit PHASE4-N-I scope decision). N-I ships an
    **in-memory** SnapshotReader (the `InMemorySnapshotCache`),
    which fully closes `CN-STORE-07`, `DC-CONS-22`, `DC-STORE-07`,
    and `DC-CONS-20`; persistence across restarts is the follow-on
    cluster's deliverable.
  - **`RO-LIVE-02.open_obligation = "blocked_until_operator_peer_available"`**
    — carried forward from PHASE4-N-H. Unchanged.
  - **`RO-LIVE-01.open_obligation = "blocked_until_operator_peer_available"`**
    — carried forward from PHASE4-N-G. Unchanged.
  - **`CN-CONS-06.open_obligation = "blocked_until_operator_stake_available"`**
    — carried forward from PHASE4-N-C. Unchanged.
  - **`OP-OPS-04.open_obligation`** (Sum6KES skey loader) — carried
    forward from PHASE4-N-C. Unchanged.

Family counts at HEAD: registry total **206** (= 202 + 4 from the
N-I family). The `DC` family grew by 3 (1 CONS-21 + 1 CONS-22 +
1 STORE-07); the `CN` family grew by 1 (STORE-07). Per the
constitution coverage gate, `ci_check_constitution_coverage.sh`
PASSES at HEAD with the 4 new rules' `ci_script` and `tests` arrays
populated (except `DC-CONS-21`, which carries `ci_script = ""` +
`open_obligation` per the follow-on-cluster scope decision); rule
status at HEAD breaks down to **78 enforced, 17 partial, 111
declared** across the 206-entry registry.

Normative-doc rule extraction (the `normative_docs` list in
`.idd-config.json`) is approximate and not regenerated here — the
structured registry is the authoritative source.

---

## Anomalies and Cross-Reference Warnings

- **PHASE4-N-I cluster mechanically closed; one `open_obligation`
  retained (`DC-CONS-21` persistent-encoding), one closed
  (`DC-CONS-20` rollback-side).** All 6 implementing slices (S1 →
  S6) land their CE — every CE-N-I-1..6 is mechanically enforced by
  a named CI script + named tests. The `DC-CONS-21` open obligation
  follows the explicit cluster-scope split: N-I deliberately scopes
  to an **in-memory** SnapshotReader; the persistent on-disk
  encoder (~1500-2000 LoC of field-walk code mirroring
  `ade_ledger::fingerprint`) is too large for a single slice and is
  carved out to a follow-on cluster. The in-memory scope fully
  closes `CN-STORE-07`, `DC-CONS-22`, `DC-STORE-07`, and (via the S6
  wiring) `DC-CONS-20`. This is the documented scope-decision
  pattern, not a discipline gap.
- **CODEMAP / SEAMS / TRACEABILITY are stale at this HEAD — expected
  drift between cluster close and grounding ripple.** This regen
  refreshes HEAD_DELTAS only. Prior CODEMAP (`f143984`) does NOT
  contain the N-I new submodules (`ade_ledger::rollback`,
  `ade_runtime::rollback`); prior SEAMS does NOT contain the
  `materialize_rolled_back_state` single-rollback-authority seam,
  the `commit_rollback` atomic-commit seam, the narrow
  `SnapshotReader` + `BlockSource` trait seams, or the
  `ChainDbWrite::rollback_to_slot` extended persistence seam; prior
  TRACEABILITY does NOT contain the 4 new rules + 2 new
  `ci_script ↔ rule` edges + the `DC-CONS-20` status flip + the
  `DC-CONS-20.strengthened_in` update. The grounding ripple
  immediately following this HEAD_DELTAS regen will bring all four
  docs to self-consistency.
- **No new cross-crate dep edge in PHASE4-N-I.** The new BLUE
  `rollback::*` types are consumed via the existing PHASE4-N-C
  production edge `ade_runtime → ade_ledger`; the N-D `ChainDb`
  trait is reused by `ChainDbBlockSource` through the internal
  `ade_runtime → ade_runtime::chaindb` path (no new edge). No BLUE
  → RED edge was introduced; the BLUE rollback module tree depends
  only on existing BLUE consensus types and the `ChainDbWrite` +
  `SnapshotReader` + `BlockSource` traits.
- **N-I corpus is the existing Conway-576 corpus** (via the N-H
  `receive_pipeline_corpus_drive` test) plus the new synthetic
  admit-then-rollback sequences in `receive_rollback_integration.rs`.
  No new on-disk corpus was added.
- **PHASE4-N-I cluster directory NOT YET archived.** Still at
  active `docs/clusters/PHASE4-N-I/` (8 files: `cluster.md` +
  `N-I-S1.md` through `N-I-S6.md`). Planning spillovers live at
  `docs/planning/phase4-n-i-cluster-slice-plan.md` +
  `docs/planning/ledger-snapshot-rollback-invariants.md`. The
  cluster-close grounding ripple immediately following this
  HEAD_DELTAS regen will archive the cluster directory to
  `docs/clusters/completed/PHASE4-N-I/`.
- **`strengthened_in` records one strengthening for N-I.**
  `DC-CONS-20.strengthened_in = ["PHASE4-N-I"]` — the
  ChainDb-ledger-chain_dep lockstep rule was admit-side only at
  N-H close; N-I closes the rollback-side half. Recorded as a
  proper `strengthened_in` entry, not a cross-ref — continuing the
  canonical pattern N-G normalized.
- **No removed canonical types** (n/a — no separate registry;
  canonical types at HEAD grew by ~12 from the N-I cluster on top
  of N-H's ~12 + N-G's ~10 + N-C's 22 + PPD's 1 since the prior
  baseline-snapshot count).
- **No removed registry rules** (expected: 0; actual: 0). **PHASE4-N-I
  added 4 new rules + flipped 1 carried-forward rule
  (`DC-CONS-20`) from `declared` to `enforced`.** Registry total:
  **206** at HEAD (was 202 at prior refresh).
- **All commit subjects in this regen carry a conventional-commits
  prefix.** The 6 PHASE4-N-I commits are `feat(ledger)` ×3,
  `feat(runtime)` ×3. **All 6 commits in the `f143984..75f75da`
  span carry the repo-required `Co-Authored-By: Claude Opus 4.7
  (1M context)` model-attribution trailer** (per the CLAUDE.md
  project override for the bounty trailer ratio). The project hook
  `ci/git-hooks/commit-msg` is active in this clone and enforces
  the trailer mechanically.
- **Cluster docs archived as of this HEAD.** Eighteen cluster
  directories archived under `docs/clusters/completed/`:
  COMMITTEE-CRED-FIDELITY, DREP-VOTE-FIDELITY,
  ENACTMENT-COMMITTEE-FIDELITY, ENACTMENT-COMMITTEE-WRITEBACK,
  OQ5-CREDENTIAL-FIDELITY, PHASE4-B1, PHASE4-B2, PHASE4-B3,
  PHASE4-B3F, PHASE4-B4, PHASE4-B5, PHASE4-N-A, PHASE4-N-B,
  PHASE4-N-C, PHASE4-N-D, PHASE4-N-E, PHASE4-N-G, PHASE4-N-H,
  PROPOSAL-PROCEDURES-DECODE. PHASE4-N-I cluster directory is
  pending archive (immediately following grounding ripple).
- **B5 / B4 / B3F / B3 / B2 / B1 / N-D / N-B / N-A / PHASE4-N-E /
  PROPOSAL-PROCEDURES-DECODE / PHASE4-N-C / PHASE4-N-G / PHASE4-N-H
  closures — carried forward unchanged.**
- **Pre-existing `boundary_fingerprint_matches_pins` failure on
  `byron_pre_hfc` predates this cluster.** Out-of-scope for
  PHASE4-N-I; not introduced by any N-I slice. Tracked under a
  separate future cluster.
- **DC-VAL status mismatch vs. closure claim (B1, carried forward).**
  Only `DC-VAL-01` is `enforced`; `DC-VAL-02` → `DC-VAL-05` remain
  `declared` despite named tests and the extended closed-enums
  enforcement point. Flip on the next `/traceability` pass.
- **Adversarial corpora are derived, not committed (carried forward).**
  N-E reuses the B2 B-track corpus verbatim; PPD PP-S2 ships its
  corpus in code; PHASE4-N-C ships its corpus in code; PHASE4-N-G
  + PHASE4-N-H + **PHASE4-N-I** all reuse the existing Conway-576
  corpus (no new corpus). The corpus pattern continues the
  no-new-on-disk-artifacts trend.
- **Corpus relayout: credentialed snapshots removed, then regenerated
  off-repo (carried forward).** `corpus/snapshots/` `.gitignore`-d;
  canonical home `s3://ade-corpus-snapshots`.

---

## Generation Notes

Regenerate via `/head-deltas <baseline>` or by re-running the
`head-deltas-generator` agent with the same baseline. Baseline lives
in `.idd-config.json` `head_deltas_baseline` (still `d509f02` —
**this is a cluster-close grounding refresh, not a phase boundary,
so the baseline is unchanged**). Update the baseline on the next
phase boundary (Phase 4 close, which PHASE4-N-I brings further into
reach: the symmetric receive-side bridge is now mechanically closed
admit + rollback; the remaining Phase-4 closure work is operator-
action live evidence for CE-N-C-8 / CE-N-G-8 / CE-N-H-6 /
CE-NODE-N2C-LTX, the persistent on-disk ledger-snapshot encoder
cluster that closes `DC-CONS-21`, the N-F operator surface, and the
OP-OPS-04 Sum6KES skey loader gap). Note the commit-hash rewrite
caveat at the top — re-derive hashes from `git log` at each regen
rather than carrying them forward. This regen is cut at HEAD
`75f75da` (PHASE4-N-I S6). The prior regen narrated HEAD `efe1fb9`
(PHASE4-N-H S6, archived at `f143984`); the new span is
`f143984..75f75da` — 6 commits: `0e7e9ee` (S1 BLUE rollback traits +
closed error sums), `0efdce3` (S2 BLUE materialize_rolled_back_state
driver), `02b5e31` (S3 BLUE commit_rollback atomic helper +
ChainDbWrite trait extension), `3a9bab8` (S4 GREEN snapshot cadence
+ InMemorySnapshotCache + ChainDbBlockSource), `e7add4d` (S5 GREEN
maybe_capture_snapshot hook), `75f75da` (S6 wire RollBackward →
close DC-CONS-20).
