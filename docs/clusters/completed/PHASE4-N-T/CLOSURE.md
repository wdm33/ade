# PHASE4-N-T — CLOSURE

> **Cluster:** PHASE4-N-T — produce_mode real-bootstrap composition.
> **Scope-locked to Problem 1** (real in-memory production state). Durable
> restart (WAL / ChainDB / snapshot / warm-start) deferred to a later
> cluster (N-U) and explicitly **not** claimed.
> **Predecessor HEAD:** `dbee4d5` (PHASE4-N-S close + wiring).

## What shipped (5 slices)

| Slice | Commit | Summary |
|---|---|---|
| docs | `a1213d8` | cluster doc + invariants/plan + 3 declared rules |
| S1 | `9f525df` | produce-mode cold-start from operator seed via `bootstrap_initial_state`; `ProduceCli` requires `--json-seed` + `--consensus-inputs-path`; GREEN `PoolDistrView` projection (doubles as `&dyn LedgerView`) |
| S2 | `dbf6ea7` | GREEN `ade_runtime::producer::chain_evolution` linear typestate (`seed`/`derive`/`advance`); `advance` reconstructs the `AcceptedBlock` only via BLUE `self_accept`; `reconcile_verdicts` guard (`AuthorityMismatch`) |
| S3 | `6353dfd` | wire `ChainEvolution` into produce_mode; real `ForgeRequestContext` (operator pool = `blake2b_224(cold_vk)`, real `query_leader_schedule`); absolute slot from bootstrap tip; **deleted `SyntheticForgeInputs`** |
| S4 | `b46a0c6` | `BroadcastBlock` → `ServedChainHandle::push_atomic`; `BroadcastPushError::SelfAcceptReplayRejected` fail-closed; `BlockServed` emitted by observing the served snapshot |
| S5 | `e31e636` | loopback serve test (`produce_loopback.rs`) + CI gate `ci_check_produce_mode_uses_bootstrap_initial_state.sh` |
| close | (this) | registry flips + strengthenings + gate update + archive |

## Registry delta

- **Enforced (new):** `CN-PROD-03` (bootstrap-derived forge state, no synthetic), `CN-PROD-04` (broadcast reaches served via `push_atomic`), `DC-PROD-03` (chain-forward continuity + in-memory replay). All with populated `tests`.
- **Strengthened (`strengthened_in += "PHASE4-N-T"`):** `CN-NODE-01`, `CN-PROD-02`, `CN-FORGE-01`, `CN-SNAPSHOT-01`, `DC-CONS-18`.
- **CI gate `ci_check_produce_mode_uses_bootstrap_initial_state.sh`** added to `CN-NODE-01` + `CN-PROD-02` `ci_script` (the gate strengthens both).
- **No WAL/storage strengthening** (`DC-WAL-*` / `DC-STORE-*` / `CN-WAL-*` / `CN-STORE-*` / `CN-ANCHOR-*` / `CN-SEED-*` untouched), per scope lock.
- Registry total: 285 → **288** (+3). Append-only; no removals.

## CI gate update (load-bearing — note for review)

`ci/ci_check_node_binary_uses_single_bootstrap.sh` (CN-NODE-01 / DC-NODE-04)
previously required `ade_node` to call `bootstrap_initial_state` **exactly
once** across the crate — a proxy for "single startup path" valid when only
`node.rs::run_node_until_shutdown` called it. N-T adds a **second legitimate
caller** (`produce_mode::run_produce_mode`) — the first *production-binary*
bootstrap call (note: `run_node_until_shutdown` is test-driven; no `Mode::Run`
is wired in `main.rs`). The gate was updated to enforce the real invariant:
**each production `.rs` file calls `bootstrap_initial_state` at most once (no
path double-bootstraps) AND the crate calls it at least once (no zero-call
bypass)**. Per-mode no-synthetic-bypass is enforced by the new
`ci_check_produce_mode_uses_bootstrap_initial_state.sh` (produce) and the
existing `ReceiveState::new ≤ 1` guard (run). This is a strengthening of the
gate's precision, not a weakening — a synthetic bypass still fails closed.

