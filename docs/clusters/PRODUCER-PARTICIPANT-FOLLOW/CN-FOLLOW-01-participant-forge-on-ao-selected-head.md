# Slice CN-FOLLOW-01 — Participant venue forges on the AO-selected durable head

> **Status:** IMPLEMENTED + sealed. Hermetic MAC green (8 tests + `ci_check_participant_forge_on_selected_head.sh`); per-slice security review PASS (no HIGH+). Live forge-readiness + preview-adoption verification pending — NO BA02 claim until the Haskell peer logs `AddedToCurrentChain` for Ade's forged hash.
> **Cluster:** PRODUCER-PARTICIPANT-FOLLOW
> Planning artifact. Normative documents + CI enforcement are authoritative.

---

## 1. Intent

Make it impossible for a keyed producer running in the **Participant** venue to follow the
AO-selected chain correctly yet be unable to produce on it.

Stated as correctness: **a keyed Participant producer forges if and only if it is leader on the
AO-selected durable head (`ChainDb::tip`) with no fork-choice decision pending — never on a
private or stale spine, and never gated out by a per-tick exact-equality re-check that the racing
live frontier makes permanently unsatisfiable.**

This closes the verified gap (2026-06-19): the `--participant-venue` forge followed for ~5 h
(760 admits, AO routing, caught up, stable) but produced **0** blocks — `forge_result` =
18052 `no_tip_available` + 988 `not_leader`. The Participant venue takes the `else` pure
DC-NODE-15 gate (`durable_servable_tip == followed_peer_tip`, exact) on **every** tick and never
transitions to an extend mode, so it refused ~95% of ticks and missed leader slot 115152430.

---

## 1a. Authority boundary + tightened contract (load-bearing)

**Wording guard.** A keyed producer running with **participant follow authority** may forge —
participants do NOT forge by virtue of being participants. Following a public multi-producer chain
is participant/AO chain-selection authority; forging is producer authority (producer keys + leader
proof + the AO-selected durable head). The two are distinct (CN-FOLLOW-01).

The slice is sealed against these six clauses (each mechanically enforced):

1. **Separate decision.** `participant_forge_decision` is NEW and distinct from
   `single_producer_forge_decision`; the latter is unmodified (only an unreachable fail-closed arm)
   and re-asserted byte-identical (`single_producer_forge_decision_unchanged`).
2. **Keyed condition.** A Participant venue enters the extend/forge mode ONLY if producer keys (the
   forge activation) are present and valid; a non-keyed Participant venue is non-producing forever
   (`participant_venue_requires_forge_activation`).
3. **Fork-choice fence.** No forge while ANY of `pending_reselection` / `pending_fork_switch` /
   `pending_missing_bridge` is set (DC-NODE-28) — each yields a typed `ForkChoicePending` refusal
   (`participant_forge_refused_while_fork_choice_pending` asserts all three separately).
4. **Forge base.** The base is the AO-selected `ChainDb::tip` — never `followed_peer_tip`, never
   `observed_peer_tip`, never a stale durable tip, never a local private spine
   (`participant_forge_base_is_ao_selected_chaindb_tip`, `..._is_servable_before_forge`).
5. **No-tip behaviour.** `no_tip_available` is acceptable ONLY before the initial caught-up →
   extend transition; it must NOT be the steady-state outcome (the pre-fix ~95% per-tick race). The
   latch-on-first-caught-up replaces the per-tick exact-equality re-check, so a post-latch
   leader-slot tick on the AO-selected head forges or refuses with a typed fence — never
   `no_tip_available`.
6. **Evidence semantics.** Convergence evidence proves follow / admission / agreement ONLY. BA02
   requires the Haskell peer's `AddedToCurrentChain` for Ade's exact forged block hash — claimed
   only after that correlation, and only after separately proving the keyed Participant reaches the
   live extend/forge state.

---

## 2. Scope

