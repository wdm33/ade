# Ade — HEAD Deltas (Changes Since Baseline)

> **Status:** Living architectural document. Regenerated; not hand-edited.
>
> Regenerate with `/head-deltas <baseline>` after every cluster close. Baseline is recorded in `.idd-config.json` `head_deltas_baseline`.

> Baseline: `e87e8a43` (PHASE4-N-AL AL-S1 — participant recovered-anchor rollback no-op, DC-NODE-33, 2026-06-10 21:43)
> HEAD: `b8860b16` (archive PHASE4-N-AM + PHASE4-N-AN to completed/, 2026-06-11 12:48)
> Span: **two closed clusters — PHASE4-N-AM (the wire-pump keep-alive CLIENT that sustains the live follow past the peer's ~97s keep-alive timeout, `DC-PUMP-03`) and PHASE4-N-AN (rollback-materialization replay-equivalence — the recovered seed-epoch eta0 is overlaid onto the replay `chain_dep` so a rolled-back block validates its header VRF against the SAME nonce live admit used, `T-REC-06`)** — plus a **stale-gate triage** (`89facbea`, CI-script + registry only — `DC-PUMP-02` refined for the AI-S4a `RollBackward` event + `+= PHASE4-N-AN`; two pre-existing red gates repaired) and the **cluster-doc archive** (`b8860b16`, moved both clusters to `docs/clusters/completed/`).
> **12 commits** (no merges), **32 files changed, +2288 / −516 lines**. **This span TOUCHES BLUE but adds ZERO new canonical type** — `ade_core::consensus::praos_state`, `ade_ledger::receive::reducer`, and `ade_ledger::rollback::materialize` all changed (N-AN's eta0 overlay), but **no new `pub struct`/`pub enum`**: BLUE canonical types **462 → 462** (CODEMAP's BLUE-tree metric; **486 → 486** by the raw BLUE `pub struct`/`pub enum` grep, **921 → 921** whole-tree). **NO new crate** (`git diff --diff-filter=A '**/Cargo.toml'` empty — still **11 crates**), **NO new module** (`git diff --diff-filter=A 'crates/**/*.rs'` empty), and **NO new canonical type**. **+2 CI gates** (`ci_check_keep_alive_wire_only.sh` for `DC-PUMP-03`, `ci_check_rollback_materialize_eta0.sh` for `T-REC-06`; 3 modified in place, 0 removed — **159 → 161**). **Registry 359 → 361** (+2 rules `DC-PUMP-03` + `T-REC-06`, both enforced at close; +1 strengthening `DC-PUMP-02 += "PHASE4-N-AN"`; **zero removals**). The substantive production change is **RED** — the keep-alive client in `ade_runtime::admission::wire_pump` (+285) — plus the **BLUE** eta0-overlay authority threaded through `materialize_rolled_back_state` (`ade_ledger::rollback::materialize` +140, `ade_core::consensus::praos_state` +15) and its RED carriers (`ade_runtime::bootstrap` +51, `ade_runtime::forward_sync::reducer` +9, `ade_node::node_lifecycle` +12).

> **Baseline note (load-bearing — read before §0).** This window's baseline is **`e87e8a43`**, the
> PHASE4-N-AL AL-S1 close (the prior HEAD_DELTAS HEAD), and it is **valid**: `git rev-parse e87e8a43` resolves and
> `git merge-base e87e8a43 HEAD == e87e8a43` (it is a strict ancestor of HEAD; `e87e8a43` carries no tag). HEAD is
> **`b8860b16`** (the cluster-doc archive that closes the N-AM + N-AN window). The **committed** `.idd-config.json`
> baseline at HEAD already reads **`e87e8a43`** (the prior N-AL regen's bump committed cleanly), so this window measures
> from the committed baseline forward with **no working-tree skew**. The span has **four parts**: (1) the
> **PHASE4-N-AL close commit** — `35a851b9` (`Close PHASE4-N-AL …`), which flipped `DC-NODE-33` `declared → enforced`
> (populating its `tests` array) and regenerated all four grounding docs to `e87e8a43` + archived the N-AL cluster docs —
> **docs/registry only, 0 code** (registry stayed 359; this is the previously-uncommitted N-AL close-pass, now committed);
> (2) the **PHASE4-N-AM cluster** (`DC-PUMP-03`) — the wire-pump keep-alive client (5 commits: invariants note + declare,
> AM-S1 impl, bank, close); (3) the **PHASE4-N-AN cluster** (`T-REC-06`) — rollback-materialize eta0 replay-equivalence
> (4 commits: declare, AN-S1 repro, AN-S2 fix, CE-AN-LIVE record); and (4) a **stale-gate triage** (`89facbea`,
> CI + registry only) + the **archive** (`b8860b16`).
>
> **Working-tree note.** At this regen the working tree is **CLEAN** — `git status --porcelain` shows only untracked
> scratch (`.mithril-scratch/`, `wire_smoke.jsonl`), neither part of this doc. Unlike the prior (N-AL) regen, there are
> **no uncommitted close artifacts**: the N-AM + N-AN closes (`8b18fc8e`, the `T-REC-06` flip in `deaa6e28`, the archive
> `b8860b16`) are all committed. §1 narrates the committed span `e87e8a43..b8860b16` verbatim from `git log`; §0/§7 read
> rule **status** from the registry at HEAD `b8860b16` (`DC-PUMP-03` + `T-REC-06` **enforced**, **361** rules). The
> operator bumps `head_deltas_baseline` `e87e8a43 → b8860b16` as the **post-close step this regen performs** (and demotes
> the N-AL paragraph to "PRIOR baseline").

This window is **led by two tightly-paired live-readiness clusters that together unblock the CE-AI-6 induced-reorg
convergence capture** — both surfaced by trying to run a live participant follow long enough to capture a peer reorg:

1. **PHASE4-N-AM — the wire-pump keep-alive CLIENT.** The live follow died at ~96s with the relay
   `ShutdownPeer`'ing a silent client (`ExceededTimeLimit (KeepAlive) ClientHasAgency`). The cluster adds the N2N
   keep-alive client to the SOLE per-peer wire pump so the link survives the peer's ~97s keep-alive deadline.
2. **PHASE4-N-AN — rollback-materialization replay-equivalence.** With the link sustained, the actual reorg-follow
   then died at `ReplayFailedAt … VrfCert`: rollback-materialize was replaying the rolled-back block against the
   persisted snapshot's `Nonce::ZERO` PLACEHOLDER nonce, not the recovered seed-epoch eta0 the live-admit path uses.
   The cluster overlays the recovered eta0 onto the replay `chain_dep` so the rollback path validates the header VRF
   against the SAME nonce as live admit (replay-equivalence by construction).

With both fixes the **CE-AI-6 reorg capture** ran live (recorded as `T-REC-06`-backing evidence, NOT a `RO-LIVE` flip).

### PHASE4-N-AM — wire-pump keep-alive client (`DC-PUMP-03`, enforced)

> **PHASE4-N-AM runs the N2N keep-alive CLIENT (mini-protocol 8) inside `run_admission_wire_pump` — the SOLE
> per-peer pump (`CN-PUMP-01`).** A `tokio::select!` cadence arm, firing every `KEEP_ALIVE_CADENCE = 20s` STRICTLY
> under the peer's observed ~97s keep-alive timeout, sends `KeepAliveMessage::KeepAlive(cookie)` (Initiator) via the
> EXISTING outbound `OutboundFrame` path and advances the REUSED BLUE `ade_network::keep_alive` state machine
> (`keep_alive_transition`: `ClientIdle → ServerHasAgency{cookie}`); the inbound `MsgResponseKeepAlive(cookie')`
> advances the SAME machine back to `ClientIdle`, validating `cookie' == cookie`. **WIRE-ONLY:** the keep-alive
> client produces no canonical input, no WAL entry, and **NO `AdmissionPeerEvent`** (`Block` / `TipUpdate` /
> `RollBackward` / `Disconnected`) — the `DC-PUMP-01` emit-set stays unwidened; it never touches admission, the
> durable chain, fork-choice, the convergence-evidence vocabulary, replay-equivalence, or any BLUE state. The cookie
> is a monotonic `u16` counter (deterministic — no `rand`); the cadence is wall-clock, a RED transport concern that
> never reaches the BLUE core. A grammar violation (cookie mismatch / illegal transition / undecodable payload) fails
> closed via `AdmissionWirePumpError::KeepAlive` — drop the peer. **MUST NOT:** redefine the BLUE keep-alive grammar
> (reuse `ade_network::keep_alive` + `::codec::keep_alive`); use a cadence ≥ the peer timeout; send a new
> `MsgKeepAlive` while one is in flight (respect `ServerHasAgency`); block / starve / reorder the chain-sync or
> block-fetch flow; dispatch a keep-alive frame as a chain-sync/block-fetch event (closed match over
> `AcceptedMiniProtocol`); implement a keep-alive SERVER/responder (client only — a responder is a CE-AM-LIVE-gated
> follow-on). **SCOPE:** the keep-alive client ONLY — does NOT add multi-peer ChainSel, does NOT flip `CN-CONS-03`,
> does NOT broaden `CN-PUMP-01` / `DC-PUMP-01` / `DC-PUMP-02`. RED-only; NO BLUE change; NO new module; NO new
> canonical type.

- **PHASE4-N-AM / AM-S1 / `DC-PUMP-03` (enforced) (RED keep-alive client in the wire pump).** `run_admission_wire_pump`
  (`crates/ade_runtime/src/admission/wire_pump.rs`, +285) gains: (a) a `const KEEP_ALIVE_CADENCE: Duration =
  Duration::from_secs(20)`; (b) per-peer keep-alive state (`keep_alive_state: KeepAliveState::ClientIdle`, a monotonic
  `next_cookie: u16`, a `KeepAliveVersion` derived from the negotiated version, a `tokio::time::interval` with
  `MissedTickBehavior::Delay` whose immediate first tick is consumed so the first ping fires after one full cadence);
  (c) a `tokio::select!` over the inbound `recv()` (byte-identical to pre-AM behaviour — `mpsc recv()` is cancel-safe,
  so a keep-alive tick never drops a chunk) and the cadence `tick()` — the cadence arm, only while `ClientIdle`,
  drives `keep_alive_transition(…, KeepAliveAgency::Client, …, KeepAlive(cookie))`, enqueues the encoded frame on the
  EXISTING outbound path, and `continue`s; (d) a `handle_keep_alive(payload, &mut state, version)` consuming the
  peer's `MsgResponseKeepAlive` via `decode_keep_alive_message` + `keep_alive_transition(…, KeepAliveAgency::Server,
  …)` — agency fixed `Server` (Ade is the client), so a client-originated message from the peer is an
  `IllegalTransition` and fails closed, and an undecodable payload is a `MalformedMessage`. The `AcceptedMiniProtocol`
  match's previously-silent `KeepAlive` drop is replaced by a call to `handle_keep_alive`; all other accepted
  mini-protocols stay silently dropped. New error variant `AdmissionWirePumpError::KeepAlive(KeepAliveError)`. The
  keep-alive paths emit only stderr diagnostics — NO `AdmissionPeerEvent`. +3 hermetic tests (CE-AM-1
  `wire_pump_sends_keep_alive_on_quiescent_cadence`, under `#[tokio::test(start_paused)]` virtual time, asserts a real
  proto-8 `MsgKeepAlive` over a loopback mux during inbound quiescence AND no `AdmissionPeerEvent`; CE-AM-2
  `wire_pump_keep_alive_response_validates_cookie_no_event`; CE-AM-3 `wire_pump_keep_alive_cookie_mismatch_fails_closed`
  — mismatch + unsolicited-`ClientIdle` response + undecodable payload all fail closed). New gate
  `ci_check_keep_alive_wire_only.sh`. The lone manifest touch is a **dev-dependency** `tokio` `test-util` feature in
  `crates/ade_runtime/Cargo.toml` (+4 — for the `start_paused` virtual clock; dev-only, never in production builds,
  NOT a project `[features]` flag).

