# Cluster PHASE4-N-F-C — Build the real Ade node lifecycle

> **Status: OPEN (revised plan, code-verified 2026-05-30).** This document REPLACES the prior
> PHASE4-N-F-C cluster framing (the withdrawn "evolve `Mode::Admission` / Option A" owner plan). That
> earlier plan rested on a false claim — that `admission::run_admission_inner` already routes through
> `bootstrap_initial_state` — which reading the actual code disproved (`admission` builds ledger/chain_dep
> manually via `seed_to_snapshot` and never calls `bootstrap_initial_state`). The false Admission-owner
> plan is not preserved as active guidance.
>
> Every "reuse X" below is verified by reading the actual function body at `a885bb3`, not headers/docs.
> Companion docs: `C1-production-lifecycle-owner.md` (the L1 slice), `../../planning/phase4-n-f-c-cluster-slice-plan.md`
> (the L1–L6 ordered plan), `../../planning/phase4-n-f-c-invariants.md` (invariant sketch — still valid;
> provenance-not-shape safety line holds).

## Primary invariant
The Ade node lifecycle forges from Ade-owned **recovered** state — established at a Mithril-certified
first-run bootstrap, persisted to `PersistentChainDb` + `FileWalStore`, WAL-proven, and warm-start
restored+verified — never from a forge-time operator bundle; the shape-swap of an operator bundle into
the recovered `SeedEpochConsensusInputs` surface is unrepresentable, and exactly one lifecycle owner
(`--mode node`) threads first-run/recovery → produce without a second bootstrap authority (CN-NODE-01).

## The one lifecycle

```
FIRST RUN (empty persistent store):
  Mithril-certified bootstrap ONLY → verify_mithril_binding (fail-closed, before any state admitted)
    → bootstrap_from_mithril_snapshot → PersistentChainDb + FileWalStore
    → one BootstrapAnchor lineage → persist SeedEpochConsensusInputs sidecar → WAL provenance
WARM START (non-empty persistent store):
  WAL replay → RecoveredBootstrapProvenance → bootstrap_initial_state(RequiredFromRecoveredProvenance)
    → recovered BootstrapState (fail-closed; no fallback)
BOTH →
  peer BlockFetch → validated durable apply → selected recoverable tip
    → forge base = recovered tip + PoolDistrView::from_seed_epoch_consensus_inputs(recovered)
    → run_real_forge → success ONLY on real Haskell-peer acceptance
```

**Mithril gets Ade started. After bootstrap, WAL + `PersistentChainDb` explain all Ade state.** Mithril
is queried exactly once (first run only); the first-run-vs-warm-start branch is a pure function of
on-disk state.

## Locked rules (from the reset conversation)

- **Mithril-only first run, fail-closed.** **No real Mithril provenance, no first-run bootstrap.** The
  cardano-cli UTxO/consensus-input extraction is valid **only** when it is bound to Mithril-certified /
  Mithril-restored state **and** `verify_mithril_binding` succeeds. Absent a verified Mithril
  certificate/binding, first-run bootstrap fails closed — no genesis branch, no `--consensus-inputs-path`
  substitute, no tip-bundle, no cold-`produce_mode` substitute. Conway genesis / genesis full replay are
  out of scope for this cluster (a private-testnet genesis path, if ever needed, is a separate named
  cluster — never a branch here).
- **Mithril = provenance/certification, not byte source.** `verify_mithril_binding` is the trust gate;
  the seed *bytes* arrive via documented cardano-cli extraction from the Mithril-restored peer. Native
  decode of Mithril UTXO-HD/LedgerDB bytes is a locked Tier-4 non-goal.
- **Mithril queried once.** After the anchor, WAL + persisted Ade state are the sole authority; Mithril is
  never re-queried, never a forge-time input, never a recovery fallback.
- **No shape-swap laundering.** An operator bundle converted to a `SeedEpochConsensusInputs`-shaped object
  at forge time is CI-unrepresentable (CN-CINPUT-03).
- **No second bootstrap/recovery/storage-init authority** (CN-NODE-01).
- **No BA-02 claim** until a real Haskell peer accepts an Ade-forged block.
- Scope: **single-epoch, single-shot** until forged-block durability (N-U) lands.

## Verified component inventory (read, not assumed)

