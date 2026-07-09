# S5 — restart + bounded-rollback recovery admission + replay-equivalence (the last safety rail before S4)

> **Status: OPEN (recovery-authority slice).** The named recovery precondition for S4. CE-3d (S3, e476415a)
> proved the accumulator's boundary OUTPUTS are byte-exact against cardano; S5 proves those outputs
> REMATERIALIZE byte-identically after restart and a bounded controlled rollback, AND makes it impossible
> for recovery to rematerialize accumulator state from an INADMISSIBLE prefix. The rollback-admission guards
> (k-bound + lineage) are recovery authority — they land HERE, before promotion, so S4 stays a clean sealed
> flip. Declares no new invariant IDs; it is the recovery-equivalence + admission gate for S4.

**Cluster:** LIVE-LEDGER-EPOCH-TRANSITION. **Depends on:** S3 CE-3d (GREEN, e476415a). **Gates:** S4.

---

## 1. Intent

For the CE-3d v5/schema-v4 seed lineage: (a) the accumulator authority, reduced-UTxO checkpoint, WAL/ChainDB
state, and leadership-relevant `go` authority **rematerialize identically** after process restart and one
bounded controlled rollback; and (b) recovery ADMITS a rollback prefix only when it is within the bounded,
same-lineage, contiguous, schema-matched envelope — every inadmissible prefix is a typed deterministic
terminal, never a silent rematerialization. "Identically" = byte-equal canonical fingerprints. This is the
project's replay-equivalence integration law applied to recovery: the same bootstrap anchor + inputs + WAL +
checkpoints recover to byte-identical outputs, or fail closed.

## 2. Scope

Reuse the already-green CE-3d **v5 seed** (`~/.cardano-ce3d-s1seed-v5`, accumulator schema-v4). Prove
recovery equivalence around its boundary surfaces AND add the bounded-rollback admission guards that recovery
authority requires before the accumulator can be leadership authority (S4). Small on purpose: this is NOT
fork-choice — it makes recovery reject an inadmissible prefix, nothing more.

## 3. Execution boundary (TCB colors)

- **BLUE:** the rollback-admission decision (k-bound, lineage, anchor, contiguity), accumulator/checkpoint
  materialization, the rollback re-fold, and the fingerprint comparison. All deterministic + fail-closed.
- **GREEN:** test-harness / oracle-fixture orchestration; the evidence-bundle assembly.
- **RED:** process kill/restart, filesystem store loading, peer/feed simulation.

## 4. Deliverables (all IN S5)

1. **Same-lineage restart equivalence.** Clean uninterrupted advance vs. a process restart mid-advance
   (reload durable state + WAL replay) → BYTE-IDENTICAL fingerprints for accumulator, reduced checkpoint,
   `go`, reward accounts, treasury/reserves, AND the accumulator-derived authority view.
2. **Same-lineage controlled rollback re-fold.** A controlled rollback to a prior selected point re-folds the
   SAME canonical prefix into the SAME accumulator state (byte-identical), then re-advances to the SAME
   post-boundary fingerprints as the uninterrupted run.
3. **Lineage-checked reset admission.** The `reset_to_bootstrap` + re-fold admission is gated by a LINEAGE
   check (canonical hash at height), not height alone — a same-height / different-hash prefix is rejected,
   never accepted. (Replaces the current height-only `accumulator_reset_if_ahead`.)
4. **k-bounded rollback guard.** A rollback whose depth from the current tip exceeds SecurityParam `k`
   (before the immutable point = tip − k) is rejected with a typed `ExceededRollback`-style failure —
   cardano's immutable/volatile split (candidates never fork before the immutable tip). A rollback before the
   bootstrap anchor is likewise a typed terminal.
5. **Typed recovery-fingerprint mismatch.** Promote the T-REC-05 replay-equivalence gate from the
   stringly-typed `WarmStartBootstrap(String)` to a typed fingerprint-mismatch variant.

The rematerialized authority is derived ONLY from the accumulator/checkpoint (`EpochConsensusView::
canonical_hash` / `to_pool_distr_view`); `from_seed_epoch_consensus_inputs` appears solely as a comparison
oracle, never as the rematerialized authority.

## 5. Hard prohibitions

- **No S4 deletion.** The seed-window read path and the seed+2 ceiling STAY this slice.
- **No fallback** to `from_seed_epoch_consensus_inputs` as the rematerialized authority.
- **No feature gate / no dual authority mode.**
- **No "rollback works because warm-start passed"** — the rollback re-fold + admission are proven
  independently of the restart proof.
