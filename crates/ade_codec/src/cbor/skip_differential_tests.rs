//! SLICE LV-12 (DC-LEDGER-SCRIPT-02) CE-LV12-2/3: the iterative `skip_item` against the recursive
//! formulation it replaced, kept here verbatim as the reference.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use super::*;

/// The pre-LV-12 `skip_item`, unchanged. Test-only reference; it recurses once per nesting level.
fn skip_item_recursive_reference(
    data: &[u8],
    offset: &mut usize,
) -> Result<(usize, usize), CodecError> {
    let start = *offset;
    let (major, ai) = decode_initial(data, offset)?;

    match major {
        MAJOR_UNSIGNED | MAJOR_NEGATIVE => {
            if ai < AI_ONE_BYTE {
            } else {
                let _ = decode_argument(data, offset, ai, start)?;
            }
        }
        MAJOR_BYTES | MAJOR_TEXT => {
            if ai == AI_INDEFINITE {
                loop {
                    let b = peek_byte(data, *offset)?;
                    if b == BREAK_BYTE {
                        *offset += 1;
                        break;
                    }
                    let _ = skip_item_recursive_reference(data, offset)?;
                }
            } else {
                let (len, _) = decode_argument(data, offset, ai, start)?;
                let len = len as usize;
                if (*offset).checked_add(len).map_or(true, |end| end > data.len()) {
                    return Err(CodecError::UnexpectedEof { offset: *offset, needed: len });
                }
                *offset += len;
            }
        }
        MAJOR_ARRAY => {
            if ai == AI_INDEFINITE {
                loop {
                    let b = peek_byte(data, *offset)?;
                    if b == BREAK_BYTE {
                        *offset += 1;
                        break;
                    }
                    let _ = skip_item_recursive_reference(data, offset)?;
                }
            } else {
                let (count, _) = decode_argument(data, offset, ai, start)?;
                for _ in 0..count {
                    let _ = skip_item_recursive_reference(data, offset)?;
                }
            }
        }
        MAJOR_MAP => {
            if ai == AI_INDEFINITE {
                loop {
                    let b = peek_byte(data, *offset)?;
                    if b == BREAK_BYTE {
                        *offset += 1;
                        break;
                    }
                    let _ = skip_item_recursive_reference(data, offset)?;
                    let _ = skip_item_recursive_reference(data, offset)?;
                }
            } else {
                let (count, _) = decode_argument(data, offset, ai, start)?;
                for _ in 0..count {
                    let _ = skip_item_recursive_reference(data, offset)?;
                    let _ = skip_item_recursive_reference(data, offset)?;
                }
            }
        }
        MAJOR_TAG => {
            let _ = decode_argument(data, offset, ai, start)?;
            let _ = skip_item_recursive_reference(data, offset)?;
        }
        _ => match ai {
            0..=23 => {}
            AI_ONE_BYTE => {
                let _ = read_byte(data, offset)?;
            }
            AI_TWO_BYTES => {
                if *offset + 2 > data.len() {
                    return Err(CodecError::UnexpectedEof { offset: *offset, needed: 2 });
                }
                *offset += 2;
            }
            AI_FOUR_BYTES => {
                if *offset + 4 > data.len() {
                    return Err(CodecError::UnexpectedEof { offset: *offset, needed: 4 });
                }
                *offset += 4;
            }
            AI_EIGHT_BYTES => {
                if *offset + 8 > data.len() {
                    return Err(CodecError::UnexpectedEof { offset: *offset, needed: 8 });
                }
                *offset += 8;
            }
            AI_INDEFINITE => {
                return Err(CodecError::InvalidCborStructure {
                    offset: start,
                    detail: "unexpected break code outside indefinite container",
                });
            }
            _ => {
                return Err(CodecError::InvalidCborStructure {
                    offset: start,
                    detail: "reserved additional info in simple/float",
                });
            }
        },
    }

    Ok((start, *offset))
}

const FIXTURES: &[(&str, &[u8])] = &[
    ("130178856", include_bytes!("../../../ade_ledger/tests/fixtures/block_130178856.cbor")),
    ("130271804", include_bytes!("../../../ade_ledger/tests/fixtures/block_130271804.cbor")),
    ("130350133", include_bytes!("../../../ade_ledger/tests/fixtures/block_130350133.cbor")),
    ("130404902", include_bytes!("../../../ade_ledger/tests/fixtures/block_130404902.cbor")),
    ("131625032", include_bytes!("../../../ade_ledger/tests/fixtures/block_131625032.cbor")),
    ("131626888", include_bytes!("../../../ade_ledger/tests/fixtures/block_131626888.cbor")),
    ("133883340", include_bytes!("../../../ade_ledger/tests/fixtures/block_133883340.cbor")),
    ("133902219", include_bytes!("../../../ade_ledger/tests/fixtures/block_133902219.cbor")),
];

