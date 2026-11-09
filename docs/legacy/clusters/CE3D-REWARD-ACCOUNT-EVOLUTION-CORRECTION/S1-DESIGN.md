# S1 — boundary reorder design (the staged post-RUPD mark)

The narrow question this answers: **what is the single atomic epoch-boundary transition whose staged
reward-account result is visible to the mark snapshot, while every other Cardano-compatible boundary effect keeps
its required input phase and ordering?**

The good news the code review establishes: the boundary **phase order is already cardano-correct** —
`applyRUpd → SNAP(mark) → POOLREAP → governance → pots`, all computed on an immutable `&state` that is never
mutated in place, with a fresh `new_state` built once at the end and a `GovernanceTerminal` returning `Err`
before any construction. The defect is **one wrong read**: the mark is built from the pre-RUPD reward map even
though the post-RUPD reward map already exists two steps earlier in the same function. This is a **reorder of one
data dependency**, not a re-plumbing of the boundary.

## 1. Current boundary sequence (`apply_epoch_boundary_with_registrations`, rules.rs:639–1430)

All phases read the immutable `&state`; a local `delegation = state.cert_state.delegation.clone()` (`:1169`) is
the single mutable staging buffer for reward accounts; `new_state` is constructed once at `:1403`.

| # | phase | file:line | reads | writes (staged) |
|---|---|---|---|---|
| 0 | **Gov plan** (validate) | 665–708 | `&state` mark (DRep stake), go (pool stake), proposals, registrations | `gov_plan` OR `Err(GovernanceTerminal)` — zero mutation |
| 1 | **Reward inputs** (eta, ΔR1, ΔT1, pot) | 710–818 | `&state` reserves, treasury, pparams, `block_production`, `epoch_fees` | `pool_reward_pot`, `treasury_delta`, `total_stake` |
| 2 | **Pool-reward loop** (native RUPD) | 820–1159 | `&state.snapshots.go` (delegations, pool_stakes), pool params, `block_production` | `reward_deltas: BTreeMap<Hash28,Coin>` |
| 3 | **applyRUpd** | 1168–1206 | `reward_deltas`, `delegation.registrations` | **`delegation.rewards` ← POST-RUPD**; `delta_t2` (unregistered → treasury) |
| 4 | **Mark build + rotate** | 1210–1239 | ✗ `precomputed_mark` (accumulator) **or** `state.cert_state.delegation.rewards` (stub, `:1220`) | `rotated` = rotate_snapshots(state.snapshots, new_mark) |
| 5 | **POOLREAP** | 1241–1304 | `delegation` (post-RUPD), `pool_state`, `retiring==new_epoch` | adopt futures; refund deposits → `delegation.rewards` (post-RUPD) or `poolreap_to_treasury`; clear reaped delegators; drop reaped pools |
| 6 | **Gov refunds apply** | 1306–1354 | `gov_plan.deposit_returns`, `delegation` (post-RUPD, post-POOLREAP) | `delegation.rewards` or `gov_deposit_to_treasury`; `new_gov_state` (+ pparam-action root advance) |
| 7 | **Pots** | 1356–1373 | ΔR1, ΔR2 = pot − Σrewards, ΔT1, ΔT2, poolreap→treasury, gov→treasury | `new_reserves`, `new_treasury` |
| 8 | **Construct next state** | 1403–1427 | all staged | one `LedgerState` (rotated snapshots, pots, post-everything cert_state, enacted pparams, `new_gov_state`) |

Accumulator wrapper (`cross_epoch_boundary`, epoch_accumulator.rs:439–563), current order:
build mark `:486` (**PRE-RUPD**, from `acc.cert_state.delegation`) → set reward inputs `:494` (seed → empty;
else prev nesBcur/fees) → call the boundary fn `:508` (passes `Some(&new_mark)`) → read back `:521` → apply the
one-shot **bootstrap RUPD** `:531–562` (reserves/treasury deltas + `apply_bootstrap_reward_deltas` to accounts),
**AFTER** the fn.

## 2. The exact incorrect visibility point

**Phase 4, `rules.rs:1213`.** The reward accounts are already POST-RUPD in `delegation.rewards` from phase 3, but
the mark is built from the *pre-RUPD* view:

- **Stub / direct-replay path** (`:1217–1234`): reads `state.cert_state.delegation.rewards` — the original
  pre-boundary map, not the post-RUPD `delegation`.
- **Accumulator path**: the `precomputed_mark` was built at `epoch_accumulator.rs:486` from
  `acc.cert_state.delegation` **before** the boundary fn ran — necessarily pre-RUPD.

