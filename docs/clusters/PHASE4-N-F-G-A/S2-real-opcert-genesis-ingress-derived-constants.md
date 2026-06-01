# PHASE4-N-F-G-A — Slice S2: Real opcert/genesis ingress + derived constants

> **Status:** slice doc (IDD Part IV). Companion to `cluster.md` (S2 row + CE-G-A-2) and
> `../../planning/phase4-n-f-g-{invariants,cluster-slice-plan}.md`. Code-verified against HEAD
> `b5decb3e`.
>
> **Slice S2 in one line:** retire the `parse_simple_*` stubs **on the `--mode node` forge path**,
> route operator config through the real closed-contract parsers (`parse_opcert_envelope` +
> an extended `parse_shelley_genesis`), and source **every** forge constant from a single named,
> proven origin — genesis for network/clock/KES/initial config, the **recovered current
> ledger/consensus view** for the live protocol version + parameters, the opcert envelope for the
> opcert counter/metadata — so **no fabricated, defaulted, or genesis-initial-as-current literal
> drives the forged block's header.**
>
> **S2a RESOLUTION (2026-06-01 — PO-1 discharged).** The "current recovered view; else STOP/split"
> contingency throughout this doc (§5 table rows for `protocol_version`/`pparams`, §6, §10 PO-1, §13,
> §14) is **DISCHARGED — the split happened.** **S2a** (`3dba81db`, pushed) installs the oracle-bound
> current `ProtocolParameters` into the recovered ledger at seed/import time, and PO-1 re-runs green
> (`warm_start_recovery_preserves_protocol_params`: `recovered.ledger.protocol_params` carries the
> current major, not the default 2). **S2 now SOURCES `protocol_version` + `pparams` from
> `recovered.ledger.protocol_params` — it must CONSUME that source, not recreate another one** (S2a
> is the single current-pparams authority installed at seed/import time; do not re-derive from
> genesis, defaults, or a parallel ingress). **Narrowed S2 scope (operator-confirmed):** retire
> `parse_simple_{genesis,opcert}_json` on the node path; real `parse_shelley_genesis` for
> **clock/KES/network constants only**; real `parse_opcert_envelope` for opcert metadata/counter;
> `protocol_version` + `pparams` from the recovered ledger only; **no defaults; no genesis-initial as
> current; no serve/live/RO-LIVE; no registry flip yet.**

## 1. Slice identity
- **Cluster:** PHASE4-N-F-G-A (forge fidelity on the node spine). Gated **behind** S1
  (genesis-consistency, green); **gates** S3/S4 within G-A. Serve/live (G-B/G-C) stay gated until
  G-A closes.
- **Slice:** S2 — real cardano-cli opcert/genesis ingress + derived constants on the node forge
  path.
- **Modules:** **RED** `ade_node::operator_forge` (the single node-path ingress site — rewired);
  **RED** `ade_runtime::producer::genesis_parser` (narrow extension: emit the **initial** protocol
  config); reused **RED** `ade_runtime::producer::opcert_envelope` (unchanged); the node forge
  call-chain `node_lifecycle` → `node_sync::forge_one_from_recovered` (constant sourcing); consumed
  **BLUE** (`ade_ledger` forge engine, recovered `LedgerState`/pparams). **No BLUE
  authority/canonical-type change; no second bootstrap; no serve/live wiring.**

## 2. Cluster Exit Criteria addressed (verbatim)
- **CE-G-A-2** — ingress fidelity: `operator_forge` calls `parse_opcert_envelope` +
  `parse_shelley_genesis` (no `parse_simple_*` on the node path — candidate gate
  `ci_check_node_forge_real_cli_ingress.sh`); `protocol_version` + `prev_opcert_counter` derive
  from the loaded opcert/genesis (S2 extends `parse_shelley_genesis` to emit
  `protocolVersion`/`protocolParams`); candidate test
  `operator_forge_empty_conway_block_invariant_to_honest_pparams` (or a derived-pparams test);
  existing `ci_check_operator_forge_no_secret_leak.sh` still green. *(strengthens `CN-OPCERT-01`,
  `CN-GENESIS-01`.)*

(CE-G-A-1 = S1 — its green gate is a **precondition** here; CE-G-A-3 = S3, CE-G-A-4 = S4 —
explicitly out of S2 scope.)

