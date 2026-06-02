# Invariant Sketch — PHASE4-N-F-G-D: Private-testnet accepted-block bounty dry-run

> **Type:** IDD invariant sketch (Part I). Planning artifact. Predecessors in the
> PHASE4-N-F-G family: G-A forge fidelity (`62cb8718`), G-B serve handoff
> (`febee120`), G-C live feed + operator-gated evidence (`351d46bc`), G-E
> live-feed bounded memory (`da205bff`, now on `main`). Code-verified at HEAD
> `da205bff`.

---

## 0. Framing (read first — and an honesty statement)

**The mechanical accepted-block pipeline is already complete** on the `--mode node`
spine: G-A makes the forge genesis-consistent / slot-aligned / epoch-bounded /
self-accepting with real constants; G-B serves only self-accepted artifacts via the
sibling task; G-C wires the live `--peer` WirePump feed (so `ForgeTick` is reachable)
and the BA-02 evidence I/O; G-E bounds peer-driven memory before decode. SEAMS §7
candidate #0 and `RO-LIVE-01.open_obligation` confirm the **only** thing left is an
operator-witnessed live ACCEPT.

**G-D is a bounty dry-run harness, not a private-rehearsal product path.** Its sole
purpose is to be a **fast failure detector for the exact preview/preprod accepted-block
path**, run in a venue where we control stake so Ade wins slots in minutes instead of
waiting days for public-testnet leader slots. It answers exactly one question:

> **Will a real Haskell node accept an Ade-forged block when Ade has legitimate leader
> rights?**

…surfacing the bounty-blocking failure classes early: bad opcert/KES handling, bad
Praos leader proof, bad header fields, bad block body/wire encoding, bad tag-24
BlockFetch payload, bad serve path, bad peer-log evidence parsing.

**The litmus test for every G-D element: does it transfer verbatim to preview/preprod?
If not, delete or rescope it.** A private testnet is useful **only** as a fast failure
detector for the same path we later run on preview/preprod — never as a separate
private-net product path.

**G-D adds no new authority.** No new BLUE authority, no new canonical type, no new
`NodeBlockSource`/`CoordinatorEvent` variant, **no new `--mode node` argv flag, no
from-genesis consensus-inputs constructor, no private-only bootstrap.**

**Pure-transformation honesty (IDD demands I say this plainly):** most of G-D is *not*
a pure authoritative transformation `canonical input → canonical output`. The single
pure transform in scope is the **already-enforced** `ba02_evidence::correlate`
(RO-LIVE-06: pure, total, deterministic, hash-primary). The rest is shell I/O, CI
fences, and a runbook. That is correct and intentional — peer acceptance is *release /
operator evidence, never a runtime invariant* (the doctrine line carried since N-F-C).
G-D's invariants are therefore **path-fidelity + closure / labeling / fail-closed**
invariants — **not** new ledger/consensus law.

**Two halves with sharply different IDD status (mirrors G-C):**
- **Mechanical half (closeable hermetically):** the path-fidelity fence + a *distinct,
  clearly-marked rehearsal-evidence surface* (a rehearsal-labeled manifest of the same
  shape as the bounty `Ba02Manifest`, its distinct home, an I/O wrapper reusing
  `correlate`, and a vacuous-until-committed CI gate) + a C1 dry-run runbook that is a
  provable strict subset of the preprod operator-pass runbook.
- **Operator-gated half (stays blocked):** the actual C1 private-net live execution —
  `blocked_until_operator_c1_net_executed`. Needs the C1 net + Ade stake (via the
  operator-authored private genesis) + a follower Haskell peer.

**The hard limit holds:** private C1 acceptance ≠ bounty completion; preview/preprod
acceptance = bounty completion. The moment the C1 dry-run passes, the next step is **not**
more private-net work — it is: register/stake on preview/preprod, wait for a legitimate
slot, run the same path, capture the peer log, produce bounty evidence.

---

## 1. What must always be true

