# EPOCH-CONSENSUS-VIEW — S3f-4d-mat (scope): the live reduced-checkpoint materialization (DC-EPOCH-11)

> **Status:** SCOPED (2026-06-21, pre-code, user-directed). The actual missing live authority: make the ALREADY-BUILT reduced-checkpoint/window machinery (DC-EVIEW-04/10) live on the selected-chain admission path. NOT a new parallel stake system. Narrow. The boundary activation (derive→WAL→rebind) is S3f-4d-wire, NOT this slice.

## The sequence
```
bootstrap:   seed UTxO + manifest-bound cert state (DC-EVIEW-09)  ->  build reduced checkpoint
per block:   each admitted SELECTED-CHAIN durable block           ->  advance reduced checkpoint deterministically
boundary:    reduced checkpoint + durable ChainDB source window    ->  (S3f-4d-wire: derive view -> WAL -> rebind)
```

## Locked invariants (DC-EPOCH-11)
1. **BLUE-authoritative, not a cache.** The reduced checkpoint's content is a deterministic, replay-equivalent projection of the single ledger authority (the same blocks → byte-identical checkpoint). It is authoritative state (fail-closed if absent), NOT a best-effort cache that may be silently skipped/rebuilt. Stored in a RED durable store (redb); the VALUE is authoritative.
2. **Advances only after selected-chain durable admission.** The checkpoint advances exactly once per durably-admitted block (in lockstep with the WAL `AdmitBlock` chain) — never on a peer-delivered-but-unadmitted block.
3. **ChainDB/WAL ordering only.** The advance order is the durable selected-chain order (the WAL/ChainDB order); NO peer arrival order, scheduler, or async interleaving influences it.
4. **Reorg restores the exact lineage.** On a rollback (WAL `RollBack`), the checkpoint is restored/re-materialized to the EXACT lineage of the rollback target (re-materialize from the bootstrap base + replay the surviving `AdmitBlock`s) — never a forward-only drift.
5. **Bound to the bootstrap.** The bootstrap checkpoint is bound to the SAME seed, cert state, chain point, and manifest (DC-EVIEW-09) — a checkpoint whose binding does not match the recovered bootstrap fails closed.
6. **Missing/corrupt/lagging → fail closed.** A missing, corrupt, or lagging (behind the durable tip) reduced checkpoint BLOCKS EpochConsensusView production (S3f-4d-wire) and fails closed — never a stale/partial view.
7. **No resident full UTxO; bounded/disk-backed.** The normal live path keeps NO full UTxO resident (track_utxo=false preserved); any heavy materialization stays bounded + disk-backed (the redb checkpoint, the transient window per SLICE-1).
8. **Byte-identical until -wire.** Existing current-epoch follow/forge behaviour is BYTE-IDENTICAL until S3f-4d-wire activates a promoted view. -mat only MAINTAINS the checkpoint; it feeds nothing into leadership/admission yet.

## The first live proof after -mat: SHADOW DERIVATION (non-activating)
The fresh epoch-1335 leader/stake gate proves the imported view agrees with the node; it does NOT prove the live reduced checkpoint derives that view. So the first proof is a SHADOW derivation — observe-only, no activation:
```
live reduced checkpoint  ->  derive candidate EpochConsensusView (DC-EPOCH-09)
                          ->  compare candidate {pool_distribution, total_active_stake, ADE1 sigma}
                              against the fresh oracle bundle (ade-inputs-ep1335-fresh.json + stake-snapshot)
                          ->  REQUIRE exact agreement (or an explicitly justified, documented delta)
```
Only after the shadow derivation passes does S3f-4d-wire connect the checkpoint to the boundary activation machinery.

## Decomposition
- **-mat-1 (bootstrap build):** at bootstrap, build the reduced checkpoint from the seed UTxO BEFORE it is dropped (track_utxo=false keeps only the fingerprint today); bind it to the bootstrap (seed/cert/point/manifest). Durable in the snapshot/wal dir.
- **-mat-2 (per-block advance):** advance the checkpoint (DC-EVIEW-04 reduced_block_delta → apply_block_delta) on each durable `AdmitBlock`, in WAL order; fail-closed on a gap.
- **-mat-3 (reorg re-materialize):** on a `RollBack`, re-materialize the checkpoint to the rollback target's lineage.
- **-mat-4 (fail-closed gating):** the checkpoint exposes a verified lineage/completeness state; a missing/corrupt/lagging checkpoint blocks DC-EPOCH-09 production.
- **-mat-shadow (the proof):** the non-activating shadow derivation vs the oracle bundle.

## Where it sits
The reduced-checkpoint machinery (DC-EVIEW-04 reduce, DC-EVIEW-10 window driver, DC-EPOCH-09 derive) is built + hermetic. This slice makes it LIVE on the admission path — the missing authority so Ade produces its OWN next-epoch view, not just imports one. -wire then activates it at the boundary; the live forge proof (proof 2) follows.