## 3. Intent (invariant impact)
Make it **mechanically impossible for the `--mode node` forge path to drive a forged block's
header from a stub-parsed, defaulted, or genesis-initial-as-current constant.** After S2, the node
path's operator config (opcert + genesis) is admitted **only** through the real cardano-cli
closed-contract parsers (fail-closed on any shape mismatch), and **every** header-bearing constant
has exactly one named, proven source (§5). This converts the two **enforced-but-dormant** parsers
(`CN-OPCERT-01`, `CN-GENESIS-01`) into the live node-path ingress, and removes the
`{major:9,minor:0}` / `ProtocolParameters::default()` / `prev_opcert_counter: None` /
`parse_simple_*` placeholders that N-F-F left as honest-scope wiring — **without** turning genesis
parsing into a fake current-ledger authority.

## 4. Pre-conditions (verified at HEAD `b5decb3e`)
- **The real parsers exist, are enforced, and are dormant on the node path:**
  - `producer::opcert_envelope::parse_opcert_envelope(&[u8]) -> DecodedOpCertEnvelope { opcert,
    cold_vk:[u8;32] }` (`opcert_envelope.rs:87`); closed `OpCertParseError`; `CN-OPCERT-01`
    enforced; **only test callers** (dormant).
  - `producer::genesis_parser::parse_shelley_genesis(&[u8], kes_anchor_slot:u64) -> GenesisAnchor`
    (`genesis_parser.rs:53`); closed `GenesisParseError`; `CN-GENESIS-01` enforced; **only test
    callers** (dormant). **Verified: emits clock/KES anchors only — networkMagic,
    systemStart→`slot_zero_time_unix_ms`, slotLength→`slot_length_ms`, slotsPerKESPeriod,
    maxKESEvolutions — NOT `protocolVersion`/`protocolParams`.**
- **`GenesisAnchor`** (`coordinator.rs:48`) — RED, `#[derive(…, Copy …)]`, 6 fields ("Genesis-
  derived constants. All non-secret."). **`ProtocolVersion`** (`block.rs:85`) is `Copy`.
- **Real genesis JSON shape (verified against the committed fixture):**
  `protocolParams.protocolVersion = {"major":2,"minor":0}` — the **Shelley-initial** version. The
  **current** Conway major is **not** in genesis. (The private-net fixture's genesis major is
  unknown and is a PO-1 verify item — it may or may not equal the current major.)
- **The recovered state carries the current version:** `ade_ledger::pparams::ProtocolParameters
  .protocol_major` (`pparams.rs:62`; default 2; updated on `HardForkInitiation` enactment,
  `governance.rs:397` / `pparams.rs:514`). `BootstrapState { ledger: LedgerState, chain_dep }`
  (`bootstrap.rs:92`); `recovered.ledger` is already passed into the forge
  (`ForgeRequestContext.base_state`). **Whether a current-pparams accessor is reachable at the
  forge call site is the PO-1 "if available" check.**
- **The swap site + placeholders to remove:** `operator_forge::build_operator_forge_material`
  (`operator_forge.rs:136`) calls `parse_simple_opcert_json` (`:96`, via
  `load_operator_producer_shell`) + `parse_simple_genesis_json` (`:141`), and hardcodes
  `pparams: ProtocolParameters::default()` + `protocol_version: { major:9, minor:0 }` +
  `start_slot: SlotNo(0)` (`:155-158`).
- **The consumer chain:** `node_lifecycle.rs:400-421` destructures the material + `coordinator_init`;
  `node_lifecycle.rs:662-672` calls `forge_one_from_recovered(act.recovered, …, &act.pparams, …,
  act.protocol_version.clone())`; `node_sync.rs:434-447` builds `ForgeRequestContext { …,
  protocol_version, prev_opcert_counter: None }`. The `None` and the `{9,0}` are the placeholders
  S2 sources.
- **Intent confirmed in-code:** `produce_mode.rs:603-605` — *"Honest scope: `protocol_version`,
  `pparams`, and `prev_opcert_counter` are still defaults — deriving them from the loaded
  genesis/opcert is the … wiring (G4)."* G-A executes G4 **on the node path only** (the produce
  path is out of scope).
