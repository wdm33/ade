# HEAD Deltas — Ade

> **Status:** Living architectural document. Regenerated; not hand-edited.
> Regenerate via `/head-deltas <baseline>`. Baseline is declared in
> `.idd-config.json` (`head_deltas_baseline`).

> Baseline: `d509f02` (Phase 3 handoff snapshot, 2026-04-15)
> HEAD: `efe1fb9` (feat(interop): live_block_follow_session + CE-N-H-6 procedure (PHASE4-N-H S6), 2026-05-26)
> 170 commits, 11,425 files changed, +203,505 / −7,233,633 lines

Headline numbers note: the massive negative line count is dominated by
the **corpus relayout** under `corpus/snapshots/` and the deletion of
the multi-MB credentialed-snapshot text files
(`*_registered_creds.txt`, ~7M lines combined). Source-tree deltas are
far smaller — the per-crate breakdown in §3 is the representative view.

> **Commit-hash note.** This regen runs against the current (rebased)
> history. Earlier HEAD_DELTAS regens referenced commit hashes from a
> history that has since been rewritten; all hashes below are verbatim
> from `git log d509f02..HEAD` at this HEAD.

> **PHASE4-N-H cluster close note (newest thread).** This regen is cut
> at HEAD `efe1fb9`. Since the prior grounding-doc refresh `2adfb45`
> (which closed PHASE4-N-G — archive cluster + refresh
> CODEMAP/SEAMS/HEAD_DELTAS/TRACEABILITY), **six new commits have
> landed** — the **PHASE4-N-H cluster** (S1 → S6) closing the
> receive-side header→body bridge under the Path A scope split
> (admit-only; rollback deferred to a follow-on ledger-snapshot
> cluster). N-H is the symmetric receive-side counterpart to N-C
> (producer) + N-G (producer-side server response paths): bytes flow
> from a peer's `RollForward` (announced header) + `BlockDelivered`
> (body) through the BLUE receive reducer → `block_validity` (B1
> authority) → `AdmittedBlock` token → ChainDb. Sequence: `b019ee3`
> (S1, **BLUE foundation** — new submodule `ade_ledger::receive` with
> three new files: `admitted.rs` (`AdmittedBlock` type-level admission
> gate, ~257 LOC — private constructor reachable only from
> `admit_via_block_validity`, the wrapper around `block_validity` that
> returns `Ok(AdmittedBlock)` iff `BlockValidityVerdict::Valid`; a
> deliberately distinct token from N-C's `AcceptedBlock` so
> producer/receive gates are mechanically non-interfering),
> `chain_write.rs` (narrow `ChainDbWrite` trait taking `AdmittedBlock`
> by value, ~96 LOC; closed `ChainWriteError::{SlotConflict, Underlying}`
> + `ChainWriteErrorKind` static-tag taxonomy — keeps the trait BLUE
> without leaking `ade_runtime` error types), `events.rs` (closed
> sums `ReceiveEvent::{RollForward, RollBackward, BlockDelivered}` +
> `ReceiveEffect` + `ReceiveError` + `NoOpReason` + `TipPoint` +
> `TargetPoint`, ~220 LOC — receive-relevant subset of N-A
> signals/events; locally-originated chain-sync/block-fetch outputs
> are NOT constructible, the CN-PROTO-07 closure), and
> `pending_header_cache.rs` (~169 LOC, BTreeMap-indexed header-bytes
> cache keyed `(slot, block_hash)` for the announce-then-deliver
> sequence). New CI gate `ci_check_admitted_block_closure.sh` (~107
> LOC); **6 new registry rules appended at `declared`** —
> `CN-CONS-08`, `DC-CONS-19`, `DC-CONS-20`, `DC-PROTO-09`,
> `CN-PROTO-07`, `RO-LIVE-02` (196 → 202); `CN-PROTO-07` flipped to
> `enforced` at S1). `0ecf22f` (S2, **BLUE** — new file
> `ade_ledger::receive::reducer` (~628 LOC): pure total transition
> `receive_apply(state, event, deps, chain_write) -> Result<ReceiveEffect, ReceiveError>`
> + `receive_apply_sequence` driver. `RollForward` mutates only
> `state.pending_headers` (Invariant I-6 — never touches
> `ledger`/`chain_dep`/`chain_write`); `BlockDelivered` decodes the
> body, looks up cached header at `(slot, block_hash)`,
> `admit_via_block_validity` composes `block_validity`, persists
> through `ChainDbWrite`, commits new `(ledger, chain_dep)` atomically
> on `Valid`, evicts the consumed header; failure leaves state
> unchanged. `RollBackward` returns `Err(ReceiveError::RollbackOutOfScope)`
> per the Path A scope edge — the rollback half is blocked on a
> separate ledger-snapshot cluster. New CI gate
> `ci_check_receive_reducer_closure.sh`; **`CN-CONS-08` +
> `DC-CONS-19` flipped to `enforced`**). `c584691` (S3, **GREEN** —
> two new GREEN files in `ade_runtime::receive`:
> `events_to_state.rs` (~209 LOC) lifts N-A `ForkChoiceSignal` +
> `BatchDeliveryEvent` values into the BLUE `ReceiveEvent` stream
> (variants that aren't state-changing — `BatchStarted`, `NoBlocks`,
> `BatchCompleted`, `Intersected`, `NoIntersection` — return `None`;
> `header_bytes`/`block_bytes` are NEVER decoded here — pass-through
> discipline preserves the BLUE reducer's `BlockDelivered` branch as
> the canonical decode site); `in_memory_chain_write.rs` (~199 LOC)
> wraps a borrowed `&dyn ChainDb` and exposes `ChainDbWrite::write_admitted`,
> decoding the `AdmittedBlock` bytes once via `decode_block` to
> extract `(slot, hash)` for the `StoredBlock` key, then calling
> `ChainDb::put_block`; maps `ChainDbError` → BLUE `ChainWriteError`.
> Transcript replay test
> `crates/ade_runtime/tests/receive_session_transcript_replay.rs`
> (~175 LOC) drives a synthetic `ForkChoiceSignal` + `BatchDeliveryEvent`
> stream through GREEN adapter + BLUE reducer + InMemoryChainDb twice
> and asserts identical `(ledger fingerprint, ChainDb tip slot,
> ChainDb tip hash)`. New CI gate
> `ci_check_receive_replay_purity.sh`; **`DC-PROTO-09` flipped to
> `enforced`**). `1d06089` (S4, **RED** — new file
> `ade_runtime::receive::orchestrator` (~432 LOC): per-peer N2N
> receive orchestrator — pure state-driver, no socket I/O. Decodes
> inbound chain-sync (client role) + block-fetch (client role) wire
> frames via existing PHASE4-N-A codecs, lifts via S3's GREEN
> adapter, calls BLUE `receive_apply`. Multi-peer: per-peer state is
> independent; single shared `&dyn ChainDb` is the only cross-peer
> coordination point (two peers receiving the same block both
> succeed — `InMemoryChainDb` / `PersistentChainDb` are idempotent on
> byte-identity). Key-boundary doctrine: orchestrator MUST NOT
> import from `crate::producer::signing` / `producer::broadcast` /
> `producer::scheduler` — receive and producer pipelines stay
> independent. Integration test
> `crates/ade_runtime/tests/receive_two_peer_independence.rs`
> (~205 LOC). New CI gate
> `ci_check_receive_orchestrator_no_producer_dep.sh`;
> **`DC-PROTO-06.strengthened_in += "PHASE4-N-H"`** — receive-side
> client-role transitions extend the rule's session-replay-equivalence
> scope alongside N-A's client transitions and N-G's server
> transitions). `3973261` (S5, **mechanical cross-impl evidence** —
> new `crates/ade_runtime/tests/receive_pipeline_corpus_drive.rs`
> (~228 LOC, CE-N-H-5 mechanical adapter) drives the full S4
> pipeline over the Conway-576 corpus block sequence: asserts every
> block admits through the full receive pipeline; ChainDb tip equals
> the expected `(slot, hash)` after each admission; stored bytes equal
> corpus bytes byte-identically; LedgerState fingerprint changes on
> admission. This is the mechanical pre-condition closing RO-LIVE-02's
> bytes-shape claim against a real Cardano corpus, independent of any
> external Haskell peer). `efe1fb9` (S6, **operator-action evidence**
> — new **RED** binary `ade_core_interop::live_block_follow_session`
> (~136 LOC, CE-N-H-6 operator-action evidence) modeled on
> `live_block_fetch_session` (N-G) and `live_block_production_session`
> (N-C): hermetic-default + `--connect` stub for the tokio socket
> bridge wiring; build-and-start test
> `crates/ade_core_interop/tests/live_block_follow_session.rs`
> (~61 LOC); operator procedure
> `docs/clusters/completed/PHASE4-N-H/CE-N-H-6_PROCEDURE.md`; new CI
> gate `ci_check_receive_paths_corpus_present.sh`; **`RO-LIVE-02` set
> to `partial`** with `open_obligation =
> "blocked_until_operator_peer_available"` — mechanical
> pre-condition closed by S5's `receive_pipeline_corpus_drive`; the
> live half is blocked on a private Haskell peer per the documented
> conditional-closure pattern (mirrors N-C `CN-CONS-06`, N-G
> `RO-LIVE-01`, N-E `CE-NODE-N2C-LTX`)). **One new BLUE submodule**
> (`ade_ledger::receive` with `admitted` + `events` + `chain_write` +
> `pending_header_cache` + `reducer`), **one new GREEN+RED submodule**
> (`ade_runtime::receive` with `events_to_state` (GREEN) +
> `in_memory_chain_write` (GREEN) + `orchestrator` (RED)), **one new
> RED binary** (`live_block_follow_session`), **6 new registry rules**
> (all `cluster = "PHASE4-N-H"`: `CN-CONS-08` `enforced`,
> `DC-CONS-19` `enforced`, `DC-CONS-20` `declared` with
> `open_obligation = "rollback_side_blocked_until_ledger_snapshot_cluster"`,
> `DC-PROTO-09` `enforced`, `CN-PROTO-07` `enforced`, `RO-LIVE-02`
> `partial` with `open_obligation = "blocked_until_operator_peer_available"`),
> **1 strengthening** (`DC-PROTO-06`), **5 new CI scripts** (the 48th
> → 52nd: `ci_check_admitted_block_closure.sh`,
> `ci_check_receive_reducer_closure.sh`,
> `ci_check_receive_replay_purity.sh`,
> `ci_check_receive_orchestrator_no_producer_dep.sh`,
> `ci_check_receive_paths_corpus_present.sh`), **no new cross-crate
> dep edges** — the N-G `ade_runtime → ade_network` production edge
> is reused (the orchestrator dispatches existing N-A codecs); the
> N-C `ade_runtime → ade_ledger` edge carries the new BLUE
> `receive::*` types. **Two `open_obligation` entries recorded**:
> `DC-CONS-20` (rollback-side ledger-snapshot dependency) and
> `RO-LIVE-02` (Haskell peer live evidence). **Cluster status at
> HEAD: closed mechanically; CE-N-H-6 live half open as a registry
> obligation; DC-CONS-20 rollback half deferred to a follow-on
> cluster per the Path A scope split**, mirroring the N-C / N-G / N-E
> conditional-closure patterns. **Cluster directory is already
> archived to `docs/clusters/completed/PHASE4-N-H/`** (8 files:
> `cluster.md` + `N-H-S1.md` through `N-H-S6.md` +
> `CE-N-H-6_PROCEDURE.md`; planning spillovers
> `docs/planning/phase4-n-h-cluster-slice-plan.md` +
> `docs/planning/receive-side-bridge-invariants.md`). **No
> CODEMAP/SEAMS/TRACEABILITY refresh yet** for the N-H cluster —
> those three docs are stale relative to this HEAD_DELTAS regen and
> must be regenerated in the grounding ripple immediately following.

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
> PROPOSAL-PROCEDURES-DECODE / PHASE4-N-C / PHASE4-N-G cluster notes
> (carried forward).** All closed and archived at
> `docs/clusters/completed/<NAME>/`.

