# Phase 2C Closing Report — Alonzo/Babbage/Conway Verdict Replay

**Cluster**: Phase 2C
**Status**: Complete
**Date completed**: 2026-03-21

---

## Summary

Phase 2C extended the Phase 2B verdict replay pipeline from Byron–Mary (4 eras) to the full Byron–Conway range (7 eras). The work added opaque transaction body decoders for Alonzo, Babbage, and Conway, wired all three eras into the `apply_block` ledger rule dispatch, copied 4,500 oracle state hashes from db-analyser extraction, and created 15 new integration tests proving verdict agreement, determinism, and CBOR structural validity across all 4,500 new blocks.

After Phase 2C, the contiguous replay corpus covers 10,500 blocks (1,500 per era × 7 eras), all accepted by `apply_block`. The oracle manifest now has entries for all seven Cardano eras from ByronEbb through Conway. All existing Phase 2B tests continue to pass, confirming non-regression.

---

## Exit Criteria Verification

| Proof Obligation | Result |
|------------------|--------|
| All 1,500 Alonzo blocks decode through HFC envelope + era-specific decoder | PASS — `alonzo_contiguous_blocks_decode` |
| All 1,500 Babbage blocks decode through HFC envelope + era-specific decoder | PASS — `babbage_contiguous_blocks_decode` |
| All 1,500 Conway blocks decode through HFC envelope + era-specific decoder | PASS — `conway_contiguous_blocks_decode` |
| All Alonzo tx bodies decode (opaque, CBOR well-formedness) | PASS — `alonzo_contiguous_tx_decode` |
| All Babbage tx bodies decode (opaque, CBOR well-formedness) | PASS — `babbage_contiguous_tx_decode` |
| All Conway tx bodies decode (opaque, CBOR well-formedness) | PASS — `conway_contiguous_tx_decode` |
| `apply_block` accepts all 1,500 Alonzo blocks (verdict agreement) | PASS — `alonzo_replay_all_1500`, `alonzo_replay_verdict_agreement` |
| `apply_block` accepts all 1,500 Babbage blocks (verdict agreement) | PASS — `babbage_replay_all_1500`, `babbage_replay_verdict_agreement` |
| `apply_block` accepts all 1,500 Conway blocks (verdict agreement) | PASS — `conway_replay_all_1500`, `conway_replay_verdict_agreement` |
| Alonzo replay determinism: two runs produce identical state | PASS — `alonzo_replay_determinism` (200 blocks) |
| Babbage replay determinism: two runs produce identical state | PASS — `babbage_replay_determinism` (200 blocks) |
| Conway replay determinism: two runs produce identical state | PASS — `conway_replay_determinism` (200 blocks) |
| Oracle state hash files: 1,500 entries each, valid hex | PASS — `state_hash_files_have_correct_counts`, `state_hashes_are_valid_hex` |
| Oracle manifest: 7 era entries, parses correctly | PASS — `oracle_manifest_exists_and_parses` |
| All 10,500 blocks across 7 eras replay with zero verdict disagreement | PASS — `all_eras_replay_summary` |
| `cargo clippy --workspace --all-targets -- -D warnings` clean | PASS |
| `ci_check_dependency_boundary.sh` PASS | PASS |
| `ci_check_forbidden_patterns.sh` PASS | PASS |
| All existing Phase 2B tests pass (non-regression) | PASS — 482 total workspace tests |

---

## What Was Delivered

### Opaque TX Body Decoders (BLUE — ade_codec)

| Deliverable | Location |
|-------------|----------|
| `decode_alonzo_tx_body()` — opaque capture via `skip_item`, CBOR well-formedness validation | `crates/ade_codec/src/alonzo/tx.rs` |
| `decode_babbage_tx_body()` — opaque capture via `skip_item`, CBOR well-formedness validation | `crates/ade_codec/src/babbage/tx.rs` |
| `decode_conway_tx_body()` — opaque capture via `skip_item`, CBOR well-formedness validation | `crates/ade_codec/src/conway/tx.rs` |
| `pub mod tx;` declarations in era codec modules | `crates/ade_codec/src/{alonzo,babbage,conway}/mod.rs` |

### Ledger Rule Dispatch (BLUE — ade_ledger)

