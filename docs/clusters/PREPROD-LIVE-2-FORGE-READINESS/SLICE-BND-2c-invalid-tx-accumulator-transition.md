# SLICE BND-2c — the accumulator's exact phase-2-invalid transition

> **DOC BEFORE IMPL.** Slice 3 of 3. Entry: `bnd2-oracle-extraction.md` (`ea15efc9`), BND-2a
> (`d619ebda`, closed `1ac85759`), BND-2b (`89dd1aac`). This is the slice that unpins the cursor.

## The shape, and the shape it must NOT be

```
Phase2Invalid
  → ignore normal body authority effects
  → resolve collateral through the EXISTING UTxO authority
  → collAdaBalance
  → apply the exact invalid-tx accumulator effects
  → advance the cursor
```

**Not** `guard failed before → resolver exists now → delete guard`. The guards disappear as a
*consequence* of the transition becoming correct, never as its enabling step.

## Scope rail — this is NOT a second transaction-validity engine

`total_collateral` equality is a **Cardano ledger validity assertion** (`IncorrectTotalCollateralField`),
enforced by the UTXO rule before a block is ever selected. Ade's own ledger-verdict authority
establishes block validity upstream of the accumulator. **The accumulator reproduces the
accumulator-relevant state transition of an already-valid canonical block; it does not re-adjudicate
one.** So field 17 stays unread here, exactly as in BND-2b — not because it is hard, but because
re-validating it would be the accumulator growing a second validity engine on the strength of one
investigation.

## Invariant

**INV-BND-2c — a phase-2-invalid transaction contributes its consumed collateral and nothing else.**
For a tx in the block's `invalid_transactions` set the accumulator applies exactly one effect: the fee
pot gains `collAdaBalance`. Its certificates, withdrawals, voting procedures and proposal procedures
contribute **nothing** to any accumulator authority state — not because they are refused, but because
Cardano discards them. When no resolver is supplied the transition refuses (the value is genuinely
unknowable), and that refusal is about the MISSING VALUATION, never about the tx "carrying effects".

Registry: **DC-LEDGER-03** (derived). Related: DC-LEDGER-01, DC-LEDGER-02, DC-EPOCH-39.

## What retires, and precisely why

| today | after | why it is legitimate |
|---|---|---|
| `InvalidTxCarriesAuthorityEffect` (certs/withdrawals limb, `apply_tx_scan`) | **gone** | the transition now DISCARDS those effects, which is what Cardano does. The error existed because discarding was unimplemented, not because the tx was wrong. |
| `InvalidTxCarriesAuthorityEffect` (fields 19/20, `apply_one_tx_governance`) | **gone** | same, and unconditional: discarding governance effects needs no resolver. |
| `InvalidTxCollateralNeedsUtxo` (field 17 absent) | **narrowed**, not deleted | with a resolver the value comes from the UTxO authority. WITHOUT one it is genuinely uncomputable, so refusing stays correct — the guard shrinks to exactly the case where the information does not exist. |

That middle column is the retirement criterion in force: `InvalidTxCarriesAuthorityEffect` disappears
because the new transition proves those effects are discarded, **not** because the transaction now
gets past a test.

## Design — alongside, not inside, the context

`SelectedBlockCtx` is an owned struct built at many call sites; threading a `&dyn` through it would
add a lifetime to all of them. Instead the resolver rides beside it:

```rust
pub fn apply_selected_block(prior, block_bytes, ctx)                  // delegates with None
pub fn apply_selected_block_with_resolver(prior, block_bytes, ctx,
        resolver: Option<&dyn CollateralValueResolver>)
```

- every existing call site is untouched and byte-identical (`None` ⇒ today's behaviour);
- `ade_runtime` threads `Option<&dyn …>` through `advance_accumulator_over_chaindb` /
  `advance_accumulator_over_block`;
- `node_lifecycle::advance_ledger_state_to_durable_tip_memo` already holds BOTH the reduced
  checkpoint and the accumulator, so it supplies the live resolver with no new ownership.

**Anti-drift note.** The accumulator's own `TxScan` walk gains fields 13/16, while
`rules::extract_tx_utxo_effect` reads the same fields via `decode_conway_tx_body`. Two readers of
field 13 now exist. That is tolerated (they already coexist for fields 2/4/5/17) but **must be
pinned**: CE-2c-8 asserts the two agree on the real block, so a divergence is a test failure rather
than a silent disagreement between the fee path and the UTxO path.

## Mechanical acceptance criteria — the live bar

| CE | Criterion | how it is judged |
|---|---|---|
| **CE-2c-1** | At 130,350,133 the REAL accumulator walk resolves the collateral input through the existing reduced checkpoint | live |
| **CE-2c-2** | The resolved `collAdaBalance` feeds the SAME accumulator transition that handles the block — no diagnostic-only calculation | code path + gate: the value is consumed by `apply_tx_scan`, not logged beside it |
| **CE-2c-3** | tx0's withdrawals / certs / votes / proposals contribute NOTHING to accumulator authority state | unit: an invalid tx carrying all four leaves cert/gov/withdrawal state identical to a block without it |
| **CE-2c-4** | The collateral-derived fee contribution is applied exactly as the Cardano rule requires | unit + the real-block value |
| **CE-2c-5** | The cursor advances beyond 130,350,114 and THROUGH 130,350,133 | live |
| **CE-2c-6** | Warm replay/restart reproduces the resulting accumulator state byte-identically | live restart + determinism unit |
| **CE-2c-7** | Absent resolver ⇒ typed refusal about the missing valuation; never a fabricated 0 | unit |
| **CE-2c-8** | The accumulator's field-13 reading agrees with `extract_tx_utxo_effect`'s on the real block | unit (anti-drift) |
| **CE-2c-9** | Negative-tested | mutations below |

### Required mutations
apply the invalid tx's withdrawals (must fail CE-2c-3) · apply its certs · apply its votes/proposals ·
add the declared `fee` instead of `collAdaBalance` (CE-2c-4) · treat an absent resolver as 0
(CE-2c-7) · compute `collAdaBalance` but discard it, leaving the fee unchanged (CE-2c-2).

## Store semantics — re-asked, and the answer is a BUMP

BND-2b was capability-only so v4 stood. **This slice changes what the accumulator DOES**: replaying
the same canonical blocks now advances the cursor through a block that previously pinned it, and adds
a collateral-derived fee contribution that previously did not exist. Any accumulator persisted under
v4 is therefore not replay-equivalent under this binary.

⇒ **STORE_SEMANTICS_VERSION 4 → 5**, with the same fail-closed refusal and re-bootstrap the v3→v4
move proved. The v4 store built for BND-2a becomes historical evidence on the same terms
`FROZEN-b6-census-s7` did.

## Explicitly NOT in this slice
- Any B12 / DC-NODE-15 change. The `+1` is proven benign, but B12 moves only once the accumulator is
  demonstrably healthy — which is what this slice sets out to make true, not what it may assume.
- Any re-validation of `total_collateral`, or any other ledger-validity assertion, inside the
  accumulator.
- Leadership/forge behaviour. A healthy cursor is a precondition for reconsidering B12, not a licence.
