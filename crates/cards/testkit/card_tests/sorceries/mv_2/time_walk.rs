//! `cards/sorceries/mv_2/time_walk.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Time Walk: "Take an extra turn after this one."
#[test]
fn time_walk_takes_an_extra_turn() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[island(), island()])
        .hand(0, &[time_walk()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let turn = engine.state().turn.number;
    tap_all_mana(&mut engine, p0);
    cast_with_floating(&mut engine, p0, time_walk());
    pass_until(&mut engine, |e| {
        e.state().turn.number == turn + 1 && matches!(e.state().turn.phase, Phase::FirstMain)
    });
    assert_eq!(
        engine.state().turn.active,
        p0,
        "\"take an extra turn\" — its own caster's"
    );
}
