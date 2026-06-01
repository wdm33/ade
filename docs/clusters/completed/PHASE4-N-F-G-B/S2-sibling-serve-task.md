# PHASE4-N-F-G-B — Slice S2: Sibling served-chain admit task (handoff → push_atomic)

> **Status:** slice doc (IDD Part IV). Companion to `cluster.md` (S2 row + CE-G-B-2).
> Code-verified against HEAD `c518c357` (S1 merged).
>
> **Slice S2 in one line:** spawn a **sibling task off the relay spine** that consumes the S1
> `SelfAcceptedHandoff` over a **typed channel**, calls `into_accepted()`, and admits the BLUE
> `AcceptedBlock` to the served chain via the **single `ServedChainHandle::push_atomic`** authority —
> so the relay-loop body performs **no serve/tip mutation** (only a typed channel send) and the
> containment gate stays semantically unchanged. **No network serve, no block-fetch, no listener.**

## 1. Slice identity
- **Cluster:** PHASE4-N-F-G-B (self-accept→serve handoff). S1 = the fence (merged); **S2 = the admit
  half of the sibling serve task**; S3 = the network block-fetch serve + payload proof + handoff gate.
- **Slice:** S2 — sibling served-chain admit task: a dispatcher-spawned RED task drains the S1
  handoff channel and admits via `push_atomic`; the relay loop forwards the handoff by a typed
  `mpsc` send.
- **Modules:** **RED** dispatcher wiring in `node_lifecycle` (create `(ServedChainHandle,
  ServedChainView)` + `mpsc` channel + spawn the sibling task) and the `run_relay_loop` forge tick
  (send `Some(handoff)`); **RED reuse** `ServedChainHandle::push_atomic` and
  `SelfAcceptedHandoff::into_accepted`. **No BLUE change; no new `CoordinatorEvent` variant.**

## 2. Cluster Exit Criteria addressed (verbatim)
- **CE-G-B-2 (sibling serve task; containment unchanged)** — served-chain mutation only via
  `push_atomic`; relay-loop body byte-unchanged; candidate tests
  `sibling_serve_admits_via_push_atomic_only`, `relay_loop_body_unchanged_with_serve_sibling`;
  existing `ci_check_node_run_loop_containment.sh` byte-/semantically unchanged + green.

  *S2 contributes the **admit + containment half** of CE-G-B-2 (handoff → `push_atomic`; relay-loop
  containment semantically unchanged). The **network block-fetch serve** that responds to peers from
  the populated snapshot is S3 — the cluster plan has **S3 close CE-G-B-2** alongside CE-G-B-3.*

  *Wording note (S2 refinement): the loop diff adds a typed channel send, so "relay-loop body
  byte-unchanged" is read as **`ci_check_node_run_loop_containment.sh` script byte-unchanged; loop
  containment semantically unchanged** (the loop gains a typed channel send, not an authority/serve
  token). S2 finalizes the candidate test name as
  `relay_loop_containment_semantics_unchanged_with_serve_sibling`.*

(CE-G-B-1 = S1 fence (merged); CE-G-B-3 = S3 block-fetch payload + handoff gate — out of S2 scope.)

## 3. Intent (invariant impact)
Make **"the `--mode node` served chain was mutated by anything other than a BLUE self-accepted
artifact handed over by S1"** unrepresentable on the serve path. The served-chain serve-side
mutation (`push_atomic`) on the node spine is fed **only** by `SelfAcceptedHandoff::into_accepted()`
— the S1 carrier whose sole provenance is `self_accept` (CN-FORGE-01). The handoff travels from the
relay loop to a **sibling** task as a typed channel send (not a serve/tip token), so the relay-loop
remains authority-free (CN-NODE-02 / DC-NODE-05) and the containment gate is semantically unchanged.

## 4. Pre-conditions (verified at HEAD `c518c357`)
- **The S1 carrier + surfacing:** `forge_one_from_recovered` (node_sync.rs:428) returns
  `(CoordinatorEvent, Option<SelfAcceptedHandoff>)`; `SelfAcceptedHandoff::into_accepted(self) ->
  AcceptedBlock` (self_accepted_handoff.rs) is the consuming accessor. The relay-loop forge tick
  (node_lifecycle.rs:~699) today destructures `(outcome, _handoff)` and **drops** the handoff.
- **The single serve mutation authority:** `ServedChainHandle::push_atomic(accepted: AcceptedBlock)
  -> Result<ServedTip, PushError>` (served_chain_handle.rs:101); `ServedChainHandle::new() ->
  (handle, view)` (served_chain_handle.rs:86) — a `tokio::sync::watch` channel; `push_atomic` is the
  sole snapshot mutation (DQ-B1). `push_atomic` → BLUE `served_chain_admit` (only `AcceptedBlock`
  values enter the served index, CN-CONS-07).
