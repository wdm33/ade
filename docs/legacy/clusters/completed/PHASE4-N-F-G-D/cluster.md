# Cluster PHASE4-N-F-G-D — Private-testnet accepted-block bounty dry-run

> **Status: CLOSED** (S1–S4 merged; `CN-REHEARSAL-FIDELITY-01` enforced; IDD + per-cluster security reviews PASS; 4 grounding docs refreshed; no RO-LIVE flip) (from the committed plan `docs/planning/phase4-n-f-g-d-cluster-slice-plan.md` `bc39d431` + the `/invariants` sketch `docs/planning/phase4-n-f-g-d-invariants.md` `07e6a340`). Follow-on sub-cluster of **PHASE4-N-F-G** (RO-LIVE-01). Predecessors: G-A forge fidelity (`62cb8718`), G-B serve handoff (`febee120`), G-C live feed + operator-gated evidence (`351d46bc`), G-E live-feed bounded memory (`da205bff`). Code-verified at HEAD `bc39d431`.
>
> **Cluster character (load-bearing — do not broaden):** G-D is a **bounty dry-run harness**, not a private-net product path. Its sole purpose is to be a **fast failure detector for the *exact* preview/preprod accepted-block path**, run in a venue where the operator controls stake (private genesis) so Ade wins slots in minutes instead of waiting days for public-testnet leader slots. It answers one question: *will a real Haskell node accept an Ade-forged block when Ade has legitimate leader rights?* — surfacing the bounty-blocking failure classes (opcert/KES, Praos leader proof, header fields, block body/wire, tag-24 BlockFetch payload, serve path, peer-log parsing) early. **Two halves with sharply different IDD status** (mirrors G-C): (1) a **MECHANICAL half** (closeable hermetically) — the path-fidelity proof + fence, and the distinct, non-promotable rehearsal-evidence envelope + gate; (2) an **OPERATOR-GATED half** (stays `blocked_until_operator_c1_net_executed`) — the actual C1 live execution.
>
> **Hard lines (any one breached → stop and re-scope):**
> - **N0 — no private-only shortcut.** No private-only flag/branch/bootstrap authority/from-genesis constructor/assumption. **No private-only helper may make the rehearsal pass if the same condition would fail on preview/preprod** — such a condition is a *shared-path bug to fix in the shared path*, never special-cased. **Litmus test: every G-D element must transfer verbatim to preview/preprod.**
> - Must **NOT relax** `ci_check_node_run_loop_containment.sh`, `ci_check_served_chain_handoff_fence.sh`, or `ci_check_live_feed_memory_bounds.sh` (all byte-unchanged).
> - **No synthetic manifest.** Acceptance is proven ONLY by the operator-captured Haskell peer log through `ba02_evidence::correlate`.
> - **No RO-LIVE flip.** Private C1 acceptance ≠ bounty completion; preview/preprod acceptance = completion. No new BLUE authority / canonical type / `--mode node` argv flag / from-genesis constructor.

## Primary invariant
**`CN-REHEARSAL-FIDELITY-01`** (declared; `introduced_in = PHASE4-N-F-G-D`) — two coupled clauses: **(1) path fidelity** — the C1 private dry-run exercises the identical `--mode node` accepted-block path as preview/preprod (`import_live_consensus_inputs` extraction/import → forge → self-accept → sibling-serve → block-fetch → peer log → `correlate`), with no private-only flag/branch/bootstrap/from-genesis constructor; **(2) evidence non-promotability** — any private-testnet manifest wraps the correlate-produced `Ba02Manifest` payload in a distinct rehearsal envelope, stored only under the rehearsal home, sha256-bound, correlate-produced, flipping no RO-LIVE rule. *(Cited, not restated — see the registry entry.)*

