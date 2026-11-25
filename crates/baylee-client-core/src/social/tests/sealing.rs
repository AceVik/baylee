//! Opening what another player's client sealed: the delivery is hostile until every field has its size, the header says what the gateway attests, and the sender's key signed it (§4.2).

use baylee_protocol::social::{
    Delivered, HEADER_VERSION, Header, MAX_BODY_BYTES, MAX_HEADER_BYTES, Sender,
};
use baylee_seal::b64::url;
use baylee_seal::testkit::Seeded;
use baylee_seal::{DeviceSecrets, Recipient};

use crate::social::convs::Body;
use crate::social::keys::CachedDevice;
use crate::social::sealing::{
    Decoded, KIND_TEXT, OpenFault, SealFault, header_bytes, seal_for, uuid_ms, uuid_v7,
};

const CONV: &str = "dm:0012:af03";
const TS: u64 = 1_800_000_000_000;

fn device(seed: u64) -> DeviceSecrets {
    DeviceSecrets::generate(&mut Seeded::new(seed))
}

fn cached(device: &DeviceSecrets) -> CachedDevice {
    CachedDevice {
        id: device.device_id().into(),
        ik: device.ik(),
        dk: device.dk(),
        label: "macOS".into(),
        added_at: 0,
    }
}

fn sender(device: &DeviceSecrets) -> Sender {
    Sender {
        tag: "af03".into(),
        handle: "Alice#af03".into(),
        device: device.device_id().into(),
        guest: false,
    }
}

fn header(device: &DeviceSecrets) -> Vec<u8> {
    header_bytes("id-1", CONV, "af03", device.device_id(), TS)
}

fn said(text: &str) -> Body {
    Body {
        text: text.into(),
        ..Body::default()
    }
}

/// `plain`, signed by `from` under `hdr`, as the gateway hands it to `to`.
fn deliver(from: &DeviceSecrets, to: &DeviceSecrets, hdr: &[u8], plain: &[u8]) -> Delivered {
    let recipient = Recipient {
        device: to.device_id(),
        dk: to.dk(),
    };
    let sealed = baylee_seal::seal(from, hdr, plain, &[recipient], &mut Seeded::new(9)).unwrap();
    Delivered {
        hdr: url::encode(hdr),
        n: url::encode(&sealed.n),
        ct: url::encode(&sealed.ct),
        epk: url::encode(&sealed.epk),
        sig: url::encode(&sealed.sig),
        k: url::encode(&sealed.wraps[0].k),
    }
}

/// What reading a delivery under `hdr` comes to.
fn read_header(hdr: &[u8]) -> Result<(), OpenFault> {
    let (alice, bob) = (device(1), device(2));
    let delivered = deliver(&alice, &bob, hdr, b"{}");
    Decoded::read(CONV, &sender(&alice), &delivered).map(drop)
}

#[test]
fn every_device_a_message_is_sealed_for_opens_it_and_no_other() {
    let (alice, bob, carol, eve) = (device(1), device(2), device(3), device(4));
    let hdr = header(&alice);
    let (b, c) = (cached(&bob), cached(&carol));
    // Alice's own row is stale on purpose: her real key is used for herself.
    let mut own = cached(&alice);
    own.dk = eve.dk();
    let out = seal_for(
        &alice,
        &hdr,
        &said("hello"),
        &[&b, &c, &own],
        &mut Seeded::new(5),
    )
    .unwrap();
    assert_eq!(out.sealed.wraps.len(), 3, "one wrap each, hers once");
    let open = |me: &DeviceSecrets, wrap: &DeviceSecrets| {
        let wrap = out.sealed.wraps.iter().find(|w| w.d == wrap.device_id());
        let delivered = Delivered {
            hdr: out.sealed.hdr.clone(),
            n: out.sealed.n.clone(),
            ct: out.sealed.ct.clone(),
            epk: out.sealed.epk.clone(),
            sig: out.sealed.sig.clone(),
            k: wrap.unwrap().k.clone(),
        };
        Decoded::read(CONV, &sender(&alice), &delivered)
            .unwrap()
            .open(me, &cached(&alice))
            .map(|opened| opened.body.text)
    };
    for me in [&alice, &bob, &carol] {
        assert_eq!(open(me, me).unwrap(), "hello");
    }
    assert!(open(&eve, &bob).is_err());
    assert!(open(&bob, &carol).is_err(), "another device's wrap");
}

#[test]
fn a_header_must_say_what_the_gateway_attests() {
    let alice = device(1);
    let id = alice.device_id();
    assert_eq!(read_header(&header(&alice)), Ok(()));
    for (conv, from, dev) in [
        ("dm:0012:0c11", "af03", id),
        (CONV, "0012", id),
        (CONV, "af03", device(2).device_id()),
    ] {
        assert_eq!(
            read_header(&header_bytes("id-1", conv, from, dev, TS)),
            Err(OpenFault::Mismatch)
        );
    }
}

#[test]
fn a_header_of_another_version_or_kind_is_not_read() {
    let alice = device(1);
    let written = |change: fn(&mut Header)| {
        let mut header: Header = serde_json::from_slice(&header(&alice)).unwrap();
        change(&mut header);
        read_header(&serde_json::to_vec(&header).unwrap())
    };
    assert_eq!(written(|_| ()), Ok(()));
    assert_eq!(
        written(|h| h.v = HEADER_VERSION + 1),
        Err(OpenFault::Header)
    );
    assert_eq!(
        written(|h| h.kind = format!("{KIND_TEXT}2")),
        Err(OpenFault::Header)
    );
    assert_eq!(read_header(b"not json"), Err(OpenFault::Header));
    assert_eq!(read_header(b""), Err(OpenFault::Header));
}

