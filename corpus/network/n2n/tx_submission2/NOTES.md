# Tx-submission2 real-capture: limited evidence

## What we captured

- `local_preprod_msg_00_send_init.cbor` (2 bytes, `81 06`): MsgInit, sent
  by our client.

## What we couldn't capture

- MsgRequestTxIds — sent only by cardano-node's tx-submission2 **inbound**
  server, which it runs only against **outbound** peers (peers it dialed).
  We dialed in to cardano-node; the node treats us as an inbound peer
  and never opens its tx-submission2 inbound responder against us.

## What this means for the codec

- The S-A2/S-A6 synthetic round-trip tests
  (`codec::tx_submission::tests::roundtrip_every_variant` and
  `tx_submission2_mempool_trace`) prove the codec round-trips every
  variant including the era-discriminated tx wrapping in
  MsgReplyTxs (which was fixed at the same time as the
  LocalTxSubmission MsgSubmitTx wire-form fix — same skip_item +
  extend_from_slice opaque-pass-through pattern).
- Real-interop validation against cardano-node 11.0.1 will require
  a separate work item where Ade exposes an **inbound listening
  port**, the node is configured to dial us as a `localRoots` peer,
  and we passively record the wire bytes the node sends. That's a
  server-side capture harness, not a client capture binary.
- Tracked as a known coverage gap; does NOT block CE-N-A-5 since the
  codec's wire form is structurally validated by the synthetic tests
  AND the same wire-form fix is confirmed against cardano-node via
  LocalTxSubmission MsgSubmitTx real captures.
