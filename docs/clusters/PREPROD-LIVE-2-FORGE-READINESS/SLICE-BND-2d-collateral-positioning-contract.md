# SLICE BND-2d — the positioning contract for a collateral valuation

> **DOC BEFORE IMPL.** Entry: `c3e73b1a`. BND-2a (`d619ebda`/`1ac85759`), BND-2b (`89dd1aac`) and
> BND-2c (`390c8191`) are CLOSED as semantics. BND-2c's **live bar failed** for one narrow reason —
> the two derived stores are not at the same chain point when resolution is requested
> (`docs/evidence/run-stores/preprod-live2c/bnd2c-v5-live-FAILED-unresolved-collateral.txt`).
> Nothing about the Conway rule, the scalar or the transition is re-opened here.

---

## 1. THE CONTRACT — answered before any design

### C1 — where a collateral value is authoritative

cardano-ledger resolves `utxoDel = extractKeys (unUTxO utxo) (txBody ^. collateralInputsTxBodyL)`
against the UTxO threaded into *this* transaction: the canonical prefix ending at `parent(B)`, plus
the effects of transactions `0..i-1` of `B`. That is the literal point.

A stronger fact makes the literal point tractable:

> **`TxIn -> Coin` is IMMUTABLE over the chain.** A `TxIn` is
> `(blake2b_256(tx_body_bytes), output_index)`. The output it names is created exactly once, by
> those same bytes, carrying one value. Blocks only CREATE and REMOVE entries; none rewrites the
> coin bound to an existing key. (Two identical bodies would hash equal and carry equal outputs, so
> even that degenerate case agrees.)

Therefore:

**C1.** For a collateral input `x` of a phase-2-invalid transaction in block `B` on canonical chain
`C`, the authoritative value is `value_C(x)` — a function of the canonical chain alone, identical at
every point of the entry's lifetime `[create(x), spend(x))`. Because `Phase2Invalid` is what spends
the collateral, `spend(x) = B`, so the window is `[create(x), B)` and **the last instant at which
any authority can answer is the moment it applies `B`**.

**C2.** A resolver answers correctly **iff** its cursor lies in `[create(x), B)`. Outside that window
it must answer `None` — truthfully — and `None` must remain a refusal, never a zero and never a
skipped contribution.

> Position determines **only whether the authority can answer, never what the answer is.** The BND-2c
> live failure is therefore a PRESENCE failure, not a valuation failure. The authority was right to
> say `None`; nothing had made it answerable.

### C3 — which component is responsible

Not the accumulator: it must not own, index or query a UTxO (`DC-LEDGER-PHASE2-02`, gate
`ci_check_epoch_accumulator_no_utxo.sh`). Not the checkpoint: it cannot know what another derived
store is folding.

**The co-advancer (`node_lifecycle::advance_ledger_state_to_durable_tip_memo`) is responsible.** It
is the only component that holds both cursors, and it already discharges exactly this duty for the
*other* fact read off the checkpoint: `position_reduced_checkpoint_at_boundary` exists because the
SNAP mark and the checkpoint commitment are authoritative only at `s_prev` (EVIEW-R2 /
`DC-EPOCH-32`). BND-2c added a **second** derived read off the checkpoint — the collateral valuation,
authoritative in `[create(x), B)` — and did not extend the positioning duty to cover it.

**That omission, and nothing else, is the defect.**

A duty may be discharged by positioning, or by making the read position-invariant. The second is
strictly stronger: it cannot be got wrong by a later change to walk order.

---

## 2. THE THREE CANDIDATES, JUDGED AGAINST THE CONTRACT

The criterion is the simplicity of this proof, with crash/restart and replay identity:

```
same canonical chain prefix + same resolver position + same collateral TxIn  ->  same Coin
```

### Position on demand — REJECTED

