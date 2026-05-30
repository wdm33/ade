# Seams — Where New Work Can Attach (Ade)

> **Status:** Living architectural document. Regenerated; not hand-edited.
> Per-project instance of `~/.claude/methodology/templates/seams.md`.

> 11 crates, **456 canonical types**, **105 CI checks** at HEAD (`a3a0636`, PHASE4-N-F-A cluster close).
> Reads CODEMAP (`docs/ade-CODEMAP.md`, regenerated at the same HEAD) for the module
> list + TCB colors, and the invariant registry (`docs/ade-invariant-registry.toml` —
> **303 entries**) for the rule IDs that gate each closed surface.
>
> **This regeneration layers the PHASE4-N-F-A delta over the N-Z seams map.** PHASE4-N-F-A is the
> **recovered seed-epoch consensus-input CAPABILITY** cluster (A1–A4). It establishes the
> recovered seed-epoch consensus inputs (PoolDistr, ASC, per-pool VRF keyhash, total active stake
> for the single seed `epoch_no`) as a first-class Ade-owned recovered surface: a closed canonical
> record with a SOLE codec, persisted as a fingerprint-bound sidecar and reconstructable through
> verified warm-start. **It is a capability cluster, NOT production wiring** — the authority surface
> is proven end-to-end, but no production mode consumes the recovery path.
>
> **Capability-not-wiring honesty (load-bearing for SEAMS).** The three new attach points below
> are the seams the SUCCESSOR cluster **PHASE4-N-F-C** will use to wire production consumption; at
> THIS HEAD they are populate-side / extension-point surfaces only:
> - **`produce_mode` does NOT consume the recovered surface.** It still cold-starts from
>   `--consensus-inputs-path`, projects the operator bundle via the private
>   `pool_distr_view_from_consensus_inputs`, and passes `SeedEpochConsensusSource::NotRequired`.
>   Producer consumption (CE-A-4b) is deferred to PHASE4-N-F-C.
> - **The A4 projection is a capability.** `PoolDistrView::from_seed_epoch_consensus_inputs` is a
>   pure BLUE field-map proving the recovered surface is a drop-in leadership source — the producer
>   is **not** wired to consume it.
> - **The warm-start restore is proven at the authority surface only.**
>   `bootstrap_initial_state(RequiredFromRecoveredProvenance)` restores + verifies the sidecar
>   fail-closed, exercised directly; `node.rs run_node_until_shutdown` is NOT wired to the recovery
>   path and `recover_node_state` is the test-only secondary entry. **BA-02 (Haskell peer accepts an
>   Ade-forged block) is satisfied NOWHERE at this HEAD.**
>
> N-F-A introduced / extended:
>
> - **BLUE** `ade_ledger::seed_consensus_inputs` (NEW, A1) — the closed `SeedEpochConsensusInputs`
>   recovered-state record (`anchor_fp` / `epoch_no` / `active_slots_coeff` / `total_active_stake` /
>   `pool_distribution: BTreeMap<Hash28, PoolEntry>`) + its **SOLE** version-gated, byte-canonical
>   codec (`encode_/decode_seed_epoch_consensus_inputs`, `SEED_CINPUT_SCHEMA_VERSION = 1`) + the
>   closed 6-variant `SeedConsensusInputsError`. Deterministic CBOR, `BTreeMap`-ordered, no
>   `Default` / `#[non_exhaustive]`; `decode_*` fail-closes on unknown version, wrong shape, short
>   hash, non-canonical / duplicate pool-map keys, trailing bytes, and any non-byte-canonical
>   encoding (re-encode != input). **CN-CINPUT-01** (enforced).
> - **BLUE** `ade_ledger::wal` (EXTENDED, A3a) — the **additive** `WalEntry::SeedEpochConsensusInputsImported`
>   variant at append-only **wire tag 3** (tags `1`/`2` stay reserved; `AdmitBlock` = tag 0) that does
>   **not** participate in the `AdmitBlock` `prior_fp`/`post_fp` chain; `replay_from_anchor` now folds
>   the WAL into a `ReplayOutcome` carrying `tail_fp` + `admit_count` + an
>   `Option<RecoveredBootstrapProvenance>` (at most one provenance entry per store/anchor; duplicate →
>   `DuplicateProvenance`, anchor mismatch → `ProvenanceAnchorMismatch`, both fail closed). **DC-CINPUT-01**
>   (**partial** — see below). **`WalEntry` stays a CE-not-law surface** (additively evolvable behind the
>   WAL schema; tags append-only — NOT a frozen registry law).
> - **BLUE** `ade_ledger::consensus_view` (EXTENDED, A4) — `PoolDistrView::from_seed_epoch_consensus_inputs`
>   (pure field-map; the recovered record already zips stake + VRF keyhash into the single `PoolEntry`
>   map, so no second map and no zero-hash fallback). **DC-CINPUT-02a** (enforced — **projection only**,
>   NOT consumed by the producer).
> - **GREEN** `ade_runtime::seed_consensus_merge` (NEW, A2) — the deterministic, no-I/O merge lifting a
>   verified-bootstrap `LiveConsensusInputsCanonical` (two-map shape: `pool_distribution` stake +
>   separate `pool_vrf_keyhashes`) into the BLUE single-map `SeedEpochConsensusInputs`; fail-closed
>   (closed 2-variant `SeedConsensusMergeError::{PoolMissingVrfKeyhash, PoolMissingStake}`) on any pool
>   present in exactly one source map — never a zero-hash fill.
> - **RED** `ade_runtime::seed_consensus_provenance` (NEW, A3a) — the single shared
>   `append_seed_epoch_provenance` helper: `blake2b_256` of the **exact** A1 sidecar bytes →
>   `WalEntry::SeedEpochConsensusInputsImported` append. RED because it touches `WalStore`; allowed only
>   at the two verified-bootstrap composition sites.
> - **RED** `ade_runtime::bootstrap` (EXTENDED, A3b) — the closed 2-variant `SeedEpochConsensusSource`
>   enum (`NotRequired` / `RequiredFromRecoveredProvenance`), the named `BootstrapState` output struct
>   (replaces the widened triple; carries `Option<SeedEpochConsensusInputs>`),
>   `restore_seed_epoch_consensus_inputs` warm-start verification, and **5 new fail-closed**
>   `BootstrapError` variants (`SeedConsensusProvenanceMissing` / `SeedConsensusSidecarMissing` /
>   `SeedConsensusHashMismatch` / `SeedConsensusBindingMismatch` / `SeedConsensusSidecarDecode`).
>   **Every current caller passes `NotRequired`.**
> - **RED** `ade_runtime::{genesis_bootstrap, mithril_bootstrap}` (EXTENDED, A2/A3a) — both verified-
>   bootstrap composers gained a `&mut dyn WalStore` parameter and a `persist_seed_epoch_consensus_inputs`
>   tail: GREEN merge → A1 encode → **sidecar put (durable) THEN WAL provenance append (the commit point)**.
>   A crash between the two leaves the sidecar present but unrecorded — replay yields no provenance, so
>   warm-start treats the import as "not imported" and fails closed.
> - **RED** `ade_runtime::chaindb::{mod, persistent, in_memory, snapshot_contract}` (EXTENDED, A2) — the
>   `SnapshotStore` trait gained `put_/get_seed_epoch_consensus_inputs(anchor_fp, …)`: an
>   **anchor-fp-keyed** sidecar surface **disjoint** from the slot-keyed snapshot namespace (never a
>   sentinel slot; idempotent on identical bytes, `InvalidOperation` on conflicting bytes for the same
>   `anchor_fp`). The redb backend adds the `seed_cinputs_by_anchor_fp` table and bumps `SCHEMA_VERSION`
>   v2 → **v3** (fail-closed on a newer on-disk schema). **NOT a frozen contract** — a versioned gate.
> - **NEW CI gate** `ci_check_consensus_input_provenance.sh` (**CN-CINPUT-02**, enforced) — a
>   data-flow-resistant **containment** gate (global call-site scan, not a bypassable RHS grep): the
>   sidecar may be populated only on the verified-bootstrap composition path; the forge-time path
>   (`produce_mode` / `import_live_consensus_inputs` / `pool_distr_view_from_consensus_inputs` /
>   `--consensus-inputs-path`) MUST NOT build / put the sidecar nor append its WAL provenance. **This is
>   the SEAM-closure mechanism** that keeps the forge-time path from attaching to the sidecar surface.
>   It constrains **POPULATION + the forge-time fence only**; it does **not** assert the producer
>   consumes the recovered surface — the **CONSUME-side fence + producer consumption are owed to
>   PHASE4-N-F-C**.
>
> **Registry delta:** **303 entries** (299 → 303). NEW **CN-CINPUT-01** (enforced — SOLE
> version-gated codec), **CN-CINPUT-02** (enforced — populate-side containment + forge-time fence),
> **DC-CINPUT-01** (**partial** — warm-start verification proven at the authority surface; the
> PRODUCTION restart path is the open obligation, deferred to PHASE4-N-F-C / C3), **DC-CINPUT-02a**
> (enforced — projection only). Carry-forward `evidence_notes` strengthenings cite N-F-A on
> `CN-ANCHOR-01`, `DC-ANCHOR-01`, `T-REC-01`, `T-REC-02` (their `strengthened_in` arrays are
> unchanged).
>
> **Governance note (N-F-A).** The single **`bootstrap_initial_state` authority** is upheld — the
> genesis / Mithril composers route through it and write the sidecar *after* bootstrap (they never
> consume one), and the warm-start restore lives inside the same authority (CN-NODE-01). The recovered
> sidecar is a closed canonical type with a SOLE codec (CN-CINPUT-01) — the forge-time operator bundle
> can never be relabeled as recovered state (CN-CINPUT-02 containment). The producer still cold-starts
> from the operator bundle; the recovered surface is a proven *capability*, not a wired production
> input.
>
> ---
>
> **All PHASE4-N-Z + N-Y closed surfaces are unchanged** and stand below as the bulk of the seams
> map. PHASE4-N-Z is the single-slice Mithril *production-bootstrap wiring* + seed-point independence
> cluster (closed RO-MITHRIL-IMPORT-01 item (b)); it introduced **no BLUE change, no new semantic
> authority, and no new CLI surface**:
>
> - **RED composition** `ade_runtime::mithril_bootstrap::bootstrap_from_mithril_snapshot`
>   — a **composition-only** entry that fronts the single closed
>   `bootstrap::bootstrap_initial_state` authority for the Mithril path, **symmetric with**
>   `genesis_bootstrap::bootstrap_from_conway_genesis`. It composes (in order) the RED manifest
>   import, the RED anchor `mint`, the BLUE `verify_mithril_binding` cross-check, and the single
>   bootstrap authority — fail-closing on the binding **before** any `bootstrap_initial_state`
>   call. (N-F-A extended its error surface with the three `SeedConsensus*` variants and threaded the
>   new `&mut dyn WalStore` + sidecar tail.) **No new authority, no `*Anchor` trait/plugin seam, no new
>   `SeedProvenance` variant.**
> - **RED closed types** (all in `ade_runtime::mithril_bootstrap`, RED — NOT counted in the 456 BLUE
>   total): `MithrilBootstrapError` (3-variant: `Import` / `Binding` / `Bootstrap`),
>   `MithrilSeedPointInputs` (operator-provided, structurally-independent seed-point extraction),
>   `MithrilBootstrapOutput`.
> - **CI gate** `ci_check_mithril_seed_point_independence.sh` (DC-MITHRIL-02) — a data-flow-resistant
>   containment coherence guard, **not a code seam**.
>
> **Four structural decisions remain load-bearing for SEAMS:** (1) the **single
> `bootstrap_initial_state` authority** fronts produce-mode cold-start, the Conway-genesis path, the
> Mithril provenance path (N-Y), the Mithril production-bootstrap composition (N-Z), AND now the N-F-A
> warm-start restore — all route through the one authority; **no `GenesisAnchor` / `MithrilAnchor`
> trait or plugin seam exists** (explicitly rejected). (2) The **two-driver split** (GREEN reducer /
> RED pump). (3) **`WalEntry` stays a CE-not-law** surface (now carrying the additive N-F-A tag-3
> variant). (4) The **redb `chaindb` `SCHEMA_VERSION` is a versioned gate** (now v3), not a frozen
> contract.
>
> **Cluster-doc location.** The PHASE4-N-F-A cluster + A1–A4 slice docs live under
> `docs/clusters/PHASE4-N-F-A/`; on close they archive under `docs/clusters/completed/`. Every prior
> closed cluster doc — the entire **N-Q / N-R-\* / N-S-\*** set, the **N-M-\*** sub-trees, **N-O**,
> **N-P**, **N-T**, **N-V**, **N-W**, **N-X**, **N-Y**, **N-Z** — is archived under
> `docs/clusters/completed/`.

---

## 1. Surface Reduction Rules

> External inputs reduce to canonical form before entering authoritative pipelines. Ade
> is a Cardano node, not a request/response service — its "external surfaces" are the
> N2N/N2C wire, operator-supplied key/genesis/opcert files, the cardano-cli UTxO seed
> dump, the Mithril snapshot manifest (N-Y), the Mithril production-bootstrap composition
> (N-Z), the Conway genesis file (N-Y), and argv. Each reduces to a canonical BLUE type
> before any authoritative transition. There is **no HTTP/gRPC/message-bus ingress**
> (confirmed absent — not a gap).
>
> **N-F-A note:** the recovered seed-epoch consensus-input sidecar is **not an external ingress
> surface** — it is an Ade-internal recovered surface populated only on the verified-bootstrap
> path and read back only by the warm-start restore inside `bootstrap_initial_state`. The
> warm-start restore (§ "seed-epoch sidecar warm-start", below) is the read seam PHASE4-N-F-C will
> wire into a production restart path; at THIS HEAD it is exercised at the authority surface only.

