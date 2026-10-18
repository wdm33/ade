# SLICE LV-4 — a stake snapshot keeps what cardano's keeps: whole credentials and frozen pool parameters

**Status:** IMPLEMENTED 2026-10-17. Live 304→305 bar met (448 of 448 pools, §8). CE-LV4-12(b)–(d), the
305→306 boundary, are still running live; DC-EPOCH-43/44 stay `partial` until they are in. This replaces the narrower "LV-4: key/script credential" slice named in
the LV-2 record. The owner chose one combined slice, because both remaining reward defects have the
same cause. No code yet.

**Entry state:** `878cb7e5`. Evidence:
`docs/evidence/run-stores/preprod-live2c/lv2-ce10-v9-live-seed-boundary-reward-native.md` §3, §4(b), §6.

---

## 1. THE CONTRACT — answered before any design

> **What does a stake snapshot hold, and where does the reward calculation get a pool's parameters?**

**cardano** (node 11.0.1, PV 11). This is read from the reference state captured at slot 130,563,009
(`oracle-end304-node/ipc/ledger-state-306.json`, `esSnapshots`), not quoted from source:

```
pstake{Mark,Set,Go} = {
  activeStake:        { "keyHash-<28>" | "scriptHash-<28>" -> { swdDelegation: pool, swdStake } },
  stakePoolsSnapShot: { pool -> { accountId: {keyHash|scriptHash}, cost, margin, pledge,
                                  selfDelegatedOwners, selfDelegatedOwnersStake,
                                  numDelegators, stake, stakeRatio, vrf } }
}
```

- Credentials are keyed **with their key/script tag**.
- Each snapshot carries **its own copy of every pool's reward-relevant parameters**, frozen when the
  snapshot was taken. This includes the reward account with its tag. The reward calculation reads these,
  not the current registrations.

**Ade** (`epoch.rs`):

```rust
pub struct StakeSnapshot {
    pub delegations: BTreeMap<Hash28, (PoolId, Coin)>,   // tag dropped
    pub pool_stakes: BTreeMap<PoolId, Coin>,              // no pool parameters at all
}
```

The reward loop (`rules.rs`, `apply_epoch_boundary_with_registrations`) reads `cert.pool.pools`,
the registrations **as they are at the boundary**. It skips any pool that is no longer there
(`None => continue`). It keys rewards by bare hash and credits the key credential first, then the
script credential.

> **Therefore: a snapshot carries whole credentials and frozen pool parameters, and the reward
> calculation reads pool parameters only from the go snapshot.**

### 1.1 Consequences, measured

| | where | size |
|---|---|---|
| A script credential's reward is credited to the key credential with the same 28 bytes | 304→305, credential `900fbcf7…7fedb`, pool `8ed5ab11…eea88` | 11,802 |
| A retired pool is not paid for blocks it made, because it is gone from current registrations | 305→306, pool `8ed5ab11…eea88`; reserves +42,082,928,795, treasury −7,225,473; one re-delegated former member −16,453,955 | 42,082,928,795 |

**Correction to the LV-2 record.** The LV-2 record (§6) says the retired pool's blocks in epoch 304
were "its last". The reference state shows otherwise: the pool made **896 blocks in epoch 305** and
**27 more early in 306**, after it retired. Leadership for those epochs comes from snapshots taken
while it was still registered. So the defect recurs at 306→307 and 307→308, not only at 305→306.

### 1.2 Other places the same loss shows up (from reading the code; each is in scope)

- **Owners are looked up by bare hash.** cardano owners are key hashes only, so the owner lookup can
  match a script credential with the same bytes (`self_delegated_owner_stake`).
- **The operator's reward account is projected to a bare hash.** The account carries its tag in its
  header byte (`reward_account_credential` already decodes it for the deposit refund). Preprod has one
  pool with a script reward account (`cbfa34f3…3825`). It made no blocks in 304–306, so there is no
  live witness for this case.
- **The DRep voting stake is looked up by bare hash** (`governance::derive_drep_voting_stake`).
- **The boundary mark keeps only one of two credentials with the same bytes.** It inserts by
  `cred.hash()`, so a later entry overwrites an earlier one. `pool_stakes` still sums both, so the
  credential-side total (DC-EPOCH-40) can disagree with the per-pool sum.
