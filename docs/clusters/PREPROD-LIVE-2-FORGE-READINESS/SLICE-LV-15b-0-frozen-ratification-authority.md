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

**What the step-1 census proves, and what it does not.** The census (§7 step 1) proves *reconstruction*
equivalence. Ade's dumped state, combined by hand under the corrected model, equals Cardano's frozen
distributions for every enumerated entry at two preprod boundaries. It does not prove *native transition*
equivalence: Ade's own v12 transitions producing those distributions. Nor does it cover governance
lifecycle states absent from those two epochs. The slice closes only on native results (CE-9). Until
then the corrected distributions are a model, not proven authority.

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

The delegation lifecycle also differs (census §4; `gov_cert.rs`, `delegation.rs`):
- account unregistration keeps the credential's vote delegation;
- DRep unregistration only removes the DRep's expiry, keeping its delegators;
- a vote delegation to an unregistered credential DRep is accepted.

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

**`apply_ratification_record(record, new_epoch, current, is_registered_now) -> ConwayGovernanceEpochPlan`**
runs at the consuming boundary. It reads no votes and no stake. It follows `proposalsApplyEnactment`
(cardano-ledger cb57dc73, `Governance/Proposals.hs`), in this order:
- it removes the expired ids **with their descendants in every purpose tree**
  (`proposalsRemoveWithDescendants`). Ade's planner removed only the expired proposal itself, which was a
  gap independent of timing;
- it removes the enacted winner;
- it removes the winner's siblings (current `ParameterChange`s sharing its superseded root, including ones
  submitted after the freeze) with their descendants. The winner's own children are the new lineage and
  stay;
- it routes every removed deposit by registration now.

A sibling that is also expired is removed as expired, as Cardano does; the refund is the same.

`ConwayGovernanceEpochPlan` and its single application point in `rules.rs` are unchanged.

**Implementation shape.** Both halves live in `governance.rs` as `decide_ratification` (the planner's old
Phases 1–2, verbatim, taking the ratification epoch explicitly) and `apply_decision`. The frozen wrappers
`ratify_frozen` and `apply_ratification_record` are in `governance::frozen`.
`plan_conway_governance_epoch` stays public for the census and fixture tests, as exactly
`apply_decision(decide_ratification(...))` over one proposal set. That leaves one implementation of each
half. It is not an authority path: the gate (CE-14) fails if boundary code calls it.

### 4.2 Frozen distributions

**`derive_frozen_drep_distribution`** computes the DRep distribution. For each **registered** stake
credential in `vote_delegations`, its stake is:
- base UTxO stake from the boundary's `BoundaryBaseStake.canonical_credential_stake`, the instant-stake
  analogue;
- plus its reward balance *after* every boundary credit, including deposit refunds. The mark does not
  qualify, because it is taken before those credits (census §2 (c));
- plus the deposits of post-boundary proposals whose return address is that credential.

A credential DRep counts only if its credential is in `drep_expiry` (registered). This replaces
`derive_drep_voting_stake` on the authority path.

**Delegation lifecycle (census §4).** These are the reference rules (cardano-ledger cb57dc73), for every
protocol version from 10 on:
- **Account unregistration** (`Rules/Deleg.hs` `ConwayUnRegCert`, `State/Account.hs`
  `unregisterConwayAccount`) extracts the whole Conway account state. That is the balance, the deposit,
  the pool delegation and the DRep delegation. Ade already removes the first three. This slice makes
  `AccountUnregistration` and `AccountUnregistrationDeposit` also remove the vote delegation, in the same
  certificate step.
- **DRep unregistration** (`Rules/GovCert.hs` `ConwayUnRegDRep`) clears the DRep delegation of every
  account in that DRep's `drepDelegs` (`clearDRepDelegations`). Ade keeps no reverse index. The
  equivalent is to remove every `vote_delegations` entry targeting `DRepCredential(cred)`, together with
  the expiry. `drepDelegs` is exactly the set of accounts currently delegated to that DRep (maintained by
  `unDelegReDelegDRep` and reset by the PV10 hard fork), so the two are the same set.
