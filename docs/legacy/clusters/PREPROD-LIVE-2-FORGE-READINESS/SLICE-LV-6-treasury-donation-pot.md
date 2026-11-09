# SLICE LV-6 — a treasury donation is held in one pot and moved to the treasury once, at the boundary

**Status:** DRAFT 2026-10-18; owner-approved design (§7). Entry obligations E1–E6 answered (§4.1,
§4.2). Implementation next.

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
phase2_valid) -> Coin`: the donation for a valid transaction, zero for an invalid one. The
accumulator fold calls it next to its `epoch_fees` credit and passes the block's phase-2 validity set.
(The draft assumed a second path, the full ledger; E3 found it credits no valid-transaction pot effects,
so there is one caller.)
- `crates/ade_ledger/src/epoch_accumulator.rs`, `apply_tx_scan`: the contribution is computed with the
  validity gate and summed; `apply_within_epoch` credits the sum to `epoch_state.donation_pot`.
- `crates/ade_ledger/src/phase.rs` (phase-2 failure) adds no donation, as cardano's invalid branch.

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

### 4.2 E2–E6 — results, 2026-10-18

**E2 — persistence surface.** Every place that saves, fingerprints or commits to the state:

| surface | carries | change |
|---|---|---|
| `snapshot/epoch_state.rs` `encode/decode_epoch_state` | `EpochState`, array(7) | array(8) with `donation_pot` last; snapshot framing `SCHEMA_VERSION` 3 → 4 |
| accumulator codec (`epoch_accumulator.rs`) | embeds `encode_epoch_state` | `EPOCH_ACCUMULATOR_SCHEMA_VERSION` 5 → 6 |
| `fingerprint.rs` `fingerprint_epoch` | shared by v1 AND v2 | v1 gets a frozen copy (`fingerprint_epoch_v1`), as LV-4 did for snapshots; v2 adds `donation_pot` |
| `ledgerdb_state.rs` `commit_native_nonutxo_state` | every certified seed value | binds `donation_pot`; tag `…-v12` → `…-v13` |
| store-semantics lock | the files above | `STORE_SEMANTICS_VERSION` 10 → 11, new lock entry |

The reduced UTxO checkpoint persists no `EpochState`; nothing is mirrored there. **The version set
matches §3.5, plus the seed commitment v13.**

**E3 — application and boundary paths.**
- **Per transaction:** *corrected during implementation.* Only ONE production site credits transaction
  effects to the pots: `epoch_accumulator.rs` `apply_tx_scan` / `apply_within_epoch`, the accumulator
  fold. `phase.rs`'s `apply_phase_2_failure` is called only from its own tests, and the full ledger path
  credits no valid-transaction fees at all. So the accumulator fold is the single
  transaction-application authority, and it is the one caller of the contribution function.
- **At the boundary:**
  - `rules.rs` ≈1539 is the full boundary, which the accumulator's native and seed crossings also go
    through. It transfers the pot in the named last step.
  - `rules.rs` ≈157 is the reduced boundary, documented as having no pot authority. It clears the
    non-authoritative copy as it does `epoch_fees`, and transfers nothing.
- `hfc.rs` builds a fresh `EpochState` at era translation with zero pots. A pre-Conway era has no
  donations, so 0 is the translated value there, not a bootstrap default.

**E4 — bootstrap paths.** Each takes the pot from canonical state:
- `ledgerdb_state.rs` decodes `UTxOState` (≈2053, field 5 currently skipped) into
  `NativeSnapshotNonUtxoState`, which gains `donation_pot`.
- `mithril_native_assembly.rs` ≈357 carries it into the sealed `EpochState`.
- `ade_testkit` `snapshot_loader.rs` reads it from each era's own `UTxOState` where the era has one,
  and is 0 for eras before Conway.
- The `epoch.rs` and test literals are fixtures.

**E5 — snapshot 6009, read directly** (`preprod-snapshot-6009/state`, `UTxOState` field 5): donation
pot **0**. Treasury 1,945,191,497,759,078 and reserves 12,993,813,079,206,744 match the sealed seed.
0 is now a measured fixture value.

**E6 — real fixtures**, cut byte-exact from the reference ImmutableDB:
- block at slot 130,271,804: 3,150 bytes, 2 transactions, none invalid;
- block at slot 130,404,902: 5,708 bytes, 3 transactions, none invalid.

Each carries one of the two donations at transaction index 0.

## 5. MECHANICAL ACCEPTANCE CRITERIA

| CE | criterion | witness |
|---|---|---|
| **CE-LV6-1** | A valid Conway transaction's donation is added to the pot by the accumulator fold, the single transaction-application authority *(amended: E3 found no second path; see §4.2)* | unit |
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
- The fold drops the scan's donation total (must fail CE-LV6-4 and CE-LV6-7). *(Replaces "only one of two paths credits": there is one path.)*
- The pot is added before POOLREAP and the gov deposit returns, with the same final number (must fail the gate's order check: the algebraic-equivalence case).
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

Run 2026-10-18 on the uncommitted implementation, before the live leg.

**Suites (CE-LV6-1..9, 12, 13).** `ade_ledger` 1093, `ade_node` 645, `ade_runtime` 582: all pass. The
new tests:

- CE-LV6-1, CE-LV6-2: `lv6_a_valid_txs_donation_is_credited_and_an_invalid_txs_is_not`.
- CE-LV6-3: `lv6_the_boundary_moves_the_whole_pot_once_and_zeroes_it` and
  `lv6_donations_and_unclaimed_deposits_both_reach_the_treasury`.
- CE-LV6-4: `lv6_the_real_epoch_305_blocks_carry_500_000_each` and
  `lv6_the_real_pair_folds_into_the_pot_and_the_boundary_moves_it`, using the two real blocks at slots
  130,271,804 and 130,404,902, committed as fixtures.
- CE-LV6-5: `a_seed_with_donations_imports_its_donation_pot`, `the_seed_commitment_binds_the_donation_pot`
  and `a_utxo_state_without_a_donation_pot_is_refused`; Mithril assembly carries a non-zero pot
  (7,654,321) in its fixture test.
- CE-LV6-6: the accumulator codec refuses schema 5 (`codec_rejects_pre_lv6_store_rebootstrap_required`),
  framing refuses schema 3, and the v2 fingerprint's epoch component now changes with the pot.
- CE-LV6-7: `lv6_a_donation_pot_survives_a_warm_restart_byte_identically` (`ade_runtime`): the
  straight and reopened stores encode byte-identically.
- CE-LV6-12: `lv6_the_pot_never_funds_a_treasury_withdrawal`, plus the gate's order check.
- An overflowing pot is the typed terminal `TreasuryOverflow`
  (`lv6_a_pot_that_would_overflow_the_treasury_is_a_terminal`).

`ci/ci_check_donation_pot.sh` (CE-LV6-8, 12, 13) passes, as does the store-semantics lock (v11 entry
recorded) and the registry gates.

**Corpus (CE-LV6-9).** The epoch-oracle comparison ran over the corpus: 26 pass, 1 fails. The failure
was there before LV-6, with the same numbers. Compared with the pre-LV-6 baseline run, no number moved;
the preprod treasury delta is unchanged at 260,984,218,562. The one byte pin that encodes an
`EpochState` (`rvbp_live_path_reduced_dispatch.rs`) was re-anchored to the new field, with an
`assert_ne` that the old bytes no longer match.

**Gate sweep.** All `ci/ci_check_*.sh` scripts run: 46 fail, none of them new. Every one was already
failing before LV-6.

**Mutations (CE-LV6-11).** Each mutation was asserted applied, the tree restored byte-exact after each.

| mutation | caught by |
|---|---|
| M1 valid branch does not credit | the CE-LV6-1/4 unit tests, the warm-restart test, gate (a) |
| M2 invalid branch credits | the CE-LV6-2 unit test, gate (b) |
| M3 boundary does not reset the pot | the CE-LV6-3/4 unit tests, gate (d) "not zeroed" |
| M4 boundary adds the pot twice | the CE-LV6-3/4 unit tests, gate (d) "exactly once, unscaled" |
| M5 seed import sets `Coin(0)` | the two seed-import tests, gate (e) |
| M6 fold drops the scan's donation total | the real-pair test and the warm-restart test |
| M7 store semantics left at 10 | the lock gate (C), gate (f) |
| M8 pot added before POOLREAP, same final number | gate (d) "added before POOLREAP"; also the overflow-terminal test |
| M9 ratification reads the pot | gate (d) only (no test can see an unused read) |
| M10 a UTxOState without the pot is accepted | the refusal test |

The first run exposed a defect in the gate itself: under `set -euo pipefail`, a step that was not
found made a `grep | head | cut` assignment fail, and the script exited 1 with no message. M4 and M8
failed the gate, but for no named reason. The assignments are now guarded, and the gate asserts that
the boundary reads the pot exactly once. M4, M7 and M8 were re-run against the fixed gate with the
results above.

**Live (CE-LV6-10): met.** A fresh v11 bootstrap of snapshot 6009 against the local docker
`cardano-node-preprod`, with one warm restart at the known 306→307 header stop. After 305→306:

- treasury 1,959,103,174,719,172 and reserves 12,979,123,112,128,607, both equal to the reference;
- the donation pot is 0.

Before the boundary, the pot held 500,000, the first donation, at the first stop. The node then crossed
306→307 and followed on. Evidence:
`docs/evidence/run-stores/preprod-live2c/lv6-ce10-v11-live-donation-pot.md`. DC-EPOCH-45 is enforced.
