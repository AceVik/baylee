//! `cards/creatures/mv_3/horned_sliver.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Horned Sliver is a {2}{G} 2/2 Sliver with one printed sentence — "All
/// Sliver creatures have trample" — and that sentence takes three witnesses
/// to read. The Sliver itself is a Sliver creature, so the word is "all" and
/// not "another": the permanent printing the sentence has to carry trample.
/// The Elf beside it is a creature of the same seat that is not a Sliver, so
/// the subtype half of the filter has to be read rather than skipped, and the
/// Elf across the table says the grant is not "Sliver creatures you control"
/// either.
///
/// Then the keyword is spent, because a projection is not a rule in play. A
/// 2/2 with trample blocked by a 1/1 puts lethal damage on the blocker and
/// the other point over it: a board where the life total never moves would
/// satisfy every reading above while trample did nothing at all.
#[test]
fn horned_sliver_grants_trample_to_every_sliver_and_the_sliver_tramples_over_a_blocker() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[horned_sliver(), llanowar_elves()])
        .battlefield(1, &[llanowar_elves()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let sliver = on_battlefield(&engine, p0, horned_sliver()).expect("the Sliver is out");
    let mine = on_battlefield(&engine, p0, llanowar_elves()).expect("my Elf is out");
    let blocker = on_battlefield(&engine, p1, llanowar_elves()).expect("their Elf is out");
    assert!(
        keywords(&engine, sliver).contains(KeywordSet::TRAMPLE),
        "\"all Sliver creatures\" includes the Sliver printing the sentence"
    );
    assert!(
        !keywords(&engine, mine).contains(KeywordSet::TRAMPLE),
        "a creature of the same seat that is not a Sliver is declined"
    );
    assert!(
        !keywords(&engine, blocker).contains(KeywordSet::TRAMPLE),
        "and the static is not \"Sliver creatures you control\", so the Elf \
         across the table is declined too"
    );
    assert_eq!(
        pt(&engine, sliver),
        (2, 2),
        "the static grants a keyword and no body"
    );

    // Combat, so the keyword is spent rather than merely projected. Both
    // Elves stay home: the only attacker declared is the Sliver, and the
    // only blocker is the one across the table.
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    engine
        .apply(
            p0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(sliver, Defender::Player(p1))],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseBlockers { .. })
    });
    engine
        .apply(
            p1,
            PlayerAction::DeclareBlockers {
                blockers: vec![(blocker, sliver)],
            },
        )
        .unwrap();
    // Not `stack_is_empty`: the stack is empty before the combat damage step
    // too, so that predicate would stop the walk with every life total still
    // reading 20 (CR 510.2).
    pass_until(&mut engine, |e| {
        matches!(e.state().turn.phase, Phase::Ending)
    });

    assert_eq!(
        engine.state().players[1].life,
        19,
        "one damage kills the 1/1 blocker and the other tramples over it"
    );
    assert!(
        in_graveyard(&engine, p1, llanowar_elves()).is_some(),
        "the blocker took lethal damage"
    );
    assert!(
        on_battlefield(&engine, p0, horned_sliver()).is_some(),
        "and the 2/2 survived the one damage the Elf dealt back"
    );
}
