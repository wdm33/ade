// SLICE LV-12 (DC-LEDGER-SCRIPT-02): decoding, skipping, evaluating, cloning, comparing and dropping
// wire-supplied native scripts does not use stack in proportion to nesting depth. Decode outcomes,
// errors and verdicts are those of the recursive implementation, which is kept below as the reference.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::collections::BTreeSet;

use ade_codec::allegra::script::{decode_native_script, decode_native_scripts};
use ade_codec::cbor::{self, envelope::decode_block_envelope, ContainerEncoding};
use ade_codec::conway::decode_conway_block;
use ade_codec::error::CodecError;
use ade_ledger::error::NativeScriptFailure;
use ade_ledger::scripts::{evaluate_native_script, ScriptVerdict, ValidityInterval};
use ade_ledger::witness::decode_witness_infos;
use ade_types::allegra::script::{NativeScript, NativeScriptNode};
use ade_types::{Hash28, SlotNo};

/// Preprod slot 133,883,340, block 5,183,974: tx 3 carries a native script ~5,385 levels deep.
/// Original wire bytes from the peer's ImmutableDB, sha256 ca0feb0e…6710.
const BLOCK_133883340: &[u8] = include_bytes!("fixtures/block_133883340.cbor");

const FIXTURES: &[(&str, &[u8])] = &[
    ("130178856", include_bytes!("fixtures/block_130178856.cbor")),
    ("130271804", include_bytes!("fixtures/block_130271804.cbor")),
    ("130350133", include_bytes!("fixtures/block_130350133.cbor")),
    ("130404902", include_bytes!("fixtures/block_130404902.cbor")),
    ("131625032", include_bytes!("fixtures/block_131625032.cbor")),
    ("131626888", include_bytes!("fixtures/block_131626888.cbor")),
    ("133883340", BLOCK_133883340),
    ("133902219", include_bytes!("fixtures/block_133902219.cbor")),
];

const SMALL_STACK: usize = 256 << 10;

fn on_stack<T: Send + 'static>(bytes: usize, f: impl FnOnce() -> T + Send + 'static) -> T {
    std::thread::Builder::new().stack_size(bytes).spawn(f).unwrap().join().unwrap()
}

fn witness_sets_of(env_bytes: &[u8]) -> Vec<u8> {
    let env = decode_block_envelope(env_bytes).unwrap();
    let block = decode_conway_block(&env_bytes[env.block_start..env.block_end]).unwrap();
    block.decoded().witness_sets.clone()
}

// ---------------------------------------------------------------------------
// The recursive reference (the pre-LV-12 decoder and evaluator, over a recursive type)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq)]
enum Ref {
    Sig(Hash28),
    All(Vec<Ref>),
    Any(Vec<Ref>),
    MOfN(i64, Vec<Ref>),
    InvalidBefore(u64),
    InvalidHereafter(u64),
}

fn ref_decode(data: &[u8], offset: &mut usize) -> Result<Ref, CodecError> {
    let enc = cbor::read_array_header(data, offset)?;
    let arr_len = match enc {
        ContainerEncoding::Definite(n, _) => n,
        _ => {
            return Err(CodecError::InvalidCborStructure {
                offset: *offset,
                detail: "native script must be definite-length array",
            })
        }
    };
    let (tag, _) = cbor::read_uint(data, offset)?;
    match tag {
        0 => {
            let (hash_bytes, _) = cbor::read_bytes(data, offset)?;
            if hash_bytes.len() != 28 {
                return Err(CodecError::InvalidLength {
                    offset: *offset - hash_bytes.len(),
                    detail: "native script Sig key hash must be 28 bytes",
                });
            }
            let mut arr = [0u8; 28];
            arr.copy_from_slice(&hash_bytes);
            Ok(Ref::Sig(Hash28(arr)))
        }
        1 => Ok(Ref::All(ref_decode_array(data, offset)?)),
        2 => Ok(Ref::Any(ref_decode_array(data, offset)?)),
        3 => {
            if arr_len != 3 {
                return Err(CodecError::InvalidCborStructure {
                    offset: *offset,
                    detail: "native script MOfN must be array(3)",
                });
            }
            let start = *offset;
            let (arg, negative, _) = cbor::read_any_int(data, offset)?;
            let Ok(magnitude) = i64::try_from(arg) else {
                return Err(CodecError::InvalidCborStructure {
                    offset: start,
                    detail: "native script MOfN threshold outside int64",
                });
            };
            let n = if negative { -1 - magnitude } else { magnitude };
            Ok(Ref::MOfN(n, ref_decode_array(data, offset)?))
        }
        4 => Ok(Ref::InvalidBefore(cbor::read_uint(data, offset)?.0)),
        5 => Ok(Ref::InvalidHereafter(cbor::read_uint(data, offset)?.0)),
        _ => Err(CodecError::InvalidCborStructure {
            offset: *offset,
            detail: "unknown native script constructor tag",
        }),
    }
}

