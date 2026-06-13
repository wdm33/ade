// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types
// - Explicit state transitions only
// - Canonical serialization for all persisted/hashed data

//! End-to-end Plutus evaluation test using aiken's own unit-test fixture.
//!
//! Proves that `ade_plutus::eval_tx_phase_two` runs aiken's phase-2
//! evaluator against a real self-contained Plutus tx and returns
//! successful per-script results with non-zero ex_units.
//!
//! The fixture is `test_eval_0` from
//! aiken/crates/uplc/src/tx/tests.rs at commit 42babe5 (v1.1.21 —
//! the same commit pinned in `ade_plutus/Cargo.toml`).
//!
//! The tx is a Plutus V2 minting transaction whose validator is:
//! ```ignore
//! mintTestValidator :: () -> ScriptContext -> Bool
//! mintTestValidator _ ctx =
//!   txInfoFee txInfo == txInfoFee txInfo
//!   && (case txInfoSignatories txInfo of [] -> True)
//! ```
//! — a tautology. Aiken's oracle reports mem=747528 cpu=217294271.
//!
//! This test closes the gap after commit 439ecd8 where
//! `plutus_evaluator_reachable_on_corpus` showed 72 Plutus txs
//! reaching the dispatch but all landing on `Ineligible`. With
//! this fixture `Eval-ok = 1` and the full pipeline is proven.

use ade_plutus::{eval_tx_phase_two, tx_eval::SlotConfig};