### PHASE4-N-AN — rollback-materialization replay-equivalence (`T-REC-06`, enforced)

> **PHASE4-N-AN makes rollback-materialize reconstruct the replay `chain_dep` with the SAME recovered seed-epoch eta0
> the live-admit path uses (`T-REC-06`).** `materialize_rolled_back_state` (the SOLE rolled-back-state authority,
> `CN-STORE-07`) gains a `recovered_eta0: Option<&Nonce>` parameter and overlays it onto the nearest-snapshot
> `chain_dep` — via the NEW SINGLE overlay authority `PraosChainDepState::overlay_recovered_eta0` (sets both
> `epoch_nonce` and `evolving_nonce` to eta0) — **BEFORE** the degenerate snapshot-at-target return AND before the
> replay-forward `block_validity` fold. So a block that validates during live admit (against the eta0-overlaid
> `chain_dep`, `T-REC-04`) MUST NOT fail rollback-materialize replay because materialization substituted a different
> nonce source; the persisted snapshot's `Nonce::ZERO` placeholder MUST NOT reach VRF verification on the
> rollback-replay path. Same recovered store + same ordered WAL/feed ⇒ same `chain_dep` inputs ⇒ same `block_validity`
> result on the live-admit and rollback paths. eta0 is the recovered canonical input (the seed-epoch
> `SeedEpochConsensusInputs.epoch_nonce` sidecar) — never peer data, wall-clock, CLI re-supply, or a re-query. **VRF
> strength is UNCHANGED on the rollback path: NO bypass / skip / loosening** — a block whose VRF verifies against
> NEITHER eta0 nor the snapshot nonce still fails closed (the `None` arm and the wrong-eta0 test prove the overlay,
> not a skip, is the fix). **SCOPE:** the recovered seed epoch (no epoch-boundary crossing within the follow window —
> eta0 is the constant epoch nonce); a multi-epoch rollback nonce-evolution is a named out-of-scope follow-on.

- **PHASE4-N-AN / AN-S1 / `T-REC-06` (repro-first).** `crates/ade_ledger/tests/wal_rollback_ai_s1.rs` (+1) and the
  `materialize.rs` test module reproduce the divergence mechanically: with the snapshot carrying the `Nonce::ZERO`
  placeholder, rollback replay of the rolled-back block fails the header VRF (`VrfCert`) — exactly the live
  reorg-follow death. (Commit `dbf31c7a`.)
- **PHASE4-N-AN / AN-S2 / `T-REC-06` (enforced) (BLUE overlay authority + threading).** The fix (commit `deaa6e28`):
  - **BLUE.** `ade_core::consensus::praos_state` (+15) adds `pub fn overlay_recovered_eta0(&mut self, eta0: &Nonce)` —
    the SINGLE eta0-overlay authority shared by BOTH WarmStart bootstrap and rollback materialization.
    `ade_ledger::rollback::materialize` (+140) adds the `recovered_eta0: Option<&Nonce>` param to
    `materialize_rolled_back_state` and applies the overlay before the degenerate return and the replay fold; +2 tests
    (`rollback_materialize_overlays_recovered_eta0_replay_equivalent` — `None ⇒ VrfCert`, `Some(eta0) ⇒ Valid` AND the
    materialized `epoch_nonce == eta0 ==` the live-admit nonce basis; `rollback_materialize_does_not_bypass_vrf_on_wrong_eta0`
    — a WRONG eta0 still fails the header VRF). `ade_ledger::receive::reducer` (+7) threads a
    `recovered_eta0: Option<&'a Nonce>` field through `RollbackContext` into the `roll_backward` materialize call.
  - **RED carriers.** `ade_runtime::forward_sync::reducer` (+9) adds `ForwardSyncState.recovered_eta0: Option<Nonce>`
    (set once at bootstrap, `None` default). `ade_node::node_lifecycle` (+12) sets `fwd.recovered_eta0` from
    `state.seed_epoch_consensus_inputs.epoch_nonce` alongside the recovered anchor, and threads
    `fwd.recovered_eta0.as_ref()` into the `apply_chain_event` rollback-follow materialize call (the path the CE-AI-6
    reorg hit). `ade_runtime::bootstrap` (+51) **reorders** the seed-epoch sidecar restore to run BEFORE
    `materialize_rolled_back_state` and passes its eta0 into materialize — so a WarmStart from a NON-bare store (the
    WAL carries post-anchor blocks) replays against eta0, not the placeholder (this **also** fixes warm-start replay
    from a non-bare store, the SAME root cause); the explicit post-materialize overlay (the
    `ci_check_warmstart_eta0_overlay.sh` site, `T-REC-04` / `DC-CINPUT-03`) is retained and is now idempotent after the
    materialize overlay. `crates/ade_runtime/tests/receive_rollback_integration.rs` (+4) updates the new-param call sites.
  - New gate `ci_check_rollback_materialize_eta0.sh`. **`T-REC-06 → enforced`.**

### Stale-gate triage + archive

- **Stale-gate triage (`89facbea`, CI-script + registry only, 0 source).** Three pre-existing red gates (red since the
  N-AI/N-AJ window, byte-unrelated to N-AM) classified stale + repaired: (1) `ci_check_admission_wire_pump_closure.sh`
  — Guard 4 refined so a `RollBackward` chain-sync reply emits its DISTINCT `AdmissionPeerEvent::RollBackward`
  (AI-S4a, "a rollback is NEVER a TipUpdate only") rather than `tip_update` (`DC-PUMP-02` `source`/`statement` updated;
  `strengthened_in += "PHASE4-N-AN"`); (2) `ci_check_node_path_fidelity.sh` — pinned-flag allow-list extended (with
  review) for two legitimately-added, path-PRESERVING flags `+--participant-venue` (N-AI) and
  `+--convergence-evidence-path` (N-AJ); the path-diverging private flags stay excluded; (3)
  `ci_check_lifecycle_owner_uses_bootstrap_initial_state.sh` — dropped `pipefail` (the `grep -q` SIGPIPE produced
  spurious false-"missing" results) and scoped the `materialize_rolled_back_state` forbid to EXCLUDE the
  `apply_chain_event` fn (the N-AI rollback-follow legitimately materializes the rolled-back state via the sole
  `CN-STORE-07` authority — that is fork-choice, NOT initial state).
- **Archive (`b8860b16`).** Moved `docs/clusters/PHASE4-N-AM/` and `docs/clusters/PHASE4-N-AN/` to
  `docs/clusters/completed/`. (The N-AL cluster docs were moved to `completed/` by the N-AL close `35a851b9` at the
  start of this span; the two renames in §3 are that move.)

