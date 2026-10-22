//! `cards/artifacts/mv_4/whirling_catapult.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index::{AIR_ELEMENTAL, GRIZZLY_BEARS};

const WHIRLING_CATAPULT: &str = "0987b5e9-0012-4189-8745-45f10e9557f3";

#[test]
fn whirling_catapult_deals_one_damage_to_each_flyer_and_each_player() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let card = card_index(WHIRLING_CATAPULT);
    let flyer_card = AIR_ELEMENTAL;
    let mut engine = Duel::new(SEED, forest())
        .hand(0, &[card])
        .battlefield(
            0,
            &[
                forest(),
                forest(),
                forest(),
                forest(),
                forest(),
                forest(),
                GRIZZLY_BEARS,
            ],
        )
        .battlefield(1, &[flyer_card])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    cast_from_hand(&mut engine, p0, card);
    pass_until(&mut engine, stack_is_empty);

    let bear = on_battlefield(&engine, p0, GRIZZLY_BEARS).unwrap();
    let flyer = on_battlefield(&engine, p1, flyer_card).unwrap();

    tap_all_mana(&mut engine, p0);
    activate(&mut engine, p0, card, 0);
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(engine.state().object(flyer).unwrap().damage, 1);
    assert_eq!(engine.state().object(bear).unwrap().damage, 0);
    assert_eq!(engine.state().players[0].life, 19);
    assert_eq!(engine.state().players[1].life, 19);
}
