# Phase 1 Closing Report — Canonical Wire Truth

**Cluster**: Phase 1
**Status**: Complete
**Date completed**: 2026-03-16

---

## Summary

Phase 1 closed the byte-authority loop on the full golden corpus (42 mainnet blocks spanning all 7 Cardano eras), proving byte-identical round-trip and establishing the infrastructure required to extend that proof set to further corpus entries. The CBOR codec pipeline in `ade_codec` can now decode any mainnet block from the corpus, re-encode it, and produce byte-identical output. Differential comparison against independently extracted reference oracle data confirms that the decoded fields match the Haskell node's interpretation with zero divergences.

The work was organized around five invariant clusters (Wire Decode Authority, Byte Authority & Preservation, Era Envelope & Closed Discriminants, Map/Order/Width Preservation, Reference-Equivalence Evidence) and implemented across 8 slices (T-07 through T-14) in 6 commits.

---

## Exit Criteria Verification

| Proof Obligation | Cluster | Result |
|------------------|---------|--------|
| All 42 corpus blocks produce correct `CardanoEra` via `decode_block_envelope()` | C | PASS |
| `PreservedCbor<T>` has no public constructor (chokepoint enforcement) | A | PASS — `pub(crate)` verified |
| `CodecError` variants carry correct byte offsets, no `String` fields | A | PASS |
| Negative corpus: truncated/malformed/unknown-era-tag inputs produce `CodecError`, never panic | A | PASS |
| All 42 corpus blocks decode → re-encode byte-identically | B+D | PASS |
| All 42 differential comparisons match reference oracle JSON with zero divergences | E | PASS |
| `PreservedCbor<T>` `.wire_bytes()` returns original input for all decoded blocks | B | PASS |
| Byron tag 24 boundaries preserved through decode/re-encode | D | PASS — opaque substructures carry raw bytes |
| Opaque substructure preservation (SSC, delegation, update, block_sig) | B | PASS — `Vec<u8>` identity |
| Post-Byron map key ordering preserved via opaque body passthrough | D | PASS |
| Shelley-Alonzo inlined cert/version round-trips (array(15) header) | D | PASS — 3+3+3+3 = 12 blocks |
| Babbage-Conway nested cert/version round-trips (array(10) header) | D | PASS — 12+15 = 27 blocks |
| Address decode chokepoint classifies by header byte type | A | PASS |
| All existing CI scripts pass (non-regression) | Release | PASS — 11 scripts |
| `cargo test --workspace` + `cargo clippy` zero warnings | Release | PASS — 166 tests |

---

## What Was Delivered

### T-07: CBOR Codec Core & PreservedCbor

| Deliverable | Location |
|-------------|----------|
| `CodecError` structured error enum (6 variants, offset-carrying, `PartialEq + Eq`) | `crates/ade_codec/src/error.rs` |
| `AdeEncode`, `AdeDecode` traits, `CodecContext` | `crates/ade_codec/src/traits.rs` |
| `PreservedCbor<T>` (wire/canonical/decoded surfaces, `pub(crate)` constructor) | `crates/ade_codec/src/preserved.rs` |
| `RawCbor` opaque byte carrier with identity codec | `crates/ade_codec/src/preserved.rs` |
| CBOR byte-level primitives: read/write with encoding width preservation | `crates/ade_codec/src/cbor/mod.rs` |
| `IntWidth` enum, `ContainerEncoding` enum, `canonical_width()` | `crates/ade_codec/src/cbor/mod.rs` |
| `decode_block_envelope()` top-level chokepoint | `crates/ade_codec/src/cbor/envelope.rs` |
| `CardanoEra` enum (8 variants, `TryFrom<u8>`, `#[repr(u8)]`) | `crates/ade_types/src/era.rs` |
| `SlotNo`, `BlockNo`, `EpochNo`, `Hash28`, `Hash32` primitive newtypes | `crates/ade_types/src/primitives.rs` |
| `AdeEncode`/`AdeDecode` impls for all primitive newtypes | `crates/ade_codec/src/primitives.rs` |
| `cardano_era_to_harness_era()` mapping | `crates/ade_testkit/src/harness/era_mapping.rs` |
| `ci_check_cbor_round_trip.sh` | `ci/` |
| `ci_check_hash_uses_wire_bytes.sh` | `ci/` |
| `ci_check_ingress_chokepoints.sh` | `ci/` |
| Envelope integration test (all 42 blocks) | `crates/ade_codec/tests/envelope.rs` |

