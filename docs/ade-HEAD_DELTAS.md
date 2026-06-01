# Ade — HEAD Deltas (Changes Since Baseline)

> **Status:** Living architectural document. Regenerated; not hand-edited.
>
> Regenerate with `/head-deltas <baseline>` after every cluster close. Baseline is recorded in `.idd-config.json` `head_deltas_baseline`.

> Baseline: `4eb7610` (PHASE4-N-F-F S5 — single-bootstrap gate ReceiveState owner allow-list, 2026-06-01 01:14)
> HEAD: `80dac1f7` (fail closed on off-epoch forge before leadership, 2026-06-01 17:12)
> Cluster: **PHASE4-N-F-G-A — forge fidelity on the `--mode node` spine**, slice span closed; close-pass commit to follow.
> 20 commits (19 non-merge + 1 N-F-F close merge), 43 files changed, +5221 / -595 lines.

This window narrates the **PHASE4-N-F-G-A cluster** — the first of three planned
PHASE4-N-F-G sub-clusters (G-A forge fidelity / G-B self-accept→serve handoff / G-C
live operator serve). G-A **hardens the N-F-F forge-on path so the constants it
forges with are real**, and so two boundaries the prior path masked **fail closed**.
The `--mode node` forge path now sources the **real** operator opcert/genesis config
through the closed cardano-cli parsers (retiring the `parse_simple_*` stubs on the
node path), installs the **current** oracle-bound `ProtocolParameters` + protocol
version into the recovered ledger (instead of `ProtocolParameters::default()`), fails
closed on a before-genesis-anchor clock→slot saturation (S3), and fails closed on an
off-epoch forge slot **before** leadership / KES signing with **no nonce promotion**
(S4). A new GREEN `#[cfg(test)]` genesis-consistency pinning harness (S1/S1b) drives
the **real** `bootstrap_initial_state` warm-start against a committed private-net
Ade-as-leader reference fixture. **NO BLUE crate changed (456 canonical types, Δ0);
no new `CoordinatorEvent` variant; the forge stays subordinate + self-accept-only —
the N-F-E containment gate is byte-unchanged; nothing is served, admitted, gossiped,
broadcast, or durably tip-advanced.** Live serve / operator-peer ACCEPT / BA-02 /
RO-LIVE remain the gated G-B / G-C follow-ons.

## 0. Headline

| Count | Baseline (`4eb7610`) | HEAD (`80dac1f7`) | Δ |
|---|---|---|---|
| CI gates (`ci/ci_check_*.sh`) | 112 | **116** | **+4 new** (`genesis_consistency_fixture_present`, `recovered_ledger_pparams_sourced`, `node_forge_real_cli_ingress`, `node_forge_single_epoch_fail_closed`); none removed |
| Registry rules | 311 | **313** | **+2** (`DC-EPOCH-03`, `DC-NODE-06` — both `declared` at this HEAD); none removed |
| Test attributes (`#[test]`/`#[tokio::test]`, workspace) | 2188 | **2213** | **+25** (G-A surface, across `protocol_params`/`clock`/`canonical`/`genesis_pinning`/`node_sync`/`node_lifecycle`/`seed_to_snapshot`) |
| BLUE canonical types | 456 | 456 | **0 — NO BLUE change** (all code lands in RED `ade_runtime` / RED `ade_node` / GREEN `ade_testkit`) |

> **Registry note (load-bearing — read before §7).** At HEAD `80dac1f7` the
> committed registry is in **slice-span state**: `DC-EPOCH-03` sits at
> `status = "declared"` (`tests = []`, `ci_script = ""`, `introduced_in =
> "PHASE4-N-F-G-A"`, `strengthened_in = []`), and **none** of the seven rules the
> cluster composes on carry a `strengthened_in += "PHASE4-N-F-G-A"` token yet. The
> not-yet-made **close-pass commit** flips `DC-EPOCH-03` `declared → enforced`
> (populating `tests` + `ci_script` with the S4 gate + tests) and records the **7
> cross-slice strengthenings** (`CN-OPCERT-01`, `CN-GENESIS-01`, `DC-LEDGER-10`,
> `CN-NODE-01`, `DC-CINPUT-02b`, `DC-NODE-05`, `DC-NODE-03`) — exactly as the N-F-E
> close-pass flipped `DC-NODE-05` and the N-F-F close-pass flipped `CN-NODE-03` after
> their slice spans. This doc narrates **what is committed at `80dac1f7`**; the
> close-pass follows. (The already-regenerated CODEMAP narrates the *owed end-state*
> — see the anomaly note in the generation section.)

