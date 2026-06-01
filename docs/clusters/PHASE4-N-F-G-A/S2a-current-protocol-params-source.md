# PHASE4-N-F-G-A — Slice S2a: Current protocol parameters source

> **Status:** slice doc (IDD Part IV). Inserted 2026-06-01 after the S2 PO-1 entry check proved the
> recovered ledger carries the stale default `protocol_major = 2`. Companion to `cluster.md` (S2a
> row + CE-G-A-2a). Code-verified against HEAD `225e61d9`.
>
> **Slice S2a in one line:** install the **oracle-captured current `ProtocolParameters`** into the
> recovered ledger at seed/import time — carried in the consensus-inputs bundle (which already
> commits to them via `protocol_params_hash`) and set into `LedgerState.protocol_params` at
> `build_seed_ledger` + the bootstrap runner ledger — so `recovered.ledger.protocol_params` is a
> **truthful current-state source**, never `ProtocolParameters::default()` and never genesis-initial.

## 1. Slice identity
- **Cluster:** PHASE4-N-F-G-A (forge fidelity). Gated behind S1 (green); **gates S2** (S2 may
  proceed only after S2a's PO-1 re-run passes).
- **Slice:** S2a — current protocol parameters source (Design A: the oracle seeds the starting
  state, then Ade owns it).
- **Modules:** **RED** `ade_runtime::consensus_inputs` (bundle carries current `ProtocolParameters`
  + provenance bind), `ade_node::admission::{seed_to_snapshot::build_seed_ledger, bootstrap}`
  (install into the recovered ledger), `ci/build_consensus_inputs_bundle.sh` (emit the
  already-queried params); consumed **BLUE** `ade_ledger::{state::LedgerState,
  pparams::ProtocolParameters, snapshot}`. **No new BLUE authority; single bootstrap (CN-NODE-01);
  no serve/live.**

## 2. Cluster Exit Criteria addressed (verbatim)
- **CE-G-A-2a** — current protocol params source (PO-1 split): the recovered ledger's
  `protocol_params` are oracle-captured current values installed at seed/import time, never
  `LedgerState::default()` / genesis-initial. Candidate tests:
  `seed_import_installs_current_protocol_params` (bundle carries pparams; `build_seed_ledger` +
  runner ledger install them, not defaults), `warm_start_recovery_preserves_protocol_params`
  (pre-seed → recover → `protocol_params` equal the fixture), `forge_call_site_sees_current_pparams`
  (`recovered.ledger.protocol_params` yields the expected `protocol_major`/`protocol_minor`; fails
  on the default 2.0), `no_default_or_genesis_initial_pparams_fallback`; candidate gate
  `ci_check_recovered_ledger_pparams_sourced.sh`. *(strengthens `DC-LEDGER-10`, `CN-NODE-01`,
  `DC-CINPUT-02b`.)* *(gates S2.)*

(CE-G-A-1 = S1 precondition; CE-G-A-2/3/4 = S2/S3/S4 — out of S2a scope.)

## 3. Intent (invariant impact)
Make the recovered ledger **tell the truth about the current protocol version/parameters.** Today
`build_seed_ledger` + the bootstrap runner ledger construct `LedgerState::new(Conway)` with
`ProtocolParameters::default()` (major 2) and overwrite only the UTxO — so the recovered ledger
silently lies, claiming the Shelley-launch version regardless of the real Conway-era network. S2a
closes that: the operator's oracle import bundle carries the **actual current `ProtocolParameters`**
(the bundle already queries `cardano-cli query protocol-parameters` and commits to it via
`protocol_params_hash`), and the recovered-ledger construction installs them — provenance-bound,
never defaulted/fabricated. This is the precondition S2's PO-1 requires: a recovered current view
that is a genuine source.

## 4. Pre-conditions (verified at HEAD `225e61d9`)
- **The defect (two install sites):**
  - `seed_to_snapshot::build_seed_ledger` (`seed_to_snapshot.rs:77-80`) = `LedgerState::new(Conway)`
    + `ledger.utxo_state = utxo` only — doc comment: *"all other fields at their canonical
    defaults."*
  - The bootstrap runner ledger (`bootstrap.rs:177-178`) = the same `LedgerState::new(Conway)` + UTxO.
  - `LedgerState::new` (`state.rs:111`) hardcodes `protocol_params: ProtocolParameters::default()`;
    default is `protocol_major: 2, protocol_minor: 0` (`pparams.rs:62,64,129`).
- **The oracle source already exists, only the hash is carried:** `ci/build_consensus_inputs_bundle.sh:70`
  runs `PROTO_PARAMS_JSON=$(run_cli query protocol-parameters)`; the bundle commits to it as
  `LiveConsensusInputsCanonical.protocol_params_hash: Hash32` (`canonical.rs:65`; JSON
  `protocol_params_hash_hex`, `json.rs:55`) — **a commitment, not the values.** The S1 fixture's
  `consensus-inputs.json` has `protocol_params_hash_hex: "783f2b42…"`.
- **The bundle is imported at bootstrap:** `import_live_consensus_inputs` (`canonical.rs:102`) at
  `bootstrap.rs:194`, alongside the UTxO seed (`import_cardano_cli_json_utxo`, `:120`) and
  `seed_to_snapshot` (`:142`). The bundle is anchor-bound (CN-CINPUT-02 / DC-CINPUT-01) and carries
  `fingerprint: Hash32` (`canonical.rs:70`).
- **Warm-start round-trips `protocol_params`:** the snapshot capture (`restart.rs:338`
  `.capture(slot, &ledger, …)`) and decode (`snapshot/ledger.rs:100` `decode_pparams`) carry
  `LedgerState.protocol_params` — so params installed at `build_seed_ledger` are preserved through
  warm-start recovery. (S2a must confirm the snapshot pparams codec round-trips the installed fields
  incl. `protocol_minor`; AC2 catches any gap.)
- **The forge reads it directly:** `ForgeActivation.recovered.ledger` (`node_lifecycle.rs:528`) →
  `ForgeRequestContext.base_state = &recovered.ledger` (`node_sync.rs:439`). Once
  `recovered.ledger.protocol_params` is current, S2 sources `protocol_version`/`pparams` from it.

## 5. The fix (Design A1 — extend the bundle the operator already imports)
1. **Bundle builder** (`build_consensus_inputs_bundle.sh`): emit the already-queried
   `PROTO_PARAMS_JSON` current protocol parameters into the bundle (alongside the existing
   `protocol_params_hash`).
2. **Bundle type + importer** (`LiveConsensusInputsCanonical` + `import_live_consensus_inputs` +
   `consensus_inputs/json.rs`): add a `protocol_params: ProtocolParameters` field, parsed from the
   bundle and **provenance-bound** to the existing `protocol_params_hash` (a tampered/fabricated
   params set fails the bind, fail-closed).
3. **Install sites:** thread the bundle's current `ProtocolParameters` into `build_seed_ledger` and
   the bootstrap runner ledger (`bootstrap.rs:177`) so `LedgerState.protocol_params` is the oracle
   value, never `default()`.
4. **Warm-start:** the captured snapshot now carries the real params → restored on recovery
   (existing codec); the forge then reads current `protocol_major`/`protocol_minor`.

## 6. TCB color (execution boundary)
- **RED (extended):** `ade_runtime::consensus_inputs::{canonical, importer, json}` (carry +
  provenance-bind current pparams); `ade_node::admission::{seed_to_snapshot::build_seed_ledger,
  bootstrap}` (install into the recovered ledger); `ci/build_consensus_inputs_bundle.sh` (emit
  oracle params). All RED import/shell — no authoritative transition added.
- **BLUE (consumed, unchanged):** `ade_ledger::{state::LedgerState, pparams::ProtocolParameters,
  snapshot::{ledger codec}}` — read/populated, not redefined. **No new BLUE authority.** Any change
  to BLUE-owned ledger semantics, `ProtocolParameters`, or `LedgerState` definitions is a red flag
  and must stop/re-scope; S2a should populate existing fields, not redefine them.
- **GREEN:** none new.

## 7. Invariants preserved (must not weaken) — by registry ID
- `CN-NODE-01` — **single bootstrap.** S2a EXTENDS the one seed import to carry current pparams; it
  adds no second bootstrap / Mithril call / parallel recovery.
- `CN-SEED-01` / `DC-SEED-01` — the single JSON UTxO importer + its fingerprint determinism are
  untouched (pparams ride the *consensus-inputs* bundle, not the UTxO seed importer).
- `CN-CINPUT-02` / `DC-CINPUT-01` — the bundle stays anchor-fp-bound; extending it re-fingerprints
  the bundle (the fixture is re-extracted), the anchor-binding discipline is preserved.
- `DC-COMPAT-01` — comparisons stay observable/derived (param values, hashes), never an
  internal-fingerprint-vs-Haskell-state-hash equality.
- `DC-NODE-05` / `DC-EPOCH-03` — forge-slot discipline / single-epoch fail-closed: untouched (S2a
  changes the recovered ledger's pparams source, not the loop/forge/epoch contract).
- All BLUE invariants — read-only consumption.

## 8. Invariants strengthened (one family: recovered-ledger current-pparams source faithfulness)
**Family:** *the recovered ledger's `protocol_params` are the oracle-captured current values
installed at seed/import time, provenance-bound to the bundle commitment — never
`ProtocolParameters::default()`, genesis-initial, fabricated, or a runtime operator override.*
- `DC-LEDGER-10` — its "pparams sourced from canonical ledger, never testkit defaults / shell config
  / fallback constants" doctrine now governs the **recovered-ledger construction** on the node path.
  `strengthened_in += "PHASE4-N-F-G-A"`.
- `CN-NODE-01` — the single seed import now carries current pparams (extended, not duplicated).
  `strengthened_in += "PHASE4-N-F-G-A"`.
- `DC-CINPUT-02b` — the recovered/imported surface's provenance now covers current
  `ProtocolParameters` (bound to `protocol_params_hash`). `strengthened_in += "PHASE4-N-F-G-A"`.
- **No registry edit in this slice** (deferred to G-A close, per the S1 pattern + "no registry flip
  yet"). **A dedicated "recovered-ledger pparams are oracle-sourced current state" rule is a
  cluster-close candidate** if the IDD review judges the `DC-LEDGER-10` strengthening insufficient.

## 9. Slice-entry decisions (settle at implement)
- **D-1 — provenance bind to the existing oracle commitment.** Bind the carried `ProtocolParameters`
  to the existing `protocol_params_hash` using **the exact oracle JSON bytes, or a documented
  canonicalization that reproduces the committed hash.** If neither is possible, **stop and split —
  do not silently define a new hash** (the existing hash is the oracle commitment; inventing a
  second commitment is a separate authority decision). A tampered/fabricated params set **fails
  closed**.
- **D-2 — minimal carried field set.** Carry the fields the forge/ledger need with fidelity — at
  minimum `protocol_major` + `protocol_minor`; include the broader `ProtocolParameters` the
  recovered ledger legitimately holds. Do **not** widen into full mainnet-ledger pparams fidelity
  beyond what the bundle/ledger already model.
- **D-3 — fixture re-extraction (approved).** Extending the bundle changes its `fingerprint`; the
  `nfg_a_privnet_reference` consensus-inputs fixture (and any anchor_fp it feeds) is re-extracted to
  include the current params. The retained scratch dir `~/.cardano-nfg-a-privnet` exists for exactly
  this; after re-extraction, **rerun the S1 `pinning_*` tests to prove eta0/stake/ASC/VRF stayed
  stable** (they pin those surfaces, not pparams, so they should be invariant — verify).
- **D-4 — snapshot codec coverage.** Confirm the snapshot/WAL pparams codec round-trips every
  installed field (esp. `protocol_minor`); extend it if not. AC2 is the mechanical catch.

## 10. Replay / determinism obligations
Deterministic: the bundle parse + provenance bind + ledger install are pure functions of the
committed bundle bytes; `BTreeMap`-ordered, no wall-clock/rand/float. For a fixed bundle + UTxO
seed, the installed `protocol_params` and the captured snapshot are byte-identical across runs. The
pre-seed → warm-start round-trip extends the existing N-F-A replay-equivalence to include
`protocol_params` (same bundle + seed → same recovered `protocol_params`). No new authoritative
transition; no new canonical *BLUE* type (a field is added to the RED import bundle, re-fingerprinted
— not a BLUE canonical type).

## 11. Replay / crash / epoch validation (tests by name)
- **New (bundle carry + bind, in `consensus_inputs` tests):**
  - `bundle_carries_current_protocol_params` — the importer parses the current `ProtocolParameters`
    from the bundle.
  - `bundle_protocol_params_bind_to_oracle_hash` — carried params verify against
    `protocol_params_hash`.
  - `tampered_protocol_params_fail_oracle_bind` — a mutated params set fails the bind, fail-closed
    (negative).
- **New (install, in `admission` tests):**
  - `seed_import_installs_current_protocol_params` — `build_seed_ledger` + the bootstrap runner
    ledger set `LedgerState.protocol_params` from the bundle, **not** `default()`.
  - `no_default_or_genesis_initial_pparams_fallback` — a missing/absent bundle params field fails
    closed (no silent `default()`/genesis fallback).
- **New (recovery + forge view):**
  - `warm_start_recovery_preserves_protocol_params` — pre-seed → `warm_start_recovery` →
    `recovered.ledger.protocol_params` equal the fixture's current params (reuses the S1
    pre-seed→warm-start machinery).
  - `forge_call_site_sees_current_pparams` — `ForgeActivation.recovered.ledger.protocol_params`
    yields the expected current `protocol_major`/`protocol_minor`; **fails if it sees the default
    2.0** (the explicit anti-regression).
- **Updated:** the `nfg_a_privnet_reference` consensus-inputs fixture (re-extracted with current
  params); the S1 `pinning_*` tests re-confirmed green against the re-fingerprinted fixture.
- **No epoch/crash semantics changed** — S2a is a source fix, not a transition.

## 12. Mechanical acceptance criteria
- [ ] `cargo test -p ade_runtime` (the `consensus_inputs` carry/bind tests) green, two-run-stable.
- [ ] `cargo test -p ade_node` (the `admission` install + recovery + forge-view tests) green.
- [ ] Candidate gate `ci_check_recovered_ledger_pparams_sourced.sh` — asserts `build_seed_ledger` +
      the bootstrap runner ledger set `protocol_params` from the imported bundle and that neither
      path leaves it at `ProtocolParameters::default()` (grep-gate mirroring the existing
      `ci_check_*` containment gates).
- [ ] Carried gates green + unchanged: `ci_check_seed_import_closure.sh`,
      `ci_check_bootstrap_anchor_closure.sh`, `ci_check_consensus_input_provenance.sh` (guard d),
      `ci_check_node_run_loop_containment.sh` (untouched).
- [ ] `cargo build` + `cargo clippy` clean on touched crates; `cargo fmt` applied.
- [ ] **PO-1 re-run for S2 passes:** a test demonstrates the forge call site now obtains current
      `protocol_version`/`pparams` from `recovered.ledger.protocol_params` (the S2 gate).
- [ ] Acceptance scoped to touched crates (`ade_runtime`, `ade_node`, consumed `ade_ledger`) —
      **not** the full `ade_testkit` corpus/oracle lane (~600s timeout on clean HEAD).

## 13. Failure modes
All **fail-fast** (structured, secret-free; no placeholder fallback):
- Bundle missing the current params field → fail-closed import error (no silent `default()`).
- Carried params fail the `protocol_params_hash` bind → fail-closed (anti-fabrication).
- Snapshot codec drops an installed field → `warm_start_recovery_preserves_protocol_params` fails
  (caught at slice close, not in prod).
- No path may substitute a default/genesis-initial value silently.

## 14. Hard prohibitions (inherits the cluster "Forbidden during this cluster" list)
- **No `ProtocolParameters::default()` (major 2), genesis-initial, fabricated, or
  runtime-operator-override** value standing in for the recovered ledger's current
  `protocol_params`/`protocol_version`.
- **No second bootstrap / no parallel recovery / no Mithril call** (CN-NODE-01) — S2a extends the
  single seed import only.
- No deriving current Conway pparams **from genesis initial params** (that's the bug; genesis-initial
  is S2's cross-check only).
- No new **BLUE authority / canonical type / WAL/checkpoint format** beyond the RED bundle field +
  its codec; no touching the BLUE `ProtocolParameters`/`LedgerState` *definitions* (populate, don't
  redefine).
- No **serve / serve-handoff / live-feed / WirePump / RO-LIVE / BA-02** work (G-B/G-C).
- Do **not** relax `ci_check_node_run_loop_containment.sh`; no `SystemTime`/`Instant`/float; no
  `Serialize`/`Deserialize` leak vectors into custody paths.
- No **registry edit** (strengthenings deferred to cluster close).
- **Hard line:** if the fix needs a BLUE authority change, a second bootstrap, or serve/live wiring —
  **stop and re-scope.**

## 15. Explicit non-goals
No S2 parser/ingress work, no S3/S4. No produce-path change. No full mainnet `ProtocolParameters`
fidelity beyond what the recovered ledger legitimately models. No new operator runtime config
surface. No serve/live/operator-pass. No genesis-initial derivation of current params.

## 16. Completion checklist
- [ ] Bundle carries the current `ProtocolParameters`, provenance-bound to `protocol_params_hash`;
      `build_seed_ledger` + the bootstrap runner ledger install them; warm-start preserves them; the
      forge view is current (fails on default 2.0).
- [ ] All §12 gates green; carried gates green & unchanged; `cargo test` scoped to touched crates
      green; `fmt`/`clippy` clean.
- [ ] `nfg_a_privnet_reference` fixture re-extracted with current params; S1 `pinning_*` re-confirmed
      green.
- [ ] **PO-1 re-run passes → S2 unblocked.**
- [ ] Slice doc committed standalone (`docs:`) before implementation; impl committed
      (`feat:`/`test:`) after green, model-attribution trailer. **No registry edit** (deferred to
      cluster close).
- [ ] **Housekeeping (G-A close):** clean/archive `~/.cardano-nfg-a-privnet` once fixture
      re-extraction is no longer needed.

## Authority
Registry IDs `DC-LEDGER-10` + `CN-NODE-01` + `DC-CINPUT-02b` (strengthened — recovered-ledger
current-pparams source faithfulness; registry append **deferred to cluster close**; a dedicated rule
is a close candidate), `CN-SEED-01` / `DC-SEED-01` / `CN-CINPUT-02` / `DC-CINPUT-01` / `DC-COMPAT-01`
/ `DC-NODE-05` / `DC-EPOCH-03` (preserved). The cluster doc `cluster.md` and
`docs/ade-invariant-registry.toml` are authoritative; this slice doc refines, it does not override.
