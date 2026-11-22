use super::*;

#[test]
fn draft_limits_apply_before_allocation_of_storage_objects() {
    let mut draft = DeckDraft {
        name: "First".into(),
        ..DeckDraft::default()
    };
    assert!(draft.validate().is_ok()); // An unfinished deck can be kept privately.
    draft.name.push('\u{202e}');
    assert!(draft.validate().is_err());
    draft.name = "First".into();
    draft.cards = vec!["a".repeat(4096); 33];
    assert!(draft.validate().is_err());
    draft.cards = vec!["1 Island".into()];
    draft.cosmetic_origin = Some("https://user@invalid.example".into());
    assert!(draft.validate().is_err());
    draft.cosmetic_origin = Some("https://deck.example".into());
    assert!(draft.validate().is_ok());
}

#[test]
fn library_pointer_rejects_truncated_duplicated_and_unbounded_chunks() {
    use super::manifest::{Blob, Chunk};
    let first = Chunk {
        id: Uuid::now_v7(),
        revision: 1,
        bytes: 256 * 1024,
        digest: [0; 32],
    };
    let last = Chunk {
        id: Uuid::now_v7(),
        bytes: 3,
        ..first.clone()
    };
    let mut pointer = LibraryRef {
        generation: 1,
        blob: Blob {
            bytes: 256 * 1024 + 3,
            digest: [0; 32],
            chunks: vec![first.clone(), last],
        },
    };
    assert!(pointer.validate().is_ok());
    pointer.blob.chunks[1].id = first.id;
    assert!(pointer.validate().is_err());
    pointer.blob.chunks.pop();
    assert!(pointer.validate().is_err());
    pointer.blob.bytes = 8 * 1024 * 1024 + 1;
    assert!(pointer.validate().is_err());
}
