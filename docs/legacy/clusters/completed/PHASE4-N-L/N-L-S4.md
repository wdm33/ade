# Invariant Slice — PHASE4-N-L S4

**Slice Name:** GREEN `session::handshake_driver` — pure driver over `Transport` trait.
**Cluster:** PHASE4-N-L
**Status:** In Progress
**CEs addressed:** part of CE-N-L-2 (CN-SESS-02 — exercised by the driver).
**Dependencies:** S1, S2, S3.

## Intent

Drive the existing BLUE `n2n_transition` to completion against a `Transport` trait abstraction. The trait is the seam between sync core and async I/O; tests use an in-memory transport, S7 uses a TCP transport.

## Scope

- `crates/ade_network/src/session/handshake_driver.rs` —
  - `pub trait Transport { fn read(...) -> Result<Vec<u8>, HandshakeError>; fn write(...) -> Result<(), HandshakeError>; }`
  - `pub fn run_n2n_handshake_initiator(transport: &mut dyn Transport, our: VersionTable) -> Result<NegotiatedN2n, HandshakeError>;`
  - `pub fn run_n2n_handshake_responder(transport: &mut dyn Transport, our: Vec<(u16, VersionData)>) -> Result<NegotiatedN2n, HandshakeError>;`

`NegotiatedN2n { version: u16, params: VersionData }`.

## Design

Loop: encode → write → read → parse → call `n2n_transition` → check `HandshakeState`. Exit on `Accepted` or `Rejected` (HandshakeError). Pure; transport is the only sided effect.

The function uses the existing `mux::frame::{encode,decode}_frame` for envelope framing — handshake messages ride mini-protocol id 0 (handshake) on the same mux fabric.

## §12 Mechanical Acceptance Criteria

- [ ] `handshake_initiator_accepts_when_responder_supports_proposed_version` — in-memory transport pair; initiator + responder converge on `Accepted` with the highest mutually-supported version.
- [ ] `handshake_initiator_rejects_on_no_overlap` — responder supports a disjoint version set → `HandshakeError::NoOverlap` (or equivalent existing variant).
- [ ] `handshake_driver_is_pure_given_transport_trace` — replaying a captured transport read trace yields the same `NegotiatedN2n`.

## §14 Hard Prohibitions

- No tokio / SystemTime / Instant / rand.
- No second `n2n_transition` implementation.
- No silent fallback on `HandshakeError`.

## §15 Non-Goals

- No async; the trait is synchronous. The RED S7 wrapper bridges async ↔ sync via `block_on`/equivalent at the call boundary (or runs the driver inside `tokio::task::block_in_place`).
- No N2C in this slice (mirror function lands when N2C client work begins).
