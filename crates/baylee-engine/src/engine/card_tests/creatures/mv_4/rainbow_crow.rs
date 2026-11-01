//! Rainbow Crow — {3}{U} — Creature — Bird
//! Oracle: Flying
//! Oracle: {1}: This creature becomes the color of your choice until end of turn.

use super::*;
use baylee_core::color::{Color, ColorSet};

#[test]
fn rainbow_crow_flies_and_becomes_chosen_color_until_end_of_turn() {
    let crow = card_index("79bf98c8-1169-477d-838e-2ebc0fa396dc");
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, island())
        .battlefield(0, &[island(), island(), island(), island()])
        .hand(0, &[crow])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0));

    cast_from_hand(&mut engine, p0, crow);
    pass_until(&mut engine, stack_is_empty);

    let crow_id = on_battlefield(&engine, p0, crow).expect("Rainbow Crow on battlefield");
    assert!(keywords(&engine, crow_id).contains(KeywordSet::FLYING));
    assert_eq!(
        engine
            .state()
            .object(crow_id)
            .unwrap()
            .characteristics()
            .colors,
        ColorSet::from_slice(&[Color::Blue])
    );

    crate::engine::card_tests::legends_kit::float(&mut engine, p0, &[(ManaColor::Colorless, 1)]);
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: crow_id,
                ability_index: 0,
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseCastMode { .. })
    });
    engine.apply(p0, PlayerAction::ChooseMode(0)).unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        engine
            .state()
            .object(crow_id)
            .unwrap()
            .characteristics()
            .colors,
        ColorSet::from_slice(&[Color::White])
    );
}
