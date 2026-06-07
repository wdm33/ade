# Ade — HEAD Deltas (Changes Since Baseline)

> **Status:** Living architectural document. Regenerated; not hand-edited.
>
> Regenerate with `/head-deltas <baseline>` after every cluster close. Baseline is recorded in `.idd-config.json` `head_deltas_baseline`.

> Baseline: `25ddeebd` (grounding-doc refresh for PHASE4-N-AC close, 2026-06-06 11:48)
> HEAD: `a76672b9` (AE.E chain-sync server FindIntersect cursor — CE-A5 manifest achieved, 2026-06-07 12:26)
> Span: **the PHASE4-N-AC close-refresh tail + the PHASE4-N-AD cluster + the C2-LOCAL guide/finding tail + the PHASE4-N-AE cluster** — the prior-lead's PHASE4-N-AC grounding-doc refresh (`25ddeebd`, the window baseline) is the span-opening commit, followed by **PHASE4-N-AD — tip-successor durability proof** (test-only), a run of **C2-LOCAL preprod-tip / cardano-testnet venue guides + discovered-gaps findings** (docs-only), and the closing cluster **PHASE4-N-AE — Recover→Serve Continuity and Forge Admissibility**, which achieved the **CE-A5 manifest**: a real `cardano-node 11.0.1` relay **adopting an Ade-forged block** as its current chain tip.
> 19 commits (no merges), 24 files changed, +3635 / −129 lines.

> **Baseline note (load-bearing — read before §0).** This window's baseline is **`25ddeebd`**, the
> PHASE4-N-AC grounding-doc-refresh commit that *wrote the previous HEAD_DELTAS lead* — and it is
> **valid**: `git rev-parse 25ddeebd` resolves and `git merge-base 25ddeebd HEAD == 25ddeebd` (it is a
> strict ancestor of HEAD; `25ddeebd` carries no tag). HEAD is **`a76672b9`** (the PHASE4-N-AE.E CE-A5
> closer). The config baseline at the start of this regen was `1d54abb4` (the PHASE4-N-AC close *impl*);
> `25ddeebd` is one commit later (the N-AC grounding refresh), so this window measures from the doc-refresh
> commit forward and the `1d54abb4..25ddeebd` step (the N-AC refresh itself) is folded into the
> span-opening tail. The span has **three parts**: (1) the span-opening commit `25ddeebd` — the
> *grounding-doc refresh for the PHASE4-N-AC close* (it carried `DC-CRYPTO-10` into all four grounding
> docs); (2) the **PHASE4-N-AD cluster** (3 commits) — a **test-only** durability-proof cluster
> (tip-successor WAL replay regression); and (3) — after a docs-only C2-LOCAL guide/finding run — the
> **PHASE4-N-AE cluster** (8 commits across 4 impl slices), the **Recover→Serve Continuity and Forge
> Admissibility** cluster that **achieved the CE-A5 manifest**. The closer bumps `head_deltas_baseline`
> `1d54abb4 → a76672b9` after this regen so the next cluster measures from here.

This window is **led by the CE-A5 cluster: PHASE4-N-AE — Recover→Serve Continuity and Forge
Admissibility.** It is the cluster that turned the long-standing producer/serve pipeline into a **proven
end-to-end live result**: a **real `cardano-node 11.0.1` relay `AddedToCurrentChain` an Ade-forged
successor block** (block 17 @ slot 421, hash `db3b5675…`, issuerHash `a1ed4e04…` == `blake2b-224(pool1`
cold VK`)`; relay node2 forging = 0; Ade forge `succeeded = 1`) — the **CE-A5 manifest**, evidence at
`docs/evidence/phase4-n-ae-ce-a5-relay-adoption.{md,jsonl}`. Before N-AE the recover→follow→forge→serve
pipeline existed but the **adopt** half failed across four CE-A5 reruns with the real relay rejecting
`HeaderEnvelopeError (UnexpectedBlockNo (BlockNo N) (BlockNo 0))` and falling back to `Origin`. N-AE
closed that across **four implementation slices** (committed in the order A → C → B → E):

- **AE.A — forge-on-followed-tip admission gate (`5f2afc2a`; `DC-NODE-15` + `DC-CONS-24 → enforced`,
  `DC-NODE-14 → partial`).** The `--mode node` `ForgeTick` arm's **recovered-tip forge-base fallback**
  (`node_lifecycle.rs` `None => act.recovered.tip.clone()`) is **removed**: the forge base is the
  **durable servable tip** (`ChainDb::tip()`), and a forge is admissible **only when**
  `durable_servable_tip == followed_peer_tip` (hash **and** `block_no`). A closed **GREEN** classifier
  `forge_followed_tip_admission` returns `CaughtUp` iff both tips are present and equal, else
  `NotCaughtUp{reason}` (`NoFollowedPeerTip | NoDurableServableTip | TipMismatch`); a `NotCaughtUp` records
  a typed `ForgeRefused::NotCaughtUp{local_servable_tip, followed_peer_tip, reason}` into
  `ForgeActivation.last_forge_refused` (a structured local observation, never log-string-only) and the
  forge **does not fire** (no state transition, tip unchanged) — mechanically distinct from a forge
  `Failed`. The followed-peer-tip signal is a forge-**admissibility** input only — it may *prevent* a
  forge but never reaches `select_best_chain` / `chain_selector` / `fork_choice` (gate static-grep
  enforced). New gate `ci_check_forge_followed_tip_admission.sh`.
- **AE.C — recover→follow WAL prior-fp seeding (`5425b23c`; `DC-WAL-02` + `T-REC-05` strengthened).** The
  live recover→follow path seeds the follow `ForwardSyncState.prior_fp` with
  **`fingerprint(&state.ledger).combined`** (the recovered ledger-tip post-fp) instead of the all-zero
  `Hash32([0u8;32])`. The CE-A5 live run surfaced `node_lifecycle` seeding `prior_fp` with **zero**, so the
  first followed `AdmitBlock`'s `prior_fp` was `0`, not the recovered ledger-tip post-fp, and a
  recover→followed store failed warm-start (`ChainBreak@1`, exit 42). Fixed at **both** lifecycle sites
  (forge-off + forge-on); `recover_follow_zero_seed_chainbreaks` reproduces the break,
  `recover_follow_kill_warm_start_chains_from_ledger_fp` proves the fix. New gate
  `ci_check_recover_follow_wal_lineage.sh` (fences the live seed **without** loosening `verify_chain`).
- **AE.B — recovered/forge-parent intersectability, Option B (`450c6992`; `DC-NODE-14` partial→enforced,
  `CN-CONS-07` + `DC-CONS-23` strengthened).** `ChainDbServedSource::intersect` projects the `prev_hash`
  of the **earliest servable `StoredBlock`** (the forge parent) as a **FindIntersect-only**, **proof-gated**
  intersect point **iff** a real servable successor exists; it **never serves bytes** for it
  (`get_block_by_hash` / `serve_range` stay empty → BlockFetch refuses structurally; **no synthetic
  `StoredBlock`**). A recover-only store (no successor) yields **no projection** (fail-closed). Backed by an
  **additive BLUE** field — `ade_ledger::block_validity::DecodedBlock.prev_hash` is exposed (it was
  **already parsed** for `check_header_position`) so the projection can prove the parent is the parent of a
  real servable successor. New gate `ci_check_recovered_anchor_intersectable.sh` (fences FindIntersect-only
  + proof-gated + no synthetic bytes).
