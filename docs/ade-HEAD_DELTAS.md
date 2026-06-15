# Ade — HEAD Deltas (Changes Since Baseline)

> **Status:** Living architectural document. Regenerated; not hand-edited.
>
> Regenerate with `/head-deltas <baseline>` after every cluster close. Baseline is recorded in `.idd-config.json` `head_deltas_baseline`.

> Baseline: `862cd2cb` (PHASE4-N-AO close — flip `CN-CONS-03` enforced on the natural CE-AO-6 transcript, 2026-06-13 17:05)
> HEAD: `233644f7` (MEM-OPT-OPS cluster-review fixes — normalize all observational `*_kib` + correct the dup-key equivalence claim, 2026-06-15 23:12)
> Span: **`862cd2cb → 233644f7`** — three sequential bands: **(1)** the **pre-preprod local-first enforcement-mapping window** (`388d8073..0887b2ad`, the prior lead: `DC-PROTO-02` flip + Stream 3 + Plutus), **(2)** the **MEM-MEASURE + C2-venue band** (`92b78ee1..51884a78`: the bounded-admission + RSS↔replay memory-evidence substrate, the Preview-first C2 pivot, and the leader-election stake-source fixes — it introduced `OP-MEM-01` and the `mem_measure` module), and **(3)** the **MEM-OPT-OPS cluster** (`2a790fad..233644f7`, **the cluster this refresh closes**: mimalloc global allocator + streaming seed import + owned-footprint sampler). **38 commits** (no merges), **110 files changed, +11463 / −9128 lines**. **`OP-MEM-02` is the headline rule of band 3 and it STAYS `declared`** — the honest owned-memory measurement (the new RssAnon sampler) found **Ade heavier during admission** (Ade owned p50 4.59 GiB vs the Haskell reference 2.57 GiB), so MEM-OPT-OPS does **not** clear the owned posture; the on-disk UTxO (`MEM-OPT-UTXO-DISK`) is the gating lever, not the allocator/import wins. **MEM-OPT-OPS touches ZERO BLUE** (no `ade_core`/`ade_types`/`ade_codec`/`ade_crypto`/`ade_ledger`/`ade_plutus` file changed; BLUE canonical types unchanged over band 3); **no new crate** (still 11); the new `ade_node` evidence fields are **RED/GREEN observational, never authority**. The two BLUE changes in the full span (`+TxSubmissionTxId`, `+RedeemerFields`; `462 → 464`) are entirely band 1 (the prior pre-preprod window).

> **Baseline note (load-bearing — read before §0).** This refresh's baseline is **`862cd2cb`**, the PHASE4-N-AO
> cluster-close (it flipped `CN-CONS-03 → enforced`), and it is **valid**: `git rev-parse 862cd2cb` resolves and
> `git merge-base 862cd2cb HEAD == 862cd2cb` (it is a strict ancestor of HEAD; `862cd2cb` carries no tag). HEAD is
> **`233644f7`** (the MEM-OPT-OPS cluster-review-fixes commit). This is a **cluster-close refresh for `MEM-OPT-OPS`**
> (band 3, span `2a790fad..233644f7`). Two earlier bands sit between the prior doc lead and band 3 and were not yet
> narrated by this doc: the prior doc was regenerated to **`0887b2ad`** at `92b78ee1`, then the **MEM-MEASURE + C2** band
> (`92b78ee1..51884a78`) and finally **MEM-OPT-OPS** landed. This refresh keeps the prior pre-preprod window's §§0–7
> verbatim (now the "immediately-prior" section) and **leads with MEM-OPT-OPS** (§§0–7 below). All counts/§§ in the lead
> read at HEAD `233644f7`.
>
> **Working-tree note.** At this regen the working tree is **CLEAN** for tracked files — `git status --porcelain` shows
> only untracked scratch (`.mithril-scratch/`), not part of this doc. §1 narrates the committed span
> `862cd2cb..233644f7` verbatim from `git log`; §0/§6/§7 read rule **status** and canonical-type counts from the
> registry / BLUE trees at HEAD `233644f7` (`OP-MEM-02` **declared**, **378** rules, BLUE canonical types unchanged by
> band 3). **NB:** the *committed* `.idd-config.json` baseline still reads `862cd2cb`; the post-`233644f7` baseline bump
> is the named follow-on this regen performs.

---

# Lead — MEM-OPT-OPS cluster (`2a790fad → 233644f7`) + the MEM-MEASURE/C2 band (`92b78ee1 → 51884a78`)

