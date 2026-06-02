# Ade — HEAD Deltas (Changes Since Baseline)

> **Status:** Living architectural document. Regenerated; not hand-edited.
>
> Regenerate with `/head-deltas <baseline>` after every cluster close. Baseline is recorded in `.idd-config.json` `head_deltas_baseline`.

> Baseline: `90791691` (wire operator peer-log → correlate BA-02 evidence path — PHASE4-N-F-G-C S2, 2026-06-02 01:22)
> HEAD: `6f848825` (bound live-feed memory before authoritative decode — PHASE4-N-F-G-E S1, 2026-06-02 12:48)
> Cluster: **PHASE4-N-F-G-E — live-feed bounded memory before authoritative decode/apply on the `--mode node` spine**, slice span closed; close-pass commit to follow.
> 3 commits (no merges), 17 files changed, +2120 / -1109 lines.

This window narrates the **PHASE4-N-F-G-E cluster** — an operational-hardening cluster
that **bounds peer-driven memory on the live `--mode node` feed BEFORE the authoritative
BLUE decode/apply path**, with **NO BLUE crate changed**. The cluster is a single slice
(S1) prompted by the PHASE4-N-F-G-C per-cluster security review (MEDIUM) and SEAMS §7
candidate #6: the G-C live-feed wiring **exposed** two pre-existing unbounded peer-driven
memory surfaces (reused N-M-FRAG / N-M-C infra) on the binary path — it did not introduce
them. G-E caps both, each **fail-closed before the BLUE `ade_codec` decode path**:

- **GREEN `ade_network::session::core`** gains a closed const `MAX_REASSEMBLY_TAIL_BYTES =
  16 * 1024 * 1024` (16 MiB). After `drain_protocol_items` drains every COMPLETE item, an
  incomplete per-mini-protocol reassembly *tail* `> cap` returns the NEW additive closed
  variant `SessionError::ReassemblyBufferOverflow { protocol, len, cap }` — **drop the
  peer** (no silent truncation, no partial decode, no unbounded fallback).
- **RED `ade_node::node_sync`** gains a closed const `MAX_WIRE_PUMP_LOOKAHEAD = 256`. The
  `pump_lookahead` opportunistic `try_recv` drain stops at the cap so the existing bounded
  `mpsc` (`LIVE_WIRE_PUMP_CHANNEL_CAP = 64`) **back-pressures** the pump.
- **RED `ade_runtime::network::mux_pump`** maps the new `SessionError` variant
  (`session_err_to_halt`) → `PeerHaltReason::ChainSyncDecodeError` (drop the peer).

The bounds are **CLOSED CONSTANTS** — defensive implementation bounds, not Cardano
semantic parameters; **no runtime option (CLI / env / config) may disable them or set them
unbounded**, enforced by the NEW gate `ci_check_live_feed_memory_bounds.sh`. The new rule
`DC-LIVEMEM-01` (`tier = derived`, operational-hardening) is **enforced** at this close.
**The bounty acceptance criterion is NOT satisfied by this cluster** — the claim is narrow
(memory bounded before authoritative decode), and the operator-witnessed live pass remains
the gating follow-on.

The span also carries the **PHASE4-N-F-G-C close tail** (`351d46bc`) — docs/registry only
— which lands inside this window because the baseline `90791691` is the N-F-G-C
*slice-span* HEAD, not its close commit (see §1). This mirrors how the previous
HEAD_DELTAS window carried the **G-B** close tail (`febee120`), and the one before that the
**G-A** tail (`62cb8718` + `1806584c`), for the same slice-span-vs-close-commit reason.

## 0. Headline

