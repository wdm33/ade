# SLICE P2 — preprod reaches tip, then fails closed capturing a REDUCED state

> **DIAGNOSED 2026-08-05 — and it is TWO different defects wearing the same error.** The 08-03
> instance was the **P3 phantom-boundary defect and is already fixed**. What remains is structural and
> is diagnosed below. Blocks LIVE-2 on preprod.
>
> Original sealing note (kept): found 2026-08-03 03:32Z on the first successful preprod bootstrap
> (after P1), deliberately not investigated at discovery — consensus-adjacent, ~7 h into a session
> with a leader slot 10 minutes away, precisely the conditions under which a wrong fix gets written.

## What happened

P1 fixed, chunk-6009 snapshot bootstrapped cleanly, and the node caught up **without a single
error**:

```
follow: tip slot=129971821 behind=72892 slots from peer
follow: tip slot=129982521 behind=62192
... (monotonic, no faults) ...
follow: tip slot=130040378 behind=4335
follow: tip slot=130044713 -- AT PEER TIP (caught up, following live)
ade_node --mode node: relay run-loop sync step failed
    (Capture("Encode(ReducedStateNotSerializable)")); failing closed (no skip-past, no fallback).
                                                                                  exit 43
```

The failure is the **first capture attempt on reaching tip** — catch-up itself was clean.

## The error is BY DESIGN, so the bug is upstream of it

`encode_ledger_state` (`snapshot/ledger.rs:69,73`) refuses to serialise a state whose `cert_state` or
`gov_state` is `ReducedUnavailable`:

> *"A reduced follower's cert/gov is `ReducedUnavailable` — NOT serializable as a normal
> full-authority snapshot (fail closed rather than fabricate a normal CertState/gov; RVBP)."*

Pinned by `reduced_continuation_is_not_serializable_as_authority`. So the guard is correct and must
NOT be relaxed; the question is **why the preprod state is reduced at all**.

## Evidence

| | preprod | preview |
|---|---|---|
| occurrences of `ReducedStateNotSerializable` | 1, immediately at tip | **0** across hours (both `ade-rf1-ce5.log` and the baseline) |
| epoch boundary crossed during catch-up? | **no** — 129,813,427 → 130,044,713, all inside epoch 304 (129,686,400 – 130,118,400) | n/a |
| catch-up faults | none | n/a |

That the node never crossed a boundary matters: the reduced projection is associated with the
reduced-follower / window-replay path (`reduced_window_driver.rs:49,98`), and a boundary crossing is
the obvious way to enter it. **Neither applies here**, so the state appears to be reduced from
bootstrap — yet `seed_from_bootstrap_ledger` is documented to *reject* a `ReducedUnavailable` seed
(`mithril_native_assembly.rs:367`). Those two facts are in tension and that tension is the
investigation.

## Candidate directions (NONE selected — this is the open work)

1. The native Mithril bootstrap yields a full seed but the **live follow** transitions the state to a
   reduced projection before the first capture.
2. Something in the preprod state makes a cert/gov sub-projection unavailable that preview's never
   exercises — the P1 pattern again (*an empty collection on one venue hides its behaviour*).
3. The capture cadence differs between venues, so preview simply has not attempted the same capture.
   **Check this first** — it is the cheapest to falsify and would reframe the whole finding.

Explicitly NOT acceptable as a fix: relaxing the RVBP guard, or fabricating a full CertState/gov to
make the encode succeed. The guard exists so that nothing a reduced follower produced can be
rehydrated or fingerprinted as authority.

## Impact

- **LIVE-2 on preprod is blocked**, one layer deeper than P1.
- Epoch-304 leader slot 130045510 (03:45:10Z) was **missed** as a direct consequence. Remaining:
  130074818 (11:53Z) and 130081569 (13:46Z).
- Every operator prerequisite remains verified and unaffected: opcert valid to 2026-09-15 with
  KES vkey `fd2f1de3…` **proven equal** to the loaded key, stake
  `stakeGo == stakeSet == stakeMark == 1,009,506,139,807`, peer synced, bootstrap succeeded.
- Preview is unaffected and still running.

## NEW EVIDENCE 2026-08-05 (from the PREPROD-NONCE-2 CE-N2-4 run) — direction 3 is FALSIFIED

The CE-N2-4 live run bootstrapped preprod from the same snapshot 6009 and followed 129,813,427 →
130,118,424, this time CROSSING the 304→305 boundary. It hit the same error, but the surrounding facts
differ from the 08-03 observation in a way that answers P2's cheapest open question:

