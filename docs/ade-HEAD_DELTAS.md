# Ade — HEAD Deltas (Changes Since Baseline)

> **Status:** Living architectural document. Regenerated; not hand-edited.
>
> Regenerate with `/head-deltas <baseline>` after every cluster close. The baseline is recorded in `.idse-config.json` `head_deltas_baseline` (currently `1e4896eb`).

> Baseline: `1e4896eb` (2026-07-31 17:25:58 -0400)
> HEAD: `74c67670` (chore(idse): correct 14 re-evidence records (test-reference matching), 2026-11-09 10:56:49 -0500)
> 222 commits, 1091 files changed, +107842 / -15676 lines

> **Baseline note.** `1e4896eb` has no tag. `git rev-parse 1e4896eb` resolves and `git merge-base 1e4896eb HEAD == 1e4896eb`, so the baseline is a strict ancestor of HEAD. The span has no merge commits. Of the 1,091 changed paths, 622 are renames: the IDSE relocation (`fa48afa8`) moved the legacy IDD record under `docs/legacy/`. The line counts include that move, the new IDSE registry and 96 new evidence files under `docs/evidence/`. Crate source accounts for +28,039 / −2,431 over 193 files.

> **Invariant IDs in this document.** Ade moved to IDSE v3.0 (`strict` profile) inside this span, on 2026-11-08/09. Invariants are cited by their IDSE IDs from `docs/ade-invariant-registry.toml`. Commit subjects in §1 are verbatim from git and still carry legacy IDD IDs. Follow those forward through `docs/legacy/idd-to-idse-map.toml`, which gives every legacy ID a disposition.

---

## 0. Headline

| Count | Baseline (`1e4896eb`) | HEAD (`74c67670`) | Δ |
|---|---|---|---|
| Methodology | legacy IDD (`.idd-config.json`) | **IDSE v3.0, `strict`** (`.idse-config.json`, namespace `ADE`) | Opted in at `b64bb741`. The legacy record is frozen under `docs/legacy/` (`fa48afa8`). |
| Invariant registry (`docs/ade-invariant-registry.toml`) | 432 legacy IDD rules | **260 IDSE invariants + 7 assumptions (A1–A7)** | A new registry was built through **12 Define batches**. The legacy registry is frozen at `docs/legacy/idd-invariant-registry.toml` with **482** entries (432 + 50 added in this span before the freeze, none removed). All 482 have dispositions: **414 migrated / 52 merged / 16 excluded / 0 pending**. |
| Crates (workspace members) | 12 | **12** | No change. The `members` list is identical at both refs. |
| CI gates (`ci/ci_check_*.sh`) | 255 | **286** | **+31 new, 33 modified, 0 removed.** Two non-gate CI files were added: `ci/idse_legacy_carried.py` and `ci/store-semantics-surface.lock`. |
| New source modules (`crates/**/src/*.rs`) | — | **+10** | 8 BLUE (`ade_ledger` ×7, `ade_crypto` ×1), 1 BLUE test-only module (`ade_codec`) and 1 RED (`ade_node`). There are also 21 new integration-test files and 14 new block/UTxO fixtures. |
| `ade_core` (consensus authority) | — | **+1,301 / −0, 1 file** (`consensus/era_schedule.rs`) | Purely additive: the era-aware wall-clock→slot authority and the bootstrap-bound timing authority (LIVE-2b/2c). |
| Tests (`#[test]` / `#[tokio::test]`, approx) | ~3,320 | **~3,663** | ≈ +343. |
| `STORE_SEMANTICS_VERSION` | (did not exist) | **13** | Introduced at 1 in `ade_ledger::store_semantics` (P6, `e9d9db46`). Later slices raised it, for example to 8 at LV-3 (`cb65ec6e`), 12 at LV-15b-0 steps 3–5 (`2b0035d7`) and 13 at the LV-15b-0 pool-set fix (`0ddb36d1`). `e563ad94` added a semantics-neutral lock entry at 13 without bumping the version. |

### The span in fourteen bands (oldest → newest)

| # | Range (oldest … newest) | Commits | Cluster / theme | IDSE invariants carried forward |
|---|---|---|---|---|
| 1 | `d036f8e2 … df1efae2` | 4 | Grounding regen at the LIVE-FORGE-HARDENING close; `consensus_stream_replay` corpus refreshed for the rolling-nonce model; S1 forge-path rollback guard registered | SAF-017 |
| 2 | `86e29110 … b00216c8` | 5 | LIVE-WIRE-LIVENESS: keep-alive under downstream backpressure (S1); reconnect of an established live feed (S2) | LIV-008, ORD-006, LIV-009, LIV-007 |
| 3 | `18875f49 … 3a97ca9b` | 5 | ACCUMULATOR-REFOLD-BOUND: post-rollback accumulator refold bounded to a settled rewind point; forge-tick `skip_reason` | REC-001, DET-009, SAF-006, LIV-001 |
| 4 | `0c95d1ad … c2257080` | 15 | EVIEW-R1/R2 (a refold re-seals frozen leadership byte-identically) and LIVE-REFOLD-THRASH RF-1 (settled-rewind triple integrity; the rewind survives recovery) | VAL-023, DET-009, DUR-002, LIV-001 |
| 5 | `18aad6cf … 3ebdf83f` | 19 | PREPROD-ENTRY-AUTHORITY P1–P6: `psFutureStakePoolParams` grammar, venue era-schedule epoch detection, epoch-agreement invariant, mainnet-constant containment, store-semantics version gate, self-describing replay divergence, per-venue differential | VAL-025, VAL-012, EVO-005, PRC-005, OBS-004, PRC-004 |
| 6 | `47cfe76a … 7d6f4421` | 15 | PREPROD-NONCE-1/2/3 (eta0 commitment taken at the final candidate freeze; bridge-boundary differential) and SLICE-P2 (durably representable reduced projection) | VAL-022, PRC-004 |
| 7 | `937b2ab7 … 8b41b9e3` | 2 | CRE-S7: enact `minPoolCost` (Conway PPU key 16) | — |
| 8 | `732646d8 … 946185e8` | 19 | LIVE-2 / LIVE-2b / LIVE-2c: a leadership authority that cannot answer is not "not leader"; canonical era-aware slot authority in `ade_core`; forge timing bound to the store; one forge slot authority, no vanishing tick | DET-019, OBS-001 |
| 9 | `affb355d … cb751374` | 15 | LIVE-2c B6: per-pass co-advance cost census; `slot→hash` index in ChainDb; stop re-attempting a known-failed boundary (co-advance 115 s → 0.3 s) | — |
| 10 | `16e0a144 … fcbabb67` | 3 | B12 census: the +1 tip offset is benign observation order (762/762) | — |
| 11 | `941f98a2 … 8b9e9e38` | 22 | BND: typed boundary-stall vs apply-failure (BND-1); Cardano's phase-2-invalid collateral-only UTxO effect (2a); resolved collateral balance (2b); accumulator `Phase2Invalid` transition (2c); UTxO authority retains spent collateral (2d) | SAF-005, VAL-016, CON-003, DUR-011 |
| 12 | `f544fad0 … e9db66b1` | 3 | B12 slice: the followed-peer-tip signal records a tip the peer SERVED and Ade durably admitted | SAF-013 |
| 13 | `d5d26c1c … a3cef7f7` | 76 | LEADERVALUE census, then LV-1 … LV-17: sigma denominator, seed-boundary reward owner, owner stake, snapshot fidelity, donation pot, co-advance retry, relay-loop gate determinism, native-script threshold/depth/witness verdicts, every supplied witness verifies, UTxO-free admission obligations, frozen ratification record, durable FindIntersect ancestry, LV-17 opened | VAL-024, VAL-029, VAL-027, VAL-020, STR-005, VAL-028, CON-005, LIV-002, VAL-017, SEC-009, SEC-010, SEC-008, VAL-018, VAL-019, VAL-021, VAL-030, REC-005, VAL-055 |
| 14 | `fa48afa8 … 74c67670` | 19 | **IDSE migration.** M0: relocate and opt in. M1: twelve Define batches. M2: carry tests, CI scripts and code loci over; retarget legacy-registry gates; restore two broken gates; re-evidence at `e563ad94`; correct 14 re-evidence records (`74c67670`) | all 260 (registry built) |

The band counts are exact and sum to 222.

---

## 1. Commit Log

Verbatim from `git log --oneline --no-merges 1e4896eb..HEAD`, newest first. **222 commits, no merges.** The Type column is the commit-subject prefix. `define` is the IDSE Define commit format (`Define: …`, idse.md §10), not a conventional-commits type. Prefix tally (exact): **`docs`×125, `feat`×45, `fix`×25, `define`×12, `test`×6, `chore`×6, `ci`×2, `perf`×1.** Every commit has a prefix, so none is unclassified.