## 1. Commit Log (newest first)

| Hash | Type | Summary |
|------|------|---------|
| `80dac1f7` | feat | feat(node): fail closed on off-epoch forge before leadership |
| `5204a8d8` | docs | specify PHASE4-N-F-G-A S4 epoch fail-closed |
| `2049ec9d` | feat | feat(node): fail closed on before-anchor forge slot alignment |
| `2a27d352` | docs | specify PHASE4-N-F-G-A S3 slot alignment |
| `11704998` | feat | PHASE4-N-F-G-A S2 — real opcert/genesis ingress + recovered-current constants |
| `e56e8cca` | docs | refresh PHASE4-N-F-G-A S2 — PO-1 discharged by S2a, narrowed scope |
| `3dba81db` | feat | PHASE4-N-F-G-A S2a — current protocol parameters source |
| `38680107` | docs | specify PHASE4-N-F-G-A S2a current pparams source |
| `225e61d9` | docs | insert PHASE4-N-F-G-A S2a (current protocol params source) into G-A plan + cluster doc |
| `a5afb013` | docs | specify PHASE4-N-F-G-A S2 real opcert/genesis ingress + derived constants |
| `b5decb3e` | test | PHASE4-N-F-G-A S1 — genesis-consistency pinning harness |
| `23addfbb` | test | PHASE4-N-F-G-A S1b — private-net genesis reference fixture |
| `50c6a5b0` | docs | specify PHASE4-N-F-G-A S1b private reference extraction |
| `2684a618` | docs | specify PHASE4-N-F-G-A S1 genesis pinning |
| `58809947` | docs | define PHASE4-N-F-G-A forge fidelity cluster |
| `1c3b077f` | docs | plan PHASE4-N-F-G as three sub-clusters |
| `b063c6a4` | docs | declare PHASE4-N-F-G live serve invariants |
| `cd2f87ae` | docs | Close PHASE4-N-F-F — operator-key ingress + forge-on flip for --mode node |
| `73f032fb` | ci | notify ade-atlas on grounding-doc changes (event-driven; cron stays as fallback) |

Plus one merge commit in the span: `f08b12ca` *Merge origin/main (ade-atlas notify CI)
into PHASE4-N-F-F close* — the N-F-F close merge that opens the window.

The first three non-G-A commits (`cd2f87ae` N-F-F close-pass, `73f032fb` + the
`f08b12ca` merge the non-gating ade-atlas notify CI) are the **N-F-F tail** that
lands inside this span because the baseline `4eb7610` is the N-F-F *slice-span* HEAD,
not its close commit. `b063c6a4` / `1c3b077f` are the PHASE4-N-F-G **invariant
sketch + sub-cluster plan** (they declare `DC-EPOCH-03` and `DC-NODE-06` and the G-B /
G-C boundaries), and `58809947` defines the G-A cluster. Everything from `2684a618`
onward is G-A proper (S1 → S4).

(Plus the pending close-pass commit: grounding-doc refresh + `DC-EPOCH-03`
`declared → enforced` flip with 7 strengthenings + `.idd-config.json` baseline bump
`4eb7610 → 80dac1f7` and registry-count bump 311 → 313 + cluster-doc archive + this
HEAD_DELTAS.)

## 2. New Modules

Two new source modules — one GREEN parser in the RED `ade_runtime` crate, one GREEN
`#[cfg(test)]` harness in the GREEN `ade_testkit` crate — plus a committed test
fixture bundle. No new crate, no new BLUE authority, no new WAL/checkpoint/canonical
type.

