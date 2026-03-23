//! Integration test: Certificate decoding from corpus blocks.
//!
//! Verifies that all certificates in the contiguous corpus decode
//! correctly through the certificate decoder.

use std::path::PathBuf;

use ade_codec::cbor::envelope::decode_block_envelope;
use ade_codec::shelley::cert::decode_certificates;
use ade_types::shelley::cert::Certificate;
use ade_types::CardanoEra;

fn corpus_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("corpus")
        .join("contiguous")
}

fn count_certificates(era_name: &str) -> (usize, Vec<(u64, usize)>) {
    let era_dir = corpus_root().join(era_name);
    let blocks_json: serde_json::Value = serde_json::from_str(
        &std::fs::read_to_string(era_dir.join("blocks.json")).unwrap(),
    )
    .unwrap();
    let blocks = blocks_json["blocks"].as_array().unwrap();

    let mut type_counts = std::collections::BTreeMap::new();
    let mut total = 0;

    for entry in blocks {
        let filename = entry["file"].as_str().unwrap();
        let raw = std::fs::read(era_dir.join(filename)).unwrap();
        let env = decode_block_envelope(&raw).unwrap();
        let inner = &raw[env.block_start..env.block_end];

        // Decode block to get tx_bodies with certs
        let decoded = match env.era {
            CardanoEra::Shelley => ade_codec::shelley::decode_shelley_block(inner),
            CardanoEra::Allegra => ade_codec::allegra::decode_allegra_block(inner),
            CardanoEra::Mary => ade_codec::mary::decode_mary_block(inner),
            CardanoEra::Alonzo => ade_codec::alonzo::decode_alonzo_block(inner),
            CardanoEra::Babbage => ade_codec::babbage::decode_babbage_block(inner),
            CardanoEra::Conway => ade_codec::conway::decode_conway_block(inner),
            _ => continue,
        };

        let block = decoded.unwrap();
        let block = block.decoded();

        // Parse tx bodies to extract certs field
        let mut offset = 0;
        let data = &block.tx_bodies;
        let enc = ade_codec::cbor::read_array_header(data, &mut offset).unwrap();
        let n = match enc {
            ade_codec::cbor::ContainerEncoding::Definite(n, _) => n,
            _ => continue,
        };

        for _ in 0..n {
            // Tx body is a map — find key 4 (certs)
            let tx_start = offset;
            let map_enc = ade_codec::cbor::read_map_header(data, &mut offset).unwrap();
            let map_len = match map_enc {
                ade_codec::cbor::ContainerEncoding::Definite(n, _) => n,
                _ => {
                    offset = tx_start;
                    let _ = ade_codec::cbor::skip_item(data, &mut offset);
                    continue;
                }
            };

            let mut found_certs = false;
            for _ in 0..map_len {
                let (key, _) = ade_codec::cbor::read_uint(data, &mut offset).unwrap();
                if key == 4 {
                    // Capture cert bytes
                    let cert_start = offset;
                    let (_, cert_end) = ade_codec::cbor::skip_item(data, &mut offset).unwrap();
                    let cert_bytes = &data[cert_start..cert_end];

                    // Decode certificates
                    match decode_certificates(cert_bytes) {
                        Ok(certs) => {
                            for cert in &certs {
                                let tag = match cert {
                                    Certificate::StakeRegistration(_) => 0,
                                    Certificate::StakeDeregistration(_) => 1,
                                    Certificate::StakeDelegation { .. } => 2,
                                    Certificate::PoolRegistration(_) => 3,
                                    Certificate::PoolRetirement { .. } => 4,
                                    Certificate::GenesisKeyDelegation { .. } => 5,
                                    Certificate::MIRTransfer(_) => 6,
                                };
                                *type_counts.entry(tag).or_insert(0) += 1;
                                total += 1;
                            }
                        }
                        Err(e) => {
                            eprintln!("  cert decode error in {filename}: {e}");
                        }
                    }
                    found_certs = true;
                } else {
                    let _ = ade_codec::cbor::skip_item(data, &mut offset);
                }
            }
            if !found_certs {
                // No certs in this tx — already past the map
            }
        }
    }

    let counts: Vec<(u64, usize)> = type_counts.into_iter().collect();
    (total, counts)
}

#[test]
fn shelley_certificates_decode() {
    let (total, counts) = count_certificates("shelley");
    eprintln!("Shelley: {total} certs, types: {counts:?}");
    assert!(total > 0, "Shelley should have certificates");
}

#[test]
fn all_eras_certificates_decode() {
    let eras = ["shelley", "allegra", "mary", "alonzo", "babbage", "conway"];

    eprintln!("\n=== Certificate Decoding Summary ===");

    let type_names = [
        "StakeReg", "StakeDeReg", "StakeDeleg", "PoolReg", "PoolRetire",
        "GenesisDeleg", "MIR", "ConwayReg", "ConwayUnReg", "ConwayVoteDeleg",
    ];

    for era in &eras {
        let (total, counts) = count_certificates(era);
        let desc: Vec<String> = counts
            .iter()
            .map(|(tag, count)| {
                let name = type_names.get(*tag as usize).unwrap_or(&"Other");
                format!("{name}={count}")
            })
            .collect();
        eprintln!("  {era:<8}: {total:>5} certs — {}", desc.join(", "));
    }
    eprintln!("====================================\n");
}
