# Preprod stake-pool registration — ADE1 (redacted evidence)

> **Status:** operational venue setup (release/evidence scope, **NOT** runtime
> authority and **NOT** a block-production claim). Redacted: contains only public,
> on-chain / hosted values. **No key material.** Private keys, certs, and tx bodies
> live outside this repo in the operator workspace `~/Code/rust/ade-ops/` and are
> never committed.
>
> **Hard line — read first.** Registering a pool is a **ledger/venue operation**.
> It proves the pool *identity exists on preprod and can receive stake*. It does
> **NOT** prove Ade produces blocks. Pool registered **≠** Ade ready. A block-
> production claim still requires the C2 ladder: sustained local single-producer →
> local multi-producer / fork-choice → **preprod accepted-block** evidence. This
> file makes **no** acceptance claim and flips **no** RO-LIVE rule.

## What this proves (and what it does not)

It answers one operational question: *is there a registered preprod stake pool,
with a stable cold/VRF identity, whose pledge stake is delegated and whose
metadata resolves to the on-chain hash?* — so that when Ade is later proven as a
forger it can run as **this** pool without a registration round-trip, and so the
~2-epoch stake-activation delay runs in parallel with rung-1/2 work. It does
**not** prove forging, fork choice, or peer acceptance.

## Identity & on-chain parameters

| Field | Value |
|---|---|
| Network | preprod (magic 1) |
| Ticker | `ADE1` |
| Pool ID (bech32) | `pool1gkgwpms49j3nykcukqq33lcz7yu5kymwmze2y087ezc8qqpt397` |
| Pool ID (hex) | `4590e0ee152ca3325b1cb00118ff02f1394b136ed8b2a23cfec8b070` |
| VRF vkey hash | `08a5dbdae7757262ea934842f35ce5c7693fe471544b29841bb90c0d4263436f` |
| Reward / owner stake addr | `stake_test1urfnjc4jruxw2ccz9kc6qrwmp26f7kfrzafa78sv0r827zgvv0v93` |
| Pledge | 1,000 tADA (`1000000000`) |
| Fixed cost | 170 tADA (`170000000`, = network minPoolCost) |
| Margin | 0 % |
| Relay | `relay1.preprod.example.invalid:3001` — **placeholder, to be updated to Ade's real relay via re-registration before go-live** |
| Metadata URL | `https://wdm33.github.io/ade-atlas/poolMetaData.json` |
| Metadata hash | `74cd1973bcd119fb32be72c5a136de0382594fae8e7f797a5f63a8a97049cba9` |

## Registration transaction

| Field | Value |
|---|---|
| Tx hash | `64a977cc563912489604c2b1b1d2a6ccce78a0618ade3749eacb24e32f355e26` |
| Certificates | pool registration + stake-address registration (2 tADA deposit) + stake delegation (pledge self-delegation) |
| Deposits spent | 500 tADA (pool) + 2 tADA (stake key) |
| Fee | 0.204353 tADA |
| Registered in epoch | 293 (2026-06-09) |
| Pledge stake active | epoch 295 (Koios `active_epoch_no`) |

## Faucet pool delegation (electable stake)

| Field | Value |
|---|---|
| Tx hash | `4a545d61cd80e00fbea3fe7d34adcded2b8d2ddf3dbd393d9279dbf2b4fb34c5` |
| Included | block 4,820,842, epoch 294 (2026-06-14) |
| Certificate | `pool_delegation` → `pool1gkgwpms…` |
| Faucet stake address | `stake_test1uqgqru5w9ra8kwgfac0h5r5shekjdd93wh8lehsp8fpghfqk7hq6h` |
| Delegated amount | ~1,000,008 tADA (`1000008344160` lovelace) |
| Faucet stake active | epoch 296 |

Verified via Koios `account_info` on the faucet stake address (`delegated_pool` =
this pool, `total_balance` ≈ 1,000,008 tADA). The `pool_info.live_stake` aggregate
lags the account-level view. The faucet retains control of this stake and may
redelegate it later: it makes the pool **electable for testing**, it is not owned by
the operator and does not count toward pledge.

**Active-stake timeline:** pledge ~9,497 tADA active epoch 295; faucet ~1,000,008
tADA active epoch 296. From ~epoch 296 the pool holds ~1.01M tADA active (~0.1% of
total active stake) — electable, on the order of a dozen-plus leader slots per epoch.

## Verification (queried via the synced preprod gateway node)

**Registered-pool set membership:** `pool1gkgwpms…` is present in the preprod
registered-pool set (set size 506 at query time). **YES.**

**Owner stake address — registration + delegation** (`query stake-address-info`):

```json
[
  {
    "address": "stake_test1urfnjc4jruxw2ccz9kc6qrwmp26f7kfrzafa78sv0r827zgvv0v93",
    "rewardAccountBalance": 0,
    "stakeDelegation": {
      "stakePoolBech32": "pool1gkgwpms49j3nykcukqq33lcz7yu5kymwmze2y087ezc8qqpt397",
      "stakePoolHex": "4590e0ee152ca3325b1cb00118ff02f1394b136ed8b2a23cfec8b070"
    },
    "stakeRegistrationDeposit": 2000000,
    "voteDelegation": null
  }
]
```

**Pool stake snapshot** (`query stake-snapshot`) — pool `mark/set/go = 0` is
expected immediately post-registration; the delegation is captured at the
epoch-293 boundary and propagates `mark → set → go`, going active at epoch 295:

```json
{
  "pools": {
    "4590e0ee152ca3325b1cb00118ff02f1394b136ed8b2a23cfec8b070": {
      "stakeGo": 0, "stakeMark": 0, "stakeSet": 0
    }
  },
  "total": { "stakeGo": 886446899253790, "stakeMark": 1403309893493336, "stakeSet": 912041407323655 }
}
```

## Operator actions — status (2026-06-14)

Completed since registration:
- **Faucet pool delegation** — DONE (section above): faucet ~1M tADA delegated to
  the pool; active ~epoch 296.
- **KES key + operational certificate** — DONE (2026-06-14): generated and bound to
  this pool's cold/VRF lineage; opcert issued at KES period 970 (valid ~62 periods,
  ~until 2026-09). Held in the operator workspace; the C2 session forges with it.

Still open (not blocking, not part of this evidence):
- **Conway vote delegation** — `voteDelegation` is `null`; only required to withdraw
  rewards, not for block production. Optional.
- **Relay** — still the placeholder above; to be updated to Ade's real relay via
  re-registration once Ade stands up as the producer (C2 territory).

## Reproduction / provenance

Produced by the operator workspace `~/Code/rust/ade-ops/` (RED, outside this repo)
driving the dockerized `cardano-cli` (`ghcr.io/intersectmbo/cardano-node:11.0.1`)
against the synced preprod gateway node. Offline key/cert/sign steps run as the
host user; node steps run against the gateway socket only. Keys never leave
`ade-ops/keys/`. The metadata file is served from the Ade Atlas GitHub Pages site
and verified byte-identical to the on-chain hash.
