# C2 preprod ADE1 accepted-block pass — epoch-296 runbook + readiness grounding

> **PIVOT (2026-06-14):** the primary public-chain proof now goes to **Preview
> first** (1-day epochs → stake active in ~2–3 d vs ~11 on preprod; the bounty
> accepts preview OR preprod). This doc is now the **preprod instance** of the
> venue-parametric guide `c2-public-live-acceptance-runbook.md`; preprod is the
> **optional, stronger follow-up** venue. The stake-source analysis and the
> hard-gate procedure below stay valid for the preprod run if/when taken.
>
> **Scope:** the bounty-critical leg — get the **public preprod** docker node to
> validate + `AddedToCurrentChain` an **Ade-forged** block (RO-LIVE-01), now that
> ADE1 is a registered, soon-electable pool. This doc is the **operator runbook +
> readiness grounding**; it makes **no** acceptance claim and flips **no** rule.
> Builds on `docs/evidence/phase4-n-f-g-c-operator-pass-README.md` (the `--mode
> node` operator-pass procedure) and obeys `docs/active/live-pass-path-fidelity-guide.md`
> (the shared `import_live_consensus_inputs` path; **never** from-genesis).

---

## 0. Current state / grounding (verified 2026-06-14, epoch 294)

- **Identity:** pool `ADE1` = `pool1gkgwpms49j3nykcukqq33lcz7yu5kymwmze2y087ezc8qqpt397`
  (hex `4590e0ee152ca3325b1cb00118ff02f1394b136ed8b2a23cfec8b070`), VRF keyhash
  `08a5dbdae7757262ea934842f35ce5c7693fe471544b29841bb90c0d4263436f`. Runtime keys
  (cold/kes/vrf/opcert) live OFF-repo at `~/Code/rust/ade-ops/preprod/ade-pool/keys/`.
  (Full identity + on-chain params: `docs/evidence/preprod-pool-registration.md`.)
- **Producer path:** `--mode node` forge → self-accept → serve → `ba02_evidence::correlate`
  → `Ba02Manifest` is **built and C2-LOCAL-proven** (PHASE4-N-AE CE-A5: a real
  cardano-node relay `AddedToCurrentChain` an Ade-forged block). The ONLY blocker was
  `blocked_until_operator_stake_available` — now lifted by the ADE1 registration.
- **THE STAKE TIMELINE (the gate):** `cardano-cli query stake-snapshot
  --stake-pool-id <ADE1>` at epoch 294 returned **`mark/set/go = 9497795647 / 0 / 0`**:
  - **Leader election uses the `set` snapshot** — per cardano-ledger, `set` = THIS epoch's
    slot-leader election; `go` = the rewards snapshot (one epoch behind); `mark` = not yet
    active. At epoch 294 ADE1 `set = 0` → **ADE1 leads ZERO slots now**. No accepted-block
    attempt is meaningful before `set > 0`.
  - Only the pledge (~9,497 tADA) sits in `mark`; the ~1,000,008 tADA faucet delegation
    (tx `4a545d61…`, epoch 294) is **not even in `mark` yet** (delegated mid-294, after
    the boundary snapshot).
  - **Forward timeline from the ledger snapshot (authoritative):** the faucet enters `mark`
    at the 294→295 boundary, then rotates `mark→set` ⇒ **faucet active for leadership (in
    `set`) at epoch 296** (~2026-06-20), holding ~1.01M tADA (~dozen-plus slots/epoch) — the
    RELIABLE window. Epoch 295 has only the pledge in `set` (~9.5k tADA, ~0.7 expected
    slots/epoch) — a LONG-SHOT. (Earlier drafts said "epoch 297"; that was a `go`-based
    miscount — leadership uses `set`, one epoch ahead of `go`. See the two-step source fix
    below.)
- **STAKE-SOURCE BUG — FOUND AND FIXED IN TWO STEPS (2026-06-14):**
  1. The extractor originally built `pool_distribution` from `query stake-distribution`,
     whose per-pool fraction does **not** equal the node's leader-election fraction
     (different normalization base) — the gate measured **0/408 within 2%, median 35%**.
     Fixed to `query stake-snapshot --all-stake-pools` (per-pool snapshot stake, hex-keyed)
     in `ef8ac25f`.
  2. **But `ef8ac25f` sourced the `go` snapshot, which is the REWARDS snapshot — THIS
     epoch's slot-leader election uses `set`** (cardano-ledger; confirmed via cardano-node
     SPO docs). For a pool whose stake changed two epochs ago `go ≠ set`, so `go` mis-scores
     leadership. Caught live: the admission false-rejected a *valid* on-chain block from
     pool `85eb86b4…` (`go` σ=0.11% → ineligible; `set` σ=20.9% → eligible — which is why
     the real chain adopted it). **Fix:** `build_consensus_inputs_bundle.sh` +
     `check_ade1_leader_stake_active.sh` now source `stakeSet`. Post-fix the gate's
     whole-distribution check is **410/410 within 2%, median 0.00%** (bundle σ == node `set`
     fraction across the entire distribution).
  - Ade's leader-check (`view.rs::total_active_stake` = `sum(active_stake)`, so
    `sigma = active_stake / sum`) **and** `produce_mode.rs`'s leader schedule both read
    `pool_distribution`, so the admission AND the forge path are fixed by the same bundle
    change. (Affects the live import path; C2-LOCAL CE-A5 never hit it — single-pool venue
    where go≈set.)
  - ADE1 is correctly **absent** from the fixed bundle at epoch 294 (`set = 0` → leads
    zero slots). At epoch 295 only the pledge (~9.5k tADA) is in `set` (long-shot); at
    **epoch 296** the ~1M faucet is in `set` and the gate's ADE1 arm passes with reliable
    leadership.

