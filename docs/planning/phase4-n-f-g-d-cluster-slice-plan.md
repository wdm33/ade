# Cluster/Slice Plan — PHASE4-N-F-G-D (Ade)

> IDD Part IV artifact. Overall ordered plan derived from the committed invariants
> sketch (`docs/planning/phase4-n-f-g-d-invariants.md`, `07e6a340`) + the declared rule
> **`CN-REHEARSAL-FIDELITY-01`** (two coupled clauses: path fidelity + evidence
> non-promotability). Single sub-cluster — the mechanical accepted-block pipeline is
> already complete (G-A..G-E); G-D is a **bounty dry-run harness**, not a private-net
> product path. Overall plan only — full cluster/slice docs come from `/cluster-doc` +
> `/slice-doc`.
>
> **Cluster character (load-bearing — do not broaden):** G-D's sole purpose is to be a
> **fast failure detector for the exact preview/preprod accepted-block path**, run in a
> venue where the operator controls stake (private genesis) so Ade wins slots in minutes
> instead of waiting days for public-testnet leader slots. It answers one question: *will
> a real Haskell node accept an Ade-forged block when Ade has legitimate leader rights?*
> — surfacing the bounty-blocking failure classes (opcert/KES, Praos leader proof, header
> fields, block body/wire, tag-24 BlockFetch payload, serve path, peer-log parsing) early.
> **Litmus test for every element: does it transfer verbatim to preview/preprod? If not,
> delete or rescope it.**

## Cluster Index (Dependency Order)

1. **PHASE4-N-F-G-D** — Private-testnet accepted-block bounty dry-run — primary invariant:
   the C1 private dry-run exercises the **identical `--mode node` accepted-block path** as
   the preview/preprod bounty pass (N-M-C extraction → forge → self-accept → sibling-serve
   → block-fetch → peer log → `correlate`), and produces only a **clearly-marked,
   non-promotable rehearsal envelope** that flips no RO-LIVE rule. *(`CN-REHEARSAL-FIDELITY-01`.)*

> **Hard lines (every slice; any one breached → stop and re-scope):** **N0** no
> private-only flag / branch / bootstrap authority / from-genesis constructor / assumption
> — if the dry-run needs one to pass, that is a shared-path bug to fix **in the shared
> path**, never special-cased; **N5** never relax `ci_check_node_run_loop_containment.sh`
> / the served-chain handoff fence (`ci_check_served_chain_handoff_fence.sh`) /
> `ci_check_live_feed_memory_bounds.sh`; no synthetic manifest; **no RO-LIVE flip**; no new
> BLUE authority / canonical type / `--mode node` argv flag / from-genesis constructor.

---

## Cluster PHASE4-N-F-G-D — Private-testnet accepted-block bounty dry-run

- **Primary invariant:** `CN-REHEARSAL-FIDELITY-01` (declared; `introduced_in =
  PHASE4-N-F-G-D`). Carries `DC-NODE-06`, `DC-EPOCH-03`, `DC-LIVEMEM-01`, `CN-NODE-02`,
  `DC-SYNC-01/02`, `CN-CINPUT-03` / `DC-CINPUT-02b` (the shared extraction/import path),
  and `RO-LIVE-01` / `RO-LIVE-06` / `CN-OPERATOR-EVIDENCE-01` **unchanged** — **no
  `strengthened_in` bump on the live rules** (G-D does not advance the bounty deliverable).
- **TCB partition:**
  - **BLUE [reused, unchanged — NO new authority]** — `ade_ledger::producer::{self_accept,
    served_chain}`, `ade_network::block_fetch::server`, `ade_codec::cbor::tag24`, the era /
    nonce authorities; the `import_live_consensus_inputs` consume path. *A BLUE change is a
    red flag → reject.*
  - **GREEN [reused + maybe new pure type]** — `ade_node::ba02_evidence::correlate` (the
    sole acceptance-evidence constructor, reused unchanged); possibly the rehearsal-envelope
    type + its (de)serialization if pure.
  - **RED [new — thin]** — the rehearsal-envelope file-I/O wrapper (mirrors `ba02_pass`);
    the two CI fences (path-fidelity + rehearsal-envelope schema gate); the dry-run runbook.
- **Cluster Exit Criteria:**
  - **CE-G-D-1 (path fidelity — MECHANICAL, closeable)** — a test proves
    `import_live_consensus_inputs` consumes an early/private-net extraction through the
    **same** path used for a synced preprod-tip extraction (OQ1 proof obligation; if it
    cannot, the slice fixes the **shared** path, never a private-only workaround); a
    path-fidelity CI fence proves G-D adds **no new `--mode node` argv flag** and **no
    from-genesis consensus-inputs constructor** (the `--mode node` accepted-block path's
    inputs stay operator files + `import_live_consensus_inputs`).
  - **CE-G-D-2 (rehearsal-evidence non-promotability — MECHANICAL, closeable)** — a
    clearly-marked rehearsal manifest that **wraps the same correlate-produced
    `Ba02Manifest` payload but carries a distinct rehearsal envelope** (explicit
    `venue` / `is_rehearsal`; the outer envelope is structurally distinct from the bounty
    manifest and non-promotable) lives **only** under
    `docs/evidence/phase4-n-f-g-d-private-rehearsal-*.toml`, is **correlate-produced** (sole
    ctor; allow-list / hash-primary) and **sha256-bound** to a real Haskell peer log; a new
    vacuous-until-committed gate forbids the rehearsal envelope under the bounty home (and a
    bounty manifest under the rehearsal home) and forbids the `CE-G-C-LIVE_*` gate from
    referencing it; a hermetic fixture proves correlate-produced + fail-closed
    (`NoEvidence` → write nothing). Flips **no** RO-LIVE rule.
  - **CE-G-D-3 (operator-gated C1 dry-run — SCAFFOLDS ONLY; live execution BLOCKED)** — the
    C1 dry-run runbook is committed as a **provable strict subset** of the preprod
    operator-pass runbook (`docs/evidence/phase4-n-f-g-c-operator-pass-README.md`),
    differing only in venue (operator-authored private genesis stake → fast slots; follower
    peer with no forging creds, one-producer-per-key) and the rehearsal label; the
    rehearsal-envelope I/O is wired end-to-end and exercised on a hermetic fixture; **no
    synthetic manifest committed**; the actual C1 live execution is
    `blocked_until_operator_c1_net_executed` (named, **not** deferred).
