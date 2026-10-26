//! `cards/artifacts/mv_2/distorting_lens.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::color::{Color, ColorSet};
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn subject_card() -> CardIndex {
    card_index("9c9579c5-7edf-41ca-8595-713ca9245576")
}

/// Distorting Lens — {2} Artifact:
/// "{T}: Target permanent becomes the color of your choice until end of turn."
#[test]
fn distorting_lens_changes_target_permanent_color_until_end_of_turn() {
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest(), forest(), index::GRIZZLY_BEARS])
        .hand(0, &[subject_card()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    cast_from_hand(&mut engine, P0, subject_card());
    pass_until(&mut engine, stack_is_empty);

    let bears = on_battlefield(&engine, P0, index::GRIZZLY_BEARS).unwrap();
    assert_eq!(
        engine
            .state()
            .object(bears)
            .unwrap()
            .characteristics()
            .colors,
        ColorSet::of(Color::Green)
    );

    activate(&mut engine, P0, subject_card(), 0);
    aim_at(&mut engine, P0, bears);
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseCastMode { .. })
    });
    let Pending::ChooseCastMode { options, .. } = engine.pending() else {
        unreachable!()
    };
    let blue = options
        .iter()
        .position(|o| o.kind == CastModeKind::Colors(ColorSet::of(Color::Blue)))
        .unwrap();
    engine.apply(P0, PlayerAction::ChooseMode(blue)).unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        engine
            .state()
            .object(bears)
            .unwrap()
            .characteristics()
            .colors,
        ColorSet::of(Color::Blue)
    );

    reach_their_main_phase(&mut engine, P1);
    assert_eq!(
        engine
            .state()
            .object(bears)
            .unwrap()
            .characteristics()
            .colors,
        ColorSet::of(Color::Green)
    );
}
