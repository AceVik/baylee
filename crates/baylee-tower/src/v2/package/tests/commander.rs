use super::*;

#[test]
fn published_print_and_foil_are_frozen_for_both_decks_and_cannot_be_substituted() {
    use crate::cards::{CardRarity, Presentation};
    let (mut catalog, cards) = crate::cards::run_tests::fixture();
    for card in &mut catalog.cards {
        card.run.as_mut().unwrap().presentation = Some(Presentation {
            printing: uuid::Uuid::from_u128(u128::from(card.card.get()) + 1),
            finish: Finish::Foil,
            rarity: CardRarity::Rare,
        });
    }
    let run = Run::begin_for(&catalog, cards, &[7; 32], generation::RUNIC).unwrap();
    let mut executor = fixture().2.executor;
    executor.rules = "tower-6".into();
    executor.ai = "tower-house-23".into();
    let cohort = Cohort::for_generation(executor, &catalog, generation::RUNIC).unwrap();
    let opponent = Opponent {
        stage: 1,
        cohort: cohort.clone(),
        origin: Origin::House { archetype: 1 },
        build: Build::freeze(&run, &catalog).unwrap(),
    };
    let package = Package::reserve(&run, 0, &catalog, &cohort, opponent, seeds(), 4096)
        .unwrap()
        .package;
    let preset = package
        .preset(&catalog, &cohort.executor, |_| {
            panic!("must use the frozen palette")
        })
        .unwrap();
    for seat in &preset.seats {
        for entry in seat
            .deck
            .iter()
            .chain(&seat.commanders)
            .chain(&seat.sideboard)
        {
            let art = catalog
                .card(entry.card)
                .unwrap()
                .run
                .unwrap()
                .presentation
                .unwrap();
            assert_eq!(preset.prints[usize::from(entry.print.get())], art.print());
        }
    }
    catalog.cards[6]
        .run
        .as_mut()
        .unwrap()
        .presentation
        .as_mut()
        .unwrap()
        .finish = Finish::Normal;
    assert!(package.validate(&catalog, &cohort.executor).is_err());
}

#[test]
fn frozen_run_keeps_all_zones_and_commander_in_the_native_preset() {
    let (catalog, cards) = crate::cards::run_tests::fixture();
    let run = Run::begin_for(&catalog, cards, &[7; 32], generation::RUN).unwrap();
    let mut executor = fixture().2.executor;
    executor.rules = "tower-5".into();
    executor.ai = "tower-house-22".into();
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

#[test]
fn expanded_equipment_freezes_into_a_package_without_weakening_retained_admission() {
    let (catalog, cards) = crate::cards::run_tests::fixture();
    let run = Run::begin_for(&catalog, cards, &[7; 32], generation::RUNIC).unwrap();
    let mut build = Build::freeze(&run, &catalog).unwrap();
    build
        .active
        .push(crate::v2::catalog::Definition::from_id(712).unwrap());
    build.validate_for(&catalog, generation::RUNIC).unwrap();
    assert!(
        build
            .validate_for(&catalog, generation::EXPEDITIONS)
            .is_err()
    );
    let mut executor = fixture().2.executor;
    executor.rules = "tower-6".into();
    executor.ai = "tower-house-23".into();
    let cohort = Cohort::for_generation(executor, &catalog, generation::RUNIC).unwrap();
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
    assert_eq!(
        preset.house_rules.rule_pack.unwrap().seats[1].definitions,
        vec![712]
    );
}
