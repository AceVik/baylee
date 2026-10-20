//! `cards/creatures/mv_3/flow_of_maggots.rs`, played.
//!
//! Oracle:
//! - "Cumulative upkeep {1}"
//! - "This creature can't be blocked by non-Wall creatures."

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::legends_kit as lk;
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn flow_of_maggots() -> CardIndex {
    card_index("2d6a387f-7ad4-419e-951a-9dcd4c9ac823")
}

#[test]
fn flow_of_maggots_cant_be_blocked_by_non_wall_and_has_cumulative_upkeep() {
    let mut engine = lk::game_with(
        SEED,
        &[flow_of_maggots()],
        &[index::GRIZZLY_BEARS, index::WALL_OF_WOOD],
        &[],
        &[],
        |e| {
            lk::float(e, P0, &[(ManaColor::Colorless, 1)]);
            lk::yes_no(e, true);
        },
    );
    let maggots = lk::obj(&engine, P0, flow_of_maggots());
    let bears = lk::obj(&engine, P1, index::GRIZZLY_BEARS);
    let wall = lk::obj(&engine, P1, index::WALL_OF_WOOD);
    assert_eq!(
        engine
            .state()
            .object(maggots)
            .unwrap()
            .counters
            .get(baylee_cards_dsl::counters::AGE),
        1
    );

    let offer = lk::attack(&mut engine, &[maggots]);
    assert!(
        !lk::may_block(&offer, bears, maggots),
        "non-Wall Grizzly Bears cannot block Flow of Maggots"
    );
    assert!(
        lk::may_block(&offer, wall, maggots),
        "Wall of Wood can block Flow of Maggots"
    );
}
