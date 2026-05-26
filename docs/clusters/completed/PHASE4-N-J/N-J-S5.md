# Invariant Slice — PHASE4-N-J S5

## Slice Header

**Slice Name:** `ProtocolParameters` + `ConwayOnlyDepositParams` + `ConwayGovState` encode/decode
**Cluster:** PHASE4-N-J
**Status:** In Progress
**CEs addressed:** CE-N-J-5
**Registry effects on merge:** none directly; assembled into S6
combined-LedgerState encoder, framed in S7.
**Dependencies:** S1 (chain_dep, error sums), S4 (epoch_state).

---

## Intent

Encodes the three remaining LedgerState sub-states needed before
the assemble step (S6).

* `ade_ledger::snapshot::gov_state::encode_pparams` /
  `decode_pparams` — array(24) of the 24 protocol-parameter fields
  with two reserved tail slots for forward-compatible extension.
  Rationals encode as `array(2)[int, int]` so any sign is preserved
  via `MAJOR_NEGATIVE`; `cost_models_cbor` encodes as
  `null | bytes` to preserve the byte-for-byte aiken wire form.
* `ade_ledger::snapshot::gov_state::encode_conway_deposit_params` /
  `decode_conway_deposit_params` — array(3) of the three Conway-only
  deposit parameters (`drep_deposit`, `gov_action_deposit`,
  `drep_activity`). Used at the `LedgerState.conway_deposit_params`
  `Option` slot in S6.
* `ade_ledger::snapshot::gov_state::encode_gov_state` /
  `decode_gov_state` — array(9) of `ConwayGovState`'s 9 BTreeMap-
  iterating fields plus the proposals Vec, vote thresholds Vecs, and
  the committee_quorum tuple. Inner `GovActionState` rides as
  array(9) with a 7-variant tagged `GovAction` payload.

Side fix (caught while bringing S5 green): every snapshot module
that emitted runtime-sized array/map headers (S1-S4 maps,
S5 maps/vecs/24-field pparams) was passing `IntWidth::Inline`,
which silently corrupts headers when length ≥ 24 (e.g.
`PPARAMS_FIELDS = 24` → `0x98` literal, indistinguishable from
"1-byte length follows"). Migrated all runtime-sized container
headers to `ade_codec::cbor::canonical_width(n)`. Constant inline
headers (`array(2)`, `array(3)`, etc.) remain `IntWidth::Inline`.

---

## §12 Mechanical Acceptance Criteria

- `pparams_round_trip_default`
- `pparams_round_trip_with_cost_models`
- `pparams_encode_deterministic_across_runs`
- `conway_deposit_params_round_trip`
- `gov_state_round_trip_empty`
- `gov_state_round_trip_all_gov_action_variants` (covers all 7
  `GovAction` variants and both `DRep` discriminated + always-X
  variants)
- `gov_state_encode_deterministic_across_runs`

Regression coverage on the canonical-width fix: the existing 17
S1-S4 tests continue to pass against the migrated headers, confirming
backwards-compatibility with the in-cluster wire shape.

---

## §14 Hard Prohibitions

- No HashMap/HashSet/wall-clock/tokio/rand/float literals in
  `snapshot::gov_state`.
- No `String`-bearing variants in any error path.
- `IntWidth::Inline` only for headers whose length is a compile-
  time constant ≤ 23.

---

## §15 Explicit Non-Goals

- LedgerState assemble (S6), combined snapshot framing (S7),
  persistent cache (S8) and DC-CONS-21 closure.
- Cross-impl byte equivalence — deferred to the S8 corpus comparison.
- Future-version migration paths — schema version stays at 1 in
  this cluster.
