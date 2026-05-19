# Slice S-A4 — Chain-sync transition authority

> **Status**: Merged
> **Cluster**: [PHASE4-N-A](cluster.md)

## 2. Slice Header

**Slice Name**: ChainSync state machine + fork-choice signal emission (BLUE)

**Cluster Exit Criteria Addressed**:
- **CE-N-A-2**: `cargo test -p ade_network --test chain_sync_signal_trace` PASS — chain-sync state machine produces same fork-choice signal sequence as cardano-node oracle for curated divergence corpus.

CE-N-A-2 closes in **two stages**: this slice ships the state machine + synthetic signal-trace tests; real-capture verification against `corpus/network/n2n/chain_sync/` lands in S-A9. Tests added here are reused unchanged at S-A9; only input fixtures change source.

Out of scope: CE-N-A-1, CE-N-A-3, CE-N-A-4, CE-N-A-5.

**Slice Dependencies**: S-A1, S-A2 (ChainSyncMessage + Point + Tip), S-A3 (DC-PROTO-06 pattern established).

## 4. Intent

`DC-PROTO-01` (mini-protocol state machines have deterministic transitions) and `DC-PROTO-02` (transcript-equivalent mini-protocol behavior with Haskell node) gain their first per-protocol enforcement for chain-sync via a pure-transition state machine producing fork-choice signals byte-identical to cardano-node oracle output for the curated synthetic divergence corpus.

`DC-PROTO-06` (no ambient session state in BLUE transitions) gains chain-sync coverage.

## 5. Scope

