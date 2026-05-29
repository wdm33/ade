# Cluster/Slice Plan — Ade: Mithril-Anchored Bootstrap, Network Forward-Sync & WAL Recovery

> **Status:** Ordered slice plan (`/cluster-plan` output). Derived from the scoping
> spec [`mithril-anchored-bootstrap-forward-sync-recovery-cluster-spec.md`](mithril-anchored-bootstrap-forward-sync-recovery-cluster-spec.md).
> Pick-up HEAD `c83f2ba`. **NOT `/invariants`** — registry promotion is per-slice and
> minimal (see "CE, never law" below). Next step after confirmation: `/cluster-doc PHASE4-N-Y`.

## Cluster Index (Dependency Order)

1. **PHASE4-N-Y — Mithril-Anchored Bootstrap, Network Forward-Sync & WAL Recovery** —
   primary invariant: *Ade reaches the live tip and survives power loss through a
   single closed bootstrap authority (verified Mithril snapshot or controlled Conway
   genesis), preserving Cardano wire bytes + Ade-canonical WAL/checkpoints, with
   recovery byte-identical to clean execution and Cardano compatibility proven only on
   observable surfaces.*

   *(N-X is the highest closed PHASE4-N-* letter at `c83f2ba`; N-Y is the next in
   sequence. N-U remains a separately-named deferred durability cluster.)*

### Sibling clusters this one does NOT depend on (named for honesty; out of scope here)

- **Mainnet Byron→Conway historical genesis replay** (`RO-GENESIS-REPLAY-01`) —
  deferred, its own era-by-era cluster.
- **Full N2N/N2C mini-protocol coverage** — a **sibling bounty cluster, not satisfied
  here.** This cluster may depend only on the subset needed for forward-sync
  (Handshake / ChainSync / BlockFetch / KeepAlive), **but the final bounty claim
  remains blocked until the full protocol-surface cluster is complete**: N2N
  {Handshake, ChainSync, BlockFetch, TxSubmission2, KeepAlive, PeerSharing} and N2C
  {Handshake, LocalChainSync, LocalTxSubmission, LocalStateQuery, LocalTxMonitor},
  with closed version negotiation. The bounty's N2C blockfetch/txsubmission and
  two-Haskell-node interop requirements are **not** discharged by N-Y alone.
- **Operator-pass block production** (the C2 forge leg) — layers on **after** S1–S3,
  consuming Ade-derived synced+recovered state, not a hand-fed tip bundle.

---

## PHASE4-N-Y — Mithril-Anchored Bootstrap, Network Forward-Sync & WAL Recovery

- **Primary invariant:** as above.

- **TCB partition:**
  - **BLUE** — `ade_codec` (decode chokepoints), `ade_core` (`header_validate`,
    `fork_choice`, `nonce`), `ade_ledger` (`block_validity`, `wal` encode /
    `replay_from_anchor`, `snapshot`, `bootstrap_anchor`, `fingerprint`), `ade_crypto`
    (digest / anchor verification primitives).
  - **GREEN** — `ade_runtime::bootstrap` (the closed `bootstrap_initial_state`
    authority — a CE locus, **not** law), the forward-sync lifecycle reducer, the
    recovery reducer, `ade_testkit` (differential harness).
  - **RED** — `ade_runtime` (Mithril-client fetch shell, network drivers
    `mux_pump`/`n2n_dialer`, `chaindb` redb writes, node-binary lifecycle),
    `ade_core_interop` (live evidence drivers), `ade_node` (binary).

