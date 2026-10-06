//! `cards/lands/manlands/great_hall_of_the_biblioplex.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Great Hall of the Biblioplex: "{T}: Add {C}." / "{T}, Pay 1 life: Add one mana of any color. Spend this mana only to cast an instant or sorcery spell."
/// Activating ability 1 pays 1 life and adds one restricted mana of the chosen color to `pool.restricted()`.
/// The `{5}` animation is `great_hall_of_the_biblioplex_becomes_a_wizard_once_and_stays_one`.
#[test]
fn great_hall_of_the_biblioplex_adds_restricted_spell_mana() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(215, forest())
        .battlefield(0, &[great_hall_of_the_biblioplex()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let hall = on_battlefield(&engine, p0, great_hall_of_the_biblioplex()).expect("Hall deployed");
    activate(&mut engine, p0, great_hall_of_the_biblioplex(), 1);

    let Pending::ChooseColor { options, .. } = engine.pending().clone() else {
        panic!("expected color choice, got {:?}", engine.pending());
    };
    assert_eq!(options.len(), 5);

    engine
        .apply(p0, PlayerAction::ChooseColor(ManaColor::Blue))
        .unwrap();

    assert_eq!(engine.state().players[0].life, 19);
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.available(ManaColor::Blue), 0);
    assert_eq!(pool.restricted().len(), 1);
    assert_eq!(pool.restricted()[0].amount, 1);
    assert_eq!(pool.restricted()[0].color, ManaColor::Blue);
    assert!(is_tapped(&engine, hall));
}

/// Great Hall of the Biblioplex's third ability: "{5}: If this land isn't a
/// creature, it becomes a 2/4 Wizard creature with 'Whenever you cast an
/// instant or sorcery spell, this creature gets +1/+0 until end of turn.'
/// It's still a land."
///
/// Activated twice. The second time the land already is a creature, so the
/// "if" leaves it alone — and that is visible: a second animation would
/// have granted the trigger a second time, and the Bolt below would pump it
/// by two. The sentence prints no duration, so a turn later it is still a
/// 2/4 Wizard, and the +1/+0 is gone.
#[test]
fn great_hall_of_the_biblioplex_becomes_a_wizard_once_and_stays_one() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut board = vec![great_hall_of_the_biblioplex()];
    board.extend(std::iter::repeat_n(mountain(), 11));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &board)
        .hand(0, &[lightning_bolt()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");
    let hall = on_battlefield(&engine, p0, great_hall_of_the_biblioplex()).expect("the Hall");
    let mountains = all_on_battlefield(&engine, p0, mountain());
    let is_creature = |e: &Engine<RegistryLookup>| {
        e.state()
            .object(hall)
            .is_some_and(|o| o.characteristics().types.contains(TypeSet::CREATURE))
    };
    assert!(!is_creature(&engine), "a land to begin with");

    for batch in [&mountains[..5], &mountains[5..10]] {
        tap_mana_where(&mut engine, p0, |id| batch.contains(&id));
        activate(&mut engine, p0, great_hall_of_the_biblioplex(), 2);
        pass_until(&mut engine, stack_is_empty);
        let hall_now = engine
            .state()
            .object(hall)
            .expect("the Hall")
            .characteristics();
        assert!(
            hall_now
                .types
                .contains(TypeSet::CREATURE.union(TypeSet::LAND)),
            "a creature and still a land"
        );
        assert!(
            hall_now
                .subtypes
                .contains(baylee_core::generated::subtypes::creature::WIZARD)
        );
        assert_eq!(pt(&engine, hall), (2, 4));
    }

    tap_mana_where(&mut engine, p0, |id| id == mountains[10]);
    cast_with_floating(&mut engine, p0, lightning_bolt());
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![],
                players: vec![p1],
            },
        )
        .expect("the opponent is a legal target");
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        pt(&engine, hall),
        (3, 4),
        "one instant, one trigger: +1/+0, and not +2 from a second grant"
    );

    cross_into_the_next_own_main(&mut engine, p0);
    assert!(
        is_creature(&engine),
        "no duration was printed: still a creature"
    );
    assert_eq!(
        pt(&engine, hall),
        (2, 4),
        "and the pump ended with its turn"
    );
}