| Component | Real state at `a885bb3` | Use |
|---|---|---|
| `bootstrap_initial_state` + `restore_seed_epoch_consensus_inputs` | warm-start `RequiredFromRecoveredProvenance` does full fail-closed verify (hash → A1 decode → anchor/epoch binding → byte-identity) | L3 calls directly |
| `replay_from_anchor` → `ReplayOutcome{recovered_provenance}` | produces `RecoveredBootstrapProvenance` | L3 provenance source |
| `recover_node_state` | **passes `NotRequired`, ignores `recovered_provenance`, test-only** | NOT the warm-start path; stays test/capability |
| `bootstrap_from_mithril_snapshot` + `verify_mithril_binding` | composer; seed bytes are args (cardano-cli extraction), Mithril = provenance; test-only callers | L2 first production caller |
| `PersistentChainDb` (ChainDb + SnapshotStore + sidecar put/get) | real redb impls, anchor-fp-keyed sidecar, idempotent | L1/L2/L4 store |
| `admission::wire_pump` | real N2N `BlockFetch` (`RequestRange`→`Block{bytes}`); feeds `admit_via_block_validity` + WAL; **does NOT persist blocks / advance recoverable tip / call forward_sync** | L4a fetch source |
| `forward_sync::pump_block` | durable validated apply (store bytes → WAL → advance tip, durable-before-tip); **zero production callers — no fetcher** | L4b apply engine |
| `ade_core_interop::follow` | RED tip-*selection* agreement only; no header/VRF/ledger validation; trusts peer | NOT validating sync |
| `run_real_forge` | takes `ForgeRequestContext{pool_distr_view, eta0, leader_schedule_answer, base_state, …}` | L5 forge engine |
| `PoolDistrView::from_seed_epoch_consensus_inputs` | pure field map, impls LedgerView | L5 forge base |

## Slices (safety order)

### L1 — Lifecycle owner skeleton + branch (`--mode node`)
New `--mode node` owning `PersistentChainDb` + `FileWalStore`. First-run-vs-warm-start branch = **pure
function of on-disk state** (empty store ⇒ first-run; non-empty ⇒ warm-start). Both arms route through the
single `bootstrap_initial_state` authority (CN-NODE-01). No Mithril fallback path exists in the branch.
Repair the two stale-on-`main` gates (`ci_check_node_mode_closure.sh` — currently knows only
`{WireOnly, Admission}`; `ci_check_bootstrap_closure.sh` — expects the pre-A3b return shape). *Hermetic.*

### L2 — Mithril-only first-run bootstrap (live)
Wire `bootstrap_from_mithril_snapshot` as the **first production (non-test) caller**, over
`PersistentChainDb` + `FileWalStore`. **Real Mithril provenance is required** (operational prerequisite —
see below): `mithril-client cardano-db download` (certified, genesis-key-verified) → restore peer →
cardano-cli `query utxo` + consensus-input extraction **bound to that certified state** → operator seed
point → `verify_mithril_binding` (fail-closed, before any state admitted) → bootstrap → persist sidecar +
WAL provenance → one anchor lineage. If the Mithril certificate is absent or `verify_mithril_binding`
fails: **fail closed**. *Live leg gated on the Mithril operational prerequisite.*

### L3 — Production warm-start recovery (BUILD, not wire)
**Not "wire `recover_node_state`."** `recover_node_state` deliberately passes `NotRequired` and ignores the
recovered provenance; it stays a test/capability surface unless a later slice explicitly changes it. L3
**builds** the production warm-start path:

```
open PersistentChainDb + FileWalStore
  → WalStore::read_all → replay_from_anchor(anchor_fp, entries, block_bytes)
  → ReplayOutcome.recovered_provenance : RecoveredBootstrapProvenance
  → bootstrap_initial_state(RequiredFromRecoveredProvenance(provenance))
       → restore_seed_epoch_consensus_inputs (fail-closed: sidecar-present, hash, A1 decode,
         anchor/epoch binding, byte-identity re-encode; NO bundle fallback)
  → recovered BootstrapState { ledger, chain_dep, tip, seed_epoch_consensus_inputs: Some(..) }
```

Reuses the BLUE replay + the existing `restore_*` verify chain; the production wiring that threads
provenance through is new. Flips DC-CINPUT-01 partial→enforced. *Hermetic (committed small fixture).*