- **Cluster Exit Criteria:**
  - **CE-1** *(S1)* Mithril certificate/snapshot verified; anchor binding present +
    checked — {network magic, genesis hash, chain point, era, ledger fingerprint,
    immutable range, forward-sync boundary}; **fail-closed** on missing/mismatch.
  - **CE-2** *(S1, CE/wiring)* Mithril-sourced state enters via the **existing** closed
    `bootstrap_initial_state` (`genesis_initial` / `SnapshotStore`) — no new
    trait/plugin seam.
  - **CE-3** *(S2)* Forward-sync admits blocks **only** through canonical
    decode → header-validate → ledger-validate → chain-select chokepoints; no side path.
  - **CE-4** *(S2)* Each admitted block stored **verbatim** (preserved wire bytes) and
    WAL-recorded by hash+fingerprint **before** tip advance; sync reaches tip from the
    anchor.
  - **CE-5** *(S2, replay)* Forward-sync replay-equivalence: same anchor + same ordered
    block sequence → byte-identical post-state + WAL.
  - **CE-6** *(S3, replay/true)* Crash at any phase (import/sync/admission/checkpoint)
    recovers via {anchor + preserved bytes + WAL + latest checkpoint + replay} to
    byte-identical state, **no operator repair**.
  - **CE-7** *(S4)* Conway genesis loads + enters the **same** closed bootstrap
    authority; non-Conway genesis fails closed; genesis→initial-state is deterministic
    (byte-identical).
  - **CE-8** *(S4/S5, replay)* On a private Conway net where both exist:
    Conway-genesis-path fingerprint **==** snapshot-path fingerprint for the same chain
    state (internal determinism; the **only** valid fingerprint-equality claim).
  - **CE-9** *(S5, release)* Compatibility evidence: snapshot-point→tip differential vs
    Haskell; selected tip hash + block/tx verdict + `query utxo` agreement; named
    fixtures + oracle versions + reproducible inputs; regression fixture per mismatch.
    *(Harness/schema enforced mechanically; captured live evidence is operator-witnessed.)*
  - **CE-10** *(S5, release/operational)* Two-Haskell-node private Conway testnet
    interop evidence *(operator-witnessed)*.

- **Slices:**

  - **S1 — Mithril import authority** — invariant: *a Mithril-sourced anchor is admitted
    only after verification + binding to {magic, genesis hash, chain point, era, ledger
    fingerprint, immutable range, sync boundary}; mismatch fails closed and storage does
    not initialize* — addresses CE-1, CE-2 — **TCB: BLUE** (verification + binding
    chokepoint) **+ RED** (mithril-client fetch shell).
    *Promotes:* one **Derived** rule for Mithril binding + fail-closed behavior.
    `RO-MITHRIL-IMPORT-01` may move **declared→partial** in S1, and only
    **declared/partial→enforced** when the reproducible Mithril fixture, binding check,
    and CI/release evidence are present. **Reuses** existing `CN-ANCHOR-01`
    (storage-not-before-anchor) without re-promoting.
    *Slice-entry decision:* verify STM multisig in BLUE vs. trust the documented
    mithril-client + verify content-binding / fingerprint-recompute (per OP-x: Mithril
    = acquisition infra, not a BLUE trust root).

  - **S2 — Network forward-sync durable lifecycle** — invariant: *every forward-synced
    block passes the canonical chokepoints and is preserved-byte-stored + WAL-committed
    before tip advance; the admitted sequence is replay-equivalent* — addresses CE-3,
    CE-4, CE-5 — **TCB: BLUE** (decode/validate/fork-choice/WAL) **+ GREEN** (sync
    reducer) **+ RED** (sockets, redb).
    *Promotes/strengthens:* **strengthens the existing True-tier replay/recovery laws**
    (same anchor + inputs + WAL → byte-identical), and **adds/strengthens a Derived
    Cardano forward-sync admission rule** (chokepoint-only admission + preserved-bytes-
    before-tip-advance) **only if no existing `DC-CONS-*`/`DC-STORE-*` rule covers it**
    (e.g. strengthen `DC-CONS-20` admit-side rather than create a duplicate).

  - **S3 — End-to-end crash recovery wiring** — invariant: *node-binary restart
    reconstructs byte-identical authoritative state from {anchor + preserved bytes + WAL
    + latest checkpoint + replay} with no operator repair, for a crash at any phase* —
    addresses CE-6 — **TCB: GREEN** (recovery reducer) **+ RED** (restart driver, crash
    points) **+ BLUE** (`replay_from_anchor`, `fingerprint`).
    *Promotes/strengthens:* the recovery byte-identical law (specializes the supreme
    determinism law; **strengthens** existing `CN-WAL`/`DC-WAL` recovery rules) — not a
    new law if already present.

  - **S4 — Conway-genesis bootstrap source** — invariant: *a controlled Conway genesis
    enters the same closed bootstrap authority as Mithril; non-Conway fails closed;
    genesis→initial-state is a pure deterministic transform; no Byron→Conway historical
    replay* — addresses CE-7, CE-8 — **TCB: BLUE** (genesis→canonical chokepoint)
    **+ RED** (file read).
    *Promotes:* one **Derived** rule (genesis-source-through-same-closed-authority +
    Conway-only fail-closed + determinism).

  - **S5 — Compatibility evidence bundle** — invariant: *Cardano compatibility is proven
    on observable surfaces only (verdicts, selected tip hash, block hashes, `query
    utxo`, transcripts) with named fixtures + oracle versions; never Ade-vs-Haskell
    private-serialization equality* — addresses CE-9, CE-10 — **TCB: GREEN**
    (differential harness) **+ RED** (live drivers, cardano-cli).
    *Promotes:* one **Derived** rule (observable-surfaces-only compatibility proof) +
    **Release** gates (RO family). Live captures are operator-witnessed (mirrors
    `CN-OPERATOR-EVIDENCE-01`).

