# PHASE4-N-F-F — Slice S3: Binary ForgeActivation assembly + Some/None flip

> **Status:** slice doc (IDD Part IV). Companion to
> `../../planning/phase4-n-f-f-cluster-slice-plan.md` (S3 row). Code-verified
> against HEAD `5980037` (S2 merged) at authoring.

> **Slice S3 in one line:** assemble an operator-material-backed `ForgeActivation`
> from the loaded `ProducerShell` + genesis-derived anchors + the **single**
> recovered `BootstrapState`, and flip the `--mode node` binary arm to pass
> `Some(activation)` when the complete key set is present (else `None`,
> byte-identical relay). Forge-capable, but not observable on the empty-source
> binary path (the loop halts before any `ForgeTick`).

## 1. Slice identity
- **Cluster:** PHASE4-N-F-F (operator-key ingress → forge-on flip).
- **Slice:** S3 — Binary `ForgeActivation` assembly + `Some`/`None` flip.
- **Modules:** `crates/ade_node/src/operator_forge.rs` (RED — adds the material
  bundle + assembly), `crates/ade_node/src/node_lifecycle.rs` (RED — the binary
  arm wiring + recovered-state lifetime restructure + a fail-closed error
  variant), `crates/ade_node/src/produce_mode.rs` (RED — `parse_simple_genesis_json`
  widened to `pub(crate)`).
- **Reuses:** S1 `classify_forge_intent`, S2 `load_operator_producer_shell`,
  `produce_mode::parse_simple_genesis_json`, `coordinator_init`,
  `make_node_schedule`, `SystemClock`, the N-F-E `ForgeActivation` + `run_relay_loop`.

## 2. Cluster Exit Criteria addressed (verbatim)
- **CE-F-3** — The binary assembles an operator-material-backed `ForgeActivation`
  from operator material + genesis anchors (`slot_zero_time_unix_ms`→`anchor_millis`,
  `start_slot=0`, `slot_length_ms`, + the three KES fields) on the **single**
  recovered/bootstrap `BootstrapState` (no second bootstrap; recovered outlives
  both `ForwardSyncState` and `ForgeActivation`); `pool_id` is derived in one
  named place, never fabricated.
- **CE-F-4** — Keys absent ⇒ binary passes `None` and is byte-identical to
  N-F-D/N-F-E relay; complete set present ⇒ passes `Some(activation)`; the success
  record states *forge-capable, not observable on the empty-source path* and makes
  no live/serve/BA-02/RO-LIVE claim.
- contributes to **CE-F-6** — no change to the N-F-E forge-containment gate.

(CE-F-1 in S1; CE-F-2 in S2; CE-F-5 in S4.)

## 3. Intent (invariant impact)
Lands the **assembly + activation half of `CN-NODE-03`**: the `--mode node`
binary becomes forge-*capable* with the operator's real cryptographic identity —
`Some(activation)` iff the complete key set is present, `None` (exact relay)
otherwise, a partial set fail-closed (S1). The forge base is the **single**
recovered `BootstrapState` the lifecycle already produced (no second bootstrap),
borrowed read-only by the activation while the relay spine evolves its own clone
forward — so the recovered surface is the one leadership source. `pool_id` is
derived from the operator cold key in **one named place**, never fabricated.
The forge stays subordinate: with the empty-source binary path the loop halts
cleanly before any `ForgeTick` (forge subordinate to feed), so this slice makes
the node forge-capable but NOT observable — observable forge requires a live
feed (RO-LIVE-01).

## 4. Pre-conditions
- S1 + S2 merged (`5980037`): `classify_forge_intent`/`ForgePaths`;
  `load_operator_producer_shell`/`OperatorForgeError`.
- `ForgeActivation::new(clock, coordinator_state, recovered, shell, pool_id,
  pparams, protocol_version, anchor_millis, start_slot, slot_length_ms)` +
  `run_relay_loop(forge: Option<&mut ForgeActivation>)` exist (N-F-E).
- `LedgerState` + `PraosChainDepState` are `Clone`; `forge_one_from_recovered`
  reads `recovered.{seed_epoch_consensus_inputs, chain_dep, ledger, tip}`.
- `parse_simple_genesis_json` yields `slot_zero_time_unix_ms` + `slot_length_ms`
  + the three KES fields (OQ4 discharged — no separate extraction slice).