- **AE.E — chain-sync SERVER FindIntersect cursor fix (`a76672b9`; `DC-PROTO-10` NEW enforced; `CN-CONS-06`
  + `DC-NODE-14` strengthened).** **The CE-A5 closer.** After the producer chain-sync **server** answers
  `IntersectFound(point)`, it now **sets its read cursor** (`state.last_announced`) to that point — so the
  next `RequestNext` serves `next_after(point)` (the successor the client rolls forward onto), **never**
  `next_after(None)` (the chain start). Across four prior CE-A5 reruns the server resolved the intersect
  (the relay's own tip) and replied `IntersectFound`, but **left the cursor unset**, so the next
  `RequestNext` served **block 0** to a client whose read pointer was its own tip — which the relay
  rejected as `UnexpectedBlockNo(tip_block_no + 1)(0)`. Origin-sync clients were unaffected (`Origin → None`
  is correct) — which is why the earlier producer-serve clusters (N-G) passed. With the fix, venue `c2ae18`
  produced the **CE-A5 manifest**. The fix is a **BLUE** change to `ade_network::chain_sync::server` —
  additive cursor-threading logic + one regression test; **no new type, no grammar change**.

**The headline:** the **recover→follow→forge→serve→ADOPT** path is now proven **end-to-end live** — a real
`cardano-node 11.0.1` relay adopted an Ade-forged block as its current chain tip (**CE-A5 manifest**).
**Both gating reviews PASS** for the cluster work. The span is **BLUE-additive but +0 canonical type**:
the two BLUE files touched (`ade_ledger::block_validity::header_input` — additive `DecodedBlock.prev_hash`
field; `ade_network::chain_sync::server` — additive FindIntersect-cursor logic) add **zero** new
`struct`/`enum` (BLUE canonical-type count **458 → 458**). **No `RO-LIVE` rule was flipped** in the
registry this span (the CE-A5 manifest is recorded as `enforced`-backing evidence on `DC-NODE-14` /
`DC-PROTO-10`, not a `RO-LIVE-01` status flip).

## 0. Headline

| Count | Baseline (`25ddeebd`) | HEAD (`a76672b9`) | Δ |
|---|---|---|---|
| CI gates (`ci/ci_check_*.sh`) | 138 | **141** | **+3** — **three NEW gates** (`--diff-filter=A` over `ci/`): `ci_check_forge_followed_tip_admission.sh` (AE.A), `ci_check_recover_follow_wal_lineage.sh` (AE.C), `ci_check_recovered_anchor_intersectable.sh` (AE.B). **No gate removed** (`--diff-filter=D` over `ci/` empty). **One non-gate CI script modified** (`--diff-filter=M`): `ci/build_consensus_inputs_bundle.sh` (the venue-general consensus-inputs extractor — now reads `epochLength` + `activeSlotsCoeff` from the venue's `shelley-genesis` instead of hardcoding preprod's `432000` / `1/20`, so the SAME extractor is correct for a short-epoch C2-LOCAL rehearsal venue). It is **not** a `ci_check_*.sh` gate, so it does not move the gate count. |
| Registry rules (`docs/ade-invariant-registry.toml`) | 336 | **340** | **+4** — four NEW rules **`DC-CONS-24`**, **`DC-NODE-14`**, **`DC-NODE-15`** (all PHASE4-N-AE), **`DC-PROTO-10`** (PHASE4-N-AE.E). **Zero removed** (`diff` of the sorted `id =` lists shows exactly the four additions and no removal). |
| Registry status (enforced / partial / declared) | 204 / 20 / 112 | **208 / 20 / 112** | **+4 enforced** — all four new rules land `enforced` at HEAD (`DC-NODE-14` lands `partial` at AE.A, then `enforced` at AE.B). Partial / declared counts unchanged. |
| Registry strengthenings | — | **9** (8 distinct rules) | `strengthened_in += "PHASE4-N-AD"` on **2** rules (`DC-WAL-04`, `T-REC-05`); `strengthened_in += "PHASE4-N-AE"` on **7** rules (`DC-EPOCH-03`, `CN-CONS-06`, `CN-CONS-07`, `DC-WAL-02`, `DC-NODE-05`, `T-REC-05`, `DC-CONS-23`). `T-REC-05` was strengthened by **both** N-AD and N-AE (8 distinct rules touched). All strengthenings, **no** rule weakened. |
| BLUE canonical types | 458 | **458** | **0** — **BLUE-additive, +0 type.** The span touches **two** BLUE `core_paths` files: `ade_ledger::block_validity::header_input` (+8 / −1 — exposes the already-parsed `DecodedBlock.prev_hash` field for the AE.B intersect proof) and `ade_network::chain_sync::server` (+80 / −4 — the AE.E FindIntersect-cursor fix + its regression test). Both are **additive** — a new public *field* on an existing struct and new *logic* + a test — adding **zero** `^+(pub )?(struct\|enum)` lines (verified mechanically). All other source changes are RED (`ade_node::{node_sync, node_lifecycle}`) or RED-shell (`ade_runtime::network::served_chain_projection`). |
| Grounding docs | refreshed for the **N-AC** close in `25ddeebd` (span-opening) | **CODEMAP/SEAMS/TRACEABILITY already refreshed for the N-AE close; this HEAD_DELTAS completes the set** | The span-opening commit `25ddeebd` refreshed all four grounding docs for the **N-AC** close (it carried `DC-CRYPTO-10`). The other three grounding docs are **already current at HEAD `a76672b9`** in this close pass: **CODEMAP** is fully regenerated (header `a76672b9`, 458 types / 141 CI / 340 rules, full N-AE coverage incl. the load-bearing BLUE-vs-GREEN classification of `chain_sync/server.rs` as **BLUE**); **TRACEABILITY** is re-pinned to HEAD `a76672b9` (registry 340; CODEMAP cross-ref `a76672b9` / 458 / 141 / 340); **SEAMS** is re-pinned to HEAD `a76672b9` (340 entries / 141 CI; records the 4 new rules + 3 new gates). This HEAD_DELTAS is the last of the four to be brought current. The registry records the rules + bindings authoritatively at HEAD (340 rules). |

This is a **multi-part lead** — the N-AC close-refresh tail, the **test-only PHASE4-N-AD** durability
cluster, a docs-only C2-LOCAL guide/finding run, and the **CE-A5 cluster PHASE4-N-AE**. The
slice↔rule↔gate map for the N-AE cluster:

| Slice | Rule(s) | Gate | What shipped |
|---|---|---|---|
| **AE.A** (`5f2afc2a`) | **`DC-NODE-15`** + **`DC-CONS-24`** (NEW, enforced); **`DC-NODE-14`** (NEW, partial) | **`ci_check_forge_followed_tip_admission.sh`** (NEW) | Forge-on-followed-tip admission gate: recovered-tip forge-base fallback removed; GREEN `forge_followed_tip_admission` classifier; typed `ForgeRefused::NotCaughtUp`; followed-peer-tip is an admissibility input only (never reaches chain selection). |
| **AE.C** (`5425b23c`) | `DC-WAL-02` + `T-REC-05` (**strengthened**) | **`ci_check_recover_follow_wal_lineage.sh`** (NEW) | Live recover→follow `ForwardSyncState.prior_fp = fingerprint(&state.ledger).combined` (was all-zero `Hash32`); fixed at both forge-off/forge-on lifecycle sites; `verify_chain` first-entry clause now enforced on the live path **without** loosening it. |
| **AE.B** (`450c6992`) | **`DC-NODE-14`** (partial→**enforced**); `CN-CONS-07` + `DC-CONS-23` (**strengthened**) | **`ci_check_recovered_anchor_intersectable.sh`** (NEW) | `ChainDbServedSource::intersect` projects the earliest servable `StoredBlock`'s `prev_hash` as a **FindIntersect-only**, proof-gated intersect point iff a real successor exists; never serves bytes for it; additive BLUE `DecodedBlock.prev_hash` exposure. |
| **AE.E** (`a76672b9`) | **`DC-PROTO-10`** (NEW, enforced); `CN-CONS-06` + `DC-NODE-14` (**strengthened**) | *(no dedicated gate — regression-test enforced)* | **CE-A5 closer.** Producer chain-sync server `producer_chain_sync_serve` now sets `state.last_announced` to the resolved intersect point after `IntersectFound`; the next `RequestNext` rolls the client forward past it (not block 0). Regression test `producer_chain_sync_serve_find_intersect_sets_cursor_then_rolls_forward_past_it`. |

The per-commit shape:

| Commit | Kind | What it did | Code / CI / registry effect |
|---|---|---|---|
| `25ddeebd` | docs (prior-lead tail) | Grounding-doc refresh for the PHASE4-N-AC close (CODEMAP/TRACEABILITY/SEAMS/HEAD_DELTAS) | **0 code / 0 CI**; refreshed the four grounding docs to carry `DC-CRYPTO-10`; no rule added/removed |
| `68d83406` | docs (cluster+slice doc) | PHASE4-N-AD cluster + slice doc (tip-successor durability proof) | **0 code / 0 CI / 0 registry** |
| `67ce7ac6` | test(node) | PHASE4-N-AD S1 — tip-successor durability regression (WAL replay) | **RED test** (`node_sync.rs` +214); **0 CI**; registry: 2 strengthenings (`DC-WAL-04`, `T-REC-05` += N-AD); C1 README addendum |
| `0e6bff35` | chore (close) | Close PHASE4-N-AD — tip-successor durability proof | **0 code / 0 CI / 0 registry**; archives N-AD docs to `docs/clusters/completed/PHASE4-N-AD/` |
| `00144008` … `bc05cb81` | docs (C2-LOCAL guides + findings) | C2 preprod-tip guide (Conway-from-Mithril) + venue-general extractor; C2-LOCAL cardano-testnet venue recipe; discovered-gaps findings (Gap 2 forge-vs-follow race isolated into 2a/2b); N-AE Slice A invariants sketch | **0 code**; **1 CI script MODIFIED** (`build_consensus_inputs_bundle.sh` — venue-general epoch length + ASC, in `00144008`); new planning/active docs; **0 registry** |
| `48a8009c` | docs (cluster doc) | PHASE4-N-AE cluster doc; **declare `DC-NODE-14` / `DC-NODE-15` + `DC-CONS-24`** | **0 code / 0 CI**; registry: 3 rules added `declared` |
| `2108b825` | docs (slice doc) | AE.A slice doc (forge-on-followed-tip gate + serve continuity) | **0 code / 0 CI / 0 registry** |
| `5f2afc2a` | feat (AE.A impl) | AE.A — forge-on-followed-tip admission gate | **RED code** (`node_lifecycle.rs` +229, `node_sync.rs` +310); **+1 CI** (`ci_check_forge_followed_tip_admission.sh`); registry: `DC-NODE-15` + `DC-CONS-24 → enforced`, `DC-NODE-14 → partial` |
| `a058d649` | docs (slice doc) | AE.C slice doc (recover→follow WAL prior-fp seeding) | **0 code / 0 CI / 0 registry** |
| `5425b23c` | fix (AE.C impl) | AE.C — recover→follow WAL prior-fp seeding | **RED code** (`node_lifecycle.rs` +13, `node_sync.rs` +203); **+1 CI** (`ci_check_recover_follow_wal_lineage.sh`); registry: `DC-WAL-02` + `T-REC-05` += N-AE |
| `238aff61` | docs (slice doc) | AE.B slice doc (recovered/forge-parent intersectability, Option B) | **0 code / 0 CI / 0 registry** |
| `450c6992` | fix (AE.B impl) | AE.B — recovered/forge-parent intersectability (Option B) | **BLUE-additive + RED** (`header_input.rs` BLUE +8 / −1; `served_chain_projection.rs` RED +49; `node_sync.rs` RED +142); **+1 CI** (`ci_check_recovered_anchor_intersectable.sh`); registry: `DC-NODE-14 → enforced`, `CN-CONS-07` + `DC-CONS-23` += N-AE |
| `a76672b9` | fix (AE.E impl, CE-A5 closer) | AE.E — chain-sync server FindIntersect cursor; **CE-A5 manifest** | **BLUE-additive** (`chain_sync/server.rs` +80 / −4); **0 CI** (regression-test enforced); registry: `DC-PROTO-10` NEW `enforced`, `CN-CONS-06` + `DC-NODE-14` += N-AE; CE-A5 evidence `{md,jsonl}` |

## 1. Commit Log (newest first)

| Hash | Type | Summary |
|------|------|---------|
| `a76672b9` | fix | AE.E chain-sync server FindIntersect cursor — CE-A5 manifest achieved (PHASE4-N-AE.E, DC-PROTO-10) |
| `450c6992` | fix | AE.B recovered/forge-parent intersectability (Option B) — DC-NODE-14 enforced (PHASE4-N-AE.B) |
| `238aff61` | docs | AE.B slice — recovered/forge-parent intersectability (Option B), promoted to CE-A5 closer |
| `5425b23c` | fix | AE.C recover→follow WAL prior-fp seeding (PHASE4-N-AE.C, DC-WAL-02 + T-REC-05) |
| `a058d649` | docs | AE.C slice — recover→follow WAL prior-fp seeding |
| `5f2afc2a` | feat | PHASE4-N-AE.A — enforce forge-on-followed-tip admission |
| `2108b825` | docs | AE.A slice doc — forge-on-followed-tip gate + followed-block serve continuity |
| `48a8009c` | docs | cluster doc — recover→serve continuity + forge admissibility; declare DC-NODE-14/15 + DC-CONS-24 |
| `bc05cb81` | docs | c2-guide §5b status → committed PHASE4-N-AE Slice A invariants gate + grounded root cause |
| `e32afd8b` | docs | PHASE4-N-AE Slice A invariants sketch — recover→serve continuity + forge-on-followed-tip gate (Gap 2) |
| `19a400e2` | docs | C2 recover-far-behind run isolates Gap 2 into 2a (forge-on-followed-tip) + 2b (serve-continuity); scope impl slices |
| `42b42194` | docs | C2 C2-LOCAL #8 not proven — relay venue removed Gap 1, exposed Gap 2 (forge-vs-follow race); gaps recorded |
| `20e20c12` | docs | c2-guide C2-LOCAL #1-7 proven (Ade forged pool1 blocks); #8 finding — node2/node3 must be non-producing relays |
| `1a4df0a4` | docs | c2-guide proven cardano-testnet venue recipe + node1-replacement integration (#1-4 validated) |
| `88f5d6a5` | docs | c2-guide C2-local rehearsal (private chain, 2 Haskell nodes) is REQUIRED before preprod |
| `00144008` | docs | C2 preprod-tip guide (Conway-from-Mithril, never from genesis) + venue-general extractor |
| `0e6bff35` | chore | close PHASE4-N-AD — tip-successor durability proof |
| `67ce7ac6` | test | tip-successor durability regression (PHASE4-N-AD S1) |
| `68d83406` | docs | cluster + slice doc PHASE4-N-AD tip-successor durability proof |

No merge commits in the span. **19 commits, zero unclassified** — `feat(...)`/`fix(...)`/`test(...)` carry
explicit conventional-commits prefixes; the bulk are `docs:` (the C2-LOCAL guide/finding run + the N-AD/N-AE
cluster + slice docs); the two close-style commits (`0e6bff35` "close PHASE4-N-AD …" and `25ddeebd` the
N-AC refresh) classify `chore`/`docs` (their diff scope is exclusively `docs/` + the registry). The shape
is **N-AC refresh → N-AD (doc → S1 test → close) → C2-LOCAL guide/finding run → N-AE (cluster doc declaring
3 rules → AE.A → AE.C → AE.B → AE.E)**. Note the **commit order ≠ slice-letter order** for N-AE: the impl
landed A (`5f2afc2a`) → C (`5425b23c`) → B (`450c6992`) → E (`a76672b9`) — the AE.B "Option B" approach was
promoted to the CE-A5 closer (per the `238aff61` slice doc) and AE.E was the final cursor fix that made the
relay adopt. The N-AE cluster work landed 2026-06-06 → 2026-06-07 (the CE-A5 manifest on 2026-06-07).

> **Note (commit-attribution policy).** Per this repo's `CLAUDE.md` override (vibe-coded-node bounty
> trailer requirement), commits in this repo carry a `Co-Authored-By:` model-attribution trailer; that
> is an Ade-local override of the global no-AI-attribution rule and applies to **commit messages
> only**. It does not affect this doc's content.

## 2. New Modules

**None.** `git diff --diff-filter=A --name-only 25ddeebd..HEAD -- '*.rs'` shows **no new `.rs` *source*
file**, no new crate, no new `Cargo.toml`, no new workspace. The only added `.rs` file this span is a
**test** — `crates/ade_node/tests/phase4_n_ae_recover_serve_continuity_diag.rs` (the N-AE
recover→serve-continuity diagnostic/regression harness, ~844 lines at AE.A, extended through AE.B). The
other added files are **three CI gates** (§5) and the N-AD/N-AE **cluster + slice docs**, the **C2-LOCAL
guides + findings** (`docs/active/c2-preprod-tip-guide.md`, `docs/planning/c2-local-discovered-gaps.md`,
`docs/planning/phase4-n-ae-slice-a-invariants.md`), and the **CE-A5 evidence pair**
(`docs/evidence/phase4-n-ae-ce-a5-relay-adoption.{md,jsonl}`). The span is **modification only** in
production code — all touched production files are **existing** modules.

> **Cross-reference (CODEMAP/SEAMS) — N-AE surfaces, already in the refreshed docs.** This span adds
> **no new module**, but it adds **new surface to existing modules**: the GREEN forge-admissibility
> classifier `forge_followed_tip_admission` + `ForgeRefused`/`NodeForgeOutcome`/`FollowedPeerTipSignal`
> (RED `ade_node::node_sync`); the FindIntersect-only proof-gated projection in
> `ChainDbServedSource::intersect` (RED-shell `ade_runtime::network::served_chain_projection`); the
> additive BLUE `DecodedBlock.prev_hash` field (BLUE `ade_ledger::block_validity::header_input`); and the
> FindIntersect-cursor threading in the BLUE chain-sync server
> (`ade_network::chain_sync::server::producer_chain_sync_serve`). All four host modules were already in
> CODEMAP. CODEMAP and SEAMS are **already refreshed at HEAD `a76672b9`** in this close pass and fold in
> these surfaces and the four new rules (CODEMAP records the `chain_sync/server.rs` change as BLUE — under
> its `ade_network` BLUE entry — and SEAMS lists no new crate/module/TCB color). The registry already
> records the rules authoritatively (the `code_locus` fields name every site).

## 3. Modules Modified

Five modules changed this span (two BLUE — both **additive, +0 canonical type** — and three RED/RED-shell):

| Module | Color / scope | Key changes |
|--------|---------------|-------------|
| `ade_node::node_sync` (`crates/ade_node/src/node_sync.rs`) | RED, +869 / −105 | **N-AD S1 (`67ce7ac6`):** tip-successor durability regression (WAL replay) — proves a forged tip-successor survives a crash and replays byte-identically (`DC-WAL-04` + `T-REC-05` strengthened). **AE.A (`5f2afc2a`):** the closed **GREEN** classifier `forge_followed_tip_admission(durable_servable_tip, followed_peer_tip)` (returns `CaughtUp` iff both tips present + hash + `block_no` equal, else `NotCaughtUp{NoFollowedPeerTip | NoDurableServableTip | TipMismatch}`); the typed outcomes `ForgeFollowedTipAdmission` / `NotCaughtUpReason` / `ForgeRefused` / `NodeForgeOutcome` / `FollowedPeerTipSignal`. **AE.C (`5425b23c`):** the recover→follow `prior_fp` seeding path (= `fingerprint(&state.ledger).combined`). **AE.B (`450c6992`):** the live-style follow→serve store-and-intersect wiring (the forge parent is stored as a servable `StoredBlock` and made FindIntersect-able via the projection). |
| `ade_node::node_lifecycle` (`crates/ade_node/src/node_lifecycle.rs`) | RED, +242 / (net) | **AE.A (`5f2afc2a`):** the `run_relay_loop_with_sched` `ForgeTick` arm — the `recovered.tip` forge-base fallback (`None => act.recovered.tip.clone()`) is **removed**; the admission classifier is called **before** the single fenced `forge_one_from_recovered`; a `NotCaughtUp` records `ForgeRefused::NotCaughtUp` into `ForgeActivation.last_forge_refused`; the `CaughtUp` arm forges on `selected_tip = ChainDb::tip()` (= the followed peer tip), with the successor `prev_hash = PrevHash::Block(selected_tip.hash)` and `block_no = last + 1` (`DC-CONS-24`). **AE.C (`5425b23c`):** the `prior_fp` seed is set to the recovered ledger-tip post-fp at **both** lifecycle sites (forge-off + forge-on) — was the all-zero `Hash32`. |
| `ade_runtime::network::served_chain_projection` (`crates/ade_runtime/src/network/served_chain_projection.rs`) | RED shell, +49 / −2 | **AE.B (`450c6992`):** `ChainDbServedSource::intersect` projects the `prev_hash` of the **earliest servable `StoredBlock`** (the forge parent, via the new private helper `earliest_servable_block_prev_hash`) as a **FindIntersect-only**, **proof-gated** intersect point **iff** a real servable successor exists; it **never serves bytes** for that point (`get_block_by_hash` / `serve_range` stay empty → BlockFetch refuses structurally; **no synthetic `StoredBlock`**). A recover-only store (no successor) → **no projection** (fail-closed). |
| `ade_ledger::block_validity::header_input` (`crates/ade_ledger/src/block_validity/header_input.rs`) | **BLUE, additive** +8 / −1 | **AE.B (`450c6992`):** `DecodedBlock.prev_hash: PrevHash` is **exposed** — an **already-parsed** field (decoded for `check_header_position`), surfaced so the serve projection can prove a recovered/forged parent is the parent of a real servable successor (`DC-NODE-14`). **Additive — a new public *field* on an existing struct; no new type, no new parse, no behavior change** (`block_hash` / `computed_body_hash` / the position rule are byte-identical). BLUE canonical-type count unchanged (**458 → 458**). |
| `ade_network::chain_sync::server` (`crates/ade_network/src/chain_sync/server.rs`) | **BLUE, additive** +80 / −4 | **AE.E (`a76672b9`) — CE-A5 closer.** `producer_chain_sync_serve`'s `FindIntersect` handler now sets `state.last_announced` from the matched intersect point (`Point::Block{slot,hash} → Some((slot,hash))`; `Point::Origin → None`) **before** replying `IntersectFound`, so the next `RequestNext` serves `next_after(point)` (the successor), not `next_after(None)` (block 0). **Additive cursor-threading logic + one regression test** (`producer_chain_sync_serve_find_intersect_sets_cursor_then_rolls_forward_past_it`); **no new type, no wire-grammar change** (`ProducerChainSyncServerState` is unchanged — this is an existing-field write). BLUE canonical-type count unchanged (**458 → 458**). |

> **BLUE change is additive only (load-bearing).** Two BLUE `core_paths` files are touched this span, but
> **both changes are additive and add zero canonical type**: (1) `ade_ledger::block_validity::header_input`
> exposes an **already-parsed** field (`DecodedBlock.prev_hash`) — a new public *field*, not a new struct/
> enum, with no new parse path; (2) `ade_network::chain_sync::server` adds **cursor-threading logic** to an
> existing handler (`producer_chain_sync_serve`) plus a regression test — no new type, **no widening of the
> closed wire grammar** (the FindIntersect decode shape is unchanged; this fixes only the server's *read
> cursor* after a successful intersect). The BLUE canonical-type count is **458 → 458** (verified: `git
> diff 25ddeebd..HEAD` over the BLUE trees adds **zero** `^+(pub )?(struct\|enum)` lines). The header /
> body authorities, the KES verifier, forge eligibility, and the closed wire grammar are otherwise
> unchanged.

## 4. Feature Flags

**No project feature-flag deltas.** Ade declares no `[features]` table in any workspace `Cargo.toml`,
and **no `Cargo.toml` changed in this window** (`git diff --name-only 25ddeebd..HEAD -- '**/Cargo.toml'
'Cargo.toml'` is empty). No `#[cfg(feature = …)]` gate was introduced. The N-AE behavior is governed by
**fixed, typed** constructs (the closed `forge_followed_tip_admission` classifier + `ForgeRefused`
variants; the FindIntersect-only proof-gate in `ChainDbServedSource::intersect`; the chain-sync server
cursor) — not feature flags, CLI flags, env vars, or config knobs. The one venue-general behavior added —
the consensus-inputs extractor reading `epochLength`/`activeSlotsCoeff` from `shelley-genesis` — is in a
**non-gate operator script** (`ci/build_consensus_inputs_bundle.sh`), overridable via the
`ADE_LIVE_SHELLEY_GENESIS` env var, and is **not** a code feature flag.

## 5. CI Checks (138 → 141; +3 new gates, 0 gates modified, 0 gates removed; +1 non-gate script modified)

Three new gates this span; no gate modified, no gate removed. `git diff --diff-filter=A 25ddeebd..HEAD
-- ci/` lists exactly the three gates below; `--diff-filter=D` over `ci/` is **empty**; `--diff-filter=M`
over `ci/` lists exactly **one** file — and it is the **non-gate** operator script
`ci/build_consensus_inputs_bundle.sh` (so it does not move the `ci_check_*.sh` gate count).

### PHASE4-N-AE gates (`5f2afc2a`, `5425b23c`, `450c6992`)

| Check | Status | Origin / change | What it checks |
|-------|--------|-----------------|----------------|
| `ci_check_forge_followed_tip_admission.sh` | **New** | PHASE4-N-AE.A (`5f2afc2a`); `DC-NODE-15` + `DC-CONS-24` (+ `DC-NODE-14` partial) | The `--mode node` forge is admissible **only** when `durable_servable_tip == followed_peer_tip` (hash AND `block_no`): **(a)** the recovered-tip forge-base fallback is removed (forge base is `ChainDb::tip()`); **(b)** the classifier compares hash AND `block_no`; **(c)** a `NotCaughtUp` records a typed `ForgeRefused::NotCaughtUp` (no forge, tip unchanged); **(d)** static-grep — the followed-peer-tip signal never reaches `select_best_chain` / `chain_selector` / `fork_choice`. |
| `ci_check_recover_follow_wal_lineage.sh` | **New** | PHASE4-N-AE.C (`5425b23c`); `DC-WAL-02` + `T-REC-05` | The live recover→follow path seeds `ForwardSyncState.prior_fp = fingerprint(&state.ledger).combined` (the recovered ledger-tip post-fp), **not** the all-zero `Hash32`, at **both** lifecycle sites — so the first followed `AdmitBlock`'s `prior_fp` chains from the recovered ledger fp and warm-start does not `ChainBreak@1`. Fences the live **seed** without loosening `WalStore::verify_chain`. |
| `ci_check_recovered_anchor_intersectable.sh` | **New** | PHASE4-N-AE.B (`450c6992`); `DC-NODE-14` | `ChainDbServedSource::intersect` projects the earliest servable `StoredBlock`'s `prev_hash` as a **FindIntersect-only**, **proof-gated** point **iff** a real servable successor exists; it **never serves bytes** for it (no synthetic `StoredBlock`; `get_block_by_hash` / `serve_range` stay empty). A recover-only store yields **no projection** (fail-closed). |

### Non-gate operator script (`00144008`)

| Script | Status | Change | What it does |
|--------|--------|--------|--------------|
| `ci/build_consensus_inputs_bundle.sh` | **Modified** (non-gate) | C2 preprod-tip guide (`00144008`) | The venue-general consensus-inputs extractor now reads `epochLength` + `activeSlotsCoeff` from the venue's `shelley-genesis` (default the local preprod genesis; override via `ADE_LIVE_SHELLEY_GENESIS`) instead of hardcoding preprod's `epochLength 432000` / `ASC 1/20` — so the **same** extractor is correct for preprod **and** a short-epoch C2-LOCAL rehearsal venue (e.g. `--epoch-length 2000`). Not a `ci_check_*.sh` gate; does not move the gate count. |

> **Cross-reference (TRACEABILITY) — new bindings + one regression-test-only rule.** TRACEABILITY is
> **already refreshed at HEAD `a76672b9`** in this close pass; the new rule↔gate bindings
> (`DC-NODE-15` / `DC-CONS-24` / `DC-NODE-14` ↔ `ci_check_forge_followed_tip_admission.sh` +
> `ci_check_recovered_anchor_intersectable.sh`; `DC-WAL-02` / `T-REC-05` ↔
> `ci_check_recover_follow_wal_lineage.sh`) are recorded authoritatively in the **registry** at HEAD, so
> the rule↔gate links are **authoritative in the registry** regardless of TRACEABILITY's per-row trace
> depth. **`DC-PROTO-10` has an empty `ci_script`** — it is enforced by a **regression test**
> (`producer_chain_sync_serve_find_intersect_sets_cursor_then_rolls_forward_past_it`), not a dedicated
> `ci_check_*.sh` gate; this is a **deliberate** test-enforced rule (a server-cursor behavioral invariant
> pinned by a round-trip test), **not** an orphan gate. **No rule↔gate binding was removed.** |

## 6. Canonical Type Registry Delta

**n/a — no separate canonical-type registry is configured** (`canonical_type_registry: null`);
canonical-type rules live inline in the invariant registry under family **T**. **No canonical type was
added or removed in this window** — the BLUE count is unchanged (**458 → 458**). The two BLUE source
changes are **additive**: a new public *field* (`DecodedBlock.prev_hash`, already parsed) and new server
*logic* + a test — neither adds a `struct`/`enum`. No `Cargo.toml` changed.

## 7. Normative / Invariant Rule Delta (336 → 340; +4 enforced rules, 9 strengthenings, zero removals)

**Four rule IDs were added; zero removed** (336 → 340; `diff` of the sorted `id =` lists shows exactly the
four additions `DC-CONS-24`, `DC-NODE-14`, `DC-NODE-15`, `DC-PROTO-10` and no removal). The status tally
moves **204 → 208 enforced** (20 partial / 112 declared unchanged) — all four new rules are `enforced` at
HEAD.

*(The configured `normative_docs` — the CE-79 tier-gate statement + addendum, the three contract docs,
the CE-73 reclassification, and `CLAUDE.md` — were **not** changed this span: `git diff --name-only
25ddeebd..HEAD` over those paths is empty. The rule-count delta is entirely the invariant-registry change
below.)*

**New rules (`+4`, all enforced):**

| Rule | Family / Tier | Statement (summary) |
|------|---------------|---------------------|
| `DC-NODE-15` | DC / `derived` (enforced; `introduced_in = "PHASE4-N-AE"`) | **Forge admissibility requires the durable servable tip to equal the followed peer tip.** A `--mode node` forge is admissible **only** when `durable_servable_tip == followed_peer_tip` (hash AND `block_no`); otherwise it fails closed with a typed `ForgeRefused::NotCaughtUp{local_servable_tip, followed_peer_tip, reason}` — no forge, no state transition, tip unchanged. The recovered anchor is **never** a forge base. The followed-peer-tip signal is a forge-**admissibility** input only — it may *prevent* a forge but never reaches `select_best_chain` / `chain_selector` / `fork_choice`. `ci_script = ci/ci_check_forge_followed_tip_admission.sh`. |
| `DC-CONS-24` | DC / `derived` (enforced; `introduced_in = "PHASE4-N-AE"`) | **Forged parent hash byte-equals the peer-visible selected tip.** The forged successor's `prev_hash` byte-equals the followed peer tip hash **and** its `block_no == followed_tip.block_no + 1`. Parent identity is the **canonical hash**, never inferred from block number alone. `ci_script = ci/ci_check_forge_followed_tip_admission.sh`. |
| `DC-NODE-14` | DC / `derived` (partial at AE.A → enforced at AE.B; `introduced_in = "PHASE4-N-AE"`) | **Every claimed forge parent must be servable or peer-intersectable in the durable served lineage.** A `--mode node` forge may build only on a parent a Haskell peer can `FindIntersect`: the followed peer tip (a durably-stored `StoredBlock`, AE.A) or a recovered anchor made intersectable (AE.B). The served chain exposes that parent as a FindIntersect point from which the peer rolls forward onto the forged successor; the recovered snapshot anchor is **never** served as a chain head a peer cannot intersect (Option B: FindIntersect-**only**, proof-gated, never serves bytes for the projected point). `ci_script = ci/ci_check_forge_followed_tip_admission.sh ci/ci_check_recovered_anchor_intersectable.sh`. |
| `DC-PROTO-10` | DC / `derived` (enforced; `introduced_in = "PHASE4-N-AE"`) | **Chain-sync server FindIntersect cursor.** After the producer chain-sync server answers `IntersectFound(point)`, its read cursor (`last_announced`) **IS** that point — the next `RequestNext` serves `next_after(point)` (the successor the client rolls forward onto), never `next_after(None)` (the chain start). A non-`Origin` intersect that left the cursor unset would serve block 0 to a client whose read pointer is its own tip, which the client rejects as `UnexpectedBlockNo(tip_block_no + 1)(0)`. An `Origin` intersect keeps the cursor `None` (serve from the chain start, correct). **`ci_script = ""` — enforced by regression test** `producer_chain_sync_serve_find_intersect_sets_cursor_then_rolls_forward_past_it`. **This is the CE-A5 closer.** |

**Strengthenings (`strengthened_in +=`) — 9 (8 distinct rules), no rule weakened:**

| Rule | By | Strengthening |
|------|----|----|
| `DC-WAL-04` | PHASE4-N-AD | Forged-tip WAL no-orphan / prior-fp clause — the **tip-successor** durability regression proves a forged tip-successor is WAL-durable and replays byte-identically (the N-AD S1 test). |
| `T-REC-05` | PHASE4-N-AD, PHASE4-N-AE | Forged-tip crash-recovery replay-equivalence — strengthened by N-AD (tip-successor regression) **and** N-AE.C (the live recover→follow `prior_fp` seed now chains warm-start replay from the recovered ledger-tip fp instead of the all-zero seed). |
| `DC-WAL-02` | PHASE4-N-AE | WAL chain-link first-entry clause now enforced on the **live** recover→follow path: the first followed `AdmitBlock`'s `prior_fp` is the recovered ledger-tip post-fp (was zero), so the live store passes `verify_chain` warm-start (AE.C) — **without** loosening `verify_chain`. |
| `CN-CONS-07` | PHASE4-N-AE | Served-chain projection / forge-parent intersectability — the served view now also exposes a recovered/forged parent as a FindIntersect-only proof-gated point (AE.B), extending the durable-projection contract. |
| `DC-CONS-23` | PHASE4-N-AE | Own-forged durable-admit / parent-linkage — extended by the forge-on-followed-tip parent identity + the recovered-anchor intersectability proof (AE.A/AE.B). |
| `CN-CONS-06` | PHASE4-N-AE | Block-production / live forge → serve → adopt — strengthened by the AE.E chain-sync server FindIntersect-cursor fix, which made the real relay roll forward onto the forged successor (the CE-A5 manifest). |
| `DC-EPOCH-03` | PHASE4-N-AE | Single-epoch forge containment on the `--mode node` spine — hardened alongside the AE.A forge-admissibility gate (the forge base is the durable servable tip within the recovered seed epoch). |
| `DC-NODE-05` | PHASE4-N-AE | `--mode node` durable-tip-only forge rule — strengthened by the AE.A admissibility gate (the durable-tip forge now also gates on followed-tip caught-up; a non-caught-up tick fails closed with a typed `ForgeRefused`, no forge). |

> **CE-A5 manifest — load-bearing evidence (not a registry status flip).** The N-AE cluster achieved the
> **CE-A5 manifest**: a real `cardano-node 11.0.1` relay (`c2ae18`, non-producing node2) `AddedToCurrentChain`
> an **Ade-forged block** (block 17 @ slot 421, hash `db3b5675…`, `issuerHash a1ed4e04… == blake2b-224(pool1`
> cold VK`)`, `forging = 0` on the relay; Ade forge `succeeded = 1`). The relay's own log line
> (`ChainDB.AddBlockEvent.AddedToCurrentChain`, `newtip db3b5675…@421`) is committed at
> `docs/evidence/phase4-n-ae-ce-a5-relay-adoption.jsonl`; the narrative at `…-relay-adoption.md`. This is
> recorded as **`enforced`-backing evidence** on `DC-NODE-14` / `DC-PROTO-10` (and the `CN-CONS-06`
> strengthening) — **not** a `RO-LIVE-01` registry status flip. The full path proven end-to-end live is
> **recover (behind the relay tip) → follow → admissibility-gate → forge T+1 on the followed tip → serve →
> relay adopts**.

**No rule was removed (expected: 0).** The registry delta is **four new enforced rules + nine
`strengthened_in` appends** — purely additive / strengthening, consistent with append-only registry
discipline.

## Working tree at HEAD `a76672b9`

Clean of tracked changes from this span — the N-AD cluster + close, the C2-LOCAL guide/finding run, and
the N-AE slices (through the CE-A5 closer) are all committed. `git status --short` shows only an untracked
`.mithril-scratch/` (operator scratch, ignored). **This regen runs *after* all 19 span commits** (the
CE-A5 closer `a76672b9` is HEAD for this window); CODEMAP/SEAMS/TRACEABILITY are **already refreshed at
HEAD `a76672b9`** in this close pass, so the remaining close-pass actions are this HEAD_DELTAS, the
PHASE4-N-AE cluster-doc archive, and the baseline bump (`1d54abb4 → a76672b9`).

> **Cluster-archive note.** The **PHASE4-N-AD** docs are archived under
> `docs/clusters/completed/PHASE4-N-AD/` (the `0e6bff35` close). The **PHASE4-N-AE** cluster + slice docs
> are at `docs/clusters/PHASE4-N-AE/` (active path) at HEAD `a76672b9` — the CE-A5 closer `a76672b9` is a
> `fix(...)` commit, not a formal `chore: close` archive commit; the cluster archive + the `head_deltas_baseline`
> bump are part of this close pass.

## Honest residual (window scope)

PHASE4-N-AE **proved the recover→follow→forge→serve→ADOPT path end-to-end live** — a real `cardano-node
11.0.1` relay adopted an Ade-forged block (CE-A5 manifest). The honest boundary:

- **CE-A5 is the C2-LOCAL #8–#9 manifest, NOT a registry `RO-LIVE` flip.** The manifest is recorded as
  `enforced`-backing live evidence on `DC-NODE-14` / `DC-PROTO-10` (and the `CN-CONS-06` strengthening),
  **not** a `RO-LIVE-01` status flip. `RO-LIVE-01` remains as scoped. The CE-A5 venue is a **hermetic
  C2-LOCAL** `cardano-testnet` (`--testnet-magic 42`, Conway, `--epoch-length 2000`) — a private 2-Haskell-
  node rehearsal, the **required** pre-preprod venue; it is **not** a preprod/mainnet operator-pass.
- **BLUE-additive, +0 canonical type.** The span touches two BLUE files but **adds no new canonical type**:
  an already-parsed public *field* (`DecodedBlock.prev_hash`) and additive chain-sync server cursor *logic*
  + a test. The closed wire grammar, the header/body authorities, the KES verifier, and forge eligibility
  are unchanged. BLUE canonical-type count **458 → 458**.
- **The AE.B projection is FindIntersect-ONLY and proof-gated.** `ChainDbServedSource::intersect` exposes a
  recovered/forged parent as a FindIntersect point **only when** a real servable successor exists, and it
  **never serves bytes** for that point (no synthetic `StoredBlock`; BlockFetch refuses structurally). A
  recover-only store yields **no projection** (fail-closed). The recovered snapshot anchor is **never**
  served as an adoptable chain head.
- **The followed-peer-tip signal is admissibility-only.** It may *prevent* a forge (`ForgeRefused::NotCaughtUp`)
  but it **never** reaches `select_best_chain` / `chain_selector` / `fork_choice` (gate (d) static-grep
  enforced) — it cannot select, replace, reorder, or prefer chains.
- **PHASE4-N-AD is a durability *proof* (test-only).** N-AD added **no** production code — it is the
  tip-successor WAL-replay regression (one RED test, +214 lines), strengthening `DC-WAL-04` + `T-REC-05`. It
  proves the forged tip-successor is WAL-durable + replay-equivalent; it changes no authoritative behavior.
- **C2-LOCAL guide/finding run is docs + one operator script.** The `00144008..bc05cb81` run is docs-only
  except for the venue-general `ci/build_consensus_inputs_bundle.sh` change (read `epochLength`/`ASC` from
  `shelley-genesis`). It records the C2 venue recipe (Conway-from-Mithril for preprod; private 2-node
  rehearsal first) and isolates the forge-vs-follow race (Gap 2) into 2a (forge-on-followed-tip, AE.A) +
  2b (serve-continuity, AE.B/AE.E) — the scoping that drove the N-AE slices.
- **All four grounding docs current at this close.** CODEMAP/SEAMS/TRACEABILITY are already refreshed at
  HEAD `a76672b9` (carrying the N-AD strengthenings, the four N-AE rules, the new surfaces, and the nine
  strengthenings); this HEAD_DELTAS completes the set. The registry records the rules + gate bindings
  authoritatively at HEAD (340 rules).

---

## Historical — PHASE4-N-AC close + cluster window (`c6e7fafe → 1d54abb4`)

> The section below is the **previous** HEAD_DELTAS lead, preserved in condensed form. It was a
> **grounding-doc refresh + the PHASE4-N-AC cluster** (KES signing evolves the operator KES key to the
> current period before signing), narrating the `c6e7fafe → 1d54abb4` span. Counts here are the figures
> **at `1d54abb4`** (336 rules, 138 CI gates, 458 canonical types); the current window measures **forward**
> from `25ddeebd` (the N-AC grounding refresh, one commit after `1d54abb4`). The full §§0–7 narrative is
> recoverable from this doc's git history at `1d54abb4` / `25ddeebd`.

> Baseline: `c6e7fafe` (Close PHASE4-N-AB — outbound mux segmentation (CN-SESS-05), 2026-06-06 03:48)
> HEAD: `1d54abb4` (Close PHASE4-N-AC — KES signing evolves key to current period (DC-CRYPTO-10), 2026-06-06 11:08)
> Span: **a grounding-doc refresh + the PHASE4-N-AC cluster** — 5 commits, 12 files, +1029 / −340.

PHASE4-N-AC was a **RED-only live-readiness fix surfaced by the item-4 C1 re-run**: the forge's only real
KES sign required `kes.current_period() == kes_period`, and nothing evolved the minted-at-period-0 operator
key forward, so once the chain aged past one KES period the forge returned `KesPeriodNotCurrent` on every
leader slot. N-AC closed it in one slice:

- **S1 — evolve KES key to current period before signing (`7d4a4a72`; `DC-CRYPTO-10 → enforced`).** A new
  **RED** producer-shell method `ProducerShell::kes_sign_header_advancing(period, pre_image)` =
  `kes_advance_to(period)` then `kes_sign_header(period, pre_image)` — it evolves the operator KES key
  forward via the **existing deterministic `Sum6KES` update** (idempotent at the current period), then
  signs; **fails closed** `EvolutionBackwards` (before key start) / `EvolutionExhausted` (beyond
  `SUM6_MAX_PERIOD = 63`). The forge's single real KES sign is rewired to it (period passed verbatim).
  Signing stays RED. New gate `ci_check_kes_evolution_before_sign.sh`.

**N-AC headline (at `1d54abb4`):** Registry **335 → 336** (+1 enforced `DC-CRYPTO-10`; +1 strengthening
`CN-KES-HEADER-01`; 0 removed). CI gates **137 → 138** (+1 `ci_check_kes_evolution_before_sign.sh`).
**RED-only — BLUE canonical types 458 → 458.** The item-4 C1 re-run proved it live (Ade forged 3 period-1
blocks; the real cardano-node downloaded the period-1 header with no KES rejection). **No `RO-LIVE` flip.**
A genesis-window finding was recorded honestly (`slotsPerKESPeriod = 129600 == 3k/f`, so a from-genesis
rehearsal cannot show forge-at-period-1 **and** follower-adopt simultaneously — the period-1 follower
rejection is `CandidateTooSparse`, KES-independent).

---

## Historical — PHASE4-N-AB close + cluster window (`b0365df0 → c6e7fafe`)

> Preserved in condensed form. A **grounding-doc refresh + the PHASE4-N-AB cluster**, narrating the
> `b0365df0 → c6e7fafe` span. Counts here are the figures **at `c6e7fafe`** (335 rules, 137 CI gates, 458
> canonical types). The full §§0–7 narrative is recoverable from this doc's git history at `c6e7fafe`.

> Baseline: `b0365df0` (Close PHASE4-N-AA — bounded peer-driven serve range (DC-SERVEMEM-01), 2026-06-06 01:43)
> HEAD: `c6e7fafe` (Close PHASE4-N-AB — outbound mux segmentation (CN-SESS-05), 2026-06-06 03:48)
> Span: **a grounding-doc refresh + the PHASE4-N-AB cluster** — 5 commits, 10 files, +1130 / −406.

PHASE4-N-AB was **pre-RO-LIVE hardening item 2** and closed a **receive/send asymmetry**: Ade could
*receive* a block fragmented across multiple mux frames (CN-SESS-04 inbound reassembly) but could **not
transmit one** (`OutboundPayloadTooLarge` above `MAX_PAYLOAD = 65535`). N-AB closed that in one slice:

- **S1 — outbound mux segmentation (`02e6e557`; `CN-SESS-05 → enforced`).** The **GREEN** session
  reducer's `handle_outbound` now **segments** a payload in `MAX_PAYLOAD < len <= MAX_OUTBOUND_PAYLOAD_BYTES`
  into ordered `<= MAX_PAYLOAD` mux frames (each via the single `encode_inner_frame` authority) and **fails
  closed above** the new fixed `MAX_OUTBOUND_PAYLOAD_BYTES = 16 MiB`. New gate `ci_check_outbound_segmentation.sh`.

**N-AB headline (at `c6e7fafe`):** Registry **334 → 335** (+1 enforced `CN-SESS-05`; +2 strengthenings
`CN-SESS-04` + `DC-SERVEMEM-01`; 0 removed). CI gates **136 → 137** (+1 `ci_check_outbound_segmentation.sh`).
**GREEN-only — BLUE canonical types 458 → 458.** Outbound inverse of CN-SESS-04 inbound reassembly. **No
`RO-LIVE` flip.**

---

## Historical — PHASE4-N-AA close + cluster window (`999199f8 → b0365df0`)

> Preserved in condensed form. A **focused grounding refresh + the PHASE4-N-AA cluster**, narrating the
> `999199f8 → b0365df0` span. Counts here are the figures **at `b0365df0`** (334 rules, 136 CI gates, 458
> canonical types). The full §§0–7 narrative is recoverable from this doc's git history at `b0365df0`.

> Baseline: `999199f8` (repair 10 pre-existing gate-vs-code drifts (gate hygiene), 2026-06-05 19:28)
> HEAD: `b0365df0` (Close PHASE4-N-AA — bounded peer-driven serve range (DC-SERVEMEM-01), 2026-06-06 01:43)
> Span: **a focused grounding refresh + the PHASE4-N-AA cluster** — 8 commits, 15 files, +1254 / −492.

PHASE4-N-AA was **pre-RO-LIVE hardening item 1** and closed the **MEDIUM** the PHASE4-N-U cross-slice
review left open: *the `--mode node` serve path could be driven by a peer into unbounded memory + O(N²)
CPU.* N-AA closed it across two slices + an in-cluster security fix:

- **S1 — bounded hash-free ChainDb read primitives (`6b8f1779`; CE-1).** Two new bounded, slot-ordered,
  hash-free `ChainDb` primitives `range_bytes_capped` / `last_block_bytes`; new RED type `CappedSlotRange`.
- **S2 — serve projection cap + fail-closed (`3d853ec0`; `DC-SERVEMEM-01 → enforced`).** `ChainDbServedSource`
  switched onto the bounded primitives behind `MAX_SERVE_RANGE_BLOCKS = 256`; new RED enum `ServeRangeOutcome`.
  New gate `ci_check_serve_range_bounded.sh`.
- **In-cluster security-review MEDIUM (`5c9f6cf6`).** An inverted-range (`from > to`) panic fixed in-cluster.

**N-AA headline (at `b0365df0`):** Registry **333 → 334** (+1 enforced `DC-SERVEMEM-01`; +2 strengthenings
`DC-NODE-13` + `DC-LIVEMEM-01`; 0 removed). CI gates **135 → 136** (+1 `ci_check_serve_range_bounded.sh`).
**RED-only — BLUE canonical types 458 → 458.** Serve-side analog of `DC-LIVEMEM-01`. **No `RO-LIVE` flip.**

---

## Historical — earlier windows (`4e358e92 → 999199f8` and before)

> Preserved as pointers. The **PHASE4-N-U cluster CLOSE + gate-hygiene tail** (`4e358e92 → 999199f8`, 333
> rules / 135 CI gates at `999199f8` — 11 gates repaired in place, 0 added/removed, 0 invariants weakened);
> the **PHASE4-N-U cluster** (`65954fa3 → 4e358e92`, forged-block durability — `DC-NODE-12`, `DC-CONS-23`,
> `DC-WAL-04`, `T-REC-05`, `DC-NODE-13`; one new RED module `served_chain_projection`; 328 → 333 rules);
> and the **G-K…G-R + C1 multi-cluster catch-up** (`550eec3a → 65954fa3`, eight clusters G-K through G-R
> toward a live genesis-successor follower — 319 → 328 rules, 126 → 134 CI gates, the one BLUE canonical
> type `ArrayHead` 457 → 458). The full §§0–7 narrative for each is recoverable from this doc's git history
> at `999199f8` / `4e358e92` / `65954fa3`.

> *(The G-E…G-I and earlier leads were each closed with their own grounding-doc refresh and are recoverable
> from this doc's git history.)*

---

## Generation notes

### Regen `25ddeebd → a76672b9` (PHASE4-N-AD durability proof + C2-LOCAL guide/finding run + PHASE4-N-AE CE-A5 cluster — current lead)

- **Baseline valid; multi-part lead (N-AC refresh tail → N-AD test-only → C2-LOCAL docs → N-AE CE-A5).**
  Run against `25ddeebd` (the PHASE4-N-AC grounding-refresh commit that wrote the previous lead), which
  `git rev-parse` resolves and `git merge-base 25ddeebd HEAD` confirms is a strict ancestor of HEAD
  `a76672b9` (`25ddeebd` carries no tag). The start-of-regen config baseline was `1d54abb4` (the N-AC close
  *impl*); `25ddeebd` is the next commit (the N-AC refresh), so the window measures from the doc-refresh
  commit and the `1d54abb4..25ddeebd` step is folded into the span-opening tail. The closer bumps
  `head_deltas_baseline` `1d54abb4 → a76672b9` after this regen.
- **Counts are mechanical (git/grep/ls):** commit log + `--shortstat` over `25ddeebd..HEAD` (**19** commits,
  no merges / **24** files / **+3635 / −129**); CI gate count via
  `git ls-tree -r --name-only <ref> ci/ | grep -c 'ci_check_.*\.sh$'` at each ref (**138 → 141**;
  `--diff-filter=A` over `ci/` = the three new gates; `--diff-filter=D` over `ci/` **empty**;
  `--diff-filter=M` over `ci/` = exactly `build_consensus_inputs_bundle.sh`, a **non-gate** operator script);
  registry rule count via `grep -cE '^\[\[rules\]\]'` at each ref (**336 → 340**; `diff` of sorted `id =`
  lists shows the four additions `DC-CONS-24` / `DC-NODE-14` / `DC-NODE-15` / `DC-PROTO-10`, zero removals);
  registry status via `grep -E '^status = ' | sort | uniq -c` (**204 → 208 enforced**, 20 partial / 112
  declared unchanged); strengthenings via the registry `strengthened_in` scan (**9** appends across **8**
  distinct rules: N-AD on `DC-WAL-04` + `T-REC-05`; N-AE on `DC-EPOCH-03`, `CN-CONS-06`, `CN-CONS-07`,
  `DC-WAL-02`, `DC-NODE-05`, `T-REC-05`, `DC-CONS-23`); BLUE canonical types via a direct
  `git grep -hE "^(pub )?(struct|enum) "` count over the BLUE trees at each ref (**458 → 458**).
- **BLUE-additive span — two BLUE files, +0 canonical type, no Cargo.toml change.** `git diff --name-status
  25ddeebd..HEAD` shows two BLUE `core_paths` files touched —
  `crates/ade_ledger/src/block_validity/header_input.rs` (additive `DecodedBlock.prev_hash` field, already
  parsed) and `crates/ade_network/src/chain_sync/server.rs` (additive FindIntersect-cursor logic + a
  regression test) — and `git diff 25ddeebd..HEAD` over the BLUE trees adds **zero** `^+(pub )?(struct\|enum)`
  lines. The other source changes are RED (`ade_node::{node_sync, node_lifecycle}`) or RED-shell
  (`ade_runtime::network::served_chain_projection`). No new `.rs` *source* file (the one added `.rs` is the
  N-AE diagnostic *test*). `git diff --name-only … '**/Cargo.toml' 'Cargo.toml'` is empty (no feature-flag
  delta). **Note (classification):** `ade_network::chain_sync::server` is **BLUE** — it sits under the BLUE
  `core_paths` path `chain_sync/` and opens with the BLUE banner; CODEMAP documents the AE.E change in its
  BLUE `ade_network` entry, not as GREEN.
- **Registry delta is +4 enforced rules + 9 strengthenings, NOT a removal.** The four new rules are
  declared at the N-AE cluster doc (`48a8009c`, `DC-NODE-14` / `DC-NODE-15` / `DC-CONS-24`) + the AE.E impl
  (`a76672b9`, `DC-PROTO-10`), enforced at their slice impls (`DC-NODE-14` lands `partial` at AE.A
  `5f2afc2a`, `enforced` at AE.B `450c6992`). `DC-PROTO-10` carries an **empty `ci_script`** — it is
  **regression-test enforced** (`producer_chain_sync_serve_find_intersect_sets_cursor_then_rolls_forward_past_it`),
  a deliberate test-enforced server-cursor invariant. The sorted-id `diff` confirms zero removals.
- **CE-A5 manifest is `enforced`-backing evidence, NOT a `RO-LIVE` flip.** The real-relay adoption
  (`docs/evidence/phase4-n-ae-ce-a5-relay-adoption.{md,jsonl}`; relay `AddedToCurrentChain` Ade's forged
  block 17 @ slot 421) backs `DC-NODE-14` / `DC-PROTO-10` (and the `CN-CONS-06` strengthening); no
  `RO-LIVE-01` registry status changed this span.
- **Normative docs unchanged this span.** `git diff --name-only 25ddeebd..HEAD` over the configured
  `normative_docs` (CE-79 statement + addendum, the three contract docs, CE-73 reclassification, `CLAUDE.md`)
  is empty — the §7 delta is entirely the invariant-registry change.
- **Commit order ≠ slice-letter order for N-AE.** The impl landed A (`5f2afc2a`) → C (`5425b23c`) → B
  (`450c6992`) → E (`a76672b9`); the AE.B Option-B approach was promoted to the CE-A5 closer (per the
  `238aff61` slice doc) and AE.E was the final cursor fix. The §1 commit log is verbatim from
  `git log --oneline --no-merges` (newest first); the per-slice synthesis is in §0/§3.
- **Doc-refresh — all four grounding docs current at HEAD `a76672b9`.** The span-opening `25ddeebd`
  refreshed the four docs for the **N-AC** close. In this close pass CODEMAP/SEAMS/TRACEABILITY are
  **already regenerated to HEAD `a76672b9`** (verified on disk: CODEMAP header `a76672b9` / 458 / 141 / 340
  with full N-AE coverage; TRACEABILITY + SEAMS re-pinned to `a76672b9` / registry 340) — this HEAD_DELTAS
  is the last of the four brought current. The registry records the rules + gate bindings authoritatively
  at HEAD (340 rules).
- **Working tree clean.** This regen runs *after* all 19 span commits (the CE-A5 closer `a76672b9` is HEAD
  for this window); `git status --short` shows only an untracked `.mithril-scratch/` (operator scratch,
  ignored). The remaining close-pass actions are this HEAD_DELTAS, the PHASE4-N-AE cluster-doc archive, and
  the baseline bump `1d54abb4 → a76672b9`.
