# Ade — HEAD Deltas (Changes Since Baseline)

> **Status:** Living architectural document. Regenerated; not hand-edited.
>
> Regenerate with `/head-deltas <baseline>` after every cluster close. Baseline is recorded in `.idd-config.json` `head_deltas_baseline`.

> Baseline: `470f9b89` (MEM-OPT-UTXO-DISK close — `OP-MEM-02` flipped `declared → enforced` SCOPED, 2026-06-17 00:58)
> HEAD: `1e4896eb` (`chore(node): LIVE-FORGE-HARDENING cluster-close review nits`, 2026-07-31 17:25)
> Span: **`470f9b89 → 1e4896eb`** — **232 commits** (no merges), **387 files changed, +74,026 / −5,752 lines**.

> **THE HEADLINE (read first).** This is a **cluster-close refresh** and the baseline **advances** to `1e4896eb`
> (`LIVE-FORGE-HARDENING` close). Over ~6 weeks the node went from *"fails closed off its single bootstrapped seed
> epoch"* to **literal continuous self-sufficient multi-boundary operation**: `CE-4B` (`c5bdc064`) crossed
> **three real preview boundaries 1340→1341→1342→1343** (seed+2 → seed+5) in one ~2.9h run with **no halt, no manual
> arming, no re-import**, and `CE-4` was declared a milestone (`bcbae327`). Getting there required (a) the epoch-view
> **activation gate removal** (`a17c7aab`, ECA-1 — `EVIEW_ACTIVATION_ARMED` is gone; activation is automatic and
> deterministic from durable state); (b) a **rolling Praos-nonce reshape inside the consensus core** (`ade_core`,
> ECA-B1/B2) so `eta0(seed+2)` is self-derived on the live follow path and byte-matches `cardano-node`; (c) a durable
> **non-UTxO `EpochAccumulator`** (`DC-EPOCH-19`/`20`/`21`/`22`) that self-evolves the ledger across boundaries; (d) a
> **sealed authority flip** (`S4`, `db702a54`) so promotion reads epoch-indexed *frozen* leadership only, never the seed
> window; and (e) **restart + rollback + warm-restart-after-rollback replay-equivalence** (`S5`, `CE-4A.3-R1..R4`),
> byte-identical through the production loop. The just-closed **`LIVE-FORGE-HARDENING`** (`b52f2240` S1 + `dc14787a`
> S2 + `1e4896eb` nits) hardens the forge path: it now follows live rollbacks (S1) and makes the **durable store the
> sole candidate-freeze authority** via the v5→v6 seed sidecar (S2, `DC-EPOCH-16` strengthened).

> **CORRECTION to the prior (`cdcd9397`) framing — `ade_core` is NO LONGER byte-identical.** The on-disk doc this
> replaces was regenerated at `cdcd9397` (only **75 of these 232 commits** in) and asserted *"`ade_core` untouched
> (48→48) — the consensus authority is byte-identical."* **That is stale.** As of HEAD, `ade_core` has changed
> **+574 / −300 across 16 files (+1 BLUE canonical type, 48→49)** — the **ECA-B1/B2 rolling Praos-nonce reshape**
> (`nonce.rs` reshaped `NonceInput`/`HeaderContribution`/`EpochBoundary`, `CandidateFreeze` removed,
> `MissingLastEpochBlockNonce` added; `praos_state.rs` `last_epoch_block_nonce: Option<Nonce>`; `header_validate.rs`
> Step-9 threading; `era_schedule.rs` `praos_rsw_slots`). This is a **versioned, backward-compatible** reshape
> (always-write `array(10)`, accept legacy `array(9) → None`, fail-closed on an absent operand — never a fabricated
> nonce), **not** a semantic weakening — but the consensus core **was** modified. Any doc still claiming the core is
> byte-identical is describing `cdcd9397`, not HEAD.

> **Baseline note (load-bearing).** Baseline is **`470f9b89`** (MEM-OPT-UTXO-DISK cluster-close; `OP-MEM-02 → enforced`
> SCOPED; pushed to `origin/main`, 2026-06-17) and is **valid**: `git rev-parse 470f9b89` resolves and
> `git merge-base 470f9b89 1e4896eb == 470f9b89` (a strict ancestor; `470f9b89` carries no tag). HEAD is
> **`1e4896eb`** (`origin/main`, the `LIVE-FORGE-HARDENING` close). **This IS a cluster-close refresh** — so per IDD
> discipline the baseline **advances** to `1e4896eb` (the caller updates `.idd-config.json` `head_deltas_baseline`
> separately). The prior on-disk doc stopped at `cdcd9397` (mid native-Mithril band, ~157 commits short of HEAD); this
> regen supersedes it across nine additional cluster arcs.

> **Working-tree note.** At this regen the working-tree `HEAD` is `1e4896eb` (== `origin/main`), with uncommitted
> in-flight `EPOCH-CONSENSUS-VIEW` / next-cluster scratch present (`docs/active/*.md` runbooks, two untracked
> cluster-slice dirs, `wire_smoke.jsonl`, a modified live-pass guide). §1 narrates the **committed** span
> `470f9b89..1e4896eb` verbatim from `git log`; all counts read the tree at HEAD `1e4896eb`. **The other three
> grounding docs (CODEMAP / SEAMS / TRACEABILITY) are on-disk dated 2026-06-24 and were last regenerated at
> `cdcd9397`** — they are **STALE** relative to HEAD and do not contain the bands-5→13 modules/rules/gates. Run
> `/codemap`, `/seams`, `/traceability` to re-align them (see the Anomalies block).

---

# The span in thirteen bands (`470f9b89 → 1e4896eb`)

Reading oldest→newest. The first four bands (`470f9b89..cdcd9397`, 75 commits) were narrated by the prior on-disk doc
and are compressed here; bands 5–13 (`cdcd9397..1e4896eb`, 157 commits) are the new material. All counts are exact
(`git rev-list --count` per boundary; they sum to 232).

| Band | Range | Commits | Cluster / theme | Headline rule(s) |
|---|---|---|---|---|
| **1. Standalone fixes + Mithril evidence** | `1b79add0 … cf508424` | 16 | C2-preprod / live-follow hardening + participant-forge + Mithril documented-interface evidence | `DC-WAL-05`, `DC-CINPUT-05`, `DC-MEM-11`, `CN-FOLLOW-01`, `DC-FOLLOW-FORGE-01`; `RO-MITHRIL-IMPORT-01`/`T-CONS-01` flips |
| **2. EPOCH-CONSENSUS-VIEW (EVIEW substrate)** | `84e1019c … a50a3ee8` | 41 | The native cross-epoch stake/consensus view — hermetic substrate; shipped **INERT (gated off)** | `DC-EVIEW-01..11` (+`04b`), `DC-EPOCH-04..11` |
| **3. EPOCH-CONTINUITY-ACTIVATION (gate removal)** | `ad704f86 … f09cc0ec` | 7 | **Removes the `EVIEW_ACTIVATION_ARMED` gate** → activation automatic; leadership-complete view; v4 sidecar | **`DC-EPOCH-13`**, `DC-EVIEW-12/13`, `DC-CINPUT-06`, `DC-EPOCH-12/14` |
| **4. Native Mithril bootstrap decode** | `7386bf82 … cdcd9397` | 11 | Native V2 LedgerDB decode → canonical CertState + faithful UTxO → materialize → FirstRun | `DC-MITHRIL-03..07`, `DC-LEDGER-VALUE-01`, `DC-LEDGER-PARAMS-01` |
| **5. Grounding regen + registry reconcile** | `5333d0b6 … 25d11636` | 4 | The `cdcd9397` grounding-doc regen + registry status backfill / DC-EVIEW-08 re-scope (housekeeping) | — |
| **6. Native operator/judge startup (LIVE-1)** | `6e04f1fc … 25a6bde3` | 11 | The two-command judge flow (`ade mithril snapshot fetch` + `ade node run`); native FirstRun builds the EVIEW checkpoint inline; getting-started guide | `DC-MITHRIL-08` |
| **7. ECA-5 + ECA-B (cross-boundary continuity)** | `5599f297 … dabb4210` | 19 | Cross the FIRST boundary; **rolling Praos nonce in `ade_core`**; live RSW candidate-freeze; per-boundary authority advance; warm-start recovery across a boundary | `DC-EPOCH-15`, **`DC-EPOCH-16`**, `DC-EPOCH-17` |
| **8. LIVE-LEDGER-EPOCH-TRANSITION S1–S3** | `c4e0413b … aeeaf89d` | 29 | The **non-UTxO `EpochAccumulator`** + `apply_selected_block`; durable within-epoch advance; byte-exact boundary mark + POOLREAP; CE-3c/CE-3d live | `DC-EPOCH-18/19/20/21/22` |
| **9. Conway governance (CPDE + CRE)** | `d2522faf … 710f23db` | 33 | `CONWAY-PROPOSAL-DEPOSIT-EXPIRY` (close the −500B gap) + `CONWAY-RATIFICATION-AND-ENACTMENT-AUTHORITY` (import → capture votes → activate ratify/enact gate) | `DC-GOV-01`, `DC-CINPUT-07` |
| **10. CE-3d closure (B3c / go-stake / RVBP)** | `52a6e2c7 … e476415a` | 15 | Base-UTxO byte-exact; −343B go-stake localized; the reduced-validation boundary plane; fee-pot deltaF + snapshot pool-set — CE-3d byte-exact | `DC-EPOCH-23/24/25` |
| **11. LIVE-LEDGER S4/S5 + CE-4A/CE-4B** | `e096e014 … 5e83aaaa` | 38 | Restart/rollback replay-equivalence (S5); **sealed authority flip** (S4); **two-** then **three-boundary continuous operation** (CE-4A/CE-4B); warm-restart-after-rollback recovery (R4) | `DC-EPOCH-16` (enforced live) + S4/S5 rules |
| **12. LIVE ops (LIVE-1b / LIVE-2)** | `fde0dd9e … 0ef65c6c` | 3 | Bounded recovery-checkpoint retention (disk-fill fix); LIVE-2 forge-machinery + KES/opcert validity-window verification | — |
| **13. LIVE-FORGE-HARDENING (the close)** | `2f12bb0b … 1e4896eb` | 5 | **S1** forge path follows live rollbacks; **S2** durable store is the sole candidate-freeze authority (v5→v6 sidecar persists `k`); nits | **`DC-EPOCH-16`** strengthened (`+LIVE-FORGE-HARDENING-S2`) |

---

## Bands 1–4 — the EVIEW → ECA → native-Mithril arc (`470f9b89 … cdcd9397`, 75 commits) — *previously documented, compressed*

These four bands were fully narrated in the prior on-disk doc. In brief:

- **Band 1** landed the standalone C2-preprod / live-follow fixes (persist admitted bytes before the WAL `DC-WAL-05`;
  warm-start era-schedule from durable venue geometry `DC-CINPUT-05`; the LIVE-FOLLOW-THROUGHPUT cached-UTxO-fingerprint
  fix `DC-MEM-11`; participant forge on the AO-selected durable head `CN-FOLLOW-01` / `DC-FOLLOW-FORGE-01`) and the
  Mithril documented-interface evidence gate (`RO-MITHRIL-IMPORT-01 partial→enforced`, `T-CONS-01 declared→enforced`).
- **Band 2 (EVIEW)** built the native cross-epoch consensus view as a **pure projection of the single ledger authority**
  (`stake_ref` → `pointer_resolve` → `reduced_utxo`/`reduced_advance` → `reduced_aggregate` → `reduced_snapshot` →
  `reduced_epoch_view`), plus the durable/transient checkpoint storage and the activation substrate — **shipped INERT**
  behind `EVIEW_ACTIVATION_ARMED = false`.
