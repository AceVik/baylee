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
