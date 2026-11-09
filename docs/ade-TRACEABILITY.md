# Normative Rule Traceability — Ade

> **Status:** Living architectural document. Regenerated; not hand-edited.
> Per-project instance of `~/.claude/methodology/templates/traceability.md`, in IDSE v3.0 form.

> **HEAD:** `08c7c3f5` (`chore(idse): re-evidence pass at e563ad94 (migration M2)`). **Evidence revision:** `e563ad94` (every evidence record in the registry). **Registry:** `docs/ade-invariant-registry.toml`, `schema_version = 3`, `registry_count = 260`, assumptions A1–A7. Sibling grounding docs regenerated at the same HEAD: `docs/ade-CODEMAP.md`, `docs/ade-HEAD_DELTAS.md` (baseline `1e4896eb`).

**Regenerated for IDSE.** This is the first TRACEABILITY since Ade migrated from legacy IDD to IDSE v3.0 (`strict` profile, namespace `ADE`; `docs/planning/idse-migration-plan.md`). The previous edition (at `1e4896eb`) was keyed to the 432-rule legacy IDD registry and is superseded in full. Every rule below is an IDSE registry ID. Legacy IDs appear only in each rule's closing `Legacy:` note, for continuity; follow a legacy ID forward through `docs/legacy/idd-to-idse-map.toml`. The frozen legacy registry is `docs/legacy/idd-invariant-registry.toml`.

This document is the **invariant ↔ enforcement audit** of Doctrine §27. For every invariant it traces where the invariant was defined (Source), what must hold (Requirement), where it is enforced in code (Code), which tests prove it (Tests), which CI checks fail the build on violation (CI), and what the evidence currently supports (Claim). A rule that cannot fill the Code, Tests and CI cells is an **enforcement gap**, surfaced here and never hidden.

**Headline.** No invariant is `_enforced_`. Doctrine §10 counts an invariant as enforced only when its claim is Supported at the current revision, and no Ade claim is Supported yet: no IDSE cluster has declared acceptance obligations, and no critical invariant has a mutation witness. 240 invariants have at least one mechanism (`_partial_`); 20 have none at all (`_declared_`, and the reason PRC-001 is `falsified`).

## Source of rules

The single rule source is the IDSE invariant registry `docs/ade-invariant-registry.toml` (`.idse-config.json` `invariant_registry`). `normative_docs` is `[]` and `clusters_dir` (`docs/clusters`) does not exist yet, so there is no second source and no MUST/SHALL extraction. This doc is a join of registry entries with static codebase checks:

| Cell | Registry field |
|---|---|
| Heading ID | `id` |
| **Source** | `approval.record`: the Define batch consensus record in which the human approved the invariant line by line (Doctrine §32) |
| **Requirement** | `name` + `invariant`, verbatim |
| **Code** | `enforcement_compile_time` (marked *compile-time*) then `enforcement_runtime`, de-duplicated to the path. The per-locus annotations stay in the registry: they quote legacy IDs and legacy cluster names, which IDSE documents do not cite. |
| **Tests** | `tests`, verbatim up to any parenthetical note |
| **CI** | `ci_scripts`, annotated *(red at e563ad94)* where the script exited nonzero in `docs/evidence/idse-m2-reevidence.json` |
| **Claim** | `claim_status`, then the latest `[[rules.evidence]]` record: revision, `result`, `observed`, and its `limits` in order. Copied, never upgraded. |
| Closing note | criticality, locality, scope, assumptions, `depends_on`, secondary families, `legacy_ids` |

**Static existence checks at HEAD.** Every Code path and every CI script resolves on disk (0 dangling). A test name the registry lists that is not a `fn` anywhere in the tracked Rust sources is daggered **†**. A test entry that names a test module or a test file rather than a function is tagged *(test module)* / *(test file)*: it exists but cannot be checked by name. Test outcomes come from the evidence artifact; tests were not re-run for this doc.

**Status in the heading** (Doctrine §10 enforcement axis): `_enforced_` requires a Supported claim with evidence current for the rule's Code paths; `_partial_` means at least one Code, Tests or CI mechanism exists but the claim is not Supported; `_declared_` means no mechanism is named at all. **Evidence freshness (Doctrine §11):** the only commit after `e563ad94` is `08c7c3f5`, which touches `docs/ade-invariant-registry.toml` and `docs/evidence/idse-m2-reevidence.json` only; the working tree modifies `CLAUDE.md`, `docs/ade-CODEMAP.md` and `docs/ade-HEAD_DELTAS.md` only. No Code path of any rule changed, so no evidence record is stale.

**Organization.** Level 2 is the Doctrine §6 family, in §6 order. IDSE IDs have no stem, and Ade's `scope` field is near-unique (236 distinct values across 260 entries), so level 3 groups rules by the **Define batch** that approved them (`### <FAM> — <batch>`), the closest short, stable grouping the registry carries. Level 4 is the rule. Within a family, batches and rules both run in ID order (IDs are assigned in sequence at approval, so the order is stable across regenerations).

## Assumptions

Invariants cite these by ID. Each is recorded once in the registry.

| ID | Name | Statement | Approval record |
|---|---|---|---|
| `A1` | ordered-recorded-inputs | for each authoritative state machine (ledger, chain selection, and each mini-protocol's state), the shell delivers inputs one at a time in one total order, and records the starting state, that order and each input's explicit context, so the history can be replayed completely. | `docs/planning/foundations.define.consensus.md#batch-1` (2026-11-08) |
| `A2` | reference-node-semantics | the Haskell cardano-node defines the reference semantics for every observable protocol behaviour Ade claims to match; each comparison names the exact reference build it uses (the live venue runs 11.0.1; the pinned ledger-state hash corpus was extracted from 10.6.2). Ade executes only Conway semantics: it enters at a certified non-Origin Conway tip and never replays or executes historical eras, keeping historical era facts only as identifiers and slot-time geometry. | `docs/planning/foundations.define.consensus.md#batch-1` (2026-11-08) |
| `A3` | nonzero-derived-kes-seeds | a KES sub-seed derived by the Sum6 seed expansion is never all zero (probability 2^-256), so a zeroed sub-seed in a stored key marks a period that has been evolved away. | `docs/planning/codec-wire-crypto.define.consensus.md#batch-1` (2026-11-08) |
| `A4` | rollback-depth-bound-k | the node never adopts a chain that requires rolling back more than k blocks, where k is the consensus security parameter the network's genesis defines under the reference semantics of A2; blocks more than k below the selected tip are immutable. | `docs/planning/epoch.define.consensus.md#batch-1` (2026-11-08) |
| `A5` | honest-peer-delivery | after a network partition heals, the node stays connected to at least one peer that offers the chain the network is converging on, and messages from that peer are delivered and processed within a bounded time. | `docs/planning/consensus.define.consensus.md#batch-1` (2026-11-08) |
| `A6` | durable-commit-honoured | a write that the storage layer reports as durably committed (a completed fsync or committed database transaction) survives process termination and power loss intact; writes not yet reported committed may be lost. | `docs/planning/durability-recovery.define.consensus.md#batch-1` (2026-11-08) |
| `A7` | mithril-snapshot-trust | a Mithril cardano-database snapshot whose immutable files the Mithril client verifies against a certificate chain rooted in the network's published Mithril genesis verification key, and whose ancillary files (the ledger state and UTxO tables) verify under the network's published ancillary verification key, holds the reference node's ledger state at the chain point that state records. Ade does not verify the Mithril multisignature itself: the Mithril client, run outside Ade's core, does. | `docs/planning/mithril-evidence.define.consensus.md#batch-1` (2026-11-09) |

Citations across the 260 invariants: A1 35, A2 125, A3 2, A4 11, A5 4, A6 12, A7 4. Every assumption is cited; every cited assumption exists.

## Rule inventory (mechanical, at HEAD)

| Family | Invariants | critical | standard | _partial_ | _declared_ | claim partial | claim unresolved | claim falsified | claim supported |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| SEC Security | 38 | 38 | 0 | 33 | 5 | 29 | 9 | 0 | 0 |
| STR Structural | 22 | 18 | 4 | 22 | 0 | 15 | 7 | 0 | 0 |
| SAF Safety | 25 | 25 | 0 | 24 | 1 | 18 | 7 | 0 | 0 |
| LIV Liveness | 11 | 8 | 3 | 11 | 0 | 8 | 3 | 0 | 0 |
| VAL Validity | 67 | 59 | 8 | 65 | 2 | 49 | 18 | 0 | 0 |
| ORD Ordering | 6 | 6 | 0 | 5 | 1 | 3 | 3 | 0 | 0 |
| FIN Finality | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 |
| CON Conservation | 5 | 5 | 0 | 5 | 0 | 4 | 1 | 0 | 0 |
| DET Determinism | 26 | 23 | 3 | 26 | 0 | 23 | 3 | 0 | 0 |
| DUR Durability | 14 | 14 | 0 | 13 | 1 | 8 | 6 | 0 | 0 |
| REC Recovery | 8 | 8 | 0 | 7 | 1 | 3 | 5 | 0 | 0 |
| EVO Evolution | 6 | 5 | 1 | 5 | 1 | 4 | 2 | 0 | 0 |
| OBS Observability | 9 | 3 | 6 | 9 | 0 | 8 | 1 | 0 | 0 |
| PRC Process Assurance | 23 | 3 | 20 | 15 | 8 | 12 | 10 | 1 | 0 |
| **All** | **260** | **215** | **45** | **240** | **20** | **184** | **75** | **1** | **0** |

- **Define batches** (approval records, all `#batch-1`): node 38, epoch 30, pipeline-production 27, resources-release 24, ledger 23, mithril-evidence 21, network 20, foundations 19, durability-recovery 17, codec-wire-crypto 15, consensus 14, consensus-inputs 12.
- **Evidence:** 238 records, all at `e563ad94` (189 `pass`, 49 `fail`); 22 invariants have no evidence record. Artifact `docs/evidence/idse-m2-reevidence.json` (sha256 `2409cf6b…d14d47`, matches every record).
- **Redefinitions:** 16 invariants carry one recorded redefinition. **0 deprecated.** FIN has no invariant.

---

## SEC — Security

_Doctrine §6: Authentication, authorization, isolation, secrets, cryptographic verification, trust boundaries, and abuse prevention._ 38 invariant(s).

### SEC — foundations

#### `SEC-001` — _partial_

| Aspect | Location |
|--------|----------|
| **Source** | `docs/planning/foundations.define.consensus.md#batch-1` (approved by wdm33, 2026-11-08); 1 recorded redefinition(s) in the registry |
| **Requirement** | **SigningInShellVerificationInCore** — Private key material (KES, VRF and cold keys) is loaded, stored and owned only by RED shell code, and every signing, proving or key-evolution call is made from RED code; no BLUE transition takes a private key as input or produces a signature or VRF proof. Pure signing, proving and key-evolution algorithms may live in BLUE crates as library functions, provided only RED code calls them. Every signature, VRF and KES verification that decides whether a block, header or transaction is accepted is computed in BLUE code, and no RED code passes a verification verdict into a BLUE transition. |
| **Code** | `crates/ade_crypto/src/lib.rs`; `crates/ade_runtime/src/producer/keys.rs`; `crates/ade_runtime/src/producer/ade_kes_envelope.rs`; `crates/ade_runtime/src/producer/signing.rs`; `crates/ade_crypto/src/kes_sum`; `crates/ade_node/src/key_gen.rs`; `crates/ade_runtime/src/producer/producer_shell.rs`; `crates/ade_node/src/operator_forge.rs`; `crates/ade_node/src/node_lifecycle.rs` |
| **Tests** | `ade_envelope_round_trips_through_loader_at_period_0`, `ade_envelope_loader_returns_kes_at_loaded_period`, `ade_envelope_loader_rejects_signing_at_past_period`, `cardano_cli_kes_envelope_rejects_32_byte_payload`, `cardano_cli_kes_envelope_rejects_synthetic_608_byte_payload`, `cardano_cli_kes_envelope_accepts_real_608_byte_payload`, `cardano_cli_kes_envelope_rejects_612_byte_payload`, `cardano_cli_kes_envelope_rejects_608_byte_leaf_zero_payload`, `ade_envelope_loader_returns_unknown_format`, `ade_envelope_loader_returns_wrong_role`, `ade_envelope_loader_returns_unsupported_crypto`, `ade_envelope_loader_returns_missing_seed_32`, `ade_envelope_loader_returns_period_idx_overflow`, `ade_envelope_loader_returns_malformed_json`, `write_ade_kes_envelope_sets_0600_permissions`, `vrf_and_cold_cardano_cli_skey_envelopes_round_trip`, `keys_loader_rejects_wrong_envelope_type`, `keys_loader_rejects_malformed_cbor_hex`, `key_load_error_io_carries_no_path_bytes`, `ade_envelope_load_error_io_carries_no_path_bytes`, `run_key_gen_kes_writes_loadable_envelope`, `run_key_gen_kes_writes_at_loaded_period`, `run_key_gen_kes_rejects_short_seed_file`, `cardano_cli_corpus_skey_deserializes_and_vk_matches_ground_truth`, `cardano_cli_corpus_sign_then_upstream_verifies`, `shell_init_rejects_kes_period_below_opcert_start`, `op_ops_04_anchors_evolution_zero_at_opcert_start_and_evolves_to_current_period`, `op_ops_04_fails_closed_outside_window_passes_at_boundaries` |
| **CI** | `ci/ci_check_no_signing_in_blue.sh`, `ci/ci_check_private_key_custody.sh`, `ci/ci_check_kes_envelope_closed.sh`, `ci/ci_check_kes_sum_compatibility.sh` |
| **Claim** | partial — `e563ad94` (pass) — observed: tests: 28 passed, 0 failed, 0 ignored, 0 not found; CI scripts: 4 passed, 0 failed — limits: (1) No IDSE cluster has declared acceptance obligations for this invariant yet (Doctrine §10); this record re-runs the checks carried from the legacy registry. Further limits: (2) No mutation witness exists yet for this critical invariant (Doctrine §28). |

_critical · local · scope: keys, forging, verification · assumptions: — · depends on: STR-001 · secondary: STR._ Legacy: T-KEY-01

#### `SEC-002` — _declared_

| Aspect | Location |
|--------|----------|
| **Source** | `docs/planning/foundations.define.consensus.md#batch-1` (approved by wdm33, 2026-11-08) |
| **Requirement** | **BoundedPreValidationAllocation** — Before deterministic validation accepts it, an untrusted input can make the node allocate memory or do work only up to a fixed, stated bound that does not depend on length fields the input itself declares. |
| **Code** | _(no enforcement found — gap)_ |
| **Tests** | _(no tests found — gap)_ |
| **CI** | _(no CI enforcement — gap)_ |
| **Claim** | unresolved — no evidence record |

_critical · local · scope: ingress: codec, mini-protocols, block and transaction admission · assumptions: — · depends on: — · secondary: SAF._ Legacy: T-RESOURCE-01

### SEC — codec-wire-crypto

#### `SEC-003` — _partial_

| Aspect | Location |
|--------|----------|
| **Source** | `docs/planning/codec-wire-crypto.define.consensus.md#batch-1` (approved by wdm33, 2026-11-08) |
| **Requirement** | **VerificationMatchesReference** — Ed25519 signature, Praos VRF, Sum6 KES and operational-certificate verification accept exactly the signatures, proofs and inputs the reference implementation accepts, on every published and captured test vector; each verification is a pure function of its inputs, and a failed verification is final, never retried with another parser or serialization of the same input. |
| **Code** | `crates/ade_crypto/src`; `crates/ade_core/src/consensus/vrf_cert.rs`; `crates/ade_core/src/consensus/kes_check.rs` |
| **Tests** | `blake2b_256_empty`, `blake2b_256_abc`, `blake2b_256_single_byte`, `blake2b_256_multi_block`, `blake2b_256_large`, `libsodium_vector_empty_message`, `libsodium_vector_single_byte`, `libsodium_vector_two_byte`, `libsodium_vector_longer_message`, `libsodium_vector_hash_size_message`, `verify_and_proof_to_hash_match`, `generate_and_verify`, `verify_vrf_cert_accepts_valid_proof`, `verify_vrf_cert_rejects_wrong_alpha`, `verify_vrf_cert_rejects_malformed_proof`, `conway_corpus_headers_all_pass_leader_check`, `conway_corpus_nonce_contribution_is_deterministic`, `praos_malformed_vrf_proof_rejected`, `opcert_signable_format` |
| **CI** | `ci/ci_check_crypto_vectors.sh` |
| **Claim** | partial — `e563ad94` (pass) — observed: tests: 19 passed, 0 failed, 0 ignored, 0 not found; CI scripts: 1 passed, 0 failed — limits: (1) No IDSE cluster has declared acceptance obligations for this invariant yet (Doctrine §10); this record re-runs the checks carried from the legacy registry. Further limits: (2) No mutation witness exists yet for this critical invariant (Doctrine §28). |

_critical · local · scope: ade_crypto verification and the consensus checks that call it · assumptions: A2 · depends on: SEC-001 · secondary: VAL._ Legacy: DC-CRYPTO-01, CN-CRYPTO-04

#### `SEC-004` — _partial_

| Aspect | Location |
|--------|----------|
| **Source** | `docs/planning/codec-wire-crypto.define.consensus.md#batch-1` (approved by wdm33, 2026-11-08) |
| **Requirement** | **HashDomainSeparation** — Every consensus-relevant hash Ade computes uses the domain prefix and input layout the protocol defines for its purpose, so no value derived for one purpose can stand in for a value derived for another. |
| **Code** | `crates/ade_core/src/consensus/vrf_cert.rs` |
| **Tests** | `vrf_input_layout_is_41_bytes_with_correct_tag`, `vrf_role_tags_match_convention`, `vrf_input_byte_layout` |
| **CI** | _(no CI enforcement — gap)_ |
| **Claim** | partial — `e563ad94` (pass) — observed: tests: 3 passed, 0 failed, 0 ignored, 0 not found; CI scripts: 0 passed, 0 failed — limits: (1) No IDSE cluster has declared acceptance obligations for this invariant yet (Doctrine §10); this record re-runs the checks carried from the legacy registry. Further limits: (2) No mutation witness exists yet for this critical invariant (Doctrine §28). |

_critical · local · scope: consensus hashing (VRF range extension, nonces, header and body hashes) · assumptions: A2 · depends on: — · secondary: DET._ Legacy: CN-CRYPTO-02

#### `SEC-005` — _partial_

| Aspect | Location |
|--------|----------|
| **Source** | `docs/planning/codec-wire-crypto.define.consensus.md#batch-1` (approved by wdm33, 2026-11-08) |
| **Requirement** | **KesForwardSecurity** — The producer signs with the KES key evolved to exactly the period the header claims, computed relative to the operational certificate's start period; evolution only moves forward, a key evolved past a period never signs for it, a request before the key's current period, beyond its lifetime or outside the certificate's window fails closed, and the seed material of evolved-away periods is erased. |
| **Code** | `crates/ade_runtime/src/producer/signing.rs`; `crates/ade_crypto/src/kes_sum/sum.rs`; `crates/ade_runtime/src/producer/producer_shell.rs`; `crates/ade_node/src/produce_mode.rs`; `crates/ade_runtime/src/producer/coordinator.rs`; `crates/ade_runtime/src/producer/keys.rs`; `crates/ade_runtime/src/producer/ade_kes_envelope.rs`; `crates/ade_crypto/src/kes_sum`; `crates/ade_node/src/key_gen.rs`; `crates/ade_node/src/operator_forge.rs`; `crates/ade_node/src/node_lifecycle.rs` |
| **Tests** | `kes_update_chain_matches_reference`, `kes_sign_rejects_period_past_evolutions_remaining`, `kes_update_rejects_backwards_evolution`, `zeroizing_seed_drop_overwrites_bytes`, `crates/ade_runtime/src/producer/producer_shell.rs::tests::shell_kes_sign_header_advancing_evolves_then_signs`, `crates/ade_runtime/src/producer/producer_shell.rs::tests::shell_kes_sign_header_advancing_at_current_period_signs`, `crates/ade_runtime/src/producer/producer_shell.rs::tests::shell_kes_sign_header_advancing_backwards_fails_closed`, `crates/ade_runtime/src/producer/producer_shell.rs::tests::shell_kes_sign_header_advancing_beyond_lifetime_fails_closed`, `crates/ade_runtime/src/producer/coordinator.rs::tests::kes_period_for_slot_anchors_relative_to_opcert_start_period`, `ade_envelope_round_trips_through_loader_at_period_0`, `ade_envelope_loader_returns_kes_at_loaded_period`, `ade_envelope_loader_rejects_signing_at_past_period`, `cardano_cli_kes_envelope_rejects_32_byte_payload`, `cardano_cli_kes_envelope_rejects_synthetic_608_byte_payload`, `cardano_cli_kes_envelope_accepts_real_608_byte_payload`, `cardano_cli_kes_envelope_rejects_612_byte_payload`, `cardano_cli_kes_envelope_rejects_608_byte_leaf_zero_payload`, `ade_envelope_loader_returns_unknown_format`, `ade_envelope_loader_returns_wrong_role`, `ade_envelope_loader_returns_unsupported_crypto`, `ade_envelope_loader_returns_missing_seed_32`, `ade_envelope_loader_returns_period_idx_overflow`, `ade_envelope_loader_returns_malformed_json`, `write_ade_kes_envelope_sets_0600_permissions`, `vrf_and_cold_cardano_cli_skey_envelopes_round_trip`, `keys_loader_rejects_wrong_envelope_type`, `keys_loader_rejects_malformed_cbor_hex`, `key_load_error_io_carries_no_path_bytes`, `ade_envelope_load_error_io_carries_no_path_bytes`, `run_key_gen_kes_writes_loadable_envelope`, `run_key_gen_kes_writes_at_loaded_period`, `run_key_gen_kes_rejects_short_seed_file`, `cardano_cli_corpus_skey_deserializes_and_vk_matches_ground_truth`, `cardano_cli_corpus_sign_then_upstream_verifies`, `shell_init_rejects_kes_period_below_opcert_start`, `op_ops_04_anchors_evolution_zero_at_opcert_start_and_evolves_to_current_period`, `op_ops_04_fails_closed_outside_window_passes_at_boundaries` |
| **CI** | `ci/ci_check_private_key_custody.sh`, `ci/ci_check_kes_sum_compatibility.sh`, `ci/ci_check_kes_evolution_before_sign.sh`, `ci/ci_check_kes_envelope_closed.sh` |
| **Claim** | partial — `e563ad94` (pass) — observed: tests: 37 passed, 0 failed, 0 ignored, 0 not found; CI scripts: 4 passed, 0 failed — limits: (1) No IDSE cluster has declared acceptance obligations for this invariant yet (Doctrine §10); this record re-runs the checks carried from the legacy registry. Further limits: (2) No mutation witness exists yet for this critical invariant (Doctrine §28). |

_critical · local · scope: RED producer signing shell and the BLUE Sum6 KES update · assumptions: A2 · depends on: SEC-001 · secondary: SAF._ Legacy: DC-CRYPTO-05, DC-CRYPTO-10

#### `SEC-006` — _partial_

| Aspect | Location |
|--------|----------|
| **Source** | `docs/planning/codec-wire-crypto.define.consensus.md#batch-1` (approved by wdm33, 2026-11-08) |
| **Requirement** | **ClosedKeyEnvelopes** — Signing keys and operational certificates load only through closed parsers, each a deterministic function of the file's bytes, accepting exactly three grammars: the Ade KES envelope ade.kes.seed.v1 (fields format, role, crypto, seed_32, period_idx and format_version, each validated, plus the ignored metadata fields genesis_hash, network_magic, created_at_slot and created_by, and no other field); the cardano-cli text envelope of type KesSigningKey_ed25519_kes_2^6 whose cborHex is a CBOR byte string of exactly 608 bytes, a Sum6 KES signing key whose every subtree's verification-key hashes are consistent and whose leaves are not all zero; and the cardano-cli text envelope of type NodeOperationalCertificate whose cborHex is the CBOR array [[32-byte hot key, unsigned-integer sequence number, unsigned-integer KES period, 64-byte signature], 32-byte cold key]. Any other shape, role, crypto tag, version, size, arity or field type is rejected with a structured error and no fallback, and no private-key byte appears in any error, log or telemetry output. |
| **Code** | `crates/ade_runtime/src/producer/ade_kes_envelope.rs`; `crates/ade_runtime/src/producer/keys.rs`; `crates/ade_runtime/src/producer/signing.rs`; `crates/ade_crypto/src/kes_sum`; `crates/ade_node/src/key_gen.rs`; `crates/ade_runtime/src/producer/producer_shell.rs`; `crates/ade_node/src/operator_forge.rs`; `crates/ade_node/src/node_lifecycle.rs`; `crates/ade_runtime/src/producer/opcert_envelope.rs` |
| **Tests** | `parse_round_trips_serialize`, `parse_round_trips_at_nonzero_period`, `parse_rejects_unknown_format`, `parse_rejects_wrong_role`, `parse_rejects_unsupported_crypto`, `parse_rejects_unsupported_format_version`, `parse_rejects_missing_seed_32`, `parse_rejects_malformed_seed_32_length`, `parse_rejects_uppercase_seed_hex`, `parse_rejects_period_idx_overflow`, `parse_accepts_period_idx_boundary`, `parse_rejects_unknown_load_bearing_field`, `parse_accepts_optional_metadata_fields`, `parse_rejects_malformed_json`, `serialize_does_not_leak_extra_fields`, `envelope_error_carries_no_seed_bytes`, `cardano_cli_kes_envelope_rejects_32_byte_payload`, `cardano_cli_kes_envelope_rejects_synthetic_608_byte_payload`, `cardano_cli_kes_envelope_accepts_real_608_byte_payload`, `cardano_cli_kes_envelope_rejects_612_byte_payload`, `cardano_cli_kes_envelope_rejects_608_byte_leaf_zero_payload`, `cardano_cli_corpus_skey_deserializes_and_vk_matches_ground_truth`, `ade_envelope_round_trips_through_loader_at_period_0`, `ade_envelope_loader_returns_kes_at_loaded_period`, `ade_envelope_loader_rejects_signing_at_past_period`, `ade_envelope_loader_returns_unknown_format`, `ade_envelope_loader_returns_wrong_role`, `ade_envelope_loader_returns_unsupported_crypto`, `ade_envelope_loader_returns_missing_seed_32`, `ade_envelope_loader_returns_period_idx_overflow`, `ade_envelope_loader_returns_malformed_json`, `write_ade_kes_envelope_sets_0600_permissions`, `vrf_and_cold_cardano_cli_skey_envelopes_round_trip`, `keys_loader_rejects_wrong_envelope_type`, `keys_loader_rejects_malformed_cbor_hex`, `key_load_error_io_carries_no_path_bytes`, `ade_envelope_load_error_io_carries_no_path_bytes`, `run_key_gen_kes_writes_loadable_envelope`, `run_key_gen_kes_writes_at_loaded_period`, `run_key_gen_kes_rejects_short_seed_file`, `cardano_cli_corpus_sign_then_upstream_verifies`, `shell_init_rejects_kes_period_below_opcert_start`, `op_ops_04_anchors_evolution_zero_at_opcert_start_and_evolves_to_current_period`, `op_ops_04_fails_closed_outside_window_passes_at_boundaries`, `accepted_envelope_decodes_to_expected_opcert`, `malformed_type_envelope_emits_wrong_envelope_type`, `malformed_cbor_hex_envelope_emits_malformed_cbor_hex`, `wrong_arity_envelope_emits_malformed_cbor`, `parser_is_byte_identical_across_two_runs` |
| **CI** | `ci/ci_check_kes_envelope_closed.sh`, `ci/ci_check_kes_sum_compatibility.sh`, `ci/ci_check_private_key_custody.sh`, `ci/ci_check_node_forge_real_cli_ingress.sh` |
| **Claim** | partial — `e563ad94` (pass) — observed: tests: 49 passed, 0 failed, 0 ignored, 0 not found; CI scripts: 4 passed, 0 failed — limits: (1) No IDSE cluster has declared acceptance obligations for this invariant yet (Doctrine §10); this record re-runs the checks carried from the legacy registry. Further limits: (2) No mutation witness exists yet for this critical invariant (Doctrine §28). |

_critical · local · scope: RED key and certificate loading · assumptions: A2, A3 · depends on: SEC-001 · secondary: VAL._ Legacy: DC-CRYPTO-06, DC-CRYPTO-07, CN-OPCERT-01, DC-OPCERT-01

#### `SEC-007` — _partial_

| Aspect | Location |
|--------|----------|
| **Source** | `docs/planning/codec-wire-crypto.define.consensus.md#batch-1` (approved by wdm33, 2026-11-08) |
| **Requirement** | **HeaderPreImageSingleSource** — A forged header's KES signature is over exactly the header-body bytes a validator extracts from the resulting block: one BLUE recipe builds the pre-image, it gives byte-identical output to the validator's extraction for every corpus block, and the signer accepts only a pre-image built by that recipe. |
| **Code** | `crates/ade_ledger/src/block_validity/unsigned_header_pre_image.rs`; `crates/ade_runtime/src/producer/producer_shell.rs`; `crates/ade_node/src/produce_mode.rs` |
| **Tests** | `unsigned_header_preimage_matches_decode_block_extraction_for_corpus`, `recipe_output_is_byte_identical_across_two_runs`, `shell_kes_sign_header_produces_verifiable_signature` |
| **CI** | `ci/ci_check_unsigned_header_preimage_single_source.sh` |
| **Claim** | partial — `e563ad94` (pass) — observed: tests: 3 passed, 0 failed, 0 ignored, 0 not found; CI scripts: 1 passed, 0 failed — limits: (1) No IDSE cluster has declared acceptance obligations for this invariant yet (Doctrine §10); this record re-runs the checks carried from the legacy registry. Further limits: (2) No mutation witness exists yet for this critical invariant (Doctrine §28). |

_critical · local · scope: forging (pre-image recipe, signer) and header validation (extraction) · assumptions: A2 · depends on: DET-002, SEC-005 · secondary: VAL, STR._ Legacy: CN-KES-HEADER-01, DC-KES-HEADER-01, CN-PREIMAGE-FIXTURE-01

### SEC — ledger

#### `SEC-008` — _partial_

| Aspect | Location |
|--------|----------|
| **Source** | `docs/planning/ledger.define.consensus.md#batch-1` (approved by wdm33, 2026-11-08); 1 recorded redefinition(s) in the registry |
| **Requirement** | **WitnessesBindTransaction** — A Conway transaction is accepted only if every key authority it requires has a key or bootstrap witness that verifies over the transaction's preserved body hash, where the required key authorities are the closed set of the key-hash payment credentials and bootstrap addresses of its resolved spending and collateral inputs, its explicit required signers, and the key-hash credentials its certificates, withdrawals, votes and proposals require; every key and bootstrap witness it supplies, required or not, verifies under its own key over that hash. Script-hash credentials are authorized by script witnesses, not keys (SEC-010). |
| **Code** | `crates/ade_ledger/src/witness.rs`; `crates/ade_ledger/src/scripts.rs`; `crates/ade_plutus/src/evaluator.rs`; `crates/ade_ledger/src/tx_validity/witness.rs`; `crates/ade_ledger/src/tx_validity/required_signers.rs`; `crates/ade_ledger/src/rules.rs`; `crates/ade_ledger/src/tx_validity/phase1.rs` |
| **Tests** | `witness_info_no_plutus`, `witness_info_plutus_detection`, `empty_witness_set`, `aiken_fixture_tx_evaluates_end_to_end`, `all_required_covered_is_valid`, `signature_over_wrong_body_rejected`, `witness_correct_key_wrong_body_rejected`, `wrong_size_signature_rejected`, `wrong_size_vkey_rejected`, `extra_irrelevant_witness_does_not_substitute`, `missing_certificate_witness_rejected`, `missing_withdrawal_witness_rejected`, `missing_governance_voter_witness_rejected`, `missing_input_payment_witness_rejected`, `missing_explicit_required_signer_rejected`, `unresolvable_input_is_fail_fast`, `unresolvable_collateral_input_is_fail_fast`, `script_credential_input_not_a_vkey_signer`, `script_credential_certificate_not_a_vkey_signer`, `collateral_payment_key_is_required`, `non_conway_era_is_unsupported`, `closure_is_deterministic`, `lv14_a_bad_signature_on_a_non_required_witness_rejects_in_both_modes`, `lv14_required_signer_failures_keep_their_own_errors`, `lv14_wrong_size_keys_and_signatures_are_malformed`, `lv14_a_native_script_on_a_badly_signed_key_does_not_carry_the_tx`, `lv14_a_real_bootstrap_witness_verifies_and_a_flipped_one_rejects_in_both_paths`, `lv14_a_real_vkey_signature_flipped_gives_the_same_error_on_both_paths` |
| **CI** | `ci/ci_check_required_signer_closure.sh`, `ci/ci_check_consensus_closed_enums.sh`, `ci/ci_check_every_supplied_witness_verifies.sh` |
| **Claim** | partial — `e563ad94` (pass) — observed: tests: 27 passed, 0 failed, 0 ignored, 1 not found; CI scripts: 3 passed, 0 failed — limits: (1) No IDSE cluster has declared acceptance obligations for this invariant yet (Doctrine §10); this record re-runs the checks carried from the legacy registry. Further limits: (2) No mutation witness exists yet for this critical invariant (Doctrine §28). (3) 1 named test(s) were not found in the test output at this revision: aiken_fixture_tx_evaluates_end_to_end |

_critical · local · scope: transaction witnesses (block path and standalone) · assumptions: A2 · depends on: DET-002, SEC-003, SEC-010 · secondary: VAL._ Legacy: DC-LEDGER-05, CN-LEDGER-09, DC-TXV-05, DC-TXV-08

#### `SEC-009` — _partial_

| Aspect | Location |
|--------|----------|
| **Source** | `docs/planning/ledger.define.consensus.md#batch-1` (approved by wdm33, 2026-11-08) |
| **Requirement** | **BoundedStackDepth** — No code path that decodes, skips, evaluates, clones, compares or drops a wire-supplied native script, or skips any wire-supplied CBOR item, uses call-stack depth that grows with the input's nesting depth; pending work is kept on the heap, and no nesting limit below what transaction and block size allow is imposed. |
| **Code** | `crates/ade_codec/src/cbor/mod.rs`; `crates/ade_types/src/allegra/script.rs`; `crates/ade_codec/src/allegra/script.rs`; `crates/ade_ledger/src/scripts.rs` |
| **Tests** | `lv12_the_deep_preprod_block_decodes_and_evaluates_on_a_256_kib_thread`, `lv12_the_deep_preprod_script_is_what_the_recursive_reference_decodes`, `lv12_a_script_as_deep_as_a_transaction_allows_round_trips_on_a_256_kib_thread`, `lv12_a_deep_script_inside_a_witness_set_decodes_on_a_256_kib_thread`, `lv12_every_fixture_script_decodes_and_evaluates_as_the_reference_does`, `lv12_generated_scripts_decode_and_evaluate_as_the_reference_does`, `lv12_malformed_scripts_fail_exactly_as_the_reference_does`, `lv12_a_forged_sub_script_count_is_an_error_not_an_allocation`, `lv12_the_flat_type_has_no_recursive_field`, `lv12_skip_item_matches_the_recursive_reference_on_every_fixture_and_inner_offset`, `lv12_skip_item_matches_the_recursive_reference_on_every_truncated_prefix`, `lv12_skip_item_matches_the_recursive_reference_on_constructed_forms`, `lv12_skip_item_walks_ninety_thousand_levels_on_a_256_kib_thread`, `a_deep_script_drops_clones_and_compares_on_a_small_stack`, `from_post_order_accepts_exactly_one_tree` |
| **CI** | `ci/ci_check_native_script_depth_independent.sh` |
| **Claim** | partial — `e563ad94` (pass) — observed: tests: 15 passed, 0 failed, 0 ignored, 0 not found; CI scripts: 1 passed, 0 failed — limits: (1) No IDSE cluster has declared acceptance obligations for this invariant yet (Doctrine §10); this record re-runs the checks carried from the legacy registry. Further limits: (2) No mutation witness exists yet for this critical invariant (Doctrine §28). |

_critical · local · scope: CBOR skipping, native-script decoding and evaluation · assumptions: A2 · depends on: SEC-002 · secondary: SAF._ Legacy: DC-LEDGER-SCRIPT-02

#### `SEC-010` — _partial_

| Aspect | Location |
|--------|----------|
| **Source** | `docs/planning/ledger.define.consensus.md#batch-1` (approved by wdm33, 2026-11-08) |
| **Requirement** | **ScriptWitnessClosureSingleAuthority** — Whether a Conway transaction's script witnesses are sufficient and not extraneous is decided by one authority, which both the block path and the standalone transaction path call: with every spending and reference input resolved it gives the reference node's verdict; without full UTxO context it rejects exactly when a supplied native script fails evaluation and otherwise returns undetermined, never valid. |
| **Code** | `crates/ade_ledger/src/tx_validity/script_witnesses.rs`; `crates/ade_ledger/src/scripts.rs`; `crates/ade_ledger/src/witness.rs`; `crates/ade_ledger/src/tx_validity/required_signers.rs`; `crates/ade_ledger/src/rules.rs`; `crates/ade_ledger/src/tx_validity/phase1.rs`; `crates/ade_crypto/src/sha3.rs` |
| **Tests** | `lv13_a_needed_passing_native_script_is_accepted`, `lv13_the_same_tx_with_a_failing_native_script_is_rejected_at_both_levels`, `lv13_failing_all_any_and_n_of_k_each_reject`, `lv13_a_threshold_of_zero_or_below_passes_exactly_as_cardano`, `lv13_invalid_before_holds_from_the_lock_and_never_without_a_start`, `lv13_invalid_hereafter_holds_up_to_the_lock_and_never_without_an_expiry`, `lv13_the_block_slot_does_not_decide_a_timelock`, `lv13_a_missing_required_native_script_is_missing`, `lv13_a_reference_script_satisfies_the_need_and_a_witness_copy_is_extraneous`, `lv13_a_failing_needed_reference_native_script_does_not_validate`, `lv13_an_extra_passing_script_is_undetermined_without_context_and_extraneous_with_it`, `lv13_an_extra_failing_script_is_a_definite_reject_and_exactly_extraneous`, `lv13_the_needed_set_covers_every_conway_purpose`, `lv13_a_native_script_hash_is_over_its_own_bytes_not_a_re_encoding`, `lv13_a_plutus_witness_counts_toward_the_closure_but_is_not_evaluated_here`, `lv13_a_script_ref_decodes_to_the_hash_of_its_native_bytes`, `lv13_a_real_timelock_mint_is_exactly_valid`, `lv13_a_real_reference_script_multisig_spend_is_exactly_valid`, `lv13_a_real_bootstrap_witness_key_hash_is_its_byron_address_root`, `lv13_the_block_path_accepts_the_real_block_and_rejects_it_with_a_failing_timelock`, `lv13_the_mutated_real_script_is_exactly_missing_and_extraneous`, `lv13_verdicts_are_deterministic`, `invalid_before_holds_when_the_tx_starts_at_or_after_the_lock`, `invalid_before_never_holds_without_a_tx_start`, `invalid_hereafter_holds_when_the_tx_expires_at_or_before_the_lock`, `invalid_hereafter_never_holds_without_a_tx_expiry`, `fips_202_vectors` |
| **CI** | `ci/ci_check_native_script_witness_verdict.sh` |
| **Claim** | partial — `e563ad94` (pass) — observed: tests: 27 passed, 0 failed, 0 ignored, 0 not found; CI scripts: 1 passed, 0 failed — limits: (1) No IDSE cluster has declared acceptance obligations for this invariant yet (Doctrine §10); this record re-runs the checks carried from the legacy registry. Further limits: (2) No mutation witness exists yet for this critical invariant (Doctrine §28). |

_critical · local · scope: script witness closure (block path, standalone, reduced mode) · assumptions: A2 · depends on: VAL-009, VAL-017 · secondary: VAL._ Legacy: DC-LEDGER-SCRIPT-03

### SEC — consensus

#### `SEC-011` — _partial_

| Aspect | Location |
|--------|----------|
| **Source** | `docs/planning/consensus.define.consensus.md#batch-1` (approved by wdm33, 2026-11-08) |
| **Requirement** | **OpCertCounterRule** — A header is valid only under the reference OCERT rule: its pool's current issue number is the one recorded in the chain-dependent state, or 0 when none is recorded and the pool is in the epoch's stake distribution, and a header from a pool with neither is invalid; the header's operational-certificate issue number must be at least the current one and at most one more than it, otherwise the header is invalid with a typed reason. Counters are keyed by pool; a forward header transition never lowers a pool's recorded number, and a rollback or verified snapshot restore sets the counters to those of the restored point. |
| **Code** | `crates/ade_core/src/consensus/op_cert.rs`; `crates/ade_core/src/consensus/praos_state.rs`; `crates/ade_core/src/consensus/errors.rs`; `crates/ade_core/src/consensus/header_validate.rs`; `crates/ade_core/src/consensus/opcert_validate.rs` |
| **Tests** | `consensus::op_cert::tests::apply_op_cert_inserts_first_observation`, `consensus::op_cert::tests::apply_op_cert_advances_existing_strictly`, `consensus::op_cert::tests::apply_op_cert_accepts_equal_counter_as_noop`, `consensus::op_cert::tests::apply_op_cert_rejects_lower_counter`, `consensus::op_cert::tests::apply_op_cert_independent_kes_periods_dont_collide`, `consensus::op_cert::tests::apply_op_cert_independent_pools_dont_collide`, `consensus::op_cert::tests::apply_op_cert_does_not_touch_nonces`, `consensus::op_cert::tests::apply_op_cert_does_not_touch_last_slot_or_block_no`, `consensus::praos_state::tests::op_cert_upsert_rejects_regression`, `consensus::praos_state::tests::op_cert_upsert_accepts_equal_counter_as_noop`, `consensus::praos_state::tests::op_cert_upsert_accepts_monotonic_increasing`, `normal_progression_records_highest_counter_per_window`, `regression_after_progression_rejected_with_typed_error`, `op_cert_replay_is_deterministic`, `header_with_op_cert_regression_rejected`, `valid_header_accepted_advances_state`, `opcert_validate_rejects_counter_regression`, `opcert_validate_rejects_counter_repeat`, `opcert_validate_rejects_bad_signature_over_cold_key` |
| **CI** | `ci/ci_check_consensus_closed_enums.sh`, `ci/ci_check_opcert_closed.sh` |
| **Claim** | partial — `e563ad94` (pass) — observed: tests: 19 passed, 0 failed, 0 ignored, 0 not found; CI scripts: 2 passed, 0 failed — limits: (1) No IDSE cluster has declared acceptance obligations for this invariant yet (Doctrine §10); this record re-runs the checks carried from the legacy registry. Further limits: (2) No mutation witness exists yet for this critical invariant (Doctrine §28). (3) Forge-time gap (pipeline-production S6): the forge engine's own counter check receives no previous counter in node mode, so the reference counter window binds forged blocks only through self-acceptance. |

_critical · local · scope: header validation, chain-dependent state · assumptions: A2 · depends on: STR-008 · secondary: VAL._ Legacy: DC-CONS-10

#### `SEC-012` — _partial_

| Aspect | Location |
|--------|----------|
| **Source** | `docs/planning/consensus.define.consensus.md#batch-1` (approved by wdm33, 2026-11-08) |
| **Requirement** | **AnnouncedHeaderMatchesDelivered** — When a peer delivers a block, the header bytes in the delivered block are byte-for-byte equal to the header bytes that peer announced for the same slot and hash; a peer cannot switch headers between announcement and delivery. |
| **Code** | `crates/ade_ledger/src/receive/reducer.rs` |
| **Tests** | `receive_apply_block_delivered_with_no_cached_header_rejects`, `receive_apply_block_delivered_with_mismatched_cached_header_rejects`, `receive_apply_block_delivered_with_matching_header_admits` |
| **CI** | `ci/ci_check_receive_reducer_closure.sh` |
| **Claim** | partial — `e563ad94` (pass) — observed: tests: 3 passed, 0 failed, 0 ignored, 0 not found; CI scripts: 1 passed, 0 failed — limits: (1) No IDSE cluster has declared acceptance obligations for this invariant yet (Doctrine §10); this record re-runs the checks carried from the legacy registry. Further limits: (2) No mutation witness exists yet for this critical invariant (Doctrine §28). |

_critical · local · scope: receive path (chain-sync announcement, block-fetch delivery) · assumptions: — · depends on: VAL-032 · secondary: VAL._ Legacy: DC-CONS-19

#### `SEC-013` — _partial_

| Aspect | Location |
|--------|----------|
| **Source** | `docs/planning/consensus.define.consensus.md#batch-1` (approved by wdm33, 2026-11-08) |
| **Requirement** | **RolloutSkewCannotSplit** — No input, adversarial or not, can lead two supported Ade releases authorized to run on the network at the same time, starting from the same state and receiving the same inputs, to different authoritative states or different selected chains. |
| **Code** | `crates/ade_core/src/consensus/fork_choice.rs` |
| **Tests** | `reject_reason_bytes_are_stable`, `replay_is_deterministic` |
| **CI** | `ci/ci_check_no_density_in_fork_choice.sh`, `ci/ci_check_consensus_closed_enums.sh` |
| **Claim** | partial — `e563ad94` (pass) — observed: tests: 2 passed, 0 failed, 0 ignored, 0 not found; CI scripts: 2 passed, 0 failed — limits: (1) No IDSE cluster has declared acceptance obligations for this invariant yet (Doctrine §10); this record re-runs the checks carried from the legacy registry. Further limits: (2) No mutation witness exists yet for this critical invariant (Doctrine §28). |

_critical · distributed · scope: Ade release compatibility · assumptions: A2 · depends on: EVO-003, DET-001 · secondary: EVO._ Legacy: CN-CONS-02

### SEC — consensus-inputs

#### `SEC-014` — _partial_

| Aspect | Location |
|--------|----------|
| **Source** | `docs/planning/consensus-inputs.define.consensus.md#batch-1` (approved by wdm33, 2026-11-08) |
| **Requirement** | **ConsensusInputsFromVerifiedBootstrapOnly** — The persisted seed-epoch consensus-input record is written only by the verified-bootstrap paths (genesis, Mithril and the admission pre-seed), through one persistence authority; the forge and feed paths never build or write it, never construct a value of its type, and never take a consensus view from an operator-supplied bundle. Leader checks in every epoch read only the sealed leadership object for exactly that epoch (VAL-024); the bootstrap epoch's object is sealed from the certified seed record's pool distribution, the next epoch's from the certified snapshot's imported mark, and every later epoch's from canonical chain state at a boundary the node crossed (VAL-023). |
| **Code** | `crates/ade_runtime/src/seed_epoch_lineage.rs`; `crates/ade_runtime/src/genesis_bootstrap.rs`; `crates/ade_runtime/src/mithril_bootstrap.rs`; `crates/ade_node/src/admission/bootstrap.rs`; `crates/ade_runtime/src/seed_consensus_merge.rs`; `crates/ade_runtime/src/seed_consensus_provenance.rs`; `ci/ci_check_consensus_input_provenance.sh`; `crates/ade_node/src/node_sync.rs` |
| **Tests** | `bootstrap_persists_anchor_keyed_seed_consensus_inputs`, `bootstrap_seed_inputs_merge_fails_closed_on_missing_vrf_or_stake`, `snapshot_store_keyed_sidecar_is_disjoint_from_slot_snapshots`, `persist_writes_anchor_keyed_sidecar_and_recoverable_wal_provenance`, `forge_from_recovered_uses_recovered_pool_distr`, `forge_from_recovered_fails_closed_without_recovered_inputs` |
| **CI** | `ci/ci_check_consensus_input_provenance.sh` (red at e563ad94) |
| **Claim** | unresolved — `e563ad94` (fail) — observed: tests: 6 passed, 0 failed, 0 ignored, 0 not found; CI scripts: 0 passed, 1 failed; failing CI: ci/ci_check_consensus_input_provenance.sh — limits: (1) No IDSE cluster has declared acceptance obligations for this invariant yet (Doctrine §10); this record re-runs the checks carried from the legacy registry. Further limits: (2) No mutation witness exists yet for this critical invariant (Doctrine §28). (3) ci/ci_check_consensus_input_provenance.sh fails at this revision and already failed before the IDSE migration (a3cef7f7): "FAIL: production file crates/ade_runtime/src/mithril_bootstrap.rs references merge_seed_epoch_consensus_inputs( outside the shared populator authority ([legacy ref])". Diagnosed as a stale or over-broad structural gate, not a demonstrated violation; the gate needs repair before it can serve as evidence. |

_critical · local · scope: seed-epoch record writers; forge and feed consensus views · assumptions: — · depends on: VAL-034, VAL-023, VAL-024 · secondary: STR._ Legacy: CN-CINPUT-02, CN-CINPUT-03

### SEC — durability-recovery

#### `SEC-015` — _declared_

| Aspect | Location |
|--------|----------|
| **Source** | `docs/planning/durability-recovery.define.consensus.md#batch-1` (approved by wdm33, 2026-11-08) |
| **Requirement** | **StorageAfterAnchorVerification** — No authoritative store is initialized or written before the bootstrap source or anchor it derives from has been verified; a verification failure leaves no authoritative store behind. |
| **Code** | _(no enforcement found — gap)_ |
| **Tests** | _(no tests found — gap)_ |
| **CI** | _(no CI enforcement — gap)_ |
| **Claim** | unresolved — no evidence record |

_critical · transactional · scope: bootstrap, store initialization · assumptions: — · depends on: — · secondary: SAF._ Legacy: CN-STORE-01

### SEC — network

#### `SEC-016` — _partial_

| Aspect | Location |
|--------|----------|
| **Source** | `docs/planning/network.define.consensus.md#batch-1` (approved by wdm33, 2026-11-08) |
| **Requirement** | **AgencyEnforced** — Every mini-protocol enforces agency strictly: a message from the side that does not hold agency in the current state is rejected deterministically and ends the session. Beyond that runtime check, the node's own code cannot construct a message for a role or agency it does not hold: the serving side's public API can build only server-agency messages, and the receive reducer's public API offers no way to construct or admit a locally originated or client-output event. |
| **Code** | `crates/ade_network/src/chain_sync/server.rs`; `crates/ade_network/src/block_fetch/server.rs`; `crates/ade_ledger/src/block_validity/header_input.rs`; `crates/ade_ledger/src/receive/events.rs` |
| **Tests** | `chain_sync_server_reply_round_trips_through_codec`, `chain_sync_server_reply_into_message_only_yields_server_variants`, `block_fetch_server_reply_round_trips_through_codec`, `block_fetch_server_reply_into_message_only_yields_server_variants`, `accepted_block_header_bytes_equals_validator_split_on_corpus`, `accepted_block_header_bytes_is_subslice_of_as_bytes`, `accepted_block_header_bytes_rejects_malformed_envelope`, `receive_event_round_trips_through_pattern_match`, `receive_effect_round_trips_through_pattern_match`, `receive_error_round_trips_through_pattern_match` |
| **CI** | `ci/ci_check_no_parallel_header_splitter.sh`, `ci/ci_check_admitted_block_closure.sh` |
| **Claim** | partial — `e563ad94` (pass) — observed: tests: 10 passed, 0 failed, 0 ignored, 0 not found; CI scripts: 2 passed, 0 failed — limits: (1) No IDSE cluster has declared acceptance obligations for this invariant yet (Doctrine §10); this record re-runs the checks carried from the legacy registry. Further limits: (2) No mutation witness exists yet for this critical invariant (Doctrine §28). |

_critical · local · scope: mini-protocol sessions, serve and receive paths · assumptions: A2 · depends on: DET-016, EVO-002 · secondary: VAL, SAF._ Legacy: CN-PROTO-03, CN-PROTO-06, CN-PROTO-07

#### `SEC-017` — _partial_

| Aspect | Location |
|--------|----------|
| **Source** | `docs/planning/network.define.consensus.md#batch-1` (approved by wdm33, 2026-11-08) |
| **Requirement** | **PerPeerOutboundIsolation** — Outbound messages reach a peer only as typed commands through that peer's own outbound channel and the one session-aware encoder; no raw byte tunnel exists, no code can create a channel for a peer that did not connect, a peer's channel is removed when it disconnects, the channel map iterates in a deterministic order, looking up a missing peer is a structured error, bytes meant for one peer never reach another, and commands for a peer arrive in the order they were sent. |
| **Code** | `crates/ade_runtime/src/network/outbound_command.rs`; `crates/ade_runtime/src/network/mux_pump.rs`; `crates/ade_node/src/produce_mode.rs`; `crates/ade_runtime/src/network/n2n_listener.rs` |
| **Tests** | `outbound_command_peer_accessor_returns_target_peer`, `outbound_command_carries_typed_reply_not_raw_bytes` |
| **CI** | `ci/ci_check_no_produce_mode_direct_transport_writes.sh` |
| **Claim** | partial — `e563ad94` (pass) — observed: tests: 2 passed, 0 failed, 0 ignored, 0 not found; CI scripts: 1 passed, 0 failed — limits: (1) No IDSE cluster has declared acceptance obligations for this invariant yet (Doctrine §10); this record re-runs the checks carried from the legacy registry. Further limits: (2) No mutation witness exists yet for this critical invariant (Doctrine §28). |

_critical · local · scope: outbound dispatch · assumptions: — · depends on: DET-017 · secondary: ORD._ Legacy: CN-OUTBOUND-RELAY-01, CN-PEER-OUTBOUND-MAP-01, DC-OUTBOUND-FIFO-01

#### `SEC-018` — _partial_

| Aspect | Location |
|--------|----------|
| **Source** | `docs/planning/network.define.consensus.md#batch-1` (approved by wdm33, 2026-11-08) |
| **Requirement** | **BoundedBackpressure** — Every per-peer and per-mini-protocol channel in the wire session, mux pump, dialer and keep-alive is bounded, and overflow is a fail-fast error, never an unbounded queue or a silent drop. |
| **Code** | `crates/ade_network/src/mux/transport.rs`; `crates/ade_runtime/src/network/mux_pump.rs`; `crates/ade_runtime/src/network/n2n_dialer.rs` |
| **Tests** | `crates/ade_network/src/mux/transport.rs::tests::mux_transport_duplex_inbound_overflow_returns_backpressure`, `crates/ade_network/src/mux/transport.rs::tests::mux_transport_duplex_round_trips_bytes_over_loopback` |
| **CI** | `ci/ci_check_session_no_unbounded.sh` |
| **Claim** | partial — `e563ad94` (pass) — observed: tests: 2 passed, 0 failed, 0 ignored, 0 not found; CI scripts: 1 passed, 0 failed — limits: (1) No IDSE cluster has declared acceptance obligations for this invariant yet (Doctrine §10); this record re-runs the checks carried from the legacy registry. Further limits: (2) No mutation witness exists yet for this critical invariant (Doctrine §28). |

_critical · local · scope: wire session channels · assumptions: — · depends on: SEC-002 · secondary: SAF._ Legacy: DC-SESS-04

### SEC — node

#### `SEC-019` — _partial_

| Aspect | Location |
|--------|----------|
| **Source** | `docs/planning/node.define.consensus.md#batch-1` (approved by wdm33, 2026-11-09) |
| **Requirement** | **PerPeerSessionIsolation** — A failure in one peer's session (a decode error, a validity rejection, a too-deep rollback, a protocol violation) ends only that peer's session; the node keeps serving other peers and producing blocks, and peer sessions share no working state. |
| **Code** | `crates/ade_ledger/src/receive/admitted.rs`; `crates/ade_ledger/src/receive/reducer.rs`; `crates/ade_ledger/src/receive/chain_write.rs`; `crates/ade_runtime/src/orchestrator/peer_session.rs`; `crates/ade_runtime/src/orchestrator/core.rs` |
| **Tests** | `admit_via_block_validity_accepts_corpus_block`, `admit_via_block_validity_rejects_corrupted_body`, `receive_apply_block_delivered_with_matching_header_admits`, `receive_apply_block_delivered_validity_invalid_rejects`, `receive_apply_rollback_returns_out_of_scope`, `receive_apply_replay_byte_identical_over_corpus`, `crates/ade_runtime/tests/orchestrator_peer_isolation.rs::peer_session_isolation_holds_under_failure`, `crates/ade_runtime/tests/orchestrator_peer_isolation.rs::peer_session_per_peer_state_does_not_cross`, `crates/ade_runtime/tests/orchestrator_peer_isolation.rs::peer_disconnect_removes_only_that_peer`, `crates/ade_runtime/src/orchestrator/peer_session.rs::tests::peer_session_isolation_across_two_concurrent_tasks`, `crates/ade_runtime/src/orchestrator/core.rs::tests::step_per_peer_decode_error_isolates` |
| **CI** | `ci/ci_check_admitted_block_closure.sh`, `ci/ci_check_receive_reducer_closure.sh`, `ci/ci_check_peer_session_isolation.sh` |
| **Claim** | partial — `e563ad94` (pass) — observed: tests: 11 passed, 0 failed, 0 ignored, 0 not found; CI scripts: 3 passed, 0 failed — limits: (1) No IDSE cluster has declared acceptance obligations for this invariant yet (Doctrine §10); this record re-runs the checks carried from the legacy registry. Further limits: (2) No mutation witness exists yet for this critical invariant (Doctrine §28). |

_critical · local · scope: peer sessions · assumptions: — · depends on: SEC-017 · secondary: LIV._ Legacy: DC-NODE-01

#### `SEC-020` — _partial_

| Aspect | Location |
|--------|----------|
| **Source** | `docs/planning/node.define.consensus.md#batch-1` (approved by wdm33, 2026-11-09); 1 recorded redefinition(s) in the registry |
| **Requirement** | **RollbackTargetFromStoredPoint** — For a peer's RollBackward on the participant path, the rollback target is the stored chain point for the named hash, and the peer's slot must equal the stored slot; apart from the exact persisted-anchor case of SAF-016, which is evaluated first, an unknown hash, a slot mismatch or Origin fails closed before any rollback, log entry or state change, and no target is ever built from a peer slot combined with a local hash. |
| **Code** | `crates/ade_node/src/node_lifecycle.rs`; `crates/ade_runtime/src/chaindb` |
| **Tests** | `rollback_slot_hash_mismatch_fails_before_mutation`, `participant_rollback_applies_durably`, `participant_rollback_to_unknown_point_fails_closed`, `forge_path_rollback_slot_hash_mismatch_fails_before_mutation`, `forge_path_rollback_to_unknown_point_fails_closed` |
| **CI** | `ci/ci_check_rollback_target_canonical_binding.sh` (red at e563ad94) |
| **Claim** | unresolved — `e563ad94` (fail) — observed: tests: 5 passed, 0 failed, 0 ignored, 0 not found; CI scripts: 0 passed, 1 failed; failing CI: ci/ci_check_rollback_target_canonical_binding.sh — limits: (1) No IDSE cluster has declared acceptance obligations for this invariant yet (Doctrine §10); this record re-runs the checks carried from the legacy registry. Further limits: (2) No mutation witness exists yet for this critical invariant (Doctrine §28). (3) ci/ci_check_rollback_target_canonical_binding.sh fails at this revision and already failed before the IDSE migration (a3cef7f7): "FAIL: to_point must be built from stored.slot (the durable authority), not the wire slot". Diagnosed as a stale or over-broad structural gate, not a demonstrated violation; the gate needs repair before it can serve as evidence. |

_critical · local · scope: live rollback · assumptions: — · depends on: SAF-007 · secondary: VAL._ Legacy: DC-NODE-29

#### `SEC-021` — _partial_

| Aspect | Location |
|--------|----------|
| **Source** | `docs/planning/node.define.consensus.md#batch-1` (approved by wdm33, 2026-11-09) |
| **Requirement** | **CandidatesFromOwnValidatedHeaders** — Every candidate given to chain selection is built from headers Ade itself validated, never from a peer-supplied summary or a raw followed-peer tip; candidates keep their peer identity, the candidate set is ordered deterministically, and the one chain-selection authority receives it, with no second selector. |
| **Code** | `crates/ade_node/src/candidate_aggregator.rs`; `ci/ci_check_candidate_construction_validated.sh`; `crates/ade_node/src/node_lifecycle.rs`; `crates/ade_node/src/selector_state.rs`; `ci/ci_check_live_selector_dispatch.sh` |
| **Tests** | `build_candidate_fragment_assembles_from_validated_headers`, `build_candidate_fragment_empty_headers_fails_closed`, `build_candidate_fragment_rejects_invalid_header_fails_closed`, `build_candidate_fragment_two_runs_byte_identical`, `assemble_candidate_set_ordering_is_arrival_independent`, `win_emits_switch_to_winning_peer_and_durable_anchor`, `tiebreaker_loss_keeps_current`, `exceeded_rollback_keeps_current`, `best_of_two_peers_wins_and_is_identified` |
| **CI** | `ci/ci_check_candidate_construction_validated.sh`, `ci/ci_check_live_selector_dispatch.sh` |
| **Claim** | partial — `e563ad94` (pass) — observed: tests: 9 passed, 0 failed, 0 ignored, 0 not found; CI scripts: 2 passed, 0 failed — limits: (1) No IDSE cluster has declared acceptance obligations for this invariant yet (Doctrine §10); this record re-runs the checks carried from the legacy registry. Further limits: (2) No mutation witness exists yet for this critical invariant (Doctrine §28). |

_critical · local · scope: live fork choice · assumptions: — · depends on: VAL-031, DET-012 · secondary: VAL._ Legacy: DC-NODE-35, DC-NODE-36

#### `SEC-022` — _partial_

| Aspect | Location |
|--------|----------|
| **Source** | `docs/planning/node.define.consensus.md#batch-1` (approved by wdm33, 2026-11-09) |
| **Requirement** | **ForkSwitchNeverAbandons** — When chain selection picks a branch that forks below the durable tip, no rollback of the current chain is committed until the replacement branch's bodies are fetched, linked and validated as a complete branch from a fork anchor bound to Ade's stored chain point; a failed, incomplete or lying branch leaves the chain database, ledger and chain-dependent state unchanged. |
| **Code** | `crates/ade_node/src/fork_switch.rs`; `crates/ade_node/src/node_lifecycle.rs`; `ci/ci_check_fork_switch_never_abandons.sh` |
| **Tests** | `empty_branch_fails_closed_before_any_apply`, `null_source_serves_nothing`, `fork_switch_win_adopts_via_rolledback_then_chainselected`, `body_hash_mismatch_leaves_chain_unchanged`, `broken_parent_link_leaves_chain_unchanged`, `selected_peer_missing_body_leaves_chain_unchanged_fence_held`, `proof_failure_holds_fence_then_resolves_when_caught_up` |
| **CI** | `ci/ci_check_fork_switch_never_abandons.sh` |
| **Claim** | partial — `e563ad94` (pass) — observed: tests: 7 passed, 0 failed, 0 ignored, 0 not found; CI scripts: 1 passed, 0 failed — limits: (1) No IDSE cluster has declared acceptance obligations for this invariant yet (Doctrine §10); this record re-runs the checks carried from the legacy registry. Further limits: (2) No mutation witness exists yet for this critical invariant (Doctrine §28). |

_critical · transactional · scope: live fork switch · assumptions: A4, A6 · depends on: SEC-020, STR-016 · secondary: SAF._ Legacy: DC-NODE-37

#### `SEC-023` — _partial_

| Aspect | Location |
|--------|----------|
| **Source** | `docs/planning/node.define.consensus.md#batch-1` (approved by wdm33, 2026-11-09) |
| **Requirement** | **ForkAnchorWithinKValidated** — A competing branch is eligible for selection only when its parent links lead back to a durably stored fork anchor, with both the number of headers traversed and the distance from the durable tip's block number to the anchor's at most k blocks, and every header from that anchor to the branch tip has been validated; each cached header entry is bound to its own recomputed hash, slot and parent, and the anchor counts only when the chain database confirms its slot and hash. |
| **Code** | `crates/ade_node/src/lca_walk.rs`; `ci/ci_check_lca_anchor_walk.sh` |
| **Tests** | `one_block_fork_walks_in_one_step`, `multi_block_branch_walks_to_durable_lca`, `missing_intermediate_header_fails_closed`, `ancestor_older_than_k_fails_closed_block_depth`, `cache_self_binding_violation_fails_closed`, `lying_parent_link_to_genesis_fails_closed`, `arrival_order_permutation_walks_identical`, `walk_is_deterministic` |
| **CI** | `ci/ci_check_lca_anchor_walk.sh` |
| **Claim** | partial — `e563ad94` (pass) — observed: tests: 8 passed, 0 failed, 0 ignored, 0 not found; CI scripts: 1 passed, 0 failed — limits: (1) No IDSE cluster has declared acceptance obligations for this invariant yet (Doctrine §10); this record re-runs the checks carried from the legacy registry. Further limits: (2) No mutation witness exists yet for this critical invariant (Doctrine §28). |

_critical · local · scope: live fork choice · assumptions: A4 · depends on: SAF-007, SEC-021 · secondary: VAL._ Legacy: DC-NODE-38

#### `SEC-024` — _partial_

| Aspect | Location |
|--------|----------|
| **Source** | `docs/planning/node.define.consensus.md#batch-1` (approved by wdm33, 2026-11-09) |
| **Requirement** | **RetainedBranchEvidenceNonAuthoritative** — Blocks rolled back during a fork switch may be kept only as evidence for reconstructing future competing branches: bounded to k blocks, keyed by hash in a deterministic map, each entry bound to its recomputed hash; they never become durable authority, a rollback target or a fork anchor, and never bypass header or body validation. |
| **Code** | `crates/ade_node/src/node_lifecycle.rs`; `crates/ade_node/src/lca_walk.rs` |
| **Tests** | `rollback_retains_removed_blocks_for_lca_walk`, `retained_blocks_are_not_anchors`, `retained_blocks_are_k_bounded`, `retained_block_hash_self_binds`, `genuine_gap_still_missing_bridge`, `apply_fork_switch_populates_rollback_retention` |
| **CI** | `ci/ci_check_rollback_retention_evidence.sh` |
| **Claim** | partial — `e563ad94` (pass) — observed: tests: 6 passed, 0 failed, 0 ignored, 0 not found; CI scripts: 1 passed, 0 failed — limits: (1) No IDSE cluster has declared acceptance obligations for this invariant yet (Doctrine §10); this record re-runs the checks carried from the legacy registry. Further limits: (2) No mutation witness exists yet for this critical invariant (Doctrine §28). |

_critical · local · scope: fork-switch evidence cache · assumptions: A4 · depends on: SEC-023 · secondary: SAF._ Legacy: DC-NODE-40

#### `SEC-025` — _partial_

| Aspect | Location |
|--------|----------|
| **Source** | `docs/planning/node.define.consensus.md#batch-1` (approved by wdm33, 2026-11-09) |
| **Requirement** | **ForgeActivationAllOrNothing** — Forging is enabled only when the complete operator set is supplied (cold signing key, KES signing key, VRF signing key, operational certificate and genesis file): all present enables forging, all absent leaves a non-forging node, and any partial set is a structured error, never a silent non-forging fallback or a missing, zero or fabricated key; the pool id is derived in one named place, never fabricated; and private key material is never copied into coordinator, planner or loop state, or into anything persisted, logged, hashed for evidence or replayed, tests and debug output included. |
| **Code** | `crates/ade_node/src/forge_intent.rs`; `crates/ade_node/src/operator_forge.rs`; `crates/ade_node/src/node_lifecycle.rs` |
| **Tests** | `classify_forge_intent_total_over_all_32_flag_combinations`, `classify_forge_intent_none_present_is_off`, `forge_intent_error_carries_no_path_bytes`, `load_operator_producer_shell_builds_shell_from_complete_material`, `load_operator_producer_shell_kes_period_past_opcert_fails_closed`, `operator_forge_error_carries_no_path_or_key_bytes`, `build_operator_forge_material_from_complete_material`, `node_mode_with_operator_keys_warm_start_forge_capable_halts_clean`, `node_mode_partial_operator_keys_fail_closed`, `relay_loop_with_operator_material_forge_reaches_fenced_path`, `relay_loop_with_operator_material_two_runs_byte_identical` |
| **CI** | `ci/ci_check_forge_intent_closed.sh`, `ci/ci_check_operator_forge_no_secret_leak.sh`, `ci/ci_check_node_run_loop_containment.sh` |
| **Claim** | partial — `e563ad94` (pass) — observed: tests: 11 passed, 0 failed, 0 ignored, 0 not found; CI scripts: 3 passed, 0 failed — limits: (1) No IDSE cluster has declared acceptance obligations for this invariant yet (Doctrine §10); this record re-runs the checks carried from the legacy registry. Further limits: (2) No mutation witness exists yet for this critical invariant (Doctrine §28). |

_critical · local · scope: operator key ingress, forge activation · assumptions: — · depends on: SEC-006, SEC-001 · secondary: VAL._ Legacy: CN-NODE-03

### SEC — pipeline-production

#### `SEC-026` — _partial_

| Aspect | Location |
|--------|----------|
| **Source** | `docs/planning/pipeline-production.define.consensus.md#batch-1` (approved by wdm33, 2026-11-09) |
| **Requirement** | **LeaderVerdictCarriesNoKeys** — Leader eligibility crosses the shell/core boundary in one direction: shell code produces the VRF proof with the operator's VRF signing key, and the core verifies that proof and decides eligibility from canonical inputs only (slot, epoch nonce, VRF verification key, proof, and a leader-schedule answer giving the pool's stake share and threshold, which the caller derives through the leader-schedule authority of VAL-024 and passes in); the core evaluator depends on no ledger view, era schedule, chain-dependent state, clock, storage or shell crate and never receives a signing key, and its closed two-variant verdict carries forge-capable material only when eligible, an ineligible verdict carrying no more than a bounded fingerprint of the VRF output. |
| **Code** | `crates/ade_core/src/consensus/leader_check.rs`; `crates/ade_node/src/produce_mode.rs` |
| **Tests** | `eligible_on_threshold_with_high_stake_emits_eligible_verdict`, `not_eligible_with_zero_stake_emits_not_eligible_verdict`, `malformed_proof_emits_verification_failed`, `wrong_vk_emits_verification_failed`, `answer_slot_mismatch_emits_structured_error`, `vrf_input_mismatch_emits_structured_error`, `zero_stake_denominator_emits_structured_error`, `verdict_is_byte_identical_across_two_runs`, `vrf_output_fingerprint_is_first_8_bytes_of_output`, `zero_stake_answer_emits_forge_not_leader` |
| **CI** | `ci/ci_check_leader_check_authority.sh` |
| **Claim** | partial — `e563ad94` (pass) — observed: tests: 10 passed, 0 failed, 0 ignored, 0 not found; CI scripts: 1 passed, 0 failed — limits: (1) No IDSE cluster has declared acceptance obligations for this invariant yet (Doctrine §10); this record re-runs the checks carried from the legacy registry. Further limits: (2) No mutation witness exists yet for this critical invariant (Doctrine §28). |

_critical · local · scope: leader check boundary · assumptions: — · depends on: SEC-001, VAL-024, VAL-056 · secondary: STR._ Legacy: CN-FORGE-02

#### `SEC-027` — _partial_

| Aspect | Location |
|--------|----------|
| **Source** | `docs/planning/pipeline-production.define.consensus.md#batch-1` (approved by wdm33, 2026-11-09) |
| **Requirement** | **ForgeKesWindowFromBoundGenesis** — The KES evolution a forge signs at is computed by one checked function of the slot, the KES anchor slot and slots per KES period of the bound genesis (never the clock, a file read at forge time or a default) and the operational certificate's start period: the slot's absolute KES period minus the start period. A slot before the certificate's first covered slot, after its last, or whose arithmetic overflows is a typed refusal before any signing, naming the slot bound; and a forge result that arrives after a newer slot tick, or after its slot has passed, is discarded as a missed slot, never signed, served or admitted late. |
| **Code** | `crates/ade_core/src/consensus/opcert_validate.rs`; `crates/ade_codec/src/shelley/opcert.rs`; `crates/ade_runtime/src/producer/keys.rs`; `crates/ade_runtime/src/producer/ade_kes_envelope.rs`; `crates/ade_runtime/src/producer/signing.rs`; `crates/ade_crypto/src/kes_sum`; `crates/ade_node/src/key_gen.rs`; `crates/ade_runtime/src/producer/producer_shell.rs`; `crates/ade_node/src/operator_forge.rs`; `crates/ade_node/src/node_lifecycle.rs`; `crates/ade_runtime/src/producer/coordinator.rs`; `crates/ade_node/src/produce_mode.rs`; `crates/ade_runtime/src/producer/opcert_envelope.rs`; `crates/ade_runtime/src/producer/genesis_parser.rs` |
| **Tests** | `opcert_validate_accepts_canonical_fixture`, `opcert_validate_rejects_period_mismatch`, `opcert_validate_rejects_short_hot_vkey`, `opcert_validate_first_opcert_no_prev_counter`, `opcert_encoder_matches_cardano_cli_byte_identical`, `opcert_round_trip_byte_identical`, `ade_envelope_round_trips_through_loader_at_period_0`, `ade_envelope_loader_returns_kes_at_loaded_period`, `ade_envelope_loader_rejects_signing_at_past_period`, `cardano_cli_kes_envelope_rejects_32_byte_payload`, `cardano_cli_kes_envelope_rejects_synthetic_608_byte_payload`, `cardano_cli_kes_envelope_accepts_real_608_byte_payload`, `cardano_cli_kes_envelope_rejects_612_byte_payload`, `cardano_cli_kes_envelope_rejects_608_byte_leaf_zero_payload`, `ade_envelope_loader_returns_unknown_format`, `ade_envelope_loader_returns_wrong_role`, `ade_envelope_loader_returns_unsupported_crypto`, `ade_envelope_loader_returns_missing_seed_32`, `ade_envelope_loader_returns_period_idx_overflow`, `ade_envelope_loader_returns_malformed_json`, `write_ade_kes_envelope_sets_0600_permissions`, `vrf_and_cold_cardano_cli_skey_envelopes_round_trip`, `keys_loader_rejects_wrong_envelope_type`, `keys_loader_rejects_malformed_cbor_hex`, `key_load_error_io_carries_no_path_bytes`, `ade_envelope_load_error_io_carries_no_path_bytes`, `run_key_gen_kes_writes_loadable_envelope`, `run_key_gen_kes_writes_at_loaded_period`, `run_key_gen_kes_rejects_short_seed_file`, `cardano_cli_corpus_skey_deserializes_and_vk_matches_ground_truth`, `cardano_cli_corpus_sign_then_upstream_verifies`, `shell_init_rejects_kes_period_below_opcert_start`, `op_ops_04_anchors_evolution_zero_at_opcert_start_and_evolves_to_current_period`, `op_ops_04_fails_closed_outside_window_passes_at_boundaries`, `init_emits_started_event_and_zero_other_effects`, `slot_tick_emits_request_forge_and_log`, `forge_succeeded_emits_broadcast_and_log`, `forge_not_leader_emits_log_and_clears_pending`, `forge_failed_emits_slot_missed_with_mapped_reason`, `stale_forge_result_after_new_tick_drops_with_slot_missed`, `kes_period_out_of_range_errors`, `shell_init_rejects_malformed_opcert_hot_vkey_length`, `shell_kes_sign_at_current_period_succeeds_and_verifies`, `shell_kes_sign_at_wrong_period_errors`, `shell_kes_advance_to_evolves_key_one_way`, `shell_debug_redacts_secret_fields`, `zero_stake_answer_emits_forge_not_leader`, `kes_period_outside_window_emits_forge_failed_kes_period_mismatch`, `full_stake_answer_reaches_self_accept_and_rejects` |
| **CI** | `ci/ci_check_opcert_closed.sh`, `ci/ci_check_private_key_custody.sh`, `ci/ci_check_kes_envelope_closed.sh`, `ci/ci_check_kes_sum_compatibility.sh`, `ci/ci_check_producer_coordinator_no_secrets.sh`, `ci/ci_check_produce_mode_uses_bootstrap_initial_state.sh` |
| **Claim** | partial — `e563ad94` (pass) — observed: tests: 49 passed, 0 failed, 0 ignored, 0 not found; CI scripts: 6 passed, 0 failed — limits: (1) No IDSE cluster has declared acceptance obligations for this invariant yet (Doctrine §10); this record re-runs the checks carried from the legacy registry. Further limits: (2) No mutation witness exists yet for this critical invariant (Doctrine §28). |

_critical · local · scope: forge-time KES period, forge result timing · assumptions: A2 · depends on: SEC-005, DET-019, OBS-001 · secondary: SAF._ Legacy: DC-CONS-11, CN-PROD-02

#### `SEC-028` — _partial_

| Aspect | Location |
|--------|----------|
| **Source** | `docs/planning/pipeline-production.define.consensus.md#batch-1` (approved by wdm33, 2026-11-09) |
| **Requirement** | **MultiPeerFairMerge** — When several peers feed the receive path, no connected peer is starved by another: each peer's pump fills its own bounded lane, the lanes are merged round-robin (one item per peer per round) in the configured peer order, never by hash iteration, randomness, clock or scheduler timing, backpressure is per peer, and a disconnected peer's lane is retired without reordering the others. The merge decides only delivery opportunity: peer identity is preserved and chain selection stays independent of arrival order. |
| **Code** | `crates/ade_node/src/fair_merge.rs`; `crates/ade_node/src/node_lifecycle.rs`; `ci/ci_check_wire_pump_fairness.sh` |
| **Tests** | `hot_peer_cannot_starve_quiet_peer`, `closed_lane_removed_without_reordering_remaining_peers`, `peer_identity_preserved_through_merge` |
| **CI** | `ci/ci_check_wire_pump_fairness.sh` |
| **Claim** | partial — `e563ad94` (pass) — observed: tests: 3 passed, 0 failed, 0 ignored, 0 not found; CI scripts: 1 passed, 0 failed — limits: (1) No IDSE cluster has declared acceptance obligations for this invariant yet (Doctrine §10); this record re-runs the checks carried from the legacy registry. Further limits: (2) No mutation witness exists yet for this critical invariant (Doctrine §28). |

_critical · local · scope: multi-peer receive merge · assumptions: — · depends on: OBS-003, DET-012, SEC-018 · secondary: LIV._ Legacy: DC-PUMP-04

### SEC — mithril-evidence

#### `SEC-029` — _partial_

| Aspect | Location |
|--------|----------|
| **Source** | `docs/planning/mithril-evidence.define.consensus.md#batch-1` (approved by wdm33, 2026-11-09) |
| **Requirement** | **MithrilBindingIndependentOrigins** — On the cardano-cli seed route, a seed bootstraps only after a pure predicate cross-checks the Mithril manifest's network magic, genesis hash, certified point and certificate hash against the bootstrap anchor minted from the operator's separately extracted seed, before any authoritative store is initialized; each disagreement is a distinct closed error. The seed point, network magic and genesis hash on the anchor side come from the operator's separate extraction and the selected network's genesis, never from the manifest, so those three checks compare two origins; the certificate-hash check is provenance consistency between the anchor's recorded Mithril provenance and the manifest, its authenticity resting on N1 and SEC-034. |
| **Code** | `crates/ade_ledger/src/bootstrap_anchor/binding.rs`; `crates/ade_runtime/src/mithril_import`; `crates/ade_runtime/src/mithril_bootstrap.rs`; `ci/ci_check_mithril_seed_point_independence.sh` |
| **Tests** | `mithril_binding_rejects_certified_point_other_than_seed_point`, `mithril_anchor_rejects_field_mismatch`, `mithril_import_fail_closed_blocks_storage_init`, `mithril_bootstrap_verifies_before_storage_init`, `mithril_bootstrap_fails_closed_on_seed_point_mismatch`, `mithril_anchor_binding_is_deterministic`, `mithril_bootstrap_succeeds_when_seed_point_matches` |
| **CI** | `ci/ci_check_mithril_uses_bootstrap_initial_state.sh`, `ci/ci_check_mithril_seed_point_independence.sh` |
| **Claim** | partial — `e563ad94` (pass) — observed: tests: 7 passed, 0 failed, 0 ignored, 0 not found; CI scripts: 2 passed, 0 failed — limits: (1) No IDSE cluster has declared acceptance obligations for this invariant yet (Doctrine §10); this record re-runs the checks carried from the legacy registry. Further limits: (2) No mutation witness exists yet for this critical invariant (Doctrine §28). |

_critical · local · scope: cardano-cli seed route · assumptions: A7 · depends on: SEC-015, STR-021, SEC-034._ Legacy: CN-MITHRIL-01, DC-MITHRIL-01, DC-MITHRIL-02

#### `SEC-030` — _partial_

| Aspect | Location |
|--------|----------|
| **Source** | `docs/planning/mithril-evidence.define.consensus.md#batch-1` (approved by wdm33, 2026-11-09) |
| **Requirement** | **NativeSnapshotSoleSource** — The native route assembles the complete seed (ledger state, chain-dependent state and seed-epoch consensus inputs) only from the verified Mithril snapshot (its manifest, ledger state and UTxO tables) and the Shelley genesis constants, each seed field having one declared source among them; it reaches no cardano-cli importer, consensus-input bundle or operator-supplied seed, and has no fallback. The native components (manifest, ledger state, UTxO tables and Shelley genesis) are required together: a partial set is a typed terminal error before any decode and before either route is selected, never a reason to take the cardano-cli route. A cardano-cli seed flag supplied alongside the native inputs is a typed terminal error before any decode, and the two routes are never fallbacks for each other. |
| **Code** | `crates/ade_ledger/src/bootstrap_anchor/binding.rs`; `crates/ade_runtime/src/mithril_import`; `crates/ade_runtime/src/mithril_bootstrap.rs`; `crates/ade_runtime/src/mithril_native_assembly.rs`; `crates/ade_runtime/src/seed_epoch_lineage.rs`; `ci/ci_check_mithril_authority_transition.sh`; `crates/ade_node/src/native_firstrun.rs`; `crates/ade_node/src/node_lifecycle.rs`; `crates/ade_node/src/cli.rs`; `crates/ade_node/tests/native_firstrun_live.rs`; `ci/ci_check_native_firstrun_no_cli_seed.sh` |
| **Tests** | `mithril_binding_rejects_certified_point_other_than_seed_point`, `mithril_anchor_rejects_field_mismatch`, `mithril_import_fail_closed_blocks_storage_init`, `mithril_bootstrap_fails_closed_on_seed_point_mismatch`, `native_assembled_seed_is_deterministic`, `native_assembly_maps_each_field_from_its_source`, `point_mismatch_is_terminal`, `point_hash_mismatch_is_terminal`, `wrong_era_is_terminal`, `wrong_network_is_terminal`, `epoch_mismatch_is_terminal`, `native_bootstrap_persists_and_anchor_point_is_recoverable`, `interrupted_persist_leaves_no_discoverable_anchor_lineage`, `native_first_run_forbidden_json_seed_is_terminal`, `native_first_run_forbidden_consensus_inputs_is_terminal`, `native_first_run_missing_manifest_is_terminal`, `native_first_run_missing_shelley_genesis_is_terminal` **†**, `native_first_run_malformed_manifest_is_terminal`, `native_first_run_malformed_shelley_genesis_is_terminal`, `native_first_run_real_snapshot_invokes_bootstrap_and_persists`, `native_first_run_real_snapshot_wrong_network_is_terminal`, `preprod_snapshot_epoch_window_contains_manifest_point`, `shelley_genesis_active_slots_coeff_decimal_to_rational`, `parse_native_shelley_genesis_extracts_constants`, `unknown_network_magic_has_no_boundary` |
| **CI** | `ci/ci_check_mithril_uses_bootstrap_initial_state.sh`, `ci/ci_check_mithril_seed_point_independence.sh`, `ci/ci_check_mithril_documented_evidence.sh`, `ci/ci_check_mithril_authority_transition.sh`, `ci/ci_check_native_firstrun_no_cli_seed.sh` (red at e563ad94) |
| **Claim** | unresolved — `e563ad94` (fail) — observed: tests: 24 passed, 0 failed, 0 ignored, 1 not found; CI scripts: 4 passed, 1 failed; failing CI: ci/ci_check_native_firstrun_no_cli_seed.sh — limits: (1) No IDSE cluster has declared acceptance obligations for this invariant yet (Doctrine §10); this record re-runs the checks carried from the legacy registry. Further limits: (2) No mutation witness exists yet for this critical invariant (Doctrine §28). (3) 1 named test(s) were not found in the test output at this revision: native_first_run_missing_shelley_genesis_is_terminal (4) ci/ci_check_native_firstrun_no_cli_seed.sh fails at this revision and already failed before the IDSE migration (a3cef7f7): "FAIL: missing S1d dispatch test 'native_first_run_missing_shelley_genesis_is_terminal'". Diagnosed as a stale or over-broad structural gate, not a demonstrated violation; the gate needs repair before it can serve as evidence. |

_critical · local · scope: native Mithril first run · assumptions: A7, A2 · depends on: STR-013, SEC-014 · secondary: STR._ Legacy: DC-MITHRIL-03, DC-MITHRIL-07, RO-MITHRIL-IMPORT-01

#### `SEC-031` — _declared_

| Aspect | Location |
|--------|----------|
| **Source** | `docs/planning/mithril-evidence.define.consensus.md#batch-1` (approved by wdm33, 2026-11-09) |
| **Requirement** | **NativeSnapshotPointCoherence** — Before anything is assembled or persisted, the native route checks that the snapshot is one Conway chain point: the certified point equals the tip the snapshot's own ledger state records (slot and header hash), the ledger state's current era is Conway, and its epoch equals the epoch the era schedule assigns to the certified slot. The ledger state carries no network identity of its own, so the network is the manifest's magic, which must match the selected network's committed profile and genesis hash; its authenticity rests on SEC-034. The Shelley genesis file whose constants the native route uses must hash to the selected profile's committed genesis hash before anything is assembled. The era schedule is built from the genesis epoch length and the network's committed Shelley start boundary, an unknown network being terminal, and the active-slot coefficient is parsed to an exact rational with no floating point. Any disagreement is a typed terminal error with nothing persisted. |
| **Code** | _(no enforcement found — gap)_ |
| **Tests** | _(no tests found — gap)_ |
| **CI** | _(no CI enforcement — gap)_ |
| **Claim** | unresolved — no evidence record |

_critical · local · scope: native Mithril first run · assumptions: A7, A2 · depends on: SEC-030, SEC-034, DUR-005 · secondary: VAL._ Legacy: none (new in IDSE)

#### `SEC-032` — _partial_

| Aspect | Location |
|--------|----------|
| **Source** | `docs/planning/mithril-evidence.define.consensus.md#batch-1` (approved by wdm33, 2026-11-09) |
| **Requirement** | **CertStatePackageHashBound** — The seed-epoch consensus record and the bootstrap certificate state stay separately typed and are bound only through a canonical bootstrap manifest naming the network, era, source point, seed hash, certificate-state hash and source commitment. The certificate state is the complete state written and read by the canonical certificate-state codec. Node-mode bootstrap on the cardano-cli seed route requires both package files; import fails closed before anything is durable when either is missing, on a malformed manifest, a network or era mismatch, seed or certificate-state bytes that do not hash to the manifest's values, or a certificate state that does not decode; the imported state populates both the captured snapshot and the running ledger. A package-free import, yielding an empty certificate state, exists only in admission mode and diagnostics and never becomes node-mode authority. |
| **Code** | `crates/ade_ledger/src/bootstrap_manifest.rs`; `crates/ade_node/src/admission/seed_to_snapshot.rs`; `crates/ade_node/src/admission/bootstrap.rs`; `ci/ci_check_eview_bootstrap_cert_state.sh` |
| **Tests** | `manifest_round_trips_canonically`, `verify_and_import_happy_path`, `seed_hash_mismatch_fails_closed`, `cert_state_hash_mismatch_fails_closed`, `network_and_era_mismatch_fail_closed`, `malformed_manifest_fails_closed`, `malformed_cert_state_fails_closed_after_hash_ok` |
| **CI** | `ci/ci_check_eview_bootstrap_cert_state.sh` (red at e563ad94) |
| **Claim** | unresolved — `e563ad94` (fail) — observed: tests: 7 passed, 0 failed, 0 ignored, 0 not found; CI scripts: 0 passed, 1 failed; failing CI: ci/ci_check_eview_bootstrap_cert_state.sh — limits: (1) No IDSE cluster has declared acceptance obligations for this invariant yet (Doctrine §10); this record re-runs the checks carried from the legacy registry. Further limits: (2) No mutation witness exists yet for this critical invariant (Doctrine §28). (3) Expected to be falsified (mithril-evidence S6) if node mode still bootstraps with an empty certificate state when the package files are absent. (4) ci/ci_check_eview_bootstrap_cert_state.sh fails at this revision and already failed before the IDSE migration (a3cef7f7): "FAIL: build_seed_ledger does not populate cert_state". Diagnosed as a stale or over-broad structural gate, not a demonstrated violation; the gate needs repair before it can serve as evidence. |

_critical · local · scope: cardano-cli seed route bootstrap package, admission mode · assumptions: — · depends on: STR-010, SEC-015, DUR-003 · secondary: VAL._ Legacy: DC-EVIEW-09

#### `SEC-033` — _partial_

| Aspect | Location |
|--------|----------|
| **Source** | `docs/planning/mithril-evidence.define.consensus.md#batch-1` (approved by wdm33, 2026-11-09) |
| **Requirement** | **AdversarialBlockMutationsRejected** — Each of four block mutations made to a block the reference node accepts (a body byte flip that keeps the envelope's shape, a header body-hash mismatch, a KES or signature corruption, and a VRF proof or output tamper) is rejected: it is never admitted and never yields an agreed or input-not-found verdict, but a diverged verdict with admission halting, or an undecodable-bytes halt when the corruption breaks decoding. |
| **Code** | `crates/ade_node/tests/admission_adversarial_corpus.rs` |
| **Tests** | `adversarial_corpus_rejects_all_four_mutation_classes` |
| **CI** | `ci/ci_check_adversarial_false_accept_corpus.sh` |
| **Claim** | partial — `e563ad94` (pass) — observed: tests: 1 passed, 0 failed, 0 ignored, 0 not found; CI scripts: 1 passed, 0 failed — limits: (1) No IDSE cluster has declared acceptance obligations for this invariant yet (Doctrine §10); this record re-runs the checks carried from the legacy registry. Further limits: (2) No mutation witness exists yet for this critical invariant (Doctrine §28). |

_critical · local · scope: block admission (admission mode evidence) · assumptions: A2 · depends on: VAL-032, SEC-003, VAL-009 · secondary: VAL._ Legacy: DC-EVIDENCE-02

#### `SEC-034` — _declared_

| Aspect | Location |
|--------|----------|
| **Source** | `docs/planning/mithril-evidence.define.consensus.md#batch-1` (approved by wdm33, 2026-11-09) |
| **Requirement** | **SnapshotBytesBoundToVerification** — What a Mithril bootstrap consumes is exactly what the Mithril client verified under the selected network's committed Mithril keys: the acquisition records verification evidence (the certificate identity, the network, the certified point and the hashes of the verified files), a directory reused without that evidence is re-verified or refused, and on either route bootstrap refuses a manifest whose certificate identity, network or point differs from that evidence, and on the native route ledger-state or UTxO-tables files whose hashes differ from the recorded ones. No BLUE crate depends on a Mithril or multisignature crate; verification runs only in the shell. |
| **Code** | _(no enforcement found — gap)_ |
| **Tests** | _(no tests found — gap)_ |
| **CI** | _(no CI enforcement — gap)_ |
| **Claim** | unresolved — no evidence record |

_critical · local · scope: Mithril acquisition, both bootstrap routes · assumptions: A7 · depends on: SEC-015, STR-003 · secondary: DUR._ Legacy: none (new in IDSE)

### SEC — resources-release

#### `SEC-035` — _partial_

| Aspect | Location |
|--------|----------|
| **Source** | `docs/planning/resources-release.define.consensus.md#batch-1` (approved by wdm33, 2026-11-09) |
| **Requirement** | **BoundedDeterministicAdmission** — Untrusted inbound work (transactions, peer requests) is admitted through fixed, bounded policies before it consumes authoritative resources (validation time, mempool capacity, storage), and when demand exceeds a bound the work shed is chosen by a deterministic rule over the recorded arrival order, never by timing, randomness or resource exhaustion; shedding never changes the verdict of any work admitted. |
| **Code** | `crates/ade_ledger/src/mempool/policy.rs`; `crates/ade_node/src/mem_measure/bounded_admission.rs`; `crates/ade_node/src/mem_measure/runner.rs` |
| **Tests** | `policy_does_not_change_validity`, `determinism`, `bounded_admission_respects_count_budget`, `bounded_admission_respects_byte_budget`, `bounded_admission_is_deterministic`, `bounded_gate_under_budget_equals_unbounded`, `bounded_gate_preserves_admit_verdict`, `bounded_gate_no_false_accept_under_pressure`, `hermetic_measurement_verdict_is_agreed`, `hermetic_measurement_is_replay_stable` |
| **CI** | `ci/ci_check_consensus_closed_enums.sh`, `ci/ci_check_bounded_inbound_admission.sh` |
| **Claim** | partial — `e563ad94` (pass) — observed: tests: 10 passed, 0 failed, 0 ignored, 0 not found; CI scripts: 2 passed, 0 failed — limits: (1) No IDSE cluster has declared acceptance obligations for this invariant yet (Doctrine §10); this record re-runs the checks carried from the legacy registry. Further limits: (2) No mutation witness exists yet for this critical invariant (Doctrine §28). |

_critical · local · scope: transaction and request admission · assumptions: A1 · depends on: SEC-002, SEC-018 · secondary: DET._ Legacy: CN-MEM-01, DC-MEM-02, CN-MEM-03

#### `SEC-036` — _partial_

| Aspect | Location |
|--------|----------|
| **Source** | `docs/planning/resources-release.define.consensus.md#batch-1` (approved by wdm33, 2026-11-09) |
| **Requirement** | **ServeRangeWorkBounded** — A peer's block-fetch range request makes the serve path read at most a fixed number of blocks, without materializing an unbounded range or scanning the whole index per block; a larger range fails closed before storage or CPU work grows, and the bound is fixed, not a runtime option. |
| **Code** | `crates/ade_runtime/src/chaindb/mod.rs`; `crates/ade_runtime/src/chaindb/types.rs`; `crates/ade_runtime/src/chaindb/persistent.rs`; `crates/ade_runtime/src/network/served_chain_projection.rs` |
| **Tests** | `range_bytes_capped_returns_at_most_max`, `range_bytes_capped_within_cap_not_truncated`, `range_bytes_capped_respects_bounds`, `range_bytes_capped_bytes_byte_identical`, `range_bytes_capped_inverted_range_is_empty`, `last_block_bytes_returns_highest_slot`, `serve_range_over_cap_fails_closed`, `serve_range_empty_window_is_empty_not_capexceeded`, `serve_range_undecodable_in_range_fails_closed`, `serve_range_inverted_range_fails_closed`, `served_view_projects_durable_chain`, `follower_fetches_coherent_history_incl_ingested_predecessor` |
| **CI** | `ci/ci_check_serve_range_bounded.sh` |
| **Claim** | partial — `e563ad94` (pass) — observed: tests: 6 passed, 0 failed, 0 ignored, 6 not found; CI scripts: 1 passed, 0 failed — limits: (1) No IDSE cluster has declared acceptance obligations for this invariant yet (Doctrine §10); this record re-runs the checks carried from the legacy registry. Further limits: (2) No mutation witness exists yet for this critical invariant (Doctrine §28). (3) 6 named test(s) were not found in the test output at this revision: range_bytes_capped_returns_at_most_max, range_bytes_capped_within_cap_not_truncated, range_bytes_capped_respects_bounds, range_bytes_capped_bytes_byte_identical, range_bytes_capped_inverted_range_is_empty, last_block_bytes_returns_highest_slot |

_critical · local · scope: serving · assumptions: — · depends on: SEC-002, VAL-046._ Legacy: DC-SERVEMEM-01

#### `SEC-037` — _declared_

| Aspect | Location |
|--------|----------|
| **Source** | `docs/planning/resources-release.define.consensus.md#batch-1` (approved by wdm33, 2026-11-09) |
| **Requirement** | **NoPeerDiscoveryWhenForging** — A node with forging enabled dials only the peers the operator configured; it never dials a peer learned from the network (peer sharing or ledger relay registrations). |
| **Code** | _(no enforcement found — gap)_ |
| **Tests** | _(no tests found — gap)_ |
| **CI** | _(no CI enforcement — gap)_ |
| **Claim** | unresolved — no evidence record |

_critical · local · scope: outbound connections · assumptions: — · depends on: SEC-025._ Legacy: CN-NET-01, OP-NET-01

#### `SEC-038` — _partial_

| Aspect | Location |
|--------|----------|
| **Source** | `docs/planning/resources-release.define.consensus.md#batch-1` (approved by wdm33, 2026-11-09) |
| **Requirement** | **LiveFeedBuffersClosedConstants** — Peer-driven memory on the live feed is bounded before authoritative decoding by closed constants that no command-line option, environment variable or configuration can change or disable: a per-mini-protocol reassembly buffer that exceeds its bound fails closed with a typed error and drops that peer, and the pump's lookahead stops draining at its bound so the bounded channel back-pressures it; nothing is truncated or partly decoded. |
| **Code** | `crates/ade_network/src/session/core.rs`; `crates/ade_node/src/node_sync.rs`; `crates/ade_runtime/src/network/mux_pump.rs` |
| **Tests** | `session_reassembly_tail_over_cap_fails_closed`, `session_reassembly_tail_under_cap_still_drains_complete_item`, `wirepump_lookahead_stops_at_cap`, `wirepump_lookahead_cap_preserves_relay_behavior_under_normal_feed` |
| **CI** | `ci/ci_check_live_feed_memory_bounds.sh` |
| **Claim** | partial — `e563ad94` (pass) — observed: tests: 4 passed, 0 failed, 0 ignored, 0 not found; CI scripts: 1 passed, 0 failed — limits: (1) No IDSE cluster has declared acceptance obligations for this invariant yet (Doctrine §10); this record re-runs the checks carried from the legacy registry. Further limits: (2) No mutation witness exists yet for this critical invariant (Doctrine §28). |

_critical · local · scope: live feed · assumptions: — · depends on: SEC-002, SEC-018._ Legacy: DC-LIVEMEM-01

---

## STR — Structural

_Doctrine §6: Required system structure, component relationships, architectural boundaries, and structural constraints._ 22 invariant(s).

### STR — foundations

#### `STR-001` — _partial_

| Aspect | Location |
|--------|----------|
| **Source** | `docs/planning/foundations.define.consensus.md#batch-1` (approved by wdm33, 2026-11-08) |
| **Requirement** | **AuthoritativeCorePurity** — Code in the BLUE core paths performs no I/O, reads no clock, environment or randomness, spawns no tasks and contains no async code; a core function's only inputs are its arguments. |
| **Code** | `crates/ade_ledger/src/rules.rs`; `crates/ade_ledger/src/lib.rs`; `crates/ade_core/src/consensus/vrf_cert.rs`; `crates/ade_core/src/consensus/leader_schedule.rs`; `crates/ade_core/src/consensus/ledger_view.rs`; `crates/ade_ledger/src/consensus_view.rs`; `crates/ade_runtime/src/consensus/chain_selector.rs`; `crates/ade_core_interop/src/lib.rs`; `crates/ade_core_interop/src/bin/live_consensus_session.rs`; `crates/ade_plutus/src`; `crates/ade_core/src/consensus/era_schedule.rs`; `crates/ade_ledger/src/consensus_input_extract.rs`; `crates/ade_ledger/src/tx_validity/transition.rs`; `crates/ade_ledger/src/tx_validity/phase1.rs` |
| **Tests** | `apply_block_deterministic`, `apply_block_byron_ebb_passes_through`, `taylor_exp_cmp_le_zero_x_returns_false`, `taylor_exp_cmp_le_x_equals_one_returns_true`, `taylor_exp_cmp_le_monotone_in_x`, `is_leader_determinism`, `is_leader_known_vector_matches_reference`, `all_eras_determinism_summary`, `consensus::leader_schedule::tests::query_uses_state_epoch_nonce_for_vrf_input`, `consensus::leader_schedule::tests::query_returns_unknown_pool_when_no_vrf_key`, `consensus::leader_schedule::tests::query_returns_outside_forecast_range_for_far_future`, `consensus::leader_schedule::tests::query_does_not_mutate_state`, `consensus::leader_check::tests::eligible_on_threshold_with_high_stake_emits_eligible_verdict`, `consensus::leader_check::tests::not_eligible_with_zero_stake_emits_not_eligible_verdict`, `corpus_returns_canonical_answer_for_known_pools`, `corpus_rejects_unknown_pool`, `corpus_rejects_out_of_forecast_horizon`, `corpus_is_leader_helper_matches_pinned_probe`, `corpus_is_deterministic_across_runs`, `view_returns_corpus_pool_stake_and_vrf_keyhash`, `view_unknown_pool_returns_none`, `view_unknown_epoch_returns_none`, `view_is_pure`, `consensus_view::tests::pool_distr_view_no_hashmap`, `consensus::chain_selector::tests::header_arrival_updates_state_and_selector`, `consensus::chain_selector::tests::rollback_walks_back_via_recent_snapshots`, `consensus::chain_selector::tests::rollback_to_block_older_than_snapshots_rejected`, `consensus::chain_selector::tests::epoch_boundary_emits_no_event`, `cardano_node_session_sustained_window` (#[ignore]), `plutus_eval_is_deterministic`, `consensus::era_schedule::tests::slot_to_time_monotone_increasing`, `consensus::era_schedule::tests::slot_to_time_overflow_returns_structured_error`, `consensus::era_schedule::tests::determinism_across_runs`, `consensus::era_schedule::tests::locate_first_slot_of_each_era`, `consensus::era_schedule::tests::locate_last_slot_of_each_era`, `consensus::era_schedule::tests::locate_before_system_start_errors`, `mainnet_corpus_translation_matches_oracle`, `preprod_corpus_translation_matches_oracle`, `corpus_loads_and_is_self_consistent`, `extract_nonces_field_order`, `extract_nonces_requires_exactly_five`, `extract_nonces_is_deterministic`, `tx_validity_is_deterministic` |
| **CI** | `ci/ci_check_forbidden_patterns.sh`, `ci/ci_check_dependency_boundary.sh`, `ci/ci_check_no_float_in_consensus.sh`, `ci/ci_check_ledger_determinism.sh`, `ci/ci_check_no_async_in_blue.sh`, `ci/ci_check_plutus_eval_purity.sh`, `ci/ci_check_consensus_closed_enums.sh` |
| **Claim** | partial — `e563ad94` (pass) — observed: tests: 40 passed, 0 failed, 1 ignored, 2 not found; CI scripts: 7 passed, 0 failed — limits: (1) No IDSE cluster has declared acceptance obligations for this invariant yet (Doctrine §10); this record re-runs the checks carried from the legacy registry. Further limits: (2) No mutation witness exists yet for this critical invariant (Doctrine §28). (3) 2 named test(s) were not found in the test output at this revision: all_eras_determinism_summary, plutus_eval_is_deterministic (4) 1 named test(s) are #[ignore]d and did not run. |

_critical · local · scope: BLUE core paths (.idse-config.json core_paths) · assumptions: — · depends on: — · secondary: DET._ Legacy: T-CORE-01, T-CORE-02, DC-CORE-01

#### `STR-002` — _partial_

| Aspect | Location |
|--------|----------|
| **Source** | `docs/planning/foundations.define.consensus.md#batch-1` (approved by wdm33, 2026-11-08) |
| **Requirement** | **ShellCanonicalizesInputs** — Transport artifacts (socket fragmentation, mux segment boundaries, interleaving across mini-protocols, read timing and timeouts) never reach the core: two transport histories that deliver the same ordered sequence of whole messages to each mini-protocol, from the same starting state, under the same protocol version, with the same recorded context and the same authoritative input order, produce identical core outcomes. Nondeterministic facts the core legitimately needs, such as the current slot, enter only as recorded explicit context. |
| **Code** | `crates/ade_ledger/src/consensus_input_extract.rs`; `crates/ade_ledger/src/consensus_view.rs`; `crates/ade_core/src/consensus/ledger_view.rs`; `crates/ade_ledger/src/tx_validity/transition.rs`; `crates/ade_ledger/src/tx_validity/phase1.rs` |
| **Tests** | `corpus_loads_and_is_self_consistent`, `extract_nonces_field_order`, `extract_nonces_requires_exactly_five`, `extract_nonces_is_deterministic`, `view_returns_corpus_pool_stake_and_vrf_keyhash`, `view_unknown_pool_returns_none`, `view_unknown_epoch_returns_none`, `view_is_pure`, `tx_validity_is_deterministic` |
| **CI** | `ci/ci_check_consensus_closed_enums.sh` |
| **Claim** | partial — `e563ad94` (pass) — observed: tests: 9 passed, 0 failed, 0 ignored, 0 not found; CI scripts: 1 passed, 0 failed — limits: (1) No IDSE cluster has declared acceptance obligations for this invariant yet (Doctrine §10); this record re-runs the checks carried from the legacy registry. Further limits: (2) No mutation witness exists yet for this critical invariant (Doctrine §28). |

_critical · local · scope: shell/core boundary, mux demultiplexing · assumptions: A1 · depends on: STR-001 · secondary: DET._ Legacy: T-BOUND-01, T-TRANSPORT-01

#### `STR-003` — _partial_

| Aspect | Location |
|--------|----------|
| **Source** | `docs/planning/foundations.define.consensus.md#batch-1` (approved by wdm33, 2026-11-08) |
| **Requirement** | **CoreDependsOnlyInward** — No BLUE path depends on RED code: a crate whose modules are all BLUE depends, directly or transitively, on no RED crate and on no library providing networking I/O, filesystem access, an async runtime, a clock, randomness or a general-purpose serialization framework; a BLUE module inside a mixed crate imports none of that crate's RED modules and none of those libraries. |
| **Code** | _(no enforcement found — gap)_ |
| **Tests** | _(no tests found — gap)_ |
| **CI** | `ci/ci_check_dependency_boundary.sh` |
| **Claim** | partial — `e563ad94` (pass) — observed: tests: 0 passed, 0 failed, 0 ignored, 0 not found; CI scripts: 1 passed, 0 failed — limits: (1) No IDSE cluster has declared acceptance obligations for this invariant yet (Doctrine §10); this record re-runs the checks carried from the legacy registry. Further limits: (2) No mutation witness exists yet for this critical invariant (Doctrine §28). |

_critical · local · scope: workspace crate and module graph · assumptions: — · depends on: STR-001._ Legacy: T-BOUND-02

### STR — codec-wire-crypto

#### `STR-004` — _partial_

| Aspect | Location |
|--------|----------|
| **Source** | `docs/planning/codec-wire-crypto.define.consensus.md#batch-1` (approved by wdm33, 2026-11-08) |
| **Requirement** | **NamedIngressChokepoints** — Every external byte string that reaches an authoritative transition, from the network, the node-to-client socket, or storage rehydration (write-ahead log, chain database, snapshots), enters the core through the named decode chokepoint for its type and era; no path hands raw or partially decoded bytes to a core transition around it. |
| **Code** | _(no enforcement found — gap)_ |
| **Tests** | _(no tests found — gap)_ |
| **CI** | `ci/ci_check_ingress_chokepoints.sh` |
| **Claim** | partial — `e563ad94` (pass) — observed: tests: 0 passed, 0 failed, 0 ignored, 0 not found; CI scripts: 1 passed, 0 failed — limits: (1) No IDSE cluster has declared acceptance obligations for this invariant yet (Doctrine §10); this record re-runs the checks carried from the legacy registry. Further limits: (2) No mutation witness exists yet for this critical invariant (Doctrine §28). |

_critical · local · scope: ingress: network, N2C, storage rehydration · assumptions: — · depends on: VAL-001, STR-002 · secondary: VAL._ Legacy: T-INGRESS-01, DC-INGRESS-01, DC-INGRESS-02, CN-WIRE-06

### STR — ledger

#### `STR-005` — _partial_

| Aspect | Location |
|--------|----------|
| **Source** | `docs/planning/ledger.define.consensus.md#batch-1` (approved by wdm33, 2026-11-08) |
| **Requirement** | **CredentialDiscriminantPreserved** — A stake, committee or DRep credential is a closed sum of a key hash or a script hash over 28 bytes everywhere: in decoding, as a ledger-state key and in comparison; a key-hash credential and a script-hash credential with the same 28 bytes are distinct, no conversion from a bare 28-byte hash to a credential exists on the authoritative path, and every authoritative encoding and fingerprint of a credential includes its key-or-script tag. |
| **Code** | `crates/ade_types/src/shelley/cert.rs`; `crates/ade_codec/src/shelley/cert.rs`; `crates/ade_codec/src/conway/cert.rs`; `crates/ade_ledger/src/state.rs`; `crates/ade_ledger/src/fingerprint.rs`; `crates/ade_testkit/src/harness/snapshot_loader.rs`; `crates/ade_ledger/src/governance.rs`; `crates/ade_types/src/conway/governance.rs`; `crates/ade_ledger/src/rules.rs`; `crates/ade_ledger/src/epoch.rs`; `crates/ade_ledger/src/epoch_accumulator.rs`; `crates/ade_ledger/src/ledgerdb_state.rs`; `crates/ade_ledger/src/snapshot/epoch_state.rs`; `ci/ci_check_snapshot_fidelity.sh` |
| **Tests** | `shelley_credential_preserves_discriminant`, `conway_credential_preserves_discriminant`, `unknown_credential_tag_rejects`, `discriminant_changes_fingerprint`, `keyhash_scripthash_same_bytes_are_distinct_certstate`, `keyhash_scripthash_same_bytes_are_distinct_govstate`, `discriminant_changes_fingerprint_corpus`, `credential_accumulation_replays_byte_identical`, `committee_keyhash_scripthash_do_not_cross_resolve`, `committee_keyhash_scripthash_same_bytes_distinct`, `committee_discriminant_changes_fingerprint`, `drep_keyhash_scripthash_do_not_cross_resolve`, `drep_vote_discriminant_changes_fingerprint`, `enactment_committee_changes_keyhash_scripthash_distinct`, `update_committee_decodes_removed_added_threshold`, `update_committee_keyhash_scripthash_members_distinct`, `update_committee_decode_rejects_malformed`, `update_committee_structured_fields_change_fingerprint`, `enact_update_committee_keyhash_scripthash_distinct`, `committee_oracle_mainnet_575_576_noop_agreement` (FAILED at e563ad94), `a_key_and_a_script_credential_with_the_same_bytes_are_two_stakers`, `a_script_credentials_reward_is_never_credited_to_the_key_credential_with_the_same_bytes`, `an_operator_with_a_script_reward_account_is_paid_on_the_script_credential`, `an_owner_is_matched_as_a_key_credential_only`, `a_script_voter_does_not_take_the_stake_of_a_key_credential_with_the_same_bytes`, `read_stake_snapshot_full_decodes_combined_stake_and_delegation`, `a_registration_reward_account_tag_is_the_credential_kind_not_the_network`, `codec_rejects_pre_lv4_store_rebootstrap_required` |
| **CI** | `ci/ci_check_credential_discriminant_closed.sh`, `ci/ci_check_snapshot_fidelity.sh`, `ci/ci_check_store_semantics_lock.sh` |
| **Claim** | unresolved — `e563ad94` (fail) — observed: tests: 27 passed, 1 failed, 0 ignored, 0 not found; CI scripts: 3 passed, 0 failed; failing tests: committee_oracle_mainnet_575_576_noop_agreement — limits: (1) No IDSE cluster has declared acceptance obligations for this invariant yet (Doctrine §10); this record re-runs the checks carried from the legacy registry. Further limits: (2) No mutation witness exists yet for this critical invariant (Doctrine §28). (3) committee_oracle_mainnet_575_576_noop_agreement fails because the test passes DormantEpochs::Unversioned, which the ratification gate now refuses (fail-closed dormancy); the test input is stale, not a counterexample. |

_critical · local · scope: ledger credentials, certificate and governance state · assumptions: A2 · depends on: VAL-001 · secondary: VAL._ Legacy: DC-LEDGER-10

#### `STR-006` — _partial_

| Aspect | Location |
|--------|----------|
| **Source** | `docs/planning/ledger.define.consensus.md#batch-1` (approved by wdm33, 2026-11-08) |
| **Requirement** | **TypedProposalProcedures** — In the authoritative Conway transaction-body type, proposal procedures are held as typed governance actions with their payloads, decoded by one closed decoder, never as opaque bytes, and the typed form re-encodes to the original bytes. |
| **Code** | `crates/ade_codec/src/conway/governance.rs`; `crates/ade_types/src/conway/governance.rs`; `crates/ade_codec/src/conway/tx.rs`; `crates/ade_testkit/src/governance/proposal_procedures_replay.rs` |
| **Tests** | `roundtrip_info_action_proposal`, `roundtrip_hard_fork_initiation`, `roundtrip_no_confidence`, `roundtrip_treasury_withdrawals`, `roundtrip_parameter_change`, `roundtrip_new_constitution`, `roundtrip_update_committee`, `roundtrip_multi_procedure`, `rejects_unknown_gov_action_tag`, `rejects_empty_proposal_procedures_set`, `rejects_trailing_garbage`, `rejects_truncated_proposal_procedure`, `rejects_invalid_stake_credential_in_update_committee`, `update_committee_keeps_stake_credential_discriminant`, `body_with_proposal_procedures_round_trips_through_typed_field`, `body_none_skips_proposal_procedures_key_20`, `body_and_sub_decoder_agree_on_proposal_procedures`, `canonical_corpus_decodes_to_expected_shape`, `canonical_corpus_round_trips_byte_identical`, `canonical_corpus_covers_all_gov_action_variants`, `canonical_corpus_includes_update_committee_discriminant_case` |
| **CI** | `ci/ci_check_proposal_procedures_closed.sh` |
| **Claim** | partial — `e563ad94` (pass) — observed: tests: 17 passed, 0 failed, 0 ignored, 4 not found; CI scripts: 1 passed, 0 failed — limits: (1) No IDSE cluster has declared acceptance obligations for this invariant yet (Doctrine §10); this record re-runs the checks carried from the legacy registry. Further limits: (2) 4 named test(s) were not found in the test output at this revision: canonical_corpus_decodes_to_expected_shape, canonical_corpus_round_trips_byte_identical, canonical_corpus_covers_all_gov_action_variants, canonical_corpus_includes_update_committee_discriminant_case |

_standard · local · scope: Conway transaction body · assumptions: A2 · depends on: VAL-001, DET-003 · secondary: VAL._ Legacy: DC-LEDGER-11

### STR — epoch

#### `STR-007` — _partial_

| Aspect | Location |
|--------|----------|
| **Source** | `docs/planning/epoch.define.consensus.md#batch-1` (approved by wdm33, 2026-11-08) |
| **Requirement** | **SingleEpochAuthority** — At every slot exactly one authoritative epoch view exists, holding that slot's epoch's leadership stake distribution, epoch nonce, constitutional committee, governance state and protocol parameters; one owner holds it, header validation and leadership both read it afresh for each decision, and crossing an epoch boundary replaces it in one atomic transition. |
| **Code** | `crates/ade_ledger/src/governance.rs`; `crates/ade_ledger/src/rules.rs`; `crates/ade_node/src/node_lifecycle.rs`; `crates/ade_ledger/src/consensus_view.rs`; `crates/ade_node/src/node_sync.rs`; `crates/ade_node/src/epoch_activation.rs`; `crates/ade_node/src/epoch_activate.rs`; `crates/ade_node/src/epoch_wire.rs`; `ci/ci_check_eview_atomic_authority.sh` |
| **Tests** | `cre_s4_3a_cross_path_gov_delta_is_identical`, `cre_s4_3a_replay_path_refunds_all_five_expiries`, `cre_s4_3a_potentially_ratifiable_terminals_both_paths`, `cre_s4_3a_rupd_consumed_once_with_governance_refund_at_seed_boundary`, `cre_s4_3a_single_governance_authority_no_second_path`, `cre_s4_3c_enactment_is_identical_on_replay_and_accumulator_paths`, `cre_s4_3c_supported_witness_enacts_prunes_siblings_refunds_all`, `cre_s4_3c_ratifiable_chain_child_of_winner_halts`, `cre_s4_3c_competing_ratifiable_siblings_halt`, `cre_s4_3c_pending_chain_child_is_carried_not_pruned`, `ce_lv15b0_1_a_vote_cast_during_epoch_e_enacts_at_e_plus_2`, `ce_lv15b0_5_7_changes_after_the_freeze_do_not_change_the_outcome`, `lv15b0_boundary_refuses_a_state_without_a_record`, `lv15b0_boundary_refuses_a_record_from_another_epoch`, `feed_header_validates_against_recovered_surface_not_empty_view`, `authority_epoch_guard_is_mode_aware_and_identity_is_exact`, `cross_consumer_identity_validation_and_forge_resolve_one_authority_view`, `seed_only_sole_view_cannot_validate_n1_header_rejects_before_acceptance`, `forge_continuity_required_missing_promotion_at_n1_is_terminal`, `node_forge_off_epoch_slot_fails_closed`, `recover_at_boundary_round_trips_the_durable_record_and_rejects_a_tamper`, `happy_path_promotes_after_durable_wal`, `crash_before_durable_wal_keeps_seed`, `crash_after_wal_republishes_same_view`, `recovered_view_mismatch_is_terminal`, `recover_at_boundary_wrong_cli_network_magic_is_terminal_no_partial_recovery` |
| **CI** | `ci/ci_check_feed_leader_threshold_view.sh` (red at e563ad94), `ci/ci_check_eview_atomic_authority.sh` |
| **Claim** | unresolved — `e563ad94` (fail) — observed: tests: 26 passed, 0 failed, 0 ignored, 0 not found; CI scripts: 1 passed, 1 failed; failing CI: ci/ci_check_feed_leader_threshold_view.sh — limits: (1) No IDSE cluster has declared acceptance obligations for this invariant yet (Doctrine §10); this record re-runs the checks carried from the legacy registry. Further limits: (2) No mutation witness exists yet for this critical invariant (Doctrine §28). (3) ci/ci_check_feed_leader_threshold_view.sh fails at this revision and already failed before the IDSE migration (a3cef7f7): "FAIL: from_seed_epoch_consensus_inputs must be defined exactly once (the single projection authority); found 2". Diagnosed as a stale or over-broad structural gate, not a demonstrated violation; the gate needs repair before it can serve as evidence. |

_critical · transactional · scope: consensus epoch authority (header validation and leadership) · assumptions: A2 · depends on: SAF-001, EVO-001 · secondary: SAF._ Legacy: T-EPOCH-01, CN-EPOCH-02, DC-EPOCH-14

### STR — consensus

#### `STR-008` — _partial_

| Aspect | Location |
|--------|----------|
| **Source** | `docs/planning/consensus.define.consensus.md#batch-1` (approved by wdm33, 2026-11-08) |
| **Requirement** | **ChainDepStateOwnedByConsensus** — The Praos chain-dependent state (its nonces, operational-certificate counters and last slot) is owned by the consensus layer, not the ledger. Moving forward along a chain, it changes only through validated headers and epoch ticks; a rollback or a verified snapshot restore sets it to the chain-dependent state of the restored point. |
| **Code** | `crates/ade_core/src/consensus/praos_state.rs`; `crates/ade_core/src/consensus/events.rs`; `crates/ade_core/src/consensus/errors.rs`; `crates/ade_core/src/consensus/encoding.rs`; `crates/ade_core/src/consensus/nonce.rs`; `crates/ade_core/src/consensus/op_cert.rs`; `crates/ade_core/src/consensus/header_validate.rs`; `crates/ade_core/src/consensus/header_summary.rs` |
| **Tests** | `consensus::praos_state::tests::op_cert_upsert_rejects_regression`, `consensus::praos_state::tests::op_cert_upsert_accepts_equal_counter_as_noop`, `consensus::praos_state::tests::op_cert_upsert_accepts_monotonic_increasing`, `consensus::praos_state::tests::genesis_state_is_well_formed`, `consensus::praos_state::tests::nonce_zero_constant_is_zero_bytes`, `consensus::encoding::tests::op_cert_counter_map_iteration_is_deterministic`, `layout_is_stable`, `roundtrip_empty_state`, `roundtrip_genesis_state`, `roundtrip_populated_state`, `roundtrip_chain_event_all_variants`, `roundtrip_chain_selection_reject_all_variants`, `decode_rejects_unknown_discriminant`, `decode_rejects_short_array`, `op_cert_counter_map_iteration_is_deterministic`, `consensus::nonce::tests::header_contribution_rejects_non_monotonic_slot`, `consensus::nonce::tests::header_contribution_advances_evolving_nonce_deterministically` **†**, `consensus::nonce::tests::header_contribution_does_not_touch_epoch_nonce` **†**, `consensus::nonce::tests::candidate_freeze_copies_evolving_to_candidate` **†**, `consensus::nonce::tests::candidate_freeze_does_not_advance_slot` **†**, `consensus::nonce::tests::epoch_boundary_promotes_candidate_to_epoch_nonce` **†**, `consensus::nonce::tests::epoch_boundary_rotates_previous_epoch_nonce` **†**, `consensus::nonce::tests::epoch_boundary_rejects_uninitialised_candidate` **†**, `consensus::nonce::tests::epoch_boundary_preserves_op_cert_counters` **†**, `consensus::nonce::tests::epoch_boundary_preserves_lab_nonce` **†**, `consensus::nonce::tests::blake2b_input_order_is_evolving_then_vrf_output_first_32` **†**, `within_epoch_evolving_nonce_matches_corpus`, `epoch_boundary_freezes_and_rotates_correctly`, `nonce_evolution_replay_is_deterministic`, `consensus::op_cert::tests::apply_op_cert_inserts_first_observation`, `consensus::op_cert::tests::apply_op_cert_advances_existing_strictly`, `consensus::op_cert::tests::apply_op_cert_accepts_equal_counter_as_noop`, `consensus::op_cert::tests::apply_op_cert_rejects_lower_counter`, `consensus::op_cert::tests::apply_op_cert_independent_kes_periods_dont_collide`, `consensus::op_cert::tests::apply_op_cert_independent_pools_dont_collide`, `consensus::op_cert::tests::apply_op_cert_does_not_touch_nonces`, `consensus::op_cert::tests::apply_op_cert_does_not_touch_last_slot_or_block_no`, `normal_progression_records_highest_counter_per_window`, `regression_after_progression_rejected_with_typed_error`, `op_cert_replay_is_deterministic`, `consensus::header_validate::tests::pipeline_short_circuits_on_first_failure`, `consensus::header_validate::tests::nonce_contribution_uses_nonce_role_vrf_output_not_leader_role`, `valid_header_accepted_advances_state`, `header_with_slot_regression_rejected`, `header_with_block_no_regression_rejected`, `header_with_op_cert_regression_rejected`, `header_with_invalid_vrf_proof_rejected`, `header_beyond_forecast_horizon_rejected`, `validate_replay_is_deterministic` |
| **CI** | `ci/ci_check_consensus_closed_enums.sh` |
| **Claim** | partial — `e563ad94` (pass) — observed: tests: 39 passed, 0 failed, 0 ignored, 10 not found; CI scripts: 1 passed, 0 failed — limits: (1) No IDSE cluster has declared acceptance obligations for this invariant yet (Doctrine §10); this record re-runs the checks carried from the legacy registry. Further limits: (2) No mutation witness exists yet for this critical invariant (Doctrine §28). (3) 10 named test(s) were not found in the test output at this revision: consensus::nonce::tests::header_contribution_advances_evolving_nonce_deterministically, consensus::nonce::tests::header_contribution_does_not_touch_epoch_nonce, consensus::nonce::tests::candidate_freeze_copies_evolving_to_candidate, consensus::nonce::tests::candidate_freeze_does_not_advance_slot, consensus::nonce::tests::epoch_boundary_promotes_candidate_to_epoch_nonce, consensus::nonce::tests::epoch_boundary_rotates_previous_epoch_nonce, consensus::nonce::tests::epoch_boundary_rejects_uninitialised_candidate, consensus::nonce::tests::epoch_boundary_preserves_op_cert_counters … |

_critical · local · scope: Praos chain-dependent state · assumptions: — · depends on: VAL-022 · secondary: DET._ Legacy: DC-CONS-04

#### `STR-009` — _partial_

| Aspect | Location |
|--------|----------|
| **Source** | `docs/planning/consensus.define.consensus.md#batch-1` (approved by wdm33, 2026-11-08) |
| **Requirement** | **EraScheduleAnchored** — The core receives the era schedule only as a typed value bound to the bootstrap anchor hash and recorded in replay evidence; genesis files and configuration are parsed only in the shell, and no core path reads files, JSON or operator configuration. |
| **Code** | `crates/ade_core/src/consensus/era_schedule.rs`; `crates/ade_runtime/src/consensus/genesis_parser.rs` |
| **Tests** | `consensus::era_schedule::tests::eraschedule_constructor_rejects_empty`, `consensus::era_schedule::tests::eraschedule_constructor_rejects_non_monotonic`, `consensus::era_schedule::tests::eraschedule_constructor_rejects_zero_slot_length`, `consensus::era_schedule::tests::eraschedule_constructor_rejects_zero_epoch_length`, `consensus::genesis_parser::tests::anchor_hash_deterministic`, `consensus::genesis_parser::tests::anchor_hash_distinguishes_inputs`, `mainnet_corpus_translation_matches_oracle`, `preprod_corpus_translation_matches_oracle`, `bootstrap_anchor_hash_distinguishes_genesis_variants`, `mainnet_parser_eras_match_corpus_oracle`, `preprod_parser_eras_match_corpus_oracle`, `parser_anchor_is_deterministic` |
| **CI** | `ci/ci_check_no_chaindb_in_consensus_blue.sh` |
| **Claim** | partial — `e563ad94` (pass) — observed: tests: 12 passed, 0 failed, 0 ignored, 0 not found; CI scripts: 1 passed, 0 failed — limits: (1) No IDSE cluster has declared acceptance obligations for this invariant yet (Doctrine §10); this record re-runs the checks carried from the legacy registry. Further limits: (2) No mutation witness exists yet for this critical invariant (Doctrine §28). |

_critical · local · scope: era schedule, shell/core boundary · assumptions: A1, A2 · depends on: STR-002, VAL-012 · secondary: DET._ Legacy: DC-CONS-07

### STR — consensus-inputs

#### `STR-010` — _partial_

| Aspect | Location |
|--------|----------|
| **Source** | `docs/planning/consensus-inputs.define.consensus.md#batch-1` (approved by wdm33, 2026-11-08) |
| **Requirement** | **SeedRecordClosedSchema** — The persisted seed-epoch consensus-input record is one closed type holding the seed epoch, the active-slot coefficient, the total active stake, each pool's active stake and VRF key hash in canonical key order, the epoch nonce, the epoch geometry and the consensus profile; its decoder accepts only the current format version and only the canonical encoding, so re-encoding a decoded record gives exactly its input bytes, and anything else is rejected. |
| **Code** | `crates/ade_ledger/src/seed_consensus_inputs.rs` |
| **Tests** | `seed_epoch_consensus_inputs_round_trips_byte_identical`, `seed_cinput_decode_rejects_unknown_version`, `seed_cinput_decode_rejects_noncanonical_or_duplicate_keys` |
| **CI** | _(no CI enforcement — gap)_ |
| **Claim** | partial — `e563ad94` (pass) — observed: tests: 3 passed, 0 failed, 0 ignored, 0 not found; CI scripts: 0 passed, 0 failed — limits: (1) No IDSE cluster has declared acceptance obligations for this invariant yet (Doctrine §10); this record re-runs the checks carried from the legacy registry. Further limits: (2) No mutation witness exists yet for this critical invariant (Doctrine §28). |

_critical · local · scope: seed-epoch consensus-input record · assumptions: — · depends on: VAL-001, DET-003, DET-007 · secondary: VAL._ Legacy: CN-CINPUT-01

### STR — durability-recovery

#### `STR-011` — _partial_

| Aspect | Location |
|--------|----------|
| **Source** | `docs/planning/durability-recovery.define.consensus.md#batch-1` (approved by wdm33, 2026-11-08) |
| **Requirement** | **SingleRollbackMaterializer** — Ledger and chain-dependent state at a rollback target are materialized by one authority only, from one stored snapshot and the stored blocks after it, applied through the normal block application and epoch-boundary transitions; the chain-dependent state is rebuilt with the same epoch nonce the live admission path used for each epoch replayed (the recovered seed-record nonce for the seed epoch, the evolved nonce after it, VAL-022); no parallel computation of rolled-back state exists, and VRF verification on the rollback path is never weakened. |
| **Code** | `crates/ade_ledger/src/rollback/materialize.rs`; `crates/ade_ledger/src/rollback/traits.rs`; `crates/ade_ledger/src/seed_consensus_inputs.rs`; `crates/ade_node/src/node_lifecycle.rs` |
| **Tests** | `materialize_returns_rollback_too_deep_when_no_snapshot`, `materialize_with_snapshot_at_target_returns_snapshot_state`, `materialize_with_snapshot_below_target_replays_forward`, `materialize_fails_closed_on_invalid_block`, `materialize_replay_forward_equals_direct_apply`, `rollback_materialize_overlays_recovered_eta0_replay_equivalent`, `rollback_materialize_does_not_bypass_vrf_on_wrong_eta0` |
| **CI** | `ci/ci_check_rollback_materialize_closure.sh`, `ci/ci_check_rollback_materialize_eta0.sh` |
| **Claim** | partial — `e563ad94` (pass) — observed: tests: 7 passed, 0 failed, 0 ignored, 0 not found; CI scripts: 2 passed, 0 failed — limits: (1) No IDSE cluster has declared acceptance obligations for this invariant yet (Doctrine §10); this record re-runs the checks carried from the legacy registry. Further limits: (2) No mutation witness exists yet for this critical invariant (Doctrine §28). (3) Corrects this invariant's previous evidence record at the same revision: the earlier record's test matching missed references written as '<crate> tests/<file>.rs::<name>' or '<crate>::<path>'; the test outcomes are the same run's (docs/evidence/idse-m2-reevidence.json). |

_critical · local · scope: rollback materialization · assumptions: A1, A2 · depends on: DET-010, VAL-022, VAL-036 · secondary: DET._ Legacy: CN-STORE-07, T-REC-06

### STR — network

#### `STR-012` — _partial_

| Aspect | Location |
|--------|----------|
| **Source** | `docs/planning/network.define.consensus.md#batch-1` (approved by wdm33, 2026-11-08) |
| **Requirement** | **SingleSessionAuthorities** — Each layer of the wire session has one authority: one mux frame encoder and decoder pair, one handshake reducer for each of node-to-node and node-to-client, and one session step reducer; no parallel implementation exists. |
| **Code** | `crates/ade_network/src/mux/frame.rs`; `crates/ade_network/src/handshake/transition.rs`; `crates/ade_network/src/session/core.rs` |
| **Tests** | `crates/ade_network/src/mux/frame.rs::tests::frame_roundtrip_byte_identical`, `crates/ade_network/src/session/handshake_driver.rs::tests::handshake_initiator_accepts_when_responder_supports_proposed_version`, `crates/ade_network/src/session/handshake_driver.rs::tests::handshake_initiator_rejects_on_no_overlap`, `crates/ade_network/src/session/core.rs::tests::session_step_two_runs_byte_identical`, `crates/ade_network/src/session/core.rs::tests::session_handshake_completion_transitions_state`, `crates/ade_network/src/session/core.rs::tests::session_outbound_frame_encodes_via_encode_frame` |
| **CI** | `ci/ci_check_mux_frame_closure.sh`, `ci/ci_check_handshake_closure.sh`, `ci/ci_check_session_core_closure.sh` |
| **Claim** | partial — `e563ad94` (pass) — observed: tests: 6 passed, 0 failed, 0 ignored, 0 not found; CI scripts: 3 passed, 0 failed — limits: (1) No IDSE cluster has declared acceptance obligations for this invariant yet (Doctrine §10); this record re-runs the checks carried from the legacy registry. Further limits: (2) No mutation witness exists yet for this critical invariant (Doctrine §28). |

_critical · local · scope: mux, handshake and session layers · assumptions: — · depends on: VAL-001._ Legacy: CN-SESS-01, CN-SESS-02, CN-SESS-03

### STR — node

#### `STR-013` — _partial_

| Aspect | Location |
|--------|----------|
| **Source** | `docs/planning/node.define.consensus.md#batch-1` (approved by wdm33, 2026-11-09) |
| **Requirement** | **SingleBootstrapAuthority** — The node's initial ledger state, chain-dependent state and chain tip at startup come from one bootstrap authority; first-run bootstrap from a verified source and warm start from durable state are branches of that one function, never parallel paths. |
| **Code** | `crates/ade_runtime/src/bootstrap.rs` |
| **Tests** | `crates/ade_runtime/src/bootstrap.rs::tests::bootstrap_cold_start_returns_genesis_when_empty`, `crates/ade_runtime/src/bootstrap.rs::tests::bootstrap_cold_start_without_genesis_errors`, `crates/ade_runtime/src/bootstrap.rs::tests::bootstrap_warm_start_materializes_from_persistent_snapshot`, `crates/ade_runtime/src/bootstrap.rs::tests::bootstrap_warm_start_equals_direct_materialize`, `crates/ade_runtime/src/bootstrap.rs::tests::bootstrap_two_runs_produce_byte_identical_state` |
| **CI** | `ci/ci_check_bootstrap_closure.sh`, `ci/ci_check_produce_mode_uses_bootstrap_initial_state.sh`, `ci/ci_check_node_mode_closure.sh` (red at e563ad94) |
| **Claim** | unresolved — `e563ad94` (fail) — observed: tests: 5 passed, 0 failed, 0 ignored, 0 not found; CI scripts: 2 passed, 1 failed; failing CI: ci/ci_check_node_mode_closure.sh — limits: (1) No IDSE cluster has declared acceptance obligations for this invariant yet (Doctrine §10); this record re-runs the checks carried from the legacy registry. Further limits: (2) No mutation witness exists yet for this critical invariant (Doctrine §28). (3) ci/ci_check_node_mode_closure.sh fails at this revision and already failed before the IDSE migration (a3cef7f7): "FAIL: Mode variant set mismatch.". Diagnosed as a stale or over-broad structural gate, not a demonstrated violation; the gate needs repair before it can serve as evidence. |

_critical · local · scope: node startup · assumptions: A2 · depends on: SEC-015, REC-004._ Legacy: CN-NODE-01

#### `STR-014` — _partial_

| Aspect | Location |
|--------|----------|
| **Source** | `docs/planning/node.define.consensus.md#batch-1` (approved by wdm33, 2026-11-09) |
| **Requirement** | **SingleLiveLifecycleOwner** — The node mode is the single owner of the live run: the run loop changes authoritative state only through the existing closed authorities (the bootstrap authority for initial state, the one admission path for tip advance), no second binary mode drives the live node, and the loop's planner emits only a closed lifecycle vocabulary that cannot express an authority decision. |
| **Code** | `crates/ade_node/src/node_lifecycle.rs` |
| **Tests** | `relay_loop_syncs_then_halts_clean_on_source_end`, `relay_loop_halts_clean_on_shutdown_no_partial_write`, `relay_loop_idles_then_syncs_on_incremental_feed`, `relay_loop_fails_closed_on_unapplyable_block`, `plan_loop_step_forge_precedence_table_is_total` |
| **CI** | _(no CI enforcement — gap)_ |
| **Claim** | partial — `e563ad94` (pass) — observed: tests: 5 passed, 0 failed, 0 ignored, 0 not found; CI scripts: 0 passed, 0 failed — limits: (1) No IDSE cluster has declared acceptance obligations for this invariant yet (Doctrine §10); this record re-runs the checks carried from the legacy registry. Further limits: (2) No mutation witness exists yet for this critical invariant (Doctrine §28). |

_critical · local · scope: node run loop · assumptions: — · depends on: STR-013, DUR-012._ Legacy: CN-NODE-02

#### `STR-015` — _partial_

| Aspect | Location |
|--------|----------|
| **Source** | `docs/planning/node.define.consensus.md#batch-1` (approved by wdm33, 2026-11-09) |
| **Requirement** | **OwnForgedAdmitThroughChokepoint** — A self-accepted forged block becomes part of the durable chain only by entering the same admission path as received blocks (store bytes, append log, advance tip, behind header and block validation, extending the tip only), with the bytes admitted byte-identical to the bytes self-acceptance validated; the forge has no tip-advance path of its own. |
| **Code** | `crates/ade_node/src/node_sync.rs`; `crates/ade_node/src/node_lifecycle.rs`; `crates/ade_runtime/src/forward_sync`; `crates/ade_ledger/src/receive`; `crates/ade_ledger/src/block_validity/header_position.rs` |
| **Tests** | `forge_tick_durable_admit_advances_tip`, `forge_successor_builds_block_1_from_durable_tip`, `forged_admit_bytes_byte_identical_to_self_accept`, `stale_tip_forge_fails_closed` |
| **CI** | `ci/ci_check_forged_durable_admit_via_pump.sh` (red at e563ad94), `ci/ci_check_node_run_loop_containment.sh` |
| **Claim** | unresolved — `e563ad94` (fail) — observed: tests: 4 passed, 0 failed, 0 ignored, 0 not found; CI scripts: 1 passed, 1 failed; failing CI: ci/ci_check_forged_durable_admit_via_pump.sh — limits: (1) No IDSE cluster has declared acceptance obligations for this invariant yet (Doctrine §10); this record re-runs the checks carried from the legacy registry. Further limits: (2) No mutation witness exists yet for this critical invariant (Doctrine §28). (3) ci/ci_check_forged_durable_admit_via_pump.sh fails at this revision and already failed before the IDSE migration (a3cef7f7): "FAIL (forged durable admit via pump): could not isolate admit_forged_block_durably body in node_sync.rs (signature moved/renamed?)". Diagnosed as a stale or over-broad structural gate, not a demonstrated violation; the gate needs repair before it can serve as evidence. |

_critical · transactional · scope: forge, admission · assumptions: A6 · depends on: DUR-012, VAL-009 · secondary: DUR._ Legacy: DC-NODE-12

#### `STR-016` — _partial_

| Aspect | Location |
|--------|----------|
| **Source** | `docs/planning/node.define.consensus.md#batch-1` (approved by wdm33, 2026-11-09) |
| **Requirement** | **ForkChoiceAppliedThroughAuthorities** — A chain-selection outcome changes durable state only through the existing authorities (lockstep admission and rollback, the single rollback materializer, the one admission path), and a fork-choice win becomes durable only when the new chain's bodies validate and apply; there is no header-only tip advance and no second apply or rollback path. |
| **Code** | `crates/ade_node/src/node_lifecycle.rs`; `crates/ade_ledger/src/receive/reducer.rs`; `crates/ade_ledger/src/rollback`; `crates/ade_runtime/src/forward_sync/pump.rs` |
| **Tests** | `apply_rolledback_rolls_back_and_appends_wal_record_after_commit`, `apply_chain_selected_invalid_body_fails_via_pump_no_advance`, `apply_chain_selected_without_block_bytes_fails_closed`, `participant_rollback_applies_durably`, `participant_block_with_no_durable_tip_pumps` |
| **CI** | `ci/ci_check_live_fork_choice_apply.sh` (red at e563ad94), `ci/ci_check_live_fork_choice_wiring.sh` |
| **Claim** | unresolved — `e563ad94` (fail) — observed: tests: 5 passed, 0 failed, 0 ignored, 0 not found; CI scripts: 1 passed, 1 failed; failing CI: ci/ci_check_live_fork_choice_apply.sh — limits: (1) No IDSE cluster has declared acceptance obligations for this invariant yet (Doctrine §10); this record re-runs the checks carried from the legacy registry. Further limits: (2) No mutation witness exists yet for this critical invariant (Doctrine §28). (3) ci/ci_check_live_fork_choice_apply.sh fails at this revision and already failed before the IDSE migration (a3cef7f7): "FAIL: durable_tip_matches ([legacy ref] reconciliation) missing". Diagnosed as a stale or over-broad structural gate, not a demonstrated violation; the gate needs repair before it can serve as evidence. |

_critical · transactional · scope: live fork choice · assumptions: A6 · depends on: REC-003, STR-011, DUR-012._ Legacy: DC-NODE-25

#### `STR-017` — _partial_

| Aspect | Location |
|--------|----------|
| **Source** | `docs/planning/node.define.consensus.md#batch-1` (approved by wdm33, 2026-11-09) |
| **Requirement** | **SingleSnapshotCadencePolicy** — The persistent snapshot writer captures a snapshot only when the one cadence policy schedules it; no orchestrator-side override and no second cadence policy exists. |
| **Code** | `crates/ade_runtime/src/rollback/persistent_writer.rs`; `crates/ade_runtime/src/orchestrator/core.rs` |
| **Tests** | `crates/ade_runtime/src/rollback/persistent_writer.rs::tests::persistent_writer_on_admitted_captures_only_on_cadence`, `crates/ade_runtime/src/rollback/persistent_writer.rs::tests::persistent_writer_round_trips_via_framing`, `crates/ade_runtime/src/rollback/persistent_writer.rs::tests::persistent_writer_force_capture_skips_cadence_but_updates_state`, `crates/ade_runtime/src/rollback/persistent_writer.rs::tests::persistent_writer_two_runs_are_deterministic`, `crates/ade_runtime/src/orchestrator/core.rs::tests::step_admit_triggers_capture_snapshot_at_cadence` |
| **CI** | `ci/ci_check_persistent_writer_no_parallel_cadence.sh` |
| **Claim** | partial — `e563ad94` (pass) — observed: tests: 5 passed, 0 failed, 0 ignored, 0 not found; CI scripts: 1 passed, 0 failed — limits: (1) No IDSE cluster has declared acceptance obligations for this invariant yet (Doctrine §10); this record re-runs the checks carried from the legacy registry. |

_standard · local · scope: snapshot writer · assumptions: — · depends on: DET-014 · secondary: DET._ Legacy: DC-NODE-02

### STR — pipeline-production

#### `STR-018` — _partial_

| Aspect | Location |
|--------|----------|
| **Source** | `docs/planning/pipeline-production.define.consensus.md#batch-1` (approved by wdm33, 2026-11-09) |
| **Requirement** | **ProduceChainEvolutionLinear** — In produce mode each forge's base is the post-state of the previous self-accepted forge, carried by a linear value that is consumed when it advances, so forging against a stale base cannot be expressed; it advances only when block validity's post-state and self-acceptance agree on the same inputs, a disagreement being a typed error that does not advance; it never constructs an accepted-block token itself; and for the same bootstrap seed, slot sequence and keys the series of block numbers, previous hashes, post-state fingerprints and forged bytes is byte-identical across runs. |
| **Code** | `crates/ade_runtime/src/producer/chain_evolution.rs` |
| **Tests** | `advance_threads_post_state_forward`, `advance_two_runs_byte_identical`, `advance_rejects_invalid_bytes`, `reconcile_verdicts_both_valid_ok`, `reconcile_verdicts_both_invalid_ok`, `reconcile_verdicts_valid_vs_reject_mismatches`, `reconcile_verdicts_reject_vs_valid_mismatches`, `served_snapshot_two_run_replay_byte_identical` |
| **CI** | _(no CI enforcement — gap)_ |
| **Claim** | partial — `e563ad94` (pass) — observed: tests: 8 passed, 0 failed, 0 ignored, 0 not found; CI scripts: 0 passed, 0 failed — limits: (1) No IDSE cluster has declared acceptance obligations for this invariant yet (Doctrine §10); this record re-runs the checks carried from the legacy registry. Further limits: (2) No mutation witness exists yet for this critical invariant (Doctrine §28). |

_critical · local · scope: produce mode chain evolution · assumptions: A1, A2 · depends on: SAF-020, VAL-059 · secondary: DET._ Legacy: DC-PROD-03

#### `STR-019` — _partial_

| Aspect | Location |
|--------|----------|
| **Source** | `docs/planning/pipeline-production.define.consensus.md#batch-1` (approved by wdm33, 2026-11-09) |
| **Requirement** | **SingleWirePumpPerPeer** — Exactly one wire pump drives each connected peer's chain-sync and block-fetch: it owns that peer's transport handle and is the only producer on that peer's event lane. It emits only the closed events block, tip update, roll backward and disconnected, never a verdict or validity claim; and every chain-sync reply carrying a tip emits exactly one closed event: a tip update for an intersect reply or a roll forward, and the distinct roll-backward event, carrying the peer's post-rollback tip, for a roll backward. |
| **Code** | `crates/ade_runtime/src/admission/wire_pump.rs` |
| **Tests** | `admission::wire_pump::tests::pump_emits_tip_update_and_request_next_on_intersect_found_no_block_fetch`, `admission::wire_pump::tests::pump_emits_tip_update_on_intersect_not_found`, `admission::wire_pump::tests::rollforward_drives_block_fetch_then_request_next`, `admission::wire_pump::tests::extract_chain_sync_header_point_returns_slot_and_hash`, `admission::wire_pump::tests::extract_chain_sync_header_point_rejects_malformed_envelope`, `admission::wire_pump::tests::lv16_pump_ends_on_intersect_not_found_without_request_next` |
| **CI** | `ci/ci_check_admission_wire_pump_closure.sh`, `ci/ci_check_admission_no_red_verdicts.sh`, `ci/ci_check_durable_intersection.sh` |
| **Claim** | partial — `e563ad94` (pass) — observed: tests: 6 passed, 0 failed, 0 ignored, 0 not found; CI scripts: 3 passed, 0 failed — limits: (1) No IDSE cluster has declared acceptance obligations for this invariant yet (Doctrine §10); this record re-runs the checks carried from the legacy registry. Further limits: (2) No mutation witness exists yet for this critical invariant (Doctrine §28). |

_critical · local · scope: per-peer wire pump (ade_runtime::admission::wire_pump) · assumptions: — · depends on: STR-012, SEC-017._ Legacy: CN-PUMP-01, DC-PUMP-01, DC-PUMP-02

#### `STR-020` — _partial_

| Aspect | Location |
|--------|----------|
| **Source** | `docs/planning/pipeline-production.define.consensus.md#batch-1` (approved by wdm33, 2026-11-09) |
| **Requirement** | **AdmissionModeSingleEntry** — Admission mode is entered through one runner, which composes the seed import, the bootstrap anchor, the bootstrap authority's warm start, the dialer, the per-block admission loop, the log append, the verdict reducer and the evidence log; the imported seed reaches the ledger only through one bridge that persists it as a snapshot at the seed point for the bootstrap authority's warm-start branch, never around that authority. |
| **Code** | `crates/ade_node/src/admission/runner.rs`; `crates/ade_node/src/admission/bootstrap.rs`; `crates/ade_node/src/admission/seed_to_snapshot.rs` |
| **Tests** | `ade_node::admission::runner::tests::run_admission_emits_shutdown_on_signal`, `ade_node::admission::runner::tests::run_admission_emits_shutdown_on_channel_close`, `ade_node::admission::runner::tests::run_admission_disconnect_to_zero_peers_clean_exit`, `ade_node::admission::seed_to_snapshot::tests::seed_to_snapshot_writes_via_persistent_cache`, `ade_node::admission::seed_to_snapshot::tests::seed_to_snapshot_returns_initial_ledger_fingerprint`, `ade_node::admission::seed_to_snapshot::tests::seed_to_snapshot_two_runs_byte_identical`, `ade_node::admission::seed_to_snapshot::tests::seed_to_snapshot_propagates_pre_conway_encode_error_as_authority_fatal` |
| **CI** | `ci/ci_check_admission_runner_closure.sh` (red at e563ad94), `ci/ci_check_admission_no_refscript_skip.sh` |
| **Claim** | unresolved — `e563ad94` (fail) — observed: tests: 7 passed, 0 failed, 0 ignored, 0 not found; CI scripts: 1 passed, 1 failed; failing CI: ci/ci_check_admission_runner_closure.sh — limits: (1) No IDSE cluster has declared acceptance obligations for this invariant yet (Doctrine §10); this record re-runs the checks carried from the legacy registry. Further limits: (2) ci/ci_check_admission_runner_closure.sh fails at this revision and already failed before the IDSE migration (a3cef7f7): "FAIL: AdmissionInputs carries Option<_> fields (must be all-required):". Diagnosed as a stale or over-broad structural gate, not a demonstrated violation; the gate needs repair before it can serve as evidence. (3) Corrects this invariant's previous evidence record at the same revision: the earlier record's test matching missed references written as '<crate> tests/<file>.rs::<name>' or '<crate>::<path>'; the test outcomes are the same run's (docs/evidence/idse-m2-reevidence.json). |

_standard · local · scope: admission mode · assumptions: — · depends on: STR-013._ Legacy: CN-ADMIT-01, CN-ADMIT-02

### STR — mithril-evidence

#### `STR-021` — _partial_

| Aspect | Location |
|--------|----------|
| **Source** | `docs/planning/mithril-evidence.define.consensus.md#batch-1` (approved by wdm33, 2026-11-09) |
| **Requirement** | **BootstrapAnchorSingleMint** — A bootstrap anchor is produced by one mint function, with every field required and none defaulted: the network magic, the genesis hash, the seed point's slot and block hash, the seed artifact hash, the imported UTxO fingerprint, the initial ledger fingerprint and the seed provenance. Its canonical encoding carries a schema version and round-trips byte-identically, and an unknown version is rejected. |
| **Code** | `crates/ade_runtime/src/bootstrap_anchor.rs`; `crates/ade_ledger/src/bootstrap_anchor/anchor.rs` |
| **Tests** | `crates/ade_runtime/src/bootstrap_anchor.rs::tests::mint_composes_inputs_byte_identically`, `crates/ade_runtime/src/bootstrap_anchor.rs::tests::mint_then_round_trip_via_canonical_cbor`, `crates/ade_runtime/src/bootstrap_anchor.rs::tests::mint_carries_seed_point_correctly`, `crates/ade_runtime/src/bootstrap_anchor.rs::tests::mint_propagates_utxo_fingerprint_into_anchor`, `crates/ade_ledger/src/bootstrap_anchor/anchor.rs::tests::bootstrap_anchor_match_is_exhaustive`, `crates/ade_ledger/src/bootstrap_anchor/anchor.rs::tests::bootstrap_anchor_round_trips_via_canonical_cbor`, `crates/ade_ledger/src/bootstrap_anchor/anchor.rs::tests::bootstrap_anchor_encode_two_runs_byte_identical`, `crates/ade_ledger/src/bootstrap_anchor/anchor.rs::tests::bootstrap_anchor_decode_rejects_unknown_version`, `crates/ade_ledger/src/bootstrap_anchor/anchor.rs::tests::bootstrap_anchor_decode_rejects_trailing_bytes`, `crates/ade_ledger/src/bootstrap_anchor/anchor.rs::tests::bootstrap_anchor_decode_rejects_short_buffer`, `crates/ade_ledger/src/bootstrap_anchor/anchor.rs::tests::bootstrap_anchor_decode_rejects_wrong_outer_array_length`, `crates/ade_ledger/src/bootstrap_anchor/anchor.rs::tests::bootstrap_anchor_decode_rejects_short_hash` |
| **CI** | `ci/ci_check_bootstrap_anchor_closure.sh` |
| **Claim** | partial — `e563ad94` (pass) — observed: tests: 12 passed, 0 failed, 0 ignored, 0 not found; CI scripts: 1 passed, 0 failed — limits: (1) No IDSE cluster has declared acceptance obligations for this invariant yet (Doctrine §10); this record re-runs the checks carried from the legacy registry. Further limits: (2) No mutation witness exists yet for this critical invariant (Doctrine §28). |

_critical · local · scope: bootstrap anchor · assumptions: — · depends on: DUR-008, DET-003 · secondary: DET._ Legacy: CN-ANCHOR-01, DC-ANCHOR-01

### STR — resources-release

#### `STR-022` — _partial_

| Aspect | Location |
|--------|----------|
| **Source** | `docs/planning/resources-release.define.consensus.md#batch-1` (approved by wdm33, 2026-11-09) |
| **Requirement** | **ClosedTxIngress** — Every transaction source (node-to-node submission, node-to-client submission, tests) reduces to one closed ingress event before mempool admission; the source variant is metadata for evidence, policy and replay only and never changes the validity verdict. |
| **Code** | `crates/ade_ledger/src/mempool/ingress.rs` |
| **Tests** | `ingress_preserves_tx_bytes_verbatim`, `ingress_source_is_closed_two_variants`, `ingress_admits_valid_tx_via_n2n`, `ingress_admits_valid_tx_via_n2c`, `ingress_rejects_invalid_tx_no_false_accept`, `ingress_source_does_not_change_verdict_valid`, `ingress_source_does_not_change_verdict_adversarial`, `ingress_equals_direct_admit_on_synthetic_corpus` |
| **CI** | `ci/ci_check_mempool_ingress_closure.sh` (red at e563ad94) |
| **Claim** | unresolved — `e563ad94` (fail) — observed: tests: 8 passed, 0 failed, 0 ignored, 0 not found; CI scripts: 0 passed, 1 failed; failing CI: ci/ci_check_mempool_ingress_closure.sh — limits: (1) No IDSE cluster has declared acceptance obligations for this invariant yet (Doctrine §10); this record re-runs the checks carried from the legacy registry. Further limits: (2) ci/ci_check_mempool_ingress_closure.sh fails at this revision and already failed before the IDSE migration (a3cef7f7): "FAIL: admit() called from outside sanctioned production sites (mempool/admit.rs definition + mempool/ingress.rs bridge):". Diagnosed as a stale or over-broad structural gate, not a demonstrated violation; the gate needs repair before it can serve as evidence. |

_standard · local · scope: transaction ingress · assumptions: — · depends on: STR-004, VAL-067._ Legacy: DC-MEM-03

---

## SAF — Safety

_Doctrine §6: Prevention of prohibited states, unsafe outcomes, and violations of fail-closed behavior._ 25 invariant(s).

### SAF — foundations

#### `SAF-001` — _partial_

| Aspect | Location |
|--------|----------|
| **Source** | `docs/planning/foundations.define.consensus.md#batch-1` (approved by wdm33, 2026-11-08) |
| **Requirement** | **AtomicTransitions** — Every authoritative state change is a transition that takes the prior state, the input and explicit context, and returns either the complete new state with its declared effects or a structured rejection; a rejection leaves the prior state exactly as it was. |
| **Code** | `crates/ade_ledger/src/block_validity`; `crates/ade_ledger/src/block_validity/transition.rs`; `crates/ade_ledger/src/tx_validity/transition.rs`; `crates/ade_ledger/src/tx_validity/verdict.rs` |
| **Tests** | `invalid_block_leaves_state_unchanged`, `valid_block_evolves_both_states`, `valid_tx_is_valid_and_applies`, `invalid_tx_leaves_state_unchanged`, `class_mapping_is_total` |
| **CI** | `ci/ci_check_consensus_closed_enums.sh` |
| **Claim** | partial — `e563ad94` (pass) — observed: tests: 5 passed, 0 failed, 0 ignored, 0 not found; CI scripts: 1 passed, 0 failed — limits: (1) No IDSE cluster has declared acceptance obligations for this invariant yet (Doctrine §10); this record re-runs the checks carried from the legacy registry. Further limits: (2) No mutation witness exists yet for this critical invariant (Doctrine §28). |

_critical · transactional · scope: ledger, consensus and chain-state transitions · assumptions: — · depends on: STR-001 · secondary: STR._ Legacy: T-CORE-03

#### `SAF-002` — _declared_

| Aspect | Location |
|--------|----------|
| **Source** | `docs/planning/foundations.define.consensus.md#batch-1` (approved by wdm33, 2026-11-08) |
| **Requirement** | **FailFastOnInvariantRisk** — When the node detects an internal inconsistency that could corrupt authoritative state, replay or chain selection, as opposed to an invalid external input, it halts deterministically instead of continuing. |
| **Code** | _(no enforcement found — gap)_ |
| **Tests** | _(no tests found — gap)_ |
| **CI** | _(no CI enforcement — gap)_ |
| **Claim** | unresolved — no evidence record |

_critical · local · scope: node-wide · assumptions: — · depends on: SAF-001._ Legacy: T-ERR-02

#### `SAF-003` — _partial_

| Aspect | Location |
|--------|----------|
| **Source** | `docs/planning/foundations.define.consensus.md#batch-1` (approved by wdm33, 2026-11-08) |
| **Requirement** | **SingleSpendPerHistory** — Along any one adopted chain history, no transaction output is consumed more than once, whether as a regular input or as collateral. |
| **Code** | `crates/ade_ledger/src/utxo.rs` |
| **Tests** | `check_duplicate_inputs_catches_dupes`, `duplicate_inputs_detected`, `all_eras_replay_summary` |
| **CI** | `ci/ci_check_differential_divergence.sh` |
| **Claim** | partial — `e563ad94` (pass) — observed: tests: 3 passed, 0 failed, 0 ignored, 0 not found; CI scripts: 1 passed, 0 failed — limits: (1) No IDSE cluster has declared acceptance obligations for this invariant yet (Doctrine §10); this record re-runs the checks carried from the legacy registry. Further limits: (2) No mutation witness exists yet for this critical invariant (Doctrine §28). |

_critical · transactional · scope: ledger UTxO · assumptions: A2 · depends on: SAF-001 · secondary: VAL._ Legacy: T-NOSPEND-01

### SAF — epoch

#### `SAF-004` — _partial_

| Aspect | Location |
|--------|----------|
| **Source** | `docs/planning/epoch.define.consensus.md#batch-1` (approved by wdm33, 2026-11-08); 1 recorded redefinition(s) in the registry |
| **Requirement** | **AtMostOneActivationPerEpoch** — Within one canonical chain history, at most one epoch view activates for each target epoch, and its durable activation record holds the view's complete identity: target epoch, network, era, transition point, source checkpoint commitment, snapshot phase, nonce commitment and the view's canonical hashes. A second record for the same target epoch on the same history is an idempotent replay if it is byte-identical to the first and a terminal conflict otherwise. A rollback that abandons an activation's transition point invalidates that activation record with the rollback, before any replacement activation for the epoch, and the record is never reused (DET-009). |
| **Code** | `crates/ade_ledger/src/wal/event.rs`; `crates/ade_ledger/src/reduced_epoch_view.rs`; `ci/ci_check_eview_activation_wal.sh` |
| **Tests** | `wal_epoch_view_activated_round_trips_byte_identical`, `wal_epoch_view_activated_uses_tag_four`, `activation_replay_idempotent_vs_conflict` |
| **CI** | `ci/ci_check_eview_activation_wal.sh` |
| **Claim** | partial — `e563ad94` (pass) — observed: tests: 3 passed, 0 failed, 0 ignored, 0 not found; CI scripts: 1 passed, 0 failed — limits: (1) No IDSE cluster has declared acceptance obligations for this invariant yet (Doctrine §10); this record re-runs the checks carried from the legacy registry. Further limits: (2) No mutation witness exists yet for this critical invariant (Doctrine §28). |

_critical · transactional · scope: epoch-view activation · assumptions: — · depends on: STR-007 · secondary: FIN._ Legacy: DC-EPOCH-04

#### `SAF-005` — _partial_

| Aspect | Location |
|--------|----------|
| **Source** | `docs/planning/epoch.define.consensus.md#batch-1` (approved by wdm33, 2026-11-08) |
| **Requirement** | **ActivationFailsClosed** — Only a genuine epoch transition, decided positively from the block's epoch against the accumulator's epoch before applying the block, enters the boundary machinery; a missing, stale, conflicting or mismatched candidate view is a terminal halt, never a fallback to another view, and an apply failure within an epoch never triggers boundary actions. |
| **Code** | `crates/ade_node/src/epoch_rebind.rs`; `crates/ade_node/src/node_sync.rs`; `ci/ci_check_eview_epoch_rebind.sh`; `crates/ade_node/src/epoch_activation.rs`; `ci/ci_check_eview_activation_predicate.sh`; `crates/ade_runtime/src/chaindb/epoch_accumulator_advance.rs`; `crates/ade_ledger/src/epoch_accumulator.rs`; `crates/ade_node/src/node_lifecycle.rs`; `ci/ci_check_bnd_typed_stall_cause.sh` |
| **Tests** | `simulated_transition_promotes_bound_n1_view`, `same_epoch_keeps_current`, `off_epoch_without_bound_view_fails_closed`, `not_immediate_next_fails_closed`, `unlocatable_fails_closed`, `rejects_each_wrong_binding`, `rejects_tampered_view_wrong_hash`, `replay_equivalent_deterministic`, `crash_restart_redrives_same_decision_both_sides`, `predicate_promotes_only_when_every_precondition_holds`, `predicate_rejects_each_failed_precondition`, `active_view_conflicting_promotion_is_terminal`, `a_within_epoch_apply_failure_is_apply_failed_not_a_boundary`, `a_crossing_is_classified_from_the_epochs_even_when_the_apply_would_fail`, `boundary_crossing_block_stalls_observe_only`, `over_chaindb_stops_at_boundary_observe_only` |
| **CI** | `ci/ci_check_eview_epoch_rebind.sh` (red at e563ad94), `ci/ci_check_eview_activation_predicate.sh` (red at e563ad94), `ci/ci_check_bnd_typed_stall_cause.sh` |
| **Claim** | unresolved — `e563ad94` (fail) — observed: tests: 16 passed, 0 failed, 0 ignored, 0 not found; CI scripts: 1 passed, 2 failed; failing CI: ci/ci_check_eview_epoch_rebind.sh, ci/ci_check_eview_activation_predicate.sh — limits: (1) No IDSE cluster has declared acceptance obligations for this invariant yet (Doctrine §10); this record re-runs the checks carried from the legacy registry. Further limits: (2) No mutation witness exists yet for this critical invariant (Doctrine §28). (3) ci/ci_check_eview_epoch_rebind.sh fails at this revision and already failed before the IDSE migration (a3cef7f7): "FAIL: the live seam does not pass None for the bound view -- OffEpoch must stay fail-closed until S3f-4". Diagnosed as a stale or over-broad structural gate, not a demonstrated violation; the gate needs repair before it can serve as evidence. (4) ci/ci_check_eview_activation_predicate.sh fails at this revision and already failed before the IDSE migration (a3cef7f7): "FAIL: the predicate does not check the selected-chain point". Diagnosed as a stale or over-broad structural gate, not a demonstrated violation; the gate needs repair before it can serve as evidence. |

_critical · local · scope: epoch-view activation, accumulator advance · assumptions: — · depends on: ORD-004, SAF-002 · secondary: VAL._ Legacy: DC-EPOCH-07, DC-EPOCH-39

#### `SAF-006` — _partial_

| Aspect | Location |
|--------|----------|
| **Source** | `docs/planning/epoch.define.consensus.md#batch-1` (approved by wdm33, 2026-11-08) |
| **Requirement** | **RewindRestoresCoherentLeadership** — A rewind restores the sealed leadership table to exactly the epochs valid at the rewind point, together with the accumulator in one atomic restore, so no sealed leadership object is ahead of the refolded accumulator. |
| **Code** | `crates/ade_runtime/src/chaindb/epoch_accumulator_store.rs`; `ci/ci_check_accumulator_refold_bound.sh` |
| **Tests** | `reset_to_settled_restores_pair_and_leaves_store_uncertified`, `settled_leadership_encoding_roundtrips_and_fails_closed_when_torn` |
| **CI** | `ci/ci_check_accumulator_refold_bound.sh` |
| **Claim** | partial — `e563ad94` (pass) — observed: tests: 2 passed, 0 failed, 0 ignored, 0 not found; CI scripts: 1 passed, 0 failed — limits: (1) No IDSE cluster has declared acceptance obligations for this invariant yet (Doctrine §10); this record re-runs the checks carried from the legacy registry. Further limits: (2) No mutation witness exists yet for this critical invariant (Doctrine §28). |

_critical · transactional · scope: epoch accumulator rewind, leadership table · assumptions: — · depends on: REC-001 · secondary: DET._ Legacy: DC-EPOCH-28

### SAF — consensus

#### `SAF-007` — _partial_

| Aspect | Location |
|--------|----------|
| **Source** | `docs/planning/consensus.define.consensus.md#batch-1` (approved by wdm33, 2026-11-08) |
| **Requirement** | **RollbackBoundedByK** — No rollback deeper than k blocks, counted in blocks and with k from the network's genesis, is ever applied: a deeper request, or one crossing the immutable tip, is rejected with a typed error and changes nothing; forecast and stability windows, though measured in slots, always cover at least k+1 blocks. |
| **Code** | `crates/ade_core/src/consensus/rollback.rs`; `crates/ade_core/src/consensus/events.rs` |
| **Tests** | `consensus::rollback::tests::rollback_preserves_immutable_tip`, `consensus::rollback::tests::rollback_preserves_security_param`, `consensus::rollback::tests::rollback_with_zero_depth_is_noop`, `consensus::rollback::tests::rollback_to_equal_block_no_as_immutable_succeeds`, `consensus::rollback::tests::rollback_to_one_below_immutable_rejected`, `rollback_within_k_succeeds`, `rollback_exceeding_k_rejected_with_typed_reason`, `rollback_before_immutable_tip_rejected`, `rollback_event_bytes_are_stable`, `rollback_is_deterministic`, `rollback_equivalent_to_truncated_replay` |
| **CI** | `ci/ci_check_consensus_closed_enums.sh`, `ci/ci_check_no_chaindb_in_consensus_blue.sh` |
| **Claim** | partial — `e563ad94` (pass) — observed: tests: 11 passed, 0 failed, 0 ignored, 0 not found; CI scripts: 2 passed, 0 failed — limits: (1) No IDSE cluster has declared acceptance obligations for this invariant yet (Doctrine §10); this record re-runs the checks carried from the legacy registry. Further limits: (2) No mutation witness exists yet for this critical invariant (Doctrine §28). |

_critical · transactional · scope: rollback and chain selection · assumptions: A2 · depends on: SAF-001 · secondary: FIN._ Legacy: DC-CONS-05

### SAF — consensus-inputs

#### `SAF-008` — _partial_

| Aspect | Location |
|--------|----------|
| **Source** | `docs/planning/consensus-inputs.define.consensus.md#batch-1` (approved by wdm33, 2026-11-08) |
| **Requirement** | **TransientViewStorageNonAuthoritative** — Transient epoch-view storage is never authority: it never survives as a source for following, forging, recovery or activation, nothing in it reaches a core output except through the deterministic derivation it feeds, and a crash at any point leaves the durable chain, write-ahead log and checkpoint unchanged and the next replay identical. Startup purges it before normal operation: it deletes only entries inside the transient store's own directory whose names have the deterministic window-key form, makes the deletion durable by syncing the parent directory, continues only when the directory is empty, and halts on an entry it does not recognize or cannot delete. |
| **Code** | `crates/ade_runtime/src/chaindb/transient_epoch_view.rs`; `crates/ade_runtime/src/bin/transient_view_kill_target.rs`; `crates/ade_runtime/tests/transient_view_kill_harness.rs`; `crates/ade_runtime/tests/transient_view_memory.rs`; `ci/ci_check_transient_view_memory_ceiling.sh`; `ci/ci_check_transient_view_no_fallback.sh`; `ci/ci_check_transient_view_not_live.sh` |
| **Tests** | `transient_root_is_owned_subtree_of_data_root`, `window_key_is_deterministic_and_binding_sensitive`, `window_key_validator_accepts_only_the_deterministic_form`, `purge_removes_valid_named_leftovers_and_leaves_subtree_empty`, `purge_fails_closed_on_a_foreign_artifact_and_deletes_nothing`, `purge_is_clean_on_an_empty_or_absent_subtree`, `lifecycle_open_materialize_iterate_dispose`, `dispose_is_clean_when_window_already_removed`, `transient_crash_mid_materialize_smoke`, `transient_crash_mid_dispose_smoke`, `transient_crash_1000` (#[ignore]), `durable_state_intact_after_transient_kill_loop`, `transient_materialization_rss_anon_delta_bounded`, `transient_materialization_is_repeatable_without_ratchet`, `local_rss_anon_reader_present_on_linux` |
| **CI** | `ci/ci_check_transient_view_memory_ceiling.sh`, `ci/ci_check_transient_view_no_fallback.sh`, `ci/ci_check_transient_view_not_live.sh` |
| **Claim** | partial — `e563ad94` (pass) — observed: tests: 14 passed, 0 failed, 1 ignored, 0 not found; CI scripts: 3 passed, 0 failed — limits: (1) No IDSE cluster has declared acceptance obligations for this invariant yet (Doctrine §10); this record re-runs the checks carried from the legacy registry. Further limits: (2) No mutation witness exists yet for this critical invariant (Doctrine §28). (3) 1 named test(s) are #[ignore]d and did not run. |

_critical · local · scope: transient epoch-view storage · assumptions: A1 · depends on: REC-002 · secondary: REC._ Legacy: DC-EVIEW-01

#### `SAF-009` — _partial_

| Aspect | Location |
|--------|----------|
| **Source** | `docs/planning/consensus-inputs.define.consensus.md#batch-1` (approved by wdm33, 2026-11-08); 1 recorded redefinition(s) in the registry |
| **Requirement** | **CandidateViewHashVerified** — Before an epoch view or sealed leadership object is activated or read, its canonical hash is recomputed over every binding (network, era, epoch, source point, checkpoint commitment, nonce, snapshot phase and profile commitment) and over its whole stake and VRF payload, and must equal its stored hash; its bindings cannot change once bound. The profile commitment it binds is blake2b-256 over a domain tag, the genesis hash, the protocol-parameter hash and the active-slot coefficient, all three taken from the durable consensus profile (DUR-005). |
| **Code** | `crates/ade_ledger/src/reduced_epoch_view.rs`; `ci/ci_check_eview_view_binding.sh` |
| **Tests** | `bind_is_deterministic_and_self_verifies`, `matches_exact_bindings_and_rejects_mismatch`, `canonical_hash_is_binding_sensitive`, `canonical_bytes_reproduce_the_hash`, `tampered_view_fails_verification`, `leadership_complete_required_for_matches` |
| **CI** | `ci/ci_check_eview_view_binding.sh`, `ci/ci_check_eview_leadership_complete.sh` (red at e563ad94) |
| **Claim** | unresolved — `e563ad94` (fail) — observed: tests: 6 passed, 0 failed, 0 ignored, 0 not found; CI scripts: 1 passed, 1 failed; failing CI: ci/ci_check_eview_leadership_complete.sh — limits: (1) No IDSE cluster has declared acceptance obligations for this invariant yet (Doctrine §10); this record re-runs the checks carried from the legacy registry. Further limits: (2) No mutation witness exists yet for this critical invariant (Doctrine §28). (3) ci/ci_check_eview_leadership_complete.sh fails at this revision and already failed before the IDSE migration (a3cef7f7): "FAIL: aggregate_pool_stake still skips zero-stake delegated pools (cardano numDelegators>0 violated)". Diagnosed as a stale or over-broad structural gate, not a demonstrated violation; the gate needs repair before it can serve as evidence. |

_critical · local · scope: epoch views and sealed leadership objects · assumptions: — · depends on: SAF-004, SAF-005, VAL-024, DUR-005 · secondary: DUR._ Legacy: DC-EVIEW-07

### SAF — network

#### `SAF-010` — _partial_

| Aspect | Location |
|--------|----------|
| **Source** | `docs/planning/network.define.consensus.md#batch-1` (approved by wdm33, 2026-11-08) |
| **Requirement** | **ServedChainPublishedAtomically** — A block becomes visible to peers only after it is published to the served chain in one atomic update, so no peer can observe a partially updated served chain; a failed publication shuts the node down fail-closed rather than continuing; and publishing the same sequence of blocks always produces a byte-identical served view, as measured by its fingerprint. |
| **Code** | `crates/ade_runtime/src/producer/served_chain_handle.rs`; `crates/ade_runtime/src/producer/broadcast_to_served.rs` |
| **Tests** | `handle_construction_yields_empty_snapshot`, `view_subscribe_creates_independent_receiver`, `served_tip_is_closed_value_type`, `drain_and_admit_is_deterministic_over_arrival_sequence`, `drain_and_admit_no_io_no_clock`, `drain_and_admit_admits_every_queued_block` |
| **CI** | _(no CI enforcement — gap)_ |
| **Claim** | partial — `e563ad94` (pass) — observed: tests: 6 passed, 0 failed, 0 ignored, 0 not found; CI scripts: 0 passed, 0 failed — limits: (1) No IDSE cluster has declared acceptance obligations for this invariant yet (Doctrine §10); this record re-runs the checks carried from the legacy registry. Further limits: (2) No mutation witness exists yet for this critical invariant (Doctrine §28). |

_critical · transactional · scope: served chain · assumptions: — · depends on: VAL-045 · secondary: DET._ Legacy: CN-SNAPSHOT-01, DC-SNAPSHOT-01

#### `SAF-011` — _partial_

| Aspect | Location |
|--------|----------|
| **Source** | `docs/planning/network.define.consensus.md#batch-1` (approved by wdm33, 2026-11-08) |
| **Requirement** | **HandshakeBeforeTraffic** — No mini-protocol frame from a peer is delivered for processing until that session's handshake has been accepted, and this holds by construction: a session still handshaking has no way to produce a delivery effect. |
| **Code** | `crates/ade_network/src/session/state.rs`; `crates/ade_network/src/session/core.rs`; `crates/ade_runtime/src/network/n2n_listener.rs`; `crates/ade_runtime/src/orchestrator/n2n_server_pump.rs` |
| **Tests** | `crates/ade_network/src/session/core.rs::tests::session_blocks_frames_before_handshake`, `crates/ade_network/src/session/core.rs::tests::session_post_handshake_handshake_frame_is_peer_fatal`, `n2n_listener_loopback_handshake_succeeds` |
| **CI** | `ci/ci_check_session_core_closure.sh`, `ci/ci_check_n2n_server_no_signing_dep.sh` |
| **Claim** | partial — `e563ad94` (pass) — observed: tests: 3 passed, 0 failed, 0 ignored, 0 not found; CI scripts: 2 passed, 0 failed — limits: (1) No IDSE cluster has declared acceptance obligations for this invariant yet (Doctrine §10); this record re-runs the checks carried from the legacy registry. Further limits: (2) No mutation witness exists yet for this critical invariant (Doctrine §28). |

_critical · local · scope: session layer · assumptions: — · depends on: EVO-006 · secondary: ORD._ Legacy: DC-SESS-01

### SAF — node

#### `SAF-012` — _partial_

| Aspect | Location |
|--------|----------|
| **Source** | `docs/planning/node.define.consensus.md#batch-1` (approved by wdm33, 2026-11-09) |
| **Requirement** | **ForgeOncePerSlotMonotone** — A forge is attempted at most once per slot and never for a slot at or before the last forged slot. |
| **Code** | `crates/ade_node/src/node_lifecycle.rs`; `crates/ade_node/src/run_loop_planner.rs` |
| **Tests** | `plan_loop_step_forge_precedence_table_is_total`, `forge_slot_guard_none_is_due`, `forge_slot_guard_at_most_once_per_slot`, `forge_slot_guard_rejects_past_slot`, `relay_loop_forge_slot_derived_via_clock_seam`, `relay_loop_forge_tick_attempts_forge_advances_no_tip`, `relay_loop_without_producer_material_matches_nfd_relay`, `relay_loop_forge_two_runs_byte_identical`, `forge_tick_rotated_kes_period_skips_no_retroactive_sign`, `forge_tick_off_epoch_slot_fails_closed_local` |
| **CI** | `ci/ci_check_loop_planner_closed.sh`, `ci/ci_check_node_run_loop_containment.sh`, `ci/ci_check_genesis_consistency_fixture_present.sh` |
| **Claim** | partial — `e563ad94` (pass) — observed: tests: 10 passed, 0 failed, 0 ignored, 0 not found; CI scripts: 3 passed, 0 failed — limits: (1) No IDSE cluster has declared acceptance obligations for this invariant yet (Doctrine §10); this record re-runs the checks carried from the legacy registry. Further limits: (2) No mutation witness exists yet for this critical invariant (Doctrine §28). |

_critical · local · scope: forge scheduling · assumptions: — · depends on: DET-019 · secondary: ORD._ Legacy: DC-NODE-05

#### `SAF-013` — _partial_

| Aspect | Location |
|--------|----------|
| **Source** | `docs/planning/node.define.consensus.md#batch-1` (approved by wdm33, 2026-11-09); 1 recorded redefinition(s) in the registry |
| **Requirement** | **ForgeAdmissibilityGatesOnly** — Every forge is admissible only when the durable servable tip equals the followed peer tip in hash and block number, except under VAL-053's fenced extension in a single-producer venue and under SAF-022's participant extension in a participant venue; the recovered anchor is never a forge base. The followed-peer-tip signal holds two pieces of evidence separately: the peer's advertised tip, and the tip it served that Ade durably admitted, written only after a successful admission from that admission's own validated tip and cleared by any rollback. The combined view prefers the served tip only when strictly higher, a tie with differing hashes falling to the advertisement; possession questions (the forge gate and the fork-switch fence) read the combined view, and convergence evidence reads the advertisement. The signal can only prevent a forge (a refusal for not being caught up is a typed refusal with no state change), never selects, replaces or reorders chains, and never changes durable state. |
| **Code** | `crates/ade_node/src/node_sync.rs`; `crates/ade_node/src/node_lifecycle.rs`; `crates/ade_runtime/src/forward_sync/pump.rs`; `ci/ci_check_followed_peer_tip_served_evidence.sh` |
| **Tests** | `forge_refused_not_caught_up`, `forge_base_falls_back_to_snapshot_anchor`, `forge_on_followed_tip_proceeds_with_parent_byte_equal`, `the_census_frontier_tuple_resolves_caught_up_once_service_is_evidence`, `a_catch_up_gap_keeps_the_advertisement_dominant_and_the_gate_refusing`, `a_self_forged_tip_is_not_served_evidence_and_the_gate_refuses`, `a_rollback_clears_the_served_fact_and_the_signal_falls_back_to_the_advertisement`, `a_source_that_has_admitted_nothing_offers_no_served_evidence`, `a_tie_at_the_same_height_resolves_to_the_advertisement_and_refuses`, `all_three_venue_routes_refuse_on_the_pre_fix_census_tuple` |
| **CI** | `ci/ci_check_forge_followed_tip_admission.sh`, `ci/ci_check_followed_peer_tip_served_evidence.sh` |
| **Claim** | partial — `e563ad94` (pass) — observed: tests: 10 passed, 0 failed, 0 ignored, 0 not found; CI scripts: 2 passed, 0 failed — limits: (1) No IDSE cluster has declared acceptance obligations for this invariant yet (Doctrine §10); this record re-runs the checks carried from the legacy registry. Further limits: (2) No mutation witness exists yet for this critical invariant (Doctrine §28). |

_critical · local · scope: forge admissibility · assumptions: — · depends on: VAL-031._ Legacy: DC-NODE-15, DC-NODE-17, DC-NODE-47

#### `SAF-014` — _partial_

| Aspect | Location |
|--------|----------|
| **Source** | `docs/planning/node.define.consensus.md#batch-1` (approved by wdm33, 2026-11-09) |
| **Requirement** | **NoForgeDuringPendingSelection** — While a peer candidate awaits a fork-choice decision, forging is refused with a typed reason until the decision is durably applied and reconciled or rejected with durable state unchanged; a forge never builds on the tip from before the decision. |
| **Code** | `crates/ade_node/src/node_sync.rs` |
| **Tests** | `pending_reselection_forge_refusal_gate`, `participant_rollback_beyond_k_fails_closed_clears_pending`, `singleproducer_rollback_refused_by_run_node_sync`, `forge_path_rollback_applies_durably`, `forge_path_rollback_beyond_k_fails_closed_clears_pending` |
| **CI** | `ci/ci_check_live_fork_choice_wiring.sh`, `ci/ci_check_participant_venue_inert.sh` |
| **Claim** | partial — `e563ad94` (pass) — observed: tests: 5 passed, 0 failed, 0 ignored, 0 not found; CI scripts: 2 passed, 0 failed — limits: (1) No IDSE cluster has declared acceptance obligations for this invariant yet (Doctrine §10); this record re-runs the checks carried from the legacy registry. Further limits: (2) No mutation witness exists yet for this critical invariant (Doctrine §28). |

_critical · local · scope: forge, live fork choice · assumptions: — · depends on: STR-016, REC-006 · secondary: ORD._ Legacy: DC-NODE-28

#### `SAF-015` — _partial_

| Aspect | Location |
|--------|----------|
| **Source** | `docs/planning/node.define.consensus.md#batch-1` (approved by wdm33, 2026-11-09) |
| **Requirement** | **VenueSplitForkChoice** — A candidate that needs fork choice is handled according to the declared venue: a single-producer venue refuses it, a participant venue passes the validated candidate to the chain selector, and an undeclared or unknown venue takes the refusing path; a raw followed-peer tip never reaches chain selection. |
| **Code** | `crates/ade_node/src/node_sync.rs`; `crates/ade_node/src/node_lifecycle.rs` |
| **Tests** | `resolve_singleproducer_competing_refuses`, `resolve_participant_competing_needs_fork_choice`, `resolve_participant_already_have_and_linear_extend_do_not_call_fork_choice`, `resolve_unknown_venue_fails_closed` |
| **CI** | `ci/ci_check_receive_detector_venue_split.sh` (red at e563ad94) |
| **Claim** | unresolved — `e563ad94` (fail) — observed: tests: 4 passed, 0 failed, 0 ignored, 0 not found; CI scripts: 0 passed, 1 failed; failing CI: ci/ci_check_receive_detector_venue_split.sh — limits: (1) No IDSE cluster has declared acceptance obligations for this invariant yet (Doctrine §10); this record re-runs the checks carried from the legacy registry. Further limits: (2) No mutation witness exists yet for this critical invariant (Doctrine §28). (3) ci/ci_check_receive_detector_venue_split.sh fails at this revision and already failed before the IDSE migration (a3cef7f7): "FAIL: missing: pub struct CandidateSummary". Diagnosed as a stale or over-broad structural gate, not a demonstrated violation; the gate needs repair before it can serve as evidence. |

_critical · local · scope: live fork choice · assumptions: — · depends on: VAL-054, VAL-031._ Legacy: DC-NODE-24

#### `SAF-016` — _partial_

| Aspect | Location |
|--------|----------|
| **Source** | `docs/planning/node.define.consensus.md#batch-1` (approved by wdm33, 2026-11-09) |
| **Requirement** | **AnchorRollbackNoOp** — On both the single-producer and the participant follow paths, a peer RollBackward to exactly the persisted recovered anchor point (slot and hash), checked before stored-block resolution, is an idempotent no-op with no log entry and no state change; the anchor is never turned into a servable stored block, and a RollBackward to Origin or to any other non-stored point fails closed. |
| **Code** | `crates/ade_runtime/src/forward_sync/reducer.rs`; `crates/ade_node/src/node_sync.rs`; `crates/ade_node/src/node_lifecycle.rs`; `crates/ade_runtime/src/admission/wire_pump.rs` |
| **Tests** | `crates/ade_node/tests/live_fork_choice_ai_s4bii.rs::ak_s2_rollback_to_recovered_anchor_is_idempotent_noop`, `crates/ade_node/tests/live_fork_choice_ai_s4bii.rs::ak_s2_rollback_to_origin_fails_closed_even_with_anchor`, `crates/ade_node/tests/live_fork_choice_ai_s4bii.rs::ak_s2_non_anchor_rollback_fails_closed_slot_and_hash_bound`, `crates/ade_node/tests/live_fork_choice_ai_s4bii.rs::ak_s2_no_recovered_anchor_still_fails_closed`, `crates/ade_node/tests/live_fork_choice_ai_s4bii.rs::ak_s2_after_anchor_noop_forward_block_reaches_pump_block_validation_holds`, `crates/ade_node/src/node_sync.rs::ak_s2_valid_forward_block_admits_after_recovered_anchor_noop`, `crates/ade_node/tests/live_fork_choice_ai_s4bii.rs::singleproducer_rollback_refused_by_run_node_sync`, `crates/ade_node/tests/live_fork_choice_ai_s4bii.rs::participant_rollback_to_recovered_anchor_is_noop`, `crates/ade_node/tests/live_fork_choice_ai_s4bii.rs::participant_rollback_origin_fails_closed`, `crates/ade_node/tests/live_fork_choice_ai_s4bii.rs::participant_rollback_non_anchor_fails_closed`, `crates/ade_node/tests/live_fork_choice_ai_s4bii.rs::participant_first_forward_after_anchor_noop_admits_via_pump_block`, `crates/ade_node/tests/live_fork_choice_ai_s4bii.rs::participant_stored_block_rollback_still_applies` |
| **CI** | _(no CI enforcement — gap)_ |
| **Claim** | partial — `e563ad94` (pass) — observed: tests: 11 passed, 0 failed, 0 ignored, 1 not found; CI scripts: 0 passed, 0 failed — limits: (1) No IDSE cluster has declared acceptance obligations for this invariant yet (Doctrine §10); this record re-runs the checks carried from the legacy registry. Further limits: (2) No mutation witness exists yet for this critical invariant (Doctrine §28). (3) 1 named test(s) were not found in the test output at this revision: crates/ade_node/src/node_sync.rs::ak_s2_valid_forward_block_admits_after_recovered_anchor_noop |

_critical · local · scope: live rollback · assumptions: — · depends on: SEC-020, REC-007._ Legacy: DC-NODE-32, DC-NODE-33

#### `SAF-017` — _partial_

| Aspect | Location |
|--------|----------|
| **Source** | `docs/planning/node.define.consensus.md#batch-1` (approved by wdm33, 2026-11-09) |
| **Requirement** | **ForgePathRollbackWithinPromotedEpoch** — On the forge and live-follow path, a peer RollBackward whose target lies in an epoch earlier than the promoted leadership authority's epoch fails closed before any rollback, log entry or state change. |
| **Code** | `crates/ade_node/src/node_sync.rs` |
| **Tests** | `forge_path_rollback_across_epoch_start_fails_closed` |
| **CI** | _(no CI enforcement — gap)_ |
| **Claim** | partial — `e563ad94` (pass) — observed: tests: 1 passed, 0 failed, 0 ignored, 0 not found; CI scripts: 0 passed, 0 failed — limits: (1) No IDSE cluster has declared acceptance obligations for this invariant yet (Doctrine §10); this record re-runs the checks carried from the legacy registry. Further limits: (2) No mutation witness exists yet for this critical invariant (Doctrine §28). |

_critical · local · scope: forge-path rollback · assumptions: — · depends on: STR-007, SEC-020 · secondary: ORD._ Legacy: DC-NODE-42

#### `SAF-018` — _partial_

| Aspect | Location |
|--------|----------|
| **Source** | `docs/planning/node.define.consensus.md#batch-1` (approved by wdm33, 2026-11-09) |
| **Requirement** | **ReceiveIdempotentByHash** — On linear admission against the current tip, a peer-delivered block already durably stored with the same slot and hash is an idempotent no-op (no validation step, no log entry, no tip change), and a different block at or before the last applied slot is not short-circuited but reaches header validation against the current tip, which rejects it; the decision compares hashes, never slots alone. A block belonging to a competing branch is handled by fork choice and validated against its own fork state, not by this rule. |
| **Code** | `crates/ade_runtime/src/forward_sync/pump.rs` |
| **Tests** | `pump_block_reannounced_block_is_idempotent_noop`, `pump_block_different_block_at_or_before_tip_still_fails_closed`, `run_node_sync_survives_reannounced_block_in_feed` |
| **CI** | `ci/ci_check_receive_idempotency.sh` |
| **Claim** | partial — `e563ad94` (pass) — observed: tests: 3 passed, 0 failed, 0 ignored, 0 not found; CI scripts: 1 passed, 0 failed — limits: (1) No IDSE cluster has declared acceptance obligations for this invariant yet (Doctrine §10); this record re-runs the checks carried from the legacy registry. Further limits: (2) No mutation witness exists yet for this critical invariant (Doctrine §28). |

_critical · local · scope: admission · assumptions: — · depends on: DUR-012 · secondary: DET._ Legacy: DC-NODE-16

#### `SAF-019` — _partial_

| Aspect | Location |
|--------|----------|
| **Source** | `docs/planning/node.define.consensus.md#batch-1` (approved by wdm33, 2026-11-09) |
| **Requirement** | **AuthorityFatalExit** — An authority-fatal error (a failed chain write on a committed rollback, or an unknown snapshot version or fingerprint mismatch at bootstrap, among others) halts the process deterministically with a non-zero exit code, with no retry of the failed write and no fallback decode. |
| **Code** | `crates/ade_node/src/node.rs`; `crates/ade_node/src/main.rs` |
| **Tests** | `crates/ade_node/tests/shutdown_resume_identity.rs::shutdown_then_resume_produces_byte_identical_state`, `crates/ade_node/tests/shutdown_resume_identity.rs::shutdown_clean_exits_with_evidence`, `crates/ade_node/tests/shutdown_resume_identity.rs::cold_start_without_genesis_fails_with_generic_startup_code`, `crates/ade_node/tests/authority_fatal_decode.rs::binary_halts_on_authority_fatal_decode_error` |
| **CI** | `ci/ci_check_node_binary_uses_single_bootstrap.sh` |
| **Claim** | partial — `e563ad94` (pass) — observed: tests: 4 passed, 0 failed, 0 ignored, 0 not found; CI scripts: 1 passed, 0 failed — limits: (1) No IDSE cluster has declared acceptance obligations for this invariant yet (Doctrine §10); this record re-runs the checks carried from the legacy registry. Further limits: (2) No mutation witness exists yet for this critical invariant (Doctrine §28). |

_critical · local · scope: node process · assumptions: — · depends on: SAF-002._ Legacy: DC-NODE-04

### SAF — pipeline-production

#### `SAF-020` — _partial_

| Aspect | Location |
|--------|----------|
| **Source** | `docs/planning/pipeline-production.define.consensus.md#batch-1` (approved by wdm33, 2026-11-09) |
| **Requirement** | **ForgedBlockSelfAccepted** — A forge attempt ends in exactly one of three closed outcomes: forged and self-accepted, not leader (with a bounded VRF fingerprint), or failed with a structured reason. It is forged and self-accepted only if the forged bytes decode through the block decoder and full block validity (VAL-009) accepts those exact bytes against the forge base; a self-acceptance rejection is a structured failure that hands nothing on. No forged byte is served, announced, broadcast or admitted except as the bytes self-acceptance accepted, and the node keeps following after a failure, the failure being recorded as that tick's outcome. |
| **Code** | `crates/ade_ledger/src/producer/self_accept.rs`; `crates/ade_ledger/src/block_validity/transition.rs`; `crates/ade_ledger/src/producer/served_chain.rs`; `crates/ade_runtime/src/network/served_chain_projection.rs`; `crates/ade_runtime/src/network/serve_dispatch.rs`; `crates/ade_ledger/src/receive/admitted.rs`; `crates/ade_ledger/src/receive/chain_write.rs`; `crates/ade_node/src/produce_mode.rs`; `crates/ade_ledger/src/producer/forge.rs` |
| **Tests** | `self_accept_accepts_freshly_forged_block`, `self_accept_rejects_corrupted_body_hash`, `self_accept_rejects_invalid_kes_signature`, `self_accept_rejects_unbalanced_tx_in_body`, `broadcast_callable_only_with_accept_verdict`, `served_chain_admit_admits_corpus_block`, `served_chain_admit_idempotent_on_byte_identity`, `served_chain_admit_independent_of_order`, `served_chain_snapshot_iteration_is_btreemap_ordered`, `served_chain_block_bytes_accessor_returns_accepted_block_slice`, `served_chain_range_bytes_returns_inclusive_window`, `served_chain_fingerprint_replay_byte_identical`, `admit_via_block_validity_accepts_corpus_block`, `admit_via_block_validity_rejects_corrupted_body`, `receive_apply_block_delivered_with_matching_header_admits`, `zero_stake_answer_emits_forge_not_leader`, `kes_period_outside_window_emits_forge_failed_kes_period_mismatch`, `full_stake_answer_reaches_self_accept_and_rejects`, `run_real_forge_is_byte_identical_across_two_runs`, `forge_block_accepts_empty_mempool`, `forge_block_empty_mempool_produces_empty_body`, `produce_mode_starts_runs_three_slots_and_exits_via_max_slots`, `forge_to_self_accept_succeeds` |
| **CI** | `ci/ci_check_self_accept_gate.sh`, `ci/ci_check_served_chain_closure.sh`, `ci/ci_check_admitted_block_closure.sh`, `ci/ci_check_receive_reducer_closure.sh`, `ci/ci_check_producer_coordinator_no_secrets.sh`, `ci/ci_check_no_independent_forge_codepath.sh` |
| **Claim** | partial — `e563ad94` (pass) — observed: tests: 23 passed, 0 failed, 0 ignored, 0 not found; CI scripts: 6 passed, 0 failed — limits: (1) No IDSE cluster has declared acceptance obligations for this invariant yet (Doctrine §10); this record re-runs the checks carried from the legacy registry. Further limits: (2) No mutation witness exists yet for this critical invariant (Doctrine §28). |

_critical · local · scope: every forge path (node and produce modes) · assumptions: — · depends on: VAL-009, VAL-057, OBS-001 · secondary: VAL._ Legacy: CN-CONS-07, CN-FORGE-01

#### `SAF-021` — _partial_

| Aspect | Location |
|--------|----------|
| **Source** | `docs/planning/pipeline-production.define.consensus.md#batch-1` (approved by wdm33, 2026-11-09) |
| **Requirement** | **ForgeEpochBoundToActiveAuthority** — A forge for a slot in epoch E uses only the active epoch authority's view for exactly E (its stake distribution and epoch nonce); the authority is consulted before any leader check or signing. If the authority holds an earlier epoch than E and the node is a producer whose support ends at its seed epoch, the slot is a typed no-forge and the node keeps following; if the authority is ahead of E, or E's promotion is missing where continuity is required, the forge halts with a typed terminal error. A block is never signed for epoch E with another epoch's nonce or stake distribution. |
| **Code** | `crates/ade_node/src/node_sync.rs`; `crates/ade_node/src/node_lifecycle.rs`; `crates/ade_node/src/run_loop_planner.rs`; `crates/ade_core/src/consensus/nonce.rs` |
| **Tests** | `forge_epoch_admission_within_seed_epoch_admits`, `forge_epoch_admission_off_epoch_fails_closed`, `forge_epoch_admission_unlocatable_fails_closed`, `node_forge_off_epoch_slot_fails_closed`, `node_forge_no_epoch_boundary_promotion_on_forge_path`, `forge_tick_off_epoch_slot_fails_closed_local` |
| **CI** | `ci/ci_check_node_forge_single_epoch_fail_closed.sh` (red at e563ad94) |
| **Claim** | unresolved — `e563ad94` (fail) — observed: tests: 6 passed, 0 failed, 0 ignored, 0 not found; CI scripts: 0 passed, 1 failed; failing CI: ci/ci_check_node_forge_single_epoch_fail_closed.sh — limits: (1) No IDSE cluster has declared acceptance obligations for this invariant yet (Doctrine §10); this record re-runs the checks carried from the legacy registry. Further limits: (2) No mutation witness exists yet for this critical invariant (Doctrine §28). (3) ci/ci_check_node_forge_single_epoch_fail_closed.sh fails at this revision and already failed before the IDSE migration (a3cef7f7): "FAIL: could not isolate forge_one_from_recovered in crates/ade_node/src/node_sync.rs". Diagnosed as a stale or over-broad structural gate, not a demonstrated violation; the gate needs repair before it can serve as evidence. |

_critical · local · scope: forge path, epoch boundaries · assumptions: — · depends on: STR-007, VAL-024, SAF-005 · secondary: VAL._ Legacy: DC-EPOCH-03

#### `SAF-022` — _partial_

| Aspect | Location |
|--------|----------|
| **Source** | `docs/planning/pipeline-production.define.consensus.md#batch-1` (approved by wdm33, 2026-11-09) |
| **Requirement** | **ParticipantForgeOnSelectedHead** — In a declared participant venue, forging first passes SAF-013's catch-up gate; from the first instant the durable servable tip equals the followed peer tip, it enters a participant extension whose forge base at each decision is the live selected durable tip. A decision in the extension refuses with a typed reason when the venue is not declared participant, when any fork-choice decision is pending (a reselection, a fork switch or a missing bridge; SAF-014), or when no durable servable tip exists, and it never gates on the tip of Ade's own last forge. Immediately before signing, the durable tip is read again and the forge is refused if it differs from the decision's base. The decision is a pure total function over closed types that never invokes chain selection and holds no signing material; the extension is not persisted, a restart entering it again only through the catch-up gate. |
| **Code** | `crates/ade_node/src/node_sync.rs`; `crates/ade_node/src/node_lifecycle.rs`; `crates/ade_node/src/live_log/sched_event.rs`; `ci/ci_check_participant_forge_on_selected_head.sh` |
| **Tests** | `participant_venue_forges_on_ao_selected_head_when_leader`, `participant_forge_base_is_ao_selected_chaindb_tip`, `participant_forge_base_is_servable_before_forge`, `participant_forge_refused_while_fork_choice_pending`, `participant_venue_requires_forge_activation`, `single_producer_forge_decision_unchanged`, `orphaned_startup_holds_forge_fence_participant`, `participant_forge_two_runs_byte_identical`, `keyed_participant_extend_survives_peer_admit_and_reaches_leader_check`, `participant_forge_refuses_if_tip_changes_between_decision_and_sign` |
| **CI** | `ci/ci_check_participant_forge_on_selected_head.sh` (red at e563ad94) |
| **Claim** | unresolved — `e563ad94` (fail) — observed: tests: 10 passed, 0 failed, 0 ignored, 0 not found; CI scripts: 0 passed, 1 failed; failing CI: ci/ci_check_participant_forge_on_selected_head.sh — limits: (1) No IDSE cluster has declared acceptance obligations for this invariant yet (Doctrine §10); this record re-runs the checks carried from the legacy registry. Further limits: (2) No mutation witness exists yet for this critical invariant (Doctrine §28). (3) ci/ci_check_participant_forge_on_selected_head.sh fails at this revision and already failed before the IDSE migration (a3cef7f7): "FAIL (participant-forge-on-selected-head): pub fn participant_forge_decision not found in crates/ade_node/src/node_sync.rs". Diagnosed as a stale or over-broad structural gate, not a demonstrated violation; the gate needs repair before it can serve as evidence. |

_critical · local · scope: node mode, participant venue forge decision · assumptions: A1 · depends on: SAF-013, SAF-014, VAL-031, STR-015 · secondary: LIV._ Legacy: CN-FOLLOW-01, DC-FOLLOW-FORGE-01

#### `SAF-023` — _partial_

| Aspect | Location |
|--------|----------|
| **Source** | `docs/planning/pipeline-production.define.consensus.md#batch-1` (approved by wdm33, 2026-11-09) |
| **Requirement** | **ProduceServesOnlySelfAccepted** — In produce mode every broadcast reconstructs the accepted block from the forged bytes through self-acceptance against the pre-forge base and publishes it to the served chain through the one atomic publication (SAF-010) before the next slot tick; if self-acceptance rejects, nothing is published and a structured broadcast error is recorded; and a block-served event is emitted only for a block present in the served chain. |
| **Code** | `crates/ade_node/src/produce_mode.rs`; `crates/ade_runtime/src/producer/served_chain_handle.rs`; `crates/ade_ledger/src/producer/self_accept.rs` |
| **Tests** | `broadcast_pushes_self_accepted_block_to_served`, `broadcast_rejects_non_self_accepted_block`, `forge_to_served_block_fetch_roundtrip` |
| **CI** | _(no CI enforcement — gap)_ |
| **Claim** | partial — `e563ad94` (pass) — observed: tests: 2 passed, 0 failed, 0 ignored, 1 not found; CI scripts: 0 passed, 0 failed — limits: (1) No IDSE cluster has declared acceptance obligations for this invariant yet (Doctrine §10); this record re-runs the checks carried from the legacy registry. Further limits: (2) No mutation witness exists yet for this critical invariant (Doctrine §28). (3) 1 named test(s) were not found in the test output at this revision: broadcast_rejects_non_self_accepted_block |

_critical · local · scope: produce mode serving · assumptions: — · depends on: SAF-020, SAF-010 · secondary: OBS._ Legacy: CN-PROD-04

#### `SAF-024` — _partial_

| Aspect | Location |
|--------|----------|
| **Source** | `docs/planning/pipeline-production.define.consensus.md#batch-1` (approved by wdm33, 2026-11-09) |
| **Requirement** | **AdmissionDivergenceHalts** — In admission mode a block whose slot lies outside the consensus inputs' epoch window, and undecodable peer bytes, are stopped before block validity is called; they, and a diverged or input-not-found verdict derived from admission evaluation, each halt the run with a distinct non-zero exit code and a typed halt event; undecodable bytes are classified as diverged (when the peer has a tip at that slot) or as a typed undecodable-bytes halt, never as input not found and never as a clean exit. The reducer stays pure; the halt is the binary's response. |
| **Code** | `crates/ade_node/src/admission/runner.rs` |
| **Tests** | `ade_node::admission::runner::tests::exit_code_constants_round_trip_to_i32`, `ade_node::admission::runner::tests::halt_for_verdict_only_diverged_or_input_not_found_halts`, `admission_replay_equivalence::admission_exit_codes_match_registered_values`, `cross_epoch_block_triggers_halt_without_admit`, `admission_log::event::tests::admission_log_event_discriminator_round_trips_for_each_variant` |
| **CI** | `ci/ci_check_admission_runner_closure.sh` (red at e563ad94), `ci/ci_check_live_ledger_view_epoch_window.sh`, `ci/ci_check_admission_wire_pump_closure.sh` |
| **Claim** | unresolved — `e563ad94` (fail) — observed: tests: 5 passed, 0 failed, 0 ignored, 0 not found; CI scripts: 2 passed, 1 failed; failing CI: ci/ci_check_admission_runner_closure.sh — limits: (1) No IDSE cluster has declared acceptance obligations for this invariant yet (Doctrine §10); this record re-runs the checks carried from the legacy registry. Further limits: (2) No mutation witness exists yet for this critical invariant (Doctrine §28). (3) ci/ci_check_admission_runner_closure.sh fails at this revision and already failed before the IDSE migration (a3cef7f7): "FAIL: AdmissionInputs carries Option<_> fields (must be all-required):". Diagnosed as a stale or over-broad structural gate, not a demonstrated violation; the gate needs repair before it can serve as evidence. (4) Corrects this invariant's previous evidence record at the same revision: the earlier record's test matching missed references written as '<crate> tests/<file>.rs::<name>' or '<crate>::<path>'; the test outcomes are the same run's (docs/evidence/idse-m2-reevidence.json). |

_critical · local · scope: admission mode · assumptions: — · depends on: OBS-006, VAL-035, SAF-019._ Legacy: DC-ADMIT-03, DC-ADMIT-11, DC-ADMIT-12

### SAF — resources-release

#### `SAF-025` — _partial_

| Aspect | Location |
|--------|----------|
| **Source** | `docs/planning/resources-release.define.consensus.md#batch-1` (approved by wdm33, 2026-11-09) |
| **Requirement** | **BoundedUtxoMemory** — The in-memory part of the UTxO (the read cache and the last-k change log) is bounded by fixed constants no option can change, and reaching a bound never changes an authoritative output. |
| **Code** | `crates/ade_ledger/src/utxo_overlay.rs`; `ci/ci_check_overlay_utxo_s2a.sh` |
| **Tests** | `overlay_matches_btreemap_across_a_sequence`, `compact_preserves_effective_set_and_clears_overlay`, `clone_shares_anchor_and_is_independent`, `s2a_overlay_split_fingerprints_identically_to_direct_build` |
| **CI** | `ci/ci_check_overlay_utxo_s2a.sh` |
| **Claim** | partial — `e563ad94` (pass) — observed: tests: 4 passed, 0 failed, 0 ignored, 0 not found; CI scripts: 1 passed, 0 failed — limits: (1) No IDSE cluster has declared acceptance obligations for this invariant yet (Doctrine §10); this record re-runs the checks carried from the legacy registry. Further limits: (2) No mutation witness exists yet for this critical invariant (Doctrine §28). |

_critical · local · scope: UTxO backend · assumptions: — · depends on: DET-025._ Legacy: DC-MEM-07

---

## LIV — Liveness

_Doctrine §6: Required progress and eventual completion under explicitly stated assumptions._ 11 invariant(s).

### LIV — epoch

#### `LIV-001` — _partial_

| Aspect | Location |
|--------|----------|
| **Source** | `docs/planning/epoch.define.consensus.md#batch-1` (approved by wdm33, 2026-11-08); 1 recorded redefinition(s) in the registry |
| **Requirement** | **BoundedRefold** — When a settled rewind point exists and every proof about it holds (canonical at its slot, at least k blocks below the tip, at or before the rollback target, fingerprint verified), re-deriving the epoch accumulator after a rollback replays at most 2k blocks, independent of how long the node has run: once the chain has advanced 2k blocks past bootstrap, the settled point is between k and 2k blocks below the tip, and it survives the recovery pass that follows a durable rollback, being re-proved against the chain as it stands after the rollback commits rather than discarded. Before a settled point exists, or when a proof fails, the refold starts from the certified bootstrap baseline (REC-001, DUR-002). |
| **Code** | `crates/ade_runtime/src/chaindb/epoch_accumulator_store.rs`; `ci/ci_check_accumulator_refold_bound.sh`; `crates/ade_node/src/node_lifecycle.rs`; `ci/ci_check_settled_rewind_survives_recovery.sh` |
| **Tests** | `settled_point_never_falls_further_than_2k_behind_the_tip`, `settled_point_is_only_promoted_once_k_blocks_settled`, `a_recertified_settled_point_makes_the_next_pass_forward_fold`, `recertification_refuses_a_settled_point_the_new_chain_abandoned`, `recertification_refuses_a_settled_point_not_k_settled_against_the_new_tip`, `recertify_settled_anchor_writes_the_anchor_at_the_settled_point`, `recertify_refuses_a_triple_whose_fingerprint_does_not_verify`, `recertify_refuses_when_the_cursor_is_not_at_the_settled_point`, `recertify_refuses_when_no_settled_triple_exists` |
| **CI** | `ci/ci_check_accumulator_refold_bound.sh`, `ci/ci_check_settled_rewind_survives_recovery.sh` |
| **Claim** | unresolved — `e563ad94` (pass) — observed: tests: 9 passed, 0 failed, 0 ignored, 0 not found; CI scripts: 2 passed, 0 failed — limits: (1) No IDSE cluster has declared acceptance obligations for this invariant yet (Doctrine §10); this record re-runs the checks carried from the legacy registry. Further limits: (2) No mutation witness exists yet for this critical invariant (Doctrine §28). (3) Open defect LV-17 (observed 2026-11-06, local): recovery deletes a valid settled triple, so each rollback triggers a rebuild of about 10 hours. No check at this revision reproduces it. |

_critical · local · scope: epoch accumulator rewind and recovery · assumptions: A1, A2, A4 · depends on: REC-001, DUR-002 · secondary: REC._ Legacy: DC-EPOCH-30, DC-EPOCH-35

#### `LIV-002` — _partial_

| Aspect | Location |
|--------|----------|
| **Source** | `docs/planning/epoch.define.consensus.md#batch-1` (approved by wdm33, 2026-11-08) |
| **Requirement** | **CoAdvanceRetryOnce** — A co-advance pass never returns with leadership authority stale only because the reduced checkpoint had not yet reached data the accumulator needs: exactly one failure is retryable, an unresolved collateral input at a block the checkpoint has not reached within the accumulator's own epoch, and the pass then advances the checkpoint to that block and retries once in the same pass. Every other apply failure, a retry that fails again and a walk fault halt the pass with a typed cause; the authoritative walk has no observe-only exit. |
| **Code** | `crates/ade_node/src/node_lifecycle.rs` |
| **Tests** | `lv8_pair_a_retries_once_and_seals_n_plus_2`, `lv8_pair_b_retries_once_and_seals_n_plus_2`, `lv8_a_retry_that_fails_again_halts_at_the_block`, `lv8_not_checkpoint_behind_halts_without_a_retry`, `lv8_classification_is_exact`, `lv8_a_walk_fault_halts`, `lv8_a_dependency_created_across_a_boundary_crosses_first_then_retries`, `a_crossing_is_classified_from_the_epochs_even_when_the_apply_would_fail` |
| **CI** | `ci/ci_check_coadvance_retry.sh` |
| **Claim** | partial — `e563ad94` (pass) — observed: tests: 8 passed, 0 failed, 0 ignored, 0 not found; CI scripts: 1 passed, 0 failed — limits: (1) No IDSE cluster has declared acceptance obligations for this invariant yet (Doctrine §10); this record re-runs the checks carried from the legacy registry. Further limits: (2) No mutation witness exists yet for this critical invariant (Doctrine §28). |

_critical · local · scope: accumulator and checkpoint co-advance · assumptions: — · depends on: REC-002._ Legacy: DC-EPOCH-46

### LIV — consensus

#### `LIV-003` — _partial_

| Aspect | Location |
|--------|----------|
| **Source** | `docs/planning/consensus.define.consensus.md#batch-1` (approved by wdm33, 2026-11-08); 1 recorded redefinition(s) in the registry |
| **Requirement** | **PartitionConvergence** — If, after a network partition heals, the node keeps receiving from at least one peer that offers the chain the network is converging on (N1), and that chain forks from the node's selection at most k blocks deep, then the node's selected chain eventually equals the chain the reference selection rule prefers among the valid chains delivered to it that SAF-007 allows it to adopt, its current selection included, with ties between equally preferred chains settled as the reference rule settles them; it chooses using only protocol-defined observables. |
| **Code** | `crates/ade_core/src/consensus/fork_choice.rs` |
| **Tests** | `higher_block_no_wins`, `equal_block_no_tiebreaker_decides`, `tiebreaker_loss_keeps_current` |
| **CI** | `ci/ci_check_no_density_in_fork_choice.sh` |
| **Claim** | partial — `e563ad94` (pass) — observed: tests: 3 passed, 0 failed, 0 ignored, 0 not found; CI scripts: 1 passed, 0 failed — limits: (1) No IDSE cluster has declared acceptance obligations for this invariant yet (Doctrine §10); this record re-runs the checks carried from the legacy registry. Further limits: (2) No mutation witness exists yet for this critical invariant (Doctrine §28). |

_critical · distributed · scope: chain selection across peers · assumptions: A2, A4, A5 · depends on: VAL-031, SAF-007 · secondary: FIN._ Legacy: CN-CONS-03

### LIV — node

#### `LIV-004` — _partial_

| Aspect | Location |
|--------|----------|
| **Source** | `docs/planning/node.define.consensus.md#batch-1` (approved by wdm33, 2026-11-09) |
| **Requirement** | **BoundedCleanShutdown** — On a shutdown request, while storage keeps completing writes, the node drains its admission, write and snapshot pipeline to a quiescent point within a fixed, stated deadline, without waiting for peer sessions, finishes writing every snapshot the cadence policy has scheduled up to that point, and exits successfully; restarting on the same stores yields a byte-identical initial state, the blocks after the last snapshot being replayed from the log. If the deadline passes or storage fails, the node exits with a distinct non-zero code without writing a partial snapshot, its durable state being that of the last commit. |
| **Code** | `crates/ade_node/src/node.rs`; `crates/ade_node/src/main.rs` |
| **Tests** | `crates/ade_node/tests/shutdown_resume_identity.rs::shutdown_then_resume_produces_byte_identical_state`, `crates/ade_node/tests/shutdown_resume_identity.rs::shutdown_clean_exits_with_evidence`, `crates/ade_node/tests/shutdown_resume_identity.rs::cold_start_without_genesis_fails_with_generic_startup_code`, `crates/ade_node/tests/authority_fatal_decode.rs::binary_halts_on_authority_fatal_decode_error` |
| **CI** | `ci/ci_check_node_binary_uses_single_bootstrap.sh` |
| **Claim** | unresolved — `e563ad94` (pass) — observed: tests: 4 passed, 0 failed, 0 ignored, 0 not found; CI scripts: 1 passed, 0 failed — limits: (1) No IDSE cluster has declared acceptance obligations for this invariant yet (Doctrine §10); this record re-runs the checks carried from the legacy registry. Further limits: (2) No mutation witness exists yet for this critical invariant (Doctrine §28). (3) Expected to be falsified (node batch S10): the node was observed five times ignoring SIGINT/SIGTERM for 120 s or more and needing SIGKILL. main.rs installs the handlers, but no check at this revision exercises shutdown, so the counterexample is not reproduced here. |

_critical · local · scope: node shutdown · assumptions: A1, A6 · depends on: REC-004 · secondary: REC._ Legacy: DC-NODE-04

#### `LIV-005` — _partial_

| Aspect | Location |
|--------|----------|
| **Source** | `docs/planning/node.define.consensus.md#batch-1` (approved by wdm33, 2026-11-09) |
| **Requirement** | **ServeOutlivesFeed** — Once the node has started serving, the end of its upstream feed does not by itself stop the serve task; serving stops only on node shutdown, a fatal serve error or cancellation by the lifecycle owner. |
| **Code** | `crates/ade_node/src/node_lifecycle.rs` |
| **Tests** | `serve_task_outlives_feed_end_and_serves_late_fetch`, `serve_task_terminates_on_shutdown_no_hang`, `served_view_projects_durable_chain`, `node_serve_start_failure_is_surfaced_not_silent` |
| **CI** | `ci/ci_check_node_serve_lifetime.sh` |
| **Claim** | partial — `e563ad94` (pass) — observed: tests: 4 passed, 0 failed, 0 ignored, 0 not found; CI scripts: 1 passed, 0 failed — limits: (1) No IDSE cluster has declared acceptance obligations for this invariant yet (Doctrine §10); this record re-runs the checks carried from the legacy registry. |

_standard · local · scope: serve task lifetime · assumptions: — · depends on: VAL-050._ Legacy: DC-NODE-09

#### `LIV-006` — _partial_

| Aspect | Location |
|--------|----------|
| **Source** | `docs/planning/node.define.consensus.md#batch-1` (approved by wdm33, 2026-11-09) |
| **Requirement** | **PostSwitchContinuity** — After adopting a winning branch at tip X, the node admits the winning peer's descendants in parent-link order (X+1, then X+2), never admitting a descendant whose parent it has not validated. When a bridge block is missing, it re-fetches the missing range from the winning peer only, with a deterministic bounded number of retries, and admits it in order only when the fetched range is complete, parent-linked and body-validated; otherwise it holds with a structured missing-bridge reason and its durable chain unchanged. While a missing bridge is held, forging stays refused; only a real linear admission or a proven fork-switch adoption clears the hold. It never stalls silently. |
| **Code** | `crates/ade_node/src/node_lifecycle.rs`; `crates/ade_node/src/fork_switch.rs`; `ci/ci_check_missing_bridge_fail_closed.sh`; `crates/ade_node/src/admission_log` |
| **Tests** | `post_switch_admits_winner_descendant_x_plus_1`, `post_switch_missing_bridge_emits_structured_and_holds_fence`, `missing_bridge_wrong_parent_maps_closed_code`, `late_bridge_clears_hold_on_progress`, `missing_bridge_reason_maps_lca_error_to_closed_discriminant`, `bridge_gap_injection_emits_missing_bridge`, `late_bridge_recovers_on_progress`, `refetched_bridge_admits_in_order`, `refetch_failure_structured`, `short_refetch_keeps_hold`, `lying_refetch_body_rejected`, `missing_bridge_triggers_range_refetch`, `bounded_retry` |
| **CI** | `ci/ci_check_missing_bridge_fail_closed.sh`, `ci/ci_check_missing_bridge_refetch.sh` |
| **Claim** | partial — `e563ad94` (pass) — observed: tests: 13 passed, 0 failed, 0 ignored, 0 not found; CI scripts: 2 passed, 0 failed — limits: (1) No IDSE cluster has declared acceptance obligations for this invariant yet (Doctrine §10); this record re-runs the checks carried from the legacy registry. Further limits: (2) No mutation witness exists yet for this critical invariant (Doctrine §28). |

_critical · distributed · scope: post-fork-switch follow · assumptions: A5 · depends on: SEC-022 · secondary: ORD._ Legacy: DC-NODE-39, DC-NODE-41

#### `LIV-007` — _partial_

| Aspect | Location |
|--------|----------|
| **Source** | `docs/planning/node.define.consensus.md#batch-1` (approved by wdm33, 2026-11-09); 1 recorded redefinition(s) in the registry |
| **Requirement** | **ReconnectResumesFromDelivered** — A reconnected peer session builds its FindIntersect candidates by VAL-055's rule from the durable chain at dial time, putting first the last block delivered downstream since the latest rollback when that block is newer than the durable tip, as a resume hint that SEC-020 still resolves; a rollback resets the hint; and blocks buffered from the old session are delivered before the new session's, the per-peer delivery order being preserved across the reconnect. |
| **Code** | `crates/ade_node/src/node_lifecycle.rs` |
| **Tests** | _(no tests found — gap)_ |
| **CI** | _(no CI enforcement — gap)_ |
| **Claim** | unresolved — no evidence record |

_critical · distributed · scope: peer reconnection · assumptions: A4 · depends on: SAF-007, VAL-055, SEC-020 · secondary: ORD._ Legacy: DC-NODE-43

### LIV — pipeline-production

#### `LIV-008` — _partial_

| Aspect | Location |
|--------|----------|
| **Source** | `docs/planning/pipeline-production.define.consensus.md#batch-1` (approved by wdm33, 2026-11-09) |
| **Requirement** | **KeepAliveSurvivesBackpressure** — No stall of the downstream consumer, of any length or cause (block application, an epoch-boundary fold, a checkpoint capture), stops the wire pump from sending keep-alive on its cadence and consuming the peer's response: the wait for downstream capacity also waits on the cadence timer and on inbound frames, so the cadence is reachable while the pump is parked. |
| **Code** | `crates/ade_runtime/src/admission/wire_pump.rs`; `ci/ci_check_wire_liveness.sh` |
| **Tests** | `ce_wl_1_keep_alive_survives_unbounded_downstream_stall` |
| **CI** | `ci/ci_check_wire_liveness.sh` |
| **Claim** | partial — `e563ad94` (pass) — observed: tests: 1 passed, 0 failed, 0 ignored, 0 not found; CI scripts: 1 passed, 0 failed — limits: (1) No IDSE cluster has declared acceptance obligations for this invariant yet (Doctrine §10); this record re-runs the checks carried from the legacy registry. Further limits: (2) No mutation witness exists yet for this critical invariant (Doctrine §28). |

_critical · local · scope: wire pump under backpressure · assumptions: A5 · depends on: VAL-061._ Legacy: DC-PUMP-05

#### `LIV-009` — _partial_

| Aspect | Location |
|--------|----------|
| **Source** | `docs/planning/pipeline-production.define.consensus.md#batch-1` (approved by wdm33, 2026-11-09) |
| **Requirement** | **ReconnectTransportOnly** — One total policy classifies every pump outcome: a transport loss (end of stream, read or write failure) on an established session reconnects it, re-dialling on a fixed, monotone, capped backoff schedule with no randomness or clock-derived jitter; a protocol or grammar violation, a deferred-frame overflow or a keep-alive violation ends that peer without retry; and a consumer that has gone away ends the pump. A peer that cannot be parsed, or whose first dial fails, is dropped without retry. Reconnecting re-establishes only the wire session and never turns a fail-closed authority outcome (admission, the rollback depth bound, boundary promotion, the forge fence) into a retry. |
| **Code** | `crates/ade_node/src/node_lifecycle.rs`; `ci/ci_check_wire_liveness.sh` |
| **Tests** | `reconnect_policy_is_transport_only`, `first_dial_failure_still_ends_the_feed_no_boot_spin`, `spawn_live_wire_pump_source_with_no_usable_peer_yields_ended_feed`, `reconnect_backoff_is_deterministic_monotone_and_capped` |
| **CI** | `ci/ci_check_wire_liveness.sh` |
| **Claim** | partial — `e563ad94` (pass) — observed: tests: 4 passed, 0 failed, 0 ignored, 0 not found; CI scripts: 1 passed, 0 failed — limits: (1) No IDSE cluster has declared acceptance obligations for this invariant yet (Doctrine §10); this record re-runs the checks carried from the legacy registry. Further limits: (2) No mutation witness exists yet for this critical invariant (Doctrine §28). |

_critical · local · scope: wire pump reconnect · assumptions: — · depends on: SEC-019, LIV-007._ Legacy: DC-PUMP-08, DC-PUMP-09, DC-PUMP-10

### LIV — resources-release

#### `LIV-010` — _partial_

| Aspect | Location |
|--------|----------|
| **Source** | `docs/planning/resources-release.define.consensus.md#batch-1` (approved by wdm33, 2026-11-09) |
| **Requirement** | **AuthoritativeWorkNotStarved** — Block validation, chain selection and persistence never wait on mempool or peer-connection work: they run on execution resources that transaction admission and connection handling cannot occupy or block, so their progress does not depend on ingress or churn load, however high it is within the configured bounds. |
| **Code** | `crates/ade_node/src/admission/runner.rs`; `crates/ade_node/src/mem_measure/rss_sampler.rs`; `crates/ade_node/src/convergence_evidence.rs` |
| **Tests** | _(no tests found — gap)_ |
| **CI** | `ci/ci_check_mem_measure_evidence.sh` |
| **Claim** | partial — `e563ad94` (pass) — observed: tests: 0 passed, 0 failed, 0 ignored, 0 not found; CI scripts: 1 passed, 0 failed — limits: (1) No IDSE cluster has declared acceptance obligations for this invariant yet (Doctrine §10); this record re-runs the checks carried from the legacy registry. |

_standard · local · scope: node scheduling · assumptions: — · depends on: SEC-035 · secondary: STR._ Legacy: CN-MEM-02, OP-MEM-01

#### `LIV-011` — _partial_

| Aspect | Location |
|--------|----------|
| **Source** | `docs/planning/resources-release.define.consensus.md#batch-1` (approved by wdm33, 2026-11-09) |
| **Requirement** | **ForgeWithinSlotDeadline** — On a leader slot, forging, self-acceptance and handing the block to serving complete before the slot's deadline on the venue's slot length; missing the deadline loses the slot (pipeline-production's ForgeKesWindowFromBoundGenesis discards the late result) but violates no other invariant. |
| **Code** | `crates/ade_runtime/src/producer/scheduler.rs`; `crates/ade_runtime/tests/producer_pipeline_slot_deadline.rs` |
| **Tests** | `producer_full_path_under_slot_deadline_on_reference_fixture` |
| **CI** | `ci/ci_check_scheduler_closure.sh` |
| **Claim** | partial — `e563ad94` (pass) — observed: tests: 1 passed, 0 failed, 0 ignored, 0 not found; CI scripts: 1 passed, 0 failed — limits: (1) No IDSE cluster has declared acceptance obligations for this invariant yet (Doctrine §10); this record re-runs the checks carried from the legacy registry. |

_standard · local · scope: forging · assumptions: A2 · depends on: DET-019._ Legacy: OP-OPS-05

---

## VAL — Validity

_Doctrine §6: Rules governing acceptable inputs, objects, states, and transitions._ 67 invariant(s).

### VAL — codec-wire-crypto

#### `VAL-001` — _partial_

| Aspect | Location |
|--------|----------|
| **Source** | `docs/planning/codec-wire-crypto.define.consensus.md#batch-1` (approved by wdm33, 2026-11-08) |
| **Requirement** | **ClosedGrammarDecoding** — Each authoritative wire or ledger type has exactly one decoder per protocol version, and it accepts exactly the closed grammar the reference node accepts for that type and version, including valid non-canonical CBOR; each of Ade's own storage record types (write-ahead log, chain database, snapshots) has exactly one decoder per format version, accepting exactly the closed grammar Ade defines for that version. Bytes outside the applicable grammar are rejected with a deterministic structured error before any authoritative state transition, and no fallback, legacy or lenient parser accepts bytes the decoder rejects, on any code path. Where a wire or ledger type's grammar and meaning are unchanged between two supported protocol versions, both versions' decoders give the same result for the same bytes, and every difference between protocol versions is one the reference node also makes; differences between Ade storage format versions are exactly those Ade's format definitions declare. |
| **Code** | `crates/ade_ledger/src/snapshot` |
| **Tests** | _(no tests found — gap)_ |
| **CI** | `ci/ci_check_snapshot_encoder_closure.sh` |
| **Claim** | partial — `e563ad94` (pass) — observed: tests: 0 passed, 0 failed, 0 ignored, 0 not found; CI scripts: 1 passed, 0 failed — limits: (1) No IDSE cluster has declared acceptance obligations for this invariant yet (Doctrine §10); this record re-runs the checks carried from the legacy registry. Further limits: (2) No mutation witness exists yet for this critical invariant (Doctrine §28). |

_critical · local · scope: codec: every authoritative wire and ledger type, and every Ade storage record type · assumptions: A2 · depends on: DET-003, SEC-002._ Legacy: T-ENC-02, CN-WIRE-03, CN-WIRE-04, CN-WIRE-05

#### `VAL-002` — _partial_

| Aspect | Location |
|--------|----------|
| **Source** | `docs/planning/codec-wire-crypto.define.consensus.md#batch-1` (approved by wdm33, 2026-11-08) |
| **Requirement** | **Tag24EnvelopeAuthority** — CBOR-in-CBOR (tag 24) envelopes in block-fetch and chain-sync messages are built and removed only by one shared BLUE authority and match the reference node's wire shape: a served block-fetch block is tag 24 over [era, block], a served chain-sync header is [era, tag 24 over header]; a received block-fetch block is unwrapped exactly once, at the receive boundary, before block decode; a malformed or missing envelope, a wrong inner length or trailing bytes are rejected with a structured error, and the inner bytes are kept verbatim. |
| **Code** | `crates/ade_codec/src/cbor/tag24.rs`; `crates/ade_codec/src/cbor/mod.rs`; `crates/ade_network/src/codec/block_fetch.rs`; `crates/ade_network/src/block_fetch/server.rs`; `crates/ade_node/src/admission/runner.rs`; `crates/ade_core_interop/src/follow.rs`; `crates/ade_runtime/src/admission/wire_pump.rs`; `crates/ade_node/src/node_sync.rs` |
| **Tests** | `wrap_then_unwrap_is_identity_across_length_classes`, `wrap_emits_canonical_tag24_marker_and_length`, `unwrap_returns_zero_copy_borrow_of_input`, `unwrap_rejects_missing_tag24_marker`, `unwrap_rejects_non_byte_string_payload`, `unwrap_rejects_truncated_inner`, `unwrap_rejects_trailing_bytes`, `inner_bytes_are_verbatim_not_reencoded`, `unwrap_rejects_huge_declared_length_without_panic`, `read_bytes_rejects_overflowing_declared_length`, `read_text_rejects_overflowing_declared_length`, `producer_block_fetch_serve_block_payload_is_tag24_wrapped_and_decodes`, `producer_block_fetch_serve_block_bytes_equal_accepted_block_as_bytes`, `block_body_is_wrapped_cbor_item`, `captured_block_era_index_matches_ade_storage_scheme`, `chain_sync_wire_era_index_is_consensus_index`, `compose_decompose_rollforward_header_round_trips`, `decompose_rollforward_header_rejects_non_tag24_inner`, `conway_rollforward_header_served_shape_matches_oracle`, `producer_chain_sync_serve_roll_forward_header_equals_accepted_block_header_bytes`, `session_transcript_served_block_bytes_equal_admitted_accepted_block_bytes`, `session_transcript_announced_header_matches_served_body_recipe`, `cross_impl_server_pipeline_request_range_returns_decodable_bytes`, `cross_impl_server_pipeline_request_range_byte_identical_to_self_accept_input`, `feed_unwrap_decodes_genesis_successor_block_zero`, `block_fetch_unwraps_tag24_emitting_bare_block`, `block_fetch_fails_closed_on_non_tag24_payload`, `served_view_projects_durable_chain` |
| **CI** | `ci/ci_check_tag24_wire_authority.sh`, `ci/ci_check_feed_tag24_unwrap.sh` |
| **Claim** | partial — `e563ad94` (pass) — observed: tests: 28 passed, 0 failed, 0 ignored, 0 not found; CI scripts: 2 passed, 0 failed — limits: (1) No IDSE cluster has declared acceptance obligations for this invariant yet (Doctrine §10); this record re-runs the checks carried from the legacy registry. |

_standard · local · scope: block-fetch and chain-sync codecs, serve and feed paths · assumptions: A2 · depends on: VAL-001, DET-002 · secondary: STR._ Legacy: CN-WIRE-08, CN-WIRE-12

#### `VAL-003` — _partial_

| Aspect | Location |
|--------|----------|
| **Source** | `docs/planning/codec-wire-crypto.define.consensus.md#batch-1` (approved by wdm33, 2026-11-08) |
| **Requirement** | **PrevHashClosedSum** — A Conway header's previous-hash field is the closed sum Genesis or Block(32-byte hash), encoded as CBOR null or as the 32-byte hash through one shared codec; no placeholder hash stands for the genesis predecessor, a block numbered 0 carries Genesis, and every other block carries Block. |
| **Code** | `crates/ade_types/src/shelley/block.rs`; `crates/ade_codec/src/shelley/block.rs`; `crates/ade_ledger/src/block_validity/header_position.rs`; `crates/ade_ledger/src/block_validity/header_input.rs`; `crates/ade_ledger/src/block_validity/verdict.rs`; `crates/ade_ledger/src/producer/forge.rs`; `crates/ade_ledger/src/block_validity/unsigned_header_pre_image.rs`; `crates/ade_runtime/src/producer/chain_evolution.rs` |
| **Tests** | `prevhash_genesis_round_trips_as_null`, `prevhash_block_round_trips_as_hash32`, `prevhash_codec_is_position_blind`, `genesis_successor_header_round_trips_with_null_prev`, `block_header_prev_hash_byte_identical_after_migration`, `header_position_zero_requires_genesis_ok`, `header_position_zero_with_block_is_rejected`, `header_position_nonzero_requires_block_ok`, `header_position_nonzero_with_genesis_is_rejected`, `decode_block_rejects_block_prev_at_block_number_zero`, `decode_block_rejects_genesis_prev_at_nonzero_block_number`, `corpus_blocks_pass_header_position_rule`, `forge_block_number_zero_emits_genesis_prev_hash`, `forge_nonzero_block_emits_block_prev_byte_identical`, `forge_block_zero_self_consistent_through_decode_block`, `pre_image_block_zero_emits_genesis_prev`, `pre_image_nonzero_block_prev_byte_identical`, `forged_block_zero_kes_preimage_equals_decoded_header_body_bytes`, `chain_evolution_prev_hash_genesis_at_cold_start`, `chain_evolution_prev_hash_block_with_tip` |
| **CI** | `ci/ci_check_prevhash_single_wire_authority.sh` |
| **Claim** | partial — `e563ad94` (pass) — observed: tests: 20 passed, 0 failed, 0 ignored, 0 not found; CI scripts: 1 passed, 0 failed — limits: (1) No IDSE cluster has declared acceptance obligations for this invariant yet (Doctrine §10); this record re-runs the checks carried from the legacy registry. |

_standard · local · scope: header codec, forging and header validation · assumptions: A2 · depends on: VAL-001, DET-003 · secondary: STR._ Legacy: CN-WIRE-09

#### `VAL-004` — _partial_

| Aspect | Location |
|--------|----------|
| **Source** | `docs/planning/codec-wire-crypto.define.consensus.md#batch-1` (approved by wdm33, 2026-11-08) |
| **Requirement** | **N2NHandshakeWireCompatibility** — The node-to-node handshake encodes version data, MsgAcceptVersion and query replies in the reference node's closed grammar for the negotiated version, and the initiator and responder use one shared per-version encoder: versions 11 to 15 carry the four-field version data, and versions 16 and later add the Peras-support field. |
| **Code** | `crates/ade_network/src/handshake/version_table.rs`; `crates/ade_network/src/session/handshake_driver.rs`; `crates/ade_node/src/admission/bootstrap.rs` |
| **Tests** | `responder_v15_accept_matches_real_cardano_node_preprod_fixture`, `responder_v15_accept_matches_failing_c1_peer_fixture`, `responder_v15_versiondata_is_a_four_element_array_not_a_bare_int`, `served_view_projects_durable_chain` |
| **CI** | `ci/ci_check_n2n_handshake_versiondata_authority.sh` |
| **Claim** | partial — `e563ad94` (pass) — observed: tests: 4 passed, 0 failed, 0 ignored, 0 not found; CI scripts: 1 passed, 0 failed — limits: (1) No IDSE cluster has declared acceptance obligations for this invariant yet (Doctrine §10); this record re-runs the checks carried from the legacy registry. |

_standard · local · scope: N2N handshake codec · assumptions: A2 · depends on: VAL-001._ Legacy: CN-WIRE-10

#### `VAL-005` — _partial_

| Aspect | Location |
|--------|----------|
| **Source** | `docs/planning/codec-wire-crypto.define.consensus.md#batch-1` (approved by wdm33, 2026-11-08) |
| **Requirement** | **FindIntersectCompatibility** — The chain-sync server decodes a MsgFindIntersect point list in both the definite-length and the indefinite-length array forms the reference node sends, while every other array in chain-sync messages stays definite-length only; it answers IntersectFound with the first listed point that lies on its served chain, Origin always counting as on it, and IntersectNotFound when none does. |
| **Code** | `crates/ade_network/src/codec/primitives.rs`; `crates/ade_network/src/codec/chain_sync.rs`; `crates/ade_network/src/chain_sync/server.rs` |
| **Tests** | `real_cardano_node_findintersect_indefinite_points_list_decodes`, `real_cardano_node_findintersect_yields_intersect_found_origin`, `producer_chain_sync_serve_find_intersect_origin_yields_intersect_found_origin`, `producer_chain_sync_serve_find_intersect_known_point_yields_intersect_found`, `producer_chain_sync_serve_find_intersect_unknown_point_yields_intersect_not_found`, `decode_array_header_still_rejects_indefinite`, `two_form_accepts_definite_and_indefinite` |
| **CI** | `ci/ci_check_chainsync_findintersect_compat.sh` |
| **Claim** | partial — `e563ad94` (pass) — observed: tests: 7 passed, 0 failed, 0 ignored, 0 not found; CI scripts: 1 passed, 0 failed — limits: (1) No IDSE cluster has declared acceptance obligations for this invariant yet (Doctrine §10); this record re-runs the checks carried from the legacy registry. |

_standard · local · scope: chain-sync server codec and intersect reply · assumptions: A2 · depends on: VAL-001._ Legacy: CN-WIRE-11

#### `VAL-006` — _partial_

| Aspect | Location |
|--------|----------|
| **Source** | `docs/planning/codec-wire-crypto.define.consensus.md#batch-1` (approved by wdm33, 2026-11-08); 1 recorded redefinition(s) in the registry |
| **Requirement** | **SigningMatchesReference** — Ade's Sum6 KES and Praos VRF signing produce output byte-identical to the Haskell cardano-base reference for every input: verification-key derivation, key generation from a seed, evolution across all 64 periods, KES signatures and VRF proofs; every KES signature and VRF proof Ade emits verifies under Ade's own verifier (SEC-003) with the matching verification key; the 608-byte expanded KES signing key serializes and deserializes byte-identically to the reference for every period from 0 to 63, with the current period inferred uniquely from the key's tree shape. The algorithm is Ade's own code: no production path builds a key of an upstream library through unsafe layout assumptions, transmute, vendored private access or fork-only constructors. |
| **Code** | `crates/ade_runtime/src/producer/signing.rs`; `crates/ade_crypto/src/kes.rs`; `crates/ade_crypto/src/kes_sum`; `crates/ade_crypto/src/kes_sum/cardano_cli_corpus.rs`; `crates/ade_crypto/src/kes_sum/tests.rs`; `crates/ade_crypto/src/kes_sum/period.rs`; `crates/ade_crypto/src/kes_sum/sum.rs`; `crates/ade_crypto/src/kes_sum/single.rs`; `crates/ade_crypto/src/kes_sum/errors.rs` |
| **Tests** | `vrf_prove_matches_reference_vectors`, `vrf_prove_then_verify_round_trip`, `kes_sign_matches_reference_vectors`, `kes_sign_then_verify_round_trip`, `kes_signature_from_bytes_round_trips`, `verify_kes_signature_agrees_with_existing_verify_kes`, `cardano_cli_corpus_sign_then_upstream_verifies`, `sum0_kes_signs_and_verifies_at_period_0`, `sum0_kes_rejects_period_1`, `sum0_kes_update_expires_after_period_0`, `sum0_kes_verify_rejects_wrong_message`, `sum1_kes_signs_at_period_0_and_period_1`, `sum6_kes_total_periods_is_64`, `sum6_kes_sizes_match_recurrence`, `sum6_kes_chain_advances_through_all_64_periods`, `sum6_kes_update_after_period_63_expires`, `sum6_kes_sign_rejects_period_64`, `sum6_kes_verify_rejects_wrong_period_signature`, `sum6_kes_seed_expansion_diverges_from_cardano_crypto_rust_1_0_8`, `sum6_kes_vk_diverges_from_cardano_crypto_rust_for_same_seed`, `cardano_cli_corpus_skey_deserializes_and_vk_matches_ground_truth`, `sum0_signing_key_debug_is_redacted`, `sum_signing_key_debug_is_redacted`, `zeroizing_seed_drop_overwrites_bytes`, `sum6_raw_serialize_signing_key_kes_size_is_608`, `sum6_raw_serialize_signature_kes_size_is_448`, `sum6_skey_round_trip_at_every_period_0_to_63`, `sum6_signature_round_trip_at_every_period`, `period_from_zeroed_sum6_tree_shape_agrees_with_update_kes_chain`, `period_from_zeroed_sum6_tree_shape_rejects_leaf_all_zero`, `raw_deserialize_signing_key_kes_rejects_wrong_payload_size`, `raw_deserialize_signing_key_kes_rejects_leaf_all_zero`, `raw_deserialize_signing_key_kes_rejects_inconsistent_vk_left_at_level_6`, `raw_deserialize_signing_key_kes_rejects_inconsistent_vk_right_at_level_6`, `raw_deserialize_signature_kes_rejects_wrong_payload_size`, `cardano_cli_corpus_skey_round_trips_through_ade_serde`, `cardano_cli_corpus_skey_period_inference_is_zero`, `cardano_cli_corpus_negative_flip_one_byte_in_vk_left_fail_closed`, `cardano_cli_corpus_constants_have_expected_sizes` |
| **CI** | `ci/ci_check_private_key_custody.sh`, `ci/ci_check_kes_sum_compatibility.sh` |
| **Claim** | partial — `e563ad94` (pass) — observed: tests: 39 passed, 0 failed, 0 ignored, 0 not found; CI scripts: 2 passed, 0 failed — limits: (1) No IDSE cluster has declared acceptance obligations for this invariant yet (Doctrine §10); this record re-runs the checks carried from the legacy registry. Further limits: (2) No mutation witness exists yet for this critical invariant (Doctrine §28). |

_critical · local · scope: ade_crypto Sum6 KES and VRF, RED producer signing · assumptions: A2, A3 · depends on: SEC-003 · secondary: DET._ Legacy: DC-CRYPTO-03, DC-CRYPTO-04, DC-CRYPTO-08, DC-CRYPTO-09

### VAL — ledger

#### `VAL-007` — _partial_

| Aspect | Location |
|--------|----------|
| **Source** | `docs/planning/ledger.define.consensus.md#batch-1` (approved by wdm33, 2026-11-08) |
| **Requirement** | **BlockValidityMatchesReference** — For every Conway block, Ade's valid or invalid verdict equals the reference node's verdict for the same prior ledger and consensus state and protocol version, and wherever the reference exposes a rejection reason, Ade's reason falls in the same class. |
| **Code** | `crates/ade_ledger/src/byron.rs`; `crates/ade_ledger/src/rules.rs`; `crates/ade_ledger/src/plutus_eval.rs`; `crates/ade_ledger/src/block_validity`; `crates/ade_testkit/src/validity/replay.rs`; `crates/ade_testkit/src/validity/adversarial.rs` |
| **Tests** | `check_duplicate_inputs_catches_dupes`, `resolve_inputs_missing_input`, `missing_witnesses_rejected`, `all_eras_replay_summary`, `all_plutus_boundaries_aggregate_zero_rejections`, `plutus_era_contiguous_smoke`, `under_declared_ex_units_must_reject`, `failing_validator_must_reject`, `extraneous_redeemer_must_reject`, `corpus_block_count_is_14`, `all_corpus_blocks_valid`, `verdict_stream_replays_identically`, `no_mutation_is_ever_valid`, `each_mutation_maps_to_expected_class`, `adversarial_replays_identically` |
| **CI** | `ci/ci_check_differential_divergence.sh`, `ci/ci_check_plutus_budget_cap.sh`, `ci/ci_check_plutus_oracle_no_false_accept.sh`, `ci/ci_check_consensus_closed_enums.sh` |
| **Claim** | partial — `e563ad94` (pass) — observed: tests: 11 passed, 0 failed, 0 ignored, 4 not found; CI scripts: 4 passed, 0 failed — limits: (1) No IDSE cluster has declared acceptance obligations for this invariant yet (Doctrine §10); this record re-runs the checks carried from the legacy registry. Further limits: (2) No mutation witness exists yet for this critical invariant (Doctrine §28). (3) 4 named test(s) were not found in the test output at this revision: all_plutus_boundaries_aggregate_zero_rejections, under_declared_ex_units_must_reject, failing_validator_must_reject, extraneous_redeemer_must_reject |

_critical · local · scope: block validation · assumptions: A2 · depends on: VAL-009, EVO-003._ Legacy: DC-LEDGER-03, CN-LEDGER-03, DC-VAL-04

#### `VAL-008` — _partial_

| Aspect | Location |
|--------|----------|
| **Source** | `docs/planning/ledger.define.consensus.md#batch-1` (approved by wdm33, 2026-11-08) |
| **Requirement** | **TxValidityMatchesReference** — For every Conway transaction, Ade's valid or invalid verdict equals the reference node's verdict for the same ledger state and protocol version, and wherever the reference exposes a rejection reason, Ade's reason falls in the same class. |
| **Code** | `crates/ade_ledger/src/byron.rs`; `crates/ade_ledger/src/rules.rs`; `crates/ade_ledger/src/plutus_eval.rs`; `crates/ade_testkit/src/tx_validity`; `crates/ade_ledger/src/tx_validity/phase1.rs`; `crates/ade_testkit/src/tx_validity/adversarial.rs`; `crates/ade_ledger/tests/tx_validity_adversarial_corpus.rs` |
| **Tests** | `check_duplicate_inputs_catches_dupes`, `resolve_inputs_missing_input`, `missing_witnesses_rejected`, `all_eras_replay_summary`, `all_plutus_boundaries_aggregate_zero_rejections`, `plutus_era_contiguous_smoke`, `under_declared_ex_units_must_reject`, `failing_validator_must_reject`, `extraneous_redeemer_must_reject`, `all_corpus_txs_valid`, `corpus_tx_count_nonzero`, `no_mutation_is_ever_valid`, `each_mutation_maps_to_expected_class`, `adversarial_replays_identically`, `adversarial_imbalanced_via_deposit`, `adversarial_imbalanced_via_withdrawal`, `adversarial_unknown_cert_tag_rejects_as_decode`, `adversarial_removed_tag_rejects_as_era_invalid`, `adversarial_truncated_withdrawals_rejects_as_decode`, `adversarial_state_dependent_unaccountable_rejects`, `adversarial_no_false_accept`, `adversarial_verdicts_replay_byte_identical`, `real_cert_withdrawal_txs_are_valid_at_track_utxo_true`, `real_positive_verdict_stream_replays_byte_identical`, `synthetic_balanced_cert_withdrawal_txs_are_valid`, `synthetic_positive_verdict_stream_replays_byte_identical` |
| **CI** | `ci/ci_check_differential_divergence.sh`, `ci/ci_check_plutus_budget_cap.sh`, `ci/ci_check_plutus_oracle_no_false_accept.sh`, `ci/ci_check_consensus_closed_enums.sh` |
| **Claim** | unresolved — `e563ad94` (pass) — observed: tests: 22 passed, 0 failed, 0 ignored, 4 not found; CI scripts: 4 passed, 0 failed — limits: (1) No IDSE cluster has declared acceptance obligations for this invariant yet (Doctrine §10); this record re-runs the checks carried from the legacy registry. Further limits: (2) No mutation witness exists yet for this critical invariant (Doctrine §28). (3) 4 named test(s) were not found in the test output at this revision: all_plutus_boundaries_aggregate_zero_rejections, under_declared_ex_units_must_reject, failing_validator_must_reject, extraneous_redeemer_must_reject (4) Known disagreement: no Conway minimum-UTxO check exists (see VAL-014), so a transaction the reference rejects for an undersized output is not rejected by Ade. |

_critical · local · scope: transaction validation (block path and standalone) · assumptions: A2 · depends on: VAL-009._ Legacy: DC-LEDGER-03, CN-LEDGER-03, DC-TXV-03

#### `VAL-009` — _partial_

| Aspect | Location |
|--------|----------|
| **Source** | `docs/planning/ledger.define.consensus.md#batch-1` (approved by wdm33, 2026-11-08) |
| **Requirement** | **CompleteValidation** — A block is valid only if both the consensus header authority and the ledger body authority accept it. A transaction in a block is accepted only if phase 1 accepts it (native scripts included) and the outcome of evaluating its Plutus scripts agrees with the phase-2 status the block declares for it: evaluation succeeds for a transaction marked valid and fails for one in the block's invalid-transactions set, which then takes the collateral path. A standalone transaction is admitted only if phase 1 accepts it and its Plutus scripts evaluate successfully. No path produces a valid verdict while skipping any of these checks. |
| **Code** | `crates/ade_ledger/src/block_validity`; `crates/ade_ledger/src/block_validity/transition.rs`; `crates/ade_ledger/src/block_validity/header_input.rs`; `crates/ade_ledger/src/tx_validity/transition.rs`; `crates/ade_ledger/src/tx_validity/phase1.rs`; `crates/ade_ledger/src/receive/admitted.rs`; `crates/ade_ledger/src/receive/reducer.rs`; `crates/ade_ledger/src/receive/chain_write.rs` |
| **Tests** | `valid_block_evolves_both_states`, `header_before_body_fail_fast`, `valid_tx_is_valid_and_applies`, `phase1_failure_short_circuits_phase2`, `admit_via_block_validity_accepts_corpus_block`, `admit_via_block_validity_rejects_corrupted_body`, `receive_apply_block_delivered_with_matching_header_admits`, `receive_apply_block_delivered_validity_invalid_rejects`, `receive_apply_rollback_returns_out_of_scope`, `receive_apply_replay_byte_identical_over_corpus` |
| **CI** | `ci/ci_check_consensus_closed_enums.sh`, `ci/ci_check_admitted_block_closure.sh`, `ci/ci_check_receive_reducer_closure.sh` |
| **Claim** | partial — `e563ad94` (pass) — observed: tests: 10 passed, 0 failed, 0 ignored, 0 not found; CI scripts: 3 passed, 0 failed — limits: (1) No IDSE cluster has declared acceptance obligations for this invariant yet (Doctrine §10); this record re-runs the checks carried from the legacy registry. Further limits: (2) No mutation witness exists yet for this critical invariant (Doctrine §28). |

_critical · local · scope: block and transaction validation · assumptions: A2 · depends on: —._ Legacy: DC-VAL-02, DC-TXV-02

#### `VAL-010` — _partial_

| Aspect | Location |
|--------|----------|
| **Source** | `docs/planning/ledger.define.consensus.md#batch-1` (approved by wdm33, 2026-11-08) |
| **Requirement** | **ChecksNeverSkip** — Every size, shape and cryptographic-input check on the validation path rejects when its input has the wrong size or shape and never skips silently; no check is defined but left unwired, and no guard compares a value with itself. |
| **Code** | `crates/ade_ledger/src/block_validity/verdict.rs`; `crates/ade_core/src/consensus/kes_check.rs`; `crates/ade_testkit/src/validity/adversarial.rs`; `crates/ade_ledger/src/tx_validity/witness.rs`; `crates/ade_ledger/src/tx_validity/required_signers.rs`; `crates/ade_ledger/src/conway.rs`; `crates/ade_testkit/src/tx_validity/adversarial.rs` |
| **Tests** | `expect_size_rejects_wrong_length`, `praos_malformed_kes_sig_rejected`, `no_mutation_is_ever_valid`, `each_mutation_maps_to_expected_class`, `wrong_size_signature_rejected`, `wrong_size_vkey_rejected`, `unresolvable_input_is_fail_fast`, `unresolvable_collateral_input_is_fail_fast`, `conway_conservation_full`, `conservation_early_out_removed`, `adversarial_imbalanced_via_deposit`, `adversarial_imbalanced_via_withdrawal`, `adversarial_unknown_cert_tag_rejects_as_decode`, `adversarial_removed_tag_rejects_as_era_invalid`, `adversarial_truncated_withdrawals_rejects_as_decode`, `adversarial_state_dependent_unaccountable_rejects`, `adversarial_no_false_accept`, `adversarial_verdicts_replay_byte_identical`, `trailing_bytes_after_cert_array_rejected`, `huge_array_count_rejects_without_overallocating` |
| **CI** | `ci/ci_check_consensus_closed_enums.sh` |
| **Claim** | partial — `e563ad94` (pass) — observed: tests: 20 passed, 0 failed, 0 ignored, 0 not found; CI scripts: 1 passed, 0 failed — limits: (1) No IDSE cluster has declared acceptance obligations for this invariant yet (Doctrine §10); this record re-runs the checks carried from the legacy registry. Further limits: (2) No mutation witness exists yet for this critical invariant (Doctrine §28). (3) The forge engine's operational-certificate period check passes the certificate's own period as the expected value, comparing a value with itself (pipeline-production S6). The forge is outside the validation path this invariant scopes, but the guard is recorded here. |

_critical · local · scope: BLUE block and transaction validation · assumptions: — · depends on: VAL-009._ Legacy: DC-VAL-06

#### `VAL-011` — _partial_

| Aspect | Location |
|--------|----------|
| **Source** | `docs/planning/ledger.define.consensus.md#batch-1` (approved by wdm33, 2026-11-08) |
| **Requirement** | **CertificateTransitionsClosed** — Every Conway certificate decodes through the Conway certificate grammar with all its payload fields and resolves to one closed disposition: either a structured rejection, or the complete list of its effects, each applied exactly once by the owner of the state it changes, delegation and pool state or governance state (vote delegation, committee hot keys, DRep expiry). A composite certificate has each half applied once by its own owner; no effect is dropped, flattened or applied twice, and DRep expiry is computed only from explicit governance context. |
| **Code** | `crates/ade_types/src/conway/cert.rs`; `crates/ade_types/src/shelley/cert.rs`; `crates/ade_codec/src/conway/cert.rs`; `crates/ade_codec/src/shelley/cert.rs`; `crates/ade_ledger/src/delegation.rs`; `crates/ade_ledger/src/gov_cert.rs`; `crates/ade_ledger/src/state.rs`; `crates/ade_ledger/src/pparams.rs`; `crates/ade_ledger/src/error.rs`; `crates/ade_ledger/src/rules.rs`; `crates/ade_ledger/src/fingerprint.rs` |
| **Tests** | `each_tag_retains_owner_payloads`, `drep_grammar_total`, `conway_cert_action_total`, `apply_outcome_agrees_with_action`, `removed_tag_rejects_as_era_invalid`, `drep_registration_is_owner_tagged_not_applied`, `era_dispatch_conway_accumulates_via_conway_path`, `era_dispatch_shelley_accumulates_via_shelley_path`, `conway_decode_error_is_fail_closed`, `conway_unknown_tag_is_fail_closed`, `conway_removed_tag_is_fail_closed`, `conway_apply_error_is_fail_closed`, `conway_governance_cert_routed_out_of_scope`, `positive_synthetic_cert_state_accumulates`, `cert_state_replay_byte_identical`, `adversarial_no_false_accept`, `gov_apply_total_over_18_tags`, `composite_gov_half_applied_once_certstate_untouched_by_b5`, `drep_expiry_uses_epoch_plus_activity`, `env_free_gov_certs_need_no_env`, `drep_register_missing_env_is_fail_fast`, `drep_expiry_overflow_is_fail_closed`, `gov_apply_is_deterministic`, `gov_cert_env_present_ok`, `gov_cert_env_missing_drep_activity_is_fail_fast`, `gov_accumulation_applies_drep_registration_into_gov_state`, `gov_apply_error_halts_accumulation`, `positive_synthetic_gov_state_accumulates`, `gov_state_accumulation_replays_byte_identical`, `adversarial_drep_register_update_missing_env_rejected`, `adversarial_drep_expiry_overflow_rejected`, `adversarial_decode_layer_rejects_guard_gov_path`, `adversarial_double_resign_is_deterministic` |
| **CI** | `ci/ci_check_forbidden_patterns.sh`, `ci/ci_check_gov_cert_accumulation_closed.sh` |
| **Claim** | partial — `e563ad94` (pass) — observed: tests: 33 passed, 0 failed, 0 ignored, 0 not found; CI scripts: 2 passed, 0 failed — limits: (1) No IDSE cluster has declared acceptance obligations for this invariant yet (Doctrine §10); this record re-runs the checks carried from the legacy registry. Further limits: (2) No mutation witness exists yet for this critical invariant (Doctrine §28). |

_critical · local · scope: ledger certificate state and Conway governance state · assumptions: A2 · depends on: VAL-001, SAF-001._ Legacy: CN-LEDGER-10, DC-LEDGER-08, DC-LEDGER-09

#### `VAL-012` — _partial_

| Aspect | Location |
|--------|----------|
| **Source** | `docs/planning/ledger.define.consensus.md#batch-1` (approved by wdm33, 2026-11-08) |
| **Requirement** | **NetworkConstantsFromSchedule** — No network-specific constant enters an authoritative computation except through the network's declared era schedule and genesis data; every epoch number on an authoritative path is derived from the caller's era schedule, and any mainnet-only constant is reachable only through an explicitly named mainnet schedule. |
| **Code** | `crates/ade_ledger/src/state.rs`; `crates/ade_ledger/src/rules.rs` |
| **Tests** | `mainnet_epoch_at_shelley_start`, `mainnet_epoch_mid_epoch`, `mainnet_epoch_allegra`, `mainnet_epoch_pre_shelley` |
| **CI** | `ci/ci_check_venue_constant_containment.sh` |
| **Claim** | partial — `e563ad94` (pass) — observed: tests: 4 passed, 0 failed, 0 ignored, 0 not found; CI scripts: 1 passed, 0 failed — limits: (1) No IDSE cluster has declared acceptance obligations for this invariant yet (Doctrine §10); this record re-runs the checks carried from the legacy registry. Further limits: (2) No mutation witness exists yet for this critical invariant (Doctrine §28). |

_critical · local · scope: ledger and consensus epoch and slot arithmetic · assumptions: A2 · depends on: EVO-001 · secondary: DET._ Legacy: DC-LEDGER-13

#### `VAL-013` — _partial_

| Aspect | Location |
|--------|----------|
| **Source** | `docs/planning/ledger.define.consensus.md#batch-1` (approved by wdm33, 2026-11-08) |
| **Requirement** | **OutputQuantityFullDomain** — Every output asset quantity holds Cardano's full unsigned 64-bit domain, a negative output quantity cannot be represented, and output-value arithmetic is checked: overflow and underflow are structured errors, never a wrap or a saturation. |
| **Code** | `crates/ade_types/src/mary/value.rs`; `crates/ade_ledger/src/value.rs`; `crates/ade_ledger/src/error.rs`; `crates/ade_ledger/src/phase.rs`; `crates/ade_ledger/src/mary.rs`; `crates/ade_ledger/src/snapshot/utxo_state.rs`; `crates/ade_ledger/src/snapshot/error.rs`; `crates/ade_ledger/src/fingerprint.rs`; `ci/ci_check_value_quantity_domain.sh` |
| **Tests** | `multi_asset_word64_add_sub_round_trips_above_i64_max`, `multi_asset_sub_underflow_returns_asset_underflow`, `multi_asset_add_overflow_returns_error`, `negative_output_quantity_is_unrepresentable`, `utxo_state_word64_multi_asset_quantity_round_trips`, `utxo_state_negative_output_quantity_is_rejected`, `representable_quantity_encodes_byte_identical_golden`, `stage2_mempack_word64_output_survives_snapshot_recovery` |
| **CI** | `ci/ci_check_value_quantity_domain.sh` |
| **Claim** | partial — `e563ad94` (pass) — observed: tests: 8 passed, 0 failed, 0 ignored, 0 not found; CI scripts: 1 passed, 0 failed — limits: (1) No IDSE cluster has declared acceptance obligations for this invariant yet (Doctrine §10); this record re-runs the checks carried from the legacy registry. Further limits: (2) No mutation witness exists yet for this critical invariant (Doctrine §28). |

_critical · local · scope: ledger values and UTxO outputs · assumptions: A2 · depends on: — · secondary: CON._ Legacy: DC-LEDGER-VALUE-01

#### `VAL-014` — _partial_

| Aspect | Location |
|--------|----------|
| **Source** | `docs/planning/ledger.define.consensus.md#batch-1` (approved by wdm33, 2026-11-08) |
| **Requirement** | **ProtocolParametersEraFaithful** — Imported protocol parameters keep the meaning their era gives them and are never remapped: Conway's coins-per-UTxO-byte is held as a per-byte coefficient, in a representation distinct from an absolute minimum, and the Conway minimum-UTxO check requires each output's coin to be at least the coefficient times (160 plus the byte length of the output's CBOR serialization as it appears in the transaction), as the reference node computes it. |
| **Code** | `crates/ade_ledger/src/pparams.rs`; `crates/ade_ledger/src/shelley.rs`; `crates/ade_ledger/src/error.rs`; `crates/ade_ledger/src/phase.rs`; `crates/ade_ledger/src/ledgerdb_state.rs`; `crates/ade_ledger/src/snapshot/gov_state.rs`; `ci/ci_check_native_nonutxo_decode.sh` |
| **Tests** | `network_id_derived_from_manifest_magic`, `reward_nibble_disagreement_is_diagnostic_not_terminal`, `conway_pparams_decode_yields_per_byte_min_utxo_rule`, `mary_min_utxo_per_byte_rule_is_terminal_not_permissive`, `mary_min_utxo_legacy_absolute_min_unchanged`, `commitment_binds_every_field`, `decode_native_nonutxo_real_snapshot` |
| **CI** | `ci/ci_check_native_nonutxo_decode.sh` (red at e563ad94) |
| **Claim** | unresolved — `e563ad94` (fail) — observed: tests: 7 passed, 0 failed, 0 ignored, 0 not found; CI scripts: 0 passed, 1 failed; failing CI: ci/ci_check_native_nonutxo_decode.sh — limits: (1) No IDSE cluster has declared acceptance obligations for this invariant yet (Doctrine §10); this record re-runs the checks carried from the legacy registry. Further limits: (2) No mutation witness exists yet for this critical invariant (Doctrine §28). (3) The Conway validation path has no minimum-UTxO check: the per-byte rule is only recognized (and refused) on the Shelley and Mary paths, so a Conway output below the reference minimum is not rejected. (4) ci/ci_check_native_nonutxo_decode.sh fails at this revision and already failed before the IDSE migration (a3cef7f7): "FAIL (ci_check_native_nonutxo_decode): network_id must be DERIVED from the manifest magic, not the literal placeholder (network_id: 0)". Diagnosed as a stale or over-broad structural gate, not a demonstrated violation; the gate needs repair before it can serve as evidence. |

_critical · local · scope: protocol parameters, snapshot import, Conway minimum-UTxO validation · assumptions: A2 · depends on: VAL-001._ Legacy: DC-LEDGER-PARAMS-01

#### `VAL-015` — _declared_

| Aspect | Location |
|--------|----------|
| **Source** | `docs/planning/ledger.define.consensus.md#batch-1` (approved by wdm33, 2026-11-08) |
| **Requirement** | **ScriptContextCanonical** — The script context a Plutus script receives is derived canonically and completely from the transaction, the resolved ledger state and the network-wide constants (system start and era history); nothing else enters it. |
| **Code** | _(no enforcement found — gap)_ |
| **Tests** | _(no tests found — gap)_ |
| **CI** | _(no CI enforcement — gap)_ |
| **Claim** | unresolved — no evidence record |

_critical · local · scope: script-context construction · assumptions: A2 · depends on: VAL-012, STR-001 · secondary: DET._ Legacy: DC-LEDGER-06, CN-PLUTUS-03

#### `VAL-016` — _partial_

| Aspect | Location |
|--------|----------|
| **Source** | `docs/planning/ledger.define.consensus.md#batch-1` (approved by wdm33, 2026-11-08) |
| **Requirement** | **SinglePhaseTwoEffect** — A transaction's UTxO and accounting effect is derived in exactly one place, from the block and its invalid-transactions set: a phase-2-valid transaction spends its ordinary inputs and produces its ordinary outputs at indices 0 to n-1; a phase-2-invalid transaction spends only its collateral inputs, produces only its collateral return at index n, where n is its number of ordinary outputs, adds its consumed collateral to the fee pot, and has no other effect, its certificates, withdrawals, votes and proposals being discarded. Consumers receive only the derived spends, outputs and fee contribution, never the phase-2 status, so they cannot branch on it. |
| **Code** | `crates/ade_ledger/src/rules.rs`; `crates/ade_ledger/src/store_semantics.rs`; `ci/ci_check_phase2_invalid_utxo_effect.sh`; `crates/ade_ledger/src/epoch_accumulator.rs`; `crates/ade_runtime/src/chaindb/epoch_accumulator_advance.rs`; `crates/ade_node/src/node_lifecycle.rs` |
| **Tests** | `real_preprod_block_130350133_produces_the_cardano_collateral_only_effect`, `a_collateral_return_is_produced_at_len_ordinary_outputs_with_verbatim_bytes`, `a_pre_alonzo_invalid_transaction_fails_closed`, `a_block_with_no_invalid_transactions_is_unchanged`, `invalid_tx_fee_is_resolved_collateral_not_declared_fee`, `an_invalid_txs_certs_are_discarded_not_refused`, `an_invalid_tx_without_a_resolver_still_refuses`, `s3_invalid_tx_proposal_is_discarded_leaving_gov_state_unchanged`, `s3_invalid_tx_vote_is_discarded_leaving_gov_state_unchanged` |
| **CI** | `ci/ci_check_phase2_invalid_utxo_effect.sh`, `ci/ci_check_collateral_balance_resolver.sh` |
| **Claim** | partial — `e563ad94` (pass) — observed: tests: 9 passed, 0 failed, 0 ignored, 0 not found; CI scripts: 2 passed, 0 failed — limits: (1) No IDSE cluster has declared acceptance obligations for this invariant yet (Doctrine §10); this record re-runs the checks carried from the legacy registry. Further limits: (2) No mutation witness exists yet for this critical invariant (Doctrine §28). |

_critical · transactional · scope: UTxO application and accumulators · assumptions: A2 · depends on: CON-001, CON-003 · secondary: CON._ Legacy: DC-LEDGER-PHASE2-01, DC-LEDGER-PHASE2-03

#### `VAL-017` — _partial_

| Aspect | Location |
|--------|----------|
| **Source** | `docs/planning/ledger.define.consensus.md#batch-1` (approved by wdm33, 2026-11-08) |
| **Requirement** | **NativeScriptThresholdSigned** — A native script's at-least-n-of-k threshold is decoded and held as a signed 64-bit integer: every integer in that range is accepted and every other refused, a threshold of zero or less is satisfied, and a positive threshold is satisfied exactly when at least that many sub-scripts pass. |
| **Code** | `crates/ade_types/src/allegra/script.rs`; `crates/ade_codec/src/allegra/script.rs`; `crates/ade_ledger/src/scripts.rs`; `crates/ade_ledger/src/error.rs` |
| **Tests** | `lv11_the_refused_preprod_block_now_decodes_its_negative_threshold`, `lv11_the_real_negative_threshold_script_passes_with_no_signatures`, `lv11_thresholds_inside_int64_decode_to_exactly_their_value`, `lv11_thresholds_outside_int64_are_refused`, `lv11_a_non_integer_threshold_is_still_refused`, `lv11_a_threshold_of_zero_or_below_is_satisfied_outright`, `lv11_a_threshold_of_one_needs_one_passing_sub_script`, `lv11_a_threshold_above_u32_is_not_truncated_into_a_pass`, `lv11_a_decoded_threshold_of_two_to_the_32_fails_end_to_end` |
| **CI** | `ci/ci_check_native_script_threshold_signed.sh` |
| **Claim** | partial — `e563ad94` (pass) — observed: tests: 9 passed, 0 failed, 0 ignored, 0 not found; CI scripts: 1 passed, 0 failed — limits: (1) No IDSE cluster has declared acceptance obligations for this invariant yet (Doctrine §10); this record re-runs the checks carried from the legacy registry. Further limits: (2) No mutation witness exists yet for this critical invariant (Doctrine §28). |

_critical · local · scope: native scripts · assumptions: A2 · depends on: VAL-001._ Legacy: DC-LEDGER-SCRIPT-01

#### `VAL-018` — _partial_

| Aspect | Location |
|--------|----------|
| **Source** | `docs/planning/ledger.define.consensus.md#batch-1` (approved by wdm33, 2026-11-08) |
| **Requirement** | **IntervalNetworkAndBootstrapChecks** — Every Conway transaction in a block lies inside its validity interval at the block's slot (invalid-before at most the slot, slot below invalid-hereafter); every output, collateral return and withdrawal account, and the optional body network field, carry the node's network id; and every bootstrap output's attributes total at most 64 bytes; a violation rejects the transaction and the block, with or without full UTxO context. |
| **Code** | `crates/ade_ledger/src/tx_validity/utxo_free.rs`; `crates/ade_ledger/src/rules.rs`; `crates/ade_ledger/src/tx_validity/phase1.rs`; `crates/ade_ledger/src/late_era_validation.rs` |
| **Tests** | `lv15a_the_validity_interval_is_half_open`, `lv15a_each_network_check_rejects_the_wrong_network`, `lv15a_a_bootstrap_address_network_is_its_magic_attribute`, `lv15a_bootstrap_attributes_may_total_64_bytes`, `lv15a_the_block_path_rejects_a_block_outside_a_tx_validity_interval`, `lv15a_the_block_path_checks_the_network_in_both_modes`, `lv15a_the_standalone_path_checks_the_network`, `lv15a_the_obligations_compose` |
| **CI** | `ci/ci_check_utxo_free_admission_obligations.sh` |
| **Claim** | partial — `e563ad94` (pass) — observed: tests: 8 passed, 0 failed, 0 ignored, 0 not found; CI scripts: 1 passed, 0 failed — limits: (1) No IDSE cluster has declared acceptance obligations for this invariant yet (Doctrine §10); this record re-runs the checks carried from the legacy registry. Further limits: (2) No mutation witness exists yet for this critical invariant (Doctrine §28). |

_critical · local · scope: transaction validation in the block path and standalone · assumptions: A2 · depends on: VAL-009._ Legacy: DC-TXV-09

#### `VAL-019` — _partial_

| Aspect | Location |
|--------|----------|
| **Source** | `docs/planning/ledger.define.consensus.md#batch-1` (approved by wdm33, 2026-11-08) |
| **Requirement** | **BlockBodySizeMatchesHeader** — A Conway block's header body-size field equals the byte length of its four body segments (transaction bodies, witness sets, auxiliary data and invalid transactions) concatenated as received; a mismatch rejects the block. |
| **Code** | `crates/ade_ledger/src/tx_validity/utxo_free.rs`; `crates/ade_ledger/src/rules.rs` |
| **Tests** | `lv15a_the_declared_body_size_must_equal_the_body` |
| **CI** | `ci/ci_check_utxo_free_admission_obligations.sh` |
| **Claim** | partial — `e563ad94` (pass) — observed: tests: 1 passed, 0 failed, 0 ignored, 0 not found; CI scripts: 1 passed, 0 failed — limits: (1) No IDSE cluster has declared acceptance obligations for this invariant yet (Doctrine §10); this record re-runs the checks carried from the legacy registry. Further limits: (2) No mutation witness exists yet for this critical invariant (Doctrine §28). |

_critical · local · scope: block validation · assumptions: A2 · depends on: DET-002._ Legacy: DC-VAL-07

### VAL — epoch

#### `VAL-020` — _partial_

| Aspect | Location |
|--------|----------|
| **Source** | `docs/planning/epoch.define.consensus.md#batch-1` (approved by wdm33, 2026-11-08) |
| **Requirement** | **EpochBoundaryMatchesReference** — For the same prior ledger state, Ade's epoch-boundary transition produces the same new state as the reference node's NEWEPOCH transition, computed solely from canonical chain state and applied in the reference order: the reward update is applied before the new stake snapshot is taken; pool reaping adopts staged re-registrations, reaps exactly the pools retiring at this epoch and returns each reaped pool's deposit to its reward account if registered, otherwise to the treasury; and the rewards, snapshots, treasury, reserves, deposit pots and governance state all match. |
| **Code** | `crates/ade_ledger/src/epoch.rs`; `crates/ade_ledger/src/rules.rs`; `crates/ade_ledger/src/reduced_aggregate.rs`; `crates/ade_runtime/src/chaindb/reduced_utxo_checkpoint.rs`; `ci/ci_check_eview_stake_aggregation.sh`; `crates/ade_ledger/src/bootstrap_reward_update.rs`; `crates/ade_ledger/src/delegation.rs`; `crates/ade_runtime/src/chaindb/reduced_window_driver.rs`; `crates/ade_node/src/epoch_candidate.rs`; `crates/ade_node/src/epoch_wire.rs`; `crates/ade_node/src/native_firstrun.rs`; `crates/ade_node/src/node_lifecycle.rs`; `crates/ade_runtime/src/chaindb`; `crates/ade_ledger/src/ledgerdb_state.rs`; `crates/ade_ledger/src/epoch_accumulator.rs`; `crates/ade_ledger/src/governance.rs`; `crates/ade_ledger/src/snapshot/epoch_state.rs`; `ci/ci_check_snapshot_fidelity.sh` |
| **Tests** | `precise_boundary_comparison_eta_diagnosis`, `alonzo_epoch_boundary_end_to_end`, `regular_epoch_boundary_comparison`, `conway_epoch_boundary_end_to_end`, `sums_utxo_plus_reward_per_delegated_pool`, `reward_without_utxo_contributes`, `undelegated_credential_contributes_nothing`, `delegated_zero_stake_pool_is_included_with_zero` **†**, `multiple_pools_aggregate_independently`, `overflow_is_fail_closed`, `aggregation_is_deterministic`, `sum_base_credential_stake_skips_non_contributing`, `bootstrap_reward_update::tests` (test module), `reduced_window_driver drive_window_applies_bootstrap_reward_delta_to_the_aggregate`, `delegation apply_bootstrap_reward_deltas_sums_and_fails_closed_on_overflow`, `epoch_candidate seed2_window_without_rupd_fails_closed / seed2_window_with_wrong_epoch_rupd_fails_closed / seed2_window_with_correct_rupd_derives / non_seed2_window_without_rupd_is_a_strict_noop`, `epoch_candidate seed2_window_with_a_native_seed_authority_is_a_typed_terminal`, `bootstrap_reward_update a_v3_record_is_refused_not_reinterpreted`, `ade_ledger::rules::cert_state_dispatch::poolreap_ce3a` (test module), `ade_ledger::epoch_accumulator::tests::cross_epoch_boundary_per_credential_mark_pays_member_rewards`, `a_key_and_a_script_credential_with_the_same_bytes_are_two_stakers`, `a_script_credentials_reward_is_never_credited_to_the_key_credential_with_the_same_bytes`, `an_operator_with_a_script_reward_account_is_paid_on_the_script_credential`, `an_owner_is_matched_as_a_key_credential_only`, `a_script_voter_does_not_take_the_stake_of_a_key_credential_with_the_same_bytes`, `read_stake_snapshot_full_decodes_combined_stake_and_delegation`, `a_registration_reward_account_tag_is_the_credential_kind_not_the_network`, `codec_rejects_pre_lv4_store_rebootstrap_required` |
| **CI** | `ci/ci_check_eview_stake_aggregation.sh`, `ci/ci_check_eview_leadership_complete.sh` (red at e563ad94), `ci/ci_check_bootstrap_rupd_window_end.sh`, `ci/ci_check_seed_reward_authority.sh`, `ci/ci_check_poolreap_single_canonical.sh`, `ci/ci_check_snapshot_fidelity.sh`, `ci/ci_check_store_semantics_lock.sh` |
| **Claim** | unresolved — `e563ad94` (fail) — observed: tests: 20 passed, 0 failed, 0 ignored, 8 not found; CI scripts: 6 passed, 1 failed; failing CI: ci/ci_check_eview_leadership_complete.sh — limits: (1) No IDSE cluster has declared acceptance obligations for this invariant yet (Doctrine §10); this record re-runs the checks carried from the legacy registry. Further limits: (2) No mutation witness exists yet for this critical invariant (Doctrine §28). (3) 8 named test(s) were not found in the test output at this revision: delegated_zero_stake_pool_is_included_with_zero, bootstrap_reward_update::tests (round-trip/version/commitment/tamper), reduced_window_driver drive_window_applies_bootstrap_reward_delta_to_the_aggregate, delegation apply_bootstrap_reward_deltas_sums_and_fails_closed_on_overflow, epoch_candidate seed2_window_without_rupd_fails_closed / seed2_window_with_wrong_epoch_rupd_fails_closed / seed2_window_with_correct_rupd_derives / non_seed2_window_without_rupd_is_a_strict_noop, epoch_candidate seed2_window_with_a_native_seed_authority_is_a_typed_terminal, bootstrap_reward_update a_v3_record_is_refused_not_reinterpreted, ade_ledger::rules::cert_state_dispatch::poolreap_ce3a (poolreap_reaps_exact_epoch_only -- == e reaped, > e and a STALE < e kept [proves == not <=]; poolreap_refund_registered_else_treasury -- registered operator refunded its deposit, unregistered -> treasury; poolreap_clears_reaped_pool_delegations -- THE dead-clear regression: a delegation to a reaped pool is cleared, a surviving one kept, the delegator registration preserved; poolreap_script_hash_reward_account_refunds_to_script_cred -- a 0xF0 reward account refunds to its ScriptHash credential, NOT a KeyHash projection of the same 28 bytes, and is not sent to treasury; poolreap_adopts_future_pool_params -- staged re-registration params adopted into the active set, future_pools drained, orphan future dropped) (4) ci/ci_check_eview_leadership_complete.sh fails at this revision and already failed before the IDSE migration (a3cef7f7): "FAIL: aggregate_pool_stake still skips zero-stake delegated pools (cardano numDelegators>0 violated)". Diagnosed as a stale or over-broad structural gate, not a demonstrated violation; the gate needs repair before it can serve as evidence. (5) Corrects this invariant's previous evidence record at the same revision: the earlier record's test matching missed references written as '<crate> tests/<file>.rs::<name>' or '<crate>::<path>'; the test outcomes are the same run's (docs/evidence/idse-m2-reevidence.json). |

_critical · transactional · scope: ledger epoch-boundary transition · assumptions: A2 · depends on: CON-001, DET-001 · secondary: CON._ Legacy: DC-LEDGER-04, CN-EPOCH-03, DC-EPOCH-21, DC-EPOCH-18, DC-EPOCH-43

#### `VAL-021` — _partial_

| Aspect | Location |
|--------|----------|
| **Source** | `docs/planning/epoch.define.consensus.md#batch-1` (approved by wdm33, 2026-11-08) |
| **Requirement** | **RatificationFrozenAtPriorBoundary** — The ratification result applied at the boundary from epoch E to E+1 is decided once from the state the boundary from E-1 to E produced (that boundary's stake distributions, with proposal deposits counted, and epoch E's proposals, votes, DRep registrations, committee, thresholds and parameters), recorded durably and commitment-bound in the governance state, and applied unchanged: the applying step reads no votes, stake, committee, thresholds or expiry, and a record frozen at another epoch, a missing record or an unenactable record halts with no state change. Enactment applies every effect of each ratified action together at that boundary, never part of them. |
| **Code** | `crates/ade_ledger/src/governance.rs`; `crates/ade_ledger/src/rules.rs`; `crates/ade_ledger/src/governance/frozen.rs`; `crates/ade_ledger/src/state.rs`; `crates/ade_ledger/src/gov_cert.rs`; `crates/ade_ledger/src/snapshot/gov_state.rs`; `crates/ade_ledger/src/ledgerdb_state.rs`; `crates/ade_runtime/src/mithril_native_assembly.rs`; `ci/ci_check_frozen_ratification_authority.sh` |
| **Tests** | `conway_epoch_boundary_end_to_end`, `conway_governance_ratification_test`, `enact_noconfidence_dissolves_committee`, `enact_update_committee_applies_changes`, `committee_enactment_replays_byte_identical`, `epoch_boundary_ratifiable_noconfidence_is_terminal_pending_enactment`, `committee_oracle_mainnet_575_576_noop_agreement` (FAILED at e563ad94), `ce_lv15b0_1_late_votes_do_not_enact_at_the_consuming_boundary`, `ce_lv15b0_2_late_proposal_is_carried`, `lv15b0_expired_proposal_removes_its_descendants`, `lv15b0_expired_sibling_is_removed_as_expired_not_pruned`, `ce_lv15b0_1_a_vote_cast_during_epoch_e_enacts_at_e_plus_2`, `ce_lv15b0_3a_account_unregistration_removes_vote_delegation_and_reregistration_starts_empty`, `ce_lv15b0_3a_drep_reregistration_does_not_regain_old_delegators`, `ce_lv15b0_3b_vote_delegation_to_unregistered_drep_is_rejected`, `ce_lv15b0_3b_drep_registration_then_delegation_is_ordered`, `ce_lv15b0_3b_dangling_delegation_is_a_terminal_not_excluded`, `ce_lv15b0_drep_distribution_counts_every_term_for_every_delegator`, `ce_lv15b0_4_pool_distribution_is_mark_plus_deposits`, `ce_lv15b0_3_4_boundary_evidence_is_post_boundary_accounts_and_the_new_mark`, `ce_lv15b0_5_7_changes_after_the_freeze_do_not_change_the_outcome`, `ce_lv15b0_6_application_prunes_late_siblings_and_routes_by_registration_now`, `ce_lv15b0_6_unenactable_halts_at_the_consuming_boundary`, `ce_lv15b0_8_import_supported_enactment_is_decided_and_cross_checked`, `ce_lv15b0_8_import_unsupported_enactments_are_unenactable`, `lv15b0_boundary_refuses_a_state_without_a_record`, `lv15b0_boundary_refuses_a_record_from_another_epoch`, `record_from_another_epoch_is_refused`, `lv15b0_v4_record_round_trips_byte_exact`, `lv15b0_v3_bytes_decode_unversioned`, `lv15b0_evidence_only_change_is_visible`, `lv15b0_record_decoder_refuses_non_ascending_keys`, `codec_rejects_pre_lv15b0_store_rebootstrap_required`, `lv15b0_drep_pulser_is_imported_as_the_snapshot_epoch_record`, `lv15b0_pulser_parameters_mismatch_is_refused`, `lv15b0_pulser_root_mismatch_is_refused`, `lv15b0_pulser_delayed_without_unenactable_action_is_refused`, `lv15b0_malformed_pulser_is_refused`, `lv15b0_dangling_vote_delegation_in_a_snapshot_is_refused`, `protocol_major_below_10_is_refused` |
| **CI** | `ci/ci_check_credential_discriminant_closed.sh`, `ci/ci_check_frozen_ratification_authority.sh`, `ci/ci_check_store_semantics_lock.sh` |
| **Claim** | unresolved — `e563ad94` (fail) — observed: tests: 39 passed, 1 failed, 0 ignored, 0 not found; CI scripts: 3 passed, 0 failed; failing tests: committee_oracle_mainnet_575_576_noop_agreement — limits: (1) No IDSE cluster has declared acceptance obligations for this invariant yet (Doctrine §10); this record re-runs the checks carried from the legacy registry. Further limits: (2) No mutation witness exists yet for this critical invariant (Doctrine §28). (3) committee_oracle_mainnet_575_576_noop_agreement fails because the test passes DormantEpochs::Unversioned, which the ratification gate now refuses (fail-closed dormancy); the test input is stale, not a counterexample. |

_critical · transactional · scope: Conway governance ratification and enactment · assumptions: A2 · depends on: VAL-020, ORD-003 · secondary: ORD._ Legacy: DC-EPOCH-01, DC-EPOCH-47

#### `VAL-022` — _partial_

| Aspect | Location |
|--------|----------|
| **Source** | `docs/planning/epoch.define.consensus.md#batch-1` (approved by wdm33, 2026-11-08) |
| **Requirement** | **PraosNonceEvolution** — Each validated header drives one indivisible nonce transition as the reference Praos protocol defines it: the evolving nonce becomes the evolving nonce combined with the header's VRF nonce output; the candidate nonce becomes the new evolving nonce while the header's slot is below firstSlotNextEpoch minus ceil(4k/f), and otherwise stays frozen; the last-block nonce becomes the hash of the header's previous block. At each epoch tick the epoch nonce becomes the candidate nonce combined with the last-epoch-block nonce, the previous epoch nonce becomes the old epoch nonce, the last-epoch-block nonce becomes the last-block nonce, and the evolving and candidate nonces carry over unchanged. Combining is blake2b-256 of the concatenation, with the neutral nonce as identity and no extra-entropy operand; an absent last-epoch-block nonce fails closed unless a valid bootstrap bridge supplies it, and no nonce is ever fabricated. The first epoch nonce after bootstrap comes from that same tick over the live chain state; a seed-time nonce stored at bootstrap must equal it when the seed sat at or after the freeze slot and is not used otherwise, and a seed position that cannot be located relative to the freeze, or a network whose stability window is unknown, is terminal. |
| **Code** | `crates/ade_core/src/consensus/nonce.rs`; `crates/ade_core/src/consensus/praos_state.rs`; `crates/ade_ledger/src/snapshot/chain_dep.rs`; `crates/ade_core/src/consensus/header_validate.rs`; `crates/ade_node/src/node_sync.rs`; `crates/ade_core/src/consensus/era_schedule.rs`; `crates/ade_runtime/src/consensus/genesis_parser.rs`; `crates/ade_node/src/bootstrap_export.rs`; `crates/ade_ledger/src/ledgerdb_state.rs`; `ci/ci_check_praos_nonce_follow_evolution.sh`; `crates/ade_ledger/src/seed_consensus_inputs.rs`; `crates/ade_node/src/node_lifecycle.rs`; `ci/build_consensus_inputs_bundle.sh`; `crates/ade_runtime/src/consensus_inputs/importer.rs`; `crates/ade_ledger/src/bootstrap_bridge.rs`; `crates/ade_node/src/epoch_wire.rs`; `crates/ade_node/src/native_firstrun.rs` |
| **Tests** | `bridge_equivalence_seeded_snapshot_tick_reproduces_eca5_eta0` **†**, `header_contribution_advances_evolving_lab_candidate` **†**, `candidate_freezes_at_freeze_boundary` **†**, `epoch_tick_combines_candidate_with_last_epoch_block_nonce_no_reset` **†**, `epoch_tick_rotates_last_epoch_block_nonce_from_lab` **†**, `epoch_tick_fails_closed_on_absent_operand` **†**, `chain_dep_array10_round_trip_some_and_none` **†**, `chain_dep_legacy_array9_decodes_to_none`, `chain_dep_always_writes_array10` **†**, `b1_store_round_trip_reproduces_next_boundary_eta0` **†**, `seed_cinput_v6_persists_k_for_durable_candidate_freeze_window`, `sidecar_freeze_rsw_derives_from_store_and_cross_checks_the_cli`, `dc_epoch_38_bridge_nonce_freeze_differential_covers_both_sides_per_venue`, `n2_pre_freeze_seed_binds_the_boundary_tick_not_the_stored_projection`, `n2_final_seed_must_corroborate_the_boundary_tick`, `n2_underivable_finality_is_terminal`, `n2_live_and_recovery_bind_the_same_view_byte_identically`, `n2_recovery_nonce_must_be_bound_to_the_target_epoch`, `preprod_snapshot_6009_seed_is_pending`, `seed_exactly_at_the_freeze_slot_is_final`, `preview_geometry_decides_identically_at_the_same_relative_position`, `oversized_rsw_saturates_and_is_final` |
| **CI** | `ci/ci_check_praos_nonce_follow_evolution.sh`, `ci/ci_check_bridge_nonce_freeze_differential.sh` |
| **Claim** | partial — `e563ad94` (pass) — observed: tests: 13 passed, 0 failed, 0 ignored, 9 not found; CI scripts: 2 passed, 0 failed — limits: (1) No IDSE cluster has declared acceptance obligations for this invariant yet (Doctrine §10); this record re-runs the checks carried from the legacy registry. Further limits: (2) No mutation witness exists yet for this critical invariant (Doctrine §28). (3) 9 named test(s) were not found in the test output at this revision: bridge_equivalence_seeded_snapshot_tick_reproduces_eca5_eta0, header_contribution_advances_evolving_lab_candidate, candidate_freezes_at_freeze_boundary, epoch_tick_combines_candidate_with_last_epoch_block_nonce_no_reset, epoch_tick_rotates_last_epoch_block_nonce_from_lab, epoch_tick_fails_closed_on_absent_operand, chain_dep_array10_round_trip_some_and_none, chain_dep_always_writes_array10 … |

_critical · local · scope: Praos chain-dependent nonce state · assumptions: A2 · depends on: SEC-004 · secondary: DET._ Legacy: DC-EPOCH-16, DC-EPOCH-38

#### `VAL-023` — _partial_

| Aspect | Location |
|--------|----------|
| **Source** | `docs/planning/epoch.define.consensus.md#batch-1` (approved by wdm33, 2026-11-08) |
| **Requirement** | **LeadershipSealedAtBoundaryPoint** — At each epoch boundary from E to E+1 that the node crosses itself, the leadership stake distribution for epoch E+2 is sealed as a frozen leadership object in the same durable commit as the accumulator advance, and no boundary advance commits without it: its pool set is the registered pools with at least one delegator in the delegation map before pool reaping, its stake comes from the stake mark captured from the reduced checkpoint positioned exactly at the closing epoch's last durable block (bound to that point and its lineage before use), and its VRF keys come from the pool parameters frozen before reaping. The two epochs no crossing produces, the bootstrap epoch and the one after it, are sealed instead from the certified snapshot's pool distribution and its imported mark snapshot, bound to the certified bootstrap point. |
| **Code** | `crates/ade_ledger/src/consensus_view.rs`; `crates/ade_core/src/consensus/vrf_cert.rs`; `crates/ade_ledger/src/reduced_aggregate.rs`; `crates/ade_runtime/src/chaindb/reduced_utxo_checkpoint.rs`; `ci/ci_check_eview_stake_aggregation.sh`; `crates/ade_ledger/src/reduced_epoch_view.rs`; `crates/ade_node/src/epoch_candidate.rs`; `ci/ci_check_eview_leadership_complete.sh`; `crates/ade_node/src/epoch_source_window.rs`; `ci/ci_check_eview_source_window.sh`; `ci/ci_check_eview_candidate.sh`; `crates/ade_node/src/node_lifecycle.rs`; `ci/ci_check_eview_refold_reseal.sh` |
| **Tests** | `recovered_surface_projects_pooldistrview_and_expected_vrf_input`, `projection_maps_recovered_fields_onto_ledgerview_surface`, `projection_two_runs_identical`, `projection_off_epoch_returns_none`, `sums_utxo_plus_reward_per_delegated_pool`, `reward_without_utxo_contributes`, `undelegated_credential_contributes_nothing`, `delegated_zero_stake_pool_is_included_with_zero` **†**, `multiple_pools_aggregate_independently`, `overflow_is_fail_closed`, `aggregation_is_deterministic`, `sum_base_credential_stake_skips_non_contributing`, `leadership_complete_required_for_matches`, `canonical_hash_is_binding_sensitive`, `derive_candidate_binds_target_epoch_and_round_trips_through_recovery`, `derive_candidate_canonical_hash_is_replay_equivalent`, `target_epoch_is_the_explicit_lag`, `valid_window_passes`, `empty_window_fails_closed`, `out_of_window_block_fails_closed`, `unordered_and_duplicate_fail_closed`, `missing_block_breaks_the_chain`, `anchor_and_lineage_pin_fail_closed`, `wrong_target_epoch_fails_closed`, `ade_runtime::chaindb::epoch_accumulator_advance::tests::boundary_block_crosses_with_mark`, `ade_runtime::chaindb::epoch_accumulator_advance::tests::boundary_cross_is_idempotent`, `ade_runtime::chaindb::epoch_accumulator_advance::tests::boundary_cross_missing_block_is_a_fault`, `ade_runtime::chaindb::epoch_accumulator_store::tests::boundary_mark_witness_bind_read_clear_round_trip`, `ade_runtime::chaindb::epoch_accumulator_store::tests::boundary_mark_bind_requires_sealed`, `ade_runtime::chaindb::epoch_accumulator_store::tests::reset_to_bootstrap_drops_the_boundary_mark_binding`, `ade_runtime::chaindb::epoch_accumulator_store::tests::boundary_mark_binding_survives_reopen`, `ade_node::node_lifecycle::tests::co_advance_ledger_state::co_advance_crosses_a_boundary`, `ade_node::node_lifecycle::tests::co_advance_ledger_state::co_advance_checkpoint_only_when_no_accumulator`, `ade_node::node_lifecycle::tests::co_advance_ledger_state::co_advance_multi_boundary_catch_up`, `ade_node::node_lifecycle::tests::co_advance_ledger_state::co_advance_observe_only_when_no_checkpoint`, `positioning_rewinds_a_checkpoint_that_sits_past_the_boundary_point`, `a_bare_forward_advance_asked_to_go_backward_silently_moves_nothing`, `a_boundary_point_before_the_sealed_seed_is_unreachable` |
| **CI** | `ci/ci_check_eview_stake_aggregation.sh`, `ci/ci_check_eview_leadership_complete.sh` (red at e563ad94), `ci/ci_check_eview_source_window.sh`, `ci/ci_check_eview_candidate.sh`, `ci/ci_check_boundary_aligned_mark_capture.sh`, `ci/ci_check_eview_refold_reseal.sh` |
| **Claim** | unresolved — `e563ad94` (fail) — observed: tests: 37 passed, 0 failed, 0 ignored, 1 not found; CI scripts: 5 passed, 1 failed; failing CI: ci/ci_check_eview_leadership_complete.sh — limits: (1) No IDSE cluster has declared acceptance obligations for this invariant yet (Doctrine §10); this record re-runs the checks carried from the legacy registry. Further limits: (2) No mutation witness exists yet for this critical invariant (Doctrine §28). (3) 1 named test(s) were not found in the test output at this revision: delegated_zero_stake_pool_is_included_with_zero (4) ci/ci_check_eview_leadership_complete.sh fails at this revision and already failed before the IDSE migration (a3cef7f7): "FAIL: aggregate_pool_stake still skips zero-stake delegated pools (cardano numDelegators>0 violated)". Diagnosed as a stale or over-broad structural gate, not a demonstrated violation; the gate needs repair before it can serve as evidence. (5) Corrects this invariant's previous evidence record at the same revision: the earlier record's test matching missed references written as '<crate> tests/<file>.rs::<name>' or '<crate>::<path>'; the test outcomes are the same run's (docs/evidence/idse-m2-reevidence.json). |

_critical · transactional · scope: epoch-view derivation, boundary mark capture · assumptions: A2 · depends on: VAL-020, STR-007 · secondary: ORD._ Legacy: DC-EPOCH-08, DC-EPOCH-09, DC-EPOCH-22, DC-EPOCH-32

#### `VAL-024` — _partial_

| Aspect | Location |
|--------|----------|
| **Source** | `docs/planning/epoch.define.consensus.md#batch-1` (approved by wdm33, 2026-11-08) |
| **Requirement** | **LeadershipFromSealedView** — The stake distribution used for a leader check in epoch E comes only from the sealed frozen leadership object whose target epoch is exactly E, read after its protocol-parameter commitment is verified; it is never re-derived at use time, never taken from a latest or nearest object, and never replaced by window replay or the bootstrap view, and a missing, unsealed or malformed object fails closed. Its sigma denominator is the snapshot's total active stake, computed once from the snapshot's credential-side stake before any pool-membership filtering, by one definition shared by native sealing and bootstrap seeding, and included in the object's canonical encoding and hash; it is never computed by summing the object's pool entries, at sealing or at reading, and a missing total fails closed. |
| **Code** | `crates/ade_node/src/node_sync.rs`; `crates/ade_node/src/node_lifecycle.rs`; `crates/ade_ledger/src/consensus_view.rs`; `crates/ade_ledger/src/reduced_epoch_view.rs`; `crates/ade_node/src/epoch_candidate.rs`; `ci/ci_check_eview_leadership_complete.sh`; `crates/ade_ledger/src/frozen_leadership.rs`; `crates/ade_runtime/src/chaindb/epoch_accumulator_store.rs`; `crates/ade_node/src/native_firstrun.rs`; `crates/ade_ledger/src/epoch_accumulator.rs`; `crates/ade_runtime/src/chaindb/epoch_accumulator_advance.rs`; `crates/ade_node/src/epoch_wire.rs`; `ci/ci_check_frozen_leadership_authority.sh`; `ci/ci_check_frozen_promotion_no_seed_window.sh`; `crates/ade_ledger/src/epoch.rs`; `crates/ade_ledger/src/store_semantics.rs`; `ci/ci_check_sigma_denominator_authority.sh` |
| **Tests** | `forge_from_recovered_uses_recovered_pool_distr`, `forge_from_recovered_fails_closed_without_recovered_inputs`, `forge_from_recovered_is_deterministic_across_two_runs`, `feed_header_validates_against_recovered_surface_not_empty_view`, `leadership_complete_required_for_matches`, `canonical_hash_is_binding_sensitive`, `derive_candidate_binds_target_epoch_and_round_trips_through_recovery`, `derive_candidate_canonical_hash_is_replay_equivalent`, `to_pool_distr_view_builds_from_bound_profile_and_rejects_wrong_params`, `projection_rejects_wrong_profile_through_the_real_derive_path`, `ade_ledger::frozen_leadership::tests` (test module), `ade_ledger::frozen_leadership::tests::from_boundary_snapshot_is_the_delegation_image_not_the_full_registered_set`, `ade_ledger::epoch_accumulator::tests::boundary_leadership_effect_batch_invariants_are_enforced`, `ade_runtime::chaindb::epoch_accumulator_store::tests` (test module), `ade_runtime::chaindb::epoch_accumulator_store::tests::reset_to_bootstrap_restores_only_bootstrap_indexed_leadership`, `ade_runtime::chaindb::epoch_accumulator_store::tests::reset_clears_current_leadership_when_no_bootstrap_object`, `ade_runtime::chaindb::epoch_accumulator_store::tests::seal_advance_reset_round_trip_is_exact`, `ce3d_boundary_differential::s4pre_frozen_leadership_seed_identity` (#[ignore]), `ce3d_boundary_differential::s4pre_1c_frozen_leadership_bootstrap_lineage` (#[ignore]), `ce3d_boundary_differential::s4_0_epoch_indexed_leadership_acceptance_1338_to_1342` (#[ignore]), `ade_node::node_lifecycle::tests::s4_l1_frozen_leadership_view_is_byte_identical_to_seed_and_fails_closed`, `ce3d_boundary_differential::ldat_classify_leadership_pools` (#[ignore]), `ce3d_boundary_differential::s5_recovery_replay_equivalence_within_k_rollback` (#[ignore]), `ade_node::epoch_wire::tests::s4_l2_frozen_promotion_crosses_seed_plus_2_and_seed_plus_3`, `ade_node::epoch_wire::tests::s4_l2_frozen_promotion_fails_closed_on_every_non_promotion_source`, `ade_runtime::chaindb::epoch_accumulator_store::tests::leadership_authority_rejects_malformed_object`, `adding_or_removing_a_pool_does_not_move_the_denominator`, `the_retired_pool_no_longer_deflates_every_other_pools_sigma`, `the_credential_side_sum_includes_stake_the_membership_filter_would_drop`, `the_total_round_trips_and_is_committed_to` |
| **CI** | `ci/ci_check_consensus_input_provenance.sh` (red at e563ad94), `ci/ci_check_recovered_ledger_pparams_sourced.sh` (red at e563ad94), `ci/ci_check_feed_leader_threshold_view.sh` (red at e563ad94), `ci/ci_check_eview_leadership_complete.sh` (red at e563ad94), `ci/ci_check_frozen_leadership_authority.sh`, `ci/ci_check_frozen_promotion_no_seed_window.sh`, `ci/ci_check_frozen_recovery_no_seed_window.sh`, `ci/ci_check_sigma_denominator_authority.sh`, `ci/ci_check_store_semantics_lock.sh` |
| **Claim** | unresolved — `e563ad94` (fail) — observed: tests: 23 passed, 0 failed, 5 ignored, 2 not found; CI scripts: 5 passed, 4 failed; failing CI: ci/ci_check_consensus_input_provenance.sh, ci/ci_check_recovered_ledger_pparams_sourced.sh, ci/ci_check_feed_leader_threshold_view.sh, ci/ci_check_eview_leadership_complete.sh — limits: (1) No IDSE cluster has declared acceptance obligations for this invariant yet (Doctrine §10); this record re-runs the checks carried from the legacy registry. Further limits: (2) No mutation witness exists yet for this critical invariant (Doctrine §28). (3) 2 named test(s) were not found in the test output at this revision: ade_ledger::frozen_leadership::tests (codec round-trip, stable + content-bound hash, zero-stake preserved, wrong-version / duplicate / unsorted / trailing rejected; to_pool_distr_view reads stake+VRF directly), ade_runtime::chaindb::epoch_accumulator_store::tests (S4-0 epoch-indexed: seal_current_and_read_exact_by_epoch, epoch_indexed_leadership_survives_reopen, leadership_authority_fails_closed_on_legacy_store, leadership_authority_rejects_missing_epoch_under_valid_marker, leadership_authority_rejects_wrong_epoch_object, leadership_authority_rejects_wrong_version_marker, leadership_authority_rejects_malformed_object, seal_bootstrap_leadership_epochs_rejects_duplicate_epoch) (4) 5 named test(s) are #[ignore]d and did not run. (5) ci/ci_check_consensus_input_provenance.sh fails at this revision and already failed before the IDSE migration (a3cef7f7): "FAIL: production file crates/ade_runtime/src/mithril_bootstrap.rs references merge_seed_epoch_consensus_inputs( outside the shared populator authority ([legacy ref])". Diagnosed as a stale or over-broad structural gate, not a demonstrated violation; the gate needs repair before it can serve as evidence. (6) ci/ci_check_recovered_ledger_pparams_sourced.sh fails at this revision and already failed before the IDSE migration (a3cef7f7): "FAIL: build_seed_ledger does not take a protocol_params argument (crates/ade_node/src/admission/seed_to_snapshot.rs)". Diagnosed as a stale or over-broad structural gate, not a demonstrated violation; the gate needs repair before it can serve as evidence. (7) ci/ci_check_feed_leader_threshold_view.sh fails at this revision and already failed before the IDSE migration (a3cef7f7): "FAIL: from_seed_epoch_consensus_inputs must be defined exactly once (the single projection authority); found 2". Diagnosed as a stale or over-broad structural gate, not a demonstrated violation; the gate needs repair before it can serve as evidence. (8) ci/ci_check_eview_leadership_complete.sh fails at this revision and already failed before the IDSE migration (a3cef7f7): "FAIL: aggregate_pool_stake still skips zero-stake delegated pools (cardano numDelegators>0 violated)". Diagnosed as a stale or over-broad structural gate, not a demonstrated violation; the gate needs repair before it can serve as evidence. (9) Corrects this invariant's previous evidence record at the same revision: the earlier record's test matching missed references written as '<crate> tests/<file>.rs::<name>' or '<crate>::<path>'; the test outcomes are the same run's (docs/evidence/idse-m2-reevidence.json). |

_critical · local · scope: leadership stake distribution · assumptions: A2 · depends on: STR-007, VAL-023._ Legacy: DC-EPOCH-12, DC-EPOCH-25, DC-EPOCH-40

#### `VAL-025` — _partial_

| Aspect | Location |
|--------|----------|
| **Source** | `docs/planning/epoch.define.consensus.md#batch-1` (approved by wdm33, 2026-11-08) |
| **Requirement** | **LedgerEpochAgreesWithSchedule** — After the epoch-boundary decision for a block, the ledger's epoch equals the network's era-schedule epoch for that block's slot; a disagreement in either direction is a typed failure that halts the apply with no state change. |
| **Code** | `crates/ade_ledger/src/state.rs`; `crates/ade_ledger/src/rules.rs`; `crates/ade_ledger/src/error.rs`; `crates/ade_ledger/src/phase.rs` |
| **Tests** | `epoch_agreement_rejects_a_stale_ledger_epoch`, `epoch_agreement_rejects_a_ledger_ahead_of_the_schedule`, `epoch_agreement_accepts_agreement`, `epoch_agreement_is_silent_on_an_unlocatable_slot` |
| **CI** | `ci/ci_check_epoch_agreement.sh` |
| **Claim** | partial — `e563ad94` (pass) — observed: tests: 4 passed, 0 failed, 0 ignored, 0 not found; CI scripts: 1 passed, 0 failed — limits: (1) No IDSE cluster has declared acceptance obligations for this invariant yet (Doctrine §10); this record re-runs the checks carried from the legacy registry. Further limits: (2) No mutation witness exists yet for this critical invariant (Doctrine §28). |

_critical · local · scope: ledger epoch, era schedule · assumptions: A2 · depends on: VAL-012, SAF-001 · secondary: SAF._ Legacy: DC-EPOCH-36

#### `VAL-026` — _partial_

| Aspect | Location |
|--------|----------|
| **Source** | `docs/planning/epoch.define.consensus.md#batch-1` (approved by wdm33, 2026-11-08) |
| **Requirement** | **SnapshotNonZeroActiveStake** — A stake snapshot includes a registered, delegated stake credential if and only if its combined active stake (base-address UTxO coin plus reward balance) is non-zero, so a pool whose delegators all hold zero stake is absent from the snapshot. |
| **Code** | `crates/ade_ledger/src/epoch_accumulator.rs`; `crates/ade_ledger/src/reduced_aggregate.rs`; `crates/ade_runtime/src/chaindb/epoch_accumulator_store.rs`; `crates/ade_ledger/src/ledgerdb_state.rs`; `crates/ade_ledger/src/fingerprint.rs`; `ci/ci_check_snapshot_pool_set_inclusion.sh` |
| **Tests** | `ade_ledger::epoch_accumulator::tests::build_boundary_mark_snapshot_omits_zero_stake_credential`, `ade_ledger::reduced_aggregate::tests::delegated_zero_stake_pool_is_omitted`, `ade_ledger::reduced_aggregate::tests::ssactivestake_membership_decision_table`, `ade_ledger::epoch_accumulator::tests::cross_epoch_boundary_per_credential_mark_pays_member_rewards`, `ade_ledger::epoch_accumulator::tests::codec_rejects_pre_c_v3_store_rebootstrap_required` |
| **CI** | `ci/ci_check_snapshot_pool_set_inclusion.sh` |
| **Claim** | partial — `e563ad94` (pass) — observed: tests: 5 passed, 0 failed, 0 ignored, 0 not found; CI scripts: 1 passed, 0 failed — limits: (1) No IDSE cluster has declared acceptance obligations for this invariant yet (Doctrine §10); this record re-runs the checks carried from the legacy registry. Further limits: (2) No mutation witness exists yet for this critical invariant (Doctrine §28). (3) Corrects this invariant's previous evidence record at the same revision: the earlier record's test matching missed references written as '<crate> tests/<file>.rs::<name>' or '<crate>::<path>'; the test outcomes are the same run's (docs/evidence/idse-m2-reevidence.json). |

_critical · local · scope: stake snapshots (mark, set, go) · assumptions: A2 · depends on: VAL-020._ Legacy: DC-EPOCH-24

#### `VAL-027` — _partial_

| Aspect | Location |
|--------|----------|
| **Source** | `docs/planning/epoch.define.consensus.md#batch-1` (approved by wdm33, 2026-11-08) |
| **Requirement** | **OwnerStakeSelfDelegated** — In the reward calculation a pool's owner stake is the go-snapshot stake of only those owners delegated to that pool in the go snapshot, defined in one place and used for both the pledge check and the leader reward; an owner delegated elsewhere, or not at all, contributes nothing. |
| **Code** | `crates/ade_ledger/src/epoch.rs`; `crates/ade_ledger/src/rules.rs`; `crates/ade_ledger/src/store_semantics.rs`; `ci/ci_check_owner_stake_authority.sh` |
| **Tests** | `a_pool_whose_owner_delegates_elsewhere_earns_nothing`, `the_same_owner_delegated_to_the_pool_satisfies_the_pledge`, `the_leader_term_counts_only_owners_delegated_to_the_pool`, `only_the_self_delegated_subset_of_owners_is_summed`, `a_pledge_met_only_by_counting_owners_elsewhere_is_not_met`, `an_owner_is_matched_as_a_key_credential_only` |
| **CI** | `ci/ci_check_owner_stake_authority.sh`, `ci/ci_check_store_semantics_lock.sh`, `ci/ci_check_snapshot_fidelity.sh` |
| **Claim** | partial — `e563ad94` (pass) — observed: tests: 6 passed, 0 failed, 0 ignored, 0 not found; CI scripts: 3 passed, 0 failed — limits: (1) No IDSE cluster has declared acceptance obligations for this invariant yet (Doctrine §10); this record re-runs the checks carried from the legacy registry. Further limits: (2) No mutation witness exists yet for this critical invariant (Doctrine §28). |

_critical · local · scope: epoch-boundary rewards · assumptions: A2 · depends on: VAL-020._ Legacy: DC-EPOCH-42

#### `VAL-028` — _partial_

| Aspect | Location |
|--------|----------|
| **Source** | `docs/planning/epoch.define.consensus.md#batch-1` (approved by wdm33, 2026-11-08) |
| **Requirement** | **FrozenPoolParameters** — Every stake snapshot carries, for each pool registered when it was taken, that pool's pledge, cost, margin, owners and reward account as they were at that moment, taken at the snapshot step before future-pool adoption and pool reaping. A seed import decodes them from the certified snapshot, checks each entry's derivable values (stake, self-delegated owner stake) against the snapshot, and refuses a staked pool with no entry. The reward calculation reads pool parameters only from the go snapshot, never from the registrations current at the boundary, and a pool with blocks and go-snapshot stake but no frozen parameters is a typed terminal, never skipped. |
| **Code** | `crates/ade_ledger/src/epoch.rs`; `crates/ade_ledger/src/epoch_accumulator.rs`; `crates/ade_ledger/src/rules.rs`; `crates/ade_ledger/src/governance.rs`; `crates/ade_ledger/src/ledgerdb_state.rs`; `crates/ade_testkit/src/harness/snapshot_loader.rs`; `ci/ci_check_snapshot_fidelity.sh` |
| **Tests** | `a_retired_pool_is_paid_for_its_last_blocks_from_its_frozen_parameters`, `a_re_registered_pool_is_paid_on_the_values_frozen_in_the_go_snapshot`, `a_pool_that_earned_with_no_frozen_parameters_is_a_terminal`, `the_mark_freezes_parameters_before_adoption_and_before_poolreap`, `read_stake_snapshot_full_decodes_combined_stake_and_delegation`, `read_stake_snapshot_full_refuses_an_entry_that_disagrees_with_its_snapshot` |
| **CI** | `ci/ci_check_snapshot_fidelity.sh`, `ci/ci_check_store_semantics_lock.sh` |
| **Claim** | partial — `e563ad94` (pass) — observed: tests: 6 passed, 0 failed, 0 ignored, 0 not found; CI scripts: 2 passed, 0 failed — limits: (1) No IDSE cluster has declared acceptance obligations for this invariant yet (Doctrine §10); this record re-runs the checks carried from the legacy registry. Further limits: (2) No mutation witness exists yet for this critical invariant (Doctrine §28). |

_critical · local · scope: stake snapshots, rewards, seed import · assumptions: A2 · depends on: VAL-020._ Legacy: DC-EPOCH-44

#### `VAL-029` — _partial_

| Aspect | Location |
|--------|----------|
| **Source** | `docs/planning/epoch.define.consensus.md#batch-1` (approved by wdm33, 2026-11-08) |
| **Requirement** | **SeedBoundaryRewardAuthority** — The reward update applied at the first boundary after bootstrap has exactly one source, chosen by the certified snapshot's reward-update state and recorded as a closed sum inside the durable record's commitment, so an absent update is never represented as a zero-valued one: a completed update is applied once, reducing the fee pot by its fee value; when no update was started, the ordinary reward calculation runs from the snapshot's inputs, the same function every later boundary uses, and the imported fee pot is reduced by the snapshot's fee value before rotation; a snapshot taken while the update was still being computed is refused. A native calculation that yields a zero pool pot while the previous epoch's block counts are non-empty is terminal, and no imported reward delta is applied except from a completed imported update. |
| **Code** | `crates/ade_ledger/src/bootstrap_reward_update.rs`; `crates/ade_ledger/src/delegation.rs`; `crates/ade_runtime/src/chaindb/reduced_window_driver.rs`; `crates/ade_node/src/epoch_candidate.rs`; `crates/ade_node/src/epoch_wire.rs`; `crates/ade_node/src/native_firstrun.rs`; `crates/ade_node/src/node_lifecycle.rs`; `crates/ade_runtime/src/chaindb`; `crates/ade_ledger/src/ledgerdb_state.rs`; `crates/ade_ledger/src/epoch_accumulator.rs`; `ci/ci_check_bootstrap_rupd_fee_reduction.sh`; `crates/ade_ledger/src/store_semantics.rs`; `ci/ci_check_seed_reward_authority.sh` |
| **Tests** | `bootstrap_reward_update::tests` (test module), `reduced_window_driver drive_window_applies_bootstrap_reward_delta_to_the_aggregate`, `delegation apply_bootstrap_reward_deltas_sums_and_fails_closed_on_overflow`, `epoch_candidate seed2_window_without_rupd_fails_closed / seed2_window_with_wrong_epoch_rupd_fails_closed / seed2_window_with_correct_rupd_derives / non_seed2_window_without_rupd_is_a_strict_noop`, `epoch_candidate seed2_window_with_a_native_seed_authority_is_a_typed_terminal`, `bootstrap_reward_update a_v3_record_is_refused_not_reinterpreted`, `ade_ledger::ledgerdb_state::tip_tests::read_reward_update_deltas_returns_delta_fees`, `ade_ledger::ledgerdb_state::tip_tests::read_reward_update_deltas_rejects_malformed_delta_fees`, `ade_ledger::bootstrap_reward_update::tests::v3_round_trips_delta_fees`, `ade_ledger::bootstrap_reward_update::tests::v3_rejects_genuine_v2_blob`, `ade_ledger::bootstrap_reward_update::tests::tampered_delta_fees_breaks_the_commitment`, `ade_ledger::bootstrap_reward_update::tests::decode_rejects_unknown_version`, `ade_ledger::epoch_accumulator::tests::seed_boundary_reduces_fee_pot_by_delta_fees`, `ade_ledger::epoch_accumulator::tests::seed_boundary_fee_reduction_underflow_fails_closed`, `ade_ledger::epoch_accumulator::tests::non_seed_boundary_never_reduces_fee_pot`, `ade_ledger::epoch_accumulator::tests::codec_rejects_unknown_version`, `snothing_decodes_to_not_yet_computed_carrying_ss_fee`, `an_empty_complete_update_is_not_not_yet_computed`, `a_pulsing_update_still_fails_closed`, `both_variants_round_trip_as_themselves`, `a_v3_record_is_refused_not_reinterpreted`, `the_variant_is_inside_the_commitment`, `native_seed_boundary_runs_the_native_reward_on_nesbprev_and_fee_ss`, `native_seed_boundary_reward_depends_on_fee_ss`, `native_seed_boundary_rotates_exactly_the_seed_epochs_own_fees`, `native_seed_boundary_with_blocks_and_a_zero_pot_is_a_terminal`, `accumulator_codec_round_trips_a_native_seed_authority`, `seed2_window_with_a_native_seed_authority_is_a_typed_terminal` |
| **CI** | `ci/ci_check_bootstrap_rupd_window_end.sh`, `ci/ci_check_seed_reward_authority.sh`, `ci/ci_check_bootstrap_rupd_fee_reduction.sh`, `ci/ci_check_store_semantics_lock.sh` |
| **Claim** | partial — `e563ad94` (pass) — observed: tests: 22 passed, 0 failed, 0 ignored, 6 not found; CI scripts: 4 passed, 0 failed — limits: (1) No IDSE cluster has declared acceptance obligations for this invariant yet (Doctrine §10); this record re-runs the checks carried from the legacy registry. Further limits: (2) No mutation witness exists yet for this critical invariant (Doctrine §28). (3) 6 named test(s) were not found in the test output at this revision: bootstrap_reward_update::tests (round-trip/version/commitment/tamper), reduced_window_driver drive_window_applies_bootstrap_reward_delta_to_the_aggregate, delegation apply_bootstrap_reward_deltas_sums_and_fails_closed_on_overflow, epoch_candidate seed2_window_without_rupd_fails_closed / seed2_window_with_wrong_epoch_rupd_fails_closed / seed2_window_with_correct_rupd_derives / non_seed2_window_without_rupd_is_a_strict_noop, epoch_candidate seed2_window_with_a_native_seed_authority_is_a_typed_terminal, bootstrap_reward_update a_v3_record_is_refused_not_reinterpreted (4) Corrects this invariant's previous evidence record at the same revision: the earlier record's test matching missed references written as '<crate> tests/<file>.rs::<name>' or '<crate>::<path>'; the test outcomes are the same run's (docs/evidence/idse-m2-reevidence.json). |

_critical · transactional · scope: first post-bootstrap epoch boundary · assumptions: A2 · depends on: VAL-020 · secondary: CON._ Legacy: DC-EPOCH-41, DC-EPOCH-23, DC-EPOCH-18

#### `VAL-030` — _partial_

| Aspect | Location |
|--------|----------|
| **Source** | `docs/planning/epoch.define.consensus.md#batch-1` (approved by wdm33, 2026-11-08) |
| **Requirement** | **VoteDelegationLifecycle** — Vote delegations follow the protocol-version-10 lifecycle: unregistering a stake account removes its vote delegation, unregistering a DRep removes every delegation to it, a delegation to an unregistered credential DRep is an invalid certificate, and a dangling vote delegation found in authoritative state is a typed terminal, never filtered out. |
| **Code** | `crates/ade_ledger/src/governance/frozen.rs`; `crates/ade_ledger/src/governance.rs`; `crates/ade_ledger/src/rules.rs`; `crates/ade_ledger/src/state.rs`; `crates/ade_ledger/src/gov_cert.rs`; `crates/ade_ledger/src/snapshot/gov_state.rs`; `crates/ade_ledger/src/ledgerdb_state.rs`; `crates/ade_runtime/src/mithril_native_assembly.rs`; `ci/ci_check_frozen_ratification_authority.sh` |
| **Tests** | `ce_lv15b0_1_late_votes_do_not_enact_at_the_consuming_boundary`, `ce_lv15b0_1_a_vote_cast_during_epoch_e_enacts_at_e_plus_2`, `ce_lv15b0_2_late_proposal_is_carried`, `ce_lv15b0_3a_account_unregistration_removes_vote_delegation_and_reregistration_starts_empty`, `ce_lv15b0_3a_drep_reregistration_does_not_regain_old_delegators`, `ce_lv15b0_3b_vote_delegation_to_unregistered_drep_is_rejected`, `ce_lv15b0_3b_drep_registration_then_delegation_is_ordered`, `ce_lv15b0_3b_dangling_delegation_is_a_terminal_not_excluded`, `ce_lv15b0_drep_distribution_counts_every_term_for_every_delegator`, `ce_lv15b0_4_pool_distribution_is_mark_plus_deposits`, `ce_lv15b0_3_4_boundary_evidence_is_post_boundary_accounts_and_the_new_mark`, `ce_lv15b0_5_7_changes_after_the_freeze_do_not_change_the_outcome`, `ce_lv15b0_6_application_prunes_late_siblings_and_routes_by_registration_now`, `ce_lv15b0_6_unenactable_halts_at_the_consuming_boundary`, `ce_lv15b0_8_import_supported_enactment_is_decided_and_cross_checked`, `ce_lv15b0_8_import_unsupported_enactments_are_unenactable`, `lv15b0_expired_proposal_removes_its_descendants`, `lv15b0_expired_sibling_is_removed_as_expired_not_pruned`, `lv15b0_boundary_refuses_a_state_without_a_record`, `lv15b0_boundary_refuses_a_record_from_another_epoch`, `record_from_another_epoch_is_refused`, `lv15b0_v4_record_round_trips_byte_exact`, `lv15b0_v3_bytes_decode_unversioned`, `lv15b0_evidence_only_change_is_visible`, `lv15b0_record_decoder_refuses_non_ascending_keys`, `codec_rejects_pre_lv15b0_store_rebootstrap_required`, `lv15b0_drep_pulser_is_imported_as_the_snapshot_epoch_record`, `lv15b0_pulser_parameters_mismatch_is_refused`, `lv15b0_pulser_root_mismatch_is_refused`, `lv15b0_pulser_delayed_without_unenactable_action_is_refused`, `lv15b0_malformed_pulser_is_refused`, `lv15b0_dangling_vote_delegation_in_a_snapshot_is_refused`, `protocol_major_below_10_is_refused` |
| **CI** | `ci/ci_check_frozen_ratification_authority.sh`, `ci/ci_check_store_semantics_lock.sh` |
| **Claim** | partial — `e563ad94` (pass) — observed: tests: 33 passed, 0 failed, 0 ignored, 0 not found; CI scripts: 2 passed, 0 failed — limits: (1) No IDSE cluster has declared acceptance obligations for this invariant yet (Doctrine §10); this record re-runs the checks carried from the legacy registry. Further limits: (2) No mutation witness exists yet for this critical invariant (Doctrine §28). |

_critical · transactional · scope: Conway vote delegation · assumptions: A2 · depends on: VAL-011, VAL-021._ Legacy: DC-EPOCH-47

### VAL — consensus

#### `VAL-031` — _partial_

| Aspect | Location |
|--------|----------|
| **Source** | `docs/planning/consensus.define.consensus.md#batch-1` (approved by wdm33, 2026-11-08) |
| **Requirement** | **ChainSelectionMatchesReference** — Given the same candidate chains and consensus state, Ade selects the same chain as the reference node's Praos chain selection: the higher block number wins, an equal block number is decided by the reference Praos tiebreak over slot, issuer, operational-certificate issue number and VRF output, density is never used for caught-up selection, and the decision depends on nothing but those canonical observables and the current selection. |
| **Code** | `crates/ade_core/src/consensus/fork_choice.rs`; `crates/ade_core/src/consensus/candidate.rs`; `crates/ade_core/src/consensus/rollback.rs`; `crates/ade_runtime/src/consensus/chain_selector.rs`; `crates/ade_core_interop/src/lib.rs`; `crates/ade_core_interop/src/bin/live_consensus_session.rs`; `crates/ade_runtime/src/consensus/candidate_fragment.rs` |
| **Tests** | `replay_is_deterministic`, `reject_reason_bytes_are_stable`, `higher_block_no_wins`, `equal_block_no_tiebreaker_decides`, `select_best_chain_arrival_order_independent_distinct_heights`, `select_best_chain_arrival_order_independent_tiebreaker`, `fork_before_immutable_tip_rejected`, `exceeded_rollback_rejected`, `tiebreaker_loss_keeps_current`, `no_candidates_returns_error`, `consensus::fork_choice::tests::tiebreaker_prefer_lower_slot_wins`, `consensus::fork_choice::tests::tiebreaker_prefer_higher_op_cert_wins_on_equal_slot_and_issuer`, `consensus::fork_choice::tests::tiebreaker_prefer_lower_vrf_value_wins_on_full_tie`, `rollback_within_k_succeeds`, `rollback_exceeding_k_rejected_with_typed_reason`, `rollback_before_immutable_tip_rejected`, `rollback_event_bytes_are_stable`, `rollback_is_deterministic`, `rollback_equivalent_to_truncated_replay`, `consensus::chain_selector::tests::header_arrival_updates_state_and_selector`, `consensus::chain_selector::tests::rollback_walks_back_via_recent_snapshots`, `consensus::chain_selector::tests::rollback_to_block_older_than_snapshots_rejected`, `consensus::chain_selector::tests::epoch_boundary_emits_no_event`, `synthetic_session_replays_identically`, `synthetic_session_event_kinds_match_corpus`, `synthetic_session_final_state_matches_corpus`, `op_cert_regression_header_rejected_mid_stream`, `cardano_node_session_sustained_window` (#[ignore]), `consensus::fork_choice::tests::no_candidates_returns_no_candidates_error`, `consensus::fork_choice::tests::equal_to_current_keeps_current_via_tiebreaker_loss`, `consensus::candidate::tests::tiebreaker_view_eq_is_field_wise`, `consensus::candidate::tests::candidate_fragment_carries_anchor_block_no` |
| **CI** | `ci/ci_check_no_density_in_fork_choice.sh`, `ci/ci_check_chain_selection_arrival_order_independent.sh`, `ci/ci_check_consensus_closed_enums.sh`, `ci/ci_check_no_chaindb_in_consensus_blue.sh`, `ci/ci_check_no_float_in_consensus.sh` |
| **Claim** | partial — `e563ad94` (pass) — observed: tests: 31 passed, 0 failed, 1 ignored, 0 not found; CI scripts: 5 passed, 0 failed — limits: (1) No IDSE cluster has declared acceptance obligations for this invariant yet (Doctrine §10); this record re-runs the checks carried from the legacy registry. Further limits: (2) No mutation witness exists yet for this critical invariant (Doctrine §28). (3) 1 named test(s) are #[ignore]d and did not run. |

_critical · local · scope: chain selection · assumptions: A2 · depends on: STR-001, STR-002 · secondary: ORD._ Legacy: T-CONS-01, CN-CONS-01, DC-CONSENSUS-01, DC-CONS-03

#### `VAL-032` — _partial_

| Aspect | Location |
|--------|----------|
| **Source** | `docs/planning/consensus.define.consensus.md#batch-1` (approved by wdm33, 2026-11-08) |
| **Requirement** | **HeaderBindsBodyAndContext** — A block is accepted only if its header binds exactly its body and its chain position: the header's body hash equals the hash of the body bytes as received, and its previous-block hash, block number and slot agree with the chain point and consensus state it is validated against. |
| **Code** | `crates/ade_core/src/consensus/header_validate.rs`; `crates/ade_core/src/consensus/header_summary.rs`; `crates/ade_core/src/consensus/fork_choice.rs`; `crates/ade_core/src/consensus/kes_check.rs`; `crates/ade_ledger/src/block_validity/transition.rs`; `crates/ade_ledger/src/block_validity/header_input.rs` |
| **Tests** | `consensus::header_validate::tests::pipeline_short_circuits_on_first_failure`, `consensus::header_validate::tests::nonce_contribution_uses_nonce_role_vrf_output_not_leader_role`, `valid_header_accepted_advances_state`, `header_with_slot_regression_rejected`, `header_with_block_no_regression_rejected`, `header_with_op_cert_regression_rejected`, `header_with_invalid_vrf_proof_rejected`, `header_beyond_forecast_horizon_rejected`, `validate_replay_is_deterministic`, `consensus::candidate::tests::candidate_fragment_carries_anchor_block_no`, `conway_corpus_headers_all_pass_leader_check`, `praos_vrf_keyhash_mismatch_rejected`, `altered_body_rejected_by_hash_binding` |
| **CI** | `ci/ci_check_header_body_binding.sh` |
| **Claim** | partial — `e563ad94` (pass) — observed: tests: 13 passed, 0 failed, 0 ignored, 0 not found; CI scripts: 1 passed, 0 failed — limits: (1) No IDSE cluster has declared acceptance obligations for this invariant yet (Doctrine §10); this record re-runs the checks carried from the legacy registry. Further limits: (2) No mutation witness exists yet for this critical invariant (Doctrine §28). |

_critical · local · scope: header validation · assumptions: A2 · depends on: DET-002, VAL-019._ Legacy: CN-CONS-04

#### `VAL-033` — _partial_

| Aspect | Location |
|--------|----------|
| **Source** | `docs/planning/consensus.define.consensus.md#batch-1` (approved by wdm33, 2026-11-08) |
| **Requirement** | **ForecastRangeEnforced** — A consensus query for a slot beyond the ledger view's safe forecast range returns a typed out-of-range result, never a guessed value; the bound is derived from the era history, the safe zone and the era schedule, never written as a constant by the caller. |
| **Code** | `crates/ade_core/src/consensus/era_schedule.rs`; `crates/ade_core/src/consensus/leader_schedule.rs` |
| **Tests** | `consensus::era_schedule::tests::forecast_horizon_boundary`, `mainnet_corpus_translation_matches_oracle`, `preprod_corpus_translation_matches_oracle`, `consensus::leader_schedule::tests::query_returns_outside_forecast_range_for_far_future`, `corpus_rejects_out_of_forecast_horizon` |
| **CI** | `ci/ci_check_consensus_closed_enums.sh` |
| **Claim** | partial — `e563ad94` (pass) — observed: tests: 5 passed, 0 failed, 0 ignored, 0 not found; CI scripts: 1 passed, 0 failed — limits: (1) No IDSE cluster has declared acceptance obligations for this invariant yet (Doctrine §10); this record re-runs the checks carried from the legacy registry. Further limits: (2) No mutation witness exists yet for this critical invariant (Doctrine §28). |

_critical · local · scope: consensus forecasting · assumptions: A2 · depends on: STR-009 · secondary: SAF._ Legacy: DC-CONS-09

### VAL — consensus-inputs

#### `VAL-034` — _partial_

| Aspect | Location |
|--------|----------|
| **Source** | `docs/planning/consensus-inputs.define.consensus.md#batch-1` (approved by wdm33, 2026-11-08) |
| **Requirement** | **ClosedConsensusInputImport** — An externally supplied consensus-input bundle enters Ade through one importer, which produces a closed canonical record holding every required field (network magic, genesis hash, era, epoch and its slot window, active-slot coefficient, epoch nonce, the pool distribution with each pool's VRF key hash, the protocol-parameter hash and the bundle's source provenance) and a fingerprint that is blake2b-256 over the record's canonical CBOR encoding, fields in their declared order; a missing or malformed field is a typed import error, never given a default, and the raw bundle never reaches the core. |
| **Code** | `crates/ade_runtime/src/consensus_inputs/importer.rs`; `crates/ade_runtime/src/consensus_inputs/canonical.rs`; `crates/ade_runtime/src/consensus_inputs/json.rs` |
| **Tests** | `consensus_inputs::importer::tests::minimal_round_trip_imports_to_typed`, `consensus_inputs::importer::tests::unsupported_era_fails_fast`, `consensus_inputs::importer::tests::import_is_deterministic_across_repeated_calls`, `consensus_inputs::canonical::tests::import_round_trip_yields_canonical_form_with_fingerprint`, `consensus_inputs::canonical::tests::fingerprint_is_deterministic_across_repeated_imports`, `consensus_inputs::importer::tests::empty_era_string_fails_fast`, `consensus_inputs::importer::tests::epoch_end_before_start_is_bad_window`, `consensus_inputs::importer::tests::tip_outside_window_is_bad_window`, `consensus_inputs::importer::tests::zero_asc_denom_is_bad_field`, `consensus_inputs::importer::tests::short_genesis_hash_is_bad_hash_hex`, `consensus_inputs::importer::tests::non_hex_in_hash_is_bad_hash_hex`, `consensus_inputs::importer::tests::pool_in_distribution_missing_from_vrf_map_is_bad_pool`, `consensus_inputs::importer::tests::pool_id_wrong_width_is_bad_hash_hex`, `consensus_inputs::importer::tests::bad_json_surface_is_json_variant`, `consensus_inputs::json::tests::missing_required_field_is_error`, `consensus_inputs::json::tests::unknown_field_is_rejected`, `consensus_inputs::canonical::tests::fingerprint_changes_when_any_canonical_input_changes`, `consensus_inputs::canonical::tests::canonical_field_count_is_fifteen`, `consensus_inputs::canonical::tests::fingerprint_is_blake2b_256_of_encode_canonical_cbor` |
| **CI** | `ci/ci_check_live_consensus_inputs_closure.sh`, `ci/ci_check_live_consensus_inputs_fingerprint.sh` |
| **Claim** | partial — `e563ad94` (pass) — observed: tests: 19 passed, 0 failed, 0 ignored, 0 not found; CI scripts: 2 passed, 0 failed — limits: (1) No IDSE cluster has declared acceptance obligations for this invariant yet (Doctrine §10); this record re-runs the checks carried from the legacy registry. Further limits: (2) No mutation witness exists yet for this critical invariant (Doctrine §28). |

_critical · local · scope: consensus-input import (shell), canonical record · assumptions: A2 · depends on: STR-002, VAL-001 · secondary: DET._ Legacy: CN-CONS-IN-01, DC-CONS-IN-01, DC-CONS-IN-02

#### `VAL-035` — _partial_

| Aspect | Location |
|--------|----------|
| **Source** | `docs/planning/consensus-inputs.define.consensus.md#batch-1` (approved by wdm33, 2026-11-08) |
| **Requirement** | **ImportedViewEpochGuard** — A ledger view built from imported consensus inputs answers only for its own epoch: a query for any other epoch returns no answer, and a block whose slot lies outside the record's epoch window halts admission instead of being judged with that view. |
| **Code** | `crates/ade_runtime/src/consensus_inputs/view.rs`; `crates/ade_node/src/admission/runner.rs` |
| **Tests** | `consensus_inputs::view::tests::out_of_window_epoch_returns_none`, `consensus_inputs::view::tests::in_window_epoch_answers_total_active_stake`, `consensus_inputs::view::tests::in_window_per_pool_lookups_return_imported_values`, `consensus_inputs::view::tests::in_window_unknown_pool_returns_none`, `admission::bootstrap::tests::imported_window_schedule_uses_bundle_epoch` |
| **CI** | `ci/ci_check_live_ledger_view_epoch_window.sh` |
| **Claim** | partial — `e563ad94` (pass) — observed: tests: 5 passed, 0 failed, 0 ignored, 0 not found; CI scripts: 1 passed, 0 failed — limits: (1) No IDSE cluster has declared acceptance obligations for this invariant yet (Doctrine §10); this record re-runs the checks carried from the legacy registry. Further limits: (2) No mutation witness exists yet for this critical invariant (Doctrine §28). |

_critical · local · scope: imported ledger view, admission · assumptions: — · depends on: VAL-034, VAL-033 · secondary: SAF._ Legacy: DC-VIEW-01

#### `VAL-036` — _partial_

| Aspect | Location |
|--------|----------|
| **Source** | `docs/planning/consensus-inputs.define.consensus.md#batch-1` (approved by wdm33, 2026-11-08) |
| **Requirement** | **VrfInputFromEpochNonce** — The Praos VRF input for a slot is blake2b-256 of the slot as 8 big-endian bytes followed by the epoch nonce of that slot's epoch, as the reference mkInputVRF defines it, for leader checks and for forged headers alike, using the IETF draft-03 VRF with 80-byte proofs. |
| **Code** | `crates/ade_core/src/consensus/vrf_cert.rs`; `crates/ade_runtime/src/producer/signing.rs`; `crates/ade_ledger/src/seed_consensus_inputs.rs`; `crates/ade_runtime/src/bootstrap.rs` |
| **Tests** | `warm_start_overlays_recovered_eta0_onto_chain_dep_g_n`, `pinning_praos_vrf_input_and_threshold_match_fixture`, `recovered_surface_projects_pooldistrview_and_expected_vrf_input` |
| **CI** | `ci/ci_check_warmstart_eta0_overlay.sh` (red at e563ad94) |
| **Claim** | unresolved — `e563ad94` (fail) — observed: tests: 3 passed, 0 failed, 0 ignored, 0 not found; CI scripts: 0 passed, 1 failed; failing CI: ci/ci_check_warmstart_eta0_overlay.sh — limits: (1) No IDSE cluster has declared acceptance obligations for this invariant yet (Doctrine §10); this record re-runs the checks carried from the legacy registry. Further limits: (2) No mutation witness exists yet for this critical invariant (Doctrine §28). (3) ci/ci_check_warmstart_eta0_overlay.sh fails at this revision and already failed before the IDSE migration (a3cef7f7): "FAIL: (B) SEED_CINPUT_SCHEMA_VERSION not bumped to 5 (versioned schema change)". Diagnosed as a stale or over-broad structural gate, not a demonstrated violation; the gate needs repair before it can serve as evidence. |

_critical · local · scope: leader checks, header VRF · assumptions: A2 · depends on: VAL-022, SEC-004._ Legacy: DC-CINPUT-03

#### `VAL-037` — _partial_

| Aspect | Location |
|--------|----------|
| **Source** | `docs/planning/consensus-inputs.define.consensus.md#batch-1` (approved by wdm33, 2026-11-08) |
| **Requirement** | **StakeReferenceClassification** — Each output's stake reference is classified by one function from its address bytes and the era bound to the block being processed, never from configuration, the clock or a caller's flag, into a closed result: a base stake credential, valid only after the address's full 57-byte form is validated; a valid address that contributes no stake (a pointer address from Conway on, decoded exactly as the reference ledger decodes it for that era; an enterprise or Byron address); or a rejection (a malformed address, or a reward address used as an output address). Rejection is never folded into contributing no stake. |
| **Code** | `crates/ade_ledger/src/stake_ref.rs`; `ci/ci_check_eview_stake_ref_classification.sh`; `crates/ade_codec/src/address/pointer.rs`; `crates/ade_ledger/src/pointer_resolve.rs`; `ci/ci_check_eview_pointer_compat.sh` |
| **Tests** | `base_type0_is_stake_key_hash`, `base_type1_is_stake_key_hash`, `base_type2_is_stake_script_hash`, `base_type3_is_stake_script_hash`, `pointer_is_decoded_pre_conway_and_retired_at_conway`, `pointer_multibyte_varint_pre_conway`, `pointer_result_exposes_no_credential`, `enterprise_and_byron_are_null_all_eras`, `reward_address_is_rejected_not_summed`, `empty_is_reject_not_null`, `unknown_type_is_reject`, `malformed_but_prefix_valid_base_is_reject_not_null`, `pointer_truncated_varint_is_reject_pre_conway`, `pointer_trailing_bytes_is_reject_pre_conway`, `pointer_overflow_varint_is_reject_pre_conway`, `pointer_too_short_is_reject_pre_conway`, `real_preview_addresses_classify_without_reject`, `classification_is_deterministic`, `bounded_leading_zero_alias_accepted_all_eras`, `conway_decodes_in_range`, `conway_rejects_txix_over_u16`, `conway_rejects_width_overflow_within_max_groups`, `conway_rejects_slot_over_u32`, `conway_rejects_trailing_bytes`, `conway_accepts_max_width_boundary`, `babbage_normalizes_overflow_to_zero_tuple`, `babbage_in_range_kept_unmodified`, `babbage_rejects_trailing_bytes`, `alonzo_normalizes_overflow_to_zero_tuple`, `alonzo_crops_trailing_bytes`, `truncated_varint_is_error`, `decode_pointer_address_validates_header_and_tail`, `decode_is_deterministic`, `resolves_a_registered_pointer`, `unregistered_pointer_is_none_fail_closed`, `duplicate_position_is_rejected_fail_closed`, `distinct_coordinates_resolve_independently`, `resolution_is_deterministic` |
| **CI** | `ci/ci_check_eview_stake_ref_classification.sh`, `ci/ci_check_eview_pointer_compat.sh` |
| **Claim** | partial — `e563ad94` (pass) — observed: tests: 38 passed, 0 failed, 0 ignored, 0 not found; CI scripts: 2 passed, 0 failed — limits: (1) No IDSE cluster has declared acceptance obligations for this invariant yet (Doctrine §10); this record re-runs the checks carried from the legacy registry. Further limits: (2) No mutation witness exists yet for this critical invariant (Doctrine §28). |

_critical · local · scope: stake attribution of outputs · assumptions: A2 · depends on: VAL-001, STR-005._ Legacy: DC-EVIEW-02, DC-EVIEW-03

#### `VAL-038` — _partial_

| Aspect | Location |
|--------|----------|
| **Source** | `docs/planning/consensus-inputs.define.consensus.md#batch-1` (approved by wdm33, 2026-11-08) |
| **Requirement** | **PoolLifecycleMatchesReference** — The pool lifecycle matches the reference ledger: a first registration takes effect immediately; a re-registration of a registered pool is staged as a future registration, leaving the active parameters and VRF key unchanged until it is adopted at the next boundary, and cancels any pending retirement; at every boundary crossed, including each boundary inside a replayed range, the stake snapshot is taken first (VAL-028), then pool reaping adopts staged registrations (dropping any whose pool is no longer active), reaps exactly the pools retiring at that epoch, and removes delegations to the reaped pools while keeping those credentials' registrations and reward accounts. |
| **Code** | `crates/ade_ledger/src/delegation.rs`; `crates/ade_runtime/src/chaindb/reduced_window_driver.rs`; `crates/ade_ledger/src/snapshot/cert_state.rs`; `ci/ci_check_eview_pool_lifecycle.sh` |
| **Tests** | `re_registration_keeps_old_vrf_until_reap`, `pool_re_registration_stages_params_adopted_at_reap`, `reaped_pool_delegation_cleared_no_silent_reattach_on_reregistration`, `pool_reap_reaps_matching_epoch_only`, `drive_boundary_adopts_futures_reaps_retiring_clears_delegations`, `drive_boundary_is_deterministic`, `cert_state_round_trip_populated` |
| **CI** | `ci/ci_check_eview_pool_lifecycle.sh` (red at e563ad94) |
| **Claim** | unresolved — `e563ad94` (fail) — observed: tests: 7 passed, 0 failed, 0 ignored, 0 not found; CI scripts: 0 passed, 1 failed; failing CI: ci/ci_check_eview_pool_lifecycle.sh — limits: (1) No IDSE cluster has declared acceptance obligations for this invariant yet (Doctrine §10); this record re-runs the checks carried from the legacy registry. Further limits: (2) No mutation witness exists yet for this critical invariant (Doctrine §28). (3) ci/ci_check_eview_pool_lifecycle.sh fails at this revision and already failed before the IDSE migration (a3cef7f7): "FAIL: the window driver does not apply POOLREAP at crossed boundaries". Diagnosed as a stale or over-broad structural gate, not a demonstrated violation; the gate needs repair before it can serve as evidence. |

_critical · transactional · scope: pool registration, retirement and reaping · assumptions: A2 · depends on: VAL-011, VAL-020, VAL-028._ Legacy: DC-EVIEW-13

### VAL — durability-recovery

#### `VAL-039` — _partial_

| Aspect | Location |
|--------|----------|
| **Source** | `docs/planning/durability-recovery.define.consensus.md#batch-1` (approved by wdm33, 2026-11-08) |
| **Requirement** | **ClosedGenesisParser** — The Shelley genesis file is read by one closed parser that requires the network magic, system start, slot length, slots per KES period and maximum KES evolutions with exact types, never assumes a default or network, ignores unknown keys only when they change nothing, converts the system start deterministically with no time library, and produces the same genesis anchor from the same file bytes and supplied KES anchor slot every time. |
| **Code** | `crates/ade_runtime/src/producer/genesis_parser.rs` |
| **Tests** | `accepted_shelley_genesis_parses_to_expected_anchor`, `missing_required_field_emits_structured_error`, `stringly_int_emits_malformed_field_type`, `extra_inert_keys_produce_byte_identical_anchor`, `malformed_numeric_negative_slot_length_rejected`, `iso8601_parse_anchors_to_known_unix_ms_values`, `parser_is_byte_identical_across_two_runs` |
| **CI** | `ci/ci_check_node_forge_real_cli_ingress.sh`, `ci/ci_check_genesis_consistency_fixture_present.sh` |
| **Claim** | partial — `e563ad94` (pass) — observed: tests: 7 passed, 0 failed, 0 ignored, 0 not found; CI scripts: 2 passed, 0 failed — limits: (1) No IDSE cluster has declared acceptance obligations for this invariant yet (Doctrine §10); this record re-runs the checks carried from the legacy registry. Further limits: (2) No mutation witness exists yet for this critical invariant (Doctrine §28). |

_critical · local · scope: genesis parsing (shell) · assumptions: — · depends on: VAL-001, STR-009 · secondary: DET._ Legacy: CN-GENESIS-01, DC-GENESIS-01

#### `VAL-040` — _declared_

| Aspect | Location |
|--------|----------|
| **Source** | `docs/planning/durability-recovery.define.consensus.md#batch-1` (approved by wdm33, 2026-11-08) |
| **Requirement** | **RecoveryRevalidatesStoredBlocks** — After an unclean shutdown, every stored block that recovery applies passes through the same header and body validation authorities as live admission; no stored verdict or validity flag stands in for validation. |
| **Code** | _(no enforcement found — gap)_ |
| **Tests** | _(no tests found — gap)_ |
| **CI** | _(no CI enforcement — gap)_ |
| **Claim** | unresolved — no evidence record |

_critical · local · scope: recovery after unclean shutdown · assumptions: — · depends on: VAL-009, STR-004 · secondary: REC._ Legacy: DC-STORE-06, CN-STORE-06

#### `VAL-041` — _partial_

| Aspect | Location |
|--------|----------|
| **Source** | `docs/planning/durability-recovery.define.consensus.md#batch-1` (approved by wdm33, 2026-11-08) |
| **Requirement** | **WarmStartNonceFromSeedRecord** — At warm start, the epoch nonce in the recovered chain-dependent state for the seed epoch is the nonce held in the recovered seed record, applied by the one bootstrap authority before any forging or self-acceptance; a snapshot placeholder nonce or a genesis-derived nonce never reaches VRF verification or forging. |
| **Code** | `crates/ade_ledger/src/seed_consensus_inputs.rs`; `crates/ade_runtime/src/seed_consensus_merge.rs`; `crates/ade_runtime/src/bootstrap.rs` |
| **Tests** | `warm_start_overlays_recovered_eta0_onto_chain_dep_g_n`, `warm_start_restores_seed_epoch_consensus_inputs_byte_identical`, `seed_cinput_decode_rejects_unknown_version`, `seed_epoch_consensus_inputs_round_trips_byte_identical`, `pinning_preseed_warmstart_roundtrip_faithful` |
| **CI** | `ci/ci_check_warmstart_eta0_overlay.sh` (red at e563ad94) |
| **Claim** | unresolved — `e563ad94` (fail) — observed: tests: 5 passed, 0 failed, 0 ignored, 0 not found; CI scripts: 0 passed, 1 failed; failing CI: ci/ci_check_warmstart_eta0_overlay.sh — limits: (1) No IDSE cluster has declared acceptance obligations for this invariant yet (Doctrine §10); this record re-runs the checks carried from the legacy registry. Further limits: (2) No mutation witness exists yet for this critical invariant (Doctrine §28). (3) ci/ci_check_warmstart_eta0_overlay.sh fails at this revision and already failed before the IDSE migration (a3cef7f7): "FAIL: (B) SEED_CINPUT_SCHEMA_VERSION not bumped to 5 (versioned schema change)". Diagnosed as a stale or over-broad structural gate, not a demonstrated violation; the gate needs repair before it can serve as evidence. |

_critical · local · scope: warm start, chain-dependent state · assumptions: A2 · depends on: DUR-004, STR-010, VAL-022._ Legacy: T-REC-04

### VAL — network

#### `VAL-042` — _partial_

| Aspect | Location |
|--------|----------|
| **Source** | `docs/planning/network.define.consensus.md#batch-1` (approved by wdm33, 2026-11-08) |
| **Requirement** | **MiniProtocolTranscriptMatchesReference** — For the same inputs and peer messages, each mini-protocol's messages and state progression match the reference node's for the same negotiated version, checked against transcripts captured from a reference node. |
| **Code** | `crates/ade_network/src/chain_sync/state.rs`; `crates/ade_network/src/chain_sync/agency.rs`; `crates/ade_network/src/chain_sync/signal.rs`; `crates/ade_network/src/chain_sync/transition.rs`; `crates/ade_network/src/block_fetch/state.rs`; `crates/ade_network/src/block_fetch/agency.rs`; `crates/ade_network/src/block_fetch/event.rs`; `crates/ade_network/src/block_fetch/transition.rs`; `crates/ade_network/src/tx_submission/state.rs`; `crates/ade_network/src/tx_submission/agency.rs`; `crates/ade_network/src/tx_submission/event.rs`; `crates/ade_network/src/tx_submission/transition.rs` |
| **Tests** | `chain_sync::transition::tests::idle_request_next_with_immediate_data_yields_can_await_then_roll_forward`, `chain_sync::transition::tests::idle_request_next_with_no_data_yields_must_reply_via_await`, `chain_sync::transition::tests::roll_forward_signal_carries_header_and_tip_byte_identical`, `chain_sync::transition::tests::roll_backward_signal_carries_point_and_tip_byte_identical`, `chain_sync::transition::tests::find_intersect_with_known_point_yields_intersected_signal`, `chain_sync::transition::tests::find_intersect_with_unknown_points_yields_no_intersection`, `chain_sync::transition::tests::client_done_terminates_session`, `chain_sync::transition::tests::illegal_message_in_idle_returns_error`, `chain_sync::transition::tests::wrong_agency_returns_error`, `chain_sync::transition::tests::version_gating_rejects_out_of_version_message`, `chain_sync::transition::tests::empty_find_intersect_points_returns_malformed`, `signal_trace_synthetic_divergence_corpus`, `block_fetch::transition::tests::idle_request_range_yields_busy_then_start_batch`, `block_fetch::transition::tests::busy_no_blocks_returns_to_idle_with_event`, `block_fetch::transition::tests::streaming_block_delivers_bytes_byte_identical`, `block_fetch::transition::tests::streaming_batch_done_returns_to_idle`, `block_fetch::transition::tests::multi_block_streaming_preserves_order`, `block_fetch::transition::tests::client_done_terminates_session`, `block_fetch::transition::tests::illegal_message_in_idle_returns_error`, `block_fetch::transition::tests::wrong_agency_returns_error`, `block_fetch::transition::tests::version_gating_rejects_out_of_version_message`, `block_fetch::transition::tests::inverted_range_returns_malformed`, `block_fetch::transition::tests::block_delivered_event_carries_bytes_verbatim`, `block_fetch_frame_corpus`, `tx_submission::transition::tests::init_handshake_opens_session`, `tx_submission::transition::tests::request_blocking_ids_then_reply_round_trips`, `tx_submission::transition::tests::request_non_blocking_ids_with_empty_reply_round_trips`, `tx_submission::transition::tests::request_txs_then_reply_delivers_bytes_byte_identical`, `tx_submission::transition::tests::request_txs_with_partial_reply_legal`, `tx_submission::transition::tests::blocking_reply_empty_is_malformed`, `tx_submission::transition::tests::reply_ids_overfill_is_malformed`, `tx_submission::transition::tests::request_txs_empty_is_malformed`, `tx_submission::transition::tests::reply_txs_overfill_is_malformed`, `tx_submission::transition::tests::done_terminates_session`, `tx_submission::transition::tests::illegal_message_in_init_returns_error`, `tx_submission::transition::tests::wrong_agency_returns_error`, `tx_submission::transition::tests::version_gating_rejects_out_of_version_message`, `tx_submission2_mempool_trace`, `chain_sync_real_capture_corpus::real_capture_round_trips_byte_identical`, `chain_sync_real_capture_corpus::rollforward_header_opaque_pass_through`, `block_fetch_real_capture_corpus::real_capture_round_trips_byte_identical`, `block_fetch_real_capture_corpus::block_body_is_wrapped_cbor_item`, `keep_alive_real_capture_corpus::real_capture_round_trips_byte_identical`, `keep_alive_real_capture_corpus::cookie_echo_invariant_holds`, `peer_sharing_real_capture_corpus::real_capture_round_trips_byte_identical`, `tx_submission2_real_capture_corpus::real_capture_round_trips_byte_identical`, `tx_submission2_real_capture_corpus::reply_txids_entries_are_real_32_byte_txids`, `codec::tx_submission::tests::real_cardano_reply_txids_decodes_and_re_encodes_byte_identical` |
| **CI** | `ci/ci_check_tx_submission2_real_capture.sh` |
| **Claim** | partial — `e563ad94` (pass) — observed: tests: 48 passed, 0 failed, 0 ignored, 0 not found; CI scripts: 1 passed, 0 failed — limits: (1) No IDSE cluster has declared acceptance obligations for this invariant yet (Doctrine §10); this record re-runs the checks carried from the legacy registry. Further limits: (2) No mutation witness exists yet for this critical invariant (Doctrine §28). |

_critical · distributed · scope: N2N and N2C mini-protocols · assumptions: A2 · depends on: DET-016, PRC-003._ Legacy: DC-PROTO-02

#### `VAL-043` — _partial_

| Aspect | Location |
|--------|----------|
| **Source** | `docs/planning/network.define.consensus.md#batch-1` (approved by wdm33, 2026-11-08) |
| **Requirement** | **TxSubmissionWireForm** — The tx-submission codec accepts and preserves the reference node's wire form: each transaction id is the era-tagged pair [era index, 32-byte hash], never stripped or guessed, so a requester echoes the exact advertised id; id and transaction sequences are accepted as definite or indefinite-length arrays, a received message re-encodes to exactly its received bytes in whichever form it arrived, and messages Ade originates use the indefinite form the reference node sends. |
| **Code** | `crates/ade_network/src/codec/tx_submission.rs`; `corpus/network/n2n/tx_submission2` |
| **Tests** | `codec::tx_submission::tests::roundtrip_every_variant`, `codec::tx_submission::tests::encoder_emits_indefinite_sequences`, `codec::tx_submission::tests::real_cardano_reply_txids_decodes_and_re_encodes_byte_identical`, `codec::tx_submission::tests::decode_accepts_definite_sequence_form`, `codec::tx_submission::tests::decode_rejects_bare_txid`, `codec::tx_submission::tests::decode_rejects_wrong_txid_hash_length`, `codec::tx_submission::tests::decode_rejects_unterminated_indefinite_sequence`, `tx_submission2_real_capture_corpus::real_capture_round_trips_byte_identical`, `tx_submission2_real_capture_corpus::reply_txids_entries_are_real_32_byte_txids` |
| **CI** | `ci/ci_check_tx_submission2_real_capture.sh` |
| **Claim** | partial — `e563ad94` (pass) — observed: tests: 9 passed, 0 failed, 0 ignored, 0 not found; CI scripts: 1 passed, 0 failed — limits: (1) No IDSE cluster has declared acceptance obligations for this invariant yet (Doctrine §10); this record re-runs the checks carried from the legacy registry. Further limits: (2) No mutation witness exists yet for this critical invariant (Doctrine §28). |

_critical · local · scope: TxSubmission2 codec · assumptions: A2 · depends on: VAL-001, DET-003._ Legacy: DC-PROTO-11

#### `VAL-044` — _partial_

| Aspect | Location |
|--------|----------|
| **Source** | `docs/planning/network.define.consensus.md#batch-1` (approved by wdm33, 2026-11-08); 1 recorded redefinition(s) in the registry |
| **Requirement** | **FullMiniProtocolSurface** — Ade implements the full node-to-node set (Handshake, ChainSync, BlockFetch, TxSubmission2, KeepAlive, PeerSharing) and the full node-to-client set (Handshake, LocalChainSync, LocalTxSubmission, LocalStateQuery, LocalTxMonitor), each in the roles the reference node plays and in agreement with VAL-042. |
| **Code** | `crates/ade_network/src/codec/handshake.rs`; `crates/ade_network/src/codec/chain_sync.rs`; `crates/ade_network/src/codec/block_fetch.rs`; `crates/ade_network/src/codec/tx_submission.rs`; `crates/ade_network/src/codec/keep_alive.rs`; `crates/ade_network/src/codec/peer_sharing.rs`; `crates/ade_network/src/codec/n2c_handshake.rs`; `crates/ade_network/src/codec/local_chain_sync.rs`; `crates/ade_network/src/codec/local_tx_submission.rs`; `crates/ade_network/src/codec/local_state_query.rs`; `crates/ade_network/src/codec/local_tx_monitor.rs`; `crates/ade_network/src/n2c/local_chain_sync/state.rs`; `crates/ade_network/src/n2c/local_chain_sync/agency.rs`; `crates/ade_network/src/n2c/local_chain_sync/event.rs`; `crates/ade_network/src/n2c/local_chain_sync/transition.rs`; `crates/ade_network/src/n2c/local_tx_submission/state.rs`; `crates/ade_network/src/n2c/local_tx_submission/agency.rs`; `crates/ade_network/src/n2c/local_tx_submission/event.rs`; `crates/ade_network/src/n2c/local_tx_submission/transition.rs`; `crates/ade_network/src/n2c/local_state_query/state.rs`; `crates/ade_network/src/n2c/local_state_query/agency.rs`; `crates/ade_network/src/n2c/local_state_query/event.rs`; `crates/ade_network/src/n2c/local_state_query/transition.rs`; `crates/ade_network/src/n2c/local_tx_monitor/state.rs`; `crates/ade_network/src/n2c/local_tx_monitor/agency.rs`; `crates/ade_network/src/n2c/local_tx_monitor/event.rs`; `crates/ade_network/src/n2c/local_tx_monitor/transition.rs` |
| **Tests** | `codec::handshake::tests::roundtrip_every_variant`, `codec::chain_sync::tests::roundtrip_every_variant`, `codec::block_fetch::tests::roundtrip_every_variant`, `codec::tx_submission::tests::roundtrip_every_variant`, `codec::keep_alive::tests::roundtrip_every_variant`, `codec::peer_sharing::tests::roundtrip_every_variant`, `codec::n2c_handshake::tests::roundtrip_every_variant`, `codec::local_chain_sync::tests::roundtrip_every_variant`, `codec::local_tx_submission::tests::roundtrip_every_variant`, `codec::local_state_query::tests::roundtrip_every_variant`, `codec::local_tx_monitor::tests::roundtrip_every_variant`, `n2c::local_chain_sync::transition::tests::local_chain_sync_request_next_then_roll_forward`, `n2c::local_chain_sync::transition::tests::local_chain_sync_roll_backward_signal`, `n2c::local_chain_sync::transition::tests::local_chain_sync_find_intersect_known_point`, `n2c::local_chain_sync::transition::tests::local_chain_sync_find_intersect_unknown`, `n2c::local_chain_sync::transition::tests::local_chain_sync_client_done_terminates`, `n2c::local_chain_sync::transition::tests::local_chain_sync_wrong_agency_returns_error`, `n2c::local_chain_sync::transition::tests::local_chain_sync_version_gating`, `n2c::local_tx_submission::transition::tests::submit_tx_then_accept_round_trips`, `n2c::local_tx_submission::transition::tests::submit_tx_then_reject_carries_reason_bytes`, `n2c::local_tx_submission::transition::tests::client_done_terminates`, `n2c::local_tx_submission::transition::tests::wrong_agency_returns_error`, `n2c::local_tx_submission::transition::tests::version_gating`, `n2c::local_state_query::transition::tests::acquire_then_acquired_transitions_to_acquired`, `n2c::local_state_query::transition::tests::acquire_then_failure_returns_to_idle`, `n2c::local_state_query::transition::tests::query_then_result_round_trips`, `n2c::local_state_query::transition::tests::release_returns_to_idle`, `n2c::local_state_query::transition::tests::re_acquire_from_acquired_transitions_to_acquiring`, `n2c::local_state_query::transition::tests::client_done_from_idle_terminates`, `n2c::local_state_query::transition::tests::client_done_from_acquired_terminates`, `n2c::local_state_query::transition::tests::wrong_agency_returns_error`, `n2c::local_tx_monitor::transition::tests::idle_acquire_then_acquired`, `n2c::local_tx_monitor::transition::tests::acquired_release_returns_to_idle`, `n2c::local_tx_monitor::transition::tests::acquired_await_acquire_re_acquires`, `n2c::local_tx_monitor::transition::tests::acquired_next_tx_then_reply_with_tx`, `n2c::local_tx_monitor::transition::tests::acquired_next_tx_then_reply_empty_mempool`, `n2c::local_tx_monitor::transition::tests::acquired_has_tx_then_reply_present`, `n2c::local_tx_monitor::transition::tests::acquired_has_tx_then_reply_absent`, `n2c::local_tx_monitor::transition::tests::acquired_get_sizes_then_reply`, `n2c::local_tx_monitor::transition::tests::acquired_get_measures_then_reply`, `n2c::local_tx_monitor::transition::tests::get_measures_rejected_below_v2`, `n2c::local_tx_monitor::transition::tests::client_done_terminates_from_idle`, `n2c::local_tx_monitor::transition::tests::wrong_agency_returns_error`, `n2c::local_tx_monitor::transition::tests::version_gating_rejects_out_of_version_message`, `local_chain_sync_event_trace`, `local_tx_submission_event_trace`, `local_state_query_event_trace`, `local_tx_monitor_event_trace` |
| **CI** | `ci/ci_check_mini_protocol_surface.sh` |
| **Claim** | partial — `e563ad94` (pass) — observed: tests: 48 passed, 0 failed, 0 ignored, 0 not found; CI scripts: 1 passed, 0 failed — limits: (1) No IDSE cluster has declared acceptance obligations for this invariant yet (Doctrine §10); this record re-runs the checks carried from the legacy registry. |

_standard · local · scope: N2N and N2C mini-protocol coverage · assumptions: A2 · depends on: VAL-042._ Legacy: DC-PROTO-03, DC-PROTO-04

#### `VAL-045` — _partial_

| Aspect | Location |
|--------|----------|
| **Source** | `docs/planning/network.define.consensus.md#batch-1` (approved by wdm33, 2026-11-08) |
| **Requirement** | **ServedBytesAreAcceptedBytes** — Block bytes served over block-fetch are byte-identical to the bytes of the accepted block they belong to, never re-encoded, and a header announced over chain-sync is the header of that same accepted block, so the body hash of the served body equals the body-hash field of the announced header. |
| **Code** | `crates/ade_network/src/block_fetch/server.rs`; `crates/ade_runtime/src/producer/served_chain_lookups.rs`; `crates/ade_ledger/src/producer/served_chain.rs`; `crates/ade_ledger/src/block_validity/header_input.rs`; `crates/ade_network/src/chain_sync/server.rs` |
| **Tests** | `producer_block_fetch_serve_block_bytes_equal_accepted_block_as_bytes`, `session_transcript_served_block_bytes_equal_admitted_accepted_block_bytes`, `n_r_b_partial_overlap_from_endpoint_not_in_snapshot_yields_no_blocks`, `n_r_b_partial_overlap_to_endpoint_not_in_snapshot_yields_no_blocks`, `n_r_b_partial_overlap_both_endpoints_fabricated_yields_no_blocks`, `producer_chain_sync_serve_roll_forward_header_equals_accepted_block_header_bytes`, `session_transcript_announced_header_matches_served_body_recipe`, `accepted_block_header_bytes_equals_validator_split_on_corpus`, `forge_block_accepts_empty_mempool`, `unsigned_header_preimage_matches_decode_block_extraction_for_corpus` |
| **CI** | `ci/ci_check_block_fetch_server_closure.sh`, `ci/ci_check_broadcast_to_served_purity.sh`, `ci/ci_check_no_parallel_header_splitter.sh`, `ci/ci_check_unsigned_header_preimage_single_source.sh` |
| **Claim** | partial — `e563ad94` (pass) — observed: tests: 10 passed, 0 failed, 0 ignored, 0 not found; CI scripts: 4 passed, 0 failed — limits: (1) No IDSE cluster has declared acceptance obligations for this invariant yet (Doctrine §10); this record re-runs the checks carried from the legacy registry. Further limits: (2) No mutation witness exists yet for this critical invariant (Doctrine §28). |

_critical · local · scope: chain-sync and block-fetch servers · assumptions: — · depends on: DET-002, VAL-032 · secondary: DET._ Legacy: DC-CONS-17, DC-CONS-18

#### `VAL-046` — _partial_

| Aspect | Location |
|--------|----------|
| **Source** | `docs/planning/network.define.consensus.md#batch-1` (approved by wdm33, 2026-11-08) |
| **Requirement** | **BlockFetchRangeAllOrNothing** — A block-fetch range request is answered with a batch only if both endpoints and every block between them are on the served chain; otherwise the reply is NoBlocks, never a partial batch, a truncated range or a strict prefix. |
| **Code** | `crates/ade_network/src/block_fetch/server.rs` |
| **Tests** | `n_r_b_partial_overlap_from_endpoint_not_in_snapshot_yields_no_blocks`, `n_r_b_partial_overlap_to_endpoint_not_in_snapshot_yields_no_blocks`, `n_r_b_partial_overlap_both_endpoints_fabricated_yields_no_blocks`, `producer_block_fetch_serve_request_range_with_origin_endpoint_yields_no_blocks`, `producer_block_fetch_serve_request_range_empty_in_chain_yields_no_blocks`, `producer_block_fetch_serve_request_range_outside_chain_yields_no_blocks` |
| **CI** | _(no CI enforcement — gap)_ |
| **Claim** | partial — `e563ad94` (pass) — observed: tests: 6 passed, 0 failed, 0 ignored, 0 not found; CI scripts: 0 passed, 0 failed — limits: (1) No IDSE cluster has declared acceptance obligations for this invariant yet (Doctrine §10); this record re-runs the checks carried from the legacy registry. Further limits: (2) No mutation witness exists yet for this critical invariant (Doctrine §28). |

_critical · local · scope: block-fetch server · assumptions: A2 · depends on: SAF-010._ Legacy: CN-SNAPSHOT-02

#### `VAL-047` — _partial_

| Aspect | Location |
|--------|----------|
| **Source** | `docs/planning/network.define.consensus.md#batch-1` (approved by wdm33, 2026-11-08); 1 recorded redefinition(s) in the registry |
| **Requirement** | **FindIntersectSetsCursor** — After the chain-sync server answers IntersectFound at a point, its read position is exactly that point: the next RequestNext is answered with the block immediately after it when that block is on the unchanged served chain, and otherwise with the waiting or rollback reply DET-017 allows; an intersection at Origin sets the position before the chain's first block. |
| **Code** | `crates/ade_network/src/chain_sync/server.rs` |
| **Tests** | `producer_chain_sync_serve_find_intersect_sets_cursor_then_rolls_forward_past_it` |
| **CI** | _(no CI enforcement — gap)_ |
| **Claim** | partial — `e563ad94` (pass) — observed: tests: 1 passed, 0 failed, 0 ignored, 0 not found; CI scripts: 0 passed, 0 failed — limits: (1) No IDSE cluster has declared acceptance obligations for this invariant yet (Doctrine §10); this record re-runs the checks carried from the legacy registry. Further limits: (2) No mutation witness exists yet for this critical invariant (Doctrine §28). |

_critical · local · scope: chain-sync server · assumptions: A2 · depends on: VAL-005, DET-017 · secondary: ORD._ Legacy: DC-PROTO-10

#### `VAL-048` — _partial_

| Aspect | Location |
|--------|----------|
| **Source** | `docs/planning/network.define.consensus.md#batch-1` (approved by wdm33, 2026-11-08) |
| **Requirement** | **ClosedMiniProtocolRegistry** — The mini-protocol identifiers the session dispatches on form a closed set; a frame for any other identifier is a peer-fatal error, never ignored or accepted. |
| **Code** | `crates/ade_network/src/session/event.rs`; `crates/ade_network/src/session/core.rs` |
| **Tests** | `crates/ade_network/src/session/event.rs::tests::accepted_mini_protocol_round_trips_all_ids`, `crates/ade_network/src/session/event.rs::tests::accepted_mini_protocol_unknown_id_returns_none`, `crates/ade_network/src/session/event.rs::tests::accepted_mini_protocol_match_is_exhaustive`, `crates/ade_network/src/session/core.rs::tests::session_unknown_mini_protocol_id_is_peer_fatal` |
| **CI** | `ci/ci_check_mini_protocol_id_registry_closed.sh` |
| **Claim** | partial — `e563ad94` (pass) — observed: tests: 4 passed, 0 failed, 0 ignored, 0 not found; CI scripts: 1 passed, 0 failed — limits: (1) No IDSE cluster has declared acceptance obligations for this invariant yet (Doctrine §10); this record re-runs the checks carried from the legacy registry. |

_standard · local · scope: session dispatch · assumptions: A2 · depends on: STR-012 · secondary: EVO._ Legacy: DC-SESS-02

#### `VAL-049` — _partial_

| Aspect | Location |
|--------|----------|
| **Source** | `docs/planning/network.define.consensus.md#batch-1` (approved by wdm33, 2026-11-08) |
| **Requirement** | **OutboundSegmentation** — An outbound mini-protocol payload larger than the mux maximum is split into ordered frames no larger than that maximum, each with the same mini-protocol id and mode, whose payloads concatenate to the original exactly, using the one frame encoder; a payload above the fixed outbound ceiling fails closed. |
| **Code** | `crates/ade_network/src/session/core.rs` |
| **Tests** | `crates/ade_network/src/session/core.rs::tests::outbound_payload_at_max_payload_is_one_frame`, `crates/ade_network/src/session/core.rs::tests::outbound_payload_over_max_payload_segments_into_two`, `crates/ade_network/src/session/core.rs::tests::outbound_segment_order_preserved`, `crates/ade_network/src/session/core.rs::tests::outbound_segments_keep_same_mini_protocol_id_and_mode`, `crates/ade_network/src/session/core.rs::tests::outbound_large_payload_reassembles_byte_identical_via_inbound`, `crates/ade_network/src/session/core.rs::tests::outbound_payload_at_upper_bound_is_allowed`, `crates/ade_network/src/session/core.rs::tests::outbound_payload_over_upper_bound_fails_closed` |
| **CI** | `ci/ci_check_outbound_segmentation.sh` |
| **Claim** | partial — `e563ad94` (pass) — observed: tests: 7 passed, 0 failed, 0 ignored, 0 not found; CI scripts: 1 passed, 0 failed — limits: (1) No IDSE cluster has declared acceptance obligations for this invariant yet (Doctrine §10); this record re-runs the checks carried from the legacy registry. Further limits: (2) No mutation witness exists yet for this critical invariant (Doctrine §28). |

_critical · local · scope: session layer, outbound · assumptions: A2 · depends on: STR-012, DET-018._ Legacy: CN-SESS-05

### VAL — node

#### `VAL-050` — _partial_

| Aspect | Location |
|--------|----------|
| **Source** | `docs/planning/node.define.consensus.md#batch-1` (approved by wdm33, 2026-11-09) |
| **Requirement** | **ServedViewIsDurableProjection** — The chain served to peers (chain-sync headers and block-fetch bodies) is a deterministic projection of the durable adopted chain, received and self-forged blocks alike, so a peer can fetch coherent history; it is served only through the existing chain-sync and block-fetch serve reducers, by a serve task that only reads it and holds no write handle to the chain, log or forge. |
| **Code** | `crates/ade_node/src/node_lifecycle.rs`; `crates/ade_node/src/node_sync.rs`; `crates/ade_runtime/src/network/served_chain_projection.rs`; `crates/ade_runtime/src/network/serve_dispatch.rs`; `crates/ade_runtime/src/producer/served_chain_handle.rs`; `crates/ade_ledger/src/producer/served_chain.rs`; `crates/ade_node/src/produce_mode.rs`; `crates/ade_network/src/chain_sync/server.rs`; `crates/ade_network/src/block_fetch/server.rs`; `crates/ade_ledger/src/block_validity/header_input.rs` |
| **Tests** | `handoff_carrier_constructs_only_from_self_accepted_forge`, `forge_surfaces_accepted_block_only_on_self_accept`, `handoff_carrier_has_no_raw_bytes_constructor`, `serve_ingress_type_rejects_failed_forge_outcome`, `sibling_serve_admits_via_push_atomic_only`, `serve_sibling_admission_replay_byte_identical`, `serve_sibling_push_atomic_fed_only_by_into_accepted`, `relay_loop_containment_semantics_after_serve_sibling_retired`, `block_fetch_payload_is_self_accepted_bytes`, `block_fetch_tag24_round_trips_to_self_accept_input`, `live_feed_forge_serve_loopback_returns_forged_block`, `served_view_projects_durable_chain`, `served_view_retires_accumulator`, `node_serve_start_failure_is_surfaced_not_silent`, `n2n_supported_for_magic_produces_configured_magic`, `node_c1_serve_live`, `follower_fetches_coherent_history_incl_ingested_predecessor` |
| **CI** | `ci/ci_check_served_chain_handoff_fence.sh`, `ci/ci_check_node_run_loop_containment.sh`, `ci/ci_check_served_chain_projection.sh`, `ci/ci_check_single_serve_dispatch_authority.sh`, `ci/ci_check_serve_listener_magic_aware.sh` |
| **Claim** | partial — `e563ad94` (pass) — observed: tests: 17 passed, 0 failed, 0 ignored, 0 not found; CI scripts: 5 passed, 0 failed — limits: (1) No IDSE cluster has declared acceptance obligations for this invariant yet (Doctrine §10); this record re-runs the checks carried from the legacy registry. Further limits: (2) No mutation witness exists yet for this critical invariant (Doctrine §28). |

_critical · local · scope: serving · assumptions: — · depends on: VAL-045, SAF-010 · secondary: STR._ Legacy: DC-NODE-13, DC-NODE-07, DC-NODE-06

#### `VAL-051` — _partial_

| Aspect | Location |
|--------|----------|
| **Source** | `docs/planning/node.define.consensus.md#batch-1` (approved by wdm33, 2026-11-09) |
| **Requirement** | **ForgeSuccessorPosition** — A forged block's previous-block hash is the hash of the forge base and its block number is the forge base's plus one, and the ledger state and chain-dependent state the forge builds on and self-accepts against are the ones the latest admission produced; none of these comes from a stale startup baseline, a guess or a default. |
| **Code** | `crates/ade_node/src/node_lifecycle.rs`; `crates/ade_node/src/node_sync.rs` |
| **Tests** | `forge_successor_reads_evolved_spine_block_no_not_stale_baseline_g_q`, `forge_one_from_recovered_cold_start_is_block_zero_genesis`, `forge_on_followed_tip_proceeds_with_parent_byte_equal` |
| **CI** | `ci/ci_check_forge_successor_evolved_spine.sh`, `ci/ci_check_forge_followed_tip_admission.sh` |
| **Claim** | partial — `e563ad94` (pass) — observed: tests: 3 passed, 0 failed, 0 ignored, 0 not found; CI scripts: 2 passed, 0 failed — limits: (1) No IDSE cluster has declared acceptance obligations for this invariant yet (Doctrine §10); this record re-runs the checks carried from the legacy registry. Further limits: (2) No mutation witness exists yet for this critical invariant (Doctrine §28). |

_critical · local · scope: forge · assumptions: — · depends on: VAL-032._ Legacy: DC-NODE-10

#### `VAL-052` — _partial_

| Aspect | Location |
|--------|----------|
| **Source** | `docs/planning/node.define.consensus.md#batch-1` (approved by wdm33, 2026-11-09) |
| **Requirement** | **ForgeParentIntersectable** — A forge builds only on a durably stored block that a reference peer can find with FindIntersect on Ade's served chain; the recovered snapshot anchor is never a forge parent and is never served as a chain head a peer cannot intersect. |
| **Code** | `crates/ade_runtime/src/network/served_chain_projection.rs`; `crates/ade_node/src/node_lifecycle.rs` |
| **Tests** | `served_chain_intersects_at_followed_tip_and_rolls_to_forged`, `recovered_anchor_is_not_peer_intersectable`, `forged_successor_on_recovered_anchor_is_not_peer_adoptable`, `recover_follow_serve_forged_parent_intersectable` |
| **CI** | `ci/ci_check_forge_followed_tip_admission.sh` |
| **Claim** | partial — `e563ad94` (pass) — observed: tests: 4 passed, 0 failed, 0 ignored, 0 not found; CI scripts: 1 passed, 0 failed — limits: (1) No IDSE cluster has declared acceptance obligations for this invariant yet (Doctrine §10); this record re-runs the checks carried from the legacy registry. Further limits: (2) No mutation witness exists yet for this critical invariant (Doctrine §28). |

_critical · distributed · scope: forge base, serving · assumptions: A2 · depends on: VAL-050, VAL-047._ Legacy: DC-NODE-14

#### `VAL-053` — _partial_

| Aspect | Location |
|--------|----------|
| **Source** | `docs/planning/node.define.consensus.md#batch-1` (approved by wdm33, 2026-11-09); 1 recorded redefinition(s) in the registry |
| **Requirement** | **SingleProducerForgeBase** — In a declared single-producer venue the forge base becomes the node's own selected durable tip only through an established extension: entered by catching up and then self-admitting an own block through the admission path, or, at warm start, by a successful recovery whose durable tip lies above the bootstrap anchor and proves own-forged continuation, and held only while no competing block has been observed on the peer receive stream since catch-up or self-admission, the relay does not produce blocks, the durable chain is contiguous and servable, and no fork-choice decision is required. A failed warm-start entry falls back to SAF-013's catch-up gate; an established extension that loses a fence (a competing block is observed, or the relay produces) refuses to forge, never falling back to the catch-up gate. In the established extension, a clean structural end of the follow feed does not stop forging, while a shutdown request or a fatal feed failure does. A file-based adoption certificate is evidence only, never forge-base authority, and appears in no multi-producer, preprod or production forge path. |
| **Code** | `crates/ade_node/src/run_loop_planner.rs`; `crates/ade_node/src/node_lifecycle.rs`; `crates/ade_node/src/node_sync.rs` |
| **Tests** | `caughtup_self_admit_enters_extend_directly_no_cert`, `forge_base_selected_transcript_witnesses_local_tip`, `local_spine_sustains_two_successors_no_cert`, `local_spine_two_runs_byte_identical`, `local_spine_cert_file_absent_from_replay_surface`, `warm_start_reentry_requires_tip_above_recovered_anchor`, `warm_start_single_producer_re_enters_extend_and_forges` |
| **CI** | `ci/ci_check_local_durable_forge_base.sh` (red at e563ad94), `ci/ci_check_forge_followed_tip_admission.sh`, `ci/ci_check_cert_evidence_only.sh`, `ci/ci_check_node_path_fidelity.sh` (red at e563ad94), `ci/ci_check_warm_start_re_entry.sh` (red at e563ad94) |
| **Claim** | unresolved — `e563ad94` (fail) — observed: tests: 7 passed, 0 failed, 0 ignored, 0 not found; CI scripts: 2 passed, 3 failed; failing CI: ci/ci_check_local_durable_forge_base.sh, ci/ci_check_node_path_fidelity.sh, ci/ci_check_warm_start_re_entry.sh — limits: (1) No IDSE cluster has declared acceptance obligations for this invariant yet (Doctrine §10); this record re-runs the checks carried from the legacy registry. Further limits: (2) No mutation witness exists yet for this critical invariant (Doctrine §28). (3) ci/ci_check_local_durable_forge_base.sh fails at this revision and already failed before the IDSE migration (a3cef7f7): "FAIL (local-durable-forge-base): pub fn forge_mode_after_admit not found in crates/ade_node/src/node_sync.rs". Diagnosed as a stale or over-broad structural gate, not a demonstrated violation; the gate needs repair before it can serve as evidence. (4) ci/ci_check_node_path_fidelity.sh fails at this revision and already failed before the IDSE migration (a3cef7f7): "FAIL (node path fidelity): cli.rs flag set diverged from the pinned closed allow-list (G-D must add no flag; a private-only / venue flag is forbidden). diff (< pinned / > current):". Diagnosed as a stale or over-broad structural gate, not a demonstrated violation; the gate needs repair before it can serve as evidence. (5) ci/ci_check_warm_start_re_entry.sh fails at this revision and already failed before the IDSE migration (a3cef7f7): "FAIL (warm-start-re-entry): pub fn warm_start_forge_mode not found in crates/ade_node/src/node_sync.rs". Diagnosed as a stale or over-broad structural gate, not a demonstrated violation; the gate needs repair before it can serve as evidence. |

_critical · local · scope: single-producer forging · assumptions: — · depends on: STR-015, SAF-013._ Legacy: DC-NODE-20, DC-NODE-22, DC-NODE-19, DC-NODE-21

#### `VAL-054` — _partial_

| Aspect | Location |
|--------|----------|
| **Source** | `docs/planning/node.define.consensus.md#batch-1` (approved by wdm33, 2026-11-09) |
| **Requirement** | **ReceiveDispositionSingleClassifier** — Every peer announcement is classified once, by one pure total function of the durable tip, the candidate header and explicit evidence of whether the candidate's hash is already stored on the durable chain, as already held, a linear extension, refused in single-producer mode, or needing fork choice; a re-announcement of a block Ade already holds is "already held", never a competing candidate. |
| **Code** | `crates/ade_node/src/node_sync.rs` |
| **Tests** | `classify_already_have_when_in_spine`, `classify_linear_extend_on_exact_parent_and_block_no`, `classify_competing_on_nonmatching_parent`, `classify_competing_on_wrong_block_no`, `classify_competing_on_genesis_prev_hash`, `resolve_passthrough_already_have_and_linear_extend` |
| **CI** | `ci/ci_check_receive_detector_venue_split.sh` (red at e563ad94) |
| **Claim** | unresolved — `e563ad94` (fail) — observed: tests: 6 passed, 0 failed, 0 ignored, 0 not found; CI scripts: 0 passed, 1 failed; failing CI: ci/ci_check_receive_detector_venue_split.sh — limits: (1) No IDSE cluster has declared acceptance obligations for this invariant yet (Doctrine §10); this record re-runs the checks carried from the legacy registry. Further limits: (2) No mutation witness exists yet for this critical invariant (Doctrine §28). (3) ci/ci_check_receive_detector_venue_split.sh fails at this revision and already failed before the IDSE migration (a3cef7f7): "FAIL: missing: pub struct CandidateSummary". Diagnosed as a stale or over-broad structural gate, not a demonstrated violation; the gate needs repair before it can serve as evidence. |

_critical · local · scope: receive path · assumptions: — · depends on: DET-012._ Legacy: DC-NODE-23

#### `VAL-055` — _partial_

| Aspect | Location |
|--------|----------|
| **Source** | `docs/planning/node.define.consensus.md#batch-1` (approved by wdm33, 2026-11-09); 1 recorded redefinition(s) in the registry |
| **Requirement** | **IntersectionCandidatesMatchReference** — The points a node offers in chain-sync FindIntersect come only from its durable chain, read back from the tip through stored blocks whose recomputed hashes, heights decreasing by exactly one at each parent link and strictly decreasing slots are checked, any break being a typed error that stops following that peer; the offered depths are the reference client's (0, the Fibonacci numbers below l, and l itself, where l is k or the number of retained blocks if fewer, newest first); the bootstrap anchor is offered only when it is the tip; and one FindIntersect carrying the whole offer is sent per connection. The one exception to durable-chain points is LIV-007's resume hint, offered first after a reconnect; per-peer FIFO delivery makes that block durable before any following rollback is resolved. An IntersectFound naming a point that was not offered fails closed without reconnecting; IntersectNotFound stops following that peer with a typed reason naming the bound that was reached, with no RequestNext, no reconnect and no durable change; and the intersection is never rollback authority, the RollBackward that follows being resolved by SEC-020. |
| **Code** | `crates/ade_ledger/src/rollback/intersection.rs`; `crates/ade_runtime/src/admission/wire_pump.rs`; `crates/ade_node/src/node_lifecycle.rs` |
| **Tests** | `rollback::intersection::tests::lv16_offsets_are_cardanos_mk_offsets`, `rollback::intersection::tests::lv16_selection_is_newest_first_and_always_offers_the_deepest`, `rollback::intersection::tests::lv16_killed_store_shape_offers_the_parent_and_finds_it`, `rollback::intersection::tests::lv16_ancestry_faults_are_typed`, `rollback::intersection::tests::lv16_reply_must_name_an_offered_point`, `rollback::intersection::tests::lv16_not_found_is_no_common_point_with_the_bound`, `rollback::intersection::tests::lv16_a_point_without_stored_ancestry_is_offered_alone`, `rollback::intersection::tests::lv16_delivered_head_goes_first_only_when_newer`, `admission::wire_pump::tests::lv16_pump_offers_every_candidate_and_follows_the_found_parent`, `admission::wire_pump::tests::lv16_pump_ends_on_intersect_not_found_without_request_next`, `admission::wire_pump::tests::lv16_pump_refuses_an_unoffered_found_point`, `node_lifecycle::tests::lv16_no_reconnect_after_an_intersection_outcome`, `lv16_corpus_blocks_are_a_linked_chain`, `lv16_offer_is_read_from_the_validated_durable_ancestry`, `lv16_a_block_whose_hash_is_not_its_lookup_hash_is_refused`, `lv16_a_tip_that_is_not_stored_is_offered_alone`, `lv16_killed_store_shape_intersects_at_the_parent`, `lv16_a_deeper_fork_intersects_at_the_newest_offered_common_point`, `lv16_a_peer_sharing_no_offered_point_is_no_common_point`, `lv16_rollback_to_the_found_parent_matches_a_run_that_never_saw_the_tip` |
| **CI** | `ci/ci_check_durable_intersection.sh` |
| **Claim** | partial — `e563ad94` (pass) — observed: tests: 20 passed, 0 failed, 0 ignored, 0 not found; CI scripts: 1 passed, 0 failed — limits: (1) No IDSE cluster has declared acceptance obligations for this invariant yet (Doctrine §10); this record re-runs the checks carried from the legacy registry. Further limits: (2) No mutation witness exists yet for this critical invariant (Doctrine §28). |

_critical · local · scope: chain-sync client · assumptions: A2, A4 · depends on: REC-005._ Legacy: DC-NODE-48

### VAL — pipeline-production

#### `VAL-056` — _partial_

| Aspect | Location |
|--------|----------|
| **Source** | `docs/planning/pipeline-production.define.consensus.md#batch-1` (approved by wdm33, 2026-11-09) |
| **Requirement** | **ForgeOnlyWhenLeader** — A block is forged for a slot only if the leader check passes for that slot, evaluated by the same leader-check functions header validation uses (VRF input by VAL-036, the leader value from the VRF output, the threshold from the slot's epoch's sealed stake distribution by VAL-024), with no producer-side variant of any of them; for the same canonical inputs the leader verdict is byte-identical across runs, and for the Conway era exactly one Praos VRF construction exists, with no fallback that builds or accepts the TPraos input. |
| **Code** | `crates/ade_ledger/src/producer/forge.rs`; `crates/ade_core/src/consensus/leader_schedule.rs`; `crates/ade_core/src/consensus/leader_check.rs`; `crates/ade_node/tests/forge_handler_variants.rs`; `crates/ade_node/src/node_sync.rs`; `crates/ade_core/src/consensus/vrf_cert.rs`; `crates/ade_node/src/produce_mode.rs` |
| **Tests** | `forge_block_rejects_non_leader_tick`, `forge_block_uses_validator_leader_check_function`, `verdict_is_byte_identical_across_two_runs`, `run_real_forge_is_byte_identical_across_two_runs`, `forge_from_recovered_is_deterministic_across_two_runs`, `forge_to_self_accept_succeeds`, `praos_call_with_tpraos_answer_emits_vrf_input_mismatch`, `tpraos_producer_forge_fails_closed_with_unsupported_era`, `is_praos_only_babbage_and_conway`, `query_uses_state_epoch_nonce_for_vrf_input` |
| **CI** | `ci/ci_check_forge_purity.sh` (red at e563ad94), `ci/ci_check_producer_praos_vrf.sh` |
| **Claim** | unresolved — `e563ad94` (fail) — observed: tests: 10 passed, 0 failed, 0 ignored, 0 not found; CI scripts: 1 passed, 1 failed; failing CI: ci/ci_check_forge_purity.sh — limits: (1) No IDSE cluster has declared acceptance obligations for this invariant yet (Doctrine §10); this record re-runs the checks carried from the legacy registry. Further limits: (2) No mutation witness exists yet for this critical invariant (Doctrine §28). (3) ci/ci_check_forge_purity.sh fails at this revision and already failed before the IDSE migration (a3cef7f7): "FAIL: Guard 2 (parallel is_leader definition outside leader_schedule.rs):". Diagnosed as a stale or over-broad structural gate, not a demonstrated violation; the gate needs repair before it can serve as evidence. |

_critical · local · scope: leader check on the forge path · assumptions: A2 · depends on: VAL-036, VAL-024, SEC-003 · secondary: DET._ Legacy: DC-CONS-15, DC-FORGE-01, CN-FORGE-04

#### `VAL-057` — _partial_

| Aspect | Location |
|--------|----------|
| **Source** | `docs/planning/pipeline-production.define.consensus.md#batch-1` (approved by wdm33, 2026-11-09) |
| **Requirement** | **ForgedBlockSingleEncoding** — A forged block is emitted as the era-tagged [era, block] envelope by the one block encoder that is the inverse of the block decoder header validation uses, and its header's body hash is computed by the validator's body-hash recipe over the very body bytes it emits; the forge rejects, with a typed error and before emitting any bytes, a tick whose block number and previous hash form an illegal header position (VAL-003); and decoding a forged block yields exactly the header fields and four body segments that were forged, so no forge output exists that the block decoder rejects. |
| **Code** | `crates/ade_ledger/src/block_body_hash.rs`; `crates/ade_ledger/src/producer/forge.rs`; `crates/ade_ledger/src/block_validity/header_input.rs`; `crates/ade_runtime/src/producer/served_chain_lookups.rs`; `crates/ade_ledger/src/receive/reducer.rs`; `crates/ade_codec/src/cbor/envelope.rs` |
| **Tests** | `block_body_hash_pinned_recipe_byte_identical`, `block_body_hash_from_block_equals_from_buckets`, `block_body_hash_none_invalid_txs_equals_empty_bucket`, `forged_body_hash_matches_validator_recomputation`, `body_encoder_is_single_authority`, `accepted_block_header_bytes_equals_validator_split_on_corpus`, `accepted_block_header_bytes_is_subslice_of_as_bytes`, `session_transcript_announced_header_matches_served_body_recipe`, `receive_apply_block_delivered_with_matching_header_admits`, `receive_pipeline_corpus_drive_admitted_bytes_equal_corpus_bytes`, `encode_decode_block_envelope_round_trips`, `conway_envelope_head_is_82_07`, `encode_block_envelope_reencodes_corpus_block_identically`, `forge_block_output_decodes_via_decode_block` |
| **CI** | `ci/ci_check_no_producer_body_encoder.sh`, `ci/ci_check_no_parallel_header_splitter.sh`, `ci/ci_check_receive_reducer_closure.sh`, `ci/ci_check_forge_decode_round_trip.sh` |
| **Claim** | unresolved — `e563ad94` (pass) — observed: tests: 14 passed, 0 failed, 0 ignored, 0 not found; CI scripts: 4 passed, 0 failed — limits: (1) No IDSE cluster has declared acceptance obligations for this invariant yet (Doctrine §10); this record re-runs the checks carried from the legacy registry. Further limits: (2) No mutation witness exists yet for this critical invariant (Doctrine §28). (3) Expected to be falsified (pipeline-production S15): the forge engine emits blocks whose block number and previous hash form an illegal header position, which the decoder then rejects; no check asserts the forge-time rejection yet. |

_critical · local · scope: forged block bytes · assumptions: A2 · depends on: VAL-032, VAL-003, DET-002, DET-007 · secondary: DET._ Legacy: DC-CONS-16, CN-FORGE-03

#### `VAL-058` — _partial_

| Aspect | Location |
|--------|----------|
| **Source** | `docs/planning/pipeline-production.define.consensus.md#batch-1` (approved by wdm33, 2026-11-09) |
| **Requirement** | **ForgedTransactionsAdmissibleInOrder** — Every transaction in a forged block is admissible by the mempool admission function against the forge base's ledger state, applied in the transaction snapshot's accumulating order; the forge re-admits the snapshot in that order before building the body and fails with a typed error at the first transaction that is rejected or differs, and never reorders, adds, drops or skips a transaction. |
| **Code** | `crates/ade_ledger/src/producer/forge.rs`; `crates/ade_ledger/src/mempool/admit.rs` |
| **Tests** | `forge_block_rejects_tx_not_in_mempool_accepted_prefix`, `forge_block_rejects_tx_permuted_from_accumulating_order`, `forge_block_empty_mempool_produces_empty_body`, `admit_prefix_property_documented` |
| **CI** | `ci/ci_check_forge_purity.sh` (red at e563ad94) |
| **Claim** | unresolved — `e563ad94` (fail) — observed: tests: 4 passed, 0 failed, 0 ignored, 0 not found; CI scripts: 0 passed, 1 failed; failing CI: ci/ci_check_forge_purity.sh — limits: (1) No IDSE cluster has declared acceptance obligations for this invariant yet (Doctrine §10); this record re-runs the checks carried from the legacy registry. Further limits: (2) No mutation witness exists yet for this critical invariant (Doctrine §28). (3) ci/ci_check_forge_purity.sh fails at this revision and already failed before the IDSE migration (a3cef7f7): "FAIL: Guard 2 (parallel is_leader definition outside leader_schedule.rs):". Diagnosed as a stale or over-broad structural gate, not a demonstrated violation; the gate needs repair before it can serve as evidence. |

_critical · local · scope: forged block body · assumptions: — · depends on: VAL-008, SAF-003 · secondary: ORD._ Legacy: DC-LEDGER-12

#### `VAL-059` — _partial_

| Aspect | Location |
|--------|----------|
| **Source** | `docs/planning/pipeline-production.define.consensus.md#batch-1` (approved by wdm33, 2026-11-09) |
| **Requirement** | **ForgeBaseFromBootstrapAuthority** — In every mode, the forge base (ledger state, chain-dependent state, stake distribution view, epoch nonce and tip) is initialized from the state the one bootstrap authority produced and is updated by every later admission, admissions before the first forge included, so each forge builds on the latest admission's state (VAL-051); the protocol version and parameters a forge uses come from that base's ledger state. No forge path builds a synthetic base: no zero-stake or empty ledger, no default protocol parameters or version, no constant previous hash and no second recovered state. |
| **Code** | `crates/ade_node/src/produce_mode.rs`; `crates/ade_runtime/src/producer/chain_evolution.rs`; `crates/ade_node/src/cli.rs` |
| **Tests** | `produce_cli_requires_seed_and_consensus_inputs`, `produce_mode_bootstrap_cold_start_seeds_real_ledger` |
| **CI** | `ci/ci_check_produce_mode_uses_bootstrap_initial_state.sh` |
| **Claim** | partial — `e563ad94` (pass) — observed: tests: 2 passed, 0 failed, 0 ignored, 0 not found; CI scripts: 1 passed, 0 failed — limits: (1) No IDSE cluster has declared acceptance obligations for this invariant yet (Doctrine §10); this record re-runs the checks carried from the legacy registry. Further limits: (2) No mutation witness exists yet for this critical invariant (Doctrine §28). |

_critical · local · scope: forge base, every mode · assumptions: A2 · depends on: STR-013, VAL-051 · secondary: STR._ Legacy: CN-PROD-03

#### `VAL-060` — _partial_

| Aspect | Location |
|--------|----------|
| **Source** | `docs/planning/pipeline-production.define.consensus.md#batch-1` (approved by wdm33, 2026-11-09) |
| **Requirement** | **ForgedBlocksAcceptedByReference** — A block Ade forges, self-accepts and serves is accepted by the reference node: a reference cardano-node peer that follows Ade fetches it over chain-sync and block-fetch, validates it and adopts it when it is that peer's best chain, and never rejects it as invalid. |
| **Code** | `crates/ade_runtime/tests/cross_impl_server_pipeline.rs`; `crates/ade_core_interop/src/bin/live_block_fetch_session.rs`; `crates/ade_runtime/src/network/n2n_listener.rs`; `crates/ade_node/src/produce_mode.rs` |
| **Tests** | `cross_impl_server_pipeline_request_range_returns_decodable_bytes`, `cross_impl_server_pipeline_request_range_byte_identical_to_self_accept_input`, `live_block_fetch_session_hermetic_default_prints_readiness`, `n2n_listener_loopback_handshake_succeeds`, `produce_mode_starts_runs_three_slots_and_exits_via_max_slots`, `live_wire_pump_feed_reaches_forge_tick`, `live_feed_forge_serve_loopback_returns_forged_block` |
| **CI** | `ci/ci_check_server_paths_corpus_present.sh` |
| **Claim** | partial — `e563ad94` (pass) — observed: tests: 7 passed, 0 failed, 0 ignored, 0 not found; CI scripts: 1 passed, 0 failed — limits: (1) No IDSE cluster has declared acceptance obligations for this invariant yet (Doctrine §10); this record re-runs the checks carried from the legacy registry. Further limits: (2) No mutation witness exists yet for this critical invariant (Doctrine §28). (3) No live reference peer in this environment; acceptance of Ade-forged blocks by a reference node is not exercised here. |

_critical · distributed · scope: forged and served blocks, live venue · assumptions: A2 · depends on: VAL-007, VAL-002, SAF-020 · secondary: PRC._ Legacy: none (new in IDSE)

#### `VAL-061` — _partial_

| Aspect | Location |
|--------|----------|
| **Source** | `docs/planning/pipeline-production.define.consensus.md#batch-1` (approved by wdm33, 2026-11-09) |
| **Requirement** | **KeepAliveClientWireOnly** — The wire pump runs the node-to-node keep-alive client through the shared keep-alive state machine and codec: it sends a keep-alive on a fixed cadence shorter than the peer's keep-alive timeout, never while one is outstanding, with a cookie from a deterministic wrapping 16-bit counter, and accepts a response only if its cookie matches; a grammar or cookie violation ends that peer's session with a typed error. Keep-alive traffic produces no pump event, no event in any evidence vocabulary and no canonical input (shell diagnostics, excluded from evidence and replay, may record it), is never dispatched as chain-sync or block-fetch traffic, and does not delay or reorder them. |
| **Code** | `crates/ade_runtime/src/admission/wire_pump.rs`; `crates/ade_network/src/keep_alive/transition.rs`; `crates/ade_network/src/codec/keep_alive.rs` |
| **Tests** | `admission::wire_pump::tests::wire_pump_sends_keep_alive_on_quiescent_cadence`, `admission::wire_pump::tests::wire_pump_keep_alive_response_validates_cookie_no_event`, `admission::wire_pump::tests::wire_pump_keep_alive_cookie_mismatch_fails_closed` |
| **CI** | `ci/ci_check_keep_alive_wire_only.sh` (red at e563ad94) |
| **Claim** | unresolved — `e563ad94` (fail) — observed: tests: 3 passed, 0 failed, 0 ignored, 0 not found; CI scripts: 0 passed, 1 failed; failing CI: ci/ci_check_keep_alive_wire_only.sh — limits: (1) No IDSE cluster has declared acceptance obligations for this invariant yet (Doctrine §10); this record re-runs the checks carried from the legacy registry. Further limits: (2) ci/ci_check_keep_alive_wire_only.sh fails at this revision and already failed before the IDSE migration (a3cef7f7): "FAIL: keep-alive cadence must enqueue an OutboundFrame on AcceptedMiniProtocol::KeepAlive". Diagnosed as a stale or over-broad structural gate, not a demonstrated violation; the gate needs repair before it can serve as evidence. |

_standard · local · scope: wire pump keep-alive client · assumptions: — · depends on: DET-016, SEC-016 · secondary: LIV._ Legacy: DC-PUMP-03

#### `VAL-062` — _partial_

| Aspect | Location |
|--------|----------|
| **Source** | `docs/planning/pipeline-production.define.consensus.md#batch-1` (approved by wdm33, 2026-11-09) |
| **Requirement** | **SeedImportFailsClosed** — The UTxO seed importer accepts only the transaction-output features it fully supports; an unsupported feature or a malformed reference script is a typed import error that stops the import, and no admission path matches that error with a permissive arm, skips the output or falls back to a partial seed. |
| **Code** | `crates/ade_runtime/src/seed_import/importer.rs` |
| **Tests** | _(no tests found — gap)_ |
| **CI** | `ci/ci_check_admission_no_refscript_skip.sh` |
| **Claim** | partial — `e563ad94` (pass) — observed: tests: 0 passed, 0 failed, 0 ignored, 0 not found; CI scripts: 1 passed, 0 failed — limits: (1) No IDSE cluster has declared acceptance obligations for this invariant yet (Doctrine §10); this record re-runs the checks carried from the legacy registry. |

_standard · local · scope: UTxO seed import · assumptions: — · depends on: VAL-001._ Legacy: DC-ADMIT-09

### VAL — mithril-evidence

#### `VAL-063` — _partial_

| Aspect | Location |
|--------|----------|
| **Source** | `docs/planning/mithril-evidence.define.consensus.md#batch-1` (approved by wdm33, 2026-11-09) |
| **Requirement** | **LedgerStateDecodeFaithful** — The reference node's ledger-state file is decoded into Ade's canonical certificate state, pool distribution, Praos nonces and protocol parameters by one deterministic decoder: the same bytes and certified epoch give byte-identical canonical output; the hard-fork telescope is navigated by counting past eras to the one current era, which must be Conway; every active pool's VRF key hash is decoded from its registration parameters, a zero VRF key hash or a disagreement with the pool distribution's VRF key hash being terminal; the decoded epoch is checked against the certified point's epoch; the decoded certificate state survives a canonical round trip; and malformed input halts with a typed error after bounded reads, never a partial decode. |
| **Code** | `crates/ade_ledger/src/ledgerdb_state.rs`; `crates/ade_ledger/tests/ledgerdb_state_hermetic.rs`; `crates/ade_runtime/tests/ledgerdb_state_corpus.rs`; `crates/ade_runtime/tests/ledgerdb_state_mithril.rs`; `ci/ci_check_ledgerdb_state_decode.sh` |
| **Tests** | `happy_minimal_state_decodes_with_required_elements`, `determinism_same_bytes_same_commitment`, `zero_vrf_is_terminal`, `wrong_era_is_terminal_no_fallback_to_latest`, `pool_distr_vrf_mismatch_is_terminal`, `epoch_mismatch_is_terminal`, `malformed_cbor_is_terminal`, `decode_local_preview_corpus`, `decode_verified_mithril_ledger_state` |
| **CI** | `ci/ci_check_ledgerdb_state_decode.sh` |
| **Claim** | partial — `e563ad94` (pass) — observed: tests: 9 passed, 0 failed, 0 ignored, 0 not found; CI scripts: 1 passed, 0 failed — limits: (1) No IDSE cluster has declared acceptance obligations for this invariant yet (Doctrine §10); this record re-runs the checks carried from the legacy registry. Further limits: (2) No mutation witness exists yet for this critical invariant (Doctrine §28). |

_critical · local · scope: ledger-state decode (native route) · assumptions: A2 · depends on: VAL-001, SEC-002 · secondary: DET._ Legacy: DC-MITHRIL-04

#### `VAL-064` — _partial_

| Aspect | Location |
|--------|----------|
| **Source** | `docs/planning/mithril-evidence.define.consensus.md#batch-1` (approved by wdm33, 2026-11-09) |
| **Requirement** | **UtxoTablesMaterializeFaithfully** — The snapshot's UTxO tables are decoded and materialized into Ade's UTxO state without loss: every asset quantity keeps its full unsigned 64-bit value; inline datums and scripts keep their original bytes, wrapped by the shared tag-24 authority and never re-encoded; every nesting boundary consumes exactly its bytes, with explicit endianness; an unknown output tag, address form, value tag or script language is a typed terminal error; keys must arrive in strictly ascending transaction-input order; and the era is taken from the same snapshot's ledger state. The materialized UTxO's fingerprint is bound, with the manifest's certified point, the ledger-state commitment and the tables commitment, into one record whose check is terminal on any mismatch, and the UTxO state round-trips through persistence with an identical fingerprint, quantities above the signed 64-bit maximum included. |
| **Code** | `crates/ade_ledger/src/ledgerdb_tables.rs`; `crates/ade_runtime/tests/ledgerdb_tables_decode.rs`; `crates/ade_runtime/tests/ledgerdb_tables_oracle.rs`; `ci/ci_check_ledgerdb_tables_decode.sh`; `crates/ade_ledger/src/mithril_utxo_materialize.rs`; `crates/ade_runtime/tests/mithril_tables_to_utxostate.rs`; `ci/ci_check_tables_to_utxostate.sh` |
| **Tests** | `multiasset_quantities_preserved_exactly_as_u64_no_i64_cast`, `coin_varlen_overflow_is_terminal`, `addr28_base_address_reconstruction_round_trip`, `staking_credential_tag_is_fail_closed`, `compact_value_ada_only_and_multiasset`, `txout_dispatch_tag0_tag5_and_fail_closed`, `tables_commitment_deterministic_era_bound_and_sorted`, `varlen_big_endian_7bit_matches_real_coin`, `decode_real_preprod_tables_commitment`, `deterministic_utxo_commitment`, `u64_above_i64_max_materializes_persists_recovers_exactly`, `datum_and_script_bytes_preserved_verbatim_in_raw`, `alonzo_plus_raw_round_trips_to_same_fields`, `canonical_txin_ordering_asserted`, `fail_closed_negatives`, `binding_is_terminal_on_mismatch`, `persist_recover_identical_fingerprint`, `no_datum_no_script_is_shelley_mary_byron_is_byron`, `materialized_count_matches_stage2_commitment_count`, `materialize_real_preprod_tables_to_utxo_state` |
| **CI** | `ci/ci_check_ledgerdb_tables_decode.sh`, `ci/ci_check_tables_to_utxostate.sh` |
| **Claim** | partial — `e563ad94` (pass) — observed: tests: 20 passed, 0 failed, 0 ignored, 0 not found; CI scripts: 2 passed, 0 failed — limits: (1) No IDSE cluster has declared acceptance obligations for this invariant yet (Doctrine §10); this record re-runs the checks carried from the legacy registry. Further limits: (2) No mutation witness exists yet for this critical invariant (Doctrine §28). |

_critical · local · scope: UTxO tables decode and materialization (native route) · assumptions: A2 · depends on: VAL-013, VAL-002, VAL-001 · secondary: DET._ Legacy: DC-MITHRIL-05, DC-MITHRIL-06

#### `VAL-065` — _partial_

| Aspect | Location |
|--------|----------|
| **Source** | `docs/planning/mithril-evidence.define.consensus.md#batch-1` (approved by wdm33, 2026-11-09) |
| **Requirement** | **NativeBootstrapBoundaryReady** — When the decoded certificate state carries delegations, the native first run builds the reduced UTxO checkpoint from the materialized UTxO and seals it at the certified slot before the UTxO is consumed and before the bootstrap commits, so the node can derive its next epoch's authority at the first boundary it crosses; when it carries none, no checkpoint is built and the bootstrap output is byte-identical to a build without this step; a checkpoint failure is terminal before the commit. |
| **Code** | `crates/ade_node/src/native_firstrun.rs`; `crates/ade_node/src/node_lifecycle.rs`; `ci/ci_check_native_firstrun_reduced_checkpoint.sh` |
| **Tests** | `native_first_run_real_snapshot_invokes_bootstrap_and_persists` |
| **CI** | `ci/ci_check_native_firstrun_reduced_checkpoint.sh` |
| **Claim** | partial — `e563ad94` (pass) — observed: tests: 1 passed, 0 failed, 0 ignored, 0 not found; CI scripts: 1 passed, 0 failed — limits: (1) No IDSE cluster has declared acceptance obligations for this invariant yet (Doctrine §10); this record re-runs the checks carried from the legacy registry. Further limits: (2) No mutation witness exists yet for this critical invariant (Doctrine §28). |

_critical · local · scope: native Mithril first run · assumptions: A2 · depends on: DUR-003, DET-013, DUR-014 · secondary: LIV._ Legacy: DC-MITHRIL-08

#### `VAL-066` — _partial_

| Aspect | Location |
|--------|----------|
| **Source** | `docs/planning/mithril-evidence.define.consensus.md#batch-1` (approved by wdm33, 2026-11-09) |
| **Requirement** | **ConwayDepositParamsFromSnapshot** — The Conway deposit parameters (DRep deposit, governance-action deposit and DRep activity) are decoded from the certified snapshot's current protocol parameters by the one protocol-parameter decoder and carried as a required field into the assembled ledger state and the epoch accumulator; they are never defaulted, guessed or read by a second parser; they are covered by the ledger fingerprint and the ledger-state commitment; missing or malformed parameters are terminal; and an accumulator encoded without them, or in the format from before them, fails closed at decode, its remedy being re-bootstrap. |
| **Code** | `crates/ade_ledger/src/ledgerdb_state.rs`; `crates/ade_runtime/src/mithril_native_assembly.rs`; `crates/ade_ledger/src/epoch_accumulator.rs`; `crates/ade_ledger/src/fingerprint.rs`; `crates/ade_ledger/src/state.rs`; `crates/ade_testkit/src/harness/snapshot_loader.rs`; `ci/ci_check_conway_deposit_params_bootstrap.sh` |
| **Tests** | `ade_ledger tests/ledgerdb_nonutxo_hermetic.rs::happy_minimal_state_decodes_all_fields`, `ade_ledger tests/ledgerdb_nonutxo_hermetic.rs::malformed_drep_activity_type_is_terminal`, `ade_ledger tests/ledgerdb_nonutxo_hermetic.rs::malformed_pparams_arity_is_terminal`, `ade_ledger tests/ledgerdb_nonutxo_hermetic.rs::wrong_era_is_terminal`, `ade_ledger::ledgerdb_state::tip_tests::v6_commitment_is_deterministic_and_binds_gov`, `ade_ledger::fingerprint::tests::tampered_drep_activity_flips_pparams_fingerprint`, `ade_ledger::fingerprint::tests::pparams_fingerprint_includes_conway_deposits_when_present`, `ade_ledger::epoch_accumulator::tests::codec_v2_conway_without_deposit_params_fails_closed`, `ade_ledger::epoch_accumulator::tests::codec_rejects_unknown_version`, `ade_ledger::epoch_accumulator::tests::imported_deposit_params_cross_governance_boundary_and_reach_gov_cert_env`, `ade_runtime::mithril_native_assembly::tests::native_assembly_maps_each_field_from_its_source`, `ade_runtime::chaindb::epoch_accumulator_store::tests::seal_advance_reset_round_trip_is_exact`, `ade_testkit tests/conway_deposit_params_single_decoder.rs::harness_and_blue_decoder_agree_on_conway_deposit_params` (FAILED at e563ad94) |
| **CI** | `ci/ci_check_conway_deposit_params_bootstrap.sh` |
| **Claim** | unresolved — `e563ad94` (fail) — observed: tests: 12 passed, 1 failed, 0 ignored, 0 not found; CI scripts: 1 passed, 0 failed; failing tests: ade_testkit tests/conway_deposit_params_single_decoder.rs::harness_and_blue_decoder_agree_on_conway_deposit_params (one-decoder proof: harness == BLUE on the same bytes) — limits: (1) No IDSE cluster has declared acceptance obligations for this invariant yet (Doctrine §10); this record re-runs the checks carried from the legacy registry. Further limits: (2) No mutation witness exists yet for this critical invariant (Doctrine §28). (3) harness_and_blue_decoder_agree_on_conway_deposit_params fails because its synthetic fixture holds a pool with stake but no frozen parameters, which the decoder now refuses (VAL-028); the fixture is stale, not a counterexample to this invariant. (4) Corrects this invariant's previous evidence record at the same revision: the earlier record's test matching missed references written as '<crate> tests/<file>.rs::<name>' or '<crate>::<path>'; the test outcomes are the same run's (docs/evidence/idse-m2-reevidence.json). |

_critical · local · scope: native Mithril first run, epoch accumulator · assumptions: A2 · depends on: VAL-014, CON-002, EVO-005._ Legacy: DC-CINPUT-07

### VAL — resources-release

#### `VAL-067` — _partial_

| Aspect | Location |
|--------|----------|
| **Source** | `docs/planning/resources-release.define.consensus.md#batch-1` (approved by wdm33, 2026-11-09) |
| **Requirement** | **MempoolAgreesWithLedger** — A transaction that passes resource admission (SEC-035) is admitted to the mempool if and only if standalone transaction validity (VAL-009: phase 1 accepts it and its scripts evaluate successfully), computed by the same phase-1 and script-evaluation functions block validity uses, accepts it against the mempool's running state (the base ledger state with every earlier admitted transaction applied); no mempool-only validity rule exists, and a resource refusal is reported as a distinct reason, never as invalidity. |
| **Code** | `crates/ade_ledger/src/mempool/admit.rs`; `crates/ade_ledger/src/mempool/ingress.rs`; `crates/ade_testkit/src/mempool/ingress_replay.rs` |
| **Tests** | `valid_tx_admitted_and_accumulates`, `invalid_tx_rejected_no_false_accept`, `admission_equals_tx_validity_verdict`, `dependent_tx_admitted_against_accumulating_state`, `ingress_admit_equals_direct_admit_on_b_track_corpus`, `b_track_adversarial_rejections_preserved_through_ingress`, `dependent_pair_through_ingress_admits_b_after_a` |
| **CI** | `ci/ci_check_consensus_closed_enums.sh` |
| **Claim** | partial — `e563ad94` (pass) — observed: tests: 4 passed, 0 failed, 0 ignored, 3 not found; CI scripts: 1 passed, 0 failed — limits: (1) No IDSE cluster has declared acceptance obligations for this invariant yet (Doctrine §10); this record re-runs the checks carried from the legacy registry. Further limits: (2) No mutation witness exists yet for this critical invariant (Doctrine §28). (3) 3 named test(s) were not found in the test output at this revision: ingress_admit_equals_direct_admit_on_b_track_corpus, b_track_adversarial_rejections_preserved_through_ingress, dependent_pair_through_ingress_admits_b_after_a |

_critical · local · scope: mempool admission · assumptions: A2 · depends on: VAL-008, VAL-009._ Legacy: DC-MEM-01, CN-MEM-04

---

## ORD — Ordering

_Doctrine §6: Required sequencing, precedence, and ordering of events or transitions._ 6 invariant(s).

### ORD — foundations

#### `ORD-001` — _declared_

| Aspect | Location |
|--------|----------|
| **Source** | `docs/planning/foundations.define.consensus.md#batch-1` (approved by wdm33, 2026-11-08) |
| **Requirement** | **NoRetroactiveValidation** — The verdict on a block or transaction depends only on the chain prefix before it and on context fixed by that prefix or recorded before its validation, such as the slot it claims; no data first derived after that point (a later block, a later epoch's stake distribution or nonce, a later checkpoint) changes its verdict, and an adopted checkpoint is never reinterpreted under a different rule version. |
| **Code** | _(no enforcement found — gap)_ |
| **Tests** | _(no tests found — gap)_ |
| **CI** | _(no CI enforcement — gap)_ |
| **Claim** | unresolved — no evidence record |

_critical · local · scope: ledger and consensus validation · assumptions: A1, A2 · depends on: DET-001 · secondary: VAL._ Legacy: T-CAUSAL-01

### ORD — ledger

#### `ORD-002` — _partial_

| Aspect | Location |
|--------|----------|
| **Source** | `docs/planning/ledger.define.consensus.md#batch-1` (approved by wdm33, 2026-11-08) |
| **Requirement** | **HeaderBeforeBody** — A block's header is validated before its body, its body is never validated when its header is invalid, and the first authority to fail determines the block's rejection reason. |
| **Code** | `crates/ade_ledger/src/block_validity/transition.rs` |
| **Tests** | `header_before_body_fail_fast` |
| **CI** | `ci/ci_check_consensus_closed_enums.sh` |
| **Claim** | partial — `e563ad94` (pass) — observed: tests: 1 passed, 0 failed, 0 ignored, 0 not found; CI scripts: 1 passed, 0 failed — limits: (1) No IDSE cluster has declared acceptance obligations for this invariant yet (Doctrine §10); this record re-runs the checks carried from the legacy registry. Further limits: (2) No mutation witness exists yet for this critical invariant (Doctrine §28). |

_critical · local · scope: block validation · assumptions: — · depends on: VAL-009 · secondary: VAL._ Legacy: DC-VAL-03

### ORD — epoch

#### `ORD-003` — _partial_

| Aspect | Location |
|--------|----------|
| **Source** | `docs/planning/epoch.define.consensus.md#batch-1` (approved by wdm33, 2026-11-08) |
| **Requirement** | **EffectsActivateAtBoundaries** — Stake distribution changes, rewards, protocol-parameter changes and governance effects take effect only at the epoch boundaries where the protocol defines them to, never in the middle of an epoch. |
| **Code** | `crates/ade_core/src/consensus/leader_schedule.rs`; `crates/ade_core/src/consensus/ledger_view.rs`; `crates/ade_ledger/src/consensus_view.rs` |
| **Tests** | `consensus::leader_schedule::tests::query_uses_state_epoch_nonce_for_vrf_input`, `corpus_returns_canonical_answer_for_known_pools`, `corpus_rejects_unknown_pool`, `corpus_is_deterministic_across_runs`, `view_returns_corpus_pool_stake_and_vrf_keyhash`, `view_unknown_epoch_returns_none`, `view_is_pure` |
| **CI** | _(no CI enforcement — gap)_ |
| **Claim** | partial — `e563ad94` (pass) — observed: tests: 7 passed, 0 failed, 0 ignored, 0 not found; CI scripts: 0 passed, 0 failed — limits: (1) No IDSE cluster has declared acceptance obligations for this invariant yet (Doctrine §10); this record re-runs the checks carried from the legacy registry. Further limits: (2) No mutation witness exists yet for this critical invariant (Doctrine §28). |

_critical · local · scope: ledger and governance effects · assumptions: A2 · depends on: VAL-020._ Legacy: CN-EPOCH-01

#### `ORD-004` — _partial_

| Aspect | Location |
|--------|----------|
| **Source** | `docs/planning/epoch.define.consensus.md#batch-1` (approved by wdm33, 2026-11-08) |
| **Requirement** | **ActivationAdvancesOneBoundary** — The active epoch view moves forward one boundary at a time, from the view for epoch P to the view for epoch P+1 only; decisions for epoch N+1 never read epoch N's view, and a candidate slot two or more epochs past the active view is a terminal failure, never a skip. |
| **Code** | `crates/ade_node/src/epoch_rebind.rs`; `crates/ade_node/src/node_sync.rs`; `ci/ci_check_eview_epoch_rebind.sh`; `crates/ade_node/src/epoch_activation.rs`; `ci/ci_check_eview_activation_predicate.sh`; `crates/ade_node/src/epoch_wire.rs` |
| **Tests** | `simulated_transition_promotes_bound_n1_view`, `same_epoch_keeps_current`, `off_epoch_without_bound_view_fails_closed`, `not_immediate_next_fails_closed`, `unlocatable_fails_closed`, `rejects_each_wrong_binding`, `rejects_tampered_view_wrong_hash`, `replay_equivalent_deterministic`, `crash_restart_redrives_same_decision_both_sides`, `active_view_one_way_promote_and_idempotence`, `active_view_conflicting_promotion_is_terminal`, `seed_exposes_no_n1_view_until_promotion` |
| **CI** | `ci/ci_check_eview_epoch_rebind.sh` (red at e563ad94), `ci/ci_check_eview_activation_predicate.sh` (red at e563ad94) |
| **Claim** | unresolved — `e563ad94` (fail) — observed: tests: 12 passed, 0 failed, 0 ignored, 0 not found; CI scripts: 0 passed, 2 failed; failing CI: ci/ci_check_eview_epoch_rebind.sh, ci/ci_check_eview_activation_predicate.sh — limits: (1) No IDSE cluster has declared acceptance obligations for this invariant yet (Doctrine §10); this record re-runs the checks carried from the legacy registry. Further limits: (2) No mutation witness exists yet for this critical invariant (Doctrine §28). (3) ci/ci_check_eview_epoch_rebind.sh fails at this revision and already failed before the IDSE migration (a3cef7f7): "FAIL: the live seam does not pass None for the bound view -- OffEpoch must stay fail-closed until S3f-4". Diagnosed as a stale or over-broad structural gate, not a demonstrated violation; the gate needs repair before it can serve as evidence. (4) ci/ci_check_eview_activation_predicate.sh fails at this revision and already failed before the IDSE migration (a3cef7f7): "FAIL: the predicate does not check the selected-chain point". Diagnosed as a stale or over-broad structural gate, not a demonstrated violation; the gate needs repair before it can serve as evidence. |

_critical · local · scope: epoch-view activation and promotion · assumptions: — · depends on: STR-007, SAF-004._ Legacy: DC-EPOCH-05, DC-EPOCH-17

#### `ORD-005` — _partial_

| Aspect | Location |
|--------|----------|
| **Source** | `docs/planning/epoch.define.consensus.md#batch-1` (approved by wdm33, 2026-11-08) |
| **Requirement** | **ForecastHorizonFollowsPromotion** — The era schedule's forecast horizon extends past an epoch boundary if and only if the next epoch's view has been durably promoted, and the extension is part of the same transition as the promotion; a header at a slot beyond the horizon is rejected as out of forecast range before any leader view is read. The extended schedule is derived state: it is computed from the durable activation record, the recovered promoted view and the committed network geometry and profile, never persisted as a separate authority, and recovery reconstructs a schedule identical to the one before the restart, whether or not the promotion had happened; no flag, clock or peer input affects it. |
| **Code** | `crates/ade_node/src/node_lifecycle.rs`; `crates/ade_core/src/consensus/era_schedule.rs`; `ci/ci_check_eview_forecast_crossing.sh` |
| **Tests** | `forecast_extends_only_on_promotion`, `warmstart_reconstruction_is_byte_identical_to_live_append`, `eraschedule_supports_adjacent_same_era_summaries` |
| **CI** | `ci/ci_check_eview_forecast_crossing.sh` (red at e563ad94) |
| **Claim** | unresolved — `e563ad94` (fail) — observed: tests: 3 passed, 0 failed, 0 ignored, 0 not found; CI scripts: 0 passed, 1 failed; failing CI: ci/ci_check_eview_forecast_crossing.sh — limits: (1) No IDSE cluster has declared acceptance obligations for this invariant yet (Doctrine §10); this record re-runs the checks carried from the legacy registry. Further limits: (2) No mutation witness exists yet for this critical invariant (Doctrine §28). (3) ci/ci_check_eview_forecast_crossing.sh fails at this revision and already failed before the IDSE migration (a3cef7f7): "FAIL: the extension does not atomically replace the owned schedule". Diagnosed as a stale or over-broad structural gate, not a demonstrated violation; the gate needs repair before it can serve as evidence. |

_critical · transactional · scope: era-schedule forecast, header validation · assumptions: A2 · depends on: DUR-001 · secondary: SAF._ Legacy: DC-EPOCH-15

### ORD — pipeline-production

#### `ORD-006` — _partial_

| Aspect | Location |
|--------|----------|
| **Source** | `docs/planning/pipeline-production.define.consensus.md#batch-1` (approved by wdm33, 2026-11-09) |
| **Requirement** | **PumpOrderPreservedUnderBackpressure** — Backpressure may delay a peer's events but never reorder them: the events the pump delivers are the same sequence it would deliver unstalled, frames decoded while parked join the same single FIFO as the rest of their chunk, and one frame is dispatched per loop iteration. Frames held while parked are bounded by a fixed, non-configurable constant, exceeding which fails that peer closed with a typed error; the bound holds because the pump issues no new chain-sync or block-fetch request while parked. |
| **Code** | `crates/ade_runtime/src/admission/wire_pump.rs`; `ci/ci_check_wire_liveness.sh` |
| **Tests** | `ce_wl_2_backpressure_preserves_event_order`, `ce_wl_3_deferred_frames_are_bounded_and_fail_closed` |
| **CI** | `ci/ci_check_wire_liveness.sh` |
| **Claim** | partial — `e563ad94` (pass) — observed: tests: 2 passed, 0 failed, 0 ignored, 0 not found; CI scripts: 1 passed, 0 failed — limits: (1) No IDSE cluster has declared acceptance obligations for this invariant yet (Doctrine §10); this record re-runs the checks carried from the legacy registry. Further limits: (2) No mutation witness exists yet for this critical invariant (Doctrine §28). |

_critical · local · scope: wire pump under backpressure · assumptions: — · depends on: STR-019, SEC-018._ Legacy: DC-PUMP-06, DC-PUMP-07

---

## FIN — Finality

_Doctrine §6: Conditions under which authoritative decisions or transitions become final._ 0 invariant(s).

_No FIN invariant is defined in the registry._

---

## CON — Conservation

_Doctrine §6: Preservation and accounting of quantities, resources, balances, and other conserved properties._ 5 invariant(s).

### CON — foundations

#### `CON-001` — _partial_

| Aspect | Location |
|--------|----------|
| **Source** | `docs/planning/foundations.define.consensus.md#batch-1` (approved by wdm33, 2026-11-08) |
| **Requirement** | **ValueConservation** — Every accepted ledger transition preserves value as the Conway ledger defines it. For a transaction applied with its scripts valid, consumed value (inputs, withdrawals, deposit refunds, minted value) equals produced value (outputs, fee, new deposits, treasury donation, burned value); for a transaction applied with its scripts invalid, only its collateral inputs are consumed, and they equal the collateral return plus the collateral collected as fee. Reserves, treasury, reward accounts, deposits and the fee pot change only through the protocol rules that authorize those movements. |
| **Code** | `crates/ade_ledger/src/value.rs`; `crates/ade_ledger/src/byron.rs` |
| **Tests** | `all_eras_replay_summary`, `byron_replay_all_1500`, `shelley_replay_all_1500`, `conway_conservation_full` |
| **CI** | `ci/ci_check_differential_divergence.sh` |
| **Claim** | partial — `e563ad94` (pass) — observed: tests: 4 passed, 0 failed, 0 ignored, 0 not found; CI scripts: 1 passed, 0 failed — limits: (1) No IDSE cluster has declared acceptance obligations for this invariant yet (Doctrine §10); this record re-runs the checks carried from the legacy registry. Further limits: (2) No mutation witness exists yet for this critical invariant (Doctrine §28). |

_critical · transactional · scope: ledger (transactions and epoch transitions) · assumptions: A2 · depends on: SAF-001 · secondary: VAL._ Legacy: T-CONSERV-01

### CON — ledger

#### `CON-002` — _partial_

| Aspect | Location |
|--------|----------|
| **Source** | `docs/planning/ledger.define.consensus.md#batch-1` (approved by wdm33, 2026-11-08) |
| **Requirement** | **DepositAccountingClosed** — Every certificate's deposit effect resolves through a closed, total function to exactly one of a new deposit, a refund, no deposit effect, or a rejection; each amount comes only from the certificate's explicit deposit field or the ledger's current protocol parameters, never from a default, configuration or fallback constant; an unrecognized or malformed certificate, or a state-dependent case Ade cannot account for, is rejected. |
| **Code** | `crates/ade_codec/src/conway/cert.rs`; `crates/ade_codec/src/error.rs`; `crates/ade_types/src/conway/cert.rs`; `crates/ade_ledger/src/cert_classify.rs`; `crates/ade_ledger/src/error.rs`; `corpus/conway_certs`; `crates/ade_ledger/src/pparams.rs`; `crates/ade_ledger/src/state.rs`; `crates/ade_ledger/src/fingerprint.rs` |
| **Tests** | `decode_total_over_tags_0_18`, `unknown_cert_tag_is_codec_error`, `removed_tag_5_6_is_not_valid_in_conway`, `malformed_cert_cbor_rejected`, `decode_is_replay_deterministic`, `class_mapping_is_total`, `legacy_unregistration_unresolved_is_unsupported_state_dependent`, `legacy_unregistration_resolves_recorded_deposit`, `pool_reregistration_is_neutral`, `pool_new_registration_charges_pool_deposit`, `conway_deposit_params_canonical_source`, `non_conway_state_has_no_conway_deposit_params`, `pparams_fingerprint_stable_for_non_conway`, `pparams_fingerprint_includes_conway_deposits_when_present` |
| **CI** | `ci/ci_check_conway_cert_classification_closed.sh`, `ci/ci_check_deposit_param_authority.sh` |
| **Claim** | partial — `e563ad94` (pass) — observed: tests: 14 passed, 0 failed, 0 ignored, 0 not found; CI scripts: 2 passed, 0 failed — limits: (1) No IDSE cluster has declared acceptance obligations for this invariant yet (Doctrine §10); this record re-runs the checks carried from the legacy registry. Further limits: (2) No mutation witness exists yet for this critical invariant (Doctrine §28). |

_critical · local · scope: transaction certificates, value accounting · assumptions: A2 · depends on: CON-001 · secondary: VAL._ Legacy: DC-TXV-06, DC-TXV-07

#### `CON-003` — _partial_

| Aspect | Location |
|--------|----------|
| **Source** | `docs/planning/ledger.define.consensus.md#batch-1` (approved by wdm33, 2026-11-08) |
| **Requirement** | **CollateralFromResolvedInputs** — The ADA a phase-2-invalid transaction consumes equals the sum of its resolved collateral inputs' values minus its collateral return; an unresolved collateral input is a typed refusal, never zero or a skipped contribution, and the declared total-collateral field is never used as the value. The accumulator receives these values from the UTxO authority through a resolver; it neither owns nor rebuilds a UTxO. |
| **Code** | `crates/ade_ledger/src/collateral.rs`; `crates/ade_runtime/src/chaindb/reduced_utxo_checkpoint.rs`; `ci/ci_check_collateral_balance_resolver.sh` |
| **Tests** | `multiple_collateral_inputs_sum_deterministically`, `the_collateral_return_is_subtracted_exactly_once`, `an_unresolved_collateral_input_is_a_typed_refusal_not_zero`, `one_unresolved_input_among_several_still_refuses`, `a_return_exceeding_the_collateral_fails_closed`, `the_refusal_is_replay_identical` |
| **CI** | `ci/ci_check_collateral_balance_resolver.sh` |
| **Claim** | partial — `e563ad94` (pass) — observed: tests: 6 passed, 0 failed, 0 ignored, 0 not found; CI scripts: 1 passed, 0 failed — limits: (1) No IDSE cluster has declared acceptance obligations for this invariant yet (Doctrine §10); this record re-runs the checks carried from the legacy registry. Further limits: (2) No mutation witness exists yet for this critical invariant (Doctrine §28). |

_critical · local · scope: collateral accounting · assumptions: A2 · depends on: CON-001 · secondary: VAL._ Legacy: DC-LEDGER-PHASE2-02

### CON — epoch

#### `CON-004` — _partial_

| Aspect | Location |
|--------|----------|
| **Source** | `docs/planning/epoch.define.consensus.md#batch-1` (approved by wdm33, 2026-11-08) |
| **Requirement** | **ProposalDepositReturn** — At each epoch boundary, every governance proposal that leaves the proposal set (enacted, expired, or removed because a sibling or ancestor was enacted or expired) has its deposit returned exactly once: to its return reward account if that account is registered, otherwise to the treasury. |
| **Code** | _(no enforcement found — gap)_ |
| **Tests** | `ade_ledger::epoch_accumulator::tests` (test module), `ade_ledger::ledgerdb_state::tip_tests::v6_commitment_is_deterministic_and_binds_gov`, `ade_ledger tests/ledgerdb_nonutxo_hermetic.rs::happy_minimal_state_decodes_all_fields`, `ade_runtime::mithril_native_assembly::tests::native_assembly_maps_each_field_from_its_source`, `ade_runtime::chaindb::epoch_accumulator_store::tests::governance_import_gate_rejects_absent_but_allows_empty` |
| **CI** | `ci/ci_check_gov_proposal_capture.sh` (red at e563ad94) |
| **Claim** | unresolved — `e563ad94` (fail) — observed: tests: 4 passed, 0 failed, 0 ignored, 1 not found; CI scripts: 0 passed, 1 failed; failing CI: ci/ci_check_gov_proposal_capture.sh — limits: (1) No IDSE cluster has declared acceptance obligations for this invariant yet (Doctrine §10); this record re-runs the checks carried from the legacy registry. Further limits: (2) No mutation witness exists yet for this critical invariant (Doctrine §28). (3) 1 named test(s) were not found in the test output at this revision: ade_ledger::epoch_accumulator::tests (s3_live_proposal_captured_with_identity_epoch_and_expiry, s3_two_proposals_one_tx_get_sequential_indices_same_txid, s3_vote_on_tracked_proposal_is_terminal, s3_vote_on_untracked_proposal_is_carried_forward, s3_cross_tx_same_block_vote_on_just_submitted_proposal_is_terminal, s3_invalid_tx_carrying_proposal_is_fail_closed, s3_invalid_tx_carrying_vote_is_fail_closed, s3_malformed_field20_is_fail_closed, s3_malformed_field19_is_fail_closed, s3_block_without_governance_fields_is_noop, s3_capture_skips_non_gov_fields_and_is_replay_equivalent, s3_unproven_zero_lifetime_refuses_to_fabricate_expiry, s3_gov_state_none_is_untracked_and_skipped) (4) ci/ci_check_gov_proposal_capture.sh fails at this revision and already failed before the IDSE migration (a3cef7f7): "FAIL: VoteOnTrackedProposal terminal (vote tripwire) is missing". Diagnosed as a stale or over-broad structural gate, not a demonstrated violation; the gate needs repair before it can serve as evidence. (5) Corrects this invariant's previous evidence record at the same revision: the earlier record's test matching missed references written as '<crate> tests/<file>.rs::<name>' or '<crate>::<path>'; the test outcomes are the same run's (docs/evidence/idse-m2-reevidence.json). |

_critical · transactional · scope: Conway governance deposits · assumptions: A2 · depends on: VAL-021, CON-001 · secondary: VAL._ Legacy: DC-GOV-01

#### `CON-005` — _partial_

| Aspect | Location |
|--------|----------|
| **Source** | `docs/planning/epoch.define.consensus.md#batch-1` (approved by wdm33, 2026-11-08) |
| **Requirement** | **DonationPot** — Ade holds exactly one treasury-donation pot with exactly three transitions: a phase-2-valid Conway transaction adds its donation, a phase-2-invalid one adds nothing, and the epoch boundary moves the whole pot into the treasury exactly once, as its last treasury step, and empties it. Every bootstrap takes the pot from the certified snapshot's state and refuses a snapshot without it, never defaulting it to zero; the pot is never an input to ratification, enactment or withdrawal affordability; and an overflow is a typed terminal, never a saturation. |
| **Code** | `crates/ade_ledger/src/state.rs`; `crates/ade_ledger/src/epoch_accumulator.rs`; `crates/ade_ledger/src/rules.rs`; `crates/ade_ledger/src/governance.rs`; `crates/ade_ledger/src/ledgerdb_state.rs`; `crates/ade_runtime/src/mithril_native_assembly.rs`; `crates/ade_ledger/src/snapshot/epoch_state.rs`; `ci/ci_check_donation_pot.sh` |
| **Tests** | `lv6_a_valid_txs_donation_is_credited_and_an_invalid_txs_is_not`, `lv6_the_real_epoch_305_blocks_carry_500_000_each`, `lv6_the_real_pair_folds_into_the_pot_and_the_boundary_moves_it`, `lv6_the_boundary_moves_the_whole_pot_once_and_zeroes_it`, `lv6_donations_and_unclaimed_deposits_both_reach_the_treasury`, `lv6_the_pot_never_funds_a_treasury_withdrawal`, `lv6_a_pot_that_would_overflow_the_treasury_is_a_terminal`, `codec_rejects_pre_lv6_store_rebootstrap_required`, `a_seed_with_donations_imports_its_donation_pot`, `the_seed_commitment_binds_the_donation_pot`, `a_utxo_state_without_a_donation_pot_is_refused`, `lv6_a_donation_pot_survives_a_warm_restart_byte_identically` |
| **CI** | `ci/ci_check_donation_pot.sh`, `ci/ci_check_store_semantics_lock.sh` |
| **Claim** | partial — `e563ad94` (pass) — observed: tests: 12 passed, 0 failed, 0 ignored, 0 not found; CI scripts: 2 passed, 0 failed — limits: (1) No IDSE cluster has declared acceptance obligations for this invariant yet (Doctrine §10); this record re-runs the checks carried from the legacy registry. Further limits: (2) No mutation witness exists yet for this critical invariant (Doctrine §28). |

_critical · transactional · scope: treasury donations · assumptions: A2 · depends on: CON-001, VAL-016._ Legacy: DC-EPOCH-45

---

## DET — Determinism

_Doctrine §6: Reproducible computations, deterministic transitions, and replay consistency._ 26 invariant(s).

### DET — foundations

#### `DET-001` — _partial_

| Aspect | Location |
|--------|----------|
| **Source** | `docs/planning/foundations.define.consensus.md#batch-1` (approved by wdm33, 2026-11-08) |
| **Requirement** | **ReplayByteIdentity** — Replaying the same canonical starting state with the same ordered canonical inputs, protocol version and recorded context produces byte-identical authoritative outputs, including the ledger-state fingerprint, persisted state encodings, the consensus event stream, declared effects and every validity verdict. |
| **Code** | `crates/ade_ledger/src/rules.rs`; `crates/ade_core/src/consensus/encoding.rs`; `crates/ade_runtime/src/consensus/chain_selector.rs`; `crates/ade_testkit/src/consensus/stream_replay.rs`; `crates/ade_testkit/src/validity/replay.rs`; `crates/ade_testkit/src/tx_validity`; `crates/ade_ledger/src/state.rs`; `crates/ade_ledger/src/fingerprint.rs`; `crates/ade_testkit/src/validity/adversarial.rs`; `crates/ade_testkit/src/tx_validity/adversarial.rs`; `crates/ade_ledger/src/receive/reducer.rs`; `crates/ade_runtime/src/receive/events_to_state.rs`; `crates/ade_runtime/src/receive/in_memory_chain_write.rs` |
| **Tests** | `apply_block_deterministic`, `byron_determinism`, `shelley_determinism`, `allegra_determinism`, `mary_determinism`, `alonzo_determinism`, `babbage_determinism`, `conway_determinism`, `layout_is_stable`, `roundtrip_empty_state`, `roundtrip_genesis_state`, `roundtrip_populated_state`, `roundtrip_chain_event_all_variants`, `roundtrip_chain_selection_reject_all_variants`, `op_cert_counter_map_iteration_is_deterministic`, `synthetic_session_replays_identically`, `synthetic_session_event_kinds_match_corpus`, `synthetic_session_final_state_matches_corpus`, `op_cert_regression_header_rejected_mid_stream`, `verdict_stream_replays_identically`, `tx_verdict_stream_replays_identically`, `apply_block_byron_ebb_passes_through`, `all_eras_determinism_summary`, `utxo_state_deterministic`, `boundary_fingerprint_matches_pins`, `no_mutation_is_ever_valid`, `adversarial_replays_identically`, `receive_apply_replay_byte_identical_over_corpus`, `receive_session_transcript_replay_byte_identical` |
| **CI** | `ci/ci_check_forbidden_patterns.sh`, `ci/ci_check_ledger_determinism.sh`, `ci/ci_check_consensus_closed_enums.sh`, `ci/ci_check_differential_divergence.sh`, `ci/ci_check_receive_reducer_closure.sh`, `ci/ci_check_receive_replay_purity.sh` |
| **Claim** | partial — `e563ad94` (pass) — observed: tests: 20 passed, 0 failed, 0 ignored, 8 not found; CI scripts: 6 passed, 0 failed — limits: (1) No IDSE cluster has declared acceptance obligations for this invariant yet (Doctrine §10); this record re-runs the checks carried from the legacy registry. Further limits: (2) No mutation witness exists yet for this critical invariant (Doctrine §28). (3) 8 named test(s) were not found in the test output at this revision: byron_determinism, shelley_determinism, allegra_determinism, mary_determinism, alonzo_determinism, babbage_determinism, conway_determinism, all_eras_determinism_summary (4) Skipped as non-completing corpus-replay tests in this environment: boundary_fingerprint_matches_pins |

_critical · local · scope: authoritative core (ledger, consensus, block and transaction validity, persisted state) · assumptions: A1 · depends on: — · secondary: REC._ Legacy: T-DET-01

#### `DET-002` — _partial_

| Aspect | Location |
|--------|----------|
| **Source** | `docs/planning/foundations.define.consensus.md#batch-1` (approved by wdm33, 2026-11-08) |
| **Requirement** | **DesignatedByteAuthority** — Each authoritative hash, signature check and persisted or transmitted byte string uses its designated byte authority: protocol hashes and signatures (block header, block body, transaction ID, script and datum hashes) are computed over the bytes as they were received or produced on the wire, never over a re-encoding of decoded data; Ade's own commitments (state fingerprints and persisted records) are computed over the output of the single canonical encoder for that type. |
| **Code** | `crates/ade_codec/src/preserved.rs`; `crates/ade_ledger/src/block_validity/header_input.rs`; `crates/ade_ledger/src/tx_validity/phase1.rs` |
| **Tests** | `preserved_wire_bytes_returned_exactly`, `altered_body_rejected_by_hash_binding`, `tx_id_uses_preserved_bytes` |
| **CI** | `ci/ci_check_hash_uses_wire_bytes.sh` (red at e563ad94) |
| **Claim** | unresolved — `e563ad94` (fail) — observed: tests: 3 passed, 0 failed, 0 ignored, 0 not found; CI scripts: 0 passed, 1 failed; failing CI: ci/ci_check_hash_uses_wire_bytes.sh — limits: (1) No IDSE cluster has declared acceptance obligations for this invariant yet (Doctrine §10); this record re-runs the checks carried from the legacy registry. Further limits: (2) No mutation witness exists yet for this critical invariant (Doctrine §28). (3) ci/ci_check_hash_uses_wire_bytes.sh fails at this revision and already failed before the IDSE migration (a3cef7f7): "FAIL: Hash computation using canonical_bytes in ade_ledger:". Diagnosed as a stale or over-broad structural gate, not a demonstrated violation; the gate needs repair before it can serve as evidence. |

_critical · local · scope: codec, ledger, consensus, storage · assumptions: A2 · depends on: — · secondary: VAL._ Legacy: T-ENC-01

#### `DET-003` — _partial_

| Aspect | Location |
|--------|----------|
| **Source** | `docs/planning/foundations.define.consensus.md#batch-1` (approved by wdm33, 2026-11-08) |
| **Requirement** | **RoundTripIdentity** — For every byte string that decodes successfully, encoding the decoded value yields exactly the original bytes. |
| **Code** | `crates/ade_codec/src/preserved.rs`; `crates/ade_network/src/codec/handshake.rs`; `crates/ade_network/src/codec/n2c_handshake.rs`; `crates/ade_network/src/codec/chain_sync.rs`; `crates/ade_network/src/codec/block_fetch.rs`; `crates/ade_network/src/codec/tx_submission.rs`; `crates/ade_network/src/codec/keep_alive.rs`; `crates/ade_network/src/codec/peer_sharing.rs`; `crates/ade_network/src/codec/local_chain_sync.rs`; `crates/ade_network/src/codec/local_tx_submission.rs`; `crates/ade_network/src/codec/local_state_query.rs`; `crates/ade_network/src/codec/local_tx_monitor.rs` |
| **Tests** | `full_corpus_round_trip::all_42_blocks_round_trip_byte_identical`, `full_corpus_round_trip::all_42_blocks_fields_match_reference`, `codec::handshake::tests::roundtrip_every_variant`, `codec::n2c_handshake::tests::roundtrip_every_variant`, `codec::chain_sync::tests::roundtrip_every_variant`, `codec::block_fetch::tests::roundtrip_every_variant`, `codec::tx_submission::tests::roundtrip_every_variant`, `codec::keep_alive::tests::roundtrip_every_variant`, `codec::peer_sharing::tests::roundtrip_every_variant`, `codec::local_chain_sync::tests::roundtrip_every_variant`, `codec::local_tx_submission::tests::roundtrip_every_variant`, `codec::local_state_query::tests::roundtrip_every_variant`, `codec::local_tx_monitor::tests::roundtrip_every_variant`, `handshake_real_capture_corpus::real_capture_round_trips_byte_identical`, `chain_sync_real_capture_corpus::real_capture_round_trips_byte_identical`, `block_fetch_real_capture_corpus::real_capture_round_trips_byte_identical`, `keep_alive_real_capture_corpus::real_capture_round_trips_byte_identical`, `peer_sharing_real_capture_corpus::real_capture_round_trips_byte_identical`, `n2c_handshake_real_capture_corpus::real_capture_round_trips_byte_identical` |
| **CI** | `ci/ci_check_cbor_round_trip.sh` |
| **Claim** | partial — `e563ad94` (pass) — observed: tests: 19 passed, 0 failed, 0 ignored, 0 not found; CI scripts: 1 passed, 0 failed — limits: (1) No IDSE cluster has declared acceptance obligations for this invariant yet (Doctrine §10); this record re-runs the checks carried from the legacy registry. Further limits: (2) No mutation witness exists yet for this critical invariant (Doctrine §28). |

_critical · local · scope: codec (ledger and mini-protocol encodings) · assumptions: A2 · depends on: DET-002 · secondary: VAL._ Legacy: T-ENC-03

#### `DET-004` — _partial_

| Aspect | Location |
|--------|----------|
| **Source** | `docs/planning/foundations.define.consensus.md#batch-1` (approved by wdm33, 2026-11-08) |
| **Requirement** | **HostIndependentResults** — No BLUE computation's result depends on floating point, on a collection's iteration order unless that order is canonical, or on the host's locale, timezone, pointer width or endianness. |
| **Code** | `crates/ade_ledger/src/lib.rs`; `crates/ade_core/src/consensus/vrf_cert.rs`; `crates/ade_plutus/src`; `crates/ade_ledger/src/consensus_input_extract.rs`; `crates/ade_ledger/src/consensus_view.rs`; `crates/ade_core/src/consensus/ledger_view.rs`; `crates/ade_ledger/src/tx_validity/transition.rs`; `crates/ade_ledger/src/tx_validity/phase1.rs` |
| **Tests** | `taylor_exp_cmp_le_zero_x_returns_false`, `taylor_exp_cmp_le_x_equals_one_returns_true`, `taylor_exp_cmp_le_monotone_in_x`, `is_leader_determinism`, `is_leader_known_vector_matches_reference`, `plutus_eval_is_deterministic`, `corpus_loads_and_is_self_consistent`, `extract_nonces_field_order`, `extract_nonces_requires_exactly_five`, `extract_nonces_is_deterministic`, `view_returns_corpus_pool_stake_and_vrf_keyhash`, `view_unknown_pool_returns_none`, `view_unknown_epoch_returns_none`, `view_is_pure`, `tx_validity_is_deterministic` |
| **CI** | `ci/ci_check_forbidden_patterns.sh`, `ci/ci_check_no_float_in_consensus.sh`, `ci/ci_check_plutus_eval_purity.sh`, `ci/ci_check_consensus_closed_enums.sh` |
| **Claim** | partial — `e563ad94` (pass) — observed: tests: 14 passed, 0 failed, 0 ignored, 1 not found; CI scripts: 4 passed, 0 failed — limits: (1) No IDSE cluster has declared acceptance obligations for this invariant yet (Doctrine §10); this record re-runs the checks carried from the legacy registry. Further limits: (2) No mutation witness exists yet for this critical invariant (Doctrine §28). (3) 1 named test(s) were not found in the test output at this revision: plutus_eval_is_deterministic |

_critical · local · scope: BLUE core paths · assumptions: — · depends on: STR-001._ Legacy: T-CORE-02, T-COLL-01, T-PLATFORM-01

#### `DET-005` — _partial_

| Aspect | Location |
|--------|----------|
| **Source** | `docs/planning/foundations.define.consensus.md#batch-1` (approved by wdm33, 2026-11-08); 1 recorded redefinition(s) in the registry |
| **Requirement** | **StructuredDeterministicFailures** — Every consensus-relevant failure, whether a rejection a transition returns or an internal inconsistency that triggers the halt of SAF-002, is reported as a value of a closed, structured type that carries no free-form text and no host-dependent content; given equivalent prior state, canonical input, protocol version and explicit context, two executions report byte-identical failures. |
| **Code** | `crates/ade_ledger/src/error.rs`; `crates/ade_plutus/src/tx_eval.rs`; `crates/ade_ledger/src/plutus_eval.rs` |
| **Tests** | `ledger_error_equality`, `conservation_error_display`, `codec_error_conversion`, `under_declared_ex_units_must_reject`, `failing_validator_must_reject`, `extraneous_redeemer_must_reject`, `declared_ex_units_array_form_parsed_by_pointer`, `declared_ex_units_conway_map_form_parsed_by_pointer`, `extract_redeemer_fields_reads_pointer_and_ex_units`, `aiken_fixture_tx_evaluates_end_to_end` |
| **CI** | `ci/ci_check_plutus_budget_cap.sh` |
| **Claim** | partial — `e563ad94` (pass) — observed: tests: 3 passed, 0 failed, 0 ignored, 7 not found; CI scripts: 1 passed, 0 failed — limits: (1) No IDSE cluster has declared acceptance obligations for this invariant yet (Doctrine §10); this record re-runs the checks carried from the legacy registry. Further limits: (2) 7 named test(s) were not found in the test output at this revision: under_declared_ex_units_must_reject, failing_validator_must_reject, extraneous_redeemer_must_reject, declared_ex_units_array_form_parsed_by_pointer, declared_ex_units_conway_map_form_parsed_by_pointer, extract_redeemer_fields_reads_pointer_and_ex_units, aiken_fixture_tx_evaluates_end_to_end |

_standard · local · scope: ledger, consensus and codec failures, and the halt reports of #8 · assumptions: A1 · depends on: STR-001 · secondary: OBS._ Legacy: T-ERR-01, CN-META-02

#### `DET-006` — _partial_

| Aspect | Location |
|--------|----------|
| **Source** | `docs/planning/foundations.define.consensus.md#batch-1` (approved by wdm33, 2026-11-08) |
| **Requirement** | **BuildInvariantSemantics** — No cargo feature, cfg flag, build profile or optimization level changes an authoritative result or a persisted byte. |
| **Code** | `crates/ade_ledger/src/lib.rs` |
| **Tests** | _(no tests found — gap)_ |
| **CI** | `ci/ci_check_no_semantic_cfg.sh` |
| **Claim** | partial — `e563ad94` (pass) — observed: tests: 0 passed, 0 failed, 0 ignored, 0 not found; CI scripts: 1 passed, 0 failed — limits: (1) No IDSE cluster has declared acceptance obligations for this invariant yet (Doctrine §10); this record re-runs the checks carried from the legacy registry. Further limits: (2) No mutation witness exists yet for this critical invariant (Doctrine §28). |

_critical · local · scope: BLUE core paths and persisted formats · assumptions: — · depends on: STR-001 · secondary: EVO._ Legacy: T-BUILD-01, CN-BUILD-01

### DET — codec-wire-crypto

#### `DET-007` — _partial_

| Aspect | Location |
|--------|----------|
| **Source** | `docs/planning/codec-wire-crypto.define.consensus.md#batch-1` (approved by wdm33, 2026-11-08) |
| **Requirement** | **SingleProjectEncoding** — Every byte surface Ade defines that replay compares or that is persisted (state fingerprints, persisted records, verdict and event streams) has exactly one canonical encoding, produced by one encoder; no surface is written by two encoders or in two encodings of the same value. |
| **Code** | `crates/ade_ledger/src/snapshot` |
| **Tests** | `snapshot::chain_dep::tests::chain_dep_encode_deterministic_across_runs`, `snapshot::utxo_state::tests::utxo_state_encode_deterministic_across_runs`, `snapshot::cert_state::tests::cert_state_encode_deterministic_across_runs`, `snapshot::epoch_state::tests::epoch_state_encode_deterministic_across_runs`, `snapshot::gov_state::tests::pparams_encode_deterministic_across_runs`, `snapshot::gov_state::tests::gov_state_encode_deterministic_across_runs`, `snapshot::ledger::tests::ledger_state_encode_deterministic_across_runs`, `snapshot::framing::tests::snapshot_encode_deterministic_across_runs`, `snapshot::framing::tests::round_trip_via_fingerprint_combined` |
| **CI** | `ci/ci_check_snapshot_encoder_closure.sh` |
| **Claim** | partial — `e563ad94` (pass) — observed: tests: 9 passed, 0 failed, 0 ignored, 0 not found; CI scripts: 1 passed, 0 failed — limits: (1) No IDSE cluster has declared acceptance obligations for this invariant yet (Doctrine §10); this record re-runs the checks carried from the legacy registry. Further limits: (2) No mutation witness exists yet for this critical invariant (Doctrine §28). |

_critical · local · scope: Ade-defined replay and persisted surfaces · assumptions: A1 · depends on: DET-001, DET-002 · secondary: REC._ Legacy: CN-WIRE-02

### DET — ledger

#### `DET-008` — _partial_

| Aspect | Location |
|--------|----------|
| **Source** | `docs/planning/ledger.define.consensus.md#batch-1` (approved by wdm33, 2026-11-08) |
| **Requirement** | **PlutusEvaluationDeterministic** — Evaluating a Plutus script with the same script, Plutus language version, arguments, script context, cost model and initial execution budget always gives the same result and the same execution-budget consumption, on every run and every host. |
| **Code** | `crates/ade_plutus/src/tx_eval.rs`; `crates/ade_plutus/src/evaluator.rs`; `crates/ade_testkit/tests/plutus_conformance.rs`; `docs/evidence/plutus-conformance-manifest.toml` |
| **Tests** | `plutus_eval_is_deterministic`, `plutus_conformance_evaluation_suite` |
| **CI** | `ci/ci_check_plutus_eval_purity.sh`, `ci/ci_check_plutus_conformance.sh` |
| **Claim** | partial — `e563ad94` (pass) — observed: tests: 0 passed, 0 failed, 0 ignored, 2 not found; CI scripts: 2 passed, 0 failed — limits: (1) No IDSE cluster has declared acceptance obligations for this invariant yet (Doctrine §10); this record re-runs the checks carried from the legacy registry. Further limits: (2) No mutation witness exists yet for this critical invariant (Doctrine §28). (3) 2 named test(s) were not found in the test output at this revision: plutus_eval_is_deterministic, plutus_conformance_evaluation_suite |

_critical · local · scope: ade_plutus evaluation · assumptions: — · depends on: DET-004._ Legacy: DC-PLUTUS-01, CN-PLUTUS-01

### DET — epoch

#### `DET-009` — _partial_

| Aspect | Location |
|--------|----------|
| **Source** | `docs/planning/epoch.define.consensus.md#batch-1` (approved by wdm33, 2026-11-08) |
| **Requirement** | **RewindReplayEquivalence** — Refolding the epoch accumulator from a settled rewind point produces state byte-identical to folding from the bootstrap baseline over the same canonical chain; re-deriving an epoch boundary over the same canonical source prefix re-seals a frozen leadership object byte-identical to the one first sealed; and no leadership object or activation record derived from a prefix that a rollback abandoned is ever reused. |
| **Code** | `crates/ade_runtime/src/chaindb/epoch_accumulator_advance.rs`; `crates/ade_runtime/src/chaindb/epoch_accumulator_store.rs`; `ci/ci_check_accumulator_refold_bound.sh`; `crates/ade_node/src/node_lifecycle.rs`; `crates/ade_ledger/src/epoch_accumulator.rs`; `ci/ci_check_eview_refold_reseal.sh` |
| **Tests** | `refold_from_settled_point_equals_fold_from_bootstrap`, `a_refold_reseals_frozen_leadership_byte_identically` |
| **CI** | `ci/ci_check_accumulator_refold_bound.sh`, `ci/ci_check_eview_refold_reseal.sh` |
| **Claim** | partial — `e563ad94` (pass) — observed: tests: 2 passed, 0 failed, 0 ignored, 0 not found; CI scripts: 2 passed, 0 failed — limits: (1) No IDSE cluster has declared acceptance obligations for this invariant yet (Doctrine §10); this record re-runs the checks carried from the legacy registry. Further limits: (2) No mutation witness exists yet for this critical invariant (Doctrine §28). |

_critical · local · scope: epoch accumulator refold, leadership sealing · assumptions: A1, A4 · depends on: DET-001 · secondary: REC._ Legacy: DC-EPOCH-31, DC-EPOCH-33

### DET — consensus

#### `DET-010` — _partial_

| Aspect | Location |
|--------|----------|
| **Source** | `docs/planning/consensus.define.consensus.md#batch-1` (approved by wdm33, 2026-11-08) |
| **Requirement** | **RollbackAndReplayEquivalence** — Rolling back to a point, or restoring a snapshot and replaying forward from it, yields state byte-identical to applying the same blocks directly from the bootstrap baseline; snapshots and replay are caches of the direct application, never a separate authority, and a replay over an epoch boundary goes through the one epoch-boundary transition. |
| **Code** | `crates/ade_runtime/src/recovery/mod.rs`; `crates/ade_runtime/src/recovery/restart.rs`; `crates/ade_core/src/consensus/rollback.rs`; `crates/ade_ledger/src/rollback/materialize.rs` |
| **Tests** | `recover_from_snapshot_and_replay_forward`, `recover_from_genesis_when_no_snapshot`, `no_starting_point_error`, `snapshot_with_no_post_blocks_is_ok`, `rollback_equivalent_to_truncated_replay`, `rollback_is_deterministic`, `rollback_within_k_succeeds`, `rollback_before_immutable_tip_rejected`, `rollback_event_bytes_are_stable`, `consensus::rollback::tests::rollback_to_one_below_immutable_rejected`, `consensus::rollback::tests::rollback_preserves_immutable_tip`, `materialize_with_snapshot_at_target_returns_snapshot_state`, `materialize_with_snapshot_below_target_replays_forward`, `materialize_replay_forward_equals_direct_apply`, `materialize_fails_closed_on_invalid_block` |
| **CI** | `ci/ci_check_recovery_contract.sh`, `ci/ci_check_consensus_closed_enums.sh`, `ci/ci_check_rollback_materialize_closure.sh` |
| **Claim** | partial — `e563ad94` (pass) — observed: tests: 15 passed, 0 failed, 0 ignored, 0 not found; CI scripts: 3 passed, 0 failed — limits: (1) No IDSE cluster has declared acceptance obligations for this invariant yet (Doctrine §10); this record re-runs the checks carried from the legacy registry. Further limits: (2) No mutation witness exists yet for this critical invariant (Doctrine §28). |

_critical · local · scope: rollback, snapshot restore, replay forward · assumptions: A1, A2 · depends on: DET-001, VAL-020 · secondary: REC._ Legacy: DC-CONS-06, DC-CONS-22

#### `DET-011` — _partial_

| Aspect | Location |
|--------|----------|
| **Source** | `docs/planning/consensus.define.consensus.md#batch-1` (approved by wdm33, 2026-11-08) |
| **Requirement** | **SnapshotRoundTrip** — Every persisted ledger and chain-dependent state snapshot begins with a closed version tag and embeds the state's fingerprint, encodes canonically, and decodes to a state with that same fingerprint; an unknown version or a fingerprint mismatch is rejected. |
| **Code** | `crates/ade_ledger/src/snapshot`; `crates/ade_runtime/src/rollback/persistent_cache.rs` |
| **Tests** | `snapshot::framing::tests::snapshot_round_trip`, `snapshot::framing::tests::round_trip_via_fingerprint_combined`, `snapshot::ledger::tests::encode_then_decode_roundtrips_via_fingerprint`, `rollback::persistent_cache::tests::persistent_cache_capture_then_nearest_le_round_trips`, `rollback::persistent_cache::tests::persistent_cache_matches_in_memory_cache_semantics`, `snapshot::chain_dep::tests::chain_dep_encode_deterministic_across_runs`, `snapshot::utxo_state::tests::utxo_state_encode_deterministic_across_runs`, `snapshot::cert_state::tests::cert_state_encode_deterministic_across_runs`, `snapshot::epoch_state::tests::epoch_state_encode_deterministic_across_runs`, `snapshot::gov_state::tests::pparams_encode_deterministic_across_runs`, `snapshot::gov_state::tests::gov_state_encode_deterministic_across_runs`, `snapshot::ledger::tests::ledger_state_encode_deterministic_across_runs`, `snapshot::framing::tests::snapshot_encode_deterministic_across_runs` |
| **CI** | `ci/ci_check_snapshot_encoder_closure.sh` |
| **Claim** | partial — `e563ad94` (pass) — observed: tests: 13 passed, 0 failed, 0 ignored, 0 not found; CI scripts: 1 passed, 0 failed — limits: (1) No IDSE cluster has declared acceptance obligations for this invariant yet (Doctrine §10); this record re-runs the checks carried from the legacy registry. Further limits: (2) No mutation witness exists yet for this critical invariant (Doctrine §28). |

_critical · local · scope: state snapshots · assumptions: A1 · depends on: DET-007, VAL-001 · secondary: VAL._ Legacy: DC-CONS-21

#### `DET-012` — _partial_

| Aspect | Location |
|--------|----------|
| **Source** | `docs/planning/consensus.define.consensus.md#batch-1` (approved by wdm33, 2026-11-08) |
| **Requirement** | **ConsensusIndependentOfArrival** — Every consensus decision (header validity, chain selection, leadership, rollback) is a function of the consensus state, the current selection, the set of candidates delivered, the protocol version and the recorded explicit context, the current slot included; with all of those equal, the decision is the same whatever order peers' messages arrived in, however tasks were scheduled, and whatever the wall clock or OS reported. |
| **Code** | `crates/ade_core/src/consensus/header_validate.rs`; `crates/ade_core/src/consensus/fork_choice.rs`; `crates/ade_core/src/consensus/candidate.rs` |
| **Tests** | `validate_replay_is_deterministic`, `consensus::header_validate::tests::pipeline_short_circuits_on_first_failure`, `consensus::header_validate::tests::nonce_contribution_uses_nonce_role_vrf_output_not_leader_role`, `replay_is_deterministic`, `reject_reason_bytes_are_stable` |
| **CI** | `ci/ci_check_no_density_in_fork_choice.sh`, `ci/ci_check_no_float_in_consensus.sh` |
| **Claim** | partial — `e563ad94` (pass) — observed: tests: 5 passed, 0 failed, 0 ignored, 0 not found; CI scripts: 2 passed, 0 failed — limits: (1) No IDSE cluster has declared acceptance obligations for this invariant yet (Doctrine §10); this record re-runs the checks carried from the legacy registry. Further limits: (2) No mutation witness exists yet for this critical invariant (Doctrine §28). |

_critical · local · scope: consensus decisions, shell/core boundary · assumptions: A1 · depends on: STR-001, STR-002, VAL-031 · secondary: STR._ Legacy: T-CONS-02, CN-CONS-05

### DET — consensus-inputs

#### `DET-013` — _partial_

| Aspect | Location |
|--------|----------|
| **Source** | `docs/planning/consensus-inputs.define.consensus.md#batch-1` (approved by wdm33, 2026-11-08) |
| **Requirement** | **ReducedCheckpointMirrorsLedger** — The reduced UTxO checkpoint is a projection of the ledger's own UTxO application: each block's delta removes exactly the inputs and adds exactly the outputs the ledger's application would, under the same keys, each reduced to its coin and stake reference; its certificate, delegation and governance state advance in block order through the ledger's own certificate processing, starting from the bootstrap certificate state; any step failure aborts the window without producing a result, and a partially applied window leaves the checkpoint marked incomplete, never producing a stake snapshot from partial data; the checkpoint can be rebuilt from the blocks by replay. |
| **Code** | `crates/ade_ledger/src/reduced_utxo.rs`; `crates/ade_runtime/src/chaindb/reduced_utxo_checkpoint.rs`; `ci/ci_check_eview_reduced_utxo_checkpoint.sh`; `crates/ade_ledger/src/reduced_advance.rs`; `crates/ade_ledger/src/rules.rs`; `ci/ci_check_eview_windowed_advance.sh`; `crates/ade_runtime/src/chaindb/reduced_window_driver.rs`; `ci/ci_check_eview_window_driver.sh` |
| **Tests** | `base_output_reduces_to_base_credential`, `enterprise_and_byron_are_non_contributing`, `pointer_output_is_non_contributing_at_conway`, `reduced_stake_ref_round_trips_canonically`, `decode_fails_closed_on_bad_tag_or_truncation`, `record_encoding_is_deterministic_and_canonical`, `build_then_query_and_complete`, `durable_across_reopen`, `replay_equivalent_two_builds_byte_identical`, `fingerprint_changes_with_content`, `crash_mid_build_is_incomplete_then_rebuilds`, `fresh_store_is_incomplete`, `reduced_delta_equals_reduce_of_track_utxo_on_real_conway_block`, `intra_block_chained_spend_cancels_phantom_matches_track_utxo`, `reduced_block_delta_is_deterministic`, `empty_block_yields_empty_delta`, `advance_cert_state_over_real_block_does_not_error`, `apply_block_delta_then_finalize`, `advance_over_real_conway_block_matches_build_from`, `empty_window_aggregates_bootstrap_state`, `real_conway_block_drive_equals_composed_pieces` |
| **CI** | `ci/ci_check_eview_reduced_utxo_checkpoint.sh`, `ci/ci_check_eview_windowed_advance.sh`, `ci/ci_check_eview_window_driver.sh` |
| **Claim** | partial — `e563ad94` (pass) — observed: tests: 21 passed, 0 failed, 0 ignored, 0 not found; CI scripts: 3 passed, 0 failed — limits: (1) No IDSE cluster has declared acceptance obligations for this invariant yet (Doctrine §10); this record re-runs the checks carried from the legacy registry. Further limits: (2) No mutation witness exists yet for this critical invariant (Doctrine §28). |

_critical · transactional · scope: reduced UTxO checkpoint · assumptions: A1 · depends on: VAL-037, REC-002, VAL-016 · secondary: REC._ Legacy: DC-EVIEW-04, DC-EVIEW-04b, DC-EVIEW-10

### DET — durability-recovery

#### `DET-014` — _partial_

| Aspect | Location |
|--------|----------|
| **Source** | `docs/planning/durability-recovery.define.consensus.md#batch-1` (approved by wdm33, 2026-11-08) |
| **Requirement** | **SnapshotCadenceDeterministic** — Whether a snapshot is taken at a block is a pure function of its slot, its block number, the fixed cadence parameters and the last snapshot taken, so the same chain history produces the same set of snapshot points; the cadence is not an operator setting. |
| **Code** | `crates/ade_runtime/src/rollback/cadence.rs`; `crates/ade_runtime/src/rollback/in_memory_cache.rs`; `crates/ade_runtime/src/rollback/chaindb_block_source.rs` |
| **Tests** | `should_snapshot_after_block_every_n_returns_true_at_cadence`, `should_snapshot_after_block_returns_false_off_cadence`, `should_snapshot_after_block_returns_false_when_already_at_or_after_slot`, `should_snapshot_after_block_is_pure`, `snapshot_cadence_default_is_100_blocks`, `in_memory_snapshot_cache_nearest_le_returns_largest_key`, `in_memory_snapshot_cache_iteration_is_btreemap_ordered` |
| **CI** | `ci/ci_check_snapshot_cadence_purity.sh` |
| **Claim** | partial — `e563ad94` (pass) — observed: tests: 7 passed, 0 failed, 0 ignored, 0 not found; CI scripts: 1 passed, 0 failed — limits: (1) No IDSE cluster has declared acceptance obligations for this invariant yet (Doctrine §10); this record re-runs the checks carried from the legacy registry. |

_standard · local · scope: snapshot scheduling · assumptions: — · depends on: STR-001._ Legacy: DC-STORE-07

#### `DET-015` — _partial_

| Aspect | Location |
|--------|----------|
| **Source** | `docs/planning/durability-recovery.define.consensus.md#batch-1` (approved by wdm33, 2026-11-08) |
| **Requirement** | **SnapshotHeaderChecks** — A snapshot's version tag is read and checked before any of its payload is decoded, and its fingerprint is blake2b-256 of the decoded state's canonical encoding, recomputed after decoding and compared with the stored one. |
| **Code** | `crates/ade_ledger/src/snapshot/framing.rs` |
| **Tests** | `snapshot::framing::tests::snapshot_round_trip`, `snapshot::framing::tests::decode_rejects_unknown_version`, `snapshot::framing::tests::decode_rejects_fingerprint_mismatch` |
| **CI** | `ci/ci_check_snapshot_encoder_closure.sh` |
| **Claim** | partial — `e563ad94` (pass) — observed: tests: 3 passed, 0 failed, 0 ignored, 0 not found; CI scripts: 1 passed, 0 failed — limits: (1) No IDSE cluster has declared acceptance obligations for this invariant yet (Doctrine §10); this record re-runs the checks carried from the legacy registry. Further limits: (2) No mutation witness exists yet for this critical invariant (Doctrine §28). |

_critical · local · scope: state snapshots · assumptions: — · depends on: DET-011 · secondary: VAL._ Legacy: DC-STORE-09

### DET — network

#### `DET-016` — _partial_

| Aspect | Location |
|--------|----------|
| **Source** | `docs/planning/network.define.consensus.md#batch-1` (approved by wdm33, 2026-11-08) |
| **Requirement** | **MiniProtocolStateMachinesDeterministic** — Each mini-protocol is an explicit state machine with only legal typed transitions, and each transition is a pure function of the prior protocol state, the incoming message, the negotiated protocol version, fixed configuration and recorded explicit inputs (served-chain updates and session events); given the same starting state and the same ordered peer messages, served-chain updates, session events and recorded context, the resulting protocol state and the outbound transcript are identical, and no ambient session state changes the result. |
| **Code** | `crates/ade_network/src/chain_sync/state.rs`; `crates/ade_network/src/chain_sync/agency.rs`; `crates/ade_network/src/chain_sync/signal.rs`; `crates/ade_network/src/chain_sync/transition.rs`; `crates/ade_network/src/block_fetch/state.rs`; `crates/ade_network/src/block_fetch/agency.rs`; `crates/ade_network/src/block_fetch/event.rs`; `crates/ade_network/src/block_fetch/transition.rs`; `crates/ade_network/src/tx_submission/state.rs`; `crates/ade_network/src/tx_submission/agency.rs`; `crates/ade_network/src/tx_submission/event.rs`; `crates/ade_network/src/tx_submission/transition.rs`; `crates/ade_network/src/keep_alive/state.rs`; `crates/ade_network/src/keep_alive/agency.rs`; `crates/ade_network/src/keep_alive/event.rs`; `crates/ade_network/src/keep_alive/transition.rs`; `crates/ade_network/src/peer_sharing/state.rs`; `crates/ade_network/src/peer_sharing/agency.rs`; `crates/ade_network/src/peer_sharing/event.rs`; `crates/ade_network/src/peer_sharing/transition.rs`; `crates/ade_network/src/n2c/local_chain_sync/state.rs`; `crates/ade_network/src/n2c/local_chain_sync/agency.rs`; `crates/ade_network/src/n2c/local_chain_sync/event.rs`; `crates/ade_network/src/n2c/local_chain_sync/transition.rs`; `crates/ade_network/src/n2c/local_tx_submission/state.rs`; `crates/ade_network/src/n2c/local_tx_submission/agency.rs`; `crates/ade_network/src/n2c/local_tx_submission/event.rs`; `crates/ade_network/src/n2c/local_tx_submission/transition.rs`; `crates/ade_network/src/n2c/local_state_query/state.rs`; `crates/ade_network/src/n2c/local_state_query/agency.rs`; `crates/ade_network/src/n2c/local_state_query/event.rs`; `crates/ade_network/src/n2c/local_state_query/transition.rs`; `crates/ade_network/src/n2c/local_tx_monitor/state.rs`; `crates/ade_network/src/n2c/local_tx_monitor/agency.rs`; `crates/ade_network/src/n2c/local_tx_monitor/event.rs`; `crates/ade_network/src/n2c/local_tx_monitor/transition.rs` |
| **Tests** | `chain_sync::transition::tests::idle_request_next_with_immediate_data_yields_can_await_then_roll_forward`, `chain_sync::transition::tests::idle_request_next_with_no_data_yields_must_reply_via_await`, `chain_sync::transition::tests::roll_forward_signal_carries_header_and_tip_byte_identical`, `chain_sync::transition::tests::roll_backward_signal_carries_point_and_tip_byte_identical`, `chain_sync::transition::tests::find_intersect_with_known_point_yields_intersected_signal`, `chain_sync::transition::tests::find_intersect_with_unknown_points_yields_no_intersection`, `chain_sync::transition::tests::client_done_terminates_session`, `chain_sync::transition::tests::illegal_message_in_idle_returns_error`, `chain_sync::transition::tests::wrong_agency_returns_error`, `chain_sync::transition::tests::version_gating_rejects_out_of_version_message`, `chain_sync::transition::tests::empty_find_intersect_points_returns_malformed`, `signal_trace_synthetic_divergence_corpus`, `block_fetch::transition::tests::idle_request_range_yields_busy_then_start_batch`, `block_fetch::transition::tests::busy_no_blocks_returns_to_idle_with_event`, `block_fetch::transition::tests::streaming_block_delivers_bytes_byte_identical`, `block_fetch::transition::tests::streaming_batch_done_returns_to_idle`, `block_fetch::transition::tests::multi_block_streaming_preserves_order`, `block_fetch::transition::tests::client_done_terminates_session`, `block_fetch::transition::tests::illegal_message_in_idle_returns_error`, `block_fetch::transition::tests::wrong_agency_returns_error`, `block_fetch::transition::tests::version_gating_rejects_out_of_version_message`, `block_fetch::transition::tests::inverted_range_returns_malformed`, `block_fetch::transition::tests::block_delivered_event_carries_bytes_verbatim`, `block_fetch_frame_corpus`, `tx_submission::transition::tests::init_handshake_opens_session`, `tx_submission::transition::tests::request_blocking_ids_then_reply_round_trips`, `tx_submission::transition::tests::request_non_blocking_ids_with_empty_reply_round_trips`, `tx_submission::transition::tests::request_txs_then_reply_delivers_bytes_byte_identical`, `tx_submission::transition::tests::request_txs_with_partial_reply_legal`, `tx_submission::transition::tests::blocking_reply_empty_is_malformed`, `tx_submission::transition::tests::reply_ids_overfill_is_malformed`, `tx_submission::transition::tests::request_txs_empty_is_malformed`, `tx_submission::transition::tests::reply_txs_overfill_is_malformed`, `tx_submission::transition::tests::done_terminates_session`, `tx_submission::transition::tests::illegal_message_in_init_returns_error`, `tx_submission::transition::tests::wrong_agency_returns_error`, `tx_submission::transition::tests::version_gating_rejects_out_of_version_message`, `tx_submission2_mempool_trace`, `keep_alive::transition::tests::client_ping_then_server_pong_round_trips`, `keep_alive::transition::tests::cookie_mismatch_returns_malformed`, `keep_alive::transition::tests::client_done_terminates_session`, `keep_alive::transition::tests::illegal_message_in_idle_returns_error`, `keep_alive::transition::tests::wrong_agency_returns_error`, `keep_alive::transition::tests::version_gating_rejects_out_of_version_message`, `peer_sharing::transition::tests::share_request_then_full_reply_round_trips`, `peer_sharing::transition::tests::share_request_then_empty_reply_is_legal`, `peer_sharing::transition::tests::reply_exceeds_amount_returns_malformed`, `peer_sharing::transition::tests::client_done_terminates_session`, `peer_sharing::transition::tests::illegal_message_in_idle_returns_error`, `peer_sharing::transition::tests::wrong_agency_returns_error`, `peer_sharing::transition::tests::version_gating_rejects_out_of_version_message`, `keep_alive_event_trace`, `peer_sharing_event_trace`, `n2c::local_chain_sync::transition::tests::local_chain_sync_request_next_then_roll_forward`, `n2c::local_chain_sync::transition::tests::local_chain_sync_roll_backward_signal`, `n2c::local_chain_sync::transition::tests::local_chain_sync_find_intersect_known_point`, `n2c::local_chain_sync::transition::tests::local_chain_sync_find_intersect_unknown`, `n2c::local_chain_sync::transition::tests::local_chain_sync_client_done_terminates`, `n2c::local_chain_sync::transition::tests::local_chain_sync_wrong_agency_returns_error`, `n2c::local_chain_sync::transition::tests::local_chain_sync_version_gating`, `n2c::local_tx_submission::transition::tests::submit_tx_then_accept_round_trips`, `n2c::local_tx_submission::transition::tests::submit_tx_then_reject_carries_reason_bytes`, `n2c::local_tx_submission::transition::tests::client_done_terminates`, `n2c::local_tx_submission::transition::tests::wrong_agency_returns_error`, `n2c::local_tx_submission::transition::tests::version_gating`, `n2c::local_state_query::transition::tests::acquire_then_acquired_transitions_to_acquired`, `n2c::local_state_query::transition::tests::acquire_then_failure_returns_to_idle`, `n2c::local_state_query::transition::tests::query_then_result_round_trips`, `n2c::local_state_query::transition::tests::release_returns_to_idle`, `n2c::local_state_query::transition::tests::re_acquire_from_acquired_transitions_to_acquiring`, `n2c::local_state_query::transition::tests::client_done_from_idle_terminates`, `n2c::local_state_query::transition::tests::client_done_from_acquired_terminates`, `n2c::local_state_query::transition::tests::wrong_agency_returns_error`, `n2c::local_tx_monitor::transition::tests::idle_acquire_then_acquired`, `n2c::local_tx_monitor::transition::tests::acquired_release_returns_to_idle`, `n2c::local_tx_monitor::transition::tests::acquired_await_acquire_re_acquires`, `n2c::local_tx_monitor::transition::tests::acquired_next_tx_then_reply_with_tx`, `n2c::local_tx_monitor::transition::tests::acquired_next_tx_then_reply_empty_mempool`, `n2c::local_tx_monitor::transition::tests::acquired_has_tx_then_reply_present`, `n2c::local_tx_monitor::transition::tests::acquired_has_tx_then_reply_absent`, `n2c::local_tx_monitor::transition::tests::acquired_get_sizes_then_reply`, `n2c::local_tx_monitor::transition::tests::acquired_get_measures_then_reply`, `n2c::local_tx_monitor::transition::tests::get_measures_rejected_below_v2`, `n2c::local_tx_monitor::transition::tests::client_done_terminates_from_idle`, `n2c::local_tx_monitor::transition::tests::wrong_agency_returns_error`, `n2c::local_tx_monitor::transition::tests::version_gating_rejects_out_of_version_message`, `local_chain_sync_event_trace`, `local_tx_submission_event_trace`, `local_state_query_event_trace`, `local_tx_monitor_event_trace` |
| **CI** | `ci/ci_check_mini_protocol_transition_purity.sh` |
| **Claim** | partial — `e563ad94` (pass) — observed: tests: 90 passed, 0 failed, 0 ignored, 0 not found; CI scripts: 1 passed, 0 failed — limits: (1) No IDSE cluster has declared acceptance obligations for this invariant yet (Doctrine §10); this record re-runs the checks carried from the legacy registry. Further limits: (2) No mutation witness exists yet for this critical invariant (Doctrine §28). |

_critical · local · scope: N2N and N2C mini-protocol state machines · assumptions: A1 · depends on: STR-002 · secondary: STR._ Legacy: DC-PROTO-01, CN-PROTO-01, DC-PROTO-06, CN-PROTO-02

#### `DET-017` — _partial_

| Aspect | Location |
|--------|----------|
| **Source** | `docs/planning/network.define.consensus.md#batch-1` (approved by wdm33, 2026-11-08) |
| **Requirement** | **ServerSessionDeterministic** — The serving side's per-session reducer is a pure function of the negotiated version, the peer's message sequence, the sequence of newly served blocks and the recorded session events, so the same inputs give byte-identical outbound frames. In each state where the server holds agency it returns a reply legal for that state as the reference protocol defines it (IntersectFound or IntersectNotFound after FindIntersect; RollForward, RollBackward or AwaitReply after RequestNext; StartBatch with blocks or NoBlocks after RequestRange) or a structured close; after AwaitReply it waits only until a recorded served-chain or session event makes a RollForward or RollBackward possible, never in an undefined state. |
| **Code** | `crates/ade_network/src/chain_sync/server.rs`; `crates/ade_network/src/block_fetch/server.rs`; `crates/ade_runtime/src/producer/broadcast_to_served.rs`; `crates/ade_runtime/src/producer/served_chain_lookups.rs` |
| **Tests** | `producer_chain_sync_serve_replays_byte_identical_over_corpus`, `producer_block_fetch_serve_replays_byte_identical_over_corpus`, `drain_and_admit_is_deterministic_over_arrival_sequence`, `session_transcript_replay_byte_identical`, `producer_chain_sync_serve_request_next_idle_yields_roll_forward_when_served_has_block`, `producer_chain_sync_serve_request_next_idle_yields_await_reply_when_served_empty`, `producer_chain_sync_serve_find_intersect_known_point_yields_intersect_found`, `producer_chain_sync_serve_find_intersect_unknown_point_yields_intersect_not_found`, `producer_chain_sync_serve_done_terminates_session`, `producer_chain_sync_serve_rejects_illegal_grammar_pair`, `producer_chain_sync_advance_tip_idle_yields_none`, `producer_chain_sync_advance_tip_can_await_yields_roll_forward_when_block_available`, `producer_chain_sync_advance_tip_must_reply_yields_roll_forward_when_block_available`, `producer_chain_sync_advance_tip_can_await_yields_none_when_cursor_at_head`, `producer_chain_sync_serve_roll_forward_header_equals_accepted_block_header_bytes` |
| **CI** | `ci/ci_check_chain_sync_server_closure.sh`, `ci/ci_check_block_fetch_server_closure.sh`, `ci/ci_check_broadcast_to_served_purity.sh` |
| **Claim** | partial — `e563ad94` (pass) — observed: tests: 15 passed, 0 failed, 0 ignored, 0 not found; CI scripts: 3 passed, 0 failed — limits: (1) No IDSE cluster has declared acceptance obligations for this invariant yet (Doctrine §10); this record re-runs the checks carried from the legacy registry. Further limits: (2) No mutation witness exists yet for this critical invariant (Doctrine §28). |

_critical · local · scope: chain-sync and block-fetch servers · assumptions: A1, A2 · depends on: DET-016 · secondary: LIV._ Legacy: DC-PROTO-07, DC-PROTO-08

#### `DET-018` — _partial_

| Aspect | Location |
|--------|----------|
| **Source** | `docs/planning/network.define.consensus.md#batch-1` (approved by wdm33, 2026-11-08) |
| **Requirement** | **SessionReplayEquivalence** — Two guarantees hold for the session layer. Replay: given the same starting state, negotiated version, inbound byte chunks and recorded timestamps, it produces byte-identical outbound frames and an identical sequence of delivered messages. Fragmentation: any two chunkings of the same inbound byte stream produce the same sequence of delivered messages. Each delivery is exactly one complete CBOR item of one mini-protocol, never one mux frame; incomplete trailing bytes are kept until complete, in a reassembly buffer per mini-protocol indexed by the closed set of accepted mini-protocols, so its membership, lookup and iteration are deterministic; malformed CBOR at an item boundary is a structured error, never dropped or partly accepted; order within each peer and mini-protocol is preserved; and the session layer reads no clock, timestamps entering only as values the caller supplies. |
| **Code** | `crates/ade_network/src/session/core.rs`; `crates/ade_network/src/session/demux.rs`; `crates/ade_network/src/session`; `crates/ade_runtime/src/orchestrator/keep_alive_session.rs`; `crates/ade_network/src/session/state.rs` |
| **Tests** | `crates/ade_network/tests/session_replay_equivalence.rs::session_replay_equivalence_holds`, `crates/ade_network/tests/session_replay_equivalence.rs::session_replay_corpus_builds_deterministically`, `crates/ade_network/src/session/demux.rs::tests::frame_buffer_two_runs_deterministic`, `crates/ade_runtime/src/orchestrator/keep_alive_session.rs::tests::keep_alive_session_emits_one_event_per_clock_tick`, `crates/ade_runtime/src/orchestrator/keep_alive_session.rs::tests::keep_alive_session_is_pure_under_deterministic_clock`, `crates/ade_runtime/src/orchestrator/keep_alive_session.rs::tests::keep_alive_cadence_default_is_60s`, `crates/ade_network/src/session/core.rs::tests::fragmented_chain_sync_message_assembles_one_deliver`, `crates/ade_network/src/session/core.rs::tests::fragmented_block_fetch_block_assembles_one_deliver`, `crates/ade_network/src/session/core.rs::tests::interleaved_chain_sync_and_block_fetch_fragments_stay_isolated`, `crates/ade_network/src/session/core.rs::tests::pipelined_two_chain_sync_messages_in_one_mux_frame_emit_two_delivers`, `crates/ade_network/src/session/core.rs::tests::truncated_then_complete_two_step_drain`, `crates/ade_network/src/session/core.rs::tests::proto_buffers_isolation_across_accepted_protocols`, `crates/ade_network/src/session/core.rs::tests::fragmented_replay_equivalence_two_runs_byte_identical`, `crates/ade_network/src/session/core.rs::tests::malformed_cbor_at_item_boundary_returns_session_error` |
| **CI** | `ci/ci_check_session_core_closure.sh`, `ci/ci_check_clock_seam.sh`, `ci/ci_check_session_proto_reassembly.sh` |
| **Claim** | partial — `e563ad94` (pass) — observed: tests: 14 passed, 0 failed, 0 ignored, 0 not found; CI scripts: 3 passed, 0 failed — limits: (1) No IDSE cluster has declared acceptance obligations for this invariant yet (Doctrine §10); this record re-runs the checks carried from the legacy registry. Further limits: (2) No mutation witness exists yet for this critical invariant (Doctrine §28). |

_critical · local · scope: session layer · assumptions: A1 · depends on: STR-002, STR-012 · secondary: ORD._ Legacy: DC-SESS-03, CN-SESS-04, DC-SESS-06, DC-SESS-05

### DET — node

#### `DET-019` — _partial_

| Aspect | Location |
|--------|----------|
| **Source** | `docs/planning/node.define.consensus.md#batch-1` (approved by wdm33, 2026-11-09) |
| **Requirement** | **SingleSlotAuthority** — The current slot is derived by one authority: the shell observes wall-clock time, and one conversion projects it over the network's complete timing calendar from system start, the calendar selected by the durable record's genesis hash and never by operator flags (operator-supplied network or genesis values are only fail-closed cross-checks). The authority is established only if the calendar is self-consistent (each segment boundary aligned to the previous segment's epoch length), reproduces the store's recorded epoch start and epoch length, places the seed point's slot inside the recorded seed epoch, and yields an anchor whose lineage verifies against the calendar; it is reconstructed from the bootstrap anchor slot, never persisted, and a forging start without the durable seed record fails closed; a schedule that does not cover system start is refused for both conversions, time to slot and slot to time. Only the slot crosses into the orchestrator core, which reads no clock, so replaying the core with a recorded clock and event stream gives byte-identical final state. |
| **Code** | `crates/ade_runtime/src/clock.rs`; `crates/ade_runtime/src/orchestrator/core.rs`; `crates/ade_node/src/node_lifecycle.rs`; `crates/ade_node/src/run_loop_planner.rs`; `crates/ade_core/src/consensus/era_schedule.rs`; `crates/ade_node/src/forge_timing.rs`; `crates/ade_node/src/operator_forge.rs`; `ci/ci_check_forge_slot_authority.sh` |
| **Tests** | `crates/ade_runtime/tests/orchestrator_replay_equivalence.rs::replay_equivalence_under_deterministic_clock_holds`, `crates/ade_runtime/tests/orchestrator_replay_equivalence.rs::replay_corpus_is_present_and_decodable`, `crates/ade_runtime/src/orchestrator/core.rs::tests::step_two_runs_produce_byte_identical_effects`, `crates/ade_runtime/src/clock.rs::tests::deterministic_clock_is_pure`, `crates/ade_runtime/src/orchestrator/leadership_session.rs::tests::leadership_session_slot_arithmetic_is_pure`, `plan_loop_step_forge_precedence_table_is_total`, `forge_slot_guard_none_is_due`, `forge_slot_guard_at_most_once_per_slot`, `forge_slot_guard_rejects_past_slot`, `relay_loop_forge_slot_derived_via_clock_seam`, `relay_loop_forge_tick_attempts_forge_advances_no_tip`, `relay_loop_without_producer_material_matches_nfd_relay`, `relay_loop_forge_two_runs_byte_identical`, `forge_tick_rotated_kes_period_skips_no_retroactive_sign`, `forge_tick_off_epoch_slot_fails_closed_local`, `ce_l2c_a2_committed_calendar_reproduces_the_durable_epoch_and_the_measured_slot`, `dropping_the_historical_timing_segment_is_refused_by_the_durable_binding`, `a_self_inconsistent_calendar_is_refused_at_the_transition`, `ce_l2c_a4_an_altered_timing_schedule_is_rejected`, `ce_l2c_a3_reconstruction_is_byte_identical_and_replayable`, `an_anchor_slot_outside_the_bootstrap_epoch_is_refused`, `preview_single_segment_calendar_uses_the_same_path`, `a_truncated_calendar_cannot_establish_a_timing_authority`, `the_durable_epoch_binding_pins_boundaries_not_slot_durations`, `ce_l2c_a1_the_live_store_facts_establish_and_convert_the_measured_instant`, `the_operator_cannot_choose_the_calendar`, `an_uncommitted_venue_fails_closed`, `the_operator_genesis_file_cross_checks_origin_and_active_slot_length`, `a_store_epoch_the_calendar_cannot_reproduce_fails_closed`, `reconstruction_is_byte_identical_across_restarts`, `the_timing_table_and_the_identity_registry_cannot_drift`, `ce_l2c_5_and_6_live_instant_derives_the_measured_slot_and_refuses_typed`, `node_forge_slot_drift_fails_closed`, `millis_to_slot_cannot_express_a_multi_length_venue` |
| **CI** | `ci/ci_check_clock_seam.sh`, `ci/ci_check_orchestrator_core_purity.sh`, `ci/ci_check_loop_planner_closed.sh`, `ci/ci_check_node_run_loop_containment.sh`, `ci/ci_check_genesis_consistency_fixture_present.sh`, `ci/ci_check_forge_slot_authority.sh` |
| **Claim** | partial — `e563ad94` (pass) — observed: tests: 34 passed, 0 failed, 0 ignored, 0 not found; CI scripts: 6 passed, 0 failed — limits: (1) No IDSE cluster has declared acceptance obligations for this invariant yet (Doctrine §10); this record re-runs the checks carried from the legacy registry. Further limits: (2) No mutation witness exists yet for this critical invariant (Doctrine §28). |

_critical · local · scope: slot clock, orchestrator core · assumptions: A1, A2 · depends on: STR-002, DUR-005 · secondary: STR._ Legacy: DC-NODE-03, DC-NODE-45, DC-NODE-05

### DET — pipeline-production

#### `DET-020` — _partial_

| Aspect | Location |
|--------|----------|
| **Source** | `docs/planning/pipeline-production.define.consensus.md#batch-1` (approved by wdm33, 2026-11-09) |
| **Requirement** | **ForgePureOverProducerTick** — Building a block is a pure function of one canonical producer tick (the slot, the forge base's ledger and chain-dependent state, the transaction snapshot, the protocol parameters and version, and the already-produced VRF proof, KES signature and operational certificate): it reads no clock, randomness, file, unordered collection or ambient state. Replaying the same tick stream over the same starting state yields byte-identical forged blocks; replay uses the recorded signed artifacts, never calls a signing function, and no replay corpus holds private key material. |
| **Code** | `crates/ade_ledger/src/producer/forge.rs`; `crates/ade_ledger/src/producer/state.rs`; `crates/ade_ledger/src/receive/reducer.rs`; `crates/ade_testkit/src/producer/replay.rs`; `crates/ade_testkit/src/producer/fixtures.rs` |
| **Tests** | `forge_block_pure_no_io`, `forge_block_replay_byte_identical`, `receive_apply_block_delivered_with_matching_header_admits` |
| **CI** | `ci/ci_check_forge_purity.sh` (red at e563ad94), `ci/ci_check_receive_reducer_closure.sh`, `ci/ci_check_no_private_keys_in_corpus.sh` |
| **Claim** | unresolved — `e563ad94` (fail) — observed: tests: 3 passed, 0 failed, 0 ignored, 0 not found; CI scripts: 2 passed, 1 failed; failing CI: ci/ci_check_forge_purity.sh — limits: (1) No IDSE cluster has declared acceptance obligations for this invariant yet (Doctrine §10); this record re-runs the checks carried from the legacy registry. Further limits: (2) No mutation witness exists yet for this critical invariant (Doctrine §28). (3) ci/ci_check_forge_purity.sh fails at this revision and already failed before the IDSE migration (a3cef7f7): "FAIL: Guard 2 (parallel is_leader definition outside leader_schedule.rs):". Diagnosed as a stale or over-broad structural gate, not a demonstrated violation; the gate needs repair before it can serve as evidence. |

_critical · local · scope: forge engine (ade_ledger::producer) · assumptions: A1 · depends on: DET-001, SEC-001 · secondary: STR._ Legacy: DC-CONS-13, DC-CONS-14

#### `DET-021` — _partial_

| Aspect | Location |
|--------|----------|
| **Source** | `docs/planning/pipeline-production.define.consensus.md#batch-1` (approved by wdm33, 2026-11-09) |
| **Requirement** | **ProducerCoordinatorReplay** — The producer coordinator is a deterministic reducer over canonical event streams: for the same initial coordinator state, ledger state, operational-certificate public metadata and complete ordered stream of coordinator events (slot ticks, forge results succeeded or failed, peer lifecycle events and every other coordinator input), it emits byte-identical effects and producer log events. Its forge request is a closed effect carrying the slot, KES period, ledger snapshot reference and chain tip; its state holds no signing key; and wall-clock timestamps and socket arrival order are shell metadata outside the compared streams. |
| **Code** | `crates/ade_runtime/src/producer/coordinator.rs` |
| **Tests** | `replay_byte_identity_across_two_runs` |
| **CI** | _(no CI enforcement — gap)_ |
| **Claim** | partial — `e563ad94` (pass) — observed: tests: 1 passed, 0 failed, 0 ignored, 0 not found; CI scripts: 0 passed, 0 failed — limits: (1) No IDSE cluster has declared acceptance obligations for this invariant yet (Doctrine §10); this record re-runs the checks carried from the legacy registry. Further limits: (2) No mutation witness exists yet for this critical invariant (Doctrine §28). |

_critical · local · scope: producer coordinator (node and produce modes) · assumptions: A1 · depends on: SEC-001, DET-001._ Legacy: DC-PROD-02

### DET — mithril-evidence

#### `DET-022` — _partial_

| Aspect | Location |
|--------|----------|
| **Source** | `docs/planning/mithril-evidence.define.consensus.md#batch-1` (approved by wdm33, 2026-11-09) |
| **Requirement** | **CliSeedImportDeterministic** — One importer converts a cardano-cli whole-UTxO JSON dump into Ade's canonical UTxO state and its fingerprint, which is blake2b-256 over the canonical CBOR map of the UTxO in ascending transaction-input order; re-importing the same JSON yields byte-identical state and fingerprint. An import either takes every entry of the dump or fails with a typed error: no entry is skipped, no partial seed is returned, and no per-call fallback exists. No second importer exists, and the importer is never reachable from the native route (SEC-030). |
| **Code** | `crates/ade_runtime/src/seed_import/importer.rs`; `crates/ade_runtime/src/seed_import/json.rs` |
| **Tests** | `crates/ade_runtime/src/seed_import/importer.rs::tests::utxo_seed_parses_minimal_two_entry_fixture`, `crates/ade_runtime/src/seed_import/importer.rs::tests::utxo_seed_two_imports_byte_identical`, `crates/ade_runtime/src/seed_import/importer.rs::tests::utxo_seed_btree_order_independent_of_json_order`, `crates/ade_runtime/src/seed_import/importer.rs::tests::utxo_seed_rejects_unparseable_json`, `crates/ade_runtime/src/seed_import/importer.rs::tests::utxo_seed_rejects_bad_txin_key`, `crates/ade_runtime/src/seed_import/importer.rs::tests::utxo_seed_rejects_bad_address`, `crates/ade_runtime/src/seed_import/importer.rs::tests::utxo_seed_inline_datum_entry_round_trips`, `crates/ade_runtime/src/seed_import/importer.rs::tests::utxo_seed_canonical_txout_address_extracted`, `crates/ade_runtime/src/seed_import/importer.rs::tests::utxo_seed_accepts_plutus_v1_reference_script`, `crates/ade_runtime/src/seed_import/importer.rs::tests::utxo_seed_accepts_plutus_v2_reference_script`, `crates/ade_runtime/src/seed_import/importer.rs::tests::utxo_seed_accepts_plutus_v3_reference_script`, `crates/ade_runtime/src/seed_import/importer.rs::tests::utxo_seed_accepts_simple_script_reference_script`, `crates/ade_runtime/src/seed_import/importer.rs::tests::utxo_seed_reference_script_changes_fingerprint`, `crates/ade_runtime/src/seed_import/importer.rs::tests::utxo_seed_reference_script_deterministic_across_two_imports`, `crates/ade_runtime/src/seed_import/importer.rs::tests::utxo_seed_rejects_unknown_script_type`, `crates/ade_runtime/src/seed_import/importer.rs::tests::utxo_seed_rejects_non_hex_cbor_hex`, `crates/ade_runtime/src/seed_import/importer.rs::tests::utxo_seed_rejects_missing_cbor_hex`, `crates/ade_runtime/src/seed_import/importer.rs::tests::utxo_seed_canonical_script_ref_encoder_known_vector`, `crates/ade_runtime/src/seed_import/json.rs::tests::parse_utxo_seed_json_reference_script_entry` |
| **CI** | `ci/ci_check_seed_import_closure.sh`, `ci/ci_check_seed_import_full_preprod_support.sh` |
| **Claim** | partial — `e563ad94` (pass) — observed: tests: 19 passed, 0 failed, 0 ignored, 0 not found; CI scripts: 2 passed, 0 failed — limits: (1) No IDSE cluster has declared acceptance obligations for this invariant yet (Doctrine §10); this record re-runs the checks carried from the legacy registry. Further limits: (2) No mutation witness exists yet for this critical invariant (Doctrine §28). |

_critical · local · scope: cardano-cli seed import (cardano-cli seed route, admission mode) · assumptions: A2 · depends on: DET-004, DET-007 · secondary: STR._ Legacy: CN-SEED-01, DC-SEED-01

#### `DET-023` — _partial_

| Aspect | Location |
|--------|----------|
| **Source** | `docs/planning/mithril-evidence.define.consensus.md#batch-1` (approved by wdm33, 2026-11-09) |
| **Requirement** | **PostSwitchContinuityVerdict** — After a fork-choice adoption at tip X, one pure reducer classifies Ade's own admitted lineage into a closed verdict (continues selected branch, diverged, broken lineage, dangling fork-choice win, insufficient evidence); it returns continues-selected-branch only if every block admitted after X chains from X through its validated previous-hash field, no diverged verdict follows X, and every win has a terminal event. Its inputs are Ade's admitted blocks and the applied switch point, never the peer's tip; the same inputs give a byte-identical verdict; the same implementation serves the replay test and the live gate; and nothing on the node's selection, admission or forge path reads it. |
| **Code** | `crates/ade_node/src/post_switch_continuity.rs`; `crates/ade_node/src/bin/post_switch_continuity.rs`; `crates/ade_node/src/admission_log`; `crates/ade_runtime/src/forward_sync/pump.rs`; `crates/ade_node/src/node_lifecycle.rs`; `crates/ade_node/src/admission/runner.rs` |
| **Tests** | `crates/ade_node/src/post_switch_continuity.rs::tests::continuity_ok_yields_continues_selected_branch`, `crates/ade_node/src/post_switch_continuity.rs::tests::broken_parent_link_yields_broken_lineage`, `crates/ade_node/src/post_switch_continuity.rs::tests::post_switch_diverged_yields_diverged`, `crates/ade_node/src/post_switch_continuity.rs::tests::win_without_terminal_yields_dangling`, `crates/ade_node/src/post_switch_continuity.rs::tests::continuity_verdict_ignores_peer_tip`, `crates/ade_node/src/post_switch_continuity.rs::tests::post_switch_continuity_replays_byte_identical`, `crates/ade_node/src/post_switch_continuity.rs::tests::release_window_passes_on_validated_prefix`, `crates/ade_node/src/post_switch_continuity.rs::tests::release_window_passes_on_agreed_descendant`, `crates/ade_node/src/post_switch_continuity.rs::tests::release_window_prefix_requires_a_followed_descendant`, `crates/ade_node/src/post_switch_continuity.rs::tests::release_window_terminal_outside_window_fails`, `crates/ade_node/tests/live_fork_choice_ai_s4bii.rs` (test file) |
| **CI** | `ci/ci_check_post_switch_convergence_window.sh` (red at e563ad94) |
| **Claim** | unresolved — `e563ad94` (fail) — observed: tests: 10 passed, 0 failed, 0 ignored, 1 not found; CI scripts: 0 passed, 1 failed; failing CI: ci/ci_check_post_switch_convergence_window.sh — limits: (1) No IDSE cluster has declared acceptance obligations for this invariant yet (Doctrine §10); this record re-runs the checks carried from the legacy registry. Further limits: (2) 1 named test(s) were not found in the test output at this revision: crates/ade_node/tests/live_fork_choice_ai_s4bii.rs (new_tip_prev == decoded.prev_hash fidelity assertion) (3) ci/ci_check_post_switch_convergence_window.sh fails at this revision and already failed before the IDSE migration (a3cef7f7): "nonzero exit". Diagnosed as a stale or over-broad structural gate, not a demonstrated violation; the gate needs repair before it can serve as evidence. |

_standard · local · scope: post-switch evidence gate · assumptions: A1 · depends on: LIV-006, OBS-009 · secondary: OBS._ Legacy: DC-EVIDENCE-05

### DET — resources-release

#### `DET-024` — _partial_

| Aspect | Location |
|--------|----------|
| **Source** | `docs/planning/resources-release.define.consensus.md#batch-1` (approved by wdm33, 2026-11-09) |
| **Requirement** | **MempoolReplay** — Replaying the same ordered ingress trace against the same base ledger state produces a byte-identical sequence of mempool states and admission outcomes. |
| **Code** | `crates/ade_testkit/src/mempool/ingress_replay.rs`; `crates/ade_ledger/src/mempool/ingress.rs`; `crates/ade_ledger/src/mempool/canonicalize.rs` |
| **Tests** | `ingress_admit_equals_direct_admit_on_b_track_corpus`, `b_track_adversarial_rejections_preserved_through_ingress`, `ingress_trace_replay_byte_identical`, `dependent_pair_through_ingress_admits_b_after_a`, `ingress_trace_source_invariant_n2n_vs_n2c`, `multi_peer_round_robin_by_sorted_peer_id`, `unsorted_input_canonicalizes_identically_to_sorted_input`, `two_interleavings_replay_byte_identical` |
| **CI** | `ci/ci_check_mempool_ingress_replay.sh` |
| **Claim** | partial — `e563ad94` (pass) — observed: tests: 2 passed, 0 failed, 0 ignored, 6 not found; CI scripts: 1 passed, 0 failed — limits: (1) No IDSE cluster has declared acceptance obligations for this invariant yet (Doctrine §10); this record re-runs the checks carried from the legacy registry. Further limits: (2) No mutation witness exists yet for this critical invariant (Doctrine §28). (3) 6 named test(s) were not found in the test output at this revision: ingress_admit_equals_direct_admit_on_b_track_corpus, b_track_adversarial_rejections_preserved_through_ingress, ingress_trace_replay_byte_identical, dependent_pair_through_ingress_admits_b_after_a, ingress_trace_source_invariant_n2n_vs_n2c, two_interleavings_replay_byte_identical |

_critical · local · scope: mempool · assumptions: A1 · depends on: DET-001, STR-022._ Legacy: DC-MEM-04

#### `DET-025` — _partial_

| Aspect | Location |
|--------|----------|
| **Source** | `docs/planning/resources-release.define.consensus.md#batch-1` (approved by wdm33, 2026-11-09) |
| **Requirement** | **FingerprintIndependentOfStorage** — The UTxO and ledger-state fingerprints, and every post-state, are independent of how the UTxO is stored and represented: in-memory and on-disk backends produce byte-identical replay from the same log and checkpoint; the fingerprint is computed by the canonical encoder over canonically encoded keys, never from a backend's iteration order or allocation addresses; a compact representation reads the same values and exposes the same bytes as the fully parsed one; and UTxO lookup returns owned values, so changing the backend changes no verdict, fingerprint or failure. |
| **Code** | `ci/ci_check_alloc_determinism_neutral.sh`; `crates/ade_node/src/main.rs`; `crates/ade_runtime/src/seed_import/importer.rs`; `crates/ade_ledger/src/utxo.rs`; `crates/ade_ledger/src/phase.rs`; `crates/ade_ledger/src/tx_validity/phase1.rs`; `ci/ci_check_utxo_lookup_owned.sh` |
| **Tests** | `streaming_matches_whole_buffer_across_fixtures`, `streaming_fingerprint_independent_of_textual_order`, `streaming_surfaces_conversion_error_not_swallowed`, `streaming_rejects_duplicate_txin_fail_closed`, `streaming_rejects_exact_duplicate_string_key_but_oracle_collapses`, `owned_lookup_returns_stored_value_and_does_not_mutate` |
| **CI** | `ci/ci_check_utxo_lookup_owned.sh` |
| **Claim** | partial — `e563ad94` (pass) — observed: tests: 6 passed, 0 failed, 0 ignored, 0 not found; CI scripts: 1 passed, 0 failed — limits: (1) No IDSE cluster has declared acceptance obligations for this invariant yet (Doctrine §10); this record re-runs the checks carried from the legacy registry. Further limits: (2) No mutation witness exists yet for this critical invariant (Doctrine §28). |

_critical · local · scope: UTxO storage and fingerprint · assumptions: — · depends on: DET-001, DET-004, DET-007._ Legacy: DC-MEM-05, DC-MEM-06, DC-MEM-08, DC-MEM-09

#### `DET-026` — _partial_

| Aspect | Location |
|--------|----------|
| **Source** | `docs/planning/resources-release.define.consensus.md#batch-1` (approved by wdm33, 2026-11-09) |
| **Requirement** | **UtxoCommitmentVersionedIncremental** — The UTxO component of the ledger fingerprint is a named commutative set commitment (Ristretto255 multiset hash over the canonical encodings of each input and output, domain-separated and version-tagged), versions never mixed in one fold; maintaining it incrementally per block, or reusing a cached component while the UTxO is unchanged (invalidated by any UTxO mutation), always equals the full recomputation byte for byte. Forward-sync and forge admissions take the post fingerprint from the cached component rather than a full recomputation, and convergence evidence reuses the post fingerprint the admission just computed. |
| **Code** | `crates/ade_crypto/src/utxo_set_commitment.rs`; `crates/ade_ledger/src/fingerprint.rs`; `crates/ade_runtime/src/chaindb/persistent.rs`; `crates/ade_runtime/src/chaindb/error.rs`; `ci/ci_check_utxo_fp_v2.sh`; `crates/ade_runtime/src/forward_sync/reducer.rs`; `crates/ade_node/src/node_lifecycle.rs`; `ci/ci_check_forward_sync_fp_cache.sh` |
| **Tests** | `order_independent`, `add_remove_is_exact_inverse`, `binds_value_not_just_key`, `golden_empty_digest`, `golden_single_entry_digest`, `golden_two_entry_digest`, `v1_and_v2_utxo_components_differ_only_the_utxo_changes`, `fingerprint_v2_is_deterministic`, `fingerprint_v2_utxo_is_insertion_order_independent`, `incremental_v2_equals_full_recompute_after_each_block`, `incremental_produce_spend_is_exact_inverse`, `v1_unversioned_store_is_rejected_fail_closed`, `pump_block_post_fp_is_byte_identical_to_full_fingerprint`, `forward_sync_post_fp_cache_hit_is_byte_identical`, `forward_sync_replay_two_runs_byte_identical`, `forward_sync_admission_through_chokepoints` |
| **CI** | `ci/ci_check_utxo_fp_v2.sh`, `ci/ci_check_forward_sync_fp_cache.sh` |
| **Claim** | partial — `e563ad94` (pass) — observed: tests: 16 passed, 0 failed, 0 ignored, 0 not found; CI scripts: 2 passed, 0 failed — limits: (1) No IDSE cluster has declared acceptance obligations for this invariant yet (Doctrine §10); this record re-runs the checks carried from the legacy registry. Further limits: (2) No mutation witness exists yet for this critical invariant (Doctrine §28). |

_critical · local · scope: UTxO fingerprint · assumptions: — · depends on: DUR-009, DET-025._ Legacy: DC-MEM-10, DC-MEM-11

---

## DUR — Durability

_Doctrine §6: Preservation of committed state and required evidence across failures._ 14 invariant(s).

### DUR — epoch

#### `DUR-001` — _partial_

| Aspect | Location |
|--------|----------|
| **Source** | `docs/planning/epoch.define.consensus.md#batch-1` (approved by wdm33, 2026-11-08) |
| **Requirement** | **ActivationDurableBeforeVisible** — An epoch view becomes the active view only after its activation record is durable, through one ordered path: take the candidate (for an epoch past the second bootstrap epoch, the sealed leadership object for exactly that epoch with its stored provenance; for the two bootstrap epochs, the certified bootstrap seeding), check the activation predicate (the transition is due and the candidate's bindings match the selected chain), write the activation record durably, then publish; recovery restores exactly the durably recorded activation, rebuilding the view and requiring it to reproduce the record, and with no record the previous view stays active. |
| **Code** | `crates/ade_node/src/epoch_activation.rs`; `ci/ci_check_eview_activation_recovery.sh`; `crates/ade_node/src/epoch_activate.rs`; `ci/ci_check_eview_activate.sh` |
| **Tests** | `crash_before_durable_wal_keeps_seed`, `crash_after_wal_republishes_same_view`, `recovered_view_mismatch_is_terminal`, `durable_before_visible_halts_on_wal_failure`, `resolve_activation_idempotent_conflict_supersede`, `happy_path_promotes_after_durable_wal`, `non_durable_wal_is_terminal_and_does_not_publish`, `not_eligible_transition_is_not_yet_not_terminal`, `invalid_window_is_terminal_before_any_wal`, `selected_point_mismatch_declines` |
| **CI** | `ci/ci_check_eview_activation_recovery.sh`, `ci/ci_check_eview_activate.sh` |
| **Claim** | partial — `e563ad94` (pass) — observed: tests: 10 passed, 0 failed, 0 ignored, 0 not found; CI scripts: 2 passed, 0 failed — limits: (1) No IDSE cluster has declared acceptance obligations for this invariant yet (Doctrine §10); this record re-runs the checks carried from the legacy registry. Further limits: (2) No mutation witness exists yet for this critical invariant (Doctrine §28). |

_critical · transactional · scope: epoch-view activation, recovery · assumptions: A1 · depends on: SAF-004 · secondary: REC._ Legacy: DC-EPOCH-06, DC-EPOCH-10

#### `DUR-002` — _partial_

| Aspect | Location |
|--------|----------|
| **Source** | `docs/planning/epoch.define.consensus.md#batch-1` (approved by wdm33, 2026-11-08) |
| **Requirement** | **SettledTripleIntegrity** — The settled rewind state (accumulator, settled point and settled leadership table) is bound by a domain-separated, length-prefixed fingerprint written in the same durable commit that promotes it, and the fingerprint is recomputed and compared before the state is restored; an absent fingerprint, unreadable context or mismatch refuses the restore and resets to the certified bootstrap baseline. |
| **Code** | `crates/ade_runtime/src/chaindb/epoch_accumulator_store.rs`; `ci/ci_check_settled_triple_integrity.sh` |
| **Tests** | `a_corrupted_settled_triple_is_refused_and_falls_back`, `a_settled_triple_with_no_fingerprint_is_refused`, `the_settled_fingerprint_binds_every_input` |
| **CI** | `ci/ci_check_settled_triple_integrity.sh` |
| **Claim** | partial — `e563ad94` (pass) — observed: tests: 3 passed, 0 failed, 0 ignored, 0 not found; CI scripts: 1 passed, 0 failed — limits: (1) No IDSE cluster has declared acceptance obligations for this invariant yet (Doctrine §10); this record re-runs the checks carried from the legacy registry. Further limits: (2) No mutation witness exists yet for this critical invariant (Doctrine §28). |

_critical · transactional · scope: settled rewind persistence · assumptions: — · depends on: REC-001 · secondary: REC._ Legacy: DC-EPOCH-34

#### `DUR-003` — _partial_

| Aspect | Location |
|--------|----------|
| **Source** | `docs/planning/epoch.define.consensus.md#batch-1` (approved by wdm33, 2026-11-08) |
| **Requirement** | **SelfSufficientEpochState** — After every durable selected-chain block, the node holds enough durable, replayable state to derive every future epoch transition (rewards, stake snapshots, pool and certificate lifecycle, leadership authority) without another snapshot import, an external oracle or manually supplied authority. That state is kept in bounded memory: the UTxO stays in the disk-backed reduced checkpoint, never as a full in-memory map; the accumulator is persisted incrementally, with a boundary checkpoint and at most one epoch of within-epoch replay; and no block requires replaying the whole chain or copying the whole accumulator. |
| **Code** | _(no enforcement found — gap)_ |
| **Tests** | `ade_ledger::epoch_accumulator::tests` (test module), `ade_node::node_lifecycle::ce4a_continuous_self_sufficiency PRODUCTION-LOOP EVIDENCE` (test module) |
| **CI** | `ci/ci_check_epoch_accumulator_no_utxo.sh` |
| **Claim** | partial — `e563ad94` (pass) — observed: tests: 0 passed, 0 failed, 0 ignored, 2 not found; CI scripts: 1 passed, 0 failed — limits: (1) No IDSE cluster has declared acceptance obligations for this invariant yet (Doctrine §10); this record re-runs the checks carried from the legacy registry. Further limits: (2) No mutation witness exists yet for this critical invariant (Doctrine §28). (3) 2 named test(s) were not found in the test output at this revision: ade_ledger::epoch_accumulator::tests (codec round-trip byte-identical + non-canonical/unknown-version/pre-Conway-era/trailing-bytes fail-closed; boundary_rotates_block_production_two_buffer; within_epoch_withdrawal_then_boundary_pays_fresh_reward; pending_reward_update_applied_once_then_cleared; missing_boundary_stake + boundary_gap fail-closed; apply_selected_block_on_real_conway_block_is_deterministic; replay_equivalence_via_durable_checkpoint_across_a_boundary), ade_node::node_lifecycle::ce4a_continuous_self_sufficiency PRODUCTION-LOOP EVIDENCE (#[ignore] slow runs, NOT CI gates): CE-4B ce4b_three_boundary_continuous_self_sufficiency -- LITERAL three-boundary 1340->1341->1342->1343 (seed+2 -> seed+5) in one continuous run, self-sufficient (promotion-certified frozen leadership through 1344; no re-import / CLI oracle / seed-window replay / materialize_bootstrap_into; no fail-closed halt; c5bdc064); CE-4A.1 drive (1340->1342, 9c6fc3c4); #12 ce4a_3_restart_only_equivalence + #13 ce4a_3_r2_rollback_refold_equivalence (warm restart + controlled rollback + production ResetAndRefold == uninterrupted byte-identical; fd3826fd); CE-4A.2 drive_capture_at (POST-1341/1342 byte-exact vs cardano, af3dc9c7). See docs/clusters/LIVE-LEDGER-EPOCH-TRANSITION/CE-4-MILESTONE-DECLARATION.md |

_critical · local · scope: epoch accumulator, durable state · assumptions: A1, A2 · depends on: VAL-020, DET-001 · secondary: LIV._ Legacy: DC-EPOCH-19

### DUR — consensus-inputs

#### `DUR-004` — _partial_

| Aspect | Location |
|--------|----------|
| **Source** | `docs/planning/consensus-inputs.define.consensus.md#batch-1` (approved by wdm33, 2026-11-08) |
| **Requirement** | **SeedInputsRecoveryVerified** — Importing the seed-epoch consensus inputs is recorded as a write-ahead-log fact appended after the record is stored, and the append is the commit point; warm start restores the record only if exactly one such fact exists for the store's anchor and the stored bytes hash to the recorded hash, decode, bind to that anchor and epoch, and re-encode byte-identically; anything else halts startup with a typed error, with no fallback to a bundle. |
| **Code** | `crates/ade_ledger/src/wal/event.rs`; `crates/ade_ledger/src/wal/replay.rs`; `crates/ade_runtime/src/seed_consensus_provenance.rs`; `crates/ade_runtime/src/bootstrap.rs` |
| **Tests** | `wal_seed_cinput_entry_round_trips_byte_identical`, `replay_yields_bootstrap_provenance_view`, `replay_rejects_duplicate_provenance_entry`, `replay_rejects_anchor_mismatched_provenance_entry`, `admit_block_chain_unaffected_by_provenance_entry`, `warm_start_restores_seed_epoch_consensus_inputs_byte_identical`, `warm_start_fails_closed_on_missing_sidecar`, `warm_start_fails_closed_on_hash_mismatch`, `warm_start_fails_closed_on_anchor_mismatch`, `warm_start_fails_closed_on_epoch_mismatch`, `warm_start_required_provenance_rejects_malformed_sidecar`, `warm_start_never_falls_back_to_consensus_inputs_path`, `warm_start_recovers_seed_epoch_consensus_inputs_byte_identical`, `warm_start_fails_closed_on_missing_wal_provenance`, `warm_start_fails_closed_on_duplicate_provenance`, `warm_start_fails_closed_on_multiple_anchor_lineages`, `warm_start_dispatch_succeeds_end_to_end`, `node_sync_kill_then_warm_start_recovers_same_tip` |
| **CI** | `ci/ci_check_lifecycle_owner_uses_bootstrap_initial_state.sh` (red at e563ad94) |
| **Claim** | unresolved — `e563ad94` (fail) — observed: tests: 18 passed, 0 failed, 0 ignored, 0 not found; CI scripts: 0 passed, 1 failed; failing CI: ci/ci_check_lifecycle_owner_uses_bootstrap_initial_state.sh — limits: (1) No IDSE cluster has declared acceptance obligations for this invariant yet (Doctrine §10); this record re-runs the checks carried from the legacy registry. Further limits: (2) No mutation witness exists yet for this critical invariant (Doctrine §28). (3) ci/ci_check_lifecycle_owner_uses_bootstrap_initial_state.sh fails at this revision and already failed before the IDSE migration (a3cef7f7): "FAIL (lifecycle owner): owner (node_lifecycle.rs) materializes initial state OUTSIDE apply_chain_event — the initial-state path must delegate to the single bootstrap_initial_state authority ([legacy ref]).". Diagnosed as a stale or over-broad structural gate, not a demonstrated violation; the gate needs repair before it can serve as evidence. |

_critical · transactional · scope: seed-epoch record persistence and warm start · assumptions: A1 · depends on: SEC-014, REC-002 · secondary: REC._ Legacy: DC-CINPUT-01

#### `DUR-005` — _partial_

| Aspect | Location |
|--------|----------|
| **Source** | `docs/planning/consensus-inputs.define.consensus.md#batch-1` (approved by wdm33, 2026-11-08) |
| **Requirement** | **NetworkGeometryAndProfileDurable** — A recovered store replays with the epoch geometry (first slot of the seed epoch and epoch length) and the consensus profile (genesis hash and protocol-parameter hash, exactly as imported) that were persisted when the store was created, never re-derived from configuration, genesis files or command-line values a restart supplies; restart-supplied values are only a consistency check that fails closed on disagreement. A record in an older format without them is refused with a distinct unsupported-format error that names re-import as the remedy, kept separate from corruption errors, and is never upgraded silently. |
| **Code** | `crates/ade_ledger/src/seed_consensus_inputs.rs`; `crates/ade_runtime/src/seed_consensus_merge.rs`; `crates/ade_runtime/src/consensus_inputs/canonical.rs`; `crates/ade_node/src/node_lifecycle.rs`; `crates/ade_runtime/src/bootstrap.rs`; `crates/ade_ledger/src/bootstrap_manifest.rs`; `ci/ci_check_eview_seed_sidecar_v4.sh` |
| **Tests** | `warm_start_schedule_locates_block_by_venue_geometry_not_hardcoded_432000`, `merge_persists_venue_epoch_geometry_preview_and_preprod`, `restart_genesis_epoch_length_mismatch_fails_closed`, `seed_epoch_consensus_inputs_round_trips_byte_identical`, `seed_cinput_decode_rejects_unknown_version`, `seed_cinput_canonical_bytes_cover_the_consensus_profile_hashes`, `merge_persists_consensus_profile_hashes`, `warm_start_pre_v4_sidecar_is_typed_schema_upgrade_not_corruption` |
| **CI** | `ci/ci_check_eview_seed_sidecar_v4.sh` (red at e563ad94) |
| **Claim** | unresolved — `e563ad94` (fail) — observed: tests: 8 passed, 0 failed, 0 ignored, 0 not found; CI scripts: 0 passed, 1 failed; failing CI: ci/ci_check_eview_seed_sidecar_v4.sh — limits: (1) No IDSE cluster has declared acceptance obligations for this invariant yet (Doctrine §10); this record re-runs the checks carried from the legacy registry. Further limits: (2) No mutation witness exists yet for this critical invariant (Doctrine §28). (3) ci/ci_check_eview_seed_sidecar_v4.sh fails at this revision and already failed before the IDSE migration (a3cef7f7): "FAIL: schema version is not bumped to v4". Diagnosed as a stale or over-broad structural gate, not a demonstrated violation; the gate needs repair before it can serve as evidence. |

_critical · local · scope: seed-epoch record, warm start, era schedule rebuild · assumptions: A2 · depends on: DUR-004, STR-009 · secondary: REC._ Legacy: DC-CINPUT-05, DC-CINPUT-06

### DUR — durability-recovery

#### `DUR-006` — _partial_

| Aspect | Location |
|--------|----------|
| **Source** | `docs/planning/durability-recovery.define.consensus.md#batch-1` (approved by wdm33, 2026-11-08) |
| **Requirement** | **AppendOnlyWal** — The write-ahead log is append-only by construction: its store interface offers no operation that truncates, rewrites, replaces or deletes an entry, append is its only mutation, and recorded provenance is never altered; any later correction is a new appended entry. |
| **Code** | `crates/ade_runtime/src/chaindb/persistent.rs`; `crates/ade_runtime/src/chaindb/contract.rs`; `crates/ade_ledger/src/wal/store_trait.rs`; `crates/ade_runtime/src/wal/file_wal_store.rs` |
| **Tests** | `persistent_passes_contract`, `in_memory_passes_contract`, `reopen_observes_committed_block`, `crates/ade_runtime/src/wal/file_wal_store.rs::tests::file_wal_store_append_then_read_all_round_trips`, `crates/ade_runtime/src/wal/file_wal_store.rs::tests::file_wal_store_reopens_existing_directory_and_preserves_entries`, `crates/ade_runtime/src/wal/file_wal_store.rs::tests::file_wal_store_rotates_at_max_bytes_when_forced` |
| **CI** | `ci/ci_check_chaindb_contract.sh`, `ci/ci_check_wal_append_only.sh` |
| **Claim** | partial — `e563ad94` (pass) — observed: tests: 6 passed, 0 failed, 0 ignored, 0 not found; CI scripts: 2 passed, 0 failed — limits: (1) No IDSE cluster has declared acceptance obligations for this invariant yet (Doctrine §10); this record re-runs the checks carried from the legacy registry. Further limits: (2) No mutation witness exists yet for this critical invariant (Doctrine §28). |

_critical · transactional · scope: write-ahead log · assumptions: — · depends on: — · secondary: OBS._ Legacy: DC-STORE-02, CN-STORE-05, CN-WAL-01, DC-WAL-01

#### `DUR-007` — _partial_

| Aspect | Location |
|--------|----------|
| **Source** | `docs/planning/durability-recovery.define.consensus.md#batch-1` (approved by wdm33, 2026-11-08) |
| **Requirement** | **AtomicCheckpoints** — Every snapshot and checkpoint is either fully written and valid or absent: a crash at any point during a write leaves either the previous complete version or the new complete version, never a partial one that recovery could read. |
| **Code** | `crates/ade_runtime/src/chaindb/snapshot_contract.rs`; `crates/ade_runtime/src/chaindb/persistent.rs` |
| **Tests** | `persistent_passes_snapshot_contract`, `in_memory_passes_snapshot_contract`, `snapshots_persist_across_reopen`, `corrupted_magic_returns_corruption_error` |
| **CI** | `ci/ci_check_chaindb_contract.sh` |
| **Claim** | partial — `e563ad94` (pass) — observed: tests: 4 passed, 0 failed, 0 ignored, 0 not found; CI scripts: 1 passed, 0 failed — limits: (1) No IDSE cluster has declared acceptance obligations for this invariant yet (Doctrine §10); this record re-runs the checks carried from the legacy registry. Further limits: (2) No mutation witness exists yet for this critical invariant (Doctrine §28). |

_critical · transactional · scope: snapshots and checkpoints · assumptions: A6 · depends on: DET-011 · secondary: REC._ Legacy: DC-STORE-03, CN-STORE-04

#### `DUR-008` — _partial_

| Aspect | Location |
|--------|----------|
| **Source** | `docs/planning/durability-recovery.define.consensus.md#batch-1` (approved by wdm33, 2026-11-08) |
| **Requirement** | **SingleLineageBinding** — Every write-ahead-log entry, checkpoint and recovered artifact is bound to exactly one bootstrap anchor, and recovery refuses any artifact bound to a different anchor or to none. |
| **Code** | `crates/ade_node/src/node_lifecycle.rs` |
| **Tests** | `warm_start_fails_closed_on_multiple_anchor_lineages`, `warm_start_fails_closed_on_anchor_mismatch`, `warm_start_fails_closed_on_duplicate_provenance` |
| **CI** | _(no CI enforcement — gap)_ |
| **Claim** | partial — `e563ad94` (pass) — observed: tests: 3 passed, 0 failed, 0 ignored, 0 not found; CI scripts: 0 passed, 0 failed — limits: (1) No IDSE cluster has declared acceptance obligations for this invariant yet (Doctrine §10); this record re-runs the checks carried from the legacy registry. Further limits: (2) No mutation witness exists yet for this critical invariant (Doctrine §28). |

_critical · local · scope: durable artifacts, recovery · assumptions: — · depends on: SEC-015 · secondary: SAF._ Legacy: CN-STORE-02

#### `DUR-009` — _partial_

| Aspect | Location |
|--------|----------|
| **Source** | `docs/planning/durability-recovery.define.consensus.md#batch-1` (approved by wdm33, 2026-11-08) |
| **Requirement** | **WalFingerprintChain** — The write-ahead log's fingerprint chain is checked by folding the log from the bootstrap anchor's initial ledger fingerprint: a block-admission entry, received or self-forged, must record a prior fingerprint equal to the folded value, binds the exact admitted block bytes, and advances the folded value to its post fingerprint; a rollback entry whose target is the bootstrap anchor resets the folded value to the anchor's initial ledger fingerprint, and one whose target is an admission on the currently selected prefix sets it to that admission's post fingerprint, any other target being rejected; provenance and activation entries leave it unchanged. An entry that would break the chain is rejected and halts the node, and verifying the chain at startup is fatal on failure. |
| **Code** | `crates/ade_ledger/src/wal/store_trait.rs`; `crates/ade_ledger/src/wal/replay.rs`; `crates/ade_ledger/src/wal`; `crates/ade_node/src/node_sync.rs`; `crates/ade_node/src/node_lifecycle.rs`; `crates/ade_ledger/src/wal/event.rs` |
| **Tests** | `crates/ade_runtime/src/wal/file_wal_store.rs::tests::file_wal_store_verify_chain_passes_then_catches_break`, `crates/ade_ledger/src/wal/replay.rs::tests::replay_from_anchor_catches_chain_break`, `crates/ade_runtime/tests/wal_replay_from_anchor.rs::wal_replay_from_anchor_rejects_chain_break`, `crates/ade_node/src/node_sync.rs::tests::recover_follow_kill_warm_start_chains_from_ledger_fp`, `crates/ade_node/src/node_sync.rs::tests::recover_follow_zero_seed_chainbreaks`, `crates/ade_node/src/node_sync.rs::tests::recover_follow_two_runs_byte_identical`, `forged_admit_wal_prior_fp_chains`, `warm_start_drops_orphan_block_above_wal_tail`, `forge_tip_successor_kill_then_warm_start_recovers_block_one`, `apply_rolledback_replays_byte_identical_recovers_forkpoint`, `replay_with_rollback_recovers_selected_not_abandoned`, `replay_with_rollback_two_runs_byte_identical`, `rollback_replay_reanchor_fp_equals_materialized_fp` |
| **CI** | `ci/ci_check_recover_follow_wal_lineage.sh` (red at e563ad94), `ci/ci_check_forged_durable_admit_via_pump.sh` (red at e563ad94), `ci/ci_check_wal_rollback_replay_equiv.sh`, `ci/ci_check_live_fork_choice_apply.sh` (red at e563ad94) |
| **Claim** | unresolved — `e563ad94` (fail) — observed: tests: 13 passed, 0 failed, 0 ignored, 0 not found; CI scripts: 1 passed, 3 failed; failing CI: ci/ci_check_recover_follow_wal_lineage.sh, ci/ci_check_forged_durable_admit_via_pump.sh, ci/ci_check_live_fork_choice_apply.sh — limits: (1) No IDSE cluster has declared acceptance obligations for this invariant yet (Doctrine §10); this record re-runs the checks carried from the legacy registry. Further limits: (2) No mutation witness exists yet for this critical invariant (Doctrine §28). (3) ci/ci_check_recover_follow_wal_lineage.sh fails at this revision and already failed before the IDSE migration (a3cef7f7): "FAIL ci_check_recover_follow_wal_lineage: expected 2 ForwardSyncState::new sites in node_lifecycle.rs, found 5". Diagnosed as a stale or over-broad structural gate, not a demonstrated violation; the gate needs repair before it can serve as evidence. (4) ci/ci_check_forged_durable_admit_via_pump.sh fails at this revision and already failed before the IDSE migration (a3cef7f7): "FAIL (forged durable admit via pump): could not isolate admit_forged_block_durably body in node_sync.rs (signature moved/renamed?)". Diagnosed as a stale or over-broad structural gate, not a demonstrated violation; the gate needs repair before it can serve as evidence. (5) ci/ci_check_live_fork_choice_apply.sh fails at this revision and already failed before the IDSE migration (a3cef7f7): "FAIL: durable_tip_matches ([legacy ref] reconciliation) missing". Diagnosed as a stale or over-broad structural gate, not a demonstrated violation; the gate needs repair before it can serve as evidence. |

_critical · transactional · scope: write-ahead log · assumptions: A6 · depends on: DUR-006, DET-002 · secondary: REC._ Legacy: DC-WAL-02, DC-WAL-04

#### `DUR-010` — _partial_

| Aspect | Location |
|--------|----------|
| **Source** | `docs/planning/durability-recovery.define.consensus.md#batch-1` (approved by wdm33, 2026-11-08) |
| **Requirement** | **BlockBytesBeforeWal** — An admitted block's original bytes are durably stored in the chain database before its write-ahead-log entry is appended, on every admission path; a storage failure halts before the append, and at recovery a log entry whose block bytes are missing halts with a typed error, while stored bytes with no log entry are dropped as an orphan. |
| **Code** | `crates/ade_node/src/admission/runner.rs`; `crates/ade_node/src/node_lifecycle.rs` |
| **Tests** | `warmstart_from_real_admission_store_uses_persisted_bytes_no_mock`, `warmstart_fails_closed_when_wal_admitblock_missing_bytes` |
| **CI** | `ci/ci_check_admission_runner_no_block_byte_map.sh` |
| **Claim** | partial — `e563ad94` (pass) — observed: tests: 2 passed, 0 failed, 0 ignored, 0 not found; CI scripts: 1 passed, 0 failed — limits: (1) No IDSE cluster has declared acceptance obligations for this invariant yet (Doctrine §10); this record re-runs the checks carried from the legacy registry. Further limits: (2) No mutation witness exists yet for this critical invariant (Doctrine §28). |

_critical · transactional · scope: block admission, recovery · assumptions: A6 · depends on: DUR-009, REC-002 · secondary: REC._ Legacy: DC-WAL-05

#### `DUR-011` — _partial_

| Aspect | Location |
|--------|----------|
| **Source** | `docs/planning/durability-recovery.define.consensus.md#batch-1` (approved by wdm33, 2026-11-08) |
| **Requirement** | **CollateralValueRetained** — When a block's phase-2-invalid transaction spends a collateral input, the UTxO authority durably records that input's coin value in the same durable transaction that applies the block's changes to the live table and advances the checkpoint cursor, so the accumulator can resolve it at any later cursor position; retention covers exactly the bindings destroyed since bootstrap, is cleared with the live table when the store resets, and lies outside the checkpoint commitment; an input that neither the block nor the authority holds resolves to nothing, and the accumulator refuses, naming the input. |
| **Code** | `crates/ade_ledger/src/reduced_advance.rs`; `crates/ade_runtime/src/chaindb/reduced_utxo_checkpoint.rs`; `crates/ade_runtime/src/chaindb/reduced_window_driver.rs`; `crates/ade_ledger/src/store_semantics.rs`; `ci/ci_check_collateral_retention_positioning.sh` |
| **Tests** | `the_resolver_answers_after_the_authority_has_already_spent_the_collateral`, `the_accumulator_walk_resolves_collateral_the_authority_already_spent`, `without_the_retention_the_same_walk_still_refuses_and_pins`, `reset_to_bootstrap_clears_the_retention_and_the_replay_rederives_it`, `the_retention_does_not_move_the_checkpoint_commitment`, `an_unheld_collateral_binding_is_retained_as_nothing_never_as_zero`, `an_intra_block_created_collateral_binding_is_retained_from_the_block`, `an_intra_block_created_collateral_binding_is_valued_by_the_block_itself`, `the_real_failing_block_names_its_collateral_binding_for_retention`, `an_ordinary_spend_is_never_named_for_retention` |
| **CI** | `ci/ci_check_collateral_retention_positioning.sh` |
| **Claim** | partial — `e563ad94` (pass) — observed: tests: 10 passed, 0 failed, 0 ignored, 0 not found; CI scripts: 1 passed, 0 failed — limits: (1) No IDSE cluster has declared acceptance obligations for this invariant yet (Doctrine §10); this record re-runs the checks carried from the legacy registry. Further limits: (2) No mutation witness exists yet for this critical invariant (Doctrine §28). |

_critical · transactional · scope: reduced UTxO checkpoint, collateral resolution · assumptions: A6 · depends on: CON-003, DET-013 · secondary: CON._ Legacy: DC-LEDGER-PHASE2-04

### DUR — network

#### `DUR-012` — _partial_

| Aspect | Location |
|--------|----------|
| **Source** | `docs/planning/network.define.consensus.md#batch-1` (approved by wdm33, 2026-11-08) |
| **Requirement** | **TipAdvancesOnlyAfterDurableAdmit** — The selected tip advances to a block only after that block's bytes and its write-ahead-log entry are durable, and only through the one admission path (decode, header validation, block validity, chain selection); no verdict, follower or diagnostic path, and no direct storage call, moves the live tip. The tip-advance effect can be constructed only after the store-bytes and append-log effects for that block, and the pump that applies effects fails closed on any out-of-order application. |
| **Code** | `crates/ade_ledger/src/receive/admitted.rs`; `crates/ade_ledger/src/receive/reducer.rs`; `crates/ade_ledger/src/receive/chain_write.rs`; `crates/ade_runtime/src/forward_sync`; `crates/ade_runtime/src/recovery/restart.rs`; `crates/ade_node/src/node_sync.rs` |
| **Tests** | `admit_via_block_validity_accepts_corpus_block`, `admit_via_block_validity_rejects_corrupted_body`, `receive_apply_block_delivered_with_matching_header_admits`, `receive_apply_block_delivered_validity_invalid_rejects`, `receive_apply_rollback_returns_out_of_scope`, `receive_apply_replay_byte_identical_over_corpus`, `forward_sync_wal_and_bytes_precede_tip_advance`, `forward_sync_replay_two_runs_byte_identical`, `forward_sync_admission_through_chokepoints`, `recovery_torn_put_block_before_wal_append_drops_orphan`, `node_sync_pump_advances_recoverable_tip`, `node_sync_fails_closed_on_undecodable_block`, `node_sync_kill_then_warm_start_recovers_same_tip`, `relay_loop_syncs_then_halts_clean_on_source_end`, `relay_loop_idles_then_syncs_on_incremental_feed`, `relay_loop_fails_closed_on_unapplyable_block` |
| **CI** | `ci/ci_check_admitted_block_closure.sh`, `ci/ci_check_receive_reducer_closure.sh`, `ci/ci_check_forward_sync_chokepoint_only.sh`, `ci/ci_check_node_sync_via_pump.sh` (red at e563ad94) |
| **Claim** | unresolved — `e563ad94` (fail) — observed: tests: 16 passed, 0 failed, 0 ignored, 0 not found; CI scripts: 3 passed, 1 failed; failing CI: ci/ci_check_node_sync_via_pump.sh — limits: (1) No IDSE cluster has declared acceptance obligations for this invariant yet (Doctrine §10); this record re-runs the checks carried from the legacy registry. Further limits: (2) No mutation witness exists yet for this critical invariant (Doctrine §28). (3) ci/ci_check_node_sync_via_pump.sh fails at this revision and already failed before the IDSE migration (a3cef7f7): "FAIL (node sync via pump): could not isolate run_node_sync body in node_sync.rs (signature moved/renamed?)". Diagnosed as a stale or over-broad structural gate, not a demonstrated violation; the gate needs repair before it can serve as evidence. |

_critical · transactional · scope: forward sync, continuous relay loop · assumptions: A6 · depends on: DUR-010, STR-004 · secondary: STR._ Legacy: DC-SYNC-01, DC-SYNC-02

### DUR — pipeline-production

#### `DUR-013` — _partial_

| Aspect | Location |
|--------|----------|
| **Source** | `docs/planning/pipeline-production.define.consensus.md#batch-1` (approved by wdm33, 2026-11-09) |
| **Requirement** | **AdmissionAppendsExactlyOnce** — Every successful admission, received or self-forged, appends exactly one block-admission entry to the write-ahead log, chained from the folded fingerprint (DUR-009), and no admission entry exists without its admission or for a block already admitted on the selected prefix; an append that fails halts the node with the tip unadvanced (SAF-019). |
| **Code** | `crates/ade_node/src/admission/runner.rs` |
| **Tests** | `admission_replay_equivalence::admission_replay_equivalence_byte_identical_wal_after_two_runs`, `admission_replay_equivalence::admission_tip_update_does_not_emit_wal_entry` |
| **CI** | `ci/ci_check_admission_runner_closure.sh` (red at e563ad94), `ci/ci_check_wal_append_only.sh` |
| **Claim** | unresolved — `e563ad94` (fail) — observed: tests: 2 passed, 0 failed, 0 ignored, 0 not found; CI scripts: 1 passed, 1 failed; failing CI: ci/ci_check_admission_runner_closure.sh — limits: (1) No IDSE cluster has declared acceptance obligations for this invariant yet (Doctrine §10); this record re-runs the checks carried from the legacy registry. Further limits: (2) No mutation witness exists yet for this critical invariant (Doctrine §28). (3) ci/ci_check_admission_runner_closure.sh fails at this revision and already failed before the IDSE migration (a3cef7f7): "FAIL: AdmissionInputs carries Option<_> fields (must be all-required):". Diagnosed as a stale or over-broad structural gate, not a demonstrated violation; the gate needs repair before it can serve as evidence. |

_critical · local · scope: admission path, write-ahead log · assumptions: A6 · depends on: DUR-009, DUR-012, SAF-019 · secondary: REC._ Legacy: DC-ADMIT-05

### DUR — mithril-evidence

#### `DUR-014` — _declared_

| Aspect | Location |
|--------|----------|
| **Source** | `docs/planning/mithril-evidence.define.consensus.md#batch-1` (approved by wdm33, 2026-11-09) |
| **Requirement** | **NativeBootstrapCommitsAtomically** — The native route persists the bootstrap through the one bootstrap composition (the bootstrap authority, the seed-epoch consensus record, the recovered anchor point and the write-ahead-log provenance entry), and appending that provenance entry is the only point at which the bootstrap becomes discoverable: a failure at any earlier step, including building the reduced checkpoint, leaves nothing a warm start would boot from, and after the append the anchor point is recoverable. No second storage-initialization path exists. |
| **Code** | _(no enforcement found — gap)_ |
| **Tests** | _(no tests found — gap)_ |
| **CI** | _(no CI enforcement — gap)_ |
| **Claim** | unresolved — no evidence record |

_critical · transactional · scope: native Mithril first run · assumptions: A6 · depends on: SEC-015, DUR-004, STR-013 · secondary: REC._ Legacy: none (new in IDSE)

---

## REC — Recovery

_Doctrine §6: Restoration of valid system state and behavior following failures or interruptions._ 8 invariant(s).

### REC — epoch

#### `REC-001` — _partial_

| Aspect | Location |
|--------|----------|
| **Source** | `docs/planning/epoch.define.consensus.md#batch-1` (approved by wdm33, 2026-11-08) |
| **Requirement** | **RewindTargetSettledAndCanonical** — The epoch accumulator is rewound only to a settled point that is at least k blocks below the durable tip, counted in blocks, at or before the rollback target, and whose header hash is still canonical at its slot; any other target is refused and the accumulator resets to its certified bootstrap baseline. A rewound store is uncertified until a canonical refold rewrites its anchor, and a staged point taken on the abandoned chain is discarded. |
| **Code** | `crates/ade_runtime/src/chaindb/epoch_accumulator_store.rs`; `crates/ade_node/src/node_lifecycle.rs`; `ci/ci_check_accumulator_refold_bound.sh` |
| **Tests** | `settled_point_is_only_promoted_once_k_blocks_settled`, `settled_rewind_admission_requires_settled_depth_and_intact_lineage`, `reset_to_settled_restores_pair_and_leaves_store_uncertified`, `bootstrap_reset_discards_the_settled_rewind_point` |
| **CI** | `ci/ci_check_accumulator_refold_bound.sh` |
| **Claim** | partial — `e563ad94` (pass) — observed: tests: 4 passed, 0 failed, 0 ignored, 0 not found; CI scripts: 1 passed, 0 failed — limits: (1) No IDSE cluster has declared acceptance obligations for this invariant yet (Doctrine §10); this record re-runs the checks carried from the legacy registry. Further limits: (2) No mutation witness exists yet for this critical invariant (Doctrine §28). |

_critical · local · scope: epoch accumulator rewind · assumptions: A4 · depends on: DET-009 · secondary: SAF._ Legacy: DC-EPOCH-26, DC-EPOCH-27, DC-EPOCH-29

#### `REC-002` — _partial_

| Aspect | Location |
|--------|----------|
| **Source** | `docs/planning/epoch.define.consensus.md#batch-1` (approved by wdm33, 2026-11-08) |
| **Requirement** | **SingleAdmissionPrefix** — The write-ahead log's tail defines the durable selected-chain prefix: the chain database, the Praos chain-dependent state, the epoch accumulator and the reduced UTxO checkpoint are each derived from that one prefix, in chain order, each advancing only after the block is durably admitted, and recovery drops anything above the tail and re-derives any of them that lags or leads it, never resuming with them on different prefixes. The reduced checkpoint is a deterministic projection of the ledger UTxO, so the same blocks give a byte-identical checkpoint, and a missing, corrupt or lagging checkpoint blocks epoch-view production and fails closed. |
| **Code** | `crates/ade_node/src/admission/bootstrap.rs`; `ci/ci_check_eview_live_checkpoint.sh` |
| **Tests** | `live_reduced_checkpoint_builds_durable_deterministic`, `ade_runtime::chaindb::epoch_accumulator_advance::tests` (test module), `ade_runtime::chaindb::epoch_accumulator_store::tests` (test module) |
| **CI** | `ci/ci_check_eview_live_checkpoint.sh` (red at e563ad94), `ci/ci_check_epoch_accumulator_recovery.sh` (red at e563ad94) |
| **Claim** | unresolved — `e563ad94` (fail) — observed: tests: 1 passed, 0 failed, 0 ignored, 2 not found; CI scripts: 0 passed, 2 failed; failing CI: ci/ci_check_eview_live_checkpoint.sh, ci/ci_check_epoch_accumulator_recovery.sh — limits: (1) No IDSE cluster has declared acceptance obligations for this invariant yet (Doctrine §10); this record re-runs the checks carried from the legacy registry. Further limits: (2) No mutation witness exists yet for this critical invariant (Doctrine §28). (3) 2 named test(s) were not found in the test output at this revision: ade_runtime::chaindb::epoch_accumulator_advance::tests (over_chaindb_folds_durable_prefix_to_tip -- warm-start catch-up folds the durable prefix to the tip; over_chaindb_rewalk_is_idempotent -- replay-safe resume advances nothing; over_chaindb_stops_at_boundary_observe_only -- the store freezes at the last within-epoch slot, MissingBoundaryStake; reset_then_rewalk_rematerializes -- reorg = reset-to-seed + forward replay re-materializes the SAME tip, no inverse mutation), ade_runtime::chaindb::epoch_accumulator_store::tests (seal_advance_reset_round_trip_is_exact; advance_is_strictly_forward -- NonMonotonicAdvance fail-closed; readiness_gate_fails_closed -- Lagging/Ahead/Unsealed terminal) (4) ci/ci_check_eview_live_checkpoint.sh fails at this revision and already failed before the IDSE migration (a3cef7f7): "FAIL: the build is not gated on the EVIEW cert-state -- non-EVIEW bootstrap must stay byte-identical". Diagnosed as a stale or over-broad structural gate, not a demonstrated violation; the gate needs repair before it can serve as evidence. (5) ci/ci_check_epoch_accumulator_recovery.sh fails at this revision and already failed before the IDSE migration (a3cef7f7): "FAIL: (C) advance_accumulator_to_durable_tip (the observe-only wrapper) is missing". Diagnosed as a stale or over-broad structural gate, not a demonstrated violation; the gate needs repair before it can serve as evidence. |

_critical · transactional · scope: block admission, recovery · assumptions: A1 · depends on: SAF-001, DET-001 · secondary: DUR._ Legacy: DC-EPOCH-20, DC-EPOCH-11

### REC — consensus

#### `REC-003` — _partial_

| Aspect | Location |
|--------|----------|
| **Source** | `docs/planning/consensus.define.consensus.md#batch-1` (approved by wdm33, 2026-11-08) |
| **Requirement** | **LockstepAdmissionAndRollback** — A successful block admission advances the chain database, the ledger state and the Praos chain-dependent state as one transition, and a rollback returns all three to the same point; no path leaves them at different points, admits partially or rolls back partially. |
| **Code** | `crates/ade_ledger/src/receive/reducer.rs`; `crates/ade_ledger/src/rollback/commit.rs`; `crates/ade_ledger/src/rollback/materialize.rs`; `crates/ade_runtime/src/rollback` |
| **Tests** | `receive_apply_block_delivered_with_matching_header_admits`, `commit_rollback_advances_chaindb_and_ledger_atomically`, `commit_rollback_chain_write_failure_leaves_state_unchanged`, `commit_rollback_resets_pending_headers`, `rollback_branch_returns_rolled_back_on_in_memory_snapshot`, `rollback_branch_returns_rollback_too_deep_when_no_snapshot`, `rollback_branch_state_unchanged_on_materialize_failure`, `rollback_then_continue_admit_equals_straight_line_admit` |
| **CI** | `ci/ci_check_receive_reducer_closure.sh`, `ci/ci_check_rollback_materialize_closure.sh`, `ci/ci_check_receive_orchestrator_no_producer_dep.sh` |
| **Claim** | partial — `e563ad94` (pass) — observed: tests: 8 passed, 0 failed, 0 ignored, 0 not found; CI scripts: 3 passed, 0 failed — limits: (1) No IDSE cluster has declared acceptance obligations for this invariant yet (Doctrine §10); this record re-runs the checks carried from the legacy registry. Further limits: (2) No mutation witness exists yet for this critical invariant (Doctrine §28). |

_critical · transactional · scope: block admission and rollback · assumptions: A1 · depends on: SAF-001, REC-002 · secondary: SAF._ Legacy: DC-CONS-20

### REC — durability-recovery

#### `REC-004` — _partial_

| Aspect | Location |
|--------|----------|
| **Source** | `docs/planning/durability-recovery.define.consensus.md#batch-1` (approved by wdm33, 2026-11-08); 1 recorded redefinition(s) in the registry |
| **Requirement** | **CrashRecoveryReplayEquivalent** — All authoritative state is derivable by replay from the bootstrap anchor and the accepted canonical inputs. Two continuous runs from the same recovered or bootstrapped state, with the same ordered block feed, the same clock-tick schedule, the same leadership and key inputs and the same shutdown schedule, produce byte-identical durable outputs (selected tip, ledger fingerprint, write-ahead-log image, checkpoints and halt state), self-forged admissions included. After any process termination or power loss, recovery reaches exactly the last durably committed prefix, and its durable outputs are byte-identical to those of a run that stopped cleanly at that prefix; the ledger fingerprint replay reaches after log entry N equals the fingerprint the log's fold expects after entry N (DUR-009). Recovery starts from the most recent valid snapshot at or below the durable tip and replays forward only the stored blocks after it. |
| **Code** | `crates/ade_runtime/src/recovery/mod.rs`; `crates/ade_runtime/src/recovery/restart.rs`; `crates/ade_runtime/src/chaindb/crash_safety.rs`; `crates/ade_runtime/tests/stress_kill_harness.rs`; `crates/ade_ledger/src/wal/replay.rs`; `crates/ade_runtime/tests/wal_replay_from_anchor.rs`; `crates/ade_node/src/node_lifecycle.rs`; `crates/ade_node/src/node_sync.rs`; `crates/ade_runtime/src/bootstrap.rs`; `crates/ade_ledger/src/wal/event.rs` |
| **Tests** | `recover_from_snapshot_and_replay_forward`, `recover_from_genesis_when_no_snapshot`, `apply_failure_surfaces_with_slot`, `snapshot_decode_failure_surfaces_as_error`, `snapshot_with_no_post_blocks_is_ok`, `stress_kill_smoke`, `stress_kill_1000` (#[ignore]), `snapshot_table_intact_after_kill_loop`, `persistent_passes_crash_safety_with_no_kill`, `no_starting_point_error`, `crates/ade_runtime/tests/wal_replay_from_anchor.rs::wal_replay_from_anchor_two_runs_byte_identical`, `crates/ade_runtime/tests/wal_replay_from_anchor.rs::wal_replay_from_anchor_post_fp_matches_wal_tail`, `crates/ade_runtime/tests/wal_replay_from_anchor.rs::wal_replay_from_anchor_persists_across_reopen`, `crates/ade_ledger/src/wal/replay.rs::tests::replay_from_anchor_three_entry_chain_ok`, `crates/ade_ledger/src/wal/replay.rs::tests::replay_from_anchor_two_runs_byte_identical`, `relay_loop_two_clean_runs_byte_identical`, `forge_kill_then_warm_start_recovers_same_tip_via_forward_replay`, `forge_tip_successor_kill_then_warm_start_recovers_block_one`, `recover_follow_forge_two_runs_byte_identical`, `recover_follow_kill_warm_start_chains_from_ledger_fp`, `recover_follow_two_runs_byte_identical`, `same_store_same_anchor_point_same_findintersect_start`, `apply_rolledback_replays_byte_identical_recovers_forkpoint`, `replay_with_rollback_recovers_selected_not_abandoned`, `replay_with_rollback_two_runs_byte_identical`, `rollback_replay_reanchor_fp_equals_materialized_fp` |
| **CI** | `ci/ci_check_recovery_contract.sh`, `ci/ci_check_chaindb_crash_safety.sh`, `ci/ci_check_wal_rollback_replay_equiv.sh`, `ci/ci_check_live_fork_choice_apply.sh` (red at e563ad94) |
| **Claim** | unresolved — `e563ad94` (fail) — observed: tests: 25 passed, 0 failed, 1 ignored, 0 not found; CI scripts: 3 passed, 1 failed; failing CI: ci/ci_check_live_fork_choice_apply.sh — limits: (1) No IDSE cluster has declared acceptance obligations for this invariant yet (Doctrine §10); this record re-runs the checks carried from the legacy registry. Further limits: (2) No mutation witness exists yet for this critical invariant (Doctrine §28). (3) 1 named test(s) are #[ignore]d and did not run. (4) ci/ci_check_live_fork_choice_apply.sh fails at this revision and already failed before the IDSE migration (a3cef7f7): "FAIL: durable_tip_matches ([legacy ref] reconciliation) missing". Diagnosed as a stale or over-broad structural gate, not a demonstrated violation; the gate needs repair before it can serve as evidence. |

_critical · transactional · scope: node recovery, write-ahead-log replay · assumptions: A1, A6 · depends on: DET-001, REC-002 · secondary: DET, DUR._ Legacy: T-REC-01, T-REC-02, DC-STORE-01, CN-STORE-03, T-REC-03, T-REC-05, DC-WAL-03

#### `REC-005` — _partial_

| Aspect | Location |
|--------|----------|
| **Source** | `docs/planning/durability-recovery.define.consensus.md#batch-1` (approved by wdm33, 2026-11-08) |
| **Requirement** | **RecoveryFindsCommonAncestor** — After any process termination, a node recovering from valid durable state finds a common ancestor with a compatible peer, given that peer's responses to its intersection requests, whenever one exists within its supported intersection history (k blocks below its durable tip, or its oldest stored block if fewer are kept), without discarding authoritative state silently and without changing replay: the ledger state, chain-dependent state and selected chain after rolling back to that ancestor and replaying forward equal those of a run that never adopted the abandoned blocks, compared at the same canonical chain points, the write-ahead log's audit history of the abandoned admissions and the rollback being excluded from the comparison. When no such ancestor exists, it stops following that peer with a structured outcome and leaves its durable state untouched; a peer's reply never instructs it to rewrite the chain back to genesis. |
| **Code** | `crates/ade_ledger/src/rollback/intersection.rs`; `crates/ade_node/src/node_lifecycle.rs` |
| **Tests** | `lv16_killed_store_shape_intersects_at_the_parent`, `lv16_rollback_to_the_found_parent_matches_a_run_that_never_saw_the_tip`, `lv16_a_peer_sharing_no_offered_point_is_no_common_point`, `lv16_pump_ends_on_intersect_not_found_without_request_next` |
| **CI** | `ci/ci_check_durable_intersection.sh` |
| **Claim** | partial — `e563ad94` (pass) — observed: tests: 4 passed, 0 failed, 0 ignored, 0 not found; CI scripts: 1 passed, 0 failed — limits: (1) No IDSE cluster has declared acceptance obligations for this invariant yet (Doctrine §10); this record re-runs the checks carried from the legacy registry. Further limits: (2) No mutation witness exists yet for this critical invariant (Doctrine §28). |

_critical · distributed · scope: recovery, chain-sync intersection · assumptions: A1, A4 · depends on: REC-004, SAF-007 · secondary: LIV._ Legacy: T-REC-07

### REC — node

#### `REC-006` — _partial_

| Aspect | Location |
|--------|----------|
| **Source** | `docs/planning/node.define.consensus.md#batch-1` (approved by wdm33, 2026-11-09) |
| **Requirement** | **SelectorReconciledWithDurable** — After every applied receive decision, the chain selector's current tip equals the durable chain tip and its chain-dependent state equals the durable chain-dependent state; the in-memory decision state never runs ahead of, behind or forked from the durable chain. |
| **Code** | `crates/ade_node/src/node_sync.rs` |
| **Tests** | `apply_reconciliation_mismatch_fails_fast`, `apply_rejected_makes_no_durable_change` |
| **CI** | `ci/ci_check_live_fork_choice_apply.sh` (red at e563ad94) |
| **Claim** | unresolved — `e563ad94` (fail) — observed: tests: 2 passed, 0 failed, 0 ignored, 0 not found; CI scripts: 0 passed, 1 failed; failing CI: ci/ci_check_live_fork_choice_apply.sh — limits: (1) No IDSE cluster has declared acceptance obligations for this invariant yet (Doctrine §10); this record re-runs the checks carried from the legacy registry. Further limits: (2) No mutation witness exists yet for this critical invariant (Doctrine §28). (3) ci/ci_check_live_fork_choice_apply.sh fails at this revision and already failed before the IDSE migration (a3cef7f7): "FAIL: durable_tip_matches ([legacy ref] reconciliation) missing". Diagnosed as a stale or over-broad structural gate, not a demonstrated violation; the gate needs repair before it can serve as evidence. |

_critical · transactional · scope: chain selector, durable state · assumptions: — · depends on: STR-016._ Legacy: DC-NODE-26

#### `REC-007` — _partial_

| Aspect | Location |
|--------|----------|
| **Source** | `docs/planning/node.define.consensus.md#batch-1` (approved by wdm33, 2026-11-09) |
| **Requirement** | **FollowStartFromPersistedAnchor** — A store recovered from a non-Origin bootstrap anchor persists that anchor point, bound to its fingerprint, and live follow starts from the servable chain tip if one exists, otherwise from that persisted anchor point; a missing, malformed or mismatched anchor record fails closed before following starts, and Origin is used only for a store that truly starts at Origin. |
| **Code** | `crates/ade_ledger/src/recovered_anchor_point.rs`; `crates/ade_runtime/src/bootstrap.rs`; `crates/ade_runtime/src/recovered_anchor.rs`; `crates/ade_runtime/src/seed_epoch_lineage.rs`; `crates/ade_runtime/src/chaindb`; `crates/ade_node/src/node_lifecycle.rs`; `crates/ade_runtime/src/mithril_bootstrap.rs`; `crates/ade_runtime/src/admission/wire_pump.rs` |
| **Tests** | `crates/ade_runtime/src/bootstrap.rs::resolve_live_follow_start_treats_zero_hash_anchor_as_origin`, `crates/ade_runtime/src/bootstrap.rs::bootstrap_bare_anchor_recovery_surfaces_anchor_as_live_follow_tip`, `crates/ade_runtime/src/bootstrap.rs::bootstrap_true_origin_recovery_surfaces_none_tip`, `crates/ade_runtime/src/bootstrap.rs::bootstrap_servable_chaindb_tip_wins_over_anchor`, `crates/ade_runtime/src/bootstrap.rs::warm_start_loads_persisted_anchor_point`, `crates/ade_runtime/src/bootstrap.rs::warm_start_non_origin_anchor_missing_anchor_point_fails_closed`, `crates/ade_runtime/src/bootstrap.rs::warm_start_anchor_point_fingerprint_mismatch_fails_closed`, `crates/ade_runtime/src/bootstrap.rs::same_store_same_anchor_point_same_findintersect_start`, `crates/ade_runtime/src/seed_epoch_lineage.rs::bootstrap_recover_persists_anchor_point_sidecar`, `crates/ade_ledger/src/recovered_anchor_point.rs::recovered_anchor_point_round_trips_byte_identical`, `crates/ade_node/src/node_lifecycle.rs::recovered_bare_anchor_findintersect_starts_at_anchor_not_origin` |
| **CI** | _(no CI enforcement — gap)_ |
| **Claim** | unresolved — no evidence record |

_critical · local · scope: warm start, live follow · assumptions: A2 · depends on: DUR-008._ Legacy: DC-NODE-31

### REC — resources-release

#### `REC-008` — _declared_

| Aspect | Location |
|--------|----------|
| **Source** | `docs/planning/resources-release.define.consensus.md#batch-1` (approved by wdm33, 2026-11-09) |
| **Requirement** | **ReconciliationOnlyThroughCanonicalChain** — After a partition or any incident, authoritative state is reconciled only by the node's own paths over the canonical chain (following and selecting chains through admission and fork choice, and recovery from the durable chain and log) or by a verified re-bootstrap through the single bootstrap authority (STR-013, under SEC-015); no operator command, tool or import writes ledger, chain or consensus state outside those paths. |
| **Code** | _(no enforcement found — gap)_ |
| **Tests** | _(no tests found — gap)_ |
| **CI** | _(no CI enforcement — gap)_ |
| **Claim** | unresolved — no evidence record |

_critical · local · scope: incident handling · assumptions: A4 · depends on: REC-004, DUR-012, STR-014, STR-013, SEC-015._ Legacy: CN-OPS-01, OP-OPS-01

---

## EVO — Evolution

_Doctrine §6: Controlled changes to governing rules, interfaces, permitted behavior, and closed extension surfaces._ 6 invariant(s).

### EVO — foundations

#### `EVO-001` — _partial_

| Aspect | Location |
|--------|----------|
| **Source** | `docs/planning/foundations.define.consensus.md#batch-1` (approved by wdm33, 2026-11-08) |
| **Requirement** | **SingleSemanticInterpretation** — Given identical canonical inputs and identical explicit governing data (protocol version, on-chain protocol parameters, genesis and bootstrap-anchor data), exactly one semantic interpretation exists: no build option, operator configuration value, command-line flag or alternative code path changes ledger, consensus or persistence semantics. Operator configuration may change only transport, logging and telemetry. |
| **Code** | `crates/ade_node/src/epoch_wire.rs`; `crates/ade_node/src/node_lifecycle.rs`; `ci/ci_check_eview_automatic_activation.sh` |
| **Tests** | `maybe_activate_first_boundary_is_automatic_and_fails_closed_not_flag_gated` |
| **CI** | `ci/ci_check_eview_automatic_activation.sh` (red at e563ad94) |
| **Claim** | unresolved — `e563ad94` (fail) — observed: tests: 1 passed, 0 failed, 0 ignored, 0 not found; CI scripts: 0 passed, 1 failed; failing CI: ci/ci_check_eview_automatic_activation.sh — limits: (1) No IDSE cluster has declared acceptance obligations for this invariant yet (Doctrine §10); this record re-runs the checks carried from the legacy registry. Further limits: (2) No mutation witness exists yet for this critical invariant (Doctrine §28). (3) ci/ci_check_eview_automatic_activation.sh fails at this revision and already failed before the IDSE migration (a3cef7f7): "FAIL: the orchestration is not idempotent once a view is promoted". Diagnosed as a stale or over-broad structural gate, not a demonstrated violation; the gate needs repair before it can serve as evidence. |

_critical · local · scope: node-wide · assumptions: A2 · depends on: DET-006 · secondary: DET._ Legacy: T-BUILD-02, CN-BUILD-02, CN-BUILD-03, CN-BUILD-04

### EVO — codec-wire-crypto

#### `EVO-002` — _partial_

| Aspect | Location |
|--------|----------|
| **Source** | `docs/planning/codec-wire-crypto.define.consensus.md#batch-1` (approved by wdm33, 2026-11-08) |
| **Requirement** | **ClosedVersionedMessages** — Every mini-protocol message decodes into exactly one variant of a closed message type for the negotiated protocol version; bytes that match no variant are rejected and end the session, and no variant is chosen by guessing or by trying alternatives. |
| **Code** | `crates/ade_network/src/codec/error.rs`; `crates/ade_network/src/codec/version.rs`; `crates/ade_network/src/codec/handshake.rs`; `crates/ade_network/src/codec/n2c_handshake.rs`; `crates/ade_network/src/codec/chain_sync.rs`; `crates/ade_network/src/codec/block_fetch.rs`; `crates/ade_network/src/codec/tx_submission.rs`; `crates/ade_network/src/codec/keep_alive.rs`; `crates/ade_network/src/codec/peer_sharing.rs`; `crates/ade_network/src/codec/local_chain_sync.rs`; `crates/ade_network/src/codec/local_tx_submission.rs`; `crates/ade_network/src/codec/local_state_query.rs`; `crates/ade_network/src/codec/local_tx_monitor.rs` |
| **Tests** | `codec::handshake::tests::roundtrip_every_variant`, `codec::handshake::tests::decode_rejects_unknown_tag`, `codec::handshake::tests::decode_rejects_truncated_input`, `codec::handshake::tests::decode_rejects_invalid_utf8_in_text_fields`, `codec::n2c_handshake::tests::roundtrip_every_variant`, `codec::n2c_handshake::tests::decode_rejects_unknown_tag`, `codec::n2c_handshake::tests::decode_rejects_truncated_input`, `codec::n2c_handshake::tests::decode_rejects_invalid_utf8_in_text_fields`, `codec::chain_sync::tests::roundtrip_every_variant`, `codec::chain_sync::tests::decode_rejects_unknown_tag`, `codec::chain_sync::tests::decode_rejects_truncated_input`, `codec::block_fetch::tests::roundtrip_every_variant`, `codec::block_fetch::tests::decode_rejects_unknown_tag`, `codec::block_fetch::tests::decode_rejects_truncated_input`, `codec::tx_submission::tests::roundtrip_every_variant`, `codec::tx_submission::tests::decode_rejects_unknown_tag`, `codec::tx_submission::tests::decode_rejects_truncated_input`, `codec::keep_alive::tests::roundtrip_every_variant`, `codec::keep_alive::tests::decode_rejects_unknown_tag`, `codec::keep_alive::tests::decode_rejects_truncated_input`, `codec::peer_sharing::tests::roundtrip_every_variant`, `codec::peer_sharing::tests::decode_rejects_unknown_tag`, `codec::peer_sharing::tests::decode_rejects_truncated_input`, `codec::local_chain_sync::tests::roundtrip_every_variant`, `codec::local_chain_sync::tests::decode_rejects_unknown_tag`, `codec::local_chain_sync::tests::decode_rejects_truncated_input`, `codec::local_tx_submission::tests::roundtrip_every_variant`, `codec::local_tx_submission::tests::decode_rejects_unknown_tag`, `codec::local_tx_submission::tests::decode_rejects_truncated_input`, `codec::local_state_query::tests::roundtrip_every_variant`, `codec::local_state_query::tests::decode_rejects_unknown_tag`, `codec::local_state_query::tests::decode_rejects_truncated_input`, `codec::local_tx_monitor::tests::roundtrip_every_variant`, `codec::local_tx_monitor::tests::decode_rejects_unknown_tag`, `codec::local_tx_monitor::tests::decode_rejects_truncated_input`, `codec::local_tx_monitor::tests::decode_rejects_invalid_utf8_measure_name`, `handshake_real_capture_corpus::real_capture_round_trips_byte_identical`, `chain_sync_real_capture_corpus::real_capture_round_trips_byte_identical`, `chain_sync_real_capture_corpus::rollforward_header_opaque_pass_through`, `block_fetch_real_capture_corpus::real_capture_round_trips_byte_identical`, `block_fetch_real_capture_corpus::block_body_is_wrapped_cbor_item`, `keep_alive_real_capture_corpus::real_capture_round_trips_byte_identical`, `peer_sharing_real_capture_corpus::real_capture_round_trips_byte_identical`, `n2c_handshake_real_capture_corpus::real_capture_round_trips_byte_identical`, `n2c_handshake_real_capture_corpus::accept_version_decodes_to_semantic_version_range` |
| **CI** | `ci/ci_check_codec_message_closed.sh` |
| **Claim** | partial — `e563ad94` (pass) — observed: tests: 45 passed, 0 failed, 0 ignored, 0 not found; CI scripts: 1 passed, 0 failed — limits: (1) No IDSE cluster has declared acceptance obligations for this invariant yet (Doctrine §10); this record re-runs the checks carried from the legacy registry. |

_standard · local · scope: mini-protocol codecs (N2N and N2C) · assumptions: A2 · depends on: VAL-001 · secondary: VAL._ Legacy: CN-WIRE-07

### EVO — ledger

#### `EVO-003` — _declared_

| Aspect | Location |
|--------|----------|
| **Source** | `docs/planning/ledger.define.consensus.md#batch-1` (approved by wdm33, 2026-11-08) |
| **Requirement** | **ReleasesAgreeOnVerdicts** — Any two supported Ade releases that are authorized to run on the network at the same time return the same verdict for every consensus-relevant block and transaction, given the same prior state, inputs, protocol version and explicit context. |
| **Code** | _(no enforcement found — gap)_ |
| **Tests** | _(no tests found — gap)_ |
| **CI** | _(no CI enforcement — gap)_ |
| **Claim** | unresolved — no evidence record |

_critical · distributed · scope: Ade release compatibility · assumptions: A2 · depends on: EVO-001 · secondary: VAL._ Legacy: DC-LEDGER-07, CN-LEDGER-04

### EVO — epoch

#### `EVO-004` — _partial_

| Aspect | Location |
|--------|----------|
| **Source** | `docs/planning/epoch.define.consensus.md#batch-1` (approved by wdm33, 2026-11-08) |
| **Requirement** | **ProtocolVersionChangesAtBoundaries** — A protocol-version change takes effect only at the epoch boundary where the ratified hard-fork action that sets it is enacted, and the era schedule never forecasts past a boundary at which a protocol-version change is not yet fixed. |
| **Code** | `crates/ade_ledger/src/hfc.rs`; `crates/ade_core/src/consensus/era_schedule.rs` |
| **Tests** | `translation_summary_proof::shelley_allegra_summary_matches_oracle`, `translation_comparison_surface::all_non_byron_translations_preserve_sub_state`, `transition_proof_surface::shelley_allegra_transition_proof_surface`, `transition_proof_surface::all_transitions_proof_surface_summary`, `consensus::era_schedule::tests::locate_first_slot_of_each_era`, `consensus::era_schedule::tests::locate_last_slot_of_each_era`, `consensus::era_schedule::tests::forecast_horizon_boundary`, `mainnet_corpus_translation_matches_oracle`, `preprod_corpus_translation_matches_oracle` |
| **CI** | `ci/ci_check_hfc_translation.sh` |
| **Claim** | partial — `e563ad94` (pass) — observed: tests: 5 passed, 0 failed, 0 ignored, 4 not found; CI scripts: 1 passed, 0 failed — limits: (1) No IDSE cluster has declared acceptance obligations for this invariant yet (Doctrine §10); this record re-runs the checks carried from the legacy registry. Further limits: (2) No mutation witness exists yet for this critical invariant (Doctrine §28). (3) 4 named test(s) were not found in the test output at this revision: translation_summary_proof::shelley_allegra_summary_matches_oracle, translation_comparison_surface::all_non_byron_translations_preserve_sub_state, transition_proof_surface::shelley_allegra_transition_proof_surface, transition_proof_surface::all_transitions_proof_surface_summary |

_critical · local · scope: protocol versions, era schedule forecast · assumptions: A2 · depends on: ORD-003, VAL-021, EVO-001 · secondary: ORD._ Legacy: DC-EPOCH-02

### EVO — durability-recovery

#### `EVO-005` — _partial_

| Aspect | Location |
|--------|----------|
| **Source** | `docs/planning/durability-recovery.define.consensus.md#batch-1` (approved by wdm33, 2026-11-08) |
| **Requirement** | **StoreSemanticsVersionGate** — Every durable authority artifact (the chain database with its write-ahead log, the epoch accumulator and the reduced checkpoint) carries its own marker of the semantics version of the rules that produced it, checked when it is opened, before any recovery work; a marker that is absent, older or newer than the binary's fails closed with a typed error whose only remedy is re-bootstrapping. There is no stamping tool, override or bypass; a future migration is allowed only as a sealed, evidenced transform from a marked old store to a new one. |
| **Code** | `crates/ade_ledger/src/store_semantics.rs`; `crates/ade_runtime/src/chaindb/persistent.rs`; `crates/ade_runtime/src/chaindb/epoch_accumulator_store.rs`; `crates/ade_runtime/src/chaindb/reduced_utxo_checkpoint.rs` |
| **Tests** | `current_marker_is_accepted`, `absent_marker_is_rejected_as_legacy`, `older_marker_is_rejected`, `future_marker_is_rejected`, `the_only_remediation_is_rebootstrap`, `fresh_chaindb_is_stamped_and_reopens`, `unmarked_chaindb_is_rejected_as_legacy`, `older_marked_chaindb_is_rejected`, `future_marked_chaindb_is_rejected`, `the_terminal_names_rebootstrap_and_offers_no_override`, `fresh_accumulator_is_stamped_and_reopens`, `stale_accumulator_is_rejected_independently_of_the_chaindb`, `fresh_reduced_checkpoint_is_stamped_and_reopens`, `stale_reduced_checkpoint_is_rejected_independently` |
| **CI** | `ci/ci_check_store_semantics_gate.sh` |
| **Claim** | partial — `e563ad94` (pass) — observed: tests: 14 passed, 0 failed, 0 ignored, 0 not found; CI scripts: 1 passed, 0 failed — limits: (1) No IDSE cluster has declared acceptance obligations for this invariant yet (Doctrine §10); this record re-runs the checks carried from the legacy registry. Further limits: (2) No mutation witness exists yet for this critical invariant (Doctrine §28). |

_critical · local · scope: durable artifacts, binary upgrades · assumptions: — · depends on: EVO-001 · secondary: DUR._ Legacy: DC-STORE-10, DC-STORE-11

### EVO — network

#### `EVO-006` — _partial_

| Aspect | Location |
|--------|----------|
| **Source** | `docs/planning/network.define.consensus.md#batch-1` (approved by wdm33, 2026-11-08) |
| **Requirement** | **ClosedVersionNegotiation** — The node offers and accepts only an enumerated set of node-to-node and node-to-client protocol versions; negotiation selects the version the reference rule selects and, with no common version, refuses deterministically; node-to-client queries are typed, era-aware and available only in the versions that define them. |
| **Code** | `crates/ade_network/src/handshake/mod.rs`; `crates/ade_network/src/handshake/state.rs`; `crates/ade_network/src/handshake/agency.rs`; `crates/ade_network/src/handshake/selection.rs`; `crates/ade_network/src/handshake/transition.rs`; `crates/ade_network/src/handshake/version_table.rs`; `crates/ade_runtime/src/network/n2n_listener.rs`; `crates/ade_runtime/src/orchestrator/n2n_server_pump.rs` |
| **Tests** | `handshake::transition::tests::n2n_happy_path_each_supported_version`, `handshake::transition::tests::n2c_happy_path_each_supported_version`, `handshake::transition::tests::version_mismatch_refused`, `handshake::transition::tests::illegal_message_in_idle_returns_error`, `handshake::transition::tests::wrong_agency_returns_error`, `handshake::transition::tests::overlap_picks_highest_common`, `handshake::transition::tests::empty_intersection_refuses_deterministically`, `handshake::transition::tests::version_data_passed_through_byte_identical`, `handshake::transition::tests::n2n_v15_happy_path`, `handshake::transition::tests::n2n_v16_happy_path_with_peras_support_field`, `handshake::transition::tests::n2c_v21_happy_path`, `handshake::transition::tests::n2c_v22_happy_path`, `handshake::transition::tests::n2c_v23_happy_path`, `handshake::transition::tests::n2c_v15_no_longer_advertised`, `handshake::transition::tests::n2n_overlap_picks_v16_when_both_offer_v14_to_v16`, `version_negotiation_across_supported_table`, `handshake_real_capture_corpus::real_capture_round_trips_byte_identical`, `handshake_real_capture_corpus::iog_capture_accepted_at_known_version`, `handshake_real_capture_corpus::refuse_capture_carries_supported_set`, `n2c_handshake_real_capture_corpus::real_capture_round_trips_byte_identical`, `n2c_handshake_real_capture_corpus::accept_version_decodes_to_semantic_version_range`, `n2n_listener_loopback_handshake_succeeds` |
| **CI** | `ci/ci_check_ce_n_a_5_proof.sh`, `ci/ci_check_n2n_server_no_signing_dep.sh` |
| **Claim** | partial — `e563ad94` (pass) — observed: tests: 22 passed, 0 failed, 0 ignored, 0 not found; CI scripts: 2 passed, 0 failed — limits: (1) No IDSE cluster has declared acceptance obligations for this invariant yet (Doctrine §10); this record re-runs the checks carried from the legacy registry. Further limits: (2) No mutation witness exists yet for this critical invariant (Doctrine §28). |

_critical · local · scope: handshake, node-to-client queries · assumptions: A2 · depends on: VAL-004, EVO-002 · secondary: VAL._ Legacy: DC-PROTO-05, DC-QUERY-01

---

## OBS — Observability

_Doctrine §6: Availability of trustworthy information needed to inspect behavior, diagnose failures, and support verification._ 9 invariant(s).

### OBS — node

#### `OBS-001` — _partial_

| Aspect | Location |
|--------|----------|
| **Source** | `docs/planning/node.define.consensus.md#batch-1` (approved by wdm33, 2026-11-09) |
| **Requirement** | **EveryForgeTickAccounted** — Every forge tick the node admits ends in either a structured refusal carrying its own reason or a leader-schedule decision; none disappears. Refusal reasons are distinct closed discriminators: slot derivation failure; the three KES-window failures (before the operational certificate's start, after its end, period arithmetic overflow), each naming the slot bound and decided by the one checked KES-period function; tip and fence refusals, with the absent-tip reason used only when the tip is actually absent; and the leadership outcome. |
| **Code** | `crates/ade_runtime/src/producer/coordinator.rs`; `crates/ade_node/src/node_sync.rs`; `crates/ade_node/src/live_log/sched_event.rs`; `crates/ade_node/src/node_lifecycle.rs`; `ci/ci_check_forge_slot_authority.sh` |
| **Tests** | `kes_period_for_slot_checked_names_the_reason_and_the_slot_bound`, `the_option_accessor_never_disagrees_with_the_typed_one`, `the_naive_19_day_ahead_slot_was_never_refused_by_the_kes_window`, `ce_l2c_5_and_6_live_instant_derives_the_measured_slot_and_refuses_typed`, `ce_l2c_a5_a_refusal_never_outlives_its_own_tick`, `forge_skip_reason_projects_every_typed_refusal` |
| **CI** | `ci/ci_check_forge_slot_authority.sh` |
| **Claim** | partial — `e563ad94` (pass) — observed: tests: 6 passed, 0 failed, 0 ignored, 0 not found; CI scripts: 1 passed, 0 failed — limits: (1) No IDSE cluster has declared acceptance obligations for this invariant yet (Doctrine §10); this record re-runs the checks carried from the legacy registry. |

_standard · local · scope: forge scheduling, diagnostics · assumptions: A2 · depends on: SEC-005._ Legacy: DC-NODE-46

#### `OBS-002` — _partial_

| Aspect | Location |
|--------|----------|
| **Source** | `docs/planning/node.define.consensus.md#batch-1` (approved by wdm33, 2026-11-09) |
| **Requirement** | **ClosedDiagnosticVocabulary** — The node's feed and forge scheduling diagnostics use a closed, allow-listed event vocabulary with closed reason and outcome sets and no catch-all variant, so a new reason cannot be emitted without being added to the vocabulary; the diagnostics are emit-only, never read by the planner and never changing forge scheduling, base or authority; and an ambiguous or reason-less feed disconnect never makes a feed state forge-eligible. |
| **Code** | `crates/ade_node/src/node_sync.rs`; `crates/ade_node/src/node_lifecycle.rs`; `crates/ade_node/src/live_log/sched_event.rs`; `crates/ade_node/src/live_log/sched_writer.rs` |
| **Tests** | `node_sched_events_emit_closed_vocabulary`, `node_sched_event_allowlist_rejects_unknown_variants` |
| **CI** | `ci/ci_check_node_sched_events_emit_only.sh` |
| **Claim** | partial — `e563ad94` (pass) — observed: tests: 2 passed, 0 failed, 0 ignored, 0 not found; CI scripts: 1 passed, 0 failed — limits: (1) No IDSE cluster has declared acceptance obligations for this invariant yet (Doctrine §10); this record re-runs the checks carried from the legacy registry. |

_standard · local · scope: diagnostics · assumptions: — · depends on: —._ Legacy: CN-NODE-04

#### `OBS-003` — _partial_

| Aspect | Location |
|--------|----------|
| **Source** | `docs/planning/node.define.consensus.md#batch-1` (approved by wdm33, 2026-11-09) |
| **Requirement** | **PeerIdentityPreserved** — The receive path carries each block's origin peer identity through to the consumer as provenance only, so per-peer candidates can be tracked; carrying it changes no selection, admission, rollback or verdict, and a single-peer run replays byte-identically with or without it. |
| **Code** | `crates/ade_node/src/node_sync.rs`; `crates/ade_node/src/node_lifecycle.rs`; `crates/ade_node/src/convergence_evidence.rs`; `ci/ci_check_peer_identity_preserved.sh` |
| **Tests** | `best_of_two_peers_wins_and_is_identified`, `peer_identity_preserved_through_merge` |
| **CI** | `ci/ci_check_peer_identity_preserved.sh` |
| **Claim** | partial — `e563ad94` (pass) — observed: tests: 2 passed, 0 failed, 0 ignored, 0 not found; CI scripts: 1 passed, 0 failed — limits: (1) No IDSE cluster has declared acceptance obligations for this invariant yet (Doctrine §10); this record re-runs the checks carried from the legacy registry. |

_standard · local · scope: receive path · assumptions: A1 · depends on: —._ Legacy: DC-NODE-34

#### `OBS-004` — _partial_

| Aspect | Location |
|--------|----------|
| **Source** | `docs/planning/node.define.consensus.md#batch-1` (approved by wdm33, 2026-11-09) |
| **Requirement** | **ReplayDivergenceSelfDescribing** — A warm-start replay divergence is reported with a structured account naming the log-tail slot, admission count, the ledger's epoch beside the schedule's epoch at that slot, the replay anchor and span, the per-component fingerprints of the anchor and the result, the store's semantics generation and the artifact; an epoch disagreement is named as the fault, and an unreadable anchor is reported distinctly from "nothing changed". |
| **Code** | `crates/ade_ledger/src/replay_divergence.rs`; `crates/ade_node/src/node_lifecycle.rs` |
| **Tests** | `moved_components_names_only_what_changed`, `identical_components_report_none_moved`, `unreadable_anchor_is_not_reported_as_no_movement`, `epoch_disagreement_is_called_out_explicitly`, `unlocatable_schedule_epoch_is_not_a_disagreement`, `report_names_artifact_and_semantics_version` |
| **CI** | `ci/ci_check_recovery_fault_self_describing.sh` |
| **Claim** | partial — `e563ad94` (pass) — observed: tests: 6 passed, 0 failed, 0 ignored, 0 not found; CI scripts: 1 passed, 0 failed — limits: (1) No IDSE cluster has declared acceptance obligations for this invariant yet (Doctrine §10); this record re-runs the checks carried from the legacy registry. |

_standard · local · scope: recovery diagnostics · assumptions: — · depends on: REC-004._ Legacy: DC-NODE-44

#### `OBS-005` — _partial_

| Aspect | Location |
|--------|----------|
| **Source** | `docs/planning/node.define.consensus.md#batch-1` (approved by wdm33, 2026-11-09) |
| **Requirement** | **ConvergenceEvidenceNonAuthoritative** — Convergence evidence emitted on the participant path is a deterministic side output of outcomes already decided: a receipt event for every peer block the receive path considers, an admission event for every admission, and agreement verdicts, every event bound to the consensus-input fingerprint; it never gates admission, triggers or shapes a rollback, influences fork choice or changes the durable chain. A failure to write evidence does not stop the node, but marks the evidence transcript incomplete, so a partial transcript never passes as complete. |
| **Code** | `crates/ade_node/src/node_lifecycle.rs`; `crates/ade_node/src/admission/verdict.rs`; `crates/ade_node/src/admission_log`; `crates/ade_node/src/cli.rs` |
| **Tests** | `ade_node::convergence_evidence::tests::convergence_evidence_absent_path_emits_no_file`, `ade_node::convergence_evidence::tests::convergence_evidence_writer_emits_closed_vocabulary`, `ade_node::convergence_evidence::tests::convergence_evidence_write_failure_poisons_and_is_surfaced`, `ade_node::convergence_evidence::tests::convergence_evidence_context_marks_incomplete_on_write_failure`, `participant_cold_start_admit_emits_received_admitted_agreed`, `participant_block_received_does_not_imply_admission`, `participant_convergence_evidence_replay_byte_identical` |
| **CI** | `ci/ci_check_convergence_evidence_emit_only.sh`, `ci/ci_check_convergence_evidence_vocabulary_closed.sh`, `ci/ci_check_convergence_evidence_schema.sh` |
| **Claim** | partial — `e563ad94` (pass) — observed: tests: 7 passed, 0 failed, 0 ignored, 0 not found; CI scripts: 3 passed, 0 failed — limits: (1) No IDSE cluster has declared acceptance obligations for this invariant yet (Doctrine §10); this record re-runs the checks carried from the legacy registry. Further limits: (2) Corrects this invariant's previous evidence record at the same revision: the earlier record's test matching missed references written as '<crate> tests/<file>.rs::<name>' or '<crate>::<path>'; the test outcomes are the same run's (docs/evidence/idse-m2-reevidence.json). |

_standard · local · scope: participant evidence output · assumptions: — · depends on: —._ Legacy: DC-NODE-30

### OBS — pipeline-production

#### `OBS-006` — _partial_

| Aspect | Location |
|--------|----------|
| **Source** | `docs/planning/pipeline-production.define.consensus.md#batch-1` (approved by wdm33, 2026-11-09) |
| **Requirement** | **AgreementVerdictEvidenceOnly** — The agreement verdict is a closed four-variant value (agreed, lagging, diverged, input not found) computed by a pure reducer from the admission outcome and the peer's announced tip; it compares outcomes already decided and never decides validity, chain selection or canonical state. Lagging means only that no same-slot comparison was possible: the peer's announced tip is at a different slot from the admitted block, ahead or behind, with both slots in the payload so the direction is readable, or the peer announced Origin; it asserts no ancestry and no agreement, and no caller treats it as success, health, readiness or agreement. |
| **Code** | `crates/ade_node/src/admission/verdict.rs` |
| **Tests** | `ade_node::admission::verdict::tests::verdict_agreed_when_hashes_match`, `ade_node::admission::verdict::tests::verdict_diverged_when_our_admit_differs_from_peer_hash`, `ade_node::admission::verdict::tests::verdict_diverged_when_admit_invalid_at_same_slot`, `ade_node::admission::verdict::tests::verdict_lagging_when_peer_ahead_of_our_slot`, `ade_node::admission::verdict::tests::verdict_input_not_found_when_admit_missing_input`, `ade_node::admission::verdict::tests::verdict_lagging_when_peer_tip_is_origin`, `ade_node::admission::verdict::tests::verdict_derive_is_pure_two_runs_byte_identical`, `ade_node::admission::verdict::tests::verdict_kind_discriminator_round_trips_each_variant` |
| **CI** | `ci/ci_check_lagging_is_evidence_only.sh` |
| **Claim** | partial — `e563ad94` (pass) — observed: tests: 8 passed, 0 failed, 0 ignored, 0 not found; CI scripts: 1 passed, 0 failed — limits: (1) No IDSE cluster has declared acceptance obligations for this invariant yet (Doctrine §10); this record re-runs the checks carried from the legacy registry. Further limits: (2) No mutation witness exists yet for this critical invariant (Doctrine §28). (3) Corrects this invariant's previous evidence record at the same revision: the earlier record's test matching missed references written as '<crate> tests/<file>.rs::<name>' or '<crate>::<path>'; the test outcomes are the same run's (docs/evidence/idse-m2-reevidence.json). |

_critical · local · scope: agreement verdict reducer and its callers · assumptions: — · depends on: OBS-005._ Legacy: DC-ADMIT-01, DC-ADMIT-06, DC-ADMIT-08

#### `OBS-007` — _partial_

| Aspect | Location |
|--------|----------|
| **Source** | `docs/planning/pipeline-production.define.consensus.md#batch-1` (approved by wdm33, 2026-11-09) |
| **Requirement** | **AdmissionEvidenceBoundAndReplayable** — Every successful admission emits exactly one agreement-verdict event, never two for one block hash; the admission start, bootstrap-complete, block-admitted and verdict events each carry the consensus-input fingerprint (VAL-034), added as a field so the event vocabulary stays closed; and replaying an admission from the prior checkpoint and log yields the same post-admission ledger fingerprint and the same verdict. |
| **Code** | `crates/ade_node/src/admission/runner.rs`; `crates/ade_node/tests/admission_replay_equivalence.rs`; `crates/ade_node/src/admission_log/event.rs`; `crates/ade_node/src/admission_log/writer.rs` |
| **Tests** | `ade_node::admission::runner::tests::run_admission_emits_shutdown_on_signal`, `admission_replay_equivalence::admission_tip_update_does_not_emit_wal_entry`, `admission_replay_equivalence::admission_replay_equivalence_byte_identical_wal_after_two_runs`, `admission_replay_equivalence::admission_signal_shutdown_returns_clean_exit`, `admission_replay_equivalence::admission_disconnect_to_zero_peers_exits_clean`, `admission_log::writer::tests::admission_log_writer_emits_one_object_per_line`, `admission_log::writer::tests::admission_log_writer_serializes_admission_started_canonically`, `admission_log::writer::tests::admission_log_writer_emits_agreement_verdict_with_kind_field` |
| **CI** | `ci/ci_check_admission_runner_closure.sh` (red at e563ad94), `ci/ci_check_admit_replay_equivalence.sh`, `ci/ci_check_live_ledger_view_epoch_window.sh`, `ci/ci_check_admission_log_vocabulary_closed.sh` |
| **Claim** | unresolved — `e563ad94` (fail) — observed: tests: 8 passed, 0 failed, 0 ignored, 0 not found; CI scripts: 3 passed, 1 failed; failing CI: ci/ci_check_admission_runner_closure.sh — limits: (1) No IDSE cluster has declared acceptance obligations for this invariant yet (Doctrine §10); this record re-runs the checks carried from the legacy registry. Further limits: (2) No mutation witness exists yet for this critical invariant (Doctrine §28). (3) ci/ci_check_admission_runner_closure.sh fails at this revision and already failed before the IDSE migration (a3cef7f7): "FAIL: AdmissionInputs carries Option<_> fields (must be all-required):". Diagnosed as a stale or over-broad structural gate, not a demonstrated violation; the gate needs repair before it can serve as evidence. (4) Corrects this invariant's previous evidence record at the same revision: the earlier record's test matching missed references written as '<crate> tests/<file>.rs::<name>' or '<crate>::<path>'; the test outcomes are the same run's (docs/evidence/idse-m2-reevidence.json). |

_critical · local · scope: admission and convergence evidence · assumptions: A1 · depends on: OBS-006, VAL-034, REC-004 · secondary: DET._ Legacy: DC-ADMIT-02, DC-ADMIT-07, DC-ADMIT-10

#### `OBS-008` — _partial_

| Aspect | Location |
|--------|----------|
| **Source** | `docs/planning/pipeline-production.define.consensus.md#batch-1` (approved by wdm33, 2026-11-09) |
| **Requirement** | **ClosedIsolatedEventVocabularies** — Each mode's evidence log has its own closed event vocabulary (wire-only, admission, producer, and the three-event convergence subset), written through its own sink, and no mode emits another mode's events; events carry no free-form reason strings and no host-dependent content such as file paths, and peers appear in replayable events only as opaque counters, socket addresses being shell metadata outside replay comparison. Keeping key material out of every log is SEC-025. |
| **Code** | `crates/ade_node/src/admission_log`; `crates/ade_runtime/src/producer/producer_log.rs`; `crates/ade_node/src/produce_mode.rs` |
| **Tests** | `ade_node::admission_log::event::tests::admission_log_event_discriminator_round_trips_for_each_variant`, `ade_node::admission_log::event::tests::admission_log_event_match_is_exhaustive`, `ade_node::admission_log::event::tests::admission_log_event_agreement_verdict_carries_kind_discriminator`, `ade_node::admission_log::writer::tests::admission_log_writer_emits_one_object_per_line`, `ade_node::admission_log::writer::tests::admission_log_writer_serializes_admission_started_canonically`, `ade_node::admission_log::writer::tests::admission_log_writer_two_runs_are_byte_identical`, `ade_node::admission_log::writer::tests::admission_log_writer_emits_agreement_verdict_with_kind_field`, `ade_node::admission_log::writer::tests::admission_log_writer_omits_optional_fields_when_none`, `ade_node::admission_log::writer::tests::admission_log_writer_lines_are_parseable_as_one_json_object_per_line`, `event_kinds_are_distinct_and_stable`, `json_serialization_round_trips_byte_identical_for_replay`, `no_string_fields_in_any_variant`, `slot_missed_reason_serializes_to_stable_strings`, `produce_mode_starts_runs_three_slots_and_exits_via_max_slots` |
| **CI** | `ci/ci_check_admission_log_vocabulary_closed.sh`, `ci/ci_check_convergence_evidence_vocabulary_closed.sh` |
| **Claim** | partial — `e563ad94` (pass) — observed: tests: 14 passed, 0 failed, 0 ignored, 0 not found; CI scripts: 2 passed, 0 failed — limits: (1) No IDSE cluster has declared acceptance obligations for this invariant yet (Doctrine §10); this record re-runs the checks carried from the legacy registry. Further limits: (2) Corrects this invariant's previous evidence record at the same revision: the earlier record's test matching missed references written as '<crate> tests/<file>.rs::<name>' or '<crate>::<path>'; the test outcomes are the same run's (docs/evidence/idse-m2-reevidence.json). |

_standard · local · scope: evidence logs, every mode · assumptions: — · depends on: OBS-002, OBS-005, SEC-025._ Legacy: DC-ADMIT-04, DC-PROD-01

### OBS — mithril-evidence

#### `OBS-009` — _partial_

| Aspect | Location |
|--------|----------|
| **Source** | `docs/planning/mithril-evidence.define.consensus.md#batch-1` (approved by wdm33, 2026-11-09) |
| **Requirement** | **ForkChoiceEvidenceClosed** — The live fork-choice path emits a closed, observe-only sequence of evidence events (needs fork choice, common ancestor found, candidate built, selected, branch fetch started and completed, branch prevalidated, then exactly one of switch applied, switch failed or switch superseded), equal to an emit-only allow-list, with bounded typed fields, closed failure codes and a fork-switch id derived deterministically from the winning peer, fork anchor and winning tip; every win is followed by exactly one terminal event for its id; and no evidence event or type is read by chain selection, the ancestor walk, the switch application or the forge fence, so a write failure cannot change their behaviour. |
| **Code** | `crates/ade_node/src/admission_log`; `crates/ade_node/src/convergence_evidence.rs`; `crates/ade_node/src/node_lifecycle.rs`; `ci/ci_check_fork_choice_evidence_closed.sh` |
| **Tests** | `fork_choice_win_paired_with_exactly_one_terminal_applied`, `fork_choice_win_failed_terminal_carries_closed_code`, `superseded_win_pairs_to_superseded_terminal`, `fork_switch_id_is_deterministic_and_bounded` |
| **CI** | `ci/ci_check_fork_choice_evidence_closed.sh` |
| **Claim** | partial — `e563ad94` (pass) — observed: tests: 4 passed, 0 failed, 0 ignored, 0 not found; CI scripts: 1 passed, 0 failed — limits: (1) No IDSE cluster has declared acceptance obligations for this invariant yet (Doctrine §10); this record re-runs the checks carried from the legacy registry. Further limits: (2) No mutation witness exists yet for this critical invariant (Doctrine §28). |

_critical · local · scope: fork-choice evidence · assumptions: — · depends on: OBS-005, STR-016._ Legacy: DC-EVIDENCE-04

---

## PRC — Process Assurance

_Doctrine §6: Mechanically enforced engineering procedures, evidence requirements, and promotion conditions._ 23 invariant(s).

### PRC — foundations

#### `PRC-001` — _partial_

| Aspect | Location |
|--------|----------|
| **Source** | `docs/planning/foundations.define.consensus.md#batch-1` (approved by wdm33, 2026-11-08) |
| **Requirement** | **EveryInvariantMechanicallyEnforced** — Every active invariant in the registry has at least one substantive mechanical check (a test, a CI script, or a type the build compiles) that the registry names, that exists at the current revision, and that the project's check suite (test_cmd and the ci/ scripts) runs; no entry is counted as enforced by waiver. |
| **Code** | _(no enforcement found — gap)_ |
| **Tests** | _(no tests found — gap)_ |
| **CI** | `ci/ci_check_registry_code_locus_exists.sh` |
| **Claim** | falsified — `e563ad94` (fail) — observed: tests: 0 passed, 0 failed, 0 ignored, 0 not found; CI scripts: 1 passed, 0 failed; 20 active invariants name no test, CI script or enforcement locus (idse-export gap 'unprotected'): SAF-002, PRC-002, ORD-001, SEC-002, PRC-003, VAL-015, EVO-003, SEC-015, VAL-040, SEC-031, DUR-014, SEC-034, PRC-012, SEC-037, PRC-014, PRC-015, PRC-016, PRC-017, PRC-020, REC-008 — limits: (1) No IDSE cluster has declared acceptance obligations for this invariant yet (Doctrine §10); this record re-runs the checks carried from the legacy registry. |

_standard · local · scope: invariant registry and check suite · assumptions: — · depends on: —._ Legacy: T-CI-01, CN-META-01

#### `PRC-002` — _declared_

| Aspect | Location |
|--------|----------|
| **Source** | `docs/planning/foundations.define.consensus.md#batch-1` (approved by wdm33, 2026-11-08) |
| **Requirement** | **ReproducibleEquivalenceClaims** — Every claim that Ade agrees with the reference node names the exact reference build, the comparison surface, the fixtures and the replay inputs it was checked against, and rerunning the named check on those reproduces the claimed result; artifacts extracted from one reference build never establish agreement with a different build. |
| **Code** | _(no enforcement found — gap)_ |
| **Tests** | _(no tests found — gap)_ |
| **CI** | _(no CI enforcement — gap)_ |
| **Claim** | unresolved — no evidence record |

_standard · local · scope: compatibility evidence · assumptions: A2 · depends on: DET-001 · secondary: OBS._ Legacy: CN-META-03

### PRC — codec-wire-crypto

#### `PRC-003` — _declared_

| Aspect | Location |
|--------|----------|
| **Source** | `docs/planning/codec-wire-crypto.define.consensus.md#batch-1` (approved by wdm33, 2026-11-08) |
| **Requirement** | **WireCompatibilityPinnedToCapturedBytes** — Every check that Ade's wire encoding or decoding is compatible with the reference node compares against byte strings captured from a real reference node, never against Ade's own encoder output alone. |
| **Code** | _(no enforcement found — gap)_ |
| **Tests** | _(no tests found — gap)_ |
| **CI** | _(no CI enforcement — gap)_ |
| **Claim** | unresolved — no evidence record |

_standard · local · scope: wire-compatibility evidence · assumptions: A2 · depends on: PRC-002 · secondary: OBS._ Legacy: none (new in IDSE)

### PRC — epoch

#### `PRC-004` — _partial_

| Aspect | Location |
|--------|----------|
| **Source** | `docs/planning/epoch.define.consensus.md#batch-1` (approved by wdm33, 2026-11-08) |
| **Requirement** | **PerNetworkEpochEvidence** — Epoch derivation, boundary firing (exactly at the first slot of a new epoch), ledger-schedule agreement and the nonce and candidate-freeze surface are checked by a differential suite on every network Ade supports, over that network's real geometry and across bootstrap positions before and after the stability window, never inferred from a mainnet-shaped corpus alone. |
| **Code** | `crates/ade_ledger/src/state.rs`; `crates/ade_node/src/native_firstrun.rs`; `crates/ade_node/src/bootstrap_export.rs`; `ci/ci_check_venue_differential.sh`; `crates/ade_ledger/src/bootstrap_bridge.rs`; `crates/ade_node/src/epoch_wire.rs` |
| **Tests** | `venue_differential_epoch_derivation_is_correct_for_every_venue`, `venue_differential_boundary_fires_exactly_at_each_venue_boundary`, `venue_differential_epoch_agreement_discriminates_for_every_venue`, `venue_differential_mainnet_formula_is_wrong_off_mainnet`, `dc_epoch_38_bridge_nonce_freeze_differential_covers_both_sides_per_venue`, `n2_pre_freeze_seed_binds_the_boundary_tick_not_the_stored_projection`, `n2_final_seed_must_corroborate_the_boundary_tick`, `n2_underivable_finality_is_terminal`, `n2_live_and_recovery_bind_the_same_view_byte_identically`, `n2_recovery_nonce_must_be_bound_to_the_target_epoch`, `preprod_snapshot_6009_seed_is_pending`, `seed_exactly_at_the_freeze_slot_is_final`, `preview_geometry_decides_identically_at_the_same_relative_position`, `oversized_rsw_saturates_and_is_final` |
| **CI** | `ci/ci_check_venue_differential.sh`, `ci/ci_check_bridge_nonce_freeze_differential.sh` |
| **Claim** | partial — `e563ad94` (pass) — observed: tests: 14 passed, 0 failed, 0 ignored, 0 not found; CI scripts: 2 passed, 0 failed — limits: (1) No IDSE cluster has declared acceptance obligations for this invariant yet (Doctrine §10); this record re-runs the checks carried from the legacy registry. |

_standard · local · scope: epoch and nonce evidence · assumptions: A2 · depends on: PRC-002._ Legacy: DC-EPOCH-37, DC-EPOCH-38

### PRC — durability-recovery

#### `PRC-005` — _partial_

| Aspect | Location |
|--------|----------|
| **Source** | `docs/planning/durability-recovery.define.consensus.md#batch-1` (approved by wdm33, 2026-11-08) |
| **Requirement** | **SemanticsSurfaceLocked** — The code whose behaviour determines what the durable artifacts contain is listed and content-hashed in a lock file, and any change to it fails CI until the author either raises the semantics version or records the change as semantics-neutral; the lock is append-only, every entry carries a rationale, versions never decrease, every non-neutral entry strictly increases the version, and the last entry agrees with the code's version constant. |
| **Code** | `ci/store-semantics-surface.lock`; `ci/ci_check_store_semantics_lock.sh`; `crates/ade_ledger/src/store_semantics.rs` |
| **Tests** | _(no tests found — gap)_ |
| **CI** | `ci/ci_check_store_semantics_lock.sh` |
| **Claim** | partial — `e563ad94` (pass) — observed: tests: 0 passed, 0 failed, 0 ignored, 0 not found; CI scripts: 1 passed, 0 failed — limits: (1) No IDSE cluster has declared acceptance obligations for this invariant yet (Doctrine §10); this record re-runs the checks carried from the legacy registry. |

_standard · local · scope: CI, semantics versioning · assumptions: — · depends on: EVO-005 · secondary: EVO._ Legacy: DC-STORE-12

### PRC — network

#### `PRC-006` — _partial_

| Aspect | Location |
|--------|----------|
| **Source** | `docs/planning/network.define.consensus.md#batch-1` (approved by wdm33, 2026-11-08) |
| **Requirement** | **CompatibilityOnObservableSurfaces** — Agreement with the reference node is evidenced only on observable surfaces (per-block verdicts, the selected tip hash, block hashes, cardano-cli query results and protocol transcripts) with pinned reference versions and reproducible inputs; comparing Ade's internal ledger fingerprint with a hash of the reference node's serialized state is forbidden, and fingerprint equality is asserted only between Ade's own paths. |
| **Code** | `crates/ade_testkit/src/harness/sync_diff.rs`; `ci/ci_check_no_haskell_fingerprint_equality.sh` |
| **Tests** | `sync_differential_snapshot_to_tip` |
| **CI** | `ci/ci_check_no_haskell_fingerprint_equality.sh` (red at e563ad94) |
| **Claim** | unresolved — `e563ad94` (fail) — observed: tests: 1 passed, 0 failed, 0 ignored, 0 not found; CI scripts: 0 passed, 1 failed; failing CI: ci/ci_check_no_haskell_fingerprint_equality.sh — limits: (1) No IDSE cluster has declared acceptance obligations for this invariant yet (Doctrine §10); this record re-runs the checks carried from the legacy registry. Further limits: (2) ci/ci_check_no_haskell_fingerprint_equality.sh fails at this revision and already failed before the IDSE migration (a3cef7f7): "[ci_check_no_haskell_fingerprint_equality] FAIL — forbidden Ade-fingerprint == Haskell/oracle serialized-state-hash assertion(s):". Diagnosed as a stale or over-broad structural gate, not a demonstrated violation; the gate needs repair before it can serve as evidence. |

_standard · local · scope: compatibility evidence · assumptions: A2 · depends on: PRC-002 · secondary: OBS._ Legacy: DC-COMPAT-01

### PRC — mithril-evidence

#### `PRC-007` — _partial_

| Aspect | Location |
|--------|----------|
| **Source** | `docs/planning/mithril-evidence.define.consensus.md#batch-1` (approved by wdm33, 2026-11-09) |
| **Requirement** | **DifferentialLocalizesFirstDivergence** — Every differential comparison between Ade and the reference node reports the first point at which the two disagree (the first block, transaction, event or field whose outcome differs, with its position), never only that the runs differ. |
| **Code** | `crates/ade_testkit/src/harness/ledger_diff.rs` |
| **Tests** | `diff_ledger_sequence`, `mismatched_block_count_returns_error` |
| **CI** | _(no CI enforcement — gap)_ |
| **Claim** | partial — `e563ad94` (pass) — observed: tests: 1 passed, 0 failed, 0 ignored, 1 not found; CI scripts: 0 passed, 0 failed — limits: (1) No IDSE cluster has declared acceptance obligations for this invariant yet (Doctrine §10); this record re-runs the checks carried from the legacy registry. Further limits: (2) 1 named test(s) were not found in the test output at this revision: diff_ledger_sequence |

_standard · local · scope: differential harnesses · assumptions: A2 · depends on: PRC-002 · secondary: OBS._ Legacy: DC-DIFF-01

#### `PRC-008` — _partial_

| Aspect | Location |
|--------|----------|
| **Source** | `docs/planning/mithril-evidence.define.consensus.md#batch-1` (approved by wdm33, 2026-11-09) |
| **Requirement** | **OperatorPassTranscript** — An operator pass against the reference node peer counts as passing only if its committed transcript against the project's canonical local reference-node peer contains an admission-start event, a bootstrap-complete event and at least one block-admitted event, each carrying the consensus-input fingerprint, at least one agreed verdict, no diverged verdict, and no admitted block whose hash differs from the block the peer announced at the same slot; lagging verdicts do not count either way. |
| **Code** | `crates/ade_node/tests/admission_live_operator_pass.rs`; `ci/build_consensus_inputs_bundle.sh`; `crates/ade_runtime/src/seed_import`; `docs/evidence/phase4-n-m-c-consensus-inputs.json`; `docs/evidence/phase4-n-m-c-wire-only-transcript.jsonl`; `docs/evidence/phase4-n-m-a1.1-admission-bootstrap-transcript.jsonl`; `docs/evidence/phase4-n-m-c-operator-pass-README.md` |
| **Tests** | `live_operator_pass_against_docker_preprod`, `live_bundle_imports_with_conway_era_and_deterministic_fingerprint` |
| **CI** | `ci/ci_check_live_operator_pass_scaffold.sh` |
| **Claim** | partial — `e563ad94` (pass) — observed: tests: 2 passed, 0 failed, 0 ignored, 0 not found; CI scripts: 1 passed, 0 failed — limits: (1) No IDSE cluster has declared acceptance obligations for this invariant yet (Doctrine §10); this record re-runs the checks carried from the legacy registry. |

_standard · distributed · scope: operator-pass evidence · assumptions: A2 · depends on: PRC-002, OBS-005._ Legacy: DC-EVIDENCE-01

#### `PRC-009` — _partial_

| Aspect | Location |
|--------|----------|
| **Source** | `docs/planning/mithril-evidence.define.consensus.md#batch-1` (approved by wdm33, 2026-11-09) |
| **Requirement** | **ConvergenceThroughReorgTranscript** — A participant convergence pass counts as passing only if its one transcript, whose hash the committed manifest binds, shows a strict slot regression in the observed peer-block sequence (a peer roll backward was followed), at least one agreed verdict at the re-converged tip and no diverged verdict; a run in which the tip never regresses does not count. |
| **Code** | `ci/ci_check_convergence_evidence_schema.sh`; `docs/active/phase4-n-ai-convergence-runbook.md` |
| **Tests** | _(no tests found — gap)_ |
| **CI** | `ci/ci_check_convergence_evidence_schema.sh` |
| **Claim** | partial — `e563ad94` (pass) — observed: tests: 0 passed, 0 failed, 0 ignored, 0 not found; CI scripts: 1 passed, 0 failed — limits: (1) No IDSE cluster has declared acceptance obligations for this invariant yet (Doctrine §10); this record re-runs the checks carried from the legacy registry. |

_standard · distributed · scope: participant convergence evidence · assumptions: A2 · depends on: PRC-002, OBS-005._ Legacy: DC-EVIDENCE-03

#### `PRC-010` — _partial_

| Aspect | Location |
|--------|----------|
| **Source** | `docs/planning/mithril-evidence.define.consensus.md#batch-1` (approved by wdm33, 2026-11-09) |
| **Requirement** | **SyncEvidenceManifest** — A committed snapshot-to-tip sync-evidence manifest carries a closed schema (the reference versions, the chain point, the fixture references with their SHA-256 hashes and the comparison result), each named hash matches the committed fixture, and each mismatch found against the reference node becomes a named regression fixture. |
| **Code** | `ci/ci_check_sync_evidence_manifest_schema.sh`; `corpus/sync/regressions`; `crates/ade_testkit/src/harness/sync_diff.rs` |
| **Tests** | `regression_fixture_per_mismatch` |
| **CI** | `ci/ci_check_sync_evidence_manifest_schema.sh` |
| **Claim** | partial — `e563ad94` (pass) — observed: tests: 1 passed, 0 failed, 0 ignored, 0 not found; CI scripts: 1 passed, 0 failed — limits: (1) No IDSE cluster has declared acceptance obligations for this invariant yet (Doctrine §10); this record re-runs the checks carried from the legacy registry. |

_standard · local · scope: sync evidence · assumptions: A2 · depends on: PRC-002._ Legacy: RO-SYNC-EVIDENCE-01

#### `PRC-011` — _partial_

| Aspect | Location |
|--------|----------|
| **Source** | `docs/planning/mithril-evidence.define.consensus.md#batch-1` (approved by wdm33, 2026-11-09) |
| **Requirement** | **ReferenceExtractionReproducible** — Every reference artifact used to claim agreement (a fixture, a corpus entry, a captured transcript) names its reference source, the method by which it was extracted from the reference node, and a path for reproducing that extraction, so the artifact can be regenerated from the reference independently of Ade. |
| **Code** | `crates/ade_testkit/src/harness/provenance.rs` |
| **Tests** | `validate_complete_manifest_no_violations`, `validate_empty_manifest_no_violations`, `validate_detects_empty_field`, `self_comparison_zero_divergences_byron`, `self_comparison_zero_divergences_shelley`, `self_comparison_zero_divergences_allegra`, `self_comparison_zero_divergences_mary`, `self_comparison_zero_divergences_alonzo`, `self_comparison_zero_divergences_babbage`, `self_comparison_zero_divergences_conway` |
| **CI** | `ci/ci_check_ref_provenance.sh` |
| **Claim** | partial — `e563ad94` (pass) — observed: tests: 10 passed, 0 failed, 0 ignored, 0 not found; CI scripts: 1 passed, 0 failed — limits: (1) No IDSE cluster has declared acceptance obligations for this invariant yet (Doctrine §10); this record re-runs the checks carried from the legacy registry. |

_standard · local · scope: reference fixtures and corpora · assumptions: A2 · depends on: PRC-002._ Legacy: DC-REF-01

#### `PRC-012` — _declared_

| Aspect | Location |
|--------|----------|
| **Source** | `docs/planning/mithril-evidence.define.consensus.md#batch-1` (approved by wdm33, 2026-11-09) |
| **Requirement** | **ForkSwitchConvergencePromotion** — The claim of LIV-003 is promoted only on a committed two-producer transcript showing both peers' blocks received, the full fork-choice sequence of OBS-009, the switch applied at X for a fork-choice win and X admitted, followed within a bounded window by DET-023's continues-selected-branch verdict ending either in an agreed verdict with equal hashes at X or a descendant, or in a validated prefix of the peer's chain with the peer observed ahead, and no diverged verdict; never on diagnostics or a momentary equal tip. |
| **Code** | _(no enforcement found — gap)_ |
| **Tests** | _(no tests found — gap)_ |
| **CI** | _(no CI enforcement — gap)_ |
| **Claim** | unresolved — no evidence record |

_standard · distributed · scope: partition-convergence evidence · assumptions: A2, A5 · depends on: LIV-003, OBS-009, DET-023._ Legacy: none (new in IDSE)

### PRC — resources-release

#### `PRC-013` — _partial_

| Aspect | Location |
|--------|----------|
| **Source** | `docs/planning/resources-release.define.consensus.md#batch-1` (approved by wdm33, 2026-11-09) |
| **Requirement** | **OwnedMemoryBelowReference** — Measured on the same venue and chain, Ade's owned resident memory (private dirty plus anonymous resident memory) stays below the reference node's, by a measurement method recorded with the evidence, and no change made to reduce memory alters ledger semantics, chain selection, persisted bytes or replay. |
| **Code** | `crates/ade_node/src/mem_measure/rss_sampler.rs`; `crates/ade_ledger/src/fingerprint.rs`; `crates/ade_node/src/admission/bootstrap.rs`; `ci/ci_check_mem_opt_s3_owned.sh`; `ci/ci_check_utxo_fp_cache.sh` |
| **Tests** | `owned_samplers_present_on_linux`, `static_utxo_fp_fails_closed_under_track_utxo_true_and_version_mismatch` |
| **CI** | _(no CI enforcement — gap)_ |
| **Claim** | partial — `e563ad94` (pass) — observed: tests: 2 passed, 0 failed, 0 ignored, 0 not found; CI scripts: 0 passed, 0 failed — limits: (1) No IDSE cluster has declared acceptance obligations for this invariant yet (Doctrine §10); this record re-runs the checks carried from the legacy registry. |

_standard · distributed · scope: resource evidence · assumptions: A2 · depends on: DET-025, DET-001._ Legacy: OP-MEM-02

#### `PRC-014` — _declared_

| Aspect | Location |
|--------|----------|
| **Source** | `docs/planning/resources-release.define.consensus.md#batch-1` (approved by wdm33, 2026-11-09) |
| **Requirement** | **DifferentialFuzzingReleaseBlocking** — Consensus-relevant inputs (blocks, headers, transactions and wire messages) are fuzzed differentially against the reference node across every supported protocol version and every decode and validation path, and accept/reject agreement with the reference on the pinned authoritative corpora (blocks, transactions and ledger-state hashes) is checked before every release; any verdict mismatch from either blocks a release until resolved. |
| **Code** | _(no enforcement found — gap)_ |
| **Tests** | _(no tests found — gap)_ |
| **CI** | _(no CI enforcement — gap)_ |
| **Claim** | unresolved — no evidence record |

_critical · local · scope: release · assumptions: A2 · depends on: VAL-007, VAL-008, PRC-002._ Legacy: CN-TEST-01, RO-TEST-01, CN-REL-03, RO-REL-02

#### `PRC-015` — _declared_

| Aspect | Location |
|--------|----------|
| **Source** | `docs/planning/resources-release.define.consensus.md#batch-1` (approved by wdm33, 2026-11-09) |
| **Requirement** | **PermanentRegressionCorpus** — Every input that ever caused a fork, a venue mismatch or a parser disagreement with the reference becomes a named, permanent entry in the regression corpus, checked on every run of the test suite. |
| **Code** | _(no enforcement found — gap)_ |
| **Tests** | _(no tests found — gap)_ |
| **CI** | _(no CI enforcement — gap)_ |
| **Claim** | unresolved — no evidence record |

_standard · local · scope: test corpus · assumptions: A2 · depends on: PRC-001._ Legacy: CN-TEST-02, RO-TEST-02

#### `PRC-016` — _declared_

| Aspect | Location |
|--------|----------|
| **Source** | `docs/planning/resources-release.define.consensus.md#batch-1` (approved by wdm33, 2026-11-09) |
| **Requirement** | **MixedVersionEvidenceBeforeRelease** — A release is eligible for a network only with evidence that it agrees with every supported predecessor running alongside it (EVO-003, SEC-013) on malformed and boundary-case inputs. |
| **Code** | _(no enforcement found — gap)_ |
| **Tests** | _(no tests found — gap)_ |
| **CI** | _(no CI enforcement — gap)_ |
| **Claim** | unresolved — no evidence record |

_standard · distributed · scope: release · assumptions: A2 · depends on: EVO-003, SEC-013._ Legacy: CN-REL-01, RO-REL-01

#### `PRC-017` — _declared_

| Aspect | Location |
|--------|----------|
| **Source** | `docs/planning/resources-release.define.consensus.md#batch-1` (approved by wdm33, 2026-11-09) |
| **Requirement** | **UnmaskedCloseGate** — A cluster closes only on the real exit status of an unmasked full test run; piped or filtered output is display only, never the pass or fail authority; and a slice that changes canonical bytes, encoded forms, decoder inputs or golden fixtures closes only after every consumer of the changed output across all crates has been found and checked. |
| **Code** | _(no enforcement found — gap)_ |
| **Tests** | _(no tests found — gap)_ |
| **CI** | _(no CI enforcement — gap)_ |
| **Claim** | unresolved — no evidence record |

_standard · local · scope: cluster closure · assumptions: — · depends on: PRC-001._ Legacy: RO-CLOSE-01

#### `PRC-018` — _partial_

| Aspect | Location |
|--------|----------|
| **Source** | `docs/planning/resources-release.define.consensus.md#batch-1` (approved by wdm33, 2026-11-09) |
| **Requirement** | **PeerAcceptanceEvidenceCorrelated** — Peer-acceptance evidence for forged blocks is a closed, versioned manifest built only by one pure correlator: the forged block's hash is the required correlation key, a peer signal's slot must agree when present, conflicting peer signals yield no evidence, the peer-log parser accepts only the allow-listed acceptance lines (a self-acceptance, receipt or verdict line never counts), the forged block's hash and slot are read from the forge artifact, never recomputed or parsed from block bytes; and the manifest names the Ade commit, reference node and cli versions, network, block hash, slot, operational-certificate and genesis fingerprints, the Ade evidence file, the raw peer log with its SHA-256 (which must match the committed log), the command that captured it, the filter used to display it (documentation only, never authority) and the acceptance keyword it matched. |
| **Code** | `ci/ci_check_operator_evidence_manifest_schema.sh`; `docs/evidence/phase4-n-f-g-c-operator-pass-README.md`; `ci/ci_check_ba02_evidence_manifest_schema.sh`; `crates/ade_node/src/ba02_evidence.rs`; `crates/ade_node/src/ba02_pass.rs`; `ci/ci_check_ba02_evidence_closed.sh` |
| **Tests** | `ba02_manifest_schema_round_trips`, `ba02_correlate_served_block_yields_manifest`, `ba02_correlate_chain_tip_only_yields_manifest`, `ba02_correlate_both_signals_agree_records_served_primary`, `ba02_correlate_served_block_without_slot_yields_manifest`, `ba02_correlate_conflicting_signals_is_no_evidence`, `ba02_correlate_wrong_hash_is_no_evidence`, `ba02_correlate_chain_point_mismatch_is_no_evidence`, `ba02_correlate_no_slot_wrong_hash_is_no_evidence`, `ba02_correlate_stale_log_is_no_evidence`, `ba02_correlate_empty_peer_log_is_no_evidence`, `ba02_self_accept_is_not_evidence`, `ba02_forge_succeeded_alone_is_not_evidence`, `ba02_block_received_alone_is_not_evidence`, `ba02_agreement_verdict_is_not_evidence`, `ba02_parser_accepts_well_formed_served_block`, `ba02_parser_drops_unknown_and_malformed_lines`, `ba02_correlate_two_runs_byte_identical`, `correlate_wired_to_operator_peer_log`, `correlate_from_operator_log_file_is_deterministic` |
| **CI** | `ci/ci_check_operator_evidence_manifest_schema.sh`, `ci/ci_check_ba02_evidence_manifest_schema.sh`, `ci/ci_check_ba02_evidence_closed.sh` |
| **Claim** | partial — `e563ad94` (pass) — observed: tests: 20 passed, 0 failed, 0 ignored, 0 not found; CI scripts: 3 passed, 0 failed — limits: (1) No IDSE cluster has declared acceptance obligations for this invariant yet (Doctrine §10); this record re-runs the checks carried from the legacy registry. Further limits: (2) No mutation witness exists yet for this critical invariant (Doctrine §28). |

_critical · local · scope: acceptance evidence · assumptions: A2 · depends on: PRC-002._ Legacy: RO-LIVE-06, CN-OPERATOR-EVIDENCE-01

#### `PRC-019` — _partial_

| Aspect | Location |
|--------|----------|
| **Source** | `docs/planning/resources-release.define.consensus.md#batch-1` (approved by wdm33, 2026-11-09) |
| **Requirement** | **RehearsalNonPromotable** — A private-testnet rehearsal of block acceptance runs the same node-mode path as the public venue, with no rehearsal-only flag, branch, bootstrap authority or constructor, differing only in operator inputs and the evidence label; its evidence is marked rehearsal, stored only in the rehearsal location, built by the same correlator, and never counted or referenced as public-venue acceptance evidence. |
| **Code** | `crates/ade_node/src/ba02_evidence.rs`; `crates/ade_node/src/ba02_pass.rs`; `crates/ade_runtime/src/consensus_inputs`; `docs/evidence/phase4-n-f-g-c-operator-pass-README.md`; `ci/ci_check_rehearsal_manifest_schema.sh`; `ci/ci_check_node_path_fidelity.sh`; `crates/ade_node/tests/forge_succeeds.rs`; `crates/ade_node/tests/node_c1_genesis_rehearsal.rs`; `docs/evidence/phase4-n-f-g-j-genesis-rehearsal-README.md`; `ci/ci_check_genesis_successor_reachability.sh` |
| **Tests** | `node_accepted_block_consensus_inputs_via_shared_import`, `rehearsal_envelope_wraps_correlate_produced_payload`, `rehearsal_correlate_no_evidence_writes_nothing`, `rehearsal_envelope_is_structurally_distinct_from_ba02_manifest`, `c1_dry_run_correlate_to_rehearsal_envelope`, `node_c1_dry_run_rehearsal_live`, `rehearsal_gate_fails_on_archived_home_leak`, `genesis_rehearsal_manifest_binds_block_zero_genesis`, `genesis_rehearsal_no_evidence_writes_nothing`, `node_c1_genesis_rehearsal_live` |
| **CI** | `ci/ci_check_node_path_fidelity.sh` (red at e563ad94), `ci/ci_check_rehearsal_manifest_schema.sh`, `ci/ci_check_genesis_successor_reachability.sh` |
| **Claim** | unresolved — `e563ad94` (fail) — observed: tests: 10 passed, 0 failed, 0 ignored, 0 not found; CI scripts: 2 passed, 1 failed; failing CI: ci/ci_check_node_path_fidelity.sh — limits: (1) No IDSE cluster has declared acceptance obligations for this invariant yet (Doctrine §10); this record re-runs the checks carried from the legacy registry. Further limits: (2) ci/ci_check_node_path_fidelity.sh fails at this revision and already failed before the IDSE migration (a3cef7f7): "FAIL (node path fidelity): cli.rs flag set diverged from the pinned closed allow-list (G-D must add no flag; a private-only / venue flag is forbidden). diff (< pinned / > current):". Diagnosed as a stale or over-broad structural gate, not a demonstrated violation; the gate needs repair before it can serve as evidence. |

_standard · local · scope: rehearsal evidence · assumptions: A2 · depends on: PRC-018._ Legacy: CN-REHEARSAL-FIDELITY-01

#### `PRC-020` — _declared_

| Aspect | Location |
|--------|----------|
| **Source** | `docs/planning/resources-release.define.consensus.md#batch-1` (approved by wdm33, 2026-11-09) |
| **Requirement** | **SyncLiveLegWitnessed** — A claim that Ade syncs from a certified snapshot to the reference tip is supported only with an operator-witnessed live run on a private Conway testnet of two reference nodes, its transcript and peer logs committed with their hashes under mithril-evidence's SyncEvidenceManifest. |
| **Code** | _(no enforcement found — gap)_ |
| **Tests** | _(no tests found — gap)_ |
| **CI** | _(no CI enforcement — gap)_ |
| **Claim** | unresolved — no evidence record |

_standard · distributed · scope: sync evidence · assumptions: A2 · depends on: PRC-002._ Legacy: none (new in IDSE)

#### `PRC-021` — _partial_

| Aspect | Location |
|--------|----------|
| **Source** | `docs/planning/resources-release.define.consensus.md#batch-1` (approved by wdm33, 2026-11-09) |
| **Requirement** | **LiveFollowWindow** — A live follow or admission-agreement pass against the reference peer counts only if its transcript covers at least 30 continuous minutes, records the peer's tip, Ade's tip and the agreement verdict for every admitted block, and meets mithril-evidence's OperatorPassTranscript conditions. |
| **Code** | `crates/ade_runtime/tests/receive_pipeline_corpus_drive.rs`; `crates/ade_core_interop/src/bin/live_block_follow_session.rs`; `crates/ade_node/src/admission`; `crates/ade_runtime/src/admission/wire_pump.rs`; `crates/ade_runtime/src/consensus_inputs`; `crates/ade_runtime/src/seed_import` |
| **Tests** | `receive_pipeline_corpus_drive_admits_every_block`, `receive_pipeline_corpus_drive_chaindb_tip_matches_expected`, `receive_pipeline_corpus_drive_admitted_bytes_equal_corpus_bytes`, `receive_pipeline_corpus_drive_ledger_fingerprint_changes_on_admit`, `live_block_follow_session_hermetic_default_prints_readiness`, `live_operator_pass_against_docker_preprod`, `live_bundle_imports_with_conway_era_and_deterministic_fingerprint`, `cross_epoch_block_triggers_halt_without_admit`, `adversarial_corpus_rejects_all_four_mutation_classes` |
| **CI** | `ci/ci_check_receive_paths_corpus_present.sh`, `ci/ci_check_live_operator_pass_scaffold.sh` |
| **Claim** | partial — `e563ad94` (pass) — observed: tests: 9 passed, 0 failed, 0 ignored, 0 not found; CI scripts: 2 passed, 0 failed — limits: (1) No IDSE cluster has declared acceptance obligations for this invariant yet (Doctrine §10); this record re-runs the checks carried from the legacy registry. |

_standard · distributed · scope: live evidence · assumptions: A2 · depends on: PRC-002._ Legacy: RO-LIVE-02, RO-LIVE-03, RO-LIVE-05

#### `PRC-022` — _partial_

| Aspect | Location |
|--------|----------|
| **Source** | `docs/planning/resources-release.define.consensus.md#batch-1` (approved by wdm33, 2026-11-09) |
| **Requirement** | **WireSmokePass** — A wire-smoke pass against the reference peer counts only if the wire-only run completes the node-to-node handshake, sends one FindIntersect offering Origin, records the peer's announced tip, emits only the closed wire-only event vocabulary (never admission, verdict or ledger events) and exits cleanly. |
| **Code** | `crates/ade_node/src/wire_only.rs`; `crates/ade_node/src/live_log/event.rs`; `crates/ade_node/src/live_log/writer.rs`; `crates/ade_node/src/main.rs` |
| **Tests** | `crates/ade_node/tests/wire_only_loopback.rs::main_wire_only_exits_zero_after_tip_read`, `crates/ade_node/tests/wire_only_loopback.rs::main_wire_only_emits_peer_tip_read_with_responder_tip`, `crates/ade_node/tests/wire_only_loopback.rs::main_wire_only_never_emits_agreement_verdict`, `crates/ade_node/tests/wire_only_loopback.rs::main_without_genesis_does_not_attempt_admission`, `crates/ade_node/tests/wire_only_loopback.rs::peer_dial_failure_exits_nonzero_with_error_event`, `crates/ade_node/tests/wire_only_loopback.rs::admission_mode_fails_closed_with_ledger_seed_unavailable`, `crates/ade_node/tests/wire_only_loopback.rs::jsonl_events_are_valid_one_object_per_line`, `crates/ade_node/src/live_log/writer.rs::tests::live_log_writer_emits_one_object_per_line`, `crates/ade_node/src/live_log/writer.rs::tests::live_log_writer_serializes_node_started_canonically`, `crates/ade_node/src/live_log/writer.rs::tests::live_log_writer_two_runs_are_byte_identical`, `crates/ade_node/src/live_log/event.rs::tests::live_log_event_match_is_exhaustive` |
| **CI** | `ci/ci_check_wire_only_event_vocabulary_closed.sh`, `ci/ci_check_wire_only_no_bootstrap.sh` |
| **Claim** | partial — `e563ad94` (pass) — observed: tests: 11 passed, 0 failed, 0 ignored, 0 not found; CI scripts: 2 passed, 0 failed — limits: (1) No IDSE cluster has declared acceptance obligations for this invariant yet (Doctrine §10); this record re-runs the checks carried from the legacy registry. |

_standard · distributed · scope: live evidence · assumptions: A2 · depends on: VAL-004._ Legacy: RO-LIVE-04

#### `PRC-023` — _partial_

| Aspect | Location |
|--------|----------|
| **Source** | `docs/planning/resources-release.define.consensus.md#batch-1` (approved by wdm33, 2026-11-09) |
| **Requirement** | **LiveAcceptanceReleaseGate** — Block production is claimed, and a release presented as block-producing, only with committed evidence built under PRC-018 from a sustained live session on a public venue in which a reference cardano-node peer adopted Ade-forged blocks delivered over chain-sync and block-fetch. Without stake able to win slots on the venue the gate is unmet and the claim stays unresolved; it is never waived, and rehearsal evidence (PRC-019) never satisfies it. |
| **Code** | `crates/ade_testkit/src/producer/cross_impl_adapter.rs`; `crates/ade_runtime/src/producer/coordinator.rs`; `crates/ade_runtime/src/producer/producer_shell.rs`; `crates/ade_node/src/produce_mode.rs`; `crates/ade_core_interop/src/bin/live_block_production_session.rs` |
| **Tests** | `cross_impl_adapter_forged_block_decodes_through_ade_codec`, `cross_impl_adapter_forged_block_structurally_agrees_with_decoder`, `cross_impl_adapter_corpus_round_trips_byte_identical` |
| **CI** | `ci/ci_check_producer_corpus_present.sh` |
| **Claim** | partial — `e563ad94` (pass) — observed: tests: 3 passed, 0 failed, 0 ignored, 0 not found; CI scripts: 1 passed, 0 failed — limits: (1) No IDSE cluster has declared acceptance obligations for this invariant yet (Doctrine §10); this record re-runs the checks carried from the legacy registry. Further limits: (2) No mutation witness exists yet for this critical invariant (Doctrine §28). (3) No live public-venue acceptance evidence is committed; the gate is unmet and the claim cannot rise above partial. |

_critical · distributed · scope: release, block-production claim · assumptions: A2 · depends on: PRC-018, PRC-019._ Legacy: CN-CONS-06

---

## Deprecated rules

_None._ All 260 registry entries are `status = "active"`. A deprecated invariant keeps its ID and moves here with its `deprecated_in`.

---

## Enforcement gaps (need attention)

Surfaced for human judgment. This pass changes no registry field and invents no rule or mechanism.

### G1 — 20 invariants with no mechanism at all (Code, Tests and CI all empty)

`SEC-002`, `SEC-015`, `SEC-031`, `SEC-034`, `SEC-037`, `SAF-002`, `VAL-015`, `VAL-040`, `ORD-001`, `DUR-014`, `REC-008`, `EVO-003`, `PRC-002`, `PRC-003`, `PRC-012`, `PRC-014`, `PRC-015`, `PRC-016`, `PRC-017`, `PRC-020`

All `_declared_`. 13 of the 20 are `critical`. This is exactly the list in PRC-001's evidence record (`EveryInvariantMechanicallyEnforced`, claim `falsified`): the invariant is falsified by these entries, not by a code regression. Constitution rule 11: an invariant that no type, test, lint or CI check covers is not protected. Several are new in IDSE and carry no legacy mechanism (`SEC-031`, `DUR-014`, `SEC-034`, `PRC-003`, `PRC-012`, `PRC-020`).

### G2 — Code cell empty, but a test or CI check exists (5)

`STR-003`, `STR-004`, `CON-004`, `DUR-003`, `PRC-001`. Each has a test or gate but names no enforcing locus.

### G3 — Tests cell empty (10, beyond G1)

`STR-003`, `STR-004`, `LIV-007`, `LIV-010`, `VAL-001`, `VAL-062`, `DET-006`, `PRC-001`, `PRC-005`, `PRC-009`. Some are structural and enforced by CI text-checks (for example `DET-006` no-semantic-cfg, `PRC-005` store-semantics lock); the rest are candidates for a named proof.

### G4 — CI cell empty (17, beyond G1)

`SEC-004`, `STR-010`, `STR-014`, `STR-018`, `SAF-010`, `SAF-016`, `SAF-017`, `SAF-023`, `LIV-007`, `VAL-046`, `VAL-047`, `ORD-003`, `DET-021`, `DUR-008`, `REC-007`, `PRC-007`, `PRC-013`. These rely on tests, types or code loci alone; `LIV-007` has neither tests nor CI.

### G5 — 37 CI gates red at `e563ad94`, affecting 46 invariants

Every one of the 37 is recorded in its rule's evidence limits as already failing before the IDSE migration (`a3cef7f7`) and diagnosed as a stale or over-broad structural gate, not a demonstrated violation. Until repaired, none can serve as evidence (Constitution rule 15 forbids weakening the gate to make it pass).

| Gate | Invariants naming it |
|---|---|
| `ci/ci_check_admission_runner_closure.sh` | `STR-020`, `SAF-024`, `DUR-013`, `OBS-007` |
| `ci/ci_check_consensus_input_provenance.sh` | `SEC-014`, `VAL-024` |
| `ci/ci_check_epoch_accumulator_recovery.sh` | `REC-002` |
| `ci/ci_check_eview_activation_predicate.sh` | `SAF-005`, `ORD-004` |
| `ci/ci_check_eview_automatic_activation.sh` | `EVO-001` |
| `ci/ci_check_eview_bootstrap_cert_state.sh` | `SEC-032` |
| `ci/ci_check_eview_epoch_rebind.sh` | `SAF-005`, `ORD-004` |
| `ci/ci_check_eview_forecast_crossing.sh` | `ORD-005` |
| `ci/ci_check_eview_leadership_complete.sh` | `SAF-009`, `VAL-020`, `VAL-023`, `VAL-024` |
| `ci/ci_check_eview_live_checkpoint.sh` | `REC-002` |
| `ci/ci_check_eview_pool_lifecycle.sh` | `VAL-038` |
| `ci/ci_check_eview_seed_sidecar_v4.sh` | `DUR-005` |
| `ci/ci_check_feed_leader_threshold_view.sh` | `STR-007`, `VAL-024` |
| `ci/ci_check_forge_purity.sh` | `VAL-056`, `VAL-058`, `DET-020` |
| `ci/ci_check_forged_durable_admit_via_pump.sh` | `STR-015`, `DUR-009` |
| `ci/ci_check_gov_proposal_capture.sh` | `CON-004` |
| `ci/ci_check_hash_uses_wire_bytes.sh` | `DET-002` |
| `ci/ci_check_keep_alive_wire_only.sh` | `VAL-061` |
| `ci/ci_check_lifecycle_owner_uses_bootstrap_initial_state.sh` | `DUR-004` |
| `ci/ci_check_live_fork_choice_apply.sh` | `STR-016`, `DUR-009`, `REC-004`, `REC-006` |
| `ci/ci_check_local_durable_forge_base.sh` | `VAL-053` |
| `ci/ci_check_mempool_ingress_closure.sh` | `STR-022` |
| `ci/ci_check_native_firstrun_no_cli_seed.sh` | `SEC-030` |
| `ci/ci_check_native_nonutxo_decode.sh` | `VAL-014` |
| `ci/ci_check_no_haskell_fingerprint_equality.sh` | `PRC-006` |
| `ci/ci_check_node_forge_single_epoch_fail_closed.sh` | `SAF-021` |
| `ci/ci_check_node_mode_closure.sh` | `STR-013` |
| `ci/ci_check_node_path_fidelity.sh` | `VAL-053`, `PRC-019` |
| `ci/ci_check_node_sync_via_pump.sh` | `DUR-012` |
| `ci/ci_check_participant_forge_on_selected_head.sh` | `SAF-022` |
| `ci/ci_check_post_switch_convergence_window.sh` | `DET-023` |
| `ci/ci_check_receive_detector_venue_split.sh` | `SAF-015`, `VAL-054` |
| `ci/ci_check_recover_follow_wal_lineage.sh` | `DUR-009` |
| `ci/ci_check_recovered_ledger_pparams_sourced.sh` | `VAL-024` |
| `ci/ci_check_rollback_target_canonical_binding.sh` | `SEC-020` |
| `ci/ci_check_warm_start_re_entry.sh` | `VAL-053` |
| `ci/ci_check_warmstart_eta0_overlay.sh` | `VAL-036`, `VAL-041` |

### G6 — Failing and unrun tests in the evidence run

- **2 tests FAILED:** `committee_oracle_mainnet_575_576_noop_agreement` (`STR-005`, `VAL-021`; limits record the test input as stale: it passes `DormantEpochs::Unversioned`, which the ratification gate now refuses) and `harness_and_blue_decoder_agree_on_conway_deposit_params` (`VAL-066`).
- **Coverage holes:** `ade_testkit` timed out after 2400 s in `epoch_oracle_comparison`, so later testkit binaries did not run; `ade_plutus`'s `conway_validity_range_reproducer` target does not compile without the git-ignored `target/aiken_divergent_fixture/`. Across 36 evidence records, 186 named tests were "not found" in the test output; the per-rule counts are in each Claim row.

### G7 — 22 stale test names (**†**) across 5 invariants

The registry names these test functions; no `fn` of that name exists in the tracked Rust sources at HEAD. They were carried from the legacy registry and the code has since been renamed.

| Invariant | Missing | Names |
|---|---:|---|
| `SEC-030` | 1 | `native_first_run_missing_shelley_genesis_is_terminal` |
| `STR-008` | 10 | `consensus::nonce::tests::header_contribution_advances_evolving_nonce_deterministically`, `consensus::nonce::tests::header_contribution_does_not_touch_epoch_nonce`, `consensus::nonce::tests::candidate_freeze_copies_evolving_to_candidate`, `consensus::nonce::tests::candidate_freeze_does_not_advance_slot`, `consensus::nonce::tests::epoch_boundary_promotes_candidate_to_epoch_nonce`, `consensus::nonce::tests::epoch_boundary_rotates_previous_epoch_nonce`, `consensus::nonce::tests::epoch_boundary_rejects_uninitialised_candidate`, `consensus::nonce::tests::epoch_boundary_preserves_op_cert_counters`, `consensus::nonce::tests::epoch_boundary_preserves_lab_nonce`, `consensus::nonce::tests::blake2b_input_order_is_evolving_then_vrf_output_first_32` |
| `VAL-020` | 1 | `delegated_zero_stake_pool_is_included_with_zero` |
| `VAL-022` | 9 | `bridge_equivalence_seeded_snapshot_tick_reproduces_eca5_eta0`, `header_contribution_advances_evolving_lab_candidate`, `candidate_freezes_at_freeze_boundary`, `epoch_tick_combines_candidate_with_last_epoch_block_nonce_no_reset`, `epoch_tick_rotates_last_epoch_block_nonce_from_lab`, `epoch_tick_fails_closed_on_absent_operand`, `chain_dep_array10_round_trip_some_and_none`, `chain_dep_always_writes_array10`, `b1_store_round_trip_reproduces_next_boundary_eta0` |
| `VAL-023` | 1 | `delegated_zero_stake_pool_is_included_with_zero` |

`delegated_zero_stake_pool_is_included_with_zero` (`VAL-020`, `VAL-023`) has a surviving `delegated_zero_stake_pool_is_omitted` in `crates/ade_ledger/src/reduced_aggregate.rs`: the rename inverts the asserted semantics, so it needs a decision, not a find-and-replace. `native_first_run_missing_shelley_genesis_is_terminal` (`SEC-030`) is also the test whose absence turns `ci/ci_check_native_firstrun_no_cli_seed.sh` red.

**11 test entries name a module or file, not a function** (they resolve, but no single test proves them):

| Invariant | Entries |
|---|---|
| `VAL-020` | `bootstrap_reward_update::tests`, `ade_ledger::rules::cert_state_dispatch::poolreap_ce3a` |
| `VAL-024` | `ade_ledger::frozen_leadership::tests`, `ade_runtime::chaindb::epoch_accumulator_store::tests` |
| `VAL-029` | `bootstrap_reward_update::tests` |
| `CON-004` | `ade_ledger::epoch_accumulator::tests` |
| `DET-023` | `crates/ade_node/tests/live_fork_choice_ai_s4bii.rs` |
| `DUR-003` | `ade_ledger::epoch_accumulator::tests`, `ade_node::node_lifecycle::ce4a_continuous_self_sufficiency PRODUCTION-LOOP EVIDENCE` |
| `REC-002` | `ade_runtime::chaindb::epoch_accumulator_advance::tests`, `ade_runtime::chaindb::epoch_accumulator_store::tests` |

### G8 — Known disagreements and expected falsifications recorded in evidence limits

These are the most important rows in the doc: each records a behaviour the invariant forbids that the current checks do not catch.

- `VAL-008`, `VAL-014`: no Conway minimum-UTxO check exists, so a transaction the reference rejects for an undersized output is not rejected by Ade.
- `VAL-010`, `SEC-011`: forge-time gaps (the operational-certificate period check compares a value with itself; the forge's own counter check gets no previous counter in node mode).
- `LIV-001`: open defect LV-17, recovery deletes a valid settled triple, so each rollback triggers a rebuild of about 10 hours; no check reproduces it.
- `LIV-004`: expected to be falsified; the node has been observed five times ignoring SIGINT/SIGTERM for 120 s or more. No check exercises shutdown.
- `VAL-057`: expected to be falsified; the forge emits blocks with an illegal header position that the decoder then rejects.
- `SEC-032`: expected to be falsified if node mode still bootstraps with an empty certificate state when the package files are absent.
- `VAL-060`, `PRC-023`: no live reference peer or public-venue acceptance evidence in the evidence environment.

---

## Cross-reference checks (vs CODEMAP / HEAD_DELTAS / registry)

All checks are static. They are warnings, not blocks.

### vs CODEMAP (`docs/ade-CODEMAP.md`, HEAD `08c7c3f5`) — PASS

- All 11 crates named in any Code row (`ade_codec`, `ade_core`, `ade_core_interop`, `ade_crypto`, `ade_ledger`, `ade_network`, `ade_node`, `ade_plutus`, `ade_runtime`, `ade_testkit`, `ade_types`) have CODEMAP entries. `ade_mem_diag`, the twelfth crate, is named by no invariant's locus; CODEMAP records the same.
- CODEMAP lists the same 25 invariants with no enforcement locus (this doc's G1 + G2) and the same 27 unbound gates. The registry snapshot numbers agree (260 invariants; families, criticality and claim counts identical).
- CODEMAP's unlocated files (`ade_ledger::{reduced_boundary, rollback::admission, cred, pre_resolve}`, `ade_node::{mithril_fetch, ops_log}`, `ade_mem_diag`) appear in no Code row here either.

### vs HEAD_DELTAS §5 (`docs/ade-HEAD_DELTAS.md`, `1e4896eb → 08c7c3f5`)

- All **31** gates added in the span are named by at least one invariant's `ci_scripts` (0 unbound).
- §5 is a delta, not a catalog: the pre-baseline gates named in CI rows are correctly absent from it.
- The 5 modified gates HEAD_DELTAS reports as unbound (`genesis_replay_open_obligation`, `registry_unique_ids`, `utxo_admission_seam`, `utxo_fp_cache`, `utxo_pre_resolve`) are in the orphan list below.
- The 3 gates HEAD_DELTAS reports as red for pre-existing code-level reasons (`warmstart_eta0_overlay`, `feed_leader_threshold_view`, `epoch_accumulator_recovery`) are among the 37 red gates in G5. The evidence run finds 34 more red gates than HEAD_DELTAS reports.

**27 of 286 gates are named by no invariant** (orphan enforcement; candidates for a binding by redefinition, or for retirement):

`ci/ci_check_alloc_determinism_neutral.sh`, `ci/ci_check_eview_activation.sh`, `ci/ci_check_eview_stability_gate.sh`, `ci/ci_check_genesis_replay_open_obligation.sh`, `ci/ci_check_live_blockfetch_byte_only.sh`, `ci/ci_check_live_transcript_forge_base.sh`, `ci/ci_check_mem_compare_evidence.sh`, `ci/ci_check_mem_diag_quarantine.sh`, `ci/ci_check_mem_opt_s1_reduction.sh`, `ci/ci_check_mem_opt_s2_import_peak.sh`, `ci/ci_check_mem_opt_s3_owned.sh`, `ci/ci_check_mem_opt_utxo_disk_s0.sh`, `ci/ci_check_module_headers.sh`, `ci/ci_check_no_secrets.sh`, `ci/ci_check_pallas_quarantine.sh`, `ci/ci_check_recovered_anchor_intersectable.sh`, `ci/ci_check_reduced_boundary_plane.sh`, `ci/ci_check_registry_unique_ids.sh`, `ci/ci_check_single_producer_extend_own_spine.sh`, `ci/ci_check_single_producer_loop_continuation.sh`, `ci/ci_check_trusted_replay_boundary.sh`, `ci/ci_check_utxo_admission_seam.sh`, `ci/ci_check_utxo_disk_anchor.sh`, `ci/ci_check_utxo_disk_key.sh`, `ci/ci_check_utxo_fp_cache.sh`, `ci/ci_check_utxo_pre_resolve.sh`, `ci/ci_check_wire_rollback_signal_preserved.sh`

- Workspace hygiene by design: `ci_check_module_headers.sh`, `ci_check_no_secrets.sh`, `ci_check_pallas_quarantine.sh`, `ci_check_mem_diag_quarantine.sh`, `ci_check_registry_unique_ids.sh`, `ci_check_genesis_replay_open_obligation.sh`.
- Memory-optimisation and UTxO-disk evidence gates: the `mem_*`, `mem_opt_*`, `utxo_*` and `alloc_determinism_neutral` gates.
- Gates with a concrete invariant shape and no binding: `reduced_boundary_plane`, `trusted_replay_boundary`, `recovered_anchor_intersectable`, `single_producer_loop_continuation`, `single_producer_extend_own_spine`, `wire_rollback_signal_preserved`, `live_blockfetch_byte_only`, `live_transcript_forge_base`, `eview_activation`, `eview_stability_gate`.

### vs the registry

- Every rule in this doc is an active registry entry, and every active entry is here (260/260). `registry_count = 260` matches.
- Every `depends_on` ID and every cited assumption resolves.
- Every Code path and every CI script the registry names exists at HEAD (0 stale). 22 test names are stale (G7).
- **Approval-record anchors.** Every Source cites `<file>#batch-1`, but each consensus record's heading is `## Batch 1 — <date> — after round N @ blob <sha>`, so the `#batch-1` fragment does not resolve as a rendered Markdown anchor. The heading exists in all 12 files.

---

## Open questions for the user

1. **G1:** the 20 unprotected invariants falsify PRC-001. Which get a mechanism first, and through which cluster? (Six of them are new in IDSE and had no legacy enforcement.)
2. **G5:** the 37 red gates are diagnosed as stale structural gates. Repair them in a dedicated cluster before any claim that relies on them is promoted? Rule 15 rules out loosening them.
3. **G7:** refresh the 22 stale test names, by redefinition where the registry field is part of the definition. `VAL-020`/`VAL-023`'s `delegated_zero_stake_pool_is_included_with_zero` → `..._is_omitted` inverts the asserted semantics and needs a decision.
4. **Orphan gates:** bind the 10 gates with a concrete invariant shape (for example `reduced_boundary_plane`, `trusted_replay_boundary`, `recovered_anchor_intersectable`) to existing invariants, define new invariants for them, or retire them?
5. **Level-3 grouping:** this doc groups by Define batch because `scope` is near-unique per entry. Keep that, or should `scope` be normalised to a short closed vocabulary so it can serve as the grouping key? Note that the atlas parser reads these headings.
6. **Source anchors:** should the registry's `#batch-1` records point at a resolvable anchor (or should the consensus records add an explicit `batch-1` anchor)?

---

## Generation notes

Regenerated at HEAD `08c7c3f5` by parsing `docs/ade-invariant-registry.toml` with `tomllib` and joining it against static checks: `git grep` indexes of `fn` and `mod` names over the 721 tracked `.rs` files, path existence for every locus and gate, `git diff --name-only e563ad94 HEAD` plus the working tree for evidence freshness, and the per-test and per-gate outcomes in `docs/evidence/idse-m2-reevidence.json`. No test, CI script or replay was run. **No rule was invented and no Code, Tests or CI cell was guessed;** empty cells are marked as gaps.

Legacy citations: the doc cites only IDSE IDs. Legacy IDs appear only in the closing `Legacy:` note of each rule. Three evidence limits quote CI output that names a legacy ID; in the Claim rows those tokens read `[legacy ref]`, and the full text is in the registry. Per-locus annotations are omitted from Code rows for the same reason.

Regenerate; do not hand-edit. If a value drifts, fix the registry (the source), not this doc.
