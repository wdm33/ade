# Ade — HEAD Deltas (Changes Since Baseline)

> **Status:** Living architectural document. Regenerated; not hand-edited.
>
> Regenerate with `/head-deltas <baseline>` after every cluster close. Baseline is recorded in `.idd-config.json` `head_deltas_baseline`.

> Baseline: `470f9b89` (MEM-OPT-UTXO-DISK close — `OP-MEM-02` flipped `declared → enforced` SCOPED, 2026-06-17 00:58)
> HEAD: `cdcd9397` (`feat(epoch): live FirstRun → native Mithril bootstrap invocation (DC-MITHRIL-07, S1d)`, 2026-06-24 15:14)
> Span: **`470f9b89 → cdcd9397`** — **75 commits** (no merges), **179 files changed, +30,013 / −439 lines**.

> **THE HEADLINE (read first).** The single most important delta in this span is **`a17c7aab` (ECA-1): the
> `EVIEW_ACTIVATION_ARMED` semantic gate was REMOVED.** At the prior regen point (`a50a3ee8`) the epoch-view activation
> was wired into the relay loop but **doubly gated off** (`EVIEW_ACTIVATION_ARMED: bool = false` + `None` activation
> inputs) — *INERT*. That is **no longer true.** As of HEAD `cdcd9397`, `EVIEW_ACTIVATION_ARMED` **does not exist anywhere
> in `crates/`** (`git grep EVIEW_ACTIVATION_ARMED cdcd9397` is empty; it was present 6× at `a50a3ee8`). Epoch-view
> activation is now **AUTOMATIC and DETERMINISTIC from canonical durable state** — no build flag, no runtime switch, no
> `armed` parameter, no `if !armed` short-circuit decides *whether* the consensus transition occurs. The **only** gate is
> the activation predicate (candidate exists + bindings match the selected chain + source window complete + readiness
> valid + activation WAL durable ⇒ promote; else ⇒ fail-closed). This is enforced by the **new rule `DC-EPOCH-13`
> (enforced)** and the new gate `ci_check_eview_automatic_activation.sh`. **Any earlier doc that frames the epoch
> transition as "INERT / gated off / byte-identical live path" is now STALE — that framing described `a50a3ee8`, not
> HEAD.**

> **Baseline note (load-bearing — read before §0).** This refresh's baseline is **`470f9b89`**, the
> **MEM-OPT-UTXO-DISK cluster-close** (it flipped `OP-MEM-02 → enforced` SCOPED; pushed to `origin/main`, 2026-06-17),
> and it is **valid**: `git rev-parse 470f9b89` resolves and `git merge-base 470f9b89 HEAD == 470f9b89` (a strict
> ancestor of HEAD; `470f9b89` carries no tag). HEAD is **`cdcd9397`**. **This is NOT a cluster-close refresh** — every
> commit in the span is **mid-cluster `feat(epoch)` work** across four bands, and **no cluster has closed since the
> baseline**. Per IDD discipline **the baseline is NOT advanced** (`.idd-config.json` `head_deltas_baseline` stays
> `470f9b89`); the on-disk config is untouched. This regen exists because the prior on-disk HEAD_DELTAS was regenerated
> at `a50a3ee8` (mid-EVIEW, ~18 commits short of HEAD) and is now stale — it stops before the ECA gate-removal band and
> the native-Mithril band entirely.
>
> **Working-tree note.** At this regen the working tree shows the four grounding docs modified (this coordinated refresh)
> + untracked scratch (`.mithril-scratch/`) + untracked `docs/active/*.md` runbooks and two untracked cluster-slice
> dirs. §1 narrates the committed span `470f9b89..cdcd9397` verbatim from `git log`; §0/§6/§7 read rule **status** +
> canonical-type counts from the registry / BLUE trees at HEAD `cdcd9397` (**418** rules, **291** enforced; **504** BLUE
> canonical types). The four grounding docs are coordinated — CODEMAP/SEAMS/TRACEABILITY refresh alongside this doc and
> reflect HEAD `cdcd9397`.

---

# The span in four bands (`470f9b89 → cdcd9397`)

Every commit in the span is mid-cluster — there is **no cluster-close commit**. The 75 commits fall into four
contiguous bands. Reading oldest→newest:

| Band | Range | Commits | Theme | Headline rule(s) |
|---|---|---|---|---|
| **1. Standalone fix band** | `1b79add0 … cf508424` | 28 | C2-preprod / live-follow hardening + participant-forge rung + Mithril documented-interface evidence | `RO-MITHRIL-IMPORT-01` (`partial → enforced`), `DC-WAL-05`, `DC-CINPUT-05`, `OP-OPS-04`†, `DC-CRYPTO-10`†, `DC-MEM-11`, `CN-FOLLOW-01`, `DC-FOLLOW-FORGE-01`, `T-CONS-01` (`declared → enforced`) |
| **2. EPOCH-CONSENSUS-VIEW (EVIEW)** | `84e1019c … a50a3ee8` | ~36 | The native cross-epoch stake/consensus view — the hermetic substrate; **shipped INERT (gated off)** in this sub-band | `DC-EVIEW-01..11` (+`04b`), `DC-EPOCH-04..11` |
| **3. EPOCH-CONTINUITY-ACTIVATION (ECA)** | `ad704f86 … f09cc0ec` | 7 | **REMOVES the activation gate (ECA-1)** → activation AUTOMATIC; cardano-faithful pool lifecycle; leadership-complete view; v4 seed sidecar; atomic epoch-authority transition + crash recovery | **`DC-EPOCH-13`** (gate removal), `DC-EVIEW-12/13`, `DC-CINPUT-06`, `DC-EPOCH-12`, `DC-EPOCH-14` |
| **4. Native Mithril bootstrap** | `7386bf82 … cdcd9397` | 11 | Native V2 LedgerDB decode → canonical CertState + faithful UTxO → authoritative materialization + atomic persist → live FirstRun bootstrap; + standalone ledger-value/min-UTxO types | `DC-MITHRIL-03/04/05/06/07`, `DC-LEDGER-VALUE-01`, `DC-LEDGER-PARAMS-01` |

† strengthening of an existing rule (recorded in `strengthened_in`), not a new ID.

The narrative below treats each band in turn. **Crucially, band 2's "INERT / gated-off" status is SUPERSEDED by band 3**
— so when §0/§3/§4 describe the *current* activation state, they describe the band-3 outcome (automatic), and explicitly
note the gate existed earlier in the span (band 2) and was removed.

---

## Band 1 — standalone fixes (`1b79add0 … cf508424`, 28 commits)

Three threads land between the baseline bump and the EVIEW cluster:

- **C2-preprod / live-follow hardening** (`3c6c30ea`, `1be6e855`, `c51d7d81`, `88e64df2`, `5b99333c`, `0c2dae4d`):
  persist admitted block bytes before the WAL (`DC-WAL-05`, enforced); warm-start era-schedule uses durable venue
  geometry, **not** the restart CLI/genesis (`DC-CINPUT-05`, enforced — the durable store is the replay authority); the
  KES shell-init anchors evolution-0 at the opcert start (`OP-OPS-04` strengthened, enforced) and the KES-period gate
  returns the **opcert-anchored RELATIVE evolution** (not the absolute period) (`DC-CRYPTO-10` strengthened, enforced —
  the forge now KES-signs on a real-chain slot, fixing the bounty-blocking "no_tip_available since epoch 1331"); and the
  **LIVE-FOLLOW-THROUGHPUT** fix — forward-sync admit **reuses a cached UTxO fingerprint** instead of an O(n) per-block
  recompute (`DC-MEM-11`, enforced; the cache hit-path test + structural rollback invalidation), reusing the
  MEM-OPT-UTXO-DISK `UtxoFpCache`.
- **PRODUCER-PARTICIPANT-FOLLOW** (`5e3c0855`, `300959c6`, `cf508424`): the participant venue forges on the **AO-selected
  durable head** (`CN-FOLLOW-01`, enforced) and derives the forge base from the live AO-selected durable tip — **not** a
  self-forge latch (`DC-FOLLOW-FORGE-01`, enforced); the adoption channel is the **localRoot dial**, not a duplex
  responder (a documented dead-end correction — the node dials Ade's serve once LOADED; restart the node if topology
  changed after start).
- **Mithril documented-interface evidence** (`88c862cc`, `93dc99bb`, `176c7059`, `f268d3d9`, `31ae1f63`, `13d506cc`):
  the Mithril documented-interface capture + validation tooling + the preprod bundle → **`RO-MITHRIL-IMPORT-01` flipped
  `partial → enforced`** (documented evidence gate); **`T-CONS-01` flipped `declared → enforced`** (bound to
  `CN-CONS-01` enforcement). The span's first commit (`1b79add0`) is the baseline-bump bookkeeping commit.

---

## Band 2 — EPOCH-CONSENSUS-VIEW (EVIEW) (`84e1019c … a50a3ee8`, ~36 commits)

**EPOCH-CONSENSUS-VIEW** is the native cross-epoch consensus cluster. It answers the live-producer wall recorded in
`DC-EPOCH-03`: Ade fail-closes off its single bootstrapped seed epoch because it has **no next-epoch leader view** — the
per-pool stake aggregation that would produce one was absent (`new_mark` zero-fills pool stakes; nothing decoded an
output address to a staking credential). EVIEW builds that aggregation **inside the single ledger authority** (Option 3:
a pure projection of `block_validity → apply_block_with_verdicts → apply_epoch_boundary_full`, NOT a second StakeView)
and forms the next-epoch `EpochConsensusView` by **bounded self-replay**. It is mostly BLUE (the classification +
reduction + aggregation primitives in `ade_ledger` + `ade_codec`) with a RED orchestration shell (`ade_runtime` +
`ade_node`) that reads the durable ChainDB.

