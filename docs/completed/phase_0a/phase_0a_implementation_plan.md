# Phase 0A Implementation Plan — Scaffold, Registry & Corpus Plumbing

## Context

This plan implements Phase 0A of the Ade project (Cardano block-producing node in Rust), responding to Pi Lanningham's $5k USDC challenge. Phase 0A establishes foundational infrastructure: workspace purity boundaries, invariant tracking, and a real mainnet test corpus. It produces no external behavior — pure internal scaffolding.

The bounty is a **certification target**, not the constitution. Constitutional invariants govern; bounty checks are a subset. No bounty criterion may weaken a true invariant.

The implementation follows 3 slices defined in `/home/ts/Documents/ade-planning/clusters/phase_0a_scaffold_registry_corpus/`:
- **T-01**: Workspace Scaffold (contract headers, deny attributes, clippy rules, 5 CI scripts)
- **T-02**: Constitution Registry Bootstrap (constitution_registry.toml + CI validation script)
- **T-03**: Corpus Ingestion Plumbing (raw bytes + provenance for block fixtures across all 7 Cardano eras; protocol transcripts deferred to Phase 0B/T-06)

T-02 and T-03 are independent and may proceed in parallel after T-01.

---

## Slice T-01: Workspace Scaffold

### Files to modify

**1. `clippy.toml`** — Add disallowed types for determinism enforcement:
```toml
msrv = "1.75"

disallowed-types = [
    { path = "std::collections::HashMap", reason = "Non-deterministic iteration order. Use BTreeMap." },
    { path = "std::collections::HashSet", reason = "Non-deterministic iteration order. Use BTreeSet." },
    { path = "std::time::SystemTime", reason = "Non-deterministic. Use LogicalTimestamp." },
    { path = "std::time::Instant", reason = "Non-deterministic. Use LogicalTimestamp." },
    { path = "anyhow::Error", reason = "Unstructured errors. Use structured error types." },
]
```

**2. BLUE crate lib.rs files** (ade_codec, ade_types, ade_crypto, ade_core) — Each gets:
```rust
// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types
// - Explicit state transitions only
// - Canonical serialization for all persisted/hashed data

#![deny(unsafe_code)]
#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![deny(clippy::float_arithmetic)]
```

**Note on `ade_core`**: This crate is a **provisional temporary home**. Per the mandatory extraction rule (plan §5), the moment a slice introduces invariants from a second distinct invariant family (e.g., ledger + consensus, or ledger + protocol), the existing family must be extracted into its own crate before the new code lands. This is a **hard merge guard** for all future slices, not a soft recommendation. CI or review must enforce it.

**3. GREEN crate lib.rs** (ade_testkit) — Contract header + `#![deny(unsafe_code)]` only.

**4. RED crate files** (ade_runtime/lib.rs, ade_node/main.rs) — Contract header + `#![deny(unsafe_code)]` only. main.rs keeps `fn main() { println!("ade node"); }`.

### Files to create

**5. `ci/ci_check_dependency_boundary.sh`** — Parse BLUE crate Cargo.toml files, fail if any references a RED crate. Uses `cargo metadata --no-deps` for resolved dependency tree verification.

**6. `ci/ci_check_forbidden_patterns.sh`** — Grep BLUE crate `src/` dirs for: `HashMap`, `HashSet`, `SystemTime`, `Instant`, `std::fs`, `std::net`, `tokio`, `async fn`, `f32`, `f64`, `anyhow`, `rand::thread_rng`, `thread::spawn`. Exclude comment lines and deny attribute lines.

**7. `ci/ci_check_module_headers.sh`** — Verify contract header present as first line of every `.rs` source file in **BLUE crates only** (ade_codec, ade_types, ade_crypto, ade_core). GREEN (ade_testkit) and RED (ade_runtime, ade_node) crates are outside CE-04 scope. Contract headers on GREEN/RED files are optional good practice, not CI-enforced.

