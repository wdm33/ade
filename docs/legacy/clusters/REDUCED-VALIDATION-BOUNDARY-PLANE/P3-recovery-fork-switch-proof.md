# P3 — recovery, fork-switch, and boundary-crossing proof

The reduced-plane mechanism (routing to `ReducedUnavailable`, the `ReducedCertProjection`, the persistence /
fingerprint grammar) is sealed in the first commit ("safe reduced-boundary activation"), and the authoritative
post-RUPD mark correction is the second. This slice is the **proof**: a reduced follower crosses Conway
boundaries, restarts, rolls back, and continues structural / fork validation — without ever emitting or
persisting a fake snapshot — and every RVBP acceptance gate holds.

## What the proof rests on (all green)

- **Boundary-crossing + fork-switch (gate 5).** The 47 live fork-choice tests (`live_fork_choice_ai_s4bii` —
  fork-switch, rolled-back-then-chain-selected, branch-refetch, bridge recovery, participant convergence,
  cold-start restart) cross Conway boundaries on the reduced (`track_utxo=false`) plane, which now routes to
  `apply_reduced_epoch_boundary`: **no mark, no cert/gov lifecycle**, and they stay green.
- **Recovery / replay-safety (R-RVB-1).** `reduced_boundary_crossing_is_replay_safe_and_fingerprint_distinct`:
  a reduced-crossed `epoch_state` persists and decodes **byte-identically as `ReducedUnavailable`** — a warm
  restart / WAL replay can never rehydrate it into a fabricated authoritative snapshot — and its fingerprint is
  distinct from a full authoritative state at the same point.
- **Persist / fingerprint distinctness (gates 2/3).** `reduced_snapshots_encode_distinctly_and_round_trip` and
  `rvbp_reduced_snapshot_fingerprint_never_collides_with_authoritative` (array(0) vs legacy array(3);
  `.../reduced-unavailable` component header).
- **No mark / no cert lifecycle (gate 1 / N-RVB-1..3).** `reduced_epoch_boundary_produces_no_mark_or_cert_lifecycle`.
- **Capability gates (gates 4/6, I-RVB-1/3).** `LedgerBoundaryVerdict::require_full` /
  `LedgerValidityCapability::require_full_ledger` fail closed; no `From<Reduced…>` widening.

## The 7 acceptance gates — final status

| # | gate | where |
|---|---|---|
| 1 | No mark/set/go on a reduced boundary | routing → `ReducedUnavailable` (commit 1); gate-1 test; CI (F) |
| 2 | No reduced result serialized as an accumulator snapshot | array(0) vs array(3) (commit 1); CI (E) |
| 3 | WAL/recovery fingerprints distinguish reduced from full | `reduced-unavailable` header (commit 1); replay test |
| 4 | No reduced result feeds authority | no `.mark/.set/.go` fields; `require_full*` fail closed (commit 1) |
| 5 | Reduced still crosses boundaries + fork-choice | 47 fork-choice tests |
| 6 | Full verdict after a reduced boundary → `FullBoundaryStateRequired` | P1 gate + `require_full` |
| 7 | Full paths byte-identical with the corrected post-RUPD mark | commit 2 (S1); the accumulator is untouched by the reduced routing |

Plus deviation 2: the reduced projection carries `ReducedCertProjection::Unavailable`, never a full `CertState`
(commit 1; CI (G)); the reduced boundary resets cert/gov to their empty structural absence.

## Not proven here (deferred, evidence/release gate)

The byte-exact CE-3d differential rerun — it needs a seed re-bootstrapped with S1 (the current CE-3d corpus seed
predates S1, so its `go(1341)` is pre-S1). The S1 diagnosis is empirically confirmed on the existing corpus
(`crae_advanced_base_at_post1341` residual 0; `crae_rupd_accrual_equals_residual` = the −343B). That is an
evidence gate, not a reason to hold this cluster's history.
