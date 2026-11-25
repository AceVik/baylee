use super::*;
use crate::host::{Batch, Lease};
use baylee_tower::v2::execution::{MAX_BATCH_BYTES, MAX_CHUNK_BYTES};
use prost::Message as _;

fn lease() -> Lease {
    Lease {
        reservation: Uuid::now_v7(),
        claim: Uuid::now_v7(),
        fence: 3,
    }
}

fn part(index: u8, chunks: u8, expected: Prefix) -> Control {
    Control::Append {
        fence: 3,
        expected,
        chunks,
        index,
    }
}

#[test]
fn multipart_transport_keeps_one_exact_atomic_commit_across_small_frames() {
    let lease = lease();
    let mut assembly = Assembly::new(lease, [7; 32], 64 * 1024 * 1024, Prefix::default()).unwrap();
    let chunks = [b"private header".to_vec(), b" and decision\n".to_vec()];
    let mut head = Prefix::default();
    for (index, chunk) in chunks.iter().enumerate() {
        let envelope = part(index as u8, 2, Prefix::default())
            .envelope(chunk.clone())
            .unwrap();
        let encoded = envelope.encode_to_vec();
        let Msg::TowerFrame(frame) = Envelope::decode(encoded.as_slice()).unwrap().msg.unwrap()
        else {
            panic!()
        };
        let (control, data) = Control::decode(frame).unwrap();
        head = head.extended([7; 32], 64 * 1024 * 1024, chunk).unwrap();
        let batch = assembly.push(&control, data).unwrap();
        assert!(!assembly.can_seal(head));
        if index == 0 {
            assert!(batch.is_none());
        } else {
            let batch = batch.unwrap();
            let roundtrip = Batch::decode(&batch.encode().unwrap()).unwrap();
            assert_eq!(roundtrip.chunks, chunks);
            assert_eq!(roundtrip.lease, lease);
            assert_eq!(roundtrip.expected, Prefix::default());
        }
    }
    assembly
        .acknowledge(Acknowledged {
            fence: 3,
            prefix: head,
        })
        .unwrap();
    assert!(assembly.can_seal(head));
    assert!(
        Assembly::new(lease, [7; 32], 64 * 1024 * 1024, head)
            .unwrap()
            .can_seal(head)
    );
    let final_bytes = b"next\n".to_vec();
    assert!(
        assembly
            .push(&part(0, 1, head), final_bytes.clone())
            .unwrap()
            .is_some()
    );
    assert!(
        assembly
            .acknowledge(Acknowledged {
                fence: 3,
                prefix: head
            })
            .is_err(),
        "prior ACK cannot acknowledge new bytes"
    );
    assert!(!assembly.can_seal(head));
}

