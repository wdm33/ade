# Ade — HEAD Deltas (Changes Since Baseline)

> **Status:** Living architectural document. Regenerated; not hand-edited.
>
> Regenerate with `/head-deltas <baseline>` after every cluster close. Baseline is recorded in `.idd-config.json` `head_deltas_baseline`.

> Baseline: `80dac1f7` (PHASE4-N-F-G-A S4 — fail closed on off-epoch forge before leadership, 2026-06-01 17:12)
> HEAD: `339cccb1` (here-strings in served-chain handoff fence to avoid pipefail SIGPIPE false-negative, 2026-06-01 22:33)
> Cluster: **PHASE4-N-F-G-B — self-accept → serve handoff on the `--mode node` relay spine**, slice span closed; close-pass commit to follow.
> 10 commits (no merges), 26 files changed, +3391 / -1600 lines.

This window narrates the **PHASE4-N-F-G-B cluster** — the second of three planned
PHASE4-N-F-G sub-clusters (G-A forge fidelity / **G-B self-accept→serve handoff** /
G-C live operator serve). G-B **surfaces the BLUE self-accepted forged artifact that
the relay-spine forge tick already minted (and previously discarded) and routes it,
through a typed constructor-fenced carrier, into a sibling served-chain admit task**
fed by the single `ServedChainHandle::push_atomic` authority. The forge tick now
returns a typed `Option<SelfAcceptedHandoff>` alongside its `CoordinatorEvent`
(`Some` iff `ForgeSucceeded`); the relay loop forwards it over a typed mpsc channel;
the dispatcher-spawned sibling task is the **sole node-spine `push_atomic` site** and
ingests **only** `SelfAcceptedHandoff::into_accepted()`. **NO BLUE crate changed (456
canonical types, Δ0); no new `CoordinatorEvent` variant or field — the token rides a
sibling return component; the relay-loop body performs no serve / admit / gossip /
block-fetch / durable-tip mutation, so the N-F-E containment gate is semantically
unchanged.** Peer ACCEPT is still proven only by the peer's validation log
(RO-LIVE-06), never by Ade's self-accept / `ForgeSucceeded` / any wire-success signal.
Live serve / operator-peer ACCEPT / BA-02 / RO-LIVE remain the gated G-C follow-on.

The span also carries the **PHASE4-N-F-G-A close tail** (`62cb8718` + `1806584c`) —
docs/registry only — which lands inside this window because the baseline `80dac1f7` is
the N-F-G-A *slice-span* HEAD, not its close commit (see §1).

## 0. Headline

| Count | Baseline (`80dac1f7`) | HEAD (`339cccb1`) | Δ |
|---|---|---|---|
| CI gates (`ci/ci_check_*.sh`) | 116 | **117** | **+1 new** (`served_chain_handoff_fence`); none removed |
| Registry rules | 313 | 313 | **0** (no new ID; `DC-NODE-06` already existed as the G-B forward sketch — it flips `declared → enforced` at the *pending* close-pass, not at this HEAD) |
| Test attributes (`#[test]`/`#[tokio::test]`, workspace) | 2213 | **2223** | **+10** (G-B S1/S2/S3 surface, across `self_accepted_handoff`/`node_sync`/`forge_succeeds`/`forge_handler_variants`/`produce_loopback`) |
| BLUE canonical types | 456 | 456 | **0 — NO BLUE change** (all code lands in RED `ade_runtime` / RED `ade_node`) |

