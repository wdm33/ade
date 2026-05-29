# Operator-pass live leg (CN-CONS-06 / RO-LIVE-01) — C1 scoping pass

> **Type:** SCOPING pass output. No live nodes were run. Pick-up HEAD: `c83f2ba`
> (tree clean). Seed: [`operator-pass-live-leg-c1-followon.md`](operator-pass-live-leg-c1-followon.md).
> Authority read end-to-end first: the four grounding docs at `c83f2ba`
> (CODEMAP/SEAMS/HEAD_DELTAS/TRACEABILITY) + the registry rules CN-CONS-06,
> RO-LIVE-01, CN-OPERATOR-EVIDENCE-01, CN-PROD-01/02, CN-FORGE-01..04, CN-WIRE-08.

---

## 0. Headline

`blocked_until_real_forge_handler_lands` is **CLOSED at the code level**.
`produce_mode::run_real_forge` is the **live** forge path (no stub), and the
driver wires it to real, ChainEvolution-derived inputs. The full
forge → self-accept → served-chain → per-peer dispatch → tag-24 wire path is
mechanically present.

**But it is NOT pure operator execution against today's binary.** A **wiring
cluster is owed before execution** (scope in §4 / §4b).

> **RE-SCOPED 2026-05-29 → tip path is the primary target.** The bounty grades
> block production on **preview/preprod** (a live chain at its current tip /
> current epoch), not on a private genesis net. So the primary target is **C2 —
> forge a block accepted at the live preprod tip** ([[project-bounty-requirements]]
> priority #3). The genesis/epoch-0 "C1" private-net pass is retained only as a
> cheaper hermetic *rehearsal*, not a graded deliverable. §4b is the tip-primary
> plan and supersedes the genesis-centric framing in §2/§4 (kept for reference).
> The tip path adds two load-bearing gaps the genesis path hid (**G6** non-genesis
> tip bootstrap into produce, **G7** current-slot alignment — §1d), and is bounded
> to a **single epoch** (the cross-epoch case is a separate cluster — §4a).

---

## 1. Mechanically-ready inventory (forge → serve → peer-accept), file:line

### 1a. Real forge composition — WIRED, live path

`run_real_forge` (`crates/ade_node/src/produce_mode.rs:623`) is the BLUE→RED→BLUE
pipeline, and it is the **live** path: the slot-loop's `RequestForge` arm calls
it with real inputs (not a stub).

| Step | Code | Status |
|---|---|---|
| Era guard (non-Praos → `UnsupportedProducerEra`) | `produce_mode.rs:634-648` | ✓ N-W |
| RED VRF prove over the answer's `alpha_bytes()` (no RED era dispatch) | `produce_mode.rs:654-663` | ✓ N-W (CN-FORGE-04) |
| BLUE `verify_and_evaluate_leader`; `NotEligible` → `ForgeNotLeader` | `produce_mode.rs:668-699` | ✓ N-R-A (CN-FORGE-02) |
| **Real** KES-signs-**real** `unsigned_header_pre_image` (two-pass: placeholder sig → real sig over the canonical pre-image) | `produce_mode.rs:701-843` | ✓ N-S-A (CN-KES-HEADER-01) |
| BLUE `forge_block` (enveloped `[era,block]`) + `self_accept` | `produce_mode.rs:872-900` | ✓ N-R-A/N-W (CN-FORGE-03) |
| `ForgeSucceeded` only on self-accept | `produce_mode.rs:885-909` | ✓ |

> **Stale comments to ignore** (they predate N-S-A and contradict the code below
> them): the `run_real_forge` doc-comment step 3 "the signing payload is currently
> the `expected_vrf_input` bytes (a placeholder)" (`:608-614`); the
> `ForgeRequestContext` doc-comment "the main loop currently passes synthesized
> 'never-leader' placeholders" (`:579-583`); the unused-warning comment "the S5
> stub forge handler emits ForgeNotLeader only" (`:1427-1429`). The **code** at
> `:701-843` (real pre-image + `kes_sign_header`) and `:1008-1044` (real
> `evo`-derived ctx) is authoritative. **Registry CN-PROD-02.open_obligation is
> likewise stale** — it still lists remainder (c)(i) "KES-signs a placeholder" and
> (c)(ii) "MuxPump outbound-relay extension"; both shipped in N-S-A / N-S-B. The
> wiring cluster should correct these comments + the registry text.

### 1b. Driver → forge inputs — WIRED to real bootstrap state

The `RequestForge` arm (`produce_mode.rs:1001-1044`) queries the **real**
leader schedule and builds a **real** `ForgeRequestContext`:

- `query_leader_schedule(..., evo.pool_distr_view(), evo.era_schedule(), evo.base_chain_dep())` — `:1008-1020`.
- ctx from `evo.*`: `eta0 = base_chain_dep().epoch_nonce`, `base_state = evo.base_ledger()`, real `vrf_vk` from the shell — `:1027-1042`.
- `ChainEvolution` is seeded from the real cold-start bootstrap triple + operator consensus-inputs nonce — `produce_mode.rs:197-233`.
- `ForgeSucceeded` → `ChainEvolution::advance` (sole `AcceptedBlock` minter) → `push_atomic` to the served chain — `:1058-1112`.

So leadership is **driven by the operator's `--consensus-inputs-path` bundle**:
`pool_distribution` (active stake), `pool_vrf_keyhashes`, `epoch_nonce` (= eta0),
`active_slots_coeff`, `epoch_no`, `epoch_start_slot` — projected at
`produce_mode.rs:399-423` (`pool_distr_view_from_consensus_inputs`).

### 1c. Serve path — WIRED, tag-24 correct

- `peer_outbound: Some(...)` **is** wired at listener startup — `produce_mode.rs:243-249`. (Registry CN-PROD-01.open_obligation says this is "the C-deliverable not yet wired" — also stale; it is wired.)
- Per-peer dispatch through the BLUE serve reducers → typed `OutboundCommand` (no `Vec<u8>` tunnel): `dispatch_server_frame_event_to_outbound` — `produce_mode.rs:1273-1410` (ChainSync `:1287-1329`, BlockFetch `:1330-1407`).
- MuxPump outbound relay encodes via the BLUE codecs `encode_chain_sync_message` / `encode_block_fetch_message` — `crates/ade_runtime/src/network/mux_pump.rs:142-150`.
- Those encoders apply the **CN-WIRE-08 tag-24 composition**: RollForward → `compose_rollforward_header` (`codec/chain_sync.rs:98-108`, era_tag outside the wrap); MsgBlock → `compose_blockfetch_block` (`codec/block_fetch.rs:100-108`, era inside the wrap). ✓ N-X
- `BlockServed` evidence observes only blocks present in the served snapshot — `produce_mode.rs:1371-1406`.

### 1d. What is NOT mechanically ready (needs operator setup / wiring)

| # | Gap | Evidence | Why it blocks C1 |
|---|---|---|---|
| G1 | **opcert input format mismatch.** `produce_mode` parses `--opcert` with `parse_simple_opcert_json` — a custom `{hot_vkey_hex, sequence_number, kes_period, sigma_hex}` JSON (`produce_mode.rs:471-496`). The real cardano-cli `node.opcert` text-envelope parser `opcert_envelope::parse_opcert_envelope` (N-R-C, CN-OPCERT-01) **exists but is dormant** — only called from its own tests (`ade_runtime/src/producer/opcert_envelope.rs:87`, callers all in-file). | grep: no non-test caller. | cardano-cli emits `node.opcert`; the binary can't read it. Operator must hand-convert, or we wire the real parser. |
| G2 | **genesis input format mismatch.** `produce_mode` parses `--genesis-file` with `parse_simple_genesis_json` — a custom 6-field subset (`produce_mode.rs:504-525`). The real `genesis_parser::parse_shelley_genesis` (N-R-C, CN-GENESIS-01) is also **dormant** (`ade_runtime/src/producer/genesis_parser.rs:53`, no non-test caller). | grep: no non-test caller. | Same as G1 for `shelley-genesis.json`. |
| G3 | **No consensus-inputs constructor for a from-genesis private net.** `--consensus-inputs-path` is consumed by `import_live_consensus_inputs` (N-M-C), whose bundle was designed to be **extracted from an already-synced peer** via cardano-cli. For a fresh C1 net there is no documented tool to build a bundle that is **consistent with the shared genesis** the Haskell peer enforces (eta0, stake, ASC, per-pool VRF keyhash). | `produce_mode.rs:186-195`. | **Load-bearing.** Ade's leader-eligibility + `self_accept` derive entirely from this bundle; if eta0 / stake / ASC / vrf_keyhash diverge from the peer's genesis-derived view, the peer rejects the block. |
| G4 | **Honest-scope constants** that may diverge from the peer's validation: `ProtocolParameters::default()` (`:239`), `protocol_version major:9, minor:0` hardcoded (`:1040`), `prev_opcert_counter: None` (`:1041`). | as cited. | A real Conway peer validates `protocol_version` + opcert counter; mismatches are reject vectors. **On preprod this is sharper** — the live protocol major is 10/11, not 9; must derive from the real genesis/pparams, not defaults. |
| G5 | Both mandatory operator flags are real and **un-documented in the runbook**: `--json-seed <utxo.json>` (`cli.rs:476-479`) and `--consensus-inputs-path <bundle>` (`cli.rs:480-483`). | cli.rs. | Binary won't start without them (see §2 runbook bug). |
| **G6** | **(TIP PATH) `produce_mode` cannot bootstrap from a non-genesis tip.** It opens an empty `InMemoryChainDb::new()` → `bootstrap_initial_state` cold-start branch → `tip=None`, and the coordinator tip hardcodes `block_number: 0` (`produce_mode.rs:196-204, 221-224`). The `--seed-point-slot`/`--seed-block-hash` flags exist but are consumed by the **admission** bootstrap (`admission/bootstrap.rs:125-237` via `seed_to_snapshot`, proven against preprod at slot 124140368), **not** by produce. | as cited. | **Load-bearing for the tip path.** A real peer only extends its own tip; a block built on a genesis/zero `prev_hash` with `block_no=0` cannot extend a chain at epoch ~200 → rejected / loses fork-choice. produce must forge block #(tip+1) on the peer's real tip hash. The admission path's tip-seeded bootstrap must be brought into produce. |
| **G7** | **(TIP PATH) Current-slot alignment.** The slot ticker free-runs `+= 1` from `boot_tip.slot+1` (or `epoch_start_slot`) at `slot_length_ms` intervals (`produce_mode.rs:261-275`); it does not resync the absolute slot to wall-clock, and `CoordinatorError::SlotDrift` is swallowed (`:327-331`). | as cited. | A real peer rejects a header whose slot is in the future beyond tolerance, and a stale slot can't extend the tip. The forge slot must track the peer's live slot — requires G6 (a real current tip slot) + a correct genesis `slot_zero_time + slot_length` → absolute-slot map, and drift handling over a multi-minute capture. |

---

## 2. C1 private-testnet recipe (corrected for the `c83f2ba` binary)

> The existing `docs/clusters/completed/PHASE4-N-S-C/S1.md` recipe is **stale and
> will fail**: (a) it omits `--json-seed` and `--consensus-inputs-path` (both
> mandatory — argv parse fails with `ProduceMissingFlag`); (b) its acceptance
> filter reads `.kind=="BlockForged").block_hash`, but the real field is `hash`
> and it serialises as a JSON **u8 array**, not a hex string
> (`producer_log.rs:155-159`), so the `grep -F "$HASH" peer.log` correlation
> never matches. The corrected recipe is below; the wiring cluster (§4) should
> supersede S1.md.

### Step 1 — keys + opcert (cardano-cli, operator-owned, never committed)

```bash
mkdir -p ~/.cardano-private-testnet-c1 && cd ~/.cardano-private-testnet-c1
cardano-cli node key-gen --cold-verification-key-file cold.vkey \
  --cold-signing-key-file cold.skey \
  --operational-certificate-issue-counter-file cold.counter
cardano-cli node key-gen-VRF --verification-key-file vrf.vkey --signing-key-file vrf.skey
cardano-cli node key-gen-KES --verification-key-file kes.vkey --signing-key-file kes.skey
cardano-cli node issue-op-cert --kes-verification-key-file kes.vkey \
  --cold-signing-key-file cold.skey \
  --operational-certificate-issue-counter-file cold.counter \
  --kes-period 0 --out-file node.opcert
```

cardano-cli's `kes.skey` is the 608-byte expanded `Sum6KES` envelope; Ade imports
it via the BLUE `Sum6Kes::raw_deserialize_signing_key_kes` (N-O/N-P). Loader:
`load_kes_skey_any_format` (`produce_mode.rs:453-462`).

### Step 2 — shared private genesis with Ade's pool holding ~all stake

Generate a Shelley+Conway genesis (e.g. `cardano-cli genesis create-staked` or
the cardano-node `mkfiles`/local-devnet recipe). Constraints that make Ade an
(almost) every-slot leader **and** keep the peer's validation in agreement:

- One stake pool, delegated ~all active stake, cold/VRF/KES = the Step-1 keys.
- `activeSlotsCoeff` high enough for frequent leadership (e.g. `0.5`). With ~1.0
  stake fraction, P(lead) ≈ ASC per slot.
- Record the genesis-derived initial nonce (`eta0`) — shelley-genesis hash /
  extra-entropy — it must equal what Ade is fed in Step 4.

### Step 3 — bring up the private Haskell peer (pulls Ade's chain)

```bash
docker run -d --name cardano-node-c1-peer --network host \
  -v ~/.cardano-private-testnet-c1:/keys:ro \
  ghcr.io/intersectmbo/cardano-node:11.0.1 \
  cardano-node run --config /keys/config.json \
    --topology /keys/topology.json \
    --database-path /keys/db --port 3010
```

Topology points the peer at Ade's listener (`127.0.0.1:3001`) so it chain-syncs
and block-fetches **from** Ade. Ade is server-role (opens the listener; the peer
dials in) — confirmed: `produce_mode.rs:242-253` spawns `run_n2n_listener`, peers
arrive as `OrchestratorEvent::PeerConnected { role: DownstreamServer }`
(`:1138-1143`).

### Step 4 — operator inputs for Ade (the parts the wiring cluster must close)

- `utxo.json` — cardano-cli UTxO dump for the genesis state (`--json-seed`).
- `consensus-inputs.toml` — the bundle giving Ade's pool the stake, the genesis
  eta0, the genesis ASC, and `pool_vrf_keyhashes[pool] = blake2b256(vrf.vkey)`.
  **No constructor exists for a from-genesis net today (G3).**
- `opcert.json` + `genesis.json` — today the **simple-JSON** forms (G1/G2) unless
  the cluster wires the real cardano-cli parsers.

### Step 5 — run Ade in produce mode (exact flags for `c83f2ba`)

```bash
ade_node --mode produce \
  --listen 127.0.0.1:3001 \
  --cold-skey   cold.skey \
  --kes-skey    kes.skey \
  --vrf-skey    vrf.skey \
  --opcert      opcert.json \        # G1: simple-JSON form today
  --genesis-file genesis.json \      # G2: simple-JSON form today
  --json-seed   utxo.json \          # MANDATORY (missing from S1.md)
  --consensus-inputs-path consensus-inputs.toml \  # MANDATORY (missing from S1.md)
  --evidence-log evidence.jsonl
  # optional: --max-slots N  (bounded smoke run)
```

### Step 6 — observe acceptance (corrected correlation)

- Ade-side leader/forge: `evidence.jsonl` `LeaderCheckOutcome{is_leader:true}` then
  `BlockForged{slot,hash,bytes_len}` (`producer_log.rs:146-159`). `hash` is a u8
  array — render to hex before correlating.
- Peer-side accept (authoritative): `docker logs cardano-node-c1-peer 2>&1 > peer.log`
  then match the **hex** block hash against `BlockAccepted|AddedToCurrentChain`.
- Ade-observable corroboration: `PeerChainTipObserved{hash}` equal to the forged
  hash (`producer_log.rs:169-173`) — the peer's chain-sync advertising our block
  as its tip. (Corroboration, **not** proof; the peer log is the proof —
  [[feedback-shell-must-not-overstate-semantic-truth]].)

---

## 3. Evidence contract → CN-OPERATOR-EVIDENCE-01

The committed pass must satisfy the closed manifest schema (15 fields +
sha256 cross-check), enforced by `ci/ci_check_operator_evidence_manifest_schema.sh`
(vacuous until a manifest is committed). Fields:
`schema_version, ade_commit, cardano_node_version, cardano_cli_version, network,
block_hash, slot, opcert_fingerprint, genesis_fingerprint, ade_evidence_file,
peer_log_file, peer_log_capture_command, peer_log_filter, peer_log_file_sha256,
acceptance_keyword_match` — `peer_log_file_sha256` MUST equal the committed raw
`peer.log` hash; `peer_log_filter` is documentation, the raw log is authority.

**Venue / path convention for C1.** The schema is owned by N-S-C, so the natural
home is the same family:
`docs/clusters/completed/PHASE4-N-S-C/CE-N-S-LIVE_YYYYMMDD-<short_commit>.{jsonl,log,toml}`
with `network = "private-testnet-c1"`. (The CI gate globs
`docs/clusters/PHASE4-N-S-C/CE-N-S-LIVE_*.toml`; if the directory is now under
`completed/`, the gate's glob must be widened to match — a one-line fix for the
wiring cluster to verify, else the committed manifest is silently un-checked.)

**What C1 flips vs. does not.** Per N-S-C §3: C1 is the *engineering bridge proof*
(flips the deferred-bridge open_obligations — CN-FORGE-01, CN-PROD-01 remainder,
CN-SNAPSHOT-01 — and the live halves of CN-CONS-06 / RO-LIVE-01 to
`enforced_with_c1_evidence` or a clearly-labelled C1 status). It is **NOT** a
substitute for C2 (preprod), which is the bounty-facing public surface. Empty-block
forging is the explicit scope; TxSubmission2/mempool/N2C/multi-node stay open.

---

## 4. Gap → plan: a thin wiring cluster IS owed

A first successful C1 is **not** reachable by operator action against `c83f2ba`
alone (G1–G5). Recommend a small **PHASE4-N-?? "C1 produce-mode wiring"** cluster
*before* the live run. Candidate invariants (to promote via `/invariants`):

1. **Real cardano-cli opcert ingress in produce-mode** (strengthens CN-OPCERT-01 /
   CN-PROD-02). `produce_mode` MUST load `--opcert` via the existing
   `opcert_envelope::parse_opcert_envelope` (cardano-cli text envelope), retiring
   `parse_simple_opcert_json`. Closes G1; un-dormants the N-R-C parser.
2. **Real cardano-cli genesis ingress in produce-mode** (strengthens CN-GENESIS-01).
   `--genesis-file` MUST load via `genesis_parser::parse_shelley_genesis`. Closes G2.
3. **From-genesis consensus-inputs constructor** (new CN rule). A single authority
   that builds the `LiveConsensusInputs` bundle for a private net from the shared
   `shelley-genesis.json` + Ade's `vrf.vkey` + the genesis stake/delegation, so the
   bundle's `(eta0, pool_distribution, active_slots_coeff, pool_vrf_keyhashes)` is
   **provably consistent** with the peer's genesis-derived view. Closes G3 (the
   load-bearing gap). Slice-entry proof obligation
   ([[feedback-proof-discipline]]): demonstrate eta0/stake/ASC agreement against a
   captured private-genesis fixture before the live run.