### Surface: N2N inbound wire (received blocks/headers/txs)

```
Surface: N2N mini-protocol traffic over TCP+mux (RED ade_runtime::network::{n2n_listener, mux_pump, n2n_dialer})
Reduces to: decoded mini-protocol messages → tag-24-stripped inner bytes → PreservedCbor<T> → DecodedBlock (BLUE ade_codec)
Pipeline (fixed; steps may not be reordered or shortcut):
  1. mux::frame::decode_frame                       (BLUE — single frame-decode authority)
  2. session::core::step                            (GREEN — partial-frame buffer + payload reassembly + closed AcceptedMiniProtocol registry)
  3. per-mini-protocol *_transition reducer         (BLUE — chain_sync / block_fetch / etc.)
  3a. tag-24 strip (N-X)                             (BLUE — decompose_blockfetch_block / decompose_rollforward_header delegate to ade_codec::unwrap_tag24; RED admission::runner / follow call ade_codec::unwrap_tag24 directly — no hand-rolled parse)
  4. ade_codec decode_block_envelope / decode_*     (BLUE — sole PreservedCbor construction site, over the verbatim tag-24-stripped inner bytes)
  5. ade_ledger::receive::reducer / mempool_ingress (BLUE — header→body bridge / wire-ingress chokepoint)
  6. forward_sync::reducer → forward_sync::pump (N-Y)  (GREEN admit-plan over the BLUE admit chokepoint → RED durability-ordered driver; AdvanceTip only after StoreBlockBytes + AppendWal ack)
  7. block_validity / tx_validity / admission        (BLUE verdict; GREEN admission compares already-authoritative outputs)
Cross-surface state sharing: the served ServedChainSnapshot (read by both serve and broadcast paths);
  the per-peer outbound map (PerPeerOutbound) is keyed by PeerId — no cross-peer byte leakage.
  The tag-24 unwrap step (N-X) is the SAME shared ade_codec authority used by the serve path's wrap step.
  The forward-sync persisted ChainDb + FileWalStore are the same stores the recovery path (recovery::restart)
  reconciles on warm-start (DC-WAL-*; WalTailFingerprintMismatch fail-fast). The WAL is also where the N-F-A
  additive SeedEpochConsensusInputsImported (tag 3) provenance lives — disjoint from the AdmitBlock fp-chain.
```

### Surface: producer-mode forge → serve → broadcast (the live producer half)

```
Surface: --mode produce slot loop (RED ade_node::produce_mode + GREEN producer::coordinator)
Reduces to: ForgedBlock → AcceptedBlock (BLUE self_accept) → ServedChainSnapshot → tag-24-wrapped wire bytes
Pipeline (fixed; the BLUE-then-RED-then-BLUE composition of run_real_forge):
  1. bootstrap_initial_state                        (RED/GREEN — sole forge-state source; N-T; fronts genesis/Mithril cold-start, N-Y/N-Z; produce_mode passes SeedEpochConsensusSource::NotRequired — N-F-A)
  1a. era guard (N-W)                                (RED — non-Praos era fail-closes to ForgeFailureReason::UnsupportedProducerEra before any forge)
  2. RED vrf_prove over expected_vrf_input.alpha_bytes()  (operator VRF key; alpha comes from the BLUE LeaderScheduleAnswer — no RED-side era dispatch; N-W)
  3. BLUE verify_and_evaluate_leader(era, …) → LeaderCheckVerdict  (ade_core::consensus::leader_check; era-correct Praos construction; N-R-A + N-W)
  4. RED kes_sign_header(UnsignedHeaderPreImage)    (signs ONLY the branded pre-image; N-S-A)
  5. GREEN assemble_tick
  6. BLUE forge_block → encode_block_envelope       (single canonical block encoder, storage-form [era, block]; N-V)
  7. BLUE self_accept                               (gate — no ForgeSucceeded without Accepted)
  8. ChainEvolution::advance(self)                  (GREEN linear typestate; token only via self_accept; N-T)
  9. ServedChainHandle::push_atomic                 (single served-admit authority; N-R-B/N-T)
 10. BLUE serve composition (N-X)                   (block_fetch::server emits compose_blockfetch_block(storage [era, block]) = tag24(bytes([era, block]));
                                                     chain_sync::server emits compose_rollforward_header(era, header_cbor) = [era_tag, tag24(bytes(header_cbor))])
 11. OutboundCommand → MuxPump                      (typed relay; no byte tunnel; N-S-B)
Cross-surface state sharing: ChainEvolution threads each forge's post-state into the next
  forge's base; ServedChainSnapshot is shared with the N2N serve path; the per-peer outbound
  map is shared with the listener. The serve step's tag-24 wrap is the SAME ade_codec authority
  the receive path uses to unwrap (CN-WIRE-08).
N-F-A fence (populate-side enforced now; CONSUME-side owed to N-F-C): produce_mode is the forge-time
  consensus-input path (import_live_consensus_inputs + pool_distr_view_from_consensus_inputs +
  --consensus-inputs-path). At THIS HEAD it MUST pass SeedEpochConsensusSource::NotRequired and MUST NOT
  build / put the seed-epoch sidecar nor append its WAL provenance (CN-CINPUT-02). It does NOT consume the
  recovered SeedEpochConsensusInputs surface (cannot even name the recovered type); producer consumption
  (CE-A-4b) is deferred to PHASE4-N-F-C.
```

### Surface: seed-epoch sidecar warm-start (recovered consensus inputs — capability, N-F-A)

```
Surface: warm-start restore of the recovered seed-epoch consensus inputs (RED ade_runtime::bootstrap::restore_seed_epoch_consensus_inputs, inside the bootstrap_initial_state authority)
Reduces to: anchor-fp-keyed sidecar bytes (SnapshotStore::get_seed_epoch_consensus_inputs) → verified SeedEpochConsensusInputs → BootstrapState.seed_epoch_consensus_inputs: Option<SeedEpochConsensusInputs>
Pipeline (fixed; the RED-read / BLUE-verify chain; fail-closed on every step):
  1. SeedEpochConsensusSource discriminant           (RequiredFromRecoveredProvenance(provenance) ⇒ restore; NotRequired ⇒ None — the only two modes; every current caller passes NotRequired)
  2. RED get_seed_epoch_consensus_inputs(anchor_fp)  (the only RED step — reads the anchor-fp-keyed sidecar bytes; absent ⇒ BootstrapError::SeedConsensusSidecarMissing)
  3. BLUE blake2b_256 bind                            (re-hash the read bytes; != provenance.sidecar_hash ⇒ SeedConsensusHashMismatch)
  4. BLUE decode_seed_epoch_consensus_inputs          (the A1 SOLE decoder; version-gated, byte-canonical; failure ⇒ SeedConsensusSidecarDecode)
  5. BLUE anchor/epoch binding                        (decoded anchor_fp/epoch_no != provenance ⇒ SeedConsensusBindingMismatch)
  6. BLUE byte-identity re-encode                     (re-encode != input ⇒ SeedConsensusHashMismatch)
Cross-surface state sharing: the same SnapshotStore + WAL the forward-sync pump and recovery::restart use.
  The provenance comes from wal::replay_from_anchor folding the additive SeedEpochConsensusInputsImported (tag 3)
  entry into ReplayOutcome.recovered_provenance — disjoint from the AdmitBlock fp-chain.
NOT-YET-WIRED (N-F-C / C3): this restore is exercised at the authority surface only. node.rs
  run_node_until_shutdown is NOT wired to it and recover_node_state is the test-only secondary entry.
  The CONSUME-side fence (keeping a non-bootstrap path from passing RequiredFromRecoveredProvenance) is
  owed to PHASE4-N-F-C; CN-CINPUT-02's gate currently fences the POPULATE side only.
```

### Surface: operator file ingress (KES skey / opcert / Shelley genesis / UTxO seed dump)

```
Surface: operator-supplied files (RED ade_runtime::producer::{keys, opcert_envelope, genesis_parser}, seed_import)
Reduces to: Sum6Kes signing key (via BLUE deserializer) / OperationalCert / GenesisAnchor / canonical seed entries
Pipeline:
  1. RED parse text/JSON/CBOR envelope               (closed parser per file type; structured fail-closed error)
  2. BLUE structural validator                       (e.g. Sum6Kes::raw_deserialize_signing_key_kes — byte layout is the validator)
  3. canonical type handed to the BLUE core          (never raw bytes)
Cross-surface state sharing: GenesisAnchor + opcert public metadata feed the producer coordinator;
  KES/VRF/cold private material is RED-confined and never enters GREEN CoordinatorState.
```

### Surface: Mithril snapshot manifest — provenance binding (N-Y)

```
Surface: Mithril snapshot manifest JSON (RED ade_runtime::mithril_import::json::parse_mithril_manifest_json)
Reduces to: RawMithrilManifest → SeedProvenance::Mithril{..} + MithrilManifestReport → (BLUE) verify_mithril_binding verdict
Pipeline (fixed; the RED-then-BLUE provenance binding):
  1. RED parse_mithril_manifest_json                 (SOLE manifest-JSON parser → RawMithrilManifest; fail-closed MithrilManifestError; NO semantic decision)
  2. RED import_mithril_manifest                     (maps into the closed SeedProvenance::Mithril + MithrilManifestReport; NEVER re-verifies the STM multisig)
  3. BLUE verify_mithril_binding(report, anchor)     (the SOLE authority deciding whether a Mithril anchor binds; cross-checks {network_magic, genesis_hash, certified_point, certificate_hash}; fails closed with MithrilImportError)
Cross-surface state sharing: the report side (manifest) and the anchor side MUST originate
  independently — verify_mithril_binding is NOT a tautological self-check (CN-MITHRIL-01).
```

### Surface: Mithril production-bootstrap composition (N-Z, extended N-F-A sidecar tail)

```
Surface: bootstrap_from_mithril_snapshot (RED ade_runtime::mithril_bootstrap; composition-only — NO CLI flag, a future operator-UX slice)
Reduces to: (MithrilSeedPointInputs, seed (LedgerState, PraosChainDepState), manifest_bytes) → MithrilBootstrapOutput { ledger, chain_dep, tip, anchor } (+ N-F-A sidecar put + WAL provenance append)
Pipeline (fixed; the call-order is CI-pinned by ci_check_mithril_seed_point_independence.sh):
  1. RED import_mithril_manifest_from_bytes(manifest_bytes)  (→ MithrilProvenanceImport { provenance, report }; fail-closed MithrilBootstrapError::Import; NO semantic decision)
  2. RED mint(MintInputs{ seed_slot/seed_block_hash/network_magic/genesis_hash/… from MithrilSeedPointInputs (operator-INDEPENDENT origin); seed_provenance = import.provenance })  (→ BootstrapAnchor; seed_point comes ONLY from the operator inputs, NEVER the manifest — DC-MITHRIL-02)
  3. BLUE verify_mithril_binding(&import.report, &anchor)    (the SOLE binding authority; fail-closed MithrilBootstrapError::Binding BEFORE any storage init; CN-MITHRIL-01 strengthened — verify-before-bootstrap)
  4. RED bootstrap_initial_state(BootstrapInputs{ …, genesis_initial: Some((seed_ledger, seed_chain_dep)) , seed_epoch_consensus_source: NotRequired })  (the single closed bootstrap authority; never a parallel storage-init path; CN-NODE-01)
  5. RED sidecar tail (N-F-A; success path only, after binding passes)  (GREEN merge_seed_epoch_consensus_inputs → A1 encode → put_seed_epoch_consensus_inputs(anchor_fp, …) DURABLE → append_seed_epoch_provenance (WAL tag 3) COMMIT POINT; the composer WRITES the sidecar, never consumes one; CN-CINPUT-02)
Cross-surface state sharing: shares the single bootstrap authority with produce_mode + the
  Conway-genesis path. The operator's MithrilSeedPointInputs origin and the manifest origin
  MUST stay structurally independent (DC-MITHRIL-02). The N-F-A sidecar tail puts to the
  anchor-fp-keyed SnapshotStore namespace (disjoint from the slot-keyed snapshot space) THEN
  appends WAL provenance — a crash between leaves an unrecorded sidecar, which warm-start treats
  as "not imported" and fails closed.
```

### Surface: Conway genesis cold-start (N-Y, extended N-F-A sidecar tail)

```
Surface: Conway genesis config (RED ade_runtime::genesis_bootstrap::bootstrap_from_conway_genesis)
Reduces to: ConwayGenesisConfig → (LedgerState, PraosChainDepState) → BootstrapInputs.genesis_initial (+ N-F-A sidecar put + WAL provenance append)
Pipeline (fixed; the RED-read / BLUE-transform / single-authority composition):
  1. RED genesis_parser file read/parse              (shelley/Conway genesis JSON → ConwayGenesisConfig)
  2. BLUE genesis_initial_state(&ConwayGenesisConfig) (pure Conway-only transform; fail-closed GenesisSourceError::NonConwayEra)
  3. RED route through bootstrap_initial_state       (genesis pair enters ONLY via BootstrapInputs.genesis_initial; records SeedProvenance::CardanoCliJson; seed_epoch_consensus_source: NotRequired; never a second storage-init authority)
  4. RED sidecar tail (N-F-A)                         (GREEN merge → A1 encode → put_seed_epoch_consensus_inputs DURABLE → append_seed_epoch_provenance (WAL tag 3) COMMIT POINT; WRITES the sidecar, never consumes one; CN-CINPUT-02)
Cross-surface state sharing: shares the single bootstrap authority with produce_mode + the
  N-Z Mithril production-bootstrap composition. bootstrap_from_mithril_snapshot is the
  symmetric Mithril-path twin of this entry; both gained the same &mut dyn WalStore + sidecar tail.
```

