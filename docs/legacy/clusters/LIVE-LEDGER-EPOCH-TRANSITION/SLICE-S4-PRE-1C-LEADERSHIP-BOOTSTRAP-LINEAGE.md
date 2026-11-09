# S4-pre-1c — Certify the Frozen Leadership Bootstrap Lineage

> **Status: OPEN.** Makes the durable frozen leadership authority (S4-pre-1b `13829660`) REAL in the live
> bootstrap lineage and proves it survives recovery. Prerequisite for S4-pre-2, which is the last piece before
> S4 proper. S4 stays BLOCKED until 1c AND S4-pre-2 are both green.

## Intent

Wire the frozen leadership import into the native bootstrap, re-bootstrap the v5 lineage so the store actually
carries schema-v5 leadership authority, prove the object equals cardano's seed leadership `PoolDistr`, and
extend the S5 recovery evidence so restart / rollback reproduce the frozen leadership hash + view.

## Do

1. **Wire** `EpochAccumulatorStore::seal_frozen_leadership_from_seed_record` into the native first-run
   bootstrap (`crates/ade_node/src/native_firstrun.rs`, beside the existing
   `store.seal_bootstrap(&seed, source_point_slot)`). The seed record is `output.seed_epoch_consensus_inputs`;
   the source binding is the certified bootstrap point `binding.certified_point.{slot, block_hash}` (the same
   source point the bootstrap RUPD uses). Consistent with the surrounding S2 observe-only tolerance, a seal
   failure is non-fatal + logged — but a CLEAN bootstrap always certifies (the proof asserts the marker).
2. **Re-bootstrap** the v5 lineage so the durable store carries schema-v5 leadership authority (the marker +
   the frozen object).
3. **Extend the S5 recovery evidence** to include the frozen leadership canonical hash / view.
4. **Registry**: add entries marking the S4-pre leadership authority pieces enforced.
5. **CI/static guard**: forbid the quarantined wrong builder
   (`from_accumulator_go_active_params_for_test_only`) from any production authority path.

## Do NOT

- No production seed-window swap (the three `from_seed_epoch_consensus_inputs` read sites stay).
- No seed+2 ceiling deletion (`epoch_wire.rs` stays).
- No SNAP boundary freeze yet (that is S4-pre-2).
- No S4 authority promotion (nothing production READS `leadership_authority()` yet).
- No fallback to seed leadership after bootstrap.

## Required proof — the new lineage

- fresh bootstrap store has the leadership-schema-v5 marker;
- `leadership_authority()` loads the frozen object;
- the frozen leadership canonical hash is stable across reopen;
- `to_pool_distr_view` == the seed leadership `PoolDistr`: **659/659 pools**, stake exact, VRF exact,
  zero-stake pools preserved, the retired 1M-ADA pool preserved with its frozen VRF.

## Required proof — recovery

- **clean advance vs warm restart**: frozen leadership hash identical;
- **within-k rollback / refold**: frozen leadership hash identical;
- **missing / corrupt leadership blob**: terminal typed failure for the leadership authority read;
- **old v4 store**: not leadership-certified; cannot reach the S4 authority path.

## CI guard

- `from_accumulator_go_active_params_for_test_only` allowed ONLY under test / oracle / negative-regression
  paths; any production reference fails CI.
- (Deferred) guarding direct use of seed leadership authority in production S4 paths belongs to S4 proper — do
  NOT overreach now if it would break the still-unflipped code.

## Commit boundary

One commit — **"S4-pre-1c certifies frozen leadership bootstrap lineage"** — when the re-bootstrapped lineage
proves leadership authority is durable and replay-equivalent:

- wire frozen leadership import into native bootstrap;
- re-bootstrap the schema-v5 leadership-certified lineage;
- prove seed leadership `PoolDistr` identity;
- extend restart / rollback recovery evidence to frozen leadership;
- add registry and CI guard for leadership authority.

## Evidence (what shipped)

1. **Wire (item 1)** — `native_first_run_bootstrap` (`crates/ade_node/src/native_firstrun.rs`) seals the frozen
   leadership beside the accumulator baseline via `seal_frozen_leadership_from_seed_record(&output.seed_epoch_consensus_inputs,
   binding.certified_point.slot, &binding.certified_point.block_hash)`. The source binding is sound: the seed
   record's `seed_point_{slot,hash}` are derived from `binding.certified_point.{slot,block_hash}` through the
   mithril-assembly coherence gate (`mithril_native_assembly.rs:298-307`), so the source check passes on a
   legitimate bootstrap. Non-fatal like the accumulator seal (nothing READS leadership as production authority
   until S4); a clean bootstrap always leadership-certifies. Workspace builds.
2. **Certified lineage (item 2)** — `ce3d_boundary_differential::s4pre_1c_frozen_leadership_bootstrap_lineage`
   (fixture-backed, FAST, GREEN): seals a fresh store from the REAL v5 seed record via the exact wiring call,
   then proves the certified store has the v5 marker, `leadership_authority()` loads, `to_pool_distr_view` ==
   the seed leadership `PoolDistr` **659/659 byte-exact** (incl. zero-stake + the retired 1M-ADA pool), and the
   canonical hash is stable across reopen. The durable canonical fixture is (re-)certified by re-running the
   operator bootstrap (`docs/active/ce3d-s1-rebootstrap-runbook.md`) with the wired binary — the wiring produces
   this exact object deterministically from the seed record.
3. **Recovery evidence (item 3)** — the S5 differential
   `ce3d_boundary_differential::s5_recovery_replay_equivalence_within_k_rollback` gains fingerprint **#8 frozen
   leadership canonical hash**, asserted byte-identical across clean advance (A) vs within-k rollback+reset+refold
   (B) AND warm restart (durable reopen). `s5_open_resealed` leadership-certifies each isolated copy from the
   seed record. `reset_to_bootstrap` preserving the leadership is proven hermetically by 1b
   (`reset_to_bootstrap_preserves_frozen_leadership`, `frozen_leadership_survives_reopen`); the missing / corrupt
   / legacy-v4 fail-closed reads are proven by 1b (`leadership_authority_rejects_missing_object_under_valid_marker`,
   `_malformed_object`, `_fails_closed_on_legacy_store`).
4. **Registry (item 4)** — `DC-EPOCH-25` (frozen leadership authority) added to `docs/ade-invariant-registry.toml`,
   `status = "declared"` (the pieces — codec / store / import / recovery / guard — are mechanically enforced;
   production leader-schedule promotion is the S4 `open_obligation`). Registry validators green (unique ids,
   cross_refs resolve, code_locus + ci_script paths exist).
5. **CI guard (item 5)** — `ci/ci_check_frozen_leadership_authority.sh` forbids
   `from_accumulator_go_active_params_for_test_only` on any production authority path (allowing only its
   definition + test/oracle). Wired into `cargo test` by `crates/ade_node/tests/frozen_leadership_authority_gate.rs`
   (green on the clean tree, fails closed on a planted leak). The seed-leadership production guard is DEFERRED to
   S4 (guarding it now would fail against the still-unflipped code).

## After 1c

S4-pre-2 is the remaining hard piece: freeze the NEXT leadership distribution at the cardano SNAP / `nesPd`
boundary from snapshot-frozen stake + snapshot-frozen pool params/VRF, BEFORE POOLREAP. Then S4 proper (the
narrow flip: swap the three seed-window sites + delete the seed+2 ceiling + guard + prove).
