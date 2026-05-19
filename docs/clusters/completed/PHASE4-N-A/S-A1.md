# Slice S-A1 — Mux/framing + `ade_network` scaffold + sync-only CI gate

> **Status**: Merged
> **Cluster**: [PHASE4-N-A](cluster.md) — Network mini-protocols

## 2. Slice Header

**Slice Name**: Substrate — `ade_network` crate + RED mux transport + BLUE mux framing + sync-only BLUE CI gate

**Cluster**: PHASE4-N-A

**Status**: Proposed

**Cluster Exit Criteria Addressed**:

S-A1 is a substrate slice and does not directly close any headline CE. It enables the BLUE/RED mechanical partition that all subsequent BLUE slices (S-A2 through S-A8) depend on, and ships the mux framing that S-A9 (session wiring) composes against.

CEs explicitly out of scope: CE-N-A-1, CE-N-A-2, CE-N-A-3, CE-N-A-4, CE-N-A-5 (none closed by this slice).

**Slice Dependencies**: none (first slice of PHASE4-N-A; only depends on PHASE4-N-D being closed, which is enforced by main branch state)

---

## 4. Intent

`DC-CORE-01` (BLUE authoritative crates are sync-only) becomes mechanically enforced after this slice merges. Before S-A1, the rule is declared but not gated; after S-A1, any `async fn`, `.await`, `tokio::`, `async_std::`, `Future`, `futures::`, task spawn, async channel, or timer in any BLUE-classified module fails CI. This makes the BLUE/RED partition for cluster PHASE4-N-A — and the rest of Phase 4 — impossible to violate by accident before any BLUE state-machine slice ships.

---

## 5. Scope

**Modules / crates**:
- New crate: `crates/ade_network/`
  - `src/lib.rs` (re-exports, top-level color discipline header)
  - `src/mux/mod.rs` (RED transport module root)
  - `src/mux/frame.rs` (**BLUE** — pure encode/decode of Ouroboros mux frames)
  - `src/mux/transport.rs` (**RED** — tokio-based socket open/read/write skeleton; no protocol logic)
  - `src/codec/mod.rs` (BLUE placeholder — populated by S-A2)
  - `src/handshake/mod.rs`, `src/chain_sync/mod.rs`, `src/block_fetch/mod.rs`, `src/tx_submission/mod.rs`, `src/keep_alive/mod.rs`, `src/peer_sharing/mod.rs`, `src/n2c/mod.rs` (BLUE placeholders — populated by S-A3..S-A8)
  - `src/session/mod.rs` (RED placeholder — populated by S-A9)
- Workspace `Cargo.toml`: add `ade_network` to `members`
- `.idd-config.json`: extend `core_paths` to include 8 BLUE `ade_network` submodules
- 6 BLUE-scope CI scripts: add ade_network BLUE submodules to scope (`ci_check_module_headers.sh`, `ci_check_no_semantic_cfg.sh`, `ci_check_no_signing_in_blue.sh`, `ci_check_hash_uses_wire_bytes.sh`, `ci_check_ingress_chokepoints.sh`, `ci_check_dependency_boundary.sh`)
- New CI script: `ci/ci_check_no_async_in_blue.sh` (grep-based, enforces DC-CORE-01)
- Cluster doc refinement: `docs/clusters/PHASE4-N-A/cluster.md` TCB row for `ade_network::mux` split into `mux::frame` (BLUE) + `mux::transport` (RED)

**State machines affected**: none — S-A1 ships substrate, not protocol logic.

**Persistence impact**: none.

**Network-visible impact**: none — `transport.rs` opens/reads/writes raw bytes but has no protocol logic. No wire interaction is performed by this slice; `transport.rs` provides the I/O surface that S-A9 composes against.

**Out of scope**:
- Any protocol message type or codec (S-A2)
- Any mini-protocol state machine (S-A3..S-A8)
- Frame corpus capture (S-A9)
- Live cardano-node interop (S-A10)
- Flow control / windowing logic (deferred to S-A9 when the session composes mux + state machines)

