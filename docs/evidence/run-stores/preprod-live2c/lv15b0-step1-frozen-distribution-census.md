# LV-15b-0 step 1 — Ade's voting-stake inputs against cardano-node's frozen distributions

preprod, 2026-11-01/02. Slice: `SLICE-LV-15b-0-frozen-ratification-authority.md` §7 step 1. Census only; no
production code.

**Result.** At two real boundaries, every difference between Ade's ratification stake and cardano-node's
frozen `psDRepDistr` / `psPoolDistr` has a named cause. With all causes applied, the reconstruction equals
Cardano entry for entry:

| boundary | DRep distribution | SPO distribution |
|---|---|---|
| 304→305 | 181 of 181 | 505 of 505 |
| 321→322 | 194 of 194 | 514 of 514 |

Today's inputs give 78/181 and 82/194 for DReps, and 397/505 and 432/514 for pools.

One cause is a state bug Ade's slice doc did not name: a deregistered stake credential keeps its vote
delegation (§4).

---

## 1. Method

**Cardano side.** Each frozen distribution is read from the raw `state` file of a cardano-node LedgerDB
snapshot taken during the epoch, with a disposable `cbor2` reader that is not committed. The path is
`UTxOState.govState[6] = DRComplete [PulsingSnapshot, RatifyState]`, and the frozen distributions sit in
`PulsingSnapshot` fields 1 (DRep) and 3 (pool).

| epoch | snapshot |
|---|---|
| 305 | `oracle-end304-node/db/ledger/130133280` |
| 322 | the local docker peer's `db/ledger/137850288`, copied before rotation |

**Ade side.** Ade's state just after each boundary is captured with
`crates/ade_runtime/tests/frozen_ratification_census.rs` (`#[ignore]`d, run on copies). It dumps:
- vote delegations;
- DRep registrations;
- stake registrations;
- reward balances;
- pool delegations;
- the mark and go;
- per-credential base UTxO stake from the reduced checkpoint;
- proposals;
- today's `derive_drep_voting_stake`.

The node was frozen with SIGSTOP on the `CROSSED boundary` line, its two databases were copied, and it was
then killed. SIGTERM was not used, because it did not stop a catching-up node within 30 s. The checkpoint's
cursor is the boundary's `s_prev`, so the base stake is the boundary's own.

| boundary | Ade store | Ade point (first block of the epoch) |
|---|---|---|
| 304→305 | fresh bootstrap of Mithril snapshot 6009 (`bin-lv15a`, store semantics 11) | 130,118,424; checkpoint 130,118,358 |
| 321→322 | copy of the stopped LV-15a store, restarted | 137,462,401; checkpoint 137,462,335 |

Both copies of the node crossed without a failure line.

## 2. DRep voting stake, by cause

Components are summed over the vote delegators of each DRep. The second column shows how many of the
DReps each cause touches.

| cause | 304→305 | 321→322 |
|---|---|---|
| Today's value: the mark stake of pool-delegated vote delegators | 889,340,775,469,445 (111 DReps) | 832,488,401,640,232 (123 DReps) |
| (a) UTxO stake of delegators with no pool delegation | +36,308,858,033,063 (102) | +97,618,174,426,099 (114) |
| (b) reward balances of those delegators | +7,747,796,648,616 (8) | +9,302,786,682,699 (16) |
| (c) balance credited at the boundary after the mark | +1,500,000,000 (2) | +4,000,000,000 (3) |
| (d) proposal deposits returning to the delegator | 0 (no proposals frozen) | +7,000,000,000 (1) |
| (e) delegation to an unregistered DRep, excluded | 0 | 0 |
| (f) stale delegation of a deregistered stake credential, excluded | −9,986,371,293 (1 credential) | −203,694,235,975 (43 credentials, 7 DReps) |
| **after (a)–(f)** | **181/181 exact** | **194/194 exact** |

Notes:
- Reward balances of pool-delegated delegators are already inside today's mark value: 4,918,414,037,244
  at 305 and 4,417,424,037,754 at 322. They are not a gap.
- (c) is why the frozen DRep stake must read the post-boundary balance, not the mark. Cardano takes SNAP
  before the boundary credits; its DRep distribution includes them and its pool distribution does not.
  Ade's mark matches that. At 304→305 the credits are `03b56d5e…` (+1,000,000,000) and `7a507e54…`
  (+500,000,000), the "credits outside the reward update" already recorded in LV-4 and LV-5.
- (e) never occurred at either boundary. The filter is still required by Cardano's rule. It is exercised by
  a constructed fixture, not by this data.
- AlwaysAbstain gains the most from (a), by 34.3T and 35.8T. Many abstaining wallets delegate their vote
  but not their stake.

## 3. SPO voting stake

| input | 304→305 | 321→322 |
|---|---|---|
| `go` (today) | 397/505, 108 differ, sum −642,880,870,938 | 432/514, 82 differ, sum +1,973,825,905,942 |
| the mark built at this boundary | 505/505 | 513/514; pool `2f29763b…` short by 7,000,000,000 |
| mark + proposal deposits of the pool's delegators | 505/505 | **514/514** |

The single 322 difference is one proposal's 7,000,000,000 deposit, whose return account delegates to
`2f29763b…`. Pools with zero stake (57 at 305, 56 at 322) appear on the Cardano side with 0 and are
counted as equal.

## 4. The stale vote delegation (new)

At 304→305, `94857be3…4dfd5b` is in Ade's `vote_delegations` (AlwaysAbstain) but is not a registered
stake credential. Its UTxO stake, 9,986,371,293, is the whole residual. Earlier records show it registered
later in epoch 304 and had no account in either Ade or Cardano right after the boundary (LV-2, LV-5). So it
deregistered before the boundary, and Ade kept its vote delegation. At 322 the same pattern covers 43
credentials.

**Code.** `gov_cert.rs` lists `ConwayCert::AccountUnregistration` and `AccountUnregistrationDeposit` under
"no governance effect". `delegation.rs` removes the registration, pool delegation and reward balance, but
not the vote delegation.

**Cardano (cb57dc73).**
- `Conway/Rules/Deleg.hs`: unregistration removes the whole account (`unregisterConwayAccount`), so the
  DRep delegation goes with it.
- `Conway/Rules/GovCert.hs` `ConwayUnRegDRep`: DRep unregistration clears the DRep delegation of every
  account delegated to that DRep (`clearDRepDelegations (drepDelegs dRepState)`). Ade only removes the
  DRep's expiry, so a later re-registration of the same DRep credential would bring its old delegators
  back. This did not occur in this data. It is a source finding, to be fixed and pinned by a fixture.

## 5. Consequence for LV-15b-0

Every cause in §2 and §3 changes the frozen inputs, so all are in scope:
- the frozen DRep distribution = registered accounts with a vote delegation to a registered DRep (or
  AlwaysAbstain or AlwaysNoConfidence), each counting base UTxO stake + post-boundary reward balance +
  deposits returning to it;
- the frozen pool distribution = the new mark + deposits of the pool's delegators;
- account unregistration clears the vote delegation, and DRep unregistration clears its delegators'.

The 322 snapshot also shows real-time divergence between frozen and current state on preprod: 5 proposals
submitted after the freeze, and `04f23439…#0` gained an SPO vote after it.