The delta now covers twenty-eight threads of work. The newest thread —
the **PHASE4-N-H cluster** (`b019ee3` → `efe1fb9`, 6 commits) — sits
on the post-N-G grounding refresh `2adfb45`, which closed +
archived PHASE4-N-G. In rough proportion of the substantive change
budget:

0. **PHASE4-N-H (receive-side header→body bridge — the symmetric
   receive-side counterpart to N-C producer + N-G producer-side
   server response paths; admit-only under the Path A scope split,
   rollback deferred to a follow-on ledger-snapshot cluster) — closed
   in 6 slices.** S1 (`b019ee3`, **BLUE foundation**) introduces the
   new submodule `ade_ledger::receive` with four new files:
   `admitted.rs` (`AdmittedBlock` type-level admission gate —
   private constructor reachable only from `admit_via_block_validity`,
   which composes the existing B1 `block_validity` authority and
   returns `Ok(AdmittedBlock)` iff `BlockValidityVerdict::Valid`;
   deliberately distinct from N-C's `AcceptedBlock` so producer/receive
   gates are mechanically non-interfering — cross-use is a type
   error), `chain_write.rs` (narrow `ChainDbWrite` trait taking
   `AdmittedBlock` by value; closed `ChainWriteError::{SlotConflict,
   Underlying}` with a static `ChainWriteErrorKind` tag taxonomy that
   keeps the trait BLUE without leaking `ade_runtime` error types),
   `events.rs` (closed sums `ReceiveEvent::{RollForward,
   RollBackward, BlockDelivered}` + `ReceiveEffect` + `ReceiveError`
   + `NoOpReason` + `TipPoint` + `TargetPoint` — receive-relevant
   subset of N-A signals/events; locally-originated
   chain-sync/block-fetch outputs RequestNext/RequestRange/
   ClientDone/FindIntersect/Done are NOT constructible — the
   CN-PROTO-07 closure), and `pending_header_cache.rs`
   (BTreeMap-indexed header-bytes cache keyed `(slot, block_hash)`
   for the chain-sync announce-then-deliver sequence — RollForward
   carries header_bytes, BlockDelivered later carries body_bytes for
   the matching `(slot, block_hash)`). New CI gate
   `ci_check_admitted_block_closure.sh`. **Registry**: 6 rules
   appended at `declared` — `CN-CONS-08`, `DC-CONS-19`, `DC-CONS-20`,
   `DC-PROTO-09`, `CN-PROTO-07`, `RO-LIVE-02` (196 → 202);
   `CN-PROTO-07` flipped to `enforced` (closure-gated by the
   admitted-block check + exhaustive-match round-trip tests on each
   closed sum).
   S2 (`0ecf22f`, **BLUE**) introduces the receive bridge's pure
   total transition in new file `ade_ledger::receive::reducer`
   (~628 LOC): `receive_apply(state, event, deps, chain_write) ->
   Result<ReceiveEffect, ReceiveError>` + `receive_apply_sequence`
   driver consuming one `ReceiveEvent` per call. `RollForward`
   mutates only `state.pending_headers` (Invariant I-6 — never
   touches `state.ledger`, `state.chain_dep`, or `chain_write`);
   `BlockDelivered` decodes the body via `decode_block`, looks up
   the cached header at `(slot, block_hash)`, runs
   `admit_via_block_validity` (which composes `block_validity`),
   persists the resulting `AdmittedBlock` through `ChainDbWrite::write_admitted`,
   commits the new `(ledger, chain_dep)` atomically on `Valid`,
   evicts the consumed header; on `HeaderBodyMismatch` (cache miss
   or mismatched cached header) or `BlockValidityError::Invalid` the
   state is unchanged. `RollBackward` returns
   `Err(ReceiveError::RollbackOutOfScope)` per the Path A scope
   edge — the rollback half requires ledger rollback infrastructure
   (LedgerState encode/decode + snapshot+replay-forward driver) that
   does not yet exist; a follow-on rollback cluster closes that
   half. New CI gate `ci_check_receive_reducer_closure.sh`.
   Registry: `CN-CONS-08` + `DC-CONS-19` flipped to `enforced`.
   S3 (`c584691`, **GREEN**) introduces two pure GREEN adapter files
   in new submodule `ade_runtime::receive`: `events_to_state.rs`
   (~209 LOC) lifts N-A `ForkChoiceSignal` (chain-sync) +
   `BatchDeliveryEvent` (block-fetch) values into the BLUE
   `ReceiveEvent` stream — variants that aren't state-changing for
   the receive bridge (`BatchStarted`, `NoBlocks`, `BatchCompleted`,
   `Intersected`, `NoIntersection`) return `None`; the orchestrator
   (S4) filters these out before calling `receive_apply`.
   Pass-through discipline: `header_bytes` and `block_bytes` are
   NEVER decoded in the adapter — the BLUE reducer's `BlockDelivered`
   branch is the canonical decode site. `in_memory_chain_write.rs`
   (~199 LOC) wraps a borrowed `&dyn ChainDb` and exposes the BLUE
   `ChainDbWrite::write_admitted` interface, decoding the
   `AdmittedBlock` bytes once via `decode_block` to extract
   `(slot, hash)` for the `StoredBlock` key, then calling
   `ChainDb::put_block`; maps `ChainDbError` → BLUE `ChainWriteError`
   shape. End-to-end test
   `crates/ade_runtime/tests/receive_session_transcript_replay.rs`
   (~175 LOC) drives a synthetic `ForkChoiceSignal` +
   `BatchDeliveryEvent` stream through GREEN adapter + BLUE reducer
   + InMemoryChainDb twice and asserts identical
   `(ledger fingerprint, ChainDb tip slot, ChainDb tip hash)`. New
   CI gate `ci_check_receive_replay_purity.sh`. Registry:
   `DC-PROTO-09` flipped to `enforced`.
   S4 (`1d06089`, **RED**) introduces the per-peer N2N receive
   orchestrator in new file `ade_runtime::receive::orchestrator`
   (~432 LOC): pure state-driver, no socket I/O (the tokio bridge
   is operator-action work per S6). Decodes inbound chain-sync
   (client role) + block-fetch (client role) wire frames via the
   existing PHASE4-N-A codecs (`decode_chain_sync_message`,
   `decode_block_fetch_message`), lifts via S3's GREEN adapter,
   calls BLUE `receive_apply`. Multi-peer: per-peer state is
   independent — `PerPeerReceiveState` holds the per-session
   reducer state + handshake-negotiated versions; the only
   cross-peer coordination point is the single shared `&dyn ChainDb`
   (two peers receiving the same block both succeed —
   `InMemoryChainDb` / `PersistentChainDb` are idempotent on
   byte-identity, proven by
   `crates/ade_runtime/tests/receive_two_peer_independence.rs`,
   ~205 LOC). Key-boundary doctrine: the receive orchestrator MUST
   NOT import from `crate::producer::signing` /
   `crate::producer::broadcast` / `crate::producer::scheduler` —
   receive and producer pipelines stay independent so receive
   admission cannot accidentally observe producer secret-key custody.
   New CI gate `ci_check_receive_orchestrator_no_producer_dep.sh`.
   Registry: `DC-PROTO-06.strengthened_in += "PHASE4-N-H"` (receive-side
   client-role transitions extend the rule's session-replay-equivalence
   scope alongside N-A's client transitions and N-G's server transitions).
   S5 (`3973261`, **mechanical cross-impl evidence**) ships the
   cluster's bounty-facing surface independent of any external
   Haskell peer. New
   `crates/ade_runtime/tests/receive_pipeline_corpus_drive.rs`
   (~228 LOC, CE-N-H-5 mechanical adapter) drives the full S4
   pipeline over the Conway-576 corpus block sequence:
   `receive_pipeline_corpus_drive_admits_every_block` asserts every
   corpus block admits through the full receive pipeline;
   `receive_pipeline_corpus_drive_chaindb_tip_matches_expected`
   asserts ChainDb tip equals the expected `(slot, hash)` after
   each admission; `receive_pipeline_corpus_drive_admitted_bytes_equal_corpus_bytes`
   asserts stored bytes equal corpus bytes byte-identically;
   `receive_pipeline_corpus_drive_ledger_fingerprint_changes_on_admit`
   asserts LedgerState fingerprint changes on each admission. This
   is the mechanical pre-condition closing RO-LIVE-02's bytes-shape
   claim against a real Cardano corpus.
   S6 (`efe1fb9`, **operator-action evidence**) ships the cluster's
   live-half harness. New **RED** binary
   `ade_core_interop::live_block_follow_session` (~136 LOC,
   CE-N-H-6 operator-action evidence) modeled on
   `live_block_fetch_session` (N-G S7) and
   `live_block_production_session` (N-C S7): hermetic default mode
   prints a readiness banner and exits 0 (no sockets, no operator
   material read); `--connect` mode prints the wiring stub for the
   tokio socket bridge driving the receive orchestrator (S4) against
   a real peer. Args: `--network`, `--magic`, `--target`. Captures
   `docs/clusters/PHASE4-N-H/CE-N-H-LIVE_<date>.log`
   (operator-recorded). Build-and-start test
   `crates/ade_core_interop/tests/live_block_follow_session.rs`
   (~61 LOC). Operator procedure
   `docs/clusters/completed/PHASE4-N-H/CE-N-H-6_PROCEDURE.md` mirrors
   N-G's `CE-N-G-8_PROCEDURE.md` and N-C's `CE-N-C-8_PROCEDURE.md`.
   New CI gate `ci_check_receive_paths_corpus_present.sh`. Registry:
   **`RO-LIVE-02` set to `partial`** with `open_obligation =
   "blocked_until_operator_peer_available"` — the mechanical
   pre-condition is closed by S5's `receive_pipeline_corpus_drive`;
   the live half is blocked on a private Haskell peer per the
   documented conditional-closure pattern. **No new dep edge from
   S6** — `ade_core_interop` already had its `ade_ledger` /
   `ade_network` edges from N-E S4, N-G S7, and the workspace.