- **The relay loop + dispatcher:** `run_relay_loop(..., forge: Option<&mut ForgeActivation>)`
  (node_lifecycle.rs:607); the dispatcher `run_node_lifecycle_inner` (node_lifecycle.rs:272) builds
  `ForgeActivation::new(...)` (:447) and calls `run_relay_loop(..., Some(&mut activation))` (:459) in
  the forge-on arm — the spawn point for a sibling task.
- **The serve-wiring mirror (produce_mode):** `ServedChainHandle::new()` (produce_mode.rs:294) +
  `run_n2n_listener` spawn (:263) — the producer-mode pattern S2's admit half borrows from (the
  listener/serve half is S3).
- **The containment gate:** `ci_check_node_run_loop_containment.sh` greps the `run_relay_loop` body
  and forbids `push_atomic` / `served_chain_admit` / `broadcast` / `block_fetch` / `OutboundCommand`
  (serve tokens) and a second `forge_one_from_recovered(`. A typed `mpsc` `send` is not in that list.

## 5. The fix (channel from the loop; admit in a sibling task)
1. **`ForgeActivation` gains a handoff sender (RED):** `handoff_tx: Option<mpsc::Sender<
   SelfAcceptedHandoff>>`. `None` ⇒ no send ⇒ byte-identical to the N-F-E/N-F-F relay (forge-capable,
   non-serving). `Some` ⇒ the forge tick forwards the surfaced handoff. The loop holds **only** this
   `Sender` — never a `ServedChainHandle`, never a `push_atomic` call.
2. **Relay-loop forge tick (RED, minimal):** where S1 dropped `_handoff`, S2 sends it —
   `if let (Some(tx), Some(h)) = (act.handoff_tx.as_ref(), handoff) { let _ = tx.send(h); }`. This is
   a **typed channel send**, not a serve/tip token; `hermetic_forge_outcomes.push(outcome)` and the
   no-tip/no-serve behavior are unchanged. **No `push_atomic` / `served_chain_admit` / `block_fetch`
   in the loop body** (containment gate green + script byte-unchanged).
3. **Dispatcher spawns the sibling admit task (RED):** in the forge-on arm, create
   `(handle, view) = ServedChainHandle::new()` and `(handoff_tx, handoff_rx) = mpsc::channel(..)`;
   pass `handoff_tx` into `ForgeActivation`; `tokio::spawn` a sibling task that drains `handoff_rx`
   and, per handoff, calls `handle.push_atomic(h.into_accepted())` — the **only** feed to
   `push_atomic` on this spine. `view` is created for the S3 network serve (unused this slice). The
   task is spawned by the **dispatcher**, never inside `run_relay_loop` (the loop holds only the
   `Sender`).

## 6. TCB color (execution boundary)
- **RED (new wiring):** the dispatcher's channel/handle creation + sibling-task spawn
  (`node_lifecycle`); the `ForgeActivation.handoff_tx` field + the loop's typed send. tokio task +
  `mpsc` are RED shell.
- **RED (reuse, no change):** `ServedChainHandle::push_atomic` (the single serve mutation),
  `SelfAcceptedHandoff::into_accepted`.
- **GREEN (consume only):** the S1 `SelfAcceptedHandoff` carrier (the channel payload).
- **BLUE (consume only):** `ade_ledger::producer::{served_chain_admit, AcceptedBlock}` (reached only
  through `push_atomic`). **No BLUE change.**

## 7. Invariants preserved (must not weaken) — by registry ID
- `CN-NODE-02` / `DC-SYNC-02` — the relay loop owns no authority and advances the durable tip only
  via `run_node_sync` → `pump_block`. A typed channel send is neither a tip advance nor a serve.
- `DC-NODE-05` — the forge tick stays self-accept-only, advances no durable tip, serves nothing; the
  forge-attempt sequence + forged bytes stay replay-byte-identical. `forge_one_from_recovered`
  appears exactly once in the loop.
- `CN-FORGE-01` — `self_accept` stays the sole `AcceptedBlock` producer; the admit feeds on the S1
  carrier's `into_accepted()`, adding no second producer and no raw-bytes path.
- `CN-CONS-07` — only `AcceptedBlock` enters the served chain (via `push_atomic` → `served_chain_admit`).
- The closed `CoordinatorEvent` surface — **no new variant, no `ForgeSucceeded` field**.
- `ci_check_node_run_loop_containment.sh` — **script byte-unchanged; loop containment semantically
  unchanged** (the loop body gains only a typed channel send — no serve/tip token).