- **Replay obligations (this cluster introduces authoritative state — replay-equivalence
  is load-bearing):**
  - **S1:** new replay corpus — a Mithril snapshot fixture + expected anchor binding;
    possible **additive** `WalEntry::SnapshotImport` (append-only; never mutate
    `AdmitBlock` meaning).
  - **S2:** **primary obligation** — captured snapshot-point→tip block sequence
    (preprod) + expected post-state fingerprint; same-anchor + same-sequence →
    byte-identical. Possible additive WAL entries (RollForward/Rollback) — append-only.
  - **S3:** crash-recovery corpus — kill-and-restart at each phase → byte-identical
    fingerprint vs clean run.
  - **S4:** genesis→state determinism corpus — a Conway genesis fixture + expected
    initial-state fingerprint.
  - **S5:** regression fixtures — every discovered Haskell mismatch becomes a named
    corpus entry with oracle version + reproducible input.

- **FC/IS partition (cluster summary):** BLUE = verification + decode + validation +
  WAL/snapshot/fingerprint authorities; GREEN = closed bootstrap authority + sync /
  recovery reducers + differential harness; RED = mithril-client + network + storage +
  node-binary + live-evidence shells. Dependencies flow inward only.

---

## Constraints carried into every slice (hard rules)

- **CE, never law:** `bootstrap_initial_state` + cold/warm matrix, the two-driver split
  (WAL recovery replay vs network forward-sync), the `WalEntry` shape. Acceptance
  criteria / implementation loci — **not** promoted to the registry.
- **One closed bootstrap authority** — no `GenesisAnchor`/`MithrilAnchor` trait or
  plugin seam; both sources populate `BootstrapInputs.genesis_initial` / a
  `SnapshotStore`.
- **No cardano-node storage mimicry** (ImmutableDB/VolatileDB/LedgerDB/utxohd binary).
- **Preserve wire bytes** on hash-critical paths; Ade-canonical bytes only for
  WAL/checkpoints/evidence (re-encode CI-forbidden).
- **Haskell = compatibility oracle, not architecture template.**
- **Observable-surface proof only** — never Ade-fp == Haskell-private-serialization;
  internal genesis-path == snapshot-path fingerprint equality is the only valid
  fingerprint claim.
- **Each slice mergeable** and leaves the system fully correct; no carry-forward.
- **Full N2N/N2C coverage is a sibling cluster** — N-Y does not satisfy that bounty
  requirement by itself; the bounty claim stays blocked until that cluster completes.
