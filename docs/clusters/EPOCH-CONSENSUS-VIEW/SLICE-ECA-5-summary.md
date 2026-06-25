# SLICE ECA-5 (summary) — Live epoch-boundary crossing

> Committed, **non-sensitive** normative summary. Operational detail — exact venues, timing, commands,
> and live-capture procedures — is intentionally kept in an untracked, competition-secret working doc
> (`SLICE-ECA-5-live-boundary-crossing.md`). This summary plus the invariant registry entry
> **DC-EPOCH-15** are the committed normative record, so the slice does not live only in an untracked
> file.

## Cluster
EPOCH-CONSENSUS-VIEW (EPOCH-CONTINUITY-ACTIVATION).

## Problem
A following node promotes its self-derived next-epoch leadership view across an epoch boundary
(DC-EPOCH-14) but does **not** extend the consensus **forecast horizon** (the `EraSchedule`). Header
validation checks the forecast horizon first, so a header in epoch N+1 is rejected
`OutsideForecastRange` even though the N+1 view is promoted. A following node therefore cannot cross an
epoch boundary.

## Invariant — DC-EPOCH-15 (declared)
Forecast horizon ⟺ durable N+1 authority promotion: the `EraSchedule` forecast horizon extends past a
boundary N→N+1 **if and only if** the authority has durably promoted the N+1 view; it never
pre-extends; the extension is **derived** (never persisted) and reconstructed byte-identically on
warm-start; and no flag, clock, or peer/CLI datum influences it. (Full statement in
`docs/ade-invariant-registry.toml`.) Strengthens DC-EPOCH-14 (the atomic transition now also extends
the forecast) and DC-CINPUT-05 (the N+1 geometry is derived from durable sidecar geometry).

## Design rules (binding)
- **Derive, do not persist.** The post-boundary schedule is a pure function of: durable activation
  record + recovered promoted `EpochConsensusView` + v4 sidecar geometry + committed network profile.
  A second WAL field would create redundant authority and a new mismatch class.
- **Atomic-swap ordering.** promotion durable → rebuild the immutable schedule including the N+1
  summary → atomically replace the relay-loop-owned schedule → only then permit post-boundary
  validation/forging. No mutable shared reference may leave validation using the old horizon after the
  authority has promoted.

## Hard proof obligations (before any live crossing)
1. **EraSchedule adjacency.** Verify `EraSchedule::new`/`locate` correctly support adjacent same-era
   (Conway) consecutive-epoch summaries — `check_forecast_horizon` reads only the last summary, and
   `locate` must map both N and N+1. Do not rely on it casually; add a dedicated unit test.
2. **Warm-start in both states.** Test warm-start **before** promotion (schedule stays single-epoch;
   N+1 still fails closed) and **after** promotion (schedule reconstructed with N+1), proving the
   reconstructed forecast boundary exactly matches the live (pre-restart) one in each state.

## Implementation order
1. **Profile/magic wiring** — resolve the EVIEW network magic from the committed `--network` profile;
   wire the EVIEW activation into the relay-only (forge-OFF) path.
2. **Forecast-extension core** — own and atomically rebuild the `EraSchedule` on promotion; the
   DC-EPOCH-15 coupling, hermetic tests, and the CI gate.
3. **Live crossing** — the operator-run preview boundary acceptance (procedure in the untracked doc).

## Builds on (do not rebuild)
ECA-0a/0b (leadership-complete view with VRF), ECA-1 (automatic activation), ECA-2-pre (v4 sidecar),
ECA-2-3-4 (atomic authority + recovery, DC-EPOCH-14). The cardano-ledger snapshot-timing, inclusion,
and VRF-source questions are already researched and resolved.
