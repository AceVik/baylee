//! `cards/creatures/mv_3/mage_il_vec.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::partials_w1b_kit as kit;

fn mage_il_vec() -> CardIndex {
    card_index("adea7db6-634c-4c5d-be40-264b4acffc53")
}

#[test]
fn mage_il_vec_discards_random_card_to_deal_one_damage() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(1, forest())
        .battlefield(0, &[mage_il_vec()])
        .hand(0, &[forest()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0));

    let before = engine.state().zones.list(ZoneLocation::Hand(p0)).len();
    let mage = on_battlefield(&engine, p0, mage_il_vec()).unwrap();
    kit::activate_object(&mut engine, p0, mage, 0);
    kit::settle(&mut engine, p0, |_, p| kit::choose_targets(p, &[], &[p1]));

    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
        before - 1
    );
    assert_eq!(engine.state().players[1].life, 19);
}
