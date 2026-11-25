//! What a game is built from: the decks at the table, and what keeps one
//! from being built.

use super::*;
use crate::blocks::{Block, Blocks};
use crate::lobby::{LobbySeat, SeatKind};
use baylee_core::preset::SeatController;

fn deck(cards: &[&str], commanders: &[&str]) -> Deck {
    Deck {
        id: "deck".into(),
        account_id: "someone".into(),
        kind: "account".into(),
        name: "Deck".into(),
        format: "freeform".into(),
        description: None,
        origin: None,
        version: 1,
        cards: cards.iter().map(|c| (*c).to_owned()).collect(),
        sideboard: vec![],
        commanders: commanders.iter().map(|c| (*c).to_owned()).collect(),
        sleeve: None,
        playmat: None,
        updated_at: 0,
        offered: true,
    }
}

fn index(name: &str) -> baylee_core::ids::CardIndex {
    baylee_cards::decks::by_name(name).expect(name)
}

fn chair(seat: usize, kind: SeatKind, deck: Option<Deck>) -> LobbySeat {
    LobbySeat {
        kind,
        deck,
        account_id: (kind == SeatKind::Human).then(|| format!("account-{seat}")),
        ..LobbySeat::open(seat)
    }
}

fn status<T>(answer: Result<T, (StatusCode, Json<ErrorBody>)>) -> Option<StatusCode> {
    answer.err().map(|(status, _)| status)
}

#[test]
fn a_commander_among_the_rows_is_moved_out_of_the_library_with_its_printing() {
    let id = "b2c6aa39-2d2a-459c-a555-fb48ba993373";
    let row = format!("1 Island scryfall={id}");
    let loaded = loaded_deck(&deck(&["3 Forest", &row, "1 Island"], &["Island"]))
        .ok()
        .expect("a deck");
    assert_eq!(loaded.main.len(), 4, "three Forests and the other Island");
    assert_eq!(loaded.commanders.len(), 1);
    assert_eq!(loaded.commanders[0].index, index("Island"));
    let printed = baylee_core::deckrow::parse(&row).unwrap().print;
    assert_eq!(
        loaded.commanders[0],
        baylee_cards::decks::DeckCard::chosen(index("Island"), &printed)
    );
    assert!(loaded.main.iter().all(|card| *card != loaded.commanders[0]));
}

#[test]
fn a_commander_in_no_row_is_added_and_two_of_them_leave_one_row_each() {
    let outside = loaded_deck(&deck(&["3 Forest"], &["Island"])).ok().unwrap();
    assert_eq!((outside.main.len(), outside.commanders.len()), (3, 1));
    let two = loaded_deck(&deck(&["1 Island", "1 Forest"], &["Forest", "Island"]))
        .ok()
        .unwrap();
    assert!(two.main.is_empty());
    let leaders: Vec<_> = two.commanders.iter().map(|card| card.index).collect();
    assert_eq!(leaders, [index("Forest"), index("Island")]);
}

#[test]
fn a_stored_deck_that_no_longer_reads_is_refused_and_not_dealt_short() {
    for bad in ["251 Forest", "1 No Such Card", "Forest", "0 Forest"] {
        assert_eq!(
            status(loaded_deck(&deck(&["1 Forest", bad], &[]))),
            Some(StatusCode::BAD_REQUEST),
            "{bad}"
        );
    }
    let mut side = deck(&["1 Forest"], &[]);
    side.sideboard = vec!["1 No Such Card".into()];
    assert_eq!(status(loaded_deck(&side)), Some(StatusCode::BAD_REQUEST));
}

#[test]
fn a_room_seats_its_people_and_its_house_as_the_lobby_has_them() {
    let blocks = Blocks::default();
    let mut house = chair(1, SeatKind::Ai, None);
    house.team = Some(2);
    let mut person = chair(0, SeatKind::Human, Some(deck(&["60 Forest"], &[])));
    person.team = Some(1);
    let preset = room_preset(&blocks, &[person, house], 7)
        .ok()
        .expect("a game");
    assert_eq!(preset.seed, 7);
    assert!(matches!(preset.seats[0].controller, SeatController::Open));
    assert!(matches!(preset.seats[1].controller, SeatController::Ai(_)));
    assert_eq!(
        (preset.seats[0].team, preset.seats[1].team),
        (Some(1), Some(2))
    );
    assert!(
        preset.seats[0]
            .deck
            .iter()
            .all(|entry| entry.card == index("Forest"))
    );
    assert!(!preset.seats[1].deck.is_empty(), "the house brings its own");
    // Nothing in what the engine is told names an account.
    let told = serde_json::to_string(&preset).unwrap();
    assert!(!told.contains("account-0"));
}

#[test]
fn a_room_with_one_side_is_no_game() {
    let seats = [0, 1].map(|seat| {
        let mut chair = chair(seat, SeatKind::Human, Some(deck(&["60 Forest"], &[])));
        chair.team = Some(1);
        chair
    });
    assert_eq!(
        status(room_preset(&Blocks::default(), &seats, 1)),
        Some(StatusCode::CONFLICT)
    );
}

#[test]
fn a_card_blocked_after_the_deck_was_chosen_keeps_the_game_from_starting() {
    let now = time::OffsetDateTime::now_utc();
    let blocks = Blocks::default();
    let seats = |cards: &[&str], commanders: &[&str]| {
        [
            chair(0, SeatKind::Human, Some(deck(&["60 Island"], &[]))),
            chair(1, SeatKind::Human, Some(deck(cards, commanders))),
        ]
    };
    let block = |until| Block {
        index: index("Forest").get(),
        name: "Forest".into(),
        reason: "a fault".into(),
        until,
    };
    let refused = |blocks: &Blocks, cards: &[&str], commanders: &[&str]| {
        room_preset(blocks, &seats(cards, commanders), 1).is_err()
    };
    assert!(!refused(&blocks, &["60 Forest"], &[]));
    blocks.set(vec![block(None)]);
    assert!(refused(&blocks, &["59 Island", "1 Forest"], &[]));
    assert!(
        refused(&blocks, &["60 Island"], &["Forest"]),
        "a commander too"
    );
    assert!(!refused(&blocks, &["60 Island"], &[]));
    blocks.set(vec![block(Some(now - time::Duration::seconds(1)))]);
    assert!(!refused(&blocks, &["60 Forest"], &[]), "its end has passed");
    blocks.set(vec![block(Some(now + time::Duration::hours(1)))]);
    assert!(refused(&blocks, &["60 Forest"], &[]));
}

#[test]
fn every_printing_at_a_table_is_named_once() {
    let mut preset = room_preset(
        &Blocks::default(),
        &[
            chair(0, SeatKind::Human, Some(deck(&["60 Forest"], &[]))),
            chair(
                1,
                SeatKind::Human,
                Some(deck(&["60 Forest", "1 Island"], &[])),
            ),
        ],
        1,
    )
    .ok()
    .unwrap();
    let prints = preset.prints.clone();
    preset.prints.extend(prints.clone());
    let named = table_prints(&preset);
    let mut once: Vec<String> = prints.iter().map(|p| p.scryfall_id.to_string()).collect();
    once.sort();
    once.dedup();
    assert_eq!(named, once);
    assert!(named.len() >= 2);
}
