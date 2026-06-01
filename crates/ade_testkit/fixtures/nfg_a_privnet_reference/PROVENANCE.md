# PHASE4-N-F-G-A S1b — Private-net genesis reference (provenance)

> **Evidence input, not runtime authority.** This fixture exists ONLY so the
> PHASE4-N-F-G-A **S1** pinning harness can prove that the WarmStart-recovered seed epoch
> matches private-genesis-derived reference values. It MUST NOT become a production source of
> `eta0`, stake, ASC, or VRF keyhash — those come only from the recovered surface via the single
> bootstrap authority (guard d / CN-NODE-01). No live-accept / BA-02 / RO-LIVE claim is made.

## What this is
An **Ade-as-leader** genesis-derived leadership reference, extracted **offline** from an isolated
single-pool private testnet where the committed pool (`pool1` / `0374a307…`) holds ~all active
stake and forges blocks. It is the genesis-consistency reference for S1's four pinning tests
(`eta0`, stake/ASC/vrf, `praos_vrf_input`+threshold, pre-seed→warm-start round-trip).

## Files (all non-secret)
- `consensus-inputs.json` — the extracted consensus-inputs bundle (`RawConsensusInputs` shape):
  `network_magic`, `genesis_hash_hex`, `era`, `epoch_no`, `epoch_start_slot`/`epoch_end_slot`,
  `active_slots_coeff`, `epoch_nonce_hex` (eta0), `pool_distribution` (Ade pool → active_stake),
  `pool_vrf_keyhashes`, `protocol_params_hash_hex`.
- `shelley-genesis.json` — the private net's Shelley genesis (public config: ASC, systemStart,
  network magic, genesis delegates as key *hashes*). Its hash is the genesis-derived `eta0`.

**No private keys are committed.** Cold/KES/VRF signing keys lived only in an out-of-repo scratch
dir (`~/.cardano-nfg-a-privnet`, untracked) and were never staged.

## Load-bearing values (epoch 0, Conway)
- `eta0` (`epoch_nonce_hex`) = `e09b4f9cf1cc17701e00ef1db04ea040bc224254ce85ae7b44415a5b333c1fea`
- **Cross-validation:** `eta0` == the Shelley genesis hash (`cardano-cli hash genesis-file`). For
  a from-genesis Shelley/Conway network the initial epoch nonce IS the genesis hash — so the
  node-queried `eta0` and the offline genesis hash agree. Ade did **not** reimplement the
  genesis→nonce rule; the value is taken from the authoritative node query and cross-checked
  against the genesis hash.
- `active_slots_coeff` = 1/20 (0.05, from the Shelley genesis).
- Ade pool id = `0374a307c05eb3771f4d1f8e1f5a4bb9fee5d5e1badfb9586e3b008d`
  (bech32 `pool1qd62xp7qt6ehw86dr78p7kjth8lwt40pht0mjkrw8vqg6ycwnnz`).
- Ade pool active leader-election fraction (raw, from `stake-distribution`) = `9000000 / 9000001`
  (≈ 1.0 — the single pool holds ~all active stake; recorded as a scaled lovelace `active_stake`
  in the bundle, the same scaling the preprod operator bundle uses).
- Ade pool VRF keyhash = `9b90020ed6ef4659a0560a086292124ca90087945831094e978106742596ca83`.
- The private net forged real blocks under these credentials (`TraceNodeIsLeader` observed),
  confirming the pool is a genuine leader for these genesis-derived inputs.

## Extraction transcript (isolated; preprod container never touched)
cardano-node / cardano-cli: **11.0.1** (`ghcr.io/intersectmbo/cardano-node:11.0.1`).

```
# 1. Generate an isolated single-pool private net (ephemeral container, host UID):
docker run --rm --user $(id -u):$(id -g) -v ~/.cardano-nfg-a-privnet:/work \
  --entrypoint cardano-cli ghcr.io/intersectmbo/cardano-node:11.0.1 \
  conway genesis create-testnet-data --testnet-magic 42 --pools 1 --stake-delegators 1 \
  --utxo-keys 1 --total-supply 30000000000000000 --delegated-supply 30000000000000000 \
  --start-time <~now UTC> --out-dir /work/testnet

# 2. Node config: minimal Cardano config, all hard-forks forced at epoch 0 (-> Conway),
#    genesis hashes via `cardano-cli hash genesis-file` / `byron genesis print-genesis-hash`.

# 3. Run an isolated private node (separate container name/db/socket; NOT cardano-node-preprod):
docker run -d --name cardano-node-nfg-a-privnet --user $(id -u):$(id -g) \
  -v ~/.cardano-nfg-a-privnet:/work --entrypoint cardano-node \
  ghcr.io/intersectmbo/cardano-node:11.0.1 run \
  --config /work/testnet/config.json --topology /work/testnet/topology.json \
  --database-path /work/testnet/db --socket-path /work/testnet/node.socket \
  --shelley-kes-key .../pool1/kes.skey --shelley-vrf-key .../pool1/vrf.skey \
  --shelley-operational-certificate .../pool1/opcert.cert

# 4. Extract via the existing operator bundle generator, env-pointed at the private node:
ADE_LIVE_PEER_CONTAINER=cardano-node-nfg-a-privnet ADE_LIVE_NETWORK_MAGIC=42 \
ADE_LIVE_PEER_SOCKET=/work/testnet/node.socket \
  ci/build_consensus_inputs_bundle.sh <out.json>
#    (genesis_hash_hex corrected from the preprod default to the private Shelley hash, = eta0)

# 5. Tear down the private node; commit only non-secret artifacts.
```

## Honesty / containment
- The preprod container `cardano-node-preprod` was never started, stopped, or queried; all work
  ran in the separate `cardano-node-nfg-a-privnet` container + an out-of-repo scratch dir.
- No new bootstrap authority; Mithril-first recovery is untouched. The fixture is reference
  evidence consumed only by the S1 test harness; it is not a runtime seed source.