1. **PHASE4-N-G (producer-side block-fetch + chain-sync server
   response paths — the engineering bridge between N-C's
   broadcast-queue output and a real Haskell cardano-node peer's
   RequestRange / RequestNext) — closed at HEAD `a280954`, archived
   at `2adfb45`.** Three new BLUE submodules
   (`ade_ledger::producer::served_chain`, `ade_network::chain_sync::server`,
   `ade_network::block_fetch::server`), two new GREEN files, two new
   RED files, one new RED binary, 6 new registry rules, 7 new CI
   scripts; one new non-dev dep edge (`ade_runtime → ade_network`).
2. **PHASE4-N-C (last Tier-1 bounty deliverable — block-production
   authority) — closed at HEAD `694dd74`, archived at `df56e2d`.**
   New BLUE submodule `ade_ledger::producer`, new BLUE module
   `ade_ledger::block_body_hash`, new BLUE module pair
   `ade_core::consensus::opcert_validate` +
   `ade_codec::shelley::opcert`, new RED submodule
   `ade_runtime::producer` + GREEN `tick_assembler`, new RED binary
   `live_block_production_session`. 14 new rules; 8 new CI scripts;
   one new dep edge (`ade_runtime → ade_ledger`).
3. **PROPOSAL-PROCEDURES-DECODE (last open governance-domain decode
   seam) — closed in 2 slices.**
4. **PHASE4-N-E S6 (live N2N tx-submission2 evidence binary) —
   cluster close.**
5. **PHASE4-N-E S1–S5 (wire-level mempool ingress, Tier 1).**
6. **Post-WRITEBACK testkit follow-ups (four commits, GREEN-scope).**
7. **ENACTMENT-COMMITTEE-WRITEBACK — closed.**
8. **ENACTMENT-COMMITTEE-FIDELITY — closed.**
9. **DREP-VOTE-FIDELITY — closed.**
10. **COMMITTEE-CRED-FIDELITY — closed.**
11. **OQ5-CREDENTIAL-FIDELITY — closed.**
12. **Phase 4 cluster B5 (Conway gov-cert accumulation) — closed.**
13. **Phase 4 cluster B4 (Conway cert-state accumulation,
    fail-closed) — closed.**
14. **Phase 4 cluster B3F (follow-up hardening) — committed.**
15. **Phase 4 cluster B3 (Conway value-conservation accounting) —
    closed.**
16. **Phase 4 cluster B2 (tx validity agreement) — closed.**
17. **Phase 4 cluster B1 (full block validity agreement) — closed.**
18. **Phase 4 cluster N-A (network mini-protocols) — closed.**
19. **Phase 4 cluster N-B (consensus runtime) — closed.**
20. **CE-N-B-6 follow-mode bridge.**
21. **Phase 4 cluster N-D (ChainDB persistence) — closed.**
22. **Phase 2C close-out / CE-73 reclassification.**
23. **IDD canonicalization.**
24. **Grounding-doc generation + ripple.** Successive refreshes,
    including `52642e5`, `350130e`, `3af9e2b`, `96d043c`, `df56e2d`,
    `2adfb45`.
25. **BLUE-list drift closure.** Six CI scripts extended to full
    BLUE scope.
26. **Corpus relayout.** Credentialed `*_registered_creds.txt`
    removed (~7M-line negative); `corpus/snapshots/` now
    `.gitignore`-d.

---

## 1. Commit Log