| Count | Baseline (`90791691`) | HEAD (`6f848825`) | Δ |
|---|---|---|---|
| CI gates (`ci/ci_check_*.sh`) | 118 | **119** | **+1 new** (`live_feed_memory_bounds`, G-E S1); none removed, none modified |
| Registry rules | 313 | **314** | **+1** (`DC-LIVEMEM-01`, G-E); plus the 4 G-C-close `strengthened_in` tokens land in this window (no ID added by them) |
| Test attributes (`#[test]`/`#[tokio::test]`, workspace) | 2230 | **2234** | **+4** (the four `DC-LIVEMEM-01` tests, across `session::core` + `node_sync`) |
| BLUE canonical types | 456 | 456 | **0 — NO BLUE change** (`session/` is GREEN-by-content, not a BLUE `ade_network` submodule; the other two changed files are RED) |

> **Registry / grounding-doc sequencing note (load-bearing — read before §7).** This
> window contains **two** distinct close events, and the registry / sibling-doc state
> differs between *committed at HEAD `6f848825`* and *the working tree at this regen*:
>
> - **The G-C close-pass (`351d46bc`) is committed inside this window.** At HEAD
>   `6f848825` the registry therefore already reflects the G-C close: the **four** G-C
>   `strengthened_in += "PHASE4-N-F-G-C"` tokens (`RO-LIVE-01` stays `partial`;
>   `RO-LIVE-06` / `CN-OPERATOR-EVIDENCE-01` / `DC-NODE-06` stay `enforced`) are
>   **committed**, as is the G-C gate `ci_check_ba02_evidence_manifest_schema.sh`. (That
>   gate was committed at the **baseline** `90791691` itself — G-C S2 — so it does **not**
>   appear as a window-add; see the CI lineage in §5.) This is the close-pass the
>   **previous** HEAD_DELTAS window flagged as owed at its `90791691` HEAD.
> - **The G-E close-pass is NOT yet committed.** At HEAD `6f848825` the new rule
>   `DC-LIVEMEM-01` is committed as `status = "declared"` (its `tests` + `ci_script` are
>   populated by the S1 impl) — but the **flip to `enforced`** lives only in the **working
>   tree** at this regen. So does the working-tree regeneration of CODEMAP / SEAMS /
>   TRACEABILITY to the G-E close state. The pending close-pass commits the flip and the
>   three siblings — exactly as the N-F-G-C close-pass committed the G-C strengthenings
>   after its slice span.
> - **Sibling-doc split.** The CODEMAP / SEAMS / TRACEABILITY **committed at HEAD
>   `6f848825`** are still the **G-C close** docs (`351d46bc`): they report **118** CI
>   checks, **2230** tests, **313** rules, and do **not** mention `DC-LIVEMEM-01` or
>   `ci_check_live_feed_memory_bounds.sh`. The **working tree** of all three (and the
>   registry) has been **regenerated to the G-E close state** — **119** CI checks, **456**
>   canonical types, **2234** tests, **314** rules, with `DC-LIVEMEM-01` `enforced`, the
>   new `session::core` / `node_sync` caps, and the new gate cross-referenced — and they
>   **agree** with this doc's counts (CODEMAP/SEAMS headers: "119 CI checks at HEAD
>   (`6f848825`, PHASE4-N-F-G-E cluster close)"). `docs/ade-HEAD_DELTAS.md` and
>   `.idd-config.json` are the only two files this regen / the close-pass still owe; the
>   `.idd-config.json` baseline bump is the closer's edit, not this regen's.
>
> **Not a rule removal, not a discipline violation** — a sequencing artifact reconciled
> by the close-pass.

## 1. Commit Log (newest first)

| Hash | Type | Summary |
|------|------|---------|
| `6f848825` | feat | bound live-feed memory before authoritative decode (PHASE4-N-F-G-E S1) |
| `5a4d8c12` | docs | define PHASE4-N-F-G-E live-feed bounded-memory cluster + S1 |
| `351d46bc` | (close) | Close PHASE4-N-F-G-C — live feed + operator-gated evidence |

No merge commits in the span.

