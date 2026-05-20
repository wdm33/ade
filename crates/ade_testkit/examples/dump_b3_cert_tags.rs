// One-off tooling (B3-S5 prep): tally the Conway certificate tags used by the
// cert/withdrawal-bearing txs in the committed epoch-576 corpus, so we know
// whether real-oracle resolution needs registration/pool state (tags 1/3) or
// only the UTxO set. Not a test.

use std::collections::BTreeMap;

use ade_codec::conway::cert::decode_conway_certs;
use ade_codec::conway::tx::decode_conway_tx_body;
use ade_testkit::tx_validity::extract_corpus_txs;
use ade_testkit::validity::corpus::ConwayValidityCorpus;

fn main() {
    let corpus = ConwayValidityCorpus::load().expect("load corpus");
    let txs = extract_corpus_txs(&corpus.blocks).expect("extract");
    let mut tag_counts: BTreeMap<String, usize> = BTreeMap::new();
    let mut wdrl_txs = 0usize;
    let mut cert_txs = 0usize;

    for t in &txs {
        let mut off = 0usize;
        let _ = ade_codec::cbor::read_array_header(&t.tx_cbor, &mut off).unwrap();
        let body = match decode_conway_tx_body(&t.tx_cbor, &mut off) {
            Ok(b) => b,
            Err(_) => continue,
        };
        if body.withdrawals.is_some() {
            wdrl_txs += 1;
        }
        if let Some(cert_bytes) = &body.certs {
            cert_txs += 1;
            match decode_conway_certs(cert_bytes) {
                Ok(certs) => {
                    for c in &certs {
                        let name = format!("{:?}", c);
                        // bucket by the variant name prefix (up to first space/paren)
                        let key = name
                            .split(|ch: char| ch == ' ' || ch == '(' || ch == '{')
                            .next()
                            .unwrap_or("?")
                            .to_string();
                        *tag_counts.entry(key).or_insert(0) += 1;
                    }
                }
                Err(e) => {
                    *tag_counts.entry(format!("DECODE_ERR:{:?}", e)).or_insert(0) += 1;
                }
            }
        }
    }

    println!("cert-bearing txs: {}", cert_txs);
    println!("withdrawal-bearing txs: {}", wdrl_txs);
    println!("cert variant tally:");
    for (k, v) in &tag_counts {
        println!("  {:>4}  {}", v, k);
    }
}
