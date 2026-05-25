# Invariant Slice — PHASE4-N-G S3

## Slice Header

**Slice Name:** Chain-sync server reducers (`producer_chain_sync_serve` + `producer_chain_sync_advance_tip`) with deterministic-resolution discipline
**Cluster:** PHASE4-N-G
**Status:** In Progress
**CEs addressed:** CE-N-G-3
**Registry flip on merge:** `DC-PROTO-08` → `enforced`
**Dependencies:** N-G-S1 (header projection + `ServerReply`), N-G-S2 (`ServedChainSnapshot`)

---

## Intent

Introduce the pure BLUE reducers that decide *what* the producer-side
chain-sync server announces. Composes the PHASE4-N-A
`chain_sync_transition` for grammar validation; layers the
producer-side decision (FindIntersect answerability + RollForward
header projection from `ServedChainSnapshot`) on top.

Deterministic-resolution discipline (`DC-PROTO-08`): from any
server-agency state, the reducer returns one of legal `RollForward` /
`RollBackward` / `AwaitReply` / structured close-or-error. No
ambiguous silent wait.

---

## The change

### 1. New module `crates/ade_network/src/chain_sync/server.rs` (extended from S1)

Adds the reducer functions + state type next to the closed
`ServerReply` wrapper.

```rust
use ade_types::{Hash32, SlotNo};
use ade_ledger::block_validity::accepted_block_header_bytes;
use ade_ledger::producer::ServedChainSnapshot;

use crate::chain_sync::transition::chain_sync_transition;
use crate::chain_sync::agency::ChainSyncAgency;
use crate::chain_sync::state::{ChainSyncOutput, ChainSyncState, ChainSyncError};
use crate::codec::chain_sync::{ChainSyncMessage, Point, Tip};
use crate::codec::version::ChainSyncVersion;

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ProducerChainSyncServerState {
    pub state: ChainSyncState,           // delegates grammar state to N-A
    pub last_announced: Option<(SlotNo, Hash32)>,  // cursor into served chain
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProducerServerError {
    /// The peer's message violated the chain-sync grammar at the
    /// underlying state-machine layer. Carries the upstream typed
    /// error verbatim.
    Grammar(ChainSyncError),
    /// Project failure: an admitted AcceptedBlock could not be
    /// header-projected. Should never fire for AcceptedBlock-derived
    /// bytes (self_accept already proved decode-validity); the
    /// variant exists for strict totality.
    HeaderProject,
    /// The peer issued a client-originated message that the producer
    /// does not serve in the narrow scope (currently empty — the
    /// chain-sync client surface is fully covered).
    Unsupported(&'static str),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ServerStep {
    /// Client said `Done`; session terminated.
    Done,
    /// The server has emitted a legal reply (the orchestrator
    /// encodes and sends it). State has already advanced.
    Reply(ServerReply),
}

/// Process one client-originated chain-sync message. Pure, total,
/// deterministic.
pub fn producer_chain_sync_serve(
    state: ProducerChainSyncServerState,
    in_msg: ChainSyncMessage,
    served: &ServedChainSnapshot,
    version: ChainSyncVersion,
) -> Result<(ProducerChainSyncServerState, ServerStep), ProducerServerError>;

/// Poll the reducer for a deferred RollForward after the server has
/// previously emitted AwaitReply (state == MustReply) or the client
/// has just issued RequestNext (state == CanAwait). Returns
/// `Some(RollForward)` when the served chain has a block beyond the
/// cursor; `None` when the client has not yet asked (state == Idle)
/// or the cursor is already at the served-chain head.
pub fn producer_chain_sync_advance_tip(
    state: ProducerChainSyncServerState,
    served: &ServedChainSnapshot,
) -> Result<(ProducerChainSyncServerState, Option<ServerReply>), ProducerServerError>;
```

### 2. CI gate `ci/ci_check_chain_sync_server_closure.sh`

- Forbids `wall-clock` / `rand` / `tokio` / `HashMap` in
  `chain_sync/server.rs`.
- Forbids any function in `server.rs` that returns a raw
  `ChainSyncMessage` directly — outgoing replies MUST go through
  `ServerReply::into_message()`.
- Forbids construction of `RollForward` whose `header` did not come
  from `accepted_block_header_bytes` (positive grep for the call site).

