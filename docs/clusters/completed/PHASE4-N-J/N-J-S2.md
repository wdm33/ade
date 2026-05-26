# Invariant Slice — PHASE4-N-J S2

## Slice Header

**Slice Name:** `UTxOState` encode/decode (BTreeMap traversal; all 3 TxOut eras)
**Cluster:** PHASE4-N-J
**Status:** In Progress
**CEs addressed:** CE-N-J-2
**Dependencies:** N-J-S1

---

## Intent

The largest sub-state and the most important place to prove
deterministic BTreeMap traversal. Wire shape:

* outer `map(N)` keyed by TxIn (`array(2)[bytes(32), uint]`)
* TxOut tagged-array `array(3)[era_tag, address_or_bytes, payload]`
  with era_tag ∈ {0=Byron, 1=ShelleyMary, 2=AlonzoPlus}
* MultiAsset as `map(P) { policy_bytes → map(A) { name_bytes → int } }`
* Integer encoding handles `i64::MIN` via i128 intermediate to
  avoid two's-complement overflow on `-(v + 1)`.

---

## §12 Mechanical Acceptance Criteria

- `utxo_state_round_trip_empty`
- `utxo_state_round_trip_all_eras` — Byron + ShelleyMary + AlonzoPlus
- `utxo_state_encode_deterministic_across_runs`
- `utxo_state_negative_multi_asset_quantity_round_trips` — covers
  `i64::MIN`, `-1`, `0`, `1`, `i64::MAX`.
- `utxo_state_iteration_order_is_btreemap` — two insertion orders
  produce identical bytes.

---

## §14 Hard Prohibitions

- Same as S1.
- No `i64::MIN` panic: integer encoding/decoding goes through i128.
