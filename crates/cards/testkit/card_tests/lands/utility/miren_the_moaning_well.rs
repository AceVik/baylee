//! `cards/lands/utility/miren_the_moaning_well.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Miren, the Moaning Well: "{T}: Add {C}." / "{3}, {T}, Sacrifice a creature: You gain life equal to the sacrificed creature's toughness."
/// The legendary land taps for its base mana ability to add {C} to the mana pool.
#[test]
fn miren_the_moaning_well_taps_for_colorless_mana() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(137, forest())
        .battlefield(0, &[miren_the_moaning_well()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let well = on_battlefield(&engine, p0, miren_the_moaning_well()).expect("Well deployed");
    activate(&mut engine, p0, miren_the_moaning_well(), 0);

    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.total(), 1);
    assert_eq!(pool.available(ManaColor::Colorless), 1);
    assert!(is_tapped(&engine, well));
}

/// A 2/1 Cat: power and toughness differ, so the life gained says which
/// of the two numbers was read.
fn savannah_lions() -> CardIndex {
    card_index("60ba93eb-39e6-4af2-9c66-cd38f72daff2")
}

/// "{3}, {T}, Sacrifice a creature: You gain life equal to the sacrificed
/// creature's toughness." The Lions (2/1) are pumped to 5/4 with Giant
/// Growth first, so 4 life is the toughness the creature last had, not its
/// printed 1 or its power 5. The {3} is paid from floating mana and the
/// Well is tapped as part of the cost.
#[test]
fn miren_gains_the_pumped_toughness_for_three_mana_and_a_creature() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                miren_the_moaning_well(),
                forest(),
                forest(),
                forest(),
                forest(),
                savannah_lions(),
            ],
        )
        .hand(0, &[giant_growth()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let well = on_battlefield(&engine, p0, miren_the_moaning_well()).expect("Miren is out");
    let lions = on_battlefield(&engine, p0, savannah_lions()).expect("the Lions");
    let forests = mine(&engine, p0, forest(), Zone::Battlefield);
    assert_eq!(forests.len(), 4);

    // One Forest pays Giant Growth.
    tap_mana_where(&mut engine, p0, |id| id == forests[0]);
    cast_with_floating(&mut engine, p0, giant_growth());
    aim_at(&mut engine, p0, lions);
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(pt(&engine, lions), (5, 4), "Giant Growth: +3/+3");

    // Two floating are not enough; three are.
    tap_mana_where(&mut engine, p0, |id| id == forests[1] || id == forests[2]);
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("priority expected")
    };
    assert!(
        !legal.abilities.contains(&(well, 1)),
        "two floating mana are not {{3}}: {:?}",
        legal.abilities
    );
    tap_mana_where(&mut engine, p0, |id| id != well);
    assert_eq!(engine.state().players[0].mana_pool.total(), 3);

    let life = engine.state().players[0].life;
    activate(&mut engine, p0, miren_the_moaning_well(), 1);
    if let Pending::ChooseCards { options, .. } = engine.pending().clone() {
        assert!(options.contains(&lions));
        engine
            .apply(
                p0,
                PlayerAction::ChooseObjects {
                    objects: vec![lions],
                },
            )
            .expect("sacrifice the Lions");
    }
    pass_until(&mut engine, stack_is_empty);
    assert!(is_tapped(&engine, well), "{{T}} is part of the cost");
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "{{3}} spent"
    );
    assert!(in_graveyard(&engine, p0, savannah_lions()).is_some());
    assert_eq!(engine.state().players[0].life, life + 4);
}

/// With three mana floating the ability is offered and with none it is not,
/// even though a creature stands ready to be sacrificed.
#[test]
fn miren_is_offered_only_with_three_mana_to_pay() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                miren_the_moaning_well(),
                forest(),
                forest(),
                forest(),
                savannah_lions(),
            ],
        )
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let well = on_battlefield(&engine, p0, miren_the_moaning_well()).expect("Miren is out");
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("priority expected")
    };
    assert!(
        !legal.abilities.contains(&(well, 1)),
        "no mana floating: {:?}",
        legal.abilities
    );
    tap_mana_where(&mut engine, p0, |id| id != well);
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("priority expected")
    };
    assert!(legal.abilities.contains(&(well, 1)), "{{3}} floating");
}
