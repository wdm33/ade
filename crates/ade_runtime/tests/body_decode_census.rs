//! BODY-DECODE CENSUS — read-only, evidence-only, `#[ignore]`d by default. Opened by SLICE LV-10.
//!
//! Walks cardano ImmutableDB chunk files (each a concatenation of era-tagged block envelopes, the
//! bytes block-fetch serves), and runs on each block's ORIGINAL bytes the decode stages the relay
//! path runs: `decode_block` (the receive and validity entry), then the body authority's decoders
//! (block, witness sets, each transaction body, vkey witness sets). The first block any stage refuses
//! is reported with the unmapped `CodecError` where the stage exposes one, and its bytes are written
//! unmodified to `ADE_CENSUS_OUT`. Nothing is re-encoded.
//!
//! With `ADE_CENSUS_CHAINDB` (a COPY of an Ade `chain.db`; opening it is read-write), every scanned
//! block at or before that store's tip is compared byte for byte with the store's copy.
//!
//! ```text
//! ADE_CENSUS_CHUNKS="/abs/db/immutable/06198.chunk ..." ADE_CENSUS_SLOTS=133890000-133920000 \
//! ADE_CENSUS_OUT=/abs/out/dir ADE_CENSUS_CHAINDB=/abs/chain-copy.db \
//!   cargo test -p ade_runtime --test body_decode_census -- --ignored --nocapture
//! ```

use ade_codec::cbor::{self, envelope::decode_block_envelope, ContainerEncoding};
use ade_codec::conway::{decode_conway_block, tx::decode_conway_tx_body};
use ade_ledger::block_validity::decode_block;
use ade_runtime::chaindb::{ChainDb, PersistentChainDb, PersistentChainDbOptions};
use ade_types::allegra::script::NativeScriptNode;
use ade_types::{CardanoEra, SlotNo};

fn hex(b: &[u8]) -> String {
    b.iter().map(|x| format!("{x:02x}")).collect()
}

/// The CBOR head at `off`: major type and the low 5 bits.
fn head_at(data: &[u8], off: usize) -> String {
    match data.get(off) {
        Some(&b) => format!("byte 0x{b:02x} = major {} info {}", b >> 5, b & 0x1f),
        None => "past the end".to_string(),
    }
}

fn window(data: &[u8], off: usize) -> String {
    let lo = off.saturating_sub(16);
    let hi = (off + 24).min(data.len());
    format!("[{lo}..{hi}) {} | {}", hex(&data[lo..off.min(hi)]), hex(&data[off.min(hi)..hi]))
}

/// Slot and block number read straight from the header, so a block the codec refuses can still be
/// placed. Inner block = array[header = array[header_body = array[block_no, slot, prev_hash, ..], ..], ..].
fn place(inner: &[u8]) -> Option<(u64, u64, Option<Vec<u8>>)> {
    let mut o = 0;
    cbor::read_array_header(inner, &mut o).ok()?;
    cbor::read_array_header(inner, &mut o).ok()?;
    cbor::read_array_header(inner, &mut o).ok()?;
    let (block_no, _) = cbor::read_uint(inner, &mut o).ok()?;
    let (slot, _) = cbor::read_uint(inner, &mut o).ok()?;
    let prev = cbor::read_bytes(inner, &mut o).ok().map(|(b, _)| b);
    Some((slot, block_no, prev))
}

fn header_hash(inner: &[u8]) -> Option<Vec<u8>> {
    let mut o = 0;
    cbor::read_array_header(inner, &mut o).ok()?;
    let (s, e) = cbor::skip_item(inner, &mut o).ok()?;
    Some(ade_crypto::blake2b::blake2b_256(&inner[s..e]).0.to_vec())
}

/// Top-level parts of the inner block with their [start, end) offsets.
fn parts(inner: &[u8]) -> Vec<(&'static str, usize, usize)> {
    let names = ["header", "tx_bodies", "witness_sets", "auxiliary_data", "invalid_txs"];
    let mut out = Vec::new();
    let mut o = 0;
    let n = match cbor::read_array_header(inner, &mut o) {
        Ok(ContainerEncoding::Definite(n, _)) => n as usize,
        _ => return out,
    };
    for name in names.iter().take(n) {
        match cbor::skip_item(inner, &mut o) {
            Ok((s, e)) => out.push((*name, s, e)),
            Err(e) => {
                println!("  PART {name} at {o}: skip_item refused: {e:?}");
                break;
            }
        }
    }
    out
}

