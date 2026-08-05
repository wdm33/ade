"""PREPROD-NONCE-1: read the EpochConsensusViewActivated record out of a store's WAL.

Tests the promotion-timing hypothesis WITHOUT another 30-minute re-bootstrap:
if the activation's transition_point.slot precedes the candidate-freeze slot, the
bridge captured a PRE-FREEZE candidate that kept moving afterwards.

TAG 4 payload = array(10):
  uint target_epoch, uint network_magic, uint era,
  uint transition_slot, bytes32 transition_hash,
  bytes32 source_checkpoint_commitment, uint snapshot_phase,
  bytes32 nonce_commitment, bytes32 stake_view_canonical_hash, bytes32 view_canonical_hash
"""
import struct
import sys

WAL = sys.argv[1] if len(sys.argv) > 1 else \
    "/home/ts/.cardano-live1/ade-preprod/wal/wal-0000.bin"
FREEZE = 129_945_600
EPOCH305_START = 130_118_400
# cardano-node `query protocol-state --testnet-magic 1`, epoch 305 (re-confirmed 2026-08-05).
REFERENCE_ETA0_305 = "74f10bea2b467cac73efbd02b36307fe12a123b098a94cfcfe4c33ce4ef10b62"
# blake2b(candidate@SEED || last_epoch_block@SEED) -- what PREPROD-NONCE-1 wrongly committed.
SEED_TIME_ETA0_305 = "e3402a2b2d04d1055ccf6a6fbafc3febda97a6a7b3a4247f84d9c6070965c7a1"


def cbor(b, i):
    ib = b[i]; mt = ib >> 5; ai = ib & 0x1f; i += 1
    if ai < 24: v = ai
    elif ai == 24: v = b[i]; i += 1
    elif ai == 25: v = struct.unpack_from('>H', b, i)[0]; i += 2
    elif ai == 26: v = struct.unpack_from('>I', b, i)[0]; i += 4
    elif ai == 27: v = struct.unpack_from('>Q', b, i)[0]; i += 8
    else: raise ValueError(ai)
    if mt == 2:
        return mt, b[i:i + v], i + v
    return mt, v, i


data = open(WAL, 'rb').read()
off = 0
acts = []
admits = 0
truncated = 0
while off + 4 <= len(data):
    ln = struct.unpack_from('>I', data, off)[0]; off += 4
    if ln == 0 or off + ln > len(data):
        truncated = len(data) - off + 4
        break
    e = data[off:off + ln]; off += ln
    j = 0
    _, _, j = cbor(e, j)
    _, tag, j = cbor(e, j)
    if tag == 0:
        admits += 1
        continue
    if tag != 4:
        continue
    _, _, j = cbor(e, j)                      # array(10)
    _, epoch, j = cbor(e, j)
    _, magic, j = cbor(e, j)
    _, era, j = cbor(e, j)
    _, tslot, j = cbor(e, j)
    _, thash, j = cbor(e, j)
    _, ckpt, j = cbor(e, j)
    _, phase, j = cbor(e, j)
    _, nonce, j = cbor(e, j)
    _, stake, j = cbor(e, j)
    _, view, j = cbor(e, j)
    acts.append((epoch, magic, tslot, thash.hex(), nonce.hex(), view.hex()))

print("WAL: %s" % WAL)
print("AdmitBlock entries: %d | activation records: %d | trailing/torn bytes: %d\n" % (admits, len(acts), truncated))
for epoch, magic, tslot, thash, nonce, view in acts:
    print("EpochConsensusViewActivated target_epoch=%d magic=%d" % (epoch, magic))
    print("  transition_point slot = %d  (%s)" % (tslot, thash[:16]))
    print("  nonce_commitment      = %s" % nonce)
    print("  view_canonical_hash   = %s" % view[:16])
    print()
    print("  candidate freeze slot = %d" % FREEZE)
    print("  epoch 305 start       = %d" % EPOCH305_START)
    # PREPROD-NONCE-2: judge on the NONCE, not on transition_point.
    #
    # This script originally concluded from `transition_point.slot < FREEZE` that "the bridge
    # captured a still-moving candidate". The conclusion was right but the reasoning was NOT, and
    # the slice doc retracted it: `activation_record_for` sets `transition_point = view.source_point`,
    # which is the frozen-leadership MARK source (the bridge's own `source_point_slot`) -- NOT the slot
    # at which promotion ran. That field is BELOW the freeze in a correct record too, so the old
    # message printed "HYPOTHESIS CONFIRMED" next to CE-N2-4's fixed, reference-correct record.
    #
    # The field that actually decides is `nonce_commitment`, so compare it against the value
    # cardano-node reports for the epoch (`cardano-cli query protocol-state`).
    print("  (transition_point is the MARK source s_prev, not the promotion slot -- it sits")
    print("   %+d slots from the freeze in a CORRECT record too, so it decides nothing here)"
          % (tslot - FREEZE))
    print()
    if nonce == REFERENCE_ETA0_305:
        print("  => nonce_commitment == cardano-node epochNonce(305) 74f10bea..  CORRECT")
        print("     The record commits the BOUNDARY-TICK eta0 (CE-N2-4).")
    elif nonce == SEED_TIME_ETA0_305:
        print("  => nonce_commitment == the SEED-TIME projection e3402a2b..  WRONG")
        print("     blake2b(candidate@SEED || leb): the candidate was still tracking evolving")
        print("     %d slots before the freeze. This is the PREPROD-NONCE-1 defect." % (FREEZE - tslot))
    else:
        print("  => nonce_commitment matches NEITHER the reference eta0(305) nor the known")
        print("     seed-time projection. Do not guess -- re-derive both operands.")