### Surface: argv (closed mode set)

```
Surface: command line (RED ade_node::cli — Cli / ProduceCli)
Reduces to: a closed mode enum {produce, admission, wire-only, key-gen-KES} (ci_check_node_mode_closure.sh)
Pipeline: argv → Cli → mode driver. --mode produce requires --json-seed + --consensus-inputs.
Cross-surface state sharing: none.
Note: the N-Z bootstrap_from_mithril_snapshot composition has NO argv surface — it is a
  library composition entry; an operator-facing CLI flag is a deliberately-deferred future slice.
  N-F-A added NO argv surface either: no --mode and no flag consumes the recovered seed-epoch
  surface; a production restart/recovery mode is deferred to PHASE4-N-F-C.
```

**Rule:** New ingress attaches by producing the canonical BLUE type's bytes and entering
the **same** pipeline. A new mini-protocol attaches through `session::core::step` + a BLUE
`*_transition` reducer + a closed `AcceptedMiniProtocol` registry entry. A new operator
file type attaches as a RED parser feeding a BLUE structural validator. **A new bootstrap
seed source (like Mithril or genesis) attaches by populating `BootstrapInputs.genesis_initial`
and routing through the single `bootstrap_initial_state` authority — NEVER via a new
`*Anchor` trait / plugin seam, and never via a parallel storage-init path** (CN-MITHRIL-01 /
CN-NODE-01 / DC-GENESIS-SRC-01 / DC-MITHRIL-02). **A new bootstrap-source production composition
(like N-Z `bootstrap_from_mithril_snapshot`) attaches as a composition-only RED twin of
`bootstrap_from_conway_genesis` — verify-before-bootstrap, fail-closed, no new authority, no
new `SeedProvenance` variant, and (if the source attests a point) the anchor `seed_point` MUST
come from an operator-independent origin that `verify_mithril_binding` cross-checks against the
attestation (DC-MITHRIL-02).** **A recovered-state surface (like the N-F-A seed-epoch consensus
inputs) is populated ONLY on the verified-bootstrap composition path (put-then-WAL-append) and is
read back ONLY by the warm-start restore inside `bootstrap_initial_state` — the producer / forge-time
path may not populate it (CN-CINPUT-02); the CONSUME-side wiring (a production restart mode passing
`SeedEpochConsensusSource::RequiredFromRecoveredProvenance`) is the seam PHASE4-N-F-C will attach.**
New ingress **may not** introduce a second `PreservedCbor` construction site, a second
block-envelope encoder, a second era→leader-VRF-input construction (CN-FORGE-04), a second
`wrap_tag24` / `unwrap_tag24` definition or a hand-rolled tag-24 parse in RED (CN-WIRE-08), a
direct-transport write that bypasses `OutboundCommand`, a forward-sync path that advances the tip
before the durability writes ack (DC-SYNC-01), a second bootstrap/storage-init authority
(CN-NODE-01 / DC-GENESIS-SRC-01), a Mithril manifest parser other than
`parse_mithril_manifest_json` (CN-MITHRIL-01), a Mithril-bootstrap composition that drills into
the manifest import to source the anchor `seed_point` (DC-MITHRIL-02 —
`ci_check_mithril_seed_point_independence.sh`), **a second `SeedEpochConsensusInputs` codec
(CN-CINPUT-01), or a forge-time path that populates the seed-epoch sidecar / appends its WAL
provenance (CN-CINPUT-02 — `ci_check_consensus_input_provenance.sh`).**

---

## 2. Data-Only vs. Authoritative Layers

### Domain: recovered seed-epoch consensus inputs (NEW, N-F-A)

| Layer | Module | Color | Role |
|-------|--------|-------|------|
| **Authoritative canonical record + SOLE codec** | `ade_ledger::seed_consensus_inputs` (`SeedEpochConsensusInputs`, `encode_/decode_seed_epoch_consensus_inputs`, `SEED_CINPUT_SCHEMA_VERSION = 1`) | BLUE | The closed recovered-state record + its SOLE version-gated, byte-canonical encoder/decoder pair. `decode_*` fail-closes on unknown version, wrong shape, short hash, non-canonical / duplicate pool-map keys, trailing bytes, and any non-byte-canonical encoding (closed 6-variant `SeedConsensusInputsError`). No second codec. (CN-CINPUT-01.) |
| **Data-only merge glue** | `ade_runtime::seed_consensus_merge::merge_seed_epoch_consensus_inputs` | GREEN | Lifts a verified-bootstrap two-map `LiveConsensusInputsCanonical` into the BLUE single-map record; fail-closed (closed 2-variant `SeedConsensusMergeError::{PoolMissingVrfKeyhash, PoolMissingStake}`) on any pool in exactly one source map — never a zero-hash fill. Produces the BLUE record but is NOT its author; decides nothing semantic. |
| **Data-only WAL provenance appender** | `ade_runtime::seed_consensus_provenance::append_seed_epoch_provenance` | RED | `blake2b_256` of the EXACT A1 sidecar bytes → `WalEntry::SeedEpochConsensusInputsImported` (tag 3) append. RED (touches `WalStore`); allowed only at the two verified-bootstrap composition sites; called only AFTER the durable sidecar put. |
| **Data-only sidecar store** | `ade_runtime::chaindb::SnapshotStore::{put,get}_seed_epoch_consensus_inputs(anchor_fp, …)` | RED | Anchor-fp-keyed sidecar namespace **disjoint** from the slot-keyed snapshot space; idempotent on identical bytes, `InvalidOperation` on conflicting bytes for the same `anchor_fp`; redb backend = `seed_cinputs_by_anchor_fp` table, `SCHEMA_VERSION = 3` (fail-closed on a newer on-disk schema). No semantic decision. |
| **Authoritative warm-start restore** | `ade_runtime::bootstrap::restore_seed_epoch_consensus_inputs` (inside `bootstrap_initial_state`) | RED read + BLUE verify | The `get_seed_epoch_consensus_inputs` read is the only RED step; the bind → decode → anchor/epoch binding → byte-identity verification is BLUE over already-read bytes, fail-closed via the 5 new `BootstrapError::SeedConsensus*` variants. **MUST NOT** fall back to the forge-time `--consensus-inputs-path` bundle on any failure. (DC-CINPUT-01.) |
| **Authoritative replay fold** | `ade_ledger::wal::replay_from_anchor` (`ReplayOutcome`, `RecoveredBootstrapProvenance`) | BLUE | Folds the additive `SeedEpochConsensusInputsImported` (tag 3) into `ReplayOutcome.recovered_provenance` (at most one per store/anchor; `DuplicateProvenance` / `ProvenanceAnchorMismatch` fail closed) **without** disturbing the `AdmitBlock` `prior_fp`/`post_fp` chain. Pure. |
| **Authoritative projection (capability)** | `ade_ledger::consensus_view::PoolDistrView::from_seed_epoch_consensus_inputs` | BLUE | Pure field-map projecting the recovered record into the leadership `PoolDistrView` (off-epoch queries return `None`; no zero-hash fallback). **A projection the producer is NOT yet wired to consume** (DC-CINPUT-02a — CE-A-4b owed to N-F-C). |

**Rule (CN-CINPUT-01 / CN-CINPUT-02 / DC-CINPUT-01 / DC-CINPUT-02a):** the recovered seed-epoch
consensus inputs are a **closed canonical type with a SOLE codec** (no second encoder/decoder pair —
CN-CINPUT-01). The RED/GREEN shells merge, encode, put, and append provenance; **all semantic decisions
(decode, binding verification, the leadership projection) live in BLUE**. Population is **contained** to
the verified-bootstrap composition path (`genesis_bootstrap` / `mithril_bootstrap`): the forge-time path
(`produce_mode` / `import_live_consensus_inputs` / `pool_distr_view_from_consensus_inputs` /
`--consensus-inputs-path`) MUST NOT build / put the sidecar nor append its WAL provenance
(`ci_check_consensus_input_provenance.sh`, a data-flow-resistant global call-site scan — CN-CINPUT-02).
The warm-start restore + replay fold live inside the **single `bootstrap_initial_state` authority** and
the BLUE `wal::replay_from_anchor` — **neither chokepoint moves**. **This domain is a CAPABILITY, not
production wiring:** the producer does not consume `PoolDistrView::from_seed_epoch_consensus_inputs`, the
warm-start restore is exercised at the authority surface only (`node.rs run_node_until_shutdown` not
wired; `recover_node_state` test-only), and **BA-02 is satisfied nowhere**. The PRODUCTION restart path
(DC-CINPUT-01's open obligation) **and** the CONSUME-side fence + producer consumption (CE-A-4b under
CN-CINPUT-02) are **owed to PHASE4-N-F-C**.

### Domain: bootstrap seed provenance (N-Y, extended N-Z + N-F-A)

| Layer | Module | Color | Role |
|-------|--------|-------|------|
| **Data-only Mithril shell** | `ade_runtime::mithril_import::{json, importer}` | RED | `parse_mithril_manifest_json` is the SOLE manifest-JSON parser → `RawMithrilManifest`; `import_mithril_manifest` / `import_mithril_manifest_from_bytes` map it into the closed `SeedProvenance::Mithril` + `MithrilManifestReport`. **No semantic decision; never re-verifies the STM multisig.** |
| **Mithril production-bootstrap composition** *(N-Z; +N-F-A sidecar tail)* | `ade_runtime::mithril_bootstrap::bootstrap_from_mithril_snapshot` | RED | **Composition-only** entry: imports the manifest, mints the anchor from the **operator-independent** `MithrilSeedPointInputs`, runs the BLUE `verify_mithril_binding` fail-closed **before** any storage init, routes through the single `bootstrap_initial_state` (passing `SeedEpochConsensusSource::NotRequired`), then runs the N-F-A sidecar tail (GREEN merge → A1 encode → put → WAL provenance append) on the success path. Symmetric with `bootstrap_from_conway_genesis`. **No new authority, no new `SeedProvenance` variant, no CLI surface.** Closed error sum `MithrilBootstrapError` {`Import`/`Binding`/`Bootstrap`} (+ the three `SeedConsensus*` variants from N-F-A). |
| **Data-only genesis shell** | `ade_runtime::genesis_bootstrap::bootstrap_from_conway_genesis` + `producer::genesis_parser` | RED | Reads + parses the Conway genesis file → `ConwayGenesisConfig`; routes the transform through the single bootstrap authority; runs the same N-F-A sidecar tail. No semantic transform. |
| **Authoritative binding predicate** | `ade_ledger::bootstrap_anchor::binding::verify_mithril_binding` | BLUE | The **sole** authority deciding whether a Mithril anchor binds — a pure predicate cross-checking the manifest's attested `{network_magic, genesis_hash, certified_point, certificate_hash}` against the independently-minted anchor; fails closed with `MithrilImportError`. |
| **Authoritative genesis transform** | `ade_ledger::genesis_source::genesis_initial_state` | BLUE | The pure Conway-only `ConwayGenesisConfig → (LedgerState, PraosChainDepState)` transform; fail-closed `GenesisSourceError::NonConwayEra`. |
| **Single bootstrap chokepoint** | `ade_runtime::bootstrap::bootstrap_initial_state` | GREEN-by-content (+ RED A3b restore) | The ONE authority all initial state flows through. `genesis_bootstrap`, the N-Y Mithril provenance path, the N-Z `bootstrap_from_mithril_snapshot` composition, AND the N-F-A warm-start restore all route here — never a parallel storage-init path. Returns the named `BootstrapState` (carrying `Option<SeedEpochConsensusInputs>`); the `SeedEpochConsensusSource` discriminant selects cold-start vs. warm-start restore. |

**Rule (CN-MITHRIL-01 / CN-NODE-01 / DC-GENESIS-SRC-01 / DC-MITHRIL-02):** the RED shells parse
bytes and produce reports/configs / mint anchors; **all** semantic decisions live in BLUE
(`verify_mithril_binding`, `genesis_initial_state`). All initial state — produce-mode
cold-start, the Conway genesis path, the Mithril path (provenance binding + N-Z production
composition), AND the N-F-A warm-start restore — routes through the **single**
`bootstrap_initial_state` authority via `BootstrapInputs.genesis_initial` +
`SeedEpochConsensusSource`. **There is NO `GenesisAnchor` / `MithrilAnchor` trait or plugin seam**
(`ci_check_mithril_uses_bootstrap_initial_state.sh`). **`verify_mithril_binding` MUST NOT be
tautological.** New seed-source support adds a RED parse/map shell + (if a new authoritative
decision is needed) a BLUE predicate/transform + (for production wiring) a composition-only RED
twin of `bootstrap_from_{conway_genesis, mithril_snapshot}` (each of which now also writes the
N-F-A recovered sidecar after bootstrap, never consuming one); **the `bootstrap_initial_state`
chokepoint never moves.**

### Domain: network forward-sync (durable-before-tip, N-Y)

