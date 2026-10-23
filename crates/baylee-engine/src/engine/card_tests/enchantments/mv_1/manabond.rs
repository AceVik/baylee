//! `cards/enchantments/mv_1/manabond.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::partials_w1b_kit as kit;

fn manabond() -> CardIndex {
    card_index("482006a0-bb2a-4533-891b-2195da5e0f01")
}

#[test]
fn manabond_puts_land_onto_battlefield_and_discards_hand_at_end_step() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(1, forest())
        .battlefield(0, &[manabond()])
        .hand(0, &[forest(), gray_ogre()])
        .start();
    keep_mulligans(&mut engine);

    let land_card = in_hand(&engine, p0, forest()).unwrap();
    kit::drive(
        &mut engine,
        |e| on_battlefield(e, p0, forest()).is_some() && stack_is_empty(e),
        |_, p| {
            if matches!(p, Pending::YesNo { .. }) {
                Some(PlayerAction::YesNo(true))
            } else {
                kit::choose_cards(p, &[land_card])
            }
        },
    );

    assert!(on_battlefield(&engine, p0, forest()).is_some());
    assert!(in_hand(&engine, p0, gray_ogre()).is_none());
}
