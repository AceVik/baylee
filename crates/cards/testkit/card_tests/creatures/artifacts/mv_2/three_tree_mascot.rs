//! `cards/creatures/artifacts/mv_2/three_tree_mascot.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn three_tree_mascot() -> CardIndex {
    card_index("6cb9d153-4eaa-4f32-8930-bd68cd99c128")
}

/// Three Tree Mascot: a 2/1 with "Changeling" and "{1}: Add one mana of any
/// color. Activate only once each turn."
#[test]
fn three_tree_mascot_makes_one_mana_of_any_color_once_each_turn() {
    once_each_turn_any_color(three_tree_mascot());
}

/// Changeling: it is every creature type, whatever its printed line says.
#[test]
fn three_tree_mascot_is_every_creature_type() {
    use baylee_core::generated::subtypes::creature;
    let p0 = PlayerId::new(0);
    let engine = table(&[], &[three_tree_mascot()], &[]);
    let me = on_battlefield(&engine, p0, three_tree_mascot()).expect("seated");
    assert_eq!(pt(&engine, me), (2, 1));
    assert!(keywords(&engine, me).contains(KeywordSet::CHANGELING));
    let chars = engine.state().object(me).unwrap().characteristics();
    for subtype in [
        creature::GOBLIN,
        creature::ELF,
        creature::WALL,
        creature::MYR,
    ] {
        assert!(chars.subtypes.contains(subtype), "every creature type");
    }
}