- **Modules / crates:**
  - `crates/ade_node/src/node_sync.rs` — add a Participant forge-decision parallel to
    `single_producer_forge_decision`; add the Participant extend `ForgeMode` variant + its
    `forge_mode_on_caughtup`-style transition. `single_producer_forge_decision` is **not modified**.
  - `crates/ade_node/src/node_lifecycle.rs` — the `ForgeTick` arm `proceed_to_forge` block
    (`:1836-1894`): the `else` (non-SingleProducer) branch routes `VenueRole::Participant` to the
    new decision; `VenueRole::Unknown` keeps the pure DC-NODE-15 path unchanged. The forge-base
    evidence block (`:1900-1924`) records `ForgeBaseSource::LocalChaindbTip` for Participant too.
  - `ci/` — one new gate (e.g. `ci_check_participant_forge_on_selected_head.sh`).
  - `docs/ade-invariant-registry.toml` — register CN-FOLLOW-01 + the derived mechanics rule;
    append `strengthened_in` on DC-NODE-28 and DC-NODE-15.
- **State machines affected:** the forge `ForgeMode` (a new Participant extend state + transition).
  The AO fork-choice / `select_best_chain` state machine is **not** touched.
- **Persistence impact:** none new. The forged block is durably admitted through the existing
  `pump_block` authority (DC-NODE-05); no new WAL record shapes, no checkpoint changes.
- **Network-visible impact:** none new. Signing + serve are unchanged.
- **Out of scope (explicit):** the AO/fork-choice selection law (consumed, never re-implemented);
  the single-producer path; the adoption channel (duplex diffusion / node dialing Ade's serve —
  a separate downstream gap, moot until the forge produces); preprod; throughput; KES.

---

## 3. Entry obligations — ANSWERED (the pre-code gate)

### Obligation 1 — Is DC-NODE-20 sufficient as the forge-base guard for an AO-selected multi-producer head, or is an additional guard required?

**Answer: DC-NODE-20 is insufficient AND is the wrong vehicle; the additional guard already
exists — DC-NODE-28. No new fork-choice guard is required.**