fn stage(name: &str) {
    if std::env::var("ADE_CENSUS_TRACE").is_ok() {
        eprintln!("  stage {name}");
    }
}

/// Runs every stage; returns a description of the first refusal, or None.
fn first_refusal(env_bytes: &[u8]) -> Option<String> {
    stage("decode_block");
    if let Err(e) = decode_block(env_bytes) {
        let mut msg = format!("STAGE decode_block (receive/validity entry): {e:?}");
        match decode_block_envelope(env_bytes) {
            Err(c) => msg.push_str(&format!("\n  envelope: {c:?}")),
            Ok(env) => {
                let inner = &env_bytes[env.block_start..env.block_end];
                msg.push_str(&format!("\n  envelope ok: era {:?}, inner [{}..{})", env.era, env.block_start, env.block_end));
                if env.era == CardanoEra::Conway {
                    if let Err(c) = decode_conway_block(inner) {
                        msg.push_str(&format!("\n  decode_conway_block: {c:?}"));
                    } else {
                        msg.push_str("\n  decode_conway_block ok: the refusal is in the header-input stage");
                    }
                }
            }
        }
        return Some(msg);
    }
    let env = decode_block_envelope(env_bytes).ok()?;
    if env.era != CardanoEra::Conway {
        return None;
    }
    let inner = &env_bytes[env.block_start..env.block_end];
    stage("decode_conway_block");
    let block = match decode_conway_block(inner) {
        Ok(b) => b,
        Err(c) => return Some(format!("STAGE body decode_conway_block: {c:?}")),
    };
    let b = block.decoded();
    if b.tx_count == 0 {
        return None;
    }
    stage("decode_witness_infos");
    if let Err(e) = ade_ledger::witness::decode_witness_infos(&b.witness_sets) {
        return Some(format!("STAGE body decode_witness_infos (offsets relative to witness_sets): {e:?}"));
    }
    stage("tx bodies");
    let bodies = &b.tx_bodies;
    let mut o = 0;
    let n = match cbor::read_array_header(bodies, &mut o) {
        Ok(ContainerEncoding::Definite(n, _)) => n,
        Ok(ContainerEncoding::Indefinite) => u64::MAX,
        Err(c) => return Some(format!("STAGE body tx_bodies header: {c:?}")),
    };
    let mut i = 0u64;
    while i < n && !(n == u64::MAX && cbor::is_break(bodies, o).unwrap_or(true)) {
        let start = o;
        if let Err(c) = decode_conway_tx_body(bodies, &mut o) {
            let mut end = start;
            let span = cbor::skip_item(bodies, &mut end).map(|_| end).ok();
            return Some(format!(
                "STAGE body decode_conway_tx_body, tx {i}: {c:?}\n  tx body starts at {start} in tx_bodies (ends {span:?}); offsets are relative to tx_bodies"
            ));
        }
        i += 1;
    }
    stage("decode_conway_vkey_witness_sets");
    if let Err(e) = ade_ledger::shelley::decode_conway_vkey_witness_sets(&b.witness_sets) {
        return Some(format!("STAGE body decode_conway_vkey_witness_sets (offsets relative to witness_sets): {e:?}"));
    }
    None
}

