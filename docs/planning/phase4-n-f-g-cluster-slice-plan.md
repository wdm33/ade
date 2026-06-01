# Cluster/Slice Plan — PHASE4-N-F-G (Ade)

> IDD Part IV artifact. Overall ordered plan for the PHASE4-N-F-G work, derived from the
> committed invariants sketch (`docs/planning/phase4-n-f-g-invariants.md`, HEAD `b063c6a4`)
> + the two declared rules **DC-NODE-06** (self-accept→serve handoff) and **DC-EPOCH-03**
> (single-epoch forge fail-closed). Overall plan only — full cluster/slice docs come from
> `/cluster-doc` + `/slice-doc`.
>
> **Split into three sub-clusters.** RO-LIVE-01 spans three distinct authority surfaces —
> **forge validity**, **serve authority**, and **live/operator evidence** — that must close
> **separately**. A single seven-slice cluster would make DC-NODE-06 + DC-EPOCH-03 wait on
> operator-gated live evidence they do not need; the split keeps each unit invariant-sized
> and lets forge-fidelity and serve-authority close mechanically while the operator-gated
> live ACCEPT stays separately named.
>
> **Hard rule (every sub-cluster, every slice):** N-F-G may **add** a served-chain handoff
> gate but must **not relax** the N-F-E/N-F-F relay-loop containment gate
> (`ci_check_node_run_loop_containment.sh`). No BA-02 / RO-LIVE closure claim until
> operator-gated evidence exists.

## Cluster Index (Dependency Order)

1. **PHASE4-N-F-G-A** — Forge fidelity on the node spine — primary invariant: the node-path
   forge consumes a **genesis-consistent** recovered seed epoch + **real** operator config,
   is **slot-aligned**, and **fails closed at the epoch boundary** (**DC-EPOCH-03**) —
   producing a block a real peer would validate, within one epoch. *Closes hermetically.*
2. **PHASE4-N-F-G-B** — Self-accept→serve handoff — primary invariant: **only** a BLUE
   self-accepted forged artifact reaches the peer's block-fetch, via a **sibling serve
   task** (**DC-NODE-06**), **without relaxing relay-loop containment**. *Closes
   hermetically.* (Depends on G-A only for a valid artifact to serve; the serve **mechanism**
   is provable on any self-accepted artifact.)
3. **PHASE4-N-F-G-C** — Live feed + operator-gated evidence — primary invariant: the forge
   is **observable** over a live feed (`ForgeTick` reachable in production) and the
   forged block is served to a real peer; **peer ACCEPT is operator-gated evidence**
   (RO-LIVE-01 partial), proven only by the peer log through `correlate`. *Mechanical wiring
   closes hermetically; the live ACCEPT stays `blocked_until_operator_stake_available`.*
   (Depends on both G-A — valid forge — and G-B — serve path.)

> **Slice IDs** are per-sub-cluster (project convention; e.g. N-F-A used S1–S4). The mapping
> to the original single-cluster logical labels: G-A S1–S4 = old S1–S4; G-B S1–S3 = old S5
> (split into handoff / serve-task / payload+gate); G-C S1–S2 = old S6 / S7.

---

## Cluster PHASE4-N-F-G-A — Forge fidelity on the node spine

- **Primary invariant:** DC-EPOCH-03 (declared; `introduced_in = "PHASE4-N-F-G-A"`). Carries
  CN-NODE-01, DC-NODE-05, DC-NODE-03, CN-OPCERT-01, CN-GENESIS-01, CN-FORGE-04 unchanged
  (strengthened at close, not weakened).
- **TCB partition:**
  - **BLUE [reused, unchanged — NO new authority]** — `ade_core::consensus::{vrf_cert,
    leader_schedule, nonce}`; the BLUE-validate halves of `ade_runtime::producer::{opcert_envelope,
    genesis_parser}`.
  - **GREEN [new + reused]** — the genesis-consistency **pinning harness** (`ade_testkit`);
    the slot-align map (GREEN-by-content); `run_loop_planner` (reused, unchanged).
  - **RED [new + reused]** — `ade_node::operator_forge` (real-parser rewiring + derived
    constants); the `Clock` seam (reused); reused `ade_runtime::producer::{keys, producer_shell}`;
    the fenced operator private-net setup path (reuses `seed_to_snapshot` / N-M-C extraction —
    no new bootstrap authority).
