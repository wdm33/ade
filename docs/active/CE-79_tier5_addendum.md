# CE-79 Addendum — Tier 5: Intentional Divergence

> **Status**: draft addendum to `docs/active/CE-79_gate_statement.md`.
> Does NOT modify CE-79 itself until approved.

## Existing tier vocabulary (recap)

CE-79 defines four tiers:

- **Tier 1: `true`** — mechanically enforced (type system, CI hooks,
  pure-function boundaries).
- **Tier 2: `derived`** — corpus-verified property that extrapolates
  to the population.
- **Tier 3: `release`** — scope of external certification (narrow,
  with named preconditions and residuals).
- **Tier 4: `non-goal`** — explicit out-of-scope, with stated rationale.

This vocabulary handles "must conform" and "explicitly opted out of
conforming." It has no positive vocabulary for surfaces where the
project deliberately *improves* on cardano-node's choices. Without
that vocabulary, the cultural default for any non-hash-critical
surface drifts toward "match Haskell anyway," which collapses the
strategic opportunity of having a second implementation.

## Tier 5 — intentional divergence

A surface is Tier 5 when **all three** hold:

1. **No protocol requirement.** The bytes are not hashed, not signed,
   and not carried over a wire mini-protocol that cardano-node already
   speaks (or that Ade must speak to interoperate).
2. **Stated design intent.** The project has chosen to do something
   different from cardano-node for at least one of: performance,
   ergonomics, safety, simplicity, observability, packaging.
3. **Documented rationale.** The divergence carries a one-paragraph
   rationale that names what is different and why it is better.

A surface that satisfies (1) but lacks (2) and (3) is not Tier 5 — it
is **unclassified**, which usually means a design decision is owed.

## Examples

| Surface | Tier | Notes |
|---|---|---|
| Block / tx / script bytes | 1 | Hashed; conformance is consensus |
| Datum / redeemer / ScriptIntegrityHash inputs | 1 | Hashed |
| Ouroboros mini-protocol bytes | 1 | Wire bytes between nodes |
| Ledger semantics, Plutus evaluation result | 1/3 | Consensus rules |
| Internal state encoding (ExtLedgerState dump) | 4 | Already non-goal per CE-79 |
| Chain DB on-disk layout | 5 | rocksdb / sled / custom — pick what's faster |
| Local query / IPC layer | 5 | LocalStateQuery is awkward; native gRPC welcome |
| Configuration / topology files | 5 | YAML sprawl is a known operational pain |
| Telemetry / metrics surface | 5 | Prometheus / OpenTelemetry vs. bespoke trace |
| Mempool eviction & prioritization policy | 5 | Within consensus rules; policy is free |
| Operator UX, CLI shape, packaging | 5 | Single static Rust binary, no GHC RTS |
| Build / dependency footprint | 5 | Rust crate hygiene |
| State-hash agreement with Haskell ExtLedgerState | 4 (currently) | Promote to 3 if interop becomes operational req |

## Why this matters

The default cultural pressure in any compatibility-focused project is
"match the reference unless explicitly excluded." That default is
correct for hash-critical surfaces — they must match. The default is
**wrong** for everything else, because it produces a port that shadows
the reference's implementation choices instead of improving on them.

Tier 5 forces the issue. Every non-hash-critical surface gets one of
these classifications:
- Tier 4: "we explicitly do not match Haskell here, here's why."
- Tier 5: "we deliberately do this differently, here's what's better."

Either is a defensible position. Leaving a surface unclassified is not.

## Operational rule

When designing any new feature or slice, classify each observable
surface with the question:

> Does this byte / behavior get hashed, signed, or sent on a wire
> protocol cardano-node already speaks?

- **Yes** → Tier 1 (or Tier 3 for release-scoped). Conform.
- **No, and we'll match anyway** → Tier 4 with rationale (e.g., audit
  parity, snapshot interop). Document why.
- **No, and we'll diverge** → Tier 5 with rationale. Document what's
  better.

Slice-entry obligation discharge must include the tier classification
for each new observable surface the slice introduces. Reviewers reject
slices that introduce Tier 5 surfaces without rationale, the same way
they reject slices that omit comparison-surface contracts.

## Relationship to existing doctrine

This addendum extends CE-79 — it does not contradict it. The four
existing tiers stand. Tier 5 is added below Tier 4 in classification
order:

```
Tier 1: must conform (mechanically enforced)
Tier 2: must conform (corpus-verified)
Tier 3: must conform (release-scoped)
Tier 4: explicitly does not conform (non-goal, with rationale)
Tier 5: deliberately diverges (intentional improvement, with rationale)
```

The wire/canonical separation rule (`PreservedCbor::wire_bytes()` vs.
`canonical_bytes()` in `comparison_surface_contract.md`) is unchanged
and remains the mechanical enforcement of Tier 1 on hash-critical
paths.

## Revisit triggers

- A Tier 5 surface that turns out to be hash-critical after all (e.g.,
  some bytes flow into a hash via an indirect path) gets reclassified
  to Tier 1 and either fixed or the slice rolled back.
- A Tier 4 surface where the rationale changes (e.g., snapshot
  interoperability becomes operationally required) gets promoted to
  Tier 3.
- A Tier 5 surface whose rationale evaporates (e.g., upstream Haskell
  fixes the awkwardness we were diverging from) gets re-evaluated:
  conform now, or re-justify divergence.