## Invariants strengthened / carried (at close)
- **`CN-REHEARSAL-FIDELITY-01`** — flips `declared → enforced` (tests + `ci_script` populated by S1/S2/S3).
- **Carried unchanged (not weakened):** `DC-NODE-06` (G-B serve handoff), `DC-EPOCH-03` (single-epoch forge fail-closed), `DC-LIVEMEM-01` (live-feed bounded memory), `CN-NODE-02` (single live-run lifecycle owner), `DC-SYNC-01`/`DC-SYNC-02` (single durable tip-advance), `CN-CINPUT-03`/`DC-CINPUT-02b` (the shared consensus-inputs extraction/consume path), `CN-FORGE-04`/`CN-WIRE-08` (the forge + tag-24 authorities).
- **Deliberately NOT strengthened:** `RO-LIVE-01` / `RO-LIVE-06` / `CN-OPERATOR-EVIDENCE-01` receive **no `strengthened_in += PHASE4-N-F-G-D` bump** — a bump would wrongly imply G-D advanced the bounty deliverable. G-D `cross_ref`s them and records the decoupling.

## Normative anchors
- `docs/planning/phase4-n-f-g-d-cluster-slice-plan.md` — the G-D plan (1 cluster, 3 CEs, 3 slices).
- `docs/planning/phase4-n-f-g-d-invariants.md` — the `/invariants` sketch (A0 path fidelity; A1–A6; N0–N7; OQ1 proof obligation).
- `docs/planning/operator-pass-live-leg-c1-scoping.md` — the C1 scoping pass (mined for the failure-class surface + the §4a single-epoch / peer-as-follower / extract-once findings; §4b re-scope: C1 = rehearsal, C2 = graded).
- `docs/evidence/phase4-n-f-g-c-operator-pass-README.md` — the `--mode node` preprod operator-pass runbook the C1 dry-run runbook must be a **provable strict subset of**.
- Registry: `CN-REHEARSAL-FIDELITY-01`, `RO-LIVE-01`, `RO-LIVE-06`, `CN-OPERATOR-EVIDENCE-01`, `DC-NODE-06`, `DC-EPOCH-03`, `DC-LIVEMEM-01`, `CN-NODE-02`, `CN-CINPUT-03`/`DC-CINPUT-02b`.

## Entry conditions (what prior clusters guarantee)
- **G-A (closed):** the `--mode node` forge produces a genesis-consistent, slot-aligned, epoch-bounded **self-accepted** artifact with real opcert/genesis ingress + current pparams (`DC-EPOCH-03` enforced). The `ade_testkit::consensus::genesis_pinning` harness exists for the pre-sign consistency pin.
- **G-B (closed, `febee120`):** `DC-NODE-06` enforced — a self-accepted artifact reaches the served chain via the typed `SelfAcceptedHandoff` + the sibling `push_atomic` task; relay-loop body forwards a typed channel send only.
- **G-C (closed, `351d46bc`):** the `--mode node` `On` arm is live-feed-wireable from `--peer`; `ba02_pass::correlate_peer_log_file` (`ba02_pass.rs:38`) + `write_ba02_manifest` (`ba02_pass.rs:52`, accepts only a `Ba02Manifest`) wrap the GREEN `correlate` (`ba02_evidence.rs:290`, sole `Ba02Manifest` ctor); the BA-02 schema gate `ci_check_ba02_evidence_manifest_schema.sh` is vacuous-until-committed + sha256-bound; the preprod operator-pass runbook exists.
- **G-E (closed, `da205bff`):** peer-driven live-feed memory is bounded before authoritative decode (`DC-LIVEMEM-01`); `ci_check_live_feed_memory_bounds.sh` green.
- **N-M-C (shared extraction/import — RED/GREEN surface, NOT BLUE):** `import_live_consensus_inputs` (`consensus_inputs/canonical.rs:112`) is the **single** consensus-inputs importer; the `--mode node` path consumes it at `node_lifecycle.rs:1081` (same function preprod uses). This is the path-fidelity hinge — it is a shell/glue import surface, not consensus authority.

