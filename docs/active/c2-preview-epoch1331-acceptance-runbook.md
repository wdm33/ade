# C2 preview ADE1 accepted-block pass — epoch-1331 runbook (PREVIEW INSTANCE)

> **This is the PREVIEW instance** of the venue-parametric guide
> `c2-public-live-acceptance-runbook.md` (symmetric to the preprod instance
> `c2-preprod-epoch296-acceptance-runbook.md`). Per the 2026-06-14 pivot, **Preview
> is the PRIMARY public-chain proof** (1-day epochs → stake active in ~2–3 d). Same
> `--mode node` path, same BLUE invariants, **separate** evidence manifest
> (`Ba02PreviewManifest`, network_magic=2). Obeys `live-pass-path-fidelity-guide.md`
> (shared `import_live_consensus_inputs`; **never** from-genesis). Makes **no**
> acceptance claim and flips **no** rule until §3 evidence is committed.

---

## 0. Current state / grounding (verified 2026-06-17, epoch 1331)

- **Identity:** pool `ADE1` (preview) = `pool1gv25n0c5znsdf22mnl0ve0nq7essnluts86s9d3gk2u0xfhlnrh`
  (hex `431549bf1414e0d4a95b9fdeccbe60f66109ff8b81f502b628b2b8f3`), VRF keyhash
  `5b7540884e5fe865ff0588bd3dfd471978c8c0e3d1e482d7cf1ce378882c94a3`. Runtime keys
  (cold/kes/vrf/opcert) live OFF-repo at `~/Code/rust/ade-ops/preview/ade-pool/keys/`.
  (Full identity + on-chain params: `docs/evidence/preview-pool-registration.md`.)
- **STAKE IS ACTIVE (the gate is OPEN).** `query stake-snapshot` for ADE1 at epoch 1331:
  **`mark = set = go = 1,001,512,398,903`** (~1.0015M tADA; fully propagated). Leader
  election uses `set` → ADE1 `set > 0` ⇒ **electable now** (~0.0325% share ≈ 1–2
  slots/epoch). This is the lifted `blocked_until_operator_stake_available`.
- **Leader-view gate PASSED** (`ci/check_ade1_leader_stake_active.sh --network preview`):
  ADE1 `setFraction = 3.248e-04` == `bundle_sigma = 3.248e-04` (rel-err 0.00%); the
  whole-distribution check is **615/615 pools within 2%, median 0.00%**. Ade's
  leader-election view agrees with the node; a forged ADE1 block CAN be accepted.