| Module | Color | Purpose | Key sub-paths | Added in |
|--------|-------|---------|---------------|----------|
| `ade_runtime::consensus_inputs::protocol_params` | **GREEN** | The cardano-cli `query protocol-parameters` JSON parser. Converts the oracle's `protocol_params_json` preimage (carried in the operator consensus-inputs bundle) into a canonical BLUE `ProtocolParameters`, so the recovered ledger carries the **current** protocol version + modeled parameters instead of `ProtocolParameters::default()` (the stale `protocol_major = 2` the S2 PO-1 entry check exposed). | `parse_protocol_parameters_json(bytes, network_magic) -> Result<ProtocolParameters, ProtocolParamsParseError>`; closed `ProtocolParamsParseError`. **No float path (hard rule):** the rational unit-/non-negative-interval literals (`poolPledgeInfluence`, `monetaryExpansion`, `treasuryCut`) are preserved as strings via `serde_json::value::RawValue` and converted to exact `ade_ledger::rational::Rational` by **integer** decimal/scientific parsing — no `f64`, no `as f64`, no serde float deserialization; a literal that cannot be represented exactly fails closed (`InexactRational`). Conway-only fields outside the modeled forge-header surface are **ignored, not denied** (documented). | `PHASE4-N-F-G-A` S2a (`3dba81db`) |
| `ade_testkit::consensus::genesis_pinning` | **GREEN** (`#[cfg(test)]`) | A genesis-consistency pinning harness. Reads the committed private-net Ade-as-leader reference fixture (S1b), builds the recovered seed-epoch surface, drives the **real** `bootstrap_initial_state` warm-start, and pins Ade's recovered values + leader-eligibility inputs against the genesis-derived reference. Non-authoritative test infrastructure; comparisons over observable/derived surfaces only (DC-COMPAT-01). | The four named pinning tests; `include_str!` of the committed fixture bundle; the recovered-surface builder + reference cross-checks (eta0 == genesis_hash, ASC numer/denom, non-empty pool_distribution, per-pool pool_vrf_keyhashes). | `PHASE4-N-F-G-A` S1 (`b5decb3e`) |

New committed **test fixture** (not a code module): `crates/ade_testkit/fixtures/nfg_a_privnet_reference/`
— a private-net Ade-as-leader reference bundle, `consensus-inputs.json` +
`shelley-genesis.json` + `PROVENANCE.md` (S1b, `23addfbb`). Evidence input to the S1
harness, **never runtime authority**; the `ci_check_genesis_consistency_fixture_present.sh`
gate asserts no `*.skey`/`*.vkey`/cardano-cli signing-key envelope leaked into the dir.

Cross-reference: both modules + the fixture already appear in CODEMAP §GREEN (the
CODEMAP was regenerated against this same HEAD).

## 3. Modules Modified

All modified files existed at baseline. Trivial/no-behavioral-effect changes are
skipped (e.g., the one-line `serde_json` dependency-feature add in `ade_runtime/Cargo.toml`
— see §4).

