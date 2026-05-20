# Extracting `resolved_inputs.json` for the B3-S5 real-corpus conservation oracle

CE-B3-5 (`crates/ade_testkit/tests/conway_conservation_positive_corpus.rs`) drives
the BLUE `tx_validity` over every **real** cert/withdrawal-bearing Conway tx in the
committed epoch-576 corpus at `track_utxo=true`, and asserts each is `Valid`. To run,
it needs the resolved input UTxOs those txs spend.

## PROVEN method (executed 2026-05-20, non-destructive)

1. Clone the 300 GB ImmutableDB EBS (`snap-0ca994e50cb2bb1ca`), attach to the instance, mount.
2. Seed the ledger dir with the S3 boundary snapshot `snapshot_163468813` (via a **presigned GET
   URL** from a machine with creds → `curl` on the instance; no creds on the instance, single
   S3→instance hop). Extract its `state`/`meta`/`tables` into `ledger/163468813/`.
3. `~/bin/db-truncater --db $DB --truncate-after-slot 163900638 cardano --config $CFG` → tip lands
   on **block 12270961 / slot 163900587**, the block immediately before the first corpus block.
4. Run `cardano-node` with a **no-peer topology** (`localRoots/publicRoots` empty,
   `useLedgerAfterSlot` huge) so it replays the seed → window (~431K slots, ~2 min) and **freezes**
   at the window instead of syncing forward and re-spending the inputs.
5. `cardano-cli query utxo --tx-in <each> --mainnet --output-json` against the frozen node socket.
6. **Gotchas that matter:**
   - Resolve **spend + collateral + reference** inputs (not just spend) — `check_inputs_present`
     needs all three present in the UTxO. (79 inputs, not 67.)
   - Inputs created by *earlier corpus txs* (intra-corpus) are absent from the pre-window ledger;
     resolve them from corpus tx outputs (`cargo run -p ade_testkit --example resolve_b3_intra_corpus`).
   - Convert query-utxo bech32 addresses to raw hex via `cardano-cli address info`.
7. Combine into `resolved_inputs.json`, commit only that small file. Tear down: kill node, unmount,
   detach + delete the clone volume, stop (NEVER terminate) the instance.

## What to resolve

The exact set is **67 input TxIns** spent by the **29** cert/withdrawal-bearing txs
in `corpus/validity/conway_epoch576/blocks/` (regenerate with
`cargo run -p ade_testkit --example dump_b3_resolution_set`). The list is committed at
`corpus/validity/conway_epoch576/resolution_set.txt` (one `txhash#index` per line).

These txs are the **tail of mainnet epoch 576** (slots 163,900,639–163,900,784). Their
inputs must be resolved against the ledger UTxO set **as of the slot just before the
first corpus block (≈ 163,900,638)** — i.e. while these inputs are still unspent. A
snapshot taken *after* the corpus window has already spent them; the epoch-576
*boundary* snapshot (start of epoch) predates many of them.

The corpus certs are all **Neutral** (StakeDelegation / VoteDelegation — no
deposit/refund terms), so **no registration/pool/DRep state is needed** — only the
input `(coin, address)` pairs. (Verify with
`cargo run -p ade_testkit --example dump_b3_cert_tags`.)

## Target fixture format

Write `corpus/validity/conway_epoch576/resolved_inputs.json`:

```json
{
  "note": "67 resolved inputs for the 29 cert/withdrawal-bearing epoch-576 corpus txs",
  "inputs": [
    { "tx_hash": "<64-hex>", "index": 0, "coin": 1234567, "address": "<hex bytes>" }
  ]
}
```

- `address` is the **raw address bytes in hex** (not bech32) — the witness/required-
  signer path derives the payment credential from it, so it must be byte-correct.
- `coin` is lovelace. Multi-asset value is not needed (B3 conservation is coin-level).

## Path A — node-side (recommended, cleanest)

The proven pipeline lives at `/home/ts/Code/rust/cardano-node-sample` (see its
`EXTRACTION_GUIDE.md` / `SNAPSHOT_CAPTURE_PLAN.md` / `tools/capture_snapshot.sh`). The
AWS instance carries a full mainnet ImmutableDB and `cardano-cli`/`cardano-node`.

The corpus txs are the **tail of epoch 576** (first corpus block at slot 163,900,639),
so resolve against the ledger state at slot **163,900,638**:

```bash
# On the instance, cloning from the existing epoch-576-boundary snapshot
# snapshot_163468813 (slot 163,468,813 — ~431K slots / ~2 min replay):
db-truncater --db "$DB" --truncate-after-slot 163900638 cardano --config "$CFG"
cardano-node run --topology "$TOPO" --database-path "$DB" \
    --socket-path "$DB/../node.socket" --config "$CFG" --port 3002 +RTS -M15G -RTS &
# wait ~2 min for replay to the truncated tip, then for each TxIn in resolution_set.txt:
cardano-cli query utxo --tx-in "<txhash#index>" --mainnet \
    --socket-path "$DB/../node.socket" --output-json
# → { "<txhash#index>": { "address": "addr1...", "value": { "lovelace": N, ... } } }
```

Convert each bech32 `address` to raw hex (`cardano-cli address info` / `bech32`), and
assemble the `resolved_inputs.json` above. Commit only that small file (never the
multi-GB snapshot).

## Path B — from an S3 ledger-state extract

`s3://ade-corpus-snapshots/extracts/ledger_state_conway576*.json` are
`cardano-cli query ledger-state` dumps. Identify the variant whose UTxO set sits at
slot ≈163,900,638 (contains all 67 inputs unspent), then stream-filter its UTxO map
for the 67 keys and project `(address, lovelace)`. Note these are multi-GB
Haskell-format dumps with non-trivial address encoding; Path A is preferred.

## Closing CE-B3-5

Drop `resolved_inputs.json` in place and run:

```bash
cargo test -p ade_testkit --test conway_conservation_positive_corpus
```

Both tests flip from SKIP to an active assertion: every real cert/withdrawal tx →
`Valid`, verdict stream byte-identical across two runs.
