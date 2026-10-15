//! `cards/creatures/artifacts/mv_2/salvaged_manaworker.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn salvaged_manaworker() -> CardIndex {
    card_index("a96814fe-dc74-4d02-a44e-f7daa794b647")
}

/// Salvaged Manaworker: a 1/3 with "{1}: Add one mana of any color. Activate
/// only once each turn."
#[test]
fn salvaged_manaworker_makes_one_mana_of_any_color_once_each_turn() {
    once_each_turn_any_color(salvaged_manaworker());
}

/// The printed body.
#[test]
fn salvaged_manaworker_is_a_one_three_artifact_creature() {
    let p0 = PlayerId::new(0);
    let engine = table(&[], &[salvaged_manaworker()], &[]);
    let me = on_battlefield(&engine, p0, salvaged_manaworker()).expect("seated");
    assert_eq!(pt(&engine, me), (1, 3));
    let t = types(&engine, me);
    assert!(t.contains(TypeSet::ARTIFACT) && t.contains(TypeSet::CREATURE));
}
