# S4-pre-2 — Native Boundary Leadership Freeze

> **Status: OPEN.** The missing live recurrence. Bootstrap leadership is certified (S4-pre-1a/1b/1c); S4-pre-2
> proves Ade can KEEP PRODUCING that authority natively at each epoch boundary. Last authority-construction
> slice before S4 proper. S4 stays BLOCKED until this is green.

## Intent

Freeze the next epoch's leadership distribution (`FrozenLeadershipPoolDistr`) at the cardano-defined SNAP /
`nesPd` point, using snapshot-frozen stake and snapshot-frozen pool params/VRF, **before POOLREAP** can remove
retired params.

## Do

1. Add boundary freeze logic for `FrozenLeadershipPoolDistr`.
2. Source stake from the leadership SET snapshot, not reward `go`.
3. Source VRF from snapshot-frozen pool params, not current active params at use time.
4. Include zero-stake registered leadership pools exactly as cardano does.
5. Preserve retired-but-still-leadership-relevant pool VRF before POOLREAP.
6. Persist the new frozen leadership object atomically with the epoch boundary transition.
7. Extend the boundary differential to compare frozen leadership against reference `nesPd`.
8. Re-run warm-restart and rollback/refold evidence over the updated frozen leadership hash.

## Do NOT

- No production seed-window swap (the three `from_seed_epoch_consensus_inputs` reads stay).
- No seed+2 ceiling deletion.
- No active-param lookup at leadership-USE time (freeze at capture time only).
- No go-derived leadership.
- No fallback to seed leadership.
- No one-pool supplement hack (the retired-VRF case is handled by the general pre-POOLREAP freeze).

## Critical ordering rule (consensus-critical)

At each boundary, in this exact order:

1. capture snapshot-frozen leadership stake + pool params/VRF (from the pre-POOLREAP registered pool set);
2. seal `FrozenLeadershipPoolDistr` for the target leadership epoch;
3. THEN allow POOLREAP / active-pool cleanup effects.