- **Band 3 (ECA)** **removed** that gate (`a17c7aab`, `DC-EPOCH-13`): epoch-view activation became automatic and
  deterministic from canonical durable state, with a leadership-complete view (`DC-EVIEW-12`), cardano-faithful pool
  lifecycle (`DC-EVIEW-13`), and the v4 seed sidecar (`DC-CINPUT-06`).
- **Band 4 (native Mithril decode)** added the native V2 LedgerDB decoders (state → `CertState`; tables MemPack `TxOut`
  → UTxO) and the assemble/materialize/FirstRun path (`DC-MITHRIL-03..07`), plus the Word64 value domain
  (`DC-LEDGER-VALUE-01`) and era-aware min-UTxO rule (`DC-LEDGER-PARAMS-01`).

The gate-removal (band 3) remains true at HEAD: `git grep EVIEW_ACTIVATION_ARMED 1e4896eb -- crates/` is **empty**.

## Band 5 — grounding regen + registry reconcile (`5333d0b6 … 25d11636`, 4 commits)

Housekeeping around the prior refresh: the four grounding docs were regenerated at `cdcd9397` (`a24d0c39` — this is the
commit that produced the now-stale on-disk CODEMAP/SEAMS/TRACEABILITY dated 2026-06-24), `DC-EPOCH-14`/`DC-MITHRIL-04`
status was backfilled, a deleted test ref was dropped, and `DC-EVIEW-08` was re-scoped to the ECA window-replay
architecture. No source behavior change.

## Band 6 — native operator/judge startup, LIVE-1 (`6e04f1fc … 25a6bde3`, 11 commits)

The operator-facing **two-command judge flow**: `ade mithril snapshot fetch` (native acquisition + manifest,
`ade_node::mithril_fetch`, S4) then `ade node run` (the native bootstrap + warm-start entrypoint closed to legacy
inputs, S3). Native FirstRun now **resolves genesis from the committed `--network` profile** (S2 Gap 1a, manifest-bound)
and **builds the EVIEW reduced checkpoint inline** at bootstrap (**`DC-MITHRIL-08`**, S2 Gap 2). Adds native
operational continuity (warm-start snapshot + in-memory seed inputs, S5), relay-only forge-OFF follow (S6), a fix to the
leader-VRF `eta0` PraosState nonce-slot read (S7), the snapshot-fetch symlink-layout fix (S8), and the
**getting-started guide** for running Ade on preview.

## Band 7 — ECA-5 + ECA-B, cross-boundary continuity (`5599f297 … dabb4210`, 19 commits) — **the `ade_core` reshape**

The band that first **crosses a real epoch boundary** and rewrites how the Praos nonce evolves on the live path.

- **ECA-5 (`08fa37f6`, `26565bec`; `DC-EPOCH-15`, enforced):** cross the boundary — the forecast horizon extends with
  N+1 authority promotion; the native-Mithril first-boundary bridge survives seed→seed+1.
- **ECA-B1 (`79467c84`; `DC-EPOCH-16`):** the **rolling Praos nonce on the follow path** — folds the live per-header
  update into ONE `HeaderContribution` and **retires the dead `CandidateFreeze` split** inside `ade_core::consensus`
  (`nonce.rs`, `praos_state.rs`, `header_validate.rs`). Backward-compatible chain-dep codec (`array(10)`, legacy
  `array(9) → None`); an explicit no-`last_epoch_block_nonce` form that **fails closed** (`MissingLastEpochBlockNonce`)
  — never a fabricated nonce.
- **ECA-B2 (`9040615b`, `14880463`, `e8589e1e`; `DC-EPOCH-16 declared→enforced` at `44e07782`):** live **RSW
  candidate-freeze** (`ceil(4k/f)` from the verified venue era-geometry via the single BLUE `praos_rsw_slots`) + the
  boundary tick on the follow path; B2c seeds the evolving nonce from the full 6-nonce PraosState. `eta0(seed+2)` is now
  self-derived live and byte-matches `cardano-node epochNonce(seed+2)`.
- **ECA-B3 (`b058ff1c`, `23829091`, `b1d0fc7b`, `c13d4414`; `DC-EPOCH-17`, declared):** generalize the activation seam
  to **advance per boundary** — `ActiveEpochAuthority.advance`, `run_node_sync` yields a `SyncOutcome` so the checkpoint
  advances per-boundary, a lag-aware activation predicate crosses boundary 2.
- Plus a flipped-Credential-tag fix in the native-bootstrap decode (`84fec1b5`, `DC-LEDGER-10`), observable follow
  progress (`node.log`, `ade_node::ops_log`), and **warm-start recovery across a crossed boundary** (`dabb4210`).

> **This is where `ade_core` changed** (see the CORRECTION note in the header). The reshape is versioned and
> fail-closed, but the consensus authority is no longer byte-identical to baseline.

## Band 8 — LIVE-LEDGER-EPOCH-TRANSITION S1–S3, the boundary accumulator (`c4e0413b … aeeaf89d`, 29 commits)

The cluster (`DC-EPOCH-19`) that makes the live-followed ledger **self-evolve its non-UTxO state across boundaries**
instead of fail-closing off the seed epoch.

- **S1 (`5d16eaef`; `DC-EPOCH-19`, declared):** the non-UTxO **`EpochAccumulator`** + the `apply_selected_block`
  contract — the self-sustaining ledger loop (`ade_ledger::epoch_accumulator`).
- **S2 (`b2185be6 … 7c7b3a30`; `DC-EPOCH-20`, declared):** the durable **`EpochAccumulatorStore`** (single-blob home),
  the within-epoch advancer (observe-only, stalls on a boundary), seal-at-firstrun, advance-on-live-follow,
  advance-to-tip reconciliation (warm-start catch-up + reorg rematerialize), validity-aware within-epoch fees
  (invalid-tx collateral, not declared fee), and the S2 RECOVER warm-start survival proof.
- **S3 (`f41456da … b8e33ff0`; `DC-EPOCH-21`/`22`, declared):** the byte-exact boundary gate — one canonical POOLREAP,
  a **per-credential** boundary mark (byte-exact member + leader rewards), the accumulator boundary-cross entry point,
  the durable `BoundaryMark` witness (point + lineage bound before the cross), and the boundary-aligned co-advancer.
  CE-3c proven live (the accumulator crosses two real preview boundaries). Seeds the accumulator mark/set/go from the
  certified snapshot; derives monetary-expansion `eta` from the network's real epoch length (CE-3d).

## Band 9 — Conway governance: CPDE + CRE (`d2522faf … 710f23db`, 33 commits)

Two governance clusters that close the residual reward gap and stand up the ratification/enactment authority.

- **`CONWAY-PROPOSAL-DEPOSIT-EXPIRY` (`d2522faf … 84286d95`):** import all post-seed boundary inputs (fee pot, RUPD, gov
  proposals) at bootstrap; reject a pre-import accumulator at warm-start (absent ≠ empty); capture live gov proposals +
  a vote tripwire + imported expiry-lifetime authority; the S4.0 ratification census (committee-only authority resolves
  the whole tracked set); the **boundary deposit-expiry-refund evaluator** that closes the **−500B CE-3d gap**
  (`6934afb4`), proven on the real proposals (`84286d95`).
- **`CONWAY-RATIFICATION-AND-ENACTMENT-AUTHORITY` (`406888ab … 710f23db`):** import + commitment-bind per-action voting
  thresholds (S1p1), the bootstrap DRep vote-delegation baseline from the DState UMap (S1p2a) and DRep-expiry +
  committee-hot-key baseline from the VState (S1p2b); capture live votes into tracked proposals' vote maps replacing the
  tripwire (S2); the enactment-census decoders (`prevPParams`, enacted-authority `PParamUpdate` root,
  `maxBlockExUnits.mem`, deposit pot) bound to selected-chain ImmutableDB witnesses (`ade_testkit::immutabledb_witness`);
  the DRep voting-stake derivation (S3); version `ConwayGovState` with authoritative `num_dormant` (no fabricated
  default, S4.1); **activate the DRep/committee ratification gate on the live boundary** (`262415bd`, S4.2); one
  governance authority for the Conway epoch boundary + correct the expired-deposit drop (S4.3a); atomic exec-units
  parameter-change enactment (S4.3c); the S5 differential proof + provenance-bound residual report. New rules
  `DC-GOV-01` (declared), `DC-CINPUT-07` (declared, Conway deposit params bootstrap).

## Band 10 — CE-3d closure: B3c / go-stake / RVBP (`52a6e2c7 … e476415a`, 15 commits)

Drives CE-3d to **byte-exact**. B3c.0 proves the base UTxO byte-exact and adjudicates the −343B go-stake residual REAL;
the residual is localized to the reward-account contribution (`a882d304`) and root-caused to a pre-RUPD mark snapshot,
fixed by a staged **post-RUPD mark** built from a point-bound base (CE3D-REWARD-ACCOUNT-EVOLUTION-CORRECTION S1). The
**REDUCED-VALIDATION-BOUNDARY-PLANE** (RVBP) introduces the capability-typed split — reduced-boundary projection types +
`FullBoundaryStateRequired` (`ade_ledger::reduced_boundary`), reduced-plane typed non-authority on the live path
(P1/P2 + B1), and the recovery/fork-switch/boundary proof (P3). Closes the reward/pots residual by reducing the
bootstrap fee pot by the RUPD `feeSS` (deltaF, **`DC-EPOCH-23`**) and fixing the snapshot pool-set to cardano
`ssActiveStake` NonZero membership (**`DC-EPOCH-24`**); rejects a pre-C accumulator store (schema v3→v4). CE-3d final
green declaration flips `DC-EPOCH-23`/`DC-EPOCH-24` to **enforced** (`e476415a`). (`DC-EPOCH-25` — the reduced-plane /
trusted-replay boundary rule — lands **declared**.)

## Band 11 — LIVE-LEDGER S4/S5 + CE-4A/CE-4B, continuous self-sufficiency (`e096e014 … 5e83aaaa`, 38 commits) — **the milestone**

The largest band. It makes restart/rollback replay-equivalent, seals the leadership authority, and proves literal
multi-boundary continuous operation.

- **S5 — restart/rollback replay-equivalence (`e096e014 … 687fea98`):** a BLUE k-bounded + lineage-checked
  **rollback-admission guard** (`ade_ledger::rollback::admission`, step 1); persist the accumulator lineage anchor
  (`LastAdvancedPoint`, step 2a); the BLUE recovery-reconcile decision (step 2b p1) wired as event-qualified recovery
  admission into the runtime (crash-safe live-rollback pre-clear, step 2b p2); and the **recovery replay-equivalence
  positive proof** — byte-identical reset+refold vs an uninterrupted run (`687fea98`, S5 2c).
- **S4 — the sealed authority flip (`1c45479b … db702a54`):** `PoolDistrView::from_accumulator`; the Leadership
  Distribution Authority Trace (leadership = SET stake + snapshot-frozen params VRF); the self-contained
  **`FrozenLeadershipPoolDistr`** (`ade_ledger::frozen_leadership`, seed identity + durable codec/store schema + native
  boundary-freeze proven vs reference `nesPd`); epoch-index the frozen leadership behind a sole exact-epoch read (S4-0);
  **S4-L1** retire seed-window authority from the initial/warm leadership view (`e9de61e7`); **S4-L2** the sealed
  authority flip — promotion reads epoch-indexed frozen leadership only (`db702a54`).
- **CE-4A — mechanical continuous self-sufficiency (`8b5d209e … 5e83aaaa`):** CE-4A.1 production-loop continuous
  self-sufficiency across **two real boundaries** 1340→1342, `eta0(1341)` byte-exact (`9c6fc3c4`); CE-4A.2 the
  self-derived boundary outputs **byte-match cardano** at 1341 and 1342 across 6 hard surfaces (`af3dc9c7`); CE-4A.3
  restart+rollback replay-equivalence through the production loop — R1 warm-start reconstructs the frozen-promoted epoch
  authority (`7266f90c`), R2/R3 controlled durable rollback + rollback-aware EVIEW resolution byte-identical
  (`fd3826fd`), **R4 warm-restart crash-window recovery after rollback**, byte-identical (`5e83aaaa`, R4a+R4b+R4c —
  root cause: `warm_start_recovery` materialized the schedule with RSW=None → candidate over-tracked past the freeze
  slot → wrong `eta0`; fixed by threading the venue RSW into recovery).