**MEM-OPT-OPS** is the operator-memory cluster: it lands the first three memory levers (S1 ALLOC, S2 IMPORT, S3 MEASURE)
behind the unchanged BLUE ledger/consensus authority, with every change proven replay-equivalent and measured by the
RSS↔replay-verdict pairing that the **MEM-MEASURE** band (immediately before it) built. The cluster's headline rule
`OP-MEM-02` (Ade's *owned* resident memory clearly below the Haskell reference) **STAYS `declared`** — the honest
finding is the opposite: during active admission Ade is *heavier* on the owned metric, and the lever that closes the gap
is the on-disk UTxO (`MEM-OPT-UTXO-DISK`), not anything MEM-OPT-OPS ships. This is the load-bearing scope discipline of
the cluster: it shipped real import-side wins (mimalloc VmRSS −29.8%, streaming import peak −50.5%) **and** the
measurement that proves those wins are not yet enough.

### Band 3 — MEM-OPT-OPS (the cluster this refresh closes; `2a790fad..233644f7`)

> **MEM-OPT-OPS is RED/GREEN-only — ZERO BLUE change.** `git diff 2a790fad^..233644f7` over the configured BLUE
> `core_paths` trees (every `ade_core`/`ade_types`/`ade_codec`/`ade_crypto`/`ade_ledger`/`ade_plutus` path + the BLUE
> `ade_network` submodules) is **EMPTY**. BLUE canonical types are **unchanged** across band 3. Every new field is a RED
> measurement (`/proc/self/status` RSS) or a GREEN evidence record — never an authoritative output. The memory levers
> are allocator-swap (RED binary), streaming import (RED shell I/O), and observation (RED sampler); none touches the
> deterministic core.

- **`2a790fad` `docs(mem-opt)` — scaffold the MEM-OPT foundation.** Grounding (`docs/planning/mem-opt-grounding.md`) +
  a **3-cluster plan** (`docs/planning/mem-opt-cluster-plan.md`: OPS → UTXO-DISK → COMPACT) + the MEM-OPT-OPS cluster
  doc (`docs/clusters/MEM-OPT-OPS/{cluster,S1-alloc,S2-import,S3-measure}.md`) + **5 declared registry invariants**
  (`OP-MEM-02`, `DC-MEM-05`, `DC-MEM-06`, `DC-MEM-07`, `DC-MEM-08`).
- **`0f2dcbe6` + `861757f4` `feat(mem-opt)` — S1 ALLOC.** `#[global_allocator] static GLOBAL: mimalloc::MiMalloc`
  in `crates/ade_node/src/main.rs` (+7) + `mimalloc = "0.1"` in `crates/ade_node/Cargo.toml` (a **binary-only** dep,
  never a BLUE-crate dependency) + the new gate `ci/ci_check_alloc_determinism_neutral.sh` (asserts exactly one
  `#[global_allocator]` at the RED binary entry and **zero** allocator references in any BLUE crate — the allocator type
  is invisible to every canonical encoder/fingerprint). **`DC-MEM-06` flips `declared → partial`** (the
  allocator-neutrality clause is now mechanically enforced; the store-iteration-order clause stays declared pending the
  on-disk UTxO). **Live (`861757f4`):** on the identical A2 protocol against preprod, glibc → mimalloc dropped `VmRSS`
  p50 **6,874,024 → 4,824,884 kiB (−29.8%)**, `replay_verdict` agreed, 0 diverged — `CE-OPS-1` met.
- **`54975bb0` `feat(mem-opt)` — S2 IMPORT.** Streaming seed import in
  `crates/ade_runtime/src/seed_import/importer.rs` (+303) + the gate `ci/ci_check_mem_opt_s2_import_peak.sh` + the
  `rss_hwm_kib` / `seed_import` evidence tap. **Byte-identical** (bootstrap `initial_ledger_fp == S1`'s `fb7cb12a…`);
  import peak `seed_import VmHWM` **6,874,028 → 3,405,288 kiB (−50.5%)**; `replay_verdict` agreed, 0 diverged. The
  production path now **rejects any duplicate `TxIn` fail-closed** (`JsonSeedError::DuplicateTxIn`) — both the
  canonical-collision case (`#0` vs `#00`) and the exact-duplicate JSON string-key case. `DC-MEM-06` gains
  `strengthened_in += MEM-OPT-OPS` (the canonical fingerprint is now exercised on the streaming path too).
- **`3628ed16` `feat(mem-opt)` — S3 MEASURE.** Owned-footprint sampler in
  `crates/ade_node/src/mem_measure/rss_sampler.rs` (+46) + owned evidence fields + the gate
  `ci/ci_check_mem_opt_s3_owned.sh`. First measurement of the **OWNED** metric (`RssAnon`, excludes file-backed; readable
  for Ade *and* the Haskell node). Finding: `RssAnon ≈ VmRSS` at every point (so the "chain.db mmap pollutes gross VmRSS"
  hypothesis was **wrong** — redb's admission cost is anonymous write buffers + the `seed_to_snapshot` serialization,
  counted in `RssAnon`). Ade idle/recovered owned **1.95 GiB** (below the ≤3 GB target — the S1+S2 import-side wins are
  real), but **active-admission** owned p50 **4.59 GiB** (above target). **Honest owned comparison verdict:
  `ade_heavier`** (Ade owned p50 4.59 GiB vs Haskell windowed owned p50 2.57 GiB) — the OPPOSITE of the gross-VmRSS
  signal. **MEM-OPT-OPS alone does NOT clear the owned posture.**
- **`233644f7` `fix(mem-opt)` — cluster-review fixes.** Generalized the test-determinism normalizer to **all**
  observational `*_kib` fields and **corrected the dup-key equivalence claim** (the whole-buffer oracle is not
  authoritative for exact-duplicate JSON keys — serde collapses them last-wins; the streaming production path fails
  closed on every duplicate `TxIn` form, so equivalence is claimed only for valid unique-key seeds + malformed inputs
  where both paths agree). Touched `ci/ci_check_mem_measure_evidence.sh` (the only **modified** gate this band) + the
  admission/evidence reducers + the importer's dup-key tests.

### Band 2 — MEM-MEASURE + C2-venue (the substrate `OP-MEM-02` measures against; `92b78ee1..51884a78`)

> This band landed between the prior doc lead (`0887b2ad`, regenerated at `92b78ee1`) and MEM-OPT-OPS. It is **not** the
> cluster being closed here, but it built everything MEM-OPT-OPS measures with — the `mem_measure` module, the
> bounded-admission gate, and the RSS↔replay evidence pairing — so it is summarized for continuity.

- **`92b78ee1` `docs(grounding)`** — regenerated HEAD_DELTAS + TRACEABILITY to `0887b2ad` (the prior lead's own
  post-step; the source of the doc state this refresh extends).
- **C2-venue Preview pivot + leader-election stake fixes** (`ef8ac25f`, `71b59359`, `758ec953`, `bd5e4c23`, `78fd09d2`,
  `38bd1943`): `fix(consensus-inputs)` source the leader-election **`go`** stake (not stake-distribution); the
  **venue-parametric** live path (`C2-VENUE-PARAM`, `crates/ade_runtime/src/consensus_inputs/json.rs` +43) to pivot
  Preview-first; the new gate `ci/check_ade1_leader_stake_active.sh`; off-repo Preview node pointer; the Preview ADE1
  pool-registration manifest.
- **Admission fail-closed + raw-framing fixes** (`e497add0`, `02b5c9ad`): tolerate raw `[era,block]` block-fetch framing
  (drop the stale tag-24 unwrap) + the new `admission_raw_block_framing.rs` test + fixture; `fix(admission)` fail-closed
  guard — a bundle `source_tip` MUST equal the seed point.
- **MEM-MEASURE A1/A2 + COMPARE-D** (`a84f9045`, `fbe08b58`, `c54edb93`, `51884a78`): the **new `mem_measure` module**
  (`crates/ade_node/src/mem_measure/{mod,rss_sampler,bounded_admission,evidence,runner}.rs`, ~840 lines, all RED/GREEN —
  RED RSS sampler, GREEN bounded-admission gate + GREEN evidence record/validator + GREEN/RED runner) wired through
  `node_lifecycle.rs` + `lib.rs` + the admission-log/convergence-evidence reducers; the live preprod memory transcript
  (**`OP-MEM-01 → partial`**); and the committed **Haskell-vs-Ade RSS comparison** (BA-08, `MEM-COMPARE-D`): baseline
  Ade 6.56 GB vs Haskell 5.50 GB on preprod (verdict `ade_heavier`, +19%) — the baseline `OP-MEM-02` is measured against.
  CI gates added: `ci_check_bounded_inbound_admission.sh`, `ci_check_mem_measure_evidence.sh`,
  `ci_check_mem_compare_evidence.sh`.

## 0. Headline (full span `862cd2cb → 233644f7`; **bold = MEM-OPT-OPS delta**)

| Count | Baseline (`862cd2cb`) | Pre-MEM-OPT (`51884a78`) | HEAD (`233644f7`) | Δ (full span / **MEM-OPT-OPS**) |
|---|---|---|---|---|
| CI gates (`ci/ci_check_*.sh`) | 173 | 186 | **190** | **+17 new / 2 modified / 0 removed** full span; **MEM-OPT-OPS: +4 new** (`ci_check_alloc_determinism_neutral`, `ci_check_mem_opt_s1_reduction`, `ci_check_mem_opt_s2_import_peak`, `ci_check_mem_opt_s3_owned`) **+1 modified** (`ci_check_mem_measure_evidence.sh`). |
| Registry rules (`docs/ade-invariant-registry.toml`) | 372 | 373 | **378** | **+6** full span (`DC-PROTO-11` band 1; `DC-MEM-05/06/07/08` + `OP-MEM-02` MEM-OPT-OPS); **MEM-OPT-OPS: +5**. **Zero removed** (`comm -23` of sorted `id =` lists empty across every band). |
| Registry status (enforced / scaffolding / partial / declared) | 239 / 1 / 19 / 113 | 250 / 1 / 21 / 101 | **250 / 1 / 22 / 105** | **MEM-OPT-OPS: declared 101 → 105 (+4), partial 21 → 22 (+1), enforced 250 → 250 (0).** The +5 new rules land as **4 declared** (`DC-MEM-05/07/08`, `OP-MEM-02`) **+ 1 partial** (`DC-MEM-06`). No enforced flip in MEM-OPT-OPS. |
| **`OP-MEM-02` (owned memory below Haskell)** | — | — | **`declared`** | **THE headline rule — STAYS `declared`.** The S3 owned measurement returned verdict **`ade_heavier`** (Ade 4.59 GiB vs Haskell 2.57 GiB owned during admission); MEM-OPT-OPS does not clear the owned posture; **`MEM-OPT-UTXO-DISK` is the gating lever**. Honest no-flip. |
| `DC-MEM-06` (canonical fingerprint allocator-/order-neutral) | — | — | **`partial`** | **`declared → partial`** (S1: allocator-neutrality clause mechanically enforced) **+ `strengthened_in += MEM-OPT-OPS`** (S2: now exercised on the streaming-import path). Store-iteration-order clause stays declared pending the on-disk UTxO. |
| Declared → enforced flips (MEM-OPT-OPS) | — | — | **0** | MEM-OPT-OPS flips **no rule to enforced** — its one status move is `DC-MEM-06 declared → partial`. (The full span's 10 enforced flips are all band 1; see the immediately-prior section.) |
| BLUE canonical types | 462 | 464 | **464** | **+2 full span, ALL band 1** (`+TxSubmissionTxId`, `+RedeemerFields`). **MEM-OPT-OPS: +0** — band 3 touches no BLUE file. |
| Crates | 11 | 11 | **11** | **No new crate.** The only band-3 manifest change is the `mimalloc = "0.1"` binary-only dependency on `ade_node` (+ `Cargo.lock`). |
| Grounding docs (CODEMAP / SEAMS / TRACEABILITY) | regenerated to `862cd2cb` | still `862cd2cb` | still `862cd2cb` | **TWO windows + a cluster STALE.** All three pin `862cd2cb` (173 CI, 372 rules; TRACEABILITY reads 373 rules but still 173 CI). None carries band 1, band 2, or MEM-OPT-OPS: `grep -c` for `ci_check_mem_opt_s1_reduction` / `DC-MEM-06` / `mem_measure` in each = **0**. HEAD is 190 CI / 378 rules. Refresh to `233644f7` is the named follow-on. |

> **Grounding-doc state this regen (load-bearing).** **CODEMAP, SEAMS, and TRACEABILITY are all pinned to `862cd2cb`**
> (this refresh's BASELINE — they were regenerated there by the in-band PHASE4-N-AO close commit `388d8073`, before band 1
> even landed). They carry everything through PHASE4-N-AO but **nothing from band 1 (pre-preprod), band 2 (MEM-MEASURE/C2),
> or band 3 (MEM-OPT-OPS)**. They miss, cumulatively: **6 new rules** (`DC-PROTO-11`, `DC-MEM-05/06/07/08`, `OP-MEM-02`),
> the **+2 BLUE types**, the band-1 **10 declared→enforced flips**, the band-2 **`mem_measure` RED/GREEN module** + the
> band-3 **4 new MEM-OPT-OPS gates** (`grep -c` for each in all three = 0; CI pin 173 vs. HEAD 190; rule pin 372 vs. HEAD
> 378). The invariant registry holds all of it authoritatively at HEAD (**378 rules**). **Action:** regenerate CODEMAP +
> SEAMS + TRACEABILITY to `233644f7` as a follow-on so the new rules with their gates, the +2 BLUE types, the
> `mem_measure` module, and the MEM-OPT-OPS evidence levers all appear. Until then the registry is authoritative for the
> new bindings.

The thread↔rule↔gate map for the MEM-OPT-OPS cluster (the full verbatim log is §1):

| Slice | Rule(s) | Gate | What shipped |
|---|---|---|---|
| **Foundation** (`2a790fad`) | `OP-MEM-02`, `DC-MEM-05`, `DC-MEM-06`, `DC-MEM-07`, `DC-MEM-08` (all **declared**) | — | Grounding + 3-cluster plan + OPS cluster doc + 5 declared invariants. |
| **S1 ALLOC** (`0f2dcbe6`, `861757f4`) | **`DC-MEM-06`** (`declared → partial`); `OP-MEM-02` (stays declared) | `ci_check_alloc_determinism_neutral.sh` (NEW); `ci_check_mem_opt_s1_reduction.sh` (NEW) | mimalloc `#[global_allocator]` + binary-only dep; live VmRSS −29.8%; `CE-OPS-1` met. |
| **S2 IMPORT** (`54975bb0`) | **`DC-MEM-06`** (`strengthened_in += MEM-OPT-OPS`); `OP-MEM-02` (stays declared) | `ci_check_mem_opt_s2_import_peak.sh` (NEW) | Streaming seed import; byte-identical; import peak −50.5%; DuplicateTxIn fail-closed. |
| **S3 MEASURE** (`3628ed16`) | `OP-MEM-02` (stays declared — verdict `ade_heavier`) | `ci_check_mem_opt_s3_owned.sh` (NEW) | Owned `RssAnon` sampler + honest owned comparison; owned p50 4.59 GiB vs Haskell 2.57 GiB. |
| **Cluster-review fixes** (`233644f7`) | (no status change) | `ci_check_mem_measure_evidence.sh` (modified) | Generalize the `*_kib` test-determinism normalizer + correct the dup-key equivalence claim. |

## 1. Commit Log (newest first, full span `862cd2cb..233644f7`)

| Hash | Type | Summary |
|------|------|---------|
| `233644f7` | fix | fix(mem-opt): MEM-OPT-OPS cluster-review fixes -- normalize all observational *_kib + correct the dup-key equivalence claim |
| `3628ed16` | feat | feat(mem-opt): MEM-OPT-OPS S3 MEASURE -- owned-footprint sampler + honest owned comparison; verdict ade_heavier |
| `54975bb0` | feat | feat(mem-opt): MEM-OPT-OPS S2 IMPORT -- streaming seed import (byte-identical), import peak halved |
| `861757f4` | feat | feat(mem-opt): MEM-OPT-OPS S1 ALLOC live -- CE-OPS-1 met, VmRSS -29.8% on preprod with mimalloc |
| `0f2dcbe6` | feat | feat(mem-opt): MEM-OPT-OPS S1 ALLOC -- mimalloc global allocator + DC-MEM-06 determinism-neutral gate |
| `2a790fad` | docs | docs(mem-opt): scaffold MEM-OPT foundation -- grounding + 3-cluster plan + OPS cluster doc + 5 declared invariants |
| `51884a78` | docs | docs(mem-measure): MEM-COMPARE-D -- committed Haskell-vs-Ade RSS comparison (BA-08); Ade loses by ~1 GB |
| `c54edb93` | feat | feat(mem-measure): MEM-MEASURE-A2 operator pass -- live preprod memory transcript (OP-MEM-01 -> partial) |
| `fbe08b58` | feat | feat(mem-measure): MEM-MEASURE-A2 build -- live memory-evidence instrumentation |
| `a84f9045` | feat | feat(mem-measure): MEM-MEASURE-A1 bounded inbound admission + memory-evidence substrate |
| `02b5c9ad` | fix | fix(admission): fail-closed guard -- bundle source_tip must equal the seed point |
| `6d052db8` | fix | fix(consensus-inputs): source the `set` stake snapshot for leader election (was `go`) |
| `e497add0` | fix | fix(admission): tolerate raw [era,block] block-fetch framing (drop stale tag-24 unwrap) |
| `38bd1943` | docs | docs(evidence): add preview ADE1 pool registration manifest |
| `78fd09d2` | chore | chore(c2): point --network preview at off-repo ~/.cardano-node-preview |
| `bd5e4c23` | feat | feat(c2): venue-parametric live path (C2-VENUE-PARAM) — Preview-first pivot |
| `71b59359` | docs | docs(c2): correct epoch ETA — faucet reaches leader-election `go` at epoch 297 |
| `ef8ac25f` | fix | fix(consensus-inputs): source leader-election `go` stake, not stake-distribution |
| `758ec953` | docs | docs(evidence): refresh preprod ADE1 manifest — faucet delegation + forging keys |
| `92b78ee1` | docs | docs(grounding): regenerate HEAD_DELTAS + TRACEABILITY to 0887b2ad |
| `0887b2ad` | feat | feat(ade_network): flip DC-PROTO-02 enforced -- live tx-submission2 full exchange closes the last surface |
| `92b855c4` | feat | feat(ade_network): tx-submission2 codec on cardano-node's real wire form (era-tagged txid + indefinite arrays) |
| `04a857a3` | docs | docs(planning): DC-PROTO-02 assessment + option-B routing (Stream 1) |
| `717febaa` | feat | feat(ci): Plutus conformance manifest -> flip CN-PLUTUS-01 enforced (Stream 1 / slice A4) |
| `dec0fd22` | feat | feat(ci): host-environment purity gate -> flip CN-PLUTUS-04 enforced (Stream 1 / slice A2) |
| `b25d1594` | test | test(ade_plutus): broaden the adversarial Plutus reject corpus (Stream 1 / slice A3) |
| `ed408410` | fix | fix(ade_plutus): per-script declared ex_units cap -- close a Plutus false-accept (Stream 1 / slice A1) |
| `55a8a7e1` | feat | feat(ci): required-signer closure gate -- lock the witness enumeration (Stream 1 / slice B) |
| `91f63195` | test | test(ade_ledger): double-spend adversarial coverage -- close the CN-LEDGER-08 gap (Stream 1 / slice C) |
| `378ca2ca` | feat | feat(ci): FSM transition-purity gate -> flip DC-PROTO-01/06 enforced; Stream 3 complete |
| `7f13d646` | feat | feat(ci): mini-protocol surface gate -> flip DC-PROTO-03/04 enforced (Stream 3) |
| `37d4c068` | feat | feat(ci): header-body binding gate -> flip CN-CONS-04 enforced (Stream 3) |
| `8e8f8eb0` | feat | feat(ci): closed-codec-message gate -> flip CN-WIRE-07 enforced (Stream 3) |
| `b01d1d38` | docs | docs(planning): Stream-3 classification + conservative routing |
| `c27ee281` | docs | docs(registry): flip DC-CORE-01 enforced (Stream 3) -- BLUE sync-only, gate-complete |
| `86252176` | docs | docs(planning): pre-preprod local-first work streams + strict order (3->1->2) |
| `5532ddf4` | docs | docs(c2-guide): rung 2 fully closed -- CN-CONS-03 enforced (PHASE4-N-AO) |
| `388d8073` | docs | docs(phase4-n-ao): cluster-close housekeeping -- regenerate grounding docs + archive + groom registry |

No merge commits in the span. **38 commits, zero unclassified** — every subject carries an explicit conventional-commits
prefix: **`feat`×17**, **`docs`×12**, **`fix`×6**, **`test`×2**, **`chore`×1** (= 38). The **MEM-OPT-OPS** band
(`2a790fad..233644f7`, 6 commits) is `docs`×1 (foundation) + `feat`×4 (S1 build + S1 live + S2 + S3) + `fix`×1
(cluster-review). The **MEM-MEASURE/C2** band (`0887b2ad..51884a78`, 14 commits incl. the `92b78ee1` doc-regen) is
`feat`×4 (MEM-MEASURE A1, A2 build/pass + C2-VENUE-PARAM) + `fix`×4 + `docs`×5 + `chore`×1.

> **Note (commit-attribution policy).** Per this repo's `CLAUDE.md` override (vibe-coded-node bounty trailer
> requirement), commits in this repo carry a `Co-Authored-By:` model-attribution trailer; that is an Ade-local override
> of the global no-AI-attribution rule and applies to **commit messages only**. It does not affect this doc's content.

## 2. New Modules

**No new library crate or `src/` library module in MEM-OPT-OPS.** `git diff --diff-filter=A --name-only 2a790fad^..HEAD
-- 'crates/**/*.rs'` is **empty for `src/` modules** — MEM-OPT-OPS only *modifies* existing files (it extends the band-2
`mem_measure/rss_sampler.rs` with the owned sampler, and `main.rs` with the global allocator). `git diff
--diff-filter=A '**/Cargo.toml'` is empty across the whole span (still **11 crates**). The one **new** library module in
the full span belongs to **band 2 (MEM-MEASURE)**, not MEM-OPT-OPS:

| Module | Color | Purpose | Key sub-paths | Added in |
|--------|-------|---------|---------------|----------|
| `ade_node::mem_measure` | **RED + GREEN** (file-separated, three TCB colors) | Bounded inbound admission + memory-measurement substrate: pairs RED RSS observations with a GREEN replay fingerprint so a memory number can never masquerade as authority. The substrate `OP-MEM-01`/`OP-MEM-02` are measured against. | `rss_sampler.rs` (**RED** — reads `/proc/self/status`, observational only), `bounded_admission.rs` (**GREEN** — deterministic bounded-ingress gate, `CN-MEM-01`), `evidence.rs` (**GREEN** — `MemEvidenceRecord` + validator + replay-fingerprint pairing), `runner.rs` (**GREEN/RED seam** — drives the bounded workload under RSS sampling), `mod.rs` | `MEM-MEASURE-A1` (band 2) |

**MEM-OPT-OPS adds no new module** — it extends `mem_measure::rss_sampler` (owned `RssAnon`/`Private_Dirty` sampler,
+46) and `crates/ade_node/src/main.rs` (the `#[global_allocator]`, +7), and adds the streaming path to the existing
`ade_runtime::seed_import::importer` (+303). The 7 MEM-OPT-OPS **evidence transcripts**
(`docs/evidence/mem-opt-ops-s{1,2,3}-*.{jsonl,md}`) are data, not source.

> **Cross-reference (CODEMAP) — the `mem_measure` module is NOT yet registered; CODEMAP is stale.** Neither
> `mem_measure` nor any of its files appears in `docs/ade-CODEMAP.md` (`grep -c` = **0**) — CODEMAP is pinned to
> `862cd2cb`, before band 2. **Action:** run `/codemap` (and `/seams`, `/traceability`) to `233644f7` so the
> `mem_measure` RED/GREEN module lands in CODEMAP §RED/§GREEN, before relying on CODEMAP for the memory-measurement
> surface.

## 3. Modules Modified

MEM-OPT-OPS modified **`ade_runtime::seed_import::importer`** (the streaming-import bulk, RED shell), **`ade_node` main +
`mem_measure::rss_sampler`** (the allocator + owned sampler, RED), and the **`ade_node` admission/evidence reducers**
(GREEN observation fields). The rest of the band-3 churn is the cluster docs + the 7 evidence transcripts. (Band-2
module modifications — the `node_lifecycle.rs` / admission-log / convergence-evidence wiring for the `mem_measure`
substrate, +1318 lines — are summarized in the band-2 section above.)

| Module | Color / scope | Key changes (MEM-OPT-OPS) |
|--------|---------------|---------------------------|
| `ade_runtime::seed_import::importer` (`importer.rs` **+303**) | **RED** shell I/O, additive | **S2 streaming seed import.** Streams the seed JSON instead of buffering the whole file, halving the import peak (`seed_import VmHWM` −50.5%); computes the **byte-identical** canonical `UtxoFingerprint` regardless of parse/textual order; **rejects any duplicate `TxIn` fail-closed** (`JsonSeedError::DuplicateTxIn` — both the `#0`/`#00` canonical-collision and the exact-duplicate-string cases). |
| `ade_node` main (`main.rs` **+7**) | **RED** binary entry, additive | **S1.** `#[global_allocator] static GLOBAL: mimalloc::MiMalloc` — the process allocator that returns freed pages to the OS. Invisible to every BLUE crate (enforced by `ci_check_alloc_determinism_neutral.sh`). |
| `ade_node::mem_measure::rss_sampler` (`rss_sampler.rs` **+46**, `mod.rs` **+5**) | **RED** observational, additive | **S3 owned sampler.** Adds the **owned** metric (`RssAnon`, excludes file-backed) alongside `VmRSS`/`VmHWM`; readable for Ade and the Haskell node, so a true owned comparison is possible. |
| `ade_node::admission::{runner,bootstrap}` (`runner.rs` **+121**, `bootstrap.rs` **+15**) | **GREEN/RED** admission orchestration, additive | Threads the memory-evidence sampling through the admission run + bootstrap (observational; never alters the admission decision). |
| `ade_node::convergence_evidence` (`convergence_evidence.rs` **+47**) | **GREEN** evidence, additive | Adds the observational `*_kib` memory fields to the convergence-evidence record (closed vocabulary; the `233644f7` review generalized the test-determinism normalizer over **all** `*_kib` fields). |
| `ade_node::admission_log::{event,writer}` (`event.rs` **+24**, `writer.rs` **+40**) | **GREEN** closed evidence vocab, additive | Memory-evidence fields added to the admission-log event + writer (allow-list closed vocabulary). |
| `ade_node` tests (`admission_adversarial_corpus.rs`, `admission_cross_epoch_guard.rs`, `admission_replay_equivalence.rs`, `live_fork_choice_ai_s4bii.rs`) | **test**, additive | Updated for the new observational fields + the dup-key fail-closed behavior. |
| cluster docs + evidence (`docs/clusters/MEM-OPT-OPS/`, `docs/evidence/mem-opt-ops-s*`, `docs/planning/mem-opt-*`) | docs | The MEM-OPT-OPS cluster doc + 4 slice docs + the 3-cluster plan + grounding + the 7 S1/S2/S3 evidence transcripts. |

> **BLUE was NOT touched in MEM-OPT-OPS (load-bearing).** `git diff 2a790fad^..233644f7` over the configured BLUE
> `core_paths` trees is **EMPTY**: no `ade_core`/`ade_types`/`ade_codec`/`ade_crypto`/`ade_ledger`/`ade_plutus` file and
> no BLUE `ade_network` submodule changed. Every band-3 change is RED (allocator / streaming I/O / RSS sampler) or GREEN
> observation (closed-vocab evidence fields). BLUE canonical types are **unchanged** across band 3 — the new memory
> fields are observational, never authoritative outputs, so they add **zero** canonical type. (The full-span +2 BLUE
> types are entirely band 1; §6.)

## 4. Feature Flags

**No project feature-flag deltas in any band.** Ade declares no `[features]` table in any workspace `Cargo.toml` at any
ref (`git grep '^\[features\]'` is empty at `862cd2cb`, `51884a78`, and HEAD). **MEM-OPT-OPS introduces no
`#[cfg(feature = …)]` gate** (`git diff 2a790fad^..HEAD -- 'crates/**/*.rs' | grep -c '^+.*cfg(feature'` = **0**), **no
`compile_error!` coupling** (grep = **0**), and **no new CLI flag** (`crates/ade_node/src/cli.rs` untouched in band 3).
The sole band-3 manifest change is the **`mimalloc = "0.1"` dependency** on `crates/ade_node/Cargo.toml` (a binary-only
dependency on the RED node crate, never a feature flag and never a BLUE-crate dependency) + the corresponding
`Cargo.lock` entries (`mimalloc`, `libmimalloc-sys`). There is no feature-flag coupling to report.

## 5. CI Checks (173 → 190 over the full span; +17 new, 2 modified, 0 removed · **MEM-OPT-OPS: +4 new, 1 modified**)

Across the full span, **17** CI scripts were added, **2 modified**, **0 removed** (`ls ci/ci_check_*.sh | wc -l` =
**173 → 190**; `--diff-filter=D` over `ci/` empty). The **MEM-OPT-OPS** band adds **4** new gates + **modifies 1**
(`ci_check_mem_measure_evidence.sh`, the cluster-review generalization). Grouping below isolates the MEM-OPT-OPS gates;
the band-2 and band-1 gates follow.

### MEM-OPT-OPS — memory levers (new gates this cluster)

| Check | Status | What it checks |
|-------|--------|----------------|
| `ci_check_alloc_determinism_neutral.sh` | **New** (`DC-MEM-06`) | Exactly one `#[global_allocator]` at the RED binary entry, and **zero** allocator references in any BLUE crate (incl. the BLUE `ade_network` submodules) — the allocator type is invisible to every canonical encoder/fingerprint (static allocator-neutrality; the runtime guarantee rides the standing `T-DET-01` apply-twice corpus + `DC-WAL-03`). |
| `ci_check_mem_opt_s1_reduction.sh` | **New** (`OP-MEM-02` evidence) | The committed S1 transcript shows the mimalloc `VmRSS` reduction (p50 −29.8%) on the identical A2 protocol, `replay_verdict` agreed, 0 diverged (`CE-OPS-1`). |
| `ci_check_mem_opt_s2_import_peak.sh` | **New** (`DC-MEM-06`, `OP-MEM-02` evidence) | The streaming seed import is **byte-identical** to the whole-buffer path (same canonical `UtxoFingerprint`, parse-order-independent) and **halves the import peak**; the production path rejects every duplicate `TxIn` form fail-closed. |
| `ci_check_mem_opt_s3_owned.sh` | **New** (`OP-MEM-02` evidence) | The owned-metric (`RssAnon`) sampler is present on Linux and the honest owned comparison is recorded (Ade owned p50 vs the Haskell windowed owned p50); asserts the verdict is **computed and committed** (it is `ade_heavier` at HEAD — the gate enforces honest measurement, not a flip). |

### MEM-OPT-OPS — modified gate

| Check | Status | What it checks |
|-------|--------|----------------|
| `ci_check_mem_measure_evidence.sh` | **Modified** | The cluster-review fix (`233644f7`) generalized the observational-field normalizer to **all** `*_kib` fields so the RSS↔replay evidence record stays test-deterministic regardless of which memory field is sampled. |

### Band 2 — MEM-MEASURE + C2 (new gates, summarized)

| Check | Status | What it checks |
|-------|--------|----------------|
| `ci_check_bounded_inbound_admission.sh` | **New** (`CN-MEM-01`) | The GREEN bounded inbound-admission gate admits untrusted work through a deterministic bound before the scarce authoritative resource. |
| `ci_check_mem_measure_evidence.sh` | **New** (`OP-MEM-01`) | The MEM-MEASURE evidence record pairs an RSS measurement with a replay fingerprint (memory ↔ replay-verdict pairing). |
| `ci_check_mem_compare_evidence.sh` | **New** (`OP-MEM-02` baseline) | The committed Haskell-vs-Ade RSS comparison (BA-08 / MEM-COMPARE-D) is well-formed and carries a verdict. |
| `check_ade1_leader_stake_active.sh` | **New** (C2 venue) | The ADE1 pool's leader-election stake is active at the target epoch (Preview/preprod venue gate). |

> **Cross-reference (CODEMAP + SEAMS + TRACEABILITY) — stale; all MEM-OPT-OPS + MEM-MEASURE gates absent.** The 4 new
> MEM-OPT-OPS gates (and the band-2 gates) are recorded **in the registry at HEAD** (`docs/ade-invariant-registry.toml`,
> 378 rules) but are **NOT yet in TRACEABILITY, SEAMS, or CODEMAP**, all three pinned to `862cd2cb` (`grep -c` for
> `ci_check_mem_opt_s1_reduction` / `ci_check_alloc_determinism_neutral` / `ci_check_mem_measure_evidence` in TRACEABILITY
> = **0**; CI-count pin **173** vs. HEAD **190**). **No gate is orphaned** — each MEM-OPT-OPS gate binds a registry rule
> (`DC-MEM-06` for the determinism/import gates; `OP-MEM-02`'s `ci_script`/evidence for s1/s3). **Action:** regenerate
> CODEMAP + SEAMS + TRACEABILITY to `233644f7`; until then the registry is authoritative.

## 6. Canonical Type Registry Delta

**n/a — no separate canonical-type registry is configured** (`canonical_type_registry: null`); canonical-type rules
live inline in the invariant registry under family **T**. **MEM-OPT-OPS added ZERO BLUE canonical types** — `git diff
2a790fad^..233644f7` over the BLUE `core_paths` trees is empty, and the `struct`/`enum` count over those trees is
unchanged across band 3. The new memory fields are RED measurements / GREEN observational evidence, never authoritative
types.

Over the **full span** the established BLUE-tree canonical-type metric moves **`462 → 464`** (the +2 are **entirely band 1**,
the prior pre-preprod window):

- **`+TxSubmissionTxId`** — `pub struct` in `crates/ade_network/src/codec/tx_submission.rs` (the band-1 `DC-PROTO-11`
  era-tagged txid wire type).
- **`+RedeemerFields`** — private `struct` in `crates/ade_plutus/src/tx_eval.rs` (the band-1 A1 ex_units-cap fix).

**Zero BLUE canonical types removed** in any band.

## 7. Normative / Invariant Rule Delta (372 → 378 full span; **MEM-OPT-OPS: +5, zero removals**)

**MEM-OPT-OPS added 5 rule IDs; zero removed** (registry **373 → 378** across band 3; `comm -23` of the sorted `id =`
lists is empty — exactly five additions, no removal). The status tally over MEM-OPT-OPS moves **declared 101 → 105**
(+4), **partial 21 → 22** (+1), **enforced 250 → 250** (no enforced flip). The +5 new rules land as **4 declared + 1
partial**.

*(The configured `normative_docs` — the CE-79 tier-gate statement + addendum, the three contract docs, the CE-73
reclassification, and `CLAUDE.md` — were **not** changed anywhere in the full span: `git diff --name-only
862cd2cb..HEAD` over those paths is empty. The §7 delta is entirely the invariant-registry change.)*

**New MEM-OPT-OPS rules (`+5`):**

| Rule | Family / Tier · Status @ HEAD | Statement (summary) |
|------|-------------------------------|---------------------|
| `OP-MEM-02` | OP / `operational` · **`declared`** · `introduced_in = "MEM-OPT-OPS"` (foundation) | **Owned resident memory below the Haskell reference.** Ade's *owned* resident memory (`Private_Dirty`/`RssAnon`) under a representative venue stays clearly below the reference Haskell `cardano-node`'s on the same chain, WITHOUT changing ledger semantics, chain selection, persisted bytes, or replay-equivalence. **STAYS declared** — S3 verdict `ade_heavier` (Ade 4.59 GiB vs Haskell 2.57 GiB owned during admission); flips when the committed comparison becomes `ade_below` + the RSS-ceiling gate is green. `MEM-OPT-UTXO-DISK` is the gating lever. |
| `DC-MEM-05` | DC / `derived` · **`declared`** | **UTxO/ledger state is storage-backend-independent.** An in-memory UTxO and an on-disk UTxO produce byte-identical replay (same WAL + checkpoint ⇒ same tail fingerprint). A memory-representation/storage change is NEVER a consensus or replay change. (The on-disk-UTxO invariant `MEM-OPT-UTXO-DISK` enforces; declared here.) |
| `DC-MEM-06` | DC / `derived` · **`partial`** · `introduced_in = "MEM-OPT-OPS"` · `strengthened_in = ["MEM-OPT-OPS"]` | **Canonical fingerprint is allocator- and store-order-neutral.** The fingerprint is computed by the canonical CBOR encoder over canonically-encoded fixed-width big-endian keys, NEVER from a backend's native iteration order, AND is independent of the process allocator. **`declared → partial`** (S1: allocator-neutrality clause mechanically enforced via `ci_check_alloc_determinism_neutral.sh`; **strengthened** S2: now also exercised on the streaming-import path). The store-iteration-order clause stays declared pending the on-disk UTxO. |
| `DC-MEM-07` | DC / `derived` · **`declared`** | A memory/representation lever's effect is measured by the RSS↔replay-verdict pairing (the MEM-MEASURE A2 substrate); a memory number is paired with a replay fingerprint and is never an authoritative output. |
| `DC-MEM-08` | DC / `derived` · **`declared`** | (MEM-OPT representation/compaction invariant — declared at scoping for the `MEM-OPT-COMPACT` cluster; observational, no authority effect.) |

**Status moves (`DC-MEM-06`, the only MEM-OPT-OPS status change):** `declared → partial` (S1) + `strengthened_in +=
MEM-OPT-OPS` (S2). **No MEM-OPT-OPS rule flipped to `enforced`.**

**No rule was removed (expected: 0).** The MEM-OPT-OPS registry delta is **5 new rules (4 declared + 1 partial), 1
strengthening (`DC-MEM-06`), zero removals** — consistent with append-only registry discipline.

> **Honest no-flip (surfaced, NOT an anomaly).** `OP-MEM-02` is the cluster's headline rule and it **stays `declared` by
> design**: the S3 owned measurement is the cluster's own evidence that MEM-OPT-OPS does not clear the owned posture
> (verdict `ade_heavier`). This is the correct application of the proof discipline — the cluster shipped the import-side
> wins (mimalloc, streaming import) **and** the measurement that shows they are not yet sufficient, and it did not flip
> the rule on the gross-VmRSS signal that would have overstated the result. The gating lever is the on-disk UTxO
> (`MEM-OPT-UTXO-DISK`), the next cluster in the 3-cluster plan.

## Honest residual (MEM-OPT-OPS scope)

This cluster shipped three memory levers (S1 ALLOC, S2 IMPORT, S3 MEASURE) and the measurement to judge them. The honest
residual:

- **`OP-MEM-02` does NOT flip — Ade is heavier on the owned metric during admission.** The new owned (`RssAnon`)
  sampler's verdict is `ade_heavier` (Ade 4.59 GiB vs Haskell 2.57 GiB). The import-side wins (mimalloc VmRSS −29.8%,
  streaming import peak −50.5%) are real and committed, but the dominant active-admission owned cost is the
  `seed_to_snapshot`/`chain.db` serialization, which **MEM-OPT-OPS does not address**. `MEM-OPT-UTXO-DISK` is the gating
  lever.
- **`DC-MEM-06` is `partial`, not `enforced`.** Only the allocator-neutrality clause is mechanically enforced; the
  store-iteration-order clause stays declared because there is no on-disk store yet (`MEM-OPT-UTXO-DISK` will supply it).
- **All MEM-OPT-OPS changes are RED/GREEN observational — ZERO BLUE, zero authority effect.** The allocator swap,
  streaming I/O, and RSS sampler never enter the deterministic core; the new evidence fields are closed-vocabulary
  observations, replay-neutral. The byte-identical streaming-import claim is honestly scoped (equivalence claimed only
  for valid unique-key seeds + malformed inputs where both paths agree; the exact-duplicate-string-key case is a pinned,
  documented oracle asymmetry — the `233644f7` correction).
- **No new crate, no new library module, no feature flag, no CLI flag.** The only manifest change is the `mimalloc`
  binary-only dependency on `ade_node`. (The full-span new module `mem_measure` is band 2, not MEM-OPT-OPS.)
- **CODEMAP + SEAMS + TRACEABILITY refresh owed (two windows + a cluster).** All three pin `862cd2cb` and miss band 1
  (pre-preprod: `DC-PROTO-11`, +2 BLUE types, 10 flips), band 2 (`mem_measure` module, `OP-MEM-01`), and MEM-OPT-OPS
  (5 rules, 4 gates). The registry holds all of it authoritatively at HEAD (378 rules); regenerating to `233644f7` is the
  named follow-on. No orphan gate (each new gate binds a registry rule).

## Working tree at HEAD `233644f7` (clean for tracked files)

**The working tree is CLEAN for tracked files at this regen** — `git status --porcelain` shows only untracked scratch
(`.mithril-scratch/`), not part of this doc. §1 narrates the committed span `862cd2cb..233644f7` verbatim; §0/§6/§7 read
rule status + canonical-type counts from the registry / BLUE trees at HEAD (`OP-MEM-02` declared, 378 rules, BLUE
canonical types unchanged by band 3). The remaining follow-on actions are: (a) bump `.idd-config.json`
`head_deltas_baseline` `862cd2cb → 233644f7`, and (b) the CODEMAP + SEAMS + TRACEABILITY refresh to `233644f7`
(surfaced in §2 and §5).

---

## Immediately-prior — pre-preprod local-first enforcement-mapping window (`862cd2cb → 0887b2ad`)

> The section below is the **previous** HEAD_DELTAS lead, preserved verbatim. It narrated the `862cd2cb → 0887b2ad` span —
> **band 1** of the current full span: the pre-preprod local-first enforcement-mapping pass scoped by `86252176`,
> ordered strictly **3 → 1 → 2** to flip as many `declared` rules to `enforced` as the existing code already justified.
> Headline flips: **`DC-PROTO-02`** (the last N2N+N2C mini-protocol surface, on a live tx-submission2 full-exchange
> real-capture corpus) + the six **Stream 3** wire-FSM/codec/BLUE-sync flips + the two **Plutus** flips (`CN-PLUTUS-01`,
> `CN-PLUTUS-04`). **18 commits, 51 files, +4182 / −6652.** **+1 new rule `DC-PROTO-11`** (enforced), **10
> declared→enforced flips**, **+1 strengthening** (`DC-PROTO-02 += TXSUB2-CODEC-REALWIRE`), **zero removals** (372 → 373
> rules; status 239/1/19/113 → 250/1/19/103). **TOUCHED BLUE — +2 canonical types** (`462 → 464`: `+TxSubmissionTxId`
> `ade_network::codec::tx_submission`; `+RedeemerFields` `ade_plutus::tx_eval`). **+10 CI gates** (173 → 183). **One new
> RED capture bin** (`ade_tx_submission2_server_capture`) + a real-capture corpus; **no new crate** (11); no `[features]`,
> no `cfg(feature)`, no `compile_error!`, no new CLI flag.

### Band-1 detail (preserved)

This window is **not a cluster** in the slice-doc sense — it is a **pre-preprod local-first enforcement-mapping pass**
scoped by `86252176`, ordered strictly **3 → 1 → 2**. It decomposes into four threads:

1. **Stream 3 — wire-FSM / codec / BLUE-sync enforcement (6 flips).** `CN-WIRE-07` (closed codec-message taxonomy,
   `ci_check_codec_message_closed.sh`), `CN-CONS-04` (header/body binding, `ci_check_header_body_binding.sh`),
   `DC-PROTO-03` + `DC-PROTO-04` (mini-protocol surface, `ci_check_mini_protocol_surface.sh`), `DC-PROTO-01` +
   `DC-PROTO-06` (FSM transition purity, `ci_check_mini_protocol_transition_purity.sh`), `DC-CORE-01` (BLUE sync-only,
   against the existing `ci_check_no_async_in_blue.sh`). Gate/docs-only — no production code changed in Stream 3.
2. **Stream 1 Plutus — a real false-accept fix + IOG conformance (2 flips).** `ade_plutus::tx_eval` gained a per-script
   declared-`ex_units` cap (the A1 `fix`, +`RedeemerFields`); `ci_check_plutus_eval_purity.sh` flips `CN-PLUTUS-04`; a
   registry-bound IOG `plutus-conformance` manifest + `ci_check_plutus_conformance.sh` flips `CN-PLUTUS-01` (514/514).
3. **Stream 1 ledger coverage (slices B, C — gate + corpus, NO flip).** `ci_check_required_signer_closure.sh` attaches
   to the still-`partial` `DC-LEDGER-05`; a double-spend corpus broadens `ade_ledger` coverage; `CN-LEDGER-08` stays
   `declared` (the slice-C commit subject says "close the CN-LEDGER-08 gap" but the registry status did not change — a
   commit-intent-vs-status mismatch, not a removal).
4. **tx-submission2 real-wire codec (`TXSUB2-CODEC-REALWIRE`) — +1 new rule + 1 flip.** A new RED server-side capture
   harness (`ade_tx_submission2_server_capture`, option B) surfaced a real Cardano incompatibility (era-tagged txids
   `[6, h'..32']` inside CBOR indefinite arrays) that Ade's prior codec false-rejected. The fix (`DC-PROTO-11`, new,
   enforced) accepts + byte-identically preserves that wire form; with the live full exchange captured, **`DC-PROTO-02`
   flips `declared → enforced`** — the last of the 11 N2N+N2C mini-protocol surfaces.

Band 1's headline table (rule↔gate), preserved:

| Thread / slice | Rule(s) | Gate | What shipped |
|---|---|---|---|
| **Stream 3** (`c27ee281`) | `DC-CORE-01` (→ enforced) | `ci_check_no_async_in_blue.sh` (existing) | BLUE sync-only flip (docs/registry only). |
| **Stream 3** (`8e8f8eb0`) | `CN-WIRE-07` (→ enforced) | `ci_check_codec_message_closed.sh` (NEW) | Closed codec-message taxonomy gate. |
| **Stream 3** (`37d4c068`) | `CN-CONS-04` (→ enforced) | `ci_check_header_body_binding.sh` (NEW) | Header/body binding gate. |
| **Stream 3** (`7f13d646`) | `DC-PROTO-03` + `DC-PROTO-04` (→ enforced) | `ci_check_mini_protocol_surface.sh` (NEW) | Mini-protocol surface gate. |
| **Stream 3** (`378ca2ca`) | `DC-PROTO-01` + `DC-PROTO-06` (→ enforced) | `ci_check_mini_protocol_transition_purity.sh` (NEW) | FSM transition-purity gate; Stream 3 complete. |
| **Stream 1 / A1** (`ed408410`) | (Plutus false-accept fix) | — | **BLUE fix** in `ade_plutus::tx_eval` — per-script ex_units cap (+`RedeemerFields`). |
| **Stream 1 / A2** (`dec0fd22`) | `CN-PLUTUS-04` (→ enforced) | `ci_check_plutus_eval_purity.sh` (NEW) | Host-environment purity gate (+`plutus_budget_cap`, `plutus_oracle_no_false_accept`). |
| **Stream 1 / A4** (`717febaa`) | `CN-PLUTUS-01` (→ enforced) | `ci_check_plutus_conformance.sh` (NEW) | Registry-bound IOG conformance manifest gate. |
| **Stream 1 / B** (`55a8a7e1`) | `DC-LEDGER-05` (stays `partial`) | `ci_check_required_signer_closure.sh` (NEW) | Required-signer closure gate. **No flip.** |
| **Stream 1 / C** (`91f63195`) | `CN-LEDGER-08` (stays `declared`) | — | Double-spend adversarial coverage. **No flip.** |
| **TXSUB2** (`92b855c4`) | `DC-PROTO-11` (NEW, → enforced) | `ci_check_tx_submission2_real_capture.sh` (NEW) | tx-submission2 codec on the real wire form (+`TxSubmissionTxId`); new RED capture bin + corpus. |
| **TXSUB2** (`0887b2ad`) | `DC-PROTO-02` (→ enforced) | `ci_check_tx_submission2_real_capture.sh` | **Flip `DC-PROTO-02`** — live full exchange closes the last surface. |

The full §§0–7 narrative for band 1 (new modules / modules modified / feature flags / CI checks / canonical-type delta /
rule delta, with all cross-reference warnings) is recoverable from this doc's git history at `0887b2ad`. Its headline:
**`DC-PROTO-02` enforced** (the last wire surface) + the six Stream 3 flips + the two Plutus flips; **no `RO-LIVE`
flip**; **BLUE touched, +2 canonical types**; **+10 CI gates**; **+1 rule, zero removals**.

---

## Historical — PHASE4-N-AO live multi-candidate fork-choice SELECT + `CN-CONS-03` flip (`31efec44 → 862cd2cb`)

> The section below is a **previous** HEAD_DELTAS lead, preserved in condensed form. It narrated the
> `31efec44 → 862cd2cb` span — **the PHASE4-N-AO cluster** (slices S2–S14, with S1 at the baseline tip): the live
> multi-candidate fork-choice SELECT + adopt path. Ade DECIDES a fork-choice win among competing live branches, PROVES
> the replacement branch (fetch → bind → link → validate), COMMITS the adoption, and re-converges — on a NATURAL
> two-producer partition-and-reconverge venue. **This is the cluster that flipped `CN-CONS-03` `declared → enforced`**
> (Cardano post-partition convergence) on a committed, sha256-pinned natural CE-AO-6 transcript
> (`ContinuesSelectedBranch` + `AgreedAtSwitchTip{391}` + 25 descendants + 0 diverged; transcript OUTSIDE-repo per
> competition-secrecy). **42 commits, 50 files, +9797 / −98.** **GREEN+RED ONLY — ZERO new BLUE canonical type and ZERO
> BLUE-tree change** (`462 → 462`; `select_best_chain` + `validate_and_apply_header` reused byte-unchanged). **+6 new
> modules**, all GREEN/RED in `ade_node` (`candidate_aggregator`, `selector_state`, `fork_switch`, `lca_walk` GREEN;
> `fair_merge` RED; `post_switch_continuity` GREEN + bin); **NO new crate (11)**, **NO `Cargo.toml` change**, **NO new
> CLI flag**. **+11 CI gates, 6 modified, 0 removed** (162 → 173). **Registry 365 → 372** (+7 new rules `DC-NODE-38..41`,
> `DC-PUMP-04`, `DC-EVIDENCE-04/05`, all enforced; +5 declared→enforced flips `CN-CONS-03` + `DC-NODE-34..37`; +10
> strengthenings; **zero removals**; status 227/1/19/118 → 239/1/19/113). **NO `RO-LIVE` flip** (`RO-LIVE-01` stays
> operator-gated). The full §§0–7 narrative is recoverable from this doc's git history at `862cd2cb`. *(The N-AO close's
> grounding-doc regen + cluster-doc archive land in the FIRST two commits of band 1 — `388d8073`, `5532ddf4` — so
> the four grounding docs were pinned to `862cd2cb` at the start of band 1.)*

