# PHASE4-N-T — produce_mode real-bootstrap composition (cluster doc)

> **Status:** Planning. 5-slice single cluster. Closes the gap between
> "produce_mode *can* forge" (N-Q/N-R/N-S machinery) and "produce_mode
> forges a coherent chain from **real** bootstrap state and serves it."
> **Scope-locked to Problem 1 (real in-memory production state).** Durable
> restart — WAL append / ChainDB store / snapshot cadence / warm-start
> recovery — is **deferred to a later cluster (provisionally N-U)** and is
> explicitly **not** claimed here.
>
> **Predecessor:** PHASE4-N-S (HEAD `dbee4d5`).
> **Inputs:** [`docs/planning/phase4-n-t-invariants.md`](../../planning/phase4-n-t-invariants.md)
> + [`docs/planning/phase4-n-t-cluster-slice-plan.md`](../../planning/phase4-n-t-cluster-slice-plan.md).

---

## §1 Primary invariant

> produce_mode's forge inputs are derived from the **real** bootstrap
> state (cold-start from the operator seed via the sole
> `bootstrap_initial_state` authority) and threaded forward across forges
> by a linear GREEN `ChainEvolution` typestate; every **self-accepted**
> forged block is admitted to the served snapshot via the single
> `push_atomic` authority — in one continuous run.

Load-bearing guarantees:

1. **No synthetic forge state (CN-PROD-03).** `SyntheticForgeInputs` /
   `build_synthetic_forge_context` are deleted. The forge base ledger,
   `PraosChainDepState`, `PoolDistrView`, `eta0`, and absolute slot all
   trace to `bootstrap_initial_state` (cold-start) + the operator
   consensus-inputs bundle.
2. **Single bootstrap chokepoint (CN-NODE-01 strengthened).** produce_mode
   obtains initial state only through `bootstrap_initial_state`; no
   parallel synthetic path. *(This exercises the cold-start branch;
   warm-start remains test-only until N-U — not a warm-start claim.)*
3. **Linear chain-forward (DC-PROD-03).** Each forge consumes the
   immediately-prior post-state; forging against a stale base is
   structurally unrepresentable (the typestate consumes `self`). Same
   `(seed, slot-sequence, keys)` → byte-identical chain-evolution series +
   forged bytes (in-memory two-run).
4. **Only self-accepted blocks served (CN-PROD-04).** The `BroadcastBlock`
   arm reconstructs the `AcceptedBlock` through the BLUE `self_accept`
   authority; on a `self_accept` replay rejection `push_atomic` is not
   called and the loop emits `BroadcastPushError::SelfAcceptReplayRejected`.
   `ProducerLogEvent::BlockServed` is emitted only for blocks present in
   the served snapshot.
5. **Empty-block scope (carried from CN-FORGE-01).** Forged bodies are the
   empty transaction set; mempool integration remains a separate cluster.

**Persistence non-claim (honest scope).** N-T does **not** claim crash
recovery of forged blocks. After process restart any
forged-but-not-persisted in-memory chain is not recovered. This is
acceptable for the block-acceptance artifact and is **not** a substitute
for DC-STORE / DC-WAL enforcement.

## §1.5 Doctrine: where the post-state and the token come from

`self_accept` (`self_accept.rs:73`) calls `block_validity` internally but
**discards the post-state** — it returns only the `AcceptedBlock` token
(private constructor, BLUE-minted). `ChainEvolution.advance` needs **both**
the post-state (to thread the chain forward) and the token (for
`push_atomic`). So `advance` invokes **two existing BLUE authorities**
against the consumed pre-forge base, then cross-checks their verdicts:

```
advance(self, forged_bytes, era_schedule, ledger_view)
  -> Result<(ChainEvolution, AcceptedBlock), ChainEvolutionError>

  BLUE  block_validity(base_ledger, base_chain_dep, era_schedule,
                       ledger_view, forged_bytes)
          -> (verdict_bv, post_ledger, post_chain_dep)   [post-state]
  BLUE  self_accept(forged_bytes, base_ledger, base_chain_dep,
                    era_schedule, ledger_view)
          -> AcceptedBlock | SelfAcceptError             [BLUE-minted token]
  GUARD verdict_bv (Valid?) MUST agree with self_accept (Ok?):
          disagreement -> ChainEvolutionError::AuthorityMismatch; no advance.
  GREEN bundle: new ChainEvolution{post_ledger, post_chain_dep, new tip}
                + the AcceptedBlock token
```

