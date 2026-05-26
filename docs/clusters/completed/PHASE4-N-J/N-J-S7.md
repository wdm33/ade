# Invariant Slice — PHASE4-N-J S7

## Slice Header

**Slice Name:** combined snapshot framing — version tag + fingerprint cross-check
**Cluster:** PHASE4-N-J
**Status:** In Progress
**CEs addressed:** CE-N-J-7
**Registry effects on merge:**
  * DC-STORE-08 → `enforced` (9 tests + CI script)
  * DC-STORE-09 → `enforced` (3 tests + CI script)
  * CN-STORE-08 → `enforced` (CI script)
**Dependencies:** S1 (chain_dep), S6 (ledger assemble).

---

## Intent

The framing layer that lets a `(LedgerState, PraosChainDepState)`
pair ride as a single byte blob through `SnapshotStore` (from
PHASE4-N-D) — restart-safe rollback becomes possible at S8.

* `ade_ledger::snapshot::framing::encode_snapshot` /
  `decode_snapshot` — SOLE pub authority for combined snapshot
  bytes. Wire shape:
  ```text
  array(4) [
    uint      version,             // == SCHEMA_VERSION == 1
    bytes(32) source_fingerprint,  // fingerprint(ledger).combined
    bytes     ledger_state_bytes,  // S6 encode_ledger_state output
    bytes     chain_dep_bytes,     // S1 encode_chain_dep output
  ]
  ```
* `SCHEMA_VERSION: u32 == 1` lives only in `framing.rs`. Future
  schema changes bump this constant under a separate cluster.
* Decoder reads + verifies the version tag **before** touching the
  payload (DC-STORE-09): unknown version → `UnknownVersion {
  expected, found }`. After payload decode, recomputes
  `fingerprint(decoded_ledger).combined` and compares with the
  embedded hash (DC-STORE-08): mismatch →
  `FingerprintMismatch { expected, actual }`.
* Conway-only scope discipline preserved at the framing layer too
  (pre-Conway encode → `EraNotSupported`; pre-Conway decode also
  rejects at the inner `decode_ledger_state` boundary).

### CI gate — `ci/ci_check_snapshot_encoder_closure.sh`

Mechanical CN-STORE-08 + DC-STORE-08 + DC-STORE-09 enforcement:

1. Combined `encode_snapshot` / `decode_snapshot` exist as `pub fn`
   only inside `framing.rs`.
2. `encode_ledger_state` / `decode_ledger_state` exist only inside
   `ledger.rs`.
3. `encode_chain_dep` / `decode_chain_dep` exist only inside
   `chain_dep.rs`.
4. `pub const SCHEMA_VERSION` exists only inside `framing.rs`.
5. `FingerprintMismatch` and `UnknownVersion` are both referenced
   from `framing.rs` (proves the cross-check + version-check
   wiring exists, not just the variants).

### Registry flips

`DC-STORE-08`, `DC-STORE-09`, `CN-STORE-08` move from `declared`
to `enforced`, with `code_locus`, `tests`, and `ci_script` fields
populated. The `tests` arrays for DC-STORE-08 collect the
9 determinism/round-trip tests across S1-S7 — they were always
the right enforcement set but only become claimable now that
the framing layer ties them together.

---

## §12 Mechanical Acceptance Criteria

- `snapshot_round_trip` — encode → decode preserves both halves.
- `snapshot_encode_deterministic_across_runs` — byte-identical
  encoder.
- `decode_rejects_unknown_version` — byte-patched version=2 →
  `UnknownVersion { expected: 1, found: 2 }`.
- `decode_rejects_fingerprint_mismatch` — byte-patched fingerprint
  → `FingerprintMismatch`.
- `encode_pre_conway_era_rejected` — Babbage `LedgerState` at the
  framing layer → `EraNotSupported`.
- `round_trip_via_fingerprint_combined` — decoded state's combined
  fingerprint equals the source's.
- `ci/ci_check_snapshot_encoder_closure.sh` exits 0 against
  current `crates/` tree.

---

## §14 Hard Prohibitions

- No HashMap/HashSet/wall-clock/tokio/rand/float literals in
  `snapshot::framing`.
- No parallel `SCHEMA_VERSION` constant outside `framing.rs`.
- No parallel `encode_snapshot` / `decode_snapshot` / similar pub
  fn outside `framing.rs` — CI grep enforces.
- No pre-Conway encode success path at the framing layer.

---

## §15 Explicit Non-Goals

- Persistent cache wiring (S8) — bridging the framing layer to
  `SnapshotStore` + closing DC-CONS-21 is S8's work.
- Schema-version migration paths — version stays at 1 in this
  cluster; bumps live in a future cluster.
- Cross-impl byte equivalence — deferred to a separate compare
  pass on top of the persistent cache fixture in S8.
