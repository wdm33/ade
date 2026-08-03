# SLICE P4 — a SIGKILL inside the rollback window leaves an UNRECOVERABLE store

> **SEALED — ROOT CAUSE PROVEN.** Found 2026-08-03 05:09Z when the OS OOM-killed a live preview
> follower, which then could not restart.
>
> **THE TITLE IS THE WRONG FRAMING** (retained for provenance). The crash is incidental. The store is
> internally consistent (ChainDb tip == WAL tail == 119076425) and the rollback never committed.
>
> **Root cause: MIXED-SEMANTICS EXECUTION.** The store's ledger was frozen at epoch 1375 because
> pre-P3 `detect_epoch_transition` used the mainnet formula (473 on preview, never `> 1375`), so a
> ledger epoch boundary NEVER fired for the store's entire life. P3 (`5f2636c2`) bound detection to
> the venue era schedule; a post-P3 binary replaying that pre-P3 store now correctly fires and applies
> **three** epoch boundaries the live run had skipped — rotating the stake snapshots — so the recovered
> ledger cannot equal the WAL `post_fp`. ANY restart of a pre-P3 preview store on a post-P3 binary
> fails identically; no crash needed.
>
> **The actionable defect is the missing store-semantics version gate**, not the recovery path. This
> particular store is unrecoverable by any code change and must be re-bootstrapped.

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

So at kill time the rollback to 119076399 had been **admitted** and the accumulator anchor
**pre-cleared** (`anchor_after=absent`). That is precisely the window S5's pre-clear exists to make
survivable — *"a crash in the window leaves an anchor-absent (uncertified) store that the next advance
refolds from canonical."*

**Both halves behave as designed.** The evidence below (Candidate 1) shows the kill landed BEFORE
`commit_rollback`, so no ChainDb trim and no `RollBack` WAL record were ever written, and the store is
internally consistent at 119076425. The initial reading — that the ledger/WAL half had been left
inconsistent by the rollback window — was wrong.

Worth recording for a future ordering review, though it is NOT the cause here: `commit_rollback`
performs the durable ChainDb trim at step (2) and the `WalEntry::RollBack` append only at step (4)
(`node_lifecycle.rs`, *"Append the durable rollback record — ONLY after commit"*). That ordering makes
the FAILURE case clean (commit errors ⇒ no WAL record) but is write-BEHIND for rollbacks, so a crash
strictly between them would leave the WAL describing a chain the ChainDb no longer has. Recovery
tolerates this by design — `restart.rs` deliberately ignores `RollBack` entries when computing the
WAL-tail slot because *"the load-bearing recovery floor is the durable ChainDb trim"* — but the
window is real and unproven, and this crash did not exercise it.

## Why the existing warm-start proof does not cover this

`live-ledger` CE-4A.3 **R4** proved warm-restart crash-window recovery — for a *rollback-then-restart*
scenario, byte-identically. This is a **different** scenario: **SIGKILL mid-rollback-window, before
reconciliation**. R4's result stands on its own terms and is not extended to cover this.

## Candidate directions — two DISPROVEN by direct evidence, one survives

The three candidates below were the open work. Evidence now settles the first two.

### 1. "The WAL tail is above the admitted rollback target, so the comparison is mis-anchored" — DISPROVEN

The rollback to 119076399 **never committed**. Direct evidence:

- **The WAL contains no `RollBack` record for 119076399.** Decoding all 11 `RollBack` entries (payload
  is a FLATTENED `array(10)` — `write_rollback_point` inlines slot/hash/block_no, it is not nested)
  gives 11 depth-1 `PeerRollBackward` rollbacks, the newest targeting 119075131. 119076399 appears
  only as an `AdmitBlock` slot. The decode is cross-checked: the 11 `prior_tip` slots equal EXACTLY
  the 11 breaks in the WAL `prior_fp`/`post_fp` chain, found independently.
