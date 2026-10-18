//! TREASURY-DONATION CENSUS — read-only, evidence-only, `#[ignore]`d by default.
//!
//! Scans cardano ImmutableDB chunk files (each a concatenation of era-tagged block envelopes) and
//! reports every Conway transaction whose body carries a treasury donation (key 22), with Ade's own
//! envelope, block and transaction-body decoders. Opened by SLICE LV-5, F2 (chain side).
//!
//! ```text
//! ADE_DONATION_CHUNKS="/path/db/immutable/06024.chunk /path/db/immutable/06025.chunk ..." \
//! ADE_DONATION_SLOTS=130118400-130550399 \
//!   cargo test -p ade_runtime --test treasury_donation_census -- --ignored --nocapture
//! ```

use ade_codec::cbor::{self, envelope::decode_block_envelope, ContainerEncoding};
use ade_codec::conway::{decode_conway_block, tx::decode_conway_tx_body};
use ade_types::CardanoEra;

fn invalid_indices(raw: &Option<Vec<u8>>) -> Vec<u64> {
    let Some(raw) = raw else { return Vec::new() };
    let mut off = 0;
    let mut out = Vec::new();
    match cbor::read_array_header(raw, &mut off).expect("invalid_txs array") {
        ContainerEncoding::Definite(n, _) => {
            for _ in 0..n {
                out.push(cbor::read_uint(raw, &mut off).expect("index").0);
            }
        }
        ContainerEncoding::Indefinite => {
            while !cbor::is_break(raw, off).expect("break") {
                out.push(cbor::read_uint(raw, &mut off).expect("index").0);
            }
        }
    }
    out
}

#[test]
#[ignore = "evidence-only; requires ADE_DONATION_CHUNKS pointing at ImmutableDB chunk files"]
fn scan_chunks_for_treasury_donations() {
    let chunks = std::env::var("ADE_DONATION_CHUNKS").expect("set ADE_DONATION_CHUNKS");
    let (lo, hi) = match std::env::var("ADE_DONATION_SLOTS") {
        Ok(r) => {
            let (a, b) = r.split_once('-').expect("ADE_DONATION_SLOTS is lo-hi");
            (a.parse::<u64>().unwrap(), b.parse::<u64>().unwrap())
        }
        Err(_) => (0, u64::MAX),
    };

    let (mut blocks, mut txs, mut donations, mut total) = (0u64, 0u64, 0u64, 0u128);
    let (mut first_slot, mut last_slot) = (u64::MAX, 0u64);
    for path in chunks.split_whitespace() {
        let data = std::fs::read(path).expect("read chunk");
        let mut off = 0;
        while off < data.len() {
            let start = off;
            cbor::skip_item(&data, &mut off).expect("block item");
            let env_bytes = &data[start..off];
            let env = decode_block_envelope(env_bytes).expect("envelope");
            if env.era != CardanoEra::Conway {
                continue;
            }
            let block = decode_conway_block(&env_bytes[env.block_start..env.block_end])
                .expect("conway block");
            let b = block.decoded();
            let slot = b.header.body.slot;
            if slot < lo || slot > hi {
                continue;
            }
            blocks += 1;
            first_slot = first_slot.min(slot);
            last_slot = last_slot.max(slot);
            let invalid = invalid_indices(&b.invalid_txs);

            let bodies = &b.tx_bodies;
            let mut boff = 0;
            let n = match cbor::read_array_header(bodies, &mut boff).expect("tx_bodies array") {
                ContainerEncoding::Definite(n, _) => n,
                ContainerEncoding::Indefinite => u64::MAX,
            };
            let mut i = 0u64;
            while i < n && !(n == u64::MAX && cbor::is_break(bodies, boff).expect("break")) {
                let body_start = boff;
                let body = decode_conway_tx_body(bodies, &mut boff).expect("tx body");
                txs += 1;
                if let Some(d) = body.donation {
                    donations += 1;
                    total += d.0 as u128;
                    let txid = ade_crypto::blake2b::blake2b_256(&bodies[body_start..boff]);
                    let txid_hex: String = txid.0.iter().map(|x| format!("{x:02x}")).collect();
                    println!(
                        "DONATION slot={slot} block_no={} tx_index={i} txid={txid_hex} amount={} phase2_valid={}",
                        b.header.body.block_number,
                        d.0,
                        !invalid.contains(&i)
                    );
                }
                i += 1;
            }
        }
    }
    println!(
        "SUMMARY blocks={blocks} txs={txs} slots={first_slot}..{last_slot} donations={donations} total={total}"
    );
}
