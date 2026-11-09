# B3c.0 — invariants sketch (base-UTxO stake localization; GREEN/evidence)

> **PIVOT (see `B3c.0-adjudication-result.md`).** The diagnostic-METHOD invariants below all held — the
> differential is credential-sorted, exact, deterministic (I-B3c0-1/2, R-B3c0-1), with no rounding / no BLUE
> change / no compensating adjustment / no external reimport (N-B3c0-1..4). The base-UTxO CLASSIFICATION bar
> (I-B3c0-3/4) was resolved by **exoneration, not enumeration**: the credential-sorted differential found ZERO
> base-UTxO components to classify (durable checkpoint == fresh `reduce_txout`, all 254,385 credentials, diff 0),
> so the −343,260,172,883 sum-conservation target carries entirely OUTSIDE the base UTxO. The residual is REAL
> (sealed doubled adjudication) but is a go-stake **derivation** discrepancy — a later evidence slice owns that
> classification. **B3c.1 stays closed** (no base-UTxO defect to correct).

B3c.0 is a DIAGNOSTIC slice: it localizes the −343,260,172,883 lovelace CE-3d go-stake residual to a closed set
of deterministic causes at the exact POST-1341 anchor. It changes NO authoritative behaviour — it only observes
Ade's existing reduced-checkpoint result and the cardano reference. The corrective slice is B3c.1 (separate).

## What must ALWAYS be true

- **I-B3c0-1 (one authoritative stake result).** B3c.0 introduces no second stake computation. It reads Ade's
  existing `ReducedUtxoCheckpoint::sum_base_credential_stake()` (the reduced-checkpoint per-base-credential UTxO
  sum) as-is; the differential is an OBSERVER over that one result and the decoded cardano reference. (TCB: True
  — deterministic replay, one authoritative stake result.)
- **I-B3c0-2 (canonical credential-sorted differential).** The differential is keyed by `StakeCredential` in a
  `BTreeMap` (canonical order), lovelace-exact per credential; identical inputs (same reduced checkpoint at the
  same point + same reference state) yield a byte-identical differential (a replay assertion proves it).
- **I-B3c0-3 (complete, closed classification).** Every non-zero per-credential component is assigned to EXACTLY
  ONE cause from the CLOSED set — `Omitted | Duplicated | WrongCredentialExtraction | WrongCoinValue |
  UnsupportedAddressForm | StaleCheckpointPoint | Other` — never a free-form string, never unassigned.
- **I-B3c0-4 (sum conservation — the acceptance bar).** The sum of the classified components equals the total
  residual −343,260,172,883 EXACTLY. No remainder is left in `Other` without a named, reproducible sub-reason.
  The acceptance is NOT "the total is close"; it is "every component assigned to a deterministic cause."

## What must NEVER be possible

- **N-B3c0-1 (no rounding / no tolerance).** No epsilon, no "within N lovelace," no percentage tolerance. The
  differential is exact; a residual not fully assigned is a FAILURE, not a pass.
- **N-B3c0-2 (no BLUE semantic change).** B3c.0 must not alter the reduced-UTxO extraction, the stake
  aggregation, the boundary math, or any fingerprinted/authoritative output. It is GREEN/RED evidence only.
- **N-B3c0-3 (no compensating adjustment / no aggregate-only match).** The differential must not add or subtract
  any value to force a match; the classification stands at the CREDENTIAL level, never merely at the aggregate.
- **N-B3c0-4 (no external stake reimport as authority).** The cardano reference is used ONLY as the differential
  target (evidence), never fed back as a live stake source.

## What must remain identical across executions (replay)

- **R-B3c0-1.** Same reduced checkpoint (same fingerprint) advanced to the same POST-1341 point + same reference
  state (same decoder-canonical commitment) ⇒ byte-identical differential + classification + a canonical B3c.0
  report hash. Bound to the anchor identities (state commitment, reduced-checkpoint fingerprint, code commit).

## The reference-side seam (a named diagnostic assumption, to confirm in the slice)

Ade's `sum_base_credential_stake` is BASE UTxO only (rewards fold in later via `aggregate_pool_stake`). The
cardano snapshot's per-credential stake (`go.delegations` Coin) is UTxO + reward. The differential must compare
LIKE for LIKE — either subtract the cardano per-credential reward to obtain its base-UTxO stake, or add Ade's
per-credential reward to both — and the slice doc must state which, and prove the reward component is separable
(the prior decomposition put reward drift at only −35,490,238, i.e. the residual is ~99% UTxO). This seam is the
first thing B3c.0 pins down; a mis-aligned basis would masquerade as a UTxO cause.
