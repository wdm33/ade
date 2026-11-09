# Constitution-coverage gate: retirement (RESOLVED decision record)

> **Status:** RESOLVED. (Filename retained for continuity with commit `d5ecfe5`,
> which opened this as a "cross_ref bidirectional repair" note — that framing was
> wrong; see the correction below.)
> Outcome: the `ci_check_constitution_coverage.sh` gate was **retired** and its two
> load-bearing checks **folded into** `ci_check_registry_code_locus_exists.sh`.

## Correction (the finding evolved as it was investigated)

The earlier framing understated this badly, in two steps:

1. It was first reported as **"11 one-directional cross_refs (N-Y/N-Z)."** That was a
   truncation artifact (only the tail of the gate output was read). The gate actually
   reported **258** one-directional cross_refs across 107 citing rules / 85 targets —
   i.e. strict bidirectionality was **never** the registry's invariant.
2. Even "relax bidirectionality" was too narrow: the gate fails on **414 lines across
   8 independent check classes** (bidirectionality 258, unknown-cluster 51, enforced-
   missing-`ci_script` 41, CN-missing-fields 22, derived-missing-`ci_script` 19,
   release-partial-forbidden-fields 18, tier-by-prefix 2, …).

**Root cause:** `ci_check_constitution_coverage.sh` is a **ziranity-v3 import**, not an
Ade gate. Evidence: its header validates `constitution_registry.toml` (the ziranity
filename); `KNOWN_CLUSTERS` is frozen at ziranity's `{CL-CRYPTO-VERIFY, CL-LEDGER-VERDICT}`;
its coverage checks read `$HOME/Documents/ade-planning/*` (out-of-repo, non-portable);
it assumes tier-by-ID-prefix (contradicted by Ade's own `constraint` tier and
`DC-ADMIT-07`=`true`); and it forbids `code_locus` on partial release rules (contradicting
Ade's open-obligation pattern). A full-history bisect shows it **FAILED at its
introduction (2026-03-15)**, was never stably green (only transient flickers May 19–21
during rapid registry build-out), and has been continuously red since 2026-05-25.
**Nothing invokes it** (no runner/workflow references it).

## Decision

`cross_ref` is a **directed** relation: "A cross_refs B" = A depends-on / refines /
is-evidence-for / should-be-read-with B. It does **not** imply B cross_refs A. The
useful invariants are **target existence** and **ID uniqueness**, not reciprocity.

**Retire** `ci_check_constitution_coverage.sh` and **fold its only load-bearing checks**
into the already-live, already-passing `ci_check_registry_code_locus_exists.sh`
(reframed as a *registry coherence* guard):
1. rule IDs are unique (append-only);
2. every `cross_ref` resolves to a real rule ID (directed; reciprocity NOT checked);
3. every `code_locus` `crates/**.rs` + `ci/**.sh` path exists (the original drift guard).

## What was lost by retiring: nothing net

Per a check-by-check audit, the retired gate's other checks were either **stale/wrong
for Ade** (bidirectionality, `KNOWN_CLUSTERS` allowlist, tier-by-prefix, release-partial-
forbidden-fields, CN-required-fields), **redundant** (TOML parse — every registry-reading
gate already fails on malformed TOML), **not-currently-enforcing** (`enforced`⇒evidence
was itself red on 41 entries), or **unrunnable in CI** (`$HOME` coverage docs). The only
two genuinely-valuable, not-covered-elsewhere checks — unique IDs and cross_ref
resolution — are preserved by the fold above. Both currently hold (299 unique IDs, 0
dangling cross_refs), so retirement breaks nothing and keeps the guard against future
regressions.

## Residual / follow-up (separate change)

- **`T-CI-01`** ("Every true invariant has mechanical CI enforcement") had
  `ci_script = "ci/ci_check_constitution_coverage.sh"` — now a **dangling pointer** to
  the retired gate. It is `status = "partial"` with `code_locus = "ci/"` (no `.sh`
  token), so the kept coherence gate stays green; but the reference is stale. Reconciling
  it (clear/repoint the `ci_script`, and honestly reflect that "every true invariant has
  CI" has no real enforcer today) is a **registry semantic decision** — its own small
  change, not bundled here.
- An optional Ade-native **`enforced`⇒evidence** check could be added later, but it needs
  the `ci_script` vs `ci_scripts` field convention sorted first (the 41 failures suggest
  a field-name mismatch).
