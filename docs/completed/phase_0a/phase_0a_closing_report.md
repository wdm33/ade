# Phase 0A Closing Report — Scaffold, Registry & Corpus Plumbing

**Cluster**: Phase 0A
**Status**: Complete
**Date completed**: 2026-03-15
**Commits**: 5 (4d1d3cf through 7b5b7ca)

---

## Summary

Phase 0A established the foundational infrastructure for the Ade project: a compilable workspace with mechanically enforced purity boundaries, a constitution registry tracking 146 invariants across 5 families, and a 42-block mainnet test corpus covering all 7 Cardano eras. No domain types, external behavior, or runtime logic were introduced.

---

## Exit Criteria Verification

All 12 cluster exit criteria passed on 2026-03-15:

| CE | Description | Result |
|----|-------------|--------|
| CE-01 | `cargo build --workspace` | PASS |
| CE-02 | `ci_check_dependency_boundary.sh` (BLUE never depends on RED) | PASS |
| CE-03 | `ci_check_forbidden_patterns.sh` (no nondeterministic APIs in BLUE) | PASS |
| CE-04 | `ci_check_module_headers.sh` (contract headers in BLUE) | PASS |
| CE-05 | `ci_check_no_signing_in_blue.sh` (signing confined to RED) | PASS |
| CE-06 | `ci_check_no_semantic_cfg.sh` (no feature flags in BLUE) | PASS |
| CE-07 | Constitution registry coverage (146 entries, mechanically validated) | PASS |
| CE-08 | `ci_check_constitution_coverage.sh` (schema, tiers, cross-refs) | PASS |
| CE-09 | At least 1 mainnet block per era (42 blocks, all 7 eras) | PASS |
| CE-10 | `verify_checksums.sh` (SHA-256 integrity for all fixtures) | PASS |
| CE-11 | All CI scripts executable | PASS |
| CE-12 | `cargo test` + `cargo clippy` clean | PASS |

---

## What Was Delivered

### T-01: Workspace Scaffold

**Commit**: `4d1d3cf feat: enforce BLUE/RED purity boundary with deny attrs, clippy rules, and CI scripts`

| Deliverable | Location |
|-------------|----------|
| Disallowed types for determinism | `clippy.toml` |
| BLUE crate deny attributes | `crates/ade_{codec,types,crypto,core}/src/lib.rs` |
| GREEN/RED contract headers | `crates/ade_{testkit,runtime}/src/lib.rs`, `crates/ade_node/src/main.rs` |
| Dependency boundary CI | `ci/ci_check_dependency_boundary.sh` |
| Forbidden patterns CI | `ci/ci_check_forbidden_patterns.sh` |
| Module headers CI | `ci/ci_check_module_headers.sh` |
| No semantic cfg CI | `ci/ci_check_no_semantic_cfg.sh` |
| No signing in BLUE CI | `ci/ci_check_no_signing_in_blue.sh` |

**BLUE crate deny set** (applied to ade_codec, ade_types, ade_crypto, ade_core):
```rust
#![deny(unsafe_code)]
#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![deny(clippy::float_arithmetic)]
```

**GREEN/RED deny set** (ade_testkit, ade_runtime, ade_node):
```rust
#![deny(unsafe_code)]
```

### T-02: Constitution Registry Bootstrap

**Commit**: `4af8f75 feat: add constitution registry with declared invariants and coverage CI`

| Deliverable | Location |
|-------------|----------|
| Constitution registry | `constitution_registry.toml` (146 entries) |
| Coverage validation CI | `ci/ci_check_constitution_coverage.sh` |

**Registry entry counts**:

| Family | Count | Tier Rule |
|--------|-------|-----------|
| T-* | 29 | Always `"true"` |
| DC-* | 35 | Always `"derived"` |
| CN-* | 69 | Per classification_table.md (spans all 4 tiers) |
| RO-* | 6 | Always `"release"` |
| OP-* | 7 | Always `"operational"` |
| **Total** | **146** | |

**CN-* tier breakdown**: 31 true, 25 derived, 6 release, 7 operational.

All entries have `status = "declared"`. No enforcement claims.

