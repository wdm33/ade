# PHASE4-N-F-F — Invariant Sketch

> IDD Part I artifact. Produced at the `/invariants` gate (2026-05-31). Frames the
> concept before any cluster/slice/code work. Normative for the `CN-NODE-03`
> registry declaration and the forthcoming `/cluster-plan`.

**Concept:** Operator-key ingress into `--mode node` — load the operator's
KES / VRF / cold / opcert keys (+ genesis-derived clock/KES anchors) and construct
an **operator-material-backed `ForgeActivation`**, so the real binary becomes
forge-capable at leader slots. A complete key set present ⇒ `Some(activation)`
(forge on); the set absent ⇒ `None` (byte-identical relay-only); any partial
subset ⇒ structured fail-closed error.

## Scope statement (load-bearing — do not soften)

N-F-F is **narrowly operator-key ingress + operator-material-backed
`ForgeActivation` construction + the binary passing `Some(activation)` when a
complete key set is present.** It is **not** live production.

The load-bearing honesty finding: `plan_loop_step` returns `HaltCleanly` on
`LoopState::Ending` **even when a forge slot is `Due`** (the forge is subordinate
to the feed; the loop never forges past the feed). The binary's `--mode node`
source is an **empty in-memory source** (`is_ended()` immediately true), so with
no live peer the loop halts on the first iteration and `ForgeTick` is **never
reached** — even with forge ON. Therefore, stated explicitly:

> **N-F-F makes the binary forge-*capable* once paired with a live/continuing
> feed; it does NOT itself make forge observable on the current empty-source
> binary path.**

N-F-F makes **no** claim of: live forge observation, serving, gossip, peer
acceptance, BA-02, RO-LIVE closure, or durable tip advance. Making the forge
observable requires a live/continuing feed — the RO-LIVE-01 follow-on, a separate
operator-gated leg.

**Expressible as a pure transformation.** The authoritative substance is
`canonical input → canonical output`: operator key files → (RED parse) → BLUE
structural validators (`Sum6Kes::raw_deserialize_signing_key_kes`, opcert /
Ed25519 deserializers) → canonical custody types → a deterministic
`ForgeActivation` assembled from those + the recovered `BootstrapState`. The forge
decision itself is already a pure BLUE transformation inside
`forge_one_from_recovered` (unchanged). N-F-F adds **no new authoritative
transition** — it is RED ingress + assembly feeding the existing BLUE forge. The
only nondeterminism (wall-clock) enters exclusively through the **existing RED
clock seam** (DC-NODE-03), canonicalized to a `SlotNo` before any pure code sees
it.

## 1. What must always be true

- **I1 (RED-parse → BLUE-structural-validate → canonical type).** Every operator
  key reaches the forge only through an existing RED loader feeding an existing
  BLUE structural validator (KES → `ade_crypto::kes_sum::Sum6Kes::raw_deserialize_signing_key_kes`;
  VRF/cold → cardano-cli text-envelope loaders; opcert → opcert parser). No new
  BLUE authority, no parser reimplementation, no plugin/trait seam. (Strengthens
  OP-OPS-04, CN-PROD-02.)
- **I2 (key custody stays RED-confined).** KES/VRF/cold private material lives only
  inside `ProducerShell` (RED). It never enters the GREEN coordinator state, the
  GREEN planner, or any persisted/logged/replay surface. (Carries CN-PROD-02's
  T-tier custody line onto the node path.)
- **I2a (`ProducerShell` custody contract).** `ProducerShell` is RED custody.
  Passing it to the fenced forge handoff (`forge_one_from_recovered`) is allowed;
  copying or extracting its private material into the planner, the coordinator
  state, any node/loop state, or any evidence surface is forbidden.
- **I3 (forge stays subordinate + self-accept-only).** N-F-F does **not** modify
  `run_relay_loop`'s body, the planner, or the existing N-F-E containment gate.
  The forge is still reached by exactly one fenced `forge_one_from_recovered`,
  advances no durable tip, and serves/admits/broadcasts/gossips nothing.
  (DC-NODE-05 / CN-NODE-02 / CE-E-4 carried, unmodified.)
- **I4 (recovered surface is the sole leadership source).** Leadership is projected
  only via `PoolDistrView::from_seed_epoch_consensus_inputs(&recovered.…)` inside
  the fenced call. No fabricated `SeedEpochConsensusInputs` literal, no forge-time
  bundle token. (CN-CINPUT-03 / DC-CINPUT-02b carried.)
- **I5 (KES freshness fail-closed).** The forge never signs past the hot-key
  period — `kes_period_for_slot` returning `None` past max ⇒ skip with a
  structured local outcome. (Carries CN-PROD-02 / DC-CRYPTO-* onto the node path.)
