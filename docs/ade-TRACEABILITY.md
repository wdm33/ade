# Normative Rule Traceability — Ade

> **Status:** Living architectural document. Regenerated; not hand-edited.
> Per-project instance of `~/.claude/methodology/templates/traceability.md`.

> **Baseline:** `470f9b89` (MEM-OPT-UTXO-DISK close). **HEAD:** `cdcd9397` (mid-flight refresh — two clusters in flight above the baseline: **EPOCH-CONTINUITY-ACTIVATION (ECA)** and **native Mithril bootstrap (MITHRIL-VERIFIED-ANCHOR-INTEGRATION)**, plus the standalone `DC-LEDGER-VALUE-01` / `DC-LEDGER-PARAMS-01` ledger-type slices; no cluster has CLOSED since baseline).

This document is the **invariant ↔ enforcement audit** IDD §10 demands. For every rule the project commits to, it traces: where the rule is *specified* (Source), what must hold (Requirement), where it is *enforced in code* (Code), which *tests prove* it (Tests), and which *CI check(s)* fail the build on violation (CI). A rule that cannot fill all four load-bearing cells is an **enforcement gap** — surfaced here, never hidden.

## Source of rules

The canonical rule source is the **invariant registry** `docs/ade-invariant-registry.toml` (declared in `.idd-config.json` `invariant_registry`). This doc is a join: registry entries × codebase introspection. Rule IDs, families, the Requirement (from the registry `statement`, or paraphrased from `source`/`code_locus` where `statement` is empty), Source (`source`), Code (`code_locus`), Tests (`tests`), and CI (`ci_script` / `ci_scripts`) all come from the registry; **existence of every named test fn and CI script was verified against the codebase at HEAD** (static existence checks — the project `replay_cmd` was NOT executed, as `ade_testkit::epoch_boundary_logic` hangs on a known pre-existing test, commit `7c769801`).

## Rule inventory (mechanical, at HEAD)

| Status | Count |
|--------|------:|
| enforced | 291 |
| partial | 22 |
| declared | 104 |
| enforced_scaffolding | 1 |
| **Total** | **418** |

Source: `grep -c '^status = '` over the registry at `cdcd9397` = **418** rule IDs = 418 status lines (each `[[rules]]` block carries exactly one `id` and one `status`). 0 deprecated.

| Family | Rules | enforced | partial | declared | enf-scaffold |
|--------|------:|---------:|--------:|---------:|-------------:|
| T | 33 | 15 | 3 | 15 | 0 |
| CN | 120 | 59 | 5 | 56 | 0 |
| DC | 239 | 209 | 10 | 19 | 1 |
| OP | 10 | 3 | 1 | 6 | 0 |
| RO | 16 | 5 | 3 | 8 | 0 |
| **All** | **418** | **291** | **22** | **104** | **1** |

Within each family below, rules are grouped by sub-family (the `XX-YYYY` ID prefix) and ordered by stable ID. IDs are append-only and never reused; the ordering does not shuffle when new rules are added.

---

## T — True Invariants (Project Constitution §2)

_33 rules._

### T-DET

#### `T-DET-01` — _enforced_

| Aspect | Location |
|--------|----------|
| **Source** | Project constitution §2, Byte Authority Model §3 |
| **Requirement** | Same canonical inputs -> same authoritative bytes (per Byte Authority Model) |
| **Code** | crates/ade_ledger/src/rules.rs, crates/ade_core/src/consensus/encoding.rs, crates/ade_runtime/src/consensus/chain_selector.rs, crates/ade_testkit/src/consensus/stream_replay.rs, crates/ade_testkit/src/validity/replay.rs (block_validity verdict-surface replay over the Conway-576 positive corpus; PHASE4-B1-S6), crates/ade_testkit/src/tx_validity/ (tx-validity verdict-surface replay: replay_tx_validity drives the BLUE tx_validity over every extracted Conway-576 corpus tx twice and the surfaces are byte-identical; PHASE4-B2-S3) |
| **Tests** | `apply_block_deterministic`; `byron_determinism`; `shelley_determinism`; `allegra_determinism`; `mary_determinism`; `alonzo_determinism`; `babbage_determinism`; `conway_determinism`; `layout_is_stable`; `roundtrip_empty_state`; … (+11 more) |
| **CI** | `ci/ci_check_forbidden_patterns.sh`; `ci/ci_check_ledger_determinism.sh`; `ci/ci_check_consensus_closed_enums.sh` |

### T-ENC

#### `T-ENC-01` — _partial_

| Aspect | Location |
|--------|----------|
| **Source** | Project constitution §2 |
| **Requirement** | All persisted/hashed/transmitted data uses canonical encoding |
| **Code** | crates/ade_codec/src/preserved.rs, crates/ade_ledger/src/block_validity/header_input.rs (block body hash computed over preserved CBOR segment bytes, never re-encoded; PHASE4-B1-S4); crates/ade_ledger/src/tx_validity/phase1.rs (decode_tx: tx_id = blake2b_256 of the body slice lifted byte-for-byte from the full tx CBOR, never a re-encode; PHASE4-B2-S2) |
| **Tests** | `preserved_wire_bytes_returned_exactly`; `altered_body_rejected_by_hash_binding`; `tx_id_uses_preserved_bytes` |
| **CI** | `ci/ci_check_hash_uses_wire_bytes.sh` |

#### `T-ENC-02` — _declared_

| Aspect | Location |
|--------|----------|
| **Source** | Project constitution §2 |
| **Requirement** | Non-canonical bytes rejected deterministically |
| **Code** | _(no enforcement found — gap)_ |
| **Tests** | _(no tests listed — gap)_ |
| **CI** | _(no CI gate — gap)_ |

#### `T-ENC-03` — _enforced_

| Aspect | Location |
|--------|----------|
| **Source** | Project constitution §2, D-3 |
| **Requirement** | Round-trip identity: encode(decode(bytes)) == bytes for valid encodings |
| **Code** | crates/ade_codec/src/preserved.rs, crates/ade_network/src/codec/handshake.rs, crates/ade_network/src/codec/n2c_handshake.rs, crates/ade_network/src/codec/chain_sync.rs, crates/ade_network/src/codec/block_fetch.rs, crates/ade_network/src/codec/tx_submission.rs, crates/ade_network/src/codec/keep_alive.rs, crates/ade_network/src/codec/peer_sharing.rs, crates/ade_network/src/codec/local_chain_sync.rs, crates/ade_network/src/codec/local_tx_submission.rs, crates/ade_network/src/codec/local_state_query.rs, crates/ade_network/src/codec/local_tx_monitor.rs |
| **Tests** | `all_42_blocks_round_trip_byte_identical`; `all_42_blocks_fields_match_reference`; `roundtrip_every_variant`; `roundtrip_every_variant`; `roundtrip_every_variant`; `roundtrip_every_variant`; `roundtrip_every_variant`; `roundtrip_every_variant`; `roundtrip_every_variant`; `roundtrip_every_variant`; … (+9 more) |
| **CI** | `ci/ci_check_cbor_round_trip.sh` |

### T-CORE

#### `T-CORE-01` — _enforced_

| Aspect | Location |
|--------|----------|
| **Source** | Project constitution §2 |
| **Requirement** | Authoritative logic is pure, side-effect-free, and replayable |
| **Code** | crates/ade_ledger/src/rules.rs |
| **Tests** | `apply_block_deterministic`; `apply_block_byron_ebb_passes_through` |
| **CI** | `ci/ci_check_forbidden_patterns.sh`; `ci/ci_check_dependency_boundary.sh` |

#### `T-CORE-02` — _enforced_

| Aspect | Location |
|--------|----------|
| **Source** | Project constitution §2, D-2 through D-5 |
| **Requirement** | No wall-clock, unseeded randomness, floats, or nondeterministic collections in authoritative paths |
| **Code** | crates/ade_ledger/src/lib.rs, crates/ade_core/src/consensus/vrf_cert.rs |
| **Tests** | `taylor_exp_cmp_le_zero_x_returns_false`; `taylor_exp_cmp_le_x_equals_one_returns_true`; `taylor_exp_cmp_le_monotone_in_x`; `is_leader_determinism`; `is_leader_known_vector_matches_reference` |
| **CI** | `ci/ci_check_forbidden_patterns.sh`; `ci/ci_check_no_float_in_consensus.sh` |

#### `T-CORE-03` — _declared_

| Aspect | Location |
|--------|----------|
| **Source** | Project constitution §2, §5.2 |
| **Requirement** | Explicit state transitions: consume old state, produce new state |
| **Code** | _(no enforcement found — gap)_ |
| **Tests** | _(no tests listed — gap)_ |
| **CI** | _(no CI gate — gap)_ |

#### `T-CORE-04` — _declared_

| Aspect | Location |
|--------|----------|
| **Source** | Project constitution §2, §5.1 |
| **Requirement** | Illegal states unrepresentable via types where practical |
| **Code** | _(no enforcement found — gap)_ |
| **Tests** | _(no tests listed — gap)_ |
| **CI** | _(no CI gate — gap)_ |

### T-ERR

#### `T-ERR-01` — _partial_

| Aspect | Location |
|--------|----------|
| **Source** | Project constitution §2 |
| **Requirement** | Errors in authoritative paths are structured, comparable, canonical |
| **Code** | crates/ade_ledger/src/error.rs |
| **Tests** | `ledger_error_equality`; `conservation_error_display`; `codec_error_conversion` |
| **CI** | _(no CI gate — gap)_ |

#### `T-ERR-02` — _declared_

| Aspect | Location |
|--------|----------|
| **Source** | Project constitution §2, E-1 |
| **Requirement** | Safety violations fail-fast deterministically |
| **Code** | _(no enforcement found — gap)_ |
| **Tests** | _(no tests listed — gap)_ |
| **CI** | _(no CI gate — gap)_ |

### T-BUILD

#### `T-BUILD-01` — _enforced_

| Aspect | Location |
|--------|----------|
| **Source** | Project constitution §2 |
| **Requirement** | No semantic build variability in authoritative code |
| **Code** | crates/ade_ledger/src/lib.rs |
| **Tests** | _(no tests listed in registry — enforced via Code/CI; gap: name the proving test)_ |
| **CI** | `ci/ci_check_no_semantic_cfg.sh` |

#### `T-BUILD-02` — _declared_

| Aspect | Location |
|--------|----------|
| **Source** | Project constitution §2 |
| **Requirement** | One semantic interpretation per protocol version and input set |
| **Code** | _(no enforcement found — gap)_ |
| **Tests** | _(no tests listed — gap)_ |
| **CI** | _(no CI gate — gap)_ |

### T-REC

#### `T-REC-01` — _enforced_

| Aspect | Location |
|--------|----------|
| **Source** | Project constitution §2 |
| **Requirement** | Recovery is replay-equivalent: restart produces byte-identical state to clean run |
| **Code** | crates/ade_runtime/src/recovery/mod.rs, crates/ade_runtime/src/recovery/restart.rs, crates/ade_runtime/src/chaindb/crash_safety.rs |
| **Tests** | `recover_from_snapshot_and_replay_forward`; `recover_from_genesis_when_no_snapshot`; `apply_failure_surfaces_with_slot`; `snapshot_decode_failure_surfaces_as_error`; `snapshot_with_no_post_blocks_is_ok`; `stress_kill_smoke`; `stress_kill_1000`; `snapshot_table_intact_after_kill_loop`; `persistent_passes_crash_safety_with_no_kill` |
| **CI** | `ci/ci_check_recovery_contract.sh`; `ci/ci_check_chaindb_crash_safety.sh` |

#### `T-REC-02` — _enforced_

| Aspect | Location |
|--------|----------|
| **Source** | Project constitution §2 |
| **Requirement** | All authoritative state derivable by replay from inputs |
| **Code** | crates/ade_runtime/src/recovery/mod.rs, crates/ade_runtime/src/recovery/restart.rs |
| **Tests** | `recover_from_snapshot_and_replay_forward`; `recover_from_genesis_when_no_snapshot` |
| **CI** | `ci/ci_check_recovery_contract.sh` |

#### `T-REC-03` — _enforced_

| Aspect | Location |
|--------|----------|
| **Source** | docs/planning/phase4-n-f-d-live-node-run-loop-invariants.md |
| **Requirement** | docs/planning/phase4-n-f-d-live-node-run-loop-invariants.md *(paraphrased from source/code_locus; registry `statement` empty)* |
| **Code** | crates/ade_node/src/node_lifecycle.rs (run_relay_loop driver), crates/ade_node/src/node_sync.rs (relay_loop_two_clean_runs_byte_identical evidence test) |
| **Tests** | `relay_loop_two_clean_runs_byte_identical` |
| **CI** | `ci/ci_check_node_run_loop_containment.sh` |

#### `T-REC-04` — _enforced_

| Aspect | Location |
|--------|----------|
| **Source** | docs/clusters/PHASE4-N-F-G-N/cluster.md |
| **Requirement** | docs/clusters/PHASE4-N-F-G-N/cluster.md *(paraphrased from source/code_locus; registry `statement` empty)* |
| **Code** | crates/ade_ledger/src/seed_consensus_inputs.rs (SeedEpochConsensusInputs.epoch_nonce field + versioned codec, SEED_CINPUT_SCHEMA_VERSION=2 fail-closed on the schema change); crates/ade_runtime/src/seed_consensus_merge.rs (merge persists canonical.epoch_nonce); crates/ade_runtime/src/bootstrap.rs (bootstrap_initial_state overlays the recovered sidecar epoch_nonce onto chain_dep.epoch_nonce + evolving_nonce) |
| **Tests** | `warm_start_overlays_recovered_eta0_onto_chain_dep_g_n`; `warm_start_restores_seed_epoch_consensus_inputs_byte_identical`; `seed_cinput_decode_rejects_unknown_version`; `seed_epoch_consensus_inputs_round_trips_byte_identical`; `pinning_preseed_warmstart_roundtrip_faithful` |
| **CI** | `ci/ci_check_warmstart_eta0_overlay.sh` |

#### `T-REC-05` — _enforced_

| Aspect | Location |
|--------|----------|
| **Source** | docs/planning/phase4-n-u-forged-block-durability-invariants.md |
| **Requirement** | docs/planning/phase4-n-u-forged-block-durability-invariants.md *(paraphrased from source/code_locus; registry `statement` empty)* |
| **Code** | crates/ade_node/src/node_lifecycle.rs (warm_start_recovery: sidecar-reconstructed era_schedule/ledger_view + forward-replay via bootstrap_initial_state + fingerprint fail-fast guard); crates/ade_node/src/node_sync.rs (admit_forged_block_durably -> pump_block); crates/ade_runtime/src/bootstrap.rs (bootstrap_initial_state warm-start forward-replay -- reused) |
| **Tests** | `forge_kill_then_warm_start_recovers_same_tip_via_forward_replay`; `forge_tip_successor_kill_then_warm_start_recovers_block_one`; `recover_follow_forge_two_runs_byte_identical`; `recover_follow_kill_warm_start_chains_from_ledger_fp`; `recover_follow_two_runs_byte_identical`; `same_store_same_anchor_point_same_findintersect_start` |
| **CI** | _(no CI listed in registry — enforced via Code/Tests; gap: bind a CI gate)_ |

#### `T-REC-06` — _enforced_

| Aspect | Location |
|--------|----------|
| **Source** | docs/planning/phase4-n-an-rollback-materialize-eta0-invariants.md + docs/clusters/PHASE4-N-AN/cluster.md |
| **Requirement** | docs/planning/phase4-n-an-rollback-materialize-eta0-invariants.md + docs/clusters/PHASE4-N-AN/cluster.md *(paraphrased from source/code_locus; registry `statement` empty)* |
| **Code** | crates/ade_ledger/src/rollback/materialize.rs (materialize_rolled_back_state -- the SOLE rolled-back-state authority; AN-S2 overlays the recovered eta0 onto the nearest_le snapshot chain_dep before the replay-forward block_validity fold), crates/ade_ledger/src/seed_consensus_inputs.rs (SeedEpochConsensusInputs.epoch_nonce -- the persisted recovered eta0 carrier), crates/ade_node/src/node_lifecycle.rs (apply_chain_event threads state.seed_epoch_consensus_inputs eta0 into the materialize call) |
| **Tests** | `rollback_materialize_overlays_recovered_eta0_replay_equivalent`; `rollback_materialize_does_not_bypass_vrf_on_wrong_eta0` |
| **CI** | `ci/ci_check_rollback_materialize_eta0.sh` |

### T-COLL

#### `T-COLL-01` — _declared_

| Aspect | Location |
|--------|----------|
| **Source** | Project constitution §2, D-5 |
| **Requirement** | Deterministic iteration order for all semantically meaningful collections |
| **Code** | _(no enforcement found — gap)_ |
| **Tests** | _(no tests listed — gap)_ |
| **CI** | _(no CI gate — gap)_ |

### T-BOUND

#### `T-BOUND-01` — _declared_

| Aspect | Location |
|--------|----------|
| **Source** | Project constitution §2 |
| **Requirement** | Shell may observe nondeterminism but must convert to deterministic inputs before entering core |
| **Code** | _(no enforcement found — gap)_ |
| **Tests** | _(no tests listed — gap)_ |
| **CI** | _(no CI gate — gap)_ |

#### `T-BOUND-02` — _enforced_

| Aspect | Location |
|--------|----------|
| **Source** | Project constitution §2, GP-4 |
| **Requirement** | Authoritative crates never depend on shell crates |
| **Code** | crates/*/Cargo.toml |
| **Tests** | _(no tests listed in registry — enforced via Code/CI; gap: name the proving test)_ |
| **CI** | `ci/ci_check_dependency_boundary.sh` |

### T-INGRESS

#### `T-INGRESS-01` — _declared_

| Aspect | Location |
|--------|----------|
| **Source** | Project constitution §2, audit #21 |
| **Requirement** | All authoritative external bytes enter the core through named canonical decode/validation chokepoints; unchecked bypasses forbidden except for explicitly whitelisted sites with CI enforcement |
| **Code** | _(no enforcement found — gap)_ |
| **Tests** | _(no tests listed — gap)_ |
| **CI** | `ci/ci_check_ingress_chokepoints.sh` |

### T-CI

#### `T-CI-01` — _partial_

| Aspect | Location |
|--------|----------|
| **Source** | Project constitution §2, CI-1 |
| **Requirement** | Every true invariant has mechanical CI enforcement. No waivers. |
| **Code** | ci/ |
| **Tests** | _(no tests listed — gap)_ |
| **CI** | `ci/ci_check_registry_code_locus_exists.sh` |

### T-KEY

#### `T-KEY-01` — _declared_

| Aspect | Location |
|--------|----------|
| **Source** | Project constitution §2, audit #20 |
| **Requirement** | Signing and private key operations confined to shell; verification in core |
| **Code** | _(no enforcement found — gap)_ |
| **Tests** | _(no tests listed — gap)_ |
| **CI** | _(no CI gate — gap)_ |

### T-CONS

#### `T-CONS-01` — _enforced_

| Aspect | Location |
|--------|----------|
| **Source** | Project constitution §2, 01_core §12 |
| **Requirement** | Chain selection depends only on canonical observables; same candidates -> same tip |
| **Code** | crates/ade_core/src/consensus/fork_choice.rs, crates/ade_core/src/consensus/candidate.rs |
| **Tests** | `replay_is_deterministic`; `reject_reason_bytes_are_stable`; `higher_block_no_wins`; `equal_block_no_tiebreaker_decides`; `select_best_chain_arrival_order_independent_distinct_heights`; `select_best_chain_arrival_order_independent_tiebreaker` |
| **CI** | `ci/ci_check_no_density_in_fork_choice.sh`; `ci/ci_check_chain_selection_arrival_order_independent.sh` |

#### `T-CONS-02` — _declared_

| Aspect | Location |
|--------|----------|
| **Source** | Project constitution §2, 01_core §3 D-2, audit #9 |
| **Requirement** | Authoritative consensus decisions must not depend on wall-clock, arrival-order, scheduler, or OS behavior |
| **Code** | _(no enforcement found — gap)_ |
| **Tests** | _(no tests listed — gap)_ |
| **CI** | _(no CI gate — gap)_ |

### T-CONSERV

#### `T-CONSERV-01` — _enforced_

| Aspect | Location |
|--------|----------|
| **Source** | Project constitution §2, audit #13 |
| **Requirement** | UTxO and asset conservation must hold for every accepted transition, except where protocol rules explicitly authorize mint, burn, rewards, or treasury effects |
| **Code** | crates/ade_ledger/src/value.rs, crates/ade_ledger/src/byron.rs |
| **Tests** | `all_eras_replay_summary`; `byron_replay_all_1500`; `shelley_replay_all_1500`; `conway_conservation_full` |
| **CI** | `ci/ci_check_differential_divergence.sh` |

### T-NOSPEND

#### `T-NOSPEND-01` — _enforced_

| Aspect | Location |
|--------|----------|
| **Source** | Project constitution §2, audit #14 |
| **Requirement** | No input or equivalent spend authority may be consumed more than once in an accepted canonical chain |
| **Code** | crates/ade_ledger/src/utxo.rs |
| **Tests** | `check_duplicate_inputs_catches_dupes`; `duplicate_inputs_detected`; `all_eras_replay_summary` |
| **CI** | `ci/ci_check_differential_divergence.sh` |

### T-EPOCH

#### `T-EPOCH-01` — _declared_

| Aspect | Location |
|--------|----------|
| **Source** | Project constitution §2, 01_core §13, audit #18 |
| **Requirement** | Exactly one authoritative committee and governance interpretation per epoch |
| **Code** | _(no enforcement found — gap)_ |
| **Tests** | _(no tests listed — gap)_ |
| **CI** | _(no CI gate — gap)_ |

### T-CAUSAL

#### `T-CAUSAL-01` — _declared_

| Aspect | Location |
|--------|----------|
| **Source** | Project constitution §2, audit #17 |
| **Requirement** | Future decisions may not leak into present validation; no retroactive reinterpretation of prior checkpoints |
| **Code** | _(no enforcement found — gap)_ |
| **Tests** | _(no tests listed — gap)_ |
| **CI** | _(no CI gate — gap)_ |

### T-TRANSPORT

#### `T-TRANSPORT-01` — _declared_

| Aspect | Location |
|--------|----------|
| **Source** | Project constitution §2, audit #4 |
| **Requirement** | Transport nondeterminism (socket fragmentation, mux ordering, timeouts) must not leak into authoritative logic |
| **Code** | _(no enforcement found — gap)_ |
| **Tests** | _(no tests listed — gap)_ |
| **CI** | _(no CI gate — gap)_ |

### T-RESOURCE

#### `T-RESOURCE-01` — _declared_

| Aspect | Location |
|--------|----------|
| **Source** | Project constitution §2, audit #5 |
| **Requirement** | Untrusted inputs must not allocate unbounded authoritative resources before deterministic validation |
| **Code** | _(no enforcement found — gap)_ |
| **Tests** | _(no tests listed — gap)_ |
| **CI** | _(no CI gate — gap)_ |

### T-PLATFORM

#### `T-PLATFORM-01` — _declared_

| Aspect | Location |
|--------|----------|
| **Source** | Project constitution §2, audit #16 |
| **Requirement** | No host-environment property (locale, timezone, architecture, platform) may influence authoritative computation results |
| **Code** | _(no enforcement found — gap)_ |
| **Tests** | _(no tests listed — gap)_ |
| **CI** | _(no CI gate — gap)_ |

---

## CN — Classification-Table Invariants (constraint network)

_120 rules._

### CN-WIRE

#### `CN-WIRE-01` — _declared_

| Aspect | Location |
|--------|----------|
| **Source** | classification_table.md §A |
| **Requirement** | Hash-critical original bytes must be preserved and used on all hash/signature-critical paths |
| **Code** | _(no enforcement found — gap)_ |
| **Tests** | _(no tests listed — gap)_ |
| **CI** | _(no CI gate — gap)_ |

#### `CN-WIRE-02` — _declared_

| Aspect | Location |
|--------|----------|
| **Source** | classification_table.md §A |
| **Requirement** | Internal replay/state surfaces must use exactly one canonical project encoding |
| **Code** | _(no enforcement found — gap)_ |
| **Tests** | _(no tests listed — gap)_ |
| **CI** | _(no CI gate — gap)_ |

#### `CN-WIRE-03` — _declared_

| Aspect | Location |
|--------|----------|
| **Source** | classification_table.md §A |
| **Requirement** | Consensus-critical deserialization must be equivalent across all supported versions and active code paths |
| **Code** | _(no enforcement found — gap)_ |
| **Tests** | _(no tests listed — gap)_ |
| **CI** | _(no CI gate — gap)_ |

#### `CN-WIRE-04` — _declared_

| Aspect | Location |
|--------|----------|
| **Source** | classification_table.md §A |
| **Requirement** | Malformed consensus-relevant inputs must be rejected deterministically before any authoritative state transition |
| **Code** | _(no enforcement found — gap)_ |
| **Tests** | _(no tests listed — gap)_ |
| **CI** | _(no CI gate — gap)_ |

#### `CN-WIRE-05` — _declared_

| Aspect | Location |
|--------|----------|
| **Source** | classification_table.md §A |
| **Requirement** | No legacy or compatibility parser may accept bytes that the canonical parser rejects |
| **Code** | _(no enforcement found — gap)_ |
| **Tests** | _(no tests listed — gap)_ |
| **CI** | _(no CI gate — gap)_ |

#### `CN-WIRE-06` — _declared_

| Aspect | Location |
|--------|----------|
| **Source** | classification_table.md §A |
| **Requirement** | Every network/storage ingress path must pass through named era-aware decode chokepoints |
| **Code** | _(no enforcement found — gap)_ |
| **Tests** | _(no tests listed — gap)_ |
| **CI** | _(no CI gate — gap)_ |

#### `CN-WIRE-07` — _enforced_

| Aspect | Location |
|--------|----------|
| **Source** | classification_table.md §A |
| **Requirement** | Each protocol-visible message must decode into one closed, versioned message type |
| **Code** | crates/ade_network/src/codec/error.rs, crates/ade_network/src/codec/version.rs, crates/ade_network/src/codec/handshake.rs, crates/ade_network/src/codec/n2c_handshake.rs, crates/ade_network/src/codec/chain_sync.rs, crates/ade_network/src/codec/block_fetch.rs, crates/ade_network/src/codec/tx_submission.rs, crates/ade_network/src/codec/keep_alive.rs, crates/ade_network/src/codec/peer_sharing.rs, crates/ade_network/src/codec/local_chain_sync.rs, crates/ade_network/src/codec/local_tx_submission.rs, crates/ade_network/src/codec/local_state_query.rs, crates/ade_network/src/codec/local_tx_monitor.rs |
| **Tests** | `roundtrip_every_variant`; `decode_rejects_unknown_tag`; `decode_rejects_truncated_input`; `decode_rejects_invalid_utf8_in_text_fields`; `roundtrip_every_variant`; `decode_rejects_unknown_tag`; `decode_rejects_truncated_input`; `decode_rejects_invalid_utf8_in_text_fields`; `roundtrip_every_variant`; `decode_rejects_unknown_tag`; … (+35 more) |
| **CI** | `ci/ci_check_codec_message_closed.sh` |

#### `CN-WIRE-08` — _enforced_

| Aspect | Location |
|--------|----------|
| **Source** | docs/planning/phase4-n-x-invariants.md |
| **Requirement** | docs/planning/phase4-n-x-invariants.md *(paraphrased from source/code_locus; registry `statement` empty)* |
| **Code** | crates/ade_codec/src/cbor/tag24.rs (wrap_tag24/unwrap_tag24 + TagEnvelopeError); crates/ade_codec/src/cbor/mod.rs (read_bytes/read_text/skip_item length-arg overflow guard — fail-closed, no panic); crates/ade_network/src/codec/block_fetch.rs + chain_sync.rs (per-protocol compose/decompose); crates/ade_network/src/block_fetch/server.rs + chain_sync/server.rs (serve emits composed bytes); crates/ade_node/src/admission/runner.rs + crates/ade_core_interop/src/follow.rs (RED unwraps migrated onto the shared authority) |
| **Tests** | `wrap_then_unwrap_is_identity_across_length_classes`; `wrap_emits_canonical_tag24_marker_and_length`; `unwrap_returns_zero_copy_borrow_of_input`; `unwrap_rejects_missing_tag24_marker`; `unwrap_rejects_non_byte_string_payload`; `unwrap_rejects_truncated_inner`; `unwrap_rejects_trailing_bytes`; `inner_bytes_are_verbatim_not_reencoded`; `unwrap_rejects_huge_declared_length_without_panic`; `read_bytes_rejects_overflowing_declared_length`; … (+14 more) |
| **CI** | `ci/ci_check_tag24_wire_authority.sh` |

#### `CN-WIRE-09` — _enforced_

| Aspect | Location |
|--------|----------|
| **Source** | docs/planning/phase4-n-f-g-j-genesis-successor-prevhash-invariants.md |
| **Requirement** | docs/planning/phase4-n-f-g-j-genesis-successor-prevhash-invariants.md *(paraphrased from source/code_locus; registry `statement` empty)* |
| **Code** | crates/ade_types/src/shelley/block.rs (PrevHash = Genesis \| Block(Hash32) + ShelleyHeaderBody.prev_hash + block_hash() accessor); crates/ade_codec/src/shelley/block.rs (decode_prev_hash + the ShelleyHeaderBody AdeEncode null/hash32 match -- POSITION-BLIND); crates/ade_ledger/src/block_validity/header_position.rs (check_header_position -- the single POSITION-AWARE authority, block_number 0 <=> Genesis; S3); crates/ade_ledger/src/block_validity/header_input.rs (decode_block calls check_header_position before the header authority; S3); crates/ade_ledger/src/block_validity/verdict.rs (BlockValidityError::HeaderPositionInvalid -> existing HeaderInvalid class; S3);… |
| **Tests** | `prevhash_genesis_round_trips_as_null`; `prevhash_block_round_trips_as_hash32`; `prevhash_codec_is_position_blind`; `genesis_successor_header_round_trips_with_null_prev`; `block_header_prev_hash_byte_identical_after_migration`; `header_position_zero_requires_genesis_ok`; `header_position_zero_with_block_is_rejected`; `header_position_nonzero_requires_block_ok`; `header_position_nonzero_with_genesis_is_rejected`; `decode_block_rejects_block_prev_at_block_number_zero`; … (+10 more) |
| **CI** | `ci/ci_check_prevhash_single_wire_authority.sh` |

#### `CN-WIRE-10` — _enforced_

| Aspect | Location |
|--------|----------|
| **Source** | docs/clusters/PHASE4-N-F-G-L/cluster.md |
| **Requirement** | docs/clusters/PHASE4-N-F-G-L/cluster.md *(paraphrased from source/code_locus; registry `statement` empty)* |
| **Code** | crates/ade_network/src/handshake/version_table.rs (encode_n2n_version_params -- the SINGLE per-version N2N versionData wire authority); crates/ade_network/src/session/handshake_driver.rs (the serve responder builds AcceptVersion via encode_n2n_version_params(version.get(), params.network_magic), NOT a placeholder); crates/ade_node/src/admission/bootstrap.rs (build_n2n_version_table -- the initiator uses the SAME authority) |
| **Tests** | `responder_v15_accept_matches_real_cardano_node_preprod_fixture`; `responder_v15_accept_matches_failing_c1_peer_fixture`; `responder_v15_versiondata_is_a_four_element_array_not_a_bare_int`; `served_view_projects_durable_chain` |
| **CI** | `ci/ci_check_n2n_handshake_versiondata_authority.sh` |

#### `CN-WIRE-11` — _enforced_

| Aspect | Location |
|--------|----------|
| **Source** | docs/clusters/PHASE4-N-F-G-M/cluster.md |
| **Requirement** | docs/clusters/PHASE4-N-F-G-M/cluster.md *(paraphrased from source/code_locus; registry `statement` empty)* |
| **Code** | crates/ade_network/src/codec/primitives.rs (decode_array_head_two_form + try_consume_break -- the SCOPED two-form array head; decode_array_header stays definite-only); crates/ade_network/src/codec/chain_sync.rs (decode_find_intersect_points -- accepts the indefinite points list for MsgFindIntersect ONLY); crates/ade_network/src/chain_sync/server.rs (producer_chain_sync_serve FindIntersect arm -- Origin -> IntersectFound[Origin], block points via the existing closed intersect) |
| **Tests** | `real_cardano_node_findintersect_indefinite_points_list_decodes`; `real_cardano_node_findintersect_yields_intersect_found_origin`; `producer_chain_sync_serve_find_intersect_origin_yields_intersect_found_origin`; `producer_chain_sync_serve_find_intersect_known_point_yields_intersect_found`; `producer_chain_sync_serve_find_intersect_unknown_point_yields_intersect_not_found`; `decode_array_header_still_rejects_indefinite`; `two_form_accepts_definite_and_indefinite` |
| **CI** | `ci/ci_check_chainsync_findintersect_compat.sh` |

#### `CN-WIRE-12` — _enforced_

| Aspect | Location |
|--------|----------|
| **Source** | docs/clusters/PHASE4-N-F-G-O/cluster.md |
| **Requirement** | docs/clusters/PHASE4-N-F-G-O/cluster.md *(paraphrased from source/code_locus; registry `statement` empty)* |
| **Code** | crates/ade_runtime/src/admission/wire_pump.rs (handle_block_fetch BlockFetchMessage::Block arm -- calls decompose_blockfetch_block ONCE at the receive boundary, emits bare [era, block]; fail-closed BlockFetchDecode on a non-tag-24 payload); crates/ade_network/src/codec/block_fetch.rs (decompose_blockfetch_block = ade_codec::unwrap_tag24 -- the SINGLE inverse of compose_blockfetch_block, unchanged); crates/ade_node/src/node_sync.rs (run_node_sync -> pump_block consumes the bare [era, block] the wire pump now delivers) |
| **Tests** | `feed_unwrap_decodes_genesis_successor_block_zero`; `block_fetch_unwraps_tag24_emitting_bare_block`; `block_fetch_fails_closed_on_non_tag24_payload`; `served_view_projects_durable_chain` |
| **CI** | `ci/ci_check_feed_tag24_unwrap.sh` |

### CN-LEDGER

#### `CN-LEDGER-01` — _declared_

| Aspect | Location |
|--------|----------|
| **Source** | classification_table.md §B |
| **Requirement** | apply_block must be a pure deterministic function of prior state and canonical block input |
| **Code** | _(no enforcement found — gap)_ |
| **Tests** | _(no tests listed — gap)_ |
| **CI** | _(no CI gate — gap)_ |

#### `CN-LEDGER-02` — _declared_

| Aspect | Location |
|--------|----------|
| **Source** | classification_table.md §B |
| **Requirement** | Same genesis/bootstrap + same block sequence must yield byte-identical authoritative state |
| **Code** | _(no enforcement found — gap)_ |
| **Tests** | _(no tests listed — gap)_ |
| **CI** | _(no CI gate — gap)_ |

#### `CN-LEDGER-03` — _declared_

| Aspect | Location |
|--------|----------|
| **Source** | classification_table.md §B |
| **Requirement** | Validity decisions for transactions and blocks must match the Cardano reference oracle for the same era/protocol version |
| **Code** | _(no enforcement found — gap)_ |
| **Tests** | _(no tests listed — gap)_ |
| **CI** | _(no CI gate — gap)_ |

#### `CN-LEDGER-04` — _declared_

| Aspect | Location |
|--------|----------|
| **Source** | classification_table.md §B |
| **Requirement** | Any two supported production versions that may coexist must return the same validity verdict for every consensus-relevant input |
| **Code** | _(no enforcement found — gap)_ |
| **Tests** | _(no tests listed — gap)_ |
| **CI** | _(no CI gate — gap)_ |

#### `CN-LEDGER-05` — _declared_

| Aspect | Location |
|--------|----------|
| **Source** | classification_table.md §B |
| **Requirement** | Each feature must have one semantic processing result; no alternate path may disagree on whether work was already applied, failed, or remains valid |
| **Code** | _(no enforcement found — gap)_ |
| **Tests** | _(no tests listed — gap)_ |
| **CI** | _(no CI gate — gap)_ |

#### `CN-LEDGER-06` — _declared_

| Aspect | Location |
|--------|----------|
| **Source** | classification_table.md §B |
| **Requirement** | Failure-state residue must be deterministic and consensus-neutral |
| **Code** | _(no enforcement found — gap)_ |
| **Tests** | _(no tests listed — gap)_ |
| **CI** | _(no CI gate — gap)_ |

#### `CN-LEDGER-07` — _declared_

| Aspect | Location |
|--------|----------|
| **Source** | classification_table.md §B |
| **Requirement** | UTxO and asset conservation must hold for every accepted transition, except where protocol rules explicitly authorize mint, burn, rewards, or treasury effects |
| **Code** | _(no enforcement found — gap)_ |
| **Tests** | `conway_conservation_full` |
| **CI** | _(no CI gate — gap)_ |

#### `CN-LEDGER-08` — _declared_

| Aspect | Location |
|--------|----------|
| **Source** | classification_table.md §B |
| **Requirement** | No input or equivalent spend authority may be consumed more than once in an accepted canonical chain |
| **Code** | _(no enforcement found — gap)_ |
| **Tests** | _(no tests listed — gap)_ |
| **CI** | _(no CI gate — gap)_ |

#### `CN-LEDGER-09` — _partial_

| Aspect | Location |
|--------|----------|
| **Source** | classification_table.md §B |
| **Requirement** | Witnesses must bind exactly to the intended body, certificates, withdrawals, governance actions, and scripts for the era |
| **Code** | crates/ade_ledger/src/tx_validity/required_signers.rs (Conway required-signer enumeration over inputs/certs/withdrawals/voters/collateral, grounded in getConwayWitsVKeyNeeded); crates/ade_ledger/src/tx_validity/witness.rs (each required key bound by a witness whose Ed25519 sig over the preserved body hash verifies; an extra irrelevant witness never substitutes); PHASE4-B2-S1 |
| **Tests** | `all_required_covered_is_valid`; `extra_irrelevant_witness_does_not_substitute`; `missing_certificate_witness_rejected`; `missing_withdrawal_witness_rejected`; `missing_governance_voter_witness_rejected`; `witness_correct_key_wrong_body_rejected` |
| **CI** | `ci/ci_check_required_signer_closure.sh` |

#### `CN-LEDGER-10` — _declared_

| Aspect | Location |
|--------|----------|
| **Source** | classification_table.md §B |
| **Requirement** | Conway governance and certificate transitions must occur only through explicit legal state transitions |
| **Code** | _(no enforcement found — gap)_ |
| **Tests** | _(no tests listed — gap)_ |
| **CI** | _(no CI gate — gap)_ |

### CN-EPOCH

#### `CN-EPOCH-01` — _partial_

| Aspect | Location |
|--------|----------|
| **Source** | classification_table.md §C |
| **Requirement** | Stake, rewards, parameter changes, and governance effects may activate only at protocol-defined epoch boundaries |
| **Code** | crates/ade_core/src/consensus/leader_schedule.rs, crates/ade_core/src/consensus/ledger_view.rs, crates/ade_ledger/src/consensus_view.rs |
| **Tests** | `query_uses_state_epoch_nonce_for_vrf_input`; `corpus_returns_canonical_answer_for_known_pools`; `corpus_rejects_unknown_pool`; `corpus_is_deterministic_across_runs`; `view_returns_corpus_pool_stake_and_vrf_keyhash`; `view_unknown_epoch_returns_none`; `view_is_pure` |
| **CI** | _(no CI gate — gap)_ |

#### `CN-EPOCH-02` — _declared_

| Aspect | Location |
|--------|----------|
| **Source** | classification_table.md §C |
| **Requirement** | At each slot or epoch point there is exactly one authoritative committee and governance interpretation |
| **Code** | _(no enforcement found — gap)_ |
| **Tests** | _(no tests listed — gap)_ |
| **CI** | _(no CI gate — gap)_ |

#### `CN-EPOCH-03` — _declared_

| Aspect | Location |
|--------|----------|
| **Source** | classification_table.md §C |
| **Requirement** | Stake snapshots and reward computations must be derivable solely from canonical chain state |
| **Code** | _(no enforcement found — gap)_ |
| **Tests** | _(no tests listed — gap)_ |
| **CI** | _(no CI gate — gap)_ |

#### `CN-EPOCH-04` — _declared_

| Aspect | Location |
|--------|----------|
| **Source** | classification_table.md §C |
| **Requirement** | Future decisions may not leak into present validation, and later states may not retroactively reinterpret prior checkpoints |
| **Code** | _(no enforcement found — gap)_ |
| **Tests** | _(no tests listed — gap)_ |
| **CI** | _(no CI gate — gap)_ |

### CN-CONS

#### `CN-CONS-01` — _enforced_

| Aspect | Location |
|--------|----------|
| **Source** | classification_table.md §D |
| **Requirement** | Chain selection must be deterministic for the same candidate chains and protocol observables |
| **Code** | crates/ade_core/src/consensus/fork_choice.rs, crates/ade_core/src/consensus/candidate.rs |
| **Tests** | `replay_is_deterministic`; `reject_reason_bytes_are_stable`; `higher_block_no_wins`; `equal_block_no_tiebreaker_decides`; `select_best_chain_arrival_order_independent_distinct_heights`; `select_best_chain_arrival_order_independent_tiebreaker` |
| **CI** | `ci/ci_check_no_density_in_fork_choice.sh`; `ci/ci_check_chain_selection_arrival_order_independent.sh` |

#### `CN-CONS-02` — _declared_

| Aspect | Location |
|--------|----------|
| **Source** | classification_table.md §D |
| **Requirement** | Supported rollout skew must not allow a single adversarial input to induce persistent honest-node consensus divergence |
| **Code** | crates/ade_core/src/consensus/fork_choice.rs |
| **Tests** | `reject_reason_bytes_are_stable`; `replay_is_deterministic` |
| **CI** | `ci/ci_check_no_density_in_fork_choice.sh`; `ci/ci_check_consensus_closed_enums.sh` |

#### `CN-CONS-03` — _enforced_

| Aspect | Location |
|--------|----------|
| **Source** | classification_table.md §D |
| **Requirement** | After temporary partition, honest nodes must converge using only protocol-defined observables and declared emergency procedures |
| **Code** | crates/ade_core/src/consensus/fork_choice.rs |
| **Tests** | `higher_block_no_wins`; `equal_block_no_tiebreaker_decides`; `tiebreaker_loss_keeps_current` |
| **CI** | `ci/ci_check_no_density_in_fork_choice.sh` |

#### `CN-CONS-04` — _enforced_

| Aspect | Location |
|--------|----------|
| **Source** | classification_table.md §D |
| **Requirement** | Header validation must bind exactly to the accepted body and consensus context |
| **Code** | crates/ade_core/src/consensus/header_validate.rs, crates/ade_core/src/consensus/header_summary.rs, crates/ade_core/src/consensus/fork_choice.rs, crates/ade_core/src/consensus/kes_check.rs (VRF-keyhash binding + KES authentication for Praos headers; PHASE4-B1-S5), crates/ade_ledger/src/block_validity/transition.rs, crates/ade_ledger/src/block_validity/header_input.rs (wired body-hash binding: recomputed segwit body hash compared to the validated header body_hash before body application; PHASE4-B1-S4) |
| **Tests** | `pipeline_short_circuits_on_first_failure`; `nonce_contribution_uses_nonce_role_vrf_output_not_leader_role`; `valid_header_accepted_advances_state`; `header_with_slot_regression_rejected`; `header_with_block_no_regression_rejected`; `header_with_op_cert_regression_rejected`; `header_with_invalid_vrf_proof_rejected`; `header_beyond_forecast_horizon_rejected`; `validate_replay_is_deterministic`; `candidate_fragment_carries_anchor_block_no`; … (+3 more) |
| **CI** | `ci/ci_check_header_body_binding.sh` |

#### `CN-CONS-05` — _declared_

| Aspect | Location |
|--------|----------|
| **Source** | classification_table.md §D |
| **Requirement** | Authoritative consensus decisions must not depend on wall-clock time, arrival-order races, scheduler interleavings, or OS behavior |
| **Code** | crates/ade_core/src/consensus/header_validate.rs, crates/ade_core/src/consensus/fork_choice.rs, crates/ade_core/src/consensus/candidate.rs |
| **Tests** | `validate_replay_is_deterministic`; `pipeline_short_circuits_on_first_failure`; `nonce_contribution_uses_nonce_role_vrf_output_not_leader_role`; `replay_is_deterministic`; `reject_reason_bytes_are_stable` |
| **CI** | `ci/ci_check_no_density_in_fork_choice.sh`; `ci/ci_check_no_float_in_consensus.sh` |

#### `CN-CONS-06` — _enforced_

| Aspect | Location |
|--------|----------|
| **Source** | docs/planning/phase4-n-c-invariants.md §1 (NC-LIVE-1); bounty acceptance test (validation + block-production both required) |
| **Requirement** | Cross-impl acceptance: blocks forged by Ade are accepted by cardano-node when delivered via N2N block-fetch / chain-sync. Evidence is operator-action: a sustained-window live_block_production_session against a private cardano-node capturing CE-N-C-LIVE_<date>.log. Conditional on testnet stake / SPO registration; if unavailable at cluster close the live half is marked blocked_until_operator_stake_available, not deferred. |
| **Code** | crates/ade_testkit/src/producer/cross_impl_adapter.rs (mechanical half — structural cross-impl agreement: decode round-trip + body-hash binding + decoder/encoder structural field agreement); crates/ade_runtime/src/producer/coordinator.rs (PHASE4-N-Q GREEN slot+forge-result coordinator); crates/ade_runtime/src/producer/producer_shell.rs (PHASE4-N-Q RED key-custody shell); crates/ade_node/src/produce_mode.rs (RED driver — run_real_forge live forge composition: N-R-A BLUE leader-check + N-S-A real KES over unsigned_header_pre_image + N-S-B OutboundCommand relay + N-W Praos VRF + N-X tag-24 serve); crates/ade_core_interop/src/bin/live_block_production_session.rs (legacy operator-action… |
| **Tests** | `cross_impl_adapter_forged_block_decodes_through_ade_codec`; `cross_impl_adapter_forged_block_structurally_agrees_with_decoder`; `cross_impl_adapter_corpus_round_trips_byte_identical` |
| **CI** | `ci/ci_check_producer_corpus_present.sh` |

#### `CN-CONS-07` — _enforced_

| Aspect | Location |
|--------|----------|
| **Source** | docs/planning/phase4-n-c-invariants.md §1 (NC-SELF-1); docs/clusters/PHASE4-N-U/S3-serve-as-durable-chain-projection.md (serve-provenance restatement) |
| **Requirement** | docs/clusters/PHASE4-N-U/S3-serve-as-durable-chain-projection.md (serve-provenance restatement) *(paraphrased from source/code_locus; registry `statement` empty)* |
| **Code** | crates/ade_ledger/src/producer/self_accept.rs (self_accept, AcceptedBlock, SelfAcceptError); crates/ade_ledger/src/block_validity/transition.rs (block_validity — single closed validator authority self_accept wraps); crates/ade_ledger/src/producer/served_chain.rs (ServedChainSnapshot, served_chain_admit — only AcceptedBlock values may enter the produce-mode served-chain index; PHASE4-N-G S2 strengthening preserves the broadcast gate across the network seam); crates/ade_runtime/src/network/served_chain_projection.rs (ChainDbServedSource — PHASE4-N-U S3: the --mode node served view is a read-only projection of the durable ChainDb, whose sole production writers are pump_block +… |
| **Tests** | `self_accept_accepts_freshly_forged_block`; `self_accept_rejects_corrupted_body_hash`; `self_accept_rejects_invalid_kes_signature`; `self_accept_rejects_unbalanced_tx_in_body`; `broadcast_callable_only_with_accept_verdict`; `served_chain_admit_admits_corpus_block`; `served_chain_admit_idempotent_on_byte_identity`; `served_chain_admit_independent_of_order`; `served_chain_snapshot_iteration_is_btreemap_ordered`; `served_chain_block_bytes_accessor_returns_accepted_block_slice`; … (+5 more) |
| **CI** | `ci/ci_check_self_accept_gate.sh`; `ci/ci_check_served_chain_closure.sh`; `ci/ci_check_admitted_block_closure.sh`; `ci/ci_check_receive_reducer_closure.sh` |

#### `CN-CONS-08` — _enforced_

| Aspect | Location |
|--------|----------|
| **Source** | docs/planning/receive-side-bridge-invariants.md §1 (I-1, folds I-7) |
| **Requirement** | docs/planning/receive-side-bridge-invariants.md §1 (I-1, folds I-7) *(paraphrased from source/code_locus; registry `statement` empty)* |
| **Code** | crates/ade_ledger/src/receive/admitted.rs (AdmittedBlock + admit_via_block_validity — the single admission authority); crates/ade_ledger/src/receive/reducer.rs (block_delivered helper composes admit_via_block_validity then commits state atomically; failure leaves state unchanged); crates/ade_ledger/src/receive/chain_write.rs (ChainDbWrite::write_admitted takes AdmittedBlock by value) |
| **Tests** | `admit_via_block_validity_accepts_corpus_block`; `admit_via_block_validity_rejects_corrupted_body`; `receive_apply_block_delivered_with_matching_header_admits`; `receive_apply_block_delivered_validity_invalid_rejects`; `receive_apply_rollback_returns_out_of_scope`; `receive_apply_replay_byte_identical_over_corpus` |
| **CI** | `ci/ci_check_admitted_block_closure.sh`; `ci/ci_check_receive_reducer_closure.sh` |

#### `CN-CONS-IN-01` — _enforced_

| Aspect | Location |
|--------|----------|
| **Source** | docs/planning/phase4-n-m-c-operator-pass-invariants.md §1 (I-C1) |
| **Requirement** | docs/planning/phase4-n-m-c-operator-pass-invariants.md §1 (I-C1) *(paraphrased from source/code_locus; registry `statement` empty)* |
| **Code** | crates/ade_runtime/src/consensus_inputs/importer.rs (import_live_consensus_inputs_raw / _from_bytes, raw typed form + closed error sum), crates/ade_runtime/src/consensus_inputs/canonical.rs (import_live_consensus_inputs / _from_bytes — SOLE Canonical-returning authority + canonical_from_raw lift), crates/ade_runtime/src/consensus_inputs/json.rs (parse_consensus_inputs_json structural decode) |
| **Tests** | `minimal_round_trip_imports_to_typed`; `unsupported_era_fails_fast`; `import_is_deterministic_across_repeated_calls`; `import_round_trip_yields_canonical_form_with_fingerprint`; `fingerprint_is_deterministic_across_repeated_imports` |
| **CI** | `ci/ci_check_live_consensus_inputs_closure.sh`; `ci/ci_check_live_consensus_inputs_fingerprint.sh` |

### CN-PLUTUS

#### `CN-PLUTUS-01` — _enforced_

| Aspect | Location |
|--------|----------|
| **Source** | classification_table.md §E |
| **Requirement** | Same script + same redeemers/datum/context + same cost model must produce identical result and budget accounting |
| **Code** | crates/ade_plutus/src/tx_eval.rs (eval_tx_phase_two: pure function of canonical inputs); crates/ade_plutus/src/evaluator.rs (pinned aiken UPLC engine); crates/ade_testkit/tests/plutus_conformance.rs (IOG conformance suite, exact outcome); docs/evidence/plutus-conformance-manifest.toml (bound evidence) |
| **Tests** | `plutus_eval_is_deterministic`; `plutus_conformance_evaluation_suite` |
| **CI** | `ci/ci_check_plutus_eval_purity.sh`; `ci/ci_check_plutus_conformance.sh` |

#### `CN-PLUTUS-02` — _declared_

| Aspect | Location |
|--------|----------|
| **Source** | classification_table.md §E |
| **Requirement** | Budget exhaustion and script failure must have a single deterministic failure shape |
| **Code** | crates/ade_plutus/src/tx_eval.rs (per-script declared ex_units cap: declared_ex_units_by_pointer derives each redeemer's declared budget; actual<=declared binds PerScriptResult.success); crates/ade_ledger/src/plutus_eval.rs (a script over its declared cap -> PlutusEvalOutcome::Failed) |
| **Tests** | `under_declared_ex_units_must_reject`; `failing_validator_must_reject`; `extraneous_redeemer_must_reject`; `declared_ex_units_array_form_parsed_by_pointer`; `declared_ex_units_conway_map_form_parsed_by_pointer`; `extract_redeemer_fields_reads_pointer_and_ex_units`; `aiken_fixture_tx_evaluates_end_to_end` |
| **CI** | `ci/ci_check_plutus_budget_cap.sh` |

#### `CN-PLUTUS-03` — _declared_

| Aspect | Location |
|--------|----------|
| **Source** | classification_table.md §E |
| **Requirement** | Script context must be canonically and completely derived from transaction plus ledger state |
| **Code** | _(no enforcement found — gap)_ |
| **Tests** | _(no tests listed — gap)_ |
| **CI** | _(no CI gate — gap)_ |

#### `CN-PLUTUS-04` — _enforced_

| Aspect | Location |
|--------|----------|
| **Source** | classification_table.md §E |
| **Requirement** | No host-environment property may influence script results |
| **Code** | crates/ade_plutus/src/ (BLUE evaluator crate: passes only canonical inputs -- tx / resolved UTxOs / cost model / per-script budget / slot config, all parameters -- to the pinned aiken evaluator; the slot config is never read from the host) |
| **Tests** | `plutus_eval_is_deterministic` |
| **CI** | `ci/ci_check_plutus_eval_purity.sh` |

### CN-PROTO

#### `CN-PROTO-01` — _declared_

| Aspect | Location |
|--------|----------|
| **Source** | classification_table.md §F |
| **Requirement** | Each miniprotocol must be an explicit deterministic state machine with legal typed transitions only |
| **Code** | _(no enforcement found — gap)_ |
| **Tests** | _(no tests listed — gap)_ |
| **CI** | _(no CI gate — gap)_ |

#### `CN-PROTO-02` — _declared_

| Aspect | Location |
|--------|----------|
| **Source** | classification_table.md §F |
| **Requirement** | For the same peer transcript, authoritative state and outbound transcript must be identical |
| **Code** | _(no enforcement found — gap)_ |
| **Tests** | _(no tests listed — gap)_ |
| **CI** | _(no CI gate — gap)_ |

#### `CN-PROTO-03` — _declared_

| Aspect | Location |
|--------|----------|
| **Source** | classification_table.md §F |
| **Requirement** | Agency must be enforced strictly; impossible messages must fail deterministically |
| **Code** | _(no enforcement found — gap)_ |
| **Tests** | _(no tests listed — gap)_ |
| **CI** | _(no CI gate — gap)_ |

#### `CN-PROTO-04` — _declared_

| Aspect | Location |
|--------|----------|
| **Source** | classification_table.md §F |
| **Requirement** | Socket fragmentation, multiplexing, arrival order, and timeout behavior must not leak nondeterminism into authoritative logic |
| **Code** | _(no enforcement found — gap)_ |
| **Tests** | _(no tests listed — gap)_ |
| **CI** | _(no CI gate — gap)_ |

#### `CN-PROTO-05` — _declared_

| Aspect | Location |
|--------|----------|
| **Source** | classification_table.md §F |
| **Requirement** | Untrusted network inputs must not allocate unbounded authoritative resources before deterministic validation |
| **Code** | _(no enforcement found — gap)_ |
| **Tests** | _(no tests listed — gap)_ |
| **CI** | _(no CI gate — gap)_ |

#### `CN-PROTO-06` — _enforced_

| Aspect | Location |
|--------|----------|
| **Source** | docs/planning/phase4-n-a-successor-invariants.md §1 (I-4) |
| **Requirement** | The producer-side session orchestrator can only construct outgoing mini-protocol messages tagged with Server agency. Client-originated messages from the server-role pump are unrepresentable in the public API; misuse is a compile error (closed ServerReply<M> wrapper). |
| **Code** | crates/ade_network/src/chain_sync/server.rs (ServerReply for chain-sync); crates/ade_network/src/block_fetch/server.rs (ServerReply for block-fetch); crates/ade_ledger/src/block_validity/header_input.rs (accepted_block_header_bytes — the single canonical header projection the chain-sync ServerReply::roll_forward consumes) |
| **Tests** | `chain_sync_server_reply_round_trips_through_codec`; `chain_sync_server_reply_into_message_only_yields_server_variants`; `block_fetch_server_reply_round_trips_through_codec`; `block_fetch_server_reply_into_message_only_yields_server_variants`; `accepted_block_header_bytes_equals_validator_split_on_corpus`; `accepted_block_header_bytes_is_subslice_of_as_bytes`; `accepted_block_header_bytes_rejects_malformed_envelope` |
| **CI** | `ci/ci_check_no_parallel_header_splitter.sh` |

#### `CN-PROTO-07` — _enforced_

| Aspect | Location |
|--------|----------|
| **Source** | docs/planning/receive-side-bridge-invariants.md §1 (closure) |
| **Requirement** | docs/planning/receive-side-bridge-invariants.md §1 (closure) *(paraphrased from source/code_locus; registry `statement` empty)* |
| **Code** | crates/ade_ledger/src/receive/events.rs (ReceiveEvent closed sum: only RollForward, RollBackward, BlockDelivered variants — no constructor for RequestNext, RequestRange, ClientDone, FindIntersect) |
| **Tests** | `receive_event_round_trips_through_pattern_match`; `receive_effect_round_trips_through_pattern_match`; `receive_error_round_trips_through_pattern_match` |
| **CI** | `ci/ci_check_admitted_block_closure.sh` |

### CN-NET

#### `CN-NET-01` — _declared_

| Aspect | Location |
|--------|----------|
| **Source** | classification_table.md §G |
| **Requirement** | A block producer must not accept arbitrary public peer connectivity; it may connect only through trusted relay topology |
| **Code** | _(no enforcement found — gap)_ |
| **Tests** | _(no tests listed — gap)_ |
| **CI** | _(no CI gate — gap)_ |

#### `CN-NET-02` — _declared_

| Aspect | Location |
|--------|----------|
| **Source** | classification_table.md §G |
| **Requirement** | Relay paths must be geographically and topologically diverse enough that isolating one path does not prevent timely propagation |
| **Code** | _(no enforcement found — gap)_ |
| **Tests** | _(no tests listed — gap)_ |
| **CI** | _(no CI gate — gap)_ |

#### `CN-NET-03` — _declared_

| Aspect | Location |
|--------|----------|
| **Source** | classification_table.md §G |
| **Requirement** | No single peer, ASN, region, or operator cluster may dominate the node's authoritative view |
| **Code** | _(no enforcement found — gap)_ |
| **Tests** | _(no tests listed — gap)_ |
| **CI** | _(no CI gate — gap)_ |

#### `CN-NET-04` — _declared_

| Aspect | Location |
|--------|----------|
| **Source** | classification_table.md §G |
| **Requirement** | Peer selection and promotion policies must not allow one adversary-controlled set to deterministically starve honest views |
| **Code** | _(no enforcement found — gap)_ |
| **Tests** | _(no tests listed — gap)_ |
| **CI** | _(no CI gate — gap)_ |

### CN-MEM

#### `CN-MEM-01` — _partial_

| Aspect | Location |
|--------|----------|
| **Source** | classification_table.md §H |
| **Requirement** | Untrusted inbound work must be admitted through deterministic bounded policies before consuming scarce authoritative resources |
| **Code** | crates/ade_node/src/mem_measure/bounded_admission.rs; crates/ade_node/src/mem_measure/runner.rs |
| **Tests** | `bounded_admission_respects_count_budget`; `bounded_admission_respects_byte_budget`; `bounded_admission_is_deterministic`; `bounded_gate_under_budget_equals_unbounded`; `bounded_gate_preserves_admit_verdict`; `bounded_gate_no_false_accept_under_pressure`; `hermetic_measurement_verdict_is_agreed`; `hermetic_measurement_is_replay_stable` |
| **CI** | `ci/ci_check_bounded_inbound_admission.sh` |

#### `CN-MEM-02` — _declared_

| Aspect | Location |
|--------|----------|
| **Source** | classification_table.md §H |
| **Requirement** | Mempool pressure and peer churn must not starve block validation, chain selection, or persistence |
| **Code** | _(no enforcement found — gap)_ |
| **Tests** | _(no tests listed — gap)_ |
| **CI** | _(no CI gate — gap)_ |

#### `CN-MEM-03` — _declared_

| Aspect | Location |
|--------|----------|
| **Source** | classification_table.md §H |
| **Requirement** | Under overload, work shedding must follow deterministic policy, not timing-dependent collapse |
| **Code** | _(no enforcement found — gap)_ |
| **Tests** | _(no tests listed — gap)_ |
| **CI** | _(no CI gate — gap)_ |

#### `CN-MEM-04` — _declared_

| Aspect | Location |
|--------|----------|
| **Source** | classification_table.md §H |
| **Requirement** | Mempool acceptance rules must never contradict block and ledger acceptance rules for the same authoritative semantics |
| **Code** | _(no enforcement found — gap)_ |
| **Tests** | _(no tests listed — gap)_ |
| **CI** | _(no CI gate — gap)_ |

### CN-STORE

#### `CN-STORE-01` — _declared_

| Aspect | Location |
|--------|----------|
| **Source** | classification_table.md §I |
| **Requirement** | No authoritative storage initialization may occur before bootstrap or anchor verification succeeds |
| **Code** | _(no enforcement found — gap)_ |
| **Tests** | _(no tests listed — gap)_ |
| **CI** | _(no CI gate — gap)_ |

#### `CN-STORE-02` — _partial_

| Aspect | Location |
|--------|----------|
| **Source** | classification_table.md §I |
| **Requirement** | WAL entries, checkpoints, and recovered artifacts must be bound to exactly one anchor or bootstrap lineage |
| **Code** | crates/ade_node/src/node_lifecycle.rs (warm_start_recovery: anchor-lineage discovery + fail-closed on multiple/mismatched anchors) |
| **Tests** | `warm_start_fails_closed_on_multiple_anchor_lineages`; `warm_start_fails_closed_on_anchor_mismatch`; `warm_start_fails_closed_on_duplicate_provenance` |
| **CI** | _(no CI gate — gap)_ |

#### `CN-STORE-03` — _enforced_

| Aspect | Location |
|--------|----------|
| **Source** | classification_table.md §I |
| **Requirement** | Crash recovery must produce the same authoritative state as clean replay over the accepted canonical inputs |
| **Code** | crates/ade_runtime/src/chaindb/crash_safety.rs, crates/ade_runtime/tests/stress_kill_harness.rs |
| **Tests** | `stress_kill_smoke`; `stress_kill_1000`; `snapshot_table_intact_after_kill_loop`; `persistent_passes_crash_safety_with_no_kill` |
| **CI** | `ci/ci_check_chaindb_crash_safety.sh` |

#### `CN-STORE-04` — _enforced_

| Aspect | Location |
|--------|----------|
| **Source** | classification_table.md §I |
| **Requirement** | Checkpoints must be atomic: fully committed and valid, or absent |
| **Code** | crates/ade_runtime/src/chaindb/snapshot_contract.rs, crates/ade_runtime/src/chaindb/persistent.rs |
| **Tests** | `persistent_passes_snapshot_contract`; `in_memory_passes_snapshot_contract`; `snapshots_persist_across_reopen`; `corrupted_magic_returns_corruption_error` |
| **CI** | `ci/ci_check_chaindb_contract.sh` |

#### `CN-STORE-05` — _enforced_

| Aspect | Location |
|--------|----------|
| **Source** | classification_table.md §I |
| **Requirement** | Finalized provenance must be append-only, auditable, and replay-derivable |
| **Code** | crates/ade_runtime/src/chaindb/persistent.rs, crates/ade_runtime/src/chaindb/contract.rs |
| **Tests** | `persistent_passes_contract`; `in_memory_passes_contract`; `reopen_observes_committed_block` |
| **CI** | `ci/ci_check_chaindb_contract.sh` |

#### `CN-STORE-06` — _declared_

| Aspect | Location |
|--------|----------|
| **Source** | classification_table.md §I |
| **Requirement** | On-disk bytes must re-enter through the same canonical validation and decode chokepoints as network inputs |
| **Code** | _(no enforcement found — gap)_ |
| **Tests** | _(no tests listed — gap)_ |
| **CI** | _(no CI gate — gap)_ |

#### `CN-STORE-07` — _enforced_

| Aspect | Location |
|--------|----------|
| **Source** | docs/planning/ledger-snapshot-rollback-invariants.md §1 (I-5) |
| **Requirement** | docs/planning/ledger-snapshot-rollback-invariants.md §1 (I-5) *(paraphrased from source/code_locus; registry `statement` empty)* |
| **Code** | crates/ade_ledger/src/rollback/materialize.rs (materialize_rolled_back_state — the sole pub fn returning (LedgerState, PraosChainDepState) in the rollback module tree); crates/ade_ledger/src/rollback/traits.rs (SnapshotReader + BlockSource narrow read-only traits — production impls in ade_runtime go through the same single composition) |
| **Tests** | `materialize_returns_rollback_too_deep_when_no_snapshot`; `materialize_with_snapshot_at_target_returns_snapshot_state`; `materialize_with_snapshot_below_target_replays_forward`; `materialize_fails_closed_on_invalid_block`; `materialize_replay_forward_equals_direct_apply` |
| **CI** | `ci/ci_check_rollback_materialize_closure.sh` |

#### `CN-STORE-08` — _enforced_

| Aspect | Location |
|--------|----------|
| **Source** | docs/planning/persistent-snapshot-encoder-invariants.md §1 (I-5) |
| **Requirement** | docs/planning/persistent-snapshot-encoder-invariants.md §1 (I-5) *(paraphrased from source/code_locus; registry `statement` empty)* |
| **Code** | crates/ade_ledger/src/snapshot/{ledger,chain_dep,framing}.rs |
| **Tests** | _(no tests listed in registry — enforced via Code/CI; gap: name the proving test)_ |
| **CI** | `ci/ci_check_snapshot_encoder_closure.sh` |

### CN-CRYPTO

#### `CN-CRYPTO-01` — _declared_

| Aspect | Location |
|--------|----------|
| **Source** | classification_table.md §J |
| **Requirement** | Verification belongs to the authoritative core; signing belongs outside it |
| **Code** | _(no enforcement found — gap)_ |
| **Tests** | _(no tests listed — gap)_ |
| **CI** | _(no CI gate — gap)_ |

#### `CN-CRYPTO-02` — _partial_

| Aspect | Location |
|--------|----------|
| **Source** | classification_table.md §J |
| **Requirement** | All consensus-relevant hashes must be domain-separated and unambiguous |
| **Code** | crates/ade_core/src/consensus/vrf_cert.rs |
| **Tests** | `vrf_input_layout_is_41_bytes_with_correct_tag`; `vrf_role_tags_match_convention`; `vrf_input_byte_layout` |
| **CI** | _(no CI gate — gap)_ |

#### `CN-CRYPTO-03` — _declared_

| Aspect | Location |
|--------|----------|
| **Source** | classification_table.md §J |
| **Requirement** | All collections with consensus meaning must be ordered deterministically before hashing or comparison |
| **Code** | _(no enforcement found — gap)_ |
| **Tests** | _(no tests listed — gap)_ |
| **CI** | _(no CI gate — gap)_ |

#### `CN-CRYPTO-04` — _declared_

| Aspect | Location |
|--------|----------|
| **Source** | classification_table.md §J |
| **Requirement** | Verification failure must fail once and deterministically; no implicit parser or serialization fallback is allowed |
| **Code** | _(no enforcement found — gap)_ |
| **Tests** | _(no tests listed — gap)_ |
| **CI** | _(no CI gate — gap)_ |

### CN-BUILD

#### `CN-BUILD-01` — _declared_

| Aspect | Location |
|--------|----------|
| **Source** | classification_table.md §K |
| **Requirement** | No build profile, feature flag, cfg, or optimization mode may alter authoritative semantics or persisted bytes |
| **Code** | _(no enforcement found — gap)_ |
| **Tests** | _(no tests listed — gap)_ |
| **CI** | _(no CI gate — gap)_ |

#### `CN-BUILD-02` — _declared_

| Aspect | Location |
|--------|----------|
| **Source** | classification_table.md §K |
| **Requirement** | All semantic variability must be explicit runtime protocol data, not hidden compile-time choice |
| **Code** | _(no enforcement found — gap)_ |
| **Tests** | _(no tests listed — gap)_ |
| **CI** | _(no CI gate — gap)_ |

#### `CN-BUILD-03` — _declared_

| Aspect | Location |
|--------|----------|
| **Source** | classification_table.md §K |
| **Requirement** | Exactly one semantic interpretation may exist for a given protocol version and bootstrap anchor |
| **Code** | _(no enforcement found — gap)_ |
| **Tests** | _(no tests listed — gap)_ |
| **CI** | _(no CI gate — gap)_ |

#### `CN-BUILD-04` — _declared_

| Aspect | Location |
|--------|----------|
| **Source** | classification_table.md §K |
| **Requirement** | Operator configuration may tune transport, logging, and telemetry, but may not silently weaken ledger, consensus, or persistence semantics |
| **Code** | _(no enforcement found — gap)_ |
| **Tests** | _(no tests listed — gap)_ |
| **CI** | _(no CI gate — gap)_ |

### CN-TEST

#### `CN-TEST-01` — _declared_

| Aspect | Location |
|--------|----------|
| **Source** | classification_table.md §L |
| **Requirement** | Consensus-relevant inputs must be fuzzed differentially across all supported versions and decode or validation paths; any verdict mismatch is release-blocking |
| **Code** | _(no enforcement found — gap)_ |
| **Tests** | _(no tests listed — gap)_ |
| **CI** | _(no CI gate — gap)_ |

#### `CN-TEST-02` — _declared_

| Aspect | Location |
|--------|----------|
| **Source** | classification_table.md §L |
| **Requirement** | Every malformed or discrepant input that ever triggered a fork, preview mismatch, or parser disagreement becomes a permanent regression corpus entry |
| **Code** | _(no enforcement found — gap)_ |
| **Tests** | _(no tests listed — gap)_ |
| **CI** | _(no CI gate — gap)_ |

#### `CN-TEST-03` — _declared_

| Aspect | Location |
|--------|----------|
| **Source** | classification_table.md §L |
| **Requirement** | Previously failed, duplicate, or boundary-case inputs must remain verdict-stable under resubmission and replay |
| **Code** | _(no enforcement found — gap)_ |
| **Tests** | _(no tests listed — gap)_ |
| **CI** | _(no CI gate — gap)_ |

### CN-REL

#### `CN-REL-01` — _declared_

| Aspect | Location |
|--------|----------|
| **Source** | classification_table.md §L |
| **Requirement** | A release is not mainnet-eligible unless mixed-version topologies against supported predecessors show consensus equivalence on malformed and boundary-case inputs |
| **Code** | _(no enforcement found — gap)_ |
| **Tests** | _(no tests listed — gap)_ |
| **CI** | _(no CI gate — gap)_ |

#### `CN-REL-02` — _declared_

| Aspect | Location |
|--------|----------|
| **Source** | classification_table.md §L |
| **Requirement** | No single implementation bug should exceed the protocol's intended safety or liveness fault threshold at ecosystem level |
| **Code** | _(no enforcement found — gap)_ |
| **Tests** | _(no tests listed — gap)_ |
| **CI** | _(no CI gate — gap)_ |

#### `CN-REL-03` — _declared_

| Aspect | Location |
|--------|----------|
| **Source** | classification_table.md §L |
| **Requirement** | Cross-implementation accept/reject agreement on authoritative corpora is release-blocking |
| **Code** | _(no enforcement found — gap)_ |
| **Tests** | _(no tests listed — gap)_ |
| **CI** | _(no CI gate — gap)_ |

### CN-OPS

#### `CN-OPS-01` — _declared_

| Aspect | Location |
|--------|----------|
| **Source** | classification_table.md §M |
| **Requirement** | After any partition, authoritative post-incident reconciliation must be derived solely from the recovered canonical chain |
| **Code** | _(no enforcement found — gap)_ |
| **Tests** | _(no tests listed — gap)_ |
| **CI** | _(no CI gate — gap)_ |

#### `CN-OPS-02` — _declared_

| Aspect | Location |
|--------|----------|
| **Source** | classification_table.md §M |
| **Requirement** | Emergency recovery procedures must have explicit admissibility criteria, deterministic inputs and outputs, and defined authority thresholds |
| **Code** | _(no enforcement found — gap)_ |
| **Tests** | _(no tests listed — gap)_ |
| **CI** | _(no CI gate — gap)_ |

#### `CN-OPS-03` — _declared_

| Aspect | Location |
|--------|----------|
| **Source** | classification_table.md §M |
| **Requirement** | Incident evidence must be sufficient to reconstruct the canonical decision path without relying on nondeterministic logs or local operator memory |
| **Code** | _(no enforcement found — gap)_ |
| **Tests** | _(no tests listed — gap)_ |
| **CI** | _(no CI gate — gap)_ |

### CN-META

#### `CN-META-01` — _declared_

| Aspect | Location |
|--------|----------|
| **Source** | classification_table.md §N |
| **Requirement** | Every claimed invariant must have at least one mechanical enforcement point |
| **Code** | _(no enforcement found — gap)_ |
| **Tests** | _(no tests listed — gap)_ |
| **CI** | _(no CI gate — gap)_ |

#### `CN-META-02` — _declared_

| Aspect | Location |
|--------|----------|
| **Source** | classification_table.md §N |
| **Requirement** | Every consensus-relevant failure mode must have a deterministic structured error shape |
| **Code** | _(no enforcement found — gap)_ |
| **Tests** | _(no tests listed — gap)_ |
| **CI** | _(no CI gate — gap)_ |

#### `CN-META-03` — _declared_

| Aspect | Location |
|--------|----------|
| **Source** | classification_table.md §N |
| **Requirement** | Every equivalence claim must be reproducible from named fixtures, oracle versions, and replay inputs |
| **Code** | _(no enforcement found — gap)_ |
| **Tests** | _(no tests listed — gap)_ |
| **CI** | _(no CI gate — gap)_ |

### CN-NODE

#### `CN-NODE-01` — _enforced_

| Aspect | Location |
|--------|----------|
| **Source** | docs/planning/phase4-n-k-orchestrator-binary-invariants.md §1 (I-2) |
| **Requirement** | docs/planning/phase4-n-k-orchestrator-binary-invariants.md §1 (I-2) *(paraphrased from source/code_locus; registry `statement` empty)* |
| **Code** | crates/ade_runtime/src/bootstrap.rs |
| **Tests** | `bootstrap_cold_start_returns_genesis_when_empty`; `bootstrap_cold_start_without_genesis_errors`; `bootstrap_warm_start_materializes_from_persistent_snapshot`; `bootstrap_warm_start_equals_direct_materialize`; `bootstrap_two_runs_produce_byte_identical_state` |
| **CI** | `ci/ci_check_bootstrap_closure.sh`; `ci/ci_check_produce_mode_uses_bootstrap_initial_state.sh`; `ci/ci_check_node_mode_closure.sh` |

#### `CN-NODE-02` — _enforced_

| Aspect | Location |
|--------|----------|
| **Source** | docs/planning/phase4-n-f-d-live-node-run-loop-invariants.md |
| **Requirement** | docs/planning/phase4-n-f-d-live-node-run-loop-invariants.md *(paraphrased from source/code_locus; registry `statement` empty)* |
| **Code** | crates/ade_node/src/node_lifecycle.rs |
| **Tests** | `relay_loop_syncs_then_halts_clean_on_source_end`; `relay_loop_halts_clean_on_shutdown_no_partial_write`; `relay_loop_idles_then_syncs_on_incremental_feed`; `relay_loop_fails_closed_on_unapplyable_block`; `plan_loop_step_forge_precedence_table_is_total` |
| **CI** | `ci/ci_check_node_run_loop_containment.sh`; `ci/ci_check_loop_planner_closed.sh` |

#### `CN-NODE-03` — _enforced_

| Aspect | Location |
|--------|----------|
| **Source** | docs/planning/phase4-n-f-f-invariants.md |
| **Requirement** | docs/planning/phase4-n-f-f-invariants.md *(paraphrased from source/code_locus; registry `statement` empty)* |
| **Code** | crates/ade_node/src/forge_intent.rs (GREEN tri-state classifier); crates/ade_node/src/operator_forge.rs (RED ingress + activation assembly); crates/ade_node/src/node_lifecycle.rs (Some/None binary flip + ForgeKeyIngress) |
| **Tests** | `classify_forge_intent_total_over_all_32_flag_combinations`; `classify_forge_intent_none_present_is_off`; `forge_intent_error_carries_no_path_bytes`; `load_operator_producer_shell_builds_shell_from_complete_material`; `load_operator_producer_shell_kes_period_past_opcert_fails_closed`; `operator_forge_error_carries_no_path_or_key_bytes`; `build_operator_forge_material_from_complete_material`; `node_mode_with_operator_keys_warm_start_forge_capable_halts_clean`; `node_mode_partial_operator_keys_fail_closed`; `relay_loop_with_operator_material_forge_reaches_fenced_path`; … (+1 more) |
| **CI** | `ci/ci_check_forge_intent_closed.sh`; `ci/ci_check_operator_forge_no_secret_leak.sh`; `ci/ci_check_node_run_loop_containment.sh` |

#### `CN-NODE-04` — _enforced_

| Aspect | Location |
|--------|----------|
| **Source** | docs/planning/phase4-n-f-g-j-invariants.md |
| **Requirement** | docs/planning/phase4-n-f-g-j-invariants.md *(paraphrased from source/code_locus; registry `statement` empty)* |
| **Code** | crates/ade_node/src/node_sync.rs; crates/ade_node/src/node_lifecycle.rs; crates/ade_node/src/live_log/sched_event.rs; crates/ade_node/src/live_log/sched_writer.rs (closed event vocabulary + emit-only JSONL writer) |
| **Tests** | `node_sched_events_emit_closed_vocabulary`; `node_sched_event_allowlist_rejects_unknown_variants` |
| **CI** | `ci/ci_check_node_sched_events_emit_only.sh` |

### CN-SESS

#### `CN-SESS-01` — _enforced_

| Aspect | Location |
|--------|----------|
| **Source** | docs/planning/phase4-n-l-wire-protocol-invariants.md §1 (I-7) |
| **Requirement** | docs/planning/phase4-n-l-wire-protocol-invariants.md §1 (I-7) *(paraphrased from source/code_locus; registry `statement` empty)* |
| **Code** | crates/ade_network/src/mux/frame.rs |
| **Tests** | `frame_roundtrip_byte_identical` |
| **CI** | `ci/ci_check_mux_frame_closure.sh` |

#### `CN-SESS-02` — _enforced_

| Aspect | Location |
|--------|----------|
| **Source** | docs/planning/phase4-n-l-wire-protocol-invariants.md §1 (I-1) |
| **Requirement** | docs/planning/phase4-n-l-wire-protocol-invariants.md §1 (I-1) *(paraphrased from source/code_locus; registry `statement` empty)* |
| **Code** | crates/ade_network/src/handshake/transition.rs |
| **Tests** | `handshake_initiator_accepts_when_responder_supports_proposed_version`; `handshake_initiator_rejects_on_no_overlap` |
| **CI** | `ci/ci_check_handshake_closure.sh` |

#### `CN-SESS-03` — _enforced_

| Aspect | Location |
|--------|----------|
| **Source** | docs/planning/phase4-n-l-wire-protocol-invariants.md §5 |
| **Requirement** | docs/planning/phase4-n-l-wire-protocol-invariants.md §5 *(paraphrased from source/code_locus; registry `statement` empty)* |
| **Code** | crates/ade_network/src/session/core.rs |
| **Tests** | `session_step_two_runs_byte_identical`; `session_handshake_completion_transitions_state`; `session_outbound_frame_encodes_via_encode_frame` |
| **CI** | `ci/ci_check_session_core_closure.sh` |

#### `CN-SESS-04` — _enforced_

| Aspect | Location |
|--------|----------|
| **Source** | docs/clusters/PHASE4-N-M-FRAG/cluster.md §1 |
| **Requirement** | docs/clusters/PHASE4-N-M-FRAG/cluster.md §1 *(paraphrased from source/code_locus; registry `statement` empty)* |
| **Code** | crates/ade_network/src/session/state.rs (ProtoBuffers + ConnectedState.proto_buffers field), crates/ade_network/src/session/core.rs (drain_connected_frames + drain_protocol_items) |
| **Tests** | `fragmented_chain_sync_message_assembles_one_deliver`; `fragmented_block_fetch_block_assembles_one_deliver`; `interleaved_chain_sync_and_block_fetch_fragments_stay_isolated`; `pipelined_two_chain_sync_messages_in_one_mux_frame_emit_two_delivers`; `truncated_then_complete_two_step_drain`; `proto_buffers_isolation_across_accepted_protocols` |
| **CI** | `ci/ci_check_session_proto_reassembly.sh` |

#### `CN-SESS-05` — _enforced_

| Aspect | Location |
|--------|----------|
| **Source** | docs/clusters/PHASE4-N-AB/cluster.md §1; project_pre_rolive_hardening_queue.md item 2 |
| **Requirement** | docs/clusters/PHASE4-N-AB/cluster.md §1; project_pre_rolive_hardening_queue.md item 2 *(paraphrased from source/code_locus; registry `statement` empty)* |
| **Code** | crates/ade_network/src/session/core.rs (handle_outbound owns segmentation: splits MAX_PAYLOAD < len <= MAX_OUTBOUND_PAYLOAD_BYTES into ordered <=MAX_PAYLOAD frames via encode_inner_frame, reusing one captured timestamp; encode_inner_frame stays the single-frame encoder authority with its MAX_PAYLOAD guard; MAX_OUTBOUND_PAYLOAD_BYTES fixed constant) |
| **Tests** | `outbound_payload_at_max_payload_is_one_frame`; `outbound_payload_over_max_payload_segments_into_two`; `outbound_segment_order_preserved`; `outbound_segments_keep_same_mini_protocol_id_and_mode`; `outbound_large_payload_reassembles_byte_identical_via_inbound`; `outbound_payload_at_upper_bound_is_allowed`; `outbound_payload_over_upper_bound_fails_closed` |
| **CI** | `ci/ci_check_outbound_segmentation.sh` |

### CN-SEED

#### `CN-SEED-01` — _enforced_

| Aspect | Location |
|--------|----------|
| **Source** | docs/planning/phase4-n-m-ledger-seed-invariants.md §1 (I-A1) |
| **Requirement** | docs/planning/phase4-n-m-ledger-seed-invariants.md §1 (I-A1) *(paraphrased from source/code_locus; registry `statement` empty)* |
| **Code** | crates/ade_runtime/src/seed_import/importer.rs, crates/ade_runtime/src/seed_import/json.rs |
| **Tests** | `utxo_seed_parses_minimal_two_entry_fixture`; `utxo_seed_two_imports_byte_identical`; `utxo_seed_btree_order_independent_of_json_order`; `utxo_seed_rejects_unparseable_json`; `utxo_seed_rejects_bad_txin_key`; `utxo_seed_rejects_bad_address`; `utxo_seed_inline_datum_entry_round_trips`; `utxo_seed_canonical_txout_address_extracted`; `utxo_seed_accepts_plutus_v1_reference_script`; `utxo_seed_accepts_plutus_v2_reference_script`; … (+9 more) |
| **CI** | `ci/ci_check_seed_import_closure.sh`; `ci/ci_check_seed_import_full_preprod_support.sh` |

### CN-ANCHOR

#### `CN-ANCHOR-01` — _enforced_

| Aspect | Location |
|--------|----------|
| **Source** | docs/planning/phase4-n-m-ledger-seed-invariants.md §1 (I-A3) |
| **Requirement** | docs/planning/phase4-n-m-ledger-seed-invariants.md §1 (I-A3) *(paraphrased from source/code_locus; registry `statement` empty)* |
| **Code** | crates/ade_runtime/src/bootstrap_anchor.rs, crates/ade_ledger/src/bootstrap_anchor/anchor.rs |
| **Tests** | `mint_composes_inputs_byte_identically`; `mint_then_round_trip_via_canonical_cbor`; `mint_carries_seed_point_correctly`; `mint_propagates_utxo_fingerprint_into_anchor`; `bootstrap_anchor_match_is_exhaustive` |
| **CI** | `ci/ci_check_bootstrap_anchor_closure.sh` |

### CN-WAL

#### `CN-WAL-01` — _enforced_

| Aspect | Location |
|--------|----------|
| **Source** | docs/planning/phase4-n-m-ledger-seed-invariants.md §1 (I-A4) |
| **Requirement** | docs/planning/phase4-n-m-ledger-seed-invariants.md §1 (I-A4) *(paraphrased from source/code_locus; registry `statement` empty)* |
| **Code** | crates/ade_ledger/src/wal/store_trait.rs, crates/ade_runtime/src/wal/file_wal_store.rs |
| **Tests** | `file_wal_store_append_then_read_all_round_trips`; `file_wal_store_reopens_existing_directory_and_preserves_entries`; `file_wal_store_rotates_at_max_bytes_when_forced` |
| **CI** | `ci/ci_check_wal_append_only.sh` |

### CN-ADMIT

#### `CN-ADMIT-01` — _enforced_

| Aspect | Location |
|--------|----------|
| **Source** | docs/planning/phase4-n-m-b-admission-invariants.md §1 (I-B5) |
| **Requirement** | docs/planning/phase4-n-m-b-admission-invariants.md §1 (I-B5) *(paraphrased from source/code_locus; registry `statement` empty)* |
| **Code** | crates/ade_node/src/admission/runner.rs::run_admission + crates/ade_node/src/admission/bootstrap.rs::dispatch_admission |
| **Tests** | `run_admission_emits_shutdown_on_signal`; `run_admission_emits_shutdown_on_channel_close`; `run_admission_disconnect_to_zero_peers_clean_exit` |
| **CI** | `ci/ci_check_admission_runner_closure.sh` |

#### `CN-ADMIT-02` — _enforced_

| Aspect | Location |
|--------|----------|
| **Source** | docs/planning/phase4-n-m-b-admission-invariants.md §1 (I-B6) |
| **Requirement** | docs/planning/phase4-n-m-b-admission-invariants.md §1 (I-B6) *(paraphrased from source/code_locus; registry `statement` empty)* |
| **Code** | crates/ade_node/src/admission/seed_to_snapshot.rs::seed_to_snapshot |
| **Tests** | `seed_to_snapshot_writes_via_persistent_cache`; `seed_to_snapshot_returns_initial_ledger_fingerprint`; `seed_to_snapshot_two_runs_byte_identical`; `seed_to_snapshot_propagates_pre_conway_encode_error_as_authority_fatal` |
| **CI** | `ci/ci_check_admission_no_refscript_skip.sh` |

### CN-PUMP

#### `CN-PUMP-01` — _enforced_

| Aspect | Location |
|--------|----------|
| **Source** | docs/planning/phase4-n-m-c-operator-pass-invariants.md §1 (I-C5) |
| **Requirement** | docs/planning/phase4-n-m-c-operator-pass-invariants.md §1 (I-C5) *(paraphrased from source/code_locus; registry `statement` empty)* |
| **Code** | crates/ade_runtime/src/admission/wire_pump.rs (run_admission_wire_pump sole authority + closed AdmissionPeerEvent / AdmissionWirePumpError sums + extract_chain_sync_header_point header-point extractor) |
| **Tests** | `pump_emits_tip_update_and_request_next_on_intersect_found_no_block_fetch`; `pump_emits_tip_update_on_intersect_not_found`; `rollforward_drives_block_fetch_then_request_next`; `extract_chain_sync_header_point_returns_slot_and_hash`; `extract_chain_sync_header_point_rejects_malformed_envelope` |
| **CI** | `ci/ci_check_admission_wire_pump_closure.sh` |

### CN-PROD

#### `CN-PROD-01` — _enforced_

| Aspect | Location |
|--------|----------|
| **Source** | docs/planning/phase4-n-q-invariants.md §1 (I1, I6); §2 (N1, N15) |
| **Requirement** | Producer-mode listener completes the N2N handshake (CN-SESS-02) on every accepted inbound connection before any mini-protocol traffic is exchanged. Pre-handshake socket bytes never reach the n2n_server reducers; handshake failure fail-closes the connection. Bytes from a peer that has not completed handshake are dropped at the boundary. |
| **Code** | crates/ade_runtime/src/network/n2n_listener.rs (RED listener); crates/ade_runtime/src/orchestrator/n2n_server_pump.rs (per-peer dispatch into n2n_server reducers) |
| **Tests** | `n2n_listener_loopback_handshake_succeeds` |
| **CI** | `ci/ci_check_n2n_server_no_signing_dep.sh` |

#### `CN-PROD-02` — _enforced_

| Aspect | Location |
|--------|----------|
| **Source** | docs/planning/phase4-n-q-invariants.md §1 (I3, I4, I7, I8); §2 (N4, N5, N8, N9); §5 |
| **Requirement** | Producer slot loop never signs a block whose KES period has rotated past current_period. Slot → KES period is a pure function of (slot, genesis_kes_anchor, slots_per_kes_period); the coordinator fail-closes (slot_missed event + log) when wall-clock has advanced past the target slot before forge completes. No retroactive forge. The GREEN coordinator never owns or stores private signing material — it emits a closed `RequestForge { slot, kes_period, ledger_snapshot_ref, chain_tip }` effect; the RED producer shell either returns an `AcceptedBlock` via a `ForgeSucceeded` event or a structured `ForgeFailed { slot, structured_error }` event. KesSecret / VrfSigningKey / ColdSigningKey never enter CoordinatorState. T-tier key-custody boundary. |
| **Code** | crates/ade_runtime/src/producer/coordinator.rs (GREEN — CoordinatorState has no secret fields; type system prevents); crates/ade_runtime/src/producer/producer_shell.rs (RED — sole key-custody surface; ProducerShell::init enforces KES period bounds vs opcert); crates/ade_node/src/produce_mode.rs (RED driver; PHASE4-N-R-A A3 wired the real run_real_forge composition; stub replaced); crates/ade_runtime/src/producer/opcert_envelope.rs (PHASE4-N-R-C C1 opcert parser); crates/ade_runtime/src/producer/genesis_parser.rs (PHASE4-N-R-C C2 genesis parser) |
| **Tests** | `init_emits_started_event_and_zero_other_effects`; `slot_tick_emits_request_forge_and_log`; `forge_succeeded_emits_broadcast_and_log`; `forge_not_leader_emits_log_and_clears_pending`; `forge_failed_emits_slot_missed_with_mapped_reason`; `stale_forge_result_after_new_tick_drops_with_slot_missed`; `kes_period_out_of_range_errors`; `shell_init_rejects_malformed_opcert_hot_vkey_length`; `shell_init_rejects_kes_period_below_opcert_start`; `shell_kes_sign_at_current_period_succeeds_and_verifies`; … (+6 more) |
| **CI** | `ci/ci_check_producer_coordinator_no_secrets.sh`; `ci/ci_check_produce_mode_uses_bootstrap_initial_state.sh` |

#### `CN-PROD-03` — _enforced_

| Aspect | Location |
|--------|----------|
| **Source** | docs/planning/phase4-n-t-invariants.md §1 (A1, A2, A6); docs/clusters/PHASE4-N-T/cluster.md §1 |
| **Requirement** | produce_mode's forge base state is derived from bootstrap_initial_state (cold-start, fed the operator-seeded ledger from --json-seed + --consensus-inputs) plus the bundle-projected PoolDistrView, epoch nonce (eta0), and absolute slot from the bootstrap tip. SyntheticForgeInputs / build_synthetic_forge_context are deleted; no zero-stake / LedgerState::new / constant-prev-hash forge base remains. The sole path to produce_mode's initial state is the single bootstrap_initial_state authority (no parallel synthetic path). Cold-start branch only; warm-start recovery is Problem 2, deferred to N-U. |
| **Code** | crates/ade_node/src/produce_mode.rs (bootstrap cold-start wiring; SyntheticForgeInputs deleted); crates/ade_runtime/src/producer/chain_evolution.rs (derive_forge_context); crates/ade_node/src/cli.rs (ProduceCli --json-seed + --consensus-inputs) |
| **Tests** | `produce_cli_requires_seed_and_consensus_inputs`; `produce_mode_bootstrap_cold_start_seeds_real_ledger` |
| **CI** | `ci/ci_check_produce_mode_uses_bootstrap_initial_state.sh` |

#### `CN-PROD-04` — _enforced_

| Aspect | Location |
|--------|----------|
| **Source** | docs/planning/phase4-n-t-invariants.md §1 (A4, A5); docs/clusters/PHASE4-N-T/cluster.md §1 |
| **Requirement** | Every CoordinatorEffect::BroadcastBlock reconstructs the AcceptedBlock from artifact.bytes through the BLUE self_accept authority against the pre-forge base, then admits it to the served ServedChainSnapshot via the single ServedChainHandle::push_atomic authority before the next slot tick. If the self_accept replay rejects, push_atomic is NOT called and the loop emits structured BroadcastPushError::SelfAcceptReplayRejected. ProducerLogEvent::BlockServed is emitted only for blocks present in the served snapshot. No silently-dropped (no-op) broadcast; only self-accepted forged blocks are served. |
| **Code** | crates/ade_node/src/produce_mode.rs (BroadcastBlock arm -> push_atomic; BroadcastPushError); crates/ade_runtime/src/producer/served_chain_handle.rs (push_atomic, reused); crates/ade_ledger/src/producer/self_accept.rs (token reconstruction, reused) |
| **Tests** | `broadcast_pushes_self_accepted_block_to_served`; `broadcast_rejects_non_self_accepted_block`; `forge_to_served_block_fetch_roundtrip` |
| **CI** | _(no CI listed in registry — enforced via Code/Tests; gap: bind a CI gate)_ |

### CN-FORGE

#### `CN-FORGE-01` — _enforced_

| Aspect | Location |
|--------|----------|
| **Source** | docs/planning/phase4-n-r-invariants.md §1 (I1, I9); §2 (N1); §5 (state transitions) |
| **Requirement** | The producer-mode forge handler is a closed transition from CoordinatorEvent::RequestForge { slot, kes_period, ledger_snapshot_ref, chain_tip } to exactly one of three ForgeResult variants: ForgeSucceeded { slot, artifact } where artifact.bytes decodes via Ade's BLUE block decoder AND self_accept(artifact, chain_tip, ledger_snapshot) returns Accepted; ForgeNotLeader { slot, vrf_output_fingerprint }; or ForgeFailed { slot, structured_error }. No other outcome is permitted. ForgeSucceeded MUST NOT be emitted if self_accept rejects the artifact — the handler emits ForgeFailed { SelfAcceptRejected } instead. Empty-block forging is the explicit scope; mempool integration is out of scope for the rule's enforcement evidence. |
| **Code** | crates/ade_node/src/produce_mode.rs (run_real_forge BLUE-then-RED-then-BLUE pipeline; apply_effects_with_forge_handler call site); crates/ade_ledger/src/producer/forge.rs (forge_block BLUE step 5); crates/ade_ledger/src/producer/self_accept.rs (self_accept BLUE step 6 gate) |
| **Tests** | `zero_stake_answer_emits_forge_not_leader`; `kes_period_outside_window_emits_forge_failed_kes_period_mismatch`; `full_stake_answer_reaches_self_accept_and_rejects`; `run_real_forge_is_byte_identical_across_two_runs`; `forge_block_accepts_empty_mempool`; `forge_block_empty_mempool_produces_empty_body`; `produce_mode_starts_runs_three_slots_and_exits_via_max_slots`; `forge_to_self_accept_succeeds` |
| **CI** | `ci/ci_check_producer_coordinator_no_secrets.sh`; `ci/ci_check_no_independent_forge_codepath.sh` |

#### `CN-FORGE-02` — _enforced_

| Aspect | Location |
|--------|----------|
| **Source** | docs/planning/phase4-n-r-invariants.md §1 (I2); §2 (N12); §3 (D1); §4 (R3) |
| **Requirement** | Leader-check splits across the RED/BLUE color boundary: RED produces a VRF proof/output for the slot using the operator's VRF signing key; BLUE verifies the proof and evaluates leader eligibility from canonical inputs only (slot, eta0, stake_distribution, leader_threshold, vrf_vk, vrf_proof_or_output, LeaderScheduleAnswer). BLUE never sees the VRF / KES / cold signing keys. The BLUE evaluator (`verify_and_evaluate_leader`) lives at `ade_core::consensus::leader_check` and has no dependency on LedgerView, EraSchedule, ChainDepState, wall-clock, storage, or RED crates. Caller derives LeaderScheduleAnswer via the authority path (query_leader_schedule) and passes it in. The closed two-variant LeaderCheckVerdict (Eligible carries forge-capable material; NotEligible carries only bounded vrf_output_fingerprint evidence) makes illegal observation of forge-capable material structurally impossible. |
| **Code** | crates/ade_core/src/consensus/leader_check.rs (verify_and_evaluate_leader + LeaderCheckVerdict + LeaderCheckError); crates/ade_node/src/produce_mode.rs (run_real_forge composition — RED vrf_prove → BLUE verify_and_evaluate_leader → RED kes_sign_at) |
| **Tests** | `eligible_on_threshold_with_high_stake_emits_eligible_verdict`; `not_eligible_with_zero_stake_emits_not_eligible_verdict`; `malformed_proof_emits_verification_failed`; `wrong_vk_emits_verification_failed`; `answer_slot_mismatch_emits_structured_error`; `vrf_input_mismatch_emits_structured_error`; `zero_stake_denominator_emits_structured_error`; `verdict_is_byte_identical_across_two_runs`; `vrf_output_fingerprint_is_first_8_bytes_of_output`; `zero_stake_answer_emits_forge_not_leader` |
| **CI** | `ci/ci_check_leader_check_authority.sh` |

#### `CN-FORGE-03` — _enforced_

| Aspect | Location |
|--------|----------|
| **Source** | docs/planning/phase4-n-v-invariants.md; docs/clusters/PHASE4-N-V/cluster.md §1 |
| **Requirement** | Producer/validator codec symmetry: forge_block emits the era-tagged [era, block] envelope (era = Conway discriminant 7) via the single canonical ade_codec::encode_block_envelope (symmetric to decode_block_envelope), so forge_block output round-trips through the SAME decode_block authority that validates received blocks. decode_block(forge_block(tick).bytes) is Ok and yields a DecodedBlock whose header-body fields and four preserved body-bucket bytes equal what was forged. A bare-block (no-envelope) forge output, or any forge<->decode asymmetry, is CI-gated impossible. Root cause (PHASE4-N-T): forge_block emitted a bare array(5) block, so decode_block_envelope rejected EVERY forged block at offset 0 (BlockValidityError::Body(Decoding(InvalidStructure))) before any header/KES/leader/self_accept logic. |
| **Code** | crates/ade_codec/src/cbor/envelope.rs (encode_block_envelope, NEW); crates/ade_ledger/src/producer/forge.rs (forge_block wraps output via the encoder) |
| **Tests** | `encode_decode_block_envelope_round_trips`; `conway_envelope_head_is_82_07`; `encode_block_envelope_reencodes_corpus_block_identically`; `forge_block_output_decodes_via_decode_block` |
| **CI** | `ci/ci_check_forge_decode_round_trip.sh` |

#### `CN-FORGE-04` — _enforced_

| Aspect | Location |
|--------|----------|
| **Source** | docs/planning/phase4-n-w-praos-vrf-migration.md; docs/clusters/completed/PHASE4-N-V/CLOSURE.md |
| **Requirement** | Producer-side Praos VRF construction must match the Conway/Praos validator authority: the leader VRF proof alpha, the leader-schedule evidence, the LeaderScheduleAnswer.expected_vrf_input contract, and the self_accept header verification must all use ONE era-correct Praos construction. For Conway/Praos the producer alpha MUST equal the validator alpha (praos_vrf_input(slot, eta0) = blake2b256(slot\|\|eta0) + vrfLeaderValue range-extension), NOT the TPraos role-tagged alpha (slot\|\|eta0\|\|0x4C). No verification/construction fallback may accept both TPraos and Praos VRF inputs — for a given era/protocol version there is exactly one VRF transcript authority. |
| **Code** | crates/ade_core/src/consensus/vrf_cert.rs (ExpectedVrfInput + leader_vrf_input single authority + leader_value_for); crates/ade_node/src/produce_mode.rs (run_real_forge proves over the answer's alpha_bytes); crates/ade_core/src/consensus/leader_check.rs (verify_and_evaluate_leader era arg + era-correct threshold); crates/ade_core/src/consensus/leader_schedule.rs (query_leader_schedule builds via leader_vrf_input; LeaderScheduleAnswer.expected_vrf_input: ExpectedVrfInput); validator verify_praos_vrf / praos_vrf_input |
| **Tests** | `forge_to_self_accept_succeeds`; `praos_call_with_tpraos_answer_emits_vrf_input_mismatch`; `tpraos_producer_forge_fails_closed_with_unsupported_era`; `is_praos_only_babbage_and_conway`; `query_uses_state_epoch_nonce_for_vrf_input` |
| **CI** | `ci/ci_check_producer_praos_vrf.sh` |

### CN-SNAPSHOT

#### `CN-SNAPSHOT-01` — _declared_

| Aspect | Location |
|--------|----------|
| **Source** | docs/planning/phase4-n-r-invariants.md §1 (I4); §2 (N11); §3 (D6) |
| **Requirement** | A forged block becomes visible to peers only AFTER ServedChainHandle::push_atomic succeeds. The push_atomic call covers the full served_chain_admit call inside a watch::Sender::send_modify closure — no observer can read a torn snapshot mid-insertion. Coordinator emits BroadcastBlock; RED effect handler calls push_atomic; only on Ok(ServedTip) may per-peer reducers serve the block. Fail-closed shutdown on PushError. |
| **Code** | TBD — populated by N-R-B B2 slice (ade_runtime::producer::served_chain_handle::ServedChainHandle::push_atomic) + B3 slice (produce_mode BroadcastBlock arm wiring) |
| **Tests** | `handle_construction_yields_empty_snapshot`; `view_subscribe_creates_independent_receiver`; `served_tip_is_closed_value_type` |
| **CI** | _(no CI gate — gap)_ |

#### `CN-SNAPSHOT-02` — _enforced_

| Aspect | Location |
|--------|----------|
| **Source** | docs/planning/phase4-n-r-invariants.md §2 (N4); N-R-A A1 OQ8 audit |
| **Requirement** | A RequestRange covering a slot range that is not entirely present in ServedChainSnapshot MUST return NoBlocks per the Cardano block-fetch protocol's failure semantics — no partial ad-hoc response, no silent truncation, no serving of a strict prefix. Both endpoints + every block between MUST be present in the snapshot for the server to issue StartBatch + Block* + BatchDone. |
| **Code** | crates/ade_network/src/block_fetch/server.rs (producer_block_fetch_serve — endpoint-presence check via first_key/last_key comparison against requested range) |
| **Tests** | `n_r_b_partial_overlap_from_endpoint_not_in_snapshot_yields_no_blocks`; `n_r_b_partial_overlap_to_endpoint_not_in_snapshot_yields_no_blocks`; `n_r_b_partial_overlap_both_endpoints_fabricated_yields_no_blocks`; `producer_block_fetch_serve_request_range_with_origin_endpoint_yields_no_blocks`; `producer_block_fetch_serve_request_range_empty_in_chain_yields_no_blocks`; `producer_block_fetch_serve_request_range_outside_chain_yields_no_blocks` |
| **CI** | _(no CI listed in registry — enforced via Code/Tests; gap: bind a CI gate)_ |

### CN-OPCERT

#### `CN-OPCERT-01` — _enforced_

| Aspect | Location |
|--------|----------|
| **Source** | docs/planning/phase4-n-r-invariants.md §1 (I6); §2 (N6); §3 (D4); N-R-A A1 OQ4 fixtures |
| **Requirement** | The opcert envelope parser accepts a real cardano-cli `node.opcert` text envelope (closed type check `NodeOperationalCertificate` + CBOR array(2) shape locked by N-R-A A1 OQ4 fixtures against cardano-cli 11.0.0.0 / cardano-node 11.0.1). Element 0 is an array(4) of [hot_vkey(bytes(32)), sequence_number(uint), kes_period(uint), sigma(bytes(64))] mapping to the canonical `OperationalCert`. Element 1 is bytes(32) cold_vk. Any shape mismatch (wrong type field, malformed cborHex, wrong outer/inner arity, wrong field types or lengths) fail-closes with a structured `OpCertParseError` variant. No `String` payloads in load-bearing error variants. |
| **Code** | crates/ade_runtime/src/producer/opcert_envelope.rs (parse_opcert_envelope + closed OpCertParseError enum) |
| **Tests** | `accepted_envelope_decodes_to_expected_opcert`; `malformed_type_envelope_emits_wrong_envelope_type`; `malformed_cbor_hex_envelope_emits_malformed_cbor_hex`; `wrong_arity_envelope_emits_malformed_cbor`; `parser_is_byte_identical_across_two_runs` |
| **CI** | `ci/ci_check_node_forge_real_cli_ingress.sh` |

### CN-GENESIS

#### `CN-GENESIS-01` — _enforced_

| Aspect | Location |
|--------|----------|
| **Source** | docs/planning/phase4-n-r-invariants.md §1 (I7); §2 (N7); §3 (D5); N-R-A A1 OQ7 fixtures |
| **Requirement** | The Shelley genesis closed-contract parser accepts a real cardano-cli `shelley-genesis.json` and produces a canonical `GenesisAnchor`. Required fields (networkMagic, systemStart, slotLength, slotsPerKESPeriod, maxKESEvolutions) fail-closed on missing / malformed / wrong-type input. No implicit defaults (e.g., 'if missing, assume preprod' rejected). No stringly fallback (e.g., `"1"` rejected for u32 fields). Extra unknown keys accepted-and-ignored for forward compatibility, iff they do not alter interpretation — the `GenesisAnchor` produced from an extra-key fixture MUST byte-equal the canonical fixture's `GenesisAnchor`. The kes_anchor_slot is operator-supplied (not in genesis) and passed to the parser as a separate argument. systemStart parsing uses a deterministic ISO 8601 → Unix epoch milliseconds conversion (Howard Hinnant proleptic Gregorian). |
| **Code** | crates/ade_runtime/src/producer/genesis_parser.rs (parse_shelley_genesis + closed GenesisParseError enum + parse_iso8601_to_unix_ms + days_since_unix_epoch) |
| **Tests** | `accepted_shelley_genesis_parses_to_expected_anchor`; `missing_required_field_emits_structured_error`; `stringly_int_emits_malformed_field_type`; `extra_inert_keys_produce_byte_identical_anchor`; `malformed_numeric_negative_slot_length_rejected`; `iso8601_parse_anchors_to_known_unix_ms_values` |
| **CI** | `ci/ci_check_node_forge_real_cli_ingress.sh`; `ci/ci_check_genesis_consistency_fixture_present.sh` |

### CN-KES

#### `CN-KES-HEADER-01` — _enforced_

| Aspect | Location |
|--------|----------|
| **Source** | docs/planning/phase4-n-s-invariants.md §1 (I1, I2); §2 (N1, N2) |
| **Requirement** | The KES signature in a forged block's header is over the canonical unsigned-header CBOR pre-image — the CBOR encoding of ShelleyHeaderBody (the first element of the outer [header_body, kes_signature] header array). The producer-side recipe (unsigned_header_pre_image) and the validator-side extractor (header_input::decode_block.header_input.kes.header_body_bytes) produce byte-identical output for every corpus block. The branded UnsignedHeaderPreImage(Vec<u8>) type's only constructor is the canonical recipe; kes_sign_header accepts only this type — arbitrary-byte signing is mechanically unrepresentable. |
| **Code** | crates/ade_ledger/src/block_validity/unsigned_header_pre_image.rs (UnsignedHeaderPreImage branded type + canonical recipe); crates/ade_runtime/src/producer/producer_shell.rs (ProducerShell::kes_sign_header accepts only the branded type); crates/ade_node/src/produce_mode.rs (run_real_forge two-pass bridge replaces the placeholder) |
| **Tests** | `unsigned_header_preimage_matches_decode_block_extraction_for_corpus`; `recipe_output_is_byte_identical_across_two_runs`; `shell_kes_sign_header_produces_verifiable_signature` |
| **CI** | `ci/ci_check_unsigned_header_preimage_single_source.sh` |

### CN-PREIMAGE

#### `CN-PREIMAGE-FIXTURE-01` — _enforced_

| Aspect | Location |
|--------|----------|
| **Source** | docs/planning/phase4-n-s-invariants.md §7 OQ-S-A; N-S-A A1 fixture metadata |
| **Requirement** | For every block in ade_testkit::validity::corpus::ConwayValidityCorpus, Ade's unsigned_header_pre_image(...) (with inputs derived from decode_block(block_bytes).header_input) produces output byte-identical to decode_block(block_bytes).header_input.kes.unwrap().header_body_bytes. This cross-impl byte-match test is the load-bearing proof that the producer's pre-image recipe matches the validator's authority — without it the 'single source of truth' claim is unverified. |
| **Code** | crates/ade_ledger/src/block_validity/unsigned_header_pre_image.rs (unsigned_header_preimage_matches_decode_block_extraction_for_corpus test) |
| **Tests** | `unsigned_header_preimage_matches_decode_block_extraction_for_corpus` |
| **CI** | _(no CI listed in registry — enforced via Code/Tests; gap: bind a CI gate)_ |

### CN-OUTBOUND

#### `CN-OUTBOUND-RELAY-01` — _enforced_

| Aspect | Location |
|--------|----------|
| **Source** | docs/planning/phase4-n-s-invariants.md §1 (I4); §2 (N4, N8); §3 (D3) |
| **Requirement** | OutboundCommand is the sole channel between produce_mode and MuxPump's outbound encoder. The closed enum carries typed ChainSyncServerMsg / BlockFetchServerMsg variants — no Vec<u8> byte tunnel; no direct MuxTransportHandle::outbound write from produce_mode. MuxPump's session-aware encoder is the only producer of wire-byte streams. |
| **Code** | crates/ade_runtime/src/network/outbound_command.rs (OutboundCommand closed enum); crates/ade_runtime/src/network/mux_pump.rs (MuxPump::outbound_relay field + handle_outbound_command + dispatch_outbound_frame); crates/ade_node/src/produce_mode.rs (dispatch_server_frame_event_to_outbound enqueues typed ServerReply via OutboundCommand) |
| **Tests** | `outbound_command_peer_accessor_returns_target_peer`; `outbound_command_carries_typed_reply_not_raw_bytes` |
| **CI** | `ci/ci_check_no_produce_mode_direct_transport_writes.sh` |

### CN-PEER

#### `CN-PEER-OUTBOUND-MAP-01` — _enforced_

| Aspect | Location |
|--------|----------|
| **Source** | docs/planning/phase4-n-s-invariants.md §1 (I5); §2 (N5) |
| **Requirement** | Per-peer outbound senders are owned by an Arc<RwLock<BTreeMap<PeerId, mpsc::Sender<OutboundCommand>>>>. Listener (run_per_peer_session) inserts on PeerConnected; MuxPump removes on emit_peer_disconnected. produce_mode looks up by PeerId and cannot fabricate senders. BTreeMap (not HashMap) for deterministic iteration order. Lookup failure is structured: DispatchError::{UnknownPeer, PeerOutboundMissing}. No cross-peer byte leakage is structurally possible — bytes destined for PeerId(a) reach the MuxPump task owning PeerId(a)'s TCP socket, never another peer's. |
| **Code** | crates/ade_runtime/src/network/outbound_command.rs (PerPeerOutbound type alias + new_per_peer_outbound constructor); crates/ade_runtime/src/network/n2n_listener.rs (run_per_peer_session inserts sender on PeerConnected); crates/ade_node/src/produce_mode.rs (DispatchError closed enum + dispatch_server_frame_event_to_outbound) |
| **Tests** | `outbound_command_peer_accessor_returns_target_peer` |
| **CI** | _(no CI listed in registry — enforced via Code/Tests; gap: bind a CI gate)_ |

### CN-OPERATOR

#### `CN-OPERATOR-EVIDENCE-01` — _enforced_

| Aspect | Location |
|--------|----------|
| **Source** | docs/planning/phase4-n-s-invariants.md §1 (I6); §2 (N6, N7); user-direction correction 6 |
| **Requirement** | Every PHASE4-N-S-C operator-pass evidence manifest (docs/clusters/PHASE4-N-S-C/CE-N-S-LIVE_YYYYMMDD-<short_commit>.toml) carries the closed schema: schema_version, ade_commit, cardano_node_version, cardano_cli_version, network, block_hash, slot, opcert_fingerprint, genesis_fingerprint, ade_evidence_file, peer_log_file, peer_log_capture_command, peer_log_filter, peer_log_file_sha256, acceptance_keyword_match. The peer_log_file_sha256 cross-checks the committed peer.log file's actual hash. grep filter is documentation, not authority — the committed peer_log_file is the raw docker logs output. |
| **Code** | docs/clusters/PHASE4-N-S-C/cluster.md + S1.md + S2.md (runbook + manifest schema); ci/ci_check_operator_evidence_manifest_schema.sh (schema enforcement when a manifest is committed); PHASE4-N-F-G-C: docs/evidence/phase4-n-f-g-c-operator-pass-README.md (the --mode node operator-pass runbook) + ci/ci_check_ba02_evidence_manifest_schema.sh (the BA-02 manifest schema + sha256 cross-check, vacuous-until-committed — same manifest+sha256 discipline extended to the node-spine BA-02 evidence path) |
| **Tests** | _(no tests listed in registry — enforced via Code/CI; gap: name the proving test)_ |
| **CI** | `ci/ci_check_operator_evidence_manifest_schema.sh`; `ci/ci_check_ba02_evidence_manifest_schema.sh` |

### CN-MITHRIL

#### `CN-MITHRIL-01` — _enforced_

| Aspect | Location |
|--------|----------|
| **Source** | docs/clusters/PHASE4-N-Y/S1-mithril-import-authority.md; S7-real-mithril-binding.md |
| **Requirement** | docs/clusters/PHASE4-N-Y/S1-mithril-import-authority.md; S7-real-mithril-binding.md *(paraphrased from source/code_locus; registry `statement` empty)* |
| **Code** | crates/ade_ledger/src/bootstrap_anchor/binding.rs (verify_mithril_binding + MithrilManifestReport + closed MithrilImportError); crates/ade_runtime/src/mithril_import/ (RED manifest importer); crates/ade_runtime/src/mithril_bootstrap.rs (PHASE4-N-Z production composition — verify-before-bootstrap, fail-closed) |
| **Tests** | `mithril_binding_rejects_certified_point_other_than_seed_point`; `mithril_anchor_rejects_field_mismatch`; `mithril_import_fail_closed_blocks_storage_init`; `mithril_bootstrap_verifies_before_storage_init`; `mithril_bootstrap_fails_closed_on_seed_point_mismatch` |
| **CI** | `ci/ci_check_mithril_uses_bootstrap_initial_state.sh`; `ci/ci_check_mithril_seed_point_independence.sh` |

### CN-CINPUT

#### `CN-CINPUT-01` — _enforced_

| Aspect | Location |
|--------|----------|
| **Source** | docs/clusters/PHASE4-N-F-A/cluster.md; A1-seed-epoch-consensus-inputs-type.md |
| **Requirement** | docs/clusters/PHASE4-N-F-A/cluster.md; A1-seed-epoch-consensus-inputs-type.md *(paraphrased from source/code_locus; registry `statement` empty)* |
| **Code** | crates/ade_ledger/src/seed_consensus_inputs.rs (SeedEpochConsensusInputs + encode_/decode_seed_epoch_consensus_inputs + SeedConsensusInputsError) |
| **Tests** | `seed_epoch_consensus_inputs_round_trips_byte_identical`; `seed_cinput_decode_rejects_unknown_version`; `seed_cinput_decode_rejects_noncanonical_or_duplicate_keys` |
| **CI** | _(no CI listed in registry — enforced via Code/Tests; gap: bind a CI gate)_ |

#### `CN-CINPUT-02` — _enforced_

| Aspect | Location |
|--------|----------|
| **Source** | docs/clusters/PHASE4-N-F-A/cluster.md; A2-bootstrap-population-containment.md; A3a-wal-provenance-entry.md |
| **Requirement** | docs/clusters/PHASE4-N-F-A/cluster.md; A2-bootstrap-population-containment.md; A3a-wal-provenance-entry.md *(paraphrased from source/code_locus; registry `statement` empty)* |
| **Code** | crates/ade_runtime/src/seed_epoch_lineage.rs (single shared populator authority); crates/ade_runtime/src/genesis_bootstrap.rs; crates/ade_runtime/src/mithril_bootstrap.rs; crates/ade_node/src/admission/bootstrap.rs (admission pre-seed caller); crates/ade_runtime/src/seed_consensus_merge.rs; crates/ade_runtime/src/seed_consensus_provenance.rs; ci/ci_check_consensus_input_provenance.sh |
| **Tests** | `bootstrap_persists_anchor_keyed_seed_consensus_inputs`; `bootstrap_seed_inputs_merge_fails_closed_on_missing_vrf_or_stake`; `snapshot_store_keyed_sidecar_is_disjoint_from_slot_snapshots`; `persist_writes_anchor_keyed_sidecar_and_recoverable_wal_provenance` |
| **CI** | `ci/ci_check_consensus_input_provenance.sh` |

#### `CN-CINPUT-03` — _enforced_

| Aspect | Location |
|--------|----------|
| **Source** | docs/clusters/PHASE4-N-F-C/cluster.md; L5-produce-from-recovered-state.md |
| **Requirement** | docs/clusters/PHASE4-N-F-C/cluster.md; L5-produce-from-recovered-state.md *(paraphrased from source/code_locus; registry `statement` empty)* |
| **Code** | crates/ade_node/src/node_sync.rs (forge_one_from_recovered — projects leadership only from the recovered BootstrapState); ci/ci_check_consensus_input_provenance.sh (guard (d) consume-side fence) |
| **Tests** | `forge_from_recovered_uses_recovered_pool_distr`; `forge_from_recovered_fails_closed_without_recovered_inputs` |
| **CI** | `ci/ci_check_consensus_input_provenance.sh` |

### CN-REHEARSAL

#### `CN-REHEARSAL-FIDELITY-01` — _enforced_

| Aspect | Location |
|--------|----------|
| **Source** | docs/planning/phase4-n-f-g-d-invariants.md |
| **Requirement** | docs/planning/phase4-n-f-g-d-invariants.md *(paraphrased from source/code_locus; registry `statement` empty)* |
| **Code** | crates/ade_node/src/ba02_evidence.rs (correlate — reused, the sole acceptance-evidence constructor); crates/ade_node/src/ba02_pass.rs (RED evidence I/O — the rehearsal write-wrapper mirrors/extends this into the rehearsal home); crates/ade_runtime/src/consensus_inputs/ (import_live_consensus_inputs — the shared N-M-C extraction/import the path-fidelity clause pins; OQ1 slice-entry proof obligation: it must consume an early/private-net extraction through the SAME path used for a synced preprod tip); docs/evidence/phase4-n-f-g-c-operator-pass-README.md (the preprod operator-pass runbook the C1 dry-run runbook must be a strict subset of);… |
| **Tests** | `node_accepted_block_consensus_inputs_via_shared_import`; `rehearsal_envelope_wraps_correlate_produced_payload`; `rehearsal_correlate_no_evidence_writes_nothing`; `rehearsal_envelope_is_structurally_distinct_from_ba02_manifest`; `c1_dry_run_correlate_to_rehearsal_envelope`; `node_c1_dry_run_rehearsal_live`; `rehearsal_gate_fails_on_archived_home_leak`; `genesis_rehearsal_manifest_binds_block_zero_genesis`; `genesis_rehearsal_no_evidence_writes_nothing`; `node_c1_genesis_rehearsal_live` |
| **CI** | `ci/ci_check_node_path_fidelity.sh`; `ci/ci_check_rehearsal_manifest_schema.sh`; `ci/ci_check_genesis_successor_reachability.sh` |

### CN-FOLLOW

#### `CN-FOLLOW-01` — _enforced_

| Aspect | Location |
|--------|----------|
| **Source** | docs/clusters/PRODUCER-PARTICIPANT-FOLLOW/PRODUCER-PARTICIPANT-FOLLOW.md; docs/clusters/PRODUCER-PARTICIPANT-FOLLOW/CN-FOLLOW-01-participant-forge-on-ao-selected-head.md |
| **Requirement** | docs/clusters/PRODUCER-PARTICIPANT-FOLLOW/PRODUCER-PARTICIPANT-FOLLOW.md; docs/clusters/PRODUCER-PARTICIPANT-FOLLOW/CN-FOLLOW-01-participant-forge-on-ao-selected-head.md *(paraphrased from source/code_locus; registry `statement` empty)* |
| **Code** | crates/ade_node/src/node_sync.rs (participant_forge_decision GREEN decision + ParticipantForgeDecision/ParticipantForgeFenceReason + ForgeMode::ParticipantExtendOnSelectedHead + participant_forge_mode_on_caughtup/after_admit transitions); crates/ade_node/src/node_lifecycle.rs (run_relay_loop_with_sched ForgeTick: VenueRole::Participant routes to participant_forge_decision on the AO-selected durable ChainDb::tip fenced by DC-NODE-28; VenueRole::Unknown keeps the pure DC-NODE-15 path unchanged; the forge-base evidence emits ForgeBaseSource::LocalChaindbTip + cert_path_present:false for Participant too). The AO selection law (select_best_chain, DC-CONS-03 / CN-CONS-01) is CONSUMED, never… |
| **Tests** | `participant_venue_forges_on_ao_selected_head_when_leader`; `participant_forge_base_is_ao_selected_chaindb_tip`; `participant_forge_base_is_servable_before_forge`; `participant_forge_refused_while_fork_choice_pending`; `participant_venue_requires_forge_activation`; `single_producer_forge_decision_unchanged`; `orphaned_startup_holds_forge_fence_participant`; `participant_forge_two_runs_byte_identical` |
| **CI** | `ci/ci_check_participant_forge_on_selected_head.sh` |

---

## DC — Derived Cardano-Compatibility Invariants (Project Constitution §3)

_239 rules._

### DC-CBOR

#### `DC-CBOR-01` — _declared_

| Aspect | Location |
|--------|----------|
| **Source** | Project constitution §3, T-ENC-01, T-ENC-03 |
| **Requirement** | Cardano CBOR decode/encode round-trips to identical bytes for all era types |
| **Code** | _(no enforcement found — gap)_ |
| **Tests** | _(no tests listed — gap)_ |
| **CI** | _(no CI gate — gap)_ |

#### `DC-CBOR-02` — _declared_

| Aspect | Location |
|--------|----------|
| **Source** | Project constitution §3, T-ENC-01, T-DET-01 |
| **Requirement** | Original wire bytes preserved for hash computation on hash-critical paths (see Byte Authority Model) |
| **Code** | _(no enforcement found — gap)_ |
| **Tests** | _(no tests listed — gap)_ |
| **CI** | _(no CI gate — gap)_ |

### DC-CRYPTO

#### `DC-CRYPTO-01` — _enforced_

| Aspect | Location |
|--------|----------|
| **Source** | Project constitution §3, T-CORE-01, T-DET-01 |
| **Requirement** | Crypto verification is pure and matches Haskell node on all test vectors |
| **Code** | crates/ade_crypto/src/, crates/ade_core/src/consensus/vrf_cert.rs (Praos single-VRF input + leader/nonce range extension; PHASE4-B1-S5), crates/ade_core/src/consensus/kes_check.rs (KES + op-cert wiring; PHASE4-B1-S5) |
| **Tests** | `blake2b_256_empty`; `blake2b_256_abc`; `blake2b_256_single_byte`; `blake2b_256_multi_block`; `blake2b_256_large`; `libsodium_vector_empty_message`; `libsodium_vector_single_byte`; `libsodium_vector_two_byte`; `libsodium_vector_longer_message`; `libsodium_vector_hash_size_message`; … (+9 more) |
| **CI** | `ci/ci_check_crypto_vectors.sh` |

#### `DC-CRYPTO-02` — _enforced_

| Aspect | Location |
|--------|----------|
| **Source** | Project constitution §3, T-BOUND-01 |
| **Requirement** | All signing operations confined to shell |
| **Code** | crates/ade_crypto/src/lib.rs |
| **Tests** | _(no tests listed in registry — enforced via Code/CI; gap: name the proving test)_ |
| **CI** | `ci/ci_check_no_signing_in_blue.sh` |

#### `DC-CRYPTO-03` — _enforced_

| Aspect | Location |
|--------|----------|
| **Source** | docs/planning/phase4-n-c-invariants.md §1 (NC-VRF-1/2/3); IETF draft-irtf-cfrg-vrf-03 (Praos VRF) |
| **Requirement** | VRF signing transcript equivalence and verification symmetry. For canonical inputs (slot, epoch_nonce, vrf_signing_key, vrf_role) the RED signer produces a VrfProof byte-identical to cardano-node's reference output, and the emitted VrfProof verifies under ade_crypto::vrf::verify_praos_vrf with the matching verification key. Private-key execution is RED-shell confined; BLUE consumes the VrfProof as a captured signed artifact. |
| **Code** | crates/ade_runtime/src/producer/signing.rs (vrf_prove) |
| **Tests** | `vrf_prove_matches_reference_vectors`; `vrf_prove_then_verify_round_trip` |
| **CI** | `ci/ci_check_private_key_custody.sh` |

#### `DC-CRYPTO-04` — _enforced_

| Aspect | Location |
|--------|----------|
| **Source** | docs/planning/phase4-n-c-invariants.md §1 (NC-KES-1); docs/planning/phase4-n-p-invariants.md; Cardano Sum6KES specification (depth-6 sum composition over ed25519) |
| **Requirement** | KES signing transcript equivalence and verification symmetry. For canonical inputs (kes_secret, period, msg) the RED signer produces a KesSignature byte-identical to Haskell cardano-base's Sum6KES reference and verifying under ade_crypto::kes::verify_kes. After PHASE4-N-P S5 the algorithm is BLUE-owned (ade_crypto::kes_sum::Sum6Kes); cross-impl agreement with the Haskell reference is mechanically validated against a cardano-cli ground-truth corpus (DC-CRYPTO-08). Private-key execution is RED-shell confined; BLUE consumes the KesSignature as a captured signed artifact. |
| **Code** | crates/ade_runtime/src/producer/signing.rs (kes_sign via BLUE Sum6Kes); crates/ade_crypto/src/kes.rs (KesSignature, verify_kes_signature via BLUE Sum6Kes); crates/ade_crypto/src/kes_sum/ (the BLUE algorithm itself) |
| **Tests** | `kes_sign_matches_reference_vectors`; `kes_sign_then_verify_round_trip`; `kes_signature_from_bytes_round_trips`; `verify_kes_signature_agrees_with_existing_verify_kes`; `cardano_cli_corpus_sign_then_upstream_verifies` |
| **CI** | `ci/ci_check_private_key_custody.sh`; `ci/ci_check_kes_sum_compatibility.sh` |

#### `DC-CRYPTO-05` — _enforced_

| Aspect | Location |
|--------|----------|
| **Source** | docs/planning/phase4-n-c-invariants.md §1 (NC-KES-2); docs/planning/phase4-n-p-invariants.md §1 (I6); Cardano Sum6KES specification |
| **Requirement** | KES evolution discipline: evolve(k_i) -> k_{i+1} is one-way. The evolved key signs period i+1 and MUST NOT sign for period i. RED kes_sign is forbidden when the requested period > current_period + evolutions_remaining; kes_evolve is forbidden when to < from or to > from + evolutions_remaining. Forward secrecy is a RED-shell discipline; BLUE has no recovery path if RED violates it. After PHASE4-N-P S5 the underlying algorithm (`ade_crypto::kes_sum::Sum6Kes::update_kes`) is BLUE-owned; per-field zeroize on Drop of consumed sub-seeds is implemented via `ZeroizingSeed` (DC-CRYPTO-08). |
| **Code** | crates/ade_runtime/src/producer/signing.rs (kes_update, kes_sign); crates/ade_crypto/src/kes_sum/sum.rs (SumKes::update_kes + ZeroizingSeed Drop) |
| **Tests** | `kes_update_chain_matches_reference`; `kes_sign_rejects_period_past_evolutions_remaining`; `kes_update_rejects_backwards_evolution`; `zeroizing_seed_drop_overwrites_bytes` |
| **CI** | `ci/ci_check_private_key_custody.sh`; `ci/ci_check_kes_sum_compatibility.sh` |

#### `DC-CRYPTO-06` — _enforced_

| Aspect | Location |
|--------|----------|
| **Source** | docs/clusters/PHASE4-N-O/S1.md §1; docs/active/op-ops-04-ade-native-kes-flow.md (operator spec verbatim) |
| **Requirement** | Ade-native KES envelope is the sole accepted hot-signing-key envelope format. Closed grammar `ade.kes.seed.v1`: load-bearing fields {`format`, `role`, `crypto`, `seed_32`, `period_idx`, `format_version`} are validated with `#[serde(deny_unknown_fields)]`; optional metadata {`genesis_hash`, `network_magic`, `created_at_slot`, `created_by`} is ignored but does not break load. The loader returns closed `AdeKesEnvelopeError` variants for every unsupported shape (UnknownEnvelopeFormat, WrongKeyRole, UnsupportedCryptoTag, MissingSeed32, MalformedSeed32, MalformedPeriodIdx, PeriodIdxOutOfRange, UnsupportedFormatVersion, MalformedJson). No fallback parser. No heuristic guess. Private-key bytes never appear in any error/log surface. |
| **Code** | crates/ade_runtime/src/producer/ade_kes_envelope.rs (parse, serialize, AdeKesEnvelopeError); crates/ade_runtime/src/producer/keys.rs (load_ade_kes_signing_key, write_ade_kes_envelope, KeyLoadError::AdeEnvelope) |
| **Tests** | `parse_round_trips_serialize`; `parse_round_trips_at_nonzero_period`; `parse_rejects_unknown_format`; `parse_rejects_wrong_role`; `parse_rejects_unsupported_crypto`; `parse_rejects_unsupported_format_version`; `parse_rejects_missing_seed_32`; `parse_rejects_malformed_seed_32_length`; `parse_rejects_uppercase_seed_hex`; `parse_rejects_period_idx_overflow`; … (+6 more) |
| **CI** | `ci/ci_check_kes_envelope_closed.sh` |

#### `DC-CRYPTO-07` — _enforced_

| Aspect | Location |
|--------|----------|
| **Source** | docs/clusters/PHASE4-N-O/S1.md §1; docs/clusters/PHASE4-N-P/S5.md; docs/active/op-ops-04-ade-native-kes-flow.md |
| **Requirement** | cardano-cli's `KesSigningKey_ed25519_kes_2^6` envelope (the upstream `Sum6KES` expanded-tree serialization, 608 bytes for a fresh key) is loadable via the Ade-owned BLUE deserializer (`ade_crypto::kes_sum::Sum6Kes::raw_deserialize_signing_key_kes`). After PHASE4-N-P S5, structurally-valid 608-byte payloads round-trip into a `KesSecret`; any other payload shape — wrong size (32, 612, anything ≠ 608), malformed sub-tree, inconsistent vk hash, leaf-all-zero, period > 63 tree shape — fail-closes via `KeyLoadError::UnsupportedExpandedKesKeyFormat` (size mismatch) or `KeyLoadError::KesParse(KesParseError::*)` (structural defect). No fallback parser; the deserializer IS the structural validator. |
| **Code** | crates/ade_runtime/src/producer/keys.rs (load_kes_signing_key_skey routes 608-byte payloads through ade_crypto::kes_sum::Sum6Kes::raw_deserialize_signing_key_kes; wrong-size payloads return UnsupportedExpandedKesKeyFormat) |
| **Tests** | `cardano_cli_kes_envelope_rejects_32_byte_payload`; `cardano_cli_kes_envelope_rejects_synthetic_608_byte_payload`; `cardano_cli_kes_envelope_accepts_real_608_byte_payload`; `cardano_cli_kes_envelope_rejects_612_byte_payload`; `cardano_cli_kes_envelope_rejects_608_byte_leaf_zero_payload`; `cardano_cli_corpus_skey_deserializes_and_vk_matches_ground_truth` |
| **CI** | `ci/ci_check_kes_envelope_closed.sh`; `ci/ci_check_kes_sum_compatibility.sh` |

#### `DC-CRYPTO-08` — _enforced_

| Aspect | Location |
|--------|----------|
| **Source** | docs/clusters/PHASE4-N-P/cluster.md; docs/planning/phase4-n-p-invariants.md §1 (I1, I2, I4, I6); §2 (N1, N9); docs/clusters/PHASE4-N-P/S4.md (cardano-cli ground-truth + prefix-divergence discovery) |
| **Requirement** | Ade-owned Sum6KES algorithm is Haskell-equivalent. `ade_crypto::kes_sum::Sum6Kes` is byte-identical to Haskell `cardano-base`'s `Sum6KES Ed25519DSIGN`: `derive_verification_key`, `gen_key_kes_from_seed_bytes`, `update_kes` (chain across all 64 periods), and `sign_kes` produce the same bytes as the Haskell reference for every (seed, period, msg) triple. Cross-impl validation against the cardano-cli ground-truth corpus is mechanically enforced under `#[cfg(test)]` only (3 throwaway 608-byte SKEY + VKEY pairs captured from `cardano-cli 11.0.0.0`; deserializing the SKEY through our impl produces the captured VK byte-for-byte). Note: `cardano-crypto` Rust 1.0.8 uses different `expand_seed` prefix bytes (0x00/0x01 vs Haskell's 0x01/0x02) — Ade matches Haskell, NOT cardano-crypto Rust; this divergence is asserted explicitly in `sum6_kes_seed_expansion_diverges_from_cardano_crypto_rust_1_0_8`. After PHASE4-N-P S5, `KesSecret.inner` is the Ade-owned signing key; `cardano-crypto` is a `#[cfg(test)]` oracle only. No compatibility shim may construct an upstream `SumSigningKey` through unsafe layout assumptions, transmute, vendored pub(crate) access, or fork-only constructors (N9), enforced by `ci/ci_check_kes_sum_compatibility.sh` Guard 3. |
| **Code** | crates/ade_crypto/src/kes_sum/{mod,single,sum,hash,errors,period}.rs; crates/ade_crypto/src/kes_sum/cardano_cli_corpus.rs (#[cfg(test)] ground-truth corpus); crates/ade_crypto/src/kes_sum/tests.rs (35 unit tests) |
| **Tests** | `sum0_kes_signs_and_verifies_at_period_0`; `sum0_kes_rejects_period_1`; `sum0_kes_update_expires_after_period_0`; `sum0_kes_verify_rejects_wrong_message`; `sum1_kes_signs_at_period_0_and_period_1`; `sum6_kes_total_periods_is_64`; `sum6_kes_sizes_match_recurrence`; `sum6_kes_chain_advances_through_all_64_periods`; `sum6_kes_update_after_period_63_expires`; `sum6_kes_sign_rejects_period_64`; … (+8 more) |
| **CI** | `ci/ci_check_kes_sum_compatibility.sh`; `ci/ci_check_private_key_custody.sh` |

#### `DC-CRYPTO-09` — _enforced_

| Aspect | Location |
|--------|----------|
| **Source** | docs/clusters/PHASE4-N-P/cluster.md; docs/planning/phase4-n-p-invariants.md §1 (I3, I5); §2 (N2, N3, N4, N5); docs/clusters/PHASE4-N-P/period-from-zeroed-sum6-tree-shape-proof.md |
| **Requirement** | Sum6KES expanded signing-key serde and period inference. `raw_serialize_signing_key_kes` / `raw_deserialize_signing_key_kes` are byte-identical to Haskell's `rawSerialiseSignKeyKES` / `rawDeserialiseSignKeyKES` for `Sum6KES Ed25519DSIGN`. The on-disk format is exactly 608 bytes; any other payload size fails closed via `KesParseError::WrongPayloadSize`. `current_period` is uniquely inferable from which sub-seeds are zeroed in the tree (no heuristic; exactly one valid period or a closed parse error) — implemented per the proof obligation at `docs/clusters/PHASE4-N-P/period-from-zeroed-sum6-tree-shape-proof.md`. Round-trip preserves period; serialize → deserialize → serialize yields byte-identical output for every period 0..=63. Malformed sub-trees (truncated child skey, wrong VK length at any recursion level, inconsistent vk0/vk1 hashes, leaf-all-zero) → closed `KesParseError` variant; no best-effort guesswork. |
| **Code** | crates/ade_crypto/src/kes_sum/period.rs (period_from_zeroed_sum6_tree_shape); crates/ade_crypto/src/kes_sum/sum.rs (raw_serialize/raw_deserialize_signing_key_kes); crates/ade_crypto/src/kes_sum/single.rs (Sum0 leaf serde); crates/ade_crypto/src/kes_sum/errors.rs (KesParseError closed surface) |
| **Tests** | `sum6_raw_serialize_signing_key_kes_size_is_608`; `sum6_raw_serialize_signature_kes_size_is_448`; `sum6_skey_round_trip_at_every_period_0_to_63`; `sum6_signature_round_trip_at_every_period`; `period_from_zeroed_sum6_tree_shape_agrees_with_update_kes_chain`; `period_from_zeroed_sum6_tree_shape_rejects_leaf_all_zero`; `raw_deserialize_signing_key_kes_rejects_wrong_payload_size`; `raw_deserialize_signing_key_kes_rejects_leaf_all_zero`; `raw_deserialize_signing_key_kes_rejects_inconsistent_vk_left_at_level_6`; `raw_deserialize_signing_key_kes_rejects_inconsistent_vk_right_at_level_6`; … (+5 more) |
| **CI** | `ci/ci_check_kes_sum_compatibility.sh` |

#### `DC-CRYPTO-10` — _enforced_

| Aspect | Location |
|--------|----------|
| **Source** | docs/clusters/PHASE4-N-AC/cluster.md; docs/evidence/c1-genesis-rehearsal-reproduction-README.md (item-4 C1 re-run finding) |
| **Requirement** | docs/evidence/c1-genesis-rehearsal-reproduction-README.md (item-4 C1 re-run finding) *(paraphrased from source/code_locus; registry `statement` empty)* |
| **Code** | crates/ade_runtime/src/producer/producer_shell.rs (kes_sign_header_advancing = kes_advance_to(period) then kes_sign_header; kes_advance_to -> kes_update fail-closed: EvolutionBackwards / EvolutionExhausted); crates/ade_node/src/produce_mode.rs (the forge's single real KES sign uses kes_sign_header_advancing); crates/ade_runtime/src/producer/coordinator.rs (kes_period_for_slot returns the opcert-anchored RELATIVE evolution, not the absolute period; consumed by node_lifecycle.rs ForgeTick) |
| **Tests** | `shell_kes_sign_header_advancing_evolves_then_signs`; `shell_kes_sign_header_advancing_at_current_period_signs`; `shell_kes_sign_header_advancing_backwards_fails_closed`; `shell_kes_sign_header_advancing_beyond_lifetime_fails_closed`; `kes_period_for_slot_anchors_relative_to_opcert_start_period` |
| **CI** | `ci/ci_check_kes_evolution_before_sign.sh` |

### DC-LEDGER

#### `DC-LEDGER-01` — _enforced_

| Aspect | Location |
|--------|----------|
| **Source** | Project constitution §3, T-CORE-01, T-CORE-03 |
| **Requirement** | apply_block(state, block) is pure and deterministic |
| **Code** | crates/ade_ledger/src/rules.rs |
| **Tests** | `apply_block_byron_ebb_passes_through`; `apply_block_deterministic`; `all_eras_determinism_summary` |
| **CI** | `ci/ci_check_ledger_determinism.sh` |

#### `DC-LEDGER-02` — _partial_

| Aspect | Location |
|--------|----------|
| **Source** | Project constitution §3, T-DET-01 |
| **Requirement** | Same genesis + same blocks = byte-identical ledger state |
| **Code** | crates/ade_ledger/src/state.rs, crates/ade_ledger/src/fingerprint.rs, crates/ade_testkit/src/validity/adversarial.rs (GREEN no-false-accept adversarial corpus: no mutation of a real block ever yields Valid through block_validity; PHASE4-B1-S7); crates/ade_testkit/src/tx_validity/adversarial.rs (GREEN no-false-accept tx adversarial corpus: deterministic witness/value/input mutations replay byte-identically through tx_validity, no mutation ever yields Valid; PHASE4-B2-S4) |
| **Tests** | `utxo_state_deterministic`; `all_eras_determinism_summary`; `boundary_fingerprint_matches_pins`; `no_mutation_is_ever_valid`; `adversarial_replays_identically` |
| **CI** | `ci/ci_check_ledger_determinism.sh`; `ci/ci_check_differential_divergence.sh` |

#### `DC-LEDGER-03` — _partial_

| Aspect | Location |
|--------|----------|
| **Source** | Project constitution §3, T-DET-01, T-ERR-01 |
| **Requirement** | Tx/block validity agrees with Haskell node on all tested inputs |
| **Code** | crates/ade_ledger/src/byron.rs, crates/ade_ledger/src/rules.rs, crates/ade_ledger/src/plutus_eval.rs |
| **Tests** | `check_duplicate_inputs_catches_dupes`; `resolve_inputs_missing_input`; `missing_witnesses_rejected`; `all_eras_replay_summary`; `all_plutus_boundaries_aggregate_zero_rejections`; `plutus_era_contiguous_smoke`; `under_declared_ex_units_must_reject`; `failing_validator_must_reject`; `extraneous_redeemer_must_reject` |
| **CI** | `ci/ci_check_differential_divergence.sh`; `ci/ci_check_plutus_budget_cap.sh` |

#### `DC-LEDGER-04` — _partial_

| Aspect | Location |
|--------|----------|
| **Source** | Project constitution §3, T-DET-01 |
| **Requirement** | Epoch boundary computations (stake snapshots, rewards) match Haskell |
| **Code** | crates/ade_ledger/src/epoch.rs, crates/ade_ledger/src/rules.rs |
| **Tests** | `precise_boundary_comparison_eta_diagnosis`; `alonzo_epoch_boundary_end_to_end`; `regular_epoch_boundary_comparison`; `conway_epoch_boundary_end_to_end` |
| **CI** | _(no CI gate — gap)_ |

#### `DC-LEDGER-05` — _partial_

| Aspect | Location |
|--------|----------|
| **Source** | Project constitution §3, T-DET-01 |
| **Requirement** | Witness binding is era-specific: Byron TxWitness, Shelley+ WitsVKey/Scripts/BootstrapWitnesses, Alonzo+ Redeemers/Datums, Conway governance witnesses |
| **Code** | crates/ade_ledger/src/witness.rs, crates/ade_ledger/src/scripts.rs, crates/ade_plutus/src/evaluator.rs, crates/ade_ledger/src/tx_validity/witness.rs (Conway vkey-witness binding: fail-closed Ed25519 coverage over preserved body hash; PHASE4-B2-S1) |
| **Tests** | `witness_info_no_plutus`; `witness_info_plutus_detection`; `empty_witness_set`; `aiken_fixture_tx_evaluates_end_to_end`; `all_required_covered_is_valid`; `signature_over_wrong_body_rejected`; `witness_correct_key_wrong_body_rejected`; `wrong_size_signature_rejected`; `wrong_size_vkey_rejected` |
| **CI** | `ci/ci_check_required_signer_closure.sh` |

#### `DC-LEDGER-06` — _declared_

| Aspect | Location |
|--------|----------|
| **Source** | Project constitution §3, T-PLATFORM-01, T-DET-01 |
| **Requirement** | Script context (ScriptContext/TxInfo) derived from tx + ledger state + network-wide constants (EpochInfo, SystemStart); no host-environment data |
| **Code** | _(no enforcement found — gap)_ |
| **Tests** | _(no tests listed — gap)_ |
| **CI** | _(no CI gate — gap)_ |

#### `DC-LEDGER-07` — _declared_

| Aspect | Location |
|--------|----------|
| **Source** | Project constitution §3, T-DET-01, T-BUILD-02 |
| **Requirement** | Coexisting supported versions must return same validity verdict for consensus-relevant inputs |
| **Code** | _(no enforcement found — gap)_ |
| **Tests** | _(no tests listed — gap)_ |
| **CI** | _(no CI gate — gap)_ |

#### `DC-LEDGER-08` — _enforced_

| Aspect | Location |
|--------|----------|
| **Source** | Cardano ledger spec (Conway CDDL certificate tags 0..18; delegation/pool state transitions); IDD fail-fast + closed-surface doctrine |
| **Requirement** | Conway cert-state accumulation is a closed, total, era-versioned transition: for each block at track_utxo, certificates decode through the era-correct closed grammar (Conway via the completed single decode_conway_certs retaining all owner payloads, tags 0..18) selected by explicit era dispatch — never the Shelley 6-variant decoder on Conway bytes, never reduced into the 7-variant Shelley Certificate, never with payload fields dropped. Every certificate resolves to an owner-tagged disposition: it mutates B4-owned CertState (delegation/pool), or it is owner-tagged to ConwayGovState and routed out-of-mutation-scope (observed, not swallowed, not applied), or it is a structured reject (NotValidInEra for removed tags 5/6, Malformed for bad CBOR, UnsupportedUntilStateOwner only for genuinely-ownerless cases — unreachable on the real corpus). Composite certs (tags 10/12/13) carry both a B4-owned mutation and an owner-tagged governance effect; both are represented. No certificate is flattened to neutral because there is nowhere to put it (the owner exists), decode-dropped, or apply-swallowed; a decode or apply error propagates as a structured LedgerError and halts the block transition. Incomplete or best-effort accumulation is a forbidden fail-open. (Wiring the owner-tagged ConwayGovState effects into applied governance state is PHASE4-B5, not B4.) |
| **Code** | crates/ade_types/src/conway/cert.rs (owner-complete ConwayCert); crates/ade_types/src/shelley/cert.rs (PoolRegistrationCert.owners); crates/ade_codec/src/conway/cert.rs (decode_conway_certs payload retention + decode_drep); crates/ade_codec/src/shelley/cert.rs (shared read_pool_registration_cert) — PHASE4-B4-S1 decoder-completeness clause; crates/ade_ledger/src/delegation.rs (apply_conway_cert + ConwayCertAction/ConwayCertOutcome owner-tagged apply model, total over 18 tags) — PHASE4-B4-S2 apply-totality clause |
| **Tests** | `each_tag_retains_owner_payloads`; `drep_grammar_total`; `conway_cert_action_total`; `apply_outcome_agrees_with_action`; `removed_tag_rejects_as_era_invalid`; `drep_registration_is_owner_tagged_not_applied`; `era_dispatch_conway_accumulates_via_conway_path`; `era_dispatch_shelley_accumulates_via_shelley_path`; `conway_decode_error_is_fail_closed`; `conway_unknown_tag_is_fail_closed`; … (+6 more) |
| **CI** | `ci/ci_check_forbidden_patterns.sh` |

#### `DC-LEDGER-09` — _enforced_

| Aspect | Location |
|--------|----------|
| **Source** | Cardano ledger spec (Conway CDDL gov cert tags 9..18; CONWAY CERTS/GOVCERT/COMMITTEE transitions; CIP-1694); IDD fail-fast + closed-surface doctrine |
| **Requirement** | Cardano ledger spec (Conway CDDL gov cert tags 9..18; CONWAY CERTS/GOVCERT/COMMITTEE transitions; CIP-1694); IDD fail-fast + closed-surface doctrine *(paraphrased from source/code_locus; registry `statement` empty)* |
| **Code** | crates/ade_ledger/src/gov_cert.rs (apply_conway_gov_cert: native gov dispatch over ConwayCert, total over 18 tags) — PHASE4-B5-S2; crates/ade_ledger/src/state.rs (GovCertEnv + LedgerState::gov_cert_env() fail-fast) + crates/ade_ledger/src/pparams.rs (ConwayOnlyDepositParams.drep_activity) + crates/ade_ledger/src/error.rs (ValidationEnvironmentError::MissingDRepActivityParam) — PHASE4-B5-S1; crates/ade_ledger/src/rules.rs (accumulate_tx_certs / process_block_certificates thread Option<ConwayGovState>, apply the gov half, carry gov_state forward through apply_block — replaces the B4 observe-and-drop) — PHASE4-B5-S3; crates/ade_ledger/src/fingerprint.rs (gov-state + drep_activity… |
| **Tests** | `gov_apply_total_over_18_tags`; `composite_gov_half_applied_once_certstate_untouched_by_b5`; `drep_expiry_uses_epoch_plus_activity`; `env_free_gov_certs_need_no_env`; `drep_register_missing_env_is_fail_fast`; `drep_expiry_overflow_is_fail_closed`; `gov_apply_is_deterministic`; `gov_cert_env_present_ok`; `gov_cert_env_missing_drep_activity_is_fail_fast`; `gov_accumulation_applies_drep_registration_into_gov_state`; … (+7 more) |
| **CI** | `ci/ci_check_gov_cert_accumulation_closed.sh` |

#### `DC-LEDGER-10` — _enforced_

| Aspect | Location |
|--------|----------|
| **Source** | Cardano ledger spec (Credential = key\|script across UTXOW/DELEG/GOVCERT; CIP-1694); IDD illegal-states-unrepresentable + determinism doctrine |
| **Requirement** | Cardano ledger spec (Credential = key\|script across UTXOW/DELEG/GOVCERT; CIP-1694); IDD illegal-states-unrepresentable + determinism doctrine *(paraphrased from source/code_locus; registry `statement` empty)* |
| **Code** | crates/ade_types/src/shelley/cert.rs (StakeCredential enum {KeyHash,ScriptHash} + hash()) — OQ5-S1; crates/ade_codec/src/shelley/cert.rs + crates/ade_codec/src/conway/cert.rs (decode_stake_credential preserves the tag, rejects unknown) — OQ5-S1; crates/ade_ledger/src/state.rs (ConwayGovState re-keyed to StakeCredential), gov_cert.rs, governance.rs, cert_classify.rs, rules.rs (cred.hash() boundary adapter for the Hash28-keyed stake snapshot) — OQ5-S1; crates/ade_ledger/src/fingerprint.rs (write_stake_credential emits discriminant+hash; gov-map writers use it; stake-snapshot writer stays write_hash28) — OQ5-S1; crates/ade_testkit/src/harness/snapshot_loader.rs (GREEN: gov-map + DRep-reg… |
| **Tests** | `shelley_credential_preserves_discriminant`; `conway_credential_preserves_discriminant`; `unknown_credential_tag_rejects`; `discriminant_changes_fingerprint`; `keyhash_scripthash_same_bytes_are_distinct_certstate`; `keyhash_scripthash_same_bytes_are_distinct_govstate`; `discriminant_changes_fingerprint_corpus`; `credential_accumulation_replays_byte_identical`; `committee_keyhash_scripthash_do_not_cross_resolve`; `committee_keyhash_scripthash_same_bytes_distinct`; … (+10 more) |
| **CI** | `ci/ci_check_credential_discriminant_closed.sh` |

#### `DC-LEDGER-11` — _enforced_

| Aspect | Location |
|--------|----------|
| **Source** | CIP-1694 (proposal_procedure = [coin, reward_account, gov_action, anchor]); Project constitution §3 (closed semantic surfaces, T-CORE-01); DC-LEDGER-10 (downstream credential discriminant must not be re-collapsed) |
| **Requirement** | proposal_procedures MUST NOT remain an opaque byte field in the authoritative Conway tx-body shape. ConwayTxBody.proposal_procedures is Option<Vec<ProposalProcedure>>, decoded through a single closed entry point (decode_proposal_procedures) that rejects unknown gov_action tags, structural failures, empty sets, and trailing garbage deterministically; the typed form re-encodes byte-identically (PreservedCbor) for every well-formed Conway tx body. The decoder reuses the existing closed GovAction enum (preserving DC-LEDGER-10 UpdateCommittee discriminant) and the existing opaque Anchor struct. |
| **Code** | crates/ade_codec/src/conway/governance.rs (decode_proposal_procedures, decode_proposal_procedure, decode_gov_action, encode_proposal_procedures); crates/ade_types/src/conway/governance.rs (ProposalProcedure); crates/ade_codec/src/conway/tx.rs (typed key 20 path); crates/ade_testkit/src/governance/proposal_procedures_replay.rs (PP-S2 canonical synthetic corpus + replay harness) |
| **Tests** | `roundtrip_info_action_proposal`; `roundtrip_hard_fork_initiation`; `roundtrip_no_confidence`; `roundtrip_treasury_withdrawals`; `roundtrip_parameter_change`; `roundtrip_new_constitution`; `roundtrip_update_committee`; `roundtrip_multi_procedure`; `rejects_unknown_gov_action_tag`; `rejects_empty_proposal_procedures_set`; … (+11 more) |
| **CI** | `ci/ci_check_proposal_procedures_closed.sh` |

#### `DC-LEDGER-12` — _enforced_

| Aspect | Location |
|--------|----------|
| **Source** | docs/planning/phase4-n-c-invariants.md §1 (NC-FORGE-4); PHASE4-N-E mempool admit closure |
| **Requirement** | Every tx in a forged block is admissible via ade_ledger::mempool::admit against the base ledger state, in the snapshot's canonical accumulating order. No tx in a forged block bypasses mempool validation. Forge MUST NOT permute, fabricate, or skip the snapshot's canonical accumulating order. |
| **Code** | crates/ade_ledger/src/producer/forge.rs (tx-admissibility prefix gate); crates/ade_ledger/src/mempool/admit.rs (admit — reused for prefix check) |
| **Tests** | `forge_block_rejects_tx_not_in_mempool_accepted_prefix`; `forge_block_rejects_tx_permuted_from_accumulating_order`; `forge_block_empty_mempool_produces_empty_body`; `admit_prefix_property_documented` |
| **CI** | `ci/ci_check_forge_purity.sh` |

#### `DC-LEDGER-PARAMS-01` — _enforced_

| Aspect | Location |
|--------|----------|
| **Source** | docs/clusters/MITHRIL-VERIFIED-ANCHOR-INTEGRATION/SLICE-S1a-native-nonutxo-decoder.md; user directive 2026-06-23 (two S1a refinements, neither deferrable behind a flag: (1) bind network_id from the manifest network magic -- mainnet -> 1, testnet -> 0 -- onto every authority-bearing field (the operator-supplied reward-account nibble is diagnostic evidence only, never a verdict -- the manifest magic is the sole network authority); (2) preserve Conway coinsPerUTxOByte faithfully as… |
| **Requirement** | user directive 2026-06-23 (two S1a refinements, neither deferrable behind a flag: (1) bind network_id from the manifest network magic -- mainnet -> 1, testnet -> 0 -- onto every authority-bearing field (the operator-supplied… *(paraphrased from source/code_locus; registry `statement` empty)* |
| **Code** | crates/ade_ledger/src/pparams.rs: MinUtxoRule (LegacyAbsoluteMin/PerByte + coin()) + ProtocolParameters.min_utxo_rule (replaces min_utxo_value) + Default/apply_update (LegacyAbsoluteMin). crates/ade_ledger/src/shelley.rs + mary.rs: the min-UTxO check matches MinUtxoRule -- LegacyAbsoluteMin keeps the absolute check, PerByte -> UnsupportedConwayMinUtxoRule. crates/ade_ledger/src/error.rs: LedgerError::UnsupportedConwayMinUtxoRule + UnsupportedConwayMinUtxoRuleError + Display arm. crates/ade_ledger/src/phase.rs: classify_failure_phase routes UnsupportedConwayMinUtxoRule to Phase1. crates/ade_ledger/src/ledgerdb_state.rs: network_id_from_magic +… |
| **Tests** | `network_id_derived_from_manifest_magic`; `reward_nibble_disagreement_is_diagnostic_not_terminal`; `conway_pparams_decode_yields_per_byte_min_utxo_rule`; `mary_min_utxo_per_byte_rule_is_terminal_not_permissive`; `mary_min_utxo_legacy_absolute_min_unchanged`; `commitment_binds_every_field`; `decode_native_nonutxo_real_snapshot` |
| **CI** | `ci/ci_check_native_nonutxo_decode.sh` |

#### `DC-LEDGER-VALUE-01` — _enforced_

| Aspect | Location |
|--------|----------|
| **Source** | docs/clusters/LEDGER-VALUE-CORRECTNESS/SLICE-1-output-asset-quantity-u64.md; the DC-MITHRIL-05 downstream release-blocker (Ade's i64 MultiAsset model could not safely validate real Cardano outputs with quantities > i64::MAX); user directive 2026-06-23 (the authoritative value model is widened to the non-negative Word64 domain via a distinct OutputAssetQuantity(u64) newtype -- NOT a universal i128, NOT a u64->checked-i64->reject adapter, NOT a truncating cast; checked output arithmetic with a… |
| **Requirement** | the DC-MITHRIL-05 downstream release-blocker (Ade's i64 MultiAsset model could not safely validate real Cardano outputs with quantities > i64::MAX); user directive 2026-06-23 (the authoritative value model is widened to the non-negative… *(paraphrased from source/code_locus; registry `statement` empty)* |
| **Code** | crates/ade_types/src/mary/value.rs: OutputAssetQuantity(u64) (ZERO/checked_add/checked_sub/is_zero) + the dormant MintBurnQuantity(i64) + MultiAsset over OutputAssetQuantity. crates/ade_ledger/src/value.rs: MultiAsset (imports the ade_types newtype), multi_asset_add (checked_add), multi_asset_sub/value_sub (checked_sub -> AssetUnderflow), prune_zeros (canonical zero normalization). crates/ade_ledger/src/error.rs: LedgerError::AssetUnderflow + AssetUnderflowError. crates/ade_ledger/src/phase.rs: classify_failure_phase routes AssetUnderflow to Phase1. crates/ade_ledger/src/mary.rs: the type-impossible negative-output scan removed; parse_mint_field documents the MintBurnQuantity boundary.… |
| **Tests** | `multi_asset_word64_add_sub_round_trips_above_i64_max`; `multi_asset_sub_underflow_returns_asset_underflow`; `multi_asset_add_overflow_returns_error`; `negative_output_quantity_is_unrepresentable`; `utxo_state_word64_multi_asset_quantity_round_trips`; `utxo_state_negative_output_quantity_is_rejected`; `representable_quantity_encodes_byte_identical_golden`; `stage2_mempack_word64_output_survives_snapshot_recovery` |
| **CI** | `ci/ci_check_value_quantity_domain.sh` |

### DC-PLUTUS

#### `DC-PLUTUS-01` — _declared_

| Aspect | Location |
|--------|----------|
| **Source** | Project constitution §3, T-DET-01, T-CORE-01 |
| **Requirement** | UPLC evaluation is deterministic: same script + args + cost model = identical result |
| **Code** | _(no enforcement found — gap)_ |
| **Tests** | _(no tests listed — gap)_ |
| **CI** | _(no CI gate — gap)_ |

#### `DC-PLUTUS-02` — _declared_

| Aspect | Location |
|--------|----------|
| **Source** | Project constitution §3, T-ERR-01 |
| **Requirement** | Budget exhaustion produces deterministic structured error |
| **Code** | _(no enforcement found — gap)_ |
| **Tests** | _(no tests listed — gap)_ |
| **CI** | _(no CI gate — gap)_ |

### DC-CONSENSUS

#### `DC-CONSENSUS-01` — _enforced_

| Aspect | Location |
|--------|----------|
| **Source** | Project constitution §3, T-DET-01, T-CORE-01 |
| **Requirement** | Chain selection is deterministic and matches Haskell node behavior |
| **Code** | crates/ade_core/src/consensus/fork_choice.rs, crates/ade_core/src/consensus/candidate.rs, crates/ade_core/src/consensus/rollback.rs, crates/ade_runtime/src/consensus/chain_selector.rs, crates/ade_core_interop/src/lib.rs, crates/ade_core_interop/src/bin/live_consensus_session.rs |
| **Tests** | `higher_block_no_wins`; `equal_block_no_tiebreaker_decides`; `fork_before_immutable_tip_rejected`; `exceeded_rollback_rejected`; `tiebreaker_loss_keeps_current`; `replay_is_deterministic`; `reject_reason_bytes_are_stable`; `no_candidates_returns_error`; `tiebreaker_prefer_lower_slot_wins`; `tiebreaker_prefer_higher_op_cert_wins_on_equal_slot_and_issuer`; … (+16 more) |
| **CI** | `ci/ci_check_no_density_in_fork_choice.sh`; `ci/ci_check_consensus_closed_enums.sh`; `ci/ci_check_no_chaindb_in_consensus_blue.sh` |

#### `DC-CONSENSUS-02` — _partial_

| Aspect | Location |
|--------|----------|
| **Source** | Project constitution §3, T-CORE-01 |
| **Requirement** | Leadership verification is pure |
| **Code** | crates/ade_core/src/consensus/leader_schedule.rs, crates/ade_core/src/consensus/ledger_view.rs, crates/ade_core/src/consensus/vrf_cert.rs, crates/ade_ledger/src/consensus_view.rs |
| **Tests** | `query_uses_state_epoch_nonce_for_vrf_input`; `query_returns_unknown_pool_when_no_vrf_key`; `query_returns_outside_forecast_range_for_far_future`; `query_does_not_mutate_state`; `eligible_on_threshold_with_high_stake_emits_eligible_verdict`; `not_eligible_with_zero_stake_emits_not_eligible_verdict`; `corpus_returns_canonical_answer_for_known_pools`; `corpus_rejects_unknown_pool`; `corpus_rejects_out_of_forecast_horizon`; `corpus_is_leader_helper_matches_pinned_probe`; … (+6 more) |
| **CI** | _(no CI gate — gap)_ |

### DC-PROTO

#### `DC-PROTO-01` — _enforced_

| Aspect | Location |
|--------|----------|
| **Source** | Project constitution §3, T-CORE-03 |
| **Requirement** | Protocol state machines have deterministic transitions |
| **Code** | crates/ade_network/src/chain_sync/state.rs, crates/ade_network/src/chain_sync/agency.rs, crates/ade_network/src/chain_sync/signal.rs, crates/ade_network/src/chain_sync/transition.rs, crates/ade_network/src/block_fetch/state.rs, crates/ade_network/src/block_fetch/agency.rs, crates/ade_network/src/block_fetch/event.rs, crates/ade_network/src/block_fetch/transition.rs, crates/ade_network/src/tx_submission/state.rs, crates/ade_network/src/tx_submission/agency.rs, crates/ade_network/src/tx_submission/event.rs, crates/ade_network/src/tx_submission/transition.rs, crates/ade_network/src/keep_alive/state.rs, crates/ade_network/src/keep_alive/agency.rs, crates/ade_network/src/keep_alive/event.rs,… |
| **Tests** | `idle_request_next_with_immediate_data_yields_can_await_then_roll_forward`; `idle_request_next_with_no_data_yields_must_reply_via_await`; `roll_forward_signal_carries_header_and_tip_byte_identical`; `roll_backward_signal_carries_point_and_tip_byte_identical`; `find_intersect_with_known_point_yields_intersected_signal`; `find_intersect_with_unknown_points_yields_no_intersection`; `client_done_terminates_session`; `illegal_message_in_idle_returns_error`; `wrong_agency_returns_error`; `version_gating_rejects_out_of_version_message`; … (+80 more) |
| **CI** | `ci/ci_check_mini_protocol_transition_purity.sh` |

#### `DC-PROTO-02` — _enforced_

| Aspect | Location |
|--------|----------|
| **Source** | Project constitution §3, T-DET-01 |
| **Requirement** | Transcript-equivalent miniprotocol behavior with Haskell node |
| **Code** | crates/ade_network/src/chain_sync/state.rs, crates/ade_network/src/chain_sync/agency.rs, crates/ade_network/src/chain_sync/signal.rs, crates/ade_network/src/chain_sync/transition.rs, crates/ade_network/src/block_fetch/state.rs, crates/ade_network/src/block_fetch/agency.rs, crates/ade_network/src/block_fetch/event.rs, crates/ade_network/src/block_fetch/transition.rs, crates/ade_network/src/tx_submission/state.rs, crates/ade_network/src/tx_submission/agency.rs, crates/ade_network/src/tx_submission/event.rs, crates/ade_network/src/tx_submission/transition.rs |
| **Tests** | `idle_request_next_with_immediate_data_yields_can_await_then_roll_forward`; `idle_request_next_with_no_data_yields_must_reply_via_await`; `roll_forward_signal_carries_header_and_tip_byte_identical`; `roll_backward_signal_carries_point_and_tip_byte_identical`; `find_intersect_with_known_point_yields_intersected_signal`; `find_intersect_with_unknown_points_yields_no_intersection`; `client_done_terminates_session`; `illegal_message_in_idle_returns_error`; `wrong_agency_returns_error`; `version_gating_rejects_out_of_version_message`; … (+38 more) |
| **CI** | `ci/ci_check_tx_submission2_real_capture.sh` |

#### `DC-PROTO-03` — _enforced_

| Aspect | Location |
|--------|----------|
| **Source** | Project constitution §3, T-DET-01, T-CORE-03 |
| **Requirement** | Full N2N mini-protocol surface: Handshake, ChainSync, BlockFetch, TxSubmission2, KeepAlive, PeerSharing |
| **Code** | crates/ade_network/src/codec/handshake.rs, crates/ade_network/src/codec/chain_sync.rs, crates/ade_network/src/codec/block_fetch.rs, crates/ade_network/src/codec/tx_submission.rs, crates/ade_network/src/codec/keep_alive.rs, crates/ade_network/src/codec/peer_sharing.rs |
| **Tests** | `roundtrip_every_variant`; `roundtrip_every_variant`; `roundtrip_every_variant`; `roundtrip_every_variant`; `roundtrip_every_variant`; `roundtrip_every_variant` |
| **CI** | `ci/ci_check_mini_protocol_surface.sh` |

#### `DC-PROTO-04` — _enforced_

| Aspect | Location |
|--------|----------|
| **Source** | Project constitution §3, T-DET-01, T-CORE-03 |
| **Requirement** | Full N2C mini-protocol surface: Handshake, LocalChainSync, LocalTxSubmission, LocalStateQuery, LocalTxMonitor |
| **Code** | crates/ade_network/src/codec/n2c_handshake.rs, crates/ade_network/src/codec/local_chain_sync.rs, crates/ade_network/src/codec/local_tx_submission.rs, crates/ade_network/src/codec/local_state_query.rs, crates/ade_network/src/codec/local_tx_monitor.rs, crates/ade_network/src/n2c/local_chain_sync/state.rs, crates/ade_network/src/n2c/local_chain_sync/agency.rs, crates/ade_network/src/n2c/local_chain_sync/event.rs, crates/ade_network/src/n2c/local_chain_sync/transition.rs, crates/ade_network/src/n2c/local_tx_submission/state.rs, crates/ade_network/src/n2c/local_tx_submission/agency.rs, crates/ade_network/src/n2c/local_tx_submission/event.rs,… |
| **Tests** | `roundtrip_every_variant`; `roundtrip_every_variant`; `roundtrip_every_variant`; `roundtrip_every_variant`; `roundtrip_every_variant`; `local_chain_sync_request_next_then_roll_forward`; `local_chain_sync_roll_backward_signal`; `local_chain_sync_find_intersect_known_point`; `local_chain_sync_find_intersect_unknown`; `local_chain_sync_client_done_terminates`; … (+32 more) |
| **CI** | `ci/ci_check_mini_protocol_surface.sh` |

#### `DC-PROTO-05` — _declared_

| Aspect | Location |
|--------|----------|
| **Source** | Project constitution §3, T-ENC-02, T-CORE-03 |
| **Requirement** | Version negotiation is closed: enumerated N2N/N2C versions, explicit handshake, deterministic refusal on mismatch |
| **Code** | crates/ade_network/src/handshake/mod.rs, crates/ade_network/src/handshake/state.rs, crates/ade_network/src/handshake/agency.rs, crates/ade_network/src/handshake/selection.rs, crates/ade_network/src/handshake/transition.rs, crates/ade_network/src/handshake/version_table.rs |
| **Tests** | `n2n_happy_path_each_supported_version`; `n2c_happy_path_each_supported_version`; `version_mismatch_refused`; `illegal_message_in_idle_returns_error`; `wrong_agency_returns_error`; `overlap_picks_highest_common`; `empty_intersection_refuses_deterministically`; `version_data_passed_through_byte_identical`; `n2n_v15_happy_path`; `n2n_v16_happy_path_with_peras_support_field`; … (+11 more) |
| **CI** | `ci/ci_check_ce_n_a_5_proof.sh` |

#### `DC-PROTO-06` — _enforced_

| Aspect | Location |
|--------|----------|
| **Source** | Project constitution §2; PHASE4-N-A invariants §7 decision 1 (docs/active/PHASE4-N-A_invariants.md) |
| **Requirement** | BLUE mini-protocol transitions are pure functions of (canonical prior state, canonical input message, selected protocol version, deterministic configuration); no ambient session-glue state may alter authoritative behavior. Selected version is an explicit input, never read from RED context. |
| **Code** | crates/ade_network/src/chain_sync/state.rs, crates/ade_network/src/chain_sync/agency.rs, crates/ade_network/src/chain_sync/signal.rs, crates/ade_network/src/chain_sync/transition.rs, crates/ade_network/src/block_fetch/state.rs, crates/ade_network/src/block_fetch/agency.rs, crates/ade_network/src/block_fetch/event.rs, crates/ade_network/src/block_fetch/transition.rs, crates/ade_network/src/tx_submission/state.rs, crates/ade_network/src/tx_submission/agency.rs, crates/ade_network/src/tx_submission/event.rs, crates/ade_network/src/tx_submission/transition.rs, crates/ade_network/src/keep_alive/state.rs, crates/ade_network/src/keep_alive/agency.rs, crates/ade_network/src/keep_alive/event.rs,… |
| **Tests** | `idle_request_next_with_immediate_data_yields_can_await_then_roll_forward`; `idle_request_next_with_no_data_yields_must_reply_via_await`; `roll_forward_signal_carries_header_and_tip_byte_identical`; `roll_backward_signal_carries_point_and_tip_byte_identical`; `find_intersect_with_known_point_yields_intersected_signal`; `find_intersect_with_unknown_points_yields_no_intersection`; `client_done_terminates_session`; `illegal_message_in_idle_returns_error`; `wrong_agency_returns_error`; `version_gating_rejects_out_of_version_message`; … (+80 more) |
| **CI** | `ci/ci_check_mini_protocol_transition_purity.sh` |

#### `DC-PROTO-07` — _enforced_

| Aspect | Location |
|--------|----------|
| **Source** | docs/planning/phase4-n-a-successor-invariants.md §3 |
| **Requirement** | Given canonical inputs (negotiated_version, peer_message_sequence, broadcast_arrival_sequence, session_event_sequence), the producer-side chain-sync / block-fetch session orchestrator emits a byte-identical sequence of outgoing mini-protocol frames across replays. The per-session reducer is a pure deterministic transition. |
| **Code** | crates/ade_network/src/chain_sync/server.rs (producer_chain_sync_serve + producer_chain_sync_advance_tip — pure, total, deterministic); crates/ade_network/src/block_fetch/server.rs (producer_block_fetch_serve — pure, total, deterministic); crates/ade_runtime/src/producer/broadcast_to_served.rs (GREEN drain_and_admit — pure, no I/O); crates/ade_runtime/src/producer/served_chain_lookups.rs (trait impls — pure projections) |
| **Tests** | `producer_chain_sync_serve_replays_byte_identical_over_corpus`; `producer_block_fetch_serve_replays_byte_identical_over_corpus`; `drain_and_admit_is_deterministic_over_arrival_sequence`; `session_transcript_replay_byte_identical` |
| **CI** | `ci/ci_check_chain_sync_server_closure.sh`; `ci/ci_check_block_fetch_server_closure.sh`; `ci/ci_check_broadcast_to_served_purity.sh` |

#### `DC-PROTO-08` — _enforced_

| Aspect | Location |
|--------|----------|
| **Source** | docs/planning/phase4-n-a-successor-invariants.md §1 (I-8), §2 (¬P-4) |
| **Requirement** | Once chain-sync enters a state where the server holds agency, the pure per-session reducer must return exactly one of: a legal RollForward, a legal RollBackward, a legal AwaitReply, or a structured deterministic session-close/error. It must not return an ambiguous wait state unless the wait condition is an explicit replay input. |
| **Code** | crates/ade_network/src/chain_sync/server.rs (producer_chain_sync_serve + producer_chain_sync_advance_tip, total over server-agency states; exhaustive match returns ServerStep::Reply or ServerStep::Done; no silent wait) |
| **Tests** | `producer_chain_sync_serve_request_next_idle_yields_roll_forward_when_served_has_block`; `producer_chain_sync_serve_request_next_idle_yields_await_reply_when_served_empty`; `producer_chain_sync_serve_find_intersect_known_point_yields_intersect_found`; `producer_chain_sync_serve_find_intersect_unknown_point_yields_intersect_not_found`; `producer_chain_sync_serve_done_terminates_session`; `producer_chain_sync_serve_rejects_illegal_grammar_pair`; `producer_chain_sync_advance_tip_idle_yields_none`; `producer_chain_sync_advance_tip_can_await_yields_roll_forward_when_block_available`; `producer_chain_sync_advance_tip_must_reply_yields_roll_forward_when_block_available`; `producer_chain_sync_advance_tip_can_await_yields_none_when_cursor_at_head`; … (+2 more) |
| **CI** | `ci/ci_check_chain_sync_server_closure.sh` |

#### `DC-PROTO-09` — _enforced_

| Aspect | Location |
|--------|----------|
| **Source** | docs/planning/receive-side-bridge-invariants.md §3, §4 |
| **Requirement** | docs/planning/receive-side-bridge-invariants.md §3, §4 *(paraphrased from source/code_locus; registry `statement` empty)* |
| **Code** | crates/ade_ledger/src/receive/reducer.rs (receive_apply + receive_apply_sequence pure transitions); crates/ade_runtime/src/receive/events_to_state.rs (GREEN adapter — pure, no I/O); crates/ade_runtime/src/receive/in_memory_chain_write.rs (GREEN ChainDb-write adapter — pure projection over an in-memory ChainDb) |
| **Tests** | `receive_apply_replay_byte_identical_over_corpus`; `receive_session_transcript_replay_byte_identical` |
| **CI** | `ci/ci_check_receive_reducer_closure.sh`; `ci/ci_check_receive_replay_purity.sh` |

#### `DC-PROTO-10` — _enforced_

| Aspect | Location |
|--------|----------|
| **Source** | docs/clusters/PHASE4-N-AE/slices/AE.E.md; docs/evidence/phase4-n-ae-ce-a5-relay-adoption.md |
| **Requirement** | docs/clusters/PHASE4-N-AE/slices/AE.E.md; docs/evidence/phase4-n-ae-ce-a5-relay-adoption.md *(paraphrased from source/code_locus; registry `statement` empty)* |
| **Code** | crates/ade_network/src/chain_sync/server.rs (producer_chain_sync_serve FindIntersect handler -- sets state.last_announced from the matched intersect point) |
| **Tests** | `producer_chain_sync_serve_find_intersect_sets_cursor_then_rolls_forward_past_it` |
| **CI** | _(no CI listed in registry — enforced via Code/Tests; gap: bind a CI gate)_ |

#### `DC-PROTO-11` — _enforced_

| Aspect | Location |
|--------|----------|
| **Source** | Live server-side tx-submission2 capture (option B) finding; DC-PROTO-02; Byte Authority Model §3 |
| **Requirement** | Live server-side tx-submission2 capture (option B) finding; DC-PROTO-02; Byte Authority Model §3 *(paraphrased from source/code_locus; registry `statement` empty)* |
| **Code** | crates/ade_network/src/codec/tx_submission.rs (TxSubmissionTxId era-tag + decode_seq definite/indefinite + indefinite encode); corpus/network/n2n/tx_submission2/ |
| **Tests** | `roundtrip_every_variant`; `encoder_emits_indefinite_sequences`; `real_cardano_reply_txids_decodes_and_re_encodes_byte_identical`; `decode_accepts_definite_sequence_form`; `decode_rejects_bare_txid`; `decode_rejects_wrong_txid_hash_length`; `decode_rejects_unterminated_indefinite_sequence`; `real_capture_round_trips_byte_identical`; `reply_txids_entries_are_real_32_byte_txids` |
| **CI** | `ci/ci_check_tx_submission2_real_capture.sh` |

### DC-STORE

#### `DC-STORE-01` — _enforced_

| Aspect | Location |
|--------|----------|
| **Source** | Project constitution §3, T-REC-01 |
| **Requirement** | Recovery from power-loss produces replay-equivalent state |
| **Code** | crates/ade_runtime/src/chaindb/crash_safety.rs, crates/ade_runtime/tests/stress_kill_harness.rs |
| **Tests** | `stress_kill_smoke`; `stress_kill_1000`; `snapshot_table_intact_after_kill_loop`; `persistent_passes_crash_safety_with_no_kill` |
| **CI** | `ci/ci_check_chaindb_crash_safety.sh` |

#### `DC-STORE-02` — _enforced_

| Aspect | Location |
|--------|----------|
| **Source** | Project constitution §3, T-REC-02 |
| **Requirement** | Append-only provenance for finalized data |
| **Code** | crates/ade_runtime/src/chaindb/persistent.rs, crates/ade_runtime/src/chaindb/contract.rs |
| **Tests** | `persistent_passes_contract`; `in_memory_passes_contract`; `reopen_observes_committed_block` |
| **CI** | `ci/ci_check_chaindb_contract.sh` |

#### `DC-STORE-03` — _enforced_

| Aspect | Location |
|--------|----------|
| **Source** | Project constitution §3, T-REC-01 |
| **Requirement** | Atomic snapshots (fully written or absent) |
| **Code** | crates/ade_runtime/src/chaindb/snapshot_contract.rs, crates/ade_runtime/src/chaindb/persistent.rs |
| **Tests** | `persistent_passes_snapshot_contract`; `in_memory_passes_snapshot_contract`; `snapshots_persist_across_reopen`; `corrupted_magic_returns_corruption_error` |
| **CI** | `ci/ci_check_chaindb_contract.sh` |

#### `DC-STORE-04` — _declared_

| Aspect | Location |
|--------|----------|
| **Source** | Project constitution §3, T-REC-01, T-REC-02 |
| **Requirement** | ChainDB structure: ImmutableDB (append-only, blocks immutable when k-deep), VolatileDB (recent blocks within k), LedgerDB (snapshots + forward replay) |
| **Code** | _(no enforcement found — gap)_ |
| **Tests** | _(no tests listed — gap)_ |
| **CI** | _(no CI gate — gap)_ |

#### `DC-STORE-05` — _enforced_

| Aspect | Location |
|--------|----------|
| **Source** | Project constitution §3, T-REC-01 |
| **Requirement** | Recovery is snapshot + forward replay (not full genesis replay): load most recent valid snapshot, replay forward from ImmutableDB tip |
| **Code** | crates/ade_runtime/src/recovery/mod.rs, crates/ade_runtime/src/recovery/restart.rs |
| **Tests** | `recover_from_snapshot_and_replay_forward`; `recover_from_genesis_when_no_snapshot`; `no_starting_point_error`; `snapshot_with_no_post_blocks_is_ok` |
| **CI** | `ci/ci_check_recovery_contract.sh` |

#### `DC-STORE-06` — _declared_

| Aspect | Location |
|--------|----------|
| **Source** | Project constitution §3, T-INGRESS-01 |
| **Requirement** | VolatileDB uses ValidateAll after unclean shutdown; NoValidation acceptable during clean operation as optimization |
| **Code** | _(no enforcement found — gap)_ |
| **Tests** | _(no tests listed — gap)_ |
| **CI** | _(no CI gate — gap)_ |

#### `DC-STORE-07` — _enforced_

| Aspect | Location |
|--------|----------|
| **Source** | docs/planning/ledger-snapshot-rollback-invariants.md §1 (I-8) |
| **Requirement** | docs/planning/ledger-snapshot-rollback-invariants.md §1 (I-8) *(paraphrased from source/code_locus; registry `statement` empty)* |
| **Code** | crates/ade_runtime/src/rollback/cadence.rs (should_snapshot_after_block pure decision; SnapshotCadence has only the every_n_blocks BLUE-structural field); crates/ade_runtime/src/rollback/in_memory_cache.rs (InMemorySnapshotCache impl SnapshotReader); crates/ade_runtime/src/rollback/chaindb_block_source.rs (ChainDbBlockSource impl BlockSource) |
| **Tests** | `should_snapshot_after_block_every_n_returns_true_at_cadence`; `should_snapshot_after_block_returns_false_off_cadence`; `should_snapshot_after_block_returns_false_when_already_at_or_after_slot`; `should_snapshot_after_block_is_pure`; `snapshot_cadence_default_is_100_blocks`; `in_memory_snapshot_cache_nearest_le_returns_largest_key`; `in_memory_snapshot_cache_iteration_is_btreemap_ordered` |
| **CI** | `ci/ci_check_snapshot_cadence_purity.sh` |

#### `DC-STORE-08` — _enforced_

| Aspect | Location |
|--------|----------|
| **Source** | docs/planning/persistent-snapshot-encoder-invariants.md §1 (I-2) |
| **Requirement** | docs/planning/persistent-snapshot-encoder-invariants.md §1 (I-2) *(paraphrased from source/code_locus; registry `statement` empty)* |
| **Code** | crates/ade_ledger/src/snapshot/{chain_dep,utxo_state,cert_state,epoch_state,gov_state,ledger,framing}.rs |
| **Tests** | `chain_dep_encode_deterministic_across_runs`; `utxo_state_encode_deterministic_across_runs`; `cert_state_encode_deterministic_across_runs`; `epoch_state_encode_deterministic_across_runs`; `pparams_encode_deterministic_across_runs`; `gov_state_encode_deterministic_across_runs`; `ledger_state_encode_deterministic_across_runs`; `snapshot_encode_deterministic_across_runs`; `round_trip_via_fingerprint_combined` |
| **CI** | `ci/ci_check_snapshot_encoder_closure.sh` |

#### `DC-STORE-09` — _enforced_

| Aspect | Location |
|--------|----------|
| **Source** | docs/planning/persistent-snapshot-encoder-invariants.md §1 (I-3, I-4) |
| **Requirement** | docs/planning/persistent-snapshot-encoder-invariants.md §1 (I-3, I-4) *(paraphrased from source/code_locus; registry `statement` empty)* |
| **Code** | crates/ade_ledger/src/snapshot/framing.rs |
| **Tests** | `snapshot_round_trip`; `decode_rejects_unknown_version`; `decode_rejects_fingerprint_mismatch` |
| **CI** | `ci/ci_check_snapshot_encoder_closure.sh` |

### DC-INGRESS

#### `DC-INGRESS-01` — _declared_

| Aspect | Location |
|--------|----------|
| **Source** | Project constitution §3, T-INGRESS-01 |
| **Requirement** | Block/tx/protocol message decoding enters core through named chokepoints; no raw-byte bypass without CI-whitelisted justification |
| **Code** | _(no enforcement found — gap)_ |
| **Tests** | _(no tests listed — gap)_ |
| **CI** | `ci/ci_check_ingress_chokepoints.sh` |

#### `DC-INGRESS-02` — _declared_

| Aspect | Location |
|--------|----------|
| **Source** | Project constitution §3, T-INGRESS-01, T-ENC-01 |
| **Requirement** | Storage rehydration enters core through the same canonical decode chokepoints as network ingress |
| **Code** | _(no enforcement found — gap)_ |
| **Tests** | _(no tests listed — gap)_ |
| **CI** | _(no CI gate — gap)_ |

### DC-REF

#### `DC-REF-01` — _partial_

| Aspect | Location |
|--------|----------|
| **Source** | Project constitution §3, T-DET-01, T-CI-01 |
| **Requirement** | Every claimed equivalence check must identify its reference source, extraction method, and reproducibility path |
| **Code** | crates/ade_testkit/src/harness/provenance.rs |
| **Tests** | `validate_complete_manifest_no_violations`; `validate_empty_manifest_no_violations`; `validate_detects_empty_field`; `self_comparison_zero_divergences_byron`; `self_comparison_zero_divergences_shelley`; `self_comparison_zero_divergences_allegra`; `self_comparison_zero_divergences_mary`; `self_comparison_zero_divergences_alonzo`; `self_comparison_zero_divergences_babbage`; `self_comparison_zero_divergences_conway` |
| **CI** | `ci/ci_check_ref_provenance.sh` |

### DC-EPOCH

#### `DC-EPOCH-01` — _partial_

| Aspect | Location |
|--------|----------|
| **Source** | Project constitution §3, T-CAUSAL-01, T-EPOCH-01 |
| **Requirement** | Conway governance timing: proposals accumulate during epoch, ratification and enactment are atomic at epoch boundary, pulsing distributes DRep stake computation across epoch |
| **Code** | crates/ade_ledger/src/governance.rs (enact_proposals + apply_committee_enactment), crates/ade_ledger/src/rules.rs (epoch-boundary apply site) |
| **Tests** | `conway_epoch_boundary_end_to_end`; `conway_governance_ratification_test`; `enact_noconfidence_dissolves_committee`; `enact_update_committee_applies_changes`; `committee_enactment_replays_byte_identical`; `epoch_boundary_ratified_noconfidence_dissolves_committee`; `committee_oracle_mainnet_575_576_noop_agreement` |
| **CI** | `ci/ci_check_credential_discriminant_closed.sh` |

#### `DC-EPOCH-02` — _enforced_

| Aspect | Location |
|--------|----------|
| **Source** | Project constitution §3, T-DET-01, T-CORE-03 |
| **Requirement** | Hard fork transitions triggered at deterministic slot/epoch boundaries; era translation functions mandatory; forecast horizon extends to era boundary |
| **Code** | crates/ade_ledger/src/hfc.rs, crates/ade_core/src/consensus/era_schedule.rs |
| **Tests** | `shelley_allegra_summary_matches_oracle`; `all_non_byron_translations_preserve_sub_state`; `shelley_allegra_transition_proof_surface`; `all_transitions_proof_surface_summary`; `locate_first_slot_of_each_era`; `locate_last_slot_of_each_era`; `forecast_horizon_boundary`; `mainnet_corpus_translation_matches_oracle`; `preprod_corpus_translation_matches_oracle` |
| **CI** | `ci/ci_check_hfc_translation.sh` |

#### `DC-EPOCH-03` — _enforced_

| Aspect | Location |
|--------|----------|
| **Source** | docs/planning/phase4-n-f-g-invariants.md |
| **Requirement** | docs/planning/phase4-n-f-g-invariants.md *(paraphrased from source/code_locus; registry `statement` empty)* |
| **Code** | crates/ade_node/src/node_sync.rs, crates/ade_node/src/node_lifecycle.rs, crates/ade_node/src/run_loop_planner.rs, crates/ade_core/src/consensus/nonce.rs |
| **Tests** | `forge_epoch_admission_within_seed_epoch_admits`; `forge_epoch_admission_off_epoch_fails_closed`; `forge_epoch_admission_unlocatable_fails_closed`; `node_forge_off_epoch_slot_fails_closed`; `node_forge_no_epoch_boundary_promotion_on_forge_path`; `forge_tick_off_epoch_slot_fails_closed_local` |
| **CI** | `ci/ci_check_node_forge_single_epoch_fail_closed.sh` |

#### `DC-EPOCH-04` — _enforced_

| Aspect | Location |
|--------|----------|
| **Source** | docs/clusters/EPOCH-CONSENSUS-VIEW/SLICE-3f4-activation-flip.md (S3f-4a); user directive 2026-06-21 (one atomic WAL-backed activation; explicit activation idempotence) |
| **Requirement** | user directive 2026-06-21 (one atomic WAL-backed activation; explicit activation idempotence) *(paraphrased from source/code_locus; registry `statement` empty)* |
| **Code** | crates/ade_ledger/src/wal/event.rs: WalEntry::EpochConsensusViewActivated{target_epoch,network_magic,era,transition_point,source_checkpoint_commitment,snapshot_phase,nonce_commitment,stake_view_canonical_hash,view_canonical_hash} + TAG_EPOCH_CONSENSUS_VIEW_ACTIVATED=4 + canonical encode/decode (snapshot_phase_wire/era ALL-tag) + activation_replay_outcome -> ActivationReplayOutcome{Idempotent\|Conflict}. crates/ade_ledger/src/reduced_epoch_view.rs: EpochConsensusView::stake_view_canonical_hash. ci/ci_check_eview_activation_wal.sh. |
| **Tests** | `wal_epoch_view_activated_round_trips_byte_identical`; `wal_epoch_view_activated_uses_tag_four`; `activation_replay_idempotent_vs_conflict` |
| **CI** | `ci/ci_check_eview_activation_wal.sh` |

#### `DC-EPOCH-05` — _enforced_

| Aspect | Location |
|--------|----------|
| **Source** | docs/clusters/EPOCH-CONSENSUS-VIEW/SLICE-3f4-activation-flip.md (S3f-4b); user directive 2026-06-21 (no runtime flag; promoted view fully replaces the seed view) |
| **Requirement** | user directive 2026-06-21 (no runtime flag; promoted view fully replaces the seed view) *(paraphrased from source/code_locus; registry `statement` empty)* |
| **Code** | crates/ade_node/src/epoch_activation.rs: ActiveEpochView{Seed\|Promoted(EpochConsensusView)} -- one-way promote() (Seed->Promoted; same-view idempotent; differing-view -> EpochViewActivationConflict), promoted()->Option (Some only post-promotion), is_promoted(). ci/ci_check_eview_activation_predicate.sh. |
| **Tests** | `active_view_one_way_promote_and_idempotence`; `active_view_conflicting_promotion_is_terminal`; `seed_exposes_no_n1_view_until_promotion` |
| **CI** | `ci/ci_check_eview_activation_predicate.sh` |

#### `DC-EPOCH-06` — _enforced_

| Aspect | Location |
|--------|----------|
| **Source** | docs/clusters/EPOCH-CONSENSUS-VIEW/SLICE-3f4-activation-flip.md (S3f-4c); user directive 2026-06-21 (durable-before-visible; crash before/after WAL; recovered must match WAL) |
| **Requirement** | user directive 2026-06-21 (durable-before-visible; crash before/after WAL; recovered must match WAL) *(paraphrased from source/code_locus; registry `statement` empty)* |
| **Code** | crates/ade_node/src/epoch_activation.rs: activate_durable_before_visible(candidate, wal_write_durable) (durable->Promoted, else EpochViewActivationFailed); recover_active_view(record, candidate) (None->Seed; match->Promoted; else EpochViewPostPromotionMismatch); activation_record_matches (complete identity incl. both hashes + verify); activation_record_for (the record builder); resolve_activation_record (DC-EPOCH-04 fold). ci/ci_check_eview_activation_recovery.sh. |
| **Tests** | `crash_before_durable_wal_keeps_seed`; `crash_after_wal_republishes_same_view`; `recovered_view_mismatch_is_terminal`; `durable_before_visible_halts_on_wal_failure`; `resolve_activation_idempotent_conflict_supersede` |
| **CI** | `ci/ci_check_eview_activation_recovery.sh` |

#### `DC-EPOCH-07` — _enforced_

| Aspect | Location |
|--------|----------|
| **Source** | docs/clusters/EPOCH-CONSENSUS-VIEW/SLICE-3f4-activation-flip.md (S3f-4b); user directive 2026-06-21 (halt on activation failure or mismatch; no seed-view fallback) |
| **Requirement** | user directive 2026-06-21 (halt on activation failure or mismatch; no seed-view fallback) *(paraphrased from source/code_locus; registry `statement` empty)* |
| **Code** | crates/ade_node/src/epoch_activation.rs: activation_predicate(candidate, n1_bindings, selected_point, transition_eligible, wal_durable) -> ActivationOutcome{Promote\|NoPromotion(ActivationReject{TransitionIneligible\|BindingsUnverified\|WrongSelectedPoint\|WalNotDurable})}; EpochViewActivationError{EpochViewActivationFailed\|EpochViewActivationConflict\|EpochViewPostPromotionMismatch}. ci/ci_check_eview_activation_predicate.sh. |
| **Tests** | `predicate_promotes_only_when_every_precondition_holds`; `predicate_rejects_each_failed_precondition`; `active_view_conflicting_promotion_is_terminal` |
| **CI** | `ci/ci_check_eview_activation_predicate.sh` |

#### `DC-EPOCH-08` — _enforced_

| Aspect | Location |
|--------|----------|
| **Source** | docs/clusters/EPOCH-CONSENSUS-VIEW/SLICE-3f4d-live-flip.md (S3f-4d-1); user directive 2026-06-21 (durable ChainDB source only; named roles; the Mark/Set lag is a proof obligation) |
| **Requirement** | user directive 2026-06-21 (durable ChainDB source only; named roles; the Mark/Set lag is a proof obligation) *(paraphrased from source/code_locus; registry `statement` empty)* |
| **Code** | crates/ade_node/src/epoch_source_window.rs: ActivationSourceWindow{source_epoch,source_window_start,source_window_end,snapshot_phase,target_epoch,source_window_anchor,lineage_pin}; LEADERSHIP_SNAPSHOT_LAG_EPOCHS (the single lag constant, proof obligation) + target_epoch_for_source; validate_source_window -> SourceWindowError{Empty\|OutOfWindow\|NotOrdered\|Duplicate\|AnchorMismatch\|ChainGap\|LineageMismatch\|TargetEpochMismatch}. ci/ci_check_eview_source_window.sh. |
| **Tests** | `target_epoch_is_the_explicit_lag`; `valid_window_passes`; `empty_window_fails_closed`; `out_of_window_block_fails_closed`; `unordered_and_duplicate_fail_closed`; `missing_block_breaks_the_chain`; `anchor_and_lineage_pin_fail_closed`; `wrong_target_epoch_fails_closed` |
| **CI** | `ci/ci_check_eview_source_window.sh` |

#### `DC-EPOCH-09` — _enforced_

| Aspect | Location |
|--------|----------|
| **Source** | docs/clusters/EPOCH-CONSENSUS-VIEW/SLICE-3f4d-live-flip.md (S3f-4d-2) |
| **Requirement** | docs/clusters/EPOCH-CONSENSUS-VIEW/SLICE-3f4d-live-flip.md (S3f-4d-2) *(paraphrased from source/code_locus; registry `statement` empty)* |
| **Code** | crates/ade_node/src/epoch_candidate.rs: derive_candidate(window, checkpoint, bootstrap_state, blocks, era, network_magic, nonce) -> drive_window_aggregate -> checkpoint.finalize() (the commitment) -> EpochConsensusView::bind(.., window.target_epoch, Point{source_window_end, lineage_pin}, commitment, nonce, window.snapshot_phase, stake..); CandidateDeriveError{Drive\|Checkpoint}. ci/ci_check_eview_candidate.sh. |
| **Tests** | `derive_candidate_binds_target_epoch_and_round_trips_through_recovery` |
| **CI** | `ci/ci_check_eview_candidate.sh` |

#### `DC-EPOCH-10` — _enforced_

| Aspect | Location |
|--------|----------|
| **Source** | docs/clusters/EPOCH-CONSENSUS-VIEW/SLICE-3f4d-live-flip.md (S3f-4d-3a); user directive 2026-06-21 (one atomic path; durable-before-visible; halt on terminal) |
| **Requirement** | user directive 2026-06-21 (one atomic path; durable-before-visible; halt on terminal) *(paraphrased from source/code_locus; registry `statement` empty)* |
| **Code** | crates/ade_node/src/epoch_activate.rs: activate_at_boundary(window, window_blocks, checkpoint, bootstrap_state, blocks, era, network, nonce, selected_point, transition_eligible, active_view, wal_write) -> validate_source_window -> derive_candidate -> activation_predicate -> activation_record_for + wal_write (durable) -> activate_durable_before_visible -> active_view.promote; BoundaryActivationOutcome{Promoted\|NotYet}. ci/ci_check_eview_activate.sh. |
| **Tests** | `happy_path_promotes_after_durable_wal`; `non_durable_wal_is_terminal_and_does_not_publish`; `not_eligible_transition_is_not_yet_not_terminal`; `invalid_window_is_terminal_before_any_wal`; `selected_point_mismatch_declines` |
| **CI** | `ci/ci_check_eview_activate.sh` |

#### `DC-EPOCH-11` — _declared_

| Aspect | Location |
|--------|----------|
| **Source** | docs/clusters/EPOCH-CONSENSUS-VIEW/SLICE-3f4d-mat-live-checkpoint.md; user directive 2026-06-21 (make the reduced-checkpoint machinery live on the admission path; 8 locked points) |
| **Requirement** | user directive 2026-06-21 (make the reduced-checkpoint machinery live on the admission path; 8 locked points) *(paraphrased from source/code_locus; registry `statement` empty)* |
| **Code** | crates/ade_node/src/admission/bootstrap.rs: build_live_reduced_checkpoint(snapshot_dir, utxo) -> reduce_txout each output into a BTreeMap -> ReducedUtxoCheckpoint::open(snapshot_dir/reduced-checkpoint.redb).build_from; called BEFORE drop(utxo), gated on ledger.cert_state.delegation.delegations non-empty (the EVIEW package); AdmissionBootstrapError::ReducedCheckpoint fail-closed. ci/ci_check_eview_live_checkpoint.sh. |
| **Tests** | `live_reduced_checkpoint_builds_durable_deterministic` |
| **CI** | `ci/ci_check_eview_live_checkpoint.sh` |

#### `DC-EPOCH-12` — _enforced_

| Aspect | Location |
|--------|----------|
| **Source** | docs/clusters/EPOCH-CONSENSUS-VIEW/SLICE-ECA-0b-leadership-complete-view.md; user directive 2026-06-21 (no live CertState read at rebind; no unbound protocol-parameter read; ASC reaches the projection ONLY through the bound commitment) |
| **Requirement** | user directive 2026-06-21 (no live CertState read at rebind; no unbound protocol-parameter read; ASC reaches the projection ONLY through the bound commitment) *(paraphrased from source/code_locus; registry `statement` empty)* |
| **Code** | crates/ade_ledger/src/reduced_epoch_view.rs: EpochConsensusView::to_pool_distr_view(genesis_hash, protocol_params_hash, asc) (verify commitment -> require is_leadership_complete -> build PoolDistrView/PoolEntry keyed by Hash28(pool.0)); ProjectionError{ParamsCommitmentMismatch, NotLeadershipComplete}. ci/ci_check_eview_leadership_complete.sh. |
| **Tests** | `to_pool_distr_view_builds_from_bound_profile_and_rejects_wrong_params`; `projection_rejects_wrong_profile_through_the_real_derive_path` |
| **CI** | `ci/ci_check_eview_leadership_complete.sh` |

#### `DC-EPOCH-13` — _enforced_

| Aspect | Location |
|--------|----------|
| **Source** | docs/clusters/EPOCH-CONSENSUS-VIEW/SLICE-ECA-1-remove-activation-gate.md; user directive 2026-06-21 (a build/runtime flag deciding whether a consensus transition occurs is a forbidden semantic gate; activation must be automatic + deterministic from canonical state; the predicate is the only gate) |
| **Requirement** | user directive 2026-06-21 (a build/runtime flag deciding whether a consensus transition occurs is a forbidden semantic gate; activation must be automatic + deterministic from canonical state; the predicate is the only gate) *(paraphrased from source/code_locus; registry `statement` empty)* |
| **Code** | crates/ade_node/src/epoch_wire.rs: maybe_activate_first_boundary (no `armed` param; gates on the era_schedule.locate boundary detection + the idempotent active_view.promoted() check; runs try_activate_at_boundary -> the predicate); EviewActivationInputs::maybe_activate (no `armed` param). crates/ade_node/src/node_lifecycle.rs: maybe_activate_epoch_boundary keyed on (Some(inputs), Some(live)) = canonical state, not a flag. ci/ci_check_eview_automatic_activation.sh. |
| **Tests** | `maybe_activate_first_boundary_is_automatic_and_fails_closed_not_flag_gated` |
| **CI** | `ci/ci_check_eview_automatic_activation.sh` |

#### `DC-EPOCH-14` — _enforced_

| Aspect | Location |
|--------|----------|
| **Source** | docs/clusters/EPOCH-CONSENSUS-VIEW/SLICE-ECA-2-3-4-atomic-epoch-authority-transition.md; user directive 2026-06-22 (ECA-2/3/4 ship as ONE mergeable atomic-epoch-authority-transition slice -- construct deterministic inputs -> durably record activation -> atomically publish the authority -> recover the same authority after crash is ONE authoritative state transition; ECA-4 recovery is the second half of the authority contract, not cleanup); user 2026-06-23 (the slot-aware bidirectional +… |
| **Requirement** | user directive 2026-06-22 (ECA-2/3/4 ship as ONE mergeable atomic-epoch-authority-transition slice -- construct deterministic inputs -> durably record activation -> atomically publish the authority -> recover the same authority after… *(paraphrased from source/code_locus; registry `statement` empty)* |
| **Code** | crates/ade_node/src/epoch_activation.rs: ActiveEpochAuthority (the one holder; ledger_view/pool_distr_view resolved fresh; promote() the sole mutation path), EpochAuthorityMode (SeedOnly\|ContinuityRequired, established from durable state) + guard_epoch -> AuthorityEpochVerdict, active_view_identity, recover_active_view / activation_record_matches / resolve_activation_record. crates/ade_node/src/epoch_activate.rs: activate_at_boundary (live), recover_at_boundary (warm-start re-derive + recover, reject-non-recomputable). crates/ade_node/src/epoch_wire.rs: try_recover_at_boundary, maybe_recover_promoted_authority. crates/ade_node/src/node_sync.rs: forge_one_from_recovered (reads… |
| **Tests** | `authority_epoch_guard_is_mode_aware_and_identity_is_exact`; `cross_consumer_identity_validation_and_forge_resolve_one_authority_view`; `seed_only_sole_view_cannot_validate_n1_header_rejects_before_acceptance`; `forge_continuity_required_missing_promotion_at_n1_is_terminal`; `node_forge_off_epoch_slot_fails_closed`; `recover_at_boundary_round_trips_the_durable_record_and_rejects_a_tamper`; `happy_path_promotes_after_durable_wal`; `crash_before_durable_wal_keeps_seed`; `crash_after_wal_republishes_same_view`; `recovered_view_mismatch_is_terminal`; … (+1 more) |
| **CI** | `ci/ci_check_eview_atomic_authority.sh` |

### DC-QUERY

#### `DC-QUERY-01` — _declared_

| Aspect | Location |
|--------|----------|
| **Source** | Project constitution §3, T-DET-01, DC-PROTO-04 |
| **Requirement** | N2C queries are era-aware, typed, and version-gated: each NodeToClientVersion gates which queries are available |
| **Code** | _(no enforcement found — gap)_ |
| **Tests** | _(no tests listed — gap)_ |
| **CI** | _(no CI gate — gap)_ |

### DC-NET

#### `DC-NET-01` — _declared_

| Aspect | Location |
|--------|----------|
| **Source** | Project constitution §3, T-RESOURCE-01, T-TRANSPORT-01 |
| **Requirement** | Peer selection uses three-tier management (cold/warm/hot) with bounded admission, per-peer resource limits, and eviction policies |
| **Code** | _(no enforcement found — gap)_ |
| **Tests** | _(no tests listed — gap)_ |
| **CI** | _(no CI gate — gap)_ |

### DC-DIFF

#### `DC-DIFF-01` — _partial_

| Aspect | Location |
|--------|----------|
| **Source** | Project constitution §3, T-DET-01, DC-REF-01 |
| **Requirement** | Differential harness must localize first divergence point between Ade and reference oracle |
| **Code** | crates/ade_testkit/src/harness/ledger_diff.rs |
| **Tests** | `diff_ledger_sequence`; `mismatched_block_count_returns_error` |
| **CI** | _(no CI gate — gap)_ |

### DC-MEM

#### `DC-MEM-01` — _enforced_

| Aspect | Location |
|--------|----------|
| **Source** | Project constitution §3, T-DET-01, T-CORE-01 |
| **Requirement** | Mempool acceptance rules must not contradict block/ledger acceptance rules |
| **Code** | crates/ade_ledger/src/mempool/admit.rs; crates/ade_ledger/src/mempool/ingress.rs; crates/ade_testkit/src/mempool/ingress_replay.rs |
| **Tests** | `valid_tx_admitted_and_accumulates`; `invalid_tx_rejected_no_false_accept`; `admission_equals_tx_validity_verdict`; `dependent_tx_admitted_against_accumulating_state`; `ingress_admit_equals_direct_admit_on_b_track_corpus`; `b_track_adversarial_rejections_preserved_through_ingress`; `dependent_pair_through_ingress_admits_b_after_a` |
| **CI** | `ci/ci_check_consensus_closed_enums.sh` |

#### `DC-MEM-02` — _enforced_

| Aspect | Location |
|--------|----------|
| **Source** | Project constitution §3, T-DET-01, T-CORE-02 |
| **Requirement** | Overload shedding follows deterministic policy, not timing-dependent collapse |
| **Code** | crates/ade_ledger/src/mempool/policy.rs |
| **Tests** | `policy_does_not_change_validity`; `determinism` |
| **CI** | `ci/ci_check_consensus_closed_enums.sh` |

#### `DC-MEM-03` — _enforced_

| Aspect | Location |
|--------|----------|
| **Source** | Project constitution §3, T-CORE-01 (closed semantic surfaces), DC-MEM-01 |
| **Requirement** | Tx ingress reduces to a closed IngressEvent before BLUE mempool admission; the source variant is evidence/policy/replay metadata only and MUST NOT change the validity verdict. |
| **Code** | crates/ade_ledger/src/mempool/ingress.rs (IngressEvent, IngressSource, mempool_ingress) |
| **Tests** | `ingress_preserves_tx_bytes_verbatim`; `ingress_source_is_closed_two_variants`; `ingress_admits_valid_tx_via_n2n`; `ingress_admits_valid_tx_via_n2c`; `ingress_rejects_invalid_tx_no_false_accept`; `ingress_source_does_not_change_verdict_valid`; `ingress_source_does_not_change_verdict_adversarial`; `ingress_equals_direct_admit_on_synthetic_corpus` |
| **CI** | `ci/ci_check_mempool_ingress_closure.sh` |

#### `DC-MEM-04` — _enforced_

| Aspect | Location |
|--------|----------|
| **Source** | Project constitution §3, T-DET-01, DC-MEM-01 |
| **Requirement** | Replaying the same ordered ingress trace against the same base ledger state produces a byte-identical sequence of (MempoolState, AdmitOutcome) pairs. |
| **Code** | crates/ade_testkit/src/mempool/ingress_replay.rs; crates/ade_ledger/src/mempool/ingress.rs; crates/ade_ledger/src/mempool/canonicalize.rs |
| **Tests** | `ingress_admit_equals_direct_admit_on_b_track_corpus`; `b_track_adversarial_rejections_preserved_through_ingress`; `ingress_trace_replay_byte_identical`; `dependent_pair_through_ingress_admits_b_after_a`; `ingress_trace_source_invariant_n2n_vs_n2c`; `multi_peer_round_robin_by_sorted_peer_id`; `unsorted_input_canonicalizes_identically_to_sorted_input`; `two_interleavings_replay_byte_identical` |
| **CI** | `ci/ci_check_mempool_ingress_replay.sh` |

#### `DC-MEM-05` — _declared_

| Aspect | Location |
|--------|----------|
| **Source** | Project constitution §3 (replay is the integration contract); MEM-OPT cluster plan (docs/planning/mem-opt-cluster-plan.md); DC-WAL-03 |
| **Requirement** | The UTxO/ledger state fingerprint and post-state are independent of the UTxO storage backend: an in-memory UTxO and an on-disk UTxO produce byte-identical replay (same WAL + checkpoint => same tail fingerprint). A memory-representation/storage change is NEVER a consensus or replay change. |
| **Code** | _(no enforcement found — gap)_ |
| **Tests** | _(no tests listed — gap)_ |
| **CI** | _(no CI gate — gap)_ |

#### `DC-MEM-06` — _partial_

| Aspect | Location |
|--------|----------|
| **Source** | Project constitution §4 (determinism); RFC 8949 §4.2.1; MEM-OPT cluster plan |
| **Requirement** | The UTxO/ledger state fingerprint is computed by the canonical CBOR encoder over canonically-encoded (fixed-width big-endian) keys, NEVER from a storage backend's native iteration order, AND is independent of the process memory allocator (allocation addresses/sizes are never fingerprinted). Store iteration order and the allocator are implementation details and must not enter any authoritative fingerprint. |
| **Code** | ci/ci_check_alloc_determinism_neutral.sh; crates/ade_node/src/main.rs; crates/ade_runtime/src/seed_import/importer.rs |
| **Tests** | `streaming_matches_whole_buffer_across_fixtures`; `streaming_fingerprint_independent_of_textual_order`; `streaming_surfaces_conversion_error_not_swallowed`; `streaming_rejects_duplicate_txin_fail_closed`; `streaming_rejects_exact_duplicate_string_key_but_oracle_collapses` |
| **CI** | `ci/ci_check_alloc_determinism_neutral.sh; ci/ci_check_mem_opt_s2_import_peak.sh` |

#### `DC-MEM-07` — _partial_

| Aspect | Location |
|--------|----------|
| **Source** | classification_table.md §H; MEM-OPT cluster plan |
| **Requirement** | The in-memory portion of the UTxO (read cache + last-k changelog) is bounded by fixed, closed, non-configurable constants; memory pressure cannot grow it unboundedly, and the bound never changes an authoritative output. |
| **Code** | crates/ade_ledger/src/utxo_overlay.rs; ci/ci_check_overlay_utxo_s2a.sh |
| **Tests** | `overlay_matches_btreemap_across_a_sequence`; `compact_preserves_effective_set_and_clears_overlay`; `clone_shares_anchor_and_is_independent`; `s2a_overlay_split_fingerprints_identically_to_direct_build` |
| **CI** | `ci/ci_check_overlay_utxo_s2a.sh` |

#### `DC-MEM-08` — _declared_

| Aspect | Location |
|--------|----------|
| **Source** | Project constitution §3 (canonical serialization); MEM-OPT cluster plan |
| **Requirement** | A compact UTxO/TxOut representation (canonical CBOR slice as the single source of truth + lazily-decoded views) preserves canonical bytes and ledger semantics: the value a ledger rule reads, and the bytes the fingerprint sees, are identical to the fully-parsed representation. |
| **Code** | _(no enforcement found — gap)_ |
| **Tests** | _(no tests listed — gap)_ |
| **CI** | _(no CI gate — gap)_ |

#### `DC-MEM-09` — _enforced_

| Aspect | Location |
|--------|----------|
| **Source** | MEM-OPT-UTXO-DISK S1 (docs/clusters/MEM-OPT-UTXO-DISK/S1-interface.md); DC-MEM-05 |
| **Requirement** | The authoritative UTxO lookup interface returns OWNED values (Option<TxOut>), never a borrow into storage. This is the precondition for a swappable UTxO backend (DC-MEM-05): a resolved output is materialized BY VALUE, so an on-disk backend can serve it without leaking storage lifetimes into the validity rules. Changing the lookup to owned MUST NOT alter any verdict, fingerprint, or failure shape. |
| **Code** | crates/ade_ledger/src/utxo.rs; crates/ade_ledger/src/phase.rs; crates/ade_ledger/src/tx_validity/phase1.rs; ci/ci_check_utxo_lookup_owned.sh |
| **Tests** | `owned_lookup_returns_stored_value_and_does_not_mutate` |
| **CI** | `ci/ci_check_utxo_lookup_owned.sh` |

#### `DC-MEM-10` — _enforced_

| Aspect | Location |
|--------|----------|
| **Source** | MEM-OPT-UTXO-DISK S1.5 (docs/clusters/MEM-OPT-UTXO-DISK/S1.5-fp-v2-incremental.md); DC-MEM-05; OQ-UD-3 |
| **Requirement** | The v2 UTxO fingerprint component is a NAMED commutative set commitment (Ristretto255 ECMH) binding (TxIn, TxOut) over the canonical encodings, domain-separated and version-tagged (fingerprint_version: v1 vs v2 are EXPLICIT and never silently mixed). It enables an O(delta)/block post_fp (the prerequisite for the on-disk UTxO backend, DC-MEM-05). post_fp remains the full-state replay hash ('state after this block'); only the UTxO-component construction + version change. Per-block incremental maintenance MUST equal the full recompute. Internal replay contract only -- no peer-facing/Cardano-consensus change. NOT a naive XOR/sum. |
| **Code** | crates/ade_crypto/src/utxo_set_commitment.rs; crates/ade_ledger/src/fingerprint.rs; crates/ade_runtime/src/chaindb/persistent.rs; crates/ade_runtime/src/chaindb/error.rs; ci/ci_check_utxo_fp_v2.sh |
| **Tests** | `order_independent`; `add_remove_is_exact_inverse`; `binds_value_not_just_key`; `golden_empty_digest`; `golden_single_entry_digest`; `golden_two_entry_digest`; `v1_and_v2_utxo_components_differ_only_the_utxo_changes`; `fingerprint_v2_is_deterministic`; `fingerprint_v2_utxo_is_insertion_order_independent`; `incremental_v2_equals_full_recompute_after_each_block`; … (+2 more) |
| **CI** | `ci/ci_check_utxo_fp_v2.sh` |

#### `DC-MEM-11` — _enforced_

| Aspect | Location |
|--------|----------|
| **Source** | docs/active/live-follow-throughput-handoff.md (C2-PREVIEW-BA02: forge per-block admit ~20s CPU/block @ 99.8% CPU -- the producer kept pace with the chain but could never CLOSE the catch-up backlog, so it never reached the live tip / a live leader slot) |
| **Requirement** | docs/active/live-follow-throughput-handoff.md (C2-PREVIEW-BA02: forge per-block admit ~20s CPU/block @ 99.8% CPU -- the producer kept pace with the chain but could never CLOSE the catch-up backlog, so it never reached the live tip / a… *(paraphrased from source/code_locus; registry `statement` empty)* |
| **Code** | crates/ade_runtime/src/forward_sync/reducer.rs (ForwardSyncState.utxo_fp_cache; post_fp via fingerprint_v2_with_utxo + utxo_fp_cache.utxo_fingerprint; ForwardSyncState::invalidate_utxo_fp_cache); crates/ade_node/src/node_lifecycle.rs (emit_participant_admit reuses state.prior_fp; the RolledBack arm calls invalidate_utxo_fp_cache after commit_rollback); ci/ci_check_forward_sync_fp_cache.sh |
| **Tests** | `pump_block_post_fp_is_byte_identical_to_full_fingerprint`; `forward_sync_post_fp_cache_hit_is_byte_identical`; `forward_sync_replay_two_runs_byte_identical`; `forward_sync_admission_through_chokepoints` |
| **CI** | `ci/ci_check_forward_sync_fp_cache.sh` |

### DC-CORE

#### `DC-CORE-01` — _enforced_

| Aspect | Location |
|--------|----------|
| **Source** | Project constitution §2, T-CORE-02; PHASE4-N-A scope decisions §Decision 2 (docs/active/PHASE4-N-A_scope_decisions.md) |
| **Requirement** | BLUE authoritative crates are sync-only: no async fn, .await, tokio::, async_std::, Future, futures::, task spawning, async channels, or timers. Async runtime concerns are confined to RED transport/runtime code. |
| **Code** | crates/ade_runtime/src/consensus/chain_selector.rs, crates/ade_core_interop/src/lib.rs, crates/ade_core_interop/src/bin/live_consensus_session.rs |
| **Tests** | `header_arrival_updates_state_and_selector`; `rollback_walks_back_via_recent_snapshots`; `rollback_to_block_older_than_snapshots_rejected`; `epoch_boundary_emits_no_event`; `cardano_node_session_sustained_window` |
| **CI** | `ci/ci_check_no_async_in_blue.sh` |

### DC-CONS

#### `DC-CONS-03` — _enforced_

| Aspect | Location |
|--------|----------|
| **Source** | Project constitution §3, ouroboros-consensus (pin at code-lock); CN-CONS-01 |
| **Requirement** | Praos chain selection ordering: block number first, then Praos TiebreakerView (slot, issuer, op-cert issue number, VRF output). Density-based ordering is reserved for Genesis/catch-up and must not be used for caught-up Praos fork-choice. |
| **Code** | crates/ade_core/src/consensus/fork_choice.rs, crates/ade_core/src/consensus/candidate.rs, crates/ade_runtime/src/consensus/candidate_fragment.rs |
| **Tests** | `tiebreaker_prefer_lower_slot_wins`; `tiebreaker_prefer_higher_op_cert_wins_on_equal_slot_and_issuer`; `tiebreaker_prefer_lower_vrf_value_wins_on_full_tie`; `no_candidates_returns_no_candidates_error`; `equal_to_current_keeps_current_via_tiebreaker_loss`; `tiebreaker_view_eq_is_field_wise`; `candidate_fragment_carries_anchor_block_no`; `higher_block_no_wins`; `equal_block_no_tiebreaker_decides`; `fork_before_immutable_tip_rejected`; … (+5 more) |
| **CI** | `ci/ci_check_no_density_in_fork_choice.sh`; `ci/ci_check_no_chaindb_in_consensus_blue.sh`; `ci/ci_check_no_float_in_consensus.sh`; `ci/ci_check_consensus_closed_enums.sh` |

#### `DC-CONS-04` — _enforced_

| Aspect | Location |
|--------|----------|
| **Source** | Project constitution §3, ouroboros-consensus PraosChainDepState |
| **Requirement** | Praos chain-dep state (evolving/candidate/epoch/previous_epoch/lab/last_epoch_block nonces, op-cert counters, last_slot) is owned by N-B consensus, not by the ledger, and evolves deterministically as a function of validated headers and epoch boundaries. |
| **Code** | crates/ade_core/src/consensus/praos_state.rs, crates/ade_core/src/consensus/events.rs, crates/ade_core/src/consensus/errors.rs, crates/ade_core/src/consensus/encoding.rs, crates/ade_core/src/consensus/nonce.rs, crates/ade_core/src/consensus/op_cert.rs, crates/ade_core/src/consensus/header_validate.rs, crates/ade_core/src/consensus/header_summary.rs |
| **Tests** | `op_cert_upsert_rejects_regression`; `op_cert_upsert_accepts_equal_counter_as_noop`; `op_cert_upsert_accepts_monotonic_increasing`; `genesis_state_is_well_formed`; `nonce_zero_constant_is_zero_bytes`; `op_cert_counter_map_iteration_is_deterministic`; `layout_is_stable`; `roundtrip_empty_state`; `roundtrip_genesis_state`; `roundtrip_populated_state`; … (+39 more) |
| **CI** | `ci/ci_check_consensus_closed_enums.sh` |

#### `DC-CONS-05` — _enforced_

| Aspect | Location |
|--------|----------|
| **Source** | Project constitution §3, ouroboros-consensus consensus report |
| **Requirement** | Authoritative rollback must never exceed the security parameter k measured in blocks (mainnet k = 2160). Rollback requests deeper than k return ExceededRollback. Forecast/stability windows may be slot-based but must accommodate at least k+1 blocks. |
| **Code** | crates/ade_core/src/consensus/rollback.rs, crates/ade_core/src/consensus/events.rs |
| **Tests** | `rollback_preserves_immutable_tip`; `rollback_preserves_security_param`; `rollback_with_zero_depth_is_noop`; `rollback_to_equal_block_no_as_immutable_succeeds`; `rollback_to_one_below_immutable_rejected`; `rollback_within_k_succeeds`; `rollback_exceeding_k_rejected_with_typed_reason`; `rollback_before_immutable_tip_rejected`; `rollback_event_bytes_are_stable`; `rollback_is_deterministic`; … (+1 more) |
| **CI** | `ci/ci_check_consensus_closed_enums.sh`; `ci/ci_check_no_chaindb_in_consensus_blue.sh` |

#### `DC-CONS-06` — _enforced_

| Aspect | Location |
|--------|----------|
| **Source** | Project constitution §3, T-DET-01 |
| **Requirement** | rollback(state, depth) produces state byte-identical to truncated replay from the nearest checkpoint. Rollback that would cross the immutable tip (≥ k deep) returns ForkBeforeImmutableTip and never alters state. |
| **Code** | crates/ade_core/src/consensus/rollback.rs |
| **Tests** | `rollback_equivalent_to_truncated_replay`; `rollback_is_deterministic`; `rollback_within_k_succeeds`; `rollback_before_immutable_tip_rejected`; `rollback_event_bytes_are_stable`; `rollback_to_one_below_immutable_rejected`; `rollback_preserves_immutable_tip` |
| **CI** | `ci/ci_check_consensus_closed_enums.sh` |

#### `DC-CONS-07` — _enforced_

| Aspect | Location |
|--------|----------|
| **Source** | Project constitution §3, DC-CORE-01, DC-EPOCH-02 |
| **Requirement** | BLUE consensus must consume the HFC schedule only as a typed EraSchedule value anchored to BootstrapAnchorHash. Genesis text parsing happens in RED; BLUE never reads files, JSON, or operator config directly. The schedule is part of replay evidence. |
| **Code** | crates/ade_core/src/consensus/era_schedule.rs, crates/ade_runtime/src/consensus/genesis_parser.rs |
| **Tests** | `eraschedule_constructor_rejects_empty`; `eraschedule_constructor_rejects_non_monotonic`; `eraschedule_constructor_rejects_zero_slot_length`; `eraschedule_constructor_rejects_zero_epoch_length`; `anchor_hash_deterministic`; `anchor_hash_distinguishes_inputs`; `mainnet_corpus_translation_matches_oracle`; `preprod_corpus_translation_matches_oracle`; `bootstrap_anchor_hash_distinguishes_genesis_variants`; `mainnet_parser_eras_match_corpus_oracle`; … (+2 more) |
| **CI** | `ci/ci_check_no_chaindb_in_consensus_blue.sh` |

#### `DC-CONS-08` — _enforced_

| Aspect | Location |
|--------|----------|
| **Source** | Project constitution §3, CN-CONS-05, DC-CORE-01 |
| **Requirement** | slot_to_time(EraSchedule, SystemStart, SlotNo) is a pure function; no BLUE consensus path may consult the wall clock to derive a slot or UTC instant for an authoritative decision. |
| **Code** | crates/ade_core/src/consensus/era_schedule.rs |
| **Tests** | `slot_to_time_monotone_increasing`; `slot_to_time_overflow_returns_structured_error`; `determinism_across_runs`; `locate_first_slot_of_each_era`; `locate_last_slot_of_each_era`; `locate_before_system_start_errors`; `mainnet_corpus_translation_matches_oracle`; `preprod_corpus_translation_matches_oracle` |
| **CI** | `ci/ci_check_no_float_in_consensus.sh` |

#### `DC-CONS-09` — _enforced_

| Aspect | Location |
|--------|----------|
| **Source** | Project constitution §3, DC-EPOCH-02 |
| **Requirement** | Consensus-derived queries for slots beyond the ledger-view safe zone return OutsideForecastRange, never guessed values. The bound is derived from era history + safe zone + HFC schedule, not encoded as a magic constant in caller code. |
| **Code** | crates/ade_core/src/consensus/era_schedule.rs, crates/ade_core/src/consensus/leader_schedule.rs |
| **Tests** | `forecast_horizon_boundary`; `mainnet_corpus_translation_matches_oracle`; `preprod_corpus_translation_matches_oracle`; `query_returns_outside_forecast_range_for_far_future`; `corpus_rejects_out_of_forecast_horizon` |
| **CI** | `ci/ci_check_consensus_closed_enums.sh` |

#### `DC-CONS-10` — _enforced_

| Aspect | Location |
|--------|----------|
| **Source** | Project constitution §3, ouroboros-consensus OperationalCertificate |
| **Requirement** | A header's op-cert issue counter must be >= the highest observed counter for the same (pool, kes_period). Regression yields HeaderInvalid with a typed OpCertCounterError reason; ChainDepState never accepts a regression. |
| **Code** | crates/ade_core/src/consensus/op_cert.rs, crates/ade_core/src/consensus/praos_state.rs, crates/ade_core/src/consensus/errors.rs, crates/ade_core/src/consensus/header_validate.rs |
| **Tests** | `apply_op_cert_inserts_first_observation`; `apply_op_cert_advances_existing_strictly`; `apply_op_cert_accepts_equal_counter_as_noop`; `apply_op_cert_rejects_lower_counter`; `apply_op_cert_independent_kes_periods_dont_collide`; `apply_op_cert_independent_pools_dont_collide`; `apply_op_cert_does_not_touch_nonces`; `apply_op_cert_does_not_touch_last_slot_or_block_no`; `op_cert_upsert_rejects_regression`; `op_cert_upsert_accepts_equal_counter_as_noop`; … (+6 more) |
| **CI** | `ci/ci_check_consensus_closed_enums.sh` |

#### `DC-CONS-11` — _enforced_

| Aspect | Location |
|--------|----------|
| **Source** | docs/planning/phase4-n-c-invariants.md §1 (NC-KES-3/4); Praos / Shelley operational-certificate specification |
| **Requirement** | OpCert kes_period field equals the KES period at the forged slot under an operator-supplied anchor. period_at_slot(slot, anchor) = (slot - anchor) / slots_per_kes_period (integer floor). BLUE rejects header/opcert combinations with mismatched periods at forge time. BLUE MUST NOT infer the anchor from wall-clock or filesystem state; the anchor is a pure input on the canonical ProducerTick. |
| **Code** | crates/ade_core/src/consensus/opcert_validate.rs (opcert_validate, OpCertError); crates/ade_codec/src/shelley/opcert.rs (encode_opcert, decode_opcert, OpCertCodecError) |
| **Tests** | `opcert_validate_accepts_canonical_fixture`; `opcert_validate_rejects_period_mismatch`; `opcert_validate_rejects_short_hot_vkey`; `opcert_validate_first_opcert_no_prev_counter`; `opcert_encoder_matches_cardano_cli_byte_identical`; `opcert_round_trip_byte_identical` |
| **CI** | `ci/ci_check_opcert_closed.sh` |

#### `DC-CONS-12` — _enforced_

| Aspect | Location |
|--------|----------|
| **Source** | docs/planning/phase4-n-c-invariants.md §1 (NC-OC-2); Cardano operational-certificate counter discipline |
| **Requirement** | OpCert serial counter is strictly monotonically increasing per (cold-key, node). BLUE rejects regression or repetition at the RED->BLUE boundary: opcert_validate fails when prev_counter is Some(c) and opcert.counter <= c. RED feeds the value from durable per-node state; BLUE never trusts an in-memory-only counter. |
| **Code** | crates/ade_core/src/consensus/opcert_validate.rs (opcert_validate, OpCertError::{CounterRepeat, CounterRegression, BadColdSignature}) |
| **Tests** | `opcert_validate_rejects_counter_regression`; `opcert_validate_rejects_counter_repeat`; `opcert_validate_rejects_bad_signature_over_cold_key` |
| **CI** | `ci/ci_check_opcert_closed.sh` |

#### `DC-CONS-13` — _enforced_

| Aspect | Location |
|--------|----------|
| **Source** | docs/planning/phase4-n-c-invariants.md §1 (NC-FORGE-1); Project constitution §2 (T-DET-01, Functional Core / Imperative Shell) |
| **Requirement** | Forge is pure given a canonical ProducerTick. forge_block has no wall-clock, no rand, no HashMap iteration, no I/O, no locale, and no ambient state. All inputs flow through the ProducerTick value (slot, ledger_state, mempool_snapshot, pparams, vrf_proof, kes_sig, opcert). Strengthens T-DET-01 for the producer authority surface. |
| **Code** | crates/ade_ledger/src/producer/forge.rs (forge_block); crates/ade_ledger/src/producer/state.rs (ProducerTick); crates/ade_ledger/src/receive/reducer.rs (PHASE4-N-H strengthening: symmetric receive-side closure — admit = block_validity::Valid only, never a parallel path) |
| **Tests** | `forge_block_pure_no_io`; `forge_block_replay_byte_identical`; `receive_apply_block_delivered_with_matching_header_admits` |
| **CI** | `ci/ci_check_forge_purity.sh`; `ci/ci_check_receive_reducer_closure.sh` |

#### `DC-CONS-14` — _enforced_

| Aspect | Location |
|--------|----------|
| **Source** | docs/planning/phase4-n-c-invariants.md §1 (NC-FORGE-2), §4 |
| **Requirement** | Forge byte-equality across replays. For two replays of an identical canonical ProducerTick stream over the same initial LedgerState, forge_block produces a byte-identical Vec<ForgedBlockBytes>. Replay uses captured signed artifacts (vrf_proof, kes_sig, opcert) and MUST NOT invoke RED signing; private-key material does not appear in replay corpora. |
| **Code** | crates/ade_ledger/src/producer/forge.rs; crates/ade_testkit/src/producer/replay.rs (producer_replay_fixtures); crates/ade_testkit/src/producer/fixtures.rs |
| **Tests** | `forge_block_replay_byte_identical` |
| **CI** | `ci/ci_check_forge_purity.sh`; `ci/ci_check_no_private_keys_in_corpus.sh` |

#### `DC-CONS-15` — _enforced_

| Aspect | Location |
|--------|----------|
| **Source** | docs/planning/phase4-n-c-invariants.md §1 (NC-FORGE-3, NC-VRF-3); Praos / ouroboros-consensus leader-check |
| **Requirement** | Forge is invoked only when leader-check passes. forge_block is a forbidden transition for ticks where is_leader(state, vrf_output, sigma, asc) == false at tick.slot. The producer uses the same is_leader / check_leader_claim functions the validator uses; no producer-side fork of the leader-check formula is permitted. |
| **Code** | crates/ade_ledger/src/producer/forge.rs (leader-check gate); crates/ade_core/src/consensus/leader_schedule.rs (is_leader_for_vrf_output — shared with validator) |
| **Tests** | `forge_block_rejects_non_leader_tick`; `forge_block_uses_validator_leader_check_function` |
| **CI** | `ci/ci_check_forge_purity.sh` |

#### `DC-CONS-16` — _enforced_

| Aspect | Location |
|--------|----------|
| **Source** | docs/planning/phase4-n-c-invariants.md §1 (NC-FORGE-5); Project constitution §2 (T-ENC-01, Byte Authority Model) |
| **Requirement** | Forged header.body_hash MUST equal blake2b_256(forged_body_wire_bytes), where forged_body_wire_bytes are produced by the single Cardano-compatible canonical block-body encoder used by the validator hash path. The producer and validator hash the same bytes through the same encoder. Strengthens T-ENC-01 for the producer surface; closes any potential producer/validator encoder bifurcation. |
| **Code** | crates/ade_ledger/src/block_body_hash.rs (block_body_hash, block_body_hash_from_buckets — single canonical authority); crates/ade_ledger/src/producer/forge.rs (forge_block — consumer); crates/ade_ledger/src/block_validity/header_input.rs (computed_body_hash + accepted_block_header_bytes — consumer + N-G-strengthened single header-projection authority); crates/ade_runtime/src/producer/served_chain_lookups.rs (ServedHeaderLookup::next_after — third consumer; reuses accepted_block_header_bytes for producer-side server-pump header projection); crates/ade_ledger/src/receive/reducer.rs (PHASE4-N-H strengthening: receive-side BlockDelivered branch decodes block via the same block_validity… |
| **Tests** | `block_body_hash_pinned_recipe_byte_identical`; `block_body_hash_from_block_equals_from_buckets`; `block_body_hash_none_invalid_txs_equals_empty_bucket`; `forged_body_hash_matches_validator_recomputation`; `body_encoder_is_single_authority`; `accepted_block_header_bytes_equals_validator_split_on_corpus`; `accepted_block_header_bytes_is_subslice_of_as_bytes`; `session_transcript_announced_header_matches_served_body_recipe`; `receive_apply_block_delivered_with_matching_header_admits`; `receive_pipeline_corpus_drive_admitted_bytes_equal_corpus_bytes` |
| **CI** | `ci/ci_check_no_producer_body_encoder.sh`; `ci/ci_check_no_parallel_header_splitter.sh`; `ci/ci_check_receive_reducer_closure.sh` |

#### `DC-CONS-17` — _enforced_

| Aspect | Location |
|--------|----------|
| **Source** | docs/planning/phase4-n-a-successor-invariants.md §1 (I-1) |
| **Requirement** | Block bytes delivered via producer-side block-fetch Block{bytes} are byte-identical to AcceptedBlock.as_bytes() for the AcceptedBlock that cleared self_accept. The producer-side server never re-encodes. |
| **Code** | crates/ade_network/src/block_fetch/server.rs (producer_block_fetch_serve constructs Block{bytes} only from served.range_bytes outputs); crates/ade_runtime/src/producer/served_chain_lookups.rs (ServedRangeLookup impl forwards ServedChainSnapshot::range_bytes which yields AcceptedBlock-derived slices); crates/ade_ledger/src/producer/served_chain.rs (ServedChainSnapshot.block_bytes returns AcceptedBlock.as_bytes() verbatim) |
| **Tests** | `producer_block_fetch_serve_block_bytes_equal_accepted_block_as_bytes`; `session_transcript_served_block_bytes_equal_admitted_accepted_block_bytes`; `n_r_b_partial_overlap_from_endpoint_not_in_snapshot_yields_no_blocks`; `n_r_b_partial_overlap_to_endpoint_not_in_snapshot_yields_no_blocks`; `n_r_b_partial_overlap_both_endpoints_fabricated_yields_no_blocks` |
| **CI** | `ci/ci_check_block_fetch_server_closure.sh`; `ci/ci_check_broadcast_to_served_purity.sh` |

#### `DC-CONS-18` — _enforced_

| Aspect | Location |
|--------|----------|
| **Source** | docs/planning/phase4-n-a-successor-invariants.md §1 (I-2) |
| **Requirement** | Header bytes announced via chain-sync RollForward{header,tip} are the header sub-segment of the AcceptedBlock whose body bytes are subsequently servable via block-fetch. block_body_hash applied to the served body MUST equal the body-hash field of the announced header. |
| **Code** | crates/ade_ledger/src/block_validity/header_input.rs (accepted_block_header_bytes — single canonical projection); crates/ade_runtime/src/producer/served_chain_lookups.rs (ServedHeaderLookup::next_after uses accepted_block_header_bytes); crates/ade_network/src/chain_sync/server.rs (producer_chain_sync_serve sources RollForward header from the trait lookup) |
| **Tests** | `producer_chain_sync_serve_roll_forward_header_equals_accepted_block_header_bytes`; `session_transcript_announced_header_matches_served_body_recipe`; `accepted_block_header_bytes_equals_validator_split_on_corpus`; `forge_block_accepts_empty_mempool`; `unsigned_header_preimage_matches_decode_block_extraction_for_corpus` |
| **CI** | `ci/ci_check_no_parallel_header_splitter.sh`; `ci/ci_check_broadcast_to_served_purity.sh`; `ci/ci_check_unsigned_header_preimage_single_source.sh` |

#### `DC-CONS-19` — _enforced_

| Aspect | Location |
|--------|----------|
| **Source** | docs/planning/receive-side-bridge-invariants.md §1 (I-2) |
| **Requirement** | docs/planning/receive-side-bridge-invariants.md §1 (I-2) *(paraphrased from source/code_locus; registry `statement` empty)* |
| **Code** | crates/ade_ledger/src/receive/reducer.rs (block_delivered helper: decodes the body, looks up cache at (slot, block_hash); HeaderBodyMismatch if absent) |
| **Tests** | `receive_apply_block_delivered_with_no_cached_header_rejects`; `receive_apply_block_delivered_with_mismatched_cached_header_rejects`; `receive_apply_block_delivered_with_matching_header_admits` |
| **CI** | `ci/ci_check_receive_reducer_closure.sh` |

#### `DC-CONS-20` — _enforced_

| Aspect | Location |
|--------|----------|
| **Source** | docs/planning/receive-side-bridge-invariants.md §1 (I-3, I-4); docs/planning/ledger-snapshot-rollback-invariants.md §1 (I-6) |
| **Requirement** | docs/planning/ledger-snapshot-rollback-invariants.md §1 (I-6) *(paraphrased from source/code_locus; registry `statement` empty)* |
| **Code** | crates/ade_ledger/src/receive/reducer.rs (admit-side: block_delivered branch atomically advances chain_write + state.ledger + state.chain_dep; rollback-side: roll_backward branch atomically calls materialize_rolled_back_state + commit_rollback); crates/ade_ledger/src/rollback/commit.rs (commit_rollback irreversible-step-first staged commit); crates/ade_ledger/src/rollback/materialize.rs (materialize_rolled_back_state via SnapshotReader + BlockSource); crates/ade_runtime/src/rollback/{cadence,in_memory_cache,chaindb_block_source,snapshot_writer}.rs (GREEN/RED rollback infrastructure) |
| **Tests** | `receive_apply_block_delivered_with_matching_header_admits`; `commit_rollback_advances_chaindb_and_ledger_atomically`; `commit_rollback_chain_write_failure_leaves_state_unchanged`; `commit_rollback_resets_pending_headers`; `rollback_branch_returns_rolled_back_on_in_memory_snapshot`; `rollback_branch_returns_rollback_too_deep_when_no_snapshot`; `rollback_branch_state_unchanged_on_materialize_failure`; `rollback_then_continue_admit_equals_straight_line_admit` |
| **CI** | `ci/ci_check_receive_reducer_closure.sh`; `ci/ci_check_rollback_materialize_closure.sh`; `ci/ci_check_receive_orchestrator_no_producer_dep.sh` |

#### `DC-CONS-21` — _enforced_

| Aspect | Location |
|--------|----------|
| **Source** | docs/planning/ledger-snapshot-rollback-invariants.md §1 (I-1, I-2, I-9) |
| **Requirement** | docs/planning/ledger-snapshot-rollback-invariants.md §1 (I-1, I-2, I-9) *(paraphrased from source/code_locus; registry `statement` empty)* |
| **Code** | crates/ade_ledger/src/snapshot/{framing,ledger,chain_dep,utxo_state,cert_state,epoch_state,gov_state}.rs + crates/ade_runtime/src/rollback/persistent_cache.rs |
| **Tests** | `snapshot_round_trip`; `round_trip_via_fingerprint_combined`; `encode_then_decode_roundtrips_via_fingerprint`; `persistent_cache_capture_then_nearest_le_round_trips`; `persistent_cache_matches_in_memory_cache_semantics` |
| **CI** | `ci/ci_check_snapshot_encoder_closure.sh` |

#### `DC-CONS-22` — _enforced_

| Aspect | Location |
|--------|----------|
| **Source** | docs/planning/ledger-snapshot-rollback-invariants.md §1 (I-3, I-4) |
| **Requirement** | docs/planning/ledger-snapshot-rollback-invariants.md §1 (I-3, I-4) *(paraphrased from source/code_locus; registry `statement` empty)* |
| **Code** | crates/ade_ledger/src/rollback/materialize.rs (materialize_rolled_back_state: pure replay-forward fold over block_validity; epoch boundaries handled implicitly by apply_block_with_verdicts per rules.rs:244-250) |
| **Tests** | `materialize_with_snapshot_at_target_returns_snapshot_state`; `materialize_with_snapshot_below_target_replays_forward`; `materialize_replay_forward_equals_direct_apply`; `materialize_fails_closed_on_invalid_block` |
| **CI** | `ci/ci_check_rollback_materialize_closure.sh` |

#### `DC-CONS-23` — _enforced_

| Aspect | Location |
|--------|----------|
| **Source** | docs/planning/phase4-n-u-forged-block-durability-invariants.md |
| **Requirement** | docs/planning/phase4-n-u-forged-block-durability-invariants.md *(paraphrased from source/code_locus; registry `statement` empty)* |
| **Code** | crates/ade_ledger/src/receive/{reducer,admitted}.rs (extend-only admit_via_block_validity -- reused); crates/ade_ledger/src/block_validity/header_position.rs (prev_hash/position fail-closed -- reused); crates/ade_node/src/node_sync.rs (admit_forged_block_durably -> pump_block, no admit-time fork-choice) |
| **Tests** | `stale_tip_forge_fails_closed` |
| **CI** | `ci/ci_check_forged_durable_admit_via_pump.sh` |

#### `DC-CONS-24` — _enforced_

| Aspect | Location |
|--------|----------|
| **Source** | docs/clusters/PHASE4-N-AE/cluster.md; docs/clusters/PHASE4-N-AE/slices/AE.A.md |
| **Requirement** | docs/clusters/PHASE4-N-AE/cluster.md; docs/clusters/PHASE4-N-AE/slices/AE.A.md *(paraphrased from source/code_locus; registry `statement` empty)* |
| **Code** | crates/ade_node/src/node_lifecycle.rs (run_relay_loop_with_sched ForgeTick CaughtUp arm forges on selected_tip == the durable servable tip == the followed peer tip); crates/ade_node/src/node_sync.rs (forge_header_position sets prev_hash = PrevHash::Block(selected_tip.hash)) |
| **Tests** | `forge_on_followed_tip_proceeds_with_parent_byte_equal` |
| **CI** | `ci/ci_check_forge_followed_tip_admission.sh` |

#### `DC-CONS-IN-01` — _enforced_

| Aspect | Location |
|--------|----------|
| **Source** | docs/planning/phase4-n-m-c-operator-pass-invariants.md §1 (I-C2) |
| **Requirement** | docs/planning/phase4-n-m-c-operator-pass-invariants.md §1 (I-C2) *(paraphrased from source/code_locus; registry `statement` empty)* |
| **Code** | crates/ade_runtime/src/consensus_inputs/importer.rs (LiveConsensusInputsImportError closed enum + validate_and_lift) |
| **Tests** | `unsupported_era_fails_fast`; `empty_era_string_fails_fast`; `epoch_end_before_start_is_bad_window`; `tip_outside_window_is_bad_window`; `zero_asc_denom_is_bad_field`; `short_genesis_hash_is_bad_hash_hex`; `non_hex_in_hash_is_bad_hash_hex`; `pool_in_distribution_missing_from_vrf_map_is_bad_pool`; `pool_id_wrong_width_is_bad_hash_hex`; `bad_json_surface_is_json_variant`; … (+2 more) |
| **CI** | `ci/ci_check_live_consensus_inputs_closure.sh` |

#### `DC-CONS-IN-02` — _enforced_

| Aspect | Location |
|--------|----------|
| **Source** | docs/planning/phase4-n-m-c-operator-pass-invariants.md §1 (I-C3) |
| **Requirement** | docs/planning/phase4-n-m-c-operator-pass-invariants.md §1 (I-C3) *(paraphrased from source/code_locus; registry `statement` empty)* |
| **Code** | crates/ade_runtime/src/consensus_inputs/canonical.rs (LiveConsensusInputsCanonical struct, encode_canonical_cbor private fn, canonical_from_raw lift, import_live_consensus_inputs sole authority) |
| **Tests** | `import_round_trip_yields_canonical_form_with_fingerprint`; `fingerprint_is_deterministic_across_repeated_imports`; `fingerprint_changes_when_any_canonical_input_changes`; `canonical_field_count_is_fifteen`; `fingerprint_is_blake2b_256_of_encode_canonical_cbor` |
| **CI** | `ci/ci_check_live_consensus_inputs_fingerprint.sh` |

### DC-VAL

#### `DC-VAL-01` — _enforced_

| Aspect | Location |
|--------|----------|
| **Source** | Cardano ledger spec; Ouroboros Praos spec; IDD determinism doctrine |
| **Requirement** | A block's validity verdict is a pure function of (LedgerState, PraosChainDepState, EraSchedule, LedgerView, block_cbor). No wall-clock, arrival order, HashMap/HashSet iteration, float, or ambient state may influence it. |
| **Code** | crates/ade_ledger/src/consensus_input_extract.rs, crates/ade_ledger/src/consensus_view.rs, crates/ade_core/src/consensus/ledger_view.rs |
| **Tests** | `corpus_loads_and_is_self_consistent`; `extract_nonces_field_order`; `extract_nonces_requires_exactly_five`; `extract_nonces_is_deterministic`; `view_returns_corpus_pool_stake_and_vrf_keyhash`; `view_unknown_pool_returns_none`; `view_unknown_epoch_returns_none`; `view_is_pure` |
| **CI** | `ci/ci_check_consensus_closed_enums.sh` |

#### `DC-VAL-02` — _enforced_

| Aspect | Location |
|--------|----------|
| **Source** | Cardano ledger spec; Ouroboros Praos spec |
| **Requirement** | A block is Valid iff both the consensus header authority (validate_and_apply_header) and the ledger body authority (apply_block_with_verdicts) accept it. No path may produce a Valid verdict while skipping either authority. |
| **Code** | crates/ade_ledger/src/block_validity/ (closed verdict/error taxonomy substrate; PHASE4-B1-S3), crates/ade_ledger/src/block_validity/transition.rs, crates/ade_ledger/src/block_validity/header_input.rs (header ∧ body composition; PHASE4-B1-S4) |
| **Tests** | `valid_block_evolves_both_states`; `header_before_body_fail_fast` |
| **CI** | `ci/ci_check_consensus_closed_enums.sh` |

#### `DC-VAL-03` — _enforced_

| Aspect | Location |
|--------|----------|
| **Source** | Ouroboros Praos spec; IDD fail-fast doctrine |
| **Requirement** | The header is validated before the body; body validation never runs on a header-invalid block. The first failing authority determines the reason (fail-fast ordering). |
| **Code** | crates/ade_ledger/src/block_validity/transition.rs (header authority decided before body; body authority unreachable on header failure; PHASE4-B1-S4) |
| **Tests** | `header_before_body_fail_fast` |
| **CI** | `ci/ci_check_consensus_closed_enums.sh` |

#### `DC-VAL-04` — _enforced_

| Aspect | Location |
|--------|----------|
| **Source** | cardano-node reference behavior; Cardano ledger spec |
| **Requirement** | Ade's Valid/Invalid verdict for a block equals the reference cardano-node verdict, including the reason class where the reference exposes it. Established over both a positive corpus (real valid blocks) and a mandatory adversarial corpus (blocks the reference rejects). |
| **Code** | crates/ade_ledger/src/block_validity/ (closed Valid/Invalid + reject-class comparison surface; PHASE4-B1-S3), crates/ade_testkit/src/validity/replay.rs (GREEN positive-corpus replay harness driving block_validity over all 14 Conway-576 blocks; PHASE4-B1-S6), crates/ade_testkit/src/validity/adversarial.rs (GREEN deterministic block mutators M1-M6 deriving adversarial blocks from the real corpus; PHASE4-B1-S7) |
| **Tests** | `corpus_block_count_is_14`; `all_corpus_blocks_valid`; `verdict_stream_replays_identically`; `no_mutation_is_ever_valid`; `each_mutation_maps_to_expected_class`; `adversarial_replays_identically` |
| **CI** | `ci/ci_check_consensus_closed_enums.sh` |

#### `DC-VAL-05` — _enforced_

| Aspect | Location |
|--------|----------|
| **Source** | IDD explicit-total-transition doctrine; Cardano ledger spec |
| **Requirement** | A Valid block yields evolved (LedgerState', PraosChainDepState'); an Invalid block yields the unchanged input states plus a structured reason. No partial or in-place mutation occurs on the invalid path. |
| **Code** | crates/ade_ledger/src/block_validity/ (closed verdict taxonomy: Valid carries evolved-state stats, Invalid carries structured reason; PHASE4-B1-S3), crates/ade_ledger/src/block_validity/transition.rs (Valid returns evolved (ledger', chain_dep'); Invalid returns input clones; PHASE4-B1-S4) |
| **Tests** | `invalid_block_leaves_state_unchanged`; `valid_block_evolves_both_states` |
| **CI** | `ci/ci_check_consensus_closed_enums.sh` |

#### `DC-VAL-06` — _enforced_

| Aspect | Location |
|--------|----------|
| **Source** | Cardano ledger spec (mandatory witness/field checks); IDD fail-fast doctrine |
| **Requirement** | Every crypto-input, field-size, and structural check on the authority path rejects (produces Invalid) on wrong size or shape and never silently skips. The pattern `if X.len() == K { check } else { skip }` is forbidden in BLUE validation; size checks go through a helper that returns an error. No defined-but-unwired check and no tautological (value-compared-to-itself) guard may stand in for a real check. |
| **Code** | crates/ade_ledger/src/block_validity/verdict.rs (FieldKind/FieldError closed fail-closed field taxonomy; PHASE4-B1-S3); crates/ade_core/src/consensus/kes_check.rs (expect_size fail-closed header crypto-field guard; PHASE4-B1-S5); crates/ade_testkit/src/validity/adversarial.rs (M1 truncated VRF proof, M3 flipped KES sig, M4 slot-beyond-horizon adversarial mutators exercising the fail-closed checks; PHASE4-B1-S7); crates/ade_ledger/src/tx_validity/witness.rs (wrong-size vkey/sig → MalformedWitnessField via from_bytes, never skipped); crates/ade_ledger/src/tx_validity/required_signers.rs (unresolvable input / malformed certs\|withdrawals\|voters CBOR → structured RequiredSignerError, never… |
| **Tests** | `expect_size_rejects_wrong_length`; `praos_malformed_kes_sig_rejected`; `no_mutation_is_ever_valid`; `each_mutation_maps_to_expected_class`; `wrong_size_signature_rejected`; `wrong_size_vkey_rejected`; `unresolvable_input_is_fail_fast`; `unresolvable_collateral_input_is_fail_fast`; `conway_conservation_full`; `conservation_early_out_removed`; … (+10 more) |
| **CI** | `ci/ci_check_consensus_closed_enums.sh` |

### DC-TXV

#### `DC-TXV-01` — _enforced_

| Aspect | Location |
|--------|----------|
| **Source** | Cardano ledger spec; IDD determinism doctrine |
| **Requirement** | tx_validity is a pure function of (LedgerState, tx_cbor). No wall-clock, arrival order, HashMap/HashSet iteration, float, or ambient state may influence a transaction's Valid/Invalid verdict. |
| **Code** | crates/ade_ledger/src/tx_validity/transition.rs (tx_validity: pure composition over (&LedgerState, &[u8])); crates/ade_ledger/src/tx_validity/phase1.rs (tx_phase_one + decode_tx, no I/O / clock / rand); PHASE4-B2-S2 |
| **Tests** | `tx_validity_is_deterministic` |
| **CI** | `ci/ci_check_consensus_closed_enums.sh` |

#### `DC-TXV-02` — _enforced_

| Aspect | Location |
|--------|----------|
| **Source** | Cardano ledger spec (UTXOW/UTXO/PPUP, Plutus phase-2) |
| **Requirement** | A transaction is Valid iff both phase-1 (structural + UTxO rules + witnesses) and phase-2 (Plutus, when scripts are present) accept it. No path may produce a Valid verdict while skipping either phase. |
| **Code** | crates/ade_ledger/src/tx_validity/transition.rs (tx_validity: fail-fast — phase-1 decided first, phase-2 dispatch via plutus_eval::try_evaluate_tx never runs when phase-1 fails or when no Plutus scripts present); crates/ade_ledger/src/tx_validity/phase1.rs (tx_phase_one composes B2-S1 witness closure + validate_conway_state_backed); PHASE4-B2-S2 |
| **Tests** | `valid_tx_is_valid_and_applies`; `phase1_failure_short_circuits_phase2` |
| **CI** | `ci/ci_check_consensus_closed_enums.sh` |

#### `DC-TXV-03` — _enforced_

| Aspect | Location |
|--------|----------|
| **Source** | cardano-node reference behavior; Cardano ledger spec |
| **Requirement** | Ade's Valid/Invalid verdict for a transaction equals the reference cardano-node verdict, including the reason class where the reference exposes it. Established over a positive corpus (real on-chain txs) and a mandatory adversarial corpus (txs the reference rejects). A false-accept is release-blocking. |
| **Code** | crates/ade_testkit/src/tx_validity/ (GREEN harness: extracts every on-wire Conway tx from the committed Conway-576 corpus blocks and drives the BLUE tx_validity over each at track_utxo=false — partial scope: structural + witness closure; UTxO-dependent checks deferred); crates/ade_ledger/src/tx_validity/phase1.rs (tx_phase_one gates the UTxO-dependent state-backed checks behind track_utxo, mirroring the block path's verify_conway_witness_closure + run_phase_one_composers split — the witness closure runs unconditionally); PHASE4-B2-S3 (positive half); crates/ade_testkit/src/tx_validity/adversarial.rs + crates/ade_ledger/tests/tx_validity_adversarial_corpus.rs (NEGATIVE half: family A… |
| **Tests** | `all_corpus_txs_valid`; `corpus_tx_count_nonzero`; `no_mutation_is_ever_valid`; `each_mutation_maps_to_expected_class`; `adversarial_replays_identically`; `adversarial_imbalanced_via_deposit`; `adversarial_imbalanced_via_withdrawal`; `adversarial_unknown_cert_tag_rejects_as_decode`; `adversarial_removed_tag_rejects_as_era_invalid`; `adversarial_truncated_withdrawals_rejects_as_decode`; … (+7 more) |
| **CI** | `ci/ci_check_consensus_closed_enums.sh` |

#### `DC-TXV-04` — _enforced_

| Aspect | Location |
|--------|----------|
| **Source** | IDD explicit-total-transition doctrine; Cardano ledger spec |
| **Requirement** | A Valid transaction yields an applied LedgerState' (the mempool's accumulating view); an Invalid transaction leaves the input state unchanged plus a structured reason. No partial or in-place mutation occurs on the invalid path. |
| **Code** | crates/ade_ledger/src/tx_validity/transition.rs (tx_validity: Valid → applied state via rules::apply_conway_tx_to_utxo; every Invalid path returns invalid() which clones the input state unchanged); crates/ade_ledger/src/tx_validity/verdict.rs (closed TxValidityVerdict/TxRejectClass/TxValidityError; total class()); PHASE4-B2-S2 |
| **Tests** | `valid_tx_is_valid_and_applies`; `invalid_tx_leaves_state_unchanged`; `class_mapping_is_total` |
| **CI** | `ci/ci_check_consensus_closed_enums.sh` |

#### `DC-TXV-05` — _enforced_

| Aspect | Location |
|--------|----------|
| **Source** | Cardano ledger spec (UTXOW required-signers per era); IDD fail-fast + closed-surface doctrine |
| **Requirement** | For each era, required_signers(state, tx_body) is a closed, explicit, era-versioned function over every signer source (resolved input payment credentials, explicit required-signers, certificate key hashes, withdrawal key hashes, Conway governance/voting witnesses, collateral-input implications). A signer source not represented in the era's closed enumeration is impossible to silently omit; incomplete enumeration is a forbidden false-accept path. |
| **Code** | crates/ade_ledger/src/tx_validity/required_signers.rs (closed era-versioned SignerSource enum + required_signers/tx_derived_required_signers grounded in Conway getConwayWitsVKeyNeeded + getVKeyWitnessConwayTxCert; PHASE4-B2-S1); crates/ade_ledger/src/tx_validity/witness.rs (fail-closed coverage over preserved body hash); crates/ade_ledger/src/rules.rs (verify_conway_witness_closure body-path wiring) |
| **Tests** | `all_required_covered_is_valid`; `missing_input_payment_witness_rejected`; `missing_explicit_required_signer_rejected`; `missing_withdrawal_witness_rejected`; `missing_certificate_witness_rejected`; `missing_governance_voter_witness_rejected`; `unresolvable_input_is_fail_fast`; `unresolvable_collateral_input_is_fail_fast`; `script_credential_input_not_a_vkey_signer`; `script_credential_certificate_not_a_vkey_signer`; … (+3 more) |
| **CI** | `ci/ci_check_consensus_closed_enums.sh` |

#### `DC-TXV-06` — _enforced_

| Aspect | Location |
|--------|----------|
| **Source** | Cardano ledger spec (Conway CDDL certificate tags 0..18; UTXO deposit/refund accounting); IDD fail-fast + closed-surface doctrine |
| **Requirement** | For each era, the certificate-deposit classification map(state, cert) is a closed, total, era-versioned function: every certificate variant resolves to exactly one of new_deposit(coin) \| refund(coin) \| neutral \| explicit_reject, with the coin sourced from the cert (Conway explicit-deposit variants, tags 7/8) or the protocol parameter (legacy variants, tags 0/1). An unrecognized cert tag, malformed cert CBOR, or undecodable withdrawals field is a deterministic reject, never a silent neutral. State-dependent cases that cannot be accounted (e.g. pool re-registration charged as a new deposit) reject with a structured UnsupportedStateDependentDepositAccounting error rather than guessing. Incomplete classification is a forbidden false-accept path feeding the value-conservation equation. |
| **Code** | crates/ade_codec/src/conway/cert.rs (decode_conway_certs, closed ConwayCert grammar); crates/ade_codec/src/error.rs (CodecError::UnknownCertTag); crates/ade_types/src/conway/cert.rs (ConwayCert, CertDisposition, DepositEffect, CoinSource); crates/ade_ledger/src/cert_classify.rs (classify); crates/ade_ledger/src/error.rs (UnsupportedStateDependentDepositAccounting); corpus/conway_certs/{classification_table.md,tags.json} |
| **Tests** | `decode_total_over_tags_0_18`; `unknown_cert_tag_is_codec_error`; `removed_tag_5_6_is_not_valid_in_conway`; `malformed_cert_cbor_rejected`; `decode_is_replay_deterministic`; `class_mapping_is_total`; `legacy_unregistration_unresolved_is_unsupported_state_dependent`; `legacy_unregistration_resolves_recorded_deposit`; `pool_reregistration_is_neutral`; `pool_new_registration_charges_pool_deposit` |
| **CI** | `ci/ci_check_conway_cert_classification_closed.sh` |

#### `DC-TXV-07` — _enforced_

| Aspect | Location |
|--------|----------|
| **Source** | IDD determinism doctrine (deposit params are canonical input); Conway protocol-parameter surface |
| **Requirement** | All deposit/refund amounts used by Conway transaction value-conservation accounting must be sourced from canonical ledger protocol parameters or explicit certificate fields, never from testkit defaults, shell configuration, ambient ConwayGovParams, or fallback constants. |
| **Code** | crates/ade_ledger/src/pparams.rs (ConwayOnlyDepositParams, ConwayDepositParams); crates/ade_ledger/src/state.rs (LedgerState.conway_deposit_params, conway_deposit_view); crates/ade_ledger/src/fingerprint.rs (fingerprint_pparams Conway-gated migration) |
| **Tests** | `conway_deposit_params_canonical_source`; `non_conway_state_has_no_conway_deposit_params`; `pparams_fingerprint_stable_for_non_conway`; `pparams_fingerprint_includes_conway_deposits_when_present` |
| **CI** | `ci/ci_check_deposit_param_authority.sh` |

### DC-NODE

#### `DC-NODE-01` — _enforced_

| Aspect | Location |
|--------|----------|
| **Source** | docs/planning/phase4-n-k-orchestrator-binary-invariants.md §1 (I-3) |
| **Requirement** | docs/planning/phase4-n-k-orchestrator-binary-invariants.md §1 (I-3) *(paraphrased from source/code_locus; registry `statement` empty)* |
| **Code** | crates/ade_runtime/src/orchestrator/peer_session.rs, crates/ade_runtime/src/orchestrator/core.rs |
| **Tests** | `peer_session_isolation_holds_under_failure`; `peer_session_per_peer_state_does_not_cross`; `peer_disconnect_removes_only_that_peer`; `peer_session_isolation_across_two_concurrent_tasks`; `step_per_peer_decode_error_isolates` |
| **CI** | `ci/ci_check_peer_session_isolation.sh` |

#### `DC-NODE-02` — _enforced_

| Aspect | Location |
|--------|----------|
| **Source** | docs/planning/phase4-n-k-orchestrator-binary-invariants.md §1 (I-4) |
| **Requirement** | docs/planning/phase4-n-k-orchestrator-binary-invariants.md §1 (I-4) *(paraphrased from source/code_locus; registry `statement` empty)* |
| **Code** | crates/ade_runtime/src/rollback/persistent_writer.rs, crates/ade_runtime/src/orchestrator/core.rs |
| **Tests** | `persistent_writer_on_admitted_captures_only_on_cadence`; `persistent_writer_round_trips_via_framing`; `persistent_writer_force_capture_skips_cadence_but_updates_state`; `persistent_writer_two_runs_are_deterministic`; `step_admit_triggers_capture_snapshot_at_cadence` |
| **CI** | `ci/ci_check_persistent_writer_no_parallel_cadence.sh` |

#### `DC-NODE-03` — _enforced_

| Aspect | Location |
|--------|----------|
| **Source** | docs/planning/phase4-n-k-orchestrator-binary-invariants.md §1 (I-5) |
| **Requirement** | docs/planning/phase4-n-k-orchestrator-binary-invariants.md §1 (I-5) *(paraphrased from source/code_locus; registry `statement` empty)* |
| **Code** | crates/ade_runtime/src/clock.rs, crates/ade_runtime/src/orchestrator/core.rs |
| **Tests** | `replay_equivalence_under_deterministic_clock_holds`; `replay_corpus_is_present_and_decodable`; `step_two_runs_produce_byte_identical_effects`; `deterministic_clock_is_pure`; `leadership_session_slot_arithmetic_is_pure` |
| **CI** | `ci/ci_check_clock_seam.sh`; `ci/ci_check_orchestrator_core_purity.sh` |

#### `DC-NODE-04` — _enforced_

| Aspect | Location |
|--------|----------|
| **Source** | docs/planning/phase4-n-k-orchestrator-binary-invariants.md §1 (I-6, I-7) |
| **Requirement** | docs/planning/phase4-n-k-orchestrator-binary-invariants.md §1 (I-6, I-7) *(paraphrased from source/code_locus; registry `statement` empty)* |
| **Code** | crates/ade_node/src/node.rs, crates/ade_node/src/main.rs |
| **Tests** | `shutdown_then_resume_produces_byte_identical_state`; `shutdown_clean_exits_with_evidence`; `cold_start_without_genesis_fails_with_generic_startup_code`; `binary_halts_on_authority_fatal_decode_error` |
| **CI** | `ci/ci_check_node_binary_uses_single_bootstrap.sh` |

#### `DC-NODE-05` — _enforced_

| Aspect | Location |
|--------|----------|
| **Source** | docs/planning/phase4-n-f-e-invariants.md |
| **Requirement** | docs/planning/phase4-n-f-e-invariants.md *(paraphrased from source/code_locus; registry `statement` empty)* |
| **Code** | crates/ade_node/src/node_lifecycle.rs, crates/ade_node/src/run_loop_planner.rs |
| **Tests** | `plan_loop_step_forge_precedence_table_is_total`; `forge_slot_guard_none_is_due`; `forge_slot_guard_at_most_once_per_slot`; `forge_slot_guard_rejects_past_slot`; `relay_loop_forge_slot_derived_via_clock_seam`; `relay_loop_forge_tick_attempts_forge_advances_no_tip`; `relay_loop_without_producer_material_matches_nfd_relay`; `relay_loop_forge_two_runs_byte_identical`; `forge_tick_rotated_kes_period_skips_no_retroactive_sign`; `forge_tick_off_epoch_slot_fails_closed_local` |
| **CI** | `ci/ci_check_loop_planner_closed.sh`; `ci/ci_check_node_run_loop_containment.sh`; `ci/ci_check_genesis_consistency_fixture_present.sh` |

#### `DC-NODE-06` — _enforced_

| Aspect | Location |
|--------|----------|
| **Source** | docs/planning/phase4-n-f-g-invariants.md; docs/clusters/completed/PHASE4-N-U/cluster.md (S3 supersession) |
| **Requirement** | docs/clusters/completed/PHASE4-N-U/cluster.md (S3 supersession) *(paraphrased from source/code_locus; registry `statement` empty)* |
| **Code** | crates/ade_node/src/node_lifecycle.rs (run_node_serve_task -> ServedChainSource::DurableChainDb over Arc<dyn ChainDb>; the G-B push sibling + serve_gate_admits + handoff channel retired), crates/ade_node/src/node_sync.rs, crates/ade_runtime/src/network/served_chain_projection.rs (ChainDbServedSource -- the durable-provenance serve source), crates/ade_runtime/src/network/serve_dispatch.rs (ServedChainSource). HISTORICAL (G-B/G-C, superseded by N-U S3): crates/ade_runtime/src/producer/served_chain_handle.rs (ServedChainHandle -- now --mode produce only), crates/ade_ledger/src/producer/served_chain.rs |
| **Tests** | `handoff_carrier_constructs_only_from_self_accepted_forge`; `forge_surfaces_accepted_block_only_on_self_accept`; `handoff_carrier_has_no_raw_bytes_constructor`; `serve_ingress_type_rejects_failed_forge_outcome`; `sibling_serve_admits_via_push_atomic_only`; `serve_sibling_admission_replay_byte_identical`; `serve_sibling_push_atomic_fed_only_by_into_accepted`; `relay_loop_containment_semantics_after_serve_sibling_retired`; `block_fetch_payload_is_self_accepted_bytes`; `block_fetch_tag24_round_trips_to_self_accept_input`; … (+3 more) |
| **CI** | `ci/ci_check_served_chain_handoff_fence.sh`; `ci/ci_check_node_run_loop_containment.sh`; `ci/ci_check_served_chain_projection.sh` |

#### `DC-NODE-07` — _enforced_

| Aspect | Location |
|--------|----------|
| **Source** | docs/planning/phase4-n-f-g-h-invariants.md |
| **Requirement** | docs/planning/phase4-n-f-g-h-invariants.md *(paraphrased from source/code_locus; registry `statement` empty)* |
| **Code** | crates/ade_node/src/node_lifecycle.rs (retain the ServedChainView + spawn the sibling listener/serve-dispatch task outside run_relay_loop -- the only new code), crates/ade_node/src/produce_mode.rs (run_n2n_listener + dispatch_server_frame_event_to_outbound + new_per_peer_outbound -- shared serve adapter to be EXTRACTED to a shared module, not duplicated; OQ1), crates/ade_network/src/chain_sync/server.rs (producer_chain_sync_serve + producer_chain_sync_advance_tip -- reused BLUE), crates/ade_network/src/block_fetch/server.rs (producer_block_fetch_serve -- reused BLUE), crates/ade_runtime/src/producer/served_chain_handle.rs (ServedChainHandle/ServedChainView -- reused GREEN) |
| **Tests** | `served_view_projects_durable_chain`; `node_serve_start_failure_is_surfaced_not_silent`; `n2n_supported_for_magic_produces_configured_magic`; `node_c1_serve_live` |
| **CI** | `ci/ci_check_single_serve_dispatch_authority.sh`; `ci/ci_check_serve_listener_magic_aware.sh` |

#### `DC-NODE-08` — _enforced_

| Aspect | Location |
|--------|----------|
| **Source** | docs/planning/phase4-n-f-g-j-genesis-successor-prevhash-invariants.md |
| **Requirement** | docs/planning/phase4-n-f-g-j-genesis-successor-prevhash-invariants.md *(paraphrased from source/code_locus; registry `statement` empty)* |
| **Code** | crates/ade_node/src/node_sync.rs (forge_header_position -- GREEN single cold-start convention: None => block 0 + PrevHash::Genesis, Some => last_block_no+1 + Block, malformed-height edge fails closed; forge_one_from_recovered(selected_tip: Option<&ChainTip>) routes the cold-start ctx into the SAME run_real_forge S3 proved; NodeForgeError::RecoveredTipMissingBlockNo); crates/ade_node/src/node_lifecycle.rs (may_cold_start_forge -- GREEN cold-start permission: no tip + recovered lineage + forge-eligible feed; the LoopStep::ForgeTick arm passes selected_tip.as_ref() and gates the both-None genesis forge); crates/ade_node/src/forge_intent.rs (ForgeIntent::On precondition) |
| **Tests** | `forge_one_from_recovered_cold_start_is_block_zero_genesis`; `forge_one_from_recovered_with_tip_is_block_n_plus_one_block_prev`; `forge_header_position_some_tip_without_block_no_fails_closed`; `cold_start_block_number_is_zero_single_convention`; `node_spine_cold_start_forges_genesis_block_zero`; `cold_start_gate_allows_genesis_when_eligible_and_recovered`; `node_spine_cold_start_ineligible_feed_does_not_forge`; `cold_start_gate_blocks_without_recovered_lineage`; `cold_start_gate_inactive_when_tip_present` |
| **CI** | `ci/ci_check_genesis_successor_reachability.sh` |

#### `DC-NODE-09` — _enforced_

| Aspect | Location |
|--------|----------|
| **Source** | docs/clusters/PHASE4-N-F-G-K/cluster.md |
| **Requirement** | docs/clusters/PHASE4-N-F-G-K/cluster.md *(paraphrased from source/code_locus; registry `statement` empty)* |
| **Code** | crates/ade_node/src/node_lifecycle.rs (run_node_lifecycle_inner On-arm: the --listen serve task is spawned with shutdown.clone() -- the operator shutdown watch -- NOT a dedicated feed-end stop channel; the post-run_relay_loop node_serve_stop.send(true) flip is REMOVED; node_serve_handle is awaited and ends only on shutdown / fatal serve error. run_node_serve_task -- the serve loop -- is UNCHANGED: it breaks on its shutdown watch, a fatal accept error, and events-channel close, and takes only (TcpListener, ServedChainView, network_magic, watch::Receiver<bool>)) |
| **Tests** | `serve_task_outlives_feed_end_and_serves_late_fetch`; `serve_task_terminates_on_shutdown_no_hang`; `served_view_projects_durable_chain`; `node_serve_start_failure_is_surfaced_not_silent` |
| **CI** | `ci/ci_check_node_serve_lifetime.sh` |

#### `DC-NODE-10` — _enforced_

| Aspect | Location |
|--------|----------|
| **Source** | docs/clusters/PHASE4-N-F-G-Q/cluster.md |
| **Requirement** | docs/clusters/PHASE4-N-F-G-Q/cluster.md *(paraphrased from source/code_locus; registry `statement` empty)* |
| **Code** | crates/ade_node/src/node_lifecycle.rs (the relay-loop forge call threads the evolved state.receive.chain_dep + state.receive.ledger into forge_one_from_recovered -- NOT the recovered baseline); crates/ade_node/src/node_sync.rs (forge_one_from_recovered(recovered, live_chain_dep, live_ledger, selected_tip, ...) -- forge_header_position + the self-accept ctx read live_chain_dep/live_ledger; recovered supplies only the seed-epoch PoolDistr + the off-epoch guard) |
| **Tests** | `forge_successor_reads_evolved_spine_block_no_not_stale_baseline_g_q`; `forge_one_from_recovered_cold_start_is_block_zero_genesis` |
| **CI** | `ci/ci_check_forge_successor_evolved_spine.sh` |

#### `DC-NODE-11` — _enforced_

| Aspect | Location |
|--------|----------|
| **Source** | docs/clusters/PHASE4-N-F-G-R/cluster.md; docs/clusters/PHASE4-N-U/S3-serve-as-durable-chain-projection.md |
| **Requirement** | docs/clusters/PHASE4-N-F-G-R/cluster.md; docs/clusters/PHASE4-N-U/S3-serve-as-durable-chain-projection.md *(paraphrased from source/code_locus; registry `statement` empty)* |
| **Code** | crates/ade_runtime/src/network/served_chain_projection.rs (ChainDbServedSource -- PHASE4-N-U S3: serve-as-projection of the durable ChainDb provides the stable/coherent served chain; the durable chain is extend-only so it holds exactly one block 0); crates/ade_node/src/node_lifecycle.rs (run_node_serve_task serves the durable projection; serve_gate_admits RETIRED); crates/ade_ledger/src/receive/{reducer,admitted}.rs + block_validity (extend-only durable admit -- DC-CONS-23 -- rejects a re-mint block 0). HISTORICAL (PHASE4-N-F-G-R, superseded): node_lifecycle serve_gate_admits monotone-block_no gate over the ServedChainView accumulator + crates/ade_ledger/src/producer/served_chain.rs… |
| **Tests** | `served_view_projects_durable_chain`; `served_view_retires_accumulator` |
| **CI** | `ci/ci_check_served_chain_projection.sh` |

#### `DC-NODE-12` — _enforced_

| Aspect | Location |
|--------|----------|
| **Source** | docs/planning/phase4-n-u-forged-block-durability-invariants.md |
| **Requirement** | docs/planning/phase4-n-u-forged-block-durability-invariants.md *(paraphrased from source/code_locus; registry `statement` empty)* |
| **Code** | crates/ade_node/src/node_sync.rs (admit_forged_block_durably -- the fenced driver -> pump_block); crates/ade_node/src/node_lifecycle.rs (ForgeTick arm admits the self-accepted handoff via the driver); crates/ade_runtime/src/forward_sync/{reducer,pump}.rs (pump_block / AdmitPlan::durable -- reused) |
| **Tests** | `forge_tick_durable_admit_advances_tip`; `forge_successor_builds_block_1_from_durable_tip`; `forged_admit_bytes_byte_identical_to_self_accept` |
| **CI** | `ci/ci_check_forged_durable_admit_via_pump.sh`; `ci/ci_check_node_run_loop_containment.sh` |

#### `DC-NODE-13` — _enforced_

| Aspect | Location |
|--------|----------|
| **Source** | docs/planning/phase4-n-u-forged-block-durability-invariants.md; docs/clusters/PHASE4-N-U/S3-serve-as-durable-chain-projection.md |
| **Requirement** | docs/planning/phase4-n-u-forged-block-durability-invariants.md; docs/clusters/PHASE4-N-U/S3-serve-as-durable-chain-projection.md *(paraphrased from source/code_locus; registry `statement` empty)* |
| **Code** | crates/ade_runtime/src/network/served_chain_projection.rs (ChainDbServedSource — RED read-only projection adapter implementing ServedHeaderLookup + ServedRangeLookup over &dyn ChainDb: next_after/intersect/tip/range_bytes read the durable ChainDb via iter_from_slot/get_block_by_hash/tip, decode_block for block_no/era, block_header_bytes for the header, stored.bytes served verbatim); crates/ade_runtime/src/network/serve_dispatch.rs (ServedChainSource enum {Snapshot\|DurableChainDb}; the single dispatch authority reads either source — DC-NODE-07 preserved); crates/ade_node/src/node_lifecycle.rs (run_node_serve_task dispatches with ServedChainSource::DurableChainDb over… |
| **Tests** | `served_view_projects_durable_chain`; `follower_fetches_coherent_history_incl_ingested_predecessor`; `served_view_retires_accumulator` |
| **CI** | `ci/ci_check_served_chain_projection.sh` |

#### `DC-NODE-14` — _enforced_

| Aspect | Location |
|--------|----------|
| **Source** | docs/clusters/PHASE4-N-AE/cluster.md; docs/planning/phase4-n-ae-slice-a-invariants.md; docs/planning/c2-local-discovered-gaps.md |
| **Requirement** | docs/clusters/PHASE4-N-AE/cluster.md; docs/planning/phase4-n-ae-slice-a-invariants.md; docs/planning/c2-local-discovered-gaps.md *(paraphrased from source/code_locus; registry `statement` empty)* |
| **Code** | crates/ade_runtime/src/network/served_chain_projection.rs (ChainDbServedSource::intersect/next_after/tip over the durable ChainDb -- read-only, reused); crates/ade_node/src/node_lifecycle.rs (the forge-on-followed-tip gate gives the projection an intersectable parent) |
| **Tests** | `served_chain_intersects_at_followed_tip_and_rolls_to_forged`; `recovered_anchor_is_not_peer_intersectable`; `forged_successor_on_recovered_anchor_is_not_peer_adoptable`; `recover_follow_serve_forged_parent_intersectable` |
| **CI** | `ci/ci_check_forge_followed_tip_admission.sh ci/ci_check_recovered_anchor_intersectable.sh` |

#### `DC-NODE-15` — _enforced_

| Aspect | Location |
|--------|----------|
| **Source** | docs/clusters/PHASE4-N-AE/cluster.md; docs/clusters/PHASE4-N-AE/slices/AE.A.md; docs/planning/phase4-n-ae-slice-a-invariants.md |
| **Requirement** | docs/clusters/PHASE4-N-AE/cluster.md; docs/clusters/PHASE4-N-AE/slices/AE.A.md; docs/planning/phase4-n-ae-slice-a-invariants.md *(paraphrased from source/code_locus; registry `statement` empty)* |
| **Code** | crates/ade_node/src/node_sync.rs (forge_followed_tip_admission GREEN classifier + ForgeFollowedTipAdmission/NotCaughtUpReason + ForgeRefused/NodeForgeOutcome + FollowedPeerTipSignal); crates/ade_node/src/node_lifecycle.rs (run_relay_loop_with_sched ForgeTick arm: recovered.tip forge-base fallback removed, gate called before the single fenced forge_one_from_recovered, typed ForgeRefused::NotCaughtUp recorded into ForgeActivation.last_forge_refused) |
| **Tests** | `forge_refused_not_caught_up`; `forge_base_falls_back_to_snapshot_anchor`; `forge_on_followed_tip_proceeds_with_parent_byte_equal` |
| **CI** | `ci/ci_check_forge_followed_tip_admission.sh` |

#### `DC-NODE-16` — _enforced_

| Aspect | Location |
|--------|----------|
| **Source** | docs/clusters/PHASE4-N-AE/slices/AE.F.md; docs/planning/phase4-n-ae-f-echo-idempotency-invariants.md; docs/evidence/phase4-n-ae-ce-a5-relay-adoption.md |
| **Requirement** | docs/clusters/PHASE4-N-AE/slices/AE.F.md; docs/planning/phase4-n-ae-f-echo-idempotency-invariants.md; docs/evidence/phase4-n-ae-ce-a5-relay-adoption.md *(paraphrased from source/code_locus; registry `statement` empty)* |
| **Code** | crates/ade_runtime/src/forward_sync/pump.rs (pump_block -- the hash-exact get_block_by_hash already-have gate, placed before the BLUE chokepoint reducer) |
| **Tests** | `pump_block_reannounced_block_is_idempotent_noop`; `pump_block_different_block_at_or_before_tip_still_fails_closed`; `run_node_sync_survives_reannounced_block_in_feed` |
| **CI** | `ci/ci_check_receive_idempotency.sh` |

#### `DC-NODE-17` — _declared_

| Aspect | Location |
|--------|----------|
| **Source** | docs/planning/sustained-single-producer-forge-invariants.md |
| **Requirement** | docs/planning/sustained-single-producer-forge-invariants.md *(paraphrased from source/code_locus; registry `statement` empty)* |
| **Code** | CANDIDATE (pending OQ-1): crates/ade_node/src/node_sync.rs (FollowedPeerTipSignal.observe + the wire-pump pump_lookahead/wait_ready which today observes only AdmissionPeerEvent::TipUpdate) |
| **Tests** | _(no tests listed — gap)_ |
| **CI** | _(no CI gate — gap)_ |

#### `DC-NODE-18` — _enforced_

| Aspect | Location |
|--------|----------|
| **Source** | docs/planning/single-producer-extend-own-spine-invariants.md |
| **Requirement** | docs/planning/single-producer-extend-own-spine-invariants.md *(paraphrased from source/code_locus; registry `statement` empty)* |
| **Code** | crates/ade_node/src/node_sync.rs: ForgeMode enum (InitialCatchupRequired -> CaughtUpToPeerTip -> FirstOwnBlockServed -> SingleProducerExtendOwnDurableSpine; no booleans) + VenueRole + VenueAdoptionCertificate + SingleProducerFenceReason + SingleProducerForgeDecision + single_producer_forge_decision + forge_mode_on_caughtup/_on_first_own_block_served/_on_extend + forge_mode_after_admit (advances ONLY on an actual admit, never on a not_leader tick) + ForgeRefused::SingleProducerFenceViolation. crates/ade_node/src/node_lifecycle.rs: the mode-aware ForgeTick gate in run_relay_loop_with_sched (behind VenueRole::SingleProducer; default Unknown == the verbatim prior DC-NODE-15 path) +… |
| **Tests** | `forge_mode_transitions_are_total_and_deterministic`; `extend_own_spine_promotion_requires_adoption_certificate` _(NOT FOUND in source — stale registry test ref)_; `extend_own_spine_forges_on_durable_tip_without_followed_equality`; `single_producer_fence_fails_closed`; `extend_own_spine_two_runs_byte_identical`; `forge_mode_after_admit_only_advances_on_real_admit` |
| **CI** | `ci/ci_check_single_producer_extend_own_spine.sh` |

#### `DC-NODE-19` — _declared_

| Aspect | Location |
|--------|----------|
| **Source** | docs/planning/single-producer-loop-continuation-after-feed-eof-invariants.md |
| **Requirement** | docs/planning/single-producer-loop-continuation-after-feed-eof-invariants.md *(paraphrased from source/code_locus; registry `statement` empty)* |
| **Code** | CANDIDATE (pending /cluster-plan): crates/ade_node/src/run_loop_planner.rs (plan_loop_step gains a 5th closed content-blind VenuePolicy input -- the 32-case total table; + a GREEN VenueRole/ForgeMode -> VenuePolicy projection); crates/ade_node/src/node_lifecycle.rs (run_relay_loop_with_sched threads the venue policy + restructures the Idle-under-feed-end wait so a dead feed does not starve the forge cadence; the shutdown watch stays the lifecycle authority; the certified-run continuation fence reuses the DC-NODE-18 SingleProducerFenceReason); crates/ade_node/src/node_sync.rs (the venue/mode/cert continuation fence reuses single_producer_forge_decision / SingleProducerFenceReason) |
| **Tests** | _(no tests listed — gap)_ |
| **CI** | _(no CI gate — gap)_ |

#### `DC-NODE-20` — _enforced_

| Aspect | Location |
|--------|----------|
| **Source** | docs/planning/local-selected-durable-chain-forge-base-invariants.md |
| **Requirement** | docs/planning/local-selected-durable-chain-forge-base-invariants.md *(paraphrased from source/code_locus; registry `statement` empty)* |
| **Code** | CANDIDATE (pending /cluster-plan): crates/ade_node/src/node_lifecycle.rs (the proceed_to_forge gate -- replace the post-self-admit durable_servable_tip == followed_peer_tip re-check + the read_adoption_cert promotion with a local-selected-durable-tip authority derived from ChainDb::tip, fenced); crates/ade_node/src/node_sync.rs (the ForgeMode transition CaughtUpToPeerTip -> SingleProducerExtendOwnDurableSpine becomes DIRECT on self-admit, folding out the cert-gated FirstOwnBlockServed intermediate; the observed-feed competing-block fence). REUSES BLUE ChainDb::tip + pump_block (no new BLUE authority). |
| **Tests** | `caughtup_self_admit_enters_extend_directly_no_cert`; `forge_base_selected_transcript_witnesses_local_tip`; `local_spine_sustains_two_successors_no_cert`; `local_spine_two_runs_byte_identical` |
| **CI** | `ci/ci_check_local_durable_forge_base.sh`; `ci/ci_check_forge_followed_tip_admission.sh` |

#### `DC-NODE-21` — _enforced_

| Aspect | Location |
|--------|----------|
| **Source** | docs/planning/local-selected-durable-chain-forge-base-invariants.md |
| **Requirement** | docs/planning/local-selected-durable-chain-forge-base-invariants.md *(paraphrased from source/code_locus; registry `statement` empty)* |
| **Code** | CANDIDATE (pending /cluster-plan): crates/ade_node/src/node_lifecycle.rs (read_adoption_cert demoted to an evidence-only transcript record, REMOVED from the forge-base / proceed_to_forge path); a CI gate asserting the cert is never a forge-base input and never present in multi-producer/preprod/production forge paths. |
| **Tests** | `caughtup_self_admit_enters_extend_directly_no_cert`; `forge_base_selected_transcript_witnesses_local_tip`; `local_spine_cert_file_absent_from_replay_surface` |
| **CI** | `ci/ci_check_cert_evidence_only.sh`; `ci/ci_check_node_path_fidelity.sh` |

#### `DC-NODE-22` — _enforced_

| Aspect | Location |
|--------|----------|
| **Source** | docs/planning/local-selected-durable-chain-forge-base-invariants.md |
| **Requirement** | docs/planning/local-selected-durable-chain-forge-base-invariants.md *(paraphrased from source/code_locus; registry `statement` empty)* |
| **Code** | CANDIDATE (pending S4b): crates/ade_node/src/node_lifecycle.rs -- the warm-start arm of run_node_lifecycle_inner: after warm_start_recovery + declare_single_producer_venue, derive forge_mode = SingleProducerExtendOwnDurableSpine{current_tip = recovered ChainDb::tip} when venue_role == SingleProducer AND the recovered tip is above the bootstrap anchor / own-spine threshold, fenced as in the statement. REUSES the DC-NODE-20 fence + ChainDb::tip + pump_block (no new BLUE authority). |
| **Tests** | `warm_start_reentry_requires_tip_above_recovered_anchor`; `warm_start_single_producer_re_enters_extend_and_forges` |
| **CI** | `ci/ci_check_warm_start_re_entry.sh` |

#### `DC-NODE-23` — _enforced_

| Aspect | Location |
|--------|----------|
| **Source** | docs/planning/phase4-n-ai-live-fork-choice-invariants.md (I-5) |
| **Requirement** | docs/planning/phase4-n-ai-live-fork-choice-invariants.md (I-5) *(paraphrased from source/code_locus; registry `statement` empty)* |
| **Code** | CANDIDATE (pending /cluster-plan): crates/ade_node/src/node_sync.rs (a GREEN-by-function classifier over the durable tip + the arriving candidate header summary; sibling to forge_followed_tip_admission / single_producer_forge_decision). |
| **Tests** | `classify_already_have_when_in_spine`; `classify_linear_extend_on_exact_parent_and_block_no`; `classify_competing_on_nonmatching_parent`; `classify_competing_on_wrong_block_no`; `classify_competing_on_genesis_prev_hash`; `resolve_passthrough_already_have_and_linear_extend` |
| **CI** | `ci/ci_check_receive_detector_venue_split.sh` |

#### `DC-NODE-24` — _enforced_

| Aspect | Location |
|--------|----------|
| **Source** | docs/planning/phase4-n-ai-live-fork-choice-invariants.md (I-6) |
| **Requirement** | docs/planning/phase4-n-ai-live-fork-choice-invariants.md (I-6) *(paraphrased from source/code_locus; registry `statement` empty)* |
| **Code** | CANDIDATE (pending /cluster-plan): crates/ade_node/src/node_sync.rs (the venue->resolver projection) + crates/ade_node/src/node_lifecycle.rs (the receive arm dispatching to the chain_selector orchestrator on the Participant path; the SingleProducer path keeps the DC-NODE-20 fail-closed). |
| **Tests** | `resolve_singleproducer_competing_refuses`; `resolve_participant_competing_needs_fork_choice`; `resolve_participant_already_have_and_linear_extend_do_not_call_fork_choice`; `resolve_unknown_venue_fails_closed` |
| **CI** | `ci/ci_check_receive_detector_venue_split.sh` |

#### `DC-NODE-25` — _enforced_

| Aspect | Location |
|--------|----------|
| **Source** | docs/planning/phase4-n-ai-live-fork-choice-invariants.md (I-2, I-3, I-4, I-11) |
| **Requirement** | docs/planning/phase4-n-ai-live-fork-choice-invariants.md (I-2, I-3, I-4, I-11) *(paraphrased from source/code_locus; registry `statement` empty)* |
| **Code** | CANDIDATE (pending /cluster-plan): crates/ade_node/src/node_lifecycle.rs (the RED apply driver) over crates/ade_ledger/src/receive/reducer.rs + crates/ade_ledger/src/rollback/* + crates/ade_runtime/src/forward_sync/pump.rs. REUSES the enforced authorities; no new BLUE. |
| **Tests** | `apply_rolledback_rolls_back_and_appends_wal_record_after_commit`; `apply_chain_selected_invalid_body_fails_via_pump_no_advance`; `apply_chain_selected_without_block_bytes_fails_closed`; `participant_rollback_applies_durably`; `participant_block_with_no_durable_tip_pumps` |
| **CI** | `ci/ci_check_live_fork_choice_apply.sh`; `ci/ci_check_live_fork_choice_wiring.sh` |

#### `DC-NODE-26` — _enforced_

| Aspect | Location |
|--------|----------|
| **Source** | docs/planning/phase4-n-ai-live-fork-choice-invariants.md (I-7) |
| **Requirement** | docs/planning/phase4-n-ai-live-fork-choice-invariants.md (I-7) *(paraphrased from source/code_locus; registry `statement` empty)* |
| **Code** | CANDIDATE (pending /cluster-plan): crates/ade_node/src/node_sync.rs / node_lifecycle.rs (the reconciliation projection: derive ChainSelectorState from the durable stores, or hold OrchestratorState in lockstep -- OQ-2). |
| **Tests** | `apply_reconciliation_mismatch_fails_fast`; `apply_rejected_makes_no_durable_change` |
| **CI** | `ci/ci_check_live_fork_choice_apply.sh` |

#### `DC-NODE-27` — _enforced_

| Aspect | Location |
|--------|----------|
| **Source** | docs/planning/phase4-n-ai-live-fork-choice-invariants.md (section 4, OQ-1) |
| **Requirement** | docs/planning/phase4-n-ai-live-fork-choice-invariants.md (section 4, OQ-1) *(paraphrased from source/code_locus; registry `statement` empty)* |
| **Code** | CANDIDATE (pending /cluster-plan; OQ-1 RESOLVED -> A, see docs/planning/phase4-n-ai-oq1-rollback-durability-decision.md): a version-gated additive WalEntry::RollBack {to_point, reason, prior_tip, selected_tip} marker (crates/ade_ledger/src/wal/event.rs tag/encode/decode) whose replay arm in crates/ade_ledger/src/wal/replay.rs re-invokes the EXISTING materialize_rolled_back_state (CN-STORE-07) + lockstep commit_rollback (DC-CONS-20) and re-anchors the fingerprint chain to the materialized rolled-back fp. Append-only WAL preserved (CN-WAL-01); NOT a second rollback implementation. Option B (WAL-tail reconciliation) REJECTED. |
| **Tests** | `apply_rolledback_replays_byte_identical_recovers_forkpoint`; `replay_with_rollback_recovers_selected_not_abandoned`; `replay_with_rollback_two_runs_byte_identical`; `rollback_replay_reanchor_fp_equals_materialized_fp` |
| **CI** | `ci/ci_check_wal_rollback_replay_equiv.sh`; `ci/ci_check_live_fork_choice_apply.sh` |

#### `DC-NODE-28` — _enforced_

| Aspect | Location |
|--------|----------|
| **Source** | docs/planning/phase4-n-ai-live-fork-choice-invariants.md (I-12) |
| **Requirement** | docs/planning/phase4-n-ai-live-fork-choice-invariants.md (I-12) *(paraphrased from source/code_locus; registry `statement` empty)* |
| **Code** | CANDIDATE (pending /cluster-plan): crates/ade_node/src/node_sync.rs (the forge gate -- extend single_producer_forge_decision / the Participant forge-base selection with a pending-resolution fence) + node_lifecycle.rs (the ForgeTick arm respects the pending state). |
| **Tests** | `pending_reselection_forge_refusal_gate`; `participant_rollback_beyond_k_fails_closed_clears_pending`; `singleproducer_rollback_refused_by_run_node_sync` |
| **CI** | `ci/ci_check_live_fork_choice_wiring.sh`; `ci/ci_check_participant_venue_inert.sh` |

#### `DC-NODE-29` — _enforced_

| Aspect | Location |
|--------|----------|
| **Source** | docs/clusters/PHASE4-N-AI/S6-rollback-target-slot-hash-binding.md (H-1 remediation) |
| **Requirement** | docs/clusters/PHASE4-N-AI/S6-rollback-target-slot-hash-binding.md (H-1 remediation) *(paraphrased from source/code_locus; registry `statement` empty)* |
| **Code** | crates/ade_node/src/node_lifecycle.rs (run_participant_sync RollBack arm) over crates/ade_runtime/src/chaindb get_block_by_hash. RED shell binding; reuses the enforced rollback authorities; no new BLUE. |
| **Tests** | `rollback_slot_hash_mismatch_fails_before_mutation`; `participant_rollback_applies_durably`; `participant_rollback_to_unknown_point_fails_closed` |
| **CI** | `ci/ci_check_rollback_target_canonical_binding.sh` |

#### `DC-NODE-30` — _enforced_

| Aspect | Location |
|--------|----------|
| **Source** | docs/planning/phase4-n-aj-participant-convergence-evidence-invariants.md §1 (I-AJ-1) + §7 (D-1/D-2/D-3) |
| **Requirement** | docs/planning/phase4-n-aj-participant-convergence-evidence-invariants.md §1 (I-AJ-1) + §7 (D-1/D-2/D-3) *(paraphrased from source/code_locus; registry `statement` empty)* |
| **Code** | crates/ade_node/src/node_lifecycle.rs (run_participant_sync emission), crates/ade_node/src/admission/verdict.rs (derive, reused), crates/ade_node/src/admission_log/ (vocabulary/writer, reused against the --convergence-evidence-path sink), crates/ade_node/src/cli.rs (--convergence-evidence-path) |
| **Tests** | `convergence_evidence_absent_path_emits_no_file`; `convergence_evidence_writer_emits_closed_vocabulary`; `convergence_evidence_write_failure_poisons_and_is_surfaced`; `convergence_evidence_context_marks_incomplete_on_write_failure`; `participant_cold_start_admit_emits_received_admitted_agreed`; `participant_block_received_does_not_imply_admission`; `participant_convergence_evidence_replay_byte_identical` |
| **CI** | `ci/ci_check_convergence_evidence_emit_only.sh`; `ci/ci_check_convergence_evidence_vocabulary_closed.sh`; `ci/ci_check_convergence_evidence_schema.sh` |

#### `DC-NODE-31` — _enforced_

| Aspect | Location |
|--------|----------|
| **Source** | docs/planning/phase4-n-ak-recovered-anchor-tip-invariants.md (AK-INV-1) + docs/clusters/PHASE4-N-AK/cluster.md |
| **Requirement** | docs/planning/phase4-n-ak-recovered-anchor-tip-invariants.md (AK-INV-1) + docs/clusters/PHASE4-N-AK/cluster.md *(paraphrased from source/code_locus; registry `statement` empty)* |
| **Code** | crates/ade_ledger/src/recovered_anchor_point.rs (RecoveredAnchorPoint type + sole canonical CBOR codec), crates/ade_runtime/src/bootstrap.rs (resolve_live_follow_start -- private tip resolver; BootstrapInputs.recovered_anchor canonical input; BootstrapState live-follow start tip), crates/ade_runtime/src/recovered_anchor.rs (load_recovered_anchor_point -- load + fail-closed verify, kept out of bootstrap.rs to preserve CN-NODE-01 single-pub-fn), crates/ade_runtime/src/seed_epoch_lineage.rs (persist_seed_epoch_consensus_inputs -- writes the anchor-point record at seed/recover), crates/ade_runtime/src/chaindb/{mod,in_memory,persistent}.rs (put/get_recovered_anchor_point anchor-fp-keyed store… |
| **Tests** | `resolve_live_follow_start_treats_zero_hash_anchor_as_origin`; `bootstrap_bare_anchor_recovery_surfaces_anchor_as_live_follow_tip`; `bootstrap_true_origin_recovery_surfaces_none_tip`; `bootstrap_servable_chaindb_tip_wins_over_anchor`; `warm_start_loads_persisted_anchor_point`; `warm_start_non_origin_anchor_missing_anchor_point_fails_closed`; `warm_start_anchor_point_fingerprint_mismatch_fails_closed`; `same_store_same_anchor_point_same_findintersect_start`; `bootstrap_recover_persists_anchor_point_sidecar`; `recovered_anchor_point_round_trips_byte_identical`; … (+1 more) |
| **CI** | _(no CI listed in registry — enforced via Code/Tests; gap: bind a CI gate)_ |

#### `DC-NODE-32` — _enforced_

| Aspect | Location |
|--------|----------|
| **Source** | docs/planning/phase4-n-ak-s2-recovered-anchor-boundary-invariants.md + docs/clusters/PHASE4-N-AK/cluster.md |
| **Requirement** | docs/planning/phase4-n-ak-s2-recovered-anchor-boundary-invariants.md + docs/clusters/PHASE4-N-AK/cluster.md *(paraphrased from source/code_locus; registry `statement` empty)* |
| **Code** | crates/ade_runtime/src/forward_sync/reducer.rs (ForwardSyncState.recovered_anchor -- the threaded anchor field, default None), crates/ade_node/src/node_sync.rs (run_node_sync -- single-producer RollBack handler accepts rollback-to-recovered-anchor (exact slot AND hash) as idempotent no-op), crates/ade_node/src/node_lifecycle.rs (ON arm sets fwd.recovered_anchor = BootstrapState.tip; run_participant_sync UNCHANGED -- separate follow-on), crates/ade_runtime/src/admission/wire_pump.rs:447 (AI-S4a Origin fail-close -- UNCHANGED) |
| **Tests** | `ak_s2_rollback_to_recovered_anchor_is_idempotent_noop`; `ak_s2_rollback_to_origin_fails_closed_even_with_anchor`; `ak_s2_non_anchor_rollback_fails_closed_slot_and_hash_bound`; `ak_s2_no_recovered_anchor_still_fails_closed`; `ak_s2_after_anchor_noop_forward_block_reaches_pump_block_validation_holds`; `ak_s2_valid_forward_block_admits_after_recovered_anchor_noop`; `singleproducer_rollback_refused_by_run_node_sync` |
| **CI** | _(no CI listed in registry — enforced via Code/Tests; gap: bind a CI gate)_ |

#### `DC-NODE-33` — _enforced_

| Aspect | Location |
|--------|----------|
| **Source** | docs/planning/phase4-n-al-participant-recovered-anchor-boundary-invariants.md + docs/clusters/PHASE4-N-AL/cluster.md |
| **Requirement** | docs/planning/phase4-n-al-participant-recovered-anchor-boundary-invariants.md + docs/clusters/PHASE4-N-AL/cluster.md *(paraphrased from source/code_locus; registry `statement` empty)* |
| **Code** | crates/ade_node/src/node_lifecycle.rs (run_participant_sync RollBack handler -- recovered-anchor exact slot+hash no-op evaluated BEFORE the DC-NODE-29 durable-membership resolution; reads state.recovered_anchor set in the forge-ON arm at node_lifecycle.rs:563), crates/ade_runtime/src/forward_sync/reducer.rs (ForwardSyncState.recovered_anchor -- the existing AK-S2 field, reused unchanged), crates/ade_runtime/src/admission/wire_pump.rs:447 (AI-S4a Origin fail-close -- UNCHANGED) |
| **Tests** | `participant_rollback_to_recovered_anchor_is_noop`; `participant_rollback_origin_fails_closed`; `participant_rollback_non_anchor_fails_closed`; `participant_first_forward_after_anchor_noop_admits_via_pump_block`; `participant_stored_block_rollback_still_applies` |
| **CI** | _(no CI listed in registry — enforced via Code/Tests; gap: bind a CI gate)_ |

#### `DC-NODE-34` — _enforced_

| Aspect | Location |
|--------|----------|
| **Source** | docs/planning/phase4-select-multicandidate-fork-choice-invariants.md (FC-2/FC-10 enabler) + docs/clusters/PHASE4-N-AO/cluster.md |
| **Requirement** | docs/planning/phase4-select-multicandidate-fork-choice-invariants.md (FC-2/FC-10 enabler) + docs/clusters/PHASE4-N-AO/cluster.md *(paraphrased from source/code_locus; registry `statement` empty)* |
| **Code** | crates/ade_node/src/node_sync.rs (NodeSyncItem::{Block,RollBack} carry the source peer) + crates/ade_node/src/node_lifecycle.rs (peer-tagged participant feed) + crates/ade_node/src/convergence_evidence.rs (per-block block_received peer attribution). Gate ci/ci_check_peer_identity_preserved.sh. |
| **Tests** | `best_of_two_peers_wins_and_is_identified`; `peer_identity_preserved_through_merge` |
| **CI** | `ci/ci_check_peer_identity_preserved.sh` |

#### `DC-NODE-35` — _enforced_

| Aspect | Location |
|--------|----------|
| **Source** | docs/planning/phase4-select-multicandidate-fork-choice-invariants.md (FC-10; the BLUE-safety proof obligation) + docs/clusters/PHASE4-N-AO/cluster.md |
| **Requirement** | docs/planning/phase4-select-multicandidate-fork-choice-invariants.md (FC-10; the BLUE-safety proof obligation) + docs/clusters/PHASE4-N-AO/cluster.md *(paraphrased from source/code_locus; registry `statement` empty)* |
| **Code** | crates/ade_node/src/candidate_aggregator.rs (GREEN build_candidate_fragment -- each header validated via reused BLUE validate_and_apply_header, never minted -- + assemble_candidate_set, arrival-order independent). Gate ci/ci_check_candidate_construction_validated.sh. |
| **Tests** | `build_candidate_fragment_assembles_from_validated_headers`; `build_candidate_fragment_empty_headers_fails_closed`; `build_candidate_fragment_rejects_invalid_header_fails_closed`; `build_candidate_fragment_two_runs_byte_identical`; `assemble_candidate_set_ordering_is_arrival_independent` |
| **CI** | `ci/ci_check_candidate_construction_validated.sh` |

#### `DC-NODE-36` — _enforced_

| Aspect | Location |
|--------|----------|
| **Source** | docs/planning/phase4-select-multicandidate-fork-choice-invariants.md (FC-1/FC-2) + docs/clusters/PHASE4-N-AO/cluster.md |
| **Requirement** | docs/planning/phase4-select-multicandidate-fork-choice-invariants.md (FC-1/FC-2) + docs/clusters/PHASE4-N-AO/cluster.md *(paraphrased from source/code_locus; registry `statement` empty)* |
| **Code** | crates/ade_node/src/node_lifecycle.rs (RED dispatch_competing_fork_choice + GREEN decide_fork_switch routing the live participant path into the SOLE BLUE select_best_chain; DECIDE-only -- sets PendingForkSwitch + the DC-NODE-28 fence, applies nothing) + crates/ade_node/src/selector_state.rs (ForkAnchor/PendingForkSwitch/project_tiebreaker). BLUE select_best_chain UNCHANGED. Gate ci/ci_check_live_selector_dispatch.sh. |
| **Tests** | `win_emits_switch_to_winning_peer_and_durable_anchor`; `tiebreaker_loss_keeps_current`; `exceeded_rollback_keeps_current`; `best_of_two_peers_wins_and_is_identified` |
| **CI** | `ci/ci_check_live_selector_dispatch.sh` |

#### `DC-NODE-37` — _enforced_

| Aspect | Location |
|--------|----------|
| **Source** | docs/planning/phase4-select-multicandidate-fork-choice-invariants.md (FC-5/FC-6/FC-7) + docs/clusters/PHASE4-N-AO/cluster.md |
| **Requirement** | docs/planning/phase4-select-multicandidate-fork-choice-invariants.md (FC-5/FC-6/FC-7) + docs/clusters/PHASE4-N-AO/cluster.md *(paraphrased from source/code_locus; registry `statement` empty)* |
| **Code** | crates/ade_node/src/fork_switch.rs (GREEN-pure prevalidate_branch: bind+link+block_validity) + crates/ade_node/src/node_lifecycle.rs (RED prove_fork_switch -- mutation-free -- then apply_fork_switch: prove-then-commit via apply_chain_event RolledBack{ForkChoiceWin}+ChainSelected, fence cleared LAST; ProofFailed holds the fence). Gate ci/ci_check_fork_switch_never_abandons.sh. |
| **Tests** | `empty_branch_fails_closed_before_any_apply`; `null_source_serves_nothing`; `fork_switch_win_adopts_via_rolledback_then_chainselected`; `body_hash_mismatch_leaves_chain_unchanged`; `broken_parent_link_leaves_chain_unchanged`; `selected_peer_missing_body_leaves_chain_unchanged_fence_held`; `proof_failure_holds_fence_then_resolves_when_caught_up` |
| **CI** | `ci/ci_check_fork_switch_never_abandons.sh` |

#### `DC-NODE-38` — _enforced_

| Aspect | Location |
|--------|----------|
| **Source** | docs/planning/phase4-n-ao-ce-ao-6-live-gap.md + docs/clusters/PHASE4-N-AO/S7-lca-anchor-walk.md |
| **Requirement** | docs/planning/phase4-n-ao-ce-ao-6-live-gap.md + docs/clusters/PHASE4-N-AO/S7-lca-anchor-walk.md *(paraphrased from source/code_locus; registry `statement` empty)* |
| **Code** | crates/ade_node/src/lca_walk.rs (walk_to_durable_lca + CachedHeader: k-bounded by block depth, self-binding by re-derived hash, durable LCA anchor is ChainDb stored slot+hash only). Gate ci/ci_check_lca_anchor_walk.sh. |
| **Tests** | `one_block_fork_walks_in_one_step`; `multi_block_branch_walks_to_durable_lca`; `missing_intermediate_header_fails_closed`; `ancestor_older_than_k_fails_closed_block_depth`; `cache_self_binding_violation_fails_closed`; `lying_parent_link_to_genesis_fails_closed`; `arrival_order_permutation_walks_identical`; `walk_is_deterministic` |
| **CI** | `ci/ci_check_lca_anchor_walk.sh` |

#### `DC-NODE-39` — _enforced_

| Aspect | Location |
|--------|----------|
| **Source** | docs/planning/phase4-n-ao-ce-ao-6-live-gap.md (run-1 root-cause finding) + docs/clusters/PHASE4-N-AO/S11-post-forkchoicewin-forward-follow.md |
| **Requirement** | docs/planning/phase4-n-ao-ce-ao-6-live-gap.md (run-1 root-cause finding) + docs/clusters/PHASE4-N-AO/S11-post-forkchoicewin-forward-follow.md *(paraphrased from source/code_locus; registry `statement` empty)* |
| **Code** | crates/ade_node/src/node_lifecycle.rs (the dispatch walk-fail + materialize-fail arms emit structured MissingBridge + set pending_missing_bridge HOLD, never a silent no-op) + crates/ade_node/src/fork_switch.rs (closed MissingBridgeReason + fork_switch_fence_resolved requires pending_missing_bridge.is_none()). Gate ci/ci_check_missing_bridge_fail_closed.sh. |
| **Tests** | `post_switch_admits_winner_descendant_x_plus_1`; `post_switch_missing_bridge_emits_structured_and_holds_fence`; `missing_bridge_wrong_parent_maps_closed_code`; `late_bridge_clears_hold_on_progress`; `missing_bridge_reason_maps_lca_error_to_closed_discriminant`; `bridge_gap_injection_emits_missing_bridge`; `late_bridge_recovers_on_progress` |
| **CI** | `ci/ci_check_missing_bridge_fail_closed.sh` |

#### `DC-NODE-40` — _enforced_

| Aspect | Location |
|--------|----------|
| **Source** | docs/planning/phase4-n-ao-ce-ao-6-live-gap.md (S11 Fault-1 finding) + docs/clusters/PHASE4-N-AO/S13-rolled-back-branch-evidence-retention.md |
| **Requirement** | docs/planning/phase4-n-ao-ce-ao-6-live-gap.md (S11 Fault-1 finding) + docs/clusters/PHASE4-N-AO/S13-rolled-back-branch-evidence-retention.md *(paraphrased from source/code_locus; registry `statement` empty)* |
| **Code** | a BTreeMap<Hash32, CachedHeader> rollback-retention cache OWNED in crates/ade_node/src/node_lifecycle.rs ForgeActivation (cross-iteration, alongside pending_fork_switch/pending_missing_bridge -- NOT a run_participant_sync local, which is reborn empty each drain), populated in apply_fork_switch BEFORE the ChainEvent::RolledBack apply (walk old_tip->fork_anchor+1 via ChainDb decode_block, insert self-bound key==block_hash + k-bounded retain by security_param.0), consulted by crates/ade_node/src/lca_walk.rs walk_to_durable_lca on a per-peer-cache miss (cache.get(h).or_else(\|\| retention.get(h))). The durable-anchor check (chaindb.get_block_by_hash) is UNCHANGED. BLUE select/apply/validate… |
| **Tests** | `rollback_retains_removed_blocks_for_lca_walk`; `retained_blocks_are_not_anchors`; `retained_blocks_are_k_bounded`; `retained_block_hash_self_binds`; `genuine_gap_still_missing_bridge`; `apply_fork_switch_populates_rollback_retention` |
| **CI** | `ci/ci_check_rollback_retention_evidence.sh` |

#### `DC-NODE-41` — _enforced_

| Aspect | Location |
|--------|----------|
| **Source** | docs/planning/phase4-n-ao-ce-ao-6-live-gap.md (S11 Fault-2 finding) + docs/clusters/PHASE4-N-AO/S14-missing-bridge-range-refetch.md |
| **Requirement** | docs/planning/phase4-n-ao-ce-ao-6-live-gap.md (S11 Fault-2 finding) + docs/clusters/PHASE4-N-AO/S14-missing-bridge-range-refetch.md *(paraphrased from source/code_locus; registry `statement` empty)* |
| **Code** | crates/ade_node/src/fork_switch.rs: closed RangeRefetchOutcome{Admitted\|Unavailable\|ShortRange\|BodyHeaderMismatch\|ParentLinkMismatch\|ValidationFailed}+as_str, RangeRefetch/PostSwitchFollow types, MAX_RANGE_REFETCH_ATTEMPTS + range_refetch_should_retry (bounded-retry RED policy), PrefetchedBranchBodies::ordered_for_peer. crates/ade_node/src/node_lifecycle.rs: recover_missing_range (pump_block sole admit, per-block block_admitted evidence); dispatch_competing_fork_choice walk-fail arm sets pending_range_refetch ONLY for a winning-peer descendant ahead of the durable tip (gated on post_switch_follow.winning_peer == peer), ALONGSIDE the DC-NODE-39 floor hold; the participant relay loop… |
| **Tests** | `refetched_bridge_admits_in_order`; `refetch_failure_structured`; `short_refetch_keeps_hold`; `lying_refetch_body_rejected`; `missing_bridge_triggers_range_refetch`; `bounded_retry` |
| **CI** | `ci/ci_check_missing_bridge_refetch.sh` |

### DC-SESS

#### `DC-SESS-01` — _enforced_

| Aspect | Location |
|--------|----------|
| **Source** | docs/planning/phase4-n-l-wire-protocol-invariants.md §1 (I-1) |
| **Requirement** | docs/planning/phase4-n-l-wire-protocol-invariants.md §1 (I-1) *(paraphrased from source/code_locus; registry `statement` empty)* |
| **Code** | crates/ade_network/src/session/state.rs, crates/ade_network/src/session/core.rs |
| **Tests** | `session_blocks_frames_before_handshake`; `session_post_handshake_handshake_frame_is_peer_fatal` |
| **CI** | `ci/ci_check_session_core_closure.sh` |

#### `DC-SESS-02` — _enforced_

| Aspect | Location |
|--------|----------|
| **Source** | docs/planning/phase4-n-l-wire-protocol-invariants.md §1 (I-2) |
| **Requirement** | docs/planning/phase4-n-l-wire-protocol-invariants.md §1 (I-2) *(paraphrased from source/code_locus; registry `statement` empty)* |
| **Code** | crates/ade_network/src/session/event.rs, crates/ade_network/src/session/core.rs |
| **Tests** | `accepted_mini_protocol_round_trips_all_ids`; `accepted_mini_protocol_unknown_id_returns_none`; `accepted_mini_protocol_match_is_exhaustive`; `session_unknown_mini_protocol_id_is_peer_fatal` |
| **CI** | `ci/ci_check_mini_protocol_id_registry_closed.sh` |

#### `DC-SESS-03` — _enforced_

| Aspect | Location |
|--------|----------|
| **Source** | docs/planning/phase4-n-l-wire-protocol-invariants.md §1 (I-3) + §3 |
| **Requirement** | docs/planning/phase4-n-l-wire-protocol-invariants.md §1 (I-3) + §3 *(paraphrased from source/code_locus; registry `statement` empty)* |
| **Code** | crates/ade_network/src/session/core.rs, crates/ade_network/src/session/demux.rs |
| **Tests** | `session_replay_equivalence_holds`; `session_replay_corpus_builds_deterministically`; `frame_buffer_two_runs_deterministic` |
| **CI** | `ci/ci_check_session_core_closure.sh` |

#### `DC-SESS-04` — _enforced_

| Aspect | Location |
|--------|----------|
| **Source** | docs/planning/phase4-n-l-wire-protocol-invariants.md §1 (I-6) |
| **Requirement** | docs/planning/phase4-n-l-wire-protocol-invariants.md §1 (I-6) *(paraphrased from source/code_locus; registry `statement` empty)* |
| **Code** | crates/ade_network/src/mux/transport.rs, crates/ade_runtime/src/network/mux_pump.rs, crates/ade_runtime/src/network/n2n_dialer.rs |
| **Tests** | `mux_transport_duplex_inbound_overflow_returns_backpressure`; `mux_transport_duplex_round_trips_bytes_over_loopback` |
| **CI** | `ci/ci_check_session_no_unbounded.sh` |

#### `DC-SESS-05` — _enforced_

| Aspect | Location |
|--------|----------|
| **Source** | docs/planning/phase4-n-l-wire-protocol-invariants.md §1 (I-5) |
| **Requirement** | docs/planning/phase4-n-l-wire-protocol-invariants.md §1 (I-5) *(paraphrased from source/code_locus; registry `statement` empty)* |
| **Code** | crates/ade_network/src/session/, crates/ade_runtime/src/orchestrator/keep_alive_session.rs |
| **Tests** | `keep_alive_session_emits_one_event_per_clock_tick`; `keep_alive_session_is_pure_under_deterministic_clock`; `keep_alive_cadence_default_is_60s` |
| **CI** | `ci/ci_check_clock_seam.sh` |

#### `DC-SESS-06` — _enforced_

| Aspect | Location |
|--------|----------|
| **Source** | docs/clusters/PHASE4-N-M-FRAG/cluster.md §1 |
| **Requirement** | docs/clusters/PHASE4-N-M-FRAG/cluster.md §1 *(paraphrased from source/code_locus; registry `statement` empty)* |
| **Code** | crates/ade_network/src/session/core.rs (drain_protocol_items + codec_error_detail) |
| **Tests** | `fragmented_replay_equivalence_two_runs_byte_identical`; `malformed_cbor_at_item_boundary_returns_session_error`; `truncated_then_complete_two_step_drain` |
| **CI** | `ci/ci_check_session_proto_reassembly.sh` |

### DC-SEED

#### `DC-SEED-01` — _enforced_

| Aspect | Location |
|--------|----------|
| **Source** | docs/planning/phase4-n-m-ledger-seed-invariants.md §1 (I-A2) |
| **Requirement** | docs/planning/phase4-n-m-ledger-seed-invariants.md §1 (I-A2) *(paraphrased from source/code_locus; registry `statement` empty)* |
| **Code** | crates/ade_runtime/src/seed_import/importer.rs |
| **Tests** | `utxo_seed_two_imports_byte_identical`; `utxo_seed_btree_order_independent_of_json_order`; `utxo_seed_inline_datum_entry_round_trips`; `utxo_seed_reference_script_changes_fingerprint`; `utxo_seed_reference_script_deterministic_across_two_imports`; `utxo_seed_canonical_script_ref_encoder_known_vector` |
| **CI** | `ci/ci_check_seed_import_closure.sh`; `ci/ci_check_seed_import_full_preprod_support.sh` |

### DC-ANCHOR

#### `DC-ANCHOR-01` — _enforced_

| Aspect | Location |
|--------|----------|
| **Source** | docs/planning/phase4-n-m-ledger-seed-invariants.md §7 (#5) |
| **Requirement** | docs/planning/phase4-n-m-ledger-seed-invariants.md §7 (#5) *(paraphrased from source/code_locus; registry `statement` empty)* |
| **Code** | crates/ade_ledger/src/bootstrap_anchor/anchor.rs |
| **Tests** | `bootstrap_anchor_round_trips_via_canonical_cbor`; `bootstrap_anchor_encode_two_runs_byte_identical`; `bootstrap_anchor_decode_rejects_unknown_version`; `bootstrap_anchor_decode_rejects_trailing_bytes`; `bootstrap_anchor_decode_rejects_short_buffer`; `bootstrap_anchor_decode_rejects_wrong_outer_array_length`; `bootstrap_anchor_decode_rejects_short_hash` |
| **CI** | `ci/ci_check_bootstrap_anchor_closure.sh` |

### DC-WAL

#### `DC-WAL-01` — _enforced_

| Aspect | Location |
|--------|----------|
| **Source** | docs/planning/phase4-n-m-ledger-seed-invariants.md §1 (I-A4) |
| **Requirement** | docs/planning/phase4-n-m-ledger-seed-invariants.md §1 (I-A4) *(paraphrased from source/code_locus; registry `statement` empty)* |
| **Code** | crates/ade_ledger/src/wal/store_trait.rs |
| **Tests** | _(no tests listed in registry — enforced via Code/CI; gap: name the proving test)_ |
| **CI** | `ci/ci_check_wal_append_only.sh` |

#### `DC-WAL-02` — _enforced_

| Aspect | Location |
|--------|----------|
| **Source** | docs/planning/phase4-n-m-ledger-seed-invariants.md §1 (I-A5) |
| **Requirement** | docs/planning/phase4-n-m-ledger-seed-invariants.md §1 (I-A5) *(paraphrased from source/code_locus; registry `statement` empty)* |
| **Code** | crates/ade_ledger/src/wal/store_trait.rs, crates/ade_ledger/src/wal/replay.rs |
| **Tests** | `file_wal_store_verify_chain_passes_then_catches_break`; `replay_from_anchor_catches_chain_break`; `wal_replay_from_anchor_rejects_chain_break`; `recover_follow_kill_warm_start_chains_from_ledger_fp`; `recover_follow_zero_seed_chainbreaks`; `recover_follow_two_runs_byte_identical` |
| **CI** | `ci/ci_check_recover_follow_wal_lineage.sh` |

#### `DC-WAL-03` — _enforced_

| Aspect | Location |
|--------|----------|
| **Source** | docs/planning/phase4-n-m-ledger-seed-invariants.md §1 (I-A6) |
| **Requirement** | docs/planning/phase4-n-m-ledger-seed-invariants.md §1 (I-A6) *(paraphrased from source/code_locus; registry `statement` empty)* |
| **Code** | crates/ade_ledger/src/wal/replay.rs, crates/ade_runtime/tests/wal_replay_from_anchor.rs |
| **Tests** | `wal_replay_from_anchor_two_runs_byte_identical`; `wal_replay_from_anchor_post_fp_matches_wal_tail`; `wal_replay_from_anchor_persists_across_reopen`; `replay_from_anchor_three_entry_chain_ok`; `replay_from_anchor_two_runs_byte_identical` |
| **CI** | _(no CI listed in registry — enforced via Code/Tests; gap: bind a CI gate)_ |

#### `DC-WAL-04` — _enforced_

| Aspect | Location |
|--------|----------|
| **Source** | docs/planning/phase4-n-u-forged-block-durability-invariants.md |
| **Requirement** | docs/planning/phase4-n-u-forged-block-durability-invariants.md *(paraphrased from source/code_locus; registry `statement` empty)* |
| **Code** | crates/ade_ledger/src/wal/* (WalEntry::AdmitBlock prior_fp/post_fp + verify_chain -- reused); crates/ade_node/src/node_sync.rs (admit_forged_block_durably -> pump_block appends the forged AdmitBlock); crates/ade_node/src/node_lifecycle.rs (warm_start_recovery WAL-tail reconciliation: rollback_to_slot(wal_tail_slot) before warm-start) |
| **Tests** | `forged_admit_wal_prior_fp_chains`; `warm_start_drops_orphan_block_above_wal_tail`; `forge_tip_successor_kill_then_warm_start_recovers_block_one` |
| **CI** | `ci/ci_check_forged_durable_admit_via_pump.sh` |

#### `DC-WAL-05` — _enforced_

| Aspect | Location |
|--------|----------|
| **Source** | docs/active/durable-admission-bytes-slice.md (split-admission-authority bug, surfaced at C2-PREVIEW-BA02 when a fresh-WarmStart forge failed BlockBytesMissing on a graceful-shutdown admission store) |
| **Requirement** | docs/active/durable-admission-bytes-slice.md (split-admission-authority bug, surfaced at C2-PREVIEW-BA02 when a fresh-WarmStart forge failed BlockBytesMissing on a graceful-shutdown admission store) *(paraphrased from source/code_locus; registry `statement` empty)* |
| **Code** | crates/ade_node/src/admission/runner.rs (run_admission ProcessedBlock::Admitted arm: StoredBlock put_block BEFORE WalEntry::AdmitBlock; AdmissionExitCode::DurableBlockStoreIo / EXIT_LIVE_DURABLE_BLOCK_STORE_IO=36 fail-closed; block bytes MOVED into StoredBlock then dropped -- no retained map); crates/ade_node/src/node_lifecycle.rs (warm_start_recovery: per-AdmitBlock ChainDb::get_block_by_hash, fail-closed NodeLifecycleError::DurableBlockBytesMissing{block_hash,entry_index,source} -- never the prior silent skip) |
| **Tests** | `warmstart_from_real_admission_store_uses_persisted_bytes_no_mock`; `warmstart_fails_closed_when_wal_admitblock_missing_bytes` |
| **CI** | `ci/ci_check_admission_runner_no_block_byte_map.sh` |

### DC-ADMIT

#### `DC-ADMIT-01` — _enforced_

| Aspect | Location |
|--------|----------|
| **Source** | docs/planning/phase4-n-m-b-admission-invariants.md §1 (I-B1) + §6 table |
| **Requirement** | docs/planning/phase4-n-m-b-admission-invariants.md §1 (I-B1) + §6 table *(paraphrased from source/code_locus; registry `statement` empty)* |
| **Code** | crates/ade_node/src/admission/verdict.rs::{AgreementVerdict, BlockAdmitOutcome, derive} |
| **Tests** | `verdict_agreed_when_hashes_match`; `verdict_diverged_when_our_admit_differs_from_peer_hash`; `verdict_diverged_when_admit_invalid_at_same_slot`; `verdict_lagging_when_peer_ahead_of_our_slot`; `verdict_input_not_found_when_admit_missing_input`; `verdict_lagging_when_peer_tip_is_origin`; `verdict_derive_is_pure_two_runs_byte_identical`; `verdict_kind_discriminator_round_trips_each_variant` |
| **CI** | `ci/ci_check_lagging_is_evidence_only.sh` |

#### `DC-ADMIT-02` — _enforced_

| Aspect | Location |
|--------|----------|
| **Source** | docs/planning/phase4-n-m-b-admission-invariants.md §1 (I-B3) |
| **Requirement** | docs/planning/phase4-n-m-b-admission-invariants.md §1 (I-B3) *(paraphrased from source/code_locus; registry `statement` empty)* |
| **Code** | crates/ade_node/src/admission/runner.rs::run_admission (per-Block branch) |
| **Tests** | `run_admission_emits_shutdown_on_signal`; `admission_tip_update_does_not_emit_wal_entry` |
| **CI** | `ci/ci_check_admission_runner_closure.sh` |

#### `DC-ADMIT-03` — _enforced_

| Aspect | Location |
|--------|----------|
| **Source** | docs/planning/phase4-n-m-b-admission-invariants.md §1 (I-B2) |
| **Requirement** | docs/planning/phase4-n-m-b-admission-invariants.md §1 (I-B2) *(paraphrased from source/code_locus; registry `statement` empty)* |
| **Code** | crates/ade_node/src/admission/runner.rs::{EXIT_LIVE_AGREEMENT_DIVERGED, EXIT_LIVE_INPUT_NOT_FOUND, halt_for_verdict, halt_to_exit} |
| **Tests** | `exit_code_constants_round_trip_to_i32`; `halt_for_verdict_only_diverged_or_input_not_found_halts`; `admission_exit_codes_match_registered_values` |
| **CI** | `ci/ci_check_admission_runner_closure.sh` |

#### `DC-ADMIT-04` — _enforced_

| Aspect | Location |
|--------|----------|
| **Source** | docs/planning/phase4-n-m-b-admission-invariants.md §1 (I-B4) + §2 (¬P-B10) |
| **Requirement** | docs/planning/phase4-n-m-b-admission-invariants.md §1 (I-B4) + §2 (¬P-B10) *(paraphrased from source/code_locus; registry `statement` empty)* |
| **Code** | crates/ade_node/src/admission_log/{event.rs, writer.rs} |
| **Tests** | `admission_log_event_discriminator_round_trips_for_each_variant`; `admission_log_event_match_is_exhaustive`; `admission_log_event_agreement_verdict_carries_kind_discriminator`; `admission_log_writer_emits_one_object_per_line`; `admission_log_writer_serializes_admission_started_canonically`; `admission_log_writer_two_runs_are_byte_identical`; `admission_log_writer_emits_agreement_verdict_with_kind_field`; `admission_log_writer_omits_optional_fields_when_none`; `admission_log_writer_lines_are_parseable_as_one_json_object_per_line` |
| **CI** | `ci/ci_check_admission_log_vocabulary_closed.sh`; `ci/ci_check_convergence_evidence_vocabulary_closed.sh` |

#### `DC-ADMIT-05` — _enforced_

| Aspect | Location |
|--------|----------|
| **Source** | docs/planning/phase4-n-m-b-admission-invariants.md §1 (I-B7) |
| **Requirement** | docs/planning/phase4-n-m-b-admission-invariants.md §1 (I-B7) *(paraphrased from source/code_locus; registry `statement` empty)* |
| **Code** | crates/ade_node/src/admission/runner.rs::run_admission (per-AdmittedBlock branch → wal_store.append) |
| **Tests** | `admission_replay_equivalence_byte_identical_wal_after_two_runs`; `admission_tip_update_does_not_emit_wal_entry` |
| **CI** | `ci/ci_check_admission_runner_closure.sh`; `ci/ci_check_wal_append_only.sh` |

#### `DC-ADMIT-06` — _enforced_

| Aspect | Location |
|--------|----------|
| **Source** | docs/planning/phase4-n-m-b-admission-invariants.md §1 (I-B8) |
| **Requirement** | docs/planning/phase4-n-m-b-admission-invariants.md §1 (I-B8) *(paraphrased from source/code_locus; registry `statement` empty)* |
| **Code** | crates/ade_node/src/admission/verdict.rs::derive |
| **Tests** | `verdict_derive_is_pure_two_runs_byte_identical` |
| **CI** | `ci/ci_check_lagging_is_evidence_only.sh` |

#### `DC-ADMIT-07` — _enforced_

| Aspect | Location |
|--------|----------|
| **Source** | docs/planning/phase4-n-m-b-admission-invariants.md §1 (I-B7) |
| **Requirement** | docs/planning/phase4-n-m-b-admission-invariants.md §1 (I-B7) *(paraphrased from source/code_locus; registry `statement` empty)* |
| **Code** | crates/ade_node/tests/admission_replay_equivalence.rs |
| **Tests** | `admission_replay_equivalence_byte_identical_wal_after_two_runs`; `admission_signal_shutdown_returns_clean_exit`; `admission_disconnect_to_zero_peers_exits_clean`; `admission_tip_update_does_not_emit_wal_entry` |
| **CI** | `ci/ci_check_admit_replay_equivalence.sh` |

#### `DC-ADMIT-08` — _enforced_

| Aspect | Location |
|--------|----------|
| **Source** | docs/planning/phase4-n-m-b-admission-invariants.md §1 (I-B1) + §2 (¬P-B8) |
| **Requirement** | docs/planning/phase4-n-m-b-admission-invariants.md §1 (I-B1) + §2 (¬P-B8) *(paraphrased from source/code_locus; registry `statement` empty)* |
| **Code** | crates/ade_node/src/admission/verdict.rs::AgreementVerdict::Lagging |
| **Tests** | `verdict_lagging_when_peer_ahead_of_our_slot`; `verdict_lagging_when_peer_tip_is_origin` |
| **CI** | `ci/ci_check_lagging_is_evidence_only.sh` |

#### `DC-ADMIT-09` — _enforced_

| Aspect | Location |
|--------|----------|
| **Source** | docs/planning/phase4-n-m-b-admission-invariants.md §0 + §2 (¬P-B9) |
| **Requirement** | docs/planning/phase4-n-m-b-admission-invariants.md §0 + §2 (¬P-B9) *(paraphrased from source/code_locus; registry `statement` empty)* |
| **Code** | crates/ade_runtime/src/seed_import/importer.rs::build_canonical_tx_out (fail-fast guard) |
| **Tests** | _(no tests listed in registry — enforced via Code/CI; gap: name the proving test)_ |
| **CI** | `ci/ci_check_admission_no_refscript_skip.sh` |

#### `DC-ADMIT-10` — _enforced_

| Aspect | Location |
|--------|----------|
| **Source** | docs/planning/phase4-n-m-c-operator-pass-invariants.md §1 (I-C8) |
| **Requirement** | docs/planning/phase4-n-m-c-operator-pass-invariants.md §1 (I-C8) *(paraphrased from source/code_locus; registry `statement` empty)* |
| **Code** | crates/ade_node/src/admission_log/event.rs (AdmissionLogEvent — 4 binding variants carry consensus_inputs_fingerprint_hex), crates/ade_node/src/admission_log/writer.rs (JSONL emit of the new field), crates/ade_node/src/admission/runner.rs (fingerprint threaded through all 4 emits via consensus_fp_hex) |
| **Tests** | `admission_log_writer_emits_one_object_per_line`; `admission_log_writer_serializes_admission_started_canonically`; `admission_log_writer_emits_agreement_verdict_with_kind_field` |
| **CI** | `ci/ci_check_live_ledger_view_epoch_window.sh`; `ci/ci_check_admission_log_vocabulary_closed.sh` |

#### `DC-ADMIT-11` — _enforced_

| Aspect | Location |
|--------|----------|
| **Source** | docs/planning/phase4-n-m-c-operator-pass-invariants.md §1 (I-C9) + §2 (¬P-C2) |
| **Requirement** | docs/planning/phase4-n-m-c-operator-pass-invariants.md §1 (I-C9) + §2 (¬P-C2) *(paraphrased from source/code_locus; registry `statement` empty)* |
| **Code** | crates/ade_node/src/admission/runner.rs (pre-admit peek_block_slot + slot-window guard + AdmissionHaltReason::CrossEpochUse emit + EXIT_LIVE_CROSS_EPOCH_USE=32) |
| **Tests** | `cross_epoch_block_triggers_halt_without_admit` |
| **CI** | `ci/ci_check_live_ledger_view_epoch_window.sh` |

#### `DC-ADMIT-12` — _enforced_

| Aspect | Location |
|--------|----------|
| **Source** | docs/planning/phase4-n-m-c-operator-pass-invariants.md §1 (I-C10) + §2 (¬P-C7, ¬P-C9) |
| **Requirement** | docs/planning/phase4-n-m-c-operator-pass-invariants.md §1 (I-C10) + §2 (¬P-C7, ¬P-C9) *(paraphrased from source/code_locus; registry `statement` empty)* |
| **Code** | crates/ade_node/src/admission/runner.rs (ProcessedBlock::Undecodable arm routes to Diverged when peer tip exists at a Point::Block, else PeerSentUndecodableBytes — exit codes 30 / 34) |
| **Tests** | `admission_log_event_discriminator_round_trips_for_each_variant` |
| **CI** | `ci/ci_check_admission_wire_pump_closure.sh` |

### DC-VIEW

#### `DC-VIEW-01` — _enforced_

| Aspect | Location |
|--------|----------|
| **Source** | docs/planning/phase4-n-m-c-operator-pass-invariants.md §1 (I-C4) |
| **Requirement** | docs/planning/phase4-n-m-c-operator-pass-invariants.md §1 (I-C4) *(paraphrased from source/code_locus; registry `statement` empty)* |
| **Code** | crates/ade_runtime/src/consensus_inputs/view.rs (LiveLedgerView LedgerView impl — 4 epoch-window guards), crates/ade_node/src/admission/runner.rs (pre-admit slot guard before process_block, returns AdmissionExitCode::CrossEpochUse) |
| **Tests** | `out_of_window_epoch_returns_none`; `in_window_epoch_answers_total_active_stake`; `in_window_per_pool_lookups_return_imported_values`; `in_window_unknown_pool_returns_none`; `imported_window_schedule_uses_bundle_epoch` |
| **CI** | `ci/ci_check_live_ledger_view_epoch_window.sh` |

### DC-PUMP

#### `DC-PUMP-01` — _enforced_

| Aspect | Location |
|--------|----------|
| **Source** | docs/planning/phase4-n-m-c-operator-pass-invariants.md §1 (I-C6) + §2 (¬P-C3) |
| **Requirement** | docs/planning/phase4-n-m-c-operator-pass-invariants.md §1 (I-C6) + §2 (¬P-C3) *(paraphrased from source/code_locus; registry `statement` empty)* |
| **Code** | crates/ade_runtime/src/admission/wire_pump.rs (closed AdmissionPeerEvent emit-only; no AgreementVerdict reference in code; emit helper exhaustive over the 3 variants) |
| **Tests** | `pump_emits_tip_update_and_request_next_on_intersect_found_no_block_fetch`; `pump_emits_tip_update_on_intersect_not_found`; `rollforward_drives_block_fetch_then_request_next` |
| **CI** | `ci/ci_check_admission_wire_pump_closure.sh`; `ci/ci_check_admission_no_red_verdicts.sh` |

#### `DC-PUMP-02` — _enforced_

| Aspect | Location |
|--------|----------|
| **Source** | docs/planning/phase4-n-m-c-operator-pass-invariants.md §1 (I-C7); refined for PHASE4-N-AI AI-S4a in the PHASE4-N-AN gate triage |
| **Requirement** | docs/planning/phase4-n-m-c-operator-pass-invariants.md §1 (I-C7); refined for PHASE4-N-AI AI-S4a in the PHASE4-N-AN gate triage *(paraphrased from source/code_locus; registry `statement` empty)* |
| **Code** | crates/ade_runtime/src/admission/wire_pump.rs (handle_chain_sync: IntersectFound / IntersectNotFound / RollForward arms call tip_update; the RollBackward arm emits AdmissionPeerEvent::RollBackward -- the distinct AI-S4a fork-choice signal -- before any other action) |
| **Tests** | `pump_emits_tip_update_and_request_next_on_intersect_found_no_block_fetch`; `pump_emits_tip_update_on_intersect_not_found`; `rollforward_drives_block_fetch_then_request_next` |
| **CI** | `ci/ci_check_admission_wire_pump_closure.sh` |

#### `DC-PUMP-03` — _enforced_

| Aspect | Location |
|--------|----------|
| **Source** | docs/planning/phase4-n-am-wire-pump-keepalive-sustain-invariants.md + docs/clusters/PHASE4-N-AM/cluster.md |
| **Requirement** | docs/planning/phase4-n-am-wire-pump-keepalive-sustain-invariants.md + docs/clusters/PHASE4-N-AM/cluster.md *(paraphrased from source/code_locus; registry `statement` empty)* |
| **Code** | crates/ade_runtime/src/admission/wire_pump.rs (run_admission_wire_pump -- the keep-alive client: a tokio::select! cadence sends MsgKeepAlive Initiator under the peer timeout via the existing OutboundFrame path, advances the reused BLUE ade_network::keep_alive state machine, and validates the echoed cookie on the inbound DeliverPeerFrame{KeepAlive}; wire-only -- emits no AdmissionPeerEvent; AdmissionWirePumpError::KeepAlive fail-closed), crates/ade_network/src/keep_alive/transition.rs (BLUE keep_alive_transition -- REUSED, unchanged), crates/ade_network/src/codec/keep_alive.rs (BLUE KeepAliveMessage codec -- REUSED, unchanged) |
| **Tests** | `wire_pump_sends_keep_alive_on_quiescent_cadence`; `wire_pump_keep_alive_response_validates_cookie_no_event`; `wire_pump_keep_alive_cookie_mismatch_fails_closed` |
| **CI** | `ci/ci_check_keep_alive_wire_only.sh` |

#### `DC-PUMP-04` — _enforced_

| Aspect | Location |
|--------|----------|
| **Source** | docs/planning/phase4-n-ao-ce-ao-6-live-gap.md (S7-retry finding) + docs/clusters/PHASE4-N-AO/S8-multi-peer-wire-pump-fairness.md |
| **Requirement** | docs/planning/phase4-n-ao-ce-ao-6-live-gap.md (S7-retry finding) + docs/clusters/PHASE4-N-AO/S8-multi-peer-wire-pump-fairness.md *(paraphrased from source/code_locus; registry `statement` empty)* |
| **Code** | crates/ade_node/src/fair_merge.rs (RED per-peer bounded lanes + deterministic round-robin fair_merge: rotating cursor, closed-lane retire-in-place, no HashMap/wall-clock/rand) + crates/ade_node/src/node_lifecycle.rs (spawn_live_wire_pump_source per-peer-lane fan-in). Gate ci/ci_check_wire_pump_fairness.sh. |
| **Tests** | `hot_peer_cannot_starve_quiet_peer`; `closed_lane_removed_without_reordering_remaining_peers`; `peer_identity_preserved_through_merge` |
| **CI** | `ci/ci_check_wire_pump_fairness.sh` |

### DC-EVIDENCE

#### `DC-EVIDENCE-01` — _enforced_

| Aspect | Location |
|--------|----------|
| **Source** | docs/planning/phase4-n-m-c-operator-pass-invariants.md §1 (I-C12) |
| **Requirement** | docs/planning/phase4-n-m-c-operator-pass-invariants.md §1 (I-C12) *(paraphrased from source/code_locus; registry `statement` empty)* |
| **Code** | crates/ade_node/tests/admission_live_operator_pass.rs (env-gated integration test with the closed transcript-shape asserts), ci/build_consensus_inputs_bundle.sh (operator-side bundle generator), crates/ade_runtime/src/seed_import/ (full preprod UTxO importer; PHASE4-N-M-A1.1 closes the prior A1.1 reference-script gate AND the A1.2 Byron-address gate), docs/evidence/phase4-n-m-c-consensus-inputs.json (committed bundle from epoch 179 docker preprod), docs/evidence/phase4-n-m-c-wire-only-transcript.jsonl (wire-integration anchor), docs/evidence/phase4-n-m-a1.1-admission-bootstrap-transcript.jsonl (1.9M-entry full preprod bootstrap transcript),… |
| **Tests** | `live_operator_pass_against_docker_preprod`; `live_bundle_imports_with_conway_era_and_deterministic_fingerprint` |
| **CI** | `ci/ci_check_live_operator_pass_scaffold.sh` |

#### `DC-EVIDENCE-02` — _enforced_

| Aspect | Location |
|--------|----------|
| **Source** | docs/planning/phase4-n-m-c-operator-pass-invariants.md §1 (I-C11) |
| **Requirement** | docs/planning/phase4-n-m-c-operator-pass-invariants.md §1 (I-C11) *(paraphrased from source/code_locus; registry `statement` empty)* |
| **Code** | crates/ade_node/tests/admission_adversarial_corpus.rs (4 mandatory MutationClass variants applied to a real Conway block; each asserts exit in {Diverged(30), PeerSentUndecodableBytes(34)}) |
| **Tests** | `adversarial_corpus_rejects_all_four_mutation_classes` |
| **CI** | `ci/ci_check_adversarial_false_accept_corpus.sh` |

#### `DC-EVIDENCE-03` — _enforced (scaffolding)_

| Aspect | Location |
|--------|----------|
| **Source** | docs/planning/phase4-n-aj-participant-convergence-evidence-invariants.md §1 (I-AJ-5) + §9 |
| **Requirement** | docs/planning/phase4-n-aj-participant-convergence-evidence-invariants.md §1 (I-AJ-5) + §9 *(paraphrased from source/code_locus; registry `statement` empty)* |
| **Code** | ci/ci_check_convergence_evidence_schema.sh (gate), docs/evidence/phase4-n-ai-convergence-pass.{jsonl,md} (operator-produced, post-AJ), docs/active/phase4-n-ai-convergence-runbook.md |
| **Tests** | _(no tests listed — gap)_ |
| **CI** | `ci/ci_check_convergence_evidence_schema.sh` |

#### `DC-EVIDENCE-04` — _enforced_

| Aspect | Location |
|--------|----------|
| **Source** | docs/planning/phase4-n-ao-ce-ao-6-live-gap.md (S9 evidence finding) + docs/clusters/PHASE4-N-AO/S9-closed-fork-choice-evidence.md |
| **Requirement** | docs/planning/phase4-n-ao-ce-ao-6-live-gap.md (S9 evidence finding) + docs/clusters/PHASE4-N-AO/S9-closed-fork-choice-evidence.md *(paraphrased from source/code_locus; registry `statement` empty)* |
| **Code** | crates/ade_node/src/admission_log/{event.rs,writer.rs} (closed fork-choice AdmissionLogEvent variants + DISCRIMINATORS allow-list + closed ForkChoiceResult/ForkChoiceEvidenceFailure) + crates/ade_node/src/convergence_evidence.rs (observe-only emitters + bounded fork_switch_id) + crates/ade_node/src/node_lifecycle.rs (observe-only decide/apply taps; never read back by authority). Gate ci/ci_check_fork_choice_evidence_closed.sh. |
| **Tests** | `fork_choice_win_paired_with_exactly_one_terminal_applied`; `fork_choice_win_failed_terminal_carries_closed_code`; `superseded_win_pairs_to_superseded_terminal`; `fork_switch_id_is_deterministic_and_bounded` |
| **CI** | `ci/ci_check_fork_choice_evidence_closed.sh` |

#### `DC-EVIDENCE-05` — _enforced_

| Aspect | Location |
|--------|----------|
| **Source** | docs/clusters/PHASE4-N-AO/S10-post-switch-branch-continuity.md (S10) + docs/planning/phase4-n-ao-ce-ao-6-live-gap.md |
| **Requirement** | docs/clusters/PHASE4-N-AO/S10-post-switch-branch-continuity.md (S10) + docs/planning/phase4-n-ao-ce-ao-6-live-gap.md *(paraphrased from source/code_locus; registry `statement` empty)* |
| **Code** | crates/ade_node/src/post_switch_continuity.rs (derive_post_switch_continuity + closed PostSwitchContinuity verdict + evaluate_release_window, GREEN pure reducer) + crates/ade_node/src/bin/post_switch_continuity.rs (thin transcript->verdict bin) + prev_hash_hex on block_admitted (crates/ade_node/src/admission_log/{event,writer}.rs + convergence_evidence.rs emit_block_admitted/emit_admit_and_verdict) sourced from PumpTip.prev_hash (crates/ade_runtime/src/forward_sync/pump.rs) + ForkSwitchOutcome::Adopted.new_tip_prev + the fork-switch-adopt + admission-runner emit sites in crates/ade_node/src/node_lifecycle.rs + crates/ade_node/src/admission/runner.rs. BLUE selector/walk/apply/validate… |
| **Tests** | `continuity_ok_yields_continues_selected_branch`; `broken_parent_link_yields_broken_lineage`; `post_switch_diverged_yields_diverged`; `win_without_terminal_yields_dangling`; `continuity_verdict_ignores_peer_tip`; `post_switch_continuity_replays_byte_identical`; `release_window_passes_on_validated_prefix`; `release_window_passes_on_agreed_descendant`; `release_window_prefix_requires_a_followed_descendant`; `release_window_terminal_outside_window_fails`; … (+1 more) |
| **CI** | `ci/ci_check_post_switch_convergence_window.sh` |

### DC-PROD

#### `DC-PROD-01` — _enforced_

| Aspect | Location |
|--------|----------|
| **Source** | docs/planning/phase4-n-q-invariants.md §1 (I11); §2 (N9, N12, N13, N15) |
| **Requirement** | Producer-mode evidence log emits a closed `ProducerLogEvent` vocabulary: handshake_ok, slot_tick, leader_elected, block_forged, block_served, peer_chain_tip_observed, slot_missed{reason: closed_enum}, coordinator_shutdown{reason: closed_enum}. No free-form reason strings; no key material; no path strings. Socket addresses MUST NOT appear inside the replayable event stream — `PeerId` is an opaque `u64` (coordinator-internal counter); socket addresses are RED operational metadata, surfaced separately and excluded from replay-equivalence comparison. Mirrors the LiveLogEvent / AdmissionLogEvent precedent established in N-L / N-M while remaining a distinct vocabulary. |
| **Code** | crates/ade_runtime/src/producer/producer_log.rs (closed enum + closed reason sub-enums); crates/ade_node/src/produce_mode.rs (RED writer) |
| **Tests** | `event_kinds_are_distinct_and_stable`; `json_serialization_round_trips_byte_identical_for_replay`; `no_string_fields_in_any_variant`; `slot_missed_reason_serializes_to_stable_strings`; `produce_mode_starts_runs_three_slots_and_exits_via_max_slots` |
| **CI** | _(no CI listed in registry — enforced via Code/Tests; gap: bind a CI gate)_ |

#### `DC-PROD-02` — _enforced_

| Aspect | Location |
|--------|----------|
| **Source** | docs/planning/phase4-n-q-invariants.md §1 (I7, I8); §3 (D5); §4 (R3, R4); §8 |
| **Requirement** | Coordinator slot-tick + forge-result stream replay-equivalence. For a fixed initial CoordinatorState, fixed canonical slot-tick sequence, fixed ledger state, fixed opcert public metadata, and fixed RED forge-result event stream (ForgeSucceeded \| ForgeFailed sequence), the coordinator emits byte-identical broadcast effects and byte-identical ProducerLogEvent sequence. Wall-clock real-time timestamps and socket arrival order are non-load-bearing RED metadata. Replay is over canonical event streams — NOT real wall-clock time. The forge-result event stream is the canonical surface across the RED-key-custody boundary; the GREEN coordinator is replayable against it without ever seeing secret material. |
| **Code** | crates/ade_runtime/src/producer/coordinator.rs (GREEN reducer + S2 inline replay test) |
| **Tests** | `replay_byte_identity_across_two_runs` |
| **CI** | _(no CI listed in registry — enforced via Code/Tests; gap: bind a CI gate)_ |

#### `DC-PROD-03` — _enforced_

| Aspect | Location |
|--------|----------|
| **Source** | docs/planning/phase4-n-t-invariants.md §3, §4 (R1, R3); docs/clusters/PHASE4-N-T/cluster.md §1.5, §7 |
| **Requirement** | Producer chain-forward continuity + replay. The GREEN ChainEvolution linear typestate threads each forge's post-state (post-ledger, post-chain_dep, new tip) into the next forge's base; forging against a stale base is structurally unrepresentable (advance consumes self). advance obtains the post-state from BLUE block_validity and the AcceptedBlock token from BLUE self_accept against identical inputs (same pre-forge base, forged bytes, era_schedule, ledger_view); if the two authorities disagree it returns ChainEvolutionError::AuthorityMismatch and does not advance. ChainEvolution never constructs AcceptedBlock directly. For a fixed (bootstrap seed, canonical slot-sequence, KES/VRF/cold keys) the chain-evolution series (block_number, prev_hash, post-ledger fingerprint, post-chain_dep) and the forged block bytes are byte-identical across runs (in-memory two-run; no on-disk replay corpus — durability deferred to N-U). |
| **Code** | crates/ade_runtime/src/producer/chain_evolution.rs (ChainEvolution seed/derive_forge_context/advance + ChainEvolutionError incl. AuthorityMismatch) |
| **Tests** | `advance_threads_post_state_forward`; `advance_two_runs_byte_identical`; `advance_rejects_invalid_bytes`; `reconcile_verdicts_both_valid_ok`; `reconcile_verdicts_both_invalid_ok`; `reconcile_verdicts_valid_vs_reject_mismatches`; `reconcile_verdicts_reject_vs_valid_mismatches`; `served_snapshot_two_run_replay_byte_identical` |
| **CI** | _(no CI listed in registry — enforced via Code/Tests; gap: bind a CI gate)_ |

### DC-FORGE

#### `DC-FORGE-01` — _enforced_

| Aspect | Location |
|--------|----------|
| **Source** | docs/planning/phase4-n-r-invariants.md §3 (D1); §4 (R3) |
| **Requirement** | Given the same canonical input set (slot, eta0, vrf_vk, vrf_proof_or_output, LeaderScheduleAnswer), verify_and_evaluate_leader produces a byte-identical LeaderCheckVerdict across runs. Replay-equivalence anchor for the BLUE leader-check evaluator. Strengthens the existing leader-check determinism (DC-CONS-13 family) by exposing leader eligibility as a callable, replay-anchored function — not just an internal step in forge_block. |
| **Code** | crates/ade_core/src/consensus/leader_check.rs (verdict_is_byte_identical_across_two_runs unit test); crates/ade_node/tests/forge_handler_variants.rs (run_real_forge_is_byte_identical_across_two_runs end-to-end pipeline anchor); crates/ade_node/src/node_sync.rs (forge_from_recovered_is_deterministic_across_two_runs — leader-check determinism over the recovered-state forge path) |
| **Tests** | `verdict_is_byte_identical_across_two_runs`; `run_real_forge_is_byte_identical_across_two_runs`; `forge_from_recovered_is_deterministic_across_two_runs` |
| **CI** | _(no CI listed in registry — enforced via Code/Tests; gap: bind a CI gate)_ |

### DC-SNAPSHOT

#### `DC-SNAPSHOT-01` — _enforced_

| Aspect | Location |
|--------|----------|
| **Source** | docs/planning/phase4-n-r-invariants.md §3 (D6); §4 (R2) |
| **Requirement** | ServedChainHandle::push_atomic is deterministic in its argument order: the same sequence of push_atomic(a₀), push_atomic(a₁), ..., push_atomic(aₙ) produces a byte-identical ServedChainView (fingerprint equals over BTreeMap insertion order). Replay-equivalence anchor for the broadcast → serve path. |
| **Code** | crates/ade_runtime/src/producer/served_chain_handle.rs (push_atomic uses send_modify with served_chain_admit — the established broadcast_to_served drain-and-admit determinism carries through); crates/ade_runtime/src/producer/broadcast_to_served.rs (existing determinism tests cover the underlying invariant; push_atomic is a thin closure around the same primitive) |
| **Tests** | `drain_and_admit_is_deterministic_over_arrival_sequence`; `drain_and_admit_no_io_no_clock`; `drain_and_admit_admits_every_queued_block` |
| **CI** | _(no CI listed in registry — enforced via Code/Tests; gap: bind a CI gate)_ |

### DC-OPCERT

#### `DC-OPCERT-01` — _enforced_

| Aspect | Location |
|--------|----------|
| **Source** | docs/planning/phase4-n-r-invariants.md §3 (D4) |
| **Requirement** | Given the same canonical envelope bytes, parse_opcert_envelope produces a byte-identical DecodedOpCertEnvelope across runs. Replay-equivalence anchor for the opcert envelope decode. |
| **Code** | crates/ade_runtime/src/producer/opcert_envelope.rs (parser_is_byte_identical_across_two_runs unit test) |
| **Tests** | `parser_is_byte_identical_across_two_runs` |
| **CI** | _(no CI listed in registry — enforced via Code/Tests; gap: bind a CI gate)_ |

### DC-GENESIS

#### `DC-GENESIS-01` — _enforced_

| Aspect | Location |
|--------|----------|
| **Source** | docs/planning/phase4-n-r-invariants.md §3 (D5) |
| **Requirement** | Given the same canonical Shelley genesis JSON bytes + the same operator-supplied kes_anchor_slot, parse_shelley_genesis produces a byte-identical GenesisAnchor across runs. Replay-equivalence anchor for the genesis closed-contract parser. The ISO 8601 → Unix epoch milliseconds conversion (parse_iso8601_to_unix_ms) is deterministic without any chrono/time crate dependency. |
| **Code** | crates/ade_runtime/src/producer/genesis_parser.rs (parser_is_byte_identical_across_two_runs unit test) |
| **Tests** | `parser_is_byte_identical_across_two_runs` |
| **CI** | _(no CI listed in registry — enforced via Code/Tests; gap: bind a CI gate)_ |

#### `DC-GENESIS-SRC-01` — _enforced_

| Aspect | Location |
|--------|----------|
| **Source** | docs/clusters/PHASE4-N-Y/S4-conway-genesis-source.md |
| **Requirement** | docs/clusters/PHASE4-N-Y/S4-conway-genesis-source.md *(paraphrased from source/code_locus; registry `statement` empty)* |
| **Code** | crates/ade_ledger/src/genesis_source.rs (genesis_initial_state + closed GenesisSourceError); crates/ade_runtime/src/genesis_bootstrap.rs |
| **Tests** | `conway_genesis_bootstrap_through_single_authority`; `genesis_non_conway_fail_closed`; `genesis_to_initial_state_deterministic`; `genesis_path_fp_equals_snapshot_path_fp` |
| **CI** | `ci/ci_check_mithril_uses_bootstrap_initial_state.sh` |

### DC-KES

#### `DC-KES-HEADER-01` — _enforced_

| Aspect | Location |
|--------|----------|
| **Source** | docs/planning/phase4-n-s-invariants.md §3 (D1) |
| **Requirement** | unsigned_header_pre_image(slot, block_no, prev_hash, vrf_data, opcert, kes_period, hot_vkey, body_hash, body_size, protocol_version) is a pure BLUE function. Same canonical inputs → byte-identical UnsignedHeaderPreImage output. Replay-equivalence anchor for the pre-image recipe. |
| **Code** | crates/ade_ledger/src/block_validity/unsigned_header_pre_image.rs (recipe_output_is_byte_identical_across_two_runs unit test) |
| **Tests** | `recipe_output_is_byte_identical_across_two_runs` |
| **CI** | _(no CI listed in registry — enforced via Code/Tests; gap: bind a CI gate)_ |

### DC-OUTBOUND

#### `DC-OUTBOUND-FIFO-01` — _enforced_

| Aspect | Location |
|--------|----------|
| **Source** | docs/planning/phase4-n-s-invariants.md §3 (D4) |
| **Requirement** | The per-peer outbound channel preserves FIFO order: OutboundCommands enqueued for PeerId(p) in order O₁..Oₙ arrive at the peer's TCP socket in the same order (mpsc::Sender::send guarantees FIFO; MuxPump's session-aware encoder processes them sequentially). |
| **Code** | crates/ade_runtime/src/network/outbound_command.rs + mux_pump.rs (FIFO is structurally guaranteed by tokio::sync::mpsc::Sender's FIFO contract + MuxPump's sequential session::step processing) |
| **Tests** | _(no tests listed in registry — enforced via Code/CI; gap: name the proving test)_ |
| **CI** | _(no CI listed in registry — enforced via Code/Tests; gap: bind a CI gate)_ |

### DC-MITHRIL

#### `DC-MITHRIL-01` — _enforced_

| Aspect | Location |
|--------|----------|
| **Source** | docs/clusters/PHASE4-N-Y/S7-real-mithril-binding.md |
| **Requirement** | docs/clusters/PHASE4-N-Y/S7-real-mithril-binding.md *(paraphrased from source/code_locus; registry `statement` empty)* |
| **Code** | crates/ade_ledger/src/bootstrap_anchor/binding.rs |
| **Tests** | `mithril_anchor_binding_is_deterministic`; `mithril_anchor_rejects_field_mismatch`; `mithril_binding_rejects_certified_point_other_than_seed_point` |
| **CI** | `ci/ci_check_mithril_uses_bootstrap_initial_state.sh` |

#### `DC-MITHRIL-02` — _enforced_

| Aspect | Location |
|--------|----------|
| **Source** | docs/clusters/PHASE4-N-Z/cluster.md; S1-mithril-production-bootstrap.md |
| **Requirement** | docs/clusters/PHASE4-N-Z/cluster.md; S1-mithril-production-bootstrap.md *(paraphrased from source/code_locus; registry `statement` empty)* |
| **Code** | crates/ade_runtime/src/mithril_bootstrap.rs (bootstrap_from_mithril_snapshot — seed_point from operator inputs, verify-before-bootstrap, closed MithrilBootstrapError); ci/ci_check_mithril_seed_point_independence.sh (containment gate) |
| **Tests** | `mithril_bootstrap_fails_closed_on_seed_point_mismatch`; `mithril_bootstrap_verifies_before_storage_init`; `mithril_bootstrap_succeeds_when_seed_point_matches` |
| **CI** | `ci/ci_check_mithril_seed_point_independence.sh` |

#### `DC-MITHRIL-03` — _enforced_

| Aspect | Location |
|--------|----------|
| **Source** | docs/clusters/MITHRIL-VERIFIED-ANCHOR-INTEGRATION/SLICE-S1b-authority-transition.md; user directive 2026-06-23 (S1b = the native authority transition: assemble the complete seed from ONLY manifest/S1a/Stage-2/genesis, enforce point coherence, persist atomically before visibility, and remove the cardano-cli/JSON seed from the native bootstrap path; do NOT implement the Stage-2 UTxO materialization or the Conway per-byte min-UTxO calculator -- those are separate release blockers) |
| **Requirement** | user directive 2026-06-23 (S1b = the native authority transition: assemble the complete seed from ONLY manifest/S1a/Stage-2/genesis, enforce point coherence, persist atomically before visibility, and remove the cardano-cli/JSON seed… *(paraphrased from source/code_locus; registry `statement` empty)* |
| **Code** | crates/ade_runtime/src/mithril_native_assembly.rs: VerifiedManifestBinding + NativeGenesisConstants + NativeMithrilSeed + MithrilNativeAssemblyError (NonConwayEra/PointMismatch/PointHashMismatch/EpochMismatch/NetworkMismatch/EpochWindowUnresolved); assemble_native_mithril_seed (the pure assembly + point-coherence terminal gate; field sources documented per field; native_consensus_inputs builds the LiveConsensusInputsCanonical via canonical_from_raw with NATIVE_SOURCE_MARKER + protocol_params_json=None; native_protocol_params_hash = blake2b(encode_pparams); chain_dep_from_nonces maps the five S1a nonces; derive_epoch_window from the era schedule); bootstrap_from_native_mithril_snapshot… |
| **Tests** | `native_assembled_seed_is_deterministic`; `native_assembly_maps_each_field_from_its_source`; `point_mismatch_is_terminal`; `point_hash_mismatch_is_terminal`; `wrong_era_is_terminal`; `wrong_network_is_terminal`; `epoch_mismatch_is_terminal`; `native_bootstrap_persists_and_anchor_point_is_recoverable`; `interrupted_persist_leaves_no_discoverable_anchor_lineage` |
| **CI** | `ci/ci_check_mithril_authority_transition.sh` |

#### `DC-MITHRIL-04` — _enforced_

| Aspect | Location |
|--------|----------|
| **Source** | docs/clusters/EPOCH-CONSENSUS-VIEW/SLICE-MITHRIL-VERIFIED-ANCHOR-IMPORT.md; user directive 2026-06-22/23 (the bounty bootstrap is a verified Mithril Cardano DB snapshot decoded natively; Stage 1 is a narrow non-emitting format-and-fidelity slice replacing the test loader's zeroed-VRF shortcut with a production-faithful state decoder before the tables path; the manifest-certified point is authoritative, the filename only a locator; telescope navigation explicit + era-tagged; real VRF… |
| **Requirement** | user directive 2026-06-22/23 (the bounty bootstrap is a verified Mithril Cardano DB snapshot decoded natively; Stage 1 is a narrow non-emitting format-and-fidelity slice replacing the test loader's zeroed-VRF shortcut with a… *(paraphrased from source/code_locus; registry `statement` empty)* |
| **Code** | crates/ade_ledger/src/ledgerdb_state.rs: probe_ledgerdb_state (the entry; LedgerDbStateProbe the sole output), navigate_to_current_era (explicit era-tagged telescope nav -> UnsupportedEra), read_pool_params (ZeroVrf), read_cert_state/read_pool_map/read_dstate (CertState w/ real VRF + delegations/rewards), read_pool_distr + the PoolDistrVrfMismatch cross-check, extract_praos_nonces_v2 (the trailing-5 PraosState nonces), map_each (CBOR indefinite maps), the round-trip self-check (decode_cert_state), LedgerDbStateError (the closed terminal set). crates/ade_ledger/tests/ledgerdb_state_hermetic.rs (synthetic minimal-V2 fail-closed + round-trip + determinism).… |
| **Tests** | `happy_minimal_state_decodes_with_required_elements`; `determinism_same_bytes_same_commitment`; `zero_vrf_is_terminal`; `wrong_era_is_terminal_no_fallback_to_latest`; `pool_distr_vrf_mismatch_is_terminal`; `epoch_mismatch_is_terminal`; `malformed_cbor_is_terminal`; `decode_local_preview_corpus`; `decode_verified_mithril_ledger_state` |
| **CI** | `ci/ci_check_ledgerdb_state_decode.sh` |

#### `DC-MITHRIL-05` — _enforced_

| Aspect | Location |
|--------|----------|
| **Source** | docs/clusters/EPOCH-CONSENSUS-VIEW/SLICE-MITHRIL-VERIFIED-ANCHOR-IMPORT.md; user directive 2026-06-23 (Stage 2 = the V2 LedgerDB tables + MemPack TxOut COMPATIBILITY decoder, NOT a CBOR reader; faithful u64 quantity with the i64 ceiling logged as a separate BLOCKING downstream validation obligation, never widen MultiAsset inside the snapshot-decoder slice -- different authority surface; preserve original script/inline-datum bytes where Cardano hash/identity rules require wire bytes; the… |
| **Requirement** | user directive 2026-06-23 (Stage 2 = the V2 LedgerDB tables + MemPack TxOut COMPATIBILITY decoder, NOT a CBOR reader; faithful u64 quantity with the i64 ceiling logged as a separate BLOCKING downstream validation obligation, never widen… *(paraphrased from source/code_locus; registry `statement` empty)* |
| **Code** | crates/ade_ledger/src/ledgerdb_tables.rs: MemPackReader (explicit read_u16/u32/u64_le, read_varlen BE-7bit, expect_consumed); read_compact_addr/validate_address_form; read_staking_credential/read_addr28_base_address (PO#1 BE->LE double-flip + payment/stake hash asymmetry); read_compact_value/decode_multiasset_rep (faithful u64; rep regions A-E + nubOrd); read_compact_coin (tag-2/3 standalone CompactForm Coin = [0x00][VarLen]); read_datum_option/read_script (preserved bytes + Conway Plutus language byte); read_txout (6-tag dispatch); TxOutValue (u64 assets); canonical_txout_bytes + decode_tables_commitment (PO#2 era binding + deterministic sorted commitment); TablesDecodeError (closed… |
| **Tests** | `multiasset_quantities_preserved_exactly_as_u64_no_i64_cast`; `coin_varlen_overflow_is_terminal`; `addr28_base_address_reconstruction_round_trip`; `staking_credential_tag_is_fail_closed`; `compact_value_ada_only_and_multiasset`; `txout_dispatch_tag0_tag5_and_fail_closed`; `tables_commitment_deterministic_era_bound_and_sorted`; `varlen_big_endian_7bit_matches_real_coin`; `decode_real_preprod_tables_commitment` |
| **CI** | `ci/ci_check_ledgerdb_tables_decode.sh` |

#### `DC-MITHRIL-06` — _enforced_

| Aspect | Location |
|--------|----------|
| **Source** | docs/clusters/MITHRIL-VERIFIED-ANCHOR-INTEGRATION/SLICE-S1c-tables-to-utxostate.md; user directive 2026-06-24 (S1c = the Stage-2 tables -> authoritative UTxOState materialization: a pure DecodedTxOut -> ledger TxOut converter with the hash-critical inline-datum / reference-script bytes embedded verbatim via the tag-24 authority and the faithful u64 quantities carried into OutputAssetQuantity, canonical-ascending TxIn materialization into UTxOState::from_map, fail-closed on any unsupported… |
| **Requirement** | user directive 2026-06-24 (S1c = the Stage-2 tables -> authoritative UTxOState materialization: a pure DecodedTxOut -> ledger TxOut converter with the hash-critical inline-datum / reference-script bytes embedded verbatim via the tag-24… *(paraphrased from source/code_locus; registry `statement` empty)* |
| **Code** | crates/ade_ledger/src/mithril_utxo_materialize.rs: decoded_txout_to_ledger (the pure converter + TxOutMaterializeError closed enum), build_multi_asset (u64 -> OutputAssetQuantity, faithful Word64), encode_conway_txout_raw (the canonical Conway TxOut map; inline-datum + script bytes embedded verbatim via ade_codec::wrap_tag24), encode_script_inner (Native->[0,bytes] / PlutusVn->[n,bytes]), write_value (coin uint \| [coin, {policy:{name:qty}}] canonical-sorted, qty a CBOR uint), parse_txin_key (34-byte 32+2 BE), materialize_tables_to_utxo (canonical-ascending Conway-bound fail-closed -> UTxOState::from_map), bind_utxo_to_manifest / verify_utxo_binding / UtxoBindingRecord /… |
| **Tests** | `deterministic_utxo_commitment`; `u64_above_i64_max_materializes_persists_recovers_exactly`; `datum_and_script_bytes_preserved_verbatim_in_raw`; `alonzo_plus_raw_round_trips_to_same_fields`; `canonical_txin_ordering_asserted`; `fail_closed_negatives`; `binding_is_terminal_on_mismatch`; `persist_recover_identical_fingerprint`; `no_datum_no_script_is_shelley_mary_byron_is_byron`; `materialized_count_matches_stage2_commitment_count`; … (+1 more) |
| **CI** | `ci/ci_check_tables_to_utxostate.sh` |

#### `DC-MITHRIL-07` — _enforced_

| Aspect | Location |
|--------|----------|
| **Source** | docs/clusters/MITHRIL-VERIFIED-ANCHOR-INTEGRATION/SLICE-S1d-live-firstrun-native.md; user directive 2026-06-24 (S1d = wire the live --mode node FirstRun to the native Mithril bootstrap: route manifest + state + tables + Shelley genesis through the unchanged S1a/S1b/S1c chain, forbid the cardano-cli/JSON seed on the native route with a structured terminal on a forbidden flag, build the snapshot-epoch era schedule from the genesis + the per-network Shelley boundary without the operator bundle,… |
| **Requirement** | user directive 2026-06-24 (S1d = wire the live --mode node FirstRun to the native Mithril bootstrap: route manifest + state + tables + Shelley genesis through the unchanged S1a/S1b/S1c chain, forbid the cardano-cli/JSON seed on the… *(paraphrased from source/code_locus; registry `statement` empty)* |
| **Code** | crates/ade_node/src/native_firstrun.rs: native_first_run_bootstrap (the native orchestration: import_mithril_manifest_from_bytes -> VerifiedManifestBinding; parse_native_shelley_genesis -> NativeGenesisFacts {NativeGenesisConstants + epoch_length_slots; activeSlotsCoeff via decimal_text_to_rational, no float}; shelley_boundary_for_magic (closed per-network Shelley start); epoch_for_certified_slot; decode_native_nonutxo_state; materialize_tables_to_utxo(.., CONWAY_ERA_INDEX, None); build_native_schedule (single-era Conway anchored at the snapshot epoch's absolute start); bootstrap_from_native_mithril_snapshot) + NativeFirstRunError… |
| **Tests** | `native_first_run_forbidden_json_seed_is_terminal`; `native_first_run_forbidden_consensus_inputs_is_terminal`; `native_first_run_missing_manifest_is_terminal`; `native_first_run_missing_shelley_genesis_is_terminal`; `native_first_run_malformed_manifest_is_terminal`; `native_first_run_malformed_shelley_genesis_is_terminal`; `native_first_run_real_snapshot_invokes_bootstrap_and_persists`; `native_first_run_real_snapshot_wrong_network_is_terminal`; `preprod_snapshot_epoch_window_contains_manifest_point`; `shelley_genesis_active_slots_coeff_decimal_to_rational`; … (+2 more) |
| **CI** | `ci/ci_check_native_firstrun_no_cli_seed.sh` |

### DC-SYNC

#### `DC-SYNC-01` — _enforced_

| Aspect | Location |
|--------|----------|
| **Source** | docs/clusters/PHASE4-N-Y/S2-network-forward-sync.md; S6-torn-write-recovery-reconciliation.md |
| **Requirement** | docs/clusters/PHASE4-N-Y/S2-network-forward-sync.md; S6-torn-write-recovery-reconciliation.md *(paraphrased from source/code_locus; registry `statement` empty)* |
| **Code** | crates/ade_runtime/src/forward_sync/{reducer,pump}.rs; crates/ade_runtime/src/recovery/restart.rs (WAL-tail reconciliation) |
| **Tests** | `forward_sync_wal_and_bytes_precede_tip_advance`; `forward_sync_replay_two_runs_byte_identical`; `forward_sync_admission_through_chokepoints`; `recovery_torn_put_block_before_wal_append_drops_orphan`; `node_sync_pump_advances_recoverable_tip`; `node_sync_fails_closed_on_undecodable_block`; `node_sync_kill_then_warm_start_recovers_same_tip` |
| **CI** | `ci/ci_check_forward_sync_chokepoint_only.sh`; `ci/ci_check_node_sync_via_pump.sh` |

#### `DC-SYNC-02` — _enforced_

| Aspect | Location |
|--------|----------|
| **Source** | docs/planning/phase4-n-f-d-live-node-run-loop-invariants.md |
| **Requirement** | docs/planning/phase4-n-f-d-live-node-run-loop-invariants.md *(paraphrased from source/code_locus; registry `statement` empty)* |
| **Code** | crates/ade_node/src/node_sync.rs |
| **Tests** | `relay_loop_syncs_then_halts_clean_on_source_end`; `relay_loop_idles_then_syncs_on_incremental_feed`; `relay_loop_fails_closed_on_unapplyable_block`; `node_sync_pump_advances_recoverable_tip` |
| **CI** | `ci/ci_check_node_run_loop_containment.sh`; `ci/ci_check_node_sync_via_pump.sh` |

### DC-COMPAT

#### `DC-COMPAT-01` — _enforced_

| Aspect | Location |
|--------|----------|
| **Source** | docs/clusters/PHASE4-N-Y/S5-compatibility-evidence.md |
| **Requirement** | docs/clusters/PHASE4-N-Y/S5-compatibility-evidence.md *(paraphrased from source/code_locus; registry `statement` empty)* |
| **Code** | crates/ade_testkit/src/harness/sync_diff.rs; ci/ci_check_no_haskell_fingerprint_equality.sh |
| **Tests** | `sync_differential_snapshot_to_tip` |
| **CI** | `ci/ci_check_no_haskell_fingerprint_equality.sh` |

### DC-CINPUT

#### `DC-CINPUT-01` — _enforced_

| Aspect | Location |
|--------|----------|
| **Source** | docs/clusters/PHASE4-N-F-A/cluster.md; A3a-wal-provenance-entry.md; A3b-warm-start-restore.md |
| **Requirement** | docs/clusters/PHASE4-N-F-A/cluster.md; A3a-wal-provenance-entry.md; A3b-warm-start-restore.md *(paraphrased from source/code_locus; registry `statement` empty)* |
| **Code** | crates/ade_ledger/src/wal/event.rs (SeedEpochConsensusInputsImported, tag 3); crates/ade_ledger/src/wal/replay.rs (ReplayOutcome + RecoveredBootstrapProvenance); crates/ade_runtime/src/seed_consensus_provenance.rs (append helper); crates/ade_runtime/src/bootstrap.rs (SeedEpochConsensusSource, BootstrapState, restore_seed_epoch_consensus_inputs) |
| **Tests** | `wal_seed_cinput_entry_round_trips_byte_identical`; `replay_yields_bootstrap_provenance_view`; `replay_rejects_duplicate_provenance_entry`; `replay_rejects_anchor_mismatched_provenance_entry`; `admit_block_chain_unaffected_by_provenance_entry`; `warm_start_restores_seed_epoch_consensus_inputs_byte_identical`; `warm_start_fails_closed_on_missing_sidecar`; `warm_start_fails_closed_on_hash_mismatch`; `warm_start_fails_closed_on_anchor_mismatch`; `warm_start_fails_closed_on_epoch_mismatch`; … (+8 more) |
| **CI** | `ci/ci_check_lifecycle_owner_uses_bootstrap_initial_state.sh` |

#### `DC-CINPUT-02a` — _enforced_

| Aspect | Location |
|--------|----------|
| **Source** | docs/clusters/PHASE4-N-F-A/cluster.md; A4-projection-pooldistr-vrf.md |
| **Requirement** | docs/clusters/PHASE4-N-F-A/cluster.md; A4-projection-pooldistr-vrf.md *(paraphrased from source/code_locus; registry `statement` empty)* |
| **Code** | crates/ade_ledger/src/consensus_view.rs (PoolDistrView::from_seed_epoch_consensus_inputs); crates/ade_core/src/consensus/vrf_cert.rs (leader_vrf_input, reused) |
| **Tests** | `recovered_surface_projects_pooldistrview_and_expected_vrf_input`; `projection_maps_recovered_fields_onto_ledgerview_surface`; `projection_two_runs_identical`; `projection_off_epoch_returns_none` |
| **CI** | _(no CI listed in registry — enforced via Code/Tests; gap: bind a CI gate)_ |

#### `DC-CINPUT-02b` — _enforced_

| Aspect | Location |
|--------|----------|
| **Source** | docs/clusters/PHASE4-N-F-C/cluster.md; L5-produce-from-recovered-state.md |
| **Requirement** | docs/clusters/PHASE4-N-F-C/cluster.md; L5-produce-from-recovered-state.md *(paraphrased from source/code_locus; registry `statement` empty)* |
| **Code** | crates/ade_node/src/node_sync.rs (forge_one_from_recovered: recovered BootstrapState -> PoolDistrView::from_seed_epoch_consensus_inputs -> ForgeRequestContext -> run_real_forge) |
| **Tests** | `forge_from_recovered_uses_recovered_pool_distr`; `forge_from_recovered_fails_closed_without_recovered_inputs`; `forge_from_recovered_is_deterministic_across_two_runs` |
| **CI** | `ci/ci_check_consensus_input_provenance.sh`; `ci/ci_check_recovered_ledger_pparams_sourced.sh` |

#### `DC-CINPUT-03` — _enforced_

| Aspect | Location |
|--------|----------|
| **Source** | docs/clusters/PHASE4-N-F-G-N/cluster.md |
| **Requirement** | docs/clusters/PHASE4-N-F-G-N/cluster.md *(paraphrased from source/code_locus; registry `statement` empty)* |
| **Code** | crates/ade_core/src/consensus/vrf_cert.rs (praos_vrf_input = blake2b256(slot_be8 ‖ eta0_32) = mkInputVRF); crates/ade_runtime/src/producer/signing.rs (vrf_prove over the alpha); crates/ade_ledger/src/seed_consensus_inputs.rs (epoch_nonce sidecar field); crates/ade_runtime/src/bootstrap.rs (overlay onto chain_dep) |
| **Tests** | `warm_start_overlays_recovered_eta0_onto_chain_dep_g_n`; `pinning_praos_vrf_input_and_threshold_match_fixture`; `recovered_surface_projects_pooldistrview_and_expected_vrf_input` |
| **CI** | `ci/ci_check_warmstart_eta0_overlay.sh` |

#### `DC-CINPUT-04` — _enforced_

| Aspect | Location |
|--------|----------|
| **Source** | docs/clusters/PHASE4-N-F-G-P/cluster.md |
| **Requirement** | docs/clusters/PHASE4-N-F-G-P/cluster.md *(paraphrased from source/code_locus; registry `statement` empty)* |
| **Code** | crates/ade_node/src/node_lifecycle.rs (the forge-on On-arm feed ledger_view is PoolDistrView::from_seed_epoch_consensus_inputs(recovered) -- NOT PoolDistrView::new(empty); fail-closed FeedMissingRecoveredConsensusInputs when --peer is set but the recovered record is absent); crates/ade_ledger/src/consensus_view.rs (PoolDistrView::from_seed_epoch_consensus_inputs -- the single projection authority shared with the forge); crates/ade_node/src/node_sync.rs (forge_one_from_recovered uses the SAME projection -- DC-CINPUT-02b) |
| **Tests** | `feed_header_validates_against_recovered_surface_not_empty_view` |
| **CI** | `ci/ci_check_feed_leader_threshold_view.sh` |

#### `DC-CINPUT-05` — _enforced_

| Aspect | Location |
|--------|----------|
| **Source** | docs/active/warmstart-era-schedule-venue-slice.md (live C2-PREVIEW forge SlotBeforeSystemStart finding: warm-start hardcoded the preprod epoch length 432000, breaking preview replay) |
| **Requirement** | docs/active/warmstart-era-schedule-venue-slice.md (live C2-PREVIEW forge SlotBeforeSystemStart finding: warm-start hardcoded the preprod epoch length 432000, breaking preview replay) *(paraphrased from source/code_locus; registry `statement` empty)* |
| **Code** | crates/ade_ledger/src/seed_consensus_inputs.rs (SeedEpochConsensusInputs.epoch_start_slot + epoch_length_slots; SEED_CINPUT_SCHEMA_VERSION=4 since ECA-2-pre; FIELDS_OUTER=11; decode rejects v1/v2/v3 fail-closed UnknownVersion, the bootstrap authority surfaces a pre-v4 mismatch as the typed ConsensusInputsSchemaUnsupported per DC-CINPUT-06); crates/ade_runtime/src/seed_consensus_merge.rs (merge_seed_epoch_consensus_inputs persists the geometry from canonical.epoch_length_slots(); InvalidEpochWindow fail-closed on a degenerate window); crates/ade_runtime/src/consensus_inputs/canonical.rs (LiveConsensusInputsCanonical::epoch_length_slots() -> Option<u32>, end-start+1 fail-closed);… |
| **Tests** | `warm_start_schedule_locates_block_by_venue_geometry_not_hardcoded_432000`; `merge_persists_venue_epoch_geometry_preview_and_preprod`; `restart_genesis_epoch_length_mismatch_fails_closed`; `seed_epoch_consensus_inputs_round_trips_byte_identical`; `seed_cinput_decode_rejects_unknown_version` |
| **CI** | _(no CI listed in registry — enforced via Code/Tests; gap: bind a CI gate)_ |

#### `DC-CINPUT-06` — _enforced_

| Aspect | Location |
|--------|----------|
| **Source** | docs/clusters/EPOCH-CONSENSUS-VIEW/SLICE-ECA-2-pre-seed-sidecar-v4.md; user directive 2026-06-21 (the consensus profile is a single recovered authority surface; persist genesis_hash + protocol_params_hash in the v4 sidecar; no separate EVIEW manifest authority, no runtime fallback, no recomputation; typed upgrade error not corruption) |
| **Requirement** | user directive 2026-06-21 (the consensus profile is a single recovered authority surface; persist genesis_hash + protocol_params_hash in the v4 sidecar; no separate EVIEW manifest authority, no runtime fallback, no recomputation; typed… *(paraphrased from source/code_locus; registry `statement` empty)* |
| **Code** | crates/ade_ledger/src/seed_consensus_inputs.rs (SeedEpochConsensusInputs.genesis_hash + protocol_params_hash; SEED_CINPUT_SCHEMA_VERSION=4; FIELDS_OUTER=11; encode/decode the two bytes(32) after epoch_nonce; decode returns UnknownVersion for version != 4); crates/ade_runtime/src/seed_consensus_merge.rs (merge_seed_epoch_consensus_inputs copies canonical.genesis_hash + canonical.protocol_params_hash); crates/ade_runtime/src/bootstrap.rs (BootstrapError::ConsensusInputsSchemaUnsupported{found_version, required_version}; restore_seed_epoch_consensus_inputs maps a decode UnknownVersion -> it, distinct from SeedConsensusSidecarDecode; node.rs exit_code groups it with the… |
| **Tests** | `seed_epoch_consensus_inputs_round_trips_byte_identical`; `seed_cinput_canonical_bytes_cover_the_consensus_profile_hashes`; `seed_cinput_decode_rejects_unknown_version`; `merge_persists_consensus_profile_hashes`; `warm_start_pre_v4_sidecar_is_typed_schema_upgrade_not_corruption` |
| **CI** | `ci/ci_check_eview_seed_sidecar_v4.sh` |

### DC-LIVEMEM

#### `DC-LIVEMEM-01` — _enforced_

| Aspect | Location |
|--------|----------|
| **Source** | docs/planning/phase4-n-f-g-e-invariants.md |
| **Requirement** | docs/planning/phase4-n-f-g-e-invariants.md *(paraphrased from source/code_locus; registry `statement` empty)* |
| **Code** | crates/ade_network/src/session/core.rs (GREEN 16 MiB reassembly-tail cap MAX_REASSEMBLY_TAIL_BYTES + additive SessionError::ReassemblyBufferOverflow); crates/ade_node/src/node_sync.rs (RED 256-block WirePump lookahead-depth cap MAX_WIRE_PUMP_LOOKAHEAD); crates/ade_runtime/src/network/mux_pump.rs (the overflow variant maps to PeerHaltReason::ChainSyncDecodeError — drop the peer) |
| **Tests** | `session_reassembly_tail_over_cap_fails_closed`; `session_reassembly_tail_under_cap_still_drains_complete_item`; `wirepump_lookahead_stops_at_cap`; `wirepump_lookahead_cap_preserves_relay_behavior_under_normal_feed` |
| **CI** | `ci/ci_check_live_feed_memory_bounds.sh` |

### DC-SERVEMEM

#### `DC-SERVEMEM-01` — _enforced_

| Aspect | Location |
|--------|----------|
| **Source** | docs/clusters/PHASE4-N-AA/cluster.md; PHASE4-N-U cross-slice security review (MEDIUM finding) |
| **Requirement** | PHASE4-N-U cross-slice security review (MEDIUM finding) *(paraphrased from source/code_locus; registry `statement` empty)* |
| **Code** | crates/ade_runtime/src/chaindb/mod.rs (ChainDb trait range_bytes_capped + last_block_bytes -- S1); crates/ade_runtime/src/chaindb/types.rs (CappedSlotRange -- S1); crates/ade_runtime/src/chaindb/persistent.rs + in_memory.rs (impls, inverted-range guarded -- S1); crates/ade_runtime/src/network/served_chain_projection.rs (ChainDbServedSource range_bytes/next_after/tip use the bounded primitives + the MAX_SERVE_RANGE_BLOCKS cap + ServeRangeOutcome + derive the hash via decode_block, fail-closed over cap -- S2) |
| **Tests** | `range_bytes_capped_returns_at_most_max`; `range_bytes_capped_within_cap_not_truncated`; `range_bytes_capped_respects_bounds`; `range_bytes_capped_bytes_byte_identical`; `range_bytes_capped_inverted_range_is_empty`; `last_block_bytes_returns_highest_slot`; `serve_range_over_cap_fails_closed`; `serve_range_empty_window_is_empty_not_capexceeded`; `serve_range_undecodable_in_range_fails_closed`; `serve_range_inverted_range_fails_closed`; … (+2 more) |
| **CI** | `ci/ci_check_serve_range_bounded.sh` |

### DC-FOLLOW

#### `DC-FOLLOW-FORGE-01` — _enforced_

| Aspect | Location |
|--------|----------|
| **Source** | docs/clusters/PRODUCER-PARTICIPANT-FOLLOW/CN-FOLLOW-01-participant-forge-on-ao-selected-head.md (§6 DC-FOLLOW-FORGE-01, §8 changes, §10 MAC) |
| **Requirement** | docs/clusters/PRODUCER-PARTICIPANT-FOLLOW/CN-FOLLOW-01-participant-forge-on-ao-selected-head.md (§6 DC-FOLLOW-FORGE-01, §8 changes, §10 MAC) *(paraphrased from source/code_locus; registry `statement` empty)* |
| **Code** | crates/ade_node/src/node_sync.rs: ParticipantForgeDecision{UseInitialCatchupGate\|ExtendOnSelectedHead{forge_base}\|Refuse(ForgeRefused)} + ParticipantForgeFenceReason{VenueNotDeclaredParticipant\|ForkChoicePending\|NoDurableServableTip} (DurableTipDivergedFromExtendHead REMOVED -- the decision no longer gates on the latch) + ForgeRefused::ParticipantFenceViolation + ForgeRefused::ParticipantForgeBaseChangedBeforeSign{decision_base,sign_time_tip} + ForgeMode::ParticipantExtendOnSelectedHead{adopted_root,current_tip} (both DERIVED OBSERVATIONS, never the forge authority) + participant_forge_decision (derives forge_base from durable_servable_tip, not current_tip) +… |
| **Tests** | `participant_venue_forges_on_ao_selected_head_when_leader`; `participant_forge_base_is_ao_selected_chaindb_tip`; `participant_forge_base_is_servable_before_forge`; `participant_forge_refused_while_fork_choice_pending`; `participant_venue_requires_forge_activation`; `orphaned_startup_holds_forge_fence_participant`; `participant_forge_two_runs_byte_identical`; `single_producer_forge_decision_unchanged`; `keyed_participant_extend_survives_peer_admit_and_reaches_leader_check`; `participant_forge_refuses_if_tip_changes_between_decision_and_sign` |
| **CI** | `ci/ci_check_participant_forge_on_selected_head.sh` |

### DC-EVIEW

#### `DC-EVIEW-01` — _enforced_

| Aspect | Location |
|--------|----------|
| **Source** | docs/clusters/EPOCH-CONSENSUS-VIEW/SLICE-1-redb-materialization-gate.md; docs/clusters/EPOCH-CONSENSUS-VIEW/EPOCH-CONSENSUS-VIEW-design-analysis.md (Deliverable 6, the gate) |
| **Requirement** | docs/clusters/EPOCH-CONSENSUS-VIEW/EPOCH-CONSENSUS-VIEW-design-analysis.md (Deliverable 6, the gate) *(paraphrased from source/code_locus; registry `statement` empty)* |
| **Code** | crates/ade_runtime/src/chaindb/transient_epoch_view.rs: TRANSIENT_SUBTREE (the fixed owned subtree) + transient_root / transient_root_for_test (D1) + window_key (D2, blake2b over the bound-activation bindings, length-prefixed) + is_valid_window_key (D2 validator) + purge_transient_root (D3 fail-closed: enumerate -> validate -> delete -> fsync_dir -> empty-or-TransientViewError) + TransientEpochViewStore{open, materialize_batch, len, is_empty, iter_window, on_disk_bytes, dispose(self)} over the redb UtxoAnchor (default Immediate durability) + TransientViewError (structured terminal). crates/ade_runtime/src/bin/transient_view_kill_target.rs +… |
| **Tests** | `transient_root_is_owned_subtree_of_data_root`; `window_key_is_deterministic_and_binding_sensitive`; `window_key_validator_accepts_only_the_deterministic_form`; `purge_removes_valid_named_leftovers_and_leaves_subtree_empty`; `purge_fails_closed_on_a_foreign_artifact_and_deletes_nothing`; `purge_is_clean_on_an_empty_or_absent_subtree`; `lifecycle_open_materialize_iterate_dispose`; `dispose_is_clean_when_window_already_removed`; `transient_crash_mid_materialize_smoke`; `transient_crash_mid_dispose_smoke`; … (+5 more) |
| **CI** | `ci/ci_check_transient_view_memory_ceiling.sh`; `ci/ci_check_transient_view_no_fallback.sh`; `ci/ci_check_transient_view_not_live.sh` |

#### `DC-EVIEW-02` — _enforced_

| Aspect | Location |
|--------|----------|
| **Source** | docs/clusters/EPOCH-CONSENSUS-VIEW/SLICE-2-stake-reference-classification.md; docs/clusters/EPOCH-CONSENSUS-VIEW/EPOCH-CONSENSUS-VIEW-design-analysis.md (Deliverable 1/2, classification matrix) |
| **Requirement** | docs/clusters/EPOCH-CONSENSUS-VIEW/EPOCH-CONSENSUS-VIEW-design-analysis.md (Deliverable 1/2, classification matrix) *(paraphrased from source/code_locus; registry `statement` empty)* |
| **Code** | crates/ade_ledger/src/stake_ref.rs: StakeRefClass{Base(StakeCredential)\|Pointer(PointerRef)\|Null\|Reject(StakeRefReject)} + PointerRef{slot,tx_index,cert_index} (decoded, unresolved) + StakeRefReject{Empty\|UnknownAddressType\|MalformedBase\|MalformedPointer\|RewardAddressNotValidAsOutput} + classify_output_stake_ref(addr_bytes,era: CardanoEra) (routes through ade_codec::address::decode_address; era-gated pointer retirement at era>=CardanoEra::Conway; base validated to 57 bytes before the staking part is read; reward fail-closed) + decode_pointer_coords / decode_varint (exact-consumption base-128 varint, overflow-guarded). ci/ci_check_eview_stake_ref_classification.sh. |
| **Tests** | `base_type0_is_stake_key_hash`; `base_type1_is_stake_key_hash`; `base_type2_is_stake_script_hash`; `base_type3_is_stake_script_hash`; `pointer_is_decoded_pre_conway_and_retired_at_conway`; `pointer_multibyte_varint_pre_conway`; `pointer_result_exposes_no_credential`; `enterprise_and_byron_are_null_all_eras`; `reward_address_is_rejected_not_summed`; `empty_is_reject_not_null`; … (+8 more) |
| **CI** | `ci/ci_check_eview_stake_ref_classification.sh` |

#### `DC-EVIEW-03` — _enforced_

| Aspect | Location |
|--------|----------|
| **Source** | docs/clusters/EPOCH-CONSENSUS-VIEW/SLICE-3a-pointer-decode-resolution.md; docs/clusters/EPOCH-CONSENSUS-VIEW/SLICE-3-scope.md |
| **Requirement** | docs/clusters/EPOCH-CONSENSUS-VIEW/SLICE-3a-pointer-decode-resolution.md; docs/clusters/EPOCH-CONSENSUS-VIEW/SLICE-3-scope.md *(paraphrased from source/code_locus; registry `statement` empty)* |
| **Code** | crates/ade_codec/src/address/pointer.rs: Ptr{slot u32,tx_index u16,cert_index u16} + PointerDecodeError{NotAPointerAddress\|TooShort\|TruncatedVarint\|OverWidth\|TrailingBytes} + decode_pointer_address / decode_pointer_tail (era-gated) + decode_pointer_strict / decode_width_bounded (Conway width-bounded) + decode_pointer_normalized / decode_u64_wrapping / normalize_ptr (Babbage/<=Alonzo clamp-3-tuple). crates/ade_ledger/src/pointer_resolve.rs: PointerMap{insert(fail-closed on duplicate), resolve -> Option<StakeCredential>, len, is_empty}. ci/ci_check_eview_pointer_compat.sh. |
| **Tests** | `bounded_leading_zero_alias_accepted_all_eras`; `conway_decodes_in_range`; `conway_rejects_txix_over_u16`; `conway_rejects_width_overflow_within_max_groups`; `conway_rejects_slot_over_u32`; `conway_rejects_trailing_bytes`; `conway_accepts_max_width_boundary`; `babbage_normalizes_overflow_to_zero_tuple`; `babbage_in_range_kept_unmodified`; `babbage_rejects_trailing_bytes`; … (+10 more) |
| **CI** | `ci/ci_check_eview_pointer_compat.sh` |

#### `DC-EVIEW-04` — _enforced_

| Aspect | Location |
|--------|----------|
| **Source** | docs/clusters/EPOCH-CONSENSUS-VIEW/SLICE-3b-1-reduced-utxo-checkpoint.md; docs/clusters/EPOCH-CONSENSUS-VIEW/SLICE-3b-replay-window-materialization.md |
| **Requirement** | docs/clusters/EPOCH-CONSENSUS-VIEW/SLICE-3b-1-reduced-utxo-checkpoint.md; docs/clusters/EPOCH-CONSENSUS-VIEW/SLICE-3b-replay-window-materialization.md *(paraphrased from source/code_locus; registry `statement` empty)* |
| **Code** | crates/ade_ledger/src/reduced_utxo.rs: ReducedStakeRef{Base(StakeCredential)\|NonContributing} + encode/decode + reduce_txout (reuses classify_output_stake_ref(.., Conway)) + encode_reduced_record (canonical TxIn\|coin\|ref). crates/ade_runtime/src/chaindb/reduced_utxo_checkpoint.rs: ReducedUtxoCheckpoint{open, build_from (clears prior -> entries -> marker LAST), is_complete, fingerprint, len, get} over a durable redb store (REDUCED_TABLE + META_TABLE completeness marker = fp(32)\|\|count(8)); ReducedCheckpointError{Redb\|Incomplete\|Decode}. ci/ci_check_eview_reduced_utxo_checkpoint.sh. |
| **Tests** | `base_output_reduces_to_base_credential`; `enterprise_and_byron_are_non_contributing`; `pointer_output_is_non_contributing_at_conway`; `reduced_stake_ref_round_trips_canonically`; `decode_fails_closed_on_bad_tag_or_truncation`; `record_encoding_is_deterministic_and_canonical`; `build_then_query_and_complete`; `durable_across_reopen`; `replay_equivalent_two_builds_byte_identical`; `fingerprint_changes_with_content`; … (+2 more) |
| **CI** | `ci/ci_check_eview_reduced_utxo_checkpoint.sh` |

#### `DC-EVIEW-04b` — _enforced_

| Aspect | Location |
|--------|----------|
| **Source** | docs/clusters/EPOCH-CONSENSUS-VIEW/SLICE-3b-replay-window-materialization.md; docs/clusters/EPOCH-CONSENSUS-VIEW/SLICE-3b-1-reduced-utxo-checkpoint.md |
| **Requirement** | docs/clusters/EPOCH-CONSENSUS-VIEW/SLICE-3b-replay-window-materialization.md; docs/clusters/EPOCH-CONSENSUS-VIEW/SLICE-3b-1-reduced-utxo-checkpoint.md *(paraphrased from source/code_locus; registry `statement` empty)* |
| **Code** | crates/ade_ledger/src/reduced_advance.rs: ReducedBlockDelta{spent,produced} + reduced_block_delta (mirrors track_utxo, reuses extract_inputs_outputs_from_tx + reduce_txout + blake2b_256 tx_hash) + advance_cert_state (reuses crate::rules::process_block_certificates). crates/ade_runtime/src/chaindb/reduced_utxo_checkpoint.rs: ReducedUtxoCheckpoint::apply_block_delta (remove spent + insert produced, invalidates marker) + finalize (recompute marker). crates/ade_ledger/src/rules.rs: track_utxo / extract_inputs_outputs_from_tx / process_block_certificates made pub(crate) for the reuse. ci/ci_check_eview_windowed_advance.sh. |
| **Tests** | `reduced_delta_equals_reduce_of_track_utxo_on_real_conway_block`; `intra_block_chained_spend_cancels_phantom_matches_track_utxo`; `reduced_block_delta_is_deterministic`; `empty_block_yields_empty_delta`; `advance_cert_state_over_real_block_does_not_error`; `apply_block_delta_then_finalize`; `advance_over_real_conway_block_matches_build_from` |
| **CI** | `ci/ci_check_eview_windowed_advance.sh` |

#### `DC-EVIEW-05` — _enforced_

| Aspect | Location |
|--------|----------|
| **Source** | docs/clusters/EPOCH-CONSENSUS-VIEW/SLICE-3-scope.md (S3c); docs/clusters/EPOCH-CONSENSUS-VIEW/EPOCH-CONSENSUS-VIEW-design-analysis.md (Deliverable 3/7) |
| **Requirement** | docs/clusters/EPOCH-CONSENSUS-VIEW/EPOCH-CONSENSUS-VIEW-design-analysis.md (Deliverable 3/7) *(paraphrased from source/code_locus; registry `statement` empty)* |
| **Code** | crates/ade_ledger/src/reduced_aggregate.rs: StakeByPool{pool_stakes,total_active_stake} + AggregateError::StakeOverflow + aggregate_pool_stake(cred_utxo_stake, delegation) (iterates delegation.delegations, sums UTxO coin + reward per credential into its pool, checked_add fail-closed). crates/ade_runtime/src/chaindb/reduced_utxo_checkpoint.rs: ReducedUtxoCheckpoint::sum_base_credential_stake (folds only Base(cred) coins, fail-closed Overflow). ci/ci_check_eview_stake_aggregation.sh. |
| **Tests** | `sums_utxo_plus_reward_per_delegated_pool`; `reward_without_utxo_contributes`; `undelegated_credential_contributes_nothing`; `delegated_zero_stake_pool_is_included_with_zero`; `multiple_pools_aggregate_independently`; `overflow_is_fail_closed`; `aggregation_is_deterministic`; `sum_base_credential_stake_skips_non_contributing` |
| **CI** | `ci/ci_check_eview_stake_aggregation.sh`; `ci/ci_check_eview_leadership_complete.sh` |

#### `DC-EVIEW-06` — _enforced_

| Aspect | Location |
|--------|----------|
| **Source** | docs/clusters/EPOCH-CONSENSUS-VIEW/SLICE-3-scope.md (S3d); docs/clusters/EPOCH-CONSENSUS-VIEW/EPOCH-CONSENSUS-VIEW-design-analysis.md (Deliverable 4) |
| **Requirement** | docs/clusters/EPOCH-CONSENSUS-VIEW/EPOCH-CONSENSUS-VIEW-design-analysis.md (Deliverable 4) *(paraphrased from source/code_locus; registry `statement` empty)* |
| **Code** | crates/ade_ledger/src/reduced_snapshot.rs: SnapshotPhase{Mark\|Set\|Go} + LEADERSHIP_SNAPSHOT_PHASE (Set) + form_mark_snapshot (StakeByPool -> StakeSnapshot.pool_stakes) + is_boundary_stable(boundary_block_no, tip_block_no, k: SecurityParam) = (tip - boundary) > k (saturating). Reuses crate::epoch::rotate_snapshots + StakeSnapshot. ci/ci_check_eview_stability_gate.sh. |
| **Tests** | `forms_mark_snapshot_from_aggregate`; `stability_gate_requires_more_than_k_deep`; `boundary_ahead_of_tip_is_not_stable`; `leadership_reads_the_set_snapshot`; `formed_mark_rotates_into_set` |
| **CI** | `ci/ci_check_eview_stability_gate.sh` |

#### `DC-EVIEW-07` — _enforced_

| Aspect | Location |
|--------|----------|
| **Source** | docs/clusters/EPOCH-CONSENSUS-VIEW/SLICE-3-scope.md (S3e); docs/clusters/EPOCH-CONSENSUS-VIEW/EPOCH-CONSENSUS-VIEW-design-analysis.md (Deliverable 5; the bound-activation prohibition) |
| **Requirement** | docs/clusters/EPOCH-CONSENSUS-VIEW/EPOCH-CONSENSUS-VIEW-design-analysis.md (Deliverable 5; the bound-activation prohibition) *(paraphrased from source/code_locus; registry `statement` empty)* |
| **Code** | crates/ade_ledger/src/reduced_epoch_view.rs: EpochConsensusView{network_magic,era,epoch,source_point,checkpoint_commitment,nonce,snapshot_phase,stake_by_pool,total_active_stake,canonical_hash} + ViewBindings + bind (computes canonical_hash = blake2b(canonical_bytes(..))) + canonical_bytes (round-trippable) + verify_canonical_hash + matches (requires all bindings + verify). ci/ci_check_eview_view_binding.sh. |
| **Tests** | `bind_is_deterministic_and_self_verifies`; `matches_exact_bindings_and_rejects_mismatch`; `canonical_hash_is_binding_sensitive`; `canonical_bytes_reproduce_the_hash`; `tampered_view_fails_verification`; `leadership_complete_required_for_matches` |
| **CI** | `ci/ci_check_eview_view_binding.sh`; `ci/ci_check_eview_leadership_complete.sh` |

#### `DC-EVIEW-08` — _declared_

| Aspect | Location |
|--------|----------|
| **Source** | docs/clusters/EPOCH-CONSENSUS-VIEW/SLICE-3f-activation.md; docs/clusters/EPOCH-CONSENSUS-VIEW/SLICE-3-scope.md (the activation slice) |
| **Requirement** | docs/clusters/EPOCH-CONSENSUS-VIEW/SLICE-3-scope.md (the activation slice) *(paraphrased from source/code_locus; registry `statement` empty)* |
| **Code** | crates/ade_ledger/src/rules.rs: apply_epoch_boundary_with_registrations gains precomputed_mark: Option<&StakeByPool> -> new_mark = form_mark_snapshot(agg) when Some, the existing stub when None; apply_epoch_boundary_full passes None (the live path, UNCHANGED). ci/ci_check_eview_activation.sh. |
| **Tests** | `epoch_boundary_consumes_precomputed_aggregate_mark` |
| **CI** | `ci/ci_check_eview_activation.sh` |

#### `DC-EVIEW-09` — _enforced_

| Aspect | Location |
|--------|----------|
| **Source** | docs/clusters/EPOCH-CONSENSUS-VIEW/SLICE-3f-activation.md; user directive 2026-06-21 (separate manifest-bound authority surfaces) |
| **Requirement** | user directive 2026-06-21 (separate manifest-bound authority surfaces) *(paraphrased from source/code_locus; registry `statement` empty)* |
| **Code** | crates/ade_ledger/src/bootstrap_manifest.rs: BootstrapManifest{network_magic,era,source_point,seed_hash,cert_state_hash,source_commitment} + canonical encode/decode + BootstrapManifestError{MalformedManifest\|SeedHashMismatch\|CertStateHashMismatch\|NetworkMismatch\|EraMismatch\|CertStateDecode} + verify_and_import_cert_state (reuses crate::snapshot::cert_state::decode_cert_state VERBATIM). crates/ade_node/src/admission/seed_to_snapshot.rs: build_seed_ledger / seed_to_snapshot take cert_state -> ledger.cert_state. crates/ade_node/src/admission/bootstrap.rs: import_bootstrap_cert_state (convention-discovered, fail-closed) + AdmissionBootstrapError::BootstrapCertState; populates the snapshot +… |
| **Tests** | `manifest_round_trips_canonically`; `verify_and_import_happy_path`; `seed_hash_mismatch_fails_closed`; `cert_state_hash_mismatch_fails_closed`; `network_and_era_mismatch_fail_closed`; `malformed_manifest_fails_closed`; `malformed_cert_state_fails_closed_after_hash_ok` |
| **CI** | `ci/ci_check_eview_bootstrap_cert_state.sh` |

#### `DC-EVIEW-10` — _enforced_

| Aspect | Location |
|--------|----------|
| **Source** | docs/clusters/EPOCH-CONSENSUS-VIEW/SLICE-3f-activation.md (S3f-2); docs/clusters/EPOCH-CONSENSUS-VIEW/SLICE-3b-replay-window-materialization.md |
| **Requirement** | docs/clusters/EPOCH-CONSENSUS-VIEW/SLICE-3f-activation.md (S3f-2); docs/clusters/EPOCH-CONSENSUS-VIEW/SLICE-3b-replay-window-materialization.md *(paraphrased from source/code_locus; registry `statement` empty)* |
| **Code** | crates/ade_runtime/src/chaindb/reduced_window_driver.rs: drive_window_aggregate(checkpoint, bootstrap_state, blocks, era) -> loop { reduced_block_delta -> apply_block_delta; advance_cert_state -> state.cert_state/gov_state } then sum_base_credential_stake -> aggregate_pool_stake; starts from bootstrap_state.clone() (NOT LedgerState::new()); WindowDriverError fail-closed. Exported from chaindb/mod.rs. ci/ci_check_eview_window_driver.sh. |
| **Tests** | `empty_window_aggregates_bootstrap_state`; `real_conway_block_drive_equals_composed_pieces` |
| **CI** | `ci/ci_check_eview_window_driver.sh` |

#### `DC-EVIEW-11` — _enforced_

| Aspect | Location |
|--------|----------|
| **Source** | docs/clusters/EPOCH-CONSENSUS-VIEW/SLICE-3f-activation.md (S3f-3); user directive 2026-06-21 (narrow fail-closed rebind safety slice) |
| **Requirement** | user directive 2026-06-21 (narrow fail-closed rebind safety slice) *(paraphrased from source/code_locus; registry `statement` empty)* |
| **Code** | crates/ade_node/src/epoch_rebind.rs: decide_epoch_rebind(admission, bound_n1: Option<(&EpochConsensusView, &ViewBindings)>) -> EpochRebindDecision{KeepCurrent\|Promote\|FailClosed(EpochRebindReject)}; immediate-next-only (seed_epoch.0.wrapping_add(1)) + bindings.epoch==e + view.matches(bindings). crates/ade_node/src/node_sync.rs: the node-forge DC-EPOCH-03 wall calls decide_epoch_rebind(admission, None) -- FailClosed -> ForgeNotLeader (byte-identical), KeepCurrent -> proceed, Promote -> empty no-op (S3f-4). ci/ci_check_eview_epoch_rebind.sh. |
| **Tests** | `simulated_transition_promotes_bound_n1_view`; `same_epoch_keeps_current`; `off_epoch_without_bound_view_fails_closed`; `not_immediate_next_fails_closed`; `unlocatable_fails_closed`; `rejects_each_wrong_binding`; `rejects_tampered_view_wrong_hash`; `replay_equivalent_deterministic`; `crash_restart_redrives_same_decision_both_sides` |
| **CI** | `ci/ci_check_eview_epoch_rebind.sh` |

#### `DC-EVIEW-12` — _enforced_

| Aspect | Location |
|--------|----------|
| **Source** | docs/clusters/EPOCH-CONSENSUS-VIEW/SLICE-ECA-0b-leadership-complete-view.md; user directive 2026-06-21 (freeze the effective VRF + a full consensus-profile commitment; PoolDistrView derives exclusively from the sealed view; cardano numDelegators>0 pool inclusion) |
| **Requirement** | user directive 2026-06-21 (freeze the effective VRF + a full consensus-profile commitment; PoolDistrView derives exclusively from the sealed view; cardano numDelegators>0 pool inclusion) *(paraphrased from source/code_locus; registry `statement` empty)* |
| **Code** | crates/ade_ledger/src/reduced_epoch_view.rs: EpochConsensusView gains pool_vrf_keyhashes: BTreeMap<PoolId,Hash32> + protocol_params_commitment: Hash32 (both in canonical_bytes/canonical_hash + ViewBindings); is_leadership_complete (equal key sets); consensus_profile_commitment(genesis_hash, protocol_params_hash, asc) = blake2b(genesis.0 ++ protocol_params_hash.0 ++ asc.numer ++ asc.denom); matches requires is_leadership_complete() + protocol_params_commitment ==. crates/ade_node/src/epoch_candidate.rs: CandidateProfile{slots_per_epoch,genesis_hash,protocol_params_hash,asc}; derive_candidate uses drive_window_consensus_inputs + the delegated INTERSECT registered intersection… |
| **Tests** | `leadership_complete_required_for_matches`; `canonical_hash_is_binding_sensitive`; `derive_candidate_binds_target_epoch_and_round_trips_through_recovery`; `derive_candidate_canonical_hash_is_replay_equivalent` |
| **CI** | `ci/ci_check_eview_leadership_complete.sh` |

#### `DC-EVIEW-13` — _enforced_

| Aspect | Location |
|--------|----------|
| **Source** | docs/clusters/EPOCH-CONSENSUS-VIEW/SLICE-ECA-0a-pool-lifecycle-fidelity.md; user directive 2026-06-21 (correctness-first, no narrow shortcut; delegation-clearing NOT deferred) |
| **Requirement** | user directive 2026-06-21 (correctness-first, no narrow shortcut; delegation-clearing NOT deferred) *(paraphrased from source/code_locus; registry `statement` empty)* |
| **Code** | crates/ade_ledger/src/delegation.rs: PoolState.future_pools; apply_pool_registration (existing pool -> future_pools + retiring.remove; new -> pools); apply_pool_reap(cert: &mut CertState, entered_epoch) (adopt-drop-orphan; reap e.0==entered_epoch.0; delegations.retain drops reaped-pool targets; remove from pools+retiring). crates/ade_runtime/src/chaindb/reduced_window_driver.rs: drive_window_consensus_inputs(.., slots_per_epoch) -> WindowConsensusInputs{stake, pool_params}, applies apply_pool_reap at crossed boundaries, mark pre-reap; drive_window_aggregate = per-block wrapper (slots_per_epoch=u64::MAX). crates/ade_ledger/src/snapshot/cert_state.rs: 6-field codec round-trips… |
| **Tests** | `re_registration_keeps_old_vrf_until_reap`; `pool_re_registration_stages_params_adopted_at_reap`; `reaped_pool_delegation_cleared_no_silent_reattach_on_reregistration`; `pool_reap_reaps_matching_epoch_only`; `drive_boundary_adopts_futures_reaps_retiring_clears_delegations`; `drive_boundary_is_deterministic`; `cert_state_round_trip_populated` |
| **CI** | `ci/ci_check_eview_pool_lifecycle.sh` |

---

## OP — Operational Invariants (Project Constitution §4b)

_10 rules._

### OP-NET

#### `OP-NET-01` — _declared_

| Aspect | Location |
|--------|----------|
| **Source** | Project constitution §4b |
| **Requirement** | Block producer connects only through trusted relay topology; no direct public peer connectivity |
| **Code** | _(no enforcement found — gap)_ |
| **Tests** | _(no tests listed — gap)_ |
| **CI** | _(no CI gate — gap)_ |

#### `OP-NET-02` — _declared_

| Aspect | Location |
|--------|----------|
| **Source** | Project constitution §4b |
| **Requirement** | Relay paths geographically and topologically diverse; isolating one path does not prevent timely propagation |
| **Code** | _(no enforcement found — gap)_ |
| **Tests** | _(no tests listed — gap)_ |
| **CI** | _(no CI gate — gap)_ |

#### `OP-NET-03` — _declared_

| Aspect | Location |
|--------|----------|
| **Source** | Project constitution §4b |
| **Requirement** | No single peer, ASN, region, or operator cluster dominates the node's authoritative view |
| **Code** | _(no enforcement found — gap)_ |
| **Tests** | _(no tests listed — gap)_ |
| **CI** | _(no CI gate — gap)_ |

### OP-MEM

#### `OP-MEM-01` — _partial_

| Aspect | Location |
|--------|----------|
| **Source** | Project constitution §4b |
| **Requirement** | Mempool pressure and peer churn must not starve block validation, chain selection, or persistence (scheduling priority) |
| **Code** | crates/ade_node/src/admission/runner.rs; crates/ade_node/src/mem_measure/rss_sampler.rs; crates/ade_node/src/convergence_evidence.rs |
| **Tests** | _(no tests listed — gap)_ |
| **CI** | `ci/ci_check_mem_measure_evidence.sh` |

#### `OP-MEM-02` — _enforced_

| Aspect | Location |
|--------|----------|
| **Source** | Project constitution §4b; bounty BA-08; MEM-OPT cluster plan (docs/planning/mem-opt-cluster-plan.md) |
| **Requirement** | Ade's owned resident memory (Private_Dirty/RssAnon) under a representative venue stays clearly below the reference Haskell cardano-node's on the same chain, WITHOUT changing ledger semantics, chain selection, persisted bytes, or replay-equivalence. |
| **Code** | crates/ade_node/src/mem_measure/rss_sampler.rs; crates/ade_ledger/src/fingerprint.rs; crates/ade_node/src/admission/bootstrap.rs; ci/ci_check_mem_opt_s3_owned.sh; ci/ci_check_utxo_fp_cache.sh |
| **Tests** | `owned_samplers_present_on_linux`; `static_utxo_fp_fails_closed_under_track_utxo_true_and_version_mismatch` |
| **CI** | `ci/ci_check_mem_opt_s3_owned.sh; ci/ci_check_utxo_fp_cache.sh` |

### OP-OPS

#### `OP-OPS-01` — _declared_

| Aspect | Location |
|--------|----------|
| **Source** | Project constitution §4b |
| **Requirement** | Post-incident reconciliation derived solely from recovered canonical chain |
| **Code** | _(no enforcement found — gap)_ |
| **Tests** | _(no tests listed — gap)_ |
| **CI** | _(no CI gate — gap)_ |

#### `OP-OPS-02` — _declared_

| Aspect | Location |
|--------|----------|
| **Source** | Project constitution §4b |
| **Requirement** | Emergency recovery procedures have explicit admissibility criteria, deterministic inputs/outputs, and authority thresholds |
| **Code** | _(no enforcement found — gap)_ |
| **Tests** | _(no tests listed — gap)_ |
| **CI** | _(no CI gate — gap)_ |

#### `OP-OPS-03` — _declared_

| Aspect | Location |
|--------|----------|
| **Source** | Project constitution §4b |
| **Requirement** | Incident evidence sufficient to reconstruct canonical decision path without relying on nondeterministic logs |
| **Code** | _(no enforcement found — gap)_ |
| **Tests** | _(no tests listed — gap)_ |
| **CI** | _(no CI gate — gap)_ |

#### `OP-OPS-04` — _enforced_

| Aspect | Location |
|--------|----------|
| **Source** | docs/planning/phase4-n-c-invariants.md §7 (OQ1); docs/active/op-ops-04-ade-native-kes-flow.md; docs/planning/phase4-n-p-invariants.md |
| **Requirement** | Operator-supplied keys. Ade supports both KES key flows: (a) Ade-native `ade_node --mode key_gen_kes --out-file PATH` emitting an `ade.kes.seed.v1` envelope loaded via `ade_runtime::producer::keys::load_ade_kes_signing_key`; (b) cardano-cli `node key-gen-KES` emitting a 608-byte `KesSigningKey_ed25519_kes_2^6` envelope loaded via `ade_runtime::producer::keys::load_kes_signing_key_skey`. After PHASE4-N-P S5 the cardano-cli path routes through the Ade-owned BLUE deserializer (`ade_crypto::kes_sum::Sum6Kes::raw_deserialize_signing_key_kes`); both flows produce a `KesSecret` whose inner is `ade_crypto::kes_sum::Sum6Kes::SigningKey` (BLUE-owned algorithm). VRF and cold (Ed25519) keys continue to be operator-supplied via cardano-cli text-envelope `.skey` files. Private-key material never crosses into BLUE — the entire RED custody surface lives under `crates/ade_runtime/src/producer/{keys,signing,ade_kes_envelope}.rs`. Wrong-size payloads fail-close via `KeyLoadError::UnsupportedExpandedKesKeyFormat`; structurally-invalid 608-byte payloads fail-close via `KeyLoadError::KesParse(KesParseError::*)`. Mechanical enforcement: ci/ci_check_private_key_custody.sh + ci/ci_check_kes_envelope_closed.sh + ci/ci_check_kes_sum_compatibility.sh. OP-OPS-04-KES-PERIOD-ANCHOR (public-venue hardening): at producer-shell init the opcert.kes_period is the ABSOLUTE KES period the key's evolution-0 is certified for; the shell verifies the INJECTED current absolute period (derived from the genesis KES anchor + the durable tip slot, NEVER the raw key evolution index) is within [opcert_start, opcert_start+63], then anchors evolution-0 at opcert_start and evolves the key by (current - opcert_start) so it signs at the current period -- fail-closed KesPeriodBelowOpCertStart / KesPeriodPastOpCertEnd / KesEvolutionFailed outside the window. Signing stays RED/shell; opcert verification stays BLUE/core; no wall-clock in the deterministic shell (the period is injected). |
| **Code** | crates/ade_runtime/src/producer/keys.rs (load_*_signing_key_skey, load_ade_kes_signing_key, write_ade_kes_envelope, KeyLoadError); crates/ade_runtime/src/producer/ade_kes_envelope.rs (closed envelope grammar); crates/ade_runtime/src/producer/signing.rs (RED-confined custody, KesSecret with BLUE-owned inner); crates/ade_crypto/src/kes_sum/ (BLUE Sum6KES algorithm + serde + ground-truth corpus); crates/ade_node/src/key_gen.rs (one-shot key-gen-KES surface); crates/ade_runtime/src/producer/producer_shell.rs (ProducerShell::init: opcert-window check on the INJECTED current absolute period + anchor evolution-0 at opcert_start + kes_update to current-opcert_start; KesEvolutionFailed);… |
| **Tests** | `ade_envelope_round_trips_through_loader_at_period_0`; `ade_envelope_loader_returns_kes_at_loaded_period`; `ade_envelope_loader_rejects_signing_at_past_period`; `cardano_cli_kes_envelope_rejects_32_byte_payload`; `cardano_cli_kes_envelope_rejects_synthetic_608_byte_payload`; `cardano_cli_kes_envelope_accepts_real_608_byte_payload`; `cardano_cli_kes_envelope_rejects_612_byte_payload`; `cardano_cli_kes_envelope_rejects_608_byte_leaf_zero_payload`; `ade_envelope_loader_returns_unknown_format`; `ade_envelope_loader_returns_wrong_role`; … (+18 more) |
| **CI** | `ci/ci_check_private_key_custody.sh`; `ci/ci_check_kes_envelope_closed.sh`; `ci/ci_check_kes_sum_compatibility.sh` |

#### `OP-OPS-05` — _enforced_

| Aspect | Location |
|--------|----------|
| **Source** | docs/planning/phase4-n-c-invariants.md §7 (OQ9) |
| **Requirement** | Slot-deadline forging SLA. Forge + self-accept + N2N hand-off must complete within the slot's deadline (1s on mainnet, smaller on testnets). Operational, not constitutional: missing the deadline costs a slot but does not violate a hash-critical invariant. |
| **Code** | crates/ade_runtime/src/producer/scheduler.rs (scheduler_step + the full pipeline timing); crates/ade_runtime/tests/producer_pipeline_slot_deadline.rs (wall-clock measurement) |
| **Tests** | `producer_full_path_under_slot_deadline_on_reference_fixture` |
| **CI** | `ci/ci_check_scheduler_closure.sh` |

---

## RO — Release Obligations (Project Constitution §4a)

_16 rules._

### RO-TEST

#### `RO-TEST-01` — _declared_

| Aspect | Location |
|--------|----------|
| **Source** | Project constitution §4a, T-DET-01, DC-LEDGER-07 |
| **Requirement** | Consensus-relevant inputs fuzzed differentially across all supported versions; any verdict mismatch is release-blocking |
| **Code** | _(no enforcement found — gap)_ |
| **Tests** | _(no tests listed — gap)_ |
| **CI** | _(no CI gate — gap)_ |

#### `RO-TEST-02` — _declared_

| Aspect | Location |
|--------|----------|
| **Source** | Project constitution §4a, T-CI-01 |
| **Requirement** | Every fork/mismatch/parser disagreement that ever occurred becomes a permanent regression corpus entry |
| **Code** | _(no enforcement found — gap)_ |
| **Tests** | _(no tests listed — gap)_ |
| **CI** | _(no CI gate — gap)_ |

#### `RO-TEST-03` — _declared_

| Aspect | Location |
|--------|----------|
| **Source** | Project constitution §4a, T-DET-01, T-REC-01 |
| **Requirement** | Failed, duplicate, and boundary-case inputs remain verdict-stable under resubmission and replay |
| **Code** | _(no enforcement found — gap)_ |
| **Tests** | _(no tests listed — gap)_ |
| **CI** | _(no CI gate — gap)_ |

### RO-REL

#### `RO-REL-01` — _declared_

| Aspect | Location |
|--------|----------|
| **Source** | Project constitution §4a, DC-LEDGER-07, T-DET-01 |
| **Requirement** | Release not mainnet-eligible without mixed-version topology consensus equivalence on adversarial inputs |
| **Code** | _(no enforcement found — gap)_ |
| **Tests** | _(no tests listed — gap)_ |
| **CI** | _(no CI gate — gap)_ |

#### `RO-REL-02` — _declared_

| Aspect | Location |
|--------|----------|
| **Source** | Project constitution §4a, DC-LEDGER-03, T-DET-01 |
| **Requirement** | Cross-implementation accept/reject agreement on authoritative corpora is release-blocking |
| **Code** | _(no enforcement found — gap)_ |
| **Tests** | _(no tests listed — gap)_ |
| **CI** | _(no CI gate — gap)_ |

#### `RO-REL-03` — _declared_

| Aspect | Location |
|--------|----------|
| **Source** | Project constitution §4a |
| **Requirement** | No single implementation bug should exceed the protocol's intended safety or liveness fault threshold at ecosystem level |
| **Code** | _(no enforcement found — gap)_ |
| **Tests** | _(no tests listed — gap)_ |
| **CI** | _(no CI gate — gap)_ |

### RO-LIVE

#### `RO-LIVE-01` — _partial_

| Aspect | Location |
|--------|----------|
| **Source** | docs/planning/phase4-n-a-successor-invariants.md §8 |
| **Requirement** | A Haskell cardano-node peer issuing RequestRange covering an Ade-forged block receives, via the producer-side block-fetch server, bytes that pass that peer's full header+body validation. Captured as an operator-action log against a private Haskell peer; the underlying semantic invariants are DC-CONS-17, DC-CONS-18, CN-PROTO-06, DC-PROTO-07, DC-PROTO-08, and the existing self-accept + body-hash recipe (DC-CONS-16). |
| **Code** | crates/ade_runtime/tests/cross_impl_server_pipeline.rs (mechanical adapter — CE-N-G-7); crates/ade_core_interop/src/bin/live_block_fetch_session.rs (legacy operator-action binary — CE-N-G-8); crates/ade_runtime/src/network/n2n_listener.rs (PHASE4-N-Q RED listener + handshake gate); crates/ade_node/src/produce_mode.rs (RED produce-mode driver — run_real_forge live forge composition (N-R-A/N-S/N-W/N-X); per-peer block-fetch dispatch wired N-R-B/N-S-B); docs/clusters/completed/PHASE4-N-G/CE-N-G-8_PROCEDURE.md + docs/clusters/PHASE4-N-Q/CE-N-Q-OPERATOR_PROCEDURE.md (operator procedures) |
| **Tests** | `cross_impl_server_pipeline_request_range_returns_decodable_bytes`; `cross_impl_server_pipeline_request_range_byte_identical_to_self_accept_input`; `live_block_fetch_session_hermetic_default_prints_readiness`; `n2n_listener_loopback_handshake_succeeds`; `produce_mode_starts_runs_three_slots_and_exits_via_max_slots`; `live_wire_pump_feed_reaches_forge_tick`; `live_feed_forge_serve_loopback_returns_forged_block` |
| **CI** | `ci/ci_check_server_paths_corpus_present.sh` |

#### `RO-LIVE-02` — _partial_

| Aspect | Location |
|--------|----------|
| **Source** | docs/planning/receive-side-bridge-invariants.md §8 |
| **Requirement** | docs/planning/receive-side-bridge-invariants.md §8 *(paraphrased from source/code_locus; registry `statement` empty)* |
| **Code** | crates/ade_runtime/tests/receive_pipeline_corpus_drive.rs (mechanical adapter — CE-N-H-5); crates/ade_core_interop/src/bin/live_block_follow_session.rs (operator-action binary — CE-N-H-6); docs/clusters/completed/PHASE4-N-H/CE-N-H-6_PROCEDURE.md (operator procedure) |
| **Tests** | `receive_pipeline_corpus_drive_admits_every_block`; `receive_pipeline_corpus_drive_chaindb_tip_matches_expected`; `receive_pipeline_corpus_drive_admitted_bytes_equal_corpus_bytes`; `receive_pipeline_corpus_drive_ledger_fingerprint_changes_on_admit`; `live_block_follow_session_hermetic_default_prints_readiness` |
| **CI** | `ci/ci_check_receive_paths_corpus_present.sh` |

#### `RO-LIVE-03` — _declared_

| Aspect | Location |
|--------|----------|
| **Source** | docs/planning/phase4-n-l-wire-protocol-invariants.md §8 |
| **Requirement** | docs/planning/phase4-n-l-wire-protocol-invariants.md §8 *(paraphrased from source/code_locus; registry `statement` empty)* |
| **Code** | _(no enforcement found — gap)_ |
| **Tests** | _(no tests listed — gap)_ |
| **CI** | _(no CI gate — gap)_ |

#### `RO-LIVE-04` — _enforced_

| Aspect | Location |
|--------|----------|
| **Source** | docs/planning/phase4-n-l-live-wire-smoke-invariants.md §1 |
| **Requirement** | docs/planning/phase4-n-l-live-wire-smoke-invariants.md §1 *(paraphrased from source/code_locus; registry `statement` empty)* |
| **Code** | crates/ade_node/src/wire_only.rs, crates/ade_node/src/live_log/event.rs, crates/ade_node/src/live_log/writer.rs, crates/ade_node/src/main.rs |
| **Tests** | `main_wire_only_exits_zero_after_tip_read`; `main_wire_only_emits_peer_tip_read_with_responder_tip`; `main_wire_only_never_emits_agreement_verdict`; `main_without_genesis_does_not_attempt_admission`; `peer_dial_failure_exits_nonzero_with_error_event`; `admission_mode_fails_closed_with_ledger_seed_unavailable`; `jsonl_events_are_valid_one_object_per_line`; `live_log_writer_emits_one_object_per_line`; `live_log_writer_serializes_node_started_canonically`; `live_log_writer_two_runs_are_byte_identical`; … (+1 more) |
| **CI** | `ci/ci_check_wire_only_event_vocabulary_closed.sh`; `ci/ci_check_wire_only_no_bootstrap.sh` |

#### `RO-LIVE-05` — _enforced_

| Aspect | Location |
|--------|----------|
| **Source** | docs/planning/phase4-n-l-live-wire-smoke-invariants.md §1 |
| **Requirement** | docs/planning/phase4-n-l-live-wire-smoke-invariants.md §1 *(paraphrased from source/code_locus; registry `statement` empty)* |
| **Code** | crates/ade_node/src/admission/ (full admission stack), crates/ade_runtime/src/admission/wire_pump.rs (live wire pump), crates/ade_runtime/src/consensus_inputs/ (operator-supplied LiveConsensusInputs import authority + canonical fingerprint), crates/ade_runtime/src/seed_import/ (full preprod UTxO importer; PHASE4-N-M-A1.1 closure includes reference-script + Byron Base58 + Plutus-integer-tolerant field skipping), docs/evidence/phase4-n-m-* (live transcripts + bundles + runbook) |
| **Tests** | `live_operator_pass_against_docker_preprod`; `live_bundle_imports_with_conway_era_and_deterministic_fingerprint`; `cross_epoch_block_triggers_halt_without_admit`; `adversarial_corpus_rejects_all_four_mutation_classes` |
| **CI** | `ci/ci_check_live_operator_pass_scaffold.sh` |

#### `RO-LIVE-06` — _enforced_

| Aspect | Location |
|--------|----------|
| **Source** | docs/clusters/PHASE4-N-F-C/cluster.md; L6-peer-acceptance-evidence-manifest.md |
| **Requirement** | docs/clusters/PHASE4-N-F-C/cluster.md; L6-peer-acceptance-evidence-manifest.md *(paraphrased from source/code_locus; registry `statement` empty)* |
| **Code** | crates/ade_node/src/ba02_evidence.rs (Ba02Manifest, BA02Outcome, PeerAcceptEvent, NoEvidenceReason, parse_peer_accept_events, correlate); crates/ade_node/src/ba02_pass.rs (PHASE4-N-F-G-C: RED operator-pass evidence I/O — correlate_peer_log_file reads the operator-captured peer log into the GREEN correlate; write_ba02_manifest accepts ONLY a Ba02Manifest); ci/ci_check_ba02_evidence_closed.sh; ci/ci_check_ba02_evidence_manifest_schema.sh |
| **Tests** | `ba02_manifest_schema_round_trips`; `ba02_correlate_served_block_yields_manifest`; `ba02_correlate_chain_tip_only_yields_manifest`; `ba02_correlate_both_signals_agree_records_served_primary`; `ba02_correlate_served_block_without_slot_yields_manifest`; `ba02_correlate_conflicting_signals_is_no_evidence`; `ba02_correlate_wrong_hash_is_no_evidence`; `ba02_correlate_chain_point_mismatch_is_no_evidence`; `ba02_correlate_no_slot_wrong_hash_is_no_evidence`; `ba02_correlate_stale_log_is_no_evidence`; … (+10 more) |
| **CI** | `ci/ci_check_ba02_evidence_closed.sh`; `ci/ci_check_ba02_evidence_manifest_schema.sh` |

### RO-GENESIS

#### `RO-GENESIS-REPLAY-01` — _declared_

| Aspect | Location |
|--------|----------|
| **Source** | docs/planning/phase4-n-m-ledger-seed-invariants.md §0 honest-scope table |
| **Requirement** | docs/planning/phase4-n-m-ledger-seed-invariants.md §0 honest-scope table *(paraphrased from source/code_locus; registry `statement` empty)* |
| **Code** | _(no enforcement found — gap)_ |
| **Tests** | _(no tests listed — gap)_ |
| **CI** | `ci/ci_check_genesis_replay_open_obligation.sh` |

### RO-MITHRIL

#### `RO-MITHRIL-IMPORT-01` — _enforced_

| Aspect | Location |
|--------|----------|
| **Source** | docs/planning/phase4-n-m-ledger-seed-invariants.md §10 carry-forward; PHASE4-N-Y S1/S7 |
| **Requirement** | docs/planning/phase4-n-m-ledger-seed-invariants.md §10 carry-forward; PHASE4-N-Y S1/S7 *(paraphrased from source/code_locus; registry `statement` empty)* |
| **Code** | crates/ade_ledger/src/bootstrap_anchor/binding.rs (verify_mithril_binding + MithrilManifestReport); crates/ade_runtime/src/mithril_import/ (manifest importer); crates/ade_runtime/src/mithril_bootstrap.rs (PHASE4-N-Z production composition) |
| **Tests** | `mithril_binding_rejects_certified_point_other_than_seed_point`; `mithril_anchor_rejects_field_mismatch`; `mithril_import_fail_closed_blocks_storage_init`; `mithril_bootstrap_fails_closed_on_seed_point_mismatch` |
| **CI** | `ci/ci_check_mithril_uses_bootstrap_initial_state.sh`; `ci/ci_check_mithril_seed_point_independence.sh`; `ci/ci_check_mithril_documented_evidence.sh` |

### RO-CLOSE

#### `RO-CLOSE-01` — _enforced_

| Aspect | Location |
|--------|----------|
| **Source** | docs/clusters/completed/PHASE4-N-V/CLOSURE.md; user-direction (PHASE4-N-V close correction) |
| **Requirement** | Unmasked close-gate discipline. Any slice that changes canonical bytes, encoded forms, decoder inputs, or golden fixtures MUST run an UNMASKED full close gate (cargo test --workspace) and use cargo's REAL exit status / result line as the sole pass/fail authority for cluster closure. Piped output (\| tail, \| grep) may be used for display only, NEVER as the pass/fail authority — a pipeline's exit code is the last stage's, not cargo's. Before closure, ALL consumers of the changed canonical output (every golden fixture, decoder, re-encode, and byte-identity test, across all crates — not just the edited crate) MUST be grepped/audited. A cluster is not closed until the unmasked close gate exits successfully. |
| **Code** | process/release discipline — no BLUE code locus; binds /cluster-close and any slice touching canonical output |
| **Tests** | _(no tests listed in registry — enforced via Code/CI; gap: name the proving test)_ |
| **CI** | _(no CI listed in registry — enforced via Code/Tests; gap: bind a CI gate)_ |

### RO-SYNC

#### `RO-SYNC-EVIDENCE-01` — _partial_

| Aspect | Location |
|--------|----------|
| **Source** | docs/clusters/PHASE4-N-Y/S5-compatibility-evidence.md |
| **Requirement** | docs/clusters/PHASE4-N-Y/S5-compatibility-evidence.md *(paraphrased from source/code_locus; registry `statement` empty)* |
| **Code** | ci/ci_check_sync_evidence_manifest_schema.sh; corpus/sync/regressions/; crates/ade_testkit/src/harness/sync_diff.rs |
| **Tests** | `regression_fixture_per_mismatch` |
| **CI** | `ci/ci_check_sync_evidence_manifest_schema.sh` |

---
---

## Deprecated rules

None. The registry has **0** entries with `status = "deprecated"` at HEAD. (IDs are append-only and retained on deprecation; when the first rule is deprecated it will appear here with its ID preserved and clearly marked.)

---

## Enforcement gaps (need attention)

These are surfaced, not hidden — an enforcement gap is the unhidden state that lets the next slice close it. Grouped by kind.

### A. Principal live obligations — `declared` rules with a real outstanding live-flip proof

Activation is **mechanically automatic** (`DC-EPOCH-13` enforced — the `EVIEW_ACTIVATION_ARMED` semantic gate is gone), but two EVIEW rules stay `declared` pending a **committed live-flip transcript**. Their gates are present and run green over the hermetic substrate; the `declared` status is NOT vacuous — it is a real outstanding live obligation. The SELECT/flip evidence exists off-repo; the registry flip is operator-gated.

| Rule | Status | Gate (present + green hermetic) | Outstanding obligation |
|------|--------|---------------------------------|------------------------|
| `DC-EPOCH-11` | declared | `ci_check_eview_live_checkpoint.sh` | The live reduced-checkpoint **materialization on the admission path** is built/advanced and unit-proven (the `-mat-1` bootstrap build is enforced; `-mat-2a` per-block advance primitive done), but the end-to-end live materialization across a real epoch boundary awaits a committed transcript. |
| `DC-EVIEW-08` | declared | `ci_check_eview_activation.sh` (+ `ci_check_eview_view_binding.sh`) | The epoch-boundary **consumption of the precomputed aggregate mark** (`apply_epoch_boundary_with_registrations(precomputed_mark)`) is wired + hermetically tested, but the live path still passes `None`; the activation consumption point awaits the window driver over a real epoch + the live flip. |

Per HEAD_DELTAS §5 and the Anomalies block: record these as "gate present + green; rule `declared` pending the live flip" — the expected mid-cluster state, not an orphan defect. (Note the deliberate asymmetry: `DC-EPOCH-13` makes activation automatic + proves the no-semantic-gate property; it does not by itself capture a live epoch flip — that is what `DC-EPOCH-11` / `DC-EVIEW-08` still owe.)

### B. Stale registry test references (named test fn not found in source)

A registry `tests` entry names a bare test function that does not exist in the codebase at HEAD. The rule is otherwise enforced by its remaining tests; the stale name should be repaired (renamed or removed) in the registry. (Prose/path-style `tests` entries and inline-parenthetical descriptions were excluded — only bare-fn-name misses are listed.)

| Rule | Status | Missing test reference | Note |
|------|--------|------------------------|------|
| `DC-NODE-18` | enforced | `extend_own_spine_promotion_requires_adoption_certificate` | the adoption-certificate path was removed by `DC-NODE-21`; this test name is a leftover. The 5 other listed tests exist. |

### C. Enforced rules with an empty load-bearing cell

These rules are `status = "enforced"` but one cell is empty in the registry. None has an empty **Code** cell (every enforced rule names enforcing code). The gaps are in Tests / CI — the rule's enforced status rests on the populated cells; naming the missing proving test or binding a dedicated CI gate would close the gap. Many are enforced by a shared/cross-cutting gate (e.g. the BLUE forbidden-pattern / dependency-boundary scans) rather than a rule-specific script.

- **Enforced with no Code:** 0 — none.

- **Enforced with no Tests row** (9): `CN-OPERATOR-EVIDENCE-01`, `CN-STORE-08`, `DC-ADMIT-09`, `DC-CRYPTO-02`, `DC-OUTBOUND-FIFO-01`, `DC-WAL-01`, `RO-CLOSE-01`, `T-BOUND-02`, `T-BUILD-01`.

- **Enforced with no CI row** (23): `CN-CINPUT-01`, `CN-PEER-OUTBOUND-MAP-01`, `CN-PREIMAGE-FIXTURE-01`, `CN-PROD-04`, `CN-SNAPSHOT-02`, `DC-CINPUT-02a`, `DC-CINPUT-05`, `DC-FORGE-01`, `DC-GENESIS-01`, `DC-KES-HEADER-01`, `DC-NODE-31`, `DC-NODE-32`, `DC-NODE-33`, `DC-OPCERT-01`, `DC-OUTBOUND-FIFO-01`, `DC-PROD-01`, `DC-PROD-02`, `DC-PROD-03`, `DC-PROTO-10`, `DC-SNAPSHOT-01`, `DC-WAL-03`, `RO-CLOSE-01`, `T-REC-05`.

> No rule has Code, Tests, AND CI all empty (no critical triple-gap). The enforced-but-no-CI set is mostly rules enforced structurally (types/typestate) or via a shared scan; the enforced-but-no-Tests set is mostly structural or text-check rules.

### D. Source-only gaps

None — every rule carries a `source`.

---

## Cross-reference checks (vs CODEMAP / HEAD_DELTAS / registry)

These do not block doc generation; they keep the four grounding docs coherent.

### Registry ↔ TRACEABILITY

- All **418** active registry rule IDs appear in this doc exactly once; this doc introduces no rule absent from the registry. (1:1 by construction — the registry is the sole rule source.)
- Status tally matches the registry exactly: 291 enforced / 22 partial / 104 declared / 1 enforced_scaffolding (= 418).

### CI ↔ HEAD_DELTAS §5 / on-disk `ci/`

- **CI scripts referenced by a rule and present on disk:** 219 of 219 referenced. **Claimed-but-missing:** 0 — none.

- **Orphaned CI scripts** — `ci_check_*.sh` present on disk (238 total) but referenced by **no** registry rule (19). Classified:

  - **Cross-cutting / global gates (legitimately rule-less — they enforce workspace-wide discipline, not one registry rule):** `ci_check_module_headers.sh`, `ci_check_no_secrets.sh`, `ci_check_pallas_quarantine.sh`, `ci_check_registry_unique_ids.sh`.

  - **Known-owed enforcement debt** (the dormant MEM-OPT-UTXO-DISK B-infra + diagnostic gates `.idd-config.json` explicitly flags as not-yet-bound to a registry rule — they attach to `DC-MEM-05`/`DC-MEM-06` when the `track_utxo=true` / LIVE-LEDGER-APPLY band lands): `ci_check_alloc_determinism_neutral.sh`, `ci_check_live_blockfetch_byte_only.sh`, `ci_check_mem_compare_evidence.sh`, `ci_check_mem_diag_quarantine.sh`, `ci_check_mem_opt_s1_reduction.sh`, `ci_check_mem_opt_s3_owned.sh`, `ci_check_mem_opt_utxo_disk_s0.sh`, `ci_check_utxo_admission_seam.sh`, `ci_check_utxo_disk_anchor.sh`, `ci_check_utxo_disk_key.sh`, `ci_check_utxo_pre_resolve.sh`.

  - **Binding gaps** (the script header names a rule ID but that rule's registry `ci_scripts` array does not list the script — the rule IS enforced by its other bound gates; binding the script in the registry would close the gap): `ci_check_live_transcript_forge_base.sh`, `ci_check_plutus_oracle_no_false_accept.sh`, `ci_check_single_producer_loop_continuation.sh`, `ci_check_wire_rollback_signal_preserved.sh`. Specifically: `ci_check_alloc_determinism_neutral.sh`→`DC-MEM-06`, `ci_check_plutus_oracle_no_false_accept.sh`→`DC-LEDGER-03`, `ci_check_single_producer_loop_continuation.sh`→`DC-NODE-19`, `ci_check_live_transcript_forge_base.sh`→`DC-NODE-20`, `ci_check_wire_rollback_signal_preserved.sh`→a PHASE4-N-AI rule (none currently list it).

### Code ↔ CODEMAP

- CODEMAP (regenerated at the same HEAD `cdcd9397`) cites the identical rule inventory — **418 entries: 291 enforced / 22 partial / 104 declared / 1 enforced_scaffolding** — and the same module set the Code rows reference (`ade_ledger::{mithril_utxo_materialize, ledgerdb_state, ledgerdb_tables, reduced_epoch_view, value, pparams}`, `ade_node::{epoch_wire, epoch_activation, epoch_rebind, native_firstrun}`, `ade_runtime::{mithril_native_assembly, chaindb::reduced_window_driver, chaindb::transient_epoch_view}`). No Code row points at a module CODEMAP omits.
- CODEMAP independently verifies the headline: `grep -rn EVIEW_ACTIVATION_ARMED crates/` → 0 hits — consistent with the `DC-EPOCH-13` Code/CI trace above.

### Minor count discrepancy to note (not a TRACEABILITY defect)

- CODEMAP reports **508 canonical types / 2959 tests / 238 CI** at HEAD; HEAD_DELTAS §6/§7 cite **504 BLUE-core-path types / ~2934 tests** as the baseline-to-HEAD structural delta. These measure slightly different sets (CODEMAP's full canonical count + the `#[test]`/`#[tokio::test]` attribute count vs HEAD_DELTAS' BLUE-`core_paths`-only `pub struct/enum` structural count, marked approximate). TRACEABILITY counts **rules**, not types/tests, so this does not affect any cell here; surfaced for coherence. CI count agrees (238).

---

## Open questions for the user

1. **Live-flip transcripts (`DC-EPOCH-11`, `DC-EVIEW-08`).** Both stay `declared` pending a committed live epoch-boundary transcript; the SELECT/flip evidence exists off-repo and the flip is operator-gated. Should a follow-up slice commit the transcript (and the closed convergence-vocabulary events asserting the consumption) so these flip to `enforced`, or do they remain deliberately operator-gated until the broader EVIEW/ECA cluster closes?
2. **Stale test reference in `DC-NODE-18`.** `extend_own_spine_promotion_requires_adoption_certificate` no longer exists (the adoption-certificate path was removed by `DC-NODE-21`). Repair the registry `tests` array (remove the stale name) — confirm the intended replacement is none (the 5 remaining tests already cover the rule)?
3. **Orphaned-CI binding gaps.** Four green gates name a rule in their header but are not listed in that rule's registry `ci_scripts` (`DC-MEM-06`, `DC-LEDGER-03`, `DC-NODE-19`, `DC-NODE-20`), and `ci_check_wire_rollback_signal_preserved.sh` is bound to no rule. Append them to the respective `ci_scripts` arrays?
4. **Owed MEM-OPT-UTXO-DISK B-infra gates.** The dormant on-disk-UTxO + diagnostic gates remain rule-less by design until the `track_utxo=true` band lands and binds them to `DC-MEM-05`/`DC-MEM-06`. Confirm they should stay unbound until then (vs. binding the diagnostic subset now)?
5. **Enforced rules with an empty Tests or CI cell** (Tests: `T-BUILD-01`, `T-BOUND-02`, `DC-CRYPTO-02`, `CN-STORE-08`, `DC-WAL-01`, `DC-ADMIT-09`, `DC-OUTBOUND-FIFO-01`, `CN-OPERATOR-EVIDENCE-01`, `RO-CLOSE-01`; CI: 23 rules). Should the registry name the specific proving test / bind a dedicated CI gate for each, or are these accepted as structurally/shared-scan enforced?

---

## Generation notes

Regenerated at HEAD `cdcd9397` (baseline `470f9b89`, unchanged — mid-flight refresh, no cluster closed since baseline). Primary rule source: the invariant registry `docs/ade-invariant-registry.toml` (418 active entries, 0 deprecated). Each rule is a join of registry fields × codebase introspection: Requirement from the registry `statement` (or paraphrased from `source`/`code_locus` where `statement` is empty — 187 rules — and marked as such); Code from `code_locus`; Tests from `tests`; CI from `ci_script` + `ci_scripts`. **Every named CI script and every bare-fn test name was verified to exist** under `ci/` and in the `crates/` source tree respectively (static existence checks). The project `replay_cmd` (`cargo test -p ade_testkit`) was deliberately NOT run — it hangs on a known pre-existing test (`epoch_boundary_logic`, commit `7c769801`). No rule, Code, Tests, or CI value was invented; empty cells are marked as gaps. This doc is regenerated, not hand-edited — fix the registry or the code, not this doc.