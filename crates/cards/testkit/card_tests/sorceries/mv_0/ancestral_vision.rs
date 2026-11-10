//! `cards/sorceries/mv_0/ancestral_vision.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Ancestral Vision: "Suspend 4—{U} ... Target player draws three cards."
/// Suspended on turn one, it loses a time counter at each of its owner's
/// upkeeps, and at the last it is cast for free; the spell draws three.
#[test]
fn ancestral_vision_suspended_is_cast_free_at_the_last_counter_and_draws_three() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(4401, island())
        .battlefield(0, &[island()])
        .hand(0, &[ancestral_vision_card()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    tap_all_mana(&mut engine, p0);
    let card = in_hand(&engine, p0, ancestral_vision_card()).expect("in hand");
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("priority")
    };
    assert!(
        legal.suspendable.contains(&card),
        "{{U}} is paid: suspend is offered"
    );
    engine
        .apply(p0, PlayerAction::Suspend { card })
        .expect("suspended for {U}");
    assert_eq!(
        exiled(&engine, p0, ancestral_vision_card()),
        1,
        "exiled, not cast"
    );

    // Turns 3, 5, 7 remove a counter each; the fourth, on turn 9, is the last.
    for turn in [3, 5, 7, 9] {
        pass_until(&mut engine, |e| e.state().turn.number >= turn);
    }
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChoosePlayer { .. })
    });
    let before = library_size(&engine, p0);
    engine
        .apply(p0, PlayerAction::ChoosePlayer(p0))
        .expect("target player: myself");
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(library_size(&engine, p0), before - 3, "three cards drawn");
    assert_eq!(
        engine.state().turn.number,
        9,
        "at the ninth turn's upkeep, not before"
    );
}
