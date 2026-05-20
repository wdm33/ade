// One-off tooling (B3-S5 real-oracle prep): enumerate the cert/withdrawal-bearing
// Conway txs in the committed epoch-576 corpus and the exact set of input TxIns
// that must be resolved against the epoch-576 UTxO set. Writes a minimal JSON
// manifest to stdout. Not a test; not part of the build's correctness surface.

use std::collections::BTreeSet;

use ade_codec::conway::tx::decode_conway_tx_body;
use ade_testkit::tx_validity::extract_corpus_txs;
use ade_testkit::validity::corpus::ConwayValidityCorpus;

fn main() {
    let corpus = ConwayValidityCorpus::load().expect("load conway_epoch576 corpus");
    let txs = extract_corpus_txs(&corpus.blocks).expect("extract corpus txs");

    let mut all_inputs: BTreeSet<(String, u16)> = BTreeSet::new();
    let mut cw_tx_count = 0usize;
    let mut total_tx = 0usize;
    let mut rows: Vec<String> = Vec::new();

    for t in &txs {
        total_tx += 1;
        // The tx_cbor is [body, witness_set, is_valid, aux]; body is element 0.
        // decode_conway_tx_body expects to start at the body map. Walk into the
        // outer array first.
        let mut off = 0usize;
        // outer array header
        let _ = ade_codec::cbor::read_array_header(&t.tx_cbor, &mut off)
            .expect("tx outer array");
        let body = match decode_conway_tx_body(&t.tx_cbor, &mut off) {
            Ok(b) => b,
            Err(e) => {
                eprintln!("skip tx b{} i{}: body decode {:?}", t.block_index, t.tx_index, e);
                continue;
            }
        };
        let has_certs = body.certs.is_some();
        let has_wdrl = body.withdrawals.is_some();
        if !(has_certs || has_wdrl) {
            continue;
        }
        cw_tx_count += 1;
        let mut tx_ins: Vec<(String, u16)> = Vec::new();
        // check_inputs_present resolves spend ∪ collateral ∪ reference inputs,
        // so all three kinds must be in the resolved UTxO fixture.
        let mut ins: Vec<&ade_types::tx::TxIn> = body.inputs.iter().collect();
        if let Some(c) = &body.collateral_inputs {
            ins.extend(c.iter());
        }
        if let Some(r) = &body.reference_inputs {
            ins.extend(r.iter());
        }
        for i in ins {
            let h = hex(&i.tx_hash.0);
            all_inputs.insert((h.clone(), i.index));
            tx_ins.push((h, i.index));
        }
        rows.push(format!(
            "  {{ \"block\": {}, \"tx\": {}, \"certs\": {}, \"withdrawals\": {}, \"is_valid\": {}, \"inputs\": {} }}",
            t.block_index, t.tx_index, has_certs, has_wdrl, t.is_valid, tx_ins.len()
        ));
    }

    println!("{{");
    println!("  \"total_txs\": {},", total_tx);
    println!("  \"cert_or_withdrawal_txs\": {},", cw_tx_count);
    println!("  \"unique_input_txins_to_resolve\": {},", all_inputs.len());
    println!("  \"txs\": [");
    println!("{}", rows.join(",\n"));
    println!("  ],");
    println!("  \"resolution_set\": [");
    let lines: Vec<String> = all_inputs
        .iter()
        .map(|(h, idx)| format!("    \"{}#{}\"", h, idx))
        .collect();
    println!("{}", lines.join(",\n"));
    println!("  ]");
    println!("}}");
}

fn hex(b: &[u8]) -> String {
    let mut s = String::with_capacity(b.len() * 2);
    for x in b {
        s.push_str(&format!("{:02x}", x));
    }
    s
}
