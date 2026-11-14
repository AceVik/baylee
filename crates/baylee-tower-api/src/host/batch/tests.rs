use super::*;
use uuid::Uuid;

fn batch() -> Batch {
    Batch {
        lease: Lease {
            reservation: Uuid::now_v7(),
            claim: Uuid::now_v7(),
            fence: 1,
        },
        expected: Prefix::default(),
        chunks: vec![b"first".to_vec(), b" second\n".to_vec()],
    }
}

#[test]
fn framing_preserves_boundaries_and_refuses_every_truncated_or_extended_prefix() {
    let original = batch();
    let encoded = original.encode().unwrap();
    let decoded = Batch::decode(&encoded).unwrap();
    assert_eq!(decoded.lease, original.lease);
    assert_eq!(decoded.expected, original.expected);
    assert_eq!(decoded.chunks, original.chunks);
    for end in 0..encoded.len() {
        assert!(Batch::decode(&encoded[..end]).is_err());
    }
    let mut extended = encoded.clone();
    extended.push(0);
    assert!(Batch::decode(&extended).is_err());
    let mut oversized = encoded;
    oversized[..4].copy_from_slice(&u32::MAX.to_be_bytes());
    assert!(Batch::decode(&oversized).is_err());
    let metadata = serde_json::json!({ "lease": original.lease, "expected": original.expected, "chunks": 2, "outcome": "victory" });
    let mut forged = Vec::new();
    push(&mut forged, &serde_json::to_vec(&metadata).unwrap()).unwrap();
    for chunk in &original.chunks {
        push(&mut forged, chunk).unwrap();
    }
    assert!(Batch::decode(&forged).is_err());
}

#[test]
fn framing_limits_bytes_and_chunks_independently_and_requires_a_complete_operation() {
    let mut invalid = batch();
    invalid.chunks[1].pop();
    assert!(invalid.encode().is_err());
    invalid.chunks = vec![Vec::new()];
    assert!(invalid.encode().is_err());
    invalid.chunks = vec![vec![b'\n'; MAX_CHUNK_BYTES + 1]];
    assert!(invalid.encode().is_err());
    invalid.chunks = vec![b"\n".to_vec(); MAX_BATCH_CHUNKS + 1];
    assert!(invalid.encode().is_err());
    invalid.chunks = vec![vec![b'\n'; MAX_CHUNK_BYTES]; MAX_BATCH_BYTES / MAX_CHUNK_BYTES + 1];
    assert!(invalid.encode().is_err());
    invalid.chunks.pop();
    let encoded = invalid.encode().unwrap();
    assert!(encoded.len() <= MAX_FRAME_BYTES);
    assert_eq!(Batch::decode(&encoded).unwrap().chunks, invalid.chunks);
}
