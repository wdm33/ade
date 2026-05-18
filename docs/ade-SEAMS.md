# Seams — Where New Work Can Attach (Ade)

> **Status:** Living architectural document. Regenerated; not hand-edited.
> Per-project instance of `~/.claude/methodology/templates/seams.md`.

> 9 crates, 182 canonical types, 838 tests, 16 CI checks at HEAD (`3eddcbb`).
> Reads CODEMAP for the module list and TCB colors; reads the invariant
> registry (`docs/ade-invariant-registry.toml`, 147 rules) for rule IDs;
> reads the Phase 4 cluster plan (`docs/active/phase_4_cluster_plan.md`)
> and the cluster N-D slice docs for closed-surface invariants.

Ade is a Cardano block-producing node. Its closure surface is dominated
by two facts:

1. The Cardano protocol fixes wire bytes and hashes for hash-critical
   paths (Tier 1 — must-conform). New work that touches those bytes
   has essentially no degrees of freedom.
2. Everything operator-facing — storage layout, query API, telemetry,
   packaging — is Tier 5: deliberate divergence "in our own image"
   (per `docs/active/CE-79_tier5_addendum.md`).

This document names where the system opens and where it stays closed.

---

## 1. Surface Reduction Rules

> External inputs reduce to canonical form before entering authoritative
> pipelines. At HEAD the only fully-wired ingress is block bytes; the
> Phase 4 plan adds five further surfaces.

### Surface: Block bytes (wired today)

```
Surface: Block bytes (file/stream/network — caller-supplied)
Reduces to: BlockEnvelope { era: CardanoEra, era_block: PreservedCbor<EraBlock> }
            (BlockEnvelope is defined in `ade_codec::cbor::envelope`;
             EraBlock is one of the seven era-tagged decoded blocks)
Pipeline (fixed step ordering — no reorder, no shortcut):
  1. decode_block_envelope(&[u8]) -> BlockEnvelope
     (era tag dispatch; the only constructor of PreservedCbor for blocks)
  2. era-specific decode_{byron,shelley,allegra,mary,alonzo,babbage,conway}_block
     (closed set — 7 chokepoints, named in `ci_check_ingress_chokepoints.sh`)
  3. ade_ledger::rules::apply_block_with_verdicts(state, &PreservedCbor<EraBlock>, ctx)
     (BLUE — single canonical chokepoint that produces BlockVerdict + new state)
Cross-surface state sharing: none today (Phase 3 was an offline oracle).
  Phase 4 introduces shared state between this surface and the network
  ingress surface (next subsection) via `ade_runtime::chaindb`.
```

**Rule.** New ingress that produces block bytes (e.g., the upcoming
N-A `block-fetch` mini-protocol, N-D recovery replay, N-F
`local-tx-monitor`) **MUST** enter through `decode_block_envelope` and
flow through one of the seven era-specific decoders before reaching any
ledger code. The pipeline cannot be reordered: hash-bearing bytes must
be preserved via `PreservedCbor` before they reach ledger rules
(enforced by `ci_check_hash_uses_wire_bytes.sh`,
`ci_check_ingress_chokepoints.sh`).

### Surface: Snapshot bytes (wired in N-D)

```
Surface: Snapshot bytes (disk — written and read by the node itself)
Reduces to: Recoverable::decode_snapshot(&[u8]) -> R  (caller-supplied)
Pipeline:
  1. SnapshotStore::latest_snapshot() -> Option<(SlotNo, Vec<u8>)>
  2. Recoverable::decode_snapshot(bytes) -> R       (caller's impl)
  3. for block in ChainDb::iter_from_slot(slot+1):
       R::apply_block(&block.bytes) -> R            (caller's impl)
Cross-surface state sharing: `ade_runtime` is intentionally bytes-in /
  bytes-out — it never touches the ledger state type directly. The
  shared state lives at the caller (eventually `ade_node`).
```

**Rule.** The recovery primitive (`ade_runtime::recovery::recover`) is
the **single** path from on-disk state to in-memory state. It does not
import `ade_ledger`. Any callsite that wants to recover a ledger state
must provide a `Recoverable` impl; there is no second public path
through `ade_runtime`.

### Candidates — surfaces not yet wired (Phase 4 N-A, N-C, N-E, N-F)