| Layer | Module | Color | Role |
|-------|--------|-------|------|
| **Effect-plan reducer** | `ade_runtime::forward_sync::reducer` (`forward_sync_step`, `AdmitPlan::durable`) | GREEN-by-content | Composes the BLUE admit chokepoint (`ade_ledger::receive::receive_apply` / `admit_via_block_validity`) and emits the closed `SyncEffect` plan. The private `AdmitPlan::durable` is the **sole** `AdvanceTip` emitter and fixes the durable-before-tip order — an out-of-order plan is structurally inexpressible. |
| **Durability-ordered driver** | `ade_runtime::forward_sync::pump` (`pump_block`) | RED | Applies the reducer's `SyncEffect` plan in order against the persistent `ChainDb` + `FileWalStore` + snapshot writer; refuses to advance the tip before `StoreBlockBytes` + `AppendWal` return Ok — fails closed with `PumpError::TipBeforeDurable`. |

**Rule (DC-SYNC-01):** the GREEN reducer decides the effect plan; the RED pump applies it in
durable order and is the only place that touches sockets/files. **This GREEN-reducer /
RED-pump split deliberately mirrors the `ade_network::session` (GREEN) /
`ade_runtime::network::mux_pump` (RED) split.** `AdvanceTip` is unreachable before
`StoreBlockBytes` + `AppendWal` — `AdmitPlan` has no public out-of-order constructor
(`ci_check_forward_sync_chokepoint_only.sh`). New sync logic adds `SyncEffect` variants +
reducer arms; the pump applies them in plan order; **the single-`AdvanceTip`-emitter
chokepoint never moves.** This is an **acceptance-criterion** seam, not a registry-law surface.

### Domain: crash recovery (N-Y, extended N-F-A provenance fold)

| Layer | Module | Color | Role |
|-------|--------|-------|------|
| **Recovery wiring** | `ade_runtime::recovery::restart::recover_node_state` | RED | Composes the EXISTING authorities — `WalStore::read_all` + BLUE `wal::replay_from_anchor` (now returning `ReplayOutcome` with the optional `RecoveredBootstrapProvenance`) + `rollback_to_slot` — to reconcile the ChainDb to the WAL tail before warm-start. **No second recovery engine.** Fails fast on `NodeRecoveryError::WalTailFingerprintMismatch`. **Test-only secondary entry** at this HEAD — not wired to a production mode (the production restart path is owed to N-F-C). |

**Rule (recovery-contract / DC-WAL-*, strengthened N-Y; evidence-noted N-F-A):** recovery composes
existing authorities; it never re-implements replay or rollback (`ci_check_recovery_contract.sh`).
The N-F-A `ReplayOutcome` additively carries the recovered seed-epoch provenance without disturbing
the `AdmitBlock` fp-chain — every chain walk `match`es the additive variant explicitly, so a future
WAL variant is a compile error, not a silent missing link.

### Domain: N2N tag-24 wire envelope (N-X)

| Layer | Module | Color | Role |
|-------|--------|-------|------|
| **Sole byte wrap/unwrap authority** | `ade_codec::cbor::tag24::{wrap_tag24, unwrap_tag24}` | BLUE | The **single** workspace authority that wraps inner bytes in a tag-24 (`0xd8 0x18`) CBOR byte-string envelope and strips it. `unwrap_tag24` returns a zero-copy borrow of the verbatim inner bytes (no re-encode); fails closed with `TagEnvelopeError`. Each defined exactly once. |
| **BlockFetch composition** | `ade_network::codec::block_fetch::{compose,decompose}_blockfetch_block` | BLUE | A served `MsgBlock` payload = `tag24(bytes([era, block]))` — era **inside** the wrap; EBB-aware era index, **Conway = storage index 7**. |
| **ChainSync composition** | `ade_network::codec::chain_sync::{compose,decompose}_rollforward_header, chain_sync_wire_era_index}` | BLUE | A served `RollForward` header = `[era_tag, tag24(bytes(header_cbor))]` — era_tag **outside** the wrap; **CONSENSUS era index, Conway = 6 = storage − 1**. |
| **Serve emitters** | `ade_network::block_fetch::server` / `chain_sync::server` | BLUE | Emit composed (tag-24-wrapped) bytes — never a bare `[era, block]` / bare header. |
| **RED consumers (migrated)** | `ade_node::admission::runner` + `ade_core_interop::follow` | RED | Strip a peer's tag-24 envelope via `ade_codec::unwrap_tag24`; no local parse. |

**Rule (CN-WIRE-08):** one tag-24 byte authority + per-protocol composition layered over it.
The two N2N surfaces use **different era-index schemes** (BlockFetch storage Conway = 7;
ChainSync consensus Conway = 6 = storage − 1), pinned byte-identically against cardano-node
11.0.1 captures. No hand-rolled tag-24 parse in RED. **The wrap/unwrap chokepoint never moves.**

### Domain: block codec (decode + encode)

| Layer | Module | Color | Role |
|-------|--------|-------|------|
| **Authoritative ingress** | `ade_codec::cbor::envelope::decode_block_envelope` + per-era `decode_*_block` | BLUE | Sole `PreservedCbor` construction site; operates over the verbatim tag-24-stripped inner bytes on the wire path (N-X). |
| **Authoritative egress (N-V)** | `ade_codec::cbor::envelope::encode_block_envelope` | BLUE | The single block-envelope encoder; emits storage-form `[era, block]` (Conway = discriminant 7, head `82 07`). |
| **Producer consumer** | `ade_ledger::producer::forge::forge_block` | BLUE | Wraps forged output via `encode_block_envelope`. |

**Rule (CN-FORGE-03, strengthened N-X):** one block-envelope grammar in both directions;
forge and validate share it. The on-wire serve form is the N-X tag-24 composition over this
storage-form. **The encode/decode chokepoint pair never moves.**

### Domain: leader-eligibility VRF input (N-W)

| Layer | Module | Color | Role |
|-------|--------|-------|------|
| **Sole era→construction authority** | `ade_core::consensus::vrf_cert::leader_vrf_input(era, slot, eta0)` | BLUE | The single place selecting a Praos vs TPraos leader-eligibility VRF construction; returns the closed `ExpectedVrfInput`. |
| **Era-correct range-extension** | `ade_core::consensus::vrf_cert::leader_value_for` | BLUE | Praos `praos_leader_value` vs TPraos identity, dispatched on the `ExpectedVrfInput` variant. |
| **Leader-schedule producer** | `ade_core::consensus::leader_schedule::query_leader_schedule` | BLUE | Builds `LeaderScheduleAnswer.expected_vrf_input` via `leader_vrf_input`. |
| **RED prove-step consumer** | `ade_node::produce_mode::run_real_forge` | RED | Proves over `answer.expected_vrf_input.alpha_bytes()`; non-Praos era fail-closes to `ForgeFailureReason::UnsupportedProducerEra`. |

**Rule (CN-FORGE-04):** exactly one VRF transcript authority per era/protocol version; the
Praos producer alpha MUST equal the validator alpha. No both-alphas fallback. **The era→VRF
construction chokepoint never moves.**

### Domain: KES signing-key custody

| Layer | Module | Color | Role |
|-------|--------|-------|------|
| **Data-only loader** | `ade_runtime::producer::keys::load_kes_signing_key_skey` | RED | Reads the 608-byte cardano-cli skey envelope. |
| **Authoritative deserializer** | `ade_crypto::kes_sum::Sum6Kes::raw_deserialize_signing_key_kes` | BLUE | Byte layout is the structural validator. |
| **Authoritative algorithm** | `ade_crypto::kes_sum` | BLUE | Ade-native Sum6KES, byte-identical to Haskell `cardano-base`. |
| **Signing operation** | `ade_runtime::producer::signing` / `producer_shell::kes_sign_header` | RED | Sole key-custody surface; signs only the branded `UnsignedHeaderPreImage`. |

**Rule:** the RED loader may not call `KesSecret::from_*` inside `load_kes_signing_key_skey` —
only the BLUE deserializer path. Signing is RED-confined; BLUE never signs.

### Domain: leader eligibility (RED/BLUE split)

| Layer | Module | Color | Role |
|-------|--------|-------|------|
| **VRF proof producer** | `ade_node::produce_mode` (prove-step) | RED | Produces the VRF proof/output over the BLUE answer's `expected_vrf_input.alpha_bytes()`. |
| **Authoritative evaluator** | `ade_core::consensus::leader_check::verify_and_evaluate_leader` | BLUE | Verifies + evaluates eligibility from canonical inputs only; emits the closed `LeaderCheckVerdict`. |

**Rule (CN-FORGE-02):** BLUE never sees the VRF/KES/cold keys; the evaluator has no
`LedgerView`/`EraSchedule`/`ChainDepState`/clock/storage/RED dep. The RED/BLUE split never moves.

### Domain: forged-block serving (data-only serve vs. authoritative admit)

| Layer | Module | Color | Role |
|-------|--------|-------|------|
| **Authoritative admit** | `ade_ledger::producer::served_chain::served_chain_admit` | BLUE | Sole entry into the served index; only self-accepted blocks (CN-PROD-04). |
| **Atomic publisher** | `ade_runtime::producer::served_chain_handle::push_atomic` | RED (GREEN-by-content glue) | Wraps `served_chain_admit` in `watch::Sender::send_modify` — no torn snapshot. |
| **Read-side serve** | `ade_network::block_fetch::server::producer_block_fetch_serve` | BLUE | Serves a `RequestRange` only if endpoints + every intervening block are present; emits the tag-24 composition (N-X). |

**Rule:** a forged block is visible to peers only after `push_atomic`; the read-side serve is
data-only over the BLUE `ServedChainSnapshot`. The serve emitter wraps via the single tag-24
authority before bytes reach a peer (CN-WIRE-08).

---

## 3. Closed vs. Extensible Registries

### Closed (frozen — version-gated changes only)

