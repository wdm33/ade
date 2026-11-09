# Seams — Where New Work Can Attach (Ade)

> **Status:** Living architectural document. Regenerated; not hand-edited.
> Per-project instance of `~/.claude/methodology/templates/seams.md`, in IDSE v3.0 form.

> 12 modules, 637 canonical types, 286 CI checks, 260 invariants at HEAD (`08c7c3f5`). CODEMAP read: `docs/ade-CODEMAP.md` at the same HEAD.

**Regenerated for IDSE.** This is the first SEAMS since Ade migrated from legacy IDD to IDSE v3.0 (`strict`, namespace `ADE`; `docs/planning/idse-migration-plan.md`, step M2). The previous SEAMS was written at `1e4896eb` under IDD. Every invariant reference below is an IDSE ID from `docs/ade-invariant-registry.toml` (260 invariants, assumptions A1–A7). Legacy IDs are not cited; to follow one quoted in a source comment, use `docs/legacy/idd-to-idse-map.toml`.

**How to read the citations.** "(VAL-024)" means the registry entry whose `enforcement_runtime` loci or `ci_scripts` name the construct, or whose statement names it. No IDSE cluster has run yet, so every cited entry is `partial` or `unresolved` (PRC-001 is `falsified`). A citation records which invariant governs a seam. It does not claim the invariant is established (Doctrine §10). Items marked **no registry locus** are governed only by their source banner and CI gate. They are listed under §7.

This document describes **the closure surface of the system**: where new work can attach safely, where it cannot, and what shape attachments must take. CODEMAP says what each module *is*; SEAMS says where the system *opens*.

---

## 1. Surface Reduction Rules

> External inputs reduce to canonical form before entering authoritative pipelines (Doctrine §17; STR-002, STR-004). Ade has eleven external surfaces: the network in four forms, argv, three operator-file routes, durable-store rehydration, the wall clock and the process environment. Each one is listed below with its reduction.

**Rule:** New ingress surfaces attach by producing the canonical type's bytes and entering the same authoritative pipeline. They **may not** introduce new pipeline steps, reorder existing steps, or shortcut into the core via a back door (STR-004, STR-014, DUR-012).

### Surface 1: N2N client feed (dialed peers; chain-sync + block-fetch + keep-alive) — confirmed

```
Surface: N2N inbound feed from each --peer (TCP, mux, N2N handshake)
Reduces to: AdmissionPeerEvent {Block | TipUpdate | RollBackward | Disconnected} (RED ade_runtime::admission::wire_pump)
            -> NodeSyncItem {Block{peer, bytes} | RollBack{peer, point}} (RED ade_node::node_sync)
            -> decoded Conway block (BLUE ade_ledger::block_validity::decode_block over ade_codec)
            -> BlockVerdict (BLUE ade_ledger::block_validity)
Pipeline (fixed; no step may be reordered):
  1. mux frame decode                               BLUE ade_network::mux::frame::decode_frame (STR-012)
  2. handshake, then per-mini-protocol reassembly   GREEN ade_network::session::step (SAF-011, DET-018, VAL-048, SEC-038)
  3. tag-24 unwrap                                  BLUE ade_codec::unwrap_tag24, the one envelope authority (VAL-002)
  4. one wire pump per peer, closed 4-event output  RED wire_pump (STR-019, VAL-061, LIV-008, ORD-006, LIV-009)
  5. bounded round-robin merge of peer lanes        GREEN-by-content ade_node::fair_merge (SEC-028; PER_PEER_LANE_CAP = 64)
  6. peer-tagged sync item                          RED node_sync (OBS-003; MAX_WIRE_PUMP_LOOKAHEAD = 256)
  7. one receive classifier                         node_sync::classify_receive -> ReceiveDisposition
                                                    {AlreadyHave | LinearExtend | RefuseSingleProducer | NeedsForkChoice} (VAL-054, SAF-015, SAF-018)
  8a. LinearExtend: header, then body, then durable admit
                                                    validate_and_apply_header -> block validity -> ade_runtime::forward_sync::pump::pump_block
                                                    (ORD-002, VAL-009, DUR-010, DUR-012, DUR-013, REC-003)
  8b. NeedsForkChoice: candidate from own-validated headers -> select_best_chain -> prove the branch, then commit
                                                    (SEC-021, VAL-031, SEC-022, SEC-023, STR-016, REC-006)
  8c. RollBackward: target resolved from the stored chain point -> admit_rollback -> materialize_rolled_back_state + commit_rollback
                                                    (SEC-020, SAF-016, SAF-017, SAF-007, STR-011, REC-003)
Cross-surface state sharing: the per-peer lanes share one deterministic merge cursor (SEC-028). Rollback targets bind to the
durable ChainDb's stored slot and hash, never to a peer-supplied pair (SEC-020). FindIntersect points come only from the
durable chain, validated back from the tip (VAL-055, LIV-007).
```

After each durable admit, the same chokepoint advances the epoch accumulator and the reduced UTxO checkpoint in log order (REC-002). It also evolves the Praos nonce through one indivisible transition per validated header (VAL-022). None of this adds a pipeline step: it is a projection fixed to `pump_block`, and it never runs on a block that was delivered but not admitted.

### Surface 2: N2N server side (inbound peers on `--listen`) — confirmed

```
Surface: N2N listener (ade_runtime::network::n2n_listener), one session per inbound peer
Reduces to: OrchestratorEvent::{PeerN2nServerChainSyncFrame | PeerN2nServerBlockFetchFrame}
            -> BLUE server reducers ade_network::{chain_sync, block_fetch}::server
            -> OutboundCommand {ChainSync | BlockFetch | ClosePeer} (RED, typed, per-peer)
Pipeline:
  1. handshake against the closed version table (EVO-006, VAL-004)
  2. the one serve-dispatch core: ade_runtime::network::serve_dispatch (VAL-050, SAF-020; ci_check_single_serve_dispatch_authority.sh)
  3. reads only the durable projection: ChainDbServedSource, capped at MAX_SERVE_RANGE_BLOCKS = 256 (VAL-050, SEC-036)
  4. BLUE reducer reply (DET-017, VAL-045, VAL-046, VAL-047, SEC-016)
  5. typed outbound command on that peer's own channel only (SEC-017)
Cross-surface state sharing: the served chain is the durable adopted chain, published atomically (SAF-010). No peer session
shares working state with another (SEC-019). The serve side changes no authoritative state.
```

### Surface 3: transaction ingress (N2N TxSubmission2, N2C LocalTxSubmission) — chokepoint confirmed; live-node wiring is a candidate

```
Surface: transaction bytes from any source
Reduces to: ade_ledger::mempool::ingress::IngressEvent { source: IngressSource {N2N | N2C}, tx_bytes }
Pipeline:
  1. bounded inbound fold within MAX_INBOUND_ADMISSION_COUNT = 1024 / MAX_INBOUND_ADMISSION_BYTES = 4 MiB (SEC-035)
  2. mempool_ingress(mempool, event) -> admit (BLUE; the verdict never branches on `source`) (STR-022, VAL-067, DET-024)
Cross-surface state sharing: one mempool state; the source is metadata for evidence, policy and replay only.
```

**Candidate:** at HEAD, `mempool_ingress` is reached only from the `ade_core_interop` live bridges (`tx_submission.rs`, `local_tx_submission.rs`, `live_tx_submission_session`) and from `ade_node::mem_measure`. `--mode node` does not route peer transactions into it. See §7.

### Surface 4: N2C local socket — candidate

The BLUE codecs and state machines for every node-to-client mini-protocol exist (`ade_network::codec::{n2c_handshake, local_chain_sync, local_tx_submission, local_state_query, local_tx_monitor}`, `ade_network::n2c::*`; VAL-044, EVO-006 with `N2C_SUPPORTED` v16–v23). No crate under `src/` opens a Unix listener. The only `UnixStream` users are the RED capture binaries `ade_network/src/bin/capture_n2c_*`. **The node exposes no N2C ingress at HEAD**, so this surface has no reduction pipeline to state. `_(needs human judgment)_`: VAL-044 says Ade implements the N2C set "in the roles the reference node plays". See §7.