Cardano's `SNAP` runs after `applyRUpd`, so its mark reads post-RUPD rewards (proven:
`crae_cardano_mark_is_base_plus_reward` — cardano's mark == base + POST-RUPD reward for 59,687/59,701 creds). Ade
therefore undercounts each delegated credential's mark stake by exactly the boundary RUPD payout (−343/−363/−355B
across go/set/mark; `crae_rupd_accrual_equals_residual` = +315,961,836,959).

## 3. Target staged sequence (one immutable start → one atomic commit)

The reorder makes the mark read the **staged post-RUPD reward accounts** — the exact reward result phase 3
already computes — while nothing else moves. Concretely:

**S0. Immutable pre-boundary state** `&state`. No phase mutates it; `delegation` (clone) + `pool_state` (clone) +
scalar pot deltas are the only staging.

**S1. Compute the reward update against correct pre-boundary inputs** (phases 0–2, UNCHANGED): gov plan
(terminal → `Err`, zero mutation), reward inputs, the native `reward_deltas` from the pre-boundary `go` snapshot
and `block_production`.

**S2. Stage the reward-account delta** (phase 3, UNCHANGED position): apply `reward_deltas` → post-RUPD
`delegation.rewards`; unregistered → `delta_t2`. For the **seed boundary** the native delta is empty by
construction and the **bootstrap RUPD's `reward_delta` is staged HERE instead** (see §Native-vs-bootstrap) so the
staged reward view is post-RUPD for both cases before the mark.

**S3. Build the boundary mark from the STAGED post-RUPD reward view + the exact per-credential base UTxO**
(phase 4, the ONLY change): `new_mark = build_boundary_mark_snapshot(boundary_base_utxo, &delegation)` — where
`delegation` is the post-RUPD buffer from S2 and `boundary_base_utxo` is the byte-exact
`sum_base_credential_stake` (proven exact: B3c.0 + `crae_advanced_base_at_post1341`). The stub/direct path builds
its per-credential mark from `&delegation` (post-RUPD) too. **Delegations are read pre-POOLREAP** (S5 has not run),
matching cardano's SNAP-before-POOLREAP order.

**S4. Rotate** mark/set/go (phase 4 tail, UNCHANGED): `rotate_snapshots(state.snapshots, new_mark)`.

**S5. POOLREAP** (phase 5, UNCHANGED): on the post-RUPD `delegation` + `pool_state`, in the reference-proven
order (adopt futures → reap `==new_epoch` → refund deposits to post-RUPD reward accounts / treasury → clear
reaped delegators → drop reaped pools).

**S6. Governance refund/enactment plan apply** (phase 6, UNCHANGED): `gov_plan.deposit_returns` →
post-RUPD/post-POOLREAP reward accounts or treasury; build `new_gov_state`.

**S7. Pots** (phase 7, UNCHANGED): ΔR1/ΔR2/ΔT1/ΔT2 + poolreap→treasury + gov→treasury (+ bootstrap
reserves/treasury deltas at the seed boundary — see below).

**S8. Construct ONE next state** (phase 8, UNCHANGED): fresh `LedgerState`, or the early `Err` terminal — zero
mutation of `&state` either way.

**The entire change is: S3 reads `&delegation` (staged post-RUPD) instead of the pre-RUPD map, and the
accumulator path passes `boundary_base_utxo` instead of a pre-built `precomputed_mark`.** The phase order,
POOLREAP, governance, and pot arithmetic are untouched.

## 4. Per-step input state and produced delta (the dependency the reorder fixes)

The only dependency edge that moves: `mark ← reward accounts` changes its source from
`state.cert_state.delegation.rewards` (pre-RUPD) to the S2 staged `delegation.rewards` (post-RUPD). Every other
edge is unchanged:
- `reward_deltas ← go snapshot + block_production` (pre-boundary): unchanged (rewards must be computed from the
  pre-rotation go, before rotation empties it).
- `POOLREAP ← post-RUPD delegation`: unchanged (already correct — reap refunds land in post-RUPD accounts).
- `gov refunds ← post-RUPD, post-POOLREAP delegation`: unchanged.
- `pots ← ΔR1/ΔR2/ΔT1/ΔT2 + reap/gov treasury`: unchanged.
- `mark delegations ← pre-POOLREAP delegations`: preserved (mark built at S3, POOLREAP at S5).

## 5. Native RUPD vs one-shot bootstrap RUPD

