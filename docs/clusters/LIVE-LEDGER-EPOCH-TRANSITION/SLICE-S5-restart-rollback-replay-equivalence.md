# S5 — restart + controlled-rollback replay-equivalence (the authority-promotion recovery proof)

> **Status: OPEN (proof slice).** The named recovery-equivalence precondition for S4. CE-3d (S3) proved the
> accumulator's boundary OUTPUTS are byte-exact against cardano; S5 proves those same outputs
> REMATERIALIZE byte-identically after restart and one controlled rollback — before the accumulator is
> allowed to become leadership authority. Warm-restart contract tests (`boundary_stateful_replay` etc.) are
> EVIDENCE, not this proof: S5 is the authority-promotion recovery proof, pinned to the CE-3d v5/schema-v4
> seed lineage. Declares no new invariant IDs; it is the recovery-equivalence evidence gate for S4.

**Cluster:** LIVE-LEDGER-EPOCH-TRANSITION. **Depends on:** S3 CE-3d (GREEN, e476415a). **Gates:** S4.

---

## 1. Intent (the one-line contract)

For the CE-3d v5/schema-v4 seed lineage, the accumulator authority, the reduced-UTxO checkpoint, the
WAL/ChainDB state, and the leadership-relevant `go` authority **rematerialize identically** after (a) a
process restart (warm-start reload) and (b) one controlled rollback to a prior selected point followed by
re-advance. "Identically" = byte-equal canonical fingerprints (§4), never a log or a confidence call.

## 2. Scope

Reuse the already-green CE-3d **v5 seed** (`~/.cardano-ce3d-s1seed-v5`, accumulator schema-v4) and its
boundary surfaces (runtime-followed 1338→1339→1340; differential self-derived 1340→1341→1342 at db-analyser
reference labels POST-1341 /115862416, POST-1342 /115948834). Prove recovery equivalence around THOSE same
boundaries, so S4 can then delete the seed-window path with recovery already proven — not deferred.

## 3. Execution boundary (TCB colors)

- **BLUE** (deterministic authority): accumulator materialization, reduced-checkpoint replay, the rollback
  re-fold, and the authority/state fingerprint comparison.
- **GREEN** (deterministic glue): test-harness / oracle-fixture orchestration; the evidence-bundle assembly.
- **RED** (nondeterministic shell): process kill/restart, filesystem store loading, peer/feed simulation.

No RED in the BLUE comparison; GREEN never affects an authoritative fingerprint.

## 4. Required proofs

1. **Restart equivalence.** A clean uninterrupted run and a warm-start (process restart mid-lineage, reload
   durable state + WAL replay) produce BYTE-IDENTICAL fingerprints for: accumulator, reduced checkpoint,
   `go` snapshot, reward accounts, treasury/reserves pots, AND the accumulator-derived authority view.
2. **Rollback re-fold equivalence.** A controlled rollback to a prior selected point re-folds the SAME
   canonical prefix into the SAME accumulator state (byte-identical fingerprint) — bounded by k, over
   canonical observables only.
3. **Re-advance equivalence.** Re-advancing after the rollback reaches the SAME post-boundary fingerprints
   as the uninterrupted run.
4. **Authority provenance.** The old seed-window authority (`PoolDistrView::from_seed_epoch_consensus_inputs`)
   is NEVER the rematerialized authority during the S5 proof — it appears ONLY as an independent comparison
   oracle. The rematerialized authority is derived solely from the durable accumulator.
5. **Typed deterministic failures.** Each off-nominal recovery is a typed, reproducible terminal: exceeded
   rollback (deeper than k), missing WAL span, checkpoint mismatch, schema-version mismatch, non-contiguous
   prefix. [LOCI: which typed variants already exist vs are added.]

## 5. Hard prohibitions

- **No S4 deletion.** The seed-window read path and the seed+2 ceiling STAY this slice; S5 only proves
  recovery equivalence.
