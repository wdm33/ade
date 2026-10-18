# SLICE LV-6 — a treasury donation is held in one pot and moved to the treasury once, at the boundary

**Status:** DRAFT 2026-10-18. No code yet. The entry obligations (§4) are answered before any code.

**Entry state:** `a011ec94`. Cause established by LV-5
(`docs/evidence/run-stores/preprod-live2c/lv5-treasury-divergence-census.md`). Epoch 305 carried two
treasury donations of 500,000 each, in valid transactions. Cardano collected them in its donation pot
and added the pot to the treasury at 305→306. Ade keeps no donation pot, so its treasury came out
1,000,000 short. Every other pot and every reward balance agreed.

A small derived cardano-compatibility slice: one missing Conway state variable and the transition
that clears it.

---

## 1. THE CONTRACT (cardano, to be quoted verbatim at entry, E1)

- A **valid** Conway transaction adds its `treasury donation` (body key 22) to the ledger's
  donation pot (`utxosDonation`).
- A **phase-2-invalid** transaction contributes only its collateral, never its donation.
- At the epoch boundary, the **whole** pot is added to the treasury and the pot becomes zero.

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
- (d) Overflow is a typed error, never a saturation.

**Hard prohibition, carried from LV-5.** Nothing in this slice may special-case the observed amounts
(500,000, 1,000,000) or epoch 305. The live result must come from the general rule.

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

The donation pot is credited next to `epoch_fees`, under the same validity gating:
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

The reduced boundary (`rules.rs` ≈157) is checked under E3. Where in the boundary the pot moves
(before or after rewards, POOLREAP, gov refunds and enactment) is copied from cardano per E1.

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

- **E1 — cardano source, verbatim**, from the cardano-ledger release matching node 11.0.1:
  - where the Conway EPOCH rule adds `utxosDonation` to the treasury and where it resets it;
  - the order relative to reward application, POOLREAP, gov-deposit refunds and treasury-withdrawal
    enactment;
  - that a phase-2-invalid transaction contributes no donation.

  If the order matters for anything Ade computes, for example affordability of a treasury withdrawal,
  the design follows it exactly.
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

### Required mutations

Each mutation is asserted applied, the named witness must fail, and the tree is restored byte-exact.

- The valid branch does not credit the donation (must fail CE-LV6-1 and CE-LV6-4).
- The invalid branch credits the donation (must fail CE-LV6-2).
- The boundary does not reset the pot (must fail CE-LV6-3).
- The boundary adds the pot twice (must fail CE-LV6-3 and CE-LV6-4).
- The seed import sets the pot to `Coin(0)` (must fail CE-LV6-5 and the gate).
- Only one of the two per-transaction paths credits (must fail CE-LV6-1).
- `STORE_SEMANTICS_VERSION` is left at 10 (must fail the lock gate).

## 6. EXPLICITLY NOT IN THIS SLICE

- **Transaction body key 21** (`current treasury value`). It is a validity assertion, not a pot
  movement. Ade's handling of it is not examined here.
- **The catch-up stop at an epoch boundary** (`LeadershipEpochNotSealed`), the 47 gate scripts that
  already fail, and the forge leg (CE-B12-10).

## 7. OPEN DECISIONS FOR THE OWNER

- **The owner of the pot (§3.1).** Proposed: `EpochState`, beside `epoch_fees`. The alternative,
  cardano's placement in `UTxOState`, would leave the accumulator path without the pot. If E3 or E4
  find a path that holds an `EpochState` yet must not carry the pot, this comes back to the owner.

## 8. RESULTS

Not yet run.
