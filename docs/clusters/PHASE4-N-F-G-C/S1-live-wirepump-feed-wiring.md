# Invariant Slice — PHASE4-N-F-G-C S1: Live WirePump feed wiring + hermetic loopback forge→serve proof

> **Status:** Planning Artifact (Non-Normative). Normative authority is the registry + CI.

## 2. Slice Header

### Slice Name
Live WirePump feed wiring + hermetic loopback forge→serve proof.

### Cluster
**PHASE4-N-F-G-C** — Live feed + operator-gated evidence (RO-LIVE-01, mechanical half).

### Status
Ready for Review (all §12 mechanical acceptance green; `ade_node` + `ade_runtime` suites + both gates pass).

### Cluster Exit Criteria Addressed
- [x] **CE-G-C-1 (live feed wiring + hermetic e2e — MECHANICAL, closeable)** — the binary consumes a
  live peer feed wired as `NodeBlockSource::from_wire_pump`; with a Continuing feed `LoopStep::ForgeTick`
  is reachable in the live-wired `--mode node` path; `NodeBlockSource` stays the closed 2-variant
  verdict-decoupled contract; the durable tip advances ONLY via `run_node_sync → pump_block`; a
  hermetic loopback e2e proves live-feed → forge → self-accept → sibling-serve → peer-block-fetch
  returns the forged block; the broadened `ci_check_served_chain_handoff_fence.sh` and the
  byte-unchanged `ci_check_node_run_loop_containment.sh` are green.

CE-G-C-2 (operator-gated evidence) is **out of scope** for this slice (it is S2).

### Slice Dependencies
- PHASE4-N-F-G-B S1–S3 (the self-accept→serve handoff: `SelfAcceptedHandoff`, the sibling `push_atomic`
  task, the handoff fence gate) — **merged** (`febee120`).
- PHASE4-N-F-G-A (forge fidelity; the `On`-arm `ForgeActivation` assembly) — **merged**.

## 3. Implementation Instruction (AI)
Implement exactly the wiring specified in §5/§9 — on the `On` arm, build a live `from_wire_pump`
source from the **existing closed** `dial_for_admission` + `run_admission_wire_pump` functions
**when an upstream peer is supplied** via the **existing** `--peer` ingress; when `--peer` is empty,
keep the prior empty `in_memory` source (forge-capable, halts clean — the existing
`node_mode_with_operator_keys_warm_start_forge_capable_halts_clean` contract). Broaden the existing
handoff-fence gate; add the hermetic loopback e2e + replay tests. Do **not** add a new wire authority,
a new CLI flag, a second forge codepath, a `NodeBlockSource` variant, a *new fatal invariant* (e.g.
forge-on-requires-peer — out of scope), or any peer-acceptance claim. Dial / parse failures are
**logged-and-dropped** per the established admission honest-scope (C3, `spawn_wire_pumps_for_admission`),
never fatal. If the recovered-tip→`Point` conversion for the pump's `start_point` is ambiguous,
**stop and ask** (do not invent one). Commit messages carry the project attribution trailer
(CLAUDE.md) and no other AI references.

## 4. Intent
Make it **impossible** for the `--mode node` forge to be observable on a path that bypasses the
single durable-tip authority or the self-accept→serve fence: the live peer feed enters **only**
through the closed verdict-decoupled `NodeBlockSource::WirePump` arm, the tip advances **only** via
`run_node_sync → pump_block`, and the now-live serve path remains gated so that **only** a BLUE
self-accepted artifact can be served. (Strengthens the mechanical half of `RO-LIVE-01`; keeps
`DC-SYNC-01/02`, `CN-NODE-02`, `DC-NODE-06` intact under a live feed.)

## 5. Scope
- **Modules / crates:**
  - `ade_node::node_lifecycle` (RED) — `On`-arm: **when `--peer` is supplied**, build a live
    `NodeBlockSource::from_wire_pump(rx)` fed by `dial_for_admission` + `run_admission_wire_pump`
    in place of `NodeBlockSource::in_memory(Vec::new())` (`:441`); **when `--peer` is empty, keep the
    empty source** (forge-capable, halts clean — prior behavior). A small RED helper
    (`spawn_live_wire_pump_source`) holds the dial/pump wiring.
  - `ade_node::cli` (RED) — **reuses the existing ingress**: `Cli.peer_addrs: Vec<String>` (the
    repeatable `--peer ADDR` flag, `cli.rs:71`) supplies the upstream peer; `Cli.network_magic`
    supplies the handshake magic on the live path. **No new CLI flag is added** — the `On` arm
    selects/parses the upstream `SocketAddr` from the existing address-only `--peer` ingress (no
    secrets, no semantic authority); a `--peer` with no `--network-magic` is a `MissingFlag`
    fail-closed (live path only — the empty-peer path is unaffected).
  - `ade_runtime::admission::{dial_for_admission, run_admission_wire_pump}` (RED) — **reused,
    unchanged**; the closed dial → chain-sync/block-fetch → `AdmissionPeerEvent` pump.
  - `ci/ci_check_served_chain_handoff_fence.sh` — **broadened** (file scope node-spine-wide;
    guard-3 deny-list → allow-list). Never weakened.
