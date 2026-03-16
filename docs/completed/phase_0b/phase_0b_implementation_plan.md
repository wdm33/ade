# Phase 0B: Truth Capture & Differential Harnesses — Implementation Plan

## Context

Phase 0A is complete: the ade workspace has 7 skeleton crates (all empty), 42 golden mainnet CBOR blocks across 7 eras, 146 declared constitution invariants, and 6 CI enforcement scripts. **No implementation code exists yet.**

Phase 0B builds the differential comparison infrastructure that all subsequent phases will use to prove behavioral equivalence against the Haskell Cardano node. It introduces no runtime behavior — only test/verification harness code in the GREEN crate (`ade_testkit`), reference extraction artifacts, and CI validation.

**Source specs:** `/home/ts/Documents/ade-planning/phase_0b_truth_capture_differential/`
- `cluster_plan.md` — Cluster-level plan with exit criteria CE-13 through CE-23
- `T-04_differential_block_harness.md` — Block-level comparison
- `T-05_differential_ledger_harness.md` — Ledger state hash comparison
- `T-06_differential_protocol_transcript_harness.md` — Protocol transcript replay

## Execution Order

```
T-04: Differential Block Harness (foundation — provenance, Era, DiffReport)
  ├── T-05: Differential Ledger Harness (parallel after T-04)
  └── T-06: Differential Protocol Transcript Harness (parallel after T-04)
```

T-05 and T-06 are independent and can proceed in parallel once T-04 lands.

---

## Slice 1: T-04 — Differential Block Harness

**Exit Criteria:** CE-13, CE-14, CE-19, CE-20, CE-21, CE-22, CE-23

### Step 1.1: Add GREEN dependencies to ade_testkit

**File:** `crates/ade_testkit/Cargo.toml`

Add `serde`, `serde_json`, `toml` as dependencies. These are allowed in GREEN but forbidden in BLUE. CE-23 mechanically gates this — no BLUE crate may have these in its resolved dependency tree.

### Step 1.2: Create harness module tree

**Files to create:**
- `crates/ade_testkit/src/harness/mod.rs` — `Era` enum (Byron through Conway), `HarnessError` enum (including `NotYetImplemented`), module declarations
- `crates/ade_testkit/src/harness/diff_report.rs` — `DiffReport` with `BTreeMap<String, Divergence>` for deterministic ordering, `Divergence` type
- `crates/ade_testkit/src/harness/provenance.rs` — `ManifestEntry` (all DC-REF-01 fields), `Manifest`, `ProvenanceViolation`, `parse_manifest()`, `validate_manifest()`
- `crates/ade_testkit/src/harness/block_diff.rs` — `BlockDecoder` trait, `BlockFields` (project-owned comparison type using `BTreeMap<String, serde_json::Value>`), `StubBlockDecoder` (returns `NotYetImplemented`), `diff_block_fields()` function

**File to modify:**
- `crates/ade_testkit/src/lib.rs` — add `pub mod harness;`

**Key design constraints:**
- `BlockFields` is a generic comparison container, NOT a domain type
- `BTreeMap` everywhere for deterministic ordering (no HashMap/HashSet)
- `StubBlockDecoder` returns `HarnessError::NotYetImplemented` — no real decoding
- `diff_block_fields()` produces field-level comparison with divergence paths
- All types derive `Serialize`/`Deserialize` where needed for JSON/TOML handling

### Step 1.3: Reference block field extraction

**Files to create:**
- `corpus/tools/extract_block_fields.sh` — Host-local script. Reads CBOR from `corpus/golden/{era}/blocks/`, invokes `cardano-cli debug decode block`, maps output to project schema, writes to `corpus/reference/block_fields/{era}/`. Designed to run on the node host (see Infrastructure section).
- `corpus/reference/block_fields/manifest.toml` — DC-REF-01 provenance for every reference artifact (no empty/placeholder/"TODO" values)
- `corpus/reference/block_fields/{era}/*.json` — One file per reference block per era, all 7 eras

**Provenance manifest fields (all mandatory):** `file`, `source_block`, `era`, `type`, `extraction_tool`, `extraction_tool_version`, `extraction_tool_git_rev`, `cardano_node_version`, `network_magic`, `protocol_version`, `extraction_method`, `extraction_date`, `source_type`, `reproducibility`, `sha256`