Currently the bootstrap RUPD is applied in `cross_epoch_boundary` **after** the boundary fn (`:531–562`), so a
mark built inside the fn would miss it at the seed boundary. The design **unifies the reward-account delta
staging**: the boundary fn stages a single reward-account delta at S2 that is the native `reward_deltas`
(non-seed) OR the bootstrap RUPD's `reward_delta` (seed). Mechanism: pass the (already commitment-verified)
bootstrap reward-account delta into the boundary fn for the seed boundary; it is applied at S2 before the mark at
S3; the bootstrap **pot** deltas (reserves/treasury) are applied at S7 (they do not affect the mark). This keeps
the one-shot bootstrap semantics (verify commitment, consume exactly once, fail-closed on wrong boundary) while
making its reward accounts visible to the seed-boundary mark — the same post-RUPD invariant as the native path.
(For CE-3d the seed at epoch 1340 has `pending_reward_update = None`, so 1340→1341/1341→1342 are native-only and
this branch is inert; it is required for a fresh bootstrap's seed→seed+2 correctness.)

## 6. Direct-replay path, warm-restart/WAL, and the terminal (confirmed from code review)

- **Direct-replay caller.** `apply_epoch_boundary_full` (rules.rs:612), called from
  `apply_block_with_ledger_accounting` (rules.rs:123), passes `precomputed_mark = None` → the **stub** mark
  (rules.rs:1217–1234). The stub-reads-post-RUPD fix (S3) covers its reward component. **Nuance (surfaced, likely
  out of scope):** the stub's per-credential stake is REWARD-ONLY — it does not fold base UTxO (the direct
  full-ledger path never wired a base-UTxO mark). So the direct path's mark stays a legacy projection even after
  this fix; byte-exact mark/set/go is achieved on the **accumulator path** (which supplies base+reward). The
  reorder does not make the two paths agree — that pre-existing gap is a separate decision. The fix's scope is:
  (a) accumulator path mark = base + POST-RUPD reward (the CE-3d target); (b) stub reward component becomes
  post-RUPD too (a strict improvement, no regression). **Confirm with the user whether the direct/full-ledger
  path must also reach byte-exact here, or stays legacy.**
- **Warm-restart / WAL / recovery (byte-identical preserved).** The durable `EpochAccumulatorStore`
  (ade_runtime/src/chaindb/epoch_accumulator_store.rs) writes the accumulator blob + last-slot in ONE redb commit
  per advance; the boundary witness is bound durably in a separate commit BEFORE the cross
  (`bind_boundary_mark`); reorg is a reset-to-bootstrap + replay-forward (no inverse mutation); recovery
  re-folds `apply_selected_block` over the durable selected chain (the accumulator is GREEN-derivable). The
  reorder changes ONLY the staged mark VALUE inside the pure `(state,input)→state` transition — not the
  commit shape, the witness, or the recovery re-fold. So replay, restart, and reorg re-materialization all
  reproduce the corrected mark byte-identically (R-RAE-1 holds by construction).
- **Structured terminal / zero mutation (confirmed).** The boundary fn takes `state: &LedgerState`; the first and
  only mutation target is the `delegation` clone (rules.rs:1169); `plan_conway_governance_epoch`'s `?`
  (rules.rs:700–702) returns `Err(GovernanceTerminal)` BEFORE any clone or construction; the next state is one
  atomic expression (rules.rs:1403–1427). A terminal yields zero partial state, unchanged by the reorder (the
  gov plan still runs first, at S1).

## 7. Why this is a reorder, not an offset (invariants held)

No constant is added anywhere; `build_boundary_mark_snapshot`'s arithmetic (`base + reward`, group-by-pool) is
unchanged — it is simply fed the reward map that already exists post-`applyRUpd`. The credited amounts are the
same reward payouts the pots already account for (Σrewards, ΔR2), so **pot conservation is automatically
preserved** (the reorder moves no lovelace; it only changes which snapshot observes the already-credited
rewards). This is the mechanical guarantee that the −343B closes to exactly zero without a compensating term.

## 8. AMENDMENT (user-mandated): correct the direct/full-ledger path in the SAME slice

Leaving the direct/full-ledger path on a reward-only stub would keep two authoritative meanings for one epoch
transition (correct on the accumulator path, knowingly-incomplete on direct replay) — forbidden by the
single-authority / replay model and by CE-3d's own aim (match rewards, mark/set/go, pool stake, and leader inputs
from self-derived boundary state, not one favored entry path). So the direct path is corrected here, not later.

### Typed, point-bound boundary input (no precomputed mark, ever)

Replace the `precomputed_mark: Option<&StakeSnapshot>` parameter (wrong-phase API — it lets a caller freeze a
pre-RUPD mark) with an explicit, point-bound base-stake input, and make the post-RUPD phase a distinct type:

