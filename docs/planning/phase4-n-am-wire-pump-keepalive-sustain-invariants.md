# PHASE4-N-AM — Wire-Pump Keep-Alive Client (sustain the live follow)

> Invariants sketch (IDD Part I). Cluster **PHASE4-N-AM** — the prerequisite that **unblocks the
> CE-AI-6 reorg capture**. **Classification:** *improving reinterpretation that preserves semantics* —
> Ade's live follow gains the standard Ouroboros N2N keep-alive client so the peer no longer
> `ShutdownPeer`s it on the ~97s keep-alive deadline; **no consensus / admit / evidence semantics
> change** (wire-only liveness).

## The seam this exposes (confirmed LIVE, 2026-06-11)

The admission wire pump (`crates/ade_runtime/src/admission/wire_pump.rs`) runs chain-sync +
block-fetch but **DROPS keep-alive frames** (`wire_pump.rs:283`: *"the runner has no consumer for
them"*). The N2N keep-alive mini-protocol (**miniProtocol 8**) requires the CLIENT (Ade, the
initiator) to periodically send `MsgKeepAlive`; the relay's responder times out after ~97s and
`ShutdownPeer`s the connection → the pump's `transport.inbound.recv()` returns `None` →
`AdmissionWirePumpResult::Eof` → the follow ends.

**Live evidence (the CE-AL-3-LIVE / sustain-test):** Ade (participant follow, `--mode node
--participant-venue`) received 3 blocks across ~88s then EOF'd at **~96s**, while the relay (node1)
kept producing. node1's log at the close:
`Net.Mux.Remote.ExceptionExit: "ExceededTimeLimit (KeepAlive) ClientHasAgency (SingClient)"` →
`ConnectionHandler.Error: command "ShutdownPeer", reason "ExceededTimeLimit (KeepAlive)"`
(`miniProtocolNum 8`). **Cranking the relay's `ProtocolIdleTimeout` to 3600s did NOT help** — the
closer is the keep-alive protocol timeout, a different mechanism.

## Investigation outcome (resolved before this sketch)

- **The closer is the keep-alive protocol timeout (~97s), NOT `ProtocolIdleTimeout`** — verified: the
  3600s crank took (`ncProtocolIdleTimeout = 3600s` in the node config dump) yet Ade still EOF'd at ~96s.
- **chain-sync `AwaitReply` IS handled** (`wire_pump.rs:460` — the pump waits for the next
  `RollForward`/`RollBackward`), so chain-sync itself sustains; the EOF is purely the missing keep-alive
  client.
- **The convergence sink truncates** (`ConvergenceEvidenceSink::open` = `File::create`,
  `convergence_evidence.rs:85`) — so an EOF-reattach harness CANNOT preserve a continuous transcript,
  and stitching is forbidden. A **SUSTAINED** follow (this slice) needs no reattach, so append-mode is
  NOT required by PHASE4-N-AM (it can be a separate future concern).
- **Why this matters for CE-AI-6:** the reorg evidence (DC-EVIDENCE-03 — a strict slot regression Ade
  follows + re-converged `agreed`, 0 diverged, in ONE transcript) requires Ade to still be following
  when an induced reorg lands. Today it EOFs at ~97s — long before a partition→heal reorg can be staged.

## Pure transformation?
The keep-alive client is a deterministic wire loop: on a cadence under the peer's timeout, send
`MsgKeepAlive` (with a cookie); on the response, validate the cookie. Cadence is transport-driven, not
consensus-driven. No authoritative-state change; emits no `AdmissionPeerEvent` that reaches the BLUE
reducer.

## 1. What must always be true
- **AM-1 (keep-alive client).** The wire pump sends N2N `MsgKeepAlive` at a cadence strictly under the
  peer's keep-alive timeout and consumes the responses, so the connection sustains during quiescence
  (chain-sync `AwaitReply` / inter-block gaps).
- **AM-2 (wire-only — no authority change).** Keep-alive never affects admission, the durable chain,
  fork-choice, the convergence-evidence vocabulary, replay-equivalence, or any BLUE state. It is a
  liveness/transport concern; it produces no canonical input and no WAL entry.
- **AM-3 (sustained follow).** With the keep-alive client, a live participant **and** single-producer
  follow sustains past the ~97s keep-alive deadline (no `ShutdownPeer`/`Eof` on quiescence), so it
  observes later peer `RollForward`/`RollBackward` it would otherwise miss.
- **Preserved (unchanged):** chain-sync + block-fetch handling; `AwaitReply` (`:460`); the EOF path
  still fires on a genuine connection end; AI-S4a (`RollBackward(Origin)` fail-close); DC-NODE-30
  (convergence evidence); DC-NODE-33 (participant anchor no-op); `pump_block` as sole admit.

## 2. What must never be possible
- Keep-alive affecting consensus / admit / evidence (it is wire-only).
- A keep-alive cadence ≥ the peer's keep-alive timeout (would still time out → defeats the slice).
- Keep-alive blocking or starving the chain-sync / block-fetch flow.
- A keep-alive frame being mistaken for a chain-sync/block-fetch event (closed dispatch).

## 3 / 4. Determinism / replay
The keep-alive client is RED wire/transport orchestration (cookies, cadence) — not replay-visible (no
canonical input / WAL entry). The BLUE admit path's replay-equivalence is unchanged (keep-alive emits
nothing the reducer consumes).

## 5. State transitions in scope (wire pump)
The pump gains a keep-alive cadence: periodically enqueue `MsgKeepAlive` (Initiator) under the timeout;
on inbound `MsgKeepAliveResponse`, validate the echoed cookie. The existing chain-sync/block-fetch
loop is otherwise unchanged. (Mechanism — timer vs. piggyback on the inbound loop — resolved at
slice-doc.)

## 6. TCB color hypothesis
- **RED** — the keep-alive client lives in `admission/wire_pump.rs` (RED shell). No BLUE change, no new
  canonical type. (The closed keep-alive wire grammar already exists in `ade_network::keep_alive` —
  BLUE — and is reused, not re-defined.)

## 7. Open questions
- Exact cadence (e.g., every ~30–60s, well under ~97s) + cookie handling (the keep-alive cookie is a
  `u16` echoed in the response). Resolve at slice-doc.
- Timer-driven cadence vs. inbound-loop piggyback (the pump's loop currently blocks on
  `transport.inbound.recv()`; a cadence needs a `select!`/timeout so it can send keep-alives during a
  quiescent inbound). Resolve at slice-doc.
- Single-producer path (`run_node_sync`) benefits identically — confirm the keep-alive client sits at
  the shared wire-pump layer so both follow paths sustain.

## Registry (declared after investigation)
**DC-PUMP-03** (proposed; family/ID confirmed at `/cluster-doc`) — *the admission wire pump runs the
N2N keep-alive client (sends periodic `MsgKeepAlive` under the peer's keep-alive timeout, validates the
echoed cookie) so a live follow sustains past the keep-alive deadline; wire-only, no authority/
replay/evidence change.* `status = declared`. **Unblocks CE-AI-6** (the sustained follow is the
prerequisite for capturing an induced reorg in one continuous transcript). Likely test-enforced + a
dedicated `ci_check` fencing the keep-alive-is-wire-only property.
