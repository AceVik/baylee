//! `cards/creatures/mv_5/kodama_of_the_north_tree.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Kodama of the North Tree — {2}{G}{G}{G}, legendary Spirit, 6/4, printing
/// exactly two abilities: "Trample" and "Shroud (This creature can't be the
/// target of spells or abilities.)".
///
/// The body and both keywords are read off the permanent the spell left
/// behind, and shroud is *played* rather than looked up: the opponent's Swords
/// to Plowshares is cast over a board whose only other creature is a Llanowar
/// Elves, so the target question really is asked — the Elf's presence on the
/// menu is what proves it — and the Kodama is the one name missing from it. A
/// filter that had read "target creature" and stopped there would have offered
/// both, and each of the three lines alone is satisfied by a card that does
/// nothing.
#[test]
fn kodama_of_the_north_tree_is_a_six_four_with_trample_and_shroud() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                forest(),
                forest(),
                forest(),
                forest(),
                forest(),
                llanowar_elves(),
            ],
        )
        .hand(0, &[kodama_of_the_north_tree()])
        .battlefield(1, &[plains()])
        .hand(1, &[swords_to_plowshares()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    // Five Forests pay the {2}{G}{G}{G} and the Elves' own printed
    // "{T}: Add {G}" stands beside them — a mana creature is a mana route too
    // (#159), so the cast is not read as exactly the five lands.
    cast_from_hand(&mut engine, p0, kodama_of_the_north_tree());
    pass_until(&mut engine, |e| {
        stack_is_empty(e) && on_battlefield(e, p0, kodama_of_the_north_tree()).is_some()
    });
    let kodama = on_battlefield(&engine, p0, kodama_of_the_north_tree())
        .expect("the Kodama resolved onto the battlefield");
    assert_eq!(pt(&engine, kodama), (6, 4), "the body the card prints");
    assert!(
        types(&engine, kodama).contains(TypeSet::CREATURE),
        "a creature, as the printed type line reads"
    );
    assert!(
        engine
            .state()
            .object(kodama)
            .expect("the Kodama is an object")
            .characteristics()
            .supertypes
            .contains(SupertypeSet::LEGENDARY),
        "and the legendary one the header prints"
    );
    assert!(
        keywords(&engine, kodama).contains(KeywordSet::TRAMPLE),
        "the first printed line reaches the permanent"
    );
    assert!(
        keywords(&engine, kodama).contains(KeywordSet::SHROUD),
        "and the second one does — shroud is a keyword the layers project, not \
         a sentence a reader has to remember"
    );

    // The half of shroud only a spell can ask, on the board where a legal
    // target exists for it.
    reach_their_main_phase(&mut engine, p1);
    let elf = on_battlefield(&engine, p0, llanowar_elves()).expect("the Elf is still out");
    cast_from_hand(&mut engine, p1, swords_to_plowshares());
    let options = pass_until_targets(&mut engine, p1);
    assert!(
        options.contains(&elf),
        "the Elf is a creature with no shroud, so the question is a real one \
         and its menu is not empty: {options:?}"
    );
    assert!(
        !options.contains(&kodama),
        "\"This creature can't be the target of spells or abilities\": the \
         Swords reaches the Elf and never the Kodama: {options:?}"
    );

    engine
        .apply(p1, PlayerAction::ChooseObjects { objects: vec![elf] })
        .expect("the creature the question enumerated was chosen");
    pass_until(&mut engine, stack_is_empty);
    assert!(
        on_battlefield(&engine, p0, llanowar_elves()).is_none(),
        "the removal resolved against the creature it named"
    );
    assert!(
        on_battlefield(&engine, p0, kodama_of_the_north_tree()).is_some(),
        "and the Kodama — which it was never offered — is untouched"
    );
}