/// The original preallocated `Vec::with_capacity(n)` from the wire count; that is the one intended
/// divergence (see `lv12_a_forged_sub_script_count_is_an_error_not_an_allocation`), so it is not
/// reproduced here.
fn ref_decode_array(data: &[u8], offset: &mut usize) -> Result<Vec<Ref>, CodecError> {
    match cbor::read_array_header(data, offset)? {
        ContainerEncoding::Definite(n, _) => {
            let mut v = Vec::new();
            for _ in 0..n {
                v.push(ref_decode(data, offset)?);
            }
            Ok(v)
        }
        ContainerEncoding::Indefinite => {
            let mut v = Vec::new();
            while !cbor::is_break(data, *offset)? {
                v.push(ref_decode(data, offset)?);
            }
            *offset += 1;
            Ok(v)
        }
    }
}

fn ref_eval(s: &Ref, sigs: &BTreeSet<Hash28>, iv: &ValidityInterval) -> ScriptVerdict {
    let pass = ScriptVerdict::NativeScriptPassed;
    match s {
        Ref::Sig(k) => {
            if sigs.contains(k) {
                pass
            } else {
                ScriptVerdict::NativeScriptFailed(NativeScriptFailure::MissingRequiredSignature { key_hash: k.clone() })
            }
        }
        Ref::All(v) => {
            for sub in v {
                if let ScriptVerdict::NativeScriptFailed(r) = ref_eval(sub, sigs, iv) {
                    return ScriptVerdict::NativeScriptFailed(r);
                }
            }
            pass
        }
        Ref::Any(v) => {
            for sub in v {
                if ref_eval(sub, sigs, iv) == pass {
                    return pass;
                }
            }
            ScriptVerdict::NativeScriptFailed(NativeScriptFailure::ThresholdNotMet { required: 1, provided: 0 })
        }
        Ref::MOfN(n, v) => {
            let Ok(needed) = u64::try_from(*n) else { return pass };
            if needed == 0 {
                return pass;
            }
            let mut passed = 0u64;
            for sub in v {
                if ref_eval(sub, sigs, iv) == pass {
                    passed += 1;
                    if passed >= needed {
                        return pass;
                    }
                }
            }
            ScriptVerdict::NativeScriptFailed(NativeScriptFailure::ThresholdNotMet { required: *n, provided: passed })
        }
        Ref::InvalidBefore(r) => match iv.invalid_before {
            Some(start) if *r <= start => pass,
            start => ScriptVerdict::NativeScriptFailed(NativeScriptFailure::TimelockStartNotMet {
                lock_start: SlotNo(*r),
                tx_start: start.map(SlotNo),
            }),
        },
        Ref::InvalidHereafter(r) => match iv.invalid_hereafter {
            Some(expiry) if expiry <= *r => pass,
            expiry => ScriptVerdict::NativeScriptFailed(NativeScriptFailure::TimelockExpiryNotMet {
                lock_expiry: SlotNo(*r),
                tx_expiry: expiry.map(SlotNo),
            }),
        },
    }
}

