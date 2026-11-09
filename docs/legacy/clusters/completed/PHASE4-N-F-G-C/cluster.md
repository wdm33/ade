# Cluster PHASE4-N-F-G-C — Live feed + operator-gated evidence

> **Status: PLANNED** (from committed plan `docs/planning/phase4-n-f-g-cluster-slice-plan.md`,
> G-C section + the `/invariants` sketch `docs/planning/phase4-n-f-g-invariants.md`). Third and
> final sub-cluster of **PHASE4-N-F-G** (RO-LIVE-01). Predecessors: **PHASE4-N-F-G-A** (forge
> fidelity — CLOSED, origin/main `62cb8718`) + **PHASE4-N-F-G-B** (self-accept→serve handoff —
> CLOSED, origin/main `febee120`; `DC-NODE-06` enforced). Code-verified at HEAD `febee120`.
>
> **Cluster character (load-bearing — do not broaden):** this cluster has **two halves with
> sharply different IDD status.** (1) A **MECHANICAL half** (closeable hermetically): wire a
> **live** peer feed into the existing closed `NodeBlockSource::WirePump` arm so `LoopStep::ForgeTick`
> becomes reachable in the live-wired `--mode node` path, and prove live-feed → forge → self-accept →
> sibling-serve → peer-block-fetch end-to-end on a hermetic loopback. (2) An **OPERATOR-GATED half**
> (stays `blocked`): the live peer ACCEPT + the live BA-02 evidence —
> `blocked_until_operator_stake_available`, **NOT deferred**. This cluster **does not redefine
> acceptance**: peer ACCEPT is operator-gated evidence captured from the peer's validation log through
> `ba02_evidence::correlate` — never a runtime invariant, never inferred from Ade's self-accept /
> `ForgeSucceeded` / any wire-success signal.
>
> **Hard lines (any one breached → stop and re-scope):**
> - Must **NOT relax** `ci_check_node_run_loop_containment.sh` (byte-/semantically unchanged) — the
>   live source fills the existing `WirePump` arm; it adds no second tip-advance path, no serve/admit
>   token in the loop body. Served-chain mutation stays in the **sibling** task (the handoff is a
>   typed channel send).
> - Must **NOT relax** the served-chain handoff fence `ci_check_served_chain_handoff_fence.sh`
>   (`DC-NODE-06`): only a BLUE self-accepted artifact reaches `push_atomic`, via the typed
>   `SelfAcceptedHandoff`. (G-C **broadens** this gate's file scope + tightens guard-3 — see CE-G-C-1
>   — but never weakens it.)
> - `NodeBlockSource` stays the **closed 2-variant verdict-decoupled** contract — wiring a live source
>   is a *fill* of `WirePump`, **not** a new plugin point.
> - **No synthetic BA-02 manifest is committed.** Acceptance is proven ONLY by the operator-captured
>   peer log through `correlate`. **No new rule asserts "a peer accepted."**

## Primary invariant
**No new rule.** `RO-LIVE-01` (operator-gated release obligation; stays `partial`, live half
`blocked_until_operator_stake_available`) is **strengthened** by the node-spine observable-forge
wiring: with a live (Continuing) feed wired into the closed `NodeBlockSource::WirePump` arm,
`LoopStep::ForgeTick` becomes reachable in the live-wired `--mode node` path and the forged block is
served via the G-B sibling-serve path — but peer ACCEPT remains operator-gated evidence proven only
by the peer's validation log through `ba02_evidence::correlate` (`RO-LIVE-06`), never by any
Ade-internal signal. The mechanical half carries `DC-NODE-06`, `DC-SYNC-01`, `DC-SYNC-02`,
`CN-NODE-02`, and `RO-LIVE-06` **unchanged**. *(Cited, not restated — see the registry entries.)*

## Invariants strengthened (at close)
- `RO-LIVE-01` — `strengthened_in += "PHASE4-N-F-G-C"`; **stays `partial`** (live ACCEPT
  `blocked_until_operator_stake_available`). The node-spine live-feed wiring makes the forge
  *observable* on the live-wired `--mode node` path; it does **not** flip the operator-gated release
  obligation.
