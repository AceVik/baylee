//! `cards/lands/study_hall.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Study Hall prints `{{T}}: Add {{C}}.` and `{{1}}, {{T}}: Add one mana of any color.
/// When you spend this mana to cast your commander, scry X, where X is the number of
/// times it's been cast from the command zone this game.`
///
/// Under `Coverage::Partial`, the commander spend rider is omitted because commander
/// cast counts cannot be tracked. With floating mana from a Forest and Study Hall
/// kept untapped, activating ability 1 spends the floating mana, prompts for a color
/// choice via `Pending::ChooseColor`, adds one mana of the chosen color, and leaves
/// the land tapped.
#[test]
fn study_hall_filters_mana_to_any_color() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[study_hall(), forest()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let hall = on_battlefield(&engine, p0, study_hall()).expect("study hall on battlefield");

    // Tap Forest to float {G} without tapping Study Hall itself.
    tap_all_mana_but(&mut engine, p0, Some(study_hall()));
    assert_eq!(engine.state().players[0].mana_pool.total(), 1);
    assert!(!is_tapped(&engine, hall));

    activate(&mut engine, p0, study_hall(), 1);
    let Pending::ChooseColor { .. } = engine.pending().clone() else {
        panic!("expected ChooseColor, got {:?}", engine.pending());
    };
    engine
        .apply(p0, PlayerAction::ChooseColor(ManaColor::Blue))
        .unwrap();

    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.available(ManaColor::Blue), 1);
    assert_eq!(pool.available(ManaColor::Green), 0);
    assert_eq!(pool.total(), 1);
    assert!(is_tapped(&engine, hall));
}
