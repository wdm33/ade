# Slice S-A8b — LocalTxMonitor wire-grammar rework

> **Status**: Merged
> **Cluster**: [PHASE4-N-A](cluster.md)
> **Supersedes (partial)**: LocalTxMonitor portions of S-A2 and S-A8

## 2. Slice Header

**Slice Name**: LocalTxMonitor closed wire grammar + state machine alignment with cardano-node 11.0.1 (BLUE)

**Cluster Exit Criteria Addressed**: none directly. Prerequisite for CE-N-A-5 (live interop) — without this, the LocalTxMonitor mini-protocol cannot interop with a live cardano-node.

**Defect classification**: design defect in S-A2/S-A8. S-A2's `LocalTxMonitorMessage` modeled the protocol as a generic `Query(opaque) / Reply(opaque)` pair at tags 5/6. The actual Ouroboros LocalTxMonitor wire grammar has **8 distinct query/reply message pairs** with separate tags. The misimpl was discovered when verifying our codec against the cardano-node 11.0.1 ouroboros-network `LocalTxMonitor/Codec.hs`. Same defect existed against cardano-node 10.6.2 — the version bump merely surfaced it.

**Slice Dependencies**: S-A1, S-A2 (will be partially replaced), S-A8 (will be partially replaced).

## 4. Intent

`DC-PROTO-04` (full N2C mini-protocol surface) gains its proper LocalTxMonitor enforcement — the closed wire grammar per Ouroboros spec rather than the simplified opaque-payload approximation shipped by S-A2.

`CN-WIRE-07` (one closed versioned message type per protocol-visible message) becomes mechanically enforced for LocalTxMonitor's 12 distinct messages instead of the previous 7-variant approximation.

`DC-PROTO-01` (deterministic transitions) — state graph is corrected to match the Ouroboros spec: `MsgAwaitAcquire` is Client-agency in Acquired state (re-acquire pattern), not Server-agency in Acquiring as S-A8 incorrectly modeled.

## 5. Scope

**Modules to rewrite** (preserve file paths; rewrite content):
- `crates/ade_network/src/codec/local_tx_monitor.rs` — REPLACE message taxonomy + encode/decode
- `crates/ade_network/src/n2c/local_tx_monitor/state.rs` — REPLACE state enum (add Busy{query_kind} variants)
- `crates/ade_network/src/n2c/local_tx_monitor/event.rs` — REPLACE event taxonomy
- `crates/ade_network/src/n2c/local_tx_monitor/transition.rs` — REPLACE transition + tests
- `crates/ade_network/tests/local_tx_monitor_event_trace.rs` — REWRITE integration test scenarios

**Modules unchanged**:
- `crates/ade_network/src/n2c/local_tx_monitor/agency.rs` — `LocalTxMonitorAgency` enum stays the same

**Persistence / network-visible**: none. Pure refinement of an existing BLUE codec + state machine.

**Out of scope**: any other protocol; mempool integration; live interop (S-A10).

## 6. Execution Boundary

| Module | Color |
|---|---|
| `ade_network::codec::local_tx_monitor` | **BLUE** |
| `ade_network::n2c::local_tx_monitor::*` | **BLUE** |
| `tests/local_tx_monitor_event_trace.rs` | **BLUE** |

## 7. Invariants Preserved

`T-DET-01`, `T-CORE-01..03`, `T-INGRESS-01`, `T-CI-01`, `T-BUILD-01`, `T-KEY-01`, `T-BOUND-02`, `T-ERR-01`, `T-ENC-03`, `DC-CORE-01`, `DC-PROTO-02/03/05/06`.

All 16+1 CI scripts continue to PASS.

## 8. Invariants Strengthened

- **`DC-PROTO-04`** — LocalTxMonitor is now closed at the full per-query-type wire grammar instead of the simplified opaque approximation.
- **`CN-WIRE-07`** — LocalTxMonitor's 12 message variants are each their own typed enum variant with distinct CBOR tags.

Registry strengthenings: `code_locus` and `tests` arrays for `DC-PROTO-04` and `CN-WIRE-07` are updated to reflect the rewritten file contents (entries stay the same paths/test names; the rewrite supersedes the previous overstated coverage).

