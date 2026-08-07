# CE-3d — S1-era re-bootstrap + byte-exact differential runbook

> Local working doc (do NOT commit — competition secrecy). The gate it drives is
> `LIVE-LEDGER-EPOCH-TRANSITION` S3 CE-3d (item #3), the admissibility proof for
> [[SLICE-S4-contract]]. Verified against the venue 2026-07-06.

## Why a re-bootstrap (not a re-run)

The two BLUE reward fixes are landed: the reward-crediting discriminant + pool-owner attribution
(`aeeaf89d`, 2026-06-29) and the post-RUPD boundary mark (`dafe0faf` / CE3D S1, 2026-07-06). But every
retained seed accumulator predates S1:

| seed dir | built | binary |
|---|---|---|
| `.cardano-ce3c-firstrun/` | 2026-06-28 | pre-aeeaf89d, pre-S1 |
| `.cardano-ce3d-rebootstrap/` | 2026-06-29 | post-aeeaf89d, **pre-S1** |
| `.cardano-ce3d-extract/harness-work/` | 2026-07-05 | **pre-S1** |

The differential advances a POST-1340 seed across 1340→1341→1342. `go(1342)` rotates from the seed's
**stored mark(1340)**; `go(1341)` from the seed's **set(1340)**. Both were written by a pre-S1 advance, so
running the current (S1) test binary against any retained seed still shows the seed's pre-S1 mark residual —
the S1 fix only governs marks the advance BUILDS. **A clean zero requires a seed whose mark(1340)/set(1340)
were themselves built by the S1 binary** → re-follow from bootstrap with S1.

Diagnostic (`ce3d_boundary_differential_1341_1342` on the pre-S1 seed with the S1 test binary) is expected to
still show the ~−343B `go` residual (seed-carried) with an S1-corrected reward map — it confirms the
contamination, it is NOT the gate.

## Inputs (all present — no fresh Mithril fetch)

- **Bootstrap snapshot:** `.cardano-preview-judge/preview-snapshot/` (15G, certified_point slot 115676685 /
  epoch 1337 — the EXACT snapshot the original 1338 seed came from; carries `db/ledger/115676685/{state,tables}`).
- **Historical-block peer:** docker `cardano-node-preview` (Exited; `docker start cardano-node-preview`;
  N2N `127.0.0.1:3002`). Serves 1337→1340 from its immutable DB.
- **Corpus + references (already extracted):** `.cardano-ce3d-extract/corpus_blocks/` (5815 blocks,
  115758773→115953100) and `db/ledger/{115862416,115948834}_db-analyser/state` (POST-1341 / POST-1342).
- Disk: 44G free (need ~10G scratch).

## Steps

**0. Build the S1 binary (the release binary is 2026-07-04 = pre-S1 — MUST rebuild).**
```
cargo build --release -p ade_node          # target/release/ade == HEAD (S1)
```

**1. Fresh re-follow to POST-1340 with the S1 binary.** New data-dir, bootstrap from the judge snapshot,
follow the docker peer to slot ≥ 115_776_011 (first block of epoch 1340), then stop.
```
docker start cardano-node-preview
S1=/home/ts/Code/rust/ade/target/release/ade
$S1 node run --network preview \
   --bootstrap-mithril /home/ts/.cardano-preview-judge/preview-snapshot/manifest.json \
   --snapshot-dir      /home/ts/.cardano-preview-judge/preview-snapshot \
   --data-dir          /home/ts/.cardano-ce3d-s1seed \
   --peer 127.0.0.1:3002
# watch node.log for: epoch-accumulator CROSSED 1338->1339 and 1339->1340; stop shortly AFTER
# "CROSSED boundary 1339 -> 1340 at slot 115776011" (SIGINT once last_advanced >= 115776011).
```

**2. Seal the S1 seed stores** (the differential reads exactly these two files):
```
ls -la /home/ts/.cardano-ce3d-s1seed/{epoch-accumulator.redb,reduced-checkpoint.redb}
```

**3. Run the byte-exact differential against the S1 seed.**
```
CE3D_SEED_STORES=/home/ts/.cardano-ce3d-s1seed \
CE3D_WORK=/home/ts/.cardano-ce3d-extract/harness-work-s1 \
cargo test -p ade_testkit --release --test ce3d_boundary_differential -- \
   --ignored --exact ce3d_boundary_differential_1341_1342 --nocapture
```
(CE3D_CORPUS / CE3D_REF_1341 / CE3D_REF_1342 default to the extracted venue paths.)

## Acceptance (the gate — ZERO, not "small enough")

At **BOTH** POST-1341 and POST-1342 the `compare()` output must show `MATCH` for every field:
- `treasury`, `reserves` (pots) — exact.
- `go_pool_stakes` — `val_mismatch=0 only_ade=0 only_ref=0`, total delta `d0`. **Any B3c-class stake drift is
  a real finding here, not a tolerance** — it becomes the next invariant, correctness-first.
- `rewards` (per reward account, discriminant-keyed) — exact.

Then, and only then, is CE-3d green and [[SLICE-S4-contract]] admissible. The test currently only asserts the
epoch advanced; once zero is achieved, harden it to assert `exact` on every field (turn the diagnostic into a
gate) and wire the CI check.

## Notes / risks

- The re-follow reproduces the CE-3c FirstRun path (proven), so peer/handshake/throughput are known-good
  (DC-MEM-11 throughput fix in effect). If the follow stalls at a boundary cross, that is itself a finding.
- If POST-1341 is clean but POST-1342 is not (or vice-versa), the residual is boundary-specific — capture
  which of RUPD / mark-rotation / POOLREAP / gov-refund it lands in (the `compare` sample lines localize it).
- Keep the pre-S1 seeds untouched (they are the diagnosis-of-record). Work in `-s1seed` / `-s1` dirs.