---

## Historical — PHASE4-N-AM keep-alive client + PHASE4-N-AN rollback-materialize eta0 (`e87e8a43 → b8860b16`)

> Preserved as a pointer. It narrated the `e87e8a43 → b8860b16` span: the **PHASE4-N-AL close commit** (`35a851b9`) +
> the **PHASE4-N-AM cluster** (`DC-PUMP-03` — the N2N keep-alive CLIENT, mini-protocol 8) + the **PHASE4-N-AN cluster**
> (`T-REC-06` — `materialize_rolled_back_state` overlays the recovered seed-epoch eta0 before the `block_validity` fold)
> + a stale-gate triage. **12 commits, 32 files, +2288 / −516.** **Touched BLUE but added ZERO new canonical type**
> (462 → 462; a single METHOD + a field). **NO new crate (11), NO new module, NO new CLI flag** (only a `tokio`
> `test-util` dev-dependency). **+2 CI gates** (159 → 161). **Registry 359 → 361** (+2; +1 strengthening; 0 removed).
> **NO `RO-LIVE` flip; `CN-CONS-03` NOT flipped** (single-best-peer rollback-FOLLOW scope — flipped the next window,
> PHASE4-N-AO). The full §§0–7 narrative is recoverable from this doc's git history at `b8860b16`.