4. **Pin the honest-scope constants** (strengthens CN-FORGE-03 / self-accept).
   `protocol_version`, `ProtocolParameters`, `prev_opcert_counter` MUST derive from
   the loaded genesis/opcert, not hardcoded defaults — OR a test proves an empty
   Conway block's header+body validation is invariant to them at the peer. Closes G4.
5. **Evidence ↔ acceptance correlation fix** (strengthens CN-OPERATOR-EVIDENCE-01).
   Emit the forged block hash as hex in `evidence.jsonl` (or add a hex field), and
   either widen the CI manifest glob to the archived N-S-C path or define the C1
   manifest home. Fix the stale jq filter in the runbook. Closes G5 + §2/§3 bugs.
6. **Doc/registry hygiene** (no new rule): correct the stale comments in
   `produce_mode.rs` (:579-583, :608-614, :1427-1429) and the stale
   `CN-PROD-01.open_obligation` / `CN-PROD-02.open_obligation` text (the
   placeholder-KES and unwired-peer_outbound remainders already shipped).

After this cluster closes mechanically, the C1 live run becomes **operator
execution** and the relevant rules move to `blocked_until_operator_pass_executed`
with the corrected runbook committed.

### Sequencing note

Items 1, 2, 5, 6 are small and mechanical. **Item 3 is the real cluster** — it is
where the deep validity question lives ("will a real Haskell peer accept the
block?"), so it carries an adversarial / proof-of-agreement obligation, not just a
parser. Do not let 1/2/5/6 create the illusion that C1 is ready; 3 gates it.

---

## 4a. G3 drill-down — the from-genesis consensus-inputs gap

> **Question:** how hard is it to build a `LiveConsensusInputs` bundle for a
> from-genesis private net that the Haskell peer will agree with? **Answer:** it
> is mostly an *operator-procedure + one-time pinning* problem, not deep new BLUE
> code — **provided Ade produces inside a single epoch (epoch 0)**. The one fact
> that must be proven, not assumed, is the initial epoch nonce (eta0).

### What the bundle is and how Ade consumes it

The bundle (`consensus_inputs/json.rs:28` `RawConsensusInputs`) carries:
`network_magic, genesis_hash_hex, era, epoch_no, epoch_start_slot,
epoch_end_slot, active_slots_coeff{numer,denom}, epoch_nonce_hex,
pool_distribution{pool→active_stake}, pool_vrf_keyhashes{pool→hash32},
protocol_params_hash_hex, source_*`.

Only **three** of these feed leader-eligibility + header validation (the rest are
metadata):

| Field | Consumed by | Peer-side counterpart that must match |
|---|---|---|
| `epoch_nonce` (eta0) | `praos_vrf_input(slot, eta0) = blake2b256(slot_be8 ‖ eta0_32)` (`vrf_cert.rs:131`) → the VRF input the header proof certifies over | The peer's own epoch nonce at that slot, derived from its genesis + chain. **Byte-exact or the VRF proof fails → reject.** |
| `pool_distribution` + `active_slots_coeff` | `query_leader_schedule` → `stake_fraction=(pool_stake,total_stake)` + `asc` → leader threshold `1-(1-f)^σ` vs `praos_leader_value` (`leader_schedule.rs:101-123`, `vrf_cert.rs:162`) | The peer recomputes its **own** threshold from its genesis stake + ASC. Ade's must not over-claim relative to the peer's view. |
| `pool_vrf_keyhashes[pool]` | header validation binds the pool's VRF key | `blake2b256(vrf.vkey)` — deterministic from Ade's VRF key. |

### Two facts that are genuinely load-bearing

1. **eta0 has no offline derivation in Ade today.** Every eta0 in the tree is
   *operator-supplied* via `PraosChainDepState::genesis(consensus.epoch_nonce)`
   (`produce_mode.rs:192`, `admission/bootstrap.rs:207`); a grep finds **no**
   genesis→initial-nonce code anywhere. The Haskell rule for a Shelley/Conway-from-
   genesis net (initial nonce ≈ a hash of the genesis config, possibly combined
   with `extraEntropy`) is **not reimplemented** and must not be asserted from
   memory ([[feedback-proof-discipline]]).
2. **The importer does not check genesis-consistency.** Its validation
   (`importer.rs:159-229`) is purely structural — era=conway, epoch window sane,
   ASC denom≠0, pool keyset parity, hash widths. A *wrong* eta0/stake passes the
   importer cleanly and only fails at peer-accept. So the bundle is trusted; the
   correctness burden sits entirely on how it is constructed.

### The tractable path: extract-once at epoch 0 (reuse N-M-C), don't re-derive

The bundle was **designed to be cardano-cli-extracted** (`source_query_command`
field). The lowest-risk C1 construction reuses that:

- Bring the private Haskell peer up on the shared genesis as a **non-producing
  follower** (see topology note below), at epoch 0 before any block exists.
- Read the genesis-derived values straight off the fresh peer:
  - eta0 ← `cardano-cli query protocol-state` (epoch nonce) or the genesis-hash
    the node logs at startup — **this sidesteps reimplementing the Haskell nonce
    rule entirely.**
  - `pool_distribution` / `active_slots_coeff` ← `cardano-cli query stake-snapshot`
    / `stake-distribution` + the genesis `activeSlotsCoeff` (convert the decimal to
    an exact `numer/denom`).
  - `pool_vrf_keyhashes` ← `blake2b256(vrf.vkey)`.
- **Pin before forging** (the slice-entry proof obligation): assert Ade's
  `praos_vrf_input(slot, eta0)` and threshold inputs equal the peer's for the same
  slot/epoch, so the first forge is not a blind shot. This is the adversarial
  check that closes the "will the peer accept?" unknown cheaply.

So **G3 is principally: (a) a from-genesis extraction procedure + (b) a one-time
pinning harness**, plus the small structural items (decimal→fraction ASC,
hex/keyhash plumbing). Implementing an *offline* genesis→eta0 derivation is **not
recommended** — it reimplements a fiddly Haskell rule for no benefit when the
fresh peer can be queried once.

### Two constraints that make or break it

- **Single-epoch production window — VERIFIED a hard limit in code, not just a
  caution.** `praos_vrf_input` uses the per-epoch `epoch_nonce`, which the peer
  rolls at every boundary. Ade's apply path does **not** roll it: the BLUE nonce
  authority has all three transitions (`HeaderContribution`, `CandidateFreeze`,
  `EpochBoundary` — `consensus/nonce.rs:38-79`), but the per-header validator
  drives **only `HeaderContribution`** (`header_validate.rs:247`; no
  freeze/promotion, era_schedule not even consulted there). `self_accept` →
  `block_validity` → that validator is the whole produce apply path
  (`self_accept.rs:66-74`, `chain_evolution.rs:146-188`), so `ChainEvolution`'s
  `epoch_nonce` is **frozen at the seed value for the entire run**. The moment the
  chain crosses into epoch E+1, Ade keeps signing with the stale eta0 and the peer
  rejects every block. Cross-epoch production is therefore a **separate cluster**
  (drive `CandidateFreeze`/`EpochBoundary` from an epoch-aware tick + follow/
  durability), not a C1 concern. C1 must stay inside one epoch (epoch 0 cleanest).
- **Peer must be a follower, not a co-producer.** The existing `S1.md` runbook
  hands the Haskell node the **same** KES/VRF/opcert keys (`--shelley-kes-key`
  etc.) — that makes it a *second* producer of the *same* pool, which forks/double-
  forges. For C1 the Haskell node must run as a **relay/follower with no
  block-forging credentials**, peering from Ade, so Ade is the sole producer and
  the peer's job is purely to validate+accept. (Another concrete `S1.md` fix for
  the wiring cluster.)

