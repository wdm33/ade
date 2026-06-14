# C2 public-live accepted-block — venue-parametric runbook (Preview-first)

> **Scope:** the bounty leg — get a real Haskell cardano-node peer to validate +
> `AddedToCurrentChain` an **Ade-forged** block on a **public** Cardano network.
> The challenge accepts **preview OR preprod**, so we take Preview first (faster).
> This is a **venue change, NOT a correctness shortcut**: same recover-non-Origin-
> Conway-tip discipline, same BLUE invariants, **separate** evidence manifests.
> Preprod-specific procedure + the stake-timeline worked example live in
> `c2-preprod-epoch296-acceptance-runbook.md` (now the *preprod instance* of this
> guide). Obeys `live-pass-path-fidelity-guide.md` (shared
> `import_live_consensus_inputs`; **never** from-genesis).

---

## 0. The ladder (do not skip a rung)

```
C2-LOCAL-REHEARSAL   (private testnet — REQUIRED first; not replaced)
   →  C2-PREVIEW-LIVE  (first public proof; 1-day epochs → stake active in ~2–3 d)
   →  C2-PREPROD-LIVE  (later, OPTIONAL — stronger, closer-to-mainnet evidence)
```

- The private-testnet rehearsal stays mandatory before any public chain.
- Ade **recovers an existing non-Origin Conway tip** and forges tip+1; it never
  forges from genesis or slot 0, on any venue.

## Why Preview first (and why it's still rigorous)

- **Bounty-valid:** the challenge accepts a public block on *preview or preprod*.
- **Faster:** Preview epochs are 1 day (86400 slots) vs preprod's 5 days → the
  2-epoch stake-activation delay collapses from ~11 days to ~2–3.
- **Still real:** Preview is an earlier-stage public testnet (valid, less
  production-like); Preprod is the mature, mainnet-like venue (stronger evidence,
  **not required** for BA-02). We keep Preprod as a follow-up.
- **The real gate is active stake on the TARGET chain** — not preview-vs-preprod.
  σ=0 → leader threshold 0 → the peer rejects the block. So Preview needs its
  **own** pool registration + delegation (a different ledger state from preprod).

---

## 1. Slices

| Slice | What | Layer | Status |
|---|---|---|---|
| **C2-VENUE-PARAM** | venue-parametric extractor + gate + venue-tagged bundle | GREEN/RED | **DONE** |
| **C2-PREVIEW-POOL-REG** | register the Preview SPO identity + faucet delegation | operator / ade-ops | pending (operator) |
| **C2-PREVIEW-RECOVER** | recover the current Preview Conway tip (same path) | GREEN/RED | pending (needs node) |
| **C2-PREVIEW-BA02** | forge Preview tip+1; Haskell peer adopts it | RED + evidence | pending (needs stake) |

### C2-VENUE-PARAM (done)
- Extractor: `ci/build_consensus_inputs_bundle.sh --network preview|preprod <out.json>`
  → venue-tagged bundle carrying `network`, `network_magic`, `epoch_length`,
  `active_slots_coeff`, `genesis_hash_hex` (the Shelley genesis hash),
  `source_tip_slot`/`source_tip_hash_hex`, `pool_distribution_source`. ASC +
  epochLength are read from the **venue** shelley-genesis (never hardcoded).
- Gate: `ci/check_ade1_leader_stake_active.sh --network preview|preprod <bundle>`
  (Preview requires `ADE1_POOL_HEX` + `ADE1_POOL_BECH` — no preprod default leaks).
- The BLUE importer accepts the venue-tag fields as `#[serde(default)]`
  provenance — they do **not** enter the canonical fingerprint.

### C2-PREVIEW-POOL-REG (operator, ade-ops — the acceleration lever)
Register a Preview SPO identity + request faucet **pool delegation** (the Cardano
faucet supports "receive pool delegation" on preview/preprod). **Acceptance:**
```
cardano-cli query stake-snapshot --stake-pool-id <ADE-preview pool id> --testnet-magic 2
```
shows **active leader stake** (`stakeGo` > 0 once it propagates), not merely
registration. Do NOT reuse preprod stake assumptions — Preview is a different ledger.

