use ade_core::consensus::errors::{HFCError, HeaderValidationError};
use ade_core::consensus::events::Point;
use ade_ledger::block_validity::{
    decode_verdict_surface, encode_verdict_surface, BlockRejectClass, BlockValidityError,
    BlockValidityVerdict, FieldError, FieldKind, MissingInput, SurfaceDecodeError, VerdictSurface,
};
use ade_ledger::error::{ConservationError, LedgerError};
use ade_ledger::rules::BlockVerdict;
use ade_types::{BlockNo, Coin, Hash32, SlotNo};

fn zero_body() -> BlockVerdict {
    BlockVerdict {
        tx_count: 0,
        plutus_deferred_count: 0,
        non_plutus_count: 0,
        native_script_passed: 0,
        native_script_failed: 0,
        state_backed_phase1_rejected: 0,
        plutus_eval_passed: 0,
        plutus_eval_failed: 0,
        plutus_eval_ineligible: 0,
    }
}

fn fixed_valid() -> BlockValidityVerdict {
    BlockValidityVerdict::Valid {
        tip: Point {
            slot: SlotNo(0x0102_0304_0506_0708),
            hash: Hash32([0xAB; 32]),
        },
        block_no: BlockNo(0x1122_3344_5566_7788),
        body: zero_body(),
    }
}

fn invalid_with_class(class: BlockRejectClass, error: BlockValidityError) -> BlockValidityVerdict {
    BlockValidityVerdict::Invalid { class, error }
}

#[test]
fn class_mapping_is_total() {
    let cases: [(BlockValidityError, BlockRejectClass); 5] = [
        (
            BlockValidityError::Header(HeaderValidationError::HFC(HFCError::EmptyEraList)),
            BlockRejectClass::HeaderInvalid,
        ),
        (
            BlockValidityError::Body(LedgerError::Conservation(ConservationError {
                consumed_coin: Coin(10),
                produced_coin: Coin(9),
            })),
            BlockRejectClass::BodyInvalid,
        ),
        (
            BlockValidityError::BodyHashMismatch {
                header: Hash32([1u8; 32]),
                actual: Hash32([2u8; 32]),
            },
            BlockRejectClass::BodyHashMismatch,
        ),
        (
            BlockValidityError::MalformedField(FieldError {
                field: FieldKind::Ed25519Signature,
                expected: 64,
                actual: 63,
            }),
            BlockRejectClass::MalformedField,
        ),
        (
            BlockValidityError::MissingConsensusInput(MissingInput::EpochNonce),
            BlockRejectClass::MissingConsensusInput,
        ),
    ];
    for (err, expected) in cases.iter() {
        assert_eq!(err.class(), *expected);
    }
}

#[test]
fn verdict_surface_roundtrip_valid() {
    let v = fixed_valid();
    let bytes = encode_verdict_surface(&v);
    let surface = decode_verdict_surface(&bytes).expect("decode valid surface");
    assert_eq!(
        surface,
        VerdictSurface::Valid {
            tip: Point {
                slot: SlotNo(0x0102_0304_0506_0708),
                hash: Hash32([0xAB; 32]),
            },
            block_no: BlockNo(0x1122_3344_5566_7788),
        }
    );
}

#[test]
fn verdict_surface_roundtrip_invalid_all_classes() {
    let classes = [
        BlockRejectClass::HeaderInvalid,
        BlockRejectClass::BodyInvalid,
        BlockRejectClass::BodyHashMismatch,
        BlockRejectClass::MalformedField,
        BlockRejectClass::MissingConsensusInput,
    ];
    for class in classes.iter() {
        let v = invalid_with_class(
            *class,
            BlockValidityError::MissingConsensusInput(MissingInput::ActiveSlotsCoeff),
        );
        let bytes = encode_verdict_surface(&v);
        let surface = decode_verdict_surface(&bytes).expect("decode invalid surface");
        assert_eq!(surface, VerdictSurface::Invalid { class: *class });
    }
}

#[test]
fn surface_layout_is_stable() {
    let invalid = invalid_with_class(
        BlockRejectClass::MalformedField,
        BlockValidityError::MalformedField(FieldError {
            field: FieldKind::VkeyWitness,
            expected: 32,
            actual: 31,
        }),
    );
    assert_eq!(hex(&encode_verdict_surface(&invalid)), "820103");

    let valid = fixed_valid();
    assert_eq!(
        hex(&encode_verdict_surface(&valid)),
        "8300821b01020304050607085820abababababababababababababababababababababababababababababababab1b1122334455667788"
    );
}

#[test]
fn surface_decode_rejects_unknown_discriminant() {
    // Outer 2-array with an unknown outer discriminant (9).
    let bytes = [0x82, 0x09, 0x00];
    let err = decode_verdict_surface(&bytes).expect_err("unknown outer discriminant");
    assert_eq!(
        err,
        SurfaceDecodeError::UnknownDiscriminant {
            for_enum: "VerdictSurface",
            found: 9,
        }
    );

    // Valid Invalid frame but an unknown reject-class discriminant (7).
    let bytes = [0x82, 0x01, 0x07];
    let err = decode_verdict_surface(&bytes).expect_err("unknown class discriminant");
    assert_eq!(
        err,
        SurfaceDecodeError::UnknownDiscriminant {
            for_enum: "BlockRejectClass",
            found: 7,
        }
    );
}

#[test]
fn surface_decode_rejects_short_array() {
    // Invalid frame declared as a 1-array (discriminant only, no class).
    let bytes = [0x81, 0x01];
    let err = decode_verdict_surface(&bytes).expect_err("short invalid array");
    assert_eq!(err, SurfaceDecodeError::FieldCount { expected: 2, actual: 1 });
}

fn hex(b: &[u8]) -> String {
    b.iter().map(|x| format!("{:02x}", x)).collect()
}