- Carried unchanged (not weakened): `DC-NODE-06` (G-B serve handoff), `DC-SYNC-01` / `DC-SYNC-02`
  (single durable tip-advance via `run_node_sync → pump_block`), `CN-NODE-02` (single live-run
  lifecycle owner), `RO-LIVE-06` (BA-02 schema + correlation mechanics; peer ACCEPT operator-gated),
  `CN-CONS-06` (cross-impl acceptance — live half stays `blocked_until_operator_stake_available`),
  `CN-FORGE-01` (the `AcceptedBlock` provenance fence), `CN-WIRE-08` (single tag-24 envelope authority),
  `DC-EPOCH-03` (single-epoch forge fail-closed).
- **Not created here (deliberate):** no "peer accepted the block" rule. Peer acceptance is
  release/operator evidence (`RO-LIVE-01` / `RO-LIVE-06`), never a runtime invariant.

## Normative anchors
- `docs/planning/phase4-n-f-g-cluster-slice-plan.md` — G-C section (2 CEs, 2 slices).
- `docs/planning/phase4-n-f-g-invariants.md` — the `/invariants` sketch: §0 two-halves framing,
  Leg A (A1–A3 live feed), Leg B (B1–B4 sibling serve / peer-evidence), OQ4 (venue C1/C2),
  **OQ5 (genesis-consistency of the recovered seed epoch — the deep validity risk)**.
- `docs/planning/operator-pass-live-leg-c1-scoping.md` — the C1 scoping pass (mined for its gap
  analysis + the `S1.md` runbook bugs to fix; not followed literally — it is produce_mode-centric
  and predates N-F-F).
- Registry: `RO-LIVE-01`, `RO-LIVE-06`, `CN-CONS-06`, `CN-OPERATOR-EVIDENCE-01` (operator-pass
  evidence manifest schema), `DC-NODE-06`, `DC-SYNC-01/02`, `CN-NODE-02`.

## Entry conditions (what prior clusters guarantee)
- **G-B (closed, `febee120`):** `DC-NODE-06` **enforced** — a self-accepted forged artifact reaches
  the served chain via the typed `SelfAcceptedHandoff` + the dispatcher-spawned sibling
  `push_atomic` task; the relay-loop body forwards a typed channel send only
  (`ci_check_node_run_loop_containment.sh` + `ci_check_served_chain_handoff_fence.sh` green). G-C
  **reuses** this serve path unchanged — it adds the *live feed* that makes the forge observable.
- **G-A (closed):** the `--mode node` forge produces a genesis-consistent, slot-aligned,
  epoch-bounded **self-accepted** artifact (real opcert/genesis ingress + current pparams + two
  fail-closed boundaries; `DC-EPOCH-03` enforced). The G-A `genesis_pinning` harness exists for
  the OQ5 genesis-consistency proof obligation.
- **N-F-D / N-F-E:** the relay run loop (`run_relay_loop`, closed 4-variant `LoopStep` incl.
  `ForgeTick`); `NodeBlockSource`'s **content-blind readiness** (`has_work_ready` / `wait_ready`,
  the `WirePump` lookahead). `run_node_sync` is UNMODIFIED. Relay-only + hermetic to date — the
  **live unbounded peer is exactly the RO-LIVE-01 follow-on this cluster wires.**
- **N-F-C:** `NodeBlockSource` is the closed 2-variant verdict-decoupled contract
  (`WirePump` / `InMemory`); `ba02_evidence::correlate` is the **sole** `Ba02Manifest` constructor
  (library surface, reached by no binary arm to date).
- **N-L / N-M / N-S (wire infra):** `N2nDialer::dial`, `MuxPump::run`, the GREEN `session::core::step`
  reducer, and `run_admission_wire_pump` (producing `mpsc::Receiver<AdmissionPeerEvent>`) already
  exist — G-C **wires** them into the `--mode node` binary's `WirePump` arm (no new wire authority).