## 9. Design Summary

### Wire grammar (verified against `ouroboros-network/protocols/lib/Ouroboros/Network/Protocol/LocalTxMonitor/Codec.hs`)

```
localTxMonitorMessage =
    [0]                                           ; MsgDone               (Idle, Client)
  / [1]                                           ; MsgAcquire             (Idle, Client)
  / [1]                                           ; MsgAwaitAcquire        (Acquired, Client) — same tag, agency-distinguished
  / [2, slot]                                     ; MsgAcquired            (Acquiring, Server)
  / [3]                                           ; MsgRelease             (Acquired, Client)
  / [5]                                           ; MsgNextTx              (Acquired, Client)
  / [6]                                           ; MsgReplyNextTx Nothing (Busy{NextTx}, Server)
  / [6, txBytes]                                  ; MsgReplyNextTx (Just tx) (Busy{NextTx}, Server)
  / [7, txid]                                     ; MsgHasTx               (Acquired, Client)
  / [8, bool]                                     ; MsgReplyHasTx          (Busy{HasTx}, Server)
  / [9]                                           ; MsgGetSizes            (Acquired, Client)
  / [10, [capBytes, sizeBytes, txCount]]          ; MsgReplyGetSizes       (Busy{GetSizes}, Server)
  / [11]                                          ; MsgGetMeasures         (Acquired, Client) [v2+]
  / [12, txCount, measureMap]                     ; MsgReplyGetMeasures    (Busy{GetMeasures}, Server) [v2+]
```

**LocalTxMonitor_V2** (Measures support) is gated on `LocalTxMonitorVersion >= 2`, which corresponds to N2C handshake version ≥ V20.

### Message taxonomy (Rust)

```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LocalTxMonitorMessage {
    Done,
    Acquire,
    AwaitAcquire,
    Acquired { slot: SlotNo },
    Release,

    NextTx,
    ReplyNextTx { tx_bytes: Option<Vec<u8>> },

    HasTx { tx_id: TxId },
    ReplyHasTx { present: bool },

    GetSizes,
    ReplyGetSizes(MempoolSizeAndCapacity),

    GetMeasures,
    ReplyGetMeasures(MempoolMeasures),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MempoolSizeAndCapacity {
    pub capacity_bytes: u32,
    pub size_bytes: u32,
    pub tx_count: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MempoolMeasures {
    pub tx_count: u32,
    pub measures: BTreeMap<MeasureName, MeasureSizeAndCapacity>,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct MeasureName(pub Vec<u8>);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MeasureSizeAndCapacity {
    pub size: u64,
    pub capacity: u64,
}
```

`measureMap` is `Map MeasureName (SizeAndCapacity Integer)` upstream — we use `BTreeMap<MeasureName, MeasureSizeAndCapacity>` for deterministic iteration (HashMap is forbidden per DC-CORE).

`MeasureName` is the on-wire `Text` upstream — we model as `Vec<u8>` and validate UTF-8 in decode (matches existing handshake pattern).

### State graph (rewritten)

```rust
pub enum LocalTxMonitorState {
    Idle,
    Acquiring,
    Acquired,
    Busy { kind: BusyKind },
    Done,
}

pub enum BusyKind {
    NextTx,
    HasTx,
    GetSizes,
    GetMeasures,
}
```

**Transitions** (per Ouroboros spec):

- (Idle, Client, Done) → (Done, Done)
- (Idle, Client, Acquire) → (Acquiring, Event(AcquireRequested))
- (Acquiring, Server, Acquired { slot }) → (Acquired, Event(MempoolAcquired { slot }))
- (Acquired, Client, AwaitAcquire) → (Acquiring, Event(ReAcquireRequested))
- (Acquired, Client, Release) → (Idle, Event(MempoolReleased))
- (Acquired, Client, NextTx) → (Busy{NextTx}, Event(NextTxRequested))
- (Acquired, Client, HasTx { tx_id }) → (Busy{HasTx}, Event(HasTxRequested { tx_id }))
- (Acquired, Client, GetSizes) → (Busy{GetSizes}, Event(SizesRequested))
- (Acquired, Client, GetMeasures) — only if version >= LocalTxMonitor V2 (N2C V20+) → (Busy{GetMeasures}, Event(MeasuresRequested))
- (Busy{NextTx}, Server, ReplyNextTx { tx_bytes }) → (Acquired, Event(NextTxReplied { tx_bytes }))
- (Busy{HasTx}, Server, ReplyHasTx { present }) → (Acquired, Event(HasTxReplied { present }))
- (Busy{GetSizes}, Server, ReplyGetSizes(sizes)) → (Acquired, Event(SizesReplied(sizes)))
- (Busy{GetMeasures}, Server, ReplyGetMeasures(measures)) → (Acquired, Event(MeasuresReplied(measures)))
- Any other triple → IllegalTransition