> **Activation status — SUPERSEDED BY BAND 3 (do NOT read band 2 in isolation).** As **originally shipped in this
> sub-band**, EVIEW's boundary-activation path was wired into the relay loop but **doubly gated off** — a semantic gate
> `pub const EVIEW_ACTIVATION_ARMED: bool = false;` (`epoch_wire.rs`) **plus** `None` activation inputs at the relay-loop
> call site — so the live follow/forge path was byte-identical. That `EVIEW_ACTIVATION_ARMED` const was an **explicit,
> TEMPORARY dev scaffold**, named at design time as "to be REMOVED by the next slice (EPOCH-CONTINUITY-ACTIVATION)."
> **Band 3 (`a17c7aab`, ECA-1) removed it.** So the INERT framing describes the *intermediate* state at `a50a3ee8`, **not
> HEAD.** See Band 3 and §0 for the current (automatic) state.

### The dual-path architecture (load-bearing — which path is authority)

EVIEW materializes the next-epoch view **two independent ways**, with a strict authority split:

- **Durable window replay = the SOLE authoritative candidate.** The manifest-bound bootstrap cert-state checkpoint
  (`DC-EVIEW-09`) + the canonical selected-chain ChainDB window + explicit named source-window bounds drive
  `drive_window_aggregate → form_mark_snapshot → EpochConsensusView::bind`. NO peer fetch, CLI query, cache, or live
  network response may supply a missing block during derivation; window bounds are explicit **named roles**
  (`source_epoch` / `source_window_start..end` / `snapshot_phase` / `target_epoch`), **never** wall-clock or an inline
  `source + k`.
- **The live reduced checkpoint = a readiness WITNESS only.** The continuously-advanced live reduced-UTxO checkpoint
  (`DC-EPOCH-11`, the `-mat` sub-slices) is an **independent cross-check**: the live-derived view must AGREE with the
  replay candidate on the committed fields before promotion. A mismatch / missing range / late candidate is a
  **TERMINAL** epoch-activation halt. It is **never** the authority and **never** on the live follow/forge path (the
  live producer stays `track_utxo=false`).
- **Activation = predicate → WAL → promote.** `activation_predicate` → `activate_durable_before_visible` (the
  `WalEntry::EpochConsensusViewActivated` record is durable BEFORE the active view is published, `DC-EPOCH-06`) →
  atomically-published `ActiveEpochView`; warm-start reconstructs the same active view from the activation WAL + the
  bound artifacts alone.

### The slice arc

- **Design + slice 1 (`84e1019c … 85fbc04f`):** the design-analysis record + the **bounded, crash-safe
  transient-materialization gate** — `ade_runtime::chaindb::transient_epoch_view` (a GREEN, non-authoritative
  disk-backed replay-window lifecycle over the dormant redb `UtxoAnchor`; deterministic `window_key`, owned
  `transient-epoch-view/` subtree, never under WAL/snapshots/ChainDb, no runtime flag), proven by the SIGKILL
  kill-harness (`DC-EVIEW-01`).
- **Slice 2 — typed stake-reference classification (`8f74ccef`; `DC-EVIEW-02`):** the typed, era-gated address →
  `StakeRef` classifier (`ade_ledger::stake_ref`) — base contributes all eras, pointer pre-Conway only, enterprise/null
  never. **No fixed byte offset is authoritative** across variants/eras.
- **Slice 3a — pointer decode/resolution (`c71a308f`; `DC-EVIEW-03`):** the era-parameterized pointer decode
  (`ade_codec::address::pointer::{Ptr, PointerDecodeError}`) + the pointer→credential resolution
  (`ade_ledger::pointer_resolve`).
- **Slice 3b — replay-window materialization (`83ead7be`, `8c0ff66f`; `DC-EVIEW-04`/`04b`):** the **durable reduced-UTxO
  checkpoint** (`ade_runtime::chaindb::reduced_utxo_checkpoint` — a disk-backed redb `TxIn → (Coin, ReducedStakeRef)`,
  crash-safe completeness marker written LAST, fingerprint a hash chain over canonical records in `TxIn` order) + the
  windowed advance (`reduced_advance`).
- **Slice 3c–3e — aggregation → snapshot → view (`77a7e3f3 … ce778913`; `DC-EVIEW-05`/`06`/`07`):** per-pool stake
  aggregation (the linchpin, `reduced_aggregate::StakeByPool`), snapshot formation + the k-immutability stability gate
  (`reduced_snapshot::SnapshotPhase`), and the bound immutable `EpochConsensusView` (`reduced_epoch_view`). The S3c
  **live differential-oracle** result is recorded (`62eb6738`).
- **Slice 3f — activation substrate (`3c2db639 … 38aa5518`; `DC-EVIEW-08..11`, `DC-EPOCH-04..10`):** the WAL activation
  record (`DC-EPOCH-04`), the activation predicate + atomically-published view (`DC-EPOCH-05/07`), durable-before-visible
  + crash recovery (`DC-EPOCH-06`), the source window + named source→target mapping (`DC-EPOCH-08`), candidate
  derivation from a validated window (`DC-EPOCH-09`), the boundary sequenced flip (`DC-EPOCH-10`), the manifest-bound
  bootstrap cert-state import (`DC-EVIEW-09`), the window driver (`DC-EVIEW-10`), and the deterministic fail-closed
  epoch-rebind seam (`DC-EVIEW-11`, strengthening `DC-EPOCH-03`).
- **Slice 3f-4d-mat — the live reduced checkpoint (`0ac92cba … bfa0b54a`; `DC-EPOCH-11`):** build at bootstrap (BEFORE
  `drop(utxo)`, gated on the EVIEW cert-state so non-EVIEW bootstrap stays byte-identical), the per-block advance
  primitive + the ChainDB-replay advancer (`reduced_window_driver`), the relay-loop wiring, reorg re-materialize, the
  fail-closed readiness gate, and the **off-repo shadow harness** that PROVED the live derive against cardano-node on the
  real **3M-entry preview UTxO** (reduction **100% exact**, **ADE1 exact**).
- **Slice 3f-4d-wire — dual-path activation, GATED (`e14a0e15 … a50a3ee8`):** the live source-window extraction
  (`epoch_source_window`), the readiness witness + replay seed-state checkpoint, the sole authoritative derive
  (`epoch_candidate`), the boundary-activation orchestration (`epoch_activate` / `epoch_activation`), and the relay-loop
  wire — **shipped GATED OFF** (`EVIEW_ACTIVATION_ARMED = false` + `None` inputs). **Band 3 then removed the gate.**

---

## Band 3 — EPOCH-CONTINUITY-ACTIVATION (ECA) (`ad704f86 … f09cc0ec`, 7 commits) — **THE HEADLINE**

This band turns EVIEW from an inert hermetic substrate into a **live, automatic** epoch-view activation. It is the
single most consequential band in the span.

- **`a17c7aab` (ECA-1) — the gate removal.** Removes the `EVIEW_ACTIVATION_ARMED` semantic gate entirely: no `const`, no
  `armed` parameter, no `if !armed` short-circuit, no env var / build feature / CLI option anywhere in `crates/`.
  Epoch-view activation flips from **INERT / doubly-gated** to **AUTOMATIC and DETERMINISTIC from canonical durable
  state** — the **only** gate is the activation predicate (candidate exists + bindings match the selected chain + source
  window complete + readiness valid + activation WAL durable ⇒ promote; else ⇒ fail-closed). Success is **continuous
  operation across the epoch boundary**, no manual arming / restart / import. Codifies the project rule "no build/runtime
  flag may decide WHETHER consensus transitions occur." Enforced by **new rule `DC-EPOCH-13`** + new gate
  `ci_check_eview_automatic_activation.sh`. (Verified: `git grep EVIEW_ACTIVATION_ARMED` is empty at HEAD; it was present
  6× at `a50a3ee8`.)
- **`ad704f86` (ECA-0a) — cardano-faithful pool lifecycle** in the reduced window: the cert-state pool lifecycle matches
  cardano-ledger (`Pool.hs`/`PoolReap.hs`/`Epoch.hs`/`SnapShots.hs`) so a windowed replay reproduces the mark snapshot
  (new rule `DC-EVIEW-13`, enforced; gate `ci_check_eview_pool_lifecycle.sh`).
- **`4614e977` (ECA-0b) — leadership-complete EpochConsensusView + exclusive projection:** every INCLUDED pool carries
  BOTH its active stake AND its era-correct VRF/leadership data; the candidate view is the production authority for
  cross-epoch leadership (new rule `DC-EVIEW-12`, enforced; gate `ci_check_eview_leadership_complete.sh`; the exclusive
  projection is tracked as `DC-EPOCH-12`, enforced).
- **`124c87da` (ECA-2-pre) — v4 seed sidecar** persists the consensus-profile hashes (`genesis_hash` +
  `protocol_params_hash`), recovered from the store on warm-start (new rule `DC-CINPUT-06`, enforced; gate
  `ci_check_eview_seed_sidecar_v4.sh`).
- **`ad41b274` (ECA-2/3/4) — atomic epoch-authority transition + crash recovery:** the assemble-and-atomically-persist
  authority transition with warm-start recovery (gate `ci_check_eview_atomic_authority.sh`, bound to `DC-EPOCH-14`).
- **`0a500e59` (DC-EPOCH-14) — wrong-magic fail-close:** warm-start fail-closes on a wrong CLI network magic (the CLI is
  a fail-closed consistency check against the durable store, not an authority) (new rule `DC-EPOCH-14`, enforced).