#[test]
fn malformed_interleaved_partial_and_oversized_operations_never_become_acknowledged() {
    let fresh = || Assembly::new(lease(), [7; 32], 64 * 1024 * 1024, Prefix::default()).unwrap();
    for control in [
        part(1, 2, Prefix::default()),
        part(0, 0, Prefix::default()),
        part(0, 65, Prefix::default()),
        Control::Append {
            fence: 2,
            expected: Prefix::default(),
            chunks: 1,
            index: 0,
        },
        Control::Read { index: 0 },
    ] {
        let mut assembly = fresh();
        assert!(assembly.push(&control, b"hidden\n".to_vec()).is_err());
        assert!(
            assembly
                .push(&part(0, 1, Prefix::default()), b"hidden\n".to_vec())
                .is_err()
        );
    }
    let mut changed = fresh();
    assert!(
        changed
            .push(&part(0, 2, Prefix::default()), b"one".to_vec())
            .unwrap()
            .is_none()
    );
    assert!(
        changed
            .push(&part(1, 3, Prefix::default()), b"two\n".to_vec())
            .is_err()
    );
    let mut partial = fresh();
    partial
        .push(&part(0, 2, Prefix::default()), b"one".to_vec())
        .unwrap();
    let head = Prefix::default()
        .extended([7; 32], 64 * 1024 * 1024, b"one")
        .unwrap();
    assert!(
        partial
            .acknowledge(Acknowledged {
                fence: 3,
                prefix: head
            })
            .is_err()
    );
    assert!(
        fresh()
            .push(&part(0, 1, Prefix::default()), b"unterminated".to_vec())
            .is_err()
    );
    let mut interleaved = fresh();
    interleaved
        .push(&part(0, 1, Prefix::default()), b"one\n".to_vec())
        .unwrap();
    assert!(
        interleaved
            .push(&part(0, 1, Prefix::default()), b"two\n".to_vec())
            .is_err()
    );
    let mut oversized = fresh();
    for index in 0..(MAX_BATCH_BYTES / MAX_CHUNK_BYTES) {
        assert!(
            oversized
                .push(
                    &part(index as u8, 33, Prefix::default()),
                    vec![b'x'; MAX_CHUNK_BYTES]
                )
                .unwrap()
                .is_none()
        );
    }
    assert!(
        oversized
            .push(&part(32, 33, Prefix::default()), b"\n".to_vec())
            .is_err()
    );
    for (control, data) in [
        (
            br#"{"operation":"read","index":0,"win":true}"#.to_vec(),
            vec![],
        ),
        (br#"{"operation":"read","index":0}"#.to_vec(), vec![1]),
        (br#"{"operation":"recovery","index":0}"#.to_vec(), vec![]),
        (vec![b' '; MAX_CONTROL_BYTES + 1], vec![]),
        (
            br#"{"operation":"recovery","index":0}"#.to_vec(),
            vec![1; MAX_CHUNK_BYTES + 1],
        ),
    ] {
        assert!(Control::decode(TowerFrame { control, data }).is_err());
    }
}

#[test]
fn only_a_chunk_carries_data_and_it_carries_between_one_byte_and_the_limit() {
    let acknowledgement = Acknowledged {
        fence: 3,
        prefix: Prefix::default(),
    };
    for bare in [
        Control::Read { index: 0 },
        Control::Acknowledged { acknowledgement },
        Control::Seal {
            expected: Prefix::default(),
        },
        Control::Sealed { acknowledgement },
        Control::Finished {},
    ] {
        assert!(bare.envelope(Vec::new()).is_ok());
        assert_eq!(bare.envelope(vec![0]).err(), Some(Refusal::Capacity));
    }
    for chunk in [
        Control::Recovery { index: 0 },
        part(0, 1, Prefix::default()),
    ] {
        assert_eq!(chunk.envelope(Vec::new()).err(), Some(Refusal::Capacity));
        assert!(chunk.envelope(vec![b'x'; MAX_CHUNK_BYTES]).is_ok());
        assert_eq!(
            chunk.envelope(vec![b'x'; MAX_CHUNK_BYTES + 1]).err(),
            Some(Refusal::Capacity)
        );
    }
}

#[test]
fn an_append_names_a_live_fence_and_a_part_of_a_bounded_batch() {
    let send = |fence, chunks, index| {
        Control::Append {
            fence,
            expected: Prefix::default(),
            chunks,
            index,
        }
        .envelope(b"x".to_vec())
        .err()
    };
    assert_eq!(send(1, 1, 0), None);
    assert_eq!(send(u64::MAX, 64, 63), None);
    assert_eq!(send(0, 1, 0), Some(Refusal::Invalid));
    assert_eq!(send(1, 0, 0), Some(Refusal::Invalid));
    assert_eq!(send(1, 2, 2), Some(Refusal::Invalid));
    assert_eq!(send(1, 65, 0), Some(Refusal::Invalid));
    assert_eq!(send(1, u8::MAX, u8::MAX), Some(Refusal::Invalid));
}

#[test]
fn a_received_frame_is_judged_by_the_same_rules_as_a_sent_one() {
    let receive = |control: &str, data: &[u8]| {
        Control::decode(TowerFrame {
            control: control.as_bytes().to_vec(),
            data: data.to_vec(),
        })
        .err()
    };
    assert_eq!(receive(r#"{"operation":"read","index":4}"#, b""), None);
    assert_eq!(receive(r#"{"operation":"finished"}"#, b""), None);
    // A data byte smuggled behind a control that has none.
    assert_eq!(
        receive(r#"{"operation":"read","index":4}"#, b"x"),
        Some(Refusal::Capacity)
    );
    assert_eq!(
        receive(r#"{"operation":"recovery","index":4}"#, b""),
        Some(Refusal::Capacity)
    );
    for strange in [
        "",
        "{}",
        "null",
        r#"{"operation":"award","gold":1}"#,
        r#"{"operation":"read"}"#,
        r#"{"operation":"read","index":-1}"#,
        r#"{"operation":"read","index":4,"path":"/bin/sh"}"#,
        r#"{"operation":"finished","winner":0}"#,
        r#"{"operation":"read","index":4}{"operation":"finished"}"#,
    ] {
        assert_eq!(receive(strange, b""), Some(Refusal::Invalid), "{strange}");
    }
    // Size is judged before the text is parsed, so padding is not a way in.
    let padded = format!("{}{}", " ".repeat(MAX_CONTROL_BYTES), "{}");
    assert_eq!(receive(&padded, b""), Some(Refusal::Capacity));
    assert_eq!(
        receive(
            r#"{"operation":"recovery","index":4}"#,
            &vec![b'x'; MAX_CHUNK_BYTES + 1]
        ),
        Some(Refusal::Capacity)
    );
}

/// A setup the engine may start, as the gateway sends it for a first duel.
fn setup() -> Setup {
    use baylee_protocol::execution::{Identity, Variant};
    use baylee_tower::{
        cards::{Card, Collection, Stack},
        v2::{
            package::{Build, Cohort, Origin},
            run::Run,
        },
    };
    let card = |card, price, basic_land| Card {
        card: baylee_tower::cards::CardIndex::new(card),
        price,
        basic_land,
        run: None,
    };
    let stack = |card, copies| Stack {
        card: baylee_tower::cards::CardIndex::new(card),
        copies,
    };
    let catalog = Catalog {
        cards: vec![card(0, 0, true), card(1, 4, false)],
    };
    let starter = Collection {
        formation: None,
        owned: vec![stack(1, 4)],
        deck: vec![stack(0, 36), stack(1, 4)],
    };
    let run = Run::begin_for(&catalog, starter, &[3; 32], 3).unwrap();
    let mut executor = Identity::native(Variant::Tower, "setup-test", "registry-test", 21);
    executor.rules = "tower-4".into();
    let build = Build::freeze(&run, &catalog).unwrap();
    Setup {
        game: Uuid::now_v7(),
        package: Package {
            schema: 2,
            cohort: Cohort::for_generation(executor, &catalog, 3).unwrap(),
            stage: 1,
            opponent: Origin::House { archetype: 1 },
            builds: [build.clone(), build],
            shuffle_seed: 11,
            policy_seed: 19,
        },
        catalog,
        fence: 1,
        prefix: Prefix::default(),
        byte_limit: 1024,
        names: ["Player".to_owned(), "House".to_owned()],
        resources: Resources::default(),
    }
}

fn refused(change: impl FnOnce(&mut Setup)) -> Option<Refusal> {
    let mut changed = setup();
    change(&mut changed);
    changed.validate().err()
}

#[test]
fn a_setup_starts_only_a_game_with_a_live_fence_and_a_version_seven_identity() {
    assert_eq!(refused(|_| {}), None);
    assert_eq!(refused(|s| s.fence = u64::MAX), None);
    assert_eq!(refused(|s| s.fence = 0), Some(Refusal::Invalid));
    for game in [
        Uuid::nil(),
        Uuid::max(),
        Uuid::from_u128(0x0000_0000_0000_4000_8000_0000_0000_0000),
        // Version seven, and not the variant that gives the version its meaning.
        Uuid::from_u128(0x0000_0000_0000_7000_0000_0000_0000_0000),
    ] {
        assert_eq!(refused(|s| s.game = game), Some(Refusal::Invalid), "{game}");
    }
}

#[test]
fn a_setup_names_both_seats_in_bounded_printable_text() {
    for seat in [0, 1] {
        let named = |name: &str| refused(|s| name.clone_into(&mut s.names[seat]));
        assert_eq!(named("Äpfel & Birnen"), None);
        assert_eq!(named(&"n".repeat(160)), None);
        for bad in ["", "a\nb", "a\u{0}b", "a\u{1b}[2Jb", "a\u{85}b"] {
            assert_eq!(named(bad), Some(Refusal::Invalid), "{bad:?}");
        }
        assert_eq!(named(&"n".repeat(161)), Some(Refusal::Invalid));
        // Bytes are bounded, not characters.
        assert_eq!(named(&"ä".repeat(81)), Some(Refusal::Invalid));
    }
}

#[test]
fn a_setup_can_rebuild_every_byte_its_record_may_hold() {
    assert_eq!(refused(|s| s.byte_limit = s.resources.bytes), None);
    assert_eq!(
        refused(|s| s.byte_limit = s.resources.bytes + 1),
        Some(Refusal::Invalid)
    );
    assert_eq!(refused(|s| s.byte_limit = 0), Some(Refusal::Invalid));
    assert!(refused(|s| s.resources.millis = u32::MAX).is_some());
}

#[test]
fn a_setup_resumes_only_a_prefix_its_record_could_have() {
    let chunk = |prefix: Prefix| prefix.extended([7; 32], 1024, b"line\n").unwrap();
    let held = chunk(Prefix::default());
    assert_eq!(refused(|s| s.prefix = held), None);
    assert!(refused(|s| s.prefix.bytes = 1).is_some());
    assert!(refused(|s| s.prefix.digest = [1; 32]).is_some());
    assert!(
        refused(|s| {
            s.prefix = held;
            s.byte_limit = held.bytes - 1;
        })
        .is_some()
    );
}

#[test]
fn a_setup_with_a_package_of_another_schema_or_stage_is_refused() {
    assert!(refused(|s| s.package.schema = 1).is_some());
    assert!(refused(|s| s.package.stage = 0).is_some());
    assert!(refused(|s| s.package.stage = u8::MAX).is_some());
    // The catalog is bound to the cohort: a cheaper card is another catalog.
    assert!(refused(|s| s.catalog.cards[1].price = 2).is_some());
}

#[test]
fn a_start_is_refused_on_the_wire_exactly_when_its_setup_is() {
    let start = |change: fn(&mut Setup)| {
        let mut setup = setup();
        change(&mut setup);
        Control::Start {
            setup: Box::new(setup),
        }
    };
    assert!(start(|_| {}).envelope(Vec::new()).is_ok());
    assert!(start(|s| s.fence = 0).envelope(Vec::new()).is_err());

    // A frame that was not built by `envelope` is judged on arrival.
    let raw = |control: &Control| TowerFrame {
        control: serde_json::to_vec(control).unwrap(),
        data: Vec::new(),
    };
    assert!(Control::decode(raw(&start(|_| {}))).is_ok());
    assert!(Control::decode(raw(&start(|s| s.fence = 0))).is_err());
    assert!(Control::decode(raw(&start(|s| s.names[1].clear()))).is_err());
}