/// Full tx CBOR. Copied verbatim from aiken's `test_eval_0`.
const TX_HEX: &str = "84a80081825820975c17a4fed0051be622328efa548e206657d2b65a19224bf6ff8132571e6a5002018282581d60b6c8794e9a7a26599440a4d0fd79cd07644d15917ff13694f1f67235821a000f41f0a1581cc4f241450001af08f3ddbaf9335db79883cbcd81071b8e3508de3055a1400a82581d60b6c8794e9a7a26599440a4d0fd79cd07644d15917ff13694f1f672351a0084192f021a00053b6109a1581cc4f241450001af08f3ddbaf9335db79883cbcd81071b8e3508de3055a1400a0b5820b4f96b0acec8beff2adededa8ba317bcac92174f0f65ccefe569b9a6aac7375a0d818258206c732139de33e916342707de2aebef2252c781640326ff37b86ec99d97f1ba8d011082581d60b6c8794e9a7a26599440a4d0fd79cd07644d15917ff13694f1f672351b00000001af0cdfa2111a0007d912a3008182582031ae74f8058527afb305d7495b10a99422d9337fc199e1f28044f2c477a0f9465840b8b97b7c3b4e19ecfc2fcd9884ee53a35887ee6e4d36901b9ecbac3fe032d7e8a4358305afa573a86396e378255651ed03501906e9def450e588d4bb36f42a050581840100d87980821a000b68081a0cf3a5bf06815909b25909af010000323322323232323232323232323232323232323232332232323232323232323233223232223232533533223233025323233355300f1200135028502623500122333553012120013502b50292350012233350012330314800000488cc0c80080048cc0c400520000013355300e1200123500122335501c0023335001233553012120012350012233550200023550140010012233355500f0150020012335530121200123500122335502000235501300100133355500a01000200130105002300f5001533532350012222222222220045001102a2216135001220023333573466e1cd55ce9baa0044800080808c98c8080cd5ce01081000f1999ab9a3370e6aae7540092000233221233001003002323232323232323232323232323333573466e1cd55cea8062400046666666666664444444444442466666666666600201a01801601401201000e00c00a00800600466a03803a6ae854030cd4070074d5d0a80599a80e00f1aba1500a3335502075ca03e6ae854024ccd54081d7280f9aba1500833501c02835742a00e666aa040052eb4d5d0a8031919191999ab9a3370e6aae75400920002332212330010030023232323333573466e1cd55cea8012400046644246600200600466a066eb4d5d0a801181a1aba135744a004464c6406c66ae700dc0d80d04d55cf280089baa00135742a0046464646666ae68cdc39aab9d5002480008cc8848cc00400c008cd40cdd69aba150023034357426ae8940088c98c80d8cd5ce01b81b01a09aab9e5001137540026ae84d5d1280111931901919ab9c033032030135573ca00226ea8004d5d0a80299a80e3ae35742a008666aa04004a40026ae85400cccd54081d710009aba150023027357426ae8940088c98c80b8cd5ce01781701609aba25001135744a00226ae8940044d5d1280089aba25001135744a00226ae8940044d5d1280089aba25001135744a00226aae7940044dd50009aba150023017357426ae8940088c98c8080cd5ce01081000f080f89931900f99ab9c4901035054350001f135573ca00226ea8004444888ccd54c010480054040cd54c01c480048d400488cd54054008d54024004ccd54c0104800488d4008894cd4ccd54c03048004c8cd409c88ccd400c88008008004d40048800448cc004894cd400840b040040a48d400488cc028008014018400c4cd405001000d4044004cd54c01c480048d400488c8cd5405800cc004014c8004d540a4894cd40044d5402800c884d4008894cd4cc03000802044888cc0080280104c01800c008c8004d5408888448894cd40044008884cc014008ccd54c01c480040140100044484888c00c0104484888c004010c8004d5407c8844894cd400454038884cd403cc010008cd54c01848004010004c8004d5407888448894cd40044d400c88004884ccd401488008c010008ccd54c01c4800401401000488ccd5cd19b8f00200101e01d2350012222222222220091232230023758002640026aa038446666aae7c004940288cd4024c010d5d080118019aba2002015232323333573466e1cd55cea80124000466442466002006004601a6ae854008c014d5d09aba2500223263201533573802c02a02626aae7940044dd50009191919191999ab9a3370e6aae75401120002333322221233330010050040030023232323333573466e1cd55cea80124000466442466002006004602c6ae854008cd4040054d5d09aba2500223263201a33573803603403026aae7940044dd50009aba150043335500875ca00e6ae85400cc8c8c8cccd5cd19b875001480108c84888c008010d5d09aab9e500323333573466e1d4009200223212223001004375c6ae84d55cf280211999ab9a3370ea00690001091100191931900e19ab9c01d01c01a019018135573aa00226ea8004d5d0a80119a8063ae357426ae8940088c98c8058cd5ce00b80b00a09aba25001135744a00226aae7940044dd5000899aa800bae75a224464460046eac004c8004d5406488c8cccd55cf80112804119a80399aa80498031aab9d5002300535573ca00460086ae8800c04c4d5d08008891001091091198008020018891091980080180109119191999ab9a3370ea0029000119091180100198029aba135573ca00646666ae68cdc3a801240044244002464c6402066ae700440400380344d55cea80089baa001232323333573466e1d400520062321222230040053007357426aae79400c8cccd5cd19b875002480108c848888c008014c024d5d09aab9e500423333573466e1d400d20022321222230010053007357426aae7940148cccd5cd19b875004480008c848888c00c014dd71aba135573ca00c464c6402066ae7004404003803403002c4d55cea80089baa001232323333573466e1cd55cea80124000466442466002006004600a6ae854008dd69aba135744a004464c6401866ae700340300284d55cf280089baa0012323333573466e1cd55cea800a400046eb8d5d09aab9e500223263200a33573801601401026ea80048c8c8c8c8c8cccd5cd19b8750014803084888888800c8cccd5cd19b875002480288488888880108cccd5cd19b875003480208cc8848888888cc004024020dd71aba15005375a6ae84d5d1280291999ab9a3370ea00890031199109111111198010048041bae35742a00e6eb8d5d09aba2500723333573466e1d40152004233221222222233006009008300c35742a0126eb8d5d09aba2500923333573466e1d40192002232122222223007008300d357426aae79402c8cccd5cd19b875007480008c848888888c014020c038d5d09aab9e500c23263201333573802802602202001e01c01a01801626aae7540104d55cf280189aab9e5002135573ca00226ea80048c8c8c8c8cccd5cd19b875001480088ccc888488ccc00401401000cdd69aba15004375a6ae85400cdd69aba135744a00646666ae68cdc3a80124000464244600400660106ae84d55cf280311931900619ab9c00d00c00a009135573aa00626ae8940044d55cf280089baa001232323333573466e1d400520022321223001003375c6ae84d55cf280191999ab9a3370ea004900011909118010019bae357426aae7940108c98c8024cd5ce00500480380309aab9d50011375400224464646666ae68cdc3a800a40084244400246666ae68cdc3a8012400446424446006008600c6ae84d55cf280211999ab9a3370ea00690001091100111931900519ab9c00b00a008007006135573aa00226ea80048c8cccd5cd19b8750014800880348cccd5cd19b8750024800080348c98c8018cd5ce00380300200189aab9d37540029309000a4810350543100112330010020072253350021001100612335002223335003220020020013500122001122123300100300222333573466e1c00800401000c488008488004448c8c00400488cc00cc008008005f5f6";