---

## Historical — PHASE4-N-AL participant recovered-anchor rollback no-op (`b4c0983d → e87e8a43`)

> Preserved as a pointer. The **N-AK close commit** (`efa2a44e`) + a C2-guide remediation note + the **PHASE4-N-AL
> cluster** (single slice AL-S1, `DC-NODE-33` — the participant mirror of N-AK's `DC-NODE-32` recovered-anchor rollback
> no-op). **4 commits, 14 files, +1792 / −825.** **Did NOT touch BLUE** (462 → 462); **NO new crate/module/type/gate**
> (159 → 159). Registry **358 → 359** (+1; 0 strengthenings; 0 removals). **NO `RO-LIVE` flip.** The full §§0–7 narrative
> is recoverable from this doc's git history at `e87e8a43`.

---

## Historical — PHASE4-N-AK recovered-anchor live-follow start + rollback boundary (`b1bed361 → b4c0983d`)

> Preserved as a pointer. The **N-AJ close commit** (`bbdc3585`) + the **PHASE4-N-AK cluster** (AK-S1 + AK-S2). **7
> commits, 33 files, +2647 / −544.** **Touched BLUE — +2 canonical types** (`RecoveredAnchorPoint` +
> `RecoveredAnchorPointError`, new BLUE module `crates/ade_ledger/src/recovered_anchor_point.rs`; + a new RED module).
> `DC-NODE-31` (persist the bootstrap anchor POINT + resolve the live-follow FindIntersect start) + `DC-NODE-32`
> (single-producer `RollBackward(anchor)` idempotent no-op). Registry **356 → 358** (+2; `T-REC-05` strengthened; 0
> removed); CI **159 → 159**. **NO `RO-LIVE` flip.** The full §§0–7 narrative is recoverable from this doc's git history
> at `b4c0983d`.

