# Cluster scoping spec — Mithril-Anchored Bootstrap, Network Forward-Sync, and WAL Recovery

> **Status:** SCOPING SPEC ONLY. Not promoted. Pick-up HEAD `c83f2ba`.
> **Governance decision (2026-05-29):** do **not** run `/invariants` as a broad
> promotion off this doc. This spec separates *stable semantic invariants*
> (candidate registry rules, tiered below) from *implementation facts*
> (`bootstrap_initial_state`, the two drivers, the WAL entry shape) which stay as
> **mechanical acceptance criteria (CE), never constitutional law**. Promote only
> the semantic rules, later, one at a time, after this spec is confirmed.
> Companion: [`operator-pass-live-leg-c1-scoping.md`](operator-pass-live-leg-c1-scoping.md)
> (the tip-only forge framing this supersedes as the primary bounty plan).

---

## 1. Intent (plain terms)

Make Ade **bounty-runnable** from a verified **Mithril snapshot** (mainnet-scale
sync) or a controlled **Conway genesis** (private testnet), forward-sync to tip
through Ade's real validation, persist preserved Cardano bytes + Ade-canonical
WAL/checkpoints, and **recover deterministically after power loss** — then produce
blocks only from Ade-derived state. No whole-chain conversion, no cardano-node
storage mimicry, no plugin/trait bootstrap seam.

This replaces the tip-only single-forge framing as the primary bounty target,
because the bounty grades sync (Mithril *or* genesis → tip), accepted block
production on preview/preprod, N2C, private-testnet interop, and adversarial
tx/block/tip agreement — none of which a tip-seeded single forge satisfies.

## 2. Corrected doctrine (grounded in the tree, not a proposal)

The architecture below is **already the shape at `c83f2ba`**; the cluster wires it,
it does not design it.

- **One closed bootstrap authority — not a trait hierarchy.** `bootstrap_initial_state`
  (`ade_runtime/src/bootstrap.rs:88`) is the SOLE anchor-agnostic authority, with a
  closed cold/warm-start matrix (`:80-82`). Both a genesis loader and a Mithril
  importer must enter through it by populating `BootstrapInputs.genesis_initial`
  (or a `SnapshotStore` the warm-start branch materializes from). `BootstrapAnchor`
  is a **struct** (`bootstrap_anchor/anchor.rs:48`), deliberately not a trait. A
  `GenesisAnchor`/`MithrilAnchor` trait/plugin model is **rejected** — it would open
  an extensibility seam in an authority path (forbidden: closed semantic surfaces).

- **Two distinct drivers over one store — do not merge into "replay".**
  1. **WAL recovery replay** — `replay_from_anchor` (`ade_ledger/src/wal/replay.rs:39`)
     reconstructs state after a crash from *already-decided* canonical WAL entries +
     preserved block bytes (`BTreeMap<Hash32, Vec<u8>>`, fail-closed `BlockBytesMissing`).
     **Built.** Serves the power-loss test.
  2. **Network forward-sync** — fetch blocks from a peer, decode → header/ledger
     validate → chain-select → store preserved bytes → append WAL → checkpoint, to
     reach tip. Components exist (receive/admission, `ade_core_interop::follow`); the
     **durable end-to-end lifecycle is the gap.** Serves the sync test.