**8. `ci/ci_check_no_semantic_cfg.sh`** — Grep BLUE crate `src/` for `#[cfg(feature` and `cfg!(feature`.

**9. `ci/ci_check_no_signing_in_blue.sh`** — Grep BLUE crate `src/` for signing/private-key patterns (`SigningKey`, `SecretKey`, `PrivateKey`, `private_key`, `sign_message`, `sign_block`). Maintains verification allowlist (verification is permitted in BLUE; signing is not). Named per CE-05.

All scripts: bash, `chmod +x`, exit 0/1, `set -euo pipefail`.

### T-01 envelope constraints

T-01 adds **only** contract headers, deny attributes, clippy.toml updates, and CI boundary scripts. Nothing else. Per the slice's hard prohibitions:

- No domain types (block, transaction, header, ledger state, UTxO, etc.)
- No third-party dependencies (all crates depend only on Rust std)
- No tests (scaffold only — tests arrive with domain logic)
- No subdirectories in crate `src/` directories (flat for skeletons)
- No inter-crate dependencies in BLUE skeletons
- No runtime behavior in any crate (all lib.rs/main.rs are skeletons with headers only)
- No convenience scaffolding, extra abstractions, or preparatory refactors

If anything beyond headers, deny attributes, clippy.toml, and CI scripts appears, the slice is incorrect.

### Verification
- `cargo build --workspace` — zero errors, zero warnings
- `cargo clippy --workspace --all-targets -- -D warnings` — passes
- `cargo test --workspace` — passes (trivially)
- All 5 CI scripts exit 0

---

## Slice T-02: Constitution Registry Bootstrap

### Authority posture

T-02 is a **coverage-and-schema slice**. It establishes complete coverage and well-formed schema. All entries have `status = "declared"`. It does not claim any invariant is enforced.

**No hardcoded invariant counts.** The CI script must mechanically derive expected IDs from the source documents (plan §2-§4b, classification_table.md) and compare against registry contents. Counts stated in this plan are for orientation only — the CI script is authoritative, not this prose.

### File to create

**`constitution_registry.toml`** — TOML registry with `[[rules]]` entries for all registry-covered families (T/DC/CN/RO/OP):

| Family | Source | Tier Rule |
|--------|--------|-----------|
| T-* | Plan §2 (project constitution) | Always `"true"` |
| DC-* | Plan §3 (derived invariants) | Always `"derived"` |
| CN-* | `classification_table.md` | **Per classification table, NOT prefix** |
| RO-* | Plan §4a (release obligations) | Always `"release"` |
| OP-* | Plan §4b (operational invariants) | Always `"operational"` |

BA-* entries (bounty acceptance checks) are **not** registry-covered. CE-07 explicitly scopes T-02 coverage to T/DC/CN/RO/OP families. Bounty checks are certification targets that trace to derived/true invariants — they are not invariants themselves and do not appear in the registry.

### Tier-to-ID validation (critical)

- T-* → `"true"`, DC-* → `"derived"`, RO-* → `"release"`, OP-* → `"operational"` — these are prefix-determined.
- **CN-* entries span all four tiers.** CN-* tier must be validated against `classification_table.md`, not the prefix. Examples:
  - CN-BUILD-01 is `"true"` (not derived)
  - CN-BUILD-04 is `"derived"` (not true)
  - CN-TEST-01 is `"release"`
  - CN-OPS-01 is `"operational"`

### Tier-separated schemas

**Full schema for true/derived entries (T-*, DC-*, CN-* where tier is true or derived):**
```toml
[[rules]]
id = "T-DET-01"
tier = "true"
statement = "Same canonical inputs -> same authoritative bytes (per Byte Authority Model)"
source = "Project constitution S2, Byte Authority Model S3"
cross_ref = []
code_locus = ""
tests = []
ci_script = ""
status = "declared"
```

