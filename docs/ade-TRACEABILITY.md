# Normative Rule Traceability — Ade

> **Status:** Living architectural document. Regenerated; not hand-edited.
> Per-project instance of `~/.claude/methodology/templates/traceability.md`.

> **Baseline:** `470f9b89` (MEM-OPT-UTXO-DISK close). **HEAD:** `1e4896eb` (LIVE-FORGE-HARDENING cluster close, `origin/main`). This is a **cluster-close refresh** regenerated at HEAD from the invariant registry `docs/ade-invariant-registry.toml` (**432** rules) joined against the codebase; the sibling CODEMAP + HEAD_DELTAS were regenerated at the same HEAD. Prior TRACEABILITY was `cdcd9397` (2026-06-24, 418 rules) — six weeks / ~155 commits stale; ~10 clusters closed since (ECA, native-Mithril, LIVE-LEDGER-EPOCH-TRANSITION, REDUCED-VALIDATION-BOUNDARY-PLANE, Conway ratify/enact + proposal-deposit-expiry, CE3D, B3C-STAKE-RESIDUAL, LEDGER-VALUE-CORRECTNESS, LIVE-FORGE-HARDENING).

This document is the **invariant ↔ enforcement audit** IDD §10 demands. For every rule the project commits to, it traces: where the rule is *specified* (Source), what must hold (Requirement), where it is *enforced in code* (Code), which *tests prove* it (Tests), and which *CI check(s)* fail the build on violation (CI). A rule that cannot fill all four load-bearing cells is an **enforcement gap** — surfaced here (see the *Enforcement gaps* section), never hidden.

## Source of rules

The canonical rule source is the **invariant registry** `docs/ade-invariant-registry.toml` (declared in `.idd-config.json` `invariant_registry`). This doc is a join: registry entries × codebase introspection. Rule IDs, families, the Requirement (registry `statement`), Source (`source`), Code (`code_locus`), Tests (`tests`), and CI (`ci_script` / `ci_scripts`) all come from the registry. **Every named CI script and every code path was verified to exist against the codebase at HEAD** (static existence checks). Test-function existence was likewise checked statically; a test name the registry lists that is **not present at HEAD** is flagged inline with a dagger (**†**) and enumerated under *Cross-reference checks*. The project `replay_cmd` (`cargo test -p ade_testkit`) was **NOT executed** — it has 4 KNOWN pre-existing failures in `consensus_stream_replay` (an in-flight ECA-B rolling-nonce corpus issue, `NonceEvolution` `MissingLastEpochBlockNonce`) plus a pre-existing `epoch_boundary_logic` hang; both are unrelated to this audit.

The families are the registry ID prefixes; there is no `[families]` table in the registry, so the level-2 groupings (T / CN / DC / OP / RO) and the level-3 sub-family groupings (the `XX-YYYY` ID stem) are preserved from the prior TRACEABILITY for stability. Within each sub-family, rules are ordered by stable ID; IDs are append-only and never reused, so the ordering does not shuffle when rules are added.

## Rule inventory (mechanical, at HEAD)

| Status | Count |
|--------|------:|
| enforced | 297 |
| partial | 23 |
| declared | 111 |
| enforced_scaffolding | 1 |
| **Total** | **432** |

Source: the registry at `1e4896eb` — 432 `[[rules]]` blocks, one `id` + one `status` each. **0 deprecated.** Matches CODEMAP / HEAD_DELTAS (297 / 23 / 111 / 1).

| Family | Rules | enforced | partial | declared | enf-scaffold |
|--------|------:|---------:|--------:|---------:|-------------:|
| T | 33 | 15 | 4 | 14 | 0 |
| CN | 120 | 59 | 5 | 56 | 0 |
| DC | 253 | 215 | 10 | 27 | 1 |
| OP | 10 | 3 | 1 | 6 | 0 |
| RO | 16 | 5 | 3 | 8 | 0 |
| **All** | **432** | **297** | **23** | **111** | **1** |

---

## T — True Invariants (Project Constitution §2)

_33 rules._

### T-BOUND

#### `T-BOUND-01` — _declared_

| Aspect | Location |
|--------|----------|
| **Source** | Project constitution §2 |
| **Requirement** | Shell may observe nondeterminism but must convert to deterministic inputs before entering core |
| **Code** | — _(declared — not yet enforced)_ |
| **Tests** | — _(declared — not yet enforced)_ |
| **CI** | — _(declared — not yet enforced)_ |

#### `T-BOUND-02` — _enforced_

| Aspect | Location |
|--------|----------|
| **Source** | Project constitution §2, GP-4 |
| **Requirement** | Authoritative crates never depend on shell crates |
| **Code** | crates/*/Cargo.toml |
| **Tests** | _(no tests listed — gap)_ |
| **CI** | `ci/ci_check_dependency_boundary.sh` |

### T-BUILD

#### `T-BUILD-01` — _enforced_

| Aspect | Location |
|--------|----------|
| **Source** | Project constitution §2 |
| **Requirement** | No semantic build variability in authoritative code |
| **Code** | crates/ade_ledger/src/lib.rs |
| **Tests** | _(no tests listed — gap)_ |
| **CI** | `ci/ci_check_no_semantic_cfg.sh` |

#### `T-BUILD-02` — _declared_

| Aspect | Location |
|--------|----------|
| **Source** | Project constitution §2 |
| **Requirement** | One semantic interpretation per protocol version and input set |
| **Code** | — _(declared — not yet enforced)_ |
| **Tests** | — _(declared — not yet enforced)_ |
| **CI** | — _(declared — not yet enforced)_ |

### T-CAUSAL

#### `T-CAUSAL-01` — _declared_

| Aspect | Location |
|--------|----------|
| **Source** | Project constitution §2, audit #17 |
| **Requirement** | Future decisions may not leak into present validation; no retroactive reinterpretation of prior checkpoints |
| **Code** | — _(declared — not yet enforced)_ |
| **Tests** | — _(declared — not yet enforced)_ |
| **CI** | — _(declared — not yet enforced)_ |

### T-CI

#### `T-CI-01` — _partial_

| Aspect | Location |
|--------|----------|
| **Source** | Project constitution §2, CI-1 |
| **Requirement** | Every true invariant has mechanical CI enforcement. No waivers. |
| **Code** | ci/ |
| **Tests** | _(no tests listed — gap)_ |
| **CI** | `ci/ci_check_registry_code_locus_exists.sh` |

### T-COLL

#### `T-COLL-01` — _declared_

| Aspect | Location |
|--------|----------|
| **Source** | Project constitution §2, D-5 |
| **Requirement** | Deterministic iteration order for all semantically meaningful collections |
| **Code** | — _(declared — not yet enforced)_ |
| **Tests** | — _(declared — not yet enforced)_ |
| **CI** | — _(declared — not yet enforced)_ |

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
| **Code** | — _(declared — not yet enforced)_ |
| **Tests** | — _(declared — not yet enforced)_ |
| **CI** | — _(declared — not yet enforced)_ |

### T-CONSERV

#### `T-CONSERV-01` — _enforced_

| Aspect | Location |
|--------|----------|
| **Source** | Project constitution §2, audit #13 |
| **Requirement** | UTxO and asset conservation must hold for every accepted transition, except where protocol rules explicitly authorize mint, burn, rewards, or treasury effects |
| **Code** | crates/ade_ledger/src/value.rs, crates/ade_ledger/src/byron.rs |
| **Tests** | `all_eras_replay_summary`; `byron_replay_all_1500`; `shelley_replay_all_1500`; `conway_conservation_full` |
| **CI** | `ci/ci_check_differential_divergence.sh` |

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
| **Code** | — _(declared — not yet enforced)_ |
| **Tests** | — _(declared — not yet enforced)_ |
| **CI** | — _(declared — not yet enforced)_ |

#### `T-CORE-04` — _declared_

| Aspect | Location |
|--------|----------|
| **Source** | Project constitution §2, §5.1 |
| **Requirement** | Illegal states unrepresentable via types where practical |
| **Code** | — _(declared — not yet enforced)_ |
| **Tests** | — _(declared — not yet enforced)_ |
| **CI** | — _(declared — not yet enforced)_ |

### T-DET

#### `T-DET-01` — _enforced_

| Aspect | Location |
|--------|----------|
| **Source** | Project constitution §2, Byte Authority Model §3 |
| **Requirement** | Same canonical inputs -> same authoritative bytes (per Byte Authority Model) |
| **Code** | crates/ade_ledger/src/rules.rs, crates/ade_core/src/consensus/encoding.rs, crates/ade_runtime/src/consensus/chain_selector.rs, crates/ade_testkit/src/consensus/stream_replay.rs, crates/ade_testkit/src/validity/replay.rs (block_validity verdict-surface replay over the Conway-576 positive corpus; PHASE4-B1-S6), crates/ade_testkit/src/tx_validity/ (tx-validity verdict-surface replay: replay_tx_validity drives the BLUE tx_validity over every extracted Conway-576 corpus tx twice and the surfaces are byte-identical; PHASE4-B2-S3) |
| **Tests** | `apply_block_deterministic`; `byron_determinism`; `shelley_determinism`; `allegra_determinism`; `mary_determinism`; `alonzo_determinism`; `babbage_determinism`; `conway_determinism`; `layout_is_stable`; `roundtrip_empty_state` … (+11 more) |
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
| **Code** | — _(declared — not yet enforced)_ |
| **Tests** | — _(declared — not yet enforced)_ |
| **CI** | — _(declared — not yet enforced)_ |

#### `T-ENC-03` — _enforced_

| Aspect | Location |
|--------|----------|
| **Source** | Project constitution §2, D-3 |
| **Requirement** | Round-trip identity: encode(decode(bytes)) == bytes for valid encodings |
| **Code** | crates/ade_codec/src/preserved.rs, crates/ade_network/src/codec/handshake.rs, crates/ade_network/src/codec/n2c_handshake.rs, crates/ade_network/src/codec/chain_sync.rs, crates/ade_network/src/codec/block_fetch.rs, crates/ade_network/src/codec/tx_submission.rs, crates/ade_network/src/codec/keep_alive.rs, crates/ade_network/src/codec/peer_sharing.rs, crates/ade_network/src/codec/local_chain_sync.rs, crates/ade_network/src/codec/local_tx_submission.rs, crates/ade_network/src/codec/local_state_query.rs, crates/ade_network/src/codec/local_tx_monitor.rs |
| **Tests** | `full_corpus_round_trip::all_42_blocks_round_trip_byte_identical`; `full_corpus_round_trip::all_42_blocks_fields_match_reference`; `codec::handshake::tests::roundtrip_every_variant`; `codec::n2c_handshake::tests::roundtrip_every_variant`; `codec::chain_sync::tests::roundtrip_every_variant`; `codec::block_fetch::tests::roundtrip_every_variant`; `codec::tx_submission::tests::roundtrip_every_variant`; `codec::keep_alive::tests::roundtrip_every_variant`; `codec::peer_sharing::tests::roundtrip_every_variant`; `codec::local_chain_sync::tests::roundtrip_every_variant` … (+9 more) |
| **CI** | `ci/ci_check_cbor_round_trip.sh` |

### T-EPOCH

#### `T-EPOCH-01` — _partial_

| Aspect | Location |
|--------|----------|
| **Source** | Project constitution §2, 01_core §13, audit #18 |
| **Requirement** | Exactly one authoritative committee and governance interpretation per epoch |
| **Code** | crates/ade_ledger/src/governance.rs (plan_conway_governance_epoch — the single Conway governance-epoch authority); crates/ade_ledger/src/rules.rs (apply_epoch_boundary_with_registrations — the sole application point, shared by accumulator-follow and direct replay) |
| **Tests** | `cre_s4_3a_cross_path_gov_delta_is_identical`; `cre_s4_3a_replay_path_refunds_all_five_expiries`; `cre_s4_3a_potentially_ratifiable_terminals_both_paths`; `cre_s4_3a_rupd_consumed_once_with_governance_refund_at_seed_boundary`; `cre_s4_3a_single_governance_authority_no_second_path`; `cre_s4_3c_enactment_is_identical_on_replay_and_accumulator_paths`; `cre_s4_3c_supported_witness_enacts_prunes_siblings_refunds_all`; `cre_s4_3c_ratifiable_chain_child_of_winner_halts`; `cre_s4_3c_competing_ratifiable_siblings_halt`; `cre_s4_3c_pending_chain_child_is_carried_not_pruned` |
| **CI** | _(no CI script listed)_ |

### T-ERR

#### `T-ERR-01` — _partial_

| Aspect | Location |
|--------|----------|
| **Source** | Project constitution §2 |
| **Requirement** | Errors in authoritative paths are structured, comparable, canonical |
| **Code** | crates/ade_ledger/src/error.rs |
| **Tests** | `ledger_error_equality`; `conservation_error_display`; `codec_error_conversion` |
| **CI** | _(no CI script listed)_ |

#### `T-ERR-02` — _declared_

| Aspect | Location |
|--------|----------|
| **Source** | Project constitution §2, E-1 |
| **Requirement** | Safety violations fail-fast deterministically |
| **Code** | — _(declared — not yet enforced)_ |
| **Tests** | — _(declared — not yet enforced)_ |
| **CI** | — _(declared — not yet enforced)_ |

### T-INGRESS

#### `T-INGRESS-01` — _declared_

| Aspect | Location |
|--------|----------|
| **Source** | Project constitution §2, audit #21 |
| **Requirement** | All authoritative external bytes enter the core through named canonical decode/validation chokepoints; unchecked bypasses forbidden except for explicitly whitelisted sites with CI enforcement |
| **Code** | — _(declared — not yet enforced)_ |
| **Tests** | — _(declared — not yet enforced)_ |
| **CI** | `ci/ci_check_ingress_chokepoints.sh` |

### T-KEY

#### `T-KEY-01` — _declared_

| Aspect | Location |
|--------|----------|
| **Source** | Project constitution §2, audit #20 |
| **Requirement** | Signing and private key operations confined to shell; verification in core |
| **Code** | — _(declared — not yet enforced)_ |
| **Tests** | — _(declared — not yet enforced)_ |
| **CI** | — _(declared — not yet enforced)_ |

### T-NOSPEND

#### `T-NOSPEND-01` — _enforced_

| Aspect | Location |
|--------|----------|
| **Source** | Project constitution §2, audit #14 |
| **Requirement** | No input or equivalent spend authority may be consumed more than once in an accepted canonical chain |
| **Code** | crates/ade_ledger/src/utxo.rs |
| **Tests** | `check_duplicate_inputs_catches_dupes`; `duplicate_inputs_detected`; `all_eras_replay_summary` |
| **CI** | `ci/ci_check_differential_divergence.sh` |

### T-PLATFORM

#### `T-PLATFORM-01` — _declared_

| Aspect | Location |
|--------|----------|
| **Source** | Project constitution §2, audit #16 |
| **Requirement** | No host-environment property (locale, timezone, architecture, platform) may influence authoritative computation results |
| **Code** | — _(declared — not yet enforced)_ |
| **Tests** | — _(declared — not yet enforced)_ |
| **CI** | — _(declared — not yet enforced)_ |

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
| **Requirement** | Loop-as-replay: the same recovered/bootstrapped state + the same ordered canonical block feed (NodeBlockSource) + the same deterministic loop inputs + the same shutdown schedule produce byte-identical authoritative outputs (tips, WAL/checkpoints, and halt state). Extends T-REC-01/T-REC-02 from single-shot recovery to continuous relay operation; rides existing recovery laws (snapshot + forward-replay, NOT full-genesis replay) -- no new durability law. |
| **Code** | crates/ade_node/src/node_lifecycle.rs (run_relay_loop driver), crates/ade_node/src/node_sync.rs (relay_loop_two_clean_runs_byte_identical evidence test) |
| **Tests** | `relay_loop_two_clean_runs_byte_identical` |
| **CI** | `ci/ci_check_node_run_loop_containment.sh` |

#### `T-REC-04` — _enforced_

| Aspect | Location |
|--------|----------|
| **Source** | docs/clusters/PHASE4-N-F-G-N/cluster.md |
| **Requirement** | The WarmStart-recovered forge `chain_dep.epoch_nonce` (eta0) MUST come from the imported/recovered consensus input, never from a snapshot placeholder and never from genesis. Authoritative recovered state is explicit, persisted, replayable, and comparable: the seed-epoch eta0 is carried as an EXPLICIT field in the persisted `SeedEpochConsensusInputs` sidecar and applied (overlaid) onto the recovered `PraosChainDepState` in the single `bootstrap_initial_state` authority. A snapshot-seeded `Nonce::ZERO` must NEVER reach the forge/self_accept path when a seed-epoch lineage exists. Fail-closed: an old sidecar that omits `epoch_nonce` decodes as a version mismatch (`UnknownVersion`, schema v1 vs v2), never a default-to-zero eta0. Authority split preserved: snapshot = ledger/chain skeleton; seed-epoch sidecar = the Praos consensus inputs incl. eta0. |
| **Code** | crates/ade_ledger/src/seed_consensus_inputs.rs (SeedEpochConsensusInputs.epoch_nonce field + versioned codec, SEED_CINPUT_SCHEMA_VERSION=2 fail-closed on the schema change); crates/ade_runtime/src/seed_consensus_merge.rs (merge persists canonical.epoch_nonce); crates/ade_runtime/src/bootstrap.rs (bootstrap_initial_state overlays the recovered sidecar epoch_nonce onto chain_dep.epoch_nonce + evolving_nonce) |
| **Tests** | `warm_start_overlays_recovered_eta0_onto_chain_dep_g_n`; `warm_start_restores_seed_epoch_consensus_inputs_byte_identical`; `seed_cinput_decode_rejects_unknown_version`; `seed_epoch_consensus_inputs_round_trips_byte_identical`; `pinning_preseed_warmstart_roundtrip_faithful` |
| **CI** | `ci/ci_check_warmstart_eta0_overlay.sh` |

#### `T-REC-05` — _enforced_

| Aspect | Location |
|--------|----------|
| **Source** | docs/planning/phase4-n-u-forged-block-durability-invariants.md |
| **Requirement** | Replay/recovery equivalence including forged admits. Same BootstrapAnchor + same WAL (including forged AdmitBlock entries) -> byte-identical recovered durable tip and ledger fingerprint. Same recovered/bootstrapped state + same ordered canonical block feed + same deterministic clock-tick schedule + same leadership/key inputs + same shutdown schedule -> byte-identical durable outputs (tip, WAL image, checkpoints, halt state), INCLUDING forged-then-admitted blocks. Extends T-REC-01/02/03 from received-only to forged+received durable progression; rides the existing snapshot + forward-replay recovery law (NOT full-genesis replay) -- no new durability law. |
| **Code** | crates/ade_node/src/node_lifecycle.rs (warm_start_recovery: sidecar-reconstructed era_schedule/ledger_view + forward-replay via bootstrap_initial_state + fingerprint fail-fast guard); crates/ade_node/src/node_sync.rs (admit_forged_block_durably -> pump_block); crates/ade_runtime/src/bootstrap.rs (bootstrap_initial_state warm-start forward-replay -- reused) |
| **Tests** | `forge_kill_then_warm_start_recovers_same_tip_via_forward_replay`; `forge_tip_successor_kill_then_warm_start_recovers_block_one`; `recover_follow_forge_two_runs_byte_identical`; `recover_follow_kill_warm_start_chains_from_ledger_fp`; `recover_follow_two_runs_byte_identical`; `same_store_same_anchor_point_same_findintersect_start` |
| **CI** | _(no CI script listed)_ |

#### `T-REC-06` — _enforced_

| Aspect | Location |
|--------|----------|
| **Source** | docs/planning/phase4-n-an-rollback-materialize-eta0-invariants.md + docs/clusters/PHASE4-N-AN/cluster.md |
| **Requirement** | Rollback-materialization replay-equivalence (PHASE4-N-AN). A block that validates during live admit (against the eta0-overlaid chain_dep, T-REC-04) MUST NOT fail rollback-materialize replay because materialization substituted a different nonce source. materialize_rolled_back_state (the SOLE rolled-back-state authority, CN-STORE-07) MUST reconstruct the replay chain_dep with the SAME recovered eta0 (epoch nonce) the live-admit path uses (praos_vrf_input(slot, eta0), DC-CINPUT-03); the persisted snapshot's placeholder / genesis epoch_nonce MUST NOT reach VRF verification on the rollback-replay path. Same recovered store + same ordered WAL/feed => same chain_dep inputs => same block_validity result on the live-admit and rollback paths. eta0 is the recovered canonical input (the seed-epoch SeedEpochConsensusInputs.epoch_nonce sidecar) -- never peer data, wall-clock, CLI re-supply, or a re-query. VRF validation strength is UNCHANGED on the rollback path (no bypass / skip / loosening). SCOPE: the recovered seed epoch (no epoch-boundary crossing within the follow window -- eta0 is the constant epoch nonce); a multi-epoch rollback nonce-evolution is a named out-of-scope follow-on. Surfaced by the CE-AI-6 reorg (the rollback-follow died at ReplayFailedAt VrfCert); unblocks CE-AI-6. |
| **Code** | crates/ade_ledger/src/rollback/materialize.rs (materialize_rolled_back_state -- the SOLE rolled-back-state authority; AN-S2 overlays the recovered eta0 onto the nearest_le snapshot chain_dep before the replay-forward block_validity fold), crates/ade_ledger/src/seed_consensus_inputs.rs (SeedEpochConsensusInputs.epoch_nonce -- the persisted recovered eta0 carrier), crates/ade_node/src/node_lifecycle.rs (apply_chain_event threads state.seed_epoch_consensus_inputs eta0 into the materialize call) |
| **Tests** | `rollback_materialize_overlays_recovered_eta0_replay_equivalent (crates/ade_ledger/src/rollback/materialize.rs -- None overlay => VrfCert; Some(eta0) => Valid + materialized epoch_nonce == eta0 == the live-admit nonce basis)`; `rollback_materialize_does_not_bypass_vrf_on_wrong_eta0 (crates/ade_ledger/src/rollback/materialize.rs -- a WRONG eta0 still fails the header VRF: the overlay is not a bypass)` |
| **CI** | `ci/ci_check_rollback_materialize_eta0.sh` |

### T-RESOURCE

#### `T-RESOURCE-01` — _declared_

| Aspect | Location |
|--------|----------|
| **Source** | Project constitution §2, audit #5 |
| **Requirement** | Untrusted inputs must not allocate unbounded authoritative resources before deterministic validation |
| **Code** | — _(declared — not yet enforced)_ |
| **Tests** | — _(declared — not yet enforced)_ |
| **CI** | — _(declared — not yet enforced)_ |

### T-TRANSPORT

#### `T-TRANSPORT-01` — _declared_

| Aspect | Location |
|--------|----------|
| **Source** | Project constitution §2, audit #4 |
| **Requirement** | Transport nondeterminism (socket fragmentation, mux ordering, timeouts) must not leak into authoritative logic |
| **Code** | — _(declared — not yet enforced)_ |
| **Tests** | — _(declared — not yet enforced)_ |
| **CI** | — _(declared — not yet enforced)_ |

---

## CN — Classification-Table Invariants (constraint network)

_120 rules._

### CN-ADMIT

#### `CN-ADMIT-01` — _enforced_

| Aspect | Location |
|--------|----------|
| **Source** | docs/planning/phase4-n-m-b-admission-invariants.md §1 (I-B5) |
| **Requirement** | Single admission-mode entry authority: exactly one pub fn in ade_node::admission::runner::run_admission enters the admission tokio runner. No second entry point; no per-call fallback. The runner composes N-M-A seed-import + bootstrap_anchor::mint + seed_to_snapshot + bootstrap_initial_state warm-start + N-L n2n_dialer + per-AdmittedBlock loop + WalStore::append + verdict::derive + admission_log emit. |
| **Code** | crates/ade_node/src/admission/runner.rs::run_admission + crates/ade_node/src/admission/bootstrap.rs::dispatch_admission |
| **Tests** | `ade_node::admission::runner::tests::run_admission_emits_shutdown_on_signal`; `ade_node::admission::runner::tests::run_admission_emits_shutdown_on_channel_close`; `ade_node::admission::runner::tests::run_admission_disconnect_to_zero_peers_clean_exit` |
| **CI** | `ci/ci_check_admission_runner_closure.sh` |

#### `CN-ADMIT-02` — _enforced_

| Aspect | Location |
|--------|----------|
| **Source** | docs/planning/phase4-n-m-b-admission-invariants.md §1 (I-B6) |
| **Requirement** | Single seed-to-snapshot bridge authority: exactly one pub fn in ade_node::admission::seed_to_snapshot converts the imported (UTxOState, ledger_fingerprint, seed_point) into a persisted snapshot via PersistentSnapshotCache::capture. The bridge does NOT bypass bootstrap_initial_state — it persists a snapshot at seed_point.slot so the warm-start branch picks it up. CN-NODE-01 preserved. |
| **Code** | crates/ade_node/src/admission/seed_to_snapshot.rs::seed_to_snapshot |
| **Tests** | `ade_node::admission::seed_to_snapshot::tests::seed_to_snapshot_writes_via_persistent_cache`; `ade_node::admission::seed_to_snapshot::tests::seed_to_snapshot_returns_initial_ledger_fingerprint`; `ade_node::admission::seed_to_snapshot::tests::seed_to_snapshot_two_runs_byte_identical`; `ade_node::admission::seed_to_snapshot::tests::seed_to_snapshot_propagates_pre_conway_encode_error_as_authority_fatal` |
| **CI** | `ci/ci_check_admission_no_refscript_skip.sh` |

### CN-ANCHOR

#### `CN-ANCHOR-01` — _enforced_

| Aspect | Location |
|--------|----------|
| **Source** | docs/planning/phase4-n-m-ledger-seed-invariants.md §1 (I-A3) |
| **Requirement** | Single BootstrapAnchor mint authority: exactly one pub fn in ade_runtime::bootstrap_anchor::mint produces a BootstrapAnchor with all 6 fields populated (network_magic, genesis_hash, seed_point: {slot, block_hash}, seed_artifact_hash, imported_utxo_fingerprint, initial_ledger_fingerprint). The struct has no default impl and no #[non_exhaustive]; any missing field is a compile error. |
| **Code** | crates/ade_runtime/src/bootstrap_anchor.rs, crates/ade_ledger/src/bootstrap_anchor/anchor.rs |
| **Tests** | `crates/ade_runtime/src/bootstrap_anchor.rs::tests::mint_composes_inputs_byte_identically`; `crates/ade_runtime/src/bootstrap_anchor.rs::tests::mint_then_round_trip_via_canonical_cbor`; `crates/ade_runtime/src/bootstrap_anchor.rs::tests::mint_carries_seed_point_correctly`; `crates/ade_runtime/src/bootstrap_anchor.rs::tests::mint_propagates_utxo_fingerprint_into_anchor`; `crates/ade_ledger/src/bootstrap_anchor/anchor.rs::tests::bootstrap_anchor_match_is_exhaustive` |
| **CI** | `ci/ci_check_bootstrap_anchor_closure.sh` |

### CN-BUILD

#### `CN-BUILD-01` — _declared_

| Aspect | Location |
|--------|----------|
| **Source** | classification_table.md §K |
| **Requirement** | No build profile, feature flag, cfg, or optimization mode may alter authoritative semantics or persisted bytes |
| **Code** | — _(declared — not yet enforced)_ |
| **Tests** | — _(declared — not yet enforced)_ |
| **CI** | — _(declared — not yet enforced)_ |

#### `CN-BUILD-02` — _declared_

| Aspect | Location |
|--------|----------|
| **Source** | classification_table.md §K |
| **Requirement** | All semantic variability must be explicit runtime protocol data, not hidden compile-time choice |
| **Code** | — _(declared — not yet enforced)_ |
| **Tests** | — _(declared — not yet enforced)_ |
| **CI** | — _(declared — not yet enforced)_ |

#### `CN-BUILD-03` — _declared_

| Aspect | Location |
|--------|----------|
| **Source** | classification_table.md §K |
| **Requirement** | Exactly one semantic interpretation may exist for a given protocol version and bootstrap anchor |
| **Code** | — _(declared — not yet enforced)_ |
| **Tests** | — _(declared — not yet enforced)_ |
| **CI** | — _(declared — not yet enforced)_ |

#### `CN-BUILD-04` — _declared_

| Aspect | Location |
|--------|----------|
| **Source** | classification_table.md §K |
| **Requirement** | Operator configuration may tune transport, logging, and telemetry, but may not silently weaken ledger, consensus, or persistence semantics |
| **Code** | — _(declared — not yet enforced)_ |
| **Tests** | — _(declared — not yet enforced)_ |
| **CI** | — _(declared — not yet enforced)_ |

### CN-CINPUT

#### `CN-CINPUT-01` — _enforced_

| Aspect | Location |
|--------|----------|
| **Source** | docs/clusters/PHASE4-N-F-A/cluster.md; A1-seed-epoch-consensus-inputs-type.md |
| **Requirement** | The seed-epoch consensus inputs (epoch, active-slots coefficient, total active stake, and the per-pool active-stake + registered VRF keyhash distribution) persisted as Ade recovered state MUST be a single closed canonical type (SeedEpochConsensusInputs) with a SOLE encoder/decoder pair. The codec is deterministic CBOR, BTreeMap-ordered (no HashMap), version-gated (decode rejects any version != SEED_CINPUT_SCHEMA_VERSION fail-closed), and byte-canonical (a structurally valid but non-canonically-encoded buffer is rejected, not silently re-canonicalized). No second encoder/decoder for this type may exist. |
| **Code** | crates/ade_ledger/src/seed_consensus_inputs.rs (SeedEpochConsensusInputs + encode_/decode_seed_epoch_consensus_inputs + SeedConsensusInputsError) |
| **Tests** | `seed_epoch_consensus_inputs_round_trips_byte_identical`; `seed_cinput_decode_rejects_unknown_version`; `seed_cinput_decode_rejects_noncanonical_or_duplicate_keys` |
| **CI** | _(no CI script listed)_ |

#### `CN-CINPUT-02` — _enforced_

| Aspect | Location |
|--------|----------|
| **Source** | docs/clusters/PHASE4-N-F-A/cluster.md; A2-bootstrap-population-containment.md; A3a-wal-provenance-entry.md |
| **Requirement** | The SeedEpochConsensusInputs sidecar MUST be populated ONLY through the single shared ade_runtime::seed_epoch_lineage::persist_seed_epoch_consensus_inputs authority — the anchor-fp-keyed SnapshotStore surface (put_seed_epoch_consensus_inputs), built via the GREEN merge (merge_seed_epoch_consensus_inputs) and the A1 sole encoder, with the A3a WAL provenance append — called ONLY by the verified-bootstrap composition sites (genesis_bootstrap / mithril_bootstrap / the operator admission pre-seed ade_node::admission::bootstrap). PHASE4-N-F-G-I extracted the populator into this single authority (was inline-per-composer) and added the admission pre-seed caller, so a --mode node WarmStart recovers a forge-capable store seeded purely from the shared --json-seed + import_live_consensus_inputs path. The forge-time consensus-inputs path (produce_mode / import_live_consensus_inputs / pool_distr_view_from_consensus_inputs / --consensus-inputs-path) MUST NOT build or put the sidecar, nor append its WAL provenance. Enforced by a data-flow-resistant containment gate (global call-site scan, not a bypassable RHS grep). This does NOT forbid diagnostic import of LiveConsensusInputsCanonical (fixtures / pinning tests / first-run verified-bootstrap extraction may still exist); it forbids that import from populating, proving, or substituting for the recovered sidecar on any bounty-primary path. NOTE: this rule constrains POPULATION + the forge-time fence only; it does NOT assert that the producer CONSUMES the recovered surface — producer consumption is deferred to PHASE4-N-F-C and no registry rule is introduced for it in this close. |
| **Code** | crates/ade_runtime/src/seed_epoch_lineage.rs (single shared populator authority); crates/ade_runtime/src/genesis_bootstrap.rs; crates/ade_runtime/src/mithril_bootstrap.rs; crates/ade_node/src/admission/bootstrap.rs (admission pre-seed caller); crates/ade_runtime/src/seed_consensus_merge.rs; crates/ade_runtime/src/seed_consensus_provenance.rs; ci/ci_check_consensus_input_provenance.sh |
| **Tests** | `bootstrap_persists_anchor_keyed_seed_consensus_inputs`; `bootstrap_seed_inputs_merge_fails_closed_on_missing_vrf_or_stake`; `snapshot_store_keyed_sidecar_is_disjoint_from_slot_snapshots`; `persist_writes_anchor_keyed_sidecar_and_recoverable_wal_provenance` |
| **CI** | `ci/ci_check_consensus_input_provenance.sh` |

#### `CN-CINPUT-03` — _enforced_

| Aspect | Location |
|--------|----------|
| **Source** | docs/clusters/PHASE4-N-F-C/cluster.md; L5-produce-from-recovered-state.md |
| **Requirement** | Consume-side anti-laundering fence: on the node-lifecycle forge path the leadership view MUST be projected from the recovered SeedEpochConsensusInputs surface (PoolDistrView::from_seed_epoch_consensus_inputs over the recovered BootstrapState) and MUST NOT be built from a forge-time operator bundle. No SeedEpochConsensusInputs value may be CONSTRUCTED on the production forge path (no shape-swap of an operator bundle into the recovered-surface type), and the forge-time consensus-input tokens (import_live_consensus_inputs / pool_distr_view_from_consensus_inputs / --consensus-inputs-path) MUST NOT appear in the node-sync/forge driver. Enforced by a data-flow-resistant containment gate (guard (d) of ci_check_consensus_input_provenance.sh: positive recovered- projection grep + negative bundle/cold-token grep + no-literal-construction fence over the comment/test-stripped run_node_sync/forge body), not a bypassable RHS grep. |
| **Code** | crates/ade_node/src/node_sync.rs (forge_one_from_recovered — projects leadership only from the recovered BootstrapState); ci/ci_check_consensus_input_provenance.sh (guard (d) consume-side fence) |
| **Tests** | `forge_from_recovered_uses_recovered_pool_distr`; `forge_from_recovered_fails_closed_without_recovered_inputs` |
| **CI** | `ci/ci_check_consensus_input_provenance.sh` |

### CN-CONS-IN

#### `CN-CONS-IN-01` — _enforced_

| Aspect | Location |
|--------|----------|
| **Source** | docs/planning/phase4-n-m-c-operator-pass-invariants.md §1 (I-C1) |
| **Requirement** | Single LiveConsensusInputs importer authority: exactly one pub fn ade_runtime::consensus_inputs::importer::import_live_consensus_inputs converts a cardano-cli JSON bundle into a typed, validated LiveConsensusInputsCanonical. The BLUE admission path consumes only the canonicalized form. The raw JSON is RED / operational evidence; it never enters BLUE. Required canonical fields: network_magic, genesis_hash, era, epoch_no, epoch_start_slot, epoch_end_slot, active_slots_coeff, epoch_nonce, pool_distribution, pool_vrf_keyhashes, protocol_params_hash, source_cardano_node_version, source_query_command, source_tip_hash, source_tip_slot, fingerprint. |
| **Code** | crates/ade_runtime/src/consensus_inputs/importer.rs (import_live_consensus_inputs_raw / _from_bytes, raw typed form + closed error sum), crates/ade_runtime/src/consensus_inputs/canonical.rs (import_live_consensus_inputs / _from_bytes — SOLE Canonical-returning authority + canonical_from_raw lift), crates/ade_runtime/src/consensus_inputs/json.rs (parse_consensus_inputs_json structural decode) |
| **Tests** | `consensus_inputs::importer::tests::minimal_round_trip_imports_to_typed`; `consensus_inputs::importer::tests::unsupported_era_fails_fast`; `consensus_inputs::importer::tests::import_is_deterministic_across_repeated_calls`; `consensus_inputs::canonical::tests::import_round_trip_yields_canonical_form_with_fingerprint`; `consensus_inputs::canonical::tests::fingerprint_is_deterministic_across_repeated_imports` |
| **CI** | `ci/ci_check_live_consensus_inputs_closure.sh`; `ci/ci_check_live_consensus_inputs_fingerprint.sh` |

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
| **Tests** | `consensus::header_validate::tests::pipeline_short_circuits_on_first_failure`; `consensus::header_validate::tests::nonce_contribution_uses_nonce_role_vrf_output_not_leader_role`; `valid_header_accepted_advances_state`; `header_with_slot_regression_rejected`; `header_with_block_no_regression_rejected`; `header_with_op_cert_regression_rejected`; `header_with_invalid_vrf_proof_rejected`; `header_beyond_forecast_horizon_rejected`; `validate_replay_is_deterministic`; `consensus::candidate::tests::candidate_fragment_carries_anchor_block_no` … (+3 more) |
| **CI** | `ci/ci_check_header_body_binding.sh` |

#### `CN-CONS-05` — _declared_

| Aspect | Location |
|--------|----------|
| **Source** | classification_table.md §D |
| **Requirement** | Authoritative consensus decisions must not depend on wall-clock time, arrival-order races, scheduler interleavings, or OS behavior |
| **Code** | crates/ade_core/src/consensus/header_validate.rs, crates/ade_core/src/consensus/fork_choice.rs, crates/ade_core/src/consensus/candidate.rs |
| **Tests** | `validate_replay_is_deterministic`; `consensus::header_validate::tests::pipeline_short_circuits_on_first_failure`; `consensus::header_validate::tests::nonce_contribution_uses_nonce_role_vrf_output_not_leader_role`; `replay_is_deterministic`; `reject_reason_bytes_are_stable` |
| **CI** | `ci/ci_check_no_density_in_fork_choice.sh`; `ci/ci_check_no_float_in_consensus.sh` |

#### `CN-CONS-06` — _enforced_

| Aspect | Location |
|--------|----------|
| **Source** | docs/planning/phase4-n-c-invariants.md §1 (NC-LIVE-1); bounty acceptance test (validation + block-production both required) |
| **Requirement** | Cross-impl acceptance: blocks forged by Ade are accepted by cardano-node when delivered via N2N block-fetch / chain-sync. Evidence is operator-action: a sustained-window live_block_production_session against a private cardano-node capturing CE-N-C-LIVE_<date>.log. Conditional on testnet stake / SPO registration; if unavailable at cluster close the live half is marked blocked_until_operator_stake_available, not deferred. |
| **Code** | crates/ade_testkit/src/producer/cross_impl_adapter.rs (mechanical half — structural cross-impl agreement: decode round-trip + body-hash binding + decoder/encoder structural field agreement); crates/ade_runtime/src/producer/coordinator.rs (PHASE4-N-Q GREEN slot+forge-result coordinator); crates/ade_runtime/src/producer/producer_shell.rs (PHASE4-N-Q RED key-custody shell); crates/ade_node/src/produce_mode.rs (RED driver — run_real_forge live forge composition: N-R-A BLUE leader-check + N-S-A real KES over unsigned_header_pre_image + N-S-B OutboundCommand relay + N-W Praos VRF + N-X tag-24 serve); crates/ade_core_interop/src/bin/live_block_production_session.rs (legacy operator-action binary — superseded by ade_node --mode produce) |
| **Tests** | `cross_impl_adapter_forged_block_decodes_through_ade_codec`; `cross_impl_adapter_forged_block_structurally_agrees_with_decoder`; `cross_impl_adapter_corpus_round_trips_byte_identical` |
| **CI** | `ci/ci_check_producer_corpus_present.sh` |

#### `CN-CONS-07` — _enforced_

| Aspect | Location |
|--------|----------|
| **Source** | docs/planning/phase4-n-c-invariants.md §1 (NC-SELF-1); docs/clusters/PHASE4-N-U/S3-serve-as-durable-chain-projection.md (serve-provenance restatement) |
| **Requirement** | Self-acceptance bridge + serve provenance. A forged block is NOT eligible for RED broadcast unless Ade's own header validator (PHASE4-N-B path) and body validator (PHASE4-B1 path) accept it under the same slot, era, and context. Self-acceptance failure halts the producer deterministically. RED broadcast is gated on the BLUE self-accept verdict. SERVE PROVENANCE (no unvalidated bytes leave the node): every byte the node serves to a peer (ChainSync header advertisement / BlockFetch body) traces to the single validated admit path -- for --mode node, a deterministic PROJECTION of the durable ChainDb (whose sole production writers are pump_block / DC-NODE-12 and the validated warm-start / genesis replay bootstrap_initial_state), covering BOTH forged (self_accept) and received (admit_via_block_validity) durable bytes through the one durable admit; for --mode produce, the self_accept'd AcceptedBlock served-chain index. PHASE4-N-U restates the serve clause from the in-memory-token dependency (N-G S2: served bytes must originate as a live AcceptedBlock token, lost on restart) to durable-provenance -- PRESERVING the TRUE invariant (no unvalidated bytes leave) while letting serve follow the durable chain (a follower fetches coherent history A->B, never B without A; serve survives restart). See DC-NODE-13. |
| **Code** | crates/ade_ledger/src/producer/self_accept.rs (self_accept, AcceptedBlock, SelfAcceptError); crates/ade_ledger/src/block_validity/transition.rs (block_validity — single closed validator authority self_accept wraps); crates/ade_ledger/src/producer/served_chain.rs (ServedChainSnapshot, served_chain_admit — only AcceptedBlock values may enter the produce-mode served-chain index; PHASE4-N-G S2 strengthening preserves the broadcast gate across the network seam); crates/ade_runtime/src/network/served_chain_projection.rs (ChainDbServedSource — PHASE4-N-U S3: the --mode node served view is a read-only projection of the durable ChainDb, whose sole production writers are pump_block + bootstrap_initial_state; serves stored.bytes verbatim, reuses the single block_header_bytes / DC-CONS-18 header authority); crates/ade_runtime/src/network/serve_dispatch.rs (ServedChainSource enum — the single serve-dispatch authority reads either source; DC-NODE-07); crates/ade_ledger/src/receive/admitted.rs (AdmittedBlock, admit_via_block_validity — PHASE4-N-H mirror: receive-side admission gate via the same block_validity authority; symmetric to AcceptedBlock); crates/ade_ledger/src/receive/chain_write.rs (ChainDbWrite trait takes AdmittedBlock by value) |
| **Tests** | `self_accept_accepts_freshly_forged_block`; `self_accept_rejects_corrupted_body_hash`; `self_accept_rejects_invalid_kes_signature`; `self_accept_rejects_unbalanced_tx_in_body`; `broadcast_callable_only_with_accept_verdict`; `served_chain_admit_admits_corpus_block`; `served_chain_admit_idempotent_on_byte_identity`; `served_chain_admit_independent_of_order`; `served_chain_snapshot_iteration_is_btreemap_ordered`; `served_chain_block_bytes_accessor_returns_accepted_block_slice` … (+5 more) |
| **CI** | `ci/ci_check_self_accept_gate.sh`; `ci/ci_check_served_chain_closure.sh`; `ci/ci_check_admitted_block_closure.sh`; `ci/ci_check_receive_reducer_closure.sh` |

#### `CN-CONS-08` — _enforced_

| Aspect | Location |
|--------|----------|
| **Source** | docs/planning/receive-side-bridge-invariants.md §1 (I-1, folds I-7) |
| **Requirement** | Receive-side single admission authority: every block that lands in ChainDb via the receive path passed block_validity with BlockValidityVerdict::Valid. No bypass, no header-only fast path, no trusted-prefix mode. Invalid verdicts leave receive state unchanged and halt the peer pipeline with a structured error; no silent skip, no partial application. Receive-side analog of CN-CONS-07 (broadcast gate). |
| **Code** | crates/ade_ledger/src/receive/admitted.rs (AdmittedBlock + admit_via_block_validity — the single admission authority); crates/ade_ledger/src/receive/reducer.rs (block_delivered helper composes admit_via_block_validity then commits state atomically; failure leaves state unchanged); crates/ade_ledger/src/receive/chain_write.rs (ChainDbWrite::write_admitted takes AdmittedBlock by value) |
| **Tests** | `admit_via_block_validity_accepts_corpus_block`; `admit_via_block_validity_rejects_corrupted_body`; `receive_apply_block_delivered_with_matching_header_admits`; `receive_apply_block_delivered_validity_invalid_rejects`; `receive_apply_rollback_returns_out_of_scope`; `receive_apply_replay_byte_identical_over_corpus` |
| **CI** | `ci/ci_check_admitted_block_closure.sh`; `ci/ci_check_receive_reducer_closure.sh` |

### CN-CRYPTO

#### `CN-CRYPTO-01` — _declared_

| Aspect | Location |
|--------|----------|
| **Source** | classification_table.md §J |
| **Requirement** | Verification belongs to the authoritative core; signing belongs outside it |
| **Code** | — _(declared — not yet enforced)_ |
| **Tests** | — _(declared — not yet enforced)_ |
| **CI** | — _(declared — not yet enforced)_ |

#### `CN-CRYPTO-02` — _partial_

| Aspect | Location |
|--------|----------|
| **Source** | classification_table.md §J |
| **Requirement** | All consensus-relevant hashes must be domain-separated and unambiguous |
| **Code** | crates/ade_core/src/consensus/vrf_cert.rs |
| **Tests** | `vrf_input_layout_is_41_bytes_with_correct_tag`; `vrf_role_tags_match_convention`; `vrf_input_byte_layout` |
| **CI** | _(no CI script listed)_ |

#### `CN-CRYPTO-03` — _declared_

| Aspect | Location |
|--------|----------|
| **Source** | classification_table.md §J |
| **Requirement** | All collections with consensus meaning must be ordered deterministically before hashing or comparison |
| **Code** | — _(declared — not yet enforced)_ |
| **Tests** | — _(declared — not yet enforced)_ |
| **CI** | — _(declared — not yet enforced)_ |

#### `CN-CRYPTO-04` — _declared_

| Aspect | Location |
|--------|----------|
| **Source** | classification_table.md §J |
| **Requirement** | Verification failure must fail once and deterministically; no implicit parser or serialization fallback is allowed |
| **Code** | — _(declared — not yet enforced)_ |
| **Tests** | — _(declared — not yet enforced)_ |
| **CI** | — _(declared — not yet enforced)_ |

### CN-EPOCH

#### `CN-EPOCH-01` — _partial_

| Aspect | Location |
|--------|----------|
| **Source** | classification_table.md §C |
| **Requirement** | Stake, rewards, parameter changes, and governance effects may activate only at protocol-defined epoch boundaries |
| **Code** | crates/ade_core/src/consensus/leader_schedule.rs, crates/ade_core/src/consensus/ledger_view.rs, crates/ade_ledger/src/consensus_view.rs |
| **Tests** | `consensus::leader_schedule::tests::query_uses_state_epoch_nonce_for_vrf_input`; `corpus_returns_canonical_answer_for_known_pools`; `corpus_rejects_unknown_pool`; `corpus_is_deterministic_across_runs`; `view_returns_corpus_pool_stake_and_vrf_keyhash`; `view_unknown_epoch_returns_none`; `view_is_pure` |
| **CI** | _(no CI script listed)_ |

#### `CN-EPOCH-02` — _declared_

| Aspect | Location |
|--------|----------|
| **Source** | classification_table.md §C |
| **Requirement** | At each slot or epoch point there is exactly one authoritative committee and governance interpretation |
| **Code** | — _(declared — not yet enforced)_ |
| **Tests** | — _(declared — not yet enforced)_ |
| **CI** | — _(declared — not yet enforced)_ |

#### `CN-EPOCH-03` — _declared_

| Aspect | Location |
|--------|----------|
| **Source** | classification_table.md §C |
| **Requirement** | Stake snapshots and reward computations must be derivable solely from canonical chain state |
| **Code** | — _(declared — not yet enforced)_ |
| **Tests** | — _(declared — not yet enforced)_ |
| **CI** | — _(declared — not yet enforced)_ |

#### `CN-EPOCH-04` — _declared_

| Aspect | Location |
|--------|----------|
| **Source** | classification_table.md §C |
| **Requirement** | Future decisions may not leak into present validation, and later states may not retroactively reinterpret prior checkpoints |
| **Code** | — _(declared — not yet enforced)_ |
| **Tests** | — _(declared — not yet enforced)_ |
| **CI** | — _(declared — not yet enforced)_ |

### CN-FOLLOW

#### `CN-FOLLOW-01` — _enforced_

| Aspect | Location |
|--------|----------|
| **Source** | docs/clusters/PRODUCER-PARTICIPANT-FOLLOW/PRODUCER-PARTICIPANT-FOLLOW.md; docs/clusters/PRODUCER-PARTICIPANT-FOLLOW/CN-FOLLOW-01-participant-forge-on-ao-selected-head.md |
| **Requirement** | Producer / follow authority separation. (a) DETERMINISTIC SELECTION: the same candidate set yields the same selected canonical durable tip (the AO / select_best_chain law, arrival-order-independent). (b) FORGE-ONLY-ON-SELECTED-HEAD: a keyed producer forges if and only if it is leader on the AO-selected durable head (ChainDb::tip) -- never on a private or stale spine, never with a hidden authority, and never gated out by a per-tick exact-equality re-check the racing live frontier makes permanently unsatisfiable. Following a public multi-producer chain is PARTICIPANT behaviour; forging is PRODUCER behaviour; they are separate authorities. The block-producing node's follow authority IS the participant/AO chain-selection (run_participant_sync + fork-choice + store-based rewind), and the forge consumes its durable result -- the forge never re-selects, reorders, or prefers chains. |
| **Code** | crates/ade_node/src/node_sync.rs (participant_forge_decision GREEN decision + ParticipantForgeDecision/ParticipantForgeFenceReason + ForgeMode::ParticipantExtendOnSelectedHead + participant_forge_mode_on_caughtup/after_admit transitions); crates/ade_node/src/node_lifecycle.rs (run_relay_loop_with_sched ForgeTick: VenueRole::Participant routes to participant_forge_decision on the AO-selected durable ChainDb::tip fenced by DC-NODE-28; VenueRole::Unknown keeps the pure DC-NODE-15 path unchanged; the forge-base evidence emits ForgeBaseSource::LocalChaindbTip + cert_path_present:false for Participant too). The AO selection law (select_best_chain, DC-CONS-03 / CN-CONS-01) is CONSUMED, never re-implemented. |
| **Tests** | `participant_venue_forges_on_ao_selected_head_when_leader`; `participant_forge_base_is_ao_selected_chaindb_tip`; `participant_forge_base_is_servable_before_forge`; `participant_forge_refused_while_fork_choice_pending`; `participant_venue_requires_forge_activation`; `single_producer_forge_decision_unchanged`; `orphaned_startup_holds_forge_fence_participant`; `participant_forge_two_runs_byte_identical` |
| **CI** | `ci/ci_check_participant_forge_on_selected_head.sh` |

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

### CN-GENESIS

#### `CN-GENESIS-01` — _enforced_

| Aspect | Location |
|--------|----------|
| **Source** | docs/planning/phase4-n-r-invariants.md §1 (I7); §2 (N7); §3 (D5); N-R-A A1 OQ7 fixtures |
| **Requirement** | The Shelley genesis closed-contract parser accepts a real cardano-cli `shelley-genesis.json` and produces a canonical `GenesisAnchor`. Required fields (networkMagic, systemStart, slotLength, slotsPerKESPeriod, maxKESEvolutions) fail-closed on missing / malformed / wrong-type input. No implicit defaults (e.g., 'if missing, assume preprod' rejected). No stringly fallback (e.g., `"1"` rejected for u32 fields). Extra unknown keys accepted-and-ignored for forward compatibility, iff they do not alter interpretation — the `GenesisAnchor` produced from an extra-key fixture MUST byte-equal the canonical fixture's `GenesisAnchor`. The kes_anchor_slot is operator-supplied (not in genesis) and passed to the parser as a separate argument. systemStart parsing uses a deterministic ISO 8601 → Unix epoch milliseconds conversion (Howard Hinnant proleptic Gregorian). |
| **Code** | crates/ade_runtime/src/producer/genesis_parser.rs (parse_shelley_genesis + closed GenesisParseError enum + parse_iso8601_to_unix_ms + days_since_unix_epoch) |
| **Tests** | `accepted_shelley_genesis_parses_to_expected_anchor`; `missing_required_field_emits_structured_error`; `stringly_int_emits_malformed_field_type`; `extra_inert_keys_produce_byte_identical_anchor`; `malformed_numeric_negative_slot_length_rejected`; `iso8601_parse_anchors_to_known_unix_ms_values` |
| **CI** | `ci/ci_check_node_forge_real_cli_ingress.sh`; `ci/ci_check_genesis_consistency_fixture_present.sh` |

### CN-KES-HEADER

#### `CN-KES-HEADER-01` — _enforced_

| Aspect | Location |
|--------|----------|
| **Source** | docs/planning/phase4-n-s-invariants.md §1 (I1, I2); §2 (N1, N2) |
| **Requirement** | The KES signature in a forged block's header is over the canonical unsigned-header CBOR pre-image — the CBOR encoding of ShelleyHeaderBody (the first element of the outer [header_body, kes_signature] header array). The producer-side recipe (unsigned_header_pre_image) and the validator-side extractor (header_input::decode_block.header_input.kes.header_body_bytes) produce byte-identical output for every corpus block. The branded UnsignedHeaderPreImage(Vec<u8>) type's only constructor is the canonical recipe; kes_sign_header accepts only this type — arbitrary-byte signing is mechanically unrepresentable. |
| **Code** | crates/ade_ledger/src/block_validity/unsigned_header_pre_image.rs (UnsignedHeaderPreImage branded type + canonical recipe); crates/ade_runtime/src/producer/producer_shell.rs (ProducerShell::kes_sign_header accepts only the branded type); crates/ade_node/src/produce_mode.rs (run_real_forge two-pass bridge replaces the placeholder) |
| **Tests** | `unsigned_header_preimage_matches_decode_block_extraction_for_corpus`; `recipe_output_is_byte_identical_across_two_runs`; `shell_kes_sign_header_produces_verifiable_signature` |
| **CI** | `ci/ci_check_unsigned_header_preimage_single_source.sh` |

### CN-LEDGER

#### `CN-LEDGER-01` — _declared_

| Aspect | Location |
|--------|----------|
| **Source** | classification_table.md §B |
| **Requirement** | apply_block must be a pure deterministic function of prior state and canonical block input |
| **Code** | — _(declared — not yet enforced)_ |
| **Tests** | — _(declared — not yet enforced)_ |
| **CI** | — _(declared — not yet enforced)_ |

#### `CN-LEDGER-02` — _declared_

| Aspect | Location |
|--------|----------|
| **Source** | classification_table.md §B |
| **Requirement** | Same genesis/bootstrap + same block sequence must yield byte-identical authoritative state |
| **Code** | — _(declared — not yet enforced)_ |
| **Tests** | — _(declared — not yet enforced)_ |
| **CI** | — _(declared — not yet enforced)_ |

#### `CN-LEDGER-03` — _declared_

| Aspect | Location |
|--------|----------|
| **Source** | classification_table.md §B |
| **Requirement** | Validity decisions for transactions and blocks must match the Cardano reference oracle for the same era/protocol version |
| **Code** | — _(declared — not yet enforced)_ |
| **Tests** | — _(declared — not yet enforced)_ |
| **CI** | — _(declared — not yet enforced)_ |

#### `CN-LEDGER-04` — _declared_

| Aspect | Location |
|--------|----------|
| **Source** | classification_table.md §B |
| **Requirement** | Any two supported production versions that may coexist must return the same validity verdict for every consensus-relevant input |
| **Code** | — _(declared — not yet enforced)_ |
| **Tests** | — _(declared — not yet enforced)_ |
| **CI** | — _(declared — not yet enforced)_ |

#### `CN-LEDGER-05` — _declared_

| Aspect | Location |
|--------|----------|
| **Source** | classification_table.md §B |
| **Requirement** | Each feature must have one semantic processing result; no alternate path may disagree on whether work was already applied, failed, or remains valid |
| **Code** | — _(declared — not yet enforced)_ |
| **Tests** | — _(declared — not yet enforced)_ |
| **CI** | — _(declared — not yet enforced)_ |

#### `CN-LEDGER-06` — _declared_

| Aspect | Location |
|--------|----------|
| **Source** | classification_table.md §B |
| **Requirement** | Failure-state residue must be deterministic and consensus-neutral |
| **Code** | — _(declared — not yet enforced)_ |
| **Tests** | — _(declared — not yet enforced)_ |
| **CI** | — _(declared — not yet enforced)_ |

#### `CN-LEDGER-07` — _declared_

| Aspect | Location |
|--------|----------|
| **Source** | classification_table.md §B |
| **Requirement** | UTxO and asset conservation must hold for every accepted transition, except where protocol rules explicitly authorize mint, burn, rewards, or treasury effects |
| **Code** | — _(declared — not yet enforced)_ |
| **Tests** | `conway_conservation_full` |
| **CI** | — _(declared — not yet enforced)_ |

#### `CN-LEDGER-08` — _declared_

| Aspect | Location |
|--------|----------|
| **Source** | classification_table.md §B |
| **Requirement** | No input or equivalent spend authority may be consumed more than once in an accepted canonical chain |
| **Code** | — _(declared — not yet enforced)_ |
| **Tests** | — _(declared — not yet enforced)_ |
| **CI** | — _(declared — not yet enforced)_ |

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
| **Code** | — _(declared — not yet enforced)_ |
| **Tests** | — _(declared — not yet enforced)_ |
| **CI** | — _(declared — not yet enforced)_ |

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
| **Code** | — _(declared — not yet enforced)_ |
| **Tests** | — _(declared — not yet enforced)_ |
| **CI** | — _(declared — not yet enforced)_ |

#### `CN-MEM-03` — _declared_

| Aspect | Location |
|--------|----------|
| **Source** | classification_table.md §H |
| **Requirement** | Under overload, work shedding must follow deterministic policy, not timing-dependent collapse |
| **Code** | — _(declared — not yet enforced)_ |
| **Tests** | — _(declared — not yet enforced)_ |
| **CI** | — _(declared — not yet enforced)_ |

#### `CN-MEM-04` — _declared_

| Aspect | Location |
|--------|----------|
| **Source** | classification_table.md §H |
| **Requirement** | Mempool acceptance rules must never contradict block and ledger acceptance rules for the same authoritative semantics |
| **Code** | — _(declared — not yet enforced)_ |
| **Tests** | — _(declared — not yet enforced)_ |
| **CI** | — _(declared — not yet enforced)_ |

### CN-META

#### `CN-META-01` — _declared_

| Aspect | Location |
|--------|----------|
| **Source** | classification_table.md §N |
| **Requirement** | Every claimed invariant must have at least one mechanical enforcement point |
| **Code** | — _(declared — not yet enforced)_ |
| **Tests** | — _(declared — not yet enforced)_ |
| **CI** | — _(declared — not yet enforced)_ |

#### `CN-META-02` — _declared_

| Aspect | Location |
|--------|----------|
| **Source** | classification_table.md §N |
| **Requirement** | Every consensus-relevant failure mode must have a deterministic structured error shape |
| **Code** | — _(declared — not yet enforced)_ |
| **Tests** | — _(declared — not yet enforced)_ |
| **CI** | — _(declared — not yet enforced)_ |

#### `CN-META-03` — _declared_

| Aspect | Location |
|--------|----------|
| **Source** | classification_table.md §N |
| **Requirement** | Every equivalence claim must be reproducible from named fixtures, oracle versions, and replay inputs |
| **Code** | — _(declared — not yet enforced)_ |
| **Tests** | — _(declared — not yet enforced)_ |
| **CI** | — _(declared — not yet enforced)_ |

### CN-MITHRIL

#### `CN-MITHRIL-01` — _enforced_

| Aspect | Location |
|--------|----------|
| **Source** | docs/clusters/PHASE4-N-Y/S1-mithril-import-authority.md; S7-real-mithril-binding.md |
| **Requirement** | A Mithril-sourced seed may bootstrap only after a verified binding: the Mithril manifest's attested {network_magic, genesis_hash, certified_point, certificate_hash} is cross-checked against the INDEPENDENTLY-minted BootstrapAnchor (minted from the operator's --json-seed UTxO + genesis, a different origin than the Mithril cert), and fails closed on any field mismatch BEFORE storage initializes. The Mithril STM multisig is verified by the RED mithril-client (acquisition infra) and is NEVER a BLUE trust root; no mithril/STM crate is imported under any BLUE crate path. |
| **Code** | crates/ade_ledger/src/bootstrap_anchor/binding.rs (verify_mithril_binding + MithrilManifestReport + closed MithrilImportError); crates/ade_runtime/src/mithril_import/ (RED manifest importer); crates/ade_runtime/src/mithril_bootstrap.rs (PHASE4-N-Z production composition — verify-before-bootstrap, fail-closed) |
| **Tests** | `mithril_binding_rejects_certified_point_other_than_seed_point`; `mithril_anchor_rejects_field_mismatch`; `mithril_import_fail_closed_blocks_storage_init`; `mithril_bootstrap_verifies_before_storage_init`; `mithril_bootstrap_fails_closed_on_seed_point_mismatch` |
| **CI** | `ci/ci_check_mithril_uses_bootstrap_initial_state.sh`; `ci/ci_check_mithril_seed_point_independence.sh` |

### CN-NET

#### `CN-NET-01` — _declared_

| Aspect | Location |
|--------|----------|
| **Source** | classification_table.md §G |
| **Requirement** | A block producer must not accept arbitrary public peer connectivity; it may connect only through trusted relay topology |
| **Code** | — _(declared — not yet enforced)_ |
| **Tests** | — _(declared — not yet enforced)_ |
| **CI** | — _(declared — not yet enforced)_ |

#### `CN-NET-02` — _declared_

| Aspect | Location |
|--------|----------|
| **Source** | classification_table.md §G |
| **Requirement** | Relay paths must be geographically and topologically diverse enough that isolating one path does not prevent timely propagation |
| **Code** | — _(declared — not yet enforced)_ |
| **Tests** | — _(declared — not yet enforced)_ |
| **CI** | — _(declared — not yet enforced)_ |

#### `CN-NET-03` — _declared_

| Aspect | Location |
|--------|----------|
| **Source** | classification_table.md §G |
| **Requirement** | No single peer, ASN, region, or operator cluster may dominate the node's authoritative view |
| **Code** | — _(declared — not yet enforced)_ |
| **Tests** | — _(declared — not yet enforced)_ |
| **CI** | — _(declared — not yet enforced)_ |

#### `CN-NET-04` — _declared_

| Aspect | Location |
|--------|----------|
| **Source** | classification_table.md §G |
| **Requirement** | Peer selection and promotion policies must not allow one adversary-controlled set to deterministically starve honest views |
| **Code** | — _(declared — not yet enforced)_ |
| **Tests** | — _(declared — not yet enforced)_ |
| **CI** | — _(declared — not yet enforced)_ |

### CN-NODE

#### `CN-NODE-01` — _enforced_

| Aspect | Location |
|--------|----------|
| **Source** | docs/planning/phase4-n-k-orchestrator-binary-invariants.md §1 (I-2) |
| **Requirement** | Single bootstrap authority: exactly one pub fn in ade_runtime::bootstrap returns the initial (LedgerState, PraosChainDepState, ChainDb tip) at node startup. Cold-start (genesis-only) and warm-start (snapshot-resume + replay-forward) are two branches of the same function — never parallel paths. Type-level + CI grep enforcement, mirroring CN-STORE-07 / CN-STORE-08. |
| **Code** | crates/ade_runtime/src/bootstrap.rs |
| **Tests** | `crates/ade_runtime/src/bootstrap.rs::tests::bootstrap_cold_start_returns_genesis_when_empty`; `crates/ade_runtime/src/bootstrap.rs::tests::bootstrap_cold_start_without_genesis_errors`; `crates/ade_runtime/src/bootstrap.rs::tests::bootstrap_warm_start_materializes_from_persistent_snapshot`; `crates/ade_runtime/src/bootstrap.rs::tests::bootstrap_warm_start_equals_direct_materialize`; `crates/ade_runtime/src/bootstrap.rs::tests::bootstrap_two_runs_produce_byte_identical_state` |
| **CI** | `ci/ci_check_bootstrap_closure.sh`; `ci/ci_check_produce_mode_uses_bootstrap_initial_state.sh`; `ci/ci_check_node_mode_closure.sh` |

#### `CN-NODE-02` — _enforced_

| Aspect | Location |
|--------|----------|
| **Source** | docs/planning/phase4-n-f-d-live-node-run-loop-invariants.md |
| **Requirement** | `--mode node` is the single live-run lifecycle owner. The relay run loop may advance authoritative state ONLY by invoking existing closed seams (bootstrap_initial_state for initial state; run_node_sync -> pump_block for tip advance). It MUST NOT introduce any alternate bootstrap, apply, forge, evidence, or tip-advance path, and no second binary arm may drive the live node. Relay-only this cluster: no forge / evidence path is wired (those are a fenced successor sub-cluster). The GREEN loop planner emits only a closed lifecycle vocabulary { SyncOnce, Idle, HaltCleanly } and cannot express an authority decision. |
| **Code** | crates/ade_node/src/node_lifecycle.rs |
| **Tests** | `relay_loop_syncs_then_halts_clean_on_source_end`; `relay_loop_halts_clean_on_shutdown_no_partial_write`; `relay_loop_idles_then_syncs_on_incremental_feed`; `relay_loop_fails_closed_on_unapplyable_block`; `plan_loop_step_forge_precedence_table_is_total` |
| **CI** | `ci/ci_check_node_run_loop_containment.sh`; `ci/ci_check_loop_planner_closed.sh` |

#### `CN-NODE-03` — _enforced_

| Aspect | Location |
|--------|----------|
| **Source** | docs/planning/phase4-n-f-f-invariants.md |
| **Requirement** | Operator-key ingress + forge-on flip for --mode node. Ingress constructs an operator-material-backed ForgeActivation STRICTLY through RED-parse -> BLUE-structural-validator -> canonical-type, reusing the existing KES / VRF / cold / opcert loaders (KES via ade_crypto::kes_sum::Sum6Kes::raw_deserialize_signing_key_kes; VRF/cold via the cardano-cli text-envelope loaders; opcert via the opcert parser). NO new BLUE authority, NO parser reimplementation, NO plugin/trait seam, NO second forge codepath, NO new BLUE crate change. Key custody stays RED-confined to ProducerShell: passing ProducerShell to the fenced forge handoff (forge_one_from_recovered) is allowed, but copying or extracting its private material into the GREEN coordinator state, the planner, any node/loop state, or any persisted / logged / hashed-for-evidence / replay surface is forbidden. Tests and debug output MUST NOT print, snapshot, serialize, hash-for-evidence, or compare private key bytes (assertions may compare public identifiers, structured outcomes, and forged artifacts only where already produced by the fenced forge path). Forge intent is a pure total function of CLI key-flag presence: the COMPLETE required operator set { cold skey, KES skey, VRF skey, opcert, genesis file } present => Some(activation) (forge on); all absent => None (byte-identical N-F-D relay); any partial subset => structured fail-closed error (never a silent relay fallback, never a missing / zero / fabricated key). pool_id is either added to the required set or derived in one named place -- never fabricated. The forge base is the SAME recovered/bootstrap BootstrapState that seeds the relay spine, obtained via the single bootstrap_initial_state / warm-start authority (no second bootstrap, no second recovered state); the recovered state outlives both ForwardSyncState and ForgeActivation. The forge remains subordinate + self-accept-only: the existing N-F-E containment gate stays SEMANTICALLY UNCHANGED (still exactly one fenced forge_one_from_recovered call, no run_real_forge, no serve / admit / gossip / broadcast / block-fetch / durable-tip mutation); N-F-F may ADD key-ingress gates but MUST NOT relax forge containment. N-F-F makes the binary forge-CAPABLE once paired with a live/continuing feed; it does NOT itself make forge observable on the current empty-source binary path (plan_loop_step halts cleanly on LoopState::Ending even when a slot is Due) and makes NO live forge / serve / gossip / peer-acceptance / BA-02 / RO-LIVE / durable tip-advance claim -- observable forge is the RO-LIVE-01 follow-on. pparams / protocol_version reuse the existing produce-path honest-scope defaults (ProtocolParameters::default + default protocol_version): this is ingress / activation wiring, NOT mainnet-complete ledger-valid block-production fidelity. |
| **Code** | crates/ade_node/src/forge_intent.rs (GREEN tri-state classifier); crates/ade_node/src/operator_forge.rs (RED ingress + activation assembly); crates/ade_node/src/node_lifecycle.rs (Some/None binary flip + ForgeKeyIngress) |
| **Tests** | `classify_forge_intent_total_over_all_32_flag_combinations`; `classify_forge_intent_none_present_is_off`; `forge_intent_error_carries_no_path_bytes`; `load_operator_producer_shell_builds_shell_from_complete_material`; `load_operator_producer_shell_kes_period_past_opcert_fails_closed`; `operator_forge_error_carries_no_path_or_key_bytes`; `build_operator_forge_material_from_complete_material`; `node_mode_with_operator_keys_warm_start_forge_capable_halts_clean`; `node_mode_partial_operator_keys_fail_closed`; `relay_loop_with_operator_material_forge_reaches_fenced_path` … (+1 more) |
| **CI** | `ci/ci_check_forge_intent_closed.sh`; `ci/ci_check_operator_forge_no_secret_leak.sh`; `ci/ci_check_node_run_loop_containment.sh` |

#### `CN-NODE-04` — _enforced_

| Aspect | Location |
|--------|----------|
| **Source** | docs/planning/phase4-n-f-g-j-invariants.md |
| **Requirement** | --mode node emits a CLOSED, allow-listed diagnostic event vocabulary for feed/forge scheduling: feed_unavailable{reason} with a closed reason enum, forge_tick_considered, forge_tick_skipped{closed reason}, forge_attempted, and forge_result{closed outcome}. Closed reason/outcome enums only -- no stringly-typed authoritative errors and no catch-all/Other variant (a new variant is a compile error at the exhaustive JSONL encoder and fails the allow-list closedness test until wired + allow-listed). The S1-producible closed reason set from the current NodeBlockSource signals is exactly three: NoBlockAvailable (a WirePump open but momentarily empty) and CleanEmpty (an InMemory feed's provably-clean deterministic drain) are forge-eligible; UnknownDisconnected (a reason-less / ambiguous WirePump disconnect) is INELIGIBLE (fail-closed-on-ambiguity -- no ambiguous disconnect may become forge-eligible). The richer error reasons (PeerLost \| DecodeError \| ProtocolError \| SourceInvalid) and a reason-enriched live AtTip are a FUTURE wire-pump-enrichment prerequisite, NOT yet in the closed set. The events are operational/diagnostic ONLY: never a consensus-evidence, acceptance, or BA-02 signal, and emitting them changes no forge scheduling, base, or authority. CN-NODE-04 events are NEVER read by planner logic -- the planner may EMIT events but MUST NOT consume them (emit-only, one-directional planner -> log). Only the forge-eligible reasons (NoBlockAvailable \| CleanEmpty) may be eligible for the DC-NODE-08 forge allowance; the ineligible reasons are never eligible. |
| **Code** | crates/ade_node/src/node_sync.rs; crates/ade_node/src/node_lifecycle.rs; crates/ade_node/src/live_log/sched_event.rs; crates/ade_node/src/live_log/sched_writer.rs (closed event vocabulary + emit-only JSONL writer) |
| **Tests** | `node_sched_events_emit_closed_vocabulary`; `node_sched_event_allowlist_rejects_unknown_variants` |
| **CI** | `ci/ci_check_node_sched_events_emit_only.sh` |

### CN-OPCERT

#### `CN-OPCERT-01` — _enforced_

| Aspect | Location |
|--------|----------|
| **Source** | docs/planning/phase4-n-r-invariants.md §1 (I6); §2 (N6); §3 (D4); N-R-A A1 OQ4 fixtures |
| **Requirement** | The opcert envelope parser accepts a real cardano-cli `node.opcert` text envelope (closed type check `NodeOperationalCertificate` + CBOR array(2) shape locked by N-R-A A1 OQ4 fixtures against cardano-cli 11.0.0.0 / cardano-node 11.0.1). Element 0 is an array(4) of [hot_vkey(bytes(32)), sequence_number(uint), kes_period(uint), sigma(bytes(64))] mapping to the canonical `OperationalCert`. Element 1 is bytes(32) cold_vk. Any shape mismatch (wrong type field, malformed cborHex, wrong outer/inner arity, wrong field types or lengths) fail-closes with a structured `OpCertParseError` variant. No `String` payloads in load-bearing error variants. |
| **Code** | crates/ade_runtime/src/producer/opcert_envelope.rs (parse_opcert_envelope + closed OpCertParseError enum) |
| **Tests** | `accepted_envelope_decodes_to_expected_opcert`; `malformed_type_envelope_emits_wrong_envelope_type`; `malformed_cbor_hex_envelope_emits_malformed_cbor_hex`; `wrong_arity_envelope_emits_malformed_cbor`; `parser_is_byte_identical_across_two_runs` |
| **CI** | `ci/ci_check_node_forge_real_cli_ingress.sh` |

### CN-OPERATOR-EVIDENCE

#### `CN-OPERATOR-EVIDENCE-01` — _enforced_

| Aspect | Location |
|--------|----------|
| **Source** | docs/planning/phase4-n-s-invariants.md §1 (I6); §2 (N6, N7); user-direction correction 6 |
| **Requirement** | Every PHASE4-N-S-C operator-pass evidence manifest (docs/clusters/PHASE4-N-S-C/CE-N-S-LIVE_YYYYMMDD-<short_commit>.toml) carries the closed schema: schema_version, ade_commit, cardano_node_version, cardano_cli_version, network, block_hash, slot, opcert_fingerprint, genesis_fingerprint, ade_evidence_file, peer_log_file, peer_log_capture_command, peer_log_filter, peer_log_file_sha256, acceptance_keyword_match. The peer_log_file_sha256 cross-checks the committed peer.log file's actual hash. grep filter is documentation, not authority — the committed peer_log_file is the raw docker logs output. |
| **Code** | docs/clusters/PHASE4-N-S-C/cluster.md + S1.md + S2.md (runbook + manifest schema); ci/ci_check_operator_evidence_manifest_schema.sh (schema enforcement when a manifest is committed); PHASE4-N-F-G-C: docs/evidence/phase4-n-f-g-c-operator-pass-README.md (the --mode node operator-pass runbook) + ci/ci_check_ba02_evidence_manifest_schema.sh (the BA-02 manifest schema + sha256 cross-check, vacuous-until-committed — same manifest+sha256 discipline extended to the node-spine BA-02 evidence path) |
| **Tests** | _(no tests listed — gap)_ |
| **CI** | `ci/ci_check_operator_evidence_manifest_schema.sh`; `ci/ci_check_ba02_evidence_manifest_schema.sh` |

### CN-OPS

#### `CN-OPS-01` — _declared_

| Aspect | Location |
|--------|----------|
| **Source** | classification_table.md §M |
| **Requirement** | After any partition, authoritative post-incident reconciliation must be derived solely from the recovered canonical chain |
| **Code** | — _(declared — not yet enforced)_ |
| **Tests** | — _(declared — not yet enforced)_ |
| **CI** | — _(declared — not yet enforced)_ |

#### `CN-OPS-02` — _declared_

| Aspect | Location |
|--------|----------|
| **Source** | classification_table.md §M |
| **Requirement** | Emergency recovery procedures must have explicit admissibility criteria, deterministic inputs and outputs, and defined authority thresholds |
| **Code** | — _(declared — not yet enforced)_ |
| **Tests** | — _(declared — not yet enforced)_ |
| **CI** | — _(declared — not yet enforced)_ |

#### `CN-OPS-03` — _declared_

| Aspect | Location |
|--------|----------|
| **Source** | classification_table.md §M |
| **Requirement** | Incident evidence must be sufficient to reconstruct the canonical decision path without relying on nondeterministic logs or local operator memory |
| **Code** | — _(declared — not yet enforced)_ |
| **Tests** | — _(declared — not yet enforced)_ |
| **CI** | — _(declared — not yet enforced)_ |

### CN-OUTBOUND-RELAY

#### `CN-OUTBOUND-RELAY-01` — _enforced_

| Aspect | Location |
|--------|----------|
| **Source** | docs/planning/phase4-n-s-invariants.md §1 (I4); §2 (N4, N8); §3 (D3) |
| **Requirement** | OutboundCommand is the sole channel between produce_mode and MuxPump's outbound encoder. The closed enum carries typed ChainSyncServerMsg / BlockFetchServerMsg variants — no Vec<u8> byte tunnel; no direct MuxTransportHandle::outbound write from produce_mode. MuxPump's session-aware encoder is the only producer of wire-byte streams. |
| **Code** | crates/ade_runtime/src/network/outbound_command.rs (OutboundCommand closed enum); crates/ade_runtime/src/network/mux_pump.rs (MuxPump::outbound_relay field + handle_outbound_command + dispatch_outbound_frame); crates/ade_node/src/produce_mode.rs (dispatch_server_frame_event_to_outbound enqueues typed ServerReply via OutboundCommand) |
| **Tests** | `outbound_command_peer_accessor_returns_target_peer`; `outbound_command_carries_typed_reply_not_raw_bytes` |
| **CI** | `ci/ci_check_no_produce_mode_direct_transport_writes.sh` |

### CN-PEER-OUTBOUND-MAP

#### `CN-PEER-OUTBOUND-MAP-01` — _enforced_

| Aspect | Location |
|--------|----------|
| **Source** | docs/planning/phase4-n-s-invariants.md §1 (I5); §2 (N5) |
| **Requirement** | Per-peer outbound senders are owned by an Arc<RwLock<BTreeMap<PeerId, mpsc::Sender<OutboundCommand>>>>. Listener (run_per_peer_session) inserts on PeerConnected; MuxPump removes on emit_peer_disconnected. produce_mode looks up by PeerId and cannot fabricate senders. BTreeMap (not HashMap) for deterministic iteration order. Lookup failure is structured: DispatchError::{UnknownPeer, PeerOutboundMissing}. No cross-peer byte leakage is structurally possible — bytes destined for PeerId(a) reach the MuxPump task owning PeerId(a)'s TCP socket, never another peer's. |
| **Code** | crates/ade_runtime/src/network/outbound_command.rs (PerPeerOutbound type alias + new_per_peer_outbound constructor); crates/ade_runtime/src/network/n2n_listener.rs (run_per_peer_session inserts sender on PeerConnected); crates/ade_node/src/produce_mode.rs (DispatchError closed enum + dispatch_server_frame_event_to_outbound) |
| **Tests** | `outbound_command_peer_accessor_returns_target_peer` |
| **CI** | _(no CI script listed)_ |

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
| **Code** | — _(declared — not yet enforced)_ |
| **Tests** | — _(declared — not yet enforced)_ |
| **CI** | — _(declared — not yet enforced)_ |

#### `CN-PLUTUS-04` — _enforced_

| Aspect | Location |
|--------|----------|
| **Source** | classification_table.md §E |
| **Requirement** | No host-environment property may influence script results |
| **Code** | crates/ade_plutus/src/ (BLUE evaluator crate: passes only canonical inputs -- tx / resolved UTxOs / cost model / per-script budget / slot config, all parameters -- to the pinned aiken evaluator; the slot config is never read from the host) |
| **Tests** | `plutus_eval_is_deterministic` |
| **CI** | `ci/ci_check_plutus_eval_purity.sh` |

### CN-PREIMAGE-FIXTURE

#### `CN-PREIMAGE-FIXTURE-01` — _enforced_

| Aspect | Location |
|--------|----------|
| **Source** | docs/planning/phase4-n-s-invariants.md §7 OQ-S-A; N-S-A A1 fixture metadata |
| **Requirement** | For every block in ade_testkit::validity::corpus::ConwayValidityCorpus, Ade's unsigned_header_pre_image(...) (with inputs derived from decode_block(block_bytes).header_input) produces output byte-identical to decode_block(block_bytes).header_input.kes.unwrap().header_body_bytes. This cross-impl byte-match test is the load-bearing proof that the producer's pre-image recipe matches the validator's authority — without it the 'single source of truth' claim is unverified. |
| **Code** | crates/ade_ledger/src/block_validity/unsigned_header_pre_image.rs (unsigned_header_preimage_matches_decode_block_extraction_for_corpus test) |
| **Tests** | `unsigned_header_preimage_matches_decode_block_extraction_for_corpus` |
| **CI** | _(no CI script listed)_ |

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
| **Tests** | `init_emits_started_event_and_zero_other_effects`; `slot_tick_emits_request_forge_and_log`; `forge_succeeded_emits_broadcast_and_log`; `forge_not_leader_emits_log_and_clears_pending`; `forge_failed_emits_slot_missed_with_mapped_reason`; `stale_forge_result_after_new_tick_drops_with_slot_missed`; `kes_period_out_of_range_errors`; `shell_init_rejects_malformed_opcert_hot_vkey_length`; `shell_init_rejects_kes_period_below_opcert_start`; `shell_kes_sign_at_current_period_succeeds_and_verifies` … (+6 more) |
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
| **CI** | _(no CI script listed)_ |

### CN-PROTO

#### `CN-PROTO-01` — _declared_

| Aspect | Location |
|--------|----------|
| **Source** | classification_table.md §F |
| **Requirement** | Each miniprotocol must be an explicit deterministic state machine with legal typed transitions only |
| **Code** | — _(declared — not yet enforced)_ |
| **Tests** | — _(declared — not yet enforced)_ |
| **CI** | — _(declared — not yet enforced)_ |

#### `CN-PROTO-02` — _declared_

| Aspect | Location |
|--------|----------|
| **Source** | classification_table.md §F |
| **Requirement** | For the same peer transcript, authoritative state and outbound transcript must be identical |
| **Code** | — _(declared — not yet enforced)_ |
| **Tests** | — _(declared — not yet enforced)_ |
| **CI** | — _(declared — not yet enforced)_ |

#### `CN-PROTO-03` — _declared_

| Aspect | Location |
|--------|----------|
| **Source** | classification_table.md §F |
| **Requirement** | Agency must be enforced strictly; impossible messages must fail deterministically |
| **Code** | — _(declared — not yet enforced)_ |
| **Tests** | — _(declared — not yet enforced)_ |
| **CI** | — _(declared — not yet enforced)_ |

#### `CN-PROTO-04` — _declared_

| Aspect | Location |
|--------|----------|
| **Source** | classification_table.md §F |
| **Requirement** | Socket fragmentation, multiplexing, arrival order, and timeout behavior must not leak nondeterminism into authoritative logic |
| **Code** | — _(declared — not yet enforced)_ |
| **Tests** | — _(declared — not yet enforced)_ |
| **CI** | — _(declared — not yet enforced)_ |

#### `CN-PROTO-05` — _declared_

| Aspect | Location |
|--------|----------|
| **Source** | classification_table.md §F |
| **Requirement** | Untrusted network inputs must not allocate unbounded authoritative resources before deterministic validation |
| **Code** | — _(declared — not yet enforced)_ |
| **Tests** | — _(declared — not yet enforced)_ |
| **CI** | — _(declared — not yet enforced)_ |

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
| **Requirement** | Receive-side agency closure: the receive bridge consumes only peer-originated ForkChoiceSignal and BatchDeliveryEvent values valid for the client-role N2N receive surface. Constructing or admitting locally-originated / client-output events into the receive reducer is unrepresentable in the public API. |
| **Code** | crates/ade_ledger/src/receive/events.rs (ReceiveEvent closed sum: only RollForward, RollBackward, BlockDelivered variants — no constructor for RequestNext, RequestRange, ClientDone, FindIntersect) |
| **Tests** | `receive_event_round_trips_through_pattern_match`; `receive_effect_round_trips_through_pattern_match`; `receive_error_round_trips_through_pattern_match` |
| **CI** | `ci/ci_check_admitted_block_closure.sh` |

### CN-PUMP

#### `CN-PUMP-01` — _enforced_

| Aspect | Location |
|--------|----------|
| **Source** | docs/planning/phase4-n-m-c-operator-pass-invariants.md §1 (I-C5) |
| **Requirement** | Single admission wire-pump entry per peer: exactly one pub async fn ade_runtime::admission::wire_pump::run_admission_wire_pump drives the per-peer pump that produces AdmissionPeerEvents. The pump owns the MuxTransportHandle and runs the chain-sync + block-fetch state machine. The pump is the SOLE producer on the runner's peer_events channel. No second pump path; no per-call fallback. |
| **Code** | crates/ade_runtime/src/admission/wire_pump.rs (run_admission_wire_pump sole authority + closed AdmissionPeerEvent / AdmissionWirePumpError sums + extract_chain_sync_header_point header-point extractor) |
| **Tests** | `admission::wire_pump::tests::pump_emits_tip_update_and_request_next_on_intersect_found_no_block_fetch`; `admission::wire_pump::tests::pump_emits_tip_update_on_intersect_not_found`; `admission::wire_pump::tests::rollforward_drives_block_fetch_then_request_next`; `admission::wire_pump::tests::extract_chain_sync_header_point_returns_slot_and_hash`; `admission::wire_pump::tests::extract_chain_sync_header_point_rejects_malformed_envelope` |
| **CI** | `ci/ci_check_admission_wire_pump_closure.sh` |

### CN-REHEARSAL-FIDELITY

#### `CN-REHEARSAL-FIDELITY-01` — _enforced_

| Aspect | Location |
|--------|----------|
| **Source** | docs/planning/phase4-n-f-g-d-invariants.md |
| **Requirement** | Private-testnet accepted-block bounty dry-run fidelity (two coupled clauses; if either fails the rehearsal becomes misleading). (1) PATH FIDELITY: the C1 private dry-run uses the SAME --mode node accepted-block path as preview/preprod -- N-M-C extraction/import (import_live_consensus_inputs) -> forge -> self-accept -> sibling-serve -> block-fetch -> peer log -> correlate -- with NO private-only flag, branch, bootstrap authority, or from-genesis constructor. The only differences from the preprod pass are operator-controlled inputs (a private genesis whose stake allocation makes Ade win slots fast) and the evidence label (rehearsal). No private-only helper may make the rehearsal pass if the same condition would fail on preview/preprod -- such a condition is a shared-path bug to fix in the shared path, never special-cased. (2) EVIDENCE NON-PROMOTABILITY: any private-testnet manifest is clearly marked rehearsal / private-testnet, stored ONLY under the rehearsal home (docs/evidence/phase4-n-f-g-d-private-rehearsal-*.toml, never the bounty home docs/clusters/PHASE4-N-F-G-C/CE-G-C-LIVE_*.toml), sha256-bound to a real Haskell peer log, correlate-produced (ba02_evidence::correlate is the sole acceptance-evidence constructor; allow-list, hash-primary), and flips NO RO-LIVE rule. C1 rehearsal evidence may increase confidence in the bounty path, but it is NOT bounty evidence and MUST NOT be referenced by the CE-G-C-LIVE_* / ci_check_ba02_evidence_manifest_schema.sh gate. The single bounty deliverable is preview/preprod acceptance, captured separately. |
| **Code** | crates/ade_node/src/ba02_evidence.rs (correlate — reused, the sole acceptance-evidence constructor); crates/ade_node/src/ba02_pass.rs (RED evidence I/O — the rehearsal write-wrapper mirrors/extends this into the rehearsal home); crates/ade_runtime/src/consensus_inputs/ (import_live_consensus_inputs — the shared N-M-C extraction/import the path-fidelity clause pins; OQ1 slice-entry proof obligation: it must consume an early/private-net extraction through the SAME path used for a synced preprod tip); docs/evidence/phase4-n-f-g-c-operator-pass-README.md (the preprod operator-pass runbook the C1 dry-run runbook must be a strict subset of); docs/evidence/phase4-n-f-g-d-private-rehearsal-*.toml (planned — the distinct rehearsal-evidence home); ci/ci_check_rehearsal_manifest_schema.sh (planned — vacuous-until-committed schema + sha256 + label + distinct-home gate); ci/ci_check_node_path_fidelity.sh (planned — no new --mode node flag, no from-genesis consensus-inputs constructor); crates/ade_node/tests/forge_succeeds.rs (S5 genesis-rehearsal mechanics — genesis_rehearsal_manifest_binds_block_zero_genesis + _no_evidence, reusing EligibleFixture's self-accepting block 0 + Genesis); crates/ade_node/tests/node_c1_genesis_rehearsal.rs (S5 env-gated genesis live arm); docs/evidence/phase4-n-f-g-j-genesis-rehearsal-README.md + phase4-n-f-g-j-genesis-rehearsal-*.toml (S5 genesis-successor rehearsal runbook + home, covered by ci_check_rehearsal_manifest_schema.sh); ci/ci_check_genesis_successor_reachability.sh (S5 reuses — the cold-start path the genesis rehearsal exercises) |
| **Tests** | `node_accepted_block_consensus_inputs_via_shared_import`; `rehearsal_envelope_wraps_correlate_produced_payload`; `rehearsal_correlate_no_evidence_writes_nothing`; `rehearsal_envelope_is_structurally_distinct_from_ba02_manifest`; `c1_dry_run_correlate_to_rehearsal_envelope`; `node_c1_dry_run_rehearsal_live`; `rehearsal_gate_fails_on_archived_home_leak`; `genesis_rehearsal_manifest_binds_block_zero_genesis`; `genesis_rehearsal_no_evidence_writes_nothing`; `node_c1_genesis_rehearsal_live` |
| **CI** | `ci/ci_check_node_path_fidelity.sh`; `ci/ci_check_rehearsal_manifest_schema.sh`; `ci/ci_check_genesis_successor_reachability.sh` |

### CN-REL

#### `CN-REL-01` — _declared_

| Aspect | Location |
|--------|----------|
| **Source** | classification_table.md §L |
| **Requirement** | A release is not mainnet-eligible unless mixed-version topologies against supported predecessors show consensus equivalence on malformed and boundary-case inputs |
| **Code** | — _(declared — not yet enforced)_ |
| **Tests** | — _(declared — not yet enforced)_ |
| **CI** | — _(declared — not yet enforced)_ |

#### `CN-REL-02` — _declared_

| Aspect | Location |
|--------|----------|
| **Source** | classification_table.md §L |
| **Requirement** | No single implementation bug should exceed the protocol's intended safety or liveness fault threshold at ecosystem level |
| **Code** | — _(declared — not yet enforced)_ |
| **Tests** | — _(declared — not yet enforced)_ |
| **CI** | — _(declared — not yet enforced)_ |

#### `CN-REL-03` — _declared_

| Aspect | Location |
|--------|----------|
| **Source** | classification_table.md §L |
| **Requirement** | Cross-implementation accept/reject agreement on authoritative corpora is release-blocking |
| **Code** | — _(declared — not yet enforced)_ |
| **Tests** | — _(declared — not yet enforced)_ |
| **CI** | — _(declared — not yet enforced)_ |

### CN-SEED

#### `CN-SEED-01` — _enforced_

| Aspect | Location |
|--------|----------|
| **Source** | docs/planning/phase4-n-m-ledger-seed-invariants.md §1 (I-A1) |
| **Requirement** | Single JSON seed-importer authority: exactly one pub fn in ade_runtime::seed_import::import_cardano_cli_json_utxo converts a cardano-cli `query utxo --whole-utxo` JSON dump into the Ade canonical (UTxOState, UtxoFingerprint) pair. No second importer; no per-call fallback; no silent partial seed. |
| **Code** | crates/ade_runtime/src/seed_import/importer.rs, crates/ade_runtime/src/seed_import/json.rs |
| **Tests** | `crates/ade_runtime/src/seed_import/importer.rs::tests::utxo_seed_parses_minimal_two_entry_fixture`; `crates/ade_runtime/src/seed_import/importer.rs::tests::utxo_seed_two_imports_byte_identical`; `crates/ade_runtime/src/seed_import/importer.rs::tests::utxo_seed_btree_order_independent_of_json_order`; `crates/ade_runtime/src/seed_import/importer.rs::tests::utxo_seed_rejects_unparseable_json`; `crates/ade_runtime/src/seed_import/importer.rs::tests::utxo_seed_rejects_bad_txin_key`; `crates/ade_runtime/src/seed_import/importer.rs::tests::utxo_seed_rejects_bad_address`; `crates/ade_runtime/src/seed_import/importer.rs::tests::utxo_seed_inline_datum_entry_round_trips`; `crates/ade_runtime/src/seed_import/importer.rs::tests::utxo_seed_canonical_txout_address_extracted`; `crates/ade_runtime/src/seed_import/importer.rs::tests::utxo_seed_accepts_plutus_v1_reference_script`; `crates/ade_runtime/src/seed_import/importer.rs::tests::utxo_seed_accepts_plutus_v2_reference_script` … (+9 more) |
| **CI** | `ci/ci_check_seed_import_closure.sh`; `ci/ci_check_seed_import_full_preprod_support.sh` |

### CN-SESS

#### `CN-SESS-01` — _enforced_

| Aspect | Location |
|--------|----------|
| **Source** | docs/planning/phase4-n-l-wire-protocol-invariants.md §1 (I-7) |
| **Requirement** | Single mux frame authority: ade_network::mux::frame::{encode_frame, decode_frame} is the SOLE pub fn pair encoding/decoding `MuxFrame` to/from bytes in the workspace. No parallel mux codecs. Type-level + CI grep enforcement (mirrors CN-STORE-08). |
| **Code** | crates/ade_network/src/mux/frame.rs |
| **Tests** | `crates/ade_network/src/mux/frame.rs::tests::frame_roundtrip_byte_identical` |
| **CI** | `ci/ci_check_mux_frame_closure.sh` |

#### `CN-SESS-02` — _enforced_

| Aspect | Location |
|--------|----------|
| **Source** | docs/planning/phase4-n-l-wire-protocol-invariants.md §1 (I-1) |
| **Requirement** | Single handshake authority: ade_network::handshake::n2n_transition is the SOLE pub fn driving the N2N handshake state machine. ade_network::handshake::n2c_transition is the SOLE pub fn driving the N2C handshake state machine. No parallel handshake reducers. Type-level + CI grep enforcement. |
| **Code** | crates/ade_network/src/handshake/transition.rs |
| **Tests** | `crates/ade_network/src/session/handshake_driver.rs::tests::handshake_initiator_accepts_when_responder_supports_proposed_version`; `crates/ade_network/src/session/handshake_driver.rs::tests::handshake_initiator_rejects_on_no_overlap` |
| **CI** | `ci/ci_check_handshake_closure.sh` |

#### `CN-SESS-03` — _enforced_

| Aspect | Location |
|--------|----------|
| **Source** | docs/planning/phase4-n-l-wire-protocol-invariants.md §5 |
| **Requirement** | Single session step authority: ade_network::session::core::step is the SOLE pub fn reducing (SessionState, ByteChunkIn) -> (SessionState, Vec<SessionEffect>). No parallel session reducers. Mirrors CN-NODE-01 (single bootstrap) and CN-STORE-07 (single materialize) for the wire-session layer. |
| **Code** | crates/ade_network/src/session/core.rs |
| **Tests** | `crates/ade_network/src/session/core.rs::tests::session_step_two_runs_byte_identical`; `crates/ade_network/src/session/core.rs::tests::session_handshake_completion_transitions_state`; `crates/ade_network/src/session/core.rs::tests::session_outbound_frame_encodes_via_encode_frame` |
| **CI** | `ci/ci_check_session_core_closure.sh` |

#### `CN-SESS-04` — _enforced_

| Aspect | Location |
|--------|----------|
| **Source** | docs/clusters/PHASE4-N-M-FRAG/cluster.md §1 |
| **Requirement** | Session reducer per-mini-protocol payload reassembly: the GREEN session reducer maintains one accumulating Vec<u8> buffer per AcceptedMiniProtocol variant inside `ConnectedState`, and emits exactly one `SessionEffect::DeliverPeerFrame` per COMPLETE CBOR item observed in the wire stream — never per mux frame. Same canonical inbound byte stream → byte-identical `DeliverPeerFrame` sequence across runs. The per-protocol buffer is a CLOSED-sum- indexed struct (no HashMap), so iteration / membership / lookup are deterministic. |
| **Code** | crates/ade_network/src/session/state.rs (ProtoBuffers + ConnectedState.proto_buffers field), crates/ade_network/src/session/core.rs (drain_connected_frames + drain_protocol_items) |
| **Tests** | `crates/ade_network/src/session/core.rs::tests::fragmented_chain_sync_message_assembles_one_deliver`; `crates/ade_network/src/session/core.rs::tests::fragmented_block_fetch_block_assembles_one_deliver`; `crates/ade_network/src/session/core.rs::tests::interleaved_chain_sync_and_block_fetch_fragments_stay_isolated`; `crates/ade_network/src/session/core.rs::tests::pipelined_two_chain_sync_messages_in_one_mux_frame_emit_two_delivers`; `crates/ade_network/src/session/core.rs::tests::truncated_then_complete_two_step_drain`; `crates/ade_network/src/session/core.rs::tests::proto_buffers_isolation_across_accepted_protocols` |
| **CI** | `ci/ci_check_session_proto_reassembly.sh` |

#### `CN-SESS-05` — _enforced_

| Aspect | Location |
|--------|----------|
| **Source** | docs/clusters/PHASE4-N-AB/cluster.md §1; project_pre_rolive_hardening_queue.md item 2 |
| **Requirement** | Outbound mini-protocol payloads larger than MAX_PAYLOAD are segmented into ordered mux frames, each no larger than MAX_PAYLOAD, preserving mini-protocol id, mode, and byte order. Concatenating segment payloads reconstructs the original payload exactly. Payloads above MAX_OUTBOUND_PAYLOAD_BYTES fail closed. Segmentation uses the single existing frame encoder authority and is the outbound inverse of CN-SESS-04 inbound reassembly. |
| **Code** | crates/ade_network/src/session/core.rs (handle_outbound owns segmentation: splits MAX_PAYLOAD < len <= MAX_OUTBOUND_PAYLOAD_BYTES into ordered <=MAX_PAYLOAD frames via encode_inner_frame, reusing one captured timestamp; encode_inner_frame stays the single-frame encoder authority with its MAX_PAYLOAD guard; MAX_OUTBOUND_PAYLOAD_BYTES fixed constant) |
| **Tests** | `crates/ade_network/src/session/core.rs::tests::outbound_payload_at_max_payload_is_one_frame`; `crates/ade_network/src/session/core.rs::tests::outbound_payload_over_max_payload_segments_into_two`; `crates/ade_network/src/session/core.rs::tests::outbound_segment_order_preserved`; `crates/ade_network/src/session/core.rs::tests::outbound_segments_keep_same_mini_protocol_id_and_mode`; `crates/ade_network/src/session/core.rs::tests::outbound_large_payload_reassembles_byte_identical_via_inbound`; `crates/ade_network/src/session/core.rs::tests::outbound_payload_at_upper_bound_is_allowed`; `crates/ade_network/src/session/core.rs::tests::outbound_payload_over_upper_bound_fails_closed` |
| **CI** | `ci/ci_check_outbound_segmentation.sh` |

### CN-SNAPSHOT

#### `CN-SNAPSHOT-01` — _declared_

| Aspect | Location |
|--------|----------|
| **Source** | docs/planning/phase4-n-r-invariants.md §1 (I4); §2 (N11); §3 (D6) |
| **Requirement** | A forged block becomes visible to peers only AFTER ServedChainHandle::push_atomic succeeds. The push_atomic call covers the full served_chain_admit call inside a watch::Sender::send_modify closure — no observer can read a torn snapshot mid-insertion. Coordinator emits BroadcastBlock; RED effect handler calls push_atomic; only on Ok(ServedTip) may per-peer reducers serve the block. Fail-closed shutdown on PushError. |
| **Code** | TBD — populated by N-R-B B2 slice (ade_runtime::producer::served_chain_handle::ServedChainHandle::push_atomic) + B3 slice (produce_mode BroadcastBlock arm wiring) |
| **Tests** | `handle_construction_yields_empty_snapshot`; `view_subscribe_creates_independent_receiver`; `served_tip_is_closed_value_type` |
| **CI** | — _(declared — not yet enforced)_ |

#### `CN-SNAPSHOT-02` — _enforced_

| Aspect | Location |
|--------|----------|
| **Source** | docs/planning/phase4-n-r-invariants.md §2 (N4); N-R-A A1 OQ8 audit |
| **Requirement** | A RequestRange covering a slot range that is not entirely present in ServedChainSnapshot MUST return NoBlocks per the Cardano block-fetch protocol's failure semantics — no partial ad-hoc response, no silent truncation, no serving of a strict prefix. Both endpoints + every block between MUST be present in the snapshot for the server to issue StartBatch + Block* + BatchDone. |
| **Code** | crates/ade_network/src/block_fetch/server.rs (producer_block_fetch_serve — endpoint-presence check via first_key/last_key comparison against requested range) |
| **Tests** | `n_r_b_partial_overlap_from_endpoint_not_in_snapshot_yields_no_blocks`; `n_r_b_partial_overlap_to_endpoint_not_in_snapshot_yields_no_blocks`; `n_r_b_partial_overlap_both_endpoints_fabricated_yields_no_blocks`; `producer_block_fetch_serve_request_range_with_origin_endpoint_yields_no_blocks`; `producer_block_fetch_serve_request_range_empty_in_chain_yields_no_blocks`; `producer_block_fetch_serve_request_range_outside_chain_yields_no_blocks` |
| **CI** | _(no CI script listed)_ |

### CN-STORE

#### `CN-STORE-01` — _declared_

| Aspect | Location |
|--------|----------|
| **Source** | classification_table.md §I |
| **Requirement** | No authoritative storage initialization may occur before bootstrap or anchor verification succeeds |
| **Code** | — _(declared — not yet enforced)_ |
| **Tests** | — _(declared — not yet enforced)_ |
| **CI** | — _(declared — not yet enforced)_ |

#### `CN-STORE-02` — _partial_

| Aspect | Location |
|--------|----------|
| **Source** | classification_table.md §I |
| **Requirement** | WAL entries, checkpoints, and recovered artifacts must be bound to exactly one anchor or bootstrap lineage |
| **Code** | crates/ade_node/src/node_lifecycle.rs (warm_start_recovery: anchor-lineage discovery + fail-closed on multiple/mismatched anchors) |
| **Tests** | `warm_start_fails_closed_on_multiple_anchor_lineages`; `warm_start_fails_closed_on_anchor_mismatch`; `warm_start_fails_closed_on_duplicate_provenance` |
| **CI** | _(no CI script listed)_ |

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
| **Code** | — _(declared — not yet enforced)_ |
| **Tests** | — _(declared — not yet enforced)_ |
| **CI** | — _(declared — not yet enforced)_ |

#### `CN-STORE-07` — _enforced_

| Aspect | Location |
|--------|----------|
| **Source** | docs/planning/ledger-snapshot-rollback-invariants.md §1 (I-5) |
| **Requirement** | Single materialize authority for rolled-back state: the function that materializes (LedgerState, PraosChainDepState) at a target point uses ONLY one SnapshotStore lookup + ChainDb::iter_from_slot + apply_block_with_verdicts (+ apply_epoch_boundary when crossing). No bypass; no parallel rolled-back-state computation path. Mirror of CN-CONS-08 (admission gate) for the rollback path. Single- public-function discipline; type-level + CI grep enforcement. |
| **Code** | crates/ade_ledger/src/rollback/materialize.rs (materialize_rolled_back_state — the sole pub fn returning (LedgerState, PraosChainDepState) in the rollback module tree); crates/ade_ledger/src/rollback/traits.rs (SnapshotReader + BlockSource narrow read-only traits — production impls in ade_runtime go through the same single composition) |
| **Tests** | `materialize_returns_rollback_too_deep_when_no_snapshot`; `materialize_with_snapshot_at_target_returns_snapshot_state`; `materialize_with_snapshot_below_target_replays_forward`; `materialize_fails_closed_on_invalid_block`; `materialize_replay_forward_equals_direct_apply` |
| **CI** | `ci/ci_check_rollback_materialize_closure.sh` |

#### `CN-STORE-08` — _enforced_

| Aspect | Location |
|--------|----------|
| **Source** | docs/planning/persistent-snapshot-encoder-invariants.md §1 (I-5) |
| **Requirement** | Single encoder authority: encode_ledger_state + decode_ledger_state + encode_chain_dep + decode_chain_dep + encode_snapshot + decode_snapshot are the SOLE pub fn pairs in the project encoding or decoding LedgerState / PraosChainDepState / (LedgerState, PraosChainDepState) to/from bytes. No parallel canonical encoders. Type-level + CI grep enforcement, mirroring CN-STORE-07. |
| **Code** | crates/ade_ledger/src/snapshot/{ledger,chain_dep,framing}.rs |
| **Tests** | _(no tests listed — gap)_ |
| **CI** | `ci/ci_check_snapshot_encoder_closure.sh` |

### CN-TEST

#### `CN-TEST-01` — _declared_

| Aspect | Location |
|--------|----------|
| **Source** | classification_table.md §L |
| **Requirement** | Consensus-relevant inputs must be fuzzed differentially across all supported versions and decode or validation paths; any verdict mismatch is release-blocking |
| **Code** | — _(declared — not yet enforced)_ |
| **Tests** | — _(declared — not yet enforced)_ |
| **CI** | — _(declared — not yet enforced)_ |

#### `CN-TEST-02` — _declared_

| Aspect | Location |
|--------|----------|
| **Source** | classification_table.md §L |
| **Requirement** | Every malformed or discrepant input that ever triggered a fork, preview mismatch, or parser disagreement becomes a permanent regression corpus entry |
| **Code** | — _(declared — not yet enforced)_ |
| **Tests** | — _(declared — not yet enforced)_ |
| **CI** | — _(declared — not yet enforced)_ |

#### `CN-TEST-03` — _declared_

| Aspect | Location |
|--------|----------|
| **Source** | classification_table.md §L |
| **Requirement** | Previously failed, duplicate, or boundary-case inputs must remain verdict-stable under resubmission and replay |
| **Code** | — _(declared — not yet enforced)_ |
| **Tests** | — _(declared — not yet enforced)_ |
| **CI** | — _(declared — not yet enforced)_ |

### CN-WAL

#### `CN-WAL-01` — _enforced_

| Aspect | Location |
|--------|----------|
| **Source** | docs/planning/phase4-n-m-ledger-seed-invariants.md §1 (I-A4) |
| **Requirement** | Single WAL append authority: WalStore::append is the SOLE mutation method on any WalStore impl. No truncate/rewrite/replace method exists on the trait or any impl. Append-only by type, not by convention. |
| **Code** | crates/ade_ledger/src/wal/store_trait.rs, crates/ade_runtime/src/wal/file_wal_store.rs |
| **Tests** | `crates/ade_runtime/src/wal/file_wal_store.rs::tests::file_wal_store_append_then_read_all_round_trips`; `crates/ade_runtime/src/wal/file_wal_store.rs::tests::file_wal_store_reopens_existing_directory_and_preserves_entries`; `crates/ade_runtime/src/wal/file_wal_store.rs::tests::file_wal_store_rotates_at_max_bytes_when_forced` |
| **CI** | `ci/ci_check_wal_append_only.sh` |

### CN-WIRE

#### `CN-WIRE-01` — _declared_

| Aspect | Location |
|--------|----------|
| **Source** | classification_table.md §A |
| **Requirement** | Hash-critical original bytes must be preserved and used on all hash/signature-critical paths |
| **Code** | — _(declared — not yet enforced)_ |
| **Tests** | — _(declared — not yet enforced)_ |
| **CI** | — _(declared — not yet enforced)_ |

#### `CN-WIRE-02` — _declared_

| Aspect | Location |
|--------|----------|
| **Source** | classification_table.md §A |
| **Requirement** | Internal replay/state surfaces must use exactly one canonical project encoding |
| **Code** | — _(declared — not yet enforced)_ |
| **Tests** | — _(declared — not yet enforced)_ |
| **CI** | — _(declared — not yet enforced)_ |

#### `CN-WIRE-03` — _declared_

| Aspect | Location |
|--------|----------|
| **Source** | classification_table.md §A |
| **Requirement** | Consensus-critical deserialization must be equivalent across all supported versions and active code paths |
| **Code** | — _(declared — not yet enforced)_ |
| **Tests** | — _(declared — not yet enforced)_ |
| **CI** | — _(declared — not yet enforced)_ |

#### `CN-WIRE-04` — _declared_

| Aspect | Location |
|--------|----------|
| **Source** | classification_table.md §A |
| **Requirement** | Malformed consensus-relevant inputs must be rejected deterministically before any authoritative state transition |
| **Code** | — _(declared — not yet enforced)_ |
| **Tests** | — _(declared — not yet enforced)_ |
| **CI** | — _(declared — not yet enforced)_ |

#### `CN-WIRE-05` — _declared_

| Aspect | Location |
|--------|----------|
| **Source** | classification_table.md §A |
| **Requirement** | No legacy or compatibility parser may accept bytes that the canonical parser rejects |
| **Code** | — _(declared — not yet enforced)_ |
| **Tests** | — _(declared — not yet enforced)_ |
| **CI** | — _(declared — not yet enforced)_ |

#### `CN-WIRE-06` — _declared_

| Aspect | Location |
|--------|----------|
| **Source** | classification_table.md §A |
| **Requirement** | Every network/storage ingress path must pass through named era-aware decode chokepoints |
| **Code** | — _(declared — not yet enforced)_ |
| **Tests** | — _(declared — not yet enforced)_ |
| **CI** | — _(declared — not yet enforced)_ |

#### `CN-WIRE-07` — _enforced_

| Aspect | Location |
|--------|----------|
| **Source** | classification_table.md §A |
| **Requirement** | Each protocol-visible message must decode into one closed, versioned message type |
| **Code** | crates/ade_network/src/codec/error.rs, crates/ade_network/src/codec/version.rs, crates/ade_network/src/codec/handshake.rs, crates/ade_network/src/codec/n2c_handshake.rs, crates/ade_network/src/codec/chain_sync.rs, crates/ade_network/src/codec/block_fetch.rs, crates/ade_network/src/codec/tx_submission.rs, crates/ade_network/src/codec/keep_alive.rs, crates/ade_network/src/codec/peer_sharing.rs, crates/ade_network/src/codec/local_chain_sync.rs, crates/ade_network/src/codec/local_tx_submission.rs, crates/ade_network/src/codec/local_state_query.rs, crates/ade_network/src/codec/local_tx_monitor.rs |
| **Tests** | `codec::handshake::tests::roundtrip_every_variant`; `codec::handshake::tests::decode_rejects_unknown_tag`; `codec::handshake::tests::decode_rejects_truncated_input`; `codec::handshake::tests::decode_rejects_invalid_utf8_in_text_fields`; `codec::n2c_handshake::tests::roundtrip_every_variant`; `codec::n2c_handshake::tests::decode_rejects_unknown_tag`; `codec::n2c_handshake::tests::decode_rejects_truncated_input`; `codec::n2c_handshake::tests::decode_rejects_invalid_utf8_in_text_fields`; `codec::chain_sync::tests::roundtrip_every_variant`; `codec::chain_sync::tests::decode_rejects_unknown_tag` … (+35 more) |
| **CI** | `ci/ci_check_codec_message_closed.sh` |

#### `CN-WIRE-08` — _enforced_

| Aspect | Location |
|--------|----------|
| **Source** | docs/planning/phase4-n-x-invariants.md |
| **Requirement** | N2N tag-24 CBOR-in-CBOR payload envelopes are constructed and stripped through ONE shared BLUE byte authority in ade_codec (wrap_tag24/unwrap_tag24). Protocol-specific composition lives in ade_network BLUE codecs: a served BlockFetch MsgBlock payload is tag24(bytes([era,block])) (era inside the wrap); a served ChainSync RollForward header is [era_tag, tag24(bytes(header_cbor))] (era_tag outside the wrap). Both compositions are pinned byte-identically against captured cardano-node 11.0.1 wire fixtures, not codec comments. No bare [era,block] may be served over BlockFetch and no bare header over ChainSync RollForward. No hand-rolled tag-24 parse may exist in RED — admission and interop call the shared authority. unwrap_tag24 fails closed (typed error) on non-(0xd8 0x18), wrong inner length, or trailing bytes. The inner bytes are copied verbatim (no re-encode). |
| **Code** | crates/ade_codec/src/cbor/tag24.rs (wrap_tag24/unwrap_tag24 + TagEnvelopeError); crates/ade_codec/src/cbor/mod.rs (read_bytes/read_text/skip_item length-arg overflow guard — fail-closed, no panic); crates/ade_network/src/codec/block_fetch.rs + chain_sync.rs (per-protocol compose/decompose); crates/ade_network/src/block_fetch/server.rs + chain_sync/server.rs (serve emits composed bytes); crates/ade_node/src/admission/runner.rs + crates/ade_core_interop/src/follow.rs (RED unwraps migrated onto the shared authority) |
| **Tests** | `wrap_then_unwrap_is_identity_across_length_classes`; `wrap_emits_canonical_tag24_marker_and_length`; `unwrap_returns_zero_copy_borrow_of_input`; `unwrap_rejects_missing_tag24_marker`; `unwrap_rejects_non_byte_string_payload`; `unwrap_rejects_truncated_inner`; `unwrap_rejects_trailing_bytes`; `inner_bytes_are_verbatim_not_reencoded`; `unwrap_rejects_huge_declared_length_without_panic`; `read_bytes_rejects_overflowing_declared_length` … (+14 more) |
| **CI** | `ci/ci_check_tag24_wire_authority.sh` |

#### `CN-WIRE-09` — _enforced_

| Aspect | Location |
|--------|----------|
| **Source** | docs/planning/phase4-n-f-g-j-genesis-successor-prevhash-invariants.md |
| **Requirement** | The Shelley-and-later header_body `prev_hash` field is the closed wire grammar `$hash32 / null` (cardano-ledger PrevHash = GenesisHash \| BlockHash). Ade represents it as the closed sum PrevHash = Genesis \| Block(Hash32) -- never a flat Hash32. PrevHash::Genesis encodes to / decodes from CBOR null (0xf6); PrevHash::Block(h) encodes to / decodes from a 32-byte hash32. A genesis-successor block (block_number 0 on a from-genesis chain) MUST carry PrevHash::Genesis/null; a non-genesis block (block_number > 0) MUST carry PrevHash::Block(hash32). Encoding is canonical and round-trips through ONE shared BLUE ade_codec authority; the raw byte codec is POSITION-BLIND (it decodes null -> Genesis and hash32 -> Block without knowing block_number). No all-zero Hash32, no anchor fingerprint, and no Shelley genesis hash may stand in for the genesis predecessor. The position-aware check (block_number 0 requires Genesis; block_number > 0 requires Block) is enforced by the sibling forge/validation slice (CE-G-J-3, S3), NOT by this position-blind codec. |
| **Code** | crates/ade_types/src/shelley/block.rs (PrevHash = Genesis \| Block(Hash32) + ShelleyHeaderBody.prev_hash + block_hash() accessor); crates/ade_codec/src/shelley/block.rs (decode_prev_hash + the ShelleyHeaderBody AdeEncode null/hash32 match -- POSITION-BLIND); crates/ade_ledger/src/block_validity/header_position.rs (check_header_position -- the single POSITION-AWARE authority, block_number 0 <=> Genesis; S3); crates/ade_ledger/src/block_validity/header_input.rs (decode_block calls check_header_position before the header authority; S3); crates/ade_ledger/src/block_validity/verdict.rs (BlockValidityError::HeaderPositionInvalid -> existing HeaderInvalid class; S3); crates/ade_ledger/src/producer/forge.rs + crates/ade_ledger/src/block_validity/unsigned_header_pre_image.rs (forge + KES pre-image carry tick.prev_hash: PrevHash directly -- Genesis for block 0, byte-identical Block path; S3); crates/ade_runtime/src/producer/chain_evolution.rs (prev_hash() cold-start -> PrevHash::Genesis, all-zero stand-in deleted; S3) -- producer prev_hash migrated Hash32->PrevHash end to end (ProducerTick/TickInputs/ForgeRequestContext; S3) |
| **Tests** | `prevhash_genesis_round_trips_as_null`; `prevhash_block_round_trips_as_hash32`; `prevhash_codec_is_position_blind`; `genesis_successor_header_round_trips_with_null_prev`; `block_header_prev_hash_byte_identical_after_migration`; `header_position_zero_requires_genesis_ok`; `header_position_zero_with_block_is_rejected`; `header_position_nonzero_requires_block_ok`; `header_position_nonzero_with_genesis_is_rejected`; `decode_block_rejects_block_prev_at_block_number_zero` … (+10 more) |
| **CI** | `ci/ci_check_prevhash_single_wire_authority.sh` |

#### `CN-WIRE-10` — _enforced_

| Aspect | Location |
|--------|----------|
| **Source** | docs/clusters/PHASE4-N-F-G-L/cluster.md |
| **Requirement** | Ade's serve-side N2N handshake RESPONDER must encode versionData / MsgAcceptVersion / query-reply in the closed Cardano NodeToNode wire grammar a real cardano-node decodes, through the SINGLE shared per-version authority (encode_n2n_version_params) the INITIATOR also uses -- the two directions cannot diverge. The versionData arity matches the negotiated version: V11..=15 emit the 4-element [networkMagic, initiatorAndResponderDiffusionMode, peerSharing, query]; V16+ append perasSupport. The responder MUST NOT emit a placeholder / bare value (the prior VersionParams(vec![0x01]) = CBOR TInt 1, which a real cardano-node rejected with HandshakeDecodeError NodeToNodeV_15 "unknown encoding: TInt 1"). Closed grammar -- no fallback interpretation, no decoder loosening; the encoding is byte-pinned against captured real cardano-node fixtures, never an Ade<->Ade round-trip. |
| **Code** | crates/ade_network/src/handshake/version_table.rs (encode_n2n_version_params -- the SINGLE per-version N2N versionData wire authority); crates/ade_network/src/session/handshake_driver.rs (the serve responder builds AcceptVersion via encode_n2n_version_params(version.get(), params.network_magic), NOT a placeholder); crates/ade_node/src/admission/bootstrap.rs (build_n2n_version_table -- the initiator uses the SAME authority) |
| **Tests** | `responder_v15_accept_matches_real_cardano_node_preprod_fixture`; `responder_v15_accept_matches_failing_c1_peer_fixture`; `responder_v15_versiondata_is_a_four_element_array_not_a_bare_int`; `served_view_projects_durable_chain` |
| **CI** | `ci/ci_check_n2n_handshake_versiondata_authority.sh` |

#### `CN-WIRE-11` — _enforced_

| Aspect | Location |
|--------|----------|
| **Source** | docs/clusters/PHASE4-N-F-G-M/cluster.md |
| **Requirement** | Ade's serve-side ChainSync server must be wire-compatible with a real cardano-node follower's MsgFindIntersect, in two halves, served from the SINGLE ServedChainView authority: (A) REQUEST DECODE -- the codec MUST accept the MsgFindIntersect points list whether a real cardano-node encodes it as a CBOR INDEFINITE-length array (0x9f .. 0xff) or Ade encodes it definite-length. Scoped to THAT list ONLY: the outer message array, each Point, and the tip stay strictly definite (decode_array_header is unchanged and still rejects indefinite everywhere else); the indefinite form requires the 0xff break and the message is full-consumed -- no catch-all, no fallback, no unknown-CBOR acceptance. (B) REPLY -- Origin is the universal common ancestor (every chain descends from genesis), so a MsgFindIntersect whose points include Origin MUST be answered IntersectFound[Origin]; block points resolve through the existing closed ServedHeaderLookup::intersect; no match yields IntersectNotFound -- no widening beyond the Origin intersect. Closed grammar, byte-pinned against captured real cardano-node fixtures (the follower's MsgFindIntersect request + the node's MsgIntersectFound reply), never an Ade<->Ade round-trip (which passes against Ade's own definite-length encoding and so cannot catch the indefinite-array incompatibility). |
| **Code** | crates/ade_network/src/codec/primitives.rs (decode_array_head_two_form + try_consume_break -- the SCOPED two-form array head; decode_array_header stays definite-only); crates/ade_network/src/codec/chain_sync.rs (decode_find_intersect_points -- accepts the indefinite points list for MsgFindIntersect ONLY); crates/ade_network/src/chain_sync/server.rs (producer_chain_sync_serve FindIntersect arm -- Origin -> IntersectFound[Origin], block points via the existing closed intersect) |
| **Tests** | `real_cardano_node_findintersect_indefinite_points_list_decodes`; `real_cardano_node_findintersect_yields_intersect_found_origin`; `producer_chain_sync_serve_find_intersect_origin_yields_intersect_found_origin`; `producer_chain_sync_serve_find_intersect_known_point_yields_intersect_found`; `producer_chain_sync_serve_find_intersect_unknown_point_yields_intersect_not_found`; `decode_array_header_still_rejects_indefinite`; `two_form_accepts_definite_and_indefinite` |
| **CI** | `ci/ci_check_chainsync_findintersect_compat.sh` |

#### `CN-WIRE-12` — _enforced_

| Aspect | Location |
|--------|----------|
| **Source** | docs/clusters/PHASE4-N-F-G-O/cluster.md |
| **Requirement** | Ade's FEED/receive-side BlockFetch path MUST remove the protocol tag-24 wrapper using the SINGLE ade_codec unwrap authority (decompose_blockfetch_block = ade_codec::unwrap_tag24) before authoritative block decode, so decode_block / pump_block receive ONLY bare [era, block] bytes. This is the receive-side mirror of the serve-side compose_blockfetch_block (wrap_tag24, CN-WIRE-08) -- NOT a new block decoder, NOT a second unwrap implementation. The wire pump performs the unwrap EXACTLY ONCE, at the MsgBlock receive boundary, before it emits AdmissionPeerEvent::Block; the bare-bytes contract downstream (run_node_sync -> pump_block, and recovery/restart from WAL/db) is unchanged -- so the unwrap belongs on the FEED path, NOT inside pump_block (which recovery also feeds with bare bytes). Fail-closed: a malformed tag-24, a non-tag-24 payload where the BlockFetch protocol requires tag-24, or inner bytes that are not [era, block] -> a structured BlockFetchDecode error / peer drop, never a silent pass-through, skip, or fallback. |
| **Code** | crates/ade_runtime/src/admission/wire_pump.rs (handle_block_fetch BlockFetchMessage::Block arm -- calls decompose_blockfetch_block ONCE at the receive boundary, emits bare [era, block]; fail-closed BlockFetchDecode on a non-tag-24 payload); crates/ade_network/src/codec/block_fetch.rs (decompose_blockfetch_block = ade_codec::unwrap_tag24 -- the SINGLE inverse of compose_blockfetch_block, unchanged); crates/ade_node/src/node_sync.rs (run_node_sync -> pump_block consumes the bare [era, block] the wire pump now delivers) |
| **Tests** | `feed_unwrap_decodes_genesis_successor_block_zero`; `block_fetch_unwraps_tag24_emitting_bare_block`; `block_fetch_fails_closed_on_non_tag24_payload`; `served_view_projects_durable_chain` |
| **CI** | `ci/ci_check_feed_tag24_unwrap.sh` |

---

## DC — Derived Cardano-Compatibility Invariants (Project Constitution §3)

_253 rules._

### DC-ADMIT

#### `DC-ADMIT-01` — _enforced_

| Aspect | Location |
|--------|----------|
| **Source** | docs/planning/phase4-n-m-b-admission-invariants.md §1 (I-B1) + §6 table |
| **Requirement** | Closed AgreementVerdict sum (GREEN evidence, not authority): exactly four variants — Agreed{our_hash,peer_hash}, Lagging{our_slot,peer_slot}, Diverged{our_hash,peer_hash,slot}, InputNotFound{tx_in}. Each variant has narrow comparison semantics per the cluster invariants sketch §1 (I-B1). The reducer compares already-authoritative outputs (CN-CONS-08 admit verdict + peer's announced tip); it never decides validity, chain selection, or canonical state. See memory [[feedback-evidence-reducers-are-green-not-authority]] for the classification doctrine this rule embodies. |
| **Code** | crates/ade_node/src/admission/verdict.rs::{AgreementVerdict, BlockAdmitOutcome, derive} |
| **Tests** | `ade_node::admission::verdict::tests::verdict_agreed_when_hashes_match`; `ade_node::admission::verdict::tests::verdict_diverged_when_our_admit_differs_from_peer_hash`; `ade_node::admission::verdict::tests::verdict_diverged_when_admit_invalid_at_same_slot`; `ade_node::admission::verdict::tests::verdict_lagging_when_peer_ahead_of_our_slot`; `ade_node::admission::verdict::tests::verdict_input_not_found_when_admit_missing_input`; `ade_node::admission::verdict::tests::verdict_lagging_when_peer_tip_is_origin`; `ade_node::admission::verdict::tests::verdict_derive_is_pure_two_runs_byte_identical`; `ade_node::admission::verdict::tests::verdict_kind_discriminator_round_trips_each_variant` |
| **CI** | `ci/ci_check_lagging_is_evidence_only.sh` |

#### `DC-ADMIT-02` — _enforced_

| Aspect | Location |
|--------|----------|
| **Source** | docs/planning/phase4-n-m-b-admission-invariants.md §1 (I-B3) |
| **Requirement** | Verdict emitted exactly once per admit-attempt: every successful admit path produces exactly one agreement_verdict JSONL event. Never twice for the same block_hash; never zero for a successful admit. Test pins the property via DeterministicClock replay of a recorded admission run. |
| **Code** | crates/ade_node/src/admission/runner.rs::run_admission (per-Block branch) |
| **Tests** | `ade_node::admission::runner::tests::run_admission_emits_shutdown_on_signal`; `admission_replay_equivalence::admission_tip_update_does_not_emit_wal_entry` |
| **CI** | `ci/ci_check_admission_runner_closure.sh` |

#### `DC-ADMIT-03` — _enforced_

| Aspect | Location |
|--------|----------|
| **Source** | docs/planning/phase4-n-m-b-admission-invariants.md §1 (I-B2) |
| **Requirement** | Diverged + InputNotFound are authority-fatal at the binary boundary. Distinct exit codes: EXIT_LIVE_AGREEMENT_DIVERGED=30, EXIT_LIVE_INPUT_NOT_FOUND=31. Mirrors PHASE4-N-K DC-NODE-04 fail-fast discipline. The fatality is the binary's response to the evidence — the verdict reducer itself stays pure / no exit. |
| **Code** | crates/ade_node/src/admission/runner.rs::{EXIT_LIVE_AGREEMENT_DIVERGED, EXIT_LIVE_INPUT_NOT_FOUND, halt_for_verdict, halt_to_exit} |
| **Tests** | `ade_node::admission::runner::tests::exit_code_constants_round_trip_to_i32`; `ade_node::admission::runner::tests::halt_for_verdict_only_diverged_or_input_not_found_halts`; `admission_replay_equivalence::admission_exit_codes_match_registered_values` |
| **CI** | `ci/ci_check_admission_runner_closure.sh` |

#### `DC-ADMIT-04` — _enforced_

| Aspect | Location |
|--------|----------|
| **Source** | docs/planning/phase4-n-m-b-admission-invariants.md §1 (I-B4) + §2 (¬P-B10) |
| **Requirement** | Closed AdmissionLogEvent vocabulary (8 variants: admission_started, snapshot_imported, bootstrap_complete, block_received, block_admitted, agreement_verdict, admission_halted, admission_shutdown). Physically isolated from wire-only's LiveLogEvent — separate files; CI grep enforces BOTH DIRECTIONS: - wire-only-mode files do not emit AdmissionLogEvent literals; - admission-mode files do not emit wire-only-only LiveLogEvent literals. Per memory [[feedback-shell-must-not-overstate-semantic-truth]] the per-mode-isolation discipline shipped at PHASE4-N-L-LIVE carries forward. PHASE4-N-AJ extends this to a THIRD isolated closed-vocabulary file: the convergence-evidence transcript (--convergence-evidence-path) carries ONLY the closed 3-variant convergence subset {block_received, block_admitted, agreement_verdict} via the ConvergenceEvidenceSink wrapper (no inner-writer accessor); it emits no sched/forge/wire-only literals and none of the excluded admission-lifecycle variants. ci/ci_check_convergence_evidence_vocabulary_closed.sh is the file-tree half. |
| **Code** | crates/ade_node/src/admission_log/{event.rs, writer.rs} |
| **Tests** | `ade_node::admission_log::event::tests::admission_log_event_discriminator_round_trips_for_each_variant`; `ade_node::admission_log::event::tests::admission_log_event_match_is_exhaustive`; `ade_node::admission_log::event::tests::admission_log_event_agreement_verdict_carries_kind_discriminator`; `ade_node::admission_log::writer::tests::admission_log_writer_emits_one_object_per_line`; `ade_node::admission_log::writer::tests::admission_log_writer_serializes_admission_started_canonically`; `ade_node::admission_log::writer::tests::admission_log_writer_two_runs_are_byte_identical`; `ade_node::admission_log::writer::tests::admission_log_writer_emits_agreement_verdict_with_kind_field`; `ade_node::admission_log::writer::tests::admission_log_writer_omits_optional_fields_when_none`; `ade_node::admission_log::writer::tests::admission_log_writer_lines_are_parseable_as_one_json_object_per_line` |
| **CI** | `ci/ci_check_admission_log_vocabulary_closed.sh`; `ci/ci_check_convergence_evidence_vocabulary_closed.sh` |

#### `DC-ADMIT-05` — _enforced_

| Aspect | Location |
|--------|----------|
| **Source** | docs/planning/phase4-n-m-b-admission-invariants.md §1 (I-B7) |
| **Requirement** | Per-admit WAL append: every successful admit appends exactly one WalEntry::AdmitBlock to the configured WalStore. The entry's prior_fp chains to the previous entry's post_fp (or the anchor's initial_ledger_fingerprint for the first entry). Failure to append is authority-fatal. CN-WAL-01 single-authority preserved. |
| **Code** | crates/ade_node/src/admission/runner.rs::run_admission (per-AdmittedBlock branch → wal_store.append) |
| **Tests** | `admission_replay_equivalence::admission_replay_equivalence_byte_identical_wal_after_two_runs`; `admission_replay_equivalence::admission_tip_update_does_not_emit_wal_entry` |
| **CI** | `ci/ci_check_admission_runner_closure.sh`; `ci/ci_check_wal_append_only.sh` |

#### `DC-ADMIT-06` — _enforced_

| Aspect | Location |
|--------|----------|
| **Source** | docs/planning/phase4-n-m-b-admission-invariants.md §1 (I-B8) |
| **Requirement** | Verdict reducer is pure: verdict::derive(admit_outcome, peer_tip) is a pure function over closed input enums → closed output enum. No I/O, no clock, no state. The reducer is the GREEN-evidence boundary per [[feedback-evidence-reducers-are-green-not-authority]] — it compares authoritative outputs; it never decides authority. |
| **Code** | crates/ade_node/src/admission/verdict.rs::derive |
| **Tests** | `ade_node::admission::verdict::tests::verdict_derive_is_pure_two_runs_byte_identical` |
| **CI** | `ci/ci_check_lagging_is_evidence_only.sh` |

#### `DC-ADMIT-07` — _enforced_

| Aspect | Location |
|--------|----------|
| **Source** | docs/planning/phase4-n-m-b-admission-invariants.md §1 (I-B7) |
| **Requirement** | Admit-replay-equivalence (true-tier): for every successful WalEntry::AdmitBlock transition, replay from the prior checkpoint plus WAL produces (a) the same post-admit LedgerState fingerprint AND (b) the same emitted AgreementVerdict from a re-run of verdict::derive over the replayed (admit_outcome, peer_tip) pair. This is a true-tier property strengthening CN-STORE-03 (replay-equivalent recovery), not merely a logging property. The integration test asserts both halves. |
| **Code** | crates/ade_node/tests/admission_replay_equivalence.rs |
| **Tests** | `admission_replay_equivalence::admission_replay_equivalence_byte_identical_wal_after_two_runs`; `admission_replay_equivalence::admission_signal_shutdown_returns_clean_exit`; `admission_replay_equivalence::admission_disconnect_to_zero_peers_exits_clean`; `admission_replay_equivalence::admission_tip_update_does_not_emit_wal_entry` |
| **CI** | `ci/ci_check_admit_replay_equivalence.sh` |

#### `DC-ADMIT-08` — _enforced_

| Aspect | Location |
|--------|----------|
| **Source** | docs/planning/phase4-n-m-b-admission-invariants.md §1 (I-B1) + §2 (¬P-B8) |
| **Requirement** | Lagging is evidence-state only: AgreementVerdict::Lagging means the local admitted chain is a prefix of the comparison target (peer's announced chain) up to the peer's announced slot. It MUST NOT be treated as success / healthy / live-ready / consensus-equivalent by any caller. CI grep (ci_check_lagging_is_evidence_only.sh) forbids: - Lagging matched as part of a success-result pattern (Ok(Lagging), Lagging=>true, etc.) outside the verdict reducer and its tests; - Any caller passing a Lagging verdict into a "ready" / "healthy" / "live" predicate. |
| **Code** | crates/ade_node/src/admission/verdict.rs::AgreementVerdict::Lagging |
| **Tests** | `ade_node::admission::verdict::tests::verdict_lagging_when_peer_ahead_of_our_slot`; `ade_node::admission::verdict::tests::verdict_lagging_when_peer_tip_is_origin` |
| **CI** | `ci/ci_check_lagging_is_evidence_only.sh` |

#### `DC-ADMIT-09` — _enforced_

| Aspect | Location |
|--------|----------|
| **Source** | docs/planning/phase4-n-m-b-admission-invariants.md §0 + §2 (¬P-B9) |
| **Requirement** | Admission code paths do not add partial reference-script support, permissive ref-script skipping, or any seed-import fallback. N-M-A's fail-fast on JsonSeedError::UnsupportedTxOutFeature stays exactly as-is. Real preprod seed import remains blocked until a future A1.1 slice closes. CI grep (ci_check_admission_no_refscript_skip.sh) verifies admission code paths do not match UnsupportedTxOutFeature with a permissive arm. Rationale: prevents B from inheriting a known-invalid importer and rests sub-cluster C operator-pass evidence on a clean A1.1 closure. |
| **Code** | crates/ade_runtime/src/seed_import/importer.rs::build_canonical_tx_out (fail-fast guard) |
| **Tests** | _(no tests listed — gap)_ |
| **CI** | `ci/ci_check_admission_no_refscript_skip.sh` |

#### `DC-ADMIT-10` — _enforced_

| Aspect | Location |
|--------|----------|
| **Source** | docs/planning/phase4-n-m-c-operator-pass-invariants.md §1 (I-C8) |
| **Requirement** | Every admission JSONL block-event carries consensus_inputs_fingerprint. BlockAdmitted, AgreementVerdict, BootstrapComplete, and AdmissionStarted events emit the consensus_inputs_fingerprint_hex field. The admission JSONL vocabulary stays CLOSED — the field is added to existing variants, NOT a new variant. The fingerprint is the load-bearing binding between the operator-supplied oracle (LiveConsensusInputs) and every BLUE-authority claim the transcript makes. |
| **Code** | crates/ade_node/src/admission_log/event.rs (AdmissionLogEvent — 4 binding variants carry consensus_inputs_fingerprint_hex), crates/ade_node/src/admission_log/writer.rs (JSONL emit of the new field), crates/ade_node/src/admission/runner.rs (fingerprint threaded through all 4 emits via consensus_fp_hex) |
| **Tests** | `admission_log::writer::tests::admission_log_writer_emits_one_object_per_line`; `admission_log::writer::tests::admission_log_writer_serializes_admission_started_canonically`; `admission_log::writer::tests::admission_log_writer_emits_agreement_verdict_with_kind_field` |
| **CI** | `ci/ci_check_live_ledger_view_epoch_window.sh`; `ci/ci_check_admission_log_vocabulary_closed.sh` |

#### `DC-ADMIT-11` — _enforced_

| Aspect | Location |
|--------|----------|
| **Source** | docs/planning/phase4-n-m-c-operator-pass-invariants.md §1 (I-C9) + §2 (¬P-C2) |
| **Requirement** | Cross-epoch silent use forbidden. If a peer sends a block whose slot is outside [epoch_start_slot, epoch_end_slot], the runner MUST emit AdmissionHalted { reason: CrossEpochUse } and exit non-zero WITHOUT calling admit_via_block_validity. There is no silent "skip and continue" path (¬P-C2 no cross-epoch silent use). |
| **Code** | crates/ade_node/src/admission/runner.rs (pre-admit peek_block_slot + slot-window guard + AdmissionHaltReason::CrossEpochUse emit + EXIT_LIVE_CROSS_EPOCH_USE=32) |
| **Tests** | `cross_epoch_block_triggers_halt_without_admit` |
| **CI** | `ci/ci_check_live_ledger_view_epoch_window.sh` |

#### `DC-ADMIT-12` — _enforced_

| Aspect | Location |
|--------|----------|
| **Source** | docs/planning/phase4-n-m-c-operator-pass-invariants.md §1 (I-C10) + §2 (¬P-C7, ¬P-C9) |
| **Requirement** | Undecodable peer bytes are Diverged (or PeerSentUndecodableBytes); never InputNotFound; never silent clean exit. C strengthens N-M-B's ProcessedBlock::Undecodable → AdmissionExitCode::Ok path: undecodable peer bytes are adversarial by default. They map to AgreementVerdict::Diverged (when a peer tip exists at the same slot) or AdmissionHalted { reason: PeerSentUndecodableBytes } (when no peer tip exists at that slot) (¬P-C7 no InputNotFound for adversarial input, ¬P-C9 no silent clean-exit on adversarial bytes). |
| **Code** | crates/ade_node/src/admission/runner.rs (ProcessedBlock::Undecodable arm routes to Diverged when peer tip exists at a Point::Block, else PeerSentUndecodableBytes — exit codes 30 / 34) |
| **Tests** | `admission_log::event::tests::admission_log_event_discriminator_round_trips_for_each_variant` |
| **CI** | `ci/ci_check_admission_wire_pump_closure.sh` |

### DC-ANCHOR

#### `DC-ANCHOR-01` — _enforced_

| Aspect | Location |
|--------|----------|
| **Source** | docs/planning/phase4-n-m-ledger-seed-invariants.md §7 (#5) |
| **Requirement** | BootstrapAnchor canonical CBOR round-trip: encode + decode preserves all 6 fields byte-identically. SCHEMA_VERSION = 1 in the encoded bytes; unknown version on decode is fail-fast. |
| **Code** | crates/ade_ledger/src/bootstrap_anchor/anchor.rs |
| **Tests** | `crates/ade_ledger/src/bootstrap_anchor/anchor.rs::tests::bootstrap_anchor_round_trips_via_canonical_cbor`; `crates/ade_ledger/src/bootstrap_anchor/anchor.rs::tests::bootstrap_anchor_encode_two_runs_byte_identical`; `crates/ade_ledger/src/bootstrap_anchor/anchor.rs::tests::bootstrap_anchor_decode_rejects_unknown_version`; `crates/ade_ledger/src/bootstrap_anchor/anchor.rs::tests::bootstrap_anchor_decode_rejects_trailing_bytes`; `crates/ade_ledger/src/bootstrap_anchor/anchor.rs::tests::bootstrap_anchor_decode_rejects_short_buffer`; `crates/ade_ledger/src/bootstrap_anchor/anchor.rs::tests::bootstrap_anchor_decode_rejects_wrong_outer_array_length`; `crates/ade_ledger/src/bootstrap_anchor/anchor.rs::tests::bootstrap_anchor_decode_rejects_short_hash` |
| **CI** | `ci/ci_check_bootstrap_anchor_closure.sh` |

### DC-CBOR

#### `DC-CBOR-01` — _declared_

| Aspect | Location |
|--------|----------|
| **Source** | Project constitution §3, T-ENC-01, T-ENC-03 |
| **Requirement** | Cardano CBOR decode/encode round-trips to identical bytes for all era types |
| **Code** | — _(declared — not yet enforced)_ |
| **Tests** | — _(declared — not yet enforced)_ |
| **CI** | — _(declared — not yet enforced)_ |

#### `DC-CBOR-02` — _declared_

| Aspect | Location |
|--------|----------|
| **Source** | Project constitution §3, T-ENC-01, T-DET-01 |
| **Requirement** | Original wire bytes preserved for hash computation on hash-critical paths (see Byte Authority Model) |
| **Code** | — _(declared — not yet enforced)_ |
| **Tests** | — _(declared — not yet enforced)_ |
| **CI** | — _(declared — not yet enforced)_ |

### DC-CINPUT

#### `DC-CINPUT-01` — _enforced_

| Aspect | Location |
|--------|----------|
| **Source** | docs/clusters/PHASE4-N-F-A/cluster.md; A3a-wal-provenance-entry.md; A3b-warm-start-restore.md |
| **Requirement** | WARM-START VERIFICATION CAPABILITY (authority surface — NOT production restart). The seed-epoch consensus-input import is a canonical, replay-reconstructable WAL fact: an additive closed WalEntry::SeedEpochConsensusInputsImported variant (distinct tag; does NOT participate in the AdmitBlock prior_fp/post_fp chain), appended AFTER the sidecar put (the WAL append is the commit point). Replay reconstructs a typed RecoveredBootstrapProvenance view (exactly one per store/anchor; duplicate or anchor-mismatch fails closed). Given that view, bootstrap_initial_state's warm-start branch (RequiredFromRecoveredProvenance) restores the sidecar and verifies it fail-closed: sidecar present, blake2b_256 == provenance.sidecar_hash, A1 decode, anchor_fp + epoch_no binding, byte- identity re-encode — exposing the recovered SeedEpochConsensusInputs or halting (typed BootstrapError, EXIT_AUTHORITY_FATAL_DECODE, no bundle fallback). This is proven on the AUTHORITY SURFACE (bootstrap_initial_state exercised directly); no production mode is wired to it (node.rs run_node_until_shutdown + recover_node_state are test-only; produce_mode cold-starts). The PRODUCTION restart path is this rule's open obligation, deferred to PHASE4-N-F-C. |
| **Code** | crates/ade_ledger/src/wal/event.rs (SeedEpochConsensusInputsImported, tag 3); crates/ade_ledger/src/wal/replay.rs (ReplayOutcome + RecoveredBootstrapProvenance); crates/ade_runtime/src/seed_consensus_provenance.rs (append helper); crates/ade_runtime/src/bootstrap.rs (SeedEpochConsensusSource, BootstrapState, restore_seed_epoch_consensus_inputs) |
| **Tests** | `wal_seed_cinput_entry_round_trips_byte_identical`; `replay_yields_bootstrap_provenance_view`; `replay_rejects_duplicate_provenance_entry`; `replay_rejects_anchor_mismatched_provenance_entry`; `admit_block_chain_unaffected_by_provenance_entry`; `warm_start_restores_seed_epoch_consensus_inputs_byte_identical`; `warm_start_fails_closed_on_missing_sidecar`; `warm_start_fails_closed_on_hash_mismatch`; `warm_start_fails_closed_on_anchor_mismatch`; `warm_start_fails_closed_on_epoch_mismatch` … (+8 more) |
| **CI** | `ci/ci_check_lifecycle_owner_uses_bootstrap_initial_state.sh` |

#### `DC-CINPUT-02a` — _enforced_

| Aspect | Location |
|--------|----------|
| **Source** | docs/clusters/PHASE4-N-F-A/cluster.md; A4-projection-pooldistr-vrf.md |
| **Requirement** | PROJECTION EQUIVALENCE. The recovered SeedEpochConsensusInputs projects deterministically to the leadership-consumed PoolDistrView (the full LedgerView surface: total_active_stake, pool_active_stake, pool_vrf_keyhash, active_slots_coeff; single-epoch — off-epoch queries return None) via PoolDistrView::from_seed_epoch_consensus_inputs, EQUIVALENT to the prior operator-bundle projection (pool_distr_view_from_consensus_inputs) for the seed epoch; and recovered eta0 (from chain_dep) drives leader_vrf_input identically. The projection is a pure BLUE field map (A2 already merged stake + VRF keyhash). This rule covers the PROJECTION only; producer CONSUMPTION of the projected recovered surface is deferred to PHASE4-N-F-C (CE-A-4b). |
| **Code** | crates/ade_ledger/src/consensus_view.rs (PoolDistrView::from_seed_epoch_consensus_inputs); crates/ade_core/src/consensus/vrf_cert.rs (leader_vrf_input, reused) |
| **Tests** | `recovered_surface_projects_pooldistrview_and_expected_vrf_input`; `projection_maps_recovered_fields_onto_ledgerview_surface`; `projection_two_runs_identical`; `projection_off_epoch_returns_none` |
| **CI** | _(no CI script listed)_ |

#### `DC-CINPUT-02b` — _enforced_

| Aspect | Location |
|--------|----------|
| **Source** | docs/clusters/PHASE4-N-F-C/cluster.md; L5-produce-from-recovered-state.md |
| **Requirement** | PRODUCER CONSUMPTION (closes CE-A-4b). The node-lifecycle forge base is built from the recovered selected tip + the recovered SeedEpochConsensusInputs: forge_one_from_recovered projects the leadership PoolDistrView via PoolDistrView::from_seed_epoch_consensus_inputs(recovered) and drives the reused run_real_forge engine with eta0 from the recovered chain_dep — and fails closed (MissingRecoveredConsensusInputs) when the recovered record is absent, with no operator-bundle / cold-InMemoryChainDb / --consensus-inputs-path fallback. This is the consumption half of DC-CINPUT-02a's projection: A4 proved the projection; this rule binds the producer to consume it. Deterministic across runs. |
| **Code** | crates/ade_node/src/node_sync.rs (forge_one_from_recovered: recovered BootstrapState -> PoolDistrView::from_seed_epoch_consensus_inputs -> ForgeRequestContext -> run_real_forge) |
| **Tests** | `forge_from_recovered_uses_recovered_pool_distr`; `forge_from_recovered_fails_closed_without_recovered_inputs`; `forge_from_recovered_is_deterministic_across_two_runs` |
| **CI** | `ci/ci_check_consensus_input_provenance.sh`; `ci/ci_check_recovered_ledger_pparams_sourced.sh` |

#### `DC-CINPUT-03` — _enforced_

| Aspect | Location |
|--------|----------|
| **Source** | docs/clusters/PHASE4-N-F-G-N/cluster.md |
| **Requirement** | The producer Praos VRF leader/header input is `praos_vrf_input(slot, eta0)` = `blake2b256(slot_be8 ‖ eta0_32)` (= cardano `mkInputVRF`), where eta0 is the Cardano epoch nonce carried by the recovered `SeedEpochConsensusInputs` and overlaid onto `chain_dep` at WarmStart (T-REC-04). A forged Conway header therefore verifies under a real peer's `mkInputVRF(slot, eta0)`. The VRF variant is UNCHANGED -- IETF draft-03, 80-byte proof (verified against real mainnet + preprod Conway headers, both draft-03); the fix is the recovered eta0 SOURCING, not the crypto. No draft-13/batch-compat migration, no C1-only branch, no genesis-derived nonce. |
| **Code** | crates/ade_core/src/consensus/vrf_cert.rs (praos_vrf_input = blake2b256(slot_be8 ‖ eta0_32) = mkInputVRF); crates/ade_runtime/src/producer/signing.rs (vrf_prove over the alpha); crates/ade_ledger/src/seed_consensus_inputs.rs (epoch_nonce sidecar field); crates/ade_runtime/src/bootstrap.rs (overlay onto chain_dep) |
| **Tests** | `warm_start_overlays_recovered_eta0_onto_chain_dep_g_n`; `pinning_praos_vrf_input_and_threshold_match_fixture`; `recovered_surface_projects_pooldistrview_and_expected_vrf_input` |
| **CI** | `ci/ci_check_warmstart_eta0_overlay.sh` |

#### `DC-CINPUT-04` — _enforced_

| Aspect | Location |
|--------|----------|
| **Source** | docs/clusters/PHASE4-N-F-G-P/cluster.md |
| **Requirement** | The receive/feed-path header-validation consensus view -- the LedgerView passed to block_validity -> validate_and_apply_header for Step 5 (VRF-keyhash binding) and Step 7 (leader threshold) -- MUST be the RECOVERED consensus surface: ASC + total_active_stake + pool_distribution + per-pool VRF keyhash, projected from the recovered SeedEpochConsensusInputs via the SAME single authority the forge uses (PoolDistrView::from_seed_epoch_consensus_inputs). It is NEVER an empty/zero/default placeholder. Forge and feed validation share ONE recovered consensus surface. Fail-closed: a missing recovered SeedEpochConsensusInputs on a feed-wired node (--peer) -> a structured NodeLifecycleError::FeedMissingRecoveredConsensusInputs / halt -- never an empty view, never "accept if missing stake," never a leader-threshold bypass. NARROW: the header-validation consensus view ONLY; the authoritative ledger state stays the authority for ledger verdicts. Receive-side mirror of DC-CINPUT-02b (forge leadership view from recovered); same class as DC-CINPUT-03 / T-REC-04 (G-N forge eta0) but a different recovered input (stake distribution + ASC, not the nonce) on a different path (feed, not forge). |
| **Code** | crates/ade_node/src/node_lifecycle.rs (the forge-on On-arm feed ledger_view is PoolDistrView::from_seed_epoch_consensus_inputs(recovered) -- NOT PoolDistrView::new(empty); fail-closed FeedMissingRecoveredConsensusInputs when --peer is set but the recovered record is absent); crates/ade_ledger/src/consensus_view.rs (PoolDistrView::from_seed_epoch_consensus_inputs -- the single projection authority shared with the forge); crates/ade_node/src/node_sync.rs (forge_one_from_recovered uses the SAME projection -- DC-CINPUT-02b) |
| **Tests** | `feed_header_validates_against_recovered_surface_not_empty_view` |
| **CI** | `ci/ci_check_feed_leader_threshold_view.sh` |

#### `DC-CINPUT-05` — _enforced_

| Aspect | Location |
|--------|----------|
| **Source** | docs/active/warmstart-era-schedule-venue-slice.md (live C2-PREVIEW forge SlotBeforeSystemStart finding: warm-start hardcoded the preprod epoch length 432000, breaking preview replay) |
| **Requirement** | Venue epoch geometry is DURABLE REPLAY AUTHORITY. A recovered store MUST replay using the epoch geometry (epoch_start_slot + epoch_length_slots) persisted with the seed/import that CREATED the store -- NEVER re-derived from whatever genesis/CLI a restart happens to supply. The seed-epoch sidecar (SeedEpochConsensusInputs, schema v4 since ECA-2-pre) carries the venue epoch_start_slot + epoch_length_slots, populated at import from the cardano-cli-reported epoch window (epoch_length = epoch_end_slot - epoch_start_slot + 1: preview 86400, preprod 432000, ...). warm_start_recovery + the live-follow/forge era-schedule rebuild make_node_schedule from THOSE durable values; there is NO hardcoded epoch length, NO venue-name switch, NO implicit default, NO hidden fallback to 432000. A restart-supplied --genesis-file is ONLY a consistency check (RestartGenesisGeometryMismatch fail-closed on disagreement), never a re-derivation source. An old v2 sidecar (no geometry) fails closed at decode (UnknownVersion) -- a store must be re-seeded, never silently re-geometried. |
| **Code** | crates/ade_ledger/src/seed_consensus_inputs.rs (SeedEpochConsensusInputs.epoch_start_slot + epoch_length_slots; SEED_CINPUT_SCHEMA_VERSION=4 since ECA-2-pre; FIELDS_OUTER=11; decode rejects v1/v2/v3 fail-closed UnknownVersion, the bootstrap authority surfaces a pre-v4 mismatch as the typed ConsensusInputsSchemaUnsupported per DC-CINPUT-06); crates/ade_runtime/src/seed_consensus_merge.rs (merge_seed_epoch_consensus_inputs persists the geometry from canonical.epoch_length_slots(); InvalidEpochWindow fail-closed on a degenerate window); crates/ade_runtime/src/consensus_inputs/canonical.rs (LiveConsensusInputsCanonical::epoch_length_slots() -> Option<u32>, end-start+1 fail-closed); crates/ade_node/src/node_lifecycle.rs (make_node_schedule takes epoch_length_slots explicitly -- no hardcoded 432000; warm_start_recovery + recovered_node_schedule + the import-window caller all source the geometry from the sidecar/canonical; assert_restart_genesis_matches_sidecar consistency check -> RestartGenesisGeometryMismatch) |
| **Tests** | `warm_start_schedule_locates_block_by_venue_geometry_not_hardcoded_432000`; `merge_persists_venue_epoch_geometry_preview_and_preprod`; `restart_genesis_epoch_length_mismatch_fails_closed`; `seed_epoch_consensus_inputs_round_trips_byte_identical`; `seed_cinput_decode_rejects_unknown_version` |
| **CI** | _(no CI script listed)_ |

#### `DC-CINPUT-06` — _enforced_

| Aspect | Location |
|--------|----------|
| **Source** | docs/clusters/EPOCH-CONSENSUS-VIEW/SLICE-ECA-2-pre-seed-sidecar-v4.md; user directive 2026-06-21 (the consensus profile is a single recovered authority surface; persist genesis_hash + protocol_params_hash in the v4 sidecar; no separate EVIEW manifest authority, no runtime fallback, no recomputation; typed upgrade error not corruption) |
| **Requirement** | The durable consensus PROFILE includes genesis_hash + protocol_params_hash, persisted canonically in the v4 SeedEpochConsensusInputs sidecar and recovered IDENTICALLY at warm-start -- the inputs to the ECA-0b consensus-profile commitment blake2b(domain ‖ genesis_hash ‖ protocol_params_hash ‖ asc) that derive_candidate binds and to_pool_distr_view re-verifies. They are NOT optional EVIEW metadata: a continuous producer must recover the FULL profile from the STORE, never from a restart CLI/config/genesis (the same durable-authority rule as DC-CINPUT-05 venue geometry) and never by recomputing protocol_params_hash from reserialized params (byte-sensitive -- the hash is over the IMPORTED protocol-params JSON). Schema bumped v3->v4 (SEED_CINPUT_SCHEMA_VERSION=4, FIELDS_OUTER=11; the two bytes(32) encode/decode after epoch_nonce); merge_seed_epoch_consensus_inputs populates both from the canonical import bundle; the persist writes v4 bytes BEFORE state is usable. Old v1/v2/v3 sidecars fail closed at decode (UnknownVersion); the bootstrap authority maps a pre-v4 version mismatch to the TYPED, recoverable, auditable BootstrapError::ConsensusInputsSchemaUnsupported{found_version, required_version} (a reimport requirement), DISTINCT from corruption (SeedConsensusSidecarDecode). The fingerprint/WAL provenance (sidecar_hash over the full bytes) AND the BootstrapManifest seed_hash = blake2b_256(sidecar bytes) binding both cover the two new hashes TRANSITIVELY (no manifest format change). No CLI/config/genesis fallback, no recompute. The single recovered consensus-profile authority surface (rejected: splitting the profile across the seed sidecar + a separate EVIEW manifest, which creates a seed-says-A / manifest-says-B mismatch class). |
| **Code** | crates/ade_ledger/src/seed_consensus_inputs.rs (SeedEpochConsensusInputs.genesis_hash + protocol_params_hash; SEED_CINPUT_SCHEMA_VERSION=4; FIELDS_OUTER=11; encode/decode the two bytes(32) after epoch_nonce; decode returns UnknownVersion for version != 4); crates/ade_runtime/src/seed_consensus_merge.rs (merge_seed_epoch_consensus_inputs copies canonical.genesis_hash + canonical.protocol_params_hash); crates/ade_runtime/src/bootstrap.rs (BootstrapError::ConsensusInputsSchemaUnsupported{found_version, required_version}; restore_seed_epoch_consensus_inputs maps a decode UnknownVersion -> it, distinct from SeedConsensusSidecarDecode; node.rs exit_code groups it with the authority-fatal-decode warm-start failures); crates/ade_node/src/node_lifecycle.rs (warm_start_recovery -- the FIRST + live sidecar decode -- maps UnknownVersion to NodeLifecycleError::ConsensusInputsSchemaUnsupported too, with an operator-facing reimport message in report(); both decode sites surface the typed error); crates/ade_ledger/src/bootstrap_manifest.rs (seed_hash = blake2b_256 over the v4 sidecar bytes binds the two hashes transitively). ci/ci_check_eview_seed_sidecar_v4.sh. |
| **Tests** | `seed_epoch_consensus_inputs_round_trips_byte_identical`; `seed_cinput_canonical_bytes_cover_the_consensus_profile_hashes`; `seed_cinput_decode_rejects_unknown_version`; `merge_persists_consensus_profile_hashes`; `warm_start_pre_v4_sidecar_is_typed_schema_upgrade_not_corruption` |
| **CI** | `ci/ci_check_eview_seed_sidecar_v4.sh` |

#### `DC-CINPUT-07` — _declared_

| Aspect | Location |
|--------|----------|
| **Source** | docs/active (CONWAY-DEPOSIT-PARAMS-BOOTSTRAP slice); native-bootstrap continuous-operation blocker: a preview re-follow bootstrapped at epoch 1338 stalled the accumulator at the 1339->1340 boundary with CertApply(ValidationEnvironment(MissingDRepActivityParam)) because assemble_native_mithril_seed hardcoded conway_deposit_params = None; user directive 2026-07-06 (import from the certified curPParams via the ONE canonical decoder; never default drep_activity; versioned durable fail-closed; no GovCertValidationEnv fallback). |
| **Requirement** | Conway deposit-parameter bootstrap authority. The Conway-only deposit params (drep_deposit / gov_action_deposit / drep_activity) are DECODED from the certified Mithril snapshot's Conway curPParams (the verified positions 27 govActionDeposit / 28 dRepDeposit / 29 dRepActivity) by the SINGLE canonical BLUE decoder (read_conway_pparams / decode_native_nonutxo_state), NEVER defaulted, guessed, fixture-derived, or read via a parallel parser. They are carried as a REQUIRED (non-Option) field on NativeSnapshotNonUtxoState, threaded by the native Mithril assembly into the assembled LedgerState.conway_deposit_params = Some(..) (never None on the Conway path), and copied by EpochAccumulator::seed_from_bootstrap_ledger into the accumulator — so a governance-active epoch boundary reaches drep_activity through LedgerState::gov_cert_env (GovCertEnv) and no longer fail-closes CertApply(ValidationEnvironment(MissingDRepActivityParam)). Tamper-evidence is dual: the three fields fold into the pparams component of fingerprint() (so the native bootstrap seed_hash / initial_ledger_fingerprint covers a substituted deposit param) AND into the v12 native-nonutxo-state S1a commitment. Durable fail-closed: EPOCH_ACCUMULATOR_SCHEMA_VERSION is bumped 1->2; a pre-fix v1 store fails closed at decode (UnknownVersion{ expected:2, found:1}) and a v2 Conway accumulator decoded with conway_deposit_params == None fails closed with the structured EpochAccumulatorCodecError::MissingConwayDepositParams — migration is an EXPLICIT re-bootstrap, never a silent reinterpretation as a defaulted set. Missing (curPParams arity < 31) or malformed (wrong CBOR type at the deposit positions) curPParams is TERMINAL (ProtocolParamsMissing / MalformedCbor), never a default substitution. NO fallback in the governance-cert environment: a missing param stays terminal (MissingDRepActivityParam), never patched to a default activity period. |
| **Code** | crates/ade_ledger/src/ledgerdb_state.rs (read_conway_pparams reads govActionDeposit/dRepDeposit/dRepActivity at CONWAY_PP_GOV_ACTION_DEPOSIT_INDEX=27 / CONWAY_PP_DREP_DEPOSIT_INDEX=28 / CONWAY_PP_DREP_ACTIVITY_INDEX=29 with nn_read_u64, returning crate::pparams::ConwayOnlyDepositParams; read_conway_pparams_from_utxo_state + decode_native_nonutxo_state thread it up; NativeSnapshotNonUtxoState.conway_deposit_params is a REQUIRED field; commit_native_nonutxo_state binds it under the v12 tag). crates/ade_runtime/src/mithril_native_assembly.rs (assemble_native_mithril_seed sets ledger.conway_deposit_params = Some(s1a.conway_deposit_params.clone())). crates/ade_ledger/src/epoch_accumulator.rs (EPOCH_ACCUMULATOR_SCHEMA_VERSION=2; seed_from_bootstrap_ledger + as_ledger_view carry conway_deposit_params; decode_epoch_accumulator fails closed MissingConwayDepositParams when a Conway accumulator decodes with None, and UnknownVersion on a v1 store). crates/ade_ledger/src/fingerprint.rs (fingerprint_pparams folds the three fields into the pparams component when present). crates/ade_ledger/src/state.rs (gov_cert_env / conway_deposit_view stay fail-closed on None, no fallback). crates/ade_testkit/src/harness/snapshot_loader.rs (parse_conway_gov_params reads the SAME positions; proven equivalent to the BLUE decoder). ci/ci_check_conway_deposit_params_bootstrap.sh. |
| **Tests** | `ade_ledger tests/ledgerdb_nonutxo_hermetic.rs::happy_minimal_state_decodes_all_fields (conway_deposit_params decoded from curPParams idx 27/28/29)`; `ade_ledger tests/ledgerdb_nonutxo_hermetic.rs::malformed_drep_activity_type_is_terminal (wrong CBOR type at idx 29 -> MalformedCbor)`; `ade_ledger tests/ledgerdb_nonutxo_hermetic.rs::malformed_pparams_arity_is_terminal (curPParams arity < 31 -> ProtocolParamsMissing)`; `ade_ledger tests/ledgerdb_nonutxo_hermetic.rs::wrong_era_is_terminal (pre-Conway native state -> UnsupportedEra)`; `ade_ledger::ledgerdb_state::tip_tests::v6_commitment_is_deterministic_and_binds_gov (v12 binds conway_deposit_params.drep_activity)`; `ade_ledger::fingerprint::tests::tampered_drep_activity_flips_pparams_fingerprint`; `ade_ledger::fingerprint::tests::pparams_fingerprint_includes_conway_deposits_when_present`; `ade_ledger::epoch_accumulator::tests::codec_v2_conway_without_deposit_params_fails_closed (v2 + None -> MissingConwayDepositParams)`; `ade_ledger::epoch_accumulator::tests::codec_rejects_unknown_version (v1 store -> UnknownVersion{expected:3, found:1}; expected bumped 2->3 by CE3D-BOOTSTRAP-FEE-BUFFER-S1)`; `ade_ledger::epoch_accumulator::tests::imported_deposit_params_cross_governance_boundary_and_reach_gov_cert_env (boundary regression: env reached unchanged, DRep cert applies, cross carries the params)` … (+3 more) |
| **CI** | `ci/ci_check_conway_deposit_params_bootstrap.sh` |

### DC-COMPAT

#### `DC-COMPAT-01` — _enforced_

| Aspect | Location |
|--------|----------|
| **Source** | docs/clusters/PHASE4-N-Y/S5-compatibility-evidence.md |
| **Requirement** | Cardano compatibility is proven ONLY on observable surfaces — per-block accept/reject verdict, selected tip hash, block hashes, cardano-cli query-utxo result, protocol transcripts — with named fixtures pinning oracle versions (cardano_node_version, cardano_cli_version) and reproducible inputs. Asserting Ade's internal ledger fingerprint == a Haskell/cardano-node serialized-state hash is FORBIDDEN and CI-blocked. The only valid fingerprint-equality is internal Ade-vs-Ade (genesis-path == snapshot-path). |
| **Code** | crates/ade_testkit/src/harness/sync_diff.rs; ci/ci_check_no_haskell_fingerprint_equality.sh |
| **Tests** | `sync_differential_snapshot_to_tip` |
| **CI** | `ci/ci_check_no_haskell_fingerprint_equality.sh` |

### DC-CONS-IN

#### `DC-CONS-IN-01` — _enforced_

| Aspect | Location |
|--------|----------|
| **Source** | docs/planning/phase4-n-m-c-operator-pass-invariants.md §1 (I-C2) |
| **Requirement** | Closed importer error sum: Io \| Json \| BadField \| MissingField \| BadHashHex \| BadEpochWindow \| BadPoolDistribution \| EraNotSupported. No Option field receives a runtime default. Missing pool distribution, nonce, ASC, VRF keyhash, era schedule, or any hash field is fatal at import time (¬P-C4 no partial importer fallback). |
| **Code** | crates/ade_runtime/src/consensus_inputs/importer.rs (LiveConsensusInputsImportError closed enum + validate_and_lift) |
| **Tests** | `consensus_inputs::importer::tests::unsupported_era_fails_fast`; `consensus_inputs::importer::tests::empty_era_string_fails_fast`; `consensus_inputs::importer::tests::epoch_end_before_start_is_bad_window`; `consensus_inputs::importer::tests::tip_outside_window_is_bad_window`; `consensus_inputs::importer::tests::zero_asc_denom_is_bad_field`; `consensus_inputs::importer::tests::short_genesis_hash_is_bad_hash_hex`; `consensus_inputs::importer::tests::non_hex_in_hash_is_bad_hash_hex`; `consensus_inputs::importer::tests::pool_in_distribution_missing_from_vrf_map_is_bad_pool`; `consensus_inputs::importer::tests::pool_id_wrong_width_is_bad_hash_hex`; `consensus_inputs::importer::tests::bad_json_surface_is_json_variant` … (+2 more) |
| **CI** | `ci/ci_check_live_consensus_inputs_closure.sh` |

#### `DC-CONS-IN-02` — _enforced_

| Aspect | Location |
|--------|----------|
| **Source** | docs/planning/phase4-n-m-c-operator-pass-invariants.md §1 (I-C3) |
| **Requirement** | Canonical fingerprint: LiveConsensusInputsCanonical.fingerprint is Blake2b-256 over a canonical CBOR encoding of every field in declared order. Same JSON bytes → same canonical form → same fingerprint, byte-identical across two import runs. The fingerprint is the load-bearing handle for every admission JSONL block-event (DC-ADMIT-10). |
| **Code** | crates/ade_runtime/src/consensus_inputs/canonical.rs (LiveConsensusInputsCanonical struct, encode_canonical_cbor private fn, canonical_from_raw lift, import_live_consensus_inputs sole authority) |
| **Tests** | `consensus_inputs::canonical::tests::import_round_trip_yields_canonical_form_with_fingerprint`; `consensus_inputs::canonical::tests::fingerprint_is_deterministic_across_repeated_imports`; `consensus_inputs::canonical::tests::fingerprint_changes_when_any_canonical_input_changes`; `consensus_inputs::canonical::tests::canonical_field_count_is_fifteen`; `consensus_inputs::canonical::tests::fingerprint_is_blake2b_256_of_encode_canonical_cbor` |
| **CI** | `ci/ci_check_live_consensus_inputs_fingerprint.sh` |

### DC-CONS

#### `DC-CONS-03` — _enforced_

| Aspect | Location |
|--------|----------|
| **Source** | Project constitution §3, ouroboros-consensus (pin at code-lock); CN-CONS-01 |
| **Requirement** | Praos chain selection ordering: block number first, then Praos TiebreakerView (slot, issuer, op-cert issue number, VRF output). Density-based ordering is reserved for Genesis/catch-up and must not be used for caught-up Praos fork-choice. |
| **Code** | crates/ade_core/src/consensus/fork_choice.rs, crates/ade_core/src/consensus/candidate.rs, crates/ade_runtime/src/consensus/candidate_fragment.rs |
| **Tests** | `consensus::fork_choice::tests::tiebreaker_prefer_lower_slot_wins`; `consensus::fork_choice::tests::tiebreaker_prefer_higher_op_cert_wins_on_equal_slot_and_issuer`; `consensus::fork_choice::tests::tiebreaker_prefer_lower_vrf_value_wins_on_full_tie`; `consensus::fork_choice::tests::no_candidates_returns_no_candidates_error`; `consensus::fork_choice::tests::equal_to_current_keeps_current_via_tiebreaker_loss`; `consensus::candidate::tests::tiebreaker_view_eq_is_field_wise`; `consensus::candidate::tests::candidate_fragment_carries_anchor_block_no`; `higher_block_no_wins`; `equal_block_no_tiebreaker_decides`; `fork_before_immutable_tip_rejected` … (+5 more) |
| **CI** | `ci/ci_check_no_density_in_fork_choice.sh`; `ci/ci_check_no_chaindb_in_consensus_blue.sh`; `ci/ci_check_no_float_in_consensus.sh`; `ci/ci_check_consensus_closed_enums.sh` |

#### `DC-CONS-04` — _enforced_

| Aspect | Location |
|--------|----------|
| **Source** | Project constitution §3, ouroboros-consensus PraosChainDepState |
| **Requirement** | Praos chain-dep state (evolving/candidate/epoch/previous_epoch/lab/last_epoch_block nonces, op-cert counters, last_slot) is owned by N-B consensus, not by the ledger, and evolves deterministically as a function of validated headers and epoch boundaries. |
| **Code** | crates/ade_core/src/consensus/praos_state.rs, crates/ade_core/src/consensus/events.rs, crates/ade_core/src/consensus/errors.rs, crates/ade_core/src/consensus/encoding.rs, crates/ade_core/src/consensus/nonce.rs, crates/ade_core/src/consensus/op_cert.rs, crates/ade_core/src/consensus/header_validate.rs, crates/ade_core/src/consensus/header_summary.rs |
| **Tests** | `consensus::praos_state::tests::op_cert_upsert_rejects_regression`; `consensus::praos_state::tests::op_cert_upsert_accepts_equal_counter_as_noop`; `consensus::praos_state::tests::op_cert_upsert_accepts_monotonic_increasing`; `consensus::praos_state::tests::genesis_state_is_well_formed`; `consensus::praos_state::tests::nonce_zero_constant_is_zero_bytes`; `consensus::encoding::tests::op_cert_counter_map_iteration_is_deterministic`; `layout_is_stable`; `roundtrip_empty_state`; `roundtrip_genesis_state`; `roundtrip_populated_state` … (+39 more) |
| **CI** | `ci/ci_check_consensus_closed_enums.sh` |

#### `DC-CONS-05` — _enforced_

| Aspect | Location |
|--------|----------|
| **Source** | Project constitution §3, ouroboros-consensus consensus report |
| **Requirement** | Authoritative rollback must never exceed the security parameter k measured in blocks (mainnet k = 2160). Rollback requests deeper than k return ExceededRollback. Forecast/stability windows may be slot-based but must accommodate at least k+1 blocks. |
| **Code** | crates/ade_core/src/consensus/rollback.rs, crates/ade_core/src/consensus/events.rs |
| **Tests** | `consensus::rollback::tests::rollback_preserves_immutable_tip`; `consensus::rollback::tests::rollback_preserves_security_param`; `consensus::rollback::tests::rollback_with_zero_depth_is_noop`; `consensus::rollback::tests::rollback_to_equal_block_no_as_immutable_succeeds`; `consensus::rollback::tests::rollback_to_one_below_immutable_rejected`; `rollback_within_k_succeeds`; `rollback_exceeding_k_rejected_with_typed_reason`; `rollback_before_immutable_tip_rejected`; `rollback_event_bytes_are_stable`; `rollback_is_deterministic` … (+1 more) |
| **CI** | `ci/ci_check_consensus_closed_enums.sh`; `ci/ci_check_no_chaindb_in_consensus_blue.sh` |

#### `DC-CONS-06` — _enforced_

| Aspect | Location |
|--------|----------|
| **Source** | Project constitution §3, T-DET-01 |
| **Requirement** | rollback(state, depth) produces state byte-identical to truncated replay from the nearest checkpoint. Rollback that would cross the immutable tip (≥ k deep) returns ForkBeforeImmutableTip and never alters state. |
| **Code** | crates/ade_core/src/consensus/rollback.rs |
| **Tests** | `rollback_equivalent_to_truncated_replay`; `rollback_is_deterministic`; `rollback_within_k_succeeds`; `rollback_before_immutable_tip_rejected`; `rollback_event_bytes_are_stable`; `consensus::rollback::tests::rollback_to_one_below_immutable_rejected`; `consensus::rollback::tests::rollback_preserves_immutable_tip` |
| **CI** | `ci/ci_check_consensus_closed_enums.sh` |

#### `DC-CONS-07` — _enforced_

| Aspect | Location |
|--------|----------|
| **Source** | Project constitution §3, DC-CORE-01, DC-EPOCH-02 |
| **Requirement** | BLUE consensus must consume the HFC schedule only as a typed EraSchedule value anchored to BootstrapAnchorHash. Genesis text parsing happens in RED; BLUE never reads files, JSON, or operator config directly. The schedule is part of replay evidence. |
| **Code** | crates/ade_core/src/consensus/era_schedule.rs, crates/ade_runtime/src/consensus/genesis_parser.rs |
| **Tests** | `consensus::era_schedule::tests::eraschedule_constructor_rejects_empty`; `consensus::era_schedule::tests::eraschedule_constructor_rejects_non_monotonic`; `consensus::era_schedule::tests::eraschedule_constructor_rejects_zero_slot_length`; `consensus::era_schedule::tests::eraschedule_constructor_rejects_zero_epoch_length`; `consensus::genesis_parser::tests::anchor_hash_deterministic`; `consensus::genesis_parser::tests::anchor_hash_distinguishes_inputs`; `mainnet_corpus_translation_matches_oracle`; `preprod_corpus_translation_matches_oracle`; `bootstrap_anchor_hash_distinguishes_genesis_variants`; `mainnet_parser_eras_match_corpus_oracle` … (+2 more) |
| **CI** | `ci/ci_check_no_chaindb_in_consensus_blue.sh` |

#### `DC-CONS-08` — _enforced_

| Aspect | Location |
|--------|----------|
| **Source** | Project constitution §3, CN-CONS-05, DC-CORE-01 |
| **Requirement** | slot_to_time(EraSchedule, SystemStart, SlotNo) is a pure function; no BLUE consensus path may consult the wall clock to derive a slot or UTC instant for an authoritative decision. |
| **Code** | crates/ade_core/src/consensus/era_schedule.rs |
| **Tests** | `consensus::era_schedule::tests::slot_to_time_monotone_increasing`; `consensus::era_schedule::tests::slot_to_time_overflow_returns_structured_error`; `consensus::era_schedule::tests::determinism_across_runs`; `consensus::era_schedule::tests::locate_first_slot_of_each_era`; `consensus::era_schedule::tests::locate_last_slot_of_each_era`; `consensus::era_schedule::tests::locate_before_system_start_errors`; `mainnet_corpus_translation_matches_oracle`; `preprod_corpus_translation_matches_oracle` |
| **CI** | `ci/ci_check_no_float_in_consensus.sh` |

#### `DC-CONS-09` — _enforced_

| Aspect | Location |
|--------|----------|
| **Source** | Project constitution §3, DC-EPOCH-02 |
| **Requirement** | Consensus-derived queries for slots beyond the ledger-view safe zone return OutsideForecastRange, never guessed values. The bound is derived from era history + safe zone + HFC schedule, not encoded as a magic constant in caller code. |
| **Code** | crates/ade_core/src/consensus/era_schedule.rs, crates/ade_core/src/consensus/leader_schedule.rs |
| **Tests** | `consensus::era_schedule::tests::forecast_horizon_boundary`; `mainnet_corpus_translation_matches_oracle`; `preprod_corpus_translation_matches_oracle`; `consensus::leader_schedule::tests::query_returns_outside_forecast_range_for_far_future`; `corpus_rejects_out_of_forecast_horizon` |
| **CI** | `ci/ci_check_consensus_closed_enums.sh` |

#### `DC-CONS-10` — _enforced_

| Aspect | Location |
|--------|----------|
| **Source** | Project constitution §3, ouroboros-consensus OperationalCertificate |
| **Requirement** | A header's op-cert issue counter must be >= the highest observed counter for the same (pool, kes_period). Regression yields HeaderInvalid with a typed OpCertCounterError reason; ChainDepState never accepts a regression. |
| **Code** | crates/ade_core/src/consensus/op_cert.rs, crates/ade_core/src/consensus/praos_state.rs, crates/ade_core/src/consensus/errors.rs, crates/ade_core/src/consensus/header_validate.rs |
| **Tests** | `consensus::op_cert::tests::apply_op_cert_inserts_first_observation`; `consensus::op_cert::tests::apply_op_cert_advances_existing_strictly`; `consensus::op_cert::tests::apply_op_cert_accepts_equal_counter_as_noop`; `consensus::op_cert::tests::apply_op_cert_rejects_lower_counter`; `consensus::op_cert::tests::apply_op_cert_independent_kes_periods_dont_collide`; `consensus::op_cert::tests::apply_op_cert_independent_pools_dont_collide`; `consensus::op_cert::tests::apply_op_cert_does_not_touch_nonces`; `consensus::op_cert::tests::apply_op_cert_does_not_touch_last_slot_or_block_no`; `consensus::praos_state::tests::op_cert_upsert_rejects_regression`; `consensus::praos_state::tests::op_cert_upsert_accepts_equal_counter_as_noop` … (+6 more) |
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
| **Code** | crates/ade_ledger/src/block_body_hash.rs (block_body_hash, block_body_hash_from_buckets — single canonical authority); crates/ade_ledger/src/producer/forge.rs (forge_block — consumer); crates/ade_ledger/src/block_validity/header_input.rs (computed_body_hash + accepted_block_header_bytes — consumer + N-G-strengthened single header-projection authority); crates/ade_runtime/src/producer/served_chain_lookups.rs (ServedHeaderLookup::next_after — third consumer; reuses accepted_block_header_bytes for producer-side server-pump header projection); crates/ade_ledger/src/receive/reducer.rs (PHASE4-N-H strengthening: receive-side BlockDelivered branch decodes block via the same block_validity recipe; header cross-check via decoded.block_hash binds header content to the (slot, hash) cache key) |
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
| **Requirement** | Receive-side header-body sourcing coherence: when BlockDelivered {block_bytes} arrives at the receive bridge, the decoded header bytes of block_bytes equal the header_bytes cached from the most recent RollForward at the same (slot, hash). A peer cannot switch headers between announcement and body delivery. |
| **Code** | crates/ade_ledger/src/receive/reducer.rs (block_delivered helper: decodes the body, looks up cache at (slot, block_hash); HeaderBodyMismatch if absent) |
| **Tests** | `receive_apply_block_delivered_with_no_cached_header_rejects`; `receive_apply_block_delivered_with_mismatched_cached_header_rejects`; `receive_apply_block_delivered_with_matching_header_admits` |
| **CI** | `ci/ci_check_receive_reducer_closure.sh` |

#### `DC-CONS-20` — _enforced_

| Aspect | Location |
|--------|----------|
| **Source** | docs/planning/receive-side-bridge-invariants.md §1 (I-3, I-4); docs/planning/ledger-snapshot-rollback-invariants.md §1 (I-6) |
| **Requirement** | ChainDb-ledger-chain_dep lockstep: a successful receive-side admission updates ChainDb, LedgerState, and PraosChainDepState as one structural transition. A successful RollBackward rolls back all three to the same slot. No path leaves them out of sync; no partial admission; no partial rollback. |
| **Code** | crates/ade_ledger/src/receive/reducer.rs (admit-side: block_delivered branch atomically advances chain_write + state.ledger + state.chain_dep; rollback-side: roll_backward branch atomically calls materialize_rolled_back_state + commit_rollback); crates/ade_ledger/src/rollback/commit.rs (commit_rollback irreversible-step-first staged commit); crates/ade_ledger/src/rollback/materialize.rs (materialize_rolled_back_state via SnapshotReader + BlockSource); crates/ade_runtime/src/rollback/{cadence,in_memory_cache,chaindb_block_source,snapshot_writer}.rs (GREEN/RED rollback infrastructure) |
| **Tests** | `receive_apply_block_delivered_with_matching_header_admits`; `commit_rollback_advances_chaindb_and_ledger_atomically`; `commit_rollback_chain_write_failure_leaves_state_unchanged`; `commit_rollback_resets_pending_headers`; `rollback_branch_returns_rolled_back_on_in_memory_snapshot`; `rollback_branch_returns_rollback_too_deep_when_no_snapshot`; `rollback_branch_state_unchanged_on_materialize_failure`; `rollback_then_continue_admit_equals_straight_line_admit` |
| **CI** | `ci/ci_check_receive_reducer_closure.sh`; `ci/ci_check_rollback_materialize_closure.sh`; `ci/ci_check_receive_orchestrator_no_producer_dep.sh` |

#### `DC-CONS-21` — _enforced_

| Aspect | Location |
|--------|----------|
| **Source** | docs/planning/ledger-snapshot-rollback-invariants.md §1 (I-1, I-2, I-9) |
| **Requirement** | Snapshot encode/decode round-trip equivalence: for any reachable (LedgerState, PraosChainDepState), decode(encode(state)) yields a state whose ade_ledger::fingerprint::fingerprint matches the original's. Encoder is canonical (BTreeMap iteration, no HashMap, no floats, no wall-clock); encoded bytes start with a closed version tag and embed the source state's fingerprint for decode- side cross-check. |
| **Code** | crates/ade_ledger/src/snapshot/{framing,ledger,chain_dep,utxo_state,cert_state,epoch_state,gov_state}.rs + crates/ade_runtime/src/rollback/persistent_cache.rs |
| **Tests** | `snapshot::framing::tests::snapshot_round_trip`; `snapshot::framing::tests::round_trip_via_fingerprint_combined`; `snapshot::ledger::tests::encode_then_decode_roundtrips_via_fingerprint`; `rollback::persistent_cache::tests::persistent_cache_capture_then_nearest_le_round_trips`; `rollback::persistent_cache::tests::persistent_cache_matches_in_memory_cache_semantics` |
| **CI** | `ci/ci_check_snapshot_encoder_closure.sh` |

#### `DC-CONS-22` — _enforced_

| Aspect | Location |
|--------|----------|
| **Source** | docs/planning/ledger-snapshot-rollback-invariants.md §1 (I-3, I-4) |
| **Requirement** | Replay-forward correctness: given state_at_slot_S and the ordered block sequence blocks(S+1..=T) from ChainDb, the replay-forward driver yields a state whose fingerprint matches the state that would result from applying those blocks via apply_block_with_verdicts in normal forward operation. Snapshot+ replay-forward is a pure cache for direct-apply; never an authoritative side path. Replay-forward MUST honor the unique epoch-boundary authority for any range crossing one (¬P-9). |
| **Code** | crates/ade_ledger/src/rollback/materialize.rs (materialize_rolled_back_state: pure replay-forward fold over block_validity; epoch boundaries handled implicitly by apply_block_with_verdicts per rules.rs:244-250) |
| **Tests** | `materialize_with_snapshot_at_target_returns_snapshot_state`; `materialize_with_snapshot_below_target_replays_forward`; `materialize_replay_forward_equals_direct_apply`; `materialize_fails_closed_on_invalid_block` |
| **CI** | `ci/ci_check_rollback_materialize_closure.sh` |

#### `DC-CONS-23` — _enforced_

| Aspect | Location |
|--------|----------|
| **Source** | docs/planning/phase4-n-u-forged-block-durability-invariants.md |
| **Requirement** | Own-forged stale-tip race safety by extend-only durable admit. An own-forged candidate is admitted to the durable tip ONLY if it EXTENDS the current durable tip at admit time, through the existing EXTEND-ONLY durable admit validation path (receive_apply -> admit_via_block_validity -> block_validity, incl. header_position). If a feed block advanced the tip after forge time (the forge<->feed race), the forged candidate FAILS CLOSED -- via header-position / prev_hash validation, TipBeforeDurable, or WAL prior_fp mismatch -- and the next forge tick re-forges on the current durable tip. N-U adds NO admit-time fork-choice and NO own-block override path. DC-CONS-03 (select_best_chain) remains the fork-choice authority in the follow / chain_selector paths -- NOT the durable admit. |
| **Code** | crates/ade_ledger/src/receive/{reducer,admitted}.rs (extend-only admit_via_block_validity -- reused); crates/ade_ledger/src/block_validity/header_position.rs (prev_hash/position fail-closed -- reused); crates/ade_node/src/node_sync.rs (admit_forged_block_durably -> pump_block, no admit-time fork-choice) |
| **Tests** | `stale_tip_forge_fails_closed` |
| **CI** | `ci/ci_check_forged_durable_admit_via_pump.sh` |

#### `DC-CONS-24` — _enforced_

| Aspect | Location |
|--------|----------|
| **Source** | docs/clusters/PHASE4-N-AE/cluster.md; docs/clusters/PHASE4-N-AE/slices/AE.A.md |
| **Requirement** | Forged parent hash byte-equals the peer-visible selected tip. The forged successor's prev_hash byte-equals the followed peer tip hash AND its block_no == followed_tip.block_no + 1. Parent identity is the canonical hash, never inferred from block number alone. |
| **Code** | crates/ade_node/src/node_lifecycle.rs (run_relay_loop_with_sched ForgeTick CaughtUp arm forges on selected_tip == the durable servable tip == the followed peer tip); crates/ade_node/src/node_sync.rs (forge_header_position sets prev_hash = PrevHash::Block(selected_tip.hash)) |
| **Tests** | `forge_on_followed_tip_proceeds_with_parent_byte_equal` |
| **CI** | `ci/ci_check_forge_followed_tip_admission.sh` |

### DC-CONSENSUS

#### `DC-CONSENSUS-01` — _enforced_

| Aspect | Location |
|--------|----------|
| **Source** | Project constitution §3, T-DET-01, T-CORE-01 |
| **Requirement** | Chain selection is deterministic and matches Haskell node behavior |
| **Code** | crates/ade_core/src/consensus/fork_choice.rs, crates/ade_core/src/consensus/candidate.rs, crates/ade_core/src/consensus/rollback.rs, crates/ade_runtime/src/consensus/chain_selector.rs, crates/ade_core_interop/src/lib.rs, crates/ade_core_interop/src/bin/live_consensus_session.rs |
| **Tests** | `higher_block_no_wins`; `equal_block_no_tiebreaker_decides`; `fork_before_immutable_tip_rejected`; `exceeded_rollback_rejected`; `tiebreaker_loss_keeps_current`; `replay_is_deterministic`; `reject_reason_bytes_are_stable`; `no_candidates_returns_error`; `consensus::fork_choice::tests::tiebreaker_prefer_lower_slot_wins`; `consensus::fork_choice::tests::tiebreaker_prefer_higher_op_cert_wins_on_equal_slot_and_issuer` … (+16 more) |
| **CI** | `ci/ci_check_no_density_in_fork_choice.sh`; `ci/ci_check_consensus_closed_enums.sh`; `ci/ci_check_no_chaindb_in_consensus_blue.sh` |

#### `DC-CONSENSUS-02` — _partial_

| Aspect | Location |
|--------|----------|
| **Source** | Project constitution §3, T-CORE-01 |
| **Requirement** | Leadership verification is pure |
| **Code** | crates/ade_core/src/consensus/leader_schedule.rs, crates/ade_core/src/consensus/ledger_view.rs, crates/ade_core/src/consensus/vrf_cert.rs, crates/ade_ledger/src/consensus_view.rs |
| **Tests** | `consensus::leader_schedule::tests::query_uses_state_epoch_nonce_for_vrf_input`; `consensus::leader_schedule::tests::query_returns_unknown_pool_when_no_vrf_key`; `consensus::leader_schedule::tests::query_returns_outside_forecast_range_for_far_future`; `consensus::leader_schedule::tests::query_does_not_mutate_state`; `consensus::leader_check::tests::eligible_on_threshold_with_high_stake_emits_eligible_verdict`; `consensus::leader_check::tests::not_eligible_with_zero_stake_emits_not_eligible_verdict`; `corpus_returns_canonical_answer_for_known_pools`; `corpus_rejects_unknown_pool`; `corpus_rejects_out_of_forecast_horizon`; `corpus_is_leader_helper_matches_pinned_probe` … (+6 more) |
| **CI** | _(no CI script listed)_ |

### DC-CORE

#### `DC-CORE-01` — _enforced_

| Aspect | Location |
|--------|----------|
| **Source** | Project constitution §2, T-CORE-02; PHASE4-N-A scope decisions §Decision 2 (docs/active/PHASE4-N-A_scope_decisions.md) |
| **Requirement** | BLUE authoritative crates are sync-only: no async fn, .await, tokio::, async_std::, Future, futures::, task spawning, async channels, or timers. Async runtime concerns are confined to RED transport/runtime code. |
| **Code** | crates/ade_runtime/src/consensus/chain_selector.rs, crates/ade_core_interop/src/lib.rs, crates/ade_core_interop/src/bin/live_consensus_session.rs |
| **Tests** | `consensus::chain_selector::tests::header_arrival_updates_state_and_selector`; `consensus::chain_selector::tests::rollback_walks_back_via_recent_snapshots`; `consensus::chain_selector::tests::rollback_to_block_older_than_snapshots_rejected`; `consensus::chain_selector::tests::epoch_boundary_emits_no_event`; `cardano_node_session_sustained_window` |
| **CI** | `ci/ci_check_no_async_in_blue.sh` |

### DC-CRYPTO

#### `DC-CRYPTO-01` — _enforced_

| Aspect | Location |
|--------|----------|
| **Source** | Project constitution §3, T-CORE-01, T-DET-01 |
| **Requirement** | Crypto verification is pure and matches Haskell node on all test vectors |
| **Code** | crates/ade_crypto/src/, crates/ade_core/src/consensus/vrf_cert.rs (Praos single-VRF input + leader/nonce range extension; PHASE4-B1-S5), crates/ade_core/src/consensus/kes_check.rs (KES + op-cert wiring; PHASE4-B1-S5) |
| **Tests** | `blake2b_256_empty`; `blake2b_256_abc`; `blake2b_256_single_byte`; `blake2b_256_multi_block`; `blake2b_256_large`; `libsodium_vector_empty_message`; `libsodium_vector_single_byte`; `libsodium_vector_two_byte`; `libsodium_vector_longer_message`; `libsodium_vector_hash_size_message` … (+9 more) |
| **CI** | `ci/ci_check_crypto_vectors.sh` |

#### `DC-CRYPTO-02` — _enforced_

| Aspect | Location |
|--------|----------|
| **Source** | Project constitution §3, T-BOUND-01 |
| **Requirement** | All signing operations confined to shell |
| **Code** | crates/ade_crypto/src/lib.rs |
| **Tests** | _(no tests listed — gap)_ |
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
| **Tests** | `parse_round_trips_serialize`; `parse_round_trips_at_nonzero_period`; `parse_rejects_unknown_format`; `parse_rejects_wrong_role`; `parse_rejects_unsupported_crypto`; `parse_rejects_unsupported_format_version`; `parse_rejects_missing_seed_32`; `parse_rejects_malformed_seed_32_length`; `parse_rejects_uppercase_seed_hex`; `parse_rejects_period_idx_overflow` … (+6 more) |
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
| **Tests** | `sum0_kes_signs_and_verifies_at_period_0`; `sum0_kes_rejects_period_1`; `sum0_kes_update_expires_after_period_0`; `sum0_kes_verify_rejects_wrong_message`; `sum1_kes_signs_at_period_0_and_period_1`; `sum6_kes_total_periods_is_64`; `sum6_kes_sizes_match_recurrence`; `sum6_kes_chain_advances_through_all_64_periods`; `sum6_kes_update_after_period_63_expires`; `sum6_kes_sign_rejects_period_64` … (+8 more) |
| **CI** | `ci/ci_check_kes_sum_compatibility.sh`; `ci/ci_check_private_key_custody.sh` |

#### `DC-CRYPTO-09` — _enforced_

| Aspect | Location |
|--------|----------|
| **Source** | docs/clusters/PHASE4-N-P/cluster.md; docs/planning/phase4-n-p-invariants.md §1 (I3, I5); §2 (N2, N3, N4, N5); docs/clusters/PHASE4-N-P/period-from-zeroed-sum6-tree-shape-proof.md |
| **Requirement** | Sum6KES expanded signing-key serde and period inference. `raw_serialize_signing_key_kes` / `raw_deserialize_signing_key_kes` are byte-identical to Haskell's `rawSerialiseSignKeyKES` / `rawDeserialiseSignKeyKES` for `Sum6KES Ed25519DSIGN`. The on-disk format is exactly 608 bytes; any other payload size fails closed via `KesParseError::WrongPayloadSize`. `current_period` is uniquely inferable from which sub-seeds are zeroed in the tree (no heuristic; exactly one valid period or a closed parse error) — implemented per the proof obligation at `docs/clusters/PHASE4-N-P/period-from-zeroed-sum6-tree-shape-proof.md`. Round-trip preserves period; serialize → deserialize → serialize yields byte-identical output for every period 0..=63. Malformed sub-trees (truncated child skey, wrong VK length at any recursion level, inconsistent vk0/vk1 hashes, leaf-all-zero) → closed `KesParseError` variant; no best-effort guesswork. |
| **Code** | crates/ade_crypto/src/kes_sum/period.rs (period_from_zeroed_sum6_tree_shape); crates/ade_crypto/src/kes_sum/sum.rs (raw_serialize/raw_deserialize_signing_key_kes); crates/ade_crypto/src/kes_sum/single.rs (Sum0 leaf serde); crates/ade_crypto/src/kes_sum/errors.rs (KesParseError closed surface) |
| **Tests** | `sum6_raw_serialize_signing_key_kes_size_is_608`; `sum6_raw_serialize_signature_kes_size_is_448`; `sum6_skey_round_trip_at_every_period_0_to_63`; `sum6_signature_round_trip_at_every_period`; `period_from_zeroed_sum6_tree_shape_agrees_with_update_kes_chain`; `period_from_zeroed_sum6_tree_shape_rejects_leaf_all_zero`; `raw_deserialize_signing_key_kes_rejects_wrong_payload_size`; `raw_deserialize_signing_key_kes_rejects_leaf_all_zero`; `raw_deserialize_signing_key_kes_rejects_inconsistent_vk_left_at_level_6`; `raw_deserialize_signing_key_kes_rejects_inconsistent_vk_right_at_level_6` … (+5 more) |
| **CI** | `ci/ci_check_kes_sum_compatibility.sh` |

#### `DC-CRYPTO-10` — _enforced_

| Aspect | Location |
|--------|----------|
| **Source** | docs/clusters/PHASE4-N-AC/cluster.md; docs/evidence/c1-genesis-rehearsal-reproduction-README.md (item-4 C1 re-run finding) |
| **Requirement** | The RED signing shell must evolve the operator KES signing key to the requested KES period before signing, using the existing deterministic Sum6KES update primitive. It must fail closed if the requested period is before the key start, beyond the key lifetime, or cannot be reached by sequential evolution. The slot->KES-period gate (CoordinatorState::kes_period_for_slot) supplying that requested period MUST return the RELATIVE evolution index anchored at the op-cert's start period (absolute_period - kes_start_period), valid only inside the op-cert's covered window [kes_start_period, kes_start_period + kes_max_period] -- NEVER the raw absolute period (which would exceed the key lifetime and refuse every real-chain slot). |
| **Code** | crates/ade_runtime/src/producer/producer_shell.rs (kes_sign_header_advancing = kes_advance_to(period) then kes_sign_header; kes_advance_to -> kes_update fail-closed: EvolutionBackwards / EvolutionExhausted); crates/ade_node/src/produce_mode.rs (the forge's single real KES sign uses kes_sign_header_advancing); crates/ade_runtime/src/producer/coordinator.rs (kes_period_for_slot returns the opcert-anchored RELATIVE evolution, not the absolute period; consumed by node_lifecycle.rs ForgeTick) |
| **Tests** | `crates/ade_runtime/src/producer/producer_shell.rs::tests::shell_kes_sign_header_advancing_evolves_then_signs`; `crates/ade_runtime/src/producer/producer_shell.rs::tests::shell_kes_sign_header_advancing_at_current_period_signs`; `crates/ade_runtime/src/producer/producer_shell.rs::tests::shell_kes_sign_header_advancing_backwards_fails_closed`; `crates/ade_runtime/src/producer/producer_shell.rs::tests::shell_kes_sign_header_advancing_beyond_lifetime_fails_closed`; `crates/ade_runtime/src/producer/coordinator.rs::tests::kes_period_for_slot_anchors_relative_to_opcert_start_period` |
| **CI** | `ci/ci_check_kes_evolution_before_sign.sh` |

### DC-DIFF

#### `DC-DIFF-01` — _partial_

| Aspect | Location |
|--------|----------|
| **Source** | Project constitution §3, T-DET-01, DC-REF-01 |
| **Requirement** | Differential harness must localize first divergence point between Ade and reference oracle |
| **Code** | crates/ade_testkit/src/harness/ledger_diff.rs |
| **Tests** | `diff_ledger_sequence`; `mismatched_block_count_returns_error` |
| **CI** | _(no CI script listed)_ |

### DC-EPOCH

#### `DC-EPOCH-01` — _partial_

| Aspect | Location |
|--------|----------|
| **Source** | Project constitution §3, T-CAUSAL-01, T-EPOCH-01 |
| **Requirement** | Conway governance timing: proposals accumulate during epoch, ratification and enactment are atomic at epoch boundary, pulsing distributes DRep stake computation across epoch |
| **Code** | crates/ade_ledger/src/governance.rs (enact_proposals + apply_committee_enactment), crates/ade_ledger/src/rules.rs (epoch-boundary apply site) |
| **Tests** | `conway_epoch_boundary_end_to_end`; `conway_governance_ratification_test`; `enact_noconfidence_dissolves_committee`; `enact_update_committee_applies_changes`; `committee_enactment_replays_byte_identical`; `epoch_boundary_ratifiable_noconfidence_is_terminal_pending_enactment`; `committee_oracle_mainnet_575_576_noop_agreement` |
| **CI** | `ci/ci_check_credential_discriminant_closed.sh` |

#### `DC-EPOCH-02` — _enforced_

| Aspect | Location |
|--------|----------|
| **Source** | Project constitution §3, T-DET-01, T-CORE-03 |
| **Requirement** | Hard fork transitions triggered at deterministic slot/epoch boundaries; era translation functions mandatory; forecast horizon extends to era boundary |
| **Code** | crates/ade_ledger/src/hfc.rs, crates/ade_core/src/consensus/era_schedule.rs |
| **Tests** | `translation_summary_proof::shelley_allegra_summary_matches_oracle`; `translation_comparison_surface::all_non_byron_translations_preserve_sub_state`; `transition_proof_surface::shelley_allegra_transition_proof_surface`; `transition_proof_surface::all_transitions_proof_surface_summary`; `consensus::era_schedule::tests::locate_first_slot_of_each_era`; `consensus::era_schedule::tests::locate_last_slot_of_each_era`; `consensus::era_schedule::tests::forecast_horizon_boundary`; `mainnet_corpus_translation_matches_oracle`; `preprod_corpus_translation_matches_oracle` |
| **CI** | `ci/ci_check_hfc_translation.sh` |

#### `DC-EPOCH-03` — _enforced_

| Aspect | Location |
|--------|----------|
| **Source** | docs/planning/phase4-n-f-g-invariants.md |
| **Requirement** | Single-epoch forge containment on the --mode node spine: in this forge path, a forge is valid only within the single recovered seed epoch. A candidate forge slot beyond the recovered seed epoch FAILS CLOSED -- it cannot be forged, served, or signed as a valid block. The seed-epoch nonce (eta0) is frozen at the recovered value; the forge apply path does NOT drive the BLUE CandidateFreeze / EpochBoundary nonce transitions (they exist in ade_core::consensus::nonce but nothing drives them on the forge path), so signing past the boundary with a stale eta0 is a peer-reject class and is forbidden, not silently attempted. Cross-epoch production (nonce roll + epoch transition driven from an epoch-aware tick + follow/durability) is a SEPARATE nonce-roll/epoch-transition cluster, NOT this one. The off-epoch slot fails closed with a structured local outcome (a fail-closed boundary for this single-epoch forge path, hardening DC-NODE-05's single-epoch cluster-scope containment for the live-serve path). |
| **Code** | crates/ade_node/src/node_sync.rs, crates/ade_node/src/node_lifecycle.rs, crates/ade_node/src/run_loop_planner.rs, crates/ade_core/src/consensus/nonce.rs |
| **Tests** | `forge_epoch_admission_within_seed_epoch_admits`; `forge_epoch_admission_off_epoch_fails_closed`; `forge_epoch_admission_unlocatable_fails_closed`; `node_forge_off_epoch_slot_fails_closed`; `node_forge_no_epoch_boundary_promotion_on_forge_path`; `forge_tick_off_epoch_slot_fails_closed_local` |
| **CI** | `ci/ci_check_node_forge_single_epoch_fail_closed.sh` |

#### `DC-EPOCH-04` — _enforced_

| Aspect | Location |
|--------|----------|
| **Source** | docs/clusters/EPOCH-CONSENSUS-VIEW/SLICE-3f4-activation-flip.md (S3f-4a); user directive 2026-06-21 (one atomic WAL-backed activation; explicit activation idempotence) |
| **Requirement** | For a target epoch, AT MOST ONE canonically bound EpochConsensusView may activate (S3f-4a substrate). A distinct WalEntry::EpochConsensusViewActivated (append-only TAG 4) is the durable proof that THIS exact view became authoritative for `target_epoch` at THIS exact selected-chain transition; it records the ENTIRE activation identity -- target_epoch, network_magic, era, transition_point, source_checkpoint_commitment, snapshot_phase, nonce_commitment, stake_view_canonical_hash, full view_canonical_hash -- not merely hash + point. Like SeedEpochConsensusInputsImported it carries no prior_fp/post_fp and never advances the fingerprint chain. Activation idempotence is EXPLICIT and does NOT weaken the seed's DuplicateProvenance: activation_replay_outcome(existing, new) for the SAME target epoch returns Idempotent iff the records are byte-identical (structural equality of the whole record), else Conflict (fail closed). The record canonically encodes/decodes byte-identically (replay-equivalent). |
| **Code** | crates/ade_ledger/src/wal/event.rs: WalEntry::EpochConsensusViewActivated{target_epoch,network_magic,era,transition_point,source_checkpoint_commitment,snapshot_phase,nonce_commitment,stake_view_canonical_hash,view_canonical_hash} + TAG_EPOCH_CONSENSUS_VIEW_ACTIVATED=4 + canonical encode/decode (snapshot_phase_wire/era ALL-tag) + activation_replay_outcome -> ActivationReplayOutcome{Idempotent\|Conflict}. crates/ade_ledger/src/reduced_epoch_view.rs: EpochConsensusView::stake_view_canonical_hash. ci/ci_check_eview_activation_wal.sh. |
| **Tests** | `wal_epoch_view_activated_round_trips_byte_identical`; `wal_epoch_view_activated_uses_tag_four`; `activation_replay_idempotent_vs_conflict` |
| **CI** | `ci/ci_check_eview_activation_wal.sh` |

#### `DC-EPOCH-05` — _enforced_

| Aspect | Location |
|--------|----------|
| **Source** | docs/clusters/EPOCH-CONSENSUS-VIEW/SLICE-3f4-activation-flip.md (S3f-4b); user directive 2026-06-21 (no runtime flag; promoted view fully replaces the seed view) |
| **Requirement** | Epoch N+1 validation and leadership may NOT observe epoch-N seed inputs (S3f-4b). The active epoch view is a ONE-WAY ActiveEpochView transition: `Seed` (the recovered seed view is authoritative) -> `Promoted(view)` (the bound N+1 view is authoritative). There is NO config that selects between them -- the activation predicate is the gate, not a flag. `ActiveEpochView::promoted()` is `Some` ONLY after a promotion, so N+1 leadership reading it can NEVER observe the seed inputs as if they were the N+1 view; before promotion it is `None` (the seed is authoritative on its own seed-epoch path). A re-promotion with the SAME view is idempotent; with a DIFFERENT view it is the terminal EpochViewActivationConflict (never a silent swap). |
| **Code** | crates/ade_node/src/epoch_activation.rs: ActiveEpochView{Seed\|Promoted(EpochConsensusView)} -- one-way promote() (Seed->Promoted; same-view idempotent; differing-view -> EpochViewActivationConflict), promoted()->Option (Some only post-promotion), is_promoted(). ci/ci_check_eview_activation_predicate.sh. |
| **Tests** | `active_view_one_way_promote_and_idempotence`; `active_view_conflicting_promotion_is_terminal`; `seed_exposes_no_n1_view_until_promotion` |
| **CI** | `ci/ci_check_eview_activation_predicate.sh` |

#### `DC-EPOCH-06` — _enforced_

| Aspect | Location |
|--------|----------|
| **Source** | docs/clusters/EPOCH-CONSENSUS-VIEW/SLICE-3f4-activation-flip.md (S3f-4c); user directive 2026-06-21 (durable-before-visible; crash before/after WAL; recovered must match WAL) |
| **Requirement** | Activation is durable-before-visible and replay-identical (S3f-4c). The activation WAL record (EpochConsensusViewActivated) is written and made durable BEFORE the active view is published: activate_durable_before_visible publishes Promoted(view) ONLY when the WAL write is durable; a non-durable write is the terminal EpochViewActivationFailed (halt before promotion), never a publish. Recovery is replay-identical: recover_active_view(record, candidate) returns Seed when there is NO durable activation record (crash before the WAL: the old epoch stays active); Promoted(candidate) when a record exists AND the re-derived candidate reproduces its ENTIRE identity -- every binding + the stake-view hash + the full-view hash + verify_canonical_hash, via activation_record_matches -- so the recovered active view equals the WAL record (crash after the WAL or after publication); and the terminal EpochViewPostPromotionMismatch when a record exists but the candidate mismatches or cannot be re-derived (NEVER a fallback to the epoch-wrong seed view). resolve_activation_record folds repeated records via the DC-EPOCH-04 idempotence/conflict rule (same epoch byte-identical => keep; differing => terminal conflict; a different epoch => the later supersedes). |
| **Code** | crates/ade_node/src/epoch_activation.rs: activate_durable_before_visible(candidate, wal_write_durable) (durable->Promoted, else EpochViewActivationFailed); recover_active_view(record, candidate) (None->Seed; match->Promoted; else EpochViewPostPromotionMismatch); activation_record_matches (complete identity incl. both hashes + verify); activation_record_for (the record builder); resolve_activation_record (DC-EPOCH-04 fold). ci/ci_check_eview_activation_recovery.sh. |
| **Tests** | `crash_before_durable_wal_keeps_seed`; `crash_after_wal_republishes_same_view`; `recovered_view_mismatch_is_terminal`; `durable_before_visible_halts_on_wal_failure`; `resolve_activation_idempotent_conflict_supersede` |
| **CI** | `ci/ci_check_eview_activation_recovery.sh` |

#### `DC-EPOCH-07` — _enforced_

| Aspect | Location |
|--------|----------|
| **Source** | docs/clusters/EPOCH-CONSENSUS-VIEW/SLICE-3f4-activation-flip.md (S3f-4b); user directive 2026-06-21 (halt on activation failure or mismatch; no seed-view fallback) |
| **Requirement** | A missing / stale / conflicting / mismatched candidate view causes TERMINAL fail-closed behaviour, NEVER fallback consensus (S3f-4b). The activation predicate (activation_predicate) is the only gate: Promote requires transition-eligible AND bindings-verify AND selected-point-correct AND wal-durable, in that order; any failure is NoPromotion(ActivationReject) -- the seed view simply stays authoritative on its seed-epoch path (no flag, no fallback to an epoch-WRONG view past the boundary). The terminal states are EpochViewActivationError{EpochViewActivationFailed (WAL not durable -> halt before promotion), EpochViewActivationConflict (a conflicting activation for the target epoch -> halt), EpochViewPostPromotionMismatch (the active view != the WAL record after publication -> halt)}. Falling back to the seed view past the boundary is forbidden because it is known epoch-wrong and header validation / follow could then observe stale inputs. |
| **Code** | crates/ade_node/src/epoch_activation.rs: activation_predicate(candidate, n1_bindings, selected_point, transition_eligible, wal_durable) -> ActivationOutcome{Promote\|NoPromotion(ActivationReject{TransitionIneligible\|BindingsUnverified\|WrongSelectedPoint\|WalNotDurable})}; EpochViewActivationError{EpochViewActivationFailed\|EpochViewActivationConflict\|EpochViewPostPromotionMismatch}. ci/ci_check_eview_activation_predicate.sh. |
| **Tests** | `predicate_promotes_only_when_every_precondition_holds`; `predicate_rejects_each_failed_precondition`; `active_view_conflicting_promotion_is_terminal` |
| **CI** | `ci/ci_check_eview_activation_predicate.sh` |

#### `DC-EPOCH-08` — _enforced_

| Aspect | Location |
|--------|----------|
| **Source** | docs/clusters/EPOCH-CONSENSUS-VIEW/SLICE-3f4d-live-flip.md (S3f-4d-1); user directive 2026-06-21 (durable ChainDB source only; named roles; the Mark/Set lag is a proof obligation) |
| **Requirement** | The activation SOURCE WINDOW is named-role-typed, durable-lineage-pinned, and complete/ordered/bounded (S3f-4d-1). The window that produces an activation candidate is NOT generically "epoch N": the completed epoch whose admitted blocks the window drives over (source_epoch) is distinct from the epoch whose leadership reads the activated view (target_epoch), related by the Cardano Mark->Set snapshot lag. ActivationSourceWindow names every role (source_epoch, source_window_start, source_window_end, snapshot_phase, target_epoch, source_window_anchor, lineage_pin). The lag lives in ONE named constant LEADERSHIP_SNAPSHOT_LAG_EPOCHS (a PROOF OBLIGATION), applied only via target_epoch_for_source -- never an inline source+k. validate_source_window fails closed unless the blocks are: non-empty; within [start, end] (bounded); strictly increasing by slot (ordered, no duplicate); a CONTIGUOUS chain (block[0].prev_hash == source_window_anchor, block[i].prev_hash == block[i-1].hash -- so no block is missing = complete); the last block's hash == lineage_pin (pinned to the selected ChainDB lineage tip); and target_epoch == the explicit source->target mapping. NO peer/network read, wall-clock, or async side channel influences the window. |
| **Code** | crates/ade_node/src/epoch_source_window.rs: ActivationSourceWindow{source_epoch,source_window_start,source_window_end,snapshot_phase,target_epoch,source_window_anchor,lineage_pin}; LEADERSHIP_SNAPSHOT_LAG_EPOCHS (the single lag constant, proof obligation) + target_epoch_for_source; validate_source_window -> SourceWindowError{Empty\|OutOfWindow\|NotOrdered\|Duplicate\|AnchorMismatch\|ChainGap\|LineageMismatch\|TargetEpochMismatch}. ci/ci_check_eview_source_window.sh. |
| **Tests** | `target_epoch_is_the_explicit_lag`; `valid_window_passes`; `empty_window_fails_closed`; `out_of_window_block_fails_closed`; `unordered_and_duplicate_fail_closed`; `missing_block_breaks_the_chain`; `anchor_and_lineage_pin_fail_closed`; `wrong_target_epoch_fails_closed` |
| **CI** | `ci/ci_check_eview_source_window.sh` |

#### `DC-EPOCH-09` — _enforced_

| Aspect | Location |
|--------|----------|
| **Source** | docs/clusters/EPOCH-CONSENSUS-VIEW/SLICE-3f4d-live-flip.md (S3f-4d-2) |
| **Requirement** | The activation candidate is derived ONLY from a validated source window, bound to the TARGET-epoch context (S3f-4d-2). derive_candidate drives the reduced checkpoint + cert state forward over the (DC-EPOCH-08 validated) window's blocks (DC-EVIEW-10 drive_window_aggregate) -> per-pool stake, then binds it (DC-EVIEW-07 EpochConsensusView::bind) with the window's TARGET epoch (the Mark->Set lag, NEVER source_epoch), the window-end Point{source_window_end, lineage_pin}, the FINALIZED checkpoint commitment (the window drive clears the completeness marker; finalize re-marks + returns the commitment), the supplied network/nonce, and the window's snapshot_phase. Candidate binding happens BEFORE WAL activation; the candidate's identity is exactly what the WAL record (DC-EPOCH-04 activation_record_for) commits to and recovery (DC-EPOCH-06 recover_active_view) reproduces. The candidate contents are a pure function of (checkpoint, bootstrap cert state, the window's blocks, era, network, nonce) -- no peer/network read, wall-clock, or async side channel. A drive/checkpoint failure is fail-closed CandidateDeriveError -- no partial candidate reaches the predicate. |
| **Code** | crates/ade_node/src/epoch_candidate.rs: derive_candidate(window, checkpoint, bootstrap_state, blocks, era, network_magic, nonce) -> drive_window_aggregate -> checkpoint.finalize() (the commitment) -> EpochConsensusView::bind(.., window.target_epoch, Point{source_window_end, lineage_pin}, commitment, nonce, window.snapshot_phase, stake..); CandidateDeriveError{Drive\|Checkpoint}. ci/ci_check_eview_candidate.sh. |
| **Tests** | `derive_candidate_binds_target_epoch_and_round_trips_through_recovery` |
| **CI** | `ci/ci_check_eview_candidate.sh` |

#### `DC-EPOCH-10` — _enforced_

| Aspect | Location |
|--------|----------|
| **Source** | docs/clusters/EPOCH-CONSENSUS-VIEW/SLICE-3f4d-live-flip.md (S3f-4d-3a); user directive 2026-06-21 (one atomic path; durable-before-visible; halt on terminal) |
| **Requirement** | The boundary activation orchestration is ONE atomic, ordered, durable-before-visible path (S3f-4d-3a). activate_at_boundary sequences, in order: validate the durable source window (DC-EPOCH-08) -> derive the candidate (DC-EPOCH-09) -> the activation predicate BEFORE the WAL (DC-EPOCH-05/07: transition eligible + bindings verify + the candidate's point IS the selected-chain point) -> write the durable WAL activation record (DC-EPOCH-06) -> publish via activate_durable_before_visible ONLY if the write is durable -> atomically promote the active view. A predicate decline is NotYet (NOT terminal -- the seed view stays authoritative, retry the next boundary; NO WAL is written). A failure AFTER the predicate passes -- a non-durable WAL write, or a conflicting already-active view -- is a TERMINAL EpochViewActivationError the caller must halt on (no admit/forge/follow, NEVER a fallback to the seed view). An invalid (corrupt/forked/incomplete) window or a derivation failure is terminal BEFORE any WAL write. |
| **Code** | crates/ade_node/src/epoch_activate.rs: activate_at_boundary(window, window_blocks, checkpoint, bootstrap_state, blocks, era, network, nonce, selected_point, transition_eligible, active_view, wal_write) -> validate_source_window -> derive_candidate -> activation_predicate -> activation_record_for + wal_write (durable) -> activate_durable_before_visible -> active_view.promote; BoundaryActivationOutcome{Promoted\|NotYet}. ci/ci_check_eview_activate.sh. |
| **Tests** | `happy_path_promotes_after_durable_wal`; `non_durable_wal_is_terminal_and_does_not_publish`; `not_eligible_transition_is_not_yet_not_terminal`; `invalid_window_is_terminal_before_any_wal`; `selected_point_mismatch_declines` |
| **CI** | `ci/ci_check_eview_activate.sh` |

#### `DC-EPOCH-11` — _declared_

| Aspect | Location |
|--------|----------|
| **Source** | docs/clusters/EPOCH-CONSENSUS-VIEW/SLICE-3f4d-mat-live-checkpoint.md; user directive 2026-06-21 (make the reduced-checkpoint machinery live on the admission path; 8 locked points) |
| **Requirement** | The live reduced-UTxO checkpoint (S3f-4d-mat) -- the authoritative reduced-stake state Ade maintains on the selected-chain admission path so it derives its OWN next-epoch view (not just imports one). It is NOT a new parallel stake system: it makes the already-built reduced-checkpoint/window machinery (DC-EVIEW-04 reduce, DC-EVIEW-10 window driver) live. Locked points: (1) BLUE-authoritative content (a deterministic, replay- equivalent projection of the single ledger UTxO; the same blocks -> byte-identical checkpoint), not a cache; (2) advances only after selected-chain DURABLE admission (lockstep with the WAL AdmitBlock chain); (3) ChainDB/ WAL order only -- no peer-arrival/scheduler influence; (4) reorg restores/re-materializes the exact rollback lineage; (5) the bootstrap checkpoint is bound to the same seed/cert-state/chain-point/manifest (DC-EVIEW-09); (6) a missing/corrupt/lagging checkpoint BLOCKS EpochConsensusView production and fails closed; (7) no full UTxO resident in the live path (track_utxo=false preserved); bounded + disk-backed (redb); (8) existing current-epoch follow/forge stays BYTE-IDENTICAL until S3f-4d-wire activates a promoted view. -mat-1 (DONE): build the checkpoint from the seed UTxO at bootstrap (reduce_txout each output -> build_from), disk-backed, GATED on the EVIEW cert-state package (non-EVIEW bootstrap unchanged), before drop(utxo), fail-closed. |
| **Code** | crates/ade_node/src/admission/bootstrap.rs: build_live_reduced_checkpoint(snapshot_dir, utxo) -> reduce_txout each output into a BTreeMap -> ReducedUtxoCheckpoint::open(snapshot_dir/reduced-checkpoint.redb).build_from; called BEFORE drop(utxo), gated on ledger.cert_state.delegation.delegations non-empty (the EVIEW package); AdmissionBootstrapError::ReducedCheckpoint fail-closed. ci/ci_check_eview_live_checkpoint.sh. |
| **Tests** | `live_reduced_checkpoint_builds_durable_deterministic` |
| **CI** | `ci/ci_check_eview_live_checkpoint.sh` |

#### `DC-EPOCH-12` — _enforced_

| Aspect | Location |
|--------|----------|
| **Source** | docs/clusters/EPOCH-CONSENSUS-VIEW/SLICE-ECA-0b-leadership-complete-view.md; user directive 2026-06-21 (no live CertState read at rebind; no unbound protocol-parameter read; ASC reaches the projection ONLY through the bound commitment) |
| **Requirement** | The promoted-epoch PoolDistrView is derived EXCLUSIVELY from the sealed EpochConsensusView + the bound-commitment- checked consensus profile (ECA-0b). EpochConsensusView::to_pool_distr_view(genesis_hash, protocol_params_hash, asc) -> Result<PoolDistrView, ProjectionError>: it FIRST verifies consensus_profile_commitment(genesis_hash, protocol_params_hash, asc) == self.protocol_params_commitment (else ParamsCommitmentMismatch, FAIL-CLOSED -- no unbound protocol-parameter is ever read into leadership), THEN requires is_leadership_complete() (else NotLeadershipComplete), THEN builds PoolDistrView{epoch, total_active_stake, asc, pools: per kept pool PoolEntry{active_stake: stake_by_pool[p], vrf_keyhash: pool_vrf_keyhashes[p]}}. NO live CertState read, NO re-aggregation, NO unbound param: the next-epoch leadership distribution is a pure projection of the frozen view. A rebind that combined the sealed stake/VRF with a live cert-state join or an unbound ASC is structurally impossible. Pure, total, deterministic. |
| **Code** | crates/ade_ledger/src/reduced_epoch_view.rs: EpochConsensusView::to_pool_distr_view(genesis_hash, protocol_params_hash, asc) (verify commitment -> require is_leadership_complete -> build PoolDistrView/PoolEntry keyed by Hash28(pool.0)); ProjectionError{ParamsCommitmentMismatch, NotLeadershipComplete}. ci/ci_check_eview_leadership_complete.sh. |
| **Tests** | `to_pool_distr_view_builds_from_bound_profile_and_rejects_wrong_params`; `projection_rejects_wrong_profile_through_the_real_derive_path` |
| **CI** | `ci/ci_check_eview_leadership_complete.sh` |

#### `DC-EPOCH-13` — _enforced_

| Aspect | Location |
|--------|----------|
| **Source** | docs/clusters/EPOCH-CONSENSUS-VIEW/SLICE-ECA-1-remove-activation-gate.md; user directive 2026-06-21 (a build/runtime flag deciding whether a consensus transition occurs is a forbidden semantic gate; activation must be automatic + deterministic from canonical state; the predicate is the only gate) |
| **Requirement** | No semantic activation gate: no build- or runtime-level switch decides WHETHER the epoch-view activation occurs. There is no EVIEW_ACTIVATION_ARMED const, no `armed` parameter, no `if !armed` short-circuit, and no equivalent env var / build feature / CLI option anywhere in crates/. Activation is AUTOMATIC and DETERMINISTIC: the ONLY gate is the activation predicate over canonical durable state (candidate exists + bindings match the selected chain + source window complete + readiness valid + activation WAL durable => promote; else => structured terminal halt). maybe_activate_first_boundary proceeds to the sole authoritative activation whenever the seed epoch's window is COMPLETE (the durable tip located in a LATER epoch via era_schedule.locate -- never the wall clock) AND no view is promoted yet (idempotent); it fails closed (terminal ActivationError) on any error. The non-EVIEW byte-identical guarantee keys on CANONICAL STATE (the EVIEW cert-state package / reduced checkpoint absent => no EVIEW), never a flag: maybe_activate_epoch_boundary short-circuits only when (eview_activation, reduced_checkpoint) is not (Some, Some). Every replay of the same durable inputs makes the same activation decision. |
| **Code** | crates/ade_node/src/epoch_wire.rs: maybe_activate_first_boundary (no `armed` param; gates on the era_schedule.locate boundary detection + the idempotent active_view.promoted() check; runs try_activate_at_boundary -> the predicate); EviewActivationInputs::maybe_activate (no `armed` param). crates/ade_node/src/node_lifecycle.rs: maybe_activate_epoch_boundary keyed on (Some(inputs), Some(live)) = canonical state, not a flag. ci/ci_check_eview_automatic_activation.sh. |
| **Tests** | `maybe_activate_first_boundary_is_automatic_and_fails_closed_not_flag_gated` |
| **CI** | `ci/ci_check_eview_automatic_activation.sh` |

#### `DC-EPOCH-14` — _enforced_

| Aspect | Location |
|--------|----------|
| **Source** | docs/clusters/EPOCH-CONSENSUS-VIEW/SLICE-ECA-2-3-4-atomic-epoch-authority-transition.md; user directive 2026-06-22 (ECA-2/3/4 ship as ONE mergeable atomic-epoch-authority-transition slice -- construct deterministic inputs -> durably record activation -> atomically publish the authority -> recover the same authority after crash is ONE authoritative state transition; ECA-4 recovery is the second half of the authority contract, not cleanup); user 2026-06-23 (the slot-aware bidirectional + mode-aware guard via a canonical EpochAuthorityMode, not an ambient flag; header validation stays gated-fail-closed, the structured AuthorityEpochMismatch classification deferred as diagnostic refinement) |
| **Requirement** | Atomic epoch-authority transition + recovery. The node holds exactly ONE owned ActiveEpochAuthority -- the SOLE leadership + header-validation view source -- and crosses an epoch boundary as ONE authoritative state transition. (a) ONE HOLDER: header validation AND leadership/forge resolve the SAME holder, FRESH per authoritative decision (authority.ledger_view() / .pool_distr_view()), never a separately-built view nor a reference stored across the swap. (b) ATOMIC SWAP: at the boundary the holder is promoted IN PLACE (Seed -> Promoted) by the SAME path that derives the bound candidate, verifies the activation predicate, and writes the durable activation WAL record BEFORE the promotion is visible; a failure after the predicate is a terminal halt, never a seed fallback. (c) EPOCH-MATCH GUARD (slot-aware, mode-aware): at a forge decision the resolved authority's epoch MUST relate to the slot's protocol epoch (the SAME EraSchedule::locate the decision uses) per the CANONICAL EpochAuthorityMode -- recovered identically from durable state, NEVER an ambient runtime flag: authority_epoch > slot => terminal PrematurePromotion (both modes); == => proceed; < + ContinuityRequired => terminal MissingPromotion; < + SeedOnly => a graceful no-forge (ForgeNotLeader; the follow loop stays alive). Header validation rejects a wrong-epoch block through the SAME epoch-gated holder (a seed view answers None for an N+1 query -> reject before acceptance). (d) PROMOTION LINEARITY: <= 1 promoted authority per target epoch -- an identical re-promotion is idempotent, a different binding terminal. (e) CROSS-CONSUMER IDENTITY: at a slot, validation + forge resolve the SAME authority epoch AND the SAME active-view canonical hash (epoch alone is insufficient; two N+1 candidates with different bindings both report N+1). (f) RECOVERY EXACTNESS: warm-start reconstructs the promoted authority by RE-DERIVING the candidate from the durable tuple (activation-WAL record + v4 seed sidecar + bootstrap checkpoint + canonical selected-chain window) and promoting ONLY if the re-derived candidate reproduces the record's ENTIRE identity -- a record that merely parses but cannot be RECOMPUTED IDENTICALLY is a terminal halt; no record => the seed stays; never a fall back to the epoch-wrong seed view. Every replay of the same durable inputs makes the same decision and recovers the same authority. This is a hermetic guarantee: it does not prove unattended public Preview continuity (ECA-5). |
| **Code** | crates/ade_node/src/epoch_activation.rs: ActiveEpochAuthority (the one holder; ledger_view/pool_distr_view resolved fresh; promote() the sole mutation path), EpochAuthorityMode (SeedOnly\|ContinuityRequired, established from durable state) + guard_epoch -> AuthorityEpochVerdict, active_view_identity, recover_active_view / activation_record_matches / resolve_activation_record. crates/ade_node/src/epoch_activate.rs: activate_at_boundary (live), recover_at_boundary (warm-start re-derive + recover, reject-non-recomputable). crates/ade_node/src/epoch_wire.rs: try_recover_at_boundary, maybe_recover_promoted_authority. crates/ade_node/src/node_sync.rs: forge_one_from_recovered (reads authority.pool_distr_view(); the mode-aware epoch guard before leadership). crates/ade_node/src/node_lifecycle.rs: run_relay_loop_with_sched (the one authority; mode from the EVIEW package; the warm-start recovery BEFORE the loop; maybe_activate_epoch_boundary). ci/ci_check_eview_atomic_authority.sh. |
| **Tests** | `authority_epoch_guard_is_mode_aware_and_identity_is_exact`; `cross_consumer_identity_validation_and_forge_resolve_one_authority_view`; `seed_only_sole_view_cannot_validate_n1_header_rejects_before_acceptance`; `forge_continuity_required_missing_promotion_at_n1_is_terminal`; `node_forge_off_epoch_slot_fails_closed`; `recover_at_boundary_round_trips_the_durable_record_and_rejects_a_tamper`; `happy_path_promotes_after_durable_wal`; `crash_before_durable_wal_keeps_seed`; `crash_after_wal_republishes_same_view`; `recovered_view_mismatch_is_terminal` … (+1 more) |
| **CI** | `ci/ci_check_eview_atomic_authority.sh` |

#### `DC-EPOCH-15` — _enforced_

| Aspect | Location |
|--------|----------|
| **Source** | Slice ECA-5 (EPOCH-CONSENSUS-VIEW). Operational detail (venues, timing, commands, live-capture) is kept in an untracked competition-secret working doc; this registry entry + docs/clusters/EPOCH-CONSENSUS-VIEW/SLICE-ECA-5-summary.md are the committed normative record. User directive 2026-06-25 (derive-not-persist; atomic-swap ordering; warm-start byte-identical reconstruction in both pre- and post-promotion states). Live finding: a native-Mithril following node caught up WITHIN an epoch but failed Header(OutsideForecastRange) at the boundary because the leadership view promoted (DC-EPOCH-14) while the forecast horizon did not. |
| **Requirement** | Forecast horizon <=> durable N+1 authority promotion. The relay loop's EraSchedule forecast horizon extends past an epoch boundary N->N+1 IF AND ONLY IF the ActiveEpochAuthority has durably promoted the N+1 view (DC-EPOCH-14). (a) NO PRE-EXTENSION: the horizon never reaches into an epoch whose view is not promoted -- a header at an unpromoted N+1 slot fails closed OutsideForecastRange before the leader view is consulted, never accepted on a stale horizon. (b) ATOMIC ORDERING: the extension is the second half of the SAME transition as the promotion -- promotion durable -> rebuild the immutable schedule including the N+1 summary -> atomically replace the relay-loop-owned schedule -> only THEN is post-boundary validation/forging permitted; no mutable shared reference may leave validation using the old horizon after the authority has promoted. (c) DERIVED, NEVER PERSISTED: the extended schedule is derived state. Its authoritative inputs are the durable activation record + the recovered promoted EpochConsensusView + the v4 sidecar venue geometry (DC-CINPUT-05) + the committed network profile. Warm-start RECONSTRUCTS the schedule deterministically from those inputs and requires byte-identical schedule identity to the pre-restart one; a second WAL field for the schedule is forbidden (redundant authority + a new mismatch class). (d) NO AMBIENT INPUT: no flag, no wall-clock, no peer/CLI datum influences the extension (DC-EPOCH-13). Warm-start in BOTH states (before and after promotion) reconstructs a forecast boundary that exactly matches the live one. |
| **Code** | crates/ade_node/src/node_lifecycle.rs: extend_schedule_to_epoch (derive each appended N+1 EraSummary from the seed geometry -- start_slot = seed.start_slot + (e - seed.start_epoch)*epoch_length; atomic *era_schedule = EraSchedule::new(...)); run_relay_loop_with_sched owns the schedule (let mut era_schedule = era_schedule.clone()) + extends it at the maybe_activate_epoch_boundary promotion site (schedule threaded by &mut) AND the warm-start recovery (extend_schedule_to_epoch(.., authority.epoch())); resolve_network_magic resolves the EVIEW network magic from the --network profile on both forge branches; the forge-OFF branch constructs eview_inputs. crates/ade_core/src/consensus/era_schedule.rs: EraSchedule::new/locate over adjacent same-era Conway summaries (proof obligation VERIFIED -- monotonic start_slots, no same-era guard). ci/ci_check_eview_forecast_crossing.sh. |
| **Tests** | `forecast_extends_only_on_promotion`; `warmstart_reconstruction_is_byte_identical_to_live_append`; `eraschedule_supports_adjacent_same_era_summaries` |
| **CI** | `ci/ci_check_eview_forecast_crossing.sh` |

#### `DC-EPOCH-16` — _enforced_

| Aspect | Location |
|--------|----------|
| **Source** | docs/clusters/EPOCH-CONSENSUS-VIEW/SLICE-ECA-B1-rolling-praos-nonce-follow.md; user directive 2026-06-25 (fold the live per-header Praos update into ONE HeaderContribution and retire the dead CandidateFreeze split; backward-compatible array(10)/legacy-array(9) chain-dep format; an explicit no-last_epoch_block_nonce form for legacy stores that fails closed before the rolling cross-boundary path -- never a fabricated nonce; the seeded-snapshot -> B1-tick -> eta0(seed+1) == ECA-5 bridge cross-check is a mandatory hermetic assertion; the live gate stays self-evolved eta0(seed+2) == cardano-node epochNonce(seed+2)). Pinned canonical rule from ouroboros-consensus Praos.hs (reupdateChainDepState + epoch tick), cross-checked vs cardano-ledger @ cb57dc730 (Updn/Tickn, BaseTypes, StabilityWindow). Praos boundary combine drops extraEntropy and uses last_epoch_block_nonce (the lab of the last block of E-1), diverging from TPraos TICKN. Operational detail (venues, slots, timing, commands, capture) is kept in an untracked competition-secret runbook. |
| **Requirement** | Rolling Praos chain-dep nonce evolution on the live follow path. Each validated followed header drives ONE indivisible BLUE nonce transition over {slot, prev_block_hash, vrf_nonce_output, next_epoch_freeze_boundary}: evolving' = evolving (X) nonceValue(vrf_nonce_output); lab' = prevHashToNonce(prev_block_hash); candidate' = evolving' WHILE slot < freeze_boundary ELSE candidate (frozen), with freeze_boundary = firstSlotNextEpoch - RSW and RSW = ceil(4*k / f). The epoch tick computes epoch_nonce' = candidate (X) last_epoch_block_nonce, previous_epoch_nonce' = epoch_nonce, last_epoch_block_nonce' = lab, with evolving AND candidate carried UNCHANGED across the boundary (NO reset). (a) (X) = Nonce(blake2b256(a \|\| b)), NeutralNonce identity; the Praos boundary combine carries NO extraEntropy operand (unlike TPraos TICKN). (b) EXPLICIT OPERAND PRESENCE: last_epoch_block_nonce is an explicit optional; the boundary combine FAILS CLOSED (MissingLastEpochBlockNonce) on an absent operand unless supplied by a valid bootstrap bridge (ECA-5) or a freshly-seeded B1 chain-dep -- a nonce is NEVER fabricated. (c) BACKWARD-COMPATIBLE DURABLE FORMAT: the chain-dep snapshot encoder ALWAYS writes the array(10) form (10th field null\|bytes(32) = last_epoch_block_nonce); decode accepts EXACTLY arity 10 (full B1 state) OR the legacy arity 9 (last_epoch_block_nonce = explicit None, preserving the store's already-promised within-epoch operation and barred from the rolling cross-boundary combine). (d) BRIDGE EQUIVALENCE (hermetic, mandatory): the B1 epoch tick over the seeded seed-epoch snapshot nonces reproduces the live-proven ECA-5 bridge eta0(seed+1) byte-identically. (e) LIVE GROUND TRUTH: Ade's self-evolved eta0(seed+2) equals the live Cardano node's epochNonce(seed+2). CandidateFreeze as a separable transition is retired -- the per-header transition is indivisible. |
| **Code** | crates/ade_core/src/consensus/nonce.rs: reshaped NonceInput (one HeaderContribution{slot, prev_block_hash, vrf_nonce_output, freeze_boundary} -> evolving'/lab'/candidate'; EpochBoundary{new_epoch} -> combine + rotation + no-reset; CandidateFreeze removed; MissingLastEpochBlockNonce). crates/ade_core/src/consensus/praos_state.rs: PraosChainDepState.last_epoch_block_nonce: Option<Nonce>. crates/ade_ledger/src/snapshot/chain_dep.rs: always-write array(10), accept legacy array(9) -> None. crates/ade_core/src/consensus/header_validate.rs: Step 9 call site threads prev_block_hash + freeze_boundary (freeze_boundary = firstSlotNextEpoch - ceil(4k/f) from EraSchedule + k/f). crates/ade_node/src/node_sync.rs: drives the EpochBoundary tick on the live follow path (B2 -- replaces the ECA-5 bridge eta0 overlay; MANDATORY cross-check tick.epoch_nonce == bridge eta0 at the first boundary; evolving NOT reset). B2 threads live RSW = ceil(4k/f) into the candidate freeze from the era geometry via the single BLUE praos_rsw_slots (crates/ade_core/src/consensus/era_schedule.rs), fed by crates/ade_runtime/src/consensus/genesis_parser.rs + crates/ade_node/src/bootstrap_export.rs (NetworkProfile.security_param). crates/ade_ledger/src/ledgerdb_state.rs extract_praos_nonces_v2: the FirstRun bootstrap seeds the chain-dep from the SIX-nonce PraosState [evolving=tail[0], candidate=tail[1], epoch=tail[2], lab=tail[4], last_epoch_block=tail[5]] -- B2c corrected the prior trailing-5 scan that dropped evolving and mis-read previousEpoch as it (caught by the boundary-2 live eta0(seed+2) gate). ci/ci_check_praos_nonce_follow_evolution.sh. LIVE-FORGE-HARDENING S2 (warm-start self-sufficiency): the seed sidecar (crates/ade_ledger/src/seed_consensus_inputs.rs, schema v5->v6, FIELDS_OUTER 13->14) persists security_param (k); warm_start_recovery (crates/ade_node/src/node_lifecycle.rs ~3575) derives the candidate-freeze RSW = ceil(4k/f) from the DURABLE STORE via the SAME praos_rsw_slots -- byte-identical to the live/CLI freeze, closing the gap where an absent restart --network left the freeze inert (None) so a rollback+warm-restart over-tracked the candidate past the freeze slot -> wrong eta0(N+1); the restart-CLI RSW is retained ONLY as a fail-closed cross-check (mismatch -> terminal). k threads genesis securityParam -> NativeGenesisConstants -> LiveConsensusInputs{Raw,Canonical} -> SeedEpochConsensusInputs; the importer REQUIRES it (fail-closed MissingField, no fabricated default). ci/build_consensus_inputs_bundle.sh emits security_param from Shelley-genesis securityParam. Both the recovery replay (warm_start_recovery) AND the forward live-loop schedule (recovered_node_schedule, feeding both --mode node relay call sites) derive the freeze window through ONE shared crates/ade_node/src/node_lifecycle.rs sidecar_freeze_rsw helper (store k -> praos_rsw_slots; CLI = fail-closed cross-check), so the durable store is the SOLE freeze authority on both paths -- an absent/unsupported restart --network can no longer leave the forward candidate freeze INERT. The seed-sidecar decoder (crates/ade_ledger/src/seed_consensus_inputs.rs) gates the schema VERSION before the outer arity so a real older-shape (v5 array(13)) store surfaces the TYPED UnknownVersion (re-bootstrap-to-upgrade, not corruption). The live importer (crates/ade_runtime/src/consensus_inputs/importer.rs) rejects active_slots_coeff.numer==0 (f=0 -> undefined freeze window) at ingress, symmetric with denom==0. |
| **Tests** | `bridge_equivalence_seeded_snapshot_tick_reproduces_eca5_eta0` †; `header_contribution_advances_evolving_lab_candidate` †; `candidate_freezes_at_freeze_boundary` †; `epoch_tick_combines_candidate_with_last_epoch_block_nonce_no_reset` †; `epoch_tick_rotates_last_epoch_block_nonce_from_lab` †; `epoch_tick_fails_closed_on_absent_operand` †; `chain_dep_array10_round_trip_some_and_none` †; `chain_dep_legacy_array9_decodes_to_none`; `chain_dep_always_writes_array10` †; `b1_store_round_trip_reproduces_next_boundary_eta0` † … (+2 more) |
| **CI** | `ci/ci_check_praos_nonce_follow_evolution.sh` |

#### `DC-EPOCH-17` — _declared_

| Aspect | Location |
|--------|----------|
| **Source** | docs/clusters/EPOCH-CONSENSUS-VIEW/SLICE-ECA-B3-replay-derived-seed2-authority.md. Generalizes the DC-EPOCH-15 first-boundary bridge seam to a per-boundary advance; builds on DC-EPOCH-16 (the self-evolved eta0(seed+2), enforced live 2026-06-26). Pinned canonical leadership lag from cardano-ledger NewEpochState (psStakeMark/Set/Go = 2). Operational venue detail (snapshot, slots, commands) kept in the untracked competition-secret runbook. |
| **Requirement** | Replay-derived per-boundary leadership authority on the live follow path. The activation seam (prepare_authority_for_candidate_slot) ADVANCES the promoted epoch authority at EVERY boundary it crosses, not once: for a candidate slot in epoch C with the currently-promoted authority for epoch P, it promotes the C authority IFF C == P+1 (C > P+1 is terminal CandidateSlotSkipsBoundary; the candidate parent must bind the durable selected tip). The authority SOURCE is chosen deterministically by boundary index: C == seed_epoch+1 uses the ECA-5 MARK bridge (DC-EPOCH-15); C >= seed_epoch+2 uses the WINDOW-REPLAY over the durable C-2 epoch window (try_activate_at_boundary over the reduced checkpoint + canonical C-2 window blocks + v4 sidecar geometry), reflecting the MARK/SET/GO leadership snapshot lag = 2 (authority(E) = replay(E-2)). (a) the bound view's epoch_nonce is the EVOLVED chain-dep epoch_nonce at the boundary (DC-EPOCH-16, live-proven), never a recomputed/bridge nonce. (b) DURABLE-BEFORE-VISIBLE: the WAL activation record is written before the active view is advanced. (c) REPLAY-DETERMINISTIC: the promoted authority is a pure function of the durable window (blocks + reduced checkpoint + sidecar) -- same durable state => byte-identical bound view + WAL record. (d) FAIL-CLOSED: a missing/short C-2 window, a boundary-skipping candidate, or a parent not binding the durable tip is TERMINAL -- no silent bridge fallback past seed+1, no Origin fallback. AUTOMATIC + DETERMINISTIC: the sole gate is the candidate-slot/tip predicate over canonical durable state (no arming flag). LIVE GROUND TRUTH: Ade validates + admits seed+2 blocks against the replay-derived seed+2 authority + eta0(seed+2), crossing N+1->N+2 in the relay loop with no OutsideForecastRange / VrfCert / fail-close. |
| **Code** | crates/ade_node/src/epoch_wire.rs prepare_authority_for_candidate_slot (generalize the boundary detection + the per-boundary source branch) + try_activate_at_boundary (the window-replay source, reused); crates/ade_node/src/epoch_activation.rs ActiveEpochAuthority.advance (in-place P -> P+1, durable-before-visible); crates/ade_node/src/node_sync.rs (drive the seam at every boundary + extend the forecast). TBD at implementation. |
| **Tests** | — _(declared — not yet enforced)_ |
| **CI** | — _(declared — not yet enforced)_ |

#### `DC-EPOCH-18` — _enforced_

| Aspect | Location |
|--------|----------|
| **Source** | docs/clusters/EPOCH-CONSENSUS-VIEW (B3c). Strengthens the DC-EPOCH-17 seed+2 window-replay authority with the cardano-ledger epoch-boundary reward update. Replaces a rejected apply-at-bootstrap approach (known counterexample: a seed-window-tail reward-withdrawer would receive 0 instead of its boundary reward). Operational venue detail in the untracked competition-secret runbook. |
| **Requirement** | Window-end bootstrap reward update for the seed+2 leadership authority. The first post-bootstrap replay-derived authority (seed+2, DC-EPOCH-17) is the per-pool stake snapshot at the seed->seed+1 boundary; that stake = base-UTxO + reward-account balance MUST include the epoch-boundary REWARD UPDATE the chain distributes at that boundary. Cardano applies the reward update AFTER the replayed epoch's withdrawals and THEN snapshots (NewEpochState applyRUpd precedes SNAP), so the delta is applied at the WINDOW-END of the seed-epoch replay -- in drive_window_consensus_inputs after the replay loop, immediately before aggregate_pool_stake -- and EXACTLY ONCE. (a) NO MUTATED PSEUDO-STATE: the seed cert-state is NEVER mutated; the delta is a separate, manifest-bound (anchor_fp), commitment-bound (domain-separated blake2b), version-gated (v1) replay INPUT (BootstrapRewardUpdate = the snapshot's Complete nesRu rs map, aggregated per credential), persisted at bootstrap + recovered at warm-start. (b) PURE BLUE TRANSITION: apply_bootstrap_reward_deltas(&mut DelegationState, &delta) on the window-replay CLONE (the RED driver never mutates core state directly); checked_add fail-closed on overflow. (c) REPLAY-EQUIVALENT: pre-update seed + canonical replay window + the bound delta reproduce the identical post-boundary snapshot on first-run derive AND warm-start recover (they share CandidateProfile). (d) FAIL-CLOSED MECHANICALLY AT THE SINGLE DERIVATION SITE: derive_candidate REQUIRES the bound rupd (target_epoch == seed_epoch) for the seed+2 window (source_epoch == seed_epoch) -- absent or wrong-epoch is TERMINAL -- so EVERY seed+2 derivation path (live first-boundary activate, per-boundary prepare, warm-start recover) fails closed WITHOUT replicating the gate in callers; a mid-pulse (non-Complete) nesRu also fails closed at decode. No native-bootstrap route derives this authority without the reward distribution (a legacy apply-at-bootstrap store, which mutated the seed, fails closed). (e) CLOSED CODEC: decode rejects unknown version, commitment mismatch, non-canonical/duplicate credential keys, unknown credential tag, trailing bytes, non-minimal encoding. LIVE GROUND TRUTH: on preview epoch 1340 the derived pool stake equals the real node's go snapshot to the lovelace; every 1340 block admits on first-run AND warm-start; a no-rupd legacy store fails closed. |
| **Code** | crates/ade_ledger/src/bootstrap_reward_update.rs (closed codec); crates/ade_ledger/src/delegation.rs apply_bootstrap_reward_deltas (BLUE apply); crates/ade_runtime/src/chaindb/reduced_window_driver.rs drive_window_consensus_inputs (window-end call); crates/ade_node/src/epoch_candidate.rs derive_candidate (the single-site seed+2 fail-closed gate + CandidateProfile.seed_epoch); crates/ade_node/src/epoch_wire.rs EviewActivationInputs.bootstrap_reward_delta + the CandidateProfile constructions; crates/ade_node/src/native_firstrun.rs (persist); crates/ade_node/src/node_lifecycle.rs (recover, both arms); crates/ade_runtime/src/chaindb/{mod.rs,persistent.rs} bootstrap_rupd_by_anchor_fp; crates/ade_ledger/src/ledgerdb_state.rs read_reward_update_deltas (Complete-or-fail-closed). |
| **Tests** | `bootstrap_reward_update::tests (round-trip/version/commitment/tamper)`; `reduced_window_driver drive_window_applies_bootstrap_reward_delta_to_the_aggregate`; `delegation apply_bootstrap_reward_deltas_sums_and_fails_closed_on_overflow`; `epoch_candidate seed2_window_without_rupd_fails_closed / seed2_window_with_wrong_epoch_rupd_fails_closed / seed2_window_with_correct_rupd_derives / non_seed2_window_without_rupd_is_a_strict_noop` |
| **CI** | `ci/ci_check_bootstrap_rupd_window_end.sh` |

#### `DC-EPOCH-19` — _declared_

| Aspect | Location |
|--------|----------|
| **Source** | docs/clusters/LIVE-LEDGER-EPOCH-TRANSITION/cluster.md + SLICE-S1-authority-transition-contract.md. The authority-surface trace (2026-06-27) confirmed the live follow validates headers + stores blocks + advances the reduced checkpoint + the Praos nonce, but does NOT evolve rewards/block-production/fees/snapshot-rotation -- so post-bootstrap future authority runs out. Preserves MEM-OPT (no full live UTxO). Operational venue detail in the untracked competition-secret runbook. |
| **Requirement** | Self-sustaining live ledger epoch evolution. After every durable selected-chain block, the node holds enough durable, replayable state to derive EVERY future epoch transition -- rewards, stake snapshots, pool/cert lifecycle, and leadership authority -- WITHOUT another Mithril import, an external CLI oracle, or a manually injected authority. The mechanism is ONE total, deterministic, replay-equivalent transition apply_selected_block(prior EpochAccumulator, canonical block, selected ctx) -> next \| structured error, applied per durable admitted block, that evolves the non-UTxO ledger facts (cert state + delegations + pool/future-pool/retirement maps + per-epoch block_production + epoch_fees + reserves/treasury pots + mark/set/go snapshots + the consensus-relevant protocol/governance state + the pending reward update) in cardano-ledger NEWEPOCH order at each crossed boundary (apply the completed RUPD over the just-finished epoch's accumulated counts/fees AFTER that epoch's withdrawals -> SNAP rotate mark/set/go -> POOLREAP -> enactment/reset) and the within-epoch tx/cert effects per block. (a) BOUNDED MEMORY: the large UTxO/stake set stays in the disk-backed reduced checkpoint (no permanent full UTxO map in RAM, no full-chain replay per block, no full-accumulator clone per block); the accumulator is the small non-UTxO state machine, persisted incrementally with a boundary checkpoint + at-most-one-epoch within-epoch replay. (b) REPLAY-EQUIVALENT: restart + rollback re-materialize the IDENTICAL accumulator + authority from the durable selected chain via the same fold. (c) FAIL-CLOSED: a malformed block, unknown authority-path cert/governance variant, pot/reward/count overflow, missing input, or boundary gap is terminal. (d) BOOTSTRAP-TRANSITION: seed+1 (MARK bridge, DC-EPOCH-15) and seed+2 (snapshot nesRu, DC-EPOCH-18) remain the bootstrap-transient seeds; the native RUPD takes over at the first boundary whose entire input epoch was followed. LIVE GROUND TRUTH: a fresh native bootstrap at N crosses N+1->N+2->N+3 with self-derived rewards + snapshots (byte-exact vs the live cardano-node at >=2 self-derived boundaries), restarts in each phase, survives a controlled rollback -- no Mithril re-import / oracle / injection -- and bounded memory holds across the run. |
| **Code** | S1 LANDED: ade_ledger::epoch_accumulator (EpochAccumulator + apply_selected_block + cross_epoch_boundary + the canonical encode/decode_epoch_accumulator codec composing the snapshot/ sub-codecs + the nesBprev buffers + pending_reward_update). The boundary reuses ade_ledger::rules apply_epoch_boundary_with_registrations + epoch::rotate_snapshots + delegation::apply_pool_reap; the within-epoch half reuses rules::process_block_certificates + delegation::apply_bootstrap_reward_deltas; the reduced checkpoint stays the stake substrate (read via SelectedBlockCtx.boundary_mark, never stored). S2-S6 TBD: ade_runtime forward_sync/node_sync per-block evolution on the live follow + the live byte-exact boundary gate + restart/rollback + the N->N+3 self-derived proof. |
| **Tests** | `ade_ledger::epoch_accumulator::tests (codec round-trip byte-identical + non-canonical/unknown-version/pre-Conway-era/trailing-bytes fail-closed; boundary_rotates_block_production_two_buffer; within_epoch_withdrawal_then_boundary_pays_fresh_reward; pending_reward_update_applied_once_then_cleared; missing_boundary_stake + boundary_gap fail-closed; apply_selected_block_on_real_conway_block_is_deterministic; replay_equivalence_via_durable_checkpoint_across_a_boundary)`; `ade_node::node_lifecycle::ce4a_continuous_self_sufficiency PRODUCTION-LOOP EVIDENCE (#[ignore] slow runs, NOT CI gates): CE-4B ce4b_three_boundary_continuous_self_sufficiency -- LITERAL three-boundary 1340->1341->1342->1343 (seed+2 -> seed+5) in one continuous run, self-sufficient (promotion-certified frozen leadership through 1344; no re-import / CLI oracle / seed-window replay / materialize_bootstrap_into; no fail-closed halt; c5bdc064); CE-4A.1 drive (1340->1342, 9c6fc3c4); #12 ce4a_3_restart_only_equivalence + #13 ce4a_3_r2_rollback_refold_equivalence (warm restart + controlled rollback + production ResetAndRefold == uninterrupted byte-identical; fd3826fd); CE-4A.2 drive_capture_at (POST-1341/1342 byte-exact vs cardano, af3dc9c7). See docs/clusters/LIVE-LEDGER-EPOCH-TRANSITION/CE-4-MILESTONE-DECLARATION.md` |
| **CI** | `ci/ci_check_epoch_accumulator_no_utxo.sh` |

#### `DC-EPOCH-20` — _declared_

| Aspect | Location |
|--------|----------|
| **Source** | docs/clusters/LIVE-LEDGER-EPOCH-TRANSITION/SLICE-S2-live-within-epoch-wiring.md. Generalizes the WAL-is-admission-authority recovery (recovery/restart.rs) + the reduced-checkpoint LAST_SLOT lockstep (DC-EPOCH-11) + the materialize_rolled_back_state replay-equivalence pattern to INCLUDE the EpochAccumulator as a fourth derived store, so the live within-epoch wiring (S2) cannot create a recoverable split-authority prefix. The dangerous failure mode is not memory; it is a resumed mixed prefix across the four authorities. |
| **Requirement** | Atomic-or-rematerialized selected-block admission -- no RESUMED split authority. For every selected block admitted to the durable chain, four derived authorities advance from the SAME selected-chain prefix: the ChainDB/WAL block record, the Praos chain-dep state, the EpochAccumulator transition (DC-EPOCH-19), and the reduced-UTxO-checkpoint advancement (DC-EPOCH-11). The WAL tail is the single admission authority: recovery drops every block above the WAL-tail slot and reconciles the ChainDB tip TO it, so the authoritative selected-chain prefix is DEFINED as the WAL tail. The chain-dep, the reduced checkpoint, and the accumulator are derived stores -- each a pure function of that prefix. (a) IN-FLIGHT LAG ALLOWED: a derived store may momentarily lag the WAL tail (a torn write, a lazy advance cadence) during live admission, gated by DC-SYNC-01 (the tip is not advanced before the block bytes + WAL record are durable). (b) RECOVERY REMATERIALIZES: on any restart or rollback, every lagging derived store is rebuilt up to the WAL tail by folding its transition over the canonical durable blocks (the accumulator folds apply_selected_block, the reduced checkpoint folds reduced_block_delta, the ledger/chain-dep fold the trusted replay) -- never an ad hoc inverse mutation. (c) FAIL-CLOSED READINESS: validation and forging do not resume until a readiness gate confirms each derived store's last-advanced slot equals the WAL tail (Lagging / Ahead / SeedMismatch / Unsealed are terminal). The only RESUMED state is "all four at the WAL tail"; a mixed prefix (ChainDB at N, accumulator at N-1, checkpoint at N-2) is caught and rematerialized, never run on. |
| **Code** | ade_runtime/chaindb/epoch_accumulator_advance.rs (advance_accumulator_over_chaindb -- the durable forward-fold walk over (last_advanced, tip], folding advance_accumulator_over_block -> the BLUE apply_selected_block; stops at an observe-only boundary stall) + epoch_accumulator_store.rs (atomic blob+LAST_SLOT advance in one redb commit; reset_to_bootstrap = the sole reversal; verify_ready_at/verify_advanced_through = the fail-closed readiness gate, Lagging/Ahead/Unsealed terminal) + ade_node/node_lifecycle.rs (advance_accumulator_to_durable_tip -- the observe-only recovery/reorg wrapper called after each durable admit, beside advance_reduced_checkpoint_to_durable_tip: skip-if-unsealed, reorg overshoot -> reset_to_bootstrap + forward replay, swallow stalls/faults). S2 wires the within-epoch fold + the warm-start/reorg rematerialize; the readiness gate's forge call site (no consumer in observe-only S2) + the boundary crossing are S3/S4 -- status stays declared until the gate is wired to a leadership consumer. Reuses recovery/restart.rs (WAL-tail authority) + the reduced-checkpoint advance-to-tip pattern. |
| **Tests** | `ade_runtime::chaindb::epoch_accumulator_advance::tests (over_chaindb_folds_durable_prefix_to_tip -- warm-start catch-up folds the durable prefix to the tip; over_chaindb_rewalk_is_idempotent -- replay-safe resume advances nothing; over_chaindb_stops_at_boundary_observe_only -- the store freezes at the last within-epoch slot, MissingBoundaryStake; reset_then_rewalk_rematerializes -- reorg = reset-to-seed + forward replay re-materializes the SAME tip, no inverse mutation)`; `ade_runtime::chaindb::epoch_accumulator_store::tests (seal_advance_reset_round_trip_is_exact; advance_is_strictly_forward -- NonMonotonicAdvance fail-closed; readiness_gate_fails_closed -- Lagging/Ahead/Unsealed terminal)` |
| **CI** | `ci/ci_check_epoch_accumulator_recovery.sh` |

#### `DC-EPOCH-21` — _declared_

| Aspect | Location |
|--------|----------|
| **Source** | docs/clusters/LIVE-LEDGER-EPOCH-TRANSITION/SLICE-S3-boundary-gate.md. Resolves the two S1-recorded byte-exact reconciliation items (POOLREAP completeness/ordering; reward-account credential discriminant) against the cardano-ledger source (Shelley Rules/PoolReap.hs) + the actual Ade boundary surfaces. The pre-S3 split (apply_epoch_boundary_with_registrations inline retirement + a trailing delegation::apply_pool_reap call in cross_epoch_boundary) left the delegation-clear DEAD because the inline retirement emptied the retiring map first, so the trailing reap's == e match found nothing. |
| **Requirement** | The accumulator's epoch-boundary transition reproduces the canonical cardano NEWEPOCH result. POOLREAP is a SINGLE transition in the cardano order (Shelley PoolReap.hs, which Conway reuses), consolidated inside the shared apply_epoch_boundary_with_registrations so the full-ledger and accumulator paths share ONE order whose halves cannot silently fail to compose: (a) adopt staged future-pool re-registrations (drop orphans); (b) reap the pools retiring at EXACTLY this epoch (== e, never <= e); (c) refund each reaped pool's deposit to its OWN reward-account credential decoded by the REAL key/script discriminant (registered -> that reward account, unregistered -> treasury); (d) clear the reaped pools' delegators where the reap happens, so the clear can never be dead; (e) remove the reaped pools from the active set + the retiring schedule. No split POOLREAP whose clear half silently no-ops (the pre-S3 bug: the inline retirement emptied the retiring map before a trailing apply_pool_reap could match == e, so the delegation-clear was dead code), and no KeyHash projection that misroutes a script-hash reward account. The boundary's reward update is computed over the held nesBprev and the go snapshot AFTER within-epoch withdrawals, and SNAP rotates mark->set->go with the new mark = the reduced-checkpoint stake aggregate -- these two (the discriminant-correct RUPD reward crediting, still keyed by bare Hash28, and the live mark wiring) are enforced PROGRESSIVELY as S3 lands its live-mark wiring and its byte-exact differential gate vs a live cardano-node. |
| **Code** | ade_ledger/rules.rs (apply_epoch_boundary_with_registrations -- the single canonical POOLREAP block: future-pool adoption [drop orphans], retired = {retire_epoch.0 == new_epoch.0}, deposit refund via crate::epoch_accumulator::reward_account_credential [registered -> delegation.rewards, unregistered -> poolreap_to_treasury -> the downstream treasury update], delegation-clear delegations.retain(!retired.contains(pool_id)), pool/retiring removal; SHARED by the full-ledger path apply_epoch_boundary_full so both paths reap identically) + ade_ledger/epoch_accumulator.rs (cross_epoch_boundary -- the trailing apply_pool_reap call REMOVED + its import dropped; reward_account_credential made pub(crate) as the shared discriminant decoder). The discriminant-correct RUPD reward crediting (op_cred / the delta_t2 partition, still keyed by bare Hash28) + the live boundary_mark wiring + the byte-exact differential gate vs cardano-node land in S3's later items -- status stays declared until they do. delegation::apply_pool_reap stays for the reduced-window leadership replay (reduced_window_driver.rs), which needs adopt+clear but not the reward-side deposit refund. |
| **Tests** | `ade_ledger::rules::cert_state_dispatch::poolreap_ce3a (poolreap_reaps_exact_epoch_only -- == e reaped, > e and a STALE < e kept [proves == not <=]; poolreap_refund_registered_else_treasury -- registered operator refunded its deposit, unregistered -> treasury; poolreap_clears_reaped_pool_delegations -- THE dead-clear regression: a delegation to a reaped pool is cleared, a surviving one kept, the delegator registration preserved; poolreap_script_hash_reward_account_refunds_to_script_cred -- a 0xF0 reward account refunds to its ScriptHash credential, NOT a KeyHash projection of the same 28 bytes, and is not sent to treasury; poolreap_adopts_future_pool_params -- staged re-registration params adopted into the active set, future_pools drained, orphan future dropped)`; `ade_ledger::epoch_accumulator::tests::cross_epoch_boundary_per_credential_mark_pays_member_rewards (item #2a -- the PER-CREDENTIAL boundary mark pays NON-ZERO member + leader rewards where the per-pool mark paid zero; the go.delegations the reward computation reads survive) + epoch_boundary_consumes_precomputed_aggregate_mark (Some(precomputed_mark) = the per-credential StakeSnapshot used DIRECTLY, both pool_stakes and delegations survive)` |
| **CI** | `ci/ci_check_poolreap_single_canonical.sh` |

#### `DC-EPOCH-22` — _declared_

| Aspect | Location |
|--------|----------|
| **Source** | docs/clusters/LIVE-LEDGER-EPOCH-TRANSITION/SLICE-S3-boundary-gate.md section 3 + section 5 item #2b (BOUNDARY-ALIGNED-MARK-CAPTURE). The boundary mark is the only NEW live input the boundary transition needs; a naive read at the post-pass tip is byte-wrong in BOTH catch-up (the checkpoint is already past the boundary) and steady-state (the tip is the first block of the new epoch, whose UTxO delta must NOT be in the mark). The mark is sourced at the exact boundary point via the co-advancer, NOT the EVIEW BoundaryPromoted yield (which is EVIEW-gated and fires one block too late). Resolves the S2 observe-only boundary stall (MissingBoundaryStake, the forced ctx.boundary_mark = None). |
| **Requirement** | BOUNDARY-ALIGNED-MARK-CAPTURE. The live epoch-boundary stake mark is captured ONLY from the durable reduced checkpoint materialized at the EXACT selected-chain boundary point -- the last durable block of the closing epoch (s_prev) -- never at a later catch-up tip and never via a per-block stake scan. The capture is durably bound as a BoundaryMark keyed by the canonical boundary chain point (slot, hash) and the checkpoint lineage at that point, persisted BEFORE the accumulator boundary transition consumes it. The transition consumes a mark only when the binding is present and its point + lineage match the canonical chain; a reorg that removes or replaces the boundary point INVALIDATES the mark and forces deterministic rematerialization (reset-to-seed + replay) -- a mark is NEVER reused on an epoch-number match alone. This protects the boundary transition's INPUT (DC-EPOCH-21 governs the transition's OUTPUT given a correct mark): a mark read at the catch-up tip is byte-wrong (the checkpoint is already past the boundary), and even at the steady-state tip it wrongly includes the first block of the new epoch (SNAP captures the end-of-epoch stake, BEFORE that block). A co-advancer segments the reduced-checkpoint + accumulator advance at each boundary (idempotent-resume, byte-identical to advance-to-tip), captures sum_base_credential_stake() at the boundary point, durably binds the BoundaryMark before the cross, then crosses the accumulator over the boundary block with the bound mark; finally it advances the checkpoint the rest of the way to the durable tip (EVIEW currency preserved). OBSERVE-ONLY in S3: a capture/cross fault STALLS the accumulator (it stays at s_prev), never halting the proven follow; the EVIEW checkpoint still reaches tip fail-closed. Enforced PROGRESSIVELY: #2b-i wires the accumulator boundary-cross entry point (this sub-commit, cross_accumulator_over_boundary_block, the ONLY place the S2 mark-exclusion is lifted); the durable BoundaryMark witness + the point/lineage validation (#2b-ii), the node_lifecycle co-advancer segment->capture->bind->cross->tip (#2b-iii), the DC-EPOCH-22 CI guard, and the CE-3c live venue crossing ALL LANDED (#2b-i/ii/iii 8d047dee/ee33cc4c/8232fe73 + the CI guard). CE-3c PROVEN LIVE 2026-06-29: two preview crossings (1338->1339 seam + 1339->1340 native), mark captured at the boundary point s_prev while the catch-up tip ran 70,655 slots ahead (proof ~/.cardano-ce3c-proof/). Status stays declared: the mechanism is live-proven but OBSERVE-ONLY in S3 -- the flip to enforced is the cluster-close event with the accumulator-as-authority (S4) + the CE-3d byte-exact differential, in step with DC-EPOCH-19/20/21. |
| **Code** | ade_runtime/chaindb/epoch_accumulator_advance.rs (cross_accumulator_over_boundary_block -- the accumulator boundary-cross entry point, #2b-i: load_current -> idempotent AlreadyCrossed at/before tip [never re-decoded/re-applied] -> get_block_by_slot [MissingBlock fault if a directed boundary slot has no durable block] -> decode + era_schedule.locate the boundary epoch -> SelectedBlockCtx with boundary_mark = Some(mark) [the S2 mark-exclusion lifted ONLY here; WithinEpochCtx + advance_accumulator_over_block stay structurally mark-free] -> apply_selected_block -> store.advance on Crossed, observe-only Stalled on a contract fail-close; new AccumulatorBoundaryOutcome = Crossed/AlreadyCrossed/Stalled + AccumulatorChaindbError::MissingBlock; re-exported via chaindb/mod.rs). The durable BoundaryMark witness + the point/lineage validation (EpochAccumulatorStore bind_boundary_mark / boundary_mark_binding / clear_boundary_mark, with reset_to_bootstrap dropping the binding, #2b-ii DONE -- the witness carries ONLY the point + lineage (slot, hash); the mark VALUE is re-derived from the lineage-matched checkpoint, never double-stored), the node_lifecycle co-advancer (segment -> checkpoint-to-s_prev -> capture sum_base_credential_stake -> bind -> cross -> checkpoint-to-tip, the call-site swap of the two advance-to-tip calls, #2b-iii), and the DC-EPOCH-22 CI guard land next -- status stays declared until #2b-iii + the CE-3c live crossing do. |
| **Tests** | `ade_runtime::chaindb::epoch_accumulator_advance::tests::boundary_block_crosses_with_mark`; `ade_runtime::chaindb::epoch_accumulator_advance::tests::boundary_cross_is_idempotent`; `ade_runtime::chaindb::epoch_accumulator_advance::tests::boundary_cross_missing_block_is_a_fault`; `ade_runtime::chaindb::epoch_accumulator_store::tests::boundary_mark_witness_bind_read_clear_round_trip`; `ade_runtime::chaindb::epoch_accumulator_store::tests::boundary_mark_bind_requires_sealed`; `ade_runtime::chaindb::epoch_accumulator_store::tests::reset_to_bootstrap_drops_the_boundary_mark_binding`; `ade_runtime::chaindb::epoch_accumulator_store::tests::boundary_mark_binding_survives_reopen`; `ade_node::node_lifecycle::tests::co_advance_ledger_state::co_advance_crosses_a_boundary`; `ade_node::node_lifecycle::tests::co_advance_ledger_state::co_advance_checkpoint_only_when_no_accumulator`; `ade_node::node_lifecycle::tests::co_advance_ledger_state::co_advance_multi_boundary_catch_up` … (+1 more) |
| **CI** | `ci/ci_check_boundary_aligned_mark_capture.sh` |

#### `DC-EPOCH-23` — _enforced_

| Aspect | Location |
|--------|----------|
| **Source** | CE-3d bootstrap fee-buffer authority slice. Root cause CONFIRMED byte-exact: the native bootstrap decoder SKIPPED RewardUpdate.deltaF and the seed+1 apply never reduced the fee pot by it, so the imported epoch_fees retained the feeSS cardano consumes at seed+1, double-counting into the seed+2 reward update's fees input and inflating total_reward (tau share -> treasury, pool_pot -> rewards + reserves-via-deltaR2). Shelley formal-spec epoch.tex:1396 (fee pot reduced by feeSS on applyRUpd) + :459-461 (utxosFees is a running pot, feeSS the frozen snapshot). Strengthens the DC-EPOCH-18 seed+2 window-end reward authority with the fee-pot reduction cardano performs at applyRUpd. Human review + a live venue byte-exact differential gate precede commit. |
| **Requirement** | Bootstrap reward-update fee-buffer authority (CE-3d). The one-shot bootstrap reward update applied at the seed->seed+1 boundary carries the certified snapshot RewardUpdate's `feeSS` magnitude (deltaF), and the seed-boundary apply reduces the accumulated fee pot by it EXACTLY ONCE -- cardano's "the fee pot will be reduced by feeSS" (Shelley epoch.tex): applyRUpd reduces the fee pot by feeSS, THEN SNAP freezes the pot. (a) DECODED, NEVER SKIPPED OR FABRICATED: `deltaF` is READ from the Complete `nesRu` RewardUpdate by the single native decoder (`read_reward_update_deltas` via `read_any_int`, the same sign-agnostic magnitude mechanism as deltaT/deltaR), threaded onto `NativeSnapshotNonUtxoState.rupd_delta_fees` and into the persisted `BootstrapRewardUpdate.delta_fees`; a malformed (non-integer) deltaF fails closed. There is NO corrective constant -- the reduction is the decoded value, never a literal. (b) COMMITMENT-BOUND: the bootstrap RUPD v3 codec binds `delta_fees` into the domain-separated blake2b canonical commitment, verified at the seed-boundary apply before the reduction; a tampered feeSS fails closed. (c) EXACTLY ONCE, NEVER LEAKS: the reduction runs ONLY in the `is_seed_boundary` branch, before the fee pot is captured as `finished_fees` and rotated to `prev_epoch_fees` (the seed+2 reward's fee input), so the seed epoch's feeSS does not double-count into the seed+2 reward; a non-seed (native) boundary NEVER reduces; underflow (delta_fees > epoch_fees) fails closed `BootstrapRupdFeesUnderflow`. (d) SCHEMA v3 REJECTS PRE-FIX STORES: `BOOTSTRAP_RUPD_SCHEMA_VERSION` and `EPOCH_ACCUMULATOR_SCHEMA_VERSION` are both v3; a pre-fix v1/v2 store fails closed `UnknownVersion` on decode, and a v3 store whose embedded bootstrap RUPD lacks `delta_fees` is impossible (the v3 codec requires it) -- a fresh judge-snapshot re-bootstrap is the ONLY migration. GROUND TRUTH (frozen timeline, 1338 judge snapshot): imported utxosFees 2,296,344,810 + followed 1338 tail 308,031,321 = 2,604,376,131; snapshot RewardUpdate feeSS 1,157,103,223; corrected rotated fee pot 2,604,376,131 - 1,157,103,223 = 1,447,272,908; closing the residual rewards +30,800,403 / treasury +231,420,644 / reserves +894,890,405 (one defect, fanned out by tau=1/5 and pool_pot=(1-tau)). |
| **Code** | crates/ade_ledger/src/ledgerdb_state.rs (read_reward_update_deltas READS deltaF via read_any_int, returns it as the 4th tuple element; decode_native_nonutxo_state threads it to NativeSnapshotNonUtxoState.rupd_delta_fees). crates/ade_ledger/src/bootstrap_reward_update.rs (BOOTSTRAP_RUPD_SCHEMA_VERSION=3, RUPD_COMMITMENT_DOMAIN v3, FIELDS_OUTER=10; BootstrapRewardUpdate.delta_fees; bootstrap_rupd_commitment + encode_rupd_body bind delta_fees; decode reads + recomputes it). crates/ade_ledger/src/epoch_accumulator.rs (EPOCH_ACCUMULATOR_SCHEMA_VERSION=3; cross_epoch_boundary is_seed_boundary branch reduces acc.epoch_state.epoch_fees by checked_sub(rupd.delta_fees.0) before finished_fees capture, fail-closed LedgerTransitionError::BootstrapRupdFeesUnderflow; the commitment recompute now binds delta_fees). crates/ade_node/src/native_firstrun.rs (threads s1a.rupd_delta_fees into the persisted BootstrapRewardUpdate). ci/ci_check_bootstrap_rupd_fee_reduction.sh. |
| **Tests** | `ade_ledger::ledgerdb_state::tip_tests::read_reward_update_deltas_returns_delta_fees`; `ade_ledger::ledgerdb_state::tip_tests::read_reward_update_deltas_rejects_malformed_delta_fees`; `ade_ledger::bootstrap_reward_update::tests::v3_round_trips_delta_fees`; `ade_ledger::bootstrap_reward_update::tests::v3_rejects_genuine_v2_blob`; `ade_ledger::bootstrap_reward_update::tests::tampered_delta_fees_breaks_the_commitment`; `ade_ledger::bootstrap_reward_update::tests::decode_rejects_unknown_version`; `ade_ledger::epoch_accumulator::tests::seed_boundary_reduces_fee_pot_by_delta_fees`; `ade_ledger::epoch_accumulator::tests::seed_boundary_fee_reduction_underflow_fails_closed`; `ade_ledger::epoch_accumulator::tests::non_seed_boundary_never_reduces_fee_pot`; `ade_ledger::epoch_accumulator::tests::codec_rejects_unknown_version` |
| **CI** | `ci/ci_check_bootstrap_rupd_fee_reduction.sh` |

#### `DC-EPOCH-24` — _enforced_

| Aspect | Location |
|--------|----------|
| **Source** | CE-3d go pool-set inclusion slice (residual C). The differential exposed 32 phantom pools present ONLY in Ade's self-derived go (each a registered pool with a registered, zero-stake delegator), absent from the cardano reference decoded by Ade's own read_stake_snapshot_full. Root cause: build_boundary_mark_snapshot + aggregate_pool_stake inserted Coin(0) for every delegation -- a deliberately-encoded but WRONG numDelegators>0 rule that conflated the derived PoolDistr with the serialized ssActiveStake. cardano-ledger libs/cardano-ledger-core Stake.hs resolveActiveInstantStakeCredentials / getNonZeroActiveStakeWithDelegation; SnapShots.hs snapShotFromInstantStake. |
| **Requirement** | Snapshot pool-set inclusion = cardano's ssActiveStake NonZero membership (CE-3d). The per-epoch stake snapshot (mark/set/go) INCLUDES a registered+delegated stake credential IFF its combined active stake (base-address UTxO coin + reward-account balance) is NON-ZERO, and OMITS it otherwise -- so a pool whose ONLY delegators are all zero-stake is structurally ABSENT from the snapshot (no `delegations` entry, no `pool_stakes` entry). This is cardano-ledger's `resolveActiveInstantStakeCredentials` (`Stake.hs`): `ssActiveStake` is a `NonZero`-typed VMap -- `getNonZeroActiveStakeWithDelegation` drops a delegated credential whose account balance is zero when it has no UTxO, and a credential WITH a UTxO is >= minUTxO (always non-zero). It is the SERIALIZED go/set/mark SnapShot representation, DISTINCT from the DERIVED `PoolDistr` whose explicit `numDelegators>0` count keeps a 0-stake pool that has a delegator. (a) DECIDED AT CONSTRUCTION, NOT A POST-FILTER: the membership guard lives INSIDE `build_boundary_mark_snapshot` (the authoritative boundary mark that rotates into set then go) and `aggregate_pool_stake` (the reduced-checkpoint projection, DC-EVIEW-05) -- a credential with zero combined stake is never inserted, so no finished map is filtered after the fact. (b) BYTE-EQUAL TO THE DECODED REFERENCE: Ade's own decoder `read_stake_snapshot_full` reads cardano's serialized ssStake (`map(cred -> [coin, pool])`, always non-zero) and aggregates it, so the self-derived snapshot's pool SET, per-pool values, per-credential `delegations`, and the canonical snapshot fingerprint (`write_stake_snapshot` over delegations + pool_stakes) all equal the cardano reference. (c) REWARD / LEADERSHIP NEUTRAL: a zero-stake credential earns zero member reward (`floor((f-c)*(1-m)*0/sigma)=0`, gated by `if member_reward > 0`) and a zero-stake pool has zero leader probability, so the omission changes NO reward and NO leader schedule and leaves the go TOTAL unchanged -- only the canonical map cardinality, the serialized snapshot bytes, and the hash. (d) PERSISTED-SEMANTICS REJECT (compatibility/recovery slice): the (b) construction change makes a PERSISTED mark/set/go an authoritative serialized artifact whose inclusion semantics is version-bound. A v3 store's marks were built under the prior `numDelegators>0` rule (phantom 0-stake pools), so `EPOCH_ACCUMULATOR_SCHEMA_VERSION` is bumped 3 -> 4; a pre-C v3 (or v1/v2/unversioned) store fails closed on decode (`UnknownVersion`, surfaced fail-closed by `EpochAccumulatorStore::load_current`). A warm-start therefore NEVER RELOADS a stale snapshot-inclusion semantics and never carries it forward (a reloaded pre-C mark would otherwise stay non-reference-equivalent until it rotates out two boundaries later). Persisted authority has ONE unambiguous replay meaning; the only migration is an explicit fresh re-bootstrap under v4 -- never a silent reinterpretation. |
| **Code** | crates/ade_ledger/src/epoch_accumulator.rs (build_boundary_mark_snapshot: `if stake == 0 { continue }` before inserting into delegations/pool_stakes -- the authoritative boundary mark; EPOCH_ACCUMULATOR_SCHEMA_VERSION=4 -- a pre-C v3 store fails closed UnknownVersion in decode_epoch_accumulator). crates/ade_ledger/src/reduced_aggregate.rs (aggregate_pool_stake: `if cred_total.0 == 0 { continue }` before or_insert -- the reduced-checkpoint projection). crates/ade_runtime/src/chaindb/epoch_accumulator_store.rs (load_current surfaces the version-mismatch as a fail-closed Decode error -- no silent load). Reference decoder crates/ade_ledger/src/ledgerdb_state.rs read_stake_snapshot_full (reads cardano's non-zero serialized ssStake). Fingerprint crates/ade_ledger/src/fingerprint.rs write_stake_snapshot. ci/ci_check_snapshot_pool_set_inclusion.sh. |
| **Tests** | `ade_ledger::epoch_accumulator::tests::build_boundary_mark_snapshot_omits_zero_stake_credential`; `ade_ledger::reduced_aggregate::tests::delegated_zero_stake_pool_is_omitted`; `ade_ledger::reduced_aggregate::tests::ssactivestake_membership_decision_table`; `ade_ledger::epoch_accumulator::tests::cross_epoch_boundary_per_credential_mark_pays_member_rewards`; `ade_ledger::epoch_accumulator::tests::codec_rejects_pre_c_v3_store_rebootstrap_required` |
| **CI** | `ci/ci_check_snapshot_pool_set_inclusion.sh` |

#### `DC-EPOCH-25` — _declared_

| Aspect | Location |
|--------|----------|
| **Source** | docs/clusters/LIVE-LEDGER-EPOCH-TRANSITION/SLICE-S4-PRE-FROZEN-LEADERSHIP-DISTRIBUTION.md + SLICE-S4-PRE-1C-LEADERSHIP-BOOTSTRAP-LINEAGE.md. Root: the S4 same-epoch identity gate FAILED (from_accumulator go+active-params produced 626/627 pools vs the seed's 659; 1 leadership pool had no active params), and the Leadership Distribution Authority Trace (ce3d_boundary_differential::ldat_classify_leadership_pools) proved leadership = SET-snapshot stake + snapshot-frozen pool params/VRF, incl. a retired 1M-ADA pool whose VRF is absent from active state. |
| **Requirement** | Self-contained frozen leadership authority (S4-pre). Cardano's leadership PoolDistr (nesPd) -- the per-pool (active_stake, vrf_keyhash) that decides the leader schedule -- is answered by a self-contained, persisted FrozenLeadershipPoolDistr, and by NOTHING else. `to_pool_distr_view` reads stake AND VRF DIRECTLY from that frozen object; it NEVER re-derives them at leadership-use time from the go snapshot, the active cert_state.pool.pools params, future_pools, or the retiring map. Deriving leadership from go stake + active params is a DISPROVEN hypothesis (LDAT): leadership stake is the SET snapshot and includes zero-stake registered pools AND retired/POOLREAP'd pools whose VRF is ABSENT from active state -- the active params drop a real leadership-relevant pool's VRF (proven: exactly 1 retired 1M-ADA pool on the v5 seed). The disproven builder is quarantined test-only as `from_accumulator_go_active_params_for_test_only`. (a) CANONICAL + FAIL-CLOSED CODEC: `encode_frozen_leadership` / `decode_frozen_leadership` (`array(6)[version, target_leadership_epoch, source_slot, source_hash, source_checkpoint_commitment, map{ pool_keyhash -> array(2)[active_stake, vrf_keyhash] }]`, FROZEN_LEADERSHIP_SCHEMA_VERSION=6; S4-L2 added `source_checkpoint_commitment` -- the reduced-checkpoint commitment finalized AT source_slot/source_hash, captured at freeze time, so the promoted candidate authority reads its leader schedule's provenance DIRECTLY, never a live/historical checkpoint lookup) + `canonical_hash` (blake2b-256). Decode fails closed on unknown version, wrong shape, duplicate / non-canonical pool-key order, field overflow, trailing bytes, or any non-byte-canonical encoding (re-encode != input); zero-stake pools are preserved. (b) DURABLE, EPOCH-INDEXED, SEPARATE FROM THE ACCUMULATOR BLOB: persisted in EpochAccumulatorStore under a store-level leadership-schema-v6 marker + the canonical object, INDEXED BY target_leadership_epoch, written in ONE atomic redb commit. The accumulator BLOB codec is UNCHANGED (still v4-decodable), so non-authority observe-only follow still reads existing stores. The SOLE leadership read is the EXACT epoch-indexed authority `leadership_authority_for_epoch(e)`, which returns ONLY the object whose target_leadership_epoch == e and fails closed otherwise: OldAccumulatorSchemaNotLeadershipCertified (legacy v4 / no marker), LeadershipEpochNotSealed (no object sealed for e -- NEVER a "latest / current / nearest" fallback), LeadershipEpochMismatch (a mis-keyed store), MissingFrozenLeadershipDistr (torn), or MalformedFrozenLeadershipDistr (corrupt). NO production read of "the current leadership object" exists. `reset_to_bootstrap` RESTORES current := bootstrap (the two-key model: the native post-boundary epochs are dropped and the refold re-produces them -- replay-equivalent -- never a stale post-boundary object). (c) SOURCE-BOUND BOOTSTRAP IMPORT: the native first-run bootstrap seeds the epoch-indexed bootstrap-certified initial condition via `seal_bootstrap_leadership_epochs` -- nesPd_{seed} from the manifest-bound seed record's pool_distribution AND nesPd_{seed+1} from the imported MARK snapshot (`s1a.mark_pool_distr`) -- each written to BOTH the bootstrap and current epoch tables in one commit, with a no-duplicate-epoch check and an encode->decode canonical self-check (FrozenLeadershipCanonicalDecodeFailed) before any write. The SOURCE binding to the certified bootstrap point is enforced at the native_firstrun call site (the seal is skipped on a foreign lineage). The seed record's pool_distribution IS the leadership nesPd (byte-exact: from_frozen == the seed leadership PoolDistr, 659/659, incl. zero-stake + the retired 1M-ADA pool's frozen VRF), durable + replay-equivalent (byte-identical canonical hash across clean advance, within-k rollback+reset+refold, and warm restart). RECURRENCE (S4-pre-2): at each self-derived epoch boundary the node FREEZES the next epoch's leadership nesPd as an authoritative boundary effect (EpochBoundaryEffect::FreezeLeadership), sealed atomically with the accumulator advance (one redb commit). The pool SET is numDelegators>0 -- the pre-POOLREAP delegation-map image INTERSECT the registered pools (DC-EPOCH-24's derived-PoolDistr membership, DISTINCT from the full registered set: 703 registered but 658 in nesPd) -- INCLUDING zero-stake-with-delegator + retiring pools; stake is the just-built mark's per-pool stake (0 if absent); VRF is the pre-POOLREAP frozen params (capture-time, never a use-time active-param lookup). PROVEN byte-exact vs the cardano reference nes[5]: boundary 1340->1341 froze target_leadership_epoch=1342 == POST-1342 nesPd 658/658. A boundary advance NEVER commits without its matching frozen leadership (RED atomic enforcement); a reset restores current := bootstrap (never a stale post-boundary object). BRIDGE (S4-0): leadership is stored in TWO epoch-indexed tables -- bootstrap_leadership_by_epoch (the immutable bootstrap-certified initial condition) + current_leadership_by_epoch (the bootstrap epochs UNION the native boundary freezes) -- and read ONLY by exact target epoch (above). The bootstrap seeds the certified initial condition that native freezes cannot: nesPd_{seed} from the seed record's pool_distribution AND nesPd_{seed+1} from the imported MARK snapshot -- the ONE epoch (e.g. 1339) no native freeze produces (the cross into seed+1 freezes nesPd_{seed+2}); native freezes cover seed+2 and beyond. There is NO gap: for every epoch E the node validates, nesPd_E is either bootstrap-seeded (seed / seed+1) or native-frozen (seed+2+, from the cross into E-1). PROVEN across the full band 1338..=1342: exact-index reads return the object whose target == the queried epoch (bootstrap 1338 seed-record + 1339 MARK, native 1340/1341/1342), each byte-stable across reopen, with off-band (1337/1343) + a legacy store failing closed, and a reset restoring current := bootstrap. These slices PERSIST + certify + RECUR + recover + EPOCH-INDEX the frozen leadership authority behind a SOLE exact-epoch read; they do NOT yet promote it to the production leader-schedule source (that swap -- retiring the three PoolDistrView::from_seed_epoch_consensus_inputs read sites in favour of leadership_authority_for_epoch(slot_epoch) + deleting the seed+2 ceiling in epoch_wire.rs + adding a seed-authority-resurrection guard -- is S4 proper, a separate authority-promotion slice; DONE in S4-L2, below). PROMOTION (S4-L2): the FORWARD promotion path is frozen-only too. prepare_authority_for_candidate_slot sources candidate leadership BEYOND the bootstrap bridge (candidate >= seed+2) SOLELY from promotion_leadership_authority_for_epoch(candidate) (promotion-certified = current-present AND bootstrap-absent, else NotPromotionCertified) -> from_frozen_leadership over the frozen object's OWN freeze-time source point + source_checkpoint_commitment (leadership-free metadata; stake/VRF/pool set read ONLY from the frozen object). The retired seed+2 window-replay ceiling is DELETED: EVERY boundary past the bridge crosses through the frozen object (seed+2 AND the former-ceiling seed+3 proven). A missing store / unsealed / non-promotion-certified / malformed object is a fail-closed terminal (PromotionLeadershipUnavailable / LeadershipEpochNotSealed / NotPromotionCertified / MalformedFrozenLeadershipDistr), NEVER a window-replay or seed fallback. The accumulator store is threaded run_node_sync -> prepare_authority_for_candidate_slot; the run-loop freeze captures source_checkpoint_commitment = the reduced checkpoint finalized AT the mark source point (s_prev), never a fabricated zero. The observe/discard boundary path constructs NO leadership effect; only the effect-producing path (with a real commitment) freezes. |
| **Code** | crates/ade_ledger/src/frozen_leadership.rs (FrozenLeadershipPoolDistr, LeadershipPoolEntry, to_pool_distr_view, from_seed_epoch_consensus_inputs, encode_frozen_leadership / decode_frozen_leadership / canonical_hash, FrozenLeadershipError, FROZEN_LEADERSHIP_SCHEMA_VERSION=6 + the v6 source_checkpoint_commitment field on FrozenLeadershipPoolDistr / from_seed_epoch_consensus_inputs / from_boundary_snapshot / from_mark_pool_distr). crates/ade_runtime/src/chaindb/epoch_accumulator_store.rs (S4-0 epoch-indexed: leadership_authority_for_epoch, seal_current_leadership, seal_bootstrap_leadership_epochs, frozen_leadership_for_epoch, bootstrap_frozen_leadership_for_epoch, LeadershipAuthorityError -- incl. LeadershipEpochNotSealed / LeadershipEpochMismatch / DuplicateBootstrapLeadershipEpoch -- re-exported from chaindb; LEADERSHIP_SCHEMA_KEY + CURRENT_LEADERSHIP_BY_EPOCH + BOOTSTRAP_LEADERSHIP_BY_EPOCH tables). crates/ade_node/src/native_firstrun.rs (native_first_run_bootstrap seals the leadership beside seal_bootstrap). S4-pre-2 boundary freeze: crates/ade_ledger/src/frozen_leadership.rs (from_boundary_snapshot -- numDelegators>0 delegation-image pool set INTERSECT registered, stake from the just-built mark, VRF from the pre-POOLREAP frozen params). crates/ade_ledger/src/epoch_accumulator.rs (cross_epoch_boundary_with_effect captures the pre-POOLREAP delegation image + registered VRF then builds nesPd_{target+1}; EpochBoundaryEffect::FreezeLeadership; apply_selected_block_with_effects; validate_boundary_effects; typed terminals BoundaryLeadershipEffectInvariant / BoundaryLeadershipSnapshotUnavailable / LeadershipEpochOverflow). crates/ade_runtime/src/chaindb/epoch_accumulator_store.rs (advance_with_current_leadership -- one-commit atomic seal of accumulator + slot + anchor + CURRENT_LEADERSHIP_BY_EPOCH[target_leadership_epoch] + marker; two-key epoch-indexed model BOOTSTRAP_LEADERSHIP_BY_EPOCH + seal_bootstrap_leadership_epochs; reset_to_bootstrap RESTORES current := bootstrap, never preserves a stale post-boundary object). crates/ade_runtime/src/chaindb/epoch_accumulator_advance.rs (cross_accumulator_over_boundary_block consumes the FreezeLeadership effect, validates its bindings as typed BoundaryLeadership terminals, seals atomically). S4-L1 production flip (initial/warm leadership view): crates/ade_node/src/node_lifecycle.rs (leadership_view_from_frozen_authority -- the SOLE initial/warm leadership read = leadership_authority_for_epoch(record.epoch_no).to_pool_distr_view(asc); sites 658/840/3397 flipped; warm_start_recovery takes epoch_accumulator; ProductionLeadershipAuthorityUnavailable fail-closed, NO seed fallback; the legacy first-run route seals leadership from the seed record beside the native route's seal). Quarantined disproven builder crates/ade_ledger/src/consensus_view.rs (from_accumulator_go_active_params_for_test_only -- test-only). S4-L2 promotion flip (the forward candidate authority): crates/ade_runtime/src/chaindb/epoch_accumulator_store.rs (promotion_leadership_authority_for_epoch -- promotion-certified = current-present AND bootstrap-absent, else NotPromotionCertified). crates/ade_ledger/src/reduced_epoch_view.rs (FrozenLeadershipViewMetadata -- leadership-free; EpochConsensusView::from_frozen_leadership reads stake/VRF/pool set ONLY from the frozen object). crates/ade_node/src/epoch_wire.rs (prepare_authority_for_candidate_slot candidate>=seed+2 promotes via promotion_leadership_authority_for_epoch -> from_frozen_leadership; the seed+2 window-replay ceiling DELETED; store REQUIRED = PromotionLeadershipUnavailable else, no fallback; epoch_accumulator threaded through the signature). crates/ade_node/src/node_sync.rs (run_node_sync threads the accumulator store to the pump). crates/ade_node/src/node_lifecycle.rs (run-loop freeze captures source_checkpoint_commitment = reduced checkpoint finalized AT the mark source s_prev, passed to cross_accumulator_over_boundary_block). crates/ade_ledger/src/epoch_accumulator.rs (three-layer freeze split: cross_epoch_boundary_transition observe-only -> NO effect; cross_epoch_boundary_with_effect requires a real SourceCheckpointCommitment). ci/ci_check_frozen_leadership_authority.sh + ci/ci_check_frozen_promotion_no_seed_window.sh. |
| **Tests** | `ade_ledger::frozen_leadership::tests (codec round-trip, stable + content-bound hash, zero-stake preserved, wrong-version / duplicate / unsorted / trailing rejected; to_pool_distr_view reads stake+VRF directly)`; `ade_ledger::frozen_leadership::tests::from_boundary_snapshot_is_the_delegation_image_not_the_full_registered_set (numDelegators>0 membership: a registered-but-undelegated pool is excluded)`; `ade_ledger::epoch_accumulator::tests::boundary_leadership_effect_batch_invariants_are_enforced (ordered/no-dup/labeled effect, typed terminal)`; `ade_runtime::chaindb::epoch_accumulator_store::tests (S4-0 epoch-indexed: seal_current_and_read_exact_by_epoch, epoch_indexed_leadership_survives_reopen, leadership_authority_fails_closed_on_legacy_store, leadership_authority_rejects_missing_epoch_under_valid_marker, leadership_authority_rejects_wrong_epoch_object, leadership_authority_rejects_wrong_version_marker, leadership_authority_rejects_malformed_object, seal_bootstrap_leadership_epochs_rejects_duplicate_epoch)`; `ade_runtime::chaindb::epoch_accumulator_store::tests::reset_to_bootstrap_restores_only_bootstrap_indexed_leadership (two-key epoch-indexed model: reset restores current := bootstrap, the native post-boundary epochs dropped)`; `ade_runtime::chaindb::epoch_accumulator_store::tests::reset_clears_current_leadership_when_no_bootstrap_object`; `ade_runtime::chaindb::epoch_accumulator_store::tests::seal_advance_reset_round_trip_is_exact (accumulator seal -> advance -> reorg reset exact round-trip; advance_with_current_leadership seals accumulator + leadership in one redb commit)`; `ce3d_boundary_differential::s4pre_frozen_leadership_seed_identity`; `ce3d_boundary_differential::s4pre_1c_frozen_leadership_bootstrap_lineage`; `ce3d_boundary_differential::s4_0_epoch_indexed_leadership_acceptance_1338_to_1342 (full leadership band 1338..=1342 read by EXACT index: bootstrap 1338 seed-record + 1339 MARK, native 1340/1341/1342; distinct objects, byte-stable across reopen, reset restores current := bootstrap, off-band 1337/1343 + legacy store fail closed)` … (+6 more) |
| **CI** | `ci/ci_check_frozen_leadership_authority.sh`; `ci/ci_check_frozen_promotion_no_seed_window.sh`; `ci/ci_check_frozen_recovery_no_seed_window.sh` |

### DC-EVIDENCE

#### `DC-EVIDENCE-01` — _enforced_

| Aspect | Location |
|--------|----------|
| **Source** | docs/planning/phase4-n-m-c-operator-pass-invariants.md §1 (I-C12) |
| **Requirement** | Operator-pass live evidence: the C5 live operator pass against the local docker cardano-node-preprod peer produces a JSONL transcript containing AT LEAST: - 1 AdmissionStarted with consensus_inputs_fingerprint, - 1 BootstrapComplete with the fingerprint, - >= 1 BlockAdmitted with the fingerprint, - >= 1 AgreementVerdict { kind: "agreed" }. And AT MOST: - 0 AgreementVerdict { kind: "diverged" } (would mean divergence vs. live preprod — release-blocking), - 0 BlockAdmitted for any block whose hash differs from a block the live peer announces at the same slot. The Lagging count is unconstrained (Lagging is evidence-only; DC-ADMIT-08). |
| **Code** | crates/ade_node/tests/admission_live_operator_pass.rs (env-gated integration test with the closed transcript-shape asserts), ci/build_consensus_inputs_bundle.sh (operator-side bundle generator), crates/ade_runtime/src/seed_import/ (full preprod UTxO importer; PHASE4-N-M-A1.1 closes the prior A1.1 reference-script gate AND the A1.2 Byron-address gate), docs/evidence/phase4-n-m-c-consensus-inputs.json (committed bundle from epoch 179 docker preprod), docs/evidence/phase4-n-m-c-wire-only-transcript.jsonl (wire-integration anchor), docs/evidence/phase4-n-m-a1.1-admission-bootstrap-transcript.jsonl (1.9M-entry full preprod bootstrap transcript), docs/evidence/phase4-n-m-c-operator-pass-README.md (runbook + RO-LIVE-05 bounded statement) |
| **Tests** | `live_operator_pass_against_docker_preprod`; `live_bundle_imports_with_conway_era_and_deterministic_fingerprint` |
| **CI** | `ci/ci_check_live_operator_pass_scaffold.sh` |

#### `DC-EVIDENCE-02` — _enforced_

| Aspect | Location |
|--------|----------|
| **Source** | docs/planning/phase4-n-m-c-operator-pass-invariants.md §1 (I-C11) |
| **Requirement** | Adversarial false-accept rejection across 4 mandatory mutation classes: 1. Body byte flip preserving envelope shape 2. Header body-hash mismatch 3. KES / signature corruption 4. VRF proof or output tamper Each mutation produces either: (a) BlockAdmitted NOT emitted + AgreementVerdict::Diverged + exit code 30, OR (b) AdmissionHalted { reason: PeerSentUndecodableBytes } when corruption breaks decode before admit-attempt. In NO case may a mutation produce BlockAdmitted (false accept), Agreed, or InputNotFound. False-accept is release-blocking (memory [[feedback-fail-closed-validation]]). |
| **Code** | crates/ade_node/tests/admission_adversarial_corpus.rs (4 mandatory MutationClass variants applied to a real Conway block; each asserts exit in {Diverged(30), PeerSentUndecodableBytes(34)}) |
| **Tests** | `adversarial_corpus_rejects_all_four_mutation_classes` |
| **CI** | `ci/ci_check_adversarial_false_accept_corpus.sh` |

#### `DC-EVIDENCE-03` — _enforced_scaffolding_

| Aspect | Location |
|--------|----------|
| **Source** | docs/planning/phase4-n-aj-participant-convergence-evidence-invariants.md §1 (I-AJ-5) + §9 |
| **Requirement** | Convergence-through-reorg transcript shape (CE-AI-6; PHASE4-N-AJ). The participant convergence pass produces ONE JSONL transcript with AT LEAST: - a strict slot regression in the OBSERVED PEER BLOCK sequence (a peer RollBackward was actually followed), and - >= 1 AgreementVerdict { kind: "agreed" } at the re-converged tip. And AT MOST: - 0 AgreementVerdict { kind: "diverged" }. The .md manifest binds the .jsonl sha256. Vacuous-until-committed; validated by ci/ci_check_convergence_evidence_schema.sh. A boring same-tip-only run (no regression) is NOT sufficient. SINGLE-BEST-PEER scope -- NOT full multi-peer Cardano ChainSel. |
| **Code** | ci/ci_check_convergence_evidence_schema.sh (gate), docs/evidence/phase4-n-ai-convergence-pass.{jsonl,md} (operator-produced, post-AJ), docs/active/phase4-n-ai-convergence-runbook.md |
| **Tests** | _(no tests listed — gap)_ |
| **CI** | `ci/ci_check_convergence_evidence_schema.sh` |

#### `DC-EVIDENCE-04` — _enforced_

| Aspect | Location |
|--------|----------|
| **Source** | docs/planning/phase4-n-ao-ce-ao-6-live-gap.md (S9 evidence finding) + docs/clusters/PHASE4-N-AO/S9-closed-fork-choice-evidence.md |
| **Requirement** | Closed fork-choice convergence evidence (PHASE4-N-AO S9; promotes the live SELECT proof from stderr diagnostics to registry-grade evidence). The live multi-candidate SELECT path emits a CLOSED, observe-only convergence-evidence sequence proving the WHOLE path: needs_fork_choice -> lca_discovered -> candidate_fragment_built -> fork_choice_selected -> branch_fetch_started -> branch_fetch_completed -> branch_prevalidated -> fork_switch_applied \| fork_switch_failed \| fork_switch_superseded. The 10 event discriminators EQUAL the emit-only allow-list (an unknown/added variant fails closed at the allow-list test, the NodeSchedEvent pattern); every field is bounded + typed (no free-form error strings -- failure_code is a closed enum mapping BranchProofError/LcaError; fork_switch_id is a bounded deterministic id = blake2b(winning_peer \|\| fork_anchor.slot \|\| fork_anchor.hash \|\| winner_tip.slot \|\| winner_tip.hash) hex-prefix, never free-form text). For a given fork_switch_id, a fork_choice_selected{result=win} is followed by EXACTLY ONE terminal event (fork_switch_applied OR fork_switch_failed OR fork_switch_superseded -- the last when a newer win on the same fork overwrites this provisional pending before the relay loop applies it) -- never zero, never two, never dangling. The evidence OBSERVES already-computed authority outcomes: NO evidence event/type is ever consumed by select_best_chain / walk_to_durable_lca / apply_fork_switch / the forge fence, and a transcript write failure (flips incomplete, DC-EVIDENCE-01) cannot alter selection / apply / fence behavior. CN-CONS-03 flips ONLY when a committed two-producer transcript passes the refined bounded post-switch window (S10): the hard fork-switch proof (both-peer block_received -> the SELECT middle -> fork_switch_applied{rollback_reason=ForkChoiceWin} at X -> block_admitted X) then PostSwitchContinuity::ContinuesSelectedBranch within the bounded window with a terminal of agreement_verdict{agreed, our_hash==peer_hash} at X-or-descendant OR a validated-prefix-of-peer (continuity holds + peer observed ahead), 0 diverged -- never on stderr diagnostics, and never on a lucky exact-tip moment (see DC-EVIDENCE-05 for the replayable continuity verdict the terminal is derived from). |
| **Code** | crates/ade_node/src/admission_log/{event.rs,writer.rs} (closed fork-choice AdmissionLogEvent variants + DISCRIMINATORS allow-list + closed ForkChoiceResult/ForkChoiceEvidenceFailure) + crates/ade_node/src/convergence_evidence.rs (observe-only emitters + bounded fork_switch_id) + crates/ade_node/src/node_lifecycle.rs (observe-only decide/apply taps; never read back by authority). Gate ci/ci_check_fork_choice_evidence_closed.sh. |
| **Tests** | `fork_choice_win_paired_with_exactly_one_terminal_applied`; `fork_choice_win_failed_terminal_carries_closed_code`; `superseded_win_pairs_to_superseded_terminal`; `fork_switch_id_is_deterministic_and_bounded` |
| **CI** | `ci/ci_check_fork_choice_evidence_closed.sh` |

#### `DC-EVIDENCE-05` — _enforced_

| Aspect | Location |
|--------|----------|
| **Source** | docs/clusters/PHASE4-N-AO/S10-post-switch-branch-continuity.md (S10) + docs/planning/phase4-n-ao-ce-ao-6-live-gap.md |
| **Requirement** | Replayable post-switch branch-continuity verdict (PHASE4-N-AO S10). After a ForkChoiceWin adoption at tip X, a GREEN pure reducer derive_post_switch_continuity(events) -> PostSwitchContinuity classifies Ade's OWN validated admitted-block lineage into a closed verdict { ContinuesSelectedBranch \| Diverged \| BrokenLineage \| DanglingForkChoiceWin \| InsufficientEvidence } -- no free-form strings. ContinuesSelectedBranch REQUIRES: unbroken prev_hash lineage from X across every post-X block_admitted (admitted[i].prev_hash == admitted[i-1].hash; the first post-switch follow block chains to X.hash), no diverged after X, and every fork_choice_selected{win} paired to a terminal (applied\|failed\|superseded). prev_hash is the admitted block's VALIDATED header field (decoded.prev_hash -- the same canonical parent link S7's walk consumes via get_block_by_hash), NEVER peer-supplied; the peer's tip is NEVER an input to the verdict. REPLAY-EQUIVALENT: given the same post-switch admitted-block bytes and the same applied fork-switch point, the reducer yields a byte-identical PostSwitchContinuity. The reducer is the SINGLE implementation behind both the hermetic replay test and the live CE-AO-6 gate (ci/ci_check_post_switch_convergence_window.sh invokes the post_switch_continuity bin) -- no Rust/Python drift. GREEN, observe-only: PostSwitchContinuity is NEVER consumed by select_best_chain / walk_to_durable_lca / apply_fork_switch / pump_block / the forge fence; the node does not use it to select chains -- CI and /cluster-close use it to enforce the release gate. |
| **Code** | crates/ade_node/src/post_switch_continuity.rs (derive_post_switch_continuity + closed PostSwitchContinuity verdict + evaluate_release_window, GREEN pure reducer) + crates/ade_node/src/bin/post_switch_continuity.rs (thin transcript->verdict bin) + prev_hash_hex on block_admitted (crates/ade_node/src/admission_log/{event,writer}.rs + convergence_evidence.rs emit_block_admitted/emit_admit_and_verdict) sourced from PumpTip.prev_hash (crates/ade_runtime/src/forward_sync/pump.rs) + ForkSwitchOutcome::Adopted.new_tip_prev + the fork-switch-adopt + admission-runner emit sites in crates/ade_node/src/node_lifecycle.rs + crates/ade_node/src/admission/runner.rs. BLUE selector/walk/apply/validate UNCHANGED. |
| **Tests** | `crates/ade_node/src/post_switch_continuity.rs::tests::continuity_ok_yields_continues_selected_branch`; `crates/ade_node/src/post_switch_continuity.rs::tests::broken_parent_link_yields_broken_lineage`; `crates/ade_node/src/post_switch_continuity.rs::tests::post_switch_diverged_yields_diverged`; `crates/ade_node/src/post_switch_continuity.rs::tests::win_without_terminal_yields_dangling`; `crates/ade_node/src/post_switch_continuity.rs::tests::continuity_verdict_ignores_peer_tip`; `crates/ade_node/src/post_switch_continuity.rs::tests::post_switch_continuity_replays_byte_identical`; `crates/ade_node/src/post_switch_continuity.rs::tests::release_window_passes_on_validated_prefix`; `crates/ade_node/src/post_switch_continuity.rs::tests::release_window_passes_on_agreed_descendant`; `crates/ade_node/src/post_switch_continuity.rs::tests::release_window_prefix_requires_a_followed_descendant`; `crates/ade_node/src/post_switch_continuity.rs::tests::release_window_terminal_outside_window_fails` … (+1 more) |
| **CI** | `ci/ci_check_post_switch_convergence_window.sh` |

### DC-EVIEW

#### `DC-EVIEW-01` — _enforced_

| Aspect | Location |
|--------|----------|
| **Source** | docs/clusters/EPOCH-CONSENSUS-VIEW/SLICE-1-redb-materialization-gate.md; docs/clusters/EPOCH-CONSENSUS-VIEW/EPOCH-CONSENSUS-VIEW-design-analysis.md (Deliverable 6, the gate) |
| **Requirement** | Transient epoch-view replay storage is GREEN / non-authoritative substrate. A bounded, disk-backed, TRANSIENT redb store (TransientEpochViewStore) may be materialized to form a next-epoch consensus view and is then pruned -- it may NEVER survive as authority, influence BLUE outputs directly, or become a fallback source for follow, forge, recovery, or snapshot activation. Five sub-invariants are mechanically enforced. GATE-MEM: the store is disk-backed with a bounded owned-RssAnon delta -- materializing a committed corpus (CORPUS_N) keeps the bulk off the anonymous heap (delta < a FIXED committed ceiling RSS_ANON_DELTA_CEILING_KIB), while UtxoAnchor::len()==CORPUS_N proves the entries live on disk. GATE-CRASH: create -> materialize -> iterate -> dispose is crash-safe -- a SIGKILL at any point (mid-materialize or mid-dispose) leaves no transient store treated as authority, the durable tip + WAL digest + checkpoint digest UNCHANGED, the next normal replay producing IDENTICAL verdicts, and the transient root empty before normal operation resumes. GATE-PURGE: startup purge is fail-closed -- enumerate ONLY the owned transient-epoch-view subtree, validate every candidate name against the deterministic window-key form, delete all, fsync the parent, continue ONLY when empty; any failure (delete / dir-fsync / name-validation) is a STRUCTURED TERMINAL failure (TransientViewError), never best-effort. GATE-NO-FALLBACK: no live follow/forge/recovery/snapshot source file references the transient store. GATE-NOT-LIVE: the slice does not enable track_utxo=true on the live producer path and adds no runtime --transient-view-dir flag (D1: a fixed owned subtree derived from the data root, no consensus-adjacent config surface). The window key is deterministic (blake2b over network\|era\|epoch\|source-chain-point\|checkpoint-commitment; no rand/uuid), binding the store identity to the bound-activation bindings. |
| **Code** | crates/ade_runtime/src/chaindb/transient_epoch_view.rs: TRANSIENT_SUBTREE (the fixed owned subtree) + transient_root / transient_root_for_test (D1) + window_key (D2, blake2b over the bound-activation bindings, length-prefixed) + is_valid_window_key (D2 validator) + purge_transient_root (D3 fail-closed: enumerate -> validate -> delete -> fsync_dir -> empty-or-TransientViewError) + TransientEpochViewStore{open, materialize_batch, len, is_empty, iter_window, on_disk_bytes, dispose(self)} over the redb UtxoAnchor (default Immediate durability) + TransientViewError (structured terminal). crates/ade_runtime/src/bin/transient_view_kill_target.rs + crates/ade_runtime/tests/transient_view_kill_harness.rs: the deterministic SIGKILL crash harness (mid-materialize/mid-dispose) with the four-part durable-digest + replay-verdict assertion. crates/ade_runtime/tests/transient_view_memory.rs: the GATE-MEM bounded-materialization gate (committed CORPUS_N + fixed RSS_ANON_DELTA_CEILING_KIB, owned RssAnon from /proc/self/status). ci/ci_check_transient_view_memory_ceiling.sh + ci/ci_check_transient_view_no_fallback.sh + ci/ci_check_transient_view_not_live.sh. |
| **Tests** | `transient_root_is_owned_subtree_of_data_root`; `window_key_is_deterministic_and_binding_sensitive`; `window_key_validator_accepts_only_the_deterministic_form`; `purge_removes_valid_named_leftovers_and_leaves_subtree_empty`; `purge_fails_closed_on_a_foreign_artifact_and_deletes_nothing`; `purge_is_clean_on_an_empty_or_absent_subtree`; `lifecycle_open_materialize_iterate_dispose`; `dispose_is_clean_when_window_already_removed`; `transient_crash_mid_materialize_smoke`; `transient_crash_mid_dispose_smoke` … (+5 more) |
| **CI** | `ci/ci_check_transient_view_memory_ceiling.sh`; `ci/ci_check_transient_view_no_fallback.sh`; `ci/ci_check_transient_view_not_live.sh` |

#### `DC-EVIEW-02` — _enforced_

| Aspect | Location |
|--------|----------|
| **Source** | docs/clusters/EPOCH-CONSENSUS-VIEW/SLICE-2-stake-reference-classification.md; docs/clusters/EPOCH-CONSENSUS-VIEW/EPOCH-CONSENSUS-VIEW-design-analysis.md (Deliverable 1/2, classification matrix) |
| **Requirement** | Typed, era-gated stake-reference classification. Given canonical address bytes and a TYPED era / protocol-version context BOUND to the block being processed (CardanoEra, from era_schedule.locate(slot).era -- never inferred from the address bytes, local config, wall-clock, or a caller-selected flag), classify_output_stake_ref returns ONE deterministic typed result StakeRefClass = Base(StakeCredential) \| Pointer(PointerRef) \| Null \| Reject(StakeRefReject). It is the per-output attribution PRIMITIVE: it extracts the stake reference only, resolves nothing, sums nothing, and NO result directly changes stake totals (aggregation is Slice 3). No fixed byte offset is the contract across variants/eras: classification routes through the typed decode_address chokepoint plus per-form structural validation. (a) Base 0-3: the staking credential is read (key/script per header bit 5) ONLY after the form is validated to header(1) + payment(28) + stake(28) = 57 bytes. (b) Pointer 4-5 is ERA-GATED -- pre-Conway it decodes to an UNRESOLVED PointerRef{slot,txIx,certIx} that implies no credential / contribution / eligibility; at Conway (protocol major 9+, i.e. era >= CardanoEra::Conway) pointer stake is RETIRED -> Null (the address is spendable, contributes 0). (c) Enterprise 6-7 and Byron 8 are the semantic Null (valid non-staking forms, all eras). (d) Reward 14-15 is fail-closed Reject(RewardAddressNotValidAsOutput): a reward address is not a valid output payment address and must never be ordinary output stake (the full ledger rule is proven in Slice 3; here it is decoder-complete + fail-closed). Reject is DISTINCT from Null: Null is a valid non-staking form, Reject is invalid input or a wrong-position form -- a malformed / under-length / malformed-but-prefix-valid address is Reject(..), NEVER silently Null. Total, pure, deterministic (no HashMap/wall-clock/rand/float). |
| **Code** | crates/ade_ledger/src/stake_ref.rs: StakeRefClass{Base(StakeCredential)\|Pointer(PointerRef)\|Null\|Reject(StakeRefReject)} + PointerRef{slot,tx_index,cert_index} (decoded, unresolved) + StakeRefReject{Empty\|UnknownAddressType\|MalformedBase\|MalformedPointer\|RewardAddressNotValidAsOutput} + classify_output_stake_ref(addr_bytes,era: CardanoEra) (routes through ade_codec::address::decode_address; era-gated pointer retirement at era>=CardanoEra::Conway; base validated to 57 bytes before the staking part is read; reward fail-closed) + decode_pointer_coords / decode_varint (exact-consumption base-128 varint, overflow-guarded). ci/ci_check_eview_stake_ref_classification.sh. |
| **Tests** | `base_type0_is_stake_key_hash`; `base_type1_is_stake_key_hash`; `base_type2_is_stake_script_hash`; `base_type3_is_stake_script_hash`; `pointer_is_decoded_pre_conway_and_retired_at_conway`; `pointer_multibyte_varint_pre_conway`; `pointer_result_exposes_no_credential`; `enterprise_and_byron_are_null_all_eras`; `reward_address_is_rejected_not_summed`; `empty_is_reject_not_null` … (+8 more) |
| **CI** | `ci/ci_check_eview_stake_ref_classification.sh` |

#### `DC-EVIEW-03` — _enforced_

| Aspect | Location |
|--------|----------|
| **Source** | docs/clusters/EPOCH-CONSENSUS-VIEW/SLICE-3a-pointer-decode-resolution.md; docs/clusters/EPOCH-CONSENSUS-VIEW/SLICE-3-scope.md |
| **Requirement** | Era-parameterized pointer decoding + pre-Conway resolution, matching cardano-ledger EXACTLY (the wire authority -- CIP-19 is silent on canonicality, so the cardano-ledger implementation is the sole rule). A pointer address (header type 4/5) is header(1) \| payment(28) \| 3 base-128 big-endian varints (slot, txIx, certIx); the stored shape is (u32 slot, u16 txIx, u16 certIx). decode_pointer_address / decode_pointer_tail take a TYPED CardanoEra bound to the block (era_schedule.locate(slot).era -- never inferred from bytes / config / clock) and are ERA-GATED: (a) Conway (PV9+, era>=CardanoEra::Conway) -- STRICT: width-bounded varints (decode_width_bounded: at most ceil(bits/7) groups; a continuation past the max group count or a most-significant group whose surplus data bits exceed the field width is OverWidth), and trailing bytes are rejected. (b) Babbage (era==CardanoEra::Babbage) -- NORMALIZE: each coord decodes as a WRAPPING u64 (decode_u64_wrapping, bits past 64 dropped), then mkPtrNormalized clamps the WHOLE 3-tuple to (0,0,0) if ANY coord overflows its field width (not per-field masking, not wrapping the field); trailing bytes rejected. (c) <=Alonzo (era<CardanoEra::Babbage) -- NORMALIZE + crop trailing. In EVERY era a bounded leading-zero / non-minimal encoding (e.g. [0x80,0x01]==[0x01]) is ACCEPTED (the strict check is a WIDTH check, not a minimal-form check) -- reject-all-non-canonical would FALSE-REJECT txs cardano-node accepts. The parser exists in every era (pointers stay spendable post-Conway); only its strictness is era-gated; stake retirement (PV9 -> Null) is the SEPARATE Slice-2 rule. RESOLUTION (PointerMap, ade_ledger): a decoded Ptr resolves to the credential REGISTERED by the StakeRegistration cert at exactly (slot,txIx,certIx); pre-Conway only; fail-closed -- an unregistered coordinate -> None (no stake, never a fabricated credential), a duplicate coordinate -> rejected (no overwrite). Total, pure, deterministic. No live wiring, no aggregation. |
| **Code** | crates/ade_codec/src/address/pointer.rs: Ptr{slot u32,tx_index u16,cert_index u16} + PointerDecodeError{NotAPointerAddress\|TooShort\|TruncatedVarint\|OverWidth\|TrailingBytes} + decode_pointer_address / decode_pointer_tail (era-gated) + decode_pointer_strict / decode_width_bounded (Conway width-bounded) + decode_pointer_normalized / decode_u64_wrapping / normalize_ptr (Babbage/<=Alonzo clamp-3-tuple). crates/ade_ledger/src/pointer_resolve.rs: PointerMap{insert(fail-closed on duplicate), resolve -> Option<StakeCredential>, len, is_empty}. ci/ci_check_eview_pointer_compat.sh. |
| **Tests** | `bounded_leading_zero_alias_accepted_all_eras`; `conway_decodes_in_range`; `conway_rejects_txix_over_u16`; `conway_rejects_width_overflow_within_max_groups`; `conway_rejects_slot_over_u32`; `conway_rejects_trailing_bytes`; `conway_accepts_max_width_boundary`; `babbage_normalizes_overflow_to_zero_tuple`; `babbage_in_range_kept_unmodified`; `babbage_rejects_trailing_bytes` … (+10 more) |
| **CI** | `ci/ci_check_eview_pointer_compat.sh` |

#### `DC-EVIEW-04` — _enforced_

| Aspect | Location |
|--------|----------|
| **Source** | docs/clusters/EPOCH-CONSENSUS-VIEW/SLICE-3b-1-reduced-utxo-checkpoint.md; docs/clusters/EPOCH-CONSENSUS-VIEW/SLICE-3b-replay-window-materialization.md |
| **Requirement** | The durable reduced-UTxO checkpoint -- the "minimal native state" (S3b Option B). A disk-backed redb store of TxIn -> (Coin, ReducedStakeRef), built from Ade's bootstrap UTxO, that S3b-2 advances per epoch boundary and S3c aggregates. It is the SINGLE ledger authority's OWN reduced-UTxO projection -- a GREEN durable CACHE of a BLUE-derivable projection (a pure function of admitted blocks, reconstructible by replay if lost/corrupt) -- NOT a permanent parallel StakeView, NOT a second stake computation, and NEVER on the live follow/forge path (the live producer stays track_utxo=false; the checkpoint is built/advanced lazily off the per-block path). The reduced record drops datums/scripts/multi-asset, keeping (Coin, ReducedStakeRef); ReducedStakeRef is the Conway-specialized Base(StakeCredential) \| NonContributing (option b) -- Ade only snapshots at Conway, where pointer stake is retired and only base credentials contribute, so reduce_txout reuses classify_output_stake_ref(addr, Conway): Base(cred) -> Base, everything else (pointer/enterprise/Byron/reward/ malformed) -> NonContributing (the era gate is thus trivially satisfied; it is applied generally at S3c). CRASH-SAFE: build_from clears any prior partial build, writes all entries (durable redb commits), then writes the completeness marker LAST in a separate commit -- a crash before the marker leaves an INCOMPLETE checkpoint (is_complete()==false) that is rebuilt, never mistaken for complete. REPLAY-EQUIVALENT: the fingerprint is a hash chain over the canonical records (encode_reduced_record) in TxIn key order -- two builds from the same reduced UTxO yield a byte-identical checkpoint + fingerprint (DC-WAL-03 lineage). DURABLE across reopen. |
| **Code** | crates/ade_ledger/src/reduced_utxo.rs: ReducedStakeRef{Base(StakeCredential)\|NonContributing} + encode/decode + reduce_txout (reuses classify_output_stake_ref(.., Conway)) + encode_reduced_record (canonical TxIn\|coin\|ref). crates/ade_runtime/src/chaindb/reduced_utxo_checkpoint.rs: ReducedUtxoCheckpoint{open, build_from (clears prior -> entries -> marker LAST), is_complete, fingerprint, len, get} over a durable redb store (REDUCED_TABLE + META_TABLE completeness marker = fp(32)\|\|count(8)); ReducedCheckpointError{Redb\|Incomplete\|Decode}. ci/ci_check_eview_reduced_utxo_checkpoint.sh. |
| **Tests** | `base_output_reduces_to_base_credential`; `enterprise_and_byron_are_non_contributing`; `pointer_output_is_non_contributing_at_conway`; `reduced_stake_ref_round_trips_canonically`; `decode_fails_closed_on_bad_tag_or_truncation`; `record_encoding_is_deterministic_and_canonical`; `build_then_query_and_complete`; `durable_across_reopen`; `replay_equivalent_two_builds_byte_identical`; `fingerprint_changes_with_content` … (+2 more) |
| **CI** | `ci/ci_check_eview_reduced_utxo_checkpoint.sh` |

#### `DC-EVIEW-04b` — _enforced_

| Aspect | Location |
|--------|----------|
| **Source** | docs/clusters/EPOCH-CONSENSUS-VIEW/SLICE-3b-replay-window-materialization.md; docs/clusters/EPOCH-CONSENSUS-VIEW/SLICE-3b-1-reduced-utxo-checkpoint.md |
| **Requirement** | The windowed advance (S3b-2): advance the durable reduced-UTxO checkpoint (DC-EVIEW-04) per epoch boundary by replaying the epoch's admitted blocks, as the reduced PROJECTION of the ledger transition's OWN apply -- NOT a parallel reimplementation. reduced_block_delta is a FAITHFUL MIRROR of the ledger's track_utxo: it iterates the block's tx_bodies through the SAME extract_inputs_outputs_from_tx, removes the same spent TxIns, computes the same tx_hash = blake2b_256(tx_body_wire_bytes), and produces the same (tx_hash, output_index) keys -- emitting a bounded delta (spent, produced) whose produced outputs are REDUCED to (Coin, ReducedStakeRef) (S3b-1). The equality reduced_block_delta == reduce(track_utxo) is PROVEN on a REAL Conway block (not synthetic CBOR -- the real-interop discipline). The cert/delegation/pool/reward advance reuses the ledger's OWN process_block_certificates (advance_cert_state; single authority). The durable checkpoint advances via apply_block_delta (remove spent + insert produced), which INVALIDATES the completeness marker until finalize() recomputes it after the whole window -- a crash mid-window leaves an INCOMPLETE checkpoint that is rebuilt (the reduced UTxO is reconstructible by replay, DC-EVIEW-04), never a wrong stake snapshot from a partial advance. At Conway the S3a PointerMap is UNUSED (pointer outputs reduce to NonContributing), so the advance does not populate it. No live producer-path change; track_utxo=true stays out of the live path. |
| **Code** | crates/ade_ledger/src/reduced_advance.rs: ReducedBlockDelta{spent,produced} + reduced_block_delta (mirrors track_utxo, reuses extract_inputs_outputs_from_tx + reduce_txout + blake2b_256 tx_hash) + advance_cert_state (reuses crate::rules::process_block_certificates). crates/ade_runtime/src/chaindb/reduced_utxo_checkpoint.rs: ReducedUtxoCheckpoint::apply_block_delta (remove spent + insert produced, invalidates marker) + finalize (recompute marker). crates/ade_ledger/src/rules.rs: track_utxo / extract_inputs_outputs_from_tx / process_block_certificates made pub(crate) for the reuse. ci/ci_check_eview_windowed_advance.sh. |
| **Tests** | `reduced_delta_equals_reduce_of_track_utxo_on_real_conway_block`; `intra_block_chained_spend_cancels_phantom_matches_track_utxo`; `reduced_block_delta_is_deterministic`; `empty_block_yields_empty_delta`; `advance_cert_state_over_real_block_does_not_error`; `apply_block_delta_then_finalize`; `advance_over_real_conway_block_matches_build_from` |
| **CI** | `ci/ci_check_eview_windowed_advance.sh` |

#### `DC-EVIEW-05` — _enforced_

| Aspect | Location |
|--------|----------|
| **Source** | docs/clusters/EPOCH-CONSENSUS-VIEW/SLICE-3-scope.md (S3c); docs/clusters/EPOCH-CONSENSUS-VIEW/EPOCH-CONSENSUS-VIEW-design-analysis.md (Deliverable 3/7) |
| **Requirement** | Per-pool stake aggregation (S3c, the linchpin). aggregate_pool_stake computes the next-epoch per-pool active stake from the single ledger authority's own projection: the reduced checkpoint's per-base-credential UTxO coin sums (ReducedUtxoCheckpoint::sum_base_credential_stake -- folds ONLY ReducedStakeRef::Base entries; NonContributing is skipped) and the delegation map + reward balances (DelegationState, accumulated by the window's advance_cert_state via the ledger's own process_block_certificates). The cardano-ledger snapshot rule, Conway-specialized: iterate the REGISTERED+DELEGATED credentials (the delegations map -- an unregistered credential cannot delegate), and for each, active stake = sum(its base-address UTxO coin) + its reward-account balance, accumulated into its delegated pool; a credential with UTxO but no delegation contributes nothing; a delegated credential with a reward balance but no UTxO still contributes (Conway); a pool with >=1 delegator is INCLUDED even at 0 stake (ECA-0b: cardano numDelegators>0, count-not-amount, so the derived pool SET matches cardano's PoolDistr, not merely the likely-leader outcome); total_active_stake = the sum over pools. Pure, total, deterministic, FAIL-CLOSED on overflow (AggregateError::StakeOverflow via checked_add -- never a silently wrapped stake total; unreachable under the max-supply bound). OBSERVE-ONLY: the rewire of apply_epoch_boundary's new_mark stub (rules.rs:1098) to consume this aggregate, and feeding it to live leader election, are the activation slice (DC-EVIEW-08) -- NO live-path change here. ACCEPTANCE is the DIFFERENTIAL ORACLE: stake_by_pool == cardano-cli query stake-snapshot (stakeSet per pool) + total at >=2 Conway boundaries -- a LIVE gate, DECLARED (owed, run at activation; NOT faked green by the hermetic tests). |
| **Code** | crates/ade_ledger/src/reduced_aggregate.rs: StakeByPool{pool_stakes,total_active_stake} + AggregateError::StakeOverflow + aggregate_pool_stake(cred_utxo_stake, delegation) (iterates delegation.delegations, sums UTxO coin + reward per credential into its pool, checked_add fail-closed). crates/ade_runtime/src/chaindb/reduced_utxo_checkpoint.rs: ReducedUtxoCheckpoint::sum_base_credential_stake (folds only Base(cred) coins, fail-closed Overflow). ci/ci_check_eview_stake_aggregation.sh. |
| **Tests** | `sums_utxo_plus_reward_per_delegated_pool`; `reward_without_utxo_contributes`; `undelegated_credential_contributes_nothing`; `delegated_zero_stake_pool_is_included_with_zero` †; `multiple_pools_aggregate_independently`; `overflow_is_fail_closed`; `aggregation_is_deterministic`; `sum_base_credential_stake_skips_non_contributing` |
| **CI** | `ci/ci_check_eview_stake_aggregation.sh`; `ci/ci_check_eview_leadership_complete.sh` |

#### `DC-EVIEW-06` — _enforced_

| Aspect | Location |
|--------|----------|
| **Source** | docs/clusters/EPOCH-CONSENSUS-VIEW/SLICE-3-scope.md (S3d); docs/clusters/EPOCH-CONSENSUS-VIEW/EPOCH-CONSENSUS-VIEW-design-analysis.md (Deliverable 4) |
| **Requirement** | Snapshot formation + the k-immutability stability gate (S3d). form_mark_snapshot converts the S3c per-pool aggregate (StakeByPool) into the MARK StakeSnapshot's pool_stakes (the value leader election consumes). The mark/set/go rotation already exists (epoch::rotate_snapshots: mark<-new_mark, set<-old mark, go<-old set), encoding the lag -- leader election for epoch L reads the SET snapshot (the MARK captured at the previous boundary), a 2-epoch lag (LEADERSHIP_SNAPSHOT_PHASE = Set); GO drives rewards (3-epoch). S3d ADDS the STABILITY GATE Ade lacked: a boundary snapshot/view is FINALIZABLE (usable) ONLY once its boundary block is STRICTLY more than k (the SecurityParam, 2160) deep -- is_boundary_stable(boundary, tip, k) = (tip - boundary) > k (saturating; a boundary ahead of the tip is never stable). A boundary not yet > k deep can still be rolled back (DC-NODE-29), so a snapshot derived from it must NOT be used; cardano forces the lazy MARK only after one stability window for exactly this reason. Pure, total, deterministic. OBSERVE-ONLY: not wired to live leader election or the boundary authority (DC-EVIEW-08 activation); no live-path change. |
| **Code** | crates/ade_ledger/src/reduced_snapshot.rs: SnapshotPhase{Mark\|Set\|Go} + LEADERSHIP_SNAPSHOT_PHASE (Set) + form_mark_snapshot (StakeByPool -> StakeSnapshot.pool_stakes) + is_boundary_stable(boundary_block_no, tip_block_no, k: SecurityParam) = (tip - boundary) > k (saturating). Reuses crate::epoch::rotate_snapshots + StakeSnapshot. ci/ci_check_eview_stability_gate.sh. |
| **Tests** | `forms_mark_snapshot_from_aggregate`; `stability_gate_requires_more_than_k_deep`; `boundary_ahead_of_tip_is_not_stable`; `leadership_reads_the_set_snapshot`; `formed_mark_rotates_into_set` |
| **CI** | `ci/ci_check_eview_stability_gate.sh` |

#### `DC-EVIEW-07` — _enforced_

| Aspect | Location |
|--------|----------|
| **Source** | docs/clusters/EPOCH-CONSENSUS-VIEW/SLICE-3-scope.md (S3e); docs/clusters/EPOCH-CONSENSUS-VIEW/EPOCH-CONSENSUS-VIEW-design-analysis.md (Deliverable 5; the bound-activation prohibition) |
| **Requirement** | The bound, immutable EpochConsensusView (S3e). EpochConsensusView::bind emits the compact next-epoch consensus view from the finalized snapshot (S3d), BOUND to all of: network_magic, era, epoch, source_point (slot+hash), checkpoint_commitment (the reduced-UTxO checkpoint fingerprint, DC-EVIEW-04/04b), nonce (eta0), snapshot_phase (DC-EVIEW-06), plus the stake distribution payload (stake_by_pool + total_active_stake, S3c). The canonical_hash (blake2b over the canonical encoding of EVERY binding + the stake distribution, fixed field order, BTreeMap in sorted PoolId order) is the view's self-describing identity. A view is INERT -- it may NOT be activated -- unless ALL bindings match the activation context AND verify_canonical_hash() holds (recompute == stored): matches(&ViewBindings) checks both. The canonical encoding round-trips (canonical_bytes -> the same hash), so a WAL-recorded view replays byte-identically (the replay-equivalence the activation relies on). Pure, total, deterministic. ECA-0b strengthens the view to be LEADERSHIP-COMPLETE: the canonical_hash + canonical encoding now ALSO cover pool_vrf_keyhashes (the effective per-pool VRF) + protocol_params_commitment (the FULL consensus-profile commitment), and matches() additionally requires is_leadership_complete() (equal stake/VRF key sets) + the protocol-params commitment, so an incomplete or wrong-profile view is INERT (DC-EVIEW-12). OBSERVE-ONLY: the rewire into the live boundary authority, the WAL activation variant, and feeding the view to live leader election are the activation slice (DC-EVIEW-08); no live-path change. |
| **Code** | crates/ade_ledger/src/reduced_epoch_view.rs: EpochConsensusView{network_magic,era,epoch,source_point,checkpoint_commitment,nonce,snapshot_phase,stake_by_pool,total_active_stake,canonical_hash} + ViewBindings + bind (computes canonical_hash = blake2b(canonical_bytes(..))) + canonical_bytes (round-trippable) + verify_canonical_hash + matches (requires all bindings + verify). ci/ci_check_eview_view_binding.sh. |
| **Tests** | `bind_is_deterministic_and_self_verifies`; `matches_exact_bindings_and_rejects_mismatch`; `canonical_hash_is_binding_sensitive`; `canonical_bytes_reproduce_the_hash`; `tampered_view_fails_verification`; `leadership_complete_required_for_matches` |
| **CI** | `ci/ci_check_eview_view_binding.sh`; `ci/ci_check_eview_leadership_complete.sh` |

#### `DC-EVIEW-08` — _declared_

| Aspect | Location |
|--------|----------|
| **Source** | docs/clusters/EPOCH-CONSENSUS-VIEW/SLICE-3f-activation.md; docs/clusters/EPOCH-CONSENSUS-VIEW/SLICE-3-scope.md (the activation slice) |
| **Requirement** | Activation -- the live-path consumption of Ade's self-derived next-epoch view. MECHANISM (IMPLEMENTED + AUTOMATIC): the boundary activation is wired into the relay loop and runs AUTOMATICALLY -- no arming flag (ECA-1/DC-EPOCH-13 removed the semantic gate); the only gate is the deterministic activation predicate over canonical durable state. The self-derived leadership view is produced by ECA WINDOW REPLAY (epoch_wire::maybe_activate_first_boundary -> derive_authoritative_candidate -> EpochConsensusView::bind -> ActiveEpochAuthority::promote); forging + header validation read the promoted N+1 view via authority.pool_distr_view() (node_sync::forge_one_from_recovered). This window-replay derivation SUPERSEDES the original S3f-1 ledger-mark seam: apply_epoch_boundary_full still passes None and the precomputed_mark path is exercised only by tests, so the self-derived view does NOT flow through the ledger boundary mark. The S3f-1 consume/stub seam stays a fail-safe (None -> stub UNCHANGED), pinned by epoch_boundary_consumes_precomputed_aggregate_mark. REMAINING PROOF (the gate to enforced -- LIVE, not code): (1) live shadow agreement -- Ade's checkpoint-derived {pool_distribution, total_active_stake, ADE1 sigma} == cardano-cli stake-snapshot / the fresh oracle bundle (reduction already 100% exact + ADE1 exact on real preview epoch 1334; the perfectly boundary-aligned pool match owed at a real boundary); (2) real Preview Conway boundary activation + continuity -- the UNCHANGED production binary auto-activates across the wall, keeps admitting valid N+1, stays forge-ready, no manual intervention / restart / external stake import; (3) leadership-schedule agreement (ADE1's derived schedule == cardano-cli leadership-schedule) + an accepted ADE1 forge on epoch N+1. Fail-closed: an unbound / mismatched / not-yet-k-deep view is INERT; the producer never elects on it. |
| **Code** | crates/ade_ledger/src/rules.rs: apply_epoch_boundary_with_registrations gains precomputed_mark: Option<&StakeByPool> -> new_mark = form_mark_snapshot(agg) when Some, the existing stub when None; apply_epoch_boundary_full passes None (the live path, UNCHANGED). ci/ci_check_eview_activation.sh. |
| **Tests** | `epoch_boundary_consumes_precomputed_aggregate_mark` † |
| **CI** | `ci/ci_check_eview_activation.sh` |

#### `DC-EVIEW-09` — _enforced_

| Aspect | Location |
|--------|----------|
| **Source** | docs/clusters/EPOCH-CONSENSUS-VIEW/SLICE-3f-activation.md; user directive 2026-06-21 (separate manifest-bound authority surfaces) |
| **Requirement** | The manifest-bound bootstrap cert-state import (S3f-2 prerequisite). The seed (SeedEpochConsensusInputs, the compact per-POOL active epoch consensus view) and the cert state (CertState = DelegationState + PoolState, the per-CREDENTIAL ledger continuation state) are DIFFERENT authority surfaces and stay SEPARATELY TYPED -- the closed seed record is NOT widened. They bind ONLY through a canonical BootstrapManifest carrying {network_magic, era, source_point, seed_hash, cert_state_hash, source_commitment}. The cert-state artifact is the COMPLETE canonical CertState produced/consumed by the EXISTING codec (encode_cert_state / decode_cert_state, reused VERBATIM -- never hand-reconstructed loose delegation/reward maps), so it carries the registration/lifecycle facts the codec requires. verify_and_import_cert_state decodes the manifest, requires it match the bootstrap's network + era, requires the seed and cert-state bytes to hash to the manifest's committed hashes, then decodes the (now hash-bound) cert state -- FAIL-CLOSED on a malformed manifest, a seed/cert-state hash mismatch, a network/era mismatch, or a cert state that does not decode. At bootstrap (import_bootstrap_cert_state, discovered by convention next to the seed: <seed>.manifest + <seed>.certstate) both present -> verify + import; exactly one present -> FAIL CLOSED (a seed without its manifest-bound cert state, or a cert state without its binding manifest); neither present -> the pre-import empty CertState (transition). The imported CertState populates BOTH the captured snapshot (build_seed_ledger / seed_to_snapshot) and the runner ledger, BEFORE any state is durable; warm-start reloads it via the snapshot codec. NO live producer behaviour change: leader election still reads the seed's PoolDistrView; this only gives Ade the real delegation/cert state its later SELF-DERIVED epoch views (S3f-2 window driver -> S3c aggregate) require -- without depending on cardano-node at runtime or replaying genesis. |
| **Code** | crates/ade_ledger/src/bootstrap_manifest.rs: BootstrapManifest{network_magic,era,source_point,seed_hash,cert_state_hash,source_commitment} + canonical encode/decode + BootstrapManifestError{MalformedManifest\|SeedHashMismatch\|CertStateHashMismatch\|NetworkMismatch\|EraMismatch\|CertStateDecode} + verify_and_import_cert_state (reuses crate::snapshot::cert_state::decode_cert_state VERBATIM). crates/ade_node/src/admission/seed_to_snapshot.rs: build_seed_ledger / seed_to_snapshot take cert_state -> ledger.cert_state. crates/ade_node/src/admission/bootstrap.rs: import_bootstrap_cert_state (convention-discovered, fail-closed) + AdmissionBootstrapError::BootstrapCertState; populates the snapshot + runner ledgers. ci/ci_check_eview_bootstrap_cert_state.sh. |
| **Tests** | `manifest_round_trips_canonically`; `verify_and_import_happy_path`; `seed_hash_mismatch_fails_closed`; `cert_state_hash_mismatch_fails_closed`; `network_and_era_mismatch_fail_closed`; `malformed_manifest_fails_closed`; `malformed_cert_state_fails_closed_after_hash_ok` |
| **CI** | `ci/ci_check_eview_bootstrap_cert_state.sh` |

#### `DC-EVIEW-10` — _enforced_

| Aspect | Location |
|--------|----------|
| **Source** | docs/clusters/EPOCH-CONSENSUS-VIEW/SLICE-3f-activation.md (S3f-2); docs/clusters/EPOCH-CONSENSUS-VIEW/SLICE-3b-replay-window-materialization.md |
| **Requirement** | The window driver (S3f-2): advance the reduced UTxO checkpoint + the cert/delegation state forward over a window of ordered blocks, then aggregate per-pool stake. drive_window_aggregate sequences the PROVEN pieces in order -- per block: reduced_block_delta (== reduce(track_utxo), DC-EVIEW-04/04b) -> the checkpoint's apply_block_delta, and advance_cert_state (== process_block_certificates) threading cert_state + gov_state exactly as rules.rs does; then once: sum_base_credential_stake -> aggregate_pool_stake (DC-EVIEW-05) over the advanced delegation map. CRITICALLY it starts from the bootstrap LedgerState's cert state (the manifest-bound DC-EVIEW-09 import), NOT an empty map, so PRE-bootstrap delegators are counted -- the verified gap's fix made operative. Fail-closed (WindowDriverError::{Checkpoint\|Ledger\|Aggregate}): any step error aborts the window without producing a partial/wrong stake distribution. RED orchestration of individually-proven deterministic transforms; the per-step correctness is the proven pieces, the driver adds the in-order sequencing. |
| **Code** | crates/ade_runtime/src/chaindb/reduced_window_driver.rs: drive_window_aggregate(checkpoint, bootstrap_state, blocks, era) -> loop { reduced_block_delta -> apply_block_delta; advance_cert_state -> state.cert_state/gov_state } then sum_base_credential_stake -> aggregate_pool_stake; starts from bootstrap_state.clone() (NOT LedgerState::new()); WindowDriverError fail-closed. Exported from chaindb/mod.rs. ci/ci_check_eview_window_driver.sh. |
| **Tests** | `empty_window_aggregates_bootstrap_state`; `real_conway_block_drive_equals_composed_pieces` |
| **CI** | `ci/ci_check_eview_window_driver.sh` |

#### `DC-EVIEW-11` — _enforced_

| Aspect | Location |
|--------|----------|
| **Source** | docs/clusters/EPOCH-CONSENSUS-VIEW/SLICE-3f-activation.md (S3f-3); user directive 2026-06-21 (narrow fail-closed rebind safety slice) |
| **Requirement** | The deterministic, fail-closed epoch-rebind seam (S3f-3), strengthening DC-EPOCH-03. DC-EPOCH-03 fails the forge closed past the seed-epoch boundary (the recovered eta0 is the seed-epoch nonce, stale past it). This seam adds the ONLY sanctioned crossing: the recovered seed-epoch view stays authoritative until, AT the deterministic epoch transition (a candidate slot in the IMMEDIATE next epoch, seed_epoch+1), a fully-bound MATCHING N+1 EpochConsensusView atomically promotes. decide_epoch_rebind (PURE / deterministic) returns KeepCurrent within the seed epoch; Promote(view) only when the slot is the immediate next epoch AND the supplied bindings ARE that epoch's context AND the view matches them (all 7 bindings + verify_canonical_hash, via EpochConsensusView::matches); else FailClosed{Unlocatable\|NotImmediateNext\|NoBoundView\|ViewMismatch}. The live seam in the node-forge wall passes None for the bound view (S3f-4 supplies it), so OffEpoch fails closed EXACTLY as the pre-seam wall -- NO leader-election behaviour change, no early/silent activation, no fallback. It never promotes a wrong-network/era/epoch/point/commitment/nonce/phase or tampered (hash-invalid) view. |
| **Code** | crates/ade_node/src/epoch_rebind.rs: decide_epoch_rebind(admission, bound_n1: Option<(&EpochConsensusView, &ViewBindings)>) -> EpochRebindDecision{KeepCurrent\|Promote\|FailClosed(EpochRebindReject)}; immediate-next-only (seed_epoch.0.wrapping_add(1)) + bindings.epoch==e + view.matches(bindings). crates/ade_node/src/node_sync.rs: the node-forge DC-EPOCH-03 wall calls decide_epoch_rebind(admission, None) -- FailClosed -> ForgeNotLeader (byte-identical), KeepCurrent -> proceed, Promote -> empty no-op (S3f-4). ci/ci_check_eview_epoch_rebind.sh. |
| **Tests** | `simulated_transition_promotes_bound_n1_view`; `same_epoch_keeps_current`; `off_epoch_without_bound_view_fails_closed`; `not_immediate_next_fails_closed`; `unlocatable_fails_closed`; `rejects_each_wrong_binding`; `rejects_tampered_view_wrong_hash`; `replay_equivalent_deterministic`; `crash_restart_redrives_same_decision_both_sides` |
| **CI** | `ci/ci_check_eview_epoch_rebind.sh` |

#### `DC-EVIEW-12` — _enforced_

| Aspect | Location |
|--------|----------|
| **Source** | docs/clusters/EPOCH-CONSENSUS-VIEW/SLICE-ECA-0b-leadership-complete-view.md; user directive 2026-06-21 (freeze the effective VRF + a full consensus-profile commitment; PoolDistrView derives exclusively from the sealed view; cardano numDelegators>0 pool inclusion) |
| **Requirement** | The leadership-complete, self-contained EpochConsensusView (ECA-0b). The candidate view is the production authority for cross-epoch leadership: every INCLUDED pool carries BOTH its active stake AND its era-correct effective VRF keyhash -- pool_vrf_keyhashes.keys() == stake_by_pool.keys() (is_leadership_complete) -- plus a FULL consensus-profile commitment (protocol_params_commitment). derive_candidate builds the pool set by the cardano-faithful intersection delegated (the window-end stake, DC-EVIEW-05) INTERSECT registered (the window-end pool_params, DC-EVIEW-13/DC-EVIEW-10): a delegated-but-unregistered pool is DROPPED (cardano silently drops stake delegated to a pool absent from the snapshot's params); each kept pool's VRF is the window-end pool_params[p].vrf_hash (the mark VRF, ECA-0a); the total is recomputed over the kept set (checked_add, fail-closed Overflow). The protocol_params_commitment = consensus_profile_commitment(genesis_hash, protocol_params_hash, asc) = blake2b(genesis ++ protocol-params ++ ASC) -- the FULL profile (NOT ASC-only; user correction 2026-06-21), computed ONCE from the canonical CandidateProfile and bound; it is folded into the canonical_hash, and matches() requires both is_leadership_complete() AND commitment equality, so an incomplete or wrong-profile view is INERT. derive_candidate performs NO filesystem/config/network read. Pure, total, deterministic (replay-equivalent: an equivalent replay yields a byte-identical candidate canonical_hash). |
| **Code** | crates/ade_ledger/src/reduced_epoch_view.rs: EpochConsensusView gains pool_vrf_keyhashes: BTreeMap<PoolId,Hash32> + protocol_params_commitment: Hash32 (both in canonical_bytes/canonical_hash + ViewBindings); is_leadership_complete (equal key sets); consensus_profile_commitment(genesis_hash, protocol_params_hash, asc) = blake2b(genesis.0 ++ protocol_params_hash.0 ++ asc.numer ++ asc.denom); matches requires is_leadership_complete() + protocol_params_commitment ==. crates/ade_node/src/epoch_candidate.rs: CandidateProfile{slots_per_epoch,genesis_hash,protocol_params_hash,asc}; derive_candidate uses drive_window_consensus_inputs + the delegated INTERSECT registered intersection (inputs.pool_params.get) + computes the commitment + binds. ci/ci_check_eview_leadership_complete.sh. |
| **Tests** | `leadership_complete_required_for_matches`; `canonical_hash_is_binding_sensitive`; `derive_candidate_binds_target_epoch_and_round_trips_through_recovery`; `derive_candidate_canonical_hash_is_replay_equivalent` |
| **CI** | `ci/ci_check_eview_leadership_complete.sh` |

#### `DC-EVIEW-13` — _enforced_

| Aspect | Location |
|--------|----------|
| **Source** | docs/clusters/EPOCH-CONSENSUS-VIEW/SLICE-ECA-0a-pool-lifecycle-fidelity.md; user directive 2026-06-21 (correctness-first, no narrow shortcut; delegation-clearing NOT deferred) |
| **Requirement** | Cardano-faithful pool lifecycle in the reduced window (ECA-0a). The cert-state pool lifecycle matches cardano-ledger (Pool.hs/PoolReap.hs/Epoch.hs/SnapShots.hs @ 226b002d) so a windowed replay reproduces the mark snapshot's pool set + VRF keys byte-faithfully: (1) PoolState gains future_pools (psFutureStakePoolParams); apply_pool_registration STAGES a re-registration of an already-registered pool into future_pools -- the active pools entry AND its VRF are UNCHANGED until adoption (a first registration still inserts into pools immediately) -- and cancels a pending retirement. (2) apply_pool_reap (POOLREAP, over the whole CertState) adopts future_pools into pools (dropping an orphan future with no active pool, Map.dropMissing), reaps pools with retiring == entered_epoch, CLEARS the delegations targeting reaped pools (removeStakePoolDelegations -- a credential delegated to a reaped pool is un-delegated so it cannot silently reattach if that pool id re-registers later; the credential's registration + reward account are preserved), and removes the reaped pools from pools + retiring. (3) drive_window_consensus_inputs applies apply_pool_reap at EACH epoch boundary crossed within the replayed block range (slot/slots_per_epoch) and surfaces the window-end {stake, pool_params} -- the MARK, captured BEFORE any further reap (SNAP precedes POOLREAP, Epoch.hs:292-297). Pure + deterministic (replay-equivalent). A re-registration's new VRF governs leadership one epoch later, never the current mark. |
| **Code** | crates/ade_ledger/src/delegation.rs: PoolState.future_pools; apply_pool_registration (existing pool -> future_pools + retiring.remove; new -> pools); apply_pool_reap(cert: &mut CertState, entered_epoch) (adopt-drop-orphan; reap e.0==entered_epoch.0; delegations.retain drops reaped-pool targets; remove from pools+retiring). crates/ade_runtime/src/chaindb/reduced_window_driver.rs: drive_window_consensus_inputs(.., slots_per_epoch) -> WindowConsensusInputs{stake, pool_params}, applies apply_pool_reap at crossed boundaries, mark pre-reap; drive_window_aggregate = per-block wrapper (slots_per_epoch=u64::MAX). crates/ade_ledger/src/snapshot/cert_state.rs: 6-field codec round-trips future_pools. ci/ci_check_eview_pool_lifecycle.sh. |
| **Tests** | `re_registration_keeps_old_vrf_until_reap`; `pool_re_registration_stages_params_adopted_at_reap`; `reaped_pool_delegation_cleared_no_silent_reattach_on_reregistration`; `pool_reap_reaps_matching_epoch_only`; `drive_boundary_adopts_futures_reaps_retiring_clears_delegations`; `drive_boundary_is_deterministic`; `cert_state_round_trip_populated` |
| **CI** | `ci/ci_check_eview_pool_lifecycle.sh` |

### DC-FOLLOW-FORGE

#### `DC-FOLLOW-FORGE-01` — _enforced_

| Aspect | Location |
|--------|----------|
| **Source** | docs/clusters/PRODUCER-PARTICIPANT-FOLLOW/CN-FOLLOW-01-participant-forge-on-ao-selected-head.md (§6 DC-FOLLOW-FORGE-01, §8 changes, §10 MAC) |
| **Requirement** | Participant forge-decision mechanics. The keyed Participant venue uses an initial-catch-up -> extend forge mode mirroring the single-producer two-state mode: participant_forge_decision returns UseInitialCatchupGate (the existing DC-NODE-15 gate) until the FIRST caught-up instant (participant_forge_mode_on_caughtup latches ForgeMode::ParticipantExtendOnSelectedHead on the durable servable head -- exact-equality-once then latch, NOT a frontier-proximity re-test), then ExtendOnSelectedHead { forge_base = the LIVE AO-selected durable servable ChainDb::tip read at the decision boundary } iff venue == Participant AND no fork-choice decision is pending (DC-NODE-28 MANDATORY: pending_reselection / pending_fork_switch / pending_missing_bridge ALL fence, yielding a typed ForkChoicePending refusal) AND the durable servable tip is present; otherwise a typed ParticipantFenceViolation (VenueNotDeclaredParticipant / ForkChoicePending / NoDurableServableTip). The decision does NOT gate on the extend mode's latched current_tip: that latch is a DERIVED OBSERVATION (advanced only on Ade's OWN forge+admit, never the forge authority). Gating the decision on a current_tip byte-equality re-check deadlocks the forge -- the durable head also advances on every FOLLOWED peer admit, so the latch goes stale the instant a peer block is admitted and the decision refuses forever (the live proof-#1 finding, preview epoch 1333: 539 extend ticks, only 37 leader checks before the first peer admit, then 502 no_tip). Because the forge base is now derived from the live durable tip, a sign-time base-consistency re-check (participant_sign_time_base_consistent) runs in the RED ForgeTick immediately before signing/admit: re-read ChainDb::tip and refuse deterministically (ForgeRefused::ParticipantForgeBaseChangedBeforeSign) if a participant admit / fork-selection advanced the durable head between the decision and the sign -- so dropping the decision-time equality check does not become a stale-SIGNING bug; the next ForgeTick re-evaluates from the new tip. The decision is PURE / TOTAL / deterministic GREEN -- no HashMap / wall-clock / Instant / rand / float, closed typed enums only (no String / anyhow in the result), no observed-peer competing fence (the AO resolves competitors), and it NEVER reaches select_best_chain / chain_selector / fork_choice and carries NO KES/VRF signing material (signing stays RED). The forged block is durably admitted via the unchanged pump_block (DC-NODE-05); the in-memory ForgeMode is replay-derived on restart (re-catches-up, re-transitions), so replaying the same admitted chain + leader schedule yields byte-identical decisions and forged blocks. |
| **Code** | crates/ade_node/src/node_sync.rs: ParticipantForgeDecision{UseInitialCatchupGate\|ExtendOnSelectedHead{forge_base}\|Refuse(ForgeRefused)} + ParticipantForgeFenceReason{VenueNotDeclaredParticipant\|ForkChoicePending\|NoDurableServableTip} (DurableTipDivergedFromExtendHead REMOVED -- the decision no longer gates on the latch) + ForgeRefused::ParticipantFenceViolation + ForgeRefused::ParticipantForgeBaseChangedBeforeSign{decision_base,sign_time_tip} + ForgeMode::ParticipantExtendOnSelectedHead{adopted_root,current_tip} (both DERIVED OBSERVATIONS, never the forge authority) + participant_forge_decision (derives forge_base from durable_servable_tip, not current_tip) + participant_sign_time_base_consistent + participant_forge_mode_on_caughtup + participant_forge_mode_after_admit; single_producer_forge_decision gains only a fail-closed arm for the never-reached participant mode (existing behaviour unchanged). crates/ade_node/src/node_lifecycle.rs: the ForgeTick else branch routes VenueRole::Participant to participant_forge_decision (initial gate latches via participant_forge_mode_on_caughtup; the post-admit advance uses participant_forge_mode_after_admit), captures the decision's forge_base into participant_forge_base, and runs participant_sign_time_base_consistent (re-read ChainDbServedSource::new(chaindb).tip()) as a sign_time_ok guard immediately before forge_one_from_recovered; the forge-base evidence emits ForgeBaseSource::LocalChaindbTip + cert_path_present:false for Participant. crates/ade_node/src/live_log/sched_event.rs: ForgeModeKind::ParticipantExtendOnSelectedHead diagnostic discriminator. ci/ci_check_participant_forge_on_selected_head.sh. |
| **Tests** | `participant_venue_forges_on_ao_selected_head_when_leader`; `participant_forge_base_is_ao_selected_chaindb_tip`; `participant_forge_base_is_servable_before_forge`; `participant_forge_refused_while_fork_choice_pending`; `participant_venue_requires_forge_activation`; `orphaned_startup_holds_forge_fence_participant`; `participant_forge_two_runs_byte_identical`; `single_producer_forge_decision_unchanged`; `keyed_participant_extend_survives_peer_admit_and_reaches_leader_check`; `participant_forge_refuses_if_tip_changes_between_decision_and_sign` |
| **CI** | `ci/ci_check_participant_forge_on_selected_head.sh` |

### DC-FORGE

#### `DC-FORGE-01` — _enforced_

| Aspect | Location |
|--------|----------|
| **Source** | docs/planning/phase4-n-r-invariants.md §3 (D1); §4 (R3) |
| **Requirement** | Given the same canonical input set (slot, eta0, vrf_vk, vrf_proof_or_output, LeaderScheduleAnswer), verify_and_evaluate_leader produces a byte-identical LeaderCheckVerdict across runs. Replay-equivalence anchor for the BLUE leader-check evaluator. Strengthens the existing leader-check determinism (DC-CONS-13 family) by exposing leader eligibility as a callable, replay-anchored function — not just an internal step in forge_block. |
| **Code** | crates/ade_core/src/consensus/leader_check.rs (verdict_is_byte_identical_across_two_runs unit test); crates/ade_node/tests/forge_handler_variants.rs (run_real_forge_is_byte_identical_across_two_runs end-to-end pipeline anchor); crates/ade_node/src/node_sync.rs (forge_from_recovered_is_deterministic_across_two_runs — leader-check determinism over the recovered-state forge path) |
| **Tests** | `verdict_is_byte_identical_across_two_runs`; `run_real_forge_is_byte_identical_across_two_runs`; `forge_from_recovered_is_deterministic_across_two_runs` |
| **CI** | _(no CI script listed)_ |

### DC-GENESIS-SRC

#### `DC-GENESIS-SRC-01` — _enforced_

| Aspect | Location |
|--------|----------|
| **Source** | docs/clusters/PHASE4-N-Y/S4-conway-genesis-source.md |
| **Requirement** | A controlled genesis enters initial state ONLY through the single closed bootstrap_initial_state authority (genesis_initial); the genesis->initial-state transform is a pure deterministic BLUE function; a non-Conway genesis fails closed (GenesisSourceError::NonConwayEra) in this cluster — no Byron->Conway historical replay path is invoked. No GenesisAnchor/MithrilAnchor trait or plugin seam. |
| **Code** | crates/ade_ledger/src/genesis_source.rs (genesis_initial_state + closed GenesisSourceError); crates/ade_runtime/src/genesis_bootstrap.rs |
| **Tests** | `conway_genesis_bootstrap_through_single_authority`; `genesis_non_conway_fail_closed`; `genesis_to_initial_state_deterministic`; `genesis_path_fp_equals_snapshot_path_fp` |
| **CI** | `ci/ci_check_mithril_uses_bootstrap_initial_state.sh` |

### DC-GENESIS

#### `DC-GENESIS-01` — _enforced_

| Aspect | Location |
|--------|----------|
| **Source** | docs/planning/phase4-n-r-invariants.md §3 (D5) |
| **Requirement** | Given the same canonical Shelley genesis JSON bytes + the same operator-supplied kes_anchor_slot, parse_shelley_genesis produces a byte-identical GenesisAnchor across runs. Replay-equivalence anchor for the genesis closed-contract parser. The ISO 8601 → Unix epoch milliseconds conversion (parse_iso8601_to_unix_ms) is deterministic without any chrono/time crate dependency. |
| **Code** | crates/ade_runtime/src/producer/genesis_parser.rs (parser_is_byte_identical_across_two_runs unit test) |
| **Tests** | `parser_is_byte_identical_across_two_runs` |
| **CI** | _(no CI script listed)_ |

### DC-GOV

#### `DC-GOV-01` — _declared_

| Aspect | Location |
|--------|----------|
| **Source** | docs/clusters/CONWAY-PROPOSAL-DEPOSIT-EXPIRY/cluster.md (sections 2-4, declared by this cluster); CIP-1694 (proposal lifecycle: deposit -> expiry -> refund to return_addr from the deposit pot, distinct from a treasury withdrawal). Ground-truthed 2026-06-30 against a POST-1340 cardano db-analyser extraction: the CE-3d -500B reward differential is dropped expired-proposal deposit refunds (5 TreasuryWithdrawals proposals, deposit 100k ADA each, proposed_in 1309 / expires_after 1339, refunded at the 1340->1341 boundary). |
| **Requirement** | GOVERNANCE-DEPOSIT-EXPIRY-REFUND (negative proof). Ade refunds a removed governance proposal's deposit to its recorded return address ONLY when it can PROVE, from canonical governance state and the Conway rules, that the proposal could NOT have ratified or enacted; otherwise it fails closed (terminal structured failure). The refund (deposit-pot debit + return-address credit) is a total, deterministic, replay- equivalent boundary transition; the proof is canonical, persisted-or-reproducibly-derivable, and testable. Ade never decides a proposal RATIFIES, only proves when one CANNOT. The tracked proposal set is canonical at every boundary: imported from the certified snapshot (bootstrap), kept current by capturing live proposal_procedures (tx-body field 20) during follow, and PROTECTED by a vote tripwire -- any canonical selected-chain vote (field 19) targeting a tracked proposal makes its tracked vote map non-canonical and is terminal (Ade does not tally/ratify/enact). Every persisted field that decides a future refund is canonical, never defaulted: expires_after = proposed_in + govActionLifetime, and govActionLifetime is IMPORTED from the certified curPParams (never a placeholder) -- a 0/un-imported lifetime at capture is terminal, never a fabricated expiry. No silent skip / empty default: an unknown GovActionState / GovAction variant, an unsupported committee representation, or a malformed field 19/20 is terminal; an ABSENT proposal/committee set is never reinterpreted as an EMPTY one (a pre-import store fails closed -> re-bootstrap required). |
| **Code** | S1 (bootstrap import, d2522faf): ade_ledger/ledgerdb_state.rs (decode_native_nonutxo_state -> nn_read_proposals / nn_read_committee / nn_read_gov_action_id; no silent skip -> Unsupported/MalformedGovernanceState; ImportedGovState carries proposals + committee + committee_quorum; the v5->v6 commitment binds them). S2 (absent != empty load gate, 9855ad56): ade_runtime/chaindb/epoch_accumulator_store.rs (verify_governance_imported -> GovernanceImportRequired) + ade_node node_lifecycle warm-start (AccumulatorPredatesGovernanceImport). S3 (capture + tripwire + expiry-lifetime authority): ade_ledger/epoch_accumulator.rs (apply_block_governance / apply_one_tx_governance -- the dedicated within-epoch governance walk wired into apply_within_epoch, gated by the phase-2-invalid set; field-20 -> GovActionState{action_id=(transaction_id(body),index), proposed_in=block_epoch, expires_after=proposed_in+gov_action_lifetime, empty votes}; extract_voted_action_ids -- the field-19 vote tripwire; terminals VoteOnTrackedProposal / MalformedGovernanceField / GovActionLifetimeUnproven, and InvalidTxCarriesAuthorityEffect now reachable for field 19/20) + ade_ledger/ledgerdb_state.rs (read_conway_pparams captures govActionLifetime at curPParams index CONWAY_PP_GOV_ACTION_LIFETIME_INDEX into ImportedGovState.gov_action_lifetime; the v6->v7 commitment binds it as fresh-bootstrap tamper-evidence) + ade_runtime/mithril_native_assembly.rs (seeds gov_state.gov_action_lifetime from the import, never a hardcoded 0). |
| **Tests** | `ade_ledger::epoch_accumulator::tests (s3_live_proposal_captured_with_identity_epoch_and_expiry, s3_two_proposals_one_tx_get_sequential_indices_same_txid, s3_vote_on_tracked_proposal_is_terminal, s3_vote_on_untracked_proposal_is_carried_forward, s3_cross_tx_same_block_vote_on_just_submitted_proposal_is_terminal, s3_invalid_tx_carrying_proposal_is_fail_closed, s3_invalid_tx_carrying_vote_is_fail_closed, s3_malformed_field20_is_fail_closed, s3_malformed_field19_is_fail_closed, s3_block_without_governance_fields_is_noop, s3_capture_skips_non_gov_fields_and_is_replay_equivalent, s3_unproven_zero_lifetime_refuses_to_fabricate_expiry, s3_gov_state_none_is_untracked_and_skipped)`; `ade_ledger::ledgerdb_state::tip_tests::v6_commitment_is_deterministic_and_binds_gov (v7 binds the imported gov_action_lifetime)`; `ade_ledger tests/ledgerdb_nonutxo_hermetic.rs::happy_minimal_state_decodes_all_fields (imported_gov.gov_action_lifetime == 6 read from curPParams idx 26)`; `ade_runtime::mithril_native_assembly::tests::native_assembly_maps_each_field_from_its_source (gov_state.gov_action_lifetime seeded from the import, not 0)`; `ade_runtime::chaindb::epoch_accumulator_store::tests::governance_import_gate_rejects_absent_but_allows_empty (S2 absent != empty)` |
| **CI** | `ci/ci_check_gov_proposal_capture.sh` |

### DC-INGRESS

#### `DC-INGRESS-01` — _declared_

| Aspect | Location |
|--------|----------|
| **Source** | Project constitution §3, T-INGRESS-01 |
| **Requirement** | Block/tx/protocol message decoding enters core through named chokepoints; no raw-byte bypass without CI-whitelisted justification |
| **Code** | — _(declared — not yet enforced)_ |
| **Tests** | — _(declared — not yet enforced)_ |
| **CI** | `ci/ci_check_ingress_chokepoints.sh` |

#### `DC-INGRESS-02` — _declared_

| Aspect | Location |
|--------|----------|
| **Source** | Project constitution §3, T-INGRESS-01, T-ENC-01 |
| **Requirement** | Storage rehydration enters core through the same canonical decode chokepoints as network ingress |
| **Code** | — _(declared — not yet enforced)_ |
| **Tests** | — _(declared — not yet enforced)_ |
| **CI** | — _(declared — not yet enforced)_ |

### DC-KES-HEADER

#### `DC-KES-HEADER-01` — _enforced_

| Aspect | Location |
|--------|----------|
| **Source** | docs/planning/phase4-n-s-invariants.md §3 (D1) |
| **Requirement** | unsigned_header_pre_image(slot, block_no, prev_hash, vrf_data, opcert, kes_period, hot_vkey, body_hash, body_size, protocol_version) is a pure BLUE function. Same canonical inputs → byte-identical UnsignedHeaderPreImage output. Replay-equivalence anchor for the pre-image recipe. |
| **Code** | crates/ade_ledger/src/block_validity/unsigned_header_pre_image.rs (recipe_output_is_byte_identical_across_two_runs unit test) |
| **Tests** | `recipe_output_is_byte_identical_across_two_runs` |
| **CI** | _(no CI script listed)_ |

### DC-LEDGER-PARAMS

#### `DC-LEDGER-PARAMS-01` — _enforced_

| Aspect | Location |
|--------|----------|
| **Source** | docs/clusters/MITHRIL-VERIFIED-ANCHOR-INTEGRATION/SLICE-S1a-native-nonutxo-decoder.md; user directive 2026-06-23 (two S1a refinements, neither deferrable behind a flag: (1) bind network_id from the manifest network magic -- mainnet -> 1, testnet -> 0 -- onto every authority-bearing field (the operator-supplied reward-account nibble is diagnostic evidence only, never a verdict -- the manifest magic is the sole network authority); (2) preserve Conway coinsPerUTxOByte faithfully as MinUtxoRule::PerByte, NEVER remapped into an absolute min_utxo_value, with the per-byte min-UTxO validation a structured TERMINAL UnsupportedConwayMinUtxoRule rather than a permissive absolute floor) |
| **Requirement** | Imported protocol parameters are preserved era-faithfully and are NEVER semantically remapped across eras. The shared `ProtocolParameters` carries the minimum-UTxO rule as an era-aware sum type `MinUtxoRule = LegacyAbsoluteMin(Coin) \| PerByte(Coin)` (NOT a single `Coin` field): Shelley/Mary `minUTxOValue` is an ABSOLUTE per-output floor (`output.coin >= c`); Conway `coinsPerUTxOByte` is a PER-BYTE coefficient (the minimum is `c * serialized-output-size`, NOT an absolute floor). The native non-UTxO snapshot decoder (`read_conway_pparams`) decodes Conway `coinsPerUTxOByte` into `MinUtxoRule::PerByte(Coin(c))` and MUST NOT populate `LegacyAbsoluteMin` from it. The authoritative min-UTxO VALIDATION matches on the rule: `LegacyAbsoluteMin(c)` runs the existing absolute check; `PerByte(_)` is a structured TERMINAL `LedgerError::UnsupportedConwayMinUtxoRule` (fail closed rather than accept outputs under a false minimum -- the per-byte coefficient is NEVER used as an absolute floor). The canonical pparams encoders (`encode_pparams`, `fingerprint_pparams`) serialize the rule's coin payload only, BYTE-IDENTICAL to the prior single-`Coin` field for legacy `LegacyAbsoluteMin` states (so the pinned non-Conway pparams fingerprints + all differential replay suites are unchanged); the rule KIND is bound separately in the S1a native commitment (bumped to v2). DERIVED bind: the native decoder's internal `network_id` is DERIVED from the manifest network magic (mainnet 764824073 -> 1; any other (testnet) magic -> 0) and bound onto the emitted state, `protocol_params.network_id`, and the commitment, so ONE manifest binds every authority-bearing field. The pool reward-account network nibble is OPERATOR-controlled ledger data (demonstrably MIXED on real preprod) and is NOT a network discriminator: it is recorded as a DIAGNOSTIC `RewardNibbleObservation` (Uniform/Mixed/None) in the canonical report and NEVER accepts or rejects the snapshot -- a heuristic on operator metadata, unanimous or otherwise, is not an authority check; the manifest magic is the SOLE network authority. RELEASE BLOCKER -- a DERIVED compatibility PREREQUISITE for native Conway block validation/follow, NOT merely a full-compatibility enhancement: Ade's Conway min-UTxO validation must compute the era-correct per-byte minimum before a native-bootstrapped Conway state (which carries `PerByte`) can be validated/followed; until then the per-byte path refuses deterministically (`UnsupportedConwayMinUtxoRule`). SCOPE: the min-UTxO rule preservation + the per-byte validation terminal + the network-id derive/bind + the diagnostic nibble observation; the era-correct per-byte minimum COMPUTATION is future work (the terminal is the fail-closed placeholder for it). |
| **Code** | crates/ade_ledger/src/pparams.rs: MinUtxoRule (LegacyAbsoluteMin/PerByte + coin()) + ProtocolParameters.min_utxo_rule (replaces min_utxo_value) + Default/apply_update (LegacyAbsoluteMin). crates/ade_ledger/src/shelley.rs + mary.rs: the min-UTxO check matches MinUtxoRule -- LegacyAbsoluteMin keeps the absolute check, PerByte -> UnsupportedConwayMinUtxoRule. crates/ade_ledger/src/error.rs: LedgerError::UnsupportedConwayMinUtxoRule + UnsupportedConwayMinUtxoRuleError + Display arm. crates/ade_ledger/src/phase.rs: classify_failure_phase routes UnsupportedConwayMinUtxoRule to Phase1. crates/ade_ledger/src/ledgerdb_state.rs: network_id_from_magic + decode_native_nonutxo_state(manifest_network_magic) + read_conway_pparams(network_id) -> MinUtxoRule::PerByte + NativeSnapshotNonUtxoState.network_id + RewardNibbleObservation diagnostic (observe_reward_account_nibbles, never a verdict) + commit_native_nonutxo_state v2 (binds network_id + rule kind + the nibble observation). crates/ade_ledger/src/snapshot/gov_state.rs + fingerprint.rs: encoders serialize min_utxo_rule.coin() (byte-identical for legacy). ci/ci_check_native_nonutxo_decode.sh. |
| **Tests** | `network_id_derived_from_manifest_magic`; `reward_nibble_disagreement_is_diagnostic_not_terminal`; `conway_pparams_decode_yields_per_byte_min_utxo_rule`; `mary_min_utxo_per_byte_rule_is_terminal_not_permissive`; `mary_min_utxo_legacy_absolute_min_unchanged`; `commitment_binds_every_field`; `decode_native_nonutxo_real_snapshot` |
| **CI** | `ci/ci_check_native_nonutxo_decode.sh` |

### DC-LEDGER-VALUE

#### `DC-LEDGER-VALUE-01` — _enforced_

| Aspect | Location |
|--------|----------|
| **Source** | docs/clusters/LEDGER-VALUE-CORRECTNESS/SLICE-1-output-asset-quantity-u64.md; the DC-MITHRIL-05 downstream release-blocker (Ade's i64 MultiAsset model could not safely validate real Cardano outputs with quantities > i64::MAX); user directive 2026-06-23 (the authoritative value model is widened to the non-negative Word64 domain via a distinct OutputAssetQuantity(u64) newtype -- NOT a universal i128, NOT a u64->checked-i64->reject adapter, NOT a truncating cast; checked output arithmetic with a structured underflow error; the canonical prune_zeros normalization stays; mint/burn stays the distinct signed MintBurnQuantity(i64) and cannot enter outputs; representable values stay byte-identical so existing replay/snapshot/codec tests are unchanged) |
| **Requirement** | Ade's authoritative UTxO OUTPUT asset quantity preserves the full non-negative Cardano Word64 domain (0 ..= 2^64-1) via the `OutputAssetQuantity(u64)` newtype. BOTH `MultiAsset` definitions (the codec-layer `ade_types::mary::value::MultiAsset` and the ledger-layer `ade_ledger::value::MultiAsset`) hold `BTreeMap<Hash28, BTreeMap<AssetName, OutputAssetQuantity>>`; `ade_ledger` REUSES the `ade_types` newtype (it does not define its own). A negative output quantity is UNREPRESENTABLE by type. Output arithmetic is CHECKED: `multi_asset_add` uses `checked_add` (overflow -> a structured `LedgerError`), `multi_asset_sub`/`value_sub` use `checked_sub` and an output underflow (subtrahend qty > minuend qty) is the structured authoritative `LedgerError::AssetUnderflow { policy, name }` -- it NEVER wraps and NEVER produces or deletes a negative entry. The canonical output encoding is a CBOR unsigned integer (u64) on every authoritative encode path (the snapshot `write_multi_asset` and the fingerprint `write_multi_asset`), so a quantity > i64::MAX round-trips faithfully, and -- the BYTE-IDENTITY guarantee -- a representable quantity (<= i64::MAX) encodes to the SAME bytes as the prior signed form (a non-negative CBOR int and a u64 <= i64::MAX are both a CBOR major-0 uint). The snapshot decoder reads the output quantity via a dedicated non-negative reader (`read_output_quantity`); a negative CBOR integer in an output position is a structured terminal `StructuralReason::NegativeAssetQuantity`, never coerced. Mint/burn is the DISTINCT signed `MintBurnQuantity(i64)` (DORMANT until S-13 mint decoding): it is never used as a `MultiAsset` map value type and therefore cannot enter an output bundle. This widens the authoritative value model so the faithful u64 quantities the Stage-2 MemPack decoder already produces (DC-MITHRIL-05) can be promoted into `UTxOState` and persisted without loss -- the downstream release-blocker DC-MITHRIL-05 named is exactly this slice's subject. SCOPE: OUTPUT domain only; mint decoding + signed conservation remain future work (S-13). |
| **Code** | crates/ade_types/src/mary/value.rs: OutputAssetQuantity(u64) (ZERO/checked_add/checked_sub/is_zero) + the dormant MintBurnQuantity(i64) + MultiAsset over OutputAssetQuantity. crates/ade_ledger/src/value.rs: MultiAsset (imports the ade_types newtype), multi_asset_add (checked_add), multi_asset_sub/value_sub (checked_sub -> AssetUnderflow), prune_zeros (canonical zero normalization). crates/ade_ledger/src/error.rs: LedgerError::AssetUnderflow + AssetUnderflowError. crates/ade_ledger/src/phase.rs: classify_failure_phase routes AssetUnderflow to Phase1. crates/ade_ledger/src/mary.rs: the type-impossible negative-output scan removed; parse_mint_field documents the MintBurnQuantity boundary. crates/ade_ledger/src/snapshot/utxo_state.rs: write_multi_asset (CBOR uint qty.0) + read_output_quantity (non-negative; NegativeAssetQuantity terminal). crates/ade_ledger/src/snapshot/error.rs: StructuralReason::NegativeAssetQuantity. crates/ade_ledger/src/fingerprint.rs: write_multi_asset (CBOR uint qty.0). ci/ci_check_value_quantity_domain.sh. |
| **Tests** | `multi_asset_word64_add_sub_round_trips_above_i64_max`; `multi_asset_sub_underflow_returns_asset_underflow`; `multi_asset_add_overflow_returns_error`; `negative_output_quantity_is_unrepresentable`; `utxo_state_word64_multi_asset_quantity_round_trips`; `utxo_state_negative_output_quantity_is_rejected`; `representable_quantity_encodes_byte_identical_golden`; `stage2_mempack_word64_output_survives_snapshot_recovery` |
| **CI** | `ci/ci_check_value_quantity_domain.sh` |

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
| **CI** | `ci/ci_check_differential_divergence.sh`; `ci/ci_check_plutus_budget_cap.sh`; `ci/ci_check_plutus_oracle_no_false_accept.sh` |

#### `DC-LEDGER-04` — _partial_

| Aspect | Location |
|--------|----------|
| **Source** | Project constitution §3, T-DET-01 |
| **Requirement** | Epoch boundary computations (stake snapshots, rewards) match Haskell |
| **Code** | crates/ade_ledger/src/epoch.rs, crates/ade_ledger/src/rules.rs |
| **Tests** | `precise_boundary_comparison_eta_diagnosis`; `alonzo_epoch_boundary_end_to_end`; `regular_epoch_boundary_comparison`; `conway_epoch_boundary_end_to_end` |
| **CI** | _(no CI script listed)_ |

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
| **Code** | — _(declared — not yet enforced)_ |
| **Tests** | — _(declared — not yet enforced)_ |
| **CI** | — _(declared — not yet enforced)_ |

#### `DC-LEDGER-07` — _declared_

| Aspect | Location |
|--------|----------|
| **Source** | Project constitution §3, T-DET-01, T-BUILD-02 |
| **Requirement** | Coexisting supported versions must return same validity verdict for consensus-relevant inputs |
| **Code** | — _(declared — not yet enforced)_ |
| **Tests** | — _(declared — not yet enforced)_ |
| **CI** | — _(declared — not yet enforced)_ |

#### `DC-LEDGER-08` — _enforced_

| Aspect | Location |
|--------|----------|
| **Source** | Cardano ledger spec (Conway CDDL certificate tags 0..18; delegation/pool state transitions); IDD fail-fast + closed-surface doctrine |
| **Requirement** | Conway cert-state accumulation is a closed, total, era-versioned transition: for each block at track_utxo, certificates decode through the era-correct closed grammar (Conway via the completed single decode_conway_certs retaining all owner payloads, tags 0..18) selected by explicit era dispatch — never the Shelley 6-variant decoder on Conway bytes, never reduced into the 7-variant Shelley Certificate, never with payload fields dropped. Every certificate resolves to an owner-tagged disposition: it mutates B4-owned CertState (delegation/pool), or it is owner-tagged to ConwayGovState and routed out-of-mutation-scope (observed, not swallowed, not applied), or it is a structured reject (NotValidInEra for removed tags 5/6, Malformed for bad CBOR, UnsupportedUntilStateOwner only for genuinely-ownerless cases — unreachable on the real corpus). Composite certs (tags 10/12/13) carry both a B4-owned mutation and an owner-tagged governance effect; both are represented. No certificate is flattened to neutral because there is nowhere to put it (the owner exists), decode-dropped, or apply-swallowed; a decode or apply error propagates as a structured LedgerError and halts the block transition. Incomplete or best-effort accumulation is a forbidden fail-open. (Wiring the owner-tagged ConwayGovState effects into applied governance state is PHASE4-B5, not B4.) |
| **Code** | crates/ade_types/src/conway/cert.rs (owner-complete ConwayCert); crates/ade_types/src/shelley/cert.rs (PoolRegistrationCert.owners); crates/ade_codec/src/conway/cert.rs (decode_conway_certs payload retention + decode_drep); crates/ade_codec/src/shelley/cert.rs (shared read_pool_registration_cert) — PHASE4-B4-S1 decoder-completeness clause; crates/ade_ledger/src/delegation.rs (apply_conway_cert + ConwayCertAction/ConwayCertOutcome owner-tagged apply model, total over 18 tags) — PHASE4-B4-S2 apply-totality clause |
| **Tests** | `each_tag_retains_owner_payloads`; `drep_grammar_total`; `conway_cert_action_total`; `apply_outcome_agrees_with_action`; `removed_tag_rejects_as_era_invalid`; `drep_registration_is_owner_tagged_not_applied`; `era_dispatch_conway_accumulates_via_conway_path`; `era_dispatch_shelley_accumulates_via_shelley_path`; `conway_decode_error_is_fail_closed`; `conway_unknown_tag_is_fail_closed` … (+6 more) |
| **CI** | `ci/ci_check_forbidden_patterns.sh` |

#### `DC-LEDGER-09` — _enforced_

| Aspect | Location |
|--------|----------|
| **Source** | Cardano ledger spec (Conway CDDL gov cert tags 9..18; CONWAY CERTS/GOVCERT/COMMITTEE transitions; CIP-1694); IDD fail-fast + closed-surface doctrine |
| **Requirement** | Conway governance-certificate accumulation is a closed, total, era-versioned transition into ConwayGovState: every governance-affecting Conway cert that B4 owner-tagged to ConwayGovState (vote-delegation tags 9/10/12/13 -> vote_delegations; committee tags 14/15 -> committee_hot_keys; DRep tags 16/17/18 -> drep_expiry) resolves to exactly one explicit ConwayGovState mutation or a structured reject -- never observed-and-dropped, never flattened, never with payload fields lost. B5 mutates only governance-owned fields; it does not touch B4-owned CertState and does not double-apply the delegation/pool half of composite certs (10/12/13). DRep expiry is computed only from an explicit gov-env (current epoch + drepActivity); a missing required env input is a structured fail-fast, never a defaulted expiry. A cert that cannot be applied propagates a structured LedgerError and halts the block transition; incomplete or best-effort accumulation is a forbidden fail-open. ConwayGovState becomes a deterministic function of (boundary-loaded base then replayed block-stream cert effects) rather than a frozen snapshot -- a deliberate, oracle-confirmable fingerprint migration (T-DET-01). |
| **Code** | crates/ade_ledger/src/gov_cert.rs (apply_conway_gov_cert: native gov dispatch over ConwayCert, total over 18 tags) — PHASE4-B5-S2; crates/ade_ledger/src/state.rs (GovCertEnv + LedgerState::gov_cert_env() fail-fast) + crates/ade_ledger/src/pparams.rs (ConwayOnlyDepositParams.drep_activity) + crates/ade_ledger/src/error.rs (ValidationEnvironmentError::MissingDRepActivityParam) — PHASE4-B5-S1; crates/ade_ledger/src/rules.rs (accumulate_tx_certs / process_block_certificates thread Option<ConwayGovState>, apply the gov half, carry gov_state forward through apply_block — replaces the B4 observe-and-drop) — PHASE4-B5-S3; crates/ade_ledger/src/fingerprint.rs (gov-state + drep_activity fingerprint surface) |
| **Tests** | `gov_apply_total_over_18_tags`; `composite_gov_half_applied_once_certstate_untouched_by_b5`; `drep_expiry_uses_epoch_plus_activity`; `env_free_gov_certs_need_no_env`; `drep_register_missing_env_is_fail_fast`; `drep_expiry_overflow_is_fail_closed`; `gov_apply_is_deterministic`; `gov_cert_env_present_ok`; `gov_cert_env_missing_drep_activity_is_fail_fast`; `gov_accumulation_applies_drep_registration_into_gov_state` … (+7 more) |
| **CI** | `ci/ci_check_gov_cert_accumulation_closed.sh` |

#### `DC-LEDGER-10` — _enforced_

| Aspect | Location |
|--------|----------|
| **Source** | Cardano ledger spec (Credential = key\|script across UTXOW/DELEG/GOVCERT; CIP-1694); IDD illegal-states-unrepresentable + determinism doctrine |
| **Requirement** | Credential identity is faithful end-to-end: a stake/committee/DRep credential is a closed sum over {KeyHash, ScriptHash} of a 28-byte hash, never a tag-erased Hash28. Both era certificate decoders preserve the key/script discriminant (an unknown tag is a deterministic reject); the credential type makes a discriminant-less credential unrepresentable on the BLUE authority path (no Hash28->credential coercion); CertState (registrations/delegations/rewards) and ConwayGovState (vote_delegations/committee_hot_keys/drep_expiry) key on the discriminated credential, so a key-hash and a script-hash credential sharing 28 bytes are distinct authoritative-state keys (matching cardano-node's Credential-keyed UMap/VState); and the canonical fingerprint serializes the discriminant so two states differing only in a credential's key/script tag fingerprint differently. The discriminated representation is a deliberate cert-state + gov-state fingerprint migration (T-DET-01). Default scope is Shelley+; Byron is included only if proven to have an affected credential surface. |
| **Code** | crates/ade_types/src/shelley/cert.rs (StakeCredential enum {KeyHash,ScriptHash} + hash()) — OQ5-S1; crates/ade_codec/src/shelley/cert.rs + crates/ade_codec/src/conway/cert.rs (decode_stake_credential preserves the tag, rejects unknown) — OQ5-S1; crates/ade_ledger/src/state.rs (ConwayGovState re-keyed to StakeCredential), gov_cert.rs, governance.rs, cert_classify.rs, rules.rs (cred.hash() boundary adapter for the Hash28-keyed stake snapshot) — OQ5-S1; crates/ade_ledger/src/fingerprint.rs (write_stake_credential emits discriminant+hash; gov-map writers use it; stake-snapshot writer stays write_hash28) — OQ5-S1; crates/ade_testkit/src/harness/snapshot_loader.rs (GREEN: gov-map + DRep-reg parses preserve the tag) — OQ5-S1; crates/ade_ledger/src/governance.rs (committee ratification by full-credential equality, no .hash() on the committee path) + crates/ade_ledger/src/state.rs (ConwayGovState.committee StakeCredential-keyed) + crates/ade_types/src/conway/governance.rs (GovActionState.committee_votes StakeCredential) + crates/ade_ledger/src/fingerprint.rs (write_committee_vote_list) + crates/ade_testkit/src/harness/snapshot_loader.rs (parse_committee_state + parse_committee_vote_map tag-preserving) — COMMITTEE-CRED-FIDELITY-S1; crates/ade_types/src/conway/governance.rs (GovActionState.drep_votes StakeCredential) + crates/ade_ledger/src/governance.rs (lookup_stake exact-variant DRep resolution, no OR-fallback) + crates/ade_ledger/src/fingerprint.rs (write_credential_vote_list, renamed from write_committee_vote_list, serves committee+drep) + crates/ade_testkit/src/harness/snapshot_loader.rs (parse_credential_vote_map, renamed, serves committee+drep) — DREP-VOTE-FIDELITY-S1; crates/ade_ledger/src/governance.rs (EnactmentEffects.committee_changes StakeCredential-typed — dormant, prevents committee-enactment re-collapse) — ENACTMENT-COMMITTEE-FIDELITY-S1; crates/ade_types/src/conway/governance.rs (GovAction::UpdateCommittee structured {removed: BTreeSet<StakeCredential>, added: BTreeMap<StakeCredential,u64>, threshold}, replacing opaque raw bytes) + crates/ade_ledger/src/fingerprint.rs (write_gov_action emits the structured 5-field shape via write_stake_credential) + crates/ade_testkit/src/harness/snapshot_loader.rs (parse_cold_credential/_set/_epoch_map + parse_unit_interval, fail-closed on unknown tag) — ENACTMENT-COMMITTEE-WRITEBACK-S1; crates/ade_ledger/src/governance.rs (apply_committee_enactment pure transition: dissolve + discriminated remove/add + quorum) + crates/ade_ledger/src/rules.rs (epoch-boundary apply site calls it) — ENACTMENT-COMMITTEE-WRITEBACK-S2 |
| **Tests** | `shelley_credential_preserves_discriminant`; `conway_credential_preserves_discriminant`; `unknown_credential_tag_rejects`; `discriminant_changes_fingerprint`; `keyhash_scripthash_same_bytes_are_distinct_certstate`; `keyhash_scripthash_same_bytes_are_distinct_govstate`; `discriminant_changes_fingerprint_corpus`; `credential_accumulation_replays_byte_identical`; `committee_keyhash_scripthash_do_not_cross_resolve`; `committee_keyhash_scripthash_same_bytes_distinct` … (+10 more) |
| **CI** | `ci/ci_check_credential_discriminant_closed.sh` |

#### `DC-LEDGER-11` — _enforced_

| Aspect | Location |
|--------|----------|
| **Source** | CIP-1694 (proposal_procedure = [coin, reward_account, gov_action, anchor]); Project constitution §3 (closed semantic surfaces, T-CORE-01); DC-LEDGER-10 (downstream credential discriminant must not be re-collapsed) |
| **Requirement** | proposal_procedures MUST NOT remain an opaque byte field in the authoritative Conway tx-body shape. ConwayTxBody.proposal_procedures is Option<Vec<ProposalProcedure>>, decoded through a single closed entry point (decode_proposal_procedures) that rejects unknown gov_action tags, structural failures, empty sets, and trailing garbage deterministically; the typed form re-encodes byte-identically (PreservedCbor) for every well-formed Conway tx body. The decoder reuses the existing closed GovAction enum (preserving DC-LEDGER-10 UpdateCommittee discriminant) and the existing opaque Anchor struct. |
| **Code** | crates/ade_codec/src/conway/governance.rs (decode_proposal_procedures, decode_proposal_procedure, decode_gov_action, encode_proposal_procedures); crates/ade_types/src/conway/governance.rs (ProposalProcedure); crates/ade_codec/src/conway/tx.rs (typed key 20 path); crates/ade_testkit/src/governance/proposal_procedures_replay.rs (PP-S2 canonical synthetic corpus + replay harness) |
| **Tests** | `roundtrip_info_action_proposal`; `roundtrip_hard_fork_initiation`; `roundtrip_no_confidence`; `roundtrip_treasury_withdrawals`; `roundtrip_parameter_change`; `roundtrip_new_constitution`; `roundtrip_update_committee`; `roundtrip_multi_procedure`; `rejects_unknown_gov_action_tag`; `rejects_empty_proposal_procedures_set` … (+11 more) |
| **CI** | `ci/ci_check_proposal_procedures_closed.sh` |

#### `DC-LEDGER-12` — _enforced_

| Aspect | Location |
|--------|----------|
| **Source** | docs/planning/phase4-n-c-invariants.md §1 (NC-FORGE-4); PHASE4-N-E mempool admit closure |
| **Requirement** | Every tx in a forged block is admissible via ade_ledger::mempool::admit against the base ledger state, in the snapshot's canonical accumulating order. No tx in a forged block bypasses mempool validation. Forge MUST NOT permute, fabricate, or skip the snapshot's canonical accumulating order. |
| **Code** | crates/ade_ledger/src/producer/forge.rs (tx-admissibility prefix gate); crates/ade_ledger/src/mempool/admit.rs (admit — reused for prefix check) |
| **Tests** | `forge_block_rejects_tx_not_in_mempool_accepted_prefix`; `forge_block_rejects_tx_permuted_from_accumulating_order`; `forge_block_empty_mempool_produces_empty_body`; `admit_prefix_property_documented` |
| **CI** | `ci/ci_check_forge_purity.sh` |

### DC-LIVEMEM

#### `DC-LIVEMEM-01` — _enforced_

| Aspect | Location |
|--------|----------|
| **Source** | docs/planning/phase4-n-f-g-e-invariants.md |
| **Requirement** | Live-feed bounded memory (operational-hardening; NOT BLUE consensus law). Peer-driven memory on the live --mode node feed is bounded BEFORE authoritative decode/apply: a per-mini-protocol reassembly buffer (ade_network::session::core proto_buffers) over 16 MiB fails closed with a structured SessionError (drop the peer); the WirePump lookahead (ade_node::node_sync) stops opportunistic draining at 256 buffered blocks, letting the existing bounded mpsc (cap 64) back-pressure the pump. No silent truncation, no partial decode, no unbounded fallback. The bounds are CLOSED CONSTANTS -- defensive implementation bounds, NOT Cardano semantic parameters; they may be tightened by a future hardening slice, but no runtime option (CLI / env / config) may disable them or set them to unbounded. The cap fires before the BLUE decode path (unchanged); the verdict-decoupled NodeBlockSource contract, the relay-loop containment, and the served-chain handoff fence are all unchanged. |
| **Code** | crates/ade_network/src/session/core.rs (GREEN 16 MiB reassembly-tail cap MAX_REASSEMBLY_TAIL_BYTES + additive SessionError::ReassemblyBufferOverflow); crates/ade_node/src/node_sync.rs (RED 256-block WirePump lookahead-depth cap MAX_WIRE_PUMP_LOOKAHEAD); crates/ade_runtime/src/network/mux_pump.rs (the overflow variant maps to PeerHaltReason::ChainSyncDecodeError — drop the peer) |
| **Tests** | `session_reassembly_tail_over_cap_fails_closed`; `session_reassembly_tail_under_cap_still_drains_complete_item`; `wirepump_lookahead_stops_at_cap`; `wirepump_lookahead_cap_preserves_relay_behavior_under_normal_feed` |
| **CI** | `ci/ci_check_live_feed_memory_bounds.sh` |

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
| **Code** | — _(declared — not yet enforced)_ |
| **Tests** | — _(declared — not yet enforced)_ |
| **CI** | — _(declared — not yet enforced)_ |

#### `DC-MEM-06` — _partial_

| Aspect | Location |
|--------|----------|
| **Source** | Project constitution §4 (determinism); RFC 8949 §4.2.1; MEM-OPT cluster plan |
| **Requirement** | The UTxO/ledger state fingerprint is computed by the canonical CBOR encoder over canonically-encoded (fixed-width big-endian) keys, NEVER from a storage backend's native iteration order, AND is independent of the process memory allocator (allocation addresses/sizes are never fingerprinted). Store iteration order and the allocator are implementation details and must not enter any authoritative fingerprint. |
| **Code** | ci/ci_check_alloc_determinism_neutral.sh; crates/ade_node/src/main.rs; crates/ade_runtime/src/seed_import/importer.rs |
| **Tests** | `streaming_matches_whole_buffer_across_fixtures`; `streaming_fingerprint_independent_of_textual_order`; `streaming_surfaces_conversion_error_not_swallowed`; `streaming_rejects_duplicate_txin_fail_closed`; `streaming_rejects_exact_duplicate_string_key_but_oracle_collapses` |
| **CI** | `ci/ci_check_alloc_determinism_neutral.sh`; `ci/ci_check_mem_opt_s2_import_peak.sh` |

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
| **Code** | — _(declared — not yet enforced)_ |
| **Tests** | — _(declared — not yet enforced)_ |
| **CI** | — _(declared — not yet enforced)_ |

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
| **Tests** | `order_independent`; `add_remove_is_exact_inverse`; `binds_value_not_just_key`; `golden_empty_digest`; `golden_single_entry_digest`; `golden_two_entry_digest`; `v1_and_v2_utxo_components_differ_only_the_utxo_changes`; `fingerprint_v2_is_deterministic`; `fingerprint_v2_utxo_is_insertion_order_independent`; `incremental_v2_equals_full_recompute_after_each_block` … (+2 more) |
| **CI** | `ci/ci_check_utxo_fp_v2.sh` |

#### `DC-MEM-11` — _enforced_

| Aspect | Location |
|--------|----------|
| **Source** | docs/active/live-follow-throughput-handoff.md (C2-PREVIEW-BA02: forge per-block admit ~20s CPU/block @ 99.8% CPU -- the producer kept pace with the chain but could never CLOSE the catch-up backlog, so it never reached the live tip / a live leader slot) |
| **Requirement** | The network forward-sync / forge per-block admit MUST derive the WAL post_fp from the CACHED UTxO-component fingerprint (ForwardSyncState.utxo_fp_cache -> fingerprint_v2_with_utxo), never the full per-block fingerprint() recompute, and the convergence-evidence post_fp MUST reuse the running state.prior_fp the reducer just computed (no second recompute per admit). Under the live track_utxo=false follow the imported UTxO is invariant, so OverlayUtxo::generation is stable across the per-block ledger clones AND rollbacks and the UTxO component (a Ristretto255 set commitment, O(n) over the ~1.9M-entry UTxO) is computed ONCE and reused; any UTxO mutation bumps the generation and forces a full recompute, so the cached post_fp is byte-identical to fingerprint() and the WAL post_fp chain + replay-equivalence are UNCHANGED. Pure optimization, NOT authoritative state. Extends the proven MEM-OPT-UTXO-DISK StaticUtxoFp/UtxoFpCache optimization (admission path) to the forward-sync path the cluster left on the full recompute. |
| **Code** | crates/ade_runtime/src/forward_sync/reducer.rs (ForwardSyncState.utxo_fp_cache; post_fp via fingerprint_v2_with_utxo + utxo_fp_cache.utxo_fingerprint; ForwardSyncState::invalidate_utxo_fp_cache); crates/ade_node/src/node_lifecycle.rs (emit_participant_admit reuses state.prior_fp; the RolledBack arm calls invalidate_utxo_fp_cache after commit_rollback); ci/ci_check_forward_sync_fp_cache.sh |
| **Tests** | `pump_block_post_fp_is_byte_identical_to_full_fingerprint`; `forward_sync_post_fp_cache_hit_is_byte_identical`; `forward_sync_replay_two_runs_byte_identical`; `forward_sync_admission_through_chokepoints` |
| **CI** | `ci/ci_check_forward_sync_fp_cache.sh` |

### DC-MITHRIL

#### `DC-MITHRIL-01` — _enforced_

| Aspect | Location |
|--------|----------|
| **Source** | docs/clusters/PHASE4-N-Y/S7-real-mithril-binding.md |
| **Requirement** | verify_mithril_binding is a pure deterministic BLUE predicate over its inputs (the manifest report + the anchor) — no I/O, no clock, no HashMap, no float, no String errors. Each field divergence maps to a distinct closed MithrilImportError variant (NetworkMagicMismatch, GenesisHashMismatch, CertifiedPointMismatch, CertificateHashMismatch, UnsupportedArtifactType); the compared sides come from two independent origins (Mithril manifest vs --json-seed-minted anchor), never a value vs itself. |
| **Code** | crates/ade_ledger/src/bootstrap_anchor/binding.rs |
| **Tests** | `mithril_anchor_binding_is_deterministic`; `mithril_anchor_rejects_field_mismatch`; `mithril_binding_rejects_certified_point_other_than_seed_point` |
| **CI** | `ci/ci_check_mithril_uses_bootstrap_initial_state.sh` |

#### `DC-MITHRIL-02` — _enforced_

| Aspect | Location |
|--------|----------|
| **Source** | docs/clusters/PHASE4-N-Z/cluster.md; S1-mithril-production-bootstrap.md |
| **Requirement** | For Mithril bootstrap, the BootstrapAnchor seed_point MUST be derived from the operator-provided independent seed-point extraction inputs, not from the Mithril manifest. The Mithril manifest may populate provenance and attestation fields (SeedProvenance::Mithril), but the binding check (verify_mithril_binding) MUST compare two structurally independent origins and fail closed on mismatch. In the production composition the manifest import may be referenced only as whole values (import.provenance -> seed_provenance; &import.report -> the verify call); the import's point-bearing fields must never be drilled into or laundered (via a local binding or a mutate-before-mint) into the anchor's seed_point. |
| **Code** | crates/ade_runtime/src/mithril_bootstrap.rs (bootstrap_from_mithril_snapshot — seed_point from operator inputs, verify-before-bootstrap, closed MithrilBootstrapError); ci/ci_check_mithril_seed_point_independence.sh (containment gate) |
| **Tests** | `mithril_bootstrap_fails_closed_on_seed_point_mismatch`; `mithril_bootstrap_verifies_before_storage_init`; `mithril_bootstrap_succeeds_when_seed_point_matches` |
| **CI** | `ci/ci_check_mithril_seed_point_independence.sh` |

#### `DC-MITHRIL-03` — _enforced_

| Aspect | Location |
|--------|----------|
| **Source** | docs/clusters/MITHRIL-VERIFIED-ANCHOR-INTEGRATION/SLICE-S1b-authority-transition.md; user directive 2026-06-23 (S1b = the native authority transition: assemble the complete seed from ONLY manifest/S1a/Stage-2/genesis, enforce point coherence, persist atomically before visibility, and remove the cardano-cli/JSON seed from the native bootstrap path; do NOT implement the Stage-2 UTxO materialization or the Conway per-byte min-UTxO calculator -- those are separate release blockers) |
| **Requirement** | The native Mithril AUTHORITY TRANSITION assembles the COMPLETE authoritative seed (LedgerState + PraosChainDepState + a NATIVE LiveConsensusInputsCanonical) from EXCLUSIVELY the verified manifest binding + the S1a NativeSnapshotNonUtxoState + the Stage-2 `tables` UTxO + genesis constants -- with NO cardano-cli, NO JSON consensus-input bundle, NO operator seed, and NO convenience fallback on this path; the verified snapshot IS the source. (a) NATIVE ASSEMBLY (no operator bundle): every field has a single declared source -- utxo_state <- Stage-2 UTxOState; cert_state + reserves + treasury + block_production + the five Praos nonces + protocol_params (incl. MinUtxoRule::PerByte) <- S1a; epoch_state.slot + network_magic + genesis_hash + source_tip + the anchor seed_point <- the manifest point; epoch_no + epoch_nonce(eta0) + pool stake/VRF <- S1a; active_slots_coeff + max_lovelace_supply <- genesis; the epoch window <- the era schedule + the S1a epoch; era = Conway; gov_state = None; conway_deposit_params = None; track_utxo = false; snapshots = cold-start empty; epoch_fees = 0; op-cert counters empty; last_* = None. The native LiveConsensusInputsCanonical carries a fixed native-source marker (never a cardano-cli command or node version) and protocol_params_json = None; its fingerprint is computed via the SOLE canonical-form authority (canonical_from_raw). (b) POINT COHERENCE is a TERMINAL gate BEFORE any assembly or persist: the S1a era == Conway, the S1a point == the manifest certified point (slot AND hash), the S1a network_id == the manifest-magic-derived id (mainnet 764824073 -> 1, any other -> 0), the S1a epoch == the epoch the schedule resolves for the certified slot, and the assembled anchor seed_point == the manifest point (the verify_mithril_binding leg inside the single closed composition). ANY mismatch / missing input is a structured terminal MithrilNativeAssemblyError (no authority assembled, NOTHING partial persisted). (c) PERSIST-BEFORE-VISIBILITY (atomic): the native entry routes through the SAME single closed composition bootstrap_from_mithril_snapshot -- the sole bootstrap_initial_state authority + the seed-epoch consensus sidecar + the recovered-anchor point (put_recovered_anchor_point) + the WAL provenance append. The WAL append is the SOLE point at which the anchor lineage becomes discoverable (warm-start recovery gates on the WAL provenance); an interrupted import -- any write failing before the WAL commit -- leaves NO bootable partial authority state (a warm-start recovers the store as 'not imported'), and the imported anchor point is explicitly evidenced + recoverable via load_recovered_anchor_point. There is NO second storage-init path. SCOPE: assembly + atomic persistence ONLY. NOT the Stage-2 `tables` -> UTxOState materialization (the native assembly CONSUMES a UTxOState; the materialization + its i64-MultiAsset release blocker is DC-LEDGER-VALUE-01 / the LEDGER-VALUE-CORRECTNESS cluster), NOT the Conway per-byte min-UTxO calculator (DC-LEDGER-PARAMS-01 release blocker), and NOT cold-restart / warm-start recovery / ChainSync / follow (S2, the release gate). The cardano-cli / JSON-seed importers STAY as RED diagnostic / oracle tooling, never bootstrap authority on this path. |
| **Code** | crates/ade_runtime/src/mithril_native_assembly.rs: VerifiedManifestBinding + NativeGenesisConstants + NativeMithrilSeed + MithrilNativeAssemblyError (NonConwayEra/PointMismatch/PointHashMismatch/EpochMismatch/NetworkMismatch/EpochWindowUnresolved); assemble_native_mithril_seed (the pure assembly + point-coherence terminal gate; field sources documented per field; native_consensus_inputs builds the LiveConsensusInputsCanonical via canonical_from_raw with NATIVE_SOURCE_MARKER + protocol_params_json=None; native_protocol_params_hash = blake2b(encode_pparams); chain_dep_from_nonces maps the five S1a nonces; derive_epoch_window from the era schedule); bootstrap_from_native_mithril_snapshot (routes through bootstrap_from_mithril_snapshot) + NativeMithrilBootstrapError. crates/ade_runtime/src/mithril_bootstrap.rs: bootstrap_from_mithril_snapshot (the single closed composition, unchanged). crates/ade_runtime/src/seed_epoch_lineage.rs: persist_seed_epoch_consensus_inputs (sidecar -> put_recovered_anchor_point -> WAL provenance commit). ci/ci_check_mithril_authority_transition.sh. |
| **Tests** | `native_assembled_seed_is_deterministic`; `native_assembly_maps_each_field_from_its_source`; `point_mismatch_is_terminal`; `point_hash_mismatch_is_terminal`; `wrong_era_is_terminal`; `wrong_network_is_terminal`; `epoch_mismatch_is_terminal`; `native_bootstrap_persists_and_anchor_point_is_recoverable`; `interrupted_persist_leaves_no_discoverable_anchor_lineage` |
| **CI** | `ci/ci_check_mithril_authority_transition.sh` |

#### `DC-MITHRIL-04` — _enforced_

| Aspect | Location |
|--------|----------|
| **Source** | docs/clusters/EPOCH-CONSENSUS-VIEW/SLICE-MITHRIL-VERIFIED-ANCHOR-IMPORT.md; user directive 2026-06-22/23 (the bounty bootstrap is a verified Mithril Cardano DB snapshot decoded natively; Stage 1 is a narrow non-emitting format-and-fidelity slice replacing the test loader's zeroed-VRF shortcut with a production-faithful state decoder before the tables path; the manifest-certified point is authoritative, the filename only a locator; telescope navigation explicit + era-tagged; real VRF mandatory; PoolDistr<->pool VRF cross-check; raw CBOR RED/diagnostic, evidence in Ade canonical bytes; no UTxO/admission mutation; same bytes + manifest => byte-identical CertState canonical + probe commitments; acceptance = local Preview corpus + one verified Mithril snapshot, same verdict) |
| **Requirement** | Native V2 LedgerDB `state` decode is faithful, fail-closed, and non-emitting. The cardano-node V2 (utxohd-mem, tablesCodecVersion 1) LedgerDB `state` CBOR is decoded as a DETERMINISTIC BLUE projection of the Conway NewEpochState into Ade's canonical CertState + pool distribution + Praos nonces -- the raw cardano-node CBOR is RED/diagnostic INPUT, never the authority; the authority is the Ade-canonical encode_cert_state bytes. (a) DETERMINISTIC: same `state` bytes + same authoritative epoch => byte-identical canonical CertState + the same probe commitment (blake2b of encode_cert_state). (b) EXPLICIT ERA-TAGGED NAVIGATION: the HardFork telescope is navigated by counting past eras (each carrying an end bound) to the ONE current era (carrying the live state); the current era index MUST equal Conway -- never a silent "take the latest element"; any other current era is terminal UnsupportedEra. (c) MANDATORY REAL VRF: every active pool's VRF is decoded from the pstate pool-params (never the zeroed-VRF shortcut); a zero VRF is terminal ZeroVrf; PoolDistr and the decoded pools cross-check on VRF where both expose it -- a mismatch is terminal PoolDistrVrfMismatch even at zero stake. (d) POINT AUTHORITY: the decoded NES epoch (internal to the certified snapshot's content) is cross-checked against the authoritative epoch derived from the verified Mithril-certified point's beacon -- NEVER the filename-derived slot; a mismatch is terminal EpochMismatch. (e) ROUND-TRIP FAITHFUL: the decoded CertState survives a canonical encode/decode round-trip (terminal RoundTripMismatch otherwise). (f) NON-EMITTING (Stage 1): the decode yields a structured probe report only -- NO LedgerState / UTxO seed / admission artifact (the `tables`/UTxO reader + the admission anchor are Stage 2). Malformed CBOR halts deterministically (MalformedCbor; bounded reads, no best-effort partial decode). Validated against a real cardano-node V2 Preview snapshot (704 pools, all real-VRF; counts -- 704 pools / 60329 delegations / 90099 rewards -- match the independent cardano-cli consensus/certstate producer run) AND a verified Mithril preprod ancillary snapshot (epoch 296, 528 pools all real-VRF, the same structural verdict, the NES epoch == the certificate beacon). |
| **Code** | crates/ade_ledger/src/ledgerdb_state.rs: probe_ledgerdb_state (the entry; LedgerDbStateProbe the sole output), navigate_to_current_era (explicit era-tagged telescope nav -> UnsupportedEra), read_pool_params (ZeroVrf), read_cert_state/read_pool_map/read_dstate (CertState w/ real VRF + delegations/rewards), read_pool_distr + the PoolDistrVrfMismatch cross-check, extract_praos_nonces_v2 (the trailing-SIX PraosState nonces [evolving, candidate, epoch, previousEpoch, lab, lastEpochBlock]; B2c corrected the prior trailing-5 that dropped the evolving nonce), map_each (CBOR indefinite maps), the round-trip self-check (decode_cert_state), LedgerDbStateError (the closed terminal set). crates/ade_ledger/tests/ledgerdb_state_hermetic.rs (synthetic minimal-V2 fail-closed + round-trip + determinism). crates/ade_runtime/tests/ledgerdb_state_corpus.rs (local Preview corpus). crates/ade_runtime/tests/ledgerdb_state_mithril.rs (verified Mithril ancillary). ci/ci_check_ledgerdb_state_decode.sh. |
| **Tests** | `happy_minimal_state_decodes_with_required_elements`; `determinism_same_bytes_same_commitment`; `zero_vrf_is_terminal`; `wrong_era_is_terminal_no_fallback_to_latest`; `pool_distr_vrf_mismatch_is_terminal`; `epoch_mismatch_is_terminal`; `malformed_cbor_is_terminal`; `decode_local_preview_corpus`; `decode_verified_mithril_ledger_state` |
| **CI** | `ci/ci_check_ledgerdb_state_decode.sh` |

#### `DC-MITHRIL-05` — _enforced_

| Aspect | Location |
|--------|----------|
| **Source** | docs/clusters/EPOCH-CONSENSUS-VIEW/SLICE-MITHRIL-VERIFIED-ANCHOR-IMPORT.md; user directive 2026-06-23 (Stage 2 = the V2 LedgerDB tables + MemPack TxOut COMPATIBILITY decoder, NOT a CBOR reader; faithful u64 quantity with the i64 ceiling logged as a separate BLOCKING downstream validation obligation, never widen MultiAsset inside the snapshot-decoder slice -- different authority surface; preserve original script/inline-datum bytes where Cardano hash/identity rules require wire bytes; the meaningful oracle comparison is real TxIn -> native decode -> full TxOut vs cardano-cli query utxo; whole-tables proof = a deterministic commitment over sorted TxIn -> canonical TxOut, not just a count) |
| **Requirement** | Faithful Word64 multi-asset quantity on the snapshot-import path. The native V2 LedgerDB `tables` MemPack TxOut decode keeps every multi-asset quantity as a full u64 (Word64) -- NEVER truncated, saturated, or cast to i64. A persisted/imported snapshot quantity is never lost: a real Cardano output can hold up to 2^64-1 of a token (i64::MAX is the common max-supply mint; some exceed it), so the decoder's `TxOutValue` holds `BTreeMap<PolicyId, BTreeMap<AssetName, u64>>` and the whole-tables canonical commitment serializes each quantity big-endian as u64. The surrounding decode is fail-closed + faithful: the compact (non-CBOR) TxOut value is decoded via the grounded MemPack layout (the 6-way constructor tag; CompactAddr / Addr28Extra base-address reconstruction with the explicit BE->LE hash double-flip and the payment/stake hash byte-order asymmetry; CompactValue ada-only + multi-asset rep; datum/script with the ORIGINAL inline-datum / script wire bytes PRESERVED, never a re-encode where Cardano hashes the wire bytes); endianness is explicit (no host-endianness contract); CONSUME-EXACTLY is enforced at every nesting boundary; every unknown tag / address form / script language / over-long VarLen is a structured TERMINAL error (no opaque keep-bytes); the tables decode is era-bound to Conway taken from the SAME snapshot's `state` (the Stage-1 NES), never the tables file or a CLI flag; and the whole-tables commitment is a deterministic blake2b chain over the canonically (ascending-TxIn) sorted UTxO (a non-sorted map is terminal). DERIVED: Cardano multi-asset quantities are Word64-compatible in this decode path. RELEASE BLOCKER (separate, downstream): Ade's i64 `MultiAsset` model cannot yet safely validate every real Cardano UTxO containing quantities > i64::MAX -- full ledger validation of such outputs is gated until the value-model quantity is widened (this is NOT cleanup). Validated: 300000 real preprod TxOuts decode faithfully (all 6 tags, consume-exactly); a cardano-cli `query utxo` oracle cross-check matched 10/10 (6 tag-2 Addr28Extra base addresses + coins) -- closing PO#1; the i64::MAX / i64::MAX+1 / u64::MAX quantities round-trip exactly. |
| **Code** | crates/ade_ledger/src/ledgerdb_tables.rs: MemPackReader (explicit read_u16/u32/u64_le, read_varlen BE-7bit, expect_consumed); read_compact_addr/validate_address_form; read_staking_credential/read_addr28_base_address (PO#1 BE->LE double-flip + payment/stake hash asymmetry); read_compact_value/decode_multiasset_rep (faithful u64; rep regions A-E + nubOrd); read_compact_coin (tag-2/3 standalone CompactForm Coin = [0x00][VarLen]); read_datum_option/read_script (preserved bytes + Conway Plutus language byte); read_txout (6-tag dispatch); TxOutValue (u64 assets); canonical_txout_bytes + decode_tables_commitment (PO#2 era binding + deterministic sorted commitment); TablesDecodeError (closed terminal set). crates/ade_runtime/tests/ledgerdb_tables_decode.rs; crates/ade_runtime/tests/ledgerdb_tables_oracle.rs; ci/ci_check_ledgerdb_tables_decode.sh. |
| **Tests** | `multiasset_quantities_preserved_exactly_as_u64_no_i64_cast`; `coin_varlen_overflow_is_terminal`; `addr28_base_address_reconstruction_round_trip`; `staking_credential_tag_is_fail_closed`; `compact_value_ada_only_and_multiasset`; `txout_dispatch_tag0_tag5_and_fail_closed`; `tables_commitment_deterministic_era_bound_and_sorted`; `varlen_big_endian_7bit_matches_real_coin`; `decode_real_preprod_tables_commitment` |
| **CI** | `ci/ci_check_ledgerdb_tables_decode.sh` |

#### `DC-MITHRIL-06` — _enforced_

| Aspect | Location |
|--------|----------|
| **Source** | docs/clusters/MITHRIL-VERIFIED-ANCHOR-INTEGRATION/SLICE-S1c-tables-to-utxostate.md; user directive 2026-06-24 (S1c = the Stage-2 tables -> authoritative UTxOState materialization: a pure DecodedTxOut -> ledger TxOut converter with the hash-critical inline-datum / reference-script bytes embedded verbatim via the tag-24 authority and the faithful u64 quantities carried into OutputAssetQuantity, canonical-ascending TxIn materialization into UTxOState::from_map, fail-closed on any unsupported form, and a fingerprint_utxo_v2 binding to the one manifest point + the Stage-1 + Stage-2 commitments; reuse the value model, the ledger TxOut / UTxOState::from_map / fingerprint_utxo_v2, the Stage-2 read_txout, and the existing CBOR primitives -- do not fork them; this unblocks the native FirstRun route step 4, does NOT touch node_lifecycle or add CLI flags) |
| **Requirement** | The Stage-2 `tables` (MemPack-decoded TxOuts) materialize into Ade's authoritative `UTxOState` with hash-critical bytes PRESERVED and full Word64 quantities carried through -- the converter that unblocks the native Mithril FirstRun UTxO seed (the DC-MITHRIL-03 / S1b blocker). (a) PURE CONVERTER: a pure `decoded_txout_to_ledger(DecodedTxOut) -> Result<TxOut, TxOutMaterializeError>` (closed error enum). No datum AND no script -> `TxOut::ShelleyMary` (or `TxOut::Byron` when the address header nibble is Byron), the multi-asset bundle built by wrapping each Stage-2 `u64` quantity into `OutputAssetQuantity(u64)` -- NEVER truncated / saturated / i64-cast. Datum OR script present -> `TxOut::AlonzoPlus { raw, address, coin }` where `raw` is the canonical Conway TxOut CBOR map (keys ascending: 0=address; 1=value -- coin uint when ada-only, else `[coin, {policy: {name: qty}}]` with each `qty` a CBOR UNSIGNED int u64 and the policy/name maps canonical-sorted; 2 if datum -- Hash(h)->`[0,h]`, Inline(b)->`[1, #6.24(b)]`; 3 if script -- `#6.24([type, script_bytes])` with Native->`[0,bytes]` and Plutus version n->`[n,bytes]`, V1->1/V2->2/V3->3). (b) HASH-CRITICAL BYTES VERBATIM: the inline-datum bytes and the script bytes are embedded VERBATIM inside the tag-24 (`#6.24`, CBOR-encoded-CBOR) via the single workspace tag-24 authority `ade_codec::wrap_tag24` -- NEVER re-decoded/re-encoded (they are the identity bytes Cardano hashes; ade_plutus reads `raw` directly for the ScriptContext). The raw map is built with the shared ade_codec cbor primitives (write_map_header / write_uint_canonical / write_bytes_canonical / write_array_header), not a forked encoder. (c) MATERIALIZATION: `materialize_tables_to_utxo` iterates the `tables` CBOR map in canonical ASCENDING TxIn order (non-ascending / duplicate key is terminal), parses each 34-byte TxIn key (32 txid + 2 big-endian index), decodes each TxOut via the Stage-2 `read_txout` and promotes it, accumulates a `BTreeMap<TxIn, TxOut>` -> `UTxOState::from_map`; era-bound to Conway (taken from the SAME snapshot's Stage-1 `state`, never the tables file or a CLI flag); FAIL-CLOSED on any unsupported TxOut tag / address form / value tag / script language / non-ascending or malformed key -- a STRUCTURED terminal error, NEVER an opaque keep-bytes fallback. (d) COMMITMENT BINDING: the materialized `UTxOState` -> `fingerprint_utxo_v2`; a `bind_utxo_to_manifest` record (blake2b) over the manifest certified point hash + the Stage-1 `NativeSnapshotNonUtxoState` commitment + the Stage-2 `decode_tables_commitment` + the UTxO fingerprint_v2, with `verify_utxo_binding` TERMINAL on any mismatch -- the UTxO authority is visible only when all four bind to the one manifest point. (e) RECOVERY: a materialized `UTxOState` survives persist (`encode_utxo_state`) -> recover (`decode_utxo_state`) with an IDENTICAL fingerprint_v2, and a u64 > i64::MAX output quantity round-trips exactly through that cycle. DERIVED -- the Stage-2 faithful-u64 TxOuts (DC-MITHRIL-05) are promoted into the widened OUTPUT value model (DC-LEDGER-VALUE-01) and into `UTxOState` without loss. SCOPE: materialization + commitment binding ONLY -- NOT node_lifecycle wiring, NOT live CLI flags (step 4, gated on this), NOT the Conway per-byte min-UTxO calculator (DC-LEDGER-PARAMS-01). |
| **Code** | crates/ade_ledger/src/mithril_utxo_materialize.rs: decoded_txout_to_ledger (the pure converter + TxOutMaterializeError closed enum), build_multi_asset (u64 -> OutputAssetQuantity, faithful Word64), encode_conway_txout_raw (the canonical Conway TxOut map; inline-datum + script bytes embedded verbatim via ade_codec::wrap_tag24), encode_script_inner (Native->[0,bytes] / PlutusVn->[n,bytes]), write_value (coin uint \| [coin, {policy:{name:qty}}] canonical-sorted, qty a CBOR uint), parse_txin_key (34-byte 32+2 BE), materialize_tables_to_utxo (canonical-ascending Conway-bound fail-closed -> UTxOState::from_map), bind_utxo_to_manifest / verify_utxo_binding / UtxoBindingRecord / UtxoBindingMismatch (fingerprint_utxo_v2 bound to manifest point + Stage-1 + Stage-2, terminal on mismatch). crates/ade_runtime/tests/mithril_tables_to_utxostate.rs (real preprod tables sample -> UTxOState, deterministic fp_v2, binding holds, datum/script + pure-payment present). ci/ci_check_tables_to_utxostate.sh. |
| **Tests** | `deterministic_utxo_commitment`; `u64_above_i64_max_materializes_persists_recovers_exactly`; `datum_and_script_bytes_preserved_verbatim_in_raw`; `alonzo_plus_raw_round_trips_to_same_fields`; `canonical_txin_ordering_asserted`; `fail_closed_negatives`; `binding_is_terminal_on_mismatch`; `persist_recover_identical_fingerprint`; `no_datum_no_script_is_shelley_mary_byron_is_byron`; `materialized_count_matches_stage2_commitment_count` … (+1 more) |
| **CI** | `ci/ci_check_tables_to_utxostate.sh` |

#### `DC-MITHRIL-07` — _enforced_

| Aspect | Location |
|--------|----------|
| **Source** | docs/clusters/MITHRIL-VERIFIED-ANCHOR-INTEGRATION/SLICE-S1d-live-firstrun-native.md; user directive 2026-06-24 (S1d = wire the live --mode node FirstRun to the native Mithril bootstrap: route manifest + state + tables + Shelley genesis through the unchanged S1a/S1b/S1c chain, forbid the cardano-cli/JSON seed on the native route with a structured terminal on a forbidden flag, build the snapshot-epoch era schedule from the genesis + the per-network Shelley boundary without the operator bundle, and prove it hermetically + live against the real preprod snapshot; do NOT touch S1a/S1b/S1c, cold restart / warm-start / ChainSync, or the Conway per-byte min-UTxO calculator) |
| **Requirement** | The live `--mode node` FirstRun arm INVOKES the native Mithril bootstrap path (DC-MITHRIL-03 / S1b) from live snapshot files -- it routes the verified Mithril manifest + the V2 LedgerDB `state` (S1a) + the Stage-2 `tables` MemPack (S1c) + the Cardano Shelley genesis (the required metadata file) through the UNCHANGED S1a/S1b/S1c native chain, with NO cardano-cli / JSON consensus-input bundle / operator seed on this route; the verified snapshot IS the source. (a) ROUTE SELECTION: when `--mithril-state-path` AND `--mithril-tables-path` are BOTH present, the FirstRun arm takes the native route (first_run_native_mithril_bootstrap) -- which supersedes the legacy CLI-seed body; the two are NEVER a fallback for one another. (b) THE WIRING (native_firstrun::native_first_run_bootstrap): read manifest bytes -> import_mithril_manifest_from_bytes -> the VerifiedManifestBinding (certified_point, network_magic, genesis_hash, immutable_range); load the Shelley genesis -> NativeGenesisConstants (maxLovelaceSupply + activeSlotsCoeff, the latter parsed decimal-string -> exact rational with NO float arithmetic) + the genesis epochLength; derive the certified slot's epoch from the CLOSED per-network Shelley boundary (mainnet epoch 208 / slot 4_492_800; preprod epoch 4 / slot 86_400; preview epoch 0 / slot 0 -- an unknown magic is terminal, never a guessed boundary) + the genesis epoch length -> the manifest_epoch; decode_native_nonutxo_state(state, certified_point, manifest_epoch, network_magic) -> (s1a, s1a_commitment) (S1a, the snapshot's own NES epoch cross-checked == manifest_epoch); materialize_tables_to_utxo(tables, CONWAY_ERA_INDEX, None) -> UTxOState (S1c, the WHOLE file on the live route); build the single-era Conway EraSchedule anchored at the snapshot epoch's ABSOLUTE first slot (start_slot + (snapshot_epoch - start_epoch) * epoch_length) so locate(certified_slot).epoch == snapshot_epoch AND derive_epoch_window(schedule, snapshot_epoch) CONTAINS the certified slot; bootstrap_from_native_mithril_snapshot(s1a, s1a_commitment, utxo, binding, genesis_constants, manifest_bytes, chaindb, chaindb, wal, era_schedule, ledger_view) (S1b) -> the MithrilBootstrapOutput. The persistent ChainDb / FileWalStore are REUSED (not re-opened); the leadership view is built faithfully from the assembled consensus inputs (the cold-start composition never consumes it). (c) THE CLI-SEED IS FORBIDDEN ON THE NATIVE ROUTE: a forbidden flag (--json-seed-path / --consensus-inputs-path) supplied ALONGSIDE the native inputs is a structured TERMINAL NativeRouteForbiddenFlag error BEFORE any decode (no ambiguous / half-authoritative bootstrap, no fallback, no silent ignore); the native route reaches NONE of import_cardano_cli_json_utxo / import_live_consensus_inputs. (d) FAILURE SEMANTICS (TERMINAL before authority visibility -- the WAL commit-point inside bootstrap_from_native_mithril_snapshot is the SOLE discovery gate): a missing / mixed component (manifest / state / tables / shelley genesis not all present) -> terminal before any decode; a malformed manifest / shelley genesis -> terminal; a manifest / point / network / era mismatch -> terminal (the S1a epoch cross-check + the S1b point-coherence gate); a decode (S1a) / materialize (S1c) / assemble (S1b) / persist failure -> terminal. Every failure leaves NO bootable partial authority state and NO fallback to the cardano-cli / JSON seed. SCOPE: the live FirstRun INVOCATION wiring ONLY -- S1a/S1b/S1c are FROZEN (reused unchanged), no new state machinery. NOT cold restart / warm-start recovery / ChainSync / follow (S2, the release gate), NOT the Conway per-byte min-UTxO calculator (DC-LEDGER-PARAMS-01 release blocker). The legacy CLI-seed FirstRun body remains a SEPARATE explicitly-selected route (state/tables absent), NEVER a fallback from the native route. |
| **Code** | crates/ade_node/src/native_firstrun.rs: native_first_run_bootstrap (the native orchestration: import_mithril_manifest_from_bytes -> VerifiedManifestBinding; parse_native_shelley_genesis -> NativeGenesisFacts {NativeGenesisConstants + epoch_length_slots; activeSlotsCoeff via decimal_text_to_rational, no float}; shelley_boundary_for_magic (closed per-network Shelley start); epoch_for_certified_slot; decode_native_nonutxo_state; materialize_tables_to_utxo(.., CONWAY_ERA_INDEX, None); build_native_schedule (single-era Conway anchored at the snapshot epoch's absolute start); bootstrap_from_native_mithril_snapshot) + NativeFirstRunError (ComponentRead/ManifestImport/GenesisParse/UnknownNetworkBoundary/EpochGeometry/NonUtxoDecode/UtxoMaterialize/NativeBootstrap) + NativeGenesisParseError. crates/ade_node/src/node_lifecycle.rs: first_run_mithril_bootstrap (selects the native route on --mithril-state-path && --mithril-tables-path), first_run_native_mithril_bootstrap (forbids --json-seed-path / --consensus-inputs-path with NativeRouteForbiddenFlag; requires manifest+state+tables+shelley-genesis; reads the four files; routes through native_first_run_bootstrap), NodeLifecycleError::{NativeRouteForbiddenFlag, NativeFirstRun}. crates/ade_node/src/cli.rs: --mithril-state-path / --mithril-tables-path / --shelley-genesis-path. crates/ade_node/tests/native_firstrun_live.rs (real preprod snapshot). ci/ci_check_native_firstrun_no_cli_seed.sh. |
| **Tests** | `native_first_run_forbidden_json_seed_is_terminal`; `native_first_run_forbidden_consensus_inputs_is_terminal`; `native_first_run_missing_manifest_is_terminal`; `native_first_run_missing_shelley_genesis_is_terminal` †; `native_first_run_malformed_manifest_is_terminal`; `native_first_run_malformed_shelley_genesis_is_terminal`; `native_first_run_real_snapshot_invokes_bootstrap_and_persists`; `native_first_run_real_snapshot_wrong_network_is_terminal`; `preprod_snapshot_epoch_window_contains_manifest_point`; `shelley_genesis_active_slots_coeff_decimal_to_rational` … (+2 more) |
| **CI** | `ci/ci_check_native_firstrun_no_cli_seed.sh` |

#### `DC-MITHRIL-08` — _enforced_

| Aspect | Location |
|--------|----------|
| **Source** | docs/clusters/MITHRIL-VERIFIED-ANCHOR-INTEGRATION/SLICE-S2-mithril-first-run-continuity.md; user directive 2026-06-24 (make the native Mithril FirstRun boundary-usable: build the EVIEW reduced checkpoint INLINE on the native route before the UTxO is dropped, gated on delegations, so a Mithril-started node is not inert at the boundary; implement INLINE -- do NOT couple native_firstrun to admission::bootstrap) |
| **Requirement** | The native Mithril FirstRun is BOUNDARY-COMPLETE: when the decoded cert-state carries delegations (the EVIEW package), native_first_run_bootstrap builds the live EVIEW reduced-UTxO checkpoint INLINE from the materialized UTxO -- reduce_txout each output -> ReducedUtxoCheckpoint::build_from -> seal_bootstrap at the certified slot -- BEFORE the UTxO is consumed by the bootstrap. So a Mithril-started node persists (reduced-checkpoint.redb + the seed sidecar + the durable tip), exactly the triple ECA activation requires: at the next epoch boundary the node DERIVES + PROMOTES its own next-epoch authority (not inert). (a) GATED: a snapshot whose cert-state has no delegations builds NO checkpoint and the bootstrap output is BYTE-IDENTICAL (DC-EPOCH-11 point 8). (b) INLINE: the build uses the underlying BLUE/GREEN primitives (ade_ledger::reduced_utxo::reduce_txout, ade_runtime::chaindb::ReducedUtxoCheckpoint) directly -- it does NOT couple native_firstrun to admission::bootstrap's private helper; the two RED bootstrap paths stay independent (the shared authority is the primitive, not a RED helper). (c) FAIL-CLOSED: an open / build_from / seal_bootstrap failure is a terminal NativeFirstRunError::ReducedCheckpoint BEFORE authority visibility (the WAL commit-point inside the bootstrap stays the sole discovery gate; the inline build runs before it) -- NO bootable partial state. SCOPE: this is the inline checkpoint-build mechanism (Gap 2 of the slice); the judge-facing --bootstrap-mithril command (Gap 1) and the LIVE boundary continuity proof (cold restart + ChainSync + cross-boundary promotion + forge + Haskell adoption) are separate -- the live continuity flips DC-EPOCH-11 / DC-EVIEW-08, not this rule. |
| **Code** | crates/ade_node/src/native_firstrun.rs: native_first_run_bootstrap gains snapshot_dir + the gated inline reduced-checkpoint build (reduce_txout -> ReducedUtxoCheckpoint::open/build_from/seal_bootstrap(binding.certified_point.slot)) after materialize_tables_to_utxo, before the bootstrap consumes the UTxO; NativeFirstRunError::ReducedCheckpoint (terminal). crates/ade_node/src/node_lifecycle.rs: first_run_native_mithril_bootstrap threads cli.snapshot_dir. ci/ci_check_native_firstrun_reduced_checkpoint.sh. |
| **Tests** | `native_first_run_real_snapshot_invokes_bootstrap_and_persists` |
| **CI** | `ci/ci_check_native_firstrun_reduced_checkpoint.sh` |

### DC-NET

#### `DC-NET-01` — _declared_

| Aspect | Location |
|--------|----------|
| **Source** | Project constitution §3, T-RESOURCE-01, T-TRANSPORT-01 |
| **Requirement** | Peer selection uses three-tier management (cold/warm/hot) with bounded admission, per-peer resource limits, and eviction policies |
| **Code** | — _(declared — not yet enforced)_ |
| **Tests** | — _(declared — not yet enforced)_ |
| **CI** | — _(declared — not yet enforced)_ |

### DC-NODE

#### `DC-NODE-01` — _enforced_

| Aspect | Location |
|--------|----------|
| **Source** | docs/planning/phase4-n-k-orchestrator-binary-invariants.md §1 (I-3) |
| **Requirement** | Per-peer session isolation: one peer session's failure (decode error, validity reject, rollback-too-deep, protocol violation) halts only that peer's session. The orchestrator continues serving other peers and producing blocks. No cross-peer state sharing; each peer session owns its own working memory. |
| **Code** | crates/ade_runtime/src/orchestrator/peer_session.rs, crates/ade_runtime/src/orchestrator/core.rs |
| **Tests** | `crates/ade_runtime/tests/orchestrator_peer_isolation.rs::peer_session_isolation_holds_under_failure`; `crates/ade_runtime/tests/orchestrator_peer_isolation.rs::peer_session_per_peer_state_does_not_cross`; `crates/ade_runtime/tests/orchestrator_peer_isolation.rs::peer_disconnect_removes_only_that_peer`; `crates/ade_runtime/src/orchestrator/peer_session.rs::tests::peer_session_isolation_across_two_concurrent_tasks`; `crates/ade_runtime/src/orchestrator/core.rs::tests::step_per_peer_decode_error_isolates` |
| **CI** | `ci/ci_check_peer_session_isolation.sh` |

#### `DC-NODE-02` — _enforced_

| Aspect | Location |
|--------|----------|
| **Source** | docs/planning/phase4-n-k-orchestrator-binary-invariants.md §1 (I-4) |
| **Requirement** | Persistent-writer cadence fidelity: the orchestrator's persistent-snapshot writer calls PersistentSnapshotCache::capture only on the schedule emitted by the N-I SnapshotCadence policy. No orchestrator-side cadence override; no parallel cadence policy in the binary. Snapshot eviction is explicitly out of scope and is NOT an obligation of this rule (eviction is a storage concern, not node cadence fidelity). |
| **Code** | crates/ade_runtime/src/rollback/persistent_writer.rs, crates/ade_runtime/src/orchestrator/core.rs |
| **Tests** | `crates/ade_runtime/src/rollback/persistent_writer.rs::tests::persistent_writer_on_admitted_captures_only_on_cadence`; `crates/ade_runtime/src/rollback/persistent_writer.rs::tests::persistent_writer_round_trips_via_framing`; `crates/ade_runtime/src/rollback/persistent_writer.rs::tests::persistent_writer_force_capture_skips_cadence_but_updates_state`; `crates/ade_runtime/src/rollback/persistent_writer.rs::tests::persistent_writer_two_runs_are_deterministic`; `crates/ade_runtime/src/orchestrator/core.rs::tests::step_admit_triggers_capture_snapshot_at_cadence` |
| **CI** | `ci/ci_check_persistent_writer_no_parallel_cadence.sh` |

#### `DC-NODE-03` — _enforced_

| Aspect | Location |
|--------|----------|
| **Source** | docs/planning/phase4-n-k-orchestrator-binary-invariants.md §1 (I-5) |
| **Requirement** | Clock-injection seam + replay equivalence: the orchestrator depends on a Clock trait yielding now() and tick_stream(). No SystemTime::now() or tokio::time::Instant::now() reachable from the orchestrator core. Replaying the orchestrator core with a deterministic Clock and a recorded OrchestratorEvent stream against a frozen snapshot store + chaindb produces byte-identical final (LedgerState fingerprint, PraosChainDepState, ChainDb tip) across runs. |
| **Code** | crates/ade_runtime/src/clock.rs, crates/ade_runtime/src/orchestrator/core.rs |
| **Tests** | `crates/ade_runtime/tests/orchestrator_replay_equivalence.rs::replay_equivalence_under_deterministic_clock_holds`; `crates/ade_runtime/tests/orchestrator_replay_equivalence.rs::replay_corpus_is_present_and_decodable`; `crates/ade_runtime/src/orchestrator/core.rs::tests::step_two_runs_produce_byte_identical_effects`; `crates/ade_runtime/src/clock.rs::tests::deterministic_clock_is_pure`; `crates/ade_runtime/src/orchestrator/leadership_session.rs::tests::leadership_session_slot_arithmetic_is_pure` |
| **CI** | `ci/ci_check_clock_seam.sh`; `ci/ci_check_orchestrator_core_purity.sh` |

#### `DC-NODE-04` — _enforced_

| Aspect | Location |
|--------|----------|
| **Source** | docs/planning/phase4-n-k-orchestrator-binary-invariants.md §1 (I-6, I-7) |
| **Requirement** | Authority-fatal halt + shutdown-resume identity: authoritative errors (chain_write failure on a committed rollback, SnapshotDecodeError::UnknownVersion or FingerprintMismatch during bootstrap) halt the binary deterministically with a non-zero exit code — no silent retry, no fallback decode. Clean shutdown drains the admit/write/snapshot pipeline to a quiescent point (bounded — no waiting indefinitely for peer sessions) and writes a final snapshot via the persistent writer; restarting against the same (chaindb, snapshot store) produces a byte-identical initial (LedgerState, PraosChainDepState, ChainDb tip). Schema-version migration (snapshot v1 -> v2 upgrade tooling) is the snapshot-format lifecycle concern of DC-STORE-09, NOT a shutdown-semantics concern of this rule. This rule does NOT carry that open_obligation. |
| **Code** | crates/ade_node/src/node.rs, crates/ade_node/src/main.rs |
| **Tests** | `crates/ade_node/tests/shutdown_resume_identity.rs::shutdown_then_resume_produces_byte_identical_state`; `crates/ade_node/tests/shutdown_resume_identity.rs::shutdown_clean_exits_with_evidence`; `crates/ade_node/tests/shutdown_resume_identity.rs::cold_start_without_genesis_fails_with_generic_startup_code`; `crates/ade_node/tests/authority_fatal_decode.rs::binary_halts_on_authority_fatal_decode_error` |
| **CI** | `ci/ci_check_node_binary_uses_single_bootstrap.sh` |

#### `DC-NODE-05` — _enforced_

| Aspect | Location |
|--------|----------|
| **Source** | docs/planning/phase4-n-f-e-invariants.md |
| **Requirement** | Forge-slot discipline on the --mode node relay run-loop. A forge is attempted at most once per SlotNo and never for a slot <= the last forged slot (no past or duplicate forge). The current slot is derived ONLY through the clock seam: RED observes wall-clock (SystemClock), GREEN converts via millis_to_slot over the SystemStart anchor + EraSchedule slot length -- only SlotNo crosses the seam (no SystemTime/Instant/float past the RED observation boundary). The forge tick advances NO durable tip and admits/serves/gossips nothing -- it is subordinate to the sync spine, whose run_node_sync -> pump_block path remains the sole durable tip-advance authority; a forged block is a local self-accept artifact only. For a fixed recovered state, ordered block feed, injected clock tick schedule, and shutdown schedule, the forge-attempt sequence and forged block bytes are byte-identical across runs (replay-equivalent). Leadership eligibility is NOT decided in the loop or the GREEN planner (whose forge input is a content-blind Due\|NotDue) -- it stays in BLUE inside forge_one_from_recovered. Single-epoch this cluster: an unsupported slot fails closed / skips with a structured local outcome (cluster-scope containment, not permanent behavior). |
| **Code** | crates/ade_node/src/node_lifecycle.rs, crates/ade_node/src/run_loop_planner.rs |
| **Tests** | `plan_loop_step_forge_precedence_table_is_total`; `forge_slot_guard_none_is_due`; `forge_slot_guard_at_most_once_per_slot`; `forge_slot_guard_rejects_past_slot`; `relay_loop_forge_slot_derived_via_clock_seam`; `relay_loop_forge_tick_attempts_forge_advances_no_tip`; `relay_loop_without_producer_material_matches_nfd_relay`; `relay_loop_forge_two_runs_byte_identical`; `forge_tick_rotated_kes_period_skips_no_retroactive_sign`; `forge_tick_off_epoch_slot_fails_closed_local` |
| **CI** | `ci/ci_check_loop_planner_closed.sh`; `ci/ci_check_node_run_loop_containment.sh`; `ci/ci_check_genesis_consistency_fixture_present.sh` |

#### `DC-NODE-06` — _enforced_

| Aspect | Location |
|--------|----------|
| **Source** | docs/planning/phase4-n-f-g-invariants.md; docs/clusters/completed/PHASE4-N-U/cluster.md (S3 supersession) |
| **Requirement** | Self-accept -> serve handoff on the --mode node relay spine (sibling serve task, shape B). Only a BLUE self-accepted forged artifact may enter the sibling served-chain serve task: the handoff carries a typed, constructor-fenced artifact whose ONLY provenance is a ForgeSucceeded outcome (CN-FORGE-01 emits ForgeSucceeded only when BLUE self_accept accepts the forged block against its pre-forge base). The serve task MUST NOT accept raw forged bytes, a failed forge output (ForgeNotLeader / ForgeFailed), a self-declared acceptance flag, or a peer-verdict substitute. Served-chain mutation happens ONLY in the sibling serve authority via the single ServedChainHandle::push_atomic authority; the block-fetch response preserves the self-accepted forged block bytes as the payload and applies the single CN-WIRE-08 tag-24 envelope authority (no parallel serializer). The relay-loop body performs NO serve / admit / gossip / block-fetch / durable-tip mutation -- the handoff from the relay loop to the sibling serve task is a typed channel send of a constructor-fenced self-accepted artifact, not served-chain mutation and not block-fetch serving, so the relay-loop containment gate (ci_check_node_run_loop_containment.sh) stays SEMANTICALLY UNCHANGED (this cluster may ADD a served-chain handoff gate but MUST NOT relax containment). Peer acceptance is proven ONLY by the peer's validation log through ba02_evidence::correlate, never by Ade's self-accept / ForgeSucceeded / any wire-success signal (RO-LIVE-06). PHASE4-N-U S3 (DC-NODE-13) SUPERSEDES the SelfAcceptedHandoff -> ServedChainHandle::push_atomic accumulator MECHANISM on the --mode node spine with serve-as-projection: the node spine no longer feeds an in-memory accumulator; it serves a READ-ONLY PROJECTION of the durable ChainDb (ServedChainSource::DurableChainDb), whose bytes entered ONLY through the validated durable admit (pump_block, DC-NODE-12) + the trusted seed (bootstrap_initial_state). DC-NODE-06's DEEPER invariant -- only validated/admitted bytes may be served on the node spine -- is PRESERVED and STRENGTHENED (durable-provenance per the CN-CONS-07 restatement; now survives restart). The handoff-fence gate is REPOINTED to fence the evolved invariant (the node-spine serve sources ONLY the durable ChainDb projection; no retired non-durable serve ingress -- no push_atomic / served_chain_admit / ServedChainHandle / SelfAcceptedHandoff channel). The --mode produce serve path (CN-PROD-04) is a SEPARATE authority and legitimately retains the SelfAcceptedHandoff carrier + ServedChainHandle. |
| **Code** | crates/ade_node/src/node_lifecycle.rs (run_node_serve_task -> ServedChainSource::DurableChainDb over Arc<dyn ChainDb>; the G-B push sibling + serve_gate_admits + handoff channel retired), crates/ade_node/src/node_sync.rs, crates/ade_runtime/src/network/served_chain_projection.rs (ChainDbServedSource -- the durable-provenance serve source), crates/ade_runtime/src/network/serve_dispatch.rs (ServedChainSource). HISTORICAL (G-B/G-C, superseded by N-U S3): crates/ade_runtime/src/producer/served_chain_handle.rs (ServedChainHandle -- now --mode produce only), crates/ade_ledger/src/producer/served_chain.rs |
| **Tests** | `handoff_carrier_constructs_only_from_self_accepted_forge`; `forge_surfaces_accepted_block_only_on_self_accept`; `handoff_carrier_has_no_raw_bytes_constructor`; `serve_ingress_type_rejects_failed_forge_outcome`; `sibling_serve_admits_via_push_atomic_only`; `serve_sibling_admission_replay_byte_identical`; `serve_sibling_push_atomic_fed_only_by_into_accepted`; `relay_loop_containment_semantics_after_serve_sibling_retired`; `block_fetch_payload_is_self_accepted_bytes`; `block_fetch_tag24_round_trips_to_self_accept_input` … (+3 more) |
| **CI** | `ci/ci_check_served_chain_handoff_fence.sh`; `ci/ci_check_node_run_loop_containment.sh`; `ci/ci_check_served_chain_projection.sh` |

#### `DC-NODE-07` — _enforced_

| Aspect | Location |
|--------|----------|
| **Source** | docs/planning/phase4-n-f-g-h-invariants.md |
| **Requirement** | Node-spine live serve-to-peer. --mode node serves real peers ONLY from the G-B self-accepted ServedChainView (the read side of the single ServedChainHandle fed by the sibling self-accept->serve push task, DC-NODE-06), OUTSIDE run_relay_loop, THROUGH the existing ChainSync + BlockFetch serve reducers, with NO second serve authority or serializer. A sibling listener + serve-dispatch task (spawned outside the relay loop, mirroring the G-B push task) reuses the existing N2N serve machinery (run_n2n_listener + dispatch_server_frame_event_to_outbound) and the BLUE producer_chain_sync_serve + producer_block_fetch_serve (+ producer_chain_sync_advance_tip) -- serving BOTH the ChainSync header advertisement (a follower discovers Ade's served tip on each ServedChainView update, DC-CONS-18) AND the BlockFetch body (the follower fetches the served block by hash, DC-CONS-17), under the closed server-agency reply surface (CN-PROTO-06) and the deterministic/total session reducers (DC-PROTO-07, DC-PROTO-08). There is NO second ServedChain authority, NO parallel tag-24 serializer (the single CN-WIRE-08 envelope authority), and NO serve / push_atomic / served-chain mutation inside run_relay_loop (the containment gate ci_check_node_run_loop_containment.sh stays SEMANTICALLY UNCHANGED, CN-NODE-02). --mode node reuses the EXISTING --listen flag (no new --mode node argv flag; the S1 path-fidelity fence ci_check_node_path_fidelity.sh stays green) and is NOT switched to --mode produce. Wiring the serve is NOT a peer-acceptance claim: acceptance is proven ONLY by the peer's validation log through ba02_evidence::correlate (RO-LIVE-06), and RO-LIVE-01 / RO-LIVE-06 do NOT flip at this cluster's implementation close. |
| **Code** | crates/ade_node/src/node_lifecycle.rs (retain the ServedChainView + spawn the sibling listener/serve-dispatch task outside run_relay_loop -- the only new code), crates/ade_node/src/produce_mode.rs (run_n2n_listener + dispatch_server_frame_event_to_outbound + new_per_peer_outbound -- shared serve adapter to be EXTRACTED to a shared module, not duplicated; OQ1), crates/ade_network/src/chain_sync/server.rs (producer_chain_sync_serve + producer_chain_sync_advance_tip -- reused BLUE), crates/ade_network/src/block_fetch/server.rs (producer_block_fetch_serve -- reused BLUE), crates/ade_runtime/src/producer/served_chain_handle.rs (ServedChainHandle/ServedChainView -- reused GREEN) |
| **Tests** | `served_view_projects_durable_chain`; `node_serve_start_failure_is_surfaced_not_silent`; `n2n_supported_for_magic_produces_configured_magic`; `node_c1_serve_live` |
| **CI** | `ci/ci_check_single_serve_dispatch_authority.sh`; `ci/ci_check_serve_listener_magic_aware.sh` |

#### `DC-NODE-08` — _enforced_

| Aspect | Location |
|--------|----------|
| **Source** | docs/planning/phase4-n-f-g-j-genesis-successor-prevhash-invariants.md |
| **Requirement** | --mode node MAY forge the genesis-successor (FIRST) block from the recovered authoritative base when ChainDb::tip() AND the recovered tip (recovered.tip) are BOTH None, but ONLY when ALL hold: (a) the explicitly WarmStart-recovered/imported seed-epoch lineage is present (the recovered SeedEpochConsensusInputs + anchor lineage), never an unanchored / from-genesis-file-constructed or stale base; (b) ForgeIntent::On with complete operator key material; (c) the feed state is forge-eligible under the CN-NODE-04 closed split (no_block_available \| clean_empty), never an ineligible / ambiguous (unknown_disconnected) or error state; (d) the slot/epoch/KES/leader guards pass (DC-EPOCH-03 single-epoch containment + the BLUE leader check + KES-period/opcert); and (e) the forged first block carries PrevHash::Genesis (CBOR null per CN-WIRE-09) and flows through self_accept -> SelfAcceptedHandoff (DC-NODE-06) -> ServedChainView (DC-NODE-07). The recovered lineage gates PERMISSION to forge from the genesis-successor position; it is NOT the source of the prev_hash bytes, which are structurally null. The first-block reachability fires EXACTLY ONCE; once a durable tip exists, block_number > 0 takes the normal selected_tip path with PrevHash::Block. The durable tip advances ONLY through the accepted path, never from forge scheduling alone; no forge from raw / unanchored genesis; the forge base is the recovered surface only (CN-CINPUT-03 / DC-CINPUT-02b); no RO-LIVE-01/06 flip. The eligibility signal is general (forge-configured + valid recovered base), never a private-only / C1-only flag. |
| **Code** | crates/ade_node/src/node_sync.rs (forge_header_position -- GREEN single cold-start convention: None => block 0 + PrevHash::Genesis, Some => last_block_no+1 + Block, malformed-height edge fails closed; forge_one_from_recovered(selected_tip: Option<&ChainTip>) routes the cold-start ctx into the SAME run_real_forge S3 proved; NodeForgeError::RecoveredTipMissingBlockNo); crates/ade_node/src/node_lifecycle.rs (may_cold_start_forge -- GREEN cold-start permission: no tip + recovered lineage + forge-eligible feed; the LoopStep::ForgeTick arm passes selected_tip.as_ref() and gates the both-None genesis forge); crates/ade_node/src/forge_intent.rs (ForgeIntent::On precondition) |
| **Tests** | `forge_one_from_recovered_cold_start_is_block_zero_genesis`; `forge_one_from_recovered_with_tip_is_block_n_plus_one_block_prev`; `forge_header_position_some_tip_without_block_no_fails_closed`; `cold_start_block_number_is_zero_single_convention`; `node_spine_cold_start_forges_genesis_block_zero`; `cold_start_gate_allows_genesis_when_eligible_and_recovered`; `node_spine_cold_start_ineligible_feed_does_not_forge`; `cold_start_gate_blocks_without_recovered_lineage`; `cold_start_gate_inactive_when_tip_present` |
| **CI** | `ci/ci_check_genesis_successor_reachability.sh` |

#### `DC-NODE-09` — _enforced_

| Aspect | Location |
|--------|----------|
| **Source** | docs/clusters/PHASE4-N-F-G-K/cluster.md |
| **Requirement** | Once --mode node has spawned a --listen serve task (run_node_serve_task) over a ServedChainView, the end of the upstream feed (the relay loop returning -- e.g. a clean feed-end HaltCleanly with the operator shutdown watch still false) alone MUST NOT terminate that serve task. The serve listener's lifetime is owned by the node lifecycle owner: it terminates ONLY on (a) explicit node shutdown (the operator shutdown watch), (b) a fatal serve error (a post-bind accept fault), or (c) lifecycle-owner cancellation. The serve task stays read-only over ServedChainView (fed only by forge -> self_accept -> SelfAcceptedHandoff -> push_atomic, DC-NODE-06 / DC-NODE-07); it holds no ChainDb / WAL / forge handle, so extending its lifetime grants AVAILABILITY, not authority -- a peer that retries after the feed ended can still BlockFetch the already-self-accepted block. The process-termination guarantee is PRESERVED (moved from the feed-end stop to the lifecycle owner, never removed): operator shutdown ends BOTH the relay loop and the serve task. No serve of bytes outside ServedChainView; no durable tip advance; no peer-block admission; no RO-LIVE flip; no unbounded never-terminating serve. |
| **Code** | crates/ade_node/src/node_lifecycle.rs (run_node_lifecycle_inner On-arm: the --listen serve task is spawned with shutdown.clone() -- the operator shutdown watch -- NOT a dedicated feed-end stop channel; the post-run_relay_loop node_serve_stop.send(true) flip is REMOVED; node_serve_handle is awaited and ends only on shutdown / fatal serve error. run_node_serve_task -- the serve loop -- is UNCHANGED: it breaks on its shutdown watch, a fatal accept error, and events-channel close, and takes only (TcpListener, ServedChainView, network_magic, watch::Receiver<bool>)) |
| **Tests** | `serve_task_outlives_feed_end_and_serves_late_fetch`; `serve_task_terminates_on_shutdown_no_hang`; `served_view_projects_durable_chain`; `node_serve_start_failure_is_surfaced_not_silent` |
| **CI** | `ci/ci_check_node_serve_lifetime.sh` |

#### `DC-NODE-10` — _enforced_

| Aspect | Location |
|--------|----------|
| **Source** | docs/clusters/PHASE4-N-F-G-Q/cluster.md |
| **Requirement** | After the feed validation/admission advances the node spine (a block ingested -> state.receive evolved), the next forge MUST derive the successor header position -- block_no (and the chain_dep/ledger it self-accepts against) -- from the EVOLVED admitted node-spine state (state.receive: the evolved chain_dep + ledger), NOT the stale WarmStart baseline (recovered.chain_dep / recovered.ledger). The successor block_no = the evolved chain_dep.last_block_no + 1; the prev_hash = the durable selected tip's hash. RecoveredTipMissingBlockNo is reserved for a genuinely malformed recovered state and MUST NOT fire for a feed-advanced tip (the feed sets the evolved block_no). No guessed block_no, no unwrap_or(1), no synthetic numbering. The genesis-successor cold-start (BOTH tips None -> block 0 + PrevHash::Genesis) is UNCHANGED (DC-NODE-08). The seed-epoch PoolDistr + eta0 (DC-CINPUT-02b / DC-CINPUT-03) are per-epoch and unchanged -- valid for the in-epoch successor; cross-epoch is off-epoch fail-closed (DC-EPOCH-03). NARROW: selects the evolved admitted chain state for the forge; no other forge-semantics change, no durable-recovery / WAL change (the ChainBreak-on-restart is a SEPARATE N-U durability concern). |
| **Code** | crates/ade_node/src/node_lifecycle.rs (the relay-loop forge call threads the evolved state.receive.chain_dep + state.receive.ledger into forge_one_from_recovered -- NOT the recovered baseline); crates/ade_node/src/node_sync.rs (forge_one_from_recovered(recovered, live_chain_dep, live_ledger, selected_tip, ...) -- forge_header_position + the self-accept ctx read live_chain_dep/live_ledger; recovered supplies only the seed-epoch PoolDistr + the off-epoch guard) |
| **Tests** | `forge_successor_reads_evolved_spine_block_no_not_stale_baseline_g_q`; `forge_one_from_recovered_cold_start_is_block_zero_genesis` |
| **CI** | `ci/ci_check_forge_successor_evolved_spine.sh` |

#### `DC-NODE-11` — _enforced_

| Aspect | Location |
|--------|----------|
| **Source** | docs/clusters/PHASE4-N-F-G-R/cluster.md; docs/clusters/PHASE4-N-U/S3-serve-as-durable-chain-projection.md |
| **Requirement** | Once --mode node has self-accepted and SERVED a genesis-successor block at block_no 0, it MUST NOT add/replace the served view (ServedChainView) with another block_no-0 block during the same recovered NO-TIP episode (durable ChainDb tip + recovered tip both None). The node-level serve gate admits a self-accepted forge handoff to the ServedChainView ONLY when its block_no STRICTLY EXCEEDS the highest already-served block_no (serve_gate_admits) -- so the FIRST block 0 wins the served view and the hermetic forge's subsequent block-0 re-forges (DC-NODE-05, no own-tip advance) are NOT re-served; a follower then sees a STABLE block 0 to fetch + adopt. NARROW: no durable own-tip advance (the own-tip-adoption path is a separate cluster); no forged block 1+ claim; no synthetic numbering; the served block is still self-accepted (no bypass of self_accept, no serve of unvalidated bytes); the forge + served_chain_admit + durable tip are UNCHANGED (DC-NODE-05 intact). PHASE4-N-U (DC-NODE-13) SUPERSEDES the serve_gate_admits MECHANISM with serve-as-projection: own-forged blocks are now durably admitted (DC-NODE-12) and the durable chain is extend-only (DC-CONS-23), so it holds exactly one block 0 by construction; the served view PROJECTS the durable ChainDb (ChainDbServedSource), serving that stable, coherent chain WITHOUT a monotone gate. The invariant (a follower sees a STABLE block 0, no block-0-replaces-block-0 churn) is PRESERVED and strengthened -- it now also survives restart (the durable ChainDb is recovered by T-REC-05; the accumulator was not). |
| **Code** | crates/ade_runtime/src/network/served_chain_projection.rs (ChainDbServedSource -- PHASE4-N-U S3: serve-as-projection of the durable ChainDb provides the stable/coherent served chain; the durable chain is extend-only so it holds exactly one block 0); crates/ade_node/src/node_lifecycle.rs (run_node_serve_task serves the durable projection; serve_gate_admits RETIRED); crates/ade_ledger/src/receive/{reducer,admitted}.rs + block_validity (extend-only durable admit -- DC-CONS-23 -- rejects a re-mint block 0). HISTORICAL (PHASE4-N-F-G-R, superseded): node_lifecycle serve_gate_admits monotone-block_no gate over the ServedChainView accumulator + crates/ade_ledger/src/producer/served_chain.rs ServedChainSnapshot |
| **Tests** | `served_view_projects_durable_chain`; `served_view_retires_accumulator` |
| **CI** | `ci/ci_check_served_chain_projection.sh` |

#### `DC-NODE-12` — _enforced_

| Aspect | Location |
|--------|----------|
| **Source** | docs/planning/phase4-n-u-forged-block-durability-invariants.md |
| **Requirement** | Own-forged durable admit chokepoint. A self-accepted forged block may become part of the durable chain ONLY by being submitted to the same durable admit chokepoint as received blocks (run_node_sync -> pump_block / forward_sync AdmitPlan::durable): StoreBlockBytes -> AppendWal -> AdvanceTip, durable-before-tip, behind the BLUE admit authority (decode -> validate_and_apply_header -> block_validity, extend-only). The forge has NO second tip-advance path and performs NO direct tip mutation -- it submits a self-accepted artifact as an admit INPUT. The bytes admitted durably (StoreBlockBytes + WAL) are byte-identical to the bytes self_accept validated -- no re-encode / reserialize / reconstruct between self_accept and durable admit; the served projection must expose those same bytes when serving the durable block (I-10). SUPERSEDES the DC-NODE-05 containment consequence "a forged block is a local self-accept artifact only" while PRESERVING DC-NODE-05's deeper invariant: the forge tick advances no durable tip DIRECTLY, and pump_block remains the sole durable tip-advance authority. |
| **Code** | crates/ade_node/src/node_sync.rs (admit_forged_block_durably -- the fenced driver -> pump_block); crates/ade_node/src/node_lifecycle.rs (ForgeTick arm admits the self-accepted handoff via the driver); crates/ade_runtime/src/forward_sync/{reducer,pump}.rs (pump_block / AdmitPlan::durable -- reused) |
| **Tests** | `forge_tick_durable_admit_advances_tip`; `forge_successor_builds_block_1_from_durable_tip`; `forged_admit_bytes_byte_identical_to_self_accept` |
| **CI** | `ci/ci_check_forged_durable_admit_via_pump.sh`; `ci/ci_check_node_run_loop_containment.sh` |

#### `DC-NODE-13` — _enforced_

| Aspect | Location |
|--------|----------|
| **Source** | docs/planning/phase4-n-u-forged-block-durability-invariants.md; docs/clusters/PHASE4-N-U/S3-serve-as-durable-chain-projection.md |
| **Requirement** | Served view is a durable-chain projection. The ChainView served to followers (ChainSync header advertisement + BlockFetch body) is a deterministic PROJECTION of the durable adopted chain -- including any feed-ingested predecessor -- not an independent accumulator. Once own-forged blocks are durably admitted (DC-NODE-12), the served view follows the durable chain so a follower can fetch coherent history (the durable chain says A -> B; the served view serves A and B, never B without A). SUPERSEDES the PHASE4-N-F-G-R monotone serve-gate workaround (which gated an accumulator) with serve-as-projection. |
| **Code** | crates/ade_runtime/src/network/served_chain_projection.rs (ChainDbServedSource — RED read-only projection adapter implementing ServedHeaderLookup + ServedRangeLookup over &dyn ChainDb: next_after/intersect/tip/range_bytes read the durable ChainDb via iter_from_slot/get_block_by_hash/tip, decode_block for block_no/era, block_header_bytes for the header, stored.bytes served verbatim); crates/ade_runtime/src/network/serve_dispatch.rs (ServedChainSource enum {Snapshot\|DurableChainDb}; the single dispatch authority reads either source — DC-NODE-07 preserved); crates/ade_node/src/node_lifecycle.rs (run_node_serve_task dispatches with ServedChainSource::DurableChainDb over Arc<PersistentChainDb>; the G-R push sibling + serve_gate_admits retired); crates/ade_ledger/src/block_validity/header_input.rs (block_header_bytes — the DC-CONS-18 header authority exposed for raw durable bytes; accepted_block_header_bytes delegates) |
| **Tests** | `served_view_projects_durable_chain`; `follower_fetches_coherent_history_incl_ingested_predecessor`; `served_view_retires_accumulator` |
| **CI** | `ci/ci_check_served_chain_projection.sh` |

#### `DC-NODE-14` — _enforced_

| Aspect | Location |
|--------|----------|
| **Source** | docs/clusters/PHASE4-N-AE/cluster.md; docs/planning/phase4-n-ae-slice-a-invariants.md; docs/planning/c2-local-discovered-gaps.md |
| **Requirement** | Every claimed forge parent must be servable or peer-intersectable in the durable served lineage. A --mode node forge may only build on a parent a Haskell peer can FindIntersect: the followed peer tip (a durably-stored StoredBlock written by pump_block, AE.A) or a recovered anchor made intersectable (AE.B). The served chain must expose that parent as an intersect point from which the peer rolls forward onto the forged successor; the recovered snapshot anchor is never served as a chain head a peer cannot intersect. PARTIAL after AE.A (followed-tip lineage clause enforced); ENFORCED after AE.B (recovered-anchor clause). |
| **Code** | crates/ade_runtime/src/network/served_chain_projection.rs (ChainDbServedSource::intersect/next_after/tip over the durable ChainDb -- read-only, reused); crates/ade_node/src/node_lifecycle.rs (the forge-on-followed-tip gate gives the projection an intersectable parent) |
| **Tests** | `served_chain_intersects_at_followed_tip_and_rolls_to_forged`; `recovered_anchor_is_not_peer_intersectable`; `forged_successor_on_recovered_anchor_is_not_peer_adoptable`; `recover_follow_serve_forged_parent_intersectable` |
| **CI** | `ci/ci_check_forge_followed_tip_admission.sh`; `ci/ci_check_recovered_anchor_intersectable.sh` |

#### `DC-NODE-15` — _enforced_

| Aspect | Location |
|--------|----------|
| **Source** | docs/clusters/PHASE4-N-AE/cluster.md; docs/clusters/PHASE4-N-AE/slices/AE.A.md; docs/planning/phase4-n-ae-slice-a-invariants.md |
| **Requirement** | Forge admissibility requires the durable servable tip to equal the followed peer tip. A --mode node forge is admissible ONLY when durable_servable_tip == followed_peer_tip (hash AND block_no); otherwise it fails closed with a typed structured refusal ForgeRefused::NotCaughtUp { local_servable_tip, followed_peer_tip, reason } -- no forge, no state transition, tip unchanged (distinct from a forge Failed). The recovered anchor (recovered.tip) is NEVER a forge base. The followed-peer-tip signal is a forge-ADMISSIBILITY input only: it may PREVENT a forge but may not select, replace, reorder, or prefer chains (it never reaches select_best_chain / chain_selector / fork_choice). |
| **Code** | crates/ade_node/src/node_sync.rs (forge_followed_tip_admission GREEN classifier + ForgeFollowedTipAdmission/NotCaughtUpReason + ForgeRefused/NodeForgeOutcome + FollowedPeerTipSignal); crates/ade_node/src/node_lifecycle.rs (run_relay_loop_with_sched ForgeTick arm: recovered.tip forge-base fallback removed, gate called before the single fenced forge_one_from_recovered, typed ForgeRefused::NotCaughtUp recorded into ForgeActivation.last_forge_refused) |
| **Tests** | `forge_refused_not_caught_up`; `forge_base_falls_back_to_snapshot_anchor`; `forge_on_followed_tip_proceeds_with_parent_byte_equal` |
| **CI** | `ci/ci_check_forge_followed_tip_admission.sh` |

#### `DC-NODE-16` — _enforced_

| Aspect | Location |
|--------|----------|
| **Source** | docs/clusters/PHASE4-N-AE/slices/AE.F.md; docs/planning/phase4-n-ae-f-echo-idempotency-invariants.md; docs/evidence/phase4-n-ae-ce-a5-relay-adoption.md |
| **Requirement** | Receive idempotency: a peer-delivered block already durably present byte-identically in the ChainDb (same slot, same hash) is an idempotent no-op at the durable-admit chokepoint (pump_block) -- no validation step, no WAL append, no tip change; the post-state is identical and replay-equivalent. A DIFFERENT block (different hash) at/before the last-applied slot is NOT short-circuited: it reaches the unchanged BLUE header authority and fails closed (SlotBeforeLastApplied / BlockNoOutOfOrder). The skip is gated on HASH equality vs the durable store, never slot alone; no skip-past, no fork-choice (DC-CONS-03 untouched). |
| **Code** | crates/ade_runtime/src/forward_sync/pump.rs (pump_block -- the hash-exact get_block_by_hash already-have gate, placed before the BLUE chokepoint reducer) |
| **Tests** | `pump_block_reannounced_block_is_idempotent_noop`; `pump_block_different_block_at_or_before_tip_still_fails_closed`; `run_node_sync_survives_reannounced_block_in_feed` |
| **CI** | `ci/ci_check_receive_idempotency.sh` |

#### `DC-NODE-17` — _declared_

| Aspect | Location |
|--------|----------|
| **Source** | docs/planning/sustained-single-producer-forge-invariants.md |
| **Requirement** | followed_peer_tip advances ONLY from a real observed peer ChainSync advertisement of the peer's selected tip, INCLUDING the self-adoption echo case where the advertised block is already durably held by Ade (the relay re-announcing Ade's own just-adopted block). The advance is a RED scheduling observation of the peer's real selection: it updates forge ADMISSIBILITY only (DC-NODE-15) and must NEVER mutate the durable tip / WAL / ledger (DC-NODE-16 idempotency preserved; replay-neutral) and NEVER reach chain selection / fork-choice (DC-CONS-03 stays the follow authority). A sole producer therefore recognizes catch-up to its own adopted block and forges the successor, sustaining a chain (N, N+1, ...) rather than stalling at one block. NOT a chain-selection rule; a RED observation rule for forge admissibility only. |
| **Code** | CANDIDATE (pending OQ-1): crates/ade_node/src/node_sync.rs (FollowedPeerTipSignal.observe + the wire-pump pump_lookahead/wait_ready which today observes only AdmissionPeerEvent::TipUpdate) |
| **Tests** | — _(declared — not yet enforced)_ |
| **CI** | — _(declared — not yet enforced)_ |

#### `DC-NODE-18` — _enforced_

| Aspect | Location |
|--------|----------|
| **Source** | docs/planning/single-producer-extend-own-spine-invariants.md |
| **Requirement** | Successor extension after an explicit adoption certificate (single-producer, single successor). After initial peer catch-up against a real peer tip (DC-NODE-15) AND explicit relay-adoption evidence for Ade's first successor -- an operator/harness-supplied RED venue-adoption certificate naming the adopted own tip, matched by chain-point identity (hash + block_no), NEVER inferred from self-admit -- single-producer Ade may forge the next successor on its OWN durable adopted spine WITHOUT requiring a relay echo (re-announce) of the adopted block back over the follow link. A successor is adoptable by induction (it extends an already-adopted parent; the relay is a pure follower of Ade's chain). Valid ONLY while the venue is explicitly single-producer: the relay is non-producing, Ade is the sole block producer, and no competing candidate chain is admitted. A gate-APPLICABILITY refinement, NOT a weakening of fork-choice; multi-producer behaviour stays out of scope and belongs to chain-selection authority (DC-CONS-03); the followed-peer-tip signal still may not select/replace/reorder/prefer chains. Fails closed if the venue is not explicitly declared single-producer. SCOPE BOUNDARY (live-proven core only): this rule asserts ONLY the successor-extension-after-adoption-certificate authority. It does NOT assert sustained production past k, relay ImmutableDB settlement, follow-link liveness, or forge-loop continuation after a follow-link EOF -- those are adjacent liveness/loop-lifecycle obligations deferred to DC-NODE-19. |
| **Code** | crates/ade_node/src/node_sync.rs: ForgeMode enum (InitialCatchupRequired -> CaughtUpToPeerTip -> FirstOwnBlockServed -> SingleProducerExtendOwnDurableSpine; no booleans) + VenueRole + VenueAdoptionCertificate + SingleProducerFenceReason + SingleProducerForgeDecision + single_producer_forge_decision + forge_mode_on_caughtup/_on_first_own_block_served/_on_extend + forge_mode_after_admit (advances ONLY on an actual admit, never on a not_leader tick) + ForgeRefused::SingleProducerFenceViolation. crates/ade_node/src/node_lifecycle.rs: the mode-aware ForgeTick gate in run_relay_loop_with_sched (behind VenueRole::SingleProducer; default Unknown == the verbatim prior DC-NODE-15 path) + read_adoption_cert + dc_node_15_refusal + ForgeActivation.{forge_mode,venue_role,adoption_cert_path} + declare_single_producer_venue. crates/ade_node/src/cli.rs: --single-producer-venue + --adoption-cert-path. |
| **Tests** | `forge_mode_transitions_are_total_and_deterministic`; `extend_own_spine_forges_on_durable_tip_without_followed_equality`; `single_producer_fence_fails_closed`; `extend_own_spine_two_runs_byte_identical`; `forge_mode_after_admit_only_advances_on_real_admit` |
| **CI** | `ci/ci_check_single_producer_extend_own_spine.sh` |

#### `DC-NODE-19` — _declared_

| Aspect | Location |
|--------|----------|
| **Source** | docs/planning/single-producer-loop-continuation-after-feed-eof-invariants.md |
| **Requirement** | Single-producer forge-loop continuation after follow-link EOF. In an explicitly declared single-producer venue (VenueRole::SingleProducer) that has ALREADY entered the DC-NODE-18 extend state (ForgeMode::SingleProducerExtendOwnDurableSpine, reached only via DC-NODE-15 catch-up + an explicit venue-adoption-certificate promotion), a LoopState::Ending caused SOLELY by structural feed EOF (the Ade->relay follow link closing/draining) MUST NOT by itself terminate the forge loop; the loop continues forging successors on its OWN certified durable spine (each admitted via pump_block, DC-NODE-12). The continuation is FENCED to the certified single-producer run and FAILS CLOSED (no continuation; verbatim HaltCleanly / typed refusal, never a silent forge) if ANY hold: (1) not VenueRole::SingleProducer; (2) not ForgeMode::SingleProducerExtendOwnDurableSpine; (3) operator shutdown requested; (4) existing forge-validity bounds fail (off-epoch DC-EPOCH-03 / beyond forecast horizon DC-CONS-09 / KES-period invalid); (5) a competing chain was observed before EOF; (6) relay-producing evidence exists; (7) the venue certificate is absent or malformed -- the DC-NODE-18 SingleProducerFenceReason set plus the venue/mode/cert fence. NO numeric "max blind forges" cap (an artificial operator policy, not a Cardano semantic invariant); the certified-run fence + the existing BLUE bounds + operator shutdown bound it. RELOCATES the loop's termination authority off feed-liveness onto explicit operator shutdown / fatal error (the same move DC-NODE-09 made for the serve-listener lifetime) while PRESERVING DC-NODE-05's deeper invariant: the forge advances NO durable tip directly and run_node_sync -> pump_block remains the SOLE durable tip-advance authority, and available feed work still drains via SyncOnce before any ForgeTick (produce subordinate to the sync spine). The default VenueRole::Unknown venue takes the verbatim prior HaltCleanly-on-feed-end path. A loop-lifecycle refinement, NOT a fork-choice change -- DC-CONS-03 stays the sole follow/fork authority; the continuation never selects/reorders/prefers chains. Only a clean structural feed EOF is continued; a LoopState::Ending representing a real shutdown or a fatal source failure exits via Err/fail-fast and is NEVER continued. SCOPE: continuation only (don't-die-on-EOF); follow-link keep-alive / reconnect (OQ-KA) is a separate non-blocking cousin. |
| **Code** | CANDIDATE (pending /cluster-plan): crates/ade_node/src/run_loop_planner.rs (plan_loop_step gains a 5th closed content-blind VenuePolicy input -- the 32-case total table; + a GREEN VenueRole/ForgeMode -> VenuePolicy projection); crates/ade_node/src/node_lifecycle.rs (run_relay_loop_with_sched threads the venue policy + restructures the Idle-under-feed-end wait so a dead feed does not starve the forge cadence; the shutdown watch stays the lifecycle authority; the certified-run continuation fence reuses the DC-NODE-18 SingleProducerFenceReason); crates/ade_node/src/node_sync.rs (the venue/mode/cert continuation fence reuses single_producer_forge_decision / SingleProducerFenceReason) |
| **Tests** | — _(declared — not yet enforced)_ |
| **CI** | — _(declared — not yet enforced)_ |

#### `DC-NODE-20` — _enforced_

| Aspect | Location |
|--------|----------|
| **Source** | docs/planning/local-selected-durable-chain-forge-base-invariants.md |
| **Requirement** | Local selected durable chain forge-base authority (rung-1 single-producer). In a declared rung-1 single-producer venue, after Ade self-admits a valid forged block through pump_block (DC-NODE-12) onto its local durable ChainDB spine, the next forge base is Ade's LOCAL SELECTED DURABLE TIP -- the head of its own admitted ChainDB spine (ChainDb::tip) -- NOT followed_peer_tip and NOT an operator adoption certificate. This RETIRES, as forge authority, the upstream durable_servable_tip == followed_peer_tip re-check (which fired NoTipAvailable on every tick after Ade self-admitted a block the non-producing relay does not re-announce) and the DC-NODE-18 cert-promotion-into-extend mechanism. The local-tip forge base engages ONLY while ALL hold; ANY failure FAILS CLOSED (no silent fallback to followed_peer_tip or the cert): (1) VenueRole::SingleProducer; (2) NO competing block has been observed on the canonical peer receive stream since initial catch-up / self-admit -- an OBSERVED-FEED fence, NOT fork-choice; if observed, fail closed, do NOT resolve (that is rung 2); (3) the relay is non-producing; (4) the block was admitted through pump_block (DC-NODE-05 stays the SOLE durable admit authority -- DC-NODE-20 only READS the tip pump_block produced; it advances no tip); (5) the ChainDB spine is contiguous and servable; (6) no fork-choice decision is required -- in rung 1 mechanically DERIVED from (2): no competing candidate observed => the local spine head is the degenerate selected tip; a competing candidate => fork-choice required => DC-NODE-20 disabled. The FirstOwnBlockServed cert-wait intermediate is FOLDED OUT of the authority path: the transition is DIRECT -- (CaughtUpToPeerTip + self-admit valid own block via pump_block + the fence) => SingleProducerExtendOwnDurableSpine{current_tip = ChainDb::tip}, with NO cert read. DC-NODE-15 remains the INITIAL catch-up gate (durable == followed before the first own-forge); DC-NODE-20 supersedes ONLY the repeated post-self-admit durable == followed re-check. Relay adoption remains an EVIDENCE obligation for the transcript/bounty proof (DC-NODE-21), NOT a forge-loop precondition. In rung 1 'selected' is DEGENERATE (no competing candidate => local ChainDB head = selected tip); rung 2 MUST replace this with real fork-choice (DC-CONS-03, untouched here). Preserves DC-NODE-19 (continue-past-EOF in the extend state, now ENTERED via local self-admit not the cert) and T-REC-03/05 (the local-tip forge base derives from the local durable spine alone -- removing the RED cert/timing from the authority path makes the post-self-admit forge MORE deterministic / replay-equivalent). SCOPE: rung-1 single-producer only; follow-link keep-alive (OQ-KA), real fork-choice / multi-producer (rung 2), and preprod are OUT of scope. |
| **Code** | CANDIDATE (pending /cluster-plan): crates/ade_node/src/node_lifecycle.rs (the proceed_to_forge gate -- replace the post-self-admit durable_servable_tip == followed_peer_tip re-check + the read_adoption_cert promotion with a local-selected-durable-tip authority derived from ChainDb::tip, fenced); crates/ade_node/src/node_sync.rs (the ForgeMode transition CaughtUpToPeerTip -> SingleProducerExtendOwnDurableSpine becomes DIRECT on self-admit, folding out the cert-gated FirstOwnBlockServed intermediate; the observed-feed competing-block fence). REUSES BLUE ChainDb::tip + pump_block (no new BLUE authority). |
| **Tests** | `caughtup_self_admit_enters_extend_directly_no_cert`; `forge_base_selected_transcript_witnesses_local_tip`; `local_spine_sustains_two_successors_no_cert`; `local_spine_two_runs_byte_identical` |
| **CI** | `ci/ci_check_local_durable_forge_base.sh`; `ci/ci_check_forge_followed_tip_admission.sh` |

#### `DC-NODE-21` — _enforced_

| Aspect | Location |
|--------|----------|
| **Source** | docs/planning/local-selected-durable-chain-forge-base-invariants.md |
| **Requirement** | Adoption certificate is rung-1 evidence-only, never forge authority. The file-based operator adoption certificate is a rung-1 RED EVIDENCE-ONLY shim. It MAY prove relay adoption for the transcript / bounty bundle, but MUST NEVER control forge-base selection (DC-NODE-20 derives the forge base from the local durable ChainDB tip) or any durable authority, and MUST NEVER appear in multi-producer, preprod, or production forge paths. It MUST be removed or replaced by node-local selected-chain / fork-choice authority (DC-CONS-03) before rung 2 / preprod. Hard removal boundary: the shim exists ONLY because Ade does not yet have full multi-producer fork-choice + peer-state lifecycle; it must not creep back into later slices as authority. Relay adoption is evidence for the bounty proof, never a forge-loop precondition. |
| **Code** | CANDIDATE (pending /cluster-plan): crates/ade_node/src/node_lifecycle.rs (read_adoption_cert demoted to an evidence-only transcript record, REMOVED from the forge-base / proceed_to_forge path); a CI gate asserting the cert is never a forge-base input and never present in multi-producer/preprod/production forge paths. |
| **Tests** | `caughtup_self_admit_enters_extend_directly_no_cert`; `forge_base_selected_transcript_witnesses_local_tip`; `local_spine_cert_file_absent_from_replay_surface` |
| **CI** | `ci/ci_check_cert_evidence_only.sh`; `ci/ci_check_node_path_fidelity.sh` |

#### `DC-NODE-22` — _enforced_

| Aspect | Location |
|--------|----------|
| **Source** | docs/planning/local-selected-durable-chain-forge-base-invariants.md |
| **Requirement** | Single-producer warm-start re-entry derives forge mode from the recovered local durable spine. In a declared rung-1 single-producer venue, if warm-start recovery yields a durable local ChainDB tip ABOVE the recovered bootstrap anchor (the recovered tip proves an own-forged continuation of Ade's own spine, not the bare imported anchor), forge mode MUST re-enter SingleProducerExtendOwnDurableSpine{current_tip = ChainDb::tip} under the DC-NODE-20 fence, WITHOUT requiring a fresh followed-peer catch-up. This is the warm-start analog of DC-NODE-20: on the clean live path, self-admitting an own block makes ChainDb::tip the forge base and enters the extend state; on warm-start, the recovered durable own spine ALREADY makes ChainDb::tip the forge base, so re-entry into the extend state is immediate. Without it, warm-start re-initializes forge_mode = InitialCatchupRequired, which needs a fresh follow-link catch-up; if the follow link EOFs first the node stalls in NoTipAvailable forever -- re-introducing, through restart, the exact follow-link dependency DC-NODE-20 retired. The re-entry engages ONLY while ALL hold; ANY failure FAILS CLOSED (fall back to InitialCatchupRequired, never silently forge): (1) VenueRole::SingleProducer; (2) after a SUCCESSFUL warm-start recovery (no ChainBreak / recovery error); (3) the recovered ChainDB spine is contiguous and servable; (4) the recovered tip is BEYOND the original recovered bootstrap anchor / proves own-forged continuation (at the bare anchor, use the normal catch-up flow); (5) NO competing peer block observed on the canonical receive stream (the DC-NODE-20 observed-feed fence); (6) the relay is a non-producing venue; (7) no cert is read (DC-NODE-21); (8) no fork-choice decision is required; (9) pump_block remains the SOLE durable admit authority (DC-NODE-05/12 -- this rule only sets the forge MODE / reads the recovered tip; it admits nothing). SCOPE: rung-1 single-producer ONLY. NOT a general restart rule for multi-producer or preprod -- those require real fork-choice + peer-state lifecycle (DC-CONS-03, untouched). |
| **Code** | CANDIDATE (pending S4b): crates/ade_node/src/node_lifecycle.rs -- the warm-start arm of run_node_lifecycle_inner: after warm_start_recovery + declare_single_producer_venue, derive forge_mode = SingleProducerExtendOwnDurableSpine{current_tip = recovered ChainDb::tip} when venue_role == SingleProducer AND the recovered tip is above the bootstrap anchor / own-spine threshold, fenced as in the statement. REUSES the DC-NODE-20 fence + ChainDb::tip + pump_block (no new BLUE authority). |
| **Tests** | `warm_start_reentry_requires_tip_above_recovered_anchor`; `warm_start_single_producer_re_enters_extend_and_forges` |
| **CI** | `ci/ci_check_warm_start_re_entry.sh` |

#### `DC-NODE-23` — _enforced_

| Aspect | Location |
|--------|----------|
| **Source** | docs/planning/phase4-n-ai-live-fork-choice-invariants.md (I-5) |
| **Requirement** | Shared receive-side fork-choice detector (rung-2). A peer-origin candidate that is NOT already known as part of Ade's admitted durable spine / own-served lineage -- including but not limited to a header that does not build on ChainDb::tip -- is classified ONCE, venue-blind, by a pure total predicate over (durable_tip, candidate_header_summary) -> ReceiveDisposition { AlreadyHave \| LinearExtend \| RefuseSingleProducer \| NeedsForkChoice }. A duplicate / already-known peer echo is AlreadyHave, never a competing candidate merely because it is not a fresh extension. Observes no venue, no wall-clock, no network state; the single classification point that both the SingleProducer fail-closed arm (DC-NODE-20) and the Participant fork-choice arm (DC-NODE-24) consume. It never selects, reorders, or prefers chains (that is select_best_chain / DC-CONS-03). |
| **Code** | CANDIDATE (pending /cluster-plan): crates/ade_node/src/node_sync.rs (a GREEN-by-function classifier over the durable tip + the arriving candidate header summary; sibling to forge_followed_tip_admission / single_producer_forge_decision). |
| **Tests** | `classify_already_have_when_in_spine`; `classify_linear_extend_on_exact_parent_and_block_no`; `classify_competing_on_nonmatching_parent`; `classify_competing_on_wrong_block_no`; `classify_competing_on_genesis_prev_hash`; `resolve_passthrough_already_have_and_linear_extend` |
| **CI** | `ci/ci_check_receive_detector_venue_split.sh` |

#### `DC-NODE-24` — _enforced_

| Aspect | Location |
|--------|----------|
| **Source** | docs/planning/phase4-n-ai-live-fork-choice-invariants.md (I-6) |
| **Requirement** | Venue-split fork-choice resolver (rung-2). The DC-NODE-23 detector's non-spine consequent is gated by venue and TOTAL over the closed venue set: VenueRole::SingleProducer => fail closed (the DC-NODE-20 rung-1 behavior, byte-unchanged -- never adopt a peer candidate); VenueRole::Participant => NeedsForkChoice => the existing ade_runtime::consensus::chain_selector orchestrator (process_stream_input -> BLUE select_best_chain, DC-CONS-03). Venue input is explicit and fail-safe (an undeclared / unknown venue takes the conservative SingleProducer refuse arm). In Participant mode the peer's VALIDATED header summary (post validate_and_apply_header) becomes a candidate; a raw followed_peer_tip signal MUST NOT reach select_best_chain. |
| **Code** | CANDIDATE (pending /cluster-plan): crates/ade_node/src/node_sync.rs (the venue->resolver projection) + crates/ade_node/src/node_lifecycle.rs (the receive arm dispatching to the chain_selector orchestrator on the Participant path; the SingleProducer path keeps the DC-NODE-20 fail-closed). |
| **Tests** | `resolve_singleproducer_competing_refuses`; `resolve_participant_competing_needs_fork_choice`; `resolve_participant_already_have_and_linear_extend_do_not_call_fork_choice`; `resolve_unknown_venue_fails_closed` |
| **CI** | `ci/ci_check_receive_detector_venue_split.sh` |

#### `DC-NODE-25` — _enforced_

| Aspect | Location |
|--------|----------|
| **Source** | docs/planning/phase4-n-ai-live-fork-choice-invariants.md (I-2, I-3, I-4, I-11) |
| **Requirement** | Live fork-choice durable application authority (rung-2). A ChainSelected / RolledBack outcome from the chain_selector orchestrator is applied to the durable stores ONLY via the existing enforced authorities: the lockstep receive reducer (DC-CONS-20 -- ChainDb + LedgerState + PraosChainDepState advanced / rolled-back as one structural transition) + materialize_rolled_back_state (CN-STORE-07) for the rollback target + pump_block (DC-NODE-05/12) for roll-forward. NO second apply path, NO second durable tip-advance path, NO second rollback-materialize path. A fork-choice win is provisional: the chain is durably adopted ONLY when its BODIES validate and apply through pump_block (no header-only tip advance). A TiebreakerLossKeepCurrent outcome makes no durable change. |
| **Code** | CANDIDATE (pending /cluster-plan): crates/ade_node/src/node_lifecycle.rs (the RED apply driver) over crates/ade_ledger/src/receive/reducer.rs + crates/ade_ledger/src/rollback/* + crates/ade_runtime/src/forward_sync/pump.rs. REUSES the enforced authorities; no new BLUE. |
| **Tests** | `apply_rolledback_rolls_back_and_appends_wal_record_after_commit`; `apply_chain_selected_invalid_body_fails_via_pump_no_advance`; `apply_chain_selected_without_block_bytes_fails_closed`; `participant_rollback_applies_durably`; `participant_block_with_no_durable_tip_pumps` |
| **CI** | `ci/ci_check_live_fork_choice_apply.sh`; `ci/ci_check_live_fork_choice_wiring.sh` |

#### `DC-NODE-26` — _enforced_

| Aspect | Location |
|--------|----------|
| **Source** | docs/planning/phase4-n-ai-live-fork-choice-invariants.md (I-7) |
| **Requirement** | Decision / durable reconciliation (rung-2). After any applied receive decision, the chain_selector orchestrator's selector.current_tip EQUALS the durable ChainDb::tip (and the orchestrator chain_dep equals the durable PraosChainDepState). The in-memory decision state never diverges from the persisted authority: the orchestrator decides, the durable lockstep path applies, and the two are reconciled every decision. No applied decision leaves the selector ahead of, behind, or forked from the durable spine. |
| **Code** | CANDIDATE (pending /cluster-plan): crates/ade_node/src/node_sync.rs / node_lifecycle.rs (the reconciliation projection: derive ChainSelectorState from the durable stores, or hold OrchestratorState in lockstep -- OQ-2). |
| **Tests** | `apply_reconciliation_mismatch_fails_fast`; `apply_rejected_makes_no_durable_change` |
| **CI** | `ci/ci_check_live_fork_choice_apply.sh` |

#### `DC-NODE-27` — _enforced_

| Aspect | Location |
|--------|----------|
| **Source** | docs/planning/phase4-n-ai-live-fork-choice-invariants.md (section 4, OQ-1) |
| **Requirement** | Rollback+reselection replay-equivalence (rung-2). The ordered live receive-event sequence (RollForward headers, RollBackward points, body deliveries) replayed against the same bootstrap anchor + durable log produces a BYTE-IDENTICAL durable tip + ledger fingerprint + PraosChainDepState -- INCLUDING any rollback+reselection. A live rollback is recorded durably (append-only, canonical bytes -- CN-WAL-01) such that replay re-invokes the SAME materialize / reducer authority (CN-STORE-07 / DC-CONS-20) at that point; the durable record is NOT a second rollback implementation. No implicit live-only rollback: a rollback that happened live MUST be reproducible on recovery (T-REC-03/05, DC-CONS-06/22). OQ-1 RESOLVED -> A (the version-gated WalEntry::RollBack marker re-invoking the existing rollback / materialize authority on replay; option B WAL-tail reconciliation rejected); enforced when the AI-S1 BLUE foundation + replay-equivalence land. |
| **Code** | CANDIDATE (pending /cluster-plan; OQ-1 RESOLVED -> A, see docs/planning/phase4-n-ai-oq1-rollback-durability-decision.md): a version-gated additive WalEntry::RollBack {to_point, reason, prior_tip, selected_tip} marker (crates/ade_ledger/src/wal/event.rs tag/encode/decode) whose replay arm in crates/ade_ledger/src/wal/replay.rs re-invokes the EXISTING materialize_rolled_back_state (CN-STORE-07) + lockstep commit_rollback (DC-CONS-20) and re-anchors the fingerprint chain to the materialized rolled-back fp. Append-only WAL preserved (CN-WAL-01); NOT a second rollback implementation. Option B (WAL-tail reconciliation) REJECTED. |
| **Tests** | `apply_rolledback_replays_byte_identical_recovers_forkpoint`; `replay_with_rollback_recovers_selected_not_abandoned`; `replay_with_rollback_two_runs_byte_identical`; `rollback_replay_reanchor_fp_equals_materialized_fp` |
| **CI** | `ci/ci_check_wal_rollback_replay_equiv.sh`; `ci/ci_check_live_fork_choice_apply.sh` |

#### `DC-NODE-28` — _enforced_

| Aspect | Location |
|--------|----------|
| **Source** | docs/planning/phase4-n-ai-live-fork-choice-invariants.md (I-12) |
| **Requirement** | No forge across unresolved re-selection (rung-2). Once a peer-origin candidate is classified NeedsForkChoice (DC-NODE-23) in a Participant venue, forging is DISABLED until the fork-choice outcome is either (a) durably applied and reconciled (DC-NODE-25/26) or (b) rejected with durable state unchanged. The forge base is NEVER selected from a stale pre-resolution ChainDb::tip while a decision is pending -- a producer tick that fires during a pending decision fails closed (typed ForgeRefused), never forges on the old local tip. This is a producer race fence DISTINCT from pump_block durable admit, rollback replay, or selector / durable reconciliation, so it carries its own evidence. Prevents stale local authority leaking into block production. Derived tier with a true-tier authority consequence. |
| **Code** | CANDIDATE (pending /cluster-plan): crates/ade_node/src/node_sync.rs (the forge gate -- extend single_producer_forge_decision / the Participant forge-base selection with a pending-resolution fence) + node_lifecycle.rs (the ForgeTick arm respects the pending state). |
| **Tests** | `pending_reselection_forge_refusal_gate`; `participant_rollback_beyond_k_fails_closed_clears_pending`; `singleproducer_rollback_refused_by_run_node_sync` |
| **CI** | `ci/ci_check_live_fork_choice_wiring.sh`; `ci/ci_check_participant_venue_inert.sh` |

#### `DC-NODE-29` — _enforced_

| Aspect | Location |
|--------|----------|
| **Source** | docs/clusters/PHASE4-N-AI/S6-rollback-target-slot-hash-binding.md (H-1 remediation) |
| **Requirement** | Live rollback target canonical binding (rung-2; AI-S6 H-1 remediation). For a peer RollBackward(point) on the live Participant path, the rollback target MUST be resolved against the durable ChainDb and use the stored chain point (stored slot + hash) as the SOLE authority. The peer-supplied slot MUST equal the stored slot for that hash; on any mismatch (or unknown hash, or Origin) the path fails closed with a typed error BEFORE commit_rollback, BEFORE WalEntry::RollBack, BEFORE any ChainDb / LedgerState / PraosChainDepState mutation. No rollback target may be built from mixed peer/local authority (peer-supplied slot + locally-verified hash). Reconciliation (DC-NODE-26) remains the post-apply backstop but is NOT the only defense. |
| **Code** | crates/ade_node/src/node_lifecycle.rs (run_participant_sync RollBack arm) over crates/ade_runtime/src/chaindb get_block_by_hash. RED shell binding; reuses the enforced rollback authorities; no new BLUE. |
| **Tests** | `rollback_slot_hash_mismatch_fails_before_mutation`; `participant_rollback_applies_durably`; `participant_rollback_to_unknown_point_fails_closed` |
| **CI** | `ci/ci_check_rollback_target_canonical_binding.sh` |

#### `DC-NODE-30` — _enforced_

| Aspect | Location |
|--------|----------|
| **Source** | docs/planning/phase4-n-aj-participant-convergence-evidence-invariants.md §1 (I-AJ-1) + §7 (D-1/D-2/D-3) |
| **Requirement** | Participant-path convergence evidence emission (PHASE4-N-AJ). The live `--mode node --participant-venue` rollback-follow path emits the existing closed AgreementVerdict vocabulary to a dedicated `--convergence-evidence-path` JSONL as a deterministic GREEN side-output of already-authoritative outcomes: - BlockReceived for EACH peer block considered by the receive path (before drop/admit/refuse), - BlockAdmitted per pump_block admit, - AgreementVerdict = verdict::derive(outcome, observed_peer_tip), each carrying consensus_inputs_fingerprint_hex. MUST NOT become authority: never gates admission, never triggers or parameterizes a rollback, never influences fork-choice, never mutates the durable chain. pump_block stays the sole roll-forward admit; apply_chain_event the sole rollback authority; classify_receive unchanged. Emit-only on Diverged. An evidence write failure is non-fatal to authority (the node continues consensus operation; the sink is distinct from the authoritative WAL), but the convergence-evidence transcript is then marked incomplete/unusable for CE-AI-6 -- it MUST NOT silently produce a partial transcript that later passes the gate. No path supplied => no file and node behavior byte-unchanged. No new evidence enum; no BLUE change. |
| **Code** | crates/ade_node/src/node_lifecycle.rs (run_participant_sync emission), crates/ade_node/src/admission/verdict.rs (derive, reused), crates/ade_node/src/admission_log/ (vocabulary/writer, reused against the --convergence-evidence-path sink), crates/ade_node/src/cli.rs (--convergence-evidence-path) |
| **Tests** | `ade_node::convergence_evidence::tests::convergence_evidence_absent_path_emits_no_file`; `ade_node::convergence_evidence::tests::convergence_evidence_writer_emits_closed_vocabulary`; `ade_node::convergence_evidence::tests::convergence_evidence_write_failure_poisons_and_is_surfaced`; `ade_node::convergence_evidence::tests::convergence_evidence_context_marks_incomplete_on_write_failure`; `participant_cold_start_admit_emits_received_admitted_agreed`; `participant_block_received_does_not_imply_admission`; `participant_convergence_evidence_replay_byte_identical` |
| **CI** | `ci/ci_check_convergence_evidence_emit_only.sh`; `ci/ci_check_convergence_evidence_vocabulary_closed.sh`; `ci/ci_check_convergence_evidence_schema.sh` |

#### `DC-NODE-31` — _enforced_

| Aspect | Location |
|--------|----------|
| **Source** | docs/planning/phase4-n-ak-recovered-anchor-tip-invariants.md (AK-INV-1) + docs/clusters/PHASE4-N-AK/cluster.md |
| **Requirement** | Recovered-anchor live-follow start authority (PHASE4-N-AK). After recovery from a non-Origin bootstrap anchor, the recovered store PERSISTS the bootstrap anchor point (slot, hash) as replayable recovery provenance, bound to the recovered anchor fingerprint. On warm-start, BootstrapState resolves the live-follow start tip from that persisted anchor point whenever ChainDb has no servable post-anchor block; resolution order = servable ChainDb tip -> persisted recovered anchor point (non-Origin + provenance-bound) -> Origin/None only if the recovered store is truly Origin/cold-start. A non-Origin recovered store whose anchor-point record is missing / malformed / fingerprint-mismatched FAILS CLOSED before live follow starts. Same recovered store + same WAL => same anchor point => same BootstrapState.tip => same FindIntersect start (replay-equivalent; extends T-REC-05 to the recovered tip surface). The persisted anchor point is the durable restart authority -- NOT CLI re-supply (CLI seed-point is first-run input only). Does not change ChainDb::tip() semantics and does not synthesize a servable block. AI-S4a RollBackward(Origin) fail-close unchanged. The wire-pump consumer (spawn_live_wire_pump_source) is UNCHANGED. |
| **Code** | crates/ade_ledger/src/recovered_anchor_point.rs (RecoveredAnchorPoint type + sole canonical CBOR codec), crates/ade_runtime/src/bootstrap.rs (resolve_live_follow_start -- private tip resolver; BootstrapInputs.recovered_anchor canonical input; BootstrapState live-follow start tip), crates/ade_runtime/src/recovered_anchor.rs (load_recovered_anchor_point -- load + fail-closed verify, kept out of bootstrap.rs to preserve CN-NODE-01 single-pub-fn), crates/ade_runtime/src/seed_epoch_lineage.rs (persist_seed_epoch_consensus_inputs -- writes the anchor-point record at seed/recover), crates/ade_runtime/src/chaindb/{mod,in_memory,persistent}.rs (put/get_recovered_anchor_point anchor-fp-keyed store surface), crates/ade_node/src/node_lifecycle.rs (warm_start_recovery loads + threads recovered_anchor; wire_pump_start_point / spawn_live_wire_pump_source -- UNCHANGED consumer of the resolved tip), crates/ade_runtime/src/mithril_bootstrap.rs (BootstrapAnchor.seed_point -- canonical input), crates/ade_runtime/src/admission/wire_pump.rs:447 (AI-S4a -- UNCHANGED) |
| **Tests** | `crates/ade_runtime/src/bootstrap.rs::resolve_live_follow_start_treats_zero_hash_anchor_as_origin`; `crates/ade_runtime/src/bootstrap.rs::bootstrap_bare_anchor_recovery_surfaces_anchor_as_live_follow_tip`; `crates/ade_runtime/src/bootstrap.rs::bootstrap_true_origin_recovery_surfaces_none_tip`; `crates/ade_runtime/src/bootstrap.rs::bootstrap_servable_chaindb_tip_wins_over_anchor`; `crates/ade_runtime/src/bootstrap.rs::warm_start_loads_persisted_anchor_point`; `crates/ade_runtime/src/bootstrap.rs::warm_start_non_origin_anchor_missing_anchor_point_fails_closed`; `crates/ade_runtime/src/bootstrap.rs::warm_start_anchor_point_fingerprint_mismatch_fails_closed`; `crates/ade_runtime/src/bootstrap.rs::same_store_same_anchor_point_same_findintersect_start`; `crates/ade_runtime/src/seed_epoch_lineage.rs::bootstrap_recover_persists_anchor_point_sidecar`; `crates/ade_ledger/src/recovered_anchor_point.rs::recovered_anchor_point_round_trips_byte_identical` … (+1 more) |
| **CI** | _(no CI script listed)_ |

#### `DC-NODE-32` — _enforced_

| Aspect | Location |
|--------|----------|
| **Source** | docs/planning/phase4-n-ak-s2-recovered-anchor-boundary-invariants.md + docs/clusters/PHASE4-N-AK/cluster.md |
| **Requirement** | Recovered-anchor rollback boundary on the single-producer live-follow path (PHASE4-N-AK AK-S2). After recovery to a bare bootstrap anchor, the single-producer follow path (run_node_sync) accepts a peer RollBackward whose target binds EXACTLY (slot AND hash) to the persisted recovered anchor point (DC-NODE-31 / BootstrapState.tip) as an IDEMPOTENT NO-OP boundary rewind: no WAL, no ChainDb mutation, no ledger mutation, no cursor. The anchor is a recovery snapshot boundary, NOT a stored servable block, and is NEVER synthesized into one (ChainDb::tip()/last_block_bytes/serve never return it). RollBackward(Origin) still fails closed (AI-S4a unchanged); every non-anchor, non-Origin rollback fails closed; the accepted point must bind to the PERSISTED anchor on slot AND hash, never peer-supplied alone. The anchor point consumed by run_node_sync is the single authority (BootstrapState.tip), threaded in -- NEVER re-read from the store inside the loop. The first forward block after the anchor admits through the EXISTING sole pump_block path (its prev_hash binds the recovered chain_dep) -- AK-S2 adds NO forward-link code (verified live by the OQ-AK-S2-2 probe: blocks 9-13 admitted via local_chaindb_tip, caught up to the relay tip, 0 errors). Recover->follow on the single-producer path is replay-equivalent (extends T-REC-05/DC-NODE-31 to the follow): same store + same ordered peer feed => byte-identical post-state and admit sequence. SCOPE: does NOT add general stored-block rollback-follow on the single-producer path (out of scope), and does NOT touch the participant path (run_participant_sync @ node_lifecycle.rs -- a separate follow-on obligation). |
| **Code** | crates/ade_runtime/src/forward_sync/reducer.rs (ForwardSyncState.recovered_anchor -- the threaded anchor field, default None), crates/ade_node/src/node_sync.rs (run_node_sync -- single-producer RollBack handler accepts rollback-to-recovered-anchor (exact slot AND hash) as idempotent no-op), crates/ade_node/src/node_lifecycle.rs (ON arm sets fwd.recovered_anchor = BootstrapState.tip; run_participant_sync UNCHANGED -- separate follow-on), crates/ade_runtime/src/admission/wire_pump.rs:447 (AI-S4a Origin fail-close -- UNCHANGED) |
| **Tests** | `crates/ade_node/tests/live_fork_choice_ai_s4bii.rs::ak_s2_rollback_to_recovered_anchor_is_idempotent_noop`; `crates/ade_node/tests/live_fork_choice_ai_s4bii.rs::ak_s2_rollback_to_origin_fails_closed_even_with_anchor`; `crates/ade_node/tests/live_fork_choice_ai_s4bii.rs::ak_s2_non_anchor_rollback_fails_closed_slot_and_hash_bound`; `crates/ade_node/tests/live_fork_choice_ai_s4bii.rs::ak_s2_no_recovered_anchor_still_fails_closed`; `crates/ade_node/tests/live_fork_choice_ai_s4bii.rs::ak_s2_after_anchor_noop_forward_block_reaches_pump_block_validation_holds`; `crates/ade_node/src/node_sync.rs::ak_s2_valid_forward_block_admits_after_recovered_anchor_noop`; `crates/ade_node/tests/live_fork_choice_ai_s4bii.rs::singleproducer_rollback_refused_by_run_node_sync` |
| **CI** | _(no CI script listed)_ |

#### `DC-NODE-33` — _enforced_

| Aspect | Location |
|--------|----------|
| **Source** | docs/planning/phase4-n-al-participant-recovered-anchor-boundary-invariants.md + docs/clusters/PHASE4-N-AL/cluster.md |
| **Requirement** | Participant-path recovered-anchor rollback boundary (PHASE4-N-AL) -- the participant MIRROR of DC-NODE-32. On the participant live-follow path (run_participant_sync), a peer RollBackward whose target binds EXACTLY (slot AND hash) to the persisted recovered anchor point (DC-NODE-31 / BootstrapState.tip, carried in ForwardSyncState.recovered_anchor) is accepted as an IDEMPOTENT NO-OP boundary rewind: no commit_rollback, no WalEntry::RollBack, no ChainDb / ledger / chain_dep mutation, no cursor, no pending_reselection. The anchor is a recovery snapshot boundary, NOT a stored servable block, and is NEVER synthesized into one (ChainDb::tip()/serve never return it). The anchor branch is evaluated BEFORE the existing DC-NODE-29 stored-block resolution: RollBackward(Origin) still fails closed (AI-S4a unchanged); every non-anchor, non-Origin rollback still resolves through the EXISTING DC-NODE-29 authority (get_block_by_hash + stored slot/hash binding -> apply_chain_event or fail closed) UNCHANGED; the accepted anchor point binds to the PERSISTED anchor on slot AND hash, never peer-supplied alone. The anchor consumed by run_participant_sync is the single authority (state.recovered_anchor, set once in the forge-ON arm at node_lifecycle.rs:563 and threaded via run_relay_loop_with_sched -- never re-read from the store inside the loop). The first forward block after the anchor no-op admits through the EXISTING sole pump_block path (its prev_hash binds the recovered chain_dep) -- AL adds NO forward-link code. Recover->follow on the participant path is replay-equivalent (extends T-REC-05 / DC-NODE-31 / DC-NODE-32 to the participant follow): same store + same ordered peer feed => byte-identical post-state and admit sequence. DC-NODE-32 stays scoped to run_node_sync (NOT broadened; this is a distinct sibling rule). SCOPE: the recovered-anchor rollback-to-intersection case ONLY; does NOT add general multi-candidate fork-choice, does NOT change N-AJ evidence emission (DC-NODE-30), does NOT flip CN-CONS-03. |
| **Code** | crates/ade_node/src/node_lifecycle.rs (run_participant_sync RollBack handler -- recovered-anchor exact slot+hash no-op evaluated BEFORE the DC-NODE-29 durable-membership resolution; reads state.recovered_anchor set in the forge-ON arm at node_lifecycle.rs:563), crates/ade_runtime/src/forward_sync/reducer.rs (ForwardSyncState.recovered_anchor -- the existing AK-S2 field, reused unchanged), crates/ade_runtime/src/admission/wire_pump.rs:447 (AI-S4a Origin fail-close -- UNCHANGED) |
| **Tests** | `crates/ade_node/tests/live_fork_choice_ai_s4bii.rs::participant_rollback_to_recovered_anchor_is_noop`; `crates/ade_node/tests/live_fork_choice_ai_s4bii.rs::participant_rollback_origin_fails_closed`; `crates/ade_node/tests/live_fork_choice_ai_s4bii.rs::participant_rollback_non_anchor_fails_closed`; `crates/ade_node/tests/live_fork_choice_ai_s4bii.rs::participant_first_forward_after_anchor_noop_admits_via_pump_block`; `crates/ade_node/tests/live_fork_choice_ai_s4bii.rs::participant_stored_block_rollback_still_applies` |
| **CI** | _(no CI script listed)_ |

#### `DC-NODE-34` — _enforced_

| Aspect | Location |
|--------|----------|
| **Source** | docs/planning/phase4-select-multicandidate-fork-choice-invariants.md (FC-2/FC-10 enabler) + docs/clusters/PHASE4-N-AO/cluster.md |
| **Requirement** | Peer-identity restoration (PHASE4-N-AO, SELECT foundation). The live receive path preserves the origin peer identity end-to-end: AdmissionPeerEvent (peer: String) -> NodeSyncItem -> the participant loop. The NodeBlockSource -> NodeSyncItem conversion MUST carry the peer label (today it is discarded at node_sync.rs from_wire_pump/next_item), so per-peer candidate tracking (DC-NODE-35) is possible. Restoration is provenance-only: a single-peer FOLLOW run admits + replays BYTE-IDENTICALLY to the pre-restoration baseline, and identity restoration MUST NOT alter selection, admission, rollback, or evidence-verdict semantics. RED/GREEN feed shape; NO BLUE change; NodeSyncItem is a transient feed type (not persisted / hashed), so no canonical-type or replay obligation. |
| **Code** | crates/ade_node/src/node_sync.rs (NodeSyncItem::{Block,RollBack} carry the source peer) + crates/ade_node/src/node_lifecycle.rs (peer-tagged participant feed) + crates/ade_node/src/convergence_evidence.rs (per-block block_received peer attribution). Gate ci/ci_check_peer_identity_preserved.sh. |
| **Tests** | `best_of_two_peers_wins_and_is_identified`; `peer_identity_preserved_through_merge` |
| **CI** | `ci/ci_check_peer_identity_preserved.sh` |

#### `DC-NODE-35` — _enforced_

| Aspect | Location |
|--------|----------|
| **Source** | docs/planning/phase4-select-multicandidate-fork-choice-invariants.md (FC-10; the BLUE-safety proof obligation) + docs/clusters/PHASE4-N-AO/cluster.md |
| **Requirement** | BLUE-safe candidate construction (PHASE4-N-AO). A CandidateFragment fed to the BLUE fork-choice authority select_best_chain (DC-CONS-03) MUST be derived ONLY from Ade's own validated headers (validate_and_apply_header output -- the chain_selector::process_header_arrival validate-then-fragment pattern), NEVER a peer-trusted minted ValidatedHeaderSummary (the ade_core_interop::follow shape, which MUST NOT cross into BLUE or any authority path) and NEVER a raw followed_peer_tip. Byte authority: hash-critical protocol paths use the preserved original wire bytes; internal candidate comparison/proof surfaces use project-canonical bytes. Peer identity (DC-NODE-34) is preserved on each candidate. Candidate-set ordering MUST be deterministic. Malformed / missing candidate data fails closed. No live select_best_chain call may be introduced until candidate construction proves these five properties (the cluster's load-bearing S2 entry gate). The aggregator's TCB color (GREEN vs BLUE-adjacent) is resolved by this proof -- the easier color MUST NOT be picked to dodge it. |
| **Code** | crates/ade_node/src/candidate_aggregator.rs (GREEN build_candidate_fragment -- each header validated via reused BLUE validate_and_apply_header, never minted -- + assemble_candidate_set, arrival-order independent). Gate ci/ci_check_candidate_construction_validated.sh. |
| **Tests** | `build_candidate_fragment_assembles_from_validated_headers`; `build_candidate_fragment_empty_headers_fails_closed`; `build_candidate_fragment_rejects_invalid_header_fails_closed`; `build_candidate_fragment_two_runs_byte_identical`; `assemble_candidate_set_ordering_is_arrival_independent` |
| **CI** | `ci/ci_check_candidate_construction_validated.sh` |

#### `DC-NODE-36` — _enforced_

| Aspect | Location |
|--------|----------|
| **Source** | docs/planning/phase4-select-multicandidate-fork-choice-invariants.md (FC-1/FC-2) + docs/clusters/PHASE4-N-AO/cluster.md |
| **Requirement** | Live single-selector dispatch (PHASE4-N-AO). The live participant NeedsForkChoice arm (today fail-closed in run_participant_sync, node_lifecycle.rs) routes the aggregated candidate SET to the SINGLE existing BLUE select_best_chain (DC-CONS-03) -- routed-to, NEVER duplicated: no second selector, no parallel preference, no density ordering, no operator heuristic. The selected tip is arrival-order-independent over the live multi-peer set (the live analog of the CN-CONS-01 permutation proof). A TiebreakerLossKeepCurrent outcome makes NO durable change. Only validated candidate summaries (DC-NODE-35) reach select_best_chain; a raw followed_peer_tip never does. |
| **Code** | crates/ade_node/src/node_lifecycle.rs (RED dispatch_competing_fork_choice + GREEN decide_fork_switch routing the live participant path into the SOLE BLUE select_best_chain; DECIDE-only -- sets PendingForkSwitch + the DC-NODE-28 fence, applies nothing) + crates/ade_node/src/selector_state.rs (ForkAnchor/PendingForkSwitch/project_tiebreaker). BLUE select_best_chain UNCHANGED. Gate ci/ci_check_live_selector_dispatch.sh. |
| **Tests** | `win_emits_switch_to_winning_peer_and_durable_anchor`; `tiebreaker_loss_keeps_current`; `exceeded_rollback_keeps_current`; `best_of_two_peers_wins_and_is_identified` |
| **CI** | `ci/ci_check_live_selector_dispatch.sh` |

#### `DC-NODE-37` — _enforced_

| Aspect | Location |
|--------|----------|
| **Source** | docs/planning/phase4-select-multicandidate-fork-choice-invariants.md (FC-5/FC-6/FC-7) + docs/clusters/PHASE4-N-AO/cluster.md |
| **Requirement** | Fork-switch never-abandon (PHASE4-N-AO, SELECT primary invariant; the H-1 class at fork-choice scale). When select_best_chain picks a winner that forks BELOW Ade's current durable tip, Ade MUST NEVER commit a rollback of its current durable chain until the replacement branch's bodies are FETCHED, LINKED, and VALIDATED as a complete candidate branch (block-fetched via BlockFetch RequestRange anchor->tip from the winning peer; the fork anchor canonically bound to Ade's durable STORED slot+hash per DC-NODE-29, never peer-supplied / mixed authority). A failed / lying / Byzantine / incomplete replacement branch leaves ChainDb + LedgerState + PraosChainDepState UNCHANGED. Adoption then proceeds ONLY via the already-enforced authorities: RolledBack(fork_anchor) through materialize_rolled_back_state (CN-STORE-07) + the lockstep receive reducer (DC-CONS-20), then ChainSelected(body) x N through pump_block (DC-NODE-05/12) -- a fork-choice win is provisional, durably adopted ONLY when its BODIES validate+apply (no header-only tip advance). The reselection is recorded as the durable append-only WalEntry::RollBack{ForkChoiceWin} and is replay-equivalent (DC-NODE-27, extended) -- NOT a second rollback implementation. No forge across the pending reselection (DC-NODE-28). |
| **Code** | crates/ade_node/src/fork_switch.rs (GREEN-pure prevalidate_branch: bind+link+block_validity) + crates/ade_node/src/node_lifecycle.rs (RED prove_fork_switch -- mutation-free -- then apply_fork_switch: prove-then-commit via apply_chain_event RolledBack{ForkChoiceWin}+ChainSelected, fence cleared LAST; ProofFailed holds the fence). Gate ci/ci_check_fork_switch_never_abandons.sh. |
| **Tests** | `empty_branch_fails_closed_before_any_apply`; `null_source_serves_nothing`; `fork_switch_win_adopts_via_rolledback_then_chainselected`; `body_hash_mismatch_leaves_chain_unchanged`; `broken_parent_link_leaves_chain_unchanged`; `selected_peer_missing_body_leaves_chain_unchanged_fence_held`; `proof_failure_holds_fence_then_resolves_when_caught_up` |
| **CI** | `ci/ci_check_fork_switch_never_abandons.sh` |

#### `DC-NODE-38` — _enforced_

| Aspect | Location |
|--------|----------|
| **Source** | docs/planning/phase4-n-ao-ce-ao-6-live-gap.md + docs/clusters/PHASE4-N-AO/S7-lca-anchor-walk.md |
| **Requirement** | Live multi-block fork-anchor discovery (PHASE4-N-AO S7; the live-geometry gap CE-AO-6 surfaced). A live competing branch is eligible for SELECT only when Ade walks its preserved parent links back to a DURABLE STORED fork anchor within k (BLOCK DEPTH, not slot distance), then validates the COMPLETE intermediate header chain from that anchor (S2 validate_and_apply_header) BEFORE selection. The per-peer branch cache is NOT authority -- only an indexed memory of received, preserved headers; each entry self-binds (key_hash == re-derived header block hash, entry.slot == header.slot, entry.prev_hash == header.prev_hash) or the branch fails closed. The last-common-ancestor is the fork anchor, authoritative ONLY when ChainDb confirms slot AND hash (DC-NODE-29) -- never the competing block's immediate parent, never peer-supplied, never slot-only, never hash-only. The walk is k-bounded by BLOCK DEPTH (traversed-header count <= k AND current_tip_block_no minus lca_block_no <= k; no slot subtraction); over-k / branch-gap / no-durable-LCA / cache-self-binding- violation / lying-parent-link all fail closed with no durable mutation. A 1-deep fork degenerates to the prior single-step behavior. CARRY-FORWARD: a competing branch arriving via RollBackward/FOLLOW sequencing rather than competing Block arrivals is a SEPARATE wire-interleaving diagnostic and must not weaken this competing-block LCA invariant. |
| **Code** | crates/ade_node/src/lca_walk.rs (walk_to_durable_lca + CachedHeader: k-bounded by block depth, self-binding by re-derived hash, durable LCA anchor is ChainDb stored slot+hash only). Gate ci/ci_check_lca_anchor_walk.sh. |
| **Tests** | `one_block_fork_walks_in_one_step`; `multi_block_branch_walks_to_durable_lca`; `missing_intermediate_header_fails_closed`; `ancestor_older_than_k_fails_closed_block_depth`; `cache_self_binding_violation_fails_closed`; `lying_parent_link_to_genesis_fails_closed`; `arrival_order_permutation_walks_identical`; `walk_is_deterministic` |
| **CI** | `ci/ci_check_lca_anchor_walk.sh` |

#### `DC-NODE-39` — _enforced_

| Aspect | Location |
|--------|----------|
| **Source** | docs/planning/phase4-n-ao-ce-ao-6-live-gap.md (run-1 root-cause finding) + docs/clusters/PHASE4-N-AO/S11-post-forkchoicewin-forward-follow.md |
| **Requirement** | Post-ForkChoiceWin forward-follow continuity (PHASE4-N-AO S11). After a ForkChoiceWin adoption at tip X, Ade must continue receiving and admitting the winning peer's descendants in PARENT-LINK ORDER (X -> X+1 -> X+2, each via validated prev_hash == prior tip), OR fail closed with a STRUCTURED missing-bridge reason; it must NOT silently skip a required bridge block and stall behind the winning branch. A descendant whose parent link Ade has not validated is never admitted (no peer-claimed bridging -- the parent must be in Ade's validated store / proven branch). On a genuinely missing bridge (peer serves X+2 without X+1) the post-switch admit path emits a closed MissingBridge discriminant, PRESERVES the current durable chain byte-unchanged (no rollback, no admit, no adoption -- MissingBridge is never a rollback target / candidate anchor / fence-clear reason), HOLDS the forge fence (pending_missing_bridge.is_none() is now a fence-resolve precondition), and refuses the silent no-op -- never a silent stall and never a forced/guessed admit. The hold is HOLD-until-progress: a real LinearExtend admit (pump_block Some) or a proven fork-switch adoption clears it, so a late-arriving bridge releases the fence. REPLAY-EQUIVALENT: given the same post-switch served sequence and the same adopted X, Ade derives the same admit / MissingBridge outcome byte-identically. This is the robustness half of CN-CONS-03: convergence is not 'Ade can complete once' (S10/run 2 proved capability) but 'Ade reliably continues on the selected branch or fails closed structurally', closing the conditional follow-forward hole run 1 surfaced (adopted cn2@298, received 388 but missed the bridge ~340, stalled fail-closed-but-no-progress). |
| **Code** | crates/ade_node/src/node_lifecycle.rs (the dispatch walk-fail + materialize-fail arms emit structured MissingBridge + set pending_missing_bridge HOLD, never a silent no-op) + crates/ade_node/src/fork_switch.rs (closed MissingBridgeReason + fork_switch_fence_resolved requires pending_missing_bridge.is_none()). Gate ci/ci_check_missing_bridge_fail_closed.sh. |
| **Tests** | `post_switch_admits_winner_descendant_x_plus_1`; `post_switch_missing_bridge_emits_structured_and_holds_fence`; `missing_bridge_wrong_parent_maps_closed_code`; `late_bridge_clears_hold_on_progress`; `missing_bridge_reason_maps_lca_error_to_closed_discriminant`; `bridge_gap_injection_emits_missing_bridge`; `late_bridge_recovers_on_progress` |
| **CI** | `ci/ci_check_missing_bridge_fail_closed.sh` |

#### `DC-NODE-40` — _enforced_

| Aspect | Location |
|--------|----------|
| **Source** | docs/planning/phase4-n-ao-ce-ao-6-live-gap.md (S11 Fault-1 finding) + docs/clusters/PHASE4-N-AO/S13-rolled-back-branch-evidence-retention.md |
| **Requirement** | Rolled-back branch evidence retention for the LCA walk (PHASE4-N-AO S13). Rolled-back blocks MAY be retained only as walk-visible EVIDENCE for future competing-branch reconstruction: k-bounded (block depth), hash-keyed (BTreeMap, never HashMap-iterated for semantic ordering), self-binding (map key == re-derived block hash; a mismatch fails closed, CacheSelfBindingViolation). They NEVER become durable authority, a rollback target, or the LCA anchor, and NEVER bypass S2 header validation or S4 body prevalidation. The durable LCA remains the ChainDb stored slot+hash ONLY (DC-NODE-29); the retention merely lets walk_to_durable_lca traverse non-durable INTERMEDIATE headers -- exactly the blocks Ade itself rolled back during a ForkChoiceWin adoption -- until it reaches a real durable ancestor. This closes the Fault-1 MissingBridge OVER-FIRE: after cn1->cn2 switch, cn1's own rolled-back blocks (admitted LinearExtend, never in the S7 competing-only branch cache) are retained so cn1's later competing blocks are EVALUABLE (fork-choice resolves them -- they lose) instead of falsely un-bridgeable (BranchGap -> MissingBridge -> fence-held on every loser block, a producer-liveness risk). MissingBridge (DC-NODE-39) is then reserved for a GENUINE gap (neither durable nor cache nor retention has the bridge -- Fault 2). REPLAY-EQUIVALENT: same rollback + same retained set -> same walk verdict. |
| **Code** | a BTreeMap<Hash32, CachedHeader> rollback-retention cache OWNED in crates/ade_node/src/node_lifecycle.rs ForgeActivation (cross-iteration, alongside pending_fork_switch/pending_missing_bridge -- NOT a run_participant_sync local, which is reborn empty each drain), populated in apply_fork_switch BEFORE the ChainEvent::RolledBack apply (walk old_tip->fork_anchor+1 via ChainDb decode_block, insert self-bound key==block_hash + k-bounded retain by security_param.0), consulted by crates/ade_node/src/lca_walk.rs walk_to_durable_lca on a per-peer-cache miss (cache.get(h).or_else(\|\| retention.get(h))). The durable-anchor check (chaindb.get_block_by_hash) is UNCHANGED. BLUE select/apply/validate UNCHANGED. |
| **Tests** | `rollback_retains_removed_blocks_for_lca_walk`; `retained_blocks_are_not_anchors`; `retained_blocks_are_k_bounded`; `retained_block_hash_self_binds`; `genuine_gap_still_missing_bridge`; `apply_fork_switch_populates_rollback_retention` |
| **CI** | `ci/ci_check_rollback_retention_evidence.sh` |

#### `DC-NODE-41` — _enforced_

| Aspect | Location |
|--------|----------|
| **Source** | docs/planning/phase4-n-ao-ce-ao-6-live-gap.md (S11 Fault-2 finding) + docs/clusters/PHASE4-N-AO/S14-missing-bridge-range-refetch.md |
| **Requirement** | Missing-bridge range re-fetch for winner-descendant recovery (PHASE4-N-AO S14). When a post-ForkChoiceWin WINNING peer (the peer Ade just adopted from) presents a descendant whose parent chain is missing, Ade must EITHER (a) actively re-fetch the missing range from the adopted tip to that descendant via BlockFetch RequestRange(X+1..descendant) to that winning peer and admit it IN PARENT-LINK ORDER through pump_block (each body parent-link + body-hash validated before admit; X+1 then X+2 ... then the descendant), OR (b) remain fail-closed with a structured MissingBridge (closed failure code). It must NOT passively stall forever (the DC-NODE-39 floor alone cannot recover a bridge ChainSync already streamed past -- each block is delivered once, never re-sent), and must NOT admit out of order. The re-fetch is byte-only (BlockFetch transports bytes, NOT truth -- a lying/short/unservable range leaves the structured hold, no admit, no mutation), targets ONLY the winning peer (a non-winning-peer gap takes the unchanged floor path -- no fetch spam on loser orphans), is bounded-retry (deterministic, no spin), and clears the hold + fence ONLY on real admitted progress. S14 is RECOVERY, not selection: S3 (select_best_chain, DC-CONS-03) already decided the winner; S14 never decides a branch wins. pump_block remains the sole admit (BLUE unchanged); the BlockFetch byte machinery is reused from S6 (prefetch_branch_bodies). REPLAY-EQUIVALENT: same served range -> same admitted post-state. |
| **Code** | crates/ade_node/src/fork_switch.rs: closed RangeRefetchOutcome{Admitted\|Unavailable\|ShortRange\|BodyHeaderMismatch\|ParentLinkMismatch\|ValidationFailed}+as_str, RangeRefetch/PostSwitchFollow types, MAX_RANGE_REFETCH_ATTEMPTS + range_refetch_should_retry (bounded-retry RED policy), PrefetchedBranchBodies::ordered_for_peer. crates/ade_node/src/node_lifecycle.rs: recover_missing_range (pump_block sole admit, per-block block_admitted evidence); dispatch_competing_fork_choice walk-fail arm sets pending_range_refetch ONLY for a winning-peer descendant ahead of the durable tip (gated on post_switch_follow.winning_peer == peer), ALONGSIDE the DC-NODE-39 floor hold; the participant relay loop records ForgeActivation.post_switch_follow on ForkSwitchOutcome::Adopted and consumes ForgeActivation.pending_range_refetch via a staleness-guarded, bounded-retry prefetch_branch_bodies(S6, byte-only)->recover_missing_range drive, clearing the hold ONLY on RangeRefetchOutcome::Admitted. Closed range_refetch_started/completed events in crates/ade_node/src/admission_log/{event.rs,writer.rs} + emit in convergence_evidence.rs. BLUE validation/select/apply/pump_block UNCHANGED. |
| **Tests** | `refetched_bridge_admits_in_order`; `refetch_failure_structured`; `short_refetch_keeps_hold`; `lying_refetch_body_rejected`; `missing_bridge_triggers_range_refetch`; `bounded_retry` |
| **CI** | `ci/ci_check_missing_bridge_refetch.sh` |

### DC-OPCERT

#### `DC-OPCERT-01` — _enforced_

| Aspect | Location |
|--------|----------|
| **Source** | docs/planning/phase4-n-r-invariants.md §3 (D4) |
| **Requirement** | Given the same canonical envelope bytes, parse_opcert_envelope produces a byte-identical DecodedOpCertEnvelope across runs. Replay-equivalence anchor for the opcert envelope decode. |
| **Code** | crates/ade_runtime/src/producer/opcert_envelope.rs (parser_is_byte_identical_across_two_runs unit test) |
| **Tests** | `parser_is_byte_identical_across_two_runs` |
| **CI** | _(no CI script listed)_ |

### DC-OUTBOUND-FIFO

#### `DC-OUTBOUND-FIFO-01` — _enforced_

| Aspect | Location |
|--------|----------|
| **Source** | docs/planning/phase4-n-s-invariants.md §3 (D4) |
| **Requirement** | The per-peer outbound channel preserves FIFO order: OutboundCommands enqueued for PeerId(p) in order O₁..Oₙ arrive at the peer's TCP socket in the same order (mpsc::Sender::send guarantees FIFO; MuxPump's session-aware encoder processes them sequentially). |
| **Code** | crates/ade_runtime/src/network/outbound_command.rs + mux_pump.rs (FIFO is structurally guaranteed by tokio::sync::mpsc::Sender's FIFO contract + MuxPump's sequential session::step processing) |
| **Tests** | _(no tests listed — gap)_ |
| **CI** | _(no CI script listed)_ |

### DC-PLUTUS

#### `DC-PLUTUS-01` — _declared_

| Aspect | Location |
|--------|----------|
| **Source** | Project constitution §3, T-DET-01, T-CORE-01 |
| **Requirement** | UPLC evaluation is deterministic: same script + args + cost model = identical result |
| **Code** | — _(declared — not yet enforced)_ |
| **Tests** | — _(declared — not yet enforced)_ |
| **CI** | — _(declared — not yet enforced)_ |

#### `DC-PLUTUS-02` — _declared_

| Aspect | Location |
|--------|----------|
| **Source** | Project constitution §3, T-ERR-01 |
| **Requirement** | Budget exhaustion produces deterministic structured error |
| **Code** | — _(declared — not yet enforced)_ |
| **Tests** | — _(declared — not yet enforced)_ |
| **CI** | — _(declared — not yet enforced)_ |

### DC-PROD

#### `DC-PROD-01` — _enforced_

| Aspect | Location |
|--------|----------|
| **Source** | docs/planning/phase4-n-q-invariants.md §1 (I11); §2 (N9, N12, N13, N15) |
| **Requirement** | Producer-mode evidence log emits a closed `ProducerLogEvent` vocabulary: handshake_ok, slot_tick, leader_elected, block_forged, block_served, peer_chain_tip_observed, slot_missed{reason: closed_enum}, coordinator_shutdown{reason: closed_enum}. No free-form reason strings; no key material; no path strings. Socket addresses MUST NOT appear inside the replayable event stream — `PeerId` is an opaque `u64` (coordinator-internal counter); socket addresses are RED operational metadata, surfaced separately and excluded from replay-equivalence comparison. Mirrors the LiveLogEvent / AdmissionLogEvent precedent established in N-L / N-M while remaining a distinct vocabulary. |
| **Code** | crates/ade_runtime/src/producer/producer_log.rs (closed enum + closed reason sub-enums); crates/ade_node/src/produce_mode.rs (RED writer) |
| **Tests** | `event_kinds_are_distinct_and_stable`; `json_serialization_round_trips_byte_identical_for_replay`; `no_string_fields_in_any_variant`; `slot_missed_reason_serializes_to_stable_strings`; `produce_mode_starts_runs_three_slots_and_exits_via_max_slots` |
| **CI** | _(no CI script listed)_ |

#### `DC-PROD-02` — _enforced_

| Aspect | Location |
|--------|----------|
| **Source** | docs/planning/phase4-n-q-invariants.md §1 (I7, I8); §3 (D5); §4 (R3, R4); §8 |
| **Requirement** | Coordinator slot-tick + forge-result stream replay-equivalence. For a fixed initial CoordinatorState, fixed canonical slot-tick sequence, fixed ledger state, fixed opcert public metadata, and fixed RED forge-result event stream (ForgeSucceeded \| ForgeFailed sequence), the coordinator emits byte-identical broadcast effects and byte-identical ProducerLogEvent sequence. Wall-clock real-time timestamps and socket arrival order are non-load-bearing RED metadata. Replay is over canonical event streams — NOT real wall-clock time. The forge-result event stream is the canonical surface across the RED-key-custody boundary; the GREEN coordinator is replayable against it without ever seeing secret material. |
| **Code** | crates/ade_runtime/src/producer/coordinator.rs (GREEN reducer + S2 inline replay test) |
| **Tests** | `replay_byte_identity_across_two_runs` |
| **CI** | _(no CI script listed)_ |

#### `DC-PROD-03` — _enforced_

| Aspect | Location |
|--------|----------|
| **Source** | docs/planning/phase4-n-t-invariants.md §3, §4 (R1, R3); docs/clusters/PHASE4-N-T/cluster.md §1.5, §7 |
| **Requirement** | Producer chain-forward continuity + replay. The GREEN ChainEvolution linear typestate threads each forge's post-state (post-ledger, post-chain_dep, new tip) into the next forge's base; forging against a stale base is structurally unrepresentable (advance consumes self). advance obtains the post-state from BLUE block_validity and the AcceptedBlock token from BLUE self_accept against identical inputs (same pre-forge base, forged bytes, era_schedule, ledger_view); if the two authorities disagree it returns ChainEvolutionError::AuthorityMismatch and does not advance. ChainEvolution never constructs AcceptedBlock directly. For a fixed (bootstrap seed, canonical slot-sequence, KES/VRF/cold keys) the chain-evolution series (block_number, prev_hash, post-ledger fingerprint, post-chain_dep) and the forged block bytes are byte-identical across runs (in-memory two-run; no on-disk replay corpus — durability deferred to N-U). |
| **Code** | crates/ade_runtime/src/producer/chain_evolution.rs (ChainEvolution seed/derive_forge_context/advance + ChainEvolutionError incl. AuthorityMismatch) |
| **Tests** | `advance_threads_post_state_forward`; `advance_two_runs_byte_identical`; `advance_rejects_invalid_bytes`; `reconcile_verdicts_both_valid_ok`; `reconcile_verdicts_both_invalid_ok`; `reconcile_verdicts_valid_vs_reject_mismatches`; `reconcile_verdicts_reject_vs_valid_mismatches`; `served_snapshot_two_run_replay_byte_identical` |
| **CI** | _(no CI script listed)_ |

### DC-PROTO

#### `DC-PROTO-01` — _enforced_

| Aspect | Location |
|--------|----------|
| **Source** | Project constitution §3, T-CORE-03 |
| **Requirement** | Protocol state machines have deterministic transitions |
| **Code** | crates/ade_network/src/chain_sync/state.rs, crates/ade_network/src/chain_sync/agency.rs, crates/ade_network/src/chain_sync/signal.rs, crates/ade_network/src/chain_sync/transition.rs, crates/ade_network/src/block_fetch/state.rs, crates/ade_network/src/block_fetch/agency.rs, crates/ade_network/src/block_fetch/event.rs, crates/ade_network/src/block_fetch/transition.rs, crates/ade_network/src/tx_submission/state.rs, crates/ade_network/src/tx_submission/agency.rs, crates/ade_network/src/tx_submission/event.rs, crates/ade_network/src/tx_submission/transition.rs, crates/ade_network/src/keep_alive/state.rs, crates/ade_network/src/keep_alive/agency.rs, crates/ade_network/src/keep_alive/event.rs, crates/ade_network/src/keep_alive/transition.rs, crates/ade_network/src/peer_sharing/state.rs, crates/ade_network/src/peer_sharing/agency.rs, crates/ade_network/src/peer_sharing/event.rs, crates/ade_network/src/peer_sharing/transition.rs, crates/ade_network/src/n2c/local_chain_sync/state.rs, crates/ade_network/src/n2c/local_chain_sync/agency.rs, crates/ade_network/src/n2c/local_chain_sync/event.rs, crates/ade_network/src/n2c/local_chain_sync/transition.rs, crates/ade_network/src/n2c/local_tx_submission/state.rs, crates/ade_network/src/n2c/local_tx_submission/agency.rs, crates/ade_network/src/n2c/local_tx_submission/event.rs, crates/ade_network/src/n2c/local_tx_submission/transition.rs, crates/ade_network/src/n2c/local_state_query/state.rs, crates/ade_network/src/n2c/local_state_query/agency.rs, crates/ade_network/src/n2c/local_state_query/event.rs, crates/ade_network/src/n2c/local_state_query/transition.rs, crates/ade_network/src/n2c/local_tx_monitor/state.rs, crates/ade_network/src/n2c/local_tx_monitor/agency.rs, crates/ade_network/src/n2c/local_tx_monitor/event.rs, crates/ade_network/src/n2c/local_tx_monitor/transition.rs |
| **Tests** | `chain_sync::transition::tests::idle_request_next_with_immediate_data_yields_can_await_then_roll_forward`; `chain_sync::transition::tests::idle_request_next_with_no_data_yields_must_reply_via_await`; `chain_sync::transition::tests::roll_forward_signal_carries_header_and_tip_byte_identical`; `chain_sync::transition::tests::roll_backward_signal_carries_point_and_tip_byte_identical`; `chain_sync::transition::tests::find_intersect_with_known_point_yields_intersected_signal`; `chain_sync::transition::tests::find_intersect_with_unknown_points_yields_no_intersection`; `chain_sync::transition::tests::client_done_terminates_session`; `chain_sync::transition::tests::illegal_message_in_idle_returns_error`; `chain_sync::transition::tests::wrong_agency_returns_error`; `chain_sync::transition::tests::version_gating_rejects_out_of_version_message` … (+80 more) |
| **CI** | `ci/ci_check_mini_protocol_transition_purity.sh` |

#### `DC-PROTO-02` — _enforced_

| Aspect | Location |
|--------|----------|
| **Source** | Project constitution §3, T-DET-01 |
| **Requirement** | Transcript-equivalent miniprotocol behavior with Haskell node |
| **Code** | crates/ade_network/src/chain_sync/state.rs, crates/ade_network/src/chain_sync/agency.rs, crates/ade_network/src/chain_sync/signal.rs, crates/ade_network/src/chain_sync/transition.rs, crates/ade_network/src/block_fetch/state.rs, crates/ade_network/src/block_fetch/agency.rs, crates/ade_network/src/block_fetch/event.rs, crates/ade_network/src/block_fetch/transition.rs, crates/ade_network/src/tx_submission/state.rs, crates/ade_network/src/tx_submission/agency.rs, crates/ade_network/src/tx_submission/event.rs, crates/ade_network/src/tx_submission/transition.rs |
| **Tests** | `chain_sync::transition::tests::idle_request_next_with_immediate_data_yields_can_await_then_roll_forward`; `chain_sync::transition::tests::idle_request_next_with_no_data_yields_must_reply_via_await`; `chain_sync::transition::tests::roll_forward_signal_carries_header_and_tip_byte_identical`; `chain_sync::transition::tests::roll_backward_signal_carries_point_and_tip_byte_identical`; `chain_sync::transition::tests::find_intersect_with_known_point_yields_intersected_signal`; `chain_sync::transition::tests::find_intersect_with_unknown_points_yields_no_intersection`; `chain_sync::transition::tests::client_done_terminates_session`; `chain_sync::transition::tests::illegal_message_in_idle_returns_error`; `chain_sync::transition::tests::wrong_agency_returns_error`; `chain_sync::transition::tests::version_gating_rejects_out_of_version_message` … (+38 more) |
| **CI** | `ci/ci_check_tx_submission2_real_capture.sh` |

#### `DC-PROTO-03` — _enforced_

| Aspect | Location |
|--------|----------|
| **Source** | Project constitution §3, T-DET-01, T-CORE-03 |
| **Requirement** | Full N2N mini-protocol surface: Handshake, ChainSync, BlockFetch, TxSubmission2, KeepAlive, PeerSharing |
| **Code** | crates/ade_network/src/codec/handshake.rs, crates/ade_network/src/codec/chain_sync.rs, crates/ade_network/src/codec/block_fetch.rs, crates/ade_network/src/codec/tx_submission.rs, crates/ade_network/src/codec/keep_alive.rs, crates/ade_network/src/codec/peer_sharing.rs |
| **Tests** | `codec::handshake::tests::roundtrip_every_variant`; `codec::chain_sync::tests::roundtrip_every_variant`; `codec::block_fetch::tests::roundtrip_every_variant`; `codec::tx_submission::tests::roundtrip_every_variant`; `codec::keep_alive::tests::roundtrip_every_variant`; `codec::peer_sharing::tests::roundtrip_every_variant` |
| **CI** | `ci/ci_check_mini_protocol_surface.sh` |

#### `DC-PROTO-04` — _enforced_

| Aspect | Location |
|--------|----------|
| **Source** | Project constitution §3, T-DET-01, T-CORE-03 |
| **Requirement** | Full N2C mini-protocol surface: Handshake, LocalChainSync, LocalTxSubmission, LocalStateQuery, LocalTxMonitor |
| **Code** | crates/ade_network/src/codec/n2c_handshake.rs, crates/ade_network/src/codec/local_chain_sync.rs, crates/ade_network/src/codec/local_tx_submission.rs, crates/ade_network/src/codec/local_state_query.rs, crates/ade_network/src/codec/local_tx_monitor.rs, crates/ade_network/src/n2c/local_chain_sync/state.rs, crates/ade_network/src/n2c/local_chain_sync/agency.rs, crates/ade_network/src/n2c/local_chain_sync/event.rs, crates/ade_network/src/n2c/local_chain_sync/transition.rs, crates/ade_network/src/n2c/local_tx_submission/state.rs, crates/ade_network/src/n2c/local_tx_submission/agency.rs, crates/ade_network/src/n2c/local_tx_submission/event.rs, crates/ade_network/src/n2c/local_tx_submission/transition.rs, crates/ade_network/src/n2c/local_state_query/state.rs, crates/ade_network/src/n2c/local_state_query/agency.rs, crates/ade_network/src/n2c/local_state_query/event.rs, crates/ade_network/src/n2c/local_state_query/transition.rs, crates/ade_network/src/n2c/local_tx_monitor/state.rs, crates/ade_network/src/n2c/local_tx_monitor/agency.rs, crates/ade_network/src/n2c/local_tx_monitor/event.rs, crates/ade_network/src/n2c/local_tx_monitor/transition.rs |
| **Tests** | `codec::n2c_handshake::tests::roundtrip_every_variant`; `codec::local_chain_sync::tests::roundtrip_every_variant`; `codec::local_tx_submission::tests::roundtrip_every_variant`; `codec::local_state_query::tests::roundtrip_every_variant`; `codec::local_tx_monitor::tests::roundtrip_every_variant`; `n2c::local_chain_sync::transition::tests::local_chain_sync_request_next_then_roll_forward`; `n2c::local_chain_sync::transition::tests::local_chain_sync_roll_backward_signal`; `n2c::local_chain_sync::transition::tests::local_chain_sync_find_intersect_known_point`; `n2c::local_chain_sync::transition::tests::local_chain_sync_find_intersect_unknown`; `n2c::local_chain_sync::transition::tests::local_chain_sync_client_done_terminates` … (+32 more) |
| **CI** | `ci/ci_check_mini_protocol_surface.sh` |

#### `DC-PROTO-05` — _declared_

| Aspect | Location |
|--------|----------|
| **Source** | Project constitution §3, T-ENC-02, T-CORE-03 |
| **Requirement** | Version negotiation is closed: enumerated N2N/N2C versions, explicit handshake, deterministic refusal on mismatch |
| **Code** | crates/ade_network/src/handshake/mod.rs, crates/ade_network/src/handshake/state.rs, crates/ade_network/src/handshake/agency.rs, crates/ade_network/src/handshake/selection.rs, crates/ade_network/src/handshake/transition.rs, crates/ade_network/src/handshake/version_table.rs |
| **Tests** | `handshake::transition::tests::n2n_happy_path_each_supported_version`; `handshake::transition::tests::n2c_happy_path_each_supported_version`; `handshake::transition::tests::version_mismatch_refused`; `handshake::transition::tests::illegal_message_in_idle_returns_error`; `handshake::transition::tests::wrong_agency_returns_error`; `handshake::transition::tests::overlap_picks_highest_common`; `handshake::transition::tests::empty_intersection_refuses_deterministically`; `handshake::transition::tests::version_data_passed_through_byte_identical`; `handshake::transition::tests::n2n_v15_happy_path`; `handshake::transition::tests::n2n_v16_happy_path_with_peras_support_field` … (+11 more) |
| **CI** | `ci/ci_check_ce_n_a_5_proof.sh` |

#### `DC-PROTO-06` — _enforced_

| Aspect | Location |
|--------|----------|
| **Source** | Project constitution §2; PHASE4-N-A invariants §7 decision 1 (docs/active/PHASE4-N-A_invariants.md) |
| **Requirement** | BLUE mini-protocol transitions are pure functions of (canonical prior state, canonical input message, selected protocol version, deterministic configuration); no ambient session-glue state may alter authoritative behavior. Selected version is an explicit input, never read from RED context. |
| **Code** | crates/ade_network/src/chain_sync/state.rs, crates/ade_network/src/chain_sync/agency.rs, crates/ade_network/src/chain_sync/signal.rs, crates/ade_network/src/chain_sync/transition.rs, crates/ade_network/src/block_fetch/state.rs, crates/ade_network/src/block_fetch/agency.rs, crates/ade_network/src/block_fetch/event.rs, crates/ade_network/src/block_fetch/transition.rs, crates/ade_network/src/tx_submission/state.rs, crates/ade_network/src/tx_submission/agency.rs, crates/ade_network/src/tx_submission/event.rs, crates/ade_network/src/tx_submission/transition.rs, crates/ade_network/src/keep_alive/state.rs, crates/ade_network/src/keep_alive/agency.rs, crates/ade_network/src/keep_alive/event.rs, crates/ade_network/src/keep_alive/transition.rs, crates/ade_network/src/peer_sharing/state.rs, crates/ade_network/src/peer_sharing/agency.rs, crates/ade_network/src/peer_sharing/event.rs, crates/ade_network/src/peer_sharing/transition.rs, crates/ade_network/src/n2c/local_chain_sync/state.rs, crates/ade_network/src/n2c/local_chain_sync/agency.rs, crates/ade_network/src/n2c/local_chain_sync/event.rs, crates/ade_network/src/n2c/local_chain_sync/transition.rs, crates/ade_network/src/n2c/local_tx_submission/state.rs, crates/ade_network/src/n2c/local_tx_submission/agency.rs, crates/ade_network/src/n2c/local_tx_submission/event.rs, crates/ade_network/src/n2c/local_tx_submission/transition.rs, crates/ade_network/src/n2c/local_state_query/state.rs, crates/ade_network/src/n2c/local_state_query/agency.rs, crates/ade_network/src/n2c/local_state_query/event.rs, crates/ade_network/src/n2c/local_state_query/transition.rs, crates/ade_network/src/n2c/local_tx_monitor/state.rs, crates/ade_network/src/n2c/local_tx_monitor/agency.rs, crates/ade_network/src/n2c/local_tx_monitor/event.rs, crates/ade_network/src/n2c/local_tx_monitor/transition.rs |
| **Tests** | `chain_sync::transition::tests::idle_request_next_with_immediate_data_yields_can_await_then_roll_forward`; `chain_sync::transition::tests::idle_request_next_with_no_data_yields_must_reply_via_await`; `chain_sync::transition::tests::roll_forward_signal_carries_header_and_tip_byte_identical`; `chain_sync::transition::tests::roll_backward_signal_carries_point_and_tip_byte_identical`; `chain_sync::transition::tests::find_intersect_with_known_point_yields_intersected_signal`; `chain_sync::transition::tests::find_intersect_with_unknown_points_yields_no_intersection`; `chain_sync::transition::tests::client_done_terminates_session`; `chain_sync::transition::tests::illegal_message_in_idle_returns_error`; `chain_sync::transition::tests::wrong_agency_returns_error`; `chain_sync::transition::tests::version_gating_rejects_out_of_version_message` … (+80 more) |
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
| **Tests** | `producer_chain_sync_serve_request_next_idle_yields_roll_forward_when_served_has_block`; `producer_chain_sync_serve_request_next_idle_yields_await_reply_when_served_empty`; `producer_chain_sync_serve_find_intersect_known_point_yields_intersect_found`; `producer_chain_sync_serve_find_intersect_unknown_point_yields_intersect_not_found`; `producer_chain_sync_serve_done_terminates_session`; `producer_chain_sync_serve_rejects_illegal_grammar_pair`; `producer_chain_sync_advance_tip_idle_yields_none`; `producer_chain_sync_advance_tip_can_await_yields_roll_forward_when_block_available`; `producer_chain_sync_advance_tip_must_reply_yields_roll_forward_when_block_available`; `producer_chain_sync_advance_tip_can_await_yields_none_when_cursor_at_head` … (+2 more) |
| **CI** | `ci/ci_check_chain_sync_server_closure.sh` |

#### `DC-PROTO-09` — _enforced_

| Aspect | Location |
|--------|----------|
| **Source** | docs/planning/receive-side-bridge-invariants.md §3, §4 |
| **Requirement** | Receive-side transcript determinism: given canonical inputs (initial_ledger, initial_chain_dep, initial_chaindb, event_sequence), the bridge reducer's output state (ledger', chain_dep', chaindb') is byte-identical across replays. The reducer is a pure, total transition. |
| **Code** | crates/ade_ledger/src/receive/reducer.rs (receive_apply + receive_apply_sequence pure transitions); crates/ade_runtime/src/receive/events_to_state.rs (GREEN adapter — pure, no I/O); crates/ade_runtime/src/receive/in_memory_chain_write.rs (GREEN ChainDb-write adapter — pure projection over an in-memory ChainDb) |
| **Tests** | `receive_apply_replay_byte_identical_over_corpus`; `receive_session_transcript_replay_byte_identical` |
| **CI** | `ci/ci_check_receive_reducer_closure.sh`; `ci/ci_check_receive_replay_purity.sh` |

#### `DC-PROTO-10` — _enforced_

| Aspect | Location |
|--------|----------|
| **Source** | docs/clusters/PHASE4-N-AE/slices/AE.E.md; docs/evidence/phase4-n-ae-ce-a5-relay-adoption.md |
| **Requirement** | Chain-sync server FindIntersect cursor: after the producer chain-sync server answers IntersectFound(point), its read cursor (last_announced) IS that point -- the next RequestNext serves next_after(point) (the successor the client rolls forward onto), never next_after(None) (the chain start). A non-Origin intersect that left the cursor unset would serve block 0 to a client whose read pointer is its own tip, which the client rejects as UnexpectedBlockNo(tip_block_no + 1)(0). An Origin intersect keeps the cursor None (serve from the chain start, correct). |
| **Code** | crates/ade_network/src/chain_sync/server.rs (producer_chain_sync_serve FindIntersect handler -- sets state.last_announced from the matched intersect point) |
| **Tests** | `producer_chain_sync_serve_find_intersect_sets_cursor_then_rolls_forward_past_it` |
| **CI** | _(no CI script listed)_ |

#### `DC-PROTO-11` — _enforced_

| Aspect | Location |
|--------|----------|
| **Source** | Live server-side tx-submission2 capture (option B) finding; DC-PROTO-02; Byte Authority Model §3 |
| **Requirement** | TxSubmission2 codec accepts + byte-identically preserves cardano-node's REAL wire form for the txid/tx messages: each txId is era-tagged [eraIndex, hash32] (the HardFork GenTxId, e.g. [6, h'..'] for Conway), and the txid/tx sequences are CBOR indefinite-length arrays (9f .. ff). Decode accepts definite AND indefinite sequences; encode reproduces the indefinite form so a captured frame re-encodes byte-identically; the era tag is preserved (never stripped or guessed) so a requester echoes the exact advertised id. |
| **Code** | crates/ade_network/src/codec/tx_submission.rs (TxSubmissionTxId era-tag + decode_seq definite/indefinite + indefinite encode); corpus/network/n2n/tx_submission2/ |
| **Tests** | `codec::tx_submission::tests::roundtrip_every_variant`; `codec::tx_submission::tests::encoder_emits_indefinite_sequences`; `codec::tx_submission::tests::real_cardano_reply_txids_decodes_and_re_encodes_byte_identical`; `codec::tx_submission::tests::decode_accepts_definite_sequence_form`; `codec::tx_submission::tests::decode_rejects_bare_txid`; `codec::tx_submission::tests::decode_rejects_wrong_txid_hash_length`; `codec::tx_submission::tests::decode_rejects_unterminated_indefinite_sequence`; `tx_submission2_real_capture_corpus::real_capture_round_trips_byte_identical`; `tx_submission2_real_capture_corpus::reply_txids_entries_are_real_32_byte_txids` |
| **CI** | `ci/ci_check_tx_submission2_real_capture.sh` |

### DC-PUMP

#### `DC-PUMP-01` — _enforced_

| Aspect | Location |
|--------|----------|
| **Source** | docs/planning/phase4-n-m-c-operator-pass-invariants.md §1 (I-C6) + §2 (¬P-C3) |
| **Requirement** | Wire pump emits AdmissionPeerEvent::{Block, TipUpdate, Disconnected} only. It MUST NOT synthesize AgreementVerdict values or any validity claim. The verdict remains the GREEN reducer's sole output (memory [[feedback-evidence-reducers-are-green-not-authority]] + ¬P-C3 no RED-derived verdicts). |
| **Code** | crates/ade_runtime/src/admission/wire_pump.rs (closed AdmissionPeerEvent emit-only; no AgreementVerdict reference in code; emit helper exhaustive over the 3 variants) |
| **Tests** | `admission::wire_pump::tests::pump_emits_tip_update_and_request_next_on_intersect_found_no_block_fetch`; `admission::wire_pump::tests::pump_emits_tip_update_on_intersect_not_found`; `admission::wire_pump::tests::rollforward_drives_block_fetch_then_request_next` |
| **CI** | `ci/ci_check_admission_wire_pump_closure.sh`; `ci/ci_check_admission_no_red_verdicts.sh` |

#### `DC-PUMP-02` — _enforced_

| Aspect | Location |
|--------|----------|
| **Source** | docs/planning/phase4-n-m-c-operator-pass-invariants.md §1 (I-C7); refined for PHASE4-N-AI AI-S4a in the PHASE4-N-AN gate triage |
| **Requirement** | A CLOSED authority event is emitted on every chain-sync reply carrying a Tip: TipUpdate for IntersectFound / IntersectNotFound / RollForward; the DISTINCT AdmissionPeerEvent::RollBackward for RollBackward (PHASE4-N-AI AI-S4a -- a rollback is NEVER represented as a TipUpdate only; it is the closed fork-choice / durable-rollback signal carrying the peer's post-rollback tip). The runner's next verdict::derive / fork-choice call sees the freshest peer comparison input. (PHASE4-N-AN gate triage refined the RollBackward case to the AI-S4a event; the "closed event per reply" invariant is preserved -- the RollBackward reply's closed event is its own variant, not a generic TipUpdate.) |
| **Code** | crates/ade_runtime/src/admission/wire_pump.rs (handle_chain_sync: IntersectFound / IntersectNotFound / RollForward arms call tip_update; the RollBackward arm emits AdmissionPeerEvent::RollBackward -- the distinct AI-S4a fork-choice signal -- before any other action) |
| **Tests** | `admission::wire_pump::tests::pump_emits_tip_update_and_request_next_on_intersect_found_no_block_fetch`; `admission::wire_pump::tests::pump_emits_tip_update_on_intersect_not_found`; `admission::wire_pump::tests::rollforward_drives_block_fetch_then_request_next` |
| **CI** | `ci/ci_check_admission_wire_pump_closure.sh` |

#### `DC-PUMP-03` — _enforced_

| Aspect | Location |
|--------|----------|
| **Source** | docs/planning/phase4-n-am-wire-pump-keepalive-sustain-invariants.md + docs/clusters/PHASE4-N-AM/cluster.md |
| **Requirement** | Wire-pump keep-alive client (PHASE4-N-AM). The admission wire pump (run_admission_wire_pump -- the SOLE per-peer pump, CN-PUMP-01) runs the N2N keep-alive CLIENT (mini-protocol 8): on a cadence STRICTLY under the peer's keep-alive timeout (~97s observed) it sends KeepAliveMessage::KeepAlive(cookie) (Initiator) via the EXISTING outbound OutboundFrame path, advancing the REUSED BLUE ade_network::keep_alive state machine (keep_alive_transition: ClientIdle -> ServerHasAgency{cookie}); on the inbound MsgResponseKeepAlive(cookie') it advances the SAME state machine (ServerHasAgency{cookie} + Server -> ClientIdle, validating cookie' == cookie). WIRE-ONLY: the keep-alive client produces no canonical input, no WAL entry, and NO AdmissionPeerEvent (Block / TipUpdate / RollBackward / Disconnected) -- it never affects admission, the durable chain, fork-choice, the convergence-evidence vocabulary, replay-equivalence, or any BLUE state (the DC-PUMP-01 emit-set stays unwidened). MUST NOT: redefine the BLUE keep-alive grammar (reuse ade_network::keep_alive + ::codec::keep_alive); use a cadence >= the peer timeout; send a new MsgKeepAlive while one is in flight (respect ServerHasAgency agency); block / starve / reorder the chain-sync or block-fetch flow; dispatch a keep-alive frame as a chain-sync/block-fetch event (closed match over AcceptedMiniProtocol); silently swallow a grammar violation (fail closed via AdmissionWirePumpError::KeepAlive -- drop the peer); use rand / wall-clock for the cookie (monotonic u16 counter); implement a keep-alive SERVER/responder (client only -- the responder is a CE-AM-LIVE-gated follow-on). With the client running, a live participant AND single-producer follow sustains past the ~97s keep-alive deadline -- the prerequisite that makes the CE-AI-6 induced-reorg convergence capture runnable. SCOPE: the keep-alive client ONLY; does NOT add multi-peer ChainSel, does NOT flip CN-CONS-03, does NOT broaden CN-PUMP-01 / DC-PUMP-01 / DC-PUMP-02 (cross-refs, preserved). |
| **Code** | crates/ade_runtime/src/admission/wire_pump.rs (run_admission_wire_pump -- the keep-alive client: a tokio::select! cadence sends MsgKeepAlive Initiator under the peer timeout via the existing OutboundFrame path, advances the reused BLUE ade_network::keep_alive state machine, and validates the echoed cookie on the inbound DeliverPeerFrame{KeepAlive}; wire-only -- emits no AdmissionPeerEvent; AdmissionWirePumpError::KeepAlive fail-closed), crates/ade_network/src/keep_alive/transition.rs (BLUE keep_alive_transition -- REUSED, unchanged), crates/ade_network/src/codec/keep_alive.rs (BLUE KeepAliveMessage codec -- REUSED, unchanged) |
| **Tests** | `admission::wire_pump::tests::wire_pump_sends_keep_alive_on_quiescent_cadence`; `admission::wire_pump::tests::wire_pump_keep_alive_response_validates_cookie_no_event`; `admission::wire_pump::tests::wire_pump_keep_alive_cookie_mismatch_fails_closed` |
| **CI** | `ci/ci_check_keep_alive_wire_only.sh` |

#### `DC-PUMP-04` — _enforced_

| Aspect | Location |
|--------|----------|
| **Source** | docs/planning/phase4-n-ao-ce-ao-6-live-gap.md (S7-retry finding) + docs/clusters/PHASE4-N-AO/S8-multi-peer-wire-pump-fairness.md |
| **Requirement** | Multi-peer wire-pump fairness (PHASE4-N-AO S8; the gap the S7 live retry surfaced). When multiple peers are connected to the participant receive path, no connected peer may be STARVED by another continuously- producing peer. Each peer's run_admission_wire_pump task feeds its OWN bounded queue (per-peer channel); a fair merge over a DETERMINISTIC order DERIVED FROM the configured --peer order (an explicit Vec, never HashMap/HashSet iteration, never scheduler timing) gives each active peer bounded delivery opportunity (round-robin, one item per peer per round); backpressure is PER-PEER (a hot peer fills its own bounded queue and self-blocks), NEVER global starvation; a disconnected peer's lane is retired in place WITHOUT reordering the remaining peers. The merge order is RED scheduling discipline ONLY -- it may affect delivery OPPORTUNITY but MUST NOT decide fork-choice: select_best_chain stays arrival-order independent (CN-CONS-01), peer identity is preserved through the merge (DC-NODE-34), and the merged stream the consumer reads is unchanged in shape (one peer-attributed NodeSyncItem::Block sequence). MUST NOT: fan all peer pumps into a single shared bounded channel (the pre-S8 shape that lets a hot peer monopolise it); drop a peer's fork-choice-relevant block merely because another peer is hot; let wall-clock / rand / scheduler timing define peer priority or affect any BLUE selection result; change the selector / S7 / any BLUE authority. |
| **Code** | crates/ade_node/src/fair_merge.rs (RED per-peer bounded lanes + deterministic round-robin fair_merge: rotating cursor, closed-lane retire-in-place, no HashMap/wall-clock/rand) + crates/ade_node/src/node_lifecycle.rs (spawn_live_wire_pump_source per-peer-lane fan-in). Gate ci/ci_check_wire_pump_fairness.sh. |
| **Tests** | `hot_peer_cannot_starve_quiet_peer`; `closed_lane_removed_without_reordering_remaining_peers`; `peer_identity_preserved_through_merge` |
| **CI** | `ci/ci_check_wire_pump_fairness.sh` |

### DC-QUERY

#### `DC-QUERY-01` — _declared_

| Aspect | Location |
|--------|----------|
| **Source** | Project constitution §3, T-DET-01, DC-PROTO-04 |
| **Requirement** | N2C queries are era-aware, typed, and version-gated: each NodeToClientVersion gates which queries are available |
| **Code** | — _(declared — not yet enforced)_ |
| **Tests** | — _(declared — not yet enforced)_ |
| **CI** | — _(declared — not yet enforced)_ |

### DC-REF

#### `DC-REF-01` — _partial_

| Aspect | Location |
|--------|----------|
| **Source** | Project constitution §3, T-DET-01, T-CI-01 |
| **Requirement** | Every claimed equivalence check must identify its reference source, extraction method, and reproducibility path |
| **Code** | crates/ade_testkit/src/harness/provenance.rs |
| **Tests** | `validate_complete_manifest_no_violations`; `validate_empty_manifest_no_violations`; `validate_detects_empty_field`; `self_comparison_zero_divergences_byron`; `self_comparison_zero_divergences_shelley`; `self_comparison_zero_divergences_allegra`; `self_comparison_zero_divergences_mary`; `self_comparison_zero_divergences_alonzo`; `self_comparison_zero_divergences_babbage`; `self_comparison_zero_divergences_conway` |
| **CI** | `ci/ci_check_ref_provenance.sh` |

### DC-SEED

#### `DC-SEED-01` — _enforced_

| Aspect | Location |
|--------|----------|
| **Source** | docs/planning/phase4-n-m-ledger-seed-invariants.md §1 (I-A2) |
| **Requirement** | Canonical UtxoFingerprint determinism: the imported UTxOState uses BTreeMap<TxIn, TxOut> iteration order; UtxoFingerprint is Blake2b-256 over canonical CBOR map(N) [TxIn → TxOut] in that order. Re-importing the same JSON seed yields the same UtxoFingerprint byte-identically across runs. |
| **Code** | crates/ade_runtime/src/seed_import/importer.rs |
| **Tests** | `crates/ade_runtime/src/seed_import/importer.rs::tests::utxo_seed_two_imports_byte_identical`; `crates/ade_runtime/src/seed_import/importer.rs::tests::utxo_seed_btree_order_independent_of_json_order`; `crates/ade_runtime/src/seed_import/importer.rs::tests::utxo_seed_inline_datum_entry_round_trips`; `crates/ade_runtime/src/seed_import/importer.rs::tests::utxo_seed_reference_script_changes_fingerprint`; `crates/ade_runtime/src/seed_import/importer.rs::tests::utxo_seed_reference_script_deterministic_across_two_imports`; `crates/ade_runtime/src/seed_import/importer.rs::tests::utxo_seed_canonical_script_ref_encoder_known_vector` |
| **CI** | `ci/ci_check_seed_import_closure.sh`; `ci/ci_check_seed_import_full_preprod_support.sh` |

### DC-SERVEMEM

#### `DC-SERVEMEM-01` — _enforced_

| Aspect | Location |
|--------|----------|
| **Source** | docs/clusters/PHASE4-N-AA/cluster.md; PHASE4-N-U cross-slice security review (MEDIUM finding) |
| **Requirement** | Peer-driven serve range work is bounded. The --mode node serve path must not materialize an unbounded chain range, perform per-block full-index scans, or read more than MAX_SERVE_RANGE_BLOCKS blocks for a single peer request. Oversized ranges fail closed before unbounded storage/CPU work. The cap is a defensive implementation bound, not a Cardano semantic parameter, and cannot be disabled at runtime. |
| **Code** | crates/ade_runtime/src/chaindb/mod.rs (ChainDb trait range_bytes_capped + last_block_bytes -- S1); crates/ade_runtime/src/chaindb/types.rs (CappedSlotRange -- S1); crates/ade_runtime/src/chaindb/persistent.rs + in_memory.rs (impls, inverted-range guarded -- S1); crates/ade_runtime/src/network/served_chain_projection.rs (ChainDbServedSource range_bytes/next_after/tip use the bounded primitives + the MAX_SERVE_RANGE_BLOCKS cap + ServeRangeOutcome + derive the hash via decode_block, fail-closed over cap -- S2) |
| **Tests** | `range_bytes_capped_returns_at_most_max`; `range_bytes_capped_within_cap_not_truncated`; `range_bytes_capped_respects_bounds`; `range_bytes_capped_bytes_byte_identical`; `range_bytes_capped_inverted_range_is_empty`; `last_block_bytes_returns_highest_slot`; `serve_range_over_cap_fails_closed`; `serve_range_empty_window_is_empty_not_capexceeded`; `serve_range_undecodable_in_range_fails_closed`; `serve_range_inverted_range_fails_closed` … (+2 more) |
| **CI** | `ci/ci_check_serve_range_bounded.sh` |

### DC-SESS

#### `DC-SESS-01` — _enforced_

| Aspect | Location |
|--------|----------|
| **Source** | docs/planning/phase4-n-l-wire-protocol-invariants.md §1 (I-1) |
| **Requirement** | Handshake-before-traffic: no mini-protocol frame reaches the orchestrator inbox until the handshake state machine has emitted Accepted. Type-state: a `MuxSession` in `Handshaking` cannot emit `SessionEffect::DeliverPeerFrame`; only `Connected` can. Compile- time guarantee + runtime test. |
| **Code** | crates/ade_network/src/session/state.rs, crates/ade_network/src/session/core.rs |
| **Tests** | `crates/ade_network/src/session/core.rs::tests::session_blocks_frames_before_handshake`; `crates/ade_network/src/session/core.rs::tests::session_post_handshake_handshake_frame_is_peer_fatal` |
| **CI** | `ci/ci_check_session_core_closure.sh` |

#### `DC-SESS-02` — _enforced_

| Aspect | Location |
|--------|----------|
| **Source** | docs/planning/phase4-n-l-wire-protocol-invariants.md §1 (I-2) |
| **Requirement** | Closed mini-protocol id registry: the dispatch table over `MiniProtocolId` is a closed `match` on a closed `AcceptedMiniProtocol` enum. Unknown ids return `SessionError::UnknownMiniProtocolId { id }` (peer-fatal). No runtime extension; no silent acceptance. |
| **Code** | crates/ade_network/src/session/event.rs, crates/ade_network/src/session/core.rs |
| **Tests** | `crates/ade_network/src/session/event.rs::tests::accepted_mini_protocol_round_trips_all_ids`; `crates/ade_network/src/session/event.rs::tests::accepted_mini_protocol_unknown_id_returns_none`; `crates/ade_network/src/session/event.rs::tests::accepted_mini_protocol_match_is_exhaustive`; `crates/ade_network/src/session/core.rs::tests::session_unknown_mini_protocol_id_is_peer_fatal` |
| **CI** | `ci/ci_check_mini_protocol_id_registry_closed.sh` |

#### `DC-SESS-03` — _enforced_

| Aspect | Location |
|--------|----------|
| **Source** | docs/planning/phase4-n-l-wire-protocol-invariants.md §1 (I-3) + §3 |
| **Requirement** | Per-mini-protocol ordering + session replay equivalence: replaying the same byte chunks through `session::core::step` yields byte-identical outbound frames and an identical sequence of `SessionEffect::DeliverPeerFrame` values. Within a single (peer, mini_protocol_id) pair, frame ordering is preserved. |
| **Code** | crates/ade_network/src/session/core.rs, crates/ade_network/src/session/demux.rs |
| **Tests** | `crates/ade_network/tests/session_replay_equivalence.rs::session_replay_equivalence_holds`; `crates/ade_network/tests/session_replay_equivalence.rs::session_replay_corpus_builds_deterministically`; `crates/ade_network/src/session/demux.rs::tests::frame_buffer_two_runs_deterministic` |
| **CI** | `ci/ci_check_session_core_closure.sh` |

#### `DC-SESS-04` — _enforced_

| Aspect | Location |
|--------|----------|
| **Source** | docs/planning/phase4-n-l-wire-protocol-invariants.md §1 (I-6) |
| **Requirement** | Backpressure discipline: every per-peer + per-mini-protocol channel is bounded; queue overflow is fail-fast `TransportError::BackpressureExceeded` rather than silent drop. No `mpsc::unbounded_channel` in any wire-session / mux-pump / dialer / keep-alive-session file. |
| **Code** | crates/ade_network/src/mux/transport.rs, crates/ade_runtime/src/network/mux_pump.rs, crates/ade_runtime/src/network/n2n_dialer.rs |
| **Tests** | `crates/ade_network/src/mux/transport.rs::tests::mux_transport_duplex_inbound_overflow_returns_backpressure`; `crates/ade_network/src/mux/transport.rs::tests::mux_transport_duplex_round_trips_bytes_over_loopback` |
| **CI** | `ci/ci_check_session_no_unbounded.sh` |

#### `DC-SESS-05` — _enforced_

| Aspect | Location |
|--------|----------|
| **Source** | docs/planning/phase4-n-l-wire-protocol-invariants.md §1 (I-5) |
| **Requirement** | Wire-layer clock injection: the session reducer + dispatch table contain no SystemTime / Instant::now / tokio::time reads. Keep-alive is driven by the PHASE4-N-K Clock seam (ade_runtime::clock::Clock). Mux frame timestamps enter through caller-supplied parameters at the RED runner boundary, not the GREEN core. |
| **Code** | crates/ade_network/src/session/, crates/ade_runtime/src/orchestrator/keep_alive_session.rs |
| **Tests** | `crates/ade_runtime/src/orchestrator/keep_alive_session.rs::tests::keep_alive_session_emits_one_event_per_clock_tick`; `crates/ade_runtime/src/orchestrator/keep_alive_session.rs::tests::keep_alive_session_is_pure_under_deterministic_clock`; `crates/ade_runtime/src/orchestrator/keep_alive_session.rs::tests::keep_alive_cadence_default_is_60s` |
| **CI** | `ci/ci_check_clock_seam.sh` |

#### `DC-SESS-06` — _enforced_

| Aspect | Location |
|--------|----------|
| **Source** | docs/clusters/PHASE4-N-M-FRAG/cluster.md §1 |
| **Requirement** | Replay equivalence under fragmented inbound streams: two reducer runs over the same byte-chunk sequence (including inputs where single CBOR items span multiple mux frames) produce byte-identical `DeliverPeerFrame` lists. Truncated tails are preserved across `step()` calls until completion; malformed CBOR at an item boundary surfaces as `SessionError::ProtocolPayloadMalformed { protocol, detail }` — NEVER a silent drop, NEVER a partial accept. |
| **Code** | crates/ade_network/src/session/core.rs (drain_protocol_items + codec_error_detail) |
| **Tests** | `crates/ade_network/src/session/core.rs::tests::fragmented_replay_equivalence_two_runs_byte_identical`; `crates/ade_network/src/session/core.rs::tests::malformed_cbor_at_item_boundary_returns_session_error`; `crates/ade_network/src/session/core.rs::tests::truncated_then_complete_two_step_drain` |
| **CI** | `ci/ci_check_session_proto_reassembly.sh` |

### DC-SNAPSHOT

#### `DC-SNAPSHOT-01` — _enforced_

| Aspect | Location |
|--------|----------|
| **Source** | docs/planning/phase4-n-r-invariants.md §3 (D6); §4 (R2) |
| **Requirement** | ServedChainHandle::push_atomic is deterministic in its argument order: the same sequence of push_atomic(a₀), push_atomic(a₁), ..., push_atomic(aₙ) produces a byte-identical ServedChainView (fingerprint equals over BTreeMap insertion order). Replay-equivalence anchor for the broadcast → serve path. |
| **Code** | crates/ade_runtime/src/producer/served_chain_handle.rs (push_atomic uses send_modify with served_chain_admit — the established broadcast_to_served drain-and-admit determinism carries through); crates/ade_runtime/src/producer/broadcast_to_served.rs (existing determinism tests cover the underlying invariant; push_atomic is a thin closure around the same primitive) |
| **Tests** | `drain_and_admit_is_deterministic_over_arrival_sequence`; `drain_and_admit_no_io_no_clock`; `drain_and_admit_admits_every_queued_block` |
| **CI** | _(no CI script listed)_ |

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
| **Code** | — _(declared — not yet enforced)_ |
| **Tests** | — _(declared — not yet enforced)_ |
| **CI** | — _(declared — not yet enforced)_ |

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
| **Code** | — _(declared — not yet enforced)_ |
| **Tests** | — _(declared — not yet enforced)_ |
| **CI** | — _(declared — not yet enforced)_ |

#### `DC-STORE-07` — _enforced_

| Aspect | Location |
|--------|----------|
| **Source** | docs/planning/ledger-snapshot-rollback-invariants.md §1 (I-8) |
| **Requirement** | Snapshot cadence determinism: the decision to take a snapshot at slot S is a pure function of (slot, block_no, cadence_params, last_snapshot). Same canonical input chain history produces the same set of snapshot slot keys. Cadence is BLUE-structural; not operator-tunable in this cluster (operator-tunable cadence is out of scope until represented as anchored, replay-derivable runtime data). |
| **Code** | crates/ade_runtime/src/rollback/cadence.rs (should_snapshot_after_block pure decision; SnapshotCadence has only the every_n_blocks BLUE-structural field); crates/ade_runtime/src/rollback/in_memory_cache.rs (InMemorySnapshotCache impl SnapshotReader); crates/ade_runtime/src/rollback/chaindb_block_source.rs (ChainDbBlockSource impl BlockSource) |
| **Tests** | `should_snapshot_after_block_every_n_returns_true_at_cadence`; `should_snapshot_after_block_returns_false_off_cadence`; `should_snapshot_after_block_returns_false_when_already_at_or_after_slot`; `should_snapshot_after_block_is_pure`; `snapshot_cadence_default_is_100_blocks`; `in_memory_snapshot_cache_nearest_le_returns_largest_key`; `in_memory_snapshot_cache_iteration_is_btreemap_ordered` |
| **CI** | `ci/ci_check_snapshot_cadence_purity.sh` |

#### `DC-STORE-08` — _enforced_

| Aspect | Location |
|--------|----------|
| **Source** | docs/planning/persistent-snapshot-encoder-invariants.md §1 (I-2) |
| **Requirement** | Snapshot encoder canonicality: encode_snapshot(s) is byte-identical across runs. Encoder uses BTreeMap iteration only; no HashMap, no wall-clock, no floats, no rand. Definite-length CBOR containers. |
| **Code** | crates/ade_ledger/src/snapshot/{chain_dep,utxo_state,cert_state,epoch_state,gov_state,ledger,framing}.rs |
| **Tests** | `snapshot::chain_dep::tests::chain_dep_encode_deterministic_across_runs`; `snapshot::utxo_state::tests::utxo_state_encode_deterministic_across_runs`; `snapshot::cert_state::tests::cert_state_encode_deterministic_across_runs`; `snapshot::epoch_state::tests::epoch_state_encode_deterministic_across_runs`; `snapshot::gov_state::tests::pparams_encode_deterministic_across_runs`; `snapshot::gov_state::tests::gov_state_encode_deterministic_across_runs`; `snapshot::ledger::tests::ledger_state_encode_deterministic_across_runs`; `snapshot::framing::tests::snapshot_encode_deterministic_across_runs`; `snapshot::framing::tests::round_trip_via_fingerprint_combined` |
| **CI** | `ci/ci_check_snapshot_encoder_closure.sh` |

#### `DC-STORE-09` — _enforced_

| Aspect | Location |
|--------|----------|
| **Source** | docs/planning/persistent-snapshot-encoder-invariants.md §1 (I-3, I-4) |
| **Requirement** | Snapshot bytes carry a closed u32 version tag (initial == 1) and the source state's blake2b-256 fingerprint. Decoder reads the version tag first and rejects unknown versions before decoding the ledger or chain-dep payload; decoder recomputes the fingerprint on the decoded state and rejects on mismatch. |
| **Code** | crates/ade_ledger/src/snapshot/framing.rs |
| **Tests** | `snapshot::framing::tests::snapshot_round_trip`; `snapshot::framing::tests::decode_rejects_unknown_version`; `snapshot::framing::tests::decode_rejects_fingerprint_mismatch` |
| **CI** | `ci/ci_check_snapshot_encoder_closure.sh` |

### DC-SYNC

#### `DC-SYNC-01` — _enforced_

| Aspect | Location |
|--------|----------|
| **Source** | docs/clusters/PHASE4-N-Y/S2-network-forward-sync.md; S6-torn-write-recovery-reconciliation.md |
| **Requirement** | During network forward-sync, a block's preserved wire bytes and its WAL entry MUST be durable before the chain tip advances to it, and admission is chokepoint-only (decode -> validate_and_apply_header -> block_validity -> fork-choice). The GREEN forward-sync lifecycle reducer emits a closed SyncEffect set; AdvanceTip is constructible only after StoreBlockBytes+AppendWal (private AdmitPlan, single durable() emit site), and the RED pump fail-closes (TipBeforeDurable) on any out-of-order apply. Because tip is derived from stored blocks, recovery reconciles the chaindb to the WAL tail so a torn put_block/wal-append crash cannot incorporate an un-WAL'd orphan (S6). |
| **Code** | crates/ade_runtime/src/forward_sync/{reducer,pump}.rs; crates/ade_runtime/src/recovery/restart.rs (WAL-tail reconciliation) |
| **Tests** | `forward_sync_wal_and_bytes_precede_tip_advance`; `forward_sync_replay_two_runs_byte_identical`; `forward_sync_admission_through_chokepoints`; `recovery_torn_put_block_before_wal_append_drops_orphan`; `node_sync_pump_advances_recoverable_tip`; `node_sync_fails_closed_on_undecodable_block`; `node_sync_kill_then_warm_start_recovers_same_tip` |
| **CI** | `ci/ci_check_forward_sync_chokepoint_only.sh`; `ci/ci_check_node_sync_via_pump.sh` |

#### `DC-SYNC-02` — _enforced_

| Aspect | Location |
|--------|----------|
| **Source** | docs/planning/phase4-n-f-d-live-node-run-loop-invariants.md |
| **Requirement** | Continuous relay sync: every loop iteration preserves durable-before-advance (DC-SYNC-01) and advances the tip ONLY through run_node_sync -> pump_block. Verdict / admission / follower paths (derive_verdict, run_admission, ade_core_interop::follow) cannot drive the live tip, and there is no manual tip advance (put_block / AdvanceTip / rollback_to_slot) outside the pump. |
| **Code** | crates/ade_node/src/node_sync.rs |
| **Tests** | `relay_loop_syncs_then_halts_clean_on_source_end`; `relay_loop_idles_then_syncs_on_incremental_feed`; `relay_loop_fails_closed_on_unapplyable_block`; `node_sync_pump_advances_recoverable_tip` |
| **CI** | `ci/ci_check_node_run_loop_containment.sh`; `ci/ci_check_node_sync_via_pump.sh` |

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
| **Code** | crates/ade_testkit/src/tx_validity/ (GREEN harness: extracts every on-wire Conway tx from the committed Conway-576 corpus blocks and drives the BLUE tx_validity over each at track_utxo=false — partial scope: structural + witness closure; UTxO-dependent checks deferred); crates/ade_ledger/src/tx_validity/phase1.rs (tx_phase_one gates the UTxO-dependent state-backed checks behind track_utxo, mirroring the block path's verify_conway_witness_closure + run_phase_one_composers split — the witness closure runs unconditionally); PHASE4-B2-S3 (positive half); crates/ade_testkit/src/tx_validity/adversarial.rs + crates/ade_ledger/tests/tx_validity_adversarial_corpus.rs (NEGATIVE half: family A witness mutations on real corpus txs at track_utxo=false + family B synthetic value/input/witness mutations at track_utxo=true — every mutation → Invalid, no false accept; PHASE4-B2-S4) |
| **Tests** | `all_corpus_txs_valid`; `corpus_tx_count_nonzero`; `no_mutation_is_ever_valid`; `each_mutation_maps_to_expected_class`; `adversarial_replays_identically`; `adversarial_imbalanced_via_deposit`; `adversarial_imbalanced_via_withdrawal`; `adversarial_unknown_cert_tag_rejects_as_decode`; `adversarial_removed_tag_rejects_as_era_invalid`; `adversarial_truncated_withdrawals_rejects_as_decode` … (+7 more) |
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
| **Tests** | `all_required_covered_is_valid`; `missing_input_payment_witness_rejected`; `missing_explicit_required_signer_rejected`; `missing_withdrawal_witness_rejected`; `missing_certificate_witness_rejected`; `missing_governance_voter_witness_rejected`; `unresolvable_input_is_fail_fast`; `unresolvable_collateral_input_is_fail_fast`; `script_credential_input_not_a_vkey_signer`; `script_credential_certificate_not_a_vkey_signer` … (+3 more) |
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
| **Code** | crates/ade_ledger/src/block_validity/verdict.rs (FieldKind/FieldError closed fail-closed field taxonomy; PHASE4-B1-S3); crates/ade_core/src/consensus/kes_check.rs (expect_size fail-closed header crypto-field guard; PHASE4-B1-S5); crates/ade_testkit/src/validity/adversarial.rs (M1 truncated VRF proof, M3 flipped KES sig, M4 slot-beyond-horizon adversarial mutators exercising the fail-closed checks; PHASE4-B1-S7); crates/ade_ledger/src/tx_validity/witness.rs (wrong-size vkey/sig → MalformedWitnessField via from_bytes, never skipped); crates/ade_ledger/src/tx_validity/required_signers.rs (unresolvable input / malformed certs\|withdrawals\|voters CBOR → structured RequiredSignerError, never silent skip; PHASE4-B2-S1); crates/ade_ledger/src/conway.rs (check_conway_coin_conservation: closed coin-level preservation-of-value check wired into validate_conway_state_backed — closes the state-backed fail-open gap where outputs+fee != inputs was accepted at track_utxo=true; PHASE4-B2-S4); crates/ade_testkit/src/tx_validity/adversarial.rs (GREEN witness/value/input mutators driving tx_validity fail-closed; PHASE4-B2-S4) |
| **Tests** | `expect_size_rejects_wrong_length`; `praos_malformed_kes_sig_rejected`; `no_mutation_is_ever_valid`; `each_mutation_maps_to_expected_class`; `wrong_size_signature_rejected`; `wrong_size_vkey_rejected`; `unresolvable_input_is_fail_fast`; `unresolvable_collateral_input_is_fail_fast`; `conway_conservation_full`; `conservation_early_out_removed` … (+10 more) |
| **CI** | `ci/ci_check_consensus_closed_enums.sh` |

### DC-VIEW

#### `DC-VIEW-01` — _enforced_

| Aspect | Location |
|--------|----------|
| **Source** | docs/planning/phase4-n-m-c-operator-pass-invariants.md §1 (I-C4) |
| **Requirement** | LiveLedgerView determinism + epoch-window guard. The view is constructed deterministically from LiveConsensusInputsCanonical. Two guards on every LedgerView query: (a) Queried epoch != canonical.epoch_no → return None (BLUE then fails closed via MissingConsensusInput). (b) Block slot outside [epoch_start_slot, epoch_end_slot] → runner intercepts before admit and emits AdmissionHalted { reason: CrossEpochUse } (DC-ADMIT-11). No ambient default view; no cross-epoch silent use. |
| **Code** | crates/ade_runtime/src/consensus_inputs/view.rs (LiveLedgerView LedgerView impl — 4 epoch-window guards), crates/ade_node/src/admission/runner.rs (pre-admit slot guard before process_block, returns AdmissionExitCode::CrossEpochUse) |
| **Tests** | `consensus_inputs::view::tests::out_of_window_epoch_returns_none`; `consensus_inputs::view::tests::in_window_epoch_answers_total_active_stake`; `consensus_inputs::view::tests::in_window_per_pool_lookups_return_imported_values`; `consensus_inputs::view::tests::in_window_unknown_pool_returns_none`; `admission::bootstrap::tests::imported_window_schedule_uses_bundle_epoch` |
| **CI** | `ci/ci_check_live_ledger_view_epoch_window.sh` |

### DC-WAL

#### `DC-WAL-01` — _enforced_

| Aspect | Location |
|--------|----------|
| **Source** | docs/planning/phase4-n-m-ledger-seed-invariants.md §1 (I-A4) |
| **Requirement** | WAL is append-only by type: the WalStore trait carries no method named truncate / rewrite / replace / delete / clear. CI grep enforces across the workspace (no impl adds such methods out-of-trait). |
| **Code** | crates/ade_ledger/src/wal/store_trait.rs |
| **Tests** | _(no tests listed — gap)_ |
| **CI** | `ci/ci_check_wal_append_only.sh` |

#### `DC-WAL-02` — _enforced_

| Aspect | Location |
|--------|----------|
| **Source** | docs/planning/phase4-n-m-ledger-seed-invariants.md §1 (I-A5) |
| **Requirement** | WAL fingerprint-chain integrity: every WalEntry::AdmitBlock has prior_fp == previous entry's post_fp (or anchor's initial_ledger_fingerprint for the first entry). WalStore::verify_chain walks the WAL + asserts the chain holds. Verify failure is authority-fatal at the binary boundary. |
| **Code** | crates/ade_ledger/src/wal/store_trait.rs, crates/ade_ledger/src/wal/replay.rs |
| **Tests** | `crates/ade_runtime/src/wal/file_wal_store.rs::tests::file_wal_store_verify_chain_passes_then_catches_break`; `crates/ade_ledger/src/wal/replay.rs::tests::replay_from_anchor_catches_chain_break`; `crates/ade_runtime/tests/wal_replay_from_anchor.rs::wal_replay_from_anchor_rejects_chain_break`; `crates/ade_node/src/node_sync.rs::tests::recover_follow_kill_warm_start_chains_from_ledger_fp`; `crates/ade_node/src/node_sync.rs::tests::recover_follow_zero_seed_chainbreaks`; `crates/ade_node/src/node_sync.rs::tests::recover_follow_two_runs_byte_identical` |
| **CI** | `ci/ci_check_recover_follow_wal_lineage.sh` |

#### `DC-WAL-03` — _enforced_

| Aspect | Location |
|--------|----------|
| **Source** | docs/planning/phase4-n-m-ledger-seed-invariants.md §1 (I-A6) |
| **Requirement** | Anchor + WAL replay-equivalence: replaying (BootstrapAnchor + WAL entries 1..N) against (initial_ledger from import + per-entry block bytes) produces a final ledger whose fingerprint equals WAL[N].post_fp byte-identically across two runs. The runtime contract "same anchor + same inputs + same WAL → byte-identical outputs" is mechanically proven by integration test. |
| **Code** | crates/ade_ledger/src/wal/replay.rs, crates/ade_runtime/tests/wal_replay_from_anchor.rs |
| **Tests** | `crates/ade_runtime/tests/wal_replay_from_anchor.rs::wal_replay_from_anchor_two_runs_byte_identical`; `crates/ade_runtime/tests/wal_replay_from_anchor.rs::wal_replay_from_anchor_post_fp_matches_wal_tail`; `crates/ade_runtime/tests/wal_replay_from_anchor.rs::wal_replay_from_anchor_persists_across_reopen`; `crates/ade_ledger/src/wal/replay.rs::tests::replay_from_anchor_three_entry_chain_ok`; `crates/ade_ledger/src/wal/replay.rs::tests::replay_from_anchor_two_runs_byte_identical` |
| **CI** | _(no CI script listed)_ |

#### `DC-WAL-04` — _enforced_

| Aspect | Location |
|--------|----------|
| **Source** | docs/planning/phase4-n-u-forged-block-durability-invariants.md |
| **Requirement** | Forged-block WAL chain integrity. A forged AdmitBlock WAL entry's prior_fp MUST equal the current durable post_fp (the BootstrapAnchor's initial_ledger_fingerprint for a genesis-successor block 0; the previous entry's post_fp otherwise). A forged block that would ChainBreak (prior_fp mismatch) is rejected fail-closed (authority-fatal). The WAL binds the EXACT canonical self-accepted bytes (no re-encode; I-10). Warm-start recovery reconciles durable block storage and the WAL tail so a torn forge-admit crash leaves no un-WAL'd forged orphan at or ahead of the durable tip. |
| **Code** | crates/ade_ledger/src/wal/* (WalEntry::AdmitBlock prior_fp/post_fp + verify_chain -- reused); crates/ade_node/src/node_sync.rs (admit_forged_block_durably -> pump_block appends the forged AdmitBlock); crates/ade_node/src/node_lifecycle.rs (warm_start_recovery WAL-tail reconciliation: rollback_to_slot(wal_tail_slot) before warm-start) |
| **Tests** | `forged_admit_wal_prior_fp_chains`; `warm_start_drops_orphan_block_above_wal_tail`; `forge_tip_successor_kill_then_warm_start_recovers_block_one` |
| **CI** | `ci/ci_check_forged_durable_admit_via_pump.sh` |

#### `DC-WAL-05` — _enforced_

| Aspect | Location |
|--------|----------|
| **Source** | docs/active/durable-admission-bytes-slice.md (split-admission-authority bug, surfaced at C2-PREVIEW-BA02 when a fresh-WarmStart forge failed BlockBytesMissing on a graceful-shutdown admission store) |
| **Requirement** | Received/followed-block durable-admit is BYTES-FIRST. The live admission runner (run_admission) MUST persist an admitted block's preserved ORIGINAL bytes to the disk-backed ChainDb (ChainDb::put_block) BEFORE appending its WalEntry::AdmitBlock -- the SAME bytes-first ordering pump_block uses (DC-NODE-12, DC-WAL-04). A put_block failure halts the runner fail-closed (AdmissionExitCode::DurableBlockStoreIo, exit 36) BEFORE the WAL append, so a WAL admission record can never outlive its block bytes. Symmetrically, warm_start_recovery MUST fail closed (DurableBlockBytesMissing) when a WAL AdmitBlock's bytes are absent from the ChainDb -- corrupted durable state, NOT block absence; the prior silent skip (which masked the persistence gap behind an empty replay map) is forbidden. bytes-without-WAL stays a tolerable orphan (DC-WAL-04 reconciliation drops it); WAL-without-bytes now halts fail-closed at BOTH the write and the read. MEMORY: the live runner holds at most ONE block's bytes at a time (received -> MOVED into StoredBlock -> put_block -> dropped at the admission-step end; not cloned, not cached) and builds NO heap-resident block-bytes collection -- BA-08 owned-RSS (OP-MEM-02) must not regress. The warm-start replay map is a bootstrap-only recovery surface, never live admission. |
| **Code** | crates/ade_node/src/admission/runner.rs (run_admission ProcessedBlock::Admitted arm: StoredBlock put_block BEFORE WalEntry::AdmitBlock; AdmissionExitCode::DurableBlockStoreIo / EXIT_LIVE_DURABLE_BLOCK_STORE_IO=36 fail-closed; block bytes MOVED into StoredBlock then dropped -- no retained map); crates/ade_node/src/node_lifecycle.rs (warm_start_recovery: per-AdmitBlock ChainDb::get_block_by_hash, fail-closed NodeLifecycleError::DurableBlockBytesMissing{block_hash,entry_index,source} -- never the prior silent skip) |
| **Tests** | `warmstart_from_real_admission_store_uses_persisted_bytes_no_mock`; `warmstart_fails_closed_when_wal_admitblock_missing_bytes` |
| **CI** | `ci/ci_check_admission_runner_no_block_byte_map.sh` |

---

## OP — Operational Invariants (Project Constitution §4b)

_10 rules._

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
| **CI** | `ci/ci_check_mem_opt_s3_owned.sh`; `ci/ci_check_utxo_fp_cache.sh` |

### OP-NET

#### `OP-NET-01` — _declared_

| Aspect | Location |
|--------|----------|
| **Source** | Project constitution §4b |
| **Requirement** | Block producer connects only through trusted relay topology; no direct public peer connectivity |
| **Code** | — _(declared — not yet enforced)_ |
| **Tests** | — _(declared — not yet enforced)_ |
| **CI** | — _(declared — not yet enforced)_ |

#### `OP-NET-02` — _declared_

| Aspect | Location |
|--------|----------|
| **Source** | Project constitution §4b |
| **Requirement** | Relay paths geographically and topologically diverse; isolating one path does not prevent timely propagation |
| **Code** | — _(declared — not yet enforced)_ |
| **Tests** | — _(declared — not yet enforced)_ |
| **CI** | — _(declared — not yet enforced)_ |

#### `OP-NET-03` — _declared_

| Aspect | Location |
|--------|----------|
| **Source** | Project constitution §4b |
| **Requirement** | No single peer, ASN, region, or operator cluster dominates the node's authoritative view |
| **Code** | — _(declared — not yet enforced)_ |
| **Tests** | — _(declared — not yet enforced)_ |
| **CI** | — _(declared — not yet enforced)_ |

### OP-OPS

#### `OP-OPS-01` — _declared_

| Aspect | Location |
|--------|----------|
| **Source** | Project constitution §4b |
| **Requirement** | Post-incident reconciliation derived solely from recovered canonical chain |
| **Code** | — _(declared — not yet enforced)_ |
| **Tests** | — _(declared — not yet enforced)_ |
| **CI** | — _(declared — not yet enforced)_ |

#### `OP-OPS-02` — _declared_

| Aspect | Location |
|--------|----------|
| **Source** | Project constitution §4b |
| **Requirement** | Emergency recovery procedures have explicit admissibility criteria, deterministic inputs/outputs, and authority thresholds |
| **Code** | — _(declared — not yet enforced)_ |
| **Tests** | — _(declared — not yet enforced)_ |
| **CI** | — _(declared — not yet enforced)_ |

#### `OP-OPS-03` — _declared_

| Aspect | Location |
|--------|----------|
| **Source** | Project constitution §4b |
| **Requirement** | Incident evidence sufficient to reconstruct canonical decision path without relying on nondeterministic logs |
| **Code** | — _(declared — not yet enforced)_ |
| **Tests** | — _(declared — not yet enforced)_ |
| **CI** | — _(declared — not yet enforced)_ |

#### `OP-OPS-04` — _enforced_

| Aspect | Location |
|--------|----------|
| **Source** | docs/planning/phase4-n-c-invariants.md §7 (OQ1); docs/active/op-ops-04-ade-native-kes-flow.md; docs/planning/phase4-n-p-invariants.md |
| **Requirement** | Operator-supplied keys. Ade supports both KES key flows: (a) Ade-native `ade_node --mode key_gen_kes --out-file PATH` emitting an `ade.kes.seed.v1` envelope loaded via `ade_runtime::producer::keys::load_ade_kes_signing_key`; (b) cardano-cli `node key-gen-KES` emitting a 608-byte `KesSigningKey_ed25519_kes_2^6` envelope loaded via `ade_runtime::producer::keys::load_kes_signing_key_skey`. After PHASE4-N-P S5 the cardano-cli path routes through the Ade-owned BLUE deserializer (`ade_crypto::kes_sum::Sum6Kes::raw_deserialize_signing_key_kes`); both flows produce a `KesSecret` whose inner is `ade_crypto::kes_sum::Sum6Kes::SigningKey` (BLUE-owned algorithm). VRF and cold (Ed25519) keys continue to be operator-supplied via cardano-cli text-envelope `.skey` files. Private-key material never crosses into BLUE — the entire RED custody surface lives under `crates/ade_runtime/src/producer/{keys,signing,ade_kes_envelope}.rs`. Wrong-size payloads fail-close via `KeyLoadError::UnsupportedExpandedKesKeyFormat`; structurally-invalid 608-byte payloads fail-close via `KeyLoadError::KesParse(KesParseError::*)`. Mechanical enforcement: ci/ci_check_private_key_custody.sh + ci/ci_check_kes_envelope_closed.sh + ci/ci_check_kes_sum_compatibility.sh. OP-OPS-04-KES-PERIOD-ANCHOR (public-venue hardening): at producer-shell init the opcert.kes_period is the ABSOLUTE KES period the key's evolution-0 is certified for; the shell verifies the INJECTED current absolute period (derived from the genesis KES anchor + the durable tip slot, NEVER the raw key evolution index) is within [opcert_start, opcert_start+63], then anchors evolution-0 at opcert_start and evolves the key by (current - opcert_start) so it signs at the current period -- fail-closed KesPeriodBelowOpCertStart / KesPeriodPastOpCertEnd / KesEvolutionFailed outside the window. Signing stays RED/shell; opcert verification stays BLUE/core; no wall-clock in the deterministic shell (the period is injected). |
| **Code** | crates/ade_runtime/src/producer/keys.rs (load_*_signing_key_skey, load_ade_kes_signing_key, write_ade_kes_envelope, KeyLoadError); crates/ade_runtime/src/producer/ade_kes_envelope.rs (closed envelope grammar); crates/ade_runtime/src/producer/signing.rs (RED-confined custody, KesSecret with BLUE-owned inner); crates/ade_crypto/src/kes_sum/ (BLUE Sum6KES algorithm + serde + ground-truth corpus); crates/ade_node/src/key_gen.rs (one-shot key-gen-KES surface); crates/ade_runtime/src/producer/producer_shell.rs (ProducerShell::init: opcert-window check on the INJECTED current absolute period + anchor evolution-0 at opcert_start + kes_update to current-opcert_start; KesEvolutionFailed); crates/ade_node/src/operator_forge.rs (build_operator_forge_material derives the current period from the injected current_slot + genesis KES anchor); crates/ade_node/src/node_lifecycle.rs (injects the recovered durable tip slot as the forge KES anchor) |
| **Tests** | `ade_envelope_round_trips_through_loader_at_period_0`; `ade_envelope_loader_returns_kes_at_loaded_period`; `ade_envelope_loader_rejects_signing_at_past_period`; `cardano_cli_kes_envelope_rejects_32_byte_payload`; `cardano_cli_kes_envelope_rejects_synthetic_608_byte_payload`; `cardano_cli_kes_envelope_accepts_real_608_byte_payload`; `cardano_cli_kes_envelope_rejects_612_byte_payload`; `cardano_cli_kes_envelope_rejects_608_byte_leaf_zero_payload`; `ade_envelope_loader_returns_unknown_format`; `ade_envelope_loader_returns_wrong_role` … (+18 more) |
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

### RO-CLOSE

#### `RO-CLOSE-01` — _enforced_

| Aspect | Location |
|--------|----------|
| **Source** | docs/clusters/completed/PHASE4-N-V/CLOSURE.md; user-direction (PHASE4-N-V close correction) |
| **Requirement** | Unmasked close-gate discipline. Any slice that changes canonical bytes, encoded forms, decoder inputs, or golden fixtures MUST run an UNMASKED full close gate (cargo test --workspace) and use cargo's REAL exit status / result line as the sole pass/fail authority for cluster closure. Piped output (\| tail, \| grep) may be used for display only, NEVER as the pass/fail authority — a pipeline's exit code is the last stage's, not cargo's. Before closure, ALL consumers of the changed canonical output (every golden fixture, decoder, re-encode, and byte-identity test, across all crates — not just the edited crate) MUST be grepped/audited. A cluster is not closed until the unmasked close gate exits successfully. |
| **Code** | process/release discipline — no BLUE code locus; binds /cluster-close and any slice touching canonical output |
| **Tests** | _(no tests listed — gap)_ |
| **CI** | _(no CI script listed)_ |

### RO-GENESIS-REPLAY

#### `RO-GENESIS-REPLAY-01` — _declared_

| Aspect | Location |
|--------|----------|
| **Source** | docs/planning/phase4-n-m-ledger-seed-invariants.md §0 honest-scope table |
| **Requirement** | Ade independently replays the chain from byron genesis through the bootstrap point P, producing the same UTxOState the oracle seed provides. Closes the "Ade has independently replayed genesis → P" honest-scope claim left open at PHASE4-N-M-A close. Carried as an open obligation because the era-transition sequence (byron → shelley → allegra → mary → alonzo → babbage → conway) requires multi-month work that is NOT bounty-critical. Mithril-authenticated import (RO-MITHRIL-IMPORT-01) is a partial alternative. |
| **Code** | — _(declared — not yet enforced)_ |
| **Tests** | — _(declared — not yet enforced)_ |
| **CI** | `ci/ci_check_genesis_replay_open_obligation.sh` |

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
| **Requirement** | A cardano-node peer's RollForward + BlockDelivered stream, consumed by the receive bridge, produces a ChainDb tip equal to the peer's announced tip at every step over a captured follow window. Live evidence captured against a private cardano-node peer; underlying invariants are CN-CONS-08, DC-CONS-19, DC-CONS-20, DC-PROTO-09, and the existing block_validity (B1) authority. |
| **Code** | crates/ade_runtime/tests/receive_pipeline_corpus_drive.rs (mechanical adapter — CE-N-H-5); crates/ade_core_interop/src/bin/live_block_follow_session.rs (operator-action binary — CE-N-H-6); docs/clusters/completed/PHASE4-N-H/CE-N-H-6_PROCEDURE.md (operator procedure) |
| **Tests** | `receive_pipeline_corpus_drive_admits_every_block`; `receive_pipeline_corpus_drive_chaindb_tip_matches_expected`; `receive_pipeline_corpus_drive_admitted_bytes_equal_corpus_bytes`; `receive_pipeline_corpus_drive_ledger_fingerprint_changes_on_admit`; `live_block_follow_session_hermetic_default_prints_readiness` |
| **CI** | `ci/ci_check_receive_paths_corpus_present.sh` |

#### `RO-LIVE-03` — _declared_

| Aspect | Location |
|--------|----------|
| **Source** | docs/planning/phase4-n-l-wire-protocol-invariants.md §8 |
| **Requirement** | Live tip-following pass: operator runs `ade_node --peer ADDR` against a private cardano-node peer, captures a 30-minute JSONL log of (peer_tip, ade_tip, agreement_verdict) per admitted block, attaches the log to the cluster doc. Cross-impl evidence that cardano-node's chain and Ade's chain agree under real-wire conditions. |
| **Code** | — _(declared — not yet enforced)_ |
| **Tests** | — _(declared — not yet enforced)_ |
| **CI** | — _(declared — not yet enforced)_ |

#### `RO-LIVE-04` — _enforced_

| Aspect | Location |
|--------|----------|
| **Source** | docs/planning/phase4-n-l-live-wire-smoke-invariants.md §1 |
| **Requirement** | Live wire-smoke pass: operator runs `ade_node --mode wire_only --peer ADDR --network NAME` against a private cardano-node peer. The binary opens TCP, completes the N2N handshake, issues one chain-sync FindIntersect(Origin), reads the peer's announced tip, emits a closed-vocabulary JSONL log of the exchange, and exits cleanly. The JSONL event vocabulary is closed: only node_started / peer_dial_started / handshake_ok / peer_tip_read / peer_dial_failed / wire_smoke_complete / node_shutdown variants are emitted. The wire-only mode MUST NOT emit agreement_verdict / admitted_block / ledger_applied / projection_updated (CI grep enforces). The pass closes the wire-liveness half of RO-LIVE-03. |
| **Code** | crates/ade_node/src/wire_only.rs, crates/ade_node/src/live_log/event.rs, crates/ade_node/src/live_log/writer.rs, crates/ade_node/src/main.rs |
| **Tests** | `crates/ade_node/tests/wire_only_loopback.rs::main_wire_only_exits_zero_after_tip_read`; `crates/ade_node/tests/wire_only_loopback.rs::main_wire_only_emits_peer_tip_read_with_responder_tip`; `crates/ade_node/tests/wire_only_loopback.rs::main_wire_only_never_emits_agreement_verdict`; `crates/ade_node/tests/wire_only_loopback.rs::main_without_genesis_does_not_attempt_admission`; `crates/ade_node/tests/wire_only_loopback.rs::peer_dial_failure_exits_nonzero_with_error_event`; `crates/ade_node/tests/wire_only_loopback.rs::admission_mode_fails_closed_with_ledger_seed_unavailable`; `crates/ade_node/tests/wire_only_loopback.rs::jsonl_events_are_valid_one_object_per_line`; `crates/ade_node/src/live_log/writer.rs::tests::live_log_writer_emits_one_object_per_line`; `crates/ade_node/src/live_log/writer.rs::tests::live_log_writer_serializes_node_started_canonically`; `crates/ade_node/src/live_log/writer.rs::tests::live_log_writer_two_runs_are_byte_identical` … (+1 more) |
| **CI** | `ci/ci_check_wire_only_event_vocabulary_closed.sh`; `ci/ci_check_wire_only_no_bootstrap.sh` |

#### `RO-LIVE-05` — _enforced_

| Aspect | Location |
|--------|----------|
| **Source** | docs/planning/phase4-n-l-live-wire-smoke-invariants.md §1 |
| **Requirement** | Live admission-agreement pass: operator runs `ade_node` against a private cardano-node peer with admission enabled (bootstrap loads a real initial ledger state from either a genesis-bundle seed or an imported cardano-node ledger snapshot), follows the peer's chain, admits blocks via the single admit authority (CN-CONS-08), and emits a JSONL log of (peer_tip, ade_tip, agreement_verdict) per admitted block over a 30-minute window. Closes the admission/agreement half of RO-LIVE-03. |
| **Code** | crates/ade_node/src/admission/ (full admission stack), crates/ade_runtime/src/admission/wire_pump.rs (live wire pump), crates/ade_runtime/src/consensus_inputs/ (operator-supplied LiveConsensusInputs import authority + canonical fingerprint), crates/ade_runtime/src/seed_import/ (full preprod UTxO importer; PHASE4-N-M-A1.1 closure includes reference-script + Byron Base58 + Plutus-integer-tolerant field skipping), docs/evidence/phase4-n-m-* (live transcripts + bundles + runbook) |
| **Tests** | `live_operator_pass_against_docker_preprod`; `live_bundle_imports_with_conway_era_and_deterministic_fingerprint`; `cross_epoch_block_triggers_halt_without_admit`; `adversarial_corpus_rejects_all_four_mutation_classes` |
| **CI** | `ci/ci_check_live_operator_pass_scaffold.sh` |

#### `RO-LIVE-06` — _enforced_

| Aspect | Location |
|--------|----------|
| **Source** | docs/clusters/PHASE4-N-F-C/cluster.md; L6-peer-acceptance-evidence-manifest.md |
| **Requirement** | BA-02 peer-acceptance evidence closure (SCHEMA + CORRELATION MECHANICS ONLY). The BA-02 evidence surface is a closed, versioned manifest (Ba02Manifest) plus a pure, total, deterministic, HASH-PRIMARY correlator: correlate is the SOLE Ba02Manifest constructor; the forged-block hash is the REQUIRED correlation key; a peer-accept signal's slot is OPTIONAL context that must AGREE when present (a present-but-different slot contradicts -> NoEvidence); conflicting peer signals -> NoEvidence; and the peer-accept-log parser is allow-list only (peer_served_block / peer_chain_tip), dropping every weaker/self/unknown/ malformed line (ForgeSucceeded / self_accept / block_received / agreement_verdict are never coerced to acceptance). The forged evidence uses only forge-event- exposed fields (ForgedBlockArtifact.{hash,slot}); the hash is never recomputed and block bytes are never parsed. ENFORCED for the schema + correlation mechanics; NOT a claim that BA-02 was achieved live. |
| **Code** | crates/ade_node/src/ba02_evidence.rs (Ba02Manifest, BA02Outcome, PeerAcceptEvent, NoEvidenceReason, parse_peer_accept_events, correlate); crates/ade_node/src/ba02_pass.rs (PHASE4-N-F-G-C: RED operator-pass evidence I/O — correlate_peer_log_file reads the operator-captured peer log into the GREEN correlate; write_ba02_manifest accepts ONLY a Ba02Manifest); ci/ci_check_ba02_evidence_closed.sh; ci/ci_check_ba02_evidence_manifest_schema.sh |
| **Tests** | `ba02_manifest_schema_round_trips`; `ba02_correlate_served_block_yields_manifest`; `ba02_correlate_chain_tip_only_yields_manifest`; `ba02_correlate_both_signals_agree_records_served_primary`; `ba02_correlate_served_block_without_slot_yields_manifest`; `ba02_correlate_conflicting_signals_is_no_evidence`; `ba02_correlate_wrong_hash_is_no_evidence`; `ba02_correlate_chain_point_mismatch_is_no_evidence`; `ba02_correlate_no_slot_wrong_hash_is_no_evidence`; `ba02_correlate_stale_log_is_no_evidence` … (+10 more) |
| **CI** | `ci/ci_check_ba02_evidence_closed.sh`; `ci/ci_check_ba02_evidence_manifest_schema.sh` |

### RO-MITHRIL-IMPORT

#### `RO-MITHRIL-IMPORT-01` — _enforced_

| Aspect | Location |
|--------|----------|
| **Source** | docs/planning/phase4-n-m-ledger-seed-invariants.md §10 carry-forward; PHASE4-N-Y S1/S7 |
| **Requirement** | Ade imports a Mithril-authenticated snapshot as an alternative to the cardano-cli JSON seed. Provides cryptographic provenance for the seed artifact (over and above the Blake2b-256 seed_artifact_hash that CN-ANCHOR-01 records). Lower priority than RO-LIVE-05; resolves after PHASE4-N-M-C closes. |
| **Code** | crates/ade_ledger/src/bootstrap_anchor/binding.rs (verify_mithril_binding + MithrilManifestReport); crates/ade_runtime/src/mithril_import/ (manifest importer); crates/ade_runtime/src/mithril_bootstrap.rs (PHASE4-N-Z production composition) |
| **Tests** | `mithril_binding_rejects_certified_point_other_than_seed_point`; `mithril_anchor_rejects_field_mismatch`; `mithril_import_fail_closed_blocks_storage_init`; `mithril_bootstrap_fails_closed_on_seed_point_mismatch` |
| **CI** | `ci/ci_check_mithril_uses_bootstrap_initial_state.sh`; `ci/ci_check_mithril_seed_point_independence.sh`; `ci/ci_check_mithril_documented_evidence.sh` |

### RO-REL

#### `RO-REL-01` — _declared_

| Aspect | Location |
|--------|----------|
| **Source** | Project constitution §4a, DC-LEDGER-07, T-DET-01 |
| **Requirement** | Release not mainnet-eligible without mixed-version topology consensus equivalence on adversarial inputs |
| **Code** | — _(declared — not yet enforced)_ |
| **Tests** | — _(declared — not yet enforced)_ |
| **CI** | — _(declared — not yet enforced)_ |

#### `RO-REL-02` — _declared_

| Aspect | Location |
|--------|----------|
| **Source** | Project constitution §4a, DC-LEDGER-03, T-DET-01 |
| **Requirement** | Cross-implementation accept/reject agreement on authoritative corpora is release-blocking |
| **Code** | — _(declared — not yet enforced)_ |
| **Tests** | — _(declared — not yet enforced)_ |
| **CI** | — _(declared — not yet enforced)_ |

#### `RO-REL-03` — _declared_

| Aspect | Location |
|--------|----------|
| **Source** | Project constitution §4a |
| **Requirement** | No single implementation bug should exceed the protocol's intended safety or liveness fault threshold at ecosystem level |
| **Code** | — _(declared — not yet enforced)_ |
| **Tests** | — _(declared — not yet enforced)_ |
| **CI** | — _(declared — not yet enforced)_ |

### RO-SYNC-EVIDENCE

#### `RO-SYNC-EVIDENCE-01` — _partial_

| Aspect | Location |
|--------|----------|
| **Source** | docs/clusters/PHASE4-N-Y/S5-compatibility-evidence.md |
| **Requirement** | A committed snapshot->tip sync-evidence manifest carries the closed schema (oracle versions, chain point, fixture refs, sha256, diff/acceptance result) and its sha256 cross-checks the committed fixture; the gate is vacuously satisfied until a manifest is committed (mirrors CN-OPERATOR-EVIDENCE-01). Each discovered Haskell mismatch becomes a named regression fixture under corpus/sync/regressions/. The two-Haskell- node private-Conway-testnet live leg is operator-witnessed. |
| **Code** | ci/ci_check_sync_evidence_manifest_schema.sh; corpus/sync/regressions/; crates/ade_testkit/src/harness/sync_diff.rs |
| **Tests** | `regression_fixture_per_mismatch` |
| **CI** | `ci/ci_check_sync_evidence_manifest_schema.sh` |

### RO-TEST

#### `RO-TEST-01` — _declared_

| Aspect | Location |
|--------|----------|
| **Source** | Project constitution §4a, T-DET-01, DC-LEDGER-07 |
| **Requirement** | Consensus-relevant inputs fuzzed differentially across all supported versions; any verdict mismatch is release-blocking |
| **Code** | — _(declared — not yet enforced)_ |
| **Tests** | — _(declared — not yet enforced)_ |
| **CI** | — _(declared — not yet enforced)_ |

#### `RO-TEST-02` — _declared_

| Aspect | Location |
|--------|----------|
| **Source** | Project constitution §4a, T-CI-01 |
| **Requirement** | Every fork/mismatch/parser disagreement that ever occurred becomes a permanent regression corpus entry |
| **Code** | — _(declared — not yet enforced)_ |
| **Tests** | — _(declared — not yet enforced)_ |
| **CI** | — _(declared — not yet enforced)_ |

#### `RO-TEST-03` — _declared_

| Aspect | Location |
|--------|----------|
| **Source** | Project constitution §4a, T-DET-01, T-REC-01 |
| **Requirement** | Failed, duplicate, and boundary-case inputs remain verdict-stable under resubmission and replay |
| **Code** | — _(declared — not yet enforced)_ |
| **Tests** | — _(declared — not yet enforced)_ |
| **CI** | — _(declared — not yet enforced)_ |

---
## Deprecated rules

_None._ The registry has **0** deprecated entries at HEAD (`ci_check_registry_unique_ids.sh` enforces ID uniqueness; IDs are append-only and never reused). If a rule is ever deprecated it keeps its ID and moves here with an explicit `deprecated_in`.

---

## Enforcement gaps (need attention)

These are surfaced for human judgment. The registry is **human-curated** — this pass does **not** auto-flip statuses or invent rule IDs; it reports what the join reveals.

### G1 — LIVE-FORGE-HARDENING **S1** forge-path rollback-follow is unbound in the registry (INV-FH-1..4)

The HEAD cluster (LIVE-FORGE-HARDENING, close `1e4896eb`) shipped its **S1** forge-path rollback-follow authority with **no registry binding**:

- The rollback-follow family S1 *reuses and widens to the forge path* — **`DC-NODE-23`, `DC-NODE-27`, `DC-NODE-28`, `DC-NODE-29`, `DC-NODE-33`** — does **not** list `LIVE-FORGE-HARDENING` / `LIVE-FORGE-HARDENING-S1` in `strengthened_in` (verified: their `strengthened_in` arrays stop at `PHASE4-N-AO` / `CN-FOLLOW-01` / empty).
- **INV-FH-4** (the new *within-epoch* forge-path guard — the forge path follows a rollback only if its target is at/after the promoted authority's epoch-start slot) has **no rule ID at all**. The cluster doc's "**DC-NODE-3x** candidate" (`CLUSTER-LIVE-FORGE-HARDENING.md` §Registry) was never appended.
- **Code (present, unbound):** `ade_node::node_lifecycle::resolve_and_apply_peer_rollback` (`node_lifecycle.rs:5173`) — the shared authority; the forge-path rewire calls it at `node_sync.rs:668`.
- **Tests (partial):** `crates/ade_node/tests/live_fork_choice_ai_s4bii.rs` has **4 of the 5** S1 tests — `forge_path_rollback_applies_durably`, `forge_path_rollback_to_unknown_point_fails_closed`, `forge_path_rollback_beyond_k_fails_closed_clears_pending`, `forge_path_rollback_slot_hash_mismatch_fails_before_mutation`. The **5th** test the `SLICE-S1-forge-path-rollback-follow.md` §Tests names for INV-FH-4 — **`forge_path_rollback_across_epoch_start_fails_closed`** — is **absent from the entire codebase** (verified: 0 hits). So INV-FH-4 has neither a rule ID **nor** its named proof.
- **CI:** S1 shipped no standalone `ci_check` (it reuses the rollback-materialize + `ci_check_live_fork_choice_apply.sh` / `ci_check_wal_rollback_replay_equiv.sh` gates that back DC-NODE-27/29).

**Contrast — S2 landed correctly:** `DC-EPOCH-16` carries `strengthened_in += LIVE-FORGE-HARDENING-S2` and gained two tests at the close. Only S1 is unbound.

**Recommended fix (for the human):** append `strengthened_in += "LIVE-FORGE-HARDENING-S1"` to `DC-NODE-27` and `DC-NODE-28` (the no-forge-across-pending-reselection fence S1 now exercises from the single-producer loop) and to `DC-NODE-23`/`DC-NODE-29`/`DC-NODE-33` as the rollback-follow family widened to the forge path; and **either** add a new **`DC-NODE-3x`** for INV-FH-4 (the within-epoch forge-path guard) **or** fold it into `DC-NODE-29` (canonical rollback-target binding). Land the missing `forge_path_rollback_across_epoch_start_fails_closed` test so INV-FH-4 has a proof before its rule is marked enforced.

### G2 — Two invariants with **no registry rule naming the file** (unbound `code_locus`)

| Invariant / module | Cluster | Shipping CI (green, but rule-unbound) | Suggested |
|---|---|---|---|
| `ade_ledger::reduced_boundary` (`ReducedBoundaryProjection`, the typed non-authority reduced plane) | REDUCED-VALIDATION-BOUNDARY-PLANE (**I-RVB-\***) | `ci/ci_check_reduced_boundary_plane.sh`, `ci/ci_check_trusted_replay_boundary.sh` — **both referenced by no registry rule** | new family `DC-RVB-*` binding I-RVB-1..N to `reduced_boundary.rs` |
| `ade_ledger::rollback::admission` (`admit_rollback`, the S5 recovery-admission rail) | LIVE-LEDGER-EPOCH-TRANSITION **S5** | (covered by rollback-materialize gates; no admission-specific gate) | new rule for the recovery-admission rail (fail-closed target admissibility; k-block-bound; not fork-choice) |

Both files are pure BLUE with module banners + fail-closed tests, but no `code_locus` in the registry points at them — so they appear in **no** rule's Code row. CODEMAP independently surfaces the same two (its `ade_ledger` "Gap (registry traceability)" note + Generation-notes §1–2).

### G3 — `declared`-but-CI-gated (registry status lags shipped enforcement)

Seven rules are registry-status **`declared`** yet ship a **green CI gate** (verified on disk + referenced by the rule). Per the project pattern (also `DC-EPOCH-11` / `DC-EVIEW-08`, both gated-and-declared) the `enforced` flip is *owed pending a committed live-flip transcript* — not a defect, but the status field should be reconciled:

| Rule | Status | Shipping CI gate | What the gate checks |
|---|---|---|---|
| `DC-EPOCH-19` | declared | `ci/ci_check_epoch_accumulator_no_utxo.sh` | `EpochAccumulator` is non-UTxO; `apply_selected_block` self-sustains |
| `DC-EPOCH-20` | declared | `ci/ci_check_epoch_accumulator_recovery.sh` | durable accumulator advances observe-only, seals, reconciles (warm-start + reorg) |
| `DC-EPOCH-21` | declared | `ci/ci_check_poolreap_single_canonical.sh` | exactly one canonical POOLREAP at the boundary; per-credential mark |
| `DC-EPOCH-22` | declared | `ci/ci_check_boundary_aligned_mark_capture.sh` | boundary-aligned co-advancer + durable `BoundaryMark` witness bound before the cross |
| `DC-EPOCH-25` | declared | `ci/ci_check_frozen_leadership_authority.sh`, `ci/ci_check_frozen_promotion_no_seed_window.sh`, `ci/ci_check_frozen_recovery_no_seed_window.sh` | `FrozenLeadershipPoolDistr` is the self-contained leadership authority |
| `DC-GOV-01` | declared | `ci/ci_check_gov_proposal_capture.sh` | live gov proposals + votes captured into tracked proposals' vote maps |
| `DC-CINPUT-07` | declared | `ci/ci_check_conway_deposit_params_bootstrap.sh` | snapshot-bound Conway deposit params imported into the native-Mithril bootstrap authority |

(Registry-wide, HEAD_DELTAS §7 counts 10 of the 52 new rules landing `declared`; `DC-EPOCH-17` is `declared`-**and-ungated**, i.e. genuinely not-yet-enforced, and is *not* in this table.)

### G4 — Load-bearing rules with an empty load-bearing cell (lower severity)

- **12** `enforced`/`partial`/`enforced_scaffolding` rules carry an **empty Tests** row (shown `_(no tests listed — gap)_`): `T-BUILD-01`, `T-BOUND-02`, `T-CI-01`, `DC-CRYPTO-02`, `OP-MEM-01`, `CN-STORE-08`, `DC-WAL-01`, `DC-ADMIT-09`, `DC-OUTBOUND-FIFO-01`, `CN-OPERATOR-EVIDENCE-01`, `RO-CLOSE-01`, `DC-EVIDENCE-03`. Several are legitimately structural / CI-text-checks (e.g. `T-BUILD-01`, `T-CI-01`, `CN-STORE-08`) where a unit test is not the enforcement mechanism; the rest are candidates for a named proof.
- **23** `enforced` rules carry **no CI** row and rely on tests + BLUE types alone (e.g. `DC-PROTO-10`, `DC-NODE-31/32/33`, `T-REC-05`, `DC-CINPUT-05`, `DC-PROD-01..03`, `DC-FORGE-01`). Acceptable where a unit test is the mechanical enforcement, but each is a candidate for a defense-in-depth CI grep.

---

## Cross-reference checks (vs CODEMAP / HEAD_DELTAS / registry)

All checks below are static (no build / no test run). They generate warnings, not blocks.

### vs CODEMAP (`docs/ade-CODEMAP.md`, same HEAD `1e4896eb`) — PASS

- All **11** crates named in any `code_locus` (`ade_codec`, `ade_types`, `ade_crypto`, `ade_core`, `ade_plutus`, `ade_ledger`, `ade_network`, `ade_runtime`, `ade_node`, `ade_core_interop`, `ade_testkit`) are documented in CODEMAP.
- Every `crates/**/*.rs` path appearing in a Code row **resolves on disk** (0 dangling Code references).
- CODEMAP independently surfaces the **same** three registry-traceability gaps (G1, G2) in its `ade_ledger` / `ade_node` "Gap" notes and Generation-notes §1–4 — the two docs agree.

### vs HEAD_DELTAS §5 (`docs/ade-HEAD_DELTAS.md`, same HEAD, CI Checks)

- **16 of the 17** new gates listed in HEAD_DELTAS §5 are referenced by ≥1 registry rule's CI row. The **2 exceptions** — `ci_check_reduced_boundary_plane.sh` and `ci_check_trusted_replay_boundary.sh` — are referenced by **no** rule; they are the **G2** RVBP unbound-invariant gates (consistent across both docs).
- **18 gate scripts (of 255) are referenced by no registry rule.** Classified:
  - **2 — RVBP unbound invariant (G2):** `ci_check_reduced_boundary_plane.sh`, `ci_check_trusted_replay_boundary.sh`.
  - **6 — documented OWED dormant UTxO-disk-B gates** (`.idd-config.json` states these "are not yet bound to a registry rule; they attach to `DC-MEM-05/06` when B lands"): `ci_check_utxo_disk_anchor.sh`, `ci_check_utxo_disk_key.sh`, `ci_check_utxo_pre_resolve.sh`, `ci_check_utxo_admission_seam.sh`, `ci_check_mem_opt_utxo_disk_s0.sh`, `ci_check_mem_diag_quarantine.sh`.
  - **~6 — cross-cutting / meta gates** (enforce a whole-workspace property, not one rule ID; unbound-by-a-single-rule is expected): `ci_check_module_headers.sh`, `ci_check_no_secrets.sh`, `ci_check_pallas_quarantine.sh`, `ci_check_registry_unique_ids.sh`, `ci_check_mem_compare_evidence.sh`, `ci_check_mem_opt_s1_reduction.sh`.
  - **4 — candidates for a missing rule binding** (a concrete invariant with a gate but no rule): `ci_check_single_producer_loop_continuation.sh` (fits `DC-NODE-19`, currently declared), `ci_check_wire_rollback_signal_preserved.sh`, `ci_check_live_blockfetch_byte_only.sh`, `ci_check_live_transcript_forge_base.sh`.
- The reverse direction ("every rule-cited CI script appears in HEAD_DELTAS §5") is **not applicable**: §5 is a *delta* (55 `A` / 2 `M` since baseline `470f9b89`), not a full catalog of all 255 gates; the ~238 pre-baseline gates are correctly absent from §5.

### vs the registry itself

- **CI:** every `.sh` named in any rule's `ci_script` / `ci_scripts` **exists on disk** (0 stale CI references).
- **Code:** every `crates/**/*.rs` in any `code_locus` **exists on disk** (0 stale code references).
- **Tests — 22 stale test-fn names across 5 rules** (flagged inline with **†**). The registry names test functions that are **absent at HEAD**; the *enclosing test modules exist* — the fn names were **renamed by the in-flight ECA-B rolling-nonce reshape** and the registry `tests` arrays were not updated:
  - `DC-CONS-04` (10): e.g. `header_contribution_advances_evolving_nonce_deterministically` → code has `header_contribution_advances_evolving_sets_lab_deterministically`; `epoch_boundary_rejects_uninitialised_candidate` → `epoch_boundary_rejects_uninitialised`; `epoch_boundary_preserves_op_cert_counters` → `..._and_block_no`.
  - `DC-EPOCH-16` (9): `bridge_equivalence_seeded_snapshot_tick_reproduces_eca5_eta0`, `candidate_freezes_at_freeze_boundary` (→ `header_contribution_freezes_candidate_at_freeze_boundary`), the `epoch_tick_*` + `chain_dep_array10_*` + `b1_store_round_trip_*` set — none present at HEAD under those names.
  - `DC-EVIEW-05` (1): `delegated_zero_stake_pool_is_included_with_zero` — the surviving test is `delegated_zero_stake_pool_is_omitted` (`reduced_aggregate.rs`); the rename **inverts the asserted semantics** (included-with-zero → omitted) and deserves a human look, not a mechanical rename.
  - `DC-EVIEW-08` (1): `epoch_boundary_consumes_precomputed_aggregate_mark` — no surviving fragment at HEAD.
  - `DC-MITHRIL-07` (1): `native_first_run_missing_shelley_genesis_is_terminal` — code has `native_first_run_missing_manifest_is_terminal` / `native_first_run_missing_genesis_and_unknown_network_is_terminal`.
  This cluster of stale names is expected given the ECA-B reshape is the same in-flight work behind the 4 KNOWN `consensus_stream_replay` failures; refreshing these `tests` arrays is owed when that corpus is regenerated.

---

## Open questions for the user

1. **Bind LIVE-FORGE-HARDENING S1?** Append `strengthened_in += "LIVE-FORGE-HARDENING-S1"` to `DC-NODE-23/27/28/29/33`, and give **INV-FH-4** a home — a new `DC-NODE-3x` or a fold into `DC-NODE-29`? And should the missing `forge_path_rollback_across_epoch_start_fails_closed` proof be landed before INV-FH-4 is called enforced? (G1)
2. **Create rules for the two unbound BLUE modules?** A `DC-RVB-*` family for `reduced_boundary` (I-RVB-\*) that also claims `ci_check_reduced_boundary_plane.sh` + `ci_check_trusted_replay_boundary.sh`, and a rule for `rollback::admission` (S5 `admit_rollback`)? (G2)
3. **Flip the seven `declared`-but-gated rules to `enforced` now** (`DC-EPOCH-19/20/21/22/25`, `DC-GOV-01`, `DC-CINPUT-07`), or hold each for its committed live-flip transcript per the existing convention? (G3)
4. **Refresh the 22 stale test-fn names** in `DC-CONS-04` / `DC-EPOCH-16` / `DC-EVIEW-05` / `DC-EVIEW-08` / `DC-MITHRIL-07` post ECA-B rename — noting `DC-EVIEW-05`'s inverted-semantics rename needs a decision, not a find-replace?
5. **Bind or retire** the 4 candidate-unbound gates (`ci_check_single_producer_loop_continuation.sh`, `ci_check_wire_rollback_signal_preserved.sh`, `ci_check_live_blockfetch_byte_only.sh`, `ci_check_live_transcript_forge_base.sh`)?

---

## Generation notes

Regenerated at HEAD `1e4896eb` (LIVE-FORGE-HARDENING cluster close, `origin/main`) by joining the invariant registry (`docs/ade-invariant-registry.toml`, **432** rules parsed via `tomllib`: 297 enforced / 23 partial / 111 declared / 1 enforced_scaffolding, 0 deprecated) against codebase introspection. Per-rule cells are lifted verbatim from the registry (`source` → Source, `statement` → Requirement, `code_locus` → Code, `tests` → Tests, `ci_script`/`ci_scripts` → CI). **No rule was invented; no Code/Tests/CI cell was guessed** — empty load-bearing cells are marked as gaps, and every named CI script + code path + test-fn was checked for existence at HEAD (a stale test-fn name is daggered **†** and enumerated under *Cross-reference checks*; CI and code paths had zero stale references). The `replay_cmd` was **not** executed (4 KNOWN pre-existing `consensus_stream_replay` failures from the in-flight ECA-B reshape + a pre-existing `epoch_boundary_logic` hang — both unrelated to this audit). Families and sub-family groupings (T / CN / DC / OP / RO and the `XX-YYYY` stems) are preserved from the prior TRACEABILITY for stability; the registry has no `[families]` table. Regenerate — do not hand-edit; if a value drifts, fix the registry (the source), not this doc.