---

## 1. HARD pre-launch gate (bounty-critical — run BEFORE any live KES signature)

Run these at the candidate pass epoch (≥ 296) and **do not launch the producer until
all pass.** A `set = 0` or a `bundle-fraction ≠ set-fraction` means Ade would forge
slots the reference node rejects.

```
# (a) ADE1 leader-election (set) stake must be > 0.
docker exec cardano-node-preprod sh -c \
  'export CARDANO_NODE_SOCKET_PATH=/ipc/node.socket; \
   cardano-cli query stake-snapshot --stake-pool-id pool1gkgwpms49j3nykcukqq33lcz7yu5kymwmze2y087ezc8qqpt397 --testnet-magic 1'
# EXPECT (~epoch 296): stakeSet ≈ 1.01M tADA (NON-zero). stakeSet == 0 -> ABORT.
# (stakeSet is THIS epoch's leader-election stake; stakeGo is rewards, one epoch behind.)

# (b) Re-extract the bundle from the SAME node (shared path; never from-genesis).
ci/build_consensus_inputs_bundle.sh ~/.cardano-c2-readiness/consensus-inputs.json

# (c) Stake-equality gate: ADE1's bundle leader-fraction == the node's set-fraction.
ci/check_ade1_leader_stake_active.sh ~/.cardano-c2-readiness/consensus-inputs.json
# PASS = (set > 0) AND (|bundle_sigma - set_fraction| < epsilon). FAIL -> ABORT + fix
# the extractor's stake source (it must equal the leader-election `set`), NOT a
# from-genesis workaround.

# (d) DEFINITIVE cross-check (optional but strongest): the node's own leader schedule
#     for ADE1 this epoch. Needs the vrf signing key available to cardano-cli.
#     (Run from the ade-ops workspace, which mounts the keys for cardano-cli; do NOT
#      copy keys into the node container.)
#   cardano-cli query leadership-schedule --genesis <shelley-genesis> \
#     --stake-pool-id <ADE1> --vrf-signing-key-file vrf.skey --current --testnet-magic 1
#   EXPECT: a non-empty slot list. Ade's leader-check over the same bundle MUST
#   produce the SAME slot set (if it diverges, the extraction stake view is wrong).
```

**Necessary but not sufficient:** a passing gate means Ade's leadership view matches
the node's, so a forged block CAN be accepted — but acceptance is proven ONLY by the
operator-captured peer log through `correlate` (§3).

---

## 2. Launch sequence (`--mode node`, the shared path)

Per `phase4-n-f-g-c-operator-pass-README.md` §1–§2 (the authoritative flag set):

1. **Pre-seed the store** (so the node takes the **WarmStart** arm — `FirstRun` is
   Mithril-only): dump the seed UTxO from the docker node and import via the N-M-C
   `seed_to_snapshot` extraction into `--wal-dir`/`--snapshot-dir`, so the recovered
   tip == the live preprod tip (DC-NODE-20/22).
2. **Genesis-consistency pin** (OQ5, before any KES signature):
   `cargo test -p ade_testkit --test <genesis_pinning>` green for the recovered seed epoch.
3. **Launch the forge-capable node** (complete operator key set or none — partial fails
   closed, `EXIT_NODE_FORGE_KEY_INGRESS_FAILED = 44`):
   ```
   ade_node --mode node \
     --peer 127.0.0.1:3001 --network-magic 1 \
     --json-seed <utxo.json> \
     --consensus-inputs-path ~/.cardano-c2-readiness/consensus-inputs.json \
     --cold-skey  ~/Code/rust/ade-ops/preprod/ade-pool/keys/cold.skey \
     --kes-skey   ~/Code/rust/ade-ops/preprod/ade-pool/keys/kes.skey \
     --vrf-skey   ~/Code/rust/ade-ops/preprod/ade-pool/keys/vrf.skey \
     --opcert     ~/Code/rust/ade-ops/preprod/ade-pool/keys/node.opcert \
     --genesis-file <shelley-genesis.json> --genesis-hash <64-hex> --genesis <config> \
     --wal-dir <wal/> --snapshot-dir <snap/> --listen 0.0.0.0:<port>
   ```
   - **Live feed:** `--peer` must reach `Continuing` (else no `ForgeTick` — not a live pass).
   - On a won slot the leader-check returns `Eligible` → `run_real_forge` (KES-signs the
     unsigned-header pre-image, advancing the KES period) → `pump_block` durable admit →
     the serve task offers it (ChainSync RollForward + BlockFetch).
