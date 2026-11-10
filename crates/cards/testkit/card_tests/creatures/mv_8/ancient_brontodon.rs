//! `cards/creatures/mv_8/ancient_brontodon.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn ancient_brontodon() -> CardIndex {
    card_index("745be03c-69dc-4239-a5e7-38587b56eac0")
}

/// Ancient Brontodon is a vanilla 9/9 for {6}{G}{G}: eight Forests cast it,
/// and the creature that lands is the printed body with no ability on it.
#[test]
fn ancient_brontodon_is_cast_for_eight_as_a_vanilla_nine_nine() {
    let mut engine = at_main(&[forest(); 8], &[ancient_brontodon()]);
    let dino = cast_and_settle(&mut engine, ancient_brontodon());
    assert_eq!(pt(&engine, dino), (9, 9), "the printed body");
    assert!(types(&engine, dino).contains(TypeSet::CREATURE));
    assert!(
        keywords(&engine, dino).is_empty(),
        "a vanilla creature carries no keyword"
    );
}

/// The mana value is eight: seven Forests are one short.
#[test]
fn ancient_brontodon_cannot_be_cast_for_seven() {
    let mut engine = at_main(&[forest(); 7], &[ancient_brontodon()]);
    cannot_cast(&mut engine, ancient_brontodon());
}