- **CE-4B (`c5bdc064`) + CE-4 milestone (`bcbae327`):** literal **three-boundary continuous operation 1340→1343**
  (N→N+1→N+2→N+3), self-sufficient, ~2.9h, frozen leadership 1341–1344 sealed, no halt.

## Band 12 — LIVE ops, LIVE-1b / LIVE-2 (`fde0dd9e … 0ef65c6c`, 3 commits)

`LIVE-1b`: **bounded recovery-checkpoint retention** fixes a `chain.db` disk-fill (`fde0dd9e`). `LIVE-2`: verify the
forge machinery on the current binary + record the live-forge procedure and the verified KES/opcert validity window
(docs).

## Band 13 — LIVE-FORGE-HARDENING, the close (`2f12bb0b … 1e4896eb`, 5 commits) — **the reason for this regen**

The just-closed cluster (`HEAD`). Two slices + nits:

- **S1 (`b52f2240`) — forge-path rollback-follow:** the forge path now **follows live rollbacks**, reusing the existing
  BLUE rollback machinery (no new BLUE transition). RED shell only (`node_lifecycle.rs`, `node_sync.rs`).
- **S2 (`dc14787a`) — the durable store is the sole candidate-freeze authority (`DC-EPOCH-16` strengthened):** the seed
  sidecar schema advances **v5→v6** (`FIELDS_OUTER 13→14`) to persist `security_param` (`k`). A single
  `sidecar_freeze_rsw` helper derives the candidate-freeze `RSW = ceil(4k/f)` **from the durable store** (via the same
  BLUE `praos_rsw_slots`) for **both** `warm_start_recovery` **and** the forward live-loop schedule, so an absent /
  unsupported restart `--network` can no longer leave the candidate freeze INERT; the restart-CLI RSW is retained **only
  as a fail-closed cross-check** (mismatch → terminal). The importer **requires** `k` (fail-closed `MissingField`, no
  fabricated default). This appended `strengthened_in += "LIVE-FORGE-HARDENING-S2"` and **two tests** to `DC-EPOCH-16`
  (append-only; the rule stays `enforced`).
- **Nits (`1e4896eb`) = HEAD:** an importer **ingress guard rejecting a degenerate active-slot coefficient**
  (`active_slots_coeff.numer == 0`, i.e. `f = 0` → undefined freeze window; symmetric with the existing `denom == 0`
  guard) + a one-line `node_lifecycle.rs` doc-rot fix. `importer.rs +26`, `node_lifecycle.rs ±2`.

> **BLUE-touch precision (do not over-read "no BLUE edit").** `LIVE-FORGE-HARDENING` leaves the **authoritative
> consensus core `ade_core` untouched** and adds **no new ledger/consensus transition**. Its only edit under a BLUE
> `core_path` prefix is the **additive, versioned seed-sidecar codec** (`ade_ledger::seed_consensus_inputs`, +116, the
> v5→v6 `k` persistence — a durable-input schema, append-only) plus a **1-line** `ade_ledger::consensus_view.rs` touch.
> Everything else (`node_lifecycle`, `node_sync`, `consensus_inputs::importer`, `mithril_native_assembly`,
> `seed_consensus_merge`, …) is the RED `ade_node`/`ade_runtime` shell.

---

## 0. Headline (full span `470f9b89 → 1e4896eb`)

| Count | Baseline (`470f9b89`) | HEAD (`1e4896eb`) | Δ (full span) |
|---|---|---|---|
| **Continuous operation** | fail-closes off the seed epoch | **3 real boundaries crossed live, self-sufficient** (`CE-4B` 1340→1343) | The span's reason for being. Restart + rollback + warm-restart-after-rollback all byte-identical through the production loop (`S5`, `CE-4A.3-R1..R4`). |
| **Epoch-view activation** | — | **AUTOMATIC (no semantic gate)** | `EVIEW_ACTIVATION_ARMED` removed (`a17c7aab`, ECA-1); **ZERO at HEAD** (`git grep` empty in `crates/`). Enforced by `DC-EPOCH-13`. |
| **`ade_core` (consensus authority)** | `48` BLUE types, byte-identical | **`49` BLUE types, +574 / −300 over 16 files** | **CHANGED** — the ECA-B1/B2 rolling Praos-nonce reshape (versioned, backward-compatible, fail-closed). **Supersedes the prior `48→48` claim.** |
| Crates (workspace members) | 12 | **12** | **No delta.** No new `Cargo.toml`; member list byte-identical (`ade_mem_diag`/`ade_core_interop` already present at baseline). |
| CI gates (`ci/ci_check_*.sh`) | 200 | **255** | **+55 new / 2 modified / 0 removed.** (`ci_check_credential_discriminant_closed.sh`, `ci_check_warmstart_eta0_overlay.sh` modified.) One non-gate helper also added (`ci/capture_mithril_documented_evidence.sh`); one modified (`ci/build_consensus_inputs_bundle.sh`). |
| Registry rules (`docs/ade-invariant-registry.toml`) | 380 | **432** | **+52, ZERO removed** (`comm -23` of sorted `id =` lists empty; `comm -13` = exactly 52). |
| Registry status (enforced / scaffolding / partial / declared) | 253 / 1 / 23 / 103 | **297 / 1 / 23 / 111** | enforced **+44**, scaffolding 0, partial **0**, declared **+8**. The 52 new IDs land **42 enforced / 10 declared**; existing-rule flips (`RO-MITHRIL-IMPORT-01`, `T-CONS-01`, `DC-EPOCH-16` → enforced) net the tally. |
| BLUE canonical types (approx, over BLUE `core_paths`) | ~466 | **~558** | **≈ +92** — `ade_ledger 185 → 272` (+87), `ade_codec 11 → 13` (+2), `ade_types 82 → 84` (+2), **`ade_core 48 → 49` (+1)**; `ade_crypto 22`/`ade_plutus 8`/`ade_network` BLUE sub-paths (~110) unchanged. Approximate (structural `pub struct`/`pub enum` grep). |
| Tests (`#[test]` / `#[tokio::test]` attrs, approx) | ~2,666 | **~3,320** | **≈ +654** — the EVIEW/ECA/accumulator hermetic suites, the LedgerDB-decode corpus/oracle suites, the governance (CPDE/CRE) census suites, the CE-3d differential suites, the kill-harness/memory tests. |
| New source modules (`.rs` under `crates/**/src/`) | — | **+38** | 19 BLUE (`ade_ledger` ×18 + `ade_codec` ×1), 18 RED (`ade_node` ×8 + `ade_runtime` ×10), 1 testkit harness. Plus **25 new test files**. No new crate. |
| Grounding docs (CODEMAP / SEAMS / TRACEABILITY) | regenerated at MEM-OPT-UTXO-DISK content-HEAD | **STALE at `cdcd9397` (2026-06-24)** | Only this HEAD_DELTAS is regenerated to HEAD. The other three are **~157 commits behind** — see Anomalies. |

## 1. Commit Log (newest first, full span `470f9b89..1e4896eb`)

Verbatim from `git log --oneline --no-merges 470f9b89..1e4896eb`. **232 commits, no merges.** Type is the
conventional-commits prefix (or a non-standard-but-unambiguous prefix, marked `*(…)*`). Prefix tally (exact):
**`feat`×122, `docs`×55, `fix`×22, `test`×22, `chore`×6**, plus **1 each** of `refactor`, `perf`, and the non-standard
`harden` / `registry` / `evidence`. **All 232 carry a clear scope; none is unclassifiable.**

