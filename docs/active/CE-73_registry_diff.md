# CE-73 Registry Diff

> **Status**: draft for review. Proposed change to
> `docs/ade-invariant-registry.toml`. Apply only on approval.

## Context

CE-73 is a doctrinal exit criterion documented in
`docs/active/phase_2c_progress_report.md` and referenced in
`docs/active/phase_3_status.md`. It is **not** itself a `[[rules]]`
entry in the registry. The registry tracks `T-` / `CN-` / `DC-` rules,
not CE labels.

The rule that carries CE-73's scope is **`DC-EPOCH-02`** at
`docs/ade-invariant-registry.toml:662-673`. Its statement is "Hard fork
transitions triggered at deterministic slot/epoch boundaries; era
translation functions mandatory; forecast horizon extends to era
boundary." Its `authority_surface` already mentions the byte-parity
non-goal.

The CE-73 reclassification (see
`docs/active/CE-73_reclassification.md`) splits the doctrinal claim
into:
- **CE-73-semantic** (closed, Tier 2)
- **CE-73-bytes** (explicit non-goal, Tier 4)

In the registry, that split is reflected by **flipping
`DC-EPOCH-02.status`** from `partial` to `enforced` and tightening the
`authority_surface` text to make the split explicit.

## Why `enforced`, not `partial`?

The schema's three status values are `declared`, `partial`, `enforced`.
Post-reclassification:
- The semantic translation claim is fully proven (22/22 fields).
- The byte-parity claim is **explicit non-goal**, not "open work."
- Consensus-side HFC scheduling is **Phase 4**, tracked separately
  under the new `phase_4_cluster_plan.md` (cluster N-B).

There is no remaining "open work" inside DC-EPOCH-02's *ledger-side*
scope. The `partial` status was carrying CE-73-bytes as implied open
work; reclassification removes that implication.

`enforced` is correct under the doctrinal split. The Phase 4
consensus-side concerns get their own DC entries (TBD in Phase 4
planning) rather than being smuggled inside DC-EPOCH-02 as a partial
residual.

## Diff

### Before (current state)

```toml
[[rules]]
id = "DC-EPOCH-02"
tier = "derived"
statement = "Hard fork transitions triggered at deterministic slot/epoch boundaries; era translation functions mandatory; forecast horizon extends to era boundary"
source = "Project constitution §3, T-DET-01, T-CORE-03"
cross_ref = []
code_locus = "crates/ade_ledger/src/hfc.rs"
tests = ["translation_summary_proof", "translation_comparison_surface", "transition_proof_surface"]
ci_script = ""
status = "partial"
authority_surface = "all 6 HFC ledger-side translations implemented; 22/22 encoding-independent fields match oracle; consensus-side HFC deferred to Phase 4; byte-parity with Haskell on-disk ExtLedgerState deliberately not pursued (see docs/active/CE-79_gate_statement.md Tier 4)"
cluster = "CL-LEDGER-VERDICT"
```

### After (proposed)

```toml
[[rules]]
id = "DC-EPOCH-02"
tier = "derived"
statement = "Hard fork transitions triggered at deterministic slot/epoch boundaries; era translation functions mandatory; forecast horizon extends to era boundary"
source = "Project constitution §3, T-DET-01, T-CORE-03"
cross_ref = []
code_locus = "crates/ade_ledger/src/hfc.rs"
tests = ["translation_summary_proof", "translation_comparison_surface", "transition_proof_surface"]
ci_script = ""
status = "enforced"
authority_surface = "ledger-side HFC translation enforced: all 6 transitions implemented, 22/22 encoding-independent fields match oracle (CE-73-semantic, Tier 2 closed). Byte-parity with Haskell on-disk ExtLedgerState CBOR is explicit non-goal (CE-73-bytes, Tier 4 per docs/active/CE-79_gate_statement.md and docs/active/CE-73_reclassification.md). Consensus-side HFC scheduling tracked separately under Phase 4 cluster N-B."
cluster = "CL-LEDGER-VERDICT"
evidence = [
  "docs/active/phase_2c_progress_report.md",
  "docs/active/CE-73_reclassification.md",
  "docs/active/T-26_hfc_ledger_side.md",
]
```

### Changed fields summary

| Field | Before | After |
|---|---|---|
| `status` | `"partial"` | `"enforced"` |
| `authority_surface` | mentions non-goal but in mixed prose | explicit split: CE-73-semantic enforced + CE-73-bytes non-goal + Phase 4 carve-out |
| `evidence` | (absent) | added — points at progress report, reclassification doc, T-26 slice doc |

No change to: `id`, `tier`, `statement`, `source`, `cross_ref`,
`code_locus`, `tests`, `ci_script`, `cluster`.

## Verification

After applying:
- `grep -A 12 'id = "DC-EPOCH-02"' docs/ade-invariant-registry.toml`
  shows the new state.
- `ci/ci_check_constitution_coverage.sh` passes (no CE-73 specific
  CI; the change is metadata).
- The status flip is consistent with the existing `feedback_hard_closure_gates.md`
  rule "use partial aggressively and honestly" — partial is no longer
  honest because there's no open work, only an explicit non-goal.

## Side effects

- The Phase 3 status doc (`docs/active/phase_3_status.md`) references
  CE-73 as "OPEN" indirectly via Phase 2C residual. After this
  registry update, that doc should be updated in a follow-up to
  reflect:
  - CE-73-semantic: closed
  - CE-73-bytes: non-goal (Tier 4)
- The CE-73 reclassification doc
  (`docs/active/CE-73_reclassification.md`) has a registry-update
  section that proposes fictional `[exit_criteria.*]` sections.
  That section should be replaced with a pointer to this diff doc.

## Apply checklist

- [ ] Edit `docs/ade-invariant-registry.toml:662-673` per the diff above.
- [ ] Update `docs/active/CE-73_reclassification.md` "Registry update"
  section to reference this diff doc instead of the fictional
  `[exit_criteria.*]` schema.
- [ ] Update `docs/active/phase_3_status.md` Phase 2C residual section
  (if it implies CE-73 is open, mark it reclassified).
- [ ] Run `ci/ci_check_constitution_coverage.sh` to confirm no breakage.
- [ ] Commit with message
  `docs(phase-2c): close DC-EPOCH-02 via CE-73 reclassification (semantic enforced, bytes Tier 4)`.