**Note:** Extraction requires `cardano-cli` 10.6.2. If Byron extraction fails, fallback to `cardano-ledger-byron` library extraction is acceptable per spec.

### Step 1.4: CI provenance validation

**File to create:**
- `ci/ci_check_ref_provenance.sh` — Python-in-bash pattern. Scans `corpus/reference/*/manifest.toml`, validates DC-REF-01 fields present and non-empty, verifies SHA-256 checksums match file content, reports untracked files not in manifest, exits non-zero on first violation. **Scope: DC-REF-01 provenance integrity only.** Does not handle secret scanning (see `ci_check_no_secrets.sh`).

### Step 1.5: Tests

- Self-comparison test: load reference JSON as both expected and actual → zero divergences (validates comparison machinery without real decoder)
- Provenance validation test: parse manifest, validate all fields
- Self-comparison on >=1 block per era (7 eras)
- `StubBlockDecoder` returns `NotYetImplemented`

### Step 1.6: Constitution registry update

**File:** `constitution_registry.toml`

- **DC-REF-01:** `status` moves from `"declared"` to `"partial"`, add `code_locus`, `ci_script`, `tests`
- **CN-META-03:** stays `"declared"` (provenance necessary but not sufficient)

### Step 1.7: Verify Phase 0A non-regression

- All 6 existing CI scripts pass (CE-21)
- `cargo test --workspace` and `cargo clippy` pass (CE-22)
- No BLUE crate source files modified
- CE-23: verify no BLUE crate has `serde`/`serde_json`/`toml` in resolved tree

---

## Slice 2: T-05 — Differential Ledger Harness

**Exit Criteria:** CE-15, CE-16, CE-21, CE-22
**Depends on:** T-04 (reuses `Era`, `HarnessError`, `provenance.rs`, CI script)

### Step 2.1: Create ledger harness module

**File to create:**
- `crates/ade_testkit/src/harness/ledger_diff.rs`

**Types:**
- `StateHash` — opaque 32-byte type: `pub struct StateHash(pub [u8; 32])` with `Eq`, `Ord`, `Serialize`, `Deserialize`
- `LedgerApplicator` trait — `apply_block(&mut self, era: Era, cbor: &[u8])`, `current_state_hash(&self)`, `reset_to(&mut self, state_hash: &StateHash)`
- `StubLedgerApplicator` — returns `NotYetImplemented`
- `LedgerDiffReport` — `first_divergence: Option<LedgerDivergence>`
- `LedgerDivergence` — block index, expected `StateHash`, actual `StateHash`
- `LedgerHashSequence` — parsed reference hash sequence
- `diff_ledger_sequence()` — applies blocks sequentially, compares state hash after each against reference, reports first divergence

**File to modify:**
- `crates/ade_testkit/src/harness/mod.rs` — add `pub mod ledger_diff;`

### Step 2.2: Reference state hash extraction

**Files to create:**
- `corpus/tools/extract_state_hashes.sh` — Host-local script. Replays blocks through cardano-node, queries state hashes at block boundaries. Runs on the node host (see Infrastructure section).
- `corpus/reference/ledger_state_hashes/manifest.toml` — DC-REF-01 provenance (includes `hash_surface` field)
- `corpus/reference/ledger_state_hashes/hash_surface.md` — Documents exact Haskell type (`ExtLedgerState`), serialization method, byte authority classification, version stability, reproduction instructions
- `corpus/reference/ledger_state_hashes/{era}/*.json` — >=3 contiguous blocks per era, >=1 sequence per era

**Manifest extra fields:** `cardano_node_git_rev`, `genesis_hash`, `hash_surface`, `extraction_method` (LocalStateQuery or LedgerDB)

### Oracle evidence discipline (version-scoped)