> **Registry note (load-bearing — read before §7).** At HEAD `339cccb1` the committed
> registry is in **slice-span state**: `DC-NODE-06` (the G-B primary invariant) sits at
> `status = "declared"` (`tests = []`, `ci_script = ""`, `introduced_in =
> "PHASE4-N-F-G-B"`, `strengthened_in = []`), and **neither** `CN-PROD-04` **nor**
> `CN-CONS-07` (the two rules the cluster composes on) carries a `strengthened_in +=
> "PHASE4-N-F-G-B"` token yet. The not-yet-made **close-pass commit** flips `DC-NODE-06`
> `declared → enforced` (populating `tests` + `ci_script = "ci/ci_check_served_chain_handoff_fence.sh"`
> with the S1/S2/S3 tests + the S3 gate) and records the **2 cross-slice strengthenings**
> (`CN-PROD-04`, `CN-CONS-07`) — exactly as the N-F-G-A close-pass flipped `DC-EPOCH-03`
> after its slice span (see the prior baseline window). This doc narrates **what is
> committed at `339cccb1`**; the close-pass follows. The committed registry and the
> committed CODEMAP / SEAMS / TRACEABILITY **agree** at this HEAD that `DC-NODE-06` is
> `declared` (those three docs are still at the **G-A** close — see the staleness note in
> the generation section); this HEAD_DELTAS regen is the first G-B grounding-doc refresh,
> and the close-pass regenerates the other three together with the registry flip.
>
> **G-A tail already reconciled.** Unlike `DC-NODE-06`, the two **G-A** registry edits
> (`62cb8718` `DC-EPOCH-03` `declared → enforced` + the 7 G-A strengthenings; `1806584c`
> binds the four G-A gates to their `ci_script` slots) **are** committed in this span —
> they are the N-F-G-A close-pass that the previous HEAD_DELTAS window flagged as owed.
> The rule count is unchanged at 313 because both `DC-EPOCH-03` (G-A) and `DC-NODE-06`
> (G-B) IDs already existed at the baseline as forward sketches from the shared
> PHASE4-N-F-G invariant pass.

## 1. Commit Log (newest first)

| Hash | Type | Summary |
|------|------|---------|
| `339cccb1` | fix(ci) | here-strings in served-chain handoff fence to avoid pipefail SIGPIPE false-negative (PHASE4-N-F-G-B S3) |
| `738dddd1` | test | prove node-spine block-fetch serves self-accepted bytes + add handoff fence gate (PHASE4-N-F-G-B S3) |
| `69476831` | docs | specify PHASE4-N-F-G-B S3 block-fetch payload proof |
| `a5af52e8` | feat | wire sibling served-chain admit task fed by self-accepted handoff (PHASE4-N-F-G-B S2) |
| `2dfa296e` | docs | specify PHASE4-N-F-G-B S2 sibling admit task |
| `c518c357` | feat | surface self-accepted forge token via typed handoff fence (PHASE4-N-F-G-B S1) |
| `5e96ec74` | docs | specify PHASE4-N-F-G-B S1 self-accepted handoff fence |
| `d3478e6f` | docs | define PHASE4-N-F-G-B serve handoff cluster |
| `1806584c` | docs | bind G-A gates to registry traceability |
| `62cb8718` | docs | close PHASE4-N-F-G-A forge fidelity |

No merge commits in the span.

The first two commits (newest-listed-last) are the **PHASE4-N-F-G-A close tail** that
lands inside this window because the baseline `80dac1f7` is the N-F-G-A *slice-span*
HEAD, not its close commit: `62cb8718` is the N-F-G-A close-pass (registry
`DC-EPOCH-03` `declared → enforced` + 7 strengthenings + grounding-doc refresh +
cluster-doc archive), and `1806584c` is the G-A gate-binding follow-up (the four G-A
gates bound to their registry `ci_script` slots + a TRACEABILITY regen; the registry
stays byte-stable at 313 rules). Both are **docs/registry only** — no source change.
`d3478e6f` defines the G-B cluster; everything from `5e96ec74` onward is G-B proper
(S1 → S3, doc-then-impl per slice).

(Plus the pending close-pass commit: grounding-doc refresh + `DC-NODE-06`
`declared → enforced` flip with 2 strengthenings + `.idd-config.json` baseline bump
`80dac1f7 → 339cccb1` + cluster-doc archive + this HEAD_DELTAS.)

## 2. New Modules

One new source module — a GREEN constructor-fenced carrier in the RED `ade_runtime`
crate. No new crate, no new BLUE authority, no new WAL/checkpoint/canonical type, no
new `CoordinatorEvent` variant.

