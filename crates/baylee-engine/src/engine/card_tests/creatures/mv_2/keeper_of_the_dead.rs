//! `cards/creatures/mv_2/keeper_of_the_dead.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::partials_w1b_kit as kit;

fn keeper_of_the_dead() -> CardIndex {
    card_index("50ddd513-e06e-4728-b6cd-981839d0fba6")
}

#[test]
fn keeper_of_the_dead_destroys_target_nonblack_creature_an_opponent_controls() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(1, forest())
        .battlefield(0, &[keeper_of_the_dead(), swamp()])
        .battlefield(1, &[gray_ogre()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0));

    tap_all_mana(&mut engine, p0);
    let keeper = on_battlefield(&engine, p0, keeper_of_the_dead()).unwrap();
    let bears = on_battlefield(&engine, p1, gray_ogre()).unwrap();
    kit::activate_object(&mut engine, p0, keeper, 0);
    kit::settle(&mut engine, p0, |_, p| {
        kit::choose_targets(p, &[bears], &[])
    });

    assert!(on_battlefield(&engine, p1, gray_ogre()).is_none());
}
