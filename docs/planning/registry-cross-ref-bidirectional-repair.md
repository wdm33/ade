# Follow-up: registry cross_ref bidirectional repair (registry-consistency)

> **Status:** Scoping note for a small, standalone registry-consistency change.
> **Not** PHASE4-N-F; **not** to be bundled with produce-mode wiring planning.
> Surfaced 2026-05-29 while doing the `e8bde40` docs/CI hygiene pass.

## Finding

`ci/ci_check_constitution_coverage.sh` is **red on `main`** (pre-existing; not
caused by `e8bde40`). It enforces, among other things, that every `cross_ref`
in `docs/ade-invariant-registry.toml` is **bidirectional** — if rule A lists B
in `cross_ref`, B must list A back. Eleven references from the recent N-Y/N-Z
clusters are one-directional:

| Rule | `cross_ref` → target with no back-reference |
|------|----------------------------------------------|
| `DC-SYNC-01` | `DC-STORE-02`, `DC-STORE-05`, `T-DET-01` |
| `DC-GENESIS-SRC-01` | `CN-GENESIS-01`, `CN-ANCHOR-01`, `CN-NODE-01`, `T-DET-01` |
| `DC-COMPAT-01` | `T-DET-01`, `CN-OPERATOR-EVIDENCE-01` |
| `RO-SYNC-EVIDENCE-01` | `CN-OPERATOR-EVIDENCE-01` |
| `DC-MITHRIL-02` | `DC-MITHRIL-01` |

(Reproduce: `bash ci/ci_check_constitution_coverage.sh` — the `FAIL: … cross_ref
… is not bidirectional` lines.)

## The decision (not a casual patch)

Two coherent resolutions; pick deliberately — this is about what the gate is
meant to prove, not just turning it green:

- **(A) Add the reciprocal `cross_ref` entries** to each referenced rule
  (append-only to the `cross_ref` arrays — a *strengthening* of the
  cross-reference graph, never a rule weakening; no `status`/`statement`/
  `tests`/`ci_script` change). Straightforward for the peer-level pairs
  (e.g. `DC-MITHRIL-02 ↔ DC-MITHRIL-01`, `DC-GENESIS-SRC-01 ↔ CN-GENESIS-01`).
- **(B) Reconsider bidirectionality for foundational invariants.** `T-DET-01`
  (determinism) is referenced *by* many derived rules; forcing it to enumerate
  every consumer back-references bloats it and inverts the dependency
  direction. The gate may instead exempt a small set of foundational
  true-invariant targets (`T-DET-01`, possibly `CN-NODE-01`) from the
  bidirectional requirement, or treat "referenced-by" as implicit.

**Recommendation:** (A) for the genuine peer pairs + (B) for `T-DET-01`-style
foundational targets. Decide before editing — adding ~11 back-refs blindly may
encode the wrong graph shape.

## Scope fence

- Standalone registry-consistency change. **Separate from PHASE4-N-F.**
- No status flips, no invariant promotion, no code behavior. Append-only +
  strengthen-only preserved.
- Done when `ci/ci_check_constitution_coverage.sh` is green (the two
  `WARN: … document not found at , skipping` lines are invocation-arg related,
  not part of this fix).

Relates to: `DC-MITHRIL-02`, `DC-SYNC-01`, `DC-GENESIS-SRC-01`, `DC-COMPAT-01`,
`RO-SYNC-EVIDENCE-01` (N-Y/N-Z), `T-DET-01`.
