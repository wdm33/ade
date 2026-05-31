# Seams — Where New Work Can Attach (Ade)

> **Status:** Living architectural document. Regenerated; not hand-edited.
> Per-project instance of `~/.claude/methodology/templates/seams.md`.

> 11 crates, **456 canonical types**, **110 CI checks** at HEAD (`cd2484f`, PHASE4-N-F-E cluster close).
> Reads CODEMAP (`docs/ade-CODEMAP.md`, regenerated at the same HEAD) for the module
> list + TCB colors, and the invariant registry (`docs/ade-invariant-registry.toml` —
> **310 entries** at HEAD) for the rule IDs that gate each closed surface.
>
> **This regeneration catches the on-disk SEAMS up through TWO clusters.** The prior file was generated
> at the PHASE4-N-F-C close (header `71a6c80` / 108 CI checks / 303-306 rules). It is brought current
> through **PHASE4-N-F-D** (closed at origin/main `7de1462` — the live relay run-loop) AND
> **PHASE4-N-F-E** (closing now at `cd2484f` — forge-tick on the relay-loop spine). Both deltas are
> **almost entirely surface REDUCTIONS — closed-vocabulary extensions and a closed opt-in activation
> bundle, NOT new extension points.** **No BLUE crate was modified by either cluster** — the 456
> canonical-type total is unchanged; all N-F-D/N-F-E code lands in the RED binary/driver crate `ade_node`
> (`run_loop_planner` GREEN-by-content, `node_lifecycle` RED, `node_sync` RED).
>
> **Boundary language (load-bearing — do not soften).** N-F-E is a **hermetic, single-epoch,
> self-accept-only** forge-tick wiring cluster. There is NO operator-key ingestion in `--mode node`
> (a separate RED key-ingress cluster); NO serve/broadcast/gossip of a forged block; NO durable apply /
> tip mutation by the forge (the single durable tip-advance authority stays `run_node_sync → pump_block`);
> NO live peer / BA-02 / RO-LIVE claim. The clock seam keeps `SystemTime` / `Instant` / float out of
> GREEN/BLUE. The forge handoff is reached by the `--mode node` loop only when a `ForgeActivation` is
> supplied (hermetic/test-only this cluster); the binary path passes `None` and reproduces the exact
> N-F-D relay behavior.
>
> ### PHASE4-N-F-E (closing, `cd2484f`) — forge-tick on the relay run-loop spine
>
> N-F-E introduced / extended (all CLOSED — classified under §3 Closed / §4 Frozen):
>
> - **CLOSED, additively extended (GREEN, `ade_node::run_loop_planner`):** the pure `LoopStep`
>   vocabulary went **3 → 4 variants** (`+ ForgeTick`), and a NEW content-blind `ForgeSlotStatus { Due,
>   NotDue }` planner input was added. `plan_loop_step` is now
>   `plan_loop_step(loop_state, sync_status, forge_slot_status, shutdown_status)` with a total decision
>   table; a NEW pure `forge_slot_status(last_forged_slot, current_slot)` monotonic guard (the ONLY fn in
>   the module that observes a `SlotNo`) emits `Due` at most once per `SlotNo` and never for a past slot.
>   With `ForgeSlotStatus::NotDue` the table collapses to the exact N-F-D relay mapping (never returns
>   `ForgeTick`). Both new closed vocabularies are pinned by `ci_check_loop_planner_closed.sh`, which now
>   scopes the `SlotNo` ban to `plan_loop_step` (so the pure `forge_slot_status` guard may legitimately
>   consume a `SlotNo`). These are **CE-not-law additively-evolvable closed planner vocabularies** (like
>   the existing `WalEntry` / chaindb `SCHEMA_VERSION` governance posture) — **NOT new plugin/extension
>   seams.** DC-NODE-05.
> - **CLOSED (RED, `ade_node::node_lifecycle`):** NEW `pub struct ForgeActivation<'a>` — the **opt-in
>   forge-activation bundle** threaded into `run_relay_loop` (now
>   `forge: Option<&mut ForgeActivation<'_>>`). When `Some`, the loop's `ForgeTick` branch derives the
>   slot via the **clock seam** (`Clock::next_tick → millis_to_slot → SlotNo`; only `SlotNo` crosses the
>   seam — no `SystemTime`/`Instant`/float past the RED observation boundary), reuses
>   `CoordinatorState::kes_period_for_slot`, and calls **exactly ONE** fenced
>   `node_sync::forge_one_from_recovered`. It advances NO durable tip and serves/admits/gossips nothing —
>   a forged block is a local self-accept artifact recorded only into the in-memory
>   `hermetic_forge_outcomes: Vec<CoordinatorEvent>` test-observation field. When `None`, it is byte-
>   identical to N-F-D relay (the binary path passes `None`). **This is a closed, opt-in activation
>   surface, not an extension point.** CN-NODE-02 / DC-SYNC-02 / DC-NODE-05.
> - **Gate evolution (the seam-closure mechanism, NO new gate):** `ci_check_node_run_loop_containment.sh`
>   now permits **exactly one fenced `forge_one_from_recovered`** while RETAINING every
>   tip/serve/admit/`run_real_forge`/`correlate(`/`Ba02Manifest`/second-bootstrap prohibition and ADDING
>   no-serve tokens (`served_chain_admit` / `push_atomic` / `OutboundCommand` / `broadcast` /
>   `block_fetch`). A **net tightening.** `ci_check_loop_planner_closed.sh` scoped the `SlotNo` ban +
>   pinned `ForgeTick` / `ForgeSlotStatus` in the closed set.
> - **Consume-side fence still holds (CN-CINPUT-03 / DC-CINPUT-02b guard (d)):** the forge path projects
>   leadership ONLY via the recovered `PoolDistrView::from_seed_epoch_consensus_inputs` inside the fenced
>   `forge_one_from_recovered`. No fabricated `SeedEpochConsensusInputs` / pparams / pool-id / KES literal;
>   no forge-time bundle token. `node_sync` had **no production change** (`#[cfg(test)]` additions only) —
>   `forge_one_from_recovered` and `run_node_sync` are byte-for-byte the N-F-D production surface.
>
> **Registry delta (N-F-E):** **310 rules** at HEAD. `DC-NODE-05` flipped `declared → enforced` (its
> `tests` + `ci_script` arrays populated). `strengthened_in += "PHASE4-N-F-E"` on six rules:
> `CN-NODE-02`, `DC-SYNC-02`, `T-REC-03`, `DC-NODE-03` (the clock seam), `CN-PROD-02`, `DC-CINPUT-02b`.
> **No `id =` was added/removed** by N-F-E — `DC-NODE-05` was already present as a `declared` N-F-E
> sketch row (the count was already 310). **No new CI gate** — both N-F-D gates were extended in place.
>
> **Governance note (N-F-E).** The load-bearing structural lines hold. The forge stays **subordinate**:
> the GREEN planner learns only whether a slot is *due* (content-blind `ForgeSlotStatus`), never who is a
> leader — leadership eligibility is decided BLUE inside `forge_one_from_recovered`. The single durable
> tip-advance path remains `run_node_sync → pump_block`; the forge advances no tip. A forged block is
> **self-accept-only** — the loop serves/admits/broadcasts/gossips nothing (CE-E-4). Forge is **opt-in via
> `ForgeActivation`** — the binary path passes `None` and is byte-identical to N-F-D relay.
>
> ### PHASE4-N-F-D (closed, `7de1462`) — the live relay run-loop spine
>
> N-F-D introduced (all CLOSED):
>
> - **NEW (GREEN, `ade_node::run_loop_planner`)** — the pure lifecycle decision function for the
>   `--mode node` relay run loop. At N-F-D close `plan_loop_step` selected each iteration's step from the
>   closed **3-variant** `LoopStep { SyncOnce, Idle, HaltCleanly }` vocabulary over a content-blind
>   `SyncStatus` (`WorkReady` / `NoWorkReady` / `Ending`) + shutdown signal. The planner **cannot express
>   an authority decision** — no apply / admit / forge / evidence / verdict variant. (N-F-E extended this
>   to 4 variants + `ForgeSlotStatus`, above.) CN-NODE-02.
> - **NEW / EXTENDED (RED, `ade_node::node_lifecycle::run_relay_loop`)** — the single live-run owner. Both
>   `--mode node` arms (FirstRun Mithril bootstrap; WarmStart WAL-replay recovery) now **converge into
>   `run_relay_loop`** (no print-and-exit). Each iteration is driven by the GREEN `plan_loop_step`; the
>   tip advances ONLY through `run_node_sync → pump_block` (durable-before-advance). The loop body reaches
>   NO direct `pump_block` / manual tip advance / forge / evidence / verdict / follower / second-bootstrap
>   path. `NodeLifecycleError::RelaySync` is the fail-closed halt. CN-NODE-02 / DC-SYNC-02.
> - **EXTENDED (RED, `ade_node::node_sync`)** — `NodeBlockSource` gained a content-blind, non-consuming
>   readiness signal (`has_work_ready` / `is_ended` / `wait_ready`, filled via non-blocking `try_recv`);
>   `next_block` is non-blocking (drain-available-then-`None`) so `run_node_sync` remains the SOLE block-
>   consumption path and is never awaited across a shutdown-cancellation boundary. `run_node_sync` is
>   otherwise **UNMODIFIED** (still the first/only production `pump_block` caller; DC-SYNC-01 /
>   `ci_check_node_sync_via_pump.sh` stays green). DC-SYNC-02.
> - **NEW CI gates (the +2 the prior CODEMAP/SEAMS predated):** `ci_check_loop_planner_closed.sh` (the
>   GREEN planner's closed `LoopStep` vocabulary + content-blind step selection — at N-F-D the whole
>   module banned `SlotNo`) and `ci_check_node_run_loop_containment.sh` (the RED relay-loop body fence:
>   advances the tip only via `run_node_sync`; no second bootstrap/apply/forge/evidence/manual-tip path).
> - **Loop-as-replay (T-REC-03, NEW):** the same recovered/bootstrapped state + the same ordered canonical
>   block feed + the same deterministic loop inputs + the same shutdown schedule produce byte-identical
>   authoritative outputs (tips, WAL/checkpoints, halt state). Extends T-REC-01/02 from single-shot
>   recovery to continuous relay operation; rides existing recovery laws — no new durability law.
>
> **Honest scope (N-F-D, carried into N-F-E):** relay-only + strictly hermetic; a **live unbounded peer**
> remains the **RO-LIVE-01 follow-on** (not closed at this HEAD).
>
> ---
>
> **The PHASE4-N-F-C surface stands below, carried forward.** PHASE4-N-F-C wires the single `--mode node`
> Ade node lifecycle and is proven through evidence closure. Its SEAMS deltas were almost entirely surface
> REDUCTIONS — new CLOSED surfaces, not new extension points — and it CLOSED the consume-side seed-epoch
> consensus-input seam that N-F-A had left open.
>
> N-F-C introduced / extended (all NEW surfaces are CLOSED — classified under §3 Closed / §4 Frozen):
>
> - **CLOSED (RED, `ade_node::cli`)** the `Mode` run-mode enum is a **5-variant CLOSED set**
>   `{WireOnly, Admission, KeyGenKes, Produce, Node}` — **no `#[non_exhaustive]`**, and `main.rs`
>   dispatch has **no wildcard arm**. The `Node` variant is the N-F-C addition. Gate:
>   `ci_check_node_mode_closure.sh`. A surface REDUCTION (a closed mode taxonomy), not an extension point.
> - **CLOSED (RED, `ade_node::node_sync`)** `NodeBlockSource` — a **verdict-decoupled** peer-block source:
>   closed 2-variant enum (`WirePump` / `InMemory`) whose `next_block` yields **only** ordered block bytes
>   (`AdmissionPeerEvent::Block`), skips `TipUpdate`, ends on `Disconnected` / closed channel. It NEVER
>   derives / surfaces / depends on a verdict.
> - **CLOSED (GREEN, `ade_node::ba02_evidence`)** the BA-02 peer-acceptance evidence vocabulary — closed
>   sums `PeerAcceptEvent` (2-variant allow-list), `PeerAcceptSource` (3), `NoEvidenceReason` (4),
>   `BA02Outcome` (2) — plus the **versioned** `Ba02Manifest` (`BA02_MANIFEST_SCHEMA_VERSION = 1`).
>   `correlate` is the **SOLE** `Ba02Manifest` constructor. GREEN evidence comparing already-authoritative
>   outputs. Gate: `ci_check_ba02_evidence_closed.sh`. **BA-02 is satisfied NOWHERE at this HEAD.**
> - **CLOSURE of the N-F-A consume-side seam (CN-CINPUT-03 / DC-CINPUT-02b).** The node-lifecycle forge
>   path `node_sync::forge_one_from_recovered` may attach ONLY via the recovered surface — it projects
>   leadership via `PoolDistrView::from_seed_epoch_consensus_inputs` and fails closed when none. Fenced by
>   `ci_check_consensus_input_provenance.sh` guard (d). _(N-F-E carries this fence unchanged.)_
> - **READ-ONLY discovery (RED)** `SnapshotStore::list_seed_epoch_consensus_anchor_fps()` — the WarmStart
>   anchor-lineage discovery. Discovery is NOT proof; the warm-start verify chain is the authority.
> - **Lifecycle-owner rule (RED, `ade_node::node_lifecycle`)** — THE single `--mode node` recovered-state
>   lifecycle owner (`PHASE4-N-F-C-LIFECYCLE-OWNER`). Both arms route initial state through the single
>   `bootstrap_initial_state` authority. Gates: `ci_check_lifecycle_owner_uses_bootstrap_initial_state.sh`
>   (CN-NODE-01) + `ci_check_node_sync_via_pump.sh` (DC-SYNC-01 driver containment).
>
> **What the binary `Node` arm actually runs (precise wiring honesty, updated through N-F-E).** `main()`
> routes `Mode::Node → run_node_lifecycle → run_node_lifecycle_inner`. The arm is **fully wired + durable
> for bootstrap + recovery** (FirstRun Mithril bootstrap; WarmStart WAL-replay recovery). Both arms then
> **converge into `run_relay_loop`** (PHASE4-N-F-D S2) — the single live-run owner — and drive each
> iteration over the closed `LoopStep` vocabulary; the tip advances ONLY through `run_node_sync →
> pump_block`. **N-F-E forge path:** `run_relay_loop` now takes `forge: Option<&mut ForgeActivation>`.
> With `None` (the binary path) the planner is fed `ForgeSlotStatus::NotDue`, never returns
> `LoopStep::ForgeTick`, and the loop is byte-identical to N-F-D relay. With `Some(activation)`
> (hermetic/test-only at this HEAD) the `ForgeTick` arm derives the slot via the clock seam, reuses
> `kes_period_for_slot`, calls exactly one fenced `forge_one_from_recovered`, advances NO durable tip, and
> records the outcome into the in-memory `hermetic_forge_outcomes` field. The L6 BA-02 evidence correlator
> (`ba02_evidence`) remains a library surface reached by no binary arm (tested only). **BA-02 is satisfied
> nowhere — no Haskell peer accepts an Ade-forged block at this HEAD.**
>
> **Governance note (N-F-C).** The single `bootstrap_initial_state` authority is the sole initial-state
> path on both lifecycle arms (CN-NODE-01); the recovered surface is consumed for leadership ONLY via the
> closed `PoolDistrView::from_seed_epoch_consensus_inputs` projection. The BA-02 manifest is a CLAIM ABOUT
> authority, not authority. The `Mode` sum stays closed with no wildcard dispatch. **No BLUE crate was
> modified by N-F-C / N-F-D / N-F-E** (the 456 canonical-type total is unchanged; all this code lands in
> the RED `ade_node` + RED `ade_runtime`).
>
> ---
>
> **The PHASE4-N-F-A surface stands below, carried forward.** PHASE4-N-F-A is the **recovered seed-epoch
> consensus-input CAPABILITY** cluster (A1–A4) — a closed canonical record with a SOLE codec, persisted as
> a fingerprint-bound sidecar and reconstructable through verified warm-start. It was a capability cluster,
> NOT production wiring; PHASE4-N-F-C wired production consumption.
>
> N-F-A introduced / extended:
>
> - **BLUE** `ade_ledger::seed_consensus_inputs` (NEW, A1) — the closed `SeedEpochConsensusInputs`
>   recovered-state record + its **SOLE** version-gated, byte-canonical codec
>   (`encode_/decode_seed_epoch_consensus_inputs`, `SEED_CINPUT_SCHEMA_VERSION = 1`) + the closed 6-variant
>   `SeedConsensusInputsError`. **CN-CINPUT-01**.
> - **BLUE** `ade_ledger::wal` (EXTENDED, A3a) — the **additive** `WalEntry::SeedEpochConsensusInputsImported`
>   variant at append-only **wire tag 3** that does not participate in the `AdmitBlock` fp-chain;
>   `replay_from_anchor` folds the WAL into a `ReplayOutcome` carrying an
>   `Option<RecoveredBootstrapProvenance>`. **DC-CINPUT-01**. `WalEntry` stays a CE-not-law surface.
> - **BLUE** `ade_ledger::consensus_view` (EXTENDED, A4) — `PoolDistrView::from_seed_epoch_consensus_inputs`
>   (pure field-map). **DC-CINPUT-02a** (enforced; CONSUMED by the node-lifecycle forge path since N-F-C
>   under DC-CINPUT-02b — and exercised by the N-F-E forge tick).
> - **GREEN** `ade_runtime::seed_consensus_merge` (NEW, A2) — deterministic no-I/O merge; fail-closed,
>   never a zero-hash fill.
> - **RED** `ade_runtime::seed_consensus_provenance` (NEW, A3a) — the single shared
>   `append_seed_epoch_provenance` helper.
> - **RED** `ade_runtime::bootstrap` (EXTENDED, A3b) — the closed 2-variant `SeedEpochConsensusSource`
>   enum (`NotRequired` / `RequiredFromRecoveredProvenance`), the named `BootstrapState` struct,
>   `restore_seed_epoch_consensus_inputs` warm-start verification, and 5 new fail-closed `BootstrapError`
>   variants.
> - **RED** `ade_runtime::{genesis_bootstrap, mithril_bootstrap}` (EXTENDED, A2/A3a) — both verified-
>   bootstrap composers gained the sidecar tail: GREEN merge → A1 encode → sidecar put (durable) THEN WAL
>   provenance append (the commit point).
> - **RED** `ade_runtime::chaindb` (EXTENDED, A2) — the `SnapshotStore` trait gained the **anchor-fp-keyed**
>   sidecar surface, disjoint from the slot-keyed snapshot namespace; redb backend bumped
>   `SCHEMA_VERSION` v2 → **v3**. A versioned gate, NOT a frozen contract.
> - **NEW CI gate** `ci_check_consensus_input_provenance.sh` (**CN-CINPUT-02**, enforced) — a data-flow-
>   resistant containment gate; the populate-side fence (N-F-C added guard (d), the consume-side fence).
>
> **Four structural decisions remain load-bearing for SEAMS:** (1) the **single `bootstrap_initial_state`
> authority** fronts produce-mode cold-start, the Conway-genesis path, the Mithril provenance path (N-Y),
> the Mithril production-bootstrap composition (N-Z), the N-F-A warm-start restore, AND both N-F-C
> lifecycle arms (FirstRun + WarmStart) — **no `GenesisAnchor` / `MithrilAnchor` trait or plugin seam
> exists**. (2) The **two-driver split** (GREEN reducer / RED pump). (3) **`WalEntry` stays a CE-not-law**
> surface. (4) The **redb `chaindb` `SCHEMA_VERSION` is a versioned gate** (v3), not a frozen contract.
>
> **Cluster-doc location.** The PHASE4-N-F-E cluster doc + slice docs (S1, S2, S3a, S3b) live under
> `docs/clusters/PHASE4-N-F-E/`; on close they archive under `docs/clusters/completed/`. Every prior
> closed cluster doc — the **PHASE4-N-F-A / N-F-C / N-F-D** sets, the entire **N-Q / N-R-\* / N-S-\*** set,
> the **N-M-\*** sub-trees, **N-O**, **N-P**, **N-T**, **N-V**, **N-W**, **N-X**, **N-Y**, **N-Z** — is
> archived under `docs/clusters/completed/`.

