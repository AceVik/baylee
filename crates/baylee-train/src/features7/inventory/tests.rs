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
