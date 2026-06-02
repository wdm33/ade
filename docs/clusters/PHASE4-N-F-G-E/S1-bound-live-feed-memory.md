# Invariant Slice — PHASE4-N-F-G-E S1: Bound the live-feed memory surfaces

> **Status:** Planning Artifact (Non-Normative). Normative authority is the registry + CI.

## 2. Slice Header

### Slice Name
Bound the live-feed memory surfaces (reassembly-tail cap + WirePump lookahead-depth cap).

### Cluster
**PHASE4-N-F-G-E** — Live-feed bounded memory.

### Status
Ready for Review (all §12 mechanical acceptance green; `ade_network` + `ade_node` suites pass, both
fences byte-unchanged + green, `ci_check_live_feed_memory_bounds.sh` green; no BLUE change).

### Cluster Exit Criteria Addressed
- [x] **CE-G-E-1 (live-feed memory bounded + fail-closed)** — the four named tests pass; the existing
  containment + handoff-fence gates are byte-unchanged + green; `ci_check_live_feed_memory_bounds.sh`
  green; `cargo test -p ade_network` + `cargo test -p ade_node` green.

### Slice Dependencies
- PHASE4-N-F-G-C (live WirePump feed wiring) — **merged** (`main` `351d46bc`).
- N-M-FRAG (`CN-SESS-04` / `DC-SESS-06`: session `proto_buffers` reassembly) — merged.

## 3. Implementation Instruction (AI)
Add two closed-constant caps applied **before authoritative decode/apply**: a 16 MiB per-mini-protocol
reassembly-tail cap in GREEN `ade_network::session::core` (returns a structured `SessionError` — additive
closed-enum variant — and drops the peer) and a 256-block WirePump lookahead-depth cap in RED
`ade_node::node_sync` (stops opportunistic draining; the existing bounded mpsc back-pressures). Do **not**
change BLUE, truncate semantically, swallow malformed/incomplete input as success, add an unbounded /
operator-disable option, touch serve/forge/containment, or make any live-evidence claim. The bounds are
**closed constants** — never wired to CLI / env / config. Commit messages carry the project attribution
trailer (CLAUDE.md) and no other AI references.

## 4. Intent
Make it **impossible** for a peer on the live `--mode node` feed to drive unbounded memory growth before
authoritative decode/apply: a per-mini-protocol reassembly buffer over 16 MiB fails closed (drop), and
the WirePump lookahead never exceeds 256 buffered blocks (the bounded channel back-pressures). (Introduces
`DC-LIVEMEM-01`; preserves the session reassembly contract, the verdict-decoupled source contract, and the
serve/containment fences.)

## 5. Scope
- **Modules / crates:**
  - `ade_network::session::core` (GREEN) — a 16 MiB cap on each `proto_buffers[proto]`; on over-cap
    return a structured `SessionError` (drop the peer). Additive `SessionError` variant
    (`ReassemblyBufferOverflow`) — closed-enum extension, no wildcard.
  - `ade_node::node_sync` (RED) — `pump_lookahead` stops draining at 256 buffered blocks; closed const.
  - (optional) `ci/ci_check_live_feed_memory_bounds.sh` (NEW gate) — both constants exist + are not
    wired to CLI / env / config.
- **State machines affected:** the session reducer gains one fail-closed transition (over-cap →
  `SessionError`); no new state, no new mini-protocol, no change to the message vocabulary.
- **Persistence impact:** none. No WAL/checkpoint/canonical change.
- **Network-visible impact:** an over-cap peer connection is dropped with a structured error (the peer
  observes a disconnect). No new/changed wire messages; under-cap behavior is byte-identical.
- **Out of scope:** any other DoS surface; serve/forge/containment; any live-evidence / BA-02 / rehearsal
  / RO-LIVE flip; CLI/config.

## 6. Execution Boundary
- **BLUE (none — unchanged):** `ade_codec` decode path. The cap fires **before** it. A BLUE change → reject.
- **GREEN:** `ade_network::session::core` — the 16 MiB reassembly-tail cap + the additive `SessionError`
  variant. Deterministic (a pure function of bytes seen); no I/O, clock, rand, float.
- **RED:** `ade_node::node_sync` — the 256-block WirePump lookahead-depth cap (content-blind scheduling
  state).
