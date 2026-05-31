# Ade — HEAD Deltas (Changes Since Baseline)

> **Status:** Living architectural document. Regenerated; not hand-edited.
>
> Regenerate with `/head-deltas <baseline>` after every cluster close. Baseline is recorded in `.idd-config.json` `head_deltas_baseline`.

> Baseline: `e606ed6` (PHASE4-N-F-E close — forge-tick on the relay spine, hermetic, self-accept-only, 2026-05-31 23:33)
> HEAD: `4eb7610` (PHASE4-N-F-F S5 — single-bootstrap gate ReceiveState owner allow-list, 2026-06-01 01:14)
> Cluster: **PHASE4-N-F-F — operator-key ingress into `--mode node` + operator-material-backed `ForgeActivation` + the binary `Some`/`None` forge-on flip**, slice span closed; close-pass commit to follow.
> 13 commits, 17 files changed, +2577 / -48 lines.

This window narrates the **PHASE4-N-F-F cluster** — making the `--mode node`
binary **forge-CAPABLE** with real operator keys. It ingests the complete operator
key set (cold / KES / VRF / opcert / genesis) from CLI flags, loads it into RED
custody, builds an operator-material-backed `ForgeActivation` on the **single
recovered `BootstrapState`** that already seeds the relay spine, and flips the
binary forge argument from the hardcoded `None` (the N-F-E relay-only behavior) to
`Some(activation)` when a complete key set is present. A **partial** key set fails
closed (exit 44); the **empty** set reproduces the byte-identical N-F-D/N-F-E relay
path. The forge stays **subordinate + self-accept-only** — the N-F-E containment
gate is semantically unchanged; nothing is served, admitted, broadcast, gossiped,
or durably tip-advanced. **No second bootstrap (CN-NODE-01 held), no Mithril call,
no new BLUE authority, no BLUE crate change.** The span also carries a close-surfaced
CI remediation (`4eb7610`): the single-bootstrap gate's `ReceiveState::new`
owner check, stale and red since N-F-C/N-F-D, is replaced with a per-file owner
allow-list and is green again.

## 0. Headline

| Count | Baseline (`e606ed6`) | HEAD (`4eb7610`) | Δ |
|---|---|---|---|
| CI gates (`ci/ci_check_*.sh`) | 110 | **112** | +2 new (`forge_intent_closed`, `operator_forge_no_secret_leak`); 1 **modified in place** (`node_binary_uses_single_bootstrap`) |
| Registry rules | 310 | **311** | +1 (CN-NODE-03 declared) |
| Test attributes (`#[test]`/`#[tokio::test]`, workspace) | 2169 | **2188** | +19 (all in `ade_node`: S1/S3/S4 + the binary On/Off arms; ~+25 case-level) |
| BLUE canonical types | 456 | 456 | **0 — NO BLUE change** (all code lands in RED/GREEN `ade_node`) |

> Registry note: at HEAD `4eb7610`, `CN-NODE-03` sits at `status = "declared"`
> (`tests = []`, `ci_script = ""`). The not-yet-made close-pass commit flips it to
> `enforced` (populating `tests` + `ci_script`) and records the **4 cross-slice
> strengthenings** against the rules it composes on — mirroring how the N-F-E
> close-pass flipped `DC-NODE-05` after its slice span. This doc narrates the slice
> span; the close-pass commit follows, as in prior cluster closes.

## 1. Commit Log (newest first)

| Hash | Type | Summary |
|------|------|---------|
| `4eb7610` | fix | PHASE4-N-F-F S5 — single-bootstrap gate ReceiveState owner allow-list |
| `d08a065` | docs | specify PHASE4-N-F-F S5 single-bootstrap gate precision |
| `58acca1` | test | PHASE4-N-F-F S4 — operator-material forge proof |
| `a6b5c2a` | docs | specify PHASE4-N-F-F S4 operator-material forge proof |
| `217ad15` | feat | PHASE4-N-F-F S3 — forge activation assembly + Some/None flip |
| `fc95eb4` | docs | tighten PHASE4-N-F-F S3 Mithril boundary |
| `4d11d88` | docs | specify PHASE4-N-F-F S3 forge activation assembly |
| `5980037` | feat | PHASE4-N-F-F S2 — operator material into RED custody |
| `c53755a` | docs | specify PHASE4-N-F-F S2 operator material custody |
| `3c4bcca` | feat | PHASE4-N-F-F S1 — forge intent classifier |
| `0e5f040` | docs | specify PHASE4-N-F-F S1 forge intent |
| `b2a2df6` | docs | plan PHASE4-N-F-F cluster slices |
| `a3eee84` | docs | declare PHASE4-N-F-F operator key ingress |

(Plus the pending close-pass commit: grounding-doc refresh + CN-NODE-03 `declared→enforced`
flip with 4 strengthenings + `.idd-config.json` baseline / registry-count bump + cluster-doc
archive + this HEAD_DELTAS.)