```
follow: ... 129,813,427 -> 130,118,358   (all of epoch 304)   -- ~152 capture opportunities, ALL SILENT
nonce1-boundary-operands: from_epoch=304 to_epoch=305 ... match=true   -- boundary crossed CORRECTLY
ade_node: relay run-loop sync step failed (Capture("Encode(ReducedStateNotSerializable)"))   rc=43
```

| question | answer from this run |
|---|---|
| **Direction 3** — "does preview simply not attempt the same capture?" (marked *check this first*) | **FALSIFIED for preprod.** The E4 cadence capture (`RECOVERY_CHECKPOINT_CADENCE_SLOTS = 2000`) runs every sync step. Over ~305k slots of epoch-304 catch-up that is ~152 attempts, and **every one succeeded**: zero `capture-refused` emits, zero errors. Preprod captures fine; it is not a cadence artifact. |
| **Direction 1/2** — is the state reduced *from bootstrap*? | **No, not in this run.** A state reduced from bootstrap would have failed the first cadence capture ~2000 slots in. It captured cleanly through the whole seed epoch and failed only *after* the boundary. |
| which call site fails | **The B3b yield-at-boundary capture** (`node_sync.rs:894`), not the cadence one. The boundary path `return`s early with `BoundaryPromoted`, so the E4 block is skipped and the unconditional boundary capture runs instead. |

So in this run the reduced projection appears **at/through the epoch-boundary crossing**, not at
bootstrap — which resolves the tension this slice recorded between "the state appears to be reduced
from bootstrap" and `seed_from_bootstrap_ledger` rejecting a `ReducedUnavailable` seed. The seed was
full, and it stayed full for an entire epoch of live follow.

**Held deliberately narrow.** The 08-03 symptom (failure at tip, *inside* epoch 304, no boundary) is
NOT the same as this one (failure at the boundary, after ~152 clean captures). Several clusters landed
between the two observations, so this run does not prove the original symptom still reproduces — only
that the boundary-triggered variant does, and that direction 3 does not explain it.

## DIAGNOSIS 2026-08-05 — the 08-03 instance is ALREADY FIXED; a second, structural one remains

### (a) The 08-03 instance was the P3 phantom boundary

The original observation — failure at tip, **inside** epoch 304, no boundary crossed — is explained by
the defect P3 fixed, and `state.rs:415` names this exact chain in its own words:

> *"preprod slot 130,046,891 -> 498 instead of 304 … a fictitious epoch ABOVE the real one declares a
> phantom boundary, routes through `apply_reduced_epoch_boundary`, and leaves cert/gov/snapshots
> permanently `ReducedUnavailable`."*

A phantom boundary inside epoch 304 is precisely "no boundary crossed, yet the state went reduced,
then the first capture failed". P3 removed the mainnet-constant epoch formula, so that instance cannot
recur. **This is why the 08-03 symptom did not reproduce on 08-05.** Candidate directions 1–3 above
were all aimed at that instance; none of them was the cause.

### (b) What remains is structural, and every step of it is BY DESIGN

The CE-N2-4 run crossed a **real** 304→305 boundary and hit the same error from a different cause:

```
mithril_native_assembly.rs:364   the native seed ledger is track_utxo: false
                                 with cert_state: Authoritative(...)   <- captures SUCCEED
rules.rs:202  dispatch_epoch_boundary: Conway && !track_utxo -> apply_reduced_epoch_boundary
rules.rs:145  apply_reduced_epoch_boundary: cert_state = ReducedUnavailable
                                            gov_state  = ReducedUnavailable   (N-RVB-1 / N-RVB-3)
ledger.rs:69  encode_ledger_state: as_authoritative() -> None -> ReducedStateNotSerializable
```

Each line is correct in isolation, and together they are a **collision**:

| | |
|---|---|
| RVBP requires | a `track_utxo=false` Conway follower crossing a boundary has cert/gov `ReducedUnavailable` — never a stale full `CertState` a later reader could mistake for advanced lifecycle |
| the encoder requires | a serializable `LedgerState` to have authoritative cert/gov |
| therefore | **a reduced follower can capture recovery checkpoints only until its first boundary, and never again** |

The seed is `Authoritative`, which is why ~152 cadence captures succeeded through epoch 304 and the
failure lands exactly at the first boundary. Nothing here is venue-specific; preprod is simply the
first venue where a `--mode node` reduced follower actually crossed a boundary.

### The fix — make the reduced projection DURABLY REPRESENTABLE, following the sibling that already is