| Hash | Type | Summary |
|------|------|---------|
| `1e4896eb` | chore | chore(node): LIVE-FORGE-HARDENING cluster-close review nits |
| `dc14787a` | feat | feat(node): S2 the durable store is the sole candidate-freeze authority (DC-EPOCH-16) |
| `240cfab3` | docs | docs(live-forge): S2 slice doc -- warm-start candidate-nonce identity (DC-EPOCH-16) |
| `b52f2240` | feat | feat(node): LIVE-FORGE-HARDENING S1 -- the forge path follows live rollbacks |
| `2f12bb0b` | docs | docs(live-forge): open LIVE-FORGE-HARDENING -- cluster + S1 (forge-path rollback-follow) |
| `0ef65c6c` | docs | docs(live): LIVE-2 -- record verified KES/opcert validity window |
| `f08191a7` | docs | docs(live): LIVE-2 -- verify forge machinery on the current binary + live-forge procedure |
| `fde0dd9e` | fix | fix(node): LIVE-1b -- bounded recovery-checkpoint retention (chain.db disk-fill) |
| `5e83aaaa` | feat | feat(node): CE-4A.3-R4 -- warm-restart crash-window recovery after rollback (R4a+R4b+R4c, byte-identical) |
| `bcbae327` | docs | docs(ledger): CE-4 milestone declaration + registry evidence (literal three-boundary continuous operation) |
| `c5bdc064` | feat | feat(ledger): CE-4B -- three-boundary continuous operation 1340->1343 self-sufficient (N->N+1->N+2->N+3) |
| `8a085b10` | docs | docs(ledger): open CE-4B -- literal three-boundary continuous-operation proof (N->N+1->N+2->N+3) |
| `4bc49fa6` | docs | docs(ledger): CE-4A.3-R4 findings -- R4a/R4b fixed+validated, R4c (VRF/nonce reconstruction) OPEN; impl parked |
| `5858bf00` | docs | docs(ledger): file CE-4A.3-R4 -- warm-restart-after-rollback-before-refold hardening (not a #13 blocker) |
| `fd3826fd` | feat | feat(epoch): CE-4A.3-R3 rollback-aware eview resolution + #13 rollback/refold byte-identical |
| `849bc9f0` | docs | docs(ledger): open CE-4A.3-R3 -- rollback-aware eview activation resolution (the #13 blocker) |
| `58b87dbb` | docs | docs(ledger): CE-4A.3-R2 (#13) ratified mechanism -- controlled durable rollback (option a) |
| `ceb390e7` | docs | docs(ledger): open CE-4A.3-R2 (#13) -- rollback + refold replay-equivalence through the production loop (scoped) |
| `7266f90c` | feat | feat(epoch): CE-4A.3-R1 -- warm-start recovery reconstructs the frozen-promoted epoch authority |
| `ca1ae06b` | docs | docs(ledger): open CE-4A.3-R1 — warm-start recovery reconstructs the frozen-promoted epoch authority (scoped) |
| `7f4aa463` | docs | docs(ledger): open CE-4A.3 — restart + rollback replay-equivalence through the production loop (scoped) |
| `af3dc9c7` | test | test(node): CE-4A.2 — self-derived boundary outputs byte-match cardano at 1341 and 1342 (6 hard surfaces) |
| `22903e8d` | docs | docs(ledger): open CE-4A.2 — boundary outputs byte-match cardano at both self-derived boundaries (scoped) |
| `9c6fc3c4` | test | test(node): CE-4A.1 — production-loop continuous self-sufficiency across two real boundaries |
| `5c04eefb` | docs | docs(ledger): CE-4A.1 fail-loud + machine-readable evidence bundle (spec refinement) |
| `8b5d209e` | docs | docs(ledger): open CE-4A -- mechanical continuous self-sufficiency (two boundaries via the production run-loop) |
| `db702a54` | feat | feat(ledger): S4-L2 sealed authority flip -- promotion reads epoch-indexed frozen leadership only (LIVE-LEDGER-EPOCH-TRANSITION S4-L2) |
| `e9de61e7` | feat | feat(node): S4-L1 — retire seed-window authority from the initial/warm leadership view |
| `7158ddc2` | docs | docs(ledger): open S4 — the sealed authority flip (epoch-indexed frozen leadership → sole production leader schedule) |
| `c7e1c18f` | feat | feat(ledger): epoch-index the frozen leadership authority behind a sole exact-epoch read (S4-0) |
| `8cdd1471` | feat | feat(ledger): native boundary leadership freeze proven vs reference nesPd (S4-pre-2) |
| `3f93252d` | feat | feat(node): certify the frozen leadership bootstrap lineage (S4-pre-1c) |
| `13829660` | feat | feat(ledger): persist the frozen leadership authority — canonical codec + durable store schema (S4-pre-1b) |
| `501bf89a` | feat | feat(ledger): FrozenLeadershipPoolDistr — the self-contained leadership authority + seed identity (S4-pre-1a) |
| `952a03b6` | docs | docs(ledger): open S4-pre — Frozen Leadership Distribution Authority (self-contained leadership PoolDistr) |
| `67890681` | feat | feat(ledger): Leadership Distribution Authority Trace — leadership = SET stake + snapshot-frozen params VRF (S4 discovered-proof-failure) |
| `ae30fe18` | docs | docs(ledger): open the Leadership Distribution Authority Trace slice (S4 discovered-proof-failure follow-up) |
| `d37af69a` | feat | feat(ledger): PoolDistrView::from_accumulator — the accumulator-derived leadership authority (S4 step 1, no behavior change) |
| `1c45479b` | docs | docs(ledger): open S4 — the sealed authority flip (accumulator-derived PoolDistrView replaces the seed-window read) |
| `687fea98` | test | test(ledger): S5 2c — recovery replay-equivalence positive proof (byte-identical reset+refold vs uninterrupted) |
| `8d6bf874` | feat | feat(node): S5 2b part 2 — wire event-qualified recovery admission into the runtime (crash-safe live-rollback pre-clear) |
| `aa2bba37` | feat | feat(rollback): BLUE recovery-reconcile decision (S5 step 2b, part 1 — recovery authority) |
| `3682068b` | feat | feat(chaindb): persist the accumulator lineage anchor (LastAdvancedPoint) (S5 step 2a — store authority) |
| `48fc423a` | feat | feat(rollback): BLUE k-bounded + lineage-checked rollback-admission guard (S5 step 1) |
| `306ceb40` | docs | docs(ledger): revise S5 scope — the k-bounded rollback guard + lineage-checked reset land in S5, not S4 |
| `e096e014` | docs | docs(ledger): open S5 — restart/rollback replay-equivalence contract (the S4 recovery-promotion precondition) |
| `e476415a` | docs | docs(ledger): CE-3d final green declaration — flip DC-EPOCH-23 (fee/pot) + DC-EPOCH-24 (snapshot pool-set) to enforced, byte-exact on the v5 schema-v4 seed |
| `392433a1` | feat | feat(epoch): reject a pre-C accumulator store (schema v3->v4) — one replay meaning for the persisted snapshot-inclusion semantics (DC-EPOCH-24) |
| `e469f878` | feat | feat(epoch): snapshot pool-set = cardano ssActiveStake NonZero membership — close the CE-3d go phantom-pool residual (DC-EPOCH-24) |
| `fd8b07c8` | feat | feat(epoch): reduce the bootstrap fee pot by the RUPD feeSS (deltaF) — close the CE-3d reward/pots residual (DC-EPOCH-23) |
| `88128e72` | feat | feat(bootstrap): import snapshot-bound Conway deposit params into native-Mithril bootstrap authority (DC-CINPUT-07) |
| `1580b123` | test | test(ledger): reduced-plane recovery/fork-switch/boundary proof (RVBP P3) |
| `dafe0faf` | feat | feat(ledger): build the epoch-boundary mark POST-RUPD from a point-bound base (CE3D-REWARD-ACCOUNT-EVOLUTION-CORRECTION S1) |
| `aa9107a9` | feat | feat(ledger): reduced-plane typed non-authority on the live path (RVBP P1/P2 + B1) |
| `c15be61e` | feat | feat(ledger): reduced-boundary projection types + FullBoundaryStateRequired (RVBP P1 foundation) |
| `5c5b8f7f` | docs | docs(ledger): open REDUCED-VALIDATION-BOUNDARY-PLANE -- invariants + P1 slice (the reduced-plane typed non-authority) |
| `3de15187` | docs | docs(ledger): S2 design -- the reduced-validation boundary plane (capability-typed split) |
| `a3faf1e0` | docs | docs(ledger): CE3D-REWARD-ACCOUNT-EVOLUTION-CORRECTION S1 -- boundary reorder design (staged post-RUPD mark, both paths) |
| `a52afd77` | test | test(ledger): CE3D-REWARD-ACCOUNT-EVOLUTION-CORRECTION S0 -- the go-stake residual root-caused to a pre-RUPD mark snapshot (GREEN diagnosis) |
| `a882d304` | test | test(ledger): CE3D-GO-STAKE-DERIVATION-LOCALIZATION S1 -- the -343B go-stake residual localized to the reward-account contribution (GREEN evidence) |
| `52a6e2c7` | test | test(ledger): B3c.0 -- base UTxO proven byte-exact; the -343B go-stake residual adjudicated REAL (GREEN evidence) |
| `710f23db` | test | test(gov): bind S5 report provenance -- exact fixtures, decoder, code commits, canonical hash (reproducible) |
| `dff581ab` | test | test(gov): the CRE S5 differential proof + residual report -- one artifact, three separated claims |
| `d02cff14` | feat | feat(gov): atomic exec-units parameter-change enactment -- the single-authority enact path (CRE S4.3c) |
| `16a40260` | test | test(gov): V11 live-seed lineage evidence -- fresh re-bootstrap + warm-restart proven (CRE S4.3b-bootstrap obligation A) |
| `163e1428` | test | test(gov): the real-witness manifest -- 69c948cd..#0 decodes via keys 20/21 from real bytes (CRE S4.3b-bootstrap obligation B) |
| `0e13d139` | feat | feat(gov): V11 executable execution-memory + previous-action state, inert (CRE S4.3b) |
| `f4a1f748` | refactor | refactor(gov): one governance authority for the Conway epoch boundary + correct the replay expired-deposit drop (CRE S4.3a) |
| `262415bd` | feat | feat(gov): activate the DRep/committee ratification gate on the live boundary (CRE S4.2) |
| `6d2948c7` | test | test(gov): S4.1b operational capstone — live V2 governance seed re-bootstrap + warm-restart proof |
| `2bf3b41d` | test | test(gov): S4.1b — V2 governance seed binding + restart proof (the sealed re-bootstrap boundary) |
| `d6efe3e6` | feat | feat(gov): version ConwayGovState with authoritative num_dormant — no fabricated default (CRE S4.1) |
| `c2f5960e` | test | test(gov): the CRE S4 oracle anchor — the real ratify gate reproduces the census enactment |
| `96793f46` | feat | feat(ledger): extract the DRep voting-stake derivation as the CRE S3 distribution authority |
| `bae70fe9` | test | test(ledger): repair the stale VState stub in the ledgerdb_state hermetic fixture |
| `c0471d09` | feat | feat(testkit): bind the CRE enactment census to selected-chain witnesses (ImmutableDB reader + permanent fixture) |
| `b73701a7` | feat | feat(ledger): decode prevPParams + the enacted-authority PParamUpdate root (CRE enactment census) |
| `04f1c2b8` | test | test(governance): CRE enactment-census row-emission scaffold (canonical rows + the four evidence additions) |
| `7b2f4755` | feat | feat(ledger): decode maxBlockExUnits.mem + the deposit pot for the CRE enactment census |
| `7a27c475` | fix | fix(ledger): tolerate a retired block producer in the non-UTxO decode (surfaced by the CRE enactment census) |
| `965b308a` | test | test(governance): CRE enactment-census probe -- validate the decoder at epoch 1088 (ratify/enact ground-truth fixture, WIP) |
| `feb5cf10` | test | test(governance): lock S2 vote-capture replay determinism + reclassify as canonical vote-record authority (CONWAY-RATIFICATION-AND-ENACTMENT-AUTHORITY S2) |
| `c497db9b` | feat | feat(governance): capture live votes into the tracked proposals' vote maps, replacing the vote tripwire (CONWAY-RATIFICATION-AND-ENACTMENT-AUTHORITY S2) |
| `9a2b4818` | feat | feat(governance): import the DRep-expiry + committee-hot-key baseline from the VState (CONWAY-RATIFICATION-AND-ENACTMENT-AUTHORITY S1 part 2b) |
| `e57433cc` | feat | feat(governance): import the bootstrap DRep vote-delegation baseline from the DState UMap (CONWAY-RATIFICATION-AND-ENACTMENT-AUTHORITY S1 part 2a) |
| `524829e5` | feat | feat(governance): import + commitment-bind the per-action voting thresholds, without activating the gate (CONWAY-RATIFICATION-AND-ENACTMENT-AUTHORITY S1 part 1) |
| `406888ab` | docs | docs(governance): open CONWAY-RATIFICATION-AND-ENACTMENT-AUTHORITY + the S0 oracle ground-truth harness |
| `84286d95` | test | test(governance): S5 — prove the S4 refund closes the -500B CE-3d gap on the real proposals (CONWAY-PROPOSAL-DEPOSIT-EXPIRY) |
| `6934afb4` | feat | feat(epoch): the boundary deposit-expiry-refund evaluator — close the -500B CE-3d gap (CONWAY-PROPOSAL-DEPOSIT-EXPIRY S4) |
| `0fbb85e2` | feat | feat(governance): the S4.0 ratification census — prove the committee-only authority resolves the whole tracked set (CONWAY-PROPOSAL-DEPOSIT-EXPIRY) |
| `27e23fd9` | feat | feat(epoch): capture live gov proposals + a vote tripwire + the imported expiry-lifetime authority (CONWAY-PROPOSAL-DEPOSIT-EXPIRY S3) |
| `665f72e4` | fix | fix(test): repair the stale non-UTxO hermetic fixture for the gov-state decoder |
| `9855ad56` | feat | feat(bootstrap): reject a pre-import accumulator at warm-start — absent != empty (CONWAY-PROPOSAL-DEPOSIT-EXPIRY S2) |
| `d2522faf` | feat | feat(bootstrap): import all post-seed boundary inputs — fee pot, RUPD, gov proposals (CONWAY-PROPOSAL-DEPOSIT-EXPIRY S1) |
| `aeeaf89d` | fix | fix(ledger): pay script-hash and pool-owner stakers their exact reward share |
| `7e3c09ef` | feat | feat(epoch): seed nesBcur and retire the redundant bootstrap-RUPD application |
| `1702006e` | fix | fix(epoch): derive monetary-expansion eta from the network's real epoch length, not a mainnet constant (LIVE-LEDGER-EPOCH-TRANSITION, CE-3d) |
| `e8127575` | fix | fix(bootstrap): seed the EpochAccumulator's mark/set/go from the certified snapshot (LIVE-LEDGER-EPOCH-TRANSITION, CE-3d) |
| `b8e33ff0` | docs | docs(epoch): CE-3c proven live -- the accumulator crosses two real preview boundaries (LIVE-LEDGER-EPOCH-TRANSITION S3, DC-EPOCH-22) |
| `8232fe73` | feat | feat(epoch): the BOUNDARY-ALIGNED co-advancer -- the accumulator crosses live (DC-EPOCH-22, LIVE-LEDGER-EPOCH-TRANSITION S3 #2b-iii) |
| `ee33cc4c` | feat | feat(epoch): the durable BoundaryMark witness -- point + lineage, bound before the cross (DC-EPOCH-22, LIVE-LEDGER-EPOCH-TRANSITION S3 #2b-ii) |
| `8d047dee` | feat | feat(epoch): the accumulator boundary-cross entry point -- supply the mark, fire NEWEPOCH (DC-EPOCH-22, LIVE-LEDGER-EPOCH-TRANSITION S3 #2b-i) |
| `c1a0ec85` | feat | feat(epoch): per-credential boundary mark -- byte-exact member + leader rewards (DC-EPOCH-21, LIVE-LEDGER-EPOCH-TRANSITION S3) |
| `ce22ca27` | docs | docs(epoch): S3 item #2 finding -- the boundary mark must be PER-CREDENTIAL, not per-pool (LIVE-LEDGER-EPOCH-TRANSITION) |
| `f41456da` | fix | fix(epoch): one canonical POOLREAP at the boundary -- fix the dead delegation-clear + the reward-account discriminant (DC-EPOCH-21, LIVE-LEDGER-EPOCH-TRANSITION S3) |
| `e05a3ec1` | docs | docs(epoch): scope S3 -- the byte-exact boundary gate, two reconciliation items RESOLVED (LIVE-LEDGER-EPOCH-TRANSITION) |
| `7c7b3a30` | test | test(epoch): close S2 CE-2b with an exactly-one-fee-scan-per-admit capstone (LIVE-LEDGER-EPOCH-TRANSITION) |
| `d7653561` | fix | fix(epoch): warm-start recovery dispatch -- recover the seed+1 bridge authority, not only seed+2 (EPOCH-CONSENSUS-VIEW) |
| `f76d738b` | docs | docs(epoch): record the S2 RECOVER warm-start survival proof + a separate EVIEW finding |
| `89c1bc79` | docs | docs(epoch): record + mechanically guard S2 RECOVER (LIVE-LEDGER-EPOCH-TRANSITION DC-EPOCH-20) |
| `59153f36` | feat | feat(epoch): advance-to-tip accumulator reconciliation -- warm-start catch-up + reorg rematerialize (LIVE-LEDGER-EPOCH-TRANSITION S2) |
| `6f18ee0e` | docs | docs(epoch): record the S2 within-epoch wiring live-proven on preview (LIVE-LEDGER-EPOCH-TRANSITION S2) |
| `68846dcc` | feat | feat(epoch): advance the durable accumulator on the live follow -- observe-only after each durable admit (LIVE-LEDGER-EPOCH-TRANSITION S2, DC-EPOCH-20) |
| `a842cfe1` | feat | feat(epoch): seal the bootstrap SEED accumulator at native firstrun -- the durable within-epoch substrate (LIVE-LEDGER-EPOCH-TRANSITION S2, DC-EPOCH-20) |
| `53e6829a` | feat | feat(epoch): the bootstrap SEED accumulator -- manifest-bound, two-buffer split (LIVE-LEDGER-EPOCH-TRANSITION S2, PO-3/CE-2f) |
| `9fe01011` | feat | feat(epoch): the within-epoch accumulator advancer -- observe-only stall on a boundary (LIVE-LEDGER-EPOCH-TRANSITION S2, DC-EPOCH-20) |
| `b2185be6` | feat | feat(epoch): durable EpochAccumulatorStore -- the accumulator's single-blob home (LIVE-LEDGER-EPOCH-TRANSITION S2, DC-EPOCH-20, PO-2) |
| `cbf3e68a` | feat | feat(epoch): validity-aware within-epoch fees -- invalid-tx collateral, not declared fee (LIVE-LEDGER-EPOCH-TRANSITION S2, PO-1) |
| `59f04758` | docs | docs(epoch): scope LIVE-LEDGER-EPOCH-TRANSITION S2 + declare DC-EPOCH-20 -- atomic-or-rematerialized selected-block admission |
| `5d16eaef` | feat | feat(epoch): the non-UTxO EpochAccumulator + apply_selected_block contract -- the self-sustaining ledger loop (DC-EPOCH-19, LIVE-LEDGER-EPOCH-TRANSITION S1) |
| `2ba2bdb3` | docs | docs(epoch): scope LIVE-LEDGER-EPOCH-TRANSITION + declare DC-EPOCH-19 -- the continuous self-sustaining ledger loop |
| `d6e52170` | fix | fix(epoch): surface the specific bootstrap-RUPD-absent reason at the seed+2 activation seam (DC-EPOCH-18) |
| `c4e0413b` | feat | feat(epoch): apply the bootstrap reward update at the window-end -- byte-exact seed+2 stake (DC-EPOCH-18, EPOCH-CONSENSUS-VIEW B3c) |
| `dabb4210` | feat | feat(consensus): warm-start recovery across a crossed epoch boundary |
| `bbae56be` | feat | feat(node): observable follow progress + a known log destination (node.log) |
| `84fec1b5` | fix | fix(ledger): correct flipped Credential tag in native-bootstrap decode (DC-LEDGER-10) |
| `c13d4414` | fix | fix(epoch): lag-aware activation predicate + did-advance seam -- cross boundary 2 (DC-EPOCH-17, ECA-B3) |
| `b1d0fc7b` | feat | feat(epoch): yield-at-boundary -- run_node_sync returns SyncOutcome so the checkpoint advances per-boundary (DC-EPOCH-17, ECA-B3b) |
| `23829091` | feat | feat(epoch): generalize the activation seam to advance per boundary -- replay-derived seed+2 (DC-EPOCH-17, ECA-B3) |
| `b058ff1c` | feat | feat(epoch): ActiveEpochAuthority.advance -- per-boundary authority advance (DC-EPOCH-17, ECA-B3) |
| `fc68a295` | docs | docs(epoch): scope ECA-B3 + declare DC-EPOCH-17 -- replay-derived seed+2 authority |
| `44e07782` | docs | docs(epoch): flip DC-EPOCH-16 declared -> enforced -- eta0(seed+2) proven live (ECA-B2c) |
| `e8589e1e` | fix | fix(epoch): seed the evolving nonce from the full 6-nonce PraosState (DC-EPOCH-16, ECA-B2c) |
| `14880463` | feat | feat(epoch): B2 part 2 -- live RSW freeze + boundary tick on the follow path (DC-EPOCH-16) |
| `9040615b` | feat | feat(epoch): B2 part 1 -- RSW era-geometry field + the verified venue k source (DC-EPOCH-16) |
| `7356679a` | docs | docs(epoch): scope ECA-B2 -- live candidate-freeze (RSW) + boundary tick + the eta0(seed+2) gate |
| `79467c84` | feat | feat(epoch): rolling Praos nonce on the follow path -- chain-dep combine + back-compat snapshot (DC-EPOCH-16, ECA-B1) |
| `c0bc425b` | docs | docs(epoch): declare DC-EPOCH-16 + scope ECA-B1 (rolling Praos nonce on the follow path) |
| `4657cee5` | chore | chore(epoch): reconcile EVIEW gates + registry to the post-activation reality (ECA Tier A) |
| `26565bec` | feat | feat(epoch): native Mithril first-boundary bridge -- survive seed->seed+1 (ECA-5, DC-EPOCH-15) |
| `08fa37f6` | feat | feat(epoch): cross the epoch boundary -- forecast horizon extends with N+1 authority promotion (DC-EPOCH-15, ECA-5) |
| `5599f297` | docs | docs(epoch): declare DC-EPOCH-15 (forecast horizon <=> N+1 authority promotion) + ECA-5 slice summary |
| `25a6bde3` | docs | docs: getting-started guide for running Ade on Cardano preview |
| `87e74843` | fix | fix(mithril): snapshot fetch layout symlinks must be absolute (relative --output-dir dangled) (S8) |
| `886ca138` | fix | fix(epoch): native Mithril decode read the leader-VRF eta0 from the wrong PraosState nonce slot (S7) |
| `b0bbaaf5` | feat | feat(epoch): relay-only live follow -- port the forge-ON follow setup into forge-OFF (S6) |
| `54833173` | feat | feat(epoch): native operational continuity -- warm-start snapshot + in-memory seed inputs (S5) |
| `6d223a36` | feat | feat(mithril): `ade mithril snapshot fetch` -- native acquisition + manifest (S4) |
| `3af74b8a` | feat | feat(cli): `ade node run` native entrypoint -- bootstrap + warm-start, closed to legacy inputs (S3) |
| `769196af` | feat | feat(epoch): the judge-facing --bootstrap-mithril native startup command (S2 Gap 1b) |
| `59cfa802` | feat | feat(epoch): native FirstRun resolves genesis from the committed --network profile, manifest-bound (S2 Gap 1a) |
| `aa7503bc` | feat | feat(epoch): native Mithril FirstRun builds the EVIEW reduced checkpoint inline (DC-MITHRIL-08, S2 Gap 2) |
| `6e04f1fc` | test | test(epoch): hermetic ADE1 shadow-derive regression + shadow stake-agreement evidence |
| `25d11636` | docs | docs(invariant-registry): re-scope DC-EVIEW-08 to the ECA window-replay architecture |
| `7964d4df` | chore | chore(registry): drop a deleted test ref + bind a self-declaring CI gate |
| `a24d0c39` | docs | docs(grounding): regenerate the four grounding docs at cdcd9397 |
| `5333d0b6` | chore | chore(registry): backfill omitted status on DC-EPOCH-14 + DC-MITHRIL-04 |
| `cdcd9397` | feat | feat(epoch): live FirstRun -> native Mithril bootstrap invocation (DC-MITHRIL-07, S1d) |
| `942cd97c` | feat | feat(epoch): tables -> authoritative UTxOState materialization (DC-MITHRIL-06, S1c) |
| `c952c767` | feat | feat(epoch): native Mithril authority transition -- assemble + atomic persist (DC-MITHRIL-03, S1b) |
| `e84ebb0c` | chore | chore(registry): repair DC-MITHRIL-01/02 ID collision + add a uniqueness guard |
| `53c27bc4` | feat | feat(epoch): native non-UTxO snapshot decoder + manifest-bound network identity (S1a-1) |
| `cb20ab02` | feat | feat(ledger): era-aware protocol-parameter min-UTxO representation (DC-LEDGER-PARAMS-01, S1a-2) |
| `7c769801` | docs | docs(testkit): track pre-existing epoch_boundary_logic hang as a CI hygiene blocker |
| `5426dceb` | feat | feat(ledger): output asset quantity is the Word64 domain (OutputAssetQuantity, DC-LEDGER-VALUE-01) |
| `6cab0d6c` | feat | feat(epoch): native V2 LedgerDB tables MemPack TxOut decoder -> faithful UTxO (DC-MITHRIL-02, Stage 2) |
| `3bbba530` | feat | feat(epoch): native V2 LedgerDB state decoder -> canonical CertState (DC-MITHRIL-01, Stage 1) |
| `7386bf82` | feat | feat(epoch): reclassify cli exporter as auxiliary; V2 LedgerDB native-decode probe + manifest v2 |
| `f09cc0ec` | feat | feat(epoch): bootstrap-cert-state producer, live-verified on Preview |
| `0a500e59` | test | test(epoch): prove warm-start fail-closes on a wrong CLI network magic (DC-EPOCH-14) |
| `ad41b274` | feat | feat(epoch): atomic epoch-authority transition + crash recovery (ECA-2/3/4) |
| `124c87da` | feat | feat(epoch): persist the consensus-profile hashes in the v4 seed sidecar (ECA-2-pre) |
| `a17c7aab` | feat | feat(epoch): remove the EVIEW_ACTIVATION_ARMED semantic gate (ECA-1) |
| `4614e977` | feat | feat(epoch): leadership-complete EpochConsensusView + exclusive projection (ECA-0b) |
| `ad704f86` | feat | feat(epoch): cardano-faithful pool lifecycle in the reduced window (ECA-0a) |
| `a50a3ee8` | feat | feat(epoch): wire the gated epoch-view activation into the relay loop (S3f-4d-wire-3b-2) |
| `4c63c03d` | feat | feat(epoch): the gated boundary orchestration (S3f-4d-wire-3b-1) |
| `e6e07ae0` | feat | feat(epoch): the boundary-activation orchestration (S3f-4d-wire-3a) |
| `bcef6404` | feat | feat(epoch): the readiness witness + the sole authoritative derive (S3f-4d-wire-2b) |
| `39b2c314` | feat | feat(epoch): runtime readiness witness + replay seed-state checkpoint (S3f-4d-wire-2a) |
| `e14a0e15` | feat | feat(epoch): live source-window extraction for dual-path activation (S3f-4d-wire-1) |
| `bfa0b54a` | feat | feat(epoch): live stake-by-pool derive for the shadow proof (DC-EPOCH-11, S3f-4d-mat-shadow mechanism) |
| `3c7d9cc2` | feat | feat(epoch): fail-closed readiness gate for the live reduced checkpoint (DC-EPOCH-11, S3f-4d-mat-4) |
| `b151f399` | feat | feat(epoch): reorg re-materialize for the live reduced checkpoint (DC-EPOCH-11, S3f-4d-mat-3) |
| `a916eece` | feat | feat(epoch): wire the live reduced checkpoint into the relay loop (DC-EPOCH-11, S3f-4d-mat-2c) |
| `3d597fcb` | feat | feat(epoch): live ChainDB-replay checkpoint advancer (DC-EPOCH-11, S3f-4d-mat-2b) |
| `fdc3d062` | feat | feat(epoch): reduced-checkpoint per-block advance primitive (DC-EPOCH-11, S3f-4d-mat-2a) |
| `0ac92cba` | feat | feat(epoch): live reduced-checkpoint build at bootstrap (DC-EPOCH-11, S3f-4d-mat-1) |
| `38aa5518` | feat | feat(epoch): boundary activation orchestration -- the sequenced flip (DC-EPOCH-10) |
| `28c05bff` | feat | feat(epoch): activation candidate derivation from a validated window (DC-EPOCH-09) |
| `235e3183` | feat | feat(epoch): activation source window + named-role source->target mapping (DC-EPOCH-08) |
| `49a4d8ce` | feat | feat(epoch): activation durable-before-visible + crash recovery (DC-EPOCH-06) |
| `91293215` | feat | feat(epoch): activation predicate + atomically-published active view (DC-EPOCH-05/07) |
| `29253e4c` | feat | feat(epoch): WAL activation record -- the durable activation substrate (DC-EPOCH-04) |
| `86353625` | feat | feat(epoch): deterministic fail-closed epoch-rebind seam (DC-EVIEW-11, strengthens DC-EPOCH-03) |
| `7f7d266a` | feat | feat(epoch): the window driver -- advance + aggregate over a block window (DC-EVIEW-10) |
| `bd8b0def` | feat | feat(epoch): manifest-bound bootstrap cert-state import (DC-EVIEW-09) |
| `3c2db639` | feat | feat(epoch): activation consumption point -- the boundary consumes the aggregate (DC-EVIEW-08 S3f-1) |
| `62eb6738` | docs | docs(epoch): record the S3c live differential-oracle result (DC-EVIEW-05) |
| `a9d1f148` | fix | fix(ci): S3b-1 checkpoint gate no longer false-positives on the S3c reader |
| `ce778913` | feat | feat(epoch): the bound, immutable EpochConsensusView (DC-EVIEW-07) |
| `88fdfadf` | feat | feat(epoch): snapshot formation + the k-immutability stability gate (DC-EVIEW-06) |
| `77a7e3f3` | feat | feat(epoch): per-pool stake aggregation -- the linchpin (DC-EVIEW-05) |
| `8c0ff66f` | feat | feat(epoch): windowed advance of the reduced-UTxO checkpoint (DC-EVIEW-04b) |
| `83ead7be` | feat | feat(epoch): durable reduced-UTxO checkpoint -- the minimal native state (DC-EVIEW-04) |
| `388a3b61` | docs | docs(epoch): scope EPOCH-CONSENSUS-VIEW S3b-1 -- durable reduced-UTxO checkpoint (pre-code) |
| `d6d015eb` | docs | docs(epoch): scope EPOCH-CONSENSUS-VIEW S3b (umbrella) -- replay-window materialization |
| `c71a308f` | feat | feat(epoch): era-parameterized pointer decode + resolution (DC-EVIEW-03) |
| `7a2462b1` | docs | docs(epoch): scope EPOCH-CONSENSUS-VIEW S3a -- pointer decode/resolution (pre-code) |
| `a8b5d1c6` | docs | docs(epoch): scope EPOCH-CONSENSUS-VIEW slice 3 -- native next-epoch view (pre-code) |
| `8f74ccef` | feat | feat(epoch): typed era-gated stake-reference classification (DC-EVIEW-02) |
| `502b23b5` | docs | docs(epoch): scope EPOCH-CONSENSUS-VIEW slice 2 -- typed stake-reference classification |
| `85fbc04f` | feat | feat(epoch): prove the bounded crash-safe transient-materialization gate (DC-EVIEW-01) |
| `28be6635` | docs | docs(epoch): slice 1 resolved entry obligations + tightenings + GREEN classification |
| `39a6b5af` | docs | docs(epoch): EPOCH-CONSENSUS-VIEW slice 1 scope -- redb temporary-materialization gate |
| `84e1019c` | docs | docs(epoch): EPOCH-CONSENSUS-VIEW design-analysis record (architecture selected, mechanism unapproved) |
| `cf508424` | docs | docs(node): adoption channel is the localRoot dial, not a duplex responder |
| `300959c6` | fix | fix(node): participant forge derives base from the live AO-selected durable tip, not a self-forge latch (DC-FOLLOW-FORGE-01) |
| `5e3c0855` | feat | feat(node): participant venue forges on the AO-selected durable head (CN-FOLLOW-01) |
| `0c2dae4d` | fix | fix(forge): KES-period gate returns the opcert-anchored relative evolution, not the absolute period (DC-CRYPTO-10) |
| `5b99333c` | *(harden)* | harden(node): forward-sync cache hit-path test + structural rollback invalidation (DC-MEM-11) |
| `88e64df2` | perf | perf(node): forward-sync admit reuses cached UTxO fingerprint, not O(n) per-block recompute (DC-MEM-11) |
| `c51d7d81` | fix | fix(forge): KES shell-init anchors evolution-0 at opcert_start, evolves to current (OP-OPS-04) |
| `1be6e855` | fix | fix(node): warm-start era-schedule uses durable venue geometry (DC-CINPUT-05) |
| `3c6c30ea` | fix | fix(admission): persist admitted block bytes before WAL (DC-WAL-05) |
| `13d506cc` | *(registry)* | registry: enforce RO-MITHRIL-IMPORT-01 with documented evidence gate |
| `31ae1f63` | *(evidence)* | evidence: add Mithril documented-interface preprod bundle |
| `f268d3d9` | fix | fix(evidence): capture runs end-to-end on the live venue + out-of-tree seed handling |
| `176c7059` | fix | fix(evidence): harden mithril capture for non-destructive scratch venue (no tautology) |
| `93dc99bb` | feat | feat(evidence): mithril documented-interface capture + validation tooling (prep, no flip) |
| `88c862cc` | docs | docs(invariant-registry): bind T-CONS-01 to CN-CONS-01 enforcement (declared -> enforced) |
| `1b79add0` | chore | chore(idd): bump head_deltas_baseline 862cd2cb -> 470f9b89 (MEM-OPT-UTXO-DISK close) |

> **Note (commit-attribution policy).** Per this repo's `CLAUDE.md` override (vibe-coded-node bounty trailer
> requirement), commits carry a `Co-Authored-By:` model-attribution trailer — an Ade-local override of the global
> no-AI-attribution rule, applying to **commit messages only**. It does not affect this doc's content.

## 2. New Modules

The span adds **38 new source modules** (`.rs` under `crates/**/src/`, no new crate, no new `Cargo.toml`) + **25 new
test files**. BLUE classification/reduction/decode primitives live in `ade_ledger` + `ade_codec`; the durable-storage
checkpoints/accumulators + the RED orchestration shell live in `ade_runtime` + `ade_node`. Modules first documented in
the prior (`cdcd9397`) refresh are marked *(bands 1–4)*; the rest are new to this refresh.

### BLUE modules (`ade_ledger`, `ade_codec`)

| Module | Color | Purpose | Added in |
|--------|-------|---------|----------|
| `ade_codec::address::pointer` | BLUE | Era-parameterized pointer decode (`Ptr{slot,txIx,certIx}`, `PointerDecodeError`). | EVIEW S3a *(band 2)* |
| `ade_ledger::stake_ref` | BLUE | Typed, era-gated stake-reference classification; no fixed byte offset is authoritative. | EVIEW S2 *(band 2)* |
| `ade_ledger::pointer_resolve` | BLUE | Pointer→credential resolution (pre-Conway only). | EVIEW S3a *(band 2)* |
| `ade_ledger::reduced_utxo` / `reduced_advance` / `reduced_aggregate` / `reduced_snapshot` / `reduced_epoch_view` | BLUE | The EVIEW reduction pipeline: `TxIn→(Coin,ReducedStakeRef)` → per-block advance → per-pool `StakeByPool` → k-immutable snapshot → bound immutable `EpochConsensusView`. | EVIEW S3b–S3e *(band 2)* |
| `ade_ledger::bootstrap_manifest` | BLUE | Manifest-bound bootstrap cert-state import (`BootstrapManifest`/`…Error`). | EVIEW S3f *(band 2)* |
| `ade_ledger::ledgerdb_state` / `ledgerdb_tables` / `mithril_utxo_materialize` | BLUE | Native V2 LedgerDB decoders (state→`CertState`+pool distr+Praos nonces; tables MemPack `TxOut`→UTxO) + tables→`UTxOState`. Raw CBOR is RED input; the projection is authority. | Mithril decode *(band 4)* |
| `ade_ledger::epoch_accumulator` | BLUE | The **non-UTxO `EpochAccumulator`** + `apply_selected_block` — the self-sustaining ledger loop. | LIVE-LEDGER S1 (`5d16eaef`) |
| `ade_ledger::frozen_leadership` | BLUE | **`FrozenLeadershipPoolDistr`** — the self-contained leadership authority (SET stake + snapshot-frozen VRF) + seed identity. | S4-pre (`501bf89a`) |
| `ade_ledger::reduced_boundary` | BLUE | RVBP reduced-boundary projection types + `FullBoundaryStateRequired` (capability-typed non-authority). | RVBP P1 (`c15be61e`) |
| `ade_ledger::bootstrap_reward_update` | BLUE | Apply the bootstrap reward update at window-end — byte-exact seed+2 stake (`DC-EPOCH-18`). | B3c (`c4e0413b`) |
| `ade_ledger::bootstrap_bridge` | BLUE | Bootstrap bridge plumbing for the seed→seed+1 authority transition. | ECA/native-Mithril bridge |
| `ade_ledger::cred` | BLUE | Canonical credential type (surfaced by the flipped-Credential-tag decode fix, `DC-LEDGER-10`). | band 7 (`84fec1b5`) |
| `ade_ledger::rollback::admission` | BLUE | The **k-bounded + lineage-checked rollback-admission guard** + recovery-reconcile decision. | S5 (`48fc423a`) |

### RED / storage modules (`ade_node`, `ade_runtime`)

| Module | Color | Purpose | Added in |
|--------|-------|---------|----------|
| `ade_runtime::chaindb::reduced_utxo_checkpoint` / `reduced_window_driver` / `transient_epoch_view` | GREEN-by-contract / RED | The durable reduced-UTxO checkpoint (crash-safe, marker-LAST, hash-chain fingerprint; never authority, never on the live path), its window driver, and the transient replay-window lifecycle. | EVIEW *(band 2)* |
| `ade_runtime::chaindb::epoch_accumulator_store` / `epoch_accumulator_advance` | RED | The durable single-blob `EpochAccumulatorStore` + the within-epoch advancer (observe-only; stalls on a boundary). | LIVE-LEDGER S2 (`b2185be6`) |
| `ade_runtime::mithril_native_assembly` | RED | The native Mithril authority transition — assemble the canonical state + atomic persist. | Mithril S1b *(band 4)* |
| `ade_runtime::consensus_inputs::cert_state_extract` | RED | Cert-state extraction plumbing for the bootstrap-cert-state producer. | ECA (`f09cc0ec`) |
| `ade_runtime::bin::transient_view_kill_target` | RED (test bin) | The SIGKILL kill-target for the crash-safe transient-materialization proof (`DC-EVIEW-01`). | EVIEW S1 *(band 2)* |
| `ade_node::epoch_source_window` / `epoch_candidate` / `epoch_activate` / `epoch_activation` / `epoch_wire` / `epoch_rebind` | RED | The activation orchestration shell (source-window validate → sole authoritative derive → predicate → WAL→publish → rebind to forge). `epoch_wire` no longer carries `EVIEW_ACTIVATION_ARMED` (removed ECA-1). | EVIEW wire *(band 2)*; gate removed *(band 3)* |
| `ade_node::native_firstrun` / `bootstrap_export` | RED | The live FirstRun → native Mithril bootstrap invocation + the (reclassified auxiliary) CLI exporter / V2 native-decode probe. | Mithril *(band 4)* |
| `ade_node::mithril_fetch` | RED | `ade mithril snapshot fetch` — native snapshot acquisition + manifest. | LIVE-1 S4 (`6d223a36`) |
| `ade_node::ops_log` | RED | Observable follow progress + a known log destination (`node.log`). | band 7 (`bbae56be`) |

### Testkit

| Module | Color | Purpose | Added in |
|--------|-------|---------|----------|
| `ade_testkit::harness::immutabledb_witness` | test | The ImmutableDB reader + permanent fixture binding the CRE enactment census to selected-chain witnesses. | CRE (`c0471d09`) |

> **Cross-reference (CODEMAP) — STALE.** The on-disk `docs/ade-CODEMAP.md` was regenerated at `cdcd9397` and does **not**
> contain the bands-5→13 modules — verified: `frozen_leadership`, `epoch_accumulator`, `reduced_boundary`,
> `mithril_fetch`, `ops_log`, `bootstrap_reward_update` each return **0 hits** in the CODEMAP. **Run `/codemap`** to add
> these under §BLUE (`ade_ledger::{epoch_accumulator, frozen_leadership, reduced_boundary, bootstrap_reward_update,
> bootstrap_bridge, cred, rollback::admission}`) and §RED (`ade_runtime::chaindb::{epoch_accumulator_store,
> epoch_accumulator_advance}`, `ade_node::{mithril_fetch, ops_log}`, `ade_testkit::harness::immutabledb_witness`).

## 3. Modules Modified

Per-module diffstats over the full span. Trivial touches (single-line, formatting) are omitted.

| Module / crate | Color / scope | Key changes |
|--------|---------------|-------------|
| `ade_ledger` (`+20,147 / −611`, 63 files) | **BLUE** | The single largest surface. New EVIEW/accumulator/Mithril/governance/reduced-plane families (§2), plus modified `rules.rs` (EVIEW-gated mark path; per-credential boundary mark; POOLREAP), `delegation.rs` (cardano-faithful pool lifecycle), `snapshot/{utxo_state,chain_dep}.rs` (materialization; `array(10)` chain-dep codec), `value.rs`/`mary.rs`/`pparams.rs` (Word64 value domain + `MinUtxoRule`), `seed_consensus_inputs.rs` (v4→v6 sidecar: consensus-profile hashes then `security_param`/`k`), `wal/event.rs` (`EpochConsensusViewActivated`). |
| `ade_node` (`+16,400 / −298`, 45 files) | **RED** | `node_lifecycle.rs` / `node_sync.rs` carry the relay-loop reduced-checkpoint advance, the automatic epoch-boundary activation, the accumulator co-advance, the per-boundary authority advance, the sealed-leadership promotion (S4), the rollback-follow forge path (LFH S1), and the `sidecar_freeze_rsw` shared freeze-window derivation (LFH S2). `cli.rs` gains `ade node run` / `ade mithril snapshot fetch` / `--bootstrap-mithril`. |
| `ade_runtime` (`+8,840 / −82`, 64 files) | **RED** | The chaindb checkpoint/accumulator stores (§2), `consensus_inputs::importer.rs` (bootstrap import of fee pot / RUPD / gov proposals / Conway deposit params; the `k`-required + `f≠0` ingress guards), `mithril_bootstrap.rs`/`genesis_bootstrap.rs`/`seed_consensus_merge.rs` (native bootstrap path), `consensus/genesis_parser.rs` (`security_param`). |
| **`ade_core`** (`+574 / −300`, 16 files) | **BLUE — consensus authority** | **The ECA-B1/B2 rolling Praos-nonce reshape.** `consensus/nonce.rs` (reshaped `NonceInput`, one `HeaderContribution`, `EpochBoundary` combine+rotation+no-reset, `CandidateFreeze` removed, `MissingLastEpochBlockNonce` fail-closed), `consensus/praos_state.rs` (`last_epoch_block_nonce: Option<Nonce>`), `consensus/header_validate.rs` (Step-9 threading of `prev_block_hash` + `freeze_boundary`), `consensus/era_schedule.rs` (`praos_rsw_slots`). Versioned + backward-compatible; **not** byte-identical to baseline. |
| `ade_testkit` (`+5,413 / −117`, 26 files) | test | The EVIEW/accumulator hermetic suites, the LedgerDB-decode corpus/oracle suites, the CPDE/CRE governance census suites, the CE-3d differential + B3c-localization suites, the ImmutableDB witness harness. |
| `ade_codec` (`+383 / −1`, 3 files) | **BLUE** | The `address::pointer` module (§2) + the address module wiring. |
| `ade_types` (`+38 / −4`, 1 file) | **BLUE** | Value/min-UTxO domain types (`OutputAssetQuantity` / `MinUtxoRule` surface). |
| `ade_network` (`+2 / −0`, 2 files) | mixed (BLUE sub-paths unchanged) | Trivial; no BLUE sub-path type change. |

> **BLUE was touched, including the consensus core.** Unlike the prior refresh (which correctly reported `ade_core`
> untouched at `cdcd9397`), the full span **does** modify `ade_core` (the Praos-nonce reshape). `ade_crypto` and
> `ade_plutus` are unchanged; the `ade_network` BLUE sub-paths are unchanged.

## 4. Feature Flags

**No project feature-flag deltas anywhere in the span.** Ade declares **no `[features]` table** in any workspace
`Cargo.toml` at either ref (`git grep '^\[features\]'` empty at `470f9b89` and `1e4896eb`), **no `#[cfg(feature = …)]`**
gate in `crates/`, and **no `compile_error!`** coupling. The former activation gate (`EVIEW_ACTIVATION_ARMED`) was a
plain `const bool` — a *semantic* gate, not a feature flag — and it was **removed** (`a17c7aab`); it is absent at HEAD.
There is no flag coupling to report.

## 5. CI Checks (200 → 255 over the full span; +55 new, 2 modified, 0 removed)

`git diff --name-status 470f9b89..1e4896eb -- 'ci/ci_check_*.sh'`: **55 `A`, 2 `M`, 0 `D`**
(`git ls-tree … | grep -c ci_check_` = **200 → 255**). Plus one non-gate helper added
(`ci/capture_mithril_documented_evidence.sh`) and one modified (`ci/build_consensus_inputs_bundle.sh`, LFH-S2 emits
`security_param`).

### Through `cdcd9397` (bands 1–4): 38 new gates *(previously documented)*

The prior refresh detailed these: the EVIEW substrate (+21: `ci_check_eview_*` ×18 + `ci_check_transient_view_*` ×3),
the ECA band (+5: `automatic_activation`, `atomic_authority`, `leadership_complete`, `pool_lifecycle`,
`seed_sidecar_v4`), the native-Mithril band (+9: `ledgerdb_state_decode`, `ledgerdb_tables_decode`,
`mithril_authority_transition`, `tables_to_utxostate`, `native_firstrun_no_cli_seed`, `native_nonutxo_decode`,
`value_quantity_domain`, `mithril_documented_evidence`, `registry_unique_ids`), and band 1 (+3: `forward_sync_fp_cache`,
`participant_forge_on_selected_head`, `admission_runner_no_block_byte_map`).

### `cdcd9397 → HEAD` (bands 6–13): 17 new gates + 2 modified

| Check | Status | Rule / cluster | What it checks |
|-------|--------|----------------|----------------|
| `ci_check_native_firstrun_reduced_checkpoint.sh` | **New** | `DC-MITHRIL-08` (band 6) | Native Mithril FirstRun builds the EVIEW reduced checkpoint inline at bootstrap. |
| `ci_check_eview_forecast_crossing.sh` | **New** | `DC-EPOCH-15` (ECA-5) | Crossing a boundary extends the forecast horizon with N+1 authority promotion. |
| `ci_check_praos_nonce_follow_evolution.sh` | **New** | `DC-EPOCH-16` (ECA-B1/B2) | The rolling Praos nonce on the follow path (one `HeaderContribution`; RSW candidate-freeze; boundary tick; fail-closed on an absent operand — never a fabricated nonce). |
| `ci_check_epoch_accumulator_no_utxo.sh` | **New** | `DC-EPOCH-19` (S1) | The `EpochAccumulator` is non-UTxO; `apply_selected_block` is the self-sustaining loop. |
| `ci_check_epoch_accumulator_recovery.sh` | **New** | `DC-EPOCH-20` (S2) | The durable accumulator advances observe-only, seals at firstrun, and reconciles (warm-start catch-up + reorg rematerialize). |
| `ci_check_poolreap_single_canonical.sh` | **New** | `DC-EPOCH-21` (S3) | Exactly one canonical POOLREAP at the boundary; per-credential mark; reward-account discriminant. |
| `ci_check_boundary_aligned_mark_capture.sh` | **New** | `DC-EPOCH-22` (S3) | The boundary-aligned co-advancer + the durable `BoundaryMark` witness (point + lineage, bound before the cross). |
| `ci_check_bootstrap_rupd_window_end.sh` | **New** | `DC-EPOCH-18` (B3c) | The bootstrap reward update applies at window-end → byte-exact seed+2 stake. |
| `ci_check_bootstrap_rupd_fee_reduction.sh` | **New** | `DC-EPOCH-23` (CE-3d) | The bootstrap fee pot is reduced by the RUPD `feeSS` (deltaF). |
| `ci_check_snapshot_pool_set_inclusion.sh` | **New** | `DC-EPOCH-24` (CE-3d) | Snapshot pool-set = cardano `ssActiveStake` NonZero membership; pre-C store rejected (schema v3→v4). |
| `ci_check_conway_deposit_params_bootstrap.sh` | **New** | `DC-CINPUT-07` (band 9) | Snapshot-bound Conway deposit params imported into the native-Mithril bootstrap authority. |
| `ci_check_gov_proposal_capture.sh` | **New** | `DC-GOV-01` (band 9) | Live gov proposals + votes captured into the tracked proposals' vote maps (replacing the tripwire). |
| `ci_check_reduced_boundary_plane.sh` | **New** | RVBP (band 10) | The reduced-boundary plane is typed non-authority on the live path (`FullBoundaryStateRequired`). |
| `ci_check_trusted_replay_boundary.sh` | **New** | RVBP / recovery (band 10) | The trusted replay boundary for the reduced plane (recovery / fork-switch / boundary). |
| `ci_check_frozen_leadership_authority.sh` | **New** | S4-pre | `FrozenLeadershipPoolDistr` is the self-contained leadership authority (SET stake + snapshot-frozen VRF), proven vs reference `nesPd`. |
| `ci_check_frozen_recovery_no_seed_window.sh` | **New** | S4-L1 | Warm/initial leadership recovery no longer reads the seed-window authority. |
| `ci_check_frozen_promotion_no_seed_window.sh` | **New** | S4-L2 | Promotion reads the epoch-indexed frozen leadership only (candidate ≥ seed+2), never the seed window. |
| `ci_check_credential_discriminant_closed.sh` | **Modified** | `DC-LEDGER-10` (band 7) | Tightened for the flipped-Credential-tag native-bootstrap decode fix. |
| `ci_check_warmstart_eta0_overlay.sh` | **Modified** | ECA-B (band 7) | Updated as the B2 live boundary-tick replaced the ECA-5 bridge `eta0` overlay. |

> **Cross-reference (TRACEABILITY) — STALE.** The on-disk `docs/ade-TRACEABILITY.md` was regenerated at `cdcd9397`; the
> 17 gates above (and the modified pair) are **not** yet reflected there. Each binds a registry rule (verified: the new
> gate names appear in the corresponding rule's `ci_scripts` / `code_locus` at HEAD). **Run `/traceability`.** Note the
> **declared-but-gated** pattern: the continuous-operation rules `DC-EPOCH-17/19/20/21/22/25`, `DC-CINPUT-07`,
> `DC-GOV-01` have gates that run green over the hermetic/corpus substrate but the registry status is **`declared`** —
> gate present + green; the flip to `enforced` is owed pending a committed live-flip transcript (the same pattern as
> `DC-EPOCH-11` / `DC-EVIEW-08`). This is the expected mid-arc state, not an orphan defect.

## 6. Canonical Type Registry Delta

**n/a — no separate canonical-type registry is configured** (`.idd-config.json` `canonical_type_registry: null`);
canonical-type rules live inline in the invariant registry under family **T**. The informational structural count
(`pub struct`/`pub enum` over the BLUE `core_paths`) moves **≈ `466 → 558` (+92, approximate)**:

- **`ade_ledger` +87** (`185 → 272`): the EVIEW reduction family, the accumulator/frozen-leadership/reduced-boundary
  families, the Mithril decode family, the governance/bootstrap families, the value/min-UTxO domain.
- **`ade_codec` +2** (`11 → 13`): `Ptr`, `PointerDecodeError`.
- **`ade_types` +2** (`82 → 84`): the value/min-UTxO surface.
- **`ade_core` +1** (`48 → 49`): the Praos-nonce reshape (`NonceInput`/`HeaderContribution` grammar). **This is the one
  structural change to the authoritative consensus core** — see §3 and the header CORRECTION.
- **Unchanged:** `ade_crypto` (22), `ade_plutus` (8), the `ade_network` BLUE sub-paths (~110).

**Zero BLUE canonical types removed** (append-only within the major version).

## 7. Normative / Invariant Rule Delta (380 → 432 full span; **+52, ZERO removals**)

The span added **52 rule IDs, zero removed** (registry **380 → 432**; `comm -23` of the sorted `id =` lists is empty;
`comm -13` = exactly 52). Status tally: **enforced 253 → 297 (+44)**, **declared 103 → 111 (+8)**, **partial 23 → 23
(0)**, **enforced_scaffolding 1 → 1 (0)**. Of the 52 new IDs, **42 land `enforced`, 10 land `declared`**; existing-rule
flips (`RO-MITHRIL-IMPORT-01`, `T-CONS-01`, `DC-EPOCH-16` → `enforced`) net the tally.

**The 52 new IDs by family:**

- **`DC-EVIEW-01..13` (+`04b`)** — 14 IDs, the EVIEW substrate (all `enforced` except `DC-EVIEW-08` `declared`).
- **`DC-EPOCH-04..25`** — 22 IDs, the activation + continuity + boundary arc. `enforced`: `04,05,06,07,08,09,10,12,13,14,15,16,18,23,24`. **`declared`**: `11,17,19,20,21,22,25` (the live-continuity + reduced-plane rules; gated + green, live-flip owed).
- **`DC-MITHRIL-03..08`** — 6 IDs, native Mithril decode/assemble/materialize/FirstRun (all `enforced`).
- **`DC-CINPUT-05/06/07`** — 3 IDs. `05`/`06` `enforced`; **`07`** (Conway deposit params bootstrap) `declared`.
- **`DC-LEDGER-VALUE-01` / `DC-LEDGER-PARAMS-01`** — Word64 value domain + era-aware min-UTxO (`enforced`).
- **`DC-WAL-05`, `DC-MEM-11`, `CN-FOLLOW-01`, `DC-FOLLOW-FORGE-01`** — band-1 fixes (all `enforced`).
- **`DC-GOV-01`** — live gov-proposal/vote capture (`declared`).

**Strengthenings of existing rules** (append-only `strengthened_in`) include `OP-OPS-04`, `DC-CRYPTO-10` (band 1), and
notably **`DC-EPOCH-16`** which gained `strengthened_in += "LIVE-FORGE-HARDENING-S2"` and **two appended tests**
(`seed_cinput_v6_persists_k_for_durable_candidate_freeze_window`,
`sidecar_freeze_rsw_derives_from_store_and_cross_checks_the_cli`) at the HEAD close — the rule stays `enforced`.

*(The configured `normative_docs` — the CE-79 tier-gate statement + addendum, the three contract docs, the CE-73
reclassification, and `CLAUDE.md` — were **not** changed anywhere in the span: `git diff --name-only 470f9b89..1e4896eb`
over those six paths is empty. The §7 delta is entirely the invariant-registry change.)*

This section is informational; the rule IDs, status values, and the zero-removal result are exact (read from the
registry at both refs). The BLUE-type and test counts are approximate attribute greps.

---

## Anomalies & cross-reference summary (surface prominently)

- **`ade_core` (the consensus authority) IS modified — the prior `48→48` byte-identical claim is STALE.** The
  ECA-B1/B2 rolling Praos-nonce reshape changed `ade_core` (+574 / −300, +1 BLUE type). It is versioned + backward-
  compatible + fail-closed (never a fabricated nonce), but the core is **not** byte-identical to baseline. Any doc
  asserting otherwise is describing `cdcd9397`.
- **The three sibling grounding docs are STALE (`cdcd9397`, 2026-06-24).** CODEMAP / SEAMS / TRACEABILITY do not contain
  the bands-5→13 modules, rules, or gates (verified: 6 new module names return 0 CODEMAP hits). **Run `/codemap`,
  `/seams`, `/traceability`** to re-align them with HEAD before relying on them for structural questions.
- **KNOWN pre-existing test failures in the configured replay command (do NOT attribute to this cluster).**
  `cargo test -p ade_testkit` (`.idd-config.json` `replay_cmd`) currently has **4 failures in `consensus_stream_replay`**
  — `NonceEvolution` `MissingLastEpochBlockNonce` at the epoch boundary — from the **in-flight ECA-B rolling-nonce
  reshape** (that corpus needs refreshing by the still-open `EPOCH-CONSENSUS-VIEW` cluster). These are **unrelated to
  `LIVE-FORGE-HARDENING`** (which added no BLUE transition). A separate pre-existing `ade_testkit` hang in
  `epoch_boundary_logic::all_epoch_boundaries_fire` was already noted at `7c769801`. The full-workspace replay is thus
  known-red until the corpus is regenerated; per-cluster gating runs on targeted suites.
- **Zero canonical-type removals; zero rule removals; zero CI removals; zero crate removals** across the span (all
  expected: 0). The registry enforces ID uniqueness mechanically (`ci_check_registry_unique_ids.sh`).
- **10 of the 52 new rules land `declared`** — the live-continuity + reduced-plane + governance-capture rules
  (`DC-EPOCH-11/17/19/20/21/22/25`, `DC-CINPUT-07`, `DC-GOV-01`, `DC-EVIEW-08`). Their gates run green over the
  hermetic/corpus substrate, but the `enforced` flip is owed pending committed live-flip transcripts — notable given
  `CE-4A`/`CE-4B` proved multi-boundary continuous operation live; the *registry* has not yet recorded those flips.
- **`LIVE-FORGE-HARDENING` BLUE-touch is precise, not zero.** LFH left `ade_core` and the ledger transition untouched;
  its only BLUE-`core_path` edit is the additive versioned seed-sidecar codec (`ade_ledger::seed_consensus_inputs`,
  v5→v6 `k` persistence) + a 1-line `consensus_view.rs` touch. Report it as "no authoritative-transition change,"
  **not** "no BLUE edit."
- **All 232 commits carry a clear conventional scope** — 3 use a non-`feat/fix/…` but unambiguous prefix
  (`harden(node):`, `registry:`, `evidence:`), surfaced rather than guessed. None is unclassifiable.

---

## Generation notes

### Regen `470f9b89 → 1e4896eb` (cluster-close refresh — LIVE-FORGE-HARDENING; baseline advances)

- **Baseline valid; IS a cluster-close refresh.** `git rev-parse 470f9b89` resolves; `git merge-base 470f9b89
  1e4896eb == 470f9b89` (strict ancestor; no tag). HEAD `1e4896eb` is the `LIVE-FORGE-HARDENING` close on `origin/main`.
  Per IDD discipline the baseline **advances** to `1e4896eb` (config `head_deltas_baseline` updated by the caller).
- **Counts are mechanical (git/grep/ls).** Commit log + `--shortstat` over `470f9b89..1e4896eb` (**232** commits, no
  merges / **387** files / **+74,026 / −5,752**); CI gate count via `git ls-tree -r --name-only <ref> ci/ | grep -c
  ci_check_` (**200 → 255**; name-status **55 A / 2 M / 0 D**); registry via `grep -c '^id = '` (**380 → 432**;
  `comm -23` empty; `comm -13` = 52) and `grep '^status = ' | sort | uniq -c` (enforced/scaffolding/partial/declared
  **253/1/23/103 → 297/1/23/111**); BLUE types via `pub (struct|enum)` grep over the BLUE `core_paths` src trees
  (`ade_ledger 185→272`, `ade_codec 11→13`, `ade_types 82→84`, **`ade_core 48→49`**, `ade_crypto 22`, `ade_plutus 8`);
  tests via the `#[test]`/`#[tokio::test]` attribute grep (**~2,666 → ~3,320**, approximate).
- **`ade_core` change verified in source.** `git diff --numstat 470f9b89..1e4896eb -- crates/ade_core/` = +574/−300 over
  16 files; the DC-EPOCH-16 registry `code_locus` names `ade_core/src/consensus/{nonce,praos_state,header_validate,
  era_schedule}.rs`. Surfaced as a CORRECTION to the prior `cdcd9397` framing.
- **The activation gate is GONE (verified).** `git grep EVIEW_ACTIVATION_ARMED 1e4896eb -- crates/` is empty.
- **Crate count unchanged (12 → 12).** Workspace `members` lists byte-identical at both refs; no new `Cargo.toml`.
- **No feature flag / `compile_error!` / new `--feature` CLI surface** at any ref.
- **Normative docs unchanged across the span.** `git diff --name-only 470f9b89..1e4896eb` over the six configured
  `normative_docs` paths is empty; the §7 delta is entirely the invariant-registry change.
- **§1 is verbatim from `git log` (newest first).** No editorial per commit; aggregation lives in the band narratives
  and §3.
- **This doc is regenerated in isolation; the other three grounding docs are NOT.** CODEMAP / SEAMS / TRACEABILITY are
  on-disk at `cdcd9397` (2026-06-24) and must be regenerated to re-align with HEAD — see the Anomalies block. Prefer
  regenerating over patching.
