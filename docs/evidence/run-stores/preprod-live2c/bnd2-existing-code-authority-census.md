# BND-2 EXISTING-CODE AUTHORITY CENSUS — who in Ade already knows about phase-2-invalid txs?

> **Findings-first, by reading existing code.** No harness was built and nothing was measured on a
> live venue: Ade already follows Conway blocks and maintains both derived stores, so the behaviour
> exists in the tree. Companion to `bnd2-oracle-extraction.md` (the cardano-ledger rule).
> Nothing was changed.

## The question
`Phase2Invalid` in cardano-ledger removes the **collateral** inputs, creates `collOuts`, adds
`collAdaBalance` to fees, and adjusts instant stake — leaving the regular inputs unspent and the
regular outputs uncreated. Which Ade surfaces implement that, and which do not?

## The census

| surface | consults the invalid set? | what it does with block 130,350,133 |
|---|---|---|
| `epoch_accumulator::scan_block_tx_effects` (fees/withdrawals) | **YES** — `decode_invalid_tx_indices_canonical`, then fail-closed | **halts** — `InvalidTxCarriesAuthorityEffect` (the guard under study) |
| `epoch_accumulator::apply_one_tx_governance` (fields 19/20) | **YES** — same set, fail-closed | not reached (no votes/proposals on this tx) |
| `reduced_advance::reduced_block_delta` (**the stake authority**) | **NO** — the file never mentions `invalid` | spends the regular inputs, creates the 4 outputs, ignores the collateral input |
| `rules::track_utxo` (the full UTxO tracker) | **NO** — takes no invalid set; its body never reads `block.invalid_txs` | same |
| `rules::apply_block` → `BlockApplyResult.invalid_tx_indices` | decodes it | **reports only** — computed alongside/after the UTxO work and never fed into it |
| `plutus_eval` | has `decode_invalid_tx_indices`; its own header notes `is_valid` is "defaulted to `true` (the typical mainnet case)" | — |

Both UTxO-shaped paths receive the whole `ShelleyBlock`, and `invalid_txs: Option<Vec<u8>>` is a field
**on that very struct**. The information is present at both call sites and is simply not consulted.
This is not a plumbing gap; it is a missing rule.

## What that means for block 130,350,133

| | cardano-ledger (`Phase2Invalid`) | Ade's reduced checkpoint / `track_utxo` |
|---|---|---|
| `b9fede11…#1`, `b9fede11…#3` (regular inputs) | **not spent** | **SPENT** |
| the 4 regular outputs | **not created** | **CREATED** |
| `0326ab20…#1` (collateral input) | **spent** | **not spent** |
| fee pot | `+ collAdaBalance` | n/a for these surfaces |

Three divergences, in the opposite direction to the accumulator's: where the accumulator **fail-closes
and stops**, the UTxO/stake path **silently proceeds with the wrong answer**.

## Why this outranks the accumulator's collateral scalar

The reduced checkpoint is the **stake authority**: `sum_base_credential_stake()` over it produces the
boundary SNAP mark, which seals frozen leadership (DC-EPOCH-32/33). A UTxO set that diverges from
Cardano's yields a divergent stake distribution, hence a divergent leader schedule — and unlike the
accumulator's halt, nothing announces it.

So the earlier framing of BND-2 — "the accumulator needs one collateral scalar from the UTxO
authority" — is **necessary but not sufficient**. Supplying that scalar while the UTxO authority still
applies invalid transactions as ordinary traffic would fix the surface that noticed and leave the
surfaces that did not.

**Restated:** Ade has no phase-2-invalid transaction semantics anywhere in the UTxO/stake path. One
surface fail-closes on the gap; the others diverge quietly. The accumulator's guard is, in effect, the
only thing that ever noticed.

## Scope and severity — stated carefully

- The divergence is **per phase-2-invalid transaction**, which is rare but not exceptional traffic.
  Each occurrence perturbs the UTxO set permanently (wrong entries removed, wrong entries created).
- `track_utxo` is gated on `current_state.track_utxo`, which the live admission path runs with
  **false** (MEM-OPT-UTXO-DISK), so the *full* tracker's exposure on the live path is limited. The
  **reduced checkpoint is not so gated** — it is advanced on the live follow path and is the stake
  authority. That is the live-reachable one.
- This census establishes *what the code does*. It does **not** measure how far the live stake view
  has actually drifted, and no such claim is made here.

## What this changes about the ordering

The BND-2 slice should establish phase-2-invalid semantics **once**, for the UTxO/stake path and the
accumulator together, rather than adding a collateral scalar to the accumulator first:

1. teach the UTxO-effect derivation the `Phase2Invalid` rule (skip regular inputs/outputs; consume
   collateral inputs; create `collOuts`) — the invalid set is already decodable at both call sites;
2. derive the accumulator's fee-pot delta `collAdaBalance` from the same resolution, which is the
   scalar the earlier extraction identified;
3. only then revisit the two accumulator guards.

Doing (2) before (1) fixes the halting surface and leaves the silent one.

## NOT DONE HERE
No code changed. No guard touched. No live run. `InvalidTxCarriesAuthorityEffect`,
`InvalidTxCollateralNeedsUtxo`, the reduced-checkpoint path and B12 are all exactly as they were.
Whether the live reduced checkpoint has measurably drifted is an open question this census
deliberately does not answer.