## 5. Implementation boundary
- **`operator_forge.rs` (RED) adds:**
  - `OperatorForgeError::GenesisParse(&'static str)` (additive variant).
  - `pub struct OperatorForgeMaterial { shell: ProducerShell, genesis:
    GenesisAnchor, pool_id: Hash28, pparams: ProtocolParameters, protocol_version:
    ProtocolVersion, anchor_millis: u64, start_slot: SlotNo, slot_length_ms: u32 }`.
  - `pub fn build_operator_forge_material(paths: &ForgePaths) -> Result<OperatorForgeMaterial, OperatorForgeError>`
    — `load_operator_producer_shell` (S2) + `parse_simple_genesis_json`; `pool_id
    = Hash28(blake2b_224(shell.cold_vk()))` (the one named derivation, never
    fabricated); `anchor_millis = genesis.slot_zero_time_unix_ms`, `start_slot =
    SlotNo(0)`, `slot_length_ms = genesis.slot_length_ms`; honest-scope
    `ProtocolParameters::default()` + `ProtocolVersion { major: 9, minor: 0 }`
    (matches the produce path). Does **not** build a `CoordinatorState` (kept in
    `node_lifecycle` so the S2 gate's `CoordinatorState` ban stays intact).
- **`produce_mode.rs`:** `parse_simple_genesis_json` → `pub(crate)` (behavior
  unchanged).
- **`node_lifecycle.rs` (RED) adds:**
  - `NodeLifecycleError::ForgeKeyIngress(String)` (fail-closed: partial key set,
    material load failure, or genesis parse failure — structured, secret-free) +
    `EXIT_NODE_FORGE_KEY_INGRESS_FAILED = 44` + `exit_code_for` arm.
  - In `run_node_lifecycle_inner`, after the recovered `state` is produced:
    `classify_forge_intent(cli.{cold_skey,kes_skey,vrf_skey,opcert,genesis_file}.as_deref())`
    mapped to a fail-closed `ForgeKeyIngress` on `PartialKeySet`. Then a `match`:
    - **`Off`** → exact current behavior: move `state.ledger`/`state.chain_dep`
      into `fwd`, empty source, `run_relay_loop(.., None)`.
    - **`On(paths)`** → `build_operator_forge_material`; `coordinator_init`
      (genesis anchor host for the reused `kes_period_for_slot`); `SystemClock::new`;
      clone `state.ledger`/`state.chain_dep` into `fwd` and keep `state` owned as
      the recovered baseline; `ForgeActivation::new(&mut clock, &coordinator_state,
      &state, &mut shell, pool_id, pparams, protocol_version, anchor_millis,
      start_slot, slot_length_ms)`; `run_relay_loop(.., Some(&mut activation))`;
      honest forge-capable-not-observable record.
  - Recovered-state lifetime: the `On` arm clones ledger+chain_dep into `fwd`
    (the spine evolves its copy) while `&state` is the recovered baseline the
    forge reads — semantically correct; one recovered state, no second bootstrap.

## 6. TCB color
- **RED:** `operator_forge` (assembly), `node_lifecycle` (arm wiring + error),
  `produce_mode` (visibility). Reused loaders/coordinator/clock are RED.
- **GREEN:** `classify_forge_intent` (S1), `run_loop_planner`,
  `CoordinatorState::kes_period_for_slot` — unchanged.
- **BLUE:** `forge_one_from_recovered`'s projection + forge body — reached only on
  a live feed; no BLUE change.

## 7. Invariants preserved (must not weaken) — by registry ID
- **CN-NODE-01** — initial state still via the single `bootstrap_initial_state`
  authority; the `On` arm reuses the **same** recovered `state` (no second
  bootstrap, no parallel init).
- **CN-NODE-02 / DC-SYNC-02** — the durable tip still advances ONLY via
  `run_node_sync → pump_block`; the `On` arm adds no tip-advance path. `run_relay_loop`
  body unchanged.
- **DC-NODE-05 / CE-F-6** — the forge tick + containment gate are unchanged; S3
  only supplies `Some/None` and builds the activation inputs.
- **CN-CINPUT-03 / DC-CINPUT-02b** — leadership still projected only via the
  recovered surface inside the fenced `forge_one_from_recovered`; S3 fabricates no
  `SeedEpochConsensusInputs`, names no bundle token.
- **CN-PROD-02 / CE-F-2** — key custody stays in `ProducerShell`; the S2 no-leak
  gate stays green (no `CoordinatorState` added to `operator_forge`).
- All BLUE invariants — no BLUE crate modified.

## 8. Invariants strengthened (one family: CN-NODE-03)
- **CN-NODE-03** (`declared`) — lands its **assembly + activation half**: the
  binary builds an operator-material-backed `ForgeActivation` on the single
  recovered state and flips `Some`/`None` by key presence. Contributes **CE-F-3**
  + **CE-F-4**. (Flips `declared → enforced` at cluster close; no `strengthened_in`
  bumps this slice.)

## 9. Replay / determinism obligations
- No new authoritative state / canonical type / WAL / checkpoint. The `Off` path
  is byte-identical to N-F-D/N-F-E relay (re-proven by the unchanged warm/first-run
  lifecycle tests + a new empty-source test). The clock anchors are deterministic
  from genesis; `build_operator_forge_material` is deterministic (same files ⇒
  same material ⇒ same `pool_id`). The forge-attempt replay-equivalence proof with
  operator material is S4 (CE-F-5).

## 10. Mechanical acceptance criteria
- [ ] `build_operator_forge_material` defined; `OperatorForgeMaterial` +
      `OperatorForgeError::GenesisParse` added; `parse_simple_genesis_json` is
      `pub(crate)`.
- [ ] `NodeLifecycleError::ForgeKeyIngress` + `EXIT_NODE_FORGE_KEY_INGRESS_FAILED
      = 44` + `exit_code_for` arm added.
- [ ] Test `build_operator_forge_material_from_complete_material` — asserts
      `pool_id == Hash28(blake2b_224(cold_vk))`, `anchor_millis ==
      genesis.slot_zero_time_unix_ms`, `start_slot == SlotNo(0)`, `slot_length_ms
      == genesis.slot_length_ms`, `protocol_version == {9,0}`.
- [ ] Test `build_operator_forge_material_bad_genesis_fails_closed` — malformed
      genesis ⇒ `Err(OperatorForgeError::GenesisParse(_))`.
- [ ] Test `build_operator_forge_material_pool_id_is_deterministic` — two builds
      ⇒ identical `pool_id`.
- [ ] Test `relay_loop_with_operator_material_empty_source_halts_no_forge`
      (node_sync tests) — an operator-material-backed `ForgeActivation` + an
      **empty** in-memory source ⇒ `run_relay_loop` halts cleanly with
      `hermetic_forge_outcomes` empty (forge-capable, not observable; CE-F-4).
- [ ] Test `node_mode_with_operator_keys_warm_start_forge_capable_halts_clean`
      (node_lifecycle tests) — drive `run_node_lifecycle_inner` over the warm-start
      fixture + the five operator-key flags set to real-format material ⇒ `Ok`
      (the `On` arm classifies, assembles, enters `run_relay_loop`, and halts
      cleanly on the empty source). Fixture must not snapshot/print/compare key bytes.
- [ ] Test `node_mode_partial_operator_keys_fail_closed` (node_lifecycle tests) —
      a warm-start `Cli` with only some key flags set ⇒
      `Err(NodeLifecycleError::ForgeKeyIngress(_))` (no forge, no silent relay).
- [ ] `cargo build -p ade_node` clean; `cargo test -p ade_node` green (count > 0,
      incl. the unchanged warm/first-run lifecycle tests = `Off`-path byte
      identity); `rustfmt`; all gates pass — incl. the unchanged
      `ci_check_node_run_loop_containment.sh` (CE-F-6), `ci_check_operator_forge_no_secret_leak.sh`,
      `ci_check_forge_intent_closed.sh`, `ci_check_private_key_custody.sh`.

## 11. Forbidden in this slice (inherits the cluster Forbidden list)
- **No relaxation of the N-F-E forge-containment gate** (hard rule, CE-F-6) — and
  no relaxation of the S2 no-leak gate (do not add `CoordinatorState` to
  `operator_forge`).
- No second bootstrap / parallel init / cold/genesis/bundle fallback (CN-NODE-01);
  the `On` arm reuses the existing recovered `state`.
- No tip-advance / serve / admit / gossip path in the arm (CN-NODE-02 / CE-F-6).
- No fabricated `SeedEpochConsensusInputs`, no forge-time bundle token
  (CN-CINPUT-03).
- No `pool_id` fabrication — derive in the one named place only.
- No new BLUE reference / canonical type / WAL / checkpoint.
- No live-peer / serve / BA-02 / RO-LIVE claim in the success record.

## 12. Slice completion checklist
- [ ] `operator_forge` assembly + `produce_mode` visibility + `node_lifecycle`
      arm wiring + error variant/exit code; tests added.
- [ ] `cargo build/test -p ade_node` green; `rustfmt`; all gates pass; no existing
      gate modified.
- [ ] Slice doc committed standalone (`docs:`) before impl; impl (`feat:`) after green.

## Authority
Registry IDs `CN-NODE-03` (assembly + activation half; `declared`), `CN-NODE-01` /
`CN-NODE-02` / `DC-SYNC-02` / `DC-NODE-05` / `CN-CINPUT-03` / `DC-CINPUT-02b` /
`CN-PROD-02` (preserved). The cluster-slice-plan and the invariant registry are
authoritative; this slice doc refines.