- **Cluster Exit Criteria:**
  - **CE-G-A-1 (genesis-consistency — OQ5)** — the WarmStart-recovered seed epoch is
    genesis-consistent: a pinning harness proves Ade's `praos_vrf_input(slot, eta0)` +
    leader-threshold inputs (stake / ASC / vrf_keyhash) match a **committed private-genesis
    fixture**'s genesis-derived view, BEFORE any live forge; the operator private-net setup
    path is fenced + reuses existing extraction authorities (no new bootstrap authority —
    CN-NODE-01 intact).
  - **CE-G-A-2a (current protocol params source — PO-1 split)** — the recovered ledger's
    `protocol_params` are **oracle-captured current values installed at seed/import time**, never
    `LedgerState::default()` (major 2) and never genesis-initial; tests prove the import bundle
    carries current pparams + `build_seed_ledger`/runner-ledger install them, warm-start recovery
    preserves them, the forge call site sees the expected current `protocol_major`/`protocol_minor`
    (failing on the old default 2.0), and there is no operator runtime override / genesis-initial /
    default fallback. Single bootstrap (CN-NODE-01), oracle-sourced, replay-faithful. *(gates S2.)*
  - **CE-G-A-2 (ingress fidelity — G1/G2/G4)** — the operator-forge ingress loads the real
    cardano-cli `node.opcert` text-envelope (`parse_opcert_envelope`) + `shelley-genesis.json`
    (`parse_shelley_genesis`), retiring the simple-JSON forms on the node path; `prev_opcert_counter`
    derives from the loaded opcert, `protocol_version` / `ProtocolParameters` from the **S2a
    recovered current view** (not defaults, not genesis-initial).
  - **CE-G-A-3 (slot alignment — G7)** — the forge slot tracks the peer's live wall-clock
    slot via the genesis `slot_zero_time + slot_length` pure map at the single RED `Clock`
    seam; `SlotDrift` fails closed (not swallowed); same (genesis, millis) → same `SlotNo`.
  - **CE-G-A-4 (epoch fail-closed — DC-EPOCH-03)** — a candidate forge slot beyond the
    recovered seed epoch fails closed (cannot forge / serve / sign); a test proves an
    off-epoch slot yields a structured fail-closed outcome and no served/signed block; the
    stale-eta0 sign is unrepresentable on this path.
- **Slices:**
  - **S1 — Genesis-consistent recovered seed epoch + pinning harness (the first hard fork)** —
    invariant: the WarmStart-recovered seed epoch is genesis-consistent; a pinning harness
    cross-checks Ade's leader-VRF-input + threshold inputs vs a committed private-genesis
    fixture; the operator private-net setup path is fenced + reuses `seed_to_snapshot` / N-M-C
    extraction (no new bootstrap authority — CN-NODE-01 intact) — addresses: **CE-G-A-1** —
    TCB: **GREEN** (pinning harness) + **RED** (fenced setup) + consume **BLUE**.
    > **Hard-fork contingency:** if S1 cannot prove the WarmStart-recovered seed epoch is
    > genesis-consistent for the private fixture, **G-A stops and inserts S1b** (a private-net
    > bootstrap/anchor slice) **before any serve/live work** in G-B/G-C. Flagged, not expected
    > (the sketch hypothesis is that WarmStart-from-operator-pre-seeded-store suffices).
  - **S2a — Current protocol parameters source (PO-1 split, 2026-06-01)** — invariant: the
    recovered ledger's `protocol_params` (and thus current `protocol_version` =
    `protocol_major`/`protocol_minor`) are the **oracle-captured current values installed at
    seed/import time**, NEVER `LedgerState::default()` (major 2) and NEVER genesis-initial. Design
    A: capture current pparams from the oracle into the operator import bundle and install them
    into the recovered ledger (`build_seed_ledger` + the bootstrap runner ledger `bootstrap.rs:177`),
    so the recovered current view is a truthful source — addresses: **CE-G-A-2a** — TCB: **RED**
    (seed import + ledger construction) + consume **BLUE**. *(Inserted because the S2 PO-1 entry
    check proved the recovered ledger carried the stale default major 2 — `build_seed_ledger` =
    `LedgerState::new(Conway)` + UTxO only. S2 is blocked on S2a.)*
  - **S2 — Real opcert/genesis ingress + derived constants** — invariant: the operator-forge
    ingress loads the real `node.opcert` envelope + `shelley-genesis.json` via the dormant
    parsers (retire simple-JSON on the node path); `prev_opcert_counter` derives from the loaded
    opcert, `protocol_version` / `pparams` from the **S2a recovered current view** — addresses:
    **CE-G-A-2** — TCB: **RED** (`operator_forge`) + consume **BLUE**. **Depends on S2a:** may
    proceed only after S2a proves the recovered ledger carries current pparams/protocol_version
    (re-run the PO-1 entry check — it must pass).
  - **S3 — Slot alignment to live wall-clock + SlotDrift fail-closed** — invariant: the forge
    slot tracks the peer's live slot via the genesis slot map at the `Clock` seam; `SlotDrift`
    fails closed — addresses: **CE-G-A-3** — TCB: **RED** (clock seam) + **GREEN/BLUE** (pure
    map).
  - **S4 — Epoch-boundary forge fail-closed (DC-EPOCH-03)** — invariant: a slot past the
    recovered seed epoch fails closed (cannot forge / serve / sign); off-epoch test proves a
    structured fail-closed outcome + no block — addresses: **CE-G-A-4** — TCB: **GREEN/BLUE**
    guard + consume the **BLUE** nonce authority.
