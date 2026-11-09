# EPOCH-CONSENSUS-VIEW — Design Analysis (pre-invariants grounding)

> **Status:** DESIGN RECORD (2026-06-20) — **architecture selected in principle; mechanism UNAPPROVED; first gate = bounded disk-backed materialization with crash-safe rollback/disposal.** A design record, NOT an approved build plan. Pre-code, pre-invariants-sketch; NOT a cluster invariants sketch, NOT a slice. No invariants/cluster/implementation work proceeds until the first gate is proven and the cluster is explicitly approved.
> **Decision under analysis:** Option 3 — a native next-epoch stake/consensus view formed by a **bounded, disk-backed, transient replay window** over Ade's own validated chain, emitting one compact immutable `EpochConsensusView`, then pruning the transient state.

## Hard constraints (binding on the whole cluster)
- **No code in this doc** — analysis only.
- **No `track_utxo=true` on the live producer path.** The full live ledger apply is unbuilt and unproven; it must never be switched on under leader production as a side effect.
- **No permanent parallel StakeView.** Option 1 (a standing stake/balance index separate from the ledger transition) is rejected as a second ledger authority (semantic duplication).
- **Single authority.** The `EpochConsensusView` is a *pure projection* of the one ledger-transition authority (`block_validity` → `apply_block_with_verdicts` → `apply_epoch_boundary_full`), reusing CE-71 (rewards) and DC-WAL-03 (replay-equivalence). No second computation of stake.
- **Independence after bootstrap.** The view is derived solely from Ade's own persisted canonical blocks/checkpoints. The imported cardano-cli distribution is a *bootstrap seed only*, never a per-epoch oracle. Anything that asks a normal node each epoch is the rejected baseline, not a candidate.
- **Option-3 viability gate (load-bearing):** `track_utxo=true` has never run live and the on-disk UTxO backend (redb) is dormant + never durability-proven. **The first implementation slice must prove bounded disk-backed materialization AND crash-safe disposal before anything touches leader production.** Until proven, Option 3 is the chosen *architecture*, not an approved *mechanism*.
- **Prohibition — no fallback authority:** the temporary replay UTxO must NEVER become an implicit fallback authority for normal live follow/forge. It exists only to form a view and is then pruned; the live follow/forge authority is unchanged by its presence or absence.
- **Prohibition — bound activation only:** a compact `EpochConsensusView` may activate ONLY when bound to all of — network, era, epoch, source chain point, ledger/checkpoint commitment, nonce state, snapshot phase, and canonical-bytes hash. A view missing any binding is inert. This prevents combining a correct pool distribution with the wrong fork, epoch, or nonce.

## Why this exists
The live forge fail-closes off its single bootstrapped seed epoch (`DC-EPOCH-03`, `node_sync.rs:1537-1556`): Ade cannot produce past one epoch because it has no next-epoch leader view. The stake-by-pool aggregation that would produce one is **absent** — `new_mark` (`rules.rs:1097-1115`) zero-fills pool stakes and substitutes reward balances; nothing decodes an output's address to a staking credential at all. This cluster builds that aggregation **inside the single ledger authority** and forms the next-epoch view by bounded self-replay.

---

## Deliverable 1 — Address / stake-reference classification matrix
Header byte `tttt nnnn` (type nibble + network nibble); all credential hashes = 28-byte blake2b-224. Decode dispatch: bit4 = payment-is-script; bit6 clear ⇒ base (bit5 = stake-is-script); bit6 set ⇒ non-base (bit5 set ⇒ enterprise/null, bit5 clear ⇒ pointer). Ade's codec already models the five forms (variant tags 0=Base,1=Pointer,2=Enterprise,3=Byron,4=Reward); the absent piece is the **typed decode → era-gated classification → pool attribution**.

**Attribution principle (binding):** stake attribution is derived ONLY from a fully decoded canonical address form — via a typed address decoder, then an era-gated classifier — and realized as the typed `StakeRef` below. **No fixed byte offset is authoritative across address variants or eras.** The `[29..57]` layout holds only for base addresses; it must never become the implementation contract. Pointer/enterprise/reward/Byron forms have different layouts and different (era-gated) attribution, so a byte-offset shortcut would be silently wrong on the very cases that matter.

| Type | Nibble | Stake reference | Contributes stake? |
|---|---|---|---|
| 0–3 base | `00xx` | explicit key/script stake credential (decoded form, not a fixed offset) | **Yes**, all eras |
| 4–5 pointer | `010x` | `Ptr(slot,txIx,certIx)` → registered credential | **Yes pre-Conway; NO Conway+ (retired)** |
| 6–7 enterprise | `011x` | null | **No**, all eras |
| 8 Byron | `0x82` | bootstrap (outside stake machinery) | **No** |
| 14–15 reward | `111x` | the credential itself | it *is* a stake credential (not a UTxO output) |

