# Slice S-A7 — Keep-alive + Peer-sharing transition authority

> **Status**: Merged
> **Cluster**: [PHASE4-N-A](cluster.md)

## 2. Slice Header

**Slice Name**: KeepAlive + PeerSharing state machines (BLUE)

**Cluster Exit Criteria Addressed**: none. This slice closes no CE on its own — it ships the two remaining N2N state machines per the cluster's 11-protocol enforcement scope (DC-PROTO-03 closure depends on having every N2N protocol's state machine compiled and tested).

Slice closes the **structural completion gap** for N2N: after this slice, all 6 N2N protocols have a pure-transition state machine + per-protocol event taxonomy. The remaining cluster CEs (CE-N-A-1..4) close via earlier slices' state-machine portions plus S-A9 real-capture corpus; CE-N-A-5 closes via S-A10 live interop. S-A7 has no CE because keep-alive and peer-sharing do not gate consensus correctness — they manage connection health and peer-book population.

Out of scope: all 5 cluster CEs.

**Slice Dependencies**: S-A1, S-A2 (KeepAliveMessage + KeepAliveCookie; PeerSharingMessage + PeerAddress), S-A3..S-A6 (DC-PROTO-06 pattern established).

## 4. Intent

`DC-PROTO-01` (mini-protocol state machines have deterministic transitions) gains its keep-alive and peer-sharing per-protocol enforcement via pure-transition state machines producing health-and-discovery events byte-identical across replays.

`DC-PROTO-06` (no ambient session state in BLUE transitions) gains keep-alive and peer-sharing coverage.

DC-PROTO-02 is NOT strengthened here — these two protocols are non-authoritative; transcript equivalence with cardano-node oracle is desirable but not invariant-load-bearing in the same way ChainSync/BlockFetch/TxSubmission2 are.

## 5. Scope

**Modules** (under `crates/ade_network/src/keep_alive/` and `peer_sharing/`):
- `keep_alive/mod.rs` — barrel
- `keep_alive/state.rs` — `KeepAliveState`; `KeepAliveOutput`; `KeepAliveError`
- `keep_alive/agency.rs` — `KeepAliveAgency`
- `keep_alive/event.rs` — `KeepAliveEvent` (PingSent / PongReceived)
- `keep_alive/transition.rs` — `keep_alive_transition` pure function
- `peer_sharing/mod.rs` — barrel
- `peer_sharing/state.rs` — `PeerSharingState`; `PeerSharingOutput`; `PeerSharingError`
- `peer_sharing/agency.rs` — `PeerSharingAgency`
- `peer_sharing/event.rs` — `PeerSharingEvent` (SharingRequested / PeersShared)
- `peer_sharing/transition.rs` — `peer_sharing_transition` pure function
- `crates/ade_network/tests/keep_alive_event_trace.rs` — integration test
- `crates/ade_network/tests/peer_sharing_event_trace.rs` — integration test

**Persistence / network-visible**: none.

**Out of scope**: other mini-protocols; session composition; real-capture corpus; live interop; rate-limiting policy; peer-book population (peer-sharing emits events; population is RED session-level concern per cluster TCB map).

## 6. Execution Boundary

| Module | Color |
|---|---|
| `ade_network::keep_alive::*` | **BLUE** |
| `ade_network::peer_sharing::*` | **BLUE** |
| `tests/keep_alive_event_trace.rs` | **BLUE** |
| `tests/peer_sharing_event_trace.rs` | **BLUE** |

## 7. Invariants Preserved

`T-DET-01`, `T-CORE-01..03`, `T-INGRESS-01`, `T-CI-01`, `T-BUILD-01`, `T-KEY-01`, `T-BOUND-02`, `T-ERR-01`, `T-ENC-03`, `DC-CORE-01`, `CN-WIRE-07`, `DC-PROTO-02/03/04/05`.

All 16+1 CI scripts continue to PASS.

## 8. Invariants Strengthened

Exactly one invariant family:

- **`DC-PROTO-01`** — protocol state machines have deterministic transitions.

Corollary gaining first per-protocol enforcement: `DC-PROTO-06` (keep-alive and peer-sharing portions).

Registry strengthenings on this commit:
- `DC-PROTO-01`, `DC-PROTO-06`: `code_locus` += keep_alive and peer_sharing module paths; `tests` += new tests. `strengthened_in` already contains `PHASE4-N-A` — no duplicate. No status flips.

