# Invariant Slice — PHASE4-N-L-LIVE S1

**Slice Name:** GREEN `ade_node::live_log::{event, writer, mod}` — closed `LiveLogEvent` enum + hand-rolled JSON serializer + writer.
**Cluster:** PHASE4-N-L-LIVE
**Status:** In Progress
**CEs addressed:** CE-N-L-LIVE-2 (¬P-1 enforcement), CE-N-L-LIVE-4 (I-7 enforcement).
**Dependencies:** none.

## Intent

Pin the wire-only JSONL event vocabulary as a closed sum that
type-level forbids any admission/agreement claim. The writer is
the SOLE site that serializes events; a CI grep forbids the
disallowed event-name string literals anywhere in `ade_node/src/`.

## Scope

- `crates/ade_node/src/live_log/event.rs` — `LiveLogEvent`
  closed sum with 7 variants (per invariants §1 I-1) + closed
  `WireOnlyShutdownReason` + closed `PeerDialFailureKind`.
- `crates/ade_node/src/live_log/writer.rs` —
  `LiveLogWriter<W: Write>` with `emit(event)` that serializes
  + flushes one JSON object per line.
- `crates/ade_node/src/live_log/mod.rs` — re-exports.
- `crates/ade_node/src/lib.rs` — `pub mod live_log`.
- `ci/ci_check_wire_only_event_vocabulary_closed.sh` — grep
  gate.

## Design

```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LiveLogEvent {
    NodeStarted { mode: ModeTag, peer_count: u32 },
    PeerDialStarted { peer: String },
    HandshakeOk { peer: String, negotiated_version: u16 },
    PeerTipRead { peer: String, slot: u64, hash_hex: String, block_no: u64 },
    PeerDialFailed { peer: String, kind: PeerDialFailureKind, detail: String },
    WireSmokeComplete { admission_enabled: bool, peer_count_ok: u32, peer_count_failed: u32 },
    NodeShutdown { reason: WireOnlyShutdownReason },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModeTag { WireOnly /* + future: Admission */ }

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WireOnlyShutdownReason {
    TipReadComplete,
    SignalReceived,
    PeerDialFailure,
    LedgerSeedUnavailable,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PeerDialFailureKind {
    TcpConnectFailed,
    HandshakeRejected,
    TipReadTimeout,
    TipReadProtocolError,
    OrchestratorDropped,
}

pub struct LiveLogWriter<W: Write> { sink: W }
impl<W: Write> LiveLogWriter<W> {
    pub fn new(sink: W) -> Self;
    pub fn emit(&mut self, event: &LiveLogEvent) -> io::Result<()>;
    pub fn flush(&mut self) -> io::Result<()>;
}
```

Serialization is a hand-rolled JSON writer (the surface is tiny;
no serde-derive dep added). Each emit writes one line:
`{"event":"<kind>",<fields>}\n`. Flush after every emit so a
SIGINT mid-line is impossible.

## §12 Mechanical Acceptance Criteria

- [ ] `live_log_writer_emits_one_object_per_line` — round-trip:
  emit 5 events of mixed variants, parse the file as
  `\n`-separated JSON, expect 5 objects with the right `event`
  discriminator.
- [ ] `live_log_writer_flushes_after_every_emit` — emit one
  event, observe the byte is in the sink before the next
  call.
- [ ] `live_log_writer_serializes_node_started_canonically` —
  pin the exact string for `NodeStarted { mode: WireOnly,
  peer_count: 1 }` so future serializer changes are caught.
- [ ] `live_log_event_round_trips_through_jq_check` — invoke
  `jq -c '.event' < log` (skip on CI machines without jq) +
  assert no parse errors.
- [ ] `live_log_writer_two_runs_are_byte_identical` — same
  event vector → same bytes.
- [ ] `ci/ci_check_wire_only_event_vocabulary_closed.sh` —
  greps `ade_node/src/` for the forbidden literals
  (`agreement_verdict`, `admitted_block`, `ledger_applied`,
  `projection_updated`). Production-body grep (strips
  `#[cfg(test)]`-block).

## §14 Hard Prohibitions

- No serde / serde_json dep in `ade_node/Cargo.toml` for this
  slice.
- No `panic!()` / `unwrap()` in production code.
- No emitting any of the four forbidden event names.
- No HashMap / float / tokio in `live_log/*` (pure GREEN).

## §15 Non-Goals

- No tokio integration (S2).
- No actual dialer (S2).
- No admission events (RO-LIVE-05).
