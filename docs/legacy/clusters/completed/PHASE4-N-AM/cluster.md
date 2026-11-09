# Invariant Cluster — PHASE4-N-AM — Wire-Pump Keep-Alive Client (sustain the live follow; CE-AI-6 unblock)

> NARROW liveness follow-on. The admission wire pump (`crates/ade_runtime/src/admission/wire_pump.rs`)
> drives chain-sync + block-fetch but **never runs the N2N keep-alive client** (mini-protocol 8). The
> peer's keep-alive responder holds `ClientHasAgency` and times the connection out at ~97s
> (`ShutdownPeer (KeepAlive) ExceededTimeLimit`) → `transport.inbound.recv()` returns `None` →
> `AdmissionWirePumpResult::Eof` → the follow ends. This is a **wire-only liveness gap**, NOT a
> consensus/admit/evidence defect — confirmed live (2026-06-11) and recorded as a loop-lifecycle
> obligation as far back as the PHASE4-N-AF baseline note (*"the follow link EOF'd on the relay idle
> timeout, no keep-alive"*).
>
> **Why CE-AI-6 is blocked on it.** The CE-AI-6 induced-reorg convergence capture (DC-EVIDENCE-03 — a
> strict slot regression Ade follows + re-converges `agreed`, 0 diverged, in ONE continuous transcript)
> requires Ade to STILL be following when a partition→heal reorg lands. Today Ade EOFs at ~97s — long
> before a reorg can be staged. The convergence sink truncates on open
> (`ConvergenceEvidenceSink::open = File::create`), so an EOF-reattach harness cannot preserve a
> continuous transcript and stitching is forbidden; a **SUSTAINED** follow is the only path. N-AM is
> the prerequisite.
>
> **Single change point.** `run_admission_wire_pump` is reused VERBATIM by both
> `spawn_live_wire_pump_source` (the `--mode node` feed consumed by `run_node_sync` AND
> `run_participant_sync`) and `spawn_wire_pumps_for_admission`. The keep-alive client lives entirely in
> the pump, so BOTH follow paths sustain — no change to either sync loop.
>
> **Single slice (AM-S1).** The cluster closes when (a) the pump sends `MsgKeepAlive` on a cadence
> strictly under the peer's keep-alive timeout, validates the echoed cookie, and emits no
> `AdmissionPeerEvent` for keep-alive (hermetic + CI-gated), and (b) a live participant/single-producer
> follow SUSTAINS past the ~97s deadline (CE-AM-LIVE — the operator-run enforced-backing preflight, the
> bright-red CE-AI-6 gate). The downstream CE-AI-6 reorg capture is the CONSUMER, not this cluster's bar.

## Primary invariant

**DC-PUMP-03** (declared here, targeted **enforced** at close): *The admission wire pump
(`run_admission_wire_pump`) runs the N2N keep-alive CLIENT (mini-protocol 8): on a cadence strictly
under the peer's keep-alive timeout it sends `KeepAliveMessage::KeepAlive(cookie)` (Initiator) via the
EXISTING outbound `OutboundFrame` path, advancing the BLUE `ade_network::keep_alive` state machine
(`keep_alive_transition`, `ClientIdle → ServerHasAgency{cookie}`); on the inbound
`MsgResponseKeepAlive(cookie')` it advances the SAME state machine (`ServerHasAgency{cookie} +
Server → ClientIdle`, which validates `cookie' == cookie`). The keep-alive client is **wire-only**: it
produces no canonical input, no WAL entry, and NO `AdmissionPeerEvent` (Block / TipUpdate /
RollBackward / Disconnected) — it never affects admission, the durable chain, fork-choice, the
convergence-evidence vocabulary, replay-equivalence, or any BLUE state. A keep-alive grammar violation
(cookie mismatch, illegal/out-of-version message) fails closed via the new
`AdmissionWirePumpError::KeepAlive` (drop the peer), consistent with the pump's fail-closed treatment of
chain-sync/block-fetch violations. The keep-alive cadence MUST NOT block, starve, or reorder the
chain-sync / block-fetch flow; a keep-alive frame MUST NOT be mistaken for a chain-sync/block-fetch
event (closed dispatch over `AcceptedMiniProtocol`). With the client running, a live participant AND
single-producer follow sustains past the ~97s keep-alive deadline.*

**No new BLUE authority.** The closed keep-alive wire grammar already exists in `ade_network::keep_alive`
(BLUE, S-A7) + `ade_network::codec::keep_alive` (BLUE); N-AM REUSES it — it does NOT redefine a
keep-alive message, cookie, state machine, or codec.

**CN-CONS-03 untouched — stays `declared`.** N-AM sustains the follow so the CE-AI-6 operator pass
becomes RUNNABLE; it does not emit convergence evidence, add multi-peer ChainSel, or flip CN-CONS-03.

**CN-PUMP-01 / DC-PUMP-01 / DC-PUMP-02 preserved (cross-ref, not strengthened).** The pump stays the
SOLE per-peer wire-pump entry (CN-PUMP-01); its `AdmissionPeerEvent` emit-set is UNWIDENED (DC-PUMP-01 —
keep-alive emits none); every chain-sync Tip-carrying reply still emits `TipUpdate` first (DC-PUMP-02 —
unchanged).

## Normative anchors

- `docs/planning/phase4-n-am-wire-pump-keepalive-sustain-invariants.md` (AM-1..3, the investigation
  outcome — keep-alive protocol timeout ~97s is the closer, NOT `ProtocolIdleTimeout`; chain-sync
  `AwaitReply` already sustains; the sink truncates so SUSTAIN-not-reattach is the path — and the
  prohibitions).
- `ade_network::keep_alive` (S-A7 — the BLUE keep-alive state machine `keep_alive_transition` +
  `KeepAliveState` + `KeepAliveAgency` + `KeepAliveEvent`; REUSED, not redefined).
- `ade_network::codec::keep_alive` (S-A2 — the BLUE `KeepAliveMessage` / `KeepAliveCookie` + canonical
  `encode/decode_keep_alive_message`; REUSED).
- `ade_network::session::{event, core}` — `AcceptedMiniProtocol::KeepAlive → KEEP_ALIVE_ID = 8`;
  `ByteChunkIn::OutboundFrame` framing via `handle_outbound` (post-handshake; the same path chain-sync
  and block-fetch frames already use); `SessionEffect::DeliverPeerFrame{KeepAlive}` (the inbound the
  pump currently drops at `wire_pump.rs:283`).
- CN-PUMP-01 / DC-PUMP-01 / DC-PUMP-02 (the pump sole-authority + emit-vocabulary closure — all
  PRESERVED; the keep-alive path emits NO `AdmissionPeerEvent`).
- AI-S4a (`wire_pump.rs:447` — `RollBackward(Origin)` fail-close) and the chain-sync `AwaitReply` arm
  (`wire_pump.rs:460`) — both PRESERVED; the keep-alive select arm is additive.
- DC-NODE-30 (N-AJ convergence evidence) / DC-NODE-33 (N-AL participant anchor no-op) — untouched; the
  keep-alive client emits nothing those reducers consume.

## Entry Conditions (guaranteed by prior clusters)

- **S-A7 / S-A2 (`ade_network::keep_alive` + `::codec::keep_alive`):** the closed BLUE keep-alive
  grammar (`keep_alive_transition`, `KeepAliveMessage`, `KeepAliveCookie`, canonical codec) exists and
  is tested.
- **PHASE4-N-L (session):** `AcceptedMiniProtocol::KeepAlive` is in the closed registry (id 8);
  `handle_outbound` frames any post-handshake `OutboundFrame{KeepAlive}`; the demuxer delivers inbound
  proto-8 frames as `DeliverPeerFrame{KeepAlive}`.
- **PHASE4-N-M-C (pump):** `run_admission_wire_pump` is the SOLE per-peer pump (CN-PUMP-01) with the
  outbound `outbox_payloads`/`flush_outbound` path and the closed inbound dispatch over
  `AcceptedMiniProtocol` (the `KeepAlive` arm currently drops, `:283`).
- **PHASE4-N-F-G-C (feed):** `spawn_live_wire_pump_source` builds the live `NodeBlockSource::WirePump`
  by reusing `dial_for_admission` + `run_admission_wire_pump` VERBATIM — so a pump-layer keep-alive
  client reaches the `--mode node` follow paths with NO feed/loop change.
- **PHASE4-N-AL (DC-NODE-33):** the participant bare-anchor recover→follow reaches the first forward
  block cleanly (CE-AL-3-LIVE) — the participant follow N-AM must SUSTAIN.

## Exit Criteria (CI-verifiable — named checks, not intent)

- **CE-AM-1** (cadence send, hermetic — `ade_runtime`): `wire_pump_sends_keep_alive_on_quiescent_cadence`
  — a `#[tokio::test(start_paused = true)]` loopback peer completes the post-handshake state; the pump
  sends `FindIntersect`; the peer stays QUIESCENT (sends nothing back); after the keep-alive cadence
  elapses the peer side receives a mux frame on mini-protocol id 8 whose payload decodes to
  `KeepAliveMessage::KeepAlive(_)`. Proves AM-1 (the pump sends `MsgKeepAlive` during inbound
  quiescence, under the deadline).
- **CE-AM-2** (cookie round-trip, hermetic): `wire_pump_keep_alive_response_validates_cookie_no_event`
  — with an outstanding `ServerHasAgency{cookie}`, an inbound `MsgResponseKeepAlive(cookie)` advances
  the BLUE state machine back to `ClientIdle` AND emits NO `AdmissionPeerEvent` (the event channel
  stays empty). Proves AM-1 (validation) + AM-2 (wire-only).
- **CE-AM-3** (mismatch fail-closed, hermetic): `wire_pump_keep_alive_cookie_mismatch_fails_closed` —
  an inbound `MsgResponseKeepAlive(wrong)` against an outstanding `ServerHasAgency{cookie}` (`wrong !=
  cookie`), or a `MsgResponseKeepAlive` while `ClientIdle` (no outstanding request), yields
  `AdmissionWirePumpError::KeepAlive` (drop the peer) — never a silent continue, never an
  `AdmissionPeerEvent`.
- **CE-AM-4** (wire-only fence, CI gate): `ci_check_keep_alive_wire_only.sh` — the `KeepAlive` dispatch
  arm in `wire_pump.rs` (a) is no longer the silent multi-protocol drop (it decodes via
  `decode_keep_alive_message` and drives `keep_alive_transition`), and (b) the keep-alive handler
  constructs NO `AdmissionPeerEvent::*` value (wire-only — DC-PUMP-03 / DC-PUMP-01 preserved). Mirrors
  the existing `ci_check_admission_wire_pump_closure.sh` / `ci_check_admission_no_red_verdicts.sh`
  grep-heuristic style.
- **CE-AM-5** (chain-sync/block-fetch unperturbed, hermetic): the existing pump CEs
  (`rollforward_drives_block_fetch_then_request_next`,
  `pump_emits_tip_update_and_request_next_on_intersect_found_no_block_fetch`, the AI-S4a RollBackward
  CEs) stay green with the keep-alive select arm present — the cadence never reorders or starves the
  chain-sync→block-fetch sequencing.
- **CE-AM-6** (no collateral): `cargo test -p ade_runtime` green; `cargo test -p ade_node` green;
  `cargo clippy -p ade_runtime` clean; `ci_check_keep_alive_wire_only.sh` passes; the existing
  `ci_check_admission_wire_pump_closure.sh` / `ci_check_admission_no_red_verdicts.sh` still pass.
- **CE-AM-LIVE** (live sustain preflight — operator-run at close; the **bright-red CE-AI-6 gate**): on
  the live venue (C2-LOCAL 2-pool `cardano-testnet` or preprod docker), a `--mode node`
  participant/single-producer follow runs PAST the ~97s keep-alive deadline — Ade keeps observing the
  peer's `RollForward`/`AwaitReply` beyond ~97s with **0** `ShutdownPeer (KeepAlive)` on the peer's log
  and **0** premature `Eof`, vs. the pre-AM ~96s EOF. This is the AM-3 enforced-backing evidence and the
  prerequisite that makes the CE-AI-6 reorg capture runnable. (Live transcript OUTSIDE-REPO; in-repo
  scrubbed note only.)

## Expected Slice Types

- **AM-S1** (keep-alive client — the single slice; DC-PUMP-03). In `run_admission_wire_pump`
  (`wire_pump.rs`):
  1. **Loop select.** Replace the bare `transport.inbound.recv().await` (`:186`) with a
     `tokio::select!` over (a) `transport.inbound.recv()` — the inbound arm is BYTE-IDENTICAL to today
     (a chunk → `step(Inbound)`; `None` → `Eof`), and (b) a `tokio::time::interval(KEEP_ALIVE_CADENCE)`
     tick. On tick: if the keep-alive state is `ClientIdle`, generate the next cookie (a `u16`
     monotonic counter, deterministic — no rand), drive `keep_alive_transition(ClientIdle, Client,
     version, KeepAlive(cookie))` → `ServerHasAgency{cookie}`, enqueue
     `OutboundFrame{KeepAlive, encode_keep_alive_message(&KeepAlive(cookie)), Initiator, 0}`, and
     `continue` (the loop-top flush sends it). If not `ClientIdle` (a keepalive is in flight), skip the
     tick.
  2. **Inbound consume.** Change the `AcceptedMiniProtocol::KeepAlive` dispatch arm (`:283`) from the
     silent drop to: `decode_keep_alive_message(&payload)` → drive `keep_alive_transition(state,
     Server, version, msg)` (validates the cookie / agency / version); on `Ok` update the keep-alive
     state (back to `ClientIdle` on a matched response) and emit NOTHING; on `Err` return
     `AdmissionWirePumpError::KeepAlive` (fail closed). The other dropped arms (Handshake /
     TxSubmission / Local* / PeerSharing) stay dropped.
  3. **Error sum.** Add the additive closed variant `AdmissionWirePumpError::KeepAlive(KeepAliveError)`.
  4. **Const + version.** `const KEEP_ALIVE_CADENCE: Duration = Duration::from_secs(20)` (strictly under
     the observed ~97s peer deadline; documented). Thread the keep-alive protocol version as
     `KeepAliveVersion::new(negotiated_version)` (the N2N version, well under the `MAX_KEEP_ALIVE_VERSION
     = 100` guard — explicit input per DC-PROTO-06, never a session global).

  **NO** new BLUE type / module / codec (reuses `ade_network::keep_alive` + `::codec::keep_alive`);
  **NO** new `AdmissionPeerEvent` variant; **NO** change to chain-sync/block-fetch handling,
  `pump_block`, `handle_chain_sync`, `handle_block_fetch`, AI-S4a, the `AwaitReply` arm, the runner, or
  either sync loop. Mechanical proof CE-AM-1..6 + CE-AM-LIVE.

## TCB Color Map (FC/IS Partition)

- **BLUE (reused, unchanged)** — `ade_network::keep_alive::keep_alive_transition` (the cookie/agency/
  version-checked state machine) + `ade_network::codec::keep_alive::{encode,decode}_keep_alive_message`
  (the canonical wire grammar). N-AM adds NO BLUE code and changes none.
- **Canonical input** — none. The keep-alive client produces no canonical input, no WAL entry, no
  reducer input. The cookie is a transport nonce, not consensus state.
- **RED (wiring — the entire slice)** — the keep-alive cadence (`tokio::time::interval`), the cookie
  counter, the `select!`, the outbound enqueue, and the inbound decode→transition→fail-closed live in
  the RED shell `ade_runtime::admission::wire_pump`. Wall-clock cadence is a RED concern (the pump is
  already nondeterministic — real sockets, `tokio::time::timeout` in tests); it never reaches BLUE.
- **RED / unchanged** — `handle_chain_sync`, `handle_block_fetch`, `flush_outbound`, the runner, the
  `AdmissionPeerEvent` emit-set, `pump_block`, `run_node_sync`, `run_participant_sync`,
  `spawn_live_wire_pump_source`.

## Forbidden during this cluster (slices inherit)

Redefining the keep-alive message / cookie / state machine / codec (REUSE `ade_network::keep_alive`) ·
emitting ANY `AdmissionPeerEvent` (Block / TipUpdate / RollBackward / Disconnected) from the keep-alive
path · writing a WAL entry or producing a canonical input from keep-alive · a cadence ≥ the peer's
keep-alive timeout (must be strictly under ~97s) · sending a new `MsgKeepAlive` while one is in flight
(respect `ServerHasAgency` agency) · blocking / starving / reordering the chain-sync or block-fetch
flow on the keep-alive tick · mistaking a keep-alive frame for a chain-sync/block-fetch event (the
dispatch stays a closed `match` over `AcceptedMiniProtocol`) · silently swallowing a keep-alive grammar
violation (fail closed via `AdmissionWirePumpError::KeepAlive`) · using `rand` / wall-clock for the
cookie (monotonic `u16` counter) · implementing a keep-alive SERVER/responder in this slice (client
only — the responder is a proof-obligation-gated follow-on ONLY if CE-AM-LIVE shows the peer keepalives
Ade) · stitching an EOF-reattached transcript (the sink truncates; SUSTAIN is the path) · flipping
CN-CONS-03 · running the CE-AI-6 reorg/convergence pass before CE-AM-LIVE proves the sustained follow
(the bright-red gate).

## Registry declarations (this cluster-doc appends as `declared`)

- **DC-PUMP-03** (family DC, derived, `introduced_in = PHASE4-N-AM`, status `declared`) — statement as
  the Primary invariant above. `tests = []` (AM-S1 populates the named tests);
  `ci_scripts = ["ci/ci_check_keep_alive_wire_only.sh"]` (added at AM-S1). `cross_ref = ["CN-PUMP-01",
  "DC-PUMP-01", "DC-PUMP-02", "DC-NODE-30", "CN-CONS-03"]`. Appended to the registry (declared) after the
  investigation (→ 360 rules, coherence gate green).
- **CN-PUMP-01 / DC-PUMP-01 / DC-PUMP-02 explicitly NOT modified** (no `strengthened_in += PHASE4-N-AM`)
  — the keep-alive client is a NEW derived rule on the pump, not a re-scoping of the pump-authority /
  emit-vocabulary rules; those are PRESERVED (the keep-alive path emits no `AdmissionPeerEvent`, so the
  emit-set closure is unchanged). DC-PUMP-03 cross-refs them.
- **DC-PUMP-03 `enforced`** (PHASE4-N-AM, 2026-06-11) — CE-AM-1..6 (hermetic + the new gate) GREEN + both
  close-gate reviews CLEAN (IDD 0 BLOCK; security 0 HIGH/MEDIUM) + **CE-AM-LIVE PASSED** (the operator
  sustain pass: recover @ slot 202 → caught up → `agreed` @ slot 241 with our_hash == peer_hash → keep-alive
  SUSTAINED 152s vs the prior ~96s EOF, 7 ping/pong @ 20s, cookies validated, 0 grammar failures,
  0 idle-EOF; transcript OUTSIDE-REPO, sha `b96b842a…`). Banked at `enforced_scaffolding` (`c0430322`)
  then flipped to `enforced` once CE-AM-LIVE was recorded — NO overclaim. NOT a CE-AI-6 claim (no reorg /
  slot regression).

## Close-record note (preserve verbatim at `/cluster-close`)

> **AM adds the wire-pump keep-alive CLIENT (mini-protocol 8):** `run_admission_wire_pump` now sends
> `MsgKeepAlive` on a ~20s cadence (strictly under the peer's ~97s keep-alive timeout) via the EXISTING
> outbound frame path, advancing the REUSED BLUE `ade_network::keep_alive` state machine, and validates
> the echoed cookie on the inbound `MsgResponseKeepAlive` — so a live participant/single-producer
> follow sustains past the keep-alive deadline instead of EOFing at ~97s. It is **wire-only**: no
> canonical input, no WAL entry, NO `AdmissionPeerEvent`; a grammar violation fails closed
> (`AdmissionWirePumpError::KeepAlive`). It does NOT add a keep-alive server/responder, redefine the
> BLUE grammar, widen the pump emit-set, touch chain-sync/block-fetch/`pump_block`/either sync loop, or
> flip CN-CONS-03. It **unblocks — but does NOT run** — the CE-AI-6 convergence pass (gated on
> CE-AM-LIVE). NO BLUE change; NO new canonical type; NO new module.
