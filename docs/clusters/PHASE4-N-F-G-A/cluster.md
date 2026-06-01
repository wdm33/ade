# Cluster PHASE4-N-F-G-A — Forge fidelity on the node spine

> **Status: PLANNED** (derived from committed sketch `b063c6a4` + committed plan `1c3b077f`).
> First sub-cluster of **PHASE4-N-F-G** (RO-LIVE-01). Successor context: **PHASE4-N-F-F**
> (operator-key ingress, `origin/main f08b12ca`) made `--mode node` forge-CAPABLE; N-F-E
> wired the self-accept-only forge tick. G-A makes the forged block **genuinely valid for a
> real peer** — genesis-consistent leadership inputs, real operator config, slot-aligned,
> epoch-boundary fail-closed.
>
> Companion docs: `../../planning/phase4-n-f-g-invariants.md` (sketch — OQ1/2/3 resolved),
> `../../planning/phase4-n-f-g-cluster-slice-plan.md` (the G-A/G-B/G-C plan).
>
> **Cluster character (load-bearing — do not broaden):** forge **validity** only. Proves the
> node can identify **when it may forge** and forges a block a real peer would validate. *Not*
> a serve cluster, *not* a live-feed cluster, *not* a peer-evidence cluster.
>
> **Hard line:** if a slice needs serve/admit/gossip, live-feed/WirePump wiring, a relay-loop-
> containment-gate relaxation, a new BLUE authority, or a second bootstrap — **stop and
> re-scope** rather than smuggling G-B/G-C work into G-A. **And: if S1 cannot prove the
> recovered seed epoch is genesis-consistent against the committed private fixture, stop and
> insert S1b — do not proceed to S2–S4 or any serve/live work.**

## Primary invariant
On the `--mode node` forge path, leadership is computed **only** from a **genesis-consistent**
recovered seed epoch (eta0 from the recovered `PraosChainDepState.epoch_nonce`; stake/ASC/
per-pool VRF-keyhash from the recovered `SeedEpochConsensusInputs`), the forge slot is derived
**only** through the clock seam over the **real** genesis anchor, and a candidate slot
**outside the single recovered seed epoch fails closed** — it cannot be forged, served, or
signed (stale-eta0 past the boundary is a peer-reject class). [`DC-EPOCH-03`]

**Invariants strengthened:** `CN-OPCERT-01` + `CN-GENESIS-01` (real cardano-cli text-envelope
ingress wired on the node path), `DC-NODE-05` (recovered-surface forge now genesis-consistency-
proven + epoch-boundary-hardened), `DC-NODE-03` (clock seam derives the slot from the real
genesis anchor).

## Normative anchors
- `docs/planning/phase4-n-f-g-invariants.md` (the `/invariants` sketch; OQ1/2/3 resolved,
  OQ4/5/6 open).
- `docs/planning/phase4-n-f-g-cluster-slice-plan.md` (the committed three-sub-cluster plan).
- `docs/planning/operator-pass-live-leg-c1-scoping.md` §4a (the G3/eta0 / single-epoch /
  extract-once analysis — mined, not followed literally; it is produce_mode-centric).
- Registry: `DC-EPOCH-03` (declared, `introduced_in = PHASE4-N-F-G-A`) + the four strengthened
  rules above. Carried: `docs/active/CE-79_gate_statement.md`, `CLAUDE.md`.

## Locked rules (from the OQ ratification, G-A scope)
- **Genesis-consistency is the gate (OQ5 — S1 first).** No S2/S3/S4 forge-fidelity claim — and
  no G-B/G-C work — proceeds until S1 proves the WarmStart-recovered seed epoch is genesis-
  consistent against a committed reference fixture. **Contingency: if S1 disproves it, G-A stops
  and inserts S1b** (a private-net bootstrap/anchor path) before anything downstream.
- **Reuse the recovered surface, don't fabricate (guard d holds).** Leadership inputs come ONLY
  from the recovered `chain_dep` (eta0) + recovered `SeedEpochConsensusInputs` (via
  `PoolDistrView::from_seed_epoch_consensus_inputs`). No fabricated `SeedEpochConsensusInputs`/
  eta0/pparams/pool_id literal; no `--consensus-inputs-path` bundle token on the forge path
  (`ci_check_consensus_input_provenance.sh` guard d).
