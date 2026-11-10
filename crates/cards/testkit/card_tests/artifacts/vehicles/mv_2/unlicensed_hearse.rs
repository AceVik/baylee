//! `cards/artifacts/vehicles/mv_2/unlicensed_hearse.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Unlicensed Hearse: "{T}: Exile up to two target cards from a single
/// graveyard." and "Unlicensed Hearse's power and toughness are each equal
/// to the number of cards exiled with it."
///
/// A Forest lies in p0's graveyard and a Plains and a Swamp in p1's. Two
/// graveyards hold cards, so the activation asks which one first; p1's is
/// named, and the targets offered are its two cards and not the Forest.
/// Both go to their owner's exile, and the Hearse, 0/0 before, is 2/2.
#[test]
fn unlicensed_hearse_exiles_two_cards_from_one_graveyard_and_counts_them() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[unlicensed_hearse()])
        .hand(0, &[forest()])
        .hand(1, &[plains(), swamp()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");
    let their_cards = [
        hand_to_graveyard(&mut engine, p1, plains()),
        hand_to_graveyard(&mut engine, p1, swamp()),
    ];
    let my_forest = hand_to_graveyard(&mut engine, p0, forest());
    let hearse = on_battlefield(&engine, p0, unlicensed_hearse()).expect("the Hearse is out");
    assert_eq!(pt(&engine, hearse), (0, 0), "nothing is exiled with it yet");

    activate(&mut engine, p0, unlicensed_hearse(), 0);
    let Pending::ChoosePlayer { player, options } = engine.pending().clone() else {
        panic!(
            "expected the graveyard question, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p0);
    assert_eq!(options, vec![p0, p1], "both graveyards hold a card");
    engine
        .apply(p0, PlayerAction::ChoosePlayer(p1))
        .expect("p1's graveyard is named");
    let Pending::ChooseTargets {
        options, min, max, ..
    } = engine.pending().clone()
    else {
        panic!("expected the targets, got {:?}", engine.pending())
    };
    assert_eq!((min, max), (0, 2), "\"up to two target cards\"");
    assert_eq!(options.len(), 2, "p1's two cards: {options:?}");
    assert!(their_cards.iter().all(|card| options.contains(card)));
    assert!(
        !options.contains(&my_forest),
        "\"from a single graveyard\": the Forest is in the other one"
    );
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: their_cards.to_vec(),
            },
        )
        .expect("both of p1's cards are targeted");
    pass_until(&mut engine, stack_is_empty);

    for card in their_cards {
        assert_eq!(
            engine.state().object(card).map(|o| (o.zone, o.owner)),
            Some((Zone::Exile, p1)),
            "exiled, into its owner's exile"
        );
    }
    assert_eq!(
        engine.state().object(my_forest).map(|o| o.zone),
        Some(Zone::Graveyard)
    );
    assert_eq!(
        pt(&engine, hearse),
        (2, 2),
        "\"equal to the number of cards exiled with it\""
    );
}

/// Unlicensed Hearse with cards in one graveyard only: nothing asks which
/// graveyard, and the targets are asked at once.
#[test]
fn unlicensed_hearse_asks_no_graveyard_when_only_one_holds_cards() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[unlicensed_hearse()])
        .hand(1, &[plains()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");
    let plains_card = hand_to_graveyard(&mut engine, p1, plains());

    activate(&mut engine, p0, unlicensed_hearse(), 0);
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!("expected the targets at once, got {:?}", engine.pending())
    };
    assert_eq!(options, vec![plains_card]);
}

/// Unlicensed Hearse's Crew 2 (CR 702.122a): with two cards exiled with it
/// and two Llanowar Elves beside it, crew asks for creatures and offers both
/// Elves and never the Hearse ("other"). One Elf, power 1, is refused and
/// the question stands; both are taken, both are tapped, and once the
/// ability resolves the Hearse is a 2/2 artifact creature.
#[test]
fn unlicensed_hearse_is_crewed_by_two_elves_and_not_by_one() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[unlicensed_hearse(), llanowar_elves(), llanowar_elves()],
        )
        .hand(1, &[plains(), swamp()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");
    let their_cards = [
        hand_to_graveyard(&mut engine, p1, plains()),
        hand_to_graveyard(&mut engine, p1, swamp()),
    ];
    let hearse = on_battlefield(&engine, p0, unlicensed_hearse()).expect("the Hearse is out");
    let elves = elves_of(&engine, p0);
    assert_eq!(elves.len(), 2);

    activate(&mut engine, p0, unlicensed_hearse(), 0);
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: their_cards.to_vec(),
            },
        )
        .expect("both of p1's cards are targeted");
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(pt(&engine, hearse), (2, 2), "two cards exiled with it");
    let creature = |engine: &Engine<RegistryLookup>| {
        engine.state().object(hearse).is_some_and(|o| {
            o.characteristics()
                .types
                .contains(baylee_core::types::TypeSet::CREATURE)
        })
    };
    assert!(!creature(&engine), "a Vehicle is not a creature uncrewed");

    // Ability 2 is Crew 2 (0 is the exile, 1 the power-and-toughness static).
    activate(&mut engine, p0, unlicensed_hearse(), 2);
    let Pending::ChooseCards {
        player,
        options,
        min,
        max,
        prompt,
        ..
    } = engine.pending().clone()
    else {
        panic!("expected the crew question, got {:?}", engine.pending())
    };
    assert_eq!(player, p0);
    assert_eq!(prompt, ChoicePrompt::CostCrew { power: 2 });
    assert_eq!((min, max), (1, 2), "any number of the two Elves");
    assert_eq!(options.len(), 2);
    assert!(elves.iter().all(|elf| options.contains(elf)));
    assert!(!options.contains(&hearse), "\"other untapped creatures\"");

    assert!(
        engine
            .apply(
                p0,
                PlayerAction::ChooseObjects {
                    objects: vec![elves[0]],
                },
            )
            .is_err(),
        "one Elf is power 1, short of Crew 2"
    );
    assert!(
        matches!(engine.pending(), Pending::ChooseCards { .. }),
        "the question stands after a short answer"
    );
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: elves.clone(),
            },
        )
        .expect("two Elves are power 2");
    for elf in &elves {
        assert!(
            engine
                .state()
                .object(*elf)
                .is_some_and(|o| o.status.contains(Status::TAPPED)),
            "a creature that crews is tapped to pay"
        );
    }
    pass_until(&mut engine, stack_is_empty);
    assert!(creature(&engine), "\"becomes an artifact creature\"");
    assert!(
        engine.state().object(hearse).is_some_and(|o| o
            .characteristics()
            .types
            .contains(baylee_core::types::TypeSet::ARTIFACT)),
        "and stays an artifact"
    );
    assert_eq!(pt(&engine, hearse), (2, 2), "the exiled cards still count");
}

/// Unlicensed Hearse beside one Llanowar Elves: power 1 cannot reach Crew 2,
/// so crew is not offered at all, while the Hearse's own {T} is.
#[test]
fn unlicensed_hearse_is_not_offered_crew_below_its_number() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[unlicensed_hearse(), llanowar_elves()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");
    let hearse = on_battlefield(&engine, p0, unlicensed_hearse()).expect("the Hearse is out");
    assert!(
        hearse_offers(&engine, hearse, 0),
        "the {{T}} ability is offered"
    );
    assert!(
        !hearse_offers(&engine, hearse, 2),
        "one power-1 creature cannot crew 2"
    );
}