- **`parse_simple_*` stay alive for the produce path:** defined `produce_mode.rs:488,524`, also
  called `produce_mode.rs:115,122`. S2 retires them **on the node path** (`operator_forge`), not
  globally; no dead-code.
- **No-leak contract to preserve:** `ci_check_operator_forge_no_secret_leak.sh` greps the
  `operator_forge.rs` production body for forbidden tokens
  (`println!`/`to_bytes`/`as_bytes`/`Serialize`/`Deserialize`/`unsafe`/`CoordinatorState`/…) +
  requires the `//! RED` banner + `load_operator_producer_shell` + `ProducerShell::init` + forbids
  any `pub fn … -> ([u8|Vec<u8>|&[u8])`.

## 5. Constant-source table (the forge-fidelity contract)
**Every forge constant the node path feeds into the header has exactly ONE named source.** A
constant with no proven source is an OPEN proof obligation (§10) — it may **not** be defaulted,
fabricated, or silently borrowed from a test/simple-JSON field.

| Forge constant | Named source | Status | Notes |
|---|---|---|---|
| `network_magic` | genesis parser (`parse_shelley_genesis`) | **pinned** | `networkMagic` |
| `slot_zero_time_unix_ms` | genesis parser | **pinned** | `systemStart` → ISO8601→ms |
| `slot_length_ms` | genesis parser | **pinned** | `slotLength` (s→ms) |
| `slots_per_kes_period` | genesis parser | **pinned** | |
| `kes_max_period` | genesis parser | **pinned** | `maxKESEvolutions` |
| **`kes_anchor_slot`** | **one named, proven, enforced source — still to pin (PO-3)** | **OPEN** | NOT in genesis; may **not** fabricate `0` or reuse a test/simple-JSON field unless proven correct for the fixture/network **and** mechanically enforced |
| opcert metadata (`hot_vkey`, `kes_period`, `sigma`, `sequence_number`) | opcert envelope parser (`parse_opcert_envelope`) | **pinned (this slice)** | replaces `parse_simple_opcert_json`; flows through `ProducerShell::init` (CN-PROD-02 unchanged) |
| **`prev_opcert_counter`** | opcert parser / derived opcert metadata (PO-2) | **OPEN (semantics)** | retire `None`; confirm what the BLUE forge engine consumes it for, then pin the source |
| **`protocol_version`** | **current recovered ledger / consensus-ledger view (PO-1)** | **OPEN** | if unavailable → **STOP and split** a "current protocol params source" slice; **never** genesis-initial as a stand-in |
| **`pparams`** | **current recovered ledger / consensus-ledger view (PO-1)** | **OPEN** | empty-block invariance (PO-4) bounds the fidelity burden; it does **not** license genesis/default fallback — if the recovered view lacks them → STOP/split |
| `pool_id` | operator cold key — one named place `blake2b_224(cold_vk)` (`operator_forge.rs:143`) | **pinned (N-F-F)** | unchanged |
| genesis-initial protocol config | extended `parse_shelley_genesis` (`protocolParams.protocolVersion`) | **cross-check only** | emitted for CN-GENESIS-01 closed-contract completeness + the PO-1 mechanical distinction; **never** the forge's current source |

## 6. Implementation boundary (real-parser retirement + named-source wiring)
**A — Real-parser retirement (node path no longer uses `parse_simple_*`):**
- `load_operator_producer_shell`: read `paths.opcert` bytes → `parse_opcert_envelope(&bytes)?.opcert`
  (replacing `parse_simple_opcert_json`); the real `OperationalCert` flows into `ProducerShell::init`
  (freshness bound CN-PROD-02 unchanged). The envelope's `cold_vk` is available for an **optional**
  cold-key cross-check (not required this slice).
- `build_operator_forge_material`: read `paths.genesis` bytes → `parse_shelley_genesis(&bytes,
  kes_anchor_slot)?` (replacing `parse_simple_genesis_json`); clock/KES anchors flow exactly as
  today.
- `OperatorForgeError::{OpcertParse, GenesisParse}` change payload `&'static str` → the structured
  `OpCertParseError`/`GenesisParseError` (closed-grammar errors; still path-byte-free — preserves
  the no-leak property and `operator_forge_error_carries_no_path_or_key_bytes`).

