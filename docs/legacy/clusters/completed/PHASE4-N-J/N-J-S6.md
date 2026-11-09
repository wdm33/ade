# Invariant Slice — PHASE4-N-J S6

## Slice Header

**Slice Name:** `LedgerState` assemble — full encoder/decoder over S2-S5
**Cluster:** PHASE4-N-J
**Status:** In Progress
**CEs addressed:** CE-N-J-6
**Registry effects on merge:** none directly; carries the `LedgerState`
half of the combined-snapshot bytes flipped to enforced by S7.
**Dependencies:** S2 (utxo), S3 (cert), S4 (epoch), S5 (pparams + gov).

---

## Intent

Composes S2-S5's sub-state encoders into the full `LedgerState`
snapshot.

* `ade_ledger::snapshot::ledger::encode_ledger_state` /
  `decode_ledger_state` — single authority pair for
  `LedgerState <-> bytes`. Wire shape:
  ```text
  array(9) [
    uint  era,                     // == 7 (Conway). Pre-Conway rejected.
    uint  max_lovelace_supply,
    bool  track_utxo,              // harness flag; round-tripped.
    bytes utxo_state_encoded,
    bytes cert_state_encoded,
    bytes epoch_state_encoded,
    bytes pparams_encoded,
    null | bytes gov_state_encoded,
    null | bytes conway_deposit_params_encoded,
  ]
  ```
  Sub-state bodies ride inside `bstr` containers so the outer
  decoder can hand each slice to its specialized decoder without
  sharing offset state — keeps each S2-S5 decoder's `&[u8]`-rooted
  signature intact.
* Conway-only scope discipline (matches S1-S5): encoder + decoder
  both fail closed with `EraNotSupported` for any
  `(era as u8) < (CardanoEra::Conway as u8)`. Unknown era tags
  (≥ 8) reject as `EraTagOutOfRange`.

Field order on the wire matches `LedgerState`'s struct field order
(not the `fingerprint` walk order — fingerprint groups by component
hash rather than struct shape). Round-trip equivalence is proven
via `fingerprint(decode(encode(s))) == fingerprint(s)`.

---

## §12 Mechanical Acceptance Criteria

- `ledger_state_round_trip_empty_conway`
- `ledger_state_round_trip_populated_conway` (UTxOState with Byron +
  ShelleyMary variants, CertState with registrations + delegations,
  EpochState with all three snapshot slots populated + block_production,
  optional gov_state, optional conway_deposit_params)
- `ledger_state_encode_deterministic_across_runs`
- `encode_then_decode_roundtrips_via_fingerprint` (proves semantic
  equivalence under the project's canonical fingerprint walk)
- `pre_conway_era_is_structurally_rejected_on_encode` (all 7
  pre-Conway eras → `EraNotSupported`)
- `pre_conway_era_is_structurally_rejected_on_decode` (byte-patched
  Shelley tag → `EraNotSupported`)
- `decode_rejects_unknown_era_tag` (era tag 9 → `EraTagOutOfRange`)

---

## §14 Hard Prohibitions

- No HashMap/HashSet/wall-clock/tokio/rand/float literals in
  `snapshot::ledger`.
- No pre-Conway encode/decode success path.
- Sub-state decoders must continue to take `&[u8]` (no shared
  offset) — the bstr wrapping is what preserves that contract.

---

## §15 Explicit Non-Goals

- Combined snapshot framing (S7) — version tag + fingerprint
  cross-check + `(LedgerState, PraosChainDepState)` tuple
  ride at a higher framing layer added in S7.
- Persistent cache wiring (S8) — DC-CONS-21 closure happens at S8
  after the framing layer is in place.
