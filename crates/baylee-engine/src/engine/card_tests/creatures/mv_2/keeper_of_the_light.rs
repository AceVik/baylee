//! `cards/creatures/mv_2/keeper_of_the_light.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::partials_w1b_kit as kit;

fn keeper_of_the_light() -> CardIndex {
    card_index("62d1300f-2b76-42b9-8e6c-f920712cd547")
}

#[test]
fn keeper_of_the_light_gains_three_life() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(1, forest())
        .battlefield(0, &[keeper_of_the_light(), plains()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0));

    tap_all_mana(&mut engine, p0);
    let keeper = on_battlefield(&engine, p0, keeper_of_the_light()).unwrap();
    kit::activate_object(&mut engine, p0, keeper, 0);
    kit::settle(&mut engine, p0, |_, p| kit::choose_targets(p, &[], &[p1]));

    assert_eq!(engine.state().players[0].life, 23);
}