| Hash | Type | Summary |
|------|------|---------|
| `74c67670` | chore | chore(idse): correct 14 re-evidence records (test-reference matching) |
| `08c7c3f5` | chore | chore(idse): re-evidence pass at e563ad94 (migration M2) |
| `e563ad94` | ci | ci: restore two gates the IDSE relocation broke |
| `225933b4` | ci | ci: retarget the legacy-registry gates to the IDSE registry |
| `233cf8ab` | chore | chore(idse): carry legacy tests, CI scripts and code loci into the IDSE registry |
| `3009610f` | define | Define: resources-release — VAL-067, STR-022, DET-024, SEC-035, LIV-010, DET-025, SAF-025, DET-026, PRC-013, SEC-036, SEC-037, PRC-014, PRC-015, PRC-016, PRC-017, LIV-011, PRC-018, PRC-019, PRC-020, SEC-038, REC-008, PRC-021, PRC-022, PRC-023, 14 reinterpretations |
| `e61adb99` | define | Define: mithril-evidence — PRC-007, DET-022, STR-021, SEC-029, SEC-030, SEC-031, DUR-014, VAL-063, VAL-064, SEC-032, VAL-065, VAL-066, SEC-033, PRC-008, PRC-009, OBS-009, DET-023, PRC-010, SEC-034, PRC-011, PRC-012, A7 |
| `91e54254` | define | Define: pipeline-production — DET-020, VAL-056, SEC-026, SEC-027, VAL-057, VAL-058, SAF-020, VAL-059, SAF-021, SAF-022, DET-021, STR-018, SAF-023, VAL-060, STR-019, VAL-061, LIV-008, ORD-006, SEC-028, LIV-009, STR-020, OBS-006, OBS-007, SAF-024, OBS-008, VAL-062, DUR-013, SAF-013 redefined |
| `3ff46dcd` | define | Define: node — STR-013, STR-014, DET-019, SEC-019, LIV-004, VAL-050, LIV-005, SAF-012, STR-015, VAL-051, SAF-013, VAL-052, VAL-053, SAF-014, OBS-001, OBS-002, VAL-054, SAF-015, STR-016, REC-006, SEC-020, REC-007, SAF-016, OBS-003, SEC-021, SEC-022, SEC-023, LIV-006, SEC-024, SAF-017, LIV-007, OBS-004, VAL-055, OBS-005, SAF-018, SEC-025, SAF-019, STR-017 |
| `fd1e24d2` | define | Define: network — DET-016, SEC-016, VAL-042, VAL-043, VAL-044, EVO-006, VAL-045, SAF-010, VAL-046, DET-017, VAL-047, SEC-017, STR-012, SAF-011, VAL-048, DET-018, SEC-018, VAL-049, DUR-012, PRC-006 |
| `21e504b8` | define | Define: durability-recovery — REC-004, DUR-006, DUR-007, SEC-015, DUR-008, STR-011, DET-014, EVO-005, PRC-005, DUR-009, DUR-010, VAL-039, DUR-011, REC-005, VAL-040, DET-015, VAL-041, A6 |
| `e10a8aea` | define | Define: consensus-inputs — VAL-034, VAL-035, SEC-014, DUR-004, VAL-036, DUR-005, SAF-008, VAL-037, DET-013, VAL-038, STR-010, SAF-009 |
| `00591e38` | define | Define: consensus — VAL-031, LIV-003, VAL-032, STR-008, SAF-007, DET-010, DET-011, STR-009, VAL-033, SEC-011, SEC-012, REC-003, DET-012, SEC-013, A5 |
| `87f5905d` | define | Define: epoch — VAL-020, STR-007, ORD-003, VAL-021, CON-004, VAL-022, SAF-004, ORD-004, SAF-005, DUR-001, VAL-023, VAL-024, ORD-005, VAL-025, REC-001, DET-009, SAF-006, LIV-001, DUR-002, DUR-003, REC-002, VAL-026, VAL-027, VAL-028, CON-005, VAL-029, PRC-004, LIV-002, EVO-004, VAL-030, A4 |
| `8b49f686` | define | Define: ledger — VAL-007, VAL-008, VAL-009, ORD-002, VAL-010, SEC-008, CON-002, VAL-011, STR-005, VAL-012, VAL-013, VAL-014, DET-008, VAL-015, VAL-016, CON-003, VAL-017, SEC-009, SEC-010, VAL-018, VAL-019, EVO-003, STR-006 |
| `8bc4db2e` | define | Define: codec-wire-crypto — VAL-001, VAL-002, VAL-003, VAL-004, VAL-005, VAL-006, STR-004, EVO-002, SEC-003, SEC-004, SEC-005, SEC-006, SEC-007, DET-007, PRC-003, A3; redefines SEC-001 |
| `e218bc41` | define | Define: foundations — DET-001, DET-002, DET-003, DET-004, DET-005, DET-006, STR-001, STR-002, STR-003, SAF-001, SAF-002, SAF-003, EVO-001, PRC-001, PRC-002, SEC-001, SEC-002, CON-001, ORD-001, A1, A2 |
| `b64bb741` | chore | chore(idse): opt Ade into IDSE v3.0 (strict) |
| `fa48afa8` | chore | chore(idse): move the legacy IDD record under docs/legacy |
| `a3cef7f7` | test | test(runtime): LV-16 -- same-cursor recovery comparators with built-in sensitivity checks |
| `e4fa7f00` | docs | docs(lv): SLICE LV-17 opened -- certified accumulator recovery across rollback |
| `352bf783` | fix | fix(node): LV-16 -- a recovered node offers its durable ancestry in FindIntersect |
| `fe44ae21` | docs | docs(lv): SLICE LV-16 -- DC-PUMP-02 TipUpdate kept; single-point callers; checkable same-point comparisons |
| `cf10839c` | docs | docs(lv): SLICE LV-16 -- a recovered node finds its common ancestor with the peer |
| `0ddb36d1` | fix | fix(ledger): LV-15b-0 -- frozen SPO distribution keeps cardano's pool set; store semantics 13 |
| `0300fd3c` | docs | docs(lv): SLICE LV-15b-0 -- CE-9 at 305 finds the pool membership defect; 322 reference regenerated |
| `15105624` | docs | docs(lv): SLICE LV-15b-0 -- 322 stays an open adverse case; step 6 evidence rules |
| `2b0035d7` | feat | feat(ledger): LV-15b-0 steps 3-5 -- frozen ratification record, store semantics 12, bootstrap import |
| `277eb39f` | docs | docs(lv): SLICE LV-15b-0 -- record shape, import on real bytes, CE-9 reference, mutations |
| `29adcf95` | docs | docs(lv): SLICE LV-15b-0 -- step 2 is live execution; lands only with store 12 |
| `a1d36385` | feat | feat(ledger): LV-15b-0 step 2 -- delegation lifecycle, frozen distributions, decide/apply split |
| `d07e09cf` | docs | docs(lv): SLICE LV-15b-0 -- record shape and application follow proposalsApplyEnactment |
| `b94d31e8` | docs | docs(lv): SLICE LV-15b-0 -- native reproduction, lifecycle reference rules, recovery equivalence |
| `dfc3a42c` | docs | docs(lv): LV-15b-0 step 1 census -- every voting-stake difference has a named cause |
| `45183daa` | docs | docs(lv): SLICE LV-15b-0 -- frozen distributions are evidence only; census by cause |
| `e7c88eab` | docs | docs(lv): SLICE LV-15b-0 -- ratification consumes the authority frozen at the start of the epoch |
| `98d3c4fb` | docs | docs(lv): CENSUS LV-15b section 9 -- the Mithril snapshot carries a finished ratification result |
| `3fae80d3` | test | test(ledger): LV-15b ratification timing -- Ade enacts a late vote one epoch early |
| `978dd469` | docs | docs(lv): CENSUS LV-15b -- which protocol parameters admission may trust, and when |
| `e51da2f4` | docs | docs(lv): LV-15a results -- 997,827 txs censused clean; live restart at peer tip |
| `bbde0036` | fix | fix(ledger): LV-15a -- admission obligations needing neither UTxO nor pparams |
| `b424aa17` | docs | docs(lv): SLICE LV-15a -- admission obligations needing neither UTxO nor pparams |
| `59ad7fa1` | docs | docs(lv): CENSUS LV-15 decision -- full UTxO is the single admission authority |
| `a772e04f` | docs | docs(lv): CENSUS LV-15 -- what the block path decides, against Conway LEDGER |
| `64964b4b` | docs | docs(lv): LV-14 results -- 1,028,987 supplied witnesses verified; live restart follows |
| `df057072` | fix | fix(ledger): LV-14 -- every supplied witness signature verifies |
| `2b7468df` | docs | docs(lv): SLICE LV-14 -- every supplied witness signature verifies |
| `7d933a9d` | docs | docs(lv): LV-13 scope -- reduced mode closes only the UTxO-free part |
| `fb576e9e` | docs | docs(lv): LV-13 results -- zero census findings over 991,602 txs; live restart follows |
| `e6910d5a` | fix | fix(ledger): LV-13 -- native-script witness verdicts decide transaction validity |
| `69934446` | docs | docs(lv): SLICE LV-13 -- native-script witness verdicts decide transaction validity |
| `6435e3ab` | docs | docs(lv): LV-12 results -- deep native scripts on a 256 KiB thread; live restart follows |
| `a82d11b8` | fix | fix(codec): LV-12 -- native scripts and CBOR skipping keep their work on the heap |
| `4ae299cd` | docs | docs(lv): SLICE LV-12 -- native scripts and CBOR skipping do not depend on thread stack |
| `25a59869` | docs | docs(lv): LV-11 results -- the stopped store resumes past block 5,184,855; no store bump |
| `a4610e16` | fix | fix(ledger): LV-11 -- script_n_of_k thresholds are signed int64 end to end |
| `091ee63f` | docs | docs(lv): SLICE LV-11 -- native-script thresholds keep Cardano's signed 64-bit meaning |
| `09e42263` | docs | docs(lv): LV-10 census -- the node stopped on a negative native-script threshold Ade decodes as unsigned |
| `bffdfa21` | docs | docs(lv): SLICE LV-10 -- census of the block-body decode that stopped the node at 313 |
| `f6f2cf26` | docs | docs(lv): LV-8 live record postscript -- the same run later failed closed on an unrelated body decode |
| `cc8a5a5f` | fix | fix(ci): LV-9 -- the relay-loop containment gate reads the loop body from a here-string |
| `f2d05149` | docs | docs(lv): SLICE LV-9 -- the relay-loop containment gate gives the same answer every run |
| `d226e33d` | docs | docs(lv): DC-EPOCH-46 enforced -- live, a fresh bootstrap reaches epoch 310 with no restart |
| `6b5c34b9` | fix | fix(node): LV-8 -- a retry target must be in the accumulator's own epoch; the boundary edge is pinned |
| `4a37624c` | fix | fix(node): LV-8 -- a co-advance pass retries once when only the checkpoint was behind, and halts otherwise |
| `b813de72` | docs | docs(lv): SLICE LV-8 -- a co-advance pass never leaves leadership stale only because the checkpoint was behind |
| `f7c7c7ad` | docs | docs(lv): LV-7 census -- LeadershipEpochNotSealed is a skipped crossing; the walk's resolver sits behind it |
| `73fda1d1` | docs | docs(lv): DC-EPOCH-45 enforced -- live, the treasury equals cardano's after 305->306 |
| `fec37fb9` | fix | fix(ledger): LV-6 -- treasury donations are held in one pot and moved to the treasury once, at the boundary |
| `5f887e7e` | docs | docs(lv): LV-6 entry obligations answered -- the seed's donation pot is read as 0, the persistence surface is named |
| `8854651c` | docs | docs(lv): LV-6 design approved; cardano's boundary order quoted -- withdrawals first, donations after |
| `0b62cbf7` | docs | docs(lv): SLICE LV-6 -- a treasury donation is held in one pot and moved to the treasury once, at the boundary |
| `a011ec94` | docs | docs(lv): DC-EPOCH-41 enforced -- every reward balance equals cardano's on the real seed-boundary crossing |
| `2bb47336` | docs | docs(lv): LV-5 findings -- the 1 ADA is two epoch-305 treasury donations Ade never moves into the treasury |
| `7596bd65` | docs | docs(lv): SLICE LV-5 -- find where Ade's treasury first leaves cardano's, before touching any arithmetic |
| `bc828c57` | docs | docs(lv): close LV-4 -- DC-EPOCH-40/43/44 enforced; the 1 ADA treasury gap becomes LV-5 |
| `76e77411` | docs | docs(lv): LV-4 live -- every pool equal at both boundaries, reserves exact; 1 ADA treasury gap remains |
| `19387e86` | feat | feat(ledger): LV-4 -- a stake snapshot keeps whole credentials and frozen pool parameters |
| `4dfa6b0c` | docs | docs(lv): LV-4 entry obligations answered -- the seed snapshot matches cardano field for field; one more import defect found |
| `1e4db2b1` | docs | docs(lv): SLICE LV-4 -- a stake snapshot keeps what cardano's keeps: whole credentials and frozen pool parameters |
| `878cb7e5` | docs | docs(lv): the boundary after the seed -- fees confirmed live, and a retired pool's last rewards go unpaid |
| `d7de0fbd` | feat | feat(ledger): LV-2 -- the seed boundary's reward has an owner; "not computed yet" is no longer "empty" |
| `67ba3e3c` | docs | docs(lv): LV-2 entry obligations all answered -- the fee pot is exact and the replay path gets a rule |
| `cb65ec6e` | feat | feat(ledger): LV-3 -- an owner's stake counts for a pool only when it is delegated to that pool |
| `99077bb1` | docs | docs(lv): SLICE LV-3 -- an owner's stake counts for a pool only when it is delegated to that pool |
| `8c1328df` | test | test(lv): LV-2 E1 -- the native seed reward matches cardano for 11,050 of 11,056 credentials |
| `c5a2ec69` | docs | docs(lv): SLICE LV-2 -- "not computed yet" is not "computed, and empty" |
| `029e082c` | docs | docs(leadervalue): CE-LV1-9 RUN and NOT MET -- the halt is a missing first-boundary reward update |
| `b502bd5c` | feat | feat(ledger): LV-1 -- the sigma denominator stops being a sum of the pools that survived a filter |
| `2880cb96` | docs | docs(lv): SLICE LV-1 -- the sigma denominator is a snapshot fact, not a sum of survivors |
| `4f3cd689` | docs | docs(leadervalue): ORACLE EXTRACTION -- the sigma denominator is folded over the STAKE map, never the pool map |
| `dc14d6a6` | docs | docs(leadervalue): correct the census record -- the anomaly is ONE membership entry |
| `68bf2a3e` | docs | docs(leadervalue): WITHDRAW two claims -- the census answer was one command away |
| `88264dca` | feat | feat(evidence): the seed-window gap is code, not data -- and the oracle for it is gone |
| `d5d26c1c` | feat | feat(evidence): LEADERVALUE census -- the FIRST native leadership freeze copies the bootstrap stake |
| `e9db66b1` | docs | docs(b12): CE-B12-10 BLOCKED -- and the blocker is not the one that was recorded |
| `96c7af68` | feat | feat(node): B12 -- the peer that SERVED us a block has demonstrably got it |
| `f544fad0` | docs | docs(b12): SLICE B12 -- the signal records what a peer SAYS, not what it PROVED |
| `8b9e9e38` | docs | docs(bnd): retire the BND handoffs -- the question is answered, the precondition is discharged |
| `ba2e95e9` | docs | docs(bnd): BND-2d addendum -- the accumulator crossed the NEXT boundary too |
| `f19be266` | docs | docs(bnd): BND-2d LIVE BAR MET -- the retention resolves what the authority already spent |
| `ef6f06e0` | chore | chore(evidence): retire the v5 run store at the v6 bump |
| `56e0a4e4` | feat | feat(ledger): BND-2d -- the UTxO authority retains what it destroys |
| `3422dfe1` | docs | docs(bnd): SLICE BND-2d -- the positioning contract, and the design it selects |
| `eb55a637` | fix | fix(registry): the three BND rules took ids that were already assigned |
| `c3e73b1a` | docs | docs(bnd): next entry point -- answer the positioning contract before picking a design |
| `4171d2ac` | docs | docs(bnd): BND-2c live bar FAILED -- the resolver cannot see a spent collateral input |
| `737f5881` | fix | fix(ci): reconcile the store-semantics lock -- the v3->v4 bump skipped its own gate |
| `390c8191` | feat | feat(ledger): BND-2c -- the accumulator applies Cardano's Phase2Invalid transition |
| `d2673b20` | docs | docs(bnd): SLICE BND-2c -- the accumulator's exact phase-2-invalid transition |
| `89dd1aac` | feat | feat(ledger): BND-2b -- the UTxO authority resolves, the accumulator consumes one scalar |
| `7badddee` | docs | docs(bnd): SLICE BND-2b -- the UTxO authority resolves, the accumulator consumes one scalar |
| `1ac85759` | docs | docs(bnd): BND-2a CLOSED -- fresh v4 bootstrap proves all six live criteria |
| `d619ebda` | feat | feat(ledger): BND-2a -- phase-2-invalid txs get Cardano's collateral-only UTxO effect |
| `c3bd7e0d` | docs | docs(bnd): SLICE BND-2a -- one authoritative UTxO effect, validity-aware |
| `3092ce51` | docs | docs(bnd): BND-2 existing-code census -- the UTxO/stake path has NO phase-2-invalid semantics |
| `ea15efc9` | docs | docs(bnd): BND-2 ORACLE EXTRACTION -- the phase-2-invalid transition, from cardano-ledger |
| `3505c0c6` | feat | feat(accumulator): BND-1 -- a boundary stall and an apply failure stop being one state |
| `ad5e0a5f` | docs | docs(bnd): SLICE BND-1 -- a boundary stall and an apply failure stop being one state |
| `941f98a2` | feat | feat(ledger): BND CENSUS -- 130,350,133 is NOT a boundary; the withdrawal limb fired |
| `fcbabb67` | feat | feat(node): B12 CENSUS -- the +1 is (a) benign observation order, measured 762/762 |
| `ed5cab43` | docs | docs(b12): capture the gate's own VERDICT alongside the tuple it decided on |
| `16e0a144` | docs | docs(b12): SUPERSEDE the candidate fix -- B12 is a gate to investigate, not one to weaken |
| `cb751374` | docs | docs(b6): the fix VALIDATED live -- 0 ForgeTicks -> 449, B12 now the sole barrier |
| `a42c686d` | fix | fix(node): stop re-attempting a known-failed boundary every pass -- co-advance 115s -> 0.3s |
| `6fabd089` | feat | feat(node): B6 ROOT CAUSE -- a FAILING boundary crossing, retried at full cost every pass |
| `686c961f` | feat | feat(node): decompose the co-advance -- the index fix was only ~25% of it |
| `1175ba20` | fix | fix(chaindb): backfill the slot->hash index on open -- the fallback was 8x SLOWER than the code it replaced |
| `82b2e8f3` | perf | perf(chaindb): a slot->hash index, so reads cost what they RETURN not what the store HOLDS |
| `aeee4974` | docs | docs(b6): the census ran and answered -- hypothesis C, a FIXED per-pass cost |
| `ef310eba` | feat | feat(node): B6 census instrument -- one pass, four competing hypotheses |
| `1ae5c1ea` | docs | docs(live2c): the store snapshot is sparse -- 8.8 GB actual, 12.8 GB apparent, 21 GB free |
| `06f8fae0` | docs | docs(live2c): the B6 census is ONE time-attribution pass, and "same store" is a moving target |
| `b365d056` | docs | docs(live2c): pin the B6 entry point -- census first, and two obligations that must not merge |
| `b5589ad9` | docs | docs(live2c): a starved loop derives a STALE slot, and it will look like this fix regressing |
| `99e4f5e2` | docs | docs(live2c): record the post-reorg refold COST measured in run 6 |
| `1fe28c76` | docs | docs(live2c): run 6 refutes the run-5 reading of B6, and the claim is withdrawn |
| `affb355d` | docs | docs(live2c): the LIVE preprod result -- 19-day slot defect closed, and B6 is not merely transient |
| `946185e8` | docs | docs(live2): correct LIVE-2b's B11 attribution and sync the 2c CE tables |
| `5bc7f7fe` | docs | docs(registry): DC-NODE-45/46 -- one forge slot authority, no vanishing tick |
| `747b01ae` | feat | feat(node): ONE forge slot authority, and no admitted tick may disappear (LIVE-2c parts 2+3) |
| `8dd36d1c` | feat | feat(consensus): bind the forge timing calendar to the STORE, not the operator (LIVE-2c part 1) |
| `3b099e09` | docs | docs(live2c): pin the ACTIVATION contract, and correct two findings by measurement |
| `68e62c78` | docs | docs(live): sync the LIVE-2c CE table with what is actually closed |
| `8cf14529` | feat | feat(consensus): anchor the timing domain to a BOOTSTRAP FACT, not the clock (LIVE-2c) |
| `352dfb95` | feat | feat(consensus): DerivedTimingAnchor -- a compact timing projection WITH lineage (CE-L2c-13) |
| `f050f472` | test | test(consensus): make the Conway-only scope boundary MECHANICAL (CE-L2c-12) |
| `e25bfcdb` | docs | docs(live): pin the Conway scope boundary in LIVE-2c -- calendar geometry, not era support |
| `a8bb1ac9` | docs | docs(consensus): classify slot_at as BLUE; open LIVE-2c forge-slot wiring |
| `31f7c754` | feat | feat(consensus): canonical era-aware wall-clock -> slot authority (LIVE-2b) |
| `7e2c66ee` | docs | docs(live): LIVE-2b root cause SELECTED -- the forge slot ignores Byron's 20s slots |
| `d30605ec` | feat | feat(node): LIVE-2b discriminator -- it named the branch, and the answer was neither candidate |
| `eeacfde0` | docs | docs(live): LIVE-2b ForgeTick reachability -- the branch table, read from code |
| `4c03592a` | fix | fix(forge): close the CANNOT-ANSWER collapse on the AUTHORITATIVE node path (LIVE-2) |
| `bef325bb` | docs | docs(live): record the CE-L2-6 finding -- the gap was evidentiary, not diagnostic |
| `4985a59b` | fix | fix(forge): an authority that CANNOT ANSWER must not read as not-leader (LIVE-2, CE-L2-6) |
| `732646d8` | docs | docs(live): open LIVE-2 forge readiness; release the v2 ade-preprod-p2 store |
| `8b41b9e3` | feat | feat(governance): enact minPoolCost (Conway key 16) -- CRE-S7, all 8 CEs |
| `937b2ab7` | docs | docs(governance): open CRE-S7 with the ratified action MEASURED against cardano-node |
| `7d6f4421` | docs | docs(epoch): close CE-N2-5 and PREPROD-NONCE-3; seal the next preprod blocker (CRE-S7) |
| `2367f170` | fix | fix(ledger): make the reduced projection durably representable (SLICE-P2, CE-P2-1..6) |
| `0bfefdce` | docs | docs(ledger): diagnose SLICE-P2 -- it is two defects wearing one error |
| `b2546f5a` | feat | feat(epoch): bind the bridge boundary to the FINAL eta0 (PREPROD-NONCE-2, CE-N2-4) |
| `6c65cb39` | docs | docs(epoch): resolve the CE-N2-4 bridge-nonce design question (PREPROD-NONCE-2) |
| `41484298` | docs | docs(node): record the PREPROD-NONCE-2 stopping point and the binding next-session order |
| `0481db1b` | feat | feat(ledger): add the typed bridge-eta0 finality decision (PREPROD-NONCE-2, CE-N2-4 step 1) |
| `81df0bac` | feat | feat(node): refuse a non-final bridge eta0 commitment (PREPROD-NONCE-2, CE-N2-3) |
| `d4e534bf` | docs | docs(node): narrow PREPROD-NONCE-2 to a bootstrap-bridge commitment slice |
| `368a8685` | docs | docs(node): PREPROD-NONCE-2 -- reference-prove eta0(305) against cardano-node |
| `6960bf12` | docs | docs(node): open PREPROD-NONCE-2 -- epoch activation waits for candidate-freeze finality |
| `cb68e333` | docs | docs(node): PREPROD-NONCE-1 confirmed on a fresh store -- the mismatch localizes to ONE operand |
| `d115cc42` | docs | docs(node): PREPROD-NONCE-1 root cause PROVEN -- the eta0 commitment is taken at the seed point |
| `55805c32` | feat | feat(node): emit the first-boundary nonce operands (PREPROD-NONCE-1 instrumentation) |
| `47cfe76a` | docs | docs(node): open PREPROD-NONCE-1 -- preprod 304->305 eta0 boundary equivalence |
| `3ebdf83f` | docs | docs(evidence): retire the pre-P6 preprod store with its forensic record |
| `221031cb` | test | test(ledger): prove epoch semantics PER VENUE, not from a mainnet corpus (P6-S5, DC-EPOCH-37) |
| `734ac76e` | feat | feat(node): make a warm-start replay divergence self-describing (P6-S4, DC-NODE-44) |
| `e9d9db46` | feat | feat(store): enforce the authority-semantics version gate -- strict, no stamp tool (P6) |
| `2359963a` | docs | docs(store): scope P6 (P4-S2) -- the store-semantics version gate |
| `45c2b942` | feat | feat(ledger): enforce the epoch-agreement invariant and contain the mainnet constants (P5) |
| `f2a2a8d9` | docs | docs(ledger): open P5 -- the epoch-agreement invariant and venue-constant containment |
| `e1de7a2e` | feat | feat(node): prove the P4 root cause -- warm-start replays a pre-P3 store under post-P3 ledger semantics |
| `6795964b` | feat | feat(node): instrument warm-start fingerprint mismatch; seal P4 -- a SIGKILL in the rollback window is unrecoverable |
| `05a2654f` | docs | docs: land two uncommitted corrections -- seed-import/forge split, and PO-S3-2 stale-pending fix |
| `5f2636c2` | fix | fix(ledger): bind epoch-boundary detection to the venue era schedule (P3) |
| `3bda3ade` | docs | docs(ledger): P3 -- prove the live path by probe; the era schedule is dropped at the last hop |
| `4b3341a6` | feat | feat(node): instrument capture refusal; seal P3 -- slot_to_epoch hardcodes MAINNET constants |
| `42fa8e64` | docs | docs(node): seal P2 -- preprod reaches tip then fails closed capturing a REDUCED state |
| `6a581493` | docs | docs(ledger): P1 fixed; record that a Mithril snapshot is only bootstrappable from part of an epoch |
| `1b49007e` | fix | fix(ledger): decode psFutureStakePoolParams in its own CDDL grammar (P1) |
| `adcc6ddf` | docs | docs(ledger): P1 -- future_pools is AUTHORITATIVE, so the fix needs a real proof |
| `05753c85` | docs | docs(ledger): P1 root cause RESOLVED -- psFutureStakePoolParams keeps the certificate's CDDL form |
| `18aad6cf` | feat | feat(ledger): report the byte offset on a fixed-width decode mismatch; seal PREPROD-ENTRY P1 |
| `c2257080` | docs | docs(node): record that ARMING the settled bound may be rarer than discarding it was |
| `d4f19c87` | feat | feat(node): re-certify the settled rewind after a durable rollback (RF-1) |
| `218c6d41` | docs | docs(node): close EVIEW-R2 -- CE-R2-5 met live; negative control retained |
| `a6d584e2` | feat | feat(runtime): bind the settled rewind triple with a verified fingerprint (CE-RF-6) |
| `7ca03756` | docs | docs(node): RF-1 severity raised -- refold thrash is a DEMONSTRATED forge blocker |
| `db131647` | docs | docs(node): record the measured RF-1 refold-growth baseline from two live stores |
| `e9946576` | docs | docs(evidence): retire superseded run stores behind a retention rule (15G -> 87G free) |
| `237c7596` | docs | docs(node): open LIVE-REFOLD-THRASH RF-1 -- the bounded rewind is discarded one pass later |
| `80aac15c` | docs | docs(node): EVIEW-R2 CE-R2-4 met live -- refold re-seals byte-identically on preview |
| `1dc48e74` | fix | fix(node): position the reduced checkpoint on the boundary point before sealing leadership |
| `ebec6b34` | docs | docs(node): seal EVIEW-R2 -- a refold must re-seal frozen leadership byte-identically |
| `0fc2e478` | feat | feat(node): emit the eview activation record-vs-candidate diff; resolve EVIEW-R1 mechanism |
| `41f511eb` | feat | feat(node): typed emit-only recovery/refold trace (who reset the accumulator, and why) |
| `a23f5da3` | feat | feat(node): emit both compared tips on a DC-NODE-15 forge refusal |
| `0c95d1ad` | docs | docs(eview): seal EVIEW-R1 -- activation WAL vs refolded accumulator lineage defect |
| `3a97ca9b` | feat | feat(node): surface WHY a forge tick skipped (CN-NODE-04 skip_reason) |
| `6f014cf9` | docs | docs(registry): register the bounded-refold invariants as enforced (DC-EPOCH-26..31) |
| `c4f88e84` | test | test(node): close CE-AR-3 + CE-AR-5 for the bounded accumulator rewind |
| `8a81e6ee` | feat | feat(node): bound the post-rollback accumulator refold to a settled rewind point |
| `18875f49` | fix | fix(node): label a post-rollback accumulator refold distinctly from a fresh boundary crossing |
| `b00216c8` | docs | docs(registry): register wire-liveness invariants as enforced (DC-PUMP-05..10, DC-NODE-43) |
| `daa793db` | docs | docs(wire): record LIVE-WIRE-LIVENESS S2 live acceptance (CE-WL-5 + CE-WL-8) |
| `5cf45a6f` | fix | fix(node): reconnect an established live feed (LIVE-WIRE-LIVENESS S2) |
| `db9bde19` | fix | fix(runtime): keep-alive survives downstream backpressure (LIVE-WIRE-LIVENESS S1) |
| `86e29110` | docs | docs(wire): LIVE-WIRE-LIVENESS S1 slice doc -- keep-alive under backpressure |
| `df1efae2` | feat | feat(node): land S1 INV-FH-4 within-epoch forge-path rollback guard proof + DC-NODE-42 |
| `a9c21eab` | docs | docs(node): register LIVE-FORGE-HARDENING S1 rollback-follow (DC-NODE-28/29) |
| `bc83e20d` | fix | fix(testkit): refresh the consensus_stream_replay corpus for the ECA-B rolling-nonce model |
| `d036f8e2` | docs | docs(grounding): regenerate the four grounding docs at LIVE-FORGE-HARDENING close + advance baseline |