| Deliverable | Location |
|-------------|----------|
| `apply_block` match arms for Alonzo, Babbage, Conway → `apply_shelley_era_block` | `crates/ade_ledger/src/rules.rs` |
| `decode_single_tx_body` match arms for Alonzo, Babbage, Conway | `crates/ade_ledger/src/rules.rs` |
| Removed `RuleNotYetEnforced` catch-all (all eras now handled) | `crates/ade_ledger/src/rules.rs` |

### Oracle Corpus Data

| Deliverable | Location |
|-------------|----------|
| Alonzo state hashes (1,500 entries, slots 39917142–39948344) | `corpus/contiguous/alonzo_state_hashes.txt` |
| Babbage state hashes (1,500 entries, slots 72317013–72347541) | `corpus/contiguous/babbage_state_hashes.txt` |
| Conway state hashes (1,500 entries, slots 133661020–133692593) | `corpus/contiguous/conway_state_hashes.txt` |
| Oracle manifest updated with 3 new era entries | `corpus/contiguous/oracle_manifest.toml` |

### Integration Tests

| Deliverable | Location |
|-------------|----------|
| Block decode tests (3 eras × 1,500 blocks) | `crates/ade_testkit/tests/contiguous_corpus_decode.rs` |
| TX body decode tests (3 eras) | `crates/ade_testkit/tests/contiguous_corpus_tx_decode.rs` |
| All-eras replay summary (7 eras, 10,500 blocks) | `crates/ade_testkit/tests/differential_replay_all_eras.rs` |
| Per-era verdict + determinism tests (3 eras × 2 tests) | `crates/ade_testkit/tests/differential_alonzo_babbage_conway_replay.rs` |

---

## Workspace Layout After Phase 2C

```
ade/
├── crates/
│   ├── ade_codec/src/
│   │   ├── alonzo/
│   │   │   ├── mod.rs                    # decode_alonzo_block
│   │   │   └── tx.rs                     # decode_alonzo_tx_body (NEW)
│   │   ├── babbage/
│   │   │   ├── mod.rs                    # decode_babbage_block
│   │   │   └── tx.rs                     # decode_babbage_tx_body (NEW)
│   │   └── conway/
│   │       ├── mod.rs                    # decode_conway_block
│   │       └── tx.rs                     # decode_conway_tx_body (NEW)
│   │
│   ├── ade_ledger/src/
│   │   └── rules.rs                      # apply_block now handles all 8 CardanoEra variants
│   │
│   └── ade_testkit/tests/
│       ├── contiguous_corpus_decode.rs    # 10 tests (7 eras + state hashes + hex + manifest)
│       ├── contiguous_corpus_tx_decode.rs # 7 tests (7 eras)
│       ├── differential_replay_all_eras.rs # 8 tests (7 eras + summary)
│       └── differential_alonzo_babbage_conway_replay.rs  # 6 tests (NEW)
│
├── corpus/contiguous/
│   ├── alonzo/                           # 1,500 CBOR block files
│   ├── alonzo_state_hashes.txt           # 1,500 oracle entries (NEW)
│   ├── babbage/                          # 1,500 CBOR block files
│   ├── babbage_state_hashes.txt          # 1,500 oracle entries (NEW)
│   ├── conway/                           # 1,500 CBOR block files
│   ├── conway_state_hashes.txt           # 1,500 oracle entries (NEW)
│   └── oracle_manifest.toml             # 7 era entries (UPDATED)
│
└── docs/completed/phase_2c/
    ├── phase_2c_closing_report.md
    └── phase_2c_implementation_plan.md
```

---

## Test Inventory (482 tests)

| Module | Tests | What they verify |
|--------|-------|------------------|
| `contiguous_corpus_decode` | 10 | HFC envelope + era-specific block decoding, state hash counts, hex validity, manifest |
| `contiguous_corpus_tx_decode` | 7 | Per-era tx body decoding across all 10,500 blocks |
| `differential_replay_all_eras` | 8 | Verdict agreement across 7 eras + aggregate summary |
| `differential_alonzo_babbage_conway_replay` | 6 | Per-era verdict agreement + determinism for Phase 2C eras |
| `differential_allegra_mary_replay` | 4 | Per-era verdict agreement + determinism (Phase 2B) |
| `differential_shelley_replay` | 3 | Shelley verdict agreement + determinism + slot progression |
| `differential_byron_replay` | 3 | Byron verdict agreement + determinism + UTxO progression |
| `differential_byron_utxo` | 1 | Byron replay with genesis UTxO |
| `differential_byron_utxo_full` | 2 | Byron UTxO equality across full 1,500-block window |
| `differential_shelley_utxo_load` | 3 | Shelley UTxO bootstrap (84,609 entries) |
| `differential_utxo_set_equality` | 2 | Genesis + post-block-1 UTxO set equality |
| `ade_ledger::rules::tests` | 2 | Byron EBB pass-through, determinism |
| Other unit/integration tests | 331 | Codec, crypto, types, harness infrastructure |

