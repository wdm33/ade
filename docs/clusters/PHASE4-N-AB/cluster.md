# PHASE4-N-AB — Outbound mux segmentation (CN-SESS-05)

> **Pre-RO-LIVE hardening, item 2.** Makes a single large BlockFetch payload (a Conway block >64 KB) actually transmittable: the `--mode node` serve path must segment an outbound mini-protocol payload larger than the mux SDU limit into ordered frames — the outbound inverse of the CN-SESS-04 inbound reassembly Ade already performs. Pairs with PHASE4-N-AA (item 1 bounded *how many* blocks per request; this makes *one large* block sendable). User-confirmed scope + invariant wording 2026-06-06; tracked in `project_pre_rolive_hardening_queue.md` item 2.

## §1 Primary invariant (CN-SESS-05)
Outbound mini-protocol payloads larger than `MAX_PAYLOAD` are segmented into ordered mux frames, each no larger than `MAX_PAYLOAD`, preserving mini-protocol id, mode, and byte order. Concatenating segment payloads reconstructs the original payload exactly. Payloads above `MAX_OUTBOUND_PAYLOAD_BYTES` fail closed. Segmentation uses the single existing frame encoder authority and is the outbound inverse of CN-SESS-04 inbound reassembly.

Constants (fixed, closed, non-configurable — no CLI/env/config):
- `MAX_PAYLOAD = 65535` (`u16::MAX`; the mux SDU payload limit, `ade_network::mux::frame`).
- `MAX_OUTBOUND_PAYLOAD_BYTES = 16 MiB` (the outbound ceiling; symmetric with the inbound `MAX_REASSEMBLY_TAIL_BYTES = 16 MiB` / DC-LIVEMEM-01).

## §2 The problem (proven from code)
`crates/ade_network/src/session/core.rs` (GREEN `session::core` — the framing reducer):
- `handle_outbound` (line ~325) returns `Err(SessionError::OutboundPayloadTooLarge)` when `payload.len() > MAX_PAYLOAD` — it does NOT segment.
- `encode_inner_frame` (line ~380) has the same guard (correct as a single-frame encoder).

So serving any mini-protocol message larger than 64 KB fails. A Conway BlockFetch `Block` carrying a >64 KB block cannot be sent. Yet the **inbound** side already reassembles multi-frame payloads per mini-protocol (`session::core` `proto_buffers`, CN-SESS-04, bounded by the 16 MiB `MAX_REASSEMBLY_TAIL_BYTES` / DC-LIVEMEM-01). The asymmetry is the bug: Ade can *receive* segmented large payloads but cannot *send* them.