/// Intervals that put every timelock on both sides of its bound, with open ends.
fn intervals(points: &[u64]) -> Vec<ValidityInterval> {
    let mut out = vec![ValidityInterval::UNBOUNDED];
    for &a in points {
        out.push(ValidityInterval { invalid_before: Some(a), invalid_hereafter: None });
        out.push(ValidityInterval { invalid_before: None, invalid_hereafter: Some(a) });
        for &b in points {
            out.push(ValidityInterval { invalid_before: Some(a), invalid_hereafter: Some(b) });
        }
    }
    out
}

/// The reference tree in post-order, for comparison with the flat form.
fn ref_post_order(s: &Ref, out: &mut Vec<NativeScriptNode>) {
    let (subs, node): (&[Ref], NativeScriptNode) = match s {
        Ref::Sig(k) => (&[], NativeScriptNode::Sig(k.clone())),
        Ref::All(v) => (v, NativeScriptNode::All { arity: v.len() as u64 }),
        Ref::Any(v) => (v, NativeScriptNode::Any { arity: v.len() as u64 }),
        Ref::MOfN(n, v) => (v, NativeScriptNode::MOfN { threshold: *n, arity: v.len() as u64 }),
        Ref::InvalidBefore(x) => (&[], NativeScriptNode::InvalidBefore(*x)),
        Ref::InvalidHereafter(x) => (&[], NativeScriptNode::InvalidHereafter(*x)),
    };
    for sub in subs {
        ref_post_order(sub, out);
    }
    out.push(node);
}

/// Decode one script both ways from `bytes` at 0 and require the same result and end offset.
fn same_decode(bytes: &[u8]) {
    let mut a = 0;
    let new = decode_native_script(bytes, &mut a).map(|s| s.nodes().to_vec());
    let mut b = 0;
    let old = ref_decode(bytes, &mut b).map(|r| {
        let mut v = Vec::new();
        ref_post_order(&r, &mut v);
        v
    });
    assert_eq!((new, a), (old, b), "bytes {:02x?}", &bytes[..bytes.len().min(48)]);
}

// ---------------------------------------------------------------------------
// Script builders for constructed cases
// ---------------------------------------------------------------------------

fn key(b: u8) -> Hash28 {
    Hash28([b; 28])
}

fn enc_sig(out: &mut Vec<u8>, b: u8) {
    out.extend([0x82, 0x00, 0x58, 0x1c]);
    out.extend([b; 28]);
}

/// `[1, [[1, [ ... [0, key] ... ]]]]`, `depth` levels of `all`, each with one sub-script.
fn nested_all(depth: usize) -> Vec<u8> {
    let mut out = Vec::with_capacity(depth * 3 + 32);
    for _ in 0..depth {
        out.extend([0x82, 0x01, 0x81]);
    }
    enc_sig(&mut out, 0x42);
    out
}

/// Deterministic pseudo-random scripts for the differential (a fixed LCG; test-only).
struct Lcg(u64);
impl Lcg {
    fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        self.0 >> 33
    }
}

fn gen_script(rng: &mut Lcg, depth: u32, out: &mut Vec<u8>) {
    let pick = if depth == 0 { rng.next() % 3 } else { rng.next() % 7 };
    match pick {
        0 => enc_sig(out, (rng.next() % 4) as u8),
        1 => {
            out.extend([0x82, 0x04]);
            out.extend(cbor_uint(rng.next() % 300));
        }
        2 => {
            out.extend([0x82, 0x05]);
            out.extend(cbor_uint(rng.next() % 300));
        }
        3 | 4 | 5 | 6 => {
            let n = (rng.next() % 4) as usize;
            let indefinite = rng.next() % 4 == 0;
            match pick {
                3 => out.extend([0x82, 0x01]),
                4 => out.extend([0x82, 0x02]),
                _ => {
                    out.extend([0x83, 0x03]);
                    let t = (rng.next() % 7) as i64 - 2; // -2..=4
                    out.extend(cbor_int(t));
                }
            }
            if indefinite {
                out.push(0x9f);
            } else {
                out.push(0x80 | n as u8);
            }
            for _ in 0..n {
                gen_script(rng, depth - 1, out);
            }
            if indefinite {
                out.push(0xff);
            }
        }
        _ => unreachable!(),
    }
}

