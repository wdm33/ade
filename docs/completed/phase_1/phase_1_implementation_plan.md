# Implementation Plan: Phase 1 — Canonical Wire Truth

## Context

Phase 0A/0B established the workspace scaffold (8 crates), golden CBOR corpus (42 mainnet blocks, 7 eras), differential block harness (`BlockDecoder` trait, `BlockFields`, `DiffReport`), reference oracle data (42 JSON block field files), and 8 CI scripts. `ade_codec` and `ade_types` are empty shells (contract headers + deny attributes only). `ade_testkit` has the full harness infrastructure.

Phase 1 must close the byte-authority loop on the full current golden corpus (42 mainnet blocks spanning all 7 eras), proving byte-identical round-trip and establishing the infrastructure required to extend that proof set to further corpus entries. This is scoped to the evidence available — it does not yet prove ingestion of every block the network has ever produced, only that the codec pipeline is correct on the named corpus and that the invariant clusters are closed for that evidence set.

## Constitutional Framing

### Dual Byte-Authority Model

Phase 1 enforces two distinct byte-authority surfaces. These are never collapsed:

1. **`.wire_bytes()`** — Original bytes as received from the network or ImmutableDB. Mandatory for all hash-critical paths. Round-trip identity is a compatibility proof: `decode(wire_bytes).re_encode() == wire_bytes`.

2. **`.canonical_bytes()`** — Project-canonical re-encoding. Governs internal replay and evidence surfaces. Deterministic but NOT used for hash computation.

### Four-Tier Classification

| Tier | Meaning |
|------|---------|
| **True** | Runtime invariants that must hold at all times |
| **Derived** | Proven consequences of true invariants |
| **Release** | CI/test gates enforcing true/derived invariants |
| **Operational** | Process discipline |

## Declared Representation Policies

1. **Wire-Order Maps**: `Vec<(K, V)>` for all wire-ordered CBOR maps (preserves insertion/wire order)
2. **Opaque Substructures**: `PreservedCbor<RawCbor>` for sub-structures not needed for Phase 1
3. **CBOR Encoding Metadata**: Non-canonical encoding details preserved (indefinite-length, non-minimal widths)
4. **Tag 24 Boundaries**: Every tag 24 wraps inner content in `PreservedCbor<T>`
5. **Malformed Input Regression**: Discovered malformed inputs become permanent regression fixtures
6. **Storage-Path Chokepoint Scope**: Declared but not yet implemented until Phase 5

## Invariant Cluster Organization

- **Cluster A**: Wire Decode Authority (T-INGRESS-01, DC-INGRESS-01, CN-WIRE-04, CN-WIRE-06, T-ERR-01)
- **Cluster B**: Byte Authority & Preservation (T-ENC-01, T-ENC-03, DC-CBOR-02, CN-WIRE-01, CN-WIRE-02, T-DET-01)
- **Cluster C**: Era Envelope & Closed Discriminants (T-ENC-02, CN-WIRE-07, T-CORE-03)
- **Cluster D**: Map/Order/Width Preservation (T-COLL-01, T-ENC-03, DC-CBOR-01)
- **Cluster E**: Reference-Equivalence Evidence (T-CI-01, DC-REF-01, CN-META-03)

## Execution Slices

| Slice | ID | Scope | Corpus |
|-------|----|-------|--------|
| 1 | T-07 | CBOR codec core: PreservedCbor, traits, error, primitives, envelope, CardanoEra, 3 CI scripts | Infrastructure |
| 2 | T-08 | Byron EBB + regular block types, decode/encode, GREEN adapter | 3 blocks |
| 3 | T-09 | Shelley block types (15-field header), decode/encode, GREEN adapter | 3 blocks |
| 4 | T-10 | Allegra + Mary block round-trip (combined — minimal incremental extensions) | 6 blocks |
| 5 | T-11 | Alonzo block round-trip (Plutus infrastructure, array(5) block) | 3 blocks |
| 6 | T-12 | Babbage block round-trip (dual-format TxOut, array(10) header) | 12 blocks |
| 7 | T-13 | Conway block round-trip (CIP-1694 governance) — final era, triggers corpus closure | 15 blocks |
| 8 | T-14 | Address encoding round-trip (6 address variants, VarInt pointer, Byron double-CBOR) | Cross-era |

### Dependency Graph

```
T-07 ─┬── T-08 (Byron)          [parallel after T-07]
       ├── T-09 (Shelley)        [parallel after T-07]
       │    ├── T-10 (Allegra/Mary)  [after T-09]
       │    │    └── T-11 (Alonzo)   [after T-10]
       │    │         └── T-12 (Babbage) [after T-11]
       │    │              └── T-13 (Conway) [after T-12]
       │    └── T-14 (Addresses)     [parallel with T-10+]
       └── [T-08 & T-09 independent]
```

## Verification (per slice)

```bash
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
bash ci/ci_check_module_headers.sh
bash ci/ci_check_forbidden_patterns.sh
bash ci/ci_check_dependency_boundary.sh
bash ci/ci_check_no_semantic_cfg.sh
bash ci/ci_check_no_signing_in_blue.sh
bash ci/ci_check_no_secrets.sh
bash ci/ci_check_ref_provenance.sh
bash ci/ci_check_constitution_coverage.sh
bash ci/ci_check_cbor_round_trip.sh
bash ci/ci_check_hash_uses_wire_bytes.sh
bash ci/ci_check_ingress_chokepoints.sh
```

After T-13 (final era): all 42 blocks round-trip, all 42 differential comparisons zero divergences.