State hash artifacts are **version-scoped oracle evidence**, not timeless semantic truth:
- The exact Haskell type (`ExtLedgerState` from `Ouroboros.Consensus.Ledger.Extended`) and serialization path (`encodeLedgerState` via `Codec.Serialise` / `cardano-binary`) must be named in `hash_surface.md`
- The `cardano-node` version and git revision must be pinned in provenance
- `ExtLedgerState` serialization is NOT stable across node versions (hard fork combinator transitions, era-specific changes, consensus library updates all change serialization)
- Comparison is valid **only** against the pinned oracle version — artifacts do not carry forward automatically across node upgrades
- The artifact is oracle-derived evidence, not project-canonical internal encoding, not protocol wire-byte surface — falls outside both branches of the Byte Authority Model
- Extraction script must include a stability check: re-extracting from the same node version and data must produce identical bytes
- Recovery-oriented reference extraction must distinguish full replay from ChainDB's audited snapshot + forward replay model — do not overstate the recovery surface when documenting hash provenance

### Step 2.3: Tests

- Self-comparison test on >=1 era
- `StubLedgerApplicator` returns `NotYetImplemented`
- `ci_check_ref_provenance.sh` automatically validates new manifest

---

## Slice 3: T-06 — Differential Protocol Transcript Harness

**Exit Criteria:** CE-17, CE-18, CE-21, CE-22
**Depends on:** T-04 provenance infrastructure (can land independently if needed)

### Step 3.1: Create protocol harness modules

**Files to create:**
- `crates/ade_testkit/src/harness/protocol_diff.rs`
- `crates/ade_testkit/src/harness/transcript.rs`

**Types in `protocol_diff.rs`:**
- `MiniProtocolId` enum — **Reference source: Cardano network spec / ouroboros-network codebase.** Each ID must be verified against the authoritative Haskell source before implementation.
  - N2N: Handshake (0), ChainSync (2), BlockFetch (3), TxSubmission2 (4), KeepAlive (8), PeerSharing (10)
  - N2C: Handshake (0), LocalChainSync (5), LocalTxSubmission (6), LocalStateQuery (7), LocalTxMonitor (9)
  - Must disambiguate overlapping IDs across connection types (e.g., `N2NHandshake` vs `N2CHandshake`)
  - **Proof obligation:** Each numeric ID must cite the exact reference source (ouroboros-network module path, Haskell type, or network spec section). Protocol-number mapping is a must-match derived compatibility surface.
- `Direction` enum — `InitiatorToResponder`, `ResponderToInitiator`
- `ProtocolStateMachine` trait — `receive_message()`, `current_state_label()`, `reset()`
- `StubProtocolStateMachine` — returns `NotYetImplemented`
- `ProtocolDiffReport` — `first_divergence: Option<ProtocolDivergence>`
- `ProtocolDivergence` — message index, direction, expected payload, actual payload/error
- `replay_transcript()` — feeds inbound messages to state machine, compares outbound against transcript

**Types in `transcript.rs`:**
- `Transcript` — protocol metadata + `Vec<TranscriptMessage>`
- `TranscriptMessage` — `index`, `direction`, `payload_hex`, `payload_length`
- `parse_transcript()` — JSON -> `Transcript`

**File to modify:**
- `crates/ade_testkit/src/harness/mod.rs` — add `pub mod protocol_diff;` and `pub mod transcript;`

### Step 3.2: Transcript capture and demux tools

**Files to create:**
- `corpus/tools/capture_transcripts.sh` — Host-local script. Runs two cardano-node 10.6.2 instances on loopback, captures with tcpdump, produces pcap, invokes demux script. Runs on the node host (see Infrastructure section).
- `corpus/tools/demux_transcript.py` — Parses Cardano mux framing (4-byte timestamp, 2-byte miniprotocol ID, 2-byte payload length, payload), reassembles fragmented messages, produces per-miniprotocol JSON. Handles both N2N and N2C framing.

### Demux correctness proof obligation

Must store at demuxed miniprotocol message level per T-TRANSPORT-01, CN-PROTO-04, DC-PROTO-02. Raw mux frames contain transport nondeterminism (socket fragmentation, mux ordering, timeouts) that must not leak into the comparison surface.

**Proof obligation:** `demux_transcript.py` must be shown to preserve the authoritative message sequence while discarding only transport-level nondeterminism. Verification method: replay demuxed output against a known protocol exchange with independently verified message boundaries. Without this proof, the demuxer becomes a silent semantic transformer.

### Step 3.3: Reference transcripts

