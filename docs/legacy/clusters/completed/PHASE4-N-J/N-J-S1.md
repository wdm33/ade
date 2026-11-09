# Invariant Slice — PHASE4-N-J S1

## Slice Header

**Slice Name:** `PraosChainDepState` encode/decode + closed error sums
**Cluster:** PHASE4-N-J
**Status:** In Progress
**CEs addressed:** CE-N-J-1
**Registry effects on merge:** none directly; foundation for S7's flips.
**Dependencies:** PHASE4-N-I (rollback module tree).

---

## Intent

Sets the encoder pattern the cluster will reuse 7 more times.
Smaller sub-state (5 nonces + 3 Option<u64> + OpCertCounterMap)
gets the cbor-primitives plumbing right before tackling
LedgerState's larger components.

* `ade_ledger::snapshot::error` — `SnapshotEncodeError` + 5-variant
  `SnapshotDecodeError` + `StructuralReason` closed sums. No
  `String`; no `#[non_exhaustive]`.
* `ade_ledger::snapshot::chain_dep` — `encode_chain_dep` +
  `decode_chain_dep` via `ade_codec::cbor::*` writers.

Wire shape:
```text
array(9) [
  bytes(32)  evolving_nonce,
  bytes(32)  candidate_nonce,
  bytes(32)  epoch_nonce,
  bytes(32)  previous_epoch_nonce,
  bytes(32)  lab_nonce,
  null | uint  last_epoch_block,
  null | uint  last_slot,
  null | uint  last_block_no,
  array(N) [ array(3)[hash28, kes_period, counter], ... ]
]
```

Definite-length containers; BTreeMap iteration order for the
op_cert_counters map.

---

## §12 Mechanical Acceptance Criteria

- `chain_dep_round_trip_empty`
- `chain_dep_round_trip_full`
- `chain_dep_encode_deterministic_across_runs`
- `chain_dep_decode_rejects_truncated`
- `chain_dep_decode_rejects_wrong_array_length`
- `snapshot_encode_error_round_trips_through_pattern_match`
- `snapshot_decode_error_round_trips_through_pattern_match`
- `structural_reason_round_trips_through_pattern_match`

---

## §14 Hard Prohibitions

- No HashMap/HashSet/wall-clock/tokio/rand/float literals in the
  snapshot modules.
- No String-bearing variants in the error sums.
- No `#[non_exhaustive]`.

---

## §15 Explicit Non-Goals

- UTxOState (S2), CertState (S3), EpochState (S4), GovState (S5),
  assemble (S6), combined framing (S7), persistent cache (S8).
- Cluster-wide CI canonicality gate — added at S7 when the
  encoder authority site stabilizes.