- **Replay obligations:** no new authoritative state, no new canonical types, no new BLUE
  replay corpus. Obligation: same recovered state + real ingress + injected clock ⇒
  byte-identical forge decision (extends DC-NODE-05's replay clause to real-ingress +
  slot-aligned inputs); the slot map (pure) and the epoch fail-closed boundary are
  unit-tested for determinism. The pinning harness is a proof, not replay corpus.
- **FC/IS partition:** BLUE consumed-unchanged (456 canonical types expected unchanged);
  GREEN gains the pinning harness + slot map; RED gains the real-parser ingress + derived
  constants + fenced setup. No BLUE→RED edge.
- **Close point:** G-A closes when CE-G-A-1, CE-G-A-2a, CE-G-A-2, CE-G-A-3, CE-G-A-4 are green in
  CI. DC-EPOCH-03 flips `declared→enforced`; `strengthened_in += "PHASE4-N-F-G-A"` on CN-OPCERT-01,
  CN-GENESIS-01, DC-NODE-05, and (S2a) DC-LEDGER-10 + CN-NODE-01 + DC-CINPUT-02b (current-pparams
  source faithfulness; a dedicated source-of-current-ledger-state rule is a cluster-close candidate
  if the strengthening is judged insufficient).

---

## Cluster PHASE4-N-F-G-B — Self-accept→serve handoff

- **Primary invariant:** DC-NODE-06 (declared; `introduced_in = "PHASE4-N-F-G-B"`). Carries
  CN-NODE-02, CN-PROD-04, CN-CONS-07, CN-WIRE-08, DC-NODE-05 unchanged (strengthened at
  close, not weakened).
- **TCB partition:**
  - **BLUE [reused, unchanged — NO new authority]** — `ade_ledger::producer::{self_accept,
    served_chain}`; `ade_network::block_fetch::server` + the tag-24 composition;
    `ade_codec::tag24`.
  - **GREEN [new]** — the typed self-accepted-artifact **handoff fence** (constructor-gated).
  - **RED [new + reused]** — the **sibling serve task** (`ade_node` + `ade_runtime::network::{n2n_listener,
    mux_pump, n2n_server, served_chain_handle::push_atomic}`).
- **Cluster Exit Criteria:**
  - **CE-G-B-1 (handoff fence)** — only a BLUE self-accepted forged artifact (the
    `ForgeSucceeded` payload) may enter the sibling serve task, via a typed,
    constructor-fenced handoff; the serve task **cannot** be fed raw forged bytes, a failed
    forge output (`ForgeNotLeader` / `ForgeFailed`), a self-declared acceptance flag, or a
    peer-verdict substitute (the dangerous "serve an unaccepted artifact" state is
    unrepresentable).
  - **CE-G-B-2 (sibling serve task; containment unchanged)** — served-chain mutation happens
    ONLY in the sibling serve task via the single `ServedChainHandle::push_atomic`; the
    relay-loop body performs no serve / admit / gossip / block-fetch / durable-tip mutation
    (the handoff is a typed channel send of a constructor-fenced artifact, not served-chain
    mutation and not block-fetch serving); `ci_check_node_run_loop_containment.sh` stays
    **byte-/semantically unchanged**.
  - **CE-G-B-3 (block-fetch payload/envelope proof + handoff gate)** — a block-fetch
    `RequestRange` returns a `MsgBlock` whose **payload preserves the self-accepted forged
    block bytes** and applies the single CN-WIRE-08 tag-24 envelope authority (decode round-
    trips to the self-accept input); a **new** served-chain handoff CI gate bans raw forge
    bytes / failed outcomes / self-declared acceptance / peer-verdict substitutes from the
    serve-task ingress (additive — the relay-loop containment gate is untouched).