The following surfaces are named in the Phase 4 cluster plan but have
no source today. They are listed so future slice docs can attach
without reinventing the reduction step. **Each is a candidate seam
pending confirmation at cluster entry.**

| Cluster | Surface | Expected reduction target | Expected chokepoint | Confidence |
|---------|---------|---------------------------|---------------------|------------|
| N-A | N2N `chain-sync` / `block-fetch` wire frames | `BlockEnvelope` then ledger apply (same as block-bytes surface) | `ade_runtime::network::decode_chain_sync_msg` (proposed) | candidate |
| N-A | N2N `tx-submission2` frames | Per-era tx body (the `*TxBody` types in `ade_types`) | `ade_runtime::network::decode_tx_submission_msg` (proposed) | candidate |
| N-A | N2C `local-state-query` requests | Internal Query enum (closed, not yet defined) | Single dispatch fn in `ade_runtime::query` (proposed) | candidate |
| N-C | Forge-block inputs (mempool + state + slot + KES + VRF) | `BlockEnvelope` bytes (forged, then re-decoded for validation) | `ade_runtime::forge::forge_block` (proposed) | candidate |
| N-E | Mempool tx ingest (from network OR local IPC) | Per-era tx body (canonical bytes preserved) | `ade_runtime::mempool::ingest_tx` (proposed) | candidate |
| N-F | gRPC / HTTP query API | Same internal Query enum as N-A LSQ | Shared dispatch with N-A — Tier 5 wire, Tier 1 semantics | candidate |

These candidates need user confirmation when each cluster is opened:
"Is the canonical reduction target named above the right one? Does the
chokepoint name fit the project's emerging naming convention?"

---

## 2. Data-Only vs. Authoritative Layers

Ade has three authoritative domains. For each, a single BLUE chokepoint
holds enforcement authority; tooling layers (when they exist) live in
GREEN (`ade_testkit`) or RED (`ade_runtime`).

### Ledger application

| Layer | Module | Color | Role |
|-------|--------|-------|------|
| **Data-only tooling** | `ade_codec` | BLUE\* | Decodes block / tx / cert bytes into typed values, preserves wire bytes via `PreservedCbor`. **Never interprets ledger semantics.** |
| **Authoritative enforcement** | `ade_ledger` | BLUE | `rules::apply_block_with_verdicts` is the single chokepoint that produces `BlockVerdict` + new `LedgerState`. |
| **Loader** | `ade_runtime::chaindb` + `ade_runtime::recovery` | RED | Reads block / snapshot bytes from disk; feeds them through caller-supplied `Recoverable` impl into ledger. |

\* `ade_codec` is BLUE-data-only: it builds typed shapes but never
asserts a transition is valid. The semantic split between "this is
what the bytes say" (codec) and "this is whether the bytes are
allowed" (ledger) is the project's central design rib.

**Rule.** New work that touches ledger transitions adds enforcement
inside `ade_ledger` (typically a new composer step, or a tightening of
`apply_block_with_verdicts` / `apply_epoch_boundary_full`). New work
that touches block / tx CBOR adds parse / pack support inside
`ade_codec` only. **The compilation chokepoint
(`apply_block_with_verdicts`) never moves.**

### Plutus phase-2 evaluation

| Layer | Module | Color | Role |
|-------|--------|-------|------|
| **Data-only tooling** | `ade_plutus::cost_model`, `ade_plutus::script_context` | BLUE | Decodes cost-model CBOR; builds the V1/V2/V3 `ScriptContext`. Does not run programs. |
| **Authoritative enforcement** | `ade_plutus::tx_eval::eval_tx_phase_two` | BLUE | Single entry to phase-two evaluation. Internally wraps the aiken `uplc` machine; aiken types do not leak (enforced by `ci_check_pallas_quarantine.sh`). |
| **Quarantine** | (the `aiken_uplc` git dep, pinned tag `v1.1.21` commit `42babe5d`) | external | Frozen at tag — never re-exported. PV11 builtins gated off (S-29). |

**Rule.** Adding a new Plutus version, builtin, or cost-model entry
requires a registry diff (see §3) plus a pinned-version bump of
`aiken_uplc`; the chokepoint `eval_tx_phase_two` does not move. No
second public entry into the evaluator is allowed; tests use the same
entry as production callers.

### Governance ratification / enactment (Conway)

