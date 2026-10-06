//! `cards/lands/deserts/eroded_canyon.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// `Eroded Canyon` enters tapped, deals 1 damage to target opponent on entry,
/// and taps for `{{U}}` or `{{R}}` under `Coverage::Implemented`.
/// Playing the land places its enters-the-battlefield trigger on the stack, which targets the opponent
/// and reduces their life total while leaving the land tapped.
#[test]
fn eroded_canyon_enters_tapped_and_burns_target_opponent() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(1904, forest())
        .hand(0, &[eroded_canyon()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let land = play_land(&mut engine, p0, eroded_canyon());
    assert!(entered_tapped(&engine, land));

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });

    let Pending::ChooseTargets {
        player,
        player_options,
        min,
        max,
        ..
    } = engine.pending().clone()
    else {
        panic!("expected ChooseTargets prompt, got {:?}", engine.pending());
    };
    assert_eq!(player, p0);
    assert_eq!(player_options, vec![p1]);
    assert_eq!((min, max), (1, 1));

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

    assert_eq!(engine.state().players[1].life, 19);
    assert!(is_tapped(&engine, land));
}