- **The ChainDb was never trimmed.** `recovered_tip=Some(119076425)`, and
  `resolve_live_follow_start` prefers a servable ChainDb tip — so the ChainDb still holds 119076425,
  above the rollback target.
- No `rollback-followed:` line was logged, whereas the two preceding rollbacks
  (119070742, 119074635) both logged one.

So the kill landed between `admit_rollback` (accumulator pre-clear, durable — `anchor_after=absent`)
and `commit_rollback`. **ChainDb tip == WAL tail == 119076425: the store is CONSISTENT**, and
`wal_tail_fp` correctly describes the admitted chain. The rollback window is not the fault.

### 2. "The snapshot at 119075343 holds a ledger captured mid-refold" — DISPROVEN

`maybe_capture_snapshot` stores `(state.ledger, state.chain_dep)` AFTER an admitted block, so a
snapshot at slot S must equal the WAL's `post_fp` at S. Probing each durable snapshot with a
degenerate `materialize_rolled_back_state` (snapshot at target ⇒ read back, no replay) and comparing
against the WAL:

```
slot=118881828 snap_fp=992520f5 wal_fp=-        no-wal-entry   (bootstrap anchor, below first admit)
slot=119053690 snap_fp=3fe190e2 wal_fp=3fe190e2 CLEAN
slot=119057862 snap_fp=98628853 wal_fp=98628853 CLEAN
slot=119059222 snap_fp=55f2b72f wal_fp=55f2b72f CLEAN
slot=119063350 snap_fp=ef1fb120 wal_fp=ef1fb120 CLEAN
```

Every WAL-comparable anchor is byte-identical to the admitted state. There is no off-by-one and no
mid-refold capture.

### 3. "The forward replay applies the span differently from live admission" — CONFIRMED, ROOT CAUSE PROVEN

**The warm-start replay applies THREE epoch boundaries that live admission never applied.** Measured
directly, either side of the first divergent block:

```
anchor   119075343  ledger_epoch=1375  schedule_epoch=1378  snapshots=63eb3877  combined=008ae0ff  (== WAL post_fp)
diverged 119075391  ledger_epoch=1378  schedule_epoch=1378  snapshots=98d77496  combined=832af426  (WAL says 72e09750)
```

The ledger epoch jumps **1375 → 1378 on ONE block**, and `EpochStakeSnapshots` rotates with it. The
`utxo` and `pparams` components do NOT move (correct for `track_utxo=false`), so this is not a UTxO or
parameter divergence — it is an epoch-boundary application.

**Why the store's ledger was three epochs stale.** Pre-P3, `detect_epoch_transition` computed the
epoch with the hardcoded MAINNET formula, which on preview returns 473. `473 > 1375` is false, so the
ledger boundary **never fired for the entire life of the store** — the ledger sat at its seed epoch
while the epoch-accumulator (its own, correct geometry) advanced normally to 1378:

```
epoch-accumulator: CROSSED boundary 1377 -> 1378 at slot 119059222     <- accumulator advanced
ledger_epoch=1375                                                       <- ledger never did
```

P3 (`5f2636c2`) bound detection to the venue era schedule. Post-P3 the schedule correctly reports
1378, `1378 > 1375` is true, and the replay applies the boundaries the live run had skipped.

**This is MIXED-SEMANTICS EXECUTION** — a store written under pre-P3 ledger semantics replayed under
post-P3 semantics — which IDD principle 7 forbids outright. The store is NOT corrupt and the
fingerprint check is NOT mis-anchored; the recovered ledger genuinely is not replay-equivalent to the
admitted chain, because the two were produced by different rules.

**Consequences, in order of importance:**

1. **The SIGKILL is incidental.** ANY restart of a pre-P3 preview store on a post-P3 binary fails
   identically. A crash is not required to reproduce this, so the slice title is wrong twice over.
2. **The real defect is a missing store-semantics version gate.** A change to authoritative ledger
   application silently invalidated every existing durable store, and surfaced as an opaque
   `FingerprintMismatch { expected, recovered }` instead of a typed *"this store was written under
   ledger-semantics v1; this binary is v2; re-bootstrap required."* P3's commit verified mainnet
   callers were byte-identical but did not consider already-durable preview/preprod stores.