- **State machines affected:** none new. The relay loop (`LoopStep`) and `NodeBlockSource` reach
  states (`ForgeTick`, `WirePump`-Continuing) that were unreachable on the empty source; **no new
  variant, no new transition.**
- **Persistence impact:** none. No new WAL record, no checkpoint shape change. The durable tip path
  (`run_node_sync → pump_block`) is unchanged.
- **Network-visible impact:** when `--peer` is supplied, the production `On` arm opens an **outbound**
  N2N connection to the operator-supplied upstream peer (via `dial_for_admission`) and runs chain-sync
  + block-fetch as a **client** (existing protocol; no new/changed messages); with no `--peer` it
  keeps the prior empty source (forge-capable, halts clean — no connection). **The required CI proof
  uses an in-process loopback transport and does not claim live peer acceptance.** No inbound serve to
  a real peer in this slice (live serve to a real peer is the operator-gated S2/RO-LIVE-01 leg).
- **Out of scope:** the operator-pass runbook, evidence manifest, and `correlate` wiring (S2); any
  peer-acceptance / BA-02 claim; the `Off` arm (it may stay relay-only/empty — this slice does not
  prove live relay-only behavior).

## 6. Execution Boundary
- **BLUE (reuse only, unchanged):** `ade_ledger::producer::{self_accept, served_chain}`,
  `ade_network::block_fetch::server`, `ade_codec::cbor::tag24`, `EraSchedule`/nonce authorities.
  A BLUE change is out of scope → reject.
- **GREEN (reuse, unchanged):** `ade_network::session::core::step` (driven inside the admission pump),
  `ade_node::run_loop_planner::plan_loop_step`.
- **RED (the slice's work):** `ade_node::node_lifecycle` (`On`-arm source swap + spawn the admission
  pump), `ade_node::cli` (reuse the existing `--peer` ingress for the upstream address),
  `ade_runtime::admission::{dial_for_admission, run_admission_wire_pump}` (reused). Key custody stays
  RED-confined to `ProducerShell`.
- **Color resolved:** no ambiguity — every touched surface exists at its stated color; the slice adds
  only RED wiring and consumes existing BLUE/GREEN authorities + the existing RED CLI ingress.

## 7. Invariants Preserved
- `DC-SYNC-01` — durable tip advances only via `run_node_sync → pump_block`; the live source feeds
  that single path (no second tip-advance).
- `DC-SYNC-02` — every relay-loop iteration preserves durable-before-advance; no manual tip advance.
- `CN-NODE-02` — `--mode node` stays the single live-run lifecycle owner; the relay-loop body drives
  sync only via `run_node_sync` and forges via exactly one fenced `forge_one_from_recovered`
  (`ci_check_node_run_loop_containment.sh` byte-unchanged).
- `DC-NODE-06` — only a BLUE self-accepted artifact (typed `SelfAcceptedHandoff`) reaches the single
  `ServedChainHandle::push_atomic`; the relay-loop body forwards a typed channel send only.
- `CN-NODE-01` — single recovered/bootstrap `BootstrapState`; the live source adds no second bootstrap.
- `DC-NODE-05` — the forge tick stays subordinate (planner input `Due | NotDue`) + self-accept-only.
- `DC-EPOCH-03` — off-epoch forge fails closed before leadership/KES signing.
- `CN-FORGE-01`, `CN-WIRE-08` — sole `AcceptedBlock` producer; single tag-24 envelope authority.
- Determinism: no nondeterminism from the live wire reaches BLUE — only canonical `SlotNo` and
  canonical block bytes cross the RED seam (the `WirePump` lookahead is content-blind RED scheduling
  state).

