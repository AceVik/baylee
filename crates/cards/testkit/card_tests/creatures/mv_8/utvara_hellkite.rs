//! `cards/creatures/mv_8/utvara_hellkite.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn utvara_hellkite() -> CardIndex {
    card_index("05a6a571-643e-429e-8e5c-1c3f8b0dc746")
}

/// Utvara Hellkite is a 6/6 flier for {6}{R}{R}.
#[test]
fn utvara_hellkite_is_cast_for_eight_with_flying() {
    let mut engine = at_main(&[mountain(); 8], &[utvara_hellkite()]);
    let dragon = cast_and_settle(&mut engine, utvara_hellkite());
    assert_eq!(pt(&engine, dragon), (6, 6));
    assert!(keywords(&engine, dragon).contains(KeywordSet::FLYING));
}

/// "Whenever a Dragon you control attacks, create a 6/6 red Dragon creature
/// token with flying": attacking with Utvara itself makes one, and none
/// exists before the attack.
#[test]
fn utvara_hellkite_attacking_makes_a_six_six_flying_dragon() {
    let p0 = PlayerId::new(0);
    let mut engine = at_main(&[utvara_hellkite()], &[]);
    let dragon = on_battlefield(&engine, p0, utvara_hellkite()).expect("Utvara");
    assert!(tokens_of(&engine, p0).is_empty());

    attack_player(&mut engine, dragon);

    let tokens = tokens_of(&engine, p0);
    assert_eq!(tokens.len(), 1, "one token for one attacking Dragon");
    let token = tokens[0];
    assert_eq!(pt(&engine, token), (6, 6));
    assert!(keywords(&engine, token).contains(KeywordSet::FLYING));
    let printed = engine
        .state()
        .object(token)
        .and_then(|o| o.token)
        .expect("a token");
    assert_eq!(printed.name, "Dragon");
    assert!(printed.colors.contains(baylee_core::color::Color::Red));
}