- **Color resolved:** no ambiguity — `session::core` is GREEN-by-content (`//! GREEN`), `node_sync` is
  RED (`//! RED`), no BLUE.

## 7. Invariants Preserved
- `CN-SESS-04` / `DC-SESS-06` — the per-mini-protocol reassembly contract; the cap is an **additive
  bound** (under-cap reassembly is byte-unchanged).
- `DC-SYNC-01` / `DC-SYNC-02` — the WirePump feed → `run_node_sync → pump_block` single tip-advance path;
  the lookahead cap adds back-pressure, no second path, no reordering.
- The closed verdict-decoupled `NodeBlockSource` contract — the lookahead stays content-blind; a buffered
  block is still delivered next in arrival order once below the cap.
- `DC-NODE-06` / `CN-NODE-02` — the serve handoff + relay-loop containment; gates byte-unchanged.
- Determinism: the GREEN cap is a pure byte-count check; no nondeterminism enters the reducer.

## 8. Invariants Strengthened or Introduced
- **`DC-LIVEMEM-01` (introduced — derived / operational-hardening; NOT BLUE consensus law).** Peer-driven
  live-feed memory is bounded before authoritative decode/apply: reassembly-tail over 16 MiB → structured
  `SessionError`; WirePump lookahead capped at 256 (bounded channel back-pressures); bounds are closed
  constants with no runtime/CLI/env/config disable or unbounded option. Enforced by the four named tests +
  (optionally) `ci_check_live_feed_memory_bounds.sh`. Flips `declared → enforced` at cluster close.

> Single invariant family: "peer-driven live-feed memory is bounded before authoritative decode/apply."
> Both surfaces (GREEN reassembly tail + RED lookahead depth) are the same live-feed DoS boundary.

## 9. Design Summary
- **GREEN reassembly-tail cap** (`session::core`): a closed `const MAX_REASSEMBLY_TAIL_BYTES: usize = 16 *
  1024 * 1024;`. At the reassembly append site (`proto_buffers[proto].extend_from_slice(&frame.payload)`),
  if the buffer would exceed / exceeds the cap **without** having yielded a complete item, return
  `Err(SessionError::ReassemblyBufferOverflow { .. })` (the caller drops the peer). The cap is checked on
  the accumulating buffer, so a legit under-cap multi-frame item (a block is far under 16 MiB) drains
  unchanged; a pathological endless-incomplete item fails closed.
- **RED lookahead-depth cap** (`node_sync`): a closed `const MAX_WIRE_PUMP_LOOKAHEAD: usize = 256;`. In
  `pump_lookahead`, stop the opportunistic `try_recv` drain once `lookahead.len() >= MAX_WIRE_PUMP_LOOKAHEAD`.
  The bounded mpsc (cap 64) then fills and the pump's `events_tx.send().await` back-pressures — end-to-end
  bound. `next_block` continues to drain the lookahead in arrival order; once below the cap, draining
  resumes.
- **No escape hatch:** both constants are module-level `const` — not read from `Cli`, `std::env`, or any
  config; the (optional) grep gate pins this.

## 10. Changes Introduced
### Types
- New: closed `const MAX_REASSEMBLY_TAIL_BYTES` (GREEN, `session::core`) + `const MAX_WIRE_PUMP_LOOKAHEAD`
  (RED, `node_sync`). Additive `SessionError::ReassemblyBufferOverflow` variant. No new canonical type, no
  new `Mode`, no CLI field.
### State Transitions
- New (T1): over-cap reassembly → `SessionError::ReassemblyBufferOverflow` (fail closed). T2: lookahead
  drain stops at cap (scheduling, not a state-machine transition).
### Persistence
- None.
### Removal / Refactors
- None (additive caps).

## 11. Replay, Crash, and Epoch Validation
- **Replay:** no new authoritative state → replay unaffected. The same captured feed replays identically;
  an over-cap input deterministically fails closed on both runs (the GREEN cap is a pure byte-count
  function). No new corpus lane.
- **Crash/restart:** unchanged — an over-cap peer is dropped (a clean disconnect); recovery re-dials. No
  durable state involved.
- **Epoch boundary:** n/a — pre-decode memory bound only.

## 12. Mechanical Acceptance Criteria
This slice is complete only when **all** of the following exist and pass in CI (hermetic):