**Registry schema** (for future slices updating entries):
- true/derived entries: `id`, `tier`, `statement`, `source`, `cross_ref`, `code_locus`, `tests`, `ci_script`, `status`
- CN-* true/derived additionally: `attack_rationale`, `evidence_notes`
- release/operational entries: `id`, `tier`, `statement`, `source`, `cross_ref`, `status` (no `code_locus`/`tests`/`ci_script`)

**CI validation** (`ci_check_constitution_coverage.sh`) checks:
- TOML syntax, unique IDs, valid status
- Tier-to-prefix rules (T→true, DC→derived, RO→release, OP→operational)
- CN-* tiers validated against `classification_table.md` (not by prefix)
- Cross-ref bidirectionality
- Tier-appropriate field presence/absence
- Enforcement regression guard (enforced entries must have non-empty code_locus/tests/ci_script)
- Mechanical count validation against source documents (no hardcoded counts)

**Source document dependency**: The CI script reads `ade_replay_first_constitutional_node_plan_v1.md` (§2-§4b) and `classification_table.md` from `~/Documents/ade-planning/`. These files must remain accessible for the CI script to perform coverage validation. Future clusters should consider copying relevant ID lists into the repo if the external dependency becomes problematic.

### T-03: Corpus Ingestion Plumbing

**Commits**:
- `39e0095 feat: add corpus directory structure with manifest schema and scripts`
- `1585446 feat: add golden mainnet CBOR blocks for 5 Cardano eras`
- `7b5b7ca feat: add golden mainnet CBOR blocks for Allegra and Mary eras`

| Deliverable | Location |
|-------------|----------|
| Corpus README | `corpus/README.md` |
| Acquisition status script | `corpus/acquire_fixtures.sh` |
| Checksum verification | `corpus/verify_checksums.sh` |
| 7 era manifest files | `corpus/golden/{era}/manifest.toml` |
| 42 mainnet CBOR blocks | `corpus/golden/{era}/blocks/*.cbor` |

**Corpus inventory**:

| Era | Blocks | Chunks | Era Tag | Size |
|-----|--------|--------|---------|------|
| Byron | 3 | 0 | 0, 1 | 648K |
| Shelley | 3 | 500 | 2 | 16K |
| Allegra | 3 | 900 | 3 | 12K |
| Mary | 3 | 1400 | 4 | 40K |
| Alonzo | 3 | 3000 | 5 | 148K |
| Babbage | 12 | 4000, 4700, 5000, 5500 | 6 | 620K |
| Conway | 15 | 6500, 7500, 8100, 8400, 8419 | 7 | 380K |
| **Total** | **42** | | | **~1.8 MB** |

**Provenance**: All blocks extracted from cardano-node 10.6.2 ImmutableDB (git rev `0d697f14`), obtained via Mithril snapshot (epoch 618, immutable file 8419, mithril-client 0.12.38). Blocks read from chunk files using ImmutableDB secondary index (56-byte entries), transferred via SCP from EC2.

**File naming**: `chunk{NNNNN}_blk{NNNNN}.cbor` — chunk number and block index within that chunk.

**Manifest schema** (per fixture):
```toml
file, era, type, chunk, block_index, era_tag, sha256, source, fetch_tool, fetch_date, reproducibility
```

---

## Workspace Layout After Phase 0A

```
ade/
├── Cargo.toml                          # Workspace root (7 crates)
├── CLAUDE.md
├── clippy.toml                         # Disallowed types for BLUE purity
├── constitution_registry.toml          # 146 invariant entries
├── rustfmt.toml
│
├── ci/
│   ├── ci_check_dependency_boundary.sh # T-BOUND-02
│   ├── ci_check_forbidden_patterns.sh  # T-CORE-02
│   ├── ci_check_module_headers.sh      # 01_core §14
│   ├── ci_check_no_semantic_cfg.sh     # T-BUILD-01
│   ├── ci_check_no_signing_in_blue.sh  # T-KEY-01
│   └── ci_check_constitution_coverage.sh # T-CI-01
│
├── corpus/
│   ├── README.md
│   ├── acquire_fixtures.sh
│   ├── verify_checksums.sh
│   └── golden/
│       ├── {byron,shelley,allegra,mary,alonzo,babbage,conway}/
│       │   ├── manifest.toml
│       │   ├── blocks/*.cbor
│       │   └── transactions/           # scaffolded, empty
│
├── crates/
│   ├── ade_codec/src/lib.rs            # BLUE — empty skeleton + deny attrs
│   ├── ade_types/src/lib.rs            # BLUE — empty skeleton + deny attrs
│   ├── ade_crypto/src/lib.rs           # BLUE — empty skeleton + deny attrs
│   ├── ade_core/src/lib.rs             # BLUE — empty skeleton + deny attrs (PROVISIONAL)
│   ├── ade_testkit/src/lib.rs          # GREEN — empty skeleton + deny(unsafe_code)
│   ├── ade_runtime/src/lib.rs          # RED — empty skeleton + deny(unsafe_code)
│   └── ade_node/src/main.rs            # RED — println!("ade node") + deny(unsafe_code)
│
└── docs/
    └── completed/
        └── phase_0a/
```