- **A0 — Path/transfer fidelity (the load-bearing invariant).** The C1 private dry-run
  exercises the **identical `--mode node` accepted-block path** as the preview/preprod
  bounty pass: **N-M-C extraction/import (`import_live_consensus_inputs`) → forge →
  self-accept → sibling-serve → block-fetch → peer log → `correlate`.** No private-only
  flag, branch, bootstrap authority, or from-genesis constructor. The only differences
  from the preprod pass are operator-controlled **inputs** (a private genesis whose
  stake allocation makes Ade win slots fast) and the evidence **label** (rehearsal).
- **A1 — Acceptance is correlate-produced from a real Haskell peer log.** A committed
  rehearsal manifest's acceptance signal is produced **only** by
  `ba02_evidence::correlate` over a real operator-captured **Haskell-peer validation
  log** naming the **exact Ade-forged block hash** (reuses the RO-LIVE-06
  sole-constructor + hash-primary + allow-list discipline **verbatim** —
  `peer_served_block` / `peer_chain_tip` only).
- **A2 — Same evidence shape, clearly marked rehearsal, distinct home.** A rehearsal
  manifest has the **same shape** as the bounty `Ba02Manifest`, carries an explicit
  closed `venue = "private-testnet-c1"` + `is_rehearsal` marker, and lives **only**
  under `docs/evidence/phase4-n-f-g-d-private-rehearsal-*.toml` — never the bounty home
  `docs/clusters/PHASE4-N-F-G-C/CE-G-C-LIVE_*.toml`.
- **A3 — No RO-LIVE flip; narrow claim.** Committing a rehearsal manifest flips **no**
  RO-LIVE rule (RO-LIVE-01 stays `partial`; RO-LIVE-06 stays schema+mechanics). The
  rehearsal closes a **narrow** claim only: *the exact `--mode node` accepted-block path
  was exercised end-to-end against a real Haskell peer on a C1 private testnet, as a
  dry-run.*
- **A4 — Single-epoch, pinned-before-signing (over the N-M-C-extracted bundle).** The
  dry-run forge stays inside the recovered seed epoch (`DC-EPOCH-03`, carried); the
  consensus-inputs bundle — **extracted from the fresh follower peer via the same
  `import_live_consensus_inputs` path preprod uses** (`cardano-cli query protocol-state`
  for eta0, `query stake-snapshot` for stake) — is **pinned consistent** with the shared
  private genesis via `genesis_pinning` **before any live KES signature** (a fast failure
  detector in its own right: a failed pin finds a bug before a slot is spent).
- **A5 — Peer-as-follower; one-producer-per-key.** The validating Haskell peer runs with
  **no forging credentials** (follower, not co-producer); **exactly one producer per pool
  key** (no equivocation / slot-battle).
- **A6 — Gate fail-closed, vacuous-until-committed.** A **new** CI gate enforces the
  rehearsal-manifest closed schema, **sha256-binds** the committed peer-log fixture, and
  enforces the rehearsal label + distinct home; it is **vacuously satisfied** when no
  rehearsal manifest is committed.

## 2. What must never be possible

- **N0 — A private-only shortcut (the decisive prohibition).** Any private-only code
  path, branch, flag, bootstrap authority, from-genesis constructor, or assumption that
  does not transfer to preview/preprod. **No private-only helper may make the rehearsal
  pass if the same condition would fail on preview/preprod.** If G-D needs one to make
  the dry-run pass, **stop** — that is a bug in the shared bounty path masked by the
  private net, to be fixed in the shared path, not special-cased.
- **N1 — Rehearsal ⇄ bounty confusion.** A rehearsal manifest under the bounty
  `CE-G-C-LIVE_*` home; a bounty `Ba02Manifest` under the rehearsal home; a rehearsal
  manifest missing/contradicting its rehearsal label; **any path that promotes a
  rehearsal manifest into a RO-LIVE flip or references it from the
  `CE-G-C-LIVE_*` / `ci_check_ba02_evidence_manifest_schema.sh` gate.**
- **N2 — Self-signal as acceptance.** A rehearsal manifest produced from Ade self-accept
  / `ForgeSucceeded` / served-block / wire-success / peer *connect* or *fetch* / any
  non-Haskell-peer-validation signal (the allow-list drops everything else).
