# SLICE LV-6 — a treasury donation is held in one pot and moved to the treasury once, at the boundary

**Status:** DRAFT 2026-10-18; owner-approved design (§7). E1 answered (§4.1); the other obligations are
answered before any code.

**Entry state:** `a011ec94`. Cause established by LV-5
(`docs/evidence/run-stores/preprod-live2c/lv5-treasury-divergence-census.md`). Epoch 305 carried two
treasury donations of 500,000 each, in valid transactions. Cardano collected them in its donation pot
and added the pot to the treasury at 305→306. Ade keeps no donation pot, so its treasury came out
1,000,000 short. Every other pot and every reward balance agreed.

A small derived cardano-compatibility slice: one missing Conway state variable and the transition
that clears it.

---

## 1. THE CONTRACT (cardano; quoted verbatim in §4.1)

- A **valid** Conway transaction adds its `treasury donation` (body key 22) to the ledger's
  donation pot (`utxosDonation`).
- A **phase-2-invalid** transaction contributes only its collateral, never its donation.
- At the epoch boundary, the **whole** pot is added to the treasury and the pot becomes zero.
- **Order at the boundary:** SNAP → future-pool adoption → POOLREAP → enacted treasury withdrawals
  subtracted → removed-proposal deposits (registered → reward account, otherwise "unclaimed") →
  **treasury += donation pot + unclaimed** → pot cleared. Donations accumulated during epoch N are
  NOT available to the treasury withdrawals enacted at the N→N+1 boundary.

## 2. INVARIANT — DC-EPOCH-45 (new; registered with the implementation)

**Treasury donation pot.** Ade holds exactly one canonical donation pot: one integer, persisted,
inside the versioned authoritative surface.
- (a) A valid Conway transaction adds its donation to the pot. A phase-2-invalid transaction adds
  nothing. Every path that applies a transaction does both, the same way.
- (b) At every epoch boundary the whole pot is added to the treasury exactly once, and the pot is
  zero afterwards. There is no rounding and no special case.
- (c) Every bootstrap takes the pot from the canonical bootstrap state, or from an existing
  authoritative extraction of it. Initialising it to zero is never the rule. Zero is only what a
  snapshot without donations contains.
- (d) The pot enters the treasury as the boundary's LAST treasury addition, together with unclaimed
  gov deposits, after POOLREAP and after any treasury-withdrawal step. It is never an input to
  ratification, enactment or withdrawal affordability.
- (e) There are exactly three transitions of the pot: transaction application credits it, the
  boundary transfers and clears it, and bootstrap imports it. No consumer reconstructs the pot from
  historical transactions while the persisted value exists.
- (f) Overflow is a typed error, never a saturation.

**Hard prohibitions.**
- Carried from LV-5: nothing may special-case the observed amounts (500,000, 1,000,000) or epoch 305.
  The live result must come from the general rule.
- The boundary is not collapsed into an algebraically equal form such as `treasury += donations;
  treasury -= withdrawals`. The order of §1 is kept as written, even where the final number would match.
- No consumer reconstructs the pot from historical transactions (invariant (e)).
- The pot is not mirrored into the reduced UTxO checkpoint.

## 3. DESIGN (proposed; confirmed or changed by the obligations)

### 3.1 The owner

**`EpochState` (`crates/ade_ledger/src/state.rs`) gets `donation_pot: Coin`, next to `epoch_fees`.**

Cardano keeps the pot in `UTxOState`. Ade's `UTxOState` holds only the UTxO, and the epoch
accumulator has no `UTxOState` at all: it holds "the epoch state minus its UTxO". Putting the pot
there would give it one owner on the full path and none on the accumulator path.

`EpochState` is already the single owner of every other pot: treasury, reserves, and `epoch_fees`,
the analog of cardano's `utxosFees`. It is shared by both paths, through `LedgerState.epoch_state`
and `EpochAccumulator.epoch_state`. The donation pot follows the same pattern: credited per
transaction, consumed and reset at the boundary.

