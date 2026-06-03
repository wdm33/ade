# Ade — HEAD Deltas (Changes Since Baseline)

> **Status:** Living architectural document. Regenerated; not hand-edited.
>
> Regenerate with `/head-deltas <baseline>` after every cluster close. Baseline is recorded in `.idd-config.json` `head_deltas_baseline`.

> Baseline: `13028d49` (Close PHASE4-N-F-G-I — shared admission bootstrap persists seed-epoch anchor lineage, 2026-06-03 12:27)
> HEAD: `550eec3a` (C1 genesis-successor rehearsal harness — PHASE4-N-F-G-J S5, 2026-06-03 22:02)
> Cluster: **PHASE4-N-F-G-J — genesis-successor block correctness** (empty-feed forge scheduling → null PrevHash wire authority → position rule → cold-start reachability → C1 genesis rehearsal harness), slice span closed; close-pass commit to follow.
> 17 commits (no merges), 48 files changed, +3587 / -84 lines.

This window narrates the **PHASE4-N-F-G-J cluster** — **genesis-successor block
correctness** on the `--mode node` spine. The cluster answers one structural question:
*can the node legitimately forge and serve the FIRST block of a from-genesis chain — the
genesis-successor — with the CORRECT wire shape and the CORRECT cold-start permission?*
It does so across **five slices**, walking inward from a diagnostic surface to the BLUE
wire grammar and back out to a path-faithful rehearsal harness:

- **S1 (`60303079`) — emit-only feed/forge scheduling events.** New **GREEN**
  `ade_node::live_log::sched_event` (a closed, fail-closed-on-unknown JSONL event
  vocabulary — `feed_unavailable{reason}`, `forge_tick_considered`, `forge_tick_skipped`,
  `forge_attempted`, `forge_result`, all with closed reason/outcome enums and **no**
  catch-all variant) + new **GREEN** `ade_node::live_log::sched_writer` (a hand-rolled
  byte-deterministic one-object-per-line JSONL writer mirroring `live_log/writer.rs`). The
  closed S1 reason set is exactly three — `NoBlockAvailable` and `CleanEmpty` are
  forge-**eligible**; `UnknownDisconnected` (a reason-less / ambiguous WirePump disconnect)
  is **INELIGIBLE** (fail-closed-on-ambiguity). The events are **emit-only** — the planner
  may EMIT but MUST NOT consume them. New gate `ci_check_node_sched_events_emit_only.sh`
  (later hardened by `36b2216f` against a pipefail race). New rule **`CN-NODE-04`**
  (`enforced`).
- **S2 (`3b24c572`) — PrevHash null/hash32 wire authority.** New **BLUE** canonical sum
  type `PrevHash = Genesis | Block(Hash32)` in `ade_types` (replaces the flat `Hash32` for
  the Shelley-and-later `header_body.prev_hash`) + the **POSITION-BLIND** `$hash32 / null`
  codec in `ade_codec` (`PrevHash::Genesis` ⇄ CBOR `null` `0xf6`; `PrevHash::Block(h)` ⇄ a
  32-byte `hash32` — decoded without knowing `block_number`). New gate
  `ci_check_prevhash_single_wire_authority.sh` pins the codec as the **one** shared wire
  authority. Backs **`CN-WIRE-09`** clause 1 (wire grammar). The CI fix `36b2216f`
  (`fix(ci)`) repairs the S1 sched-event gate's pipefail race.
- **S3 (`0c1939a1`) — genesis-successor position rule + genesis forge.** New **BLUE**
  `ade_ledger::block_validity::header_position` (`check_header_position` — the **single
  POSITION-AWARE authority**: `block_number 0 ⟺ PrevHash::Genesis`, `block_number > 0 ⟺
  PrevHash::Block`); new variant `BlockValidityError::HeaderPositionInvalid` folded into the
  **existing** `BlockRejectClass::HeaderInvalid` coarse class (no new reject class). The
  producer `prev_hash` is migrated **`Hash32` → `PrevHash` end-to-end** (`ade_ledger`
  `producer/{state,forge}.rs` + `block_validity/unsigned_header_pre_image.rs`; `ade_runtime`
  `producer/{chain_evolution,tick_assembler,scheduler}.rs`; `ade_node`
  `produce_mode.rs`/`node_sync.rs` `ForgeRequestContext`) — the cold-start `prev_hash()`
  now yields `PrevHash::Genesis` and the **all-zero `Hash32` stand-in is deleted**.
  `decode_block` calls `check_header_position` **before** the header authority. Backs
  **`CN-WIRE-09`** clause 2 (position rule). **The `Block`-path wire encoding is
  byte-identical post-migration** — no BLUE-authority weakening.
- **S4 (`3df8bd4f`) — node-spine cold-start first-block reachability.** `node_sync`
  `forge_one_from_recovered` now takes `Option<&ChainTip>` + a **GREEN**
  `forge_header_position` (single cold-start convention: `None` ⇒ block 0 + `PrevHash::Genesis`,
  `Some` ⇒ `last_block_no + 1` + `Block`; the old `.unwrap_or(1)` is **deleted**) +
  `NodeForgeError::RecoveredTipMissingBlockNo` (malformed-height fails closed). `node_lifecycle`
  gains a **GREEN** `may_cold_start_forge` permission gate on the `LoopStep::ForgeTick`
  arm: the genesis-successor (both-`None`) forge fires EXACTLY ONCE, only when the
  WarmStart-recovered seed-epoch lineage is present, `ForgeIntent::On`, and the feed is
  forge-eligible under the `CN-NODE-04` split. New gate
  `ci_check_genesis_successor_reachability.sh`. New rule **`DC-NODE-08`** (`enforced`).
- **S5 (`550eec3a`) — C1 genesis-successor rehearsal harness.** A path-faithful,
  **non-promotable** rehearsal harness for the C1 genesis-successor accepted-block leg:
  `docs/evidence/phase4-n-f-g-j-genesis-rehearsal-README.md` (operator README, no `.toml`
  manifest committed) + `tests/node_c1_genesis_rehearsal.rs` (an **env-gated**
  `ADE_LIVE_C1_GENESIS_REHEARSAL=1` operator live arm + a hermetic correlate→envelope
  proof) + two hermetic genesis-rehearsal tests in `forge_succeeds.rs`. The harness
  **reuses** `ba02_evidence::correlate` and the G-D `PrivateRehearsalManifest` envelope
  **verbatim** — **no new evidence type** (`rehearsal_evidence.rs` is byte-unchanged). The
  G-D rehearsal gate `ci_check_rehearsal_manifest_schema.sh` is **EXTENDED in place** to add
  the G-J genesis-rehearsal home glob. **`CN-REHEARSAL-FIDELITY-01`** gains
  `strengthened_in += PHASE4-N-F-G-J`.

**NARROW CLAIM (load-bearing — recorded honestly).** G-J enforces the genesis-successor
forge **MECHANISM** (cold-start reachability) + **wire authority** (null/hash32 PrevHash +
the position rule) + the rehearsal **HARNESS**. It does **NOT** claim: a live C1 accepted
block, preprod acceptance, any RO-LIVE flip (**`RO-LIVE-01` stays `partial`**), bounty
satisfaction, or durable block-1+ progression (the durable tip advances ONLY through the
accepted path, never from forge scheduling alone). The live C1 genesis rehearsal stays
**`blocked_until_operator_c1_genesis_successor_rehearsal`**. There is **NO BLUE-authority
weakening**: the `Block`-path wire encoding is **byte-identical** after the `Hash32 →
PrevHash` migration (genesis is a *new representable* predecessor, not a changed one).

> **Baseline-gap note (load-bearing — read before §1).** This window's baseline is the
> **PHASE4-N-F-G-I** close (`13028d49`, 2026-06-03 12:27). The **previous** HEAD_DELTAS lead
> (preserved below as *Historical*) narrates **PHASE4-N-F-G-D** (`6f848825..6bd60c80`). The
> intervening closes **G-E (committed `da205bff` inside the G-D window), G-F, G-G, G-H, and
> G-I** were each closed and their grounding docs refreshed, but **HEAD_DELTAS was not
> re-led** for them (each `/head-deltas` regen narrates its own cluster and the prior lead is
> demoted; the G-E…G-I leads are recoverable from their own close-pass commits and the
> registry, not reconstructed here). Their cumulative effect is visible only in the count
> baselines this window measures **from**: at `13028d49` the registry holds **316 rules** and
> the on-disk gate count is **123** — already well past the G-D figures (315 rules, 121
> gates). This regen does **NOT fabricate** the missing G-E…G-I narratives; it measures the
> explicit `13028d49..550eec3a` G-J span and preserves the G-D lead verbatim as the most
> recent surviving narrative.

## 0. Headline

| Count | Baseline (`13028d49`) | HEAD (`550eec3a`) | Δ |
|---|---|---|---|
| CI gates (`ci/ci_check_*.sh`) | 123 | **126** | **+3 new** (`node_sched_events_emit_only` S1, `prevhash_single_wire_authority` S2, `genesis_successor_reachability` S4); **+1 modified in place** (`rehearsal_manifest_schema` S5 — genesis-home glob added); none removed |
| Registry rules | 316 | **319** | **+3 new** (`CN-NODE-04` S1, `CN-WIRE-09` S2/S3, `DC-NODE-08` S4); **1 strengthening** (`CN-REHEARSAL-FIDELITY-01` `strengthened_in += PHASE4-N-F-G-J`, S5); none removed |
| Test attributes (`#[test]`/`#[tokio::test]`, workspace) | 2245 | **2284** | **+39** (the G-J tests, concentrated in `CN-WIRE-09` (20) + `DC-NODE-08` (9) + `CN-NODE-04` (2) + the S5 rehearsal proofs + the producer-migration round-trips) |
| BLUE canonical types | 456 | **457** | **+1** (`PrevHash = Genesis \| Block(Hash32)` in `ade_types`, S2 — absent at baseline; replaces the flat `Hash32` for `header_body.prev_hash`) |

