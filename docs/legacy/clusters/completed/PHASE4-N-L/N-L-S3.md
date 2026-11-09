# Invariant Slice — PHASE4-N-L S3

**Slice Name:** GREEN `session::demux` — partial-frame buffer + per-protocol fanout.
**Cluster:** PHASE4-N-L
**Status:** In Progress
**CEs addressed:** part of CE-N-L-3 (DC-SESS-01) and CE-N-L-5 (DC-SESS-03 ordering).
**Dependencies:** S1.

## Intent

Provide a pure `FrameBuffer` type that accepts inbound byte chunks, emits complete `MuxFrame`s, and preserves per-protocol ordering. Used by S2's `step` to split a byte chunk into one or more frames.

## Scope

- `crates/ade_network/src/session/demux.rs` — `FrameBuffer` (pure ring/queue) + `pull_complete_frames`.

## Design

```rust
pub struct FrameBuffer {
    pending: Vec<u8>, // append-only; trimmed when full frames are popped
}

impl FrameBuffer {
    pub fn new() -> Self;
    pub fn append(&mut self, bytes: &[u8]);
    pub fn pull_one_frame(&mut self) -> Result<Option<MuxFrame>, MuxError>;
    pub fn pull_all_complete_frames(&mut self) -> Result<Vec<MuxFrame>, MuxError>;
    pub fn pending_bytes(&self) -> usize;
}
```

`pull_one_frame` returns `Ok(None)` when fewer than `HEADER_LEN` bytes are buffered or when a header is parseable but the full payload hasn't arrived. Decode errors propagate (the session core treats them as peer-fatal).

## §12 Mechanical Acceptance Criteria

- [ ] `frame_buffer_accumulates_partial_then_emits_full` — feed bytes in 1-byte chunks → single frame at the end.
- [ ] `frame_buffer_emits_multiple_frames_when_buffered` — feed two frames worth of bytes → both pop in order.
- [ ] `frame_buffer_rejects_truncated_payload_oversized_length` — header says length 10, only 5 payload bytes → `Ok(None)` until completed.
- [ ] `frame_buffer_propagates_invalid_mini_protocol_id` — header with mode-bit-set id → `MuxError::InvalidMiniProtocolId`.
- [ ] `frame_buffer_two_runs_deterministic` — same byte sequence → same frame sequence.

## §14 Hard Prohibitions

- No tokio / SystemTime / Instant.
- No HashMap / HashSet.
- No unsafe.

## §15 Non-Goals

- No I/O. No async. No timeouts.