---

## Reducer semantics (locked)

### `producer_chain_sync_serve(state, in_msg, &served, version)`

Composes `chain_sync_transition(state.state, Client, version, in_msg)`
to validate grammar + advance state. Then:

| Incoming client msg | After grammar transition | Producer-side decision |
|----|----|----|
| `RequestNext` (legal only when `state == Idle`) | `state' = CanAwait` | If served chain has a block with key > `last_announced` → emit `RollForward(header, tip)` (state → Idle, last_announced updated). Else → emit `AwaitReply` (state → MustReply). |
| `FindIntersect { points }` (legal only when `state == Idle`) | `state' = Intersect` | If any point matches a served-chain key → emit `IntersectFound { point, tip }` (state → Idle). Else → emit `IntersectNotFound { tip }` (state → Idle). |
| `Done` (legal only when `state == Idle`) | `state' = Done` | Return `ServerStep::Done`. |
| Any other (non-Idle state + client msg, server msg, etc.) | grammar reject | Return `Err(ProducerServerError::Grammar(_))`. |

`tip` is constructed from the served-chain head: `(slot, hash, block_no)`
where block_no is the count of admitted blocks ≤ that key — derived
deterministically.

### `producer_chain_sync_advance_tip(state, &served)`

| Current state | Decision |
|----|----|
| `Idle` | Return `(state, None)`. Client has not asked. |
| `CanAwait` or `MustReply` | If served chain has block > `last_announced` → emit `RollForward(header, tip)`; advance state → Idle, update cursor. Else → return `(state, None)`. |
| `Intersect` | Return `(state, None)`. The intersect leg is resolved via `serve`, not `advance_tip`. |
| `Done` | Return `(state, None)`. Session is over. |

---

## §12 Mechanical Acceptance Criteria (named tests)

In `crates/ade_network/src/chain_sync/server.rs` (tests).

- `producer_chain_sync_serve_request_next_idle_yields_roll_forward_when_served_has_block`
- `producer_chain_sync_serve_request_next_idle_yields_await_reply_when_served_empty`
- `producer_chain_sync_serve_find_intersect_known_point_yields_intersect_found`
- `producer_chain_sync_serve_find_intersect_unknown_point_yields_intersect_not_found`
- `producer_chain_sync_serve_done_terminates_session`
- `producer_chain_sync_serve_rejects_illegal_grammar_pair`
- `producer_chain_sync_advance_tip_idle_yields_none`
- `producer_chain_sync_advance_tip_can_await_yields_roll_forward_when_block_available`
- `producer_chain_sync_advance_tip_must_reply_yields_roll_forward_when_block_available`
- `producer_chain_sync_advance_tip_can_await_yields_none_when_cursor_at_head`
- `producer_chain_sync_serve_roll_forward_header_equals_accepted_block_header_bytes`
- `producer_chain_sync_serve_replays_byte_identical_over_corpus`

CI: `ci/ci_check_chain_sync_server_closure.sh` (new).

---

## §14 Hard Prohibitions

- No `pub fn` in `chain_sync/server.rs` that returns a raw
  `ChainSyncMessage` — outgoing replies must come from
  `ServerReply::into_message()`.
- No construction of `RollForward { header, .. }` whose `header` did
  not come from `accepted_block_header_bytes`.
- No `wall-clock` / `rand` / `tokio` / `HashMap`.
- No silent unbounded wait — server-agency states must return either
  a legal reply or a structured close-or-error.
- No `panic!` / `unimplemented!` / `todo!` in any reducer arm.

---

## §15 Explicit Non-Goals

- Block-fetch server reducer (S4).
- GREEN adapter / RED orchestrator (S5/S6).
- Multi-peer coordination (S6).
- Rollback announcement on chain reorg — narrow scope: the
  AcceptedBlock pipeline doesn't reorg the served chain; rollback
  semantics belong to the receive-side bridge cluster.

---

## Replay obligations

The reducers' outputs are deterministic functions of `(state, in_msg,
&served, version)` — replay-equivalence is structural. S5 covers the
end-to-end session-transcript replay corpus.

---

## Authority reminder

If this slice conflicts with the project's normative specifications
or the invariant registry, those win.
