# Cluster MEM-MEASURE — Memory-footprint evidence + bounded inbound admission

**Primary invariants:** `CN-MEM-01` (derived — deterministic bounded inbound admission) + `OP-MEM-01` (operational — mempool/peer pressure must not starve block validation, chain selection, or persistence). Anchored by the enforced floor `DC-MEM-01/02/03/04`.
**Status:** Planned — **A1 in flight** (hermetic substrate). **Stake-independent parallel track:** runs while the bounty live path is stake-gated (preview epoch 1330 / preprod epoch 296, see `project_c2_preprod_producer_readiness`). **Venue:** C2-LOCAL first (the A2+ rungs), NOT preprod.
**Tier split:** *true* — memory work must not violate determinism / replay / canonical bytes / structured errors. *derived* — bounded inbound admission + overload shedding stay deterministic (`CN-MEM-01`, later `CN-MEM-03`). *operational* — `OP-MEM-01` (no starvation under pressure). *release* — memory evidence becomes a repeatable artifact, not an anecdotal run.

## 1. Primary Invariants
- **`CN-MEM-01` (TARGET, derived):** *untrusted inbound work must be admitted through deterministic bounded policies before consuming scarce authoritative resources.* Today `declared` with empty `code_locus`/`tests`/`ci_script`. A1 flips it `declared→partial` with a hermetic bounded-admission model + proof; **B** (`MEM-BOUND-B`) wires it into the live `--mode node` inbound path and flips `partial→enforced`.
- **`OP-MEM-01` (TARGET, operational):** *mempool pressure and peer churn must not starve block validation, chain selection, or persistence (scheduling priority).* Today `declared`. **A2** flips it `declared→partial` with one committed C2-LOCAL memory-run artifact (RSS across the six measurement points, paired with recovered-tip + WAL/checkpoint fingerprint + replay verdict, proving no starvation). NOT touched by A1.
- **THE LOAD-BEARING DISCIPLINE:** every RSS measurement is paired with a **replay fingerprint + verdict** over the *authoritative* output. A low-memory run that silently changed chain selection, WAL shape, mempool ordering, or admission behavior is **INVALID evidence**. The verdict is computed only from the fingerprint pairing — never from the RSS numbers.

## 2. Normative Anchors
- Invariant registry `docs/ade-invariant-registry.toml`: `CN-MEM-01` (L1479), `CN-MEM-02` (L1492), `CN-MEM-03` (L1500), `CN-MEM-04` (L1513), `OP-MEM-01` (L1914); enforced floor `DC-MEM-01` (L863), `DC-MEM-02` (L875), `DC-MEM-03` (L887), `DC-MEM-04` (L900).
- `classification_table.md §H` (Mempool, Admission Control, and Overload Behavior); Project constitution §3 (determinism) + §4b (operational scheduling).
- Planning context `docs/planning/phase4-n-e-tier1-invariants.md` (DC-MEM-01 enforced; DC-MEM-02 / CN-MEM-01/03/04 held declared).
- **Existing asset reused as seed:** `crates/ade_testkit/src/mempool/ingress_replay.rs` (the DC-MEM-04 GREEN ingress-replay harness over the B-track adversarial corpus) + `ci/ci_check_mempool_ingress_replay.sh`.
- C2-LOCAL venue: the established `--mode node` single-producer-vs-real-cardano-node-relay recipe used by PHASE4-N-AH/N-AO (NOT preprod; the AWS node is offline-reference-only).

## 3. Entry Conditions (prior clusters guarantee)
- **The mempool admission core is enforced + reused unchanged (BLUE):** `admit` is a thin no-false-accept gate over `tx_validity` (`DC-MEM-01`); `mempool_ingress` is the single closed chokepoint, `source` is metadata only (`DC-MEM-03`); overload shedding follows deterministic policy (`DC-MEM-02`); the ingress trace replays byte-identically (`DC-MEM-04`). All four are `enforced` with CI gates.
- **A deterministic GREEN ingress-replay harness exists:** `replay_ingress_trace` folds `mempool_ingress` over a canonical `IngressEvent` trace, no clock/RNG/HashMap — the seed A1's bounded fold extends.
- **No memory-measurement prior art exists in the crates** — the RSS sampler is greenfield (built + unit-tested hermetically in A1 before it is trusted in the variable C2-LOCAL venue in A2).

