//! `cards/lands/utility/valakut_the_molten_pinnacle.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

/// The entering Mountain is excluded from the five-other-Mountains condition.
#[test]
fn valakut_does_not_trigger_on_fifth_mountain() {
    let p0 = PlayerId::new(0);
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
    let life = engine.state().players[1].life;
    play_land(&mut engine, p0, mountain());
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(engine.state().players[1].life, life);
    assert!(!matches!(engine.pending(), Pending::ChooseTargets { .. }));
}

#[test]
fn valakut_triggers_on_sixth_mountain_and_damages_opponent() {
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
                mountain(),
                valakut_the_molten_pinnacle(),
            ],
        )
        .hand(0, &[mountain()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let life = engine.state().players[1].life;
    play_land(&mut engine, p0, mountain());
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
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
    assert_eq!(engine.state().players[1].life, life - 3);
}
