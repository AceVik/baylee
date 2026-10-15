//! `cards/creatures/mv_8/godsire.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn godsire() -> CardIndex {
    card_index("4252d7f4-c582-4a21-b3af-829c84cb0b09")
}

/// Godsire is an 8/8 for {4}{R}{G}{G}{W} with vigilance.
#[test]
fn godsire_is_cast_for_eight_with_vigilance() {
    let mut engine = at_main(
        &[
            forest(),
            forest(),
            forest(),
            forest(),
            forest(),
            forest(),
            mountain(),
            plains(),
        ],
        &[godsire()],
    );
    let beast = cast_and_settle(&mut engine, godsire());
    assert_eq!(pt(&engine, beast), (8, 8));
    assert!(keywords(&engine, beast).contains(KeywordSet::VIGILANCE));
}

/// "{T}: Create an 8/8 Beast creature token that's red, green, and white":
/// one token, 8/8, in exactly those colours, and Godsire is tapped for it.
#[test]
fn godsire_taps_for_an_eight_eight_red_green_white_beast() {
    let p0 = PlayerId::new(0);
    let mut engine = at_main(&[godsire()], &[]);
    let beast = on_battlefield(&engine, p0, godsire()).expect("Godsire");
    assert!(tokens_of(&engine, p0).is_empty());

    activate(&mut engine, p0, godsire(), 0);
    pass_until(&mut engine, stack_is_empty);

    assert!(is_tapped(&engine, beast), "{{T}} is the cost");
    let tokens = tokens_of(&engine, p0);
    assert_eq!(tokens.len(), 1, "one token");
    let token = tokens[0];
    assert_eq!(pt(&engine, token), (8, 8));
    assert!(types(&engine, token).contains(TypeSet::CREATURE));
    let printed = engine
        .state()
        .object(token)
        .and_then(|o| o.token)
        .expect("it is a token");
    assert_eq!(printed.name, "Beast");
    for color in [
        baylee_core::color::Color::Red,
        baylee_core::color::Color::Green,
        baylee_core::color::Color::White,
    ] {
        assert!(
            printed.colors.contains(color),
            "{color:?} in {:?}",
            printed.colors
        );
    }
    assert!(!printed.colors.contains(baylee_core::color::Color::Blue));
}

/// Vigilance: attacking does not tap Godsire.
#[test]
fn godsire_attacks_without_tapping() {
    let p0 = PlayerId::new(0);
    let mut engine = at_main(&[godsire()], &[]);
    let beast = on_battlefield(&engine, p0, godsire()).expect("Godsire");
    attack_player(&mut engine, beast);
    assert!(!is_tapped(&engine, beast));
}