| Module | Color | Purpose | Key sub-paths | Added in |
|--------|-------|---------|---------------|----------|
| `ade_runtime::producer::self_accepted_handoff` | **GREEN** | The typed, constructor-fenced carrier that moves a BLUE self-accepted forged block (`ade_ledger::producer::AcceptedBlock`) from the relay-spine forge tick toward the sibling served-chain admit task. Its **sole** constructor, `SelfAcceptedHandoff::from_self_accepted(AcceptedBlock)`, takes a token producible only by BLUE `self_accept` returning `Ok` (CN-FORGE-01) — there is **no** constructor from raw `Vec<u8>`, from a `ForgedBlockArtifact` (re-deriving the token from `artifact.bytes` would breach CN-FORGE-01; the carrier holds the **original** token), from a `CoordinatorEvent`, from a self-declared acceptance flag, or from a peer verdict. "Hand a not-BLUE-self-accepted artifact to the serve task" is therefore type-unrepresentable. This is the GREEN fence backing **DC-NODE-06**. | `SelfAcceptedHandoff` (private `accepted: AcceptedBlock` field); `from_self_accepted` (sole ctor); `accepted(&self) -> &AcceptedBlock`; `into_accepted(self) -> AcceptedBlock` (the value consumed by `ServedChainHandle::push_atomic` in S2). Pure: no I/O, clock, rand, or float; the carrier wraps the token verbatim and never re-validates it (same `AcceptedBlock` ⇒ same carrier). `#[cfg(test)]` surface-pinning tests assert the sole constructor is `fn(AcceptedBlock) -> SelfAcceptedHandoff` (no raw-bytes / artifact / event / flag path). | `PHASE4-N-F-G-B` S1 (`c518c357`) |

Cross-reference: the new module is **not yet** in CODEMAP §GREEN or SEAMS — those two
docs are still at the **G-A** close (HEAD `80dac1f7`) and have not been regenerated for
G-B. This HEAD_DELTAS regen is the first G-B grounding-doc refresh; the close-pass
regenerates CODEMAP / SEAMS / TRACEABILITY together. (Not a staleness *defect* — a
sequencing artifact; the close-pass reconciles all four.)

## 3. Modules Modified

All modified source files existed at baseline. Trivial/no-behavioral-effect changes are
skipped (the one-line `pub mod self_accepted_handoff;` registration in
`ade_runtime/src/producer/mod.rs`, and the `ade_node` test-file diffs that only adapt
call sites to the new `(CoordinatorEvent, Option<SelfAcceptedHandoff>)` tuple return).