> **Commit-attribution note.** Under the Ade-only exception recorded in `CLAUDE.md` (granted 2026-11-08), commit messages carry a `Co-Authored-By:` model trailer. It covers commit messages only and does not affect this document.

---

## 2. New Modules

**No new crate.** The workspace `members` list is identical at `1e4896eb` and HEAD (12 crates), and no `Cargo.toml` was added. The span adds **10 new source modules** under `crates/**/src/`:

| Module | Color | Purpose | Key sub-paths / types | Added in (cluster/slice) |
|--------|-------|---------|---------------|--------------------------|
| `ade_ledger::collateral` | BLUE | The ADA a phase-2-invalid transaction consumes, computed over the **resolved** collateral UTxO entries. Mirrors `Cardano.Ledger.Babbage.Collateral` `collAdaBalance`. | `collateral_balance`, `CollateralBalanceError` | BND-2b (`89dd1aac`). VAL-016, CON-003 |
| `ade_ledger::governance::frozen` | BLUE | Ratification for boundary E→E+1 is decided from the authority frozen at the start of E (`ratify_frozen` → `RatificationRecord`) and applied only at E→E+1 (`apply_ratification_record`). | `FrozenRatificationInput`, `FrozenGovernanceView`, `FrozenPoolStake`, `RatificationSource`, `RatificationOutcome`, `UnenactableReason` | LV-15b-0 step 2 (`a1d36385`), steps 3–5 (`2b0035d7`). VAL-021, VAL-030 |
| `ade_ledger::replay_divergence` | BLUE | A self-describing warm-start replay divergence report that replaces the bare `FingerprintMismatch { expected, recovered }`. | `ReplayDivergenceReport` | P6-S4 (`734ac76e`). OBS-004 |
| `ade_ledger::rollback::intersection` | BLUE | Chain-sync intersection authority: which durable-ancestry points a recovering node offers in `FindIntersect`, and what the peer's reply means. The reply is captured input and is never treated as rollback authority. | `ChainPoint`, `AncestorHeader`, `IntersectionCandidates`, `IntersectionBound`, `IntersectionAncestryError` | LV-16 (`352bf783`). REC-005, VAL-055 |
| `ade_ledger::store_semantics` | BLUE | The authority-semantics version of a durable store, separate from its byte layout. Strict check, with no stamp or remediation tool. | `STORE_SEMANTICS_VERSION` (= 13 at HEAD), `check_store_semantics_version`, `AuthorityArtifact`, `FoundSemanticsVersion`, `StoreSemanticsVersionMismatch` | P6 (`e9d9db46`). EVO-005, PRC-005 |
| `ade_ledger::tx_validity::script_witnesses` | BLUE | The one authority for a Conway transaction's witness-script closure (`babbageUtxowTransition`), with reduced and exact evidence levels. | `ScriptsNeeded`, `ScriptPurpose`, `ReducedScriptVerdict`, `ExactScriptVerdict`, `NativeScriptVerdict`, `ScriptClosureFailures` | LV-13 (`e6910d5a`). SEC-010 |
| `ade_ledger::tx_validity::utxo_free` | BLUE | Admission obligations that need neither the UTxO nor protocol parameters (validity interval, network IDs, bootstrap attribute size, declared body size). Each equals Cardano's predicate or is weaker than it. | `check_utxo_free_tx`, `check_validity_interval`, `check_tx_networks`, `check_declared_body_size`, `UtxoFreeRejection` | LV-15a (`bbde0036`). VAL-018, VAL-019 |
| `ade_crypto::sha3` | BLUE | SHA3-256 (FIPS 202), used only for the bootstrap-witness key hash `Blake2b-224(SHA3-256(..))`. Adds the `sha3 = "0.10"` dependency. | `sha3_256` | LV-13 (`e6910d5a`) |
| `ade_codec::cbor::skip_differential_tests` | BLUE (test-only) | The pre-LV-12 recursive `skip_item`, kept verbatim as a differential reference for the new iterative skipper. | — | LV-12 (`a82d11b8`). SEC-009 |
| `ade_node::forge_timing` | RED | The committed venue timing registry. The store, keyed by the seed sidecar's genesis hash, selects the timing calendar; `--network` cannot. This is the one place the bootstrap-bound forge slot authority is established. | `venue_timing_history_for_genesis`, `establish_forge_timing_authority`, `GenesisTimingCrossCheck`, `ForgeTimingError` | LIVE-2c part 1 (`8dd36d1c`). DET-019 |

