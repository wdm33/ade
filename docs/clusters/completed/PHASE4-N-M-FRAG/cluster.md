# PHASE4-N-M-FRAG — Session-reducer per-mini-protocol payload reassembly (cluster doc)

> **Status:** Planning. Single-slice strengthening of PHASE4-N-L
> session reducer. Closes the
> `blocked_until_session_reducer_per_protocol_reassembly` gate
> that holds `DC-EVIDENCE-01` + `RO-LIVE-05` at
> `enforced_scaffolding` after the PHASE4-N-M-A1.1 closure
> surfaced this defect against a fully-synced peer.

**Predecessors:** PHASE4-N-L (wire-protocol session reducer +
mux frame layer), PHASE4-N-M-A1.1 (full preprod UTxO seed
import). PHASE4-N-M-C (live operator pass scaffolding).

**Successor:** none planned. After FRAG closes, the C5 operator
pass against a fully-synced peer produces a real BlockAdmitted +
AgreementVerdict::Agreed transcript and DC-EVIDENCE-01 +
RO-LIVE-05 flip from `enforced_scaffolding` to `enforced`.

## §1 Primary invariant

> The session reducer's `Connected`-state inbound path
> reassembles mini-protocol payload bytes across consecutive
> mux frames carrying the SAME mini-protocol id. The reducer
> emits exactly one `SessionEffect::DeliverPeerFrame` per
> COMPLETE CBOR item in the wire stream — never per mux frame.
> Same canonical input stream → byte-identical
> `DeliverPeerFrame` sequence. Truncated bytes mid-item stay
> buffered until the next mux frame arrives; malformed bytes
> at item boundaries fail-closed via `SessionError`.

### Why this matters

Cardano N2N mux frames carry a u16 payload length (max 65535
bytes). Conway preprod blocks routinely exceed this; the
cardano-node block-fetch server splits one `MsgBlock` CBOR item
across multiple mux frames bearing the same protocol id.
Without per-mini-protocol reassembly, the consumer sees a
truncated CBOR item in the first frame and either:
- Decodes it as malformed (today: `BlockFetchDecode` →
  pump exits → `admission_shutdown { upstream_dropped }`), OR
- Worse, accepts a malformed half-decode silently.

Per `[[feedback-real-interop-finds-codec-bugs]]`: synthetic
single-frame test vectors miss this entirely. The capture
binary `crates/ade_network/src/bin/capture_block_fetch.rs::
read_for_protocol` already implements the correct pattern —
per-protocol byte buffer + `try_decode` predicate. Porting it
into the session reducer is mechanical.

## §2 Scope

### In scope

- Extend `ade_network::session::state::ConnectedState` with a
  closed `ProtoBuffers` struct: ten `Vec<u8>` payload
  accumulators, one per `AcceptedMiniProtocol` variant.
- Extend `ade_network::session::core::drain_connected_frames`:
  when a complete mux frame's payload arrives for protocol P,
  append the payload bytes to `proto_buffers[P]`; loop and
  attempt to peel COMPLETE CBOR items off the head of the
  buffer via `ade_codec::cbor::skip_item`; emit one
  `DeliverPeerFrame { mini_protocol: P, payload: item_bytes }`
  per complete item; on `CodecError::UnexpectedEof` stop
  (truncated, wait for more bytes); on any other `CodecError`
  fail-closed via a new `SessionError::ProtocolPayloadMalformed
  { protocol: u16, detail: &'static str }` variant.
- Preserve the existing post-handshake-handshake-frame rejection
  (still emitted BEFORE buffer append).
- New unit tests covering: fragmented chain-sync IntersectFound,
  fragmented block-fetch Block, interleaved chain-sync +
  block-fetch fragments, pipelined multi-item single mux frame,
  malformed CBOR mid-item fail-closed, per-protocol buffer
  isolation across protocols.

### Out of scope (explicit)

- Outbound payload fragmentation. Our pump sends one
  `OutboundFrame` request per message; the session reducer
  encodes it into ONE mux frame. If a future need arises to
  send messages exceeding 65535 bytes (e.g. our own block-fetch
  Server responses), that's a separate slice (`PHASE4-N-M-FRAG-OUT`).
- Re-architecting the mux layer's frame boundary detection.
  `pull_one_frame` works correctly today; this slice adds
  per-mini-protocol reassembly ABOVE the mux layer.
- Handshake-state reassembly. Handshake messages are small
  (< 65535 bytes) and the existing `drain_handshake_frames`
  path is unchanged.

## §3 Slice index

| Slice | Purpose | New rules / strengthenings |
|---|---|---|
| **F1** | Per-mini-protocol payload reassembly | new CN-SESS-04 + DC-SESS-06; strengthens CN-SESS-01..03 + DC-SESS-01..05 (T-DET-01 by extension) |