#[test]
#[ignore = "evidence-only; requires ADE_CENSUS_CHUNKS pointing at ImmutableDB chunk files"]
fn find_the_first_refused_block() {
    let chunks = std::env::var("ADE_CENSUS_CHUNKS").expect("set ADE_CENSUS_CHUNKS");
    let (lo, hi) = match std::env::var("ADE_CENSUS_SLOTS") {
        Ok(r) => {
            let (a, b) = r.split_once('-').expect("ADE_CENSUS_SLOTS is lo-hi");
            (a.parse::<u64>().unwrap(), b.parse::<u64>().unwrap())
        }
        Err(_) => (0, u64::MAX),
    };
    let out = std::env::var("ADE_CENSUS_OUT").ok();
    let db = std::env::var("ADE_CENSUS_CHAINDB")
        .ok()
        .map(|p| PersistentChainDb::open(PersistentChainDbOptions::at(p)).expect("open chain.db copy"));
    let tip = db.as_ref().map(|d| d.tip().expect("tip").expect("non-empty store"));
    if let Some(t) = &tip {
        println!("ADE TIP slot={} hash={}", t.slot.0, hex(&t.hash.0));
    }

    let (mut scanned, mut compared, mut identical) = (0u64, 0u64, 0u64);
    for path in chunks.split_whitespace() {
        let data = std::fs::read(path).expect("read chunk");
        let mut off = 0;
        while off < data.len() {
            let start = off;
            cbor::skip_item(&data, &mut off).expect("block item");
            let env_bytes = &data[start..off];
            let Ok(env) = decode_block_envelope(env_bytes) else { continue };
            let inner = &env_bytes[env.block_start..env.block_end];
            let Some((slot, block_no, prev)) = place(inner) else { continue };
            if slot < lo || slot > hi {
                continue;
            }
            scanned += 1;

            if let (Some(d), Some(t)) = (&db, &tip) {
                if slot <= t.slot.0 {
                    if let Some(stored) = d.get_block_by_slot(SlotNo(slot)).expect("read") {
                        compared += 1;
                        if stored.bytes == env_bytes {
                            identical += 1;
                        } else {
                            println!("BYTE MISMATCH slot={slot}: store {} bytes, chunk {} bytes", stored.bytes.len(), env_bytes.len());
                        }
                    }
                }
            }

            if std::env::var("ADE_CENSUS_TRACE").is_ok() {
                eprintln!("TRACE slot={slot} block_no={block_no} bytes={}", env_bytes.len());
            }
            if let Some(why) = first_refusal(env_bytes) {
                let hh = header_hash(inner).map(|h| hex(&h)).unwrap_or_default();
                println!("REFUSED slot={slot} block_no={block_no} header_hash={hh}");
                println!("  chunk {path} at byte {start}, envelope {} bytes", env_bytes.len());
                println!("  prev_hash={}", prev.as_deref().map(hex).unwrap_or_default());
                if let Some(t) = &tip {
                    println!("  prev_hash == Ade tip hash: {}", prev.as_deref() == Some(&t.hash.0[..]));
                }
                println!("{why}");
                for (name, s, e) in parts(inner) {
                    println!("  PART {name} inner [{s}..{e}) = envelope [{}..{})", s + env.block_start, e + env.block_start);
                }
                println!("  inner head at 9: {}  window {}", head_at(inner, 9), window(inner, 9));
                if let Some(dir) = &out {
                    let f = format!("{dir}/block_{slot}.cbor");
                    std::fs::write(&f, env_bytes).expect("write block");
                    println!("  WROTE {f}");
                }
                println!("SUMMARY scanned={scanned} store_compared={compared} store_identical={identical}");
                return;
            }
        }
    }
    println!("SUMMARY scanned={scanned} store_compared={compared} store_identical={identical} refused=none");
}

/// Decodes the native-script list of witness set 0 of a saved block with the codec's own decoder, to
/// recover the `CodecError` that `decode_witness_infos` maps to a bare offset and reason.
///
/// ```text
/// ADE_CENSUS_BLOCK=/abs/block_133902219.cbor \
///   cargo test -p ade_runtime --test body_decode_census -- --ignored --nocapture unmapped
/// ```
#[test]
#[ignore = "evidence-only; requires ADE_CENSUS_BLOCK pointing at a saved block"]
fn unmapped_native_script_error() {
    let path = std::env::var("ADE_CENSUS_BLOCK").expect("set ADE_CENSUS_BLOCK");
    let env_bytes = std::fs::read(path).expect("read block");
    let env = decode_block_envelope(&env_bytes).expect("envelope");
    let inner = &env_bytes[env.block_start..env.block_end];
    let block = decode_conway_block(inner).expect("conway block");
    let ws = &block.decoded().witness_sets;
    // witness_sets = array[ map{ 1: tag 258 [ native_script, .. ] }, .. ]
    let mut o = 0;
    cbor::read_array_header(ws, &mut o).expect("witness_sets array");
    cbor::read_map_header(ws, &mut o).expect("witness set 0 map");
    let (key, _) = cbor::read_uint(ws, &mut o).expect("key");
    let (tag, _) = cbor::read_tag(ws, &mut o).expect("set tag");
    println!("witness set 0: key {key}, tag {tag}, script list at witness_sets offset {o}");
    let r = ade_codec::allegra::script::decode_native_scripts(ws, &mut o);
    println!("decode_native_scripts: {r:?}");
}

