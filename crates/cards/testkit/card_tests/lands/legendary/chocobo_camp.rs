//! `cards/lands/legendary/chocobo_camp.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Chocobo Camp: "This land enters tapped unless you control a legendary creature." / "{T}: Add {G}." / "{2}{G}{G}, {T}: Create a 2/2 green Bird creature token..."
/// Controlling a legendary creature allows Chocobo Camp to enter untapped and immediately tap for green mana.
#[test]
fn chocobo_camp_enters_untapped_with_legendary_creature() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(213, forest())
        .battlefield(0, &[katara_the_fearless()])
        .hand(0, &[chocobo_camp()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let land = play_land(&mut engine, p0, chocobo_camp());
    assert!(!entered_tapped(&engine, land));

    activate(&mut engine, p0, chocobo_camp(), 0);
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.available(ManaColor::Green), 1);
    assert!(is_tapped(&engine, land));
}

/// Chocobo Camp's `{{2}}{{G}}{{G}}, {{T}}`: one 2/2 green Bird creature token.
#[test]
fn chocobo_camp_makes_a_bird_for_four() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(213, forest())
        .battlefield(0, &[chocobo_camp(), forest(), forest(), forest(), forest()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let camp = on_battlefield(&engine, p0, chocobo_camp()).expect("Chocobo Camp deployed");
    tap_mana_except(&mut engine, p0, camp);
    activate(&mut engine, p0, chocobo_camp(), 1);
    pass_until(&mut engine, stack_is_empty);

    assert!(is_tapped(&engine, camp));
    let tokens = tokens_of(&engine, p0);
    assert_eq!(tokens.len(), 1, "one Bird");
    assert_eq!(pt(&engine, tokens[0]), (2, 2));
    assert!(types(&engine, tokens[0]).contains(TypeSet::CREATURE));
}

/// A nonlegendary creature or an opposing legend does not satisfy the entry condition.
#[test]
fn chocobo_camp_requires_your_legendary_creature_to_enter_untapped() {
    for own_creatures in [vec![], vec![llanowar_elves()]] {
        let p0 = PlayerId::new(0);
        let mut engine = Duel::new(SEED, forest())
            .battlefield(0, &own_creatures)
            .battlefield(1, &[katara_the_fearless()])
            .hand(0, &[chocobo_camp()])
            .start();
        keep_mulligans(&mut engine);
        reach_main_phase(&mut engine, p0);
        let camp = play_land(&mut engine, p0, chocobo_camp());
        assert!(entered_tapped(&engine, camp));
        assert!(!priority_offer(&engine).abilities.contains(&(camp, 0)));
    }
}

/// The created Bird's landfall affects itself, expires, and ignores opposing lands.
#[test]
fn chocobo_camp_bird_landfall_is_personal_and_until_end_of_turn() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                chocobo_camp(),
                forest(),
                forest(),
                forest(),
                forest(),
                llanowar_elves(),
            ],
        )
        .hand(0, &[forest()])
        .hand(1, &[forest()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let camp = on_battlefield(&engine, p0, chocobo_camp()).unwrap();
    let elf = on_battlefield(&engine, p0, llanowar_elves()).unwrap();
    tap_mana_except(&mut engine, p0, camp);
    activate(&mut engine, p0, chocobo_camp(), 1);
    pass_until(&mut engine, stack_is_empty);
    let birds = tokens_of(&engine, p0);
    assert_eq!(birds.len(), 1);
    let bird = birds[0];
    let chars = engine.state().object(bird).unwrap().characteristics();
    assert_eq!(pt(&engine, bird), (2, 2));
    assert_eq!(
        chars.colors,
        baylee_core::color::ColorSet::from_slice(&[baylee_core::color::Color::Green])
    );
    assert!(
        chars
            .subtypes
            .contains(baylee_core::generated::subtypes::creature::BIRD)
    );
    play_land(&mut engine, p0, forest());
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(pt(&engine, bird), (3, 2));
    assert_eq!(pt(&engine, elf), (1, 1));
    reach_their_main_phase(&mut engine, p1);
    assert_eq!(pt(&engine, bird), (2, 2));
    play_land(&mut engine, p1, forest());
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(pt(&engine, bird), (2, 2));
}

/// The rider follows the next Bird cast, independent of where its mana was spent.
#[test]
fn batch55_chocobo_camp_next_bird_gets_one_counter_only() {
    let p0 = PlayerId::new(0);
    let bird_card = baylee_core::generated::index::BIRDS_OF_PARADISE;
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[chocobo_camp(), forest(), forest()])
        .hand(0, &[llanowar_elves(), bird_card, bird_card])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    activate(&mut engine, p0, chocobo_camp(), 0);
    cast_with_floating(&mut engine, p0, llanowar_elves());
    pass_until(&mut engine, stack_is_empty);
    let elf = on_battlefield(&engine, p0, llanowar_elves()).unwrap();
    assert_eq!(
        counters_on(&engine, elf, baylee_cards_dsl::CounterKind::P1P1),
        0
    );
    cast_from_hand(&mut engine, p0, bird_card);
    pass_until(&mut engine, stack_is_empty);
    let first = on_battlefield(&engine, p0, bird_card).unwrap();
    assert_eq!(
        counters_on(&engine, first, baylee_cards_dsl::CounterKind::P1P1),
        1
    );
    assert_eq!(pt(&engine, first), (1, 2));
    cast_with_floating(&mut engine, p0, bird_card);
    pass_until(&mut engine, stack_is_empty);
    let birds: Vec<_> = engine
        .state()
        .zones
        .list(ZoneLocation::Battlefield)
        .iter()
        .copied()
        .filter(|id| {
            engine
                .state()
                .object(*id)
                .is_some_and(|o| o.card.is_some_and(|card| card.index == bird_card))
        })
        .collect();
    assert_eq!(birds.len(), 2);
    let second = *birds.iter().find(|id| **id != first).unwrap();
    assert_eq!(
        counters_on(&engine, second, baylee_cards_dsl::CounterKind::P1P1),
        0
    );
    assert_eq!(pt(&engine, second), (0, 1));
}

