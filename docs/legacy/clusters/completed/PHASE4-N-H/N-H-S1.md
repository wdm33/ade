# Invariant Slice — PHASE4-N-H S1

## Slice Header

**Slice Name:** `AdmittedBlock` private-constructor token + `ReceiveEvent` / `ReceiveEffect` / `ReceiveError` closed sums + `PendingHeaderCache` + `ChainDbWrite` narrow trait
**Cluster:** PHASE4-N-H
**Status:** In Progress
**CEs addressed:** CE-N-H-1
**Registry flips on merge:** `CN-PROTO-07` → `enforced`
**Dependencies:** PHASE4-N-C (precedent for private-constructor admission tokens); PHASE4-N-D (`ChainDb` types)

---

## Intent

Stand up the BLUE skeleton the receive bridge will compose:

* `AdmittedBlock` — opaque-bytes admission token whose only
  constructor is `admit_via_block_validity` (lives next to
  `block_validity`'s site so it can read the verdict directly).
  Distinct from `AcceptedBlock` (producer-side broadcast token) so
  cross-use is mechanically impossible.
* `ReceiveEvent` — closed sum lifting the receive subset of N-A
  signals/events: `RollForward`, `RollBackward`, `BlockDelivered`.
  Locally-originated chain-sync/block-fetch outputs (client requests
  the orchestrator sends) are NOT constructible here — that's the
  CN-PROTO-07 closure.
* `ReceiveEffect` — closed report sum: `Admitted`, `Cached`,
  `RolledBack` (unused at S1 — reducer returns `RollbackOutOfScope`
  instead), `NoOp`.
* `ReceiveError` — closed: `HeaderBodyMismatch`,
  `Validity(BlockValidityError)`, `RollbackOutOfScope { target_point }`,
  `ChainDb(ChainDbError)`.
* `PendingHeaderCache` — `BTreeMap<(SlotNo, Hash32), Vec<u8>>` with
  canonical iteration.
* `ChainDbWrite` — narrow trait the reducer calls on the admit
  branch; takes `AdmittedBlock` by value so admission cannot be
  faked.

The reducer itself lands in S2.

---

## The change

### 1. New module tree `crates/ade_ledger/src/receive/`

```
receive/
  mod.rs                      // pub use re-exports
  admitted.rs                 // AdmittedBlock + admit_via_block_validity
  events.rs                   // ReceiveEvent / ReceiveEffect / ReceiveError
  pending_header_cache.rs     // PendingHeaderCache
  chain_write.rs              // ChainDbWrite trait + ChainWriteError
```

Each module follows the project's BLUE conventions: closed enums,
`BTreeMap`-only iteration, no `String` in error sums, no
`#[non_exhaustive]`.

### 2. CI gate `ci/ci_check_admitted_block_closure.sh`

Forbids `pub fn .* -> *AdmittedBlock` and `impl .* AdmittedBlock` /
`pub struct AdmittedBlock` outside the canonical site
`crates/ade_ledger/src/receive/admitted.rs`. Forbids re-export of
the inner `AdmittedBlock(...)` tuple constructor as `pub`.

---

## §12 Mechanical Acceptance Criteria (named tests)

In `crates/ade_ledger/src/receive/{admitted, events, pending_header_cache}.rs`:

- `admit_via_block_validity_accepts_corpus_block` — corpus block →
  `Ok(AdmittedBlock)` whose `.as_bytes()` equals the input bytes.
- `admit_via_block_validity_rejects_corrupted_body` — flipped-body
  bytes → `Err(BlockValidityError::BodyHashMismatch { .. })`.
- `admitted_block_as_bytes_is_subslice_of_input` — pointer-arithmetic
  check.
- `receive_event_round_trips_through_pattern_match` — exhaustive
  match over `ReceiveEvent` enumerates only the 3 variants.
- `receive_effect_round_trips_through_pattern_match` — same for the 4
  effect variants.
- `receive_error_round_trips_through_pattern_match` — same for the 4
  error variants.
- `pending_header_cache_insert_and_lookup` — basic put/get.
- `pending_header_cache_iteration_is_btreemap_ordered` — sorted-key
  iteration witness.
- `chain_write_trait_admits_via_admitted_block` — trait test against
  a hand-rolled mock impl; the mock receives the admitted bytes
  byte-identically.

CI: `ci/ci_check_admitted_block_closure.sh` (new).

---

## §14 Hard Prohibitions

- No `pub fn` returning `AdmittedBlock` outside
  `ade_ledger::receive::admitted::admit_via_block_validity`.
- No public field on `AdmittedBlock`; only `as_bytes(&self)` /
  `into_bytes(self)` accessors.
- No `pub` re-export of `AdmittedBlock`'s tuple struct constructor.
- No `HashMap` / `HashSet` / wall-clock / tokio / rand in any of
  the new modules.
- No `impl From<Vec<u8>> for AdmittedBlock` — bytes must flow only
  via `admit_via_block_validity`.

---

## §15 Explicit Non-Goals

- The `receive_apply` reducer — S2.
- The GREEN adapter — S3.
- The RED orchestrator — S4.
- Mechanical cross-impl + live evidence — S5/S6.
- Ledger rollback (Path A scope edge).

---

## Replay obligations

No new corpus at this slice; the types are skeletons. S3 introduces
the receive-paths replay corpus.

---

## Authority reminder

If this slice conflicts with the project's normative specifications
or the invariant registry, those win.
