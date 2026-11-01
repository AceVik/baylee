//! The supported creature body is played; unsupported abilities are omitted.
#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::legends_kit as lk;
use crate::engine::card_tests::partials_w1b_kit as kit;

fn keeper_of_the_beasts() -> CardIndex {
    card_index("86dcdffd-c14f-4e06-a74f-d2c727f55b1c")
}

#[test]
fn keeper_of_the_beasts_keeps_its_body_without_an_inaccurate_ability() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[keeper_of_the_beasts()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let body = on_battlefield(&engine, p0, keeper_of_the_beasts()).unwrap();
    lk::float(
        &mut engine,
        p0,
        &[
            (ManaColor::Black, 10),
            (ManaColor::Blue, 10),
            (ManaColor::Green, 10),
            (ManaColor::White, 10),
            (ManaColor::Red, 10),
        ],
    );
    assert!(
        kit::offered(&engine, body).is_empty(),
        "unsupported activations are unavailable"
    );
    kit::declare_attack(&mut engine, &[body], p1);
    pass_until(&mut engine, |e| {
        e.state().turn.phase == Phase::SecondMain && stack_is_empty(e)
    });
    assert_eq!(engine.state().players[1].life, 20 - 1);
    assert!(on_battlefield(&engine, p0, keeper_of_the_beasts()).is_some());
}
