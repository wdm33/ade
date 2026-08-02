# run-store: ce3d-extract

| field | value |
|---|---|
| store size at retirement | 32G |
| last written | 2026-07-27 |
| venue | Cardano preview (magic 2) |
| referenced by | docs/clusters/LIVE-LEDGER-EPOCH-TRANSITION/SLICE-CE-4B-CONTINUOUS-MULTI-BOUNDARY.md docs/clusters/LIVE-LEDGER-EPOCH-TRANSITION/SLICE-S4-PRE-2-BOUNDARY-LEADERSHIP-FREEZE.md docs/clusters/LIVE-LEDGER-EPOCH-TRANSITION/SLICE-CE-4A-2-BOUNDARY-BYTE-EXACT.md docs/active/handover-2026-07-31.md docs/active/ce3d-s1-rebootstrap-runbook.md  |
| store contents | ade data dir: chain.db + reduced-checkpoint.redb + epoch-accumulator.redb + wal |
| reconstructible from | Mithril snapshot + peer re-follow (chain.db ~85% of size, re-fetchable) |
| harvested artifacts | ce4a-2-evidence.json ce4a-3-r2-evidence.json ce4a-3-r2-STOP-evidence.json ce4a-3-r4-evidence.json ce4a-3-r4-parked-fixes-ab-and-harness.patch ce4a-3-restart-evidence.json ce4a-3-STOP-evidence.json ce4b-evidence.json dba.sh extract_refs.out extract_refs.sh probe_cmd.sh probe.log ref_1340.log ref_1341.log ref_1342.log ref_1343.log roundtrip.out roundtrip.sh rt1.log rt2.log rt3.log slice_chunks.py  |