| Module | Color | Scope | Key changes |
|--------|-------|-------|-------------|
| `ade_node::node_sync` | **RED** (host of GREEN-pure `forge_epoch_admission` + the fenced BLUE forge composition) | +165 / -? lines (mostly `#[cfg(test)]`) | **S1/S2 (`c518c357`, `a5af52e8`) — surface the self-accepted token at the recovered-forge boundary.** `forge_one_from_recovered` now returns `Result<(CoordinatorEvent, Option<SelfAcceptedHandoff>), NodeForgeError>` (was `Result<CoordinatorEvent, …>`). Every **fail-closed** branch (off-epoch `OffEpoch`, not-a-leader) returns `(.., None)` — a non-self-accepted outcome yields **no** servable token. On the success path it wraps `run_real_forge`'s newly-surfaced `Option<AcceptedBlock>` via `SelfAcceptedHandoff::from_self_accepted` (the **original** `self_accept` token, never re-derived from `artifact.bytes`). New `#[cfg(test)]` proofs: handoff present **iff** `ForgeSucceeded`; off-epoch fail-closed surfaces no handoff; the surfaced `Option<SelfAcceptedHandoff>` is **replay byte-identical** across two runs (same recovered base + keys); and `relay_loop_containment_semantics_unchanged_with_serve_sibling` (S2) — wiring the handoff sender leaves the forge tick at exactly one self-accept-only attempt, advances **no** durable tip, persists **no** snapshot, and adds **only** a typed channel send emitted exactly on `ForgeSucceeded`. |
| `ade_node::node_lifecycle` | **RED** | +64 / -? lines | **S2 (`a5af52e8`) — dispatcher-owned sibling served-chain admit task.** The `--mode node` forge-capable `On`-arm now spawns a sibling `tokio` task that owns the `ServedChainHandle` and is the **sole node-spine `push_atomic` site**, fed **only** by `handoff.into_accepted()` drained off a typed `mpsc::UnboundedSender<SelfAcceptedHandoff>`. The relay loop holds **only** the `Sender` (never a `ServedChainHandle`, never a `push_atomic` call). `ForgeActivation` gains a private `handoff_tx: Option<…>` + a `with_handoff_sender(tx)` opt-in builder; `None` ⇒ forge-capable but **non-serving** (byte-identical to the N-F-E/N-F-F relay). The relay-loop forge arm forwards the surfaced `Option<SelfAcceptedHandoff>` to the sender (best-effort typed send; if the sibling task ended, the handoff is dropped — forge stays self-accept-only). On loop exit the dispatcher drops the `Sender` so the sibling drains + exits, then joins it. The `ServedChainView` is retained for the S3 network-serve read path. The single `SystemClock` wall-clock seam (DC-NODE-03) and the `Off`/`On` dispatch shape are unchanged. |
| `ade_node::produce_mode` | **RED** | +48 / -? lines | **S1 (`c518c357`) — `run_real_forge` return-type split (no behavioral change).** `run_real_forge` now returns `(CoordinatorEvent, Option<ade_ledger::producer::AcceptedBlock>)`; the closed `CoordinatorEvent` / `ForgeSucceeded` surface is **unchanged** (the token rides a sibling return component, **not** a new variant or field). The forge composition moved verbatim into a private `run_real_forge_inner(.., self_accepted_out: &mut Option<AcceptedBlock>)` that keeps every pre-split fail-closed early-return byte-identical and writes the **original** owned `accepted` token into `self_accepted_out` at the **sole** success site (after `artifact_from_accepted` borrows it) — so `Some(..)` is structurally reachable only when the inner fn returns `ForgeSucceeded`. The **produce-mode** serve path is functionally unaffected: it takes `run_real_forge(..).0` (the event) and advances via the existing sole `ChainEvolution::advance`, ignoring the surfaced token. |

## 4. Feature Flags

**No project feature-flag deltas.** Ade declares no `[features]` table in any
workspace `Cargo.toml` (confirmed at both refs — the table is absent), and no
`#[cfg(feature = …)]` gate was introduced in the span. **No `Cargo.toml` change at all
in this window** (the prior G-A `serde_json` `raw_value` dependency-feature add already
landed before the baseline). No coupling, no `compile_error!` guard.

## 5. CI Checks (116 → 117; +1 new, 0 modified, 0 removed)

One new gate, repo-root-relative, mirroring the existing `ci/ci_check_*.sh`
convention. It strips the `#[cfg(test)]` module + line comments before its greps so
commentary naming a forbidden token cannot trip the guard, and feeds every grep via a
**here-string** (`<<<`) rather than `echo | grep` — the `339cccb1` SIGPIPE fix that
avoids a `pipefail` false-negative when `grep -q` exits early on a large input.

### PHASE4-N-F-G-B gate (1, from baseline through HEAD)

| Check | Status | Cluster origin | What it checks |
|-------|--------|----------------|----------------|
| `ci_check_served_chain_handoff_fence.sh` | **New** | S3 (`738dddd1`; SIGPIPE fix `339cccb1`) | Backs **CE-G-B-2 / CE-G-B-3** / the **DC-NODE-06** serve-ingress clause. Scoped to the `--mode node` lifecycle owner (`crates/ade_node/src/node_lifecycle.rs`), production body only (`#[cfg(test)]` + line comments stripped). Three guards: (1) **every** `push_atomic(` on the node spine is fed by `into_accepted()` (no raw-bytes / non-handoff serve ingress) and at least one such site exists; (2) **no** direct `served_chain_admit(` on the node spine — served-chain mutation goes only through the single `push_atomic` authority (CN-PROD-04); (3) the handoff channel is typed `UnboundedSender<SelfAcceptedHandoff>` and **not** `<Vec<u8>>` / `<ForgedBlockArtifact>` / `<bool>` (the serve-ingress carrier is the S1 self-accepted fence, never raw bytes / a flag). Hermetic — no Docker / cardano-cli / live node. |