**New integration-test files (21):**
- `ade_ledger/tests/`: `bnd2a_phase2_invalid_utxo_effect`, `bnd2d_collateral_retention`, `lv1_sigma_denominator_authority`, `lv3_self_delegated_owner_stake`, `lv4_snapshot_fidelity`, `lv11_native_script_threshold`, `lv12_native_script_depth`, `lv13_native_script_witness_verdict`, `lv14_every_supplied_witness_verifies`, `lv15a_utxo_free_admission_obligations`. There are also 14 block/UTxO fixtures under `tests/fixtures/`.
- `ade_runtime/tests/`: `body_decode_census`, `frozen_ratification_census`, `leadership_distr_census`, `lv15b0_native_record_differential`, `lv15b0_real_pulser_import`, `lv16_recovered_store_equivalence`, `seed_boundary_reward_census`, `store_semantics_version_gate`, `treasury_donation_census`.
- `ade_node/tests/`: `b12_served_or_advertised_peer_tip`, `lv16_durable_intersection`.

> **Cross-reference (CODEMAP).** `docs/ade-CODEMAP.md` has been regenerated at `08c7c3f5`; the regeneration is in the working tree and not yet committed. It names 9 of the 10 new modules. The exception is the test-only `ade_codec::cbor::skip_differential_tests`, which is expected since CODEMAP does not catalog test modules.