- **The seed import may write a script reward account with a key header.** The cert-state import
  (`ledgerdb_state::read_pool_params`) always writes the `0xe0` key header. Whether the source
  encoding carries the tag at all is obligation E4.

---

## 2. INVARIANTS

**DC-EPOCH-43 (new). A stake snapshot is keyed by the whole stake credential.** Key and script
credentials with the same 28 bytes are distinct entries everywhere a snapshot is built, stored,
fingerprinted, imported or read: the reward calculation, the DRep stake, the leadership total, the
codec and the seed import. Reward deltas are keyed by the whole credential and credited to exactly
that credential. If that credential is unregistered, the reward goes to the treasury. There is no
"key first, then script" resolution. Pool owners are matched as key credentials only.

**DC-EPOCH-44 (new). The reward calculation reads pool parameters only from the go snapshot.**

- Every snapshot carries, for each pool registered when it was taken, that pool's pledge, cost,
  margin, owners and tagged reward account, frozen at that moment.
- The mark built at a boundary takes them from the registrations **before** future-pool adoption
  and **before** POOLREAP, matching cardano's order (SNAP runs before POOLREAP).
- The reward loop never reads current registrations for these values.
- A pool with blocks and go-snapshot stake but no frozen parameters is a typed terminal, never a
  silent skip.

**Strengthened:** DC-EPOCH-42. Owner stake is read over whole credentials, so only the owner's key
credential counts.

---

## 3. DESIGN

### 3.1 The type

```rust
pub struct StakeSnapshot {
    pub delegations: BTreeMap<StakeCredential, (PoolId, Coin)>,
    pub pool_stakes: BTreeMap<PoolId, Coin>,
    pub pool_params: BTreeMap<PoolId, SnapshotPoolParams>,
}

/// The reward-relevant parameters of one pool, frozen when the snapshot was taken.
pub struct SnapshotPoolParams {
    pub pledge: Coin,
    pub cost: Coin,
    pub margin: (u64, u64),
    pub reward_account: StakeCredential,
    pub owners: BTreeSet<Hash28>,
}
```

The reward account is a `StakeCredential`, not the 29 raw bytes. Network identity is not a reward
input; cardano's `accountId` is the credential alone.

VRF is **not** added. Leadership already freezes each pool's VRF when the leadership distribution is
captured (`FrozenLeadershipPoolDistr::from_boundary_snapshot`, DC-EPOCH-25). Measured on the reference
state: no pool's set-snapshot VRF differs from its current one, and nothing in this slice needs it.

### 3.2 Who builds pool parameters

| path | source |
|---|---|
| boundary mark (`build_boundary_mark_snapshot`) | `cert.pool.pools` at SNAP: before future-pool adoption, before POOLREAP |
| pre-Conway reward-only stub (`rules.rs`) | the same |
| seed import (`ledgerdb_state::read_stake_snapshot_full`) | the snapshot's own per-pool map, which is skipped today (E1) |
| reduced follower (`reduced_snapshot::form_mark_snapshot`) | none. It already carries no delegations and feeds no reward (E6) |
| test corpus loader (`ade_testkit` `snapshot_loader`) | each era's own snapshot pool-params map (E7) |

### 3.3 The reward loop

- Iterate `go.pool_stakes` and take `params` from `go.pool_params`. Missing params for a pool with
  blocks is `LedgerTransitionError`/terminal (DC-EPOCH-44). A missing pool without blocks keeps
  today's zero-reward skip.
- The operator credential comes from `params.reward_account`.
- Owner stake looks up `StakeCredential::KeyHash(owner)`.
- `reward_deltas: BTreeMap<StakeCredential, Coin>`. The apply step looks up exactly that credential in
  registrations; if it is not registered, the reward goes to `delta_t2`.
- The PV ≤ 6 pre-filter (`is_cred_registered`) uses the real credential in place of the `KeyHash`
  projection.

### 3.4 Versions

Every encoding that contains a `StakeSnapshot` changes, so:

- `STORE_SEMANTICS_VERSION` goes 9 → 10, with a new append-only lock entry;
- the snapshot framing `SCHEMA_VERSION` and `EPOCH_ACCUMULATOR_SCHEMA_VERSION` bump if their bytes
  change, which they will;
- the fingerprint encoding of a snapshot changes.

A v9 store is refused, not migrated, and the live run uses a fresh bootstrap.

### 3.5 What does NOT change

