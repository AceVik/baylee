use super::*;
use draft::{DeckZone, Draft};

#[test]
fn run_draft_conserves_singletons_between_commander_library_and_sideboard() {
    let (_, cohort, mut run) = super::rewards::fixture();
    run.phase = Phase::Shop;
    run.cards.owned.extend([6, 7].map(|id| Stack {
        card: CardIndex::new(id),
        copies: 1,
    }));
    let mut draft = Draft::default();
    draft.bind(&run);
    let forest = CardIndex::new(4);
    let first = CardIndex::new(6);
    let second = CardIndex::new(7);
    draft.card(&run, &cohort, first, true);
    assert!(
        draft.deck_unchanged(Some(&run)),
        "library has 39 cards already"
    );
    draft.card(&run, &cohort, forest, false);
    draft.card(&run, &cohort, CardIndex::new(1), true);
    draft.card(&run, &cohort, CardIndex::new(8), true);
    assert_eq!(
        draft.deck.len(),
        1,
        "off-color basic and unowned spell refused"
    );
    draft.card(&run, &cohort, first, true);
    draft.card_in(&run, &cohort, first, DeckZone::Sideboard, true);
    assert!(
        draft.formation.as_ref().unwrap().sideboard.is_empty(),
        "same copy cannot be in two zones"
    );
    draft.card_in(&run, &cohort, second, DeckZone::Sideboard, true);
    assert!(draft::allowed(&run, &cohort, &draft.save_deck()));
    draft.commander(&run, &cohort, first);
    assert_eq!(draft.formation.as_ref().unwrap().commander, first);
    assert!(!draft.deck.iter().any(|row| row.card == first));
    assert!(
        !draft::allowed(&run, &cohort, &draft.save_deck()),
        "needs one replacement library card"
    );
    draft.card(&run, &cohort, forest, true);
    assert!(draft::allowed(&run, &cohort, &draft.save_deck()));
    assert!(matches!(draft.save_deck(), Command::Refit { .. }));
    assert!(!draft.deck_unchanged(Some(&run)));
    draft.reset_deck(&run);
    assert!(draft.deck_unchanged(Some(&run)));
}

#[test]
fn sideboard_limit_and_revision_binding_include_the_entire_formation() {
    let (_, cohort, mut run) = super::rewards::fixture();
    run.phase = Phase::Shop;
    let mut draft = Draft::default();
    draft.bind(&run);
    for _ in 0..8 {
        draft.card_in(&run, &cohort, CardIndex::new(4), DeckZone::Sideboard, true);
    }
    assert_eq!(draft.formation.as_ref().unwrap().sideboard[0].copies, 5);
    assert!(!draft.deck_unchanged(Some(&run)));
    draft.bind(&run);
    assert_eq!(
        draft.formation.as_ref().unwrap().sideboard[0].copies,
        5,
        "refresh retains edits"
    );
    run.revision += 1;
    draft.bind(&run);
    assert!(draft.deck_unchanged(Some(&run)));
    assert!(draft.formation.as_ref().unwrap().sideboard.is_empty());
}
