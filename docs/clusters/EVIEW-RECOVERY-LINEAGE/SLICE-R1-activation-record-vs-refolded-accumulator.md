# SLICE R1 — a durable eview activation record must stay reproducible from the recovered authority

> **SEALED — DIAGNOSED, NOT FIXED.** Opened 2026-08-02 from a live preview rehearsal halt.
> A deterministic reproducer is preserved off-repo at
> `~/.cardano-live1/KEEP-eview-r1-reproducer/` (see its `README.md`). **Do not patch this while
> racing a forge window** — it is consensus-adjacent and needs a real proof.

## RESOLVED MECHANISM 2026-08-02 (supersedes BOTH statements below)

The eview-comparison instrumentation named the differing fields on the 70-second reproducer:

```
differing = [checkpoint_commitment, stake_view_canonical_hash, view_canonical_hash]

                        RECORD                CANDIDATE
target_epoch            1377            =     1377               MATCH
transition_point        118886384/38f11866  = same                MATCH
nonce                   88c236d6        =     88c236d6           MATCH
checkpoint_commitment   cbb12da0        !=    de32979c           DIFFER
stake_view_hash         b35be7b6        !=    42681f92           DIFFER
view_hash               091b1881        !=    18892c1b           DIFFER
candidate self-hash: valid
```

Epoch, boundary point and NONCE all match — that nonce is the same eta0(1377) independently
verified byte-identical against cardano-node, so the leader-schedule inputs are sound. What
diverges is the reduced-checkpoint commitment and the stake view; `view_hash` differs only
because it covers both.

The candidate is built from the accumulator's FROZEN LEADERSHIP object for 1377, whose
`source_checkpoint_commitment` is `de32979c`; the WAL recorded `cbb12da0`. **The frozen
leadership now stored for epoch 1377 is not the one that produced the activation record.**

### The causal chain

```
refold thrash
  -> each refold re-crosses the boundary and RE-SEALS frozen leadership via
     advance_with_current_leadership, using the boundary mark captured from the reduced
     checkpoint AT THAT MOMENT
  -> if the checkpoint is not positioned identically to the original crossing, the re-sealed
     object carries a DIFFERENT source_checkpoint_commitment and stake view
  -> the WAL activation record survives, still committing the ORIGINAL identity
  -> divergence lies LATENT (a running node never compares)
  -> the next RESTART compares record vs candidate -> terminal mismatch
```

**The store becomes latent-poisoned during refold and only fails later, on restart.** That is a
recovery/durability defect, not merely a diagnostic gap.

### Both earlier statements were wrong, in opposite directions

- The **original** mechanism (below) was directionally right about the cause but wrong to
  require the reset to be concurrent with the failure.
- The **correction** (below) over-corrected to "independent of reset/refold". The ForwardFold
  trace proved something narrower: no reset occurred *at the moment of detection*. The refold
  is still the upstream cause — it just happened earlier.

This also merges what were being tracked as two defects: **the refold thrash and EVIEW-R1 are
one causal chain** — thrash is the cause, EVIEW-R1 the detector. That is why the 9-hour
thrashing run never halted: it never restarted, so it never compared.

**Fix slice: `EVIEW-R2` — ResetAndRefold re-seals frozen leadership byte-identically.**
Of the two candidate shapes recorded below, the evidence selects **deterministic
re-derivation**. Invalidating the WAL record is explicitly NOT the fix: it would paper over a
store whose leadership is genuinely divergent.

## SUPERSEDED CORRECTION — the original mechanism below was FALSIFIED

This slice was first sealed claiming the mismatch was **caused by** `reset_to_bootstrap` +
refold re-deriving the accumulator while the WAL activation record survived. **That is
wrong.** The two were merely co-present in the first reproducer.

The `recovery-trace` instrumentation (`41f511eb`) settled it on a cleaner reproducer:

```
recovery-trace: path=recovery_admit action=forward_fold reason=forward_fold_no_reset
                anchor_before=119029216/4533821/10d12a55
                durable_tip=119029216/4533821/10d12a55
                rollback_target=none anchor_after=119029216/4533821/10d12a55
eview recovery: Activate(EpochViewPostPromotionMismatch); failing closed   exit 43
```

**`ForwardFold` — no reset, no refold, no rollback**, anchor exactly equal to the durable
tip, and the mismatch fires 0.4 s later regardless. Corroborating: the run immediately
before it thrashed with refolds for **9 hours and never halted**, while this one halted in
70 s with no refold at all.

So the terminal defect is **independent of reset/refold**. What actually correlates is a
**restart of a store carrying a durable activation record**. The precise trigger is still
unidentified and is the subject of the next instrumentation step.

This also separates two defects that were being treated as one:

| | |
|---|---|
| **EVIEW-R1** (this slice) | terminal, on restart, activation record vs re-derived candidate |
| **Refold thrash** | liveness only — accumulator resets repeatedly, follow starves. Cause unknown. Never halts. |

## Original problem statement (retained — the mechanism claim is superseded above)

On an admitted rollback, `accumulator_admit_and_clear_for_rollback` calls
`EpochAccumulatorStore::reset_to_bootstrap()`, which rewinds the accumulator to the
bootstrap baseline and truncates `CURRENT_LEADERSHIP_BY_EPOCH` to the bootstrap set. The
next advance re-derives every boundary since the anchor.

The durable WAL activation record is not reset and survives that rewind. EVIEW
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

- **INV-ER-1.** A durable eview activation record is either **reproducible** from the
  authority recovered on restart, or **invalidated** in the same durable step that made it
  unreproducible — never left to disagree. (Restated 2026-08-02: the earlier wording scoped
  this to "after any accumulator reset/refold", which the ForwardFold reproducer falsified.
  The obligation holds on ANY restart carrying a record, reset or not.)

Candidate shapes — the trigger is not yet identified, so NONE of these may be chosen yet:

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

## Scope — NARROWED 2026-08-02

The earlier, stronger claim was *"a fresh Mithril bootstrap avoids EVIEW-R1"*. **That is not
supported** — the cleaner reproducer IS a fresh Mithril bootstrap. It had simply crossed
boundaries during catch-up and so carried activation records.

The supported claim is narrower:

> A store that has **not crossed an epoch boundary** has no durable eview activation record,
> so recovery resolves `(None, _) => Seed` and it cannot hit this activation-record mismatch.
> A fresh store that **does** cross boundaries creates activation records and may still
> reproduce EVIEW-R1 on restart.

Consequence for the Aug-4 forge recipe (bootstrap early in the active epoch → hold → forge
**within** that epoch): it is off this path only for as long as it does not cross a boundary,
and only if it is not restarted after crossing one. That is a real constraint on the
operating procedure, not a guarantee of immunity.

Blocked: **restart of any store carrying a durable activation record.**

## Claim discipline

Permitted: *a terminal eview activation-record mismatch is preserved as a deterministic
70-second reproducer and will be fixed in its own sealed slice; it fires on restart of a
store carrying an activation record, independent of reset/refold.*

NOT permitted (and previously claimed here in error): "a fresh Mithril bootstrap avoids
it", or that reset/refold is the cause.

NOT permitted: "warm-start recovery fully certified", "all recovery windows closed",
"R4-class recovery fully solved". [[live-ledger]]'s CE-4A.3 R4 warm-restart result stands
on its own terms and is **not** extended by anything here.

## Not claimed

No fix, no invariant registered, no CE. This slice exists to seal the diagnosis and the
reproducer so the defect cannot be silently re-discovered or silently forgotten.

**The trigger is still unidentified.** Three inference-based theories have now been
falsified by instrumentation in this investigation (tip-bookkeeping, reset/refold causation,
and the fresh-bootstrap immunity claim). The next step is emit-only instrumentation of the
eview comparison itself — the resolved record and the re-derived candidate, field by field —
and the fix will be chosen from the observed differing field, not from a theory.
