//! Every tamper is refused, as its own error: the signature, the wrap, the lengths and the records at rest.

mod common;

use baylee_seal::testkit::{Seeded, device, registered};
use baylee_seal::{
    Envelope, Error, Recipient, RestKind, at_rest_open, at_rest_seal, open, seal, verify_message,
};
use common::{BODY, HDR, flipped, pair};

/// Opens Alice's message to Bob with the given pieces replaced.
struct Parts {
    hdr: Vec<u8>,
    n: Vec<u8>,
    ct: Vec<u8>,
    epk: Vec<u8>,
    sig: Vec<u8>,
    k: Vec<u8>,
}

fn parts() -> Parts {
    let p = pair();
    Parts {
        hdr: HDR.to_vec(),
        n: p.sealed.n.to_vec(),
        ct: p.sealed.ct.clone(),
        epk: p.sealed.epk.to_vec(),
        sig: p.sealed.sig.to_vec(),
        k: p.sealed.wraps[0].k.to_vec(),
    }
}

impl Parts {
    fn open(&self) -> Result<(), Error> {
        let p = pair();
        self.open_as(&p.bob, &p.alice_pub.ik)
    }

    fn open_as(&self, me: &baylee_seal::DeviceSecrets, sender_ik: &[u8; 32]) -> Result<(), Error> {
        let envelope = Envelope {
            hdr: &self.hdr,
            n: &self.n,
            ct: &self.ct,
            epk: &self.epk,
            sig: &self.sig,
        };
        open(me, sender_ik, &envelope, &self.k).map(|_| ())
    }
}

#[test]
fn the_untouched_message_opens() {
    assert_eq!(parts().open(), Ok(()));
}

#[test]
fn a_flipped_bit_in_the_header_fails_the_signature() {
    let mut t = parts();
    t.hdr = flipped(&t.hdr);
    assert_eq!(t.open(), Err(Error::Signature));
}

#[test]
fn a_flipped_bit_in_the_last_byte_of_the_header_fails_the_signature() {
    let mut t = parts();
    *t.hdr.last_mut().unwrap() ^= 0x80;
    assert_eq!(t.open(), Err(Error::Signature));
}

#[test]
fn a_flipped_bit_in_the_nonce_fails_the_signature() {
    let mut t = parts();
    t.n = flipped(&t.n);
    assert_eq!(t.open(), Err(Error::Signature));
}

#[test]
fn a_flipped_bit_in_the_ciphertext_fails_the_signature() {
    let mut t = parts();
    t.ct = flipped(&t.ct);
    assert_eq!(t.open(), Err(Error::Signature));
}

#[test]
fn a_flipped_bit_in_the_last_byte_of_the_ciphertext_fails_the_signature() {
    let mut t = parts();
    *t.ct.last_mut().unwrap() ^= 0x80;
    assert_eq!(t.open(), Err(Error::Signature));
}

#[test]
fn a_flipped_bit_in_the_ephemeral_key_fails_the_signature() {
    let mut t = parts();
    t.epk = flipped(&t.epk);
    assert_eq!(t.open(), Err(Error::Signature));
}

#[test]
fn a_flipped_bit_in_the_signature_fails_the_signature() {
    let mut t = parts();
    t.sig = flipped(&t.sig);
    assert_eq!(t.open(), Err(Error::Signature));
}

#[test]
fn a_flipped_bit_in_the_wrap_is_refused_as_a_wrap() {
    let mut t = parts();
    t.k = flipped(&t.k);
    assert_eq!(t.open(), Err(Error::Wrap));
}

#[test]
fn a_flipped_bit_in_the_last_byte_of_the_wrap_is_refused_as_a_wrap() {
    let mut t = parts();
    *t.k.last_mut().unwrap() ^= 0x80;
    assert_eq!(t.open(), Err(Error::Wrap));
}