| Registry | Location | Count | Change Rule |
|----------|----------|-------|-------------|
| `SeedEpochConsensusInputs` *(NEW, N-F-A)* | `ade_ledger::seed_consensus_inputs` (BLUE) | closed canonical record (`anchor_fp` / `epoch_no` / `active_slots_coeff` / `total_active_stake` / `pool_distribution: BTreeMap<Hash28, PoolEntry>`); **version-gated** behind `SEED_CINPUT_SCHEMA_VERSION = 1` | The recovered seed-epoch consensus-input record with a **SOLE** encoder/decoder pair (`encode_/decode_seed_epoch_consensus_inputs`). `decode_*` rejects any version != `SEED_CINPUT_SCHEMA_VERSION` fail-closed, and rejects a structurally-valid-but-non-canonical buffer (never silently re-canonicalizes). No `Default`, no `#[non_exhaustive]`, `BTreeMap` (no `HashMap`). New field / version = a `decode_*` arm + a `SEED_CINPUT_SCHEMA_VERSION` bump + a strengthening of **CN-CINPUT-01**. **No second codec** (the SOLE-codec property is enforced by the single-module surface; no dedicated grep gate). |
| `SeedConsensusInputsError` *(NEW, N-F-A)* | `ade_ledger::seed_consensus_inputs` (BLUE) | 6 (`MalformedCbor` / `UnknownVersion { expected, found }` / `Structural { reason }` / `NonCanonicalMapOrder` / `DuplicatePoolKey` / `TrailingBytes { extra }`) | The closed `decode_seed_epoch_consensus_inputs` failure set. New variant = a strengthening of **CN-CINPUT-01**; carries only non-secret primitives; MUST fail closed (no silent re-canonicalize). |
| `SeedConsensusMergeError` *(NEW, N-F-A)* | `ade_runtime::seed_consensus_merge` (GREEN) | 2 (`PoolMissingVrfKeyhash { pool }` / `PoolMissingStake { pool }`) | The closed bootstrap-time merge failure set — a pool present in exactly one source map fails closed here, **never a zero-hash fill**. New variant = a strengthening of the merge contract (CN-CINPUT-02 populate-side). No catch-all / `String` variant. |
| `SeedEpochConsensusSource` *(NEW, N-F-A)* | `ade_runtime::bootstrap` (RED) | 2 (`NotRequired` / `RequiredFromRecoveredProvenance(RecoveredBootstrapProvenance)`) | The **input-mode discriminant** for warm-start: `NotRequired` ⇒ no sidecar restore (every current caller); `RequiredFromRecoveredProvenance` ⇒ the A3b restore chain. **The CONSUME-side seam PHASE4-N-F-C wires** — a production restart mode passing the `RequiredFromRecoveredProvenance` variant. New variant = a strengthening of DC-CINPUT-01; no open/wildcard mode. |
| `BootstrapError` (N-F-A new variants) | `ade_runtime::bootstrap` (RED) | +5 (`SeedConsensusProvenanceMissing` / `SeedConsensusSidecarMissing { anchor_fp }` / `SeedConsensusHashMismatch { expected, actual }` / `SeedConsensusBindingMismatch { … }` / `SeedConsensusSidecarDecode(SeedConsensusInputsError)`) | The fail-closed warm-start-restore failure set. The restore MUST NOT fall back to the forge-time `--consensus-inputs-path` bundle on any of these. New variant = a strengthening of **DC-CINPUT-01**; carries only non-secret primitives. |
| `MithrilBootstrapError` *(N-Z; +N-F-A SeedConsensus* variants)* | `ade_runtime::mithril_bootstrap` (RED) | 3 base (`Import(MithrilManifestError)` / `Binding(MithrilImportError)` / `Bootstrap(BootstrapError)`) + the N-F-A `SeedConsensus*` variants | The closed RED-composition error sum for `bootstrap_from_mithril_snapshot` — one variant per composed step (import / BLUE binding verdict / single bootstrap authority) plus the N-F-A sidecar-tail steps (merge / persist / WAL append). No catch-all / `String` variant; the binding step is the SOLE semantic decision (BLUE). |
| `MithrilSeedPointInputs` *(N-Z)* | `ade_runtime::mithril_bootstrap` (RED) | closed struct (`seed_slot` / `seed_block_hash` / `network_magic` / `genesis_hash` / `seed_artifact_hash` / `imported_utxo_fingerprint` / `initial_ledger_fingerprint`) | The **operator-provided, structurally-independent** seed-point origin. **DC-MITHRIL-02:** the anchor `seed_point` is minted from these fields, NEVER from the manifest import — guarded by `ci_check_mithril_seed_point_independence.sh`. A new attested field = a struct addition + a strengthening of DC-MITHRIL-02 (and the corresponding `verify_mithril_binding` cross-check). |
| `MithrilBootstrapOutput` *(N-Z)* | `ade_runtime::mithril_bootstrap` (RED) | closed struct (`ledger` / `chain_dep` / `tip: Option<ChainTip>` / `anchor`) | The typed cold-start triple + the minted `BootstrapAnchor` recording `SeedProvenance::Mithril`. A new field = a struct addition behind the composition contract. |
| `SeedProvenance` *(N-Y; UNCHANGED by N-Z/N-F-A)* | `ade_ledger::bootstrap_anchor::anchor` (BLUE) | 2 (`CardanoCliJson` / `Mithril { certificate_hash, certified_point, immutable_range }`) | **Version-gated** behind `ANCHOR_SCHEMA_VERSION = 2` (additive 1→2; `decode_bootstrap_anchor` rejects an unknown version and round-trips byte-canonically). The enum is closed — no open/wildcard variant. N-Z/N-F-A added **NO new variant**. New variant = a `decode_bootstrap_anchor` arm + an `ANCHOR_SCHEMA_VERSION` bump + a strengthening of **CN-ANCHOR-01 / DC-ANCHOR-01**. |
| `MithrilImportError` *(N-Y)* | `ade_ledger::bootstrap_anchor::binding` (BLUE) | 5 (`NetworkMagicMismatch` / `GenesisHashMismatch` / `CertifiedPointMismatch` / `CertificateHashMismatch` / `UnsupportedArtifactType`) | The closed `verify_mithril_binding` failure set. New variant = a strengthening of **CN-MITHRIL-01 / DC-MITHRIL-01**; carries only non-secret primitives; MUST fail closed. |
| `MithrilManifestReport` *(N-Y)* | `ade_ledger::bootstrap_anchor::binding` (BLUE) | closed struct (attested `{network_magic, genesis_hash, certified_point, certificate_hash}`) | The report side fed to `verify_mithril_binding`. A new attested field = a struct addition + a strengthening of the binding predicate's cross-check. |
| `GenesisSourceError` *(N-Y)* | `ade_ledger::genesis_source` (BLUE) | 1 load-bearing (`NonConwayEra { found }`) | `genesis_initial_state` is Conway-only — any other era fail-closes here. New variant = a strengthening of **DC-GENESIS-SRC-01**; no implicit defaults / stringly fallback. |
| `SyncEffect` *(N-Y)* | `ade_runtime::forward_sync::reducer` (GREEN-by-content) | 4 (`StoreBlockBytes` / `AppendWal` / `CommitCheckpoint` / `AdvanceTip`) | The closed forward-sync effect plan. `AdvanceTip` is unreachable before `StoreBlockBytes` + `AppendWal` (`AdmitPlan::durable` is the sole emitter). New variant = a reducer arm + a pump apply-step + a strengthening of **DC-SYNC-01**. No open/wildcard effect. |
| `MithrilManifestError` *(N-Y)* | `ade_runtime::mithril_import::importer` (RED) | closed sum | The closed manifest-JSON parse failure set. New variant = a strengthening of the import-shell contract; no `String` in load-bearing variants; no semantic decision (binding is BLUE). |
| `PumpError` *(N-Y)* | `ade_runtime::forward_sync::pump` (RED) | closed sum (incl. `TipBeforeDurable`) | A tip-before-durable condition fail-closes to `TipBeforeDurable`. New variant = a strengthening of **DC-SYNC-01**. No catch-all. |
| `NodeRecoveryError` *(N-Y)* | `ade_runtime::recovery::restart` (RED) | closed sum (incl. `WalTailFingerprintMismatch { expected, actual }`) | A WAL-tail fingerprint divergence fails fast. New variant = a strengthening of the recovery contract / **DC-WAL-***. |
| `BlockVerdict` (observable surface) *(N-Y)* | `ade_testkit::harness::sync_diff` (GREEN) | 2 (`Admitted` / `Rejected`) | The closed **observable-surface** per-block verdict in the snapshot→tip differential harness. Compared on observable surfaces only — never Ade's internal `fingerprint` vs a Haskell hash (DC-COMPAT-01). New variant = a strengthening of **DC-COMPAT-01 / RO-SYNC-EVIDENCE-01**. |
| `RegressionFixtureViolation` *(N-Y)* | `ade_testkit::harness::sync_diff` (GREEN) | closed sum | Each discovered Haskell mismatch becomes a named regression fixture under `corpus/sync/regressions/`. New variant = a strengthening of **RO-SYNC-EVIDENCE-01**. |
| `TagEnvelopeError` *(N-X)* | `ade_codec::cbor::tag24` (BLUE) | 4 (`NotTag24` / `NotByteString` / `Truncated` / `TrailingBytes`) | New variant = a strengthening of **CN-WIRE-08**; carries only non-secret offset/length primitives. |
| `ExpectedVrfInput` *(N-W)* | `ade_core::consensus::vrf_cert` (BLUE) | 2 (`Praos([u8;32])` / `Tpraos([u8;41])`) | The 2-variant enum IS the protocol-family tag. New variant = a `leader_vrf_input` arm + a strengthening of **CN-FORGE-04**. No both-alphas fallback. |
| `LeaderCheckVerdict` *(N-R-A)* | `ade_core::consensus::leader_check` (BLUE) | 2 (`Eligible` / `NotEligible`) | New variant = a strengthening of **CN-FORGE-02**; `NotEligible` carries only a bounded fingerprint, never forge-capable material. |
| `ForgeFailureReason` *(extended N-W)* | `ade_runtime::producer::producer_log` (GREEN) | closed sum incl. `UnsupportedProducerEra` | New variant = a strengthening of **CN-FORGE-04 / DC-PROD-01**. No free-form reason strings. |
| `OutboundCommand` *(N-S-B)* | `ade_runtime::network::outbound_command` (RED) | typed `ChainSyncServerMsg` / `BlockFetchServerMsg` | New variant = a new typed mini-protocol reply. **No `Vec<u8>` byte tunnel** (CN-OUTBOUND-RELAY-01). |
| `DispatchError` *(N-S-B)* | `ade_node::produce_mode` + `ade_runtime::network::n2n_server` (RED) | closed sum (incl. `UnknownPeer`, `PeerOutboundMissing`) | No `String`-bearing / catch-all variant (CN-PEER-OUTBOUND-MAP-01). |
| `ChainEvolutionError` *(N-T)* | `ade_runtime::producer::chain_evolution` (GREEN) | closed sum (incl. `AuthorityMismatch`, `SelfAcceptRejected`) | New variant = a strengthening of **DC-PROD-03**. |
| `BroadcastPushError` *(N-T)* | `ade_node::produce_mode` (RED) | closed sum (incl. `SelfAcceptReplayRejected`) | New variant = a strengthening of **CN-PROD-04**. |
| `ProducerLogEvent` *(N-Q)* | `ade_runtime::producer::producer_log` (GREEN) | closed JSONL vocab | New variant = a strengthening of **DC-PROD-01**. No free-form reason strings, no key material. |
| `GenesisParseError` *(N-R-C)* | `ade_runtime::producer::genesis_parser` (RED) | closed sum | New variant = a strengthening of **CN-GENESIS-01**. |
| `OpCertParseError` *(N-R-C)* | `ade_runtime::producer::opcert_envelope` (RED) | closed sum | New variant = a strengthening of **CN-OPCERT-01**. |
| `UnsignedHeaderPreImageError` *(N-S-A)* | `ade_ledger::block_validity::unsigned_header_pre_image` (BLUE) | closed sum | New variant = a strengthening of **DC-KES-HEADER-01**. |
| `AcceptedMiniProtocol` *(N-L)* | `ade_network::session` (GREEN) | closed registry | New mini-protocol = a registry entry + a `match` arm with **no wildcard accept**. |
| `KesError` / `KesParseError` *(N-P)* | `ade_crypto::kes_sum::errors` (BLUE) | 5 / 6 variants | New variant = a strengthening of **DC-CRYPTO-08/09**; only non-secret primitives. |
| Operator-evidence manifest TOML schema *(N-S-C)* | `ci_check_operator_evidence_manifest_schema.sh` + `docs/clusters/completed/PHASE4-N-S-C/cluster.md` | closed key set | Any committed `CE-N-S-LIVE_*.toml` MUST conform; `peer_log_file_sha256` cross-checks the committed peer-log hash (CN-OPERATOR-EVIDENCE-01). |
| Sync-evidence manifest schema *(N-Y)* | `ci_check_sync_evidence_manifest_schema.sh` + `corpus/sync/regressions/` | closed key set (oracle versions, chain point, fixture refs, sha256, diff/acceptance result) | Mirrors the operator-evidence pattern; vacuously satisfied until a manifest is committed (RO-SYNC-EVIDENCE-01, **partial**). |
| `CardanoEra` + Conway cert / governance / withdrawal enums | `ade_types::{era, conway::*}` + `ade_codec::conway::*` | closed | New era / cert / gov-action = a versioned gate; no unknown-tag swallow, no silent skip (DC-LEDGER-08/09/10/11). `is_praos()` classifies exactly {Babbage, Conway}. |
| Consensus message + verdict enums | `ade_core::consensus`, `ade_ledger::block_validity` / `tx_validity` | closed | `ci_check_consensus_closed_enums.sh` — `match` with no wildcard. |
| JSONL event vocabularies (admission / wire-only / live-log) | `ade_node::{admission_log, live_log}`, `ade_runtime::admission` | closed | New event = a strengthening of the owning DC rule; allow-list + negative tests. |

### Extensible (open within constraints)

| Registry | Location | Extension Rule |
|----------|----------|----------------|
| Ade-native WAL (append-only) | `ade_runtime::wal` (GREEN-by-content) + `ade_ledger::wal::event` (BLUE encoder/decoder) | Append-only; committed entries are never mutated/rewritten (`ci_check_wal_append_only.sh`). **`WalEntry` itself is a deliberately CE-not-law surface** — its vocabulary is additively evolvable behind the WAL schema (append-only **wire tags**; `AdmitBlock` = 0, the **N-F-A** `SeedEpochConsensusInputsImported` = 3, tags 1/2 **reserved**). Every fp-chain walk `match`es the additive variant explicitly, so a new tag is a compile error, not a silent missing link. An acceptance criterion, NOT a frozen registry-law enum. |
| Seed-epoch sidecar store (anchor-fp-keyed) *(NEW, N-F-A)* | `ade_runtime::chaindb::SnapshotStore::{put,get}_seed_epoch_consensus_inputs` | A new entry is `put` only on the verified-bootstrap composition path (`genesis_bootstrap` / `mithril_bootstrap`), keyed by `anchor_fp` in a namespace **disjoint** from the slot-keyed snapshot space; idempotent on identical bytes, `InvalidOperation` on conflicting bytes for the same `anchor_fp` (redb `seed_cinputs_by_anchor_fp` table, `SCHEMA_VERSION = 3`). **This is the extension point PHASE4-N-F-C consumes** to wire a production restart that reads back the recovered surface; the forge-time path may NOT `put` here (CN-CINPUT-02). |
| `PerPeerOutbound` map *(N-S-B)* | `ade_runtime::network::outbound_command` — `Arc<RwLock<BTreeMap<PeerId, mpsc::Sender<OutboundCommand>>>>` | Grows at runtime: listener inserts on `PeerConnected`; MuxPump removes on disconnect. **`BTreeMap`, not `HashMap`** — deterministic iteration; no cross-peer byte leakage (CN-PEER-OUTBOUND-MAP-01, DC-OUTBOUND-FIFO-01). |
| `OpCertCounterMap` | `ade_core::consensus::praos_state` (BLUE) | Grows as op-certs are observed; deterministic ordering. |
| `ServedChainSnapshot` (served blocks) | `ade_ledger::producer::served_chain` (BLUE) | Grows via `served_chain_admit` only; `push_atomic` is the sole publisher. |
| `MempoolState` (admitted txs) | `ade_ledger::mempool` (BLUE) | Grows via `mempool_ingress` → `admit` only; sorted/deduplicated. |
| Seed entries (imported UTxO) | `ade_runtime::seed_import` (GREEN-by-content) | Grows at import time from a cardano-cli UTxO dump; canonical decoders only. |
| Persisted ChainDb (synced blocks) *(N-Y)* | `ade_runtime::chaindb` via `forward_sync::pump` | Grows via the forward-sync pump applying the GREEN reducer's `SyncEffect` plan in durable order; the tip advances only after `StoreBlockBytes` + `AppendWal` ack (DC-SYNC-01). |
| Sync regression fixtures *(N-Y)* | `corpus/sync/regressions/` | Each discovered Haskell observable-surface mismatch is committed as a named regression fixture (RO-SYNC-EVIDENCE-01); the harness replays them. |
| Sum_n KES family | `ade_crypto::kes_sum` (BLUE) | A new `Sum_n` attaches as an internal type-alias step; the `KesAlgorithm` trait surface does not change. |
| Per-protocol tag-24 compositions *(N-X)* | `ade_network::codec::{block_fetch, chain_sync}` | A new CBOR-in-CBOR composition attaches as a `compose_*` / `decompose_*` pair delegating to the single `ade_codec::{wrap_tag24, unwrap_tag24}` authority (CN-WIRE-08). |
| Bootstrap-source production compositions *(N-Z; +N-F-A sidecar tail)* | `ade_runtime::{genesis_bootstrap, mithril_bootstrap}` | A new bootstrap-source production entry attaches as a **composition-only RED twin** of `bootstrap_from_{conway_genesis, mithril_snapshot}`: import/parse + (if a point is attested) mint the anchor from an **operator-independent** origin + verify-before-bootstrap (fail-closed) + route through the single `bootstrap_initial_state` authority (passing `SeedEpochConsensusSource::NotRequired`) + the N-F-A sidecar tail (put-then-WAL-append; writes, never consumes). **No new authority, no new `*Anchor` trait/plugin, no new `SeedProvenance` variant unless the source genuinely differs** (CN-MITHRIL-01 / CN-NODE-01 / DC-MITHRIL-02 / CN-CINPUT-02). |