The **last-listed** commit (`351d46bc`) is the **PHASE4-N-F-G-C close-pass** that lands
inside this window because the baseline `90791691` is the N-F-G-C *slice-span* HEAD, not
its close commit: `351d46bc` flipped the **four** G-C cross-rule strengthenings
(`RO-LIVE-01` stays `partial` / `blocked_until_operator_stake_available`; `RO-LIVE-06` /
`CN-OPERATOR-EVIDENCE-01` / `DC-NODE-06` stay `enforced` — each gains
`strengthened_in += "PHASE4-N-F-G-C"`), bound the G-C gate
`ci_check_ba02_evidence_manifest_schema.sh` to `RO-LIVE-06` / `CN-OPERATOR-EVIDENCE-01`,
regenerated the four grounding docs to the **G-C** close, bumped the `.idd-config.json`
baseline `339cccb1 → 90791691`, and archived the G-C cluster. It is **docs/registry only**
— no source change — and **count-neutral on CI** (118 → 118; the G-C gate
`ba02_evidence_manifest_schema` was already committed at the baseline `90791691`, at G-C
S2, so the close added no gate). `5a4d8c12` defines the G-E cluster + S1; `6f848825` is the
G-E S1 implementation (doc-then-impl per slice).

(Plus the pending close-pass commit: the `DC-LIVEMEM-01` `declared → enforced` flip, the
grounding-doc commit of the working-tree CODEMAP/SEAMS/TRACEABILITY G-E regen, the
`.idd-config.json` baseline bump `90791691 → 6f848825`, the G-E cluster-doc archive, and
this HEAD_DELTAS.)

## 2. New Modules

**None.** G-E adds **no new source module, no new crate, no new BLUE authority, no new
WAL/checkpoint/canonical type, no new `CoordinatorEvent` variant, no new GREEN evidence
reducer.** The cluster lands as two closed memory **caps** added to **existing** modules,
plus one additive closed enum variant and its single halt-mapping arm:

- a closed const in the existing GREEN `ade_network::session::core`,
- a closed const in the existing RED `ade_node::node_sync`,
- one additive variant on the existing closed enum `SessionError` in
  `ade_network::session::event`,
- one additive match arm in the existing RED `ade_runtime::network::mux_pump`.

See §3 for the per-module change detail.

## 3. Modules Modified

All four modified source files existed at baseline. There are no trivial/skipped source
changes in this window — every changed source file carries the bounded-memory hardening.