```rust
/// The canonical per-credential base-UTxO stake at a specific boundary point — the ONLY base input a mark
/// may be built from. Point-bound so the two callers cannot silently supply a mismatched-point base.
pub struct BoundaryBaseStake {
    pub boundary_point: SlotNo,                                  // the exact block the base is sampled at
    pub canonical_credential_stake: BTreeMap<StakeCredential, Coin>, // sum_base_credential_stake at that point
}

/// Reward accounts AFTER the boundary reward-update has been staged. Constructed ONLY inside the boundary
/// transition, after applyRUpd; `build_boundary_mark_snapshot` requires it, so a pre-RUPD map is unrepresentable
/// as a mark input. (Native RUPD and the one-shot bootstrap RUPD both produce this same staged view.)
pub struct PostRupdRewards<'a>(&'a crate::delegation::DelegationState); // private field; post-RUPD by construction
```

- **Accumulator caller** (`cross_epoch_boundary`): supplies `BoundaryBaseStake { boundary_point: s_prev,
  canonical_credential_stake: <reduced-checkpoint sum_base_credential_stake at s_prev> }`. It no longer builds a
  mark (`:486` deleted).
- **Direct/full-ledger caller** (`apply_epoch_boundary_full`): derives `BoundaryBaseStake` from its full
  `state.utxo_state` at the identical boundary point (fold `reduce_txout` over the UTxO → per-credential base).
- **Shared boundary fn**: stages RUPD (native or bootstrap) → `PostRupdRewards` → builds the mark from
  `BoundaryBaseStake + PostRupdRewards`. Same mark on both paths for the same inputs.

### No silent fallback (structured terminal)

The shared fn must NOT derive a reward-only mark when the base input is absent. For a **Conway** boundary,
absence is a structured terminal — `BoundaryBaseStakeRequired { boundary_point }` — returned BEFORE any
next-state construction (same zero-mutation guarantee as the governance terminal), never a fallback. (Pre-Conway
eras keep the legacy stub only where they already run it; the Conway boundary — the CE-3d / continuous-operation
path — is base-required on both callers.)

**Scope refinement (implementation, reconciled with REDUCED-VALIDATION-BOUNDARY-PLANE).** The terminal is
**FULL-authority-path only** — a Conway boundary with `track_utxo=true` (the accumulator, which always supplies
the base; and the direct full-ledger `apply_epoch_boundary_full`, which derives it from its own UTxO via
`derive_boundary_base_stake`). A **reduced follower** (`track_utxo=false`) reaches the same fn (via
`apply_block_with_accounting` during live fork-choice) but is NOT authoritative there: it keeps a
non-authoritative stub and never halts. Its proper treatment — a `ReducedBoundaryProjection` that emits **no**
mark at all — is the REDUCED-VALIDATION-BOUNDARY-PLANE P3 routing slice, sequenced AFTER this correction.
Terminaling the reduced path here would break the 10 live fork-choice tests, which this correction must leave
green ("FULL path only"). Both stub paths (pre-Conway, reduced) now read the POST-RUPD `delegation`.

### Invariants carried (unchanged by the amendment)

POOLREAP still observes post-RUPD rewards + pre-POOLREAP delegations; governance refunds still land after mark
construction; native and one-shot bootstrap RUPD both feed the one typed `PostRupdRewards`; a governance terminal
still returns before any constructed next-state; the reorder still moves no lovelace (pot conservation).

## 9. Required proof gates (all green BEFORE the BLUE slice commits)

1. **Accumulator/direct exactness** — the same `BoundaryBaseStake` + pre-boundary state through both callers
   yields byte-identical mark, set, go, reward accounts, pots, governance result, and terminal shape.
2. **Credential proof** — all 55,820 affected credentials match cardano's reward contribution exactly; the
   aggregate go residual is EXACTLY zero (re-run the CE3D-GO-STAKE-DERIVATION per-credential differential).
3. **Bootstrap-RUPD proof** — the one-shot bootstrap RUPD is consumed exactly once AND is visible to mark
   construction at the seed boundary (the staged `PostRupdRewards` includes it before the mark).
4. **No-fallback proof** — a direct Conway boundary without `BoundaryBaseStake` fails structurally
   (`BoundaryBaseStakeRequired`), and cannot construct a reward-only mark (a test asserts the terminal).
5. **Recovery proof** — warm restart + replay + reorg re-materialization reproduce the corrected mark/set/go
   byte-for-byte.
6. **Full CE-3d rerun** — the boundary differential is re-run and reaches byte-exact rewards + mark/set/go, only
   after gates 1–5 are green.
