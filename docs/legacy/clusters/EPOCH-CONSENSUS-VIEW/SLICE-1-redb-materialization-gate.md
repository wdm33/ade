# EPOCH-CONSENSUS-VIEW — Slice 1: redb temporary-materialization gate

> **Status:** SCOPED + entry-obligations RESOLVED (2026-06-20, pre-code). The FIRST slice of EPOCH-CONSENSUS-VIEW. Narrow, path-agnostic. The mechanism stays UNAPPROVED until this gate passes; **no gate code is written until this doc is accepted.**

## Purpose (the invariant this slice proves)
**Prove that Ade can create, crash through, recover from, and dispose of a bounded disk-backed temporary replay store WITHOUT changing any durable or authoritative state.** This slice is NOT "make epoch snapshots" — it is the substrate-safety invariant the whole EpochConsensusView cluster depends on. No stake attribution, no leader view, no live wiring.

## TCB classification — GREEN / non-authoritative (binding)
> **Transient replay storage is GREEN execution support. It may accelerate or enable bounded materialization, but it may not survive as authority, influence BLUE outputs directly, or become a fallback source for follow, forge, recovery, or snapshot activation.**

The transient store is RED-spawned execution support governed by a GREEN (non-authoritative) contract: it never feeds an authoritative output and never outlives its window.

## Scope
- Activate the dormant redb on-disk UTxO backend (`chaindb/utxo_anchor.rs`, `utxo_key.rs`) ONLY inside a bounded proof context (test/bench) — NEVER on the live `--mode node` producer path.
- Exercise the full lifecycle: create → materialize N entries on disk → iterate (the aggregation-pass shape) → dispose.
- Prove the gates below under normal completion AND under crash-through.

## Resolved design decisions (the four NET-NEW surfaces)
**D1 — Transient location: a fixed, owned subtree; NO runtime flag.** `<existing data root>/transient-epoch-view/`, derived from the node's existing storage configuration (the `--snapshot-dir`/`--wal-dir` data root) — NEVER under WAL, snapshots, ChainDb, or any directory scanned for durable artifacts. **No `--transient-view-dir` runtime/CLI flag** (it would create configuration surface around a consensus-adjacent lifecycle and invite semantic divergence). A test-only override is permitted ONLY if it is impossible to use in production builds (a `#[cfg(test)]` path — never a runtime/semantic feature flag).

**D2 — Deterministic window key (Blake2b, not random).** `epochview-window-<hex(blake2b(network ‖ era ‖ epoch ‖ source-chain-point ‖ checkpoint-commitment))>.redb`. Deterministic (no `rand`/`uuid` dependency exists), and hashing exactly the design record's "bound-activation-only" bindings ties the store's identity to the view it forms.

**D3 — Fail-closed purge-on-startup (not best-effort).** On startup, before any materialization:
1. enumerate ONLY the owned `transient-epoch-view/` subtree;
2. verify every candidate name is a valid deterministic window key (the D2 form);
3. delete all candidates;
4. `fsync` the parent directory;
5. continue ONLY when the subtree is empty.
Any failure — deletion, directory `fsync`, or name validation — is a **structured terminal failure**; Ade does NOT continue with stale transient material lying around. A transient store is by definition not authority and not resumable, so recovery is unconditional purge, never reconcile.

**D4 — Memory cap: committed corpus + fixed CI threshold (not "calibrate later").** The slice ships a committed, reproducible corpus and a fixed threshold before merge. Two limits:
- **Hard regression ceiling (CI):** a fixed `RssAnon`-delta (peak over the cycle minus the pre-materialize baseline) for the committed corpus — codified in a CI gate. Initially conservative, but committed, not deferred.
- **Evidence metric (reported, not asserted):** peak RSS (`VmHWM`) + the redb on-disk byte count.

`RssAnon` is the primary RAM metric because it excludes the mmap-backed redb bulk — a correctly disk-backed materialization shows a small bounded anonymous delta even with all N entries on disk. **BA-08 (1.94 < 2.57 GiB) remains release evidence, NOT the test threshold.**

