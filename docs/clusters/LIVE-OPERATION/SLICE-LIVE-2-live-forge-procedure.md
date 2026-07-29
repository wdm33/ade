# LIVE-2 — live block-production procedure (leader detection + forge), and machinery verification

> **Cluster:** LIVE-OPERATION. Follows LIVE-1 (sustained follow, proven) + LIVE-1b (bounded retention). This
> slice: (1) VERIFY the forge machinery on the current post-S4/CE-4/R4/LIVE-1b binary; (2) document the
> live-forge procedure. **LIVE-2's live forge is GATED on ADE1 preview stake** (currently ~1497 ADA, σ≈4.6e-7
> → ~0 leader slots; needs a faucet re-delegation to `pool1gv25n0c5znsdf22mnl0ve0nq7essnluts86s9d3gk2u0xfhlnrh`
> + ~2-epoch activation). The peer adoption of an Ade-forged block is LIVE-3.

## 1. Forge machinery — VERIFIED (2026-07-28, current binary)

Hermetic forge tests all green on the current binary (S4/CE-4/R4/LIVE-1b did NOT regress the forge path):

- `produce_loopback` (4/4): forge -> serve -> BlockFetch roundtrip; served-snapshot two-run replay
  byte-identical (block ASSEMBLY + serve/fetch/validate).
- `forge_handler_variants` (4/4); `node_operator_pass_ba02` (3/3, the `correlate` evidence path).
- lib: `forge_intent` (all 32 operator-key-flag combinations -> forge ON / OFF / fail-closed), KES key-gen +
  load, `node_forge_protocol_version_and_pparams_from_recovered_current_view`, `extend_own_spine_forges_on_durable_tip`,
  `node_spine_cold_start_ineligible_feed_does_not_forge`, `ba02_self_accept_is_not_evidence`.

Live forge-capable INGRESS verified: `ade node run --mode node ... --cold-skey/--kes-skey/--vrf-skey/--opcert
--genesis-file` with the real ADE1 preview keys -> the current binary loads the cardano-cli KES key (VK
fingerprint 8a6cdd3e…), classifies **forge ON**, warm-starts, and HOLDS the live tip forge-capable (waiting
for leader slots). The only missing input is stake.

Underlying already-proven (pre-LIVE): CE-A5 (N-AE) — a real cardano-node 11.0.1 relay `AddedToCurrentChain`
an Ade-forged block on C2-LOCAL; N-O/N-P Ade-native Sum6KES == Haskell; N-W producer Praos VRF; N-AC KES
forward-evolution before signing.

## 2. Prerequisites (before a live forge)

1. **Active leader stake on preview** — the hard gate. Faucet "delegate to a stake pool" ->
   `pool1gv25n0c5znsdf22mnl0ve0nq7essnluts86s9d3gk2u0xfhlnrh`; wait ~2 epochs; verify
   `cardano-cli query stake-snapshot --stake-pool-id <pool> --testnet-magic 2` shows `stakeSet` >> 0
   (≈1e12 lovelace / ~1M ADA gives σ≈3e-4 -> ~1.4 leader slots/epoch, as in June).
2. **Operator keys current — VERIFIED VALID (2026-07-28).** `~/Code/rust/ade-ops/preview/ade-pool/keys/{cold.skey,kes.skey,vrf.skey,node.opcert}`.
   `cardano-cli query kes-period-info` (authoritative): opcert start period **885**, current period **915**,
   end interval **947** (genesis `maxKESEvolutions` 62 → valid window [885, 946]), **"✓ within the correct KES
   period interval"**, **expires 2026-09-14T12:00Z** (~32 evolutions / ~47 days of headroom). The opcert KES VK
   is `8a6cdd3e…`, matching the key Ade prints at load. Counter 0 (pool never minted; on-chain counter `null`)
   → the first forge issues under counter 0. The ~2-epoch stake activation lands ~period 916, deep inside the
   window — **no re-issue needed before the forge**. (Preview KES period = 129600 slots ≈ 1.5 d; re-issue only
   if the forge is deferred past ~2026-09-14. Runtime note: Ade forward-evolves the KES key in-memory (N-AC)
   from evolution 0 → period 915 = 30 evolutions before signing; well within the 62-evolution lifetime.)
3. **A recovered Conway tip** — mithril bootstrap (LIVE-1 flow) OR warm-start an existing `--data-dir`; never
   from genesis (live-path-fidelity).

## 3. The live-forge command (SAME --mode node path as LIVE-1, keys added)

```sh
ade node run --network preview \
  --data-dir ~/.cardano-live1/ade-preview --peer 127.0.0.1:3002 \
  --cold-skey  <keys>/cold.skey \
  --kes-skey   <keys>/kes.skey \
  --vrf-skey   <keys>/vrf.skey \
  --opcert     <keys>/node.opcert \
  --genesis-file ~/.cardano-node-preview/config/shelley-genesis.json
```
(First run adds `--bootstrap-mithril <manifest> --snapshot-dir <snap>`.) Ade then: recovers the tip ->
follows -> at each slot runs its OWN leader check (VRF over eta0 + ADE1 σ + ASC) -> at a leader slot forges
tip+1 (Praos VRF proof + KES-signed header via the forward-evolved KES key + op-cert), self-accepts it
through the SAME `pump_block` durable-admit chokepoint, and serves it.

## 4. Acceptance (LIVE-2, then LIVE-3)

- **LIVE-2 (leader detection + forge):** Ade's leader-schedule for the epoch matches
  `cardano-cli query leadership-schedule --stake-pool-id <pool> --current`; at a scheduled slot
  `forge_result: succeeded=1`; the block is durably admitted + served; issuer = the ADE1 cold-key hash;
  relay forging = 0.
- **LIVE-3 (peer adoption — the bounty leg):** the Haskell `cardano-node-preview` peer logs
  `AddedToCurrentChain` for the SAME block hash. `ba02_evidence::correlate` over the operator-captured peer
  log produces a `Ba02PreviewManifest` (network_magic=2). Evidence off-repo; commit under a venue-tagged path.

## 5. NOT claimed here

Live forge (needs stake); peer adoption (LIVE-3); bounty (CERT); preprod. LIVE-2 documents the procedure and
verifies the machinery + forge-capable ingress on the current binary — the forge itself awaits stake.
