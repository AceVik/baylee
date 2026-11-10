//! `cards/lands/tapland/arcane_sanctum.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// `Arcane Sanctum` is a tri-color tapland under `Coverage::Implemented`.
/// It prints "This land enters tapped." and "{T}: Add {W}, {U}, or {B}."
/// When played via `play_land_face`, it enters tapped, untaps on the subsequent turn,
/// and offers a choice among white, blue, and black mana.
#[test]
fn arcane_sanctum_enters_tapped_and_produces_three_colors() {
    let (mut engine, land) =
        play_land_face(arcane_sanctum(), 0).expect("Arcane Sanctum plays legally as face 0");
    assert!(is_tapped(&engine, land), "enters tapped on arrival");

    reach_their_main_phase(&mut engine, PlayerId::new(1));
    reach_their_main_phase(&mut engine, PlayerId::new(0));

    assert!(!is_tapped(&engine, land), "untaps on next turn");

    activate(&mut engine, PlayerId::new(0), arcane_sanctum(), 0);

    let Pending::ChooseColor { options, .. } = engine.pending().clone() else {
        panic!(
            "expected ChooseColor prompt for Arcane Sanctum, got {:?}",
            engine.pending()
        );
    };
    assert_eq!(
        options,
        vec![ManaColor::White, ManaColor::Blue, ManaColor::Black],
        "offers white, blue, and black mana"
    );

    engine
        .apply(PlayerId::new(0), PlayerAction::ChooseColor(ManaColor::Blue))
        .unwrap();

    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Blue),
        1,
        "added one blue mana to pool"
    );
    assert!(
        is_tapped(&engine, land),
        "Arcane Sanctum is tapped from producing mana"
    );
}