| Module | Color | Scope | Key changes |
|--------|-------|-------|-------------|
| `ade_network::session::core` | **GREEN-by-content** (the `session/` subtree is **not** one of the 9 BLUE `ade_network` submodule `core_paths`; deterministic GREEN session reducer) | +110 lines (incl. `#[cfg(test)]`) | **G-E S1 (`6f848825`) — reassembly-tail cap.** New closed const `MAX_REASSEMBLY_TAIL_BYTES = 16 * 1024 * 1024` (16 MiB). In `step` / `drain_protocol_items`: after every **COMPLETE** item is drained, if a per-mini-protocol reassembly **tail** still buffered satisfies `buf.len() > MAX_REASSEMBLY_TAIL_BYTES` the reducer returns the NEW additive `SessionError::ReassemblyBufferOverflow { protocol, len, cap }` (**fail closed** — drop the peer; **no** silent truncation, **no** partial decode, **no** unbounded fallback). Pure (`buf.len()` comparison only; the GREEN no-clock/rand/float/`HashMap` contract is preserved). The cap fires **before** the BLUE `ade_codec` decode path (unchanged). New `#[cfg(test)]` proofs `session_reassembly_tail_over_cap_fails_closed` (tail `> cap` → `ReassemblyBufferOverflow`) and `session_reassembly_tail_under_cap_still_drains_complete_item` (a complete item under the cap still drains intact — the cap does not regress the happy path). |
| `ade_network::session::event` | **GREEN-by-content** (same `session/` subtree) | +12 lines | **G-E S1 (`6f848825`) — additive closed-enum variant.** `SessionError` gains the closed variant `ReassemblyBufferOverflow { protocol, len, cap }` (no wildcard arm anywhere; the enum stays exhaustively matched). The cap is carried on the variant so the halt reason is self-describing. Its sole exhaustive consumer is `ade_runtime::network::mux_pump::session_err_to_halt` (see below). |
| `ade_node::node_sync` | **RED** (host of the closed `NodeBlockSource` + GREEN-pure `forge_epoch_admission` + the fenced BLUE forge composition; the file itself is RED) | +79 lines (incl. `#[cfg(test)]`) | **G-E S1 (`6f848825`) — WirePump lookahead cap (bounded-channel back-pressure).** New closed const `MAX_WIRE_PUMP_LOOKAHEAD = 256`. `pump_lookahead` stops the opportunistic `try_recv` drain once `lookahead.len() >= MAX_WIRE_PUMP_LOOKAHEAD`, so the existing bounded `mpsc` (`LIVE_WIRE_PUMP_CHANNEL_CAP = 64`) **back-pressures** the upstream pump instead of the pump draining unboundedly into an in-memory lookahead vector. Content-blind: the verdict-decoupled `NodeBlockSource` contract and the arrival ordering are unchanged. New `#[cfg(test)]` proofs `wirepump_lookahead_stops_at_cap` (feeding `cap + 50` items, one opportunistic drain stops at exactly `MAX_WIRE_PUMP_LOOKAHEAD`) and `wirepump_lookahead_cap_preserves_relay_behavior_under_normal_feed` (a normal-depth feed under the cap relays unchanged — the cap does not perturb the common path). |
| `ade_runtime::network::mux_pump` | **RED** (shell mux pump) | +6 lines | **G-E S1 (`6f848825`) — halt mapping.** `session_err_to_halt` gains one additive arm: `SessionError::ReassemblyBufferOverflow { .. } => PeerHaltReason::ChainSyncDecodeError` — the overflow drops the peer through the existing structured halt path (no panic, no unbounded retry; the RED retry/halt discipline is unchanged). |

## 4. Feature Flags

**No project feature-flag deltas.** Ade declares no `[features]` table in any workspace
`Cargo.toml` (confirmed at both refs — the table is absent), and no `#[cfg(feature =
…)]` gate was introduced in the span. **No `Cargo.toml` change at all in this window.**
No coupling, no `compile_error!` guard. The two new bounds are plain `const` items, not
feature flags.

## 5. CI Checks (118 → 119; +1 new, 0 modified, 0 removed)

One new gate, repo-root-relative and mirroring the existing `ci/ci_check_*.sh` convention.

### PHASE4-N-F-G-E gate

| Check | Status | Cluster origin | What it checks |
|-------|--------|----------------|----------------|
| `ci_check_live_feed_memory_bounds.sh` | **New** | G-E S1 (`6f848825`) | Backs the new **`DC-LIVEMEM-01`** rule. Verifies both live-feed memory bounds are **CLOSED LITERAL constants** — `MAX_REASSEMBLY_TAIL_BYTES` in `crates/ade_network/src/session/core.rs` and `MAX_WIRE_PUMP_LOOKAHEAD` in `crates/ade_node/src/node_sync.rs` — **AND** that neither is wired to a runtime escape hatch: no CLI flag, env var, or config field may set or disable them (the **no-escape-hatch** guard). Line comments are stripped first so the doc-comments that *name* "CLI / env / config" (to forbid them) do not self-trip the grep. Hermetic — no Docker / cardano-cli / live node. |

The serve/forge/containment fences are **byte-unchanged** in this window:
`ci_check_node_run_loop_containment.sh` (relay-loop containment — no serve / admit / gossip
/ broadcast / block-fetch / durable-tip mutation in the loop body) and
`ci_check_served_chain_handoff_fence.sh` (self-accept→serve handoff) are both untouched.
G-E **added** a bounded-memory gate and relaxed **nothing**.

### CI gate lineage (for cross-window readers — explains the headline +1)