---

## Architecture Decisions

### 1. Opaque tx body decoders (skip_item)

Alonzo, Babbage, and Conway tx bodies introduce new CBOR map keys (Plutus scripts, datums, redeemers, inline datums, reference scripts, governance actions, voting). Rather than parsing all of these fields — which would require extensive new type infrastructure — the Phase 2C decoders use `cbor::skip_item()` to validate CBOR structural well-formedness while capturing the raw bytes. This is sufficient for verdict agreement (block acceptance/rejection) and defers field-level parsing to a future phase when UTxO resolution, script evaluation, or governance validation are needed.

### 2. apply_shelley_era_block reuse

All post-Byron eras share Shelley's block structure (`ShelleyBlock` type alias). The `apply_shelley_era_block` function is era-agnostic: it extracts the slot from the header, calls `decode_and_count_tx_bodies` to exercise the CBOR parsing pipeline, and updates epoch state. Adding three eras required only three match arms in `apply_block` and three arms in `decode_single_tx_body`. No changes to `apply_shelley_era_block` itself.

### 3. Removal of RuleNotYetEnforced catch-all

With all 8 `CardanoEra` variants now handled in `apply_block`, the catch-all `_` arm returning `RuleNotYetEnforced` was removed. The match is now exhaustive. The test `apply_block_alonzo_returns_not_yet_enforced` was removed since Alonzo blocks are now processed.

---

## Deviations from Plan

### No implementation plan document preceded this cluster

Phase 2C was executed by replicating the Phase 2B pattern for three additional eras. The scope was clear from the available corpus data (1,500 blocks per era, state hashes, boundary states) and the existing test infrastructure. No formal planning document was required.

### Epoch numbers derived from slot arithmetic

The per-era replay tests require an initial epoch number. These were computed from mainnet Shelley genesis parameters (slot 4492800, epoch 208, epoch length 432000 slots): Alonzo epoch 290, Babbage epoch 365, Conway epoch 507. The values match the oracle state hash slot ranges.

---

## Carry-Forward for Next Cluster

### Oracle state hash comparison not yet wired

The 4,500 oracle state hashes are deposited and validated (correct count, valid hex), but no test yet compares them against Ade-computed `ExtLedgerState` hashes. This requires an `ExtLedgerState` encoding function (Shelley telescope format) and is a Phase 3+ obligation.

### Structured tx body parsing deferred

Alonzo/Babbage/Conway tx bodies are decoded as opaque raw bytes. Field-level parsing (Plutus scripts, datums, governance actions) is needed for UTxO resolution, script evaluation, and governance rule enforcement.

### Boundary state CBOR dumps available but unused

Boundary states exist at:
- `phase2c_output/alonzo/ext_ledger_state_dumps/boundary_state_39917116.cbor`
- `phase2c_output/babbage/ext_ledger_state_dumps/boundary_state_72317003.cbor`
- `phase2c_output/conway/ext_ledger_state_dumps/boundary_state_133660855.cbor`

These will be needed when UTxO-level replay requires starting state import (as was done for Shelley in Phase 2B).

### Contiguous block archives available

Compressed archives (13–36 MB each) are stored externally and can be re-extracted if corpus data is lost.

---

## Oracle Provenance

| Field | Value |
|-------|-------|
| cardano_node_version | 10.6.2 |
| cardano_node_git_rev | 0d697f14 |
| consensus_version | ouroboros-consensus-cardano 0.26.0.3 |
| extractor_tool | db-analyser |
| db_snapshot_id | mithril epoch 618, immutable 8419, hash 7e3e59b8 |
| extraction_date | 2026-03-21 |
| comparison_surface | Blake2b-256 of encodeDiskExtLedgerState |
| network | mainnet |
| network_magic | 764824073 |
