# Slice AM-S1 — Wire-Pump Keep-Alive Client Sustains the Live Follow

## 1. Title
The admission wire pump (`run_admission_wire_pump`) runs the N2N keep-alive CLIENT (mini-protocol 8):
on a cadence strictly under the peer's keep-alive timeout it sends `MsgKeepAlive` (Initiator), advances
the REUSED BLUE `ade_network::keep_alive` state machine, and validates the echoed cookie on the inbound
`MsgResponseKeepAlive` — so a live participant/single-producer follow sustains past the ~97s keep-alive
deadline instead of EOFing. The single slice of PHASE4-N-AM.

## 2. Slice Header
- **Cluster:** PHASE4-N-AM. **Status:** Merged (impl `a1655449`; authority docs `c1b9eee2`; banked
  `enforced_scaffolding` 2026-06-11).
- **Cluster Exit Criteria Addressed:** CE-AM-1..6 (hermetic + the new gate) **GREEN**; **CE-AM-LIVE
  OPEN** — the operator sustain pass, discharged as the FIRST leg of the CE-AI-6 bridge-venue session
  (one fresh venue serves both; the sustain leg is the hard guard on starting the CE-AI-6 reorg).
- **Primary registry rule:** DC-PUMP-03 (`declared` → `enforced_scaffolding` at AM bank; `enforced`
  ONLY after CE-AM-LIVE is recorded).

## 4. Intent (invariant impact)
Strengthen **DC-PUMP-03** `declared → enforced`: the wire pump is the SOLE driver of the per-peer N2N
connection (CN-PUMP-01), so it must run the keep-alive client to keep that connection alive during
inbound quiescence. Today the pump's loop blocks on `transport.inbound.recv().await` (`wire_pump.rs:186`)
and never sends a keep-alive; the peer's keep-alive responder holds `ClientHasAgency` and times the
connection out at ~97s (`ShutdownPeer (KeepAlive) ExceededTimeLimit`), ending the follow. AM-S1 makes
the pump a keep-alive client — wire-only, no authority/admit/evidence change — so the follow survives
inter-block gaps and chain-sync `AwaitReply`. This is the prerequisite that makes the CE-AI-6 induced
reorg capture runnable (Ade must still be following when the reorg lands; the convergence sink truncates
on open, so SUSTAIN — not EOF-reattach — is the only path).

## 6. Execution Boundary (TCB color)
- **BLUE (reused, unchanged)** — `ade_network::keep_alive::keep_alive_transition` (cookie/agency/version
  checked state machine) + `ade_network::codec::keep_alive::{encode,decode}_keep_alive_message`. AM adds
  no BLUE code.
- **Canonical input** — NONE. The keep-alive client produces no canonical input, no WAL entry, no
  reducer input. The cookie is a transport nonce.
- **RED (the entire slice)** — the cadence (`tokio::time::interval`), the `u16` cookie counter, the
  `tokio::select!`, the outbound `OutboundFrame{KeepAlive}` enqueue, and the inbound
  decode→transition→fail-closed all live in the RED shell `ade_runtime::admission::wire_pump`. Wall-clock
  cadence is RED (the pump is already nondeterministic — real sockets, `tokio::time::timeout` in tests).
- **RED / UNCHANGED** — `handle_chain_sync`, `handle_block_fetch`, `flush_outbound`, the runner, the
  `AdmissionPeerEvent` emit-set, `pump_block`, `run_node_sync`, `run_participant_sync`,
  `spawn_live_wire_pump_source`. AI-S4a (`wire_pump.rs:447`) and the chain-sync `AwaitReply` arm
  (`wire_pump.rs:460`) untouched.

## 7. Invariants Preserved
- **CN-PUMP-01** — `run_admission_wire_pump` stays the SOLE per-peer pump; the keep-alive client is
  inside it, not a second pump.
- **DC-PUMP-01** — the pump's `AdmissionPeerEvent` emit-set is UNWIDENED; the keep-alive path emits NONE
  (Block / TipUpdate / RollBackward / Disconnected). No `AgreementVerdict`.
- **DC-PUMP-02** — every chain-sync Tip-carrying reply still emits `TipUpdate` first; the keep-alive
  select arm is additive, never intercepting a chain-sync/block-fetch frame.
- **AI-S4a / DC-NODE-23..29** — `RollBackward(Origin)` stays fail-closed; the rollback signal path is
  untouched.
- **DC-NODE-30 (N-AJ)** / **DC-NODE-33 (N-AL)** — convergence evidence + the participant anchor no-op:
  the keep-alive client emits nothing those reducers consume.
- **The BLUE keep-alive grammar (S-A7 / S-A2)** — REUSED verbatim; no new message/cookie/state/codec.
- **CN-CONS-03** — untouched; AM sustains the follow, it does not add multi-peer ChainSel.