/// SLICE LV-11 CE-LV11-6: every `script_n_of_k` threshold in the blocks an Ade store holds, read raw
/// from the CBOR (both integer major types, full 64-bit argument), independently of the native-script
/// decoder. The script walk keeps an explicit stack, so no script's nesting depth depends on thread
/// stack. Witness-set key 1 is the only place Ade decodes native scripts.
///
/// ```text
/// ADE_CENSUS_CHAINDB=/abs/chain-copy.db \
///   cargo test -p ade_runtime --test body_decode_census -- --ignored --nocapture persisted_threshold
/// ```
#[test]
#[ignore = "evidence-only; requires ADE_CENSUS_CHAINDB pointing at a COPY of an Ade chain.db"]
fn persisted_threshold_census() {
    let path = std::env::var("ADE_CENSUS_CHAINDB").expect("set ADE_CENSUS_CHAINDB (a COPY)");
    let db = PersistentChainDb::open(PersistentChainDbOptions::at(path)).expect("open chain.db copy");
    let tip = db.tip().expect("tip").expect("non-empty store");

    #[derive(Default)]
    struct Tally {
        blocks: u64,
        non_conway: u64,
        witness_sets: u64,
        script_lists: u64,
        top_scripts: u64,
        nodes: u64,
        mofn: u64,
        min: Option<i128>,
        max: Option<i128>,
        below_zero: u64,
        above_u32: u64,
        indefinite_script_arrays: u64,
        max_depth: usize,
        outside: Vec<(u64, u64, i128)>,
        raw: Vec<i128>,
    }
    let mut t = Tally::default();
    let mut decoded: Vec<i128> = Vec::new();
    let mut decoded_nodes = 0u64;
    let (mut first, mut last) = (None, 0u64);

    enum Frame {
        List(Option<u64>),
        Break,
    }

    for item in db.iter_from_slot(SlotNo(0)).expect("iter") {
        let b = item.expect("stored block");
        t.blocks += 1;
        first.get_or_insert(b.slot.0);
        last = b.slot.0;
        let env = decode_block_envelope(&b.bytes).expect("envelope");
        if env.era != CardanoEra::Conway {
            t.non_conway += 1;
            continue;
        }
        let block = decode_conway_block(&b.bytes[env.block_start..env.block_end]).expect("conway block");
        let ws = &block.decoded().witness_sets;
        let mut o = 0;
        let sets = match cbor::read_array_header(ws, &mut o).expect("witness_sets") {
            ContainerEncoding::Definite(n, _) => Some(n),
            ContainerEncoding::Indefinite => None,
        };
        let mut tx = 0u64;
        loop {
            match sets {
                Some(n) if tx == n => break,
                None if cbor::is_break(ws, o).expect("break") => break,
                _ => {}
            }
            t.witness_sets += 1;
            let keys = match cbor::read_map_header(ws, &mut o).expect("witness set map") {
                ContainerEncoding::Definite(n, _) => Some(n),
                ContainerEncoding::Indefinite => None,
            };
            let mut k = 0u64;
            loop {
                match keys {
                    Some(n) if k == n => break,
                    None if cbor::is_break(ws, o).expect("break") => {
                        o += 1;
                        break;
                    }
                    _ => {}
                }
                k += 1;
                let (key, _) = cbor::read_uint(ws, &mut o).expect("witness key");
                if key != 1 {
                    cbor::skip_item(ws, &mut o).expect("skip witness value");
                    continue;
                }
                t.script_lists += 1;
                if cbor::peek_major(ws, o).expect("peek") == cbor::MAJOR_TAG {
                    cbor::read_tag(ws, &mut o).expect("set tag");
                }
                let top = match cbor::read_array_header(ws, &mut o).expect("script list") {
                    ContainerEncoding::Definite(n, _) => Some(n),
                    ContainerEncoding::Indefinite => None,
                };
                t.top_scripts += top.unwrap_or(0);
                let mut stack = vec![Frame::List(top)];
                while let Some(frame) = stack.last_mut() {
                    match frame {
                        Frame::Break => {
                            assert!(cbor::is_break(ws, o).expect("break"), "indefinite script not closed");
                            o += 1;
                            stack.pop();
                            continue;
                        }
                        Frame::List(Some(0)) => {
                            stack.pop();
                            continue;
                        }
                        Frame::List(None) => {
                            if cbor::is_break(ws, o).expect("break") {
                                o += 1;
                                stack.pop();
                                continue;
                            }
                        }
                        Frame::List(Some(n)) => *n -= 1,
                    }
                    t.nodes += 1;
                    t.max_depth = t.max_depth.max(stack.len());
                    if matches!(cbor::read_array_header(ws, &mut o).expect("script"), ContainerEncoding::Indefinite) {
                        t.indefinite_script_arrays += 1;
                        stack.push(Frame::Break);
                    }
                    let (ctor, _) = cbor::read_uint(ws, &mut o).expect("script constructor");
                    let sub = |ws: &[u8], o: &mut usize| match cbor::read_array_header(ws, o).expect("sub-scripts") {
                        ContainerEncoding::Definite(n, _) => Frame::List(Some(n)),
                        ContainerEncoding::Indefinite => Frame::List(None),
                    };
                    match ctor {
                        0 => {
                            cbor::read_bytes(ws, &mut o).expect("key hash");
                        }
                        1 | 2 => stack.push(sub(ws, &mut o)),
                        3 => {
                            let (arg, neg, _) = cbor::read_any_int(ws, &mut o).expect("threshold");
                            let v = if neg { -1 - i128::from(arg) } else { i128::from(arg) };
                            t.mofn += 1;
                            t.raw.push(v);
                            t.min = Some(t.min.map_or(v, |m| m.min(v)));
                            t.max = Some(t.max.map_or(v, |m| m.max(v)));
                            if v < 0 {
                                t.below_zero += 1;
                            }
                            if v > i128::from(u32::MAX) {
                                t.above_u32 += 1;
                            }
                            if v < 0 || v > i128::from(u32::MAX) {
                                t.outside.push((b.slot.0, tx, v));
                            }
                            stack.push(sub(ws, &mut o));
                        }
                        4 | 5 => {
                            cbor::read_uint(ws, &mut o).expect("slot");
                        }
                        other => panic!("slot {} tx {tx}: native script constructor {other}", b.slot.0),
                    }
                }
            }
            tx += 1;
        }
        if sets.is_none() {
            o += 1;
        }
        assert_eq!(o, ws.len(), "slot {}: witness_sets not fully walked", b.slot.0);

        // Cross-check: the production witness decoder accepts the block and yields the same thresholds.
        let infos = ade_ledger::witness::decode_witness_infos(ws)
            .unwrap_or_else(|e| panic!("slot {}: decode_witness_infos refused a stored block: {e:?}", b.slot.0));
        for s in infos.iter().flat_map(|i| i.native_scripts.iter()) {
            decoded_nodes += s.script.nodes().len() as u64;
            for node in s.script.nodes() {
                if let NativeScriptNode::MOfN { threshold, .. } = node {
                    decoded.push(i128::from(*threshold));
                }
            }
        }
    }
    let mut raw: Vec<i128> = Vec::new();
    std::mem::swap(&mut raw, &mut t.raw);
    raw.sort_unstable();
    decoded.sort_unstable();
    println!("DECODER CROSS-CHECK: {} thresholds decoded, multiset equal to raw: {}", decoded.len(), raw == decoded);
    assert_eq!(raw, decoded, "the decoder's thresholds differ from the raw CBOR");
    println!("DECODER CROSS-CHECK: {decoded_nodes} script nodes decoded, raw walk {}", t.nodes);
    assert_eq!(decoded_nodes, t.nodes, "the decoder's node count differs from the raw CBOR");

    println!("STORE TIP slot={} hash={}", tip.slot.0, hex(&tip.hash.0));
    println!("BLOCKS {} (slots {}..={}), non-Conway {}", t.blocks, first.unwrap_or(0), last, t.non_conway);
    println!(
        "WITNESS SETS {}  native-script lists {}  top-level scripts {}  script nodes {}  max depth {}",
        t.witness_sets, t.script_lists, t.top_scripts, t.nodes, t.max_depth
    );
    println!("INDEFINITE SCRIPT ARRAYS {}", t.indefinite_script_arrays);
    println!("SCRIPT_N_OF_K {}  min {:?}  max {:?}", t.mofn, t.min, t.max);
    println!("OUTSIDE 0..=u32::MAX: {} (below 0: {}, above u32::MAX: {})", t.outside.len(), t.below_zero, t.above_u32);
    for (slot, tx, v) in &t.outside {
        println!("  slot {slot} tx {tx} threshold {v}");
    }
    assert_eq!(last, tip.slot.0, "the walk must end at the store tip");
}

