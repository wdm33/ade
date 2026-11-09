# PHASE4-N-F — Operator-Production Wiring · Invariant Sketch

> **Status:** invariants-first sketch (IDD Part I). No code, no cluster/slice plan, no
> registry promotion, no cluster close. Two load-bearing scope questions (OQ2, OQ3) are
> **resolved below** before this artifact was saved.
> **Concept-slug:** `phase4-n-f`

## Guardrail (self-fenced)

PHASE4-N-F is **BA-02 operator-production wiring only**. It consumes an **Ade-derived
selected tip** from the verified bootstrap / forward-sync / WAL-recovery spine. It does
**not** satisfy BA-01 sync, BA-03 N2C, BA-04 private testnet, or BA-09 recovery. Any
seed-point/tip-graft is **diagnostic/fixture-only** unless backed by the sync/recovery
pipeline.

**The chain every invariant preserves:**
`verified bootstrap / forward-sync / WAL recovery → Ade-derived selected tip → produce-mode block construction → Haskell peer acceptance evidence`

**Central reframe.** Today `produce_mode` cold-starts from an empty `InMemoryChainDb`
→ `bootstrap_initial_state` cold branch → `tip=None`, `block_no=0` (C1 scoping §1d/G6).
The C1 doc's G6 proposed reusing the admission `seed_to_snapshot` *operator tip bundle*.
**Under this guardrail that bundle path is diagnostic-only.** PHASE4-N-F makes produce
consume the tip Ade itself reached via the spine (the persistent ChainDb the forward-sync
pump wrote and recovery reconciled), through the single `bootstrap_initial_state` warm
path — never a hand-fed bundle as the bounty-primary base.

---

## Resolved scope decisions