It does NOT go into `EpochAccumulator` as a separate field. The accumulator is one consumer of
`EpochState`, not the owner of the pots.

### 3.2 Per-transaction crediting

The per-transaction contribution is ONE pure function, for example `tx_donation_contribution(body,
phase2_valid) -> Coin`: the donation for a valid transaction, zero for an invalid one. Ade has two
paths that apply a transaction to an `EpochState`, the full ledger path and the accumulator fold. Each
calls that one function next to its `epoch_fees` credit, and neither decides validity on its own
terms:
- `crates/ade_ledger/src/phase.rs`: the full path; valid → donation, invalid → none.
- `crates/ade_ledger/src/epoch_accumulator.rs`, `apply_tx_scan`: the valid branch adds the donation;
  the `Phase2Invalid` branch is unchanged.

The scan reads body key 22. E3 lists every other place a transaction is applied.

### 3.3 The boundary

Every boundary construction that resets `epoch_fees` also moves `donation_pot` into the treasury and
resets it:
- the full boundary, `rules.rs` at the `EpochState` built at ≈1539;
- the native accumulator boundary;
- the seed boundary.

The reduced boundary (`rules.rs` ≈157) states it has no authority over any pot ("No rewards, pots, or
governance enactment"). It does not transfer the pot; E3 decides whether it clears the non-authoritative
copy, the way it resets `epoch_fees`.

In the full boundary the transfer is a named last step after the gov deposit returns, following §1.
Today Ade enacts no treasury withdrawal: a ratified `TreasuryWithdrawals` is the typed terminal
`UnsupportedRatifiedAction`, raised before any mutation. So the withdrawal step is absent rather than
reordered, and the pot cannot reach it.

### 3.4 Seed import

The seed's `UTxOState` is already read for `fees` and `govState` (`ledgerdb_state.rs` ≈2053/2268); the
donation pot (field 5) is currently skipped. It is decoded there and carried into the sealed
accumulator through every bootstrap path that builds an `EpochState` from a snapshot (E4).

### 3.5 Versions (E2 decides the exact set)

The fold now changes persisted treasury state for identical blocks. Expected bumps:
- `STORE_SEMANTICS_VERSION` 10 → 11, with v10 stores refused (not reinterpreted);
- `EPOCH_ACCUMULATOR_SCHEMA_VERSION` 5 → 6;
- snapshot framing 3 → 4.

The v2 fingerprint takes the field. The frozen v1 fingerprint does not.

## 4. SLICE-ENTRY OBLIGATIONS — answered by measurement before code

- **E1 — cardano source, verbatim.** ANSWERED, §4.1: the boundary order, and validity gating.
- **E2 — persistence review.** Every encoder, decoder, fingerprint and commitment that carries
  `EpochState` or the accumulator: `snapshot/epoch_state.rs`, `fingerprint.rs` v1/v2, the accumulator
  codec, the store-semantics surface lock, and any other store that persists `EpochState`. The version
  set follows from this list, not from §3.5's expectation.
- **E3 — every application and boundary path.** A grep-complete list of every place that credits
  `epoch_fees` per transaction and every place that resets it at a boundary, the reduced path
  included. Each one either gets the donation counterpart or is shown not to need it.
- **E4 — every bootstrap path.** Every constructor of a sealed `EpochState` from a snapshot: native
  first run, Mithril assembly, the reduced and full snapshot loaders, and the test-kit loader. Each
  reads the pot from the canonical state.
- **E5 — the seed's own value.** Read snapshot 6009's donation pot directly. LV-5 inferred 0; it
  becomes the expected fixture value only after it has been read.
- **E6 — the real fixtures.** The two epoch-305 transaction bodies (`24428858…5cd5`, `853764ba…58a5`),
  byte-exact from the chain, for CE-LV6-4.

### 4.1 E1 — cardano's EPOCH rule, verbatim

Source: `eras/conway/impl/src/Cardano/Ledger/Conway/Rules/Epoch.hs` at tag
`cardano-ledger-conway-1.17.0.0` (the release the registry already cites for node 11.0.1), function
`epochTransition`. Excerpts in order:

```haskell
  snapshots1 <-
    trans @(EraRule "SNAP" era) $ TRC (SnapEnv ledgerState0 curPParams, snapshots0, ())
  ...
  PoolreapState utxoState1 accountState1 dState1 pState2 <-
    trans @(EraRule "POOLREAP" era) $
      TRC (ShelleyPoolreapEnv vState, PoolreapState utxoState0 accountState0 dState0 pState1, eNo)
  ...
    (accountState2, dState2, EnactState {..}) =
      applyEnactedWithdrawals accountState1 dState1 rsEnactState
  ...
    (newUMap, unclaimed) =
      returnProposalDeposits allRemovedGovActions $
        dState2 ^. dsUnifiedL
  ...
    accountState3 =
      accountState2
        -- Move donations and unclaimed rewards from proposals to treasury:
        & asTreasuryL <>~ (utxoState0 ^. utxosDonationL <> fold unclaimed)
    utxoState2 =
      utxoState1
        & utxosDepositedL .~ totalObligation certState govState1
        -- Clear the donations field:
        & utxosDonationL .~ zero
```

and in `applyEnactedWithdrawals`:

```haskell
          & asTreasuryL %~ (<-> fold successfulWithdrawls)
```

The withdrawals reduce `accountState1` to `accountState2`. The donation pot is added only afterwards,
in `accountState3`, and is then cleared. The pot added is `utxoState0`'s, the value at boundary entry.
Withdrawal affordability is decided earlier, by RATIFY/ENACT inside the pulser. That check reads the
pulser's enact-state treasury, never the current donation pot.

**Validity gating**, from `Conway/Rules/Utxos.hs` at the same tag:

```haskell
    case tx ^. isValidTxL of
      IsValid True -> conwayEvalScriptsTxValid
      IsValid False -> babbageEvalScriptsTxInvalid
  ...
  -- end of conwayEvalScriptsTxValid:
  pure $! utxos' & utxosDonationL <>~ txBody ^. treasuryDonationTxBodyL
```

`babbageEvalScriptsTxInvalid` (`Babbage/Rules/Utxos.hs`, reused by Conway) returns

```haskell
    us {- (collInputs txb ⋪ utxo) ∪ collouts tx -}
      { utxosUtxo = UTxO (Map.union utxoKeep collouts) -- NEW to Babbage
      {- fees + collateralFees -}
      , utxosFees = fees <> Coin collateralFees -- NEW to Babbage
      , utxosStakeDistr = updateStakeDistribution pp (utxosStakeDistr us) (UTxO utxoDel) (UTxO collouts)
      }
```

It updates only the UTxO, the fees and the stake distribution. An invalid transaction's donation never
reaches the pot.

## 5. MECHANICAL ACCEPTANCE CRITERIA

| CE | criterion | witness |
|---|---|---|
| **CE-LV6-1** | A valid Conway transaction's donation is added to the pot, on the full path AND the accumulator path, with the same result | unit |
| **CE-LV6-2** | A phase-2-invalid transaction's donation adds nothing; its collateral contribution is unchanged | unit |
| **CE-LV6-3** | At every boundary path the whole pot goes to the treasury exactly once and the pot is 0 afterwards; a second boundary moves nothing | unit, every path in E3 |
| **CE-LV6-4** | **The real epoch-305 pair:** the two real bodies give a pot of 500,000 + 500,000 = 1,000,000; the 305→306 boundary adds exactly 1,000,000 to the treasury and resets the pot | integration, real bytes |
| **CE-LV6-5** | **Seed import reads the pot.** A snapshot fixture with a NON-ZERO donation pot seals an accumulator with that pot, and the next boundary moves it to the treasury. Snapshot 6009 seals with the value E5 read | integration |
| **CE-LV6-6** | Codec and fingerprint round-trip with the field; stores and records of the previous versions are refused | unit |
| **CE-LV6-7** | **Warm-restart and replay identity:** folding blocks with donations, persisting, reopening and continuing gives a canonical hash byte-identical to the uninterrupted fold, across a boundary | integration |
| **CE-LV6-8** | **Structural:** one crediting definition per path, both gated on validity; no `Coin(0)` initialisation of the donation pot in any bootstrap path; version lock entry recorded | `ci/ci_check_donation_pot.sh` (new) |
| **CE-LV6-9** | Every existing suite passes, mainnet corpus included; each changed number is named and matched to the reference | existing suites |
| **CE-LV6-10** | **Live, fresh follower-only bootstrap** of snapshot 6009, local docker `cardano-node-preprod`: after 305→306 the treasury is **1,959,103,174,719,172** and the reserves **12,979,123,112,128,607**, both equal to the reference; the pot is 0 after the boundary; the node follows on | live run plus existing captures |
| **CE-LV6-11** | Negative-tested | mutations below |
| **CE-LV6-12** | **Order: the pot never funds a withdrawal.** A constructed boundary holds a ratified treasury withdrawal that is affordable only if the current pot were credited first (treasury < amount ≤ treasury + pot). Ade does not use the pot to make it succeed. Today that means the typed terminal is raised before any mutation, with the pot and treasury untouched. Structurally, the pot is added in the named last step, after the gov deposit returns, and no ratification or enactment code reads `donation_pot`. **Carried forward:** when treasury-withdrawal enactment is implemented, this CE becomes the affordability test for that slice: withdrawal subtracted first, pot added after | unit and `ci/ci_check_donation_pot.sh` |
| **CE-LV6-13** | **No reconstruction:** outside the one per-transaction function, no production code adds a transaction's `donation` to any pot | `ci/ci_check_donation_pot.sh` |

### Required mutations

Each mutation is asserted applied, the named witness must fail, and the tree is restored byte-exact.

- The valid branch does not credit the donation (must fail CE-LV6-1 and CE-LV6-4).
- The invalid branch credits the donation (must fail CE-LV6-2).
- The boundary does not reset the pot (must fail CE-LV6-3).
- The boundary adds the pot twice (must fail CE-LV6-3 and CE-LV6-4).
- The seed import sets the pot to `Coin(0)` (must fail CE-LV6-5 and the gate).
- Only one of the two per-transaction paths credits (must fail CE-LV6-1).
- `STORE_SEMANTICS_VERSION` is left at 10 (must fail the lock gate).
- The boundary adds the pot before the gov deposit returns / withdrawal step, or ratification reads
  the pot (must fail CE-LV6-12).

## 6. EXPLICITLY NOT IN THIS SLICE

- **Transaction body key 21** (`current treasury value`). It is a validity assertion, not a pot
  movement. Ade's handling of it is not examined here.
- **The catch-up stop at an epoch boundary** (`LeadershipEpochNotSealed`), the 47 gate scripts that
  already fail, and the forge leg (CE-B12-10).

## 7. OPEN DECISIONS FOR THE OWNER

- **Decided 2026-10-18 by the owner:** the pot lives in `EpochState`, not mirrored into the reduced
  UTxO state. Versions: accumulator schema 5 → 6 and store semantics 10 → 11, with old stores refused
  rather than restamped, unless E2 finds otherwise. Seed import: the pot comes from authoritative
  snapshot state; 0 is the expected value for snapshot 6009 only once read directly (E5).
- **Adapted, for the owner to see:** CE-LV6-12. The owner asked that a withdrawal fundable only by
  donations is shown not to succeed. Ade has no withdrawal enactment today; a ratified withdrawal is a
  terminal before any mutation. So the CE pins the terminal-before-mutation behaviour plus the
  structural order, and becomes a full affordability test in the slice that adds withdrawals.

## 8. RESULTS

Not yet run.