### Surface 5: argv — confirmed

```
Surface: argv
Reduces to: ade_node::cli::{Cli, Mode}. Mode is closed, 7 variants:
            WireOnly | Admission | KeyGenKes | Produce | Node | BootstrapExport | MithrilSnapshotFetch
Pipeline: parse (closed flag set; an unknown action is terminal) -> Mode dispatch in main() -> the one driver per mode
Cross-surface state sharing: none.
```

The `ade node run …` and `ade mithril snapshot fetch …` subcommands are parsed into the same `Mode` set. They do not add a mode. `--mode node` is the single live-lifecycle owner (STR-014). A flag selects a route, never a semantics (EVO-001, DET-006). The native route refuses `--json-seed-path` and `--consensus-inputs-path` as terminal `NativeRouteForbiddenFlag` errors (SEC-030).

### Surface 6: operator key, opcert and genesis files — confirmed (one warning)

```
Surface: cold / KES / VRF signing keys, operational certificate, Shelley (and companion) genesis files
Reduces to: ColdSigningKey / KesSecret / VrfSigningKey (RED ade_runtime::producer::keys, ade_kes_envelope; SEC-006, SEC-001)
            DecodedOpCertEnvelope (RED producer::opcert_envelope; SEC-006)
            GenesisAnchor / EraSchedule bound to BootstrapAnchorHash (RED genesis parsers -> BLUE ade_core::consensus::era_schedule; STR-009, VAL-039)
Pipeline: read bytes -> one closed RED parser per grammar -> canonical type -> ProducerShell (keys never leave it) / bootstrap authority
Cross-surface state sharing: forging is enabled only for the complete operator set (SEC-025). The forge slot authority is selected
by the durable genesis hash, so operator genesis values act only as fail-closed cross-checks (DET-019).
```

