# SLICE R2 — a `ResetAndRefold` must re-seal frozen leadership byte-identically

> **FIX SLICE for the defect diagnosed in [R1](SLICE-R1-activation-record-vs-refolded-accumulator.md).**
> R1 is diagnosis-only and stays sealed. This slice carries the fix, its mechanical proof, and
> the live re-run.
>
> **Hard line (carried from the R1 close-out, restated because it constrains the design):** this
> is NOT solved by invalidating, deleting or ignoring the WAL activation record, nor by accepting
> the candidate anyway, nor by weakening `EpochViewPostPromotionMismatch`. The terminal is
> correct. The defect is that a refold is allowed to produce a *different* object at all.

## The invariant

**INV-ER-2 — refold re-seal identity.** Re-deriving an epoch boundary crossing that the node has
already crossed MUST seal a frozen-leadership object byte-identical to the one the original
crossing sealed, for every field the durable eview activation record commits to:

| field | source |
|---|---|
| `target_leadership_epoch` | accumulator epoch arithmetic |
| `source_slot` / `source_hash` (the boundary point `s_prev`) | accumulator cursor + ChainDB lineage |
| `source_checkpoint_commitment` | **reduced checkpoint finalized AT `s_prev`** |
| per-pool `active_stake` + `vrf_keyhash` (the stake view) | **reduced-checkpoint mark AT `s_prev`** |

Equivalently, and this is the shape that is actually enforceable: **the boundary mark and the
boundary commitment must be captured with the reduced checkpoint positioned EXACTLY at `s_prev`
— never at whatever slot it happens to be sitting on.**

Corollary (INV-ER-1, from R1): a durable eview activation record must stay reproducible from the
authority recovered on restart. INV-ER-2 is the mechanism that discharges it for the refold path.

## Mechanism — established from code, not inferred

The R1 instrumentation named the differing fields on the 70-second reproducer:

```
differing = [checkpoint_commitment, stake_view_canonical_hash, view_canonical_hash]

                        RECORD              CANDIDATE
target_epoch            1377          =     1377              MATCH
transition_point        118886384/38f11866 = same             MATCH
nonce                   88c236d6      =     88c236d6          MATCH
checkpoint_commitment   cbb12da0      !=    de32979c          DIFFER
stake_view_hash         b35be7b6      !=    42681f92          DIFFER
view_hash               091b1881      !=    18892c1b          DIFFER
```

That split is a **signature, not a coincidence**:

- every field derived from the **accumulator** matched (epoch, boundary point, eta0 nonce);
- every field derived from the **reduced checkpoint** differed.

So the accumulator refolds correctly and the checkpoint does not follow it. The code says exactly
why. `advance_reduced_checkpoint_over_chaindb`
(`crates/ade_runtime/src/chaindb/reduced_window_driver.rs:204`) is purely forward:

```rust
let from = checkpoint.last_advanced_slot()?.map(|s| s + 1).unwrap_or(bootstrap_slot);
for stored in chaindb.iter_from_slot(from)? {
    if stored.slot.0 > to_slot.0 { break; }   // cursor already past to_slot -> breaks immediately
    ...
}
Ok(())                                        // <-- SILENT SUCCESS, checkpoint left where it was
```

When the cursor is already past `to_slot` this returns `Ok(())` **having done nothing and having
signalled nothing**. The caller cannot tell "positioned at the target" from "left 140,000 slots
past the target".

And `advance_ledger_state_to_durable_tip` (`crates/ade_node/src/node_lifecycle.rs`) drives the
checkpoint to the durable tip at the end of **every** pass (line 2761), while the accumulator's
reset on an admitted rollback / `ResetAndRefold` does **not** rewind the checkpoint —
`reduced_checkpoint_reset_if_ahead` only fires when the checkpoint is ahead of the **tip**, which
after a same-chain refold it is not.

The resulting sequence:

```
pass N      : checkpoint driven to tip            -> cursor = 119029216 (epoch 1377)
rollback /  : accumulator reset_to_bootstrap()    -> checkpoint NOT rewound
ResetAndRefold
pass N+1    : accumulator refolds from bootstrap, stalls at the 1375->1376 boundary,
              s_prev = 118886384
              advance_reduced_checkpoint_forward_to(cp, .., 118886384)  -> SILENT NO-OP
              cp.sum_base_credential_stake()  -> stake at 119029216   WRONG MARK
              cp.finalize()                   -> commitment at 119029216  WRONG COMMITMENT
              cross_accumulator_over_boundary_block(.., &mark, s_prev, .., &commitment)
                 -> re-seals frozen leadership for epoch 1377 with BOTH wrong
WAL         : activation record for 1377 survives with the ORIGINAL identity
              -> divergence is LATENT; a running node never compares
restart     : recover_active_view compares record vs candidate -> TERMINAL
```

