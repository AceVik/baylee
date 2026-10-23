//! `cards/creatures/mv_2/keeper_of_the_mind.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::partials_w1b_kit as kit;

fn keeper_of_the_mind() -> CardIndex {
    card_index("45393fba-b1cc-4db7-b7e0-3851bf421a65")
}

#[test]
fn keeper_of_the_mind_draws_a_card() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(1, forest())
        .battlefield(0, &[keeper_of_the_mind(), island()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0));

    let before = engine.state().zones.list(ZoneLocation::Hand(p0)).len();
    tap_all_mana(&mut engine, p0);
    let keeper = on_battlefield(&engine, p0, keeper_of_the_mind()).unwrap();
    kit::activate_object(&mut engine, p0, keeper, 0);
    kit::settle(&mut engine, p0, |_, p| kit::choose_targets(p, &[], &[p1]));

    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
        before + 1
    );
}
