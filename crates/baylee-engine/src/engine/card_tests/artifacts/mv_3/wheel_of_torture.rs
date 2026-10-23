//! `cards/artifacts/mv_3/wheel_of_torture.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

fn wheel_of_torture() -> CardIndex {
    card_index("7a3e6e84-a8c2-472d-99fb-7cd14850b5ce")
}

#[test]
fn wheel_of_torture_deals_three_minus_hand_size_at_opponents_upkeep() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, plains())
        .hand(0, &[wheel_of_torture()])
        .battlefield(0, &[plains(), plains(), plains()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    cast_from_hand(&mut engine, p0, wheel_of_torture());
    pass_until(&mut engine, stack_is_empty);

    pass_until(&mut engine, |e| {
        e.state().turn.active == p1 && e.state().turn.step == crate::turn::Step::Upkeep
    });
    pass_until(&mut engine, |e| {
        e.state().turn.step != crate::turn::Step::Upkeep
    });

    // Opponent has 0 cards in hand at upkeep, so Wheel of Torture deals 3 - 0 = 3 damage.
    assert_eq!(engine.state().players[1].life, 17);
}

#[test]
fn wheel_never_hits_its_controller_or_a_full_hand() {
    use crate::engine::card_tests::legends_kit as lk;
    let mut engine = lk::game(
        SEED,
        &[wheel_of_torture()],
        &[],
        &[],
        &[forest(), forest(), forest(), forest()],
    );
    assert_eq!(lk::life(&engine, lk::P0), 20, "own upkeep does not trigger");
    reach_their_main_phase(&mut engine, lk::P1);
    assert_eq!(
        lk::life(&engine, lk::P1),
        20,
        "negative damage floors to zero"
    );
}