---

## 3. Modules Modified

Per-crate diffstats over `1e4896eb..HEAD`. Two kinds of change are excluded as trivial: the IDSE relocation's comment-path rewrites (`docs/clusters/…` → `docs/legacy/clusters/completed/…`, from `fa48afa8`) and single-line touches.

| Module | Scope | Key changes |
|--------|-------|-------------|
| `ade_ledger` | **BLUE.** 83 files, +12,861 / −1,347 (7 new src modules, 10 new test files, 14 fixtures) | **Accumulator and boundary.** `epoch_accumulator.rs` (+1,222 / −204) gains: a typed boundary-stall vs apply-failure state (BND-1); a `Phase2Invalid` transition consuming the resolved collateral scalar (BND-2b/2c); a seed-boundary reward owner, where "not computed yet" is distinct from "empty" (LV-2); a treasury donation pot moved to the treasury once, at the boundary (LV-6); and the frozen ratification record (LV-15b-0). `rules.rs` binds epoch-boundary detection to the venue era schedule (P3). **Snapshot and leadership.** The sigma denominator is a snapshot fact (LV-1). Owner stake counts only for owners delegated to that pool (LV-3, store v8). Stake snapshots keep whole tagged credentials and frozen pool parameters (LV-4). The frozen SPO distribution keeps Cardano's pool set (LV-15b-0, store semantics 13). **Native Mithril decode.** `ledgerdb_state.rs` (+787 / −125) decodes `psFutureStakePoolParams` in its own CDDL grammar (P1), reports the byte offset on a fixed-width mismatch, and imports the snapshot's finished ratification result (LV-15b-0). **Governance.** `governance.rs` / `gov_cert.rs` / `snapshot/gov_state.rs` add `minPoolCost` enactment (CRE-S7; `ExecUnitsParamUpdate` → `EnactableParamUpdate`), the delegation lifecycle and the decide/apply split (LV-15b-0). **Tx validity.** Native-script `n_of_k` thresholds are signed int64 (LV-11, `scripts.rs`). Native-script witness verdicts decide validity (LV-13, `witness.rs`, `script_witnesses`). Every supplied vkey/bootstrap witness signature verifies (LV-14). UTxO-free admission obligations (LV-15a). **Store and recovery.** `store_semantics` (P6), `replay_divergence` (P6-S4), `rollback::intersection` (LV-16), the bridge-eta0 finality decision (`bootstrap_bridge.rs`, PREPROD-NONCE-2), and a durably representable reduced projection (SLICE-P2). Mainnet constants are contained (P5). |
| `ade_node` | **RED.** 22 files, +6,539 / −365 (1 new src module, 2 new test files) | `node_lifecycle.rs` (+3,340 / −170) carries most of the shell work: reconnecting an established live feed (LIVE-WIRE-LIVENESS S2); a bounded post-rollback refold with a distinct refold label (ACCUMULATOR-REFOLD-BOUND); a typed emit-only refold/recovery trace and the eview activation record-vs-candidate diff (EVIEW-R1); positioning the reduced checkpoint on the boundary point before sealing leadership (EVIEW-R2); re-certifying the settled rewind after a durable rollback (RF-1); warm-start fingerprint and P4 root-cause instrumentation; first-boundary nonce operands (PREPROD-NONCE-1); B6 co-advance decomposition and the known-failed-boundary short-circuit; BND census and BND-1/2c/2d wiring; one co-advance retry when only the checkpoint was behind, with the retry target pinned to the accumulator's epoch (LV-8); and durable ancestry in `FindIntersect` (LV-16). `node_sync.rs` / `produce_mode.rs` close the CANNOT-ANSWER → not-leader collapse on the authoritative path (LIVE-2) and add the B12 served-tip signal. `epoch_wire.rs` / `native_firstrun.rs` refuse a non-final bridge eta0 commitment and bind the bridge boundary to the final eta0 (PREPROD-NONCE-2). The seed-reward owner is threaded into activation (LV-2). `live_log/sched_event.rs` adds forge-tick `skip_reason`, both compared tips on a forge refusal, and the no-vanishing-tick accounting (LIVE-2c). The new `forge_timing` module is described in §2. |
| `ade_runtime` | **RED** (with GREEN-by-contract chaindb caches). 33 files, +6,410 / −457 (9 new test files) | `admission/wire_pump.rs` (+987 / −243): keep-alive survives downstream backpressure (LIVE-WIRE-LIVENESS S1), and the LV-16 intersection is wired. `chaindb/epoch_accumulator_store.rs` (+954): the settled-rewind triple with a verified fingerprint (CE-RF-6), bounded refold, the store-semantics gate, the frozen ratification record and the LV-1 snapshot. `chaindb/epoch_accumulator_advance.rs`: BND-1 typed stall, BND-2c/2d, LV-6 donations. `chaindb/persistent.rs`: a `slot→hash` index plus a backfill on open (B6 perf, `82b2e8f3` / `1175ba20`) and the semantics-version gate. `chaindb/reduced_utxo_checkpoint.rs`: resolved collateral and retained spent collateral (BND-2b/2d). `clock.rs` / `producer/coordinator.rs`: a single forge slot authority (LIVE-2c). `mithril_native_assembly.rs`: LV-2/LV-4/LV-6/LV-15b-0 import fields. |
| `ade_core` | **BLUE (consensus authority).** 1 file, +1,301 / −0 | `consensus/era_schedule.rs` changes additively only. Added: `slot_at` (canonical era-aware wall-clock → slot, LIVE-2b, `31f7c754`), `ScheduleCommitment` / `schedule_timing_commitment`, `DerivedTimingAnchor` (a compact timing projection with lineage, `352dfb95`), `VenueTimingSegment` / `VenueTimingHistory`, `BootstrapTimingBinding` / `BootstrapBoundTimingAuthority` (the timing domain anchored to a bootstrap fact, not the clock, `8cf14529`), `slot_start_time_ms`, and the error enums `SlotDerivationError`, `TimingAnchorError` and `TimingAuthorityError`. It also adds 21 `#[test]`s, including the mechanical Conway-only scope boundary (`f050f472`). No existing item was changed or removed. |
| `ade_types` | **BLUE.** 1 file, +170 / −84 | `allegra/script.rs`: `NativeScript` changes from a recursive enum to a post-order struct over `NativeScriptNode` (`All{arity}`, `Any{arity}`, `MOfN{threshold: i64, arity}`, …), plus `MalformedPostOrder`. These carry LV-11 (signed threshold) and LV-12 (depth-independent, heap-only work). |
| `ade_codec` | **BLUE.** 4 files, +427 / −51 | `cbor/mod.rs`: `skip_item` is now iterative (LV-12). `allegra/script.rs`: the post-order native-script decode with an int64 threshold (LV-11/12). The new test module is described in §2. |
| `ade_crypto` | **BLUE.** 7 files, +49 / −5 | New `sha3` module and dependency (LV-13). The remaining 4 files change only comment paths (relocation). |
| `ade_testkit` | **GREEN/test.** 30 files, +264 / −104 | `harness/snapshot_loader.rs` (+99 / −4): snapshot-loader support for the LV-4/LV-15b-0 fields. `consensus_stream_replay` corpus refreshed for the rolling-nonce model (`bc83e20d`). `ce3d_boundary_differential`, `epoch_oracle_comparison` and `conway_deposit_params_single_decoder` are adjusted. The other ~24 files change only comment paths (relocation). |
| `ade_core_interop` | test bins | **Trivial.** 12 files, +18 / −18: relocation comment-path rewrites only. |
| `ade_network`, `ade_plutus`, `ade_mem_diag` | — | Unchanged. |

