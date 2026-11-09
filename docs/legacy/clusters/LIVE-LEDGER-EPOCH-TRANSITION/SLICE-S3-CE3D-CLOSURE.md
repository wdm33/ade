# CE-3d — Final Green Declaration (S3 boundary-gate closure)

**Date:** 2026-07-08. **Status:** CE-3d byte-exact, CLOSED. **Scope of this note:** docs / registry /
evidence only — no code. It records the admissibility proof for the LIVE-LEDGER-EPOCH-TRANSITION S4 slice;
it does not itself perform S4.

## Claim boundary (authoritative)

> CE-3d is closed for rewards, fee/pot accounting, go-snapshot pool-set membership, go-stake values, and
> warm-restart replay across the two tested self-derived boundaries. This authorizes S4 scoping; it does
> not itself activate accumulator-derived leadership authority.

**S4 is NOT part of this closure.** The accumulator-derived leadership-authority flip remains a separate,
later authority-promotion slice ([[SLICE-S4-contract]]).

## Two distinct boundary sequences — read carefully (no off-by-one)

There are TWO separate boundary sequences in this proof. They must not be conflated.

1. **Runtime-followed boundary lineage (builds the declaration seed).** The Ade node bootstrapped at
   epoch **1338** (Mithril judge snapshot, certified point slot 115676685) and FOLLOWED the preview peer,
   crossing the runtime epoch boundaries it observed on the wire:
   - `CROSSED boundary 1338 -> 1339` at slot 115689630 — builds `mark(1339)`.
   - `CROSSED boundary 1339 -> 1340` at slot 115776011 (via a warm-start after the first boundary's
     wire-pump EOF) — builds `mark(1340)`.

   The seed is sealed at the **runtime-followed 1339->1340 boundary** (last_advanced in epoch 1340). Both
   marks are built under the DC-EPOCH-24 `ssActiveStake` NonZero rule + the schema-reject strengthening.

2. **Differential-comparison boundaries (the two tested self-derived boundaries).** The CE-3d differential
   harness takes that seed and advances it across two FURTHER self-derived boundaries, comparing Ade's
   self-derived post-boundary state against cardano `db-analyser` REFERENCE ledger states:
   - self-derived cross `1340 -> 1341`, compared at the reference label **POST-1341** (db-analyser
     reference at slot **115862416**).
   - self-derived cross `1341 -> 1342`, compared at the reference label **POST-1342** (db-analyser
     reference at slot **115948834**).

**The connection (why this is not off-by-one):** the mark/set/go rotation means `go(1341) = mark(1339)` and
`go(1342) = mark(1340)`. So the reference-labeled POST-1341 / POST-1342 comparison is exactly what
validates the runtime-built `mark(1339)` / `mark(1340)`, two rotations downstream. The runtime boundaries
(1339, 1340) and the reference labels (1341, 1342) are distinct index sets bridged by the rotation — NOT the
same boundary described two ways.

## Declaration-grade artifact + manifest

| Item | Value |
|---|---|
| Declaration seed | `~/.cardano-ce3d-s1seed-v5`, accumulator **schema v4** |
| Bootstrap source | Mithril judge snapshot `~/.cardano-preview-judge/preview-snapshot` (certified point slot 115676685, epoch 1338) |
| Historical-block peer | docker `cardano-node-preview`, N2N `127.0.0.1:3002` |
| Binary | `target/release/ade` @ HEAD `392433a1` |
| Accumulator content hash (blake2b-256) | `b8c80ad1c9dd7c33af2bcb07584aff1a…` |
| Reduced-checkpoint content hash (blake2b-256) | `b7b2fb3d3a8d9826eb6f76a5b1906277…` |
| Reference POST-1341 | `db/ledger/115862416_db-analyser/state` |
| Reference POST-1342 | `db/ledger/115948834_db-analyser/state` |

## Byte-exact result (v5, both tested self-derived boundaries)

| Field | reference POST-1341 | reference POST-1342 | closes |
|---|---|---|---|
| treasury | d0 MATCH | d0 MATCH | fee/pot (DC-EPOCH-23) |
| reserves | d0 MATCH | d0 MATCH | fee/pot (DC-EPOCH-23) |
| rewards (per account) | 90142 keys, only_ade=0, only_ref=0, d0 | 90143 keys, only_ade=0, only_ref=0, d0 | rewards |
| go_pool_stakes (set) | 626/626, only_ade=0, only_ref=0, d0 | 626/626, only_ade=0, only_ref=0, d0 | pool-set membership (DC-EPOCH-24) |
| go_pool_stakes (total / values) | d0 | d0 | go-stake values |

## Warm-restart replay proof (under schema-v4)

- `boundary_stateful_replay` (4), `boundary_fingerprint_agreement` (1), `boundary_replay` (12) — all green
  after the schema-v4 bump.
- The v5 seed's `mark(1340)` was built during a genuine **cross-process warm-restart** (the seed sealed at
  the 1338->1339 boundary, a fresh process reloaded the schema-v4 store and advanced to the 1339->1340
  boundary); the differential's POST-1342 `only_ade=0` (i.e. `go(1342)=mark(1340)`) validates that
  warm-started mark byte-for-byte.

## Enforcement flips (this note's registry changes)

- **DC-EPOCH-23** `declared -> enforced`, scoped STRICTLY to fee/pot vector exactness (treasury + reserves
  d0 above).
- **DC-EPOCH-24** `declared -> enforced`, scoped STRICTLY to snapshot pool-set membership (go_pool_stakes
  626/626 only_ade=0 above), including the CE3D-GO-POOLSET-SCHEMA-REJECT strengthening.

Neither flip asserts accumulator-derived leadership authority.

## Commit lineage

`fd8b07c8` (fee-pot deltaF, A+B) -> `e469f878` (snapshot pool-set membership, C) ->
`392433a1` (schema-reject, C persisted-side) -> this declaration.

## Next valid work item

The [[SLICE-S4-contract]] sealed authority-promotion flip (contract + implementation). NOT more CE-3d
residual hunting — CE-3d is byte-exact across the two tested self-derived boundaries per the claim boundary
above.
