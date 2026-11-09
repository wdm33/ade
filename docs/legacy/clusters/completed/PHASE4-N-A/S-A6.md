# Slice S-A6 — Tx-submission2 transition authority

> **Status**: Merged
> **Cluster**: [PHASE4-N-A](cluster.md)

## 2. Slice Header

**Slice Name**: TxSubmission2 state machine + inventory event taxonomy (BLUE)

**Cluster Exit Criteria Addressed**:
- **CE-N-A-4**: `cargo test -p ade_network --test tx_submission2_mempool_trace` PASS — tx-submission2 round-trips curated mempool trace with byte-identical wire frames.

CE-N-A-4 closes in **two stages**: this slice ships the state machine + synthetic mempool-trace tests; real-capture verification against `corpus/network/n2n/tx_submission2/` lands in S-A9. Tests added here are reused unchanged at S-A9; only input fixtures change source.

Out of scope: CE-N-A-1, CE-N-A-2, CE-N-A-3, CE-N-A-5.

**Slice Dependencies**: S-A1, S-A2 (TxSubmission2Message + TxId + TxIdAndSize), S-A3/S-A4/S-A5 (DC-PROTO-06 pattern established).

## 4. Intent

`DC-PROTO-01` (mini-protocol state machines have deterministic transitions) gains its tx-submission2 per-protocol enforcement via a pure-transition state machine producing inventory events byte-identical to cardano-node oracle output for the curated synthetic mempool trace.

`DC-PROTO-02` (transcript-equivalent mini-protocol behavior with Haskell node) and `DC-PROTO-06` (no ambient session state in BLUE transitions) gain tx-submission2 coverage.

## 5. Scope