- **No fallback** to `PoolDistrView::from_seed_epoch_consensus_inputs` as the rematerialized authority.
- **No feature gate / no dual authority mode.**
- **No "rollback works because warm-start passed"** — the rollback re-fold is proven independently (§4.2/4.3),
  never inferred from the restart proof.
- **No logs as authority.** Logs may WITNESS evidence; the equality claim is over canonical fingerprints.

## 6. Acceptance — the committed evidence bundle

S5 is GREEN only when a committed evidence bundle records these six fingerprints and their required
equalities, over the v5 lineage:

| fingerprint | source |
|---|---|
| clean-run | uninterrupted v5 advance to the reference boundary |
| warm-start | restart mid-advance → reload → replay → same boundary |
| rollback-target | accumulator at the controlled-rollback selected point |
| re-folded | accumulator after re-folding the same canonical prefix |
| re-advanced | post-boundary after re-advance following the rollback |
| authority-view | accumulator-derived `PoolDistrView` at the boundary |

Required equalities: clean = warm-start (§4.1); rollback-target re-fold identity (§4.2); re-advanced =
clean (§4.3); authority-view derived-only, equal to its independent seed-oracle where CE-3d already proved
byte-exactness (§4.4). Every mismatch is typed + reproducible (§4.5). Where equality is claimed it is
byte-exact; nothing is "close enough."

## 7. IDD classification

- **True (unconditional):** one authoritative selected-chain state that is replay-equivalent under restart
  and bounded rollback; recovery is deterministic over canonical observables (snapshot + forward replay;
  rollback bounded by k).
- **Derived (Cardano-compatible):** the rematerialized accumulator / checkpoint / go / pots / rewards /
  authority — all reproduced from durable canonical state, equal to the uninterrupted derivation.
- **Release / evidence:** the §6 fingerprint bundle across restart + one controlled rollback on the v5 seed.

## 8. What S5 does NOT do

No S4 deletion, no authority promotion, no seed-window retirement, no seed+2-ceiling removal — those are S4.
No new invariant IDs. No governance-coverage change. S5 is the narrow recovery-equivalence proof and its
committed evidence bundle, nothing else. Once S5 is GREEN, S4 becomes admissible (both preconditions met).

## 9. Code loci (grounding)

- **Warm-start recovery:** `node_lifecycle.rs::warm_start_recovery` L3046-3237; WAL replay
  `wal/replay.rs::replay_from_anchor` L93-200; accumulator re-materialization
  `node_lifecycle.rs::advance_ledger_state_to_durable_tip` L1756+ →
  `chaindb/epoch_accumulator_advance.rs::advance_accumulator_over_chaindb` L181; replay-equivalence gate
  **T-REC-05** `node_lifecycle.rs` L3216-3223 (currently `WarmStartBootstrap(String)` — S5 promotes to typed).
- **Durable reload:** `EpochAccumulatorStore::load_current` epoch_accumulator_store.rs:215;
  `ReducedUtxoCheckpoint::open` + `advance_reduced_checkpoint_over_chaindb` reduced_window_driver.rs:204.
- **Rollback re-fold:** ledger `rollback/materialize.rs::materialize_rolled_back_state` L43-136 (CN-STORE-07,
  proven `materialize_replay_forward_equals_direct_apply` L504); accumulator reset
  `EpochAccumulatorStore::reset_to_bootstrap` L238-264 (round-trip proof L550) + co-advancer re-fold;
  checkpoint reset `ReducedUtxoCheckpoint::reset_to_bootstrap` L292-313.
- **Fingerprint surfaces:** `fingerprint.rs::fingerprint`=`fingerprint_v2` L126 (`LedgerFingerprint{era,utxo,
  cert,epoch,snapshots,pparams,governance,combined}`); pots `fingerprint_epoch` L432 (reserves/treasury);
  snapshots `fingerprint_snapshots` L451 (mark/set/go); rewards `fingerprint_cert` L381;
  `ReducedUtxoCheckpoint::fingerprint` reduced_utxo_checkpoint.rs:508. **Accumulator canonical fingerprint =
  `blake2b_256(encode_epoch_accumulator(acc))`** (epoch_accumulator.rs:1529) — S5 adds a small BLUE helper.