## 2. New Modules

Two new submodules, both in the existing RED `ade_node` crate. No new crate, no new
WAL/checkpoint/canonical type, no BLUE change.

| Module | Color | Purpose | Key sub-paths | Added in |
|--------|-------|---------|---------------|----------|
| `ade_node::forge_intent` | **GREEN** | Pure tri-state classifier: "may `--mode node` forge?" as a total function of which operator-key CLI flags are *present* — never of their contents. Opens no file, parses no key, touches no secret. | `classify_forge_intent(cold, kes, vrf, opcert, genesis) -> Result<ForgeIntent, ForgeIntentError>`; closed two-variant `ForgeIntent` (`On(ForgePaths)` \| `Off`); `ForgePaths` (the presence-validated complete path set); `ForgeIntentError::PartialKeySet { present, missing }` (static flag-name strings only). Exhaustive without a wildcard arm — a partial set can never collapse into `On`/`Off`. | `PHASE4-N-F-F` S1 (`3c4bcca`) |
| `ade_node::operator_forge` | **RED** | The single named `--mode node` operator-material ingress site. Consumes a presence-validated `ForgePaths` and builds a `ProducerShell` (RED key-custody holder) by **reusing** the existing cold/VRF/KES/opcert loaders — no parser reimplementation. Then assembles the canonical forge material. | `load_operator_producer_shell(paths) -> Result<.., OperatorForgeError>` (S2); `build_operator_forge_material(paths) -> Result<OperatorForgeMaterial, ..>` + the `OperatorForgeMaterial` carrier (S3); closed secret-free `OperatorForgeError` (inner `KeyLoadError` carries no path bytes per OP-OPS-04). RED-confined custody: no byte accessor, no serialization, no logging. | `PHASE4-N-F-F` S2 (`5980037`) / S3 (`217ad15`) |

Both modules are wired in `crates/ade_node/src/lib.rs` (`pub mod forge_intent;`,
`pub mod operator_forge;`).

Cross-reference: both already appear in CODEMAP §GREEN / §RED.

## 3. Modules Modified

| Module | Color | Scope | Key changes |
|--------|-------|-------|-------------|
| `ade_node::node_lifecycle` | **RED** | +280 / -38 lines | **S3 (`217ad15`) — the forge-on flip.** `run_node_lifecycle_inner` now classifies forge intent from operator-key flag *presence* (`classify_forge_intent` over the `cli` flags) and `match`es it: `ForgeIntent::Off` moves the recovered ledger + chain_dep into the relay spine and calls `run_relay_loop(.., None)` (the exact N-F-D/N-F-E relay path); `ForgeIntent::On(paths)` builds the operator-material-backed `ForgeActivation` (via `operator_forge::build_operator_forge_material`, hosting the reused `kes_period_for_slot` through a secret-free `coordinator_init`), constructs a `SystemClock` as the sole wall-clock seam (DC-NODE-03), and calls `run_relay_loop(.., Some(&mut activation))`. New `NodeLifecycleError::ForgeKeyIngress(String)` + `EXIT_NODE_FORGE_KEY_INGRESS_FAILED = 44` (distinct exit so an operator distinguishes a partial/failed key-ingress from a bootstrap/recovery/sync/bad-CLI exit; secret-free message). **Recovered-state lifetime restructure:** on the `On` arm the recovered ledger + chain_dep are *cloned* into the relay spine (the spine evolves its copy), while `state` is kept owned as the forge baseline — one recovered state, forge base = spine base. The forge base does **not** bootstrap and does **not** call Mithril. Two new `#[tokio::test]`s: forge-on warm-start halts clean (forge-capable) and partial keys fail closed. |
| `ade_node::produce_mode` | **RED** | +12 / -3 lines | **S2 (`5980037`) — visibility only.** Three loaders widened from private `fn` to `pub(crate)` so the new `operator_forge` ingress site reuses them rather than duplicating parsers: `load_kes_skey_any_format`, `parse_simple_opcert_json`, `parse_simple_genesis_json`. No behavioral change. |
| `ade_node::node_sync` | **RED** | +201 / -0 lines (`#[cfg(test)]` only) | **S4 (`58acca1`) — operator-material forge proof, tests only.** New `#[cfg(test)]` fixtures + tests that drive the binary On path through the relay loop to the fenced `forge_one_from_recovered`: `relay_loop_with_operator_material_forge_reaches_fenced_path` (operator-material activation reaches the single fenced forge handoff, self-accept-only) and `relay_loop_with_operator_material_two_runs_byte_identical` (replay-equivalence — same recovered state + ordered feed + injected clock ⇒ byte-identical forge sequence). `run_node_sync` itself is **UNMODIFIED**. |