- **N3 — Synthetic manifest.** A hand-authored rehearsal manifest with no real,
  sha256-matching peer-log fixture; a hermetic `correlate` fixture committed under the
  rehearsal home (hermetic fixtures are mechanics-only, never under the evidence home).
- **N4 — Preview/preprod claim.** Any claim that the C1 dry-run satisfies the bounty
  preview/preprod deliverable or substitutes for the C2 graded surface. **C1 rehearsal
  evidence may increase confidence in the bounty path, but it is not bounty evidence and
  must not be referenced by the `CE-G-C-LIVE_*` gate.**
- **N5 — Containment / authority relaxation.** Any relaxation of
  `ci_check_node_run_loop_containment.sh`, the served-chain handoff fence
  (`ci_check_served_chain_handoff_fence.sh`), the `ci_check_live_feed_memory_bounds.sh`
  bounds, or the closed `NodeBlockSource` contract; any new BLUE authority / canonical
  type / second forge codepath / new `NodeBlockSource`/`CoordinatorEvent` variant /
  parallel serializer.
- **N6 — Cross-epoch stale-eta0 forge.** Forging across an epoch boundary with a stale
  `eta0` (`DC-EPOCH-03` fails closed; cross-epoch production is a separate nonce-roll
  cluster).
- **N7 — eta0 asserted from memory.** Asserting the Haskell genesis→initial-nonce rule
  from memory instead of **extracting it once from a fresh peer** (`cardano-cli query
  protocol-state`) + pinning (proof-discipline).

## 3. What must remain identical across executions (deterministic surface)

- **I1 — Correlate determinism (reused, unchanged).** `correlate(forged_artifact,
  peer_log) → BA02Outcome` is pure/total/deterministic/hash-primary (RO-LIVE-06). Same
  inputs → same rehearsal manifest body.
- **I2 — Canonical rehearsal-manifest serialization.** The rehearsal manifest serializes
  canonically and round-trips; the gate's schema/sha256/label/home checks are
  deterministic and content-only.

## 4. What must be replay-equivalent

- **R1 — Correlate replay.** Same captured Haskell peer log + same forged artifact →
  **byte-identical** rehearsal manifest on replay (the existing RO-LIVE-06 property).
- **R2 — Forge replay (carried).** Same recovered state + captured ordered live-feed
  transcript + clock-tick schedule + shutdown schedule → **byte-identical** forge
  sequence (`DC-NODE-05` / `T-REC-03`; the live wire is the nondeterministic *source*,
  canonicalized once captured). **No new authoritative state, no new canonical type, no
  WAL/checkpoint change.**

## 5. State transitions in scope

These are **evidence-surface** transitions, not authoritative-core transitions (the
forge/serve/correlate/extraction transitions are G-A..G-C + N-M-C, consumed unchanged):

| # | Transition | Color | Status |
|---|---|---|---|
| T1 | `(forged_artifact, captured_haskell_peer_log) → Result<RehearsalManifest, NoEvidence>` (correlate + rehearsal label). `NoEvidence` → write nothing, fail closed | GREEN (correlate, reused) + thin label | NEW (label only) |
| T2 | `write_rehearsal_manifest(&RehearsalManifest, out_under_rehearsal_home) → io::Result<()>` — accepts **only** a correlate-produced rehearsal manifest; home is the rehearsal path | RED file I/O | NEW |
| T3 | `(committed rehearsal manifest?, peer-log fixture) → pass | fail` — schema + sha256 + label + distinct-home; vacuous when absent | RED/CI gate | NEW |
| T4 | path-fidelity fence: `(--mode node source tree) → pass | fail` — no new `--mode node` flag, no from-genesis consensus-inputs constructor, inputs stay operator files + `import_live_consensus_inputs` | RED/CI gate | NEW |

## 6. TCB color hypothesis

- **BLUE:** none — reuse only (`self_accept`, `served_chain`, `block_fetch::server`,
  `tag24`, era/nonce, the `import_live_consensus_inputs` consume path). **A BLUE change
  is a red flag → reject.**
