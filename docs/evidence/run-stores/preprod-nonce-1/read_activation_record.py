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
    rel = "BEFORE" if tslot < FREEZE else "AT/AFTER"
    print("  => promotion recorded %s the freeze (delta %+d slots)" % (rel, tslot - FREEZE))
    if tslot < FREEZE:
        print("     HYPOTHESIS CONFIRMED: the bridge captured a candidate that was still")
        print("     tracking evolving, so it kept moving until the freeze.")
    else:
        print("     Hypothesis REFUTED for this record: the candidate was already frozen,")
        print("     so a stale-candidate explanation does not hold. Look elsewhere.")