**No types, traits, functions, or modules were created in any crate.** All lib.rs files contain only the contract header comment and deny attributes. main.rs contains only the placeholder `println!`.

**No inter-crate dependencies exist.** Each crate's Cargo.toml has only `[package]` — no `[dependencies]`.

**No third-party dependencies.** The entire workspace depends only on Rust std.

---

## Deviations from Plan

### T-03: Blockfrost → ImmutableDB extraction

The original plan specified Blockfrost API as the primary acquisition source. During implementation, blocks were instead extracted directly from a cardano-node 10.6.2 ImmutableDB obtained via Mithril snapshot.

**Why**: Direct ImmutableDB extraction provides strictly better provenance — bytes are identical to what cardano-node stores and reads, with no API normalization, hex conversion, or rate-limit dependency.

**Impact on manifest schema**: The planned `height` and `hash` fields were replaced with `chunk`, `block_index`, and `era_tag`, which precisely identify extraction coordinates in the ImmutableDB. The `verify_checksums.sh` required fields were updated accordingly.

### T-03: 42 blocks instead of 7

The plan required "at least one real mainnet block per era." Implementation delivered 42 blocks (3-15 per era) sampled from multiple ImmutableDB chunks, providing broader coverage per era.

### T-03: corpus.rs not added

The optional `ade_testkit/src/corpus.rs` path resolution helper was deferred — no consumer exists yet. It should be added when the first slice needs to load fixtures from Rust test code.

---

## Lessons Learned

### 1. CI scripts that grep comments need careful regex

The `ci_check_forbidden_patterns.sh` script initially false-positive'd on the contract header comment (`// - No wall-clock time, true randomness, HashMap/HashSet, or floats`) because the comment line matched the `HashMap`/`HashSet` patterns. The fix required matching the grep output format (which includes `path:lineno:` prefix) rather than just `^\s*//`.

**Guidance for future CI scripts**: When grepping source files and excluding comments, account for the `grep -rn` output format where the line content appears after `filename:lineno:`.

### 2. CN-* tier validation requires table body parsing, not summary counts

The classification_table.md summary header states 33 true-tier CN-* entries, but the table body contains 31. The CI script correctly parses the table body (ground truth) rather than trusting summary counts. The `extract_ids_from_first_column` function uses `re.fullmatch` on only the first data column to avoid matching IDs that appear in cross-reference columns.

**Guidance for future CI scripts**: Always derive counts mechanically from the authoritative table body, never from summary prose.

### 3. ImmutableDB chunk numbers are not epoch numbers

ImmutableDB chunk numbering is sequential but does not map 1:1 to epoch numbers after Byron. Byron uses one chunk per epoch (~21,600 blocks per chunk). Post-Byron eras use ~20 chunks per epoch (~500-1,100 blocks per chunk). The CBOR era tag byte (second byte of the `[era_tag, block_body]` envelope) is the authoritative era identifier.

**Era tag mapping** (critical for Phase 0B/1 codec work):

| Byte | Era |
|------|-----|
| 0x00 | Byron EBB (Epoch Boundary Block) |
| 0x01 | Byron regular block |
| 0x02 | Shelley |
| 0x03 | Allegra |
| 0x04 | Mary |
| 0x05 | Alonzo |
| 0x06 | Babbage |
| 0x07 | Conway |