**CN-* true/derived entries additionally carry:**
```toml
attack_rationale = "..."   # from classification_table.md "Why It Exists" column
evidence_notes = "..."     # from classification_table.md "Evidence Note" column
```

**Reduced schema for release/operational entries (RO-*, OP-*, CN-* where tier is release or operational):**
```toml
[[rules]]
id = "RO-TEST-01"
tier = "release"
statement = "..."
source = "..."
cross_ref = []
status = "declared"
```
**Must NOT carry** `code_locus`, `tests`, or `ci_script` — release/operational entries have different enforcement expectations. Presence of these fields on a release/operational entry is a schema violation.

### Anti-flattening rule (hard prohibition)

`cross_ref` links overlapping entries but **never substitutes for enforcement ownership**:
- A CN-* entry's "enforced" status does not propagate to the linked T-*/DC-* entry (or vice versa).
- Each entry's enforcement is tracked independently.
- A T-* entry may be "enforced" while its overlapping CN-* entry is "declared" — they are independent enforcement paths.
- No cross-ref may substitute for independent enforcement tracking.

**Violation of the anti-flattening rule makes the registry incorrect.**

### Bidirectional cross-ref requirement

Where overlaps exist (e.g., CN-BUILD-01 overlaps T-BUILD-01, CN-META-01 overlaps T-CI-01), `cross_ref` links must be present on **both** sides. The CI script validates bidirectionality.

### Status constraint

Every entry in Phase 0A: `status = "declared"`. No entry may be `"partial"` or `"enforced"`. The registry creates tracking demands — it does not claim enforcement is satisfied.

### CI script to create

**`ci/ci_check_constitution_coverage.sh`** — Uses `python3` with `tomllib` (Python 3.11+) to validate:
1. Valid TOML syntax
2. Every entry has all tier-appropriate required fields (full schema for true/derived, reduced schema for release/operational)
3. Release/operational entries do NOT carry `code_locus`/`tests`/`ci_script`
4. Tier-to-ID prefix validation for T-*/DC-*/RO-*/OP-*
5. **CN-* tier validation against classification_table.md** — not by prefix
6. All statuses are `"declared"`
7. No duplicate IDs
8. Enforcement regression guard: any future entry marked `"enforced"` must have non-empty `code_locus`, `tests`, `ci_script` (not triggered in Phase 0A)
9. Cross-ref bidirectionality: if A lists B in cross_ref, B must list A
10. CN-* true/derived entries have `attack_rationale` and `evidence_notes`
11. **Mechanical count validation**: CI extracts expected IDs from source documents and compares against registry — no hardcoded counts in the script

### Verification
- `ci/ci_check_constitution_coverage.sh` exits 0
- All T-01 CI scripts still pass (no regression)
- No Rust code changes
- No entry claims enforcement

---

## Slice T-03: Corpus Ingestion Plumbing

### Governing constraint: raw bytes + provenance ONLY

T-03 is ruthlessly non-semantic. The corpus stores **opaque CBOR as received on the wire** and **provenance metadata**. Nothing else.

**Hard prohibitions for T-03:**
- No CBOR decoding or parsing of fixture content
- No Haskell-derived semantic interpretations (validity verdicts, decoded headers, state hashes)
- No domain types for blocks, transactions, or headers
- No re-encoding of fixture data — bytes stored as received
- No protocol transcript capture or `protocol_transcripts/` directory (deferred to T-06/Phase 0B)
- No BLUE crate modifications of any kind
- No placeholder or "TODO" values in manifest fields
- ManifestEntry (if in ade_testkit) contains only provenance metadata — zero domain semantics

The acquisition script fetches and stores bytes. The verification script checks checksums. Neither interprets content.

### Directory structure to create