## 8. Invariants Strengthened or Introduced
- **`RO-LIVE-01` (strengthened — mechanical observable-forge wiring half).** With a live (Continuing)
  feed wired into the closed `WirePump` arm, `LoopStep::ForgeTick` becomes reachable in the live-wired
  `--mode node` path and the forged block is served via the G-B sibling path — enforced by the new
  hermetic e2e + replay tests (§11/§12). **Stays `partial`**: peer ACCEPT is operator-gated
  (`blocked_until_operator_stake_available`), not touched by this slice. Recorded as
  `strengthened_in += "PHASE4-N-F-G-C"` at cluster close.
- **`DC-NODE-06` (enforcement hardened, not weakened).** Because the live-feed path is now paired
  with the existing served-chain path, `ci_check_served_chain_handoff_fence.sh` is **broadened**:
  file scope extended beyond `node_lifecycle.rs` to every node-spine serve owner, and guard-3
  converted from a deny-list (3 named bad channel types) to an **allow-list** (only
  `UnboundedSender<SelfAcceptedHandoff>` permitted). This tightens the existing gate; it does not add
  a new invariant family.

> The two strengthenings are causally bound: you cannot make the forge observable (RO-LIVE-01) without
> exercising the serve path live, which requires the DC-NODE-06 fence to be broadened to stay valid.
> RO-LIVE-01 is the primary family; the DC-NODE-06 gate-hardening is the coupled enforcement update.

## 9. Design Summary
Production path (`run_node_lifecycle_inner`, `On` arm): when `cli.peer_addrs` is **non-empty**, build
the live source via a small RED helper `spawn_live_wire_pump_source(&cli.peer_addrs, network_magic,
state.tip.as_ref())` (else keep `NodeBlockSource::in_memory(Vec::new())`). The helper:
1. builds `our_versions` by reusing the existing `build_n2n_version_table(network_magic)` (no reimpl);
2. builds `start_point` from the recovered tip (`ChainTip { hash, slot }` → `Point::Block { slot, hash }`,
   else `Point::Origin`);
3. `let (events_tx, events_rx) = mpsc::channel::<AdmissionPeerEvent>(CAP);` (the runtime
   `ade_runtime::admission::AdmissionPeerEvent` — the WirePump arm consumes it **directly**, no bridge);
4. for each parseable `--peer` addr, `tokio::spawn` `dial_for_admission(addr, versions)` then
   `run_admission_wire_pump(transport, label, start_point, version, network_magic, events_tx.clone())`
   — dial / parse failures `eprintln`-and-`return` (C3 honest-scope), never fatal;
5. returns `NodeBlockSource::from_wire_pump(events_rx)`.

The `WirePump` arm (`node_sync.rs:72`) already filters to `AdmissionPeerEvent::Block` (skips
`TipUpdate`, ends on `Disconnected`) — **no change**. The relay loop, sibling serve task, and clock
seam are unchanged from G-B. A `DialError`/`AdmissionDialError` fails the lifecycle closed with a
structured exit code (no silent relay fallback).

**The production On arm is wired to dial an operator-supplied upstream peer; the required CI proof
uses an in-process loopback transport and does not claim live peer acceptance.** Hermetic e2e (test,
at the `run_relay_loop` level — not full `main()`): use the existing `loopback_pair()` +
`spawn_duplex` harness to feed `run_admission_wire_pump` a loopback transport whose peer side serves
a known block; the `On`-arm relay loop reaches `ForgeTick`, forges + self-accepts, hands off to the
sibling `push_atomic`, and an **in-process block-fetch loopback over the served view returns the
forged block payload** (tag-24, CN-WIRE-08). The seam is clean because `from_wire_pump(rx)` accepts
any `mpsc::Receiver<AdmissionPeerEvent>` — production fills it from a real dial, the test from a
loopback pump.

## 10. Changes Introduced
### Types
- No new type. **No new CLI field** — the upstream peer is read from the existing `Cli.peer_addrs`.
  No canonical type, no `NodeBlockSource` variant, no `CoordinatorEvent` variant.
### State Transitions
- None new. `LoopStep::ForgeTick` and `WirePump`-Continuing become **reachable**; the transitions
  themselves are pre-existing (N-F-E/N-F-D).
### Persistence
- None.
### Removal / Refactors
- The empty `NodeBlockSource::in_memory(Vec::new())` on the `On` arm is replaced by the live source
  **when `--peer` is supplied**; with no `--peer` the empty source is retained (prior behavior). A
  small RED helper `spawn_live_wire_pump_source` is added (wiring only; reuses the closed pump). The
  `Off` arm is untouched.

