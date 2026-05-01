# CE-73 Reclassification Proposal

> **Status**: draft for review. Does NOT modify
> `constitution_registry.toml` or `docs/active/phase_2c_progress_report.md`
> until approved.

## Problem

CE-73 as currently written:

> "Era translations for all 6 transitions: ledger state translated
> correctly, producing identical state hashes to oracle after
> translation."

CE-79 Tier 4 (`docs/active/CE-79_gate_statement.md:137-138`) explicitly
carves out as non-goal:

> "Byte-parity with Haskell `ExtLedgerState` on-disk CBOR" — the
> format is implementation artifact, version-coupled, not spec.

These two statements contradict. CE-73's "identical state hashes" path
requires reproducing Haskell's internal `serialise`-compatible encoder
(estimated 4-6 weeks per `phase_2c_progress_report.md`). CE-79 Tier 4
says exactly that work is out-of-scope. The contradiction is what's
keeping CE-73 nominally "open" while no one is willing to fund the
encoder spike.

## Proposed split

Replace the single CE-73 with two separate gates that each stand on
their own classification.

### CE-73-semantic — CLOSED

**Criterion**: Era translations for all six transitions
(Byron→Shelley, Shelley→Allegra, Allegra→Mary, Mary→Alonzo,
Alonzo→Babbage, Babbage→Conway) produce semantically correct ledger
state. Every observable field — treasury, reserves, UTxO count, pool
distribution, delegation set, protocol parameters, and (where
applicable) governance state — matches oracle after translation.

**Tier**: 2 (derived — corpus-verified, extrapolates).

**Evidence**:
- Allegra→Mary transition: 22/22 fields match
  (`docs/active/phase_2c_progress_report.md:374`).
- Translation predicates are uniform across the five non-Byron
  transitions; the Allegra→Mary proof generalizes.
- Byron→Shelley is a structural reformat (UTxO and account migration);
  proven separately.

### CE-73-bytes — NON-GOAL (Tier 4 per CE-79)

**Criterion**: Byte-identical state hash with Haskell's on-disk
`ExtLedgerState` CBOR after translation.

**Status**: explicit non-goal.

**Reasoning**:
1. The Haskell on-disk format is implementation-internal,
   version-coupled, and not part of any wire protocol or consensus
   rule.
2. State-hash agreement with cardano-node would provide audit-grade
   snapshot interoperability with cardano-node operators. It does not
   gate any aspect of running on the network.
3. The same divergence-detection guarantee is provided by Ade's
   canonical fingerprint format (`ade_ledger::fingerprint`) without
   version-coupling to Haskell's internal serializer.
4. A node can produce correct ledger state, validate blocks, produce
   blocks, and participate in consensus without ever reproducing
   cardano-node's snapshot hash.

**If state-hash agreement becomes operationally required later** (e.g.,
to bootstrap from a cardano-node operator's on-disk snapshot rather
than from genesis-replay), it can be revisited as a separate
release-scoped (Tier 3) optional feature, not as a Tier 1 hard gate.

## Registry update (proposed)

CE-73 is a doctrinal exit criterion, not a `[[rules]]` registry entry.
The registry tracks `T-` / `CN-` / `DC-` rules; the rule that carries
CE-73's scope is **`DC-EPOCH-02`** at
`constitution_registry.toml:662-673`.

The proposed registry-side change is documented as a unified diff in
**`docs/active/CE-73_registry_diff.md`**. Summary:

- `DC-EPOCH-02.status`: `partial` → `enforced` (no remaining open work
  in ledger-side scope; consensus-side HFC moved to Phase 4 N-B).
- `DC-EPOCH-02.authority_surface`: tightened to reference CE-73-semantic
  (Tier 2 closed) and CE-73-bytes (Tier 4 non-goal) explicitly.
- `DC-EPOCH-02.evidence`: added pointers to progress report,
  reclassification doc, and T-26 slice doc.

See the diff doc for the exact before/after and apply checklist.

## Effect on roadmap

Reclassifying CE-73 closes Phase 2C's last-open hard gate without the
4-6 week encoder spike. That capacity (and the strategic uncertainty
the open gate created) redirects to Phase 4 cluster planning, which is
the actual gating constraint on having a runnable node.

## Verification

This reclassification is a documentation change with no code
implications. Verification is doctrinal:
- Confirm CE-73-semantic evidence in
  `docs/active/phase_2c_progress_report.md` matches the 22/22 field
  claim.
- Confirm CE-79 Tier 4 statement still names the on-disk format as
  non-goal.
- Confirm `constitution_registry.toml` accepts the split entries.