**Modules** (under `crates/ade_network/src/chain_sync/`):
- `mod.rs` — barrel
- `state.rs` — `ChainSyncState` (Idle, CanAwait, MustReply, Intersect, Done); `ChainSyncOutput`; `ChainSyncError`
- `agency.rs` — `ChainSyncAgency` (non-interchangeable per §7 #7)
- `signal.rs` — `ForkChoiceSignal` (the consensus-interface taxonomy: RollForward / RollBackward / Intersected / NoIntersection)
- `transition.rs` — `chain_sync_transition` pure function
- `crates/ade_network/tests/chain_sync_signal_trace.rs` — integration test

**Persistence / network-visible**: none.

**Out of scope**: other mini-protocols; session composition; real-capture corpus; live interop.

## 6. Execution Boundary

| Module | Color |
|---|---|
| `ade_network::chain_sync::*` | **BLUE** |
| `tests/chain_sync_signal_trace.rs` | **BLUE** |

## 7. Invariants Preserved

`T-DET-01`, `T-CORE-01..03`, `T-INGRESS-01`, `T-CI-01`, `T-BUILD-01`, `T-KEY-01`, `T-BOUND-02`, `T-ERR-01`, `T-ENC-03`, `DC-CORE-01`, `CN-WIRE-07`, `DC-PROTO-03/04/05`.

All 16+1 CI scripts continue to PASS.

## 8. Invariants Strengthened

Exactly one invariant family:

- **`DC-PROTO-01`** — protocol state machines have deterministic transitions.

Corollaries gaining first per-protocol enforcement: `DC-PROTO-02` (chain-sync portion), `DC-PROTO-06` (chain-sync portion).

Registry strengthenings on this commit:
- `DC-PROTO-01`, `DC-PROTO-02`, `DC-PROTO-06`: `code_locus` += `crates/ade_network/src/chain_sync/*.rs`; `tests` += new tests; `strengthened_in` appends `PHASE4-N-A`. No status flips.

## 9. Design Summary

```rust
pub fn chain_sync_transition(
    state: ChainSyncState,
    agency: ChainSyncAgency,
    version: ChainSyncVersion,
    msg: ChainSyncMessage,
) -> Result<(ChainSyncState, ChainSyncOutput), ChainSyncError>;
```

**Transitions** per Ouroboros spec:
- Idle [Client] + RequestNext → CanAwait (forward request immediately) | MustReply
- CanAwait [Server] + RollForward(header, tip) → Idle + Signal(RollForward{header, tip})
- CanAwait [Server] + RollBackward(point, tip) → Idle + Signal(RollBackward{point, tip})
- CanAwait [Server] + AwaitReply → MustReply
- MustReply [Server] + RollForward / RollBackward → Idle + Signal
- Idle [Client] + FindIntersect(points) → Intersect
- Intersect [Server] + IntersectFound(point, tip) → Idle + Signal(Intersected(point, tip))
- Intersect [Server] + IntersectNotFound(tip) → Idle + Signal(NoIntersection(tip))
- Idle [Client] + ClientDone → Done
- Other → `ChainSyncError::IllegalTransition`

**Fork-choice signals** (values, not effects):
- `RollForward { header_bytes: Vec<u8>, tip: Tip }` — N-B reads header
- `RollBackward { point: Point, tip: Tip }`
- `Intersected(Point, Tip)`
- `NoIntersection(Tip)`

N-A produces signals; N-B interprets them. Boundary matches cluster doc.

**Version threading**: `chain_sync_transition` takes `ChainSyncVersion` as input. Version-gated message variants rejected with `ChainSyncError::InvalidForVersion`.

**No header decoding here**: `header_bytes` is opaque. Decoding lives in N-B (consensus runtime).

## 10. Changes Introduced

### Types (new)
- `ChainSyncState` (5 variants)
- `ChainSyncAgency` (3 variants, non-interchangeable)
- `ChainSyncOutput` (Reply, Signal, Done — 3 variants)
- `ForkChoiceSignal` (4 variants)
- `ChainSyncError` (IllegalTransition, InvalidForVersion, MalformedMessage)

### State Transitions
- Chain-sync state machine (Idle/CanAwait/MustReply/Intersect/Done)

## 11. Replay, Crash, and Epoch Validation

**Unit tests** (`chain_sync::transition::tests`):
- `idle_request_next_with_immediate_data_yields_can_await_then_roll_forward`
- `idle_request_next_with_no_data_yields_must_reply_via_await`
- `roll_forward_signal_carries_header_and_tip_byte_identical`
- `roll_backward_signal_carries_point_and_tip_byte_identical`
- `find_intersect_with_known_point_yields_intersected_signal`
- `find_intersect_with_unknown_points_yields_no_intersection`
- `client_done_terminates_session`
- `illegal_message_in_idle_returns_error`
- `wrong_agency_returns_error`
- `version_gating_rejects_out_of_version_message`

**Integration test** (`tests/chain_sync_signal_trace.rs`):
- `signal_trace_synthetic_divergence_corpus` — ≥10 curated scenarios (linear rollforward, single rollback, multi-rollback, deep rollback, no-intersection, server-stall pattern). Each asserts:
  - Signal sequence is deterministic across 1000 runs
  - Signal sequence matches the spec-derived expected sequence

**Replay equivalence MAC**: 1000-run determinism check at integration level.

**Crash/restart**: n/a (stateless per call).
**Epoch boundary**: n/a.

## 12. Mechanical Acceptance Criteria

- [ ] `cargo build --workspace --all-targets` — clean
- [ ] `cargo test -p ade_network --lib chain_sync::` — 10 unit tests PASS
- [ ] `cargo test -p ade_network --test chain_sync_signal_trace` — integration PASS
- [ ] `cargo clippy -p ade_network --all-targets -- -D warnings` — clean
- [ ] All 8 named CI scripts PASS (no_async_in_blue, module_headers, no_semantic_cfg, no_signing_in_blue, hash_uses_wire_bytes, ingress_chokepoints, dependency_boundary, constitution_coverage)
- [ ] Registry DC-PROTO-01, DC-PROTO-02, DC-PROTO-06 strengthened: `code_locus` += chain_sync paths, `tests` += new tests, `strengthened_in` += `PHASE4-N-A`. No status flips.

## 13. Failure Modes

| Variant | Cause | Recovery |
|---|---|---|
| `ChainSyncError::IllegalTransition { state, message_tag, agency }` | Wrong (state, msg, agency) triple | Fail-fast |
| `ChainSyncError::InvalidForVersion { version, message_tag }` | Out-of-version variant | Fail-fast |
| `ChainSyncError::MalformedMessage { reason }` | Grammar violation (e.g., empty intersect points) | Fail-fast |

All fail-fast. Replay-safe.

## 14. Hard Prohibitions

Inherited cluster + slice-specific:
- All cluster prohibitions
- Mutating any chain state (state machine emits signals; mutation is N-B)
- Cross-protocol agency type reuse
- `String` errors
- `HashMap` (use `BTreeMap`)
- Global / static reads
- `#[non_exhaustive]` on any chain_sync enum
- `dyn` dispatch
- `unwrap`/`expect`/`panic` outside `#[cfg(test)]`
- Decoding `header_bytes` content (N-B's job)

## 15. Explicit Non-Goals

- Other mini-protocols
- Fork-choice interpretation (N-B)
- Block validation (N-B)
- Persistence (N-D)
- Session composition (S-A9)
- Real-capture corpus (S-A9)
- Live cardano-node interop (S-A10)
- Performance optimization

## 16. Completion Checklist

- [ ] State machine pure (no ambient state, no globals, no I/O)
- [ ] Per-protocol `ChainSyncAgency`
- [ ] Closed enums (no `#[non_exhaustive]`)
- [ ] Structured `ChainSyncError`
- [ ] Version threaded as explicit input
- [ ] 10 unit + 1 integration test PASS
- [ ] Synthetic divergence corpus ≥10 scenarios
- [ ] 1000-run determinism check
- [ ] All 16+1 CI scripts PASS
- [ ] Registry DC-PROTO-01/02/06 strengthened (no status flips)
- [ ] No TODOs / placeholders / `unimplemented!()`

## 17. Review Notes

**Invariant risk**:
- **CE-N-A-2 two-stage closure** (same as S-A3). Synthetic vectors now; real-capture at S-A9.
- **`header_bytes` opaque**: chain-sync state machine does not decode block headers. Block-semantic interpretation is N-B's responsibility.

**Assumptions challenged**:
- Considered embedding header decode in chain-sync. Rejected — keeps chain-sync narrowly responsible for protocol bytes.

**Follow-up implied**: S-A9 adds real-capture corpus; same tests re-run.

## 18. Authority Reminder

Authority for invariants in `docs/ade-invariant-registry.toml`; mechanical acceptance in §12.