## 11. Replay, Crash, and Epoch Validation
- **Replay (R1) — satisfied by the EXISTING `node_sync::tests::relay_loop_forge_two_runs_byte_identical`
  (CE-E-6).** That test is already a two-run forge replay over a `NodeBlockSource::from_wire_pump`
  (live-source-shaped) feed, proving byte-identical tip + WAL + checkpoints + forge-attempt sequence
  (incl. forged bytes). S1 adds **no new authoritative state** and the live wire is confined to the
  RED content-blind `WirePump` lookahead (only canonical `SlotNo` + block bytes cross into BLUE/GREEN),
  so R1 holds unchanged for the live-wired source — no redundant new test is introduced.
  Populated-feed post-state replay over real blocks is covered by `tests/admission_replay_equivalence.rs`.
- **Crash/restart behavior:** unchanged — durable state advances only via `run_node_sync →
  pump_block`; a dial/pump failure fails the lifecycle closed and recovery re-runs warm-start
  (`CN-NODE-01` intact). No new durable state to recover.
- **Epoch boundary behavior:** unchanged — off-epoch forge fails closed (`DC-EPOCH-03`); this slice
  forges only within the recovered seed epoch.

## 12. Mechanical Acceptance Criteria
This slice is complete only when **all** of the following exist and pass in CI (hermetic — no Docker,
no live peer):

- [x] `node_sync::tests::live_wire_pump_feed_reaches_forge_tick` — a Continuing `WirePump` feed makes
  the relay loop reach `LoopStep::ForgeTick` (forge attempted); the empty `InMemory` source halts
  before any `ForgeTick` (the live-vs-empty contrast isolates the live-feed effect).
- [x] `node_sync::tests::node_block_source_stays_closed_two_variant` — `NodeBlockSource` has exactly
  `{WirePump, InMemory}` (exhaustive no-wildcard match); the live source is a `WirePump` fill, not a
  new variant.
- [x] `node_lifecycle::tests::spawn_live_wire_pump_source_with_no_usable_peer_yields_ended_feed` — the
  live-wire helper is fail-soft (C3): empty `--peer` / an unparseable addr yields an ended feed (the
  relay loop halts clean) — never fatal, never a fabricated address, never a graft.
- [x] `tests/forge_succeeds.rs::live_feed_forge_serve_loopback_returns_forged_block` — hermetic e2e
  serve leg: a FORGE-derived self-accepted block → S1 handoff → single `push_atomic` → **in-process
  block-fetch loopback over the served view returns the forged block payload** (tag-24 unwraps to the
  forged bytes). (The feed→ForgeTick leg is `live_wire_pump_feed_reaches_forge_tick`; the dial/pump
  are proven in `ade_runtime` wire_pump tests.)
- [x] **R1 replay** — the existing `node_sync::tests::relay_loop_forge_two_runs_byte_identical`
  (CE-E-6, a `from_wire_pump` two-run forge replay) + `tests/admission_replay_equivalence.rs`; S1 adds
  no new authoritative state (see §11). No redundant new test.
- [x] `ci_check_served_chain_handoff_fence.sh` — **broadened** (node-spine owner set
  `{node_lifecycle.rs, node_sync.rs}`; guard-3 allow-list: every node-spine unbounded handoff channel
  MUST be `<SelfAcceptedHandoff>`) and **green**.
- [x] `ci_check_node_run_loop_containment.sh` — **byte-/semantically unchanged and green** (no
  serve/admit/`push_atomic`/second-tip token added to the relay-loop body).
- [x] `cargo test -p ade_node -p ade_runtime` green (scoped to touched crates per the project's
  selective-acceptance discipline; `ade_testkit` corpus lane excluded — pre-existing timeout).

### Operator smoke (optional; non-blocking; NOT a slice-completion criterion)
A bounded docker/preprod smoke against the local `cardano-node-preprod` peer validates the live dial +
session wiring ONLY and proves **wiring, not acceptance** (Ade has no preprod stake → never a leader
there). **This slice commits no transcript** and adds no transcript-replay gate — the docker smoke is
an operator-run sanity check, not a closeable criterion. (A committed transcript + its replay gate
belong to the operator-gated S2/RO-LIVE-01 leg, mirroring `RO-SYNC-EVIDENCE-01`.)

## 13. Failure Modes
- `dial_for_admission` failure (`AdmissionDialError`) → lifecycle fails closed with a structured exit
  code; **fail-fast**; no silent relay-only fallback; no replay impact (no durable state written).
- `run_admission_wire_pump` termination (`AdmissionWirePumpResult::{Eof, Error, EventsChannelDropped}`)
  → the `WirePump` source observes channel disconnect and ends the feed once the lookahead drains
  (existing N-F-D semantics); the loop halts cleanly. **Recoverable** (re-dial on next run).