## Verified component inventory (read at HEAD `febee120`, not assumed)
| Component | Real state (verified) | Use |
|---|---|---|
| `NodeBlockSource::WirePump { rx: mpsc::Receiver<AdmissionPeerEvent>, lookahead, disconnected }` (`node_sync.rs:72`) + `from_wire_pump(rx)` ctor (`:91`) | the **already-closed** live arm; verdict-decoupled; `next_block` skips `TipUpdate`, ends on disconnect | **S1** the arm the live feed fills (no new variant) |
| empty `NodeBlockSource::in_memory(Vec::new())` at `node_lifecycle.rs:378` (Off arm) + `:441` (On arm) | the empty hermetic source that halts the loop before any `ForgeTick` | **S1** the **On-arm** site G-C replaces with a live `from_wire_pump` source (the Off arm may stay relay-only/empty) |
| `N2nDialer::dial(self) -> Result<PeerId, DialError>` (`n2n_dialer.rs:61`); `DialError` (`:44`) | RED dialer (N-L); TCP + mux + handshake | **S1** the live-feed origin |
| `MuxPump::run(self)` (`mux_pump.rs:59`) | RED mux pump (N-L/N-M) | **S1** drives the mux demux into the session |
| `session::core::step(...)` (`session/core.rs:43`) | GREEN session reducer; emits `Block` events for the `BlockFetch` mini-protocol | **S1** decodes frames → block events → WirePump mpsc |
| `run_admission_wire_pump` → `mpsc::Receiver<AdmissionPeerEvent>` (referenced by `WirePump` doc) | RED pump that taps the peer's `BlockFetch` source as ordered block events | **S1** the channel the live `WirePump` arm consumes |
| G-B serve path: `ForgeActivation::with_handoff_sender` + sibling `push_atomic` task (`node_lifecycle.rs`) | RED; `DC-NODE-06` enforced; sole node-spine `push_atomic` | **S1** reused unchanged — the observable forge feeds it |
| `ba02_evidence::correlate(ade, peer_log) -> BA02Outcome` (`ba02_evidence.rs:290`) | GREEN; **sole** `Ba02Manifest` constructor; `Ba02Manifest` is "a CLAIM ABOUT authority, not authority" | **S2** wired to the operator-captured peer log (no synthetic manifest) |
| `ade_testkit::consensus::genesis_pinning` (G-A S1, `#[cfg(test)]`) | GREEN; pins recovered values + leader-eligibility inputs against the genesis-derived reference | **S2** the OQ5 genesis-consistency proof obligation harness |
| `ci_check_node_run_loop_containment.sh` (N-F-E) | forbids serve/tip tokens in the loop body | **UNCHANGED** by G-C (hard line) |
| `ci_check_served_chain_handoff_fence.sh` (G-B; `OWNER=node_lifecycle.rs`, guard-3 deny-list) | DC-NODE-06 serve-ingress fence | **S1 broadens** (file scope + guard-3 → allow-list) — never weakens (carried IDD WARN) |

## Slices (safety order)

### S1 — Live WirePump feed wiring + hermetic loopback forge→serve proof *(mechanical; CE-G-C-1)*
Replace the empty `NodeBlockSource::in_memory(Vec::new())` on the `--mode node` **On** arm
(`node_lifecycle.rs:441`) with a **live** `NodeBlockSource::from_wire_pump(rx)` fed by
`N2nDialer::dial → MuxPump::run + session::core::step → run_admission_wire_pump`. **S1 wires the
forge-capable On arm to the live WirePump; the Off arm may remain relay-only/empty unless the slice
explicitly proves the live relay-only behavior too** (do not silently expand S1 into two lifecycle
modes). With a Continuing feed, `LoopStep::ForgeTick` becomes reachable in the live-wired `--mode
node` path; the forge fires *because* the feed is Continuing (forge subordinate to feed — `CN-NODE-02`
/ `DC-NODE-05` hold; planner input stays the content-blind `Due | NotDue`). `NodeBlockSource` stays
the closed 2-variant verdict-decoupled contract (no plugin point); the durable tip advances **only**
via `run_node_sync → pump_block` (`DC-SYNC-01`), no second bootstrap (`CN-NODE-01`). A **hermetic
loopback e2e** proves live-feed → forge → self-accept → sibling-serve → peer-block-fetch returns the
forged block (reusing the G-B serve path unchanged). **Gate evolution (carried IDD WARN — must land
here):** because the live-feed path is now paired with the existing served-chain path, broaden
`ci_check_served_chain_handoff_fence.sh` beyond `node_lifecycle.rs` to every node-spine serve owner,
and convert guard-3 from a deny-list (bans 3 named bad channel types) to an **allow-list** (only
`UnboundedSender<SelfAcceptedHandoff>` is permitted). A **bounded docker/preprod smoke validates
live feed / session wiring ONLY**; it does **NOT** prove peer acceptance (Ade has no preprod stake →
never a leader there) — it captures a replay-equivalent transcript as a reusable artifact, never a
substitute for the deliverable. Addresses **CE-G-C-1**. TCB: **RED** (dialer→WirePump wiring) +
consume the G-B serve path + the GREEN `session::core::step` reducer.