- **GREEN:** `ba02_evidence::correlate` (reused, the sole acceptance-evidence
  constructor); *possibly* the rehearsal-manifest type + its (de)serialization if pure
  (mirrors GREEN `ba02_evidence`).
- **RED:** the rehearsal file-I/O wrapper (`write_rehearsal_manifest` + a
  `correlate_peer_log_file`-analog into the rehearsal home — mirrors RED `ba02_pass`);
  the C1 dry-run runbook (docs); the two CI fences.
- **Open color:** the rehearsal-manifest type — GREEN if a pure correlate-output
  wrapper, RED if pure file-I/O metadata. Lean GREEN type + RED I/O (the
  ba02_evidence/ba02_pass split).

## 7. Registry surface (one combined rule proposed — schema is the project's)

One new combined rule (path-fidelity and evidence-non-promotability are **coupled** — if
either fails the rehearsal becomes misleading — so they stay one rule, not two). No
`strengthened_in` bump on the live rules (G-D does not advance the bounty deliverable).

- **`CN-REHEARSAL-FIDELITY-01`** *(tier `release`; `introduced_in = PHASE4-N-F-G-D`;
  `declared` at sketch, `enforced` at close)* — two clauses:
  1. **Path fidelity:** the C1 private dry-run uses the same `--mode node`
     accepted-block path as preview/preprod — N-M-C extraction/import
     (`import_live_consensus_inputs`) → forge → self-accept → sibling-serve → block-fetch
     → peer log → `correlate` — with **no private-only flag, branch, bootstrap authority,
     or from-genesis constructor**.
  2. **Evidence non-promotability:** any private-testnet manifest is clearly marked
     `rehearsal`/`private-testnet`, stored **only** under the rehearsal home,
     **sha256-bound to a real Haskell peer log**, **correlate-produced**, and **flips no
     RO-LIVE rule** (and is never referenced by the `CE-G-C-LIVE_*` bounty gate).

## 8. Open questions (resolve before / at slice entry)

- **OQ1 — N-M-C extraction transfers? (slice-entry proof obligation, not an assumption).**
  Verify in code that `import_live_consensus_inputs` can consume an **early / private-net
  extraction** (epoch-0 / fresh-peer `query protocol-state` + `query stake-snapshot`)
  through the **same path** used for a synced preprod tip extraction. **If it cannot, fix
  the shared extraction/import path — do not add a private-only workaround** (N0). This is
  the bounty-aligned standard and the one place a real gap could hide.
- **OQ2 — Mechanical-vs-live boundary (confirm).** Mirror G-C: the mechanical CE closes
  hermetically (path-fidelity fence + rehearsal-evidence surface + gate + runbook); the
  actual C1 execution is the operator-gated CE (`blocked_until_operator_c1_net_executed`).
  The C1 net + stake (via operator-authored private genesis) + follower peer are
  operator-provided.

## 9. Generation notes

- **Carried scope guards:** bounded-smoke discipline (a C1 dry-run is a bounded failure
  detector producing a reusable, clearly-labeled artifact, never the deliverable);
  shell-must-not-overstate (wire success ≠ admission ≠ accept; only the peer's validation
  log through `correlate`); produce-subordinate-to-sync-spine; fail-closed-validation;
  Mithril-is-peer-infra-not-Ade-authority.
- **Likely shape (for `/cluster-plan`):** a small cluster (G-E-sized), bounty-pure — a
  path-fidelity fence, a thin rehearsal-evidence surface (manifest label + distinct home
  + I/O wrapper reusing `correlate`) + its vacuous-until-committed gate, and a dry-run
  runbook that is a provable strict subset of the preprod operator-pass runbook. Mechanical
  CE closes hermetically; the C1 execution is the operator-gated CE.
- **Next:** `/cluster-plan PHASE4-N-F-G-D` → `/cluster-doc` → `/slice-doc` → implement on
  the `rehearsal/private-testnet-g-d` branch (already created). The cluster doc is
  committed standalone before any implementation.
