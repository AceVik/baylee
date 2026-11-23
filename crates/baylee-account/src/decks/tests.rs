use super::*;

#[test]
fn advent_appearance_is_private_per_deck_and_preserves_the_legacy_upload() {
    let a = DeckDraft {
        name: "A".into(),
        sleeve: Some("private-upload".into()),
        cosmetic_origin: Some("https://saved.example".into()),
        ..DeckDraft::default()
    };
    let old = serde_json::to_string(&a).unwrap();
    assert!(!old.contains("\"look\""));
    let mut worn: DeckDraft = serde_json::from_str(&old).unwrap();
    worn.look.sleeve = Some(*Uuid::now_v7().as_bytes());
    let b = DeckDraft {
        name: "B".into(),
        cards: worn.cards.clone(),
        ..DeckDraft::default()
    };
    assert_ne!(b.look, worn.look);
    let saved: DeckDraft = serde_json::from_str(&serde_json::to_string(&worn).unwrap()).unwrap();
    assert_eq!(saved.look, worn.look);
    assert_eq!(saved.sleeve, a.sleeve);
    assert_eq!(saved.cosmetic_origin, a.cosmetic_origin);
    worn.look = central_api::advent::Equipment::default();
    assert_eq!(serde_json::to_string(&worn).unwrap(), old);
}

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