### S2 — Operator-pass runbook + evidence manifest + correlate wiring *(operator-gated; CE-G-C-2)*
Commit the **corrected** C1/C2 operator-pass runbook (fixing the `S1.md` bugs the C1 scoping doc
identified), define the evidence-manifest home + sha256 cross-check (mirroring
`CN-OPERATOR-EVIDENCE-01` / `RO-SYNC-EVIDENCE-01`), and wire `ba02_evidence::correlate` to an
operator-captured peer log. **No synthetic manifest is committed** — acceptance is proven ONLY by
the peer's raw validation log naming the exact Ade-forged hash. The live ACCEPT stays
`blocked_until_operator_stake_available` (named, **not** deferred). **Slice-entry proof obligation
(OQ5):** before any live KES signature, pin Ade's `praos_vrf_input(slot, eta0)` + threshold inputs
against the peer for the recovered seed epoch via the G-A `genesis_pinning` harness — a from-genesis/
tip private net must take the **WarmStart** arm (operator pre-seeds the store; node `FirstRun` is
Mithril-only, so no new bootstrap path; `CN-NODE-01` intact). **Passing the genesis-pinning harness
is necessary but not sufficient for peer acceptance; peer acceptance still requires operator-captured
peer logs through `correlate`.** Addresses **CE-G-C-2** (scaffolds the operator-gated path; the live
execution flips `RO-LIVE-01` in a later operator-witnessed pass). TCB: **RED** (runbook / evidence
I/O) + **GREEN** (`correlate`, reused).

## Exit criteria (mechanical, CI-verifiable)
New test/check names are **candidate** (created by the owning slice); existing artifacts named as-is.

- **CE-G-C-1 (live feed wiring + hermetic e2e — MECHANICAL, closeable)** — the binary consumes a
  live peer feed wired as `NodeBlockSource::from_wire_pump`; with a Continuing feed `LoopStep::ForgeTick`
  is reachable in the live-wired `--mode node` path; `NodeBlockSource` stays the closed 2-variant
  verdict-decoupled contract; the durable tip advances ONLY via `run_node_sync → pump_block`.
  Candidate tests `live_wire_pump_feed_reaches_forge_tick`, `node_block_source_stays_closed_two_variant`,
  `live_feed_forge_serve_loopback_returns_forged_block`; existing
  `ci_check_node_run_loop_containment.sh` **byte-/semantically unchanged + green**; the **broadened**
  `ci_check_served_chain_handoff_fence.sh` (node-spine-wide scope + guard-3 allow-list) green.
  A bounded docker/preprod smoke validates **wiring only** (not acceptance) and commits a
  replay-equivalent transcript artifact.
- **CE-G-C-2 (operator-gated evidence — SCAFFOLDS ONLY; live ACCEPT BLOCKED)** — the corrected C1/C2
  runbook is committed; the evidence-manifest home + sha256 cross-check is defined;
  `ba02_evidence::correlate` is wired to the operator-captured peer log; **no synthetic manifest is
  committed**; the OQ5 genesis-consistency pin (`genesis_pinning`) passes for the recovered seed
  epoch. Candidate tests `correlate_wired_to_operator_peer_log`,
  `no_committed_synthetic_ba02_manifest`. Live peer ACCEPT stays
  `blocked_until_operator_stake_available` (`RO-LIVE-01` live half / `CN-CONS-06`) — named, not
  deferred; proven ONLY by the peer's validation log.

> No human review may substitute for these checks. CE-G-C-1 closes the cluster mechanically;
> CE-G-C-2 closes its **scaffolding** mechanically — the live ACCEPT is a separate operator-witnessed
> pass that flips `RO-LIVE-01` later.

## TCB color map
- **BLUE (none — reuse only):** `ade_ledger::producer::{self_accept, served_chain}`,
  `ade_network::block_fetch::server`, `ade_codec::cbor::tag24`, the era schedule / nonce authorities.
  A BLUE change is a red flag → reject.
- **GREEN:** `ade_network::session::core::step` (the wire session reducer), `ade_node::run_loop_planner`
  (`plan_loop_step`), `ade_node::ba02_evidence::correlate` — all **reused, unchanged**.
