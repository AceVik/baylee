//! What the private deck library may claim about itself once it is open: a
//! root, a head for every deck, and a log that agrees with its head.

use crate::decks::{
    Blob, Chunk, DeckDraft, DeckSave, DeckSource, DeckSummary, Head, Log, Revision, Root,
};
use uuid::Uuid;

fn blob() -> Blob {
    Blob {
        bytes: 1,
        digest: [0; 32],
        chunks: vec![Chunk {
            id: Uuid::now_v7(),
            revision: 1,
            bytes: 1,
            digest: [0; 32],
        }],
    }
}

fn source(deck: &str) -> DeckSource {
    DeckSource {
        gateway: Some("https://gateway.example".into()),
        account: Some("account".into()),
        deck: deck.into(),
        revision: 1,
    }
}

fn head(id: Uuid) -> Head {
    Head {
        summary: DeckSummary {
            id,
            revision: 3,
            name: "Deck".into(),
            format: "modern".into(),
            cards: 60,
            sideboard: 15,
            copies: 60,
            side_copies: 15,
            commanders: Vec::new(),
            updated_at: 0,
        },
        source: None,
        log: blob(),
    }
}

/// Whether a root holding one deck stands after `change`.
fn with(change: impl FnOnce(&mut Root, Uuid)) -> bool {
    let id = Uuid::now_v7();
    let mut root = Root::empty();
    root.generation = 5;
    root.heads.insert(id, head(id));
    change(&mut root, id);
    root.validate(5).is_ok()
}

/// Whether a root stands after `change` to its one deck's head.
fn with_head(change: impl FnOnce(&mut Head)) -> bool {
    with(|root, id| change(root.heads.get_mut(&id).unwrap()))
}

#[test]
fn a_root_is_of_the_generation_its_pointer_names() {
    assert!(with(|_, _| ()));
    assert!(!with(|root, _| root.generation = 6));
    assert!(!with(|root, _| root.generation = 4));
    assert!(!with(|root, _| root.kind = "baylee/deck-library/2".into()));
    assert!(Root::empty().validate(0).is_ok());
}

#[test]
fn a_deck_is_kept_under_its_own_identifier_and_is_not_also_removed() {
    assert!(!with(|root, id| {
        let head = root.heads.remove(&id).unwrap();
        root.heads.insert(Uuid::now_v7(), head);
    }));
    assert!(!with(|root, _| {
        root.heads.insert(Uuid::nil(), head(Uuid::nil()));
    }));
    assert!(!with(|root, id| {
        root.removed.insert(id, (1, None));
    }));
    let removed = |id, revision| {
        with(|root, _| {
            root.removed.insert(id, (revision, None));
        })
    };
    assert!(removed(Uuid::now_v7(), 1));
    assert!(!removed(Uuid::now_v7(), 0));
    assert!(!removed(Uuid::nil(), 1));
}

#[test]
fn every_figure_of_a_head_is_bounded() {
    assert!(with_head(|h| h.summary.name = "x".repeat(64)));
    assert!(!with_head(|h| h.summary.name = "x".repeat(65)));
    assert!(!with_head(|h| h.summary.name = " ".into()));
    assert!(!with_head(|h| h.summary.name = "a\u{202e}b".into()));
    assert!(with_head(|h| h.summary.format = "x".repeat(64)));
    assert!(!with_head(|h| h.summary.format = "x".repeat(65)));
    assert!(!with_head(|h| h.summary.revision = 0));
    assert!(with_head(
        |h| h.summary.commanders = vec!["x".repeat(512); 2]
    ));
    assert!(!with_head(|h| h.summary.commanders = vec!["x".repeat(513)]));
    assert!(!with_head(|h| h.summary.commanders = vec!["x".into(); 3]));
    for (count, taken) in [(250, true), (251, false)] {
        assert_eq!(with_head(|h| h.summary.cards = count), taken);
        assert_eq!(with_head(|h| h.summary.sideboard = count), taken);
        let copies = u32::try_from(count).unwrap();
        assert_eq!(with_head(|h| h.summary.copies = copies), taken);
        assert_eq!(with_head(|h| h.summary.side_copies = copies), taken);
    }
    assert!(!with_head(|h| h.log.bytes = 0));
}