### Event taxonomy

```rust
pub enum LocalTxMonitorEvent {
    AcquireRequested,
    ReAcquireRequested,
    MempoolAcquired { slot: SlotNo },
    MempoolReleased,
    NextTxRequested,
    NextTxReplied { tx_bytes: Option<Vec<u8>> },
    HasTxRequested { tx_id: TxId },
    HasTxReplied { present: bool },
    SizesRequested,
    SizesReplied(MempoolSizeAndCapacity),
    MeasuresRequested,
    MeasuresReplied(MempoolMeasures),
}
```

### Version gating

`LocalTxMonitor_V2` (the Measures variant) maps to N2C handshake versions V20+. The transition function gates `GetMeasures` / `ReplyGetMeasures` with explicit version-check; lower N2C versions reject with `InvalidForVersion`.

Soft sentinel `MAX_LOCAL_TX_MONITOR_VERSION` updated to match the `LocalTxMonitorVersion` newtype's intended ceiling.

### Removed types

The S-A2 `LocalTxMonitorQuery(Vec<u8>)` and `LocalTxMonitorReply(Vec<u8>)` newtypes are deleted. No re-export needed since they were never used outside `n2c/local_tx_monitor/event.rs`.

## 10. Changes Introduced

### Types (new)
- `MempoolSizeAndCapacity` (3 u32 fields)
- `MempoolMeasures` (struct with tx_count + BTreeMap)
- `MeasureName` newtype
- `MeasureSizeAndCapacity` (2 u64 fields)
- `BusyKind` enum (4 variants)

### Types (deleted)
- `LocalTxMonitorQuery`
- `LocalTxMonitorReply`

### Types (reshaped)
- `LocalTxMonitorMessage` — was 7 variants, now 13
- `LocalTxMonitorState` — was 5 plain variants, now Idle/Acquiring/Acquired/Busy{kind}/Done
- `LocalTxMonitorEvent` — was 6 variants, now 12

## 11. Replay, Crash, and Epoch Validation

**Unit tests** (`tx_submission::transition::tests` for codec + `n2c::local_tx_monitor::transition::tests` for state machine):

Codec (`codec::local_tx_monitor::tests`):
- `roundtrip_every_variant` — round-trips all 13 messages
- `decode_rejects_unknown_tag`
- `decode_rejects_truncated_input`
- `measures_variants_gated_by_version` — encode/decode of GetMeasures + ReplyGetMeasures requires LocalTxMonitor_V2 awareness

State machine (`n2c::local_tx_monitor::transition::tests`):
- `idle_acquire_then_acquired`
- `acquired_release_returns_to_idle`
- `acquired_await_acquire_re_acquires`
- `acquired_next_tx_then_reply_with_tx`
- `acquired_next_tx_then_reply_empty_mempool`
- `acquired_has_tx_then_reply_present`
- `acquired_has_tx_then_reply_absent`
- `acquired_get_sizes_then_reply`
- `acquired_get_measures_then_reply` (requires V2)
- `get_measures_rejected_below_v2`
- `client_done_terminates_from_idle`
- `wrong_agency_returns_error`
- `version_gating_rejects_out_of_version_message`

**Integration test** (`tests/local_tx_monitor_event_trace.rs::local_tx_monitor_event_trace`):
- ≥10 curated scenarios covering the 4 query kinds, release-and-reacquire patterns, version-gated Measures use cases. 1000-run determinism check.

## 12. Mechanical Acceptance Criteria