**Cross-reference warning:** VAL-039 requires *one* closed Shelley-genesis parser. Three exist at HEAD: `ade_runtime::producer::genesis_parser::parse_shelley_genesis` (the one VAL-039's loci name), `ade_runtime::consensus::genesis_parser::parse_genesis` (era schedule) and `ade_node::native_firstrun::parse_native_shelley_genesis` (native route constants). See §7.

### Surface 7: native Mithril snapshot (acquisition + first-run directory) — confirmed

```
Surface A: ade mithril snapshot fetch --network <preview|preprod>   (RED ade_node::mithril_fetch; the only code that talks to Mithril)
Reduces to: a local verified directory (ledger state + UTxO tables) + manifest.json + receipt. These are bytes, not authority.
Surface B: ade node run --snapshot-dir <dir>   (native first run)
Reduces to: canonical CertState + UTxOState + SeedEpochConsensusInputs, bound to one manifest point
Pipeline (fixed):
  1. snapshot point coherence: certified point == the ledger state's own tip; era is Conway (SEC-031)
  2. BLUE decoders: ledgerdb_state::decode_native_nonutxo_state (VAL-063), ledgerdb_tables + mithril_utxo_materialize (VAL-064)
  3. manifest binding of separately typed seed / cert-state / UTxO seed (SEC-032; BootstrapManifest v2)
  4. GREEN assembly: ade_runtime::mithril_native_assembly::assemble_native_mithril_seed (SEC-030, VAL-066)
  5. reduced checkpoint sealed at the certified slot before the UTxO is consumed (VAL-065)
  6. the one bootstrap authority: ade_runtime::bootstrap::bootstrap_initial_state (STR-013, SEC-015)
  7. commit point: the WAL provenance append (DUR-014, DUR-004)
Cross-surface state sharing: none at fetch time. The network fetch grants bytes; the BLUE decoders and the manifest binding
decide what they mean (SEC-034).
```

### Surface 8: cardano-cli seed + consensus-input bundle route — confirmed

```
Surface: --json-seed-path (cardano-cli UTxO dump), --consensus-inputs-path (cardano-cli JSON), --mithril-manifest-path
Reduces to: canonical UTxOState + fingerprint (GREEN ade_runtime::seed_import; DET-022, VAL-062)
            SeedEpochConsensusInputs v6 (GREEN ade_runtime::consensus_inputs importer; VAL-034, VAL-035, STR-010)
            BootstrapAnchor via the one mint (STR-021)
Pipeline: import -> Mithril-manifest cross-check against the anchor from the separately extracted seed (SEC-029)
          -> bootstrap_initial_state (STR-013) -> WAL fact (DUR-004)
Cross-surface state sharing: the seed record is written only by verified-bootstrap paths (SEC-014).
```

This route serves `--mode node` first run (`first_run_mithril_bootstrap`) and `--mode admission` (STR-020). The tooling that produces its bundle is `--mode bootstrap_export` (RED `ade_node::bootstrap_export`, §2).

### Surface 9: durable-store rehydration — confirmed

```
Surface: chain.db (blocks, WAL, snapshots, sidecars), epoch-accumulator.redb, reduced-checkpoint.redb
Reduces to: WalEntry (closed, 4 variants), snapshot payloads behind a version tag, the accumulator blob,
            FrozenLeadershipPoolDistr, SeedEpochConsensusInputs, RecoveredAnchorPoint
Pipeline (fixed):
  1. semantics marker check per artifact, before any recovery work (EVO-005; check_store_semantics_version)
  2. layout and fingerprint version checks (ChainDb FINGERPRINT_VERSION, fail-closed both ways)
  3. lineage binding to one bootstrap anchor (DUR-008)
  4. snapshot version tag before payload; fingerprint recomputed (DET-015, DET-011)
  5. WAL fingerprint-chain fold from the anchor (DUR-009)
  6. one bootstrap authority, warm-start branch (STR-013, VAL-041, DUR-005, REC-007)
Cross-surface state sharing: one admission prefix defines ChainDb, chain-dependent state, accumulator and checkpoint (REC-002).
```

### Surface 10: wall clock — confirmed

```
Surface: wall-clock time
Reduces to: SlotNo, through one conversion: BootstrapBoundTimingAuthority::slot_at (BLUE ade_core::consensus::era_schedule)
Pipeline: SystemClock (RED ade_runtime::clock; the only SystemTime::now in ade_runtime, ci_check_clock_seam.sh)
          -> captured ms -> slot over the committed venue calendar chosen by the durable genesis hash (DET-019)
Cross-surface state sharing: only the slot crosses into the orchestrator core (DET-019, DET-012).
```

`ade_node::node_lifecycle` also reads `Instant::now` for timing diagnostics. Those reads feed logs only (OBS-002).

### Surface 11: process environment — candidate

Production `src/` reads two environment inputs. `ADE_MEM_PHASE_DIAGNOSTIC` (in `ade_node::admission::bootstrap`) gates the quarantined diagnostic. `ADE_MITHRIL_AGGREGATOR`, `ADE_MITHRIL_GENESIS_VKEY` and `ADE_MITHRIL_ANCILLARY_VKEY` (in `ade_node::mithril_fetch::resolve_mithril_profile`) override the committed Mithril profile, and the preprod keys are empty, so they must be supplied this way. The `CE4A_*` reads in `node_lifecycle.rs` sit in its `#[cfg(test)]` module. `_(needs human judgment)_`: SEC-034 binds verification to "the selected network's committed Mithril keys". See §7.

---

## 2. Data-Only vs. Authoritative Layers

> For each domain where a tooling or transport layer meets an authoritative layer, the boundary is named. **The chokepoint never moves.** Colors follow CODEMAP: crate color from `.idse-config.json` `core_paths`, and sub-tree color from file banners.

### Domain: ledger transition and the epoch accumulator — confirmed

| Layer | Module | Color | Role |
|-------|--------|-------|------|
| **Authoritative transition** | `ade_ledger::epoch_accumulator::apply_selected_block` (+ `_with_resolver`) | BLUE | The one non-UTxO ledger transition. Boundaries come first, in NEWEPOCH order (VAL-020, SAF-005, VAL-025). Fails with the closed `LedgerTransitionError` (21 variants). |
| **Durable cache of a derivable value** | `ade_runtime::chaindb::epoch_accumulator_store` | RED store, BLUE-derivable content | Settled triple bound by fingerprint (DUR-002). Rewind only to a settled canonical point (REC-001, SAF-006). Refold is replay-equivalent and bounded (DET-009, LIV-001). Semantics marker (EVO-005). |
| **Loader / advance** | `ade_runtime::chaindb::epoch_accumulator_advance` | RED | Sequences `apply_selected_block` over the durable chain in WAL order (REC-002). Retries once only for the checkpoint-lag case (LIV-002). |

**Rule:** new epoch effects go into `apply_selected_block` and the BLUE boundary. The accumulator holds no full UTxO map (DUR-003; `ci_check_epoch_accumulator_no_utxo.sh`). A lost or stale store is rebuilt by replay, never read as a fallback (DET-010).

### Domain: leadership authority — confirmed

| Layer | Module | Color | Role |
|-------|--------|-------|------|
| **Sealed authority** | `ade_ledger::frozen_leadership::{FrozenLeadershipPoolDistr, canonical_hash, to_pool_distr_view}` | BLUE | Sealed for E+2 in the same commit as the boundary advance (VAL-023). It is the only source for a leader check in epoch E (VAL-024). The hash is recomputed before it is read (SAF-009). |
| **Store** | the epoch-indexed leadership table in `epoch_accumulator_store` | RED | Atomic with the accumulator (SAF-006). |
| **Consumers** | `ade_core::consensus::leader_check`, `ade_node` forge path | BLUE / RED | Same leader-check functions for validation and forge (VAL-056, VAL-036). Forge epoch bound to the active authority (SAF-021). |

**Rule:** leadership is never re-derived at use time. A new binding field means a schema bump (§4).

### Domain: reduced follower vs. authoritative boundary — confirmed (one candidate link)

| Layer | Module | Color | Role |
|-------|--------|-------|------|
| **Authoritative boundary** | `ade_ledger::rules::apply_epoch_boundary_with_registrations` | BLUE | The only producer of reward-applied pots, mark/set/go and enactment (VAL-020). |
| **Typed non-authority** | `ade_ledger::reduced_boundary::ReducedBoundaryProjection` (+ `ReducedCertProjection::Unavailable`, `ReducedGovernanceProjection::Unavailable`) | BLUE | Epoch and slot progression plus block-window rollover only. Its banner says it is not a `LedgerState` and cannot be widened, serialized as an accumulator or fingerprinted as authority. **No registry locus** (§7). Gates: `ci_check_reduced_boundary_plane.sh`, `ci_check_trusted_replay_boundary.sh` (both named by no registry entry). |
| **Reduced UTxO checkpoint** | `ade_runtime::chaindb::reduced_utxo_checkpoint` | GREEN cache in a RED crate | Mirrors the ledger's own UTxO application (DET-013). Retains collateral values (DUR-011). Serves as the `CollateralValueResolver` (CON-003). |

### Domain: Conway governance ratification — confirmed

| Layer | Module | Color | Role |
|-------|--------|-------|------|
| **Loader** | `ade_ledger::snapshot::gov_state`, `ledgerdb_state` (snapshot import, `RatificationSource::ImportedFromSnapshot`) | BLUE | Refuses an inconsistent import with `RatificationImportRefusal` (3). |
| **Authoritative** | `ade_ledger::governance::frozen::{ratify_frozen, apply_ratification_record}` | BLUE | Decided once from the prior boundary and applied atomically, without rereading votes or stake (VAL-021). Deposits are returned exactly once (CON-004). Vote-delegation lifecycle (VAL-030). Typed proposals (STR-006). |

**Rule:** a new governance effect goes into the frozen ratification record and its single application. Gate: `ci_check_frozen_ratification_authority.sh`.

### Domain: consensus-input import — confirmed

| Layer | Module | Color | Role |
|-------|--------|-------|------|
| **Data-only tooling** | `ade_runtime::consensus_inputs::{json, importer}` | GREEN (banner) in a RED crate | Parses the cardano-cli JSON. A missing field fails closed, with no default (VAL-034). |
| **Authoritative record** | `ade_ledger::seed_consensus_inputs::{SeedEpochConsensusInputs, encode_/decode_…}` (schema v6) | BLUE | One closed type (STR-010), written only by verified-bootstrap paths (SEC-014). Geometry and profile stay durable (DUR-005). |
| **Merge** | `ade_runtime::seed_consensus_merge` | GREEN | Maps into the record without re-deriving geometry (DUR-005). |

### Domain: genesis, era schedule and slot timing — confirmed (VAL-039 warning)

| Layer | Module | Color | Role |
|-------|--------|-------|------|
| **Data-only tooling** | `ade_runtime::{consensus, producer}::genesis_parser`, `ade_node::native_firstrun::parse_native_shelley_genesis` | RED | Parse genesis JSON. No float; no default network (VAL-039). |
| **Reviewed venue constants** | `ade_node::forge_timing` (banner GREEN: "transports reviewed venue constants inward; owns no conversion") | GREEN in a RED crate | Selects the committed calendar by the durable genesis hash. `ForgeTimingError` (5) is closed. |
| **Authoritative** | `ade_core::consensus::era_schedule::{EraSchedule, BootstrapBoundTimingAuthority, VenueTimingHistory}` | BLUE | The one slot conversion (DET-019). Anchored schedule (STR-009). Forecast range (VAL-033, ORD-005). Protocol-version boundaries (EVO-004). |

### Domain: native Mithril bootstrap — confirmed

| Layer | Module | Color | Role |
|-------|--------|-------|------|
| **Acquisition (data-only)** | `ade_node::mithril_fetch`, `ade_runtime::mithril_import` | RED | Downloads, verifies and lays out bytes. **`mithril_fetch` has no registry locus** (SEC-034 names no locus). |
| **Decoders** | `ade_ledger::{ledgerdb_state, ledgerdb_tables, mithril_utxo_materialize, bootstrap_manifest, cred}` | BLUE | Faithful decode (VAL-063, VAL-064). One credential discriminant (STR-005; `cred.rs` has no registry locus). Manifest binding (SEC-032). |
| **Assembly** | `ade_runtime::mithril_native_assembly::assemble_native_mithril_seed` | GREEN (pure) | Snapshot plus genesis only (SEC-030, VAL-066). |
| **Authority** | `ade_runtime::bootstrap::bootstrap_initial_state` | GREEN (banner "single bootstrap authority") | The one bootstrap function (STR-013). The WAL append is the commit point (DUR-014). |

### Domain: cardano-cli seed route and bootstrap export — confirmed; export tooling is a candidate

| Layer | Module | Color | Role |
|-------|--------|-------|------|
| **Data-only tooling** | `ade_node::bootstrap_export` (`--mode bootstrap_export`) | RED | Captures a pinned-point bundle against the closed `NetworkProfile`. Its loci name VAL-022 and PRC-004 only. It is not classified as data-only by any invariant. `_(needs human judgment)_` |
| **Importer** | `ade_runtime::seed_import` | GREEN | One deterministic importer (DET-022), fail-closed (VAL-062). |
| **Authority** | `ade_ledger::bootstrap_anchor` mint, Mithril binding predicate | BLUE / RED | One mint (STR-021). Independent-origin cross-check (SEC-029). |

### Domain: fork choice, rollback and intersection — confirmed

| Layer | Module | Color | Role |
|-------|--------|-------|------|
| **Candidate construction** | `ade_node::{candidate_aggregator, selector_state}`, `ade_runtime::consensus::{chain_selector, candidate_fragment}` | GREEN | Candidates built only from Ade-validated headers (SEC-021). |
| **Selector** | `ade_core::consensus::fork_choice::select_best_chain` | BLUE | The single selector (VAL-031, DET-012). |
| **Branch bytes (loader)** | `ade_node::fork_switch::{BranchBodySource, prevalidate_branch}` | RED | Bytes only. No rollback commits before the branch validates (SEC-022). Anchor within k (SEC-023; `lca_walk`). |
| **Commit** | `ade_ledger::rollback::{admission::admit_rollback, materialize::materialize_rolled_back_state, commit::commit_rollback}` | BLUE | One materializer (STR-011). Lockstep (REC-003). Bounded by k (SAF-007). `admit_rollback` has no registry locus. |
| **Intersection** | `ade_ledger::rollback::intersection::{select_intersection_candidates, classify_intersect_reply}` | BLUE | Points from durable ancestry only. A reply is never rollback authority (VAL-055, REC-005). |

### Domain: store semantics versioning — confirmed

| Layer | Module | Color | Role |
|-------|--------|-------|------|
| **Tooling** | `ci/store-semantics-surface.lock` + `ci/ci_check_store_semantics_lock.sh` | GREEN (CI) | Content-hashes the semantics-bearing files. A change fails CI until it is reconciled as a bump or a recorded neutral entry (PRC-005). |
| **Authoritative** | `ade_ledger::store_semantics::{STORE_SEMANTICS_VERSION, check_store_semantics_version}` | BLUE | Fails closed on an absent, older or newer marker. The only remedy is re-bootstrap; there is no stamp tool (EVO-005). |
| **Marker writers** | `chaindb::persistent`, `epoch_accumulator_store`, `reduced_utxo_checkpoint` | RED | Each stores its own marker (EVO-005). |

### Domain: mempool — chokepoint confirmed; live wiring is a candidate

| Layer | Module | Color | Role |
|-------|--------|-------|------|
| **Transport bridges** | `ade_core_interop::{tx_submission, local_tx_submission}` | RED | Build `IngressEvent`s. |
| **Authoritative** | `ade_ledger::mempool::{ingress::mempool_ingress, admit}` | BLUE | Agrees with standalone tx validity (VAL-067). Closed ingress (STR-022). Replay (DET-024). |
| **Policy** | `ade_ledger::mempool::policy` (banner GREEN, inside a BLUE crate) | GREEN | Eviction and ordering. Deterministic shedding (SEC-035). |

### Domain: serving the chain — confirmed

| Layer | Module | Color | Role |
|-------|--------|-------|------|
| **Read source** | `ade_runtime::network::served_chain_projection::ChainDbServedSource` | RED | Durable projection, bounded range (VAL-050, SEC-036). |
| **Dispatch** | `ade_runtime::network::serve_dispatch` | RED | The one dispatch core for `produce` and `node` (VAL-050, SAF-020). |
| **Authoritative reducers** | `ade_network::{chain_sync, block_fetch}::server` | BLUE | VAL-045, VAL-046, VAL-047, DET-017. |

### Domain: keys, signing and forge — confirmed

| Layer | Module | Color | Role |
|-------|--------|-------|------|
| **Key custody and signing** | `ade_runtime::producer::{keys, signing, producer_shell::ProducerShell}` | RED | The only owner of private keys (SEC-001). KES window from the bound genesis (SEC-027, SEC-005). Byte parity with the reference (VAL-006). |
| **Authoritative** | `ade_core::consensus::leader_check`, `ade_ledger::producer` (forge pure over the producer tick, self-acceptance) | BLUE | No keys cross in (SEC-026). Pure forge (DET-020). Single encoding (VAL-057). Self-acceptance (SAF-020). Shared header pre-image (SEC-007). |
| **Coordinators** | `ade_runtime::producer::{coordinator, chain_evolution, tick_assembler, …}`, `ade_node::{forge_intent, run_loop_planner}` | GREEN | DET-021, STR-018, SAF-012, SEC-025, OBS-001. |

### Domain: evidence and diagnostics — confirmed (non-authority layer)

| Layer | Module | Color | Role |
|-------|--------|-------|------|
| **Evidence reducers** | `ade_node::{admission::verdict, admission_log, convergence_evidence, post_switch_continuity, ba02_evidence, rehearsal_evidence, live_log}` | GREEN | Closed vocabularies. Emit-only, and never read by admission, selection, rollback or forge (OBS-002, OBS-005, OBS-006, OBS-008, OBS-009, DET-023, PRC-018, PRC-019). |

### Domain: Plutus evaluation — confirmed (a BLUE quarantine over a vendored evaluator, not a RED/BLUE pair)

`ade_plutus` (BLUE) is the only importer of `aiken_uplc` (git tag `v1.1.21`) and `pallas-primitives`. No other BLUE crate calls an evaluator entry point (`lib.rs` banner; `ci_check_pallas_quarantine.sh`). Results and budgets are deterministic (DET-008, DET-004), and the script context is bound to the preserved body (SEC-008, VAL-015).

---

## 3. Closed vs. Extensible Registries

### Closed (frozen; changes are version-gated only)

> Every row is findable in source at HEAD; the counts are the variant counts from the source. The change rule cites the governing IDSE invariant and its gate. **Location corrections against CODEMAP are flagged inline.**

| Registry | Location | Count | Change Rule |
|----------|----------|-------|-------------|
| `AcceptedMiniProtocol` | `ade_network::session::event` | 10 (Handshake, ChainSync, BlockFetch, TxSubmission, LocalChainSync, LocalTxSubmission, LocalStateQuery, KeepAlive, LocalTxMonitor, PeerSharing) | A frame for any other id ends the peer (VAL-048; `ci_check_mini_protocol_id_registry_closed.sh`). A new protocol needs a new variant, codec and state machine (VAL-044). |
| `N2N_SUPPORTED` / `N2C_SUPPORTED` | `ade_network::handshake::version_table` | N2N v11–v16 (6); N2C v16–v23 (8) | Offer and accept only this set (EVO-006, VAL-004; `ci_check_n2n_handshake_versiondata_authority.sh`). Adding a version is a version-gated change (§4). |
| Mini-protocol message types | `ade_network::codec::*` | one closed message type per protocol | No variant is chosen by guessing; non-matching bytes end the session (EVO-002; `ci_check_codec_message_closed.sh`). |
| `ArrayHead` | `ade_network::codec::primitives` | 2 (`Definite(u64)`, `Indefinite`) | Indefinite length is accepted only for the FindIntersect point list (VAL-005). |
| `TxSubmissionTxId` | `ade_network::codec::tx_submission` | struct `[era index, 32-byte hash]` | Never stripped or guessed (VAL-043). |
| `OutboundCommand` | `ade_runtime::network::outbound_command` | 3 (ChainSync, BlockFetch, ClosePeer) | No raw byte tunnel (SEC-017). |
| `TagEnvelopeError` | `ade_codec::cbor::tag24` | 4 | One envelope authority (VAL-002; `ci_check_tag24_wire_authority.sh`). |
| `CardanoEra`, `PrevHash` | `ade_types::era`, `ade_types::shelley::block` | 8 (ByronEbb … Conway); 2 (Genesis, Block) | Era identifiers are fixed (A2). Genesis is never a placeholder hash (VAL-003; `ci_check_prevhash_single_wire_authority.sh`). |
| `NativeScriptNode` (replaces the removed `enum NativeScript`) | `ade_types::allegra::script` | 6 (Sig, All, Any, MOfN, InvalidBefore, InvalidHereafter) | Signed 64-bit threshold (VAL-017). Heap-bounded traversal, with no recursion that grows with nesting (SEC-009). Gates: `ci_check_native_script_threshold_signed.sh`, `ci_check_native_script_depth_independent.sh`. |
| `OutputAssetQuantity(u64)` / `MintBurnQuantity(i64)` | `ade_types::mary::value` | 2 distinct domains | Full unsigned domain; checked arithmetic (VAL-013; `ci_check_value_quantity_domain.sh`). |
| `MinUtxoRule` | `ade_ledger::pparams` | 2 (LegacyAbsoluteMin, PerByte) | Era-faithful; never remapped (VAL-014). |
| Conway certificate grammar and deposit disposition | `ade_types::conway::cert`, `ade_ledger::{pparams, gov_cert, delegation}` | closed | One disposition per certificate, each effect applied once (VAL-011). Closed deposit function (CON-002; `ci_check_conway_cert_classification_closed.sh`). |
| Proposal procedures | `ade_types::conway::governance` | typed governance actions | Never opaque bytes (STR-006; `ci_check_proposal_procedures_closed.sh`). |
| `IngressSource` | `ade_ledger::mempool::ingress` | 2 (N2N, N2C) | Metadata only; the verdict never branches on it (STR-022; `ci_check_mempool_ingress_closure.sh`). |
| Script-witness closure: `ScriptPurpose`, `ReducedScriptVerdict`, `ExactScriptVerdict`, `NativeScriptVerdict`, `ScriptWitnessError`, `ReferenceScript` | `ade_ledger::tx_validity::script_witnesses` | 6 / 2 / 2 / 2 / 4 / 2 | One authority for sufficiency; the reduced level never returns "valid" (SEC-010; `ci_check_native_script_witness_verdict.sh`, `ci_check_every_supplied_witness_verifies.sh`). |
| UTxO-free admission: `UtxoFreeRejection`, `OutputPosition`, `AddressNetworkError` | `ade_ledger::tx_validity::utxo_free` | 9 / 2 / 2 | Interval, network, bootstrap-address and body-size checks (VAL-018, VAL-019; `ci_check_utxo_free_admission_obligations.sh`). |
| `CollateralBalanceError` | `ade_ledger::collateral` | 3 (UnresolvedCollateralInput, ArithmeticOverflow, ReturnExceedsCollateral) | Collateral comes from resolved inputs; an unresolved input is a refusal (CON-003; `ci_check_collateral_balance_resolver.sh`). |
| `LedgerTransitionError`, `EpochBoundaryEffect` | `ade_ledger::epoch_accumulator` | 21; 1 (FreezeLeadership) | Terminal and typed (DET-005, SAF-005). A new rejection needs a new variant plus a schema decision (§4). |
| `FrozenLeadershipPoolDistr` + `FrozenLeadershipError` | `ade_ledger::frozen_leadership` | outer `array(7)`; 8 error variants; schema **v7** | A new field means the next schema version, a `canonical_hash` change and a store-semantics decision (VAL-023, VAL-024, SAF-009; `ci_check_frozen_leadership_authority.sh`). |
| Ratification: `RatificationSource`, `RatificationOutcome`, `UnenactableReason`, `RatificationImportRefusal` | `ade_ledger::governance::frozen` | 2 / 2 / 3 / 3 | Decided once, applied atomically (VAL-021). |
| `RatificationState` | **`ade_ledger::state`** (CODEMAP lists it under `governance::frozen`) | 2 (Unversioned, Recorded) | Same rule (VAL-021). |
| `DanglingVoteDelegationKind`, `EnactableUpdateError` | `ade_ledger::governance` | 2 / 4 | VAL-030; parameter-update decode is closed. |
| Intersection: `AncestryEnd`, `IntersectionBound`, `CandidateSource`, `IntersectReply`, `IntersectionOutcome`, `IntersectionReplyFault`, `IntersectionAncestryError` | `ade_ledger::rollback::intersection` | 2 / 3 / 2 / 2 / 2 / 1 / 6 | Durable ancestry only (VAL-055, REC-005; `ci_check_durable_intersection.sh`). |
| `RollbackAdmissionError`, `RecoveryAction`, `ResetReason` | `ade_ledger::rollback::admission` (**`ResetReason` is here, not in `replay_divergence` as CODEMAP lists**) | 4 / 2 / 2 | Fails closed on an inadmissible target. **No registry locus.** The semantics overlap REC-001 and SAF-007 (§7). |
| Reduced plane: `LedgerBoundaryVerdict`, `LedgerValidityCapability`, `ReducedCertProjection`, `ReducedGovernanceProjection` | `ade_ledger::reduced_boundary` | 2 / 2 / 1 / 1 | Never widened into authority. **No registry locus** (§7). |
| Store semantics: `AuthorityArtifact`, `FoundSemanticsVersion`, `RemediationAction` | `ade_ledger::store_semantics` | 3 (ChainDb, EpochAccumulator, ReducedCheckpoint) / 2 / 1 (RebootstrapRequired) | A new durable authority artifact needs a new variant plus its marker (EVO-005; `ci_check_store_semantics_gate.sh`). |
| Bootstrap bridge and seed reward: `BridgeSourceKind`, `BridgeEta0Finality`, `SeedRewardAuthority` | `ade_ledger::{bootstrap_bridge, bootstrap_reward_update}` | 1 / 2 / 2 | One reward source at the first boundary (VAL-029; `ci_check_seed_reward_authority.sh`). Bridge nonce (VAL-022). |
| `SnapshotRewardUpdate` | **`ade_ledger::ledgerdb_state`** (CODEMAP lists it under `bootstrap_reward_update`) | 2 (Complete, NotYetComputed) | VAL-029. |
| `WalEntry` | `ade_ledger::wal::event` | 4: tag 0 AdmitBlock, tag 1 RollBack, tag 3 SeedEpochConsensusInputsImported, tag 4 EpochConsensusViewActivated; tag 2 reserved and never reused | Append-only store (DUR-006). Fingerprint chain (DUR-009). Activation record (SAF-004). Seed fact (DUR-004). Gate: `ci_check_wal_append_only.sh`. |
| Era-schedule errors: `SlotDerivationError`, `TimingAnchorError`, `TimingAuthorityError` | `ade_core::consensus::era_schedule` | 5 / 4 / 8 | One slot authority (DET-019; `ci_check_forge_slot_authority.sh`). |
| `LeaderCheckVerdict` | `ade_core::consensus::leader_check` | 2 (Eligible, NotEligible) | The ineligible verdict carries at most a bounded VRF fingerprint (SEC-026, VAL-056). |
| `LeaderEligibility` | `ade_core::consensus::header_validate` | 2 (Enforce, TrustDurable) | `TrustDurable` skips the stake threshold when replaying durable blocks. **Open question against VAL-040** (§7). |
| `Mode` | `ade_node::cli` | **7** (WireOnly, Admission, KeyGenKes, Produce, Node, BootstrapExport, MithrilSnapshotFetch) | One live owner (STR-014; `ci_check_node_mode_closure.sh` is named by STR-013). Path fidelity (`ci_check_node_path_fidelity.sh`; VAL-053, PRC-019). |
| `NetworkProfile` / `resolve_network_profile` | `ade_node::bootstrap_export` (CODEMAP does not list this module) | 2 (preview, preprod); else fail-closed | Reviewed per-network constants (magic, genesis hash, f, epoch length, k, max lovelace supply). Adding a network takes a reviewed entry and a test. Constant containment gate `ci_check_venue_constant_containment.sh` (VAL-012). |
| Venue timing calendar + `ForgeTimingError` | `ade_node::forge_timing` | 2 (preprod, preview); 5 errors | Keyed by the durable genesis hash, never by `--network` (DET-019). The registry has no second network identity. |
| `MithrilProfile` + `FetchError` | `ade_node::mithril_fetch` | 2 (preview, preprod); 6 errors | Closed network set, but the keys and aggregator are env-overridable (§1 surface 11; §7). |
| Venue roles: `VenueRole`, `VenuePolicy` | `ade_node::{node_sync, run_loop_planner}` | 3 (Unknown, SingleProducer, Participant); 2 | An unknown venue takes the refusing path (SAF-015; `ci_check_receive_detector_venue_split.sh`). |
| Receive vocabulary: `AdmissionPeerEvent`, `NodeSyncItem`, `ReceiveDisposition` | `ade_runtime::admission::wire_pump`, `ade_node::node_sync` | 4 / 2 / 4 | One pump per peer (STR-019). Peer identity is provenance only (OBS-003). One classifier (VAL-054). |
| Evidence vocabularies: `AdmissionLogEvent`, `ForkChoiceEvidenceFailure`, `AgreementVerdict`, `PostSwitchContinuity`, `NodeSchedEvent`, `ForgeSkipReason`, `ForgeOutcome`, `FeedReason`, `ForgeIntent` | `ade_node::{admission_log, admission::verdict, post_switch_continuity, live_log::sched_event, forge_intent}` | 23 / 6 / 4 / 5 / 6 / 10 / 5 / 3 / 2 | Closed, no catch-all, emit-only (OBS-002, OBS-006, OBS-007, OBS-008, OBS-009, DET-023, OBS-001, SEC-025). Gates: `ci_check_admission_log_vocabulary_closed.sh`, `ci_check_fork_choice_evidence_closed.sh`, `ci_check_forge_intent_closed.sh`, `ci_check_node_sched_events_emit_only.sh`. |
| Key and opcert grammars | `ade_runtime::producer::{keys, ade_kes_envelope, opcert_envelope}` | 3 grammars; `ADE_KES_FORMAT_VERSION = 1` | SEC-006; `ci_check_kes_envelope_closed.sh`, `ci_check_opcert_closed.sh`. |

**CODEMAP removals confirmed absent at HEAD:** `governance::{ExecUnitsParamUpdate, ExecUnitsUpdateError}`, `SlotAlignmentError`, `enum NativeScript`.

### Extensible (open within constraints)

> Doctrine §17 forbids plugin-style extension in authority paths. Ade has **no runtime-registered or plugin registry**: no `register_*` functions, no `HashMap` registries, and no `#[non_exhaustive]` types in BLUE. Every extension point is a **compile-time trait seam** with a fixed set of implementations, or a runtime collection that grows under a fixed bound. Traits used only in tests (`ade_testkit`'s `LedgerApplicator` and `ProtocolStateMachine`) are left out.

| Registry | Location | Extension Rule |
|----------|----------|---------------|
| `BranchBodySource` | `ade_node::fork_switch` (RED) | Production implementations: `NullBranchBodySource` (fail-closed fence) and `PrefetchedBranchBodies`. A new implementation supplies **bytes only** and must not bypass `prevalidate_branch` (SEC-022). |
| `LedgerView` | `ade_core::consensus::ledger_view` (BLUE trait) | Implementations: `PoolDistrView` (BLUE, read from the frozen leadership object), `LiveLedgerView` (runtime consensus inputs), `NoopLedgerView` (admission runner), and test stubs. A new view must answer leader checks only from the sealed object for exactly that epoch (VAL-024). An imported view answers only for its own epoch (VAL-035). Out-of-range queries are typed (VAL-033). |
| `ChainDb` + `SnapshotStore` | `ade_runtime::chaindb` (RED) | `InMemoryChainDb`, `PersistentChainDb`. A new backend must keep the bounded range primitives (SEC-036), atomic checkpoints (DUR-007), the semantics marker (EVO-005) and storage-independent fingerprints (DET-025). |
| `WalStore` | `ade_ledger::wal::store_trait` (BLUE trait) | `FileWalStore` and test `VecWal`s. `append` is the only mutation; adding a truncate, rewrite, replace, delete or clear method fails CI (DUR-006; `ci_check_wal_append_only.sh`). |
| `SnapshotReader` / `BlockSource` | `ade_ledger::rollback::traits` (BLUE traits) | RED caches (`InMemorySnapshotCache`, `PersistentSnapshotCache`, `ChainDbBlockSource`). They feed the one materializer only (STR-011). |
| `ChainDbWrite` | `ade_ledger::receive::chain_write` (BLUE trait) | `ChainDbWriter`. Lockstep admission and rollback (REC-003). |
| `ServedHeaderLookup` / `ServedRangeLookup` | `ade_network::{chain_sync, block_fetch}::server` (BLUE traits) | `ChainDbServedSource`, `ServedChainLookups`. Durable projection only (VAL-050), bounded (SEC-036). |
| `Transport` | `ade_network::session::handshake_driver` (GREEN trait) | `BlockingTransport` in the dialer, listener, wire pump and wire-only mode. Transport artifacts never reach the core (STR-002). Framing goes through the one codec (STR-012). |
| `CollateralValueResolver` | `ade_ledger::collateral` (BLUE trait) | `ReducedUtxoCheckpoint` plus test resolvers. Resolved values only (CON-003). Retained values (DUR-011). |
| `UtxoStore` / `UtxoMembership` | `ade_ledger::utxo` (BLUE traits) | `UTxOState`, `OverlayUtxo`, `pre_resolve::WorkingSet`, `BTreeMap`. Fingerprints are independent of representation (DET-025). Memory is bounded (SAF-025). |
| `Clock` | `ade_runtime::clock` (RED) | `SystemClock` (the only wall-clock read in `ade_runtime`) and `DeterministicClock` (replay). Only the slot crosses inward (DET-019; `ci_check_clock_seam.sh`). |
| `NodeSchedSink` | `ade_node::live_log::sched_writer` | Emit-only diagnostics, never read by the planner (OBS-002). |
| Mempool contents | `ade_ledger::mempool` (BLUE) | Transactions enter at runtime only through `mempool_ingress`, behind the bounded fold (SEC-035, VAL-067). |
| Peer set | `ade_node::cli` `--peer` (repeatable) | Peers are transport endpoints merged fairly (SEC-028). A forging node dials only configured peers (SEC-037). |
| Store-semantics lock entries | `ci/store-semantics-surface.lock` | Append-only entries: a version bump or a recorded semantics-neutral declaration (PRC-005). |

---

## 4. Version-Gated vs. Frozen Contracts

### Frozen (immutable at the current version; a change requires a new version)

- **Wire format and encoding:** Cardano CBOR through the `ade_codec` canonical read and write primitives (`minicbor` 0.25 underneath in `ade_core` and `ade_ledger`). There is one decoder per type and protocol version (VAL-001). Protocol hashes are computed over the received wire bytes, never a re-encoding (DET-002; `ci_check_hash_uses_wire_bytes.sh`). Decode then encode gives back the same bytes (DET-003; `ci_check_cbor_round_trip.sh`). Ade's own byte surfaces have one encoder each (DET-007).
- **Envelope:** tag 24 (`0xd8 0x18`) over `[era, block]` / `[era, header]`, through `ade_codec::cbor::tag24` only (VAL-002).
- **Hash and crypto algorithms:** blake2b-256 and blake2b-224, SHA3-256 for bootstrap-witness key hashes, Praos VRF IETF draft-03 with domain tags `'L'` and `'N'` (`ade_core::consensus::vrf_cert`), Ed25519, and Ade's Sum6 KES (SEC-003, SEC-004, VAL-036, VAL-006). The body hash comes from one recipe, `ade_ledger::block_body_hash` (VAL-032, VAL-057).
- **UTxO set commitment v2:** Ristretto255 multiset hash with domains `ade.utxo.fp.v2.entry` / `ade/fp/utxo/v2` (`ade_crypto::utxo_set_commitment`). Versions are never mixed in one fold (DET-026; `ci_check_utxo_fp_v2.sh`). `fingerprint` = `fingerprint_v2`, and `fingerprint_v1` is kept, frozen.
- **Credential discriminant:** `array(2)[tag, hash28]`, tag 0 = key hash, tag 1 = script hash, decoded through `ade_ledger::cred` (STR-005; `ci_check_credential_discriminant_closed.sh`).
- **Closed resource bounds** (no flag, environment variable or configuration may change them): mux `MAX_PAYLOAD = 65535` (VAL-049), session reassembly tail 16 MiB (SEC-038), wire-pump lookahead 256 blocks (SEC-038), per-peer lane 64 (SEC-028), serve range 256 blocks (SEC-036), inbound admission 1024 items / 4 MiB (SEC-035), and the in-memory UTxO cache and change-log bounds (SAF-025).
- **Reviewed network constants:** `NetworkProfile` (preview: magic 2, k 432, epoch 86 400; preprod: magic 1, k 2160, epoch 432 000; both f = 1/20) and the venue timing calendars. These are never operator-supplied (VAL-012, DET-019, DUR-005).
- **Vendored evaluator:** `aiken_uplc` pinned at git tag `v1.1.21` behind the `ade_plutus` quarantine (DET-008).
- **Dependency pins:** no manifest uses an exact `=` pin. Versions are fixed only by the tracked `Cargo.lock` and the `aiken_uplc` git tag. See §7.
- **All 637 canonical types:** existing wire formats are frozen; new types may be added.

### Version-gated (can evolve across versions)

| Contract | Current | Gate | Invariant |
|---|---|---|---|
| Authority semantics of durable stores | `STORE_SEMANTICS_VERSION = 13` (`ade_ledger::store_semantics`), surface locked in `ci/store-semantics-surface.lock` (latest entries: v13, then one semantics-neutral v13) | Any absent, older or newer marker is terminal, and the only remedy is re-bootstrap. Changing a surface file needs a bump or a recorded neutral entry. | EVO-005, PRC-005 |
| Seed-epoch consensus-input record | `SEED_CINPUT_SCHEMA_VERSION = 6` | The version is checked before the arity. An older record is a typed refusal. | STR-010, DUR-005 |
| Frozen leadership object | `FROZEN_LEADERSHIP_SCHEMA_VERSION = 7`, `OUTER_FIELDS = 7` | A new field needs the next version and a hash change. The source doc comment still describes v6 (§7). | VAL-023, SAF-009 |
| Epoch accumulator blob | `EPOCH_ACCUMULATOR_SCHEMA_VERSION = 7` | Decode refuses any other version, with no migration except re-bootstrap. | DET-009, EVO-005 |
| Bootstrap reward update | `BOOTSTRAP_RUPD_SCHEMA_VERSION = 4`, domain-separated v4 commitment | Next version for a new field. | VAL-029 |
| Bootstrap anchor / recovered anchor / bridge | `ANCHOR_SCHEMA_VERSION = 2`, `RECOVERED_ANCHOR_POINT_SCHEMA_VERSION = 1`, `BRIDGE_SCHEMA_VERSION = 1` | Next version for a new field. | STR-021, REC-007, VAL-022 |
| Bootstrap manifest | domain byte `0xB0` (v1, 142 bytes) and `0xB1` (v2, 174 bytes, adds the UTxO-seed hash) | Next domain byte. | SEC-032 |
| Consensus-profile commitment | domain `ADE-ECA0b-consensus-profile-v1` | New domain string. | SAF-009 |
| ChainDb | `SCHEMA_VERSION = 3` (layout; upgraded forward on the next write); `FINGERPRINT_VERSION = 2` (fails closed in either direction) | Layout and meaning are separate axes, and both are separate from the store-semantics version. | DUR-007, DET-026, EVO-005 |
| Snapshot framing | `SCHEMA_VERSION = 5` | Tag checked before the payload; fingerprint recomputed. | DET-011, DET-015 |
| WAL entry sum | tags 0, 1, 3, 4 (2 reserved) | A new forward step takes the next free tag. Existing variants are never widened. | DUR-006, DUR-009 |
| Negotiated protocol versions | N2N v11–v16, N2C v16–v23 | A new version needs a version-table entry, a version-data encoder and captured-byte evidence. | EVO-006, VAL-004, PRC-003 |
| Protocol version and era | the era schedule | A change takes effect only at an enacted hard-fork boundary. The forecast never extends past an unfixed one. | EVO-004, ORD-003 |
| KES envelope / rehearsal manifest | `ADE_KES_FORMAT_VERSION = 1`, `REHEARSAL_MANIFEST_SCHEMA_VERSION = 1` | Next version. | SEC-006, PRC-019 |
| CI gates | 286 `ci_check_*.sh` | Tighten only, never relax (Constitution rule 15). The close gate uses the unmasked exit status (PRC-017). | PRC-001 |
| Invariants | 260 in the registry | Redefined only by human approval (Constitution rule 2; Doctrine §5). | — |

**No mixed semantics** (Doctrine §17): one interpretation per input set (EVO-001), releases that run together agree (EVO-003, SEC-013), and no feature or `cfg` changes a result (DET-006; `ci_check_no_semantic_cfg.sh`).

---

## 5. Module Addition Rules

Derived from CODEMAP "Cross-Module Rules" and the BLUE shared header.

| Color | Naming convention | Build-config flags | May depend on | MUST NOT depend on |
|-------|-------------------|--------------------|----------------|--------------------|
| **BLUE** | `ade_*` (crate names don't encode color). Color comes from `.idse-config.json` `core_paths`: `ade_codec`, `ade_types`, `ade_crypto`, `ade_core`, `ade_plutus`, `ade_ledger`, plus 9 `ade_network` paths. | `// Core Contract:` banner (`ci_check_module_headers.sh`); `#![deny(unsafe_code, clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::float_arithmetic)]`; `clippy.toml` `disallowed-types` (HashMap, HashSet, SystemTime, Instant, anyhow::Error); no semantic `cfg` (DET-006) | Lower BLUE crates (`ade_types` ← `ade_codec`/`ade_crypto` ← `ade_core`/`ade_plutus` ← `ade_ledger`) and pure libraries (`minicbor`, `blake2`, `cardano-crypto`, `ed25519-dalek`, `sha3`, `curve25519-dalek`, `num-*`) | RED or GREEN crates, directly or transitively (STR-003); I/O, clock, randomness, async (STR-001; `ci_check_no_async_in_blue.sh`); signing (SEC-001; `ci_check_no_signing_in_blue.sh`); Mithril or multisignature crates (SEC-034); `pallas_*` outside `ade_plutus` (`ci_check_pallas_quarantine.sh`); `redb` or any storage backend; `ade_mem_diag` |
| **GREEN** | `ade_testkit`; `ade_network::session`; banner-GREEN sub-trees in RED crates (CODEMAP lists them) | Same deny attributes; per-sub-tree purity gate | BLUE | Nondeterminism; any influence on authoritative outputs; RED in non-test dependencies (`ade_testkit` breaks this today; see §7) |
| **RED** | `ade_runtime`, `ade_node`, `ade_core_interop`, `ade_mem_diag`, `ade_network::mux::transport` and the capture binaries | `//! RED` banner; `ade_node` keeps `#![deny(unsafe_code)]` with zero local allows (`ci_check_mem_diag_quarantine.sh`); the only `unsafe` lives in `ade_mem_diag` | Any module | No BLUE or GREEN crate may depend on `ade_mem_diag`; nothing may modify core state except through the BLUE transitions |

### New module checklist

1. Add the crate to the root `Cargo.toml` `[workspace] members` and commit the resulting `Cargo.lock`.
2. Decide its color **before** writing code (Doctrine §12: the human owns the BLUE boundary). For BLUE, add its path to `.idse-config.json` `core_paths`.
3. **Update the hard-coded BLUE lists.** Every color gate except `ci_check_no_async_in_blue.sh` carries its own crate list (CODEMAP). Update `ci_check_forbidden_patterns.sh`, `ci_check_dependency_boundary.sh`, `ci_check_no_float_in_consensus.sh`, `ci_check_no_semantic_cfg.sh`, `ci_check_no_signing_in_blue.sh`, `ci_check_ingress_chokepoints.sh`, `ci_check_hash_uses_wire_bytes.sh` and `ci_check_module_headers.sh`.
4. Add the banner and lints for its color.
5. New external bytes enter through a named decode chokepoint (STR-004; `ci_check_ingress_chokepoints.sh`).
6. New canonical types: closed sums, no `Default` for authoritative records, no `#[non_exhaustive]`; add round-trip tests (DET-003).
7. If the module writes or derives durable authority, add its file to `ci/store-semantics-surface.lock` and decide bump or neutral (PRC-005).
8. Name the invariants it enforces through `/define` and a cluster. A new module gets no invariant without human approval (Constitution rule 2).

---

## 6. Forbidden Patterns (per color)

Universal prohibitions (Doctrine §12, §14):

- **BLUE:** no clock, randomness, raw HashMap/HashSet, float, environment access, network or filesystem, async runtime, locale-dependent operations or OS-dependent ordering (STR-001, DET-004, STR-003).
- **GREEN:** no nondeterminism; no participation in authoritative outputs.
- **RED:** no direct mutation of BLUE state, no unsafe construction of semantic types, no bypassing canonical validation, no new semantic state variants, no reinterpretation of core outputs.

### Project-specific additions (Ade)

- **No second authority.** Each of these has exactly one: chain selector (VAL-031), rollback materializer (STR-011), bootstrap function (STR-013), live-lifecycle owner (STR-014), slot conversion (DET-019), mux codec, handshake reducer and session step (STR-012), serve dispatch (VAL-050), phase-1 and script-evaluation path (VAL-067), script-witness closure (SEC-010), phase-2 effect derivation (VAL-016), anchor mint (STR-021), seed-record writer (SEC-014), consensus-input importer (VAL-034), CLI seed importer (DET-022), snapshot cadence policy (STR-017), wire pump per peer (STR-019) and receive classifier (VAL-054).
- **Tip advances only through the one admission path, after bytes and WAL are durable** (DUR-012, DUR-010). Self-forged blocks use the same path (STR-015).
- **Leadership comes from the sealed object only** (VAL-024), sealed in the boundary commit (VAL-023).
- **Peers supply bytes, never truth.** A rollback target is a stored point, never a peer pair (SEC-020). An intersection reply is never rollback authority (VAL-055). Candidates come only from Ade-validated headers (SEC-021). A fork switch commits only after the full branch validates (SEC-022).
- **Durable state has one replay meaning.** Any semantics marker mismatch is terminal, with no stamp or override (EVO-005). Snapshot versions are checked before payloads (DET-015). The WAL is append-only (DUR-006).
- **No fabricated defaults** in authoritative records: deposits (CON-002), donation pot (CON-005), collateral (CON-003), seed reward (VAL-029), warm-start nonce (VAL-041), consensus inputs (VAL-034).
- **No feature flag, build option or operator value changes semantics** (EVO-001, DET-006). Network constants enter only through the era schedule, genesis or the reviewed registries (VAL-012).
- **Evidence never gates authority** (OBS-005, OBS-006, OBS-009). Rehearsal evidence is never promoted (PRC-019). Ade's internal fingerprint is never compared with a hash of the reference node's state (PRC-006).
- **Keys never leave the RED producer shell** or appear in logs or errors (SEC-001, SEC-006). No private keys in corpora (DET-020; `ci_check_no_private_keys_in_corpus.sh`).
- **The only `unsafe`** is the mimalloc collect in `ade_mem_diag` (`ci_check_mem_diag_quarantine.sh`).
- **Mithril verification runs only in the shell**; no BLUE crate depends on a Mithril crate (SEC-034).

---

## 7. Candidates, Cross-Reference Warnings and Open Questions

> Surfaced for confirmation. None of these is asserted as wired or settled.

### Candidates needing confirmation

1. **Transaction ingress into `--mode node`.** `mempool_ingress` has no caller in `node_lifecycle` or `ade_runtime`. The live node does not admit peer transactions. Is the mempool surface (§1 surface 3) meant to be live, or interop-only for now? (STR-022, VAL-067)
2. **N2C ingress.** The codecs and state machines exist, but there is no listener. VAL-044 says "in the roles the reference node plays". (§1 surface 4)
3. **Environment overrides of Mithril keys.** `resolve_mithril_profile` lets `ADE_MITHRIL_*` replace the committed aggregator and keys, and the preprod keys are empty. Does SEC-034's "committed Mithril keys" allow an override? (§1 surface 11)
4. **`bootstrap_export` as data-only tooling.** No invariant classifies it. Its loci cite VAL-022 and PRC-004 only.
5. **Files with no registry locus that carry seam rules:**
   - `ade_ledger::reduced_boundary` (the typed non-authority)
   - `ade_ledger::rollback::admission` (`admit_rollback`)
   - `ade_ledger::cred` (the credential discriminant chokepoint)
   - `ade_node::mithril_fetch` (the only Mithril talker)
   Their rules rest on banners plus `ci_check_reduced_boundary_plane.sh` and `ci_check_trusted_replay_boundary.sh` (both named by no registry entry).
6. **Exact dependency pins.** No manifest uses `=` pins. Wire-format anchors such as `minicbor` 0.25 and `cardano-crypto` 1.0 are fixed only by `Cargo.lock`. Is the lockfile the intended anchor?

### Cross-reference warnings (CODEMAP vs. source at `08c7c3f5`)

- CODEMAP lists `RatificationState` under `governance::frozen`. It is in `ade_ledger::state`.
- CODEMAP lists `ResetReason` under `replay_divergence`. It is in `ade_ledger::rollback::admission`.
- CODEMAP lists `SnapshotRewardUpdate` under `bootstrap_reward_update`. It is in `ade_ledger::ledgerdb_state`.
- CODEMAP omits `ade_node::bootstrap_export` (the `NetworkProfile` registry and `Mode::BootstrapExport`) from `ade_node`'s key modules. `Mode` has **7** variants, not the 6 the previous SEAMS recorded.
- `ade_node::forge_timing`'s banner says GREEN ("transports reviewed venue constants inward"). CODEMAP lists it inside the RED crate without noting the banner. Add it to the color ambiguities.
- `FROZEN_LEADERSHIP_SCHEMA_VERSION` is **7** (`OUTER_FIELDS = 7`, which adds `total_active_stake`), while its doc comment still describes v6 as current. The previous SEAMS recorded v6.
- VAL-039 requires one Shelley-genesis parser. Three exist (§1 surface 6). Only `producer::genesis_parser` is named in VAL-039's loci.

### Open questions for the user

1. **`LeaderEligibility::TrustDurable` vs. VAL-040.** Replay of durable blocks skips the stake-threshold leader check, which `ade_ledger::block_validity::transition` uses. VAL-040 says no stored verdict stands in for validation during recovery. Is the skip in scope for VAL-040, does VAL-040 need redefining, or should the check run?
2. **One genesis parser or three?** Should the three Shelley-genesis readers become one closed parser (VAL-039), or are they three grammars over different fields that VAL-039 should name?
3. **Doctrine §12 core layout.** The BLUE crates are not `#![no_std]`, use `deny(unsafe_code)` rather than `forbid`, and depend on external libraries. Doctrine §12 describes a `no_std` core crate with `forbid(unsafe_code)` and no dependencies. Is this a profile deviation to record, or migration work?
4. **Color decisions carried from CODEMAP.**
   - Promote `ade_network::session` to BLUE?
   - Demote `ade_testkit` to RED? It does file I/O and has a plain dependency on the RED `ade_runtime`, which breaks STR-003 "transitively" through dev-dependencies.
   - What color are the GREEN and RED banner files inside BLUE `ade_ledger` (`mempool::policy`, `pre_resolve`, `consensus_input_extract`)?
   - What color is `ade_node::forge_timing`?
5. **Dependency gate coverage.** `ci_check_dependency_boundary.sh` checks direct edges only, and only against `ade_runtime` and `ade_node`. Should it treat `ade_core_interop` and `ade_mem_diag` as RED and check transitive edges (STR-003)?
6. **`NetworkProfile` and the venue calendars vs. VAL-012.** Do reviewed per-network constants in RED `ade_node` count as "declared era schedule and genesis data" for VAL-012, given that `k` flows into the durable seed record?

---

## Generation notes

- Generated by `/seams` at HEAD `08c7c3f5`. Inputs:
  - `docs/ade-CODEMAP.md` (IDSE form, same HEAD)
  - `docs/ade-invariant-registry.toml` (schema 3, 260 invariants)
  - `.idse-config.json`, `CLAUDE.md` and `clippy.toml`
  - the crate manifests, `Cargo.lock` tracking and `ci/`
  - direct source introspection
- **Method.**
  - Variant counts were taken by parsing each `pub enum` body at HEAD.
  - Trait seams were taken from `dyn Trait` uses and their `impl … for` sites.
  - Constants were taken from `const …VERSION|DOMAIN|TAG` declarations.
  - Registry citations come from `enforcement_runtime`, `ci_scripts` and statement text.
  - No cluster document was read: no IDSE cluster exists, and the legacy documents under `docs/legacy/` are frozen design input.
- **Changes from the previous (IDD) SEAMS at `1e4896eb`:**
  - Every citation moved to IDSE IDs.
  - `Mode` corrected to 7 variants.
  - Frozen leadership corrected to v7.
  - New closed registries added: era-schedule timing, frozen ratification, durable intersection, store semantics, script-witness closure, UTxO-free admission, collateral, the bootstrap bridge and seed reward, `NativeScriptNode`, and the venue timing calendar.
  - The trait-seam inventory replaces the earlier "few extensible registries" list.
  - New surfaces recorded: durable-store rehydration, the wall clock, the process environment and N2C.
- Nothing in the codebase was modified. Only this file was written.
