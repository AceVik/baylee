//! What a hostile client may put into a sealed message, and one socket's
//! deliveries. The database's side of a send is `tests/e2e_social_*.rs`.

use super::*;
use baylee_protocol::social::Wrap;

const DEVICE: &str = "0123456789abcdef";
const OTHER: &str = "fedcba9876543210";

fn text(len: usize) -> String {
    URL_SAFE_NO_PAD.encode(vec![0xA5; len])
}

fn header() -> Header {
    Header {
        v: HEADER_VERSION,
        id: uuid::Uuid::now_v7().to_string(),
        conv: "global".into(),
        from: "af03".into(),
        dev: DEVICE.into(),
        ts: 1,
        kind: "text".into(),
    }
}

fn header_text(padding: usize) -> String {
    let mut json = serde_json::to_vec(&header()).unwrap();
    json.resize(json.len() + padding, b' ');
    URL_SAFE_NO_PAD.encode(json)
}

fn wrap(d: &str) -> Wrap {
    Wrap {
        d: d.into(),
        k: text(wire::WRAP_BYTES),
    }
}

fn sealed() -> Sealed {
    Sealed {
        hdr: header_text(0),
        n: text(24),
        ct: text(65),
        epk: text(32),
        sig: text(64),
        wraps: vec![wrap(DEVICE), wrap(OTHER)],
    }
}

/// [`sealed`] after `change`, as [`shapes`] refuses it.
fn fault(change: impl FnOnce(&mut Sealed)) -> Option<Why> {
    let mut message = sealed();
    change(&mut message);
    shapes(&message).err()
}

#[test]
fn a_well_shaped_message_gives_its_header_and_every_wrap_in_order() {
    let (read, wraps) = shapes(&sealed()).expect("well shaped");
    assert_eq!(read.dev, DEVICE);
    assert_eq!(read.from, "af03");
    assert_eq!(read.kind, "text");
    let devices: Vec<_> = wraps.iter().map(|(d, _)| d.as_str()).collect();
    assert_eq!(devices, [DEVICE, OTHER]);
    assert!(
        wraps
            .iter()
            .all(|(_, k)| k == &vec![0xA5; wire::WRAP_BYTES])
    );
}

#[test]
fn a_field_one_byte_short_or_long_of_its_key_is_refused() {
    type Field = fn(&mut Sealed) -> &mut String;
    let fields: [(Field, usize); 4] = [
        (|s| &mut s.n, 24),
        (|s| &mut s.epk, 32),
        (|s| &mut s.sig, 64),
        (|s| &mut s.wraps[1].k, wire::WRAP_BYTES),
    ];
    for (field, len) in fields {
        for wrong in [0, len - 1, len + 1, len * 8] {
            assert_eq!(
                fault(|s| *field(s) = text(wrong)),
                Some(Why::BadSealed),
                "{wrong} for {len}"
            );
        }
        assert_eq!(fault(|s| *field(s) = text(len)), None, "{len}");
    }
}

#[test]
fn only_unpadded_base64url_is_read() {
    // 32 bytes need one `=` in padded base64, and 0xFB 0xFF give `+` and
    // `/` in the standard alphabet.
    for bad in [
        format!("{}=", text(32)),
        format!("{}\n", text(32)),
        format!(" {}", text(32)),
        base64::engine::general_purpose::STANDARD_NO_PAD.encode([0xFB, 0xFF].repeat(16)),
        "é".repeat(16),
        "\0".repeat(43),
    ] {
        assert_eq!(
            fault(|s| s.epk.clone_from(&bad)),
            Some(Why::BadSealed),
            "{bad:?}"
        );
    }
}

#[test]
fn a_body_holds_its_tag_and_at_most_the_bound() {
    assert_eq!(fault(|s| s.ct = text(15)), Some(Why::BadSealed));
    assert_eq!(fault(|s| s.ct = text(16)), None);
    assert_eq!(fault(|s| s.ct = text(wire::MAX_BODY_BYTES + 16)), None);
    assert!(fault(|s| s.ct = text(wire::MAX_BODY_BYTES + 17)).is_some());
    assert_eq!(
        fault(|s| s.ct = text(wire::MAX_BODY_BYTES * 2)),
        Some(Why::TooLarge)
    );
}

#[test]
fn a_header_is_json_of_at_most_its_bound_with_every_field() {
    let bare = serde_json::to_vec(&header()).unwrap().len();
    assert_eq!(
        fault(|s| s.hdr = header_text(wire::MAX_HEADER_BYTES - bare)),
        None
    );
    assert!(fault(|s| s.hdr = header_text(wire::MAX_HEADER_BYTES - bare + 1)).is_some());
    assert_eq!(
        fault(|s| s.hdr = header_text(wire::MAX_HEADER_BYTES)),
        Some(Why::TooLarge)
    );
    let mut whole = serde_json::to_value(header()).unwrap();
    for field in ["v", "id", "conv", "from", "dev", "ts", "kind"] {
        let kept = whole.as_object_mut().unwrap().remove(field).unwrap();
        let json = serde_json::to_vec(&whole).unwrap();
        assert_eq!(
            fault(|s| s.hdr = URL_SAFE_NO_PAD.encode(json)),
            Some(Why::BadSealed),
            "without {field}"
        );
        whole[field] = kept;
    }
    for bad in ["", "[]", "null", "{\"v\":-1}"] {
        assert_eq!(
            fault(|s| s.hdr = URL_SAFE_NO_PAD.encode(bad)),
            Some(Why::BadSealed),
            "{bad:?}"
        );
    }
    // A second document behind the first is not a header either.
    let doubled = [serde_json::to_vec(&header()).unwrap(), b"{}".to_vec()].concat();
    assert_eq!(
        fault(|s| s.hdr = URL_SAFE_NO_PAD.encode(doubled)),
        Some(Why::BadSealed)
    );
}