/// `Vec<TransactionInput>` CBOR (4 inputs).
const INPUTS_HEX: &str = "84825820b16778c9cf065d9efeefe37ec269b4fc5107ecdbd0dd6bf3274b224165c2edd9008258206c732139de33e916342707de2aebef2252c781640326ff37b86ec99d97f1ba8d01825820975c17a4fed0051be622328efa548e206657d2b65a19224bf6ff8132571e6a500282582018f86700660fc88d0370a8f95ea58f75507e6b27a18a17925ad3b1777eb0d77600";

/// `Vec<TransactionOutput>` CBOR (4 outputs).
const OUTPUTS_HEX: &str = "8482581d60b6c8794e9a7a26599440a4d0fd79cd07644d15917ff13694f1f67235821a000f8548a1581c15be994a64bdb79dde7fe080d8e7ff81b33a9e4860e9ee0d857a8e85a144576177610182581d60b6c8794e9a7a26599440a4d0fd79cd07644d15917ff13694f1f672351b00000001af14b8b482581d60b6c8794e9a7a26599440a4d0fd79cd07644d15917ff13694f1f672351a0098968082581d60b6c8794e9a7a26599440a4d0fd79cd07644d15917ff13694f1f672351a00acd8c6";

/// Slot config from aiken's unit test — Preview network anchor.
const PREVIEW_SLOT_CONFIG: SlotConfig = (1660003200000, 0, 1000);

fn decode_hex(s: &str) -> Vec<u8> {
    let cleaned: String = s.chars().filter(|c| !c.is_whitespace()).collect();
    let bytes = cleaned.as_bytes();
    let mut out = Vec::with_capacity(bytes.len() / 2);
    let mut i = 0;
    while i + 1 < bytes.len() {
        out.push((hex_digit(bytes[i]) << 4) | hex_digit(bytes[i + 1]));
        i += 2;
    }
    out
}

fn hex_digit(b: u8) -> u8 {
    match b {
        b'0'..=b'9' => b - b'0',
        b'a'..=b'f' => b - b'a' + 10,
        b'A'..=b'F' => b - b'A' + 10,
        _ => panic!("bad hex digit: {b:?}"),
    }
}

/// Split a `Vec<T>` CBOR into individual item slices.
fn split_array_items(cbor: &[u8]) -> Vec<Vec<u8>> {
    use ade_codec::cbor::{self, ContainerEncoding};
    let mut off = 0;
    let enc = cbor::read_array_header(cbor, &mut off).expect("array header");
    let mut items = Vec::new();
    match enc {
        ContainerEncoding::Definite(n, _) => {
            for _ in 0..n {
                let start = off;
                cbor::skip_item(cbor, &mut off).expect("skip item");
                items.push(cbor[start..off].to_vec());
            }
        }
        ContainerEncoding::Indefinite => {
            while !cbor::is_break(cbor, off).expect("break") {
                let start = off;
                cbor::skip_item(cbor, &mut off).expect("skip item");
                items.push(cbor[start..off].to_vec());
            }
        }
    }
    items
}