### T-08: Byron Block Round-Trip

| Deliverable | Location |
|-------------|----------|
| `ByronEbbBlock`, `ByronEbbHeader` types | `crates/ade_types/src/byron/block.rs` |
| `ByronRegularBlock`, `ByronRegularHeader`, `ByronConsensusData` types | `crates/ade_types/src/byron/block.rs` |
| `ByronTx`, `ByronTxIn`, `ByronTxOut`, `ByronTxWitness` stubs | `crates/ade_types/src/byron/tx.rs` |
| `Lovelace`, `ByronAddress` stubs | `crates/ade_types/src/byron/common.rs` |
| `decode_byron_ebb_block()`, `decode_byron_regular_block()` chokepoints | `crates/ade_codec/src/byron/mod.rs` |
| `AdeEncode` impls for all Byron types | `crates/ade_codec/src/byron/block.rs` |
| Byron GREEN adapter (`decode_byron_block_fields()`) | `crates/ade_testkit/src/harness/adapters/byron.rs` |
| Byron round-trip integration test (3 blocks) | `crates/ade_codec/tests/byron_round_trip.rs` |

### T-09: Shelley Block Round-Trip

| Deliverable | Location |
|-------------|----------|
| `ShelleyBlock`, `ShelleyHeader`, `ShelleyHeaderBody` types | `crates/ade_types/src/shelley/block.rs` |
| `VrfData` enum (Split/Combined), `OperationalCert`, `ProtocolVersion` | `crates/ade_types/src/shelley/block.rs` |
| `decode_shelley_block()` chokepoint + generalized `decode_shelley_block_inner()` | `crates/ade_codec/src/shelley/` |
| Shelley GREEN adapter | `crates/ade_testkit/src/harness/adapters/shelley.rs` |
| Shared post-Shelley adapter logic | `crates/ade_testkit/src/harness/adapters/shelley_common.rs` |
| Shelley round-trip integration test (3 blocks) | `crates/ade_codec/tests/shelley_round_trip.rs` |

### T-10: Allegra/Mary Block Round-Trip

| Deliverable | Location |
|-------------|----------|
| `AllegraBlock`, `MaryBlock` type aliases (= `ShelleyBlock`) | `crates/ade_types/src/{allegra,mary}/mod.rs` |
| `AllegraTxBody`, `TimelockScript`, `MaryTxBody`, `Value`, `MultiAsset` stubs | `crates/ade_types/src/{allegra,mary}/` |
| `decode_allegra_block()`, `decode_mary_block()` chokepoints | `crates/ade_codec/src/{allegra,mary}/mod.rs` |
| Allegra/Mary GREEN adapters | `crates/ade_testkit/src/harness/adapters/{allegra,mary}.rs` |
| Allegra/Mary round-trip integration test (6 blocks) | `crates/ade_codec/tests/allegra_mary_round_trip.rs` |

### T-11/T-12/T-13: Alonzo/Babbage/Conway Block Round-Trip

| Deliverable | Location |
|-------------|----------|
| Generalized post-Byron decoder (array(4)/array(5) blocks, array(15)/array(10) headers) | `crates/ade_codec/src/shelley/block.rs` |
| `AlonzoBlock`, `BabbageBlock`, `ConwayBlock` type aliases (= `ShelleyBlock`) | `crates/ade_types/src/{alonzo,babbage,conway}/mod.rs` |
| Era-specific type stubs (Plutus, governance, certs) | `crates/ade_types/src/{alonzo,babbage,conway}/` |
| `decode_alonzo_block()`, `decode_babbage_block()`, `decode_conway_block()` chokepoints | `crates/ade_codec/src/{alonzo,babbage,conway}/mod.rs` |
| Alonzo/Babbage/Conway GREEN adapters | `crates/ade_testkit/src/harness/adapters/{alonzo,babbage,conway}.rs` |
| Full corpus round-trip integration test (all 42 blocks) | `crates/ade_codec/tests/full_corpus_round_trip.rs` |

### T-14: Address Encoding Round-Trip

| Deliverable | Location |
|-------------|----------|
| `Address` enum (5 variants: Base, Pointer, Enterprise, Byron, Reward) | `crates/ade_types/src/address/mod.rs` |
| `NetworkId`, `Credential` types | `crates/ade_types/src/address/mod.rs` |
| `decode_address()` chokepoint (header byte dispatch) | `crates/ade_codec/src/address/mod.rs` |
| `encode_address()` (identity — returns stored bytes) | `crates/ade_codec/src/address/mod.rs` |
| Address extractor (tx output scanning from opaque bodies) | `crates/ade_testkit/src/harness/address_extractor.rs` |