#[test]
fn one_deck_of_one_account_is_imported_once() {
    assert!(with_head(|h| h.source = Some(source("deck"))));
    assert!(with(|root, id| {
        root.heads.get_mut(&id).unwrap().source = Some(source("deck"));
        root.removed
            .insert(Uuid::now_v7(), (1, Some(source("another"))));
    }));
    assert!(!with(|root, id| {
        root.heads.get_mut(&id).unwrap().source = Some(source("deck"));
        root.removed
            .insert(Uuid::now_v7(), (1, Some(source("deck"))));
    }));
}

#[test]
fn an_import_names_a_real_gateway_an_account_and_a_deck() {
    let takes = |change: fn(&mut DeckSource)| {
        let mut source = source("deck");
        change(&mut source);
        with_head(|h| h.source = Some(source))
    };
    assert!(takes(|s| s.gateway = None));
    assert!(takes(|s| s.gateway = Some("http://127.0.0.1:9".into())));
    assert!(!takes(|s| s.gateway = Some("http://gateway.example".into())));
    assert!(!takes(|s| s.account = None));
    assert!(!takes(|s| s.account = Some(String::new())));
    assert!(takes(|s| s.account = Some("x".repeat(128))));
    assert!(!takes(|s| s.account = Some("x".repeat(129))));
    assert!(!takes(|s| s.deck = String::new()));
    assert!(takes(|s| s.deck = "x".repeat(128)));
    assert!(!takes(|s| s.deck = "x".repeat(129)));
    assert!(!takes(|s| s.revision = 0));
}

fn revision(revision: u64) -> Revision {
    Revision {
        save: DeckSave {
            revision,
            summary: None,
            cards: 60,
            sideboard: 15,
            superseded_at: 0,
        },
        blob: blob(),
    }
}

/// Whether a log of these revisions stands beside a head at revision 3.
fn log(change: impl FnOnce(&mut Log), revisions: &[u64]) -> bool {
    let id = Uuid::now_v7();
    let mut log = Log::empty(id);
    log.history = revisions.iter().copied().map(revision).collect();
    change(&mut log);
    log.validate(&head(id)).is_ok()
}

#[test]
fn a_log_rises_to_the_revision_its_head_is_at() {
    assert!(log(|_| (), &[1, 2, 3]));
    assert!(log(|_| (), &[3]), "older ones may be gone");
    assert!(log(|_| (), &[1, 3]));
    assert!(!log(|_| (), &[]));
    assert!(!log(|_| (), &[1, 2]));
    assert!(!log(|_| (), &[1, 2, 3, 4]));
    assert!(!log(|_| (), &[2, 1, 3]));
    assert!(!log(|_| (), &[3, 3]));
    assert!(!log(|_| (), &[0, 3]));
}

#[test]
fn a_log_belongs_to_its_deck_and_keeps_bounded_figures() {
    assert!(!log(|l| l.deck = Uuid::now_v7(), &[3]));
    assert!(!log(|l| l.kind = "baylee/deck-history/2".into(), &[3]));
    let figure = |change: fn(&mut DeckSave)| log(|l| change(&mut l.history[0].save), &[3]);
    assert!(figure(|s| s.cards = 250));
    assert!(!figure(|s| s.cards = 251));
    assert!(figure(|s| s.sideboard = 250));
    assert!(!figure(|s| s.sideboard = 251));
    assert!(figure(|s| s.summary = Some("x".repeat(1024))));
    assert!(!figure(|s| s.summary = Some("x".repeat(1025))));
    assert!(!log(|l| l.history[0].blob.bytes = 0, &[3]));
}