The on-disk gate count rose **116 → 117 → 118 → 119** across the G-A…G-E sub-clusters:

- **116 → 117** — G-B S3 added `ci_check_served_chain_handoff_fence.sh` (narrated in the
  N-F-G-B HEAD_DELTAS).
- **117 → 118** — G-C **S2** added `ci_check_ba02_evidence_manifest_schema.sh` **at the
  baseline commit `90791691` itself** (narrated in the **previous**, N-F-G-C, HEAD_DELTAS
  window `339cccb1 → 90791691`, which recorded 117 → 118). Because that gate was committed
  **at** this window's baseline, it is **not** a window-add here — the G-C close-pass
  `351d46bc` was docs/registry only and count-neutral (118 → 118).
- **118 → 119** — G-E S1 added `ci_check_live_feed_memory_bounds.sh` (this window).

So **this** window's CI delta is the **single** G-E gate: **118 → 119 (+1)**.

> Cross-reference (TRACEABILITY): in the **working tree** at `6f848825`,
> `ci_check_live_feed_memory_bounds.sh` is cited by `DC-LIVEMEM-01.ci_script`, and the
> working-tree TRACEABILITY renders that row (11 working-tree TRACEABILITY mentions of the
> gate name). At the **committed** HEAD `6f848825`, the committed TRACEABILITY is still the
> **G-C** close doc (`351d46bc`) and does **not** yet show the gate — the close-pass
> commits the working-tree TRACEABILITY regen. See the warnings in the generation section.

## 6. Canonical Type Registry Delta

**n/a — no separate canonical-type registry is configured** (`canonical_type_registry:
null`), and **no BLUE crate changed**. The 456 BLUE canonical-type total is **unchanged
(Δ0)** across the span (independently re-verified: `git diff --name-only
90791691..6f848825` matches **no** `core_paths` (BLUE) entry — the four changed source
files are `session::core.rs` + `session::event.rs` (the `session/` subtree is GREEN — it
is **not** one of the 9 BLUE `ade_network` submodule paths: `mux/frame.rs`, `codec/`,
`handshake/`, `chain_sync/`, `block_fetch/`, `tx_submission/`, `keep_alive/`,
`peer_sharing/`, `n2c/`), plus RED `node_sync.rs` + RED `mux_pump.rs`). The working-tree
CODEMAP at `6f848825` reports 456 canonical. The new closed `SessionError` variant is
**GREEN-by-content and not canonical-counted**; no new type was added to any BLUE crate.
**No new `CoordinatorEvent` variant or field was introduced.**

## 7. Normative / Invariant Rule Delta (313 → 314)

**One rule ID added, zero removed** (313 → 314). G-E ships exactly one new rule; this
window also commits the **G-C** close-pass cross-rule strengthenings (`351d46bc`), which
add **no** new ID (they extend existing rules).

### G-E new rule (committed in this span, by `6f848825`; flip to `enforced` owed at close)

| Rule | Tier | Status (committed at HEAD) | What it pins |
|------|------|---------------------------|--------------|
| `DC-LIVEMEM-01` | `derived` (operational-hardening; **NOT** BLUE consensus law) | `declared` at `6f848825`; **flips to `enforced`** at the close-pass (already `enforced` in the working-tree registry at this regen) | **Live-feed bounded memory before authoritative decode/apply.** Peer-driven memory on the live `--mode node` feed is bounded BEFORE the BLUE decode/apply path: the per-mini-protocol reassembly tail (`session::core` `MAX_REASSEMBLY_TAIL_BYTES`, 16 MiB) fails closed with a structured `SessionError::ReassemblyBufferOverflow` (drop the peer); the WirePump lookahead (`node_sync` `MAX_WIRE_PUMP_LOOKAHEAD`, 256) stops opportunistic draining so the bounded `mpsc` (cap 64) back-pressures the pump. No silent truncation, no partial decode, no unbounded fallback. The bounds are **CLOSED CONSTANTS** — defensive implementation bounds, NOT Cardano semantic parameters; **no runtime option (CLI / env / config) may disable them or set them unbounded**. `tests = [session_reassembly_tail_over_cap_fails_closed, session_reassembly_tail_under_cap_still_drains_complete_item, wirepump_lookahead_stops_at_cap, wirepump_lookahead_cap_preserves_relay_behavior_under_normal_feed]`; `ci_script = ci/ci_check_live_feed_memory_bounds.sh`. `cross_ref = CN-SESS-04, DC-SESS-06, DC-SYNC-01, DC-SYNC-02, DC-NODE-06, CN-NODE-02`. |

