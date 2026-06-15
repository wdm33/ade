# Slice MEM-MEASURE-A1 — Bounded inbound admission + memory-measurement substrate

### Cluster
MEM-MEASURE (primary invariant for this slice: `CN-MEM-01`).

### Status
In Progress.

### Cluster Exit Criteria Addressed
- [ ] CE-MM-1 (`CN-MEM-01` deterministic bounded inbound admission)
- [ ] CE-MM-2 (memory-measurement substrate: evidence schema + RED RSS sampler + replay-fingerprint pairing)

Exit criteria not listed here (CE-MM-3 `OP-MEM-01` live artifact, CE-MM-4 live wiring, CE-MM-5/6) are explicitly out of scope for this slice.

### Slice Dependencies
None. Hermetic — no live peer, no C2-LOCAL venue.

---

## 3. Implementation Instruction (AI)
Implement exactly what is specified. Create the measurement and bounded-admission proof substrate **without depending on a live peer**. No BLUE change. The bounded gate fronts the existing BLUE `mempool_ingress`; it never modifies it and never changes a verdict. The RED RSS sampler observes process memory and influences no authoritative output. Refer to §14 Hard Prohibitions and §15 Explicit Non-Goals before writing any code. §12 Mechanical Acceptance Criteria is the only proof of completion.

> Commit message follows this repo's override: it MUST carry the `Co-Authored-By: Claude <model+context>` trailer (CLAUDE.md). Source comments carry no AI attribution.

---

## 4. Intent
Make it impossible for untrusted inbound work to consume the scarce authoritative resource (BLUE mempool/tx validation) without first passing a **deterministic bounded policy**, and make every memory measurement of Ade **paired with a replay fingerprint + verdict** so that low-memory evidence which silently changed an authoritative output is mechanically rejected. This slice flips `CN-MEM-01 declared→partial`.

---

## 5. Scope
- **Modules / crates:**
  - `crates/ade_node/src/mem_measure/mod.rs` (new module root)
  - `crates/ade_node/src/mem_measure/bounded_admission.rs` (new, GREEN)
  - `crates/ade_node/src/mem_measure/rss_sampler.rs` (new, RED)
  - `crates/ade_node/src/mem_measure/evidence.rs` (new, GREEN)
  - `crates/ade_node/src/mem_measure/runner.rs` (new, GREEN/RED seam)
  - `crates/ade_node/src/lib.rs` (register `pub mod mem_measure;`)
  - `ci/ci_check_bounded_inbound_admission.sh` (new gate)
  - `docs/ade-invariant-registry.toml` (`CN-MEM-01` `declared→partial`)
- **State machines affected:** none.
- **Persistence impact:** none (hermetic; no WAL/checkpoint change — the `wal_checkpoint_fp` field is a fingerprint *over* the authoritative output, populated live in A2).
- **Network-visible impact:** none.
- **Out of scope:** any `--mode node` / C2-LOCAL run; live wiring of the bounded queue (B); `CN-MEM-03` shedding policy; `OP-MEM-01`; Haskell comparison; performance tuning.

---

## 6. Execution Boundary
- **BLUE (deterministic, authoritative):** none. The slice references `ade_ledger::mempool::{mempool_ingress, AdmitOutcome, IngressEvent, MempoolState}` and `ade_ledger::state::LedgerState` **read-only** (calls `mempool_ingress`; constructs no new authoritative type).
- **GREEN (deterministic glue, non-authoritative):** `bounded_admission.rs` (the bounded fold) + `evidence.rs` (record schema, `validate_evidence`, `pair_replay`, blake2b fingerprint). No clock / RNG / `HashMap`/`HashSet` / float / `std::fs` / `/proc`.
- **RED (nondeterministic shell, observe-only):** `rss_sampler.rs` — the single site that reads `/proc/self/status`. `runner.rs` is the GREEN/RED seam: it drives the GREEN workload + GREEN fingerprinting and calls the RED sampler, writing RSS only into the record's *observational* fields.

**Resolution:** colors are unambiguous and file-separated. The RED sampler's output flows only into non-authoritative evidence fields; the replay verdict is computed purely from the fingerprint pairing.

---

## 7. Invariants Preserved
- `DC-MEM-01` — mempool acceptance ⊆ block/ledger acceptance (the gate never changes a verdict).
- `DC-MEM-03` — `mempool_ingress` stays the single closed chokepoint; `source` is metadata only.
- `DC-MEM-04` — the ingress trace replays byte-identically (the bounded fold below the cap is byte-identical to `replay_ingress_trace`).
- Determinism of the authoritative core: no clock/RNG/HashMap/float introduced into any deterministic path.