**Files to create:**
- `corpus/reference/protocol_transcripts/manifest.toml` — DC-REF-01 with protocol-specific fields (`protocol`, `miniprotocol_id`, `capture_method`, `protocol_version`, `peer_configuration`, `message_count`)
- `corpus/reference/protocol_transcripts/*.json` — >=1 ChainSync (>=10 messages), >=1 BlockFetch (>=5 messages)

**Explicitly initial seed coverage only — not representative protocol-surface proof.**

### Step 3.4: Tests

- Transcript parsing test: valid JSON -> `Transcript` with correct fields
- `StubProtocolStateMachine` returns `NotYetImplemented`
- `ci_check_ref_provenance.sh` validates new manifest

---

## Cross-Cutting Constraints

### Tier Discipline

This plan touches concerns across multiple tiers. They must not be flattened:

**True (deterministic ordering, BLUE purity):**
- Deterministic ordering in all harness outputs (BTreeMap, not HashMap)
- Structured comparable errors (HarnessError, not String)
- No BLUE dependence on nondeterministic shell concerns

**Derived (reference truth must match Cardano observable surfaces):**
- MiniProtocolId numeric values must match ouroboros-network authoritative source
- Transcript semantics, era-aware behavior must match Cardano
- Block field comparison schemas must be project-owned but Cardano-compatible

**Release (CI gates):**
- CI scripts pass
- Provenance manifests validate (ci_check_ref_provenance.sh)
- No secret files tracked (ci_check_no_secrets.sh)

**Operational (AWS infrastructure — outside semantic authority):**
- AWS host access, instance roles, SSM, Secrets Manager
- Hostnames, usernames, keys, socket paths kept private
- Execution environment for extraction scripts
- Not part of any semantic invariant slice

### BLUE/GREEN Boundary (CE-23)
- `serde`, `serde_json`, `toml` only in `ade_testkit` (GREEN)
- Verify: `cargo tree -p ade_codec --no-default-features` (and other BLUE crates) must NOT contain these
- `ci_check_dependency_boundary.sh` enforces dependency-boundary constraints (CE-23 gate)
- `ci_check_ref_provenance.sh` enforces DC-REF-01 provenance integrity only
- `ci_check_no_secrets.sh` enforces operational secret hygiene only

### Forbidden in Phase 0B
- No domain types (Block, Transaction, Header, LedgerState, UTxO)
- No CBOR encoding/decoding logic
- No ledger rule implementations
- No protocol state machine implementations (only trait + stub)
- No BLUE crate modifications
- No mocks
- No networking/storage I/O in GREEN harness code
- No claims that harnesses "enforce" invariants (say "establishes comparison surface")

### Phase 0A Non-Regression (CE-21)
All 6 existing CI scripts must continue to pass:
- `ci_check_dependency_boundary.sh`
- `ci_check_forbidden_patterns.sh`
- `ci_check_module_headers.sh`
- `ci_check_no_semantic_cfg.sh`
- `ci_check_no_signing_in_blue.sh`
- `ci_check_constitution_coverage.sh`

---

## Verification

After each slice:
1. `cargo build --workspace`
2. `cargo test --workspace` — all tests pass
3. `cargo clippy --all` — zero warnings
4. All CI scripts pass (existing 6 + new `ci_check_ref_provenance.sh` + `ci_check_no_secrets.sh`)
5. CE-23 check: no BLUE crate has serde/serde_json/toml in resolved tree

After all slices:
- 10 proof obligations from cluster plan satisfied
- Exit criteria CE-13 through CE-23 met
- DC-REF-01 moved to `"partial"` status in constitution registry
- Self-comparison tests pass for all three harnesses
- Reference artifacts exist with complete provenance for all corpora

---

## Files Modified (Existing)

| File | Change |
|------|--------|
| `crates/ade_testkit/Cargo.toml` | Add serde, serde_json, toml deps |
| `crates/ade_testkit/src/lib.rs` | Add `pub mod harness;` |
| `constitution_registry.toml` | DC-REF-01 status → partial |
| `.gitignore` | Add secret/credential patterns |

## Files Created (New)