---

## 6. Execution Boundary

| Module | Color | Constraint |
|---|---|---|
| `ade_network::lib` | **GREEN** | Re-exports and crate-level deny attributes; no logic |
| `ade_network::mux::frame` | **BLUE** | Pure `encode_frame(header, payload) -> Vec<u8>` and `decode_frame(&[u8]) -> Result<MuxFrame, MuxError>`. Sync-only, no I/O, no async. |
| `ade_network::mux::transport` | **RED** | tokio-based socket I/O scaffold. `async fn` allowed here; nowhere else in this slice. |
| `ade_network::mux::mod` | **GREEN** | Module barrel; no logic |
| `ade_network::codec::mod` (placeholder) | **BLUE** | Empty pub mod, contract header, no symbols. Populated by S-A2. |
| `ade_network::{handshake,chain_sync,block_fetch,tx_submission,keep_alive,peer_sharing,n2c}::mod` (placeholders) | **BLUE** | Empty pub mod, contract header, no symbols. Populated by S-A3..S-A8. |
| `ade_network::session::mod` (placeholder) | **RED** | Empty pub mod, no symbols. Populated by S-A9. |
| `ci/ci_check_no_async_in_blue.sh` (new) | CI | Grep-based scanner |

**Color resolution**: `mux::frame` is BLUE because Ouroboros mux frame encode/decode is a pure transformation `(header, payload) ↔ bytes` with no I/O or async. Previous cluster doc lumped all of `mux` as RED; this slice refines that into `mux::frame` (BLUE) + `mux::transport` (RED). Cluster doc updated accordingly in this slice's diff.

---

## 7. Invariants Preserved

All currently-enforced invariants must continue to pass after this slice merges:

- `T-DET-01` — determinism in authoritative paths
- `T-CORE-01` — explicit pure state transitions
- `T-CORE-02` — no wall-clock / unseeded rand / floats / nondeterministic collections in authoritative paths
- `T-CORE-03` — no I/O in authoritative paths
- `T-INGRESS-01` — ingress paths through named chokepoints
- `T-CI-01` — invariants are mechanically enforced
- `T-BOUND-02` — BLUE crates never depend on RED crates
- `T-BUILD-01` — no semantic cfg attributes in BLUE
- `T-ENC-01`, `T-ENC-03` — canonical encoding rules
- `T-KEY-01` — no signing in BLUE
- All `DC-*` and `CN-*` rules currently enforced

Existing 16 CI scripts must continue to PASS unmodified semantics. The 6 BLUE-scope scripts pick up the new `ade_network` BLUE submodules in scope; the substrate's empty placeholder modules must satisfy headers/forbidden-pattern checks.

---

## 8. Invariants Strengthened

Exactly one invariant family becomes mechanically enforced by this slice:

- `DC-CORE-01` — "BLUE authoritative crates are sync-only: no `async fn`, `.await`, `tokio::`, `async_std::`, `Future`, `futures::`, task spawning, async channels, or timers. Async runtime concerns are confined to RED transport/runtime code."

Enforcement mechanism: `ci/ci_check_no_async_in_blue.sh` scans every path declared in `.idd-config.json` `core_paths` (the BLUE substring matches) for any of the forbidden async patterns. Detection is grep-based for the initial implementation; the rule is precise enough that grep false-positive risk is low for the BLUE codec/state-machine modules. Graduate to a syn-based scanner if the grep version produces false positives on legitimate non-async usage of the matched substrings.

Cross-referenced existing rules whose enforcement gains this gate:
- `T-CORE-02` (DC-CORE-01 is the version-specific strengthening for the async family)
- `DC-PROTO-01` (transition determinism precondition)
- `DC-PROTO-06` (no ambient session state in BLUE transitions)

---

## 9. Design Summary