- **I6 (operator-material-backed anchors when forge is on).** When a complete key
  set is present, the `ForgeActivation`'s clock-seam anchors
  (`anchor_millis` / `start_slot` / `slot_length_ms`), `coordinator_state` (genesis
  KES anchor), and `era_schedule` are **genesis-derived** values, not the
  "provably-unconsumed placeholders" the forge-off binary path uses today — so the
  forge is *correct* the moment a live feed pairs with it.
- **I7 (single recovered-state forge base).** The forge base is the SAME
  recovered/bootstrap `BootstrapState` used to seed the relay spine. N-F-F must
  obtain it via the single `bootstrap_initial_state` / warm-start authority and
  must NOT call bootstrap/warm-start twice or construct a second recovered state.
  (CN-NODE-01 carried.)

## 2. What must never be possible

- **N1 (no partial-key forge).** A partial operator key set (e.g. KES present,
  VRF absent) must **fail closed** with a structured error — never silently fall
  back to relay-only, and never forge with a missing / zero / fabricated key.
- **N2 (no key in GREEN/BLUE/persistence).** No path places `KesSecret` /
  `VrfSigningKey` / `ColdSigningKey` into `CoordinatorState`, the planner, the WAL,
  a snapshot, a log line, or any replay surface.
- **N2a (no key-byte leakage in tests/debug).** Tests and debug output must not
  print, snapshot, serialize, hash-for-evidence, or compare private key bytes.
  Test assertions may compare public identifiers, structured outcomes, and forged
  artifacts only where already produced by the fenced forge path.
- **N3 (no live / serve / BA-02 overclaim).** Forge-on must not serve, admit,
  broadcast, gossip, advance a durable tip, or emit any BA-02/RO-LIVE evidence.
  The success record must not overstate (no "produced a block accepted by a peer").
- **N4 (no new BLUE authority / plugin seam).** No `*Anchor` trait, no key-source
  registry, no second forge codepath, no new BLUE crate change.
- **N5 (no retroactive / duplicate forge).** The monotonic `forge_slot_status`
  guard is not weakened: at most once per `SlotNo`, never a past slot.
- **N6 (no forge past the feed).** Operator keys do not change the planner's
  subordination: `LoopState::Ending ⇒ HaltCleanly` still wins over a due slot.
- **N7 (no relaxation of forge containment).** The existing N-F-E containment gate
  must remain **semantically unchanged**: still exactly one fenced
  `forge_one_from_recovered` call, no `run_real_forge`, no serve/admit/gossip/tip
  mutation. N-F-F may add key-ingress gates, but must not relax forge containment.

## 3. What must remain identical across executions

- The RED key-parse of fixed operator key files → identical custody types /
  identical fail-closed errors.
- The assembly of a `ForgeActivation` from fixed key files + a fixed recovered
  `BootstrapState` + fixed genesis → an identical activation (identical anchors,
  `pool_id`, `coordinator_state`).
- The forge-intent classification (complete set ⇒ On; none ⇒ Off; partial ⇒
  structured error) is a pure, total function of which CLI flags are present.

## 4. What must be replay-equivalent

- For a fixed recovered state + ordered block feed + injected clock tick schedule
  + shutdown schedule **+ fixed operator key set**, the forge-attempt sequence and
  forged block bytes are byte-identical across runs. (This is DC-NODE-05's replay
  clause, now exercised with operator-material-backed keys instead of hermetic
  ones — a strengthening, not a new law.)
- `None` (no keys) reproduces the exact N-F-D relay byte-for-byte (the existing
  N-F-E guarantee, carried).

## 5. State transitions in scope

```
classify_forge_intent(cli_key_flags)
  → Ok(ForgeIntent::On{kes,vrf,cold,opcert,genesis})   // complete set present
  → Ok(ForgeIntent::Off)                                // all absent → relay-only
  → Err(PartialKeySet{missing})                         // some-but-not-all → fail closed

load_operator_forge_material(ForgeIntent::On, recovered: &BootstrapState)
  → Ok(ForgeActivation { clock, coordinator_state, &recovered, &mut shell,
                         pool_id, pparams, protocol_version, anchor_millis,
                         start_slot, slot_length_ms })
  → Err(KeyLoadError | OpcertParse | GenesisParse | KesPeriodOutOfRange | …)   // structured, fail closed

run_node_lifecycle_inner(...)
  (recovered state, ForgeIntent)
  → run_relay_loop(..., forge = Some(&mut activation) | None)
  → Ok(()) | Err(NodeLifecycleError)
```

