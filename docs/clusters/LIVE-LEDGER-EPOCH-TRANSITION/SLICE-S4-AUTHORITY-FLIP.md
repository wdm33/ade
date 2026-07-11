# S4 — the sealed authority flip: epoch-indexed frozen leadership replaces the seed-window read

> **Status: staged in two sealed slices.** All admissibility preconditions GREEN + committed: CE-3d byte-exact
> (`e476415a`), S5 restart/rollback replay-equivalence (`8d6bf874` + `687fea98`), S4-pre frozen leadership
> authority (`501bf89a`/`13829660`/`3f93252d`/`8cdd1471`), S4-0 epoch-indexed reader (`c7e1c18f`). This declares
> no new invariant; it PROMOTES the frozen leadership authority (DC-EPOCH-25) to the sole production
> leader-schedule source and lifts the seed+2 ceiling. Supersedes `SLICE-S4-contract.md` §1/§7.2 (the disproven
> `from_accumulator(go+active params)` route — LDAT `67890681`).
>
> - **S4-L1 — DONE (this doc's L1 + L2 layers split at a strict boundary).** Retires seed-window authority from
>   the INITIAL/WARM live leadership view: sites 658/840/3397 read `leadership_authority_for_epoch(seed_epoch)`,
>   byte-identical to the retired seed projection (proven), fail-closed with NO seed fallback. Every production
>   first-run route (native + legacy) now seals readable leadership; all end-to-end fixtures seal a
>   leadership-certified store. **S4 is NOT complete:** the seed+2 ceiling and the promotion path
>   (`prepare_authority_for_candidate_slot`, boundary 2+) remain seed-window-bound until S4-L2. This is a staged
>   authority retirement (each of the three sites has exactly ONE authority), not a fallback.
> - **S4-L2 — OPEN.** The real ceiling lift: thread the store through `run_node_sync` → the promotion; read
>   `leadership_authority_for_epoch(candidate_epoch)` for boundary 2+; delete the `epoch_wire.rs` seed+2 ceiling;
>   add the production seed-authority resurrection guard; prove the former ceiling crossed with no seed-window
>   authority. The higher-risk consensus wiring, its own sealed proof.

**Cluster:** LIVE-LEDGER-EPOCH-TRANSITION. **Depends on:** CE-3d + S5 + S4-pre + S4-0 (all GREEN).

---

## 1. The one-line contract

For any epoch `E` the node validates a slot in, the leadership/header-validation authority is
`store.leadership_authority_for_epoch(E).to_pool_distr_view(asc)` — the epoch-indexed frozen leadership
(DC-EPOCH-25), and **nothing else**. Never the seed-window projection
`PoolDistrView::from_seed_epoch_consensus_inputs`, never a window-replay derivation, never a re-import, never a
CLI oracle. If the store cannot answer for `E` (absent object / uncertified / mis-keyed / corrupt), that is a
**fail-closed terminal**, never a silent seed read.

## 2. Census (exhaustive — the production seed-window leadership reads)

`from_seed_epoch_consensus_inputs` on a PRODUCTION leadership path appears at EXACTLY three sites (file-verified,
HEAD `c7e1c18f`):

| site | fn | role | epoch |
|---|---|---|---|
| `node_lifecycle.rs:658` | `run_node_lifecycle_inner` (forge-OFF relay) | initial header-validation `ledger_view` | `record.epoch_no` (seed) |
| `node_lifecycle.rs:840` | `run_node_lifecycle_inner` (forge-ON feed) | initial header-validation `ledger_view` | `record.epoch_no` (seed) |
| `node_lifecycle.rs:3397` | `warm_start_recovery` | recovered `ledger_view` | `sidecar.epoch_no` (seed) |

Every OTHER occurrence STAYS (verified non-authority):
- `native_firstrun.rs:663` — the BOOTSTRAP SEAL (`FrozenLeadershipPoolDistr::from_seed_epoch_consensus_inputs`
  builds the object to persist into the store; it is the bootstrap-certified initial condition, NOT a
  leadership-use read).
- `node_sync.rs:3556` (`seed_pdv`) — inside `#[cfg(test)] mod tests` (line 1918); test helper.
- `seed_consensus_merge.rs:282` — inside a `#[test]`; equivalence test.
- all other `node_sync.rs` hits — comments + tests.

## 3. Architecture (how leadership flows today)

- **Initial view:** one of the 3 sites builds `ledger_view: PoolDistrView` for the seed epoch, wrapped as
  `ActiveEpochAuthority::seed(&ledger_view)` (`epoch_activation.rs`). Per-slot header validation + the forge read
  `authority.pool_distr_view()`.
- **Promotion:** the sync pump (`node_sync.rs:632`) calls
  `epoch_wire::prepare_authority_for_candidate_slot(...)` at each first-post-boundary candidate. It promotes:
  - boundary 1 (seed → seed+1): from `inputs.next_epoch_bridge` (nesPd_{seed+1} projected from the imported
    MARK at bootstrap, DC-EPOCH-15) → `active_view.promote(EpochConsensusView)`;
  - boundary 2 (seed+2): via `try_activate_at_boundary` — the seed **window-replay**;
  - boundary 3+ (`candidate >= seed+2 && != seed+2`): **FAIL CLOSED** — `WindowReplayPrepare("window-replay
    beyond seed+2 not yet wired")` at `epoch_wire.rs:624-628` — **the seed+2 ceiling**.
- **Native freeze (S4-pre-2):** as the accumulator advances across each self-derived boundary
  (`cross_accumulator_over_boundary_block`), `advance_with_current_leadership` seals `nesPd_{E+1}` into the
  epoch-indexed store. So for every epoch the node crosses, `leadership_authority_for_epoch(E)` already answers
  natively — that is what replaces both the window-replay and the ceiling.

## 4. The flip (four layers, ONE sealed slice — no dual authority for even one commit)

**(L1) Initial view — the 3 sites.** Replace
`PoolDistrView::from_seed_epoch_consensus_inputs(record)` with
`epoch_accumulator.leadership_authority_for_epoch(record.epoch_no)?.to_pool_distr_view(record.active_slots_coeff)`.
- `epoch_accumulator: Option<&EpochAccumulatorStore>` is already in scope at 658/840 (opened
  `node_lifecycle.rs:580`, the comment there already reads *"S4 makes it the leadership authority"*); thread it
  into `warm_start_recovery` for 3397.
- Byte-identical to the seed projection: S4-0's 1c lineage test proved
  `leadership_authority_for_epoch(seed) == from_seed_epoch_consensus_inputs(record)`.
- **Fail closed:** if the store is absent / uncertified / missing the seed epoch while a live feed is wired
  (`--peer`), return the SAME terminal shape the seed path used (`FeedMissingRecoveredConsensusInputs` /
  a typed leadership terminal). Never an empty view, never accept-if-missing. (`asc` comes from the seed record's
  `active_slots_coeff` — the venue genesis constant, geometry not leadership; it is NOT a leadership-authority
  read and the grep-gate targets the `from_seed_epoch_consensus_inputs` call, which is gone.)

**(L2) Promotion — boundary 2+.** Thread `epoch_accumulator: Option<&EpochAccumulatorStore>` through
`run_node_sync` (`node_sync.rs:556`) → the pump → `prepare_authority_for_candidate_slot`. For
`candidate_epoch >= seed+2`, promote from the native store:
`active_view.promote(EpochConsensusView::from(store.leadership_authority_for_epoch(candidate_epoch)?.to_pool_distr_view(asc)))`
— no window-replay, no bridge, no seed read. Fail closed (`LeadershipEpochNotSealed`/`LeadershipEpochMismatch`)
if the epoch is not yet natively sealed. Boundary 1 (seed+1) MAY stay on the bootstrap bridge (it is the
bootstrap-certified nesPd_{seed+1}, identical to `leadership_authority_for_epoch(seed+1)`) OR also read the store
— the slice reads the store for BOTH so there is one path; the eta0/nonce tick machinery (DC-EPOCH-16) is
unchanged.

**(L3) Delete the ceiling.** Remove `epoch_wire.rs:624-628` (`WindowReplayPrepare` seed+2 gate) and the
seed+2-only window-replay special case (630-681) it guards, replaced by the L2 store read for all
`candidate >= seed+1`.

**(L4) CI/static guard.** Extend `ci/ci_check_frozen_leadership_authority.sh`: (a) production leadership paths
(`node_lifecycle.rs`, `node_sync.rs`, `epoch_wire.rs`) carry ZERO `from_seed_epoch_consensus_inputs` outside
`#[cfg(test)]` / the `native_firstrun` bootstrap seal; (b) no `prefer`/`otherwise`/fallback branch between the
accumulator and the seed; (c) the `"window-replay beyond seed+2 not yet wired"` ceiling string is gone.

## 5. Hard prohibitions (user, load-bearing)

No fallback to seed / go / set / active params. No latest/current/nearest leadership read (S4-0 already removed
the bare reader). No feature flag / build / env gate deciding WHETHER the accumulator is authority
([[feedback_no_semantic_activation_gate]]). No dual authority mode / two production leader-authority paths in
any commit. The production read is EXACT `leadership_authority_for_epoch(slot_epoch)`; if missing → fail closed.

## 6. Acceptance (S4 is green only when ALL hold)

1. **3 sites flipped** — all read epoch-indexed frozen leadership; seed-window read count on production
   leadership paths = 0 (CI grep-gate).
2. **Ceiling deleted** — `epoch_wire.rs` seed+2 `WindowReplayPrepare` gone; a hermetic proof crosses PAST the
   former ceiling (seed+3 and beyond) with epoch-indexed frozen leadership only — no `rc=43`, no seed read, no
   re-import, no CLI oracle.
3. **Same-epoch byte-identical** — within an already-followed epoch the accumulator-derived authority equals
   what the seed view produced (source-only change), on the existing corpus.
4. **Replay-equivalence** — S5 warm-restart + within-k rollback+reset+refold still byte-identical with the flip
   (the resolved authority is reproducible from the durable accumulator + WAL).
5. **Fail-closed** — a missing / corrupt / uncertified leadership authority halts deterministically; the seed
   authority cannot silently resume control.

## 7. Invariants (enforcement, no new IDs)

Flips DC-EPOCH-19/20/21/22 (self-derived epoch authority) + DC-EPOCH-17 (continuous crossing past seed+2) from
`declared` → `enforced`; promotes DC-EPOCH-25 (frozen leadership) from "persisted + epoch-indexed but not read"
to the SOLE production leader schedule. No governance-coverage expansion (unsupported ratified kinds stay the
fail-closed `UnsupportedRatifiedAction`). No operational reconnect/forge gates. Surgical.

## 8. Commit boundary

ONE sealed commit when the final proof is green: `feat(...): S4 promotes frozen leadership authority` — replace
the 3 production seed-window reads with epoch-indexed frozen leadership; rewire promotion for boundary 2+; remove
the seed+2 ceiling; forbid seed-authority resurrection; prove continuous crossing past the former ceiling with no
seed-window authority. No intermediate commit with two production authority paths.

**Then:** CE-4 continuous multi-epoch operation proof.