---

## 1. Surface Reduction Rules

> External inputs reduce to canonical form before entering authoritative pipelines. Ade
> is a Cardano node, not a request/response service — its "external surfaces" are the
> N2N/N2C wire, operator-supplied key/genesis/opcert files, the cardano-cli UTxO seed
> dump, the Mithril snapshot manifest (N-Y), the Mithril production-bootstrap composition
> (N-Z), the Conway genesis file (N-Y), and argv. Each reduces to a canonical BLUE type
> before any authoritative transition. There is **no HTTP/gRPC/message-bus ingress**
> (confirmed absent — not a gap).
>
> **N-F-D/N-F-E note:** the relay run-loop and the forge tick add **no external ingress surface**. The
> wall-clock that drives the forge tick enters through the existing RED **clock seam** (`Clock::next_tick`
> → `millis_to_slot` → `SlotNo`); only the canonical `SlotNo` crosses into the GREEN planner / the fenced
> forge call (DC-NODE-03 / DC-NODE-05). The recovered seed-epoch consensus-input sidecar remains an
> Ade-internal recovered surface, populated only on the verified-bootstrap path and read back only by the
> warm-start restore inside `bootstrap_initial_state`.

### Surface: N2N inbound wire (received blocks/headers/txs)

```
Surface: N2N mini-protocol traffic over TCP+mux (RED ade_runtime::network::{n2n_listener, mux_pump, n2n_dialer})
Reduces to: decoded mini-protocol messages → tag-24-stripped inner bytes → PreservedCbor<T> → DecodedBlock (BLUE ade_codec)
Pipeline (fixed; steps may not be reordered or shortcut):
  1. mux::frame::decode_frame                       (BLUE — single frame-decode authority)
  2. session::core::step                            (GREEN — partial-frame buffer + payload reassembly + closed AcceptedMiniProtocol registry)
  3. per-mini-protocol *_transition reducer         (BLUE — chain_sync / block_fetch / etc.)
  3a. tag-24 strip (N-X)                             (BLUE — decompose_blockfetch_block / decompose_rollforward_header delegate to ade_codec::unwrap_tag24; RED admission::runner / follow call ade_codec::unwrap_tag24 directly — no hand-rolled parse)
  4. ade_codec decode_block_envelope / decode_*     (BLUE — sole PreservedCbor construction site, over the verbatim tag-24-stripped inner bytes)
  5. ade_ledger::receive::reducer / mempool_ingress (BLUE — header→body bridge / wire-ingress chokepoint)
  6. forward_sync::reducer → forward_sync::pump (N-Y)  (GREEN admit-plan over the BLUE admit chokepoint → RED durability-ordered driver; AdvanceTip only after StoreBlockBytes + AppendWal ack)
  7. block_validity / tx_validity / admission        (BLUE verdict; GREEN admission compares already-authoritative outputs)
Cross-surface state sharing: the served ServedChainSnapshot (read by both serve and broadcast paths);
  the per-peer outbound map (PerPeerOutbound) is keyed by PeerId — no cross-peer byte leakage.
  The tag-24 unwrap step (N-X) is the SAME shared ade_codec authority used by the serve path's wrap step.
  The forward-sync persisted ChainDb + FileWalStore are the same stores the recovery path (recovery::restart)
  reconciles on warm-start (DC-WAL-*; WalTailFingerprintMismatch fail-fast). N-F-C: the SAME stores the
  --mode node lifecycle (node_lifecycle + node_sync) opens; pump_block gains its FIRST production caller
  in node_sync::run_node_sync. N-F-D: the relay loop (run_relay_loop) is the live-run owner that drives
  run_node_sync each iteration. The WAL is also where the N-F-A additive SeedEpochConsensusInputsImported
  (tag 3) provenance lives — disjoint from the AdmitBlock fp-chain.
```

### Surface: --mode node relay run-loop (the live-run owner, N-F-D; forge-tick N-F-E)

```
Surface: --mode node relay run loop (RED ade_node::node_lifecycle::run_relay_loop; the single live-run owner)
Reduces to: per-iteration step over the closed GREEN LoopStep vocabulary; the tip advances ONLY via run_node_sync → pump_block
Pipeline (fixed; GREEN plans the iteration, RED performs effects, BLUE authority stays behind the seams):
  0. (entry) both --mode node arms converge here     (FirstRun bootstrap + WarmStart recovery → run_relay_loop; no print-and-exit)
  1. (top of iteration) reset pending_slot = None     (N-F-E: a skipped/failed path can never forge for a stale slot)
  1a. (forge-on path only) clock seam → SlotNo         (N-F-E: RED Clock::next_tick → millis_to_slot → SlotNo; only SlotNo crosses; DC-NODE-03/DC-NODE-05)
  1b. (forge-on path only) forge_slot_status guard      (N-F-E: GREEN pure monotonic guard → ForgeSlotStatus {Due|NotDue}; at most once per SlotNo, never a past slot)
  2. GREEN plan_loop_step(loop_state, sync_status, forge_slot_status, shutdown)  (→ closed LoopStep {SyncOnce | ForgeTick | Idle | HaltCleanly}; content-blind; total table)
  3a. SyncOnce  → run_node_sync → pump_block            (DC-SYNC-01/DC-SYNC-02: the SOLE durable tip-advance; durable-before-advance; UNMODIFIED since N-F-C)
  3b. ForgeTick → exactly one forge_one_from_recovered  (N-F-E: reuses kes_period_for_slot; recovered-surface leadership; advances NO durable tip; serves/admits/gossips NOTHING; updates last_forged_slot only on a real attempt; records into in-memory hermetic_forge_outcomes)
  3c. Idle      → cancellation-safe wait                (select on source-readiness or shutdown; the only branch that awaits across a cancellation boundary)
  3d. HaltCleanly → exit, on-disk state recoverable
Cross-surface state sharing: shares the persistent ChainDb + FileWalStore with the forward-sync pump +
  warm-start restore; ForgeActivation borrows the recovered BootstrapState (the SOLE leadership source),
  the CoordinatorState (genesis-anchor host for the REUSED kes_period_for_slot), the ProducerShell
  (hermetic/fenced key custody), and the Clock seam. The forge tick shares NO served-chain / outbound map.
Rule (CN-NODE-02 / DC-SYNC-02 / DC-NODE-05, ci_check_node_run_loop_containment.sh + ci_check_loop_planner_closed.sh):
  the relay-loop body advances the tip ONLY via run_node_sync; references NO run_real_forge / correlate( /
  Ba02Manifest / second-bootstrap path, NO direct manual tip-mutation token, and (N-F-E) NO serve token
  (served_chain_admit / push_atomic / OutboundCommand / broadcast / block_fetch); it may have EXACTLY ONE
  fenced forge_one_from_recovered( call (CE-E-4). The GREEN planner emits only the closed LoopStep
  vocabulary and cannot express an authority decision; SlotNo is observable ONLY in the pure
  forge_slot_status guard (banned in plan_loop_step).
HONEST SCOPE: relay-only + strictly hermetic; the forge tick is reached only when a ForgeActivation is
  supplied (hermetic/test-only this cluster); the binary path passes None (byte-identical to N-F-D relay).
  A live unbounded peer is the RO-LIVE-01 follow-on. No BA-02 / serve / gossip / durable-forge claim.
```

### Surface: producer-mode forge → serve → broadcast (the live producer half)

```
Surface: --mode produce slot loop (RED ade_node::produce_mode + GREEN producer::coordinator)
Reduces to: ForgedBlock → AcceptedBlock (BLUE self_accept) → ServedChainSnapshot → tag-24-wrapped wire bytes
Pipeline (fixed; the BLUE-then-RED-then-BLUE composition of run_real_forge):
  1. bootstrap_initial_state                        (RED/GREEN — sole forge-state source; N-T; fronts genesis/Mithril cold-start, N-Y/N-Z; produce_mode passes SeedEpochConsensusSource::NotRequired — N-F-A)
  1a. era guard (N-W)                                (RED — non-Praos era fail-closes to ForgeFailureReason::UnsupportedProducerEra before any forge)
  2. RED vrf_prove over expected_vrf_input.alpha_bytes()  (operator VRF key; alpha comes from the BLUE LeaderScheduleAnswer — no RED-side era dispatch; N-W)
  3. BLUE verify_and_evaluate_leader(era, …) → LeaderCheckVerdict  (ade_core::consensus::leader_check; era-correct Praos construction; N-R-A + N-W)
  4. RED kes_sign_header(UnsignedHeaderPreImage)    (signs ONLY the branded pre-image; N-S-A)
  5. GREEN assemble_tick
  6. BLUE forge_block → encode_block_envelope       (single canonical block encoder, storage-form [era, block]; N-V)
  7. BLUE self_accept                               (gate — no ForgeSucceeded without Accepted)
  8. ChainEvolution::advance(self)                  (GREEN linear typestate; token only via self_accept; N-T)
  9. ServedChainHandle::push_atomic                 (single served-admit authority; N-R-B/N-T)
 10. BLUE serve composition (N-X)                   (block_fetch::server emits compose_blockfetch_block(storage [era, block]) = tag24(bytes([era, block]));
                                                     chain_sync::server emits compose_rollforward_header(era, header_cbor) = [era_tag, tag24(bytes(header_cbor))])
 11. OutboundCommand → MuxPump                      (typed relay; no byte tunnel; N-S-B)
Cross-surface state sharing: ChainEvolution threads each forge's post-state into the next
  forge's base; ServedChainSnapshot is shared with the N2N serve path; the per-peer outbound
  map is shared with the listener. The serve step's tag-24 wrap is the SAME ade_codec authority
  the receive path uses to unwrap (CN-WIRE-08).
N-F-A/N-F-C/N-F-E fence (populate-side AND consume-side enforced): produce_mode is the forge-time
  consensus-input path (import_live_consensus_inputs + pool_distr_view_from_consensus_inputs +
  --consensus-inputs-path). It MUST pass SeedEpochConsensusSource::NotRequired and MUST NOT build / put
  the seed-epoch sidecar nor append its WAL provenance (CN-CINPUT-02 populate-side). produce_mode stays
  DIAGNOSTIC and does NOT consume the recovered surface. The recovered-surface CONSUME-side seam is
  CLOSED on the SEPARATE node-lifecycle forge path: node_sync::forge_one_from_recovered projects the
  leadership view ONLY via PoolDistrView::from_seed_epoch_consensus_inputs(&recovered.…), and may NOT
  fabricate a SeedEpochConsensusInputs literal or name the bundle tokens
  (ci_check_consensus_input_provenance.sh guard (d); CN-CINPUT-03 / DC-CINPUT-02b). N-F-E exercises that
  fenced forge path from the relay loop's ForgeTick branch (hermetic/test-only; binary passes None).
```

### Surface: seed-epoch sidecar warm-start (recovered consensus inputs — N-F-A; WIRED in N-F-C)

```
Surface: warm-start restore of the recovered seed-epoch consensus inputs (RED ade_runtime::bootstrap::restore_seed_epoch_consensus_inputs, inside the bootstrap_initial_state authority)
Reduces to: anchor-fp-keyed sidecar bytes (SnapshotStore::get_seed_epoch_consensus_inputs) → verified SeedEpochConsensusInputs → BootstrapState.seed_epoch_consensus_inputs: Option<SeedEpochConsensusInputs>
Pipeline (fixed; the RED-read / BLUE-verify chain; fail-closed on every step):
  1. SeedEpochConsensusSource discriminant           (RequiredFromRecoveredProvenance(provenance) ⇒ restore; NotRequired ⇒ None — the only two modes; the node-lifecycle WarmStart arm passes RequiredFromRecoveredProvenance; produce_mode + every other caller passes NotRequired)
  2. RED get_seed_epoch_consensus_inputs(anchor_fp)  (the only RED step — reads the anchor-fp-keyed sidecar bytes; absent ⇒ BootstrapError::SeedConsensusSidecarMissing)
  3. BLUE blake2b_256 bind                            (re-hash the read bytes; != provenance.sidecar_hash ⇒ SeedConsensusHashMismatch)
  4. BLUE decode_seed_epoch_consensus_inputs          (the A1 SOLE decoder; version-gated, byte-canonical; failure ⇒ SeedConsensusSidecarDecode)
  5. BLUE anchor/epoch binding                        (decoded anchor_fp/epoch_no != provenance ⇒ SeedConsensusBindingMismatch)
  6. BLUE byte-identity re-encode                     (re-encode != input ⇒ SeedConsensusHashMismatch)
Cross-surface state sharing: the same SnapshotStore + WAL the forward-sync pump, recovery::restart, the
  N-F-C node lifecycle, and the N-F-D relay loop use. The recovered BootstrapState this produces is the
  SOLE leadership source the N-F-E forge tick projects (PoolDistrView::from_seed_epoch_consensus_inputs).
N-F-C WIRING: the production restart path is wired — ade_node::node_lifecycle (--mode node) drives this
  restore on its WarmStart arm via list_seed_epoch_consensus_anchor_fps discovery → WAL replay →
  bootstrap_initial_state(RequiredFromRecoveredProvenance). The CONSUME-side fence
  (ci_check_consensus_input_provenance.sh guard (d): CN-CINPUT-03 / DC-CINPUT-02b) keeps the forge path
  from fabricating the record.
```

### Surface: --mode node lifecycle (FirstRun / WarmStart — the real Ade node, N-F-C)

```
Surface: --mode node (RED ade_node::node_lifecycle; THE single PHASE4-N-F-C-LIFECYCLE-OWNER)
Reduces to: on-disk state → closed NodeStart {FirstRun | WarmStart} → verified BootstrapState through the single bootstrap_initial_state authority → run_relay_loop (N-F-D)
Pipeline (fixed; classification is a PURE function of on-disk state):
  1. open persistent ChainDb (PersistentChainDb) + FileWalStore
  2. classify_start(has_tip, has_snapshots)            (pure → NodeStart::FirstRun | NodeStart::WarmStart)
  3a. FirstRun  → first_run_mithril_bootstrap          (Mithril-only; bootstrap_from_mithril_snapshot; verify_mithril_binding fail-closed BEFORE any state admitted; persists seed-epoch sidecar + WAL provenance; NO genesis/bundle/cold/graft fallback)
  3b. WarmStart → warm_start_recovery                  (list_seed_epoch_consensus_anchor_fps discovery → WAL replay → bootstrap_initial_state(RequiredFromRecoveredProvenance) verify chain)
  4. (N-F-D) both arms converge into run_relay_loop    (the single live-run owner; the tip advances ONLY via run_node_sync → pump_block; N-F-E: forge: None on the binary path)
Cross-surface state sharing: shares the single bootstrap_initial_state authority with produce_mode +
  the Conway-genesis + Mithril production-bootstrap paths; shares the persistent ChainDb + FileWalStore
  with the forward-sync pump + warm-start restore + the relay loop. Closed fail-closed NodeLifecycleError
  (incl. RelaySync, N-F-D).
Rule (CN-NODE-01, ci_check_lifecycle_owner_uses_bootstrap_initial_state.sh): exactly one module carries
  the PHASE4-N-F-C-LIFECYCLE-OWNER marker; both arms route through the SINGLE bootstrap_initial_state
  authority — no second bootstrap/recovery/storage-init path, no genesis/bundle/cold/graft fallback, no
  recover_node_state overclaim. A new mode that needs initial state MUST obtain it via this one authority.
```

### Surface: --mode node sync source (verdict-decoupled peer-block source, N-F-C; readiness extended N-F-D)

```
Surface: NodeBlockSource (RED ade_node::node_sync; closed 2-variant enum {WirePump | InMemory})
Reduces to: ordered peer-block BYTES only (AdmissionPeerEvent::Block → Vec<u8>); NEVER a verdict
Pipeline (the verdict-decoupled source contract, E1/E2; N-F-D readiness signal):
  1. next_block() selects ONLY AdmissionPeerEvent::Block, in arrival order
  1a. AdmissionPeerEvent::TipUpdate is SKIPPED            (a comparison input for admission's verdict loop, not a block, not a sync tip authority)
  1b. AdmissionPeerEvent::Disconnected / closed channel ENDS the feed (a clean disconnect is not a tip authority)
  1c. (N-F-D) content-blind readiness                     (has_work_ready / is_ended / wait_ready via non-blocking try_recv; next_block is non-blocking drain-then-None; never awaited across a shutdown cancellation boundary)
  2. (driver) run_node_sync feeds bytes to forward_sync::pump_block — its production caller — durable StoreBlockBytes + AppendWal BEFORE AdvanceTip (DC-SYNC-01), then a tip checkpoint via PersistentSnapshotCache
Cross-surface state sharing: the same persistent ChainDb + FileWalStore the lifecycle owner + warm-start
  restore + the relay loop use; the captured tip checkpoint is the exact PersistentSnapshotCache artifact warm_start_recovery reads back.
Rule (DC-SYNC-01 / DC-SYNC-02, ci_check_node_sync_via_pump.sh): the source yields ordered block bytes and
  NOTHING else (no derive_verdict / run_admission / follow); run_node_sync advances the tip ONLY via
  pump_block (no follower-as-sync, no verdict-as-sync, no manual put_block/AdvanceTip/rollback_to_slot).
  run_node_sync is UNMODIFIED in production by N-F-D / N-F-E (#[cfg(test)] additions only).
HONEST SCOPE: run_node_sync is driven on the live run path ONLY by the N-F-D relay loop (SyncOnce);
  forge_one_from_recovered is reached ONLY via the N-F-E ForgeTick branch when a ForgeActivation is
  supplied (hermetic/test-only). No live unbounded peer this HEAD (RO-LIVE-01 follow-on).
```