Pointer encoding = three base-128 big-endian varints (slot, txIx, certIx); cardano-ledger `master` bounds them `Word32/Word16/Word16` (older Shelley used wider slot — **[FLAG] pin to the target ledger tag for historical byte-round-trip**). Source: CIP-19; cardano-ledger `Address.hs`; Conway ledger spec §9.1.2.

## Deliverable 2 — Minimal temporary UTxO record per class
The existing canonical `TxOut` (`snapshot/utxo_state.rs:112-145`) already separates `coin` from an opaque `raw` blob (datum/script_ref/multi-asset). The stake-only transient record drops `raw` and multi-asset entirely:

- **`{ coin: Coin, stake_ref: StakeRef }`** where `StakeRef ∈ { Base(Credential), Ptr(slot,txIx,certIx) [pre-Conway only], Null }`.
- Base/reward credential: from the **fully decoded canonical address** (typed decoder + era-gated classifier), never a fixed byte offset; key/script discrimination from the header bits.
- Pointer: store the `(slot,txIx,certIx)` coordinates pre-Conway; **drop to `Null` for Conway+ inputs** (era-gated at the materialization era).
- Enterprise/Byron: `Null`.
- No datums, scripts, or assets are needed for stake — only `coin` + `stake_ref`. (This is also what makes the transient store far smaller than a full UTxO.)

## Deliverable 3 — Snapshot-formation inputs (reward / delegation / certificate)
Active pool stake per credential = `(Σ UTxO coin whose stake_ref resolves to the credential) + reward_account_balance(credential)`, aggregated per credential, **restricted to credentials registered AND delegated to a registered pool**. To form the snapshot the replay must maintain, from the window's certs + accounts:
- **Delegation map** `credential → pool` (from stake-delegation certs).
- **Reward-account balances** `credential → Coin` (from reward distribution + withdrawals).
- **Registered-pool set + VRF keys** (from pool reg/retire certs).
- **Pointer map** `Ptr → credential` (from registration certs) — **pre-Conway only**.
- **Conway:** governance-action deposits attributed to the depositor's credential join the stake (Conway spec Fig. 50); reward-without-UTxO contributes (issue #4171, version-gated).
- Exclusions: registration **deposit** (~2 ADA) is held in a separate pot, never counted as stake; on deregistration it refunds into the tx value-balance (spendable UTxO), not auto-credited to rewards. Undelegated registered stake does **not** enter `ssTotalActiveStake` either (it must not leak into the lottery denominator). Deregistration requires a zero reward balance (`StakeKeyHasNonZeroAccountBalanceDELEG`). Source: cardano-ledger `State/Stake.hs`, `Conway/Rules/Deleg.hs`; Shelley `epoch.tex`; CE-71 reward formula (`rules.rs:625-1310`, corpus-validated 0-lovelace).

## Deliverable 4 — Snapshot stability + k-window rules
- **mark→set→go** rotate at the epoch boundary (`Snap.hs`): new MARK = freshly computed as of the ending epoch's last block; SET = old MARK; GO = old SET. **Leadership reads SET (2-epoch lag); rewards read GO (3-epoch lag).** (`NewEpoch.hs` `nesPd`, ADR-007 TICKF: value identical to SET.)
- **Stability window = ⌈3k/f⌉** (mainnet 129,600 slots ≈ 36 h), all eras.
- **Randomness-stabilisation window = 3k/f pre-Conway/Babbage, 4k/f from Conway** (172,800 slots ≈ 48 h) — erratum 17.3. (My earlier "Praos vs TPraos" direction was inverted; the bump is at Conway.)
- **Finality:** a boundary snapshot is **not final at the boundary instant**; it is immutable only once the boundary block is `> k` deep (k = 2160). The ledger forces the lazy MARK thunks only after one stability window. **Ade models the rotation but NOT this stability gate — adding it is a fresh obligation.** Source: `StabilityWindow.hs`, `BaseTypes.hs` Globals, `Snap.hs`, `NewEpoch.hs`; ouroboros-consensus erratum 17.3 + "Basics of Ouroboros Praos".