#[test]
fn aiken_fixture_tx_evaluates_end_to_end() {
    let tx_cbor = decode_hex(TX_HEX);
    let inputs_cbor = decode_hex(INPUTS_HEX);
    let outputs_cbor = decode_hex(OUTPUTS_HEX);

    let inputs = split_array_items(&inputs_cbor);
    let outputs = split_array_items(&outputs_cbor);
    assert_eq!(inputs.len(), outputs.len(), "inputs/outputs must zip");

    let resolved_utxos: Vec<(Vec<u8>, Vec<u8>)> =
        inputs.into_iter().zip(outputs.into_iter()).collect();

    let result = eval_tx_phase_two(
        &tx_cbor,
        &resolved_utxos,
        None, // aiken's defaults suffice for this fixture's tautological script
        (10_000_000_000, 14_000_000),
        PREVIEW_SLOT_CONFIG,
    );

    match result {
        Ok(res) => {
            assert_eq!(
                res.scripts.len(),
                1,
                "aiken fixture has exactly one Plutus script",
            );
            let s = &res.scripts[0];
            assert!(
                s.success,
                "script should succeed (tautological mintTestValidator)",
            );
            assert!(s.cpu > 0, "cpu must be > 0 post-eval");
            assert!(s.mem > 0, "mem must be > 0 post-eval");
            eprintln!(
                "ade_plutus end-to-end eval OK: mem={}, cpu={}",
                s.mem, s.cpu,
            );
        }
        Err(e) => panic!("aiken fixture should evaluate to Ok, got: {e:?}"),
    }
}

/// Adversarial: a transaction whose redeemer UNDER-DECLARES its ex_units
/// must be rejected.
///
/// cardano-ledger phase-2 caps each script at the ex_units DECLARED in
/// its redeemer; a script that overruns its declared budget fails
/// (`ValidationTagMismatch`, collateral consumed). aiken's
/// `eval_phase_two_raw` does NOT enforce this — it evaluates each script
/// against the `initial_budget` (Ade passes protocol-max) and only
/// rewrites the redeemer's ex_units to the *actual* consumption. So the
/// per-script declared cap must be enforced by Ade.
///
/// This test takes the honest fixture (declares (mem=747528,
/// cpu=217294271), == actual) and mutates the DECLARED ex_units down to
/// (1, 1), same byte width so no other offset shifts. The script still
/// consumes 747528 / 217294271 — far over its declared (1, 1) — so it
/// must NOT succeed. Before the per-script-cap fix this test fails
/// (Ade reports `success = true`); that failure is the confirmation of
/// the budget-cap false-accept.
#[test]
fn under_declared_ex_units_must_reject() {
    // redeemer = [1, 0, d87980, [747528, 217294271]] — the ex_units array
    // is `821a000b68081a0cf3a5bf`, prefixed here by the redeemer data
    // `d87980` to pin a unique match.
    const HONEST_EX_UNITS: &str = "d87980821a000b68081a0cf3a5bf";
    // Same shape, declared ex_units rewritten to (mem=1, cpu=1).
    const UNDER_DECLARED: &str = "d87980821a000000011a00000001";
    assert_eq!(
        TX_HEX.matches(HONEST_EX_UNITS).count(),
        1,
        "redeemer ex_units must appear exactly once for a surgical mutation",
    );
    let mutated = TX_HEX.replace(HONEST_EX_UNITS, UNDER_DECLARED);

    let tx_cbor = decode_hex(&mutated);
    let inputs = split_array_items(&decode_hex(INPUTS_HEX));
    let outputs = split_array_items(&decode_hex(OUTPUTS_HEX));
    let resolved_utxos: Vec<(Vec<u8>, Vec<u8>)> =
        inputs.into_iter().zip(outputs.into_iter()).collect();

    let result = eval_tx_phase_two(
        &tx_cbor,
        &resolved_utxos,
        None,
        // Protocol max — the budget Ade currently (wrongly) caps each
        // script at. The script is within this, so aiken runs it Ok; the
        // declared (1, 1) cap is the smaller bound that must be enforced.
        (10_000_000_000, 14_000_000),
        PREVIEW_SLOT_CONFIG,
    );

    let res = result.expect("eval runs: the script is within protocol max");
    assert_eq!(res.scripts.len(), 1, "fixture has exactly one Plutus script");
    let s = &res.scripts[0];
    // The script genuinely consumed far more than its declared (1, 1):
    // aiken measures mem=747528, cpu=168699461 regardless of the declared
    // value (it caps at `initial_budget`, not the redeemer's ex_units).
    assert!(
        s.mem >= 747_528 && s.cpu > 1_000_000,
        "actual consumption measured: mem={}, cpu={}",
        s.mem,
        s.cpu,
    );
    // Over-ran its declared budget → must be rejected, not accepted.
    assert!(
        !s.success,
        "script consumed mem={} cpu={} against declared (mem=1, cpu=1) — \
         must be rejected as over-budget (false-accept if accepted)",
        s.mem,
        s.cpu,
    );
}

