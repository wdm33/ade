# run-store: ce3d-s1seed-v5

| field | value |
|---|---|
| store size at retirement | 6.7G |
| last written | 2026-07-12 |
| venue | Cardano preview (magic 2) |
| referenced by | docs/clusters/LIVE-FORGE-HARDENING/SLICE-S2-warm-start-nonce-identity.md docs/active/handover-2026-07-31.md docs/clusters/LIVE-LEDGER-EPOCH-TRANSITION/SLICE-S5-restart-rollback-replay-equivalence.md docs/clusters/LIVE-LEDGER-EPOCH-TRANSITION/SLICE-S3-CE3D-CLOSURE.md  |
| store contents | ade data dir: chain.db + reduced-checkpoint.redb + epoch-accumulator.redb + wal |
| reconstructible from | Mithril snapshot + peer re-follow (chain.db ~85% of size, re-fetchable) |
| harvested artifacts | node.log  |
