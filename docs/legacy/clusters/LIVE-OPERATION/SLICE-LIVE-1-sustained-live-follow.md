# LIVE-1 — sustained preview live operation (bootstrap → follow → boundary → warm restart)

> **Cluster:** LIVE-OPERATION (the "leave the lab" ladder: **LIVE-1** sustained live follow →
> **LIVE-2** block-production rehearsal / leader detection → **LIVE-3** Haskell peer adopts an
> Ade-forged block → **CERT** bounty evidence package). LIVE-1 is the FOLLOW-only precursor: no
> forge, no operator stake required.

## Intent

Prove Ade sustains operation against the REAL Cardano network using the SAME authority machinery
proven internally in CE-4 (CE-4A.1/A.2/A.3, R4, CE-4B) — now fed from a live peer over the wire
instead of an in-memory corpus:

```
Mithril/bootstrap-certified start  →  live peer follow  →  hold the live tip
  →  epoch boundary crossing (if timing permits)  →  frozen-leadership promotion
  →  warm restart  →  continue following
```

with **no re-import, no CLI oracle, no seed-window fallback** — the same forbidden-paths-clean
posture CE-4B asserted, now LIVE.

**What LIVE-1 proves that is NEW.** The live-follow throughput proof (`88e64df2`, ~190 b/s, held
the live tip) predates the S4 authority flip + the CE-4 machinery, and `docs/getting-started-preview.md`
still states *"continuous operation across epoch boundaries is in active development; following and
restarting within an epoch works."* LIVE-1 exercises the CURRENT (post-S4 / CE-4 / R4) binary
against a live peer, so its distinctive claim is a **live epoch-boundary crossing** driven by the
frozen-leadership + epoch-consensus-view authority — the property the older binary could not sustain
past seed+2 (the `rc=43` EVIEW ceiling this cluster's predecessor removed). A green LIVE-1 upgrades
the getting-started boundary caveat (flag before editing — that guide is protected).

## Venue (preview first — bounty-valid, faster boundaries)

Preview epochs are 1 day (86400 slots), so a live boundary is reachable inside a sustained window;
preprod (5-day epochs) is the stronger, optional follow-up (`c2-preprod-*` runbooks). Obeys
`live-pass-path-fidelity-guide.md`: the SAME `--mode node` path, shared `import_live_consensus_inputs`,
**never** from-genesis; the durable `--data-dir` store is the sole warm-start authority.

| param | value |
|---|---|
| network / magic | preview / 2 |
| peer (Haskell oracle) | `cardano-node-preview` docker, N2N `127.0.0.1:3002` |
| Ade store (`--data-dir`) | `~/.cardano-live1/ade-preview` |
| Mithril snapshot (`--snapshot-dir`) | `~/.cardano-live1/preview-snapshot` (read-only input) |
| binary | `target/release/ade` (current: post-S4/CE-4/R4, incl. the R4c RSW fix `5e83aaaa`) |

## Procedure (the canonical three-command flow + the LIVE-1 observations)

1. **Peer up + synced.** `docker start cardano-node-preview`; `cardano-cli query tip` shows it
   caught up to the live preview tip (`syncProgress` 100).
2. **Fetch a verified snapshot.** `ade mithril snapshot fetch --network preview --output-dir <snap>`
   (mithril-client 0.13.x; certified point receipt).
3. **First run: bootstrap + follow.** `ade node run --network preview --bootstrap-mithril
   <snap>/manifest.json --snapshot-dir <snap> --data-dir <store> --peer 127.0.0.1:3002`. Observe the
   native-Mithril bootstrap receipt (certified Conway anchor), then ChainSync holding the live tip.
4. **Sustained hold.** Ade closes any backlog and tracks the peer tip for a sustained window
   (target: hold within a few slots of the peer tip; record the tip-vs-peer gap over time).
5. **Boundary crossing (opportunistic).** If a preview epoch boundary falls in the window, Ade
   crosses it with a self-derived eta0 + a promotion-certified frozen leadership for the new epoch
   (NOT a halt, NOT a re-import).
6. **Warm restart.** Stop Ade; restart with ONLY `--network preview --data-dir <store> --peer …`
   (no `--bootstrap-mithril`, no `--snapshot-dir`); it recovers from its own store and resumes
   following. Authority fingerprint after recovery == before the stop (for the same durable tip).

## Acceptance (LIVE-1 green)

- **Holds the live tip** for a sustained window (bounded lag; closes any bootstrap backlog).
- **Survives ≥1 warm restart** (durable-store recovery, resumes ChainSync, no re-bootstrap).
- **Crosses ≥1 live epoch boundary IF timing permits** (self-derived eta0 + frozen-leadership
  promotion for the new epoch; the distinctive LIVE-1 claim when a boundary lands in the window).
- **Forbidden paths clean:** no re-import mid-run, no CLI oracle, no seed-window fallback (the
  CE-4B forbidden-paths posture, LIVE).
- **Peer protocol surfaces stable:** handshake / ChainSync / BlockFetch steady; no wire desync,
  no repeated reconnect storm.
- **No panic / fail-closed halt** across the window (a legitimate fail-close on a genuinely bad
  input is admissible and must be recorded, not patched around — but the happy path must not halt).
- **Authority fingerprints recorded** (acc hash / checkpoint / frozen-leadership per epoch reached)
  as the live evidence bundle.

## NOT claimed by LIVE-1 (explicit — the ladder boundary)

- **Block production / forge** (LIVE-2) — LIVE-1 is follow-only; no operator stake, no leader forge.
- **Haskell peer adopting an Ade-forged block** (LIVE-3).
- **Bounty certification** (CERT).
- **Preprod** — preview only here; preprod is the optional stronger follow-up.
- **Byte-exact vs a POST-1343 reference** (CE-4B-S, deferred) — LIVE-1 is an operational
  sustained-run proof, not a byte-for-byte oracle diff.

## Evidence

A `Live1Manifest` (venue-self-describing, network_magic=2): the certified bootstrap anchor; the
tip-vs-peer lag samples over the window; any boundary crossing (from/to epoch, self-derived eta0,
frozen-leadership hash); the warm-restart before/after authority fingerprint; forbidden-paths-clean;
protocol-surface stability. Recorded OUTSIDE the repo under `~/.cardano-live1/` (venue transcripts
are not committed — competition-secrecy), with a short in-repo evidence summary at close.