| Hash | Type | Summary |
|------|------|---------|
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
| `ade_ledger::receive` (new submodule of an existing BLUE crate) | BLUE | **Receive-side header→body bridge.** Pure total transition consuming peer-originated `ReceiveEvent` values (`RollForward`, `RollBackward`, `BlockDelivered`), composing the existing B1 `block_validity` authority through the new type-level admission gate `AdmittedBlock` (private constructor reachable only from `admit_via_block_validity`), and persisting through the narrow BLUE `ChainDbWrite` trait. Locally-originated chain-sync/block-fetch outputs (RequestNext, RequestRange, ClientDone, FindIntersect, Done) are NOT constructible — the CN-PROTO-07 closure. `RollBackward` returns `Err(ReceiveError::RollbackOutOfScope)` per the Path A scope edge — the rollback half awaits a follow-on ledger-snapshot cluster. **Symmetric receive-side counterpart to N-C's `ade_ledger::producer` (producer authority) + N-G's `ade_ledger::producer::served_chain` (producer-side served chain).** Enforced by `ci_check_admitted_block_closure.sh` + `ci_check_receive_reducer_closure.sh`. | `receive/mod.rs`, `receive/admitted.rs` (~257 LOC), `receive/chain_write.rs` (~96 LOC), `receive/events.rs` (~220 LOC), `receive/pending_header_cache.rs` (~169 LOC), `receive/reducer.rs` (~628 LOC) | PHASE4-N-H / S1 (`b019ee3`); S2 reducer (`0ecf22f`) |
| `ade_runtime::receive` (new submodule of an existing RED crate) | GREEN+RED mix | **Imperative-shell composition for the N2N receive bridge.** GREEN: `events_to_state.rs` lifts N-A `ForkChoiceSignal` + `BatchDeliveryEvent` values into the BLUE `ReceiveEvent` stream (pass-through discipline — bytes are NEVER decoded here; the BLUE reducer's `BlockDelivered` branch is the canonical decode site); `in_memory_chain_write.rs` wraps a borrowed `&dyn ChainDb` and exposes `ChainDbWrite::write_admitted`. RED: `orchestrator.rs` is the per-peer N2N receive orchestrator — pure state-driver decoding inbound chain-sync (client role) + block-fetch (client role) wire frames via the existing PHASE4-N-A codecs, lifting via S3 GREEN, calling BLUE `receive_apply`. Multi-peer determinism: per-peer state is independent; the only cross-peer coordination point is the single shared `&dyn ChainDb`. **Key-boundary doctrine**: `orchestrator.rs` MUST NOT import from `crate::producer::signing` / `producer::broadcast` / `producer::scheduler`. Enforced by `ci_check_receive_replay_purity.sh` + `ci_check_receive_orchestrator_no_producer_dep.sh`. | `receive/mod.rs`, `receive/events_to_state.rs` (~209 LOC, GREEN), `receive/in_memory_chain_write.rs` (~199 LOC, GREEN), `receive/orchestrator.rs` (~432 LOC, RED) | PHASE4-N-H / S3 (`c584691`); S4 orchestrator (`1d06089`) |
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
| `ade_network` (new workspace crate) | BLUE-majority (per-submodule scoped) | Ouroboros mini-protocol authority. **N-G S1 / S3 / S4 added the new server-side files** `chain_sync/server.rs` + `block_fetch/server.rs` + their ServerReply wrappers; the rest is unchanged. **N-H added no new file in this crate** — the new receive orchestrator (`ade_runtime::receive::orchestrator`) dispatches the existing PHASE4-N-A codecs. | `codec/`, `handshake/`, `chain_sync/`, `block_fetch/`, `tx_submission/`, `keep_alive/`, `peer_sharing/`, `n2c/`, `mux/frame.rs` (BLUE), `mux/transport.rs` (RED), `session/` (RED), `chain_sync/server.rs` (**N-G S1/S3**), `block_fetch/server.rs` (**N-G S1/S4**) | PHASE4-N-A; PHASE4-N-G |
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
Both are RED-or-mixed. **PHASE4-N-H added no new crate** — S1's
BLUE `ade_ledger::receive` submodule, S3/S4's GREEN+RED
`ade_runtime::receive` submodule, and S6's RED binary all live as
new files / submodules under the existing 8 workspace crates.
**PHASE4-N-G added no new crate** either (server reducers, served
chain, GREEN adapter, RED driver, and live-fetch binary all live as
new files under existing crates).

Crate dependency shape at HEAD: **PHASE4-N-H added no new cross-crate
dep edges.** The PHASE4-N-G S6 production edge `ade_runtime →
ade_network` is reused (the receive orchestrator dispatches the
existing PHASE4-N-A codecs `decode_chain_sync_message` +
`decode_block_fetch_message`); the PHASE4-N-C S6 production edge
`ade_runtime → ade_ledger` carries the new BLUE `receive::*` types
(`receive_apply`, `ChainDbWrite`, `ReceiveEffect`, `ReceiveError`,
`ReceiveState`). **PHASE4-N-G S6** added one new non-dev dep edge
(`ade_runtime → ade_network`) and four new dev-dep edges on
`ade_network` (`ade_ledger`, `ade_testkit`, `ade_core`, `ade_crypto`,
S3). Carried forward: the **PHASE4-N-C S6** edge `ade_runtime →
ade_ledger` and the **PHASE4-N-E S4** edge `ade_core_interop →
ade_ledger`. No edge from a BLUE crate to a RED crate was
introduced. Dependency direction RED → BLUE is permitted by
`ci_check_dependency_boundary.sh`.

Corpora at HEAD: N-A capture corpus, N-B replay corpus, B1 validity
corpus, B3 conservation corpora, B4/B5 README-only synthetic notes,
the credential-fidelity corpus from OQ5-S2, the PPD in-code
synthetic-canonical corpus, the N-C in-code synthetic producer
corpus, and `corpus/snapshots/` under `.gitignore` (canonical home
`s3://ade-corpus-snapshots`). **PHASE4-N-H added no external
corpus** — the receive transcript replay test
(`tests/receive_session_transcript_replay.rs`) drives against
synthetic events; the receive pipeline corpus drive
(`tests/receive_pipeline_corpus_drive.rs`) and the live-follow
binary build-and-start test both drive against **the existing
Conway-576 corpus** consumed by N-C S3/S7, N-G S5/S7, and B1.
The CE-N-H-6 operator-action evidence is a live log captured against
a private Haskell peer, not a committed corpus. PHASE4-N-G likewise
added no new on-disk corpus.

Cross-reference: **The `ade-CODEMAP.md` regenerated in parallel with
this HEAD_DELTAS will record the new BLUE submodule
`ade_ledger::receive` (with `admitted`, `chain_write`, `events`,
`pending_header_cache`, `reducer`), the new GREEN+RED submodule
`ade_runtime::receive` (with `events_to_state`, `in_memory_chain_write`,
`orchestrator`), and the new RED binary `ade_core_interop::live_block_follow_session`**
as rows under their respective crates' BLUE/GREEN/RED listings; the
prior CODEMAP at `2adfb45` does NOT yet contain any of those. SEAMS
will pick up `AdmittedBlock` as the single canonical receive-side
admission gate seam (symmetric to N-C's `AcceptedBlock` broadcast
gate), the BLUE `ChainDbWrite` trait as the receive-bridge persistence
seam, and the `PendingHeaderCache` `(slot, block_hash)` BTreeMap as
the chain-sync announce-then-deliver coordination seam. TRACEABILITY
will pick up the 6 new registry rules (`CN-CONS-08`, `DC-CONS-19`,
`DC-CONS-20`, `DC-PROTO-09`, `CN-PROTO-07`, `RO-LIVE-02`) with their
5 new `ci_script ↔ rule` edges plus the strengthening
(`DC-PROTO-06`); the prior TRACEABILITY at `2adfb45` does NOT
contain any of them. All three rewrites are in flight in the
grounding ripple immediately following this HEAD_DELTAS regen; the
four docs will be self-consistent at the next grounding-doc commit.

---

## 3. Modules Modified

| Module | Scope | Key changes |
|--------|-------|-------------|
| `ade_ledger` | +75 source/test files over the full delta; **PHASE4-N-H touched 6 files**: `receive/mod.rs` (new, ~36 LOC: register submodule), `receive/admitted.rs` (new, ~257 LOC), `receive/chain_write.rs` (new, ~96 LOC), `receive/events.rs` (new, ~220 LOC), `receive/pending_header_cache.rs` (new, ~169 LOC), `receive/reducer.rs` (new, ~628 LOC), `lib.rs` (+1: register `receive`). **PHASE4-N-G touched 5 files** (carried forward). | **PHASE4-N-H (S1 + S2):** new BLUE submodule `ade_ledger::receive` — the receive-side header→body bridge. `AdmittedBlock` type-level admission gate with private constructor reachable only from `admit_via_block_validity` (which composes the existing B1 `block_validity` authority); narrow `ChainDbWrite` trait taking `AdmittedBlock` by value; closed sums `ReceiveEvent` / `ReceiveEffect` / `ReceiveError` / `NoOpReason` (CN-PROTO-07 closure forbids locally-originated chain-sync/block-fetch outputs in the receive surface); BTreeMap-indexed `PendingHeaderCache` keyed `(slot, block_hash)`; pure total reducer `receive_apply` mutating only `state.pending_headers` on `RollForward` (I-6), atomically committing `(ledger, chain_dep)` on `Valid` from `block_validity`, returning `Err(RollbackOutOfScope)` on `RollBackward` per the Path A scope edge. **`DC-PROTO-06.strengthened_in += "PHASE4-N-H"`** (receive-side client-role transitions extend the rule's session-replay-equivalence scope). **PHASE4-N-G (S1 + S2 + S5, carried forward):** new BLUE `producer::served_chain`; lifted `accepted_block_header_bytes` as the single canonical header projection; `self_accept.rs` extension accessors. **Carried forward:** PHASE4-N-C producer submodule + `block_body_hash`; PHASE4-N-E mempool ingress chokepoint; B-series; OQ5/FIDELITY/WRITEBACK; PROPOSAL-PROCEDURES-DECODE. |
| `ade_runtime` | +24 files, +5,840 lines from prior threads; **PHASE4-N-H added 4 source files + 3 integration tests / +1,041 LOC**: `receive/mod.rs` (~26 LOC, S3+S4: register `events_to_state`, `in_memory_chain_write`, `orchestrator`), `receive/events_to_state.rs` (~209 LOC, S3 GREEN), `receive/in_memory_chain_write.rs` (~199 LOC, S3 GREEN), `receive/orchestrator.rs` (~432 LOC, S4 RED), `lib.rs` (+1: register `receive`); + tests `receive_session_transcript_replay.rs` (~175 LOC, S3), `receive_two_peer_independence.rs` (~205 LOC, S4), `receive_pipeline_corpus_drive.rs` (~228 LOC, S5). **PHASE4-N-G added 6 source files + 3 integration tests / +1,214 LOC** (carried forward). | **PHASE4-N-H (S3 + S4 + S5):** new GREEN+RED submodule `ade_runtime::receive`. GREEN `events_to_state` lifts N-A `ForkChoiceSignal` + `BatchDeliveryEvent` into BLUE `ReceiveEvent` with pass-through discipline (bytes never decoded in the adapter — BLUE reducer is the canonical decode site); GREEN `in_memory_chain_write` wraps `&dyn ChainDb` and exposes `ChainDbWrite::write_admitted`, decoding the AdmittedBlock once for the StoredBlock key and mapping `ChainDbError` → BLUE `ChainWriteError`. RED `orchestrator` is the pure per-peer N2N receive state-driver — decodes inbound chain-sync (client) + block-fetch (client) frames via PHASE4-N-A codecs, lifts via S3 GREEN, calls BLUE `receive_apply`. Multi-peer determinism: per-peer state is independent; only cross-peer coordination is the single shared `&dyn ChainDb` (idempotent on byte-identity; proven by `receive_two_peer_independence`). Key-boundary doctrine: `receive::orchestrator` MUST NOT import from `crate::producer::signing` / `broadcast` / `scheduler`. Three integration tests cover DC-PROTO-09 transcript determinism, two-peer independence, and CE-N-H-5 mechanical cross-impl over Conway-576. **No new dep edge** — reuses the N-G `ade_runtime → ade_network` production edge for the codec imports and the N-C `ade_runtime → ade_ledger` edge for the new BLUE `receive::*` types. **Carried forward:** N-G `producer::broadcast_to_served` + `producer::served_chain_lookups` + `network::n2n_server`; N-C producer submodule; N-B consensus runtime; N-D chaindb/recovery. |
| `ade_core_interop` | +1,793 across 9 files from prior threads; **PHASE4-N-H added 2 files / +197 LOC**: `src/bin/live_block_follow_session.rs` (~136 LOC, S6 RED binary), `tests/live_block_follow_session.rs` (~61 LOC, S6 build-and-start test). `Cargo.toml` (+4) adds the new `[[bin]]` entry. **PHASE4-N-G added 2 files / +202 LOC** (carried forward). | **PHASE4-N-H (S6):** new RED operator-action evidence binary modeled on `live_block_fetch_session` (N-G S7) and `live_block_production_session` (N-C S7); hermetic-default + `--connect` stub for the tokio socket bridge. **Carried forward:** N-G S7 live-fetch binary; N-C S7 producer-side live binary; N-E S4/S5/S6 tx-submission bridges; CE-N-B-6 follow-bridge. |
| `ade_network` | 100 files, +17,861 lines (full N-A); **PHASE4-N-G added 4 files / +1,402 LOC** (carried forward); **no PHASE4-N-H change** — the receive orchestrator (`ade_runtime::receive::orchestrator`) dispatches the existing PHASE4-N-A codecs unchanged. | **Unchanged in PHASE4-N-H.** **Carried forward:** N-G server reducers + ServerReply wrappers; N-A wire-grammar work + DoS hardening. |
| `ade_ledger` (block_validity sub-area) | (counted above) | **No PHASE4-N-H change** — the N-H reducer composes `block_validity` unchanged through `admit_via_block_validity`. **Carried forward:** N-G S1 `accepted_block_header_bytes` lift. |
| `ade_core` | +30 source files + tests (N-B); +828 / −86 across 16 files (B1); +1 new file (N-C S2). **No PHASE4-N-H source change** — the N-H reducer depends only on existing `ade_core::consensus::{era_schedule, ledger_view, praos_state}` types. **No PHASE4-N-G source change** either. | **Unchanged in PHASE4-N-H.** **Carried forward:** N-B consensus authority + N-C S2 opcert validator. |
| `ade_codec` | +14 source/test files over the full delta; **no PHASE4-N-H change** — the receive reducer composes existing `block_validity::decode_block` for the body decode and `pending_header_cache` for the announced-header storage; no new codec authority. | **Unchanged in PHASE4-N-H.** **Carried forward:** PPD PP-S1 `conway::governance`; N-C S2 `shelley::opcert`; N-C S3 `shelley::tx_components` producer assembly path; B3 / B4 / OQ5 era decoder work. |
| `ade_crypto` | 2 files: `kes.rs` (+122 / −81), `lib.rs` (+5). **No PHASE4-N-H change.** | **Unchanged in PHASE4-N-H.** **Carried forward:** N-C S1 `KesSignature` + `verify_kes_signature`. |
| `ade_testkit` | +33 files across the full delta; **no PHASE4-N-H change** — N-H's tests live as integration tests under `ade_runtime/tests/` and `ade_core_interop/tests/`, not under the testkit crate (same pattern as N-G). | **Unchanged in PHASE4-N-H.** **Carried forward:** N-C `producer/` harness; PPD PP-S2 `governance/proposal_procedures_replay`; N-E `mempool/ingress_replay`. |

No other crate had non-trivial source changes since baseline.
`ade_plutus` and `ade_node` were untouched by code commits.
**PHASE4-N-H touched 3 of 8 workspace crates** (`ade_ledger`,
`ade_runtime`, `ade_core_interop`). **No `.idd-config.json` change.**
**No BLUE authority-path semantics changed apart from the new
receive surfaces** — the prior validator authorities
(`ade_core::consensus::*`, `ade_ledger::block_validity::*`,
`ade_ledger::block_body_hash`) and producer authorities
(`ade_ledger::producer::*`) were re-used unchanged by the receive
reducer. The receive reducer composes `block_validity` through
`admit_via_block_validity` without changing what `block_validity`
computes.

---

## 4. Feature Flags

No Cargo `[features]` tables exist at HEAD in any workspace crate, and
none existed at baseline. The project does not use Cargo feature flags
as a semantic surface — closed semantic surfaces are encoded in the
type system per the IDD core principles, and conditional compilation
is checked out of BLUE code via `ci/ci_check_no_semantic_cfg.sh`
(scoped over the full 6-crate BLUE set, covering all surfaces
introduced through the PHASE4-N-H receive submodule, the PHASE4-N-G
server reducers, served-chain index, and header-projection seam).

No `#[cfg(feature = ...)]` gates appear at either ref.
**PHASE4-N-H introduced no new Ade-side feature flag and no new
upstream-crate feature selection.**

**Status: unchanged — zero Ade feature flags at baseline, zero at HEAD.**

---

## 5. CI Checks

The CI surface is the shell-script set under `ci/` (no
`.github/workflows` in this repo). At baseline there were 15 scripts.
At HEAD there are **52 scripts plus one git hook**
(`ci/git-hooks/commit-msg`). Across the full delta: CE-73 added one,
N-D added three, N-A added two, N-B added four, B3 added one, B3F
added one, B5 added one, OQ5 added one, PHASE4-N-E S1/S2 added two,
PROPOSAL-PROCEDURES-DECODE PP-S1 added one, PHASE4-N-C added eight
(the 33rd → 40th), PHASE4-N-G added seven (the 41st → 47th), and
**PHASE4-N-H added five (the 48th → 52nd)**:
`ci_check_admitted_block_closure.sh` (S1),
`ci_check_receive_reducer_closure.sh` (S2),
`ci_check_receive_replay_purity.sh` (S3),
`ci_check_receive_orchestrator_no_producer_dep.sh` (S4),
`ci_check_receive_paths_corpus_present.sh` (S6).
Grouped by cluster.

### CE-73 reclassification (Phase 2C close-out)

| Check | Status | What it checks |
|-------|--------|----------------|
| `ci/ci_check_hfc_translation.sh` | **New** (`9b15378`) | CE-73-semantic gate: runs the three HFC ledger-side translation proof surfaces. Authoritative test for invariant `DC-EPOCH-02`. |

### IDD canonicalization (post-Phase-4-N-D)

| Check | Status | What it checks |
|-------|--------|----------------|
| `ci/ci_check_constitution_coverage.sh` | Modified (`39865f6`, `aa7a7dd`) | Path-only edit (`39865f6`): registry path now `docs/ade-invariant-registry.toml`. **Extended** (`aa7a7dd`, N-C S5) to recognize new closed types under `producer/`. |
| `ci/git-hooks/commit-msg` | **New** (`2047c42`) | Local git hook: rejects commit messages lacking a `Co-Authored-By: Claude ...` trailer. |

### BLUE-list drift closure (`5b70bee`)

| Check | Status | What it checks |
|-------|--------|----------------|
| `ci/ci_check_module_headers.sh` | Modified — BLUE-scope (`5b70bee`) | `// Core Contract:` header on every `.rs` in BLUE crates. Continues to PASS at HEAD: the new N-H BLUE files (`receive/admitted.rs`, `receive/reducer.rs`, etc.) all carry the canonical Core Contract header. |
| `ci/ci_check_no_semantic_cfg.sh` | Modified — BLUE-scope (`5b70bee`) | No semantic `#[cfg(...)]` in BLUE `src/`. |
| `ci/ci_check_no_signing_in_blue.sh` | Modified — BLUE-scope (`5b70bee`) | No signing primitives in BLUE crates. |
| `ci/ci_check_hash_uses_wire_bytes.sh` | Modified — BLUE-scope (`5b70bee`) | All BLUE hashing via wire-byte fingerprint surfaces. |
| `ci/ci_check_ingress_chokepoints.sh` | Modified — BLUE-scope + registry growth (`5b70bee`) | No raw CBOR decoding outside named chokepoints in BLUE. |
| `ci/ci_check_dependency_boundary.sh` | Modified — BLUE-scope (`5b70bee`) | BLUE crates must not depend on RED crates. Continues to PASS at HEAD: the N-G S6 new edge is RED → BLUE (`ade_runtime → ade_network`), permitted; **N-H added no new dep edge.** |

### Phase 4 N-D CI gap closure (`78da6c9`)

| Check | Status | What it checks |
|-------|--------|----------------|
| `ci/ci_check_chaindb_contract.sh` | **New** (`78da6c9`) | `cargo test -p ade_runtime --lib chaindb::` — 8 contract tests. |
| `ci/ci_check_recovery_contract.sh` | **New** (`78da6c9`) | `cargo test -p ade_runtime --lib recovery::` — 6-test recovery bundle. |
| `ci/ci_check_chaindb_crash_safety.sh` | **New** (`78da6c9`) | Smoke variant of the subprocess-SIGKILL harness + integrity post-checks. |

### Phase 4 N-A wire + semantic enforcement (S-A1, S-A10)

| Check | Status | What it checks |
|-------|--------|----------------|
| `ci/ci_check_no_async_in_blue.sh` | **New** (`4fde3a7`, S-A1) | Enforces `DC-CORE-01` — BLUE code is sync-only. Continues to PASS at HEAD over the new N-H BLUE `receive/` files (no async). |
| `ci/ci_check_ce_n_a_5_proof.sh` | **New** (`56bfa7b`, S-A10) | CE-N-A-5 closure-gate evidence over the real-cardano-node corpus. |

### Phase 4 N-B consensus authority enforcement — extended by B1, B2, N-C, N-G, and N-H

| Check | Status | What it checks |
|-------|--------|----------------|
| `ci/ci_check_consensus_closed_enums.sh` | **New** (N-B); **Modified** (B1, `7b95ccd`); **Modified** (B2); **Modified** (N-C); **Modified** (N-G); **Modified** (N-H, implicit via new closed enums in `receive/events.rs` — `ReceiveEvent`, `ReceiveEffect`, `ReceiveError`, `NoOpReason`, and the structural records `TipPoint`, `TargetPoint`) | Closed-enum scan over `ade_core/src/consensus/`, `ade_ledger/src/block_validity/`, `ade_ledger/src/tx_validity/`, `ade_ledger/src/mempool/`, `ade_ledger/src/producer/`, `ade_network/src/chain_sync/server.rs` + `ade_network/src/block_fetch/server.rs`, and (now) `ade_ledger/src/receive/events.rs`. |
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
| `ci/ci_check_credential_discriminant_closed.sh` | **New** (`a3ee2da`, OQ5-S2) | Enforces `DC-LEDGER-10`. **Unmodified by PHASE4-N-H.** |

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
| `ci/ci_check_no_parallel_header_splitter.sh` | **New** (`8cd17c9`, S1) — the **41st** script | Single-authority header projection (foundation for `DC-CONS-16` strengthening + `DC-CONS-18` + `CN-PROTO-06`). |
| `ci/ci_check_served_chain_closure.sh` | **New** (`dc069cf`, S2) — the **42nd** script | Closure of the BLUE served-chain index (BTreeMap-only, no caller-supplied asserted hash). |
| `ci/ci_check_chain_sync_server_closure.sh` | **New** (`cc49b1d`, S3) — the **43rd** script | Enforces `DC-PROTO-08` + `DC-PROTO-07` partial. |
| `ci/ci_check_block_fetch_server_closure.sh` | **New** (`03d120f`, S4) — the **44th** script | Enforces `DC-PROTO-07` + `DC-CONS-17` foundation. |
| `ci/ci_check_broadcast_to_served_purity.sh` | **New** (`1a1b8e0`, S5) — the **45th** script | Enforces `DC-CONS-17` + `DC-CONS-18` + `DC-PROTO-07` GREEN-glue closure. |
| `ci/ci_check_n2n_server_no_signing_dep.sh` | **New** (`f773b1c`, S6) — the **46th** script | Key-boundary doctrine: `ade_runtime/src/network/` MUST NOT import from `producer::signing`. |
| `ci/ci_check_server_paths_corpus_present.sh` | **New** (`a280954`, S7) — the **47th** script | Enforces `RO-LIVE-01` mechanical half (CE-N-G-7 + CE-N-G-8 binary presence). |

### PHASE4-N-H receive-side header→body bridge closure (`b019ee3` → `efe1fb9`) — 5 new scripts (the 48th → 52nd)

| Check | Status | What it checks |
|-------|--------|----------------|
| `ci/ci_check_admitted_block_closure.sh` | **New** (`b019ee3`, S1) — the **48th** script | Enforces `CN-CONS-08` + `CN-PROTO-07` via multiple mechanical guards: (1) `AdmittedBlock` has a private inner field — only `admit_via_block_validity` constructs it; no struct-literal back-door; (2) `admit_via_block_validity` is the only public path from raw bytes to `AdmittedBlock` and composes `block_validity`; (3) no production code in `ade_ledger/src/receive/events.rs` may reference any locally-originated chain-sync/block-fetch variant name (RequestNext, RequestRange, ClientDone, FindIntersect, Done) — the CN-PROTO-07 closure positively asserts only `RollForward` / `RollBackward` / `BlockDelivered` are constructible; (4) `AdmittedBlock` is deliberately distinct from `AcceptedBlock` (cross-use is a type error). |
| `ci/ci_check_receive_reducer_closure.sh` | **New** (`0ecf22f`, S2) — the **49th** script | Enforces `CN-CONS-08` + `DC-CONS-19` + `DC-PROTO-09` via multiple mechanical guards: (1) `reducer.rs` may not import wall-clock / randomness / async runtime / `HashMap`; (2) `RollForward` branch mutates only `state.pending_headers` (Invariant I-6 — no path mutates `state.ledger` / `state.chain_dep` / `chain_write` in the RollForward arm); (3) `BlockDelivered` branch composes `admit_via_block_validity` before any state commit (positive presence + ordering check); (4) `RollBackward` branch returns `Err(ReceiveError::RollbackOutOfScope)` (positive presence; the Path A scope edge). |
| `ci/ci_check_receive_replay_purity.sh` | **New** (`c584691`, S3) — the **50th** script | Enforces `DC-PROTO-09` via 3+ mechanical guards: (1) `ade_runtime/src/receive/events_to_state.rs` and `in_memory_chain_write.rs` may not import wall-clock / randomness / async runtime / `HashMap`; (2) `events_to_state.rs` never decodes bytes (pass-through discipline — `header_bytes`/`block_bytes` flow through verbatim; the BLUE reducer is the canonical decode site); (3) the transcript-replay integration test `receive_session_transcript_replay_byte_identical` is present and asserts byte-identical replay state. |
| `ci/ci_check_receive_orchestrator_no_producer_dep.sh` | **New** (`1d06089`, S4) — the **51st** script | Key-boundary doctrine for the receive orchestrator: `crates/ade_runtime/src/receive/` MUST NOT import from `crate::producer::signing` / `crate::producer::broadcast` / `crate::producer::scheduler`. Receive and producer pipelines stay independent — the receive orchestrator cannot accidentally observe producer secret-key custody. Single mechanical grep over the receive module tree. |
| `ci/ci_check_receive_paths_corpus_present.sh` | **New** (`efe1fb9`, S6) — the **52nd** script | Enforces `RO-LIVE-02` mechanical half (CE-N-H-5 + CE-N-H-6 binary presence) via 4 mechanical guards: (1) the mechanical cross-impl integration test `crates/ade_runtime/tests/receive_pipeline_corpus_drive.rs` exists with the expected test names; (2) the transcript replay test `crates/ade_runtime/tests/receive_session_transcript_replay.rs` exists; (3) the `live_block_follow_session` binary source file + `[[bin]]` entry are present; (4) the CE-N-H-6 procedure doc `docs/clusters/completed/PHASE4-N-H/CE-N-H-6_PROCEDURE.md` exists. |

TRACEABILITY cross-reference: every script listed above appears as a
`ci_script` for at least one rule in `docs/ade-invariant-registry.toml`,
re-traced via `ci/ci_check_constitution_coverage.sh`. **PHASE4-N-H
added 5 new `ci_script ↔ rule` edges** (one per new script; see §7).
The constitution-coverage gate continues to PASS at HEAD.

---

## 6. Canonical Type Registry Delta

n/a — `.idd-config.json` `canonical_type_registry` is null.
Canonical-type rules live inline in the invariant registry under
family `T`.

**PHASE4-N-H introduced ~12 new closed types** in support of the
receive-side header→body bridge: `AdmittedBlock`, `AdmittedOutcome`,
`ChainDbWrite` (trait), `ChainWriteError`, `ChainWriteErrorKind`,
`ReceiveEvent`, `ReceiveEffect`, `ReceiveError`, `NoOpReason`,
`TipPoint`, `TargetPoint`, `PendingHeaderCache`, `ReceiveState`. The
`AdmittedBlock` token is the load-bearing type-level admission gate
(CN-CONS-08): private inner field, single construction site
`admit_via_block_validity` reachable only when `block_validity` returns
`BlockValidityVerdict::Valid`; the `ChainDbWrite::write_admitted`
trait takes `AdmittedBlock` by value, preserving the gate across the
trait surface. `AdmittedBlock` is deliberately distinct from N-C's
`AcceptedBlock` (cross-use is a type error). The rest are closed
event/effect/error taxonomies + capability records. Exact
whole-project recount belongs to the TRACEABILITY regen that follows.

**Removals: 0** (expected under append-only discipline).

---

## 7. Normative Rule Delta

The project's invariant registry tracks structured rules (TOML), not
prose normative-doc rules; this section reports on it.

- Rules at baseline (`d509f02:constitution_registry.toml`): **147**
- Rules at prior refresh (`2adfb45:docs/ade-invariant-registry.toml`): **196**
- Rules at HEAD (`efe1fb9:docs/ade-invariant-registry.toml`): **202**
- Net additions vs baseline: **+55** (PHASE4-N-A: 2; PHASE4-N-B: 8;
  PHASE4-B1: 6; PHASE4-B2: 5; PHASE4-B3: 2; PHASE4-B3F: 0; PHASE4-B4: 1;
  PHASE4-B5: 1; OQ5: 1; COMMITTEE-CRED / DREP-VOTE / FIDELITY /
  WRITEBACK / post-3d94c22 testkit: 0 each; PHASE4-N-E S1–S5: 2;
  PHASE4-N-E S6: 0; PROPOSAL-PROCEDURES-DECODE: 1; PHASE4-N-C: 14;
  PHASE4-N-G: 6; **PHASE4-N-H: 6** — `CN-CONS-08`, `DC-CONS-19`,
  `DC-CONS-20`, `DC-PROTO-09`, `CN-PROTO-07`, `RO-LIVE-02`, all
  introduced at `declared` in S1 (`b019ee3`); `CN-PROTO-07` flipped
  to `enforced` at S1; `CN-CONS-08` + `DC-CONS-19` flipped to
  `enforced` at S2 (`0ecf22f`); `DC-PROTO-09` flipped to `enforced`
  at S3 (`c584691`); `RO-LIVE-02` set to `partial` at S6 (`efe1fb9`);
  `DC-CONS-20` remains at `declared` with `open_obligation =
  "rollback_side_blocked_until_ledger_snapshot_cluster"` per the
  Path A scope split).
- Net additions vs prior refresh: **+6** — the full N-H 6-rule family.
- Removals: **0** (expected under append-only discipline; clean).

- **Strengthenings recorded by PHASE4-N-H:**
  - **`DC-PROTO-06.strengthened_in += "PHASE4-N-H"`** — the
    mini-protocol session-replay-equivalence rule is strengthened
    by the receive-side client-role transitions (the new receive
    orchestrator drives chain-sync (client) + block-fetch (client)
    per-peer sessions whose transcripts replay byte-identically by
    DC-PROTO-09). `strengthened_in` at HEAD reads
    `["PHASE4-N-A", "PHASE4-N-G", "PHASE4-N-H"]` — N-A introduced
    the client-role transitions, N-G added the symmetric server-role
    transitions, N-H extends to the receive-side client-role session
    composing those transitions through the BLUE reducer.
- **Strengthenings carried forward unchanged**: `CN-CONS-07`
  (PHASE4-N-G — broadcast gate preserved across the network seam);
  `DC-MEM-01` (PHASE4-N-E); `DC-MEM-02` (B2); `DC-EPOCH-01`
  (WRITEBACK + oracle); `DC-LEDGER-10` (OQ5 → COMMITTEE-CRED →
  DREP-VOTE → ENACTMENT-COMMITTEE-FIDELITY → WRITEBACK → oracle →
  PPD cross_ref); `DC-LEDGER-08` (B5); `T-DET-01` / `T-ENC-03`
  (OQ5 + N-C cross_ref); `DC-TXV-06` (B3F); `DC-VAL-06` (B3F + B4);
  `T-CONSERV-01` / `CN-LEDGER-07` (B3); `DC-MEM-01,02` (B2);
  `DC-EPOCH-02` (CE-73); the N-D bundle; the N-A real-capture
  bundle; `T-CORE-02` (S-B1); `T-ENC-01` (N-C `block_body_hash`).

- **Open obligations recorded by PHASE4-N-H:**
  - **`DC-CONS-20.open_obligation = "rollback_side_blocked_until_ledger_snapshot_cluster"`**
    — the rollback-side half of the ChainDb-ledger-chain_dep lockstep
    rule is out of scope for the receive-bridge cluster per the
    Path A scope split. Ledger rollback infrastructure does not yet
    exist (no LedgerState encode/decode, no snapshot+replay-forward
    driver). A follow-on rollback cluster closes the half. Until
    then, the receive bridge returns
    `Err(ReceiveError::RollbackOutOfScope)` on any peer
    `RollBackward`; receive state stays consistent. The rule stays
    at `declared`; admit-side enforcement is recorded under
    `CN-CONS-08` + `DC-CONS-19` + `DC-PROTO-09`.
  - **`RO-LIVE-02.open_obligation = "blocked_until_operator_peer_available"`**
    — the live half of CE-N-H-6 (a cardano-node peer's
    `RollForward` + `BlockDelivered` stream produces a ChainDb tip
    equal to the peer's announced tip at every step over a captured
    follow window) is blocked on a private Haskell peer being
    provisioned. Mechanical pre-condition closed by S5's
    `receive_pipeline_corpus_drive` test (every Conway-576 corpus
    block admits through the full receive pipeline; ChainDb tip
    matches expected `(slot, hash)`; stored bytes equal corpus
    bytes byte-identically; LedgerState fingerprint changes on
    admission). The binary `live_block_follow_session` builds +
    starts in hermetic mode; `--connect` mode requires the private
    Haskell peer. Reopen procedure documented at
    `docs/clusters/completed/PHASE4-N-H/CE-N-H-6_PROCEDURE.md` —
    mirrors the PHASE4-N-C / CE-N-C-8 + PHASE4-N-G / CE-N-G-8 +
    PHASE4-N-E / CE-NODE-N2C-LTX patterns.

Family counts at HEAD: registry total **202** (= 196 + 6 from the
N-H family). The `DC` family grew by 2 (1 CONS + 1 PROTO); the `CN`
family grew by 2 (CONS-08 + PROTO-07); the `RO` family grew by 1
(LIVE-02). Per the constitution coverage gate,
`ci_check_constitution_coverage.sh` PASSES at HEAD with the 6 new
rules' `ci_script` and `tests` arrays populated; rule status at HEAD
breaks down to **74 enforced, 17 partial, 111 declared** across the
202-entry registry.

Normative-doc rule extraction (the `normative_docs` list in
`.idd-config.json`) is approximate and not regenerated here — the
structured registry is the authoritative source.

---

## Anomalies and Cross-Reference Warnings

- **PHASE4-N-H cluster mechanically closed; CE-N-H-6 live half is a
  registry `open_obligation`, and DC-CONS-20 rollback half is a
  Path-A-scope `open_obligation` — neither is a regression.** All 6
  implementing slices (S1 → S6) land their CE — every CE-N-H-1..5 is
  mechanically enforced by a named CI script + named tests. CE-N-H-6
  follows the cluster doc's explicit conditional-closure pattern:
  `RO-LIVE-02.open_obligation = "blocked_until_operator_peer_available"`
  + named blocker (private Haskell peer unavailable at HEAD
  `efe1fb9`) + re-open criteria documented in
  `CE-N-H-6_PROCEDURE.md`. The DC-CONS-20 rollback half is the
  documented Path A scope split: a follow-on ledger-snapshot cluster
  ships ledger rollback infrastructure (encode/decode +
  snapshot+replay-forward driver), at which point the rollback half
  flips to `enforced`. Both follow the documented conditional-closure
  doctrine, not discipline gaps.
- **CODEMAP / SEAMS / TRACEABILITY are stale at this HEAD — expected
  drift between cluster close and grounding ripple.** This regen
  refreshes HEAD_DELTAS only. Prior CODEMAP (`2adfb45`) does NOT
  contain the N-H new submodules (`ade_ledger::receive`,
  `ade_runtime::receive`) or the N-H new binary; prior SEAMS does
  NOT contain the `AdmittedBlock` receive-admission gate seam, the
  BLUE `ChainDbWrite` trait persistence seam, or the
  `PendingHeaderCache` `(slot, block_hash)` coordination seam; prior
  TRACEABILITY does NOT contain the 6 new rules + 5 new
  `ci_script ↔ rule` edges. The grounding ripple immediately
  following this HEAD_DELTAS regen will bring all four docs to
  self-consistency.
- **No new cross-crate dep edge in PHASE4-N-H.** Both new edges
  introduced by PHASE4-N-G (`ade_runtime → ade_network` non-dev, plus
  the four dev-dep edges on `ade_network`) are reused: the receive
  orchestrator dispatches the existing PHASE4-N-A codecs via the N-G
  production edge; the BLUE `receive::*` types are consumed via the
  N-C production edge `ade_runtime → ade_ledger`. No BLUE → RED edge
  was introduced; the cluster's CI gate
  `ci_check_receive_orchestrator_no_producer_dep.sh` explicitly
  forbids the orchestrator from importing `producer::signing` /
  `broadcast` / `scheduler`, preserving receive/producer pipeline
  independence at the key-custody boundary.
- **N-H corpus is the existing Conway-576 corpus** consumed by N-C
  S3/S7, N-G S5/S7, and B1. No new on-disk corpus was added. The
  receive-pipeline corpus drive test drives against the same
  AcceptedBlock arrivals as the N-G server-paths transcript replay.
  The CE-N-H-6 operator-action evidence (a live log against a
  private Haskell peer) is by design out of CI reach and is captured
  as an operator-recorded artifact, mirroring CE-N-C-8, CE-N-G-8,
  and CE-N-E-6.
- **PHASE4-N-H cluster directory IS archived.** Already moved to
  `docs/clusters/completed/PHASE4-N-H/` (8 files: `cluster.md` +
  `N-H-S1.md` through `N-H-S6.md` + `CE-N-H-6_PROCEDURE.md`).
  Planning spillovers live at
  `docs/planning/phase4-n-h-cluster-slice-plan.md` +
  `docs/planning/receive-side-bridge-invariants.md`. Active
  `docs/clusters/` at HEAD now contains only `completed/` and
  `PHASE4-N-B/` (carried-forward stray log directory). This is a
  cleaner end-state than the N-G regen, which surfaced an unarchived
  cluster directory.
- **`strengthened_in` records one strengthening for N-H.**
  `DC-PROTO-06.strengthened_in = ["PHASE4-N-A", "PHASE4-N-G", "PHASE4-N-H"]`
  (receive-side client-role transitions extend the rule's
  session-replay-equivalence scope alongside N-A's client transitions
  and N-G's server transitions). Recorded as a proper
  `strengthened_in` entry, not a cross-ref — continuing the
  canonical pattern N-G normalized.
- **No removed canonical types** (n/a — no separate registry;
  canonical types at HEAD grew by ~12 from the N-H cluster on top
  of N-G's ~10 + N-C's 22 + PPD's 1 since the prior baseline-snapshot
  count).
- **No removed registry rules** (expected: 0; actual: 0). **PHASE4-N-H
  added 6 new rules.** Registry total: **202** at HEAD (was 196 at
  prior refresh).
- **All commit subjects in this regen carry a conventional-commits
  prefix.** The 6 PHASE4-N-H commits are `feat(ledger)` ×2,
  `feat(runtime)` ×2, `test(runtime)` ×1, `feat(interop)` ×1. **All
  6 commits in the `2adfb45..efe1fb9` span carry the repo-required
  `Co-Authored-By: Claude Opus 4.7 (1M context)` model-attribution
  trailer** (per the CLAUDE.md project override for the bounty
  trailer ratio). The project hook `ci/git-hooks/commit-msg` is
  active in this clone and enforces the trailer mechanically.
- **Cluster docs archived as of this HEAD.** Seventeen cluster
  directories archived under `docs/clusters/completed/`:
  COMMITTEE-CRED-FIDELITY, DREP-VOTE-FIDELITY,
  ENACTMENT-COMMITTEE-FIDELITY, ENACTMENT-COMMITTEE-WRITEBACK,
  OQ5-CREDENTIAL-FIDELITY, PHASE4-B1, PHASE4-B2, PHASE4-B3,
  PHASE4-B3F, PHASE4-B4, PHASE4-B5, PHASE4-N-A, PHASE4-N-B,
  PHASE4-N-C, PHASE4-N-D, PHASE4-N-E, PHASE4-N-G, PHASE4-N-H,
  PROPOSAL-PROCEDURES-DECODE.
- **B5 / B4 / B3F / B3 / B2 / B1 / N-D / N-B / N-A / PHASE4-N-E /
  PROPOSAL-PROCEDURES-DECODE / PHASE4-N-C / PHASE4-N-G closures —
  carried forward unchanged.**
- **Pre-existing `boundary_fingerprint_matches_pins` failure on
  `byron_pre_hfc` predates this cluster.** Out-of-scope for
  PHASE4-N-H; not introduced by any N-H slice. Tracked under a
  separate future cluster.
- **DC-VAL status mismatch vs. closure claim (B1, carried forward).**
  Only `DC-VAL-01` is `enforced`; `DC-VAL-02` → `DC-VAL-05` remain
  `declared` despite named tests and the extended closed-enums
  enforcement point. Flip on the next `/traceability` pass.
- **Adversarial corpora are derived, not committed (carried forward).**
  N-E reuses the B2 B-track corpus verbatim; PPD PP-S2 ships its
  corpus in code; PHASE4-N-C ships its corpus in code; PHASE4-N-G
  + **PHASE4-N-H** both reuse the existing Conway-576 corpus
  (no new corpus). The corpus pattern continues the
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
phase boundary (Phase 4 close, which PHASE4-N-H brings further into
reach: the symmetric receive-side bridge is mechanically closed
admit-only; the remaining Phase-4 closure work is operator-action
live evidence for CE-N-C-8 / CE-N-G-8 / CE-N-H-6 / CE-NODE-N2C-LTX,
the rollback half of DC-CONS-20 via a follow-on ledger-snapshot
cluster, the N-F operator surface, and the OP-OPS-04 Sum6KES skey
loader gap). Note the commit-hash rewrite caveat at the top —
re-derive hashes from `git log` at each regen rather than carrying
them forward. This regen is cut at HEAD `efe1fb9` (PHASE4-N-H S6).
The prior regen narrated HEAD `a280954` (PHASE4-N-G S7, archived at
`2adfb45`); the new span is `2adfb45..efe1fb9` — 6 commits:
`b019ee3` (S1 BLUE AdmittedBlock token + receive closed sums +
PendingHeaderCache + ChainDbWrite), `0ecf22f` (S2 BLUE receive_apply
reducer composing block_validity), `c584691` (S3 GREEN
events_to_state + in_memory_chain_write + transcript replay),
`1d06089` (S4 RED N2N receive orchestrator), `3973261` (S5
mechanical cross-impl receive pipeline drive), `efe1fb9` (S6
live_block_follow_session + CE-N-H-6 procedure).
