//! `cards/artifacts/equipment/mv_2/glimmerlight.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn glimmerlight() -> CardIndex {
    card_index("8fd88b98-9899-4bc0-a4b7-7ef6fba36dfd")
}

/// Glimmerlight: "Equipped creature gets +1/+1. Equip {1}".
#[test]
fn glimmerlight_gives_plus_one_plus_one() {
    equipped_gets(
        glimmerlight(),
        1,
        llanowar_elves(),
        (1, 1),
        KeywordSet::EMPTY,
    );
}

/// "When this Equipment enters, create a 1/1 white Glimmer enchantment
/// creature token."
#[test]
fn glimmerlight_entering_creates_a_white_glimmer_token() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .hand(0, &[glimmerlight()])
        .battlefield(0, &[forest(), forest()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    assert!(
        tokens_of(&engine, p0).is_empty(),
        "no token before the cast"
    );
    cast_from_hand(&mut engine, p0, glimmerlight());
    pass_until(&mut engine, stack_is_empty);
    let tokens = tokens_of(&engine, p0);
    assert_eq!(tokens.len(), 1, "exactly one token");
    let glimmer = tokens[0];
    assert_eq!(pt(&engine, glimmer), (1, 1));
    let t = types(&engine, glimmer);
    assert!(
        t.contains(TypeSet::ENCHANTMENT) && t.contains(TypeSet::CREATURE),
        "{t:?}"
    );
    assert_eq!(
        colors_of(&engine, glimmer),
        baylee_core::color::ColorSet::of(baylee_core::color::Color::White)
    );
    assert!(
        subtypes_of(&engine, glimmer).contains(baylee_core::generated::subtypes::creature::GLIMMER)
    );
}