**B — Narrow genesis-parser extension (initial protocol config, cross-check only):**
- `parse_shelley_genesis` extended to also read `protocolParams.protocolVersion {major,minor}` and
  expose it (struct-shape choice — extend the `Copy` `GenesisAnchor` vs. a sibling return — settled
  at implement; both preserve determinism + the extra-inert-keys byte-identity). This is the
  **initial/genesis** protocol config, **not** the forge's current source.

**C — Named-source constant wiring (no placeholder literals on the node forge path):**
- `protocol_version` + `pparams` — sourced from the **recovered current ledger/consensus view**
  (PO-1); if unavailable, **STOP and split** — do **not** fall back to genesis-initial or defaults.
- `prev_opcert_counter` — sourced from the loaded opcert / derived opcert metadata (PO-2),
  replacing the `None` at `node_sync.rs:446`.
- `kes_anchor_slot` — sourced from one named, proven, mechanically-enforced origin (PO-3).

Out of scope: produce path, serve/live, second bootstrap, BLUE engine changes, full pparams
fidelity.

## 7. TCB color (execution boundary)
- **RED (rewired):** `ade_node::operator_forge` (real-parser ingress + structured errors + named-
  source constant sourcing). Stays a RED-custody site; no new leak vector.
- **RED (narrow extension):** `ade_runtime::producer::genesis_parser::parse_shelley_genesis` — emit
  the genesis-initial `protocolVersion`; closed parser contract preserved.
- **RED (reused, unchanged):** `producer::opcert_envelope::parse_opcert_envelope`;
  `ProducerShell::init`.
- **GREEN/RED (constant-sourcing seam):** `node_lifecycle` / `node_sync::forge_one_from_recovered`
  — reads the recovered current view + opcert metadata into the existing `ForgeRequestContext`;
  pure plumbing, no new authority.
- **BLUE (consumed, unchanged):** `ade_ledger` forge engine + `ProtocolParameters` reads; recovered
  `LedgerState`/`PraosChainDepState`. **No BLUE change.** A BLUE edit is a red flag S2 is absorbing
  authority → reject.

## 8. Invariants preserved (must not weaken) — by registry ID
- `CN-NODE-01` — single `bootstrap_initial_state`/`warm_start_recovery` authority; S2 adds **no**
  second bootstrap / Mithril call / parallel recovery. Recovered state is read-only here.
- `CN-PROD-02` — KES-period-vs-opcert freshness bound at `ProducerShell::init` still enforced (the
  real opcert flows through it unchanged).
- `DC-CINPUT-02b` / `CN-CINPUT-03` — leadership inputs still come only from the recovered surface
  (guard d); S2 introduces **no** fabricated `SeedEpochConsensusInputs`/eta0/pool_id and **no**
  `--consensus-inputs-path` bundle token on the forge path.
- `DC-NODE-05` — forge-slot discipline, self-accept-only, no durable-tip advance, single-epoch
  fail-closed: untouched (S2 changes constant *sourcing*, not the loop/forge-slot contract).
  Registry append for G-A deferred to cluster close.
- `DC-NODE-03` — clock-seam slot derivation: untouched by S2 (that is S3).
- `DC-EPOCH-03` — `declared`; off-epoch fail-closed is S4. S2 must not pre-empt or weaken it.
- `DC-COMPAT-01` — no Ade-internal-fingerprint-vs-Haskell-state-hash equality introduced.
- `CN-NODE-03` — node-path operator ingress stays a RED-custody, no-leak site
  (`ci_check_operator_forge_no_secret_leak.sh` green).
- All BLUE invariants — read-only consumption.

## 9. Invariants strengthened (one family: node-forge-path constant fidelity)
**Family:** *the `--mode node` forge path admits operator config only through the real closed-
contract parsers, and sources every header-bearing constant from a single named, proven origin —
no `parse_simple_*` stub, no fabricated/defaulted literal, and no genesis-initial value standing in
for the current protocol version/parameters.*
- `CN-OPCERT-01` — real cardano-cli opcert envelope ingress goes **live on the node forge path**
  (was enforced but dormant). `strengthened_in += "PHASE4-N-F-G-A"`.