## 8. Invariants Strengthened
- **DC-PUMP-03** `declared → enforced_scaffolding` (banked) — AM-S1 populates its `tests` (CE-AM-1..3)
  and `ci_scripts` (`ci_check_keep_alive_wire_only.sh`); `enforced` is WITHHELD until CE-AM-LIVE (the
  live sustain leg, in the CE-AI-6 bridge venue) is recorded.
- No existing rule's `strengthened_in` gains `PHASE4-N-AM` — CN-PUMP-01 / DC-PUMP-01 / DC-PUMP-02 are
  cross-refs (preserved), not strengthenings (the keep-alive client is a NEW derived rule on the pump).

## 9. Design Summary
- **Loop select (the only structural change).** Replace the bare `transport.inbound.recv().await`
  (`:186`) with `tokio::select!` over (a) `transport.inbound.recv()` and (b) a
  `tokio::time::interval(KEEP_ALIVE_CADENCE)` tick. The inbound arm is BYTE-IDENTICAL to today (`Some(c)`
  → `step(Inbound(c))`; `None` → `finalize(Eof)`). On the tick arm: if `keep_alive_state ==
  KeepAliveState::ClientIdle`, take the next cookie (`next_cookie`, a `u16` monotonic counter —
  deterministic, no rand), drive `keep_alive_transition(ClientIdle, Client,
  KeepAliveVersion::new(negotiated_version), KeepAlive(cookie))` → `ServerHasAgency{cookie}`, enqueue
  `OutboundFrame{KeepAlive, encode_keep_alive_message(&KeepAlive(cookie)), Initiator, 0}`, and `continue`
  (the loop-top flush sends it). If not `ClientIdle` (a keepalive is in flight — respect
  `ServerHasAgency` agency), skip the tick.
- **Inbound consume.** Change the `AcceptedMiniProtocol::KeepAlive` arm (`:283`) from the silent
  multi-protocol drop to: `decode_keep_alive_message(&payload)` → drive `keep_alive_transition(state,
  Server, version, msg)`; on `Ok((new_state, _output))` set `keep_alive_state = new_state` (back to
  `ClientIdle` on a matched response) and emit NOTHING; on `Err` return
  `AdmissionWirePumpError::KeepAlive(e)` (fail closed → `finalize`). The other dropped arms
  (Handshake / TxSubmission / Local* / PeerSharing) stay dropped.
- **State threading.** `keep_alive_state: KeepAliveState` (init `ClientIdle`) and `next_cookie: u16`
  (init 0, `wrapping_add(1)` per send) are pump-loop locals — no new pump param, no struct field.