The N-F-E forge-containment gate (`ci_check_node_run_loop_containment.sh`) is
**semantically unchanged** — the relay-loop body still performs no serve / admit /
gossip / broadcast / block-fetch / durable-tip mutation; the handoff is a typed
channel **send** from the loop, and served-chain mutation happens only in the **sibling**
task. G-B **added** a served-chain handoff gate but did **not** relax containment.

> Cross-reference (TRACEABILITY): at HEAD `339cccb1`, `DC-NODE-06.ci_script` is still
> `""` (the rule is `declared`), so `ci_check_served_chain_handoff_fence.sh` is **not
> yet** cross-referenced from any registry rule — it currently enforces a rule that has
> no committed `ci_script` binding, and TRACEABILITY (still at the G-A close, HEAD
> `62cb8718`) renders `DC-NODE-06` with empty Tests/CI cells. The close-pass flips
> `DC-NODE-06` to `enforced` (binding this gate + the S1/S2/S3 tests) and regenerates
> TRACEABILITY. See the warnings in the generation section.

## 6. Canonical Type Registry Delta

**n/a — no separate canonical-type registry is configured** (`canonical_type_registry: null`),
and **no BLUE crate changed**. The 456 BLUE canonical-type total is **unchanged (Δ0)**
across the span (independently re-verified at HEAD `339cccb1`: the CODEMAP at this HEAD
— still the G-A close — reports 456 canonical, and `git diff --name-only
80dac1f7..339cccb1` matches no `core_paths` entry). The new `SelfAcceptedHandoff`
carrier lives in the RED `ade_runtime` crate and is not canonical-counted;
`run_real_forge`'s newly-surfaced `Option<AcceptedBlock>` reuses the existing BLUE
`ade_ledger::producer::AcceptedBlock` (no new type). **No new `CoordinatorEvent`
variant or field was introduced** — the self-accepted token rides a sibling return
component.

## 7. Normative / Invariant Rule Delta (313 → 313)

**No rule ID was added or removed in the span** (313 at both refs). The G-B primary
invariant `DC-NODE-06` already existed at the baseline as the **forward sketch**
declared at the shared PHASE4-N-F-G invariant pass.

### G-B primary rule (still `declared` at this HEAD)

| ID | Tier | Status @ `339cccb1` | `introduced_in` | Summary |
|----|------|---------------------|-----------------|---------|
| `DC-NODE-06` | derived | **declared** | `PHASE4-N-F-G-B` | Self-accept → serve handoff on the `--mode node` relay spine (sibling serve task, shape B). Only a BLUE self-accepted forged artifact (whose **only** provenance is a `ForgeSucceeded` outcome — CN-FORGE-01) may enter the sibling served-chain serve task via the single `ServedChainHandle::push_atomic` authority; the serve task must not accept raw forged bytes, a failed forge output (`ForgeNotLeader` / `ForgeFailed`), a self-declared acceptance flag, or a peer-verdict substitute. The relay-loop body performs **no** serve/admit/gossip/block-fetch/durable-tip mutation — the handoff is a typed channel send of a constructor-fenced artifact, so the containment gate stays **semantically unchanged** (a cluster may ADD a served-chain handoff gate but MUST NOT relax containment). Peer acceptance is proven **only** by the peer's validation log (RO-LIVE-06), never by Ade's self-accept / `ForgeSucceeded` / any wire-success signal. **G-B S1/S2/S3 implement this rule** (the GREEN `SelfAcceptedHandoff` fence; the dispatcher-owned sibling `push_atomic` task; the hermetic block-fetch payload proof + `ci_check_served_chain_handoff_fence.sh`) — but at this HEAD the registry entry is still the sketch (`tests = []`, `ci_script = ""`, `strengthened_in = []`). The close-pass flips it `declared → enforced` when CE-G-B-1..3 are all green. |