- `CN-GENESIS-01` — real Shelley-genesis ingress goes **live on the node forge path**, and the
  closed contract is **extended** to emit the initial `protocolVersion`.
  `strengthened_in += "PHASE4-N-F-G-A"`.
- **No registry edit in this slice.** Per the S1 pattern and the "no registry flip yet" directive,
  the `strengthened_in` appends (and `tests`/`ci_script` array additions) are **deferred to cluster
  close**, when CE-G-A-1..4 are all green. No status flip in S2.

## 10. Slice-entry proof obligations (S2-specific, load-bearing)
*Each is a fact to establish at implement-slice, not a footnote. This is the forge-fidelity problem
S2 exists to catch — encoded here as a gate before any code.*

- **PO-1 — `protocol_version` + `pparams`: current recovered view, or STOP/split.** Genesis emits
  the **initial** protocol config (the committed preprod fixture shows major `2`); a Conway forge
  needs the **current** version/parameters for the forge slot. **Source the forge's
  `protocol_version` + `pparams` from the recovered seed-epoch ledger state / recovered consensus-
  ledger view.** If a current-protocol-params accessor is **not** reachable from the recovered state
  at the forge call site, **S2 STOPS and splits a small "current protocol params source" slice** —
  it does **not** fall back to genesis-initial or `ProtocolParameters::default()`. `parse_shelley_
  genesis` must **not** become a current-ledger authority. **Verify at entry:** the recovered
  current major is the real network major (not a stale default `2`) on the S1 fixture / a
  recovered-state fixture.
- **PO-2 — `prev_opcert_counter`: pin the source + confirm engine semantics.** The block header's
  *embedded* opcert (incl. its `sequence_number`) already comes from the `ProducerShell`;
  `prev_opcert_counter` is a **separate** engine input. Confirm at entry what `ade_ledger` forge
  consumes it for (embedded counter vs. a no-downgrade check input). If it is the operator's own
  opcert counter, source it from `shell.opcert().sequence_number`; if it is a *previously-seen*
  chain counter, source it from recovered chain state. Either way the `None` on the node forge path
  is retired and a test pins the chosen source.
- **PO-3 — `kes_anchor_slot`: one named, proven, enforced source (not in genesis).** S2 must
  identify a single named source for `kes_anchor_slot` on the node path. It may **not** silently use
  a test/simple-JSON field or fabricate `0` **unless** that value is explicitly proven correct for
  the fixture/network **and** mechanically enforced (a test that pins it + a structured failure if
  the assumption is violated). The opcert's `kes_period` + the CN-PROD-02 freshness bound are the
  honesty anchor.
- **PO-4 — pparams honest-scope burden is *bounded*, not waived.** Prove (CE-G-A-2 named test) that
  an **empty** Conway block's header+body validation verdict is **invariant** to perturbations of
  the honest-scope pparams, holding the derived `protocol_version` fixed. This bounds how much
  pparam fidelity the empty-block forge needs — it is a robustness proof, **not** a license to use
  genesis/default pparams in place of the recovered view (PO-1 still governs the source).

