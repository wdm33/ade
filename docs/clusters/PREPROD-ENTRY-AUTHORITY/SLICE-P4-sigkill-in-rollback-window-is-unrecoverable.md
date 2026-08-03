# SLICE P4 — a SIGKILL inside the rollback window leaves an UNRECOVERABLE store

> **SEALED — OBSERVED + INSTRUMENTED, NOT DIAGNOSED.** Found 2026-08-03 05:09Z when the OS OOM-killed
> a live preview follower. **A node must be able to crash and recover.** This one cannot: the store is
> fully intact, warm-start reaches the correct tip, and recovery then fails closed permanently.

## What happened

The CE-RF-5 preview follower was OOM-killed (`rc=137`) — the operator had stacked several preprod
diagnostic runs on the same 30 GB box. On restart, warm-start recovery fails **terminally**:

```
TERMINAL recovery-admission fault (FingerprintMismatch {
    expected:  c395bad17d451d0921cbe1615c465e055230fa42d0e66daa158e4919193fdc37,   <- WAL tail post_fp
    recovered: 2cda6765856c17ebdc525392b93ceee31024b356e758bb137300fb02d4819b4f })  <- warm-start replay
                                                                              exit 42
```

Every restart reproduces it. The supervisor (which only halts on `rc=43`) restart-looped 4× against it.

## This is NOT corruption — the store is whole

| | |
|---|---|
| `chain.db` | 12.9 GB |
| `reduced-checkpoint.redb` | 2.2 GB |
| `epoch-accumulator.redb` | 307 MB |
| `wal-0000.bin` | 803 KB |

Nothing is truncated or unreadable. The failure is a **replay divergence**, not a damaged file.

## Replay geometry (from the instrumentation this slice adds)

```
warmstart-fp-mismatch: wal_tail_slot=119076425 admit_count=6953 recovered_tip=Some(119076425)
                       snapshots=9 newest_snapshot=Some(119075343) replay_span_from_newest=Some(1082)
```

Both facts matter:

- **The replay reaches the CORRECT tip** (`recovered_tip == wal_tail_slot == 119076425`). Recovery is
  not landing in the wrong place.
- **It only replays 1,082 slots** from a snapshot 9 deep. This is not long-replay drift or a missing
  checkpoint — a short, well-anchored forward replay produces a ledger that disagrees with what the
  WAL says was admitted.

## The crash landed in the S5 rollback window

The last event before the kill:

```
recovery-trace: path=rollback_admit action=reset_to_bootstrap reason=rollback_admission
                anchor_before=119075343/4535487/f7c7a8c1
                durable_tip=119076425/4535540/5dab3fd9
                rollback_target=119076399/4535539/d4007c3f
                anchor_after=absent
=== SUPERVISOR: node EXITED rc=137 ===
```

So at kill time: the rollback to 119076399 had been **admitted**, the accumulator anchor **pre-cleared**
(`anchor_after=absent`), and the ChainDb/WAL reconciliation had **not completed**. That is precisely
the window S5's pre-clear exists to make survivable — *"a crash in the window leaves an anchor-absent
(uncertified) store that the next advance refolds from canonical."*

The accumulator half behaves as designed. The **ledger/WAL** half does not: the WAL tail still sits at
119076425, above the rollback target 119076399, and the recovered ledger no longer matches it.

## Why the existing warm-start proof does not cover this

`live-ledger` CE-4A.3 **R4** proved warm-restart crash-window recovery — for a *rollback-then-restart*
scenario, byte-identically. This is a **different** scenario: **SIGKILL mid-rollback-window, before
reconciliation**. R4's result stands on its own terms and is not extended to cover this.

## Candidate directions (NONE selected — this is the open work)

1. The WAL tail is above the admitted rollback target, so `wal_tail_fp` may describe a state the
   post-rollback chain no longer has. If so the comparison itself is mis-anchored on this path.
2. The snapshot at 119075343 may hold a ledger captured mid-refold that the forward replay cannot
   reproduce. (The store had been refolding repeatedly — 5 rollbacks, all `reset_to_bootstrap`.)
3. The forward replay may apply the 1,082-slot span differently from the live admission path.

Cheapest first check: whether the WAL contains a rollback record for 119076399 at all, and what
`rollback_to_slot(wal_tail_slot)` does when the tail is above an admitted rollback target.

**Explicitly NOT acceptable as a fix:** relaxing or skipping the WAL-tail fingerprint check
(T-REC-05 / DC-NODE-22). It is the guard that proved the recovered ledger is replay-equivalent to the
admitted chain, and it is doing its job here — the bug is that recovery cannot produce a matching
state, not that it notices.

## Impact

- **A crash-killed node cannot restart.** Operator recovery today means discarding a 12.9 GB store and
  re-bootstrapping (hours). For a block producer that is an availability defect, not a nuisance.
- CE-RF-5 evidence ends here: 5 rollbacks, all `reset_to_bootstrap`, 0 re-certifications, 0 eview
  mismatches — consistent with the recorded prediction that the settled bound rarely arms.
- Preprod is unaffected and still following.

## Instrumentation landed with this slice

`warmstart-fp-mismatch` — emit-only, annotating a fault already returned. Before it the error named
two hashes and nothing about replay geometry, so "recovery landed in the wrong place" could not be
distinguished from "recovery landed correctly and replayed differently". It is the latter, and that
took one line to establish.

## Not claimed

No fix, no root cause, no invariant, no CE. This records a reproducible, fully-preserved failing
store and the geometry needed to diagnose it.
