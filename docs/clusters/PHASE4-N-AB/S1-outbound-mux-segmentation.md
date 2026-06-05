# Invariant Slice — PHASE4-N-AB S1: outbound mux segmentation

## §2 Slice Header
- **Slice Name:** outbound mux segmentation
- **Cluster:** PHASE4-N-AB (outbound mux segmentation) — primary invariant **CN-SESS-05**
- **Status:** in progress
- **Cluster Exit Criteria Addressed:** **CE-1** (the whole cluster — one slice).

## §3 Dependencies
CN-SESS-04 / N-M-FRAG (GREEN `session::core` inbound reassembly — the loopback verifier). `mux::frame` (BLUE `encode_frame` / `MuxFrame` / `MAX_PAYLOAD`, unchanged). DC-LIVEMEM-01 (the inbound 16 MiB cap mirrored by `MAX_OUTBOUND_PAYLOAD_BYTES`).

## §4 Intent (invariant impact)
Make `handle_outbound` segment an outbound mini-protocol payload larger than `MAX_PAYLOAD` into ordered ≤`MAX_PAYLOAD` mux frames (the outbound inverse of CN-SESS-04), so a large served BlockFetch `Block` (a Conway block >64 KB) is transmittable instead of erroring. Byte-preserving, lossless, deterministic.

## §5 Scope / What is built (`ade_network::session::core`, GREEN)
- **NEW constant `MAX_OUTBOUND_PAYLOAD_BYTES: usize = 16 * 1024 * 1024`** (16 MiB) — the outbound ceiling, beside `MAX_REASSEMBLY_TAIL_BYTES`. Fixed, closed, non-configurable (no CLI/env/config).
- **`handle_outbound` owns segmentation:**
  - `payload.len() > MAX_OUTBOUND_PAYLOAD_BYTES` → `Err(SessionError::OutboundPayloadTooLarge { len })` (fail closed; the threshold MOVES from `MAX_PAYLOAD` to `MAX_OUTBOUND_PAYLOAD_BYTES`).
  - empty payload → one empty frame (unchanged single-frame behaviour).
  - else → `payload.chunks(MAX_PAYLOAD)`, each chunk encoded via the existing `encode_inner_frame` (each ≤ `MAX_PAYLOAD`, so it passes the per-frame guard), in order. One captured `timestamp` (the existing `handle_outbound` input) is reused for **every** segment — GREEN calls no clock.
  - The ordered frame bytes are concatenated into a single `SessionEffect::SendBytes` (atomic on the outbound channel; the receiver's demux + CN-SESS-04 reassembly splits + reconstructs).
- **`encode_inner_frame` stays strict**: one frame, keeps its `> MAX_PAYLOAD` guard. It remains the sole per-frame encoder authority (wraps `mux::frame::encode_frame`). NO second/alternate frame encoder.
- **NEW gate `ci/ci_check_outbound_segmentation.sh`**.
- **Out of scope:** any BLUE `mux::frame` change; any inbound-reassembly change (CN-SESS-04 unchanged); the handshake single-frame path (untouched — small payloads).

## §6 Execution Boundary (TCB color)
- **GREEN (changed):** `ade_network::session::core` (`handle_outbound`, the `MAX_OUTBOUND_PAYLOAD_BYTES` const). `session::core` is GREEN deterministic glue, not BLUE.
- **BLUE (reused, NOT edited):** `mux::frame::{encode_frame, MuxFrame, MAX_PAYLOAD}`.
- **No new canonical type, no new authority, no schema change.**

## §7 Invariants Preserved
The BLUE per-frame `mux::frame` authority (unchanged). CN-SESS-04 inbound reassembly (unchanged — this is its inverse). DC-CONS-17 block-fetch byte-identity (segmentation is byte-preserving end-to-end). Determinism: `handle_outbound` stays a pure function of `(state, mini_protocol, payload, mode, timestamp)`; no clock, no rand, no HashMap, no float introduced; the timestamp is an input reused across segments.

## §8 Invariants Strengthened or Introduced
**CN-SESS-05 → enforced** (the 7 required tests + `ci_check_outbound_segmentation.sh`). At close: `strengthened_in += "PHASE4-N-AB"` on CN-SESS-04 (receive+send symmetry now complete) + DC-SERVEMEM-01 (the bounded serve can now transmit a large in-range block).

## §11 Replay / Crash / Epoch Validation
`handle_outbound` is a deterministic pure transform; same `(payload, mini_protocol, mode, timestamp)` → byte-identical ordered frames. Byte-preserving: `concat(segment payloads) == payload`. Verified by the loopback round-trip (Ade serves → Ade's own CN-SESS-04 inbound reassembles → byte-identical). No WAL/checkpoint/schema surface.

## §12 Mechanical Acceptance Criteria (the 7 required tests + gate)
`cargo test -p ade_network` green incl.:
1. `outbound_payload_at_max_payload_is_one_frame` — `len = 65535` → exactly 1 frame.
2. `outbound_payload_over_max_payload_segments_into_two` — `len = 65536` → exactly 2 frames (65535 + 1).
3. `outbound_large_payload_reassembles_byte_identical_via_inbound` — a >64 KB payload, segmented then fed back through `session::core` inbound (CN-SESS-04), reconstructs the original byte-for-byte.
4. `outbound_payload_at_upper_bound_is_allowed` — `len = MAX_OUTBOUND_PAYLOAD_BYTES (16 MiB)` exactly → segmented (Ok), not an error.
5. `outbound_payload_over_upper_bound_fails_closed` — `len = MAX_OUTBOUND_PAYLOAD_BYTES + 1` → `OutboundPayloadTooLarge`.
6. `outbound_segment_order_preserved` — concatenated segment payloads equal the original in order.
7. `outbound_segments_keep_same_mini_protocol_id_and_mode` — every segment frame carries the same id + mode.
Gate `ci/ci_check_outbound_segmentation.sh` green: (a) no second/alternate mux frame encoder (exactly one `encode_frame(` call + one `MuxFrame {` construction in `session/core.rs`, both inside `encode_inner_frame`); (b) `encode_inner_frame` retains its `> MAX_PAYLOAD` guard; (c) `handle_outbound` owns segmentation (references `MAX_PAYLOAD`, `MAX_OUTBOUND_PAYLOAD_BYTES`, `chunks`); (d) `MAX_OUTBOUND_PAYLOAD_BYTES` is a fixed `const` literal, no CLI/env/config read.
Cluster-wide: `cargo test -p ade_network -p ade_runtime -p ade_node` green; full `ci/ci_check_*.sh` sweep 136 + 1 = **137 / 0**.

## §14 Hard Prohibitions (cluster §11)
No truncation. No dropping tail bytes. No partial-success send (fully segment+emit or fail closed). No alternate mux frame encoder (reuse `encode_inner_frame`/`encode_frame`). No runtime-configurable/unbounded payload cap (`MAX_OUTBOUND_PAYLOAD_BYTES` fixed `const`). No BLUE change. No RO-LIVE flip. No per-segment clock call in GREEN (one captured `timestamp` reused).

## §15 Explicit Non-Goals
Any inbound-reassembly change (CN-SESS-04); any BLUE `mux::frame` change; live cardano-node interop (a real node reassembling Ade's segments = operator-pass / hardening item 4); the handshake single-frame path; per-connection / multi-message scheduling.
