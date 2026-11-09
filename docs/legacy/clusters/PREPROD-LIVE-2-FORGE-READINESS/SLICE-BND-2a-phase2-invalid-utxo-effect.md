# SLICE BND-2a — one authoritative UTxO effect, and it knows about phase-2-invalid transactions

> **DOC BEFORE IMPL.** Entry evidence: `bnd2-oracle-extraction.md` (`ea15efc9`, the cardano-ledger
> rule) + `bnd2-existing-code-authority-census.md` (`3092ce51`, what Ade does today).
> This is **Slice 1 of 3**. It does NOT touch the accumulator's guards
> (`InvalidTxCarriesAuthorityEffect`, `InvalidTxCollateralNeedsUtxo`) and does NOT touch B12.

## The defect

Ade's UTxO/stake path has **no phase-2-invalid semantics at all**. `reduced_block_delta` and
`track_utxo` both apply every transaction's ordinary inputs and outputs regardless of validity, and
neither consumes collateral. The invalid set is decoded in `apply_block` and **reported only**.

Against cardano-ledger's `Phase2Invalid`, for block 130,350,133:

| | cardano-ledger | Ade today |
|---|---|---|
| `b9fede11…#1`, `#3` (ordinary inputs) | not spent | **SPENT** |
| the 4 ordinary outputs | not created | **CREATED** |
| `0326ab20…#1` (collateral input) | spent | **not spent** |

The reduced checkpoint is the **stake authority** — `sum_base_credential_stake()` over it produces the
boundary SNAP mark that seals frozen leadership (DC-EPOCH-32/33). A divergent UTxO set is therefore a
divergent leader schedule. Unlike the accumulator, which fail-closes and stops, this surface proceeds
silently with the wrong answer. That is a consensus-compatibility defect, not an optimisation.

## Invariant

**INV-BND-2a — one authoritative UTxO effect per transaction, validity-aware.** The UTxO effect of a
transaction is derived in exactly ONE place, from the canonical block plus the block's
`invalid_transactions` set. For a phase-2-**valid** transaction it is (ordinary inputs spent, ordinary
outputs produced). For a phase-2-**invalid** transaction it is (collateral inputs spent, collateral
return produced at index `len(ordinary outputs)` when present) and **nothing else** — ordinary inputs
survive and ordinary outputs are never created.

Registry: **DC-LEDGER-\<next\>** (derived). Related: DC-EPOCH-11, DC-EPOCH-32/33, DC-MEM-09.

## Design — extend the EXISTING single seam, do not add a second one

`extract_inputs_outputs_from_tx` is **already** the sole per-tx extractor, called by both
`reduced_advance::process_one_tx` and `rules::track_utxo`. Making *it* validity-aware gives both
consumers the rule from one derivation. Bolting `if invalid` onto each consumer is explicitly rejected:
that is how two implementations drift.

```rust
pub(crate) enum TxUtxoEffect {
    /// Phase-2 VALID: ordinary inputs spent, ordinary outputs produced at 0..n.
    Valid { inputs: Vec<TxIn>, outputs: Vec<TxOut> },
    /// Conway `Phase2Invalid`: collateral inputs spent; ordinary inputs and outputs NOT applied;
    /// the collateral return produced at index `len(ordinary outputs)` when present.
    Phase2Invalid { collateral_inputs: Vec<TxIn>, collateral_return: Option<(u64, TxOut)> },
}

pub(crate) fn extract_tx_utxo_effect(
    data: &[u8], offset: &mut usize, era: CardanoEra, phase2_invalid: bool,
) -> Result<TxUtxoEffect, LedgerError>
```

Three facts make this cheap rather than speculative:
- `ConwayTxBody` **already** decodes `collateral_inputs`, `collateral_return`, `total_collateral`. No
  new parsing, no new decoder.
- `locate_alonzo_plus_output_slices` **already** walks the whole body map to slice field 1; slicing
  field 16 in the same walk keeps the collateral return's **true bytes** (`TxOut::AlonzoPlus.raw`),
  so nothing is re-encoded.
- The collateral-return index is `len(ordinary outputs)`, matching cardano-ledger's
  `mkCollateralTxIn` (`mkTxIxPartial (length (txBody ^. outputsTxBodyL))`).

**Pre-Alonzo fails closed.** Shelley/Allegra/Mary have no `invalid_transactions` field and no
collateral, so `phase2_invalid = true` there is unrepresentable in a well-formed chain. If it is ever
asserted, return a typed error rather than silently treating the tx as valid.

