# Genesis fixtures — OQ7 proof obligation capture

> **Phase:** PHASE4-N-R-A S1 / N-R-PREFLIGHT.
> **Captured:** 2026-05-27 from the local docker container
> `cardano-node-preprod`'s bind-mounted config directory
> (`.cardano-node-preprod/config/`).
>
> **Important finding from capture:** despite the cluster
> plan's framing as "Conway genesis parser," the
> load-bearing slot/KES geometry fields actually live in
> **`shelley-genesis.json`**, not `conway-genesis.json`.
> Conway genesis carries governance parameters
> (`poolVotingThresholds`, `dRepVotingThresholds`,
> `committeeMinSize`, etc.) but NOT the
> `(systemStart, slotLength, slotsPerKESPeriod,
> maxKESEvolutions, networkMagic)` set the N-Q
> `GenesisAnchor` requires.
>
> **Decision (records refinement to the cluster plan):** the
> "Conway genesis parser" of N-R-C C2 is in fact a
> **Shelley-genesis-+-Conway-genesis-addendum parser**. The
> required-field set lives in Shelley genesis. Conway-only
> fields (governance thresholds) are NOT required by the
> N-Q `GenesisAnchor`; C2 parses Shelley + treats Conway as
> an optional companion file for forward-compat.

## Required-field set (Shelley genesis)

C2's closed-contract parser requires these fields with the
listed types:

| JSON key | Type | Maps to `GenesisAnchor` field |
|---|---|---|
| `networkMagic` | non-negative integer (u32) | `network_magic` |
| `systemStart` | ISO 8601 UTC timestamp (`"YYYY-MM-DDTHH:MM:SSZ"`) | `slot_zero_time_unix_ms` (parsed via deterministic ISO-to-unix-ms helper; no library permissiveness) |
| `slotLength` | non-negative integer or non-negative number representing seconds | `slot_length_ms` (multiply by 1000) |
| `slotsPerKESPeriod` | non-negative integer (u64) | `slots_per_kes_period` |
| `maxKESEvolutions` | non-negative integer (u32) | `kes_max_period` |

**Note on `kes_anchor_slot`:** the Shelley genesis does NOT
provide this field directly. It is operator-supplied (the
slot at which the KES seed was generated) and lives outside
the genesis parser. In production it equals 0 for a fresh
KES key generated at chain start; for live runs it equals
the slot at which the operator generated their KES seed.

## Fixtures

| File | Purpose | Expected parser outcome |
|---|---|---|
| `accepted-shelley-genesis.json` | Real preprod Shelley genesis from docker | Accept; produce `GenesisAnchor` populated from required fields |
| `accepted-conway-genesis.json` | Real preprod Conway genesis from docker (governance addendum) | Accept (optional companion); fields not consumed by `GenesisAnchor` |
| `missing-required.shelley-genesis.json` | Missing `networkMagic` field | Reject with `MissingRequiredField { name: "networkMagic" }` |
| `stringly-int.shelley-genesis.json` | `networkMagic` is `"1"` (string) instead of `1` (number) | Reject with `MalformedFieldType { name: "networkMagic", expected: "u32", found: "string" }`; per DQ-C2, no stringly fallback |
| `extra-inert-key.shelley-genesis.json` | Extra `futureExtensionField` + `unknownNestedObject` keys not in the required set | Accept (extra-key tolerance per DQ-C2); required fields parse identically to `accepted-shelley-genesis.json` |
| `malformed-numeric.shelley-genesis.json` | `slotLength: -1` (negative integer) | Reject with `MalformedFieldValue { name: "slotLength", reason: "must be non-negative" }` |

## Hard rules (carry-forward from DQ-C2)

- **Required fields MUST fail-closed** on missing / malformed
  / wrong-type input. No implicit defaults
  ("if missing, assume preprod"). No stringly fallback
  (`"1"` rejected for u32 fields).
- **Extra keys are accepted-and-ignored** for forward
  compatibility, but only if they do not collide with
  required field names. The parser MUST NOT use any extra
  key to alter interpretation of the required set.
- **No semantic weakening through extra-key tolerance.**
  Extras must be inert: the `GenesisAnchor` produced by
  `extra-inert-key.shelley-genesis.json` MUST byte-equal the
  `GenesisAnchor` produced by `accepted-shelley-genesis.json`.

## Real preprod values (for cross-impl checks)

From `accepted-shelley-genesis.json`:

| Field | Value |
|---|---|
| `networkMagic` | `1` |
| `systemStart` | `"2022-06-01T00:00:00Z"` → `1654041600000` ms |
| `slotLength` | `1` second → `1000` ms |
| `slotsPerKESPeriod` | `129600` |
| `maxKESEvolutions` | `62` |
| `epochLength` | `432000` |
| `activeSlotsCoeff` | `0.05` (consumed elsewhere, not by `GenesisAnchor`) |
| `securityParam` | `2160` (consumed elsewhere) |

## Use sites

- N-R-A A2: not consumed (leader-check evaluator gets
  `LeaderScheduleAnswer` from the caller).
- N-R-A A3: produce_mode integration test uses the captured
  `GenesisAnchor` values directly.
- N-R-C C2: parser test suite consumes ALL fixtures here
  (2 accept + 4 reject paths).