---

## Historical — PHASE4-N-AJ Participant-path convergence evidence emission (`e99a86c7 → b1bed361`)

> Preserved as a pointer. The **PHASE4-N-AJ cluster** — Participant-path convergence evidence emission, the CE-AI-6
> bridge. **9 commits, 19 files, +1813 / −35.** **EVIDENCE-ONLY — ZERO BLUE change.** Added a deterministic GREEN
> evidence side-output (the EXISTING closed `AgreementVerdict` vocabulary to a `--convergence-evidence-path` JSONL sink,
> the new GREEN/RED module `ade_node::convergence_evidence`). CI **157 → 159** (+2). Registry **354 → 356** (+2;
> `DC-ADMIT-04` strengthened; **`CN-CONS-03` NOT flipped**; 0 removed). **NO `RO-LIVE` flip.** The full §§0–7 narrative
> is recoverable from this doc's git history at `b1bed361`.

---

## Historical — earlier windows (`8e2c3672 → e99a86c7` and before)

> Preserved as pointers. The **PHASE4-N-AI cluster** (live fork-choice rollback-follow wiring — single-best-peer FOLLOW,
> `DC-NODE-23..29`; +2 BLUE types `RollbackPoint`/`RollbackReason`; 347 → 354 rules / 148 → 157 CI; H-1 found + closed by
> AI-S6; `CN-CONS-03` NOT flipped); the **PHASE4-N-AG/N-AH** (single-producer loop-continuation + local-tip forge-base
> authority, `DC-NODE-19..22`; 343 → 347 rules / 143 → 148 CI; cert-free single-producer block production on C2-LOCAL);
> the **PHASE4-N-AF / N-AE.F / N-AD-N-AE CE-A5 window** (single-producer durable-spine extend + receive idempotency +
> recover→serve continuity + forge-on-followed-tip admissibility — the CE-A5 manifest: a real `cardano-node 11.0.1`
> relay `AddedToCurrentChain` an Ade-forged block; `DC-NODE-14..18`, `DC-CONS-24`, `DC-PROTO-10`); the **PHASE4-N-AC/AB/AA**
> (KES key evolution `DC-CRYPTO-10`, outbound mux segmentation `CN-SESS-05`, bounded serve range `DC-SERVEMEM-01`); the
> **PHASE4-N-U** (forged-block durability); and the **G-K…G-R + C1 multi-cluster catch-up**. The full §§0–7 narrative
> for each is recoverable from this doc's git history at the respective HEADs.

