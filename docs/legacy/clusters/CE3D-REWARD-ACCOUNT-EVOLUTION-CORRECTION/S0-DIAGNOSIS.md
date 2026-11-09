# S0 — root-cause diagnosis (GREEN / evidence; NO BLUE change)

The CE-3d go-stake residual (−343,260,172,883 on go, −363,268,230,670 on set, −355,446,908,982 on mark at
POST-1342) is a **snapshot-ordering defect**: Ade builds the epoch-boundary mark snapshot from the reward
accounts **before** the boundary reward-update (RUPD) is applied, so every delegated credential's snapshot stake
is short by exactly the reward it accrues at that boundary. This slice PROVES that by measurement-driven
elimination — it changes no authoritative behaviour. The corrective change is the separate BLUE slice.

## The elimination trail (each link a pinned `#[ignore]` test in `ce3d_boundary_differential.rs`)

The go-stake localization (CE3D-GO-STAKE-DERIVATION-LOCALIZATION) proved the residual is a per-credential
stake-VALUE difference (same credential, same delegation target) and named the reward-account contribution as the
suspect. A precondition (per the user: don't patch the fold if the inputs are wrong upstream) then eliminated
every candidate until only the ordering remained:

| # | test | question | result |
|---|---|---|---|
| 1 | `crae_raw_reward_map_post1340` | are the raw reward balances wrong before the fold? | **NO** — POST-1340 reward matches cardano (+30,792,174 net; delegated +29,435,384, ~0.00001%) |
| 2 | `crae_reward_map_post1341` | does the boundary RUPD *compute* the wrong reward? | **NO** — reward still correct at POST-1341 (delegated +29,441,734; the boundary moved it +6,350) |
| 3 | `crae_advanced_base_at_post1341` | does the checkpoint base drift as it ADVANCES through the epoch? | **NO** — advanced base == cardano UTxO at POST-1341, residual 0, 0 mismatches |
| 4 | `crae_cardano_mark_is_base_plus_reward` | is the snapshot even base+reward? | **YES** — cardano's own mark(1341) == base+reward for 59,687/59,701 creds (14 whale point-artifacts) |
| 5 | code `epoch_accumulator.rs:486` vs `:508` | WHEN does Ade read the reward for the mark? | **BEFORE the RUPD** — `build_boundary_mark_snapshot(base_utxo, &acc.cert_state.delegation)` runs before `apply_epoch_boundary_with_registrations` applies the boundary reward-update |
| 6 | `crae_rupd_accrual_equals_residual` | does the missed RUPD equal the residual? | **YES** — reward accrued to the mark's creds across the boundary = +315,961,836,959 ≈ the −363B residual (gap = within-epoch withdrawals) |

Base UTxO stays EXONERATED (B3c.0 + link 3). The raw reward evolution and the RUPD *computation* are correct
(links 1–2). The fold and the stake model are correct (link 4). The only defect is the **order**: the mark reads
the reward one RUPD too early.

## Root cause (one sentence)

`cross_epoch_boundary` builds `new_mark` from `acc.cert_state.delegation` at `epoch_accumulator.rs:486` — BEFORE
`apply_epoch_boundary_with_registrations` (`:508`) applies the boundary reward-update — whereas cardano's `SNAP`
takes the mark AFTER `applyRUpd`, so Ade's mark freezes pre-RUPD rewards and undercounts each delegated
credential's go-stake by the boundary RUPD payout.

## What the BLUE corrective slice must do (scope, NOT implemented here)

Apply the boundary reward-update to the reward accounts **before** `build_boundary_mark_snapshot` reads them
(reorder the RUPD relative to the SNAP inside `cross_epoch_boundary` / `apply_epoch_boundary_with_registrations`),
so the mark reflects post-RUPD rewards — the same order cardano uses. It is a **reorder**, never an offset or a
change to `build_boundary_mark_snapshot`'s arithmetic (the arithmetic is correct; it is fed stale inputs).

## Disjoint facts to preserve (regression assertions, not folded into the fix)

- The base UTxO is byte-exact (reducer + fresh checkpoint + advanced checkpoint). Do not touch it.
- The −500,037,651,836 reward-map residual is the CPDE governance-refund gap on UNDELEGATED accounts (closed in
  CONWAY-PROPOSAL-DEPOSIT-EXPIRY; this seed predates CPDE S1). It is disjoint from the go-stake delta set at the
  credential level and must remain a separate regression assertion.

## Report hashes (pinned)

`crae_raw_reward_map_post1340` = 5a416c29…; `crae_reward_map_post1341` = c718f7e6…; `crae_advanced_base_at_post1341`
= b5ee48e8… (residual 0); `crae_cardano_mark_is_base_plus_reward` = 97138693…; `crae_rupd_accrual_equals_residual`
= 36892823…. All `#[ignore]` local-artifact evidence; no BLUE change.