`maxPool`, apparent performance, the member formula, pots arithmetic, the zero-stake omission rule
of the mark, leadership membership and its VRF capture, POOLREAP and the deposit refund, the
seed-boundary reward authority (LV-2).

---

## 4. SLICE-ENTRY OBLIGATIONS — answered by measurement before code

| # | Question | How |
|---|---|---|
| **E1** | What is the CBOR shape of the per-pool map in the seed snapshot's mark/set/go? Ade skips it today ("not vrf-first"). Decode it from the certified seed bytes and compare every pool's pledge, cost, margin, owners and reward account with the nearest reference capture | the seed bytes plus the reference JSON |
| **E2** | Which pools does cardano's per-pool snapshot hold: every registered pool at SNAP, or only some? Go has 555 pools for 448 with stake, so zero-stake pools are included. Confirm the rule against the registrations at the matching boundary, so the boundary-built mark holds the same set | reference captures at the end of 304 and 305 |
| **E3** | Does any pool with stake in a reference go snapshot lack frozen parameters? Expected: none, which justifies the terminal in DC-EPOCH-44 | reference JSON |
| **E4** | How does the seed's cert-state pool encoding carry a script reward account (`cbfa34f3…3825`)? Does Ade's import write it with a key header today? If yes, it is fixed here; it is the same loss | the seed bytes |
| **E5** | Can a key credential and a script credential with the same bytes both be delegated in one snapshot on preprod? Count them in the reference snapshots (0 in the end-of-303 go). If none exist, the overwrite in §1.2 has unit witnesses only | reference JSON |
| **E6** | Does any path feed a reduced (`form_mark_snapshot`) snapshot to the reward calculation? If one does, DC-EPOCH-44's terminal would fire there, so confirm it cannot | code reading plus existing tests |
| **E7** | The mainnet historical-era comparisons (`ade_testkit` `epoch_oracle_comparison`) build `go` from older snapshot formats. What does each era's snapshot carry for pool parameters, and can the loader fill `pool_params` from it? If an era carries none, that comparison must fail loudly, not pay nothing silently | corpus snapshots |

### 4.1 Results — 2026-10-17

Raw seed bytes: `preprod-snapshot-6009/db/ledger/129813427/state`, read with the standard `cbor2`
library. Reference: `oracle-rupd304-node/ipc/ledger-state-seed304.json`, the same point, from
cardano-node.

**E1 — the per-pool entry is `array(10)`**, keyed by pool id:

```
[ stake, stakeRatio(tag30), selfDelegatedOwners(set keyhash), selfDelegatedOwnersStake,
  vrf, pledge, cost, margin(tag30), numDelegators, accountId = [tag, hash28] ]
```

Credentials in `ssStake` are `[tag, hash28]` (0 = key, 1 = script). Decoded this way, all three seed
snapshots equal the reference **in every field, for every pool and every credential**:

| | credentials (script) | pools (zero stake) | field mismatches |
|---|---|---|---|
| mark | 25,382 (251) | 555 (109) | 0 |
| set | 25,362 (251) | 554 (110) | 0 |
| go | 25,324 (251) | 546 (107) | 0 |

The snapshot does not store the full owner list. It stores the owners **delegated to the pool** at
snapshot time, which is LV-3's set.

**E2 — every pool registered at SNAP, before POOLREAP.** Reference at slot 130,563,009: the mark
taken at 305→306 holds 558 pools. That is the 557 still registered plus `f9647122…`, which was
reaped at that boundary. No registered pool is missing, including pools with no delegators. The
future-pool map was empty at that boundary, so the "before adoption" half has unit witnesses only.

**E3 — none.** No pool with stake in any of the three seed snapshots lacks parameters. The same holds
in the slot-130,563,009 reference. DC-EPOCH-44's terminal stands.

**E4 — YES, a real import defect, wider than suspected.** In the cert state a pool's reward account
is `[credential tag, hash28]`, for example `cbfa34f3…` → `[1, 97161932…]`. `read_pool_params`
documents it as `[network, hash28]`, reads the tag as a network number and writes header
`0xe0 | tag`. So:

- every key account is imported with header `0xe0`. That is right on preprod by accident and has the
  wrong network nibble on mainnet;
- the script account is imported as `0xe1`, a **key** credential.

`reward_account_credential` therefore sees a key credential, and both the leader reward and the
POOLREAP deposit refund would go to the wrong credential. Fixed here: decode the tag and build the
header from the tag and the manifest network id.

