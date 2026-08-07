# C2-PREVIEW-BA02 — Phase B runbook (live forge → peer adoption)

**Date:** 2026-06-17 (~11:15 UTC). **Status:** Phase A restored + leadership
verified; **Phase B LAUNCHED** — persistent forge running toward leader slot
**115077884** (≈22:04:44 UTC, ~11h out).
**This is NOT BA02 until the Haskell peer adopts Ade's forged hash.**

## The three-slice arc that restored Phase A (all on origin/main)
- DURABLE-ADMISSION-BYTES (`3c6c30ea`): bytes-first durable-admit + fail-closed
  WarmStart read. DC-WAL-05.
- WARMSTART-ERA-SCHEDULE-VENUE (`1be6e855`): venue epoch geometry persisted in the
  sidecar (v2→v3) as durable replay authority; no hardcoded 432000. DC-CINPUT-05.
- OP-OPS-04 (`c51d7d81`): KES shell-init anchors evolution-0 at opcert_start,
  evolves to the current period. OP-OPS-04-KES-PERIOD-ANCHOR.

Phase A (all 6 checkpoints green): forge WarmStarts clean, passes operator-key
ingress, forge-capable, live feed wired (keep_alive validated). The binary
self-states NO BA-02 claim.

## leadership_reachability (VERIFIED GREEN)
- ADE1 pool id `431549bf1414e0d4a95b9fdeccbe60f66109ff8b81f502b628b2b8f3`.
- epoch 1331, ASC 1/20 = 0.05, 615 pools.
- ADE1 active_stake **1,001,512,398,903** (~1.0M ADA), total **3,083,667,184,479,782**
  (~3.08B ADA) — both NON-ZERO. sigma **0.000325**, phi **1.67e-05/slot**.
- VRF keyhash `5b7540884e5fe865…` present.
- cardano-cli `query leadership-schedule --current` (OPERATOR AID): one leader slot
  in the remaining window — **115077884 @ 2026-06-17T22:04:44Z**.
- Ade's OWN leader-check code path (`query_leader_schedule` +
  `is_leader_for_vrf_output`, PoolDistrView from the merged sidecar, Ade's VRF)
  INDEPENDENTLY found **115077884** (stake_fraction `(1001512398903, 3083667184479782)`,
  0 query errors). SEMANTIC AUTHORITY: the forge loop uses the non-zero epoch-1331 view.

## block_production (PENDING — target slot 115077884)
- Persistent forge: `ade_node --mode node` on `store6` (v3 sidecar),
  `--listen 0.0.0.0:3033`, pid in `$C2/ba02-forge.pid` (186759), nohup.
  Logs: `$C2/ba02-forge.{jsonl,out}`.
- Follows cardano-node-preview (`--peer 127.0.0.1:3002`), catches up from store6's
  tip, races to 115077884. Single-epoch limit (track_utxo=false): cannot cross epoch
  1331 end 115084799; 115077884 < 115084799 → in-window.
- At 115077884: Ade forges block **H** (slot 115077884, issuer ADE1). Capture H from
  `ba02-forge.jsonl` (the block-production line).

## peer_adoption (PENDING — the BA02 claim)
- Dedicated NON-DESTRUCTIVE topology: `cardano-node-preview/config/topology.json`
  localRoots = `[{172.17.0.1:3033, trustable}]` (172.17.0.1 = docker bridge gateway =
  host Ade from inside the container). Backup: `topology.json.ba02-bak`. SIGHUP-reloaded.
- VERIFY the node connects OUTBOUND to 172.17.0.1:3033:
  `docker logs cardano-node-preview 2>&1 | grep -iE '3033|localroot'`.
- At 115077884: the node pulls Ade's block + adopts → AddedToCurrentChain for H:
  `docker logs cardano-node-preview 2>&1 | grep -iE 'AddedToCurrentChain|115077884'`.

## BA02 correlation (the manifest — build from RAW logs, not a derived view)
BA02 lands ONLY when **block_production** (Ade forged H @ 115077884) AND
**peer_adoption** (cardano-node AddedToCurrentChain H) are both true and H matches.

## Monitoring (operational RED; no binary changes; no log-derived decisions in Ade)
```
tail -F /home/ts/.cardano-c2-preview/ba02-forge.out \
  | grep -E "ForgeTick|forged|leader|fatal|error|KesPeriod|warm-start"
docker logs -f cardano-node-preview 2>&1 \
  | grep -iE "AddedToCurrentChain|115077884|3033"
```

## Cleanup / restore (after BA02 or abort)
- Restore shared topology: `cp topology.json.ba02-bak topology.json` then
  `docker exec cardano-node-preview kill -HUP 1`.
- Stop forge: `kill -TERM $(cat $C2/ba02-forge.pid)`.
- (`$C2 = /home/ts/.cardano-c2-preview`)