- **`f09cc0ec` — bootstrap-cert-state producer, live-verified on Preview.**

---

## Band 4 — Native Mithril bootstrap (`7386bf82 … cdcd9397`, 11 commits)

This band gives Ade a **native** Mithril-snapshot bootstrap path: it decodes the cardano-node V2 LedgerDB (utxohd-mem,
tablesCodecVersion 1) directly into Ade-canonical state, rather than importing via the `cardano-cli` JSON seed. The raw
cardano-node CBOR is **RED/diagnostic INPUT, never the authority** — the authority is the Ade-canonical projection.

- **`7386bf82`** reclassifies the CLI exporter as **auxiliary** and adds the V2 LedgerDB native-decode probe + manifest
  v2.
- **`53c27bc4` (S1a-1)** — native non-UTxO snapshot decoder + manifest-bound network identity (`network_id_from_magic` +
  `NetworkIdentityMismatch`; gate `ci_check_native_nonutxo_decode.sh`, bound to `DC-LEDGER-PARAMS-01`).
- **`3bbba530` (DC-MITHRIL-01, Stage 1)** — native V2 LedgerDB **state** decoder → canonical `CertState` + pool distr +
  Praos nonces (`ade_ledger::ledgerdb_state`; gate `ci_check_ledgerdb_state_decode.sh`, bound to the new `DC-MITHRIL-04`,
  enforced). Note: `DC-MITHRIL-01` itself **pre-existed at baseline** (the cross-impl Mithril obligation); this commit
  builds the decoder that `DC-MITHRIL-04` enforces.
- **`6cab0d6c` (DC-MITHRIL-02, Stage 2)** — native V2 LedgerDB **tables** MemPack `TxOut` decoder → faithful UTxO
  (`ade_ledger::ledgerdb_tables`; gate `ci_check_ledgerdb_tables_decode.sh`, bound to the new `DC-MITHRIL-05`, enforced).
  Like `-01`, `DC-MITHRIL-02` pre-existed at baseline.
- **`e84ebb0c`** — registry repair: a **`DC-MITHRIL-01`/`-02` ID collision** introduced earlier in this band was
  repaired, and a **uniqueness guard** (`ci_check_registry_unique_ids.sh`) was added so a duplicate `id =` can never
  recur. Net effect: `-01`/`-02` remain singular at HEAD (verified: total `id =` lines == unique `id =` lines == 418).
- **`c952c767` (DC-MITHRIL-03, S1b)** — native Mithril authority transition: assemble + atomic persist
  (`ade_runtime::mithril_native_assembly`; gate `ci_check_mithril_authority_transition.sh`, enforced).
- **`942cd97c` (DC-MITHRIL-06, S1c)** — tables → authoritative `UTxOState` materialization
  (`ade_ledger::mithril_utxo_materialize`; gate `ci_check_tables_to_utxostate.sh`, enforced).
- **`cdcd9397` (DC-MITHRIL-07, S1d) = HEAD** — live FirstRun → native Mithril bootstrap invocation
  (`ade_node::native_firstrun`; gate `ci_check_native_firstrun_no_cli_seed.sh`, enforced).
- **Standalone ledger types in this band:** `5426dceb` (`DC-LEDGER-VALUE-01`, enforced) — output asset quantity is the
  **Word64 domain** (`OutputAssetQuantity`, never truncated/cast to i64; gate `ci_check_value_quantity_domain.sh`);
  `cb20ab02` (`DC-LEDGER-PARAMS-01`, enforced) — era-aware protocol-parameter **min-UTxO representation** (`MinUtxoRule`:
  `LegacyAbsoluteMin` keeps the absolute check, Conway `PerByte` → fail-closed `UnsupportedConwayMinUtxoRule`).
- **CI-hygiene note:** `7c769801` records a **pre-existing** `epoch_boundary_logic` test hang in
  `ade_testkit` (the `all_epoch_boundaries_fire` hang) as a CI hygiene blocker — band-4 gating runs on targeted suites,
  not `cargo test -p ade_testkit`.

---

## 0. Headline (full span `470f9b89 → cdcd9397`)

| Count | Baseline (`470f9b89`) | HEAD (`cdcd9397`) | Δ (full span) |
|---|---|---|---|
| **Epoch-view activation** | — | **AUTOMATIC (no semantic gate)** | **`EVIEW_ACTIVATION_ARMED` REMOVED (`a17c7aab`, ECA-1).** Present 6× at the intermediate `a50a3ee8`; **ZERO at HEAD** (`git grep` empty in `crates/`). Activation is now automatic + deterministic from canonical durable state; the activation predicate is the only gate. New rule **`DC-EPOCH-13` (enforced)**, gate `ci_check_eview_automatic_activation.sh`. |
| CI gates (`ci/ci_check_*.sh`) | 200 | **238** | **+38 new / 0 modified / 0 removed.** EVIEW: +21 (`ci_check_eview_*` ×18 + `ci_check_transient_view_*` ×3). ECA: +4 (`automatic_activation`, `atomic_authority`, `leadership_complete`, `pool_lifecycle`, `seed_sidecar_v4` — note 5; one of the EVIEW-counted under `eview_`). Mithril/ledger band: +9 (`ledgerdb_state_decode`, `ledgerdb_tables_decode`, `mithril_authority_transition`, `tables_to_utxostate`, `native_firstrun_no_cli_seed`, `native_nonutxo_decode`, `value_quantity_domain`, `mithril_documented_evidence`, `registry_unique_ids`). Band 1: +3 (`forward_sync_fp_cache`, `participant_forge_on_selected_head`, `admission_runner_no_block_byte_map`). One non-gate helper also added (`ci/capture_mithril_documented_evidence.sh`). |
| Registry rules (`docs/ade-invariant-registry.toml`) | 380 | **418** | **+38, ZERO removed** (`comm -23` of sorted `id =` lists empty; `comm -13` = exactly 38). |
| Registry status (enforced / scaffolding / partial / declared) | 253 / 1 / 23 / 103 | **291 / 1 / 22 / 104** | enforced **+38**, partial **−1**, declared **+1**, scaffolding 0. The 38 new IDs land **mostly enforced**; **2 land `declared`** (`DC-EPOCH-11`, `DC-EVIEW-08` — both inherited from band 2 before ECA; not yet re-flipped). Flips on existing rules (`RO-MITHRIL-IMPORT-01 partial → enforced`, `T-CONS-01 declared → enforced`) net the tally. |
| `DC-EPOCH-11` (live reduced checkpoint) | — | **`declared`** (NEW) | Built at bootstrap + advanced per durable admit + reorg re-materialize + fail-closed readiness; PROVEN against cardano-node on the 3M-entry preview UTxO (off-repo shadow harness). Stays `declared` — the live checkpoint is the cross-check witness; its registry flip is owed even though activation itself is now automatic. Gate `ci_check_eview_live_checkpoint.sh`. |
| `DC-EVIEW-08` (activation consumption point) | — | **`declared`** (NEW) | The boundary consumes the aggregate. Stays `declared` pending a committed live-flip transcript. |
| BLUE canonical types | 466 | **504** | **+38.** `ade_ledger 185 → 219` (+34: the `reduced_*` / `stake_ref` / `pointer_resolve` / `bootstrap_manifest` / `ledgerdb_state` / `ledgerdb_tables` / `mithril_utxo_materialize` + `MinUtxoRule` / `OutputAssetQuantity` families); `ade_codec 11 → 13` (+2: `Ptr`, `PointerDecodeError`); `ade_types 82 → 84` (+2). **`ade_core` untouched (48 → 48)** — the consensus authority is byte-identical. `ade_crypto`/`ade_plutus`/`ade_network` BLUE sub-paths unchanged. |
| Crates | 12 | **12** | **No delta.** `ade_mem_diag` + `ade_core_interop` already exist at baseline; no new workspace member; no new `Cargo.toml`. |
| Tests (`#[test]` / `#[tokio::test]` attrs) | 2640 | **2934** | **+294** (approximate per the attribute-count fallback) — the EVIEW/ECA hermetic suites + the LedgerDB-decode hermetic/corpus suites + the kill-harness/memory tests + the band-1 fix tests. |
| Grounding docs (CODEMAP / SEAMS / TRACEABILITY) | regenerated to MEM-OPT-UTXO-DISK content-HEAD | **all four → `cdcd9397`** | Coordinated refresh at HEAD. **Baseline NOT advanced** (no cluster closed in the span). |

## 1. Commit Log (newest first, full span `470f9b89..cdcd9397`)