## 11. Replay / determinism obligations
The parsers are pure, deterministic, and already replay-anchored (`DC-OPCERT-01`, `DC-GENESIS-01` —
byte-identical across runs). The genesis-parser extension preserves that: same bytes + same
`kes_anchor_slot` → byte-identical output, and the extra-inert-keys byte-identity property holds
(the new field must not break `extra_inert_keys_produce_byte_identical_anchor`). Constant sourcing
from the recovered view is a pure read — for a fixed recovered state + opcert/genesis bytes, the
derived `(protocol_version, pparams, prev_opcert_counter, anchors)` and the forged block bytes are
byte-identical across runs (extends DC-NODE-05's replay-equivalence). No new on-disk corpus, no new
canonical type, no new WAL/checkpoint format.

## 12. Replay / crash / epoch validation (tests by name)
- **New (node-path ingress, in `operator_forge` tests):**
  - `node_forge_loads_real_opcert_envelope` — `load_operator_producer_shell` parses a real
    `NodeOperationalCertificate` envelope; malformed → structured `OpCertParseError` (fail-closed).
  - `node_forge_loads_real_shelley_genesis` — `build_operator_forge_material` parses a real
    `shelley-genesis.json`; malformed → structured `GenesisParseError` (fail-closed).
  - `operator_forge_error_carries_no_path_or_key_bytes` — **preserved**, re-asserted against the
    structured-enum error payloads.
- **New (genesis-parser extension, in `genesis_parser` tests):**
  - `shelley_genesis_emits_initial_protocol_version` — extended parse emits
    `protocolParams.protocolVersion {major,minor}`.
  - `extra_inert_keys_produce_byte_identical_anchor` — **preserved/extended** to cover the new field.
- **New (named-source constants, in the node forge tests):**
  - `node_forge_protocol_version_from_recovered_current_view` — the forged header's
    `protocol_version` == the recovered current view's `(protocol_major, minor)`, and is **not**
    sourced from genesis-initial (pins PO-1). *(If unavailable, this test does not exist because S2
    stopped and split — see §14.)*
  - `node_forge_pparams_from_recovered_current_view` — pparams sourced from the recovered view (PO-1).
  - `node_forge_kes_anchor_slot_from_named_proven_source` — `kes_anchor_slot` comes from the named
    source and a violated assumption fails closed (pins PO-3).
  - `node_forge_prev_opcert_counter_sourced_not_none` — the node forge `ForgeRequestContext` no
    longer hardcodes `None` (pins PO-2).
  - `operator_forge_empty_conway_block_invariant_to_honest_pparams` — empty Conway block
    header+body validation verdict invariant to honest-scope pparams perturbation, `protocol_version`
    fixed (CE-G-A-2; PO-4).
- **Updated:** `build_operator_forge_material_*` + `load_operator_producer_shell_*` — the existing
  `write_operator_material` test helper migrates from simple-JSON to **real cardano-cli formats**
  (real shelley-genesis + real opcert envelope; reuse the existing `write_cardano_cli_envelope`
  helper).
- **No crash/epoch validation in S2** — epoch-boundary fail-closed is S4; crash recovery is
  N-F-A/N-F-D's domain.

## 13. Mechanical acceptance criteria
- [ ] `cargo test -p ade_node` (the `operator_forge` + node-forge ingress/named-source tests above)
      green, two-run-stable.
- [ ] `cargo test -p ade_runtime` (the `genesis_parser` extension tests) green.
- [ ] Candidate gate `ci_check_node_forge_real_cli_ingress.sh` — asserts `operator_forge.rs`
      references `parse_opcert_envelope` + `parse_shelley_genesis` and **no**
      `parse_simple_opcert_json`/`parse_simple_genesis_json` (node-path retirement; mirrors the
      existing containment-grep gates).
- [ ] `ci_check_operator_forge_no_secret_leak.sh` **stays green** (no new forbidden token from the
      rewire; no raw-byte-returning `pub fn`).
- [ ] Carried gates green + unchanged: `ci_check_node_run_loop_containment.sh` (byte-/semantically
      untouched), `ci_check_consensus_input_provenance.sh` (guard d),
      `ci_check_genesis_consistency_fixture_present.sh` (S1),
      `ci_check_no_haskell_fingerprint_equality.sh` (DC-COMPAT-01).
- [ ] `cargo build` + `cargo clippy` clean on touched crates; `cargo fmt` applied.
- [ ] Acceptance scoped to touched crates (`ade_node`, `ade_runtime`, consumed
      `ade_ledger`/`ade_core`) — **not** the full `ade_testkit` corpus/oracle lane (times out ~600s
      on clean HEAD).

## 14. Failure modes
All **fail-fast** (structured, secret-free; a parse/source failure halts ingress — no placeholder
fallback):
- Malformed/wrong-type opcert envelope → `OperatorForgeError::OpcertParse(OpCertParseError::…)`.
- Malformed/wrong-type/missing-field genesis → `OperatorForgeError::GenesisParse(GenesisParseError::…)`.
- **Recovered current protocol version/pparams unavailable (PO-1)** → **STOP and split** a "current
  protocol params source" slice; do **not** ship genesis-initial / default pparams. (This is a
  scope decision surfaced at implement, not a runtime fallback.)
- **`kes_anchor_slot` source cannot be proven/enforced (PO-3)** → STOP; do not fabricate `0`.
- No new failure path may affect consensus/replay silently — every path returns a closed error or
  halts.