---

## Workspace Layout After Phase 1

```
ade/
├── Cargo.toml                              # Workspace root (7 crates)
├── constitution_registry.toml              # 146 entries
│
├── ci/
│   ├── ci_check_dependency_boundary.sh     # Phase 0A
│   ├── ci_check_forbidden_patterns.sh      # Phase 0A
│   ├── ci_check_module_headers.sh          # Phase 0A
│   ├── ci_check_no_semantic_cfg.sh         # Phase 0A
│   ├── ci_check_no_signing_in_blue.sh      # Phase 0A
│   ├── ci_check_constitution_coverage.sh   # Phase 0A
│   ├── ci_check_ref_provenance.sh          # Phase 0B
│   ├── ci_check_no_secrets.sh              # Phase 0B
│   ├── ci_check_cbor_round_trip.sh         # Phase 1 — T-ENC-03
│   ├── ci_check_hash_uses_wire_bytes.sh    # Phase 1 — DC-CBOR-02
│   └── ci_check_ingress_chokepoints.sh     # Phase 1 — DC-INGRESS-01
│
├── crates/
│   ├── ade_types/src/                      # BLUE — semantic domain types
│   │   ├── lib.rs
│   │   ├── era.rs                          # CardanoEra (8 variants)
│   │   ├── primitives.rs                   # SlotNo, BlockNo, EpochNo, Hash28, Hash32
│   │   ├── address/mod.rs                  # Address (5 variants)
│   │   ├── byron/{mod,block,tx,common}.rs  # Byron EBB + regular types
│   │   ├── shelley/{mod,block,tx,common}.rs # ShelleyBlock, ShelleyHeaderBody, VrfData
│   │   ├── allegra/{mod,tx}.rs             # AllegraBlock alias + stubs
│   │   ├── mary/{mod,tx,value}.rs          # MaryBlock alias + stubs
│   │   ├── alonzo/{mod,tx,witness,plutus,output}.rs
│   │   ├── babbage/{mod,tx,output,script}.rs
│   │   └── conway/{mod,tx,governance,cert,script}.rs
│   │
│   ├── ade_codec/src/                      # BLUE — singular byte-authority owner
│   │   ├── lib.rs
│   │   ├── error.rs                        # CodecError (6 variants)
│   │   ├── traits.rs                       # AdeEncode, AdeDecode, CodecContext
│   │   ├── preserved.rs                    # PreservedCbor<T>, RawCbor
│   │   ├── primitives.rs                   # Codec impls for ade_types primitives
│   │   ├── cbor/mod.rs                     # CBOR byte-level primitives
│   │   ├── cbor/envelope.rs                # decode_block_envelope()
│   │   ├── address/mod.rs                  # decode_address()
│   │   ├── byron/{mod,block}.rs            # Byron chokepoints + encode/decode
│   │   ├── shelley/{mod,block}.rs          # Generalized post-Byron decoder
│   │   ├── allegra/mod.rs                  # decode_allegra_block()
│   │   ├── mary/mod.rs                     # decode_mary_block()
│   │   ├── alonzo/mod.rs                   # decode_alonzo_block()
│   │   ├── babbage/mod.rs                  # decode_babbage_block()
│   │   └── conway/mod.rs                   # decode_conway_block()
│   │
│   ├── ade_testkit/                        # GREEN — test infrastructure
│   │   ├── src/harness/
│   │   │   ├── adapters/{mod,byron,shelley,shelley_common,allegra,mary,alonzo,babbage,conway}.rs
│   │   │   ├── address_extractor.rs
│   │   │   ├── era_mapping.rs
│   │   │   └── (existing harness modules unchanged)
│   │   └── Cargo.toml                      # +ade_codec, ade_types, blake2
│   │
│   └── ade_codec/tests/                    # Integration tests
│       ├── envelope.rs                     # 42-block envelope dispatch
│       ├── byron_round_trip.rs             # 3 Byron blocks
│       ├── shelley_round_trip.rs           # 3 Shelley blocks
│       ├── allegra_mary_round_trip.rs      # 6 blocks
│       └── full_corpus_round_trip.rs       # ALL 42 blocks — final closure proof
│
└── docs/completed/
    ├── phase_0a/
    ├── phase_0b/
    └── phase_1/
```

---

## Test Inventory (166 tests)