- **Keys load AS-IS (no conversion).** `operator_forge.rs` parses the cardano-cli
  `NodeOperationalCertificate` envelope directly (`parse_opcert_envelope`; the "simple
  JSON" conversion was retired in PHASE4-N-F-G-A S2) and loads the
  `KesSigningKey_ed25519_kes_2^6` via `load_kes_skey_any_format`; cold/vrf are the
  cardano-cli text-envelope loaders. No `kes.ade.skey` / `node.opcert.simple.json` step.
- **opcert KES period is in-window.** opcert issued at KES period **885** (valid 62 →
  [885, 947]); preview `slotsPerKESPeriod = 129600`, current slot ~115,021,211 ⇒
  current KES period **887** ∈ [885, 947]. The forge evolves the KES key 885→887 before
  signing (DC-CRYPTO-10).
- **Preview peer synced.** `cardano-node-preview` (magic 2, N2N `127.0.0.1:3002`,
  config `~/.cardano-node-preview/config`) at epoch 1331 / Conway / 100%.
- **Prior recover caveat.** A 2026-06-14 preview recover ended `agreement_verdict:
  diverged` (stale-seed era; pre AO peer-attribution fixes). §2.1 below pre-seeds from
  the CURRENT preview tip and §2 requires `agreement: agreed` before any KES signature.

## Venue parameters (preview)

| param | value |
|---|---|
| `--network` / magic | `preview` / `2` |
| node container / socket | `cardano-node-preview` / `/ipc/node.socket` |
| genesis dir | `~/.cardano-node-preview/config` (off-repo) |
| consensus-inputs | `~/.cardano-c2-preview/ade-inputs.json` |
| snapshot dir / wal dir | `~/.cardano-c2-preview/store` / `~/.cardano-c2-preview/wal` |
| peer | `127.0.0.1:3002` |
| pool id env | `ADE1_POOL_HEX=431549bf…b8f3` `ADE1_POOL_BECH=pool1gv25n0c5…lnrh` |
| keys | `~/Code/rust/ade-ops/preview/ade-pool/keys/{cold,vrf,kes}.skey + node.opcert` |

---

## 1. HARD pre-launch gate (run BEFORE any live KES signature) — PASSED 2026-06-17

```
# (a) ADE1 leader-election (set) stake > 0:
docker exec cardano-node-preview sh -c 'export CARDANO_NODE_SOCKET_PATH=/ipc/node.socket; \
  cardano-cli query stake-snapshot --stake-pool-id pool1gv25n0c5znsdf22mnl0ve0nq7essnluts86s9d3gk2u0xfhlnrh --testnet-magic 2'
# (b) re-extract the bundle from the SAME node (shared path; never from-genesis):
ci/build_consensus_inputs_bundle.sh --network preview ~/.cardano-c2-preview/ade-inputs.json
# (c) stake-equality gate (Preview needs the pool id via env — no preprod default leaks):
ADE1_POOL_HEX=431549bf1414e0d4a95b9fdeccbe60f66109ff8b81f502b628b2b8f3 \
ADE1_POOL_BECH=pool1gv25n0c5znsdf22mnl0ve0nq7essnluts86s9d3gk2u0xfhlnrh \
  ci/check_ade1_leader_stake_active.sh --network preview ~/.cardano-c2-preview/ade-inputs.json
# (d) strongest (optional): cardano-cli query leadership-schedule --current with vrf.skey,
#     cross-checked against Ade's leader-check over the same bundle.
```
**Necessary, not sufficient** — acceptance is proven ONLY by the operator-captured
peer log through `correlate` (§3).

---

## 2. Launch sequence (`--mode node`, the shared path; `--network preview`)

1. **Pre-seed the store (WarmStart from the CURRENT preview tip):** dump the seed UTxO
   from `cardano-node-preview` (`query utxo --whole-utxo`) at the current tip + import
   via the N-M-C `seed_to_snapshot` path into `--wal-dir`/`--snapshot-dir`, so the
   recovered tip == the live preview tip (DC-NODE-20/22). Use FRESH dirs (the Jun-14
   `snap/chain.db` is stale/diverged).
2. **Genesis-consistency pin (OQ5, before any KES signature):** the genesis pinning
   test green for the recovered seed epoch.
3. **Recover + REQUIRE `agreed` (the go/no-go the Jun-14 run failed):** start `--mode
   node` with the peer but NO keys first (or read the relay-loop verdict) and confirm
   `agreement: agreed` against `127.0.0.1:3002` before launching the forge.
4. **Launch the forge-capable node** (complete operator key set or none — partial fails
   closed, `EXIT_NODE_FORGE_KEY_INGRESS_FAILED = 44`):
   ```
   ade_node --mode node \
     --peer 127.0.0.1:3002 --network-magic 2 --network preview \
     --json-seed <preview-utxo.json> \
     --consensus-inputs-path ~/.cardano-c2-preview/ade-inputs.json \
     --cold-skey ~/Code/rust/ade-ops/preview/ade-pool/keys/cold.skey \
     --kes-skey  ~/Code/rust/ade-ops/preview/ade-pool/keys/kes.skey \
     --vrf-skey  ~/Code/rust/ade-ops/preview/ade-pool/keys/vrf.skey \
     --opcert    ~/Code/rust/ade-ops/preview/ade-pool/keys/node.opcert \
     --genesis-file ~/.cardano-node-preview/config/shelley-genesis.json \
     --genesis-hash <preview ShelleyGenesisHash 64-hex> \
     --genesis ~/.cardano-node-preview/config \
     --wal-dir ~/.cardano-c2-preview/wal --snapshot-dir ~/.cardano-c2-preview/store \
     --listen 0.0.0.0:<port>
   ```
   - **Live feed:** `--peer` must reach `Continuing` (else no `ForgeTick`).
   - On a won slot: leader-check `Eligible` → `run_real_forge` (KES-signs the
     unsigned-header pre-image, advancing the KES period) → `pump_block` durable admit →
     serve task offers it (ChainSync RollForward + BlockFetch). ~1–2 slots/epoch ⇒ the
     producer must run sustained (a full preview epoch = 1 day).
5. **The validating peer is a FOLLOWER, not a co-producer** — `cardano-node-preview`
   carries NO ADE1 forging credentials; it only validates + adopts + serves.

---

## 3. Evidence capture (the ONLY thing that produces acceptance evidence)

1. Capture the preview peer's `ChainDB.AddBlockEvent.AddedToCurrentChain` log line
   naming the **exact forged block hash**.
2. **Decode the forged block** (`cardano-cli debug decode block` / Ade's
   `ForgedBlockArtifact`): record `issuerHash`, `blockNo`, `slot`, `hash`; **verify
   `issuerHash == blake2b-224(ADE1 preview cold vkey)`** — proves **ADE1** forged it.
3. Normalize the peer log to the closed JSONL the allow-list accepts
   (`peer_served_block` strongest / `peer_chain_tip` corroborating); self-forge
   exclusion is automatic.
4. Run the env-gated wiring (writes a manifest ONLY on a real `correlate` exact-match;
   `NoEvidence` panics) — **network magic 2**:
   ```
   ADE_LIVE_OPERATOR_TEST=1 ADE_LIVE_FORGED_BLOCK_HASH=<64-hex> ADE_LIVE_FORGED_SLOT=<N> \
   ADE_LIVE_NETWORK_MAGIC=2 ADE_LIVE_PEER_LOG=<peer.log> \
   ADE_LIVE_BA02_MANIFEST_OUT=<ade-preview-evidence.json> \
     cargo test -p ade_node --test node_operator_pass_ba02 node_operator_pass_ba02_live -- --nocapture
   ```
5. Commit the peer log + `correlate`-produced evidence + the venue-tagged
   `Ba02PreviewManifest` (sha256 of the peer log; capture command; filter; decoded
   `issuerHash`/`blockNo`/`slot`/`hash`; `live_feed_exercised = true`) under a
   **preview-tagged** evidence path — NOT a preprod path. The schema gate
   `ci_check_ba02_evidence_manifest_schema.sh` verifies it; a registry review records
   RO-LIVE-01.

---

## 4. Failure classifications

| Symptom | Class | Action |
|---|---|---|
| `agreement: diverged` on recover | recover not on the peer's chain | do NOT forge; diagnose (seed epoch vs peer tip; stale store; peer-attribution) — the Jun-14 failure mode. |
| `stakeSet = 0` | not-yet-active stake | n/a now (set ~1.0M). |
| `bundle_sigma ≠ set_fraction` | extractor stake-view mismatch | ABORT; fix extractor to source `set` (shared path). |
| `leadership-schedule` empty this epoch | ADE1 won no slots this epoch | keep the producer running into the next epoch. |
| Ade forges, node REJECTS | leader-check divergence OR block-validity bug | compare Ade `Eligible` vs `leadership-schedule`; decode the rejected block. |
| `correlate → NoEvidence` | peer did not accept | re-check gate/leadership/genesis; self-accept ≠ peer acceptance. |
| `--peer` not `Continuing` | no live feed | fix connectivity before claiming evidence. |
