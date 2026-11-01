use super::*;
use crate::housedeck::HouseDeck;
use baylee_core::color::{Color, ColorPair};
use baylee_core::ids::{CardIndex, PrintRef, RulePrintRef, WatermarkId};
use baylee_core::preset::{AIProfile, GamePreset};

fn preset() -> GamePreset {
    let deck = HouseDeck::named("victory").unwrap();
    crate::selfplay::table_for(42, &[&deck, &deck], &[AIProfile::STEADY; 2], &[])
}

fn entry(card: u32) -> DeckEntry {
    DeckEntry {
        card: CardIndex::new(card),
        print: PrintRef::new(0),
        rule_print: None,
        choices: DeckChoices::default(),
    }
}

fn own(preset: &GamePreset) -> RegisteredDeck7 {
    RegisteredDeck7::from_submission(PlayerId::new(0), &preset.seats[0], &preset.rules).unwrap()
}

fn empty_own(preset: &mut GamePreset) {
    let seat = &mut preset.seats[0];
    seat.deck.clear();
    seat.sideboard.clear();
    seat.commanders.clear();
    seat.starting_hand = None;
    seat.starting_battlefield.clear();
}

#[test]
fn multiplicity_sections_and_card_zero_are_exact() {
    let mut p = preset();
    empty_own(&mut p);
    p.seats[0].deck = vec![entry(0); 300];
    p.seats[0].sideboard = vec![entry(0); 2];
    p.seats[0].commanders = vec![entry(0)];
    p.seats[0].starting_hand = Some(vec![entry(0); 3]);
    p.seats[0].starting_battlefield = vec![entry(0); 4];
    let inventory = own(&p);
    assert_eq!(inventory.cards.len(), 5);
    for (section, count) in [
        (DeckSection7::Library, 300),
        (DeckSection7::Sideboard, 2),
        (DeckSection7::Commander, 1),
        (DeckSection7::StartingHand, 3),
        (DeckSection7::StartingBattlefield, 4),
    ] {
        let row = inventory
            .cards
            .iter()
            .find(|r| r.section == section)
            .unwrap();
        assert_eq!(row.card_id, 0);
        assert_eq!(row.copies, count);
    }
    let bytes = serde_json::to_vec(&inventory).unwrap();
    assert_eq!(
        serde_json::from_slice::<RegisteredDeck7>(&bytes).unwrap(),
        inventory
    );
}

#[test]
fn deck_order_artwork_opponents_and_unreferenced_metadata_do_not_leak() {
    let mut p = preset();
    p.seats[0].deck = vec![entry(0), entry(7), entry(0), entry(9)];
    let initial = own(&p);
    p.seats[0].deck.reverse();
    for entry in &mut p.seats[0].deck {
        entry.print = PrintRef::new(123);
    }
    p.seed = 999_999;
    p.seats[1].deck = vec![entry(4); 500];
    p.seats[1].sideboard = vec![entry(8); 50];
    p.rules
        .insert(CardIndex::new(8), &[Some("opponent-only")], true)
        .unwrap();
    assert_eq!(own(&p), initial);
    p.seats[0].deck[0].card = CardIndex::new(10);
    assert_ne!(
        own(&p),
        initial,
        "own exact card identity must reach inventory"
    );
}

#[test]
fn per_copy_choices_and_physical_rules_are_not_merged() {
    let mut p = preset();
    empty_own(&mut p);
    let mut first = entry(0);
    first.choices.circled_colors = Some(ColorPair::new(Color::White, Color::Blue));
    let mut second = first;
    second.choices.circled_colors = Some(ColorPair::new(Color::Black, Color::Red));
    let mut third = first;
    third.rule_print = Some(
        p.rules
            .insert(first.card, &[Some("azorius")], false)
            .unwrap(),
    );
    let mut fourth = third;
    fourth.rule_print = Some(
        p.rules
            .insert(first.card, &[Some("azorius")], true)
            .unwrap(),
    );
    p.seats[0].deck = vec![first, first, second, third, fourth];
    let inventory = own(&p);
    assert_eq!(inventory.cards.len(), 4);
    assert_eq!(inventory.cards.iter().map(|r| r.copies).sum::<u32>(), 5);
    assert_eq!(inventory.cards.iter().filter(|r| r.copies == 2).count(), 1);
}