fn cbor_uint(v: u64) -> Vec<u8> {
    match v {
        0..=23 => vec![v as u8],
        24..=0xff => vec![0x18, v as u8],
        _ => vec![0x19, (v >> 8) as u8, v as u8],
    }
}

fn cbor_int(v: i64) -> Vec<u8> {
    if v >= 0 {
        cbor_uint(v as u64)
    } else {
        let mut h = cbor_uint((-1 - v) as u64);
        h[0] |= 0x20;
        h
    }
}

// ---------------------------------------------------------------------------
// CE-LV12-1: the real block on a small stack
// ---------------------------------------------------------------------------

#[test]
fn lv12_the_deep_preprod_block_decodes_and_evaluates_on_a_256_kib_thread() {
    let out = on_stack(SMALL_STACK, || {
        ade_ledger::block_validity::decode_block(BLOCK_133883340).expect("decode_block");
        let ws = witness_sets_of(BLOCK_133883340);
        let infos = decode_witness_infos(&ws).expect("decode_witness_infos");
        let deepest = infos
            .iter()
            .flat_map(|i| i.native_scripts.iter())
            .map(|s| s.script.nodes().len())
            .max()
            .unwrap();
        let mut verdicts = Vec::new();
        for i in &infos {
            for s in &i.native_scripts {
                verdicts.push(evaluate_native_script(&s.script, &i.available_key_hashes, &ValidityInterval::UNBOUNDED));
            }
        }
        let copy = infos.clone();
        let equal = copy.iter().zip(infos.iter()).all(|(a, b)| a.native_scripts == b.native_scripts);
        drop(copy);
        drop(infos);
        (deepest, verdicts.len(), equal)
    });
    // ~5,385 `all` levels plus the leaf: the script LV-10 found.
    assert!(out.0 > 5_000, "deepest script has {} nodes", out.0);
    assert!(out.1 > 0);
    assert!(out.2);
}

#[test]
fn lv12_the_deep_preprod_script_is_what_the_recursive_reference_decodes() {
    // The reference needs tens of MiB in debug for this block; it runs on a large stack.
    on_stack(512 << 20, || {
        let ws = witness_sets_of(BLOCK_133883340);
        let infos = decode_witness_infos(&ws).unwrap();
        let mut found = 0;
        for (o, list) in script_lists(&ws) {
            let _ = o;
            let mut a = 0;
            let new = decode_native_scripts(&list, &mut a).unwrap();
            let mut b = 0;
            let old = ref_decode_array(&list, &mut b).unwrap();
            assert_eq!(a, b);
            for (n, r) in new.iter().zip(old.iter()) {
                let mut v = Vec::new();
                ref_post_order(r, &mut v);
                assert_eq!(n.nodes(), v.as_slice());
                for i in &infos {
                    for iv in intervals(&[0, 133_883_340]) {
                        assert_eq!(
                            evaluate_native_script(n, &i.available_key_hashes, &iv),
                            ref_eval(r, &i.available_key_hashes, &iv)
                        );
                    }
                }
                found += 1;
            }
        }
        assert!(found > 0);
    });
}

// ---------------------------------------------------------------------------
// CE-LV12-2: constructed worst cases on a small stack
// ---------------------------------------------------------------------------