### Strengthenings (owed at close, NOT yet committed)

At HEAD `339cccb1` **zero** `strengthened_in += "PHASE4-N-F-G-B"` tokens are committed.
The pending close-pass records the **2 cross-slice strengthenings** the cluster
composes on:

| Rule | Why strengthened by G-B |
|------|--------------------------|
| `CN-PROD-04` | The single `ServedChainHandle::push_atomic` admit authority is extended onto the `--mode node` relay spine via the dispatcher-owned sibling task; the new gate pins it as the **sole** node-spine served-chain mutation site (no direct `served_chain_admit`). |
| `CN-CONS-07` | The self-acceptance → serve bridge (only BLUE-self-accepted bytes may reach the served chain) is extended across the node-spine relay→sibling-task handoff seam — the typed `SelfAcceptedHandoff` carrier prevents the channel from being an end-run around the gate. |

### G-A close tail (already committed in this span)

For completeness — these were the *owed* strengthenings flagged in the **previous**
(N-F-G-A) HEAD_DELTAS window and are **committed here** by `62cb8718` + `1806584c`,
which is why the rule count holds at 313 rather than rising:

- `DC-EPOCH-03` flipped `declared → enforced` (`tests` + `ci_script =
  "ci/ci_check_node_forge_single_epoch_fail_closed.sh"` populated).
- 7 G-A strengthenings recorded (`CN-OPCERT-01`, `CN-GENESIS-01`, `DC-LEDGER-10`,
  `CN-NODE-01`, `DC-CINPUT-02b`, `DC-NODE-05`, `DC-NODE-03`).
- The four G-A gates bound to their `ci_script` slots (`1806584c`), with a TRACEABILITY
  regen; the registry stays byte-stable at 313 rules.

This section is informational and reflects the **committed** registry state at HEAD.
**No rule was removed (expected: 0).**

## 8. Honest residual (cluster scope)

**The relay-spine forge now hands its BLUE self-accepted artifact to a sibling
served-chain admit task. It still does NOT gossip, broadcast to a peer, or advance a
durable tip — and with no live feed wired this cluster, it does not even observably
forge.**

- **Typed handoff, not a serve.** The forge tick surfaces a constructor-fenced
  `SelfAcceptedHandoff` (S1) and the relay loop **sends** it over a typed mpsc channel;
  the dispatcher-owned **sibling** task is the sole node-spine `push_atomic` site, fed
  only by `into_accepted()` (S2). The relay loop holds only the `Sender` — never a
  `ServedChainHandle`, never `push_atomic`, never `served_chain_admit`.
- **Containment semantically unchanged.** The N-F-E forge-containment gate is
  semantically unchanged; the relay-loop body performs no serve / admit / gossip /
  broadcast / block-fetch / durable-tip mutation. `ci_check_served_chain_handoff_fence.sh`
  (S3) adds the serve-ingress guard without relaxing containment.
- **Self-accept gate held.** Only a BLUE `AcceptedBlock` (provenance `ForgeSucceeded`,
  CN-FORGE-01) can populate the carrier; fail-closed outcomes (`OffEpoch` /
  `ForgeNotLeader` / `ForgeFailed`) surface **no** handoff. The token is the **original**
  `self_accept` token, never re-derived from `artifact.bytes`. Replay byte-identical
  (same recovered base + keys ⇒ same `Option<SelfAcceptedHandoff>`).
- **Peer ACCEPT not claimed.** Served-chain admission is **not** peer acceptance.
  RO-LIVE-06 acceptance is proven only by the peer's validation log via
  `ba02_evidence::correlate` — never by Ade's self-accept / `ForgeSucceeded` / any
  wire-success signal.