- **Single bootstrap held (CN-NODE-01).** The genesis-consistent recovered state is obtained
  through the **existing** single `bootstrap_initial_state` / `warm_start_recovery` authority —
  the operator pre-seeds the store (reusing the proven `admission::seed_to_snapshot`
  extraction); **no new bootstrap authority, no Mithril call, no second recovered state.**
- **Single recovered seed epoch only (OQ3).** Crossing the epoch boundary fails closed (S4).
  Cross-epoch nonce-roll (driving the BLUE `NonceInput::{CandidateFreeze, EpochBoundary}`
  transitions) is a **separate cluster**, not G-A.
- **Real operator config, narrowest extension (G1/G2/G4).** S2 reuses the **dormant** real
  parsers (`parse_opcert_envelope`, `parse_shelley_genesis`); `protocol_version` +
  `prev_opcert_counter` derive from the loaded opcert/genesis; full `ProtocolParameters` may
  stay honest-scope **iff** a test proves an empty Conway block's header+body validation is
  invariant to them (else they derive too). **Sharpening (verified): `parse_shelley_genesis`
  yields only the clock/KES anchors — NOT `protocolVersion`/`protocolParams` — so S2 must
  EXTEND it (or a sibling) to supply protocol-version / pparams derivation, or stop and split.**
- **Compatibility is proven on observable/derived surfaces, not state hashes (DC-COMPAT-01).**
  The S1 pinning harness compares Ade's `praos_vrf_input` bytes + leader-threshold inputs + the
  recovered (eta0, stake, ASC, vrf-keyhash) to a committed reference fixture — never an
  Ade-internal-fingerprint-vs-Haskell-serialized-state-hash equality.
- **Containment untouched (hard line).** `ci_check_node_run_loop_containment.sh` stays
  byte-/semantically unchanged; G-A adds no serve token to the loop body.

## Verified component inventory (read at `f08b12ca`/HEAD, not assumed)

| Component | Real state (verified) | Use |
|---|---|---|
| `consensus::vrf_cert::praos_vrf_input(slot, &Nonce) -> [u8;32]` (`vrf_cert.rs:131`); `leader_vrf_input` (`:221`); `ActiveSlotsCoeff{numer,denom}` (`:252`) | BLUE, enforced (validation-side corpus-proven) | **S1** pinning harness asserts Ade's VRF-input + threshold inputs match the reference fixture |
| `consensus::leader_schedule::query_leader_schedule(...)` (`leader_schedule.rs:79`) | BLUE leader-eligibility threshold `1-(1-f)^σ` | **S1** the leader-set agreement the harness pins |
| `SeedEpochConsensusInputs{anchor_fp, epoch_no, active_slots_coeff, total_active_stake, pool_distribution: BTreeMap<Hash28, PoolEntry>}`; `PoolEntry{active_stake, vrf_keyhash: Hash32}` | BLUE recovered record; **carries stake/ASC/vrf — NOT eta0** | **S1** the recovered stake/ASC/vrf half of the genesis-consistency check |
| `PraosChainDepState.epoch_nonce: Nonce` (`praos_state.rs:118`) | BLUE; **this is eta0** | **S1** the recovered eta0 half (the other genesis-consistency surface) |
| `consensus_view::from_seed_epoch_consensus_inputs(...)`; off-epoch query → `None` (`consensus_view.rs:82,95`) | BLUE projection; already fails closed off-epoch | **S4** the BLUE foundation the epoch-fail-closed guard surfaces structurally |
| `bootstrap::warm_start_recovery` / `SeedEpochConsensusSource::RequiredFromRecoveredProvenance` (`node_lifecycle.rs:728`, `bootstrap.rs:64,229`) | RED single-authority recovery; populates `seed_epoch_consensus_inputs` | **S1** the recovery path the operator pre-seed drives (no new bootstrap) |
| `admission::seed_to_snapshot(utxo, chain_dep, slot, store)` (`seed_to_snapshot.rs:55`) | RED proven extraction (admission) | **S1** reused to pre-seed the private-net store (no new extraction authority) |
| `producer::opcert_envelope::parse_opcert_envelope(&[u8]) -> DecodedOpCertEnvelope` (`opcert_envelope.rs:87`) | RED, **dormant — zero non-test callers (verified)** | **S2** un-dormants it on the node path; carries `prev_opcert_counter` |
| `producer::genesis_parser::parse_shelley_genesis(&[u8], kes_anchor_slot) -> GenesisAnchor` (`genesis_parser.rs:53`) | RED, **dormant — only a doc-comment reference (verified)**; yields networkMagic/systemStart→`slot_zero_time_unix_ms`/slotLength→`slot_length_ms`/KES fields — **NOT pparams/protocolVersion** | **S2** un-dormants it; **must extend** to extract `protocolVersion`/`protocolParams` for the derived constants |
| `operator_forge::build_operator_forge_material` calling `parse_simple_opcert_json` (`:96`) + `parse_simple_genesis_json` (`:141`) | RED N-F-F ingress (simple-JSON) | **S2** swaps to the real parsers; keeps the `ci_check_operator_forge_no_secret_leak.sh` no-leak property |
| `clock::millis_to_slot(...)` (`clock.rs:82`); `clock::SystemClock` (`:107`) | GREEN/RED clock seam (N-K) | **S3** reuses, now over the **real** genesis anchor from S2 |
| `producer::coordinator::CoordinatorError::SlotDrift{from,to}` (`coordinator.rs:254`, raised `:395`) | exists in the **produce** coordinator (swallowed on the produce tip path per C1 §1d) | **S3** the node forge path adds a drift guard that **fails closed** (does not swallow) |
| `consensus::nonce::NonceInput::{HeaderContribution, CandidateFreeze, EpochBoundary}` (`nonce.rs:42,51,56`) | BLUE; `EpochBoundary` exists but is **undriven on the forge path** | **S4** the rationale: recovered eta0 is the seed-epoch nonce; past the boundary it's stale → fail closed (no promotion this cluster) |
| `ci_check_node_run_loop_containment.sh` (N-F-E) | forbids serve/tip tokens in the loop body | **UNCHANGED** by G-A (hard line) |

