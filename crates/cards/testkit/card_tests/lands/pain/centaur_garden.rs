//! `cards/lands/pain/centaur_garden.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Centaur Garden prints `{{T}}: Add {{G}}. This land deals 1 damage to you.`
/// and `Threshold — {{G}}, {{T}}, Sacrifice this land: Target creature gets +3/+3
/// until end of turn. Activate only if there are seven or more cards in your graveyard.`
///
/// Under `Coverage::Partial`, the threshold condition is unsupported and the
/// ability is offered unconditionally. This test floats `{G}` from a Forest,
/// activates ability 1 targeting an `aurochs()`, verifies the land is sacrificed
/// as part of the activation cost, and checks that the target creature gets +3/+3.
#[test]
fn centaur_garden_activates_to_give_target_creature_plus_three_plus_three() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[centaur_garden(), forest(), aurochs()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    // Seven cards, because the ability is gated on threshold now.
    // This test read the ability as offered on an empty graveyard for
    // as long as `Condition` could not say the sentence, which is a
    // land strictly stronger than the printed one.
    seed_graveyard(&mut engine, p0, 7);

    let cow = on_battlefield(&engine, p0, aurochs()).expect("aurochs on battlefield");
    assert_eq!(pt(&engine, cow), (2, 3));

    tap_all_mana_but(&mut engine, p0, Some(centaur_garden()));
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Green),
        1
    );

    activate(&mut engine, p0, centaur_garden(), 1);

    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!("expected target choice, got {:?}", engine.pending())
    };
    assert!(options.contains(&cow));

    engine
        .apply(p0, PlayerAction::ChooseObjects { objects: vec![cow] })
        .unwrap();

    assert!(
        in_graveyard(&engine, p0, centaur_garden()).is_some(),
        "centaur garden sacrificed as cost"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "mana spent to pay activation cost"
    );

    pass_until(&mut engine, stack_is_empty);
    assert_eq!(pt(&engine, cow), (5, 6), "aurochs received +3/+3");
}

/// Centaur Garden: "+3/+3 until end of turn", and it was ungated before.
#[test]
fn centaur_garden_pumps_by_three_only_at_threshold() {
    let p0 = PlayerId::new(0);

    let mut engine = Duel::new(17, forest())
        .battlefield(0, &[centaur_garden(), forest(), llanowar_elves()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let land = on_battlefield(&engine, p0, centaur_garden()).expect("in play");
    let elves = on_battlefield(&engine, p0, llanowar_elves()).expect("in play");

    tap_mana_except(&mut engine, p0, land);
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("still priority")
    };
    assert!(
        !legal.abilities.contains(&(land, 1)),
        "an empty graveyard is not threshold, and the green mana is floating"
    );

    seed_graveyard(&mut engine, p0, 7);
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: land,
                ability_index: 1,
            },
        )
        .unwrap();
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![elves],
                players: Vec::new(),
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(pt(&engine, elves), (4, 4), "a 1/1 with +3/+3");
}
