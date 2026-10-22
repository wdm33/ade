// SLICE LV-11 (DC-LEDGER-SCRIPT-01): a `script_n_of_k` threshold keeps Cardano's signed 64-bit
// meaning. Conway CDDL types it `int64`; cardano-ledger decodes a signed `Int` and evaluates
// `isValidMOf n … = n <= 0 || …`.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::collections::BTreeSet;

use ade_codec::allegra::script::decode_native_scripts;
use ade_codec::cbor::envelope::decode_block_envelope;
use ade_codec::conway::decode_conway_block;
use ade_ledger::error::NativeScriptFailure;
use ade_ledger::scripts::{evaluate_native_script, ScriptVerdict};
use ade_ledger::witness::decode_witness_infos;
use ade_types::allegra::script::NativeScript;
use ade_types::Hash28;

/// Preprod slot 133,902,219, block 5,184,855: the first block the LV-8 live node refused. Original
/// wire bytes from the peer's ImmutableDB, sha256 0fb060f0…0329 (LV-10 CE-LV10-2).
const BLOCK_133902219: &[u8] = include_bytes!("fixtures/block_133902219.cbor");

const KEY_A: [u8; 28] = [0x11; 28];

fn witness_sets_of(env_bytes: &[u8]) -> Vec<u8> {
    let env = decode_block_envelope(env_bytes).unwrap();
    let block = decode_conway_block(&env_bytes[env.block_start..env.block_end]).unwrap();
    block.decoded().witness_sets.clone()
}

/// CBOR integer head for `v`, major 0 for v >= 0 and major 1 (argument -1 - v) below.
fn cbor_int(v: i128) -> Vec<u8> {
    let (major, arg) = if v >= 0 { (0u8, v as u64) } else { (1u8, (-1 - v) as u64) };
    cbor_head(major, arg)
}

fn cbor_head(major: u8, arg: u64) -> Vec<u8> {
    let m = major << 5;
    match arg {
        0..=23 => vec![m | arg as u8],
        24..=0xff => vec![m | 24, arg as u8],
        0x100..=0xffff => [vec![m | 25], (arg as u16).to_be_bytes().to_vec()].concat(),
        0x1_0000..=0xffff_ffff => [vec![m | 26], (arg as u32).to_be_bytes().to_vec()].concat(),
        _ => [vec![m | 27], arg.to_be_bytes().to_vec()].concat(),
    }
}

/// `[[3, <threshold>, [[0, KEY_A]]]]`: one script_n_of_k over one signature script, inside the
/// witness-set script array.
fn n_of_k_with_threshold(threshold: Vec<u8>) -> Vec<u8> {
    let mut b = vec![0x81, 0x83, 0x03];
    b.extend(threshold);
    b.extend([0x81, 0x82, 0x00, 0x58, 0x1c]);
    b.extend(KEY_A);
    b
}

fn decode_threshold(threshold: Vec<u8>) -> Result<i64, ade_codec::error::CodecError> {
    let bytes = n_of_k_with_threshold(threshold);
    let mut o = 0;
    let scripts = decode_native_scripts(&bytes, &mut o)?;
    assert_eq!(o, bytes.len());
    match scripts.as_slice() {
        [NativeScript::MOfN(n, subs)] => {
            assert_eq!(subs, &vec![NativeScript::Sig(Hash28(KEY_A))]);
            Ok(*n)
        }
        other => panic!("expected one MOfN, got {other:?}"),
    }
}

fn sig(k: [u8; 28]) -> NativeScript {
    NativeScript::Sig(Hash28(k))
}

fn eval(script: &NativeScript, sigs: &[[u8; 28]]) -> ScriptVerdict {
    let available: BTreeSet<Hash28> = sigs.iter().map(|k| Hash28(*k)).collect();
    evaluate_native_script(script, &available, 0)
}

// ---------------------------------------------------------------------------
// CE-LV11-1: the real block
// ---------------------------------------------------------------------------

#[test]
fn lv11_the_refused_preprod_block_now_decodes_its_negative_threshold() {
    let ws = witness_sets_of(BLOCK_133902219);
    let infos = decode_witness_infos(&ws).expect("the witness sets of block 5,184,855 must decode");
    let tx0 = &infos[0].native_scripts;
    assert_eq!(tx0.len(), 1);
    match &tx0[0] {
        NativeScript::MOfN(n, subs) => {
            assert_eq!(*n, -1);
            assert_eq!(subs.len(), 2);
            assert!(subs.iter().all(|s| matches!(s, NativeScript::Sig(_))));
        }
        other => panic!("tx 0 script is not script_n_of_k: {other:?}"),
    }
}