/// Adversarial: a Plutus validator that always evaluates to `false` must be
/// rejected (phase-2 failure → the ledger maps it to `PlutusEvalOutcome::Failed`,
/// collateral consumed). Ported from aiken's `test_eval_4` (v1.1.21): a Plutus V1
/// minting policy compiled from `func main() Bool { false }`. The script logically
/// fails, so `eval_tx_phase_two` must surface an error — never a successful verdict.
#[test]
fn failing_validator_must_reject() {
    const TX4_HEX: &str = "84A80081825820275B5DA338C8B899035081EB34BFA950B634911A5DD3271B3AD6CF4C2BBA0C50010182825839000AF00CC47500BB64CFFFB783E8C42F746B4E8B8A70EDE9C08C7113ACF3BDE34D1041F5A2076EF9AA6CF4539AB1A96ED462A0300ACBDB65D5821A00111958A1581C1E8BCA1FA1D937F408AFE2FD4DBF343AB7A09CF07984071ED95B3C92A1400A825839000AF00CC47500BB64CFFFB783E8C42F746B4E8B8A70EDE9C08C7113ACF3BDE34D1041F5A2076EF9AA6CF4539AB1A96ED462A0300ACBDB65D51A029F7B29021A0002BC5009A1581C1E8BCA1FA1D937F408AFE2FD4DBF343AB7A09CF07984071ED95B3C92A1400A0B58205013DBE72526511F63B0C4A235FBBC5D09D11D42F310113AAAB1A28E01E0BDE60D81825820275B5DA338C8B899035081EB34BFA950B634911A5DD3271B3AD6CF4C2BBA0C500110825839000AF00CC47500BB64CFFFB783E8C42F746B4E8B8A70EDE9C08C7113ACF3BDE34D1041F5A2076EF9AA6CF4539AB1A96ED462A0300ACBDB65D51A02AF3659111A00041A78A30081825820065DD553FBE4E240A8F819BB9E333A7483DE4A22B65C7FB6A95CE9450F84DFF758401679B607EABEF3DBBC9AC0ABB03AFB3A979EA32243BB5B99E299290D709BB4A6C2AA528C447C2DB610A103CC9C0E7C018CAA4FC8322D8EC217620E6D4BC2EF0B03815453010000322233335734600693124C4C931250010581840100D87980821909611A00094D78F5F6";
    const INPUTS4_HEX: &str = "8682582075a419179618ca358554fc47aeb33b6c93d12ba8f752495a4e5ef6ea0a1a099a03825820b810e77e706ccaebf7284f6c3d41e2a1eb4af5fabae452618f4175ad1b2aaded03825820975c17a4fed0051be622328efa548e206657d2b65a19224bf6ff8132571e6a50038258207453531a00f98db47c8c2b05e5c38f2c40a0be4f91d42d835bc3bc998b612a8e00825820452b2fc0d170323f86ad1e5b761dcae912774c42c1b1af4de2905a094f2f541403825820275b5da338c8b899035081eb34bfa950b634911a5dd3271b3ad6cf4c2bba0c5001";
    const OUTPUTS4_HEX: &str = "86825839000af00cc47500bb64cfffb783e8c42f746b4e8b8a70ede9c08c7113acf3bde34d1041f5a2076ef9aa6cf4539ab1a96ed462a0300acbdb65d5821a00412c6aad581c01dd79d464e2446231d662c9422445c4cf709b691baceb8b040a34d4a14d28323232294d6174726978363701581c11638d4d600d32b2849c93314e7e6bc656fded924f30514749e1eb3ea64c28323232294d617472697830014c28323232294d617472697831014d28323232294d61747269783133014d28323232294d61747269783330014d28323232294d61747269783534014d28323232294d6174726978383601581c1e60ac8228a21f6e3685c73970584aa54504a0476c4b66f3ef5c4dd2a14c28323232294d61747269783001581c25da1064122988292665c14259ea26cb4dd96d7f04535125fea248ffa14c28323232294d61747269783001581c46214283f4b5cc5d66836a4fe743e121190f1e5b91448a1b52f1b7bfa14d28323232294d6174726978313801581c4788a484721270845917e0986ab55b51922a46b514eb7a1f871e917ca14d28323232294d6174726978323101581c6ed9951ddcd79c98bc50142ba033890815330d4de1cb4c96870a234ca24c28323232294d617472697830014c28323232294d61747269783101581c6f7fd77c85b9856bdb1cfac1afa90c65d92c3c5e2fcca4a993e7fb52a14d28323232294d6174726978323401581ca07afd05db7f0ccb144052935be97b48593e5c8435f9eb859191de81a34c28323232294d617472697830014c28323232294d617472697831014d28323232294d6174726978323101581ca5ca38805c14270ec4c3c1c2446b28a95324054fac98066c5e82a016a14d28323232294d6174726978313901581ca65e6e94d1a260dbc6c4d9319b45585fa54b83742a33a2c599df56b9a2494265727279436f616c014c426572727954616e67656c6f01581cb3e2625ebd6bd613ce904db9fedb0565eec0671054d30d08bc5edadda44c28323232294d617472697835014d28323232294d61747269783237014d28323232294d61747269783430014d28323232294d6174726978343701581ce3ef435a5910f74d890b2a7cb0d1f7288efc22c75823d57acdab9f52a14d28323232294d6174726978363101825839000af00cc47500bb64cfffb783e8c42f746b4e8b8a70ede9c08c7113acf3bde34d1041f5a2076ef9aa6cf4539ab1a96ed462a0300acbdb65d5821a0011f436a1581c11638d4d600d32b2849c93314e7e6bc656fded924f30514749e1eb3ea14d28323232294d6174726978363801825839000af00cc47500bb64cfffb783e8c42f746b4e8b8a70ede9c08c7113acf3bde34d1041f5a2076ef9aa6cf4539ab1a96ed462a0300acbdb65d5821a0011f436a1581c11638d4d600d32b2849c93314e7e6bc656fded924f30514749e1eb3ea14d28323232294d6174726978343101825839000af00cc47500bb64cfffb783e8c42f746b4e8b8a70ede9c08c7113acf3bde34d1041f5a2076ef9aa6cf4539ab1a96ed462a0300acbdb65d5821a0012378ea1581c2c04f7a15aec58b2bec5dab3d201f3e3898370b98d2f01d4ac8bc270a14d28323232294d6174726978323801825839000af00cc47500bb64cfffb783e8c42f746b4e8b8a70ede9c08c7113acf3bde34d1041f5a2076ef9aa6cf4539ab1a96ed462a0300acbdb65d5821a0011f436a1581c11638d4d600d32b2849c93314e7e6bc656fded924f30514749e1eb3ea14d28323232294d6174726978323101825839000af00cc47500bb64cfffb783e8c42f746b4e8b8a70ede9c08c7113acf3bde34d1041f5a2076ef9aa6cf4539ab1a96ed462a0300acbdb65d51a02b350d1";

    let tx = decode_hex(TX4_HEX);
    let inputs = split_array_items(&decode_hex(INPUTS4_HEX));
    let outputs = split_array_items(&decode_hex(OUTPUTS4_HEX));
    assert_eq!(inputs.len(), outputs.len(), "inputs/outputs must zip");
    let resolved: Vec<(Vec<u8>, Vec<u8>)> = inputs.into_iter().zip(outputs).collect();

    let result = eval_tx_phase_two(
        &tx,
        &resolved,
        None,
        (10_000_000_000, 14_000_000),
        PREVIEW_SLOT_CONFIG,
    );
    assert!(
        result.is_err(),
        "a Plutus validator that returns false must be rejected, got: {result:?}",
    );
}