- **Slices:**
  - **S1 — Typed self-accepted-artifact handoff** — invariant: a typed, constructor-fenced
    handoff carrier whose ONLY provenance is a `ForgeSucceeded` outcome (CN-FORGE-01 emits it
    only when BLUE `self_accept` accepts); no constructor from raw bytes / failed outcome /
    self-declared flag / peer verdict — addresses: **CE-G-B-1** — TCB: **GREEN** (constructor
    fence) + consume **BLUE** (`self_accept`).
  - **S2 — Sibling serve task** — invariant: a sibling task (listener + `block_fetch::server`
    + `push_atomic`) consumes the handoff and admits via the single `push_atomic`; the
    relay-loop body is byte-unchanged; the handoff is a channel send, not a serve token —
    addresses: **CE-G-B-2** — TCB: **RED** (serve task) + consume **BLUE** (`served_chain`).
  - **S3 — Block-fetch payload/envelope proof + served-chain handoff gate** — invariant:
    loopback serve → `RequestRange` → `MsgBlock` payload = self-accepted forged bytes under
    CN-WIRE-08 tag-24, decode-round-trip-identical to the self-accept input; the new handoff
    CI gate is green on the full sub-cluster diff and the containment gate is unchanged —
    addresses: **CE-G-B-3**, closes **CE-G-B-2** (containment-unchanged confirmed on the full
    diff) — TCB: **RED** test/serve + consume **BLUE** (tag-24 / `block_fetch::server`).
- **Replay obligations:** no new authoritative state, no new canonical types (the served
  payload = existing canonical forged bytes). Obligation: a given self-accepted artifact ⇒ a
  deterministic served admission + deterministic block-fetch payload bytes (unit-tested,
  not corpus).
- **FC/IS partition:** BLUE consumed-unchanged; GREEN gains the handoff fence; RED gains the
  sibling serve task. No BLUE→RED edge.
- **Close point:** G-B closes when CE-G-B-1..3 are green in CI. DC-NODE-06 flips
  `declared→enforced`; `strengthened_in += "PHASE4-N-F-G-B"` on CN-PROD-04, CN-CONS-07.

---

## Cluster PHASE4-N-F-G-C — Live feed + operator-gated evidence

- **Primary invariant:** the forge is observable over a live feed and the forged block is
  served to a real peer; **peer ACCEPT is operator-gated evidence, not a runtime
  invariant** — proven only by the peer log through `ba02_evidence::correlate`. Carries
  DC-SYNC-01/02, CN-NODE-02, RO-LIVE-06 unchanged; **scaffolds** RO-LIVE-01 (stays `partial`)
  + CN-CONS-06 live half (stays `blocked_until_operator_stake_available`). **Does not
  redefine acceptance.**
- **TCB partition:**
  - **BLUE [reused, unchanged]** — the G-A forge path + the G-B serve path.
  - **GREEN [reused]** — `ba02_evidence::correlate` (the sole `Ba02Manifest` constructor,
    unchanged).
  - **RED [new + reused]** — the **live feed wiring** (`n2n_dialer → mux_pump → session →
    WirePump` mpsc into `node_lifecycle` / `node_sync`, replacing the empty
    `in_memory(Vec::new())`); the operator-pass driver + evidence-manifest I/O.
- **Cluster Exit Criteria:**
  - **CE-G-C-1 (live feed wiring + hermetic e2e)** — the binary consumes a live peer feed
    wired as `NodeBlockSource::WirePump`; the feed reaches `Continuing` so `LoopStep::ForgeTick`
    is reachable in production; `NodeBlockSource` stays the closed 2-variant verdict-decoupled
    contract (no plugin point); the durable tip advances ONLY via `run_node_sync → pump_block`;
    a hermetic loopback e2e proves live-feed → forge → self-accept → sibling-serve →
    peer-block-fetch returns the forged block. A **bounded docker/preprod smoke validates
    live feed / session wiring only; it does NOT prove peer acceptance unless the operator
    stake condition is met** (it captures a replay-equivalent transcript as a reusable
    artifact, never a substitute for the deliverable).
  - **CE-G-C-2 (operator-gated evidence — BLOCKED)** — **CE-G-C-2 scaffolds the operator-gated
    evidence path; live peer acceptance remains blocked until operator stake/pool
    availability.** The corrected C1/C2 runbook is committed (fixing the C1-doc-identified
    `S1.md` bugs); the evidence-manifest home + sha256 cross-check is defined;
    `ba02_evidence::correlate` is wired to the operator-captured peer log; **no synthetic
    manifest is committed**; acceptance is proven ONLY by the peer's raw validation log.
    `blocked_until_operator_stake_available` (RO-LIVE-01 live half / CN-CONS-06 — named, not
    deferred).