- **No logs as authority.** Logs may WITNESS evidence; equality is over canonical fingerprints.

## 6. Acceptance — the committed evidence bundle (GREEN)

**Positive — PROVEN** by `ade_testkit/tests/ce3d_boundary_differential.rs::s5_recovery_replay_equivalence_within_k_rollback`
(`#[ignore]`; env `S5_SEED_STORES` / `CE3D_CORPUS` / `CE3D_WORK`; crosses the 1340→1341 self-derived boundary).
The UNINTERRUPTED advance (A) vs. advance → admit within-k same-lineage rollback (`admit_rollback`, depth 150 ≤
k=2160) → event-qualified CLEAR (`reset_to_bootstrap` on BOTH derived stores) → refold from the canonical
ChainDB prefix (B) reach BYTE-IDENTICAL fingerprints:

| # | fingerprint | how |
|---|---|---|
| 1 | accumulator canonical hash | `blake2b_256(encode_epoch_accumulator)` — byte-covers the whole non-UTxO ledger (`c97225a4…e30e9aa` @ POST-1341) |
| 2 | reduced-checkpoint state | `blake2b(sum_base_credential_stake)` (`3b386961…3aff8ce`) |
| 3 | treasury + reserves | `epoch_state.treasury/reserves` |
| 4 | reward map | `cert_state.delegation.rewards` |
| 5 | go pool-set + values | `snapshots.go.pool_stakes` |
| 6 | accumulator-derived authority stake view | the `stake_by_pool` `to_pool_distr_view` consumes (`7952679f…c3efc6d3`) |
| 7 | warm-start replay | durable reopen re-materialises the in-memory state (`c97225a4…` == #1) |

Fixture notes (file-verified): `cp.fingerprint()` is `Incomplete` after an ADVANCE (its build-marker is
written only by a fresh `build_from`), so #2 commits the reduced *content*. The `ce3d-s1seed-v5` store's
bootstrap is epoch 1338 but its current is advanced to 1340, and the CE-3d corpus is late-1339→1342, so the
differential RE-SEALS the current 1340 state as the corpus-refoldable recovery baseline (`reset_to_bootstrap`
→ 1340) — the exact state b3c0 folds from.

**Negative — a LAYERED proof** (mechanical enforcement + the seam that expresses it):

| fault | enforcement locus |
|---|---|
| LineageMismatch | wired integration (`ade_node` `s5_warm_start_contradiction_present_anchor_wrong_hash_is_terminal`) |
| ExceededRollback | wired integration (`s5_live_rollback_beyond_k_is_terminal_exceeded`) |
| TargetNotOnCanonicalChain | wired integration (`s5_live_rollback_target_absent_from_chain_is_terminal`) |
| CorruptLastAdvancedPoint | wired/store (`epoch_accumulator_store::malformed_lineage_anchor_bytes_fail_closed`) |
| MissingCanonicalSpan | wired/refold (`accumulator_recover_admit` seed/tip resolve path) |
| NonContiguousCanonicalSpan | wired/refold (`resolve_canonical_point` decode path) |
| FingerprintMismatch | typed T-REC-05 (`warm_start_recovery` WAL-tail gate) |
| BeforeBootstrapAnchor | BLUE admission guard (`rollback::admission::rollback_before_bootstrap_anchor_is_typed`); the live rollback seam is STRUCTURALLY UNREACHABLE (a selected rollback target is never below the immutable bootstrap floor), so there is no wired fixture without fabricating a lower-block second fixture |
| SchemaMismatch | schema-v4 rejection path (`epoch_accumulator` `UnknownVersion` / `codec_rejects_pre_c_v3_store_rebootstrap_required`) |

Where equality is claimed it is byte-exact; every rejection is typed and reproducible.

**S5 closure claim:** S5 proves accumulator recovery admission and rematerialization are replay-equivalent for
restart and controlled rollback. It does NOT activate accumulator-derived leadership authority (that is S4).
With 2c green + committed, S4 becomes admissible — no seed-window deletion before that.

## 7. IDD classification

- **True (unconditional):** recovery is replay-equivalent from canonical state under restart and bounded
  rollback; an inadmissible prefix cannot rematerialize authoritative state (fail-closed, deterministic).
- **Derived (Cardano-compatible):** the rematerialized accumulator/checkpoint/go/pots/rewards/authority equal
  the uninterrupted derivation; rollback bounded by SecurityParam k (immutable/volatile split).
- **Release / evidence:** the §6 positive + negative bundle on the v5 seed.

## 8. Do NOT include yet (S4 or later)

- S4 deletion targets (the seed-window reads, the seed+2 ceiling).
- Live multi-producer fork-choice.
- Accumulator-derived leadership as PRODUCTION authority (that promotion IS S4).
- Arbitrary adversarial fork recovery BEYOND the bounded admission rules (S5 admits/rejects; it does not
  select among competing valid forks).

S5 makes recovery admissibility safe; S4 then flips authority cleanly on green preconditions.

## 9. Code loci (grounding)

- **Warm-start recovery:** `node_lifecycle.rs::warm_start_recovery` L3046-3237; WAL replay
  `wal/replay.rs::replay_from_anchor` L93-200; accumulator re-materialization
  `node_lifecycle.rs::advance_ledger_state_to_durable_tip` L1756+ →
  `chaindb/epoch_accumulator_advance.rs::advance_accumulator_over_chaindb` L181; T-REC-05 gate L3216-3223
  (`WarmStartBootstrap(String)` → promote to typed).
- **Rollback admission to harden:** `node_lifecycle.rs::accumulator_reset_if_ahead` L1717-1735 (HEIGHT-only —
  add lineage + k-bound); `reduced_checkpoint_reset_if_ahead` L1677-1702; ledger rollback
  `rollback/materialize.rs::materialize_rolled_back_state` L43-136 (has `RollbackTooDeep` vs snapshot
  availability — add the k/immutable-point bound + anchor check); `EpochAccumulatorStore::reset_to_bootstrap`
  L238-264; `ReducedUtxoCheckpoint::reset_to_bootstrap` L292-313. k=2160 parsed
  `native_firstrun.rs:144-147`; `rollback_depth` computed (audit-only) `candidate_aggregator.rs:113-122`.
- **Fingerprint surfaces:** `fingerprint.rs::fingerprint`=`fingerprint_v2` L126 (pots `fingerprint_epoch`
  L432, snapshots `fingerprint_snapshots` L451, rewards `fingerprint_cert` L381);
  `ReducedUtxoCheckpoint::fingerprint` reduced_utxo_checkpoint.rs:508; accumulator canonical fingerprint =
  `blake2b_256(encode_epoch_accumulator(acc))` (epoch_accumulator.rs:1529, small BLUE helper).
- **Accumulator-derived authority (NOT seed):** `EpochConsensusView::canonical_hash` reduced_epoch_view.rs:169
  + `to_pool_distr_view` L239-264; seed oracle only `PoolDistrView::from_seed_epoch_consensus_inputs`
  consensus_view.rs:89.
- **Reused harness:** ce3d_boundary_differential.rs `co_advance` L111, `ade_post_state` L186, isolated-copy
  `open`, `load_corpus` L87, `b3c0_adjudication` L516 (report-hash pin); reopen-equivalence units
  epoch_accumulator_store.rs:632 / reduced_utxo_checkpoint.rs:830; run-twice-identical oracle
  consensus_stream_replay.rs:354.
- **Typed failure variants:** exceeded-rollback `MaterializeError::RollbackTooDeep` rollback/error.rs:23
  (extend with the k/immutable-point + anchor cases); schema `EpochAccumulatorCodecError::UnknownVersion`
  epoch_accumulator.rs:1473; non-contiguous `EpochAccumulatorStoreError::NonMonotonicAdvance` L75 /
  `WalError::ChainBreak` wal/error.rs:29 (explicit missing-span if needed); NEW lineage-mismatch + NEW typed
  recovery fingerprint-mismatch.

## 10. Design + feasibility (the v5-lineage proof shape)

- **Rollback admission (BLUE, new):** a pure `admit_rollback(current, target, anchor, k, canonical_lineage) ->
  Result<(), RollbackAdmissionError>` — rejects target < anchor, depth(current,target) > k (immutable point),
  and target-hash ≠ canonical-hash-at-height; called BEFORE any `reset_to_bootstrap` + re-fold. The same typed
  sum drives the negative acceptance.
- **Restart equivalence (§4.1):** corpus boundary range (POST-1340 seed → 1341 → 1342), `co_advance` clean
  vs. drop-store-handles-mid-advance + reopen + continue; compare the full fingerprint set.
- **Rollback re-fold (§4.2):** the accumulator rolls back by `reset_to_bootstrap` + re-fold; proven over the
  v5 seed's own runtime-followed 1338→1340 lineage (its `chain.db`): admit → reset → re-fold → byte-identity
  with the sealed POST-1340; ledger arbitrary-point re-fold already unit-proven (materialize.rs:504).
- **Authority (§4 last para):** hash `EpochConsensusView::canonical_hash` via the accumulator path; the seed
  sidecar view is an independent oracle only.
