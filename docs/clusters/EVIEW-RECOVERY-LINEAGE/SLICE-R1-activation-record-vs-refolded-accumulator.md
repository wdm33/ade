# SLICE R1 — the activation WAL and the accumulator must stay lineage-consistent across a reset/refold

> **SEALED — DIAGNOSED, NOT FIXED.** Opened 2026-08-02 from a live preview rehearsal halt.
> A deterministic reproducer is preserved off-repo at
> `~/.cardano-live1/EVIEW-R1-reproducer/` (see its `README.md`). **Do not patch this while
> racing a forge window** — it is consensus-adjacent and needs a real proof.

## Problem

On an admitted rollback, `accumulator_admit_and_clear_for_rollback` calls
`EpochAccumulatorStore::reset_to_bootstrap()`, which rewinds the accumulator to the
bootstrap baseline and truncates `CURRENT_LEADERSHIP_BY_EPOCH` to the bootstrap set. The
next advance re-derives every boundary since the anchor.

The **durable WAL activation record is not reset** and survives that rewind. EVIEW
recovery then compares the surviving record against the newly re-derived candidate:

```rust
// crates/ade_node/src/epoch_activation.rs:481
(None, _)                                    => Seed
(Some(rec), Some(cand)) if matches(rec,cand) => Promoted
(Some(_), _)                                 => TERMINAL EpochViewPostPromotionMismatch
```

If they disagree the node halts — correctly, and deliberately: it will never fall back to
a wrong-epoch seed view. **The fail-closed behaviour is right. The defect is that the two
durable stores can be left disagreeing at all.**

### Observed

```
epoch-accumulator: REFOLD re-crossed boundary 1375 -> 1376 at slot 118886413
  -- re-derived after a rollback reset; durable tip is already in epoch 1377,
     98859 slots left to refold
epoch-accumulator: CROSSED boundary 1376 -> 1377 at slot 118972807
relay run-loop sync step failed (eview recovery: Activate(EpochViewPostPromotionMismatch));
  failing closed (no skip-past, no fallback).   exit 43
```

Deterministic: ~75 s from start on the preserved store.

### It is pre-existing (A/B, same store/peer/keys)

| Arm | Commit | Contains | Result |
|---|---|---|---|
| A | `3a97ca9b` | S1+S2 + refold labelling + settled-rewind buffers + skip_reason | FAILED exit 43 |
| B | `18875f49` | S1+S2 + refold labelling, **no settled-rewind buffers** | **FAILED identically** |

So ACCUMULATOR-REFOLD-BOUND S1 neither caused it nor fixes it.

**But LIVE-FORGE-HARDENING S1 is why we can see it.** Before S1 a live rollback killed the
node outright, so the refold-across-a-boundary path was never reached in a live run. The
wire fixes made the node survive long enough to reach a latent defect underneath. That is
the fixes working.

## Candidate invariant (to be stated properly when this is worked)

- **INV-ER-1.** After any accumulator reset/refold, the durable activation WAL and the
  re-derived accumulator authority are lineage-consistent: either the record is
  reproducible from the refolded accumulator, or the record is itself invalidated as part
  of the same reset — never left to disagree.

Two shapes worth weighing (do NOT pick one from the armchair):

1. **Invalidate on reset** — the rollback that resets the accumulator also retires the
   activation records it un-derives, in the same durable step. Attractive because the reset
   already truncates leadership for exactly this reason; the WAL was simply not included.
   Must not erase a record for a promotion the chain still holds.
2. **Re-derive deterministically** — guarantee the refold reproduces a byte-identical
   candidate, making the mismatch impossible. Requires the boundary mark used during a
   refold to equal the one used originally, which the reduced checkpoint's position may
   not guarantee.

Either needs a replay-equivalence proof, and (1) needs a careful argument that it can
never retire a record that is still canonical.

## Scope — what this does NOT invalidate

A **fresh Mithril bootstrap** does not take this path: a new data dir has no prior
activation record, so recovery resolves `(None, _) => Seed`. The Aug-4 forge recipe
(fresh bootstrap early in the active epoch → hold → forge **within** that epoch) never
creates a live promotion record, so it is off this path entirely.

Blocked: **warm-start after a reset/refold that crossed a boundary.**

## Claim discipline

Permitted: *fresh-bootstrap live rehearsal is unblocked; a separate
warm-start-after-reset/refold EVIEW lineage defect is preserved as a reproducer and will
be fixed in its own sealed slice.*

NOT permitted: "warm-start recovery fully certified", "all recovery windows closed",
"R4-class recovery fully solved". [[live-ledger]]'s CE-4A.3 R4 warm-restart result stands
on its own terms and is **not** extended by anything here.

## Not claimed

No fix, no invariant registered, no CE. This slice exists to seal the diagnosis and the
reproducer so the defect cannot be silently re-discovered or silently forgotten.