- **Forge-CAPABLE but NOT observable.** With no live/continuing feed wired this cluster,
  `run_relay_loop` still halts before any `ForgeTick` on the empty binary source (the
  `On` arm is forge-capable, the serve sibling spawns, but neither is observable).
  **Observable forge / live serve / operator-peer ACCEPT / BA-02 / RO-LIVE-01
  acceptance is the gated G-C follow-on.** BA-02 is satisfied nowhere.
- **No BLUE change.** 456 BLUE canonical types unchanged (Δ0); no new `CoordinatorEvent`
  variant or field. All code lands in RED `ade_runtime` (the GREEN
  `producer::self_accepted_handoff` carrier) and RED `ade_node` (`produce_mode`
  `run_real_forge` return-type split, `node_sync` surfacing, `node_lifecycle` S2 sibling
  task). The carrier is pure — no I/O / clock / rand / float.

---

## Generation notes (regen `80dac1f7 → 339cccb1`, PHASE4-N-F-G-B)

- **Baseline is `80dac1f7`** (the `.idd-config.json` `head_deltas_baseline` value at
  regen time — the PHASE4-N-F-G-A slice-span HEAD). **The close-pass commit must bump
  `head_deltas_baseline` to `339cccb1`** (the G-B slice-span HEAD) so the next cluster's
  `/head-deltas` measures from here. The registry-count comment stays 313 (no ID added
  or removed across the window). The `.idd-config.json` edit itself is handled by the
  closer, not this regen.
- Counts are mechanical: commit log + `--shortstat` over `80dac1f7..339cccb1` (10
  commits, no merges / 26 files / +3391 / -1600); CI gate count via `ls-tree | grep
  ci_check_*.sh` at each ref (116 → 117, +1 new — `served_chain_handoff_fence`, none
  removed); registry rule count via `grep -c '^\[\[rules\]\]'` at each ref (313 → 313,
  no ID added or removed); workspace test attributes via `git grep -hE
  '#\[(tokio::)?test\]'` (2213 → 2223, +10); BLUE canonical types unchanged at 456 (no
  BLUE crate file in the diff — verified: `git diff --name-only` matches no
  `core_paths` entry).
- **Grounding-doc staleness (NOT an anomaly).** The committed CODEMAP (`80dac1f7`),
  SEAMS (`80dac1f7`), and TRACEABILITY (`62cb8718`) at this HEAD are all from the **G-A
  close** — they report **116** CI checks and render `DC-NODE-06` as a **declared
  forward sketch** with empty Tests/CI. They have **not** been regenerated for G-B, so
  they do **not** yet reflect G-B's new gate (117), the new GREEN
  `producer::self_accepted_handoff` module, or the S1/S2/S3 source. The committed
  registry (`DC-NODE-06` `declared`) and these three committed docs (`DC-NODE-06`
  declared sketch) therefore **agree** — there is **no** CODEMAP-vs-registry conflict.
  This HEAD_DELTAS regen is the first G-B grounding-doc refresh; the close-pass
  regenerates CODEMAP / SEAMS / TRACEABILITY alongside the `DC-NODE-06`
  `declared → enforced` registry flip + the 2 strengthenings. **Not a rule removal, not
  a discipline violation** — a sequencing artifact reconciled by the close-pass.
- **CI cross-reference warning.** `ci_check_served_chain_handoff_fence.sh` is **not yet**
  cited by any registry rule's `ci_script` at this HEAD (because `DC-NODE-06` is still
  `declared` with `ci_script = ""`), so it does **not** appear in TRACEABILITY as
  enforcing a named invariant. The close-pass flips `DC-NODE-06` to `enforced`
  (binding the gate) and regenerates TRACEABILITY. The four **G-A** gates, by contrast,
  are now cited (the `1806584c` gate-binding follow-up landed in this span).
- `DC-NODE-06` is `declared` at this HEAD; its `tests`/`ci_script` are empty, so it does
  not yet appear in TRACEABILITY and the new gate is not yet cross-referenced there — the
  close-pass flips it to `enforced` and refreshes TRACEABILITY (and the four grounding
  docs G-A → G-B). G-C (live operator serve) is the next sub-cluster; RO-LIVE-01 /
  RO-LIVE-06 acceptance stays the gated follow-on.