## Slices (safety order)

### S1 — Genesis-consistent recovered seed epoch + pinning harness *(the first hard fork; hermetic)*
A committed reference fixture (a private genesis + Haskell-sourced expected leadership values,
reference-data discipline) drives a GREEN `ade_testkit` **pinning harness** that proves:
(a) the WarmStart-recovered eta0 (`chain_dep.epoch_nonce`) + the recovered
`SeedEpochConsensusInputs` (stake/ASC/per-pool vrf-keyhash) equal the fixture's genesis-derived
view — **both surfaces** (eta0/chain-dep AND stake/ASC/vrf/SeedEpochConsensusInputs); (b) Ade's
`praos_vrf_input(slot, eta0)` + `query_leader_schedule` threshold inputs equal the fixture's
expected values; (c) the pre-seed→`warm_start_recovery`→recovered-state round-trip carries the
inputs **faithfully** (no corruption), reusing `admission::seed_to_snapshot` to pre-seed (no new
bootstrap authority — CN-NODE-01). Compares observable/derived surfaces only (DC-COMPAT-01).
Addresses **CE-G-A-1**. TCB: **GREEN** harness + **RED** fenced setup + consume **BLUE**.
> **Hard-fork contingency:** if (a)/(b)/(c) cannot be proven for the fixture, **G-A stops and
> inserts S1b** (a private-net bootstrap/anchor path or a formula remediation) **before S2–S4
> and before any G-B/G-C work**. Flagged, not expected.

### S2 — Real cardano-cli opcert/genesis ingress + derived constants *(hermetic)*
`operator_forge` loads `--opcert` via `parse_opcert_envelope` and `--genesis-file` via
`parse_shelley_genesis`, retiring `parse_simple_opcert_json`/`parse_simple_genesis_json` **on the
node path**; `protocol_version` + `prev_opcert_counter` derive from the loaded opcert/genesis.
**Sharpening (verified proof obligation): `parse_shelley_genesis` does not currently output
`protocolVersion`/`protocolParams` — S2 must EXTEND it (or a sibling) to supply them, or stop and
split.** Full `ProtocolParameters` derive **or** a test proves empty-Conway-block header+body
validation is invariant to the honest-scope defaults. Keeps the
`ci_check_operator_forge_no_secret_leak.sh` no-leak property. Addresses **CE-G-A-2**. TCB:
**RED** (`operator_forge` + reused/extended RED parsers) + consume **BLUE**. *(May split S2a
parsers / S2b constants at slice-doc.)*