| Module | Color | Scope | Key changes |
|--------|-------|-------|-------------|
| `ade_node::node_sync` | **RED** (host of GREEN-pure `forge_epoch_admission` + the fenced BLUE `forge_one_from_recovered`) | +284 / -0 lines (mostly `#[cfg(test)]`) | **S4 (`80dac1f7`) — the off-epoch fail-closed boundary.** New closed two-variant `ForgeEpochAdmission` (`WithinSeedEpoch` \| `OffEpoch { candidate_epoch, seed_epoch }`) + the GREEN-pure `forge_epoch_admission(slot, era_schedule, seed_epoch) -> ForgeEpochAdmission` total function that derives the candidate epoch via the BLUE `EraSchedule::locate` (no fabricated epoch math; a `locate` error is treated as off-epoch). `forge_one_from_recovered` now calls `forge_epoch_admission` and fails closed on `OffEpoch` **BEFORE** `query_leader_schedule` — off-epoch forge fails before leadership / KES signing, and the apply path drives **no** `NonceInput::EpochBoundary` / `CandidateFreeze` nonce promotion (the seed-epoch eta0 stays frozen at the recovered value). New `#[cfg(test)]` proofs: `forge_epoch_admission_within_seed_epoch_admits`, off-epoch fail-closed, no-nonce-promotion. |
| `ade_node::operator_forge` | **RED** | +156 / -88 lines | **S2 (`11704998`) — real config ingress on the node path.** The operator-forge ingress site retires the `parse_simple_opcert_json` / `parse_simple_genesis_json` stubs **on the node path** and loads operator config through the **real** closed-contract cardano-cli parsers `parse_opcert_envelope` + `parse_shelley_genesis`. `protocol_version` / `pparams` are taken from the recovered **current** view (S2a) rather than produce-path honest-scope defaults. Custody stays RED-confined; the `ci_check_node_forge_real_cli_ingress.sh` gate fails closed if a simple-JSON parser reappears on the node forge path. |
| `ade_node::node_lifecycle` | **RED** | +98 / -? lines | **S2/S3 wiring.** Threads the recovered **current** `ProtocolParameters` + protocol version into the forge-capable `On`-arm activation (S2/S2a), and wires the S3 checked clock→slot guard so a before-genesis-anchor candidate slot fails closed (`SlotAlignmentError::BeforeGenesisAnchor`) instead of saturating to `start_slot`. The single `SystemClock` wall-clock seam (DC-NODE-03) is unchanged in placement; the N-F-F forge-on `Off`/`On` dispatch is unchanged in shape. New tests: protocol-version/pparams sourced from the recovered current view; before-anchor fail-closed. |
| `ade_node::admission::seed_to_snapshot` | **RED** | +66 / -? lines | **S2a (`3dba81db`) — install current pparams at seed/import.** `build_seed_ledger` now installs the **caller-supplied current** `ProtocolParameters` into the recovered ledger at the forge-capable seed import — never `ProtocolParameters::default()` / genesis-initial. The bind is **fail-closed** on an absent preimage / hash mismatch (via the new `require_forge_current_pparams` accessor). The `ci_check_recovered_ledger_pparams_sourced.sh` gate fails closed if a future change reverts the forge recovered-ledger to a defaulted `protocol_params`. |
| `ade_node::admission::bootstrap` | **RED** | +23 / -? lines | **S2a — forge-capable bootstrap binds the current pparams.** The forge-capable bootstrap path binds + installs the current pparams preimage (hash-bound), preserving warm-start behavior. No second bootstrap (CN-NODE-01 held). |
| `ade_runtime::clock` | **GREEN-by-content seam** | +63 / -0 lines | **S3 (`2049ec9d`) — checked clock→slot.** New `checked_millis_to_slot(tick_millis, slot_length_ms, start_slot, ..) -> Result<SlotNo, SlotAlignmentError>` + closed `SlotAlignmentError` (`BeforeGenesisAnchor`). Returns the aligned slot exactly when `tick_millis >= anchor`, and `Err(BeforeGenesisAnchor)` when `tick_millis < anchor` — replacing the saturating `millis_to_slot` behavior on the forge path. Two `#[cfg(test)]` proofs: matches `millis_to_slot` when aligned; before-anchor fails closed. The existing `SystemClock` / `DeterministicClock` seams are unchanged. |
| `ade_runtime::consensus_inputs::canonical` | **GREEN** | +103 / -0 lines | **S2a — the hash-bound preimage accessor.** New `LiveConsensusInputsCanonical::require_forge_current_pparams` (+ supporting carrier) — a hash-bound preimage accessor that yields the operator-bundle current-pparams JSON only when its hash matches the canonical commitment, so the S2a install path is preimage-bound and fails closed on absence / mismatch. `#[cfg(test)]` present / hash-mismatch proofs. |
| `ade_runtime::consensus_inputs::{importer, json, mod, seed_consensus_merge}` | **GREEN** | +17 / -0 lines total | **S2a plumbing.** `mod.rs` registers `pub mod protocol_params;`; `importer.rs` (+5) / `json.rs` (+9) / `seed_consensus_merge.rs` (+1) thread the current-pparams preimage through the import + merge path. No behavioral change beyond carrying the new preimage. |

## 4. Feature Flags

**No project feature-flag deltas.** Ade declares no `[features]` table in any
workspace `Cargo.toml` (confirmed at both refs — the table is absent), and no
`#[cfg(feature = …)]` gate was introduced in the span.

The only `Cargo.toml` change is a **dependency-feature** add in
`crates/ade_runtime/Cargo.toml`: `serde_json` gains the `raw_value` feature
(`serde_json = { version = "1", features = ["raw_value"] }`). This is required by the
new GREEN `protocol_params` parser to preserve rational literals as strings
(`serde_json::value::RawValue`) and convert them to exact `Rational` by integer
parsing — the mechanical enforcement of the "no float path" hard rule. It gates no
Ade code via a `cfg`; it is a transitive capability of a dependency, not an Ade
feature flag. No coupling, no `compile_error!` guard.

## 5. CI Checks (112 → 116; +4 new, 0 modified, 0 removed)

All four new gates are repo-root-relative and mirror the existing `ci/ci_check_*.sh`
convention. Every one strips the `#[cfg(test)]` module + doc/line comments before its
negative greps so commentary naming a retired/forbidden token cannot trip the guard.

