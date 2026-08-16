# NEXT — BND-2c: the positioning contract, before any of the three designs

Entry state: `4171d2ac`. BND-2a CLOSED, BND-2b CLOSED, BND-2c implemented (`390c8191`) with
`DC-LEDGER-03` registered **`partial`** because its live bar failed. Store semantics **v5**.

## What is NOT the problem

The Conway rule is known and quoted verbatim (`bnd2-oracle-extraction.md`). The scalar is right and
unit-proven both ways (`DC-LEDGER-02`, negative-tested). The transition is right and the cert-walk
gate is load-bearing. The accumulator's discard semantics are correct.

**BND-2c failed because the two authorities are not positioned at the same chain point when
resolution is requested.** Nothing more.

Measured: `node_lifecycle` advances the reduced checkpoint to the durable TIP at the end of every
co-advance pass, so during the accumulator's walk at cursor 130,350,114 the checkpoint sits at
~130,550,441, where `0326ab20…#1` has already been spent — by that very block, under BND-2a's
now-correct collateral consumption. The authority truthfully answers `None` and the transition
refuses. Evidence: `bnd2c-v5-live-FAILED-unresolved-collateral.txt`.

## THE QUESTION TO ANSWER FIRST — do not open with a design

> **At what chain point is a collateral value authoritative for an accumulator transition, and which
> component is responsible for guaranteeing the resolver is positioned at that point?**

Answer that as a contract. Only then evaluate the candidates against it.

## The three candidates, and the honest cost of each

| | shape | strength | risk |
|---|---|---|---|
| **Lockstep** | advance checkpoint and accumulator per block together | simplest authority story — position is true by construction | may couple two replay pipelines too tightly |
| **Resolve at first sight** | the checkpoint computes `collAdaBalance` while applying the block, when it still holds the entry, and carries the scalar forward | preserves decoupling | creates a NEW durable/intermediate fact that must itself be canonical, replayable and correctly scoped — i.e. it is a store-semantics question of its own |
| **Position on demand** | rewind the checkpoint to the accumulator's cursor per lookup | preserves the current separation | this is the B6 thrash shape; risks making a narrow lookup expensive and hiding rewind/replay behind it |

## The selection criterion — NOT elegance

The winner is whichever gives the cleanest proof of:

```
same canonical chain prefix
+ same resolver position
+ same collateral TxIn
    -> same Coin
```

with **crash/restart and replay identity**. If a candidate cannot state that proof simply, it is the
wrong candidate regardless of how tidy the code looks.

## Rails that carry forward

- **Do not retire the unresolved-collateral refusal.** It is what caught this. It goes only when a
  mechanically equivalent transition makes it unreachable — not when tests stop failing.
- **Do not give the accumulator a UTxO map.** The authority resolves; the accumulator consumes.
- **Do not re-validate `total_collateral`.** Block validity is established upstream; the accumulator
  reproduces the transition of an already-valid block.
- **B12 stays untouched.** Its `+1` is proven benign (`fcbabb67`); the authority behind it is still
  unhealthy, and both statements are true at once.
- Any `STORE_SEMANTICS_VERSION` edit must run `ci/ci_check_store_semantics_lock.sh` **in the same
  commit** — that gate is not wired into `cargo build` and was missed twice before an audit caught it.

## Test gap to close alongside the fix

Every unit resolver in the tree ANSWERS. `FixedResolver` always returns a value; `EmptyResolver`
proves the refusal path. **Nothing covers "the live authority answers AT WALK TIME"** — which is
exactly what failed. Whatever design wins should be provable without needing the venue to discover a
positioning bug again.

## Venue

Live v5 store `~/.cardano-live1/ade-preprod-v5`, kept at cursor 130,350,114 as the reproducer — it
reaches the failing block in one walk from a warm start, so the next attempt does not need a fresh
bootstrap to see the defect. A fresh v-next bootstrap IS required to close the slice.
`FROZEN-b6-census-s7` remains v3 historical evidence (needs a binary ≤ `3505c0c6`).
