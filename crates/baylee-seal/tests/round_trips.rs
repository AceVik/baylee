//! Round trips: seal for one, two and 250 devices and open on each; records at rest.

mod common;

use baylee_seal::testkit::{Seeded, device};
use baylee_seal::{Recipient, RestKind, at_rest_open, at_rest_seal, open, seal, verify_message};
use common::{BODY, HDR};

fn round_trip_for(count: u64) {
    let sender = device(1000);
    let receivers: Vec<_> = (0..count).map(|i| device(2000 + i)).collect();
    let ids: Vec<String> = receivers.iter().map(|d| d.device_id().to_owned()).collect();
    let dks: Vec<[u8; 32]> = receivers
        .iter()
        .map(baylee_seal::DeviceSecrets::dk)
        .collect();
    let recipients: Vec<Recipient<'_>> = ids
        .iter()
        .zip(&dks)
        .map(|(device, dk)| Recipient { device, dk: *dk })
        .collect();
    let sealed = seal(&sender, HDR, BODY, &recipients, &mut Seeded::new(count)).unwrap();
    assert_eq!(sealed.wraps.len(), receivers.len());
    verify_message(&sender.ik(), &sealed.envelope(HDR)).unwrap();
    let mut mks = Vec::new();
    for (receiver, wrap) in receivers.iter().zip(&sealed.wraps) {
        assert_eq!(wrap.device, receiver.device_id());
        let opened = open(receiver, &sender.ik(), &sealed.envelope(HDR), &wrap.k).unwrap();
        assert_eq!(&opened.body[..], BODY);
        mks.push(*opened.mk);
    }
    assert!(
        mks.iter().all(|mk| *mk == mks[0]),
        "one message key for all"
    );
}

#[test]
fn a_message_for_one_device_opens_on_it() {
    round_trip_for(1);
}

#[test]
fn a_message_for_two_devices_opens_on_each() {
    round_trip_for(2);
}

#[test]
fn a_message_for_250_devices_opens_on_each() {
    round_trip_for(250);
}

#[test]
fn every_wrap_of_one_message_is_different() {
    let sender = device(1);
    let receivers: Vec<_> = (10..13).map(device).collect();
    let ids: Vec<String> = receivers.iter().map(|d| d.device_id().to_owned()).collect();
    let recipients: Vec<Recipient<'_>> = receivers
        .iter()
        .zip(&ids)
        .map(|(r, device)| Recipient { device, dk: r.dk() })
        .collect();
    let sealed = seal(&sender, HDR, BODY, &recipients, &mut Seeded::new(3)).unwrap();
    assert_ne!(sealed.wraps[0].k, sealed.wraps[1].k);
    assert_ne!(sealed.wraps[1].k, sealed.wraps[2].k);
    assert_ne!(sealed.wraps[0].k, sealed.wraps[2].k);
}

#[test]
fn an_empty_body_seals_to_a_commitment_and_a_tag_and_opens() {
    let sender = device(1);
    let receiver = device(2);
    let id = receiver.device_id().to_owned();
    let sealed = seal(
        &sender,
        HDR,
        b"",
        &[Recipient {
            device: &id,
            dk: receiver.dk(),
        }],
        &mut Seeded::new(3),
    )
    .unwrap();
    assert_eq!(sealed.ct.len(), 32 + 16);
    let opened = open(
        &receiver,
        &sender.ik(),
        &sealed.envelope(HDR),
        &sealed.wraps[0].k,
    )
    .unwrap();
    assert!(opened.body.is_empty());
}

#[test]
fn a_message_for_no_device_seals_and_verifies() {
    let sender = device(1);
    let sealed = seal(&sender, HDR, BODY, &[], &mut Seeded::new(3)).unwrap();
    assert!(sealed.wraps.is_empty());
    verify_message(&sender.ik(), &sealed.envelope(HDR)).unwrap();
}

#[test]
fn a_record_at_rest_opens_for_both_kinds() {
    let key = *device(1).vault_key();
    for (kind, name) in [(RestKind::State, "state"), (RestKind::History, "a1b2c3")] {
        let record = at_rest_seal(&key, kind, name, b"{\"k\":1}", &mut Seeded::new(4)).unwrap();
        assert_eq!(record.len(), 24 + 7 + 16);
        let plain = at_rest_open(&key, kind, name, &record).unwrap();
        assert_eq!(&plain[..], b"{\"k\":1}");
    }
}

#[test]
fn an_empty_record_at_rest_is_a_nonce_and_a_tag() {
    let key = *device(1).vault_key();
    let record = at_rest_seal(&key, RestKind::State, "state", b"", &mut Seeded::new(4)).unwrap();
    assert_eq!(record.len(), 40);
    assert!(
        at_rest_open(&key, RestKind::State, "state", &record)
            .unwrap()
            .is_empty()
    );
}

#[test]
fn two_records_of_one_plaintext_differ_by_their_nonce() {
    let key = *device(1).vault_key();
    let mut rng = Seeded::new(4);
    let a = at_rest_seal(&key, RestKind::State, "state", b"x", &mut rng).unwrap();
    let b = at_rest_seal(&key, RestKind::State, "state", b"x", &mut rng).unwrap();
    assert_ne!(a[..24], b[..24]);
    assert_ne!(a, b);
}

#[test]
fn a_record_sealed_from_the_same_draw_has_a_nonce_of_its_own_for_each_plaintext() {
    let key = *device(1).vault_key();
    let seal_with = |name: &str, plain: &[u8]| {
        at_rest_seal(&key, RestKind::State, name, plain, &mut Seeded::new(4)).unwrap()
    };
    let a = seal_with("state", b"one");
    let b = seal_with("state", b"two");
    assert_ne!(a[..24], b[..24], "another plaintext");
    let c = seal_with("other", b"one");
    assert_ne!(a[..24], c[..24], "another file");
    assert_eq!(a, seal_with("state", b"one"), "the same record twice");
}
