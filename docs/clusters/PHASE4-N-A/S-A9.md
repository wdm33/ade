# Slice S-A9 — Session composition + frame corpus + replay harness

> **Status**: Merged (entry-blocker resolved by S-A9 real-capture work + S-A10 CE-N-A-5 evidence script)
> **Cluster**: [PHASE4-N-A](cluster.md)

## 2. Slice Header

**Slice Name**: RED session composition + GREEN frame corpus harness + GREEN replay machinery (partial-CE-N-A-5 reproducibility)

**Cluster Exit Criteria Addressed**:
- **CE-N-A-1, CE-N-A-2, CE-N-A-3, CE-N-A-4 — real-capture portions**: each of S-A3/S-A4/S-A5/S-A6 closed the state-machine portion against synthetic vectors. This slice supplies the captured-frame fixtures and re-runs the same tests against them. The four integration tests (`handshake_version_negotiation`, `chain_sync_signal_trace`, `block_fetch_frame_corpus`, `tx_submission2_mempool_trace`) gain corpus-driven scenarios appended to their existing synthetic ones. Test names unchanged.
- **CE-N-A-5 (partial)**: reproducibility infrastructure. The full CE-N-A-5 closure (live transcript equivalence against pinned Docker cardano-node 10.6.2) lands in S-A10.

Out of scope: live cardano-node interop (S-A10).

**Slice Dependencies**: S-A1..S-A8 (all state machines + codecs landed).

**External infrastructure dependency**: this slice cannot begin implementation until **`corpus/network/{n2n,n2c}/<protocol>/` is populated with real captured frames** from cardano-node 10.6.2. Capture mechanism: tcpdump or equivalent against a running cardano-node 10.6.2 N2N/N2C handshake + chain-sync + block-fetch + tx-submission2 + N2C client session. Each captured frame must be paired with per-frame metadata (cardano-node version, network magic, protocol, mini-protocol version, direction, agency, raw bytes, expected decode result, expected re-encode bytes).

Capture-collection is itself a multi-hour operator task. The slice doc is a forward obligation — no implementation work begins until the corpus directory contains fixtures.

## 4. Intent

`DC-PROTO-02` (transcript-equivalent mini-protocol behavior with Haskell node) closes for the four authoritative N2N protocols against captured frames. The existing synthetic-corpus tests added by S-A3..S-A6 are re-run unchanged with real-capture fixtures swapped in. Byte-identical re-encode of every captured frame is the closure criterion.

`DC-PROTO-04` (full N2C surface) flips status from "declared" to "enforced" once N2C protocols are exercised against captured frames (handshake, local-chain-sync, local-tx-submission, local-state-query, local-tx-monitor).

`CN-WIRE-07` (one closed versioned message type per protocol-visible message) flips status from "declared" to "enforced" once every captured frame round-trips byte-identically.

`T-ENC-03` (valid encodings round-trip byte-identically): same flip.

## 5. Scope

**Modules** (new under `crates/ade_network/src/session/` and `crates/ade_testkit/src/network/`):
- `crates/ade_network/src/session/mod.rs` — barrel
- `crates/ade_network/src/session/composer.rs` — RED composition glue: socket ↔ mux ↔ codec ↔ state machine. Holds selected protocol versions; threads them as explicit inputs to BLUE transitions per DC-PROTO-06.
- `crates/ade_network/src/session/connection.rs` — RED N2N/N2C connection lifecycle (open, handshake, multiplex, close)
- `crates/ade_testkit/src/network/mod.rs` — barrel
- `crates/ade_testkit/src/network/corpus.rs` — GREEN frame corpus reader (canonical schema per cluster doc §"Replay Obligations")
- `crates/ade_testkit/src/network/replay.rs` — GREEN replay harness: drives `corpus/network/...` fixtures through state machines
- `crates/ade_testkit/src/network/schema.rs` — GREEN per-frame metadata schema (TOML or JSON; one schema choice committed)

**Corpus** (new at `corpus/network/`):
- `n2n/handshake/` — captured handshake frames per cardano-node 10.6.2 supported version table
- `n2n/chain_sync/` — captured chain-sync frames including divergence scenarios
- `n2n/block_fetch/` — captured block-fetch frames including batches of varying sizes
- `n2n/tx_submission2/` — captured tx-submission2 frames including inventory rounds
- `n2n/keep_alive/`, `n2n/peer_sharing/` — captured frames
- `n2c/handshake/`, `n2c/local_chain_sync/`, `n2c/local_tx_submission/`, `n2c/local_state_query/`, `n2c/local_tx_monitor/` — N2C captures

**Tests** (existing files gain corpus-driven scenarios; NO new test names):
- `handshake_version_negotiation.rs::version_negotiation_across_supported_table` — gains real-capture scenarios
- `chain_sync_signal_trace.rs::signal_trace_synthetic_divergence_corpus` — gains real-capture scenarios
- `block_fetch_frame_corpus.rs::block_fetch_frame_corpus` — gains real-capture scenarios
- `tx_submission2_mempool_trace.rs::tx_submission2_mempool_trace` — gains real-capture scenarios

**Persistence / network-visible impact**: RED session layer opens sockets, multiplexes traffic, threads selected versions to BLUE — but does not alter the BLUE protocol semantics.

**Out of scope**: live interop (S-A10); peer-book population (future cluster); rate-limiting policy beyond what mux frame-size cap already enforces.

## 6. Execution Boundary

| Module | Color |
|---|---|
| `ade_network::session::*` | **RED** |
| `ade_testkit::network::*` | **GREEN** |
| `corpus/network/**` | **non-code data** |

## 7. Invariants Preserved