`total_collateral` (field 17) is **not** read by this slice. Per the oracle it is a declared assertion,
not the source of truth; the fee-pot scalar is BND-2b's business.

## Mechanical acceptance criteria

| CE | Criterion | how it is judged |
|---|---|---|
| **CE-2a-1** | A phase-2-invalid tx spends its collateral inputs and NOT its ordinary inputs | unit over a constructed block |
| **CE-2a-2** | A phase-2-invalid tx produces no ordinary outputs; produces the collateral return at `len(outputs)` when present | unit, both present and absent |
| **CE-2a-3** | ONE derivation: both `reduced_block_delta` and `track_utxo` route through `extract_tx_utxo_effect`; neither branches on validity itself | CI gate (structural) |
| **CE-2a-4** | **Differential on the real block**: for 130,350,133 the derived effect equals the oracle — spent = {`0326ab20…#1`}, produced = ∅, `b9fede11…#1/#3` absent from spent | test against the durable block bytes |
| **CE-2a-5** | Pre-Alonzo + `phase2_invalid` ⇒ typed error, never silent | unit |
| **CE-2a-6** | A phase-2-VALID tx is byte-identical to today | regression: existing reduced/track_utxo tests unchanged and green |
| **CE-2a-7** | **Store semantics bumped**; an old artifact is refused, not reinterpreted | see below |
| **CE-2a-8** | Negative-tested | mutations below |

### Required mutations
apply ordinary inputs for an invalid tx (must fail CE-2a-1/4) · produce ordinary outputs for an invalid
tx (CE-2a-2/4) · place the collateral return at index 0 (CE-2a-2) · skip collateral consumption
(CE-2a-1/4) · branch on validity inside one consumer instead of the shared extractor (CE-2a-3) ·
treat pre-Alonzo invalid as valid (CE-2a-5).

## STORE SEMANTICS — **v3 → v4, a real bump**

This changes the authoritative interpretation of blocks used to construct **persisted** reduced
UTxO/stake state. The existing constant's own precedent is this exact test:

> *"Replaying the same blocks under this binary therefore produces different protocol params … so a
> store written by an earlier binary is not replay-equivalent."* — the v3 rationale.

Replaying the same blocks under the `Phase2Invalid` rule produces a different reduced UTxO, hence a
different stake view. So the marker must move. `STORE_SEMANTICS_VERSION` is one **global** marker
carried by `chain.db`, `EpochAccumulatorStore` **and** `ReducedUtxoCheckpoint`, and
`check_store_semantics_version` already fail-closes on mismatch — so the mechanism exists and needs no
new machinery.

**Consequence, stated plainly because it is expensive:** every existing v3 store is refused at open
with a typed re-bootstrap terminal — including `~/.cardano-live1/ade-preprod-s7` and
`FROZEN-b6-census-s7`. Live validation of this slice requires a re-bootstrap from the Mithril
snapshot. That is the correct outcome: an artifact built under the wrong rule must not be silently
reinterpreted under the right one.

**This also settles the contamination question.** Whether the current store actually crossed a
phase-2-invalid transaction stops being load-bearing for correctness — the bump forces reconstruction
either way. Measuring it remains useful *operationally* (it tells us whether the observed stake view
was already wrong), and is cheap: scan the checkpoint's replay interval for blocks with a non-empty
`invalid_transactions` field, reusing the existing decoder. Recorded as operational evidence, NOT as a
gate on this slice.

## Colour law

```
BLUE   extract_tx_utxo_effect + the Phase2Invalid selection   | authority (ade_ledger)
BLUE   reduced_block_delta / track_utxo consuming it          | authority
GREEN  the invalid set decoded from the canonical block       | derived from canonical bytes only
```

No wall-clock, no I/O, no peer input. The invalid set comes from the block's own
`invalid_transactions` field — canonical data, already durable.

## Explicitly NOT in this slice
- The accumulator's fee-pot scalar `collAdaBalance` (BND-2b) and the resolver contract that feeds it.
- Removing or narrowing `InvalidTxCarriesAuthorityEffect` / `InvalidTxCollateralNeedsUtxo` (BND-2c).
  The accumulator continues to fail closed throughout this slice — deliberately, so the silent
  authority becomes correct while the halting one stays honest.
- Unpinning the accumulator cursor. This slice does not let it cross 130,350,133.
- Any B12 / DC-NODE-15 change.
- Measuring historical drift of the existing store (operational, above).
