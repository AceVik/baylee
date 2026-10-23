//! `cards/instants/mv_1/prismatic_lace.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::legends_kit as lk;
use baylee_core::color::{Color, ColorSet};
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

// oracle_id = "d4a3c9a5-3c91-4135-a19d-98eb5bcf5e00"
fn prismatic_lace() -> CardIndex {
    card_index("d4a3c9a5-3c91-4135-a19d-98eb5bcf5e00")
}

#[test]
fn prismatic_lace_changes_target_permanents_colors_indefinitely() {
    let mut engine = lk::game(SEED, &[index::GRIZZLY_BEARS], &[], &[prismatic_lace()], &[]);
    let bears = lk::obj(&engine, P0, index::GRIZZLY_BEARS);

    lk::float(&mut engine, P0, &[(ManaColor::Blue, 1)]);
    cast_with_floating(&mut engine, P0, prismatic_lace());
    lk::aim(&mut engine, &[bears], &[]);
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseCastMode { .. })
    });
    let wanted = ColorSet::of(Color::Red).union(ColorSet::of(Color::White));
    let Pending::ChooseCastMode { options, .. } = engine.pending() else {
        unreachable!();
    };
    let mode_idx = options
        .iter()
        .position(|o| o.kind == CastModeKind::Colors(wanted))
        .unwrap();
    engine
        .apply(P0, PlayerAction::ChooseMode(mode_idx))
        .unwrap();
    lk::settle(&mut engine);

    assert_eq!(
        engine
            .state()
            .object(bears)
            .unwrap()
            .characteristics()
            .colors,
        wanted
    );
}

/// "Target permanent … indefinitely": a land of the opponent's is a legal
/// target too, and the colour is still there after the turn is over.
#[test]
fn prismatic_lace_colours_an_opponents_land_and_keeps_it_past_the_turn() {
    let mut engine = lk::game(SEED, &[], &[plains()], &[prismatic_lace()], &[]);
    let land = lk::obj(&engine, P1, plains());
    assert_eq!(
        engine
            .state()
            .object(land)
            .unwrap()
            .characteristics()
            .colors,
        ColorSet::EMPTY
    );

    lk::float(&mut engine, P0, &[(ManaColor::Blue, 1)]);
    cast_with_floating(&mut engine, P0, prismatic_lace());
    lk::aim(&mut engine, &[land], &[]);
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseCastMode { .. })
    });
    let wanted = ColorSet::of(Color::Blue);
    let Pending::ChooseCastMode { options, .. } = engine.pending() else {
        unreachable!();
    };
    let mode_idx = options
        .iter()
        .position(|o| o.kind == CastModeKind::Colors(wanted))
        .unwrap();
    engine
        .apply(P0, PlayerAction::ChooseMode(mode_idx))
        .unwrap();
    lk::settle(&mut engine);
    assert_eq!(
        engine
            .state()
            .object(land)
            .unwrap()
            .characteristics()
            .colors,
        wanted
    );

    reach_their_main_phase(&mut engine, P1);
    assert_eq!(
        engine
            .state()
            .object(land)
            .unwrap()
            .characteristics()
            .colors,
        wanted,
        "the effect lasts indefinitely, not until end of turn"
    );
}
