# Slice S-A5 — Block-fetch transition authority

> **Status**: Merged
> **Cluster**: [PHASE4-N-A](cluster.md)

## 2. Slice Header

**Slice Name**: BlockFetch state machine + batch delivery (BLUE)

**Cluster Exit Criteria Addressed**:
- **CE-N-A-3**: `cargo test -p ade_network --test block_fetch_frame_corpus` PASS — block-fetch delivers blocks byte-identically and in same order as cardano-node oracle for curated batch corpus.

CE-N-A-3 closes in **two stages**: this slice ships the state machine + synthetic batch-trace tests; real-capture verification against `corpus/network/n2n/block_fetch/` lands in S-A9. Tests added here are reused unchanged at S-A9; only input fixtures change source.

Out of scope: CE-N-A-1, CE-N-A-2, CE-N-A-4, CE-N-A-5.

**Slice Dependencies**: S-A1, S-A2 (BlockFetchMessage + Point + Range), S-A3/S-A4 (DC-PROTO-06 pattern established).

## 4. Intent

`DC-PROTO-01` (mini-protocol state machines have deterministic transitions) gains its block-fetch per-protocol enforcement via a pure-transition state machine producing batch-delivery events byte-identical to cardano-node oracle output for the curated synthetic batch corpus.

`DC-PROTO-02` (transcript-equivalent mini-protocol behavior with Haskell node) and `DC-PROTO-06` (no ambient session state in BLUE transitions) gain block-fetch coverage.

## 5. Scope