**BLUE IS TOUCHED this span (unlike N-AL), but adds ZERO new canonical type** — `git diff e87e8a43..HEAD` over the BLUE
`core_paths` trees lists `ade_core/src/consensus/praos_state.rs`, `ade_ledger/src/receive/reducer.rs`,
`ade_ledger/src/rollback/materialize.rs`, and the `ade_ledger/tests/wal_rollback_ai_s1.rs` test, but **no `^+\s*(pub )?(struct|enum)`
line**: BLUE canonical types **462 → 462** (CODEMAP's BLUE-tree metric; 486 → 486 raw / 921 → 921 whole-tree). The new
surfaces are a BLUE **method** (`overlay_recovered_eta0`) and new **fields** on the existing `RollbackContext` (BLUE) and
`ForwardSyncState` (RED) — not new types. **No `RO-LIVE` rule flipped** — `RO-LIVE-01` stays operator-gated. The live
**CE-AM-LIVE** (152s keep-alive sustain) and **CE-AN-LIVE** (CE-AI-6 reorg capture) passes are recorded as
`enforced`-backing evidence for `DC-PUMP-03` / `T-REC-06`, NOT bounty/preprod claims (both transcripts OUTSIDE-REPO,
scrubbed in-repo notes only).

## 0. Headline

| Count | Baseline (`e87e8a43`) | HEAD (`b8860b16`) | Δ |
|---|---|---|---|
| CI gates (`ci/ci_check_*.sh`) | 159 | **161** | **+2** — `ci_check_keep_alive_wire_only.sh` (DC-PUMP-03 / N-AM) + `ci_check_rollback_materialize_eta0.sh` (T-REC-06 / N-AN), both **ADDED**. **3 modified in place** (`ci_check_admission_wire_pump_closure.sh`, `ci_check_node_path_fidelity.sh`, `ci_check_lifecycle_owner_uses_bootstrap_initial_state.sh` — the stale-gate triage). **0 removed** (`--diff-filter=D` over `ci/` is empty; `ls ci/ci_check_*.sh \| wc -l` = 159 → 161). |
| Registry rules (`docs/ade-invariant-registry.toml`) | 359 | **361** | **+2** — two NEW rules `DC-PUMP-03` + `T-REC-06`. **Zero removed** (`comm -23` of the sorted `id =` lists is empty). |
| Registry status (enforced / enforced_scaffolding / partial / declared) | 224 / 1 / 19 / 126 | **227 / 1 / 19 / 125** | **+3 enforced**, **−1 declared** (`enforced_scaffolding=1` and `partial=19` unchanged). Reconciliation: the two new rules close enforced (`DC-PUMP-03` + `T-REC-06`, +2 enforced — each was declared at its cluster doc then flipped at close, transiently moving `declared` +2 then −2), **and** the in-span **N-AL close commit `35a851b9`** flipped `DC-NODE-33` `declared → enforced` (+1 enforced, −1 declared — at the committed baseline `e87e8a43` it was still `declared`). Net: +3 enforced, −1 declared. |
| Registry strengthenings | — | **+1** | **`DC-PUMP-02 += "PHASE4-N-AN"`** (the stale-gate triage refined `DC-PUMP-02` so the `RollBackward` chain-sync reply emits its distinct `AdmissionPeerEvent::RollBackward` per AI-S4a; `strengthened_in` becomes `["PHASE4-N-M-FOLLOW", "PHASE4-N-AN"]`). The new rules cross-ref existing rules (`DC-PUMP-03` → `CN-PUMP-01` / `DC-PUMP-01` / `DC-PUMP-02` / `DC-NODE-30` / `CN-CONS-03`; `T-REC-06` → `T-REC-04` / `DC-CINPUT-03` / `CN-STORE-07` / `DC-NODE-27` / `DC-CONS-20`) but append to no other rule's `strengthened_in`. No rule weakened; no rule removed. |
| BLUE canonical types | 462 | **462** | **±0** — BLUE IS touched (N-AN's eta0 overlay in `praos_state` / `receive::reducer` / `rollback::materialize`), but **no new `pub struct`/`pub enum`** (`git diff e87e8a43..HEAD` over the BLUE trees has no `^+\s*(pub )?(struct\|enum)` line). CODEMAP's BLUE-tree metric **462 → 462**; raw BLUE grep **486 → 486**; whole-tree **921 → 921**. The new BLUE surface is a **method** (`PraosChainDepState::overlay_recovered_eta0`) + a **field** (`RollbackContext.recovered_eta0`), not a type. Still 11 crates (the only `Cargo.toml` touch is a `tokio` `test-util` **dev-dependency**, not a project feature flag). |
| Grounding docs | CODEMAP / SEAMS / TRACEABILITY all regenerated to **`e87e8a43`** by the in-span N-AL close `35a851b9` (462 canonical types / 159 CI / 359 rules; they carry `DC-NODE-33` — CODEMAP 19 / SEAMS 1 / TRACEABILITY 4 mentions) | Still pinned at **`e87e8a43`** — now **one window (N-AM + N-AN) stale**: none carries `DC-PUMP-03` or `T-REC-06` (`grep -c` in each = 0 for both). N-AM + N-AN add **NO new module and NO new canonical type**, so CODEMAP's module/type inventory (462 types / 11 crates) stays accurate; the owed refresh is TRACEABILITY's two new rows (`DC-PUMP-03` ↔ `ci_check_keep_alive_wire_only.sh`; `T-REC-06` ↔ `ci_check_rollback_materialize_eta0.sh`) + the `DC-PUMP-02` strengthening note + a HEAD-pin/count bump (159 → 161 CI, 359 → 361 rules) across all three. | **CODEMAP + SEAMS + TRACEABILITY are now ONE window STALE** (missing `DC-PUMP-03` + `T-REC-06`; CI count 159 vs. HEAD 161) — the registry holds both new rules + their gate bindings authoritatively at HEAD (**361 rules**); the refresh to `b8860b16` is a follow-on item this close. See the cross-reference warning at the end of §5. |

> **Grounding-doc state this close (load-bearing).** **CODEMAP, SEAMS, and TRACEABILITY were all regenerated to
> `e87e8a43` at the N-AL close `35a851b9`** (the in-span first commit), so they carry `DC-NODE-33` and pin to
> `e87e8a43` / 462 types / 159 CI / 359 rules. They are now **one window stale** (N-AM + N-AN): `grep -c DC-PUMP-03`
> and `grep -c T-REC-06` in all three are **0**, and their CI-count pins read **159** vs. HEAD's **161**. Because
> N-AM + N-AN introduce **no new module and no new canonical type**, CODEMAP's structural inventory (462 types / 11
> crates) is unaffected; the owed refresh is the two new four-cell rows in TRACEABILITY (`DC-PUMP-03` +
> `T-REC-06`, each with its new `ci_check_*` gate), the `DC-PUMP-02 += PHASE4-N-AN` strengthening note, and a
> HEAD-pin/count bump (159 → 161 CI, 359 → 361 rules) across all three. The invariant registry holds both new rules +
> their gate bindings authoritatively at HEAD (**361 rules**); the CODEMAP + SEAMS + TRACEABILITY refresh to
> `b8860b16` is the follow-on item this close (surfaced in §5).

The slice↔rule↔gate map for this window:

| Slice | Rule(s) | Gate | What shipped |
|---|---|---|---|
| **N-AL close** (`35a851b9`) | flip `DC-NODE-33 → enforced` (populates `tests`); CODEMAP/SEAMS/TRACEABILITY/HEAD_DELTAS regenerated to `e87e8a43` | — (no new gate) | **docs/registry only — 0 code.** Closed the N-AL cluster: flipped `DC-NODE-33` to `enforced` (the previously-uncommitted N-AL close-pass), regenerated all four grounding docs to `e87e8a43`, and archived the N-AL cluster/slice docs to `docs/clusters/completed/PHASE4-N-AL/`. Folded into this span because it sits inside `e87e8a43..HEAD`; it is **not** N-AM/N-AN work. |
| **N-AM declare** (`6894f96f` + `c1b9eee2`) | `DC-PUMP-03` **declared** | — | N-AM keep-alive sustain finding + invariants sketch + cluster authority doc; declares `DC-PUMP-03`. 0 code. |
| **AM-S1** (`a1655449`) | **`DC-PUMP-03`** (NEW) | `ci_check_keep_alive_wire_only.sh` (NEW) | **RED keep-alive client.** `run_admission_wire_pump` `tokio::select!` cadence sends proto-8 `MsgKeepAlive` ~20s; `handle_keep_alive` validates cookies via the reused BLUE `keep_alive_transition`; wire-only (no `AdmissionPeerEvent`); fail-closed `AdmissionWirePumpError::KeepAlive`. +3 tests. (`+285` in `wire_pump.rs`; `+4` dev-dep `tokio` `test-util`.) |
| **N-AM bank + close** (`c0430322` → `8b18fc8e`) | `DC-PUMP-03` enforced_scaffolding → **enforced** | — | Bank (CE-AM-LIVE open), then close on the CE-AM-LIVE 152s sustain pass (`DC-PUMP-03 → enforced`). |
| **N-AN declare** (`180292f6`) | `T-REC-06` **declared** | — | N-AN rollback-materialize eta0 replay-equivalence authority doc; declares `T-REC-06`. 0 code. |
| **AN-S1** (`dbf31c7a`) | `T-REC-06` (repro) | — | **Repro-first.** Reproduces the rollback-materialize eta0 `VrfCert` divergence mechanically. |
| **AN-S2** (`deaa6e28`) | **`T-REC-06`** (NEW, → enforced) | `ci_check_rollback_materialize_eta0.sh` (NEW) | **BLUE overlay authority + RED threading.** `PraosChainDepState::overlay_recovered_eta0` (new BLUE method); `materialize_rolled_back_state` gains `recovered_eta0` param + overlays before the replay fold; `RollbackContext` + `ForwardSyncState` gain `recovered_eta0`; `bootstrap.rs` reorders sidecar-restore before materialize; `node_lifecycle` threads eta0 into `apply_chain_event`. +2 BLUE tests. |
| **CE-AN-LIVE** (`4380d02e`) | `T-REC-06` (live-validated) | — | Records the CE-AN-LIVE PASS (CE-AI-6 reorg capture): Ade followed a live `RollBackward` slot regression 371→361, re-converged `agreed` @ 383 (our==peer hash), 0 diverged, 0 `VrfCert`. |
| **stale-gate triage** (`89facbea`) | `DC-PUMP-02 += PHASE4-N-AN` | 3 gates MODIFIED | CI-script + registry only; 0 source. Repaired 3 pre-existing red gates (see above). |
| **archive** (`b8860b16`) | — | — | docs-only; moved PHASE4-N-AM + PHASE4-N-AN to `docs/clusters/completed/`. |

The per-commit shape (the full verbatim log is §1):

| Commit | Kind | What it did | Code / CI / registry effect |
|--------|------|-------------|-----------------------------|
| `35a851b9` | (close) | Close PHASE4-N-AL — participant-path recovered-anchor rollback boundary (DC-NODE-33) | **0 code / 0 CI**; docs/registry: flipped `DC-NODE-33 → enforced` (populates `tests`), regenerated all four grounding docs to `e87e8a43`, archived N-AL cluster docs. Registry count unchanged (359). **N-AL, not N-AM/N-AN** |
| `6894f96f` | docs (phase4-n-am) | Record keep-alive sustain finding + invariants sketch | **0 code / 0 CI / 0 registry**; planning doc only |
| `c1b9eee2` | docs (phase4-n-am) | Keep-alive client authority; declare DC-PUMP-03 | **0 code / 0 CI**; registry: `DC-PUMP-03` declared; + N-AM cluster doc |
| `a1655449` | feat (phase4-n-am) | AM-S1 — wire-pump keep-alive client sustains the live follow (DC-PUMP-03) | **RED code** (`wire_pump.rs` +285) + 3 tests + dev-dep `tokio` `test-util` (+4); **+0 BLUE type**; **+0 module**; **+1 CI** (`ci_check_keep_alive_wire_only.sh`); registry: enforcement scaffolding |
| `c0430322` | (bank) | Bank PHASE4-N-AM — DC-PUMP-03 enforced_scaffolding; CE-AM-LIVE open | **0 code / 0 CI**; registry: `DC-PUMP-03 → enforced_scaffolding` (CE-AM-LIVE open) |
| `8b18fc8e` | (close) | Close PHASE4-N-AM — DC-PUMP-03 enforced (CE-AM-LIVE sustain pass) | **0 code / 0 CI**; registry: `DC-PUMP-03 → enforced` (152s sustain pass) |
| `180292f6` | docs (phase4-n-an) | Rollback-materialize eta0 replay-equivalence authority; declare T-REC-06 | **0 code / 0 CI**; registry: `T-REC-06` declared; + N-AN cluster doc |
| `dbf31c7a` | test (phase4-n-an) | AN-S1 — reproduce rollback-materialize eta0 divergence (repro-first) | **test code** (`wal_rollback_ai_s1.rs` +1, `materialize.rs` repro); **+0 BLUE type**; **+0 CI** |
| `deaa6e28` | feat (phase4-n-an) | AN-S2 — carry recovered eta0 into rollback materialization (T-REC-06 enforced) | **BLUE code** (`praos_state.rs` +15, `materialize.rs` +140, `receive/reducer.rs` +7) + **RED** (`bootstrap.rs` +51, `forward_sync/reducer.rs` +9, `node_lifecycle.rs` +12) + tests; **+0 BLUE type**; **+0 module**; **+1 CI** (`ci_check_rollback_materialize_eta0.sh`); registry: `T-REC-06 → enforced` |
| `4380d02e` | docs (phase4-n-an) | Record CE-AN-LIVE PASS — CE-AI-6 reorg capture (T-REC-06 live-validated) | **0 code / 0 CI / 0 registry-rule** (evidence note in `T-REC-06.open_obligation` + runbook) |
| `89facbea` | ci (gates) | Stale-gate triage — 3 pre-existing red gates classified stale + repaired | **0 source**; CI: 3 gates MODIFIED; registry: `DC-PUMP-02` `source`/`statement` refined + `strengthened_in += PHASE4-N-AN` |
| `b8860b16` | docs (clusters) | Archive PHASE4-N-AM + PHASE4-N-AN to completed/ | **0 code / 0 CI / 0 registry**; cluster-doc move only |

## 1. Commit Log (newest first)

| Hash | Type | Summary |
|------|------|---------|
| `b8860b16` | docs | docs(clusters): archive PHASE4-N-AM + PHASE4-N-AN to completed/ |
| `89facbea` | ci | ci(gates): stale-gate triage -- 3 pre-existing red gates classified stale + repaired |
| `4380d02e` | docs | docs(phase4-n-an): record CE-AN-LIVE PASS -- CE-AI-6 reorg capture (T-REC-06 live-validated) |
| `deaa6e28` | feat | feat(phase4-n-an): AN-S2 -- carry recovered eta0 into rollback materialization (T-REC-06 enforced) |
| `dbf31c7a` | test | test(phase4-n-an): AN-S1 -- reproduce rollback-materialize eta0 divergence (T-REC-06, repro-first) |
| `180292f6` | docs | docs(phase4-n-an): rollback-materialize eta0 replay-equivalence authority; declare T-REC-06 |
| `8b18fc8e` | (close) | Close PHASE4-N-AM -- DC-PUMP-03 enforced (CE-AM-LIVE sustain pass) |
| `c0430322` | (bank) | Bank PHASE4-N-AM -- DC-PUMP-03 enforced_scaffolding; CE-AM-LIVE open (CE-AI-6 venue) |
| `a1655449` | feat | feat(phase4-n-am): AM-S1 -- wire-pump keep-alive client sustains the live follow (DC-PUMP-03) |
| `c1b9eee2` | docs | docs(phase4-n-am): keep-alive client authority; declare DC-PUMP-03 |
| `6894f96f` | docs | docs(phase4-n-am): record keep-alive sustain finding + invariants sketch (CE-AI-6 prerequisite) |
| `35a851b9` | (close) | Close PHASE4-N-AL — participant-path recovered-anchor rollback boundary (DC-NODE-33) |

No merge commits in the span. **12 commits, zero unclassified.** Nine subjects carry an explicit conventional-commits
prefix (`feat(...)` ×2, `docs(...)` ×5, `test(...)` ×1, `ci(...)` ×1); the other three are project close/bank
commits without a prefix (`8b18fc8e` Close N-AM, `c0430322` Bank N-AM — the project's close/bank convention; and
`35a851b9` `Close PHASE4-N-AL …`, the PRIOR-window close, folded in because it sits inside `e87e8a43..HEAD`). The
substantive production code lands in the two `feat(...)` commits (`a1655449` AM-S1 RED keep-alive, `deaa6e28` AN-S2
BLUE+RED eta0 overlay) plus the `test(...)` AN-S1 repro (`dbf31c7a`); the `ci(...)` commit (`89facbea`) is CI-script +
registry only (0 source). **`35a851b9` is N-AL close work, not PHASE4-N-AM/AN** (docs/registry only — 0 code).
`35a851b9` landed 2026-06-10; everything else 2026-06-11.

> **Note (commit-attribution policy).** Per this repo's `CLAUDE.md` override (vibe-coded-node bounty
> trailer requirement), commits in this repo carry a `Co-Authored-By:` model-attribution trailer; that
> is an Ade-local override of the global no-AI-attribution rule and applies to **commit messages
> only**. It does not affect this doc's content.