---

## 4. Version-Gated vs. Frozen Contracts

### Frozen (immutable at current version — change = new major version)

- **Seed-epoch consensus-input codec (N-F-A)** — `encode_/decode_seed_epoch_consensus_inputs` is the
  **SOLE** codec for `SeedEpochConsensusInputs`: deterministic CBOR, `BTreeMap`-ordered, byte-canonical
  (a structurally-valid-but-non-canonical buffer is rejected, never silently re-canonicalized). The wire
  shape at `SEED_CINPUT_SCHEMA_VERSION = 1` is frozen; the codec is version-gated for evolution (below).
  (CN-CINPUT-01.)
- **WAL additive-tag chain disjointness (N-F-A)** — the `SeedEpochConsensusInputsImported` (tag 3)
  provenance entry MUST stay distinct from the `AdmitBlock` `prior_fp`/`post_fp` chain; `replay_from_anchor`
  allows at most one provenance entry per store/anchor (fail-closed `DuplicateProvenance` /
  `ProvenanceAnchorMismatch`). The tag-3 wire layout is frozen; the WAL *vocabulary* is version-gated
  (CE-not-law, below). (DC-CINPUT-01.)
- **Warm-start restore verification chain (N-F-A)** — `restore_seed_epoch_consensus_inputs` verifies in a
  frozen order: `blake2b_256` bind → `decode_seed_epoch_consensus_inputs` → anchor/epoch binding →
  byte-identity re-encode, failing closed at each step; it MUST NOT fall back to the forge-time
  `--consensus-inputs-path` bundle. (DC-CINPUT-01.)
- **Sidecar populate ordering (N-F-A)** — the verified-bootstrap composers MUST put the sidecar (durable)
  **THEN** append the WAL provenance (the commit point), never the reverse; a crash between leaves an
  unrecorded sidecar that warm-start treats as "not imported". (CN-CINPUT-02.)
- **Mithril production-bootstrap composition order (N-Z)** — `bootstrap_from_mithril_snapshot`
  composes import → mint → `verify_mithril_binding` → `bootstrap_initial_state` (→ N-F-A sidecar tail)
  in that fixed order. `verify_mithril_binding` MUST precede `bootstrap_initial_state` (verify-before-
  bootstrap — no storage initializes on a mismatched binding); the anchor `seed_point` MUST be minted
  from the operator-independent `MithrilSeedPointInputs`, never the manifest import. The production
  composition may reference the import only as whole values (`import.provenance`, `&import.report`) —
  no field-drill, no `certified_point` mention. (CN-MITHRIL-01 strengthened / DC-MITHRIL-02.)
- **Mithril provenance binding cross-check (N-Y)** — `verify_mithril_binding` cross-checks
  the manifest's attested `{network_magic, genesis_hash, certified_point, certificate_hash}`
  against the independently-minted anchor. The four-field cross-check is the frozen binding
  contract; it MUST fail closed and MUST NOT be tautological. The STM multisig is the
  mithril-client's job — Ade never re-verifies it. (CN-MITHRIL-01 / DC-MITHRIL-01.)
- **N2N tag-24 wire envelope (N-X)** — the CBOR-in-CBOR `0xd8 0x18` byte-string envelope
  through the single `ade_codec::{wrap_tag24, unwrap_tag24}` authority. Per-protocol
  composition pinned byte-identically against cardano-node 11.0.1 captures: a served
  **BlockFetch** `MsgBlock` is `tag24(bytes([era, block]))` (era inside, storage index,
  Conway = 7); a served **ChainSync** `RollForward` header is
  `[era_tag, tag24(bytes(header_cbor))]` (era_tag outside, consensus index, Conway = 6).
  The two era-index schemes differ. (CN-WIRE-08.)
- **Leader-eligibility VRF transcript (N-W)** — for Praos eras the leader alpha is
  `praos_vrf_input(slot, eta0) = blake2b256(slot‖eta0)` + the `praos_leader_value`
  range-extension; for TPraos the role-tagged `slot‖eta0‖0x4C`. One era→construction
  authority (`leader_vrf_input`). (CN-FORGE-04.)
- **Block-envelope grammar (N-V)** — storage-form `[era, block]`, Conway = discriminant 7
  (head `82 07`); one encoder, one decoder, inverse-symmetric. The on-wire serve form is the
  N-X tag-24 composition over this. (CN-FORGE-03, strengthened N-X.)
- **Unsigned-header KES pre-image recipe (N-S-A)** — the canonical CBOR encoding of
  `ShelleyHeaderBody`; branded `UnsignedHeaderPreImage`'s only constructor is the canonical
  recipe; byte-identical to the validator extractor. (CN-KES-HEADER-01.)
- **Sum6KES algorithm + expand_seed prefix (N-P)** — byte-identical to Haskell `cardano-base`;
  `expand_seed` prefix bytes `0x01`/`0x02`. 608-byte skey + 448-byte signature layouts pinned.
- **Conway-genesis initial-state transform (N-Y)** — `genesis_initial_state` is the pure
  Conway-only `ConwayGenesisConfig → (LedgerState, PraosChainDepState)` transform; any other
  era fail-closes. (DC-GENESIS-SRC-01.)
- **Durable-before-tip ordering (N-Y)** — the forward-sync pump MUST persist
  `StoreBlockBytes` + `AppendWal` and receive durable acks before issuing the tip write; the
  GREEN reducer's `AdmitPlan::durable` is the sole `AdvanceTip` emitter. (DC-SYNC-01.)
- **Wire encoding** — `minicbor` / canonical CBOR; field order = wire order; `PreservedCbor`
  aliases the input bytes (no re-encode for hashing).
- **Hash algorithms** — Blake2b-224 / Blake2b-256; the single `block_body_hash` recipe; the
  `blake2b_256` sidecar-provenance bind (N-F-A).
- **Mux frame format** — single `encode_frame` / `decode_frame` pair workspace-wide.
- **All 456 canonical types** — existing wire formats frozen; new types may be added. (N-F-A added
  4 BLUE types in `ade_ledger` — `SeedEpochConsensusInputs`, `SeedConsensusInputsError`,
  `RecoveredBootstrapProvenance`, `ReplayOutcome`; the N-F-A RED types in `ade_runtime`
  — `BootstrapState`, `SeedEpochConsensusSource`, `SeedConsensusMergeError`, new `BootstrapError`
  variants — do NOT count toward the 456 BLUE total.)

### Version-gated (can evolve across major versions)

- **Seed-epoch consensus-input schema (N-F-A)** — `SEED_CINPUT_SCHEMA_VERSION` (currently `1`) gates
  `decode_seed_epoch_consensus_inputs`: any version != the constant fail-closes (`UnknownVersion`). A new
  field / shape = a `decode_*` arm + an additive version bump + a strengthening of CN-CINPUT-01.
- **WAL schema (CE-not-law)** — `WalEntry` is additively evolvable behind the WAL schema version
  (append-only wire tags; `AdmitBlock` = 0, `SeedEpochConsensusInputsImported` = 3, tags 1/2 reserved).
  It is exercised as a cluster acceptance criterion, NOT a frozen registry-law enum.
- **redb `chaindb` schema (N-F-A)** — `SCHEMA_VERSION` (currently `3`; v2→v3 added the
  `seed_cinputs_by_anchor_fp` sidecar table) gates the on-disk store; a newer on-disk schema fail-closes.
  A versioned gate, NOT a frozen contract. A new table / version = an additive bump.
- **Bootstrap-anchor schema (N-Y)** — `ANCHOR_SCHEMA_VERSION` (currently `2`) gates the
  `SeedProvenance` decode: `decode_bootstrap_anchor` rejects an unknown version. A new
  provenance variant = a `decode_bootstrap_anchor` arm + an additive version bump + a
  strengthening of CN-ANCHOR-01 / DC-ANCHOR-01. (N-Z / N-F-A added no new variant.)
- New era support: a `decode_*_block` arm + an `encode_block_envelope` discriminant + a
  `CardanoEra` variant + (leader path) an `ExpectedVrfInput` variant + a `leader_vrf_input`
  arm + (wire path) the per-protocol tag-24 era-index entries.
- New mini-protocol: an `AcceptedMiniProtocol` entry + a BLUE `*_transition` reducer +
  (serving) an `OutboundCommand` variant + (CBOR-in-CBOR) a `compose_*` / `decompose_*` pair.
- New seed source: a RED parse/map shell + (if a new authoritative decision is needed) a BLUE
  predicate/transform + (production wiring) a composition-only RED twin of
  `bootstrap_from_{conway_genesis, mithril_snapshot}`, routed through `bootstrap_initial_state`
  (NO new `*Anchor` trait/plugin; operator-independent `seed_point` origin per DC-MITHRIL-02; the
  N-F-A sidecar tail writes — never consumes — the recovered surface).
- **Recovered-surface consumption (N-F-C seam)** — a production restart mode passing
  `SeedEpochConsensusSource::RequiredFromRecoveredProvenance` + (leadership) consuming
  `PoolDistrView::from_seed_epoch_consensus_inputs` (CE-A-4b) attaches in PHASE4-N-F-C; it requires the
  CONSUME-side fence (the dual of the populate-side CN-CINPUT-02 containment).
- New `SyncEffect` variant: a reducer arm + a pump apply-step + a strengthening of DC-SYNC-01.
- New closed-enum variant (any of the §3 closed enums): a `[[rules]]` entry + a strengthening.
- New canonical-type fields (sort/dedup invariants preserved).
- New CI checks (existing checks may be tightened, never relaxed — RO-CLOSE-01).

---

## 5. Module Addition Rules

Derived from CODEMAP's Cross-Module Rules + the shared BLUE header.

| Color | Naming convention | Build-config flags | May depend on | MUST NOT depend on |
|-------|-------------------|--------------------|----------------|--------------------|
| **BLUE** | `ade_*` crate, or a BLUE `ade_network` submodule path in `.idd-config.json` `core_paths`; `// Core Contract:` + `//! BLUE …` banner first line | `#![deny(unsafe_code)]`, `deny(unwrap_used / expect_used / panic / float_arithmetic)`; no `#[cfg(feature = …)]` semantic gating | Other BLUE modules only (`ade_types` ← `ade_codec`/`ade_crypto` ← `ade_core` ← `ade_ledger`/`ade_plutus`; `ade_network` BLUE submodules ← `ade_codec`+`ade_types`) | `ade_runtime`, `ade_node`, `ade_core_interop`, the RED half of `ade_network`; std runtime / I/O / clock / rand / `HashMap` / float / async |
| **GREEN** | `ade_testkit` crate, `ade_network::session`, or a GREEN-by-content sub-tree inside `ade_runtime` / `ade_node` (incl. `forward_sync::reducer`, `seed_consensus_merge` (N-F-A), `harness::sync_diff`) with a `//! GREEN …` banner | Same deny attributes as BLUE; a purity CI gate per sub-tree | BLUE modules | RED modules in non-test deps; nondeterminism; secret material |
| **RED** | `ade_runtime`, `ade_node`, `ade_core_interop`, `ade_network::mux::transport` (incl. `forward_sync::pump`, `mithril_import`, `genesis_bootstrap`, `mithril_bootstrap` (N-Z), `seed_consensus_provenance` (N-F-A), `recovery::restart`); `//! RED …` banner | tokio/std/I/O allowed | Any module | — (RED is the leaf) |

### New module checklist

1. Add to `Cargo.toml` `[workspace] members` (BLUE submodule paths: also add to
   `.idd-config.json` `core_paths`).
2. Apply the `// Core Contract:` + `//! BLUE|GREEN|RED` banner first line
   (`ci_check_module_headers.sh`).
3. BLUE/GREEN: inherit the deny attributes; pass `ci_check_forbidden_patterns.sh`,
   `ci_check_no_async_in_blue.sh`, `ci_check_no_semantic_cfg.sh`.
4. `ci_check_dependency_boundary.sh` rejects forbidden cross-color imports;
   `ci_check_pallas_quarantine.sh` confines `pallas-*` to `ade_plutus`.