### PHASE4-N-F-G-A gates (4, from baseline through HEAD)

| Check | Status | Cluster origin | What it checks |
|-------|--------|----------------|----------------|
| `ci_check_genesis_consistency_fixture_present.sh` | **New** | S1 (`b5decb3e`) | Backs **CE-G-A-1** (genesis-consistency). The three fixture files are committed; the bundle is well-formed and Ade-as-leader (eta0, ASC numer/denom, non-empty pool_distribution, per-pool pool_vrf_keyhashes, eta0 == genesis_hash); **no secret key material** is committed (no `*.skey`/`*.vkey`, no cardano-cli signing-key envelope); and the GREEN harness module exists, is wired into `ade_testkit`, embeds the fixture via `include_str!`, and defines the four named pinning tests. Hermetic — no Docker / cardano-cli / live node. |
| `ci_check_recovered_ledger_pparams_sourced.sh` | **New** | S2a (`3dba81db`) | Backs **CE-G-A-2a**. The recovered ledger's `protocol_params` are sourced from the operator consensus-inputs bundle's oracle preimage at the forge-capable seed import — **never** `ProtocolParameters::default()` / genesis-initial. Positive: `build_seed_ledger` installs the caller-supplied current pparams. Negative: its production body must NOT default `protocol_params`. The bind is fail-closed on absent preimage / hash mismatch. |
| `ci_check_node_forge_real_cli_ingress.sh` | **New** | S2 (`11704998`) | Backs **CE-G-A-2**. The `--mode node` operator-forge ingress loads config through the **real** closed-contract parsers (`parse_opcert_envelope` + `parse_shelley_genesis`) and **retires the `parse_simple_*` stubs on the node path**. Fails closed if a future change reintroduces a simple-JSON parser on the node forge path. |
| `ci_check_node_forge_single_epoch_fail_closed.sh` | **New** | S4 (`80dac1f7`) | Backs **CE-G-A-4** / **DC-EPOCH-03**. The node forge path fails closed at the single recovered seed-epoch boundary **before** leadership / KES signing and drives **no** nonce promotion: (a) `forge_one_from_recovered` calls the explicit `forge_epoch_admission` guard BEFORE `query_leader_schedule`; (b) the guard derives the candidate epoch via the BLUE `EraSchedule::locate` (no fabricated epoch math); (c) the node forge path drives no `NonceInput::EpochBoundary` / `CandidateFreeze` promotion. Fails closed if a future change reorders the guard after leadership, fabricates the epoch, or introduces a nonce roll. |

The N-F-E forge-containment gate (`ci_check_node_run_loop_containment.sh`) is
**semantically unchanged** — still exactly one fenced `forge_one_from_recovered`
call, no serve / admit / gossip / broadcast / block-fetch / durable-tip path. G-A
added forge-**fidelity** gates but did **not** relax forge containment.

> Cross-reference: at HEAD `80dac1f7`, `DC-EPOCH-03.ci_script` is still `""` (the rule
> is `declared`), so these four gates are **not yet** cross-referenced from
> TRACEABILITY. The close-pass flips `DC-EPOCH-03` to `enforced` (populating its
> `ci_script` + `tests`) and refreshes TRACEABILITY. See the warnings in the
> generation section.

## 6. Canonical Type Registry Delta

**n/a — no separate canonical-type registry is configured** (`canonical_type_registry: null`),
and **no BLUE crate changed**. The 456 BLUE canonical-type total is **unchanged (Δ0)**
across the span (independently re-verified in CODEMAP: `ade_ledger` 177, `ade_core`
49, `ade_codec` 11, `ade_types` 81, `ade_crypto` 21, `ade_plutus` 8, `ade_network`
BLUE submodules 109). The new `ProtocolParamsParseError`, `SlotAlignmentError`, and
`ForgeEpochAdmission` types all live in the RED `ade_runtime` / RED `ade_node` crates
and are not canonical-counted. No new `CoordinatorEvent` variant was introduced.

## 7. Normative / Invariant Rule Delta (311 → 313)

Two rule IDs were added in the span. **Both are `declared` sketches at HEAD
`80dac1f7`** — neither is `enforced` in the committed registry yet.

### New rules (declared at the PHASE4-N-F-G invariant sketch this cluster)