**`ChainEvolution` (GREEN) never constructs `AcceptedBlock` directly** — it
is obtained only from `self_accept`. The double validation (block_validity
+ self_accept over the same bytes / base / era_schedule / ledger_view) is a
negligible cost (one empty block per ~1s slot), pure, and deterministic.
The **verdict-agreement guard (CE-T-6b)** makes the redundancy *safe*: if
the two authorities ever disagree on the same inputs, `advance`
fail-closes with `ChainEvolutionError::AuthorityMismatch` and the chain
does not advance. Collapsing the two calls into a single BLUE
`self_accept_with_post_state` is a deferred optimization (OI-T.1),
**not** N-T scope.

## §2 Scope

### In scope
- **New GREEN module `ade_runtime::producer::chain_evolution`** — the
  linear `ChainEvolution` typestate (`seed`, `derive_forge_context`,
  `advance`) + closed `ChainEvolutionError` (incl. `AuthorityMismatch`).
  Holds fixed `{era_schedule, pool_distr_view, eta0}` + evolving
  `{base_ledger, base_chain_dep, tip}`; `ledger_view` (a trait object) is
  **passed as an argument**, not held, so the type stays a pure value
  (OI-T.2).
- **produce_mode startup (RED)** — build real `(ledger, chain_dep, tip)`
  from `--json-seed` (`import_cardano_cli_json_utxo`) + `--consensus-inputs`
  (`import_live_consensus_inputs`) and route through
  `bootstrap_initial_state` cold-start (empty `InMemoryChainDb` as
  chaindb+snapshot_store ⇒ cold-start ⇒ returns the seeded `genesis_initial`);
  derive `EraSchedule` (`make_schedule_for_imported_window`), `LiveLedgerView`,
  `PoolDistrView`.
- **produce_mode loop (RED)** — seed `ChainEvolution`; per-slot
  `derive_forge_context` (real stake/eta0/prev-hash/block-number, **absolute
  slot from `tip.slot`**); `advance` on `ForgeSucceeded`; **delete
  `SyntheticForgeInputs`**.
- **BroadcastBlock wiring (RED)** — route the `advance`-produced
  `AcceptedBlock` to `ServedChainHandle::push_atomic` (the handle currently
  discarded at `produce_mode.rs:209`); fail-closed
  `BroadcastPushError::SelfAcceptReplayRejected`; emit `BlockServed`.
- **`ProduceCli` extension (RED)** — require `--json-seed` + `--consensus-inputs`.
- **Tests + CI gate** — loopback forge→served→block-fetch readback; new
  `ci/ci_check_produce_mode_uses_bootstrap_initial_state.sh`.
- **3 new registry entries** (`declared` at this cluster doc):
  `CN-PROD-03`, `CN-PROD-04`, `DC-PROD-03`.

### Out of scope (deferred to N-U / later)
- WAL append of forged blocks, ChainDB block store, snapshot cadence,
  crash→warm-start recovery (**Problem 2 = N-U**).
- Warm-start branch of `bootstrap_initial_state` (test-only until N-U).
- Mempool integration / non-empty-block forging.
- Multi-epoch forging (N-T is single-epoch cold-start, matching admission's
  `make_schedule_for_imported_window`).
- Bounty-facing operator-pass live evidence against cardano-node (separate
  operator-action work; N-T is the in-process mechanical artifact).

### Honest-scope reminder
N-T proves forge→served byte-coherence in one continuous run against a real
operator seed. It does not persist, does not recover, and does not by
itself execute the cardano-node-accepts-our-block operator pass.

## §3 Slice index