- **Delegation to an unregistered DRep** (`Rules/Deleg.hs` `checkDRepRegistered`) is rejected with
  `DelegateeDRepNotRegisteredDELEG` outside the PV9 bootstrap phase. The PV10 hard fork
  (`Rules/HardFork.hs` `updateDRepDelegations`) removed every dangling delegation that existed. Ade's
  vote-delegation certificate gets the same check, as a typed `LedgerError`.

The reference expected result for a delegation to an unregistered DRep is therefore not "excluded from the
distribution". It is "cannot exist". `computeDRepDistr`'s `Map.member cred regDReps` filter is defensive;
its own comment says the delegation "is guaranteed to exist". Ade states this as an invariant, not a
filter. `derive_frozen_drep_distribution` fails closed with a typed terminal if it ever sees a vote
delegation from an unregistered account, or to an unregistered credential DRep. Neither case is silently
dropped.

How each part of this is established:
- **Derived:** valid Cardano transitions from PV10 on cannot produce a dangling DRep delegation. This rests
  on the reference source above, not on the census.
- **True:** Ade never silently ignores or repairs unexpected authoritative state. A dangling delegation is
  a typed terminal, not a filter.
- **Release:** constructed negative tests show the deterministic rejection at every entry point that can
  carry one in: block application, the freeze, bootstrap import and store recovery.

The reference rules do not prove Ade cannot meet such a delegation through a corrupted checkpoint, older
persisted state or a malformed import. That is why the terminal exists.

These are two different failure authorities and stay separate types. A certificate that delegates to an
unregistered DRep is an ordinary invalid transaction (`CertFailureReason::DRepNotRegistered`); the block
is rejected and the state is unchanged. A dangling delegation found in authoritative state is a broken
internal invariant (`GovernanceTerminal::DanglingVoteDelegation`), and it halts.

**Protocol-version floor.** These rules hold from PV10. Bootstrap refuses a Conway state below PV10
with `NativeNonUtxoError::UnsupportedProtocolVersion`. The check is in `decode_native_nonutxo_state`, a
pure decode, so it fires before any state is constructed or persisted. It restricts Ade's supported
bootstrap inputs. It does not claim that protocol versions below 10 are invalid on Cardano. PV9 bootstrap
semantics are out of scope.

### 4.2a Temporal separation in the types

The frozen derivation reads from three sources observed at different points. Each gets a distinct type,
so none can be passed where another is expected. There is no generic "stake" function with flags.
- **`FrozenPoolStake`**: the mark built at this boundary, before the boundary credits. It is used only for
  the SPO distribution.
- **`PostBoundaryBalances`**: the UTxO stake and reward balances after every boundary credit. They are
  used only for the DRep distribution.
- **`FrozenGovernanceView`**: the proposals, votes, DRep registrations and expiry, committee, parameters
  and roots of the constructed next state.

`FrozenRatificationInput` has one constructor. It takes the constructed next state and the boundary's base
stake, and builds all three. `ratify_frozen` takes only a `FrozenRatificationInput`. The consuming
boundary's `apply_ratification_record` takes the record and a `CurrentGovernanceState`, a fourth type that
cannot be converted into any of the three.

**`derive_frozen_pool_distribution`** computes the SPO distribution: the mark pool stakes built at this
boundary, plus the deposits of proposals whose return credential is delegated to that pool, for pools present
in the mark.

Whether base UTxO stake equals Cardano's instant stake at the boundary is a fact the differential (CE-9)
measures. It is not assumed.

### 4.3 The record