| ID | Tier | Status @ `80dac1f7` | `introduced_in` | Summary |
|----|------|---------------------|-----------------|---------|
| `DC-EPOCH-03` | derived | **declared** | `PHASE4-N-F-G-A` | Single-epoch forge fail-closed on the `--mode node` spine. A candidate forge slot is valid only within the single recovered seed epoch; an off-epoch candidate fails closed and the seed-epoch nonce (eta0) stays frozen at the recovered value — the forge apply path does **not** drive the BLUE `CandidateFreeze` / `EpochBoundary` nonce transitions. Cross-epoch production (nonce roll + epoch boundary) is **forbidden, not silently attempted**. **S4 (`80dac1f7`) implements this rule** (`forge_epoch_admission` + the before-leadership guard + the no-nonce-promotion property) and `ci_check_node_forge_single_epoch_fail_closed.sh` enforces it — but at this HEAD the registry entry is still the sketch (`tests = []`, `ci_script = ""`, `strengthened_in = []`). The close-pass flips it `declared → enforced`. |
| `DC-NODE-06` | derived | **declared** | `PHASE4-N-F-G-B` | Self-accept → serve handoff on the `--mode node` relay spine (sibling serve task, shape B). A **forward sketch for the next sub-cluster (G-B)** — declared at the PHASE4-N-F-G invariant pass, **NOT implemented or enforced by G-A**. Only a BLUE self-accepted forged artifact (`ForgeSucceeded` provenance) may enter the sibling served-chain serve task via the single `ServedChainHandle::push_atomic` authority; the relay-loop body performs no serve/admit/gossip/block-fetch/durable-tip mutation, so the containment gate stays semantically unchanged. Peer acceptance is proven ONLY by the peer's validation log (RO-LIVE-06), never by Ade's self-accept / any wire-success signal. `tests = []`, `ci_script = ""`, `status = "declared"`. |

### Strengthenings (owed at close, NOT yet committed)

At HEAD `80dac1f7` **zero** `strengthened_in += "PHASE4-N-F-G-A"` tokens are
committed. The pending close-pass records the **7 cross-slice strengthenings** the
cluster composes on:

| Rule | Why strengthened by G-A |
|------|--------------------------|
| `CN-OPCERT-01` | S2 retires the opcert stub on the node path for the real `parse_opcert_envelope` closed-contract parser. |
| `CN-GENESIS-01` | S2 retires the genesis stub on the node path for the real `parse_shelley_genesis` closed-contract parser. |
| `DC-LEDGER-10` | S2a installs the **current** oracle-bound `ProtocolParameters` (real protocol version / modeled params) into the recovered ledger instead of the defaulted view. |
| `CN-NODE-01` | S2a forge-capable bootstrap binds the current-pparams preimage on the **same** single recovered bootstrap — no second bootstrap. |
| `DC-CINPUT-02b` | S2a threads the hash-bound current-pparams preimage through the consensus-inputs import/merge path. |
| `DC-NODE-05` | S3/S4 forge-tick now fails closed on before-anchor (clock→slot) and off-epoch boundaries — the self-accept-only forge gains two fail-closed boundaries. |
| `DC-NODE-03` | S3 keeps the single `SystemClock` wall-clock seam and routes its output through the checked `checked_millis_to_slot` guard. |

This section is informational and reflects the **committed** registry state at HEAD.
**No rule was removed (expected: 0).**

## 8. Honest residual (cluster scope)

**Forge-fidelity hardening on the relay spine — real config + current pparams + two
new fail-closed boundaries. It does NOT serve, admit, gossip, or advance a durable
tip.**

- **Real constants, not stubs.** The `--mode node` forge path now sources the real
  operator opcert/genesis config (S2, closed cardano-cli parsers; `parse_simple_*`
  retired on the node path) and the **current** oracle-bound `ProtocolParameters` +
  protocol version (S2a, hash-bound preimage), instead of the prior stub/default
  constants. The fix targets the stale `protocol_major = 2` the S2 PO-1 check exposed.
- **Two new fail-closed boundaries.** A before-genesis-anchor candidate slot fails
  closed (`SlotAlignmentError::BeforeGenesisAnchor`, S3) instead of saturating to
  `start_slot`; an off-epoch candidate slot fails closed **before** leadership / KES
  signing (`ForgeEpochAdmission::OffEpoch`, S4) and drives **no** nonce promotion.