#[test]
fn lv12_a_script_as_deep_as_a_transaction_allows_round_trips_on_a_256_kib_thread() {
    // Each `all` level costs 3 bytes, so a 16,384-byte transaction holds fewer than 5,500 levels.
    // 50,000 levels is far past that.
    for depth in [5_500usize, 50_000] {
        let bytes = nested_all(depth);
        let (nodes, with_sig, without, equal) = on_stack(SMALL_STACK, move || {
            let mut o = 0;
            let s = decode_native_script(&bytes, &mut o).unwrap();
            assert_eq!(o, bytes.len());
            let with_sig = evaluate_native_script(&s, &[key(0x42)].into_iter().collect(), &ValidityInterval::UNBOUNDED);
            let without = evaluate_native_script(&s, &BTreeSet::new(), &ValidityInterval::UNBOUNDED);
            let c = s.clone();
            let equal = c == s;
            let nodes = s.nodes().len();
            drop(c);
            drop(s);
            (nodes, with_sig, without, equal)
        });
        assert_eq!(nodes, depth + 1);
        assert_eq!(with_sig, ScriptVerdict::NativeScriptPassed);
        assert_eq!(
            without,
            ScriptVerdict::NativeScriptFailed(NativeScriptFailure::MissingRequiredSignature { key_hash: key(0x42) })
        );
        assert!(equal);
    }
}

#[test]
fn lv12_a_deep_script_inside_a_witness_set_decodes_on_a_256_kib_thread() {
    // witness_sets = [ { 1: [ <script> ] } ]
    let mut ws = vec![0x81, 0xa1, 0x01, 0x81];
    ws.extend(nested_all(50_000));
    let n = on_stack(SMALL_STACK, move || {
        let infos = decode_witness_infos(&ws).unwrap();
        infos[0].native_scripts[0].script.nodes().len()
    });
    assert_eq!(n, 50_001);
}

// ---------------------------------------------------------------------------
// CE-LV12-4: decode and verdict differential against the recursive reference
// ---------------------------------------------------------------------------

/// Every witness-set native-script list in `ws`, with its offset (key 1 values, tag 258 stripped).
fn script_lists(ws: &[u8]) -> Vec<(usize, Vec<u8>)> {
    let mut out = Vec::new();
    let mut o = 0;
    let n = match cbor::read_array_header(ws, &mut o).unwrap() {
        ContainerEncoding::Definite(n, _) => n,
        ContainerEncoding::Indefinite => u64::MAX,
    };
    let mut i = 0;
    while i < n && !(n == u64::MAX && cbor::is_break(ws, o).unwrap()) {
        let keys = match cbor::read_map_header(ws, &mut o).unwrap() {
            ContainerEncoding::Definite(k, _) => k,
            ContainerEncoding::Indefinite => panic!("indefinite witness map in a fixture"),
        };
        for _ in 0..keys {
            let (k, _) = cbor::read_uint(ws, &mut o).unwrap();
            if k == 1 {
                if cbor::peek_major(ws, o).unwrap() == cbor::MAJOR_TAG {
                    cbor::read_tag(ws, &mut o).unwrap();
                }
                let (s, e) = cbor::skip_item(ws, &mut o).unwrap();
                out.push((s, ws[s..e].to_vec()));
            } else {
                cbor::skip_item(ws, &mut o).unwrap();
            }
        }
        i += 1;
    }
    out
}

#[test]
fn lv12_every_fixture_script_decodes_and_evaluates_as_the_reference_does() {
    on_stack(512 << 20, || {
        let mut scripts = 0;
        for (name, block) in FIXTURES {
            let ws = witness_sets_of(block);
            for (o, list) in script_lists(&ws) {
                let mut a = 0;
                let new = decode_native_scripts(&list, &mut a);
                let mut b = 0;
                let old = ref_decode_array(&list, &mut b);
                let (new, old) = (new.unwrap(), old.unwrap());
                assert_eq!(a, b, "block {name} list at {o}");
                assert_eq!(new.len(), old.len());
                for (n, r) in new.iter().zip(old.iter()) {
                    let mut v = Vec::new();
                    ref_post_order(r, &mut v);
                    assert_eq!(n.nodes(), v.as_slice(), "block {name} list at {o}");
                    for sigs in [BTreeSet::new(), [key(0)].into_iter().collect()] {
                        for iv in intervals(&[0, 133_000_000, u64::MAX]) {
                            assert_eq!(evaluate_native_script(n, &sigs, &iv), ref_eval(r, &sigs, &iv));
                        }
                    }
                    scripts += 1;
                }
            }
        }
        assert!(scripts > 0);
    });
}