```rust
pub struct RatificationRecord {                     // all fields private to governance::frozen
    frozen_at: EpochNo,                             // E; consumed only at E→E+1
    source: RatificationSource,                     // Frozen | ImportedFromSnapshot
    drep_distribution: BTreeMap<DRep, Coin>,        // evidence for CE-9; read by no rule
    pool_distribution: BTreeMap<PoolId, Coin>,      // evidence for CE-9; read by no rule
    outcome: RatificationOutcome,
}

pub enum RatificationOutcome {
    Decided(RatificationDecision),                  // Cardano's rsEnacted / rsExpired
    // A threshold-passing action Ade cannot enact (UnsupportedRatifiedAction, ChainedEnactment,
    // CompetingRatifiableActions, …). Persisted, not raised at the freeze: the halt happens at the
    // consuming boundary, the same boundary where Ade halts today.
    Unenactable(GovernanceTerminal),
}

pub struct RatificationDecision {
    pub enacted: Option<EnactedParameterChange>,    // id, superseded parent root, built pparams
    pub expired: BTreeSet<GovActionId>,
}
```

Two fields of the first draft are gone. They existed only in this document; no store ever persisted
them, so dropping them changes no recovery behaviour and adds no dependence on another snapshot moment:
- **`pruned`**: Cardano computes pruning when the result is applied, over the proposals current then.
  Freezing it as well would give two sources for one removal.
- **`delayed`**: `rsDelayed` is true only after a delaying action is enacted. Ade maps any enacted
  non-`ParameterChange` to `Unenactable`, so a `Decided` record always has `delayed = false`. CE-9 checks
  that the snapshot agrees.

The distributions are evidence for the decision already made, not authority for any later transition.
They are encoded, fingerprinted and replayed byte-identically, so a change to them is visible. No rule may
read them, and that is enforced mechanically:
- the fields are private to `governance::frozen`;
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

This function is the only epoch-transition authority. Consume, apply and freeze are steps inside it. No
governance code decides on its own when an epoch starts or ends, and no second boundary path exists for
the record.

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

The imported cert state must also satisfy §4.2's lifecycle invariant. A snapshot with a DRep delegation
from an unregistered account, or to an unregistered credential DRep, is a typed refusal. It is not
cleaned up on import.

### 4.6 Store semantics 12

- Encode the record in the governance snapshot and the accumulator. `EPOCH_ACCUMULATOR_SCHEMA_VERSION` goes
  6 → 7, and the record goes into the governance fingerprint.
- `STORE_SEMANTICS_VERSION` goes 11 → 12. A v11 store is refused with the existing typed refusal. The record
  cannot be synthesized from end-of-epoch state, which is the bug. Recovery is a fresh Mithril bootstrap.
  No migration is specified, so none is offered; a version bump alone is not migration evidence.

**Step 2 already changes live execution.** The step 2 rules are not behind the unwired record. They are
reachable in production today:
- block application (`rules.rs` `accumulate_tx_certs` → `apply_conway_gov_cert`, and
  `apply_stake_deregistration`): account and DRep unregistration, and the rejection of a delegation to an
  unregistered DRep;
- the boundary planner (`rules.rs` → `plan_conway_governance_epoch`): removal of an expired proposal's
  descendants;
- bootstrap (`decode_native_nonutxo_state`): the protocol-version floor.

A v11 store can hold vote delegations that these rules now say cannot exist, such as delegations left
behind by an earlier account or DRep unregistration. Running the step 2 rules over such a store would mix
semantics. Step 2's code therefore reaches `main` only together with store semantics 12, never alone.

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
- **CE-LV15b0-3a Delegation lifecycle.**
  - An account that deregisters during E has no vote delegation afterwards. Its balance, deposit and pool
    delegation are also gone, and it is absent from E+1's DRep distribution.
  - That account then re-registers without delegating its vote. It has no vote delegation and is still
    absent from the DRep distribution. It counts again only after a new vote-delegation certificate.
  - A DRep deregisters, and every account that delegated to it loses its vote delegation. The same DRep
    credential then re-registers: none of the old delegators return, and its distribution entry is absent
    until a new delegation arrives.
  - The census credential `94857be3…` case is replayed as a fixture.
