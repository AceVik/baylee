//! `cards/sorceries/mv_2/gaze_of_pain.rs`, played.
//!
//! Oracle:
//! - "Until end of turn, whenever a creature you control attacks and isn't blocked, you may choose to have it deal damage equal to its power to a target creature. If you do, it assigns no combat damage this turn."

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::legends_kit as lk;

const P0: PlayerId = PlayerId::new(0);

fn gaze_of_pain() -> CardIndex {
    card_index("b3a479d4-3e36-496b-abf8-b83231628d76")
}

#[test]
fn gaze_of_pain_casts_and_resolves_to_graveyard() {
    let mut engine = lk::game(SEED, &[], &[], &[gaze_of_pain()], &[]);
    let id = in_hand(&engine, P0, gaze_of_pain()).unwrap();
    lk::float(
        &mut engine,
        P0,
        &[(ManaColor::Colorless, 1), (ManaColor::Black, 1)],
    );
    lk::cast_at(&mut engine, P0, gaze_of_pain(), &[], &[]);
    assert_eq!(lk::zone_of(&engine, id), Zone::Graveyard);
}
