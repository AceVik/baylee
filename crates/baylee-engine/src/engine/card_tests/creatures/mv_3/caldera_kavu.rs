//! `cards/creatures/mv_3/caldera_kavu.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::partials_w2c_kit::power_toughness;
use baylee_core::color::{Color, ColorSet};

/// Caldera Kavu — {2}{R} Creature — Kavu 2/2
/// "{1}{B}: This creature gets +1/+1 until end of turn.
/// {G}: This creature becomes the color of your choice until end of turn."
#[test]
fn caldera_kavu_pumps_itself_and_becomes_chosen_color_until_end_of_turn() {
    let kavu = card_index("7cdcfbdf-5ec7-451b-8c96-f19d75a4d76c");
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(0, &[kavu, swamp(), swamp(), forest()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let kavu_id = on_battlefield(&engine, p0, kavu).expect("Caldera Kavu on battlefield");
    assert_eq!(power_toughness(&engine, kavu_id), (2, 2));
    assert_eq!(
        engine
            .state()
            .object(kavu_id)
            .unwrap()
            .characteristics()
            .colors,
        ColorSet::of(Color::Red)
    );

    tap_all_mana(&mut engine, p0);
    // Ability 0: {1}{B}: +1/+1 until end of turn.
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: kavu_id,
                ability_index: 0,
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(power_toughness(&engine, kavu_id), (3, 3));

    // Ability 1: {G}: becomes the color of your choice until end of turn.
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: kavu_id,
                ability_index: 1,
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseCastMode { .. })
    });
    let Pending::ChooseCastMode { options, .. } = engine.pending() else {
        panic!("expected ChooseCastMode for color choice");
    };
    let blue = ColorSet::of(Color::Blue);
    let mode_idx = options
        .iter()
        .find(|opt| opt.kind == crate::choice::CastModeKind::Colors(blue))
        .expect("Blue color option present")
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
            .object(kavu_id)
            .unwrap()
            .characteristics()
            .colors,
        blue
    );

    assert!(walk_to_own_main(&mut engine, p1));
    assert_eq!(power_toughness(&engine, kavu_id), (2, 2));
    assert_eq!(
        engine
            .state()
            .object(kavu_id)
            .unwrap()
            .characteristics()
            .colors,
        ColorSet::of(Color::Red)
    );
}