#[test]
fn lv11_the_real_negative_threshold_script_passes_with_no_signatures() {
    let ws = witness_sets_of(BLOCK_133902219);
    let infos = decode_witness_infos(&ws).unwrap();
    let script = &infos[0].native_scripts[0];
    assert_eq!(eval(script, &[]), ScriptVerdict::NativeScriptPassed);
}

// ---------------------------------------------------------------------------
// CE-LV11-2: decode boundaries
// ---------------------------------------------------------------------------

#[test]
fn lv11_thresholds_inside_int64_decode_to_exactly_their_value() {
    let u32_max = i128::from(u32::MAX);
    for v in [
        -1i128,
        0,
        1,
        u32_max,
        u32_max + 1,
        1i128 << 32,
        i128::from(i64::MAX),
        i128::from(i64::MIN),
        -24,
        -25,
        -256,
        -257,
    ] {
        let got = decode_threshold(cbor_int(v)).unwrap_or_else(|e| panic!("{v} refused: {e:?}"));
        assert_eq!(i128::from(got), v, "threshold {v} decoded as {got}");
    }
}

#[test]
fn lv11_thresholds_outside_int64_are_refused() {
    // i64::MAX + 1 as major 0, i64::MIN - 1 as major 1 (argument 2^63), and u64::MAX.
    for (label, head) in [
        ("i64::MAX + 1", cbor_head(0, 1u64 << 63)),
        ("i64::MIN - 1", cbor_head(1, 1u64 << 63)),
        ("u64::MAX", cbor_head(0, u64::MAX)),
        ("-u64::MAX - 1", cbor_head(1, u64::MAX)),
    ] {
        let r = decode_threshold(head);
        assert!(
            matches!(
                r,
                Err(ade_codec::error::CodecError::InvalidCborStructure {
                    detail: "native script MOfN threshold outside int64",
                    ..
                })
            ),
            "{label}: {r:?}"
        );
    }
}

#[test]
fn lv11_a_non_integer_threshold_is_still_refused() {
    let r = decode_threshold(vec![0x40]); // empty byte string
    assert!(matches!(r, Err(ade_codec::error::CodecError::UnexpectedCborType { .. })), "{r:?}");
}

// ---------------------------------------------------------------------------
// CE-LV11-3: evaluation matches isValidMOf
// ---------------------------------------------------------------------------

#[test]
fn lv11_a_threshold_of_zero_or_below_is_satisfied_outright() {
    for n in [0i64, -1, -2, i64::MIN] {
        assert_eq!(eval(&NativeScript::MOfN(n, vec![sig(KEY_A)]), &[]), ScriptVerdict::NativeScriptPassed);
        assert_eq!(eval(&NativeScript::MOfN(n, vec![]), &[]), ScriptVerdict::NativeScriptPassed);
    }
}

#[test]
fn lv11_a_threshold_of_one_needs_one_passing_sub_script() {
    let s = NativeScript::MOfN(1, vec![sig(KEY_A)]);
    assert_eq!(eval(&s, &[KEY_A]), ScriptVerdict::NativeScriptPassed);
    assert_eq!(
        eval(&s, &[]),
        ScriptVerdict::NativeScriptFailed(NativeScriptFailure::ThresholdNotMet { required: 1, provided: 0 })
    );
}

#[test]
fn lv11_a_threshold_above_u32_is_not_truncated_into_a_pass() {
    let u32_max = i64::from(u32::MAX);
    for n in [1i64 << 32, u32_max + 1, u32_max + 2, i64::MAX] {
        let s = NativeScript::MOfN(n, vec![sig(KEY_A)]);
        assert_eq!(
            eval(&s, &[KEY_A]),
            ScriptVerdict::NativeScriptFailed(NativeScriptFailure::ThresholdNotMet { required: n, provided: 1 }),
            "threshold {n}"
        );
    }
}

#[test]
fn lv11_a_decoded_threshold_of_two_to_the_32_fails_end_to_end() {
    let n = decode_threshold(cbor_int(1i128 << 32)).unwrap();
    let s = NativeScript::MOfN(n, vec![sig(KEY_A)]);
    assert!(matches!(eval(&s, &[KEY_A]), ScriptVerdict::NativeScriptFailed(_)));
}