**The store is latent-poisoned during the refold and only fails later, on restart.** That is a
recovery/durability defect. It is also why the 9-hour thrashing run never halted — it never
restarted, so it never compared.

Note the asymmetry that made this survivable for so long: the checkpoint's own readiness gate
`verify_ready_at` **already** fails closed on `Ahead` (`reduced_utxo_checkpoint.rs:367`). The
derive path refuses a checkpoint that sits past the required slot. The **seal** path never asked.

### Why deterministic re-derivation is sufficient (and invalidation is not needed)

The re-derived state at `s_prev` is `seed -> s_prev` over the canonical ChainDB. The original
crossing folded exactly those same blocks in the same order. So the two agree **provided the
canonical blocks in `(seed, s_prev]` are the same** — and they are: `s_prev` belongs to a boundary
the refold is re-crossing, hence already more than `k` blocks deep, and rollback admission
(`admit_rollback` / `settled_rewind_admissible`) refuses anything deeper than `k`. Within
admissible rollbacks that prefix is immutable.

That is the argument for R1's candidate shape **(2) re-derive deterministically** over shape
**(1) invalidate on reset**. Shape (1) would additionally need a proof that it can never retire a
record for a promotion the chain still holds; shape (2) needs no such proof because it never makes
the record unreproducible in the first place.

## The change

One production seal site exists: `node_lifecycle.rs:2610` (the `StalledAt` boundary arm). The
`epoch_candidate.rs:363` occurrence is inside a test.

1. **Make "positioned exactly at" a real operation, not a hope.** Replace the bare forward advance
   at the boundary stall with a positioning step that:
   - rewinds (`cp.reset_to_bootstrap()`, the only way back — the reduced delta is not invertible)
     when the cursor is **ahead** of `s_prev`;
   - forward-advances to `s_prev`;
   - **verifies** the cursor is now exactly `s_prev` (`verify_ready_at`) and fails closed otherwise.

   Mark and commitment are captured only after that verification succeeds. A positioning failure
   takes the existing observe-only stall arm — the accumulator does not cross, the next pass
   retries. Stalling is strictly safer than sealing a wrong object.

2. **Make the silent no-op unrepresentable at the seam** so this cannot regress into a different
   caller: the positioning helper reports what it did as a closed sum (`AlreadyAt` /
   `AdvancedForward` / `RewoundAndReplayed`) rather than `Ok(())`.

Cost: one checkpoint re-materialisation per refold (not per boundary — after the first rewind the
remaining boundaries in the same pass are forward). It roughly doubles refold work, which is the
correct trade and is separately bounded by ACCUMULATOR-REFOLD-BOUND S1.

TCB: the decision is deterministic glue over a RED store; no BLUE authority logic changes and no
new BLUE surface. Rollback admission, retry/backoff, anchor lifecycle and refold scheduling are
untouched.

## Mechanical acceptance criteria

- **CE-R2-1** — positioning is exact. A checkpoint advanced past a target and then positioned at
  that target lands with `last_advanced_slot() == target`, and its `finalize()` + mark are
  **byte-identical** to the values it had when it first passed through that slot. Direct proof of
  INV-ER-2's operative clause.
- **CE-R2-2** — the old behaviour is pinned as a negative. A test that fails if the forward
  advance is ever again allowed to silently leave the cursor past the requested slot.
- **CE-R2-3** — refold re-seal identity end-to-end at the production seam: cross a boundary via
  `advance_ledger_state_to_durable_tip`, capture the sealed frozen-leadership object, force a
  `reset_to_bootstrap` + refold with the checkpoint left at the tip, and assert the re-sealed
  object is byte-identical — `source_slot`, `source_hash`, `source_checkpoint_commitment` and the
  full pool map.
- **CE-R2-4** — the preserved 70-second reproducer (`~/.cardano-live1/ade-fresh-1377`) no longer
  exits 43. **Read this one carefully:** that store is *already* poisoned, so the fix cannot
  un-poison it by re-derivation alone. CE-R2-4 is satisfied by a store that crosses a boundary,
  refolds, and restarts **under the fixed binary** — not by the stale one. Recorded as such.
- **CE-R2-5** — live: preview follow across a boundary with a restart, no
  `EpochViewPostPromotionMismatch`, eta0 still byte-matching cardano-node.