> **Status sequencing (mirrors the prior G-B/G-C windows).** At the committed HEAD
> `6f848825` (the G-E **slice-span** HEAD), `DC-LIVEMEM-01.status = "declared"` — the rule
> is committed with its `tests` and `ci_script` populated, but the `declared → enforced`
> flip is the close-pass edit and lives only in the **working-tree** registry at this
> regen (where it is already `enforced`). This is the exact pattern the N-F-G-B window
> documented for `DC-NODE-06` (`declared` at its slice-span HEAD, flipped at close). The
> count itself rises 313 → 314 at the slice-span HEAD because the **ID** is added by the
> S1 impl `6f848825`; only the status word changes at close.

### G-C close-pass cross-rule strengthenings (committed in this span, by `351d46bc`)

For completeness — these were the *owed* edits flagged in the **previous** (N-F-G-C)
HEAD_DELTAS window and are **committed here** (no new ID; the count holds at 313 across
them, then rises to 314 with `DC-LIVEMEM-01`):

| Rule | Status (held) | Strengthening recorded by `351d46bc` |
|------|---------------|--------------------------------------|
| `RO-LIVE-01` | **`partial`** (held — `blocked_until_operator_stake_available`) | `strengthened_in += "PHASE4-N-F-G-C"` — G-C wired the MECHANICAL live-feed half (`live_wire_pump_feed_reaches_forge_tick`); the LIVE peer-ACCEPT half stays operator-gated. |
| `RO-LIVE-06` | **`enforced`** (held) | `strengthened_in += "PHASE4-N-F-G-C"`; `ci_script += "ci/ci_check_ba02_evidence_manifest_schema.sh"` — the operator-pass BA-02 evidence path is wired (schema + mechanics only; no live BA-02 claim). |
| `CN-OPERATOR-EVIDENCE-01` | **`enforced`** (held) | `strengthened_in += "PHASE4-N-F-G-C"`; `ci_script += "ci/ci_check_ba02_evidence_manifest_schema.sh"` — operator-evidence manifest discipline now covers the `--mode node` BA-02 manifest family. |
| `DC-NODE-06` | **`enforced`** (held) | `strengthened_in += "PHASE4-N-F-G-C"` — the served-chain handoff fence it binds was broadened in place (node-spine owner set + allow-list guard-3) when the live feed was wired. |

**No rule was removed (expected: 0).** The 313 → 314 delta is a single additive ID
(`DC-LIVEMEM-01`); the four G-C tokens are strengthenings, never weakenings.

## 8. Honest residual (cluster scope)

**G-E closes a NARROW operational-hardening claim: peer-driven memory on the live
`--mode node` feed is BOUNDED BEFORE authoritative decode/apply. It is NOT full network
DoS resistance, NOT peer resource fairness, NOT BA-02 / live-evidence readiness, and it
does NOT satisfy the bounty acceptance criterion.**

- **Bounded-before-decode, not DoS-proof.** The two caps fail closed before the BLUE
  `ade_codec` decode path runs, so a single peer cannot drive unbounded in-memory growth
  through the reassembly tail or the lookahead. This is **not** a claim of full network DoS
  resistance and **not** a claim of peer resource fairness.