- **CE-LV15b0-3b Delegation to an unregistered DRep (reference-derived).** The expected results come from
  the cited cardano-ledger rules (§4.2), not from Ade:
  - a vote-delegation certificate naming an unregistered credential DRep is rejected with the typed error
    (`DelegateeDRepNotRegisteredDELEG`);
  - a DRep registration and a delegation to it in the same transaction, in that order, is accepted;
  - `derive_frozen_drep_distribution` given a constructed state with a dangling delegation returns the
    typed terminal; it does not exclude the entry;
  - a bootstrap snapshot carrying a dangling delegation is refused (§4.5).

  The real-chain side is the step-1 census: zero dangling delegations at 305 and 322.
- **CE-LV15b0-4 SPO stake basis.** The SPO distribution is the mark built at E start plus deposits, not `go`.
  The fixture's mark and go differ so that the outcome flips between them.
- **CE-LV15b0-5 Committee freeze.** A hot-key authorization or resignation during E does not change E's
  committee tally.
- **CE-LV15b0-6 Application to current state.**
  - Enacted and expired proposals are removed from the current set.
  - An expired proposal's descendants go with it, in every purpose tree, each refunded.
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
  - `rsDelayed`, which must be false wherever Ade's outcome is `Decided`.

  The record must come from Ade's own v12 transitions. No test-only correction, no hand combination of
  dumped fields, and no oracle-derived runtime input is allowed. The required boundaries are those of the
  step-1 census:
  - **305**: a fresh v12 bootstrap of Mithril snapshot 6009 imports epoch 304's record. It then crosses
    304→305 and freezes 305 natively. Compare with `oracle-end304-node/db/ledger/130133280`.
  - **322**: the same store follows the chain from 305 across every boundary to 321→322. It is not
    restarted from a v11 store, which §4.6 refuses. Compare with the saved copy of the peer's
    `db/ledger/137850288`.
  - **The live boundary the run reaches last**: compare with a snapshot copied from the local docker peer
    during that epoch.

  A preview boundary is also checked if a store is available. This is the center of the slice: it does not
  close on timing alone.
- **CE-LV15b0-10 Restart identity.** Two runs from the same store copy must give byte-identical
  authoritative outputs, both after the reload and after the next boundary:
  - **warm restart**: a clean stop mid-epoch, then a restart;
  - **crash recovery**: SIGKILL mid-epoch, with no clean shutdown, then a restart.

  The outputs compared are the record, including both frozen distributions; the vote delegations and DRep
  registrations; and the governance fingerprint.
- **CE-LV15b0-11 Replay identity.** The accumulator split and unsplit, and the direct-replay path, produce
  identical records and identical governance deltas at every boundary in the fixtures. In addition, a
  **cold replay** gives the same outputs at the same chain point as the live store (CE-10's list, plus the
  delegation lifecycle state). The cold replay is a second fresh bootstrap of the same snapshot, folded
  over the same blocks.
- **CE-LV15b0-12 Store refusal.** A v11 store is refused with the typed error; a fresh Mithril bootstrap
  writes 12.
- **CE-LV15b0-13 Mutations.** Each must be killed by its intended test and named by the gate:
  - (a) apply reads current votes, by calling the ratifier at the consuming boundary;
  - (b) freeze before applying the previous record, which changes the proposal set seen;
  - (c) SPO basis `go` instead of the new mark;
  - (d) drop proposal deposits from the DRep distribution;
  - (e) drop the fail-closed check on a dangling delegation, so the derive silently skips it;
  - (f) skip pruning later-submitted siblings;
  - (g) route refunds by registration at the freeze;
  - (h) accept a record whose `frozen_at` is not `new_epoch − 1`;
  - (i) on bootstrap, recompute instead of importing;
  - (j) a production rule reads the record's frozen distributions;
  - (k) account unregistration leaves the vote delegation;
  - (l) take DRep reward balances from the mark instead of post-boundary balances;
  - (m) DRep unregistration leaves its delegators' vote delegations, killed by CE-3a's re-registration case;
  - (n) a vote delegation to an unregistered credential DRep is accepted, killed by CE-3b;
  - (o) an expired proposal's descendants are kept.
- **CE-LV15b0-14 Gate.** `ci/ci_check_frozen_ratification_authority.sh` runs the hermetic tests and asserts
  they ran, the structural check from §5.1, and the store version.
