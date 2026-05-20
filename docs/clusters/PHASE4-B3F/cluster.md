# Invariant Cluster — PHASE4-B3F (B3 follow-up: DC-TXV-06 mechanical closure)

> **Status:** Planning Artifact (Non-Normative). Single-slice follow-up to the closed
> PHASE4-B3 cluster. Authority: `docs/ade-invariant-registry.toml`.

### Cluster PHASE4-B3F — Mechanically gate the Conway cert-classification closure

**Primary invariant:**
> The closed/total Conway certificate-deposit classification (`DC-TXV-06`) is enforced
> **mechanically by CI**, not only by the compiler-exhaustive match + named tests — a
> reintroduced catch-all accept arm, an open-tail (`Other`/`Unknown`) cert variant, a
> `#[non_exhaustive]`, or a `_ =>` wildcard in `classify` fails CI.

**Normative anchors:** `docs/ade-invariant-registry.toml` — `DC-TXV-06` (flips `partial → enforced`).

**Entry conditions:** PHASE4-B3 closed. The cert surface already satisfies the target
properties (decoder `_ => Err(UnknownCertTag)`, closed `ConwayCert`, exhaustive `classify`);
this cluster adds the CI gate that defends them against regression. **No cert-code change.**

**Exit Criteria (CI-verifiable):**
- [ ] **CE-B3F-1** — `ci/ci_check_conway_cert_classification_closed.sh` exits 0 on HEAD AND
  exits 1 when any of these anti-patterns is injected (verified by a documented probe and
  reverted): a catch-all arm constructing a `ConwayCert` in the decoder; an `Other`/`Unknown`
  open-tail variant or `#[non_exhaustive]` on the cert types; a `_ =>` wildcard in `classify`.
  `docs/ade-invariant-registry.toml` `DC-TXV-06` → `status = "enforced"`,
  `ci_script = "ci/ci_check_conway_cert_classification_closed.sh"`; `ci_check_constitution_coverage.sh` still PASSES.

**TCB color map:** the gate is a CI script (off the BLUE authority path; enforcement tooling).
No BLUE/GREEN/RED code changes.

**Forbidden during this cluster:**
- Changing the cert decoder/classifier/types to make the gate pass (they already pass; a code
  change would mean the gate is wrong).
- A theatrical gate that cannot fail — the probe (gate exits 1 on injected violation) is mandatory.
- Marking `DC-TXV-06` `enforced` without a `ci_script` that actually greps the cert surface.

## Authority Reminder
Planning aid. Authority: `docs/ade-invariant-registry.toml` + Cardano ledger spec.