---

## Generation notes

### Regen `862cd2cb → 233644f7` (MEM-OPT-OPS cluster-close refresh — current lead)

- **Baseline valid; cluster-close refresh for MEM-OPT-OPS.** Run against `862cd2cb` (the PHASE4-N-AO cluster-close),
  which `git rev-parse` resolves and `git merge-base 862cd2cb HEAD == 862cd2cb` confirms is a strict ancestor of HEAD
  `233644f7` (`862cd2cb` carries no tag). The full span contains **three bands**: band 1 (pre-preprod, `388d8073..0887b2ad`,
  the prior lead — preserved as the "immediately-prior" section), band 2 (MEM-MEASURE + C2, `92b78ee1..51884a78`,
  summarized in the lead for continuity — it introduced `OP-MEM-01` + the `mem_measure` module), and band 3 (MEM-OPT-OPS,
  `2a790fad..233644f7`, the cluster this refresh closes — the §§0–7 lead). This regen's post-step is the baseline bump
  `862cd2cb → 233644f7`.
- **Counts are mechanical (git/grep/ls).** Commit log + `--shortstat` over `862cd2cb..HEAD` (**38** commits, no merges /
  **110** files / **+11463 / −9128**); CI gate count via `git ls-tree -r --name-only <ref> ci/ | grep -c
  ci_check_.*\.sh` = **173** at baseline, **186** at `51884a78`, **190** at HEAD (full span `--diff-filter=A` = 17 new,
  `=M` = 2 modified, `=D` empty; MEM-OPT-OPS `2a790fad^..HEAD` = 4 new + 1 modified); registry rule count via `grep -c
  '^id = '` (**372 → 378**; MEM-OPT-OPS `373 → 378`; `comm -23` of sorted `id =` lists empty in every band — zero
  removals); registry status via `grep '^status = ' | sort | uniq -c` (MEM-OPT-OPS `2a790fad^` 101/250/1/21 → HEAD
  105/250/1/22 declared/enforced/scaffolding/partial); the 5 new MEM-OPT-OPS IDs via `comm -13`
  (`DC-MEM-05/06/07/08`, `OP-MEM-02`).