## 2. New Modules

**No new modules this window.** `git diff --diff-filter=A --name-only e87e8a43..HEAD -- 'crates/**/*.rs'` is
**empty** — neither N-AM nor N-AN adds a new library module or test file (N-AM's 3 tests are added to the EXISTING
`crates/ade_runtime/src/admission/wire_pump.rs` `mod tests`; N-AN's 2 BLUE tests to the EXISTING `materialize.rs`
test module). There is **no new crate, no new workspace** (`git diff --diff-filter=A '**/Cargo.toml'` is empty; still
**11 crates**). The only manifest change is a **dev-dependency** addition (`tokio` `test-util` in
`crates/ade_runtime/Cargo.toml`) — see §4. All source changes are confined to **six existing files** (one RED for
N-AM, three BLUE + two RED for N-AN) plus their existing test modules (§3).

> **Cross-reference (CODEMAP) — no new module to register.** Because this window introduces no module and no canonical
> type, CODEMAP's module inventory (11 crates / 462 canonical types, regenerated to `e87e8a43` at the N-AL close
> `35a851b9`) remains structurally accurate at HEAD. `DC-PUMP-03` attaches to the EXISTING RED
> `ade_runtime::admission::wire_pump` module; `T-REC-06` attaches to the EXISTING BLUE `ade_ledger::rollback::materialize`
> + `ade_core::consensus::praos_state` modules (all already in CODEMAP) — only their rule↔enforcement bindings
> (TRACEABILITY) need the refresh (§5).

## 3. Modules Modified

Beyond the (zero) new modules (§2), **six existing source files** changed for the production work — one RED for
PHASE4-N-AM, three BLUE + two RED for PHASE4-N-AN — plus their existing test modules. (The remaining span churn is the
in-span N-AL close `35a851b9` regenerating the four grounding docs + archiving cluster docs, the N-AM/N-AN cluster
docs, the stale-gate triage's three CI scripts + the `DC-PUMP-02` registry edit, and the `b8860b16` archive — docs/CI
only.)

| Module | Color / scope | Key changes |
|--------|---------------|-------------|
| `ade_runtime::admission::wire_pump` (`wire_pump.rs` +285) | **RED** per-peer wire pump, additive | **PHASE4-N-AM (AM-S1, `a1655449`):** the N2N keep-alive CLIENT (`DC-PUMP-03`). New `const KEEP_ALIVE_CADENCE = 20s`; per-peer keep-alive state (`KeepAliveState`, monotonic `u16` cookie, `KeepAliveVersion`, a `tokio::time::interval` with `MissedTickBehavior::Delay`, first tick consumed); a `tokio::select!` over inbound `recv()` (byte-identical pre-AM behaviour) + the cadence `tick()` — the cadence arm sends `MsgKeepAlive` (Initiator) only while `ClientIdle`, via the EXISTING outbound `OutboundFrame` path, advancing the REUSED BLUE `keep_alive_transition`; new `handle_keep_alive` consumes `MsgResponseKeepAlive` (agency fixed `Server`) and validates the echoed cookie; the previously-silent `KeepAlive` arm of the `AcceptedMiniProtocol` match now calls it. New error variant `AdmissionWirePumpError::KeepAlive(KeepAliveError)` (fail-closed → drop the peer). **WIRE-ONLY** — no `AdmissionPeerEvent`; cookie is deterministic (no `rand`); cadence is wall-clock RED. +3 hermetic tests (`wire_pump_sends_keep_alive_on_quiescent_cadence` under `start_paused`, `…_response_validates_cookie_no_event`, `…_cookie_mismatch_fails_closed`). The BLUE `ade_network::keep_alive` grammar is REUSED, not redefined. |
| `ade_ledger::rollback::materialize` (`materialize.rs` +140) | **BLUE** sole rolled-back-state authority (`CN-STORE-07`), additive | **PHASE4-N-AN (AN-S2, `deaa6e28`):** `materialize_rolled_back_state` gains a `recovered_eta0: Option<&Nonce>` param and overlays it onto the nearest-snapshot `chain_dep` (via `overlay_recovered_eta0`) **BEFORE** the degenerate snapshot-at-target return AND the replay-forward `block_validity` fold, so rollback replay validates each block's header VRF against eta0, NOT the snapshot `Nonce::ZERO` placeholder (`T-REC-06`). `None` (cold-start / no sidecar) keeps the snapshot nonce as-is. **VRF strength UNCHANGED** — not a bypass. +2 tests (`rollback_materialize_overlays_recovered_eta0_replay_equivalent` — `None ⇒ VrfCert`, `Some(eta0) ⇒ Valid` + materialized `epoch_nonce == eta0`; `rollback_materialize_does_not_bypass_vrf_on_wrong_eta0` — wrong eta0 still fails VRF). |
| `ade_core::consensus::praos_state` (`praos_state.rs` +15) | **BLUE** Praos chain-dep state, additive | **PHASE4-N-AN (AN-S2, `deaa6e28`):** new `pub fn overlay_recovered_eta0(&mut self, eta0: &Nonce)` — the SINGLE eta0-overlay authority (sets both `epoch_nonce` and `evolving_nonce` to eta0), shared by BOTH WarmStart bootstrap (the live-admit `chain_dep`) and rollback materialization (the replay `chain_dep`), so live admit and rollback replay validate against the SAME nonce by construction. **New method, not a new type.** |
| `ade_ledger::receive::reducer` (`receive/reducer.rs` +7) | **BLUE** receive bridge, additive | **PHASE4-N-AN (AN-S2, `deaa6e28`):** `RollbackContext<'a>` gains a `recovered_eta0: Option<&'a Nonce>` **field**, threaded through `roll_backward` into the `materialize_rolled_back_state` call. `None` keeps the snapshot nonce as-is. **New field, not a new type.** |
| `ade_runtime::bootstrap` (`bootstrap.rs` +51) | **RED**/GREEN single bootstrap authority (`CN-NODE-01`), reorder + thread | **PHASE4-N-AN (AN-S2, `deaa6e28`):** **reorders** the seed-epoch consensus-input sidecar restore to run BEFORE `materialize_rolled_back_state` (the restore is independent of the materialize result — order only, not outcome) and passes its `epoch_nonce` into materialize, so a WarmStart from a NON-bare store replays against eta0 (this **also** fixes warm-start replay from a non-bare store — the SAME root cause as the live-rollback bug). The explicit post-materialize overlay (the `ci_check_warmstart_eta0_overlay.sh` site, `T-REC-04` / `DC-CINPUT-03`) is retained and now idempotent. |
| `ade_runtime::forward_sync::reducer` (`forward_sync/reducer.rs` +9) | **RED**/GREEN forward-sync lifecycle reducer, additive | **PHASE4-N-AN (AN-S2, `deaa6e28`):** `ForwardSyncState` gains a `recovered_eta0: Option<Nonce>` **field** (set once at bootstrap from `BootstrapState.seed_epoch_consensus_inputs`, threaded into the rollback-follow materialize; `None` default for cold-start / non-recover callers). **New field, not a new type.** |
| `ade_node::node_lifecycle` (`node_lifecycle.rs` +12) | **RED** `--mode node`, additive | **PHASE4-N-AN (AN-S2, `deaa6e28`):** sets `fwd.recovered_eta0` from `state.seed_epoch_consensus_inputs.epoch_nonce` alongside the recovered anchor (set once, never peer/CLI/wall-clock), and threads `fwd.recovered_eta0.as_ref()` into the `apply_chain_event` rollback-follow `materialize_rolled_back_state` call — the live rollback-follow path the CE-AI-6 reorg hit. |
| `ade_ledger::tests::wal_rollback_ai_s1` (`wal_rollback_ai_s1.rs` +1) · `ade_runtime::tests::receive_rollback_integration` (+4) | **test**, additive | **PHASE4-N-AN (AN-S1 `dbf31c7a` / AN-S2 `deaa6e28`):** AN-S1 repro touch + new-param call-site updates for the `recovered_eta0` threading. |

> **BLUE IS touched this span (load-bearing) — but no new canonical type.** Unlike the N-AL window (BLUE-empty), N-AN
> touches three BLUE files (`praos_state.rs`, `receive/reducer.rs`, `rollback/materialize.rs`). The new BLUE surface is
> a **method** (`overlay_recovered_eta0`) and a **field** (`RollbackContext.recovered_eta0`) — **`git diff e87e8a43..HEAD`
> over the BLUE trees has no `^+\s*(pub )?(struct\|enum)` line**, so BLUE canonical types are **462 → 462** (486 → 486
> raw / 921 → 921 whole-tree). The change strengthens replay-equivalence on the rollback path without loosening VRF
> verification (the `None`-overlay and wrong-eta0 tests prove the overlay is the fix, not a skip). N-AM is **RED-only**
> (the `wire_pump` shell) and reuses the BLUE `ade_network::keep_alive` grammar unchanged. |

