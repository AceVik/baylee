//! `cards/enchantments/auras/mv_2/maniacal_rage.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::partials_w1b_kit as kit;

fn maniacal_rage() -> CardIndex {
    card_index("b5dbb212-0399-475b-a831-921f7c76cdc4")
}

#[test]
fn maniacal_rage_gives_plus_two_plus_two() {
    use crate::engine::card_tests::legends_kit as lk;

    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(1, forest())
        .battlefield(0, &[gray_ogre(), grizzly_bears(), mountain(), mountain()])
        .battlefield(1, &[gray_ogre()])
        .hand(0, &[maniacal_rage()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0));

    let bears = on_battlefield(&engine, p0, gray_ogre()).unwrap();
    cast_from_hand(&mut engine, p0, maniacal_rage());
    kit::settle(&mut engine, p0, |_, p| {
        kit::choose_targets(p, &[bears], &[])
    });

    assert_eq!(pt(&engine, bears), (4, 4));
    let p1 = PlayerId::new(1);
    reach_their_main_phase(&mut engine, p1);
    let enemy = on_battlefield(&engine, p1, gray_ogre()).unwrap();
    let free = on_battlefield(&engine, p0, grizzly_bears()).unwrap();
    let offer = lk::attack(&mut engine, &[enemy]);
    assert!(!lk::may_block(&offer, bears, enemy));
    assert!(lk::may_block(&offer, free, enemy));
}