- **Slices (safety order):**
  - **S1 — Path-fidelity proof + fence** — invariant: the `--mode node` accepted-block path
    is input-driven and venue-agnostic; OQ1 is proven (shared `import_live_consensus_inputs`
    consumes a private/early extraction) **or** the shared path is fixed; no private-only
    divergence exists — addresses: **CE-G-D-1** — TCB: **RED** (CI fence) + a
    transfer-fidelity test over the shared extraction/import path. *Closes hermetically.*
  - **S2 — Rehearsal-evidence surface + gate** — invariant: the rehearsal envelope wraps
    the correlate-produced `Ba02Manifest` payload, is clearly-marked, lives only under the
    distinct rehearsal home, is sha256-bound, and is structurally non-promotable (never the
    bounty home; never RO-LIVE-flipping; never referenced by the `CE-G-C-LIVE_*` gate) —
    addresses: **CE-G-D-2** — TCB: **GREEN** (envelope type, if pure) + **RED** (I/O
    wrapper reusing `correlate`) + **CI** (vacuous-until-committed gate). *Closes hermetically.*
  - **S3 — C1 dry-run runbook + operator-gated execution scaffolding** — invariant: the
    dry-run runbook is a provable strict subset of the preprod operator-pass procedure (same
    path; only venue + label differ); end-to-end correlate → rehearsal-envelope wiring proven
    on a hermetic fixture; the live C1 execution stays `blocked_until_operator_c1_net_executed`
    — addresses: **CE-G-D-3** — TCB: **RED** (runbook / evidence I/O wiring) + **GREEN**
    (`correlate`, reused). *Scaffolding closes hermetically; the live execution is operator-gated.*
- **Replay obligations:** **No new authoritative state, no new canonical type, no new BLUE
  replay corpus.** **R1** — `correlate(forged_artifact, peer_log) →` byte-identical
  rehearsal envelope payload on replay (the existing `RO-LIVE-06` property; unit-tested).
  **R2** — forge replay carried (`DC-NODE-05` / `T-REC-03`: same recovered state + captured
  ordered live-feed transcript + clock/shutdown schedule → byte-identical forge sequence).
  Acceptance scoped to touched crates (`ade_node` + consumed
  `ade_runtime`/`ade_network`/`ade_ledger`/`ade_codec`) + `ci` + `docs` — **not** the full
  `ade_testkit` corpus lane.
- **FC/IS partition:** BLUE consumed-unchanged (456 canonical types expected unchanged — **no
  BLUE→RED edge**); GREEN reuses `correlate` (+ maybe a pure rehearsal-envelope type); RED
  gains the thin I/O wrapper + the two CI fences + the runbook.
- **Close point:** G-D closes when CE-G-D-1, CE-G-D-2, and the CE-G-D-3 **scaffolding** are
  green in CI. `CN-REHEARSAL-FIDELITY-01` flips `declared → enforced` (tests + ci_script
  populated). **No RO-LIVE flip; no `strengthened_in` bump on the live rules.** The live C1
  execution is a separate operator-witnessed leg; even when it passes it produces a
  **rehearsal** envelope — the bounty deliverable remains the C2 preprod pass.

## Notes carried into the cluster

- **Why one cluster, not a split:** unlike RO-LIVE-01's three authority surfaces (which
  forced the G-A/B/C split), G-D is one invariant family — "the private dry-run rehearses
  the exact bounty path and produces non-promotable evidence." Path fidelity and evidence
  non-promotability are **coupled** (if either fails the rehearsal is misleading), so they
  stay one rule and one small cluster. They are split into **separate slices** (S1, S2)
  because path fidelity is the load-bearing bounty-alignment invariant and earns a
  front-loaded slice; evidence non-promotability is distinct enough to close separately.
- **Slice order is the honesty order:** first prove the private dry-run uses the same path
  (S1), then create the non-promotable evidence surface (S2), then write the operator
  runbook / scaffold (S3).
- **The bounty-aligned standard (OQ1):** the only place a real gap can hide is whether the
  shared extraction/import transfers to a private/early net. S1 makes that a proof
  obligation; a gap is fixed in the **shared** path (benefiting preprod), never
  special-cased.
- **What this is NOT:** not preview/preprod, not bounty completion, not a broad private
  rehearsal, not a new bootstrap. After a passing C1 dry-run the next step is register/stake
  on preprod and run the same path — **not** more private-net work.
