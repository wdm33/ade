// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types
// - Explicit state transitions only
// - Canonical serialization for all persisted/hashed data

#![allow(clippy::unwrap_used)]
#![allow(clippy::expect_used)]
#![allow(clippy::panic)]

use ade_core::consensus::{
    decode_chain_dep_state, decode_chain_event, encode_chain_dep_state, encode_chain_event,
    BlockDistance, ChainEvent, ChainSelectionReject, DecodeError, HeaderValidationError, Nonce,
    OpCertCounterMap, Point, PraosChainDepState, SecurityParam, VrfCertError,
};
use ade_types::{BlockNo, EpochNo, Hash28, Hash32, SlotNo};

fn nonce(byte: u8) -> Nonce {
    Nonce(Hash32([byte; 32]))
}

fn pool(byte: u8) -> Hash28 {
    Hash28([byte; 28])
}

/// Fixture used by `layout_is_stable`. Five distinct nonces, three
/// op-cert counter entries, all three Option-u64 fields populated.
fn layout_fixture() -> PraosChainDepState {
    let mut counters = OpCertCounterMap::new();
    counters.upsert_strict(pool(0x10), 7, 11).unwrap();
    counters.upsert_strict(pool(0x20), 3, 22).unwrap();
    counters.upsert_strict(pool(0x10), 9, 33).unwrap();

    PraosChainDepState {
        evolving_nonce: nonce(0x01),
        candidate_nonce: nonce(0x02),
        epoch_nonce: nonce(0x03),
        previous_epoch_nonce: nonce(0x04),
        lab_nonce: nonce(0x05),
        last_epoch_block: Some(EpochNo(123)),
        last_slot: Some(SlotNo(456)),
        last_block_no: Some(BlockNo(789)),
        op_cert_counters: counters,
    }
}

/// Hex of the canonical CBOR encoding of `layout_fixture()`.
///
/// Layout (CBOR array of 9 elements, definite-length):
///   89                                  # array(9)
///   5820 01..(32 bytes)..               # evolving_nonce  (bytes 32)
///   5820 02..(32 bytes)..               # candidate_nonce (bytes 32)
///   5820 03..(32 bytes)..               # epoch_nonce     (bytes 32)
///   5820 04..(32 bytes)..               # previous_epoch_nonce (bytes 32)
///   5820 05..(32 bytes)..               # lab_nonce       (bytes 32)
///   187b                                # last_epoch_block = uint(123)
///   1901c8                              # last_slot       = uint(456)
///   190315                              # last_block_no   = uint(789)
///   83                                  # op_cert_counters = array(3)
///     83 581c 10..(28 bytes).. 07 0b    # entry: pool 0x10*28, kes 7,  ctr 11
///     83 581c 10..(28 bytes).. 09 1821  # entry: pool 0x10*28, kes 9,  ctr 33
///     83 581c 20..(28 bytes).. 03 16    # entry: pool 0x20*28, kes 3,  ctr 22
const LAYOUT_FIXTURE_HEX: &str = "\
89\
5820 0101010101010101010101010101010101010101010101010101010101010101 \
5820 0202020202020202020202020202020202020202020202020202020202020202 \
5820 0303030303030303030303030303030303030303030303030303030303030303 \
5820 0404040404040404040404040404040404040404040404040404040404040404 \
5820 0505050505050505050505050505050505050505050505050505050505050505 \
187b \
1901c8 \
190315 \
83 \
83 581c 10101010101010101010101010101010101010101010101010101010 07 0b \
83 581c 10101010101010101010101010101010101010101010101010101010 09 1821 \
83 581c 20202020202020202020202020202020202020202020202020202020 03 16";

fn hex_decode(s: &str) -> Vec<u8> {
    let cleaned: String = s.chars().filter(|c| !c.is_whitespace()).collect();
    assert!(cleaned.len() % 2 == 0, "hex string length must be even");
    let mut out = Vec::with_capacity(cleaned.len() / 2);
    let bytes = cleaned.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        let hi = hex_nibble(bytes[i]);
        let lo = hex_nibble(bytes[i + 1]);
        out.push((hi << 4) | lo);
        i += 2;
    }
    out
}