### L4 — Peer BlockFetch → durable validated apply (split; not hidden under "sync")
The two mature halves exist but were never connected: `admission::wire_pump` fetches real peer blocks but
only admits+WALs+derives-verdict (no persist, no recoverable tip); `forward_sync::pump_block` does durable
validated apply but has no fetcher. L4 connects them. **Admission must not remain only a verdict/comparison
engine for the Ade node lifecycle; `ade_core_interop::follow` is not validating sync; no "sync-to-tip" is
claimed until L4a+L4b+L4c are connected and green.**

- **L4a — Block-fetch source abstraction.** Extract a peer-block source from `admission::wire_pump` (the
  N2N `BlockFetch` `RequestRange`→`Block{bytes}` stream) behind a closed interface yielding ordered block
  bytes, decoupled from admission's verdict loop.
- **L4b — Feed fetched blocks into `forward_sync::pump_block`.** Drive `pump_block` (validated apply →
  `PersistentChainDb` block bytes + WAL `AdmitBlock`, durable-before-tip) from the L4a source. First
  production caller of `pump_block`.
- **L4c — Selected-tip recovery proof.** Prove the advanced tip is a **recoverable** selected tip: kill
  after L4b advances, warm-start (L3) recovers byte-identically to the same tip — the join point between
  sync and recovery.

*L4a/L4b hermetic against committed block fixtures; L4c hermetic crash/restart test.*

### L5 — Produce from recovered selected tip + recovered consensus inputs
Forge base built from the **recovered selected tip** (L4c) + **recovered `SeedEpochConsensusInputs`** (L3)
via `PoolDistrView::from_seed_epoch_consensus_inputs`; feed `run_real_forge` (reused engine). Delete the
cold `InMemoryChainDb` / `--consensus-inputs-path` forge base from the canonical node lifecycle path. CI:
the forge path references the recovered surface only; no shape-swap populator anywhere (CN-CINPUT-03);
producer consumes the recovered surface (DC-CINPUT-02b). *Hermetic.*

### L6 — Peer-acceptance evidence
Closed manifest correlating forged-block-hash ↔ Haskell-peer accept log. Synthetic dry-run is mechanical;
the live peer-accept run is gated on the Mithril operational prerequisite + operator green-light. No BA-02
claim until a real peer accepts. *Live leg gated.*

## Exit criteria (mechanical, CI-verifiable)
New test/check names are candidate (created by the owning slice); existing artifacts named as-is.

- **CE-L-1** — `--mode node` owner exists; mode set stays closed (`ci_check_node_mode_closure.sh`, repaired);
  owner obtains initial state solely via `bootstrap_initial_state` (`ci_check_bootstrap_closure.sh`,
  repaired; candidate `ci_check_lifecycle_owner_uses_bootstrap_initial_state.sh`); first-run-vs-warm-start
  branch is a pure function of on-disk state.
- **CE-L-2** — first production caller of `bootstrap_from_mithril_snapshot` over `PersistentChainDb` +
  `FileWalStore`; `verify_mithril_binding` runs before any state is admitted; fail-closed with no
  genesis/bundle/cold substitute; sidecar + WAL provenance persisted; one anchor lineage. *(Live binding
  exercised under the Mithril operational prerequisite; hermetic fixture proves the fail-closed wiring.)*
- **CE-L-3** — production warm-start: `replay_from_anchor` → `RequiredFromRecoveredProvenance` →
  recovered `BootstrapState` byte-identical; fail-closed on missing sidecar / missing WAL / hash mismatch /
  anchor mismatch / duplicate provenance; no bundle fallback. Flips DC-CINPUT-01 → enforced.
- **CE-L-4** — L4a source yields ordered peer block bytes; L4b drives `pump_block` (first production
  caller) persisting block bytes + WAL `AdmitBlock` durable-before-tip; L4c proves kill→warm-start
  recovers the same selected tip byte-identically.
- **CE-L-5** — node-lifecycle forge base derives from recovered tip + recovered `SeedEpochConsensusInputs`
  via the A4 projection; no `InMemoryChainDb`/`--consensus-inputs-path` forge base on the lifecycle path;
  CN-CINPUT-03 + DC-CINPUT-02b enforced.
