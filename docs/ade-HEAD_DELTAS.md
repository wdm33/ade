# Ade — HEAD Deltas (Changes Since Baseline)

> **Status:** Living architectural document. Regenerated; not hand-edited.
>
> Regenerate with `/head-deltas <baseline>` after every cluster close. Baseline is recorded in `.idd-config.json` `head_deltas_baseline`.

> Baseline: `339cccb1` (here-strings in served-chain handoff fence to avoid pipefail SIGPIPE false-negative, 2026-06-01 22:33)
> HEAD: `90791691` (wire operator peer-log → correlate BA-02 evidence path — PHASE4-N-F-G-C S2, 2026-06-02 01:22)
> Cluster: **PHASE4-N-F-G-C — live WirePump feed + BA-02 operator-pass evidence wiring on the `--mode node` spine**, slice span closed; close-pass commit to follow.
> 7 commits (no merges), 23 files changed, +2348 / -464 lines.

This window narrates the **PHASE4-N-F-G-C cluster** — the third and last of the three
planned PHASE4-N-F-G sub-clusters (G-A forge fidelity / G-B self-accept→serve handoff /
**G-C live operator serve + BA-02 evidence wiring**). G-C **makes the forge-capable
`--mode node` `On` arm live-feed-wireable and wires the operator-pass BA-02 evidence
I/O** — without claiming peer acceptance. **S1** wires a live `NodeBlockSource::WirePump`
feed from the operator-supplied `--peer` by **reusing the closed admission dial+pump
verbatim** (`dial_for_admission` → `run_admission_wire_pump` → `from_wire_pump`), making
`LoopStep::ForgeTick` reachable on the live-wired path; `NodeBlockSource` stays the
**closed 2-variant** (a *fill* of `WirePump`, no new variant), and the empty-`--peer`
path keeps the prior empty source (forge-capable, halts clean). **S2** adds a NEW RED
module `ade_node::ba02_pass` (peer-log file I/O over the GREEN `ba02_evidence::correlate`
— the sole `Ba02Manifest` constructor), a NEW gate that enforces the no-synthetic-manifest
line, and an operator-pass runbook. **NO BLUE crate changed (456 canonical types, Δ0);
no new `CoordinatorEvent` variant; `correlate` stays the sole `Ba02Manifest` ctor.**
Peer ACCEPT is still proven only by the operator-captured peer validation log
(RO-LIVE-06), never by Ade's self-accept / served-block / `ForgeSucceeded` / any
wire-success signal. **The bounty acceptance criterion is NOT satisfied by this cluster
— the remaining step is an operator-witnessed live pass on a peer that can grant
leadership (C1/C2).**

The span also carries the **PHASE4-N-F-G-B close tail** (`febee120`) — docs/registry
only — which lands inside this window because the baseline `339cccb1` is the N-F-G-B
*slice-span* HEAD, not its close commit (see §1). This mirrors how the previous
HEAD_DELTAS window carried the **G-A** close tail (`62cb8718` + `1806584c`) for the same
slice-span-vs-close-commit reason.

## 0. Headline

| Count | Baseline (`339cccb1`) | HEAD (`90791691`) | Δ |
|---|---|---|---|
| CI gates (`ci/ci_check_*.sh`) | 117 | **118** | **+1 new** (`ba02_evidence_manifest_schema`, S2); none removed. `served_chain_handoff_fence` was **broadened in place** by S1 — count-neutral |
| Registry rules | 313 | 313 | **0** (no new ID) |
| Test attributes (`#[test]`/`#[tokio::test]`, workspace) | 2223 | **2230** | **+7** (G-C S1/S2 surface, across `node_lifecycle`/`node_sync`/`forge_succeeds`/`node_operator_pass_ba02`) |
| BLUE canonical types | 456 | 456 | **0 — NO BLUE change** (all code lands in RED `ade_node`) |

