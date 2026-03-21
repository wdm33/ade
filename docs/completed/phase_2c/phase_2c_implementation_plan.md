# Phase 2C Implementation Plan — Alonzo/Babbage/Conway Verdict Replay

**Cluster**: Phase 2C
**Scope**: Extend Phase 2B verdict replay from Byron–Mary (4 eras) to Byron–Conway (7 eras)
**Prerequisite**: Phase 2B complete (Byron/Shelley/Allegra/Mary verdict replay, 6,000 blocks)

---

## Constitutional Framing

Phase 2C extends the same invariant family as Phase 2B:

- **T-DET-01**: Same canonical inputs → same authoritative bytes
- **DC-LEDGER-01**: Block decode + apply_block must accept every block the oracle accepted
- **RO-TEST-01**: Regression corpus with automated replay

All three new eras (Alonzo, Babbage, Conway) share Shelley's block structure and fit the existing `apply_shelley_era_block` pipeline.

---

## Slice Structure

### Slice 2C-1: Opaque TX Body Decoders

Create `tx.rs` in `ade_codec/src/{alonzo,babbage,conway}/` using `cbor::skip_item()` to validate CBOR well-formedness while capturing raw bytes. No field-level parsing.

### Slice 2C-2: Ledger Rule Dispatch

Add Alonzo/Babbage/Conway arms to `apply_block` and `decode_single_tx_body` in `ade_ledger/src/rules.rs`. Remove `RuleNotYetEnforced` catch-all.

### Slice 2C-3: Oracle Corpus Deposit

Copy 4,500 state hashes from db-analyser extraction. Update `oracle_manifest.toml` with three new era entries.

### Slice 2C-4: Integration Tests

- Block decode tests (3 eras × 1,500 blocks)
- TX body decode tests (3 eras)
- All-eras replay summary (7 eras, 10,500 blocks)
- Per-era verdict agreement + determinism tests

---

## Verification

```bash
cargo build --workspace
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
bash ci/ci_check_dependency_boundary.sh
bash ci/ci_check_forbidden_patterns.sh
```
