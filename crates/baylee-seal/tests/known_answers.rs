//! Known answers: every construction's bytes are pinned, so a silent change of construction fails.

mod common;

use baylee_seal::testkit::{Seeded, device, registered};
use baylee_seal::{
    DeviceSecrets, Entropy, RestKind, device_id, open, safety_number, seal, verify_device,
};
use common::{BODY, HDR, hex, pair};

// Pinned: a change here is a change of construction; re-pin only with a reason in the commit.
const DEVICE_1_IK: &str = "478b8e507e0bb2b18c0f9e0824769e8562d10df9abe2e774896f82b4b4405266";
const DEVICE_1_DK: &str = "09a8173423eb0e24e19f4ad87cb92bb008dc99a5d722baa9a3fad3eb08b49f3a";
const DEVICE_1_ID: &str = "41cf7431df60b65b";
const DEVICE_1_AF03_DSIG: &str = "535b5e8aa81445862e5a716c92524a25e6c35a9c23ad6628524b2181bbc4a6af7cc8c78983f1d3a1e698320589c93bfb4669515c83285ae819c05a5b89d6870d";
const DEVICE_2_ID: &str = "320240c00cdfe609";
const SEALED_N: &str = "3a9afa8c98415d2fde7ae061aed1ef6821fb9ab3e89e9c7d";
const SEALED_CT: &str = "41167bb1b7f68a9e3746b1f7d045fe0829f8d9abebf4ca1da832";
const SEALED_EPK: &str = "a84805be914fd238e13af41f5310f9d578e4d2ee4141313161dc59e61b604665";
const SEALED_SIG: &str = "3481526e3e1640e88a8f959b520fb0e59053a9a19470c0d03d7dfbe7305e39f979416711918f548ee63254972d057c1272ec16abed57e5ce81114ca90fa6420a";
const SEALED_WRAP_K: &str = "be7f922b053d8b6260331b2a6fa3a08dd3ec50ca251a1e6a7fe94f28db4b4e402bb07b510f6beeb4dec662f012830c29";
const AT_REST_RECORD: &str = "fa422e4b92882fdd13c615f7bcfff882f74eb2b38eb67653066f7d2d259ddef89e3356c13955617e0d7f3c976b8a305c38e578cc";
const SAFETY_NUMBER: &str = "544341658696580393492989330337862547088083736398920419616886";

#[test]
fn the_first_testkit_device_has_pinned_keys_and_id() {
    let (secrets, public) = registered("af03", 1);
    assert_eq!(hex(&public.ik), DEVICE_1_IK);
    assert_eq!(hex(&public.dk), DEVICE_1_DK);
    assert_eq!(device_id(&public.ik), DEVICE_1_ID);
    assert_eq!(public.id, DEVICE_1_ID);
    assert_eq!(secrets.device_id(), DEVICE_1_ID);
}

#[test]
fn a_device_statement_signature_is_pinned_and_verifies() {
    let (_, public) = registered("af03", 1);
    assert_eq!(hex(&public.dsig), DEVICE_1_AF03_DSIG);
    verify_device("af03", &public.ik, &public.dk, &public.dsig).unwrap();
}

#[test]
fn a_sealed_message_has_pinned_bytes() {
    let p = pair();
    assert_eq!(p.bob_pub.id, DEVICE_2_ID);
    assert_eq!(hex(&p.sealed.n), SEALED_N);
    assert_eq!(hex(&p.sealed.ct), SEALED_CT);
    assert_eq!(hex(&p.sealed.epk), SEALED_EPK);
    assert_eq!(hex(&p.sealed.sig), SEALED_SIG);
    assert_eq!(p.sealed.wraps.len(), 1);
    assert_eq!(p.sealed.wraps[0].device, DEVICE_2_ID);
    assert_eq!(hex(&p.sealed.wraps[0].k), SEALED_WRAP_K);
}

#[test]
fn the_pinned_message_opens_to_its_body() {
    let p = pair();
    let opened = open(
        &p.bob,
        &p.alice_pub.ik,
        &p.sealed.envelope(HDR),
        &p.sealed.wraps[0].k,
    )
    .unwrap();
    assert_eq!(&opened.body[..], BODY);
}

#[test]
fn an_at_rest_record_has_pinned_bytes() {
    let record = baylee_seal::at_rest_seal(
        &[9u8; 32],
        RestKind::History,
        "3f9a.hist",
        b"{\"lines\":[]}",
        &mut Seeded::new(11),
    )
    .unwrap();
    assert_eq!(hex(&record), AT_REST_RECORD);
}

#[test]
fn a_safety_number_has_pinned_digits() {
    let a = registered("af03", 1).1.ik;
    let b = registered("0012", 2).1.ik;
    let c = registered("0012", 3).1.ik;
    assert_eq!(
        safety_number(("af03", &[a]), ("0012", &[b, c])),
        SAFETY_NUMBER
    );
}

#[test]
fn generate_draws_ik_seed_dk_seed_and_vault_key_in_that_order() {
    for seed in [1u64, 2, 99] {
        let mut direct = [0u8; 96];
        Seeded::new(seed).fill(&mut direct);
        assert_eq!(&device(seed).to_bytes()[..], &direct[..]);
        assert_eq!(
            &DeviceSecrets::generate(&mut Seeded::new(seed)).to_bytes()[..],
            &direct[..]
        );
    }
}

#[test]
fn seal_draws_the_message_key_then_the_nonce() {
    let (alice, _) = registered("af03", 1);
    let (bob, bob_pub) = registered("0012", 2);
    let mut stream = [0u8; 56];
    Seeded::new(5).fill(&mut stream);
    let sealed = seal(
        &alice,
        HDR,
        BODY,
        &[baylee_seal::Recipient {
            device: &bob_pub.id,
            dk: bob_pub.dk,
        }],
        &mut Seeded::new(5),
    )
    .unwrap();
    assert_eq!(&sealed.n[..], &stream[32..56]);
    let opened = open(&bob, &alice.ik(), &sealed.envelope(HDR), &sealed.wraps[0].k).unwrap();
    assert_eq!(&opened.mk[..], &stream[..32]);
}

#[test]
fn the_same_seed_seals_the_same_message_twice() {
    let a = pair();
    let b = pair();
    assert_eq!(a.sealed, b.sealed);
}
