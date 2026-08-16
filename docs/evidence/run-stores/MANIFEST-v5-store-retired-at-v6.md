# MANIFEST — `ade-preprod-v5` retired at the v6 bump (BND-2d)

Written **before** the drop, per `docs/evidence/run-stores/RETENTION.md`.

| | |
|---|---|
| store | `~/.cardano-live1/ade-preprod-v5` |
| size at retirement | 7.5 GB (`chain.db` 6.18 GB, `reduced-checkpoint.redb` 1.08 GB, `epoch-accumulator.redb` 208 MB, `node.log` 11 KB) |
| last written | 2026-08-16 00:35 |
| venue / network | preprod, peer `docker cardano-node-preprod` 127.0.0.1:3001 |
| binary | `390c8191` (`STORE_SEMANTICS_VERSION = 5`) |
| bootstrap | native Mithril, `~/.cardano-live1/preprod-snapshot-6009`, certified anchor slot 129,813,427 |
| final cursor | accumulator pinned at 130,350,114; durable tip ~130,550,441 |

## What it proved

The **BND-2c live bar FAILED**, and the failure was the finding: with the reduced checkpoint driven
to the durable tip at the end of every co-advance pass, the accumulator's walk asked the UTxO
authority for collateral input `0326ab20…#1` at a point the authority had already moved past, so it
truthfully answered `None` and the transition refused with
`CollateralBalance(UnresolvedCollateralInput)`. Full write-up:
`docs/evidence/run-stores/preprod-live2c/bnd2c-v5-live-FAILED-unresolved-collateral.txt`.

## Why it is retirable now

It was kept as the **reproducer** for that failure. That role is discharged twice over:

1. **The failure is reproduced in-tree.** `the_accumulator_walk_resolves_collateral_the_authority_already_spent`
   (BND-2d, `56e0a4e4`) drives the real production checkpoint advancer over the real block
   130,350,133, asserts `cp.get(…) == None` — the exact live condition — and then runs the real
   accumulator walk. Its paired control
   `without_the_retention_the_same_walk_still_refuses_and_pins` reproduces the refusal itself. A
   7.5 GB store is no longer the only way to see this.
2. **It cannot be opened any more.** `STORE_SEMANTICS_VERSION` is 6 as of BND-2d; a v5 store is
   refused up front with the typed re-bootstrap terminal. Keeping it would preserve bytes no
   current binary will read.

## Harvested before the drop

- `node.log` (106 lines, 11 KB) → `~/.cardano-evidence/run-stores/ade-preprod-v5-bnd2c-failed/node.log`
  (off-repo; the in-repo narrative already quotes the load-bearing lines verbatim).
- No `ref_*`, `*-evidence.json` or parked `*.patch` files existed in the store.

## Repo references at retirement

Three, all narrative and all still correct — they describe what the store *was*, not a path that
must resolve:

- `docs/evidence/run-stores/preprod-live2c/bnd2c-v5-live-FAILED-unresolved-collateral.txt:8`
- `docs/active/handoff-bnd2c-positioning-contract.md:71`
- `docs/active/handoff-bnd2c-positioning.md:59`

The two handoff docs describe it as "KEPT … as the reproducer". That instruction was written for the
session that had not yet fixed the defect; BND-2d closes it, and this manifest supersedes it.

## Reproduction, if ever needed

`(Mithril snapshot cert `preprod-snapshot-6009` + binary `390c8191` + the preprod peer)` reconstructs
this store exactly. The store is a cache; the log and this provenance are the artifact.

## Not retired

`FROZEN-b6-census-s7` (8.2 GB) — v3 historical evidence, needs a binary ≤ `3505c0c6`. **KEEP.**