### Joining at the *current tip* instead of genesis (the general case)

Epoch 0 is the easy entry, not the only one. To start Ade against an **already-
running** chain at its current tip *within the current epoch* (this is exactly
what the N-M-C extraction was built for — note the bundle's `source_tip_*` fields):

- Query the running peer for the **current** epoch's values: eta0 ←
  `cardano-cli query protocol-state`; stake ← `query stake-snapshot` /
  `stake-distribution`; tip (slot/hash/block_no) ← `query tip`.
- Seed Ade's chain-dep with that current eta0 and build on the tip. Ade forges
  within that epoch and the peer accepts (same eta0/stake/ASC). This is C2
  (preprod) and needs no genesis-from-scratch; it is the same single-epoch bundle,
  just sourced at the tip rather than at slot 0.

What you **cannot** do today is keep producing **across** the next boundary — see
the verified single-epoch limit above. Sustained tip-following production is a
distinct, larger cluster: an epoch-transition driver that fires `CandidateFreeze`
at the 8k/f stability slot and `EpochBoundary` promotion (the BLUE transitions
already exist; nothing drives them), fed by having accumulated **every** block's
nonce contribution in the epoch — which on a shared chain means following+applying
the peer's blocks too (receive path + durable state, overlapping N-U). That is the
real cost of "compatible with the live tip indefinitely," and it is out of C1/C2
scope.

### Bottom line for G3 (revises §4 item 3)

The new BLUE/GREEN code is modest: ASC decimal→fraction conversion, a from-genesis
bundle assembler (or just a documented cardano-cli procedure feeding the existing
importer), and a **pinning test** that cross-checks Ade's derived VRF-input +
threshold inputs against the live peer at epoch 0. The risk is concentrated in the
*pinning step*, not in volume of code — it is where the eta0 / stake-snapshot
agreement is proven. Treat item 3 as: **"from-genesis bundle + epoch-0 pinning
harness,"** scoped to a single-epoch window, peer-as-follower, with the eta0
agreement as the explicit slice-entry proof obligation.

## 4b. RE-SCOPE: tip path (C2/preprod) as primary target

Supersedes the genesis-centric §2/§4 framing. The deliverable is **a block
accepted at the live preprod tip, within one epoch** — bounty priority #3.

### Why the tip path is *more* work than genesis (not less)

The genesis/epoch-0 path hid two gaps because it starts from a trivial state
(`prev_hash = 0`, `block_no = 0`, `eta0 = genesis constant`, slot = 0). The tip
path must instead **graft onto a live chain**: real tip hash, real next block
number, real current-epoch nonce, and a slot that is plausibly *now*. That is G6 +
G7 on top of G1–G5.

### Re-scoped cluster invariants (tip-primary ordering)

1. **Tip-seeded produce bootstrap (G6 — load-bearing, new primary).** `produce_mode`
   MUST be able to start from an operator-supplied tip point (`--seed-point-slot` +
   `--seed-block-hash`, reusing the admission `seed_to_snapshot` authority) so the
   first forge is block #(tip_block_no + 1) on the peer's real tip hash. No genesis
   cold-start on the tip path; `block_number` derives from the seeded tip, not `0`.
2. **Current-epoch consensus-inputs at the tip (G3, reframed).** Reuse the **proven**
   N-M-C extraction (`import_live_consensus_inputs`) against the synced preprod peer
   — `query protocol-state` (current eta0), `query stake-snapshot` (current go-snapshot),
   `query tip`. This already worked for admission (operator-pass transcript, slot
   124140368); the produce direction consumes the same bundle. The from-genesis
   constructor is **demoted to the C1 rehearsal only**.
3. **Current-slot alignment (G7).** Forge at a slot tracking the peer's live slot;
   correct absolute-slot↔wall-clock map from the real genesis; do not swallow
   `SlotDrift` on the tip path.
4. **Real cardano-cli opcert + genesis ingress (G1+G2).** Wire the dormant
   `opcert_envelope::parse_opcert_envelope` + `genesis_parser::parse_shelley_genesis`;
   on preprod the genesis is the real one (network magic, slot length, KES params).
5. **Pin protocol version + opcert counter to live values (G4).** `protocol_version`
   (preprod major 10/11, not hardcoded 9) + `prev_opcert_counter` from the operator's
   registered opcert, derived from genesis/opcert — not defaults.
6. **Epoch-0 pinning harness → current-epoch pinning (proof obligation).** Before
   forging, assert Ade's `praos_vrf_input(slot, eta0)` + threshold inputs equal the
   peer's for the **current** epoch/slot.
7. **Evidence ↔ acceptance correlation fix (G5)** + doc/registry hygiene (stale
   comments + open_obligation text).

C1 (genesis rehearsal) needs only items 4–7 + the from-genesis bundle; C2 (the
graded target) additionally needs items 1–3.

### Operator precondition (time gate, not code)

C2 still requires **preprod stake provisioned for the operator's cold key**:
register the pool, delegate ADA, wait ~2 epochs (~10 days on preprod) for the
snapshot to go active. Until then C2 is `blocked_until_preprod_stake_available`
(per N-S-C/S2.md §1). C1 can rehearse the engineering meanwhile.

## 4c. Security ramifications of starting from the tip

Starting production from a peer-supplied tip means **Ade trusts an external party
for its base state**, which is a different (larger) trust surface than genesis.
This is acceptable *only* because the peer is the bootstrap **oracle**, not the
runtime authority ([[feedback-oracle-seed-then-ade-owns]]) — but the boundary must
be enforced, not assumed. Concrete ramifications:

1. **Poisoned-seed / false base state.** Genesis is a fixed, publicly-verifiable
   constant; a *tip* (UTxO dump + consensus-inputs + tip point) is whatever the
   operator/peer hands over. A wrong or malicious bundle (inflated stake, wrong
   eta0, fabricated UTxO, off-tip hash) is **not caught by the importer** — its
   validation is purely structural (`importer.rs:159-229`: era/window/ASC-denom/
   keyset parity, no genesis-consistency check). Mitigation: the bundle must be
   bound to a verifiable anchor — `genesis_hash` + `source_tip_hash` cross-checked
   against the peer's `query tip`, and ideally a **Mithril-verified** snapshot as
   the UTxO source ([[feedback-mithril-is-peer-infra-not-ade-authority]] — Mithril
   is operator infra for *obtaining* a trustworthy snapshot, never a BLUE trust
   root). The pinning harness (item 6) is the security gate, not just a
   compatibility check.

2. **Stake / leadership over-claim (forge-side).** If Ade's bundle over-states its
   pool's active stake, Ade *believes* it is leader more often and forges blocks the
   peer's lower threshold rejects — harmless to the network (peer rejects) but it
   **fabricates false "I was leader" evidence** if the evidence log trusts Ade's own
   leader-check. Mitigation: leadership in the evidence manifest is only ever proven
   by the **peer's** acceptance log, never Ade's self-assessment
   ([[feedback-shell-must-not-overstate-semantic-truth]]).

3. **KES / opcert misuse window.** Forging at the live tip uses the **real hot KES
   key** over real header pre-images at the current period. Risks: (a) signing at a
   wrong/rotated KES period mints an opcert-counter or period mismatch the peer
   rejects — and worse, a careless counter could **burn the opcert sequence** (the
   peer remembers the highest counter; re-using or skipping strands the pool).
   (b) the hot key is in RED custody (`producer_shell`, redacted Debug, zeroizing
   Drop — DC-CRYPTO-03/05) but a live run puts it on a network-connected host.
   Mitigation: derive `prev_opcert_counter` from the registered opcert (G4); never
   reuse a counter across runs; keep the cold key off the producing host.

4. **Equivocation / slot-battle risk (the sharp one).** If Ade forges on the **same
   pool keys** as a node that is *also* producing (e.g. the operator left the
   Haskell peer in block-producing mode, or a prior Ade run), two different blocks
   get signed for the same slot with the same VRF/KES — that is **equivocation**,
   which real nodes detect and which can get a pool flagged/adversarially reported
   during the bounty's hostile test window. Mitigation: exactly **one** producer per
   pool key at a time; on the tip path the Haskell peer must be a **follower/relay
   with no forging credentials** (the same fix flagged for the C1 runbook), and no
   two Ade instances share the KES key.

5. **Fork / chain-quality impact.** Building on the live tip means Ade's block enters
   the **real** preprod fork-choice. A malformed or late block is rejected (safe),
   but a block that is valid-but-on-a-stale-parent (G7 drift, or building on an
   ancestor after a rollback) can briefly **create a competing fork** the peer must
   resolve. On preprod this is low-stakes, but it is real network behavior, not a
   sandbox. Mitigation: forge only on the *current* tip (re-read tip immediately
   before forging), bounded single-block / single-epoch smoke runs, and abort on
   `SlotDrift` rather than swallowing it (G7).

6. **No durability ⇒ unsafe restart.** The produce path has **no WAL/snapshot**
   (N-U deferred; `produce_mode` MUST NOT write durability — N-T prohibition). If
   Ade crashes mid-run and restarts from a re-extracted tip, it can re-enter at a
   slot/counter it has already used → equivocation (risk 4) or counter reuse (risk
   3). Mitigation: treat each tip-path run as single-shot until N-U lands durable
   produce state; do not auto-restart a producer from a stale seed.

**Net:** the security cost of the tip path is the **seed-trust boundary** — Ade is
only as safe as the bundle it is handed, and the importer does not vet it. The
defenses already exist in doctrine (oracle-not-authority, shell-must-not-overstate,
Mithril-as-infra, fail-closed, key-custody-in-RED); the cluster's job is to make
them **mechanical** for the tip seed: bind the bundle to a verifiable tip/genesis
anchor, prove leadership only via the peer log, enforce one-producer-per-key, and
gate on the pinning harness before any real KES signature is emitted.

## 5. Hard cautions carried (unchanged)

- Scope is the scoping pass — **do not run the live nodes** until green-lit.
- Live acceptance is proven only by the committed **peer** validation log
  (`acceptance_keyword_match` over the raw `peer.log`); wire success ≠ admission ≠
  accept ([[feedback-shell-must-not-overstate-semantic-truth]]).
- C1 is a real Cardano private testnet (real ledger+consensus validation on the
  peer), not a mock; C1 only removes stake provisioning, not the validity bar.
- C1 ≠ the bounty deliverable; C2 (preprod) is the public surface
  ([[feedback-bounded-smoke-slices]], [[project-bounty-requirements]]).
- Never commit private keys / genesis secrets ([[feedback-no-credential-leaks]]).