type Outcome = (Result<(usize, usize), CodecError>, usize);

fn both(data: &[u8], from: usize) -> (Outcome, Outcome) {
    let mut a = from;
    let ra = skip_item(data, &mut a);
    let mut b = from;
    let rb = skip_item_recursive_reference(data, &mut b);
    ((ra, a), (rb, b))
}

/// The reference recurses ~2.7 KiB per level in debug; the deep fixture needs tens of MiB.
fn on_big_stack<T: Send + 'static>(f: impl FnOnce() -> T + Send + 'static) -> T {
    std::thread::Builder::new().stack_size(512 << 20).spawn(f).unwrap().join().unwrap()
}

#[test]
fn lv12_skip_item_matches_the_recursive_reference_on_every_fixture_and_inner_offset() {
    on_big_stack(|| {
        for (name, data) in FIXTURES {
            let (new, old) = both(data, 0);
            assert_eq!(new, old, "block {name} from 0");
            assert_eq!(new.0.as_ref().unwrap().1, data.len(), "block {name} not skipped whole");
            // Every start offset in the first 4 KiB: mid-item starts, strings, breaks, bad heads.
            for from in 0..data.len().min(4096) {
                let (new, old) = both(data, from);
                assert_eq!(new, old, "block {name} from {from}");
            }
        }
    });
}

#[test]
fn lv12_skip_item_matches_the_recursive_reference_on_every_truncated_prefix() {
    on_big_stack(|| {
        for (name, data) in FIXTURES {
            if data.len() > 16 * 1024 {
                continue;
            }
            for len in 0..data.len() {
                let (new, old) = both(&data[..len], 0);
                assert_eq!(new, old, "block {name} truncated to {len}");
                assert!(new.0.is_err(), "block {name} truncated to {len} skipped whole");
            }
        }
        // The two large fixtures: every prefix ending in their first and last 4 KiB.
        for (name, data) in FIXTURES.iter().filter(|(_, d)| d.len() > 16 * 1024) {
            for len in (0..4096).chain(data.len() - 4096..data.len()) {
                let (new, old) = both(&data[..len], 0);
                assert_eq!(new, old, "block {name} truncated to {len}");
            }
        }
    });
}

#[test]
fn lv12_skip_item_matches_the_recursive_reference_on_constructed_forms() {
    let cases: &[&[u8]] = &[
        &[0x5f, 0x41, 0x00, 0x82, 0x01, 0x02, 0xff],       // indefinite bytes holding an array
        &[0x7f, 0x61, 0x61, 0xff],                         // indefinite text
        &[0x9f, 0xbf, 0x01, 0x9f, 0xff, 0xff, 0xff],       // indefinite array/map nesting
        &[0xa2, 0x01, 0x02, 0x03],                         // map short a value
        &[0xa1, 0x01, 0xff],                               // break where a value belongs
        &[0xd8, 0x18, 0x43, 0x01, 0x02, 0x03],             // tag 24 over bytes
        &[0xd8, 0x18],                                     // tag with no content
        &[0xc6, 0xc6, 0xc6, 0x00],                         // nested tags
        &[0x9b, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0x00], // huge count
        &[0xbb, 0x80, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00], // map count over u64/2
        &[0xff],                                           // bare break
        &[0xfc],                                           // reserved simple
        &[0x1c],                                           // reserved uint ai
        &[0x9f, 0x01],                                     // unterminated indefinite
        &[0xf9, 0x00],                                     // short half float
        &[0x58],                                           // byte string missing length
        &[0x5b, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff], // length past the end
        &[0x81, 0x9f, 0x80, 0xff],                         // definite holding indefinite
        &[],                                               // empty input
    ];
    for case in cases {
        let (new, old) = both(case, 0);
        assert_eq!(new, old, "case {case:02x?}");
    }
}

#[test]
fn lv12_skip_item_walks_ninety_thousand_levels_on_a_256_kib_thread() {
    // One byte per level: 0x81 (array of one) repeated, closed by a 0x00. A block body is at most
    // 90,112 bytes, so this is past the deepest nesting a block can carry.
    const DEPTH: usize = 90_000;
    let mut data = vec![0x81u8; DEPTH];
    data.push(0x00);
    let len = data.len();
    let r = std::thread::Builder::new()
        .stack_size(256 << 10)
        .spawn(move || {
            let mut o = 0;
            let r = skip_item(&data, &mut o);
            (r, o)
        })
        .unwrap()
        .join()
        .unwrap();
    assert_eq!(r, (Ok((0, len)), len));
}
