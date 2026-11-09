# Cluster/Slice Plan — PHASE4-N-F-F (Ade)

> IDD Part IV artifact. Overall ordered plan for the PHASE4-N-F-F cluster, derived
> from the confirmed invariants sketch (`docs/planning/phase4-n-f-f-invariants.md`,
> committed at HEAD `a3eee84`). Overall plan only — full cluster/slice docs are
> produced by `/cluster-doc` and `/slice-doc`.

## Cluster Index (Dependency Order)

1. **PHASE4-N-F-F** — Operator-key ingress + operator-material-backed `ForgeActivation`
   for `--mode node` — primary invariant: **CN-NODE-03** — operator material reaches
   the forge only through RED-parse → BLUE-structural-validate → canonical-type into an
   all-or-nothing, fail-closed, RED-confined `ForgeActivation`; the binary passes `Some`
   iff the complete key set is present; the forge stays subordinate + self-accept-only
   and the N-F-E containment gate is never relaxed.

*(Single cluster. No new BLUE authority, no new canonical types, no new replay corpus —
so no downstream cluster depends on invariants this one enforces. The live-feed pairing
that makes forge observable is the separate operator-gated RO-LIVE-01 leg, explicitly out
of scope.)*

---

## Cluster PHASE4-N-F-F — Operator-key ingress → forge-on flip

- **Primary invariant:** CN-NODE-03 (declared). Carries CN-NODE-01, CN-NODE-02,
  CN-PROD-02, OP-OPS-04, DC-NODE-05, DC-CINPUT-02b, CN-CINPUT-03 unchanged.

- **TCB partition:**
  - **BLUE [reused, unchanged]** — `ade_crypto::kes_sum::Sum6Kes::raw_deserialize_signing_key_kes`;
    `ade_core::consensus::leader_check`; `ade_ledger::producer` forge body + the BLUE
    leadership projection inside `forge_one_from_recovered`. **No BLUE change.**
  - **GREEN [reused + 1 candidate new helper]** — `ade_node::run_loop_planner`
    (content-blind, unchanged); `ade_runtime::producer::coordinator::CoordinatorState::kes_period_for_slot`
    (reused); **`classify_forge_intent`** (new pure tri-state helper — GREEN-by-content
    if kept pure/secret-free; color resolved at slice-doc per the sketch's open color
    question).
  - **RED [new + reused]** — `ade_node::cli` (key-flag reading; fields already present as
    `Option<PathBuf>`); a single named RED ingress site (proposed `ade_node::operator_forge`,
    or `pub(crate)` reuse of `produce_mode`'s `parse_simple_genesis_json` /
    `parse_simple_opcert_json` / `load_kes_skey_any_format` — *do not duplicate*);
    `ade_node::node_lifecycle` (`ForgeActivation` assembly + `Some`/`None` flip +
    recovered-state lifetime restructure); reused `ade_runtime::producer::{keys,
    opcert_envelope, genesis_parser, producer_shell::ProducerShell}`.

- **Cluster Exit Criteria:**
  - **CE-F-1** — Forge intent is a pure, total tri-state function of CLI key-flag presence:
    complete required set ⇒ `On`; none ⇒ `Off`; any partial subset ⇒ structured fail-closed
    error. A partial key set can never produce a forge (mechanically: no path from
    `PartialKeySet` to `Some(activation)`).
  - **CE-F-2** — Operator material loads only through the existing RED loaders into
    `ProducerShell`; no private key bytes enter the GREEN coordinator/planner, node/loop
    state, WAL, log, snapshot, or evidence surface; tests/debug never
    print/serialize/hash-for-evidence/compare private key bytes (CI gate).
  - **CE-F-3** — The binary assembles an operator-material-backed `ForgeActivation` from
    operator material + genesis anchors (`slot_zero_time_unix_ms`→`anchor_millis`,
    `start_slot=0`, `slot_length_ms`, + the three KES fields) on the **single**
    recovered/bootstrap `BootstrapState` (no second bootstrap; recovered outlives both
    `ForwardSyncState` and `ForgeActivation`); `pool_id` is derived in one named place,
    never fabricated.
  - **CE-F-4** — Keys absent ⇒ binary passes `None` and is byte-identical to N-F-D/N-F-E
    relay; complete set present ⇒ passes `Some(activation)`; the success record states
    *forge-capable, not observable on the empty-source path* and makes no
    live/serve/BA-02/RO-LIVE claim.
  - **CE-F-5** — With a *continuing* hermetic feed + operator-material-backed activation +
    injected clock/shutdown schedule, **each forge attempt reaches only the fenced
    `forge_one_from_recovered` path** (no alternate forge codepath), self-accept-only
    (advances no durable tip; serves/admits/broadcasts/gossips nothing), **and the test
    asserts the expected number of attempts for its fixed clock schedule**; the
    forge-attempt sequence and forged bytes are byte-identical across runs
    (replay-equivalent).
  - **CE-F-6** — The N-F-E forge-containment gate (`ci_check_node_run_loop_containment.sh`)
    remains **semantically unchanged** (still exactly one fenced `forge_one_from_recovered`
    in the loop body, no `run_real_forge`, no serve/admit/gossip/broadcast/block-fetch/
    durable-tip); any new key-ingress CI gate is additive only — green at close on the full
    cluster diff.