## 4. What Changes (design) — measure + bound, never perturb
- **A1 — bounded-admission + measurement substrate (GREEN + RED, no BLUE). IN FLIGHT.** A deterministic bounded inbound-admission model (closed per-batch count+byte budgets, head-of-line forward/shed) fronting the BLUE `mempool_ingress`; the evidence-record schema; a RED `/proc/self/status` RSS sampler; the replay-fingerprint pairing. Hermetic, no live peer. Flips `CN-MEM-01 declared→partial`. *(See `A1-bounded-admission-measurement-substrate.md`.)*
- **A2 — live C2-LOCAL memory artifact (RED + GREEN evidence).** Drive a real `--mode node` C2-LOCAL run; sample RSS across the six measurement points (idle recovered-tip; ChainSync follow; BlockFetch serve; TxSubmission/mempool admission; WAL/checkpoint recovery; sustained rung-1); emit the A1 schema populated live, paired with recovered-tip identity + WAL/checkpoint fingerprint + replay verdict; prove no starvation. Flips `OP-MEM-01 declared→partial`. Operator-gated.
- **B (`MEM-BOUND-B`) — bounded inbound queues wired live.** Wire the A1 bounded-admission model into the live `--mode node` inbound path (deterministic bounded queues before authoritative validation). Flips `CN-MEM-01 partial→enforced`.
- **C (`MEM-STRESS-C`) — sustained pressure.** A C2-LOCAL sustained single-producer run through `>k` settlement + one epoch transition; memory stays bounded; replay verdict `Agreed` throughout.
- **D (`MEM-COMPARE-D`) — BA-08 comparison.** Side-by-side Haskell-node RSS at the same venue/workload (the bounty's average-memory-over-10-days criterion). The Haskell node is the behavior oracle, NOT an architecture template.

## 5. Exit Criteria (CE — each CI-verifiable)
- **CE-MM-1 (`CN-MEM-01` bounded admission) [A1]:** hermetic — a deterministic bounded-admission gate fronts `mempool_ingress`; the count of events that reach the authoritative path is `≤` a fixed closed budget regardless of input length; the gate is verdict-preserving (a forwarded event gets the same `AdmitOutcome` as a direct call) and transparent below the cap; no false-accept under pressure (an over-budget valid tx is *shed*, never silently accepted; a forwarded adversarial tx stays rejected); the fold is replay-stable. New gate `ci/ci_check_bounded_inbound_admission.sh` green; `cargo test -p ade_node` green. → `CN-MEM-01 declared→partial`.
- **CE-MM-2 (measurement substrate) [A1]:** hermetic — the evidence record carries `{scenario_id, git_sha, build_profile, venue, anchor, tip_before, tip_after, wal_checkpoint_fp, workload_hash, rss_p50/p95/peak, final_fingerprint, replay_verdict}`; the verdict is computed only from the fingerprint pairing; the validator rejects a `Diverged` record as invalid evidence and **ignores RSS magnitude**; the RED `/proc/self/status` read is confined to the sampler. Same gate + tests.
- **CE-MM-3 (`OP-MEM-01` live artifact) [A2]:** committed transcript `docs/evidence/mem-measure-a2-preprod-memory.{md,jsonl}` — a real `--mode node` C2-LOCAL run records RSS across the six measurement points, each sample paired with the recovered-tip identity + WAL/checkpoint fingerprint + replay verdict `Agreed`, proving no starvation of block validation / chain selection / persistence. → `OP-MEM-01 declared→partial`. `blocked_until_operator_c2local_memory_pass_executed`.
- **CE-MM-4 (`CN-MEM-01` closure) [B]:** the bounded-admission model is wired into the live `--mode node` inbound path before authoritative validation; a live-path bounded-queue test proves the bound holds under inbound pressure. → `CN-MEM-01 partial→enforced`.
- **CE-MM-5 (sustained) [C]:** a C2-LOCAL sustained run through `>k` settlement + one epoch transition keeps RSS bounded and the replay verdict `Agreed` throughout.
- **CE-MM-6 (BA-08 compare) [D]:** DONE — committed side-by-side comparison `docs/evidence/mem-compare-d-preprod.{md,jsonl}` (gate `ci_check_mem_compare_evidence.sh`): Ade `--mode admission` 6.56 GB vs Haskell `cardano-node-preprod` 5.50 GB on preprod, **verdict `ade_heavier` (+19.1%)** — Ade does NOT yet match/beat Haskell. The winning memory optimization (leaner UTxO) is the follow-on (see `D-haskell-rss-comparison.md` §17).
- **CE-MM-close [/cluster-close]:** registry flips recorded (`CN-MEM-01`, `OP-MEM-01`); the four grounding docs refreshed; cluster archived. Per-cluster security review clean.

## 6. Expected Slices
- **A1** bounded-admission + measurement substrate — CE-MM-1, CE-MM-2 — **GREEN + RED** (`ade_node::mem_measure`). Hermetic. **Lands first.**
- **A2** live C2-LOCAL memory artifact — CE-MM-3 — **RED + GREEN evidence** — operator-gated.
- **B** bounded inbound queues wired live — CE-MM-4 — **RED + GREEN** + BLUE-reused.
- **C** sustained pressure — CE-MM-5 — **RED** — operator-gated.
- **D** BA-08 comparison — CE-MM-6 — **RED** — operator-gated.
- **close** — CE-MM-close via `/cluster-close`.

## 7. TCB Color Map
- **BLUE (reused unchanged — zero new canonical type):** `ade_ledger::mempool::{admit, mempool_ingress}` + `ade_ledger::tx_validity`. The bounded gate *fronts* these; it never modifies them and never changes a verdict.
- **GREEN (deterministic, non-authoritative):** `ade_node::mem_measure::bounded_admission` (the bounded fold) + `ade_node::mem_measure::evidence` (record schema, validator, replay-fingerprint pairing). No clock / RNG / HashMap / float / I/O.
- **RED (nondeterministic shell, observe-only):** `ade_node::mem_measure::rss_sampler` (`/proc/self/status` reader — the single OS-memory read site) + `ade_node::mem_measure::runner` (the GREEN/RED measurement seam that drives the GREEN workload under RED sampling). The A2/B/C/D live drivers (`--mode node`, wire pump, evidence emission) are RED.
- **Affected gates:** new — `ci_check_bounded_inbound_admission.sh`. Reused/extended — `ci_check_mempool_ingress_replay.sh` (the DC-MEM-04 seed, stays green), `ci_check_registry_code_locus_exists.sh` (CN-MEM-01 locus now populated). Stay green — `ci_check_mempool_ingress_closure.sh`, `ci_check_consensus_closed_enums.sh`.

## 8. Forbidden During This Cluster (slice-level prohibitions inherit)
1. **NO memory optimization that changes block validity, tx validity, chain selection, persisted bytes, or protocol-transcript semantics.** The bound governs *whether* untrusted work reaches the authoritative path, never *what verdict* it gets.
2. **No semantic feature flags; no configuration switches** that alter authoritative behavior. The admission budgets are fixed, closed constants (the `MAX_SERVE_RANGE_BLOCKS` / `MAX_WIRE_PUMP_LOOKAHEAD` pattern).
3. **No `HashMap`/`HashSet` in the GREEN bounded/evidence model; no wall-clock, no RNG, no float, no `std::fs`/`/proc` in GREEN.** The OS-memory read lives only in the RED sampler.
4. **No "drop whatever arrived last"** unless arrival order is explicitly RED-only and cannot affect an authoritative output. Bounded shedding is deterministic head-of-line over a canonical input order.
5. **RSS magnitude never gates anything.** It is release-tier evidence. A measurement is valid evidence only when its replay verdict is `Agreed`; the RSS numbers never enter a fingerprint or a pass/fail.
6. **The Haskell node is the behavior oracle, NOT an architecture template** (`feedback_oracle_seed_then_ade_owns`). No mimicking its allocator/queue internals.
7. **No `OP-MEM-01` flip without a committed C2-LOCAL artifact; no `CN-MEM-01 partial→enforced` flip without live wiring (B).** A1 is honest `partial` — hermetic proof + measurement substrate, no operational live evidence yet.

## 9. Replay / Evidence Obligations
- The bounded fold is a pure function of `(base, ordered events)` — same inputs yield byte-identical `(MempoolState, Vec<BoundedOutcome>)` (A1, `bounded_admission_is_deterministic`). Below the cap it is byte-identical to the unbounded `replay_ingress_trace` (`bounded_gate_under_budget_equals_unbounded`), so it strengthens, not weakens, `DC-MEM-04`.
- The evidence record carries a `final_fingerprint` over the authoritative output and a `replay_verdict` from re-running the same workload. **New replay-equivalence obligation (A2+):** a live memory run replays to the same recovered-tip + WAL/checkpoint fingerprint + authoritative outputs under memory pressure (verdict `Agreed`); a `Diverged` verdict invalidates the run.

## 10. Open Questions
- **OQ-MM-1 (bound granularity) [A1]:** per-batch count+byte budget (chosen for A1 — simplest closed bound) vs a per-peer fair budget. *Lean: per-batch for A1; per-peer fairness is a B concern (it composes with the N-AO `fair_merge` per-peer lanes).*
- **OQ-MM-2 (six measurement points → scenarios) [A2]:** are the six points one long run with phase markers, or six scoped runs each with its own paired fingerprint? *Lean: phase-marked single run for the sustained point; scoped runs for the isolated ones.*
- **OQ-MM-3 (`CN-MEM-03` shedding) [B]:** A1's over-budget shedding is deterministic head-of-line; `CN-MEM-03` (full deterministic shedding policy) is a B/own-slice concern, not claimed by A1.

## 11. Cluster Close Record
*(Filled at `/cluster-close`.)*

## 12. Follow-ons & Notes
- `CN-MEM-02` (operational dup of `OP-MEM-01`) and `CN-MEM-04` (mempool/ledger consistency, dup of `DC-MEM-01`) stay `declared`; they are subsumed by `OP-MEM-01` / `DC-MEM-01` and are not separately targeted here.
- The BA-08 (10-day average memory vs Haskell) bounty criterion is the cluster's release-tier north star (D); A1–C build the reproducible substrate it requires so the comparison is grounded, not anecdotal.
