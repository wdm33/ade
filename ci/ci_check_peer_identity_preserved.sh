#!/usr/bin/env bash
# ci_check_peer_identity_preserved.sh -- PHASE4-N-AO S1 (DC-NODE-34).
#
# Peer identity is restored through the receive feed: NodeSyncItem carries `peer`,
# both NodeBlockSource->NodeSyncItem conversion sites capture it from
# AdmissionPeerEvent, the consumers destructure-and-ignore it (no S1 selection /
# admission / rollback / verdict decision is keyed on it), and NodeSyncItem stays a
# transient (non-serialized) feed type.
set -euo pipefail

NS="crates/ade_node/src/node_sync.rs"
NL="crates/ade_node/src/node_lifecycle.rs"
fail() { echo "FAIL (ci_check_peer_identity_preserved): $1" >&2; exit 1; }

# (A) both NodeSyncItem variants carry a `peer` field.
grep -Eq 'Block \{ peer: String, bytes: Vec<u8> \},' "$NS" \
  || fail "NodeSyncItem::Block must carry { peer: String, bytes: Vec<u8> }"
grep -Eq 'RollBack \{ peer: String, point: Point \},' "$NS" \
  || fail "NodeSyncItem::RollBack must carry { peer: String, point: Point }"

# (B) both conversion sites (the non-blocking pump_lookahead AND the blocking
# next_item recv) capture peer from AdmissionPeerEvent -- never a peer-dropping
# `{ block_bytes, .. }`. Exactly two of each shorthand binding (production only).
[ "$(grep -c 'AdmissionPeerEvent::Block { peer, block_bytes }' "$NS")" -eq 2 ] \
  || fail "both Block conversions must bind peer (expected exactly 2: pump_lookahead + blocking recv)"
[ "$(grep -c 'AdmissionPeerEvent::RollBackward { peer, point, .. }' "$NS")" -eq 2 ] \
  || fail "both RollBackward conversions must bind peer (expected exactly 2)"
# No tuple-style NodeSyncItem construction may remain (peer must always be carried).
if grep -nE 'NodeSyncItem::(Block|RollBack)\(' "$NS" "$NL"; then
  fail "tuple-style NodeSyncItem construction found -- peer must be carried (struct variant)"
fi

# (C) consumers destructure-and-IGNORE peer -- no S1 decision is keyed on it.
grep -Eq 'NodeSyncItem::Block \{ bytes, \.\. \}' "$NS" \
  || fail "run_node_sync must ignore peer: NodeSyncItem::Block { bytes, .. }"
grep -Eq 'NodeSyncItem::RollBack \{ point, \.\. \}' "$NS" \
  || fail "run_node_sync must ignore peer: NodeSyncItem::RollBack { point, .. }"
grep -Eq 'NodeSyncItem::Block \{ bytes, \.\. \}' "$NL" \
  || fail "run_participant_sync must ignore peer: NodeSyncItem::Block { bytes, .. }"
grep -Eq 'point: wire_point, \.\.' "$NL" \
  || fail "run_participant_sync must ignore peer (NodeSyncItem::RollBack binds 'point: wire_point, ..')"

# (D) NodeSyncItem stays transient -- never serialized / persisted.
if grep -nE 'impl .* for NodeSyncItem|encode_node_sync_item|decode_node_sync_item|Serialize for NodeSyncItem|AdeEncode for NodeSyncItem' "$NS"; then
  fail "NodeSyncItem must stay a transient feed type (no encode/decode/Serialize impl)"
fi

echo "OK: peer identity preserved through NodeSyncItem (A variants carry peer; B both conversions capture it; C consumers ignore it; D non-serialized) -- DC-NODE-34"