> **Sibling-doc coherence (load-bearing — read before §7).** At this regen the **other three
> grounding docs are ALREADY G-J-regenerated in the working tree** (CODEMAP / SEAMS /
> TRACEABILITY are dirty per `git status`), and their **counts agree with this doc**: CODEMAP
> header reads "**457 canonical types, 2284 tests, 126 CI checks** … PHASE4-N-F-G-J cluster
> close"; TRACEABILITY reads "**319 rules** at HEAD … `+3` this close … `CN-WIRE-09`,
> `DC-NODE-08`, `CN-NODE-04` … `CN-REHEARSAL-FIDELITY-01` is `strengthened_in +=
> PHASE4-N-F-G-J`." All four new/strengthened G-J rule leaves (20 `CN-WIRE-09`, 9
> `DC-NODE-08`, 2 `CN-NODE-04`, 3 G-J `CN-REHEARSAL-FIDELITY-01`) and all five live G-J gates
> are cross-checked present on disk. **SHA labels reconciled at close:** all four grounding
> docs (CODEMAP / TRACEABILITY / SEAMS / this) now label the close HEAD **`550eec3a`** (the
> S5 impl `feat(node): C1 genesis-successor rehearsal harness`); the in-span S2 CI-fix
> **`36b2216f`** (an ancestor of `550eec3a`) is referenced only where it is the actual commit
> (the §1 commit table + the gate-hardening notes). Counts are the `550eec3a` figures (457 /
> 2284 / 126). No content divergence.
>
> **Not a rule removal, not a discipline violation** — a label artifact, reconciled at close.

## 1. Commit Log (newest first)

| Hash | Type | Summary |
|------|------|---------|
| `550eec3a` | feat | C1 genesis-successor rehearsal harness (PHASE4-N-F-G-J S5) |
| `33162030` | docs | slice doc PHASE4-N-F-G-J S5 C1 genesis rehearsal |
| `3df8bd4f` | feat | node-spine cold-start first-block reachability (PHASE4-N-F-G-J S4) |
| `8b71766c` | docs | slice doc PHASE4-N-F-G-J S4 node-spine cold-start reachability |
| `0c1939a1` | feat | enforce genesis-successor PrevHash position rule (PHASE4-N-F-G-J S3) |
| `6b754e6a` | docs | slice doc PHASE4-N-F-G-J S3 position validation + genesis forge |
| `36b2216f` | fix | avoid pipefail race in node sched event gate |
| `3b24c572` | feat | enforce PrevHash null/hash32 wire authority (PHASE4-N-F-G-J S2) |
| `599e7d9b` | docs | slice doc PHASE4-N-F-G-J S2 PrevHash codec authority |
| `c167cd41` | docs | re-scope PHASE4-N-F-G-J around PrevHash null authority |
| `b85a6170` | docs | replan PHASE4-N-F-G-J around PrevHash null authority |
| `60303079` | feat | emit-only CN-NODE-04 feed/forge scheduling events (PHASE4-N-F-G-J S1) |
| `b6554715` | docs | amend PHASE4-N-F-G-J to fail-closed UnknownDisconnected (option b) |
| `d97dc293` | docs | slice doc PHASE4-N-F-G-J S1 feed/forge events |
| `ff03e244` | docs | cluster doc PHASE4-N-F-G-J empty-feed forge scheduling |
| `6461160e` | docs | plan PHASE4-N-F-G-J empty-feed forge scheduling |
| `9eb6f39b` | docs | declare PHASE4-N-F-G-J empty-feed forge scheduling |

No merge commits in the span. **Unlike the G-D window, the baseline `13028d49` IS a close
commit** (the PHASE4-N-F-G-I close-pass), so **no prior-cluster close tail is carried** into
this window — the span is the G-J cluster only: **6 declare/plan/cluster/replan docs**
(`9eb6f39b` / `6461160e` / `ff03e244` / `c167cd41` / `b85a6170`) followed by **five
doc-then-impl slices** (S1 `d97dc293`+`b6554715`+`60303079`, S2 `599e7d9b`+`3b24c572`, S3
`6b754e6a`+`0c1939a1`, S4 `8b71766c`+`3df8bd4f`, S5 `33162030`+`550eec3a`) plus the one S2
CI hotfix `36b2216f`.

Six commits carry a `feat:`/`fix:` conventional prefix; eleven carry `docs:`. **Zero
unclassified** — every commit follows conventional commits.

(Plus the pending G-J close-pass commit: the working-tree CODEMAP / SEAMS / TRACEABILITY /
registry G-J regen, the `.idd-config.json` baseline bump (`6bd60c80 → 550eec3a` — see the
generation notes; the config baseline is **two clusters stale** at `6bd60c80`), the G-J
cluster-doc archive, and this HEAD_DELTAS.)

## 2. New Modules

Three new source modules: one **BLUE** (`ade_ledger` block-validity authority) and two
**GREEN** (`ade_node` diagnostic surface). All three are confirmed **absent at the baseline**
`13028d49` and registered by the `mod`/`pub mod` lines added in their slices.

| Module | Color | Purpose | Key sub-paths | Added in (cluster/slice) |
|--------|-------|---------|---------------|--------------------------|
| `ade_ledger::block_validity::header_position` | **BLUE** (deterministic block-validity authority; `core_paths` `crates/ade_ledger/`) | The **single POSITION-AWARE authority** for genesis-successor correctness: `block_number 0 ⟺ PrevHash::Genesis`, `block_number > 0 ⟺ PrevHash::Block(hash32)`. The position-blind codec (§3, `ade_codec`) decodes shape only; this module is where shape is bound to chain position. | `crates/ade_ledger/src/block_validity/header_position.rs` — `check_header_position(block_number, prev_hash)`; on violation surfaces `BlockValidityError::HeaderPositionInvalid { block_number, prev_is_genesis }` (folded into the existing `HeaderInvalid` coarse class). Registered via `pub mod header_position;` in `block_validity/mod.rs`; called by `decode_block` (`header_input.rs`) **before** the header authority. | `PHASE4-N-F-G-J` S3 (`0c1939a1`) |
| `ade_node::live_log::sched_event` | **GREEN** (deterministic Core-Contract banner + `//! GREEN`; pure closed-enum types, no I/O / clock / rand / float / `HashMap`) | The **closed, emit-only** `--mode node` feed/forge scheduling event vocabulary: `feed_unavailable{reason}`, `forge_tick_considered`, `forge_tick_skipped{reason}`, `forge_attempted`, `forge_result{outcome}` — closed reason/outcome enums with **no** catch-all `Other` variant (a new variant is a compile error at the exhaustive encoder + fails the allow-list closedness test until wired). | `crates/ade_node/src/live_log/sched_event.rs` — `NodeSchedEvent` closed enum; the closed reason set (`NoBlockAvailable` / `CleanEmpty` forge-eligible; `UnknownDisconnected` ineligible, fail-closed-on-ambiguity); exhaustive JSONL encoder. | `PHASE4-N-F-G-J` S1 (`60303079`) |
| `ade_node::live_log::sched_writer` | **GREEN** (deterministic Core-Contract banner + `//! GREEN`; byte-deterministic JSONL writer) | The byte-deterministic JSONL writer for `NodeSchedEvent` — one JSON object per line, flushed after every emit, mirroring `live_log/writer.rs`. **Emit-only**: the planner writes events but never reads them (one-directional planner → log). | `crates/ade_node/src/live_log/sched_writer.rs` — hand-rolled JSON serializer over the closed sched-event enum. | `PHASE4-N-F-G-J` S1 (`60303079`) |

> **Cross-reference (CODEMAP): consistent.** All three modules appear in the **working-tree**
> CODEMAP (27 `header_position` + 19 `sched_event`/`sched_writer` mentions, in the G-J delta
> block). No staleness warning for this window — the working-tree CODEMAP is G-J-current (the
> only caveat is the SHA-label discrepancy in §0; counts agree).

No new crate, no new workspace, no new WAL/checkpoint, no new `CoordinatorEvent` variant.
The S5 rehearsal harness adds **no new module** — it reuses the G-D
`ade_node::rehearsal_evidence` / `rehearsal_pass` and the `PrivateRehearsalManifest` envelope
verbatim (confirmed: `rehearsal_evidence.rs` is **not** in the span diff).

## 3. Modules Modified

The producer `prev_hash` migration (S3) and the cold-start reachability wiring (S4) touch a
spread of BLUE + RED + GREEN files. Trivial test-fixture churn (the `Hash32 → PrevHash`
call-site edits in `ade_codec`/`ade_testkit`/`ade_runtime` round-trip tests — each a few
lines) is folded into the relevant module row rather than listed separately.