### RD-OQ2 — consensus inputs come from Ade-derived state, not the operator bundle
eta0 / stake / ASC / chain-dep inputs for **bounty-primary** produce MUST come from the
synced/recovered Ade-**selected** state. The N-M-C operator `--consensus-inputs-path`
bundle is **diagnostic / pinning-only** in PHASE4-N-F and **cannot be BA-02 bounty-primary
input**. (Inverts the C1 scoping's G3; closes the regression path "operator hands us a
tip+nonce bundle, then we forge.")

### RD-OQ3 — single-shot until N-U forged-block durability lands
PHASE4-N-F is **single-shot** until N-U forged-block durability exists. It may make **one
explicit BA-02 production attempt** per explicit operator run from an Ade-derived selected
tip. Restart **after signing / after emitting a forge attempt must fail closed for
production** unless N-U forged-block WAL/durability is present. **No** automatic retry,
**no** repeated signing, **no** "resume produce" semantics, **no** restart-safe production
claim, and **no** BA-09 claim.

---

## 1. What must always be true

- **NF-1 — Produce builds on the Ade-derived selected tip, including its consensus inputs.**
  The forge base — parent hash, `block_no`, ledger state, chain-dep state, **epoch
  nonce/eta0, stake, ASC** — is the selected tip of Ade's own persistent ChainDb: the store
  the forward-sync pump wrote (DC-SYNC-01) and recovery reconciled to the WAL tail
  (T-REC-01/02). `parent = selected_tip.hash`, `block_no = selected_tip.block_no + 1`;
  eta0/stake/ASC derive from that tip's chain-dep + ledger state — **not** from an operator
  bundle (RD-OQ2). *(Derived, Cardano-specific. Strengthens CN-NODE-01; depends on
  DC-SYNC-01, T-REC-01/02, DC-STORE-05. Closes G6 guardrail-correct.)*
- **NF-2 — Produce layers after the spine, through the single bootstrap authority.**
  Produce obtains initial state **only** via `bootstrap_initial_state` (warm path fed by
  the recovered/synced ChainDb) — never a parallel storage-init or a second source of
  truth. *(Derived. Strengthens CN-NODE-01 / CN-PROD-03; the meta-invariant encoding the
  guardrail.)*
- **NF-3 — Operator-file ingress uses real Cardano formats.** `--opcert` parses via
  `opcert_envelope::parse_opcert_envelope` (cardano-cli text envelope, CN-OPCERT-01);
  `--genesis-file` via `genesis_parser::parse_shelley_genesis` (CN-GENESIS-01). The
  `parse_simple_*` stand-ins are retired from the produce path. *(Derived. Strengthens
  CN-OPCERT-01 / CN-GENESIS-01; closes G1/G2.)*
- **NF-4 — Header inputs are live, not defaults.** `protocol_version`, `prev_opcert_counter`,
  `pparams`, and the VRF/KES context derive from the validated Ade-derived state + the
  operator's real opcert/genesis — never `ProtocolParameters::default()`, never hardcoded
  `major:9`, never `prev_opcert_counter: None`. *(Derived. Strengthens CN-KES-HEADER-01 /
  CN-FORGE-03; closes G4.)*
- **NF-5 — The forge slot is an explicit canonical input; BLUE never reads the clock.**
  RED observes wall-clock and maps it to an absolute slot via the real genesis
  (`slot_zero_time + slot_length`); BLUE/forge receives the slot as canonical input.
  *(True/Derived split. Strengthens T-CONS-02 / DC-CORE-01 / DC-NODE-03; closes G7 BLUE side.)*
- **NF-6 — BA-02 is single-epoch and the boundary is explicit.** Produce forges only within
  the epoch of its selected tip's eta0; the per-epoch nonce roll is not driven on the
  produce apply path today, so a boundary crossing is a defined, surfaced, fail-closed
  condition — never a silent stale-eta0 forge. *(Derived. Relates CN-FORGE-04.)*
- **NF-7 — BA-02 acceptance is proven only by the peer.** The sole proof an Ade-forged
  block was accepted is the Haskell peer's own validation/accept log
  (`acceptance_keyword_match`), correlated to the forged block hash, per
  CN-OPERATOR-EVIDENCE-01. Ade's own self-accept/leader-check is never acceptance or
  leadership evidence. *(Release. Strengthens RO-LIVE-01 / CN-CONS-06 live half /
  CN-OPERATOR-EVIDENCE-01.)*
- **NF-8 — Single-shot, non-restartable until N-U (RD-OQ3).** At most one explicit BA-02
  production attempt per operator run, from an Ade-derived selected tip. After a forge
  attempt is signed/emitted, the producer is latched: (re)start must fail closed for
  production unless N-U forged-block durability is present. No automatic retry, no repeated
  signing, no resume-produce. *(Derived/operational. Relates CN-PROD-03 / DC-PROD-03 (N-T/N-U
  durability); does NOT claim BA-09.)*

## 2. What must never be possible

- **¬NF-1:** forge on a genesis/zero `prev_hash` with `block_no=0` when a synced/recovered
  tip exists; **source eta0/stake/ASC from the operator `--consensus-inputs-path` bundle as
  BA-02 bounty-primary input** (RD-OQ2); build the bounty-primary base from a hand-fed bundle.
- **¬NF-2:** a produce-path parallel storage-init / second bootstrap authority; produce
  mutating the spine's authoritative tip outside the bootstrap warm path.
- **¬NF-3:** produce parsing opcert/genesis via `parse_simple_*` stand-ins or any format
  real cardano-cli doesn't emit.
- **¬NF-4:** forging a header with default protocol params / fabricated `prev_opcert_counter`;
  **reusing or skipping an opcert counter** (burns the pool's opcert sequence; equivocation vector).
- **¬NF-5:** BLUE reading wall-clock; **silently swallowing `SlotDrift`** (current
  `produce_mode.rs:327-331`); forging for a future-beyond-tolerance or stale slot.
- **¬NF-6:** continuing to sign with a **stale eta0** across an epoch boundary.
- **¬NF-7:** emitting BA-02 evidence from Ade's own self-assessment; conflating wire success /
  peer fetch with acceptance; a **diagnostic graft producing bounty evidence**, claiming sync
  (BA-01), or writing durability as if synced.
- **¬NF-8:** automatic retry; repeated signing across runs without durable proof of the prior
  attempt; restart-safe / "resume produce" semantics; any BA-09 recovery claim from this cluster.

## 3. What must remain identical across executions (deterministic surface)

Given a **fixed selected tip**, a **fixed canonical slot** (and sequence), **fixed operator
artifacts** (opcert/genesis/KES/VRF/cold keys), and the **consensus inputs derived from that
tip**, the following are byte-identical run-to-run: the forged block bytes, the unsigned-header
pre-image, the VRF proof/output (ECVRF is deterministic given `sk‖alpha`), the KES signature
(deterministic given `sk‖period‖msg`), the `LeaderCheckVerdict`, and the `self_accept` verdict.
**The concept *does* express as a pure transformation** `canonical input → canonical output`.
The only nondeterminism (wall-clock; the peer's behavior) is confined to RED and enters BLUE
only as the canonical slot (NF-5) or stays out of the authoritative path entirely (peer accept
is release evidence, NF-7).

## 4. What must be replay-equivalent

Replaying the same `(selected tip, canonical slot sequence, operator artifacts, tip-derived
consensus inputs)` produces **byte-identical forged blocks** and a **byte-identical
`ProducerLogEvent` stream** (filtered to the replayable vocabulary — extends DC-PROD-02). The
selected-tip *derivation itself* is already replay-equivalent through the spine (DC-SYNC-01
durable-before-tip, T-REC-01/02 recovery replay-equivalence), so the whole chain
`spine → tip → forge` is replay-stable.

## 5. State transitions in scope

- `select_forge_base(synced/recovered ChainDb selected tip, era schedule) → Result<ForgeBase{parent, block_no, ledger, chain_dep, eta0, stake, ASC}, BaseError>` — no clock, no graft, consensus inputs tip-derived (NF-1, RD-OQ2).
- `ingest_operator_files(opcert_path, genesis_path) → Result<(OperationalCert, GenesisAnchor), ParseError>` — real parsers (NF-3).
- `align_slot(wallclock_observation [RED], genesis time-map) → Result<CanonicalSlot, SlotDrift>` — RED→canonical (NF-5).
- `epoch_guard(ForgeBase.eta0_epoch, CanonicalSlot) → Result<(), CrossEpochUnsupported>` — fail-closed at the boundary (NF-6).
- `produce_run_guard(persisted prior-attempt state | none, N-U durability present?) → Result<ProductionPermitted, RestartFailClosed>` — on (re)start after a prior forge attempt without N-U durability → fail closed (NF-8, RD-OQ3).
- `run_real_forge(ForgeBase, CanonicalSlot, keys, live protocol inputs) → Result<ForgeSucceeded(AcceptedBlock) | ForgeNotLeader | ForgeFailed, ForgeError>` — existing BLUE-then-RED-then-BLUE pipeline, now fed an Ade-derived base + live inputs; emits at most one attempt then halts production (NF-1/NF-4/NF-8).
- `correlate_ba02_evidence(forged_block_hash, peer_accept_log) → Result<BA02Evidence, EvidenceError>` — peer-acceptance-only (NF-7).

## 6. TCB color hypothesis

- **BLUE (existing, reused):** `leader_check`, `unsigned_header_pre_image`, `forge_block`,
  `self_accept`, `genesis_initial_state`, the `vrf_cert` leader-input authority. The canonical
  **slot** is a BLUE input.
- **GREEN:** the `select_forge_base` reducer (pure over a ChainDb read — mirrors
  `forward_sync::reducer`); the producer coordinator; the `produce_run_guard` reducer; the BA-02
  evidence-correlation reducer (closed vocabulary).
- **RED:** operator-file **I/O** (reading bytes off disk); the wall-clock observation; KES/VRF
  custody + signing; the persistent ChainDb/spine read; the peer-log read.
- **Parsing classification (wording correction).** Parsing real Cardano operator files is **not
  inherently RED**. File I/O is RED; the **deterministic parse/verdict authority** is BLUE or
  GREEN, classified by where the parser lives and what it does — the byte layout is the
  validator (mirroring the established `load_kes_signing_key_skey` [RED read] →
  `Sum6Kes::raw_deserialize_signing_key_kes` [BLUE deserializer] split). Parse/verdict behavior
  must not be treated as shell discretion.
- **Open colors:** (a) where `select_forge_base` sits — GREEN reducer over a RED ChainDb read
  (the `reducer`/`pump` split, hypothesis) vs. absorbed into the `bootstrap_initial_state` warm
  path. (b) `parse_opcert_envelope` / `parse_shelley_genesis` currently live in
  `ade_runtime::producer::*` (RED by location); per the parsing-classification note above their
  deterministic parse/verdict authority is a candidate for a BLUE/GREEN split (RED read → BLUE/GREEN
  verdict). Resolve in cluster-plan.

## 7. Remaining open questions (for cluster-plan)

1. **Handoff shape:** does PHASE4-N-F presuppose the spine has *already* produced a recovered
   ChainDb (produce = "start from a recovered tip" mode), or does it sequence sync→produce in one
   process? The guardrail + NF-1 imply the former — needs an explicit handoff contract (how
   produce obtains the recovered selected tip + post-state).
2. **Epoch-boundary detection source:** confirm the boundary is detectable from the selected tip's
   chain-dep (`epoch_nonce` + schedule) so NF-6 is expressible without driving the (absent) nonce roll.
3. **Diagnostic-graft fence mechanism:** how the diagnostic seed/graft path (and the diagnostic-only
   operator consensus bundle, RD-OQ2) is *structurally* prevented from emitting BA-02 evidence,
   claiming sync, or writing durability-as-synced — a distinct evidence label + a CI containment gate,
   mirroring the N-Z `ci_check_mithril_seed_point_independence.sh` data-flow-resistant approach.
4. **Parser color split:** whether NF-3 also entails promoting the deterministic parse/verdict of
   `parse_opcert_envelope` / `parse_shelley_genesis` out of RED-by-location (see §6 open color b).

*(OQ2 and OQ3 from the prior draft are now RD-OQ2 / RD-OQ3 above — resolved.)*

---

## Candidate registry entries (proposed only — NOT appended; promotion is a later step)

Most of PHASE4-N-F **strengthens existing rules**; a few new entries are needed. Exact
per-family numbers are placeholders pending the next free number at promotion.

| Candidate | New / Strengthen | Tier | One-line |
|---|---|---|---|
| `CN-PROD-05` (new) | new | derived/constraint | Produce forge base = the Ade-derived selected tip, including tip-derived eta0/stake/ASC (NF-1, RD-OQ2); never a hand-fed bundle. |
| `DC-PROD-04` (new) | new | derived | Produce header inputs (protocol_version / opcert counter / pparams) derive from validated state + real operator artifacts, not defaults (NF-4). |
| `DC-PROD-05` (new) | new | derived | Produce slot is an explicit canonical input; `SlotDrift` fail-closed; BLUE reads no clock (NF-5). |
| `CN-PROD-06` (new) | new | constraint | Diagnostic seed/tip-graft **and the operator consensus-inputs bundle** are structurally fenced: no BA-02 evidence, no sync claim, no durability-as-synced (NF-7/¬NF-7, RD-OQ2). |
| `DC-PROD-06` (new) | new | derived | Single-epoch produce; cross-epoch boundary is fail-closed, never a stale-eta0 forge (NF-6). |
| `DC-PROD-07` (new) | new | derived/operational | Single-shot, non-restartable until N-U: one BA-02 attempt per run; restart-after-signing fails closed; no auto-retry/resume; no BA-09 claim (NF-8, RD-OQ3). |
| `CN-OPCERT-01`, `CN-GENESIS-01` | strengthen | — | wired into produce (`strengthened_in += PHASE4-N-F`); closes G1/G2 (NF-3). |
| `CN-NODE-01` | strengthen | — | produce warm-path routes through the single bootstrap authority off the Ade-derived tip (NF-1/NF-2). |
| `RO-LIVE-01`, `CN-CONS-06`, `CN-OPERATOR-EVIDENCE-01` | strengthen | release | BA-02 peer-acceptance evidence is the live-half target (NF-7). |
| `T-CONS-02` | strengthen | true | produce honors no-wall-clock-in-authoritative-decisions (NF-5). |

---

## Recommendation (prose)

With RD-OQ2 and RD-OQ3 folded in, the sketch is guardrail-tight: **NF-1 + NF-2 + RD-OQ2** force
produce to consume the Ade-derived selected tip — *including its consensus inputs* — through the
single bootstrap authority, closing the regression path where an operator bundle becomes the
forge base. **NF-8 + RD-OQ3** keep the cluster honest about durability: it can wire BA-02
acceptance evidence but explicitly does not claim BA-09 recovery or restart-safe production.
The mechanical wiring (NF-3/NF-4/NF-5) largely strengthens already-enforced rules.

Remaining open questions are now genuinely cluster-plan-shaped (handoff contract, boundary
detection source, the diagnostic-fence mechanism, the parser color split) rather than
scope-defining — none can reintroduce the tip-graft model. Ready for `/cluster-plan` once these
are addressed there; **not** before.
