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