#[test]
fn another_devices_wrap_is_refused_as_a_wrap() {
    let (alice, _) = registered("af03", 1);
    let (bob, bob_pub) = registered("0012", 2);
    let (carol, carol_pub) = registered("0044", 3);
    let hdr = HDR;
    let sealed = seal(
        &alice,
        hdr,
        BODY,
        &[
            Recipient {
                device: &bob_pub.id,
                dk: bob_pub.dk,
            },
            Recipient {
                device: &carol_pub.id,
                dk: carol_pub.dk,
            },
        ],
        &mut Seeded::new(7),
    )
    .unwrap();
    let envelope = sealed.envelope(hdr);
    let (bobs, carols) = (&sealed.wraps[0].k, &sealed.wraps[1].k);
    assert!(open(&bob, &alice.ik(), &envelope, bobs).is_ok());
    assert!(open(&carol, &alice.ik(), &envelope, carols).is_ok());
    assert_eq!(
        open(&bob, &alice.ik(), &envelope, carols).unwrap_err(),
        Error::Wrap
    );
    assert_eq!(
        open(&carol, &alice.ik(), &envelope, bobs).unwrap_err(),
        Error::Wrap
    );
}

#[test]
fn a_device_that_was_not_a_recipient_cannot_open_the_wrap() {
    let p = pair();
    let stranger = device(77);
    let envelope = p.sealed.envelope(HDR);
    assert_eq!(
        open(&stranger, &p.alice_pub.ik, &envelope, &p.sealed.wraps[0].k).unwrap_err(),
        Error::Wrap
    );
}

#[test]
fn a_header_with_another_conversation_fails_the_signature() {
    let mut t = parts();
    t.hdr = br#"{"v":1,"conv":"dm:0012:ffff","id":"m1"}"#.to_vec();
    assert_eq!(t.open(), Err(Error::Signature));
}

#[test]
fn a_header_of_another_length_fails_the_signature() {
    let mut t = parts();
    t.hdr.push(b' ');
    assert_eq!(t.open(), Err(Error::Signature));
    let mut t = parts();
    t.hdr.pop();
    assert_eq!(t.open(), Err(Error::Signature));
}

#[test]
fn a_header_moved_into_the_nonce_field_fails_the_signature() {
    let mut t = parts();
    let first = t.hdr.remove(0);
    t.n.insert(0, first);
    t.n.pop();
    assert_eq!(t.open(), Err(Error::Signature));
}

#[test]
fn the_identity_key_of_another_device_fails_the_signature() {
    let p = pair();
    let envelope = p.sealed.envelope(HDR);
    let other = registered("af03", 5).1;
    assert_eq!(
        open(&p.bob, &other.ik, &envelope, &p.sealed.wraps[0].k).unwrap_err(),
        Error::Signature
    );
    assert_eq!(
        verify_message(&other.ik, &envelope).unwrap_err(),
        Error::Signature
    );
    assert_eq!(verify_message(&p.alice_pub.ik, &envelope), Ok(()));
}

#[test]
fn a_sender_key_of_small_order_is_refused_as_weak() {
    let p = pair();
    let envelope = p.sealed.envelope(HDR);
    let mut identity = [0u8; 32];
    identity[0] = 1;
    assert_eq!(
        verify_message(&identity, &envelope).unwrap_err(),
        Error::WeakKey
    );
    assert_eq!(
        open(&p.bob, &identity, &envelope, &p.sealed.wraps[0].k).unwrap_err(),
        Error::WeakKey
    );
}

#[test]
fn a_sender_key_that_is_no_point_is_refused_as_a_key() {
    let p = pair();
    let envelope = p.sealed.envelope(HDR);
    let mut not_a_point = [0u8; 32];
    not_a_point[0] = 2;
    assert_eq!(
        verify_message(&not_a_point, &envelope).unwrap_err(),
        Error::Key
    );
}

#[test]
fn a_recipient_key_of_all_zeros_is_refused_at_sealing() {
    let (alice, _) = registered("af03", 1);
    let result = seal(
        &alice,
        HDR,
        BODY,
        &[Recipient {
            device: "0000000000000000",
            dk: [0u8; 32],
        }],
        &mut Seeded::new(7),
    );
    assert_eq!(result.unwrap_err(), Error::NonContributory);
}

