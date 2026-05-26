# HEAD Deltas — Ade

> **Status:** Living architectural document. Regenerated; not hand-edited.
> Regenerate via `/head-deltas <baseline>`. Baseline is declared in
> `.idd-config.json` (`head_deltas_baseline`).

> Baseline: `d62c2bc` (PHASE4-N-K close — orchestrator + `ade_node` binary; CN-NODE-01 + DC-NODE-01..04 enforced, 2026-05-26 11:47 +0700)
> HEAD: working tree on top of `d62c2bc` (PHASE4-N-L cluster-close staged, 2026-05-26)
> 1 cluster-close commit (pending), 35 files changed (28 new + 7 modified), +4,159 / −12 lines

> **Baseline shift note.** This regen narrows the baseline from the
> prior `1946573` (PHASE4-N-K handoff) used by the N-K narrative to
> `d62c2bc` (PHASE4-N-K close — the prior cluster's final commit).
> HEAD_DELTAS now narrates **only** the PHASE4-N-L cluster: the
> wire-protocol layer (mux session driver + handshake driver +
> RED tokio pumps + replay-equivalence harness) over 9 slices
> (S1 → S9). The prior cluster-by-cluster narratives (Phase 4 N-A
> through N-K) are preserved in the archived cluster docs under
> `docs/clusters/completed/` and in the SEAMS / CODEMAP / TRACEABILITY
> companions. `.idd-config.json` `head_deltas_baseline` was bumped
> from `1946573` to `d62c2bc` as part of this regen.

> **Cluster summary.** PHASE4-N-L ships the wire-protocol layer
> that turns the PHASE4-N-K binary from "boots + idles" into "drives
> a real cardano-node peer end-to-end" over 9 slices, registering
> and enforcing 8 new registry rules (`CN-SESS-01/02/03`,
> `DC-SESS-01..05`) plus 1 declared-with-open-obligation
> (`RO-LIVE-03`, `blocked_until_operator_peer_available`). The new
> rules use the `CN-SESS` / `DC-SESS` families because `CN-NET-*`
> and `DC-NET-01` were already in use (operator-topology rules).
> 4 carried-forward rules gain `strengthened_in` updates
> (`T-DET-01`, `CN-CONS-08`, `DC-NODE-01`, `DC-NODE-03`). 5 new
> CI scripts gate the cluster (`ci_check_*` script count moves
> from 61 → 66) and 1 existing script (`ci_check_clock_seam.sh`)
> is extended to cover the wire-side wall-clock-free guarantee.
> 4 open obligations remain unchanged: `RO-LIVE-01`, `RO-LIVE-02`,
> `CN-CONS-06`, `DC-STORE-09`. The cluster is purely additive to
> the existing module graph; no removals; one `OrchestratorEvent`
> variant added (`OutboundKeepAlive { peer_id }`, additive).

---

## 1. Commit Log

Verbatim from `git log --oneline --no-merges d62c2bc..HEAD` (HEAD is
the staged cluster-close commit — slice-by-slice commits are
collapsed into a single cluster-close commit per the project's
cluster-close discipline).

| Hash | Type | Summary |
|------|------|---------|
| (pending) | feat | feat(network+runtime): PHASE4-N-L close — wire-protocol session driver (S1..S9), flip CN-SESS-01/02/03 + DC-SESS-01..05 to enforced |

All cluster work (S1 CI gates for mux frame + handshake closure +
closed mini-protocol id enum, S2 GREEN `session::core::step` +
`SessionState` type-state, S3 GREEN `session::demux` frame buffer,
S4 GREEN `session::handshake_driver` over opaque `Transport`,
S5 RED full-duplex bounded-queue `MuxTransportHandle`, S6 RED
`mux_pump` per-connection tokio task, S7 RED `n2n_dialer` outbound
TCP + handshake driver call, S8 RED `keep_alive_session` Clock-driven
ping pump, S9 `session_replay_equivalence` integration test) is
contained in a single cluster-close commit; per-slice context lives
under `docs/clusters/completed/PHASE4-N-L/N-L-S{1..9}.md`. No fix /
docs / chore / refactor commits in this window — the cluster is a
single linear feature stream.

---

## 2. New Modules

| Module | Color | Purpose | Key sub-paths | Added in (cluster/slice) |
|--------|-------|---------|---------------|--------------------------|
| `ade_network::session` | **GREEN** | Wire-session reducer + supporting state, event, demux, and handshake-driver sub-modules. `session::core::step` is the SOLE pub reducer over `(SessionState, ByteChunkIn) -> (SessionState, Vec<SessionEffect>)` (CN-SESS-03). Pure — no `tokio`, no `SystemTime`/`Instant`, no `rand`. Drives handshake-before-traffic (DC-SESS-01), closed mini-protocol id dispatch (DC-SESS-02), per-protocol ordering (DC-SESS-03), and wire-layer clock-injection seam on the GREEN side (DC-SESS-05). | `event.rs` (`AcceptedMiniProtocol` closed registry, `ByteChunkIn`, `SessionEffect`, `SessionError`, `HandshakeRole`), `state.rs` (`SessionState` Handshaking/Connected type-state), `demux.rs` (`FrameBuffer` partial-frame accumulator + per-protocol fanout), `core.rs` (`step` reducer), `handshake_driver.rs` (`Transport` trait + `run_n2n_handshake_initiator` / `run_n2n_handshake_responder`), `mod.rs` (barrel re-exports — was a 5-line placeholder at baseline, now the package barrel) | PHASE4-N-L / S1 + S2 + S3 + S4 |
| `ade_runtime::network::mux_pump` | **RED** | Per-connection tokio task bridging a `MuxTransportHandle` (full-duplex bounded-queue RED transport from `ade_network::mux::transport`) to the GREEN `session::core::step` reducer. Forwards `SessionEffect`s to the orchestrator inbox (bounded `mpsc::Sender<OrchestratorEvent>`); enforces backpressure via `try_send` → `TransportError::BackpressureExceeded` (DC-SESS-04). Single async run-loop; exits on transport close or orchestrator drop. | `mux_pump.rs` (`MuxPump`, `MuxPumpError`, `spawn_mux_pump`) | PHASE4-N-L / S6 |
| `ade_runtime::network::n2n_dialer` | **RED** | Outbound TCP dialer: opens a `TcpStream`, wraps it in `MuxTransportHandle::spawn_duplex`, runs the handshake driver, and on success emits an `OrchestratorEvent::PeerConnected` and spawns a `MuxPump`. Closed `DialError` sum maps TCP / handshake / transport failures. | `n2n_dialer.rs` (`N2nDialer`, `DialError`, `dial_outbound`) | PHASE4-N-L / S7 |
| `ade_runtime::orchestrator::keep_alive_session` | **RED** | Clock-driven keep-alive ping pump. Each tick from `Clock::tick_stream` emits an `OrchestratorEvent::OutboundKeepAlive { peer_id }` into the orchestrator inbox. SOLE driver of keep-alive cadence; the GREEN session core never reads wall-clock. Drives DC-SESS-05 end-to-end via the PHASE4-N-K `Clock` seam. Cadence pinned at `KeepAliveCadence::DEFAULT = 60_000ms`. Exits when the orchestrator inbox is dropped. | `keep_alive_session.rs` (`KeepAliveSession`, `KeepAliveCadence`) | PHASE4-N-L / S8 |

No new workspace crates. Workspace member count unchanged.
`ade_network::session` existed at baseline as a 5-line `mod.rs`
placeholder; PHASE4-N-L S2..S4 elevate it to a populated GREEN
sub-tree with 5 source files.

Cross-reference: the new modules must be reflected in CODEMAP §GREEN
(`ade_network::session::{event, state, demux, core, handshake_driver}`)
and §RED (`ade_runtime::network::{mux_pump, n2n_dialer}`,
`ade_runtime::orchestrator::keep_alive_session`, plus the
`ade_network::mux::transport` RED-extension noted in §3). If absent
at the next read, CODEMAP is stale — regenerate via `/codemap`.

---

## 3. Modules Modified

| Module | Scope | Key changes |
|--------|-------|-------------|
| `ade_network::mux::transport` | +198 / −3 lines | RED full-duplex extension. Original `MuxTransport` / `open_tcp` API retained byte-identically for existing N-G consumers. New: `spawn_duplex` returning `MuxTransportHandle` (paired bounded `mpsc::Receiver<MuxFrame>` for inbound + `mpsc::Sender<MuxFrame>` for outbound), `TransportError` closed sum with `BackpressureExceeded`, `DuplexCapacity::DEFAULT`, `JoinHandle` ownership of the spawned reader/writer tasks. Bounded-queue overflow surfaces as `TransportError::BackpressureExceeded` (no silent drop) — DC-SESS-04 evidence. Re-classified in core_paths note as RED (already RED at baseline; the extension keeps it RED). |
| `ade_network::session::mod.rs` | +37 / −4 lines | Rewritten from a 5-line module placeholder into the package barrel: `pub mod {core, event, state, demux, handshake_driver};` + public re-exports. Module-level doc comment now anchors the cluster scope (CN-SESS-01..03 + DC-SESS-01..05). |
| `ade_network/Cargo.toml` | +3 / −1 lines | tokio features extended: `sync` (for bounded `mpsc` channels in `mux::transport`'s duplex extension) and `rt-multi-thread` (for `tokio::spawn` in the duplex reader/writer task pair). Confined to RED files by `ci/ci_check_session_core_closure.sh` and `ci/ci_check_clock_seam.sh` extension — the GREEN `session/*.rs` files MUST NOT import `tokio::*`. |
| `ade_runtime::network::mod.rs` | +5 lines | Adds `pub mod mux_pump; pub mod n2n_dialer;` declarations wiring the two new RED runner files. |
| `ade_runtime::orchestrator::mod.rs` | +1 line | Adds `pub mod keep_alive_session;` declaration wiring the new RED keep-alive pump. |
| `ade_runtime::orchestrator::core.rs` | +8 lines | One new branch in the reducer: `OrchestratorEvent::OutboundKeepAlive { peer_id }` is recorded into state (no immediate `OrchestratorEffect` emission — keep-alive frame encoding is deliberately deferred to a future session-side rule). Purely additive, no existing branch modified. |
| `ade_runtime::orchestrator::event.rs` | +8 lines | New `OrchestratorEvent::OutboundKeepAlive { peer_id: PeerId }` variant on the closed inbound-event sum. Additive — the existing variants are byte-identical at HEAD. Variant doc comment references `DC-NET-05`; the registered rule is `DC-SESS-05` (see Anomalies §). |
| `docs/ade-invariant-registry.toml` | +221 / −0 lines | 9 new PHASE4-N-L rules appended (`CN-SESS-01/02/03`, `DC-SESS-01/02/03/04/05`, `RO-LIVE-03`); 8 of the 9 land at `status = "enforced"` with `code_locus` + `tests` + `ci_script` + `evidence_notes` populated; `RO-LIVE-03` lands at `status = "declared"` with `open_obligation = "blocked_until_operator_peer_available"`. Each new rule carries `strengthened_in = ["PHASE4-N-L"]`. **Gap**: the closure record promises that 4 existing rules (`T-DET-01`, `CN-CONS-08`, `DC-NODE-01`, `DC-NODE-03`) gain `strengthened_in += "PHASE4-N-L"`; at HEAD these 4 rules' `strengthened_in` arrays terminate at `"PHASE4-N-K"` and do **not** include `"PHASE4-N-L"`. See Anomalies §. |

No other source modules were touched. The cluster is **purely
additive** to the existing module graph — one new GREEN sub-tree
(`ade_network::session`), three new RED runner files, one extended
RED transport file, five new CI scripts, one new integration test,
one registry append. No refactors, no API breakage, no removals
from any existing module.

---

## 4. Feature Flags

No Cargo `[features]` table is declared in `ade_network`,
`ade_runtime`, or any other workspace crate at baseline or at HEAD.
No new feature flags introduced; no existing feature flags
modified or removed.

The cluster adds two new closed constants that are **not** Cargo
features but are referenced here for completeness — they pin
deterministic behaviour at the RED-runner boundary:

| Constant | Module | Purpose | Status |
|----------|--------|---------|--------|
| `DuplexCapacity::DEFAULT` | `ade_network::mux::transport` | Closed default bounded-queue capacity for the `MuxTransportHandle` inbound + outbound channel pair. Overflow → `TransportError::BackpressureExceeded` (DC-SESS-04). | **New** since baseline |
| `KeepAliveCadence::DEFAULT.interval_ms = 60_000` | `ade_runtime::orchestrator::keep_alive_session` | Closed default keep-alive ping cadence (60 s). Drives `Clock::tick_stream` cadence in `KeepAliveSession::run`. | **New** since baseline |

No coupling between the two: each is a stand-alone struct constant.
Both are exercised by deterministic tests (`mux_transport_duplex_*`
+ `keep_alive_*`).

The two new tokio features (`sync`, `rt-multi-thread`) on the
`ade_network` crate are **not** gated by a Cargo feature — they
are structurally confined by CI:
`ci/ci_check_session_core_closure.sh` greps `crates/ade_network/src/session/`
for any `tokio::` import and fails the build if one appears.
`ci/ci_check_clock_seam.sh` (extended in this cluster) greps the
same paths for `SystemTime`, `Instant`, `tokio::time`, `tokio::spawn`,
and `rand::*` and fails otherwise. The structural confinement is
mechanical (CI-enforced), not Cargo-feature-enforced.

---

## 5. CI Checks

### PHASE4-N-L wire-protocol closure — 5 new scripts + 1 extended (`ci_check_*.sh` 62nd – 66th)

| Check | Status | What it checks |
|-------|--------|----------------|
| `ci/ci_check_mux_frame_closure.sh` | **New** (S1) — script 62 | Enforces `CN-SESS-01`: greps every `.rs` under `crates/` for `pub fn encode_frame` and `pub fn decode_frame`; asserts the SOLE site for both is `crates/ade_network/src/mux/frame.rs`. Mirrors `ci_check_admitted_block_closure.sh` shape (single-pub-fn-site closure). |
| `ci/ci_check_handshake_closure.sh` | **New** (S1) — script 63 | Enforces `CN-SESS-02`: greps every `.rs` under `crates/` for `pub fn n2n_transition` and `pub fn n2c_transition`; asserts the SOLE site for both is `crates/ade_network/src/handshake/transition.rs`. |
| `ci/ci_check_session_core_closure.sh` | **New** (S2) — script 64 | Enforces `CN-SESS-03` + `DC-SESS-01` (type-state half) + `DC-SESS-05` (session-side wall-clock-free): asserts `session::core::step` is the SOLE pub reducer fn under `crates/ade_network/src/session/`; asserts `SessionState::{Handshaking, Connected}` type-state present; negative grep on `tokio::`, `SystemTime`, `Instant`, `tokio::time`, `rand::*` across all `session/*.rs`. |
| `ci/ci_check_mini_protocol_id_registry_closed.sh` | **New** (S1) — script 65 | Enforces `DC-SESS-02`: asserts the `AcceptedMiniProtocol::from_id` body closes with `_ => None`; asserts the dispatch site in `session::core` has no wildcard accept arm; positive grep that every variant of `AcceptedMiniProtocol` is named in the from_id match. |
| `ci/ci_check_session_no_unbounded.sh` | **New** (S5+S6+S7+S8) — script 66 | Enforces `DC-SESS-04`: negative grep for `mpsc::unbounded_channel` across the wire-session file set (`crates/ade_network/src/session/`, `crates/ade_network/src/mux/transport.rs`, `crates/ade_runtime/src/network/mux_pump.rs`, `crates/ade_runtime/src/network/n2n_dialer.rs`, `crates/ade_runtime/src/orchestrator/keep_alive_session.rs`); positive grep that `MuxTransportHandle`'s constructor takes a bounded `DuplexCapacity`. |
| `ci/ci_check_clock_seam.sh` | **Modified** (S2 + S8) | Extended for `DC-SESS-05`: in addition to its existing `DC-NODE-03` body (orchestrator core wall-clock-free; `clock.rs` sole `SystemTime::now`/`Instant::now` site in `ade_runtime`), Rule 3 adds a negative grep over `crates/ade_network/src/session/*.rs` for any of `SystemTime`, `Instant`, `tokio::time`, `tokio::spawn`, `rand::*`, `rand_core`. Script header references the new rule as `DC-NET-05`; the registered rule is `DC-SESS-05` (see Anomalies §). |

Total CI script count: **61 → 66** (`ci/ci_check_*.sh`). 5 new
scripts; 1 modified script; no removals — the cluster strictly
appends + extends.

TRACEABILITY cross-reference: each of the 5 new scripts appears as
a `ci_script` on at least one rule in
`docs/ade-invariant-registry.toml` (8 new `ci_script ↔ rule`
edges across the 8 newly-enforced PHASE4-N-L rules; the extended
`ci_check_clock_seam.sh` adds a 9th edge to `DC-SESS-05`).
Re-traced via `ci/ci_check_constitution_coverage.sh` — expected to
pass at HEAD.

---

## 6. Canonical Type Registry Delta

n/a — `.idd-config.json` `canonical_type_registry` is null.
Canonical-type rules live inline in the invariant registry under
family `T`.

**PHASE4-N-L introduced new closed sum types** in support of the
session reducer (GREEN) and the RED runners:

- `ade_network::session::event::AcceptedMiniProtocol` — closed
  registry of accepted mini-protocol ids (chain-sync, block-fetch,
  keep-alive, ...); `from_id` closes with `_ => None` (DC-SESS-02).
- `ade_network::session::event::ByteChunkIn` — inbound byte-chunk
  variant (per-peer raw bytes from the transport).
- `ade_network::session::event::SessionEffect` — closed outbound
  effect sum (`SendOutboundFrame`, `DeliverPeerFrame`,
  `HandshakeCompleted`, ...).
- `ade_network::session::event::SessionError` — closed reducer-
  failure sum (`PreHandshakeMiniProtocolFrame`,
  `PostHandshakeHandshakeFrame`, `UnknownMiniProtocolId { id }`, ...);
  every variant is peer-fatal.
- `ade_network::session::event::HandshakeRole` — closed
  `Initiator`/`Responder` discriminant.
- `ade_network::session::state::SessionState` — `Handshaking` /
  `Connected` type-state (DC-SESS-01 compile-time guarantee).
- `ade_network::mux::transport::TransportError` — closed RED-side
  failure sum, including `BackpressureExceeded` (DC-SESS-04).
- `ade_runtime::network::mux_pump::MuxPumpError` — closed pump-task
  failure sum.
- `ade_runtime::network::n2n_dialer::DialError` — closed outbound-
  dial failure sum (TCP / handshake / transport).

Plus the canonical authority sites that are now SOLE-authority
(CN-SESS-01 / CN-SESS-02 / CN-SESS-03):

- `ade_network::mux::frame::{encode_frame, decode_frame}` — SOLE
  mux-frame codec pair (CN-SESS-01).
- `ade_network::handshake::{n2n_transition, n2c_transition}` —
  SOLE handshake reducers (CN-SESS-02).
- `ade_network::session::core::step` — SOLE session reducer
  (CN-SESS-03).

**Removals: 0** (expected under append-only discipline).

One additive variant on a previously-closed sum:
- `ade_runtime::orchestrator::event::OrchestratorEvent::OutboundKeepAlive { peer_id: PeerId }`
  — additive extension of the closed inbound-event sum; no existing
  variant modified; all match sites in the orchestrator handled.

Exact whole-project type recount belongs to the TRACEABILITY regen
that follows this HEAD_DELTAS.

---

## 7. Normative Rule Delta

The project's invariant registry tracks structured rules (TOML),
not prose normative-doc rules; this section reports on it.

- Rules at baseline (`d62c2bc:docs/ade-invariant-registry.toml`): **214**
- Rules at HEAD (`HEAD:docs/ade-invariant-registry.toml`): **223**
- Net additions: **9** (`CN-SESS-01/02/03`, `DC-SESS-01..05`, `RO-LIVE-03`).
- Removals: **0** (expected under append-only discipline; clean).

- **New rules (9) at HEAD:**
  - **`CN-SESS-01` `enforced`** — single mux-frame authority
    (`encode_frame`/`decode_frame` SOLE pair in
    `ade_network::mux::frame`). `ci/ci_check_mux_frame_closure.sh`.
  - **`CN-SESS-02` `enforced`** — single handshake authority
    (`n2n_transition` / `n2c_transition` SOLE in
    `ade_network::handshake::transition`). `ci/ci_check_handshake_closure.sh`.
  - **`CN-SESS-03` `enforced`** — single session reducer authority
    (`session::core::step` SOLE pub reducer).
    `ci/ci_check_session_core_closure.sh`.
  - **`DC-SESS-01` `enforced`** — handshake-before-traffic
    (Handshaking/Connected type-state forbids pre-handshake
    mini-protocol frames; peer-fatal on violation).
    `ci/ci_check_session_core_closure.sh`.
  - **`DC-SESS-02` `enforced`** — closed mini-protocol id registry
    (`AcceptedMiniProtocol::from_id` closes with `_ => None`;
    dispatch has no wildcard accept). `ci/ci_check_mini_protocol_id_registry_closed.sh`.
  - **`DC-SESS-03` `enforced`** — session replay equivalence + per-
    (peer, mini_protocol_id) ordering (`tests/session_replay_equivalence.rs`).
    `ci/ci_check_session_core_closure.sh`.
  - **`DC-SESS-04` `enforced`** — backpressure discipline (bounded
    `mpsc` + `TransportError::BackpressureExceeded`; no unbounded
    channels in the wire-session file set).
    `ci/ci_check_session_no_unbounded.sh`.
  - **`DC-SESS-05` `enforced`** — wire-layer clock-injection seam
    (`session/*.rs` wall-clock-free; `KeepAliveSession` routes via
    `Clock` trait). `ci/ci_check_clock_seam.sh` (extended).
  - **`RO-LIVE-03` `declared`** — live tip-following pass (operator
    runs `ade_node --peer ADDR` against a private cardano-node
    peer + captures a 30-minute JSONL log). `open_obligation =
    "blocked_until_operator_peer_available"`. The mechanical wire
    layer is ready; live operator pass is a one-slice follow-on
    cluster (`PHASE4-N-L-LIVE`).

- **Strengthenings recorded at HEAD by PHASE4-N-L (8 of the 9 new
  rules carry `strengthened_in = ["PHASE4-N-L"]`):**
  - All 8 newly-enforced rules above carry the marker.
  - `RO-LIVE-03` does not (it remains `declared`; strengthening
    semantics are reserved for enforced rules).

- **Strengthenings PROMISED by the closure record but NOT applied
  at HEAD (gap — see Anomalies §):**
  - `T-DET-01.strengthened_in += "PHASE4-N-L"` — promised; at HEAD
    the array terminates at `"PHASE4-N-K"`.
  - `CN-CONS-08.strengthened_in += "PHASE4-N-L"` — promised; at
    HEAD the array terminates at `"PHASE4-N-K"`.
  - `DC-NODE-01.strengthened_in += "PHASE4-N-L"` — promised; at
    HEAD the array contains only `["PHASE4-N-K"]`.
  - `DC-NODE-03.strengthened_in += "PHASE4-N-L"` — promised; at
    HEAD the array contains only `["PHASE4-N-K"]`.

- **Open obligations status at HEAD:**
  - **`RO-LIVE-03.open_obligation = "blocked_until_operator_peer_available"`**
    — **NEW** since baseline. Operator-action work; the cluster
    ships the mechanical layer that makes the pass runnable.
  - **`RO-LIVE-02.open_obligation = "blocked_until_operator_peer_available"`**
    — carried forward from PHASE4-N-H. Unchanged.
  - **`RO-LIVE-01.open_obligation = "blocked_until_operator_peer_available"`**
    — carried forward from PHASE4-N-G. Unchanged.
  - **`CN-CONS-06.open_obligation = "blocked_until_operator_stake_available"`**
    — carried forward from PHASE4-N-C. Unchanged.
  - **`DC-STORE-09.open_obligation = "snapshot_schema_migration_follow_on_cluster"`**
    — carried forward from PHASE4-N-K. Unchanged.
  - **`OP-OPS-04.open_obligation`** (Sum6KES skey loader) — carried
    forward; unchanged.

---

## Anomalies and Cross-Reference Warnings

- **No canonical-type or invariant-rule removals.** Append-only
  discipline preserved across the cluster.
- **No conventional-commits violations.** The pending cluster-close
  commit follows the `feat(network+runtime): PHASE4-N-L close —
  ...` scope+suffix pattern.
- **Gap (registry strengthening discipline)**: the closure record
  at `docs/clusters/completed/PHASE4-N-L/CLOSURE.md` §
  "Existing rules strengthened" promises that
  `T-DET-01`, `CN-CONS-08`, `DC-NODE-01`, and `DC-NODE-03` each
  gain `PHASE4-N-L` in their `strengthened_in` arrays. At HEAD,
  none of the four arrays include `"PHASE4-N-L"` — they still
  terminate at `"PHASE4-N-K"` (or earlier for rules that were
  not strengthened by N-K). The registry rule-counts are correct
  and consistent (9 new rules, 0 removals); only the
  `strengthened_in` cross-references on the four existing rules
  are missing. Apply the 4 array appends before flipping the
  registry-coverage CI gate to enforced on these rules' next
  read.
- **Anomaly (in-source ID mismatch)**: three sites in source +
  one CI script header reference the wire-layer clock-injection
  rule as `DC-NET-05`, but the registered rule id is `DC-SESS-05`
  (the cluster deliberately used the `*-SESS-*` family to avoid
  collision with the operator-topology `DC-NET-01`). Affected
  sites:
    - `crates/ade_runtime/src/orchestrator/event.rs:65` doc comment.
    - `crates/ade_runtime/src/orchestrator/core.rs:150` doc comment.
    - `crates/ade_runtime/src/orchestrator/keep_alive_session.rs:11` module doc.
    - `ci/ci_check_clock_seam.sh` header + 4 echo strings.
  None of these is a CI-gating string match (the script's rule-3
  body checks code patterns, not the rule id literal), so this
  does not break enforcement. Cosmetic / traceability hygiene —
  reconcile to `DC-SESS-05` to avoid registry-grep confusion.
- **CODEMAP cross-reference**: the four new modules (§2) must
  appear in CODEMAP. If absent at the next read, CODEMAP is stale
  — regen via `/codemap`. Specifically: GREEN entries for
  `ade_network::session::{event, state, demux, core, handshake_driver}`;
  RED entries for `ade_runtime::network::{mux_pump, n2n_dialer}`,
  `ade_runtime::orchestrator::keep_alive_session`, plus the
  RED-extension annotation on `ade_network::mux::transport` (now
  full-duplex bounded-queue).
- **SEAMS cross-reference**: the new session-event surface
  (`AcceptedMiniProtocol` closed registry + `SessionEffect` /
  `SessionError` / `HandshakeRole` closed sums) is a closed-
  registry attachment surface; SEAMS should classify it under
  closed registries. The additive `OrchestratorEvent::OutboundKeepAlive`
  variant is an inbound-event surface extension — SEAMS should
  note the closed sum grew by 1 variant. Regen via `/seams` if
  absent.
- **TRACEABILITY cross-reference**: the 5 new CI scripts + 1
  extended script (§5) and the 9 new rules (§7) must appear in
  TRACEABILITY. If absent at the next read, regen via
  `/traceability`.
- **Honest-scope note (RED runner)**: the three new RED tokio
  runner files (`mux_pump.rs`, `n2n_dialer.rs`,
  `keep_alive_session.rs`) and the RED `ade_network::mux::transport`
  extension are the mechanical-layer plumbing that makes the
  live pass runnable. The actual operator pass against a private
  cardano-node peer + JSONL log capture is `RO-LIVE-03`'s
  `open_obligation = "blocked_until_operator_peer_available"`,
  closed by the one-slice follow-on cluster `PHASE4-N-L-LIVE`.
  Block production over a real peer is enabled by the same mux
  fabric via the existing PHASE4-N-G server pump path (no new
  block-production code in this cluster).

---

## Generation Notes

This regen was produced by `/head-deltas d62c2bc` against the
staged PHASE4-N-L cluster-close working tree (uncommitted at regen
time — `git add -N` applied to surface new files in `git diff`).
The baseline was shifted from the prior N-K handoff (`1946573`) to
the PHASE4-N-K close (`d62c2bc`) per the cluster-close cadence —
each grounding regen baselines at the previous cluster's close so
the narrative stays narrow and reviewable per-cluster.
`.idd-config.json` `head_deltas_baseline` was bumped from `1946573`
to `d62c2bc` as part of this regen. Future regens should continue
to baseline at the **previous** cluster's close.

Mechanical inputs:
- `git log --oneline --no-merges d62c2bc..HEAD` (cluster-close
  commit pending) → §1.
- `git diff --name-status d62c2bc -- crates/ ci/ docs/` (with
  `git add -N` on new files) → §2 + §3.
- `git diff --numstat d62c2bc -- crates/<crate>/` → §3 scope column.
- `crates/ade_network/Cargo.toml` diff → §4 (no Cargo features
  changed; tokio `sync` + `rt-multi-thread` features added,
  structural CI confinement noted).
- `ls ci/ci_check_*.sh` vs `git ls-tree -r --name-only d62c2bc ci/`
  → §5 (61 → 66; 1 modified).
- `git diff d62c2bc -- docs/ade-invariant-registry.toml` + entry
  count (`grep -c '^\[\[rules\]\]'`) → §7 (214 → 223; 9 new
  rules, 4 missing strengthening markers — surfaced as a gap).
- `docs/clusters/completed/PHASE4-N-L/CLOSURE.md` →
  cluster-summary header, Modules Modified narrative, registry-
  delta promises (cross-checked against the registry).
