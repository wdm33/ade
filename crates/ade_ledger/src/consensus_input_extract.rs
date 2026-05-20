// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types
// - Explicit state transitions only
// - Canonical serialization for all persisted/hashed data

//! RED consensus-input extractor: a tail-scan of a snapshot `state` CBOR for the
//! five `PraosState` nonces.
//!
//! Classified RED because it parses an external dump format (a UTxO-HD
//! `utxohd-mem` `ExtLedgerState` CBOR) rather than an authoritative canonical
//! type. The scan itself is pure over the input bytes. Fail-closed: the
//! `PraosState` record always carries exactly five non-neutral nonces in the
//! captured snapshots, so anything other than five is a hard error rather than a
//! best-effort pick.

/// A 32-byte Praos nonce as it appears on the wire: the body of a non-neutral
/// `Nonce = [1, bytes .size 32]` CBOR value (`82 01 5820 <32B>`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Nonce(pub [u8; 32]);

/// The five `PraosState` nonces in record order. The third (`epoch`) is eta0.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PraosNonces {
    pub evolving: Nonce,
    pub candidate: Nonce,
    pub epoch: Nonce,
    pub lab: Nonce,
    pub last_epoch_block: Nonce,
}

/// Fail-closed scan error.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NonceScanError {
    /// The scan found a number of nonce wrappers other than the required five.
    NotFiveNonces { found: usize },
}

/// The byte prefix of a non-neutral `Nonce`: CBOR `array(2)`, `uint 1`,
/// `bytes .size 32` (`0x5820`).
const NONCE_PREFIX: [u8; 4] = [0x82, 0x01, 0x58, 0x20];

/// Scan an `ExtLedgerState` `state` CBOR for the five contiguous `PraosState`
/// nonce wrappers (`82 01 5820 <32B>`). Requires EXACTLY five; otherwise
/// fail-fast. The nonces are returned in record order
/// `[evolving, candidate, epoch, lab, lastEpochBlock]`.
pub fn extract_praos_nonces(state_cbor: &[u8]) -> Result<PraosNonces, NonceScanError> {
    let mut found: Vec<Nonce> = Vec::new();
    let mut i = 0usize;
    // A nonce wrapper occupies 4 (prefix) + 32 (body) = 36 bytes.
    while i + 36 <= state_cbor.len() {
        if state_cbor[i..i + 4] == NONCE_PREFIX {
            let mut body = [0u8; 32];
            body.copy_from_slice(&state_cbor[i + 4..i + 36]);
            found.push(Nonce(body));
            i += 36;
        } else {
            i += 1;
        }
    }

    if found.len() != 5 {
        return Err(NonceScanError::NotFiveNonces { found: found.len() });
    }

    Ok(PraosNonces {
        evolving: found[0],
        candidate: found[1],
        epoch: found[2],
        lab: found[3],
        last_epoch_block: found[4],
    })
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
#[allow(clippy::expect_used)]
#[allow(clippy::panic)]
mod tests {
    use super::*;

    fn wrap(body: [u8; 32]) -> Vec<u8> {
        let mut v = NONCE_PREFIX.to_vec();
        v.extend_from_slice(&body);
        v
    }

    fn fixture_five() -> (Vec<u8>, [[u8; 32]; 5]) {
        let bodies = [[1u8; 32], [2u8; 32], [3u8; 32], [4u8; 32], [5u8; 32]];
        // Leading non-matching filler, then five contiguous nonce wrappers.
        let mut buf: Vec<u8> = vec![0xa5, 0x00, 0x00, 0x00];
        for b in &bodies {
            buf.extend_from_slice(&wrap(*b));
        }
        (buf, bodies)
    }

    #[test]
    fn extract_nonces_field_order() {
        let (buf, bodies) = fixture_five();
        let n = extract_praos_nonces(&buf).expect("five nonces");
        assert_eq!(n.evolving.0, bodies[0]);
        assert_eq!(n.candidate.0, bodies[1]);
        assert_eq!(n.epoch.0, bodies[2]);
        assert_eq!(n.lab.0, bodies[3]);
        assert_eq!(n.last_epoch_block.0, bodies[4]);
    }

    #[test]
    fn extract_nonces_post_boundary_reseed_corroboration() {
        // For the epoch-576 snapshot, evolving == candidate (post-boundary
        // reseed). A fixture with that property must preserve the equality.
        let same = [7u8; 32];
        let mut buf: Vec<u8> = Vec::new();
        for b in [same, same, [3u8; 32], [4u8; 32], [5u8; 32]] {
            buf.extend_from_slice(&wrap(b));
        }
        let n = extract_praos_nonces(&buf).expect("five nonces");
        assert_eq!(n.evolving, n.candidate);
    }

    #[test]
    fn extract_nonces_requires_exactly_five() {
        // Four wrappers -> error.
        let mut four: Vec<u8> = Vec::new();
        for b in [[1u8; 32], [2u8; 32], [3u8; 32], [4u8; 32]] {
            four.extend_from_slice(&wrap(b));
        }
        assert_eq!(
            extract_praos_nonces(&four),
            Err(NonceScanError::NotFiveNonces { found: 4 })
        );

        // Six wrappers -> error.
        let mut six: Vec<u8> = Vec::new();
        for b in [
            [1u8; 32], [2u8; 32], [3u8; 32], [4u8; 32], [5u8; 32], [6u8; 32],
        ] {
            six.extend_from_slice(&wrap(b));
        }
        assert_eq!(
            extract_praos_nonces(&six),
            Err(NonceScanError::NotFiveNonces { found: 6 })
        );

        // Empty -> error.
        assert_eq!(
            extract_praos_nonces(&[]),
            Err(NonceScanError::NotFiveNonces { found: 0 })
        );
    }

    #[test]
    fn extract_nonces_is_deterministic() {
        let (buf, _) = fixture_five();
        let a = extract_praos_nonces(&buf);
        let b = extract_praos_nonces(&buf);
        assert_eq!(a, b);
    }
}
