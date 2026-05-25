# HEAD Deltas — Ade

> **Status:** Living architectural document. Regenerated; not hand-edited.
> Regenerate via `/head-deltas <baseline>`. Baseline is declared in
> `.idd-config.json` (`head_deltas_baseline`).

> Baseline: `d509f02` (Phase 3 handoff snapshot, 2026-04-15)
> HEAD: `a280954` (feat(interop): mechanical cross-impl + live_block_fetch_session (PHASE4-N-G S7), 2026-05-25)
> 163 commits, 11,395 files changed, +197,640 / −7,233,633 lines

Headline numbers note: the massive negative line count is dominated by
the **corpus relayout** under `corpus/snapshots/` and the deletion of
the multi-MB credentialed-snapshot text files
(`*_registered_creds.txt`, ~7M lines combined). Source-tree deltas are
far smaller — the per-crate breakdown in §3 is the representative view.

> **Commit-hash note.** This regen runs against the current (rebased)
> history. Earlier HEAD_DELTAS regens referenced commit hashes from a
> history that has since been rewritten; all hashes below are verbatim
> from `git log d509f02..HEAD` at this HEAD.

> **PHASE4-N-G cluster close note (newest thread).** This regen is cut
> at HEAD `a280954`. Since the prior grounding-doc refresh `df56e2d`
> (which closed PHASE4-N-C — archive cluster + refresh
> CODEMAP/SEAMS/HEAD_DELTAS/TRACEABILITY), **seven new commits have
> landed** — the **PHASE4-N-G cluster** (S1 → S7) closing the
> producer-side block-fetch + chain-sync server response paths (the
> "engineering bridge" between N-C's broadcast-queue output and a real
> Haskell cardano-node peer's RequestRange / RequestNext). Sequence:
> `8cd17c9` (S1, **BLUE foundation** — `accepted_block_header_bytes` lifted
> as the single canonical header projection in
> `ade_ledger::block_validity::header_input` (~29 LOC delta, reuses
> existing `header_cbor_slice` walker); closed `ServerReply<M>`
> wrappers in new files `ade_network::chain_sync::server` and
> `ade_network::block_fetch::server` whose private inner enums host
> only server-agency variants (RollForward / RollBackward / AwaitReply /
> IntersectFound / IntersectNotFound for chain-sync; StartBatch /
> NoBlocks / Block / BatchDone for block-fetch); new CI gate
> `ci_check_no_parallel_header_splitter.sh`; **6 new registry rules
> appended at `declared`** — `DC-CONS-17/18`, `DC-PROTO-07/08`,
> `CN-PROTO-06`, `RO-LIVE-01` (190 → 196); `CN-PROTO-06` flipped to
> `enforced` at S1). `dc069cf` (S2, **BLUE** — new module
> `ade_ledger::producer::served_chain` (~171 LOC): `ServedChainSnapshot`
> + `served_chain_admit` BTreeMap-indexed append-only chain index; the
> only path bytes enter is via an `AcceptedBlock` token (broadcast-gate
> preservation across the network seam, strengthens `CN-CONS-07`); key
> derived from bytes via `decode_block` (no caller-supplied "asserted
> hash"); admission-order-independent `fingerprint()` over BTreeMap
> traversal; new CI gate `ci_check_served_chain_closure.sh` forbids
> `HashMap`/`HashSet`/wall-clock/rand in the file and forbids an
> asserted-hash admit parameter). `cc49b1d` (S3, **BLUE** — pure
> per-session reducers in new file
> `ade_network::chain_sync::server.rs` (~804 LOC):
> `producer_chain_sync_serve` + `producer_chain_sync_advance_tip`
> compose the N-A `chain_sync_transition` for grammar validation (no
> parallel state machine); RollForward header bytes sourced via the
> `ServedHeaderLookup` trait whose canonical impl forwards
> `accepted_block_header_bytes` (DC-CONS-18); deterministic-resolution
> discipline per DC-PROTO-08 — every server-agency state returns one
> legal reply / structured close / error; new CI gate
> `ci_check_chain_sync_server_closure.sh`; **DC-PROTO-08 flipped to
> `enforced`**; trait-bound seam keeps `ade_network` → `ade_ledger` out
> of production deps — `ade_ledger`, `ade_testkit`, `ade_core`,
> `ade_crypto` added as dev-dependencies for tests). `03d120f` (S4,
> **BLUE** — pure reducer in new file
> `ade_network::block_fetch::server.rs` (~596 LOC):
> `producer_block_fetch_serve` — RequestRange{Block,Block} →
> [StartBatch, Block{bytes}*, BatchDone] sourced via the
> `ServedRangeLookup` trait that yields AcceptedBlock-derived slices
> verbatim (DC-CONS-17 foundation: reducer never re-encodes);
> RequestRange covering genesis Origin → NoBlocks; ClientDone → Done;
> grammar reject on server-originated message from client agency; new
> CI gate `ci_check_block_fetch_server_closure.sh`; **DC-PROTO-07
> flipped to `enforced`**). `1a1b8e0` (S5, **GREEN** — closes three
> N-G invariants by wiring BLUE reducers end-to-end through pure
> GREEN glue: new module `ade_runtime::producer::broadcast_to_served`
> (~188 LOC): `drain_and_admit(BroadcastQueue, ServedChainSnapshot)
> -> (ServedChainSnapshot, BroadcastQueue, Vec<AcceptedBlock>)` — pure
> adapter, no I/O, observably deterministic over arrival sequences;
> new module `ade_runtime::producer::served_chain_lookups` (~120 LOC):
> `ServedChainLookups` borrow-wrapper implementing both
> `ServedHeaderLookup` and `ServedRangeLookup`, header projection
> through the canonical `accepted_block_header_bytes` (no parallel
> splitter); `ServedChainSnapshot::{iter_accepted, block_at}` extension
> (+ ~237 LOC in `self_accept.rs` re-exposes accessors so the GREEN
> adapter can call the canonical splitter directly); end-to-end
> transcript replay test `tests/server_paths_transcript_replay.rs`
> (~310 LOC) including `session_transcript_replay_byte_identical`
> (DC-PROTO-07 evidence) +
> `session_transcript_served_block_bytes_equal_admitted_accepted_block_bytes`
> (DC-CONS-17 evidence) +
> `session_transcript_announced_header_matches_served_body_recipe`
> (DC-CONS-18 evidence); new CI gate
> `ci_check_broadcast_to_served_purity.sh`; **DC-CONS-17 +
> DC-CONS-18 flipped to `enforced`**). `f773b1c` (S6, **RED** — new
> module `ade_runtime::network::n2n_server` (~216 LOC) is the pure
> per-peer session driver composing S5 GREEN + S3/S4 BLUE:
> `PerPeerN2nServerState::new(cs_v, bf_v)`, `dispatch_chain_sync_frame`,
> `dispatch_block_fetch_frame`, `poll_chain_sync_advance` — decode →
> serve → encode (no sockets; tokio bridge is operator-action work
> per S7); per-peer state is independent so OQ-4 multi-peer
> determinism holds against a shared `&ServedChainSnapshot` (proven
> by `tests/n2n_server_two_peer_determinism.rs`, ~167 LOC); key
> boundary doctrine: `n2n_server` MUST NOT import from
> `crate::producer::signing` — enforced by new CI gate
> `ci_check_n2n_server_no_signing_dep.sh`; **new production dep edge
> `ade_runtime → ade_network`** (non-dev; required so the GREEN
> adapter / trait impls can host the BLUE reducer interface
> bindings)). `a280954` (S7, **mechanical cross-impl + operator-action
> evidence** — new
> `crates/ade_runtime/tests/cross_impl_server_pipeline.rs` (~193 LOC,
> CE-N-G-7 mechanical adapter) drives the full S5 pipeline against
> the Conway-576 corpus AcceptedBlock arrivals and asserts every
> served Block{bytes} decodes via Ade's envelope + block decoders,
> body-hash binding re-runs cleanly, and bytes are byte-identical to
> the corpus block the operator fed `self_accept`; new **RED** binary
> `ade_core_interop::live_block_fetch_session` (~141 LOC, CE-N-G-8
> operator-action evidence) modeled on `live_block_production_session`:
> hermetic-default + `--connect` stub; build-and-start test
> `crates/ade_core_interop/tests/live_block_fetch_session.rs`
> (~61 LOC); operator procedure doc
> `docs/clusters/PHASE4-N-G/CE-N-G-8_PROCEDURE.md`; new CI gate
> `ci_check_server_paths_corpus_present.sh`; **RO-LIVE-01 set to
> `partial`** with `open_obligation =
> "blocked_until_operator_peer_available"` — the mechanical
> pre-condition (bytes round-trip via Ade's own decoder + body-hash
> recipe) is closed by the cross-impl adapter, the live half is
> blocked on a private Haskell peer per the documented
> conditional-closure pattern; `CN-CONS-07.strengthened_in += PHASE4-N-G`
> (broadcast gate preserved across the network seam); `DC-PROTO-06.strengthened_in
> += PHASE4-N-G`). **Three new BLUE submodules** (`ade_ledger::producer::served_chain`,
> `ade_network::chain_sync::server`, `ade_network::block_fetch::server`),
> **two new GREEN files** (`ade_runtime::producer::broadcast_to_served`,
> `ade_runtime::producer::served_chain_lookups`), **two new RED files**
> (`ade_runtime::network::mod` + `ade_runtime::network::n2n_server`),
> **one new RED binary** (`live_block_fetch_session`), **6 new registry
> rules** (all `cluster = "PHASE4-N-G"` and all `enforced` except
> `RO-LIVE-01` at `partial`: `DC-CONS-17/18`, `DC-PROTO-07/08`,
> `CN-PROTO-06`, `RO-LIVE-01`), **2 strengthenings** (`CN-CONS-07`,
> `DC-PROTO-06`), **7 new CI scripts** (the 41st → 47th:
> `ci_check_no_parallel_header_splitter.sh`,
> `ci_check_served_chain_closure.sh`,
> `ci_check_chain_sync_server_closure.sh`,
> `ci_check_block_fetch_server_closure.sh`,
> `ci_check_broadcast_to_served_purity.sh`,
> `ci_check_n2n_server_no_signing_dep.sh`,
> `ci_check_server_paths_corpus_present.sh`), **one new non-dev Cargo
> dep edge** (`ade_runtime → ade_network`, S6) **+ four new dev-dep
> edges on `ade_network`** (`ade_ledger`, `ade_testkit`, `ade_core`,
> `ade_crypto`, all S3) so the trait-bound seam keeps
> `ade_network → ade_ledger` out of production deps. **One
> `open_obligation` entry recorded**: `RO-LIVE-01` (Haskell peer
> live evidence — `blocked_until_operator_peer_available`).
> **Cluster status at HEAD: closed mechanically; CE-N-G-8 live half
> open as a registry obligation**, mirroring the N-C / N-E pattern.
> Cluster directory remains at `docs/clusters/PHASE4-N-G/` (10 files:
> `cluster.md` + N-G-S1..S7 + `CE-N-G-8_PROCEDURE.md` + planning
> spillovers under `docs/planning/`); archival to
> `docs/clusters/completed/PHASE4-N-G/` is deferred to a separate
> `/cluster-close` commit. **No CODEMAP/SEAMS/TRACEABILITY refresh
> yet** for the N-G cluster — those three docs are stale relative to
> this HEAD_DELTAS regen and must be regenerated in the grounding
> ripple immediately following.

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
> PROPOSAL-PROCEDURES-DECODE / PHASE4-N-C cluster notes (carried
> forward).** All closed and archived at
> `docs/clusters/completed/<NAME>/`.

The delta now covers twenty-seven threads of work. The newest thread —
the **PHASE4-N-G cluster** (`8cd17c9` → `a280954`, 7 commits) — sits
on the post-N-C grounding refresh `df56e2d`, which closed +
archived PHASE4-N-C. In rough proportion of the substantive change
budget:

0. **PHASE4-N-G (producer-side block-fetch + chain-sync server
   response paths — the engineering bridge between N-C's
   broadcast-queue output and a real Haskell cardano-node peer's
   RequestRange / RequestNext) — closed in 7 slices.** S1
   (`8cd17c9`, **BLUE**) lifts `accepted_block_header_bytes` as the
   single canonical header projection in
   `ade_ledger::block_validity::header_input` (reusing the validator's
   existing `header_cbor_slice` walker — the same recipe the
   body-hash binding hashes) and introduces closed `ServerReply<M>`
   wrappers in two new files `ade_network::chain_sync::server` /
   `ade_network::block_fetch::server` whose private inner enums host
   only server-agency variants. Attempting to construct a
   client-originated variant from the server pump is a compile error
   against undefined items (CN-PROTO-06 type-level closure). New CI
   gate `ci_check_no_parallel_header_splitter.sh` (~86 LOC) forbids
   any new `pub fn .*header_bytes` outside the canonical site and
   positively asserts the canonical accessor exists. **Registry**:
   6 rules appended at `declared` — `DC-CONS-17/18`, `DC-PROTO-07/08`,
   `CN-PROTO-06`, `RO-LIVE-01` (190 → 196); `CN-PROTO-06` flipped to
   `enforced` (closure-gated by the no-parallel-splitter check + 7
   named tests across ServerReply round-trips and header-projection
   self-consistency).
   S2 (`dc069cf`, **BLUE**) introduces the single canonical chain
   index for N-G's server reducers — new module
   `ade_ledger::producer::served_chain` (~171 LOC): `ServedChainSnapshot`
   + `served_chain_admit` are BTreeMap-backed, append-only,
   deterministic; the key `(slot, blake2b_256(header))` is derived
   from the bytes via `decode_block` — there is no caller-supplied
   "asserted hash" parameter. The broadcast gate (`CN-CONS-07`) is
   preserved across the network seam: bytes enter the served chain
   *only* via an `AcceptedBlock` token, which only `self_accept`
   returning `Ok` produces. Accessors: `block_bytes(slot, &hash)`
   (DC-CONS-17 foundation — returns AcceptedBlock slice
   byte-identically), `range_bytes(from, to)` (S4's RequestRange
   source), `iter()` (BTreeMap order), `fingerprint()`
   (admission-order-independent replay anchor). New CI gate
   `ci_check_served_chain_closure.sh` (~81 LOC) enforces no
   `HashMap`/`HashSet`/`std::collections::Hash*`, no
   wall-clock/rand/tokio::time, and a positive check that admit's
   signature has no asserted-hash parameter.
   S3 (`cc49b1d`, **BLUE**) introduces the pure chain-sync server
   reducers in new file `ade_network::chain_sync::server.rs`
   (~804 LOC): `producer_chain_sync_serve(state, in_msg, &served,
   version)` processes one client-originated message — RequestNext
   → RollForward (if a fresh block sits past the cursor) or
   AwaitReply (parks in MustReply); FindIntersect → IntersectFound
   or IntersectNotFound by scanning the served chain; Done →
   `ServerStep::Done`; grammar violation → `Err(ProducerServerError::Grammar(_))`.
   `producer_chain_sync_advance_tip(state, &served)` is polled by
   the orchestrator after broadcast-queue admission — emits a
   deferred RollForward from CanAwait / MustReply when fresh data
   appears. Composes the PHASE4-N-A `chain_sync_transition` for
   grammar validation (no parallel state machine). RollForward
   header bytes flow through the `ServedHeaderLookup` trait whose
   canonical impl forwards `accepted_block_header_bytes`
   (DC-CONS-18). Deterministic-resolution discipline (DC-PROTO-08):
   every server-agency state returns one of legal RollForward /
   RollBackward / AwaitReply / structured close-or-error — no
   ambiguous silent wait. Trait-bound seam keeps `ade_network →
   ade_ledger` out of production deps; `ade_ledger`, `ade_testkit`,
   `ade_core`, `ade_crypto` are added as dev-dependencies for tests.
   New CI gate `ci_check_chain_sync_server_closure.sh`. Registry:
   `DC-PROTO-08` flipped to `enforced`.
   S4 (`03d120f`, **BLUE**) introduces the pure block-fetch server
   reducer in new file `ade_network::block_fetch::server.rs`
   (~596 LOC): `producer_block_fetch_serve(state, in_msg, &served,
   version)` — RequestRange{Block,Block} → look up
   `served.range_bytes`; if non-empty emit [StartBatch,
   Block{bytes}*, BatchDone]; if empty emit [NoBlocks].
   RequestRange covering genesis Origin → narrow scope: emit
   [NoBlocks] (the producer does not serve genesis). ClientDone →
   `BlockFetchServerStep::Done`. Server-originated message from
   client agency → grammar reject. Every Block{bytes} payload
   sources from `served.range_bytes()` — which returns
   AcceptedBlock-derived slices verbatim via `ServedChainSnapshot`.
   DC-CONS-17 enforcement foundation: the reducer never re-encodes;
   bytes flow verbatim from AcceptedBlock through the served-chain
   index out to the wire. Trait-bound seam (`ServedRangeLookup`)
   mirrors S3. New CI gate `ci_check_block_fetch_server_closure.sh`.
   Registry: `DC-PROTO-07` flipped to `enforced`.
   S5 (`1a1b8e0`, **GREEN**) closes three N-G invariants by wiring
   the BLUE reducers end-to-end through pure GREEN glue. New module
   `ade_runtime::producer::broadcast_to_served` (~188 LOC):
   `drain_and_admit(BroadcastQueue, ServedChainSnapshot) ->
   (ServedChainSnapshot, BroadcastQueue, Vec<AcceptedBlock>)` is the
   pure adapter — no I/O, no clock, observably deterministic over
   arrival sequences. New module
   `ade_runtime::producer::served_chain_lookups` (~120 LOC):
   `ServedChainLookups` borrow-wrapper implementing both
   `ServedHeaderLookup` and `ServedRangeLookup`; the header
   projection goes through the canonical `accepted_block_header_bytes`
   (no parallel splitter). `ServedChainSnapshot::{iter_accepted,
   block_at}` extension (in `served_chain.rs` + a ~237-LOC
   `self_accept.rs` re-expose) lets the GREEN adapter call the
   canonical splitter directly — replaces an earlier inline-walker
   approach that would have tripped the no-parallel-splitter gate.
   End-to-end test `tests/server_paths_transcript_replay.rs`
   (~310 LOC): `session_transcript_replay_byte_identical` (DC-PROTO-07
   evidence — two runs over identical canonical inputs produce
   identical outgoing frames),
   `session_transcript_served_block_bytes_equal_admitted_accepted_block_bytes`
   (DC-CONS-17 evidence — every Block{bytes} equals
   `AcceptedBlock.as_bytes()` at the matching (slot,hash)), and
   `session_transcript_announced_header_matches_served_body_recipe`
   (DC-CONS-18 evidence — `block_body_hash` applied to the served
   body equals the announced header's body_hash field). New CI gate
   `ci_check_broadcast_to_served_purity.sh`. Registry: `DC-CONS-17`
   + `DC-CONS-18` flipped to `enforced`.
   S6 (`f773b1c`, **RED**) introduces the per-peer session driver —
   new module `ade_runtime::network::n2n_server` (~216 LOC) is the
   pure decode → serve → encode driver composing S5 GREEN + S3/S4
   BLUE: `PerPeerN2nServerState::new(cs_v, bf_v)` (independent
   per-peer state holding both reducer states + handshake-negotiated
   versions), `dispatch_chain_sync_frame(state, frame, &snap)`,
   `dispatch_block_fetch_frame(state, frame, &snap)` (returns
   `Vec<frame>` since RequestRange yields a multi-frame batch),
   `poll_chain_sync_advance(state, &snap)` (drains a deferred
   RollForward after broadcast-queue admission). No socket I/O —
   S7's evidence binary plugs this into tokio. Multi-peer
   determinism (OQ-4): per-peer state is independent; cross-peer
   coordination only via `&ServedChainSnapshot`. Integration test
   `tests/n2n_server_two_peer_determinism.rs` (~167 LOC) drives two
   synthetic peers in parallel against one shared snapshot and
   asserts each peer's transcript equals its solo run — and equals
   the other peer's (same inputs, same outputs). Key-boundary
   doctrine: `n2n_server` modules MUST NOT import from
   `crate::producer::signing`. New CI gate
   `ci_check_n2n_server_no_signing_dep.sh`. **New non-dev Cargo dep
   edge**: `ade_runtime → ade_network` (was absent entirely — the
   GREEN adapter / trait impls now host the BLUE reducer interface
   bindings). Cargo.lock change folded into this commit.
   S7 (`a280954`, **mechanical cross-impl + operator-action evidence**)
   ships the cluster's bounty-facing surface. New
   `crates/ade_runtime/tests/cross_impl_server_pipeline.rs` (~193
   LOC, CE-N-G-7) drives the full S5 pipeline against the Conway-576
   corpus AcceptedBlock arrivals and asserts every served
   Block{bytes} decodes via Ade's envelope + block decoders,
   body-hash binding re-runs cleanly, and bytes are byte-identical
   to the corpus block the operator fed `self_accept`. Independent
   of any external Haskell peer — the mechanical pre-condition
   proving the bytes will be validator-acceptable. New **RED**
   binary `ade_core_interop::live_block_fetch_session` (~141 LOC,
   CE-N-G-8 operator-action evidence) mirrors
   `live_block_production_session`'s pattern: hermetic default
   prints a readiness banner and exits 0; `--connect` mode prints
   the tokio-bridge wiring stub (the socket bridge is operator-action
   work — `n2n_server` is the pure driver). Build-and-start test
   `crates/ade_core_interop/tests/live_block_fetch_session.rs`
   (~61 LOC). Operator procedure
   `docs/clusters/PHASE4-N-G/CE-N-G-8_PROCEDURE.md` mirrors N-C's
   `CE-N-C-8_PROCEDURE.md`. New CI gate
   `ci_check_server_paths_corpus_present.sh`. Registry:
   **`RO-LIVE-01` set to `partial`** with `open_obligation =
   "blocked_until_operator_peer_available"` — the mechanical
   pre-condition is closed by the cross-impl adapter; the live half
   is blocked on a private Haskell peer per the documented
   conditional-closure pattern (mirrors N-C `CN-CONS-06`, N-E
   `CE-NODE-N2C-LTX`). `CN-CONS-07.strengthened_in += PHASE4-N-G`
   (broadcast gate preserved across the network seam);
   `DC-PROTO-06.strengthened_in += PHASE4-N-G` (the new server-role
   transitions extend DC-PROTO-06's scope). **No new dep edge from
   S7** — `ade_core_interop` already had its `ade_ledger` /
   `ade_network` edges from N-E S4 and the workspace.
1. **PHASE4-N-C (last Tier-1 bounty deliverable — block-production
   authority) — closed at HEAD `694dd74`, archived at `df56e2d`.**
   New BLUE submodule `ade_ledger::producer`, new BLUE module
   `ade_ledger::block_body_hash`, new BLUE module pair
   `ade_core::consensus::opcert_validate` +
   `ade_codec::shelley::opcert`, new RED submodule
   `ade_runtime::producer` + GREEN `tick_assembler`, new RED binary
   `live_block_production_session`. 14 new rules; 8 new CI scripts;
   one new dep edge (`ade_runtime → ade_ledger`).
2. **PROPOSAL-PROCEDURES-DECODE (last open governance-domain decode
   seam) — closed in 2 slices.**
3. **PHASE4-N-E S6 (live N2N tx-submission2 evidence binary) —
   cluster close.**
4. **PHASE4-N-E S1–S5 (wire-level mempool ingress, Tier 1).**
5. **Post-WRITEBACK testkit follow-ups (four commits, GREEN-scope).**
6. **ENACTMENT-COMMITTEE-WRITEBACK — closed.**
7. **ENACTMENT-COMMITTEE-FIDELITY — closed.**
8. **DREP-VOTE-FIDELITY — closed.**
9. **COMMITTEE-CRED-FIDELITY — closed.**
10. **OQ5-CREDENTIAL-FIDELITY — closed.**
11. **Phase 4 cluster B5 (Conway gov-cert accumulation) — closed.**
12. **Phase 4 cluster B4 (Conway cert-state accumulation,
    fail-closed) — closed.**
13. **Phase 4 cluster B3F (follow-up hardening) — committed.**
14. **Phase 4 cluster B3 (Conway value-conservation accounting) —
    closed.**
15. **Phase 4 cluster B2 (tx validity agreement) — closed.**
16. **Phase 4 cluster B1 (full block validity agreement) — closed.**
17. **Phase 4 cluster N-A (network mini-protocols) — closed.**
18. **Phase 4 cluster N-B (consensus runtime) — closed.**
19. **CE-N-B-6 follow-mode bridge.**
20. **Phase 4 cluster N-D (ChainDB persistence) — closed.**
21. **Phase 2C close-out / CE-73 reclassification.**
22. **IDD canonicalization.**
23. **Grounding-doc generation + ripple.** Successive refreshes,
    including `52642e5`, `350130e`, `3af9e2b`, `96d043c`, `df56e2d`.
24. **BLUE-list drift closure.** Six CI scripts extended to full
    BLUE scope.
25. **Corpus relayout.** Credentialed `*_registered_creds.txt`
    removed (~7M-line negative); `corpus/snapshots/` now
    `.gitignore`-d.

---

## 1. Commit Log

| Hash | Type | Summary |
|------|------|---------|
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
| `ade_ledger::producer::served_chain` (new file in an existing BLUE crate) | BLUE | **Single canonical append-only chain index** from which N-G's server reducers source wire bytes. `ServedChainSnapshot` (BTreeMap-backed, deterministic) + `served_chain_admit(snapshot, AcceptedBlock) -> Result<(ServedChainSnapshot, ServedAdmitOutcome), ServedChainError>` — key `(slot, blake2b_256(header))` is derived from the bytes via `decode_block`; there is no caller-supplied "asserted hash" parameter. The broadcast gate (CN-CONS-07) is preserved across the network seam: the only path bytes enter the served chain is via an `AcceptedBlock` token, which only `self_accept` returning `Ok` produces. Accessors: `block_bytes(slot, &hash)` (point lookup; DC-CONS-17 foundation), `range_bytes(from, to)` (inclusive BTreeMap range; S4's RequestRange source), `iter()` (BTreeMap order), `iter_accepted` / `block_at` (S5 extensions — expose `&AcceptedBlock` so the GREEN adapter can call `accepted_block_header_bytes` directly), `fingerprint()` (blake2b_256 over `(slot_be8 \|\| hash \|\| bytes_len \|\| bytes)` triples in BTreeMap order — admission-order-independent replay anchor). Closed `ServedChainError::{Decode, KeyByteConflict}`. Enforced by `ci_check_served_chain_closure.sh`. | `producer/served_chain.rs` (~171 LOC) | PHASE4-N-G / S2 (`dc069cf`); S5 extension (`1a1b8e0`) |
| `ade_network::chain_sync::server` (new file in an existing BLUE-scoped network submodule) | BLUE | **Pure chain-sync server-pump reducers.** `producer_chain_sync_serve(state, in_msg, &served, version)` processes one client-originated message; `producer_chain_sync_advance_tip(state, &served)` is polled by the orchestrator after broadcast-queue admission. Composes the PHASE4-N-A `chain_sync_transition` for grammar validation (no parallel state machine). Header bytes in any RollForward come from the canonical `accepted_block_header_bytes` (DC-CONS-18) via the `ServedHeaderLookup` trait. Deterministic-resolution discipline (DC-PROTO-08): every server-agency state returns one of legal RollForward / RollBackward / AwaitReply / structured close-or-error — no ambiguous silent wait. Closed `ServerReply<ChainSyncMessage>` whose private inner enum carries only server-agency variants (RollForward / RollBackward / AwaitReply / IntersectFound / IntersectNotFound). Closed `ProducerServerError`. Trait-bound seam keeps `ade_network → ade_ledger` out of production deps. Enforced by `ci_check_chain_sync_server_closure.sh` + `ci_check_no_parallel_header_splitter.sh`. | `chain_sync/server.rs` (~804 LOC); registered in `chain_sync/mod.rs` | PHASE4-N-G / S1 (`8cd17c9`, ServerReply wrapper); S3 (`cc49b1d`, reducer) |
| `ade_network::block_fetch::server` (new file in an existing BLUE-scoped network submodule) | BLUE | **Pure block-fetch server-pump reducer.** `producer_block_fetch_serve(state, in_msg, &served, version)` — RequestRange{Block,Block} → look up `served.range_bytes`; if non-empty emit [StartBatch, Block{bytes}*, BatchDone]; if empty emit [NoBlocks]. RequestRange covering genesis Origin → [NoBlocks] (the producer does not serve genesis). ClientDone → `BlockFetchServerStep::Done`. Server-originated message from client agency → grammar reject. **Every Block{bytes} payload sources from `served.range_bytes()`** — which returns AcceptedBlock-derived slices verbatim via `ServedChainSnapshot`. DC-CONS-17 enforcement foundation: the reducer never re-encodes; bytes flow verbatim from AcceptedBlock through the served-chain index out to the wire. Closed `ServerReply<BlockFetchMessage>` whose private inner enum carries only server-agency variants (StartBatch / NoBlocks / Block / BatchDone). Trait-bound seam (`ServedRangeLookup`) mirrors S3. Enforced by `ci_check_block_fetch_server_closure.sh`. | `block_fetch/server.rs` (~596 LOC); registered in `block_fetch/mod.rs` | PHASE4-N-G / S1 (`8cd17c9`, ServerReply wrapper); S4 (`03d120f`, reducer) |
| `ade_runtime::producer::broadcast_to_served` (new file in an existing RED crate) | GREEN | **Pure adapter draining a BroadcastQueue and admitting each AcceptedBlock into a ServedChainSnapshot.** `drain_and_admit(BroadcastQueue, ServedChainSnapshot) -> (ServedChainSnapshot, BroadcastQueue, Vec<AcceptedBlock>)` is pure — no I/O, no clock, no rand, observably deterministic over arrival sequences. Bridges the RED scheduler / broadcast outputs into the BLUE server-pump input shape. Enforced by `ci_check_broadcast_to_served_purity.sh`. | `producer/broadcast_to_served.rs` (~188 LOC) | PHASE4-N-G / S5 (`1a1b8e0`) |
| `ade_runtime::producer::served_chain_lookups` (new file in an existing RED crate) | GREEN | **Borrow-wrapper around `ServedChainSnapshot` implementing both `ServedHeaderLookup` (chain-sync) and `ServedRangeLookup` (block-fetch).** The header projection goes through the canonical `accepted_block_header_bytes` (DC-CONS-16 / DC-CONS-18 — no parallel splitter). Pure projection — no I/O. Enforced by `ci_check_broadcast_to_served_purity.sh` (positive grep on the canonical import) + `ci_check_no_parallel_header_splitter.sh`. | `producer/served_chain_lookups.rs` (~120 LOC) | PHASE4-N-G / S5 (`1a1b8e0`) |
| `ade_runtime::network::n2n_server` (new file in a new RED submodule) | RED | **Pure per-peer N2N server-role session driver composing S5 GREEN + S3/S4 BLUE.** Decodes inbound mini-protocol frames, runs the reducers, encodes outgoing frames. No socket I/O — S7's evidence binary plugs this into tokio. Surface: `PerPeerN2nServerState::new(cs_v, bf_v)` (independent per-peer state holding both reducer states + handshake-negotiated versions), `dispatch_chain_sync_frame(state, frame, &snap)`, `dispatch_block_fetch_frame(state, frame, &snap)` (returns `Vec<frame>` since RequestRange yields a multi-frame batch), `poll_chain_sync_advance(state, &snap)` (drains a deferred RollForward after broadcast-queue admission). Multi-peer determinism (OQ-4): per-peer state is independent; cross-peer coordination only via `&ServedChainSnapshot` (proven by `tests/n2n_server_two_peer_determinism.rs`). Key-boundary doctrine: MUST NOT import from `crate::producer::signing`. Enforced by `ci_check_n2n_server_no_signing_dep.sh`. | `network/mod.rs`, `network/n2n_server.rs` (~216 LOC) | PHASE4-N-G / S6 (`f773b1c`) |
| `ade_core_interop` bin `live_block_fetch_session` (new RED binary in an existing RED crate) | RED | **Operator-action live-evidence probe for CE-N-G-8 / RO-LIVE-01.** Mirrors `live_block_production_session` and `live_tx_submission_session`. Hermetic default mode prints a readiness banner and exits 0 (no sockets, no operator material read); `--connect` mode prints the wiring stub (the tokio socket bridge to `n2n_server` is operator-action work; the `n2n_server` module itself is the pure driver). Args: `--network`, `--magic`, `--target`, `--out`. Captures `docs/clusters/PHASE4-N-G/CE-N-G-LIVE_<date>.log` (operator-recorded). **Conditional on private Haskell peer availability**: at HEAD, `RO-LIVE-01.status = "partial"` with `open_obligation = "blocked_until_operator_peer_available"`. Build-and-start test asserts hermetic-mode banner; the byte-shape claim is closed by S7's mechanical `cross_impl_server_pipeline` test against the Conway-576 corpus. | `src/bin/live_block_fetch_session.rs` (~141 LOC); `[[bin]]` entry in `crates/ade_core_interop/Cargo.toml`; operator procedure at `docs/clusters/PHASE4-N-G/CE-N-G-8_PROCEDURE.md` | PHASE4-N-G / S7 (`a280954`) |
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
| `ade_network` (new workspace crate) | BLUE-majority (per-submodule scoped) | Ouroboros mini-protocol authority. **N-G S1 / S3 / S4 added the new server-side files** `chain_sync/server.rs` + `block_fetch/server.rs` + their ServerReply wrappers; the rest is unchanged. | `codec/`, `handshake/`, `chain_sync/`, `block_fetch/`, `tx_submission/`, `keep_alive/`, `peer_sharing/`, `n2c/`, `mux/frame.rs` (BLUE), `mux/transport.rs` (RED), `session/` (RED), `chain_sync/server.rs` (**N-G S1/S3**), `block_fetch/server.rs` (**N-G S1/S4**) | PHASE4-N-A; PHASE4-N-G |
| `ade_core::consensus` (new submodule of an existing BLUE crate) | BLUE | Praos consensus authority. | `mod.rs`, `era_schedule.rs`, `header_validate.rs`, `vrf_cert.rs`, `nonce.rs`, `op_cert.rs`, `leader_schedule.rs`, `fork_choice.rs`, `rollback.rs`, `kes_check.rs` (B1), `praos_state.rs`, `candidate.rs`, `events.rs`, `errors.rs`, `encoding.rs`, `ledger_view.rs`, `header_summary.rs`, `opcert_validate.rs` (N-C S2) | PHASE4-N-B; PHASE4-N-C / S2 |
| `ade_runtime::consensus` (new submodule of an existing RED crate) | GREEN/RED mix | Imperative-shell composition for consensus runtime. | `mod.rs`, `candidate_fragment.rs`, `chain_selector.rs`, `genesis_parser.rs` | PHASE4-N-B / S-B8, S-B10 |
| `ade_runtime::producer` (new submodule of an existing RED crate) | RED + GREEN mix | Imperative-shell composition for producer runtime. **N-G S5 added `broadcast_to_served.rs` + `served_chain_lookups.rs` as new GREEN files.** | `mod.rs`, `signing.rs`, `keys.rs`, `scheduler.rs`, `broadcast.rs`, `tick_assembler.rs`, `broadcast_to_served.rs` (**N-G S5**), `served_chain_lookups.rs` (**N-G S5**) | PHASE4-N-C; PHASE4-N-G / S5 |
| `ade_runtime::network` (new submodule of an existing RED crate) | RED | **Imperative-shell composition for the N2N server-role session driver** (N-G S6). | `network/mod.rs`, `network/n2n_server.rs` | PHASE4-N-G / S6 (`f773b1c`) |
| `ade_core_interop` (new workspace crate) | RED | Live cardano-node interop driver. **N-G S7 added `live_block_fetch_session.rs`.** | `src/lib.rs`, `src/follow.rs`, `src/tx_submission.rs` (N-E S4), `src/local_tx_submission.rs` (N-E S5), `src/bin/live_consensus_session.rs`, `src/bin/live_tx_submission_session.rs` (N-E S6), `src/bin/live_block_production_session.rs` (N-C S7), `src/bin/live_block_fetch_session.rs` (**N-G S7**), `tests/` | PHASE4-N-B; PHASE4-N-E; PHASE4-N-C; PHASE4-N-G |
| `ade_testkit::consensus` (new submodule of an existing crate) | GREEN | Test-only harness for consensus replay corpora. | `consensus/mod.rs`, `consensus/corpus.rs`, `consensus/ledger_view_stub.rs`, `consensus/stream_replay.rs` | PHASE4-N-B |
| `ade_runtime::chaindb` | RED | Block-store abstraction and impls. | `mod.rs`, `types.rs`, `error.rs`, `in_memory.rs`, `persistent.rs`, `contract.rs`, `snapshot_contract.rs`, `crash_safety.rs` | PHASE4-N-D |
| `ade_runtime::recovery` | RED | Composes ChainDb + SnapshotStore. | `recovery.rs` | PHASE4-N-D / S-36 |
| `ade_runtime` bin `chaindb_kill_target` | RED | Kill-target child process for the 1,000-kill-9 stress harness. | `src/bin/chaindb_kill_target.rs`, `tests/stress_kill_harness.rs` | PHASE4-N-D / S-37 |

Workspace-level membership grew by **two crates** across the full
delta: `ade_network` (PHASE4-N-A) and `ade_core_interop` (PHASE4-N-B).
Both are RED-or-mixed. **PHASE4-N-G added no new crate** — S1's
BLUE ServerReply wrappers, S2's BLUE served-chain module, S3/S4's
BLUE per-protocol reducers, S5's GREEN adapter + lookups, S6's RED
`network::n2n_server` submodule, and S7's mechanical cross-impl
adapter + RED binary all live as new files / submodules under the
existing 8 workspace crates.

Crate dependency shape at HEAD: **PHASE4-N-G S6 added one new
non-dev dep edge** — `ade_runtime` now depends directly on
`ade_network` (was absent entirely — the GREEN adapter / trait impls
in `producer::broadcast_to_served` + `producer::served_chain_lookups`
host the BLUE reducer interface bindings; the RED `network::n2n_server`
driver dispatches the reducers). **PHASE4-N-G S3 added four new
dev-dep edges on `ade_network`** — `ade_ledger`, `ade_testkit`,
`ade_core`, `ade_crypto` (all `[dev-dependencies]`, all required so
the trait-bound reducer tests can construct AcceptedBlock fixtures
and exercise the canonical header projection). The trait-bound seam
keeps `ade_network → ade_ledger` out of production deps. Dependency
direction RED → BLUE is permitted by `ci_check_dependency_boundary.sh`.
**PHASE4-N-G S7 added no new dep edge** — `ade_core_interop`
already has its workspace edges. Carried forward: the **PHASE4-N-C
S6** edge `ade_runtime → ade_ledger` and the **PHASE4-N-E S4** edge
`ade_core_interop → ade_ledger`. No edge from a BLUE crate to a RED
crate was introduced.

Corpora at HEAD: N-A capture corpus, N-B replay corpus, B1 validity
corpus, B3 conservation corpora, B4/B5 README-only synthetic notes,
the credential-fidelity corpus from OQ5-S2, the PPD in-code
synthetic-canonical corpus, the N-C in-code synthetic producer
corpus, and `corpus/snapshots/` under `.gitignore` (canonical home
`s3://ade-corpus-snapshots`). **PHASE4-N-G added no external
corpus** — the server-paths transcript replay test
(`tests/server_paths_transcript_replay.rs`) and the cross-impl
adapter (`tests/cross_impl_server_pipeline.rs`) both drive against
**the existing Conway-576 corpus** consumed by N-C S3/S7 and B1.
The CE-N-G-8 operator-action evidence is a live log captured against
a private Haskell peer, not a committed corpus.

Cross-reference: **The `ade-CODEMAP.md` regenerated in parallel with
this HEAD_DELTAS will record the new BLUE modules
`ade_ledger::producer::served_chain`, `ade_network::chain_sync::server`,
`ade_network::block_fetch::server`, the new GREEN files
`ade_runtime::producer::broadcast_to_served` and
`ade_runtime::producer::served_chain_lookups`, the new RED submodule
`ade_runtime::network` with `n2n_server`, and the new RED binary
`ade_core_interop::live_block_fetch_session`** as rows under their
respective crates' BLUE/GREEN/RED listings; the prior CODEMAP at
`df56e2d` does NOT yet contain any of those. SEAMS will pick up
`accepted_block_header_bytes` as the single canonical header
projection seam, `ServedChainSnapshot` as the single canonical
served-chain index seam, the trait pair `ServedHeaderLookup` /
`ServedRangeLookup` as the BLUE-reducer dependency-direction seam,
the closed `ServerReply<M>` wrapper pattern as the
server-agency-only construction seam, and the new RED → BLUE
production edge `ade_runtime → ade_network`. TRACEABILITY will pick
up the 6 new registry rules (`DC-CONS-17/18`, `DC-PROTO-07/08`,
`CN-PROTO-06`, `RO-LIVE-01`) with their 7 new `ci_script ↔ rule`
edges, plus the two strengthenings (`CN-CONS-07`, `DC-PROTO-06`);
the prior TRACEABILITY at `df56e2d` does NOT contain any of them.
All three rewrites are in flight in the grounding ripple immediately
following this HEAD_DELTAS regen; the four docs will be
self-consistent at the next grounding-doc commit.

---

## 3. Modules Modified

| Module | Scope | Key changes |
|--------|-------|-------------|
| `ade_ledger` | +75 source/test files over the full delta; **PHASE4-N-G touched 5 files**: `producer/served_chain.rs` (new), `producer/self_accept.rs` (+237: extension accessors `iter_accepted` / `block_at` exposing `&AcceptedBlock` so the GREEN adapter can call the canonical splitter without an inline walker), `producer/mod.rs` (+2: register `served_chain`), `block_validity/header_input.rs` (+29: lifts `accepted_block_header_bytes` to a public accessor wrapping the existing `header_cbor_slice` walker), `block_validity/mod.rs` (+1). | **PHASE4-N-G (S1 + S2 + S5):** new BLUE `producer::served_chain` (`ServedChainSnapshot`, `served_chain_admit`, BTreeMap-indexed append-only chain index; CN-CONS-07 broadcast gate preserved across the network seam); lifted `accepted_block_header_bytes` as the single canonical header projection (DC-CONS-16 strengthening, DC-CONS-18 foundation); `self_accept.rs` extension accessors exposing `&AcceptedBlock` so the GREEN adapter can call the canonical splitter directly. **Carried forward:** PHASE4-N-C producer submodule + `block_body_hash`; PHASE4-N-E mempool ingress chokepoint; B-series; OQ5/FIDELITY/WRITEBACK; PROPOSAL-PROCEDURES-DECODE. |
| `ade_network` | 100 files, +17,861 lines (full N-A); **PHASE4-N-G added 4 files / +1,402 LOC**: `chain_sync/server.rs` (~804 LOC), `chain_sync/mod.rs` (+1: register `server`), `block_fetch/server.rs` (~596 LOC), `block_fetch/mod.rs` (+1: register `server`). | **PHASE4-N-G (S1 + S3 + S4):** new BLUE `chain_sync::server` (closed `ServerReply<ChainSyncMessage>`, pure `producer_chain_sync_serve` + `producer_chain_sync_advance_tip`) and `block_fetch::server` (closed `ServerReply<BlockFetchMessage>`, pure `producer_block_fetch_serve`). Composes the existing PHASE4-N-A `chain_sync_transition` for grammar validation — no parallel state machine. Trait-bound seam (`ServedHeaderLookup`, `ServedRangeLookup`) keeps the BLUE reducer interface generic over the served-chain index, so `ade_network → ade_ledger` stays out of production deps. `Cargo.toml` (+4) adds `ade_ledger` / `ade_testkit` / `ade_core` / `ade_crypto` as `[dev-dependencies]` for the trait-bound tests. **Carried forward:** N-A wire-grammar work + DoS hardening. |
| `ade_runtime` | +24 files, +5,840 lines from prior threads; **PHASE4-N-G added 6 source files + 3 integration tests / +1,214 LOC**: `producer/broadcast_to_served.rs` (~188 LOC, S5 GREEN), `producer/served_chain_lookups.rs` (~120 LOC, S5 GREEN), `network/mod.rs` (S6 RED, +20), `network/n2n_server.rs` (~216 LOC, S6 RED), `producer/mod.rs` (+2), `lib.rs` (+1: register `network`); + tests `server_paths_transcript_replay.rs` (~310 LOC, S5), `n2n_server_two_peer_determinism.rs` (~167 LOC, S6), `cross_impl_server_pipeline.rs` (~193 LOC, S7). | **PHASE4-N-G (S5 + S6 + S7):** new GREEN `producer::broadcast_to_served` + `producer::served_chain_lookups`; new RED `network::n2n_server` (the pure decode → serve → encode driver, no socket I/O); three integration tests covering DC-PROTO-07 byte-identical replay, DC-CONS-17 served-bytes parity, DC-CONS-18 header-recipe agreement, OQ-4 two-peer determinism, and CE-N-G-7 mechanical cross-impl. **`Cargo.toml` adds a new non-dev `ade_network` dep** (the GREEN adapter / RED driver consume the BLUE reducer interface). **Carried forward:** N-C producer submodule (`signing`, `keys`, `scheduler`, `broadcast`, `tick_assembler`); N-B consensus runtime; N-D chaindb/recovery. |
| `ade_core_interop` | +1,793 across 9 files from prior threads; **PHASE4-N-G added 2 files / +202 LOC**: `src/bin/live_block_fetch_session.rs` (~141 LOC, S7 RED binary), `tests/live_block_fetch_session.rs` (~61 LOC, S7 build-and-start test). `Cargo.toml` (+4) adds the new `[[bin]]` entry. | **PHASE4-N-G (S7):** new RED operator-action evidence binary modeled on `live_block_production_session` and `live_tx_submission_session`; hermetic-default + `--connect` stub. **Carried forward:** N-C S7 producer-side live binary; N-E S4/S5/S6 tx-submission bridges; CE-N-B-6 follow-bridge. |
| `ade_ledger` (block_validity sub-area) | (counted above) | **PHASE4-N-G S1 surfaced `accepted_block_header_bytes` from the validator's body-hash recipe** (`header_input.rs` +29, `mod.rs` +1) — single canonical header projection across validator + producer-side server. The pre-existing `header_cbor_slice` walker is unchanged; the slice exposes the existing recipe rather than introducing a new one. This is the single change in `block_validity` for N-G. |
| `ade_core` | +30 source files + tests (N-B); +828 / −86 across 16 files (B1); +1 new file (N-C S2). **No PHASE4-N-G source change** — the N-G reducers depend only on `ade_ledger::producer::AcceptedBlock` + `ade_network` types. | **Unchanged in PHASE4-N-G.** **Carried forward:** N-B consensus authority + N-C S2 opcert validator. |
| `ade_codec` | +14 source/test files over the full delta; **no PHASE4-N-G change** — the served-chain index keys via `decode_block` (existing entry), and the server reducers never re-encode. | **Unchanged in PHASE4-N-G.** **Carried forward:** PPD PP-S1 `conway::governance`; N-C S2 `shelley::opcert`; N-C S3 `shelley::tx_components` producer assembly path; B3 / B4 / OQ5 era decoder work. |
| `ade_crypto` | 2 files: `kes.rs` (+122 / −81), `lib.rs` (+5). **No PHASE4-N-G change.** | **Unchanged in PHASE4-N-G.** **Carried forward:** N-C S1 `KesSignature` + `verify_kes_signature`. |
| `ade_testkit` | +33 files across the full delta; **no PHASE4-N-G change** — N-G's tests live as integration tests under `ade_runtime/tests/` and `ade_core_interop/tests/`, not under the testkit crate. | **Unchanged in PHASE4-N-G.** **Carried forward:** N-C `producer/` harness; PPD PP-S2 `governance/proposal_procedures_replay`; N-E `mempool/ingress_replay`. |

No other crate had non-trivial source changes since baseline.
`ade_plutus` and `ade_node` were untouched by code commits.
**PHASE4-N-G touched 4 of 8 workspace crates** (`ade_ledger`,
`ade_network`, `ade_runtime`, `ade_core_interop`). **No
`.idd-config.json` change.** **No BLUE authority-path semantics
changed apart from the S1 lift of `accepted_block_header_bytes` to a
public accessor + S2's new `ServedChainSnapshot` admission surface
+ S3/S4's new reducer surfaces** — the prior validator authorities
(`ade_core::consensus::*`, `ade_ledger::block_validity::*`,
`ade_ledger::block_body_hash`) were re-used unchanged by the server
pumps. The `accepted_block_header_bytes` lift reuses the existing
`header_cbor_slice` walker without changing what it computes.

---

## 4. Feature Flags

No Cargo `[features]` tables exist at HEAD in any workspace crate, and
none existed at baseline. The project does not use Cargo feature flags
as a semantic surface — closed semantic surfaces are encoded in the
type system per the IDD core principles, and conditional compilation
is checked out of BLUE code via `ci/ci_check_no_semantic_cfg.sh`
(scoped over the full 6-crate BLUE set, covering all surfaces
introduced through the PHASE4-N-G server reducers, served-chain
index, and header-projection seam).

No `#[cfg(feature = ...)]` gates appear at either ref.
**PHASE4-N-G introduced no new Ade-side feature flag and no new
upstream-crate feature selection.**

**Status: unchanged — zero Ade feature flags at baseline, zero at HEAD.**

---

## 5. CI Checks

The CI surface is the shell-script set under `ci/` (no
`.github/workflows` in this repo). At baseline there were 15 scripts.
At HEAD there are **47 scripts plus one git hook**
(`ci/git-hooks/commit-msg`). Across the full delta: CE-73 added one,
N-D added three, N-A added two, N-B added four, B3 added one, B3F
added one, B5 added one, OQ5 added one, PHASE4-N-E S1/S2 added two,
PROPOSAL-PROCEDURES-DECODE PP-S1 added one, PHASE4-N-C added eight
(the 33rd → 40th), and **PHASE4-N-G added seven (the 41st → 47th)**:
`ci_check_no_parallel_header_splitter.sh` (S1),
`ci_check_served_chain_closure.sh` (S2),
`ci_check_chain_sync_server_closure.sh` (S3),
`ci_check_block_fetch_server_closure.sh` (S4),
`ci_check_broadcast_to_served_purity.sh` (S5),
`ci_check_n2n_server_no_signing_dep.sh` (S6),
`ci_check_server_paths_corpus_present.sh` (S7).
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
| `ci/ci_check_module_headers.sh` | Modified — BLUE-scope (`5b70bee`) | `// Core Contract:` header on every `.rs` in BLUE crates. |
| `ci/ci_check_no_semantic_cfg.sh` | Modified — BLUE-scope (`5b70bee`) | No semantic `#[cfg(...)]` in BLUE `src/`. |
| `ci/ci_check_no_signing_in_blue.sh` | Modified — BLUE-scope (`5b70bee`) | No signing primitives in BLUE crates. |
| `ci/ci_check_hash_uses_wire_bytes.sh` | Modified — BLUE-scope (`5b70bee`) | All BLUE hashing via wire-byte fingerprint surfaces. |
| `ci/ci_check_ingress_chokepoints.sh` | Modified — BLUE-scope + registry growth (`5b70bee`) | No raw CBOR decoding outside named chokepoints in BLUE. |
| `ci/ci_check_dependency_boundary.sh` | Modified — BLUE-scope (`5b70bee`) | BLUE crates must not depend on RED crates. Continues to PASS at HEAD: the N-G S6 new edge is RED → BLUE (`ade_runtime → ade_network`), which is permitted. |

### Phase 4 N-D CI gap closure (`78da6c9`)

| Check | Status | What it checks |
|-------|--------|----------------|
| `ci/ci_check_chaindb_contract.sh` | **New** (`78da6c9`) | `cargo test -p ade_runtime --lib chaindb::` — 8 contract tests. |
| `ci/ci_check_recovery_contract.sh` | **New** (`78da6c9`) | `cargo test -p ade_runtime --lib recovery::` — 6-test recovery bundle. |
| `ci/ci_check_chaindb_crash_safety.sh` | **New** (`78da6c9`) | Smoke variant of the subprocess-SIGKILL harness + integrity post-checks. |

### Phase 4 N-A wire + semantic enforcement (S-A1, S-A10)

| Check | Status | What it checks |
|-------|--------|----------------|
| `ci/ci_check_no_async_in_blue.sh` | **New** (`4fde3a7`, S-A1) | Enforces `DC-CORE-01` — BLUE code is sync-only. |
| `ci/ci_check_ce_n_a_5_proof.sh` | **New** (`56bfa7b`, S-A10) | CE-N-A-5 closure-gate evidence over the real-cardano-node corpus. |

### Phase 4 N-B consensus authority enforcement — extended by B1, B2, N-C, and N-G

| Check | Status | What it checks |
|-------|--------|----------------|
| `ci/ci_check_consensus_closed_enums.sh` | **New** (N-B); **Modified** (B1, `7b95ccd`); **Modified** (B2); **Modified** (N-C); **Modified** (N-G, implicit via new closed enums in `chain_sync/server.rs`, `block_fetch/server.rs`, `producer/served_chain.rs`, `network/n2n_server.rs`) | Closed-enum scan over `ade_core/src/consensus/`, `ade_ledger/src/block_validity/`, `ade_ledger/src/tx_validity/`, `ade_ledger/src/mempool/`, `ade_ledger/src/producer/`, and (now) `ade_network/src/chain_sync/server.rs` + `ade_network/src/block_fetch/server.rs` (the new `ServerReply<M>` wrappers + `ProducerServerError`). |
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
| `ci/ci_check_credential_discriminant_closed.sh` | **New** (`a3ee2da`, OQ5-S2) | Enforces `DC-LEDGER-10`. **Unmodified by PHASE4-N-G.** |

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

### PHASE4-N-G producer-side server response paths closure (`8cd17c9`, `dc069cf`, `cc49b1d`, `03d120f`, `1a1b8e0`, `f773b1c`, `a280954`) — 7 new scripts (the 41st → 47th)

| Check | Status | What it checks |
|-------|--------|----------------|
| `ci/ci_check_no_parallel_header_splitter.sh` | **New** (`8cd17c9`, S1) — the **41st** script | Enforces single-authority header projection (foundation for `DC-CONS-16` strengthening + `DC-CONS-18` + `CN-PROTO-06`) via 2+ mechanical guards: (1) the only `pub fn .*header_bytes\b` definition across `crates/` lives in the canonical site `crates/ade_ledger/src/block_validity/header_input.rs` (specifically `accepted_block_header_bytes`); (2) no new `pub fn .*split_header(...)` or `pub fn .*split_block_envelope(...)` anywhere; (3) positive presence of the canonical accessor + the file-private walker `header_cbor_slice`. The GREEN adapter's import of `accepted_block_header_bytes` is the load-bearing positive check that no parallel splitter lives in `ade_runtime`. |
| `ci/ci_check_served_chain_closure.sh` | **New** (`dc069cf`, S2) — the **42nd** script | Enforces `served_chain.rs` closure via 3+ mechanical guards: (1) no `HashMap` / `HashSet` / any `std::collections::Hash*` type in `served_chain.rs` — BTreeMap is the only iteration order; (2) no wall-clock / rand / `tokio::time` in the file; (3) `served_chain_admit` signature is single-argument with no caller-supplied "asserted hash" parameter; (4) `ServedChainSnapshot` has no public constructor outside `new()` and `served_chain_admit` (private `blocks` field — no struct-literal back-door). Closure foundation for `DC-CONS-17` / `DC-CONS-18` (the served-chain index is the only path from `AcceptedBlock` to served wire bytes). |
| `ci/ci_check_chain_sync_server_closure.sh` | **New** (`cc49b1d`, S3) — the **43rd** script | Enforces `DC-PROTO-08` + `DC-PROTO-07` partial via 3+ mechanical guards: (1) production code in `chain_sync/server.rs` may not import wall-clock, randomness, async runtime, or `HashMap` iteration; (2) no `pub fn` in `chain_sync/server.rs` may return a raw `ChainSyncMessage` — outgoing replies go through `ServerReply::into_message()`; (3) positive presence: `producer_chain_sync_serve` + `producer_chain_sync_advance_tip`. Determinism-via-totality discipline: every server-agency state must return one of legal RollForward / RollBackward / AwaitReply / structured close-or-error. |
| `ci/ci_check_block_fetch_server_closure.sh` | **New** (`03d120f`, S4) — the **44th** script | Enforces `DC-PROTO-07` + `DC-CONS-17` foundation via 4+ mechanical guards: (1) production code in `block_fetch/server.rs` may not import wall-clock / randomness / async runtime / `HashMap`; (2) no `pub fn` returning a raw `BlockFetchMessage` (except `into_message`); (3) positive presence: `producer_block_fetch_serve` + `ServedRangeLookup` + the `served.range_bytes` call site; (4) `Block { bytes }` construction must source from `ServedRangeLookup` lookup output — never from re-encoding (`ServerReply::block(` is called only with `bytes` from the lookup iterator). |
| `ci/ci_check_broadcast_to_served_purity.sh` | **New** (`1a1b8e0`, S5) — the **45th** script | Enforces `DC-CONS-17` + `DC-CONS-18` + `DC-PROTO-07` GREEN-glue closure via 4+ mechanical guards: (1) `broadcast_to_served.rs` and `served_chain_lookups.rs` may not import wall-clock / randomness / async runtime / `HashMap`; (2) the adapter `drain_and_admit` exists with the expected signature consuming `BroadcastQueue` and returning `(ServedChainSnapshot, BroadcastQueue, Vec<AcceptedBlock>)`; (3) `ServedChainLookups` impls both `ServedHeaderLookup` and `ServedRangeLookup` (positive presence); (4) `served_chain_lookups.rs` uses the canonical `accepted_block_header_bytes` import — proving no parallel header splitter lives in the GREEN adapter. |
| `ci/ci_check_n2n_server_no_signing_dep.sh` | **New** (`f773b1c`, S6) — the **46th** script | Key-boundary doctrine for the producer-side server orchestrator: `crates/ade_runtime/src/network/` MUST NOT import from `crate::producer::signing` (or any item therein). Private-key custody stays RED-confined to the producer signing pipeline; the server pump only handles AcceptedBlock-derived bytes. Single mechanical grep over the network module tree. |
| `ci/ci_check_server_paths_corpus_present.sh` | **New** (`a280954`, S7) — the **47th** script | Enforces `RO-LIVE-01` mechanical half (CE-N-G-7 + CE-N-G-8 binary presence) via 4 mechanical guards: (1) the mechanical cross-impl integration test `crates/ade_runtime/tests/cross_impl_server_pipeline.rs` exists with the expected test names; (2) the transcript replay test `crates/ade_runtime/tests/server_paths_transcript_replay.rs` exists; (3) the `live_block_fetch_session` binary source file + `[[bin]]` entry are present; (4) the CE-N-G-8 procedure doc `docs/clusters/PHASE4-N-G/CE-N-G-8_PROCEDURE.md` exists. |

TRACEABILITY cross-reference: every script listed above appears as a
`ci_script` for at least one rule in `docs/ade-invariant-registry.toml`,
re-traced via `ci/ci_check_constitution_coverage.sh`. **PHASE4-N-G
added 7 new `ci_script ↔ rule` edges** (one per new script;
see §7). The constitution-coverage gate continues to PASS at HEAD.

---

## 6. Canonical Type Registry Delta

n/a — `.idd-config.json` `canonical_type_registry` is null.
Canonical-type rules live inline in the invariant registry under
family `T`.

**PHASE4-N-G introduced ~10 new closed types** in support of the
producer-side server response surface: `ServedChainSnapshot`,
`ServedChainError`, `ServedAdmitOutcome`, `ServerReply<ChainSyncMessage>`,
`ServerReply<BlockFetchMessage>`, `ProducerServerError`, `ServerStep`,
`BlockFetchServerStep`, `ServedHeaderLookup` (trait), `ServedRangeLookup`
(trait), `ServedChainLookups`, `PerPeerN2nServerState`. The two
`ServerReply<M>` wrappers are the load-bearing type-level
server-agency closure (CN-PROTO-06). `ServedChainSnapshot` is the
single canonical served-chain index; `accepted_block_header_bytes`
is the single canonical header-projection function (DC-CONS-18). The
rest are closed error taxonomies + capability tokens. Exact
whole-project recount belongs to the TRACEABILITY regen that follows.

**Removals: 0** (expected under append-only discipline).

---

## 7. Normative Rule Delta

The project's invariant registry tracks structured rules (TOML), not
prose normative-doc rules; this section reports on it.

- Rules at baseline (`d509f02:constitution_registry.toml`): **147**
- Rules at prior refresh (`df56e2d:docs/ade-invariant-registry.toml`): **190**
- Rules at HEAD (`a280954:docs/ade-invariant-registry.toml`): **196**
- Net additions vs baseline: **+49** (PHASE4-N-A: 2; PHASE4-N-B: 8;
  PHASE4-B1: 6; PHASE4-B2: 5; PHASE4-B3: 2; PHASE4-B3F: 0; PHASE4-B4: 1;
  PHASE4-B5: 1; OQ5: 1; COMMITTEE-CRED / DREP-VOTE / FIDELITY /
  WRITEBACK / post-3d94c22 testkit: 0 each; PHASE4-N-E S1–S5: 2;
  PHASE4-N-E S6: 0; PROPOSAL-PROCEDURES-DECODE: 1; PHASE4-N-C: 14;
  **PHASE4-N-G: 6** — `DC-CONS-17/18`, `DC-PROTO-07/08`, `CN-PROTO-06`,
  `RO-LIVE-01`, all introduced at `declared` in S1 (`8cd17c9`),
  `CN-PROTO-06` flipped to `enforced` at S1, `DC-PROTO-08` at S3,
  `DC-PROTO-07` at S4, `DC-CONS-17` + `DC-CONS-18` at S5, and
  `RO-LIVE-01` set to `partial` at S7).
- Net additions vs prior refresh: **+6** — the full N-G 6-rule family.
- Removals: **0** (expected under append-only discipline; clean).

- **Strengthenings recorded by PHASE4-N-G:**
  - **`CN-CONS-07.strengthened_in += "PHASE4-N-G"`** — the type-level
    broadcast gate is now preserved across the network seam: bytes
    that reach the wire must trace through `AcceptedBlock → ServedChainSnapshot`
    (the `served_chain_admit` API only accepts `AcceptedBlock`
    tokens, which only `self_accept` returning `Ok` produces). The
    `code_locus` field is extended to name `producer/served_chain.rs`
    alongside the existing `producer/self_accept.rs` /
    `block_validity/transition.rs` entries; the `attack_rationale`
    is amended to call out the network-seam end-run that the
    strengthening forbids.
  - **`DC-PROTO-06.strengthened_in += "PHASE4-N-G"`** — the
    mini-protocol session-replay-equivalence rule is strengthened by
    the producer-side server-role transitions (the new
    `chain_sync_server` + `block_fetch_server` per-session reducers
    extend DC-PROTO-06's scope from N-A's client-role transitions to
    cover the symmetric server-role role assignment used by the
    producer).
- **Strengthenings carried forward unchanged**: `DC-MEM-01`
  (PHASE4-N-E); `DC-MEM-02` (B2); `DC-EPOCH-01` (WRITEBACK + oracle);
  `DC-LEDGER-10` (OQ5 → COMMITTEE-CRED → DREP-VOTE →
  ENACTMENT-COMMITTEE-FIDELITY → WRITEBACK → oracle → PPD cross_ref);
  `DC-LEDGER-08` (B5); `T-DET-01` / `T-ENC-03` (OQ5 + N-C cross_ref);
  `DC-TXV-06` (B3F); `DC-VAL-06` (B3F + B4); `T-CONSERV-01` /
  `CN-LEDGER-07` (B3); `DC-MEM-01,02` (B2); `DC-EPOCH-02` (CE-73);
  the N-D bundle; the N-A real-capture bundle; `T-CORE-02` (S-B1);
  `T-ENC-01` (N-C `block_body_hash`).

- **Open obligations recorded by PHASE4-N-G:**
  - **`RO-LIVE-01.open_obligation = "blocked_until_operator_peer_available"`**
    — the live half of CE-N-G-8 (a Haskell cardano-node peer issuing
    RequestRange covering an Ade-forged block and accepting the
    bytes through full header+body validation) is blocked on a
    private Haskell peer being provisioned. The mechanical
    pre-condition is closed by S7's `cross_impl_server_pipeline`
    test (every served `Block{bytes}` decodes via Ade's
    envelope+block decoder and the recomputed body-hash matches the
    announced header's body_hash field, against the Conway-576
    corpus). The binary `live_block_fetch_session` builds + starts
    in hermetic mode; `--connect` mode requires the private Haskell
    peer. Reopen procedure documented at
    `docs/clusters/PHASE4-N-G/CE-N-G-8_PROCEDURE.md` — mirrors the
    PHASE4-N-C / CE-N-C-8 + PHASE4-N-E / CE-NODE-N2C-LTX patterns.

Family counts at HEAD: registry total **196** (= 190 + 6 from the
N-G family). The `DC` family grew by 4 (2 CONS + 2 PROTO); the `CN`
family entries grew by 1 (PROTO-06); the `RO` family is new with 1
entry (LIVE-01). Per the constitution coverage gate,
`ci_check_constitution_coverage.sh` PASSES at HEAD with the 6 new
rules' `ci_script` and `tests` arrays populated; rule status at HEAD
breaks down to **70 enforced, 16 partial, 110 declared** across the
196-entry registry.

Normative-doc rule extraction (the `normative_docs` list in
`.idd-config.json`) is approximate and not regenerated here — the
structured registry is the authoritative source.

---

## Anomalies and Cross-Reference Warnings

- **PHASE4-N-G cluster mechanically closed; CE-N-G-8 live half is a
  registry `open_obligation`, not a regression.** All 7 implementing
  slices (S1 → S7) land their CE — every CE-N-G-1..7 is mechanically
  enforced by a named CI script + named tests. CE-N-G-8 follows the
  cluster doc's explicit conditional-closure pattern:
  `RO-LIVE-01.open_obligation = "blocked_until_operator_peer_available"`
  + named blocker (private Haskell peer unavailable at HEAD
  `a280954`) + re-open criteria documented in
  `CE-N-G-8_PROCEDURE.md`. Mirrors N-C's `CN-CONS-06` and N-E's
  `CE-NODE-N2C-LTX` patterns — the documented conditional-closure
  doctrine, not a discipline gap.
- **CODEMAP / SEAMS / TRACEABILITY are stale at this HEAD — expected
  drift between cluster close and grounding ripple.** This regen
  refreshes HEAD_DELTAS only. Prior CODEMAP (`df56e2d`) does NOT
  contain any of the N-G new modules; prior SEAMS does NOT contain
  the `accepted_block_header_bytes` single header-projection seam,
  the `ServedChainSnapshot` single served-chain index, the trait
  pair `ServedHeaderLookup` / `ServedRangeLookup` as the
  BLUE-reducer dependency-direction seam, the closed `ServerReply<M>`
  wrapper pattern, or the new `ade_runtime → ade_network` production
  edge; prior TRACEABILITY does NOT contain the 6 new rules + 7 new
  `ci_script ↔ rule` edges. The grounding ripple immediately
  following this HEAD_DELTAS regen will bring all four docs to
  self-consistency.
- **One new RED → BLUE Cargo dep edge in PHASE4-N-G S6.**
  `ade_runtime → ade_network` is direct (was absent entirely); the
  GREEN adapter / RED driver in `producer::broadcast_to_served` +
  `producer::served_chain_lookups` + `network::n2n_server` host the
  BLUE reducer interface bindings. RED → BLUE direction is allowed
  by `ci_check_dependency_boundary.sh`. **Four new dev-dep edges on
  `ade_network`** (`ade_ledger`, `ade_testkit`, `ade_core`,
  `ade_crypto`) keep the trait-bound seam mechanical: the BLUE
  reducer surface in `ade_network` consumes only `ServedHeaderLookup`
  / `ServedRangeLookup` traits — the impls (which need
  `AcceptedBlock` + the canonical splitter) live in `ade_runtime`
  for production and in `ade_network`'s test modules for reducer
  unit tests. No BLUE → RED edge was introduced; the cluster's CI
  gates explicitly forbid the reverse direction
  (`ci_check_no_signing_in_blue.sh`,
  `ci_check_n2n_server_no_signing_dep.sh`).
- **N-G corpus is the existing Conway-576 corpus** consumed by N-C
  S3/S7 and B1. No new on-disk corpus was added. The transcript
  replay test and the cross-impl adapter both drive against
  AcceptedBlock arrivals derived from this corpus. The CE-N-G-8
  operator-action evidence (a live log against a private Haskell
  peer) is by design out of CI reach and is captured as an
  operator-recorded artifact, mirroring CE-N-C-8 and CE-N-E-6.
- **PHASE4-N-G cluster directory is NOT yet archived.**
  `docs/clusters/PHASE4-N-G/` (10 files: `cluster.md` +
  `N-G-S1.md` through `N-G-S7.md` + `CE-N-G-8_PROCEDURE.md` +
  planning spillovers `docs/planning/phase4-n-a-successor-invariants.md`
  + `docs/planning/phase4-n-g-cluster-slice-plan.md`) remains at the
  active cluster location. Archival to
  `docs/clusters/completed/PHASE4-N-G/` is deferred to a separate
  `/cluster-close` commit. Active `docs/clusters/` at HEAD now
  contains `completed/`, `PHASE4-N-B/` (carried-forward stray log
  directory), and **`PHASE4-N-G/`** (the newly closed but
  unarchived cluster).
- **`strengthened_in` records two strengthenings for N-G.**
  `CN-CONS-07.strengthened_in = ["PHASE4-N-G"]` (broadcast gate
  preserved across the network seam) and
  `DC-PROTO-06.strengthened_in = ["PHASE4-N-A", "PHASE4-N-G"]`
  (server-role transitions extend DC-PROTO-06's scope). Both are
  recorded as proper `strengthened_in` entries, not just cross-refs —
  this is the canonical pattern (and contrasts with the N-C
  cross_ref-only convention noted at `df56e2d`, which the next
  registry curation pass should normalize).
- **No removed canonical types** (n/a — no separate registry;
  canonical types at HEAD grew by ~10 from the N-G cluster on top
  of N-C's 22 + PPD's 1 since the prior baseline-snapshot count).
- **No removed registry rules** (expected: 0; actual: 0). **PHASE4-N-G
  added 6 new rules.** Registry total: **196** at HEAD (was 190 at
  prior refresh).
- **All commit subjects in this regen carry a conventional-commits
  prefix.** The 7 PHASE4-N-G commits are `feat(network)` ×3,
  `feat(ledger)` ×1, `feat(runtime)` ×2, `feat(interop)` ×1. **All 7
  commits in the `df56e2d..a280954` span carry the repo-required
  `Co-Authored-By: Claude Opus 4.7 (1M context)` model-attribution
  trailer** (per the CLAUDE.md project override for the bounty
  trailer ratio). The project hook `ci/git-hooks/commit-msg` is
  active in this clone and enforces the trailer mechanically.
- **Cluster docs archived as of this HEAD.** Sixteen cluster
  directories archived under `docs/clusters/completed/`:
  COMMITTEE-CRED-FIDELITY, DREP-VOTE-FIDELITY,
  ENACTMENT-COMMITTEE-FIDELITY, ENACTMENT-COMMITTEE-WRITEBACK,
  OQ5-CREDENTIAL-FIDELITY, PHASE4-B1, PHASE4-B2, PHASE4-B3,
  PHASE4-B3F, PHASE4-B4, PHASE4-B5, PHASE4-N-A, PHASE4-N-B,
  PHASE4-N-C, PHASE4-N-D, PHASE4-N-E, PROPOSAL-PROCEDURES-DECODE.
  **PHASE4-N-G is closed mechanically but its cluster directory is
  not yet archived.**
- **B5 / B4 / B3F / B3 / B2 / B1 / N-D / N-B / N-A / PHASE4-N-E /
  PROPOSAL-PROCEDURES-DECODE / PHASE4-N-C closures — carried
  forward unchanged.**
- **Pre-existing `boundary_fingerprint_matches_pins` failure on
  `byron_pre_hfc` predates this cluster.** Out-of-scope for
  PHASE4-N-G; not introduced by any N-G slice. Tracked under a
  separate future cluster.
- **DC-VAL status mismatch vs. closure claim (B1, carried forward).**
  Only `DC-VAL-01` is `enforced`; `DC-VAL-02` → `DC-VAL-05` remain
  `declared` despite named tests and the extended closed-enums
  enforcement point. Flip on the next `/traceability` pass.
- **Adversarial corpora are derived, not committed (carried forward).**
  N-E reuses the B2 B-track corpus verbatim; PPD PP-S2 ships its
  corpus in code; PHASE4-N-C ships its corpus in code; **PHASE4-N-G
  reuses the existing Conway-576 corpus** (no new corpus). The N-G
  corpus pattern continues the no-new-on-disk-artifacts trend.
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
phase boundary (Phase 4 close, which PHASE4-N-G brings further into
reach: producer-side server response paths are mechanically closed;
the remaining Phase-4 closure work is operator-action live evidence
for CE-N-C-8 / CE-N-G-8 / CE-NODE-N2C-LTX, the N-F operator surface,
and the OP-OPS-04 Sum6KES skey loader gap). Note the commit-hash
rewrite caveat at the top — re-derive hashes from `git log` at each
regen rather than carrying them forward. This regen is cut at HEAD
`a280954` (PHASE4-N-G S7). The prior regen narrated HEAD `694dd74`
(PHASE4-N-C S7, archived at `df56e2d`); the new span is
`df56e2d..a280954` — 7 commits: `8cd17c9` (S1 BLUE header projection
+ closed ServerReply wrappers), `dc069cf` (S2 BLUE
ServedChainSnapshot + served_chain_admit), `cc49b1d` (S3 BLUE
chain-sync server reducers), `03d120f` (S4 BLUE block-fetch server
reducer), `1a1b8e0` (S5 GREEN broadcast→served adapter + transcript
replay), `f773b1c` (S6 RED N2N server session driver), `a280954` (S7
mechanical cross-impl + live_block_fetch_session binary).