The RVBP guard is not relaxed and no cert/gov is fabricated (both forbidden by this slice). Instead the
snapshot encoding gains the same typed treatment `EpochStakeSnapshots` **already has**:

```
epoch_state.snapshots   Authoritative -> array(3) mark/set/go     (existing, byte-identical)
                        ReducedUnavailable -> array(0)            <- ALREADY SHIPPED (RVBP gates 1/2/3, 7)
cert_state              Authoritative -> bytes                    (existing, byte-identical)
                        ReducedUnavailable -> array(0)            <- ADD
gov_state               Authoritative(Some) -> bytes              (existing, byte-identical)
                        Authoritative(None) -> null               (existing, byte-identical)
                        ReducedUnavailable -> array(0)            <- ADD
```

`array(0)` already MEANS "reduced" in this encoding, so the convention is inherited rather than
invented. Properties:

- **Backward compatible.** Every authoritative encoding is byte-identical, so existing snapshots
  decode unchanged and no store is invalidated. `array(0)` is a form the encoder previously could not
  emit (it errored instead), so it collides with nothing.
- **Fails closed for old readers.** A pre-fix binary meeting `array(0)` where it expects `bytes` errors
  in `read_bytes` — it cannot misread a reduced snapshot as authority.
- **Reduced can never rehydrate as authority.** Decode maps `array(0)` → `ReducedUnavailable`, and
  `Authoritative` is reachable only from the `bytes`/`null` forms. Consumers needing authority still go
  through `require_full` / `as_authoritative` and fail closed.

### This CHANGES A LANDED USER GATE — stated plainly, because it must not be mistaken for a relaxation

`reduced_continuation_is_not_serializable_as_authority` (user gate #3) currently asserts that encoding a
reduced continuation **errors**. Its stated property, in its own words, is that *"nothing a reduced
follower produced across the boundary can be rehydrated or fingerprinted as authority"*. Refusing to
encode is one way to secure that property — a blunt one that also makes the follower unrecoverable.

The property is preserved and the mechanism upgraded from *refuse* to *typed round-trip*, which is the
mechanism its sibling projection already uses. The gate is rewritten to assert the property directly
and more strongly:

1. a reduced continuation now ENCODES;
2. it decodes back with cert AND gov `ReducedUnavailable` — never `Authoritative`;
3. an authoritative state's bytes are UNCHANGED (no regression on the full path);
4. an authoritative state never decodes as reduced.

(1) alone would be a relaxation. (1)+(2)+(4) is the same guarantee expressed in the type system rather
than by absence, which is the direction this codebase moves in everywhere else.

### What this does NOT fix

`PREPROD-NONCE-3` (warm-start replay re-validating the bridge-boundary block, `VrfCert`) is a separate
failure on the restart path and is **not** addressed here. Order is deliberate: fix the forward path
first so NONCE-3 is diagnosed from a clean post-boundary durable state rather than a contaminated one.

### Instrumentation gap this exposes

P2's own emit-only diagnostic (`capture-refused:`, added at `node_sync.rs:966`) is attached to the
**cadence** capture site only. The **boundary** capture site at `node_sync.rs:894` has no such emit, so
the failure that actually fires arrives as a bare `Capture("Encode(ReducedStateNotSerializable)")` with
none of the projection detail the emit was written to supply. Extending the emit to that site is the
cheapest next step and costs nothing behaviourally.

## Acceptance criteria

| CE | Criterion | status |
|---|---|---|
| **CE-P2-1** | A reduced continuation ENCODES, and decodes back with cert AND gov `ReducedUnavailable` | open |
| **CE-P2-2** | Authoritative encodings are BYTE-IDENTICAL to pre-fix (no existing store invalidated) | open |
| **CE-P2-3** | An authoritative state never decodes as reduced, and a reduced one never as authoritative | open |
| **CE-P2-4** | User gate #3's property (nothing reduced rehydrates/fingerprints as authority) still holds, by type | open |
| **CE-P2-5** | Live: preprod crosses 304→305 and the boundary capture SUCCEEDS (no rc=43) | open |
| **CE-P2-6** | Negative-tested: the gate FAILS when reduced is made to decode as authoritative | open |

## Not claimed

No invariant and no CE flipped yet. The 08-03 instance is attributed to P3 (fixed); the remaining
structural collision is diagnosed above with the fix direction stated. This slice records a
reproducible failure with its evidence so it is not re-discovered, and records that the P1 fix
genuinely worked — preprod bootstrapped and reached tip, which had never happened before.