## 9. Design Summary

### KeepAlive

```rust
pub fn keep_alive_transition(
    state: KeepAliveState,
    agency: KeepAliveAgency,
    version: KeepAliveVersion,
    msg: KeepAliveMessage,
) -> Result<(KeepAliveState, KeepAliveOutput), KeepAliveError>;
```

**States**:
- `ClientIdle` — client may send KeepAlive or Done
- `ServerHasAgency { cookie: KeepAliveCookie }` — server must respond with matching cookie
- `Done` — terminal

**Transitions**:
- (ClientIdle, Client, KeepAlive(cookie))                  → (ServerHasAgency { cookie }, Event(PingSent { cookie }))
- (ServerHasAgency { cookie }, Server, ResponseKeepAlive(resp_cookie)):
  - if cookie != resp_cookie → MalformedMessage { reason: "ResponseKeepAlive cookie does not match request" }
  - else → (ClientIdle, Event(PongReceived { cookie }))
- (ClientIdle, Client, Done) → (Done, Done)
- Other → IllegalTransition

**Cookie matching is a state invariant**: the request cookie is carried in `ServerHasAgency` and must equal the response cookie. This is the only stateful invariant in keep-alive.

### PeerSharing

```rust
pub fn peer_sharing_transition(
    state: PeerSharingState,
    agency: PeerSharingAgency,
    version: PeerSharingVersion,
    msg: PeerSharingMessage,
) -> Result<(PeerSharingState, PeerSharingOutput), PeerSharingError>;
```

**States**:
- `Idle` — client may request peers or send Done
- `Busy { amount: u8 }` — server must respond with at most `amount` peers
- `Done` — terminal

**Transitions**:
- (Idle, Client, ShareRequest { amount })          → (Busy { amount }, Event(SharingRequested { amount }))
- (Busy { amount }, Server, SharePeers { peers }):
  - if peers.len() > amount as usize → MalformedMessage { reason: "SharePeers count exceeds requested amount" }
  - else → (Idle, Event(PeersShared { peers }))
- (Idle, Client, Done) → (Done, Done)
- Other → IllegalTransition

**Reply size invariant**: peer reply count ≤ requested amount. Empty reply is legal (server has no peers to share).

**No peer-book mutation here**: state machine emits `PeersShared { peers }` event; the RED session layer feeds peers into the peer-book (future cluster).

### Output type per protocol

- `KeepAliveOutput` = `Event(KeepAliveEvent) | Done`
- `PeerSharingOutput` = `Event(PeerSharingEvent) | Done`

### Version threading

Both transitions take their respective `*Version` newtype as explicit input. `MAX_KEEP_ALIVE_VERSION` and `MAX_PEER_SHARING_VERSION` are 100 (mirrors S-A4/S-A5/S-A6 convention).

## 10. Changes Introduced

### Types (new)
- `KeepAliveState` (3 variants), `KeepAliveAgency` (3), `KeepAliveOutput` (2), `KeepAliveEvent` (2), `KeepAliveError` (3)
- `PeerSharingState` (3 variants), `PeerSharingAgency` (3), `PeerSharingOutput` (2), `PeerSharingEvent` (2), `PeerSharingError` (3)

### State Transitions
- Keep-alive state machine
- Peer-sharing state machine

## 11. Replay, Crash, and Epoch Validation

**KeepAlive unit tests** (`keep_alive::transition::tests`):
- `client_ping_then_server_pong_round_trips`
- `cookie_mismatch_returns_malformed`
- `client_done_terminates_session`
- `illegal_message_in_idle_returns_error`
- `wrong_agency_returns_error`
- `version_gating_rejects_out_of_version_message`

**PeerSharing unit tests** (`peer_sharing::transition::tests`):
- `share_request_then_full_reply_round_trips`
- `share_request_then_empty_reply_is_legal`
- `reply_exceeds_amount_returns_malformed`
- `client_done_terminates_session`
- `illegal_message_in_idle_returns_error`
- `wrong_agency_returns_error`
- `version_gating_rejects_out_of_version_message`