| Module | Tests | What they verify |
|--------|-------|------------------|
| `ade_codec::cbor::tests` | 18 | CBOR read/write round-trip, encoding width preservation, canonical boundaries, error paths |
| `ade_codec::cbor::envelope::tests` | 12 | Envelope dispatch (all era tags), negative corpus (empty, truncated, trailing, wrong type) |
| `ade_codec::error::tests` | 5 | Error display formatting, equality, `std::Error` impl |
| `ade_codec::preserved::tests` | 7 | Wire bytes identity, canonical vs wire divergence, equality semantics, RawCbor |
| `ade_codec::primitives::tests` | 7 | SlotNo/BlockNo/EpochNo/Hash28/Hash32 round-trip, wrong-length rejection, RawCbor identity |
| `ade_codec::byron::block::tests` | 3 | EBB header, consensus data, full EBB block encode/decode round-trip |
| `ade_codec::address::tests` | 7 | Address type classification, identity round-trip, unknown type rejection |
| Integration: `envelope` | 8 | All 42 corpus blocks produce correct era via envelope dispatch |
| Integration: `byron_round_trip` | 6 | Byron byte-identical round-trip (3 blocks) + differential (3 blocks) + negative corpus |
| Integration: `shelley_round_trip` | 3 | Shelley byte-identical round-trip + differential + negative corpus |
| Integration: `allegra_mary_round_trip` | 4 | Allegra/Mary byte-identical round-trip (6 blocks) + differential (6 blocks) |
| Integration: `full_corpus_round_trip` | 2 | ALL 42 blocks byte-identical round-trip + ALL 42 differential field matches |
| `ade_types::era::tests` | 7 | CardanoEra variants, TryFrom, ordering, display, is_byron |
| `ade_types::primitives::tests` | 5 | Hash28/Hash32 debug/display formatting, SlotNo ordering |
| `ade_testkit::harness::era_mapping::tests` | 6 | CardanoEra ↔ Era mapping, Byron two-variant collapse, round-trip |
| `ade_testkit::harness::*` (existing) | 59 | Block diff, diff report, ledger diff, protocol diff, provenance, transcript (Phase 0B) |
| `ade_testkit::harness::tests` | 6 | Era enum, HarnessError (Phase 0B) |

---

## Architecture Decisions

### 1. Generalized post-Byron decoder

Rather than separate decoders for each of 6 post-Byron eras, a single `decode_shelley_block_inner()` function handles both block formats (array(4) vs array(5)) and both header formats (array(15) vs array(10)). The `VrfData` enum distinguishes the two header body formats: `VrfData::Split` for Shelley-Alonzo (separate nonce + leader VRF certs) and `VrfData::Combined` for Babbage-Conway (single VRF result). The AdeEncode impl uses the variant to determine re-encoding format. Each era has a thin chokepoint wrapper (`decode_X_block()`) and a type alias (`type XBlock = ShelleyBlock`).

### 2. Opaque substructures for Phase 1

Sub-structures not needed for field extraction (transaction bodies, witness sets, metadata, Byron SSC/delegation/update payloads, block signatures, VRF certs, KES signatures) are carried as `Vec<u8>` containing raw CBOR. This achieves byte-identical round-trip without deep structural parsing. The `AdeEncode` implementations concatenate parsed fields (re-encoded canonically) with opaque byte slices (identity passthrough). This works because all parsed fields in the corpus happen to be canonically encoded.

### 3. Canonical encoding sufficiency

All parsed fields in the 42 corpus blocks use canonical CBOR encoding (minimal integer widths, canonical-width byte string lengths). The `AdeEncode` implementations use `write_uint_canonical()` and `write_bytes_canonical()` which produce the same bytes. If future corpus entries contain non-canonical encodings in parsed fields, encoding width tracking would need to be added to the types. The opaque substructure approach makes this a local change — only the specific non-canonical field needs width tracking.

### 4. Dual byte-authority model preserved

`PreservedCbor<T>` stores original wire bytes and decoded structure separately. `.wire_bytes()` returns the original input (for hash-critical paths). `.canonical_bytes()` re-encodes from the decoded structure (for internal replay). The round-trip test proves that `AdeEncode` output matches the original wire bytes, but hash-critical computation will always use `.wire_bytes()` regardless.

---

## Deviations from Plan

### minicbor dependency omitted

