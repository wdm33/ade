//! OQ5-S1: the Shelley certificate decoder preserves the key/script
//! discriminant on a `StakeCredential`, mapping wire tag 0 → KeyHash and 1 →
//! ScriptHash.

use ade_codec::shelley::cert::decode_certificates;
use ade_types::shelley::cert::{Certificate, StakeCredential};
use ade_types::Hash28;

fn cbor_uint(buf: &mut Vec<u8>, major: u8, value: u64) {
    let m = major << 5;
    if value < 24 {
        buf.push(m | value as u8);
    } else if value < 0x100 {
        buf.push(m | 24);
        buf.push(value as u8);
    } else {
        buf.push(m | 25);
        buf.extend_from_slice(&(value as u16).to_be_bytes());
    }
}

const MARK: u8 = 0x11;

/// A one-element certificate set holding a StakeRegistration whose credential
/// carries the given type tag.
fn reg_cert_set(tag: u64) -> Vec<u8> {
    let mut cred = Vec::new();
    cbor_uint(&mut cred, 4, 2); // array(2) [type, hash28]
    cbor_uint(&mut cred, 0, tag);
    cbor_uint(&mut cred, 2, 28);
    cred.extend_from_slice(&[MARK; 28]);

    let mut cert = Vec::new();
    cbor_uint(&mut cert, 4, 2); // array(2) [0, credential]
    cbor_uint(&mut cert, 0, 0); // StakeRegistration tag
    cert.extend_from_slice(&cred);

    let mut set = Vec::new();
    cbor_uint(&mut set, 4, 1); // array(1)
    set.extend_from_slice(&cert);
    set
}

#[test]
fn shelley_credential_preserves_discriminant() {
    // tag 0 → KeyHash
    let certs = decode_certificates(&reg_cert_set(0)).expect("decode key-hash cred");
    assert_eq!(certs.len(), 1);
    match &certs[0] {
        Certificate::StakeRegistration(c) => {
            assert_eq!(*c, StakeCredential::KeyHash(Hash28([MARK; 28])));
        }
        other => panic!("expected StakeRegistration, got {other:?}"),
    }

    // tag 1 → ScriptHash, distinct from KeyHash over identical 28 bytes
    let certs = decode_certificates(&reg_cert_set(1)).expect("decode script-hash cred");
    match &certs[0] {
        Certificate::StakeRegistration(c) => {
            assert_eq!(*c, StakeCredential::ScriptHash(Hash28([MARK; 28])));
            assert_ne!(*c, StakeCredential::KeyHash(Hash28([MARK; 28])));
        }
        other => panic!("expected StakeRegistration, got {other:?}"),
    }
}