Note: Byron has two era tags (0 for EBBs, 1 for regular blocks). All other eras have one tag each. The first byte of every block is `0x82` (CBOR 2-element array).

### 4. Constitution coverage CI depends on external files

`ci_check_constitution_coverage.sh` reads source documents from `~/Documents/ade-planning/` to mechanically derive expected invariant IDs. This external dependency works for local development but will break in CI environments that don't have those files. Future work should either:
- Copy the relevant ID lists into the repo (e.g., as a manifest)
- Make the source-document validation optional with a clear warning
- Accept the coupling as intentional (planning docs are the source of truth)

### 5. Contract header comment triggers forbidden pattern checks

The contract header includes the text "HashMap/HashSet" as a reminder of what's forbidden. If future BLUE crate source files include similar documentation comments mentioning forbidden patterns, the CI script will correctly skip them (it filters lines matching `:[0-9]*:\s*//`). However, doc comments (`///` or `//!`) in non-comment-only lines could potentially false-positive. The current codebase has no doc comments so this hasn't been tested.

---

## Carry-Forward for Next Cluster

### Hard merge guards still active

1. **ade_core extraction rule**: The moment a slice introduces invariants from a second distinct invariant family into `ade_core`, the existing family must be extracted into its own crate. Families: DC-LEDGER-*, DC-CONSENSUS-*, DC-PROTO-*, DC-PLUTUS-*. This is a merge-blocking gate.

2. **All 146 registry entries remain `status = "declared"`**. Future slices that enforce an invariant must update the corresponding registry entry to `"partial"` or `"enforced"` with non-empty `code_locus`, `tests`, and `ci_script`.

### What the next cluster will need

**Phase 0B (Truth Capture & Differential Harnesses)** will need to:

1. **Read corpus fixtures from Rust code** — add `ade_testkit/src/corpus.rs` or equivalent for path resolution and manifest parsing. This will likely require a TOML parsing dependency in ade_testkit (GREEN crate — allowed).

2. **Decode CBOR blocks** — Phase 1 work, but Phase 0B differential harnesses will need at minimum to distinguish block boundaries. The corpus blocks use the Cardano `[era_tag, block_body]` envelope.

3. **Extract Haskell reference outputs** — DC-REF-01 requires documented reference source, extraction method, and reproducibility path. The corpus provides the input blocks; Phase 0B must capture corresponding Haskell node outputs (decoded fields, state hashes, validity verdicts).

4. **Protocol transcript capture** — T-06 (deferred from Phase 0A) creates the `corpus/golden/{era}/protocol_transcripts/` directories and captures real ChainSync/BlockFetch message exchanges.

### No types, traits, or functions to inherit

Phase 0A created no domain types, no traits, no functions, and no modules beyond the skeleton entry points. The next cluster starts from a clean slate with only:
- Contract headers and deny attributes in each crate
- `clippy.toml` disallowed types
- 6 CI enforcement scripts
- Constitution registry (tracking artifact)
- 42 golden CBOR blocks with provenance

### CI script inventory

| Script | Invariant | What it checks |
|--------|-----------|----------------|
| `ci_check_dependency_boundary.sh` | T-BOUND-02 | BLUE crates never depend on RED crates (via `cargo metadata`) |
| `ci_check_forbidden_patterns.sh` | T-CORE-02 | No HashMap, HashSet, SystemTime, Instant, std::fs, std::net, tokio, async fn, f32, f64, anyhow, rand::thread_rng, thread::spawn in BLUE src/ |
| `ci_check_module_headers.sh` | 01_core §14 | First line of every .rs in BLUE crates is `// Core Contract:` |
| `ci_check_no_semantic_cfg.sh` | T-BUILD-01 | No `#[cfg(feature` or `cfg!(feature` in BLUE src/ |
| `ci_check_no_signing_in_blue.sh` | T-KEY-01 | No SigningKey, SecretKey, PrivateKey, private_key, sign_message, sign_block in BLUE src/ |
| `ci_check_constitution_coverage.sh` | T-CI-01 | Registry schema, coverage, tiers, cross-refs, enforcement regression |
| `corpus/verify_checksums.sh` | CN-META-03 | SHA-256 integrity for all corpus fixtures |
