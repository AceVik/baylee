//! `cards/instants/mv_4/nocturnal_raid.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::legends_kit as lk;
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

// oracle_id = "5b4b08a5-3af8-4c33-ab5e-7bee7f79e77a"
fn nocturnal_raid() -> CardIndex {
    card_index("5b4b08a5-3af8-4c33-ab5e-7bee7f79e77a")
}

#[test]
fn nocturnal_raid_gives_all_black_creatures_plus_2_plus_0_until_end_of_turn() {
    let mut engine = lk::game(
        SEED,
        &[index::BOG_IMP, index::GRIZZLY_BEARS],
        &[index::BOG_IMP],
        &[nocturnal_raid()],
        &[],
    );
    let my_imp = lk::obj(&engine, P0, index::BOG_IMP);
    let my_bears = lk::obj(&engine, P0, index::GRIZZLY_BEARS);
    let opp_imp = lk::obj(&engine, P1, index::BOG_IMP);

    lk::float(
        &mut engine,
        P0,
        &[(ManaColor::Colorless, 2), (ManaColor::Black, 2)],
    );
    cast_with_floating(&mut engine, P0, nocturnal_raid());
    lk::settle(&mut engine);

    let my_imp_ch = engine.state().object(my_imp).unwrap().characteristics();
    assert_eq!((my_imp_ch.power, my_imp_ch.toughness), (Some(3), Some(1)));

    let opp_imp_ch = engine.state().object(opp_imp).unwrap().characteristics();
    assert_eq!((opp_imp_ch.power, opp_imp_ch.toughness), (Some(3), Some(1)));

    let bears_ch = engine.state().object(my_bears).unwrap().characteristics();
    assert_eq!((bears_ch.power, bears_ch.toughness), (Some(2), Some(2)));
}

/// "Until end of turn": the Imps are 1/1 again in the next turn.
#[test]
fn nocturnal_raid_wears_off_at_end_of_turn() {
    let mut engine = lk::game(SEED, &[index::BOG_IMP], &[], &[nocturnal_raid()], &[]);
    let imp = lk::obj(&engine, P0, index::BOG_IMP);

    lk::float(
        &mut engine,
        P0,
        &[(ManaColor::Colorless, 2), (ManaColor::Black, 2)],
    );
    cast_with_floating(&mut engine, P0, nocturnal_raid());
    lk::settle(&mut engine);
    let ch = engine.state().object(imp).unwrap().characteristics();
    assert_eq!((ch.power, ch.toughness), (Some(3), Some(1)));

    reach_their_main_phase(&mut engine, P1);
    let ch = engine.state().object(imp).unwrap().characteristics();
    assert_eq!((ch.power, ch.toughness), (Some(1), Some(1)));
}
