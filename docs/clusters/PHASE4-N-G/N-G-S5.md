# Invariant Slice — PHASE4-N-G S5

## Slice Header

**Slice Name:** GREEN broadcast-to-served adapter + `ServedChainLookups` trait impls + end-to-end session-transcript replay
**Cluster:** PHASE4-N-G
**Status:** In Progress
**CEs addressed:** CE-N-G-5
**Registry flips on merge:** `DC-CONS-17`, `DC-CONS-18`, `DC-PROTO-07` → `enforced`
**Dependencies:** N-G-S1..S4

---

## Intent

Wire PHASE4-N-C's `BroadcastQueue` into PHASE4-N-G S2's
`ServedChainSnapshot` via a pure GREEN adapter, then bridge the
snapshot to the BLUE reducers (S3 chain-sync, S4 block-fetch) via
`ServedHeaderLookup` / `ServedRangeLookup` trait impls. Drive an
end-to-end session-transcript replay test that closes the three
remaining N-G invariants in one shot.

---

## The change

### 1. `crates/ade_runtime/src/producer/broadcast_to_served.rs` (GREEN, new)

`drain_and_admit(snap, queue) -> (snap', queue', drained)`:
deterministically drains the BroadcastQueue and admits each
AcceptedBlock into the served chain. Returns the drained list so the
orchestrator (S6) can drive `advance_tip` per arrival.

### 2. `crates/ade_runtime/src/producer/served_chain_lookups.rs` (GREEN, new)

`ServedChainLookups<'a> { snap: &'a ServedChainSnapshot }`:
- `impl ServedHeaderLookup` — `next_after`, `intersect`, `tip`.
  `next_after` projects the next admitted block's header via the
  canonical `accepted_block_header_bytes` (DC-CONS-16); decodes once
  for `block_no`.
- `impl ServedRangeLookup` — `range_bytes` projects the snapshot's
  range into owned `Vec` triples for the block-fetch reducer.

### 3. `crates/ade_ledger/src/producer/served_chain.rs` (S2 extension)

Adds `iter_accepted` and `block_at` accessors that expose
`&AcceptedBlock` references. This is the only path that lets the
GREEN adapter project headers without bifurcating the splitter.

### 4. End-to-end test `crates/ade_runtime/tests/server_paths_transcript_replay.rs`

Drives the full pipeline:
1. Build broadcast queue from corpus AcceptedBlock arrivals.
2. `drain_and_admit` → `ServedChainSnapshot`.
3. `ServedChainLookups` → BLUE reducer trait inputs.
4. Drive a synthetic peer-message sequence.
5. Collect outgoing wire-frame bytes.

### 5. CI gate `ci/ci_check_broadcast_to_served_purity.sh`

- No HashMap/HashSet/wall-clock/tokio/rand in either GREEN module.
- Positive presence: `drain_and_admit`, both `impl` blocks, and
  `accepted_block_header_bytes` import in `served_chain_lookups.rs`.

---

## §12 Mechanical Acceptance Criteria (named tests)

In `crates/ade_runtime/src/producer/broadcast_to_served.rs` (unit):
- `drain_and_admit_admits_every_queued_block`
- `drain_and_admit_is_deterministic_over_arrival_sequence`
- `drain_and_admit_no_io_no_clock`

In `crates/ade_runtime/src/producer/served_chain_lookups.rs` (unit):
- `empty_snapshot_next_after_yields_none`
- `empty_snapshot_intersect_yields_none`
- `empty_snapshot_tip_yields_none`

In `crates/ade_runtime/tests/server_paths_transcript_replay.rs`
(integration; the three CE-N-G-5-closing tests):
- `session_transcript_replay_byte_identical` (DC-PROTO-07 closure)
- `session_transcript_served_block_bytes_equal_admitted_accepted_block_bytes`
  (DC-CONS-17 closure)
- `session_transcript_announced_header_matches_served_body_recipe`
  (DC-CONS-18 closure)

CI: `ci/ci_check_broadcast_to_served_purity.sh` (new).

---

## §14 Hard Prohibitions

- No HashMap/HashSet/wall-clock/tokio/rand in the GREEN modules.
- No parallel header splitter — `served_chain_lookups.rs` MUST use
  `accepted_block_header_bytes` directly. Enforced by
  `ci/ci_check_no_parallel_header_splitter.sh` + the positive grep
  in the new purity gate.
- No public constructor of `ServedChainLookups` that takes anything
  other than a `&ServedChainSnapshot` reference (the only canonical
  source of admitted bytes).

---

## §15 Explicit Non-Goals

- RED per-peer orchestrator (S6) — this slice provides the GREEN
  adapter + trait impls the orchestrator will call; the orchestrator
  itself is S6.
- Cross-impl mechanical adapter + live evidence (S7).

---

## Replay obligations

`session_transcript_replay_byte_identical` is the canonical
replay-equivalence test for the new session-transcript surface;
strengthens `T-DET-01` and `T-ENC-01` (registry update at cluster
close, not at this slice).

---

## Authority reminder

If this slice conflicts with the project's normative specifications
or the invariant registry, those win.
