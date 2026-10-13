//! Reported messages (`verify_report`) and device statements (`verify_device`).

mod common;

use baylee_seal::testkit::registered;
use baylee_seal::{Envelope, Error, open, verify_device, verify_report};
use common::{BODY, HDR, flipped, pair};

fn real_mk() -> [u8; 32] {
    let p = pair();
    let opened = open(
        &p.bob,
        &p.alice_pub.ik,
        &p.sealed.envelope(HDR),
        &p.sealed.wraps[0].k,
    )
    .unwrap();
    *opened.mk
}

#[test]
fn a_report_with_the_real_key_and_body_verifies() {
    let p = pair();
    assert_eq!(
        verify_report(&p.alice_pub.ik, &p.sealed.envelope(HDR), &real_mk(), BODY),
        Ok(())
    );
}

#[test]
fn a_report_with_the_wrong_message_key_fails_as_a_body() {
    let p = pair();
    let wrong = flipped(&real_mk());
    let wrong: [u8; 32] = wrong.try_into().unwrap();
    assert_eq!(
        verify_report(&p.alice_pub.ik, &p.sealed.envelope(HDR), &wrong, BODY),
        Err(Error::Body)
    );
}

#[test]
fn a_report_with_a_body_that_differs_by_one_byte_is_a_mismatch() {
    let p = pair();
    let body = flipped(BODY);
    assert_eq!(
        verify_report(&p.alice_pub.ik, &p.sealed.envelope(HDR), &real_mk(), &body),
        Err(Error::Mismatch)
    );
    let mut last = BODY.to_vec();
    *last.last_mut().unwrap() ^= 1;
    assert_eq!(
        verify_report(&p.alice_pub.ik, &p.sealed.envelope(HDR), &real_mk(), &last),
        Err(Error::Mismatch)
    );
}

#[test]
fn a_report_with_a_body_one_byte_longer_or_shorter_is_a_mismatch() {
    let p = pair();
    let mut longer = BODY.to_vec();
    longer.push(b'!');
    assert_eq!(
        verify_report(
            &p.alice_pub.ik,
            &p.sealed.envelope(HDR),
            &real_mk(),
            &longer
        ),
        Err(Error::Mismatch)
    );
    assert_eq!(
        verify_report(
            &p.alice_pub.ik,
            &p.sealed.envelope(HDR),
            &real_mk(),
            &BODY[..BODY.len() - 1]
        ),
        Err(Error::Mismatch)
    );
    assert_eq!(
        verify_report(&p.alice_pub.ik, &p.sealed.envelope(HDR), &real_mk(), b""),
        Err(Error::Mismatch)
    );
}

#[test]
fn a_report_checked_against_another_senders_key_fails_the_signature() {
    let p = pair();
    // Bob's key stands for "a device the gateway did not look up as the sender".
    assert_eq!(
        verify_report(&p.bob_pub.ik, &p.sealed.envelope(HDR), &real_mk(), BODY),
        Err(Error::Signature)
    );
}

#[test]
fn a_report_of_a_changed_message_fails_before_the_key_is_tried() {
    let p = pair();
    let mut ct = p.sealed.ct.clone();
    ct[0] ^= 1;
    let envelope = Envelope {
        ct: &ct,
        ..p.sealed.envelope(HDR)
    };
    assert_eq!(
        verify_report(&p.alice_pub.ik, &envelope, &real_mk(), BODY),
        Err(Error::Signature)
    );
}