- [ ] `cargo build -p ade_network --all-targets` — clean
- [ ] `cargo test -p ade_network --lib codec::local_tx_monitor::` — codec PASS
- [ ] `cargo test -p ade_network --lib n2c::local_tx_monitor::` — state machine PASS
- [ ] `cargo test -p ade_network --test local_tx_monitor_event_trace` — integration PASS
- [ ] `cargo clippy -p ade_network --all-targets -- -D warnings` — clean
- [ ] All 8 named CI scripts PASS
- [ ] Wire grammar match verified by inspection against `ouroboros-network/protocols/lib/Ouroboros/Network/Protocol/LocalTxMonitor/Codec.hs` from cardano-node 11.0.1 tag

## 13. Failure Modes

`LocalTxMonitorError` (unchanged shape):
- `IllegalTransition { state, message_tag, agency }`
- `InvalidForVersion { version, message_tag }` — fires on GetMeasures/ReplyGetMeasures below V2
- `MalformedMessage { reason }` — fires on, e.g., invalid UTF-8 in MeasureName

All fail-fast.

## 14. Hard Prohibitions

Inherited cluster + slice-specific:
- No HashMap — `MempoolMeasures.measures` uses `BTreeMap<MeasureName, MeasureSizeAndCapacity>`
- No String errors — MalformedMessage reasons are `&'static str`
- No `#[non_exhaustive]` on any rewritten enum
- No re-exporting deleted types under aliases — `LocalTxMonitorQuery`/`Reply` are fully removed; callers (none outside the slice) recompile cleanly

## 15. Explicit Non-Goals

- N2C version 21+ table extension (separate slice)
- N2N V16 `perasSupport` field (separate slice)
- Mempool integration / actual measurement plumbing (future cluster)
- LSQ rework (separate concern; LSQ's Query/Reply opaque approximation IS correct per spec)

## 16. Completion Checklist

- [ ] codec/local_tx_monitor.rs rewritten with 13-variant taxonomy
- [ ] state.rs has Busy{kind} state
- [ ] event.rs has 12-variant event taxonomy
- [ ] transition.rs has 13+ legal transitions
- [ ] Version-gated GetMeasures + ReplyGetMeasures
- [ ] Integration test rewritten with ≥10 scenarios
- [ ] 1000-run determinism check
- [ ] All 16+1 CI scripts PASS
- [ ] Registry DC-PROTO-04 and CN-WIRE-07 code_locus/tests reflect the rewrite (entries stay; content represents the corrected coverage)

## 17. Review Notes

**Invariant risk**:
- **Backward-incompatible API break** within the slice: `LocalTxMonitorQuery` / `LocalTxMonitorReply` newtypes are removed. Internal-only types (no public consumers outside `ade_network` at this point) so the break is localized. Future N-B (consensus runtime) consumers will use the new event taxonomy.
- **Wire grammar verified**: the design summary's tag table is taken verbatim from cardano-node 11.0.1's `LocalTxMonitor/Codec.hs` (`encodeWord 0..12` tags + agency/state-discriminated decode arms).
- **Same-tag-different-message disambiguation**: `MsgAcquire` (Idle/Client) and `MsgAwaitAcquire` (Acquired/Client) both encode as `[1]`. The state machine disambiguates by current state; the codec emits `[1]` and the state machine knows what it must mean given the state.

**Assumptions challenged**:
- Considered keeping the opaque `Query(Vec<u8>)` shape and pretending each variant was a different opaque payload. Rejected — fails the closed-wire-grammar discipline (DC-PROTO-04, CN-WIRE-07) and would still fail live interop because the tag values differ.
- Considered modeling `measureMap` as `Vec<(MeasureName, MeasureSizeAndCapacity)>`. Rejected — upstream is `Map`, so `BTreeMap` matches determinism and ordering guarantees better.

**Follow-up implied**: N2C version table extension to V21..V23 (separate small slice S-A8c if needed); N2N V15/V16 + perasSupport (separate small slice S-A8d).

## 18. Authority Reminder

Authority for invariants in `docs/ade-invariant-registry.toml`; mechanical acceptance in §12. Wire-grammar authority: cardano-node 11.0.1 `ouroboros-network/protocols/lib/Ouroboros/Network/Protocol/LocalTxMonitor/Codec.hs`.