> **Registry / grounding-doc sequencing note (load-bearing — read before §7).** This
> window contains **two** distinct close events, and the registry / sibling-doc state
> differs between *committed at HEAD `90791691`* and *the working tree at this regen*:
>
> - **The G-B close-pass (`febee120`) is committed inside this window.** At HEAD
>   `90791691` the registry therefore already reflects the G-B close: `DC-NODE-06` is
>   `enforced` (flipped from `declared` by `febee120`, with its tests + `ci_script`
>   populated) and `RO-LIVE-06` is `enforced` — **but neither yet carries a
>   `strengthened_in += "PHASE4-N-F-G-C"` token** (those are owed at the *next*
>   close-pass; see below). This is the close-pass the **previous** HEAD_DELTAS window
>   flagged as owed, and it is why the rule count holds at 313 rather than rising
>   (`DC-NODE-06` was a pre-existing ID).
> - **The G-C close-pass is NOT yet committed.** At HEAD `90791691` the **four** G-C
>   `strengthened_in += "PHASE4-N-F-G-C"` tokens (`RO-LIVE-01` stays `partial`;
>   `RO-LIVE-06` / `CN-OPERATOR-EVIDENCE-01` / `DC-NODE-06` stay `enforced`) are
>   **uncommitted** — they exist only in the **working tree** at this regen. So is the
>   `RO-LIVE-06.ci_script += "ci/ci_check_ba02_evidence_manifest_schema.sh"` binding
>   (and the same binding on `CN-OPERATOR-EVIDENCE-01`). The pending close-pass commits
>   them — exactly as the N-F-G-B close-pass recorded the G-B strengthenings after its
>   slice span.
> - **Sibling-doc split.** The CODEMAP / SEAMS / TRACEABILITY **committed at HEAD
>   `90791691`** are still the **G-B close** docs (`febee120`): they report **117** CI
>   checks, **2223** tests, and do **not** mention `ba02_pass`. The **working tree** of
>   all three (and the registry) has been **regenerated to the G-C close state** —
>   **118** CI checks, **456** canonical types, **2230** tests, **313** rules, with the
>   new `ba02_pass` locus and the `ba02_evidence_manifest_schema` gate cross-referenced
>   — and they **agree** with this doc's counts. `docs/ade-HEAD_DELTAS.md` and
>   `.idd-config.json` are the only two files this regen / the close-pass still owe; the
>   `.idd-config.json` baseline bump is the closer's edit, not this regen's.
>
> **Not a rule removal, not a discipline violation** — a sequencing artifact reconciled
> by the close-pass.

## 1. Commit Log (newest first)

| Hash | Type | Summary |
|------|------|---------|
| `90791691` | feat | wire operator peer-log → correlate BA-02 evidence path (PHASE4-N-F-G-C S2) |
| `b880c310` | docs | specify PHASE4-N-F-G-C S2 operator evidence |
| `71036d10` | feat | wire live WirePump feed on the --mode node On arm (PHASE4-N-F-G-C S1) |
| `39c18f61` | docs | correct PHASE4-N-F-G-C S1 to conditional live-feed wiring |
| `aa2fe4a8` | docs | specify PHASE4-N-F-G-C S1 live WirePump wiring |
| `aebe913f` | docs | define PHASE4-N-F-G-C live evidence cluster |
| `febee120` | docs | close PHASE4-N-F-G-B — self-accept→serve handoff (DC-NODE-06 enforced) |

No merge commits in the span.

The **last-listed** commit (`febee120`) is the **PHASE4-N-F-G-B close-pass** that lands
inside this window because the baseline `339cccb1` is the N-F-G-B *slice-span* HEAD, not
its close commit: `febee120` is the G-B close-pass (registry `DC-NODE-06`
`declared → enforced` + the 2 G-B strengthenings `CN-PROD-04` / `CN-CONS-07` + a
four-doc grounding-doc refresh CODEMAP/SEAMS/TRACEABILITY/HEAD_DELTAS + the
`.idd-config.json` baseline bump `80dac1f7 → 339cccb1` + the G-B cluster-doc archive).
It is **docs/registry only** — no source change, and **count-neutral on CI** (117 → 117;
the G-B gate `served_chain_handoff_fence` was already committed at the baseline
`339cccb1`). `aebe913f` defines the G-C cluster; everything from `aa2fe4a8` onward is
G-C proper (S1 doc → correction → impl, then S2 doc → impl — doc-then-impl per slice).

