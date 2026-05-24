# Invariant Slice — B3F-S2

## 2. Slice Header

### Slice Name
Conway cert decoder strictness — reject trailing bytes + bound preallocation

### Cluster
PHASE4-B3F (B3 follow-up)

### Status
Merged

### Cluster Exit Criteria Addressed
- [x] **CE-B3F-2** — `decode_conway_certs` rejects trailing bytes after the cert array
  (`CodecError::TrailingBytes`, parity with `decode_withdrawals`) and bounds its
  preallocation by remaining input; covered by `trailing_bytes_after_cert_array_rejected`
  and `huge_array_count_rejects_without_overallocating`; existing decoder + conservation
  tests stay green.

### Slice Dependencies
PHASE4-B3 (closed) — the cert decoder this hardens. Independent of B3F-S1.

---

## 4. Intent
Close the strictness asymmetry both PHASE4-B3 reviewers flagged: `decode_conway_certs`
silently ignored trailing bytes after the cert array (unlike `decode_withdrawals`), and its
`Vec::with_capacity(n)` trusted the declared CBOR count. The cert field is an exact CBOR
item; trailing bytes are malformed input and a crafted huge count must not drive an
allocation. Strengthens `DC-VAL-06` (fail-closed, no silent skip) and the closed-grammar posture.

---

## 5. Scope
- **MODIFIED** `crates/ade_codec/src/conway/cert.rs::decode_conway_certs`:
  - reject `offset != data.len()` after the array (and the indefinite break) with
    `CodecError::TrailingBytes { consumed, total }`;
  - `Vec::with_capacity((n as usize).min(data.len()))` — a CBOR array of `n` elements needs
    ≥ `n` bytes, so the cap cannot under-allocate for valid input and defangs a crafted count.
- **NEW tests** in `crates/ade_codec/tests/conway_cert_classification.rs`.
- **MODIFIED** `docs/ade-invariant-registry.toml` — `DC-VAL-06` tests/`strengthened_in`.
- **No behavioral change for valid input** — the real cert slice (tx-body key 4) is an exact
  item; conservation + corpus paths unaffected.

---

## 6. Execution Boundary
- **BLUE:** `ade_codec::conway::cert` (the decoder). No GREEN/RED change.

---

## 7. Invariants Preserved
- `DC-TXV-06` (cert-classification closure) — unaffected; the cert-closure gate still passes.
- Conservation (`T-CONSERV-01`) + the B3 corpora — re-verified green (decoder accepts the same
  valid inputs; only trailing-byte / oversize-count malformations now reject).

---

## 8. Invariants Strengthened
- **`DC-VAL-06`** (fail-closed / no silent skip) — the cert decoder now rejects trailing bytes
  and a crafted oversize count, matching the withdrawals decoder. `strengthened_in += PHASE4-B3F`.

---

## 9. Design Summary
Mirror `decode_withdrawals`: after consuming the definite array (or the indefinite array's
break byte), require `offset == data.len()` or reject `TrailingBytes`. Cap the preallocation at
`data.len()`; the per-element loop still validates every cert and hits `UnexpectedEof` if the
data runs out, so a huge declared count rejects deterministically without a large allocation.

---

## 11. Replay / Crash / Epoch Validation
- Not applicable (pure decoder). Mechanical proof = the two negative tests + the unchanged
  positive/replay decoder tests.

---

## 12. Mechanical Acceptance Criteria
- [x] `cargo test -p ade_codec --test conway_cert_classification` PASS (7 tests incl.
  `trailing_bytes_after_cert_array_rejected`, `huge_array_count_rejects_without_overallocating`).
- [x] `cargo test -p ade_ledger --test conway_conservation_full --test conway_conservation_adversarial`
  + `ade_testkit --test conway_conservation_positive_corpus` stay green (valid input unaffected).
- [x] `ci/ci_check_conway_cert_classification_closed.sh` still PASS (no catch-all/wildcard introduced).
- [x] `ci/ci_check_constitution_coverage.sh` PASS.

---

## 14. Hard Prohibitions
- No change to which valid certs decode (only malformed trailing/oversize input newly rejects).
- No catch-all accept arm or `_ =>` wildcard introduced (cert-closure gate guards this).
- No AI-style source comments. Commit with the repo trailer.

---

## 15. Explicit Non-Goals
- Conway block-body vkey-witness closure (separate, B2-carried).
- Any conservation-equation or deposit-accounting change.

---

## 18. Authority Reminder
Planning/review aid. Authority: `docs/ade-invariant-registry.toml` + Cardano ledger spec.