**Modules** (under `crates/ade_network/src/block_fetch/`):
- `mod.rs` — barrel
- `state.rs` — `BlockFetchState` (Idle, Busy, Streaming, Done); `BlockFetchOutput`; `BlockFetchError`
- `agency.rs` — `BlockFetchAgency` (non-interchangeable per §7 #7)
- `event.rs` — `BatchDeliveryEvent` (the consensus-interface taxonomy: BatchStarted / BlockDelivered / NoBlocks / BatchCompleted)
- `transition.rs` — `block_fetch_transition` pure function
- `crates/ade_network/tests/block_fetch_frame_corpus.rs` — integration test

**Persistence / network-visible**: none.

**Out of scope**: other mini-protocols; session composition; real-capture corpus; live interop; block-body decoding.

## 6. Execution Boundary

| Module | Color |
|---|---|
| `ade_network::block_fetch::*` | **BLUE** |
| `tests/block_fetch_frame_corpus.rs` | **BLUE** |

## 7. Invariants Preserved

`T-DET-01`, `T-CORE-01..03`, `T-INGRESS-01`, `T-CI-01`, `T-BUILD-01`, `T-KEY-01`, `T-BOUND-02`, `T-ERR-01`, `T-ENC-03`, `DC-CORE-01`, `CN-WIRE-07`, `DC-PROTO-03/04/05`.

All 16+1 CI scripts continue to PASS.

## 8. Invariants Strengthened

Exactly one invariant family:

- **`DC-PROTO-01`** — protocol state machines have deterministic transitions.

Corollaries gaining first per-protocol enforcement: `DC-PROTO-02` (block-fetch portion), `DC-PROTO-06` (block-fetch portion).

Registry strengthenings on this commit:
- `DC-PROTO-01`, `DC-PROTO-02`, `DC-PROTO-06`: `code_locus` += `crates/ade_network/src/block_fetch/*.rs`; `tests` += new tests; `strengthened_in` already contains `PHASE4-N-A` (from S-A4) — no further append needed. No status flips.

## 9. Design Summary

```rust
pub fn block_fetch_transition(
    state: BlockFetchState,
    agency: BlockFetchAgency,
    version: BlockFetchVersion,
    msg: BlockFetchMessage,
) -> Result<(BlockFetchState, BlockFetchOutput), BlockFetchError>;
```

**Transitions** per Ouroboros spec:
- Idle [Client] + RequestRange(range) → Busy + Reply(RequestRange)
- Busy [Server] + StartBatch → Streaming + Event(BatchStarted)
- Busy [Server] + NoBlocks → Idle + Event(NoBlocks)
- Streaming [Server] + Block(bytes) → Streaming + Event(BlockDelivered { bytes })
- Streaming [Server] + BatchDone → Idle + Event(BatchCompleted)
- Idle [Client] + ClientDone → Done + Done
- Other → `BlockFetchError::IllegalTransition`

**Batch-delivery events** (values, not effects):
- `BatchStarted` — server accepted range; stream begins
- `BlockDelivered { block_bytes: Vec<u8> }` — N-B reads body
- `NoBlocks` — server has no blocks in requested range
- `BatchCompleted` — stream terminates; back to Idle

N-A produces events; N-B interprets them. Boundary matches cluster doc.

**Version threading**: `block_fetch_transition` takes `BlockFetchVersion` as input. Out-of-version variants rejected with `BlockFetchError::InvalidForVersion`.

**No block decoding here**: `block_bytes` is opaque. Decoding lives in N-B (consensus runtime) via `ade_codec` era decoders.

**Range validation**: empty/inverted ranges (`from` > `to` by slot) are rejected as `MalformedMessage` at the state machine. Origin-to-Origin is legal (empty fetch, server returns NoBlocks).

## 10. Changes Introduced

### Types (new)
- `BlockFetchState` (4 variants)
- `BlockFetchAgency` (3 variants, non-interchangeable)
- `BlockFetchOutput` (Reply, Event, Done — 3 variants)
- `BatchDeliveryEvent` (4 variants)
- `BlockFetchError` (IllegalTransition, InvalidForVersion, MalformedMessage)

### State Transitions
- Block-fetch state machine (Idle/Busy/Streaming/Done)

## 11. Replay, Crash, and Epoch Validation

**Unit tests** (`block_fetch::transition::tests`):
- `idle_request_range_yields_busy_then_start_batch`
- `busy_no_blocks_returns_to_idle_with_event`
- `streaming_block_delivers_bytes_byte_identical`
- `streaming_batch_done_returns_to_idle`
- `multi_block_streaming_preserves_order`
- `client_done_terminates_session`
- `illegal_message_in_idle_returns_error`
- `wrong_agency_returns_error`
- `version_gating_rejects_out_of_version_message`
- `inverted_range_returns_malformed`
- `block_delivered_event_carries_bytes_verbatim`

**Integration test** (`tests/block_fetch_frame_corpus.rs`):
- `block_fetch_frame_corpus` — ≥10 curated scenarios (single-block batch, 10-block batch, 100-block batch, empty range → NoBlocks, Origin-to-Origin → NoBlocks, varying block sizes, mid-stream done check). Each asserts:
  - Batch-delivery event sequence is deterministic across 1000 runs
  - Event sequence matches the spec-derived expected sequence
  - Block bytes delivered byte-identically across replays

**Replay equivalence MAC**: 1000-run determinism check at integration level.

**Crash/restart**: n/a (stateless per call).
**Epoch boundary**: n/a.

## 12. Mechanical Acceptance Criteria

- [ ] `cargo build --workspace --all-targets` — clean
- [ ] `cargo test -p ade_network --lib block_fetch::` — 11 unit tests PASS
- [ ] `cargo test -p ade_network --test block_fetch_frame_corpus` — integration PASS
- [ ] `cargo clippy -p ade_network --all-targets -- -D warnings` — clean
- [ ] All 8 named CI scripts PASS (no_async_in_blue, module_headers, no_semantic_cfg, no_signing_in_blue, hash_uses_wire_bytes, ingress_chokepoints, dependency_boundary, constitution_coverage)
- [ ] Registry DC-PROTO-01, DC-PROTO-02, DC-PROTO-06 strengthened: `code_locus` += block_fetch paths, `tests` += new tests. No status flips.

## 13. Failure Modes

| Variant | Cause | Recovery |
|---|---|---|
| `BlockFetchError::IllegalTransition { state, message_tag, agency }` | Wrong (state, msg, agency) triple | Fail-fast |
| `BlockFetchError::InvalidForVersion { version, message_tag }` | Out-of-version variant | Fail-fast |
| `BlockFetchError::MalformedMessage { reason }` | Grammar violation (e.g., inverted range) | Fail-fast |

All fail-fast. Replay-safe.

## 14. Hard Prohibitions

Inherited cluster + slice-specific:
- All cluster prohibitions
- Mutating any chain state (state machine emits events; mutation is N-B)
- Cross-protocol agency type reuse
- `String` errors
- `HashMap` (use `BTreeMap`)
- Global / static reads
- `#[non_exhaustive]` on any block_fetch enum
- `dyn` dispatch
- `unwrap`/`expect`/`panic` outside `#[cfg(test)]`
- Decoding `block_bytes` content (N-B's job)
- Buffering all blocks in state — emit events per block as they arrive

## 15. Explicit Non-Goals

- Other mini-protocols
- Block-body decoding (N-B)
- Block validation (N-B)
- Persistence (N-D)
- Session composition (S-A9)
- Real-capture corpus (S-A9)
- Live cardano-node interop (S-A10)
- Performance optimization
- Concurrent batch streaming (single-batch state machine only)

## 16. Completion Checklist

- [ ] State machine pure (no ambient state, no globals, no I/O)
- [ ] Per-protocol `BlockFetchAgency`
- [ ] Closed enums (no `#[non_exhaustive]`)
- [ ] Structured `BlockFetchError`
- [ ] Version threaded as explicit input
- [ ] 11 unit + 1 integration test PASS
- [ ] Synthetic batch corpus ≥10 scenarios
- [ ] 1000-run determinism check
- [ ] All 16+1 CI scripts PASS
- [ ] Registry DC-PROTO-01/02/06 strengthened (no status flips)
- [ ] No TODOs / placeholders / `unimplemented!()`

## 17. Review Notes

**Invariant risk**:
- **CE-N-A-3 two-stage closure** (same as S-A3/S-A4). Synthetic vectors now; real-capture at S-A9.
- **`block_bytes` opaque**: block-fetch state machine does not decode block bodies. Block-semantic interpretation is N-B's responsibility via `ade_codec` era decoders.
- **Streaming semantics**: state machine emits `BlockDelivered` per `Block` message — does not accumulate or batch internally. N-B is responsible for collecting/processing the stream.

**Assumptions challenged**:
- Considered accumulating blocks in `Streaming` state. Rejected — keeps state machine memory-bounded and shifts batching to N-B where the policy belongs.

**Follow-up implied**: S-A9 adds real-capture corpus; same tests re-run.

## 18. Authority Reminder

Authority for invariants in `docs/ade-invariant-registry.toml`; mechanical acceptance in §12.