(Plus the pending close-pass commit: grounding-doc reconciliation of the committed
CODEMAP/SEAMS/TRACEABILITY from the G-B close to the G-C working-tree state + the four
`strengthened_in += "PHASE4-N-F-G-C"` registry tokens + the two `ci_script` bindings on
`RO-LIVE-06` / `CN-OPERATOR-EVIDENCE-01` + `.idd-config.json` baseline bump
`339cccb1 → 90791691` + the G-C cluster-doc archive + this HEAD_DELTAS.)

## 2. New Modules

One new source module — a RED file-I/O shim in the RED `ade_node` crate. No new crate,
no new BLUE authority, no new WAL/checkpoint/canonical type, no new `CoordinatorEvent`
variant, no new GREEN evidence reducer (`ba02_evidence::correlate` is reused unchanged).

| Module | Color | Purpose | Key sub-paths | Added in |
|--------|-------|---------|---------------|----------|
| `ade_node::ba02_pass` | **RED** | RED operator-pass BA-02 evidence **file I/O only**. Reads the operator-captured peer-accept JSONL log from disk and runs it through the GREEN `crate::ba02_evidence` reducer (`parse_peer_accept_events` allow-list → `correlate`, the **sole** `Ba02Manifest` constructor). It **constructs no evidence, derives no acceptance, and never coerces a non-acceptance line** — `correlate` stays the only authority; a `Ba02Manifest` is a *claim about* authority, not authority (`[[feedback-evidence-reducers-are-green-not-authority]]`). The honesty line is enforced by construction: the **only** input that can yield a manifest is a real operator-captured peer log naming the **exact** forged hash, through `correlate`; a missing/unreadable peer-log file fails closed (`io::Error`), **never** a synthesized acceptance (`[[feedback-shell-must-not-overstate-semantic-truth]]`). | `correlate_peer_log_file(&AdeForgeRecord, &Path) -> io::Result<BA02Outcome>` — file bytes → `parse_peer_accept_events` → `correlate`; a missing/unreadable file is an `io::Error` (fail-closed), **not** a `NoEvidence` and **not** a manifest. `write_ba02_manifest(&Ba02Manifest, &Path) -> io::Result<()>` — the **argument type is the gate**: only a `Ba02Manifest` (which only `correlate`'s exact-match arm constructs) is writable, so a written manifest is **always** `correlate`-produced (no path emits a manifest from `NoEvidence` or from raw operator input). RED I/O only: `std::fs` read/write; no clock/rand/float; constructs no evidence and re-validates nothing. `#[cfg(test)]` proofs `correlate_wired_to_operator_peer_log` + `correlate_from_operator_log_file_is_deterministic` live in `tests/node_operator_pass_ba02.rs`. | `PHASE4-N-F-G-C` S2 (`90791691`) |

Cross-reference: the new module is in CODEMAP §RED and SEAMS in the **working tree**
(both regenerated to the G-C close state at `90791691` — `ba02_pass` is present and the
`code_locus` `crates/ade_node/src/ba02_pass.rs` resolves), but is **not yet** in the
CODEMAP/SEAMS **committed at HEAD `90791691`** — those committed copies are still the
**G-B close** docs (`febee120`). This is the sibling-doc split called out in §0; the
close-pass commits the working-tree CODEMAP/SEAMS/TRACEABILITY alongside this doc. (Not
a staleness *defect* — a sequencing artifact reconciled by the close-pass.)

## 3. Modules Modified

All modified source files existed at baseline. Trivial/no-behavioral-effect changes are
skipped (the one-line `pub mod ba02_pass;` registration in `ade_node/src/lib.rs`, and
the one-line `state.tip.as_ref()` accessor adjustment in
`ade_node/src/admission/bootstrap.rs` so the live-feed start-point reads the recovered
tip).

| Module | Color | Scope | Key changes |
|--------|-------|-------|-------------|
| `ade_node::node_lifecycle` | **RED** | +159 / -? lines (incl. `#[cfg(test)]`) | **S1 (`71036d10`) — wire the live WirePump feed on the `--mode node` `On` arm.** New `spawn_live_wire_pump_source(peer_addrs, network_magic, recovered_tip) -> NodeBlockSource` (+ `const LIVE_WIRE_PUMP_CHANNEL_CAP = 64`): it builds a LIVE `NodeBlockSource::WirePump` by **reusing the closed admission dial+pump VERBATIM** (`dial_for_admission` → `run_admission_wire_pump`, no reimplementation, no new wire authority); the runtime `ade_runtime::admission::AdmissionPeerEvent` output feeds the `WirePump` arm **directly** (the node spine consumes the runtime event type — no bridge). The `On` arm wires it iff `--peer` is supplied (`live_feed_wired = !cli.peer_addrs.is_empty()`; requires `--network-magic`, else `MissingFlag`); empty `--peer` keeps the prior `NodeBlockSource::in_memory(Vec::new())` (forge-CAPABLE, halts clean). The start point is the recovered `ChainTip` (`Point::Block` / `Point::Origin`). **Honest-scope C3** (mirrors `admission::bootstrap::spawn_wire_pumps_for_admission`): an unparseable `--peer` addr or a `dial_for_admission` failure is **logged-and-dropped — never fatal, never a fabricated address, never a silent tip graft**; if no peer yields a live pump, the feed ends and the relay loop halts clean (same outcome as the empty source). The honest end-of-run log split into a `live_feed_wired` branch (forge observable when the feed is Continuing + a due leader slot is reached — peer ACCEPT **not** claimed, operator-gated RO-LIVE-01/06) vs the empty branch (forge-CAPABLE, halts clean). The single `SystemClock` wall-clock seam (DC-NODE-03) and the `Off`/`On` dispatch shape are unchanged; the durable tip still advances only via `run_node_sync → pump_block` (no second tip-advance, no verdict). New `#[cfg(test)]` proof `spawn_live_wire_pump_source_with_no_usable_peer_yields_ended_feed` (empty / unparseable `--peer` → already-closed feed → `next_block()` is `None`). |
| `ade_node::node_sync` | **RED** (host of the closed `NodeBlockSource` + GREEN-pure `forge_epoch_admission` + the fenced BLUE forge composition) | +98 lines (all `#[cfg(test)]`) | **S1 (`71036d10`) — consume-side proofs that the live feed makes the forge observable.** No production-body change in this file — the surface that S1 exercises (`NodeBlockSource::from_wire_pump` / `in_memory`, the closed 2-variant enum, `run_relay_loop`) already existed. New `#[cfg(test)]` proofs: `node_block_source_stays_closed_two_variant` (an exhaustive match with **no** wildcard arm pins `NodeBlockSource` as the closed `{WirePump, InMemory}` — a third "alternative live source" variant would fail to compile, so the live feed is a **fill**, not a plugin point); and `live_wire_pump_feed_reaches_forge_tick` (same recovered base / keys / clock / schedule; the **only** difference is source liveness — a Continuing `WirePump` feed makes `LoopStep::ForgeTick` reachable while the empty `InMemory` source halts before any `ForgeTick`, isolating exactly the live-feed effect; forge stays **subordinate** to the feed per CN-NODE-02 / DC-NODE-05). |
| `ade_node::admission::bootstrap` | **RED** | +2 / -? lines | **S1 (`71036d10`) — visibility promotion + start-point read.** `build_n2n_version_table` promoted `fn` → `pub(crate) fn` so `node_lifecycle::spawn_live_wire_pump_source` reuses the closed N2N version table **verbatim** (no reimplementation). No behavioral change to the admission bootstrap path itself. |

## 4. Feature Flags

**No project feature-flag deltas.** Ade declares no `[features]` table in any workspace
`Cargo.toml` (confirmed at both refs — the table is absent), and no `#[cfg(feature =
…)]` gate was introduced in the span. **No `Cargo.toml` change at all in this window.**
No coupling, no `compile_error!` guard.

## 5. CI Checks (117 → 118; +1 new, 1 broadened-in-place, 0 removed)

One new gate plus one broadened-in-place gate, both repo-root-relative and mirroring the
existing `ci/ci_check_*.sh` convention.

### PHASE4-N-F-G-C gates (from baseline through HEAD)

| Check | Status | Cluster origin | What it checks |
|-------|--------|----------------|----------------|
| `ci_check_ba02_evidence_manifest_schema.sh` | **New** | S2 (`90791691`) | Backs the **RO-LIVE-06** BA-02-evidence schema clause (mirrors `ci_check_operator_evidence_manifest_schema.sh`). **Vacuous-until-committed**: when no `docs/clusters/PHASE4-N-F-G-C/CE-G-C-LIVE_*.toml` is present (the typical state — the live operator pass is `blocked_until_operator_stake_available`), the gate passes (**no manifest, no claim**). When a manifest *is* committed, it enforces (1) all 8 required schema fields present (`schema_version`, `block_hash`, `slot`, `peer_log_file`, `peer_log_file_sha256`, `peer_log_capture_command`, `peer_log_filter`, `accept_event_kind`); (2) `schema_version == 1` (the canonical `BA02_MANIFEST_SCHEMA_VERSION`); (3) **`peer_log_file_sha256` matches the actual SHA-256 of the committed peer-log fixture** — the **no-synthetic-manifest** line: a hand-authored manifest with no real fixture, or a tampered fixture, **FAILS**. The complementary provenance fence is in code (`ba02_pass::write_ba02_manifest` accepts only a `Ba02Manifest`, which only `ba02_evidence::correlate`'s exact-match arm constructs). Hermetic — no Docker / cardano-cli / live node. |
| `ci_check_served_chain_handoff_fence.sh` | **Modified (broadened in place)** | S1 (`71036d10`); introduced N-F-G-B S3 | Still backs the **DC-NODE-06** serve-ingress clause. **Broadened** for the live-feed path: (a) the scope grew from the single `node_lifecycle.rs` owner to the node-spine serve **owner set** `{node_lifecycle.rs, node_sync.rs}` (so the fence still holds if the serve wiring moves between them; the `--mode produce` path `produce_mode.rs` / CN-PROD-04 stays a **separate** serve authority + gate, deliberately out of scope here); (b) **guard-3 became an allow-list** (was a 3-name deny-list): **every** node-spine unbounded handoff channel (`UnboundedSender<…>` / `UnboundedReceiver<…>` / `unbounded_channel::<…>`) MUST carry `SelfAcceptedHandoff` — **any** other payload fails, not just the three previously named (`<Vec<u8>>` / `<ForgedBlockArtifact>` / `<bool>`) — and at least one `UnboundedSender<SelfAcceptedHandoff>` must exist. The new bounded live-feed channel (`mpsc::channel::<AdmissionPeerEvent>`) is **not** a handoff channel and is intentionally not matched. Guards (1) (every node-spine `push_atomic(` fed by `into_accepted()`) and (2) (no direct `served_chain_admit(` on the node spine) are unchanged. |

The N-F-E forge-containment gate (`ci_check_node_run_loop_containment.sh`) is
**byte-unchanged** — the relay-loop body still performs no serve / admit / gossip /
broadcast / block-fetch / durable-tip mutation. G-C **added** an evidence-schema gate
and **broadened** the handoff fence's reach, but did **not** relax containment.

> Cross-reference (TRACEABILITY): in the **working tree** at `90791691`,
> `ci_check_ba02_evidence_manifest_schema.sh` is cited by `RO-LIVE-06.ci_script`
> **and** `CN-OPERATOR-EVIDENCE-01.ci_script`, and TRACEABILITY (working tree) renders
> those rows with the gate present (14 working-tree TRACEABILITY mentions of the gate
> name). At the **committed** HEAD `90791691`, the registry already binds
> `ci_check_served_chain_handoff_fence.sh` + `ci_check_node_run_loop_containment.sh`
> to `DC-NODE-06` (from the G-B close `febee120`), but the **new** S2 gate's
> `ci_script` binding on `RO-LIVE-06` / `CN-OPERATOR-EVIDENCE-01` is **working-tree
> only** (uncommitted) — the close-pass commits it and regenerates the committed
> TRACEABILITY. See the warnings in the generation section.

## 6. Canonical Type Registry Delta

**n/a — no separate canonical-type registry is configured** (`canonical_type_registry: null`),
and **no BLUE crate changed**. The 456 BLUE canonical-type total is **unchanged (Δ0)**
across the span (independently re-verified: `git diff --name-only 339cccb1..90791691`
matches **no** `core_paths` (BLUE) entry — every changed source file is RED `ade_node`;
the working-tree CODEMAP at `90791691` reports 456 canonical). The new `ba02_pass`
module is RED file I/O in `ade_node` and is not canonical-counted; it reuses the existing
GREEN `ade_node::ba02_evidence` types (`AdeForgeRecord`, `BA02Outcome`, `Ba02Manifest`)
verbatim — no new type. **No new `CoordinatorEvent` variant or field was introduced.**

## 7. Normative / Invariant Rule Delta (313 → 313)

**No rule ID was added or removed in the span** (313 at both refs). G-C ships **no new
rule** — it extends existing rules. This window also commits the **G-B** close-pass
registry edits (`febee120`), which is why the count holds at 313 (`DC-NODE-06` was a
pre-existing ID, not a new one).

### G-B close-pass registry edits (committed in this span, by `febee120`)

For completeness — these were the *owed* edits flagged in the **previous** (N-F-G-B)
HEAD_DELTAS window and are **committed here**:

- `DC-NODE-06` flipped `declared → enforced` (`tests` populated with the 10 G-B
  handoff/serve tests; `ci_script = "ci/ci_check_served_chain_handoff_fence.sh,
  ci/ci_check_node_run_loop_containment.sh"`).
- 2 G-B strengthenings recorded: `CN-PROD-04` (`strengthened_in += "PHASE4-N-F-G-B"`)
  and `CN-CONS-07` (`strengthened_in += "PHASE4-N-F-G-B"`).

### G-C strengthenings (owed at close, NOT yet committed)

At HEAD `90791691` **zero** `strengthened_in += "PHASE4-N-F-G-C"` tokens are committed —
they exist only in the **working tree** at this regen. The pending close-pass records the
**4 cross-rule strengthenings** the cluster extends, plus the new S2 gate's `ci_script`
bindings:

| Rule | Status (held) | Why strengthened by G-C |
|------|---------------|--------------------------|
| `RO-LIVE-01` | **`partial`** (held — `blocked_until_operator_stake_available`) | G-C wired the **MECHANICAL** half on the `--mode node` spine: a live `NodeBlockSource::WirePump` feed makes `LoopStep::ForgeTick` reachable (`live_wire_pump_feed_reaches_forge_tick`) and the forge-derived self-accepted block is served byte-identically over in-process block-fetch (`live_feed_forge_serve_loopback_returns_forged_block`). The **LIVE** half is unchanged: peer ACCEPT stays `blocked_until_operator_stake_available`, proven **only** by the operator-captured peer log through `correlate`. |
| `RO-LIVE-06` | **`enforced`** (held) | The operator-pass BA-02 evidence path is now wired: `ba02_pass::correlate_peer_log_file` reads the operator-captured peer log and runs it through `correlate`; the new `ci_check_ba02_evidence_manifest_schema.sh` gate (added to `ci_script`) enforces schema + sha256-binding on any committed manifest. **Live BA-02 is still NOT claimed** — schema + mechanics only. |
| `CN-OPERATOR-EVIDENCE-01` | **`enforced`** (held) | The new BA-02 manifest-schema gate is added to its `ci_script` set (alongside `ci_check_operator_evidence_manifest_schema.sh`) — the operator-evidence manifest discipline now also covers the `--mode node` BA-02 manifest family (`CE-G-C-LIVE_*.toml`). |
| `DC-NODE-06` | **`enforced`** (held; flipped by the G-B close `febee120` earlier in this window) | The served-chain handoff fence it binds (`ci_check_served_chain_handoff_fence.sh`) was **broadened in place** to the node-spine owner set + an allow-list guard-3 — strengthening (never relaxing) the serve-ingress clause as the live feed is wired. |

This section is informational and reflects the **committed** registry state at HEAD
(`DC-NODE-06` `enforced`, `RO-LIVE-06` `enforced`, both **without** a G-C token yet;
`RO-LIVE-01` `partial`). **No rule was removed (expected: 0).**

## 8. Honest residual (cluster scope)

**G-C closes the MECHANICAL live-feed + evidence scaffolding ONLY. The forge-capable
`--mode node` `On` arm is now live-feed-wireable, and the operator-pass BA-02 evidence
I/O is wired — but peer ACCEPT is NOT claimed, and the bounty acceptance criterion is
NOT satisfied.**

- **Live feed, not acceptance.** S1 wires a live `NodeBlockSource::WirePump` from
  `--peer` by **reusing** the closed admission dial+pump verbatim; a Continuing feed
  makes `LoopStep::ForgeTick` reachable (the empty source halts before any `ForgeTick`).
  `NodeBlockSource` stays the **closed 2-variant** (a fill of `WirePump`); the durable
  tip still advances only via `run_node_sync → pump_block` (no second tip-advance, no
  verdict). Empty `--peer` preserves the prior forge-CAPABLE, halts-clean contract;
  dial/parse failures are logged-and-dropped (C3), never fatal, never a fabricated
  address, never a silent tip graft.
- **Self-accept / served-block / wire-success ≠ peer acceptance.** A live wire feed and
  an in-process served-block loopback prove the serve **mechanism**, not acceptance.
  The bounty acceptance criterion (an operator-witnessed accepted block on a peer that
  can grant leadership) is **NOT** satisfied by this cluster.
- **`correlate` is the sole evidence authority.** S2's `ba02_pass` is RED file I/O over
  the GREEN `ba02_evidence::correlate` — the **sole** `Ba02Manifest` constructor. It
  constructs no evidence, derives no acceptance, and never coerces a non-acceptance
  line; a missing/unreadable peer log fails closed (`io::Error`), never a synthesized
  acceptance. `write_ba02_manifest` accepts only a `Ba02Manifest`, so a written manifest
  is **always** correlate-produced. **No synthetic manifest is committed** — the
  `ci_check_ba02_evidence_manifest_schema.sh` gate is vacuously satisfied until an
  operator commits a real `CE-G-C-LIVE_*.toml` bound to a real peer-log fixture by
  sha256.
- **Peer ACCEPT operator-gated.** `RO-LIVE-01` stays `partial` /
  `blocked_until_operator_stake_available`; `RO-LIVE-06` enforces **schema + mechanics
  only** — **no live BA-02 claim**. The remaining step is an operator-witnessed live
  pass on a peer that can grant leadership: **C1** (private testnet — Ade holds ~all
  stake, the cheapest real ACCEPT) or **C2** (preprod — needs ~2 epochs of provisioned
  active stake; the public preprod docker peer **cannot** grant acceptance — Ade has no
  stake there). See `docs/evidence/phase4-n-f-g-c-operator-pass-README.md` (the new
  operator-pass runbook; supersedes the stale N-S-C S1 runbook and closes the
  C1-scoping-doc bugs).
- **No BLUE change.** 456 BLUE canonical types unchanged (Δ0); no new `CoordinatorEvent`
  variant or field. All code lands in RED `ade_node` (`node_lifecycle`
  `spawn_live_wire_pump_source` + the live-feed `On`-arm wiring; `node_sync` consume-side
  `#[cfg(test)]` proofs; the new RED `ba02_pass` file-I/O shim; `bootstrap`
  visibility promotion). No new GREEN evidence reducer — `ba02_evidence::correlate` is
  reused unchanged.
- **New MEDIUM follow-on (exposed, not introduced here).** The live feed newly exposes
  the WirePump's bounded mux-reassembly tail and its lookahead on the `--mode node`
  spine (reused admission infra — **not** introduced by G-C). This is a MEDIUM
  follow-on for a dedicated slice (bounded mux-reassembly tail hardening + WirePump
  lookahead behavior on the live feed), tracked alongside the operator-witnessed live
  pass.

---

## Generation notes (regen `339cccb1 → 90791691`, PHASE4-N-F-G-C)

- **Baseline is `339cccb1`** (the `.idd-config.json` `head_deltas_baseline` value at
  regen time — the PHASE4-N-F-G-B slice-span HEAD). **The close-pass commit must bump
  `head_deltas_baseline` to `90791691`** (the G-C slice-span HEAD) so the next cluster's
  `/head-deltas` measures from here. The registry-count comment stays 313 (no ID added
  or removed across the window). The `.idd-config.json` edit itself is handled by the
  closer, not this regen.
- Counts are mechanical (git/grep/ls only, no cargo): commit log + `--shortstat` over
  `339cccb1..90791691` (7 commits, no merges / 23 files / +2348 / -464); CI gate count
  via `git ls-tree -r --name-only <ref> ci/ | grep -E 'ci_check_.*\.sh' | wc -l` at each
  ref (117 → 118, **+1 new** — `ba02_evidence_manifest_schema` at S2, none removed; the
  G-B gate `served_chain_handoff_fence` already existed at the baseline `339cccb1` and
  was **broadened in place** by S1, so the net count change is the single S2 add);
  registry rule count via `grep -c '^id = '` (and `grep -c '^\[\[rules\]\]'`) at each ref
  (313 → 313, no ID added or removed); workspace test attributes via `git grep -hE
  '#\[(tokio::)?test\]'` (2223 → 2230, +7); BLUE canonical types unchanged at 456 (no
  BLUE crate file in the diff — verified: `git diff --name-only` matches no `core_paths`
  entry).
- **Two close events in one window (NOT an anomaly).** The **G-B** close-pass
  (`febee120`) is **committed** inside this window (because the baseline `339cccb1` is
  the G-B slice-span HEAD, not its close commit) — it flipped `DC-NODE-06`
  `declared → enforced`, recorded the 2 G-B strengthenings, and refreshed the four
  grounding docs to the **G-B** close. The **G-C** close-pass is **not yet committed**.
  So at HEAD `90791691`: the registry has `DC-NODE-06` / `RO-LIVE-06` `enforced` (G-B
  state) but **none** of the four G-C `strengthened_in` tokens; and the **committed**
  CODEMAP / SEAMS / TRACEABILITY are still the **G-B** close docs (117 CI, 2223 tests,
  no `ba02_pass`).
- **Sibling-doc coherence (working tree).** The CODEMAP / SEAMS / TRACEABILITY **working
  tree** (and the registry) have already been regenerated to the **G-C** close state —
  **118** CI checks (matches the on-disk `ls ci/ci_check_*.sh | wc -l = 118`), **456**
  canonical types, **2230** tests, **313** rules, with the new `ba02_pass` locus and the
  `ba02_evidence_manifest_schema` gate cross-referenced — and they **agree** with this
  doc's counts. `git status` shows `docs/ade-CODEMAP.md`, `docs/ade-SEAMS.md`,
  `docs/ade-TRACEABILITY.md`, and `docs/ade-invariant-registry.toml` modified
  (working-tree close-pass edits already applied); `docs/ade-HEAD_DELTAS.md` and
  `.idd-config.json` are the two files this regen / the close-pass still owe.
- **CI cross-reference warning.** `ci_check_ba02_evidence_manifest_schema.sh` is cited by
  `RO-LIVE-06` / `CN-OPERATOR-EVIDENCE-01` only in the **working-tree** registry
  (uncommitted at this HEAD); the **committed** TRACEABILITY at `90791691` is the G-B
  close doc and does not yet show it. The close-pass commits the `ci_script` bindings and
  regenerates the committed TRACEABILITY. (The G-B gate `served_chain_handoff_fence` +
  the containment gate are already cited by `DC-NODE-06` in the committed registry, via
  `febee120`.)
- **Not a rule removal, not a discipline violation** — the working-tree-vs-committed
  split is a sequencing artifact reconciled by the close-pass (which commits the four
  G-C `strengthened_in` tokens, the two new `ci_script` bindings, the
  CODEMAP/SEAMS/TRACEABILITY working-tree regen, the `.idd-config.json` baseline bump
  `339cccb1 → 90791691`, the G-C cluster-doc archive, and this HEAD_DELTAS). The next
  sub-cluster work is the **operator-witnessed live pass** (C1/C2) that flips
  `RO-LIVE-01` off `partial` and produces a real BA-02 manifest — plus the MEDIUM
  mux-reassembly-tail / WirePump-lookahead follow-on newly exposed on the live feed.
