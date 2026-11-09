# Invariant Sketch — PHASE4-N-F-G (RO-LIVE-01: observable block production on the `--mode node` relay spine)

> **Type:** IDD invariant sketch (Part I). Planning artifact — no clusters, slices, or
> code yet. Predecessor: PHASE4-N-F-F (operator-key ingress + forge-on flip), closed +
> pushed at `origin/main` `f08b12ca`.
> Authority read end-to-end first: the four grounding docs at HEAD `4eb7610`
> (CODEMAP / SEAMS / HEAD_DELTAS / TRACEABILITY) + the registry rules RO-LIVE-01,
> CN-CONS-06, CN-NODE-01/02/03, DC-NODE-05, CN-PROD-04, CN-CONS-07, CN-WIRE-08,
> CN-FORGE-01..04, CN-OPCERT-01, CN-GENESIS-01, RO-LIVE-06 + the C1 scoping pass
> (`operator-pass-live-leg-c1-scoping.md`, mined for its gap analysis, NOT followed
> literally — it is produce_mode-centric and predates N-F-F).

---

## 0. The honest framing (read first)

RO-LIVE-01's headline — "a Haskell cardano-node peer issuing RequestRange covering an
Ade-forged block receives bytes that pass that peer's full header+body validation" —
**cannot be expressed as a pure `canonical input → canonical output` transformation.**
The acceptance is a live, nondeterministic, external event: a real peer's verdict over a
real socket. Per IDD that means the *acceptance itself is not authoritative core work* —
it is operator-gated evidence captured from an external authority (the peer's log),
exactly the shell-must-not-overstate / evidence-reducers-are-green-not-authority line.

So the cluster decomposes into two halves with sharply different IDD status:

- **MECHANICAL half (this cluster builds + closes):** everything *up to the wire
  boundary* — the live-feed wiring (Leg A), the sibling serve path from the node spine
  (Leg B), the real opcert/genesis ingress, the slot-alignment map, the epoch-boundary
  fail-closed guard. Each **is** a pure transformation, gets real invariants + CI gates,
  and is hermetic / loopback-testable.
- **OPERATOR-GATED half (stays `partial`):** the live ACCEPT and the BA-02 evidence.
  `blocked_until_operator_stake_available` — **NOT deferred** (per CN-CONS-06's own
  discipline). A peer only accepts if Ade is a legitimate slot leader (real stake +
  registered pool), which the docker preprod peer can never grant (public preprod, Ade
  has no stake → never leader there).

### Resolved load-bearing calls (this sketch is saved with these fixed)

- **OQ1 → `--mode node` relay spine.** RO-LIVE-01 lives on the node lifecycle that
  syncs, forges, and serves as one operator-facing path — **not** `produce_mode`.
  `produce_mode` stays useful precedent but is **not** the authority path for RO-LIVE-01.
  Forge stays **subordinate to sync**.
- **OQ2 → Shape B: sibling serve task.** Do **not** extend the relay-loop `ForgeTick`
  branch to admit/serve directly — that would relax the N-F-E/N-F-F containment gate in
  the most dangerous place. Instead, a **sibling serve task fed only by self-accepted
  forged artifacts** from the fenced forge path owns served-chain mutation. The relay
  loop keeps exactly one fenced `forge_one_from_recovered`, advances no durable tip from
  forge, and performs no direct serve/admit/gossip in its body. The handoff to the
  sibling is a typed channel send (not a forbidden serve token), so the containment gate
  stays **semantically unchanged**.
- **OQ3 → single recovered seed epoch only.** RO-LIVE-01 needs **one** accepted block,
  not cross-epoch production. Crossing the epoch boundary must **fail closed**; the
  nonce-roll / epoch-transition machinery is a **separate cluster**. Do not patch nonce
  rollover into this cluster — signing past the boundary with a stale `eta0` is a
  peer-reject class (verified hard limit, C1 doc §4a).

> **GUARDRAIL (do not weaken):** PHASE4-N-F-G may add a **served-chain handoff gate**,
> but **must not relax the relay-loop containment gate**
> (`ci_check_node_run_loop_containment.sh`, CN-NODE-02 / DC-NODE-05 / CE-E-4). No
> BA-02 / RO-LIVE closure claim until operator-gated evidence exists.

---

## 1. What must always be true

**Leg A — live feed into the relay spine (the consume side that makes `ForgeTick` fire):**

- **A1.** A live cardano-node peer's block bytes enter the spine through the *same*
  closed `NodeBlockSource::WirePump` contract — ordered block bytes only, never a verdict
  (strengthens **DC-SYNC-01 / DC-SYNC-02**). The live source is a *fill* of the existing
  closed contract, **not a new plugin point** (this is the SEAMS §7 candidate-#1
  confirmation: `NodeBlockSource` stays a closed verdict-decoupled contract).
- **A2.** With a live (Continuing) feed, `LoopStep::ForgeTick` becomes reachable; the
  forge fires *because* the feed is Continuing — forge stays **subordinate to the feed**
  (CN-NODE-02 / DC-NODE-05 hold; the planner's forge input stays the content-blind
  `Due | NotDue`).
- **A3.** The durable tip still advances **only** via `run_node_sync → pump_block`
  (DC-SYNC-01). Wiring a live source adds no second tip-advance path and no second
  bootstrap (**CN-NODE-01 / CN-NODE-02** byte-unchanged on the consume side).

**Leg B — sibling serve task + the self-accept→serve handoff (the new authority surface):**

- **B1 (the new handoff invariant — DC-NODE-06).** **Only BLUE self-accepted forged
  artifacts may enter the sibling serve task.** The serve task must **not** accept raw
  forged bytes, failed forge outputs (`ForgeNotLeader` / `ForgeFailed`), a self-declared
  acceptance flag, or a peer-verdict substitute. The handoff carries a typed,
  constructor-fenced artifact whose only provenance is a `ForgeSucceeded` outcome (which
  by CN-FORGE-01 is emitted only when BLUE `self_accept` accepts). Strengthens
  **CN-PROD-04 / CN-CONS-07** (the node-spine analog of produce_mode's broadcast bridge).
- **B2.** Served-chain mutation happens **only** in the sibling serve authority via the
  single **`ServedChainHandle::push_atomic`** authority — never in the relay-loop body.
- **B3.** The block-fetch response bytes are **byte-identical** to the self-accepted
  forged bytes, tag-24-wrapped via the *single* **CN-WIRE-08** authority — no parallel
  serializer (`compose_blockfetch_block` = `tag24(bytes([era, block]))`).
- **B4.** Acceptance is proven **only** by the peer's validation log through
  `ba02_evidence::correlate` (the sole `Ba02Manifest` constructor) — never by Ade's
  self-accept / `ForgeSucceeded` / any wire-success signal (**RO-LIVE-06** holds).

**Single-epoch / epoch-boundary (the new fail-closed invariant — DC-EPOCH-03):**

- **E1.** Forge is valid **only within the single recovered seed epoch**. Crossing the
  epoch boundary **fails closed** — a slot past the recovered seed epoch cannot be forged,
  served, or signed as a valid block. The seed-epoch nonce (`eta0`) is frozen at the
  recovered value; the forge apply path does **not** drive the BLUE
  `CandidateFreeze` / `EpochBoundary` nonce transitions (they exist but nothing drives
  them on the forge path — C1 doc §4a). Cross-epoch production is a **separate
  nonce-roll/epoch-transition cluster**.

**Ingress fidelity (so a real peer *would* accept — closes C1-doc gaps G1/G2/G4 on the node path):**

- **C1.** Real cardano-cli `node.opcert` text-envelope ingress via the dormant
  `producer::opcert_envelope::parse_opcert_envelope` (**CN-OPCERT-01** strengthened;
  un-dormants the N-R-C parser), in place of `parse_simple_opcert_json` on the node path.
- **C2.** Real `shelley-genesis.json` ingress via
  `producer::genesis_parser::parse_shelley_genesis` (**CN-GENESIS-01** strengthened),
  in place of `parse_simple_genesis_json` on the node path.
- **C3.** `protocol_version` / `ProtocolParameters` / `prev_opcert_counter` derive from
  the loaded genesis/opcert, **not** `::default()` / hardcoded — so the forged header
  passes the peer's protocol-version + opcert-counter validation. (Honest-scope defaults
  were acceptable for N-F-F ingress wiring; a live accept requires real values.)

**Slot alignment (G7):**

- **D1.** The forge slot tracks the peer's live wall-clock slot via the genesis
  `slot_zero_time + slot_length` map; the map is a **pure, total function** of
  (genesis anchor, wall-clock-millis) read at the single RED `Clock` seam (**DC-NODE-03**).
  `SlotDrift` **fails closed** (it must not be swallowed).

## 2. What must never be possible

- **N1.** Serve/broadcast a forged block that failed BLUE `self_accept` (B1/B2 — no serve
  without self-accept). The serve task cannot be fed raw forged bytes or a failed outcome.
- **N2.** Overstate acceptance — no "accepted" / `Ba02Manifest` from Ade's own
  self-accept, `ForgeSucceeded`, `block_received`, or any wire-success signal; only the
  peer's allow-listed accept log through `correlate`.
- **N3.** **Forge across an epoch boundary with a stale `eta0`** (E1). Forging past the
  boundary with a frozen seed-epoch nonce → guaranteed peer reject; it **must fail
  closed**, never silently sign with a stale nonce.
- **N4.** Equivocate — never two different blocks signed for the same slot with the same
  VRF/KES (one-producer-per-key); never reuse/skip an opcert counter that strands the pool.
- **N5.** Trust a from-genesis/tip seed bundle without binding it to a verifiable anchor
  (poisoned-seed defense — the importer's validation is purely structural; the recovered
  seed-epoch surface must be genesis-consistent — see OQ5).
- **N6.** Let the live feed's nondeterminism reach BLUE — only canonical `SlotNo` and
  canonical block bytes cross the RED seam.
- **N7.** Turn `NodeBlockSource` into a plugin point, or add a wildcard / second forge
  codepath / second bootstrap (CN-NODE-01 / CN-NODE-03 closures hold).
- **N8.** Relax the relay-loop containment gate — the loop body must keep exactly one
  fenced `forge_one_from_recovered`, no `served_chain_admit` / `push_atomic` /
  `OutboundCommand` / `broadcast` / `block_fetch` token. The sibling serve task is the
  *only* new home for those tokens.

## 3. What must remain identical across executions (deterministic surface)

- **I1.** Given the same recovered `BootstrapState` + the same ordered feed of peer block
  bytes + the same injected clock sequence, the **forge decision sequence** (leader /
  not-leader, the forged bytes) is byte-identical. (This is the N-F-F S4
  `relay_loop_with_operator_material_two_runs_byte_identical` property — it must survive a
  live-wired source: the live bytes are the canonical input.)
- **I2.** The served block-fetch response bytes for a given forged block are deterministic
  (tag-24 wrap is pure; B3).
- **I3.** The slot map `(genesis_anchor, wall_clock_millis) → SlotNo` is a pure, total
  function (D1).
- **I4.** The self-accept→serve handoff is deterministic: a given `ForgeSucceeded`
  artifact yields one served-chain admission via `push_atomic`; a non-success outcome
  yields none (B1/B2).

## 4. What must be replay-equivalent

- **R1.** The captured live feed (ordered peer block bytes) + the recovered checkpoint +
  WAL → **byte-identical post-state AND byte-identical forge sequence**. The live wire is
  the nondeterministic *source*; once captured as canonical ordered bytes, replay must
  reproduce. (Extends the N-F-A/N-F-C/N-F-E replay-equivalence obligation to the
  live-wired feed.)
- **R2.** `correlate(forged_artifact, peer_accept_log)` → byte-identical `BA02Outcome` on
  replay (existing RO-LIVE-06 property; unchanged).

## 5. State transitions in scope

| # | Transition | Color | Status |
|---|---|---|---|
| T1 | `(Disconnected, dial(peer_addr)) → Result<(Connected{mux}, [tcp_open, handshake]), DialError>` | RED | exists (`n2n_dialer`) — **wire into binary** |
| T2 | `(SessionState, frame_bytes) → Result<(SessionState', [Block→WirePump mpsc]), SessionError>` | GREEN | exists (`session::core::step`) |
| T3 | `(LoopState, Continuing, ForgeSlotStatus, Shutdown) → LoopStep` | GREEN | exists (`plan_loop_step`) — now reaches `ForgeTick` |
| T4 | `(recovered BootstrapState, SlotNo, ProducerShell) → Result<(ForgeSucceeded∣ForgeNotLeader, [self_accept]), ForgeError>` | BLUE | exists (`forge_one_from_recovered`) |
| **T5** | `(ServedChain, SelfAcceptedForgedArtifact) → Result<(ServedChain', [push_atomic]), ServedChainAdmitError>` in the **sibling serve task** | BLUE-authority / RED-task | **NEW (DC-NODE-06)** — fenced handoff; only a self-accepted artifact may construct the input |
| T6 | `(ServedChainSnapshot, RequestRange) → Result<(MsgBlock=tag24(bytes), [outbound]), …>` | BLUE+RED | exists (`block_fetch::server` + `OutboundCommand→MuxPump`) |
| **T7** | `(genesis_anchor, wall_clock_millis) → Result<SlotNo, SlotDriftError>` | BLUE/GREEN | **NEW (D1)** — slot-align map, fail-closed on drift |
| **T8** | `(seed_epoch, candidate_slot) → Result<InEpoch(SlotNo), EpochBoundaryFailClosed>` | BLUE/GREEN | **NEW (DC-EPOCH-03)** — single-epoch guard; off-epoch fails closed |
| **T9** | `(opcert_envelope_bytes) → Result<OperationalCert, OpCertError>` | BLUE-validate / RED-read | **NEW reach (C1)** — un-dormant real parser |
| **T10** | `(shelley_genesis_bytes) → Result<(ShelleyGenesis, pparams, protocol_version), GenesisError>` | BLUE-validate / RED-read | **NEW reach (C2/C3)** — un-dormant real parser; derive constants |
| T11 | `(AdeForgeRecord, peer_accept_log) → BA02Outcome` | GREEN | exists (`correlate`) — operator-gated input |

## 6. TCB color hypothesis

- **BLUE (all already exist; cluster mostly *consumes*):** forge, leader-check,
  self-accept, `served_chain_admit` / `push_atomic`, block-fetch serve composition,
  tag-24 wrap, the opcert/genesis structural parsers, era schedule, the nonce authority +
  its (undriven) epoch-boundary transitions. The genuinely open BLUE questions are
  **T8's epoch-boundary fail-closed guard** (new rule DC-EPOCH-03 — BLUE or GREEN guard?)
  and whether **T7's slot map** is BLUE or GREEN-by-content.
- **GREEN:** the wire session reducer, run-loop planner, `correlate` (all exist); the
  slot-align map (T7) and the epoch guard (T8) are likely GREEN-by-content or
  promotable-BLUE; the typed self-accept→serve handoff envelope (T5 input type) is a
  GREEN/BLUE constructor fence.
- **RED (the bulk of the new work — *wiring*):** connect
  `n2n_dialer → mux_pump → session → WirePump mpsc` into the `--mode node` arm (replacing
  the empty `in_memory(Vec::new())` at `node_lifecycle.rs:376/441`); the **sibling serve
  task** (listener + `block_fetch::server` + `push_atomic`, fed by the typed handoff
  channel); the real opcert/genesis file readers; the live operator-pass driver; the
  `SystemClock` seam (exists). Key custody stays RED-confined to `ProducerShell`.

**Net:** mostly **RED wiring + consuming existing BLUE/GREEN authorities.** The two real
invariant evolutions are **T5 (self-accept→serve handoff on the node spine, DC-NODE-06)**
and **T8 (epoch-boundary fail-closed, DC-EPOCH-03)** — both *additive* fences that **do
not relax** the relay-loop containment gate.

## 7. Registry surfaces (two new derived rules — proposed; see §9)

- **DC-NODE-06 — self-accept→serve handoff.** *tier: derived.* Only BLUE self-accepted
  forged artifacts can enter the sibling served-chain task; raw forge bytes, failed forge
  outputs, self-declared acceptance, and peer-verdict substitutes are rejected.
  Enforcement: a type/constructor fence on the handoff envelope + a CI ban on raw forge
  bytes / non-success outcomes entering the serve task; the relay-loop containment gate
  stays semantically unchanged.
- **DC-EPOCH-03 — epoch-boundary forge fail-closed.** *tier: derived (Cardano-specific in
  its nonce/KES/Praos details).* No stale seed-epoch forge past the boundary.
  Enforcement: a test proving an off-epoch slot cannot be served/signed as a valid block;
  the BLUE nonce/era authorities are consulted (not bypassed); single-epoch containment.

> **Deliberately NOT a registry rule:** "a peer accepted the block." Peer acceptance is
> **release/operator evidence** (RO-LIVE-01, operator-gated), **not** a runtime invariant.
> RO-LIVE-01 is strengthened (the node-spine observable-forge wiring) but stays `partial`
> / `blocked_until_operator_stake_available`. RO-LIVE-06 (BA-02) stays operator-gated.

## 8. Open questions (carried — resolve before / during `/cluster-plan`)

- **OQ4 (venue):** C1 private testnet (Ade holds ~all stake → the only tractable real
  ACCEPT) vs C2 preprod registered stake (the public bounty surface, but needs ~2 epochs
  of provisioned stake). Either way the live accept is
  `blocked_until_operator_stake_available`; the cluster builds the mechanical wiring and
  the corrected runbook. C1 is the cheaper rehearsal; C2 is the graded deliverable.
- **OQ5 (genesis-consistency of the recovered seed epoch — the deep validity risk):** on
  the node path the leadership source is the recovered `SeedEpochConsensusInputs` (N-F-A).
  For a from-genesis/tip private net, **what populates that recovered state, and is it
  genesis-consistent** (eta0 / stake / ASC / per-pool vrf_keyhash consistent with the
  shared genesis the peer enforces)? Note: node `FirstRun` is **Mithril-only** — a private
  C1 net has no aggregator, so the likely shape is the operator **pre-seeds the store**
  (reusing the proven N-M-C / admission `seed_to_snapshot` extraction) and the node takes
  the **WarmStart** arm — keeping CN-NODE-01 intact (no new bootstrap path). Must be
  verified; this is where the "will the peer accept?" unknown actually lives (slice-entry
  proof obligation: pin Ade's `praos_vrf_input(slot, eta0)` + threshold inputs against the
  peer for the recovered epoch *before* any live KES signature).
- **OQ6 (ranking):** worth ranking RO-LIVE-01 vs the validation-agreement leg
  (false-accept is release-blocking, arguably higher severity) — does not block this
  sketch or the cluster plan.

## 9. Proposed registry entries (await per-entry confirmation before append)

Schema follows the project registry (ziranity-v3 shape): `id` prefix = family;
`status = "declared"` at sketch (tests + ci_script populate at slice time, flip to
`enforced` at close); `introduced_in = "PHASE4-N-F-G"`. The two entries are the **only**
new rules; RO-LIVE-01 / CN-OPCERT-01 / CN-GENESIS-01 / CN-PROD-04 / CN-CONS-07 are
**strengthened** (recorded via `strengthened_in += "PHASE4-N-F-G"` at close), not
re-created. No new rule asserts peer acceptance.

(Full TOML skeletons proposed in chat for confirmation.)

---

## Generation notes

- This sketch is saved with **OQ1/OQ2/OQ3 resolved** (`--mode node` spine; sibling serve
  task — shape B; single recovered seed epoch only, epoch boundary fail-closed) and
  **OQ4/OQ5/OQ6 open**. No BA-02 / RO-LIVE closure claim until operator-gated evidence
  exists.
- The relay-loop containment gate (`ci_check_node_run_loop_containment.sh`) is **not
  relaxed** by this cluster. PHASE4-N-F-G may add a served-chain handoff gate (DC-NODE-06),
  but the loop body stays semantically unchanged (the handoff is a typed channel send, not
  a serve token).
- Next step: `/cluster-plan PHASE4-N-F-G` once the two registry entries are confirmed and
  OQ4/OQ5 are settled enough to order the slices (OQ5 is the load-bearing one — it gates
  the from-genesis/tip seed and the genesis-consistency proof obligation).
