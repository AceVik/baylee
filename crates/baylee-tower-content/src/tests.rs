use super::*;

#[test]
fn five_distinct_starters_use_only_tested_cards_and_actual_commander_identity() {
    let mut unavailable = Vec::new();
    for template in starters::ALL {
        for name in std::iter::once(&template.commander).chain(template.spells) {
            if named(name).is_err() {
                unavailable.push(*name);
            }
        }
        assert_eq!(template.spells.len(), 22);
        assert_eq!(
            template
                .lands
                .iter()
                .map(|(_, n)| u16::from(*n))
                .sum::<u16>(),
            17
        );
    }
    assert!(
        unavailable.is_empty(),
        "unavailable or untested cards: {unavailable:?}"
    );
    let content = standard().unwrap();
    validate_catalog(&content.catalog).unwrap();
    assert_eq!(content.starters.len(), 5);
    for collection in &content.starters {
        collection.validate(&content.catalog).unwrap();
        assert_eq!(collection.owned.len(), 23);
        assert!(collection.formation.as_ref().unwrap().sideboard.is_empty());
    }
}

fn seat(collection: &Collection) -> SeatSpec {
    use baylee_core::{card_metadata::DeckChoices, ids::PrintRef, preset::DeckEntry};
    let entry = |card| DeckEntry {
        card,
        print: PrintRef::new(0),
        rule_print: None,
        choices: DeckChoices::default(),
    };
    let forest = named("Forest").unwrap().card;
    let mut seat = baylee_cards::decks::probe_preset(37, forest)
        .unwrap()
        .seats
        .remove(0);
    seat.deck = collection
        .deck
        .iter()
        .flat_map(|row| (0..row.copies).map(move |_| entry(row.card)))
        .collect();
    seat.commanders = vec![entry(collection.formation.as_ref().unwrap().commander)];
    seat.sideboard.clear();
    seat
}

#[test]
fn native_admission_rechecks_colors_singleton_and_eligibility_from_the_registry() {
    let content = standard().unwrap();
    for starter in &content.starters {
        validate_seat(&seat(starter)).unwrap();
    }
    let valid = seat(&content.starters[0]);
    let forest = *valid
        .deck
        .iter()
        .find(|row| row.card == named("Forest").unwrap().card)
        .unwrap();
    let mut invalid = valid.clone();
    invalid.deck[0].card = named("Lightning Bolt").unwrap().card;
    assert!(validate_seat(&invalid).is_err());
    invalid = valid.clone();
    invalid.deck[0].card = invalid.commanders[0].card;
    assert!(validate_seat(&invalid).is_err());
    invalid = valid.clone();
    invalid.commanders[0] = forest;
    assert!(validate_seat(&invalid).is_err());
    invalid = valid.clone();
    invalid.sideboard = vec![forest; 5];
    assert!(validate_seat(&invalid).is_ok());
    invalid.sideboard.push(invalid.sideboard[0]);
    assert!(validate_seat(&invalid).is_err());
    invalid = valid.clone();
    invalid.sideboard.push(invalid.commanders[0]);
    assert!(validate_seat(&invalid).is_err());
    invalid = valid.clone();
    invalid.deck[0].rule_print = Some(baylee_core::ids::RulePrintRef::new(0));
    assert!(validate_seat(&invalid).is_err());
    invalid = valid;
    invalid.deck[0].choices.circled_colors = Some(baylee_core::color::ColorPair::new(
        baylee_core::color::Color::Blue,
        baylee_core::color::Color::Red,
    ));
    assert!(validate_seat(&invalid).is_err());
}

#[test]
fn forged_commander_facts_cannot_admit_an_unreviewed_catalog() {
    let content = standard().unwrap();
    let mut forged = content.catalog.clone();
    let leader = content.starters[0].formation.as_ref().unwrap().commander;
    let row = forged
        .cards
        .iter_mut()
        .find(|row| row.card == leader)
        .unwrap();
    row.run.as_mut().unwrap().identity = baylee_core::color::ColorSet::EMPTY;
    assert!(
        forged.validate().is_ok(),
        "the authority checks structure, not executable rules"
    );
    assert!(validate_catalog(&forged).is_err());
    forged = content.catalog;
    let row = forged
        .cards
        .iter_mut()
        .find(|row| row.card == leader)
        .unwrap();
    row.run.as_mut().unwrap().commander = false;
    assert!(validate_catalog(&forged).is_err());
}