/// LV-13 census: the witness-script closure (DC-LEDGER-SCRIPT-03) over every stored block, before it
/// is wired to reject. A forward UTxO is built from the outputs created inside the interval, so the
/// transactions whose spend and reference inputs all lie inside it get the exact verdict; the rest
/// get the reduced one. cardano-node accepted every block here, so any rejection is an Ade bug until
/// shown otherwise.
///
/// ```text
/// ADE_CENSUS_CHAINDB=/abs/chain-copy.db \
///   cargo test --release -p ade_runtime --test body_decode_census persisted_script_witness_census \
///   -- --ignored --nocapture
/// ```
#[test]
#[ignore = "evidence-only; requires ADE_CENSUS_CHAINDB pointing at a COPY of an Ade chain.db"]
fn persisted_script_witness_census() {
    use ade_ledger::scripts::{evaluate_native_script, ScriptVerdict};
    use ade_ledger::tx_validity::script_witnesses::{
        native_script_verdict, reference_scripts, scripts_needed, script_signers, validity_interval,
        ExactScriptVerdict, NativeScriptVerdict, ReducedScriptVerdict, ReferenceScript,
        ResolvedScriptContext, ResolvedScriptOutput, ScriptPurpose,
    };
    use ade_ledger::witness::decode_witness_info_single;
    use ade_types::tx::TxIn;
    use std::collections::{BTreeMap, BTreeSet};

    let path = std::env::var("ADE_CENSUS_CHAINDB").expect("set ADE_CENSUS_CHAINDB (a COPY)");
    let db = PersistentChainDb::open(PersistentChainDbOptions::at(path)).expect("open chain.db copy");
    let tip = db.tip().expect("tip").expect("non-empty store");

    // Slices of each element of a CBOR array.
    fn elements(data: &[u8]) -> Vec<&[u8]> {
        let mut o = 0;
        let n = match cbor::read_array_header(data, &mut o).expect("array") {
            ContainerEncoding::Definite(n, _) => Some(n),
            ContainerEncoding::Indefinite => None,
        };
        let mut out = Vec::new();
        loop {
            match n {
                Some(n) if out.len() as u64 == n => break,
                None if cbor::is_break(data, o).expect("break") => break,
                _ => {}
            }
            let (s, e) = cbor::skip_item(data, &mut o).expect("element");
            out.push(&data[s..e]);
        }
        out
    }

    // The block-slot rule the evaluator used before LV-13, for the "verdict changed" tally.
    fn block_slot_passes(script: &ade_types::allegra::script::NativeScript, signers: &BTreeSet<ade_types::Hash28>, slot: u64) -> bool {
        let mut st: Vec<bool> = Vec::new();
        for node in script.nodes() {
            let k = node.arity() as usize;
            let subs = st.split_off(st.len() - k);
            let passed = subs.iter().filter(|b| **b).count() as i64;
            st.push(match node {
                NativeScriptNode::Sig(h) => signers.contains(h),
                NativeScriptNode::All { .. } => subs.iter().all(|b| *b),
                NativeScriptNode::Any { .. } => subs.iter().any(|b| *b),
                NativeScriptNode::MOfN { threshold, .. } => *threshold <= passed,
                NativeScriptNode::InvalidBefore(l) => slot >= *l,
                NativeScriptNode::InvalidHereafter(l) => slot < *l,
            });
        }
        st.pop().unwrap_or(true)
    }

    let mut utxo: BTreeMap<TxIn, ResolvedScriptOutput> = BTreeMap::new();
    // ADE_CENSUS_FIXTURE_OUT: write the first exact-valid tx of each fixture kind (its stored block
    // and the outputs its spend and reference inputs resolve to) for the committed LV-13 fixtures.
    let fixture_out = std::env::var("ADE_CENSUS_FIXTURE_OUT").ok();
    let mut written: BTreeSet<&'static str> = BTreeSet::new();
    let mut c: BTreeMap<&'static str, u64> = BTreeMap::new();
    let mut purposes: BTreeMap<ScriptPurpose, u64> = BTreeMap::new();
    let mut findings: Vec<String> = Vec::new();
    let mut changed: Vec<String> = Vec::new();
    let (mut first, mut last) = (None, 0u64);
    let bump = |c: &mut BTreeMap<&'static str, u64>, k: &'static str, n: u64| *c.entry(k).or_insert(0) += n;

    for item in db.iter_from_slot(SlotNo(0)).expect("iter") {
        let b = item.expect("stored block");
        bump(&mut c, "blocks", 1);
        first.get_or_insert(b.slot.0);
        last = b.slot.0;
        let env = decode_block_envelope(&b.bytes).expect("envelope");
        if env.era != CardanoEra::Conway {
            bump(&mut c, "non-Conway blocks", 1);
            continue;
        }
        let block = decode_conway_block(&b.bytes[env.block_start..env.block_end]).expect("conway block");
        let blk = block.decoded();
        let invalid = ade_ledger::plutus_eval::decode_invalid_tx_indices(blk.invalid_txs.as_deref());
        let bodies = elements(&blk.tx_bodies);
        let wsets = elements(&blk.witness_sets);
        assert_eq!(bodies.len(), wsets.len(), "bodies and witness sets pair up");
        for (i, (body_bytes, ws_bytes)) in bodies.iter().zip(wsets.iter()).enumerate() {
            bump(&mut c, "transactions", 1);
            let mut o = 0;
            let body = decode_conway_tx_body(body_bytes, &mut o).expect("tx body");
            let wi = decode_witness_info_single(ws_bytes).expect("witness set");
            let tx_id = ade_crypto::blake2b_256(body_bytes);
            let signers = script_signers(&wi);
            let iv = validity_interval(&body);
            let where_ = format!("slot {} tx {i} id {}", b.slot.0, hex(&tx_id.0));

            bump(&mut c, "witness native scripts", wi.native_scripts.len() as u64);
            bump(&mut c, "witness Plutus scripts", wi.plutus_script_hashes.len() as u64);
            if !wi.native_scripts.is_empty() {
                bump(&mut c, "txs with witness native scripts", 1);
            }
            if !wi.bootstrap_key_hashes.is_empty() {
                bump(&mut c, "txs with bootstrap witnesses", 1);
            }
            for w in &wi.native_scripts {
                let new_pass = matches!(evaluate_native_script(&w.script, &signers, &iv), ScriptVerdict::NativeScriptPassed);
                if new_pass {
                    bump(&mut c, "witness native scripts passing", 1);
                }
                let timelock = w.script.nodes().iter().any(|n| {
                    matches!(n, NativeScriptNode::InvalidBefore(_) | NativeScriptNode::InvalidHereafter(_))
                });
                if timelock {
                    bump(&mut c, "witness native scripts with a timelock", 1);
                    if block_slot_passes(&w.script, &signers, b.slot.0) != new_pass {
                        bump(&mut c, "timelock verdicts changed vs block slot", 1);
                        if changed.len() < 20 {
                            changed.push(format!("{where_} script {} interval {:?} new_pass {new_pass}", hex(&w.hash.0), iv));
                        }
                    }
                }
            }

            match native_script_verdict(&body, &wi, None).expect("reduced verdict") {
                NativeScriptVerdict::Reduced(ReducedScriptVerdict::DefiniteReject(e)) => {
                    bump(&mut c, "REDUCED definite rejects", 1);
                    findings.push(format!("REDUCED {where_}: {e:?}"));
                }
                NativeScriptVerdict::Reduced(ReducedScriptVerdict::Undetermined) => {}
                NativeScriptVerdict::Exact(_) => panic!("no context, so no exact verdict"),
            }

            let ctx = ResolvedScriptContext::resolve(&body, |t| utxo.get(t).cloned());
            match &ctx {
                None => bump(&mut c, "txs with an unresolved spend or reference input", 1),
                Some(ctx) => {
                    bump(&mut c, "txs fully resolved", 1);
                    let needed = scripts_needed(&body, ctx).expect("needed");
                    for (p, _) in &needed.provenance {
                        *purposes.entry(*p).or_insert(0) += 1;
                    }
                    let refs = reference_scripts(&body, ctx).expect("refs");
                    for h in &needed.hashes {
                        match refs.get(h) {
                            Some(ReferenceScript::Native(_)) => bump(&mut c, "needed reference native scripts", 1),
                            Some(ReferenceScript::Plutus) => bump(&mut c, "needed reference Plutus scripts", 1),
                            None => {}
                        }
                    }
                    let witness_timelock_needed = wi.native_scripts.iter().any(|w| {
                        needed.hashes.contains(&w.hash)
                            && w.script.nodes().iter().any(|n| {
                                matches!(n, NativeScriptNode::InvalidBefore(_) | NativeScriptNode::InvalidHereafter(_))
                            })
                    });
                    let reference_native_needed = needed
                        .hashes
                        .iter()
                        .any(|h| matches!(refs.get(h), Some(ReferenceScript::Native(_))));
                    match native_script_verdict(&body, &wi, Some(ctx)).expect("exact verdict") {
                        NativeScriptVerdict::Exact(ExactScriptVerdict::Valid) => {
                            bump(&mut c, "EXACT valid", 1);
                            for (kind, hit) in [
                                ("timelock", witness_timelock_needed),
                                ("refnative", reference_native_needed),
                                ("bootstrap", !wi.bootstrap_key_hashes.is_empty()),
                            ] {
                                if let (true, Some(dir), false) = (hit, &fixture_out, written.contains(kind)) {
                                    written.insert(kind);
                                    let mut lines = format!("slot {} tx {i}\n", b.slot.0);
                                    for t in body.inputs.iter().chain(body.reference_inputs.iter().flatten()) {
                                        let o = &utxo[t];
                                        lines += &format!(
                                            "{}#{} {} {}\n",
                                            hex(&t.tx_hash.0),
                                            t.index,
                                            hex(&o.address),
                                            o.script_ref.as_ref().map(|r| hex(r)).unwrap_or_else(|| "-".into())
                                        );
                                    }
                                    std::fs::write(format!("{dir}/lv13_{kind}_block.cbor"), &b.bytes).expect("write block");
                                    std::fs::write(format!("{dir}/lv13_{kind}_utxo.txt"), lines).expect("write utxo");
                                    println!("FIXTURE {kind}: slot {} tx {i}", b.slot.0);
                                }
                            }
                        }
                        NativeScriptVerdict::Exact(ExactScriptVerdict::Invalid(f)) => {
                            bump(&mut c, "EXACT not-validating", f.not_validating.len() as u64);
                            bump(&mut c, "EXACT extraneous", f.extraneous.len() as u64);
                            bump(&mut c, "EXACT missing", f.missing.len() as u64);
                            findings.push(format!("EXACT {where_}: {f:?} needed {:?}", needed.provenance));
                        }
                        NativeScriptVerdict::Reduced(_) => panic!("context given, so exact"),
                    }
                }
            }

            // Apply the tx to the forward UTxO.
            if invalid.contains(&(i as u64)) {
                bump(&mut c, "phase-2-invalid txs", 1);
                for t in body.collateral_inputs.iter().flatten() {
                    utxo.remove(t);
                }
                if let Some(r) = &body.collateral_return {
                    utxo.insert(
                        TxIn { tx_hash: tx_id.clone(), index: body.outputs.len() as u16 },
                        ResolvedScriptOutput { address: r.address.clone(), script_ref: r.script_ref.clone() },
                    );
                }
            } else {
                for t in &body.inputs {
                    utxo.remove(t);
                }
                for (ix, out) in body.outputs.iter().enumerate() {
                    utxo.insert(
                        TxIn { tx_hash: tx_id.clone(), index: ix as u16 },
                        ResolvedScriptOutput { address: out.address.clone(), script_ref: out.script_ref.clone() },
                    );
                }
            }
        }
    }

    println!("STORE TIP slot={} hash={}", tip.slot.0, hex(&tip.hash.0));
    println!("SLOTS {}..={}", first.unwrap_or(0), last);
    for (k, v) in &c {
        println!("{k}: {v}");
    }
    for (p, v) in &purposes {
        println!("needed by {p:?}: {v}");
    }
    println!("TIMELOCK VERDICT CHANGES (first 20):");
    for l in &changed {
        println!("  {l}");
    }
    println!("FINDINGS {} (first 50):", findings.len());
    for l in findings.iter().take(50) {
        println!("  {l}");
    }
    assert_eq!(last, tip.slot.0, "the walk must end at the store tip");
}