- **Precision (recorded from the close reviews — do not soften, do not broaden):**
  - The reassembly check is **post-extend** (`buf.len() > cap` after the buffer grew by the
    arriving frame), so a single buffer's transient peak is `cap + one ≤64 KiB mux frame`
    (~**16.06 MiB**), **not** an absolute 16 MiB.
  - `ProtoBuffers` holds up to **~10 INDEPENDENT** per-mini-protocol buffers, each capped
    separately, so the per-connection aggregate ceiling is **~10× the single-buffer cap** —
    still **O(constant) per connection**. **Per-connection-COUNT limits / peer fairness are
    a SEPARATE, out-of-scope surface** (not addressed by G-E).
- **No BLUE change.** 456 BLUE canonical types unchanged (Δ0); `session/` is GREEN-by-content
  (not a BLUE `ade_network` submodule), and the other two changed files are RED. The new
  `SessionError` variant is GREEN-by-content and not canonical-counted. No new
  `CoordinatorEvent` variant or field.
- **Fences byte-unchanged; no live-evidence / BA-02 / RO-LIVE claim.** The serve/forge
  containment gate (`ci_check_node_run_loop_containment.sh`) and the served-chain handoff
  fence (`ci_check_served_chain_handoff_fence.sh`) are **byte-unchanged**. G-E flips **no**
  RO-LIVE rule and makes **no** BA-02 / live-evidence claim. `RO-LIVE-01` stays `partial`
  / `blocked_until_operator_stake_available`; the live operator pass is unchanged by this
  cluster.
- **Gating follow-ons (unchanged).** The bounty acceptance criterion — an
  operator-witnessed accepted block on a peer that can grant leadership — remains gated on
  the **operator-witnessed live pass** (**C1** private testnet, the cheapest real ACCEPT;
  or **C2** preprod with ~2 epochs of provisioned active stake — the public preprod docker
  peer cannot grant acceptance), plus the **PHASE4-N-F-G-D** private-testnet rehearsal.
  G-E removes a hardening blocker for those runs; it does not advance the acceptance claim.

---

## Generation notes (regen `90791691 → 6f848825`, PHASE4-N-F-G-E)

- **Baseline is `90791691`** (the `.idd-config.json` `head_deltas_baseline` value at regen
  time — the PHASE4-N-F-G-C slice-span HEAD). **The close-pass commit must bump
  `head_deltas_baseline` to `6f848825`** (the G-E slice-span HEAD) so the next cluster's
  `/head-deltas` measures from here. The registry-count comment must also bump:
  `_invariant_registry_doc` currently reads **"313 entries at HEAD"** and must become
  **"314 entries at HEAD"**. Both `.idd-config.json` edits are handled by the closer, not
  this regen.
- Counts are mechanical (git/grep/ls only, no cargo): commit log + `--shortstat` over
  `90791691..6f848825` (**3** commits, no merges / **17** files / **+2120 / -1109**); CI
  gate count via `git ls-tree -r --name-only <ref> ci/ | grep -c 'ci_check_.*\.sh'` at each
  ref (**118 → 119**, **+1 new** — `live_feed_memory_bounds` at G-E S1, none removed, none
  modified; the only file in `git diff --diff-filter=A 90791691..6f848825 -- ci/` is that
  gate); registry rule count via `grep -c '^id = '` at each ref (**313 → 314**, the single
  new ID `DC-LIVEMEM-01`; `diff` of sorted `^id =` lines shows exactly one `>` add and zero
  `<` removals); workspace test attributes via `git grep -hE '#\[(tokio::)?test\]'`
  (**2230 → 2234**, +4); BLUE canonical types unchanged at 456 (no BLUE `core_paths` file
  in the diff — `session/` is GREEN-by-content, not a BLUE `ade_network` submodule).
