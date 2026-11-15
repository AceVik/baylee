use super::*;

#[test]
fn frozen_run_keeps_all_zones_and_commander_in_the_native_preset() {
    let (catalog, cards) = crate::cards::run_tests::fixture();
    let run = Run::begin_for(&catalog, cards, &[7; 32], generation::RUN).unwrap();
    let mut executor = fixture().2.executor;
    executor.rules = "tower-5".into();
    let cohort = Cohort::for_generation(executor, &catalog, generation::RUN).unwrap();
    let build = Build::freeze(&run, &catalog).unwrap();
    assert_eq!(build.formation, run.cards.formation);
    let opponent = Opponent {
        stage: 1,
        cohort: cohort.clone(),
        origin: Origin::House { archetype: 1 },
        build,
    };
    let package = Package::reserve(&run, 0, &catalog, &cohort, opponent, seeds(), 4096)
        .unwrap()
        .package;
    let preset = package
        .preset(&catalog, &cohort.executor, |_| printing(CardIndex::new(0)))
        .unwrap();
    for seat in &preset.seats {
        assert_eq!(seat.deck.len(), 39);
        assert_eq!(seat.commanders.len(), 1);
        assert_eq!(seat.commanders[0].card, CardIndex::new(5));
        assert_eq!(seat.sideboard.len(), 1);
        assert_eq!(seat.sideboard[0].card, CardIndex::new(9));
        assert!(
            !seat
                .deck
                .iter()
                .any(|entry| entry.card == CardIndex::new(5))
        );
    }
    crate::v2::loadout::validate_for(&preset, generation::RUN).unwrap();
    let mut changed = preset;
    changed.seats[0].commanders.clear();
    assert!(crate::v2::loadout::validate_for(&changed, generation::RUN).is_err());
}

#[test]
fn catalog_metadata_and_sideboard_are_bound_and_cannot_cross_generations() {
    let (catalog, cards) = crate::cards::run_tests::fixture();
    let run = Run::begin_for(&catalog, cards, &[7; 32], generation::RUN).unwrap();
    let build = Build::freeze(&run, &catalog).unwrap();
    assert!(build.validate_for(&catalog, 3).is_err());
    let before = catalog_digest_for(&catalog, generation::RUN).unwrap();
    let mut changed = catalog.clone();
    changed.cards[9].run.as_mut().unwrap().commander = true;
    assert_ne!(
        before,
        catalog_digest_for(&changed, generation::RUN).unwrap()
    );
    let mut duplicate = build;
    duplicate.formation.as_mut().unwrap().sideboard[0].card = CardIndex::new(6);
    assert!(duplicate.validate(&catalog).is_err());
    assert!(Run::begin_for(&catalog, run.cards, &[7; 32], 3).is_err());
}