#[test]
fn a_report_of_a_message_with_another_header_fails_the_signature() {
    let p = pair();
    let envelope = p
        .sealed
        .envelope(br#"{"v":1,"conv":"dm:0012:ffff","id":"m1"}"#);
    assert_eq!(
        verify_report(&p.alice_pub.ik, &envelope, &real_mk(), BODY),
        Err(Error::Signature)
    );
}

#[test]
fn a_report_with_a_field_of_the_wrong_length_fails_by_length() {
    let p = pair();
    let envelope = Envelope {
        sig: &p.sealed.sig[..63],
        ..p.sealed.envelope(HDR)
    };
    assert_eq!(
        verify_report(&p.alice_pub.ik, &envelope, &real_mk(), BODY),
        Err(Error::Length)
    );
}

#[test]
fn a_device_statement_verifies() {
    let (_, public) = registered("af03", 1);
    assert_eq!(
        verify_device("af03", &public.ik, &public.dk, &public.dsig),
        Ok(())
    );
}

#[test]
fn a_device_statement_checked_against_another_tag_fails_the_signature() {
    let (_, public) = registered("af03", 1);
    for tag in ["af04", "af0", "af033", "", "AF03"] {
        assert_eq!(
            verify_device(tag, &public.ik, &public.dk, &public.dsig),
            Err(Error::Signature),
            "tag {tag:?}"
        );
    }
}

#[test]
fn a_device_statement_with_a_flipped_bit_fails_the_signature() {
    let (_, public) = registered("af03", 1);
    let dsig: [u8; 64] = flipped(&public.dsig).try_into().unwrap();
    assert_eq!(
        verify_device("af03", &public.ik, &public.dk, &dsig),
        Err(Error::Signature)
    );
}

#[test]
fn a_device_statement_with_another_devices_encryption_key_fails_the_signature() {
    let (_, public) = registered("af03", 1);
    let (_, other) = registered("af03", 2);
    assert_eq!(
        verify_device("af03", &public.ik, &other.dk, &public.dsig),
        Err(Error::Signature)
    );
}

#[test]
fn a_device_statement_cannot_be_borrowed_from_another_device() {
    let (_, one) = registered("af03", 1);
    let (_, two) = registered("af03", 2);
    assert_eq!(
        verify_device("af03", &two.ik, &two.dk, &one.dsig),
        Err(Error::Signature)
    );
}

#[test]
fn an_encryption_key_of_small_order_is_refused_as_non_contributory() {
    let (_, public) = registered("af03", 1);
    assert_eq!(
        verify_device("af03", &public.ik, &[0u8; 32], &public.dsig),
        Err(Error::NonContributory)
    );
    let mut one = [0u8; 32];
    one[0] = 1;
    assert_eq!(
        verify_device("af03", &public.ik, &one, &public.dsig),
        Err(Error::NonContributory)
    );
}

#[test]
fn a_tag_of_256_bytes_is_refused_by_length() {
    let (_, public) = registered("af03", 1);
    let tag = "a".repeat(256);
    assert_eq!(
        verify_device(&tag, &public.ik, &public.dk, &public.dsig),
        Err(Error::Length)
    );
    assert_eq!(
        public_for(&tag),
        Err(Error::Length),
        "a device cannot publish for one either"
    );
}

fn public_for(tag: &str) -> Result<(), Error> {
    baylee_seal::testkit::device(1).public(tag).map(|_| ())
}

#[test]
fn a_tag_of_255_bytes_is_the_longest_that_signs_and_verifies() {
    let tag = "a".repeat(255);
    let secrets = baylee_seal::testkit::device(1);
    let public = secrets.public(&tag).unwrap();
    assert_eq!(
        verify_device(&tag, &public.ik, &public.dk, &public.dsig),
        Ok(())
    );
}

#[test]
fn an_identity_key_of_small_order_is_refused_as_weak() {
    let (_, public) = registered("af03", 1);
    let mut identity = [0u8; 32];
    identity[0] = 1;
    assert_eq!(
        verify_device("af03", &identity, &public.dk, &public.dsig),
        Err(Error::WeakKey)
    );
    assert_eq!(
        verify_device("af03", &[0u8; 32], &public.dk, &public.dsig),
        Err(Error::WeakKey)
    );
}

#[test]
fn an_identity_key_that_is_no_point_is_refused_as_a_key() {
    let (_, public) = registered("af03", 1);
    let mut identity = [0u8; 32];
    identity[0] = 2;
    assert_eq!(
        verify_device("af03", &identity, &public.dk, &public.dsig),
        Err(Error::Key)
    );
}

#[test]
fn a_report_with_the_key_of_another_message_fails_as_a_body() {
    let p = pair();
    let (alice, _) = registered("af03", 1);
    let (bob, bob_pub) = registered("0012", 2);
    let other = baylee_seal::seal(
        &alice,
        HDR,
        BODY,
        &[baylee_seal::Recipient {
            device: &bob_pub.id,
            dk: bob_pub.dk,
        }],
        &mut baylee_seal::testkit::Seeded::new(8),
    )
    .unwrap();
    let other_mk = open(
        &bob,
        &p.alice_pub.ik,
        &other.envelope(HDR),
        &other.wraps[0].k,
    )
    .unwrap()
    .mk;
    assert_ne!(*other_mk, real_mk());
    assert_eq!(
        verify_report(&p.alice_pub.ik, &p.sealed.envelope(HDR), &other_mk, BODY),
        Err(Error::Body)
    );
}

#[test]
fn a_report_with_a_ciphertext_shorter_than_its_commitment_and_tag_fails_by_length() {
    let p = pair();
    let envelope = Envelope {
        ct: &p.sealed.ct[..47],
        ..p.sealed.envelope(HDR)
    };
    assert_eq!(
        verify_report(&p.alice_pub.ik, &envelope, &real_mk(), BODY),
        Err(Error::Length)
    );
}

#[test]
fn a_report_with_a_non_canonical_sender_key_fails_as_a_key() {
    let p = pair();
    assert_eq!(
        verify_report(&[0xff; 32], &p.sealed.envelope(HDR), &real_mk(), BODY),
        Err(Error::Key)
    );
}

#[test]
fn a_non_canonical_identity_key_is_refused_as_a_key_in_a_device_statement() {
    let (_, public) = registered("af03", 1);
    assert_eq!(
        verify_device("af03", &[0xff; 32], &public.dk, &public.dsig),
        Err(Error::Key)
    );
}

#[test]
fn a_fetched_device_verifies_for_its_tag() {
    let (_, public) = registered("af03", 1);
    assert_eq!(public.verify("af03"), Ok(()));
}

#[test]
fn a_fetched_device_checked_against_another_tag_fails_the_signature() {
    let (_, public) = registered("af03", 1);
    assert_eq!(public.verify("af04"), Err(Error::Signature));
}

#[test]
fn a_fetched_device_with_another_devices_id_is_refused_as_a_key() {
    let (_, public) = registered("af03", 1);
    let (_, other) = registered("af03", 2);
    let mut moved = public.clone();
    moved.id = other.id;
    assert_eq!(moved.verify("af03"), Err(Error::Key));
}

#[test]
fn a_fetched_device_with_an_id_that_is_not_the_keys_spelling_is_refused_as_a_key() {
    let (_, public) = registered("af03", 1);
    for id in [
        public.id.to_uppercase(),
        public.id[..15].to_owned(),
        format!("{}0", public.id),
        String::new(),
    ] {
        let mut changed = public.clone();
        changed.id = id.clone();
        assert_eq!(changed.verify("af03"), Err(Error::Key), "id {id:?}");
    }
}

#[test]
fn a_fetched_device_with_an_encryption_key_of_small_order_is_refused() {
    let (_, public) = registered("af03", 1);
    let mut weak = public.clone();
    weak.dk = [0u8; 32];
    assert_eq!(weak.verify("af03"), Err(Error::NonContributory));
}

#[test]
fn a_fetched_device_with_a_changed_signature_fails_the_signature() {
    let (_, public) = registered("af03", 1);
    let mut changed = public.clone();
    changed.dsig = flipped(&public.dsig).try_into().unwrap();
    assert_eq!(changed.verify("af03"), Err(Error::Signature));
}