| Slice | Purpose | Strengthens | Introduces |
|---|---|---|---|
| **S1** | produce_mode startup builds real `(ledger, chain_dep, tip)` from `--json-seed` + `--consensus-inputs` and routes through `bootstrap_initial_state` cold-start (empty in-memory store); derives `EraSchedule`/`LiveLedgerView`/`PoolDistrView`. **Behavior-preserving** — forge path still synthetic (deleted S3). Extends `ProduceCli`. | — (additive) | — |
| **S2** | New GREEN module `ade_runtime::producer::chain_evolution`: `seed(bootstrap_triple, era_schedule, pool_distr, eta0)`, `derive_forge_context(&self, slot) -> ForgeRequestContext`, `advance(self, forged_bytes, era_schedule, &dyn LedgerView) -> Result<(ChainEvolution, AcceptedBlock), ChainEvolutionError>` (via BLUE `block_validity` + `self_accept`, verdict-agreement guarded; **never mints `AcceptedBlock`**). Closed `ChainEvolutionError` incl. `AuthorityMismatch`. Unit tests: chain-forward determinism, two-run byte identity, rejection path, authority-mismatch fail-close. Not yet wired. | `DC-PROD-03` (anchor) | `chain_evolution` module |
| **S3** | Wire `ChainEvolution` into produce_mode: seed from S1 state; per-slot `derive_forge_context` (absolute slot from `tip.slot`); `advance` on `ForgeSucceeded` (chain threads; advance failure fail-closes); **delete `SyntheticForgeInputs` + `build_synthetic_forge_context`**. | `CN-PROD-02`, `CN-FORGE-01`, `DC-CONS-18` | — |
| **S4** | `BroadcastBlock` arm → `push_atomic` (consumes the `advance`-produced `AcceptedBlock` on the handle no longer discarded at `:209`); `BroadcastPushError::SelfAcceptReplayRejected` on rejection (no push); emit `BlockServed` on the serve path for present blocks. `drain_and_admit` not used. | `CN-SNAPSHOT-01` | `BroadcastPushError` |
| **S5** | Loopback integration test (forge→served→block-fetch readback; byte identity + two-run replay); new CI gate `ci_check_produce_mode_uses_bootstrap_initial_state.sh` (positive `bootstrap_initial_state(` grep + negative `SyntheticForgeInputs`/`build_synthetic_forge_context`/inline `LedgerState::new(` forge-base grep); flip `CN-PROD-03/04` + `DC-PROD-03` to `enforced`; record 5 strengthenings; cluster close. | all N-T entries → `enforced`; CN-NODE-01 strengthened | CI gate |

## §4 Exit criteria (CI-verifiable)

