//! `cards/lands/utility/big_apple_3_a_m.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

#[test]
fn big_apple_chooses_color_enters_tapped_and_taps_for_chosen_color() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .hand(0, &[big_apple_3_a_m()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let played = play_land(&mut engine, p0, big_apple_3_a_m());

    let Pending::ChooseColor { options, .. } = engine.pending().clone() else {
        panic!("expected color choice upon entry");
    };
    assert_eq!(options.len(), 5);

    engine
        .apply(p0, PlayerAction::ChooseColor(ManaColor::Blue))
        .unwrap();

    pass_until(&mut engine, stack_is_empty);
    assert!(entered_tapped(&engine, played));

    let from = engine.state().turn.number;
    pass_until(&mut engine, |e| {
        e.state().turn.number > from
            && e.state().turn.active == p0
            && matches!(e.state().turn.phase, Phase::FirstMain)
    });
    assert!(!is_tapped(&engine, played));

    activate(&mut engine, p0, big_apple_3_a_m(), 0);
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.available(ManaColor::Blue), 1);
    assert!(is_tapped(&engine, played));
}

#[test]
fn big_apple_makes_one_rat_per_opponent_not_per_other_seat() {
    let p0 = PlayerId::new(0);
    let mut e = Duel::table(SEED, forest(), 4)
        .team(0, 1)
        .team(1, 1)
        .team(2, 2)
        .team(3, 3)
        .battlefield(
            0,
            &[
                big_apple_3_a_m(),
                forest(),
                forest(),
                forest(),
                forest(),
                forest(),
            ],
        )
        .start();
    keep_mulligans(&mut e);
    pass_until(&mut e, |e| {
        matches!(e.pending(), Pending::ChooseColor { .. })
    });
    e.apply(p0, PlayerAction::ChooseColor(ManaColor::Blue))
        .unwrap();
    reach_main_phase(&mut e, p0);
    tap_all_mana_but(&mut e, p0, Some(big_apple_3_a_m()));
    activate(&mut e, p0, big_apple_3_a_m(), 1);
    pass_until(&mut e, stack_is_empty);
    let rats = tokens_of(&e, p0);
    assert_eq!(rats.len(), 2);
    for rat in rats {
        assert_eq!(pt(&e, rat), (1, 1));
    }
}