| Hash | Type | Summary |
|------|------|---------|
| `cdcd9397` | feat | feat(epoch): live FirstRun → native Mithril bootstrap invocation (DC-MITHRIL-07, S1d) |
| `942cd97c` | feat | feat(epoch): tables → authoritative UTxOState materialization (DC-MITHRIL-06, S1c) |
| `c952c767` | feat | feat(epoch): native Mithril authority transition — assemble + atomic persist (DC-MITHRIL-03, S1b) |
| `e84ebb0c` | chore | chore(registry): repair DC-MITHRIL-01/02 ID collision + add a uniqueness guard |
| `53c27bc4` | feat | feat(epoch): native non-UTxO snapshot decoder + manifest-bound network identity (S1a-1) |
| `cb20ab02` | feat | feat(ledger): era-aware protocol-parameter min-UTxO representation (DC-LEDGER-PARAMS-01, S1a-2) |
| `7c769801` | docs | docs(testkit): track pre-existing epoch_boundary_logic hang as a CI hygiene blocker |
| `5426dceb` | feat | feat(ledger): output asset quantity is the Word64 domain (OutputAssetQuantity, DC-LEDGER-VALUE-01) |
| `6cab0d6c` | feat | feat(epoch): native V2 LedgerDB tables MemPack TxOut decoder → faithful UTxO (DC-MITHRIL-02, Stage 2) |
| `3bbba530` | feat | feat(epoch): native V2 LedgerDB state decoder → canonical CertState (DC-MITHRIL-01, Stage 1) |
| `7386bf82` | feat | feat(epoch): reclassify cli exporter as auxiliary; V2 LedgerDB native-decode probe + manifest v2 |
| `f09cc0ec` | feat | feat(epoch): bootstrap-cert-state producer, live-verified on Preview |
| `0a500e59` | test | test(epoch): prove warm-start fail-closes on a wrong CLI network magic (DC-EPOCH-14) |
| `ad41b274` | feat | feat(epoch): atomic epoch-authority transition + crash recovery (ECA-2/3/4) |
| `124c87da` | feat | feat(epoch): persist the consensus-profile hashes in the v4 seed sidecar (ECA-2-pre) |
| `a17c7aab` | feat | **feat(epoch): remove the EVIEW_ACTIVATION_ARMED semantic gate (ECA-1)** |
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
| `38aa5518` | feat | feat(epoch): boundary activation orchestration — the sequenced flip (DC-EPOCH-10) |
| `28c05bff` | feat | feat(epoch): activation candidate derivation from a validated window (DC-EPOCH-09) |
| `235e3183` | feat | feat(epoch): activation source window + named-role source→target mapping (DC-EPOCH-08) |
| `49a4d8ce` | feat | feat(epoch): activation durable-before-visible + crash recovery (DC-EPOCH-06) |
| `91293215` | feat | feat(epoch): activation predicate + atomically-published active view (DC-EPOCH-05/07) |
| `29253e4c` | feat | feat(epoch): WAL activation record — the durable activation substrate (DC-EPOCH-04) |
| `86353625` | feat | feat(epoch): deterministic fail-closed epoch-rebind seam (DC-EVIEW-11, strengthens DC-EPOCH-03) |
| `7f7d266a` | feat | feat(epoch): the window driver — advance + aggregate over a block window (DC-EVIEW-10) |
| `bd8b0def` | feat | feat(epoch): manifest-bound bootstrap cert-state import (DC-EVIEW-09) |
| `3c2db639` | feat | feat(epoch): activation consumption point — the boundary consumes the aggregate (DC-EVIEW-08 S3f-1) |
| `62eb6738` | docs | docs(epoch): record the S3c live differential-oracle result (DC-EVIEW-05) |
| `a9d1f148` | fix | fix(ci): S3b-1 checkpoint gate no longer false-positives on the S3c reader |
| `ce778913` | feat | feat(epoch): the bound, immutable EpochConsensusView (DC-EVIEW-07) |
| `88fdfadf` | feat | feat(epoch): snapshot formation + the k-immutability stability gate (DC-EVIEW-06) |
| `77a7e3f3` | feat | feat(epoch): per-pool stake aggregation — the linchpin (DC-EVIEW-05) |
| `8c0ff66f` | feat | feat(epoch): windowed advance of the reduced-UTxO checkpoint (DC-EVIEW-04b) |
| `83ead7be` | feat | feat(epoch): durable reduced-UTxO checkpoint — the minimal native state (DC-EVIEW-04) |
| `388a3b61` | docs | docs(epoch): scope EPOCH-CONSENSUS-VIEW S3b-1 — durable reduced-UTxO checkpoint (pre-code) |
| `d6d015eb` | docs | docs(epoch): scope EPOCH-CONSENSUS-VIEW S3b (umbrella) — replay-window materialization |
| `c71a308f` | feat | feat(epoch): era-parameterized pointer decode + resolution (DC-EVIEW-03) |
| `7a2462b1` | docs | docs(epoch): scope EPOCH-CONSENSUS-VIEW S3a — pointer decode/resolution (pre-code) |
| `a8b5d1c6` | docs | docs(epoch): scope EPOCH-CONSENSUS-VIEW slice 3 — native next-epoch view (pre-code) |
| `8f74ccef` | feat | feat(epoch): typed era-gated stake-reference classification (DC-EVIEW-02) |
| `502b23b5` | docs | docs(epoch): scope EPOCH-CONSENSUS-VIEW slice 2 — typed stake-reference classification |
| `85fbc04f` | feat | feat(epoch): prove the bounded crash-safe transient-materialization gate (DC-EVIEW-01) |
| `28be6635` | docs | docs(epoch): slice 1 resolved entry obligations + tightenings + GREEN classification |
| `39a6b5af` | docs | docs(epoch): EPOCH-CONSENSUS-VIEW slice 1 scope — redb temporary-materialization gate |
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
| `88c862cc` | docs | docs(invariant-registry): bind T-CONS-01 to CN-CONS-01 enforcement (declared → enforced) |
| `1b79add0` | chore | chore(idd): bump head_deltas_baseline 862cd2cb → 470f9b89 (MEM-OPT-UTXO-DISK close) |

No merge commits in the span. **75 commits.** Conventional-commits prefix tally (exact, from `git log`): **`feat`×48**,
**`docs`×12**, **`fix`×8**, **`chore`×2**, **`test`×1**, **`perf`×1** (= 72) + **3 non-`feat/fix/...` but unambiguous
prefixes**: `harden(node):` (`5b99333c`, a `DC-MEM-11` hardening test), `registry:` (`13d506cc`), `evidence:`
(`31ae1f63`). All 75 commits have a clear scope; **none is unclassifiable.** Band breakdown: band 1
(`1b79add0..cf508424`, 28 commits) carries the standalone fixes + Mithril evidence; band 2 EVIEW
(`84e1019c..a50a3ee8`, 36 commits); band 3 ECA (`ad704f86..f09cc0ec`, 7 commits — `feat`×6 + `test`×1); band 4 Mithril
(`7386bf82..cdcd9397`, 11 commits).

> **Note (commit-attribution policy).** Per this repo's `CLAUDE.md` override (vibe-coded-node bounty trailer
> requirement), commits in this repo carry a `Co-Authored-By:` model-attribution trailer; that is an Ade-local override
> of the global no-AI-attribution rule and applies to **commit messages only**. It does not affect this doc's content.

## 2. New Modules

The span adds **37 new `.rs` files** (no new crate; no new `Cargo.toml`) — 11 source modules under `ade_ledger`/
`ade_codec`/`ade_runtime`/`ade_node` from EVIEW + the Mithril decoders, plus the `epoch_*` RED orchestration shell, plus
the LedgerDB-decode/Mithril test suites. The BLUE classification + reduction + decode primitives live in `ade_ledger` +
`ade_codec`; the durable/transient checkpoint storage + the native-assembly + the RED orchestration shell live in
`ade_runtime` + `ade_node`.