Registry: `DC-EPOCH-32` (positioning is exact-or-fail-closed), `DC-EPOCH-33` (refold re-seal
identity). Registered as `enforced` only once CE-R2-1..3 exist as tests **and** the CI gate lands;
the live run is supporting evidence, never the reason.

## RESULT 2026-08-02 — CE-R2-1..4 MET

Fix landed in `1dc48e74`. CE-R2-1/2/3 are the four tests plus
`ci/ci_check_eview_refold_reseal.sh`; CE-R2-3 was verified to DISCRIMINATE (reverting the seal path
makes it fail on exactly the field the live node halted on), and the gate was mutated three ways
(bare forward advance / dropped verification / seal-anyway-when-unreachable) and caught each.

### CE-R2-4 — live, on a fresh store, under the fixed binary

Store `~/.cardano-live1/ade-r2-live`, preview, same Mithril 1376 snapshot / peer / keys as the
poisoned one. Catch-up crossed 1375→1376 (`s_prev` 118886384) and 1376→1377, writing activation
records. Then a genuine peer rollback produced the exact poisoning condition:

```
18:38:29.823Z rollback_admit  action=reset_to_settled   reason=rollback_admission
                              rollback_target=119039873/4534207/e98682f7  anchor_after=absent
18:39:50.958Z recovery_admit  action=reset_and_refold   reason=anchor_absent
                              durable_tip=119039978/4534210/ddce5472
18:40:59.578Z reduced checkpoint REWOUND onto boundary point 118886384 before sealing
                              (it sat past it -- DC-EPOCH-32)
18:41:01.654Z REFOLD re-crossed boundary 1375 -> 1376 at slot 118886413 (mark from s_prev 118886384)
                              -- durable tip is already in epoch 1377, 153565 slots left to refold
```

The refold re-crossed the boundary with the checkpoint **153,565 slots ahead of it, in a later
epoch** — pre-fix, exactly the moment epoch 1377's leadership was re-sealed wrong. A second refold
at 19:18:23 repeated it. Then the node was **SIGKILLed mid-refold** and restarted:

```
19:24:35.988Z recovery_admit  action=forward_fold  anchor_before=118908160/4529489/97183a57
                                                   durable_tip=119042209/4534276/bbef2f74
   ... ran past 200s with NO EpochViewPostPromotionMismatch and no halt
```

**Positive control — the pass is not vacuous.** `maybe_recover_promoted_authority` returns `Ok`
silently when no record exists (`None => Seed`), so a clean restart proves nothing unless a record
is present. This store's WAL contains an activation record binding **every** field the poisoned
store's record bound, and **none** of the corrupted ones:

| field | poisoned store's RECORD | fresh WAL | corrupted value | fresh WAL |
|---|---|---|---|---|
| checkpoint_commitment | `cbb12da0` | present | `de32979c` | **absent** |
| nonce | `88c236d6` | present | — | — |
| stake_view_canonical_hash | `b35be7b6` | present | `42681f92` | **absent** |
| view_canonical_hash | `091b1881` | present | `18892c1b` | **absent** |

So recovery took `(Some(rec), Some(cand)) if matches => Promoted`. This also **independently
confirms which side was correct**: a completely separate bootstrap reproduced `cbb12da0` /
`b35be7b6` / `091b1881` byte for byte, so those were the true values and `de32979c` / `42681f92` /
`18892c1b` were the corruption.

**Negative control.** The preserved poisoned store, run under the SAME fixed binary at 17:50Z,
**still halts identically** (exit 43, same field diff). The fix stops a store *becoming* poisoned;
it does not repair one already divergent, and the terminal was not weakened.

**Incidental confirmation of the cost claim.** No second `REWOUND` fired for the 1376→1377
re-crossing in the same pass: after the first rewind the checkpoint sits *behind* the next boundary
point and simply advances forward onto it. One re-materialisation per refold, not per boundary —
observed, not merely argued.

CE-R2-5 (a live boundary crossing plus restart) remains open; the node is still following.

## Not claimed

- No claim that all refold defects are fixed. The refold **thrash** (accumulator resetting
  repeatedly, follow starving) is the same causal chain's trigger, and its own root cause is still
  open. This slice makes the refold *harmless when it happens*; it does not stop it happening.
- No claim about warm-start recovery beyond this path. `live-ledger` CE-4A.3 R4 stands on its own
  terms and is not extended here.
- No BA-08 memory claim, no RO-LIVE flip, no block-production claim.