- Unparseable `--peer` addr / `dial_for_admission` failure → **logged-and-dropped** per the established
  admission honest-scope (C3, `spawn_wire_pumps_for_admission`); the feed ends and the loop halts
  clean. **Non-fatal** (never a silent tip graft, never a fabricated address).
- Empty `--peer` on the `On` arm → **keep the empty `in_memory` source** (forge-capable, halts clean —
  the prior N-F-F contract; **not** an error). The live feed is opt-in via `--peer`.
- `--peer` supplied without `--network-magic` → `NodeLifecycleError::MissingFlag("--network-magic")`
  on the **live path only**. **Fail-fast** (the empty-peer path is unaffected).

## 14. Hard Prohibitions
### Inherited Cluster-Level Prohibitions
All prohibitions in the PHASE4-N-F-G-C cluster doc "Forbidden During This Cluster" apply (no
containment relaxation; no handoff-fence relaxation; no `NodeBlockSource` plugin point; no synthetic
BA-02 manifest; no cross-epoch stale-`eta0` forge; no new BLUE authority/canonical type; no parallel
serializer; no second serve authority; no "peer accepted" rule).
### Slice-Specific Prohibitions
- No new `NodeBlockSource` variant; no wildcard match on `NodeBlockSource`.
- No second tip-advance path; the live feed feeds **only** `run_node_sync → pump_block`.
- No reinterpretation of `AdmissionPeerEvent::TipUpdate`/`Disconnected` as a verdict or tip authority
  (the source stays verdict-decoupled).
- No fork or reimplementation of `run_admission_wire_pump` (or `dial_for_admission`); reuse the
  existing admission pump verbatim.
- No new CLI flag; reuse the existing `--peer` ingress (address only).
- No peer-acceptance / BA-02 / `correlate` wiring (that is S2).
- No key/secret bytes in the ingress (address only); no secret in logs/debug.
- No live-serve-to-a-real-peer claim from the hermetic e2e.

## 15. Explicit Non-Goals
This slice MUST NOT: implement the operator-pass runbook / evidence manifest / `correlate` wiring
(S2); claim or infer peer acceptance; wire the `Off` arm to a live feed; **make `--peer` mandatory for
forge-on or add a forge-on-requires-peer fatal invariant** (the empty-peer path stays forge-capable +
halts clean); add a protocol version, feature flag, or new CLI flag; optimize performance; prepare
cross-epoch production; add a second forge codepath or a second bootstrap; modify any BLUE crate.

## 16. Completion Checklist
- [ ] All new state is replay-derivable (R1 test passes; no new durable state).
- [ ] All new data is canonically encoded (served payload = existing canonical forged bytes, tag-24).
- [ ] All failure modes are deterministic (§13 — dial/pump/peer-ingress fail-fast, structured).
- [ ] No TODOs/placeholders in authoritative (BLUE) paths (no BLUE change at all).
- [ ] CI enforces the strengthened invariant (broadened handoff fence + unchanged containment gate).
- [ ] Replay-equivalence test passes across runs.

## 17. Review Notes
- **Invariant risk considered:** the live wire is the only new nondeterminism; it is confined to the
  RED `WirePump` lookahead and never observed by BLUE/GREEN authority — only canonical `SlotNo` +
  block bytes cross.
- **Assumption challenged:** that wiring a live feed is "just a one-line swap" — it is not; it adds an
  outbound dial + selects the upstream peer from the existing `--peer` ingress. Both are RED and
  bounded.
- **Ingress confirmed:** the upstream-peer address ingress **already exists** (`Cli.peer_addrs`,
  `--peer ADDR`, repeatable, address-only) — S1 reuses it; no new flag.
- **Recovered-tip→`Point` confirmed:** `state.tip: Option<ChainTip { hash: Hash32, slot: SlotNo }>`
  (`chaindb/types.rs:29`) → `Point::Block { slot, hash }`, else `Point::Origin`. No invention.
- **Conditional-wiring decision (resolved during implementation, doc amended):** empty `--peer`
  **preserves** the prior empty-source forge-capable-halts-clean contract
  (`node_mode_with_operator_keys_warm_start_forge_capable_halts_clean`); a forge-on-requires-peer
  fatal invariant is **out of scope** (purely additive live wiring, dial failures non-fatal per C3).
- **Follow-up implied:** S2 (operator-pass runbook + evidence manifest + `correlate`); the live peer
  ACCEPT remains the operator-gated RO-LIVE-01 leg.