**E5 — none on preprod.** No key and script credential share bytes inside any of the three seed
snapshots, or the reference snapshots at slot 130,563,009. So the `900fbcf7…` collision is between
the snapshot's script credential and a **key registration** with the same bytes. That is the
apply-step resolution, not the mark overwrite. The overwrite has unit witnesses only.

**E6 — no.** `reduced_snapshot::form_mark_snapshot` has no caller outside its own module.

**E7 — the older formats carry full pool parameters.** The corpus go snapshot is `array(3)[stake,
delegations, poolParams]`. `snapshot_loader::parse_go_pool_params` already parses pledge, cost,
margin, the raw 29-byte reward account and owners, and already uses them as the harness's
`cert.pool.pools`. The historical comparisons in effect already pay from frozen parameters. The
loader fills `pool_params` from the same parse, so no change in those numbers is expected.

### 4.2 Design amendments made by the obligations

1. `SnapshotPoolParams.owners` holds the **self-delegated** owners, as cardano's snapshot does. The
   boundary mark computes them at SNAP from the pool's owners filtered by delegation to that pool in
   the cert state. That is cardano's `spsDelegators` membership, not the zero-stake-filtered mark, so
   an owner with zero stake is still listed and contributes 0. The seed import takes the stored set
   as-is. `self_delegated_owner_stake` is unchanged.
2. The cert-state pool import decodes the reward account as a tagged credential (E4). The 29-byte
   `reward_account` it builds carries the real tag bit and the manifest network nibble. The native
   import's commitment bytes change with it, which is covered by the store bump.
3. The stored per-pool values are pledge, cost, margin, self-delegated owners and the tagged reward
   account. `stake`, `stakeRatio`, `numDelegators` and `selfDelegatedOwnersStake` can all be derived
   from the snapshot itself, so they are not stored and cannot disagree with it.

### 4.3 Amendments made during implementation

4. **The empty-owner pledge skip is gone at PV ≥ 4.** The frozen set is the *self-delegated* owners, so
   "no owners listed" and "no owner delegated here" are now the same empty set. Keeping the skip would
   have paid exactly the pool LV-3 stopped paying. cardano has no such skip: an empty set means owner
   stake 0, and any positive pledge is unmet. The skip was a workaround for the corpus loader's
   error-tolerant owner parse, and CE-LV4-13 measures what removing it does there. The pre-Mary path
   (`protocol_major < 4`) is unchanged.
5. **The v1 fingerprint is frozen, not moved.** `fingerprint_v1` is historical evidence only, so it
   keeps its own copy of the pre-LV-4 snapshot encoding (`fingerprint_snapshots_v1`). Only the
   production v2 component changes. The test that pinned "v1 and v2 differ only in the UTxO component"
   now pins that they also differ in the snapshot component.
6. **The byte pin for a full-state encoding is re-anchored, not dropped.** The pin in
   `rvbp_live_path_reduced_dispatch.rs` stays anchored to the pre-P2 bytes, with LV-4's one substitution
   spelled out: each snapshot gains an empty third map, and the enclosing byte string grows by 3.
   Nothing else can move unnoticed.
7. **Versions:** `EPOCH_ACCUMULATOR_SCHEMA_VERSION` 4 → 5 and snapshot framing `SCHEMA_VERSION` 2 → 3,
   so an old blob is refused by its version and not by a shape error. A new test refuses a v4
   accumulator. `ci_check_snapshot_pool_set_inclusion.sh` pinned the accumulator version at exactly 4;
   it now requires ≥ 4, and its own rule (a v3 store fails closed) still holds.
8. **The corpus loader keeps its KeyHash convention.** Its parsers drop the credential tag in 24 places,
   and it registers every delegator as a key. Its snapshot keys the same way so the two surfaces agree.
   It gains frozen pool parameters, built from each era's own snapshot by the ledger's
   `reward_account_credential`.
9. **The diagnostic nibble test now asserts the corrected behaviour.**
   `reward_nibble_disagreement_is_diagnostic_not_terminal` keeps its name, which the registry and its
   gate reference. A registration's account carries no network, so the observed nibble is the
   manifest's, and a script tag is not reported as "network 1".
10. **Untouched but retyped:** the legacy reward skeleton in `epoch.rs` (`compute_rewards`,
    `retire_pools`) is called only from its own tests. It gets the type changes and nothing else.