---

## 8. Invariants Strengthened or Introduced
- **`CN-MEM-01` `declared→partial`:** untrusted inbound work is admitted through a deterministic bounded policy (closed per-batch count + byte budget, head-of-line forward/shed) **before** the BLUE `mempool_ingress` validation is consumed; the count of events reaching the authoritative path is `≤` the fixed budget regardless of input length.
- **New measurement discipline (CE-MM-2):** an Ade memory measurement is valid evidence only when paired with a replay fingerprint whose verdict is `Agreed`; RSS magnitude never gates.

---

## 9. Design Summary
- `bounded_admission.rs` (GREEN): two fixed closed constants `MAX_INBOUND_ADMISSION_COUNT` + `MAX_INBOUND_ADMISSION_BYTES`. `replay_bounded_ingress_trace(base, events)` folds the canonical ordered trace: while both budgets hold, the event is **`Forwarded`** to `mempool_ingress` (inner `AdmitOutcome` is the unchanged BLUE verdict); the first event that would breach either budget — and every event after it — is **`Shed`** with a closed `ShedReason`. `BoundedOutcome`/`ShedReason` are closed enums. The number of `Forwarded` events (hence `mempool_ingress` calls) is `≤ MAX_INBOUND_ADMISSION_COUNT` and their cumulative bytes `≤ MAX_INBOUND_ADMISSION_BYTES`, always.
- `rss_sampler.rs` (RED): `sample_vm_rss_kib()` / `sample_vm_hwm_kib()` parse `VmRSS` / `VmHWM` from `/proc/self/status` (fail-soft `None` off-Linux/unreadable). `RssWindow` accumulates `u64` kiB samples and derives `p50/p95/peak` by integer nearest-rank — deterministic given the sample multiset; the samples themselves are nondeterministic OS values and never enter a fingerprint.
- `evidence.rs` (GREEN): `MemEvidenceRecord` (serde) with the full field set; `fingerprint_hex` = `blake2b_256` hex via `ade_crypto::blake2b`; `pair_replay(fp1, fp2) -> ReplayVerdict` (`Agreed` iff equal); `validate_evidence(&rec) -> Vec<EvidenceDefect>` checks structure + `verdict == Agreed` + percentile shape (`p50 ≤ p95 ≤ peak`) and **never references RSS magnitude** for pass/fail.
- `runner.rs` (GREEN/RED seam): `run_hermetic_bounded_ingress_measurement(...)` fingerprints the canonical input (`workload_hash`) and the authoritative output (`final_fingerprint` over `mempool.accepted()` + per-event disposition), runs the bounded fold twice under RSS sampling, pairs the two fingerprints into the verdict, and returns a populated `MemEvidenceRecord` (`venue = "hermetic"`).

---

## 10. Changes Introduced
### Types
- New (GREEN): `ShedReason` (`CountBudgetExhausted` | `ByteBudgetExhausted`), `BoundedOutcome` (`Forwarded(AdmitOutcome)` | `Shed(ShedReason)`), `MemEvidenceRecord`, `ReplayVerdict` (`Agreed` | `Diverged`), `EvidenceDefect`.
- New (RED): `RssSampleKib`, `RssWindow`.
- No new BLUE/canonical/persisted type.
### State Transitions
- None (no authoritative state machine changed).
### Persistence
- None.
### Removal / Refactors
- None.

---

## 11. Replay, Crash, and Epoch Validation
- **Replay tests:** `bounded_admission_is_deterministic` (same `(base, events)` → byte-identical `(MempoolState, Vec<BoundedOutcome>)`); `bounded_gate_under_budget_equals_unbounded` (below the cap, byte-identical to `replay_ingress_trace`); `hermetic_measurement_is_replay_stable` (the whole measurement re-run yields an identical `final_fingerprint`/`workload_hash` while the RSS fields may differ — proving RSS variation does NOT perturb the authoritative output).
- **Crash/restart:** n/a (hermetic, no persistence).
- **Epoch boundary:** n/a.

---