5. New canonical types: add round-trip tests (`canonical_type_registry: null`; canonical-type
   rules live inline in registry family T).
6. New closed surface: add a `[[rules]]` entry + a CI gate; reference it by ID in the docs.
7. **New seed source: route through `bootstrap_initial_state` — NO `*Anchor` trait/plugin
   seam** (`ci_check_mithril_uses_bootstrap_initial_state.sh`). **A production-bootstrap
   composition attaches as a composition-only RED twin of `bootstrap_from_{conway_genesis,
   mithril_snapshot}`: verify-before-bootstrap, fail-closed, operator-independent `seed_point`
   origin if a point is attested** (`ci_check_mithril_seed_point_independence.sh` — DC-MITHRIL-02),
   **and (N-F-A) a sidecar tail that WRITES — never consumes — the recovered seed-epoch surface
   (put-then-WAL-append), populate-side only** (`ci_check_consensus_input_provenance.sh` —
   CN-CINPUT-02).
8. **New recovered/canonical record with a SOLE codec (like N-F-A):** put the type + its
   encoder/decoder in a single BLUE module (the single-module surface IS the SOLE-codec
   enforcement — no dedicated grep gate); version-gate the decoder; keep it `BTreeMap`-ordered,
   byte-canonical, no `Default` / `#[non_exhaustive]` (CN-CINPUT-01-style).
9. **If a rule cites a moved/renamed source path: update its `code_locus` to match** —
   `ci_check_registry_code_locus_exists.sh` fails closed on any `crates/**.rs` / `ci/**.sh`
   path cited in a rule's `code_locus` that does not exist on disk.

### CI gates that enforce the boundary (105 total; the N-F-A / N-Z / N-Y / producer / network set)

| Script | Enforces | Cluster |
|---|---|---|
| `ci_check_consensus_input_provenance.sh` *(NEW, N-F-A)* | **CN-CINPUT-02** — a data-flow-resistant **containment** guard (global call-site scan): the seed-epoch consensus-input sidecar is populated only on the verified-bootstrap composition path (`genesis_bootstrap` / `mithril_bootstrap` → anchor-fp-keyed `put_seed_epoch_consensus_inputs`, via the GREEN merge + A1 SOLE encoder); the forge-time path (`produce_mode` / `import_live_consensus_inputs` / `pool_distr_view_from_consensus_inputs` / `--consensus-inputs-path`) may not populate it or append its WAL provenance. **Populate-side + forge-time-fence only; the CONSUME-side fence is owed to PHASE4-N-F-C.** | N-F-A |
| `ci_check_mithril_seed_point_independence.sh` *(N-Z)* | **DC-MITHRIL-02 + CN-MITHRIL-01 (strengthened)** — a data-flow-resistant **containment** guard on `bootstrap_from_mithril_snapshot`: (a) `verify_mithril_binding(` precedes `bootstrap_initial_state(`; (b) the `MintInputs.seed_slot`/`.seed_block_hash` RHS never traces to a manifest-origin token; (c) the production composition references the import only as whole values (no `import.report.<field>` / `import.provenance.<field>` drill, no `certified_point` mention). **A coherence guard, not a code seam.** | N-Z |
| `ci_check_forward_sync_chokepoint_only.sh` *(N-Y)* | DC-SYNC-01 — durable-before-tip; the GREEN reducer's `AdvanceTip` reachable only after `StoreBlockBytes` + `AppendWal`; `AdmitPlan` is the sole emitter. | N-Y |
| `ci_check_mithril_uses_bootstrap_initial_state.sh` *(N-Y)* | CN-MITHRIL-01 — the Mithril path routes initial state through the single `bootstrap_initial_state` authority + decides binding only via the BLUE `verify_mithril_binding`; never re-verifies the STM multisig; **no `*Anchor` trait/plugin seam.** | N-Y |
| `ci_check_no_haskell_fingerprint_equality.sh` *(N-Y)* | DC-COMPAT-01 — the compatibility harness compares observable surfaces only; no internal-ledger-fingerprint-vs-Haskell-hash equality. | N-Y |
| `ci_check_sync_evidence_manifest_schema.sh` *(N-Y)* | RO-SYNC-EVIDENCE-01 — closed sync-evidence manifest schema. | N-Y |
| `ci_check_recovery_contract.sh` *(strengthened N-Y)* | recovery-contract / DC-WAL-* — recovery composes existing authorities; reconciles ChainDb to the WAL tail; fail-fast on `WalTailFingerprintMismatch`. | N-Y |
| `ci_check_registry_code_locus_exists.sh` *(`5db9aae`, extended `a2af041`)* | Registry↔source coherence — every `crates/**.rs` + `ci/**.sh` path cited in any rule's `code_locus` must exist on disk; fails closed on a moved/renamed/deleted path; now also absorbs the retired constitution-coverage coherence checks. | post-N-Y |
| `ci_check_tag24_wire_authority.sh` | CN-WIRE-08 — single tag-24 wrap/unwrap authority; no hand-rolled tag-24 parse in RED; serve paths compose. | N-X |
| `ci_check_producer_praos_vrf.sh` | CN-FORGE-04 — single era→leader-VRF-input authority; closed `ExpectedVrfInput`. | N-W |
| `ci_check_leader_check_authority.sh` | CN-FORGE-02 — BLUE leader-check has no LedgerView/EraSchedule/RED dep; closed `LeaderCheckVerdict`. | N-R-A |
| `ci_check_unsigned_header_preimage_single_source.sh` | CN-KES-HEADER-01 / DC-KES-HEADER-01 — single canonical pre-image recipe. | N-S-A |
| `ci_check_no_produce_mode_direct_transport_writes.sh` | CN-OUTBOUND-RELAY-01 — bytes only via `OutboundCommand` → `MuxPump`. | N-S-B |
| `ci_check_operator_evidence_manifest_schema.sh` | CN-OPERATOR-EVIDENCE-01 — closed evidence-manifest TOML schema. | N-S-C |
| `ci_check_produce_mode_uses_bootstrap_initial_state.sh` | CN-PROD-03 / N-T — `produce_mode` obtains initial state only via `bootstrap_initial_state`. | N-T |
| `ci_check_forge_decode_round_trip.sh`, `ci_check_no_independent_forge_codepath.sh` | CN-FORGE-03 (strengthened N-X) — single forge codepath; round-trips. | N-V |
| `ci_check_producer_coordinator_no_secrets.sh`, `ci_check_node_mode_closure.sh` | CN-PROD-02 — GREEN coordinator holds no secrets; closed `ade_node` mode set. | N-Q |

> The retired `ci_check_constitution_coverage.sh` (a ziranity-v3 import) was removed in `a2af041`
> (105 → 104), its coherence checks folded into `ci_check_registry_code_locus_exists.sh`; the N-F-A
> `ci_check_consensus_input_provenance.sh` restored the count to **105**. Earlier-cluster gates
> (N-A..N-P, the N-M-* admission/seed/WAL/anchor set, the N-L wire-session set) are present in the
> 105 total; per-script detail is in the registry's `ci_script` fields. The full list is
> `ls ci/ci_check_*.sh`.

---

## 6. Forbidden Patterns (per color)

- **BLUE:** no clock, rand, raw `HashMap`/`HashSet`/`IndexMap`, float, env access,
  network/filesystem, async runtime, locale-dependent ops, OS-dependent ordering. No signing
  (`ci_check_no_signing_in_blue.sh`). No `#[cfg(feature = …)]` semantic gating. No
  `PreservedCbor` construction outside `ade_codec`. No re-encode of wire bytes when hashing.
  No second era→leader-VRF-input construction (CN-FORGE-04). No second `wrap_tag24` /
  `unwrap_tag24` definition (CN-WIRE-08). No second bootstrap/storage-init authority
  (CN-NODE-01 / DC-GENESIS-SRC-01); no tautological Mithril binding check (CN-MITHRIL-01);
  `genesis_initial_state` is Conway-only and never a second bootstrap authority
  (DC-GENESIS-SRC-01). **(N-F-A) No second `SeedEpochConsensusInputs` encoder/decoder pair
  (CN-CINPUT-01 — SOLE codec); `decode_seed_epoch_consensus_inputs` MUST be version-gated and
  byte-canonical (reject a non-canonical buffer, never silently re-canonicalize) and MUST keep the
  record `BTreeMap`-ordered with no `Default` / `#[non_exhaustive]`. The `wal::replay_from_anchor`
  fold MUST keep the `SeedEpochConsensusInputsImported` provenance distinct from the `AdmitBlock`
  fp-chain (explicit `match`, no wildcard) and MUST be pure. `PoolDistrView::from_seed_epoch_consensus_inputs`
  MUST be a pure field-map (off-epoch ⇒ `None`, no `HashMap`) — it is a projection, NOT a producer
  consumption point.**