4. **The validating peer is a FOLLOWER, not a co-producer** — the docker preprod node
   runs with NO ADE1 forging credentials (it only validates+adopts+serves). A
   co-producing peer would muddy the acceptance signal and is dropped by the allow-list.

---

## 3. Evidence capture (the ONLY thing that produces acceptance evidence)

1. When the docker node validates + adopts the Ade-forged block, capture its log line
   `ChainDB.AddBlockEvent.AddedToCurrentChain` naming the **exact forged block hash**.
2. **Provenance — decode the forged block** (`cardano-cli debug decode block` on the
   forged bytes, or read Ade's `ForgedBlockArtifact`): record `issuerHash`, `blockNo`,
   `slot`, `hash`. **Verify `issuerHash == blake2b-224(ADE1 cold vkey)`** (the pool's
   cold-key hash) — this is what proves **ADE1** forged it, not some other producer.
3. Normalize the peer log to the closed JSONL the allow-list accepts
   (`ba02_evidence::parse_peer_accept_events`): `peer_served_block` (strongest) /
   `peer_chain_tip` (corroborating). Every other line (incl. any Ade-internal
   `self_accept` / `forge_succeeded`) is dropped — **self-forge exclusion** is automatic.
4. Run the env-gated wiring (writes a manifest ONLY on a real `correlate` exact-match):
   ```
   ADE_LIVE_OPERATOR_TEST=1 ADE_LIVE_FORGED_BLOCK_HASH=<64-hex> ADE_LIVE_FORGED_SLOT=<N> \
   ADE_LIVE_NETWORK_MAGIC=1 ADE_LIVE_PEER_LOG=<peer.log> ADE_LIVE_BA02_MANIFEST_OUT=<ade-evidence.json> \
     cargo test -p ade_node --test node_operator_pass_ba02 node_operator_pass_ba02_live -- --nocapture
   ```
   `NoEvidence` panics + writes nothing (peer did not accept — re-check the gate).
5. Commit the peer log + `correlate`-produced `ade-evidence.json` + the
   `CE-…-LIVE_<run>.toml` binding (sha256 of the peer log; capture command; filter; the
   decoded `issuerHash`/`blockNo`/`slot`/`hash`; `live_feed_exercised = true`) under
   `docs/clusters/PHASE4-N-F-G-C/`; the schema gate
   `ci_check_ba02_evidence_manifest_schema.sh` then verifies it. A registry review at the
   next close records RO-LIVE-01/06.

---

## 4. Failure classifications

| Symptom | Class | Action |
|---|---|---|
| `stakeSet = 0` | not-yet-active stake | WAIT — not a real attempt before `set > 0` (§0). |
| `bundle_sigma ≠ set_fraction` (gate (c) fails) | extraction stake-view mismatch | ABORT; fix `build_consensus_inputs_bundle.sh` to source the leader-election `set` stake (shared path — NOT a from-genesis branch). |
| `leadership-schedule` empty this epoch | ADE1 won no slots this epoch | WAIT for an epoch with a won slot (more likely with more active stake). |
| Ade forges, node REJECTS | leader-check divergence OR block-validity bug | Compare Ade's `Eligible` slots vs `leadership-schedule`; decode the rejected block; if leadership differs → stake-view bug (gate (c)/(d) should have caught it). |
| `correlate` → `NoEvidence` | peer did not accept | Re-check stake/leadership/genesis-consistency; Ade self-accept ≠ peer acceptance. |
| `--peer` not `Continuing` | no live feed | Not a live pass; fix connectivity before claiming evidence. |

---

## 5. Until epoch 296

No live producer attempt is meaningful while `set = 0`. The pre-296 readiness that IS
done: the producer binary builds; the consensus-input extraction runs (R1); the BA-02
`correlate` extractor is hermetically green (18 tests); this runbook + the stake-gate
script are in place. The next real C2 event is the **epoch-296 re-extraction + §1
stake-equality gate + accepted-block attempt**. (A pre-296 dry/watch `--mode node` run
would only prove identity-load / tip-recovery / `NotEligible` parking — not the elected
path — so it is deferred per the operator's call.)