- **MEM-OPT-OPS is RED/GREEN-only — ZERO BLUE (the load-bearing cluster property).** `git diff 2a790fad^..233644f7` over
  the configured BLUE `core_paths` trees is **EMPTY**; the BLUE `struct`/`enum` count over those trees is unchanged
  across band 3. The new fields are RED measurements (`/proc/self/status` RSS) or GREEN observational evidence — never
  authoritative outputs, so zero canonical type. (The full-span +2 BLUE types — `+TxSubmissionTxId`, `+RedeemerFields`,
  `462 → 464` — are entirely band 1.)
- **No new library module in MEM-OPT-OPS.** `git diff --diff-filter=A --name-only 2a790fad^..HEAD -- 'crates/**/*.rs'`
  has no `src/` library module — MEM-OPT-OPS only *modifies* existing files (`mem_measure/rss_sampler.rs` +46, `main.rs`
  +7, `seed_import/importer.rs` +303). The one new library module in the full span is `ade_node::mem_measure` (band 2,
  MEM-MEASURE-A1, RED+GREEN). No new crate / workspace anywhere — still 11 crates.
- **Manifest change is the `mimalloc` binary-only dep; no feature flag, no CLI flag.** `git diff --name-only
  2a790fad^..HEAD -- '**/Cargo.toml'` = `crates/ade_node/Cargo.toml` (`mimalloc = "0.1"`, binary-only, + `Cargo.lock`);
  no `[features]` table at any ref; 0 `cfg(feature)` and 0 `compile_error!` added in band 3; `cli.rs` untouched.
