# LV-2 entry obligation E1 — Ade's native reward on the seed state vs cardano's reward update

preprod, 2026-10-10. Slice: `SLICE-LV-2-seed-boundary-reward-authority.md` §4. Read-only; no
production code changed.

**Result.** The wiring LV-2 proposes is sound: fed `nesBprev` and `ssFee`, Ade's existing reward
function reproduces cardano's 304→305 reward update for **11,050 of 11,056 credentials exactly**.
Every residual is attributed. One of them is a real, pre-existing defect in the shared reward function
that LV-2 does not cover: **the pledge check counts an owner's stake wherever it is delegated; cardano
counts only owners delegated to the pool itself.** Ade pays a pool cardano pays nothing.

E1 as written ("reproduce `rs` exactly, or the slice stops") is therefore NOT met.

---

## 1. METHOD

Harness: `crates/ade_runtime/tests/seed_boundary_reward_census.rs` (`#[ignore]`d, evidence-only).

On a COPY of the v7 store's `epoch-accumulator.redb`: `reset_to_bootstrap`, load the sealed seed
accumulator (slot 129,813,427), then emulate LV-2's `Native` path — drop the empty imported record,
set `prev_epoch_fees` to the snapshot's `ssFee` (23,699,540,134, read from the reference ledger
state), and call `cross_epoch_boundary` into epoch 305. Compare each reward account's change against
cardano's Complete `nesRu.rs` (`rupd-304to305-rs-by-credential.json`, decoded from the reference
node's CBOR ledger state at slot 130,062,360).

The crossing runs on the SEED state, not the end-of-304 state: registrations and delegations made in
the rest of epoch 304 are absent. Two residuals below are that artifact and nothing else.

## 2. NUMBERS

```
SEED  reserves=12993813079206744 treasury=1945191497759078 bprev_pools=79 bprev_blocks=19463
      pending = Some(0 credentials, 0, 0, 0)          <- the empty record, as sealed
[epoch_boundary] pool_pot=28118802294396              <- was 0 on the live run

cardano credentials 11,056    exact 11,050    differ 3    only-cardano 3    only-Ade 138
```

| | cardano | Ade (native, seed state) | difference |
|---|---|---|---|
| reserves draw (`deltaR`) | 7,422,111,617,574 | 7,428,329,809,017 | +6,218,191,443 |
| treasury gain (`deltaT`) | 7,029,700,573,599 | 7,029,706,882,108 | +6,308,509 |

## 3. EVERY RESIDUAL, ATTRIBUTED

| what | amount | cause |
|---|---|---|
| 135 credentials, all members of pool `13e8600b…2418a`, paid only by Ade | 2,528,789,508 | **pledge check — §4** |
| `d2cfce07…8ee5f` (that pool's reward account) paid more by Ade | 3,689,401,935 | **pledge check — §4** |
| sum of the two rows above | **6,218,191,443** | = the extra reserves draw, exactly |
| `94857be3…4dfd5b` not paid by Ade | 6,296,662 | unregistered at the seed, re-registered later in 304 (reference: no account at slot 129,816,426, account at 130,062,360). Seed-state artifact; Ade sent it to the treasury |
| `85ecddcb…99163` not paid by Ade | 11,847 | unregistered in both; cardano's `rs` lists it and filters at apply. Ade sent it to the treasury |
| sum of the two rows above | **6,308,509** | = the extra treasury gain, exactly |
| `900fbcf7…7fedb`: cardano pays the SCRIPT credential, Ade pays the KEY credential with the same 28 bytes | 11,802 | **credential discriminant — §5** |
| `7a507e54…98756` +500,000,000 | | pool deposit refund: reward account of pool `8ed5ab11…`, reaped at this boundary. Not a reward |
| `4245236a…a6454`, `03b56d5e…0a984`, `6103000c…4c0a5` +1,000,000,000 each | | non-reward boundary credits, not from reserves or treasury (both pots reconcile without them). Source not identified. For the two that are pool-delegated, cardano's end-of-304 mark agrees with Ade's to the lovelace, so cardano credits them too |

E2 (go-snapshot pool parameters vs current registration): no residual is attributable to it.

## 4. THE PLEDGE CHECK

Pool `13e8600b86987e7b20fc539c83a57343d2e5b7fc646ea78f8322418a`, 48 blocks in epoch 303:

```
pledge                     4,500,000,000,000
owner                      d2cfce07…8ee5f
owner's go stake           5,275,349,445,153   delegated to pool 8aa46908…0258b — a DIFFERENT pool
cardano go snapshot        selfDelegatedOwners: []   selfDelegatedOwnersStake: 0
```

- **cardano** (node 11.0.1, PV 11): pledge 4.5M > self-delegated owner stake 0 → the pool earns
  nothing. Its 135 members and its reward account get no reward from it.
- **Ade** (`rules.rs`, "Shelley pledge satisfaction"): sums each owner's stake from the full go
  distribution "regardless of their delegation" → 5.275M ≥ 4.5M → pays 6,218,191,443.

cardano-ledger `master`, `Cardano.Ledger.State.SnapShots`:

```haskell
selfDelegatedOwners =
  Set.filter (\ownerKeyHash -> KeyHashObj ownerKeyHash `Set.member` spsDelegators) spsOwners
```

and `Cardano.Ledger.Shelley.Rewards`: `if pledge <= selfDelegatedOwnersStake then maxPool' … else 0`,
with `poolOwnerRelativeStake = selfDelegatedOwnersStake %? totalStake`. The same quantity feeds the
leader term.

Of the 79 producing pools this is the only one where the two definitions give different outcomes
(three others have a non-self-delegated owner, each with zero pledge and no stake).

⚠ **This contradicts a recorded project rule** — that leader/pledge owner stake is the owner's full
go stake and must never be filtered by delegation target, adopted on 2026-06-30 on the strength of
the preview CE-3d differential. This file does not explain that earlier result. It records that on
preprod at PV 11 the reference pays nothing where that rule pays 6.2 billion, and that the current
ledger source defines the quantity as self-delegated only.

**Scope of the damage.** This is the shared reward function, so it applies at every boundary, not
just the seed. Each epoch that pool produces, Ade over-draws reserves and over-credits ~136 accounts.

## 5. THE CREDENTIAL DISCRIMINANT

The reference state holds two accounts with the same 28 bytes `900fbcf7…7fedb`: a script credential
(delegated to pool `8ed5ab11…`, in the go snapshot) and a key credential (registered, undelegated,
balance 0). Cardano rewards the script credential. Ade's go snapshot is keyed by the bare hash and the
reward is credited to the key credential. 11,802 lovelace here; the class is a reward landing in the
wrong account whenever a key and a script credential share a hash.

## 6. WHAT THIS MEANS FOR LV-2

- The `Native` seed path is the right design and needs no seed-only arithmetic.
- CE-LV2-3 (all 11,056 credentials) and CE-LV2-10(b) (448 pools byte-equal) cannot pass while §4
  stands. §5 would not move a pool total at this boundary (the pool is reaped) but fails CE-LV2-3.
- E3 and E4 were not run.

## 7. ARTIFACTS (off-repo)

```
~/.cardano-live1/oracle-rupd304-node/rupd-304to305-rs-by-credential.{json,txt}
~/.cardano-live1/oracle-rupd304-node/ipc/ledger-state-seed304.json     go snapshot, accounts, feeSS
```