## Deliverable 5 — Replay-window start point + checkpoint requirements
- **Start point:** the existing `materialize_rolled_back_state` (`rollback/materialize.rs:43-117`, live-wired at `node_lifecycle.rs:3017/3370/3803`) already *is* "snapshot `nearest_le` + replay-forward over `block_validity`." Reuse it to replay the bounded window **[prior snapshot boundary → next snapshot point]** into a transient `track_utxo=true` ledger.
- **Compact self-owned checkpoint:** today the checkpoint is a monolithic in-RAM blob holding the full UTxO. A compact per-epoch checkpoint **drops the `utxo_state` array** (`ledger.rs:63`) and keeps `epoch_state` — which already carries `reserves`/`treasury` + the mark/set/go `SnapshotState` — plus a **new fingerprint domain** (today's `framing.rs:108-115` cross-check covers the full state incl. UTxO). Version namespaces are independent (snapshot `SCHEMA_VERSION=1`, chaindb `=3`, fingerprint `=2`) — do not conflate.
- **Window bound:** the start checkpoint must sit at/before the prior snapshot boundary; `RollbackTooDeep` + the k=2160 cap bound how far back is admissible. The window is ≤ one epoch of blocks.

## Deliverable 6 — redb durability / rollback requirements (the gate)
- The on-disk UTxO backend (`chaindb/utxo_anchor.rs`, `utxo_key.rs`) is **DORMANT** (dead_code, no live callers, CI guards keep it out of BLUE) and **never durability-proven live**. Its `encode_tx_out_canonical` is byte-identical to the snapshot encoder (DC-MEM-05 equivalence).
- Option 3 uses it as the **transient substrate** for the replay window: create → materialize the window's UTxO on disk → emit the view → **drop/dispose**. Required + unproven:
  1. **Bounded disk-backed materialization** — RAM working set stays within the BA-08 budget while the full window UTxO lives on disk.
  2. **Crash-safe disposal** — a crash mid-window or mid-dispose must leave no corrupt/partial state and must be recoverable to a clean point (no half-written transient store mistaken for authority).
  3. **Rollback within the window** — a rollback shorter than the window is a shorter replay; a rollback crossing the prior boundary is bounded by checkpoint availability.
- **These three are the first slice's acceptance gate; none may be assumed.**

## Deliverable 7 — Differential-oracle plan (vs cardano-node)
The acceptance bar: Ade's derived view must match cardano-node **exactly** at chosen chain points. Oracle = the live preview node via cardano-cli (bootstrap-only role; not a runtime authority).
- **Pool distribution + per-pool stake:** Ade's `EpochConsensusView.stake_by_pool` vs `cardano-cli query stake-distribution` and per-pool `query stake-snapshot` (`stakeSet`/`stakeGo`).
- **Total active stake:** Ade's `total_active_stake` vs `stake-snapshot` `total.stakeSet`.
- **Snapshot timing:** verify Ade's SET-drives-epoch-N (2-epoch lag) by matching at a known boundary; confirm the view used for epoch N equals the MARK captured at the N−2→N−1 boundary.
- **Leader verdicts:** Ade's derived leader schedule vs `cardano-cli query leadership-schedule` (the existing tooling) for ADE1 — the ultimate end-to-end check, since a stake error shifts the schedule.
- **Method:** byte/numeric equality at ≥2 distinct epoch boundaries spanning the Conway era (to exercise pointer-exclusion + the 4k/f window), plus a deliberate fixture containing a pointer-address UTxO to prove it contributes **zero** in Conway.

---

## Load-bearing risks / fresh proof obligations (not wiring)
1. **Conway pointer exclusion** is era-gated and the highest-risk attribution detail (verified two ways; must be enforced + oracle-tested).
2. **Stake-credential derivation via a typed canonical-address decoder + era-gated classifier** is implemented nowhere — and **no fixed byte offset (`[29..57]` etc.) is authoritative across address variants or eras**; per-era correctness (key/script/reward/pointer/null × era) is a fresh proof obligation.
3. **Snapshot-stability gate** (use a view only once its boundary is >k deep) is absent in Ade; must be added (k=2160 machinery exists).
4. **`track_utxo=true` never ran live**; **redb durability/crash-safe disposal unproven** — Deliverable 6's gate.
5. **CE-71 correct but never live** (0-lovelace vs corpus, zero non-test callers) — running it on the live path is itself unproven.
6. **[FLAG]s to pin to the target node/ledger version:** pointer-coordinate widths; issue-#4171 reward-without-UTxO gating; the pre-Babbage TPraos randomness-window value (code 4k/f vs narrative 3k/f — verify live only if pre-Babbage replay is ever needed; preview is Conway, so 4k/f applies).

## What this doc does NOT decide
The cluster invariants sketch, the cluster plan, and the slices are deferred until this analysis is accepted. The natural first slice is the gate (Deliverable 6: bounded disk-backed materialization + crash-safe disposal of the transient replay window), independent of the attribution logic; the attribution + aggregation (Deliverables 1–3) is the second, validated against the Deliverable-7 oracle.

## Sources
CIP-19; cardano-ledger (`Address.hs`, `State/Stake.hs`, `State/Account.hs`, `Conway/Rules/Deleg.hs`, `Shelley/Rules/{Snap,NewEpoch}.hs`, `StabilityWindow.hs`, `BaseTypes.hs`); Shelley `epoch.tex`; Conway formal ledger spec (§9.1.1–9.1.2, Figs. 36/50/51); ouroboros-consensus CHANGELOG (erratum 17.3) + "Basics of Ouroboros Praos"; Ouroboros Praos paper (eprint 2017/573). Ade groundings: prior EPOCH-VIEW-MINIMUM-AUTHORITY investigation + the Ade-primitives + Cardano-ground-truth passes (2026-06-20).