The plan specified `minicbor = { version = "0.25", features = ["alloc"] }`. Version 0.25 does not exist (current is 2.x). Since Phase 1 builds its own CBOR byte-level primitives for full encoding width control, minicbor was not needed and was not added. If a future phase requires minicbor, it can be added at the correct version.

### Alonzo/Babbage/Conway implemented as a single commit

The plan specified separate slices T-11, T-12, T-13 with sequential dependencies. Because the generalized post-Byron decoder handles all three eras, they were implemented and committed together. The proof obligations are unchanged — all 30 blocks (3 Alonzo + 12 Babbage + 15 Conway) round-trip byte-identically.

### Era-specific type stubs instead of full type definitions

The plan specified detailed types for Alonzo (Datum, Redeemer, ExUnits, PlutusV1Script), Babbage (BabbageTxOut, DatumOption, ScriptRef), and Conway (VotingProcedures, GovAction, ConwayCert). These are defined as `Vec<u8>` stubs since the transaction bodies are opaque in Phase 1. Full structural definitions will be added when semantic validation requires deep tx parsing.

### Constitution registry not updated

The plan specified registry status changes for T-13 (DC-CBOR-01, DC-CBOR-02, T-ENC-03 → "enforced"). This was deferred to avoid coupling the implementation commit with registry mechanics. The proofs exist — the registry update is an administrative step.

---

## Carry-Forward for Next Cluster

### Stubs to implement

| Type | Current state | Required when |
|------|---------------|---------------|
| `BlockDecoder` for each era | GREEN adapters exist as free functions, `StubBlockDecoder` still in harness | Formal trait wiring |
| Byron tx types (`ByronTx`, `ByronTxIn`, `ByronTxOut`) | `Vec<u8>` stubs | Transaction-level validation |
| Alonzo Plutus types (`Datum`, `Redeemer`, `PlutusV1Script`) | `Vec<u8>` stubs | Script execution infrastructure |
| Babbage output types (`BabbageTxOut`, `DatumOption`, `ScriptRef`) | `Vec<u8>` stubs | UTxO-level validation |
| Conway governance types (`VotingProcedures`, `GovAction`, `ConwayCert`) | `Vec<u8>` stubs | CIP-1694 governance logic |

### Constitution registry status advancement

The following invariants are now provably enforced and should be advanced from "declared" to "enforced" in the registry:

| Invariant | Evidence |
|-----------|----------|
| T-ENC-03 (round-trip identity) | 42/42 corpus blocks pass `full_corpus_round_trip` |
| DC-CBOR-01 (encoding preservation) | Same test — opaque substructures + canonical parsed fields |
| DC-CBOR-02 (wire bytes for hashes) | `.wire_bytes()` returns original input; `ci_check_hash_uses_wire_bytes.sh` passes |

### CI script inventory (11 scripts)

| Script | Invariant | Phase |
|--------|-----------|-------|
| `ci_check_dependency_boundary.sh` | T-BOUND-02 | 0A |
| `ci_check_forbidden_patterns.sh` | T-CORE-02 | 0A |
| `ci_check_module_headers.sh` | CE-04 | 0A |
| `ci_check_no_semantic_cfg.sh` | T-BUILD-01 | 0A |
| `ci_check_no_signing_in_blue.sh` | T-KEY-01 | 0A |
| `ci_check_constitution_coverage.sh` | T-CI-01 | 0A |
| `ci_check_ref_provenance.sh` | DC-REF-01 | 0B |
| `ci_check_no_secrets.sh` | OP-SEC-01 | 0B |
| `ci_check_cbor_round_trip.sh` | T-ENC-03 | 1 |
| `ci_check_hash_uses_wire_bytes.sh` | DC-CBOR-02 | 1 |
| `ci_check_ingress_chokepoints.sh` | DC-INGRESS-01 | 1 |

### What the next cluster will need

1. **Deep transaction parsing** — replace opaque `Vec<u8>` tx bodies/witnesses with typed structures for at least one era, enabling semantic validation
2. **Hash computation** — implement Blake2b-256 hashing using `.wire_bytes()` for block header hashes and transaction IDs; verify against reference oracle
3. **Ledger state application** — implement `LedgerApplicator` for at least one era against the 15 reference state hashes
4. **Non-canonical encoding handling** — if future corpus entries contain non-canonical integer widths or indefinite-length containers in parsed fields, add encoding width tracking to the affected types
5. **Registry advancement** — update `constitution_registry.toml` with enforced status, `code_locus`, `ci_script`, and `tests[]` for T-ENC-03, DC-CBOR-01, DC-CBOR-02