#[test]
fn a_device_is_wrapped_for_once_and_named_as_the_gateway_names_it() {
    assert_eq!(
        fault(|s| s.wraps[1].d = DEVICE.into()),
        Some(Why::BadSealed),
        "the same device twice"
    );
    for bad in [
        "",
        "0123456789ABCDEF",
        "0123456789abcde",
        "0123456789abcdef0",
        "0123456789abcdeg",
        "0123456789abcd é",
    ] {
        assert_eq!(
            fault(|s| s.wraps[1].d = bad.into()),
            Some(Why::BadSealed),
            "{bad:?}"
        );
    }
}

#[test]
fn a_message_wraps_for_a_full_group_and_not_for_one_device_more() {
    let full = wire::MAX_GROUP_MEMBERS * wire::MAX_DEVICES;
    let wraps = |n: usize| (0..n).map(|d| wrap(&format!("{d:016x}"))).collect();
    assert_eq!(fault(|s| s.wraps = wraps(full)), None);
    assert_eq!(fault(|s| s.wraps = wraps(full + 1)), Some(Why::TooLarge));
}

#[test]
fn a_message_id_is_a_version_seven_uuid_in_its_one_spelling() {
    let id = uuid::Uuid::from_u128(0x0190_b7c4_9a3e_7d1f_8abc_0123_4567_89ab);
    assert_eq!(uuid_v7(&id.to_string()), Some(id));
    for other in [
        id.to_string().to_uppercase(),
        id.simple().to_string(),
        id.braced().to_string(),
        id.urn().to_string(),
        format!(" {id}"),
        "f47ac10b-58cc-4372-a567-0e02b2c3d479".into(),
        uuid::Uuid::nil().to_string(),
        uuid::Uuid::max().to_string(),
        String::new(),
    ] {
        // Two spellings of one id would be two messages to the duplicate check.
        assert_eq!(uuid_v7(&other), None, "{other}");
    }
}

#[test]
fn a_window_sends_each_slot_once_and_never_more_than_its_size_unacked() {
    let mut window = Window::default();
    let size = i64::try_from(wire::DELIVERY_WINDOW).unwrap();
    for seq in 1..=size {
        assert!(window.take(seq), "{seq}");
        assert!(!window.take(seq), "{seq} again");
    }
    assert!(!window.behind, "a repeat leaves nothing waiting");
    assert!(!window.take(size + 1));
    assert!(window.behind, "the slot waits in the database");
    assert_eq!(window.acked(1), [1]);
    assert!(window.take(size + 1));
    assert!(!window.take(size + 2));
}

#[test]
fn an_ack_covers_only_slots_that_were_sent() {
    let mut window = Window::default();
    for seq in [9, 3, 5] {
        assert!(window.take(seq));
    }
    // 4 was stored and never sent on this socket: no ack may delete it.
    assert_eq!(window.acked(5), [3, 5]);
    assert_eq!(window.acked(5), [] as [i64; 0], "an ack again");
    assert_eq!(window.acked(0), [] as [i64; 0]);
    assert_eq!(window.acked(u64::MAX), [9], "past what a slot may be");
    assert!(window.inflight.is_empty());
}

fn pending(seq: i64, kept: &[u8]) -> baylee_db::social::Pending {
    baylee_db::social::Pending {
        seq,
        conv: "global".into(),
        sender: baylee_db::social::Who {
            id: uuid::Uuid::from_u128(0xDEAD_BEEF_0000_1111_2222_3333_4444_5555),
            tag: 0xAF03,
            name: "Alice".into(),
            guest: true,
        },
        sender_dev: DEVICE.into(),
        sealed: kept.to_vec(),
        wrap: vec![0xFB, 0xFF, 0x01],
        at: time::OffsetDateTime::from_unix_timestamp_nanos(1_234_567_891).unwrap(),
    }
}

fn kept() -> Vec<u8> {
    let message = sealed();
    serde_json::to_vec(&Kept {
        hdr: message.hdr,
        n: message.n,
        ct: message.ct,
        epk: message.epk,
        sig: message.sig,
    })
    .unwrap()
}

#[test]
fn a_delivery_carries_this_devices_wrap_and_no_account_id() {
    let slot = pending(7, &kept());
    let frame = delivery(&slot).expect("a slot as it was stored");
    let json = serde_json::to_string(&frame).unwrap();
    let Down::Msg {
        seq,
        at,
        conv,
        from,
        sealed: delivered,
    } = frame
    else {
        panic!("a message");
    };
    assert_eq!((seq, at, conv.as_str()), (7, 1234, "global"));
    assert_eq!(from.device, DEVICE);
    assert_eq!(from.handle, crate::handle::handle("Alice", 0xAF03));
    assert!(from.guest);
    // Base64url: 0xFB 0xFF 0x01 has both letters the alphabets differ in.
    assert_eq!(delivered.k, "-_8B");
    let sent = sealed();
    assert_eq!(
        (delivered.n, delivered.ct, delivered.epk, delivered.sig),
        (sent.n, sent.ct, sent.epk, sent.sig),
        "every part goes where it came from"
    );
    for spelling in [
        slot.sender.id.to_string(),
        slot.sender.id.simple().to_string(),
    ] {
        assert!(!json.contains(&spelling), "{json}");
    }
}

#[test]
fn a_slot_that_cannot_be_read_is_no_delivery() {
    for bad in [&b""[..], b"{}", b"\xff", b"{\"hdr\":1}"] {
        assert!(delivery(&pending(7, bad)).is_none(), "{bad:?}");
    }
    assert!(delivery(&pending(-1, &kept())).is_none());
}