> **BLUE was touched, including the consensus core.** `ade_core` grew by 1,301 lines with no deletions. Every added item is a new timing/slot authority; the nonce, header-validation and fork-choice authorities did not change. `ade_types::NativeScript` changed its shape (an enum became a post-order struct) to remove recursion on adversarial depth.

---

## 4. Feature Flags

**No feature-flag deltas.** No workspace `Cargo.toml` declares a `[features]` table at either ref (`git grep '^\[features\]'` is empty at `1e4896eb` and HEAD). There is no `#[cfg(feature = …)]` in `crates/` and no `compile_error!` coupling. The only manifest-level change is a new non-optional dependency, `sha3 = "0.10"`, in `ade_crypto/Cargo.toml` (LV-13). There is no flag coupling to report.

---

## 5. CI Checks

`git diff --name-status 1e4896eb..HEAD -- ci/` shows **31 `A` gates, 33 `M` gates and 0 `D`**. Gate count (`git ls-tree -r --name-only <ref> ci/ | grep -c ci_check_`): **255 → 286**. Two non-gate CI files were added: `ci/store-semantics-surface.lock` (P6, `e9d9db46`), the semantics-bearing surface lock read by `ci_check_store_semantics_lock.sh`, and `ci/idse_legacy_carried.py` (`225933b4`), the retarget helper that checks a legacy ID was carried into an active IDSE invariant that lists the gate.

### Cluster work before the migration (bands 1–13): 31 new gates, 3 materially modified