### Surface: BA-02 peer-acceptance evidence (GREEN correlator, N-F-C — tested-but-unwired)

```
Surface: operator-captured peer-accept JSONL log (GREEN ade_node::ba02_evidence::parse_peer_accept_events)
Reduces to: closed PeerAcceptEvent set → BA02Outcome {Ba02Manifest | NoEvidence} via correlate
Pipeline (allow-list parse → exact-match correlate; the SOLE Ba02Manifest constructor):
  1. parse_peer_accept_events                            (ALLOW-LIST: only `peer_served_block` / `peer_chain_tip` discriminators → PeerAcceptEvent; every weaker/unknown/malformed line DROPPED, never coerced)
  2. AdeForgeRecord::from_forge_artifact                 (reads the BLUE-minted forged hash + slot VERBATIM from ForgedBlockArtifact — NEVER recomputed; no new BLUE authority)
  3. correlate(ade, peer_log)                            (pure/total/deterministic, HASH-PRIMARY; emits BA02Outcome::Ba02Manifest ONLY on an exact forged-hash↔peer-accept match at the matching chain point, no conflicting signal; else NoEvidence{reason})
Cross-surface state sharing: NONE. GREEN evidence comparing already-authoritative outputs; forges nothing,
  admits nothing, persists no node state. A Ba02Manifest is a CLAIM ABOUT authority, not authority.
Rule (RO-LIVE-06, ci_check_ba02_evidence_closed.sh): correlate is the ONLY Ba02Manifest constructor;
  NO self-evidence token (ForgeSucceeded / self_accept / block_received / agreement_verdict / "agreed")
  may be an acceptance source; NO committed synthetic docs/evidence/*ba02* manifest. The versioned
  Ba02Manifest (BA02_MANIFEST_SCHEMA_VERSION = 1) is a version-GATED contract (§4).
HONEST SCOPE: tested-but-unwired library surface — no binary arm drives it; synthetic logs prove the
  mechanics only and CANNOT satisfy BA-02. BA-02 is satisfied NOWHERE at this HEAD (incl. N-F-E's
  self-accept-only forge tick); RO-LIVE-01 remains partial/operator-gated.
```

### Surface: operator file ingress (KES skey / opcert / Shelley genesis / UTxO seed dump)

```
Surface: operator-supplied files (RED ade_runtime::producer::{keys, opcert_envelope, genesis_parser}, seed_import)
Reduces to: Sum6Kes signing key (via BLUE deserializer) / OperationalCert / GenesisAnchor / canonical seed entries
Pipeline:
  1. RED parse text/JSON/CBOR envelope               (closed parser per file type; structured fail-closed error)
  2. BLUE structural validator                       (e.g. Sum6Kes::raw_deserialize_signing_key_kes — byte layout is the validator)
  3. canonical type handed to the BLUE core          (never raw bytes)
Cross-surface state sharing: GenesisAnchor + opcert public metadata feed the producer coordinator;
  KES/VRF/cold private material is RED-confined and never enters GREEN CoordinatorState.
N-F-E note: --mode node does NOT ingest real operator keys this cluster. The N-F-E forge tick consumes
  only hermetic/fenced producer-shell material via ForgeActivation (test-only); real --mode node KES/VRF/
  cold/opcert/pool-id/pparams ingress is a SEPARATE RED key-ingress cluster (a declared follow-on, §7).
```

### Surface: Mithril snapshot manifest — provenance binding (N-Y)

```
Surface: Mithril snapshot manifest JSON (RED ade_runtime::mithril_import::json::parse_mithril_manifest_json)
Reduces to: RawMithrilManifest → SeedProvenance::Mithril{..} + MithrilManifestReport → (BLUE) verify_mithril_binding verdict
Pipeline (fixed; the RED-then-BLUE provenance binding):
  1. RED parse_mithril_manifest_json                 (SOLE manifest-JSON parser → RawMithrilManifest; fail-closed MithrilManifestError; NO semantic decision)
  2. RED import_mithril_manifest                     (maps into the closed SeedProvenance::Mithril + MithrilManifestReport; NEVER re-verifies the STM multisig)
  3. BLUE verify_mithril_binding(report, anchor)     (the SOLE authority deciding whether a Mithril anchor binds; cross-checks {network_magic, genesis_hash, certified_point, certificate_hash}; fails closed with MithrilImportError)
Cross-surface state sharing: the report side (manifest) and the anchor side MUST originate
  independently — verify_mithril_binding is NOT a tautological self-check (CN-MITHRIL-01).
```

### Surface: Mithril production-bootstrap composition (N-Z, extended N-F-A sidecar tail; first non-test caller N-F-C)

```
Surface: bootstrap_from_mithril_snapshot (RED ade_runtime::mithril_bootstrap; composition-only — NO standalone argv flag; N-F-C first non-test caller = the --mode node FirstRun arm)
Reduces to: (MithrilSeedPointInputs, seed (LedgerState, PraosChainDepState), manifest_bytes) → MithrilBootstrapOutput { ledger, chain_dep, tip, anchor } (+ N-F-A sidecar put + WAL provenance append)
Pipeline (fixed; the call-order is CI-pinned by ci_check_mithril_seed_point_independence.sh):
  1. RED import_mithril_manifest_from_bytes(manifest_bytes)  (→ MithrilProvenanceImport { provenance, report }; fail-closed MithrilBootstrapError::Import; NO semantic decision)
  2. RED mint(MintInputs{ seed_slot/seed_block_hash/network_magic/genesis_hash/… from MithrilSeedPointInputs (operator-INDEPENDENT origin); seed_provenance = import.provenance })  (→ BootstrapAnchor; seed_point comes ONLY from the operator inputs, NEVER the manifest — DC-MITHRIL-02)
  3. BLUE verify_mithril_binding(&import.report, &anchor)    (the SOLE binding authority; fail-closed MithrilBootstrapError::Binding BEFORE any storage init; CN-MITHRIL-01 strengthened — verify-before-bootstrap)
  4. RED bootstrap_initial_state(BootstrapInputs{ …, genesis_initial: Some((seed_ledger, seed_chain_dep)) , seed_epoch_consensus_source: NotRequired })  (the single closed bootstrap authority; never a parallel storage-init path; CN-NODE-01)
  5. RED sidecar tail (N-F-A; success path only, after binding passes)  (GREEN merge_seed_epoch_consensus_inputs → A1 encode → put_seed_epoch_consensus_inputs(anchor_fp, …) DURABLE → append_seed_epoch_provenance (WAL tag 3) COMMIT POINT; the composer WRITES the sidecar, never consumes one; CN-CINPUT-02)
Cross-surface state sharing: shares the single bootstrap authority with produce_mode + the
  Conway-genesis path + the N-F-C node lifecycle. The operator's MithrilSeedPointInputs origin and the
  manifest origin MUST stay structurally independent (DC-MITHRIL-02). The N-F-A sidecar tail puts to the
  anchor-fp-keyed SnapshotStore namespace (disjoint from the slot-keyed snapshot space) THEN appends WAL
  provenance — a crash between leaves an unrecorded sidecar, which warm-start treats as "not imported"
  and fails closed.
```

### Surface: Conway genesis cold-start (N-Y, extended N-F-A sidecar tail)

```
Surface: Conway genesis config (RED ade_runtime::genesis_bootstrap::bootstrap_from_conway_genesis)
Reduces to: ConwayGenesisConfig → (LedgerState, PraosChainDepState) → BootstrapInputs.genesis_initial (+ N-F-A sidecar put + WAL provenance append)
Pipeline (fixed; the RED-read / BLUE-transform / single-authority composition):
  1. RED genesis_parser file read/parse              (shelley/Conway genesis JSON → ConwayGenesisConfig)
  2. BLUE genesis_initial_state(&ConwayGenesisConfig) (pure Conway-only transform; fail-closed GenesisSourceError::NonConwayEra)
  3. RED route through bootstrap_initial_state       (genesis pair enters ONLY via BootstrapInputs.genesis_initial; records SeedProvenance::CardanoCliJson; seed_epoch_consensus_source: NotRequired; never a second storage-init authority)
  4. RED sidecar tail (N-F-A)                         (GREEN merge → A1 encode → put_seed_epoch_consensus_inputs DURABLE → append_seed_epoch_provenance (WAL tag 3) COMMIT POINT; WRITES the sidecar, never consumes one; CN-CINPUT-02)
Cross-surface state sharing: shares the single bootstrap authority with produce_mode + the
  N-Z Mithril production-bootstrap composition. bootstrap_from_mithril_snapshot is the
  symmetric Mithril-path twin of this entry; both gained the same &mut dyn WalStore + sidecar tail.
  NOTE (N-F-C): the --mode node FirstRun arm is Mithril-ONLY (it routes through
  bootstrap_from_mithril_snapshot, NOT this genesis path — no genesis fallback on the node lifecycle).
```

### Surface: argv (closed mode set)

```
Surface: command line (RED ade_node::cli — Cli / ProduceCli / AdmissionCli / KeyGenKesCli)
Reduces to: a 5-variant CLOSED Mode enum {wire_only, admission, key_gen_kes, produce, node}
  (Mode::parse; NOT #[non_exhaustive]; main.rs dispatch has NO wildcard arm; ci_check_node_mode_closure.sh)
Pipeline: argv → Cli → mode driver. --mode produce requires --json-seed + --consensus-inputs-path;
  --mode node (N-F-C) requires --snapshot-dir + --wal-dir, and on FirstRun the documented-extraction
  inputs (--json-seed + --consensus-inputs-path + --mithril-manifest-path, Mithril-bound — NEVER forge inputs).
Cross-surface state sharing: none.
N-F-C: the `node` variant is the addition; main() routes Mode::Node → run_node_lifecycle. Adding a
  Mode variant is a SURFACE REDUCTION (closed taxonomy), not an extension point — a new variant forces a
  main.rs compile error until an explicit (wildcard-free) arm is added (ci_check_node_mode_closure.sh).
N-F-D/N-F-E: NO new argv flag. The relay loop is reached from the existing --mode node arm; the N-F-E
  forge tick is opt-in via ForgeActivation (no argv surface — binary path passes None / NotDue).
```

**Rule:** New ingress attaches by producing the canonical BLUE type's bytes and entering
the **same** pipeline. A new mini-protocol attaches through `session::core::step` + a BLUE
`*_transition` reducer + a closed `AcceptedMiniProtocol` registry entry. A new operator
file type attaches as a RED parser feeding a BLUE structural validator. **A new bootstrap
seed source (like Mithril or genesis) attaches by populating `BootstrapInputs.genesis_initial`
and routing through the single `bootstrap_initial_state` authority — NEVER via a new
`*Anchor` trait / plugin seam, and never via a parallel storage-init path** (CN-MITHRIL-01 /
CN-NODE-01 / DC-GENESIS-SRC-01 / DC-MITHRIL-02). **A new bootstrap-source production composition
(like N-Z `bootstrap_from_mithril_snapshot`) attaches as a composition-only RED twin of
`bootstrap_from_conway_genesis` — verify-before-bootstrap, fail-closed, no new authority, no
new `SeedProvenance` variant, and (if the source attests a point) the anchor `seed_point` MUST
come from an operator-independent origin that `verify_mithril_binding` cross-checks against the
attestation (DC-MITHRIL-02).** **A recovered-state surface (like the N-F-A seed-epoch consensus
inputs) is populated ONLY on the verified-bootstrap composition path (put-then-WAL-append) and is
read back ONLY by the warm-start restore inside `bootstrap_initial_state` — the producer / forge-time
path may not populate it (CN-CINPUT-02 populate-side); the CONSUME-side wiring (the `--mode node`
WarmStart arm passing `SeedEpochConsensusSource::RequiredFromRecoveredProvenance`, and the node-lifecycle
forge path projecting leadership via `PoolDistrView::from_seed_epoch_consensus_inputs`) is CLOSED in
PHASE4-N-F-C, fenced by `ci_check_consensus_input_provenance.sh` guard (d) (CN-CINPUT-03 / DC-CINPUT-02b);
`produce_mode` stays diagnostic.** **A new live-run step attaches to the relay loop as a closed `LoopStep`
variant + a content-blind GREEN planner input + a fenced RED branch (N-F-D / N-F-E pattern) — the planner
emits a closed vocabulary and cannot decide authority; the loop body advances the tip ONLY via
`run_node_sync` and (if it forges) reaches EXACTLY ONE fenced `forge_one_from_recovered`, serving/admitting/
gossiping nothing (CN-NODE-02 / DC-NODE-05 — `ci_check_node_run_loop_containment.sh` +
`ci_check_loop_planner_closed.sh`).** A new `--mode` that needs initial state MUST obtain it via the single
`bootstrap_initial_state` authority (CN-NODE-01).
New ingress **may not** introduce a second `PreservedCbor` construction site, a second
block-envelope encoder, a second era→leader-VRF-input construction (CN-FORGE-04), a second
`wrap_tag24` / `unwrap_tag24` definition or a hand-rolled tag-24 parse in RED (CN-WIRE-08), a
direct-transport write that bypasses `OutboundCommand`, a forward-sync path that advances the tip
before the durability writes ack (DC-SYNC-01), a second bootstrap/storage-init authority
(CN-NODE-01 / DC-GENESIS-SRC-01), a Mithril manifest parser other than
`parse_mithril_manifest_json` (CN-MITHRIL-01), a Mithril-bootstrap composition that drills into
the manifest import to source the anchor `seed_point` (DC-MITHRIL-02 —
`ci_check_mithril_seed_point_independence.sh`), **a second `SeedEpochConsensusInputs` codec
(CN-CINPUT-01), a forge-time path that populates the seed-epoch sidecar / appends its WAL
provenance (CN-CINPUT-02 — `ci_check_consensus_input_provenance.sh`), a `Mode` enum variant without an
explicit wildcard-free `main.rs` dispatch arm (CN-NODE-MODE-01 — `ci_check_node_mode_closure.sh`), a
second `--mode node` lifecycle owner or a lifecycle arm bypassing `bootstrap_initial_state` (CN-NODE-01 —
`ci_check_lifecycle_owner_uses_bootstrap_initial_state.sh`), a node-sync driver that advances the tip by
any path other than `pump_block` (DC-SYNC-01 — `ci_check_node_sync_via_pump.sh`), a second live-run owner
or a relay-loop body that advances the tip / forges / serves / admits / gossips outside the closed seams
(CN-NODE-02 / DC-SYNC-02 — `ci_check_node_run_loop_containment.sh`), a GREEN planner that observes a
`SlotNo` in `plan_loop_step` or emits a non-closed `LoopStep` (DC-NODE-05 —
`ci_check_loop_planner_closed.sh`), a forge tick that advances a durable tip / serves / gossips / signs a
past or duplicate slot (DC-NODE-05), a node-lifecycle forge path that fabricates a
`SeedEpochConsensusInputs` literal or names a forge-time bundle token (CN-CINPUT-03 / DC-CINPUT-02b —
`ci_check_consensus_input_provenance.sh` guard (d)), or a second `Ba02Manifest` constructor / a
self-evidence acceptance source / a committed synthetic BA-02 manifest (RO-LIVE-06 —
`ci_check_ba02_evidence_closed.sh`).**

---

## 2. Data-Only vs. Authoritative Layers

### Domain: live relay run-loop (GREEN planner / RED driver / BLUE authority-behind-seams; N-F-D, forge-tick N-F-E)

| Layer | Module | Color | Role |
|-------|--------|-------|------|
| **Pure lifecycle planner** | `ade_node::run_loop_planner` (`plan_loop_step`, closed `LoopStep`, content-blind `SyncStatus` + `ForgeSlotStatus`, the pure `forge_slot_status` guard) | GREEN | Decides each iteration's `LoopStep` from content-blind inputs over a total table. Emits ONLY `{SyncOnce, ForgeTick, Idle, HaltCleanly}` — **cannot express an authority decision** (no apply/admit/forge/evidence/verdict variant). `forge_slot_status` is the ONLY fn here that observes a `SlotNo` (a monotonic guard: at most once per slot, never a past slot). With `ForgeSlotStatus::NotDue` the table collapses to the N-F-D relay mapping. Decides nothing semantic. CN-NODE-02 / DC-NODE-05. |
| **Live-run driver (the single owner)** | `ade_node::node_lifecycle::run_relay_loop` | RED | Both `--mode node` arms converge here. Performs effects per the GREEN plan: `SyncOnce → run_node_sync` (the SOLE durable tip-advance), `ForgeTick →` exactly one fenced `forge_one_from_recovered` (N-F-E; advances no tip, serves/admits/gossips nothing), `Idle →` cancellation-safe wait, `HaltCleanly →` exit. Reaches NO second bootstrap/apply/forge/evidence/manual-tip/serve token. CN-NODE-02 / DC-SYNC-02 / DC-NODE-05. |
| **Opt-in forge-activation bundle** | `ade_node::node_lifecycle::ForgeActivation<'a>` (N-F-E) | RED | The closed opt-in bundle (`forge: Option<&mut ForgeActivation>`): borrows the `Clock` seam (sole wall-clock observation), the `CoordinatorState` (reused `kes_period_for_slot`), the recovered `BootstrapState` (SOLE leadership source), the `ProducerShell` (hermetic/fenced key custody), `pool_id`/`pparams`/`protocol_version`/the `millis_to_slot` anchor, and the in-memory `hermetic_forge_outcomes` test-observation field. `None` ⇒ exact N-F-D relay. Decides nothing semantic. |
| **Authoritative durable apply** | `ade_runtime::forward_sync::pump_block` (carried fwd, N-Y) | RED + BLUE admit | The durability-ordered driver `run_node_sync` feeds; the BLUE admit chokepoint + `StoreBlockBytes`/`AppendWal`-before-`AdvanceTip` invariant live here. UNMODIFIED. |
| **Authoritative forge handoff** | `ade_node::node_sync::forge_one_from_recovered` (carried fwd, N-F-C) | RED (driver) → BLUE | Projects leadership ONLY via `PoolDistrView::from_seed_epoch_consensus_inputs`; eligibility is the BLUE leader-schedule. UNMODIFIED in production by N-F-E (`#[cfg(test)]` additions only). CN-CINPUT-03 / DC-CINPUT-02b. |