- **Registry delta is +5 rules (4 declared + 1 partial) + 1 strengthening, NOT a removal.** MEM-OPT-OPS added
  `OP-MEM-02`/`DC-MEM-05`/`DC-MEM-07`/`DC-MEM-08` (declared) + `DC-MEM-06` (`declared → partial`, `strengthened_in +=
  MEM-OPT-OPS`). No MEM-OPT-OPS rule flipped to `enforced`. The sorted-id `comm -23` confirms zero removals.
- **`OP-MEM-02` is the headline rule and it STAYS `declared` (honest no-flip).** The S3 owned (`RssAnon`) measurement
  returned verdict `ade_heavier` (Ade 4.59 GiB vs Haskell 2.57 GiB owned during admission); MEM-OPT-OPS does not clear
  the owned posture; the gating lever is the on-disk UTxO (`MEM-OPT-UTXO-DISK`). Surfaced in §0/§7/residual.
- **+4 CI gates, 1 modified, 0 removed (MEM-OPT-OPS).** New: `ci_check_alloc_determinism_neutral.sh`,
  `ci_check_mem_opt_s1_reduction.sh`, `ci_check_mem_opt_s2_import_peak.sh`, `ci_check_mem_opt_s3_owned.sh`. Modified:
  `ci_check_mem_measure_evidence.sh` (the cluster-review `*_kib` normalizer generalization).
- **Normative docs unchanged across the full span.** `git diff --name-only 862cd2cb..HEAD` over the configured
  `normative_docs` (CE-79 statement + addendum, the three contract docs, CE-73 reclassification, `CLAUDE.md`) is empty —
  the §7 delta is entirely the invariant-registry change.
- **§1 commit log verbatim from `git log` (newest first).** The per-band synthesis is in §0/§3. All 38 subjects carry a
  conventional-commits prefix (`feat`×17 / `docs`×12 / `fix`×6 / `test`×2 / `chore`×1 = 38); zero unclassified.
- **Doc-refresh state — CODEMAP + SEAMS + TRACEABILITY now TWO windows + a cluster STALE.** All three pin `862cd2cb` and
  carry everything through PHASE4-N-AO but **nothing from band 1, band 2, or MEM-OPT-OPS** — they miss `DC-PROTO-11`, the
  +2 BLUE types, the band-1 10 flips, the band-2 `mem_measure` module + `OP-MEM-01`, and the 5 MEM-OPT-OPS rules + 4
  gates (`grep -c` for `ci_check_mem_opt_s1_reduction` / `DC-MEM-06` / `mem_measure` in all three = 0; CI pin 173 vs.
  HEAD 190; rule pin 372 vs. HEAD 378). **Cross-reference warnings surfaced in §2 and §5.** Regenerate to `233644f7` as a
  follow-on; the registry holds all of it authoritatively in the interim (378 rules). No orphan gate (each new gate binds
  a registry rule).
- **Working tree CLEAN for tracked files.** This regen runs with all MEM-OPT-OPS artifacts committed
  (`git status --porcelain` = `.mithril-scratch/` scratch only). Follow-on actions: bump `.idd-config.json`
  `head_deltas_baseline` `862cd2cb → 233644f7`, and refresh CODEMAP + SEAMS + TRACEABILITY to `233644f7`.

### Regen `862cd2cb → 0887b2ad` (pre-preprod local-first enforcement-mapping window — band 1, now the "immediately-prior" section)

- **Not a cluster — a local-first enforcement-mapping pass** scoped by `86252176`, ordered strictly 3→1→2. **18 commits,
  51 files, +4182 / −6652.** CI **173 → 183** (+10 new, 0 modified, 0 removed); registry **372 → 373** (+1 rule
  `DC-PROTO-11`, 10 declared→enforced flips, +1 strengthening `DC-PROTO-02`, zero removals; status 239/1/19/113 →
  250/1/19/103); BLUE canonical types **462 → 464** (+`TxSubmissionTxId`, +`RedeemerFields`). One new RED capture bin
  (`ade_tx_submission2_server_capture`) + a real-capture corpus; no new crate (11); no `[features]`, no `cfg(feature)`,
  no `compile_error!`, no new CLI flag. The headline flips are `DC-PROTO-02` (the last wire surface) + the six Stream 3
  flips + the two Plutus flips. The full §§0–7 narrative is recoverable from this doc's git history at `0887b2ad`.
