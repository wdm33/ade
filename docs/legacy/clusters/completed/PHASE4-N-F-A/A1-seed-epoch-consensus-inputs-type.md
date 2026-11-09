# Slice PHASE4-N-F-A / A1 — SeedEpochConsensusInputs type + sole codec + sidecar storage shape

## 2. Slice Header
- **Slice Name:** SeedEpochConsensusInputs BLUE type + sole canonical codec + fingerprint-bound sidecar storage shape.
- **Cluster:** PHASE4-N-F-A — Seed-Epoch Consensus Input Provenance.
- **Status:** Merged (`c13c2e9`); CE-A-1 closed.
- **Cluster Exit Criteria Addressed:**
  - [ ] **CE-A-1** — a closed canonical persisted consensus-input surface exists (version-gated decode rejecting unknown version; round-trips byte-canonically).
  - *(CE-A-2 / CE-A-3 / CE-A-4 are explicitly out of scope for A1.)*
- **Slice Dependencies:** none (first slice of the cluster).

## 3. Implementation Instruction (AI)
Implement exactly §10. Define one BLUE type + one sole codec pair + the tests in §12. Do **not** add persistence I/O, bootstrap population, recovery, projection, or any produce wiring — those are A2/A3/A4. Do not embed the record in `BootstrapAnchor`. Commit message carries the repo's model-attribution trailer (per CLAUDE.md), no other AI references.

## 4. Intent
Make it impossible to represent the seed-epoch consensus inputs as anything other than a **closed, version-gated, byte-canonical** record — so that the recovered surface a producer will later consume (N-F-B) has exactly one encoding and one decoder, and a malformed or unknown-version record fails closed rather than being silently reinterpreted. *(Introduces candidate `CN-CINPUT-01`.)*

## 5. Scope
- **Modules / crates:** `ade_ledger` (BLUE) — a new module `ade_ledger::seed_consensus_inputs` (type + sole codec). Reuses `ade_ledger::consensus_view::PoolEntry` and `ade_core::consensus::vrf_cert::ActiveSlotsCoeff`, `ade_types::{Hash28, Hash32, EpochNo}`, `ade_codec::cbor` canonical helpers.
- **State machines affected:** none.
- **Persistence impact:** defines the canonical **bytes** of the sidecar record; **no disk I/O in this slice** (write/read is A2/A3). The storage-shape *decision* is recorded here (sidecar), not its I/O.
- **Network-visible impact:** none.
- **Out of scope:** bootstrap population (A2), recovery restore (A3), projection API (A4), produce wiring, any `BootstrapAnchor` change, any CI gate (A2).

## 6. Execution Boundary (TCB color)
- **BLUE:** `ade_ledger::seed_consensus_inputs` (the `SeedEpochConsensusInputs` type + `encode_/decode_seed_epoch_consensus_inputs` + `SEED_CINPUT_SCHEMA_VERSION`). Pure, deterministic, no I/O.
- **GREEN:** none.
- **RED:** none.
- *No ambiguous colors.* This is a pure BLUE/data slice; it inherits the `ade_ledger` `#![deny(...)]` BLUE lints.

## 7. Invariants Preserved
- `CN-ANCHOR-01` / `DC-ANCHOR-01` — the anchor codec stays the **sole** anchor encoder/decoder; A1 adds a *separate* sidecar codec and does **not** touch `BootstrapAnchor` or its codec (no schema bump).
- `T-DET-01` and the BLUE forbidden-pattern set — deterministic, no `HashMap`/clock/float/async/`String`-error in the new BLUE module (`BTreeMap` only).
- `T-ENC-01` — canonical encoding of persisted/hashed data (the new codec is canonical + byte-identical round-trip).

## 8. Invariants Strengthened or Introduced
- **Introduces** candidate `CN-CINPUT-01` — *the seed-epoch consensus inputs have exactly one closed, version-gated, byte-canonical encoding and one decoder; unknown versions fail closed.* (Registry promotion happens at cluster verify/close, not here — the candidate ID + its `tests` become real once §12 passes.) This slice strengthens exactly one invariant family (consensus-input-surface closure/encoding).

## 9. Design Summary
- Type (BLUE, closed, all fields required — no `Default`, no `#[non_exhaustive]`, mirroring `BootstrapAnchor` discipline):
  ```
  SeedEpochConsensusInputs {
      anchor_fp: Hash32,                       // binds this record to a specific BootstrapAnchor (self-describing)
      epoch_no: EpochNo,                       // the single seed epoch these inputs are valid for
      active_slots_coeff: ActiveSlotsCoeff,
      total_active_stake: u64,
      pool_distribution: BTreeMap<Hash28, PoolEntry>,  // PoolEntry = { active_stake: u64, vrf_keyhash: Hash32 }
  }
  ```
  `vrf_keyhash` is carried inside `PoolEntry` (reusing `consensus_view::PoolEntry`), so there is **no separate `pool_vrf_keyhashes` map** — one map, deterministic `BTreeMap<Hash28,_>` ordering.