- **Accumulator-derived authority (NOT seed):** `EpochConsensusView::canonical_hash` reduced_epoch_view.rs:169
  + `to_pool_distr_view` L239-264 (fail-closed on params/leadership incompleteness). Seed path S5 uses ONLY
  as an oracle: `PoolDistrView::from_seed_epoch_consensus_inputs` consensus_view.rs:89.
- **Reused harness (v5 seed drive):** ce3d_boundary_differential.rs `co_advance` L111, `ade_post_state` L186,
  `EpochAccumulatorStore::open`/`ReducedUtxoCheckpoint::open` on `std::fs::copy` isolated copies,
  `load_corpus(dir, up_to_slot)` L87; the `b3c0_adjudication` L516 isolated-copy+report-hash pin pattern;
  reopen-equivalence units epoch_accumulator_store.rs:632 / reduced_utxo_checkpoint.rs:830; run-twice-
  identical oracle consensus_stream_replay.rs:354.
- **Typed failure variants:** exceeded-rollback `MaterializeError::RollbackTooDeep` rollback/error.rs:23 +
  `AccumulatorReadinessError::Ahead` L92; schema `EpochAccumulatorCodecError::UnknownVersion`
  epoch_accumulator.rs:1473; non-contiguous `EpochAccumulatorStoreError::NonMonotonicAdvance` L75 /
  `WalError::ChainBreak` wal/error.rs:29; missing-bytes `WalError::BlockBytesMissing` L36. **S5 adds:** a
  typed recovery **fingerprint-mismatch** (promote T-REC-05 from `WarmStartBootstrap(String)`).

## 10. Design + feasibility (the v5-lineage proof shape)

- **Restart equivalence (§4.1):** on the corpus boundary range (POST-1340 seed → 1340→1341→1342), run
  `co_advance` clean vs. drop-the-store-handles-mid-advance + reopen + continue; compare the full fingerprint
  set. Directly feasible with the ce3d harness + the reopen-equivalence units.
- **Rollback re-fold (§4.2/4.3):** the accumulator rolls back by `reset_to_bootstrap` (to the seed anchor)
  then re-folds the durable canonical chain — so the re-fold needs the bootstrap->target chain. The corpus
  starts ~1340, so the accumulator rollback re-fold is proven over the v5 seed's OWN runtime-followed
  1338->1340 lineage (its chain.db has those blocks): reset to bootstrap, re-fold, assert byte-identity with
  the sealed POST-1340 accumulator/checkpoint/authority the follow produced; the ledger-level arbitrary-point
  rollback re-fold is already unit-proven (materialize.rs:504). Controlled rollback = same canonical prefix
  (same lineage); a diverging-fork lineage guard is NOT in S5 scope (see §11).
- **Authority (§4.4):** hash `EpochConsensusView::canonical_hash` built from the accumulator/checkpoint
  (`to_pool_distr_view` path), never `from_seed_epoch_consensus_inputs`; where CE-3d already proved the go
  set byte-exact, cross-check the seed sidecar view ONLY as an independent oracle.

## 11. Explicitly OUT of S5 scope (recorded, deferred)

- **k-bounded rollback guard.** There is no "no rollback deeper than k" guard today — only `RollbackTooDeep`
  vs snapshot availability (k=2160 is consumed only for the Praos randomness-stabilisation window). S5's
  controlled rollback stays within the available window; a k-depth guard is a separate hardening.
- **Lineage-checked (not height-only) accumulator reset.** `node_lifecycle.rs` L1712-1716 resets-if-ahead on
  HEIGHT only; a lineage-diverging longer chain below `last_advanced` is height-invisible. This is the S4
  MEDIUM-2 reorg-driver obligation — required before accumulator authority faces adversarial forks, NOT
  before S5's same-canonical-prefix controlled-rollback proof. Pinned here so S4 inherits it.

These are named so S4 cannot silently assume them; they are not S5 deliverables.