| Layer | Module | Color | Role |
|-------|--------|-------|------|
| **Data-only tooling** | `ade_types::conway` (governance types) | BLUE | Holds `GovAction`, `GovActionState`, `DRep`, `Anchor`, `VotingProcedures` shapes. |
| **Authoritative enforcement** | `ade_ledger::governance::{evaluate_ratification, enact_proposals, expire_proposals}` | BLUE | The three chokepoints that compute Conway ratification outcomes. |

**Rule.** A new governance action variant (CIP-1694 extension) adds a
variant to `GovAction` (§3 closed registry — version-gated) **and**
arms in all three chokepoints. The CI check
`ci_check_constitution_coverage.sh` enforces the invariant-registry ↔
code coverage for governance rules.

### Where the boundary is enforced

- `ci_check_dependency_boundary.sh` — no BLUE crate may depend on
  `ade_runtime` or `ade_node`.
- `ci_check_pallas_quarantine.sh` — only `ade_plutus` may name
  `pallas_*`.
- `ci_check_no_signing_in_blue.sh` — signing patterns
  (`SigningKey`/`sign_message`/etc.) forbidden in BLUE; only
  `ade_runtime` may sign.
- `ci_check_ingress_chokepoints.sh` — `PreservedCbor::new` constructed
  only by the 7 named `decode_*_block` chokepoints.

---

## 3. Closed vs. Extensible Registries

Ade's authority surface is **almost entirely closed.** This is a
consequence of being a chain-compatibility implementation: the
protocol fixes most variants. The few extensible surfaces are
operator-config or testkit-only.

### Closed (frozen — version-gated changes only)