#[test]
fn equivalent_resolved_printings_ignore_preset_local_table_indices() {
    let mut p = preset();
    empty_own(&mut p);
    let mut first = entry(0);
    first.rule_print = Some(
        p.rules
            .insert(first.card, &[Some("azorius"), None], false)
            .unwrap(),
    );
    p.seats[0].deck = vec![first];
    let expected = own(&p);
    p.rules = RulesMetadata::default();
    p.rules
        .insert(CardIndex::new(9), &[Some("secret")], true)
        .unwrap();
    p.seats[0].deck[0].rule_print = Some(
        p.rules
            .insert(first.card, &[Some("azorius"), None], false)
            .unwrap(),
    );
    assert_eq!(own(&p), expected);
}

#[test]
fn malformed_referenced_metadata_is_rejected_instead_of_defaulted() {
    let mut p = preset();
    empty_own(&mut p);
    p.seats[0].deck = vec![entry(0)];
    let rejected = |p: &GamePreset| {
        assert!(RegisteredDeck7::from_submission(PlayerId::new(0), &p.seats[0], &p.rules).is_err());
    };
    p.seats[0].deck[0].rule_print = Some(RulePrintRef::new(500));
    rejected(&p);
    let index = p.rules.insert(CardIndex::new(7), &[None], false).unwrap();
    p.seats[0].deck[0].rule_print = Some(index);
    rejected(&p);
    let index = p.rules.insert(CardIndex::new(0), &[None], false).unwrap();
    p.seats[0].deck[0].rule_print = Some(index);
    p.rules.prints[usize::from(index.get())].watermarks[0] = Some(WatermarkId::new(500));
    rejected(&p);
    p.seats[0].deck[0].rule_print = None;
    p.seats[0].deck[0].card = CardIndex::new(crate::carddsl::UNKNOWN_CARD);
    rejected(&p);
    p.seats[0].deck[0].card = CardIndex::new(u32::MAX - 1);
    rejected(&p);
    p.seats[0].deck[0] = entry(0);
    p.seats[0].deck[0].choices =
        serde_json::from_str(r#"{"circled_colors":["Blue","Blue"]}"#).unwrap();
    rejected(&p);
}

#[test]
fn registration_multiset_does_not_confuse_fixed_hand_with_library_cards() {
    let mut p = preset();
    empty_own(&mut p);
    p.seats[0].deck = vec![entry(0); 60];
    let ordinary = own(&p);
    p.seats[0].starting_hand = Some(vec![]);
    assert_eq!(
        own(&p),
        ordinary,
        "setup draw mode is outside this multiset"
    );
    p.seats[0].starting_hand = Some(vec![entry(0); 7]);
    let explicit = own(&p);
    assert_eq!(
        explicit
            .cards
            .iter()
            .find(|r| r.section == DeckSection7::Library)
            .unwrap()
            .copies,
        60
    );
    assert_eq!(
        explicit
            .cards
            .iter()
            .find(|r| r.section == DeckSection7::StartingHand)
            .unwrap()
            .copies,
        7
    );
}

#[test]
fn ledger_distinguishes_ordinary_draw_empty_fixed_hand_and_additional_fixed_hand() {
    let p0 = PlayerId::new(0);

    // 1. Ordinary opening draw (starting_hand = None): 7 drawn from 60 -> 53 left in library.
    let mut ordinary = preset();
    empty_own(&mut ordinary);
    ordinary.seats[0].deck = vec![entry(0); 60];
    let mut ledger_ordinary = InventoryLedger7::from_preset(&ordinary, p0).unwrap();
    let session_ordinary = baylee_gamehost::Session::new(&ordinary).unwrap();
    let view_ordinary = session_ordinary.observe_view(p0);
    ledger_ordinary.observe(&view_ordinary);
    assert_eq!(ledger_ordinary.remaining_copies_in_library(0), 53);
    assert_eq!(ledger_ordinary.total_remaining_library_cards(), 53);

    // 2. Explicitly empty fixed hand (starting_hand = Some([])): 0 drawn -> 60 left in library.
    let mut empty_fixed = ordinary.clone();
    empty_fixed.seats[0].starting_hand = Some(vec![]);
    let mut ledger_empty = InventoryLedger7::from_preset(&empty_fixed, p0).unwrap();
    let session_empty = baylee_gamehost::Session::new(&empty_fixed).unwrap();
    let view_empty = session_empty.observe_view(p0);
    ledger_empty.observe(&view_empty);
    assert_eq!(ledger_empty.remaining_copies_in_library(0), 60);
    assert_eq!(ledger_empty.total_remaining_library_cards(), 60);

    // 3. Explicit additional starting hand (starting_hand = Some(7)): 7 additional in hand -> 60 left in library.
    let mut explicit_fixed = ordinary;
    explicit_fixed.seats[0].starting_hand = Some(vec![entry(0); 7]);
    let mut ledger_explicit = InventoryLedger7::from_preset(&explicit_fixed, p0).unwrap();
    let session_explicit = baylee_gamehost::Session::new(&explicit_fixed).unwrap();
    let view_explicit = session_explicit.observe_view(p0);
    ledger_explicit.observe(&view_explicit);
    assert_eq!(ledger_explicit.remaining_copies_in_library(0), 60);
    assert_eq!(ledger_explicit.total_remaining_library_cards(), 60);
}

#[test]
fn ledger_preserves_counts_across_zone_transitions_and_ignores_tokens() {
    let p0 = PlayerId::new(0);
    let mountain = CardIndex::new(0);
    let shock = CardIndex::new(7);

    let mut p = preset();
    empty_own(&mut p);
    p.seats[0].deck = {
        let mut d = vec![entry(mountain.get()); 40];
        d.extend(std::iter::repeat_n(entry(shock.get()), 20));
        d
    };
    p.seats[0].starting_hand = Some(vec![entry(mountain.get()), entry(shock.get())]);
    p.seats[0].sideboard = vec![entry(shock.get()); 3];

    let mut ledger = InventoryLedger7::from_preset(&p, p0).unwrap();
    let session = baylee_gamehost::Session::new(&p).unwrap();
    let mut view = session.observe_view(p0);
    ledger.observe(&view);
    assert_eq!(ledger.remaining_copies_in_library(mountain.get()), 40);
    assert_eq!(ledger.remaining_copies_in_library(shock.get()), 20);
    assert_eq!(ledger.remaining_sideboard_groups()[0].remaining_copies, 3);

    // Simulate drawing one Mountain from library into hand and injecting a token onto the battlefield.
    let drawn_id = ObjectId::new(99_991, 0);
    view.hand.push(baylee_view::HandObject {
        id: drawn_id,
        card: baylee_view::CardIdentity {
            index: mountain,
            print: PrintRef::new(0),
            face: 0,
        },
        name: "Mountain".into(),
        mana_value: 0,
        colors: baylee_core::color::ColorSet::EMPTY,
        types: baylee_core::types::TypeSet::LAND,
        commander: false,
    });
    if let Some(seat) = view.seats.iter_mut().find(|s| s.player == p0) {
        seat.library_count = 59;
        seat.hand_count = 3;
    }
    ledger.observe(&view);
    assert_eq!(ledger.remaining_copies_in_library(mountain.get()), 39);
    assert_eq!(ledger.remaining_copies_in_library(shock.get()), 20);
    assert_eq!(ledger.total_remaining_library_cards(), 59);

    // Move the drawn Mountain from hand to battlefield, and inject a token with the same card_id.
    view.hand.retain(|h| h.id != drawn_id);
    let mut bf_mountain: baylee_view::PublicObject = serde_json::from_value(serde_json::json!({
        "id": drawn_id,
        "card": { "index": mountain, "print": 0, "face": 0 },
        "rules": { "card": mountain, "face": 0 },
        "name": "Mountain",
        "controller": p0,
        "owner": p0,
        "status": 0,
        "types": baylee_core::types::TypeSet::LAND,
        "supertypes": baylee_core::types::SupertypeSet::BASIC,
        "subtypes": baylee_core::types::SubtypeSet::EMPTY,
        "colors": baylee_core::color::ColorSet::EMPTY,
        "keywords": 0,
        "mana_value": 0,
        "damage": 0,
        "counters": [],
        "targets": [],
        "summoning_sick": false,
        "ability_mana_spending": baylee_core::mana::ManaSpending::EXACT
    }))
    .unwrap();
    bf_mountain.id = drawn_id;
    bf_mountain.owner = p0;
    bf_mountain.controller = p0;
    bf_mountain.status = ObjectStatus::NONE;
    bf_mountain.token = None;
    bf_mountain.card = Some(baylee_view::CardIdentity {
        index: mountain,
        print: PrintRef::new(0),
        face: 0,
    });

    let mut token_copy = bf_mountain.clone();
    token_copy.id = ObjectId::new(99_992, 0);
    token_copy.status = ObjectStatus::TOKEN;

    view.battlefield.push(bf_mountain);
    view.battlefield.push(token_copy);
    ledger.observe(&view);

    assert_eq!(ledger.remaining_copies_in_library(mountain.get()), 39);
    assert_eq!(ledger.remaining_copies_in_library(shock.get()), 20);
}