#[test]
fn a_header_may_be_512_bytes_long_and_not_one_more() {
    let alice = device(1);
    let padded = |length: usize| {
        let mut hdr = header(&alice);
        hdr.resize(length, b' ');
        hdr
    };
    assert_eq!(read_header(&padded(MAX_HEADER_BYTES)), Ok(()));
    assert_eq!(
        read_header(&padded(MAX_HEADER_BYTES + 1)),
        Err(OpenFault::Header)
    );
    let seal = |hdr: &[u8]| seal_for(&alice, hdr, &said("x"), &[], &mut Seeded::new(1)).err();
    assert_eq!(seal(&padded(MAX_HEADER_BYTES)), None);
    assert_eq!(
        seal(&padded(MAX_HEADER_BYTES + 1)),
        Some(SealFault::TooLarge),
        "nor is one sent"
    );
}

/// One field of a delivery.
type Field = fn(&mut Delivered) -> &mut String;

#[test]
fn every_field_of_a_delivery_has_exactly_its_size() {
    let (alice, bob) = (device(1), device(2));
    let good = deliver(&alice, &bob, &header(&alice), b"{}");
    let read = |change: &dyn Fn(&mut Delivered)| {
        let mut delivered = good.clone();
        change(&mut delivered);
        Decoded::read(CONV, &sender(&alice), &delivered).map(drop)
    };
    assert_eq!(read(&|_| ()), Ok(()));
    let fields: [(&str, Field, usize); 4] = [
        ("nonce", |d| &mut d.n, 24),
        ("ephemeral key", |d| &mut d.epk, 32),
        ("signature", |d| &mut d.sig, 64),
        ("wrap", |d| &mut d.k, 48),
    ];
    for (name, field, size) in fields {
        for wrong in [0, size - 1, size + 1, 10_000] {
            assert_eq!(
                read(&|d| *field(d) = url::encode(&vec![7; wrong])),
                Err(OpenFault::Header),
                "{name} of {wrong} bytes"
            );
        }
        assert_eq!(
            read(&|d| field(d).replace_range(..1, "!")),
            Err(OpenFault::Header),
            "{name} in another alphabet"
        );
    }
    let body = |length: usize| read(&|d| d.ct = url::encode(&vec![7; length]));
    assert_eq!(body(MAX_BODY_BYTES + 16), Ok(()));
    assert_eq!(body(MAX_BODY_BYTES + 17), Err(OpenFault::Header));
}

#[test]
fn a_message_signed_with_another_key_does_not_open() {
    let (alice, bob, eve) = (device(1), device(2), device(4));
    // Eve seals it and the gateway, in league with her, says it is Alice's.
    let delivered = deliver(&eve, &bob, &header(&alice), br#"{"text":"pay me"}"#);
    let read = Decoded::read(CONV, &sender(&alice), &delivered).unwrap();
    assert!(matches!(
        read.open(&bob, &cached(&alice)),
        Err(OpenFault::Check(_))
    ));
}

#[test]
fn a_header_swapped_after_signing_does_not_open() {
    let (alice, bob) = (device(1), device(2));
    let mut delivered = deliver(&alice, &bob, &header(&alice), br#"{"text":"hi"}"#);
    let later = header_bytes("id-1", CONV, "af03", alice.device_id(), TS + 1);
    delivered.hdr = url::encode(&later);
    let read = Decoded::read(CONV, &sender(&alice), &delivered).unwrap();
    assert!(matches!(
        read.open(&bob, &cached(&alice)),
        Err(OpenFault::Check(_))
    ));
}

#[test]
fn a_properly_signed_body_that_is_no_message_is_refused() {
    let (alice, bob) = (device(1), device(2));
    let open = |plain: &[u8]| {
        let delivered = deliver(&alice, &bob, &header(&alice), plain);
        Decoded::read(CONV, &sender(&alice), &delivered)
            .unwrap()
            .open(&bob, &cached(&alice))
            .map(|opened| opened.body)
    };
    assert_eq!(open(br#"{"text":"hi"}"#), Ok(said("hi")));
    for plain in [&b""[..], b"hi", b"{}", br#"{"text":7}"#, b"\xff"] {
        assert_eq!(open(plain), Err(OpenFault::Body));
    }
}

#[test]
fn a_body_longer_than_the_limit_is_not_sealed() {
    let alice = device(1);
    let empty = serde_json::to_vec(&said("")).unwrap().len();
    let seal = |letters: usize| {
        let body = said(&"x".repeat(letters));
        seal_for(&alice, &header(&alice), &body, &[], &mut Seeded::new(1)).err()
    };
    assert_eq!(seal(MAX_BODY_BYTES - empty), None);
    assert_eq!(seal(MAX_BODY_BYTES - empty + 1), Some(SealFault::TooLarge));
}

#[test]
fn an_identifier_carries_its_time_and_sorts_by_it() {
    let mut rng = Seeded::new(3);
    let ids: Vec<String> = [0, 1, TS, TS + 1, 0xFFFF_FFFF_FFFF]
        .into_iter()
        .map(|at| {
            let id = uuid_v7(at, &mut rng);
            assert_eq!(uuid_ms(&id), Some(at), "at={at}");
            id
        })
        .collect();
    assert!(ids.is_sorted());
    assert_ne!(uuid_v7(TS, &mut rng), uuid_v7(TS, &mut rng));
}

#[test]
fn only_a_version_seven_identifier_has_a_time() {
    let id = uuid_v7(TS, &mut Seeded::new(3));
    let mut random = id.clone();
    random.replace_range(14..15, "4");
    for (idx, bad) in [random.as_str(), "", "x", &id[1..], &id.replace('-', "")]
        .into_iter()
        .enumerate()
    {
        assert_eq!(uuid_ms(bad), None, "case={idx}");
    }
}
