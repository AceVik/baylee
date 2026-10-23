//! `cards/creatures/mv_2/keeper_of_the_beasts.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::partials_w1b_kit as kit;

fn keeper_of_the_beasts() -> CardIndex {
    card_index("86dcdffd-c14f-4e06-a74f-d2c727f55b1c")
}

#[test]
fn keeper_of_the_beasts_creates_a_green_beast_token() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(1, forest())
        .battlefield(0, &[keeper_of_the_beasts(), forest()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0));

    tap_all_mana(&mut engine, p0);
    let keeper = on_battlefield(&engine, p0, keeper_of_the_beasts()).unwrap();
    kit::activate_object(&mut engine, p0, keeper, 0);
    kit::settle(&mut engine, p0, |_, p| kit::choose_targets(p, &[], &[p1]));

    let tokens = tokens_of(&engine, p0);
    assert_eq!(tokens.len(), 1);
    assert_eq!(pt(&engine, tokens[0]), (2, 2));
}