#[test]
fn a_recipient_key_of_u_equal_to_one_is_refused_at_sealing() {
    let (alice, _) = registered("af03", 1);
    let mut dk = [0u8; 32];
    dk[0] = 1;
    let result = seal(
        &alice,
        HDR,
        BODY,
        &[Recipient {
            device: "0000000000000000",
            dk,
        }],
        &mut Seeded::new(7),
    );
    assert_eq!(result.unwrap_err(), Error::NonContributory);
}

#[test]
fn one_weak_recipient_among_good_ones_refuses_the_whole_message() {
    let (alice, _) = registered("af03", 1);
    let (_, bob_pub) = registered("0012", 2);
    let result = seal(
        &alice,
        HDR,
        BODY,
        &[
            Recipient {
                device: &bob_pub.id,
                dk: bob_pub.dk,
            },
            Recipient {
                device: "0000000000000000",
                dk: [0u8; 32],
            },
        ],
        &mut Seeded::new(7),
    );
    assert_eq!(result.unwrap_err(), Error::NonContributory);
}

#[test]
fn an_all_zero_ephemeral_key_in_place_of_the_real_one_fails_the_signature() {
    let mut t = parts();
    t.epk = vec![0u8; 32];
    assert_eq!(t.open(), Err(Error::Signature));
}

#[test]
fn a_truncated_wrap_is_refused_by_length() {
    for len in [0, 1, 47] {
        let mut t = parts();
        t.k.truncate(len);
        assert_eq!(t.open(), Err(Error::Length), "k of {len} bytes");
    }
}

#[test]
fn a_wrap_with_a_byte_too_many_is_refused_by_length() {
    let mut t = parts();
    t.k.push(0);
    assert_eq!(t.open(), Err(Error::Length));
}

#[test]
fn a_nonce_of_the_wrong_length_is_refused_by_length() {
    for len in [0, 23, 25] {
        let mut t = parts();
        t.n.resize(len, 0);
        assert_eq!(t.open(), Err(Error::Length), "n of {len} bytes");
    }
}

#[test]
fn an_ephemeral_key_of_the_wrong_length_is_refused_by_length() {
    for len in [0, 31, 33] {
        let mut t = parts();
        t.epk.resize(len, 0);
        assert_eq!(t.open(), Err(Error::Length), "epk of {len} bytes");
    }
}

#[test]
fn a_signature_of_the_wrong_length_is_refused_by_length() {
    for len in [0, 63, 65] {
        let mut t = parts();
        t.sig.resize(len, 0);
        assert_eq!(t.open(), Err(Error::Length), "sig of {len} bytes");
    }
}

#[test]
fn a_ciphertext_shorter_than_its_commitment_and_tag_is_refused_by_length() {
    for len in [0, 1, 15, 16, 31, 32, 47] {
        let mut t = parts();
        t.ct.truncate(len);
        assert_eq!(t.open(), Err(Error::Length), "ct of {len} bytes");
    }
}

#[test]
fn verify_message_refuses_the_same_lengths() {
    let p = pair();
    let short = &p.sealed.n[..23];
    let envelope = Envelope {
        n: short,
        ..p.sealed.envelope(HDR)
    };
    assert_eq!(
        verify_message(&p.alice_pub.ik, &envelope).unwrap_err(),
        Error::Length
    );
}

#[test]
fn the_error_arms_are_distinct() {
    let all = [
        Error::Length,
        Error::Key,
        Error::WeakKey,
        Error::NonContributory,
        Error::Signature,
        Error::Wrap,
        Error::Body,
        Error::Mismatch,
        Error::Record,
        Error::Encoding,
    ];
    for (i, a) in all.iter().enumerate() {
        for b in &all[i + 1..] {
            assert_ne!(a, b);
            assert_ne!(a.to_string(), b.to_string());
        }
    }
}

fn record(name: &str, kind: RestKind) -> (Vec<u8>, [u8; 32]) {
    let key = *device(1).vault_key();
    (
        at_rest_seal(&key, kind, name, b"{\"lines\":[]}", &mut Seeded::new(11)).unwrap(),
        key,
    )
}