## 4. Feature Flags

**No project feature-flag deltas.** Ade declares no `[features]` table in any workspace `Cargo.toml` at either ref
(`git grep '^\[features\]'` is empty at both `e87e8a43` and HEAD). **No `#[cfg(feature = …)]` gate was introduced**
(`git diff e87e8a43..HEAD -- 'crates/**/*.rs' | grep '^+.*cfg(feature'` is empty) and **no `compile_error!` coupling
was added.** The single `Cargo.toml` change is a **dev-dependency**, not a feature flag:

| Dependency | Module | Purpose | Status |
|------------|--------|---------|--------|
| `tokio` `test-util` feature (dev-dependency) | `ade_runtime` (`crates/ade_runtime/Cargo.toml`) | Deterministic virtual-time (`#[tokio::test(start_paused)]`) for the DC-PUMP-03 keep-alive cadence test (CE-AM-1). **Dev-only — never in production builds; cargo unifies it with the production `tokio` features for tests.** | **New** (dev-dependency) |

**No new CLI flag this span** — neither N-AM nor N-AN adds a runtime flag. N-AM's keep-alive cadence is a fixed
`const` (no flag); N-AN's eta0 source is the EXISTING recovered seed-epoch sidecar (`ForwardSyncState.recovered_eta0`,
populated by the recover path), so a follow over a store with no recovered sidecar (`recovered_eta0 == None`)
reproduces pre-AN behaviour verbatim. (The `--participant-venue` / `--convergence-evidence-path` flags that appear in
the `ci_check_node_path_fidelity.sh` allow-list edit this span are **pre-existing** N-AI/N-AJ flags — the triage only
extended the gate's pinned allow-list to recognize them; no new flag was added.)

## 5. CI Checks (159 → 161; +2 new, 3 modified, 0 removed)

Five CI scripts changed this span: **2 added**, **3 modified in place**, **0 removed** (`git diff --diff-filter=D`
over `ci/` is empty; `ls ci/ci_check_*.sh | wc -l` = **159 → 161**). The two new gates back the two new rules; the
three modified gates are the stale-gate triage (`89facbea`, CI + registry only).

### PHASE4-N-AM / PHASE4-N-AN enforcement (new gates)

| Check | Status | What it checks |
|-------|--------|----------------|
| `ci_check_keep_alive_wire_only.sh` | **New** (`DC-PUMP-03`, N-AM) | The wire-pump keep-alive client is WIRE-ONLY: `handle_keep_alive` is defined exactly once and drives the BLUE `keep_alive_transition` over a `decode_keep_alive_message` (consumes the peer's `MsgResponseKeepAlive`); its body constructs NO `AdmissionPeerEvent` (returns `Result<(), KeepAliveError>`, no event channel); the cadence tick enqueues an `OutboundFrame` on `AcceptedMiniProtocol::KeepAlive` via `encode_keep_alive_message` and synthesizes NO semantic event; the pump REUSES the BLUE `ade_network::keep_alive` grammar (does not redefine the state machine or message type). |
| `ci_check_rollback_materialize_eta0.sh` | **New** (`T-REC-06`, N-AN) | Rollback materialization preserves the recovered eta0: (A) the SINGLE overlay authority `PraosChainDepState::overlay_recovered_eta0` exists; (B) `materialize_rolled_back_state` takes the `recovered_eta0: Option<&Nonce>` param AND applies the overlay; (C) NO VRF bypass (the replay still runs `block_validity`; no skip/unchecked); (D) eta0 is sourced from the recovered sidecar (`ForwardSyncState.recovered_eta0` via `apply_chain_event`; bootstrap via the seed-epoch sidecar) — never peer/CLI/wall-clock; (E) the replay-equivalence + no-bypass regression tests exist; (F) `T-REC-06` is enforced in the registry. |

### Stale-gate triage (`89facbea`) — 3 modified, no new rule

| Check | Status | What changed |
|-------|--------|--------------|
| `ci_check_admission_wire_pump_closure.sh` | **Modified** (`DC-PUMP-02`) | Guard 4 refined: a `RollBackward` chain-sync reply must emit its DISTINCT `AdmissionPeerEvent::RollBackward` (AI-S4a — "a rollback is NEVER a TipUpdate only") rather than `tip_update`; the per-arm context window widened (10→20). The `DC-PUMP-02` invariant ("a closed authority event per reply") is preserved — the RollBackward reply's closed event is its own variant. (Registry: `DC-PUMP-02` `source`/`statement` updated + `strengthened_in += PHASE4-N-AN`.) |
| `ci_check_node_path_fidelity.sh` | **Modified** | Pinned-flag allow-list extended (with review) for two legitimately-added, path-PRESERVING flags: `+--participant-venue` (N-AI, the σ=0 participant role; the `--mode node` admit path is UNCHANGED) and `+--convergence-evidence-path` (N-AJ, an emit-only evidence sink). The path-diverging private flags (`--private-net`, `--from-genesis`, `--devnet`, `--rehearsal`) stay excluded. |
| `ci_check_lifecycle_owner_uses_bootstrap_initial_state.sh` | **Modified** | Dropped `pipefail` (the `grep -q` SIGPIPE produced spurious false-"missing" results on tokens that ARE present); scoped the `materialize_rolled_back_state` forbid to EXCLUDE the `apply_chain_event` fn — the N-AI rollback-follow legitimately materializes the rolled-back state via the sole `CN-STORE-07` authority (fork-choice, NOT initial state), so a PARALLEL/cold INITIAL-STATE materialize still trips while the rollback-follow is permitted (`CN-NODE-01`). |

> **Cross-reference (CODEMAP + SEAMS + TRACEABILITY) — ONE window stale this close; refresh owed.** The two new
> rule↔enforcement bindings (`DC-PUMP-03` ↔ `ci_check_keep_alive_wire_only.sh`; `T-REC-06` ↔
> `ci_check_rollback_materialize_eta0.sh`) and the `DC-PUMP-02 += PHASE4-N-AN` strengthening are recorded **in the
> registry at HEAD** (`docs/ade-invariant-registry.toml`, 361 rules). They are **NOT yet in TRACEABILITY, SEAMS, or
> CODEMAP**, all three of which were regenerated to the N-AL close `e87e8a43` (`grep -c DC-PUMP-03` and `grep -c
> T-REC-06` in each = 0; their CI-count pins read **159** vs. HEAD's **161**). **No gate is orphaned** — both new gates
> bind a registry rule, and all three modified gates bind their existing rules. **No new module / no new type**, so
> CODEMAP's structural inventory needs no change — only the two new TRACEABILITY rows, the `DC-PUMP-02` strengthening
> note, and the HEAD-pin/count bump (159 → 161 CI, 359 → 361 rules). **Action:** regenerate CODEMAP + SEAMS +
> TRACEABILITY to `b8860b16` as a follow-on this close so `DC-PUMP-03` + `T-REC-06` appear in TRACEABILITY with their
> named gates and all three docs pin to the N-AN HEAD; until then the registry is authoritative for the new bindings.

## 6. Canonical Type Registry Delta

**n/a — no separate canonical-type registry is configured** (`canonical_type_registry: null`);
canonical-type rules live inline in the invariant registry under family **T**. **This window added ZERO BLUE
canonical types:** `git diff e87e8a43..HEAD` over the BLUE `core_paths` trees touches three BLUE files (N-AN's eta0
overlay) but has **no `^+\s*(pub )?(struct|enum)` line** — the new BLUE surface is a **method**
(`PraosChainDepState::overlay_recovered_eta0`) and a **field** (`RollbackContext.recovered_eta0`), not a type. BLUE
`pub struct`/`pub enum` over the `core_paths` trees: **`486 → 486`** raw (CODEMAP's BLUE-tree metric **`462 → 462`**;
whole-tree **`921 → 921`**). **Zero BLUE canonical types added; zero removed.** The only `Cargo.toml` change is a
`tokio` `test-util` dev-dependency (still 11 crates).

## 7. Normative / Invariant Rule Delta (359 → 361; +2 rules, +1 strengthening, zero removals)

**Two rule IDs were added; zero removed** (`359 → 361`; `comm -23` of the sorted `id =` lists is empty — exactly two
additions `DC-PUMP-03` + `T-REC-06`, no removal). The status tally moves **224 → 227 enforced** and **126 → 125
declared** (`enforced_scaffolding = 1` and `partial = 19` unchanged). The +3-enforced / −1-declared reconciles as: the
two new rules close `enforced` (`DC-PUMP-03` + `T-REC-06`, +2 enforced — each was `declared` at its cluster doc, then
flipped at close), **and** the in-span **N-AL close commit `35a851b9`** flipped the pre-existing `DC-NODE-33`
`declared → enforced` (+1 enforced, −1 declared — at the committed baseline `e87e8a43` it was still `declared`, having
been declared by `f8275c55` in the prior window).

*(The configured `normative_docs` — the CE-79 tier-gate statement + addendum, the three contract docs, the CE-73
reclassification, and `CLAUDE.md` — were **not** changed this span: `git diff --name-only e87e8a43..HEAD` over those
paths is empty. The rule-count delta is entirely the invariant-registry change.)*

**New rules (`+2`, both enforced):**

| Rule | Family / Tier · Status | Statement (summary) |
|------|------------------------|---------------------|
| `DC-PUMP-03` | DC / `derived` · **enforced** · `introduced_in = "PHASE4-N-AM"` | **Wire-pump keep-alive client.** `run_admission_wire_pump` (the SOLE per-peer pump, `CN-PUMP-01`) runs the N2N keep-alive CLIENT (mini-protocol 8): on a cadence STRICTLY under the peer's ~97s keep-alive timeout it sends `KeepAliveMessage::KeepAlive(cookie)` (Initiator) via the EXISTING outbound path, advancing the REUSED BLUE `ade_network::keep_alive` machine (`ClientIdle → ServerHasAgency{cookie}`); the inbound `MsgResponseKeepAlive(cookie')` advances the SAME machine back to `ClientIdle`, validating `cookie' == cookie`. **WIRE-ONLY:** no canonical input, no WAL entry, NO `AdmissionPeerEvent` (the `DC-PUMP-01` emit-set stays unwidened) — never affects admission, the durable chain, fork-choice, the convergence-evidence vocabulary, replay-equivalence, or any BLUE state. **MUST NOT:** redefine the BLUE keep-alive grammar; use a cadence ≥ the peer timeout; send a new `MsgKeepAlive` while one is in flight (respect `ServerHasAgency`); block/starve/reorder chain-sync or block-fetch; dispatch a keep-alive frame as a chain-sync/block-fetch event; silently swallow a grammar violation (fail closed via `AdmissionWirePumpError::KeepAlive` — drop the peer); use `rand`/wall-clock for the cookie (monotonic `u16`); implement a keep-alive SERVER/responder (client only — CE-AM-LIVE-gated follow-on). With the client running, a live participant AND single-producer follow sustains past the ~97s deadline — the prerequisite that makes the CE-AI-6 induced-reorg convergence capture runnable. **SCOPE:** the keep-alive client ONLY; does NOT add multi-peer ChainSel, does NOT flip `CN-CONS-03`, does NOT broaden `CN-PUMP-01` / `DC-PUMP-01` / `DC-PUMP-02` (cross-refs, preserved). |
| `T-REC-06` | T / `true` · **enforced** · `introduced_in = "PHASE4-N-AN"` | **Rollback-materialization replay-equivalence.** A block that validates during live admit (against the eta0-overlaid `chain_dep`, `T-REC-04`) MUST NOT fail rollback-materialize replay because materialization substituted a different nonce source. `materialize_rolled_back_state` (the SOLE rolled-back-state authority, `CN-STORE-07`) MUST reconstruct the replay `chain_dep` with the SAME recovered eta0 (epoch nonce) the live-admit path uses (`praos_vrf_input(slot, eta0)`, `DC-CINPUT-03`); the persisted snapshot's placeholder / genesis `epoch_nonce` MUST NOT reach VRF verification on the rollback-replay path. Same recovered store + same ordered WAL/feed ⇒ same `chain_dep` inputs ⇒ same `block_validity` result on the live-admit and rollback paths. eta0 is the recovered canonical input (the seed-epoch `SeedEpochConsensusInputs.epoch_nonce` sidecar) — never peer data, wall-clock, CLI re-supply, or a re-query. **MUST NOT:** bypass / skip / loosen VRF validation on the rollback path (VRF strength UNCHANGED — a block whose VRF verifies against NEITHER eta0 nor the snapshot nonce still fails closed). **SCOPE:** the recovered seed epoch (no epoch-boundary crossing within the follow window — eta0 is the constant epoch nonce); a multi-epoch rollback nonce-evolution is a named out-of-scope follow-on. Surfaced by the CE-AI-6 reorg (the rollback-follow died at `ReplayFailedAt … VrfCert`); unblocks CE-AI-6. |

**Strengthenings (`strengthened_in += "PHASE4-N-AN"`) — 1:** `DC-PUMP-02` gained `PHASE4-N-AN` (its
`strengthened_in` becomes `["PHASE4-N-M-FOLLOW", "PHASE4-N-AN"]`) — the stale-gate triage refined it so the
`RollBackward` chain-sync reply emits its distinct `AdmissionPeerEvent::RollBackward` (AI-S4a) rather than a generic
`TipUpdate`; the "closed authority event per reply" invariant is preserved, not weakened. The two new rules cross-ref
existing rules (`DC-PUMP-03` → `CN-PUMP-01`/`DC-PUMP-01`/`DC-PUMP-02`/`DC-NODE-30`/`CN-CONS-03`; `T-REC-06` →
`T-REC-04`/`DC-CINPUT-03`/`CN-STORE-07`/`DC-NODE-27`/`DC-CONS-20`) but append to no other rule's `strengthened_in`.
**No rule was weakened.**

**No rule was removed (expected: 0).** The registry delta is **two new rules (`DC-PUMP-03` + `T-REC-06`, both
enforced), one strengthening (`DC-PUMP-02`), zero removals** — consistent with append-only registry discipline. **No
anomaly.** (The +3-enforced / −1-declared tally additionally reflects the `DC-NODE-33` `declared → enforced` flip
carried by the in-span N-AL close commit `35a851b9`, accounted above.)

## Honest residual (window scope)

PHASE4-N-AM + PHASE4-N-AN together **unblocked the CE-AI-6 induced-reorg convergence capture** — N-AM kept the live
link alive past the peer's keep-alive timeout, N-AN made the reorg-follow replay-equivalent. The honest residual:

- **N-AM is WIRE-ONLY.** The keep-alive client moves bytes and sustains the link; it produces no canonical input, no
  WAL entry, and NO `AdmissionPeerEvent` — it never affects admission, the durable chain, fork-choice, or any BLUE
  state. **CE-AM-LIVE** (2026-06-11, FROZEN c2-testnet relay, magic 42) is `enforced`-backing evidence: a bare-anchor
  recover caught up 5 admits → `agreement_verdict{agreed}` (exact hash) → then SUSTAINED the link **152s** (vs. the
  prior ~96s EOF): 7 `MsgKeepAlive` on the ~20s cadence, 7 `MsgResponseKeepAlive` cookies validated, 0 grammar
  failures, 0 `ShutdownPeer(KeepAlive)`. It is **NOT** a CE-AI-6 claim — no reorg occurred in the sustain run.
  Transcript OUTSIDE-REPO (scrubbed note only).
- **N-AN does NOT loosen VRF.** The eta0 overlay supplies the CORRECT recovered nonce to the rollback-replay path; it
  does not skip or weaken VRF verification (the `None`-overlay arm fails on `VrfCert`, and a WRONG eta0 still fails —
  both regression-tested). **CE-AN-LIVE** (2026-06-11, fresh hermetic 2-pool bridge venue, magic 42) is
  `enforced`-backing evidence for the CE-AI-6 reorg capture: a bare-anchor recover → induced peer reorg → Ade received
  the `RollBackward` and FOLLOWED it (strict slot REGRESSION admit 371 → 361) → re-converged to
  `agreement_verdict{agreed}` @ slot 383 with `our_hash == peer_hash` (the reorged tip) → 16 admits, 2 agreed, 0
  diverged, Ade ALIVE, **0 `VrfCert` / 0 `ReplayFailedAt`** — the eta0 overlay HELD through the live rollback (without
  AN-S2 Ade died here with `ReplayFailedAt … VrfCert`; this is the live proof of the fix). Transcript OUTSIDE-REPO
  (bridge-venue methodology stays internal; scrubbed note only).
- **No `RO-LIVE` flip.** Neither CE-AM-LIVE nor CE-AN-LIVE is a bounty/preprod claim. `RO-LIVE-01` stays
  operator-gated / partial; no `RO-LIVE` registry status changed this span. `CN-CONS-03` (full multi-peer ChainSel
  convergence) is **NOT** flipped — it stays `declared`; single-best-peer rollback-FOLLOW is the proven scope.
- **SCOPE limits (load-bearing).** N-AN covers the **recovered seed epoch** (no epoch-boundary crossing within the
  follow window — eta0 is the constant epoch nonce); a **multi-epoch rollback nonce-evolution** is a named
  out-of-scope follow-on. N-AM's keep-alive **responder** (Ade as keep-alive SERVER toward a peer running a keep-alive
  client) is a CE-AM-LIVE-gated follow-on (the live run showed the peer is the keep-alive server; Ade is the client).
- **BLUE touched, no new type.** `git diff e87e8a43..HEAD` over the BLUE trees touches `praos_state.rs` /
  `receive/reducer.rs` / `rollback/materialize.rs` (N-AN), but adds **no `pub struct`/`pub enum`** — BLUE canonical
  types **462 → 462** (486 → 486 raw / 921 → 921 whole-tree). The new BLUE surface is a method
  (`overlay_recovered_eta0`) + a field (`RollbackContext.recovered_eta0`). N-AM is RED-only.
- **A separate warm-start replay issue was OUT OF SCOPE but partially addressed.** N-AN's bootstrap reorder
  (`bootstrap.rs`) also fixes a WarmStart-from-a-non-bare-store replay-VRF failure (the SAME root cause). A distinct
  warm-start `ReplayFailedAt … VrfCert` symptom surfaced during the N-AM CE-AM-LIVE run (a caught-up store) was
  deferred at the N-AM close to its own investigation; N-AN's overlay is the structural fix for that root cause, and
  the explicit warm-start overlay (`T-REC-04`) is retained.
- **No new module, no new type, no new CLI flag.** The only manifest change is a `tokio` `test-util` dev-dependency.
  N-AM's cadence is a fixed `const`; N-AN's eta0 source is the EXISTING recovered sidecar (`None` reproduces pre-AN
  behaviour verbatim).
- **CODEMAP + SEAMS + TRACEABILITY refresh owed this close — now ONE window behind.** All three were regenerated to
  the N-AL close `e87e8a43` (the in-span `35a851b9`) and carry `DC-NODE-33` but lack `DC-PUMP-03` + `T-REC-06`
  (`grep -c` in each = 0 for both) and pin CI at **159** vs. HEAD's **161**. Because this window adds no module and no
  type, only TRACEABILITY's two new four-cell rows (each with its new `ci_check_*` gate), the `DC-PUMP-02 +=
  PHASE4-N-AN` strengthening note, and a HEAD-pin/count bump (159 → 161 CI, 359 → 361 rules) are owed. The registry
  holds both new rules + their bindings authoritatively at HEAD (361 rules) in the interim. Regenerating CODEMAP +
  SEAMS + TRACEABILITY to `b8860b16` is the named follow-on (surfaced in §5).
- **Two in-span commits are prior-window / hygiene.** `35a851b9` (`Close PHASE4-N-AL …`) is docs/registry only (0
  code) — it flipped `DC-NODE-33` to enforced and regenerated all four grounding docs to `e87e8a43`; it is N-AL work,
  recorded in §1/§0 for completeness. `89facbea` (the stale-gate triage) is CI-script + registry only (0 source) — it
  repaired three pre-existing red gates (red since the N-AI/N-AJ window) and refined `DC-PUMP-02`; it is not new
  cluster authority.

## Working tree at HEAD `b8860b16` (clean)

**The working tree is CLEAN at this regen** — `git status --porcelain` shows only untracked scratch
(`.mithril-scratch/`, `wire_smoke.jsonl`), neither part of this doc. The N-AM and N-AN closes (the `DC-PUMP-03` /
`T-REC-06` flips, the cluster-doc archive `b8860b16`) are all committed; the committed `.idd-config.json` baseline at
HEAD already reads `e87e8a43` (the prior N-AL regen's bump committed cleanly). §1 narrates the committed span
`e87e8a43..b8860b16` verbatim; §0/§7 read rule status from the registry at HEAD (`DC-PUMP-03` + `T-REC-06` enforced,
361 rules). **This regen performs the baseline bump** (`e87e8a43 → b8860b16` in `.idd-config.json`
`head_deltas_baseline`, with the `_head_deltas_baseline_doc` lead prepended for N-AM/N-AN and the N-AL paragraph
demoted to "PRIOR baseline"), per the task's post-close step. The remaining close-pass action is the CODEMAP + SEAMS +
TRACEABILITY refresh to `b8860b16` (surfaced in §5).

---

## Historical — PHASE4-N-AL participant recovered-anchor rollback no-op (`b4c0983d → e87e8a43`)

> The section below is the **previous** HEAD_DELTAS lead, preserved in condensed form. It narrated the
> `b4c0983d → e87e8a43` span (measured from the PHASE4-N-AK AK-S2 close `b4c0983d`): the **N-AK close commit**
> (`efa2a44e`, docs/registry only — flipped `DC-NODE-31`/`DC-NODE-32` `enforced_scaffolding → enforced`, regenerated
> all four grounding docs to `b4c0983d`, archived the N-AK cluster docs; registry stayed 358) + a **C2-guide
> remediation note** (`c3ec7466`, docs-only, records the N-AK recover→follow fix) + the **PHASE4-N-AL cluster** (single
> slice AL-S1). **4 commits, 14 files, +1792 / −825.** **This span did NOT touch BLUE** — `git diff b4c0983d..HEAD`
> over the BLUE `core_paths` trees was empty (456 → 456 BLUE / 462 → 462 whole-tree by the prior metric); **NO new
> crate (11), NO new module, NO new canonical type, NO new CI gate (159 → 159).** **AL-S1 / `DC-NODE-33`** (enforced):
> the participant MIRROR of N-AK's single-producer `DC-NODE-32` — `run_participant_sync`'s `RollBack` handler gains a
> 17-line recovered-anchor branch (`if slot == anchor.slot && hash == anchor.hash { continue; }`) evaluated AFTER the
> `RollBackward(Origin)` fail-close (AI-S4a) and BEFORE the `DC-NODE-29` `get_block_by_hash` stored-block resolution, so
> a peer `RollBackward` binding EXACTLY (slot AND hash) to the persisted recovered anchor (`ForwardSyncState.recovered_anchor`,
> AK-S2's carrier, reused unchanged) is an idempotent NO-OP — no `commit_rollback` / `WalEntry::RollBack` / ChainDb /
> ledger / chain_dep / cursor / `pending_reselection` mutation. Every non-anchor / non-Origin / slot-only / hash-only
> rollback still routes through the UNCHANGED `DC-NODE-29` authority. **`DC-NODE-32` NOT broadened** (stays scoped to
> `run_node_sync`; a distinct sibling). Registry **358 → 359** (+1 `DC-NODE-33`; **0 strengthenings**; 0 removals);
> CI gates **159 → 159** (`DC-NODE-33` `ci_script=""` — test-enforced by 5 `participant_*` tests in
> `live_fork_choice_ai_s4bii.rs`, matching the `DC-NODE-31`/`DC-NODE-32`/`DC-PROTO-10`/`T-REC-05` precedent). **At the
> N-AL close `35a851b9` (committed in the SUCCEEDING N-AM window) all four grounding docs were regenerated to
> `e87e8a43`.** Live **CE-AL-3-LIVE** (2026-06-10, FRESH 2-pool `cardano-testnet` venue, magic 42) PASSED end-to-end:
> bare-anchor recover @ slot 741 → `RollBackward(741)` idempotent no-op → first admit @ slot 777 →
> `agreement_verdict{agreed}` @ slot 801 (our_hash == peer_hash) → 0 `UnexpectedRollback` + 0 `UnsupportedRollbackPoint`
> + 0 diverged; transcript OUTSIDE-REPO. **NO `RO-LIVE` flip.** It did NOT prove CE-AI-6 reorg convergence or full
> ChainSel (CE-AI-6 is a SEPARATE induced-reorg operator pass — unblocked by the SUCCEEDING N-AM + N-AN window). The
> full §§0–7 narrative is recoverable from this doc's git history at `e87e8a43`.

---

## Historical — PHASE4-N-AK recovered-anchor live-follow start + rollback boundary (`b1bed361 → b4c0983d`)

> Preserved as a pointer. It narrated the `b1bed361 → b4c0983d` span (measured from the PHASE4-N-AJ close `b1bed361`):
> the **N-AJ close commit** (`bbdc3585`, docs/registry/config only — registry 354→356, `DC-NODE-30 → enforced` +
> `DC-EVIDENCE-03 → enforced_scaffolding`, baseline bump `e99a86c7 → b1bed361`) + the **PHASE4-N-AK cluster** (two
> slices AK-S1 + AK-S2) — a post-N-AH/N-AI/N-AJ live recover→follow regression remediation. **7 commits, 33 files,
> +2647 / −544.** **This span TOUCHED BLUE — +2 canonical types** (`456 → 458` by the prior metric): one NEW BLUE
> module `crates/ade_ledger/src/recovered_anchor_point.rs` shipping `RecoveredAnchorPoint` (the closed, version-gated,
> byte-canonical anchor-point record) + `RecoveredAnchorPointError` + the sole canonical CBOR codec
> (`RECOVERED_ANCHOR_POINT_SCHEMA_VERSION = 1`) + one NEW RED module `crates/ade_runtime/src/recovered_anchor.rs`
> (`load_recovered_anchor_point`, kept OUT of `bootstrap.rs` to preserve the `CN-NODE-01` single-`pub fn` closure).
> **AK-S1 / `DC-NODE-31`** (enforced): persist the bootstrap anchor POINT as fingerprint-bound recovery provenance +
> resolve the live-follow FindIntersect start from it (`resolve_live_follow_start`). **AK-S2 / `DC-NODE-32`**
> (enforced): the single-producer `run_node_sync` `RollBack` handler accepts `RollBackward(anchor)` (exact slot AND
> hash) as an idempotent no-op; new `ForwardSyncState.recovered_anchor` field. CI gates **159 → 159** (both rules
> `ci_script=""`). Registry **356 → 358** (+2; `T-REC-05` strengthened `+= PHASE4-N-AK`; 0 removed). Live **CE-AK-3**
> (frozen c2-relay) PASSED end-to-end. **NO `RO-LIVE` flip.** SCOPE was the single-producer `run_node_sync` path ONLY —
> the participant path was closed by **PHASE4-N-AL / `DC-NODE-33`**. The full §§0–7 narrative is recoverable from this
> doc's git history at `b4c0983d`.

---

## Historical — PHASE4-N-AJ Participant-path convergence evidence emission (`e99a86c7 → b1bed361`)

> Preserved as a pointer. It narrated the `e99a86c7 → b1bed361` span (measured from the PHASE4-N-AI close
> `e99a86c7`): the **N-AI baseline-bump chore** (`c1f4c876`) + **one unrelated docs commit** (`c95e2592`) + the
> **PHASE4-N-AJ cluster** — Participant-path convergence evidence emission, the CE-AI-6 bridge. **9 commits, 19 files,
> +1813 / −35.** **EVIDENCE-ONLY — ZERO BLUE change, 460 canonical types unchanged** (old whole-tree metric). It added
> a **deterministic GREEN evidence side-output** — emitting the EXISTING closed `AgreementVerdict` vocabulary
> (`block_received` / `block_admitted` / `agreement_verdict` via `verdict::derive`) to a dedicated
> `--convergence-evidence-path` JSONL sink (the new GREEN/RED module `ade_node::convergence_evidence`). CI gates
> **157 → 159** (+2). Registry **354 → 356** (+2: `DC-NODE-30` enforced + `DC-EVIDENCE-03` enforced_scaffolding;
> `DC-ADMIT-04` strengthened; **`CN-CONS-03` NOT flipped**; 0 removed). Headline: the live `--mode node
> --participant-venue` rollback-follow path now emits convergence EVIDENCE — **NOT authority**. **NO `RO-LIVE` flip.**
> *(The N-AJ close artifacts were committed by `bbdc3585`, the first commit of the SUCCEEDING N-AK window.)* The full
> §§0–7 narrative is recoverable from this doc's git history at `b1bed361`.

---

## Historical — PHASE4-N-AI live fork-choice rollback-follow wiring (`8e2c3672 → 5ec841c8` / close `e99a86c7`)

> Preserved as a pointer. It narrated the `8e2c3672 → 5ec841c8` span: the **N-AH baseline-bump chore**
> (`c66fa9a9`) + the **PHASE4-N-AI cluster** (live fork-choice rollback-follow wiring of the EXISTING
> `chain_selector` → BLUE `select_best_chain` into the live `--mode node` receive path — single-best-peer FOLLOW,
> NOT full ChainSel; `DC-NODE-23`…`DC-NODE-29`; close `5ec841c8`, docs/baseline `e99a86c7`) + one unrelated docs
> commit (`cbad2ae3`). **26 commits, 46 files, +5350 / −53.** **FIRST BLUE delta since G-N: +2 canonical types**
> (`458 → 460` by the old whole-tree metric — the `ade_ledger::wal::event::{RollbackPoint, RollbackReason}` payload
> types of the new closed-sum `WalEntry::RollBack` durable MARKER). CI gates **148 → 157** (+9; 0 modified, 0
> removed). Registry **347 → 354** (+7: `DC-NODE-23..29` enforced; `CN-CONS-01` flipped partial→enforced; 13
> strengthenings; 0 removed). Headline (honest boundary): Ade follows ONE peer's chain-sync `RollBackward` reorg
> end-to-end on a declared Participant venue — replay-equivalently and fail-closed. **Single-best-peer
> rollback-FOLLOW, NOT full multi-peer Cardano ChainSel.** **`CN-CONS-03` was NOT flipped.** The per-cluster
> security review found **H-1** (mixed peer/local rollback target → durable-chain truncation) → remediated by
> **AI-S6 / `DC-NODE-29`** (durable stored point as sole authority, validated pre-mutation, fail-closed) →
> re-review **H-1 CLOSED.** **NO `RO-LIVE` flip.** The full §§0–7 narrative is recoverable from this doc's git
> history at `5ec841c8` / `e99a86c7`.

---

## Historical — PHASE4-N-AG superseded + PHASE4-N-AH local-tip forge-base authority (`f87d0056 → 5858288e`)

> Preserved as a pointer. It narrated the `f87d0056 → 5858288e` span: the **PHASE4-N-AF close tail** (`600581e8`
> + `2d99cdf2`) + the **PHASE4-N-AG cluster** (single-producer loop-continuation-after-feed-EOF, `DC-NODE-19`;
> **superseded-close**) + the **PHASE4-N-AH cluster** (local selected durable chain forge-base authority
> `DC-NODE-20` + cert evidence-only `DC-NODE-21` + single-producer warm-start re-entry `DC-NODE-22`). **32 commits,
> 48 files, +5155 / −743.** **RED/GREEN-only — ZERO BLUE change, 458 → 458 (old metric).** CI gates **143 → 148**
> (+5; 3 modified in place; 0 removed). Registry **343 → 347** (+4; 9 strengthenings; 0 removed). Headline (honest
> boundary): Ade sustained **cert-free single-producer block production on C2-LOCAL** (`cardano-testnet` magic 42)
> against a real Haskell relay (`cardano-node 11.0.1`) — forged on its OWN local durable `ChainDb::tip`, crossed a
> follow-link EOF, settled `> k` immutable, and resumed forging after a hard restart (run-4). NOT preprod. NOT
> bounty completion. No `RO-LIVE` flip. The full §§0–7 narrative is recoverable from this doc's git history at
> `5858288e`.

---

## Historical — PHASE4-N-AF single-producer extend-own-durable-spine (`6363683e → f87d0056`)

> Preserved as a pointer. A **single-slice cluster lead** narrating the `6363683e → f87d0056` span: the
> PHASE4-N-AE.F close grounding-doc refresh (`d3f52e7c`) + a C2-guide doc (`1302417d`) followed by the **OQ-1 /
> DC-NODE-17 investigation** (`bd1a7a73` declared DC-NODE-17 → `dadf4743` live-disproved it as the fix) and the
> **PHASE4-N-AF cluster** (single slice AF.S1 — `DC-NODE-18`, single-producer extend-own-durable-spine). Counts
> at `f87d0056`: 343 rules, 143 CI gates, 458 canonical types (old metric). **GREEN+RED only — BLUE 458 → 458.**
> New gate `ci_check_single_producer_extend_own_spine.sh`. No `RO-LIVE` flip. The full §§0–7 narrative is
> recoverable from this doc's git history at `f87d0056`.

---

## Historical — PHASE4-N-AE.F post-CE-A5 echo-idempotency follow-up (`a76672b9 → 6363683e`)

> Preserved as a pointer. A **single-slice lead** narrating the `a76672b9 → 6363683e` span: the PHASE4-N-AE
> close grounding-doc refresh (`62811a4e`) followed by the **PHASE4-N-AE.F** slice (`DC-NODE-16` receive
> idempotency at the durable-admit chokepoint — a re-announced block Ade already durably holds (same hash, same
> slot) is an idempotent no-op at `pump_block`). Counts at `6363683e`: 341 rules, 142 CI gates, 458 canonical types
> (old metric). **RED chokepoint only — BLUE 458 → 458.** New gate `ci_check_receive_idempotency.sh`. No `RO-LIVE`
> flip. The full §§0–7 narrative is recoverable from this doc's git history at `6363683e`.

---

## Historical — earlier windows (`25ddeebd → a76672b9` and before)

> Preserved as pointers. The **PHASE4-N-AD/N-AE CE-A5 window** (`25ddeebd → a76672b9`, recover→serve continuity +
> forge-on-followed-tip admissibility — the CE-A5 manifest: a real `cardano-node 11.0.1` relay
> `AddedToCurrentChain` an Ade-forged successor block; `DC-NODE-14`/`DC-NODE-15`/`DC-CONS-24`/`DC-PROTO-10`; 336 →
> 340 rules at `a76672b9`); the **PHASE4-N-AC cluster** (KES signing evolves the operator key to the current period
> — `DC-CRYPTO-10`; 335 → 336 rules); the **PHASE4-N-AB cluster** (outbound mux segmentation — `CN-SESS-05`; 334 →
> 335 rules); the **PHASE4-N-AA cluster** (bounded peer-driven serve range — `DC-SERVEMEM-01`; 333 → 334 rules);
> the **PHASE4-N-U cluster + gate-hygiene tail** (forged-block durability — `DC-NODE-12`/`DC-CONS-23`/`DC-WAL-04`/
> `T-REC-05`/`DC-NODE-13`; 328 → 333 rules); and the **G-K…G-R + C1 multi-cluster catch-up** (`550eec3a →
> 65954fa3`, 319 → 328 rules, 126 → 134 CI gates). The full §§0–7 narrative for each is recoverable from this
> doc's git history at the respective HEADs.

---

## Generation notes

### Regen `e87e8a43 → b8860b16` (PHASE4-N-AM keep-alive client + PHASE4-N-AN rollback-materialize eta0 — current lead)

- **Baseline valid; two closed clusters + the prior-window close commit + a stale-gate triage + an archive.** Run
  against `e87e8a43` (the PHASE4-N-AL AL-S1 close, the prior HEAD_DELTAS HEAD), which `git rev-parse` resolves and
  `git merge-base e87e8a43 HEAD == e87e8a43` confirms is a strict ancestor of HEAD `b8860b16` (`e87e8a43` carries no
  tag). The committed `.idd-config.json` baseline at HEAD already reads `e87e8a43` (the prior N-AL regen's bump
  committed cleanly). This regen **performs the baseline bump** `e87e8a43 → b8860b16` as the task's post-close step.
- **Counts are mechanical (git/grep/ls):** commit log + `--shortstat` over `e87e8a43..HEAD` (**12** commits, no
  merges / **32** files / **+2288 / −516**); CI gate count via `git ls-tree -r --name-only <ref> ci/ | grep -c
  ci_check_.*\.sh` = **159** at baseline, **161** at HEAD (`--diff-filter=A` = 2 new, `--diff-filter=M` = 3,
  `--diff-filter=D` = empty); registry rule count via `grep -c '^id = '` at each ref (**359 → 361**; `comm -23` of the
  sorted `id =` lists is empty — exactly two additions `DC-PUMP-03` + `T-REC-06`, zero removals); registry status via
  `grep '^status = ' | sort | uniq -c` (baseline **224 / 1 / 19 / 126**, HEAD **227 / 1 / 19 / 125**); strengthening =
  **1** (`DC-PUMP-02 += PHASE4-N-AN`, the only `strengthened_in` line mentioning AM/AN in the registry diff); BLUE
  canonical types **462 → 462** (`git diff e87e8a43..HEAD` over the BLUE `core_paths` trees has no
  `^+\s*(pub )?(struct|enum)` line; raw BLUE grep 486 → 486 / whole-tree 921 → 921).
- **BLUE IS touched but adds no canonical type.** `git diff e87e8a43..HEAD` over the configured BLUE `core_paths`
  trees lists `ade_core/src/consensus/praos_state.rs`, `ade_ledger/src/receive/reducer.rs`,
  `ade_ledger/src/rollback/materialize.rs` (+ the `ade_ledger/tests/wal_rollback_ai_s1.rs` test) — N-AN's eta0 overlay
  — but no `^+\s*(pub )?(struct|enum)` line. The new BLUE surface is a method (`overlay_recovered_eta0`) + a field
  (`RollbackContext.recovered_eta0`), not a type. N-AM is RED-only (the `wire_pump` shell), reusing the BLUE
  `ade_network::keep_alive` grammar.
- **No new module.** `git diff --diff-filter=A --name-only e87e8a43..HEAD -- 'crates/**/*.rs'` is empty — no new
  library module and no new test file (N-AM's 3 tests + N-AN's 2 tests are added to EXISTING test modules). No new
  crate / workspace — still 11 crates.
- **Only manifest change is a dev-dependency, NOT a feature flag.** `git diff --name-only e87e8a43..HEAD --
  '**/Cargo.toml' 'Cargo.toml'` lists only `crates/ade_runtime/Cargo.toml`, a `tokio` `test-util` **dev-dependency**
  (for the `start_paused` virtual clock in the DC-PUMP-03 cadence test). No `[features]` table at either ref; no
  `cfg(feature)` gate or `compile_error!` added (both greps over the span are empty). No new CLI flag.
- **Production change is RED keep-alive (+285) + BLUE/RED eta0 threading.** The `crates/**/*.rs` files in the span:
  `admission/wire_pump.rs` (+285, AM-S1 RED keep-alive client + 3 tests), `praos_state.rs` (+15, AN-S2 BLUE overlay
  method), `rollback/materialize.rs` (+140, AN-S2 BLUE param/overlay + 2 tests), `receive/reducer.rs` (+7, AN-S2 BLUE
  field), `bootstrap.rs` (+51, AN-S2 RED reorder + thread), `forward_sync/reducer.rs` (+9, AN-S2 RED field),
  `node_lifecycle.rs` (+12, AN-S2 RED thread), + `wal_rollback_ai_s1.rs` (+1) / `receive_rollback_integration.rs`
  (+4) test touches. Everything else in the +2288/−516 is the in-span N-AL close `35a851b9` (four grounding docs +
  archive), the N-AM/N-AN cluster + planning docs, the stale-gate triage's three CI scripts + the `DC-PUMP-02`
  registry edit, and the `b8860b16` archive.
- **Registry delta is +2 rules + 1 strengthening, NOT a removal.** `DC-PUMP-03` (declared at `c1b9eee2`, enforced at
  the N-AM close `8b18fc8e` via `c0430322` enforced_scaffolding) + `T-REC-06` (declared at `180292f6`, enforced at
  AN-S2 `deaa6e28`). `DC-PUMP-02 += PHASE4-N-AN` (the stale-gate triage `89facbea`). The sorted-id `comm -23` confirms
  zero removals. The +3-enforced / −1-declared status tally additionally reflects the `DC-NODE-33` `declared →
  enforced` flip carried by the in-span **N-AL close commit** (`35a851b9`); at the COMMITTED baseline `e87e8a43`
  `DC-NODE-33` was still `declared`.
- **+2 CI gates, 3 modified, 0 removed.** New: `ci_check_keep_alive_wire_only.sh` (DC-PUMP-03) +
  `ci_check_rollback_materialize_eta0.sh` (T-REC-06). Modified (stale-gate triage, 0 source):
  `ci_check_admission_wire_pump_closure.sh` (DC-PUMP-02 RollBackward event refinement),
  `ci_check_node_path_fidelity.sh` (allow-list +`--participant-venue` +`--convergence-evidence-path`),
  `ci_check_lifecycle_owner_uses_bootstrap_initial_state.sh` (dropped pipefail + scoped the materialize check to
  exclude `apply_chain_event`).
- **No `RO-LIVE` flip; CE-AM-LIVE + CE-AN-LIVE are enforced-backing evidence.** `DC-PUMP-03` is `enforced` (hermetic
  CE-AM-1..3 + the CE-AM-LIVE 152s keep-alive sustain pass); `T-REC-06` is `enforced` (hermetic CE-AN-2..4 + the
  CE-AN-LIVE CE-AI-6 reorg capture: live `RollBackward` slot regression 371→361, re-converged `agreed` @ 383 exact
  hash, 0 diverged, 0 `VrfCert`). Both transcripts OUTSIDE-REPO (scrubbed notes only). NOT bounty/preprod claims; no
  `RO-LIVE` registry status changed (`RO-LIVE-01` stays operator-gated / partial).
- **Normative docs unchanged this span.** `git diff --name-only e87e8a43..HEAD` over the configured `normative_docs`
  (CE-79 statement + addendum, the three contract docs, CE-73 reclassification, `CLAUDE.md`) is empty — the §7 delta
  is entirely the invariant-registry change.
- **§1 commit log verbatim from `git log` (newest first).** The per-cluster synthesis is in §0/§3. Nine subjects carry
  a conventional-commits prefix (`feat`×2 / `docs`×5 / `test`×1 / `ci`×1); the other three are project close/bank
  commits (`8b18fc8e`, `c0430322`, and `35a851b9` `Close PHASE4-N-AL …`). `35a851b9` is **N-AL work, not N-AM/AN**
  (docs/registry only, 0 code); `89facbea` is CI-script + registry only (0 source).
- **Doc-refresh state — CODEMAP + SEAMS + TRACEABILITY now ONE window STALE (refresh owed).** All three were
  regenerated to the N-AL close `e87e8a43` by the in-span `35a851b9` and carry `DC-NODE-33` (CODEMAP 19 / SEAMS 1 /
  TRACEABILITY 4 mentions) but lack `DC-PUMP-03` + `T-REC-06` (`grep -c` in each = 0 for both) and pin CI at **159**
  vs. HEAD's **161**. This window adds **no module and no type**, so CODEMAP's structural inventory is unaffected —
  only TRACEABILITY's two new four-cell rows (each with its new `ci_check_*` gate), the `DC-PUMP-02 += PHASE4-N-AN`
  strengthening note, and a HEAD-pin/count bump (159 → 161 CI, 359 → 361 rules) across all three are owed.
  **Cross-reference warning surfaced in §5.** Regenerate CODEMAP + SEAMS + TRACEABILITY to `b8860b16` as a follow-on
  this close; the registry holds both new rules + their bindings authoritatively in the interim (361 rules). No orphan
  gate (both new gates bind a registry rule).
- **Working tree CLEAN.** This regen runs with all N-AM/N-AN close artifacts committed (`git status --porcelain` =
  untracked scratch only). **This regen performs the `.idd-config.json` baseline bump** `e87e8a43 → b8860b16` (the
  prior N-AL bump `b4c0983d → e87e8a43` committed cleanly; this regen advances it to `b8860b16`), per the task's
  post-close step. The remaining close-pass action is the CODEMAP + SEAMS + TRACEABILITY refresh to `b8860b16`.
