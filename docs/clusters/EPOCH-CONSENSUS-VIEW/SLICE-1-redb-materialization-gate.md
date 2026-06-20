# EPOCH-CONSENSUS-VIEW — Slice 1 (scope): redb temporary-materialization gate

> **Status:** SCOPED (2026-06-20, pre-code). The FIRST slice of EPOCH-CONSENSUS-VIEW. Narrow + path-agnostic: it proves the bounded / crash-safe transient-materialization MECHANISM, independent of any stake-attribution logic. Gated by the design record (`EPOCH-CONSENSUS-VIEW-design-analysis.md`): the architecture is selected in principle; the **mechanism stays UNAPPROVED until this gate passes**.

## Intent
Prove that Ade can materialize a TRANSIENT, disk-backed UTxO replay window (activating the dormant redb backend) with **bounded memory** and dispose of it **crash-safely** — before any of this is allowed near leader production or the live producer path. This slice contains NO stake attribution, NO leader view, NO live wiring. It is purely the `materialize → use → dispose` substrate proof.

## Scope
- Activate the dormant redb on-disk UTxO backend (`chaindb/utxo_anchor.rs`, `utxo_key.rs`) ONLY inside a bounded proof/window context (a test/bench harness) — NEVER on the live `--mode node` producer path.
- Materialize a UTxO set of N entries on disk; exercise create → populate → read/iterate (the shape an aggregation pass would use) → dispose.
- Measure the RAM working set; assert it stays below a fixed cap while the bulk lives on disk.
- Prove crash-safe disposal: a crash mid-materialization OR mid-dispose leaves NO corrupt/partial transient store, and recovery reaches a clean point with the live ChainDb + ledger checkpoint authority intact.

## Invariants (this slice only)
- **GATE-MEM** — the transient replay UTxO is disk-backed with a BOUNDED RAM working set (a fixed, measured ceiling within the BA-08 / RssAnon budget); materializing a full window never resides the whole UTxO in RAM.
- **GATE-CRASH** — `create → use → dispose` is crash-safe: a crash at any point leaves a recoverable clean state; no half-written transient store is ever mistaken for authority; the live ChainDb + checkpoint are untouched.
- **GATE-NO-FALLBACK** (the design prohibition, mechanically enforced) — the transient store is NEVER read by the live follow/forge path; the live path's authority is unchanged whether the transient store exists or not.
- **GATE-NOT-LIVE** — this slice does NOT enable `track_utxo=true` on the live producer path; materialization runs only in the bounded proof/window context.

## MAC (mechanical acceptance)
1. **Bounded materialization** — materialize N (large) entries on disk via redb; assert peak working-set < a fixed cap; assert the on-disk store holds all N.
2. **Crash-safe — mid-materialize** — inject a deterministic crash partway through materialization; on restart assert NO usable/partial transient store survives, the live ChainDb/checkpoint are intact, and recovery reaches a clean point.
3. **Crash-safe — mid-dispose** — inject a crash during disposal; on restart assert disposal cleans (no residual store) and authority is intact.
4. **No-fallback CI guard** — a gate proving no live follow/forge call site reaches the transient redb backend (the transient store is unreachable from the authority path).
5. **Not-live CI guard** — a gate proving the live `--mode node` producer path does not enable `track_utxo=true` via this slice.

## Hard prohibitions / non-goals
- NO stake attribution, NO address decoding, NO `EpochConsensusView`, NO leader-view wiring (later slices, gated on this one).
- NO `track_utxo=true` on the live producer path.
- NO permanent parallel StakeView — the store here is TRANSIENT (created per window, pruned).
- The dormant backend's mere existence is NOT proof — this slice EARNS the "live-proven" status it currently lacks.

## Entry obligations (answer before code)
1. How is a crash simulated DETERMINISTICALLY (kill point / fault injection) so GATE-CRASH is a real test, not a happy path?
2. What is the fixed RAM cap, and how is "peak working set" measured hermetically (the existing `ade_mem_diag` / RssAnon harness)?
3. Where does the transient store live on disk, and how is its lifecycle (create/dispose) keyed so a stale store from a prior crashed window is unambiguously identifiable + purgeable on restart?