### C2-PREVIEW-RECOVER / C2-PREVIEW-BA02
Same `--mode node` path as preprod (see the preprod instance §2–§3), with
`--network preview` venue params. **BA-02 acceptance:** `forge_result:succeeded=1`;
Haskell peer `AddedToCurrentChain` = same block hash; `issuerHash` = the **Preview**
pool cold-key hash; relay forging = 0; manifest `network` = preview.

---

## 2. Venue parameterization (quick reference)

| param | preview | preprod |
|---|---|---|
| `--network` | `preview` | `preprod` |
| network magic | `2` | `1` |
| node container | `cardano-node-preview` | `cardano-node-preprod` |
| genesis dir | `~/.cardano-node-preview/config` (off-repo) | `.cardano-node-preprod/config` (in-repo) |
| socket (in-container) | `/ipc/node.socket` | `/ipc/node.socket` |
| consensus-inputs out | `~/.cardano-c2-preview/ade-inputs.json` | `~/.cardano-c2-readiness/consensus-inputs.json` |
| snapshot dir | `~/.cardano-c2-preview/store` | `~/.cardano-c2-readiness/store` |
| wal dir | `~/.cardano-c2-preview/wal` | `~/.cardano-c2-readiness/wal` |
| pool id | `ADE1_POOL_HEX`/`ADE1_POOL_BECH` (Preview pool) | ADE1 preprod (script default) |

Env overrides (`ADE_LIVE_PEER_CONTAINER`, `ADE_LIVE_NETWORK_MAGIC`,
`ADE_LIVE_PEER_SOCKET`, `ADE_LIVE_SHELLEY_GENESIS`, `ADE_LIVE_GENESIS_CONFIG`)
always win over the `--network` defaults.

## 3. The hard pre-launch gate (venue-parametric — bounty-critical)

Identical to the preprod instance §1, with `--network <venue>`:
1. `cardano-cli query stake-snapshot --stake-pool-id <pool> --testnet-magic <magic>` → `stakeGo > 0`.
2. `ci/build_consensus_inputs_bundle.sh --network <venue> <bundle>` (shared path; never from-genesis).
3. `ci/check_ade1_leader_stake_active.sh --network <venue> <bundle>` → PASS (pool `go>0` **and** bundle sigma == node go-fraction + whole-distribution consistency).
4. (strongest) `cardano-cli query leadership-schedule --current` → non-empty; Ade's leader-check produces the same slot set.

Only then launch the producer. A passing gate is **necessary, not sufficient** —
acceptance is proven ONLY by the operator-captured peer log through `correlate` (§4).

## 4. Evidence manifests — keep them SEPARATE

| manifest | venue | bounty status |
|---|---|---|
| `PrivateRehearsalManifest` | local private testnet | rehearsal only — **not** a public-chain claim |
| `Ba02PreviewManifest` | Preview (public) | **BOUNTY-SUFFICIENT** |
| `Ba02PreprodManifest` | Preprod (public) | stronger, optional follow-up |

Do **not** label Preview evidence as preprod. Each `correlate`-produced manifest
already carries `network_magic` (preview=2 / preprod=1), so it is venue-self-
describing; commit under a venue-tagged path and record the operator-decoded
`issuerHash`/`blockNo`/`slot`/`hash`. `ba02_evidence::correlate` is unchanged
(venue is just data); the self-forge-exclusion allow-list is unchanged.

## 5. BLUE invariants — UNCHANGED by the venue pivot

| Tier | Requirement |
|---|---|
| true | Same anchor + inputs + seeds + WAL + checkpoints ⇒ byte-identical outputs. |
| true | No wall-clock / unseeded rand / `HashMap` iteration / fs+net reads / scheduler effects in BLUE. |
| derived | Chain selection, ledger verdicts, transcripts, CBOR bytes, recovery match the Cardano oracle. |
| release | Differential replay, malformed-input corpus, mixed-version, crash-recovery stay release gates. |
| operational | Preview is an **acceleration venue**, not a mainnet-readiness claim. |

The Haskell node is a **compatibility oracle**, not an architecture template; prove
observable equivalence on the required surfaces. No BLUE-path logic changes because
of Preview — venue is data threaded through the RED shell + GREEN evidence only.