- **CI-gate premise corrected against git (not an anomaly).** The on-disk gate count at the
  **baseline** `90791691` is **118** (verified `git ls-tree`), and the G-C close commit
  `351d46bc` is also **118**. `ci_check_ba02_evidence_manifest_schema.sh` was added at
  `90791691` **itself** (`git log --diff-filter=A -- ci/ci_check_ba02_evidence_manifest_schema.sh`
  → `90791691`, G-C S2) — i.e., **at** this window's baseline, so it is **not** a
  window-add; the previous (N-F-G-C) HEAD_DELTAS already counted it in its `339cccb1 →
  90791691` window (117 → 118). This window's CI delta is therefore **118 → 119 (+1)** —
  the single G-E gate — **not** 117 → 119 (+2). The G-C close `351d46bc` was docs/registry
  only and added no gate.
- **Two close events in one window (NOT an anomaly).** The **G-C** close-pass (`351d46bc`)
  is **committed** inside this window (because the baseline `90791691` is the G-C slice-span
  HEAD, not its close commit) — it recorded the four G-C `strengthened_in` tokens, bound the
  G-C gate to `RO-LIVE-06` / `CN-OPERATOR-EVIDENCE-01`, refreshed the four grounding docs to
  the **G-C** close, and bumped the baseline `339cccb1 → 90791691`. The **G-E** close-pass
  is **not yet committed**. So at HEAD `6f848825`: the registry has the four G-C tokens
  committed and `DC-LIVEMEM-01` as `declared` (the `enforced` flip is working-tree only);
  and the **committed** CODEMAP / SEAMS / TRACEABILITY are still the **G-C** close docs (118
  CI, 2230 tests, 313 rules, no `DC-LIVEMEM-01`).
- **Sibling-doc coherence (working tree).** The CODEMAP / SEAMS / TRACEABILITY **working
  tree** (and the registry) have already been regenerated to the **G-E** close state —
  **119** CI checks (matches the on-disk `ls ci/ci_check_*.sh | wc -l = 119`), **456**
  canonical types, **2234** tests, **314** rules, with `DC-LIVEMEM-01` `enforced`, the new
  `session::core` / `node_sync` caps, and the `live_feed_memory_bounds` gate cross-referenced
  — and they **agree** with this doc's counts (CODEMAP/SEAMS headers explicitly read "119 CI
  checks at HEAD (`6f848825`, PHASE4-N-F-G-E cluster close)"; `DC-LIVEMEM-01` appears 13× in
  CODEMAP, 27× in SEAMS, 18× in TRACEABILITY). `git status` shows `docs/ade-CODEMAP.md`,
  `docs/ade-SEAMS.md`, `docs/ade-TRACEABILITY.md`, and `docs/ade-invariant-registry.toml`
  modified (working-tree close-pass edits already applied); `docs/ade-HEAD_DELTAS.md` and
  `.idd-config.json` are the two files this regen / the close-pass still owe.
- **CI cross-reference warning.** `ci_check_live_feed_memory_bounds.sh` is cited by
  `DC-LIVEMEM-01` only in the **working-tree** registry / TRACEABILITY (the committed
  TRACEABILITY at `6f848825` is still the G-C close doc and does not yet show it). The
  close-pass commits the `ci_script` binding's enforced state and regenerates the committed
  TRACEABILITY. (The G-C gate `ba02_evidence_manifest_schema` and the handoff/containment
  fences are already cited in the committed registry, via `351d46bc` / earlier.)
- **Not a rule removal, not a discipline violation** — the working-tree-vs-committed split
  is a sequencing artifact reconciled by the close-pass (which commits the `DC-LIVEMEM-01`
  `declared → enforced` flip, the CODEMAP/SEAMS/TRACEABILITY working-tree regen, the
  `.idd-config.json` baseline bump `90791691 → 6f848825` + the `313 → 314` registry-count
  comment, the G-E cluster-doc archive, and this HEAD_DELTAS). The next gating work is the
  **operator-witnessed live pass** (C1/C2) and the **PHASE4-N-F-G-D** private-testnet
  rehearsal — neither advanced by G-E, which only removes a bounded-memory hardening
  blocker ahead of them.