---

## 5. MECHANICAL ACCEPTANCE CRITERIA

| CE | Criterion | judged by |
|---|---|---|
| **CE-LV4-1** | A key credential and a script credential with the same bytes are two snapshot entries. Both stakes count in `pool_stakes` and in `total_active_stake`, and the two totals agree | unit |
| **CE-LV4-2** | A script credential's reward is credited to that script credential, and never to a registered key credential with the same bytes. An unregistered script credential's reward goes to the treasury | unit |
| **CE-LV4-3** | **Retired pool.** A pool absent from current registrations but present, with blocks, in the go snapshot is paid from its frozen parameters: operator and members | unit |
| **CE-LV4-4** | **Re-registered pool.** A pool whose pledge, cost or margin changed after the snapshot is paid with the frozen values | unit |
| **CE-LV4-5** | An operator with a script reward account receives the leader reward on the script credential. An owner is matched as a key credential only | unit |
| **CE-LV4-6** | A pool with blocks and go stake but no frozen parameters is a typed terminal | unit |
| **CE-LV4-7** | DRep voting stake is summed over whole credentials | unit |
| **CE-LV4-8** | The snapshot codec and fingerprint round-trip both credential kinds and all pool parameters. A v9 store, accumulator or snapshot record is refused | unit plus `ci_check_store_semantics_lock.sh` |
| **CE-LV4-9** | **Seed import.** The three seed snapshots carry frozen parameters equal to the reference for every pool, and credentials with their tags | evidence harness |
| **CE-LV4-9b** | **Cert-state import (E4).** A registration whose reward account is `[1, h]` imports as script credential `h`, and `[0, h]` as key credential `h`. The network nibble comes from the manifest and never from the tag | unit |
| **CE-LV4-10** | **Structural.** No `Hash28`-keyed credential map and no `.hash()` projection remains in the snapshot, reward or DRep-stake paths. The reward loop does not read `cert.pool.pools` | `ci/ci_check_snapshot_fidelity.sh` (new) |
| **CE-LV4-11** | **304→305 on the sealed seed** (`seed_boundary_reward_census`): `900fbcf7…7fedb` matches. The only residuals left are the two seed-point artifacts and the two non-RUPD credits already named in the LV-2 record | evidence harness |
| **CE-LV4-12** | **Live, fresh follower-only bootstrap**, local docker `cardano-node-preprod`: (a) sealed epoch-306 stake equals the reference for 448 of 448 pools; (b) after 305→306, reserves and treasury equal the reference; (c) sealed epoch-307 stake equals the reference for 447 of 447 pools; (d) the node follows past 307→308 with no terminal | live run plus the existing reference captures |
| **CE-LV4-13** | Every existing reward suite passes, including the mainnet historical-era comparisons. Each change in a reward number is named and matched to the reference | existing suites |
| **CE-LV4-14** | Negative-tested | mutations below |

CE-LV4-12(a)–(c) close LV-2's CE-LV2-10(b) as well. When they are green, DC-EPOCH-40 and DC-EPOCH-41
can move from `partial` to `enforced`.

### Required mutations

Each mutation is asserted applied, the named witness must fail, and the tree is restored byte-exact.

- Reward apply falls back from the script credential to the key credential (must fail CE-LV4-2 and
  CE-LV4-11).
- The reward loop reads `cert.pool.pools` (must fail CE-LV4-3 and CE-LV4-4).
- The mark takes pool parameters after future-pool adoption (must fail CE-LV4-4).
- The mark inserts by bare hash (must fail CE-LV4-1).
- The missing-params terminal becomes `continue` (must fail CE-LV4-6).
- The operator credential comes from a key projection (must fail CE-LV4-5).
- The cert-state import reads the reward-account tag as a network number again (must fail
  CE-LV4-9b).
- `STORE_SEMANTICS_VERSION` is left at 9 (must fail the lock gate).

---

## 6. EXPLICITLY NOT IN THIS SLICE

- **The forge leg (CE-B12-10).** It needs operator keys and the owner's go.
- **The catch-up stop at an epoch boundary** (`LeadershipEpochNotSealed`). The v9 follower stopped
  this way at 306→307 and again at 309→310, and carries on after a restart. It is recorded as a
  separate issue and should be looked at before an unattended forge run.