| File | Slice |
|------|-------|
| `crates/ade_testkit/src/harness/mod.rs` | T-04 |
| `crates/ade_testkit/src/harness/diff_report.rs` | T-04 |
| `crates/ade_testkit/src/harness/provenance.rs` | T-04 |
| `crates/ade_testkit/src/harness/block_diff.rs` | T-04 |
| `crates/ade_testkit/src/harness/ledger_diff.rs` | T-05 |
| `crates/ade_testkit/src/harness/protocol_diff.rs` | T-06 |
| `crates/ade_testkit/src/harness/transcript.rs` | T-06 |
| `corpus/reference/block_fields/manifest.toml` | T-04 |
| `corpus/reference/block_fields/{era}/*.json` | T-04 |
| `corpus/reference/ledger_state_hashes/manifest.toml` | T-05 |
| `corpus/reference/ledger_state_hashes/hash_surface.md` | T-05 |
| `corpus/reference/ledger_state_hashes/{era}/*.json` | T-05 |
| `corpus/reference/protocol_transcripts/manifest.toml` | T-06 |
| `corpus/reference/protocol_transcripts/*.json` | T-06 |
| `corpus/tools/extract_block_fields.sh` | T-04 |
| `corpus/tools/extract_state_hashes.sh` | T-05 |
| `corpus/tools/capture_transcripts.sh` | T-06 |
| `corpus/tools/demux_transcript.py` | T-06 |
| `corpus/tools/.env.example` | T-04 |
| `ci/ci_check_ref_provenance.sh` | T-04 |
| `ci/ci_check_no_secrets.sh` | T-04 |

## Infrastructure (Operational Tier)

This section describes operational concerns. These are **outside semantic authority** — they do not participate in true/derived invariant enforcement.

- **AWS Cardano node available** — provides `cardano-node` 10.6.2, `cardano-cli`, ImmutableDB, and LedgerDB for all extraction tasks
- **Execution model:** Extraction scripts are repo-owned and reproducible. Execution is **host-local on the AWS node** (or a tightly coupled sidecar). The repo does not depend on baked-in SSH topology or remote-control semantics. Scripts are copied to the node host and run there; only sanitized output artifacts and provenance manifests are committed back.
- **Committed outputs:** Sanitized artifacts + manifests only. No operational details.

### Credential Safety (Public Repo)

This is a public repository. **No credentials, hostnames, IPs, keys, or connection details may be committed.**

**Approach:**
1. **`.gitignore` updates** — Add patterns to block accidental commits:
   - `.env`, `.env.*`, `*.pem`, `*.key`, `id_rsa*`, `ssh_config.local`
   - `corpus/tools/.env`, `corpus/tools/ssh_config.local`
2. **Extraction scripts use environment variables** — All connection details sourced from env vars (e.g., `$ADE_NODE_SOCKET_PATH`, `$ADE_CARDANO_CLI`), never hardcoded. These env vars are set on the node host, not in the repo.
3. **`corpus/tools/.env.example` template** — Checked in with placeholder names only (no real values):
   ```
   ADE_NODE_SOCKET_PATH=/path/to/node.socket
   ADE_CARDANO_CLI=/path/to/cardano-cli
   ADE_IMMUTABLEDB_PATH=/path/to/immutable
   ADE_LEDGERDB_PATH=/path/to/ledger
   ```
4. **Scripts fail-fast on missing env** — Each extraction script checks required env vars at top and exits with clear error if unset
5. **Provenance manifests** — Record tool versions, network magic, extraction method, but **never** record hostnames, IPs, usernames, or key paths
6. **Pre-commit guard** — `ci/ci_check_no_secrets.sh` scans committed files for accidental credential patterns (IP addresses, AWS hostnames, `.pem` references, key file paths). **Separate script from `ci_check_ref_provenance.sh`** — different invariant concerns must not be conflated.

## Remaining Risks

1. **Byron extraction:** `cardano-cli` may not decode all Byron blocks. Fallback: direct Haskell library extraction (acceptable per spec)
2. **Mux demuxing correctness:** `demux_transcript.py` must preserve authoritative message sequence while discarding only transport nondeterminism — verification against known protocol exchanges required (see demux proof obligation above)
3. **`ExtLedgerState` serialization instability:** State hash comparison valid only against pinned oracle version — version-scoped evidence, not timeless truth (see oracle evidence discipline above)
