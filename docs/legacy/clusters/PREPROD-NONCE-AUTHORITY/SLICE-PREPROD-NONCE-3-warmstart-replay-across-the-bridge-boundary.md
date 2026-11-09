# SLICE PREPROD-NONCE-3 — warm-start replay across the seed→seed+1 BRIDGE boundary

> **CLOSED 2026-08-05 — NOT AN INDEPENDENT DEFECT.** Resolved by SLICE-P2. It was a *consequence* of
> the missing boundary snapshot, not a separate fault in the replay or the bridge. Sealed and closed
> the same day, so the record shows both what it looked like and what it actually was.

## What it looked like

The CE-N2-4 run crossed preprod 304→305 correctly, then died at the P2 capture. Restarting the store
it left behind failed before the relay loop:

```
ade_node --mode node: warm-start recovery failed in the bootstrap authority
  (Materialize(ReplayFailedAt { slot: SlotNo(130118424),
                                error: Header(VrfCert(VerificationFailed)) }));
rc=42
```

`130118424` is the first block of epoch 305 — the bridge-boundary block itself. The hypothesis recorded
at sealing was the "restart half of ECA-5": the warm-start materialize replay re-validating an
epoch-305 header without the bridge's imported MARK leadership. **That hypothesis was wrong**, and the
slice deliberately did not act on it.

## What it actually was

The store it was diagnosed from had **no boundary snapshot at all**, because the P2 capture halt fired
one block after the boundary and killed the node before one could be written. Warm-start therefore had
to materialize from a pre-boundary snapshot and *replay across* the boundary block — and that replay is
what failed.

Once SLICE-P2 let the boundary capture succeed, the store has a snapshot at the boundary, and warm-start
never replays across it. Same binary, same venue, same seed, same boundary — restarting the CLEAN
post-boundary store:

```
recovery-trace: path=recovery_admit action=forward_fold reason=forward_fold_no_reset
                anchor_before=130118358/5013814/b954f5a0 durable_tip=130300485/5022099/2aa65bb5
follow: tip slot=130300878 -- AT PEER TIP (caught up, following live)
```

A **forward fold from a real anchor**, not a reset-and-replay from an absent one. Zero occurrences of
`VrfCert`, `ReplayFailedAt`, `warm-start recovery failed`, `EpochViewPostPromotionMismatch` or
`eview-mismatch` in the whole restart. Evidence:
`docs/evidence/run-stores/preprod-nonce-1/ce-n2-5-warmstart-green.log`.

## This is what closes CE-N2-5

PREPROD-NONCE-2's CE-N2-5 ("restart after the bootstrap bridge recovers the SAME value") is **MET** by
the same run. The warm-started store's durable activation record still reads

```
EpochConsensusViewActivated target_epoch=305  nonce_commitment = 74f10bea…   == cardano-node
```

and `recover_active_view` fails closed with `EpochViewPostPromotionMismatch` on *any* divergence between
that record and the re-bound bridge view. No such terminal fired and the node resumed to tip, so the
recovery reproduced the record's committed identity — nonce included.

## The lesson worth keeping

The store a failure is diagnosed from is part of the diagnosis. This one was produced by a node that
had *already failed*, so it was missing an artifact the healthy path always writes, and the resulting
symptom pointed at a subsystem (bridge leadership in replay) that was never involved. Fixing the
forward path first — rather than diagnosing the restart from a store the forward path could not
complete — is what dissolved it, and that ordering call was made before either fix existed.

## Not claimed

No fix here, no invariant, no CE of its own. The replay path was never changed; nothing about ECA-5's
restart half is proven or disproven, because the case that appeared to exercise it does not arise on a
store the forward path completed. If a genuine bridge-boundary replay case is ever reached — a store
with no snapshot at or after the bridge boundary — this document is the record of what it looks like.