- **The 47 gate scripts that already fail** at `67ba3e3c`.
- **Reference captures past the end of epoch 305.** CE-LV4-12 is judged only where captures exist.
  Following to 307→308 checks that the node continues, not that the numbers there are exact.

---

## 7. OPEN DECISIONS FOR THE OWNER

None at entry. If E7 finds a historical era whose snapshots carry no pool parameters, that
comparison would need to be reclassified or fed another way. That choice would come back to the
owner before it is made.

---

## 8. RESULTS — 2026-10-17

### Live, fresh follower-only v10 bootstrap

Snapshot 6009, local docker `cardano-node-preprod`, no operator keys. Binary sha256 `831554492ddf3b51…`,
store `~/.cardano-live1/ade-preprod-v10`.

| CE | result |
|---|---|
| seed | sealed NATIVE, `ssFee` 23,699,540,134 (unchanged from LV-2) |
| **CE-LV4-11** | 304→305 on the sealed seed: **11,052 of 11,056** credentials equal the reference update, up from 11,051. `900fbcf7…7fedb` now matches. Reserves draw 7,422,111,617,574, exact. The rest are the LV-2 record's named residuals: two seed-point artifacts (only in the reference, together the +6,308,509 treasury difference), two credits outside the reward update (+1,000,000,000 and +500,000,000), and two 1,000,000,000 credits to credentials in no go pool |
| **CE-LV4-12(a)** | live crossing 304→305, `pool_pot=28118802294396`. Sealed epoch-306 stake against the reference end-of-304 snapshot: **448 of 448 pools exact, total 1,626,474,372,102,448, denominator delta 0.** The v9 store was 11,802 short on one pool. The header issuer `0ed90e52…` matches |
| CE-LV4-12(b)–(d) | the 305→306 boundary: pending, running live |

CE-LV4-12(a) is also LV-2's CE-LV2-10(b).

### Mechanical

| | |
|---|---|
| CE-LV4-1…7, 9b | 8 integration tests (`lv4_snapshot_fidelity.rs`) and 4 unit tests, passing |
| CE-LV4-8 | codec and fingerprint round-trip; v4 accumulator, v2 framing and v9 store refused; lock entry v10 recorded |
| CE-LV4-10 | `ci/ci_check_snapshot_fidelity.sh` OK |
| CE-LV4-13 | mainnet `epoch_oracle_comparison`: 26 pass and 1 pre-existing failure (`committee_oracle_mainnet_575_576_noop_agreement`), the same as the baseline at `4dfa6b0c`. One output differs: the diagnostic that swaps in *current* registration parameters now shows zero effect, which is LV-4's point. A first run also moved one Conway `deltaT2` by 816,261,323. That was a harness artifact: script reward accounts against the loader's key-only registrations. The loader now applies its key convention to reward accounts, and `deltaT2` equals the baseline exactly. Removing the empty-owner pledge skip changed no mainnet number |
| suites | ade_ledger 1082, ade_node 645, ade_runtime 581 pass; ade_testkit builds |
| gates | 47 of 277 fail, the same 47 as at the baseline. `ci_check_owner_stake_authority.sh` now requires the frozen owner set (DC-EPOCH-42 strengthened) |

### Mutations (each asserted applied; tree restored byte-exact)

| mutation | caught by |
|---|---|
| M1 apply falls back script → key | CE-LV4-2, the operator test, gate (c); **real data:** CE-LV4-11 back to 11,051 with `900fbcf7…` mismatched |
| M2 reward loop reads `cert.pool.pools` | the LV-3 tests, the LV-4 tests, gate (b) |
| M3 mark takes post-adoption params | the mark test, gate (d) |
| M4 mark keys by bare hash | CE-LV4-1, gate (d) |
| M5 missing-params terminal → `continue` | CE-LV4-6, gate (b) |
| M6 operator from key projection | CE-LV4-5, gate (c) |
| M7 import reads tag as network | CE-LV4-9b, gate (h) |
| M8 store semantics left at 9 | lock gate, gate (g) |
| M9 DRep stake by bare hash | CE-LV4-7, gate (e) |
| M10 seed import skips the per-pool map | both decoder tests, gate (h) |

For M2, `cargo test -p ade_ledger` stops at the first failing test binary, so the named list shows the
LV-3 tests. The gate's own run of the LV-4 tests shows them failing as well.