| Registry | Location | Count | Change Rule |
|----------|----------|-------|-------------|
| `CardanoEra` | `ade_types::era` | 8 variants (ByronEbb, ByronRegular, Shelley, Allegra, Mary, Alonzo, Babbage, Conway) | New variant = new hard fork. Requires a coordinated change across `ade_codec` (new era's `decode_*_block` chokepoint), `ade_ledger` (new era composer + hfc translation), and the canonical type list. Comment in source explicitly says "this enum is closed — unknown era tags produce a `CodecError`, never a fallback variant." |
| `Certificate` | `ade_types::shelley::cert` | 7 variants (StakeRegistration, StakeDeregistration, StakeDelegation, PoolRegistration, PoolRetirement, GenesisKeyDelegation, MoveInstantaneousRewards) | Frozen Shelley-era certificate set. New cert types live in `ConwayCert`. |
| `ConwayCert` | `ade_types::conway::cert` | N variants (Conway-era certificates) | Version-gated per protocol — extends but does not modify `Certificate`. |
| `GovAction` | `ade_types::conway::governance` | 7 variants (ParameterChange, HardForkInitiation, TreasuryWithdrawals, NoConfidence, UpdateCommittee, NewConstitution, InfoAction) | CIP-1694 fixed; new variant = CIP amendment + ratification chokepoint update. |
| `MIRPot` | `ade_types::shelley::cert` | 2 variants (Reserves, Treasury) | Frozen. |
| `DRep` | `ade_types::conway::cert` | 4 variants (KeyHash, ScriptHash, AlwaysAbstain, AlwaysNoConfidence) | CIP-1694 fixed. |
| `PlutusLanguage` | `ade_plutus::evaluator` | 3 variants (V1, V2, V3) | New variant = new Plutus version. Requires cost-model table extension + aiken bump. PV11 builtins gated off (S-29) — they cannot be activated without a documented gate flip. |
| `Datum` / `DatumOption` | `ade_types::alonzo::plutus`, `ade_types::babbage::output` | Closed shapes — Datum hash vs. inline | Schema frozen at Babbage. |
| `NativeScript` | `ade_types::allegra::script` | Shelley/Allegra/Mary native script variants | Frozen. |
| **Per-era decode chokepoints (`BlockEnvelope` consumers)** | `ade_codec::{byron,shelley,allegra,mary,alonzo,babbage,conway}::decode_*_block` | 7 (one per non-EBB era + `decode_byron_block` for both Byron variants) | New era = new chokepoint added in lockstep with a `CardanoEra` variant; `ci_check_ingress_chokepoints.sh` enumerates them. |
| **`PreservedCbor::new` constructor** | `ade_codec::preserved` | 1 chokepoint, `pub(crate)` | Construction lives inside `ade_codec`. Adding a new `PreservedCbor<T>` requires either (a) extending an existing decoder or (b) adding a new named decoder added to the `ci_check_ingress_chokepoints.sh` allowlist. |
| **`ChainDb` trait surface** | `ade_runtime::chaindb::mod` | 6 methods (`put_block`, `get_block_by_hash`, `get_block_by_slot`, `tip`, `iter_from_slot`, `rollback_to_slot`) | Object-safe; intended for multiple impls (in-memory + redb at HEAD; future: sharded / network-backed). **Surface is closed** — new method = new contract test in `chaindb::contract`; method removal forbidden. |
| **`SnapshotStore` trait surface** | `ade_runtime::chaindb::mod` | 5 methods (`put_snapshot`, `get_snapshot`, `latest_snapshot`, `list_snapshot_slots`, `delete_snapshot`) | Same closure discipline as `ChainDb`. Bytes are opaque at this layer (S-35). |
| **`Recoverable` trait surface** | `ade_runtime::recovery` | 2 methods (`decode_snapshot`, `apply_block`) + 1 associated type (`Error`) | Caller-supplied. Trait deliberately commits to a single error type per impl; multi-error callers wrap. |
| **`recover` entry point** | `ade_runtime::recovery::recover` | 1 free function | The sole composition of `ChainDb` + `SnapshotStore` + `Recoverable` into a recovery sequence. Async wrappers (if added) layer above; recovery itself stays sync (S-36). |
| **Hash domain functions** | `ade_crypto::blake2b::{block_header_hash, transaction_id, script_hash, credential_hash}` | 4 named domains | Algorithm immutable per protocol version; new domain = new function (not a parameter to an existing one). |
| **CI check set** | `ci/ci_check_*.sh` | 16 scripts | Existing checks may be tightened, never relaxed. New CI check is **additive**. Deleting a CI script requires recording the deprecation in the invariant registry's `ci_scripts` arrays. |
| **Invariant registry families** | `docs/ade-invariant-registry.toml` | 5 family letters (T = true, CN = constraint network, DC = derived constraint, OP = operational, RO = release obligation) | Append-only IDs; rules may be strengthened, never weakened; deprecation needs an explicit `deprecated_in`. |

### Extensible (open within constraints)

| Registry | Location | Extension Rule |
|----------|----------|---------------|
| `CostModels` map (Plutus V1/V2/V3 cost tables) | `ade_plutus::cost_model::CostModels` | New entries enter via the cost-model CBOR decoder when a protocol parameter update lands. Decoder mode (`DecoderMode`) controls strict vs. permissive parsing — strict is the default. Not runtime-pluggable; constrained by the closed `PlutusLanguage` set. |
| `ProtocolParameters` / `ProtocolParameterUpdate` field set | `ade_ledger::pparams` | Fields are appended per era. Conway's `pparams_update` is the largest. New field = new struct field + new arm in any update-applying chokepoint. Not extensible at runtime — versioned-gated by era. |
| Pool registrations / DRep registrations / Stake registrations | `ade_ledger::state::{DelegationState, CertState}` | Mutated at runtime by `ade_ledger::delegation::apply_cert` (and certs in turn come from in-block data). The **shape** of what can be registered is closed (see `Certificate` / `ConwayCert` above); the **set** of registrations is open and grows monotonically (within deregister rules). |
| Governance proposal set | `ade_ledger::state::ConwayGovState::proposals` | Same pattern — shape closed (`GovAction` variants), instance set open, lifecycle managed by `evaluate_ratification` / `enact_proposals` / `expire_proposals`. |
| Oracle reference snapshots / regression corpus | `ade_testkit::harness::{snapshot_loader, regression_corpus}` | Tooling-only. New oracle data added via `corpus/` directory plus a manifest update. `ci_check_ref_provenance.sh` enforces manifest checksum integrity. GREEN — never feeds back into authoritative state. |
| `KillStrategy<D>` trait impls | `ade_runtime::chaindb::crash_safety` | RED-only test infrastructure. New strategies (e.g., latency-injecting, partial-disk) plug in via the trait; `NoKill` is the production default. Object-safe by intent. |
| Recovery state types | callers of `Recoverable` | Open: any state type with a canonical encode + apply-block step can be recovered. The trait is the only way in; no central registry of state types. |
| Pinned external crates (`redb`, `aiken_uplc`, `pallas-primitives`, `blake2`, `ed25519-dalek`, `cardano-crypto`) | `crates/*/Cargo.toml` | New external crate addition requires a Tier-5 rationale doc (per `docs/active/CE-79_tier5_addendum.md`) — Ade does not casually expand its dependency surface. |

### Candidates — extensible surfaces not yet wired

| Cluster | Candidate registry | Rationale |
|---------|-------------------|-----------|
| N-A | Network mini-protocol version table | Cardano's versioned handshake. Closed within a node release; extensible across releases. Needs a closed enum + a CI check that no in-flight version is dropped silently. |
| N-A | Peer address book | Operator-supplied; runtime mutable. Should live in `ade_runtime`. |
| N-E | Mempool tx prioritization policy | Tier 5 — operator-tunable. Plugin trait candidate: `MempoolPolicy`. |
| N-F | Query API method set | Tier 5 wire / Tier 1 semantics. Should be a closed enum internally, mapped to gRPC / HTTP at the edge. |
| N-F | Prometheus metric names | Tier 5; append-only registry expected (parallel to invariant registry's append-only discipline). |

User confirmation needed for each at cluster entry: closed enum vs.
trait-based registry; runtime-extensible vs. compile-time-fixed; CI
enforcement shape.

---

## 4. Version-Gated vs. Frozen Contracts

### Frozen (immutable at current version — change = new major version)

- **Cardano-canonical CBOR wire format**: Each `decode_*_block` in
  `ade_codec` produces values whose wire bytes are preserved
  byte-identically. Hash inputs are wire bytes, not re-encoded bytes
  (enforced by `ci_check_hash_uses_wire_bytes.sh`). The encoding is
  the protocol, not a library — there is no `postcard` / `bincode`
  pinning, only the project's own readers/writers in `ade_codec::cbor`.
- **Block envelope shape**: `[era_tag:u8, era_block:CBOR]` as a
  definite-length 2-element CBOR array; era tags 0..=7 (closed). Adding
  era 8 is a hard fork.
- **`PreservedCbor<T>` invariant**: `wire_bytes()` is exactly what the
  decoder consumed, byte-identical. Re-encoding is permitted only via
  `canonical_bytes(ctx)` and never used for hashing.
- **Hash algorithms**: Blake2b-224 for credential / script hashes,
  Blake2b-256 for block / transaction / Merkle hashes. Algorithm is
  protocol-fixed. Ed25519, Byron-bootstrap (extended Ed25519), KES-sum,
  VRF-draft-03 — all wired in `ade_crypto`, all protocol-frozen.
- **Plutus language set**: V1, V2, V3. PV11 builtins (`ExpModInteger`,
  `CaseList`, `CaseData`) deliberately gated off to match mainnet's
  unactivated state — see S-29.
- **Aiken UPLC quarantine pin**: `aiken_uplc` (git dep) at tag
  `v1.1.21`, commit `42babe5d`. Bump = explicit slice with a Plutus
  conformance run.
- **All 182 canonical types**: shapes are frozen at the era / version
  they entered. Adding fields requires a versioned gate; renaming is
  forbidden.
- **TCB color assignments**: Per `.idd-config.json` `core_paths`. BLUE
  ↔ RED separation is mechanical (`ci_check_dependency_boundary.sh`).
- **`ChainDb` / `SnapshotStore` / `Recoverable` trait shapes** (N-D, in
  flight): once N-D closes, the trait method set is frozen. Adding a
  method = new slice with a contract-test extension.

### Version-gated (can evolve across major versions)

- **New `CardanoEra` variant**: requires new `decode_*_block`
  chokepoint, new per-era composer in `ade_ledger`, new hfc
  translation arm, new addition to `CardanoEra::ALL`.
- **New `GovAction` / `ConwayCert` / Plutus version variant**:
  requires registry diff (§3) plus arms in every chokepoint.
- **New protocol parameter field**: append to `ProtocolParameters`;
  CBOR field-order discipline preserved by `ade_codec`.
- **New CI check**: additive. Removing an existing check requires
  invariant-registry deprecation note.
- **Pinned external crate bump**: Tier-5 rationale doc required;
  cross-references invariant-registry `O-29.*` family (pallas /
  aiken quarantine).
- **Phase-4 cluster surface additions** (N-A through N-F): each
  cluster's wire surface gates additions via its own cluster doc;
  pipeline steps added there cannot collide with existing chokepoints.

---

## 5. Module Addition Rules

Ade's workspace is small and color-disciplined. New modules enter as
new crates under `crates/`.

| Color | Naming convention | Build-config flags | May depend on | MUST NOT depend on |
|-------|-------------------|--------------------|----------------|--------------------|
| **BLUE** | `ade_*` (no color in name) | First line of every `.rs` is the contract banner `// Core Contract:`. `lib.rs` carries `#![deny(unsafe_code)]`, `#![deny(clippy::unwrap_used/expect_used/panic/float_arithmetic)]`. No `#[cfg(feature = ...)]` (semantic gates forbidden). | Other BLUE crates only | Any RED (`ade_runtime`, `ade_node`); GREEN (`ade_testkit`) in non-dev deps; `pallas_*` (except `ade_plutus`); async runtime; `HashMap`/`HashSet`/`IndexMap`; clock/rand/float/env/I/O. |
| **GREEN** | `ade_*` | Banner + deny attrs are project convention but not currently enforced for `ade_testkit`. May use `HashMap`/`serde_json`/`flate2`/`tar` for fixture I/O. | BLUE crates + standard library + ecosystem crates (serde/toml/flate2/tar) | `ade_runtime`, `ade_node`. Results must never feed back into BLUE state. |
| **RED** | `ade_*` | No special header. Free to use clocks, I/O, async, `HashMap`, signing keys (`ade_runtime` is the only crate that may sign). | Any BLUE / GREEN crate (one-way) | Cannot be depended on by BLUE (`ci_check_dependency_boundary.sh`). |

### New module checklist

1. **Add to `Cargo.toml` workspace members.** `version = "0.1.0"`,
   `edition = "2021"`.
2. **Declare TCB color** by editing `.idd-config.json` `core_paths` if
   BLUE, or by documenting GREEN / RED placement (the doc string
   `_core_paths_doc` is the authoritative classifier for GREEN / RED).
3. **CI script update obligations:**
   - BLUE: add the crate path to `ci_check_forbidden_patterns.sh` and
     `ci_check_dependency_boundary.sh`. **Decide whether the new crate
     joins the narrow BLUE list** (`ci_check_module_headers.sh`,
     `ci_check_no_semantic_cfg.sh`, `ci_check_no_signing_in_blue.sh`,
     `ci_check_hash_uses_wire_bytes.sh`, `ci_check_ingress_chokepoints.sh`)
     or stays only in the wider list. **The current drift between
     these lists is a known gap** (see CODEMAP §Gap surfaced) — new
     BLUE crates should default to the wider list and explicitly opt
     into narrower scripts as needed.
   - RED: add to `ci_check_dependency_boundary.sh` as a "BLUE-cannot-depend-on"
     entry. Verify no BLUE crate's `Cargo.toml` lists the new crate.
   - GREEN: no script change required; runtime tests cover it.
4. **Add contract banner** (BLUE) to every `.rs` file.
5. **Add deny attributes** to `lib.rs` (BLUE).
6. **New canonical types:** at HEAD the canonical-type registry is
   inline in the invariant registry (`canonical_type_registry: null`
   in `.idd-config.json`) — add a `[[rules]]` block under family `T`
   with `tier = "true"` for each new type, plus a round-trip test
   referenced in the rule's `tests` array.
7. **Run `cargo test --workspace` and the full CI script suite.** Both
   must be green before the cluster can close.

### Phase 4 anticipated additions

Cluster N-A introduces network code that will likely become its own
RED crate (`ade_network` or similar). N-E's mempool likely lives in
`ade_runtime` (RED) but its tx-validation entry must call into
`ade_ledger`. N-F's query API surface likely becomes a thin RED layer
mapping a closed Query enum to multiple wire encodings. **These are
candidate placements** — user confirmation needed at cluster entry.

---

## 6. Forbidden Patterns (per color)

### BLUE (universal IDD prohibitions; enforced by CI where marked)

- No `HashMap`, `HashSet`, `IndexMap`, `IndexSet`, `indexmap::*` —
  `ci_check_forbidden_patterns.sh`.
- No `SystemTime`, `Instant`, `std::time::*` clocks —
  `ci_check_forbidden_patterns.sh`.
- No `rand::thread_rng`, `thread::spawn` —
  `ci_check_forbidden_patterns.sh`.
- No `f32`, `f64`, floating-point arithmetic — enforced by
  `#![deny(clippy::float_arithmetic)]` plus the pattern script.
- No `std::fs`, `std::net`, `tokio`, `async fn` —
  `ci_check_forbidden_patterns.sh`.
- No `anyhow` (errors are structured); `unwrap`/`expect`/`panic` denied
  at the lint level.
- No `unsafe` outside an explicit allowlist (currently only
  `ade_crypto::vrf`'s FFI binding).
- No `#[cfg(feature = ...)]` semantic gating —
  `ci_check_no_semantic_cfg.sh` (narrow BLUE list).
- No signing patterns (`SigningKey`/`SecretKey`/`PrivateKey`/
  `sign_message`/`sign_block`) — `ci_check_no_signing_in_blue.sh`
  (narrow BLUE list).
- No re-hashing of `canonical_bytes` or re-encoded bytes — wire bytes
  only. `ci_check_hash_uses_wire_bytes.sh` (narrow BLUE list).
- No construction of `PreservedCbor` outside the 7 named
  `decode_*_block` chokepoints — `ci_check_ingress_chokepoints.sh`.
- No `pallas_*` reference outside `ade_plutus` —
  `ci_check_pallas_quarantine.sh`.

### GREEN (`ade_testkit`)

- No nondeterminism that leaks into stored fixtures — fixtures must be
  byte-reproducible.
- No participation in authoritative outputs — `ade_testkit` results
  must never feed back into `ade_ledger`/`ade_codec`/`ade_crypto`
  state.
- No import of `ade_runtime` (preserves the GREEN-not-RED stance).
- No inbound dep from any RED crate.

### RED (`ade_runtime`, `ade_node`)

- No direct mutation of `ade_ledger` state — all transitions go
  through `ade_ledger::rules::*`.
- No bypassing `ade_codec` to construct semantic types from raw bytes.
- (`ade_runtime` specifically) No dep on `ade_ledger` — bytes-in /
  bytes-out only. The decoupling is what allows `Recoverable` to be
  caller-supplied (S-36 invariant).
- (`ade_runtime` specifically) No leakage of `redb` types through the
  `chaindb::*` public surface (S-34 invariant).
- No second public `chaindb` path — the trait is the only surface.
- No automatic snapshot pruning — operator-driven only (S-35, S-36).
- No partial-recovery success — mid-replay failure aborts (S-36).
- No async recovery surface — sync only; callers wrap if cancellation
  is needed (S-36).

### Project-specific additions

- **No commits of credentials, hostnames, IPs, private keys** — public
  repo discipline; enforced by `ci_check_no_secrets.sh`.
- **No `Phase 4 internal-mode mock network`** — Tier 1 surfaces must
  be exercised against real cardano-node peers (Phase 4 cluster plan
  §Forbidden patterns).
- **No collapsing wire and canonical bytes** — dual-authority rule
  carries forward from Phases 1-3.
- **No Tier 5 surface without a stated rationale** — divergence from
  cardano-node requires naming "what's better" per
  `docs/active/CE-79_tier5_addendum.md`.
- **No "we'll match it later" stubs on Tier 1 surfaces** — Tier 1
  closure is hard-gated.

---

## Cross-references

- CODEMAP: `docs/ade-CODEMAP.md` — module-by-module authority table,
  upstream of this document.
- Invariant registry: `docs/ade-invariant-registry.toml` — 147 rules
  across families T / CN / DC / OP / RO. The registry's `tests` and
  `ci_scripts` arrays are the authoritative cross-reference for
  enforcement.
- Phase 4 cluster plan: `docs/active/phase_4_cluster_plan.md` — names
  the seven Phase 4 clusters and their tier classifications.
- Tier doctrine: `docs/active/CE-79_gate_statement.md` and
  `docs/active/CE-79_tier5_addendum.md`.
- Cluster N-D slices (in-flight): `docs/clusters/PHASE4-N-D/S-{33..37}.md`
  — own the closed-surface invariants for `ChainDb`, `SnapshotStore`,
  `Recoverable`, and `recover`.
