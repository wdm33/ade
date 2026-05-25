# Invariant Slice — PHASE4-N-G S4

## Slice Header

**Slice Name:** Block-fetch server reducer (`producer_block_fetch_serve`)
**Cluster:** PHASE4-N-G
**Status:** In Progress
**CEs addressed:** CE-N-G-4 (DC-CONS-17 + DC-CONS-18 foundation; full enforcement at CE-N-G-5)
**Registry effects on merge:** none directly — DC-CONS-17/18 flip at S5 when end-to-end replay closes them.
**Dependencies:** N-G-S1 (`ServerReply` for block-fetch), N-G-S2 (`ServedChainSnapshot::range_bytes`)

---

## Intent

Introduce the pure BLUE reducer that responds to client
`RequestRange { from, to }` over the producer's served chain. Composes
the PHASE4-N-A `block_fetch_transition` for grammar validation; layers
the producer-side range lookup on top.

Every `Block { bytes }` payload the reducer emits is sourced from the
served-chain snapshot byte-identically — no re-encoding, no parsing
+ re-serialization. This is the structural enforcement of DC-CONS-17:
served bytes equal admitted `AcceptedBlock.as_bytes()`.

---

## The change

### 1. New module `crates/ade_network/src/block_fetch/server.rs` (extended from S1)

Adds the reducer + state + trait seam alongside the closed `ServerReply`
wrapper.

```rust
use ade_types::{Hash32, SlotNo};

use crate::block_fetch::agency::BlockFetchAgency;
use crate::block_fetch::state::{BlockFetchError, BlockFetchState};
use crate::block_fetch::transition::block_fetch_transition;
use crate::codec::block_fetch::{BlockFetchMessage, Point, Range};
use crate::codec::version::BlockFetchVersion;

/// Read-side trait over the producer's served chain for block-fetch.
/// Wraps `ServedChainSnapshot::range_bytes`; production impl in
/// `ade_runtime` GREEN adapter (S5).
pub trait ServedRangeLookup {
    /// Inclusive range of `(slot, hash, bytes)` in BTreeMap order.
    fn range_bytes(
        &self,
        from: (SlotNo, Hash32),
        to: (SlotNo, Hash32),
    ) -> Vec<(SlotNo, Hash32, Vec<u8>)>;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProducerBlockFetchServerState {
    pub state: BlockFetchState,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProducerBlockFetchServerError {
    Grammar(BlockFetchError),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BlockFetchServerStep {
    /// Client said ClientDone; session terminated.
    Done,
    /// Sequence of server replies for the orchestrator to encode +
    /// transmit in order. Possibly empty (no-op state ack).
    Replies(Vec<ServerReply>),
}

pub fn producer_block_fetch_serve(
    state: ProducerBlockFetchServerState,
    in_msg: BlockFetchMessage,
    served: &dyn ServedRangeLookup,
    version: BlockFetchVersion,
) -> Result<(ProducerBlockFetchServerState, BlockFetchServerStep), ProducerBlockFetchServerError>;
```

### 2. CI gate `ci/ci_check_block_fetch_server_closure.sh`

- Forbids wall-clock / rand / tokio / HashMap in
  `block_fetch/server.rs`.
- Forbids pub fn returning raw `BlockFetchMessage` (except
  `ServerReply::into_message`).
- Positive presence: `producer_block_fetch_serve` +
  `ServedRangeLookup` trait.

---

## Reducer semantics (locked)

### `producer_block_fetch_serve(state, in_msg, &served, version)`

Composes `block_fetch_transition(state.state, Client, version, in_msg)`
for grammar validation. Then:

| Incoming client msg | After grammar transition | Producer-side decision |
|----|----|----|
| `RequestRange { from, to }` where both endpoints are `Point::Block` | `state' = Busy` | Lookup `&served.range_bytes(from_key, to_key)`. If empty → emit `[NoBlocks]`, state → Idle. If non-empty → emit `[StartBatch, Block(b1), ..., Block(bN), BatchDone]`, state → Idle. |
| `RequestRange` containing `Point::Origin` | `state' = Busy` | Narrow scope: producer does not serve genesis. Emit `[NoBlocks]`, state → Idle. |
| `ClientDone` (legal only when `state == Idle`) | `state' = Done` | Return `BlockFetchServerStep::Done`. |
| Any other (grammar violation) | reject | Return `Err(Grammar(_))`. |

Range key derivation: `Point::Block { slot, hash }` maps to `(slot, hash)`.

---

## §12 Mechanical Acceptance Criteria (named tests)

In `crates/ade_network/src/block_fetch/server.rs` (tests).

- `producer_block_fetch_serve_request_range_in_chain_yields_start_batch_blocks_batch_done`
- `producer_block_fetch_serve_request_range_empty_in_chain_yields_no_blocks`
- `producer_block_fetch_serve_request_range_outside_chain_yields_no_blocks`
- `producer_block_fetch_serve_block_bytes_equal_accepted_block_as_bytes`
- `producer_block_fetch_serve_request_range_with_origin_endpoint_yields_no_blocks`
- `producer_block_fetch_serve_client_done_terminates_session`
- `producer_block_fetch_serve_rejects_illegal_grammar_pair`
- `producer_block_fetch_serve_replays_byte_identical_over_corpus`

CI: `ci/ci_check_block_fetch_server_closure.sh` (new).

---

## §14 Hard Prohibitions

- No `pub fn` in `block_fetch/server.rs` returning raw
  `BlockFetchMessage` — must go through `ServerReply::into_message`.
- No construction of `Block { bytes }` from anything other than a
  `ServedRangeLookup` lookup output (positive grep gate).
- No wall-clock / rand / tokio / HashMap in production code.
- No silent unbounded wait.

---

## §15 Explicit Non-Goals

- GREEN adapter, RED orchestrator (S5/S6).
- N2C local block-fetch — out of scope.
- Streaming-batch back-pressure — narrow scope: full batch is
  returned synchronously; back-pressure is orchestrator scope (S6).

---

## Replay obligations

The reducer's outputs are deterministic functions of `(state, in_msg,
&served, version)` — replay-equivalence is structural. S5 covers the
end-to-end session-transcript replay corpus.

---

## Authority reminder

If this slice conflicts with the project's normative specifications
or the invariant registry, those win.
