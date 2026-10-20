//! `cards/instants/mv_1/gravebind.rs`, played.
//!
//! Oracle:
//! - "Target creature can't be regenerated this turn."
//! - "Draw a card at the beginning of the next turn's upkeep."

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::legends_kit as lk;
use crate::turn::Step;
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn gravebind() -> CardIndex {
    card_index("0ce01e24-42db-42db-9ba2-38f653383991")
}

#[test]
fn gravebind_prevents_regeneration_this_turn_and_draws_next_upkeep() {
    let mut engine = lk::game(
        SEED,
        &[],
        &[index::DRUDGE_SKELETONS],
        &[gravebind(), index::LIGHTNING_BOLT],
        &[],
    );
    let skeletons = lk::obj(&engine, P1, index::DRUDGE_SKELETONS);
    lk::float(&mut engine, P1, &[(ManaColor::Black, 1)]);
    lk::pass_once(&mut engine);
    lk::use_ability(&mut engine, P1, index::DRUDGE_SKELETONS, 0, &[], &[]);

    lk::float(
        &mut engine,
        P0,
        &[(ManaColor::Black, 1), (ManaColor::Red, 1)],
    );
    lk::cast_at(&mut engine, P0, gravebind(), &[skeletons], &[]);
    lk::cast_at(&mut engine, P0, index::LIGHTNING_BOLT, &[skeletons], &[]);
    assert_eq!(
        lk::zone_of(&engine, skeletons),
        Zone::Graveyard,
        "Gravebind stops regeneration so Drudge Skeletons dies"
    );

    assert!(in_hand(&engine, P0, index::FOREST).is_none());
    lk::to_step(&mut engine, P1, Step::Upkeep);
    lk::settle(&mut engine);
    assert!(in_hand(&engine, P0, index::FOREST).is_some());
}
