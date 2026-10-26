//! Tidal Visionary — {U} — Creature — Merfolk Wizard
//! Oracle: {T}: Target creature becomes the color of your choice until end of turn.

use super::*;
use baylee_core::color::{Color, ColorSet};
use baylee_core::generated::index;

#[test]
fn tidal_visionary_overwrites_target_creature_color_until_end_of_turn() {
    let visionary = card_index("8fc72080-8d4d-427d-ae30-065bc8ee9a28");
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, island())
        .battlefield(0, &[visionary, index::GRIZZLY_BEARS])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0));

    let vis_id = on_battlefield(&engine, p0, visionary).expect("Tidal Visionary on board");
    let bears_id = on_battlefield(&engine, p0, index::GRIZZLY_BEARS).unwrap();
    assert_eq!(
        engine
            .state()
            .object(bears_id)
            .unwrap()
            .characteristics()
            .colors,
        ColorSet::of(Color::Green)
    );

    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: vis_id,
                ability_index: 0,
            },
        )
        .unwrap();
    aim_at(&mut engine, p0, bears_id);
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseCastMode { .. })
    });
    let Pending::ChooseCastMode { options, .. } = engine.pending() else {
        panic!("expected ChooseCastMode for color choice");
    };
    let black = ColorSet::of(Color::Black);
    let mode_idx = options
        .iter()
        .find(|opt| opt.kind == crate::choice::CastModeKind::Colors(black))
        .expect("Black color option present")
        .index;
    engine
        .apply(
            p0,
            PlayerAction::ChooseMode(usize::try_from(mode_idx).unwrap()),
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        engine
            .state()
            .object(bears_id)
            .unwrap()
            .characteristics()
            .colors,
        black
    );

    assert!(walk_to_own_main(&mut engine, p1));
    assert_eq!(
        engine
            .state()
            .object(bears_id)
            .unwrap()
            .characteristics()
            .colors,
        ColorSet::of(Color::Green)
    );
}
