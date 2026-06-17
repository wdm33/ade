# Mithril documented-interface evidence bundle — schema

> **Slice:** `RO-MITHRIL-IMPORT-01-EVIDENCE-SCHEMA` (preparation only).
> **Status of the obligation:** `RO-MITHRIL-IMPORT-01` remains **partial**.
> This schema + its validator + the capture script exist so the remaining
> item (c) — a *committed, reproducible, operator-witnessed* documented-interface
> pass — is turnkey. **No registry flip and no `ci_check_*` gate ship with this
> slice** (no document-only / theater enforcement; the runbook is right).

## What a bundle proves

A committed, validator-green bundle proves the **documented-interface chain**
ran end-to-end on **real artifacts**:

```
mithril-client-verified snapshot  →  cardano-node at the certified state
  →  cardano-cli query utxo / tip  →  Ade seed_import (import_cardano_cli_json_utxo)
  →  ade_node --mode node first-run (--mithril-manifest-path)
  →  bootstrap_from_mithril_snapshot → verify_mithril_binding (PASS)
```

…and that the **binding discriminates** (a deliberately-mismatched negative
control fails closed before storage init). It does **not** pass on "Mithril
cert verified" alone — that is only the first link.

This is the gap [`.mithril-scratch/`](../../../.mithril-scratch) does **not**
close: that scratch records a mithril-client *cert verify* of the peer DB
(link 1), is self-described as "strong but not airtight", and carries none of
the Ade-side links (seed_import, binding, recomputed `seed_artifact_hash`).

## Bundle = flat TOML manifest + sha256-bound artifacts

A bundle is one `docs/evidence/mithril-documented-evidence_<network>_<date>.toml`
manifest plus the artifact files it references (committed in the same
directory). Every `*_file` is relative to the manifest dir; every paired
`*_sha256` MUST equal the real file's sha256. The validator
(`ci/validate_mithril_documented_evidence.sh`) recomputes and compares, so a
hand-authored manifest with no real fixtures **fails** — the same
"no synthetic manifest" teeth as `ci_check_ba02_evidence_manifest_schema.sh`.

### Required fields

| Field | Meaning |
|---|---|
| `schema_version` | `1`. |
| `ade_commit` | git HEAD of the `ade_node` binary used. |
| `network` | `preprod` \| `preview` \| `mainnet`. |
| `captured_by` | operator handle (this is operator-witnessed). |
| `mithril_aggregator_endpoint` | aggregator the snapshot/cert came from. |
| `mithril_certificate_hash` | 64-hex cert hash the mithril-client verified. |
| `mithril_client_version` / `cardano_node_version` / `cardano_cli_version` | tool versions. |
| `mithril_signed_entity` | the cert's signed entity (e.g. `CardanoDatabase { epoch, immutable_file_number }`). |
| `mithril_immutable_range_lo` / `_hi` | snapshot immutable-file range. |
| `mithril_certified_slot` / `mithril_certified_block_hash` | the cert's attested `certified_point`. |
| `operator_seed_point_slot` / `operator_seed_point_block_hash` | the operator's **independently-extracted** seed point (DC-MITHRIL-02). |
| `utxo_json_file` / `_sha256` | `cardano-cli query utxo --whole-utxo` output (the seed). **Out-of-tree large artifact** — gitignored like the repo's `*-utxo-seed.json` (multi-GB; not committed). Hash-pinned via `_sha256` + `ade_recomputed_seed_artifact_hash`; the validator sha-verifies it only if present locally, else accepts it as hash-pinned + reproducible. |
| `consensus_inputs_file` / `_sha256` | `ci/build_consensus_inputs_bundle.sh` output. |
| `mithril_manifest_file` / `_sha256` | the `--mithril-manifest-path` JSON (`RawMithrilManifest`). |
| `node_transcript_file` / `_sha256` | captured `ade_node --mode node` stderr + exit. |
| `ade_recomputed_seed_artifact_hash` | Ade `blake2b_256_of_file(utxo.json)`. |
| `ade_initial_ledger_fingerprint` | from the `first-run Mithril bootstrap complete` transcript line. |
| `ade_imported_utxo_fingerprint` | imported `UtxoFingerprint`. |
| `binding_result` | MUST be `"pass"`. |
| `node_exit_code` | MUST be `0`. |
| `negative_control_manifest_file` / `_sha256` | the deliberately-mismatched manifest. |
| `negative_control_outcome` | MUST be `"fail_closed"`. |
| `negative_control_error` | one of `CertifiedPointMismatch`, `EpochMismatch`, `NetworkMagicMismatch`, `GenesisHashMismatch`, `CertificateHashMismatch`, `UnsupportedArtifactType`. |
| `negative_control_node_exit_code` | MUST be nonzero. |

### On DC-MITHRIL-02 (independence vs. equality)

The validator asserts `mithril_certified_{slot,block_hash} ==
operator_seed_point_{slot,block_hash}` for a **passing** bundle. That is not a
contradiction of DC-MITHRIL-02: independence is about the seed point's
**origin** (operator-extracted via cardano-cli, never laundered from the
manifest — enforced in code and by `ci_check_mithril_seed_point_independence.sh`),
while `verify_mithril_binding` requires the two independently-sourced **values**
to *agree*. The negative control is the case where they disagree → fail-closed.

## Promotion path (the future item-(c) flip slice — NOT this slice)

When a real bundle has been captured and committed and
`ci/validate_mithril_documented_evidence.sh` is green over it:

1. Copy this validator body to `ci/ci_check_mithril_documented_evidence.sh`
   (it keeps the vacuous-pass-when-absent semantics, so it is safe in the gate set).
2. Append `ci/ci_check_mithril_documented_evidence.sh` to
   `RO-MITHRIL-IMPORT-01.ci_script` in `docs/ade-invariant-registry.toml`,
   and add the committed bundle to its `evidence`.
3. Flip `RO-MITHRIL-IMPORT-01.status` `partial → enforced`; record the bundle
   path + commit in `open_obligation`/`evidence_notes`.

Until then the obligation is honestly open.