| Module | Color | Purpose | Key sub-paths | Added in |
|--------|-------|---------|---------------|----------|
| `ade_ledger::stake_ref` | **BLUE** | Typed, era-gated stake-reference classification (`StakeRefClass`/`StakeRefReject`/`PointerRef`) — base all eras, pointer pre-Conway only, enterprise/null never. **No fixed byte offset is authoritative.** | `stake_ref.rs` | EVIEW S2 (`8f74ccef`) |
| `ade_ledger::pointer_resolve` | **BLUE** | Pointer→credential resolution (pre-Conway only). | `pointer_resolve.rs` | EVIEW S3a (`c71a308f`) |
| `ade_ledger::reduced_utxo` | **BLUE** | The reduced UTxO record + reduction (`ReducedStakeRef`, `reduce_txout`) — `TxIn → (Coin, ReducedStakeRef)`. | `reduced_utxo.rs` | EVIEW S3b (`83ead7be`) |
| `ade_ledger::reduced_advance` | **BLUE** | Per-block reduced-state advance (`ReducedBlockDelta`, `advance_cert_state`). | `reduced_advance.rs` | EVIEW S3b (`8c0ff66f`) |
| `ade_ledger::reduced_aggregate` | **BLUE** | Per-pool stake aggregation — the linchpin (`StakeByPool`, `AggregateError`). | `reduced_aggregate.rs` | EVIEW S3c (`77a7e3f3`) |
| `ade_ledger::reduced_snapshot` | **BLUE** | Snapshot formation + the k-immutability stability gate (`SnapshotPhase`). | `reduced_snapshot.rs` | EVIEW S3d (`88fdfadf`) |
| `ade_ledger::reduced_epoch_view` | **BLUE** | The bound, immutable `EpochConsensusView` + `ViewBindings`; inert if any binding is missing. | `reduced_epoch_view.rs` | EVIEW S3e (`ce778913`) |
| `ade_ledger::bootstrap_manifest` | **BLUE** | The manifest-bound bootstrap cert-state import (`BootstrapManifest`/`BootstrapManifestError`). | `bootstrap_manifest.rs` | EVIEW S3f (`bd8b0def`) |
| `ade_ledger::ledgerdb_state` | **BLUE** | Native V2 LedgerDB **state** decoder → canonical `CertState` + pool distr + Praos nonces (`LedgerDbStateProbe`, `LedgerDbStateError`; the raw CBOR is RED INPUT, the projection is authority). | `ledgerdb_state.rs` | Mithril Stage 1 (`3bbba530`) |
| `ade_ledger::ledgerdb_tables` | **BLUE** | Native V2 LedgerDB **tables** MemPack `TxOut` decoder → faithful Word64-quantity UTxO. | `ledgerdb_tables.rs` | Mithril Stage 2 (`6cab0d6c`) |
| `ade_ledger::mithril_utxo_materialize` | **BLUE** | Tables → authoritative `UTxOState` materialization. | `mithril_utxo_materialize.rs` | Mithril S1c (`942cd97c`) |
| `ade_codec::address::pointer` | **BLUE** | Era-parameterized pointer decode (`Ptr{slot,txIx,certIx}`, `PointerDecodeError`). | `address/pointer.rs` | EVIEW S3a (`c71a308f`) |
| `ade_runtime::chaindb::reduced_utxo_checkpoint` | **GREEN-by-contract** (RED crate) | The DURABLE reduced-UTxO checkpoint — disk-backed redb of a BLUE-derivable projection; crash-safe (marker LAST), fingerprint a hash chain in `TxIn` order; **never authority, never on the live path**, reconstructible by replay. | `reduced_utxo_checkpoint.rs` | EVIEW S3b (`83ead7be`) |
| `ade_runtime::chaindb::reduced_window_driver` | **RED** | RED orchestration sequencing `reduced_block_delta` + `advance_cert_state` + `aggregate_pool_stake` over the window replay; reads/writes the durable redb; never the hot path. | `reduced_window_driver.rs` | EVIEW S3f / mat (`7f7d266a`) |
| `ade_runtime::chaindb::transient_epoch_view` | **GREEN** (non-authoritative) | The bounded, crash-safe transient replay-window lifecycle over the dormant `UtxoAnchor`; deterministic `window_key`, owned `transient-epoch-view/` subtree, no runtime flag. | `transient_epoch_view.rs` | EVIEW S1 (`85fbc04f`) |
| `ade_runtime::mithril_native_assembly` | **RED** | The native Mithril authority transition — assemble the canonical state from the V2 decode + atomic persist. | `mithril_native_assembly.rs` | Mithril S1b (`c952c767`) |
| `ade_runtime::consensus_inputs::cert_state_extract` | **RED** | Cert-state extraction plumbing for the bootstrap-cert-state producer. | `consensus_inputs/cert_state_extract.rs` | ECA (`f09cc0ec`) |
| `ade_node::native_firstrun` | **RED** | The live FirstRun → native Mithril bootstrap invocation (no cardano-cli JSON seed). | `native_firstrun.rs` | Mithril S1d (`cdcd9397`) |
| `ade_node::bootstrap_export` | **RED** | The reclassified (auxiliary) CLI exporter + the V2 LedgerDB native-decode probe driver. | `bootstrap_export.rs` | Mithril probe (`7386bf82`) |
| `ade_node::epoch_source_window` | **RED** | `ActivationSourceWindow` + `validate_source_window` (complete/ordered/bounded fail-closed) + the explicit source→target mapping. | `epoch_source_window.rs` | EVIEW wire-1 (`e14a0e15`) |
| `ade_node::epoch_candidate` | **RED** (pure pieces) | `derive_candidate` — the SOLE authoritative candidate from a VALIDATED window (window → aggregate → snapshot → bind). | `epoch_candidate.rs` | EVIEW wire-2b (`bcef6404`) |
| `ade_node::epoch_activate` | **RED** | The boundary activation ORCHESTRATION (the sequenced flip): derive → predicate → WAL→publish → HALT on terminal. | `epoch_activate.rs` | EVIEW wire-3a (`e6e07ae0`) |
| `ade_node::epoch_activation` | **RED** | The activation predicate + the atomically-published `ActiveEpochView` + the `WalEntry::EpochConsensusViewActivated` build + warm-start `recover_active_view`. | `epoch_activation.rs` | EVIEW S3f-4b (`91293215`) |
| `ade_node::epoch_wire` | **RED** | The live dual-path activation orchestration (`EviewActivationInputs`). **As of ECA-1 the `EVIEW_ACTIVATION_ARMED` scaffold is GONE** — `maybe_activate` is governed purely by the activation predicate. | `epoch_wire.rs` | EVIEW wire-3b (`4c63c03d`); gate removed (`a17c7aab`) |
| `ade_node::epoch_rebind` | **RED** | The deterministic fail-closed epoch-rebind seam (`DC-EVIEW-11`, strengthening `DC-EPOCH-03`) — feeds the promoted view to the forge. | `epoch_rebind.rs` | EVIEW S3f-3 (`86353625`) |
| `ade_runtime::bin::transient_view_kill_target` | **RED** (test bin) | The SIGKILL kill-target binary for the crash-safe transient-materialization proof (`DC-EVIEW-01`). | `bin/transient_view_kill_target.rs` | EVIEW S1 (`85fbc04f`) |
| `ade_*` test suites (10 new files) | **test** | EVIEW kill-harness/memory (`transient_view_kill_harness`, `transient_view_memory`); LedgerDB-decode hermetic/corpus/oracle/Mithril (`ledgerdb_state_hermetic`, `ledgerdb_nonutxo_hermetic`, `ledgerdb_state_corpus`, `ledgerdb_state_mithril`, `ledgerdb_tables_decode`, `ledgerdb_tables_oracle`, `ledgerdb_nonutxo_mithril`, `mithril_tables_to_utxostate`); the native-firstrun live test (`native_firstrun_live`). | `tests/*.rs` | EVIEW S1 + Mithril band |

> **Cross-reference (CODEMAP).** All BLUE/GREEN/RED modules above should appear in the coordinated CODEMAP refresh at
> HEAD — `ade_ledger::{stake_ref, pointer_resolve, reduced_utxo, reduced_advance, reduced_aggregate, reduced_snapshot,
> reduced_epoch_view, bootstrap_manifest, ledgerdb_state, ledgerdb_tables, mithril_utxo_materialize}` (§BLUE),
> `ade_codec::address::pointer` (§BLUE), `ade_runtime::chaindb::{reduced_utxo_checkpoint, reduced_window_driver,
> transient_epoch_view}` + `ade_runtime::mithril_native_assembly` + `ade_runtime::consensus_inputs::cert_state_extract`
> (§GREEN/RED), and `ade_node::{native_firstrun, bootstrap_export, epoch_source_window, epoch_candidate, epoch_activate,
> epoch_activation, epoch_wire, epoch_rebind}` (§RED). **If any is missing, CODEMAP is stale — run `/codemap`.** All four
> grounding docs refresh together at `cdcd9397`, so CODEMAP at HEAD reflects this set.

## 3. Modules Modified

The bands modified the BLUE `ade_ledger` (the `rules.rs` mark-snapshot path + `seed_consensus_inputs` + the WAL grammar +
the `value.rs`/`mary.rs`/`pparams.rs` value/min-UTxO representation + `delegation.rs` pool lifecycle +
`snapshot/utxo_state.rs` materialization) and the RED `ade_runtime`/`ade_node` orchestration (the relay loop, the forge,
the producer shell, the seed-merge, the CLI). The band-1 fixes modified the forward-sync reducer, the operator forge, and
the producer shell. Per-module diffstats are over the full span `470f9b89..cdcd9397`.

| Module | Color / scope | Key changes |
|--------|---------------|-------------|
| `ade_node::node_lifecycle` (`node_lifecycle.rs` **+1273**) | **RED**, EVIEW + ECA wire | `run_relay_loop_with_sched` gains the live reduced-checkpoint advance (per durable admit) + the **automatic first-boundary activation** (`maybe_activate_first_boundary` / `maybe_activate_epoch_boundary`); **ECA-1 removed the `EVIEW_ACTIVATION_ARMED` short-circuits here** (5 occurrences gone) — activation now runs whenever the predicate is satisfied. The forge feed consumes the promoted view at the rebind. |
| `ade_node::node_sync` (`node_sync.rs` **+1046**) | **RED**, CN-FOLLOW-01 + EVIEW | The participant extend-on-selected-head fence (`ParticipantFenceViolation` / `ParticipantForgeBaseChangedBeforeSign`, `CN-FOLLOW-01` / `DC-FOLLOW-FORGE-01`) + the EVIEW seam wiring + the `DC-EPOCH-03` rebind wall. |
| `ade_ledger::delegation` (`delegation.rs` **+223**) | **BLUE**, ECA-0a | Cardano-faithful pool lifecycle in the reduced window (`Pool.hs`/`PoolReap.hs`/`Epoch.hs`/`SnapShots.hs`-matching mark/reap) — `DC-EVIEW-13`. |
| `ade_ledger::snapshot::utxo_state` (`snapshot/utxo_state.rs` **+196**) | **BLUE**, Mithril S1c | Tables → authoritative `UTxOState` materialization hooks (`DC-MITHRIL-06`). |
| `ade_ledger::value` (`value.rs` **+187 / −…**) | **BLUE**, DC-LEDGER-VALUE-01 | `OutputAssetQuantity` = the Word64 domain; multi-asset quantity never truncated/cast to i64. |
| `ade_ledger::wal::event` (`wal/event.rs` **+186**) | **BLUE**, EVIEW | The `WalEntry::EpochConsensusViewActivated` variant (wire tag, canonical encode/decode round-trip, replay handling) — `DC-EPOCH-04`. Durable BEFORE the active view is published. |
| `ade_node::cli` (`cli.rs` **+152**) | **RED**, plumbing | KES-period argument-threading (band 1, `OP-OPS-04`) + EVIEW/Mithril bootstrap seams. Not a feature flag. |
| `ade_ledger::mary` (`mary.rs` **+128**) | **BLUE**, DC-LEDGER-PARAMS-01 | The Mary min-UTxO check matches `MinUtxoRule` (`LegacyAbsoluteMin` absolute; Conway `PerByte` → fail-closed). |
| `ade_runtime::producer::{producer_shell, coordinator}` (`producer_shell.rs` **+126**, `coordinator.rs`) | **RED**, OP-OPS-04 / DC-CRYPTO-10 | `ProducerShell::init` takes `current_kes_period`; opcert-window check fail-closes; the KES gate evolves from the opcert-anchored relative evolution. The forge KES-signs on a real-chain slot. |
| `ade_ledger::seed_consensus_inputs` (`seed_consensus_inputs.rs` **+121**) | **BLUE**, EVIEW/ECA | Seed-state plumbing for the replay seed-state checkpoint + manifest binding + v4 sidecar consensus-profile hashes (`DC-CINPUT-06`). |
| `ade_runtime::seed_consensus_merge` (`seed_consensus_merge.rs` **+103**) | **RED**, ECA | Seed-merge plumbing for the v4 seed sidecar + the native bootstrap path. |
| `ade_ledger::rules` (`rules.rs` **+96**) | **BLUE**, EVIEW | The mark-snapshot path consumes the per-pool aggregation (replacing the `new_mark` zero-fill on the EVIEW path; the non-EVIEW path stays behavior-invariant). |
| `ade_runtime::forward_sync::reducer` (`forward_sync/reducer.rs`) | **RED/GREEN**, DC-MEM-11 | Forward-sync admit reuses a per-loop `UtxoFpCache` of the constant UTxO-component fingerprint instead of an O(n) per-block recompute; structural rollback invalidates the cache. The LIVE-FOLLOW-THROUGHPUT fix. |
| `ade_node::operator_forge` (`operator_forge.rs`) | **RED**, OP-OPS-04 | `load_operator_producer_shell` threads the current absolute KES period (from the injected slot + genesis KES anchor, no wall-clock) into shell init. |
| `ade_node::admission::{runner, bootstrap}` (`bootstrap.rs` **+134**, `runner.rs`) | **RED/GREEN**, DC-WAL-05 + EVIEW | Persist admitted block bytes before the WAL (`DC-WAL-05`); the EVIEW bootstrap reduced-checkpoint build (`build_live_reduced_checkpoint` before `drop(utxo)`); the native-firstrun hooks. |
| `ade_ledger::pparams` (`pparams.rs`) | **BLUE**, DC-LEDGER-PARAMS-01 | `MinUtxoRule` replaces `min_utxo_value: Coin` across all consumers; legacy payload byte-identical. |