## Exit criteria status

CE-T-1..CE-T-15: **met.** CE-T-3/4 (S1), CE-T-5/6/6b/7 (S2), CE-T-8/9 (S3),
CE-T-10 (S4), CE-T-11/12 (S5), CE-T-13 (3 rules enforced), CE-T-14 (5
strengthenings recorded), CE-T-15 (workspace test + carry-forward gates).

## Honest residual — in-process `ForgeSucceeded` not demonstrated

The S5 loopback proves N-T's **new wiring** end-to-end (real bootstrap state →
`ChainEvolution::advance` → `push_atomic` → served snapshot → block-fetch
readback, byte-identical, two-run replay-stable) using a real
`self_accept`-cleared corpus block through the same `advance` path produce_mode
runs on a `ForgeSucceeded`.

A **full in-process `ForgeSucceeded`** — produce_mode forging an empty-body
block that passes its *own* validator — is **not** demonstrated. The S5 Tier-A
attempt (consistent eligible-leader setup) found `run_real_forge` returns
`ForgeFailed { Other }` because the forged empty-Conway-body placeholder bytes
do not re-decode (`decode_block` → `Body(Decoding(InvalidStructure))`), *before*
`self_accept`. This is a **pre-existing forge-authority limitation** (N-R-A
already deferred `ForgeSucceeded`; consistent with `full_stake_answer_reaches_
self_accept_and_rejects` accepting `Other`), **not introduced by N-T** and
outside N-T's no-BLUE-change scope. It is the binding blocker for the
`CN-CONS-06` / `RO-LIVE-01` bounty artifact and warrants a dedicated
forge-authority investigation cluster (empty Conway block body encode↔decode
round-trip). Tracked, not hidden.

## Persistence non-claim (carried)

N-T does not claim crash recovery of forged blocks. After process restart any
forged-but-not-persisted in-memory chain is not recovered. This is acceptable
for the block-acceptance wiring artifact and is **not** a substitute for
DC-STORE / DC-WAL enforcement. Forged-block durability (WAL append + ChainDB
store + snapshot cadence + warm-start recovery) is the deferred N-U cluster.

## Security assessment (per-cluster)

Low new surface. N-T composes already-validated BLUE authorities
(`bootstrap_initial_state`, `query_leader_schedule`, `self_accept`,
`block_validity`, `served_chain_admit`) behind a new GREEN typestate +
RED produce_mode wiring. **No new parsing of untrusted network bytes** (the
`--json-seed` + `--consensus-inputs` files are operator-controlled and parsed
by the *existing, unchanged* `seed_import` / `consensus_inputs` importers). No
crypto changes, no new `unsafe`, no new auth path. `ChainEvolution` is pure
(no I/O/clock/HashMap/float) and never mints `AcceptedBlock` (CI-gated). The
serve path emits `BlockServed` only for snapshot-present blocks (no
over-claim). No HIGH+ findings.

## Grounding-doc note

`docs/ade-CODEMAP.md` / `-SEAMS.md` / `-TRACEABILITY.md` remain N-P-scope-narrow
(gap (yy) from the CODEMAP residual list — they do not yet inventory N-Q / N-R /
N-S / N-T modules + rules). Full regeneration covering N-Q..N-T is a tracked
separate work item, not blocking N-T close. `docs/ade-HEAD_DELTAS.md` regen
(`/head-deltas`) is likewise deferred to the next grounding refresh.

## Open obligations carried after closure

- `CN-CONS-06` / `RO-LIVE-01` — `blocked_until_operator_pass_executed` (carried; further gated by the in-process forge residual above).
- `CN-SNAPSHOT-01` remains `declared` (its full enforcement is N-R-B's gated item; N-T provides the production `push_atomic` driver + recorded the strengthening).
- Forged-block durability → N-U.
- In-process `ForgeSucceeded` / empty-body forge round-trip → dedicated forge-authority cluster.