#[test]
fn a_record_moved_to_another_file_name_does_not_open() {
    let (record, key) = record("3f9a.hist", RestKind::History);
    assert!(at_rest_open(&key, RestKind::History, "3f9a.hist", &record).is_ok());
    assert_eq!(
        at_rest_open(&key, RestKind::History, "3f9b.hist", &record).unwrap_err(),
        Error::Record
    );
    assert_eq!(
        at_rest_open(&key, RestKind::History, "", &record).unwrap_err(),
        Error::Record
    );
}

#[test]
fn a_record_moved_to_another_kind_does_not_open() {
    let (record, key) = record("state", RestKind::State);
    assert_eq!(
        at_rest_open(&key, RestKind::History, "state", &record).unwrap_err(),
        Error::Record
    );
    let (record, key) = self::record("state", RestKind::History);
    assert_eq!(
        at_rest_open(&key, RestKind::State, "state", &record).unwrap_err(),
        Error::Record
    );
}

#[test]
fn a_record_under_another_vault_key_does_not_open() {
    let (record, _) = record("state", RestKind::State);
    let other = *device(2).vault_key();
    assert_eq!(
        at_rest_open(&other, RestKind::State, "state", &record).unwrap_err(),
        Error::Record
    );
}

#[test]
fn a_flipped_bit_anywhere_in_a_record_does_not_open() {
    let (record, key) = record("state", RestKind::State);
    for at in [0, 23, 24, record.len() - 1] {
        let mut damaged = record.clone();
        damaged[at] ^= 1;
        assert_eq!(
            at_rest_open(&key, RestKind::State, "state", &damaged).unwrap_err(),
            Error::Record,
            "bit flipped at byte {at}"
        );
    }
}

#[test]
fn a_record_shorter_than_a_nonce_and_a_tag_is_refused_by_length() {
    let (record, key) = record("state", RestKind::State);
    for len in [0, 1, 24, 39] {
        assert_eq!(
            at_rest_open(&key, RestKind::State, "state", &record[..len]).unwrap_err(),
            Error::Length,
            "record of {len} bytes"
        );
    }
}

#[test]
fn a_record_with_a_byte_cut_off_its_end_does_not_open() {
    let (record, key) = record("state", RestKind::State);
    assert_eq!(
        at_rest_open(&key, RestKind::State, "state", &record[..record.len() - 1]).unwrap_err(),
        Error::Record
    );
}

#[test]
fn a_ciphertext_of_exactly_a_commitment_and_a_tag_is_not_refused_by_length() {
    let mut t = parts();
    t.ct.truncate(48);
    // Too short to be this message's, but long enough to be looked at: the signature says no.
    assert_eq!(t.open(), Err(Error::Signature));
}

#[test]
fn a_ciphertext_whose_commitment_names_another_key_fails_the_signature() {
    use sha2::{Digest, Sha256};
    let mut t = parts();
    let other = Sha256::new()
        .chain_update(baylee_seal::C_COMMIT)
        .chain_update([0x42u8; 32])
        .finalize();
    assert_ne!(t.ct[..32], other[..]);
    t.ct[..32].copy_from_slice(&other);
    assert_eq!(t.open(), Err(Error::Signature));
}

#[test]
fn a_flipped_bit_in_the_commitment_fails_the_signature() {
    let mut t = parts();
    t.ct[31] ^= 1;
    assert_eq!(t.open(), Err(Error::Signature));
}

#[test]
fn a_non_canonical_sender_key_is_refused_as_a_key() {
    let p = pair();
    let envelope = p.sealed.envelope(HDR);
    let mut p_minus_nothing = [0xffu8; 32];
    p_minus_nothing[0] = 0xed;
    p_minus_nothing[31] = 0x7f; // y = p, which is y = 0
    let mut p_plus_one = p_minus_nothing;
    p_plus_one[0] = 0xee; // y = p + 1, which is y = 1
    for (name, ik) in [
        ("all ones", [0xffu8; 32]),
        ("y = p", p_minus_nothing),
        ("y = p + 1", p_plus_one),
    ] {
        assert_eq!(
            verify_message(&ik, &envelope).unwrap_err(),
            Error::Key,
            "{name}"
        );
        assert_eq!(
            open(&p.bob, &ik, &envelope, &p.sealed.wraps[0].k).unwrap_err(),
            Error::Key,
            "{name}"
        );
    }
}