## Verified component inventory (read at HEAD `bc39d431`, not assumed)
| Component | Real state (verified) | Use |
|---|---|---|
| `import_live_consensus_inputs` (`consensus_inputs/canonical.rs:112`), consumed by the node path at `node_lifecycle.rs:1081` | the **single** consensus-inputs extraction/import (RED/GREEN surface); same function `produce_mode.rs:188` uses | **S1** the shared path the C1 dry-run must use unchanged (OQ1 hinge) |
| `ba02_evidence::correlate` (`ba02_evidence.rs:290`) + `Ba02Manifest` (`ba02_evidence.rs:180`) | GREEN; **sole** `Ba02Manifest` constructor; allow-list / hash-primary | **S2/S3** the rehearsal payload producer — reused unchanged |
| `ba02_pass::{correlate_peer_log_file (:38), write_ba02_manifest (:52)}` | RED; `write_ba02_manifest` accepts only a `Ba02Manifest` | **S2** the model the rehearsal-envelope I/O wrapper mirrors |
| `ade_testkit::consensus::genesis_pinning` (G-A S1, `#[cfg(test)]`) | GREEN; pins recovered values vs the genesis-derived reference | **S3** the pre-sign consistency pin (over the N-M-C-extracted bundle) |
| `docs/evidence/phase4-n-f-g-c-operator-pass-README.md` | the `--mode node` preprod operator-pass runbook | **S3** the runbook the C1 dry-run runbook is a strict subset of |
| `ci_check_ba02_evidence_manifest_schema.sh` | the bounty-manifest schema gate (vacuous-until-committed, sha256-bound) | **S2** must NOT match the rehearsal home (non-promotability cross-check) |
| `ci_check_node_run_loop_containment.sh`, `ci_check_served_chain_handoff_fence.sh`, `ci_check_live_feed_memory_bounds.sh` | the containment / handoff / memory fences | **UNCHANGED** by G-D (hard line) |
| `--mode node` argv (`cli.rs`): `--peer`, `--network-magic`, `--json-seed`, `--consensus-inputs-path`, operator key flags | the closed operator-input flag set | **S1** the fence asserts G-D adds **no** new flag |

## Slices (safety order)

