//! `cards/creatures/mv_5/rashida_scalebane.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::legends_kit as lk;
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

// oracle_id = "4aff0f09-0607-41b7-8f60-4ef35ff6183e"
fn rashida_scalebane() -> CardIndex {
    card_index("4aff0f09-0607-41b7-8f60-4ef35ff6183e")
}

#[test]
fn rashida_scalebane_destroys_attacking_dragon_and_gains_life_equal_to_its_power() {
    let mut engine = lk::game(
        SEED,
        &[rashida_scalebane()],
        &[index::SHIVAN_DRAGON],
        &[],
        &[],
    );
    let dragon = lk::obj(&engine, P1, index::SHIVAN_DRAGON);

    reach_their_main_phase(&mut engine, P1);
    let _offer = lk::attack(&mut engine, &[dragon]);
    lk::block(&mut engine, &[]);
    pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::Priority { player, .. } if *player == P0),
    );

    lk::use_ability(&mut engine, P0, rashida_scalebane(), 0, &[dragon], &[]);

    assert!(on_battlefield(&engine, P1, index::SHIVAN_DRAGON).is_none());
    assert_eq!(lk::life(&engine, P0), 25);
}