#[test]
fn lv12_generated_scripts_decode_and_evaluate_as_the_reference_does() {
    on_stack(64 << 20, || {
        let mut rng = Lcg(0x1d_12);
        let sig_sets: Vec<BTreeSet<Hash28>> = vec![
            BTreeSet::new(),
            [key(0)].into_iter().collect(),
            [key(1), key(2)].into_iter().collect(),
            (0..4).map(key).collect(),
        ];
        for _ in 0..4_000 {
            let mut bytes = Vec::new();
            gen_script(&mut rng, 5, &mut bytes);
            same_decode(&bytes);
            let s = decode_native_script(&bytes, &mut 0).unwrap();
            let r = ref_decode(&bytes, &mut 0).unwrap();
            for sigs in &sig_sets {
                for iv in intervals(&[0, 150, 299, 1_000]) {
                    assert_eq!(evaluate_native_script(&s, sigs, &iv), ref_eval(&r, sigs, &iv));
                }
            }
            // Every truncation and every single-byte corruption decodes to the same error or value.
            for len in 0..bytes.len() {
                same_decode(&bytes[..len]);
            }
            for i in 0..bytes.len().min(40) {
                let mut c = bytes.clone();
                c[i] ^= 0x5a;
                same_decode(&c);
            }
        }
    });
}

#[test]
fn lv12_malformed_scripts_fail_exactly_as_the_reference_does() {
    let cases: &[&[u8]] = &[
        &[0x9f, 0x00, 0xff],                     // indefinite script array
        &[0x82, 0x06, 0x00],                     // unknown constructor
        &[0x82, 0x00, 0x41, 0x00],               // short key hash
        &[0x82, 0x03, 0x01, 0x80],               // MOfN as array(2)
        &[0x83, 0x03, 0x40, 0x80],               // non-integer threshold
        &[0x83, 0x03, 0x1b, 0x80, 0, 0, 0, 0, 0, 0, 0, 0x80], // threshold > i64::MAX
        &[0x82, 0x01, 0x82, 0x82, 0x04, 0x01],   // definite list short a script
        &[0x82, 0x01, 0x9f, 0x82, 0x04, 0x01],   // indefinite list never closed
        &[0x82, 0x01, 0x9f, 0xff],               // empty indefinite list
        &[0x82, 0x02, 0x80],                     // empty Any
        &[0x83, 0x00, 0x58, 0x1c],               // Sig in array(3), truncated hash
        &[0x82, 0x01, 0x81, 0x83, 0x01, 0x80, 0x00], // array(3) All: extra element left unread
        &[0x82, 0x04],                           // InvalidBefore without slot
        &[0x82, 0x01, 0xa0],                     // map where a list belongs
        &[],
    ];
    for case in cases {
        same_decode(case);
    }
}

#[test]
fn lv12_a_forged_sub_script_count_is_an_error_not_an_allocation() {
    // `[1, array(2^63)]` with nothing after it. The recursive decoder preallocated from the wire
    // count; the flat decoder allocates only for scripts it has read, and reports the end of input.
    for head in [vec![0x9b, 0x80, 0, 0, 0, 0, 0, 0, 0], vec![0x9b, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff]] {
        let mut bytes = vec![0x82, 0x01];
        bytes.extend(head);
        let r = decode_native_script(&bytes, &mut 0);
        assert!(matches!(r, Err(CodecError::UnexpectedEof { .. })), "{r:?}");
    }
}

#[test]
fn lv12_the_flat_type_has_no_recursive_field() {
    // A structural witness the gate also checks: a node never holds a script.
    let n = NativeScriptNode::All { arity: 1 };
    assert_eq!(n.arity(), 1);
    let s = NativeScript::from_post_order(vec![NativeScriptNode::InvalidBefore(1), n]).unwrap();
    assert_eq!(s.root(), &NativeScriptNode::All { arity: 1 });
}