The retired 1M-ADA pool proved why: its VRF exists in the active params ONLY before POOLREAP reaps it. In Ade
this maps to `rules::apply_epoch_boundary_with_registrations` — the SNAP builds the mark reading
delegations PRE-POOLREAP (already cardano's SNAP-before-POOLREAP order), then `rotate_snapshots`, then POOLREAP
removes retiring pools from `cert_state.pool.pools`. The VRF freeze injects between the SNAP mark build and
POOLREAP, where `pool_state.pools` (the active `PoolParams` incl. `vrf_hash`) is still intact.

## Design (candidate — VRF capture is TRANSIENT, no accumulator-blob schema change)

`build_boundary_mark_snapshot` returns `epoch::StakeSnapshot { delegations, pool_stakes }` — stake only, and it
OMITS zero-stake credentials (DC-EPOCH-24, the serialized `ssActiveStake` NonZero rule). But `nesPd` includes
zero-stake REGISTERED pools. So the leadership pool SET is the registered pool-param key set, not the stake
map: `nesPd = { pid: (pool_stakes.get(pid).unwrap_or(0), frozen_vrf[pid]) for pid in registered_pools }`.

Candidate: at the boundary, capture the VRF for EVERY registered pool from the pre-POOLREAP `pool_state.pools`,
build `FrozenLeadershipPoolDistr` for the target leadership epoch directly (stake from the just-built mark's
`pool_stakes`, VRF + pool-set from the captured params), and seal it into the SEPARATE schema-v5 leadership
store object (`seal_frozen_leadership`) — atomically with the boundary. Because the capture is transient and
the leadership object is a separate store key, the accumulator BLOB codec is UNCHANGED (no
`EPOCH_ACCUMULATOR_SCHEMA_VERSION` bump; the CE-3d / S5 v4-blob fixtures are unaffected). On recovery/refold the
boundary is re-applied deterministically, re-sealing the identical leadership object (replay-equivalent).

**Confirmed loci (agent-mapped).** Snapshot payload `epoch::StakeSnapshot { delegations, pool_stakes }`
(epoch.rs:26) — stake only, no VRF. Active params `cert.pool.pools: BTreeMap<PoolId, PoolParams>` with
`vrf_hash: Hash32` (delegation.rs:106). The freeze injection is **`rules.rs:1324`** (the
`build_boundary_mark_snapshot` call) — `cert.pool.pools` is an immutable borrow intact for the whole boundary;
POOLREAP mutates a CLONE (`pool_state = cert.pool.clone()`, rules.rs:1376) at rules.rs:1382/1428, so the active
VRF is fully present pre-POOLREAP at :1324. `apply_epoch_boundary_with_registrations` returns
`(LedgerState, EpochBoundaryAccounting)` (rules.rs:753) → the boundary-frozen `FrozenLeadershipPoolDistr` rides
out through `EpochBoundaryAccounting` (rules.rs:1605), which `cross_epoch_boundary` currently discards
(epoch_accumulator.rs:576) → `apply_selected_block` (epoch_accumulator.rs:419) returns it as an explicit effect
→ the RED advancer (ade_runtime) seals it. `FrozenLeadershipPoolDistr` + its codec + the store seal already
exist (S4-pre-1a/1b); only the boundary builder is new. The disproven `from_accumulator_go_active_params_for_test_only`
(consensus_view.rs:109, go-stake + active-VRF) stays test-only (DC-EPOCH-25 CI guard).

**Schema decision: NO accumulator-blob bump.** VRF is captured TRANSIENTLY at the SNAP and sealed into the
separate schema-v5 leadership object — it is NEVER persisted into `StakeSnapshot` (which would bump
`EPOCH_ACCUMULATOR_SCHEMA_VERSION` 4→5 and fail-close the CE-3d / S5 v4-blob fixtures, which cannot be
regenerated here without a live re-bootstrap). Consequence: the object built at the boundary into `e` is
`nesPd_{e+1}` (the mark just built serves as `set` one rotation later), NOT `nesPd_e` (which would need the
PREVIOUS mark's persisted VRF). This is cardano-faithful: `nesPd_{e+1} = calculatePoolDistr(set_{e+1} = M_e)`,
`M_e._poolParams` = active params at the boundary into `e` (pre-POOLREAP). Capture-time, not use-time.

**Reference `nesPd` — CONFIRMED extractable (agent-mapped).** `ade_ledger::ledgerdb_state::decode_native_nonutxo_state(state, point, epoch, 2).pool_distr` decodes the TRUE `nes[5]` leadership PoolDistr as
`BTreeMap<PoolId, (u64 active_stake, Hash32 vrf)>` (ledgerdb_state.rs:1332) — the SAME field that produced the
proven-byte-exact seed (1338) `pool_distribution`, incl. zero-stake + retired pools. The differential's
`ref_post_state` (ce3d_boundary_differential.rs:216) ALREADY calls this decoder (it just ignores `.pool_distr`).
Reference `NewEpochState` states are on disk under `/home/ts/.cardano-ce3d-extract/db/ledger/<slot>_db-analyser/state`:
epoch **1340** (`115776011`), **1341** (`115862416`), **1342** (`115948834`). Compare against `.pool_distr`
(the literal `nes[5]`), NEVER `.mark_pool_distr` (a lossy derivation that drops zero-stake + no-active-VRF pools).

**OPEN VERIFICATION (proof discipline — do NOT assume from memory):** pin EMPIRICALLY (1) the rotation→use epoch
mapping — Ade crosses `1338→1339→1340`; the object built at the boundary into `e` is compared against reference
`nesPd_{e+1}` (candidate) i.e. the boundary into 1339 → reference 1340, the boundary into 1340 → reference 1341;
the acceptance test IS the verification (a mismatch re-maps the offset). (2) whether `nesPd_{seed+1}` (1339) is a
gap needing a bootstrap-side derivation. (3) seal shape: overwrite suffices for S4-pre-2's produce+recover proof
(the current-epoch reader is an S4 concern), and the boundary effect returns ALL produced objects (one per
crossed boundary) so the test compares each against its own reference epoch.

## Reset semantics correction (LANDED, before the plumbing) — the two-key model

The S4-pre-1b rule "`reset_to_bootstrap` preserves frozen leadership" was safe while leadership was a single
bootstrap object. Once leadership is RECURRENT, preserving an arbitrary post-boundary object across a reset is
WRONG: e.g. CURRENT = `nesPd` for epoch 1341 while a rollback/refold resets the accumulator to bootstrap epoch
1338 — the refold has not yet crossed the boundary that justifies 1341, so a preserved 1341 object outruns the
accumulator (replay-equivalence violation). Fix (`epoch_accumulator_store.rs`, DONE + tested):

- Two durable keys: `bootstrap_frozen_leadership` (IMMUTABLE — sealed once at bootstrap, never by a boundary
  freeze) and `frozen_leadership` (CURRENT — overwritten each boundary freeze).
- `seal_bootstrap_frozen_leadership` (bootstrap): writes BOOTSTRAP + CURRENT + marker atomically;
  `seal_frozen_leadership_from_seed_record` now calls it.
- `seal_frozen_leadership` (boundary freeze): overwrites CURRENT only.
- `reset_to_bootstrap`: CURRENT := BOOTSTRAP (if present), else clear CURRENT + marker (an uncertified store
  never preserves a stray current object as authority).
- Tests: `reset_to_bootstrap_restores_bootstrap_frozen_leadership` (reset restores bootstrap, NOT the stale
  boundary object), `reset_clears_current_leadership_when_no_bootstrap_object`. 22 store tests green.

## Plumbing (explicit, ordered authoritative effect — NOT a side channel)

`EpochBoundaryEffect::FreezeLeadership(FrozenLeadershipPoolDistr)`, returned as an ordered `Vec` (a batch may
cross multiple boundaries). Minimal-churn seam: `cross_epoch_boundary` computes the freeze itself from the PRIOR
accumulator's `cert_state.pool.pools` (the pre-POOLREAP registered params — POOLREAP happens inside the boundary
fn, so the prior state's params are exactly the SNAP-time set) + the just-built new mark's `pool_stakes` — so the
shared, byte-exact `apply_epoch_boundary_with_registrations` is NOT touched. A core/wrapper pair
(`*_with_effects` core + the existing signature as a discarding wrapper) avoids rewriting ~26 test callers. The
crossing block's `source_slot`/`source_hash` bind the effect (a `block_header_hash` added to `SelectedBlockCtx`).
Enforced on the `Vec`: deterministic order by `source_slot`, no duplicate `target_epoch`, no missing effect when
a boundary crossed, every effect carries `source_slot` + `source_hash` + `target_epoch`. The RED advancer seals
`current_frozen_leadership` from the effects atomically with the accumulator advance (a pending/complete marker
fails closed on torn state — a store never durably exposes a new accumulator epoch without its matching frozen
leadership, or vice-versa).

## Acceptance (green only when a self-derived boundary produces)

- `FrozenLeadershipPoolDistr == cardano reference leadership PoolDistr / nesPd`: pool count exact, pool ids
  exact, stake exact, VRF exact, zero-stake registered pools exact, retired-leadership-relevant pools preserved;
- canonical hash stable across reopen;
- rollback/refold hash exact;
- warm restart hash exact.

Negative cases:

- missing frozen params for a leadership pool → typed fail closed;
- malformed frozen leadership object → typed fail closed;
- attempted use of active params at leadership-USE time → CI/test failure;
- old schema-v4 store → not leadership-certified.

**The epoch-label mapping MUST be pinned EMPIRICALLY in the test output**, unambiguously, e.g.:
`boundary 1339→1340 produced leadership distribution for epoch 1341; reference compared: POST-1341 nesPd for
epoch 1341`. No ambiguous mark/set/go naming in the final proof — use the reference field name (`nesPd`) and the
target leadership epoch.

## Evidence (PROVEN)

**Layer 1 (BLUE effect).** `EpochBoundaryEffect::FreezeLeadership { source_epoch, target_leadership_epoch,
distr }`; `cross_epoch_boundary_with_effect` captures the pre-POOLREAP delegation image + registered VRF and
builds `nesPd_{target+1}`; `apply_selected_block_with_effects` returns the ordered `Vec`; `validate_boundary_effects`
rejects order/dup/label violations as typed terminals. The shared byte-exact `apply_epoch_boundary_with_registrations`
is untouched. Unit-tested.

**Layer 2 (RED atomicity).** `advance_with_current_leadership` writes accumulator + `LAST_SLOT` + anchor +
current leadership + marker in ONE redb commit; `cross_accumulator_over_boundary_block` consumes the effect,
validates every binding (`source_epoch`, `target==source+1`, `source_slot`/`source_hash` bound to the block,
exactly-one-boundary) as typed `BoundaryLeadership` terminals, then seals atomically. A boundary advance never
commits without its matching leadership. 23 store + 11 advancer tests green.

**The pool-set correction (the key finding).** The first run mismatched — Ade froze 703 pools (all registered)
vs the reference 658. Root cause: `nesPd` is the DERIVED PoolDistr (`numDelegators > 0`, DC-EPOCH-24), NOT the
full registered set. Verified by a fast probe (v5 @ 1340: `registered=703, delegation_image=658, image∩registered=658`)
then fixed: the leadership set is the pre-POOLREAP delegation-map image ∩ registered pools.

**Layer 3 (reference proof — GREEN, `s5_recovery_replay_equivalence_within_k_rollback`).** The mapping was
DECIDED by the test, printed explicitly:
```
boundary_source_epoch          = 1340
boundary_target_epoch          = 1341
frozen_leadership_target_epoch = 1342
reference                      = POST-1342 nesPd
pool_count ade / ref@target    = 658 / 658
zero_stake_pools               = 32
```
The boundary-frozen leadership byte-matches reference `nesPd_1342` (POST-1342): pool count + ids + stake + VRF
+ 32 zero-stake, via map equality. The leadership hash (`7ff9d8d3…`) is byte-identical across clean advance vs
within-k rollback+reset+refold (`#8` `lead_a==lead_b`) AND warm restart (`warm_lead==lead`). #1 accumulator
hash unchanged from the prior S5 baseline (Layer 2 did not perturb the accumulator).

**Fixture constraint (honest).** ONE boundary is drivable: the v5 store is at epoch 1340 and the corpus starts
mid-1339, so an earlier store (`post1339` @ 1339) has a fold gap. The single `1340→1341 → nesPd_1342 → POST-1342`
comparison is an exact byte-match against the real cardano reference; a second boundary would need a
1339-anchored store foldable from the corpus.

## Commit boundary

Commit S4-pre-2 when the native boundary freeze is proven. After that, S4 proper becomes admissible as the
narrow sealed flip: replace the 3 seed-window production reads, delete the seed+2 ceiling, add the
seed-authority resurrection guard, and prove the former ceiling is crossed with accumulator frozen leadership
only.