### S1 — Path-fidelity proof + fence *(mechanical; CE-G-D-1)*
Prove the `--mode node` accepted-block path is **input-driven and venue-agnostic**, and fence it. **Slice-entry proof obligation (OQ1):** verify in code that `import_live_consensus_inputs` (`canonical.rs:112`) consumes an **early/private-net extraction** (epoch-0 / fresh-peer `query protocol-state` + `query stake-snapshot` shape) through the **same** path used for a synced preprod-tip extraction. **If it cannot, S1 fixes the shared extraction/import path — never a private-only workaround (N0).** Add the path-fidelity CI fence: (a) the `--mode node` argv flag set is unchanged (G-D adds no new flag — diff against the closed set); (b) no from-genesis consensus-inputs constructor exists (negative grep — the only populator of the node forge base's consensus inputs is `import_live_consensus_inputs`). Addresses **CE-G-D-1**. TCB: **RED** (CI fence) + a GREEN/RED transfer-fidelity test over the shared importer.

### S2 — Rehearsal-evidence surface + gate *(mechanical; CE-G-D-2)*
Introduce the rehearsal envelope: a thin type that **wraps the correlate-produced `Ba02Manifest` payload** and carries a **distinct rehearsal envelope** (closed `venue = "private-testnet-c1"` + `is_rehearsal` marker) — structurally distinct from, and non-promotable to, the bounty manifest. Add the RED I/O wrapper (mirrors `ba02_pass`) that writes **only** a correlate-produced rehearsal envelope to the rehearsal home `docs/evidence/phase4-n-f-g-d-private-rehearsal-*.toml`. Add a new vacuous-until-committed CI gate that, when a rehearsal envelope is committed, verifies the closed schema + `is_rehearsal`/`venue` marker + sha256-binds the committed peer log + **forbids** the rehearsal envelope under the bounty home (and a `Ba02Manifest` under the rehearsal home), and cross-checks that `ci_check_ba02_evidence_manifest_schema.sh` does **not** match the rehearsal home. A `NoEvidence` correlate outcome writes nothing (fail closed). Addresses **CE-G-D-2**. TCB: **GREEN** (envelope type, if pure) + **RED** (I/O wrapper reusing `correlate`) + **CI** (the new gate).

### S3 — C1 dry-run runbook + operator-gated execution scaffolding *(operator-gated; CE-G-D-3)*
Commit the C1 dry-run runbook `docs/evidence/phase4-n-f-g-d-private-rehearsal-README.md` as a **provable strict subset** of the preprod operator-pass runbook (`phase4-n-f-g-c-operator-pass-README.md`), differing **only** in venue (operator-authored private genesis stake → fast slots; follower peer with no forging creds, one-producer-per-key; the bundle **extracted via `import_live_consensus_inputs`** exactly as preprod) and the rehearsal label; pin via `genesis_pinning` before any live KES signature. Wire the rehearsal-envelope I/O end-to-end and exercise it on a **hermetic** fixture (mechanics-only, never committed under the evidence home). The actual C1 live execution stays `blocked_until_operator_c1_net_executed` — **named, not deferred**; flips no RO-LIVE rule. Addresses **CE-G-D-3**. TCB: **RED** (runbook / evidence I/O wiring) + **GREEN** (`correlate`, reused).

## Exit criteria (mechanical, CI-verifiable)
New test/check names are **candidate** (created by the owning slice); existing artifacts named as-is.

- **CE-G-D-1 (path fidelity — MECHANICAL, closeable)** — a candidate test `node_accepted_block_consensus_inputs_via_shared_import` proves the `--mode node` forge base's consensus inputs are populated **only** via `import_live_consensus_inputs`, for **both** a private/early-net-shaped extraction and a synced-tip-shaped extraction (OQ1 proof: same path, both shapes); a candidate CI fence `ci_check_node_path_fidelity.sh` is green: (a) no new `--mode node` argv flag vs the closed set, (b) no from-genesis consensus-inputs constructor (negative grep — `import_live_consensus_inputs` is the sole node-path populator).
- **CE-G-D-2 (rehearsal-evidence non-promotability — MECHANICAL, closeable)** — candidate tests `rehearsal_envelope_wraps_correlate_produced_payload`, `rehearsal_envelope_is_structurally_distinct_from_ba02_manifest`, `rehearsal_correlate_no_evidence_writes_nothing` pass; a candidate gate `ci_check_rehearsal_manifest_schema.sh` is green (vacuous-until-committed; verifies the closed envelope schema + `is_rehearsal`/`venue` + sha256 cross-check + forbids the bounty home + the `ci_check_ba02_evidence_manifest_schema.sh`-does-not-match-rehearsal-home cross-check); `ci_check_node_run_loop_containment.sh` + `ci_check_served_chain_handoff_fence.sh` + `ci_check_live_feed_memory_bounds.sh` **byte-unchanged + green**. No RO-LIVE flip.
- **CE-G-D-3 (operator-gated C1 dry-run — SCAFFOLDS ONLY; live execution BLOCKED)** — the runbook `docs/evidence/phase4-n-f-g-d-private-rehearsal-README.md` is committed (a strict subset of the preprod runbook); a candidate hermetic test `c1_dry_run_correlate_to_rehearsal_envelope` proves end-to-end correlate → rehearsal-envelope wiring; a candidate env-gated `node_c1_dry_run_rehearsal_live` (`ADE_LIVE_C1_DRY_RUN`) is **skipped/blocked** without the C1 net; **no synthetic manifest committed**; live execution stays `blocked_until_operator_c1_net_executed`.

> No human review may substitute for these checks. CE-G-D-1 + CE-G-D-2 close the cluster mechanically; CE-G-D-3 closes its **scaffolding** mechanically — the live C1 execution is a separate operator-witnessed leg that produces a **rehearsal** envelope, never a RO-LIVE flip.

## TCB color map
- **BLUE (none — reuse only):** `ade_ledger::producer::{self_accept, served_chain}`, `ade_network::block_fetch::server`, `ade_codec::cbor::tag24`, the era/nonce authorities. **A BLUE change is a red flag → reject.**
- **GREEN:** `ade_node::ba02_evidence::correlate` (reused, the sole acceptance-evidence ctor); the rehearsal-envelope type + its (de)serialization **if pure** (the only open color — resolved at S2: GREEN if a pure correlate-output wrapper, RED if pure file-I/O metadata; lean GREEN type + RED I/O, mirroring the `ba02_evidence`/`ba02_pass` split); `ade_testkit::consensus::genesis_pinning` (reused).
- **RED:** the `import_live_consensus_inputs` extraction/import surface (`ade_runtime::consensus_inputs`, reused unchanged — the shared import the path-fidelity clause pins, not authority); the rehearsal-envelope file-I/O wrapper (mirrors `ba02_pass`); the two CI fences (`ci_check_node_path_fidelity.sh`, `ci_check_rehearsal_manifest_schema.sh`); the C1 dry-run runbook.

## Forbidden during this cluster *(slice-level prohibitions inherit)*
- **N0 — no private-only shortcut.** No private-only flag/branch/bootstrap/from-genesis constructor/assumption; no helper that makes the rehearsal pass where preprod would fail. A discovered gap is fixed in the **shared** path.
- **Do not relax** `ci_check_node_run_loop_containment.sh`, `ci_check_served_chain_handoff_fence.sh`, or `ci_check_live_feed_memory_bounds.sh` (byte-unchanged).
- **Do not commit a synthetic manifest.** Acceptance is proven ONLY by the operator-captured Haskell peer log through `correlate`. No "accepted" inferred from Ade self-accept / `ForgeSucceeded` / served-block / wire-success.
- **Do not let the rehearsal envelope be promotable** — never under the bounty home, never RO-LIVE-flipping, never referenced by `ci_check_ba02_evidence_manifest_schema.sh` / the `CE-G-C-LIVE_*` family.
- **Do not forge across an epoch boundary with a stale `eta0`** (`DC-EPOCH-03` fails closed).
- No new **BLUE authority / canonical type / `--mode node` argv flag / from-genesis constructor / `NodeBlockSource` or `CoordinatorEvent` variant / parallel serializer**.
- **Hard line:** if the dry-run needs a containment relaxation, a private-only branch, a from-genesis constructor, a synthetic manifest, or a new "peer accepted" rule — **stop and re-scope** (the gap is a shared-path bug or a scope error, not a thing to special-case).

## Replay obligations (scoped)
- **R1** — `correlate(forged_artifact, peer_accept_log) →` byte-identical rehearsal-envelope **payload** on replay (the existing `RO-LIVE-06` property; unit/transcript-tested).
- **R2** — forge replay carried (`DC-NODE-05` / `T-REC-03`): same recovered state + captured ordered live-feed transcript + clock-tick + shutdown schedule → byte-identical forge sequence (the live wire is the nondeterministic source, canonicalized once captured).
- **No new BLUE canonical type, no new authoritative state, no new replay corpus.** Acceptance scoped to touched crates (`ade_node` + consumed `ade_runtime`/`ade_network`/`ade_ledger`/`ade_codec`) + `ci` + `docs` — **not** the full `ade_testkit` corpus lane.

## Registry impact (at close)
- **`CN-REHEARSAL-FIDELITY-01`** — `declared → enforced`; `tests` + `ci_script` populated (S1: `ci_check_node_path_fidelity.sh` + the transfer-fidelity test; S2: `ci_check_rehearsal_manifest_schema.sh` + the envelope tests; S3: the hermetic + env-gated dry-run tests).
- **No status flip on any RO-LIVE rule; no `strengthened_in` bump on `RO-LIVE-01` / `RO-LIVE-06` / `CN-OPERATOR-EVIDENCE-01`.**
- **Not added here:** any "peer accepted the block" rule; any new canonical type; any bounty-completion claim.

## Non-goals
- **The bounty deliverable** (preview/preprod acceptance that flips `RO-LIVE-01`) — that is a separate operator-witnessed C2 leg. G-D is the C1 *dry-run* that de-risks it; a passing C1 run produces a **rehearsal** envelope only.
- **A from-genesis consensus-inputs constructor / offline eta0 derivation** — a private-only path that does not transfer to preprod (N0). G-D reuses `import_live_consensus_inputs`.
- **Cross-epoch production / nonce-roll** (separate cluster; `DC-EPOCH-03` fails closed at the boundary).
- **Mainnet-complete validation fidelity** beyond the accepted-block dry-run.
- **Grounding-doc regeneration** (that's `/cluster-close`).