## 8. Invariants strengthened (one family: node-spine serve admission is handoff-fed only)
**Family:** *on the `--mode node` spine the served-chain serve-side mutation (`push_atomic`) is fed
**only** by the S1 `SelfAcceptedHandoff` via `into_accepted()`, in a sibling task off the relay
spine, with the relay-loop containment gate semantically unchanged.*
- `DC-NODE-06` — S2 **wires the serve-side admit clause** ("served-chain mutation happens ONLY in
  the sibling serve authority via the single `ServedChainHandle::push_atomic`" + "the handoff … is a
  typed channel send … so the containment gate stays semantically unchanged"). **No registry status
  flip in this slice** — `DC-NODE-06` flips `declared`→`enforced` at G-B close when CE-G-B-1..3 are
  all green (the S1 per-slice pattern).
- `CN-PROD-04` — the `push_atomic` serve-side authority, previously wired only in produce_mode
  (BroadcastBlock arm), is now exercised on the node spine fed by the handoff. `strengthened_in +=
  "PHASE4-N-F-G-B"` is recorded **at G-B close**, not here.

## 9. Slice-entry decisions (settled)
- **D-1 — handoff transport (DECIDED: typed `tokio::sync::mpsc` channel).** The relay loop sends
  `SelfAcceptedHandoff` over `mpsc`; the sibling task receives + admits. A `send` is not a serve/tip
  token, so containment holds. **Rejected:** giving the loop a `ServedChainHandle` and calling
  `push_atomic` inline — that puts a serve token in the loop body (containment violation + the user
  hard line "the loop must never hold `ServedChainHandle` or call `push_atomic`").
- **D-2 — spawn site (DECIDED: the dispatcher, not the loop).** `run_node_lifecycle_inner` creates
  the channel + handle/view and `tokio::spawn`s the sibling task; the loop receives only the
  `Sender` (in `ForgeActivation`). Mirrors how setup/bootstrap happens before the loop (CN-NODE-02).
- **D-3 — the only `push_atomic` feed (DECIDED: `into_accepted()`).** The channel payload type is
  `SelfAcceptedHandoff`; `into_accepted()` is the only way to obtain the `AcceptedBlock`. No raw
  bytes, no second source, no direct `AcceptedBlock` injection on this spine.
- **D-4 — relay-only/forge-only unchanged (DECIDED: `Option` sender).** `handoff_tx = None` ⇒ no
  send ⇒ the N-F-D/E/F relay + forge-capable behavior is byte-identical (no serve sibling).

## 10. Replay / determinism obligations
The served snapshot is a serve-side index, not the durable tip — `push_atomic` → `served_chain_admit`
is deterministic in the admitted `AcceptedBlock` (DQ-B1; `served_chain_fingerprint_replay_byte_identical`
holds). Same handoff sequence ⇒ same served snapshot fingerprint. The relay loop's durable-tip /
forge replay-equivalence (`relay_loop_two_clean_runs_byte_identical`,
`relay_loop_forge_two_runs_byte_identical`) is preserved — the channel send does not enter the
tip/forge result. No new canonical type, no WAL/checkpoint change, no new corpus entry.

## 11. Replay / crash / epoch validation (tests by name)
- **New:**
  - `sibling_serve_admits_via_push_atomic_only` — a `SelfAcceptedHandoff` fed to the sibling task is
    admitted to the served snapshot **via `push_atomic`** (present by `(slot, hash)`), and the served
    chain is mutated by **no other** path.
  - `serve_sibling_push_atomic_fed_only_by_into_accepted` — the node-spine `push_atomic` call site
    takes exactly `handoff.into_accepted()` (type-level: the channel carries `SelfAcceptedHandoff`;
    no raw-bytes / direct-`AcceptedBlock` feed exists on this spine).
  - `relay_loop_containment_semantics_unchanged_with_serve_sibling` — with the handoff sender wired,
    the relay loop's **authority semantics** are unchanged: the forge tick stays self-accept-only and
    advances no durable tip (same `hermetic_forge_outcomes` as the N-F-F baseline), and the loop body
    holds no serve/tip token — only a typed channel send. (The code diff adds the send; the authority
    semantics do not change.)
  - `serve_sibling_admission_replay_byte_identical` — the same handoff sequence yields a
    byte-identical served-snapshot fingerprint across two runs.
- **Preserved:** `relay_loop_forge_tick_attempts_forge_advances_no_tip`,
  `relay_loop_forge_two_runs_byte_identical`, `relay_loop_without_producer_material_matches_nfd_relay`
  (DC-NODE-05); `broadcast_pushes_self_accepted_block_to_served`,
  `broadcast_rejects_non_self_accepted_block` (CN-PROD-04, produce_mode — unaffected); the S1 carrier
  + surfacing tests.
- **No network/block-fetch test here** — that is S3.

## 12. Mechanical acceptance criteria
- [ ] `cargo test -p ade_node` — the four S2 tests green; the §11 preserved tests still green.
- [ ] `ci_check_node_run_loop_containment.sh` green + **script byte-unchanged; loop containment
      semantically unchanged** (loop body holds only a typed channel send; no
      `push_atomic`/`served_chain_admit`/`broadcast`/`block_fetch` token; one `forge_one_from_recovered(`).
- [ ] `ci_check_no_independent_forge_codepath.sh` green (no new forge path).
- [ ] `grep` proof: on the node spine, `push_atomic(` is fed **only** by `…into_accepted()` (no
      raw-bytes / second-source feed); only one `ServedChainHandle::push_atomic` admit site on the
      node spine; the `run_relay_loop` body references no `ServedChainHandle`.
- [ ] `cargo build` + `cargo clippy` clean on `ade_node` + `ade_runtime`; `rustfmt` on changed files
      only (no workspace `cargo fmt -p`).
- [ ] No new `CoordinatorEvent` variant / `ForgeSucceeded` field; no relay-loop serve/tip token
      (diff inspection + containment gate).
- [ ] Acceptance scoped to `ade_node` + `ade_runtime` (consumed `ade_ledger`) — not the full
      `ade_testkit` corpus lane.

## 13. Failure modes
All **fail-closed**:
- `push_atomic` returns `PushError` (unreachable for a real self-accepted token; `AdmitError` needs a
  decode failure / hash collision) → the served snapshot is **not** mutated; the sibling task
  surfaces the structured error; no panic, no partial admit (DQ-B1 atomic `send_modify`).
- Channel closed (relay loop ended / shutdown) → the sibling task drains remaining handoffs and exits
  cleanly; no panic, no orphaned admit.
- A non-`ForgeSucceeded` tick → `None` handoff → nothing sent → nothing admitted (the S1 fence).

## 14. Hard prohibitions (inherits the cluster "Forbidden during this cluster" list)
- **No relax of `ci_check_node_run_loop_containment.sh`** (script byte-unchanged; loop containment
  semantically unchanged); **no serve / admit / gossip / block-fetch / durable-tip token in the
  `run_relay_loop` body** — only a typed channel send. The relay loop **must never hold a
  `ServedChainHandle` or call `push_atomic`**.
- **Only `SelfAcceptedHandoff::into_accepted()` may feed `push_atomic` on the node spine; no raw
  bytes, direct `AcceptedBlock`, failed forge outcome, self-declared flag, or peer verdict may feed
  serve admission.**
- **No second served-chain mutation authority** (only `ServedChainHandle::push_atomic`).
- **No live feed / `WirePump` / `n2n_dialer` / peer evidence / BA-02 / RO-LIVE claim.**
- **No new `CoordinatorEvent` variant / `ForgeSucceeded` field.**
- No new **BLUE authority / canonical type / WAL / checkpoint**.
- **Hard line:** if the admit needs a BLUE change, a containment relaxation, a second `push_atomic`
  feed, or live wiring — **stop and re-scope.**

## 15. Explicit non-goals
- **No network block-fetch serve** — `n2n_listener` + `producer_block_fetch_serve` over the
  `ServedChainView`, the loopback `RequestRange → MsgBlock` round-trip, the CN-WIRE-08 tag-24 payload
  proof, and `ci_check_served_chain_handoff_fence.sh` are **S3** (which closes CE-G-B-2 + addresses
  CE-G-B-3).
- No live feed / operator pass / peer acceptance (G-C, operator-gated).
- No `DC-NODE-06` registry status flip (G-B close).

## 16. Completion checklist
- [ ] `ForgeActivation.handoff_tx` added (`Option` sender); the forge tick sends `Some(handoff)`; the
      dispatcher creates the channel + `ServedChainHandle` and spawns the sibling admit task that
      drains the channel and calls `push_atomic(h.into_accepted())`; `None` ⇒ byte-identical relay.
- [ ] All §12 tests green; containment + no-independent-forge gates green & unchanged; `clippy` clean;
      changed files rustfmt'd.
- [ ] Slice doc committed standalone (`docs:`) before implementation; impl committed
      (`feat:`/`test:`) after green, model-attribution trailer. **No registry edit** (DC-NODE-06 flip
      + CN-PROD-04 strengthening deferred to G-B close).

## Authority
Registry IDs `DC-NODE-06` (wires the serve-side admit clause; registry flip **deferred to G-B
close**); `CN-PROD-04` (node-spine `push_atomic` serve authority; `strengthened_in` recorded at
close); `CN-NODE-02` / `DC-SYNC-02` / `DC-NODE-05` / `CN-FORGE-01` / `CN-CONS-07` (preserved). The
cluster doc `cluster.md` and `docs/ade-invariant-registry.toml` are authoritative; this slice doc
refines, it does not override.