- **Self-accept-only, unchanged.** No serve / admit / gossip / broadcast / block-fetch
  / durable-tip claim. The N-F-E containment gate is **byte-unchanged**; the forge
  remains the single fenced `forge_one_from_recovered` call. `run_node_sync →
  pump_block` remains the sole durable tip-advance authority.
- **Forge-CAPABLE but NOT observable.** With no live/continuing feed wired this
  cluster, `run_relay_loop` still halts before any `ForgeTick` on the empty binary
  source (the `On` arm is forge-capable but not observable). **Observable forge / live
  serve / operator-peer ACCEPT / BA-02 / RO-LIVE-01 acceptance is the gated G-B / G-C
  follow-on.** BA-02 is satisfied nowhere.
- **Single bootstrap held (CN-NODE-01).** S2a binds the current-pparams preimage on
  the **same** single recovered bootstrap — no Mithril call, no second bootstrap, no
  second recovered state.
- **No BLUE change.** 456 BLUE canonical types unchanged (Δ0); no new `CoordinatorEvent`
  variant. All code lands in RED `ade_runtime` (`clock` checked guard,
  `consensus_inputs::protocol_params` GREEN parser, `consensus_inputs::canonical`
  accessor), RED `ade_node` (`operator_forge` real parsers, `node_lifecycle` S2/S3
  wiring, `node_sync` S4 epoch guard, `admission::{seed_to_snapshot, bootstrap}` S2a
  install), and GREEN `ade_testkit` (`consensus::genesis_pinning` `#[cfg(test)]`). No
  float path in `protocol_params`; `BTreeMap` / sorted keys / integer-only rational
  arithmetic throughout. The S1 fixture is **evidence input, never runtime authority**.

---

## Generation notes (regen `4eb7610 → 80dac1f7`, PHASE4-N-F-G-A)

- **Baseline is `4eb7610`** (the `.idd-config.json` `head_deltas_baseline` value at
  regen time — the PHASE4-N-F-F slice-span HEAD). **The close-pass commit must bump
  `head_deltas_baseline` to `80dac1f7`** (the G-A HEAD) so the next cluster's
  `/head-deltas` measures from here, and bump the registry-count comment 311 → 313.
- Counts are mechanical: commit log + `--shortstat` over `4eb7610..80dac1f7` (20
  commits incl. 1 merge / 43 files / +5221 / -595); CI gate count via `ls-tree |
  grep ci_check_*.sh` at each ref (112 → 116, +4 new, none removed); registry rule
  count via `grep -c '^\[\[rules\]\]'` at each ref (311 → 313, two IDs added —
  `DC-EPOCH-03` + `DC-NODE-06`, none removed); workspace test attributes via `git
  grep -hE '#\[(tokio::)?test\]'` (2188 → 2213, +25); BLUE canonical types unchanged
  at 456 (no BLUE crate in the diff).
- **ANOMALY surfaced (CODEMAP vs. committed registry).** The already-regenerated
  CODEMAP (HEAD `80dac1f7`) narrates `DC-EPOCH-03` at `status = enforced` and "Seven
  `strengthened_in += "PHASE4-N-F-G-A"` bumps." The **committed registry at the same
  HEAD** shows `DC-EPOCH-03` at `status = "declared"` (`tests = []`, `ci_script = ""`,
  `strengthened_in = []`) and **zero** committed G-A strengthenings. This is the
  standard slice-span-vs-close-pass gap (same pattern N-F-E used for `DC-NODE-05` and
  N-F-F used for `CN-NODE-03`): CODEMAP narrates the **owed end-state**; this
  HEAD_DELTAS narrates **what is committed**. The close-pass that flips `DC-EPOCH-03`
  to `enforced` + records the 7 strengthenings reconciles them. This is **not** a rule
  removal and **not** a discipline violation — it is a sequencing artifact of
  regenerating grounding docs before the close-pass registry edit.
- `DC-EPOCH-03` is `declared` at this HEAD; its `tests`/`ci_script` are empty, so it
  does not yet appear in TRACEABILITY and the four new gates are not yet
  cross-referenced there — the close-pass flips it to `enforced` and refreshes
  TRACEABILITY (and the four grounding docs N-F-F → N-F-G-A). `DC-NODE-06` is a
  forward sketch for G-B and is expected to stay `declared` until that sub-cluster.