All cluster invariants. All 16+1 CI scripts continue to PASS.

## 8. Invariants Strengthened

Three invariant families flip from "declared" to "enforced":

- **`DC-PROTO-02`** — transcript equivalence with cardano-node confirmed against captured frames.
- **`DC-PROTO-04`** — full N2C surface exercised against captures (combined with S-A3 + S-A8).
- **`CN-WIRE-07`** — one closed versioned message type per protocol-visible message, byte-identical round-trip on real captures.
- **`T-ENC-03`** — valid encodings round-trip byte-identically (real-capture coverage).

## 9. Design Summary

### Session composer (RED)

```rust
pub struct Session {
    // ...
    negotiated_n2n_version: Option<N2NVersion>,
    negotiated_n2c_version: Option<N2CVersion>,
    // ...
}

impl Session {
    pub async fn run(&mut self) -> Result<(), SessionError>;
}
```

Session composer reads decoded messages from mux, threads them with the negotiated version into the BLUE state machine, sends emitted Reply messages back through mux, dispatches Events to consumers (mempool, consensus). Sync-only BLUE invariant preserved: composer is async (RED), but every call into BLUE is sync.

### Frame corpus schema (GREEN)

```toml
# corpus/network/n2n/chain_sync/<scenario>/frame_001.toml
[frame]
cardano_node_version = "10.6.2"
network_magic = 764824073
protocol = "ChainSync"
mini_protocol_version = 11
direction = "Client->Server"  # or "Server->Client"
agency = "Client"             # who sent
expected_decode_tag = "RollForward"
[bytes]
hex = "..."
```

Companion file per scenario: `expected_events.toml` listing the spec-derived event sequence the state machine must emit.

### Replay harness (GREEN)

```rust
pub struct FrameReplay { /* loads corpus, drives state machine, asserts */ }
impl FrameReplay {
    pub fn from_corpus_dir(path: &Path) -> Result<Self, ReplayError>;
    pub fn assert_round_trip_byte_identical(&self) -> Result<(), ReplayError>;
    pub fn assert_event_sequence_matches_expected(&self) -> Result<(), ReplayError>;
}
```

Replay harness is GREEN test infrastructure — non-authoritative. The state machine + codec it drives are BLUE; the corpus is data.

## 10. Changes Introduced

### Types (new)
- `Session` struct (RED)
- `FrameReplay` struct (GREEN testkit)
- per-frame schema struct (GREEN testkit)

### State Transitions
- No new BLUE transitions.

## 11. Replay, Crash, and Epoch Validation

The four existing CE-bound integration tests gain corpus-driven scenarios. Test names unchanged — only fixture sources expand.

**Crash/restart**: session composer (RED) handles socket-level failure → reconnect; BLUE state machine state is reset on reconnect.

**Epoch boundary**: not protocol-visible; chain-sync emits signals around epoch transitions but the state machine itself is epoch-agnostic.

## 12. Mechanical Acceptance Criteria

- [ ] `cargo build --workspace --all-targets` — clean
- [ ] `cargo test -p ade_network` — all integration tests PASS including corpus-driven scenarios
- [ ] `cargo clippy --workspace --all-targets -- -D warnings` — clean
- [ ] All 16+1 CI scripts PASS
- [ ] Corpus directory `corpus/network/**` populated with ≥10 captured frames per N2N protocol and ≥5 per N2C protocol
- [ ] Registry DC-PROTO-02, DC-PROTO-04, CN-WIRE-07, T-ENC-03 flip from "declared" to "enforced"

## 13. Failure Modes

`SessionError` (new, RED) — closed enum covering socket failure, handshake failure, mux desync, decode failure, state-machine illegal transition. All RED-handled (retry or terminate session); BLUE never sees them.

`ReplayError` (new, GREEN) — closed enum covering corpus-missing, schema-mismatch, byte-identicality-failure, event-sequence-mismatch.

## 14. Hard Prohibitions

Inherited cluster:
- All cluster prohibitions
- BLUE state machine remains sync; only session composer is async
- No new BLUE invariants from this slice
- No state machine changes (only composition + replay infrastructure)

## 15. Explicit Non-Goals

- Live cardano-node interop (S-A10)
- Peer-book population (future cluster)
- Mempool / ledger integration (future cluster)
- Performance optimization

## 16. Completion Checklist

- [ ] Corpus directory populated
- [ ] Session composer compiles and runs (sync-only into BLUE)
- [ ] Replay harness drives all 4 N2N + 5 N2C corpus dirs
- [ ] All 16+1 CI scripts PASS
- [ ] Registry status flips applied: DC-PROTO-02, DC-PROTO-04, CN-WIRE-07, T-ENC-03 → "enforced"
- [ ] No TODOs / placeholders / `unimplemented!()`

## 17. Review Notes

**Invariant risk**:
- **External infrastructure dependency**: this slice cannot begin until `corpus/network/**` is populated. Capture collection itself is a multi-hour operator task against a running cardano-node 10.6.2 instance.
- **RED/BLUE boundary**: session composer is async (RED); state machine remains sync (BLUE). The boundary type-enforced by `tokio::spawn` only existing under `session::*`, never reaching BLUE transitions.

**Assumptions challenged**:
- Considered making session composer GREEN (deterministic glue). Rejected — sockets, tokio, retry policy, version-table threading all carry shell concerns; the FC/IS rule places this in RED.

**Follow-up implied**: S-A10 adds the live interop binary that exercises the composer against pinned-Docker cardano-node 10.6.2.

## 18. Authority Reminder

Authority for invariants in `docs/ade-invariant-registry.toml`; mechanical acceptance in §12.