fn hex_nibble(b: u8) -> u8 {
    match b {
        b'0'..=b'9' => b - b'0',
        b'a'..=b'f' => 10 + b - b'a',
        b'A'..=b'F' => 10 + b - b'A',
        _ => panic!("non-hex byte"),
    }
}

fn hex_encode(bytes: &[u8]) -> String {
    let mut s = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        let hi = b >> 4;
        let lo = b & 0x0f;
        s.push(hex_char(hi));
        s.push(hex_char(lo));
    }
    s
}

fn hex_char(n: u8) -> char {
    match n {
        0..=9 => (b'0' + n) as char,
        10..=15 => (b'a' + n - 10) as char,
        _ => panic!("non-nibble"),
    }
}

#[test]
fn layout_is_stable() {
    let state = layout_fixture();
    let bytes = encode_chain_dep_state(&state);
    let actual = hex_encode(&bytes);
    let expected = LAYOUT_FIXTURE_HEX
        .chars()
        .filter(|c| !c.is_whitespace())
        .collect::<String>();
    assert_eq!(
        actual, expected,
        "PraosChainDepState canonical layout changed; if intentional, update \
         LAYOUT_FIXTURE_HEX and record the format change."
    );

    let decoded = decode_chain_dep_state(&bytes).expect("decode roundtrip");
    assert_eq!(decoded, state);

    let bytes2 = hex_decode(LAYOUT_FIXTURE_HEX);
    let decoded2 = decode_chain_dep_state(&bytes2).expect("decode from hex");
    assert_eq!(decoded2, state);
}

#[test]
fn roundtrip_empty_state() {
    let s = PraosChainDepState::empty();
    let bytes = encode_chain_dep_state(&s);
    let d = decode_chain_dep_state(&bytes).expect("decode empty");
    assert_eq!(d, s);
}

#[test]
fn roundtrip_genesis_state() {
    let s = PraosChainDepState::genesis(Nonce(Hash32([0xab; 32])));
    let bytes = encode_chain_dep_state(&s);
    let d = decode_chain_dep_state(&bytes).expect("decode genesis");
    assert_eq!(d, s);
}

#[test]
fn roundtrip_populated_state() {
    let mut counters = OpCertCounterMap::new();
    counters.upsert_strict(pool(1), 0, 1).unwrap();
    counters.upsert_strict(pool(2), 0, 2).unwrap();
    counters.upsert_strict(pool(3), 0, 3).unwrap();
    counters.upsert_strict(pool(4), 0, 4).unwrap();
    counters.upsert_strict(pool(5), 0, 5).unwrap();
    let s = PraosChainDepState {
        evolving_nonce: nonce(0xaa),
        candidate_nonce: nonce(0xbb),
        epoch_nonce: nonce(0xcc),
        previous_epoch_nonce: Nonce::ZERO,
        lab_nonce: Nonce::ZERO,
        last_epoch_block: None,
        last_slot: Some(SlotNo(42_000)),
        last_block_no: Some(BlockNo(99)),
        op_cert_counters: counters,
    };
    let bytes = encode_chain_dep_state(&s);
    let d = decode_chain_dep_state(&bytes).expect("decode populated");
    assert_eq!(d, s);
}

fn point(slot: u64, byte: u8) -> Point {
    Point {
        slot: SlotNo(slot),
        hash: Hash32([byte; 32]),
    }
}

#[test]
fn roundtrip_chain_event_all_variants() {
    let events = vec![
        ChainEvent::ChainExtended {
            new_tip: point(10, 0x10),
            block_no: BlockNo(7),
        },
        ChainEvent::RolledBack {
            to_point: point(8, 0x08),
            depth: BlockDistance(2),
        },
        ChainEvent::RolledForward {
            from: point(1, 0x01),
            to: point(2, 0x02),
        },
        ChainEvent::ChainSelected {
            new_tip: point(100, 0x64),
            replaced_tip: None,
        },
        ChainEvent::ChainSelected {
            new_tip: point(100, 0x64),
            replaced_tip: Some(point(99, 0x63)),
        },
        ChainEvent::Rejected {
            reason: ChainSelectionReject::ExceededRollback {
                requested: BlockDistance(3000),
                max: SecurityParam(2160),
            },
        },
    ];
    for ev in events {
        let bytes = encode_chain_event(&ev);
        let d = decode_chain_event(&bytes).expect("decode chain event");
        assert_eq!(d, ev);
    }
}