- **Slices:**
  - **S1 — Forge intent classification** — invariant: pure tri-state
    `classify_forge_intent(cli_key_flags)` — complete ⇒ `On`, none ⇒ `Off`, partial ⇒
    structured `PartialKeySet{missing}`; no file reads, no key parsing, no secrets;
    partial-key forge unrepresentable — addresses: **CE-F-1** — TCB: **GREEN-by-content**
    (color confirmed at slice-doc).
  - **S2 — Operator material loading into RED custody** — invariant: reuse the existing
    KES/VRF/cold/opcert loaders (no reimpl) to build `ProducerShell`; key custody
    RED-confined; no-leak proof (no private bytes in planner/coordinator/WAL/log/evidence;
    tests/debug never emit/compare key bytes) + a no-leak CI gate — addresses: **CE-F-2**,
    contributes to **CE-F-6** (additive gate only) — TCB: **RED** (reused loaders) + a
    no-leak CI gate.
  - **S3 — Binary `ForgeActivation` assembly + `Some`/`None` flip** — invariant: from
    `ForgeIntent::On`, build the operator-material-backed `ForgeActivation` (genesis anchors
    via the existing parser, single recovered `BootstrapState` as forge base,
    recovered-state lifetime restructured to outlive `fwd`+activation, `pool_id` in one
    named place); wire the `--mode node` arm to pass `Some(activation)` iff complete, else
    `None` (byte-identical relay); honest forge-capable-not-observable record — addresses:
    **CE-F-3**, **CE-F-4**, contributes to **CE-F-6** — TCB: **RED** (`node_lifecycle` +
    the named ingress site).
  - **S4 — Hermetic operator-material-backed forge proof + replay-equivalence** — invariant:
    with a continuing hermetic feed + operator-material-backed activation + injected
    clock/shutdown, prove each forge attempt reaches only the fenced
    `forge_one_from_recovered` path, self-accept-only, advances no tip, serves/admits/
    gossips nothing, asserts the expected attempt count for its fixed clock schedule, and is
    replay-equivalent across runs. The fixture may use **real-format test keys**
    (cardano-cli-generated / N-P KES-corpus idiom) **but must not snapshot, print, compare,
    or evidence private key bytes** — addresses: **CE-F-5**, closes **CE-F-6** (gate green
    on full diff) — TCB: **RED test** over reused GREEN planner + BLUE forge.

- **Replay obligations:** **No new authoritative state, no new canonical types, no new
  replay corpus.** The replay obligation is DC-NODE-05's existing replay clause now
  exercised with operator-material-backed keys — proven hermetically in **S4** (fixed
  recovered state + ordered feed + injected clock + shutdown + fixed operator key set ⇒
  byte-identical forge-attempt sequence + forged bytes). The `None` path stays
  byte-identical to the N-F-D relay (T-REC-03 carried; re-proven in **S3**'s byte-identity
  test). No BLUE crate change ⇒ the 456 canonical-type total is unchanged.

- **FC/IS partition (restated for the close diff):** BLUE unchanged; GREEN gains at most
  the pure `classify_forge_intent`; all wiring/custody/parse is RED in `ade_node` (+ reused
  `ade_runtime::producer`). Dependencies flow inward only; no BLUE→RED edge introduced.

---

### Notes carried into every slice

- **Hard rule (all slices):** N-F-F may **add** key-ingress gates, but must **not relax**
  the N-F-E forge-containment gate (CE-F-6).
- **S2-genesis-extraction was evaluated and collapsed into S3:** the existing
  `parse_simple_genesis_json`→`GenesisAnchor` already yields `slot_zero_time_unix_ms` +
  `slot_length_ms` + `slots_per_kes_period`/`kes_anchor_slot`/`kes_max_period` cleanly, so
  no separate extraction slice is warranted. *If* slice-doc S3 finds the parser cannot
  supply an anchor cleanly (it won't, per inspection), the fallback is to split a tiny
  genesis-anchor slice ahead of S3 — flagged, not expected.
- **Reuse-not-duplicate:** `produce_mode`'s `parse_simple_genesis_json` /
  `parse_simple_opcert_json` / `load_kes_skey_any_format` are currently private; S2/S3 must
  share them (one named ingress site), not copy them.
- **Honest scope reminder:** `pparams`/`protocol_version` carry produce-path honest-scope
  defaults; this is ingress/activation wiring, not mainnet-complete block-production
  fidelity.
- **Registry discipline:** CN-NODE-03 stays `declared`; no `strengthened_in` bumps to
  OP-OPS-04 / CN-PROD-02 / DC-NODE-05 until cluster close.