> **BLUE was touched (load-bearing).** The bands' BLUE work is in `ade_ledger` (the `stake_ref`/`pointer_resolve`/
> `reduced_*`/`bootstrap_manifest`/`ledgerdb_*`/`mithril_utxo_materialize` modules + `rules.rs` mark path + `wal/event.rs`
> grammar + `value.rs`/`mary.rs`/`pparams.rs` representation + `delegation.rs` lifecycle) and `ade_codec`
> (`address::pointer`). It is the home of the +38 BLUE canonical types. **`ade_core` is UNTOUCHED (`48 → 48`)** — the
> consensus authority `select_best_chain` / `validate_and_apply_header` is byte-identical; EVIEW is a *projection* of the
> single ledger authority, not a change to it. The non-EVIEW path stays behavior-invariant (the `rules.rs` mark change is
> EVIEW-gated; the legacy min-UTxO/value payload is byte-identical).

## 4. Feature Flags

**No project feature-flag deltas in any band.** Ade declares no `[features]` table in any workspace `Cargo.toml` at any
ref (`git grep '^\[features\]'` empty at `470f9b89` and `cdcd9397`). No band introduces a `#[cfg(feature = …)]` gate or a
`compile_error!` coupling.

> **The activation gate is GONE — there is now nothing flag-like to report on the activation path.** At the prior regen
> point the EVIEW activation was governed by a plain `const bool` `EVIEW_ACTIVATION_ARMED = false` (a **semantic gate**,
> not a feature flag) read at runtime. **`a17c7aab` (ECA-1) removed it.** As of HEAD there is **no const, no `armed`
> parameter, no env var, no build feature, no CLI option** anywhere in `crates/` that decides *whether* the epoch-view
> activation occurs — the only gate is the activation predicate over canonical durable state. The transient-view
> directory still has **no runtime `--transient-view-dir` flag** (the override is `#[cfg(test)]`-only). `cli.rs` changed
> (band 1: KES-period plumbing; band 4: native-bootstrap seams) but added **no new user-facing `--feature` surface**.
> There is no feature-flag coupling to report.

## 5. CI Checks (200 → 238 over the full span; +38 new, 0 modified, 0 removed)

Across the span, **38** `ci_check_*.sh` scripts were added, **0** materially modified, **0** removed
(`git ls-tree -r --name-only <ref> ci/ | grep -c ci_check_.*\.sh` = **200 → 238**; `--diff-filter=D` over `ci/` empty).
One non-`ci_check` helper was also added (`ci/capture_mithril_documented_evidence.sh`, the Mithril capture runner — not
an invariant gate).

### EVIEW substrate (band 2 — +21 gates)

| Check | Status | What it checks |
|-------|--------|----------------|
| `ci_check_eview_stake_ref_classification.sh` | **New** (`DC-EVIEW-02`) | The typed, era-gated address→`StakeRef` classifier is exhaustive; no fixed byte offset is authoritative; pointer pre-Conway only. |
| `ci_check_eview_pointer_compat.sh` | **New** (`DC-EVIEW-03`) | The era-parameterized pointer decode (`Ptr`/`PointerDecodeError`) + pointer→credential resolution. |
| `ci_check_eview_reduced_utxo_checkpoint.sh` | **New** (`DC-EVIEW-04`) | The durable reduced-UTxO checkpoint is disk-backed, crash-safe (marker LAST), fingerprint a hash chain in `TxIn` order; replay-equivalent; never authority / never on the live path. |
| `ci_check_eview_windowed_advance.sh` | **New** (`DC-EVIEW-04b`) | The per-block windowed advance matches a direct build. |
| `ci_check_eview_stake_aggregation.sh` | **New** (`DC-EVIEW-05`) | Per-pool stake aggregation (`sum_base_credential_stake` → `aggregate_pool_stake`) is correct (the linchpin); the S3c live differential-oracle result. |
| `ci_check_eview_stability_gate.sh` | **New** (`DC-EVIEW-06`) | Snapshot formation + the k-immutability stability gate (Mark/Set/Go). |
| `ci_check_eview_view_binding.sh` | **New** (`DC-EVIEW-07`/`08`) | The `EpochConsensusView` is bound to all of {network, era, epoch, point, commitment, nonce, phase, canonical-hash}; a view missing any binding is inert. |
| `ci_check_eview_bootstrap_cert_state.sh` | **New** (`DC-EVIEW-09`) | The manifest-bound bootstrap cert-state import — the window starts from Ade's own complete state. |
| `ci_check_eview_window_driver.sh` | **New** (`DC-EVIEW-10`) | The window driver sequences advance + aggregate deterministically. |
| `ci_check_eview_epoch_rebind.sh` | **New** (`DC-EVIEW-11`) | The deterministic fail-closed epoch-rebind seam (strengthening `DC-EPOCH-03`). |
| `ci_check_eview_activation_wal.sh` | **New** (`DC-EPOCH-04`) | The `WalEntry::EpochConsensusViewActivated` record round-trips canonically. |
| `ci_check_eview_activation_predicate.sh` | **New** (`DC-EPOCH-05`/`07`) | The activation predicate + the atomically-published active view. |
| `ci_check_eview_activation_recovery.sh` | **New** (`DC-EPOCH-06`) | Activation is durable-before-visible (WAL→publish) + warm-start reconstructs the same active view from the WAL + bound artifacts alone. |
| `ci_check_eview_source_window.sh` | **New** (`DC-EPOCH-08`) | The source window is lineage-pinned, complete/ordered/bounded; missing/duplicate/out-of-window fail closed; the explicit source→target mapping (no inline `source + k`). |
| `ci_check_eview_candidate.sh` | **New** (`DC-EPOCH-09`) | The candidate is derived from a VALIDATED window; binding before WAL; NO peer/network/wall-clock influence. |
| `ci_check_eview_activate.sh` | **New** (`DC-EPOCH-10`) | The boundary activation orchestration — the sequenced flip; HALT on any terminal. |
| `ci_check_eview_activation.sh` | **New** (activation umbrella) | The end-to-end activation path is sequenced (derive→predicate→WAL→publish) and fail-closed. |
| `ci_check_eview_live_checkpoint.sh` | **New** (`DC-EPOCH-11`) | The live reduced checkpoint is built at bootstrap BEFORE `drop(utxo)`, reduces via `reduce_txout`, is a disk-backed redb gated on the EVIEW cert-state (non-EVIEW bootstrap byte-identical), advances per durable admit, reorg re-materializes, fail-closed on readiness. |
| `ci_check_transient_view_memory_ceiling.sh` | **New** (`DC-EVIEW-01`) | The transient replay window respects a bounded memory ceiling. |
| `ci_check_transient_view_no_fallback.sh` | **New** (`DC-EVIEW-01`) | The transient replay UTxO NEVER becomes an implicit fallback authority for normal live follow/forge. |
| `ci_check_transient_view_not_live.sh` | **New** (`DC-EVIEW-01`) | The transient view is not on the live path; GREEN execution support, pruned after forming the view. |

### EPOCH-CONTINUITY-ACTIVATION (band 3 — +4 gates)

