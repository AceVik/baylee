//! `cards/lands/storage/city_of_shadows.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn city_of_shadows() -> CardIndex {
    card_index("043192d8-6077-46c3-b43f-b7caf6762869")
}

fn grizzly_bears() -> CardIndex {
    card_index("14c8f55d-d177-4c25-a931-ebeb9e6062a0")
}

fn savannah_lions() -> CardIndex {
    card_index("60ba93eb-39e6-4af2-9c66-cd38f72daff2")
}

fn elves() -> CardIndex {
    card_index("68954295-54e3-4303-a6bc-fc4547a4e3a3")
}

fn hill_giant() -> CardIndex {
    card_index("342199e0-15b6-4824-83da-25caef2592b3")
}

fn board() -> Engine<RegistryLookup> {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, swamp())
        .battlefield(
            0,
            &[
                city_of_shadows(),
                grizzly_bears(),
                savannah_lions(),
                elves(),
            ],
        )
        .battlefield(1, &[hill_giant()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");
    engine
}

/// Presses the first line and exiles `victim` for it, checking the menu on
/// the way. Lets the ability resolve.
#[track_caller]
fn bank_by_exiling(engine: &mut Engine<RegistryLookup>, city: ObjectId, victim: CardIndex) {
    let p0 = PlayerId::new(0);
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: city,
                ability_index: 0,
            },
        )
        .expect("an untapped City with a creature to exile may bank");
    let Pending::ChooseCards {
        player,
        options,
        min,
        max,
        ..
    } = engine.pending().clone()
    else {
        panic!("expected the creature to exile, got {:?}", engine.pending())
    };
    assert_eq!(player, p0);
    assert_eq!((min, max), (1, 1), "one creature");
    let chosen = engine
        .state()
        .zones
        .list(ZoneLocation::Battlefield)
        .iter()
        .copied()
        .find(|id| {
            options.contains(id)
                && engine
                    .state()
                    .object(*id)
                    .is_some_and(|o| o.card.is_some_and(|c| c.index == victim))
        })
        .expect("the creature to exile is on the menu");
    for id in &options {
        let object = engine.state().object(*id).expect("an offered object");
        assert_eq!(
            object.controller, p0,
            "only creatures I control may be exiled"
        );
    }
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![chosen],
            },
        )
        .expect("the creature is on the menu");
    pass_until(engine, stack_is_empty);
}

/// City of Shadows (043192d8-6077-46c3-b43f-b7caf6762869) — "{T}, Exile a
/// creature you control: Put a storage counter on this land."
///
/// Only my own creatures are offered; the one chosen leaves the battlefield
/// without going to the graveyard, and the City, tapped for it, holds one
/// counter.
#[test]
fn city_of_shadows_exiles_a_creature_you_control_for_a_storage_counter() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = board();
    let city = on_battlefield(&engine, p0, city_of_shadows()).expect("the City is out");

    bank_by_exiling(&mut engine, city, grizzly_bears());

    assert_eq!(counters_on(&engine, city, counters::STORAGE), 1);
    assert!(
        on_battlefield(&engine, p0, grizzly_bears()).is_none(),
        "the Bears left"
    );
    assert!(
        in_graveyard(&engine, p0, grizzly_bears()).is_none(),
        "exiled, not dead"
    );
    assert!(on_battlefield(&engine, p0, savannah_lions()).is_some());
    assert!(on_battlefield(&engine, p1, hill_giant()).is_some());
    assert!(
        engine
            .state()
            .object(city)
            .expect("the City is there")
            .status
            .contains(Status::TAPPED),
        "{{T}} is part of the cost"
    );
}

/// "{T}: Add {C} for each storage counter on this land": two turns, two
/// counters, and the third turn's tap makes two colorless.
#[test]
fn city_of_shadows_adds_one_colorless_per_storage_counter() {
    let p0 = PlayerId::new(0);
    let mut engine = board();
    let city = on_battlefield(&engine, p0, city_of_shadows()).expect("the City is out");

    bank_by_exiling(&mut engine, city, grizzly_bears());
    cross_into_the_next_own_main(&mut engine, p0);
    bank_by_exiling(&mut engine, city, savannah_lions());
    assert_eq!(counters_on(&engine, city, counters::STORAGE), 2);
    cross_into_the_next_own_main(&mut engine, p0);

    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: city,
                ability_index: 1,
            },
        )
        .expect("the City taps for mana");
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Colorless),
        2,
        "{{C}} for each of the two counters"
    );
}

/// With no counters the second line makes nothing.
#[test]
fn city_of_shadows_without_counters_makes_no_mana() {
    let p0 = PlayerId::new(0);
    let mut engine = board();
    let city = on_battlefield(&engine, p0, city_of_shadows()).expect("the City is out");

    // Whether the engine refuses the tap or lets it make nothing, no mana
    // may appear.
    let _ = engine.apply(
        p0,
        PlayerAction::ActivateAbility {
            source: city,
            ability_index: 1,
        },
    );
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Colorless),
        0
    );
}