### S3 — Slot alignment + SlotDrift fail-closed *(hermetic)*
The node forge slot derives via `millis_to_slot` over the **real** genesis anchor
(`slot_zero_time_unix_ms` + `slot_length_ms` from S2), at the single RED `SystemClock` seam (only
`SlotNo` crosses); a drift guard **fails closed** on an implausible slot (the node path must not
swallow drift the way the produce coordinator does); same (genesis-anchor, millis) → same
`SlotNo`. Addresses **CE-G-A-3**. TCB: **RED** (clock seam) + **GREEN/BLUE** (pure map + guard).

### S4 — Epoch-boundary forge fail-closed (DC-EPOCH-03) *(hermetic)*
A candidate forge slot whose epoch ≠ the recovered seed epoch **fails closed** with a structured
outcome — cannot forge/serve/sign; the recovered `PoolDistrView` returns `None` off-epoch (BLUE
foundation) and the forge surfaces it as a fail-closed result (not a silent skip); the recovered
`chain_dep` eta0 is the seed-epoch nonce — no `EpochBoundary` promotion on the forge path.
Hardens N-F-E's `forge_tick_off_epoch_slot_fails_closed_local` into the named DC-EPOCH-03 boundary
with the eta0-staleness rationale. Addresses **CE-G-A-4**; flips **DC-EPOCH-03** declared→enforced.
TCB: **GREEN/BLUE** guard + consume **BLUE** nonce/consensus-view.

## Exit criteria (mechanical, CI-verifiable)
New test/check names are **candidate** (created by the owning slice); existing artifacts named
as-is.

- **CE-G-A-1** — genesis-consistency: candidate harness/tests
  `pinning_recovered_eta0_matches_genesis_fixture`,
  `pinning_recovered_stake_asc_vrf_matches_genesis_fixture`,
  `pinning_praos_vrf_input_and_threshold_match_fixture`,
  `pinning_preseed_warmstart_roundtrip_faithful` pass against a **committed reference fixture**;
  comparisons are observable/derived-surface only (existing
  `ci_check_no_haskell_fingerprint_equality.sh` / DC-COMPAT-01 holds). *(gates S2–S4 + all
  G-B/G-C work.)*
- **CE-G-A-2** — ingress fidelity: `operator_forge` calls `parse_opcert_envelope` +
  `parse_shelley_genesis` (no `parse_simple_*` on the node path — candidate gate
  `ci_check_node_forge_real_cli_ingress.sh`); `protocol_version` + `prev_opcert_counter` derive
  from the loaded opcert/genesis (S2 extends `parse_shelley_genesis` to emit
  `protocolVersion`/`protocolParams`); candidate test
  `operator_forge_empty_conway_block_invariant_to_honest_pparams` (or a derived-pparams test);
  existing `ci_check_operator_forge_no_secret_leak.sh` still green. *(strengthens `CN-OPCERT-01`,
  `CN-GENESIS-01`.)*
- **CE-G-A-3** — slot alignment: candidate tests
  `node_forge_slot_via_millis_to_slot_over_real_genesis_anchor`,
  `node_forge_slot_drift_fails_closed`; existing `ci_check_clock_seam.sh` +
  `ci_check_forbidden_patterns.sh` hold (no `SystemTime`/`Instant`/float past the seam).
  *(strengthens `DC-NODE-03`.)*
- **CE-G-A-4** — epoch fail-closed: candidate tests `node_forge_off_epoch_slot_fails_closed`,
  `node_forge_no_epoch_boundary_promotion_on_forge_path`; candidate gate
  `ci_check_node_forge_single_epoch_fail_closed.sh`. *(`DC-EPOCH-03` flips declared→enforced when
  CE-G-A-1..4 are all green.)*

## TCB color map
- **BLUE (none — reuse only):** `consensus::{vrf_cert, leader_schedule, nonce}`,
  `ade_ledger::consensus_view` (off-epoch `None`). A BLUE change is a red flag G-A is absorbing
  authority → reject.
- **GREEN:** the `ade_testkit` pinning harness (S1); reused `ade_runtime::clock::millis_to_slot`;
  the pure slot-drift guard / off-epoch guard if kept pure (color confirmed at slice-doc).
- **RED:** `ade_node::operator_forge` (real-parser rewiring, S2); reused/extended RED parsers
  `ade_runtime::producer::{opcert_envelope::parse_opcert_envelope, genesis_parser::parse_shelley_genesis}`
  (S2); `ade_runtime::clock::SystemClock` (S3); the fenced operator pre-seed setup (S1, reuses
  `admission::seed_to_snapshot`).