- **CE-LV15b0-15 Sweep.** Failures identical to the LV-15a baseline (284 scripts, 46 failing), except
  where this slice changes them and says so.
- **CE-LV15b0-16 Live.** The CE-9 run, a fresh v12 bootstrap of snapshot 6009 on preprod, follows from 304
  to the peer tip with no failure line. It crosses every boundary from 305 on, and CE-9 is taken at 305,
  at 322 and at the last live boundary. The run's evidence must record:
  - the node's own PID, not a wrapper's;
  - its exit status and termination reason if it stops;
  - its last durable progress point;
  - which of three kinds any stop was: a clean shutdown on a signal Ade received and logged, a clean halt
    the loop chose, or an unexpected disappearance.

  A zero failure-line count is not liveness evidence. Liveness is a live PID plus a growing log and an
  advancing durable tip.

  This CE closes the slice's live leg only. It is not a sustained-operation claim, which stays blocked by
  §9.

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

   **Done:** `docs/evidence/run-stores/preprod-live2c/lv15b0-step1-frozen-distribution-census.md`. At
   304→305 and 321→322, the corrected model equals Cardano entry for entry: DReps 181/181 and 194/194,
   pools 505/505 and 514/514. It added the delegation-lifecycle fixes (§4.2) and the post-boundary balance
   rule.
2. **Pure functions.** The delegation-lifecycle certificate fixes; the temporal input types (§4.2a); the
   record type; `derive_frozen_*`, `ratify_frozen` and `apply_ratification_record`. With CE-1..7, 3a
   and 3b, and the PV-floor check.

   **Done locally (a1d36385, not on `main`):** the unit tests pass. The rules change live execution
   (§4.6), so they need more than unit tests: CE-9's differential must show that the lifecycle changes
   and the expired-descendant removal give Cardano's results. That covers which descendants are removed,
   where each refund goes, the effect on reward balances, and the next frozen distributions. Until then
   these are tested rules, not proven-equivalent transitions.
3. **Boundary order and persistence.** Wiring, and store semantics 12. CE-10..12. Lands on `main`
   together with step 2.
4. **Bootstrap import.** CE-8.
5. **Mutations and gate.** CE-13 and CE-14, then the sweep (CE-15).
6. **Live and native reproduction.** CE-16 and CE-9 at 305, 322 and the last live boundary, plus CE-10's
   warm and crash restarts and CE-11's cold replay on copies of that store.

## 8. Not in this slice

- Treasury-withdrawal enactment, and every other action kind outside the supported subset (§4.7).
- Admission and forge parameter authority, the complete parameter record, and the tick-before-body split.
  These are LV-15b-1.
- Parameter-dependent rejection. That is LV-15b-2.
- The unexplained clean stops (§9).

## 9. Open, separate: unexplained clean stops

The LV-13 and LV-15a runs each ended in a clean halt with no logged cause
(`lv15a-run-ended-clean-halt-2026-10-27.md`). A restarted copy crossing 321→322 shows that the boundary
does not fail deterministically on that path. It does not show that the halt was harmless, and it does not
exclude a defect that depends on state or scheduling. The open issue has three tiers:
- **operational**: process liveness and unexplained termination;
- **release**: sustained live-follow evidence is unproven;
- **true-tier risk**: RED scheduling or lifecycle behaviour may end processing while admissible work
  remains.

The issue does not block this slice's pure or wiring work. It blocks any claim of sustained live operation
until it is resolved or bounded by evidence. CE-16's stop classification is the first new evidence for it.

## 10. Test-environment limitation: `ade_plutus` fixture

`ade_plutus`'s `conway_validity_range_reproducer` test needs `target/aiken_divergent_fixture`, which is
missing on this machine, so that crate's tests do not build. Workspace test runs for this slice exclude
`ade_plutus`, and the result is reported as "all crates except `ade_plutus`", never as a passing
workspace. The limitation is not waived as a slice gate: CE-15's sweep and the closure test run must
either have the fixture restored or list this exclusion explicitly.