| Check | Status | What it checks |
|-------|--------|----------------|
| `ci_check_eview_automatic_activation.sh` | **New** (`DC-EPOCH-13`) | **No semantic activation gate exists** — no `EVIEW_ACTIVATION_ARMED`, no `armed` parameter, no `if !armed`, no env/feature/CLI switch in `crates/`; activation is automatic + deterministic from canonical state. **The gate-removal enforcement.** |
| `ci_check_eview_leadership_complete.sh` | **New** (`DC-EVIEW-12`) | The candidate view is leadership-complete (every included pool carries active stake AND era-correct VRF/leadership) + exclusive projection (`DC-EPOCH-12`). |
| `ci_check_eview_pool_lifecycle.sh` | **New** (`DC-EVIEW-13`) | The reduced-window cert-state pool lifecycle matches cardano-ledger (mark/reap reproduce the mark snapshot). |
| `ci_check_eview_seed_sidecar_v4.sh` | **New** (`DC-CINPUT-06`) | The v4 seed sidecar persists the consensus-profile hashes (`genesis_hash` + `protocol_params_hash`), recovered from the store on warm-start. |
| `ci_check_eview_atomic_authority.sh` | **New** (`DC-EPOCH-14`) | The epoch-authority transition assembles + atomically persists + recovers; warm-start fail-closes on a wrong CLI network magic. |

### Native Mithril bootstrap + ledger types (band 4 — +9 gates)

| Check | Status | What it checks |
|-------|--------|----------------|
| `ci_check_ledgerdb_state_decode.sh` | **New** (`DC-MITHRIL-04`) | Native V2 LedgerDB `state` decode is faithful, fail-closed, non-emitting → canonical `CertState` + pool distr + Praos nonces (raw CBOR is RED input, the projection is authority). |
| `ci_check_ledgerdb_tables_decode.sh` | **New** (`DC-MITHRIL-05`) | Native V2 LedgerDB `tables` MemPack `TxOut` decode → faithful Word64-quantity UTxO. |
| `ci_check_mithril_authority_transition.sh` | **New** (`DC-MITHRIL-03`) | The native Mithril authority transition assembles + atomically persists. |
| `ci_check_tables_to_utxostate.sh` | **New** (`DC-MITHRIL-06`) | Tables → authoritative `UTxOState` materialization. |
| `ci_check_native_firstrun_no_cli_seed.sh` | **New** (`DC-MITHRIL-07`) | The live FirstRun invokes the native Mithril bootstrap (no cardano-cli JSON seed). |
| `ci_check_native_nonutxo_decode.sh` | **New** (`DC-LEDGER-PARAMS-01`-adjacent) | Native non-UTxO snapshot decode + manifest-bound network identity (`network_id_from_magic` + `NetworkIdentityMismatch`); the era-aware `MinUtxoRule`. |
| `ci_check_value_quantity_domain.sh` | **New** (`DC-LEDGER-VALUE-01`) | Output asset quantity is the Word64 domain — never truncated/saturated/cast to i64. |
| `ci_check_mithril_documented_evidence.sh` | **New** (`RO-MITHRIL-IMPORT-01`) | The Mithril documented-interface evidence bundle validates against its schema (positive + negative manifest); the documented-interface import gate. |
| `ci_check_registry_unique_ids.sh` | **New** (registry hygiene) | No duplicate `id =` in the invariant registry — the uniqueness guard added by the `DC-MITHRIL-01/02` collision repair (`e84ebb0c`). |

### Standalone fix band (band 1 — +3 gates)

| Check | Status | What it checks |
|-------|--------|----------------|
| `ci_check_forward_sync_fp_cache.sh` | **New** (`DC-MEM-11`) | Forward-sync admit reuses the cached UTxO fingerprint (no O(n) per-block recompute); structural rollback invalidates the cache; the cached `post_fp` is byte-identical to the full recompute under `track_utxo=false`. |
| `ci_check_participant_forge_on_selected_head.sh` | **New** (`CN-FOLLOW-01` / `DC-FOLLOW-FORGE-01`) | The participant forges on the AO-selected durable head; the forge base derives from the live selected tip, not a self-forge latch; the fence fails closed if the base changes before sign. |
| `ci_check_admission_runner_no_block_byte_map.sh` | **New** (`DC-WAL-05`-adjacent) | The admission runner does not retain a block-byte map (the persist-before-WAL discipline; no accidental retention). |

> **Cross-reference (TRACEABILITY).** All 38 new gates bind to a registry rule — each is named in some rule's
> `ci_scripts` / `code_locus` field (verified: `ci_check_ledgerdb_state_decode.sh` and `ci_check_native_nonutxo_decode.sh`
> do not print the rule ID in their script header but **are** referenced by `DC-MITHRIL-04` / `DC-LEDGER-PARAMS-01` in the
> registry, so they are not orphans). TRACEABILITY at HEAD should reflect this. **Two EVIEW rules remain `declared`**
> (`DC-EPOCH-11`, `DC-EVIEW-08`) — their gates (`ci_check_eview_live_checkpoint.sh`, `ci_check_eview_view_binding.sh`) are
> present and run green over the hermetic substrate, but the rule stays `declared` pending a committed live-flip
> transcript. Record these as "gate present + green; rule `declared` pending the live flip." This is the expected
> mid-cluster state, not an orphan defect.

## 6. Canonical Type Registry Delta

**n/a — no separate canonical-type registry is configured** (`canonical_type_registry: null`); canonical-type rules
live inline in the invariant registry under family **T**. The span added **38 BLUE canonical types** — the structural
`pub struct`/`pub enum` count over the BLUE `core_paths` trees moves **`466 → 504`**.

The **+38**:

- **`ade_codec` +2** (`11 → 13`): `Ptr`, `PointerDecodeError` (`crates/ade_codec/src/address/pointer.rs`).
- **`ade_types` +2** (`82 → 84`): new domain types in the value/min-UTxO refactor surface (`DC-LEDGER-VALUE-01` /
  `DC-LEDGER-PARAMS-01`).
- **`ade_ledger` +34** (`185 → 219`): the EVIEW family (`StakeRefClass`, `StakeRefReject`, `PointerRef`,
  `ReducedStakeRef`, `ReducedBlockDelta`, `StakeByPool`, `AggregateError`, `SnapshotPhase`, `EpochConsensusView`,
  `ViewBindings`, `BootstrapManifest`, `BootstrapManifestError`, …) + the Mithril decode family
  (`LedgerDbStateProbe`, `LedgerDbStateError`, the tables/MemPack decode types, the materialization types) + the value /
  min-UTxO domain (`OutputAssetQuantity`, `MinUtxoRule`).

**`ade_core` is unchanged (`48 → 48`)** — the consensus authority is byte-identical. `ade_crypto` (`22 → 22`),
`ade_plutus` (`8 → 8`), and every `ade_network` BLUE sub-path (`mux/frame.rs` 5, `codec/` 41, `handshake/` 9,
`chain_sync/` 10, `block_fetch/` 9, `tx_submission/` 5, `keep_alive/` 5, `peer_sharing/` 5, `n2c/` 21 — all unchanged)
hold. The RED `ade_runtime` checkpoint/transient/assembly types and the RED `ade_node` `epoch_*`/`native_firstrun`/
`bootstrap_export` orchestration types live outside the BLUE `core_paths` and are **NOT** canonical-counted.

**Zero BLUE canonical types removed** in any band (append-only within the major version).

## 7. Normative / Invariant Rule Delta (380 → 418 full span; **+38, ZERO removals**)

**The span added 38 rule IDs, zero removed** (registry **380 → 418**; `comm -23` of the sorted `id =` lists is empty —
exactly 38 additions, no removal). The status tally moves **enforced 253 → 291** (+38), **partial 23 → 22** (−1),
**declared 103 → 104** (+1) (the `enforced_scaffolding` count holds at 1).

**Band 1 (+8 IDs + 2 flips):** `DC-WAL-05` (persist admitted block bytes before WAL) — enforced; `DC-CINPUT-05`
(warm-start era-schedule from durable venue geometry) — enforced; `DC-MEM-11` (forward-sync cached UTxO fingerprint) —
enforced; `CN-FOLLOW-01` (participant forge on the AO-selected durable head) — enforced; `DC-FOLLOW-FORGE-01`
(participant forge base from the live selected tip) — enforced. **Flips:** `RO-MITHRIL-IMPORT-01` **`partial → enforced`**
(documented evidence gate); `T-CONS-01` **`declared → enforced`** (bound to `CN-CONS-01`). **Strengthenings:**
`OP-OPS-04` (KES shell-init opcert anchor), `DC-CRYPTO-10` (KES-period gate opcert-anchored relative evolution).

**Band 2 EVIEW (+20 IDs):**

- `DC-EVIEW-01` (transient materialization gate — bounded, crash-safe, no-fallback) — **enforced**
- `DC-EVIEW-02` (typed era-gated stake-reference classification) — **enforced**
- `DC-EVIEW-03` (era-parameterized pointer decode/resolution) — **enforced**
- `DC-EVIEW-04` / `DC-EVIEW-04b` (durable reduced-UTxO checkpoint + windowed advance) — **enforced**
- `DC-EVIEW-05` (per-pool stake aggregation + S3c live oracle) — **enforced**
- `DC-EVIEW-06` (snapshot formation + k-immutability stability gate) — **enforced**
- `DC-EVIEW-07` (the bound, immutable `EpochConsensusView`) — **enforced**
- `DC-EVIEW-08` (activation consumption point) — **`declared`** (the live flip is owed)
- `DC-EVIEW-09` (manifest-bound bootstrap cert-state import) — **enforced**
- `DC-EVIEW-10` (the window driver) — **enforced**
- `DC-EVIEW-11` (deterministic fail-closed epoch-rebind seam, strengthening `DC-EPOCH-03`) — **enforced**
- `DC-EPOCH-04` (WAL activation record) — **enforced**
- `DC-EPOCH-05` / `DC-EPOCH-07` (activation predicate + atomically-published view) — **enforced**
- `DC-EPOCH-06` (durable-before-visible + crash recovery) — **enforced**
- `DC-EPOCH-08` (source window + named source→target mapping) — **enforced**
- `DC-EPOCH-09` (candidate derivation from a validated window) — **enforced**
- `DC-EPOCH-10` (boundary activation orchestration — the sequenced flip) — **enforced**
- `DC-EPOCH-11` (the live reduced checkpoint) — **`declared`** (the live flip is owed)