## §3 The design — segment at the outbound scheduling layer, single encoder authority
- **`encode_inner_frame` stays strict**: one frame only; keeps its `> MAX_PAYLOAD` guard. It remains the sole per-frame encoder authority (wraps `mux::frame::encode_frame`). No second/alternate frame encoder is introduced.
- **`handle_outbound` owns segmentation**: `payload.len() <= MAX_PAYLOAD` → one frame (unchanged). `MAX_PAYLOAD < len <= MAX_OUTBOUND_PAYLOAD_BYTES` → split into `ceil(len / MAX_PAYLOAD)` chunks, encode each via `encode_inner_frame` (each chunk satisfies the per-frame guard), emit the frames in order. `len > MAX_OUTBOUND_PAYLOAD_BYTES` → `Err(OutboundPayloadTooLarge)` (fail closed).
- **Determinism (GREEN)**: segmentation is a pure function of `(payload, mini_protocol, mode, timestamp)`. The `timestamp: u32` is an INPUT to `handle_outbound` (the caller captures it once); GREEN never calls a clock. **All segments of one outbound message reuse that single captured timestamp** — no per-segment clock call, no per-segment timestamp drift. Mini-protocol id + mode are identical across all segments of a message.
- **Receiver**: a conformant peer (and Ade's own inbound CN-SESS-04 path) concatenates consecutive same-protocol frame payloads and decodes complete items — so Ade's segmented output round-trips.

## §4 Normative anchors
- Invariant registry `docs/ade-invariant-registry.toml` — adds **CN-SESS-05** (tier derived; declared → enforced at S1) + the `MAX_OUTBOUND_PAYLOAD_BYTES` constant.
- Cross-ref: **CN-SESS-04** (inbound reassembly — the inverse), **DC-LIVEMEM-01** (the 16 MiB inbound cap this bound mirrors), **CN-WIRE-08** (tag-24 block-fetch envelope — the payloads being segmented), **DC-SERVEMEM-01** (item 1 — the per-request block cap this composes with), **DC-CONS-17** (block-fetch byte-identity — preserved: segmentation is byte-preserving).
- Source: `project_pre_rolive_hardening_queue.md` item 2.

## §5 Entry conditions (what prior clusters guarantee)
- **CN-SESS-04 / N-M-FRAG**: the GREEN `session::core` inbound reassembly (`proto_buffers`) that reconstructs multi-frame payloads — the loopback verifier for this cluster's output.
- **DC-LIVEMEM-01 / N-F-G-E**: the inbound 16 MiB `MAX_REASSEMBLY_TAIL_BYTES` cap — the symmetric precedent for `MAX_OUTBOUND_PAYLOAD_BYTES`.
- **`mux::frame` (BLUE)**: `encode_frame` / `MuxFrame` / `MAX_PAYLOAD` — the per-frame wire authority (unchanged).
- **DC-SERVEMEM-01 / N-AA**: the bounded serve range — each served `Block` message is a separate outbound payload this cluster segments if large.

## §6 TCB color map (FC/IS partition)
- **GREEN (changed):** `ade_network::session::core` (`handle_outbound` segmentation). `session::core` is GREEN deterministic glue (framing reducer), not BLUE authority — it frames bytes, defines no truth.
- **BLUE (reused, NOT edited):** `ade_network::mux::frame` (`encode_frame`, `MAX_PAYLOAD`). No BLUE change.
- **No new canonical type, no new authority, no schema change.** `MAX_OUTBOUND_PAYLOAD_BYTES` is a GREEN closed constant.

## §7 Slices
| Slice | Scope | CE | Registry → enforced | TCB |
|---|---|---|---|---|
| **S1** | `handle_outbound` segments `MAX_PAYLOAD < len <= MAX_OUTBOUND_PAYLOAD_BYTES` into ordered ≤`MAX_PAYLOAD` frames (single captured timestamp reused; same id+mode; byte-preserving) via the strict `encode_inner_frame`; `> MAX_OUTBOUND_PAYLOAD_BYTES` fails closed. New constant `MAX_OUTBOUND_PAYLOAD_BYTES`. New gate `ci_check_outbound_segmentation.sh`. | CE-1 | CN-SESS-05 | GREEN |

## §8 Cluster Exit Criteria (all mechanical)
- **CE-1 (S1, CN-SESS-05):** `cargo test -p ade_network` green incl. the 7 required tests:
  1. `payload_len = MAX_PAYLOAD (65535)` → exactly **one** frame.
  2. `payload_len = 65536` → exactly **two** frames (65535 + 1).
  3. a large >64 KB payload **reassembles byte-identical** through Ade's own inbound path (CN-SESS-04 `session::core` loopback).
  4. `payload_len = MAX_OUTBOUND_PAYLOAD_BYTES (16 MiB)` exactly → **allowed** (segmented).
  5. `payload_len = MAX_OUTBOUND_PAYLOAD_BYTES + 1` → `OutboundPayloadTooLarge` (fail closed).
  6. segment **order preserved** (concatenated payloads == original, in order).
  7. **all frames keep the same mini-protocol id and mode**.
- **CE-1 gate:** `ci/ci_check_outbound_segmentation.sh` green — (a) NO second/alternate mux frame encoder (`encode_frame` has one authority; no parallel encoder in `session`); (b) `encode_inner_frame` still has its `MAX_PAYLOAD` guard; (c) `handle_outbound` owns segmentation (references `MAX_PAYLOAD` + `MAX_OUTBOUND_PAYLOAD_BYTES` + chunks); (d) `MAX_OUTBOUND_PAYLOAD_BYTES` is a fixed literal, NOT runtime-configurable (no CLI/env/config read).
- **Cluster-wide:** `cargo test -p ade_network -p ade_runtime -p ade_node` green; full `ci/ci_check_*.sh` sweep 136 + 1 = **137 / 0**.

## §9 Replay obligations
Segmentation is a deterministic pure function of `(payload, mini_protocol, mode, timestamp)` with `timestamp` an input (no clock in GREEN). Same inputs → byte-identical ordered frames. Byte-preserving: `concat(segment payloads) == original` (DC-CONS-17 block-fetch byte-identity preserved end-to-end). No canonical type / WAL / checkpoint / schema change. Command: `cargo test -p ade_network` + the loopback reassembly test.

## §10 Invariants
- **Adds:** CN-SESS-05 (tier derived; declared → enforced at S1).
- **Strengthens** (`strengthened_in += "PHASE4-N-AB"` at close): CN-SESS-04 (the outbound inverse now exists — receive+send symmetry); DC-SERVEMEM-01 (the bounded serve can now actually transmit a large in-range block).
- **Preserves (NOT changed):** the BLUE `mux::frame` per-frame authority; DC-LIVEMEM-01 (the inbound cap, mirrored not modified); CN-WIRE-08 (tag-24 envelope — segmented, not re-encoded); DC-CONS-17 (byte-identity).

## §11 Forbidden during this cluster (hard boundaries — user-set)
- No truncation.
- No dropping tail bytes.
- No partial-success send (a message either fully segments+emits or fails closed).
- No alternate mux frame encoder (segmentation reuses the single `encode_inner_frame` / `encode_frame` authority).
- No runtime-configurable / unbounded payload cap (`MAX_OUTBOUND_PAYLOAD_BYTES` is a fixed compile-time constant).
- No BLUE change.
- No RO-LIVE flip.
- (Determinism) No per-segment clock call in GREEN — one captured `timestamp` input reused for all segments of a message.

## §12 Open questions
- **Timestamp (resolved):** one captured `timestamp` per outbound message, reused for all segments (the existing `handle_outbound(timestamp)` input; GREEN calls no clock). The mux SDU header carries a per-segment timestamp field, but the Cardano mux does not require monotonic-per-segment within a message; reuse is spec-compatible and deterministic.
- **SDU size (resolved):** segment at `MAX_PAYLOAD = 65535` (the protocol SDU limit; receivers accept up to it). Not a smaller configured `sduSize` — fixed at the max.

## §13 Close record
*(Open — filled at `/cluster-close` once CE-1 is green.)*
