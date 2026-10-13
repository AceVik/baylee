//! No secret reaches a Debug print: neither a device's keys nor an opened message's body.

mod common;

use baylee_seal::testkit::{device, registered};
use baylee_seal::{Error, open};
use common::{HDR, hex, pair};

#[test]
fn debug_of_a_device_names_its_id_and_none_of_its_secrets() {
    let secrets = device(1);
    let text = format!("{secrets:?}");
    assert!(text.contains(secrets.device_id()), "{text}");
    let bytes = secrets.to_bytes();
    for (name, part) in [
        ("ik_seed", &bytes[..32]),
        ("dk_seed", &bytes[32..64]),
        ("vault_key", &bytes[64..]),
    ] {
        assert!(!text.contains(&hex(part)), "{name} as hex");
        assert!(!text.contains(&hex(part).to_uppercase()), "{name} as HEX");
        let decimal = part
            .iter()
            .map(u8::to_string)
            .collect::<Vec<_>>()
            .join(", ");
        assert!(!text.contains(&decimal), "{name} as a decimal list");
        let compact = format!("{part:?}");
        assert!(!text.contains(&compact), "{name} as a Debug slice");
    }
}

#[test]
fn the_alternate_debug_of_a_device_names_none_of_its_secrets_either() {
    let secrets = device(1);
    let text = format!("{secrets:#?}");
    assert!(text.contains(secrets.device_id()));
    let bytes = secrets.to_bytes();
    assert!(!text.contains(&hex(&bytes[..32])));
    assert!(!text.contains(&hex(&bytes[32..64])));
    assert!(!text.contains(&hex(&bytes[64..])));
}

#[test]
fn debug_of_an_opened_message_does_not_contain_its_body_or_key() {
    let p = pair();
    let opened = open(
        &p.bob,
        &p.alice_pub.ik,
        &p.sealed.envelope(HDR),
        &p.sealed.wraps[0].k,
    )
    .unwrap();
    assert_eq!(&opened.body[..], common::BODY);
    let text = format!("{opened:?}");
    let body = std::str::from_utf8(common::BODY).unwrap();
    assert!(!text.contains(body), "{text}");
    assert!(!text.contains(&hex(&opened.mk[..])), "{text}");
    assert!(!text.contains(&hex(common::BODY)), "{text}");
    let decimal = common::BODY
        .iter()
        .map(u8::to_string)
        .collect::<Vec<_>>()
        .join(", ");
    assert!(!text.contains(&decimal), "{text}");
}

#[test]
fn an_error_prints_no_input_bytes() {
    let (_, public) = registered("af03", 1);
    let err = baylee_seal::verify_device("af04", &public.ik, &public.dk, &public.dsig).unwrap_err();
    assert_eq!(err, Error::Signature);
    let text = format!("{err:?} {err}");
    assert!(!text.contains(&hex(&public.ik)));
    assert!(!text.contains(&hex(&public.dsig)));
}