- [x] `session_reassembly_tail_over_cap_fails_closed` (`ade_network` `session::core`) — feeding a
  per-protocol reassembly buffer past 16 MiB without a complete item returns
  `SessionError::ReassemblyBufferOverflow`; no silent truncation, no partial decode.
- [x] `session_reassembly_tail_under_cap_still_drains_complete_item` (`session::core`) — a normal
  under-cap multi-frame item reassembles + drains unchanged.
- [x] `wirepump_lookahead_stops_at_cap` (`ade_node` `node_sync`) — opportunistic draining stops at 256
  buffered blocks (lookahead capped, not unbounded).
- [x] `wirepump_lookahead_cap_preserves_relay_behavior_under_normal_feed` (`node_sync`) — under a normal
  feed the cap is never hit; every block delivered in arrival order.
- [x] `ci_check_node_run_loop_containment.sh` — **byte-unchanged + green** (verified `git diff` vs `main`).
- [x] `ci_check_served_chain_handoff_fence.sh` — **byte-unchanged + green** (verified `git diff` vs `main`).
- [x] `ci_check_live_feed_memory_bounds.sh` — NEW gate; both constants exist as closed literals + are not
  wired to CLI / env / config; green (smoke-tested fail-closed on an injected escape hatch).
- [x] `cargo test -p ade_network` + `cargo test -p ade_node` green (no regression; `ade_node` lib 166→168).

## 13. Failure Modes
- Reassembly buffer over 16 MiB → `SessionError::ReassemblyBufferOverflow` (structured); the caller drops
  the peer. **Fail-fast**; no replay impact (no durable state); never a truncated-but-accepted item.
- Lookahead at cap → opportunistic draining stops; the bounded mpsc back-pressures the pump (the peer's
  send awaits). **Recoverable** (draining resumes as `next_block` consumes); never unbounded.

## 14. Hard Prohibitions
### Inherited Cluster-Level Prohibitions
All PHASE4-N-F-G-E "Forbidden During This Cluster" prohibitions apply.
### Slice-Specific Prohibitions
- **No BLUE change.**
- No **semantic truncation** — an over-cap input is an error, never a truncated-but-accepted item.
- No **swallowing malformed / incomplete input as success** — over-cap → drop, never "no more data".
- No **partial decode** crossing into BLUE — the cap fires before authoritative decode/apply.
- No **unbounded or operator-disable option** — closed constants only; no CLI / env / config escape hatch.
- No **serve / forge / containment change**; both fences byte-unchanged.
- No **live-evidence / BA-02 / rehearsal claim**; no `RO-LIVE` flip.
- No new `SessionError` semantics beyond the additive overflow variant; no wildcard on `SessionError`.

## 15. Explicit Non-Goals
This slice MUST NOT: harden any other DoS surface; touch serve/forge/containment; add a CLI/config option
of any kind; claim live evidence / BA-02 / rehearsal; flip RO-LIVE; modify any BLUE crate; change the wire
message vocabulary.

## 16. Completion Checklist
- [ ] All failure modes are deterministic (over-cap → structured `SessionError`; §13).
- [ ] No TODOs/placeholders in authoritative (BLUE) paths (no BLUE change).
- [ ] CI enforces the strengthened invariant (the four tests + optional grep gate; both fences unchanged).
- [ ] Bounds are closed constants — no CLI/env/config escape hatch.
- [ ] `cargo test -p ade_network` + `-p ade_node` green.

## 17. Review Notes
- **Invariant risk considered:** the cap must fire **before** authoritative decode/apply (no partial
  decode crosses into BLUE) and must not truncate-and-accept (an over-cap item is an error). The GREEN cap
  is a pure byte-count → deterministic.
- **Bound rationale:** 16 MiB is generous headroom over any legit single block/batch (preprod/mainnet
  blocks ~90 KiB) while bounding the attack to a constant; 256 lookahead blocks (4× the channel cap of 64)
  absorbs burst sync before back-pressure. Both are defensive implementation bounds, NOT Cardano semantic
  parameters; tightenable by a future hardening slice, never disableable at runtime.
- **Follow-up implied:** other DoS surfaces are separate hardening slices; the operator-gated rehearsal
  (G-D) + RO-LIVE-01/06 are unaffected by this slice.
