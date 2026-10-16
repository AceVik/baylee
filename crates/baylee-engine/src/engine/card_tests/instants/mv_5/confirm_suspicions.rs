//! `cards/instants/mv_5/confirm_suspicions.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

// oracle_id = "a30d076b-c942-45e4-b943-3749ce955291"
fn confirm_suspicions() -> CardIndex {
    card_index("a30d076b-c942-45e4-b943-3749ce955291")
}

/// Confirm Suspicions: "Counter target spell. Investigate three times."
///
/// Seat 0 casts a Gray Ogre off its Mountains and answers it with the counter
/// off its Islands. The Ogre ends in the graveyard and never on the
/// battlefield, and three Clue tokens, artifacts, take its place.
#[test]
fn confirm_suspicions_counters_a_spell_and_investigates_three_times() {
    let lands = [vec![mountain(); 3], vec![island(); 5]].concat();
    let mut engine = table(
        911,
        &[gray_ogre(), confirm_suspicions()],
        &lands,
        &[],
        &[],
    );
    let ogre = ogre_on_the_stack(&mut engine);
    assert!(my_tokens(&engine).is_empty(), "no Clue before the counter");

    tap_all_mana(&mut engine, me());
    cast_with_floating(&mut engine, me(), confirm_suspicions());
    let menu = aim_at(&mut engine, me(), ogre);
    assert!(menu.contains(&ogre), "the spell on the stack: {menu:?}");
    settle(&mut engine);

    assert!(in_graveyard(&engine, me(), gray_ogre()).is_some(), "countered");
    assert!(on_battlefield(&engine, me(), gray_ogre()).is_none());
    let clues = my_tokens(&engine);
    assert_eq!(clues.len(), 3, "investigate three times");
    for clue in clues {
        assert!(
            types(&engine, clue).contains(TypeSet::ARTIFACT),
            "a Clue is an artifact"
        );
    }
}