- **CI:** candidate `ci_check_node_forge_real_cli_ingress.sh` (S2),
  `ci_check_node_forge_single_epoch_fail_closed.sh` (S4); existing
  `ci_check_operator_forge_no_secret_leak.sh`, `ci_check_clock_seam.sh`,
  `ci_check_consensus_input_provenance.sh` (guard d), `ci_check_node_run_loop_containment.sh`
  (**unchanged**), `ci_check_no_haskell_fingerprint_equality.sh` continue to hold.

## Forbidden during this cluster *(slice-level prohibitions inherit)*
- No **serve / serve-handoff / sibling serve task / `push_atomic` / `block_fetch` / broadcast /
  gossip** (that is G-B).
- No **live-feed / `WirePump` / `n2n_dialer` / session wiring into the binary** (that is G-C).
- No **RO-LIVE / BA-02 / peer-acceptance** claim (G-C + operator-gated).
- **Do not relax `ci_check_node_run_loop_containment.sh`** (byte-/semantically unchanged); no
  serve token in the loop body.
- No new **BLUE authority / canonical type / WAL/checkpoint format**; no second bootstrap / no
  Mithril call on the node path (CN-NODE-01).
- No **fabricated** `SeedEpochConsensusInputs`/eta0/pparams/pool_id literal; no bundle token on
  the forge path (guard d).
- No **cross-epoch nonce roll / `EpochBoundary` promotion** on the forge path (S4 fails closed
  instead).
- No `SystemTime`/`Instant`/float crossing past the RED seam; only `SlotNo` crosses.
- The pinning harness must **not** assert an Ade-internal-fingerprint-vs-Haskell-state-hash
  equality (DC-COMPAT-01).
- **Hard line:** if forge-fidelity needs a BLUE change, a containment relaxation, serve/live
  wiring, or a second bootstrap — **stop and re-scope.** If S1 cannot prove genesis-consistency
  against the committed fixture — **stop and insert S1b.**

## Replay obligations (scoped)
No new canonical type, no new authoritative transition, no new WAL/checkpoint format, no new
corpus entry. The recovered-state faithfulness (S1) extends the existing N-F-A
pre-seed→WarmStart-recover replay-equivalence; the slot map (S3) is a pure function
(unit-tested); the off-epoch fail-closed (S4) is deterministic; the pinning harness (S1) is a
proof + committed reference fixture, not replay corpus. Determinism guard: the wall-clock
observation (S3) is the lone RED nondeterminism, canonicalized to `SlotNo` before crossing.
Acceptance scoped to touched crates (`ade_node`, `ade_runtime`, `ade_testkit`, consumed
`ade_core`/`ade_ledger`) — **not** the full `ade_testkit` corpus/oracle lane (times out ~600s on
clean HEAD).

## Registry impact (at close)
`DC-EPOCH-03` already `declared` at sketch (registry 311 → 313 with `DC-NODE-06`). Promotion /
strengthening:
- `DC-EPOCH-03` (derived) — `declared` → **enforced** across S1–S4 (CE-G-A-1..4 green).
- `CN-OPCERT-01`, `CN-GENESIS-01` — `strengthened_in += "PHASE4-N-F-G-A"` (S2: real text-envelope
  ingress wired on the node path).
- `DC-NODE-05` — `strengthened_in += "PHASE4-N-F-G-A"` (S1/S4: recovered-surface forge
  genesis-consistency-proven + epoch-hardened).
- `DC-NODE-03` — `strengthened_in += "PHASE4-N-F-G-A"` (S3: clock seam derives the slot from the
  real genesis anchor).
- **Not added here:** `DC-NODE-06` (serve handoff — G-B); serve/live/RO-LIVE evidence (G-B/G-C).

## Non-goals
No serve handoff (G-B). No live feed / `WirePump` / operator pass / peer acceptance (G-C,
operator-gated). No cross-epoch production (separate nonce-roll/epoch-transition cluster). No
mainnet-complete `ProtocolParameters` fidelity (empty-Conway-block validation invariance is the
scope). No new BLUE authority/type, no new durability subsystem, no containment-gate relaxation,
no grounding-doc regeneration (that's `/cluster-close`).