`ade_node::lib` changed (+2) only to register the two new submodules; not a behavioral
modification.

## 4. Feature Flags

No feature-flag deltas. No `Cargo.toml` changed in the span.

## 5. CI Checks (110 → 112; +2 new, 1 modified in place)

| Check | Status | Backs |
|-------|--------|-------|
| `ci_check_forge_intent_closed.sh` | **New** (S1) | CN-NODE-03 (intent-classification half). Asserts the GREEN classifier module exists with a `//! GREEN` banner, `ForgeIntent` + `classify_forge_intent` are defined, and the production body (doc comments + `#[cfg(test)]` stripped first) is closed: no `#[non_exhaustive]`, no key-material / I/O / clock / nondeterminism token, and **no wildcard arm** in the classification decision that could collapse an unenumerated presence combination into `On` or `Off`. |
| `ci_check_operator_forge_no_secret_leak.sh` | **New** (S2) | CN-NODE-03 (RED-custody-loading half). Asserts the ingress module has a `//! RED` banner, `load_operator_producer_shell` is defined, `ProducerShell::init` is called (freshness enforced there, not reimplemented), and the production body leaks no private-key bytes — no byte accessor, no serialization, no logging vector. |
| `ci_check_node_binary_uses_single_bootstrap.sh` | **Modified in place** (S5, `4eb7610`) | CN-NODE-01 (single bootstrap authority). Replaces the stale `≤1 ReceiveState::new per crate` count — impossible-to-satisfy once two lifecycle-owner files exist, and unable to tell mutually-exclusive arms apart — with a per-file **owner allow-list** (`node.rs`, `node_lifecycle.rs`). `ReceiveState::new` in any *other* `ade_node` production file is a rogue recovered-state bypass and fails closed; the mutually-exclusive `ForgeIntent::Off`/`On` arms of the single dispatcher are no longer miscounted. Double-bootstrap-within-a-path stays covered by the per-file `bootstrap_initial_state` check + `ci_check_node_run_loop_containment.sh`. This was a close-surfaced remediation from the per-cluster security review (the guard had been red since N-F-C/N-F-D). |

The N-F-E forge-containment gate (`ci_check_node_run_loop_containment.sh`) is
**semantically unchanged** — still exactly one fenced `forge_one_from_recovered`
call, no serve/admit/gossip/broadcast/block-fetch/durable-tip path. N-F-F added
key-ingress gates but did **not** relax forge containment.

Cross-reference: the two new gates and the modified gate will be cross-referenced
in TRACEABILITY by the close-pass (CN-NODE-03 is still `declared` at this HEAD, so
its `ci_script` array is empty and TRACEABILITY does not yet list these gates — see
the warnings below).

## 6. Canonical Type Registry Delta

n/a — no separate canonical-type registry is configured (`canonical_type_registry: null`),
and no BLUE crate changed. The 456 BLUE canonical-type total is **unchanged (Δ0)**.
The new `ForgeIntent` / `ForgePaths` / `ForgeIntentError` / `OperatorForgeMaterial` /
`OperatorForgeError` / `ForgeActivation` types and the `NodeLifecycleError::ForgeKeyIngress`
variant all live in the RED/GREEN `ade_node` crate and are not canonical-counted.

## 7. Normative / Invariant Rule Delta (310 → 311)

### New rule (declared at sketch this cluster)

| ID | Tier | Status @ `4eb7610` | Summary |
|----|------|--------------------|---------|
| `CN-NODE-03` | constraint_network | **declared** | Operator-key ingress + forge-on flip for `--mode node`. Ingress builds an operator-material-backed `ForgeActivation` strictly via RED-parse → BLUE-structural-validator → canonical-type, **reusing** the existing KES/VRF/cold/opcert loaders — no new BLUE authority, no parser reimplementation, no plugin seam, no second forge codepath, no BLUE crate change. Key custody stays RED-confined to `ProducerShell` (no copy/extract into GREEN coordinator/planner/node/loop state or any persisted/logged/hashed-for-evidence/replay surface; tests must not print/snapshot/serialize/compare private key bytes). Forge intent is a pure total function of CLI key-flag presence: complete set ⇒ `Some(activation)`; all absent ⇒ `None` (byte-identical N-F-D relay); any partial subset ⇒ structured fail-closed error (never a silent relay fallback, never a missing/zero/fabricated key). The forge base is the **same** recovered `BootstrapState` that seeds the relay spine (single bootstrap authority; no second bootstrap, no second recovered state). Forge stays subordinate + self-accept-only — the N-F-E containment gate stays semantically unchanged; N-F-F may add key-ingress gates but must not relax forge containment. N-F-F makes the binary forge-**capable** once paired with a live/continuing feed; it does **not** make forge observable on the current empty-source binary path (`plan_loop_step` halts on `LoopState::Ending` even when a slot is `Due`) and makes **no** live forge / serve / gossip / peer-acceptance / BA-02 / RO-LIVE / durable-tip claim. `pparams` / `protocol_version` reuse the produce-path honest-scope defaults — ingress/activation wiring, not mainnet-complete block-production fidelity. |

