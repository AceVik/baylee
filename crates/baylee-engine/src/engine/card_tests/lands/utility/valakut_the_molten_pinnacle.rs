//! `cards/lands/utility/valakut_the_molten_pinnacle.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Valakut, the Molten Pinnacle prints `This land enters tapped`, `Whenever a Mountain you control
/// enters, if you control at least five other Mountains, you may have this land deal 3 damage to any
/// target`, and `{T}: Add {R}.`
/// The card is marked `Coverage::Partial` because `Condition::ControlCount` counts the entering Mountain.
/// With four `mountain` permanents and Valakut in play, playing a fifth `mountain` triggers Valakut's
/// intervening-if condition, dealing 3 damage to the targeted opponent upon resolution.
#[test]
fn valakut_triggers_on_fifth_mountain_and_damages_opponent() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(
            0,
            &[
                mountain(),
                mountain(),
                mountain(),
                mountain(),
                valakut_the_molten_pinnacle(),
            ],
        )
        .hand(0, &[mountain()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let p1_life = engine.state().players[1].life;
    play_land(&mut engine, p0, mountain());

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });

    let Pending::ChooseTargets { player_options, .. } = engine.pending().clone() else {
        panic!("expected target choice");
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
    assert_eq!(engine.state().players[1].life, p1_life - 3);
}