Satisfies C2, at a cost that is measured, not estimated. A "rewind" is not a cursor move: the
reduced delta is **not invertible** (`position_reduced_checkpoint_at_boundary`, verbatim: *"the
reduced delta is NOT invertible, so re-materializing from the sealed seed is the only way back"*).
Going back means `reset_to_bootstrap()` + replay from the sealed seed — from this venue's anchor
129,813,427 to 130,350,133 that is ~537k slots, the same walk a fresh bootstrap pays at 45–60
minutes. **Per lookup.** Then a second full replay to return the checkpoint to the tip, which EVIEW
requires.

It also inverts authority: the observe-only accumulator would drive the stake authority's cursor on
demand. This is the B6 thrash shape by construction, and the cluster has already paid for it once.

### Lockstep — REJECTED, and not for elegance

Its *positioning* proof is one sentence and, had the contract stopped at C2, it would win. It fails
on the two clauses the criterion actually names:

- **crash/restart.** The accumulator and the checkpoint are two separate redb databases with two
  separate commits. Lockstep makes *"the checkpoint is at `parent(B)` when the accumulator applies
  `B`"* a **cross-database invariant that no single transaction establishes**. A crash between the
  two commits leaves them off-by-one, and every start must then detect and repair that. The proof
  acquires a recovery clause it cannot discharge locally.
- **liveness / authority direction.** The co-advancer today GUARANTEES the checkpoint reaches the
  durable tip *regardless of the accumulator's outcome* (`node_lifecycle.rs`, the trailing
  `advance_reduced_checkpoint_forward_to`) — because the checkpoint is the stake authority feeding
  EVIEW and leadership while the accumulator is observe-only. Pure lockstep lets an observe-only
  stall pin the authoritative stake view: an authority inversion, and a forge-liveness regression
  strictly worse than the bug being fixed. Keeping the tip guarantee re-admits divergence, and
  re-converging needs the same non-invertible rewind that sank candidate 3.

### Resolve at first sight — CHOSEN, in its RETENTION form

The recorded cost of this candidate was: *"the carried scalar becomes a NEW durable fact that must
itself be canonical, replayable and scoped."* That cost is real **for the shape it names** — carrying
the derived scalar `collAdaBalance` forward from the block that produced it.

**C1 dissolves it.** What has to survive is not a derived scalar but the *same immutable `TxIn ->
Coin` binding the authority already holds*, retained past the instant it deletes it. So:

| the stated cost | why it does not apply |
|---|---|
| a NEW fact | not new: it is the binding already in `REDUCED_TABLE`, kept rather than computed |
| must be canonical | a 34-byte `TxIn` key (the checkpoint's existing `txin_key`) and a `u64` BE coin |
| must be replayable | a pure function of the same forward walk; a refold rewrites it identically |
| must be scoped | exactly "the bindings destroyed over `(seed, cursor]`", cleared with the live table |
| a derived scalar to keep in sync | none — BLUE's `collateral_balance()` and the `CollateralValueResolver` trait are UNTOUCHED, and the accumulator still computes its own scalar from resolved inputs |

**The proof:**

```
same canonical chain prefix   -> the authority's walk is forward-only over the durable ChainDB in
                                 admission order, so it visits B exactly once, holding the reduced
                                 UTxO of prefix(parent(B)) when it does
+ same resolver position      -> VACUOUS. The retention is written at the ONE position where the
                                 binding is present, and is position-invariant thereafter.
+ same collateral TxIn
-> same Coin                  -> C1: TxIn -> Coin is immutable, so no other position could have
                                 yielded a different value.
```

Crash/restart: the retention is written in the **same redb write transaction** as the cursor, so the
store can never claim to have applied a block it did not retain for, nor retain for a block whose
slot it did not record. **One store, one transaction, no cross-database invariant.** That is the
clause the other two candidates could not state simply.

---

## 3. INVARIANT

**INV-BND-2d — the UTxO authority retains what it destroys on another reader's behalf.**

A collateral value is authoritative at the point of consumption, and the **UTxO authority**, not the
reader, is responsible for making it available at that point. When the reduced checkpoint removes a
collateral input under the `Phase2Invalid` rule, it records that input's `TxIn -> Coin` binding
atomically with the block that removes it. The binding is thereafter **position-invariant**: the
accumulator's resolver never depends on where the authority's cursor happens to sit. An input the
authority never held is still an unresolved refusal — never a zero, never a skipped contribution.

Registry: **DC-LEDGER-PHASE2-04** (derived). Related: `DC-LEDGER-PHASE2-01/02/03`, `DC-EPOCH-11`,
`DC-EPOCH-32`, `DC-EVIEW-04`.

---

## 4. DESIGN

### 4.1 BLUE — the block says which inputs are collateral of a discarded tx

`reduced_block_delta` already derives the phase-2 gate (`DC-LEDGER-PHASE2-01`): for an invalid tx,
`extract_tx_utxo_effect` returns `spends == collateral_inputs`. It gains one output field:

```rust
pub struct ReducedBlockDelta {
    pub spent: Vec<TxIn>,
    pub produced: Vec<(TxIn, Coin, ReducedStakeRef)>,
    /// BND-2d: the collateral inputs a phase-2-invalid tx consumed in THIS block, canonical order.
    /// `Some(coin)` = created by an EARLIER tx in this same block, so the block itself is the
    /// authority for it; `None` = the binding predates this block and the checkpoint must read it
    /// from its own table BEFORE removing it.
    pub collateral_consumed: Vec<(TxIn, Option<Coin>)>,
}
```

`spent` and `produced` are **byte-identical** to today — the reduced UTxO a store records does not
change. The `Some` arm exists because the net delta CANCELS an intra-block chained spend, so such a
binding never reaches the checkpoint's table; BLUE is the only party that sees the threaded
intra-block produced map, so BLUE resolves that case and hands the value across.

### 4.2 RED/GREEN — the authority retains, atomically

`ReducedUtxoCheckpoint` gains one table, `reduced_collateral_retained: txin_key -> coin(8 BE)`:

- `advance_block(slot, spent, produced, collateral_consumed)` writes the retention **inside the
  existing write transaction**, reading each binding from `REDUCED_TABLE` *before* the removals (or
  taking the `Some` value the block supplied), then commits once as today;
- `reset_to_bootstrap()` **clears** it, exactly as it re-materializes the live table — so the
  retention is precisely "destroyed over `(seed, cursor]`" and a refold re-derives it identically;
- `seal_bootstrap()` clears it for the same reason (it resets `LAST_SLOT` to the seed);
- `compute_fingerprint()` is **untouched** — it iterates `REDUCED_TABLE` only. The checkpoint
  commitment names the reduced UTxO; a retention set is not part of the UTxO, and sealed frozen
  leadership must stay byte-identical for the same prefix.

### 4.3 The resolver — a strict widening

```rust
fn collateral_value(&self, txin: &TxIn) -> Option<Coin> {
    // the authority still holds it -> answer from the live table (C1: same value either way)
    // else -> the binding this authority destroyed under Phase2Invalid
}
```

Live table **first**, retention **second**: answers are a strict superset of v5's and identical
wherever v5 answered. The two sources cover both walk orders:

| the checkpoint is… | who answers |
|---|---|
| at or past `B` (the live-failure case) | the **retention** — this is what BND-2c lacked |
| before `B` | the **live table**, since `B` is what spends the entry |
| before `B`, and `x` was created strictly between the cursor and `B` | **neither** — refusal |

That last row is the honest residual. It is fail-closed, typed, names the exact input, and
**self-clears on the next co-advance pass**, because the trailing advance drives the checkpoint to
the durable tip unconditionally and therefore past `B`. Bounded: at most one stalled pass per block,
never silent, never a fabricated value. It is not repaired with new control flow — an
error-variant-driven retry would buy one log line at the price of a branch, and the refusal being
*reachable and observed* is worth more than its absence.

### 4.4 What the co-advancer does — nothing

Pass order is UNCHANGED. The duty of C3 is discharged by making the read position-invariant rather
than by choreographing two cursors, so no walk-order change can reintroduce the defect. The boundary
positioner (`DC-EPOCH-32`) keeps its own, separate duty untouched.

---

## 5. MECHANICAL ACCEPTANCE CRITERIA

| CE | Criterion | judged by |
|---|---|---|
| **CE-2d-1** | The resolver answers for a collateral input the authority has **already spent** — the exact live failure | unit, real block 130,350,133 |
| **CE-2d-2** | Control: the same lookup against the live table alone returns `None` after the block | unit (non-vacuity) |
| **CE-2d-3** | **THE WALK-TIME TEST.** A real accumulator walk over a ChainDB, with the checkpoint already advanced PAST the block, resolves, applies the transition and advances the cursor THROUGH it | unit, both stores, end-to-end |
| **CE-2d-4** | The retention is written in the SAME write transaction as the cursor | structural gate |
| **CE-2d-5** | `reset_to_bootstrap` clears the retention; a re-walk re-derives it byte-identically | unit (replay identity) |
| **CE-2d-6** | A collateral input created by an EARLIER tx in the same block is retained from the block itself | unit |
| **CE-2d-7** | The unresolved-collateral refusal is still REACHABLE and still typed | unit + gate |
| **CE-2d-8** | The checkpoint fingerprint/commitment is UNCHANGED by the retention | unit |
| **CE-2d-9** | Only collateral of a phase-2-**invalid** tx is retained — an ordinary spend is not | unit (scope) |
| **CE-2d-10** | **LIVE**: fresh bootstrap; at 130,350,133 the collateral resolves, `bnd2c-transition` shows a NON-ZERO fee delta AND a changed `acc_fp`, the cursor advances THROUGH 130,350,114, and a warm restart reproduces the state | live |
| **CE-2d-11** | Negative-tested | mutations below |

### Required mutations

drop the retention write (must fail CE-2d-1 and CE-2d-3) · write the retention in a SECOND
transaction (must fail the structural gate) · return `Coin(0)` when neither source holds the input
(must fail CE-2d-7) · retain ALL spent inputs rather than only collateral of an invalid tx (must fail
CE-2d-9) · skip the clear in `reset_to_bootstrap` (must fail CE-2d-5) · fold the retention into
`compute_fingerprint` (must fail CE-2d-8).

> **The test gap this closes.** Every unit resolver in the tree ANSWERS: `FixedResolver` always
> returns a value and `EmptyResolver` proves the refusal path. Nothing covered *"the live authority
> answers AT WALK TIME"* — the exact live failure. **CE-2d-3 is that test**: it reproduces the
> failure in-tree (checkpoint driven past the block first) and proves the fix, so the venue is
> confirmation rather than discovery.

---

## 6. STORE SEMANTICS — a BUMP, and the surface grows

Replaying the same canonical blocks under this binary advances the accumulator through a block that
previously pinned it, and credits a collateral-derived fee that did not exist. A v5 store is not
replay-equivalent: its checkpoint holds no retention for any block already applied, so the
accumulator would refuse at every past phase-2-invalid block.

⇒ **`STORE_SEMANTICS_VERSION` 5 → 6**, with `ci/ci_check_store_semantics_lock.sh` run **in the same
commit** and a non-neutral entry appended.

The lock's hashed surface also gains
`crates/ade_runtime/src/chaindb/reduced_utxo_checkpoint.rs`. That file defines the durable table set
of a semantics-bearing store; a future change to the retention's meaning that touched only that file
would otherwise pass the lock silently. Adding it is a strengthening, and the same lesson the v3→v4
omission taught: a gate that cannot see the change enforces nothing.

---

## 7. EXPLICITLY NOT IN THIS SLICE

- **The unresolved-collateral refusal is NOT retired.** It caught this. It goes only when a
  mechanically equivalent transition makes it unreachable — and §4.3 keeps it reachable on purpose.
- **No UTxO map, index or `get`-by-`TxIn` on the accumulator.** The authority resolves; the
  accumulator consumes one `Coin`. Gate: `ci_check_epoch_accumulator_no_utxo.sh`, RUN not restated.
- **No re-validation of `total_collateral`** or any other ledger-validity assertion. Validity is
  established upstream; this is not a second tx-validity engine.
- **No B12 / DC-NODE-15 change.** Its `+1` is proven benign AND the authority behind it is still
  unhealthy — both true at once. B12 becomes eligible only after CE-2d-10.
- **No CLK (clock catch-up).** Consensus-sensitive, separately obligated.
- **No change to the co-advance pass order, the boundary positioner, or leadership behaviour.**