#[test]
fn roundtrip_chain_selection_reject_all_variants() {
    let rejects = vec![
        ChainSelectionReject::ForkBeforeImmutableTip {
            immutable_tip: point(1000, 0x10),
            candidate_intersection: point(900, 0x09),
            rollback_depth: BlockDistance(100),
            security_param: SecurityParam(2160),
        },
        ChainSelectionReject::ExceededRollback {
            requested: BlockDistance(9999),
            max: SecurityParam(2160),
        },
        ChainSelectionReject::HeaderInvalid {
            at_point: point(42, 0x2a),
            reason: HeaderValidationError::VrfCert(VrfCertError::LeaderValueAboveThreshold {
                value: [1, 2, 3, 4, 5, 6, 7, 8],
                threshold: [0, 0, 0, 0, 0, 0, 0, 9],
            }),
        },
        ChainSelectionReject::TiebreakerLossKeepCurrent {
            current_tip: point(50, 0x32),
            candidate_tip: point(50, 0x33),
        },
    ];
    for r in rejects {
        let ev = ChainEvent::Rejected {
            reason: r.clone(),
        };
        let bytes = encode_chain_event(&ev);
        let d = decode_chain_event(&bytes).expect("decode reject roundtrip");
        assert_eq!(d, ev);
    }
}

#[test]
fn decode_rejects_unknown_discriminant() {
    // Build a valid ChainEvent envelope with disc = 99 (not in [0..4]).
    // Outer array(2), uint(99), array(0) payload.
    // CBOR: 0x82 0x18 0x63 0x80
    let bytes = [0x82u8, 0x18, 0x63, 0x80];
    let result = decode_chain_event(&bytes);
    assert!(
        matches!(
            result,
            Err(DecodeError::UnknownDiscriminant {
                for_enum: "ChainEvent",
                found: 99,
            })
        ),
        "got {:?}",
        result
    );
}

#[test]
fn decode_rejects_short_array() {
    // CBOR array(3) of three uint(0): 0x83 0x00 0x00 0x00 — has 3 elements,
    // not 9 as expected for PraosChainDepState.
    let bytes = [0x83u8, 0x00, 0x00, 0x00];
    let result = decode_chain_dep_state(&bytes);
    assert!(
        matches!(
            result,
            Err(DecodeError::FieldCountMismatch {
                expected: 9,
                actual: 3,
            })
        ),
        "got {:?}",
        result
    );
}

#[test]
fn op_cert_counter_map_iteration_is_deterministic() {
    // Same content inserted in two different orders must produce the
    // same canonical encoding bytes (iteration is BTreeMap-sorted).
    let mut a = OpCertCounterMap::new();
    a.upsert_strict(pool(0x05), 2, 1).unwrap();
    a.upsert_strict(pool(0x01), 9, 2).unwrap();
    a.upsert_strict(pool(0x03), 1, 3).unwrap();
    a.upsert_strict(pool(0x01), 1, 4).unwrap();

    let mut b = OpCertCounterMap::new();
    b.upsert_strict(pool(0x01), 1, 4).unwrap();
    b.upsert_strict(pool(0x03), 1, 3).unwrap();
    b.upsert_strict(pool(0x01), 9, 2).unwrap();
    b.upsert_strict(pool(0x05), 2, 1).unwrap();

    let sa = PraosChainDepState {
        evolving_nonce: Nonce::ZERO,
        candidate_nonce: Nonce::ZERO,
        epoch_nonce: Nonce::ZERO,
        previous_epoch_nonce: Nonce::ZERO,
        lab_nonce: Nonce::ZERO,
        last_epoch_block: None,
        last_slot: None,
        last_block_no: None,
        op_cert_counters: a,
    };
    let sb = PraosChainDepState {
        op_cert_counters: b,
        ..sa.clone()
    };
    assert_eq!(encode_chain_dep_state(&sa), encode_chain_dep_state(&sb));
}