## 12. Mechanical Acceptance Criteria
Complete only when all exist and pass in CI (`cargo test -p ade_node` + the new gate):
- [ ] `bounded_admission_respects_count_budget` — `MAX+N` tiny events ⇒ exactly `MAX` `Forwarded`, `N` `Shed(CountBudgetExhausted)`, no `Forwarded` after the first `Shed`.
- [ ] `bounded_admission_respects_byte_budget` — large events ⇒ head-of-line `Shed(ByteBudgetExhausted)`; forwarded cumulative bytes `≤ MAX_INBOUND_ADMISSION_BYTES`.
- [ ] `bounded_admission_is_deterministic` — same inputs ⇒ byte-identical outputs.
- [ ] `bounded_gate_under_budget_equals_unbounded` — below the cap, identical to `replay_ingress_trace` over the B-track corpus (incl. the dependent pair).
- [ ] `bounded_gate_preserves_admit_verdict` — a forwarded event's inner `AdmitOutcome` equals a direct `mempool_ingress` call.
- [ ] `bounded_gate_no_false_accept_under_pressure` — an over-budget valid tx is `Shed` (not silently accepted); a forwarded adversarial tx stays `Rejected`.
- [ ] `validator_ignores_rss_magnitude` — two records differing only in RSS validate identically.
- [ ] `diverged_verdict_is_invalid_evidence` — a `Diverged` record yields a `VerdictNotAgreed` defect.
- [ ] `hermetic_measurement_verdict_is_agreed` + `hermetic_measurement_is_replay_stable` — the runner produces an `Agreed`, valid record; re-run is fingerprint-stable.
- [ ] RED sampler tests: `vm_rss_sample_present_on_linux`, `percentile_nearest_rank_is_deterministic`, `empty_window_yields_none`, `peak_is_max_of_samples`.
- [ ] `ci/ci_check_bounded_inbound_admission.sh` green: files exist; model symbols present; the fold calls `mempool_ingress`; GREEN files contain no forbidden nondeterministic/I-O constructs; `/proc/self/status` appears only in `rss_sampler.rs`; the required test names exist.
- [ ] `ci/ci_check_registry_code_locus_exists.sh` green with `CN-MEM-01`'s populated `code_locus`/`cross_ref`.

---

## 13. Failure Modes
- Off-Linux / unreadable `/proc`: the sampler returns `None` (fail-soft evidence gap), never panics — RSS is observational, not authoritative.
- A workload whose replay diverges: `replay_verdict = Diverged` ⇒ `validate_evidence` flags `VerdictNotAgreed` ⇒ the record is invalid evidence (fail-closed on the evidence contract). This is the load-bearing failure mode.
- Budget breach: deterministic `Shed` with a closed reason; no partial mutation; the authoritative path is simply not reached.

---

## 14. Hard Prohibitions
Inherits all cluster §8 prohibitions. Slice-specific:
- No `HashMap`/`HashSet`, wall-clock (`SystemTime`/`Instant`), RNG (`rand`/`thread_rng`), float (`f32`/`f64`), `std::fs`, or `/proc` access in the GREEN files (`bounded_admission.rs`, `evidence.rs`).
- No change to `admit`/`mempool_ingress`/`tx_validity` (BLUE untouched).
- No new feature flag / config switch; budgets are fixed closed constants.
- RSS magnitude must not enter any fingerprint, verdict, or validator pass/fail.
- No TODO/placeholder/deferred validation in the bounded fold.

---

## 15. Explicit Non-Goals
This slice MUST NOT:
- Run `--mode node` or any C2-LOCAL/preprod venue.
- Wire the bounded model into the live inbound path (that is B).
- Flip `OP-MEM-01`, `CN-MEM-03`, or `CN-MEM-01 partial→enforced`.
- Compare against the Haskell node.
- Optimize for performance or memory.
- Add any networking, consensus, storage, or protocol-version behavior.

---

## 16. Completion Checklist
- [ ] All new GREEN state is replay-derivable; the bounded fold is pure.
- [ ] The authoritative output is canonically fingerprinted; RSS is segregated into observational fields.
- [ ] All failure modes are deterministic (Shed reasons closed; sampler fail-soft).
- [ ] No TODO/placeholder in any deterministic path.
- [ ] `ci_check_bounded_inbound_admission.sh` enforces the bounded-admission + measurement invariants.
- [ ] Replay-equivalence tests pass across runs; `CN-MEM-01` registry row reads `partial` with populated enforcement.

---

## 17. Review Notes
- Invariant risk considered: a memory-saving change silently altering an authoritative output → mitigated by the mandatory replay-fingerprint pairing (`VerdictNotAgreed` invalidates).
- Assumption challenged: "bounded admission needs a config knob" → rejected; fixed closed budgets, per the `MAX_SERVE_RANGE_BLOCKS` precedent.
- Follow-up implied: B wires the model live (`partial→enforced`); A2 produces the live `OP-MEM-01` artifact.