- **CE-L-6** — BA-02 manifest schema + synthetic dry-run mechanical. *(Live peer-accept = gated obligation
  on the RO-LIVE family, NOT a mechanical CE.)*

## TCB color map
- **BLUE (reuse only — no new authority):** `ade_ledger::{seed_consensus_inputs, wal, consensus_view,
  producer::*, block_validity, receive}`, `ade_core::consensus::{leader_check, vrf_cert, leader_schedule}`.
- **GREEN:** first-run-vs-warm-start branch (pure over on-disk state); lifecycle sequencing reducer
  (`ade_runtime`, mirroring `forward_sync::reducer`).
- **RED:** the `--mode node` owner/driver (opens `PersistentChainDb`/`FileWalStore`, anchor mint/bind,
  recovery, the L4a fetch source, slot loop, peer I/O), the C6 evidence correlator.
- **CI:** `ci_check_consensus_input_provenance.sh` (extended for CN-CINPUT-03), candidate
  `ci_check_lifecycle_owner_uses_bootstrap_initial_state.sh`, repaired mode-closure + bootstrap-closure.

## Forbidden during this cluster
- Any first-run bootstrap without verified Mithril provenance (`verify_mithril_binding` must succeed).
- cardano-cli-extracted state used as first-run seed unless bound to Mithril-certified/restored state.
- Genesis branch / `--consensus-inputs-path` / tip-bundle / cold-`produce_mode` as a first-run or recovery
  fallback.
- A second bootstrap/recovery/storage-init authority (CN-NODE-01).
- Shape-swap of an operator bundle into `SeedEpochConsensusInputs`.
- Leaving `admission` as only a verdict/comparison engine for the node lifecycle, or treating
  `ade_core_interop::follow` as validating sync.
- Claiming "sync-to-tip" before L4a+L4b+L4c are connected; claiming BA-02 before a real peer accept;
  claiming `recover_node_state`/`run_node_until_shutdown` is production-wired unless a slice wires it.
- No new BLUE authority/type; no `HashMap`/clock/float/async in BLUE.

## Replay obligations (scoped)
No new canonical type / no new authoritative transition (reuse only). New scoped fixtures: L3
`persist→recover` byte-identity; L4c `sync→kill→recover` tip identity; L5 `produce-from-recovered`
byte-identity. Acceptance scoped to touched crates (`ade_runtime`, `ade_node`, specific `ade_testkit`
harness tests) — **not** the full `ade_testkit` corpus/oracle lane (times out ~600s on clean HEAD).

## Registry impact (at close)
Promote `DC-CINPUT-02b` (L5) + `CN-CINPUT-03` (L5). Strengthen `CN-NODE-01`, `CN-PROD-03`, `DC-CINPUT-01`
(→enforced), `DC-CINPUT-02a`, `CN-STORE-02`, `T-REC-01/02`, `DC-WAL-03`, `DC-FORGE-01`, `DC-SYNC-01` (L4b
first production driver). BA-02 stays an operator-gated RO-LIVE obligation.

## Operational prerequisite (live L2 / L6 — required, not optional)
The live L2 first-run bootstrap and the L6 peer-accept evidence pass REQUIRE a real Mithril-certified
artifact; this is an operational prerequisite for those legs, not a decision to proceed without it. Verified
facts (2026-05-30): `mithril-client` 0.13.9 installs as a PGP-verified prebuilt binary; a real preprod
cardano-db snapshot exists (epoch 291, immutable_file_number 5758, cert `b6b37700…`, ~18GB); the genesis
verification key MUST be the one fetched from the mithril release-preprod config (the
`ci/mithril_restore_preprod_peer.sh` hardcoded key is stale and fails cert-chain verify). Producing the
artifact = certified download → peer restore → cardano-cli extraction. Hermetic slices (L1, L3, L4, L5, the
L2 fail-closed wiring) proceed without it; the live L2 binding and L6 accept do not.

## Non-goals
No new BLUE authority. No native Mithril UTXO-HD/LedgerDB byte decode (Tier-4). No genesis full replay /
Conway-genesis path. No cross-epoch production. No forged-block durability beyond N-U. No BA-01/03/04/09
claim. No live run without the Mithril prerequisite + operator green-light. No grounding-doc regeneration
(that's `/cluster-close`).
