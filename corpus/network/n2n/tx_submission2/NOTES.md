# Tx-submission2 real-capture (option B: server-side)

## Client-side capture (original, limited)

- `local_preprod_msg_00_send_init.cbor` (2 bytes, `81 06`): MsgInit, sent by
  Ade's *client* capture binary. cardano-node runs its tx-submission2 requester
  only against peers IT dialed, so a client (dial-in) capture can never elicit
  the rich messages. That motivated the server-side harness below.

## Server-side capture (option B) — `ade_tx_submission2_server_capture`

Ade exposes an inbound listener; the docker **public-preprod** cardano-node
11.0.1 is added to its `localRoots` so the node dials Ade, promotes it to a hot
peer, and — as the tx-submission2 **CLIENT/provider** — sends `MsgInit` then
offers its mempool. Ade plays the **SERVER/consumer** (RequestTxIds /
RequestTxs) and records the node-originated frames. Full frames (8-byte mux
header + payload):

- `preprod_server_txsub_init_00_recv.cbor` — the node's `MsgInit`.
- `preprod_server_txsub_reply_txids_00_recv.cbor` — the node's `MsgReplyTxIds`.

## What the capture FOUND (a real codec false-reject)

The node's `MsgReplyTxIds` real wire form is:

    82 01 9f  82  82 06 5820 <32-byte hash>  19 025c  ff
    [1, <indefinite-array of [ [eraIndex, hash32], size ]>]

i.e. the entries sequence is a CBOR **indefinite-length array** (`9f .. ff`) and
each txId is **era-tagged** `[6, h'..']` (the HardFork GenTxId; 6 = Conway), NOT
a bare 32-byte byte string. Ade's prior codec modelled bare txids + definite
arrays and **false-rejected** the real frame ("indefinite-length array not
allowed"). The synthetic round-trip tests (definite arrays, bare txids) had
missed it — exactly what real-interop capture is for.

## Fix + enforcement (DC-PROTO-11 / TXSUB2-CODEC-REALWIRE)

`crates/ade_network/src/codec/tx_submission.rs` now models the era-tagged txid
(`TxSubmissionTxId{era,id}`), decodes definite AND indefinite sequences, and
re-encodes the indefinite form so the captured frame round-trips
BYTE-IDENTICALLY (preserving the era tag — never stripped/guessed). Enforced by
the codec unit tests (incl. the captured-frame byte-identity test + a
malformed/negative corpus), the `tx_submission2_real_capture_corpus` round-trip
test over this corpus, and `ci/ci_check_tx_submission2_real_capture.sh`.

## Follow-up (pending public-preprod traffic)

The live full exchange (`ReplyTxIds → RequestTxs → ReplyTxs`) and the resulting
DC-PROTO-02 flip require a mempool tx on public preprod, which only flows when
real testnet users broadcast. The harness re-runs to land `MsgReplyTxs` and the
transcript-equivalence gate for the flip.
