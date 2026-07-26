# CE-4A.3-R3 — rollback-aware eview activation resolution (the #13 blocker)

> **Status: OPEN (scoped, doc-before-impl). SEALED BLOCKER for CE-4A.3-R2 (#13).** The controlled
> rollback+refold proof (#13) surfaced a REAL recovery/replay gap: the active-authority resolver reads an
> eview activation record from a chain segment that has been rolled back. This slice fixes it read-side
> (rollback-aware resolution), reviewed + committed on its own (the CE-4A.3-R1 pattern). #13 cannot pass
> until the resolver is rollback-aware.

**Cluster:** LIVE-LEDGER-EPOCH-TRANSITION. **Blocks:** `SLICE-CE-4A-3-R2-ROLLBACK-REFOLD.md` (#13).
**Depends on:** CE-4A.3-R1 (`7266f90c`, the `RecoveredEpochNonce` epoch-binding guard — which CAUGHT this,
fail-closed), S5 (the `WalEntry::RollBack` marker, `admit_rollback`, `ResetAndRefold`).

---

## 1. The finding (verbatim root cause — from the #13 hard stop)

The #13 rollback+refold proof got past the controlled rollback (admit_rollback k-guard + `apply_chain_event`
= materialize + `commit_rollback` + `WalEntry::RollBack` all SUCCEEDED) and hard-stopped in the refold at
the startup eview recovery:

```
eview recovery: RecoveryNonceEpochMismatch { nonce_epoch: EpochNo(1341), target_epoch: EpochNo(1342) }
```

- WAL still advertises eview `target_epoch = 1342` (written by run 1's 1341->1342 cross).
- After the controlled rollback to P (slot 115942640, epoch 1341), the durable chain / chain_dep is back
  in epoch 1341, so the recovered nonce is eta0(1341).
- The startup recovery (`node_lifecycle.rs:2383-2423`) calls `resolve_activation_record`, which picks the
  MAX target epoch (1342) and **ignores `WalEntry::RollBack`** (`epoch_activation.rs:504`, `_ => continue`).
  It targets epoch 1342 while the durable tip is 1341 -> the CE-4A.3-R1 epoch-binding guard **correctly**
  fails closed.

**The bug is NOT in R1.** R1's `RecoveredEpochNonce` guard caught exactly the inconsistency it was designed
to catch: the active-authority resolver is reading an activation record from a rolled-back chain segment.
The current fail-closed behavior is correct; the fix is upstream, in the resolver.

`eview.transition_point.slot (115948834) > rollback.target.slot (115942640)` => the 1342 record is
superseded by the rollback, but `resolve_activation_record` does not honor it.

---

## 2. Intent

Make warm-start eview activation recovery **respect WAL rollback history**, so authority records from
rolled-back chain segments cannot become active after restart. Read-side (rollback-aware resolver), NOT a
write-side superseding marker — the `WalEntry::RollBack` already IS the semantic event; the resolver honors it.

---

## 3. Tier

- **true:** the recovered authority MUST represent the same selected-chain prefix as ChainDB, WAL, reduced
  checkpoint, Praos chain-dep, and EpochAccumulator.
- **derived:** Cardano-compatible restart after rollback MUST NOT validate future-epoch blocks against
  authority from a rolled-back branch.
- **release:** CE-4A.3 rollback/refold (#13) cannot pass until this resolver is rollback-aware.
- **operational:** none.

---

## 4. The required rule (position-aware, NOT "latest rollback slot"-aware)

> An eview activation record is invalid if a later `RollBack` entry rolls the selected chain back below that
> activation record's `transition_point`.

Concretely:

```
Activation(record at wal_pos=A, transition_slot=S) is superseded iff
  exists RollBack at wal_pos=R where R > A  AND  rollback_target_slot < S
```

A fresh post-rollback activation MUST still be valid:

```
RollBack to P  ->  refold  ->  new Activation(record') AFTER the RollBack
=> record' is valid if no LATER RollBack removes it
```

That is why the resolver must be **position-aware**, not merely "latest rollback slot"-aware.

### Durable-tip sanity (final selected-chain check)

Even with rollback-aware scanning, add a final check on the selected activation:

```
selected activation.transition_point.slot <= durable_tip.slot
```

If an activation points ABOVE the recovered durable tip -> terminal structured failure (a new
`EpochViewActivationError` variant) or no active activation. Do NOT silently select it. Do NOT infer
selected-chain validity from `target_epoch` alone.

---

## 5. Required implementation

- `resolve_activation_record(entries)` scans the WAL **in order**, tracks activation records with their WAL
  position; on a later `RollBack(target)` it invalidates prior activations whose `transition_point.slot >
  target.slot`; an activation becomes a candidate if not superseded by a later rollback; after the scan it
  chooses the highest valid `target_epoch` / latest valid activation as the current authority. Signature
  unchanged (`&[WalEntry] -> Result<Option<WalEntry>, EpochViewActivationError>`); rollback-awareness is
  additive (a WAL with no `RollBack` is byte-identical to today).
- A tip-checked wrapper `resolve_active_activation_at_tip(entries, durable_tip_slot)` calls the resolver and
  asserts `selected.transition_point.slot <= durable_tip_slot`, else a structured `ActivationAboveDurableTip`
  terminal. The startup recovery caller (`node_lifecycle.rs:2393`) uses it (the durable tip is already read
  there for `recovered_tip_epoch`).
- Do NOT infer selected-chain validity from `target_epoch` alone.

---

## 6. Required tests (cheap unit tests — BEFORE the long #13 rerun)

1. **activation before rollback above target is ignored:** `Activation(1342 @ 115948834)` then
   `RollBack(to 115942640)` -> resolver must NOT select 1342.
2. **activation before rollback below/equal target survives:** `Activation(1341 @ 115862416)` then
   `RollBack(to 115942640)` -> 1341 may remain selected.
3. **fresh activation after rollback survives:** `Activation(old 1342)`, `RollBack(to 1341 point)`,
   `Activation(new 1342)` -> resolver selects the new 1342 (even byte-different from the old).
4. **durable-tip sanity:** `activation.transition_point > durable_tip` -> structured failure / no active
   record (never a silent select).
5. **restart-proof regression:** the rolled-back 1342 record no longer causes `RecoveryNonceEpochMismatch`.

---

## 7. Hard prohibitions

- no deleting WAL entries;
- no manual WAL surgery;
- no synthetic superseding activation marker (write-side);
- no ignoring `RollBack` entries;
- no choosing max `target_epoch` blindly;
- no weakening `RecoveryNonceEpochMismatch` (or any R1 guard);
- no accepting an activation above the durable tip;
- no bypassing `commit_rollback` / `admit_rollback` / `ResetAndRefold` in #13.

---

## 8. Then rerun #13

After R3 is green (unit tests + workspace build):

1. Re-run rollback/refold #13.
2. Confirm `ResetAndRefold` actually executes.
3. Confirm the stale 1342 activation is ignored after the rollback.
4. Confirm the refold writes/selects the current-lineage 1342 authority.
5. Compare the rollback/refold final bundle == uninterrupted.

Commit CE-4A.3 (R3 + #13) only when #13 is green. No CE-4 final claim.

---

## 9. Invariants

- **DC-EPOCH-04 / DC-EPOCH-06** (eview activation resolution / recovery exactness) — strengthened: the
  resolver honors `WalEntry::RollBack`; recovery represents the selected-chain prefix, not a rolled-back
  branch. A new/strengthened registry entry (rollback-aware resolution + durable-tip bound) lands with R3.
- The CE-4A.3-R1 `RecoveredEpochNonce` guard is UNCHANGED (it is correct; it caught this).
