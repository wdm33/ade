# MANIFEST — `ade-preprod` (pre-P6 generation), retired 2026-08-05

Retired because it predates the authority-semantics marker (P6, `e9d9db46`) and therefore **cannot be
restarted**. Not a failure of the run; a deliberate consequence of the strict no-stamp rule.

## Store at retirement

| | |
|---|---|
| Path | `~/.cardano-live1/ade-preprod` |
| Size | 6.2 GB (`chain.db` 7.0 GB sparse, `reduced-checkpoint.redb` 1.0 GB, `epoch-accumulator.redb` 103 MB, `wal/`) |
| Last written | 2026-08-03 03:17 |
| Venue | preprod, network magic 1, peer `127.0.0.1:3001` (docker `cardano-node-preprod`) |
| Semantics marker | **absent** — built before `STORE_SEMANTICS_VERSION` existed |

## Provenance (this is what makes the run reproducible)

| | |
|---|---|
| Entry | Mithril cert-grounded snapshot, NEVER from genesis (C2 rule: Conway-from-Mithril) |
| Aggregator | `https://aggregator.release-preprod.api.mithril.network/aggregator` |
| Certified point | slot 129813427 / block `4153b4f5acae17be10d66e90e9454c66ff2f69df52ec4cf4e34462d1eb86582c` |
| Immutable range | 0..6010 |
| Shelley genesis | `162d29c4e1cf6b8a84f2d692e67a3ac6bc7851bc3e6e4afe64d15778bed8bd86` (verified byte-identical to the venue's own) |
| Snapshot dir | `~/.cardano-live1/preprod-snapshot-6009` (18 GB) — **retained**, holds no Ade artifacts, reusable as the entry seed for the successor store |
| Binary lineage | built across `5f2636c2` (P3) → `6795964b`; pre-P6 throughout |

## What it proved

- **P3 fix, live**: the venue era-schedule binding took preprod from *"reaches peer tip → capture-refused
  (cert/gov/snapshots REDUCED) → exit 43"* to holding tip with **0 capture refusals and 0 phantom
  boundaries**. This store is the evidence for that.
- Reached and held peer tip (last observed `slot=130057121`, epoch 304).
- Reported **forge CAPABLE** with operator keys loaded and the live WirePump feed wired.
  Peer ACCEPT was explicitly **not** claimed — operator-gated (RO-LIVE-01/06).

## What it also proved, unintentionally

It is the second live confirmation of DC-STORE-10. Run against a post-P6 binary it fails at `open`,
before any recovery work (`p6-rejection-proof.log`):

```
ade_node --mode node: cannot open persistent ChainDb:
  StoreSemantics(StoreSemanticsVersionMismatch {
      artifact: ChainDb, found: Absent, required: 1, action: RebootstrapRequired })
```

## Not stamped — deliberately

DC-STORE-10 has no stamp path. A marker asserts *"these derived bytes were produced by the rules this
binary implements"*, and for this store that assertion cannot be made from inspection — P4 proved a
store can be structurally valid, fully decodable, and three epochs stale. See
`docs/active/preprod-store-status-2026-08-04.md`.

## Harvested here

| file | what |
|---|---|
| `node.log` | the store's own log (33,799 B) |
| `run-preprod-p3fix.log` | the run log covering the P3-fix live proof (75,790 B) |
| `p6-rejection-proof.log` | the post-P6 rejection, captured before retirement |
| `mithril-snapshot-receipt.txt` | full entry provenance |

## Referenced by

- `docs/active/preprod-store-status-2026-08-04.md`
- `docs/clusters/PREPROD-ENTRY-AUTHORITY/SLICE-P3-slot-to-epoch-hardcodes-mainnet.md`
- `docs/clusters/PREPROD-ENTRY-AUTHORITY/SLICE-P6-store-semantics-version-gate.md`

## Reproduce

Same Mithril snapshot (`preprod-snapshot-6009`, retained) + a post-P6 binary + the preprod peer. The
successor store is stamped `STORE_SEMANTICS_VERSION = 1` at creation.
