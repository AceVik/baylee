//! `cards/creatures/mv_1/fledgling_osprey.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

fn fledgling_osprey() -> CardIndex {
    card_index("d5482ccf-7c3f-45b8-aaa8-8f6686fe584d")
}

#[test]
fn fledgling_osprey_gains_flying_while_enchanted() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .hand(0, &[rancor()])
        .battlefield(0, &[fledgling_osprey(), forest()])
        .battlefield(1, &[gray_ogre()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let osprey = on_battlefield(&engine, p0, fledgling_osprey()).unwrap();
    assert!(!keywords(&engine, osprey).contains(KeywordSet::FLYING));

    cast_from_hand(&mut engine, p0, rancor());
    aim_at(&mut engine, p0, osprey);
    pass_until(&mut engine, stack_is_empty);

    assert!(keywords(&engine, osprey).contains(KeywordSet::FLYING));
    let ground = on_battlefield(&engine, PlayerId::new(1), gray_ogre()).unwrap();
    let offer = crate::engine::card_tests::legends_kit::attack(&mut engine, &[osprey]);
    assert!(!crate::engine::card_tests::legends_kit::may_block(
        &offer, ground, osprey
    ));
}

#[test]
fn unenchanted_osprey_can_be_blocked_on_the_ground() {
    use crate::engine::card_tests::legends_kit as lk;
    let mut engine = lk::game(SEED, &[fledgling_osprey()], &[gray_ogre()], &[], &[]);
    let osprey = lk::obj(&engine, lk::P0, fledgling_osprey());
    let ground = lk::obj(&engine, lk::P1, gray_ogre());
    let offer = lk::attack(&mut engine, &[osprey]);
    assert!(lk::may_block(&offer, ground, osprey));
}
