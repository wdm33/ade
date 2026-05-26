# PHASE4-N-M-A1.1 — Closure record

**Closed:** 2026-05-26
**Closure HEAD:** (set on commit)
**Predecessor HEAD:** `8843e20` (PHASE4-N-M-C close).

## Goal

Close the A1.1 reference-script seed-import gate that held
`DC-EVIDENCE-01` and `RO-LIVE-05` at `enforced_scaffolding` per
PHASE4-N-M-C's closure record.

## Outcome

**Code-level gate CLOSED.** Two additional gates emerged during
implementation and were closed in-line so the full preprod UTxO
dump imports cleanly. The strict literal A1.1 deliverable
(reference-script support) is in production; two adjacent gates
that would have re-surfaced immediately are also closed.

| Gate | Origin | Status |
|---|---|---|
| A1.1 reference-script encode (Babbage `script_ref`) | Original closure note | **closed** |
| A1.2 Byron Base58 address decode (~0.7% of preprod entries) | Surfaced during smoke against full 580MB dump | **closed** |
| Plutus-integer-tolerant JSON field shape (unused `datum` / `inlineDatum` fields with f64-overflowing numbers) | Surfaced during smoke against fresh 2.2GB dump | **closed** |
| Live BlockAdmitted transcript on fully-synced peer | Surfaced during live-pass exercise | **OPEN** — operator action |

## What shipped

### BLUE / GREEN code

- `crates/ade_runtime/src/seed_import/json.rs`:
  - Added `RawReferenceScript` + `RawScriptEnvelope` typed structs
    for the cardano-cli `referenceScript` JSON shape.
  - Stripped the parsed-form `datum` and `inlineDatum` fields
    from `RawUtxoEntry` — serde silently skips unknown JSON
    keys, which tolerates Plutus integers exceeding f64 range
    (preprod has datum trees with `1.79...e308` literals).
  - Doc updated: only `inline_datum_raw` (hex) + `datumhash`
    are consumed; the parsed-form fields are intentionally
    absent.

- `crates/ade_runtime/src/seed_import/importer.rs`:
  - Added `encode_script_ref` — emits canonical Babbage
    `tag(24, bytes(.cbor [variant_tag, raw_cborHex_bytes]))`
    for `script_ref` (TxOut map field 3). Closed-vocabulary
    variant tagging via `script_variant_tag`: `SimpleScript→0`,
    `PlutusScriptV1→1`, `PlutusScriptV2→2`, `PlutusScriptV3→3`.
  - Added `BadReferenceScript { detail: &'static str }` to
    `JsonSeedError` closed sum.
  - Renamed `decode_bech32_address` → wrapped it in a new
    `decode_cli_address` dispatcher that accepts either bech32
    (`addr…` / `stake…`) or Byron Base58 (no checksum, raw CBOR
    envelope).
  - Replaced the legacy
    `UnsupportedTxOutFeature { feature: "referenceScript" }`
    fail-fast in `build_canonical_tx_out` with the new
    acceptance + canonical-field-3 emission path.

- `crates/ade_runtime/Cargo.toml`: added `base58 = "0.2"` as a
  GREEN-only dep (same scoping discipline as the existing
  `bech32` dep — imported only by `seed_import`).

- `crates/ade_runtime/src/seed_import/mod.rs`: updated module
  docstring "Honest scope" block to reflect ref-script + Byron
  support.

### Tests added (13 net new)

In `crates/ade_runtime/src/seed_import/importer.rs::tests`:

- `utxo_seed_accepts_plutus_v2_reference_script` — asserts
  canonical `0x03 0xD8 0x18` marker for field-3 + tag-24.
- `utxo_seed_accepts_plutus_v1_reference_script` — asserts
  `uint(1)` variant tag in the inner array.
- `utxo_seed_accepts_plutus_v3_reference_script` — asserts
  `uint(3)`.
- `utxo_seed_accepts_simple_script_reference_script` — asserts
  `uint(0)` for native-script variant.
- `utxo_seed_reference_script_changes_fingerprint` — proves
  reference-script content participates in `UtxoFingerprint`.
- `utxo_seed_reference_script_deterministic_across_two_imports`
  — two-imports byte-identity preserved across ref-script
  entries.
- `utxo_seed_rejects_unknown_script_type` —
  `BadReferenceScript { detail: "unknown script type" }`.
- `utxo_seed_rejects_non_hex_cbor_hex` —
  `BadReferenceScript { detail: "cbor_hex not valid hex" }`.
- `utxo_seed_rejects_missing_cbor_hex` — closed sum fall-through
  on missing required field.
- `utxo_seed_canonical_script_ref_encoder_known_vector` —
  byte-vector regression: `(PlutusV2, "4401020304")` →
  `D8 18 47 82 02 44 01 02 03 04` (10 bytes).
- `utxo_seed_accepts_byron_base58_address` — Byron preprod
  address imports cleanly; canonical TxOut has non-empty
  address bytes.
- `utxo_seed_byron_address_decodes_to_expected_bytes` — raw
  decoded bytes start with `0x82` (Byron CBOR `array(2)`
  envelope sanity).
- `utxo_seed_rejects_garbage_address` — non-bech32 + non-Base58
  → `BadAddress`.

In `crates/ade_runtime/src/seed_import/json.rs::tests`:

- `parse_utxo_seed_json_reference_script_entry` — typed
  deserialization of the cardano-cli referenceScript shape.

Net tally: **26 seed_import unit tests** (was 13), **all
passing**.

### CI