**Rule (CN-NODE-02 / DC-SYNC-02 / DC-NODE-05):** the GREEN planner decides iteration shape over a closed,
content-blind vocabulary and cannot decide authority; the RED driver performs effects but advances the tip
ONLY via `run_node_sync → pump_block` and (if forging) reaches EXACTLY ONE fenced
`forge_one_from_recovered`, serving/admitting/gossiping nothing; BLUE authority stays behind the existing
seams (admit in `pump_block`; eligibility inside `forge_one_from_recovered`). The wall-clock enters ONLY
through the RED `Clock` seam, and only a `SlotNo` crosses into GREEN/BLUE (DC-NODE-03 / DC-NODE-05). The
forge is **subordinate** to the sync spine — a forged block is a local self-accept artifact, not a tip
advance. **None of these chokepoints move.** This is a **CE-not-law acceptance-criterion** seam (the
`LoopStep` / `ForgeSlotStatus` vocabularies are additively evolvable closed planner enums, like
`WalEntry`), NOT a registry-law surface. **Honest scope:** relay-only + hermetic; the forge tick is
reached only with a `ForgeActivation` (test-only this cluster); a live unbounded peer is the RO-LIVE-01
follow-on; BA-02 is satisfied nowhere.

### Domain: recovered seed-epoch consensus inputs (N-F-A; CONSUMED in N-F-C, exercised by the N-F-E forge tick)

| Layer | Module | Color | Role |
|-------|--------|-------|------|
| **Authoritative canonical record + SOLE codec** | `ade_ledger::seed_consensus_inputs` (`SeedEpochConsensusInputs`, `encode_/decode_seed_epoch_consensus_inputs`, `SEED_CINPUT_SCHEMA_VERSION = 1`) | BLUE | The closed recovered-state record + its SOLE version-gated, byte-canonical encoder/decoder pair. `decode_*` fail-closes on unknown version, wrong shape, short hash, non-canonical / duplicate pool-map keys, trailing bytes, and any non-byte-canonical encoding (closed 6-variant `SeedConsensusInputsError`). No second codec. (CN-CINPUT-01.) |
| **Data-only merge glue** | `ade_runtime::seed_consensus_merge::merge_seed_epoch_consensus_inputs` | GREEN | Lifts a verified-bootstrap two-map `LiveConsensusInputsCanonical` into the BLUE single-map record; fail-closed (closed 2-variant `SeedConsensusMergeError`) on any pool in exactly one source map — never a zero-hash fill. Produces the BLUE record but is NOT its author; decides nothing semantic. |
| **Data-only WAL provenance appender** | `ade_runtime::seed_consensus_provenance::append_seed_epoch_provenance` | RED | `blake2b_256` of the EXACT A1 sidecar bytes → `WalEntry::SeedEpochConsensusInputsImported` (tag 3) append. Allowed only at the two verified-bootstrap composition sites; called only AFTER the durable sidecar put. |
| **Data-only sidecar store** | `ade_runtime::chaindb::SnapshotStore::{put,get,list}_seed_epoch_consensus_*` | RED | Anchor-fp-keyed sidecar namespace **disjoint** from the slot-keyed snapshot space; idempotent on identical bytes, `InvalidOperation` on conflict; redb `seed_cinputs_by_anchor_fp` table, `SCHEMA_VERSION = 3`. N-F-C added the read-only `list_seed_epoch_consensus_anchor_fps` discovery method. No semantic decision. |
| **Authoritative warm-start restore** | `ade_runtime::bootstrap::restore_seed_epoch_consensus_inputs` (inside `bootstrap_initial_state`) | RED read + BLUE verify | The `get_seed_epoch_consensus_inputs` read is the only RED step; bind → decode → anchor/epoch binding → byte-identity verification is BLUE, fail-closed via the 5 `BootstrapError::SeedConsensus*` variants. **MUST NOT** fall back to the forge-time bundle. (DC-CINPUT-01.) Wired on the `--mode node` WarmStart arm. |
| **Authoritative replay fold** | `ade_ledger::wal::replay_from_anchor` (`ReplayOutcome`, `RecoveredBootstrapProvenance`) | BLUE | Folds the additive tag-3 entry into `ReplayOutcome.recovered_provenance` (at most one; `DuplicateProvenance` / `ProvenanceAnchorMismatch` fail closed) **without** disturbing the `AdmitBlock` fp-chain. Pure. |
| **Authoritative projection (consumed since N-F-C; exercised by the N-F-E forge tick)** | `ade_ledger::consensus_view::PoolDistrView::from_seed_epoch_consensus_inputs` | BLUE | Pure field-map projecting the recovered record into the leadership `PoolDistrView` (off-epoch ⇒ `None`; no zero-hash fallback). The SOLE leadership source the node-lifecycle forge handoff consumes (DC-CINPUT-02a + DC-CINPUT-02b / CN-CINPUT-03). |
| **Consume-side forge handoff** | `ade_node::node_sync::forge_one_from_recovered` | RED (driver) | Builds the forge base ENTIRELY from recovered state + the selected tip: projects leadership ONLY via `PoolDistrView::from_seed_epoch_consensus_inputs`, fails closed (`NodeForgeError::MissingRecoveredConsensusInputs`) when none, and MUST NOT fabricate a `SeedEpochConsensusInputs` literal or name a forge-time bundle token. The CONSUME-side fence — CN-CINPUT-03 / DC-CINPUT-02b, `ci_check_consensus_input_provenance.sh` guard (d). _N-F-E exercises it from the relay loop's `ForgeTick` branch (hermetic/test-only); production code UNMODIFIED._ |
| **Read-only anchor discovery** | `ade_runtime::chaindb::SnapshotStore::list_seed_epoch_consensus_anchor_fps` | RED | Returns persisted anchor lineages in ascending order. Discovery is NOT proof; the warm-start verify chain is the authority. Sole caller `node_lifecycle::warm_start_recovery`. |

**Rule (CN-CINPUT-01 / -02 / -03 / DC-CINPUT-01 / -02a / -02b):** the recovered seed-epoch consensus
inputs are a **closed canonical type with a SOLE codec**. The RED/GREEN shells merge, encode, put, and
append provenance; **all semantic decisions (decode, binding verification, the leadership projection) live
in BLUE**. Population is **contained** to the verified-bootstrap composition path; the forge-time path
MUST NOT build / put the sidecar nor append its WAL provenance (CN-CINPUT-02). The warm-start restore +
replay fold live inside the **single `bootstrap_initial_state` authority** and the BLUE
`wal::replay_from_anchor` — **neither chokepoint moves.** The consume side is wired (the `--mode node`
WarmStart arm + `node_sync::forge_one_from_recovered`) behind the consume-side fence (CN-CINPUT-03 /
DC-CINPUT-02b — guard (d): no fabricated literal, no bundle token). **N-F-E note:** the forge tick reaches
this handoff via the relay loop's `ForgeTick` branch with a recovered `BootstrapState` borrowed by
`ForgeActivation`; the projection stays the SOLE leadership source. `produce_mode` stays diagnostic and
still passes `SeedEpochConsensusSource::NotRequired`.

### Domain: node lifecycle + BA-02 evidence (N-F-C)

| Layer | Module | Color | Role |
|-------|--------|-------|------|
| **Lifecycle owner (single authority router)** | `ade_node::node_lifecycle` (`run_node_lifecycle`, `classify_start`, `first_run_mithril_bootstrap`, `warm_start_recovery`, `run_relay_loop`) | RED | THE single `--mode node` recovered-state lifecycle owner (`PHASE4-N-F-C-LIFECYCLE-OWNER`). Classifies FirstRun vs WarmStart as a PURE function of on-disk state; both arms route initial state through the SINGLE `bootstrap_initial_state` authority and converge into `run_relay_loop` (N-F-D). Decides nothing semantic; never a second bootstrap/recovery/storage-init path (CN-NODE-01). |
| **Verdict-decoupled block source** | `ade_node::node_sync::NodeBlockSource` + `run_node_sync` | RED | Yields ordered peer-block BYTES only (skips `TipUpdate`, ends on `Disconnected`; N-F-D added a content-blind readiness signal); `run_node_sync` is the production caller of `forward_sync::pump_block` (durable-before-tip, DC-SYNC-01). NEVER carries/derives a verdict. Driven on the live path by the N-F-D relay loop. |
| **Authoritative durable apply** | `ade_runtime::forward_sync::pump_block` (carried fwd, N-Y) | RED + BLUE admit | The durability-ordered driver the source feeds; the BLUE admit chokepoint + `StoreBlockBytes`/`AppendWal`-before-`AdvanceTip` invariant live here, not in the source. |
| **Evidence correlator (compares already-authoritative outputs)** | `ade_node::ba02_evidence` (`parse_peer_accept_events`, `correlate`) | GREEN | COMPARES the BLUE-minted forged hash (read verbatim from `ForgedBlockArtifact`, never recomputed) against an operator-captured peer-accept signal; `correlate` is the SOLE `Ba02Manifest` constructor. Forges/admits/persists nothing. RO-LIVE-06. _Tested-but-unwired._ |

**Rule (CN-NODE-01 / DC-SYNC-01 / RO-LIVE-06):** the lifecycle owner ROUTES (single
`bootstrap_initial_state` authority on both arms); the block source is data-only (bytes, never a verdict);
the durable apply + admit authority stay in `pump_block`; the BA-02 correlator is GREEN evidence whose SOLE
constructor admits no self-evidence acceptance source and emits no committed synthetic manifest. **None of
these chokepoints move.** **Honest scope:** `ba02_evidence` is tested-but-unwired; `node_sync` is driven
on the live path only by the N-F-D relay loop (SyncOnce) + the N-F-E forge tick (ForgeTick, with a
`ForgeActivation`). **BA-02 is satisfied NOWHERE at this HEAD; RO-LIVE-01 remains partial/operator-gated.**

### Domain: bootstrap seed provenance (N-Y, extended N-Z + N-F-A)

| Layer | Module | Color | Role |
|-------|--------|-------|------|
| **Data-only Mithril shell** | `ade_runtime::mithril_import::{json, importer}` | RED | `parse_mithril_manifest_json` is the SOLE manifest-JSON parser; `import_mithril_manifest` maps it into the closed `SeedProvenance::Mithril` + `MithrilManifestReport`. No semantic decision; never re-verifies the STM multisig. |
| **Mithril production-bootstrap composition** *(N-Z; +N-F-A sidecar tail; N-F-C first non-test caller)* | `ade_runtime::mithril_bootstrap::bootstrap_from_mithril_snapshot` | RED | **Composition-only**: import → mint anchor from the operator-independent `MithrilSeedPointInputs` → BLUE `verify_mithril_binding` fail-closed BEFORE storage init → single `bootstrap_initial_state` (NotRequired) → N-F-A sidecar tail. Symmetric with `bootstrap_from_conway_genesis`. No new authority, no new `SeedProvenance` variant, no CLI surface. Closed `MithrilBootstrapError`. |
| **Data-only genesis shell** | `ade_runtime::genesis_bootstrap::bootstrap_from_conway_genesis` + `producer::genesis_parser` | RED | Reads + parses the Conway genesis file; routes the transform through the single bootstrap authority; runs the same sidecar tail. NOT on the N-F-C node lifecycle (FirstRun is Mithril-only). |
| **Authoritative binding predicate** | `ade_ledger::bootstrap_anchor::binding::verify_mithril_binding` | BLUE | The sole authority deciding whether a Mithril anchor binds — a pure predicate cross-checking `{network_magic, genesis_hash, certified_point, certificate_hash}`; fails closed with `MithrilImportError`. |
| **Authoritative genesis transform** | `ade_ledger::genesis_source::genesis_initial_state` | BLUE | The pure Conway-only transform; fail-closed `GenesisSourceError::NonConwayEra`. |
| **Single bootstrap chokepoint** | `ade_runtime::bootstrap::bootstrap_initial_state` | GREEN-by-content (+ RED A3b restore) | The ONE authority all initial state flows through — `genesis_bootstrap`, the N-Y Mithril provenance path, the N-Z composition, the N-F-A warm-start restore, AND both N-F-C lifecycle arms all route here. Returns the named `BootstrapState`; the `SeedEpochConsensusSource` discriminant selects cold-start vs. warm-start restore. |

**Rule (CN-MITHRIL-01 / CN-NODE-01 / DC-GENESIS-SRC-01 / DC-MITHRIL-02):** the RED shells parse bytes and
produce reports/configs / mint anchors; **all** semantic decisions live in BLUE. All initial state routes
through the **single** `bootstrap_initial_state` authority via `BootstrapInputs.genesis_initial` +
`SeedEpochConsensusSource`. **There is NO `GenesisAnchor` / `MithrilAnchor` trait or plugin seam.**
`verify_mithril_binding` MUST NOT be tautological. New seed-source support adds a RED parse/map shell +
(if a new authoritative decision is needed) a BLUE predicate/transform + (for production wiring) a
composition-only RED twin of `bootstrap_from_{conway_genesis, mithril_snapshot}`; **the
`bootstrap_initial_state` chokepoint never moves.**

### Domain: network forward-sync (durable-before-tip, N-Y; first production driver N-F-C; driven by the N-F-D loop)

| Layer | Module | Color | Role |
|-------|--------|-------|------|
| **Effect-plan reducer** | `ade_runtime::forward_sync::reducer` (`forward_sync_step`, `AdmitPlan::durable`) | GREEN-by-content | Composes the BLUE admit chokepoint and emits the closed `SyncEffect` plan. The private `AdmitPlan::durable` is the **sole** `AdvanceTip` emitter and fixes the durable-before-tip order — an out-of-order plan is structurally inexpressible. |
| **Durability-ordered driver** | `ade_runtime::forward_sync::pump` (`pump_block`) | RED | Applies the reducer's `SyncEffect` plan in order against the persistent `ChainDb` + `FileWalStore` + snapshot writer; refuses to advance the tip before `StoreBlockBytes` + `AppendWal` return Ok — `PumpError::TipBeforeDurable`. Its production caller is `node_sync::run_node_sync`, driven each iteration by the N-F-D relay loop's `SyncOnce`. |

**Rule (DC-SYNC-01 / DC-SYNC-02):** the GREEN reducer decides the effect plan; the RED pump applies it in
durable order. This GREEN-reducer / RED-pump split mirrors the `ade_network::session` (GREEN) /
`ade_runtime::network::mux_pump` (RED) split. `AdvanceTip` is unreachable before `StoreBlockBytes` +
`AppendWal` (`ci_check_forward_sync_chokepoint_only.sh`). N-F-C's `run_node_sync` advances the tip ONLY
via `pump_block` (`ci_check_node_sync_via_pump.sh`); N-F-D's relay loop drives `run_node_sync` each
iteration and the loop body reaches no other tip-advance path (`ci_check_node_run_loop_containment.sh`).
New sync logic adds `SyncEffect` variants + reducer arms; **the single-`AdvanceTip`-emitter chokepoint
never moves.** An **acceptance-criterion** seam, not a registry-law surface.

### Domain: crash recovery (N-Y, extended N-F-A provenance fold; production restart wired N-F-C; loop-as-replay N-F-D/N-F-E)

| Layer | Module | Color | Role |
|-------|--------|-------|------|
| **Recovery wiring (test-only entry)** | `ade_runtime::recovery::restart::recover_node_state` | RED | Composes the EXISTING authorities (`WalStore::read_all` + BLUE `wal::replay_from_anchor` + `rollback_to_slot`) to reconcile the ChainDb to the WAL tail. No second recovery engine. Fail-fast on `WalTailFingerprintMismatch`. Still the test-only secondary entry — the PRODUCTION restart path is the N-F-C WarmStart arm. |
| **Production restart path** | `ade_node::node_lifecycle::warm_start_recovery` | RED | The WarmStart arm: anchor-lineage discovery → WAL replay → restore through the single `bootstrap_initial_state(RequiredFromRecoveredProvenance)` authority. No second recovery engine; fail-closed. Converges into `run_relay_loop`. |

**Rule (recovery-contract / DC-WAL-*; restart wired N-F-C; loop-as-replay N-F-D/N-F-E):** recovery composes
existing authorities; it never re-implements replay or rollback (`ci_check_recovery_contract.sh`). The
N-F-A `ReplayOutcome` additively carries the recovered seed-epoch provenance without disturbing the
`AdmitBlock` fp-chain. **N-F-D/N-F-E extend replay-equivalence to continuous operation (T-REC-03):** the
same recovered/bootstrapped state + the same ordered canonical block feed + the same deterministic loop
inputs + (N-F-E) the same injected clock-tick + shutdown schedule produce byte-identical authoritative
outputs (tips, WAL/checkpoints, halt state, and — N-F-E — the forge-attempt sequence + forged block bytes).
No new durability law. `warm_start_recovery` is the single production restart owner (CN-NODE-01).

### Domain: N2N tag-24 wire envelope (N-X)

| Layer | Module | Color | Role |
|-------|--------|-------|------|
| **Sole byte wrap/unwrap authority** | `ade_codec::cbor::tag24::{wrap_tag24, unwrap_tag24}` | BLUE | The single workspace authority that wraps inner bytes in a tag-24 (`0xd8 0x18`) envelope and strips it. `unwrap_tag24` returns a zero-copy borrow (no re-encode); fails closed with `TagEnvelopeError`. Each defined exactly once. |
| **BlockFetch composition** | `ade_network::codec::block_fetch::{compose,decompose}_blockfetch_block` | BLUE | A served `MsgBlock` payload = `tag24(bytes([era, block]))` — era **inside** the wrap; **Conway = storage index 7**. |
| **ChainSync composition** | `ade_network::codec::chain_sync::{compose,decompose}_rollforward_header, chain_sync_wire_era_index}` | BLUE | A served `RollForward` header = `[era_tag, tag24(bytes(header_cbor))]` — era_tag **outside** the wrap; **CONSENSUS era index, Conway = 6 = storage − 1**. |
| **Serve emitters** | `ade_network::block_fetch::server` / `chain_sync::server` | BLUE | Emit composed (tag-24-wrapped) bytes — never a bare `[era, block]` / bare header. |
| **RED consumers (migrated)** | `ade_node::admission::runner` + `ade_core_interop::follow` | RED | Strip a peer's tag-24 envelope via `ade_codec::unwrap_tag24`; no local parse. |

**Rule (CN-WIRE-08):** one tag-24 byte authority + per-protocol composition layered over it. The two N2N
surfaces use **different era-index schemes** (BlockFetch storage Conway = 7; ChainSync consensus Conway =
6 = storage − 1), pinned byte-identically against cardano-node 11.0.1 captures. No hand-rolled tag-24 parse
in RED. **The wrap/unwrap chokepoint never moves.**

### Domain: block codec (decode + encode)

| Layer | Module | Color | Role |
|-------|--------|-------|------|
| **Authoritative ingress** | `ade_codec::cbor::envelope::decode_block_envelope` + per-era `decode_*_block` | BLUE | Sole `PreservedCbor` construction site; operates over the verbatim tag-24-stripped inner bytes on the wire path (N-X). |
| **Authoritative egress (N-V)** | `ade_codec::cbor::envelope::encode_block_envelope` | BLUE | The single block-envelope encoder; emits storage-form `[era, block]` (Conway = discriminant 7, head `82 07`). |
| **Producer consumer** | `ade_ledger::producer::forge::forge_block` | BLUE | Wraps forged output via `encode_block_envelope`. |