| Module | Color | Scope | Key changes |
|--------|-------|-------|-------------|
| `ade_types::shelley::block` | **BLUE** (`crates/ade_types/`) | +26/-? | **G-J S2 (`3b24c572`) — new canonical type.** Introduces `PrevHash = Genesis \| Block(Hash32)` (+ `block_hash()` accessor) and migrates `ShelleyHeaderBody.prev_hash` from flat `Hash32` to `PrevHash`. **+1 canonical type** (456 → 457). |
| `ade_codec::shelley::block` | **BLUE** (`crates/ade_codec/`) | +162 | **G-J S2 (`3b24c572`) — position-blind wire codec.** `decode_prev_hash` + the `ShelleyHeaderBody` `AdeEncode` `null`/`hash32` match: `PrevHash::Genesis` ⇄ CBOR `null` (`0xf6`), `PrevHash::Block(h)` ⇄ `hash32`. **POSITION-BLIND** — decodes shape without knowing `block_number` (the position rule lives in S3's `header_position`). The **`Block` path is byte-identical** to the pre-migration `Hash32` encoding. |
| `ade_ledger::block_validity` (`mod.rs` / `header_input.rs` / `verdict.rs` / `unsigned_header_pre_image.rs`) | **BLUE** (`crates/ade_ledger/`) | +142 across 4 files (+ new `header_position.rs`, §2) | **G-J S3 (`0c1939a1`).** `mod.rs` registers `header_position`; `header_input.rs` `decode_block` calls `check_header_position` **before** the header authority; `verdict.rs` adds `BlockValidityError::HeaderPositionInvalid` → existing `BlockRejectClass::HeaderInvalid`; `unsigned_header_pre_image.rs` carries `tick.prev_hash: PrevHash` into the KES pre-image directly (`Genesis` for block 0; `Block` path byte-identical). |
| `ade_ledger::producer` (`forge.rs` / `state.rs`) | **BLUE** (`crates/ade_ledger/`) | +160 | **G-J S3 (`0c1939a1`) — producer prev_hash migration.** `ProducerTick`/`TickInputs` carry `prev_hash: PrevHash`; `forge.rs` emits `PrevHash::Genesis` at `block_number 0` and a byte-identical `Block` prev otherwise. |
| `ade_node::node_sync` | **GREEN/RED** (`crates/ade_node/` — relay-loop home) | +289 | **G-J S3+S4.** S3: `ForgeRequestContext` prev_hash `Hash32 → PrevHash`. S4 (`3df8bd4f`): `forge_one_from_recovered(selected_tip: Option<&ChainTip>)`; new **GREEN** `forge_header_position` (single cold-start convention: `None` ⇒ block 0 + `Genesis`, `Some` ⇒ `last_block_no+1` + `Block`; the **`.unwrap_or(1)` is deleted**); `NodeForgeError::RecoveredTipMissingBlockNo` (malformed height fails closed); routes the cold-start ctx through the **same** `run_real_forge` S3 proved. |
| `ade_node::node_lifecycle` | **GREEN/RED** (`crates/ade_node/`) | +166 | **G-J S4 (`3df8bd4f`) — cold-start permission.** New **GREEN** `may_cold_start_forge` gate on the `LoopStep::ForgeTick` arm: the both-`None` genesis-successor forge fires EXACTLY ONCE, only when the recovered seed-epoch lineage is present + `ForgeIntent::On` + the feed is forge-eligible under `CN-NODE-04`. Passes `selected_tip.as_ref()` into `forge_one_from_recovered`. |
| `ade_node::live_log` (`mod.rs`) | **GREEN** (`crates/ade_node/`) | +4 | **G-J S1 (`60303079`) — module registration.** Adds `pub mod sched_event;` + `pub mod sched_writer;`. |
| `ade_node::admission::runner` / `produce_mode` | **GREEN/RED** (`crates/ade_node/`) | +1 / +4 | **G-J S1/S3.** `admission/runner.rs` wires the sched-event surface (+1); `produce_mode.rs` carries the `PrevHash` migration through `ForgeRequestContext` (+4). |
| `ade_runtime::producer` (`chain_evolution.rs` / `tick_assembler.rs` / `scheduler.rs` / `producer_shell.rs`) | **RED** (`crates/ade_runtime/` — shell) | +61 | **G-J S3 (`0c1939a1`) — prev_hash migration through the shell.** `chain_evolution.rs` `prev_hash()` cold-start now yields `PrevHash::Genesis` (the **all-zero `Hash32` stand-in is deleted**); `tick_assembler`/`scheduler`/`producer_shell` thread `PrevHash` through the producer pipeline. |
| `ade_node::tests::forge_succeeds` | test | +102 | **G-J S5 (`550eec3a`) — hermetic genesis-rehearsal proofs** (`genesis_rehearsal_manifest_binds_block_zero_genesis`, `genesis_rehearsal_no_evidence_writes_nothing`) reusing `correlate` + `PrivateRehearsalManifest`. |
| `ade_codec` / `ade_testkit` / `ade_runtime` round-trip tests (5 files) + `ade_ledger::block_body_hash` | test / BLUE | ≤4 lines each | **G-J S2/S3 — `Hash32 → PrevHash` call-site churn** in `{allegra_mary,full_corpus,shelley}_round_trip.rs`, `harness/adapters/{shelley,shelley_common}.rs`, `producer/fixtures.rs`, `producer_pipeline_slot_deadline.rs`, and a 2-line `block_body_hash.rs` adjust. Trivial mechanical migration; no behavioral change. |

The non-G-J forge / serve / live-feed / containment surfaces are otherwise **unchanged** in
this window.

## 4. Feature Flags

**No project feature-flag deltas.** Ade declares no `[features]` table in any workspace
`Cargo.toml` (confirmed absent at both refs), and no `#[cfg(feature = …)]` gate was
introduced in the span. **No `Cargo.toml` change at all in this window.** No coupling, no
`compile_error!` guard. (The S5 C1 genesis rehearsal operator harness is gated by an
**environment variable** `ADE_LIVE_C1_GENESIS_REHEARSAL`, **not** a Cargo feature — it is a
`#[test]` skipped in CI, not a compile-time flag and not a runtime node mode.)

## 5. CI Checks (123 → 126; +3 new, +1 modified in place, 0 removed)

Three new gates plus one in-place extension, repo-root-relative and mirroring the existing
`ci/ci_check_*.sh` convention. The only files in `git diff --diff-filter=A 13028d49..HEAD --
ci/` are the three new gates; the only `--diff-filter=M` file is `rehearsal_manifest_schema`;
`--diff-filter=D` over `ci/` is empty.

### PHASE4-N-F-G-J gates

| Check | Status | Cluster origin | What it checks |
|-------|--------|----------------|----------------|
| `ci_check_node_sched_events_emit_only.sh` | **New** | G-J S1 (`60303079`); hotfix `36b2216f` | Backs **`CN-NODE-04`**. Pins the sched-event vocabulary as **closed + emit-only**: the `NodeSchedEvent` reason/outcome enums have no catch-all `Other`; the planner EMITS but never CONSUMES the events (one-directional); only the forge-eligible reasons (`NoBlockAvailable` / `CleanEmpty`) may feed the `DC-NODE-08` forge allowance. The hotfix `36b2216f` removed a `pipefail` race in the gate's own pipeline. |
| `ci_check_prevhash_single_wire_authority.sh` | **New** | G-J S2 (`3b24c572`) | Backs **`CN-WIRE-09`** clause 1. Pins the **single** shared BLUE `ade_codec` wire authority for `prev_hash` (`$hash32 / null`): `PrevHash::Genesis` ⇄ CBOR `null`, `PrevHash::Block` ⇄ `hash32`; forbids any second/competing prev_hash codec, any all-zero `Hash32` / anchor-fingerprint / Shelley-genesis-hash stand-in for the genesis predecessor, and confirms the codec is **position-blind** (the position rule lives in `header_position`, S3). |
| `ci_check_genesis_successor_reachability.sh` | **New** | G-J S4 (`3df8bd4f`) | Backs **`DC-NODE-08`**. Pins the cold-start forge permission: the both-`None` genesis-successor forge is reachable ONLY from the recovered seed-epoch lineage (never an unanchored / from-genesis-file base), fires EXACTLY ONCE, requires `ForgeIntent::On` + a forge-eligible feed, carries `PrevHash::Genesis`, and flows through `self_accept → SelfAcceptedHandoff → ServedChainView`; the durable tip advances only through the accepted path, never from forge scheduling alone; **no RO-LIVE-01/06 flip**. |
| `ci_check_rehearsal_manifest_schema.sh` | **Modified in place** | G-D S2 (origin); G-J S5 extension (`550eec3a`) | The G-D rehearsal-manifest schema gate, **extended** to cover the G-J genesis-rehearsal home: `REHEARSAL_GLOBS` now matches **both** `phase4-n-f-g-d-private-rehearsal-*.toml` and `phase4-n-f-g-j-genesis-rehearsal-*.toml`. Still **vacuous-until-committed** (only READMEs are committed under the rehearsal homes; the live runs are operator-gated). The closed schema, the two non-promotability markers (`is_rehearsal` / `not_bounty_evidence`), the `peer_log_file_sha256` binding, and the bounty-home leak barriers are unchanged in shape — only the glob set widened. |

The non-G-J containment / handoff / memory fences are **byte-unchanged** in this window;
G-J **added** three gates, **extended** one rehearsal gate to cover a second non-promotable
home, and relaxed **nothing**.

### CI gate lineage (for cross-window readers — explains the headline +3)

The on-disk gate count rose **121 → 123 → 126** from the G-D close to this G-J close:

- **121 → 123** — the intervening **G-E…G-I** closes added net **+2** gates (not narrated in
  this window; recoverable from those clusters' close-pass commits). At this window's
  baseline `13028d49` (the G-I close) the on-disk count is **123**.
- **123 → 126** — G-J S1 + S2 + S4 added `node_sched_events_emit_only`,
  `prevhash_single_wire_authority`, and `genesis_successor_reachability` (this window), plus
  the in-place `rehearsal_manifest_schema` extension (S5, count-neutral).

So **this** window's CI delta is the **three** new G-J gates: **123 → 126 (+3)**, plus the
one in-place rehearsal-gate extension.

> **Cross-reference (TRACEABILITY): consistent.** The working-tree TRACEABILITY is
> G-J-current and cites all five live G-J gates by their rules (`CN-WIRE-09 →
> ci_check_prevhash_single_wire_authority`, `DC-NODE-08 →
> ci_check_genesis_successor_reachability`, `CN-NODE-04 →
> ci_check_node_sched_events_emit_only`, and the two rehearsal gates under
> `CN-REHEARSAL-FIDELITY-01`), each cross-checked present on disk. SHA labels are reconciled at
> close — all four grounding docs label `550eec3a` (the in-span S2 CI-fix `36b2216f` is an ancestor).

## 6. Canonical Type Registry Delta

**n/a — no separate canonical-type registry is configured** (`canonical_type_registry:
null`); canonical-type rules live inline in the invariant registry under family **T**. The
BLUE canonical-type total **rose 456 → 457 (+1)**: the new sum type `PrevHash = Genesis |
Block(Hash32)` in `ade_types::shelley::block` (independently confirmed absent at `13028d49`
and present at `550eec3a`). It **replaces** the flat `Hash32` on `ShelleyHeaderBody.prev_hash`
— the migration is **shape-additive, not authority-weakening**: `PrevHash::Block(h)` encodes
**byte-identically** to the pre-migration `Hash32`, and `PrevHash::Genesis` adds a *new
representable* predecessor (CBOR `null`) that the flat `Hash32` could not express. The
working-tree CODEMAP at this regen reports **457 canonical**. **No new `CoordinatorEvent`
variant or field was introduced.** The S5 rehearsal harness adds **no** canonical type (it
reuses the G-D `PrivateRehearsalManifest`, which is GREEN-by-content and not
canonical-counted).

## 7. Normative / Invariant Rule Delta (316 → 319)

**Three rule IDs added, one strengthening, zero removed** (316 → 319). All three new rules
are committed `enforced` in this span (no `declared → enforced` close-flip is owed for the
new IDs — verified at HEAD); the one strengthening (`CN-REHEARSAL-FIDELITY-01`) is committed
in the working-tree registry at this regen.

### G-J new rules (committed `enforced` in this span)

| Rule | Family / Tier | Status (at HEAD) | What it pins |
|------|---------------|------------------|--------------|
| `CN-NODE-04` | CN / `operational` | `enforced` | **Closed, allow-listed, emit-only feed/forge scheduling event vocabulary** for `--mode node`. Closed reason/outcome enums (`feed_unavailable{reason}`, `forge_tick_considered`, `forge_tick_skipped`, `forge_attempted`, `forge_result`) with **no catch-all**; the S1-producible closed reason set is exactly `NoBlockAvailable` / `CleanEmpty` (forge-eligible) + `UnknownDisconnected` (INELIGIBLE, fail-closed-on-ambiguity). Operational/diagnostic ONLY — never a consensus/acceptance/BA-02 signal; **emit-only** (planner emits, never consumes). `code_locus = node_sync.rs; node_lifecycle.rs; live_log/sched_event.rs; live_log/sched_writer.rs`. `tests = [node_sched_events_emit_closed_vocabulary, node_sched_event_allowlist_rejects_unknown_variants]`; `ci_script = ci/ci_check_node_sched_events_emit_only.sh`. |
| `CN-WIRE-09` | CN / `derived` | `enforced` | **The `header_body.prev_hash` closed wire grammar `$hash32 / null`** (cardano-ledger `PrevHash = GenesisHash \| BlockHash`). Ade represents it as the closed sum `PrevHash = Genesis \| Block(Hash32)` — never a flat `Hash32`. `Genesis` ⇄ CBOR `null`; `Block(h)` ⇄ `hash32`; one shared **position-blind** BLUE codec. The **position-aware** rule (`block_number 0 ⟺ Genesis`) is enforced by `check_header_position` (S3), not the codec. No all-zero `Hash32` / anchor fingerprint / Shelley-genesis-hash stand-in. `code_locus` spans `ade_types`/`ade_codec` (S2) + `ade_ledger::block_validity::{header_position,header_input,verdict,unsigned_header_pre_image}` + `producer/forge.rs` + `ade_runtime::producer::chain_evolution` (S3, producer prev_hash migrated end-to-end). **20 tests** (round-trip + position + byte-identity + forge + pre-image); `ci_script = ci/ci_check_prevhash_single_wire_authority.sh`. |
| `DC-NODE-08` | DC / `derived` | `enforced` | **`--mode node` MAY forge the genesis-successor (FIRST) block** when `ChainDb::tip()` AND `recovered.tip` are BOTH `None`, but ONLY when ALL hold: (a) the WarmStart-recovered seed-epoch lineage is present (never unanchored/from-genesis-file/stale); (b) `ForgeIntent::On` + complete key material; (c) the feed is forge-eligible under the `CN-NODE-04` split; (d) slot/epoch/KES/leader guards pass; (e) the forged block carries `PrevHash::Genesis` and flows through `self_accept → SelfAcceptedHandoff (DC-NODE-06) → ServedChainView (DC-NODE-07)`. The recovered lineage gates **permission**, not the prev_hash bytes (which are structurally `null`). Fires EXACTLY ONCE; the durable tip advances only through the accepted path; **no RO-LIVE-01/06 flip**. `code_locus = node_sync.rs (forge_header_position + forge_one_from_recovered + RecoveredTipMissingBlockNo); node_lifecycle.rs (may_cold_start_forge)`. **9 tests**; `ci_script = ci/ci_check_genesis_successor_reachability.sh`. |

### G-J strengthening (working-tree registry at this regen)

| Rule | Change | Why |
|------|--------|-----|
| `CN-REHEARSAL-FIDELITY-01` | `strengthened_in += "PHASE4-N-F-G-J"` (stays `tier = release`, `status = enforced`) | S5 extends the G-D non-promotable rehearsal discipline to a **second** private-testnet venue — the C1 **genesis-successor** rehearsal — reusing the same `correlate`-produced `PrivateRehearsalManifest` envelope and adding the genesis-rehearsal home to the schema gate. The rule is **strengthened** (a new venue is brought under the same non-promotability + path-fidelity discipline), never weakened. |

> **Status sequencing (load-bearing — DIFFERS from the G-D window).** Unlike G-D (where
> `CN-REHEARSAL-FIDELITY-01` was committed `declared` at the slice-span HEAD and flipped at
> close), all **three** G-J rules are committed **`enforced`** in-span (verified by reading
> the registry at `550eec3a`: `CN-NODE-04` / `CN-WIRE-09` / `DC-NODE-08` all `status =
> "enforced"`, `introduced_in = "PHASE4-N-F-G-J"`, with `tests` + `ci_script` already bound).
> The `CN-REHEARSAL-FIDELITY-01` strengthening is likewise present in the working-tree
> registry. So **no `declared → enforced` close-flip is owed** for the new IDs in this
> window; the close-pass commits the working-tree registry + sibling docs + this HEAD_DELTAS,
> not a rule-status flip.

**No rule was removed (expected: 0).** The 316 → 319 delta is three additive IDs
(`CN-NODE-04`, `CN-WIRE-09`, `DC-NODE-08`); the `CN-REHEARSAL-FIDELITY-01` change is a
`strengthened_in` append on an existing rule, never a removal.

## 8. Honest residual (cluster scope)

**G-J closes a NARROW claim: the genesis-successor forge MECHANISM (cold-start
reachability), the wire AUTHORITY (null/hash32 PrevHash + the position rule), and the
rehearsal HARNESS are enforced. It is NOT a live-pass, NOT a bounty / preview / preprod
completion claim, and it does NOT enforce that a C1 genesis run has succeeded.**

- **Mechanism + wire authority + harness, not a successful run.** G-J enforces that the node
  *can* forge a correctly-shaped genesis-successor (block 0 carries `PrevHash::Genesis` ⇄
  CBOR `null`, position-rule-checked), *can* reach that forge exactly once from a recovered
  base, and *if* a C1 genesis rehearsal is executed it is path-faithful + non-promotable. It
  does **not** enforce that any C1 run has happened: `ci_check_genesis_successor_reachability.sh`
  pins the permission gate (not an executed forge), and `ci_check_rehearsal_manifest_schema.sh`
  is **vacuous until a real operator-produced genesis-rehearsal manifest is committed** (only
  the README is committed; the env-gated `ADE_LIVE_C1_GENESIS_REHEARSAL` test is skipped in
  CI). The live C1 genesis rehearsal stays **`blocked_until_operator_c1_genesis_successor_rehearsal`**.
- **NO RO-LIVE flip; no bounty/preview/preprod claim.** G-J flips **no** RO-LIVE rule.
  `RO-LIVE-01` stays **`partial`**; `RO-LIVE-06` stays schema-only. The genesis-successor leg
  is C1 rehearsal infrastructure, **not** bounty evidence — preview/preprod acceptance remains
  the single bounty deliverable, captured separately.
- **No durable block-1+ progression.** The durable tip advances **only** through the accepted
  path, never from forge scheduling alone. G-J makes block 0 *reachable and correctly shaped*;
  it does **not** demonstrate a durable chain of forged blocks (block 1, 2, … with
  `PrevHash::Block` chaining) — that is downstream of an accepted, served genesis block.
- **No BLUE-authority weakening (byte-identity preserved).** The `Hash32 → PrevHash`
  migration is **shape-additive**: `PrevHash::Block(h)` encodes byte-identically to the
  pre-migration `Hash32` (independently asserted by the `forge_nonzero_block_emits_block_prev_byte_identical`
  / `block_header_prev_hash_byte_identical_after_migration` /
  `forged_block_zero_kes_preimage_equals_decoded_header_body_bytes` tests). `PrevHash::Genesis`
  adds a *new representable* predecessor (CBOR `null`) that the flat `Hash32` could not
  express — the all-zero `Hash32` cold-start stand-in is **deleted**, closing a latent
  wrong-shape risk rather than introducing one. +1 canonical type (456 → 457), all in BLUE
  `ade_types`/`ade_codec`/`ade_ledger`.
- **Emit-only diagnostic surface, not consensus evidence.** `CN-NODE-04`'s sched-event
  vocabulary is operational/diagnostic ONLY: emitting an event changes no forge scheduling,
  base, or authority, and the planner may emit but **never consume** the events. It is not a
  BA-02, acceptance, or agreement signal.
- **Fail-closed-on-ambiguity preserved.** `UnknownDisconnected` (a reason-less WirePump
  disconnect) is **INELIGIBLE** for the cold-start forge — no ambiguous disconnect may become
  forge-eligible (`option b`, amended in `b6554715`). The richer error reasons (`PeerLost` /
  `DecodeError` / `ProtocolError` / `SourceInvalid`) and a reason-enriched live `AtTip` are a
  **future wire-pump-enrichment prerequisite**, deliberately NOT in the closed set yet.

---

## Historical — PHASE4-N-F-G-D window (`6f848825 → 6bd60c80`)

> The section below is the **previous** HEAD_DELTAS lead, preserved verbatim. It narrates the
> **PHASE4-N-F-G-D** cluster (`6f848825..6bd60c80`). The intervening **G-E (`da205bff`,
> committed inside the G-D window), G-F, G-G, G-H, G-I** closes were each closed with their own
> grounding-doc refresh but **not** re-led in HEAD_DELTAS; their narratives live in their own
> close-pass commits. The current lead (above) measures from the G-I close `13028d49`.

> Baseline: `6f848825` (bound live-feed memory before authoritative decode — PHASE4-N-F-G-E S1, 2026-06-02 12:48)
> HEAD: `6bd60c80` (scan archived bounty home in rehearsal leak gate — PHASE4-N-F-G-D S4, 2026-06-02 17:10)
> Cluster: **PHASE4-N-F-G-D — private-testnet accepted-block bounty DRY-RUN fidelity harness on the `--mode node` spine**, slice span closed; close-pass commit to follow.
> 12 commits (no merges), 24 files changed, +2928 / -499 lines.

This window narrates the **PHASE4-N-F-G-D cluster** — a bounty **dry-run fidelity** cluster
that ships a **path-faithful, NON-PROMOTABLE rehearsal HARNESS** for the private-testnet
(C1) accepted-block dry-run, **with NO BLUE crate changed**. G-D answers one question early
— *will a real Haskell node accept an Ade-forged block when Ade has legitimate leader
rights?* — by exercising the **EXACT** preview/preprod `--mode node` accepted-block path in a
venue where the operator controls stake (a private genesis that makes Ade win slots fast).
The cluster is **four slices**, each fail-faithful to the shared path:

- **S1 (`d4d0f456`) — path fidelity.** Proves the C1 private dry-run uses the **same**
  `--mode node` accepted-block path as preview/preprod (`import_live_consensus_inputs` →
  forge → self-accept → sibling-serve → block-fetch → peer log → correlate) with **NO**
  private-only flag, branch, bootstrap authority, or from-genesis constructor — the only
  differences are operator **inputs** (the private genesis stake allocation) and the
  evidence **label** (S2). New gate `ci_check_node_path_fidelity.sh` pins the `cli.rs` flag
  set to a **28-flag closed allow-list** (guard a) and forbids any from-genesis
  consensus-inputs constructor while requiring `node_lifecycle.rs` to source consensus
  inputs only via the shared `import_live_consensus_inputs` (guard b).
- **S2 (`459cf78d`) — rehearsal evidence surface.** New **GREEN** `ade_node::rehearsal_evidence`
  (`PrivateRehearsalManifest` **wraps** a correlate-produced `Ba02Manifest` in a structurally
  distinct, non-promotable envelope; sole ctor `from_correlate_outcome` returns `None` on
  `NoEvidence`; `to_canonical_toml` always emits `is_rehearsal = true` + `not_bounty_evidence
  = true` as **literals**). New **RED** `ade_node::rehearsal_pass` (file I/O that **reuses**
  `ba02_pass::correlate_peer_log_file` verbatim and writes only a `PrivateRehearsalManifest`).
  New gate `ci_check_rehearsal_manifest_schema.sh` — vacuous-until-committed; closed
  **12-field** schema + private-testnet venue + the two non-promotability markers +
  `peer_log_file_sha256` binding; **three** non-promotability barriers.
- **S3 (`076a5af5`) — C1 dry-run runbook + operator scaffold.** A runbook that is a provable
  **strict subset** of the G-C preprod operator-pass runbook, plus the RED test file
  `node_c1_dry_run_rehearsal.rs` (a hermetic correlate→envelope proof + the env-gated
  `node_c1_dry_run_rehearsal_live` operator harness — a RED test skipped in CI, **NOT** a
  runtime node mode). **No** binary wiring, no new flag, no new gate, no synthetic manifest.
- **S4 (`6bd60c80`) — close-surfaced security fix.** The S2 rehearsal leak gate's barrier
  (b) was **dead** after G-C's archival (the pre-S4 `[[ -d ]]` guard skipped the whole check
  because the active home had moved to `docs/clusters/completed/`). S4 repoints the gate to
  scan **both** real bounty homes (active **and** archived) fail-closed and adds a durable
  regression test (`rehearsal_gate_archived_home.rs`).

The new rule `CN-REHEARSAL-FIDELITY-01` (`tier = release`, two coupled clauses — path
fidelity + evidence non-promotability) is **enforced** at this close (in the working-tree
registry; committed as `declared` at the slice-span HEAD — see the sequencing note below).
**G-D closes a NARROW claim: the rehearsal HARNESS is path-faithful and non-promotable.** It
does **NOT** enforce that a C1 run has succeeded (the rehearsal gate is **vacuous until a
real operator-produced manifest is committed**); it flips **NO** RO-LIVE rule; it makes **NO**
bounty / preview / preprod completion claim; the live C1 execution stays
`blocked_until_operator_c1_net_executed`.

The span also carries the **PHASE4-N-F-G-E close tail** (`da205bff`) — docs/registry only —
which lands inside this window because the baseline `6f848825` is the N-F-G-E *slice-span*
HEAD, not its close commit (see §1). This mirrors how the previous HEAD_DELTAS window
carried the **G-C** close tail (`351d46bc`), and the ones before that the **G-B** tail
(`febee120`) and the **G-A** tail (`62cb8718` + `1806584c`), for the same
slice-span-vs-close-commit reason.

### 0. Headline

| Count | Baseline (`6f848825`) | HEAD (`6bd60c80`) | Δ |
|---|---|---|---|
| CI gates (`ci/ci_check_*.sh`) | 119 | **121** | **+2 new** (`node_path_fidelity` G-D S1, `rehearsal_manifest_schema` G-D S2); none removed, none modified |
| Registry rules | 314 | **315** | **+1** (`CN-REHEARSAL-FIDELITY-01`, G-D); no other ID added — G-D records **no** `strengthened_in` bump (it does not advance the bounty deliverable) |
| Test attributes (`#[test]`/`#[tokio::test]`, workspace) | 2234 | **2241** | **+7** (the seven G-D tests, across `node_path_fidelity` + `rehearsal_pass` + `node_c1_dry_run_rehearsal` + `rehearsal_gate_archived_home`) |
| BLUE canonical types | 456 | 456 | **0 — NO BLUE change** (`git diff --name-only 6f848825..6bd60c80` matches **no** BLUE `core_paths` entry; the changed source is all `ade_node` — RED `rehearsal_pass` + GREEN-by-content `rehearsal_evidence` + the `lib.rs` `pub mod` lines) |

> **Registry / grounding-doc sequencing note (load-bearing — read before §7).** This window
> contains **two** distinct close events, and the registry / sibling-doc state differs
> between *committed at HEAD `6bd60c80`* and *the working tree at this regen*:
>
> - **The G-E close-pass (`da205bff`) is committed inside this window.** At HEAD `6bd60c80`
>   the registry therefore already reflects the G-E close: `DC-LIVEMEM-01` is `enforced`, and
>   the G-E gate `ci_check_live_feed_memory_bounds.sh` is committed. This is the close-pass
>   the **previous** HEAD_DELTAS window flagged as owed at its `6f848825` HEAD.
> - **The G-D close-pass is NOT yet committed.** At HEAD `6bd60c80` the new rule
>   `CN-REHEARSAL-FIDELITY-01` is committed as `status = "declared"` with `tests = []` and
>   `ci_script = ""` — but the **flip to `enforced`** and the binding of its `tests` +
>   `ci_script` arrays live only in the **working tree** at this regen (where the rule is
>   already `enforced`, `tests = [node_accepted_block_consensus_inputs_via_shared_import,
>   rehearsal_envelope_wraps_correlate_produced_payload,
>   rehearsal_correlate_no_evidence_writes_nothing,
>   rehearsal_envelope_is_structurally_distinct_from_ba02_manifest,
>   c1_dry_run_correlate_to_rehearsal_envelope, node_c1_dry_run_rehearsal_live,
>   rehearsal_gate_fails_on_archived_home_leak]`, `ci_script = "ci/ci_check_node_path_fidelity.sh,
>   ci/ci_check_rehearsal_manifest_schema.sh"`). The pending close-pass commits the flip and
>   the array bindings — exactly as the N-F-G-E close-pass committed the `DC-LIVEMEM-01` flip
>   after its slice span.
> - **Sibling-doc split (partial — not uniform).** The **registry** and **CODEMAP** working
>   trees have been regenerated to the **G-D** close state (registry: 315 rules,
>   `CN-REHEARSAL-FIDELITY-01` `enforced`; CODEMAP header: "456 canonical types, **2241**
>   tests, **121** CI checks at HEAD (`6bd60c80`, PHASE4-N-F-G-D cluster close)", with the
>   full G-D delta block and **32** `rehearsal_*` mentions). But **SEAMS and TRACEABILITY
>   are still the G-E close docs** — they have **not** been regenerated for G-D: TRACEABILITY
>   still reads "**314 rules** at this working tree" referencing `6f848825` / `DC-LIVEMEM-01`,
>   and neither SEAMS nor TRACEABILITY mentions `CN-REHEARSAL-FIDELITY-01`, the two new G-D
>   gates, or G-D at all. The **committed** CODEMAP / SEAMS / TRACEABILITY at HEAD `6bd60c80`
>   are all still the **G-E close** docs (119 CI checks, 2234 tests, 314 rules, no G-D delta).
>   `docs/ade-HEAD_DELTAS.md`, the **SEAMS/TRACEABILITY G-D regen**, and the `.idd-config.json`
>   baseline bump are what this regen / the close-pass still owe; the `.idd-config.json`
>   baseline has **already** been bumped to `6bd60c80` (the closer's edit), so `/head-deltas`
>   for this window uses the **explicit** `6f848825..6bd60c80` span, not the config value.
>
> **Not a rule removal, not a discipline violation** — a sequencing artifact reconciled by
> the close-pass.

### 1. Commit Log (newest first)

| Hash | Type | Summary |
|------|------|---------|
| `6bd60c80` | fix | scan archived bounty home in rehearsal leak gate (PHASE4-N-F-G-D S4) |
| `a8003dc8` | docs | specify PHASE4-N-F-G-D S4 rehearsal leak gate hardening |
| `076a5af5` | feat | C1 private-testnet dry-run runbook + operator scaffold (PHASE4-N-F-G-D S3) |
| `93472991` | docs | specify PHASE4-N-F-G-D S3 C1 dry-run scaffold |
| `459cf78d` | feat | add non-promotable rehearsal evidence surface (PHASE4-N-F-G-D S2) |
| `e3382532` | docs | specify PHASE4-N-F-G-D S2 rehearsal evidence |
| `d4d0f456` | feat | prove --mode node accepted-block path fidelity (PHASE4-N-F-G-D S1) |
| `c01ff046` | docs | specify PHASE4-N-F-G-D S1 path fidelity |
| `ce313927` | docs | define PHASE4-N-F-G-D bounty dry-run cluster |
| `bc39d431` | docs | plan PHASE4-N-F-G-D bounty dry-run fidelity |
| `07e6a340` | docs | declare PHASE4-N-F-G-D bounty dry-run fidelity |
| `da205bff` | (close) | Close PHASE4-N-F-G-E — peer-driven live-feed memory bounded before authoritative decode/apply |

No merge commits in the span.

The **last-listed** commit (`da205bff`) is the **PHASE4-N-F-G-E close-pass** that lands inside
this window because the baseline `6f848825` is the N-F-G-E *slice-span* HEAD, not its close
commit: `da205bff` flipped `DC-LIVEMEM-01` `declared → enforced`, regenerated the four
grounding docs to the **G-E** close, bumped the `.idd-config.json` baseline `90791691 →
6f848825`, and archived the G-E cluster. It is **docs/registry only** — no source change — and
**count-neutral on CI** (119 → 119; the G-E gate `live_feed_memory_bounds` was already committed
at the baseline `6f848825`, at G-E S1, so the close added no gate). The G-D cluster is then the
**three declare/plan/define docs** (`07e6a340` / `bc39d431` / `ce313927`) followed by **four
doc-then-impl slice pairs** (S1 `c01ff046`+`d4d0f456`, S2 `e3382532`+`459cf78d`, S3
`93472991`+`076a5af5`, S4 `a8003dc8`+`6bd60c80`).

(Plus the pending G-D close-pass commit: the `CN-REHEARSAL-FIDELITY-01` `declared → enforced`
flip + `tests`/`ci_script` array binding, the grounding-doc commit of the working-tree
CODEMAP/registry G-D regen **and** the still-owed SEAMS/TRACEABILITY G-D regen, the
`.idd-config.json` baseline bump confirmation (`6f848825 → 6bd60c80`, already applied) + the
registry-count comment, the G-D cluster-doc archive, and this HEAD_DELTAS.)

### 2. New Modules

Two new source modules, both in `ade_node` (added in **PHASE4-N-F-G-D S2**, `459cf78d`). Both
are confirmed **absent at the baseline** `6f848825` and registered by the two `pub mod` lines
added to `crates/ade_node/src/lib.rs` in the same slice.

| Module | Color | Purpose | Key sub-paths | Added in (cluster/slice) |
|--------|-------|---------|---------------|--------------------------|
| `ade_node::rehearsal_evidence` | **GREEN-by-content** (the file carries the deterministic Core-Contract banner; pure types + serializer, no I/O / clock / rand / float / `HashMap`; `ade_node` is neither a BLUE `core_paths` crate nor RED at crate level — this module is deterministic GREEN evidence by content) | The non-promotable rehearsal **envelope**: `PrivateRehearsalManifest` **wraps** a correlate-produced `Ba02Manifest` payload; closed 1-variant `RehearsalVenue::PrivateTestnetC1`; `RehearsalEnvelope` operator metadata (venue + peer-log filename + sha256). | `crates/ade_node/src/rehearsal_evidence.rs` — `PrivateRehearsalManifest` (`+ ba02 / venue / peer_log_file / peer_log_file_sha256` fields); **sole ctor** `from_correlate_outcome` (returns `None` on `BA02Outcome::NoEvidence` — nothing to wrap); `to_canonical_toml` (pure; emits `is_rehearsal = true` + `not_bounty_evidence = true` as **literals** — the type cannot represent a non-rehearsal); `REHEARSAL_MANIFEST_SCHEMA_VERSION = 1`; `toml_escape` helper. | `PHASE4-N-F-G-D` S2 (`459cf78d`) |
| `ade_node::rehearsal_pass` | **RED** (file I/O; `//! RED` banner; `fs::write` + `io::Result`) | Rehearsal-evidence **I/O**: reads the operator-captured peer-accept JSONL log, runs it through the GREEN correlate **verbatim**, and writes only a `PrivateRehearsalManifest`. Constructs no evidence, synthesizes no acceptance, uses no alternate correlator. | `crates/ade_node/src/rehearsal_pass.rs` — `correlate_peer_log_file_into_rehearsal` (**reuses** `ba02_pass::correlate_peer_log_file`; `Ok(None)` iff `NoEvidence`; missing/unreadable file → `io::Error`, fail closed); `write_private_rehearsal_manifest` (the **argument type is the gate** — only a `PrivateRehearsalManifest` is writable). Hosts the 3 S2 `#[cfg(test)]` proofs. | `PHASE4-N-F-G-D` S2 (`459cf78d`) |

> **Cross-reference warning (CODEMAP):** both modules appear in the **working-tree** CODEMAP
> (32 combined `rehearsal_evidence` / `rehearsal_pass` mentions, in the G-D delta block) — but
> **not** in the CODEMAP **committed at HEAD `6bd60c80`** (still the G-E close doc, 0 mentions).
> The G-D close-pass commits the working-tree CODEMAP regen. See the generation notes.

No new crate, no new workspace, no new BLUE authority, no new WAL/checkpoint/canonical type,
no new `CoordinatorEvent` variant was added. The cluster's only other source change is the
`lib.rs` `pub mod` wiring; S1/S3/S4 add **tests and CI**, not modules.

### 3. Modules Modified

The only modified source file in the window is `crates/ade_node/src/lib.rs` (the module
wiring); all G-D behavior arrives as **new** files (§2) plus tests and CI (§5). There are no
trivial/skipped source changes in this window.

| Module | Color | Scope | Key changes |
|--------|-------|-------|-------------|
| `ade_node` (crate root `lib.rs`) | neither BLUE nor RED at crate level (RED binary/library home) | +2 lines | **G-D S2 (`459cf78d`) — module registration.** Adds `pub mod rehearsal_evidence;` and `pub mod rehearsal_pass;`. No other behavioral change to `lib.rs`. |

The G-A…G-E forge / serve / live-feed / containment source surfaces are **unchanged** in this
window. In particular, no BLUE `ade_network` submodule, no `ade_runtime` shell file, and no
`ade_core` / `ade_ledger` / `ade_codec` / `ade_types` / `ade_crypto` / `ade_plutus` file
appears in `git diff --name-only 6f848825..6bd60c80` — every BLUE count is byte-unchanged.

### 4. Feature Flags

**No project feature-flag deltas.** Ade declares no `[features]` table in any workspace
`Cargo.toml` (confirmed absent at both refs), and no `#[cfg(feature = …)]` gate was introduced
in the span. **No `Cargo.toml` change at all in this window.** No coupling, no `compile_error!`
guard. (The C1 dry-run operator harness is gated by an **environment variable**
`ADE_LIVE_C1_DRY_RUN`, **not** a Cargo feature — it is a `#[test]` skipped in CI, not a
compile-time flag and not a runtime node mode.)

### 5. CI Checks (119 → 121; +2 new, 0 modified, 0 removed)

Two new gates, repo-root-relative and mirroring the existing `ci/ci_check_*.sh` convention.
The only files in `git diff --diff-filter=A 6f848825..6bd60c80 -- ci/` are these two; no `ci/`
file was modified or removed.

#### PHASE4-N-F-G-D gates

| Check | Status | Cluster origin | What it checks |
|-------|--------|----------------|----------------|
| `ci_check_node_path_fidelity.sh` | **New** | G-D S1 (`d4d0f456`) | Backs **`CN-REHEARSAL-FIDELITY-01` clause 1 (path fidelity)**. Guard (a): the `cli.rs` argv flag-literal set **equals** a pinned **28-flag closed allow-list** — G-D adds **no** flag, and a private-only / venue flag (`--private-net`, `--from-genesis`, `--devnet`, `--rehearsal`, …) would change the set and trip. Guard (b): **no** from-genesis consensus-inputs constructor exists (a fn whose name carries **both** `genesis` and `consensus`; line comments stripped first so prose naming the forbidden construct cannot self-trip), **and** `node_lifecycle.rs` sources consensus inputs only via the shared `import_live_consensus_inputs` (the same authority the preprod pass uses). Both fail-closed-smoke-verified against an injected `--private-net` flag + a `build_consensus_inputs_from_genesis` ctor. Hermetic. |
| `ci_check_rehearsal_manifest_schema.sh` | **New** (S2; hardened in S4) | G-D S2 (`459cf78d`), S4 fix (`6bd60c80`) | Backs **`CN-REHEARSAL-FIDELITY-01` clause 2 (evidence non-promotability)**. **Vacuous-until-committed**: when no `docs/evidence/phase4-n-f-g-d-private-rehearsal-*.toml` manifest is present (the typical state — the C1 dry-run is operator-gated, only the README is committed) the gate passes. When one is present it verifies the closed **12-field** schema, `schema_version == 1`, the `is_rehearsal = true` + `not_bounty_evidence = true` markers, a `venue = "private-testnet…"` venue, and that `peer_log_file_sha256` **matches** the committed peer-log file (the no-synthetic binding). **Three** non-promotability barriers: (1) distinct `docs/evidence/` home; (2) the rehearsal markers; (3) a cross-check that **no** rehearsal marker (`^is_rehearsal =` / `^not_bounty_evidence =`) appears in any `.toml` under a bounty home. **S4 hardening:** barrier (3) now scans **all** real bounty homes — `docs/clusters/PHASE4-N-F-G-C/` (active) **and** `docs/clusters/completed/PHASE4-N-F-G-C/` (archived) — by building the **existing-homes** list first (no `[[ -d ]]` whole-check skip): *home absent* ⇒ empty contribution (deliberate); a scan error on an **existing** home (`grep` rc ≥ 2) ⇒ **fail closed**, not swallowed. Hermetic. |

The three containment / handoff / memory fences are **byte-unchanged** in this window:
`ci_check_node_run_loop_containment.sh` (relay-loop containment), `ci_check_served_chain_handoff_fence.sh`
(self-accept→serve handoff), and `ci_check_live_feed_memory_bounds.sh` (G-E live-feed memory
bounds) are all untouched. G-D **added** two gates and relaxed **nothing**.

#### CI gate lineage (for cross-window readers — explains the headline +2)

The on-disk gate count rose **117 → 118 → 119 → 121** across the G-C…G-D sub-clusters:

- **117 → 118** — G-C **S2** added `ci_check_ba02_evidence_manifest_schema.sh` (narrated in the
  N-F-G-C HEAD_DELTAS window).
- **118 → 119** — G-E S1 added `ci_check_live_feed_memory_bounds.sh` (narrated in the **previous**,
  N-F-G-E, HEAD_DELTAS window `90791691 → 6f848825`). Because that gate was committed **at**
  this window's baseline `6f848825`, it is **not** a window-add here; the G-E close-pass
  `da205bff` was docs/registry only and count-neutral (119 → 119).
- **119 → 121** — G-D S1 + S2 added `ci_check_node_path_fidelity.sh` and
  `ci_check_rehearsal_manifest_schema.sh` (this window).

So **this** window's CI delta is the **two** G-D gates: **119 → 121 (+2)**.

> Cross-reference (TRACEABILITY): in the **working-tree registry**, both new gates are cited by
> `CN-REHEARSAL-FIDELITY-01.ci_script`. But **TRACEABILITY has not been regenerated for G-D** —
> neither the committed (`6bd60c80`) nor the working-tree TRACEABILITY mentions
> `ci_check_node_path_fidelity.sh`, `ci_check_rehearsal_manifest_schema.sh`, or
> `CN-REHEARSAL-FIDELITY-01` (both still report the **G-E** close, 314 rules,
> `DC-LIVEMEM-01`). The G-D close-pass owes the TRACEABILITY regen that renders these rows. See
> the warnings in the generation section.

### 6. Canonical Type Registry Delta

**n/a — no separate canonical-type registry is configured** (`canonical_type_registry: null`),
and **no BLUE crate changed**. The 456 BLUE canonical-type total is **unchanged (Δ0)** across
the span (independently re-verified: `git diff --name-only 6f848825..6bd60c80` matches **no**
`core_paths` (BLUE) entry; every changed source file is under `ade_node`: the new RED
`rehearsal_pass.rs`, the new GREEN-by-content `rehearsal_evidence.rs`, the `lib.rs` `pub mod`
wiring, and the three new test files). The working-tree CODEMAP at `6bd60c80` reports 456
canonical. The new `PrivateRehearsalManifest` / `RehearsalVenue` / `RehearsalEnvelope` types
are **GREEN-by-content and not canonical-counted** (they wrap, but do not extend, the existing
`Ba02Manifest`); no new type was added to any BLUE crate. **No new `CoordinatorEvent` variant
or field was introduced.**

### 7. Normative / Invariant Rule Delta (314 → 315)

**One rule ID added, zero removed** (314 → 315). G-D ships exactly one new rule; this window
also commits the **G-E** close-pass `DC-LIVEMEM-01` `declared → enforced` flip (`da205bff`),
which adds **no** new ID.

#### G-D new rule (committed in this span, by S2 `459cf78d`; flip to `enforced` owed at close)

| Rule | Tier | Status (committed at HEAD) | What it pins |
|------|------|---------------------------|--------------|
| `CN-REHEARSAL-FIDELITY-01` | `release` (bounty dry-run fidelity; **NOT** BLUE consensus law) | `declared` at `6bd60c80` (`tests = []`, `ci_script = ""`); **flips to `enforced`** at the close-pass (already `enforced` in the working-tree registry at this regen, with all 7 tests + both gates bound) | **Private-testnet accepted-block bounty dry-run fidelity** — two coupled clauses; if either fails the rehearsal becomes misleading. **(1) PATH FIDELITY:** the C1 private dry-run uses the **SAME** `--mode node` accepted-block path as preview/preprod (N-M-C `import_live_consensus_inputs` → forge → self-accept → sibling-serve → block-fetch → peer log → correlate) with **NO** private-only flag, branch, bootstrap authority, or from-genesis constructor; the only differences are operator-controlled **inputs** (private genesis stake) + the evidence **label**; no private-only helper may make the rehearsal pass if the same condition would fail on preview/preprod. **(2) EVIDENCE NON-PROMOTABILITY:** any private-testnet manifest is clearly marked `rehearsal` / `private-testnet`, stored **only** under the rehearsal home (`docs/evidence/phase4-n-f-g-d-private-rehearsal-*.toml`, never the bounty home `docs/clusters/PHASE4-N-F-G-C/CE-G-C-LIVE_*.toml`), sha256-bound to a real Haskell peer log, correlate-produced (`ba02_evidence::correlate` is the **sole** acceptance-evidence constructor), and flips **NO** RO-LIVE rule — C1 rehearsal evidence is **not** bounty evidence. `cross_ref = RO-LIVE-01, RO-LIVE-06, CN-OPERATOR-EVIDENCE-01, DC-NODE-06, DC-EPOCH-03, CN-NODE-02, CN-CINPUT-03, DC-CINPUT-02b`. Working-tree `tests = [node_accepted_block_consensus_inputs_via_shared_import, rehearsal_envelope_wraps_correlate_produced_payload, rehearsal_correlate_no_evidence_writes_nothing, rehearsal_envelope_is_structurally_distinct_from_ba02_manifest, c1_dry_run_correlate_to_rehearsal_envelope, node_c1_dry_run_rehearsal_live, rehearsal_gate_fails_on_archived_home_leak]`; working-tree `ci_script = ci/ci_check_node_path_fidelity.sh, ci/ci_check_rehearsal_manifest_schema.sh`. |

> **Status sequencing (mirrors the prior G-B/G-C/G-E windows).** At the committed HEAD
> `6bd60c80` (the G-D **S4 slice-span** HEAD), `CN-REHEARSAL-FIDELITY-01.status = "declared"`,
> `tests = []`, `ci_script = ""` — the rule **ID** is committed (so the count rises 314 → 315
> at the slice-span HEAD), but the `declared → enforced` flip **and** the `tests` / `ci_script`
> array bindings are the close-pass edit and live only in the **working-tree** registry at this
> regen (where the rule is already `enforced` with all 7 tests + both gates bound). This is the
> exact pattern the N-F-G-E window documented for `DC-LIVEMEM-01` (`declared` at its slice-span
> HEAD, flipped at close).

#### No cross-rule strengthenings recorded by G-D (load-bearing)

Unlike the G-C close-pass (four `strengthened_in` tokens), **G-D records NO `strengthened_in`
bump** — verified by grep over the registry at HEAD: `RO-LIVE-01`, `RO-LIVE-06`, and
`CN-OPERATOR-EVIDENCE-01` gain **no** `PHASE4-N-F-G-D` token. This is **deliberate and
load-bearing**: G-D is a dry-run harness and **does not advance the bounty deliverable**, so it
must not strengthen the release-obligation rules. `RO-LIVE-01` stays `partial`
(`blocked_until_operator_stake_available`); `RO-LIVE-06` stays `enforced` (schema-only); the
single bounty deliverable is preview/preprod acceptance, captured separately.

**No rule was removed (expected: 0).** The 314 → 315 delta is a single additive ID
(`CN-REHEARSAL-FIDELITY-01`); the `DC-LIVEMEM-01` flip carried by the G-E close-pass is a
status change on an existing rule, never a removal.

### 8. Honest residual (cluster scope)

**G-D closes a NARROW claim: the private-testnet (C1) accepted-block dry-run is a
PATH-FAITHFUL, NON-PROMOTABLE rehearsal HARNESS (enforced). It is NOT a live-pass, NOT a
bounty / preview / preprod completion claim, and it does NOT enforce that a C1 run has
succeeded.**

- **Harness, not a successful run.** G-D enforces that *if* a C1 dry-run is executed and an
  acceptance manifest committed, it is path-faithful (clause 1) and non-promotable (clause 2).
  It does **not** enforce that any C1 run has happened: `ci_check_rehearsal_manifest_schema.sh`
  is **vacuous until a real operator-produced manifest is committed** (only the README is
  committed under the rehearsal home — no `.toml` manifest), and the env-gated
  `node_c1_dry_run_rehearsal_live` test is **skipped in CI**. The live C1 execution stays
  **`blocked_until_operator_c1_net_executed`**.
- **NO RO-LIVE flip; no bounty/preview/preprod claim.** G-D flips **no** RO-LIVE rule and
  records **no** `strengthened_in` bump on `RO-LIVE-01` / `RO-LIVE-06` /
  `CN-OPERATOR-EVIDENCE-01`. `RO-LIVE-01` stays `partial`; `RO-LIVE-06` stays schema-only /
  `enforced`. **Private C1 acceptance ≠ bounty completion** — preview/preprod acceptance is the
  single bounty deliverable, captured separately.
- **No BLUE change.** 456 BLUE canonical types unchanged (Δ0); the new modules are RED
  (`rehearsal_pass`) + GREEN-by-content (`rehearsal_evidence`), and the only modified source
  file is `lib.rs` (module wiring). No new `CoordinatorEvent` variant or field; the new
  envelope types wrap, never extend, the BLUE-derived `Ba02Manifest`.
- **Fences byte-unchanged.** The three containment / handoff / memory fences
  (`ci_check_node_run_loop_containment.sh`, `ci_check_served_chain_handoff_fence.sh`,
  `ci_check_live_feed_memory_bounds.sh`) are **byte-unchanged**; the G-A…G-E forge / serve /
  live-feed surfaces are unchanged. G-D **added** two gates and relaxed nothing.
- **S4 close-surfaced security fix (record, do not soften).** S4 fixes a per-cluster
  security-review **HIGH**: the S2 rehearsal leak gate's non-promotability barrier (b) ("no
  rehearsal marker may live in a bounty-evidence home") was **dead** after G-C's archival — the
  pre-S4 `[[ -d ]]` guard checked **only** the active home `docs/clusters/PHASE4-N-F-G-C/`,
  which had moved to `docs/clusters/completed/PHASE4-N-F-G-C/`, so the whole leak scan silently
  skipped. The exact pre-S4 smuggle (a rehearsal-marked `.toml` under the archived home) passed
  with exit 0. S4 repoints the gate to scan **both** real homes fail-closed (existing-homes list
  built first; *home absent* ⇒ empty contribution; scan error on an existing home ⇒ fail closed)
  and adds the durable regression test `rehearsal_gate_fails_on_archived_home_leak` (clean tree
  ⇒ green; archived-home leak ⇒ **fails**; Drop-guarded fixture). Verified: the exact pre-S4
  smuggle now fails with exit 1.
- **Carried follow-ups (explicit, deferred by S4's tight scope — do not treat as closed).**
  Three hardenings are deliberately **out of S4's scope** and remain owed:
  1. **The BA-02 bounty gate's same stale active-home glob.**
     `ci_check_ba02_evidence_manifest_schema.sh` still globs **only**
     `find docs/clusters/PHASE4-N-F-G-C -name "CE-G-C-LIVE_*.toml"` — the *identical* bug S4
     fixed in the rehearsal gate, but on G-C's gate. A separate G-C-gate follow-up.
  2. **`toml_escape` control-character hardening.** `rehearsal_evidence::toml_escape` escapes
     only backslash + double-quote; control characters in operator-supplied fields are not yet
     handled. Deferred — it touches the serializer (left byte-unchanged by S4).
  3. **`pub`-field sole-constructor hardening.** `PrivateRehearsalManifest` / `RehearsalEnvelope`
     expose `pub` fields (so the "sole ctor `from_correlate_outcome`" non-promotability
     guarantee is by-convention on the struct literal, not type-enforced against a hand-built
     value). Inherited from G-C, project-wide; deferred.

---

## Generation notes

### Regen `13028d49 → 550eec3a` (PHASE4-N-F-G-J — current lead)

- **Explicit span, NOT the config baseline.** This regen was run against the **explicit**
  `13028d49..550eec3a` G-J span (`13028d49` = the PHASE4-N-F-G-I close; `550eec3a` = the G-J S5
  impl, the true working-tree tip). The `.idd-config.json` `head_deltas_baseline` is **two
  clusters stale** at `6bd60c80` (the G-D close) — reading it would mis-measure this window by
  re-including the entire G-E…G-I run. The close-pass should bump `head_deltas_baseline`
  `6bd60c80 → 550eec3a` (and update the stale `_invariant_registry_doc` "315 entries" comment to
  **319**).
- **Baseline gap (G-E…G-I not re-led).** HEAD_DELTAS was **not** re-led for the five intervening
  closes G-E/G-F/G-G/G-H/G-I; each was closed with its own grounding-doc refresh, and their
  narratives live in their own close-pass commits + the registry. This regen does **NOT
  fabricate** those narratives — it narrates only the explicit G-J span and preserves the G-D
  lead verbatim as the most recent surviving narrative (the next-older lead beyond G-D was
  itself overwritten by the G-D regen and is not reconstructable from this doc).
- Counts are mechanical (git/grep/ls only, no cargo): commit log + `--shortstat` over
  `13028d49..550eec3a` (**17** commits, no merges / **48** files / **+3587 / -84**); CI gate
  count via `git ls-tree -r --name-only <ref> ci/ | grep -c 'ci_check_.*\.sh'` at each ref
  (**123 → 126**, **+3 new** — `node_sched_events_emit_only` G-J S1, `prevhash_single_wire_authority`
  G-J S2, `genesis_successor_reachability` G-J S4 — plus `rehearsal_manifest_schema` **modified in
  place** at S5; `--diff-filter=A` over `ci/` lists exactly the three new gates, `--diff-filter=M`
  lists only `rehearsal_manifest_schema`, `--diff-filter=D` is empty); registry rule count via
  `grep -c '^id = '` at each ref (**316 → 319**; `diff` of sorted `^id =` lines shows exactly
  three `>` adds — `CN-NODE-04`, `CN-WIRE-09`, `DC-NODE-08` — and zero `<` removals); workspace
  test attributes via `git grep -hE '#\[(tokio::)?test\]'` over `crates/**/*.rs` (**2245 →
  2284**, +39); BLUE canonical types **456 → 457** (the new `PrevHash` sum type in
  `ade_types::shelley::block`, confirmed absent at `13028d49` via `git show
  13028d49:crates/ade_types/src/shelley/block.rs | grep -c 'enum PrevHash'` = 0).
- **All three G-J rules committed `enforced` in-span (NO close-flip owed).** Verified by reading
  the registry at `550eec3a`: `CN-NODE-04` / `CN-WIRE-09` / `DC-NODE-08` are all `status =
  "enforced"`, `introduced_in = "PHASE4-N-F-G-J"`, with `tests` + `ci_script` bound; and
  `CN-REHEARSAL-FIDELITY-01.strengthened_in = ["PHASE4-N-F-G-J"]`. This differs from the G-D
  window's `declared`-at-slice-span-then-flip-at-close pattern: G-J needs **no** rule-status flip
  at close — only the working-tree grounding-doc commit + baseline bump + this HEAD_DELTAS.
- **Sibling-doc coherence is FULL on counts, with ONE SHA-label caveat (load-bearing).** At this
  regen CODEMAP / SEAMS / TRACEABILITY are all dirty (`git status`) and **G-J-regenerated**:
  CODEMAP header "11 crates, **457 canonical types, 2284 tests, 126 CI checks** … PHASE4-N-F-G-J
  cluster close" (27 `header_position` + 19 `sched_event`/`sched_writer` mentions); TRACEABILITY
  "**319 rules** … `+3` this close … `CN-WIRE-09`, `DC-NODE-08`, `CN-NODE-04` …
  `CN-REHEARSAL-FIDELITY-01` is `strengthened_in += PHASE4-N-F-G-J`", with all G-J leaves +
  gates cross-checked present on disk. **All three sibling-doc count-sets AGREE with this doc.**
  **SHA labels reconciled at close:** all four grounding docs label the close HEAD
  **`550eec3a`** (the S5 impl); the in-span S2 CI-fix **`36b2216f`** (an ancestor) is referenced
  only where it is the actual commit. The *counts* are the `550eec3a` figures — **not** a content
  divergence.
- **Not a rule removal, not a discipline violation** — the SHA-label discrepancy and the stale
  config baseline are sequencing/labeling artifacts reconciled by the close-pass (which commits
  the working-tree CODEMAP / SEAMS / TRACEABILITY / registry G-J regen, bumps
  `.idd-config.json` `head_deltas_baseline` `6bd60c80 → 550eec3a`, updates the registry-count
  comment to 319, archives the G-J cluster doc, and commits this HEAD_DELTAS). The next gating
  work is the **operator-witnessed live pass** — the C1 genesis-successor rehearsal
  (`blocked_until_operator_c1_genesis_successor_rehearsal`) and, for the bounty deliverable, the
  separate preview/preprod acceptance pass — **neither advanced by G-J**, which ships only the
  genesis-successor forge mechanism + wire authority + cold-start reachability + the rehearsal
  harness ahead of them.

### Regen `6f848825 → 6bd60c80` (PHASE4-N-F-G-D — historical)

- **Explicit span, NOT the config baseline.** This regen was run against the **explicit**
  `6f848825..6bd60c80` span. The `.idd-config.json` `head_deltas_baseline` has **already** been
  bumped to `6bd60c80` (the closer's edit for the **next** cluster), so reading it would
  mis-measure this window; the explicit baseline `6f848825` (the PHASE4-N-F-G-E slice-span HEAD)
  is correct for the G-D close. The `_invariant_registry_doc` comment in `.idd-config.json`
  already reads **"315 entries at HEAD"** (the closer's edit).
- Counts are mechanical (git/grep/ls only, no cargo): commit log + `--shortstat` over
  `6f848825..6bd60c80` (**12** commits, no merges / **24** files / **+2928 / -499**); CI gate
  count via `git ls-tree -r --name-only <ref> ci/ | grep -c 'ci_check_.*\.sh'` at each ref
  (**119 → 121**, **+2 new** — `node_path_fidelity` at G-D S1, `rehearsal_manifest_schema` at
  G-D S2; the only files in `git diff --diff-filter=A 6f848825..6bd60c80 -- ci/` are those two;
  `--diff-filter=M` and `--diff-filter=D` over `ci/` are empty — none modified, none removed);
  registry rule count via `grep -c '^id = '` at each ref (**314 → 315**, the single new ID
  `CN-REHEARSAL-FIDELITY-01`; `diff` of sorted `^id =` lines shows exactly one `>` add and zero
  `<` removals); workspace test attributes via `git grep -hE '#\[(tokio::)?test\]'` over
  `crates/**/*.rs` (**2234 → 2241**, +7 — 1 in `node_path_fidelity.rs`, 3 in `rehearsal_pass.rs`,
  2 in `node_c1_dry_run_rehearsal.rs`, 1 in `rehearsal_gate_archived_home.rs`); BLUE canonical
  types unchanged at 456 (no BLUE `core_paths` file in the diff — every changed source file is
  under `ade_node`).
- **Two close events in one window (NOT an anomaly).** The **G-E** close-pass (`da205bff`) is
  **committed** inside this window (because the baseline `6f848825` is the G-E slice-span HEAD,
  not its close commit) — it flipped `DC-LIVEMEM-01` to `enforced`, refreshed the four grounding
  docs to the **G-E** close, and bumped the baseline `90791691 → 6f848825`. The **G-D** close-pass
  is **not yet committed**. So at HEAD `6bd60c80`: the registry has `DC-LIVEMEM-01` `enforced`
  and `CN-REHEARSAL-FIDELITY-01` as `declared` (the `enforced` flip + array binding are
  working-tree only); and the **committed** CODEMAP / SEAMS / TRACEABILITY are still the **G-E**
  close docs (119 CI, 2234 tests, 314 rules, no `CN-REHEARSAL-FIDELITY-01`).
- **Sibling-doc coherence is PARTIAL at this regen (load-bearing).** The **registry** and
  **CODEMAP** **working trees** have been regenerated to the **G-D** close state — registry: 315
  rules, `CN-REHEARSAL-FIDELITY-01` `enforced` with `tests`/`ci_script` bound; CODEMAP header:
  "456 canonical types, 2241 tests, 121 CI checks at HEAD (`6bd60c80`, PHASE4-N-F-G-D cluster
  close)", with the full G-D delta block and 32 `rehearsal_*` mentions — and they **agree** with
  this doc's counts. But **SEAMS and TRACEABILITY have NOT been regenerated for G-D**:
  TRACEABILITY still reads "314 rules at this working tree" referencing `6f848825` /
  `DC-LIVEMEM-01`, and neither SEAMS nor TRACEABILITY mentions `CN-REHEARSAL-FIDELITY-01`, the
  two new G-D gates, or G-D. `git status` at this regen shows only `.idd-config.json`,
  `docs/ade-CODEMAP.md`, and `docs/ade-invariant-registry.toml` dirty (the SEAMS/TRACEABILITY
  G-D regen is still owed by the close-pass).
- **CI cross-reference warning.** Both new G-D gates are cited by `CN-REHEARSAL-FIDELITY-01`
  only in the **working-tree registry** — **TRACEABILITY does not yet cite them** (it is still
  the G-E close doc, in both the committed and working-tree copies). The close-pass commits the
  `ci_script` binding's enforced state and regenerates TRACEABILITY so these two gates render
  against `CN-REHEARSAL-FIDELITY-01`. (The G-E gate `live_feed_memory_bounds` and the G-C BA-02
  gate are already cited in the committed TRACEABILITY, via `da205bff` / earlier.)
- **Not a rule removal, not a discipline violation** — the working-tree-vs-committed split (and
  the partial sibling-doc regen) is a sequencing artifact reconciled by the close-pass, which
  commits the `CN-REHEARSAL-FIDELITY-01` `declared → enforced` flip + `tests`/`ci_script` array
  binding, the CODEMAP/registry working-tree regen, the **owed** SEAMS/TRACEABILITY G-D regen,
  the `.idd-config.json` baseline bump confirmation (`6f848825 → 6bd60c80`, already applied) +
  the registry-count comment, the G-D cluster-doc archive, and this HEAD_DELTAS.