> At HEAD `4eb7610`, `CN-NODE-03.status = "declared"` (`tests = []`, `ci_script = ""`,
> `introduced_in = "PHASE4-N-F-F"`, `strengthened_in = []`). The pending close-pass
> commit flips it to `enforced` (populating `tests` + `ci_script` with the S1/S2 gates
> and the S3/S4 lifecycle + node_sync tests) and records the **4 cross-slice
> strengthenings** against the rules it composes on — `OP-OPS-04`, `CN-PROD-02`,
> `DC-NODE-05`, and the single-bootstrap-authority anchor (`evidence_notes`:
> "OP-OPS-04 / CN-PROD-02 / DC-NODE-05 get `strengthened_in += "PHASE4-N-F-F"`";
> `cross_ref` also carries CN-NODE-01 / CN-NODE-02 / DC-CINPUT-02b / CN-CINPUT-03
> unchanged). This mechanical narration counts only what is committed at `4eb7610`.

This section is informational and reflects the registry state at HEAD. **No rule was
removed (expected: 0).**

## 8. Honest residual (cluster scope)

**Forge-CAPABLE with real operator keys, but NOT observable on the empty-source
binary path.**

- **Forge-capable, not observable.** The binary ingests a complete operator key set,
  loads it into RED custody, and builds the operator-material-backed `ForgeActivation`
  on the single recovered state — but with no live/continuing feed wired this cluster,
  the empty source halts the loop before any `ForgeTick` (forge is subordinate to the
  feed; `plan_loop_step` halts on `LoopState::Ending` even when a slot is `Due`).
  **Observable forge / live peer / BA-02 / RO-LIVE-01 acceptance is the operator-gated
  follow-on.**
- **Self-accept-only, unchanged.** No serve / admit / gossip / broadcast / block-fetch
  / durable-tip claim. The N-F-E containment gate is semantically unchanged; the forge
  remains the single fenced `forge_one_from_recovered` call. `run_node_sync → pump_block`
  remains the sole durable tip-advance authority.
- **Single bootstrap held (CN-NODE-01).** The forge base is the **same** recovered
  `BootstrapState` that seeds the relay spine — **no Mithril call, no second bootstrap,
  no second recovered state**. The recovered `state` outlives both `ForwardSyncState`
  and `ForgeActivation`.
- **Fail-closed ingress.** A partial operator key set fails closed (exit 44,
  `NodeLifecycleError::ForgeKeyIngress`) — never a silent relay-only fallback, never a
  forge with a missing/zero/fabricated key. The empty set is the byte-identical N-F-D
  relay path.
- **No BLUE change.** 456 BLUE canonical types unchanged (Δ0); all code lands in the
  RED `operator_forge` / RED `node_lifecycle` and the GREEN `forge_intent` classifier.
  Leadership eligibility stays in BLUE `forge_one_from_recovered`; no new BLUE authority,
  no plugin/trait seam, no parser reimplementation.

---

## Generation notes (regen `e606ed6 → 4eb7610`, PHASE4-N-F-F)

- **Baseline is `e606ed6`, the PHASE4-N-F-E close — not the `.idd-config.json` value.**
  At regen time `.idd-config.json` `head_deltas_baseline` reads `cd2484f` (the N-F-E
  **pre-close** S3b commit), which is **stale**: the N-F-E close-pass landed at
  `e606ed6` and is the correct frozen reference for the N-F-F span. This doc was
  generated against `e606ed6`. **The close-pass commit must bump
  `head_deltas_baseline` to `4eb7610`** (the N-F-F HEAD) so the next cluster's
  `/head-deltas` measures from here, and bump the registry-count comment 310 → 311.
- Counts are mechanical: commit log + `--stat` over `e606ed6..4eb7610`; CI gate count
  via `ls-tree | grep ci_check_*.sh` at each ref (110 → 112); registry rule count via
  `grep -c '^\[\[rules\]\]'` at each ref (310 → 311, one ID added: `CN-NODE-03`, none
  removed); workspace test attributes via `grep -rohE '#\[(tokio::)?test\]'` (2169 →
  2188, all +19 in `ade_node`); BLUE canonical types unchanged at 456 (no BLUE crate in
  the diff).
- CN-NODE-03 is `declared` at this HEAD; its `tests`/`ci_script` are empty, so it does
  not yet appear in TRACEABILITY and the two new gates are not yet cross-referenced
  there — the close-pass flips it to `enforced` and refreshes TRACEABILITY (and the
  four grounding docs N-F-E → N-F-F).