| Check | Status | IDSE invariant(s) (registry `ci_scripts`) | Cluster of origin | What it checks |
|-------|--------|-------------------------------|-------------------|----------------|
| `ci_check_wire_liveness.sh` | New | LIV-008, ORD-006, LIV-009 | LIVE-WIRE-LIVENESS (`b00216c8`) | Wire/pump cooperative liveness: keep-alive under backpressure, pump order, transport-only reconnect. |
| `ci_check_accumulator_refold_bound.sh` | New | REC-001, DET-009, SAF-006, LIV-001 | ACCUMULATOR-REFOLD-BOUND (`6f014cf9`) | A post-rollback accumulator refold rewinds to a settled point, not the bootstrap anchor. |
| `ci_check_eview_refold_reseal.sh` | New | VAL-023, DET-009 | EVIEW-R2 (`1dc48e74`) | A refold re-seals frozen leadership byte-identically. |
| `ci_check_settled_triple_integrity.sh` | New | DUR-002 | RF-1 / CE-RF-6 (`a6d584e2`) | The settled rewind triple is bound by a verified fingerprint. |
| `ci_check_settled_rewind_survives_recovery.sh` | New | LIV-001 | RF-1 (`d4f19c87`) | A bounded settled rewind survives the recovery pass after rollback. |
| `ci_check_epoch_agreement.sh` | New | VAL-025 | P5 (`45c2b942`) | The ledger epoch agrees with the venue schedule. |
| `ci_check_venue_constant_containment.sh` | New | VAL-012 | P5 (`45c2b942`) | Mainnet `SHELLEY_START_*` / epoch-length constants stay contained. |
| `ci_check_store_semantics_gate.sh` | New | EVO-005 | P6 (`e9d9db46`) | No authoritative store reader bypasses the semantics-version gate, and there is no "trust me" remediation. |
| `ci_check_store_semantics_lock.sh` | New | PRC-005 (+ STR-005, VAL-020/021/024/027/028/029/030, CON-005) | P6 (`e9d9db46`) | A semantics-bearing source change must bump `STORE_SEMANTICS_VERSION` or record a neutral entry in the lock. |
| `ci_check_recovery_fault_self_describing.sh` | New | OBS-004 | P6-S4 (`734ac76e`) | A recovery divergence stays self-describing. |
| `ci_check_venue_differential.sh` | New | PRC-004 | P6-S5 (`221031cb`) | Epoch semantics are proven per venue, not from a mainnet-shaped corpus. |
| `ci_check_bridge_nonce_freeze_differential.sh` | New | VAL-022, PRC-004 | PREPROD-NONCE-2 (`b2546f5a`) | The nonce / candidate-freeze differential covers seed position × venue. |
| `ci_check_forge_slot_authority.sh` | New | DET-019, OBS-001 | LIVE-2c (`747b01ae`) | One forge slot authority, and no admitted forge tick may disappear. |
| `ci_check_bnd_typed_stall_cause.sh` | New | SAF-005 | BND-1 (`3505c0c6`) | A boundary stall and a within-epoch apply failure are distinct states. |
| `ci_check_phase2_invalid_utxo_effect.sh` | New | VAL-016 | BND-2a (`d619ebda`) | The phase-2-invalid UTxO effect is collateral-only, derived in one place, with validity-blind consumers. |
| `ci_check_collateral_balance_resolver.sh` | New | VAL-016, CON-003 | BND-2b (`89dd1aac`) | `collAdaBalance` is computed over the resolved collateral entries. |
| `ci_check_collateral_retention_positioning.sh` | New | DUR-011 | BND-2d (`56e0a4e4`) | The UTxO authority retains spent collateral on the accumulator's behalf. |
| `ci_check_followed_peer_tip_served_evidence.sh` | New | SAF-013 | B12 (`96c7af68`) | The followed-peer-tip signal reports the strongest evidence: the advertised tip, or a served tip that was durably admitted. |
| `ci_check_sigma_denominator_authority.sh` | New | VAL-024 | LV-1 (`b502bd5c`) | The leader-check sigma denominator is a snapshot fact. |
| `ci_check_owner_stake_authority.sh` | New | VAL-027 | LV-3 (`cb65ec6e`) | Owner stake counts only for owners delegated to that pool, and it has one definition. |
| `ci_check_seed_reward_authority.sh` | New | VAL-020, VAL-029 | LV-2 (`d7de0fbd`) | The seed→seed+1 reward has exactly one owner, selected by a typed fact rather than an empty map. |
| `ci_check_snapshot_fidelity.sh` | New | STR-005, VAL-020, VAL-027, VAL-028 | LV-4 (`19387e86`) | Stake snapshots keep tagged credentials and frozen pool parameters. |
| `ci_check_donation_pot.sh` | New | CON-005 | LV-6 (`fec37fb9`) | Treasury donations are held in one pot and moved once, at the boundary. |
| `ci_check_coadvance_retry.sh` | New | LIV-002 | LV-8 (`4a37624c`) | A co-advance pass retries once when only the checkpoint was behind, and halts otherwise. |
| `ci_check_native_script_threshold_signed.sh` | New | VAL-017 | LV-11 (`a4610e16`) | `script_n_of_k` thresholds are signed int64 end to end. |
| `ci_check_native_script_depth_independent.sh` | New | SEC-009 | LV-12 (`a82d11b8`) | Native scripts and CBOR skipping do not depend on thread stack depth. |
| `ci_check_native_script_witness_verdict.sh` | New | SEC-010 | LV-13 (`e6910d5a`) | One authority decides a Conway tx's witness-script closure. |
| `ci_check_every_supplied_witness_verifies.sh` | New | SEC-008 | LV-14 (`df057072`) | Every supplied vkey/bootstrap witness signature verifies over the body hash. |
| `ci_check_utxo_free_admission_obligations.sh` | New | VAL-018, VAL-019 | LV-15a (`bbde0036`) | Validity-interval, network and body-size checks that need neither UTxO nor pparams. |
| `ci_check_frozen_ratification_authority.sh` | New | VAL-021, VAL-030 | LV-15b-0 (`2b0035d7`) | Ratification for E→E+1 comes from the authority frozen at the start of E. |
| `ci_check_durable_intersection.sh` | New | REC-005, VAL-055, STR-019 | LV-16 (`352bf783`) | A recovered node offers its durable ancestry in `FindIntersect`. |
| `ci_check_eview_window_driver.sh` | Modified (+26 / −9) | DET-013 | BND-2d (`56e0a4e4`) | Extended for retained spent collateral in the reduced window driver. |
| `ci_check_forge_followed_tip_admission.sh` | Modified (+13 / −1) | VAL-051, SAF-013, VAL-052, VAL-053 | B12 (`96c7af68`) | Extended for the served-tip evidence. |
| `ci_check_node_run_loop_containment.sh` | Modified (+8 / −8) | DET-019, VAL-050, SAF-012, STR-015, SEC-025 | LV-9 (`cc8a5a5f`) | Reads the relay-loop body from a here-string so every run gives the same answer. |

The cluster-era commits `d7de0fbd` (LV-2) and `19387e86` (LV-4) also made code-level edits to `ci_check_bootstrap_rupd_fee_reduction.sh`, `ci_check_epoch_accumulator_no_utxo.sh` and `ci_check_snapshot_pool_set_inclusion.sh`. Those three were then retargeted in band 14.

### IDSE migration (band 14): 0 new gates, 30 modified

| Group | Gates | Commit | What changed |
|-------|-------|--------|--------------|
| **Retargeted to the IDSE registry (16)** | `bootstrap_rupd_fee_reduction`, `boundary_aligned_mark_capture`, `chainsync_findintersect_compat`, `conway_deposit_params_bootstrap`, `epoch_accumulator_no_utxo`, `epoch_accumulator_recovery`, `feed_leader_threshold_view`, `feed_tag24_unwrap`, `forge_successor_evolved_spine`, `frozen_leadership_authority`, `frozen_promotion_no_seed_window`, `poolreap_single_canonical`, `rollback_materialize_eta0`, `served_chain_projection`, `snapshot_pool_set_inclusion`, `warmstart_eta0_overlay` | `225933b4` | Each gate used to assert a legacy ID's `status`. It now calls `ci/idse_legacy_carried.py`, which requires the ID to be migrated or merged, every target to be an active IDSE invariant, and the gate to be listed in a target's `ci_scripts`. Code-level assertions are unchanged. |
| **Rewritten for IDSE (4)** | `registry_unique_ids` (checks both the IDSE and the frozen legacy registry), `registry_code_locus_exists` (IDSE registry: unique IDs, resolvable `depends_on`, every enforcement locus and `ci_scripts` path exists; PRC-001), `genesis_replay_open_obligation` (genesis replay stays unclaimed: the legacy entry stays excluded under A2, and A2 still says Ade never replays historical eras), `producer_corpus_present` (follows the legacy production-corpus entry to PRC-023; no `supported` claim without passing evidence) | `225933b4` | Rewritten against the IDSE registry. |
| **Restored (1)** | `no_async_in_blue` | `e563ad94` | Now reads `core_paths` from `.idse-config.json` (identical paths) after the old config moved. `store-semantics-surface.lock` gained a neutral version-13 entry for the relocation's comment-path rewrite in two surface files, which also restores `ci_check_store_semantics_lock` and `ci_check_collateral_retention_positioning`. |
| **Path-only (9)** | `ba02_evidence_manifest_schema`, `operator_evidence_manifest_schema`, `rehearsal_manifest_schema`, `sync_evidence_manifest_schema`, `receive_paths_corpus_present`, `server_paths_corpus_present`, `utxo_admission_seam`, `utxo_fp_cache`, `utxo_pre_resolve` | `fa48afa8` | Document paths moved under `docs/legacy/`. No behavioral change. |