- **Slices:**
  - **S1 — Live WirePump feed wiring + hermetic loopback forge→serve proof** — invariant:
    replace the empty `in_memory` source with a live `NodeBlockSource::WirePump` (dialer →
    mux_pump → session → mpsc); `ForgeTick` fires; `NodeBlockSource` stays closed
    verdict-decoupled; tip only via `pump_block`; a hermetic loopback e2e proves
    live-feed → forge → self-accept → sibling-serve → peer-block-fetch returns the forged
    block; a bounded docker/preprod smoke validates **wiring only** (not acceptance) and
    captures a replay-equivalent transcript — addresses: **CE-G-C-1** — TCB: **RED**
    (dialer→WirePump) + consume the G-B serve path.
  - **S2 — Operator-pass runbook + evidence manifest + correlate wiring** — invariant: commit
    the corrected C1/C2 runbook, the evidence-manifest home + sha256 cross-check, and the
    `ba02_evidence::correlate` wiring to an operator-captured peer log; the live ACCEPT stays
    `blocked_until_operator_stake_available`; no synthetic manifest committed; acceptance
    proven only by the peer log — addresses: **CE-G-C-2** (scaffolds the operator-gated path;
    the live execution flips RO-LIVE-01 later) — TCB: **RED** (runbook / evidence I/O) +
    **GREEN** (`correlate`, reused).
- **Replay obligations:** **R1** — a captured bounded live-feed transcript (S1) + recovered
  checkpoint + WAL replays to **byte-identical post-state AND forge sequence** (extends the
  DC-NODE-05 / N-F-A replay clause to the live-wired feed). `correlate` determinism is the
  existing RO-LIVE-06 property. No new BLUE canonical types.
- **FC/IS partition:** BLUE consumed-unchanged; GREEN reuses `correlate`; RED gains the live
  feed wiring + operator-pass driver. No BLUE→RED edge.
- **Close point:** G-C's **mechanical** CEs (CE-G-C-1 + the CE-G-C-2 scaffolding) close
  hermetically; the **live ACCEPT** stays `blocked_until_operator_stake_available`.
  `strengthened_in += "PHASE4-N-F-G-C"` on RO-LIVE-01 (stays `partial`); CN-CONS-06 live half
  stays `blocked_until_operator_stake_available`. No new "peer accepted" rule — acceptance is
  release/operator evidence, never a runtime invariant.

---

## Registry handling (this plan)

- **DC-EPOCH-03** `introduced_in = "PHASE4-N-F-G-A"` (corrected from the umbrella
  `"PHASE4-N-F-G"` during planning — `declared`, pre-enforcement, field-not-ID; permitted).
- **DC-NODE-06** `introduced_in = "PHASE4-N-F-G-B"` (same correction).
- **RO-LIVE-01** — **no new rule.** It remains the operator-gated release obligation,
  scaffolded/strengthened by G-C; the live half stays `blocked_until_operator_stake_available`.
- No rule weakened, no ID reassigned. CN-OPCERT-01 / CN-GENESIS-01 / CN-PROD-04 / CN-CONS-07 /
  DC-NODE-05 receive `strengthened_in` bumps at their respective sub-cluster closes.

## Notes carried into every sub-cluster

- **Authority separation (why the split):** G-A proves the node can identify **when it may
  forge** (validity); G-B proves **only self-accepted artifacts can be served** (serve
  authority); G-C proves the **live wiring + captures peer evidence**, and **does not
  redefine acceptance**.
- **Close-what-is-closeable:** G-A and G-B close mechanically without waiting on operator
  stake; only G-C's live ACCEPT is operator-gated, separately named.
- **Containment guardrail (all slices):** add a served-chain handoff gate (G-B) but never
  relax the relay-loop containment gate. The forge stays subordinate + the loop body
  byte-unchanged; serving is a sibling task.
- **Honest scope:** wire success ≠ admission ≠ acceptance; BA-02 (RO-LIVE-06) needs a real
  operator peer-accept log naming the exact Ade-forged hash through `correlate` — synthetic
  fixtures cannot satisfy it. Mithril stays a bootstrap accelerator, not a forge/validation
  shortcut.