The `run_relay_loop` / `forge_one_from_recovered` transitions themselves are
**unchanged** — N-F-F only changes which argument (`Some` / `None`) the binary
supplies.

## 6. TCB color hypothesis

- **RED** — the whole of N-F-F's new code: CLI key-flag reading, the RED loaders
  (reused), `ProducerShell::init`, genesis parsing for anchors, `ForgeActivation`
  construction, the live `SystemClock` wiring, the `Some` / `None` binary flip.
  Key custody is RED by law.
- **BLUE (reused, unchanged)** — `Sum6Kes::raw_deserialize_signing_key_kes`, the
  leader check, `forge_one_from_recovered`'s leadership projection, the forge body.
  No BLUE change.
- **GREEN (reused, unchanged)** — `run_loop_planner` (content-blind),
  `CoordinatorState::kes_period_for_slot`.
- **Open color question:** `classify_forge_intent` (the key-presence → intent
  decision) is a pure total function over CLI flag presence — promotable to a
  small GREEN-by-content helper, or kept inline in the RED arm. It touches no
  secret and no authority either way. Resolve at slice-doc.

## 7. Open questions — resolved at the gate

- **OQ1 — Scope.** CONFIRMED. N-F-F = operator-key ingress + operator-material-backed
  `ForgeActivation` construction + the binary passing `Some(activation)` when a
  complete key set is present. No live forge observation / serving / gossip / peer
  acceptance / BA-02 / RO-LIVE closure / durable tip advance. Forge-capable once
  paired with a live/continuing feed; not observable on the empty-source binary
  path by itself.
- **OQ2 — Key-presence semantics.** CONFIRMED strict tri-state: complete required
  set present ⇒ `ForgeIntent::On`; all absent ⇒ `ForgeIntent::Off`; any partial
  subset ⇒ structured fail-closed error. No separate `--forge` boolean for now —
  presence of the complete operator material is the switch. **Required set:** cold
  signing key, KES signing key, VRF signing key, opcert, genesis file. `pool_id`,
  if not unambiguously derivable from existing material, is either added to the
  required set or derived in **one named place** — never fabricated.
- **OQ3 — pparams / protocol_version fidelity.** APPROVED with named limitation.
  N-F-F reuses the same `ProtocolParameters::default()` and default
  `protocol_version` scope as the existing produce path. Acceptable because N-F-F
  is ingress/activation wiring, **not ledger-valid block-production fidelity**.
  Real pparams / protocol-version derivation is a later compatibility closure if
  required. The cluster doc must NOT imply mainnet-complete block-production
  semantics.
- **OQ4 — Genesis anchor coverage.** Slice-entry proof obligation. N-F-F may extend
  `parse_simple_genesis_json` only enough to extract the real clock/KES anchors
  `ForgeActivation` needs: `system_start` / `anchor_millis`, `start_slot`,
  `slot_length_ms`, `slots_per_kes_period`, `kes_anchor_slot`, `kes_max_period`.
  RED parsing/assembly only — no BLUE change. **If the parser cannot produce these
  values cleanly, stop and split a tiny "genesis anchor extraction" slice before
  key ingress.**
- **OQ5 — Recovered-state lifetime.** APPROVED restructuring. The recovered
  `BootstrapState` must outlive both `ForwardSyncState` and `ForgeActivation`
  (today `state.ledger` / `state.chain_dep` are moved into `ForwardSyncState::new`
  before the loop call). Invariant made explicit (I7): the forge base is the same
  recovered/bootstrap state used to seed the relay spine; N-F-F must not call
  bootstrap/warm-start twice or create a second recovered state.
- **OQ6 — Bounty ranking.** Proceed, with caveat. Validation disagreement remains a
  higher-severity bounty risk than live-following; however, N-F-F is small, already
  sequenced, and does not weaken validation. It is acceptable to complete this
  producer-wiring closure now. This must NOT become a reason to expand into live
  production.

## Registry declaration

This sketch declares one new rule, **`CN-NODE-03`** (status `declared`), matching
the project registry schema (`tier` / `statement` / `attack_rationale` /
`evidence_notes` / `cross_ref` / `code_locus` / `tests` / `ci_script` /
`introduced_in` / `strengthened_in` / `status`; the project schema has no `kind`
field). `tests` + `ci_script` are populated at slice time.

Carry-forward strengthenings — recorded at **cluster close** when the slices land
them, not at declaration (mirroring N-F-E): `strengthened_in += "PHASE4-N-F-F"`
on **OP-OPS-04** (operator keys now feed `--mode node`), **CN-PROD-02** (RED
key-custody line extended to the node path), and **DC-NODE-05** (forge-slot
discipline now exercised with operator-material-backed keys).