/// An unused delayed cast rider expires at end of turn.
#[test]
fn batch55_chocobo_camp_unused_rider_expires() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let bird_card = baylee_core::generated::index::BIRDS_OF_PARADISE;
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[chocobo_camp()])
        .hand(0, &[bird_card])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    activate(&mut engine, p0, chocobo_camp(), 0);
    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);
    // The second activation would establish a new rider; instead pay with the
    // Forest drawn naturally from the library and played this turn.
    play_land(&mut engine, p0, forest());
    let camp = on_battlefield(&engine, p0, chocobo_camp()).unwrap();
    tap_mana_except(&mut engine, p0, camp);
    cast_with_floating(&mut engine, p0, bird_card);
    pass_until(&mut engine, stack_is_empty);
    let bird = on_battlefield(&engine, p0, bird_card).unwrap();
    assert_eq!(
        counters_on(&engine, bird, baylee_cards_dsl::CounterKind::P1P1),
        0
    );
    assert_eq!(pt(&engine, bird), (0, 1));
}

/// Separate activations establish separate delayed triggers for the same next Bird.
#[test]
fn batch55_chocobo_camp_multiple_riders_stack_on_the_next_bird() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let bird_card = baylee_core::generated::index::BIRDS_OF_PARADISE;
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[chocobo_camp(), chocobo_camp()])
        .hand(0, &[bird_card, bird_card])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let camps: Vec<_> = engine
        .state()
        .zones
        .list(ZoneLocation::Battlefield)
        .iter()
        .copied()
        .filter(|id| {
            engine
                .state()
                .object(*id)
                .is_some_and(|o| o.card.is_some_and(|c| c.index == chocobo_camp()))
        })
        .collect();
    assert_eq!(camps.len(), 2);
    for camp in camps {
        engine
            .apply(
                p0,
                PlayerAction::ActivateAbility {
                    source: camp,
                    ability_index: 0,
                },
            )
            .unwrap();
    }
    cast_with_floating(&mut engine, p0, bird_card);
    pass_until(&mut engine, stack_is_empty);
    let first = on_battlefield(&engine, p0, bird_card).unwrap();
    assert_eq!(
        counters_on(&engine, first, baylee_cards_dsl::CounterKind::P1P1),
        2
    );
    assert_eq!(pt(&engine, first), (2, 3));
    cast_with_floating(&mut engine, p0, bird_card);
    pass_until(&mut engine, stack_is_empty);
    let second = *engine
        .state()
        .zones
        .list(ZoneLocation::Battlefield)
        .iter()
        .find(|id| {
            **id != first
                && engine
                    .state()
                    .object(**id)
                    .is_some_and(|o| o.card.is_some_and(|c| c.index == bird_card))
        })
        .unwrap();
    assert_eq!(
        counters_on(&engine, second, baylee_cards_dsl::CounterKind::P1P1),
        0
    );
    reach_their_main_phase(&mut engine, p1);
    assert_eq!(pt(&engine, first), (2, 3), "entry counters are permanent");
}