## Invariants (this slice only)
- **GATE-GREEN** — the transient store is GREEN/non-authoritative (the binding classification): it never survives as authority, never influences BLUE outputs, never becomes a fallback for follow/forge/recovery/snapshot-activation.
- **GATE-MEM** — bounded RAM: the `RssAnon` delta over create→materialize→iterate→dispose stays below the fixed CI ceiling on the committed corpus; the bulk lives on disk (`UtxoAnchor::len()==N`).
- **GATE-CRASH** — `create→use→dispose` is crash-safe: a SIGKILL at any point recovers to a clean state with the four-part assertion below; no half-written transient store is ever authority; the durable ChainDb + WAL + checkpoint are unchanged.
- **GATE-PURGE** — startup purge is fail-closed (D3): a stale transient root is provably empty, or a structured terminal failure is raised, before normal operation resumes.
- **GATE-NO-FALLBACK** — no live follow/forge/recovery/snapshot call site reads the transient store or the `UtxoAnchor` read methods.
- **GATE-NOT-LIVE** — this slice does NOT enable `track_utxo=true` on the live producer path.

## MAC (mechanical acceptance)
1. **Bounded materialization** — materialize N (committed corpus) entries via redb; assert `RssAnon` delta < the fixed CI ceiling; assert `UtxoAnchor::len()==N`; report peak RSS + on-disk bytes as evidence.
2. **Crash mid-materialize** — SIGKILL during materialization; on restart prove ALL of: (a) no transient store is considered authority; (b) the durable **tip, WAL digest, and checkpoint digest are unchanged**; (c) the **next normal replay produces identical verdicts**; (d) the **stale transient root is empty** before normal operation resumes.
3. **Crash mid-dispose** — SIGKILL during disposal; on restart prove the same four (a)–(d).
4. **Fail-closed purge** — a valid-named leftover is purged (the D3 sequence); an invalid/foreign name in the subtree → structured terminal failure (never silently kept, never blindly deleted).
5. **No-fallback CI guard** — a static gate proving no live follow/forge/recovery/snapshot call site reaches the transient root or `UtxoAnchor` reads (the `ci_check_utxo_fp_cache.sh` grep-gate idiom).
6. **Not-live CI guard** — a static gate proving the live `--mode node` path does not enable `track_utxo=true` via this slice.

## Resolved entry obligations (grounded; EXISTS vs NET-NEW)
1. **Deterministic crash — EXISTS:** the subprocess-SIGKILL harness (`stress_kill_harness.rs` deterministic delay table `[0,1,5,10,25,50,100,200]` ms + `chaindb_kill_target.rs`) against the same redb backend; redb's default `Immediate` durability (fsync-per-commit, checksummed dual slots, auto-repair on reopen), which Ade never weakens for the anchor. **NET-NEW:** repoint the kill-worker at a transient `UtxoAnchor` + a kill-during-dispose iteration + the four-part GATE-CRASH assertion. (No failpoints crate — the idiom is real SIGKILL + file truncation.)
2. **Memory measurement — EXISTS:** the `RssAnon`/`VmHWM` reader (`mem_measure/rss_sampler.rs`), the hermetic fold-twice measurement runner (`mem_measure/runner.rs`), and `ade_mem_diag`'s `force_allocator_collect_for_diagnostic_only` for post-dispose reclaim. **NET-NEW:** the committed corpus + the fixed `RssAnon`-delta CI ceiling (D4).
3. **Lifecycle — EXISTS:** the fp-keyed disjoint-sidecar keying precedent (`chaindb/persistent.rs`), the grep-gate + `StaticUtxoFp` fail-closed-under-`track_utxo=true` idioms, the dormancy gate `ci_check_utxo_disk_anchor.sh`. **NET-NEW:** the owned `transient-epoch-view/` subtree (D1), the deterministic key (D2), and the fail-closed purge (D3) — the repo has zero temp-store / cleanup / stale-purge pattern in production code today.

## Hard prohibitions / non-goals
- NO stake attribution, NO address decoding, NO `EpochConsensusView`, NO leader-view wiring (later slices, gated on this one).
- NO `track_utxo=true` on the live producer path; NO runtime `--transient-view-dir` flag.
- NO permanent parallel StakeView; the transient store is GREEN and pruned per window.
- The dormant backend's mere existence is NOT proof — this slice EARNS the "live-proven" status it currently lacks.

## Build shape (mostly wiring + one new lifecycle)
Reuse the SIGKILL kill-harness (repointed at a transient `UtxoAnchor`) and the hermetic RSS runner (around materialize→iterate→dispose); build the new transient lifecycle (D1 location, D2 key, D3 fail-closed purge); add the committed corpus + the D4 CI ceiling + the GATE-NO-FALLBACK / GATE-NOT-LIVE CI guards. Everything fail-closed and grep-guarded with the idioms the dormant anchor already uses.