#[test]
fn a_chunked_blob_is_cut_where_its_size_says() {
    const FULL: u32 = 256 * 1024;
    let blob = |sizes: &[u32]| Blob {
        bytes: sizes.iter().sum(),
        digest: [0; 32],
        chunks: sizes
            .iter()
            .map(|bytes| Chunk {
                id: Uuid::now_v7(),
                revision: 1,
                bytes: *bytes,
                digest: [0; 32],
            })
            .collect(),
    };
    let takes = |sizes: &[u32]| blob(sizes).validate(8 * 1024 * 1024).is_ok();
    assert!(takes(&[FULL, FULL, 1]));
    assert!(takes(&[FULL]));
    assert!(!takes(&[]));
    assert!(!takes(&[1, FULL]), "only the last is short");
    assert!(!takes(&[FULL - 1, 1]));
    assert!(!takes(&[FULL, 0]));
    assert!(takes(&[FULL; 32]));
    assert!(!takes(&[FULL; 33]));
    let mut other = blob(&[1]);
    other.chunks[0].revision = 2;
    assert!(other.validate(1).is_err());
    let mut other = blob(&[1]);
    other.chunks[0].id = Uuid::nil();
    assert!(other.validate(1).is_err());
}

#[test]
fn every_text_of_a_draft_is_bounded() {
    let takes = |change: &dyn Fn(&mut DeckDraft)| {
        let mut draft = DeckDraft {
            name: "Deck".into(),
            ..DeckDraft::default()
        };
        change(&mut draft);
        draft.validate().is_ok()
    };
    let x = |n: usize| "x".repeat(n);
    assert!(takes(&|d| d.name = x(64)) && !takes(&|d| d.name = x(65)));
    assert!(takes(&|d| d.format = x(64)) && !takes(&|d| d.format = x(65)));
    let description = 16 * 1024;
    assert!(takes(&|d| d.description = x(description)));
    assert!(!takes(&|d| d.description = x(description + 1)));
    assert!(takes(&|d| d.summary = Some(x(1024))));
    assert!(!takes(&|d| d.summary = Some(x(1025))));
    assert!(takes(&|d| d.commanders = vec![x(512); 2]));
    assert!(!takes(&|d| d.commanders = vec![x(1); 3]));
    assert!(!takes(&|d| d.commanders = vec![x(513)]));
    assert!(!takes(&|d| d.commanders = vec![String::new()]));
    assert!(takes(&|d| d.sleeve = Some(x(128))));
    assert!(!takes(&|d| d.sleeve = Some(x(129))));
    assert!(!takes(&|d| d.playmat = Some(String::new())));
    assert!(!takes(&|d| d.copied_from = Some(DeckSource {
        revision: 0,
        ..source("deck")
    })));
}

#[test]
fn the_rows_of_a_draft_are_bounded_one_by_one_and_in_all() {
    let takes = |cards: Vec<String>, sideboard: Vec<String>| {
        let draft = DeckDraft {
            name: "Deck".into(),
            cards,
            sideboard,
            ..DeckDraft::default()
        };
        draft.validate().is_ok()
    };
    let row = |n: usize| "x".repeat(n);
    assert!(takes(vec![row(4096)], vec![row(4096)]));
    assert!(!takes(vec![row(4097)], Vec::new()));
    assert!(!takes(Vec::new(), vec![" ".into()]));
    assert!(takes(vec![row(1); 250], vec![row(1); 250]));
    assert!(!takes(vec![row(1); 251], Vec::new()));
    assert!(!takes(Vec::new(), vec![row(1); 251]));
    // Both zones share 128 KiB: 32 rows of 4 KiB are all of it.
    assert!(takes(vec![row(4096); 16], vec![row(4096); 16]));
    let mut side = vec![row(4096); 16];
    side.push(row(1));
    assert!(!takes(vec![row(4096); 16], side));
}
