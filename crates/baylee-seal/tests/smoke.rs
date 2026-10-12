//! A first round trip, until the full suite lands.

use baylee_seal::testkit::{Seeded, registered};
use baylee_seal::{Error, Recipient, open, seal, verify_device};

#[test]
fn a_message_opens_on_its_recipient_and_not_when_changed() {
    let (alice, alice_pub) = registered("af03", 1);
    let (bob, bob_pub) = registered("0012", 2);
    verify_device("af03", &alice_pub.ik, &alice_pub.dk, &alice_pub.dsig).unwrap();
    assert_eq!(
        verify_device("0013", &alice_pub.ik, &alice_pub.dk, &alice_pub.dsig),
        Err(Error::Signature)
    );
    let hdr = br#"{"v":1}"#;
    let sealed = seal(
        &alice,
        hdr,
        b"hello",
        &[Recipient {
            device: &bob_pub.id,
            dk: bob_pub.dk,
        }],
        &mut Seeded::new(3),
    )
    .unwrap();
    let opened = open(
        &bob,
        &alice_pub.ik,
        &sealed.envelope(hdr),
        &sealed.wraps[0].k,
    )
    .unwrap();
    assert_eq!(&opened.body[..], b"hello");
    let mut ct = sealed.ct.clone();
    ct[0] ^= 1;
    let mut env = sealed.envelope(hdr);
    env.ct = &ct;
    assert_eq!(
        open(&bob, &alice_pub.ik, &env, &sealed.wraps[0].k).unwrap_err(),
        Error::Signature
    );
    assert_eq!(
        open(
            &alice,
            &alice_pub.ik,
            &sealed.envelope(hdr),
            &sealed.wraps[0].k
        )
        .unwrap_err(),
        Error::Wrap
    );
}