- DC-NODE-20 (registry) is explicitly **rung-1 single-producer only** ("real fork-choice /
  multi-producer (rung 2) … are OUT of scope"). Its 6-condition observed-feed fence —
  condition (2) "NO competing block has been observed … if observed, fail closed, do NOT resolve
  (that is rung 2)" and condition (6) "no fork-choice required, mechanically DERIVED from (2)" —
  **assumes no competing candidate.** A Participant venue following a multi-producer network
  observes competing candidates routinely.
- `single_producer_forge_decision` (`node_sync.rs:1134-1200`) hard-rejects `venue_role !=
  VenueRole::SingleProducer` (`:1165`) and fails closed on a competing `observed_peer_tip`
  (`:1174-1183`). Reusing it for Participant would fail closed on the first competing block.
- The rung-2 guard already exists and is ENFORCED: **DC-NODE-28** ("No forge across unresolved
  re-selection … once a peer-origin candidate is classified `NeedsForkChoice` in a Participant
  venue, forging is DISABLED until the fork-choice outcome is durably applied/reconciled or
  rejected; the forge base is NEVER a stale pre-resolution `ChainDb::tip` while a decision is
  pending"). It is already wired into the ForgeTick fence (`pending_reselection`/
  `pending_fork_switch`/`pending_missing_bridge`, `node_lifecycle.rs:1735-1741, 1765-1768`).
- **Conclusion:** the Participant forge-decision builds on the AO-selected `ChainDb::tip` fenced
  by **DC-NODE-28** (a competing candidate is *resolved by the AO*, not fail-closed as in the
  single-producer observed-feed fence). DC-CONS-24 (forged parent byte-equals the served tip) and
  DC-NODE-14 (parent is peer-FindIntersect-able) still apply unchanged because the forge base is
  the same servable `ChainDb::tip`.

### Obligation 2 — Is the served projection populated on the participant path so `durable_servable_tip` is available?

**Answer: Yes — the base is available; the ~95% `no_tip_available` is the DC-NODE-15
exact-equality refusal, not a missing base.**

- `ChainDbServedSource` (`served_chain_projection.rs:76-84`) is a thin **borrow-only** projection
  of the shared ChainDb; `tip()` (`:243-251`) reads **directly** via `last_block_bytes()` on every
  call — no separate handle to install. It returns `None` only if the store is empty / read-errors
  / undecodable.
- Participant sync admits through `pump_block` to the **same** `Arc<PersistentChainDb>` the
  ForgeTick reads (`node_lifecycle.rs:417-420, 1082, 3487, 3495/3520`); the durable tip advances
  with each admit (live `durable_tip_slot` evidence), and the SingleProducer venue already forges
  on that very `ChainDb::tip`.
- Therefore `selected_tip`/`durable_servable_tip` is `Some` after the first admit. The 95%
  `no_tip_available` (`!forged`, `:2043-2052`) is `proceed_to_forge == false` from
  `dc_node_15_refusal` (`durable_servable_tip == followed_peer_tip` fails because Ade is ~1 behind
  the racing live tip), **not** an absent base.
- **Residual slice check (MAC §12):** confirm the base used by the new decision is the same
  populated `ChainDb::tip` and is servable (`ChainDbServedSource::tip()` `Some`) before forging.

### Obligation 3 — Do the startup-intersect and live-rollback legs remain covered by the participant follow authority once the keyed Participant venue forges?

**Answer: Yes — fully covered and unaffected. The forge is strictly downstream of the sync step,
and the existing fences gate it.**

- The ForgeTick (incl. the new decision) fires **after** `SyncOnce`. The startup intersect
  (`wire_pump_start_point`, shared single anchor, `node_lifecycle.rs:891-898`) executes once at
  pump startup, before any venue/forge. Live rollback is resolved in `run_participant_sync` every
  `SyncOnce` under DC-NODE-29 **stored-only** authority (`:3567-3614`: unknown hash / slot mismatch
  fails closed before any durable mutation). The AO/LCA walk (DC-NODE-38, `:3149-3223`) runs at
  follow-time; a walk failure emits a structured `MissingBridge` and **holds** the forge fence
  (`pending_missing_bridge`).
- Reaching the extend decision changes **none** of this: the forge base is read fresh from
  `ChainDb::tip` (`:1775`); `pending_reselection`/`pending_fork_switch`/`pending_missing_bridge`
  gate the ForgeTick (`:1765-1768`), and the new decision respects the same fence. Leg-1 (startup
  orphan) → `MissingBridge`/`UnsupportedRollbackPoint` fail-closed hold (no crash, no forge);
  leg-2 (live rollback / competing branch) → DC-NODE-29 + DC-NODE-38 participant authority.
- **Conclusion:** the slice adds a forge *output*; it does not alter the *input* sync legs. Both
  remain participant-authority and fail-closed.

---

## 4. Execution boundary (TCB color)

- **BLUE (unchanged):** chain selection / fork-choice (`select_best_chain`, `ade_core::consensus`),
  the ledger + header authority, `pump_block`'s validation. The slice **consumes** the BLUE-selected
  durable head; it does not select, reorder, or prefer chains.
- **GREEN:** the Participant forge-**decision** + `ForgeMode` transition (`node_sync.rs`) and its
  ForgeTick routing (`node_lifecycle.rs`). Deterministic glue: it decides *whether* to forge on the
  already-selected durable head, gated by the existing fences. It MUST NOT affect which chain is
  authoritative.
- **RED (unchanged):** KES/VRF signing (`kes_sign_header_advancing`), wire pump, serve. No key
  material enters the GREEN decision.

Rule check: no RED behavior in BLUE; the GREEN decision never reaches `select_best_chain` /
`chain_selector` / `fork_choice` (it reads their durable result). Ambiguity: none — the forge
decision is non-authoritative glue over a BLUE-selected tip.

---

## 5. Invariants preserved (must remain true)

- Deterministic replay equivalence: same canonical inputs (admitted chain + leader schedule) →
  byte-identical forge decisions + forged blocks; the forged block is durably admitted via
  `pump_block` and replays identically.
- DC-NODE-05 remains the SOLE durable-admit authority (the decision only READS `ChainDb::tip`).
- DC-NODE-15 (catch-up), DC-NODE-23/25/26 (fork-choice classify/apply/reconcile), DC-NODE-28
  (no-forge-across-pending), DC-NODE-29 (stored-only rollback target), DC-NODE-38/39/40 (LCA walk /
  MissingBridge / rollback retention), DC-CONS-03 (chain selection authority), DC-CONS-24 (forged
  parent byte-equals served tip), DC-NODE-14 (parent FindIntersect-able), DC-NODE-21 (cert
  evidence-only), CN-CONS-03 (multi-node convergence) — all preserved.
- SingleProducer venue behavior is byte-for-byte unchanged (`single_producer_forge_decision` not
  modified).
- Signing stays RED/keyed; no key material in participant/AO code.

---

## 6. Invariants strengthened / introduced

- **CN-FOLLOW-01 (NEW, true-tier) — producer/follow authority separation.** Two clauses:
  (a) *deterministic selection*: the same candidate set yields the same selected canonical durable
  tip (the AO/`select_best_chain` law, arrival-order-independent); (b) *forge-only-on-selected-head*:
  a keyed producer forges only on the AO-selected durable head, never on a private/stale spine and
  never with a hidden authority. This is the project law the slice enforces for the Participant venue.
- **DC-FOLLOW-FORGE-01 (NEW, derived — CANDIDATE id, assigned at /cluster-plan) — Participant
  forge-decision mechanics.** The keyed Participant venue uses an initial-catch-up→extend mode
  (mirroring the single-producer two-state mode): `UseInitialCatchupGate` (DC-NODE-15) until the
  first caught-up instant, then a Participant extend state that forges on `ChainDb::tip` (the
  AO-selected durable head) gated by **DC-NODE-28** (no pending fork-choice/reselection) — NOT the
  single-producer observed-feed fence, and NOT the per-tick DC-NODE-15 exact-equality re-check.
- **DC-NODE-28 (STRENGTHENED, `strengthened_in += CN-FOLLOW-01`):** its pending-reselection fence
  now gates an *active* Participant forge path (previously the Participant venue could not forge at
  all, so the fence was vacuous on the produce side).
- **DC-NODE-15 (STRENGTHENED, `strengthened_in += CN-FOLLOW-01`):** for the Participant venue it is
  the **initial** catch-up gate with a one-way transition to the extend state, not a per-tick
  re-check.

---

## 7. Design summary

The SingleProducer venue already has a two-state forge mode: `UseInitialCatchupGate` (DC-NODE-15
exact-equality) → on the first caught-up instant (`forge_mode_on_caughtup`) → `ExtendOwnSpine`
forging on `ChainDb::tip` under the observed-feed fence. The Participant venue (the `else` branch,
`node_lifecycle.rs:1881-1893`) has **no** such transition — it re-runs the exact-equality gate
every tick and the racing frontier makes `durable == live-tip` true only ~5% of the time.

The slice gives the Participant venue the analogous transition, with the extend fence supplied by
the AO instead of the single-producer observed-feed fence:

- Initial: `UseInitialCatchupGate` — the existing DC-NODE-15 gate, until the first caught-up instant.
- Transition: on first caught-up, advance to `ParticipantExtendOnSelectedHead` (the Participant
  analog of `forge_mode_on_caughtup`).
- Extend: `participant_forge_decision` returns `ExtendOnSelectedHead { forge_base = ChainDb::tip }`
  **iff** venue == Participant, no fork-choice/reselection pending (DC-NODE-28), the spine is
  contiguous/servable, and the base is the durable `ChainDb::tip`; otherwise a typed `Refuse`.

Because a competing candidate is *resolved by the AO* (DC-NODE-23/38) and the resolution holds the
forge fence while pending (DC-NODE-28), the Participant extend state never needs the single-producer
"fail-closed on any observed competitor" rule. Enforcement is type-level (a new `ForgeMode` variant
+ a total decision function returning a closed enum) plus a CI grep that the decision reads
`ChainDb::tip` and never calls the selector.

---

## 8. Changes introduced

### Types
- New `ForgeMode::ParticipantExtendOnSelectedHead { adopted_root, current_tip }` (or a shared
  generalization), and a closed `ParticipantForgeDecision { ExtendOnSelectedHead { forge_base } |
  UseInitialCatchupGate | Refuse(ForgeRefused) }`.

### State transitions
- New `participant_forge_decision(mode, durable_servable_tip, followed_peer_tip, venue_role,
  pending_reselection, pending_fork_switch, pending_missing_bridge) -> ParticipantForgeDecision`
  (total; fail-closed `Refuse` on every off-nominal input).
- New caught-up transition for the Participant mode (initial → extend), mirroring
  `forge_mode_on_caughtup`.
- Modified `node_lifecycle.rs` ForgeTick `proceed_to_forge`: the `else` branch dispatches
  `VenueRole::Participant` to `participant_forge_decision`; `VenueRole::Unknown` unchanged.
- Modified forge-base evidence (`:1900-1924`): emit `ForgeBaseSource::LocalChaindbTip` +
  `cert_path_present: false` for Participant too.

### Persistence
- None. Forged block durably admitted via `pump_block`. The in-memory `ForgeMode` is replay-derived
  on restart (re-catches-up → re-transitions), exactly as the single-producer mode is today.

### Removal / refactors
- None. `single_producer_forge_decision` untouched.

---

## 9. Replay, crash, and epoch validation

- **Replay:** `participant_forge_two_runs_byte_identical` — same admitted chain + leader schedule
  replays to byte-identical forged blocks and decisions. `forward_sync_replay_two_runs_byte_identical`
  stays green.
- **Crash/restart:** a crash before `pump_block` leaves no forge state; warm-start recovers the
  durable head and the mode re-derives to `UseInitialCatchupGate` → re-catches-up. Proven by a
  recover→forge re-entry test on the Participant venue.
- **Epoch boundary:** not applicable to the decision; the leader schedule + KES period handling are
  unchanged (DC-CRYPTO-10 preserved).

---

## 10. Mechanical acceptance criteria

Complete only when **all** exist and pass in CI:

- [ ] `participant_venue_forges_on_ao_selected_head_when_leader` — keyed Participant venue, caught
      up, leader at slot S, no pending fork-choice → forges on `ChainDb::tip`.
- [ ] `participant_forge_base_is_ao_selected_chaindb_tip` — forge base == `ChainDb::tip`
      (`ForgeBaseSource::LocalChaindbTip`), never `followed_peer_tip`, never a cert, never Origin.
- [ ] `participant_forge_base_is_servable_before_forge` — `ChainDbServedSource::tip()` is `Some`
      and byte-equals the forge base at forge time.
- [ ] `participant_forge_refused_while_fork_choice_pending` — `pending_reselection` /
      `pending_fork_switch` / `pending_missing_bridge` set → typed `ForgeRefused`, no forge
      (DC-NODE-28).
- [ ] `participant_venue_requires_forge_activation` — a non-keyed Participant venue cannot forge.
- [ ] `single_producer_forge_decision_unchanged` — existing single-producer forge tests stay green
      (`caughtup_self_admit_enters_extend_directly_no_cert`, `local_spine_*`).
- [ ] leg-1 regression: `orphaned_startup_holds_forge_fence_participant` (MissingBridge /
      UnsupportedRollbackPoint hold, no crash, no forge).
- [ ] leg-2 regression: existing DC-NODE-29 / DC-NODE-38 participant rollback + LCA-walk tests stay
      green with the forge active.
- [ ] `participant_forge_two_runs_byte_identical` (replay-equivalence).
- [ ] `ci_check_participant_forge_on_selected_head.sh` — the Participant decision reads
      `ChainDb::tip`, never calls `select_best_chain`/`chain_selector`/`fork_choice` (no duplicate
      fork-choice), never constructs a FindIntersect point-list (no DC-NODE-42), no `kes`/`vrf`
      skey symbols in the participant/AO decision module (signing stays RED), no Origin fallback
      when a durable head exists.

**Non-mechanical acceptance gate (claim discipline, not CI):** no BA02 claim until
`cardano-node-preview` logs `AddedToCurrentChain` for Ade's exact forged hash.

---

## 11. Failure modes

- Fork-choice pending at a leader slot → typed `ForgeRefused` (fail-closed, recoverable next tick;
  no replay impact).
- Base absent / spine non-contiguous / not servable → typed `Refuse` (fail-closed).
- Orphaned startup tip → `MissingBridge`/`UnsupportedRollbackPoint` hold (fail-closed; no forge).
- Unknown/slot-mismatched rollback point → DC-NODE-29 fail-closed before any durable mutation.
All forge-affecting failures are fail-fast typed refusals; none mutate durable state or replay.

---

## 12. Hard prohibitions

Inherited cluster prohibitions apply. Slice-specific:
- No duplicate fork-choice — the decision consumes the AO result; never calls the selector.
- No weakened anchor floor; no Origin fallback when a durable head exists.
- No signing or key material in participant/AO/decision code (signing stays RED).
- No resurrection of the quarantined DC-NODE-42 FindIntersect point-list.
- No modification of `single_producer_forge_decision` or the BLUE selection law.
- No `HashMap`/wall-clock/rand/float in the decision; no `String`/`anyhow` errors in the typed
  decision result; no TODOs/placeholders in the forge path.

---

## 13. Explicit non-goals

This slice MUST NOT: implement or alter the adoption channel (duplex diffusion / node dialing
`:3033`); change the AO/fork-choice selection law; add throughput/pipelining; touch KES/VRF;
introduce new protocol versions or feature flags beyond the existing `--participant-venue`; prepare
for preprod or track_utxo=true. Any work outside §2 Scope is scope creep.

---

## 14. Tier framing (do not flatten to true-tier)

| Requirement | Tier | Why |
|---|---|---|
| same selected canonical tip from the same candidate set | **true** | deterministic chain-selection law (CN-FOLLOW-01a / CN-CONS-01) |
| producer forges only on the selected head | **true** | replay / causality / no hidden authority (CN-FOLLOW-01b) |
| AO fork-choice agrees with Cardano/Haskell | **derived** | Cardano compatibility (CN-CONS-03 / DC-CONS-*) |
| participant-venue convergence evidence | **release/evidence** | proof artifact (closed convergence vocabulary) |
| preview adoption (`AddedToCurrentChain`) | **bounty** | public acceptance check |
| topology / key staging | **operational** | venue mechanics |

The core law (CN-FOLLOW-01) is true-tier; the concrete Cardano fork-choice agreement is derived;
evidence/bounty/operational stay separate. The implementation must not blur project law with
Cardano-specific shape.

---

## 15. Review notes / implied follow-ups

- Invariant risk considered: a Participant forge could (wrongly) bypass fork-choice — mitigated by
  reusing the existing DC-NODE-28 fence rather than a new path, and by the CI grep forbidding any
  selector call in the decision.
- Open design choice for implementation: whether the "caught-up" transition predicate for the
  Participant extend state should be exact-equality-once (then latch) or a frontier-proximity test;
  resolve before code, defaulting to the single-producer "first caught-up instant latches the
  extend mode" semantics to minimize divergence.
- Follow-up (separate slice): the adoption channel — Ade's `--peer` follow advertises
  `InitiatorOnlyDiffusionMode` and the node never dials Ade's serve `:3033`; required for a real
  BA02 `AddedToCurrentChain`, moot until this slice lets the forge produce.