- [ ] **CE-T-1.** `docs/planning/phase4-n-t-{invariants,cluster-slice-plan}.md` exist.
- [ ] **CE-T-2.** `docs/clusters/PHASE4-N-T/{cluster,S1,S2,S3,S4,S5}.md` exist.
- [ ] **CE-T-3.** `ProduceCli` requires `--json-seed` + `--consensus-inputs`; missing → `CliError::ProduceMissingFlag`. Test `produce_cli_requires_seed_and_consensus_inputs` passes.
- [ ] **CE-T-4.** `produce_mode` calls `bootstrap_initial_state` (cold-start) with the seeded ledger; test `produce_mode_bootstrap_cold_start_seeds_real_ledger` asserts the returned ledger fingerprint equals the imported-UTxO ledger fingerprint.
- [ ] **CE-T-5.** Module `ade_runtime::producer::chain_evolution` exists, exporting `ChainEvolution` + `seed`/`derive_forge_context`/`advance` + closed `ChainEvolutionError`; `advance` returns `Result<(ChainEvolution, AcceptedBlock), ChainEvolutionError>`.
- [ ] **CE-T-6.** `chain_evolution` unit tests pass: `advance_threads_post_state_forward`, `advance_two_runs_byte_identical`, `advance_rejects_invalid_bytes`.
- [ ] **CE-T-6b.** `advance` invokes `block_validity` and `self_accept` against the **same** consumed pre-forge base, **same** `forged_bytes`, **same** `era_schedule`, and **same** `ledger_view`; if their verdicts disagree (`block_validity` Valid xor `self_accept` Ok) `advance` returns `ChainEvolutionError::AuthorityMismatch` and **no** advance occurs. Test `advance_authority_mismatch_fail_closes` passes.
- [ ] **CE-T-7.** Grep gate: no `AcceptedBlock {` struct-literal / mint in `chain_evolution.rs` (GREEN never mints; token comes from `self_accept`).
- [ ] **CE-T-8.** `SyntheticForgeInputs` + `build_synthetic_forge_context` absent from `crates/ade_node/src/produce_mode.rs` (grep gate).
- [ ] **CE-T-9.** produce_mode slot loop uses the absolute slot from the bootstrap tip (no `current_slot = 0` start); test `produce_mode_forges_at_absolute_bootstrap_slot`.
- [ ] **CE-T-10.** The `BroadcastBlock` arm calls `ServedChainHandle::push_atomic`; `_served_chain_handle` is no longer underscore-discarded (grep gate). Rejection emits `BroadcastPushError::SelfAcceptReplayRejected` with no push (test `broadcast_rejects_non_self_accepted_block`).
- [ ] **CE-T-11.** Loopback integration test `produce_forge_to_served_block_fetch_roundtrip` passes: a forged block is served and a loopback block-fetch reads byte-identical bytes back; two runs produce identical served-snapshot fingerprints.
- [ ] **CE-T-12.** `ci/ci_check_produce_mode_uses_bootstrap_initial_state.sh` passes (positive + negative grep) — strengthens CN-NODE-01 + CN-PROD-02.
- [ ] **CE-T-13.** `CN-PROD-03`, `CN-PROD-04`, `DC-PROD-03` flip to `enforced` with populated `tests` + `code_locus` + `ci_script`.
- [ ] **CE-T-14.** Strengthenings recorded: `CN-NODE-01`, `CN-PROD-02`, `CN-FORGE-01`, `CN-SNAPSHOT-01`, `DC-CONS-18` each `strengthened_in += "PHASE4-N-T"`.
- [ ] **CE-T-15.** `cargo test --workspace` clean; carry-forward CI gates still pass (`ci_check_producer_coordinator_no_secrets.sh`, `ci_check_no_independent_forge_codepath.sh`, `ci_check_leader_check_authority.sh`, `ci_check_no_produce_mode_direct_transport_writes.sh`).

> No human review may substitute for these checks.

## §5 TCB color map (FC/IS partition)

- **BLUE (reused, no new authority):** `ade_ledger::consensus_view::PoolDistrView`; `ade_core::consensus::leader_schedule`; `ade_core::consensus::leader_check`; `ade_ledger::producer::{forge, self_accept, served_chain_admit}`; `ade_ledger::block_validity`.
- **GREEN:** `ade_runtime::bootstrap::bootstrap_initial_state` (reused); **NEW `ade_runtime::producer::chain_evolution`**; `ade_runtime::producer::{coordinator, producer_log, broadcast_to_served}` (reused). `chain_evolution` carries the BLUE-style contract banner + deny attrs and is gated by CE-T-7 (no token minting) — GREEN-by-content inside the RED `ade_runtime` crate, like its sibling producer GREEN modules.
- **RED:** `ade_node::produce_mode` (loop, absolute-slot ticker, `push_atomic` call, evidence I/O); `ade_runtime::{seed_import, consensus_inputs}` importers (reused); `ade_node::cli` `ProduceCli` extension; `ade_runtime::producer::served_chain_handle::push_atomic` (watch channel).

Rules: no RED behavior in BLUE; GREEN must not affect authoritative
outputs. Color is resolved for every module above (OI-T.2 keeps
`ChainEvolution` a pure value — no held trait object).

## §6 Hard prohibitions (slices inherit)

- **No synthetic forge state.** No `SyntheticForgeInputs`,
  `build_synthetic_forge_context`, zero-stake `LeaderScheduleAnswer`, or
  inline `LedgerState::new(...)` as a forge base after S3.
- **No parallel bootstrap path.** produce_mode obtains initial state only
  via `bootstrap_initial_state`.