- **RED (the bulk — *wiring*):** `ade_runtime::network::{n2n_dialer, mux_pump}` + `run_admission_wire_pump`
  (wired into the binary); `ade_node::node_lifecycle` (the `On`-arm source swap empty→live + retained
  G-B sibling serve task); the operator-pass driver + evidence-manifest I/O. Key custody stays
  RED-confined to `ProducerShell`.
- **Color resolved:** no open color question — every surface exists at its stated color; G-C consumes
  existing BLUE/GREEN authorities and adds only RED wiring + RED operator-pass I/O.

## Forbidden during this cluster *(slice-level prohibitions inherit)*
- **Do not relax `ci_check_node_run_loop_containment.sh`** (byte-/semantically unchanged) — no
  serve/admit/`push_atomic`/`OutboundCommand`/`broadcast`/`block_fetch`/second-tip-advance token in
  the relay-loop body. The live source fills `WirePump`; served-chain mutation stays in the sibling
  task; the handoff is a typed channel send.
- **Do not relax the served-chain handoff fence** (`DC-NODE-06`) — G-C may **broaden** its scope and
  tighten guard-3 to an allow-list, never weaken it.
- **Do not turn `NodeBlockSource` into a plugin point** — the live source is a *fill* of the closed
  `WirePump` arm; no third variant, no wildcard, no second forge codepath, no second bootstrap.
- **Do not commit a synthetic BA-02 manifest.** Acceptance is proven ONLY by the operator-captured
  peer log through `correlate`. No "accepted" inferred from Ade's self-accept / `ForgeSucceeded` /
  `block_received` / any wire-success signal (`N2` / `RO-LIVE-06`).
- **Do not forge across an epoch boundary with a stale `eta0`** (`DC-EPOCH-03` — off-epoch fails
  closed; cross-epoch production is a separate nonce-roll cluster).
- No new **BLUE authority / canonical type / WAL/checkpoint format**; no parallel serializer (only
  CN-WIRE-08 tag-24); no second serve authority (only `ServedChainHandle::push_atomic`).
- **Hard line:** if the live wiring needs a containment relaxation, a handoff-fence relaxation, a
  `NodeBlockSource` plugin point, a synthetic manifest, or a new "peer accepted" rule — **stop and
  re-scope.**

## Replay obligations (scoped)
- **R1** — a captured bounded live-feed transcript (S1, the docker/preprod smoke) + the recovered
  checkpoint + WAL replays to **byte-identical post-state AND byte-identical forge sequence**
  (extends the `DC-NODE-05` / N-F-A / N-F-E replay-equivalence clause to the live-wired feed: the
  live wire is the nondeterministic *source*; once captured as canonical ordered bytes, replay
  reproduces). Unit/transcript-tested, not a new corpus lane.
- **R2** — `correlate(forged_artifact, peer_accept_log) → byte-identical BA02Outcome` on replay
  (the existing `RO-LIVE-06` property; unchanged).
- No new BLUE canonical type. Acceptance scoped to touched crates (`ade_node`, `ade_runtime`,
  consumed `ade_network`/`ade_ledger`/`ade_codec`) — not the full `ade_testkit` corpus lane.

## Registry impact (at close)
- `RO-LIVE-01` — `strengthened_in += "PHASE4-N-F-G-C"`; **stays `partial`** (live half
  `blocked_until_operator_stake_available`). **No status flip; no new rule.**
- `CN-CONS-06` live half stays `blocked_until_operator_stake_available`.
- Candidate gate broadening: `ci_check_served_chain_handoff_fence.sh` (scope + guard-3 allow-list) —
  rebound to `DC-NODE-06`, never weakened.
- **Not added here:** any "peer accepted the block" rule (acceptance is operator/release evidence,
  never a runtime invariant). No new canonical type. No `RO-LIVE-06` flip (synthetic fixtures
  cannot satisfy BA-02).

## Non-goals
- **Live peer ACCEPT** / live BA-02 closure — operator-gated, `blocked_until_operator_stake_available`,
  a separate operator-witnessed pass (C1 private testnet or C2 preprod). G-C builds the mechanical
  wiring + the corrected runbook + the evidence scaffolding only.
- Mainnet-complete serve/validation fidelity beyond the live-feed wiring + the hermetic forge→serve
  loopback proof.
- Cross-epoch production / nonce-roll (separate cluster; `DC-EPOCH-03` fails closed at the boundary).
- Mithril as a forge/validation shortcut (Mithril stays a bootstrap accelerator only).
- Grounding-doc regeneration (that's `/cluster-close`).
