# Slice S-A2 — Protocol message codec authority (all 11 mini-protocols)

> **Status**: Proposed
> **Cluster**: [PHASE4-N-A](cluster.md)

## 2. Slice Header

**Slice Name**: All-protocols closed message taxonomy + CBOR codecs (BLUE)

**Cluster**: PHASE4-N-A

**Cluster Exit Criteria Addressed**:

S-A2 ships the codec layer that **all** of CE-N-A-1..CE-N-A-4 depend on but does not close any of them on its own (those CEs require byte-identical comparison against cardano-node-captured corpus, which arrives in S-A3..S-A6 via per-protocol oracle harnesses). S-A2 closes the **codec self-consistency** prerequisite: every protocol-visible message decodes into exactly one closed, versioned type and round-trips byte-identically through Ade's own encoder.

CEs in scope: none directly. S-A2 is precondition-ship for CE-N-A-1..4.

CEs explicitly out of scope: CE-N-A-1, CE-N-A-2, CE-N-A-3, CE-N-A-4, CE-N-A-5.

**Slice Dependencies**: S-A1 (substrate — `ade_network` crate + sync-only CI gate must be live).

---

## 4. Intent

`CN-WIRE-07` (every protocol-visible message decodes into exactly one closed, versioned message type) becomes mechanically enforced after this slice merges. Plus the surface-coverage rules `DC-PROTO-03` (full N2N coverage) and `DC-PROTO-04` (full N2C coverage) gain their first concrete enforcement, since this slice ships the closed enums for all 11 protocols. `T-ENC-03` (valid encodings round-trip byte-identically) is strengthened by per-protocol round-trip tests over every defined message variant.

---

## 5. Scope

