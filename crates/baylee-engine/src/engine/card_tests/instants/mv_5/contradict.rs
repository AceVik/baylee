//! `cards/instants/mv_5/contradict.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

// oracle_id = "a48f259a-99a0-49c7-a11b-6e796f8a12f4"
fn contradict() -> CardIndex {
    card_index("a48f259a-99a0-49c7-a11b-6e796f8a12f4")
}

/// Contradict: "Counter target spell. Draw a card."
#[test]
fn contradict_counters_a_spell_and_draws_a_card() {
    let lands = [vec![mountain(); 3], vec![island(); 5]].concat();
    let mut engine = table(913, &[gray_ogre(), contradict()], &lands, &[], &[]);
    let ogre = ogre_on_the_stack(&mut engine);

    tap_all_mana(&mut engine, me());
    let (hand_before, library_before) = (hand_len(&engine, me()), library_len(&engine, me()));
    cast_with_floating(&mut engine, me(), contradict());
    let menu = aim_at(&mut engine, me(), ogre);
    assert!(menu.contains(&ogre), "{menu:?}");
    settle(&mut engine);

    assert!(in_graveyard(&engine, me(), gray_ogre()).is_some(), "countered");
    assert!(on_battlefield(&engine, me(), gray_ogre()).is_none());
    assert_eq!(hand_len(&engine, me()), hand_before, "Contradict out, a card in");
    assert_eq!(library_len(&engine, me()), library_before - 1);
}
