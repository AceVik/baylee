//! Known answers: every construction's bytes are pinned, so a silent change of construction fails.

mod common;

use baylee_seal::testkit::{Seeded, device, registered};
use baylee_seal::{
    DeviceSecrets, Entropy, RestKind, device_id, history_name, open, safety_number, seal,
    verify_device,
};
use common::{BODY, HDR, hex, pair};

// Pinned: a change here is a change of construction; re-pin only with a reason in the commit.
const DEVICE_1_IK: &str = "478b8e507e0bb2b18c0f9e0824769e8562d10df9abe2e774896f82b4b4405266";
const DEVICE_1_DK: &str = "09a8173423eb0e24e19f4ad87cb92bb008dc99a5d722baa9a3fad3eb08b49f3a";
const DEVICE_1_ID: &str = "41cf7431df60b65b";
const DEVICE_1_AF03_DSIG: &str = "535b5e8aa81445862e5a716c92524a25e6c35a9c23ad6628524b2181bbc4a6af7cc8c78983f1d3a1e698320589c93bfb4669515c83285ae819c05a5b89d6870d";
const DEVICE_2_ID: &str = "320240c00cdfe609";
const SEALED_N: &str = "5aef1fe4674c04e0cd4fb09cc15aad781839b5aa09316109";
const SEALED_CT: &str = "b143eba8b637366102d2f42ae3fc5c728d4e2c936cacb2fff2825f524be816ef0d3b9c0230e78f1a6cebc8c984d324cc1fa9810595f223f4c85a";
const SEALED_EPK: &str = "121421dd7982b3edd36823635f159a96abd929e6f927ab2cbe6ec760c8553b3f";
const SEALED_SIG: &str = "49fe1f37b5e3c2446e7d52bb36cc25210adb0ca2a26098d51479ca7c02b0f30c28e3d82c9603d3319faabb9ead8b51fd308616f3fd89ed494f1211ffc947d801";
const SEALED_WRAP_K: &str = "2af84db1e003e67944f9fb37e3e5b498ce165f99af1c54a5d5ab2dcd157c30f5296cefcc9405dfaf8e88c8fe8f4e9d33";
const AT_REST_RECORD: &str = "f217d38a7f91226bab85dfc64bc12a2e58c1956c88dd969c0959cd04179b272f8fa555260e958a057a2d583639980cb8631368f6";
const HISTORY_NAME: &str = "09df6dd8a5e0751e";
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

/// Alice's message to Bob, from `Seeded::new(seed)`, with the given header and body.
fn sealed_with(seed: u64, hdr: &[u8], body: &[u8]) -> baylee_seal::Sealed {
    let (alice, _) = registered("af03", 1);
    let (_, bob_pub) = registered("0012", 2);
    seal(
        &alice,
        hdr,
        body,
        &[baylee_seal::Recipient {
            device: &bob_pub.id,
            dk: bob_pub.dk,
        }],
        &mut Seeded::new(seed),
    )
    .unwrap()
}

#[test]
fn a_body_that_differs_gives_another_nonce_and_ciphertext_from_the_same_stream() {
    let a = sealed_with(5, HDR, b"hello, bob");
    let b = sealed_with(5, HDR, b"hello, bobs");
    assert_ne!(a.n, b.n);
    assert_ne!(a.epk, b.epk);
    // The message key differs too: the commitment ahead of the body is its hash.
    assert_ne!(a.ct[..32], b.ct[..32]);
    assert_ne!(a.wraps[0].k, b.wraps[0].k);
}

#[test]
fn a_header_that_differs_gives_another_nonce_from_the_same_stream() {
    let a = sealed_with(5, HDR, BODY);
    let b = sealed_with(5, br#"{"v":1,"conv":"dm:0012:af03","id":"m2"}"#, BODY);
    assert_ne!(a.n, b.n);
    assert_ne!(a.epk, b.epk);
    assert_ne!(a.ct[..32], b.ct[..32]);
}

#[test]
fn a_sender_that_differs_gives_another_nonce_from_the_same_stream() {
    let (_, bob_pub) = registered("0012", 2);
    let recipients = [baylee_seal::Recipient {
        device: &bob_pub.id,
        dk: bob_pub.dk,
    }];
    let one = seal(&device(1), HDR, BODY, &recipients, &mut Seeded::new(5)).unwrap();
    let two = seal(&device(3), HDR, BODY, &recipients, &mut Seeded::new(5)).unwrap();
    assert_ne!(one.n, two.n);
    assert_ne!(one.ct[..32], two.ct[..32]);
}

#[test]
fn a_stream_that_differs_gives_another_message() {
    let a = sealed_with(5, HDR, BODY);
    let b = sealed_with(6, HDR, BODY);
    assert_ne!(a.n, b.n);
    assert_ne!(a.ct, b.ct);
}

#[test]
fn identical_streams_headers_bodies_and_senders_seal_identically() {
    assert_eq!(sealed_with(5, HDR, BODY), sealed_with(5, HDR, BODY));
}

#[test]
fn seal_asks_the_entropy_for_88_bytes_in_one_fill() {
    struct Counting(Vec<usize>);
    impl Entropy for Counting {
        fn fill(&mut self, out: &mut [u8]) {
            self.0.push(out.len());
            out.fill(7);
        }
    }
    let (alice, _) = registered("af03", 1);
    let mut rng = Counting(Vec::new());
    seal(&alice, HDR, BODY, &[], &mut rng).unwrap();
    assert_eq!(rng.0, [88]);
}

#[test]
fn a_ciphertext_opens_with_the_hash_of_its_message_key_ahead_of_the_body() {
    use sha2::{Digest, Sha256};
    let p = pair();
    let opened = open(
        &p.bob,
        &p.alice_pub.ik,
        &p.sealed.envelope(HDR),
        &p.sealed.wraps[0].k,
    )
    .unwrap();
    let commitment = Sha256::new()
        .chain_update(baylee_seal::C_COMMIT)
        .chain_update(&opened.mk[..])
        .finalize();
    assert_eq!(&p.sealed.ct[..32], &commitment[..]);
    assert_eq!(p.sealed.ct.len(), 32 + BODY.len() + 16);
}

#[test]
fn a_history_file_name_is_pinned() {
    assert_eq!(history_name(&[9u8; 32], "dm:0012:af03"), HISTORY_NAME);
}

#[test]
fn the_same_seed_seals_the_same_message_twice() {
    let a = pair();
    let b = pair();
    assert_eq!(a.sealed, b.sealed);
}
