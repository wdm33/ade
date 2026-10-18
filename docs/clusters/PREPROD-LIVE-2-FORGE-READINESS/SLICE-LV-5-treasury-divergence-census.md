# SLICE LV-5 — find where Ade's treasury first leaves cardano's, before touching any arithmetic

**Status:** DRAFT 2026-10-18. Findings only: this slice measures, it does not design or fix. Opened by
the owner's decision on LV-4 §7. That decision moved the treasury half of CE-LV4-12(b) here.

**Entry state:** `bc828c57`. Evidence: `docs/evidence/run-stores/preprod-live2c/lv4-ce12-v10-live-snapshot-fidelity.md`
§3(b), §4.

---

## 1. THE QUESTION

**Did the canonical epoch-305 chain contain a treasury donation that cardano took into its treasury
and Ade did not?**

Behind it is a broader question this slice must answer whatever the donation answer is: **at which
state transition do Ade's treasury and cardano's first differ?**

### 1.1 What is already known

| | Ade v10 | reference | difference |
|---|---|---|---|
| treasury after 305→306 | 1,959,103,173,719,172 | 1,959,103,174,719,172 | **−1,000,000** |
| reserves after 305→306 | 12,979,123,112,128,607 | 12,979,123,112,128,607 | 0 |

- The v9 run had the same −1,000,000, inside its −7,225,473. LV-4 did not cause it.
- The reserves are exact. A reward-pot error normally splits between reserves and treasury, so the
  reward calculation is an unlikely source. "Unlikely" is not a finding.
- **Not yet known:** whether the gap already exists after 304→305, which is the seed boundary,
  governed by DC-EPOCH-41. The LV-4 numbers above are cumulative.
- **Lead, not a finding:** Ade's epoch accumulator applies no Conway treasury donations. Transaction
  body key 22 is decoded (`ade_codec/src/conway/tx.rs`) and counted in the balance check
  (`ade_ledger/src/conway.rs`), but nothing adds it to a pot.
- One more treasury difference is on record, the +6,308,509 `deltaT` of the seed-only crossing
  (CE-LV4-11). It is attributed to two credentials registered after the seed point. It comes from
  crossing the seed state in isolation, not from the live run. F1 must keep it separate.

## 2. HARD PROHIBITION

**Do not repair the 1 ADA by adjusting arithmetic until the first state transition where Ade and
cardano diverge has been identified.**

A round 1,000,000 suggests rounding or an off-by-one, but it can just as well be a missing state
transition: a donation, a deposit, an unclaimed reward. A change that makes the number equal without
naming where it comes from is refused. This slice changes no production code at all (CE-LV5-5).

## 3. DECISION TREE

```
F1  localize: compare treasury (and reserves, donation pot) at every reference point
        ↓  first point P where they differ
F2  scan canonical epoch-305 transactions for a donation (body key 22)
        ↓
    donation present?
    ├─ YES → F3: identify block/tx, sum the amounts, check that cardano moves them to the treasury
    │        at 305→306, and test whether they explain EXACTLY 1,000,000 and the divergence point P
    │        → the follow-on design slice is "donations", and only then
    │
    └─ NO  → donation hypothesis mechanically REJECTED; donation support is NOT designed
             → F4: census the other treasury-affecting transitions at P
```

## 4. MEASUREMENTS (slice-entry; each answered with numbers before anything else)

**Tool ladder, in order.** First existing independent tools. Then the existing reference surfaces:
the reference node's LedgerDB snapshots, read as raw CBOR with `cbor2` as before. Then Ade's own
decoders, through an `#[ignore]` evidence test on a COPY of a store, as the census tests do. A new
reader is the last resort, and only once the ladder above bottoms out. `EpochAccumulatorStore::open`
is read-write, so every harness runs on a copy.

**F1 — first divergence point.** Compare Ade with the reference for treasury and reserves, plus the
reference's donation pot (`utxosDonation`), at:

| point | reference surface | Ade surface |
|---|---|---|
| seed (304, slot 129,813,427) | snapshot 6009 (both sides import it) | the sealed seed |
| after 304→305 | `oracle-end304-node` LedgerDB snapshot at slot 130,133,280 | accumulator state after 304→305 (copy `acc-v10-306.redb`) |
| late 305 | `oracle-end304-node` LedgerDB snapshot at slot 130,515,796 | (Ade holds no donation pot) |
| after 305→306 | capture at slot 130,563,009 | accumulator state after 305→306 (copy `acc-v10-307.redb`) |

The result names the first point P where the treasury differs. **If P is after 304→305, the gap is
inside the seed-boundary crossing DC-EPOCH-41 governs.** The owner is told before 41 is considered for
`enforced`.

**F2 — donation presence, from two independent sides.**
- **Reference:** `utxosDonation` in the two epoch-305 LedgerDB snapshots, and whether it is zero at
  the 304→305 one.
- **Chain:** every transaction of the canonical epoch-305 blocks (slots 130,118,400 to 130,550,399),
  checked for body key 22 using Ade's own Conway decoder. The block source is decided at entry: Ade's
  stored blocks if they cover the range, otherwise the local docker node's chain through an existing
  reader.

The two sides must agree. If they disagree, that is a finding, not a tie to break.

**F3 — only if F2 finds a donation.** The block and transaction id of each donation, their sum, and
whether that sum equals the reference's `utxosDonation` at the late-305 point plus any donations after
it. Also whether it explains exactly 1,000,000 and the point P from F1.

**F4 — only if F2 finds none.** At the first divergent transition P, census every path that moves
lovelace into or out of the treasury:
- rewards for unregistered credentials;
- gov-action deposit refunds to unregistered reward accounts;
- enacted treasury withdrawals;
- the `treasury_value` field (key 21);
- any deposit forfeiture.

Name the path that carries the 1,000,000.

**F5 — rule check.** The cause found is checked against the mechanisms of DC-EPOCH-40, -41, -43 and
-44. If it lies inside one of them, that is recorded against the rule and reported to the owner.
"Outside the invariant" was the condition of the LV-4 decision, and it is verified here, not assumed.

## 5. MECHANICAL ACCEPTANCE CRITERIA

| CE | criterion | witness |
|---|---|---|
| **CE-LV5-1** | F1 recorded: treasury, reserves and donation pot at every point in the F1 table, with the first divergent point P named | evidence record |
| **CE-LV5-2** | F2 answered from BOTH sides, and the two sides agree, or the disagreement is recorded as a finding | evidence record |
| **CE-LV5-3** | The outcome classified by the decision tree: YES with F3 complete, or NO with F4 naming the carrying path | evidence record |
| **CE-LV5-4** | F5 recorded for DC-EPOCH-40/41/43/44 | evidence record and slice §8 |
| **CE-LV5-5** | No production code changes: `git diff bc828c57 -- 'crates/*/src'` is empty at slice close. Evidence-only `#[ignore]` tests are allowed | `git diff` |

## 6. EXPLICITLY NOT IN THIS SLICE

- **Any fix.** The design slice that follows is chosen by what F1–F4 find, and is drafted only then.
- **DC-EPOCH-41's status.** It waits on F1, and then on the owner's decision about CE-LV2-3 (LV-4 §7).
- **Reference points after 306.** Only existing captures are used.

## 7. OPEN DECISIONS FOR THE OWNER

None at entry.

## 8. RESULTS

Not yet run.