**`ade_network::mux::frame`** (BLUE):
- Pure encode/decode of Ouroboros mux frame headers per the cardano-node protocol spec
- Frame header shape: 4-byte timestamp (microseconds mod 2^32), 16-bit (1-bit mode + 15-bit mini-protocol id), 16-bit payload length
- Payload: opaque `Vec<u8>` (up to 65535 bytes per frame); the protocol-specific decoding lives in `ade_network::codec` (S-A2) and the mini-protocol state machines (S-A3..S-A8)
- Types: `MuxHeader { timestamp: u32, mode: MuxMode, mini_protocol_id: MiniProtocolId, length: u16 }`, `MuxMode { Initiator, Responder }`, `MiniProtocolId(u16)`, `MuxFrame { header: MuxHeader, payload: Vec<u8> }`, `MuxError { ... }`
- Functions: `encode_frame(frame: &MuxFrame) -> Vec<u8>`, `decode_frame(bytes: &[u8]) -> Result<(MuxFrame, &[u8]), MuxError>` (returns frame + remaining bytes)

**`ade_network::mux::transport`** (RED):
- Tokio-based: `pub async fn open_tcp(addr: SocketAddr) -> io::Result<MuxTransport>`, `pub async fn open_unix(path: &Path) -> io::Result<MuxTransport>`
- Stub `MuxTransport` with `read_raw(buf: &mut Vec<u8>)` and `write_raw(bytes: &[u8])` — sockets only, no frame parsing (frame parsing is `mux::frame` BLUE)
- This module is the ONLY place tokio appears in `ade_network` in this slice
- `transport.rs` declares the runtime expectation; downstream session composition (S-A9) chooses how to drive it

**`ci/ci_check_no_async_in_blue.sh`**:
- Reads `.idd-config.json` `core_paths`
- For each declared BLUE path, greps for forbidden patterns
- Forbidden regex set (initial): `\basync\s+fn\b`, `\.await\b`, `\btokio\b`, `\basync_std\b`, `\bfutures::`, `\bFuture\b` (with comment-line filter), task spawn patterns, async timer patterns
- Exit 1 on any match; print the offending file:line
- Comment-line filter via `grep -v '^\s*//'` to reduce false positives

**`.idd-config.json` `core_paths` additions**:
```
crates/ade_network/src/mux/frame.rs
crates/ade_network/src/codec/
crates/ade_network/src/handshake/
crates/ade_network/src/chain_sync/
crates/ade_network/src/block_fetch/
crates/ade_network/src/tx_submission/
crates/ade_network/src/keep_alive/
crates/ade_network/src/peer_sharing/
crates/ade_network/src/n2c/
```

Note: `mux/frame.rs` is added at file granularity (not directory) so `mux/transport.rs` and `mux/mod.rs` (RED + GREEN respectively) are not BLUE-scoped.

**6 BLUE-scope CI scripts**:
Each existing script has a `BLUE_CRATES` array at crate-level granularity. For `ade_network`, the per-submodule BLUE/RED split makes whole-crate scoping unsuitable. Choose path-list per-script (hard-coded BLUE paths inside `ade_network`) for S-A1 — refactor to read from `.idd-config.json` `core_paths` in a follow-up hygiene slice if duplication becomes painful.

---

## 10. Changes Introduced

### Types (new)

- `MuxHeader`, `MuxMode`, `MiniProtocolId`, `MuxFrame`, `MuxError` (all in `ade_network::mux::frame`)
- No types in `transport.rs` beyond a stub `MuxTransport` wrapper around an async-readable handle

### State Transitions

None. S-A1 ships substrate; no state machine yet.

### Persistence

None.

### Removal / Refactors

None.

---

## 11. Replay, Crash, and Epoch Validation

**Replay tests added**:
- `ade_network::mux::frame::tests::frame_roundtrip_byte_identical` — encode then decode every variant of `MuxFrame` (every mini-protocol id ∈ {handshake, chain-sync, block-fetch, tx-submission2, keep-alive, peer-sharing, plus 5 N2C ids}, every `MuxMode`, payload sizes 0/1/255/256/65535)
- `ade_network::mux::frame::tests::frame_decode_rejects_short_input` — decoding fewer than 8 header bytes yields `MuxError::Truncated`
- `ade_network::mux::frame::tests::frame_decode_rejects_oversized_payload` — header claims length > remaining bytes → `MuxError::Truncated`
- `ade_network::mux::frame::tests::frame_decode_returns_remaining_bytes` — concatenated frames decode one at a time