**Modules / crates** (all new under `crates/ade_network/src/codec/`):
- `codec/mod.rs` (module barrel; re-exports)
- `codec/error.rs` — closed `CodecError` enum (`Truncated`, `UnknownTag`, `InvalidUtf8`, `InvalidProtocolMessage`, `InvalidIntegerRange`, `MalformedCbor`)
- `codec/primitives.rs` — small helpers wrapping `ade_codec`'s `minicbor` re-exports (the rest of the codec depends only on the local module)
- `codec/handshake.rs` — N2N `HandshakeMessage` enum + version table types
- `codec/n2c_handshake.rs` — N2C `HandshakeMessage` enum + version table types
- `codec/chain_sync.rs` — `ChainSyncMessage`, `Tip`, `Point`
- `codec/block_fetch.rs` — `BlockFetchMessage`, `Range`
- `codec/tx_submission.rs` — `TxSubmission2Message`, inventory shapes
- `codec/keep_alive.rs` — `KeepAliveMessage` + cookie newtype
- `codec/peer_sharing.rs` — `PeerSharingMessage`, `PeerAddress`
- `codec/local_chain_sync.rs` — mirrors `chain_sync` for N2C (LocalChainSync)
- `codec/local_tx_submission.rs` — `LocalTxSubmissionMessage` + `TxAcceptance`, `TxRejection`
- `codec/local_state_query.rs` — `LocalStateQueryMessage` + query envelope (closed wire grammar only; semantic content is opaque payload per locked decision §7 #3)
- `codec/local_tx_monitor.rs` — `LocalTxMonitorMessage`
- `codec/version.rs` — typed per-protocol version markers: `N2NVersion`, `N2CVersion`, `ChainSyncVersion`, `BlockFetchVersion`, `TxSubmission2Version`, `KeepAliveVersion`, `PeerSharingVersion`, `LocalChainSyncVersion`, `LocalTxSubmissionVersion`, `LocalStateQueryVersion`, `LocalTxMonitorVersion` (closed newtypes, not loose integers)

**State machines affected**: none. S-A2 ships **types and codecs only** — no transition logic.

**Persistence impact**: none.

**Network-visible impact**: none — codecs exist but are not wired to any session yet. S-A9 composes them.

**External dependency added**: `ade_codec` (already a workspace member; `ade_network` gains it as a dep). Pallas-network is **not** added — per the locked decision, pallas is CI oracle only and only appears in the testkit as a comparison source.

**Out of scope**:
- Any state machine (S-A3..S-A8)
- Any byte-identical-against-cardano-node oracle test (S-A3..S-A6 via captured corpus)
- Any session composition (S-A9)
- Live interop (S-A10)
- Capturing a real cardano-node frame corpus (S-A9)
- Semantic interpretation of LocalStateQuery payloads (N-F)
- Re-exporting any pallas-network types into `ade_network`

---

## 6. Execution Boundary

| Module | Color | Constraint |
|---|---|---|
| `ade_network::codec::*` (every submodule) | **BLUE** | Pure CBOR `bytes ↔ Message` transformations. Sync-only. No I/O. No `HashMap` / `HashSet` / float / wall-clock. No `String` errors. |

No GREEN or RED additions in this slice.

---

## 7. Invariants Preserved

- `T-DET-01` — deterministic encode/decode
- `T-CORE-01`, `T-CORE-02`, `T-CORE-03` — pure transitions, no nondeterminism, no I/O in authoritative paths
- `T-ENC-01` — canonical encoding
- `T-INGRESS-01` — typed ingress (codec is the named ingress chokepoint for mini-protocol bytes)
- `T-CI-01` — invariants mechanically enforced
- `T-BUILD-01`, `T-KEY-01`, `T-BOUND-02`, `T-ERR-01`
- `DC-CORE-01` — no async in BLUE (enforced by S-A1's `ci_check_no_async_in_blue.sh`)
- All existing CN-* and DC-* rules

All 16 CI scripts must continue to PASS, with `ade_network/src/codec/` now scanned as BLUE.

---

## 8. Invariants Strengthened

Exactly one invariant family strengthens via mechanical enforcement:

- **`CN-WIRE-07`** — "Each protocol-visible message must decode into one closed, versioned message type"

Direct corollaries that gain their first per-protocol enforcement from S-A2:
- `DC-PROTO-03` — full N2N surface (6 protocols)
- `DC-PROTO-04` — full N2C surface (5 protocols)
- `T-ENC-03` — valid encodings round-trip byte-identically (per-protocol round-trip tests)

When this slice's commit lands, the four rules gain `crates/ade_network/src/codec/*.rs` in their `code_locus` and `cargo test -p ade_network --lib codec::` in their `tests` array. None flips status to `enforced` yet — byte-identicality with cardano-node is gated by S-A3..S-A6.

---

## 9. Design Summary

**Closed-enum-per-protocol**. Each protocol module defines a closed enum (no `#[non_exhaustive]`):
```rust
pub enum HandshakeMessage {
    ProposeVersions(VersionTable),
    AcceptVersion(N2NVersion, VersionData),
    Refuse(RefuseReason),
    QueryReply(VersionTable),
}
```

**Codec interface per protocol** — explicit functions, no `dyn`, no generic codec trait:
```rust
pub fn encode_<protocol>_message(msg: &<Protocol>Message) -> Vec<u8>;
pub fn decode_<protocol>_message(bytes: &[u8]) -> Result<<Protocol>Message, CodecError>;
```

**Typed per-protocol version markers** (locked decision §7 #1) — distinct newtypes, not interchangeable.

**Closed error taxonomy** (`codec::error::CodecError`) — variants with `&'static str` context fields. No `String`, no `anyhow`.

**CBOR primitives source**: this slice uses `ade_codec`'s `minicbor` re-exports. Direct `minicbor::` imports outside `ade_codec` are forbidden by `ci_check_ingress_chokepoints.sh` (with the `ade_plutus/src/evaluator.rs` allowlist).

**Closed wire grammar for LocalStateQuery** (locked decision §7 #3): the LSQ codec models the closed envelope (Acquire/Acquired/Failure/Query/Result/Release/ReAcquire/Done). `QueryPayload(Vec<u8>)` and `ResultPayload(Vec<u8>)` carry opaque bytes; the codec verifies CBOR envelope structure and version-gating but does NOT interpret query semantics. Ledger semantic interpretation is cluster N-F.

---

## 10. Changes Introduced

### Types (new)

Closed enums and supporting types across 13 modules. ~70 message variants total across the 11 protocols. 11 typed `*Version` newtypes. `CodecError` enum (6 variants). Auxiliary types (`Point`, `Tip`, `Range`, `RefuseReason`, `PeerAddress`, `TxAcceptance`, `TxRejection`, etc.).

### State Transitions

None.

### Persistence

None.

### Removal / Refactors

None.

---

## 11. Replay, Crash, and Epoch Validation

**Round-trip tests added** (one per protocol):
- `codec::handshake::tests::roundtrip_every_variant`
- `codec::n2c_handshake::tests::roundtrip_every_variant`
- `codec::chain_sync::tests::roundtrip_every_variant`
- `codec::block_fetch::tests::roundtrip_every_variant`
- `codec::tx_submission::tests::roundtrip_every_variant`
- `codec::keep_alive::tests::roundtrip_every_variant`
- `codec::peer_sharing::tests::roundtrip_every_variant`
- `codec::local_chain_sync::tests::roundtrip_every_variant`
- `codec::local_tx_submission::tests::roundtrip_every_variant`
- `codec::local_state_query::tests::roundtrip_every_variant`
- `codec::local_tx_monitor::tests::roundtrip_every_variant`

**Negative tests added** (per protocol):
- `decode_rejects_unknown_tag`
- `decode_rejects_truncated_input`
- `decode_rejects_invalid_utf8_in_text_fields` (where applicable)

**No real-corpus replay**: byte-identicality against cardano-node-captured frames lands in S-A3..S-A6.

**Crash / restart**: not applicable — codecs are stateless pure functions.

**Epoch boundary**: not applicable.

---

## 12. Mechanical Acceptance Criteria

- [ ] `cargo build --workspace --all-targets` — clean
- [ ] `cargo test -p ade_network --lib codec::` — every `roundtrip_every_variant` test PASSES for all 11 protocols, plus negative tests
- [ ] `cargo clippy -p ade_network --all-targets -- -D warnings` — clean
- [ ] `bash ci/ci_check_no_async_in_blue.sh` — PASS
- [ ] `bash ci/ci_check_module_headers.sh` — PASS (contract headers on every codec file)
- [ ] `bash ci/ci_check_no_semantic_cfg.sh` — PASS
- [ ] `bash ci/ci_check_no_signing_in_blue.sh` — PASS
- [ ] `bash ci/ci_check_hash_uses_wire_bytes.sh` — PASS
- [ ] `bash ci/ci_check_ingress_chokepoints.sh` — PASS (no direct `minicbor::` or `from_cbor` outside `ade_codec`/`ade_plutus::evaluator` allowlist)
- [ ] `bash ci/ci_check_dependency_boundary.sh` — PASS
- [ ] `bash ci/ci_check_constitution_coverage.sh` — PASS
- [ ] 11 protocols × at least the minimum-spec message variants: Handshake 4, ChainSync 8, BlockFetch 6, TxSubmission2 6, KeepAlive 3, PeerSharing 3, N2C-Handshake 4, LocalChainSync 8, LocalTxSubmission 4, LocalStateQuery 8, LocalTxMonitor 7
- [ ] Registry entries CN-WIRE-07, DC-PROTO-03, DC-PROTO-04, T-ENC-03 have `code_locus` updated to include codec paths and `tests` arrays include the new round-trip tests

---

## 13. Failure Modes

| Variant | Cause | Recovery |
|---|---|---|
| `CodecError::Truncated { needed, got }` | Input ran out before required field decoded | Caller responsibility; session layer wraps |
| `CodecError::UnknownTag { protocol, tag }` | CBOR variant tag not in closed enum for protocol/version | Fail-fast; session SHOULD close connection |
| `CodecError::InvalidUtf8` | Non-UTF-8 in text field | Fail-fast |
| `CodecError::InvalidProtocolMessage { protocol, reason }` | Structure violates grammar | Fail-fast |
| `CodecError::InvalidIntegerRange { field, value }` | Integer out of protocol range | Fail-fast |
| `CodecError::MalformedCbor` | Raw CBOR parse failure | Fail-fast |

None affect replay equivalence — decoding is pure.

---

## 14. Hard Prohibitions

### Inherited cluster-level prohibitions

All from `docs/clusters/PHASE4-N-A/cluster.md` "Forbidden During This Cluster" apply.

### Slice-specific prohibitions

- `String` errors anywhere in `ade_network::codec` (`CodecError` is structured)
- `anyhow`, runtime-formatted error messages — `&'static str` only
- `HashMap` / `HashSet` / floating point / wall-clock
- Direct `minicbor::` imports outside `ade_codec` re-exports
- `pallas-network` types in `ade_network::codec`
- `TxId` redefinition — use `ade_types::TxId`
- Generic `Codec<P>` trait
- Per-protocol versions as loose `u16`
- `#[non_exhaustive]` on protocol message enums (must be closed)
- `dyn Trait` dispatch over messages
- `unsafe`, `transmute`, raw pointers

---

## 15. Explicit Non-Goals

- No state machines (S-A3..S-A8)
- No oracle-corpus byte-identicality tests (S-A3..S-A6)
- No session composition (S-A9)
- No live interop (S-A10)
- No flow control or windowing
- No LocalStateQuery semantic interpretation
- No performance optimization
- No feature flags or `cfg` switches in BLUE
- No `pallas-network` re-exports
- No `minicbor` re-exports from `ade_network::codec`

---

## 16. Completion Checklist

- [ ] All 11 protocols closed enums (no `#[non_exhaustive]`)
- [ ] All variants round-trip byte-identically through Ade's codec
- [ ] Every `decode_*` rejects unknown tags / truncated / invalid UTF-8 deterministically
- [ ] No async, no I/O, no clock, no `HashMap`, no float, no `String` errors
- [ ] 11 typed `*Version` newtypes
- [ ] `CodecError` closed with `&'static str` context only
- [ ] All 16 CI scripts PASS at extended BLUE scope
- [ ] Registry `code_locus` / `tests` updated for CN-WIRE-07, DC-PROTO-03/04, T-ENC-03
- [ ] No TODOs, no placeholders, no `unimplemented!()`
- [ ] Spec-deviation judgment calls documented in §17

---

## 17. Review Notes

**Invariant risk considered**:
- S-A2's MAC is **self-consistency only**, not byte-identicality with cardano-node. A correct-shape but wrong-byte codec passes S-A2's MAC. Byte-level correctness lands in S-A3..S-A6 via per-protocol corpus comparison.
- Some protocols (TxSubmission2, LocalStateQuery) have version-dependent message shapes. S-A2 ships the union across cardano-node 10.6.2 supported versions with version-gated decoder branches.
- LocalStateQuery payload bytes are opaque per locked decision §7 #3.

**Assumptions challenged**:
- Considered `minicbor::Encode/Decode` derive macros. Rejected — ties codec to a specific minicbor wire layout. Hand-rolled is more explicit and audit-friendly.
- Considered generic `Codec<P>` trait. Rejected per locked decision §7 #7.

**Follow-up slices implied**:
- S-A3..S-A6: per-protocol oracle corpus + byte-identicality test (closes CE-N-A-1..4)
- S-A9: session composes codec + mux + state machines + transport

---

## 18. Authority Reminder

Authority for invariants belongs to `docs/ade-invariant-registry.toml`. Authority for mechanical acceptance belongs to the named tests/CI checks in §12. This slice doc is a planning aid.