- **Two-layer byte authority — already enforced, must not break.** Preserved Cardano
  wire bytes for hash-critical paths (`StoredBlock.bytes` = "wire-byte authoritative
  representation", `chaindb/types.rs:24`; `PreservedCbor<T>`); Ade-canonical bytes for
  internal replay/WAL/checkpoints. The WAL references blocks **by hash + fingerprints**,
  never stores block bytes: `WalEntry::AdmitBlock { prior_fp, block_hash, slot,
  verdict_code, post_fp }` (`wal/event.rs:38`). Re-encoding hash-critical bytes is
  CI-forbidden (`ci_check_hash_uses_wire_bytes.sh`).

- **Two proof classes — fingerprint equality is internal only.** `fingerprint`
  (`ade_ledger/src/fingerprint.rs:8`) is **Ade-canonical semantic**, not cardano-node
  serialization. So:
  - *Internal determinism (Ade self-checks):* genesis-path fp **==** Mithril-path fp
    for the same chain state. Valid.
  - *Cardano compatibility (observable only):* same accept/reject verdicts, same
    **selected tip hash**, same **block hashes**, same `cardano-cli query utxo` result,
    same protocol transcript. **Never** Ade-ledger-hash **==** Haskell-ledger-hash.

## 3. What exists vs. the gap (honest inventory)

| Surface | State at `c83f2ba` |
|---|---|
| Closed bootstrap authority (`bootstrap_initial_state`, cold/warm matrix) | **Built** |
| Preserved-byte block store (`ChainDb` in-memory + persistent/redb) | **Built** |
| Ade-canonical snapshot encode/decode (`ade_ledger::snapshot::*`, schema-versioned) | **Built** |
| Canonical WAL (`AdmitBlock` by-hash + fingerprints) + recovery replay (`replay_from_anchor`) | **Built** |
| BootstrapAnchor types + lineage | **Built** |
| Operator consensus-inputs import (structural-only validation, `importer.rs:159-229`) | **Built (not anchor-vetting)** |
| Per-era ledger validation | **Partial / Conway-heavy** (Phase 2B Byron–Mary; Alonzo+ partial; reward+gov depth Conway) |
| **Mithril import authority** (`RO-MITHRIL-IMPORT-01`) | **Declared / blocked_until_mithril_import_cluster** |
| **Network forward-sync durable lifecycle** (import → sync → validate → store → WAL → checkpoint → tip) | **Components exist; lifecycle not wired** |
| **End-to-end crash recovery driven by the node binary** (`RO-GENESIS-REPLAY-01` declared) | **Pieces exist; not the actual restart path** |
| Conway-only genesis loader → `bootstrap_initial_state` | **Partial (genesis types exist; not wired as a bootstrap source)** |

## 4. Scope

**Included:** Mithril import authority; the closed bootstrap input matrix (reused, not
extended); network forward-sync driver; block-admission lifecycle through existing
chokepoints; preserved-byte `ChainDb` writes; canonical WAL append; checkpoint commit;
restart recovery via WAL replay; Conway-scope genesis as a second bootstrap input.

**Excluded (now):** full mainnet Byron→Conway historical genesis replay; cardano-node
storage-layout mimicry (ImmutableDB/VolatileDB/LedgerDB / utxohd binary); trait/plugin
bootstrap authority; any Haskell-ledger-state-hash equality claim.

## 5. Genesis-to-tip classification (policy)

| Path | Purpose | Priority |
|---|---|---|
| Mithril → forward-sync → tip | Main bounty sync (mainnet-scale) | **P0** |
| Preprod/preview Mithril-anchor → forward-sync | Block-production evidence venue | **P0** |
| Conway-only genesis → tip | Private testnet w/ 2 Haskell nodes | **P0/P1** |
| Mainnet genesis Byron→Conway → tip | Long-form assurance | **Deferred** (own era-by-era cluster; `RO-GENESIS-REPLAY-01`) |

Full historical mainnet replay is the one path the bounty lets us avoid — Mithril is
its purpose-built substitute. Genesis stays a *supported bootstrap class*, not the
primary bounty path.

## 6. Registry CANDIDATES (tiered — NOT yet assigned IDs)

> IDs are append-only + human-curated; these are **proposals**, family-tagged, to be
> promoted individually after this spec is confirmed. Tier ↔ family: True→`T`,
> Derived→`CN`/`DC`, Release→`RO`, Operational→`OP`.

### True (semantic law — strongest candidates; several already constitutional)
- T-1 Storage MUST NOT initialize before a verified bootstrap anchor.
- T-2 Every WAL entry + checkpoint is bound to exactly one anchor lineage.
- T-3 Recovery (anchor + preserved bytes + WAL + latest checkpoint + replay) is
  byte-identical to clean Ade execution. *(the supreme determinism law, specialized)*
- T-4 Hash-critical Cardano paths use preserved wire bytes; internal replay/WAL/
  checkpoint surfaces use Ade-canonical bytes. *(byte-authority split)*
- T-5 No runtime plugin/trait extensibility in authority surfaces; bootstrap sources
  are a closed input set, CI-enforced.

### Derived (Cardano-specific)
- CN/DC-a Mithril snapshot import MUST bind: network magic, genesis hash, chain point,
  era, snapshot certificate, ledger fingerprint, immutable range, forward-sync boundary.
- CN/DC-b Network forward-sync admits blocks ONLY through the normal decode → header →
  ledger → chain-selection chokepoints (no side path).
- CN/DC-c Conway-scope genesis bootstrap enters through the SAME closed
  `bootstrap_initial_state` authority as Mithril.
- CN/DC-d Cardano compatibility proof uses observable semantics (verdicts, tip hash,
  block hashes, `query utxo`, transcript) — never Ade-vs-Haskell private-serialization
  equality.

### Release (gates demo/bounty claims; not runtime law)
- RO-i Snapshot-point→tip differential run vs Haskell.
- RO-ii Crash-during-sync → restart → recover test.
- RO-iii Private Conway testnet with two Haskell nodes.
- RO-iv Regression fixture for every discovered mismatch.
- RO-v Evidence bundle proving the imported anchor + forward-sync range.

### Operational (runbook safety; not BLUE semantics)
- OP-x Mithril is acquisition infrastructure, never a BLUE runtime trust root.
- OP-y Mainnet bounty sync is Mithril-first.
- OP-z Full historical genesis replay stays declared/deferred until its own cluster.
- OP-w Cold keys off the producing host; exactly one producer per pool key.

## 7. NOT invariants — mechanical acceptance criteria (CE) only

These are implementation facts; they must be **CE acceptance criteria**, never
registry invariants (per the governance decision):

- The bootstrap seam *is* `bootstrap_initial_state` with its cold/warm matrix
  (an implementation locus for T-5, not a law itself).
- The two-driver split (WAL recovery replay vs network forward-sync) is an
  architectural decomposition, verified by tests — not a constitutional rule.
- The `WalEntry` shape (`AdmitBlock {…}` and its additive siblings) is a closed-enum
  CE; extending the vocabulary is allowed (append-only, never mutate meaning).
- Concrete cardano-cli / Mithril CLI incantations live in the operator runbook.

## 8. Provisional slice decomposition (for `/cluster-plan`, later)

1. **Mithril import authority** — verify certificate + bind to anchor lineage;
   produce the initial `(LedgerState, PraosChainDepState)` + anchor that feed
   `bootstrap_initial_state`. *(the most important missing semantic authority)*
2. **Network forward-sync driver** — durable lifecycle: anchor → ChainSync/BlockFetch
   → decode/validate → chain-select → preserved-byte store → WAL append → checkpoint →
   tip. (Bounty sync test.)
3. **End-to-end crash recovery wiring** — make restart go through anchor + preserved
   bytes + WAL + latest checkpoint + replay; prove byte-identical to clean run.
   (Power-loss test.)
4. **Conway-genesis bootstrap source** — genesis loader → same closed bootstrap
   authority; private-testnet entry. (No Byron; no historical replay.)
5. **Compatibility evidence** — snapshot→tip differential vs Haskell; tip-hash /
   verdict / `query utxo` agreement; two-Haskell-node testnet; mismatch fixtures.

Block production from Ade-derived tip (the operator-pass leg) layers on **after** 1–4,
consuming the synced+recovered state rather than a hand-fed tip bundle.

## 9. Sequencing note

Slices 1–3 are the bounty-critical core (sync + durable recovery). Slice 4 (Conway
genesis) is needed for the two-Haskell-node private-testnet test. Mainnet historical
genesis replay is explicitly **out** — deferred to its own era-by-era cluster under
`RO-GENESIS-REPLAY-01`. Each slice carries its semantic invariant(s) for promotion;
the wiring stays CE.