- **GREEN:** no nondeterminism; no participation in authoritative outputs. The
  `producer::coordinator` MUST NOT own/store private signing material. `ChainEvolution` (N-T)
  MUST NEVER mint `AcceptedBlock`. Closed vocabularies (`ProducerLogEvent`,
  `ForgeFailureReason`, `SyncEffect`, observable `BlockVerdict`) — no open/wildcard variant.
  `forward_sync::reducer` (DC-SYNC-01): MUST NOT emit `AdvanceTip` for a block before that
  block's `StoreBlockBytes` + `AppendWal` (structurally unrepresentable — `AdmitPlan` has no
  public out-of-order constructor); MUST NOT touch sockets/files (that is the RED pump's job).
  **(N-F-A) `seed_consensus_merge` MUST fail closed (`SeedConsensusMergeError`) on a pool present
  in exactly one source map — NEVER a zero-hash fill; it produces the BLUE record but is NOT its
  author and decides nothing semantic; `BTreeMap` only.**
  `harness::sync_diff` (DC-COMPAT-01): MUST NOT compare Ade's internal ledger `fingerprint` to
  a Haskell / serialized-state hash — compatibility is proven only on observable surfaces.
  Evidence/admission reducers compare already-authoritative outputs; `lagging` ≠ success; wire
  success ≠ admission ≠ agreement.
- **RED:** no direct mutation of BLUE state; no construction of semantic types from raw bytes;
  no bypassing canonical validation. `produce_mode` emits outbound bytes only via
  `OutboundCommand` (no direct transport write, no `Vec<u8>` byte tunnel). The per-peer
  outbound map is `BTreeMap` (deterministic), keyed by `PeerId`. Key custody confined to
  `producer::signing` / `producer_shell`. `run_real_forge` (N-W) MUST NOT perform RED-side era
  dispatch for the leader-VRF alpha. No hand-rolled tag-24 parse (CN-WIRE-08).
  `forward_sync::pump` (DC-SYNC-01) MUST refuse to advance the tip before the durability writes
  ack (`PumpError::TipBeforeDurable`). `mithril_import` MUST perform no semantic decision, MUST NOT
  re-verify the STM multisig, and MUST route initial state through the single
  `bootstrap_initial_state` authority (CN-MITHRIL-01). `genesis_bootstrap` / `mithril_bootstrap`
  MUST route through the same single authority — never a parallel storage-init path (CN-NODE-01 /
  DC-GENESIS-SRC-01); **(N-Z)** mint the anchor `seed_point` from the operator-independent
  `MithrilSeedPointInputs` ONLY (never drill the manifest import / name `certified_point` / launder
  via a one-hop local), and run `verify_mithril_binding` fail-closed BEFORE `bootstrap_initial_state`
  (DC-MITHRIL-02 — `ci_check_mithril_seed_point_independence.sh`). `recovery::restart` MUST compose
  the existing WAL-replay + rollback authorities (no second recovery engine) and fail fast on
  `WalTailFingerprintMismatch`. **(N-F-A) `seed_consensus_provenance::append_seed_epoch_provenance`
  MUST `blake2b_256` the EXACT A1 bytes the composer `put` (never a re-encode), be called only AFTER
  the durable sidecar put, and be referenced only at the two verified-bootstrap composition sites.
  The verified-bootstrap composers' sidecar tail MUST put-then-WAL-append (never the reverse) and
  WRITE — never consume — the recovered surface; the `chaindb` sidecar surface
  (`put_/get_seed_epoch_consensus_inputs`) is an anchor-fp-keyed namespace disjoint from the
  slot-keyed snapshot space (never a sentinel slot), idempotent on identical bytes /
  `InvalidOperation` on conflict, redb `SCHEMA_VERSION = 3` fail-closed on a newer on-disk schema.
  `bootstrap::restore_seed_epoch_consensus_inputs` MUST fail closed on a missing sidecar / hash
  mismatch / non-canonical decode / binding mismatch / non-byte-identical re-encode, and MUST NOT
  fall back to the forge-time `--consensus-inputs-path` bundle. The forge-time `produce_mode` path
  MUST pass `SeedEpochConsensusSource::NotRequired` and MUST NOT build / put the sidecar nor append
  its WAL provenance (CN-CINPUT-02 — `ci_check_consensus_input_provenance.sh`).**

### Project-specific additions (Ade)

- **Recovered-state surface is populate-contained, consume-deferred (N-F-A, CN-CINPUT-02):** the
  recovered seed-epoch consensus inputs are populated ONLY on the verified-bootstrap composition path
  (put-then-WAL-append) and read back ONLY by the warm-start restore inside `bootstrap_initial_state`.
  The forge-time path may not populate them. The CONSUME-side wiring (a production restart mode passing
  `SeedEpochConsensusSource::RequiredFromRecoveredProvenance`, and producer leadership consumption of
  `PoolDistrView::from_seed_epoch_consensus_inputs` — CE-A-4b) is the seam **PHASE4-N-F-C** attaches; it
  is NOT a capability claimed at this HEAD. `BA-02` is satisfied nowhere here.
- **No new bootstrap-source plugin seam (N-Y hard rejection, carried into N-Z + N-F-A):** a new seed
  source (Mithril, genesis, future) attaches by populating `BootstrapInputs.genesis_initial`
  and routing through `bootstrap_initial_state` — NEVER via a `GenesisAnchor` / `MithrilAnchor`
  trait or plugin registry. A production-bootstrap composition is a composition-only RED twin of
  `bootstrap_from_{conway_genesis, mithril_snapshot}` (now also writing the N-F-A recovered sidecar
  after bootstrap), not a new authority. The seam is an acceptance criterion, not a registry invariant.
- **Mithril seed-point independence (N-Z hard rule, DC-MITHRIL-02):** in a Mithril-bootstrap
  composition the anchor `seed_point` MUST originate from an operator-supplied origin
  structurally independent of the manifest; `verify_mithril_binding` cross-checks the two and
  fails closed; the binding must run before any storage init. Re-tautologization (sourcing the
  seed_point from the manifest, even via a one-hop local) is CI-blocked.
- **No synthetic forge state (N-T):** `produce_mode` MUST NOT construct `SyntheticForgeInputs`,
  a zero-stake `LeaderScheduleAnswer`, or an inline `LedgerState::new(...)` forge base.
- **No durability in the produce_mode path (N-U scope):** forged-block durability is deferred
  to N-U (see §7). (Distinct from the **network** forward-sync durability, which DID land in
  N-Y for received blocks.)
- **Registry `code_locus` must track source moves (`5db9aae`):** any rule citing a renamed /
  moved `crates/**.rs` or `ci/**.sh` path must have its `code_locus` updated — the
  `ci_check_registry_code_locus_exists.sh` gate fails closed on a stale pointer. A
  traceability-coherence guard, not a closed-surface seam.
- **`cardano_crypto::kes` is a `#[cfg(test)]` oracle only** under `crates/ade_crypto/src/**`.
  `pallas-*` confined to `ade_plutus`.
- **Commit-attribution override (CLAUDE.md):** this repo carries a model-attribution trailer
  on commit messages only (bounty requirement). Source comments, PRs, releases, issue
  comments still follow the global no-AI-attribution rule.
- **Grounding-doc → ade-atlas rebuild trigger (`f0d0bf9`, operational infra — NOT a code
  seam):** `.github/workflows/notify-atlas.yml` notifies the downstream `ade-atlas` repo to
  rebuild when the grounding docs change. It attaches nothing to the node's authority surface.

---

## 7. Candidate & Not-Yet-Wired Seams (declared follow-ons — NOT closed)

> Surfaced honestly per IDD: these are **declared** future attach points, not closed
> surfaces. Each is named in a registry rule or a cluster CLOSURE record.

1. **PHASE4-N-F-C — recovered seed-epoch consensus-input CONSUMPTION (the N-F-A successor).**
   N-F-A shipped the recovered surface as a **capability**: the closed `SeedEpochConsensusInputs`
   record + SOLE codec (CN-CINPUT-01), the populate-side containment (CN-CINPUT-02), the warm-start
   restore proven at the authority surface (DC-CINPUT-01 **partial**), and the
   `PoolDistrView::from_seed_epoch_consensus_inputs` projection (DC-CINPUT-02a). **The following seams
   are deliberately NOT wired and are owed to N-F-C:**
   - **(a) Production restart path** — a production mode that opens a persistent store + `FileWalStore`,
     replays to a `RecoveredBootstrapProvenance`, and calls
     `bootstrap_initial_state(RequiredFromRecoveredProvenance)`. `node.rs run_node_until_shutdown` is
     NOT wired to it; `recover_node_state` is test-only. (DC-CINPUT-01 `open_obligation`.)
   - **(b) Producer consumption (CE-A-4b)** — `produce_mode` consuming the recovered
     `PoolDistrView` as its leadership source instead of cold-starting from `--consensus-inputs-path`.
     At this HEAD `produce_mode` passes `SeedEpochConsensusSource::NotRequired` and cannot name the
     recovered type.
   - **(c) CONSUME-side fence** — the dual of CN-CINPUT-02's populate-side containment: a
     data-flow-resistant guard that only the production restart path may pass
     `RequiredFromRecoveredProvenance`. The current gate fences the POPULATE side only.
   - The attach points N-F-C will use: the **anchor-fp-keyed sidecar `SnapshotStore` surface**
     (`get_seed_epoch_consensus_inputs`), the **`SeedEpochConsensusSource` enum** (the
     `RequiredFromRecoveredProvenance` mode), and the **WAL additive-tag provenance**
     (`replay_from_anchor` → `ReplayOutcome.recovered_provenance`). **BA-02 (Haskell peer accepts an
     Ade-forged block) remains gated on the producer recovered-state lifecycle — satisfied nowhere
     at this HEAD.**

2. **Mithril import — remaining open obligations (RO-MITHRIL-IMPORT-01, still `partial`,
   `blocked_until_mithril_seed_bytes_and_fixture`).** N-Y shipped the BLUE *provenance binding*
   (CN-MITHRIL-01 / DC-MITHRIL-01); **N-Z CLOSED item (b)** — `bootstrap_from_mithril_snapshot`
   is the wired production composition (verify-before-bootstrap, fail-closed; DC-MITHRIL-02). **Two
   seams remain deliberately NOT wired:**
   - **(a) seed-bytes-from-Mithril decode** — a Mithril artifact-type spike + forward-replay
     (currently the operator supplies the cold-start `(LedgerState, PraosChainDepState)` and the
     independent seed-point inputs; Ade does not yet decode the Mithril snapshot artifact into
     seed bytes itself).
   - **(c) a committed reproducible Mithril fixture + CI/release evidence.**
   - **CLI surface** — `bootstrap_from_mithril_snapshot` is composition-only with **NO argv flag**;
     an operator-facing CLI flag is a deliberately-deferred **future operator-UX slice** (symmetric
     deferral with the genesis path's library-only shape).

3. **N-U — forged-block durability.** WAL / ChainDB / snapshot / warm-start for
   producer-**forged** blocks (crash → bootstrap warm-start). Out of N-T scope
   (`open_obligation` on CN-PROD-03 / DC-PROD-03). The N-Y forward-sync durability covers
   **received** blocks; the producer-forge durability is a separate follow-on. (Distinct from the
   N-F-A recovered consensus-input sidecar, which is bootstrap-time provenance, not forge-time
   block durability.)

4. **Sync-evidence live leg (N-Y — RO-SYNC-EVIDENCE-01, `partial`).** The
   snapshot→tip sync-evidence manifest schema is enforced, but the gate is **vacuously
   satisfied** until a manifest is committed (mirrors CN-OPERATOR-EVIDENCE-01). The
   two-Haskell-node private-Conway-testnet live leg is operator-witnessed; an execution gate,
   not a code seam.

### Operator-pass execution gates (schema enforced, execution blocked)

- **CN-OPERATOR-EVIDENCE-01 / CN-CONS-06 / RO-LIVE-01** — the manifest schema is enforced,
  but C1 (private testnet) / C2 (preprod) operator-pass execution is
  `blocked_until_operator_pass_executed`. With CN-FORGE-04 (N-W) and CN-WIRE-08 (N-X) enforced,
  the producer forge composition is mechanically complete through the serve step. The
  remaining blocker is the OPERATOR-PASS live leg itself. (A C1 scoping seed lives at
  `docs/planning/operator-pass-live-leg-c1-followon.md`.)

---

## Generation notes

- Regenerated (scoped refresh) at HEAD `a3a0636` (`git rev-parse --short HEAD`), downstream of the
  CODEMAP regenerated at the same HEAD. PHASE4-N-F-A (recovered seed-epoch consensus-input
  CAPABILITY) landed in A1 (`c13c2e9`), A2 (`f6bf50f`), A3a (`c507159`), A3b (`104982d`), A4
  (`8b60524`), with the A5-scoping doc revisions (`2cf28f0` → `02f3e87` → `a3a0636`) recording that
  producer recovered-state consumption is a successor cluster (PHASE4-N-F-C), not a slice. The
  refresh is **scoped to the N-F-A authority surface + the mechanical counts that drifted**; the
  N-Q/N-R/N-S/N-W/N-X/N-Y/N-Z seam entries are carried forward unchanged.
- **Capability-not-wiring honesty (load-bearing).** This cluster ships a recovered-state *capability*,
  not production wiring. A4's `PoolDistrView::from_seed_epoch_consensus_inputs` is a projection the
  producer does **not** consume (`produce_mode` still cold-starts from `--consensus-inputs-path` +
  `SeedEpochConsensusSource::NotRequired`). A3b's warm-start verification is proven at the authority
  surface (`bootstrap_initial_state` exercised directly); `node.rs run_node_until_shutdown` is not
  wired to the recovery path and `recover_node_state` is test-only. **BA-02 (Haskell peer accepts an
  Ade-forged block) is satisfied nowhere.** DC-CINPUT-01 is the only N-F-A rule at `partial` for
  exactly this reason; its open obligation (the production restart path) is deferred to PHASE4-N-F-C,
  as is the CONSUME-side fence + producer consumption (CE-A-4b) under CN-CINPUT-02.
- N-F-A delta verified at `a3a0636` (grep/ls/git only — no `cargo`):
  - `SeedEpochConsensusInputs` (`seed_consensus_inputs.rs:49`) + SOLE codec
    (`encode_seed_epoch_consensus_inputs:98` / `decode_seed_epoch_consensus_inputs:140`) +
    `SEED_CINPUT_SCHEMA_VERSION = 1` (`:39`) + 6-variant `SeedConsensusInputsError` (`:64`:
    `MalformedCbor` / `UnknownVersion` / `Structural` / `NonCanonicalMapOrder` / `DuplicatePoolKey` /
    `TrailingBytes`). All BLUE (under `crates/ade_ledger/`).
  - WAL: `TAG_ADMIT_BLOCK = 0` (`wal/event.rs:36`), `TAG_SEED_EPOCH_CONSENSUS_INPUTS_IMPORTED = 3`
    (`:42`), variant `SeedEpochConsensusInputsImported` (`:68`); comment confirms tags 1/2 reserved.
  - `SeedConsensusMergeError` (2-variant: `PoolMissingVrfKeyhash` / `PoolMissingStake`,
    `seed_consensus_merge.rs:41`, GREEN). `SeedEpochConsensusSource` (2-variant: `NotRequired` /
    `RequiredFromRecoveredProvenance`, `bootstrap.rs:57`, RED) + `BootstrapState` (`:92`) + 5 new
    `BootstrapError` variants (`:124`–`:143`). Source comment confirms "current caller passes
    `NotRequired`".
  - redb `SCHEMA_VERSION = 3` (`chaindb/persistent.rs:45`, fail-closed on `version > SCHEMA_VERSION`
    at `:137`), `seed_cinputs_by_anchor_fp` table (`:37`); `SnapshotStore::{put,get}_seed_epoch_consensus_inputs`
    trait decl (`chaindb/mod.rs:127`/`:135`) + both impls (`persistent.rs:565`/`:610`,
    `in_memory.rs:177`/`:197`).
  - `PoolDistrView::from_seed_epoch_consensus_inputs` (`consensus_view.rs:82`, BLUE projection).
  - CI gate `ci/ci_check_consensus_input_provenance.sh` present (`ls` confirmed); Guard (b) fences
    `produce_mode.rs` against `(put|merge|encode)_seed_epoch_consensus_inputs` /
    `append_seed_epoch_provenance` / `SeedEpochConsensusInputs*`; allowed populator sites =
    `genesis_bootstrap.rs` / `mithril_bootstrap.rs` / `seed_consensus_merge.rs` /
    `seed_consensus_provenance.rs`. The gate does **not** reference `get_seed_epoch_consensus_inputs`
    or `RequiredFromRecoveredProvenance` — confirming the CONSUME-side fence is genuinely not yet
    built (owed to N-F-C).
- Counts at `a3a0636`: **456** canonical types (BLUE; Δ +4 — the 4 N-F-A BLUE `ade_ledger` types;
  the N-F-A RED `ade_runtime` types do not count), **105** CI checks (Δ 0 net:
  `ci_check_constitution_coverage.sh` retired `a2af041`, `ci_check_consensus_input_provenance.sh`
  added `f6bf50f`), **303** registry rules (Δ +4: CN-CINPUT-01, CN-CINPUT-02, DC-CINPUT-01 (partial),
  DC-CINPUT-02a).
- All N-Z / N-Y closed surfaces re-verified present on disk at this HEAD and unchanged by N-F-A
  except for the threaded `&mut dyn WalStore` + sidecar tail on the two bootstrap composers and the
  three `SeedConsensus*` error variants on `MithrilBootstrapError` / `GenesisBootstrapError`.
- `.idd-config.json` `_invariant_registry_doc` still reads "299 entries" — now **stale** at 303.
  `_head_deltas_baseline` is still `67d1ccc` (the N-Z close baseline); bump it to `a3a0636` on the
  N-F-A HEAD_DELTAS refresh.
- The doc is regenerated, not edited. If a value drifts, fix the source, not the doc.
- NOTE: no `cargo build`/`test`/`check` was run during this regeneration (grep/ls/git only, per the
  task constraint).
