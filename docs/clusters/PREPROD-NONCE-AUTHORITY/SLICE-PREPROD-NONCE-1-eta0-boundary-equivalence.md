# SLICE PREPROD-NONCE-1 — preprod 304→305 eta0 boundary equivalence (DC-EPOCH-16)

> **OPEN — INSTRUMENTATION FIRST, NO BEHAVIOUR CHANGE.** Doc before impl.
>
> A fresh, correctly-marked preprod store fails closed crossing 304→305 because the epoch tick's
> `eta0(305)` disagrees with the bridge's. This blocks preprod LIVE-2 and therefore the bounty path.

## Two separate facts — do not conflate them

The P6 work is **done and working**; this is a different subsystem.

| | |
|---|---|
| **P6 store-semantics gate** | **WORKED.** The old pre-P6 store was refused (`found: Absent`). The new store was created, stamped `v1`, and **opened cleanly** on warm restart (0 `StoreSemantics` errors). Steps 5–6 of the re-bootstrap sequence are satisfied. |
| **Preprod boundary authority** | **FAILED.** `DC-EPOCH-16 epoch-tick eta0 != bridge eta0 at epoch 305`, `rc=43`. On restart, recovery then **correctly** refused with `ActivationAboveDurableTip { target_epoch: 305, durable_tip_epoch: 304 }`. |

The store-marker problem is solved. The new blocker is **preprod Praos nonce / candidate-freeze
geometry**.

## Observed

```
ade_node --mode node: relay run-loop sync step failed (Pump(
  "DC-EPOCH-16 epoch-tick eta0 Hash32(74f10bea2b467cac73efbd02b36307fe12a123b098a94cfcfe4c33ce4ef10b62)
                    != bridge eta0 Hash32(e3402a2b2d04d1055ccf6a6fbafc3febda97a6a7b3a4247f84d9c6070965c7a1)
   at epoch EpochNo(305)")); failing closed (no skip-past, no fallback)
rc=43
```

The check is `node_sync.rs:763` — the FIRST-boundary cross-check after a seeded bootstrap, where the
tick must reproduce the bridge eta0 byte-for-byte.

## Already ruled out by measurement

Geometry is sound; the node had every block it needed.

```
epoch 304 start        129_686_400
Mithril anchor         129_813_427    (127,027 slots into epoch 304)
candidate freeze slot  129_945_600    <- anchor PRECEDES it
last synced            130_108_106    <- PAST the freeze
epoch 305 start        130_118_400    (durable tip reached 130_118_358, 42 slots short)
```

- **Not a missing window**: the anchor precedes the freeze slot and the node synced past it, so the
  candidate had the full pre-freeze range available.
- **Not a wrong RSW formula**: `praos_rsw_slots(k, 1, 20)` gives preprod 172,800 and preview 34,560,
  matching `ceil(4k/f)` by hand.

Remaining candidates, none yet distinguished: the seed's imported candidate-vs-evolving pair at the
anchor; the freeze not actually firing at the computed slot; or the bridge's own eta0 provenance.

## Why this was reachable — the historical gap

`dc14787a` (LIVE-FORGE-HARDENING S2, DC-EPOCH-16) made the durable store the sole candidate-freeze
authority by persisting `k` in the v6 sidecar. Its own commit message records two things that matter
here:

1. The worked proof is **preview** — *"preview k=432, f=1/20 -> RSW 34560 — the exact window whose
   absence was the forge blocker"*.
2. The end-to-end byte-identical eta0 across a crossed boundary was **inherited, not re-run**:
   *"re-running the #[ignore] corpus proof under v6 needs a regenerated v6 seed store (documented
   follow-up in the slice doc; the v5 seed is schema-incompatible by design)"*.

So the candidate-freeze → eta0 path has only ever been proven on **preview geometry**. Preprod
(k=2160, RSW 172,800, epoch 432,000) has never been exercised across a boundary. This is a gap in
coverage, not a regression.

## Instrumentation (emit-only) — the one pass this slice buys

Front-loaded so a single fresh re-bootstrap answers it; each observation costs ~15 min of sync.

| field | why |
|---|---|
| **network profile source** (CLI / network magic / genesis-derived) | The prior preview harness bug was exactly a **wrong venue k**. Provenance must be visible, not assumed. |
| venue `k` / `f` / derived RSW | The three inputs to the freeze window |
| freeze slot actually used | Distinguishes "computed right" from "applied right" |
| candidate nonce at freeze | The operand eta0 is built from |
| evolving nonce at boundary | The other half of the rotation |
| last-epoch-block operand | `blake2b(candidate ‖ lastEpochBlock)` — the second input |
| imported seed candidate/evolving pair | Whether the seed handed us a correct starting point |
| bridge eta0 + its provenance | Which record the expectation came from |
| computed eta0(305) | The disagreeing value |
| reference eta0(305), if obtainable | An independent oracle (cardano-cli / peer), if available |
| activation record write point | For the *consequence* below — recorded, not acted on |
| durable tip at activation write | Same |

## HOLD — explicitly not in this slice

- **Do NOT patch `ActivationAboveDurableTip`.** The refusal is CORRECT: an activation targeting epoch
  305 with a durable tip in 304 must fail closed. Treat it as a **consequence** until proven otherwise.
  If fixing eta0 makes the activation ordering clean, nothing more is needed; if it does not, the
  activation-write ordering becomes its own sealed slice. Keep them separate.
- **Do NOT weaken any guard** to get past the boundary.
- **Do NOT claim** the multi-venue differential covers nonce geometry (see below).
- **Do NOT resume preprod LIVE-2 forge-readiness** until this closes.

## DC-EPOCH-38 — a NEW adjacent surface, not a reopening

DC-EPOCH-37 (P6-S5) is **complete for what it covers**: epoch *derivation* geometry per venue. It is
not wrong and is not reopened.

Candidate-freeze / nonce geometry is a **distinct observable equivalence surface** that no differential
covers, and it is now a *proven* missing surface rather than a hypothetical one. It gets its own ID:

> **DC-EPOCH-38 — multi-venue Praos nonce / candidate-freeze differential.** For every venue in the
> closed registry, the candidate-freeze window, freeze slot, and eta0 rotation must be exercised
> against that venue's real `k`/`f`/epoch geometry, and a mismatch must localize to the operand that
> differs.

It is deliberately opened only AFTER the diagnosis, so the gate encodes what the diagnosis proves
rather than what it guesses.

## Order (binding)

1. Commit this doc. *(doc-before-impl)*
2. Preserve the stuck preprod store + logs as evidence.
3. Add emit-only instrumentation — no behaviour change.
4. ONE fresh re-bootstrap under the instrumented binary.
5. Diagnose from the emitted operands.
6. Only then: fix, and open DC-EPOCH-38 encoding the proven surface.

## Why this blocks the bounty path

The bounty requires producing a valid block **accepted by other nodes** on preview or preprod, with
protocol and validity agreement against Haskell. If Ade cannot compute the correct preprod `eta0`
across 304→305, then leader checks and header VRF validation around that boundary are not trustworthy
enough for a public forge attempt. Failing closed here is right; forging on an unproven nonce is not.

## Not claimed

No root cause, no fix, no invariant. This records a reproducible blocker, what has been ruled out by
measurement, and the exact operands the next pass must emit.