- **Error sum.** Add the additive closed variant `AdmissionWirePumpError::KeepAlive(KeepAliveError)`
  (the BLUE error is `Copy + Eq`, so the closed sum's derives hold).
- **Const.** `const KEEP_ALIVE_CADENCE: Duration = Duration::from_secs(20)` — strictly under the observed
  ~97s peer deadline (~3 missed-tick margin); documented with the deadline citation. (Ecosystem default
  is 10s; 20s is quieter and safe.)
- **Helper.** A `fn handle_keep_alive(payload, &mut keep_alive_state, version) -> Result<(),
  AdmissionWirePumpError>` keeps the inbound consume unit-testable (CE-AM-2/3) and the gate's grep-target
  obvious.

### Resolved open questions
- **OQ-AM-1 (cadence):** 20s — strictly under the observed ~97s deadline. Resolved.
- **OQ-AM-2 (mechanism):** `select!` + `tokio::time::interval` (matches the RED idiom in
  `network::mux_pump`/`n2n_listener`); cookie is a loop-local `u16` counter. Resolved.
- **OQ-AM-3 (shared layer):** the client sits in `run_admission_wire_pump`, which both
  `spawn_live_wire_pump_source` (→ `run_node_sync` + `run_participant_sync`) and
  `spawn_wire_pumps_for_admission` reuse VERBATIM — so both follow paths sustain. Resolved.

### Slice-entry proof obligation (discharged by CE-AM-LIVE)
- **Does the peer also run a keep-alive CLIENT toward Ade (making Ade a keep-alive SERVER)?** The live
  evidence (the peer's `ClientHasAgency (SingClient)` timeout — the peer waiting for US as client) says
  NO on this connection: keep-alive runs one way, Ade(client) → peer(server). AM-S1 implements the
  CLIENT ONLY. CE-AM-LIVE is the proof obligation: if the sustained run shows the peer sending us
  `MsgKeepAlive` (inbound tag 0) — which would currently fail closed as an `IllegalTransition` — a
  keep-alive SERVER/responder is a scoped follow-on. (Per proof discipline, this is a slice-entry
  obligation, not a footnote.)

## 11. Replay / Crash / Epoch Validation
The keep-alive client is **not replay-visible**: it produces no canonical input and no WAL entry, so the
BLUE admit path's replay-equivalence (T-REC-05 / DC-WAL-02 / DC-NODE-33) is byte-unchanged — replaying
the same ordered canonical inputs yields the same post-state regardless of keep-alive traffic. A crash
mid-follow recovers identically (durable state is whatever `pump_block` admitted; keep-alive mutated
nothing). The cadence is wall-clock (RED), never a consensus input.

## 12. Mechanical Acceptance Criteria
- **CE-AM-1** (`ade_runtime`, hermetic): `wire_pump_sends_keep_alive_on_quiescent_cadence` —
  `#[tokio::test(start_paused = true)]`: a loopback peer completes the post-handshake state; the pump
  sends `FindIntersect`; the peer stays QUIESCENT; after `KEEP_ALIVE_CADENCE` the peer side receives a
  mux frame on mini-protocol id 8 whose payload `decode_keep_alive_message`s to
  `KeepAliveMessage::KeepAlive(_)`. (AM-1.)
- **CE-AM-2** (hermetic, unit): `wire_pump_keep_alive_response_validates_cookie_no_event` — with
  `keep_alive_state = ServerHasAgency{cookie}`, `handle_keep_alive(encode(ResponseKeepAlive(cookie)),
  &mut state, version)` returns `Ok`, sets `state = ClientIdle`, and (via the loop) emits NO
  `AdmissionPeerEvent`. (AM-1 validation + AM-2 wire-only.)
- **CE-AM-3** (hermetic, unit): `wire_pump_keep_alive_cookie_mismatch_fails_closed` — a
  `ResponseKeepAlive(wrong)` against `ServerHasAgency{cookie}` (`wrong != cookie`), AND a
  `ResponseKeepAlive` while `ClientIdle`, each yield `Err(AdmissionWirePumpError::KeepAlive(..))`.
- **CE-AM-4** (CI gate): `ci_check_keep_alive_wire_only.sh` — the `KeepAlive` dispatch arm decodes via
  `decode_keep_alive_message` + drives `keep_alive_transition` (no longer the silent drop), AND the
  keep-alive handler constructs NO `AdmissionPeerEvent::*` (wire-only). Mirrors the existing
  `ci_check_admission_wire_pump_closure.sh` / `ci_check_admission_no_red_verdicts.sh` grep-heuristic
  style.
- **CE-AM-5** (hermetic): the existing pump CEs (`rollforward_drives_block_fetch_then_request_next`,
  `pump_emits_tip_update_and_request_next_on_intersect_found_no_block_fetch`, the AI-S4a RollBackward
  CEs) stay GREEN with the keep-alive select arm present.
- **CE-AM-6** (no collateral): `cargo test -p ade_runtime` green; `cargo test -p ade_node` green;
  `cargo clippy -p ade_runtime` clean; `ci_check_keep_alive_wire_only.sh` passes;
  `ci_check_admission_wire_pump_closure.sh` + `ci_check_admission_no_red_verdicts.sh` still pass.
- **CE-AM-LIVE** (live sustain preflight, operator-run at close — the **bright-red CE-AI-6 gate**): on
  the live venue, a `--mode node` participant/single-producer follow runs PAST ~97s — Ade keeps
  observing the peer's `RollForward`/`AwaitReply` beyond the deadline with **0** peer-side `ShutdownPeer
  (KeepAlive)` and **0** premature `Eof`, vs. the pre-AM ~96s EOF. (Live transcript OUTSIDE-REPO.)

## 14. Hard Prohibitions (inherit cluster Forbidden verbatim)
- Do NOT redefine the keep-alive message / cookie / state machine / codec — REUSE
  `ade_network::keep_alive` + `::codec::keep_alive`.
- Do NOT emit ANY `AdmissionPeerEvent` from the keep-alive path; do NOT write a WAL entry or produce a
  canonical input.
- Do NOT use a cadence ≥ the peer's keep-alive timeout (must be strictly under ~97s); do NOT send a new
  `MsgKeepAlive` while one is in flight (respect `ServerHasAgency`).
- Do NOT block / starve / reorder the chain-sync or block-fetch flow on the keep-alive tick; do NOT let a
  keep-alive frame be dispatched as a chain-sync/block-fetch event (closed `match` over
  `AcceptedMiniProtocol`).
- Do NOT silently swallow a keep-alive grammar violation — fail closed via
  `AdmissionWirePumpError::KeepAlive`.
- Do NOT use `rand` / wall-clock for the cookie (monotonic `u16` counter).
- Do NOT implement a keep-alive SERVER/responder in this slice (client only; the responder is a
  CE-AM-LIVE-gated follow-on).
- Do NOT change `pump_block` / `handle_chain_sync` / `handle_block_fetch` / the runner / `run_node_sync`
  / `run_participant_sync`; do NOT weaken AI-S4a; do NOT flip CN-CONS-03.
- Do NOT run the CE-AI-6 reorg/convergence pass before CE-AM-LIVE proves the sustained follow.
