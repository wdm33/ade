//! cardano-cli TxIn oracle cross-check for the native MemPack TxOut decoder
//! (MITHRIL-VERIFIED-ANCHOR-IMPORT, Stage 2). Decodes a focused sample from a real preview `tables`
//! snapshot (tag-2/3 base addresses for PO#1, plus multi-asset / datum), writes (txin, address-hex,
//! coin, assets) to a TSV; an external script queries `cardano-cli query utxo` for those same TxIns
//! and compares — the independent full-TxOut oracle. RED: reads the snapshot. Skips when absent.

use ade_codec::cbor::{read_array_header, read_bytes, read_map_header, ContainerEncoding};
use ade_ledger::ledgerdb_tables::{read_txout, DatumField};

#[test]
fn write_oracle_sample_from_preview_tables() {
    let path =
        std::env::var("ADE_TABLES").unwrap_or_else(|_| "/tmp/ade-preview-tables.cbor".to_string());
    let data = match std::fs::read(&path) {
        Ok(d) => d,
        Err(_) => {
            eprintln!("SKIP: no tables corpus at {path}");
            return;
        }
    };
    let mut off = 0usize;
    let _ = read_array_header(&data, &mut off).unwrap();
    let _ = read_map_header(&data, &mut off).unwrap();
    assert!(matches!(
        read_array_header(&data, &mut 0usize).unwrap(),
        ContainerEncoding::Definite(1, _)
    ));

    // collect a focused sample: up to 8 of each interesting form.
    let mut lines = String::from("txin\ttag\taddress_hex\tcoin\tassets\tdatum\tscript\n");
    let mut want = [8u32, 4, 8, 4, 4, 4]; // per-tag quota (0..=5)
    let mut taken = 0;
    while off < data.len() && taken < 32 {
        if data[off] == 0xff {
            break;
        }
        let (txin, _) = read_bytes(&data, &mut off).unwrap();
        let (val, _) = read_bytes(&data, &mut off).unwrap();
        let tag = val[0] as usize;
        let o = match read_txout(&val) {
            Ok(o) => o,
            Err(e) => panic!("decode error tag {tag}: {e:?}"),
        };
        if tag < 6 && want[tag] > 0 {
            want[tag] -= 1;
            taken += 1;
            let txid = hex(&txin[0..32]);
            let ix = u16::from_be_bytes([txin[32], txin[33]]);
            let assets: String = o
                .value
                .assets
                .iter()
                .flat_map(|(p, m)| {
                    m.iter()
                        .map(move |(n, q)| format!("{}.{}={}", hex(&p.0), hex(&n.0), q))
                })
                .collect::<Vec<_>>()
                .join(",");
            let datum = match &o.datum {
                DatumField::None => "none".to_string(),
                DatumField::Hash(h) => format!("hash:{}", hex(h)),
                DatumField::Inline(b) => format!("inline:{}b", b.len()),
            };
            let script = match &o.script {
                None => "none".to_string(),
                Some(s) => format!("{s:?}").chars().take(20).collect(),
            };
            lines.push_str(&format!(
                "{txid}#{ix}\t{tag}\t{}\t{}\t{assets}\t{datum}\t{script}\n",
                hex(&o.address),
                o.value.coin.0
            ));
        }
    }
    let out = "/tmp/ade-oracle-sample.tsv";
    std::fs::write(out, &lines).unwrap();
    eprintln!("wrote {taken} sample TxOuts to {out}\n{lines}");
    assert!(taken >= 6, "expected a multi-form sample");
}

fn hex(b: &[u8]) -> String {
    b.iter().map(|x| format!("{x:02x}")).collect()
}
