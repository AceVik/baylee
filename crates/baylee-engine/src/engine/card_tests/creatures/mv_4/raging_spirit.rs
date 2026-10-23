//! `cards/creatures/mv_4/raging_spirit.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::legends_kit as lk;
use baylee_core::color::{Color, ColorSet};

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

// oracle_id = "137e0ad2-e10a-4327-86a3-5ac7bc7fc469"
fn raging_spirit() -> CardIndex {
    card_index("137e0ad2-e10a-4327-86a3-5ac7bc7fc469")
}

#[test]
fn raging_spirit_becomes_colorless_until_end_of_turn() {
    let mut engine = lk::game(SEED, &[raging_spirit()], &[], &[], &[]);
    let spirit = lk::obj(&engine, P0, raging_spirit());
    assert_eq!(
        engine
            .state()
            .object(spirit)
            .unwrap()
            .characteristics()
            .colors,
        ColorSet::of(Color::Red)
    );

    lk::float(&mut engine, P0, &[(ManaColor::Colorless, 2)]);
    lk::use_ability(&mut engine, P0, raging_spirit(), 0, &[], &[]);

    assert_eq!(
        engine
            .state()
            .object(spirit)
            .unwrap()
            .characteristics()
            .colors,
        ColorSet::EMPTY
    );
}

/// "Until end of turn": the colour is red again in the opponent's turn.
#[test]
fn raging_spirit_is_red_again_after_the_turn_ends() {
    let mut engine = lk::game(SEED, &[raging_spirit()], &[], &[], &[]);
    let spirit = lk::obj(&engine, P0, raging_spirit());

    lk::float(&mut engine, P0, &[(ManaColor::Colorless, 2)]);
    lk::use_ability(&mut engine, P0, raging_spirit(), 0, &[], &[]);
    assert_eq!(
        engine
            .state()
            .object(spirit)
            .unwrap()
            .characteristics()
            .colors,
        ColorSet::EMPTY
    );

    reach_their_main_phase(&mut engine, P1);
    assert_eq!(
        engine
            .state()
            .object(spirit)
            .unwrap()
            .characteristics()
            .colors,
        ColorSet::of(Color::Red),
        "colourless only until end of turn"
    );
}
