# MANIFEST — v3 run stores retired at the STORE_SEMANTICS v3→v4 boundary (2026-08-11)

Written per `docs/evidence/run-stores/RETENTION.md` **before** the stores were dropped. Both are
caches by that rule's own test: no `KEEP-` prefix, and the artefact (log + provenance) is harvested.

## Why now, and why these are safe to drop

`STORE_SEMANTICS_VERSION` moved 3 → 4 in `d619ebda` (BND-2a: phase-2-invalid transactions get
Cardano's collateral-only UTxO effect). Every v3 store is **mechanically refused** by the v4 binary:

```
StoreSemanticsVersionMismatch { artifact: ChainDb, found: Version(3), required: 4,
                                action: RebootstrapRequired }        exit 1
```

A v3 store can therefore never again be adopted as authoritative state by a current binary. It is a
cache in the strictest sense — and per the oracle extraction its reduced UTxO/stake content is
*known to be derived under the wrong rule* wherever the chain carried a phase-2-invalid transaction.

Disk was at **100 % (914 MB free)**, and the fresh v4 Mithril reconstruction that closes BND-2a
cannot start without headroom. That is the trigger, not the justification: the justification is that
these are superseded caches.

## Retired

| store | size | last written | venue | what it was |
|---|---|---|---|---|
| `~/.cardano-live1/ade-preprod-s7` | 12 GB | 2026-08-11 | preprod (docker `cardano-node-preprod`) | the working store for the B6 fix, the B12 census (762 ForgeTicks), the BND census, and the BND-1 live proof. Restored from `FROZEN-b6-census-s7` at the start of the B12 census. |
| `~/.cardano-live1/ade-r2-live` | 12 GB | 2026-08-04 | preprod | superseded EVIEW-R2 era run store; no `KEEP-` prefix, not an active reproducer |

## Harvested (the artefact, not the cache)

Off-repo → `~/.cardano-evidence/run-stores/<name>/`:
- `ade-preprod-s7/node.log` (1.4 MB) plus the four live session logs that were already written
  outside the store: `b12-census.log`, `bnd-census.log`, `bnd1-proof.log`, `b6-fix-validation.log`
- `ade-r2-live/node.log` (100 KB)

In-repo (KB-scale) — the findings these runs produced are already committed and are the durable
record: `b6-census-arm-live2c.txt`, `b6-root-cause-boundary-retry-thrash.txt`, `b6-fix-validated.txt`,
`b12-census-classified.txt`, `bnd-census-classified.txt`, `bnd1-typed-stall-live-proof.txt`,
`bnd2-oracle-extraction.md`, `bnd2-existing-code-authority-census.md`.

## NOT retired — deliberately

| kept | size | why |
|---|---|---|
| `~/.cardano-live1/FROZEN-b6-census-s7` | 8.2 GB, `chmod a-w` | **v3 historical evidence.** Reclassified at the v4 boundary from "controlled starting state for future validation" to "immutable evidence produced under v3". Reproduces the B6 / B12 / BND findings with a matching v3 binary and demonstrates the exact failure v4 prevents. A v4 run must NEVER adopt its authoritative reduced state. |
| `~/.cardano-live1/preprod-snapshot-6009` | 18 GB | the Mithril snapshot the fresh v4 bootstrap reconstructs FROM |
| `~/.cardano-live1/KEEP-eview-r1-reproducer` | 1.2 MB | `KEEP-` prefixed active reproducer |

## Reproducibility

Both retired runs are reproducible from (Mithril snapshot `preprod-snapshot-6009` + binary commit +
peer `cardano-node-preprod`), which is the whole basis of the retention rule. The v3 findings
additionally remain re-runnable against `FROZEN-b6-census-s7` with a **v3-era binary** — at or before
`3505c0c6`, the last commit before the v4 bump.