**Synthetic vs captured corpus**: S-A1 uses synthetic test vectors derived from the spec (header byte layouts are fully specified by the Ouroboros mini-protocol docs). Real captured-corpus replay is S-A9's job.

**Crash/restart behavior**: not applicable — `mux::frame` is stateless; `mux::transport` is a thin socket wrapper with no per-process state to recover.

**Epoch boundary behavior**: not applicable.

---

## 12. Mechanical Acceptance Criteria

S-A1 is complete only when **all** of the following exist and pass in CI:

- [ ] `cargo build --workspace --all-targets` — `ade_network` crate compiles cleanly with the new substrate
- [ ] `cargo test -p ade_network --lib mux::frame` — all 4 frame round-trip and rejection tests PASS
- [ ] `cargo clippy -p ade_network --all-targets -- -D warnings` — no clippy warnings
- [ ] `bash ci/ci_check_no_async_in_blue.sh` — PASS against the substrate (the only async lives in `mux::transport`, which is RED; BLUE paths are clean)
- [ ] `bash ci/ci_check_no_async_in_blue.sh` — fails deterministically when fed a test fixture containing `async fn` in a BLUE path (a synthetic test invocation, NOT a real BLUE-path violation)
- [ ] `bash ci/ci_check_module_headers.sh` — PASS with `ade_network` BLUE submodules in scope (every BLUE source file has the contract header)
- [ ] `bash ci/ci_check_no_semantic_cfg.sh` — PASS with `ade_network` BLUE submodules in scope
- [ ] `bash ci/ci_check_no_signing_in_blue.sh` — PASS
- [ ] `bash ci/ci_check_hash_uses_wire_bytes.sh` — PASS
- [ ] `bash ci/ci_check_ingress_chokepoints.sh` — PASS (no CBOR ingress yet — substrate has no `from_cbor` paths)
- [ ] `bash ci/ci_check_dependency_boundary.sh` — PASS (`ade_network` should depend on `ade_types` BLUE; not on any RED crate)
- [ ] `bash ci/ci_check_constitution_coverage.sh` — PASS (registry unchanged; this slice doesn't add registry entries)
- [ ] Cluster doc updated: `docs/clusters/PHASE4-N-A/cluster.md` TCB row for `ade_network::mux` shows the `frame` (BLUE) / `transport` (RED) split

---

## 13. Failure Modes

| Failure | Shape | Recovery |
|---|---|---|
| `MuxError::Truncated { needed, got }` | Structured enum variant | Caller responsibility — fail-fast at the frame boundary; the session layer (S-A9) wraps with retry-or-die per Ouroboros semantics |
| `MuxError::InvalidMiniProtocolId(u16)` | Structured enum variant | Fail-fast; the recipient SHOULD close the connection per spec |
| `MuxError::PayloadTooLarge { len }` | Structured enum variant | Fail-fast; the protocol limits to 65535-byte payloads |
| Transport I/O failure (`mux::transport`) | `std::io::Error` propagated | Caller responsibility; RED layer handles |

No failure mode in `mux::frame` can affect replay equivalence because frame encode/decode is a pure transformation — same input → same output.

---

## 14. Hard Prohibitions

### Inherited cluster-level prohibitions

This slice inherits and MUST comply with the cluster's "Forbidden During This Cluster" list:
- Any async machinery in any BLUE module (DC-CORE-01)
- Plugin-style runtime registration
- `pallas-network` in production builds
- `TxId` redefinition in `ade_network`
- Hidden version state in RED affecting BLUE transitions
- Codec-as-opaque-bytes framing (decision §7 #3)
- Generic `Agency<P>` wrapper unifying per-protocol agency
- Deferred validation / TODO logic in authoritative paths
- Live interop dependency on operator-provided peer only

### Slice-specific prohibitions

- `HashMap` / `HashSet` / floating point / wall-clock in `mux::frame` (BLUE)
- `String` errors in `mux::frame` — `MuxError` is a structured enum, not formatted text
- Any `dyn`-dispatch over mini-protocol IDs in `mux::frame` (closed enum or const u16 only)
- Implementing any specific mini-protocol's message decode/encode in this slice (those are S-A2)
- Implementing any mini-protocol state machine in this slice (those are S-A3..S-A8)
- Adding logging or tracing macros to `mux::frame` (BLUE — no I/O at all, even indirectly via tracing)

---

## 15. Explicit Non-Goals

S-A1 MUST NOT:

- Implement any mini-protocol message taxonomy (deferred to S-A2)
- Implement any mini-protocol state machine (deferred to S-A3..S-A8)
- Capture any cardano-node-frame corpus (deferred to S-A9)
- Establish or test any live connection (deferred to S-A10)
- Add flow-control or windowing logic (deferred to S-A9 session composition)
- Introduce protocol version handling (deferred to S-A3 handshake)
- Optimize for performance — the substrate uses straightforward `Vec<u8>` and `&[u8]` shapes
- Add feature flags or `cfg` switches in BLUE modules (T-BUILD-01 inherited)
- Modify existing CI scripts beyond extending BLUE-scope path lists

---

## 16. Completion Checklist

- [ ] All new types in `mux::frame` round-trip byte-identically (replay equivalence over BLUE)
- [ ] `mux::frame` has no async, no I/O, no `std::time`, no `HashMap`
- [ ] All failure modes in `mux::frame` are deterministic enum variants
- [ ] No TODOs or placeholders in `mux::frame` (RED `transport.rs` may have a documented stub)
- [ ] `ci_check_no_async_in_blue.sh` enforces DC-CORE-01 in CI and produces a deterministic FAIL on synthetic violation
- [ ] All 16 existing CI scripts PASS at extended BLUE scope
- [ ] Cluster doc TCB row updated to reflect the `mux::frame` / `mux::transport` split
- [ ] No registry edits in this slice (DC-CORE-01 is already declared at commit `492de56`)

---

## 17. Review Notes

**Invariant risk considered**:
- The `mux::frame` BLUE designation requires that the spec for the Ouroboros mux frame header be implementable without any nondeterminism. The 8-byte header is fixed-shape; encode and decode are total functions on bounded inputs. The only judgment is whether the 4-byte timestamp field is part of the BLUE encoder's input (caller-supplied) or generated internally. **It is caller-supplied** — the encoder takes a `MuxHeader { timestamp: u32, ... }` as input. The timestamp's *source* is the RED transport layer's responsibility; the encoder doesn't read a clock.
- `ci_check_no_async_in_blue.sh` initial grep approach has a known false-positive risk for the word "Future" appearing in comments or string literals. The initial scanner uses `grep -v '^\\s*//'` to skip comment lines but won't catch inline string mentions. If false positives accumulate, S-A1's commit notes this and a follow-up hygiene slice graduates to a syn-based scanner.

**Assumptions challenged**:
- Initially the cluster doc treated `ade_network::mux` as fully RED. S-A1 refines this: frame encode/decode is BLUE, transport (sockets) is RED. The cluster doc gets the corresponding TCB row update in this slice's diff.
- The transport module currently has no Tokio-based test (it's a stub). Real transport tests happen at S-A9 when sessions are composed. This is acceptable because S-A1 doesn't claim to close any CE — the transport scaffold is a placeholder for future composition.

**Follow-up slices implied**:
- S-A2 populates `ade_network::codec` for all 11 protocols
- S-A9 composes `mux::transport` + `mux::frame` + codec + state machines into a working session
- A hygiene slice (no slice ID yet) may graduate `ci_check_no_async_in_blue.sh` from grep to syn-based scanner

---

## 18. Authority Reminder

Authority for invariants belongs to `docs/ade-invariant-registry.toml`. Authority for mechanical acceptance belongs to the named tests/CI checks in §12. This slice doc is a planning aid.