```
corpus/
├── README.md
├── acquire_fixtures.sh
├── verify_checksums.sh
└── golden/
    ├── byron/
    │   ├── manifest.toml
    │   ├── blocks/
    │   │   └── {descriptive_name}.cbor
    │   └── transactions/
    ├── shelley/
    │   ├── manifest.toml
    │   ├── blocks/
    │   └── transactions/
    ├── allegra/   (same structure)
    ├── mary/      (same structure)
    ├── alonzo/    (same structure)
    ├── babbage/   (same structure)
    └── conway/    (same structure)
```

Transaction directories are scaffolded for all eras; population is deferred.

### Manifest schema (one manifest.toml per era)

```toml
[[fixtures]]
file = "blocks/mainnet_block_epoch_0_slot_1.cbor"
era = "byron"
type = "block"
height = 1
hash = "abc123..."
sha256 = "def456..."
source = "Blockfrost mainnet API v0"
fetch_tool = "curl 8.5.0"
fetch_date = "2026-03-14"
reproducibility = "curl -s -H 'project_id: ...' https://..."
```

Every field mandatory. No placeholders. No TODOs. The `source` field must identify exact API version or tool version. The `reproducibility` field must contain executable instructions.

### Acquisition strategy

**Primary: Blockfrost API** (free tier, 50k requests/day)
- `acquire_fixtures.sh` requires `BLOCKFROST_PROJECT_ID` env var
- For each era: fetch block by known height -> get CBOR hex -> convert to binary -> compute SHA-256 -> write manifest entry
- Script does NOT decode, parse, normalize, or interpret the CBOR in any way
- Idempotent: skips existing files

**Known era boundary blocks (approximate heights, for reference):**
| Era | Approx Block Height | Epoch |
|-----|-------------------|-------|
| Byron | 1 | 0 |
| Shelley | 4,490,511 | 208 |
| Allegra | 5,406,749 | 236 |
| Mary | 5,822,757 | 251 |
| Alonzo | 6,236,063 | 290 |
| Babbage | 7,791,698 | 365 |
| Conway | 10,847,962 | 503 |

### Scripts to create

**`corpus/acquire_fixtures.sh`** — Fetches one golden block per era via Blockfrost. Stores raw bytes. Populates manifest.toml entries with full provenance. Does not interpret content.

**`corpus/verify_checksums.sh`** — Reads all `manifest.toml` files, verifies every referenced file exists and SHA-256 matches. Uses `python3` with `tomllib` for reliable TOML parsing. Exits 0 on all pass, 1 on any mismatch/missing.

**`corpus/README.md`** — Documents layout, provenance model, acquisition process, manifest schema.

### Verification
- At least one real mainnet `.cbor` block per era
- `verify_checksums.sh` exits 0 on valid data
- `verify_checksums.sh` exits 1 on intentionally corrupted fixture
- All manifest entries have all required fields with real values
- No BLUE crate modifications
- No CBOR decoding, parsing, or domain type introduction
- No protocol transcript fixtures or directories
- All T-01 CI scripts still pass

---

## Commit Strategy

| # | Slice | Message | Files |
|---|-------|---------|-------|
| 1 | T-01 | `feat: enforce BLUE/RED purity boundary with deny attrs, clippy rules, and CI scripts` | clippy.toml, 7 lib.rs/main.rs, 5 CI scripts |
| 2 | T-02 | `feat: add constitution registry with declared invariants and coverage CI` | constitution_registry.toml, 1 CI script |
| 3 | T-03 (structure) | `feat: add corpus directory structure with manifest schema and acquisition scripts` | corpus/README.md, 7 manifest.toml stubs, acquire_fixtures.sh, verify_checksums.sh |
| 4 | T-03 (data) | `feat: add golden mainnet CBOR blocks for all 7 Cardano eras` | 7 .cbor files, populated manifest.toml files |

All commits include `Co-Authored-By: Claude` trailers per project CLAUDE.md.

---

## Merge Guards and Downstream Obligations

### ade_core extraction rule (HARD MERGE GUARD)

`ade_core` is provisional. The mandatory extraction rule (plan §5) states: **the first cluster that introduces a second distinct invariant family must force extraction into separate BLUE crates before the new code lands.**