**Rule (CN-FORGE-03, strengthened N-X):** one block-envelope grammar in both directions; forge and validate
share it. The on-wire serve form is the N-X tag-24 composition over this storage-form. **The encode/decode
chokepoint pair never moves.**

### Domain: leader-eligibility VRF input (N-W)

| Layer | Module | Color | Role |
|-------|--------|-------|------|
| **Sole era→construction authority** | `ade_core::consensus::vrf_cert::leader_vrf_input(era, slot, eta0)` | BLUE | The single place selecting a Praos vs TPraos leader-eligibility VRF construction; returns the closed `ExpectedVrfInput`. |
| **Era-correct range-extension** | `ade_core::consensus::vrf_cert::leader_value_for` | BLUE | Praos `praos_leader_value` vs TPraos identity, dispatched on the `ExpectedVrfInput` variant. |
| **Leader-schedule producer** | `ade_core::consensus::leader_schedule::query_leader_schedule` | BLUE | Builds `LeaderScheduleAnswer.expected_vrf_input` via `leader_vrf_input`. |
| **RED prove-step consumer** | `ade_node::produce_mode::run_real_forge` | RED | Proves over `answer.expected_vrf_input.alpha_bytes()`; non-Praos era fail-closes. Reused by `node_sync::forge_one_from_recovered` (the N-F-E forge tick's BLUE engine). |

**Rule (CN-FORGE-04):** exactly one VRF transcript authority per era/protocol version; the Praos producer
alpha MUST equal the validator alpha. No both-alphas fallback. **The era→VRF construction chokepoint never
moves.**

### Domain: KES signing-key custody

| Layer | Module | Color | Role |
|-------|--------|-------|------|
| **Data-only loader** | `ade_runtime::producer::keys::load_kes_signing_key_skey` | RED | Reads the 608-byte cardano-cli skey envelope. |
| **Authoritative deserializer** | `ade_crypto::kes_sum::Sum6Kes::raw_deserialize_signing_key_kes` | BLUE | Byte layout is the structural validator. |
| **Authoritative algorithm** | `ade_crypto::kes_sum` | BLUE | Ade-native Sum6KES, byte-identical to Haskell `cardano-base`. |
| **Signing operation** | `ade_runtime::producer::signing` / `producer_shell::kes_sign_header` | RED | Sole key-custody surface; signs only the branded `UnsignedHeaderPreImage`. |

**Rule:** the RED loader may not call `KesSecret::from_*` inside `load_kes_signing_key_skey` — only the BLUE
deserializer path. Signing is RED-confined; BLUE never signs. **N-F-E note:** the forge tick reuses
`CoordinatorState::kes_period_for_slot` (no new slot→KES code) and consumes only hermetic/fenced
`ProducerShell` material via `ForgeActivation` — real operator KES ingestion in `--mode node` is a separate
key-ingress cluster (§7).

### Domain: leader eligibility (RED/BLUE split)

| Layer | Module | Color | Role |
|-------|--------|-------|------|
| **VRF proof producer** | `ade_node::produce_mode` (prove-step) | RED | Produces the VRF proof/output over the BLUE answer's `expected_vrf_input.alpha_bytes()`. |
| **Authoritative evaluator** | `ade_core::consensus::leader_check::verify_and_evaluate_leader` | BLUE | Verifies + evaluates eligibility from canonical inputs only; emits the closed `LeaderCheckVerdict`. |

**Rule (CN-FORGE-02):** BLUE never sees the VRF/KES/cold keys; the evaluator has no
`LedgerView`/`EraSchedule`/`ChainDepState`/clock/storage/RED dep. The RED/BLUE split never moves.

### Domain: forged-block serving (data-only serve vs. authoritative admit)

| Layer | Module | Color | Role |
|-------|--------|-------|------|
| **Authoritative admit** | `ade_ledger::producer::served_chain::served_chain_admit` | BLUE | Sole entry into the served index; only self-accepted blocks (CN-PROD-04). |
| **Atomic publisher** | `ade_runtime::producer::served_chain_handle::push_atomic` | RED (GREEN-by-content glue) | Wraps `served_chain_admit` in `watch::Sender::send_modify` — no torn snapshot. |
| **Read-side serve** | `ade_network::block_fetch::server::producer_block_fetch_serve` | BLUE | Serves a `RequestRange` only if endpoints + every intervening block are present; emits the tag-24 composition (N-X). |

**Rule:** a forged block is visible to peers only after `push_atomic`; the read-side serve is data-only
over the BLUE `ServedChainSnapshot`. The serve emitter wraps via the single tag-24 authority before bytes
reach a peer (CN-WIRE-08). **N-F-E note:** the relay-loop forge tick does NOT touch this serve path — a
forged block is self-accept-only, recorded into the in-memory `hermetic_forge_outcomes`, never published
(`served_chain_admit` / `push_atomic` are forbidden in the relay-loop body).

---

## 3. Closed vs. Extensible Registries

### Closed (frozen — version-gated changes only)

| Registry | Location | Count | Change Rule |
|----------|----------|-------|-------------|
| `LoopStep` *(EXTENDED 3→4, N-F-E)* | `ade_node::run_loop_planner` (GREEN) | 4 (`SyncOnce` / `ForgeTick` / `Idle` / `HaltCleanly`) | The closed live-run iteration vocabulary the GREEN planner emits. **N-F-E added `ForgeTick`** (3→4). It **cannot express an authority decision** — no apply/admit/forge/evidence/verdict variant. A **CE-not-law additively-evolvable closed planner enum** (like `WalEntry`), NOT a plugin/extension point. New variant = a `plan_loop_step` arm + a fenced RED `run_relay_loop` branch + a strengthening of **CN-NODE-02 / DC-NODE-05** (`ci_check_loop_planner_closed.sh` + `ci_check_node_run_loop_containment.sh`). |
| `ForgeSlotStatus` *(NEW, N-F-E)* | `ade_node::run_loop_planner` (GREEN) | 2 (`Due` / `NotDue`) | The **content-blind** forge-slot planner input. `Due` ⇒ the `ForgeTick` step; `NotDue` ⇒ the table collapses to the N-F-D relay mapping. The planner learns only whether a slot is *due*, NEVER who is a leader (eligibility is BLUE inside `forge_one_from_recovered`). Derived by the pure `forge_slot_status(last_forged_slot, current_slot)` monotonic guard (the only `SlotNo`-observing fn in the module). New variant = a `plan_loop_step` arm + a strengthening of **DC-NODE-05** (`ci_check_loop_planner_closed.sh`). |
| `ForgeActivation` *(NEW, N-F-E)* | `ade_node::node_lifecycle` (RED) | closed opt-in struct (`clock` / `coordinator_state` / `recovered` / `shell` / `pool_id` / `pparams` / `protocol_version` / `anchor_millis` / `start_slot` / `slot_length_ms` / private `last_forged_slot`+`pending_slot` / `hermetic_forge_outcomes`) | The **opt-in forge-activation bundle** threaded into `run_relay_loop` as `forge: Option<&mut ForgeActivation>`. `Some` activates exactly one fenced `forge_one_from_recovered` per `ForgeTick`, advancing no durable tip and serving/admitting/gossiping nothing; `None` reproduces N-F-D relay. **A closed activation surface, NOT an extension point.** A new field = a struct addition behind the closed activation contract + a strengthening of **DC-NODE-05** (`ci_check_node_run_loop_containment.sh`). |
| `Mode` (run-mode set) *(N-F-C)* | `ade_node::cli` (RED) | 5 (`WireOnly` / `Admission` / `KeyGenKes` / `Produce` / `Node`) | The CLOSED `--mode` taxonomy. **NOT `#[non_exhaustive]`**; `Mode::parse` + `main.rs` dispatch are total with **NO wildcard arm**. New variant = a `Mode::parse` arm + an explicit wildcard-free `main.rs` arm + a strengthening of **CN-NODE-MODE-01** (`ci_check_node_mode_closure.sh`). |
| `NodeBlockSource` *(N-F-C; readiness extended N-F-D)* | `ade_node::node_sync` (RED) | 2 (`WirePump(mpsc::Receiver<AdmissionPeerEvent>)` / `InMemory(VecDeque<Vec<u8>>)`) | The **verdict-decoupled** ordered peer-block source: `next_block` yields ONLY `AdmissionPeerEvent::Block` bytes, SKIPS `TipUpdate`, ends on `Disconnected`. N-F-D added a content-blind readiness signal (`has_work_ready` / `is_ended` / `wait_ready`). NEVER carries a verdict. A closed single-method contract — **NOT a plugin point.** New variant = a `next_block` arm + a strengthening of **DC-SYNC-01 / DC-SYNC-02**. |
| `PeerAcceptEvent` *(N-F-C)* | `ade_node::ba02_evidence` (GREEN) | 2 (`PeerServedBlock` / `PeerChainTip`) | The CLOSED **allow-list** of peer-acceptance signals; `parse_peer_accept_events` recognizes ONLY these two discriminators, dropping every weaker/unknown/malformed line. New variant = a parser allow-list arm + a strengthening of **RO-LIVE-06**. |
| `PeerAcceptSource` *(N-F-C)* | `ade_node::ba02_evidence` (GREEN) | 3 (`ServedBlock` / `ChainTip` / `ServedBlockAndChainTip`) | The closed typed provenance of the accepting signal. New variant = a `correlate` source arm + a strengthening of RO-LIVE-06. |
| `NoEvidenceReason` *(N-F-C)* | `ade_node::ba02_evidence` (GREEN) | 4 (`NoPeerAccept` / `HashMismatch` / `ChainPointMismatch` / `ConflictingPeerSignals`) | The closed reason sum for `BA02Outcome::NoEvidence` — NoEvidence is the DEFAULT. New variant = a `correlate` classify arm + a strengthening of RO-LIVE-06. |
| `BA02Outcome` *(N-F-C)* | `ade_node::ba02_evidence` (GREEN) | 2 (`Ba02Manifest(Ba02Manifest)` / `NoEvidence { reason }`) | The closed correlation outcome. `correlate` is the **SOLE** `Ba02Manifest` constructor; no self-evidence acceptance source; no committed synthetic manifest (**RO-LIVE-06**). |
| `Ba02Manifest` schema *(N-F-C)* | `ade_node::ba02_evidence` (GREEN) | versioned struct — `BA02_MANIFEST_SCHEMA_VERSION = 1` | A **version-GATED** canonical evidence manifest (see §4); SOLE constructor `correlate`'s exact-match arm (RO-LIVE-06). |
| `NodeLifecycleError` *(N-F-C; +RelaySync, N-F-D)* | `ade_node::node_lifecycle` (RED) | closed sum (incl. `RelaySync`) | The closed fail-closed lifecycle-owner error set (Mithril-only, no genesis/bundle/cold/graft fallback). N-F-D added `RelaySync` for a relay-loop halt. New variant = a strengthening of **CN-NODE-01 / CN-NODE-02**. |
| `NodeStart` *(N-F-C)* | `ade_node::node_lifecycle` (RED) | 2 (`FirstRun` / `WarmStart`) | The closed start classification — a PURE function of on-disk state. No third "ambiguous" mode. New variant = a strengthening of CN-NODE-01. |
| `NodeSyncError` *(N-F-C)* | `ade_node::node_sync` (RED) | 2 (`Pump(String)` / `Capture(String)`) | The closed sync-driver fail-closed halt set. New variant = a strengthening of **DC-SYNC-01 / DC-SYNC-02**. |
| `NodeForgeError` *(N-F-C; exercised by the N-F-E forge tick)* | `ade_node::node_sync` (RED) | 1 (`MissingRecoveredConsensusInputs`) | The closed forge-handoff fail-closed set: a forge over a base that carries NO recovered seed-epoch record is unrepresentable (no bundle/cold/genesis fallback). New variant = a strengthening of **CN-CINPUT-03 / DC-CINPUT-02b / DC-NODE-05**. |
| `SeedEpochConsensusInputs` *(N-F-A)* | `ade_ledger::seed_consensus_inputs` (BLUE) | closed canonical record; **version-gated** behind `SEED_CINPUT_SCHEMA_VERSION = 1` | The recovered seed-epoch consensus-input record with a **SOLE** encoder/decoder pair. `decode_*` rejects any version != the constant fail-closed, and rejects a structurally-valid-but-non-canonical buffer. No `Default`, no `#[non_exhaustive]`, `BTreeMap`. New field / version = a `decode_*` arm + a `SEED_CINPUT_SCHEMA_VERSION` bump + a strengthening of **CN-CINPUT-01**. No second codec. |
| `SeedConsensusInputsError` *(N-F-A)* | `ade_ledger::seed_consensus_inputs` (BLUE) | 6 (`MalformedCbor` / `UnknownVersion` / `Structural` / `NonCanonicalMapOrder` / `DuplicatePoolKey` / `TrailingBytes`) | The closed `decode_*` failure set. New variant = a strengthening of **CN-CINPUT-01**; non-secret primitives only; MUST fail closed. |
| `SeedConsensusMergeError` *(N-F-A)* | `ade_runtime::seed_consensus_merge` (GREEN) | 2 (`PoolMissingVrfKeyhash` / `PoolMissingStake`) | A pool present in exactly one source map fails closed here, **never a zero-hash fill**. New variant = a strengthening of the merge contract (CN-CINPUT-02). No catch-all. |
| `SeedEpochConsensusSource` *(N-F-A; CONSUME-side wired N-F-C)* | `ade_runtime::bootstrap` (RED) | 2 (`NotRequired` / `RequiredFromRecoveredProvenance(RecoveredBootstrapProvenance)`) | The input-mode discriminant for warm-start. The `--mode node` WarmStart arm passes `RequiredFromRecoveredProvenance`; its construction is contained to {lifecycle owner, `bootstrap.rs`}. New variant = a strengthening of DC-CINPUT-01. |
| `BootstrapError` (N-F-A new variants) | `ade_runtime::bootstrap` (RED) | +5 (`SeedConsensusProvenanceMissing` / `SeedConsensusSidecarMissing` / `SeedConsensusHashMismatch` / `SeedConsensusBindingMismatch` / `SeedConsensusSidecarDecode`) | The fail-closed warm-start-restore failure set; MUST NOT fall back to the forge-time bundle. New variant = a strengthening of **DC-CINPUT-01**; non-secret primitives only. |
| `MithrilBootstrapError` *(N-Z; +N-F-A SeedConsensus* variants)* | `ade_runtime::mithril_bootstrap` (RED) | 3 base + N-F-A `SeedConsensus*` | The closed RED-composition error sum — one variant per composed step. No catch-all/`String`; the binding step is the SOLE semantic decision (BLUE). |
| `MithrilSeedPointInputs` *(N-Z)* | `ade_runtime::mithril_bootstrap` (RED) | closed struct | The **operator-provided, structurally-independent** seed-point origin (DC-MITHRIL-02). A new attested field = a struct addition + a strengthening of DC-MITHRIL-02. |
| `MithrilBootstrapOutput` *(N-Z)* | `ade_runtime::mithril_bootstrap` (RED) | closed struct (`ledger` / `chain_dep` / `tip` / `anchor`) | A new field = a struct addition behind the composition contract. |
| `SeedProvenance` *(N-Y; UNCHANGED by N-Z/N-F-A/N-F-C/N-F-D/N-F-E)* | `ade_ledger::bootstrap_anchor::anchor` (BLUE) | 2 (`CardanoCliJson` / `Mithril { … }`) | **Version-gated** behind `ANCHOR_SCHEMA_VERSION = 2`. Closed — no open/wildcard variant. New variant = a `decode_bootstrap_anchor` arm + an `ANCHOR_SCHEMA_VERSION` bump + a strengthening of **CN-ANCHOR-01 / DC-ANCHOR-01**. |
| `MithrilImportError` *(N-Y)* | `ade_ledger::bootstrap_anchor::binding` (BLUE) | 5 | The closed `verify_mithril_binding` failure set. New variant = a strengthening of **CN-MITHRIL-01 / DC-MITHRIL-01**; MUST fail closed. |
| `MithrilManifestReport` *(N-Y)* | `ade_ledger::bootstrap_anchor::binding` (BLUE) | closed struct | A new attested field = a struct addition + a strengthening of the binding predicate's cross-check. |
| `GenesisSourceError` *(N-Y)* | `ade_ledger::genesis_source` (BLUE) | 1 load-bearing (`NonConwayEra`) | `genesis_initial_state` is Conway-only. New variant = a strengthening of **DC-GENESIS-SRC-01**. |
| `SyncEffect` *(N-Y)* | `ade_runtime::forward_sync::reducer` (GREEN-by-content) | 4 (`StoreBlockBytes` / `AppendWal` / `CommitCheckpoint` / `AdvanceTip`) | The closed forward-sync effect plan. `AdvanceTip` is unreachable before `StoreBlockBytes` + `AppendWal`. New variant = a reducer arm + a pump apply-step + a strengthening of **DC-SYNC-01**. |
| `MithrilManifestError` *(N-Y)* | `ade_runtime::mithril_import::importer` (RED) | closed sum | The closed manifest-JSON parse failure set. No semantic decision. |
| `PumpError` *(N-Y)* | `ade_runtime::forward_sync::pump` (RED) | closed sum (incl. `TipBeforeDurable`) | New variant = a strengthening of **DC-SYNC-01**. |
| `NodeRecoveryError` *(N-Y)* | `ade_runtime::recovery::restart` (RED) | closed sum (incl. `WalTailFingerprintMismatch`) | A WAL-tail fingerprint divergence fails fast. New variant = a strengthening of the recovery contract / **DC-WAL-***. |
| `BlockVerdict` (observable surface) *(N-Y)* | `ade_testkit::harness::sync_diff` (GREEN) | 2 (`Admitted` / `Rejected`) | Compared on observable surfaces only (DC-COMPAT-01). New variant = a strengthening of **DC-COMPAT-01 / RO-SYNC-EVIDENCE-01**. |
| `RegressionFixtureViolation` *(N-Y)* | `ade_testkit::harness::sync_diff` (GREEN) | closed sum | New variant = a strengthening of **RO-SYNC-EVIDENCE-01**. |
| `TagEnvelopeError` *(N-X)* | `ade_codec::cbor::tag24` (BLUE) | 4 (`NotTag24` / `NotByteString` / `Truncated` / `TrailingBytes`) | New variant = a strengthening of **CN-WIRE-08**; non-secret offset/length primitives only. |
| `ExpectedVrfInput` *(N-W)* | `ade_core::consensus::vrf_cert` (BLUE) | 2 (`Praos([u8;32])` / `Tpraos([u8;41])`) | The 2-variant enum IS the protocol-family tag. New variant = a `leader_vrf_input` arm + a strengthening of **CN-FORGE-04**. |
| `LeaderCheckVerdict` *(N-R-A)* | `ade_core::consensus::leader_check` (BLUE) | 2 (`Eligible` / `NotEligible`) | New variant = a strengthening of **CN-FORGE-02**; `NotEligible` carries only a bounded fingerprint. |
| `ForgeFailureReason` *(extended N-W)* | `ade_runtime::producer::producer_log` (GREEN) | closed sum incl. `UnsupportedProducerEra` | New variant = a strengthening of **CN-FORGE-04 / DC-PROD-01**. |
| `OutboundCommand` *(N-S-B)* | `ade_runtime::network::outbound_command` (RED) | typed `ChainSyncServerMsg` / `BlockFetchServerMsg` | New variant = a new typed mini-protocol reply. **No `Vec<u8>` byte tunnel** (CN-OUTBOUND-RELAY-01). |
| `DispatchError` *(N-S-B)* | `ade_node::produce_mode` + `ade_runtime::network::n2n_server` (RED) | closed sum | No `String`/catch-all variant (CN-PEER-OUTBOUND-MAP-01). |
| `ChainEvolutionError` *(N-T)* | `ade_runtime::producer::chain_evolution` (GREEN) | closed sum | New variant = a strengthening of **DC-PROD-03**. |
| `BroadcastPushError` *(N-T)* | `ade_node::produce_mode` (RED) | closed sum | New variant = a strengthening of **CN-PROD-04**. |
| `ProducerLogEvent` *(N-Q)* | `ade_runtime::producer::producer_log` (GREEN) | closed JSONL vocab | New variant = a strengthening of **DC-PROD-01**. No free-form reason strings, no key material. |
| `GenesisParseError` *(N-R-C)* | `ade_runtime::producer::genesis_parser` (RED) | closed sum | New variant = a strengthening of **CN-GENESIS-01**. |
| `OpCertParseError` *(N-R-C)* | `ade_runtime::producer::opcert_envelope` (RED) | closed sum | New variant = a strengthening of **CN-OPCERT-01**. |
| `UnsignedHeaderPreImageError` *(N-S-A)* | `ade_ledger::block_validity::unsigned_header_pre_image` (BLUE) | closed sum | New variant = a strengthening of **DC-KES-HEADER-01**. |
| `AcceptedMiniProtocol` *(N-L)* | `ade_network::session` (GREEN) | closed registry | New mini-protocol = a registry entry + a `match` arm with **no wildcard accept**. |
| `KesError` / `KesParseError` *(N-P)* | `ade_crypto::kes_sum::errors` (BLUE) | 5 / 6 variants | New variant = a strengthening of **DC-CRYPTO-08/09**; non-secret primitives only. |
| Operator-evidence manifest TOML schema *(N-S-C)* | `ci_check_operator_evidence_manifest_schema.sh` + `docs/clusters/completed/PHASE4-N-S-C/cluster.md` | closed key set | Any committed `CE-N-S-LIVE_*.toml` MUST conform (CN-OPERATOR-EVIDENCE-01). |
| Sync-evidence manifest schema *(N-Y)* | `ci_check_sync_evidence_manifest_schema.sh` + `corpus/sync/regressions/` | closed key set | Mirrors the operator-evidence pattern; vacuously satisfied until a manifest is committed (RO-SYNC-EVIDENCE-01, **partial**). |
| `CardanoEra` + Conway cert / governance / withdrawal enums | `ade_types::{era, conway::*}` + `ade_codec::conway::*` | closed | New era / cert / gov-action = a versioned gate (DC-LEDGER-08/09/10/11). `is_praos()` classifies exactly {Babbage, Conway}. |
| Consensus message + verdict enums | `ade_core::consensus`, `ade_ledger::block_validity` / `tx_validity` | closed | `ci_check_consensus_closed_enums.sh` — `match` with no wildcard. |
| JSONL event vocabularies (admission / wire-only / live-log) | `ade_node::{admission_log, live_log}`, `ade_runtime::admission` | closed | New event = a strengthening of the owning DC rule; allow-list + negative tests. |

### Extensible (open within constraints)

| Registry | Location | Extension Rule |
|----------|----------|----------------|
| Ade-native WAL (append-only) | `ade_runtime::wal` (GREEN-by-content) + `ade_ledger::wal::event` (BLUE encoder/decoder) | Append-only; committed entries are never mutated (`ci_check_wal_append_only.sh`). **`WalEntry` is a deliberately CE-not-law surface** — additively evolvable behind the WAL schema (append-only wire tags; `AdmitBlock` = 0, `SeedEpochConsensusInputsImported` = 3, tags 1/2 reserved). Every fp-chain walk `match`es the additive variant explicitly. An acceptance criterion, NOT a frozen registry-law enum. |
| Seed-epoch sidecar store (anchor-fp-keyed) *(N-F-A; consumed N-F-C)* | `ade_runtime::chaindb::SnapshotStore::{put,get,list}_seed_epoch_consensus_*` | A new entry is `put` only on the verified-bootstrap composition path, keyed by `anchor_fp` in a namespace disjoint from the slot-keyed snapshot space; idempotent on identical bytes (redb `seed_cinputs_by_anchor_fp` table, `SCHEMA_VERSION = 3`). N-F-C consumes it via `list_seed_epoch_consensus_anchor_fps` + `get_seed_epoch_consensus_inputs` on the WarmStart arm. The forge-time path may NOT `put` here (CN-CINPUT-02). |
| `PerPeerOutbound` map *(N-S-B)* | `ade_runtime::network::outbound_command` — `Arc<RwLock<BTreeMap<PeerId, mpsc::Sender<OutboundCommand>>>>` | Grows at runtime; **`BTreeMap`, not `HashMap`** — deterministic iteration; no cross-peer byte leakage (CN-PEER-OUTBOUND-MAP-01, DC-OUTBOUND-FIFO-01). |
| `OpCertCounterMap` | `ade_core::consensus::praos_state` (BLUE) | Grows as op-certs are observed; deterministic ordering. |
| `ServedChainSnapshot` (served blocks) | `ade_ledger::producer::served_chain` (BLUE) | Grows via `served_chain_admit` only; `push_atomic` is the sole publisher. (The N-F-E relay-loop forge tick does NOT publish here.) |
| `MempoolState` (admitted txs) | `ade_ledger::mempool` (BLUE) | Grows via `mempool_ingress` → `admit` only; sorted/deduplicated. |
| Seed entries (imported UTxO) | `ade_runtime::seed_import` (GREEN-by-content) | Grows at import time from a cardano-cli UTxO dump; canonical decoders only. |
| Persisted ChainDb (synced blocks) *(N-Y; first production driver N-F-C; driven by the N-F-D loop)* | `ade_runtime::chaindb` via `forward_sync::pump` | Grows via the forward-sync pump applying the GREEN reducer's `SyncEffect` plan in durable order; the tip advances only after `StoreBlockBytes` + `AppendWal` ack (DC-SYNC-01). N-F-C's `node_sync::run_node_sync` is the first production driver; the N-F-D relay loop drives it each `SyncOnce` iteration. |
| Sync regression fixtures *(N-Y)* | `corpus/sync/regressions/` | Each discovered Haskell observable-surface mismatch is committed as a named regression fixture (RO-SYNC-EVIDENCE-01). |
| Sum_n KES family | `ade_crypto::kes_sum` (BLUE) | A new `Sum_n` attaches as an internal type-alias step; the `KesAlgorithm` trait surface does not change. |
| Per-protocol tag-24 compositions *(N-X)* | `ade_network::codec::{block_fetch, chain_sync}` | A new CBOR-in-CBOR composition attaches as a `compose_*` / `decompose_*` pair delegating to the single `ade_codec::{wrap_tag24, unwrap_tag24}` authority (CN-WIRE-08). |
| Bootstrap-source production compositions *(N-Z; +N-F-A sidecar tail)* | `ade_runtime::{genesis_bootstrap, mithril_bootstrap}` | A new bootstrap-source production entry attaches as a **composition-only RED twin** of `bootstrap_from_{conway_genesis, mithril_snapshot}`: import/parse + (if a point is attested) mint the anchor from an operator-independent origin + verify-before-bootstrap (fail-closed) + route through the single `bootstrap_initial_state` authority + the N-F-A sidecar tail. **No new authority, no new `*Anchor` trait/plugin, no new `SeedProvenance` variant unless the source genuinely differs** (CN-MITHRIL-01 / CN-NODE-01 / DC-MITHRIL-02 / CN-CINPUT-02). |

---

## 4. Version-Gated vs. Frozen Contracts

### Frozen (immutable at current version — change = new major version)

- **Live-run loop step vocabulary (N-F-D / N-F-E, CE-not-law)** — the GREEN `LoopStep` (4-variant:
  `SyncOnce` / `ForgeTick` / `Idle` / `HaltCleanly`) + `ForgeSlotStatus` (`Due` / `NotDue`) are closed
  planner enums the loop body matches exhaustively; the planner cannot express an authority decision
  (CN-NODE-02 / DC-NODE-05 — `ci_check_loop_planner_closed.sh`). They are **additively-evolvable
  acceptance criteria** (like `WalEntry`), NOT registry law: a new step is a deliberate planner change.
- **Forge-tick subordination (N-F-E, DC-NODE-05)** — the relay-loop forge tick advances NO durable tip
  and serves/admits/gossips nothing (the single durable tip-advance authority stays `run_node_sync →
  pump_block`); a forge is attempted at most once per `SlotNo` and never for a past slot; the current slot
  is derived ONLY through the RED clock seam (`SystemTime`/`Instant`/float never cross into GREEN/BLUE —
  only a `SlotNo`); leadership eligibility is NOT decided in the loop or the GREEN planner. A forged block
  is a local self-accept artifact only. (`ci_check_node_run_loop_containment.sh`.)
- **Single live-run owner + relay-loop containment (N-F-D, CN-NODE-02 / DC-SYNC-02)** — both `--mode node`
  arms converge into exactly one `run_relay_loop`; every iteration advances the tip ONLY through
  `run_node_sync → pump_block`; no verdict / admission / follower / second-bootstrap / manual-tip /
  (N-F-E) serve path can drive the live tip. (`ci_check_node_run_loop_containment.sh`.)
- **Run-mode taxonomy (N-F-C)** — the `Mode` enum is a CLOSED 5-variant set, **not `#[non_exhaustive]`**,
  with a wildcard-free `main.rs` dispatch arm per variant (CN-NODE-MODE-01).
- **Single `--mode node` lifecycle owner (N-F-C)** — exactly one `PHASE4-N-F-C-LIFECYCLE-OWNER`; both arms
  route initial state through the SINGLE `bootstrap_initial_state` authority; no genesis/bundle/cold/graft
  fallback (CN-NODE-01).
- **Verdict-decoupled block source contract (N-F-C / N-F-D)** — `NodeBlockSource` yields ordered block
  bytes and NOTHING else; `run_node_sync` advances the tip only via `pump_block` (DC-SYNC-01 / DC-SYNC-02 —
  `ci_check_node_sync_via_pump.sh`).
- **Consensus-input provenance fence (N-F-A populate / N-F-C consume)** — the seed-epoch sidecar is
  populated only on the verified-bootstrap composition path (CN-CINPUT-02); the node-lifecycle forge path
  consumes the recovered surface ONLY via `PoolDistrView::from_seed_epoch_consensus_inputs` and may not
  fabricate a literal or name a bundle token (CN-CINPUT-03 / DC-CINPUT-02b —
  `ci_check_consensus_input_provenance.sh` guard (d)). _(N-F-E exercises this fenced path unchanged.)_
- **BA-02 evidence honesty (N-F-C)** — `correlate` is the SOLE `Ba02Manifest` constructor; no self-evidence
  token may be an acceptance source; no committed synthetic manifest (RO-LIVE-06). Wire/forge success ≠
  peer acceptance.
- **Seed-epoch consensus-input codec (N-F-A)** — `encode_/decode_seed_epoch_consensus_inputs` is the SOLE
  codec: deterministic CBOR, `BTreeMap`-ordered, byte-canonical. The wire shape at
  `SEED_CINPUT_SCHEMA_VERSION = 1` is frozen; the codec is version-gated for evolution (below).
  (CN-CINPUT-01.)
- **WAL additive-tag chain disjointness (N-F-A)** — the `SeedEpochConsensusInputsImported` (tag 3) entry
  MUST stay distinct from the `AdmitBlock` fp-chain; `replay_from_anchor` allows at most one provenance
  entry per store/anchor (fail-closed). (DC-CINPUT-01.)
- **Warm-start restore verification chain (N-F-A)** — `restore_seed_epoch_consensus_inputs` verifies in a
  frozen order: `blake2b_256` bind → decode → anchor/epoch binding → byte-identity re-encode, failing
  closed at each step; never falls back to the forge-time bundle. (DC-CINPUT-01.)
- **Sidecar populate ordering (N-F-A)** — put the sidecar (durable) THEN append the WAL provenance (the
  commit point), never the reverse. (CN-CINPUT-02.)
- **Mithril production-bootstrap composition order (N-Z)** — import → mint → `verify_mithril_binding` →
  `bootstrap_initial_state` (→ N-F-A sidecar tail); `verify_mithril_binding` MUST precede
  `bootstrap_initial_state`; the anchor `seed_point` MUST be minted from the operator-independent
  `MithrilSeedPointInputs`. (CN-MITHRIL-01 strengthened / DC-MITHRIL-02.)
- **Mithril provenance binding cross-check (N-Y)** — `verify_mithril_binding` cross-checks the manifest's
  attested `{network_magic, genesis_hash, certified_point, certificate_hash}`; MUST fail closed and MUST
  NOT be tautological. (CN-MITHRIL-01 / DC-MITHRIL-01.)
- **N2N tag-24 wire envelope (N-X)** — the `0xd8 0x18` envelope through the single `wrap_tag24` /
  `unwrap_tag24` authority; per-protocol composition pinned byte-identically against cardano-node 11.0.1
  captures (BlockFetch storage Conway = 7; ChainSync consensus Conway = 6). (CN-WIRE-08.)
- **Leader-eligibility VRF transcript (N-W)** — one era→construction authority (`leader_vrf_input`).
  (CN-FORGE-04.)
- **Block-envelope grammar (N-V)** — storage-form `[era, block]`, Conway = discriminant 7; one encoder,
  one decoder. (CN-FORGE-03, strengthened N-X.)
- **Unsigned-header KES pre-image recipe (N-S-A)** — branded `UnsignedHeaderPreImage`'s only constructor is
  the canonical recipe; byte-identical to the validator extractor. (CN-KES-HEADER-01.)
- **Sum6KES algorithm + expand_seed prefix (N-P)** — byte-identical to Haskell `cardano-base`;
  `expand_seed` prefix bytes `0x01`/`0x02`. 608-byte skey + 448-byte signature layouts pinned.
- **Conway-genesis initial-state transform (N-Y)** — `genesis_initial_state` is the pure Conway-only
  transform. (DC-GENESIS-SRC-01.)
- **Durable-before-tip ordering (N-Y)** — the forward-sync pump persists `StoreBlockBytes` + `AppendWal`
  before the tip write; `AdmitPlan::durable` is the sole `AdvanceTip` emitter. (DC-SYNC-01.)
- **Clock seam (N-K, strengthened N-F-E)** — the orchestrator/relay-loop core depends on a `Clock` trait;
  no `SystemTime::now()` / `Instant::now()` reachable from GREEN/BLUE; only a `SlotNo` crosses the seam.
  (DC-NODE-03.)
- **Wire encoding** — `minicbor` / canonical CBOR; field order = wire order; `PreservedCbor` aliases the
  input bytes (no re-encode for hashing).
- **Hash algorithms** — Blake2b-224 / Blake2b-256; the single `block_body_hash` recipe; the `blake2b_256`
  sidecar-provenance bind (N-F-A).
- **Mux frame format** — single `encode_frame` / `decode_frame` pair workspace-wide.
- **All 456 canonical types** — existing wire formats frozen; new types may be added. (N-F-A added 4 BLUE
  types in `ade_ledger`. **N-F-C / N-F-D / N-F-E added NO BLUE type** — their new types (`Mode::Node`,
  `NodeStart`, `NodeBlockSource`, the BA-02 vocab; `LoopStep`/`ForgeTick`, `ForgeSlotStatus`,
  `ForgeActivation`) live in the RED/GREEN-by-content `ade_node` and do NOT count toward the 456.)

### Version-gated (can evolve across major versions)

- **Live-run loop vocabulary (N-F-D / N-F-E, CE-not-law)** — `LoopStep` / `ForgeSlotStatus` are
  additively-evolvable closed planner enums: a new step = a `plan_loop_step` arm + a fenced RED branch +
  a strengthening of CN-NODE-02 / DC-NODE-05. NOT registry law.
- **BA-02 evidence manifest schema (N-F-C)** — `BA02_MANIFEST_SCHEMA_VERSION` (currently `1`) gates the
  canonical `Ba02Manifest`; bump the tag on any field change. SOLE constructor stays `correlate` (RO-LIVE-06).
- **Seed-epoch consensus-input schema (N-F-A)** — `SEED_CINPUT_SCHEMA_VERSION` (currently `1`) gates
  `decode_seed_epoch_consensus_inputs`. A new field / shape = a `decode_*` arm + an additive version bump +
  a strengthening of CN-CINPUT-01.
- **WAL schema (CE-not-law)** — `WalEntry` is additively evolvable behind the WAL schema version
  (append-only wire tags; `AdmitBlock` = 0, `SeedEpochConsensusInputsImported` = 3, tags 1/2 reserved).
- **redb `chaindb` schema (N-F-A)** — `SCHEMA_VERSION` (currently `3`) gates the on-disk store; a newer
  on-disk schema fail-closes. A versioned gate, NOT a frozen contract.
- **Bootstrap-anchor schema (N-Y)** — `ANCHOR_SCHEMA_VERSION` (currently `2`) gates the `SeedProvenance`
  decode. A new provenance variant = a `decode_bootstrap_anchor` arm + an additive version bump +
  a strengthening of CN-ANCHOR-01 / DC-ANCHOR-01. (N-Z / N-F-A / N-F-C / N-F-D / N-F-E added no new variant.)
- New era support: a `decode_*_block` arm + an `encode_block_envelope` discriminant + a `CardanoEra`
  variant + (leader path) an `ExpectedVrfInput` variant + a `leader_vrf_input` arm + (wire path) the
  per-protocol tag-24 era-index entries.
- New mini-protocol: an `AcceptedMiniProtocol` entry + a BLUE `*_transition` reducer + (serving) an
  `OutboundCommand` variant + (CBOR-in-CBOR) a `compose_*` / `decompose_*` pair.
- New seed source: a RED parse/map shell + (if a new authoritative decision is needed) a BLUE
  predicate/transform + (production wiring) a composition-only RED twin of
  `bootstrap_from_{conway_genesis, mithril_snapshot}`, routed through `bootstrap_initial_state`.
- New `--mode` (N-F-C): a `Mode::parse` arm + a wildcard-free `main.rs` arm + (if it needs initial state)
  routing through the single `bootstrap_initial_state` authority + (if it forges) consuming the recovered
  surface ONLY via `PoolDistrView::from_seed_epoch_consensus_inputs`.
- New `SyncEffect` variant: a reducer arm + a pump apply-step + a strengthening of DC-SYNC-01.
- New closed-enum variant (any §3 closed enum): a `[[rules]]` entry + a strengthening.
- New canonical-type fields (sort/dedup invariants preserved).
- New CI checks (existing checks may be tightened, never relaxed — RO-CLOSE-01).

---

## 5. Module Addition Rules

Derived from CODEMAP's Cross-Module Rules + the shared BLUE header.

| Color | Naming convention | Build-config flags | May depend on | MUST NOT depend on |
|-------|-------------------|--------------------|----------------|--------------------|
| **BLUE** | `ade_*` crate, or a BLUE `ade_network` submodule path in `.idd-config.json` `core_paths`; `// Core Contract:` + `//! BLUE …` banner first line | `#![deny(unsafe_code)]`, `deny(unwrap_used / expect_used / panic / float_arithmetic)`; no `#[cfg(feature = …)]` semantic gating | Other BLUE modules only (`ade_types` ← `ade_codec`/`ade_crypto` ← `ade_core` ← `ade_ledger`/`ade_plutus`; `ade_network` BLUE submodules ← `ade_codec`+`ade_types`) | `ade_runtime`, `ade_node`, `ade_core_interop`, the RED half of `ade_network`; std runtime / I/O / clock / rand / `HashMap` / float / async |
| **GREEN** | `ade_testkit` crate, `ade_network::session`, or a GREEN-by-content sub-tree inside `ade_runtime` / `ade_node` (incl. `forward_sync::reducer`, `seed_consensus_merge` (N-F-A), `ba02_evidence` (N-F-C, evidence correlator), `run_loop_planner` (N-F-D/N-F-E, the pure lifecycle planner), `harness::sync_diff`) with a `//! GREEN …` banner | Same deny attributes as BLUE; a purity CI gate per sub-tree (`run_loop_planner`: `ci_check_loop_planner_closed.sh` — `SlotNo` banned outside the pure `forge_slot_status` guard, closed `LoopStep`/`ForgeSlotStatus`) | BLUE modules | RED modules in non-test deps; nondeterminism; secret material; participation in authoritative outputs |
| **RED** | `ade_runtime`, `ade_node`, `ade_core_interop`, `ade_network::mux::transport` (incl. `forward_sync::pump`, `mithril_import`, `genesis_bootstrap`, `mithril_bootstrap` (N-Z), `seed_consensus_provenance` (N-F-A), `recovery::restart`, `node_lifecycle` (incl. `run_relay_loop` + `ForgeActivation`, N-F-D/N-F-E) + `node_sync` (N-F-C); `*_mode.rs` for mode handlers); `//! RED …` banner | tokio/std/I/O allowed; the `Clock` seam is the SOLE wall-clock observation reachable from a relay-loop/orchestrator driver | Any module | — (RED is the leaf) |

### New module checklist

1. Add to `Cargo.toml` `[workspace] members` (BLUE submodule paths: also add to `.idd-config.json`
   `core_paths`).
2. Apply the `// Core Contract:` + `//! BLUE|GREEN|RED` banner first line (`ci_check_module_headers.sh`).
3. BLUE/GREEN: inherit the deny attributes; pass `ci_check_forbidden_patterns.sh`,
   `ci_check_no_async_in_blue.sh`, `ci_check_no_semantic_cfg.sh`.
4. `ci_check_dependency_boundary.sh` rejects forbidden cross-color imports; `ci_check_pallas_quarantine.sh`
   confines `pallas-*` to `ade_plutus`.
5. New canonical types: add round-trip tests (`canonical_type_registry: null`; canonical-type rules live
   inline in registry family T).
6. New closed surface: add a `[[rules]]` entry + a CI gate; reference it by ID in the docs.
7. **New seed source: route through `bootstrap_initial_state` — NO `*Anchor` trait/plugin seam**
   (`ci_check_mithril_uses_bootstrap_initial_state.sh`). **A production-bootstrap composition attaches as a
   composition-only RED twin of `bootstrap_from_{conway_genesis, mithril_snapshot}`: verify-before-bootstrap,
   fail-closed, operator-independent `seed_point` origin** (`ci_check_mithril_seed_point_independence.sh`),
   **and (N-F-A) a sidecar tail that WRITES — never consumes — the recovered seed-epoch surface, populate-side
   only** (`ci_check_consensus_input_provenance.sh` — CN-CINPUT-02).
8. **New recovered/canonical record with a SOLE codec (like N-F-A):** put the type + its encoder/decoder in
   a single BLUE module; version-gate the decoder; keep it `BTreeMap`-ordered, byte-canonical, no `Default`
   / `#[non_exhaustive]` (CN-CINPUT-01-style).
9. **If a rule cites a moved/renamed source path: update its `code_locus`** —
   `ci_check_registry_code_locus_exists.sh` fails closed on any cited `crates/**.rs` / `ci/**.sh` path that
   does not exist on disk.
10. **New `--mode` (N-F-C rule):** (i) add the variant to the CLOSED `Mode` enum (not `#[non_exhaustive]`);
    (ii) add a `Mode::parse` arm + a wildcard-free `main.rs` arm (`ci_check_node_mode_closure.sh`); (iii) if
    it needs initial state, obtain it via the SINGLE `bootstrap_initial_state` authority (CN-NODE-01,
    `ci_check_lifecycle_owner_uses_bootstrap_initial_state.sh`); (iv) if it forges, obtain consensus inputs
    ONLY via the recovered `SeedEpochConsensusInputs` → `PoolDistrView::from_seed_epoch_consensus_inputs`
    surface (CN-CINPUT-03 / DC-CINPUT-02b, `ci_check_consensus_input_provenance.sh` guard (d)).
11. **New live-run step (N-F-D / N-F-E rule):** (i) add the variant to the closed GREEN `LoopStep` enum +
    a content-blind planner input + a total `plan_loop_step` arm — the planner observes a `SlotNo` ONLY in a
    dedicated pure guard, never in `plan_loop_step` (`ci_check_loop_planner_closed.sh`); (ii) add a fenced RED
    `run_relay_loop` branch that advances the tip ONLY via `run_node_sync` and (if it forges) reaches EXACTLY
    ONE `forge_one_from_recovered`, serving/admitting/gossiping nothing (`ci_check_node_run_loop_containment.sh`
    — CN-NODE-02 / DC-NODE-05); (iii) take any wall-clock observation ONLY through the RED `Clock` seam
    (DC-NODE-03); (iv) make the step opt-in via a closed activation struct (`ForgeActivation`-style) if it
    needs producer/forge material — `None` MUST reproduce the prior relay behavior.

### CI gates that enforce the boundary (110 total; the N-F-D/N-F-E / N-F-C / N-F-A / N-Z / N-Y / producer / network set)

| Script | Enforces | Cluster |
|---|---|---|
| `ci_check_loop_planner_closed.sh` *(NEW N-F-D; EXTENDED N-F-E)* | **CN-NODE-02 / DC-NODE-05** — the GREEN `run_loop_planner` emits only the closed `LoopStep` set and selects steps content-blind. **N-F-E scoped the `SlotNo` ban to `plan_loop_step`** (so the pure `forge_slot_status` monotonic guard may consume a `SlotNo`) and pinned `ForgeTick` / `ForgeSlotStatus` in the closed set. | N-F-D / N-F-E |
| `ci_check_node_run_loop_containment.sh` *(NEW N-F-D; TIGHTENED N-F-E)* | **CN-NODE-02 / DC-SYNC-02 / DC-NODE-05** — the relay-loop body advances the tip ONLY via `run_node_sync`; references NO `run_real_forge` / `correlate(` / `Ba02Manifest` / second-bootstrap path; reaches NO direct manual tip-mutation token; uses no verdict/follower token. **N-F-E** permits **exactly one** fenced `forge_one_from_recovered` (CE-E-4) while ADDING the no-serve tokens (`served_chain_admit` / `push_atomic` / `OutboundCommand` / `broadcast` / `block_fetch` forbidden). A net tightening. | N-F-D / N-F-E |
| `ci_check_lifecycle_owner_uses_bootstrap_initial_state.sh` *(N-F-C)* | **CN-NODE-01** — exactly one `PHASE4-N-F-C-LIFECYCLE-OWNER`; FirstRun routes through `bootstrap_from_mithril_snapshot(` + WarmStart through `bootstrap_initial_state(RequiredFromRecoveredProvenance)`; no parallel/cold init, no fallback, no `recover_node_state(` overclaim. | N-F-C |
| `ci_check_node_sync_via_pump.sh` *(N-F-C)* | **DC-SYNC-01 (driver containment)** — `run_node_sync` advances the tip ONLY via `pump_block(`: no follower-as-sync, no verdict-as-sync, no manual tip advance, no forge/cold/bundle on the sync path. (L5 `forge_one_from_recovered` excluded.) | N-F-C |
| `ci_check_ba02_evidence_closed.sh` *(N-F-C)* | **RO-LIVE-06 (BA-02 honesty)** — exactly one `Ba02Manifest` constructor (inside `correlate`); no self-evidence token as an acceptance source; no committed `docs/evidence/*ba02*` manifest. | N-F-C |
| `ci_check_node_mode_closure.sh` *(N-F-C / N-Q)* | **CN-NODE-MODE-01** — pins the full 5-variant closed `Mode` set with a wildcard-free `main.rs` arm per variant. | N-F-C / N-Q |
| `ci_check_consensus_input_provenance.sh` *(N-F-A; guard (d) N-F-C)* | **CN-CINPUT-02 (populate)** + **CN-CINPUT-03 / DC-CINPUT-02b (consume, guard (d))** — the sidecar is populated only on the verified-bootstrap path; `node_sync::forge_one_from_recovered` must project leadership via `from_seed_epoch_consensus_inputs(`, name no bundle/cold token, and not fabricate a `SeedEpochConsensusInputs { … }` literal. | N-F-A / N-F-C |
| `ci_check_mithril_seed_point_independence.sh` *(N-Z)* | **DC-MITHRIL-02 + CN-MITHRIL-01** — `verify_mithril_binding(` precedes `bootstrap_initial_state(`; the `MintInputs` seed-point RHS never traces to a manifest-origin token. A coherence guard. | N-Z |
| `ci_check_forward_sync_chokepoint_only.sh` *(N-Y)* | DC-SYNC-01 — durable-before-tip; `AdmitPlan` is the sole `AdvanceTip` emitter. | N-Y |
| `ci_check_mithril_uses_bootstrap_initial_state.sh` *(N-Y)* | CN-MITHRIL-01 — the Mithril path routes initial state through the single authority; no `*Anchor` trait/plugin seam. | N-Y |
| `ci_check_no_haskell_fingerprint_equality.sh` *(N-Y)* | DC-COMPAT-01 — the harness compares observable surfaces only. | N-Y |
| `ci_check_sync_evidence_manifest_schema.sh` *(N-Y)* | RO-SYNC-EVIDENCE-01 — closed sync-evidence manifest schema. | N-Y |
| `ci_check_recovery_contract.sh` *(strengthened N-Y)* | recovery-contract / DC-WAL-* — recovery composes existing authorities; fail-fast on `WalTailFingerprintMismatch`. | N-Y |
| `ci_check_registry_code_locus_exists.sh` *(`5db9aae`, extended `a2af041`)* | Registry↔source coherence — every cited `crates/**.rs` + `ci/**.sh` path must exist on disk. | post-N-Y |
| `ci_check_clock_seam.sh`, `ci_check_orchestrator_core_purity.sh` *(N-K; strengthened N-F-E)* | DC-NODE-03 — `clock.rs` is the SOLE `SystemTime::now()`/`Instant::now()` site in `ade_runtime`; the orchestrator/relay-loop core observes no clock/rand/`HashMap`/float — only a `SlotNo` crosses. | N-K / N-F-E |
| `ci_check_tag24_wire_authority.sh` | CN-WIRE-08 — single tag-24 wrap/unwrap authority. | N-X |
| `ci_check_producer_praos_vrf.sh` | CN-FORGE-04 — single era→leader-VRF-input authority. | N-W |
| `ci_check_leader_check_authority.sh` | CN-FORGE-02 — BLUE leader-check has no LedgerView/RED dep. | N-R-A |
| `ci_check_unsigned_header_preimage_single_source.sh` | CN-KES-HEADER-01 / DC-KES-HEADER-01 — single pre-image recipe. | N-S-A |
| `ci_check_no_produce_mode_direct_transport_writes.sh` | CN-OUTBOUND-RELAY-01 — bytes only via `OutboundCommand` → `MuxPump`. | N-S-B |
| `ci_check_operator_evidence_manifest_schema.sh` | CN-OPERATOR-EVIDENCE-01 — closed evidence-manifest TOML schema. | N-S-C |
| `ci_check_produce_mode_uses_bootstrap_initial_state.sh` | CN-PROD-03 / N-T — `produce_mode` obtains initial state only via `bootstrap_initial_state`. | N-T |
| `ci_check_forge_decode_round_trip.sh`, `ci_check_no_independent_forge_codepath.sh` | CN-FORGE-03 — single forge codepath. | N-V |
| `ci_check_producer_coordinator_no_secrets.sh` | CN-PROD-02 — GREEN coordinator holds no secrets. | N-Q |

> **Count history:** the retired `ci_check_constitution_coverage.sh` was removed in `a2af041` (105 → 104),
> its checks folded into `ci_check_registry_code_locus_exists.sh`; the N-F-A
> `ci_check_consensus_input_provenance.sh` restored 105. **N-F-C added 3** gates
> (`ci_check_lifecycle_owner_uses_bootstrap_initial_state.sh`, `ci_check_node_sync_via_pump.sh`,
> `ci_check_ba02_evidence_closed.sh`) → **108**. **N-F-D added 2** gates
> (`ci_check_loop_planner_closed.sh`, `ci_check_node_run_loop_containment.sh`) → **110**. **N-F-E added NO
> gate** — it *extended* both N-F-D gates in place (S1 scoped the `SlotNo` ban + added the forge tokens to
> `ci_check_loop_planner_closed.sh`; S2 added the one-fenced-`forge_one_from_recovered` allowance + no-serve
> tokens to `ci_check_node_run_loop_containment.sh`). Earlier-cluster gates (N-A..N-P, the N-M-* set, the
> N-L wire-session set) are present in the 110 total; per-script detail is in the registry's `ci_script`
> fields. The full list is `ls ci/ci_check_*.sh`.

---

## 6. Forbidden Patterns (per color)

- **BLUE:** no clock, rand, raw `HashMap`/`HashSet`/`IndexMap`, float, env access, network/filesystem, async
  runtime, locale-dependent ops, OS-dependent ordering. No signing (`ci_check_no_signing_in_blue.sh`). No
  `#[cfg(feature = …)]` semantic gating. No `PreservedCbor` construction outside `ade_codec`. No re-encode
  of wire bytes when hashing. No second era→leader-VRF-input construction (CN-FORGE-04). No second
  `wrap_tag24` / `unwrap_tag24` definition (CN-WIRE-08). No second bootstrap/storage-init authority
  (CN-NODE-01 / DC-GENESIS-SRC-01); no tautological Mithril binding check (CN-MITHRIL-01);
  `genesis_initial_state` is Conway-only. **(N-F-A) No second `SeedEpochConsensusInputs` encoder/decoder
  pair (CN-CINPUT-01 — SOLE codec); `decode_*` MUST be version-gated and byte-canonical and keep the record
  `BTreeMap`-ordered with no `Default` / `#[non_exhaustive]`. The `wal::replay_from_anchor` fold MUST keep
  the `SeedEpochConsensusInputsImported` provenance distinct from the `AdmitBlock` fp-chain (explicit
  `match`, no wildcard) and be pure. `PoolDistrView::from_seed_epoch_consensus_inputs` MUST be a pure
  field-map (off-epoch ⇒ `None`, no `HashMap`) — it is the SOLE leadership source for the node-lifecycle
  forge path (exercised by the N-F-E forge tick); it stays a pure field-map.**
- **GREEN:** no nondeterminism; no participation in authoritative outputs. The `producer::coordinator` MUST
  NOT own/store private signing material. `ChainEvolution` (N-T) MUST NEVER mint `AcceptedBlock`. Closed
  vocabularies (`ProducerLogEvent`, `ForgeFailureReason`, `SyncEffect`, observable `BlockVerdict`,
  **`LoopStep` / `ForgeSlotStatus`**) — no open/wildcard variant. `forward_sync::reducer` (DC-SYNC-01): MUST
  NOT emit `AdvanceTip` for a block before that block's `StoreBlockBytes` + `AppendWal`; MUST NOT touch
  sockets/files. **(N-F-A) `seed_consensus_merge` MUST fail closed on a pool in exactly one source map —
  NEVER a zero-hash fill; `BTreeMap` only.** **(N-F-C) `ba02_evidence` is evidence, not authority — it
  COMPARES already-authoritative outputs and MUST read the BLUE-minted forged hash VERBATIM (never
  recompute); `correlate` MUST be the SOLE `Ba02Manifest` constructor; `parse_peer_accept_events` MUST drop
  every weaker/unknown/malformed line; NO self-evidence acceptance source; NO committed synthetic
  `docs/evidence/*ba02*` manifest (RO-LIVE-06).** **(N-F-D / N-F-E) `run_loop_planner` is a pure decision
  function — it MUST observe a `SlotNo` ONLY in the dedicated `forge_slot_status` monotonic guard, NEVER in
  `plan_loop_step` (`ci_check_loop_planner_closed.sh`); it MUST emit only the closed `LoopStep` set and MUST
  NOT decide leadership / authority (its forge input is the content-blind `Due` / `NotDue`); with
  `ForgeSlotStatus::NotDue` the table MUST collapse to the N-F-D relay mapping (CN-NODE-02 / DC-NODE-05).**
  `harness::sync_diff` (DC-COMPAT-01): MUST NOT compare Ade's internal ledger `fingerprint` to a Haskell
  hash. `lagging` ≠ success; wire success ≠ admission ≠ agreement.
- **RED:** no direct mutation of BLUE state; no construction of semantic types from raw bytes; no bypassing
  canonical validation. `produce_mode` emits outbound bytes only via `OutboundCommand` (no direct transport
  write, no `Vec<u8>` byte tunnel). The per-peer outbound map is `BTreeMap` (deterministic). Key custody
  confined to `producer::signing` / `producer_shell`. `run_real_forge` (N-W) MUST NOT perform RED-side era
  dispatch for the leader-VRF alpha. No hand-rolled tag-24 parse (CN-WIRE-08). `forward_sync::pump`
  (DC-SYNC-01) MUST refuse to advance the tip before the durability writes ack (`PumpError::TipBeforeDurable`).
  `mithril_import` MUST perform no semantic decision and route initial state through the single
  `bootstrap_initial_state` authority (CN-MITHRIL-01). `genesis_bootstrap` / `mithril_bootstrap` MUST route
  through the same authority — never a parallel storage-init path (CN-NODE-01 / DC-GENESIS-SRC-01);
  **(N-Z)** mint the anchor `seed_point` from the operator-independent `MithrilSeedPointInputs` ONLY, and run
  `verify_mithril_binding` fail-closed BEFORE `bootstrap_initial_state` (DC-MITHRIL-02). `recovery::restart`
  MUST compose the existing WAL-replay + rollback authorities (no second recovery engine) and fail fast on
  `WalTailFingerprintMismatch`. **(N-F-A) `seed_consensus_provenance::append_seed_epoch_provenance` MUST
  `blake2b_256` the EXACT A1 bytes the composer `put`, be called only AFTER the durable sidecar put, and be
  referenced only at the two verified-bootstrap composition sites. The composers' sidecar tail MUST
  put-then-WAL-append and WRITE — never consume — the recovered surface.
  `bootstrap::restore_seed_epoch_consensus_inputs` MUST fail closed on a missing sidecar / hash mismatch /
  non-canonical decode / binding mismatch / non-byte-identical re-encode, and MUST NOT fall back to the
  forge-time bundle. The forge-time `produce_mode` path MUST pass `SeedEpochConsensusSource::NotRequired`
  and MUST NOT build / put the sidecar (CN-CINPUT-02).** **(N-F-C) The `Mode` enum MUST stay closed (no
  `#[non_exhaustive]`) with a wildcard-free `main.rs` dispatch arm per variant (CN-NODE-MODE-01). There MUST
  be exactly one `--mode node` lifecycle owner; both arms route initial state through the SINGLE
  `bootstrap_initial_state` authority (CN-NODE-01). `NodeBlockSource` MUST yield ordered block bytes and
  NOTHING else; `run_node_sync` MUST advance the tip ONLY via `pump_block` (DC-SYNC-01).
  `node_sync::forge_one_from_recovered` MUST project leadership ONLY via
  `PoolDistrView::from_seed_epoch_consensus_inputs` and MUST NOT fabricate a `SeedEpochConsensusInputs`
  literal or name a forge-time bundle token (CN-CINPUT-03 / DC-CINPUT-02b —
  `ci_check_consensus_input_provenance.sh` guard (d)).** **(N-F-D) There MUST be exactly one live-run owner
  (`run_relay_loop`); the relay-loop body MUST advance the tip ONLY via `run_node_sync` and reach NO second
  bootstrap/apply/forge/evidence/manual-tip/verdict/follower path (CN-NODE-02 / DC-SYNC-02 —
  `ci_check_node_run_loop_containment.sh`).** **(N-F-E) The relay-loop forge tick MUST observe wall-clock
  ONLY through the RED `Clock` seam (only a `SlotNo` crosses; no `SystemTime`/`Instant`/float past the
  observation boundary — DC-NODE-03 / DC-NODE-05); MUST call EXACTLY ONE fenced `forge_one_from_recovered`
  per `ForgeTick`; MUST advance NO durable tip and serve/admit/gossip NOTHING (`served_chain_admit` /
  `push_atomic` / `OutboundCommand` / `broadcast` / `block_fetch` forbidden in the loop body); MUST attempt a
  forge at most once per `SlotNo` and never for a past slot; and is opt-in via `ForgeActivation` — `None`
  MUST reproduce the exact N-F-D relay behavior (DC-NODE-05 — `ci_check_node_run_loop_containment.sh`).**

### Project-specific additions (Ade)

- **Forge-tick honest scope + boundary (N-F-E, load-bearing — do not soften):** N-F-E is a **hermetic,
  single-epoch, self-accept-only** forge-tick wiring cluster. There is **NO operator-key ingestion in
  `--mode node`** (a separate RED key-ingress cluster — the forge tick consumes only hermetic/fenced
  `ProducerShell` material via `ForgeActivation`, test-only this cluster); **NO serve/broadcast/gossip** of a
  forged block; **NO durable apply / tip mutation** by the forge (the single durable tip-advance authority
  stays `run_node_sync → pump_block`); **NO live peer / BA-02 / RO-LIVE claim.** The forge handoff is reached
  only when a `ForgeActivation` is supplied; the binary path passes `None` and is byte-identical to N-F-D
  relay. **BA-02 is satisfied NOWHERE at this HEAD.**
- **Relay-run-loop honest scope (N-F-D):** the live relay run-loop is **relay-only + strictly hermetic** — a
  **live unbounded peer** is the **RO-LIVE-01 follow-on**, not closed at this HEAD.
- **Node-lifecycle honest scope (N-F-C):** PHASE4-N-F-C proves the node lifecycle mechanics through evidence
  closure; it does NOT claim live BA-02. `ba02_evidence` is a tested-but-unwired library surface; RO-LIVE-01
  remains partial/operator-gated; RO-LIVE-06 is only schema/correlation mechanics.
- **Recovered-state surface is populate-contained AND consume-fenced (N-F-A populate / N-F-C consume; N-F-E
  exercises):** the recovered seed-epoch consensus inputs are populated ONLY on the verified-bootstrap path
  and read back ONLY by the warm-start restore (CN-CINPUT-02). The forge-time `produce_mode` path may not
  populate them and stays diagnostic. The consume side (WarmStart arm + `node_sync::forge_one_from_recovered`)
  is fenced (CN-CINPUT-03 / DC-CINPUT-02b — guard (d)). The N-F-E forge tick reaches that fenced path from
  the relay loop's `ForgeTick` branch.
- **No new bootstrap-source plugin seam (N-Y hard rejection, carried into N-Z + N-F-A + N-F-C):** a new seed
  source attaches by populating `BootstrapInputs.genesis_initial` and routing through
  `bootstrap_initial_state` — NEVER via a `GenesisAnchor` / `MithrilAnchor` trait or plugin registry. A new
  `--mode` needing initial state routes through the same one authority (CN-NODE-01). An acceptance criterion,
  not a registry invariant.
- **Mithril seed-point independence (N-Z hard rule, DC-MITHRIL-02):** the anchor `seed_point` MUST originate
  from an operator-supplied origin structurally independent of the manifest; `verify_mithril_binding`
  cross-checks the two and fails closed; the binding must run before any storage init.
- **No synthetic forge state (N-T):** `produce_mode` MUST NOT construct `SyntheticForgeInputs`, a zero-stake
  `LeaderScheduleAnswer`, or an inline `LedgerState::new(...)` forge base.
- **No durability in the produce_mode forge path (N-U scope):** forged-block durability is deferred to N-U
  (§7). The network forward-sync durability (received blocks) DID land in N-Y and gained its first production
  driver in N-F-C's `node_sync`; the N-F-E relay-loop forge tick advances NO durable tip (self-accept-only).
- **Registry `code_locus` must track source moves (`5db9aae`):** any rule citing a renamed/moved
  `crates/**.rs` or `ci/**.sh` path must have its `code_locus` updated; `ci_check_registry_code_locus_exists.sh`
  fails closed on a stale pointer.
- **`cardano_crypto::kes` is a `#[cfg(test)]` oracle only** under `crates/ade_crypto/src/**`. `pallas-*`
  confined to `ade_plutus`.
- **Commit-attribution override (CLAUDE.md):** this repo carries a model-attribution trailer on commit
  messages only (bounty requirement). Source comments, PRs, releases, issue comments still follow the global
  no-AI-attribution rule.
- **Grounding-doc → ade-atlas rebuild trigger (operational infra — NOT a code seam):** the downstream
  `ade-atlas` repo polls the grounding docs every 10 min (the in-repo `notify-atlas.yml` dispatch was removed
  in `1d29597`). It attaches nothing to the node's authority surface.

---

## 7. Candidate & Not-Yet-Wired Seams (declared follow-ons — NOT closed)

> Surfaced honestly per IDD: these are **declared** future attach points, not closed surfaces. Each is named
> in a registry rule or a cluster CLOSURE record.

1. **Real `--mode node` operator-key ingress (the next cluster — DECLARED, NOT wired).** N-F-E wires the
   forge tick but consumes only **hermetic/fenced** `ProducerShell` material via `ForgeActivation` (test-only
   at this HEAD; the binary path passes `None`). A real `--mode node` forge needs a **separate RED
   key-ingress cluster**: operator KES / VRF / cold / opcert / pool-id / pparams ingress. _Candidate seams
   the user should confirm:_
   - **A RED key-ingress surface for `--mode node`** — parse the operator KES skey (via the BLUE Sum6KES
     deserializer), opcert envelope, cold/VRF keys, pool-id, and protocol params, then construct a real
     (non-hermetic) `ForgeActivation` and pass `Some(...)` into `run_relay_loop`. _Confirm: is operator-key
     ingress the next cluster, and does it stay a RED parse → BLUE structural-validator → canonical-type
     pipeline (no new BLUE authority, no plugin seam)?_
   - **Reaching the forge tick from the binary path with real producer material** — making
     `LoopStep::ForgeTick` reachable in production (currently `None` ⇒ never `ForgeTick`). _Confirm scope
     before flipping the binary path to `Some(...)`._

2. **Live unbounded peer for the relay loop (RO-LIVE-01 follow-on — DECLARED).** N-F-D/N-F-E are relay-only +
   hermetic; the run loop drives `run_node_sync` over a bounded/in-memory feed. A **live unbounded
   cardano-node peer over the wire** is the declared follow-on (operator-gated). _Confirm: is the live-peer
   wiring the next live leg, and does `NodeBlockSource` stay a closed verdict-decoupled contract (not a plugin
   point for alternative sources)?_

3. **Live BA-02 (RO-LIVE-06 follow-on — DECLARED).** The schema + correlator mechanics are closed; a real
   BA-02 result needs a real operator-captured peer log naming the exact Ade-forged hash, run through
   `correlate` (operator-gated, distinct from RO-LIVE-01). Synthetic fixtures prove the mechanics only.
   _Confirm scope before wiring `ba02_evidence::correlate` to a live binary arm / committing any manifest._

4. **Mithril import — remaining open obligations (RO-MITHRIL-IMPORT-01, still `partial`).** N-Z CLOSED item
   (b) (`bootstrap_from_mithril_snapshot` is the wired production composition, first non-test caller = the
   N-F-C FirstRun arm). Two seams remain deliberately NOT wired: **(a) seed-bytes-from-Mithril decode** (a
   Mithril artifact-type spike + forward-replay) and **(c) a committed reproducible Mithril fixture +
   CI/release evidence**. `bootstrap_from_mithril_snapshot` is composition-only with **NO standalone argv
   flag**.

5. **N-U — forged-block durability (DECLARED).** WAL / ChainDB / snapshot / warm-start for producer-**forged**
   blocks. Out of N-T scope (`open_obligation` on CN-PROD-03 / DC-PROD-03). The N-Y forward-sync durability
   covers **received** blocks (N-F-C's `node_sync::run_node_sync` is its first production driver; the N-F-E
   forge tick is self-accept-only and advances no durable tip). Distinct from the N-F-A recovered
   consensus-input sidecar (bootstrap-time provenance, not forge-time block durability).

6. **Sync-evidence live leg (N-Y — RO-SYNC-EVIDENCE-01, `partial`).** The snapshot→tip sync-evidence manifest
   schema is enforced but vacuously satisfied until a manifest is committed. An operator-witnessed execution
   gate, not a code seam.

### Operator-pass execution gates (schema enforced, execution blocked)

- **CN-OPERATOR-EVIDENCE-01 / CN-CONS-06 / RO-LIVE-01** — the manifest schema is enforced, but C1 (private
  testnet) / C2 (preprod) operator-pass execution is `blocked_until_operator_pass_executed`. With CN-FORGE-04
  (N-W) and CN-WIRE-08 (N-X) enforced, the producer forge composition is mechanically complete through the
  serve step. The remaining blocker is the OPERATOR-PASS live leg itself.
- **RO-LIVE-06 (BA-02, N-F-C)** — the evidence schema + correlator mechanics are enforced, but a real BA-02
  result is operator-gated. Synthetic fixtures CANNOT satisfy BA-02. Distinct from RO-LIVE-01.

---

## Generation notes

- Regenerated (scoped INCREMENTAL catch-up) at HEAD `cd2484f` (`git rev-parse --short HEAD`), downstream of
  the CODEMAP regenerated at the same HEAD. The prior on-disk SEAMS was generated at the **PHASE4-N-F-C
  close** (`71a6c80` / 108 CI checks / 303-306 rules) — **two clusters stale**. This refresh catches it up
  through **PHASE4-N-F-D** (the live relay run-loop, closed at origin/main `7de1462`: `run_relay_loop`, the
  GREEN `run_loop_planner` with the closed 3-variant `LoopStep`, the `run_node_sync → pump_block`
  containment, CN-NODE-02 / DC-SYNC-02 / T-REC-03 + the two new gates) AND **PHASE4-N-F-E** (the forge-tick
  wiring, closing now at `cd2484f`: `LoopStep` 3→4 + `ForgeTick`, the closed `ForgeSlotStatus`, the opt-in
  `ForgeActivation`, the clock-seam slot derivation, DC-NODE-05, both N-F-D gates extended in place).
- **N-F-D/N-F-E deltas are surface REDUCTIONS / closed-vocabulary extensions, not new extension points
  (load-bearing).** `LoopStep` / `ForgeSlotStatus` are additively-evolvable CE-not-law closed planner enums
  (like `WalEntry`); `ForgeActivation` is a closed opt-in activation bundle. All are classified under §3
  Closed / §4 Frozen+version-gated. **No BLUE crate was modified** by N-F-D or N-F-E — the 456 canonical-type
  total is unchanged; all code lands in the RED binary/driver `ade_node` (`run_loop_planner`
  GREEN-by-content, `node_lifecycle` RED, `node_sync` RED — production unchanged by N-F-E, `#[cfg(test)]`
  additions only).
- **Honest scope (load-bearing).** N-F-E is **hermetic, single-epoch, self-accept-only**. NO operator-key
  ingestion in `--mode node`; NO serve/broadcast/gossip of a forged block; NO durable apply / tip mutation by
  the forge; NO live peer / BA-02 / RO-LIVE claim. The forge handoff is reached only when a `ForgeActivation`
  is supplied (test-only this cluster); the binary path passes `None` (byte-identical to N-F-D relay). N-F-D
  is relay-only + hermetic; a live unbounded peer is the RO-LIVE-01 follow-on. **BA-02 is satisfied nowhere
  at this HEAD.**
- N-F-D/N-F-E delta verified at `cd2484f` (grep/ls/git only — no `cargo`):
  - `run_loop_planner.rs` (`//! GREEN` banner): closed `LoopStep` 4-variant (`:96` —
    `SyncOnce`/`ForgeTick` (`:104`)/`Idle`/`HaltCleanly`); closed `ForgeSlotStatus { Due, NotDue }` (`:68`);
    `plan_loop_step(loop_state, sync_status, forge_slot_status, shutdown)` (`:132`, with `Due ⇒
    LoopStep::ForgeTick` at `:147`/`:199`); pure `forge_slot_status` monotonic guard (`:162`).
  - `node_lifecycle.rs` (`//! RED` banner, `PHASE4-N-F-C-LIFECYCLE-OWNER` marker): `pub struct
    ForgeActivation<'a>` (`:400` — `clock` (the sole wall-clock observation) / `coordinator_state` (reused
    `kes_period_for_slot`) / `recovered: &BootstrapState` (SOLE leadership source) / `shell` / `pool_id` /
    `pparams` / `protocol_version` / `anchor_millis` / `start_slot` / `slot_length_ms` / private
    `last_forged_slot` + `pending_slot` / `hermetic_forge_outcomes`); `run_relay_loop` (`:465`) takes
    `forge: Option<&mut ForgeActivation<'_>>` (`:473`).
  - Gates `ci/ci_check_loop_planner_closed.sh` + `ci/ci_check_node_run_loop_containment.sh` present;
    `ls ci/ci_check_*.sh | wc -l` = **110**.
  - Registry: `grep -cE '^id = '` = **310** (working tree). `DC-NODE-05` (`:5321`) `status = "enforced"`
    with populated `tests` + `ci_script`; `CN-NODE-02` (`:5242`), `DC-SYNC-02` (`:5280`), `T-REC-03`
    (`:5300`) `introduced_in = "PHASE4-N-F-D"` `status = "enforced"` with `strengthened_in +=
    "PHASE4-N-F-E"`; `DC-NODE-03` (`:3039`) `strengthened_in` includes `"PHASE4-N-F-E"`.
  - **Registry-status note:** `docs/ade-invariant-registry.toml` is `M` in `git status` (the working-tree
    diff is the six `strengthened_in += "PHASE4-N-F-E"` bumps + the `DC-NODE-05` `declared → enforced` flip
    — `tests` + `ci_script` populated). **No `id =` was added/removed** — the count is stable at 310
    (committed = working-tree). The staged delta is owed alongside this refresh at the close commit.
- Counts at `cd2484f` (N-F-E close, this refresh): **456** canonical types (Δ 0 — no BLUE crate modified;
  all N-F-D/N-F-E types are RED/GREEN-by-content `ade_node`, not counted), **110** CI checks (Δ +2 vs the
  prior SEAMS's 108 — the two N-F-D gates; N-F-E added none, extending both in place), **310** registry
  rules (Δ 0 vs the working-tree 310 the CODEMAP cites; the N-F-E rule `DC-NODE-05` was already present as a
  `declared` sketch row — N-F-E flipped it to `enforced`).
- All N-F-C / N-F-A / N-Z / N-Y closed surfaces re-verified present on disk at this HEAD and unchanged by
  N-F-D / N-F-E (no BLUE crate modified); the refresh annotated only the seams N-F-D/N-F-E added (the relay
  loop + the forge tick) and the surfaces they extended (the GREEN planner vocabulary + the RED activation
  bundle).
- `.idd-config.json` `_invariant_registry_doc` reads "309 entries" and `_head_deltas_baseline` is `51c9fbf`
  / `71a6c80` — both **stale**: the registry is **310** at HEAD and the HEAD_DELTAS baseline should be bumped
  to `cd2484f` on the N-F-E HEAD_DELTAS refresh. (Surfaced for the user; this doc does not edit config.)
- The doc is regenerated, not edited. If a value drifts, fix the source, not the doc.
- NOTE: no `cargo build`/`test`/`check` was run during this regeneration (grep/ls/git only, per the task
  constraint).
