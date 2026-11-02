# SLICE LV-15b-0 — ratification consumes the authority frozen at the start of the epoch

**Cluster:** PREPROD-LIVE-2-FORGE-READINESS. **Follows:** CENSUS-LV-15b §8 (timing settled) and §9 (what the
Mithril snapshot carries). **Precedes:** LV-15b-1 (parameter authority, store semantics 13).
**Rules:** strengthens DC-EPOCH-01 (governance timing) and T-EPOCH-01 (one governance authority); appends
DC-EPOCH-47 (frozen ratification authority). **Store semantics:** 11 → 12.

## 1. Invariant

> Ratification for boundary E→E+1 is determined exclusively by the ratification authority frozen at the start
> of E. Transactions during E change the current governance state for the next cycle; they cannot change E's
> frozen inputs or E's result.

The contract:

- **At E−1→E:** Ade freezes E's ratification inputs from the post-boundary state, computes the ratification
  result from them, and persists that result as authoritative state.
- **At E→E+1:** Ade applies the persisted result to the state current at that boundary. Then it freezes and
  computes for E+1.

```
freeze-time authority (post-boundary state at E start)
  proposals and votes
  DRep stake, delegation, registration, expiry
  SPO stake = the mark built at this boundary
  committee members, quorum, hot keys
  parameters, enacted roots
        ↓
deterministic ratification result        (computed once, at E start)
        ↓ persisted
next boundary E→E+1
        ↓
apply the result to CURRENT state
  current proposals    (removal of enacted, expired and pruned subtrees)
  current accounts     (deposit refunds; registration checked now)
  current treasury     (withdrawals; registration checked now)
```

Tiering:

| tier | statement |
|---|---|
| true | The persisted ratification record replays byte-identically: same ordered inputs give the same record bytes, split or unsplit, and across a restart. |
| derived | Conway ratification timing and frozen-input semantics match Cardano: same frozen inputs give the same result, applied at the same boundary. |
| release | Differential result equality against cardano-node's own snapshot; the late-vote fixtures; bootstrap and restart identity; mutation gates. |
| operational | How or when the result is computed inside E is irrelevant. Ade computes eagerly at the freeze. Cardano's pulser, pulse size and scheduler are not modelled. |

## 2. Cardano's behaviour (cardano-ledger cb57dc73)

From CENSUS-LV-15b §8 and §9:

- `setFreshDRepPulsingState` (`Conway/Governance.hs`) freezes, from the post-boundary `EpochState`:
  proposals, accounts, instant stake, proposal deposits, the mark pool distribution built at this boundary,
  DRep states, committee state, enact state, pools, and the epoch.
- `finishDRepPulser` computes:
  - the DRep distribution (`computeDRepDistr`);
  - the voting pool distribution;
  - `RatifyState = [enactState, enacted, expired, delayed]` over the frozen proposals.
- The DRep distribution counts, for each account delegated to a DRep: instant stake, plus proposal deposits
  whose return address is that account, plus the reward balance. A credential DRep counts only if it is
  registered. AlwaysAbstain and AlwaysNoConfidence always count.
- The voting pool distribution is the mark pool distribution, plus proposal deposits of accounts delegated to
  that pool. Deposits are added only for pools already in the distribution.
- At E→E+1, `Conway/Rules/Epoch.hs` uses only `rsEnactState`, `rsEnacted` and `rsExpired`:
  - `proposalsApplyEnactment` removes them, with their sibling and descendant subtrees, from the **current**
    proposals;
  - `applyEnactedWithdrawals` pays the frozen withdrawal amounts from the **current** treasury, to accounts
    registered **now**;
  - `returnProposalDeposits` refunds to the **current** accounts, sending unregistered ones to the treasury;
  - committee and constitution come from `rsEnactState`;
  - parameters come from `nextEpochPParams`.
- The serialized pulser is always `DRComplete [PulsingSnapshot, RatifyState]`. All three Mithril snapshots
  censused carry it.

## 3. What Ade does today

`rules.rs` `apply_epoch_boundary_with_registrations` builds `ConwayGovernanceEpochInput` at E→E+1 from the
state current at that boundary, and `plan_conway_governance_epoch` both decides and applies in one call:

| input | Ade today | Cardano |
|---|---|---|
| proposals and votes | current at E→E+1 | frozen at E start |
| DRep stake | `derive_drep_voting_stake`: current `vote_delegations` over the pre-boundary mark's **pool** delegations | instant stake + deposits + reward balance, for every DRep-delegated account, registered DReps only, frozen at E start |
| SPO stake | `snaps.go` (built at E−2) | mark built at E start + deposits |
| DRep expiry, dormancy | current | frozen at E start |
| committee hot keys, members, quorum | current | frozen at E start |
| parameters, enacted root | current (equal to E start's: Conway changes them only at the boundary) | frozen at E start |

So beyond timing, the DRep stake computation is known to differ in three ways:
- it omits proposal deposits;
- it omits credentials delegated to a DRep but not to a pool;
- it does not filter unregistered credential DReps.

The SPO stake also omits deposits. §1's derived tier is not met by moving the call one epoch; these
differences are in scope.

The bootstrap decoder skips `ConwayGovState` index 6 (`ledgerdb_state.rs`, `read_conway_pparams_from_utxo_state`).
At the seed's closing boundary, Ade recomputes ratification from current state instead of using the
snapshot's result.

## 4. Design

### 4.1 Split decide from apply

`governance.rs` gets two pure functions in place of the single planner.

**`ratify_frozen(input: &FrozenRatificationInput) -> RatificationRecord`** runs at the freeze. It reads only
the frozen inputs:
- proposals;
- the frozen DRep and SPO distributions (§4.2);
- committee members, quorum and hot keys;
- DRep expiry and dormancy;
- thresholds;
- current parameters and enacted root;
- the epoch E.

It returns the record (§4.3). It contains everything `plan_conway_governance_epoch`'s Phases 1–2 decide
today: the threshold scan, lineage, the single supported winner, its built parameters, and the pruned set.
Its expiry is `expires_after < E`, Cardano's `reCurrentEpoch`.

**`apply_ratification_record(record, current_proposals, is_registered_now) -> ConwayGovernanceEpochPlan`**
runs at the consuming boundary. It reads no votes and no stake:
- it removes the enacted and expired ids from the current proposals;
- it removes every current `ParameterChange` whose lineage descends from a pruned sibling. This includes
  ones submitted after the freeze, as `proposalsApplyEnactment` does;
- it routes every removed deposit by registration now.

`ConwayGovernanceEpochPlan` and its single application point in `rules.rs` are unchanged.

### 4.2 Frozen distributions

**`derive_frozen_drep_distribution`** computes the DRep distribution. For each credential in
`vote_delegations`, its stake is:
- base UTxO stake from the boundary's `BoundaryBaseStake.canonical_credential_stake`, the instant-stake
  analogue;
- plus its post-RUPD reward balance;
- plus the deposits of post-boundary proposals whose return address is that credential.

A credential DRep counts only if its credential is in `drep_expiry` (registered). This replaces
`derive_drep_voting_stake` on the authority path.

**`derive_frozen_pool_distribution`** computes the SPO distribution: the mark pool stakes built at this
boundary, plus the deposits of proposals whose return credential is delegated to that pool, for pools present
in the mark.

Whether base UTxO stake equals Cardano's instant stake at the boundary is a fact the differential (CE-9)
measures. It is not assumed.

### 4.3 The record

```rust
pub struct RatificationRecord {
    pub frozen_at: EpochNo,                         // E; consumed only at E→E+1
    pub source: RatificationSource,                 // Frozen | ImportedFromSnapshot
    pub drep_distribution: BTreeMap<DRep, Coin>,    // evidence for CE-9; read by no rule
    pub pool_distribution: BTreeMap<PoolId, Coin>,  // evidence for CE-9; read by no rule
    pub outcome: RatificationOutcome,
}

pub enum RatificationOutcome {
    Decided {
        enacted: Option<EnactedParameterChange>,    // id, superseded parent root, built pparams
        pruned: BTreeSet<GovActionId>,
        expired: BTreeSet<GovActionId>,
        delayed: bool,
    },
    // A threshold-passing action Ade cannot enact (UnsupportedRatifiedAction, ChainedEnactment,
    // CompetingRatifiableActions, …). Persisted, not raised at the freeze: the halt happens at the
    // consuming boundary, the same boundary where Ade halts today.
    Unenactable(GovernanceTerminal),
}
```

The distributions are evidence for the decision already made, not authority for any later transition.
They are encoded, fingerprinted and replayed byte-identically, so a change to them is visible. No rule may
read them, and that is enforced mechanically:
- the fields are private to `governance.rs`;
- the only accessors are encode, decode, fingerprint and the CE-9 comparator;
- the gate (CE-14) fails if any other production code names them.

They are what makes the record directly comparable with the snapshot's `psDRepDistr` and `psPoolDistr`.

The record lives in `ConwayGovState` as a required field, so both the accumulator and direct-replay paths
carry it. There is no `Option` and no default. A Conway governance state without a record cannot be
constructed, and every constructor names its source.

**Why the halt is deferred.** Halting at the freeze would stop the node a whole epoch before the boundary
that needs the unsupported action. Deferring it keeps today's halt point. The decision itself is still fixed
at the freeze: votes cast during E cannot clear an `Unenactable` outcome.

### 4.4 Boundary order

Inside `apply_epoch_boundary_with_registrations`, on Conway:

1. **Consume.** Take the pre-boundary state's record. Require `frozen_at == new_epoch − 1`; otherwise return a
   typed `GovernanceTerminal::RatificationRecordEpochMismatch`. `Unenactable(t)` returns `t` with zero
   mutation, as now.
2. **Apply.** `apply_ratification_record` against the pre-boundary proposals and registrations, then the
   existing application point (rewards, refunds, treasury, parameters).
3. **Freeze.** Over the constructed next state, after the mark, dormancy and committee updates:
   `derive_frozen_*`, then `ratify_frozen`. Store the new record with `frozen_at = new_epoch`.

The accumulator and replay paths both go through this one function, so they stay identical.

### 4.5 Bootstrap

`read_conway_pparams_from_utxo_state` decodes index 6 instead of skipping it:
- `PulsingSnapshot`: `psDRepDistr` and `psPoolDistr` go into the record's evidence fields;
- `RatifyState`: `rsEnacted`, `rsExpired` and `rsDelayed`, with the record's `frozen_at` set to the snapshot
  epoch and `source = ImportedFromSnapshot`.

`rsEnacted` maps into `Decided` as follows:
- **Enacted `ParameterChange`:** its update is decoded by the same supported-key decoder, and its parameters
  are built the same way.
- **Pruned set:** computed by the same lineage rule over the snapshot's current proposals.
- **Enacted action outside the supported subset:** maps to `Unenactable` with the same terminal Ade's own
  ratifier would give.

The import is checked against `rsEnactState`:
- the built parameters' supported fields must equal `ensCurPParams`;
- the new root must equal `ensPrevGovActionIds`' parameter root.

A mismatch is a typed bootstrap refusal. A snapshot whose index 6 is absent or not `DRComplete` is a typed
refusal; there is no recompute fallback.

### 4.6 Store semantics 12

- Encode the record in the governance snapshot and the accumulator. `EPOCH_ACCUMULATOR_SCHEMA_VERSION` goes
  6 → 7, and the record goes into the governance fingerprint.
- `STORE_SEMANTICS_VERSION` goes 11 → 12. A v11 store is refused with the existing typed refusal. The record
  cannot be synthesized from end-of-epoch state, which is the bug. Recovery is a fresh Mithril bootstrap.

### 4.7 Treasury withdrawals

Ade enacts no treasury withdrawal today: a ratified one is `UnsupportedRatifiedAction` (`NotParameterChange`).
This slice keeps that enactment subset unchanged, because it repairs timing and inputs, not coverage. A
ratified withdrawal therefore becomes `Unenactable` at the freeze and halts at the consuming boundary.

`apply_ratification_record` is written so the withdrawal half (frozen amounts, current treasury, current
registration) is where a withdrawal-enactment slice attaches. This slice does not implement it. CE-6
therefore pins the halt, not a payout.

## 5. Proof obligations

1. **Timing.** Nothing read at E→E+1 can change E's decision. `apply_ratification_record` has no
   access to votes, stake, committee, thresholds or expiry. Its signature does not take them, and a
   structural gate checks that it calls no ratification predicate.
2. **Result equality.** For the same frozen state, `ratify_frozen` and `derive_frozen_*` equal
   `finishDRepPulser`. This is measured against cardano-node's own serialized result (CE-9), not argued.
3. **Application.** Applying the record to the current proposals equals `proposalsApplyEnactment`,
   including later proposals in a pruned subtree.
4. **Identity.** The record round-trips through the store byte-exact. The split and unsplit accumulator give
   identical records.

## 6. Mechanical acceptance

The test in `epoch_accumulator.rs` is renamed and inverted:
`lv15b_a_vote_cast_during_epoch_e_enacts_at_e_plus_2_not_e_plus_1`.

- **CE-LV15b0-1 Late vote.** Votes cast during E do not enact at E→E+1, and the proposal is still present
  with those votes after it. Enactment happens at E+1→E+2 with the parameter change and refund.
- **CE-LV15b0-2 Late proposal.** A proposal submitted during E is absent from E's record and present in the
  current set after E→E+1. A passing one is in E+1's record and enacts at E+1→E+2.
- **CE-LV15b0-3 Late DRep changes.** Each of these during E leaves E's DRep distribution and outcome
  unchanged, and shows up in E+1's:
  - a vote re-delegation;
  - a stake movement;
  - a DRep registration or expiry change.

  It includes one case where the change would flip the outcome under boundary-current reads.
- **CE-LV15b0-4 SPO stake basis.** The SPO distribution is the mark built at E start plus deposits, not `go`.
  The fixture's mark and go differ so that the outcome flips between them.
- **CE-LV15b0-5 Committee freeze.** A hot-key authorization or resignation during E does not change E's
  committee tally.
- **CE-LV15b0-6 Application to current state.**
  - Enacted and expired proposals are removed from the current set.
  - A sibling of the winner submitted after the freeze is pruned along with its descendants.
  - A deposit whose return account deregisters during E goes to the treasury, and one that registers goes to
    the account.
  - A ratified treasury withdrawal is `Unenactable` at the freeze and halts with zero mutation at the
    consuming boundary, not before.
- **CE-LV15b0-7 Frozen parameters.** Ratification uses the thresholds and enacted root held at E start.
- **CE-LV15b0-8 Bootstrap import.**
  - Snapshot 6009 (preprod 304) imports a record whose enacted id is `e641ec80…#0` and whose expired id is
    `90de87d4…#0`. The 304→305 enactment matches today's proven result, now taken from the imported record.
  - Preview judge snapshot 1338 imports 3 frozen DRep votes on `b1d1e5be…#0` while current proposals hold 4.
    That is the real-chain witness that frozen and current diverge.
  - A record whose built parameters disagree with `ensCurPParams` is refused.
- **CE-LV15b0-9 Differential result equality.** An Ade store crosses into epoch E. A cardano-node ledger
  snapshot taken during E, from the local docker node, is decoded independently. Ade's record must equal the
  snapshot's index 6 on:
  - the DRep distribution, entry by entry;
  - the pool distribution, entry by entry;
  - the frozen proposal ids;
  - the enacted ids;
  - the expired ids;
  - `delayed`.

  At least one boundary on preprod is required. A preview boundary is also checked if a store is available.
  This is the center of the slice: it does not close on timing alone.
- **CE-LV15b0-10 Restart identity.** Stop and restart mid-epoch: the reloaded record is byte-identical, and so
  is the record after the next boundary.
- **CE-LV15b0-11 Replay identity.** The accumulator split and unsplit, and the direct-replay path, produce
  identical records and identical governance deltas at every boundary in the fixtures.
- **CE-LV15b0-12 Store refusal.** A v11 store is refused with the typed error; a fresh Mithril bootstrap
  writes 12.
- **CE-LV15b0-13 Mutations.** Each must be killed by its intended test and named by the gate:
  - (a) apply reads current votes, by calling the ratifier at the consuming boundary;
  - (b) freeze before applying the previous record, which changes the proposal set seen;
  - (c) SPO basis `go` instead of the new mark;
  - (d) drop proposal deposits from the DRep distribution;
  - (e) drop the registered-DRep filter;
  - (f) skip pruning later-submitted siblings;
  - (g) route refunds by registration at the freeze;
  - (h) accept a record whose `frozen_at` is not `new_epoch − 1`;
  - (i) on bootstrap, recompute instead of importing;
  - (j) a production rule reads the record's frozen distributions.
- **CE-LV15b0-14 Gate.** `ci/ci_check_frozen_ratification_authority.sh` runs the hermetic tests and asserts
  they ran, the structural check from §5.1, and the store version.
- **CE-LV15b0-15 Sweep.** Failures identical to the LV-15a baseline (284 scripts, 46 failing), except
  where this slice changes them and says so.
- **CE-LV15b0-16 Live.** A fresh v12 Mithril bootstrap on preprod follows across at least one boundary to
  the peer tip with no failure line. CE-9 is taken on that boundary.

## 7. Order of work

1. **Census, before code.** At a real boundary, diff Ade's inputs against the snapshot's `psDRepDistr` and
   `psPoolDistr`. Each cause is measured separately, so the result is a causal map, not one aggregate gap:
   - reward balances in DRep voting stake;
   - proposal deposits in DRep voting stake;
   - credentials delegated to a DRep but not to a pool;
   - stake delegated to unregistered DReps, which Cardano excludes;
   - proposal deposits in SPO voting stake;
   - which epoch's SPO distribution is consumed (`go` against the new mark).

   Every difference is explained before code.
2. **Pure functions.** The record type, `derive_frozen_*`, `ratify_frozen` and `apply_ratification_record`,
   with CE-1..7.
3. **Boundary order and persistence.** Wiring, and store semantics 12. CE-10..12.
4. **Bootstrap import.** CE-8.
5. **Mutations and gate.** CE-13 and CE-14, then the sweep (CE-15).
6. **Live.** CE-16 and CE-9.

## 8. Not in this slice

- Treasury-withdrawal enactment, and every other action kind outside the supported subset (§4.7).
- Admission and forge parameter authority, the complete parameter record, and the tick-before-body split.
  These are LV-15b-1.
- Parameter-dependent rejection. That is LV-15b-2.
