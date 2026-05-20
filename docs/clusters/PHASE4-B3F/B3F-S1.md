# Invariant Slice — B3F-S1

## 2. Slice Header

### Slice Name
DC-TXV-06 CI grep-gate — defend the Conway cert-classification closure

### Cluster
PHASE4-B3F (B3 follow-up)

### Status
Merged

### Cluster Exit Criteria Addressed
- [ ] **CE-B3F-1** — `ci/ci_check_conway_cert_classification_closed.sh` exits 0 on HEAD and
  exits 1 on each injected anti-pattern (probe, reverted); `DC-TXV-06` flips
  `partial → enforced` with this script as its `ci_script`; `ci_check_constitution_coverage.sh`
  stays PASS.

### Slice Dependencies
PHASE4-B3 (closed) — the cert surface this gate guards.

---

## 4. Intent
Make `DC-TXV-06`'s closure mechanically enforced by CI rather than resting only on the
compiler-exhaustive `match` + named tests: a future edit that reintroduces a catch-all
accept arm, an open-tail cert variant, or a `_ =>` wildcard in `classify` must fail CI.

---

## 5. Scope
- **NEW** `ci/ci_check_conway_cert_classification_closed.sh` (CI tooling).
- **MODIFIED** `docs/ade-invariant-registry.toml` — `DC-TXV-06` `status` → `enforced`,
  `ci_script` set.
- **No code change** to `ade_codec::conway::cert`, `ade_ledger::cert_classify`, or
  `ade_types::conway::cert` — they already satisfy the gate.

---

## 6. Execution Boundary
- BLUE / GREEN / RED: none changed. The gate is enforcement tooling off the authority path.

---

## 7. Invariants Preserved
- All B3 invariants (the gate adds enforcement; changes no behavior).
- `ci_check_constitution_coverage.sh` must stay green (registry consistency).

---

## 8. Invariants Strengthened
- **`DC-TXV-06`** — enforcement gains a CI grep-gate; `partial → enforced`. On merge, append
  `PHASE4-B3F` to `strengthened_in` and set `ci_script`.

---

## 9. Design Summary
The gate (`set -uo pipefail`, `FAIL=0`, structured `FAIL:`/`PASS:` lines, exit 1 on any
violation) checks three properties over the cert surface:
1. **Closed enums** — `crates/ade_types/src/conway/cert.rs`: no `#[non_exhaustive]`; no
   open-tail variant matching `^\s+(Other|Unknown)\s*[{(,]` in `ConwayCert`/`CertDisposition`/
   `DepositEffect`/`CoinSource`.
2. **Decoder rejects unknowns, no catch-all accept** — `crates/ade_codec/src/conway/cert.rs`:
   `CodecError::UnknownCertTag` present; forbid any catch-all arm constructing a cert
   (`_ =>` … `ConwayCert::` / `_ => Ok(ConwayCert::` / `_ => Some(ConwayCert::`), the
   reintroduced-Shelley-fallback anti-pattern.
3. **Classify stays exhaustive** — `crates/ade_ledger/src/cert_classify.rs`: the `classify`
   match has no `_ =>` wildcard arm (so a new `ConwayCert` variant breaks the build).

Comment-only lines are stripped so docstrings describing the prohibition don't trip the gate.

---

## 11. Replay / Crash / Epoch Validation
- Not applicable (CI tooling, no state). Mechanical proof = the gate + its must-fail probe.

---

## 12. Mechanical Acceptance Criteria
- [ ] `bash ci/ci_check_conway_cert_classification_closed.sh` exits 0 on HEAD.
- [ ] **Probe (mandatory):** injecting each of (a) `_ => ConwayCert::StakeDelegation` in the
  decoder, (b) an `Unknown {}` variant in `ConwayCert`, (c) a `_ =>` arm in `classify` each
  makes the gate exit 1; all probes reverted, gate back to 0.
- [ ] `bash ci/ci_check_constitution_coverage.sh` exits 0 (DC-TXV-06 now `enforced` with a `ci_script`).
- [ ] `cargo build --workspace --exclude ade_plutus` still 0 errors (no code touched).

---

## 14. Hard Prohibitions
- No change to cert decoder/classifier/types to satisfy the gate.
- No theatrical gate — the must-fail probe is required.
- No `enforced` status without a real, cert-surface-grepping `ci_script`.
- No AI-style source comments. Do not commit until green (then with the repo trailer).

---

## 15. Explicit Non-Goals
- The Conway block-body vkey-witness closure gap (separate, B2-carried).
- The cert-decoder trailing-bytes strictness parity smell (separate follow-up).
- Any change to the conservation equation or deposit accounting.

---

## 18. Authority Reminder
Planning/review aid. Authority: `docs/ade-invariant-registry.toml` + Cardano ledger spec.