3. **Every preview store built before P3 has a ledger frozen at its seed epoch** — no stake-snapshot
   rotation, no reward updates, no treasury/reserve movement, ever. Any preview result that depended
   on ledger epoch progression is suspect. (CE-RF-5 measured refold/liveness, not ledger epoch
   evolution, so its conclusions stand; this needs a sweep of anything that did.)
4. **This store is not recoverable by any code change** and should be re-bootstrapped. There is no
   fix that reconstructs three epochs of boundary effects that were never applied.

### Original candidate-3 evidence (retained — it is what localized the fault)

What the replay must reproduce is unambiguous:

- 53 admitted blocks in (119075343, 119076425], **0 duplicate slots** (the 11 slots admitted twice
  are all outside this span), and **all 6964 admits carry `verdict=Valid`** — no invalid-tx blocks.
- The recovered fingerprint `2cda6765` **appears nowhere in the WAL fingerprint chain**. The replay
  did not stop early or land on an earlier admitted state; it produced a ledger that was **never
  admitted at any slot**.
- The divergence is **deterministic** — the original 05:11Z failure and today's reruns all produce
  `expected c395bad1 / recovered 2cda6765` byte-for-byte.

Four structural explanations were checked by reading and eliminated:

| Hypothesis | Why it fails |
|---|---|
| Live and replay use different body-apply code | Both `block_validity` (Enforce) and `block_validity_trusted_replay` (TrustDurable) funnel into `block_validity_with_eligibility`; eligibility affects only the header stake check, and the body apply `apply_block_with_verdicts` is shared |
| Live and replay build different era schedules | Both come from the same sidecar — `recovered_node_schedule` just calls `make_node_schedule(s.epoch_start_slot, s.epoch_no, s.epoch_length_slots, ..)` |
| `replay_schedule.extend_to_slot` changes epoch geometry | `locate` extrapolates `start_epoch + (slot-start_slot)/epoch_len` from the chosen era, so extended and un-extended schedules agree |
| P3 (venue era schedule) changed preview replay semantics | The span sits inside epoch 1378; pre-P3 (`473 > 1378`) and post-P3 (`1378 > 1378`) both decline to apply a boundary. The pre-fix run produced the same two hashes |

**This is therefore a replay-equivalence defect between the live-admit path and the warm-start replay
path, not a crash-window defect.** The SIGKILL only forced the restart that exposed it. The title of
this slice is retained for provenance but is now known to be the wrong framing.

Localization is in progress via an emit-only bisect (`warmstart-replay-bisect`): `materialize(S) ==
wal_post_fp(S)` is monotone over the span, so ~log2(53) materializes name the first block at which
warm-start replay stops reproducing live admission.

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

## Follow-up work this opens (NOT done here)

1. **Store-semantics version gate.** Persist a ledger-semantics version in the durable store and
   fail closed with a typed error when a binary's version differs, instead of surfacing a bare
   `FingerprintMismatch`. This is the actionable defect; it is a slice of its own.
2. **Sweep pre-P3 preview results.** Anything whose conclusion depended on ledger epoch progression
   (reward updates, stake-snapshot rotation, treasury/reserves) on a preview store built before
   `5f2636c2` was computed against a ledger frozen at its seed epoch.
3. **Rollback WAL ordering** (unrelated to this failure, found while diagnosing it): `commit_rollback`
   trims the ChainDb durably at step (2) but appends the `RollBack` record only at step (4), so the
   WAL is write-BEHIND for rollbacks. The window is real and unproven, though recovery is designed to
   tolerate it via the durable ChainDb trim.

## Not claimed

No fix and no CE. The root cause is proven, but nothing here repairs the failing store (it cannot be
repaired — three epochs of boundary effects were never applied) and no invariant is added yet. The
instrumentation is emit-only.