Concretely: ledger rules (DC-LEDGER-*), consensus logic (DC-CONSENSUS-*), protocol state machines (DC-PROTO-*), and Plutus evaluation (DC-PLUTUS-*) each have their own invariant families. The moment a slice introduces invariants from a second family into `ade_core`, the existing family must be extracted (e.g., into `ade_ledger`).

This prevents:
- Invariant families blurring across module boundaries without crate-level enforcement
- CI scripts that cannot distinguish invariant family boundaries
- Late, expensive extraction after coupling has formed

**This is a merge-blocking gate, not an advisory note. Future slices that violate it must be rejected.**

### Downstream proof obligations from constitution and audit

Phase 0A declares these invariants but does NOT enforce them. They must remain visible as explicit future obligations:

| ID | Obligation | Phase |
|----|-----------|-------|
| DC-PROTO-03 | Full N2N mini-protocol surface: Handshake, ChainSync, BlockFetch, TxSubmission2, KeepAlive, PeerSharing | Phase 4 |
| DC-PROTO-04 | Full N2C mini-protocol surface: Handshake, LocalChainSync, LocalTxSubmission, LocalStateQuery, LocalTxMonitor | Phase 4 |
| DC-PROTO-05 | Version negotiation: closed enumerated versions, explicit handshake, deterministic refusal on mismatch | Phase 4 |
| DC-EPOCH-01 | Conway governance timing: proposals accumulate during epoch, ratification/enactment atomic at boundary, pulsing for DRep stake | Phase 2/4 |
| DC-EPOCH-02 | Hard fork transitions at deterministic slot/epoch boundaries; era translation mandatory; forecast horizon extends to era boundary | Phase 4 |
| DC-STORE-04 | ChainDB structure: ImmutableDB (append-only, k-deep), VolatileDB (recent), LedgerDB (snapshots + forward replay) | Phase 5 |
| DC-STORE-05 | Recovery is snapshot + forward replay, not full genesis replay | Phase 5 |
| DC-STORE-06 | VolatileDB `ValidateAll` after unclean shutdown; `NoValidation` acceptable during clean operation | Phase 5 |

These are not Phase 0A work, but they must not disappear from the registry or planning horizon.

---

## Critical Files

| File | Role |
|------|------|
| `/home/ts/Code/rust/ade/clippy.toml` | Central lint config — determinism guardrails |
| `/home/ts/Code/rust/ade/crates/ade_codec/src/lib.rs` | Representative BLUE crate (pattern for all 4) |
| `/home/ts/Code/rust/ade/constitution_registry.toml` | Invariant registry — most labor-intensive deliverable |
| `/home/ts/Code/rust/ade/ci/ci_check_constitution_coverage.sh` | Most complex CI script (TOML parsing + tier validation) |
| `/home/ts/Code/rust/ade/corpus/acquire_fixtures.sh` | Mechanism for fetching real mainnet CBOR blocks |

## Key Decisions

1. **BA-* excluded from registry** — T-02 acceptance criteria (CE-07) explicitly lists T/DC/CN/RO/OP only
2. **`[[rules]]` not `[[invariants]]`** — per T-02 schema specification and project plan §6
3. **Corpus under `corpus/golden/`** — per T-03 directory structure specification
4. **One manifest.toml per era** — not per subdirectory
5. **Blockfrost API** — primary acquisition source (free tier, reliable, raw CBOR endpoint)
6. **Optional corpus.rs deferred** — no third-party TOML dependency in testkit yet
7. **No `float_arithmetic` deny on GREEN/RED** — only BLUE crates get the full deny attribute set
8. **Registry counts are NOT hardcoded** — CI mechanically derives expected IDs from source documents
9. **CN-* tier from classification table** — never inferred from prefix
10. **ade_core extraction is a hard merge guard** — not a soft recommendation