- **GREEN never mints `AcceptedBlock`.** `chain_evolution` obtains the
  token solely from `self_accept` (CE-T-7).
- **No advance on authority disagreement.** If `block_validity` and
  `self_accept` disagree on identical inputs, `advance` fail-closes with
  `ChainEvolutionError::AuthorityMismatch` (CE-T-6b).
- **No served block that failed self-accept.** Rejection ⇒
  `BroadcastPushError::SelfAcceptReplayRejected`, no `push_atomic`, no
  `BlockServed`.
- **No durability in N-T.** No `FileWalStore::append`,
  `PersistentSnapshotWriter`, `PersistentChainDb`, or snapshot cadence in
  the produce_mode path. Durability is N-U. *(Prevents accidental
  overclaiming of crash recovery.)*
- **No `drain_and_admit` in the broadcast arm.** Per-artifact
  `push_atomic` is the fit (OQ2).
- **No fork off a stale base.** `advance` consumes `self`; deriving a
  forge context for slot ≤ current tip slot fail-closes.

## §7 Replay obligations

- **DC-PROD-03 (new).** In-memory two-run byte identity of the
  chain-evolution series (`block_number`, `prev_hash`, post-ledger
  fingerprint, post-`chain_dep`) + forged bytes, for fixed
  `(seed, slot-sequence, keys)`. Anchored by S2 unit test + S5 integration.
- **Served-snapshot replay.** Extends `drain_and_admit`'s proven
  fingerprint stability to the `push_atomic` path (S5).
- **No new on-disk replay corpus** (durability deferred). No new BLUE
  canonical types (`ChainEvolution` is GREEN). Forged-block bytes remain
  governed by DC-CONS-18 / DC-FORGE-01.

## §8 Open issues

- **OI-T.1 — post-state retrieval. RESOLVED (user direction):** accept the
  double validation — `advance` calls `block_validity` (post-state) **and**
  `self_accept` (token), both existing BLUE, zero BLUE-surface change. A
  combined `self_accept_with_post_state` is **out of N-T scope** (deferred
  optimization). The verdict-agreement guard (CE-T-6b →
  `ChainEvolutionError::AuthorityMismatch`) makes the redundancy safe.
- **OI-T.2 — `ChainEvolution` shape. RESOLVED:** holds fixed
  `{era_schedule, pool_distr_view, eta0}` + evolving
  `{base_ledger, base_chain_dep, tip}`; `ledger_view` (`&dyn LedgerView`)
  is passed as an argument to `advance` / `derive_forge_context`, never
  held — so the type stays a pure GREEN value.
- **OI-T.3 — `PoolDistrView` projection (before S1/S3).** Confirm whether a
  helper already projects the consensus-inputs bundle's `pool_distribution`
  (`BTreeMap<Hash28, consensus_inputs::PoolEntry>`) into
  `ade_ledger::consensus_view::PoolDistrView`, or whether S1 adds a small
  GREEN projection helper (the `PoolEntry` types differ across crates).

## §9 References
- Predecessor: [`../PHASE4-N-S-A/cluster.md`](../PHASE4-N-S-A/cluster.md) + N-R-A composition doctrine.
- Existing surfaces composed: `bootstrap_initial_state` (GREEN, cold-start), `self_accept` (BLUE — note: discards post-state), `block_validity` (BLUE), `served_chain_handle::push_atomic` (RED), `coordinator` (GREEN — emits `BroadcastBlock`), `import_cardano_cli_json_utxo`, `import_live_consensus_inputs`, `make_schedule_for_imported_window`, `LiveLedgerView`.
- Doctrine: [[feedback-shell-must-not-overstate-semantic-truth]] (BlockServed only for served blocks; no durability overclaim), [[feedback-evidence-reducers-are-green-not-authority]], [[feedback-proof-discipline]] (OI-T.1/T.3 are obligations), [[feedback-read-grounding-docs-before-cluster-scoping]].

---

## §10 Authority reminder

This document is a planning aid only. All correctness rules live in the
project's normative specifications and the invariant registry. If there is
ever a disagreement: **normative documents + CI enforcement win.**
