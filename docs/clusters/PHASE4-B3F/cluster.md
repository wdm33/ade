# Invariant Cluster — PHASE4-B3F (B3 follow-up hardening)

> **Status:** Planning Artifact (Non-Normative). Follow-up hardening to the closed
> PHASE4-B3 cluster. Authority: `docs/ade-invariant-registry.toml`.

### Cluster PHASE4-B3F — Defend the Conway cert grammar & classification closure

**Primary invariant:**
> The Conway certificate decode + classification stays closed/total and fail-closed,
> defended **mechanically** — (S1) a reintroduced catch-all accept arm, open-tail
> (`Other`/`Unknown`) cert variant, `#[non_exhaustive]`, or `_ =>` wildcard in `classify`
> fails CI (`DC-TXV-06`); (S2) the cert decoder rejects trailing bytes and a crafted
> oversize array count, matching the withdrawals decoder (`DC-VAL-06`).

**Normative anchors:** `docs/ade-invariant-registry.toml` — `DC-TXV-06` (flips
`partial → enforced`, S1) and `DC-VAL-06` (decoder strictness, S2).

**Slices:**
- **B3F-S1** — DC-TXV-06 CI grep-gate (`ci/ci_check_conway_cert_classification_closed.sh`). Merged.
- **B3F-S2** — cert decoder trailing-bytes reject + bounded preallocation (`DC-VAL-06`). Merged.

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
- [ ] **CE-B3F-2** — `decode_conway_certs` rejects trailing bytes (`CodecError::TrailingBytes`)
  and bounds its preallocation; covered by `trailing_bytes_after_cert_array_rejected` +
  `huge_array_count_rejects_without_overallocating`; existing decoder/conservation tests stay
  green; `DC-VAL-06` `strengthened_in += PHASE4-B3F`.

**TCB color map:** the gate is a CI script (off the BLUE authority path; enforcement tooling).
No BLUE/GREEN/RED code changes.

**Forbidden during this cluster:**
- Changing the cert decoder/classifier/types to make the gate pass (they already pass; a code
  change would mean the gate is wrong).
- A theatrical gate that cannot fail — the probe (gate exits 1 on injected violation) is mandatory.
- Marking `DC-TXV-06` `enforced` without a `ci_script` that actually greps the cert surface.

## Authority Reminder
Planning aid. Authority: `docs/ade-invariant-registry.toml` + Cardano ledger spec.