Single-slice cluster — see `F1.md`.

## §4 Exit criteria (cluster-level MACs)

1. `CN-SESS-04` (release) registered: single per-mini-protocol
   reassembly authority lives in
   `ade_network::session::core::drain_connected_frames` —
   nowhere else in the workspace mutates `proto_buffers`.
2. `DC-SESS-06` (derived) registered: same canonical inbound
   byte stream → byte-identical `DeliverPeerFrame` sequence
   across two reducer runs (replay-equivalence).
3. `ci/ci_check_session_proto_reassembly.sh` — single drain
   authority + closed `ProtoBuffers` shape (no `HashMap`).
4. New unit tests in
   `ade_network::session::core::tests::*` covering at minimum:
   - `fragmented_chain_sync_message_assembles_one_deliver`
   - `fragmented_block_fetch_block_assembles_one_deliver`
   - `interleaved_chain_sync_and_block_fetch_fragments_stay_isolated`
   - `pipelined_two_chain_sync_messages_in_one_mux_frame_emit_two_delivers`
   - `malformed_cbor_at_item_boundary_returns_session_error`
   - `proto_buffers_isolation_across_all_ten_protocols`
   All green.
5. `cargo test --workspace` clean — no regression in existing
   session-reducer / mux / codec tests.
6. `cargo test -p ade_testkit` clean — replay-equivalence
   preserved across all eras.
7. `cargo clippy --workspace --all-targets` shows no new
   warnings in `ade_network/src/session/`.
8. Replay-equivalence smoke against the real capture corpus:
   `cargo test -p ade_network --test block_fetch_real_capture_corpus`
   passes (the existing corpus is single-frame, so it serves
   as a regression bound; FRAG must not break it).
9. C5 live operator pass against the fully-synced docker peer
   with `ADE_LIVE_REQUIRE_BLOCK_ADMITTED=1` produces:
   - ≥ 1 `block_admitted` event with `consensus_inputs_fingerprint`,
   - ≥ 1 `agreement_verdict { kind: "agreed" }`,
   - 0 `agreement_verdict { kind: "diverged" }`,
   - 0 `block_admitted` for any block whose hash differs from
     the peer's announced hash at the same slot.
   Captured transcript committed at
   `docs/evidence/phase4-n-m-c-operator-pass-transcript.jsonl`.
10. `DC-EVIDENCE-01` + `RO-LIVE-05` flipped from
    `enforced_scaffolding` to `enforced`; `open_obligation`
    removed; `strengthened_in` += `PHASE4-N-M-FRAG`;
    `evidence_notes` updated.
11. Commit + push to `main` with the project-override trailer.

## §5 Hard prohibitions

- No `HashMap` / `HashSet` for `ProtoBuffers` — closed
  enum-indexed array (or struct with named fields). Iteration
  order must be deterministic.
- No re-decoding of items already drained. Once we emit
  `DeliverPeerFrame { payload }`, the `payload` bytes are
  consumed from the buffer.
- No silent drop of truncated bytes. `UnexpectedEof` mid-item
  means "wait for more", NOT "discard the buffer".
- No second `ProtoBuffers` field elsewhere — the single
  authority is `ConnectedState::proto_buffers`.
- No coupling to clock / rand / float / OS ordering — the
  reducer remains pure.
- No outbound fragmentation in this slice. Outbound payload
  size limit (`MAX_PAYLOAD` check) stays unchanged.

## §6 Replay obligations preserved

- T-DET-01 (determinism supreme law) — strengthened: now
  covers fragmented inbound streams. Two replays of the same
  byte-chunk sequence (including the fragmented mux frames)
  yield byte-identical `DeliverPeerFrame` lists.
- DC-SESS-01..05 — unchanged in shape; FRAG strengthens the
  Connected-state reducer without changing handshake state
  semantics.
- DC-CONS-08 (admit-replay-equivalence) — unchanged. FRAG only
  affects the wire layer; the BLUE admit path is untouched.

## §7 References

- Finding doc: `docs/evidence/phase4-n-m-c-operator-pass-README.md` §8
  + commit `222292c`.
- Pattern source: `crates/ade_network/src/bin/capture_block_fetch.rs::read_for_protocol`.
- Doctrine: [[feedback-shell-must-not-overstate-semantic-truth]],
  [[feedback-real-interop-finds-codec-bugs]],
  [[project-phase4-n-m-frag-next]].
- Predecessor closures: `f988ed1` (N-L), `8843e20` (N-M-C),
  `03d1d24` (N-M-A1.1), `222292c` (finding).