- **Storage shape — DECIDED: Option A (fingerprint-bound sidecar).** Evidence: per-pool ≈ 74 bytes (Hash28 key ~30 + `PoolEntry` ~44); ~3000 mainnet pools ≈ **~220 KB**, preprod hundreds ≈ tens of KB; `BootstrapAnchor` is ~200 bytes today. Embedding (Option B schema bump 2→3) would inflate the anchor 100–1000× and turn it from an identity/provenance object into a payload container. So the record is a **standalone canonical sidecar** that carries `anchor_fp` as its binding; it is **not** a `BootstrapAnchor` field, and `ANCHOR_SCHEMA_VERSION` is **not** bumped.
- Sole codec (mirrors `encode/decode_bootstrap_anchor`): `array(N)` headed by `SEED_CINPUT_SCHEMA_VERSION` (= 1); `pool_distribution` encoded via `write_map_header` + canonically-ordered `BTreeMap` iteration (bytes(28) key → `array(2)[uint active_stake, bytes(32) vrf_keyhash]`). Decode rejects any other version fail-fast (typed error), validates the map is canonically ordered + has no duplicate keys, and is byte-canonical (re-encode = input).

## 10. Changes Introduced
### Types
- New BLUE `SeedEpochConsensusInputs` (as §9). New `SEED_CINPUT_SCHEMA_VERSION: u32 = 1`. New closed error enum `SeedConsensusInputsError` (e.g. `UnknownVersion`, `MalformedCbor`, `NonCanonicalMapOrder`, `DuplicatePoolKey`, `TrailingBytes`) — non-secret primitives only.
### State Transitions
- none (pure codec).
### Persistence
- Defines the sidecar record **bytes**; no write/read site added (A2/A3).
### Removal / Refactors
- none. (The RED `consensus_inputs::canonical` import-side canonicalizer is untouched; A1 does not reroute anything.)

## 11. Replay, Crash, and Epoch Validation
- **Replay tests added:** `seed_epoch_consensus_inputs_round_trips_byte_identical` (encode → decode → encode = identical bytes, over a multi-pool fixture); `seed_cinput_encoding_is_btreemap_ordered` (a record built from differently-ordered inserts encodes to identical bytes — determinism).
- **Crash/restart behavior:** n/a in A1 (no persistence site yet; A3 proves recovery byte-identity).
- **Epoch boundary behavior:** n/a (single-epoch record; `epoch_no` is a field, no rotation).

## 12. Mechanical Acceptance Criteria
- [ ] `seed_epoch_consensus_inputs_round_trips_byte_identical` passes (canonical round-trip byte-identity, multi-pool).
- [ ] `seed_cinput_decode_rejects_unknown_version` passes (a record with version ≠ 1 → `SeedConsensusInputsError::UnknownVersion`, fail-closed; no panic).
- [ ] `seed_cinput_encoding_is_btreemap_ordered` passes (deterministic `BTreeMap` ordering; insertion order does not affect bytes).
- [ ] `seed_cinput_decode_rejects_noncanonical_or_duplicate_keys` passes (out-of-order / duplicate pool key → typed error, fail-closed).
- [ ] `cargo build -p ade_ledger` + `cargo clippy -p ade_ledger` clean under the BLUE deny lints (no `HashMap`/clock/float/async/`unwrap`/`panic`/`String`-error in the new module).
- [ ] `cargo test --workspace` stays green (no regression).

## 13. Failure Modes
All fail-closed, typed (`SeedConsensusInputsError`), no panic, non-secret payload: unknown schema version; malformed CBOR / truncated; non-canonical map order; duplicate pool key; trailing bytes. None affect replay yet (no persistence site); all are deterministic.

## 14. Hard Prohibitions
**Inherited (cluster "Forbidden during this cluster"):** no forge/leader-check/KES/VRF signing; no second anchor codec or storage-init authority; no stake computation/rotation; no `HashMap` (use `BTreeMap`); no clock/float/async in BLUE; no registry promotion; no grounding-doc regeneration.
**Slice-specific:**
- No persistence I/O (no `std::fs`, no store read/write) — A2/A3 own that.
- No change to `BootstrapAnchor` or `ANCHOR_SCHEMA_VERSION` — sidecar only (Option A).
- No bootstrap population, no recovery wiring, no projection, no produce wiring.
- No `String`/`anyhow`/formatted errors in the BLUE module; no TODO/placeholder in the codec.

## 15. Explicit Non-Goals
No bootstrap population (A2). No recovery restore (A3). No projection to `PoolDistrView`/`ExpectedVrfInput` (A4). No produce wiring. No `BootstrapAnchor` schema bump. No CI gate (A2). No registry edits.

## 16. Completion Checklist
- [ ] New state canonically encoded (the sidecar bytes).
- [ ] All failure modes deterministic + typed.
- [ ] No TODO/placeholder in the BLUE codec.
- [ ] §12 tests pass in CI; `cargo test --workspace` green.
- [ ] Round-trip byte-identity holds across runs.