**Modules** (under `crates/ade_network/src/tx_submission/`):
- `mod.rs` — barrel
- `state.rs` — `TxSubmission2State` (Init, Idle, TxIdsBlocking{req}, TxIdsNonBlocking{req}, TxsRequested{req_count}, Done); `TxSubmission2Output`; `TxSubmission2Error`
- `agency.rs` — `TxSubmission2Agency` (non-interchangeable per §7 #7)
- `event.rs` — `InventoryEvent` (ServerOpened / IdsRequested / IdsDelivered / TxsRequested / TxsDelivered)
- `transition.rs` — `tx_submission2_transition` pure function
- `crates/ade_network/tests/tx_submission2_mempool_trace.rs` — integration test

**Persistence / network-visible**: none.

**Out of scope**: other mini-protocols; session composition; real-capture corpus; live interop; mempool inventory bookkeeping (the state machine validates grammar invariants but does not maintain a tx inventory — that's the mempool's job in a future cluster).

## 6. Execution Boundary

| Module | Color |
|---|---|
| `ade_network::tx_submission::*` | **BLUE** |
| `tests/tx_submission2_mempool_trace.rs` | **BLUE** |

## 7. Invariants Preserved

`T-DET-01`, `T-CORE-01..03`, `T-INGRESS-01`, `T-CI-01`, `T-BUILD-01`, `T-KEY-01`, `T-BOUND-02`, `T-ERR-01`, `T-ENC-03`, `DC-CORE-01`, `CN-WIRE-07`, `DC-PROTO-03/04/05`.

All 16+1 CI scripts continue to PASS.

## 8. Invariants Strengthened

Exactly one invariant family:

- **`DC-PROTO-01`** — protocol state machines have deterministic transitions.

Corollaries gaining first per-protocol enforcement: `DC-PROTO-02` (tx-submission2 portion), `DC-PROTO-06` (tx-submission2 portion).

Registry strengthenings on this commit:
- `DC-PROTO-01`, `DC-PROTO-02`, `DC-PROTO-06`: `code_locus` += `crates/ade_network/src/tx_submission/*.rs`; `tests` += new tests. `strengthened_in` already contains `PHASE4-N-A` — no duplicate. No status flips.

## 9. Design Summary

```rust
pub fn tx_submission2_transition(
    state: TxSubmission2State,
    agency: TxSubmission2Agency,
    version: TxSubmission2Version,
    msg: TxSubmission2Message,
) -> Result<(TxSubmission2State, TxSubmission2Output), TxSubmission2Error>;
```

**Transitions** per Ouroboros tx-submission2 spec:
- Init [Client] + Init → Idle + Event(ServerOpened)
- Idle [Server] + RequestTxIds { blocking:true, ack, req }   → TxIdsBlocking { req } + Event(IdsRequested{blocking, ack, req})
- Idle [Server] + RequestTxIds { blocking:false, ack, req }  → TxIdsNonBlocking { req } + Event(IdsRequested{blocking, ack, req})
- TxIdsBlocking { req } [Client] + ReplyTxIds(entries)       → Idle + Event(IdsDelivered{entries}); MalformedMessage if entries.is_empty() OR entries.len() > req as usize
- TxIdsNonBlocking { req } [Client] + ReplyTxIds(entries)    → Idle + Event(IdsDelivered{entries}); MalformedMessage if entries.len() > req as usize
- Idle [Server] + RequestTxs(ids)                            → TxsRequested { req_count } + Event(TxsRequested{ids}); MalformedMessage if ids.is_empty()
- TxsRequested { req_count } [Client] + ReplyTxs(tx_bytes)   → Idle + Event(TxsDelivered{tx_bytes}); MalformedMessage if tx_bytes.len() > req_count
- Idle [Server] + Done                                       → Done + Done
- Other → `TxSubmission2Error::IllegalTransition`

**Inventory events** (values, not effects):
- `ServerOpened` — Init handshake completed; server may now request IDs
- `IdsRequested { blocking, ack, req }` — server asked for inventory; mempool needs to assemble reply
- `IdsDelivered { entries: Vec<TxIdAndSize> }` — client advertised inventory; mempool tracks
- `TxsRequested { ids: Vec<TxId> }` — server requested specific txs; mempool serializes
- `TxsDelivered { tx_bytes: Vec<Vec<u8>> }` — client delivered tx bytes; mempool validates and ingests

N-A produces events; mempool (future cluster) interprets them. Boundary matches cluster doc.

**Inventory grammar invariants enforced**:
- Blocking ReplyTxIds must be non-empty (per Ouroboros spec — blocking call promises ≥1 ID)
- ReplyTxIds count ≤ req advertised
- RequestTxs must be non-empty (server cannot request zero txs)
- ReplyTxs count ≤ req_count (some requested txs may be unavailable; subset reply is legal)

**Version threading**: `tx_submission2_transition` takes `TxSubmission2Version` as input. Out-of-version variants rejected with `TxSubmission2Error::InvalidForVersion`.

**No tx-body validation here**: `tx_bytes: Vec<u8>` is opaque. Validation and decoding live in the mempool / ledger (future cluster).

**No inventory bookkeeping**: state carries `req` count to validate ReplyTxIds size; the actual tx ID set lives in mempool. State machine memory bounded.

## 10. Changes Introduced

### Types (new)
- `TxSubmission2State` (6 variants)
- `TxSubmission2Agency` (3 variants, non-interchangeable)
- `TxSubmission2Output` (Event, Done — 2 variants)
- `InventoryEvent` (5 variants)
- `TxSubmission2Error` (IllegalTransition, InvalidForVersion, MalformedMessage)

### State Transitions
- Tx-submission2 state machine (Init/Idle/TxIdsBlocking/TxIdsNonBlocking/TxsRequested/Done)

## 11. Replay, Crash, and Epoch Validation

**Unit tests** (`tx_submission::transition::tests`):
- `init_handshake_opens_session`
- `request_blocking_ids_then_reply_round_trips`
- `request_non_blocking_ids_with_empty_reply_round_trips`
- `request_txs_then_reply_delivers_bytes_byte_identical`
- `request_txs_with_partial_reply_legal`
- `blocking_reply_empty_is_malformed`
- `reply_ids_overfill_is_malformed`
- `request_txs_empty_is_malformed`
- `reply_txs_overfill_is_malformed`
- `done_terminates_session`
- `illegal_message_in_init_returns_error`
- `wrong_agency_returns_error`
- `version_gating_rejects_out_of_version_message`

**Integration test** (`tests/tx_submission2_mempool_trace.rs`):
- `tx_submission2_mempool_trace` — ≥10 curated scenarios (single-round inventory ad, multi-round inventory ad, single-tx fetch, multi-tx fetch, partial-fetch with unavailable txs, blocking call with full reply, blocking call followed by non-blocking, immediate done, interleaved request/reply patterns, varying tx body sizes). Each asserts:
  - Inventory event sequence is deterministic across 1000 runs
  - Event sequence matches the spec-derived expected sequence
  - tx_bytes delivered byte-identically across replays

**Replay equivalence MAC**: 1000-run determinism check at integration level.

**Crash/restart**: n/a (stateless per call; state value owned by caller).
**Epoch boundary**: n/a.

## 12. Mechanical Acceptance Criteria

- [ ] `cargo build -p ade_network --all-targets` — clean
- [ ] `cargo test -p ade_network --lib tx_submission::` — 13 unit tests PASS
- [ ] `cargo test -p ade_network --test tx_submission2_mempool_trace` — integration PASS
- [ ] `cargo clippy -p ade_network --all-targets -- -D warnings` — clean
- [ ] All 8 named CI scripts PASS
- [ ] Registry DC-PROTO-01, DC-PROTO-02, DC-PROTO-06 strengthened: `code_locus` += tx_submission paths, `tests` += new tests. No status flips.

## 13. Failure Modes

| Variant | Cause | Recovery |
|---|---|---|
| `TxSubmission2Error::IllegalTransition { state, message_tag, agency }` | Wrong (state, msg, agency) triple | Fail-fast |
| `TxSubmission2Error::InvalidForVersion { version, message_tag }` | Out-of-version variant | Fail-fast |
| `TxSubmission2Error::MalformedMessage { reason }` | Grammar violation (empty blocking reply, count > req, empty RequestTxs) | Fail-fast |

All fail-fast. Replay-safe.

## 14. Hard Prohibitions

Inherited cluster + slice-specific:
- All cluster prohibitions
- Mutating any mempool state (state machine emits events; mempool owns inventory)
- Cross-protocol agency type reuse
- `String` errors
- `HashMap` (use `BTreeMap`)
- Global / static reads
- `#[non_exhaustive]` on any tx_submission enum
- `dyn` dispatch
- `unwrap`/`expect`/`panic` outside `#[cfg(test)]`
- Decoding `tx_bytes` content (mempool/ledger job)
- Maintaining a tx ID set in state — only the request count

## 15. Explicit Non-Goals

- Other mini-protocols
- Tx-body validation (mempool/ledger)
- Mempool inventory bookkeeping
- Persistence (N-D)
- Session composition (S-A9)
- Real-capture corpus (S-A9)
- Live cardano-node interop (S-A10)
- Performance optimization
- Tx ID ordering verification on ReplyTxs (mempool's responsibility)

## 16. Completion Checklist

- [ ] State machine pure (no ambient state, no globals, no I/O)
- [ ] Per-protocol `TxSubmission2Agency`
- [ ] Closed enums (no `#[non_exhaustive]`)
- [ ] Structured `TxSubmission2Error`
- [ ] Version threaded as explicit input
- [ ] 13 unit + 1 integration test PASS
- [ ] Synthetic mempool corpus ≥10 scenarios
- [ ] 1000-run determinism check
- [ ] All 16+1 CI scripts PASS
- [ ] Registry DC-PROTO-01/02/06 strengthened (no status flips)
- [ ] No TODOs / placeholders / `unimplemented!()`

## 17. Review Notes

**Invariant risk**:
- **CE-N-A-4 two-stage closure** (same as S-A3/S-A4/S-A5). Synthetic vectors now; real-capture at S-A9.
- **`tx_bytes` opaque**: state machine does not decode tx bodies; validation is mempool/ledger responsibility.
- **No tx ID set in state**: only `req` count carried. Mempool tracks the actual ID set; state machine validates count only.
- **Inverted client-server semantics**: in tx-submission2 the Client (initiator) RESPONDS to Server (responder) requests — opposite of chain-sync. The agency labels in the state machine match the Ouroboros spec, not intuition about "who's in charge."

**Assumptions challenged**:
- Considered tracking the requested `ids: Vec<TxId>` in `TxsRequested` state to verify ReplyTxs matches by ID. Rejected — that's mempool's inventory job; the state machine validates count only, keeping state-machine memory bounded and the responsibility line clean.
- Considered single `TxIdsAwaiting { blocking: bool, req: u16 }` state instead of two TxIds states. Rejected — the blocking/non-blocking distinction has different grammar (blocking requires non-empty reply); typing the state separately keeps the transition table explicit.

**Follow-up implied**: S-A9 adds real-capture corpus; same tests re-run.

## 18. Authority Reminder

Authority for invariants in `docs/ade-invariant-registry.toml`; mechanical acceptance in §12.
