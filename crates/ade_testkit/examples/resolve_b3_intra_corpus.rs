// B3-S5 prep: resolve the corpus input TxIns that are created by OTHER corpus
// txs (intra-corpus deps) — these are absent from the pre-window ledger UTxO
// because the truncation point precedes all corpus blocks. For each such input,
// the creating tx is in the corpus; tx_id = blake2b_256(body_slice), and
// output[index] gives (address, coin). Emits JSON entries for /tmp/b3_missing.txt.

use std::collections::BTreeMap;
use std::io::Read;

use ade_codec::conway::tx::decode_conway_tx_body;
use ade_crypto::blake2b::blake2b_256;
use ade_testkit::tx_validity::extract_corpus_txs;
use ade_testkit::validity::corpus::ConwayValidityCorpus;

fn hex(b: &[u8]) -> String {
    let mut s = String::with_capacity(b.len() * 2);
    for x in b {
        s.push_str(&format!("{:02x}", x));
    }
    s
}

fn main() {
    // read missing txins (txid#index per line) from stdin
    let mut buf = String::new();
    std::io::stdin().read_to_string(&mut buf).unwrap();
    let missing: Vec<(String, u16)> = buf
        .lines()
        .filter(|l| !l.trim().is_empty())
        .map(|l| {
            let (h, i) = l.trim().split_once('#').unwrap();
            (h.to_string(), i.parse().unwrap())
        })
        .collect();

    let corpus = ConwayValidityCorpus::load().expect("corpus");
    let txs = extract_corpus_txs(&corpus.blocks).expect("extract");

    // Map (txid_hex, index) -> (address_hex, coin)
    let mut outs: BTreeMap<(String, u16), (String, u64)> = BTreeMap::new();
    for t in &txs {
        let mut off = 0usize;
        let _ = ade_codec::cbor::read_array_header(&t.tx_cbor, &mut off).unwrap();
        let body_start = off;
        // body is element 0; skip it to find its byte span
        let (_, body_end) = ade_codec::cbor::skip_item(&t.tx_cbor, &mut off).unwrap();
        let body_slice = &t.tx_cbor[body_start..body_end];
        let txid = hex(&blake2b_256(body_slice).0);
        let mut boff = 0usize;
        let body = match decode_conway_tx_body(body_slice, &mut boff) {
            Ok(b) => b,
            Err(_) => continue,
        };
        for (idx, o) in body.outputs.iter().enumerate() {
            outs.insert((txid.clone(), idx as u16), (hex(&o.address), o.coin.0));
        }
    }

    let mut resolved = 0usize;
    let mut entries: Vec<String> = Vec::new();
    let mut unresolved: Vec<String> = Vec::new();
    for (h, i) in &missing {
        match outs.get(&(h.clone(), *i)) {
            Some((addr, coin)) => {
                resolved += 1;
                entries.push(format!(
                    "  {{ \"tx_hash\": \"{}\", \"index\": {}, \"coin\": {}, \"address\": \"{}\" }}",
                    h, i, coin, addr
                ));
            }
            None => unresolved.push(format!("{}#{}", h, i)),
        }
    }

    eprintln!("intra-corpus resolved: {} of {}", resolved, missing.len());
    if !unresolved.is_empty() {
        eprintln!("UNRESOLVED ({}):", unresolved.len());
        for u in &unresolved {
            eprintln!("  {}", u);
        }
    }
    // entries to stdout (JSON array fragment)
    println!("{}", entries.join(",\n"));
}