- New gate: `ci/ci_check_seed_import_full_preprod_support.sh`
  — verifies `encode_script_ref` is the sole authority, the
  four script-variant tags are matched, both bech32 and Byron
  Base58 decoders are present, and the legacy
  reference-script fail-fast literal is gone.
- Existing gate `ci/ci_check_seed_import_closure.sh` still
  passes (CN-SEED-01 single-pub-fn discipline preserved).

### Evidence

- `docs/evidence/phase4-n-m-a1.1-admission-bootstrap-transcript.jsonl`
  — committed 3-event transcript from a live admission run
  against the docker preprod peer with the FULL 2.2 GB
  freshly-extracted UTxO dump. Records `admission_started` +
  `bootstrap_complete` events binding the canonical
  `consensus_inputs_fingerprint_hex`
  (`4cc62d3bdbf18f1c0d98188da5e1a85090e03379068e87267384f9873cf4e920`)
  and `initial_ledger_fp_hex`
  (`50a4542bdc8c51016025e29d70d9f683d5d3be1f0be960649647bda68abed8eb`)
  at `chain_tip_slot 89727988` (epoch 211). The trailing
  `admission_shutdown { reason: "upstream_dropped" }` is the
  expected outcome until the peer is fully synced (see §
  "Open obligations" below).

## Smoke fingerprints (evidence)

| Input | Entries | Fingerprint (Blake2b-256 of canonical CBOR UTxO map) |
|---|---|---|
| `/tmp/utxo_sample_mini.json` (129-entry mini sample, the original PHASE4-N-M-A smoke fixture) | 129 / 129 | `13280fc1e11bbe6d908c91deff0c4cb3731f7b7e88b13c1d716e0211bc176378` |
| `/tmp/utxo_sample_no_refscript.json` (128 entries, ref-script-stripped subset) | 128 / 128 | `5725e8a4ebc1821d84f353f2569a628c98fbb48183adb5952426f55c15cc050b` |
| `/tmp/utxo_sample_simplest.json` (105 entries, smallest fixture) | 105 / 105 | `0cb228eb796677eb125576a60481a22fa5431d4c5bf2bb41ea8329f01dc1e1c4` |
| Full preprod dump at slot 76023029 (`docs/evidence/phase4-n-m-c-utxo-seed.json`, 580 MB) | 809,120 / 809,120 | `5b6009722f71b1a90674020ebb584d6f00bf1a724308ac7c092c4fdd81a2602b` |
| Fresh preprod dump at slot 89727988 (2.2 GB, operator-extracted during this slice) | 1,909,985 / 1,909,985 | `fc5f8895e972bd3d68119db95efd42c4911e4d5eb01894870853dce07aa54923` |

All five fingerprints differ — reflecting distinct UTxO
contents — and each is deterministic across two imports of the
same input (verified by `utxo_seed_two_imports_byte_identical`
and `utxo_seed_reference_script_deterministic_across_two_imports`).

## Registry effects

- **CN-SEED-01** (release): strengthened. `status` stays
  `enforced`. `strengthened_in` += `PHASE4-N-M-A1.1`. 10 new
  test paths appended to `tests` array. `ci_script` array
  extended with the new full-preprod-support gate.
  `evidence_notes` extended with the A1.1 + A1.2 + full-dump
  smoke fingerprint.
- **DC-SEED-01** (derived): same shape — 3 new
  fingerprint-sensitivity tests appended.
- **DC-EVIDENCE-01** (derived): `status` remains
  `enforced_scaffolding`. `open_obligation` re-tagged from
  `blocked_until_A1_1_reference_script_seed_import` to
  `blocked_until_operator_pass_with_fully_synced_peer`.
  `strengthened_in` += `PHASE4-N-M-A1.1`. `evidence_notes`
  rewritten to (a) record A1.1 + A1.2 closure, (b) describe
  the freshly-surfaced remaining gate (peer is mid-sync;
  intersects at the bundle's tip race the peer's
  still-advancing chain). The committed bootstrap transcript
  is the load-bearing artifact.
- **RO-LIVE-05** (release): same shape — same status, same
  `open_obligation` re-tag, same `strengthened_in` entry.

## Open obligations (post-A1.1)

- **DC-EVIDENCE-01 + RO-LIVE-05 BlockAdmitted half** — gated
  on `blocked_until_operator_pass_with_fully_synced_peer`.
  The docker peer is at ~72% sync; the bundle's tip slot is
  ahead of the peer's committed-chain frontier. Operator
  action: wait for peer to fully sync, regenerate bundle +
  UTxO dump at a stable tip, set
  `ADE_LIVE_REQUIRE_BLOCK_ADMITTED=1` and re-run.

- **RO-GENESIS-REPLAY-01** — unchanged.
- **RO-MITHRIL-IMPORT-01** — unchanged.

## What's NOT in this cluster

- Plutus-script semantic validation (Phase 5).
- Native-script witness evaluation.
- ChainDb persistence of admitted blocks (future
  strengthening).
- Mithril-authenticated import.
- Block production live pass.

## References

- Cluster doc: `docs/clusters/PHASE4-N-M-A1.1/cluster.md`.
- Slice doc: `docs/clusters/PHASE4-N-M-A1.1/A1.1.md`.
- Predecessor closure: `docs/clusters/PHASE4-N-M-C/cluster.md`
  + `docs/evidence/phase4-n-m-c-operator-pass-README.md`.
- Doctrine: [[feedback-oracle-seed-then-ade-owns]],
  [[feedback-shell-must-not-overstate-semantic-truth]],
  [[feedback-fail-closed-validation]].
