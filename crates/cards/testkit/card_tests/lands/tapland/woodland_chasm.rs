//! `cards/lands/tapland/woodland_chasm.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// `Woodland Chasm` is a snow land with subtypes Swamp and Forest under `Coverage::Implemented`.
/// It prints "This land enters tapped." and "{T}: Add {B} or {G}."
/// When played via `play_land_face`, it enters the battlefield tapped.
/// Passing to the next turn untaps it, allowing it to be tapped for either black or green mana.
#[test]
fn woodland_chasm_enters_tapped_and_taps_for_black_or_green() {
    let (mut engine, land) =
        play_land_face(woodland_chasm(), 0).expect("Woodland Chasm plays legally as face 0");
    assert!(is_tapped(&engine, land), "enters tapped on arrival");

    reach_their_main_phase(&mut engine, PlayerId::new(1));
    reach_their_main_phase(&mut engine, PlayerId::new(0));

    assert!(!is_tapped(&engine, land), "untaps on next turn");

    activate(&mut engine, PlayerId::new(0), woodland_chasm(), 0);

    let Pending::ChooseColor { options, .. } = engine.pending().clone() else {
        panic!(
            "expected ChooseColor prompt for Woodland Chasm, got {:?}",
            engine.pending()
        );
    };
    assert_eq!(
        options,
        vec![ManaColor::Black, ManaColor::Green],
        "offers choice between black and green mana"
    );

    engine
        .apply(
            PlayerId::new(0),
            PlayerAction::ChooseColor(ManaColor::Black),
        )
        .unwrap();

    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Black),
        1,
        "added one black mana to the pool"
    );
    assert!(
        is_tapped(&engine, land),
        "Woodland Chasm is now tapped from producing mana"
    );
}