**Band 3 ECA (+5 IDs):** **`DC-EPOCH-13`** (no semantic activation gate — the `EVIEW_ACTIVATION_ARMED` removal) —
**enforced**; `DC-EPOCH-12` (exclusive projection / leadership-complete) — enforced; `DC-EPOCH-14` (warm-start
fail-closes on wrong CLI network magic) — enforced; `DC-EVIEW-12` (leadership-complete self-contained view) — enforced;
`DC-EVIEW-13` (cardano-faithful pool lifecycle) — enforced; `DC-CINPUT-06` (v4 seed sidecar consensus-profile hashes) —
enforced.

**Band 4 Mithril + ledger (+7 IDs):** `DC-MITHRIL-03` (native authority transition assemble + atomic persist) —
enforced; `DC-MITHRIL-04` (native V2 LedgerDB state decode → canonical CertState) — enforced; `DC-MITHRIL-05` (native V2
LedgerDB tables MemPack TxOut decode → faithful Word64 UTxO) — enforced; `DC-MITHRIL-06` (tables → authoritative
`UTxOState`) — enforced; `DC-MITHRIL-07` (live FirstRun → native Mithril bootstrap) — enforced; `DC-LEDGER-VALUE-01`
(Word64 output asset quantity) — enforced; `DC-LEDGER-PARAMS-01` (era-aware min-UTxO representation) — enforced. **Note:
`DC-MITHRIL-01` and `DC-MITHRIL-02` are NOT new** — both pre-existed at baseline (the cross-impl Mithril obligations);
band 4 built the decoders that the new `-04`/`-05` enforce. The `e84ebb0c` commit repaired an in-span `-01`/`-02` ID
collision and added the `ci_check_registry_unique_ids.sh` guard; at HEAD the registry has zero duplicate IDs (418 total
`id =` == 418 unique).

*(The configured `normative_docs` — the CE-79 tier-gate statement + addendum, the three contract docs, the CE-73
reclassification, and `CLAUDE.md` — were **not** changed anywhere in the span: `git diff --name-only 470f9b89..cdcd9397`
over those paths is empty. The §7 delta is entirely the invariant-registry change.)*

This section is informational and approximate where it counts attributes; the rule IDs, status values, and the
zero-removal result are exact (read from the registry at HEAD).

---

## Anomalies & cross-reference summary (surface prominently)

- **THE ACTIVATION GATE WAS REMOVED — earlier "INERT" framing is now stale.** `a17c7aab` (ECA-1) deleted the
  `EVIEW_ACTIVATION_ARMED` semantic gate; epoch-view activation is now **AUTOMATIC and DETERMINISTIC from canonical
  durable state** (the activation predicate is the only gate). Verified: `git grep EVIEW_ACTIVATION_ARMED` is empty in
  `crates/` at HEAD `cdcd9397` (it was present 6× at the intermediate `a50a3ee8`). Enforced by **`DC-EPOCH-13`** + gate
  `ci_check_eview_automatic_activation.sh`. **Any doc still describing the epoch transition as INERT / gated /
  byte-identical-live-path is describing `a50a3ee8`, NOT HEAD.**
- **No cluster closed in the span — the baseline is NOT advanced** (`head_deltas_baseline` stays `470f9b89`; the on-disk
  `.idd-config.json` is untouched). Every commit is mid-cluster `feat(epoch)` work across four bands. The next baseline
  bump waits for a cluster-close (and must name a pushed close commit).
- **Zero canonical-type removals; zero rule removals** across the span (both expected: 0). The registry now also enforces
  ID uniqueness mechanically (`ci_check_registry_unique_ids.sh`) after the in-span `DC-MITHRIL-01/02` collision repair.
- **Two new EVIEW rules land `declared`** (`DC-EPOCH-11`, `DC-EVIEW-08`) — inherited from band 2; their gates run green
  over the hermetic substrate, but the registry flip awaits a committed live-flip transcript. This is correct, not a
  discipline lapse. (Note the asymmetry: activation is *mechanically automatic* via `DC-EPOCH-13`, yet `DC-EPOCH-11`/
  `DC-EVIEW-08` stay `declared` pending the live evidence — the gate enforces the substrate + the no-semantic-gate
  property, not a captured live epoch flip.)
- **`DC-MITHRIL-01`/`-02` are pre-existing, not new** — surfaced because the band-4 commits reference them in their
  subjects (Stage 1 / Stage 2). The genuinely new Mithril IDs are `-03`/`-04`/`-05`/`-06`/`-07`.
- **All 75 commits carry a clear conventional scope** — 3 use a non-`feat/fix/...` but unambiguous prefix
  (`harden(node):`, `registry:`, `evidence:`), surfaced rather than guessed. None is unclassifiable.
- **Pre-existing CI-hygiene blocker noted (`7c769801`):** a pre-existing `all_epoch_boundaries_fire` hang in
  `ade_testkit::epoch_boundary_logic` — band-4 gating runs on targeted suites, not `cargo test -p ade_testkit`. Not
  introduced by this span; tracked so the full-workspace replay command (`replay_cmd = cargo test -p ade_testkit`) is
  known to hang until fixed.

---

## Generation notes

### Regen `470f9b89 → cdcd9397` (mid-flight refresh across four bands — current lead)

- **Baseline valid; NOT a cluster-close refresh.** Run against `470f9b89` (the MEM-OPT-UTXO-DISK cluster-close), which
  `git rev-parse` resolves and `git merge-base 470f9b89 HEAD == 470f9b89` confirms is a strict ancestor of HEAD
  `cdcd9397` (`470f9b89` carries no tag). **No cluster closed in the span** — every commit is mid-cluster `feat(epoch)`
  work — so per IDD discipline the baseline is **NOT advanced**; `.idd-config.json` `head_deltas_baseline` stays
  `470f9b89` (on-disk config untouched). This regen replaces the stale on-disk doc that stopped at `a50a3ee8` (mid-EVIEW,
  before the ECA gate-removal and the native-Mithril band).
- **Counts are mechanical (git/grep/ls).** Commit log + `--shortstat` over `470f9b89..cdcd9397` (**75** commits, no
  merges / **179** files / **+30,013 / −439**); CI gate count via `git ls-tree -r --name-only <ref> ci/ | grep -c
  ci_check_.*\.sh` = **200 → 238** (`git diff --name-status -- 'ci/ci_check_*.sh'`: 38 `A`, 0 `M`, 0 `D`); registry rule
  count via `grep -c '^id = '` (**380 → 418**; `comm -23` of sorted `id =` lists empty — zero removals; `comm -13` = 38);
  registry status via `grep '^status = ' | sort | uniq -c` (enforced/scaffolding/partial/declared
  **253/1/23/103 → 289/1/22/104**); canonical types via `git grep -hE '^\s*pub (struct|enum) '` over the BLUE `core_paths`
  crate `src/` trees + the listed `ade_network` sub-paths (**466 → 504**; `ade_ledger 185 → 219`, `ade_codec 11 → 13`,
  `ade_types 82 → 84`, `ade_core 48 → 48`); tests via the `#[test]`/`#[tokio::test]` attribute grep
  (**2640 → 2934**, approximate).
- **The activation gate is GONE (verified in source).** `git grep EVIEW_ACTIVATION_ARMED cdcd9397 -- crates/` is empty
  (6 hits at `a50a3ee8`: `epoch_wire.rs` ×1, `node_lifecycle.rs` ×5). The removal commit is `a17c7aab` (ECA-1). Surfaced
  in the header, the headline, §0, Band 3, §4, and the anomalies block as the single most important delta.
- **Crate count unchanged (12 → 12).** `diff` of the `"crates/"` member lists at both refs is empty; no new `Cargo.toml`
  (`--diff-filter=A '**/Cargo.toml'` empty). `ade_mem_diag` + `ade_core_interop` already exist at baseline.
- **No feature flag, no `compile_error!`, no new user-facing CLI flag.** No `[features]` table at any ref; the former
  activation gate was a `const bool` (now removed); `cli.rs` changed only for KES-period + native-bootstrap
  argument-threading.
- **Normative docs unchanged across the span.** `git diff --name-only 470f9b89..cdcd9397` over the configured
  `normative_docs` is empty — the §7 delta is entirely the invariant-registry change.
- **§1 commit log verbatim from `git log` (newest first).** All 75 carry a clear conventional scope (`feat`×48 /
  `docs`×12 / `fix`×8 / `chore`×2 / `test`×1 / `perf`×1 + the 3 non-standard harden/registry/evidence prefixes); the 3 non-standard prefixes (`harden:`/`registry:`/`evidence:`)
  are unambiguous from scope, surfaced not guessed.
- **Coordinated grounding-doc refresh — all four reflect HEAD `cdcd9397`.** CODEMAP + SEAMS + TRACEABILITY + this doc
  regenerate together. **Cross-reference:** the 11 new BLUE/GREEN/RED source modules + the `epoch_*`/`native_firstrun`/
  `bootstrap_export` shell belong in CODEMAP (§2); all 38 new CI gates bind a registry rule, including the 2
  `declared`-rule gates which run green over the hermetic substrate (§5). Prefer regenerating CODEMAP/TRACEABILITY over
  patching if any module or gate is missing.
