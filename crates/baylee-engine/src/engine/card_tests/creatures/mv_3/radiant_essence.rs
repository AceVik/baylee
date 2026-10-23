//! `cards/creatures/mv_3/radiant_essence.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::legends_kit as lk;
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

// oracle_id = "48e2f2c3-9d99-4535-915f-551fbb59ade5"
fn radiant_essence() -> CardIndex {
    card_index("48e2f2c3-9d99-4535-915f-551fbb59ade5")
}

#[test]
fn radiant_essence_gets_plus_1_plus_2_when_opponent_controls_a_black_permanent() {
    let mut engine = lk::game(SEED, &[radiant_essence()], &[], &[], &[index::BOG_IMP]);
    let essence = lk::obj(&engine, P0, radiant_essence());

    let ch_before = engine.state().object(essence).unwrap().characteristics();
    assert_eq!((ch_before.power, ch_before.toughness), (Some(2), Some(3)));

    reach_their_main_phase(&mut engine, P1);
    lk::float(
        &mut engine,
        P1,
        &[(ManaColor::Colorless, 2), (ManaColor::Black, 1)],
    );
    cast_with_floating(&mut engine, P1, index::BOG_IMP);
    lk::settle(&mut engine);

    let ch_after = engine.state().object(essence).unwrap().characteristics();
    assert_eq!((ch_after.power, ch_after.toughness), (Some(3), Some(5)));
}

/// "An opponent controls a black permanent": a non-black permanent of the
/// opponent's, and the controller's own black permanent, give nothing.
#[test]
fn radiant_essence_ignores_a_non_black_opposing_permanent_and_its_own_black_one() {
    let engine = lk::game(
        SEED,
        &[radiant_essence(), index::BLACK_KNIGHT],
        &[index::GRIZZLY_BEARS],
        &[],
        &[],
    );
    let essence = lk::obj(&engine, P0, radiant_essence());

    let ch = engine.state().object(essence).unwrap().characteristics();
    assert_eq!(
        (ch.power, ch.toughness),
        (Some(2), Some(3)),
        "a green opposing creature and an own black one do not count"
    );
}