## 15. Hard prohibitions (inherits the cluster "Forbidden during this cluster" list)
- **No `parse_simple_*` on the node path** (the whole point); produce-path callers untouched.
- **No genesis-initial protocol version (or genesis-initial/default pparams) as a stand-in for the
  current Conway protocol version/parameters** — unless the test fixture proves they are *identical*
  for that network **and** that identity is mechanically enforced. `parse_shelley_genesis` must not
  become a current-ledger authority.
- **No fabricated/defaulted `kes_anchor_slot`** (incl. literal `0`) unless proven correct for the
  fixture/network and mechanically enforced.
- No **fabricated/placeholder forge constant** on the node path — no hand-written `{9,0}`, no
  `prev_opcert_counter: None` left in place.
- No new **BLUE authority / canonical type / WAL / checkpoint format**; no second bootstrap / no
  Mithril call (CN-NODE-01).
- No **serve / serve-handoff / `push_atomic` / `block_fetch` / gossip** (G-B); no **live-feed /
  `WirePump` / `n2n_dialer` / session wiring** (G-C); no **RO-LIVE / BA-02 / peer-acceptance** claim.
- Do **not** relax `ci_check_node_run_loop_containment.sh`; no serve token in the loop body.
- No `SystemTime`/`Instant`/float introduced into the ingress/sourcing; no key-byte leak vector
  (`ci_check_operator_forge_no_secret_leak.sh` stays green); no `Serialize`/`Deserialize` in the
  `operator_forge` production body.
- No **registry edit** (strengthenings deferred to cluster close).
- No **full `ProtocolParameters` derivation** from genesis (empty-block invariance bounds the
  burden; the source is the recovered view, not genesis).
- **Hard line:** if the rewire needs a BLUE change, a containment relaxation, serve/live wiring, or
  a second bootstrap — **stop and re-scope.**

## 16. Explicit non-goals
No S3 slot-alignment / S4 epoch-fail-closed work. No serve handoff, live feed, operator pass, or
peer acceptance. No produce-path (`produce_mode`) rewire. No cross-epoch / nonce-roll. No
mainnet-complete pparams fidelity. No Mithril / second bootstrap. No new CLI flag or config switch
beyond what real ingress strictly requires.

## 17. Completion checklist
- [ ] Node-path ingress routed through `parse_opcert_envelope` + `parse_shelley_genesis`;
      `parse_simple_*` removed from `operator_forge.rs`; structured errors wired.
- [ ] `parse_shelley_genesis` extended to emit the initial `protocolVersion`; byte-identity +
      extra-inert-keys properties preserved.
- [ ] Constant-source table (§5) fully discharged: `protocol_version` + `pparams` from the recovered
      current view (PO-1; else stopped/split), `prev_opcert_counter` sourced (PO-2),
      `kes_anchor_slot` from a named proven enforced source (PO-3), pparams empty-block invariance
      proven (PO-4). No fabricated/defaulted/genesis-initial-as-current literal remains.
- [ ] All §13 gates green; no-leak + containment gates green & unchanged; `cargo test` scoped to
      touched crates green; `fmt`/`clippy` clean.
- [ ] Slice doc committed standalone (`docs:`) **before** implementation; impl committed
      (`feat:`/`test:`) after green, with the model-attribution trailer. **No registry edit**
      (deferred to cluster close).
- [ ] **Housekeeping (carried to G-A close):** clean or archive the throwaway scratch dir
      `~/.cardano-nfg-a-privnet` after G-A close, once fixture re-extraction is no longer needed.
- [ ] **If PO-1's recovered current view is unavailable → STOP and split a "current protocol params
      source" slice; do NOT ship a genesis-initial/default protocol version or parameters.**

## Authority
Registry IDs `CN-OPCERT-01` + `CN-GENESIS-01` (strengthened — node-path ingress goes live; registry
append **deferred to cluster close**), `DC-NODE-05` / `DC-NODE-03` / `DC-EPOCH-03` / `CN-NODE-01` /
`CN-PROD-02` / `DC-CINPUT-02b` / `CN-CINPUT-03` / `DC-COMPAT-01` / `CN-NODE-03` (preserved). The
cluster doc `cluster.md` and `docs/ade-invariant-registry.toml` are authoritative; this slice doc
refines, it does not override.
