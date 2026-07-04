# S5 — differential proof + residual report (the unmistakable-state slice)

## The slice

S5 is a **differential PROOF ONLY** — it adds NO governance effect (S4.3c completed the supported enactment).
It assembles the scattered proofs into ONE report that makes the remaining state unmistakable, and it keeps three
claims strictly separate so nothing reads as "done" that is not:

1. **S4.3c slice status = CLOSED** for the supported exec-units parameter-change subset.
2. **S5 / CE-3d status = the governance refund discrepancy is GONE; the remaining residual is quantified** (it is
   the B3c UTxO-component stake undercount, isolated from every governance effect).
3. **Full Conway governance status = still PARTIAL** — unsupported ratified actions, broader parameter changes,
   committee / constitution / treasury / hard-fork enactment, and multi-ratifiable competition all remain
   fail-closed terminals (not enacted). The cluster is NOT globally complete.

Governed by [[feedback_enactment_is_atomic_no_split_effects]] + the honest-status discipline: mark ONLY the
supported subset enforced; keep the broader authority explicitly partial.

## The one report (six items — mechanically backed)

Emitted by `cre_s5_differential_report` (`#[ignore]`, reads local states); this slice doc is the durable
narrative. Each item is asserted where the data is local, cited to its committed hermetic gate otherwise.

1. **The five expired-proposal refund totals + destinations.** Over the REAL POST-1340 proposal set the single
   authority plans exactly five expiry refunds: `00ceb13422f661e2..` (acct1) ← 4 proposals = **+400,000 ADA**,
   `00f53256bcaa4c5e..` (acct2) ← 1 proposal = **+100,000 ADA**; total **500,000,000,000 lovelace**. Backed by
   `cpde_s5_planner_refunds_close_the_500b_on_real_proposals` (committed) + ground-truthed by the POST-1340→POST-1341 extraction (acct1
   500B→900B, acct2 1,400B→1,500B; the reward refs `581de0ceb134..` 16→0 and `581de0f53256..` 4→0 all resolve).
2. **CE-3d reward / pot / accounting comparison, BEFORE vs AFTER.** BEFORE (pre-S4.3a): Ade's direct-replay
   boundary silently DROPPED the five expired proposals with no refund → reward total short by **−500,037,651,836**
   (the 500B refunds + a ~−37.6M rounding tail). AFTER (S4.3a single authority + this proof): the five refunds
   land identically on both paths; treasury/reserves are UNTOUCHED by the refunds (the deposit pot is the source,
   not treasury) — cardano ref@1340 vs Ade acc@1340 treasury/reserves differ by a STABLE +231M / +894M across
   1340/1341/1342 (that is the B3c residual, not growing, not the refunds).
3. **The 1095→1096 enactment observables.** Enacted PParamUpdate root advances `602d8572..#0 → 69c948cd..#0`;
   proposal count **59 → 53**; the six-removal manifest = 1 `Enacted` (69c948cd) + 5 `PrunedByEnactment`
   (f046a882 / c3f38851 / 0176514f / 609896ea / 4bc0ee7f, all sharing `prev_action=602d8572`); both execution-memory
   values `maxTxExUnits.mem 14M→16.5M` + `maxBlockExUnits.mem 62M→72M` (steps preserved); all six 100k deposits
   routed (registered → reward account). Backed by `cre_s4_3c_enactment_differential_1095` (real 1095 state) +
   `cre_s4_3c_classify_1095_1096_removals`.
4. **Replay-vs-accumulator identity.** A real enactment produces byte-identical pparams + gov state + refunds +
   treasury on the direct-replay and accumulator-follow configs. Backed by
   `cre_s4_3c_enactment_is_identical_on_replay_and_accumulator_paths`.
5. **The exact remaining B3c residual, ISOLATED from governance.** The remaining CE-3d gap is the go-stake
   undercount **−343,260,172,883 lovelace** — a uniform ~0.0205% undercount on the BASE UTxO stake of real
   delegated credentials (`sum_base_credential_stake` in the reduced checkpoint), NOT a governance effect. Proven
   independent: the two refund accounts (acct1/acct2) are UNDELEGATED, so they appear in `rewards` and are ABSENT
   from `go` — the governance-refund credential set and the B3c go-stake credential set are DISJOINT. The report
   asserts acct1/acct2 ∉ the POST-1341 `go` snapshot. See [[project_b3c_stake_residual]] +
   [[project_ce3d_reward_gap_decomposed]].
6. **Any terminal encountered, with action id + structured reason.** In the CE-3d (1340→1341) and 1095→1096
   windows the authority reaches a clean plan — NO terminal. The closed terminal surface it WOULD emit (each with
   the offending `action_id`): `UnsupportedRatifiedAction { NotParameterChange | NonExecUnitsField | NoExecUnitsField
   | ChangedSteps | OversizedUpdate | MalformedUpdate | ChainedEnactment | CompetingRatifiableActions }`,
   `UnversionedStateOnEnactPath`, `Malformed { ReturnAddrNotRewardAccount }`, `DormantRequired`.

## Explicit scope bounds (what S5 does NOT claim)

- **The full accumulator-level BYTE-EXACT CE-3d differential is NOT closed by S5.** It awaits (a) a seed
  RE-BOOTSTRAPPED under CRE S1 (the current CE-3d seed predates S1, so its accumulator `gov_state` is `None` and it
  cannot itself refund) AND (b) the B3c UTxO-component fix. Both are SEPARATE, TRACKED. S5 proves the refund
  discrepancy is gone at the AUTHORITY level (the planner refunds the five over the real proposals) and
  ground-truthed against the real chain, and it quantifies the ONLY remaining residual (B3c).
- **`MissingDRepActivityParam` is a SEPARATE continuous-operation blocker, not an S4.3c defect.** The observe-only
  epoch-accumulator boundary cross stalls at `CertApply(ValidationEnvironment(MissingDRepActivityParam))` — a
  pre-existing LIVE-LEDGER-EPOCH-TRANSITION cert-apply limitation (gov_cert.rs / state.rs, commit 06385d0c). It is
  observe-only (does not halt the node) and lives OUTSIDE the CRE governance authority; it is tracked for
  continuous operation, not for S5 closure.
- **The pending-chain-child regression fixture is PERMANENT.** `cre_s4_3c_pending_chain_child_is_carried_not_pruned`
  guards the silent proposal-set + deposit-pot divergence that pruning the winner's own descendants would cause;
  it must never be removed.

## Registry (honest status)

Strengthen `T-EPOCH-01` — its `authority_surface` currently says "ENACTMENT ... is NOT yet performed ... the
enactment half stays partial." S5 updates it to: the SUPPORTED exec-units parameter-change enactment subset is
ENFORCED (atomic delta — pparams + advanced root + removals + deposit returns — proven hermetic + corpus over the
real 1095→1096 state + cross-path identical + live-confirmed no-regression), while broader enactment (other action
kinds, broader parameter fields, committee/constitution/treasury/hard-fork, and multi-ratifiable competition)
stays a PARTIAL, fail-closed terminal. Append the S4.3c + S5 tests. `status` stays `partial` (the cluster is not
globally complete). Do NOT mark the cluster enforced.

## What S5 does NOT do

No new enactment effect; no broader action kind; no B3c fix; no full byte-exact accumulator differential (scope
bound above). S5 is the consolidation + separation report only. **NEXT beyond S5:** S6 (byte-exact per-action
oracle differential, gated on the S1-re-bootstrap + B3c) and the broader-enactment slices (each remaining action
kind, each its own atomic slice), plus the separate `MissingDRepActivityParam` continuous-operation fix.
