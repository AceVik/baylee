//! `cards/sorceries/mv_2/kaervek_s_purge.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::legends_kit as lk;
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

// oracle_id = "daa86c76-aacb-4244-9639-81f758c634b2"
fn kaervek_s_purge() -> CardIndex {
    card_index("daa86c76-aacb-4244-9639-81f758c634b2")
}

#[test]
fn kaervek_s_purge_destroys_creature_with_mana_value_x_and_deals_its_power_to_controller() {
    let mut engine = lk::game(SEED, &[], &[index::HILL_GIANT], &[kaervek_s_purge()], &[]);
    let giant = lk::obj(&engine, P1, index::HILL_GIANT);

    lk::float(
        &mut engine,
        P0,
        &[
            (ManaColor::Colorless, 4),
            (ManaColor::Black, 1),
            (ManaColor::Red, 1),
        ],
    );
    cast_with_floating(&mut engine, P0, kaervek_s_purge());
    engine.apply(P0, PlayerAction::ChooseNumber(4)).unwrap();
    lk::aim(&mut engine, &[giant], &[]);
    pass_until(&mut engine, stack_is_empty);

    assert!(
        on_battlefield(&engine, P1, index::HILL_GIANT).is_none(),
        "Hill Giant (mana value 4) is destroyed"
    );
    assert_eq!(
        engine.state().players[1].life,
        17,
        "Hill Giant's controller takes 3 damage equal to its power"
    );
}
