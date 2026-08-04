# Preprod store status — 2026-08-04 (post-P6)

> **OPERATIONAL RECORD.** The preprod follower store predates the authority-semantics marker
> introduced in P6 (`e9d9db46`). It is valid while the current process runs and **will fail closed on
> the next restart, by design.** Do not stamp it.

## State

| | |
|---|---|
| Store | data dir used by the live preprod follower |
| Semantics marker | **absent** (built before `STORE_SEMANTICS_VERSION` existed) |
| Current process | running, following live at tip |
| On restart | `StoreSemantics(StoreSemanticsVersionMismatch { artifact: ChainDb, found: Absent, required: 1, action: RebootstrapRequired })` — fails at `open`, before any recovery work |
| Required action before live forge work | re-bootstrap / re-derive under the current semantics marker |

## Why it is not stamped

DC-STORE-10 has no stamp path and no override, deliberately. A marker asserts *"these derived bytes
were produced by the rules this binary implements."* For this store that assertion cannot be made from
inspection: P4 proved a store can be structurally valid, fully decodable, and three epochs stale, and
no operator looking at it would have concluded otherwise. Writing a current marker onto an unverified
store would recreate exactly that failure mode behind an official-looking button.

The narrow future exception remains a **sealed migration proof** — read an old *marked* store, prove a
deterministic `old_semantics -> new_semantics` transform, write a new store, emit evidence. Not a stamp.

## What re-bootstrap actually costs

Less than it sounds, because the artifact split is deliberate:

- **Semantics-free, retained**: the raw block bytes in `chain.db`. These are canonical wire input — the
  same bytes under any semantics. The expensive part (network sync) does not need redoing in principle.
- **Semantics-bearing, must be rebuilt**: WAL fingerprint chain, ledger snapshots, epoch accumulator,
  reduced UTxO checkpoint, derived sidecars.

A re-derive-from-retained-blocks path is therefore possible and is recorded as out-of-scope follow-up
(P6 "Scope exclusions"). Until it exists, remediation is a normal re-bootstrap.

## Sequence before preprod is usable for live forge work

1. Stop the current follower (it cannot be restarted in place).
2. Re-bootstrap / re-derive the authority stores under a post-P6 binary, which stamps
   `STORE_SEMANTICS_VERSION = 1` at creation.
3. Confirm the node reaches tip and holds it.
4. Only then resume the forge-readiness work tracked in the LIVE-2 line.

## Preview store

The preserved P4 preview store at `~/.cardano-live1/ade-r2-live` is **evidence, not an operating
store**. It is unrecoverable by any code change (three epochs of boundary effects were never applied)
and is retained intact as the live proof for DC-STORE-10. Do not attempt to restart or repair it.