**Integration tests**:
- `tests/keep_alive_event_trace.rs::keep_alive_event_trace` — ≥6 scenarios (single ping-pong, sequential ping-pongs, mixed cookie sequence, immediate done, max-cookie-u16 boundary, ping-then-done). 1000-run determinism check.
- `tests/peer_sharing_event_trace.rs::peer_sharing_event_trace` — ≥6 scenarios (request 5 / reply 5, request 5 / reply 0, request 10 / reply 3, mixed IPv4/IPv6 peers, max u8 amount = 255, immediate done). 1000-run determinism check.

## 12. Mechanical Acceptance Criteria

- [ ] `cargo build -p ade_network --all-targets` — clean
- [ ] `cargo test -p ade_network --lib keep_alive::` — 6 unit tests PASS
- [ ] `cargo test -p ade_network --lib peer_sharing::` — 7 unit tests PASS
- [ ] `cargo test -p ade_network --test keep_alive_event_trace` — PASS
- [ ] `cargo test -p ade_network --test peer_sharing_event_trace` — PASS
- [ ] `cargo clippy -p ade_network --all-targets -- -D warnings` — clean
- [ ] All 8 named CI scripts PASS
- [ ] Registry DC-PROTO-01, DC-PROTO-06 strengthened: `code_locus` += paths, `tests` += new tests. No status flips.

## 13. Failure Modes

| Variant | Cause | Recovery |
|---|---|---|
| `KeepAliveError::IllegalTransition / InvalidForVersion / MalformedMessage` | Wrong triple / out-of-version / cookie mismatch | Fail-fast |
| `PeerSharingError::IllegalTransition / InvalidForVersion / MalformedMessage` | Wrong triple / out-of-version / reply overflow | Fail-fast |

All fail-fast. Replay-safe.

## 14. Hard Prohibitions

Inherited cluster + slice-specific:
- All cluster prohibitions
- Mutating peer book (state machine emits events)
- Mutating connection health metrics (events are values)
- Cross-protocol agency type reuse
- `String` errors
- `HashMap`
- Global / static reads
- `#[non_exhaustive]` on any new enum
- `dyn` dispatch
- `unwrap`/`expect`/`panic` outside `#[cfg(test)]`
- Rate-limiting policy in BLUE (mux/transport's job)
- Wall-clock time (cookies are nonces, not timestamps)

## 15. Explicit Non-Goals

- Other mini-protocols
- Rate-limiting / timeout policy (RED session layer)
- Peer-book population (future cluster)
- Persistence (N-D)
- Session composition (S-A9)
- Real-capture corpus (S-A9)
- Live cardano-node interop (S-A10)
- Performance optimization

## 16. Completion Checklist

- [ ] Both state machines pure (no ambient state, no globals, no I/O)
- [ ] Per-protocol agency types (KeepAliveAgency ≠ PeerSharingAgency)
- [ ] Closed enums everywhere (no `#[non_exhaustive]`)
- [ ] Structured errors
- [ ] Version threaded as explicit input
- [ ] 13 unit + 2 integration tests PASS
- [ ] 1000-run determinism check on each integration test
- [ ] All 16+1 CI scripts PASS
- [ ] Registry DC-PROTO-01/06 strengthened (no status flips)
- [ ] No TODOs / placeholders / `unimplemented!()`

## 17. Review Notes

**Invariant risk**:
- **No CE closure**: this slice is structural completion of the N2N protocol set. Skipping it would leave 2/6 N2N protocols without state-machine enforcement.
- **Cookie state**: KeepAlive's `ServerHasAgency { cookie }` is the only stateful invariant in this slice. It's a single u16; carries no replay risk.
- **Peer-book separation**: PeerSharing emits events; peer-book is RED session concern. Boundary is clean per cluster TCB map.

**Assumptions challenged**:
- Considered folding KeepAlive and PeerSharing into one combined module ("control-plane"). Rejected — per-protocol modules + agency types matches the §7 #7 locked decision and keeps cluster-wide grep patterns consistent.
- Considered KeepAlive emitting timestamps. Rejected — BLUE has no wall-clock. The cookie IS the nonce; the session layer (RED) attaches send timestamps if it needs latency metrics.

**Follow-up implied**: S-A9 wires both protocols into the session composition; S-A10 verifies live transcript equivalence.

## 18. Authority Reminder

Authority for invariants in `docs/ade-invariant-registry.toml`; mechanical acceptance in §12.
