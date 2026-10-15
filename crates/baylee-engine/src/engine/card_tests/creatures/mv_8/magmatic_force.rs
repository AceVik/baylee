//! `cards/creatures/mv_8/magmatic_force.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn magmatic_force() -> CardIndex {
    card_index("ea73153b-1ecb-4593-a9ec-ce4f54e69824")
}

/// Magmatic Force is a 7/7 for {5}{R}{R}{R}.
#[test]
fn magmatic_force_is_cast_for_eight_as_a_seven_seven() {
    let mut engine = at_main(&[mountain(); 8], &[magmatic_force()]);
    let force = cast_and_settle(&mut engine, magmatic_force());
    assert_eq!(pt(&engine, force), (7, 7));
}

/// "At the beginning of each upkeep, this creature deals 3 damage to any
/// target": the first upkeep asks for a target and a player can be named
/// (3 damage); the next upkeep, the other player's, asks again and a creature
/// can be named (a Bears dies).
#[test]
fn magmatic_force_deals_three_to_any_target_on_each_upkeep() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[magmatic_force()])
        .battlefield(1, &[grizzly_bears()])
        .start();
    keep_mulligans(&mut engine);
    let bear = on_battlefield(&engine, p1, grizzly_bears()).expect("their bear");

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    let Pending::ChooseTargets {
        player,
        options,
        player_options,
        ..
    } = engine.pending().clone()
    else {
        unreachable!()
    };
    assert_eq!(
        player, p0,
        "the Force's controller aims, whoever's upkeep it is"
    );
    assert!(options.contains(&bear), "any target: creatures");
    assert!(player_options.contains(&p1), "any target: players");
    let life = life_of(&engine, 1);
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![],
                players: vec![p1],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    // The first trigger resolved before the second upkeep asked again.
    assert_eq!(life_of(&engine, 1), life - 3, "3 damage to the player");

    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![bear],
                players: vec![],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert!(
        on_battlefield(&engine, p1, grizzly_bears()).is_none(),
        "3 kills a 2/2"
    );
}