/// Pull the body's input items (map key 0) out of a full tx CBOR, so a
/// fixture that resolves its inputs from the body (rather than a separate
/// array) can be zipped with explicit resolved outputs.
fn extract_body_inputs(tx_cbor: &[u8]) -> Vec<Vec<u8>> {
    use ade_codec::cbor::{self, ContainerEncoding};
    let mut o = 0;
    cbor::read_array_header(tx_cbor, &mut o).expect("tx array");
    let n = match cbor::read_map_header(tx_cbor, &mut o).expect("body map") {
        ContainerEncoding::Definite(n, _) => n,
        ContainerEncoding::Indefinite => panic!("indefinite body map unsupported in fixture"),
    };
    for _ in 0..n {
        let (key, _) = cbor::read_uint(tx_cbor, &mut o).expect("body key");
        if key == 0 {
            return split_array_items(&tx_cbor[o..]);
        }
        cbor::skip_item(tx_cbor, &mut o).expect("skip body value");
    }
    panic!("tx body has no inputs (key 0)");
}

/// Adversarial: a transaction carrying an EXTRANEOUS redeemer — one that
/// resolves to no script — must be rejected. Ported from aiken's
/// `eval_extraneous_redeemer` (v1.1.21): the tx declares two redeemers but
/// the witness set provides no script the extra one can bind to, so phase-2
/// cannot resolve it and must fail rather than silently accept.
#[test]
fn extraneous_redeemer_must_reject() {
    const TXEXT_HEX: &str = "84a70082825820275b5da338c8b899035081eb34bfa950b634911a5dd3271b3ad6cf4c2bba0c5000825820275b5da338c8b899035081eb34bfa950b634911a5dd3271b3ad6cf4c2bba0c50010181825839000af00cc47500bb64cfffb783e8c42f746b4e8b8a70ede9c08c7113acf3bde34d1041f5a2076ef9aa6cf4539ab1a96ed462a0300acbdb65d51a02cf2b47021a0002aa0a0b5820fc54f302cff3a8a1cb374f5e4979e18a1d3627dcf4539637b03f5959eb8565bf0d81825820275b5da338c8b899035081eb34bfa950b634911a5dd3271b3ad6cf4c2bba0c500110825839000af00cc47500bb64cfffb783e8c42f746b4e8b8a70ede9c08c7113acf3bde34d1041f5a2076ef9aa6cf4539ab1a96ed462a0300acbdb65d51a02af51c2111a0003ff0fa40081825820065dd553fbe4e240a8f819bb9e333a7483de4a22b65c7fb6a95ce9450f84dff758402c26125a057a696079d08f2c8c9d2b8ccda9fe7cf7360c1a86712b85a91db82a3b80996b30ba6f4b2f969c93eb50694e0f6ea0bcf129080dcc07ecd9e605f00a049fd87980ff0582840000d879808219044c1a000382d48401001864821903e81903e8068149480100002221200101f5f6";
    const OUTEXT_HEX: &str = "82825839000af00cc47500bb64cfffb783e8c42f746b4e8b8a70ede9c08c7113acf3bde34d1041f5a2076ef9aa6cf4539ab1a96ed462a0300acbdb65d51a02b3603082581d703a888d65f16790950a72daee1f63aa05add6d268434107cfa5b677121a001e8480";

    let tx = decode_hex(TXEXT_HEX);
    let inputs = extract_body_inputs(&tx);
    let outputs = split_array_items(&decode_hex(OUTEXT_HEX));
    assert_eq!(inputs.len(), outputs.len(), "inputs/outputs must zip");
    let resolved: Vec<(Vec<u8>, Vec<u8>)> = inputs.into_iter().zip(outputs).collect();

    let result = eval_tx_phase_two(
        &tx,
        &resolved,
        None,
        (10_000_000_000, 14_000_000),
        PREVIEW_SLOT_CONFIG,
    );
    assert!(
        result.is_err(),
        "an extraneous redeemer with no matching script must be rejected, got: {result:?}",
    );
}
