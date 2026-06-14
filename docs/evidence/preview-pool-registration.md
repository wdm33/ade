# Preview stake-pool registration — ADE1 (redacted evidence)

> **Status:** operational venue setup (release/evidence scope, **NOT** runtime
> authority and **NOT** a block-production claim). Redacted: contains only public,
> on-chain / hosted values. **No key material.** Private keys, certs, and tx bodies
> live outside this repo in the operator workspace `~/Code/rust/ade-ops/` and are
> never committed.
>
> **Hard line — read first.** Registering a pool (and receiving faucet delegation)
> is a **ledger/venue operation**. It proves the pool *identity exists on Preview and
> can receive stake*. It does **NOT** prove Ade produces blocks. Pool registered +
> electable **≠** Ade ready. The block-production gate is **C2-PREVIEW-BA02**: a real
> Haskell peer adopts an Ade-forged Preview block (`AddedToCurrentChain`, same hash,
> `issuerHash` = this pool's cold-key hash). This file makes **no** acceptance claim.

## What this proves (and what it does not)

Preview is the **bounty-valid acceleration venue** (1-day epochs → stake active in
~2–3 days vs preprod's ~10). This evidence shows there is a registered Preview SPO
identity, with a stable cold/VRF/KES/opcert lineage, whose pledge is self-delegated
and which has received ~1M tADA of faucet delegation — so when Ade is proven as a
forger it can run as **this** pool and be elected. It does **not** prove forging,
fork choice, or peer acceptance.

## Identity & on-chain parameters

| Field | Value |
|---|---|
| Network | preview (magic 2, 1-day / 86400-slot epochs) |
| Ticker | `ADE1` |
| Pool ID (bech32) | `pool1gv25n0c5znsdf22mnl0ve0nq7essnluts86s9d3gk2u0xfhlnrh` |
| Pool ID (hex) | `431549bf1414e0d4a95b9fdeccbe60f66109ff8b81f502b628b2b8f3` |
| VRF vkey hash | `5b7540884e5fe865ff0588bd3dfd471978c8c0e3d1e482d7cf1ce378882c94a3` |
| Reward / owner stake addr | `stake_test1upymw9mmqu35jvdz8lcc4ty8wy37mth3ke4c5h6xn040xkg55l423` |
| Pledge | 1,000 tADA (`1000000000`) |
| Fixed cost | 170 tADA (`170000000`, = **live** preview minPoolCost) |
| Margin | 0 % |
| Relay | `relay1.preview.example.invalid:3002` — **placeholder, update to Ade's real relay via re-registration before go-live** |
| Metadata URL | `https://wdm33.github.io/ade-atlas/poolMetaData.json` |
| Metadata hash | `74cd1973bcd119fb32be72c5a136de0382594fae8e7f797a5f63a8a97049cba9` |

> **Venue note:** preview **genesis** `minPoolCost` is 340 tADA, but the **live**
> value (via param update) is 170 tADA — confirmed by `query protocol-parameters` and
> used for the cost above. Deposits match preprod: keyDeposit 2 tADA, poolDeposit 500.

## Registration transaction

| Field | Value |
|---|---|
| Tx hash | `09c1c10987f8752f3fb85a7124af6165ae4293d691947be7cc75d7c179e4538c` |
| Included | block 4,380,693, epoch 1328 (2026-06-14) |
| Certificates | pool registration + stake-address registration (2 tADA) + stake delegation (pledge self-delegation) |
| Deposits spent | 500 tADA (pool) + 2 tADA (stake key) |
| Fee | 0.204177 tADA |
| Pledge stake active | ~epoch 1330 (2-epoch snapshot rule; ~2 days on Preview) |

## Faucet pool delegation (electable stake)

| Field | Value |
|---|---|
| Tx hash | `5008d1f599d8fac86b0bf55c9a86ffd0acb5086d2efe9fb6a7c8ca01e48193fb` |
| Included | block 4,380,716, epoch 1328 (2026-06-14) |
| Certificate | `pool_delegation` → `pool1gv25n0c5…` |
| Faucet stake address | `stake_test1uqv9llclxtgxs8zknjuc9yqy4nusnmngz4jp2jz97rztqrg8hnwql` |
| Delegated amount | ~1,000,015 tADA (`1000014603080` lovelace) |
| Faucet stake active | ~epoch 1330 |

Verified via Koios-preview `account_info` on the faucet stake address (`delegated_pool`
= this pool, `total_balance` ≈ 1,000,015 tADA). The faucet retains control of this
stake and may redelegate later: it makes the pool **electable for testing**, it is not
owned by the operator and does not count toward pledge.

**Electability note:** preview total active stake is ~2.97B tADA, so ~1.0M gives a
~0.034 % share → on the order of **1–2 leader slots per epoch**. Sufficient for a
forge test, but C2-PREVIEW-BA02 may wait an epoch or two for a leader slot; stacking
another faucet delegation onto the same pool id increases headroom.

## Verification (queried via the synced Preview gateway node)

**Owner stake address — registration + delegation** (`query stake-address-info`):

```json
[
  {
    "address": "stake_test1upymw9mmqu35jvdz8lcc4ty8wy37mth3ke4c5h6xn040xkg55l423",
    "rewardAccountBalance": 0,
    "stakeDelegation": {
      "stakePoolBech32": "pool1gv25n0c5znsdf22mnl0ve0nq7essnluts86s9d3gk2u0xfhlnrh",
      "stakePoolHex": "431549bf1414e0d4a95b9fdeccbe60f66109ff8b81f502b628b2b8f3"
    },
    "stakeRegistrationDeposit": 2000000,
    "voteDelegation": null
  }
]
```

**Pool stake snapshot** (`query stake-snapshot`) — the pool's `pools` entry is empty
immediately post-registration (delegations land mid-epoch and are captured at the next
boundary, then propagate `mark → set → go`, active ~epoch 1330). `total` shows the
preview-wide active stake (~2.97B tADA) used for the electability ratio above:

```json
{
  "pools": {},
  "total": { "stakeGo": 2975055901149617, "stakeMark": 2974613056055363, "stakeSet": 2976775769829495 }
}
```

## Operator key set — status

Forging key set **complete** for Preview (held in the operator workspace, never
committed): cold (→ pool id), vrf, **kes**, **node.opcert** (issued KES period 885,
valid ~62 periods ≈ until ~2026-09). The C2 session forges Preview with **this** lineage.

## Pending / deferred (not part of this evidence)

- **Active-stake wait** — ~2 days to epoch 1330, then C2-PREVIEW-RECOVER → C2-PREVIEW-BA02.
- **Conway vote delegation** — `voteDelegation` is `null`; rewards-only, not for block production.
- **Relay** — placeholder; update to Ade's real Preview relay via re-registration.

## Reproduction / provenance

Produced by the operator workspace `~/Code/rust/ade-ops/` (RED, outside this repo),
venue-parametric via `ADE_OPS_ENV=preview scripts/NN-*.sh` driving the dockerized
`cardano-cli` (`ghcr.io/intersectmbo/cardano-node:11.0.1`) against the off-repo synced
Preview gateway node (`cardano-node-preview`, `~/.cardano-node-preview`, N2N 127.0.0.1:3002,
magic 2). Offline key/cert/sign steps run as the host user; node steps run against the
gateway socket only. Keys never leave `ade-ops/preview/ade-pool/keys/`. The metadata is
the venue-agnostic Atlas file, byte-identical to the on-chain hash.