> **Known-red gates (pre-existing, not caused by the migration).** The re-evidence artifact `docs/evidence/idse-m2-reevidence.json` ran 259 registry-named CI scripts at `e563ad94`. **37 exited non-zero** (`ci[].rc != 0`), and every one already failed at `a3cef7f7`, before the migration began. The registry evidence diagnoses each as a stale or over-broad structural gate, not a demonstrated violation, that needs repair before it can serve as evidence. Where a retargeted gate is among them, its registry assertion passes and only its code-level assertion fails. The failing gates are: `admission_runner_closure`, `consensus_input_provenance`, `epoch_accumulator_recovery`, `eview_activation_predicate`, `eview_automatic_activation`, `eview_bootstrap_cert_state`, `eview_epoch_rebind`, `eview_forecast_crossing`, `eview_leadership_complete`, `eview_live_checkpoint`, `eview_pool_lifecycle`, `eview_seed_sidecar_v4`, `feed_leader_threshold_view`, `forge_purity`, `forged_durable_admit_via_pump`, `gov_proposal_capture`, `hash_uses_wire_bytes`, `keep_alive_wire_only`, `lifecycle_owner_uses_bootstrap_initial_state`, `live_fork_choice_apply`, `local_durable_forge_base`, `mempool_ingress_closure`, `native_firstrun_no_cli_seed`, `native_nonutxo_decode`, `no_haskell_fingerprint_equality`, `node_forge_single_epoch_fail_closed`, `node_mode_closure`, `node_path_fidelity`, `node_sync_via_pump`, `participant_forge_on_selected_head`, `post_switch_convergence_window`, `receive_detector_venue_split`, `recover_follow_wal_lineage`, `recovered_ledger_pparams_sourced`, `rollback_target_canonical_binding`, `warm_start_re_entry`, `warmstart_eta0_overlay`.

> **Cross-reference (TRACEABILITY).** `docs/ade-TRACEABILITY.md` has been regenerated against the IDSE registry at `08c7c3f5`; the regeneration is in the working tree and not yet committed. All 31 new gates appear in it. In the IDSE registry, every new gate is named in at least one invariant's `ci_scripts`. Of the 33 modified gates, 5 are named by no IDSE invariant: `genesis_replay_open_obligation`, `registry_unique_ids`, `utxo_admission_seam`, `utxo_fp_cache` and `utxo_pre_resolve`. Across HEAD as a whole, **27 of 286 gates** are named by no IDSE invariant's `ci_scripts`. Some are hygiene gates by design (`registry_unique_ids`, `module_headers`, `no_secrets`); others carry legacy MEM-OPT / EVIEW / producer-loop enforcement that the Define batches did not attach to a `ci_scripts` list.

---

## 6. Canonical Type Registry Delta

Omitted: `.idse-config.json` `canonical_type_registry` is `null`.

*Informational, approximate (`pub struct` / `pub enum` names over BLUE `src/`):* `ade_ledger` 269 → 328, `ade_core` 49 → 58, `ade_types` 80 → 82; `ade_codec` 13, `ade_crypto` 22 and `ade_plutus` 8 are unchanged. Two `ade_ledger` names disappear: `ExecUnitsParamUpdate` and `ExecUnitsUpdateError`. They were **renamed** in CRE-S7 (`8b41b9e3`) to `EnactableParamUpdate` / `EnactableUpdateError` when `minPoolCost` joined the enactable subset. This is a generalisation, not a semantic removal.

## 7. Normative Rule Delta

Omitted: `.idse-config.json` `normative_docs` is `[]`.

### Invariant registry delta *(informational — the IDSE migration)*

The invariant registry was **replaced, not edited.** `docs/ade-invariant-registry.toml` held 432 legacy IDD rules at the baseline. At HEAD it holds the **IDSE v3.0 registry** (`schema_version = 3`, `registry_count = 260`).

- **Legacy registry, frozen.** `docs/legacy/idd-invariant-registry.toml` has **482** entries. That is 432 at baseline plus 50 added by cluster work in bands 1–13, with zero removed (`comm -23` of the sorted ID lists is empty). Its status tally is enforced 343, declared 112, partial 26, enforced_scaffolding 1. `docs/legacy/idd-to-idse-map.toml` gives all 482 a disposition: **414 migrated, 52 merged, 16 excluded, 0 pending**. The 50 legacy IDs added in this span all map to active IDSE invariants: 49 migrated and 1 merged (into VAL-020 + STR-005).
- **IDSE registry: 260 invariants + 7 assumptions.** All 260 were approved line by line across **12 Define batches**: node 38, epoch 30, pipeline-production 27, resources-release 24, ledger 23, mithril-evidence 21, network 20, foundations 19, durability-recovery 17, codec-wire-crypto 15, consensus 14, consensus-inputs 12. Assumptions A1–A7 were added along the way (A7 `mithril-snapshot-trust` in the mithril-evidence batch).
  - By family: VAL 67, SEC 38, DET 26, SAF 25, PRC 23, STR 22, DUR 14, LIV 11, OBS 9, REC 8, EVO 6, ORD 6, CON 5.
  - Criticality: 215 critical, 45 standard. Status: 260 `active`, 0 deprecated.
  - 16 invariants carry a recorded redefinition from Define consensus (e.g. SEC-001, SAF-013).
  - 7 invariants carry no `legacy_ids` (they are new in IDSE): PRC-003, VAL-060, SEC-031, DUR-014, SEC-034, PRC-012, PRC-020.
- **Claim status after the M2 re-evidence pass at `e563ad94`** (`08c7c3f5`), as corrected by `74c67670`: **partial 184, unresolved 75, falsified 1, supported 0.** `74c67670` appended 14 corrected records after fixing how test references were matched; the earlier records stay. VAL-066 moved from partial to unresolved because its stale synthetic fixture now fails. There are 252 evidence records (195 pass, 57 fail); 22 invariants have no evidence record. Every record discloses its limits. In particular, no IDSE cluster has declared acceptance obligations yet, and no critical invariant has a mutation witness yet. The artifact is `docs/evidence/idse-m2-reevidence.json`.

---

## Anomalies & cross-reference summary

- **PRC-001 `EveryInvariantMechanicallyEnforced` is `falsified`.** The re-evidence pass found 20 active invariants that name no test, CI script or enforcement locus: SAF-002, PRC-002, ORD-001, SEC-002, PRC-003, VAL-015, EVO-003, SEC-015, VAL-040, SEC-031, DUR-014, SEC-034, PRC-012 and more. This is an open IDSE obligation, not a regression in code.
- **Registry "removal" is a replacement, not a discipline violation.** The legacy rules left the active registry path because the registry was replaced, and every one of the 482 has a recorded disposition (0 pending). The 16 `excluded` dispositions each carry an approved settlement in the map. No IDSE invariant was removed, because the IDSE registry starts in this span.
- **Canonical-type "removals": 2, both renames** (`ExecUnitsParamUpdate` / `ExecUnitsUpdateError` → `Enactable*`, CRE-S7). No canonical-type registry is configured, so this is informational.
- **37 CI scripts fail at `e563ad94`** (re-evidence artifact, `ci[].rc != 0`, of 259 run). All 37 already failed at `a3cef7f7`, before the migration. The registry evidence diagnoses them as stale or over-broad structural gates, not demonstrated violations, so they need repair before they can serve as evidence. They should be tracked and must not be attributed to M2. The full list is in §5.
- **Sibling grounding docs have been regenerated but not committed.** CODEMAP, SEAMS and TRACEABILITY were regenerated at `08c7c3f5` against the IDSE registry. They are modified in the working tree and not yet committed. They predate `74c67670`, which touched only evidence records in the registry and no code, CI or invariant text. The committed versions are still the `d036f8e2` (content-HEAD `1e4896eb`) generation.
- **27 of 286 CI gates are named by no IDSE invariant's `ci_scripts`.** Either TRACEABILITY will show them as orphan enforcement, or a later Define/redefinition should attach them.
- **`ade_core` was modified.** The change is additive (+1,301 / −0, timing/slot authority only), but the consensus core is not byte-identical to the baseline.
- **Commits without a conventional-commits prefix and no clear scope: 0.** The 12 `Define:` commits use the IDSE Define format.

---

## Generation notes

### Regen `1e4896eb → 74c67670` (IDSE migration complete; first HEAD_DELTAS under IDSE)

- **The baseline is valid.** `git rev-parse 1e4896eb` resolves, and `git merge-base 1e4896eb HEAD == 1e4896eb` (strict ancestor, no tag). The baseline is read from `.idse-config.json` `head_deltas_baseline`. This regeneration does not advance it; the caller decides.
- **Counts are mechanical.**
  - Commits: `git rev-list --count` gives 222, with `--no-merges` also 222.
  - Files and lines: `git diff --shortstat` gives 1,091 files, +107,842 / −15,676. `--name-status -M` gives 260 A / 209 M / 622 R.
  - Per-crate stats: `git diff --shortstat -- crates/<c>/`.
  - CI: `git ls-tree` + `grep -c ci_check_` gives 255 → 286; `--name-status -- ci/` gives 31 A / 33 M / 0 D.
  - Legacy registry: `grep '^id = '` + `comm` gives 432 → 482, with 0 removed.
  - IDSE registry: parsed with `tomllib` (families, criticality, claim status, evidence results, approval records).
  - Tests: an approximate `#[test]` attribute grep.
- **IDSE ID mapping.** Each gate's IDSE IDs come from the HEAD registry's `ci_scripts` fields. Cluster-to-invariant attributions in the band table come from the same join, plus `docs/legacy/idd-to-idse-map.toml` for the 50 legacy IDs added in the span.
- **§1 is verbatim from `git log`.** Synthesis lives in the band table and §3.
- **Working tree.** An untracked `wire_smoke.jsonl` exists and is not part of any count.
