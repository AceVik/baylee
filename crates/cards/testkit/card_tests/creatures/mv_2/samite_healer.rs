//! `cards/creatures/mv_2/samite_healer.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Samite Healer — `{T}`: Prevent the next 1 damage that would be dealt to
/// any target this turn. Shielding an opponent and then bolting them for 3
/// leaves only 2 through.
#[test]
fn samite_healer_prevents_the_first_point_of_damage_to_any_target() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, plains())
        .battlefield(0, &[samite_healer(), mountain()])
        .hand(0, &[lightning_bolt()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    activate(&mut engine, p0, samite_healer(), 0);
    let Pending::ChooseTargets {
        player,
        player_options,
        min,
        max,
        ..
    } = engine.pending().clone()
    else {
        panic!(
            "expected a target for the shield, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p0);
    assert_eq!((min, max), (1, 1));
    assert!(
        player_options.contains(&p0) && player_options.contains(&p1),
        "\"any target\" reaches either player: {player_options:?}"
    );
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![],
                players: vec![p1],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    cast_from_hand(&mut engine, p0, lightning_bolt());
    let Pending::ChooseTargets { player_options, .. } = engine.pending().clone() else {
        panic!(
            "expected Lightning Bolt's target, got {:?}",
            engine.pending()
        )
    };
    assert!(player_options.contains(&p1));
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![],
                players: vec![p1],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        engine.state().players[1].life,
        18,
        "3 damage, 1 of it prevented by the shield"
    );
}
