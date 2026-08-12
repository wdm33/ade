# SLICE BND-2b — the UTxO authority resolves; the accumulator consumes one scalar

> **DOC BEFORE IMPL.** Slice 2 of 3. Entry: `bnd2-oracle-extraction.md` (`ea15efc9`) and BND-2a
> (`d619ebda`, closed `1ac85759`). This slice does **not** retire `InvalidTxCarriesAuthorityEffect`
> or `InvalidTxCollateralNeedsUtxo` — that is BND-2c — and does not touch B12.

## Scope discipline

This is a **narrow completion of a known Conway rule using the existing UTxO authority**. It adds no
subsystem, no parser, no second UTxO owner, and no general framework. If the implementation starts to
grow one of those, the slice is wrong, not the constraint.

## What is already true (checked, not assumed)

`ReducedUtxoCheckpoint` already exposes the exact lookup the rule needs:

```rust
pub fn get(&self, txin: &TxIn) -> Result<Option<(Coin, ReducedStakeRef)>, ReducedCheckpointError>
```

and it retains **every** UTxO entry, not a stake-bearing subset: `ReducedStakeRef` is
`Base(StakeCredential) | NonContributing`, and `reduced_block_delta` produces an entry for every
output via `reduce_txout`. "Reduced" means the full `TxOut` (datum, script, multi-asset) is dropped —
the `TxIn → Coin` mapping is complete.

That matters twice over: the checkpoint can resolve an arbitrary collateral input, **and** it keeps
ADA `Coin` specifically, which is precisely what the rule needs —
`collAdaBalance` is defined over `sumAllCoin` and `coinTxOutL`, i.e. ADA only. The reduced form is
sufficient by construction rather than by luck.

## The rule being completed

```haskell
collAdaBalance txBody utxoCollateral = toDeltaCoin $
  case txBody ^. collateralReturnTxBodyL of
    SNothing    -> colbal
    SJust txOut -> colbal <-> (txOut ^. coinTxOutL)
  where colbal = sumAllCoin utxoCollateral
```

Decomposed across the authority boundary:

```
Σ value(collateral inputs)      <- UTxO AUTHORITY resolves   (ReducedUtxoCheckpoint::get)
− collateral_return.coin        <- already IN THE CANONICAL BLOCK (no resolution)
= collAdaBalance                -> the accumulator CONSUMES one Coin
```

## Invariant

**INV-BND-2b — the accumulator consumes a resolved scalar, it does not own a UTxO.** The fee
contribution of a phase-2-invalid transaction is `collAdaBalance`, computed from collateral-input
values supplied by the UTxO authority and the collateral return read from the block. The accumulator
never holds, indexes, queries or reconstructs a UTxO map, and `total_collateral` (field 17) is never
the source of the value — per the reference it is a declared assertion the UTXO rule checks.

## Design — a one-method seam, pointing inward

`ade_ledger` is BLUE and must not depend on `ade_runtime`, so the seam is a trait defined where the
rule lives and implemented where the storage lives:

```rust
// ade_ledger (BLUE) — the rule states what it needs.
pub trait CollateralValueResolver {
    /// Resolved ADA value of one collateral input, or None if the authority does not hold it.
    fn collateral_value(&self, txin: &TxIn) -> Option<Coin>;
}
```

- implemented in `ade_runtime` over the existing `ReducedUtxoCheckpoint::get` — no new storage, no
  new query path;
- threaded to the accumulator's block scan through `SelectedBlockCtx`, which already carries the
  per-block context (`era`, `block_epoch`, `boundary_mark`, …);
- **absent resolver ⇒ unchanged behaviour.** Callers that do not supply one keep today's fail-closed
  exactly, so every existing path is byte-identical until BND-2c chooses to rely on it.

**Fail-closed stays fail-closed.** An unresolvable collateral input is `None`, and `None` must remain
a refusal — never a zero, never a skipped contribution. Guessing a value here would put a wrong number
into the fee pot, which is the failure mode the guard has been protecting against all along.

## Mechanical acceptance criteria

| CE | Criterion | how it is judged |
|---|---|---|
| **CE-2b-1** | `collAdaBalance` == Σ resolved collateral values when no collateral return | unit |
| **CE-2b-2** | With a collateral return, the return's coin is subtracted | unit |
| **CE-2b-3** | An unresolvable collateral input ⇒ typed refusal, never 0 and never skipped | unit (negative) |
| **CE-2b-4** | `total_collateral` is never read as the value source; a body declaring a WRONG field 17 does not change the computed scalar | unit + gate |
| **CE-2b-5** | The accumulator gains no UTxO ownership: no map, no index, no `get`-by-TxIn API on the accumulator itself | CI gate (structural) |
| **CE-2b-6** | Absent resolver ⇒ byte-identical to today, including the existing guards firing | regression |
| **CE-2b-7** | Real-block value: for 130,350,133 the scalar equals the full value of `0326ab20…#1` (no return declared) | differential once the resolver is wired |
| **CE-2b-8** | Negative-tested | mutations below |

### Required mutations
subtract the return when absent · add instead of subtract the return · read `total_collateral` as the
value · treat an unresolved input as 0 · give the accumulator its own UTxO map (must fail CE-2b-5).

## Explicitly NOT in this slice
- Retiring or narrowing `InvalidTxCarriesAuthorityEffect` / `InvalidTxCollateralNeedsUtxo` (**BND-2c**).
  The accumulator continues to fail closed and its cursor stays pinned at 130,350,114.
- Any B12 / DC-NODE-15 change.
- Any store-semantics change: this slice computes a value and changes no persisted interpretation, so
  **v4 stands**. (BND-2c, which changes what the accumulator *does* with it, must re-ask that.)
