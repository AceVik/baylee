//! `cards/creatures/mv_7/plated_slagwurm.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

// oracle_id = "4aec7624-e406-45a4-b2b6-2e8d29f6268a"

/// Plated Slagwurm is a `{4}{G}{G}{G}` 8/8 whose only text is hexproof —
/// "This creature can't be the target of spells or abilities **your
/// opponents** control" — and that clause has two halves that only one board
/// can tell apart. The Wurm is cast for seven of ten mana and read as the
/// printed 8/8 carrying the keyword; its controller's own Giant Growth is then
/// offered it as a target, because "your opponents" is the word; and on the
/// following turn the opponent's Swords to Plowshares publishes a menu holding
/// the Llanowar Elves and **not** the Wurm. The Elves are the control: an empty
/// menu would satisfy "the Wurm is not on it" for a reason that has nothing to
/// do with hexproof.
#[test]
fn plated_slagwurm_is_an_eight_eight_only_its_opponents_cannot_aim_at() {
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
                forest(),
                forest(),
                forest(),
                forest(),
                llanowar_elves(),
            ],
        )
        .hand(0, &[plated_slagwurm(), giant_growth()])
        .battlefield(1, &[plains(), plains(), plains(), plains()])
        .hand(1, &[swords_to_plowshares()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    // Nine Forests and the Elf's own printed `{T}: Add {G}` are ten mana in
    // this one main phase: the seven the Wurm prints and the one the Growth
    // charges, with two to spare.
    tap_all_mana(&mut engine, p0);
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        10,
        "nine Forests and one Llanowar Elves, which is a mana source too"
    );
    cast_with_floating(&mut engine, p0, plated_slagwurm());
    pass_until(&mut engine, |e| {
        on_battlefield(e, p0, plated_slagwurm()).is_some()
    });

    let wurm = on_battlefield(&engine, p0, plated_slagwurm()).expect("the Wurm resolved");
    assert!(
        types(&engine, wurm).contains(TypeSet::CREATURE),
        "it is the creature the card prints"
    );
    assert_eq!(pt(&engine, wurm), (8, 8), "the printed 8/8 body");
    assert!(
        keywords(&engine, wurm).contains(KeywordSet::HEXPROOF),
        "the printed hexproof reaches the permanent"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        3,
        "ten mana less the {{4}}{{G}}{{G}}{{G}} the Wurm actually cost"
    );

    // Hexproof names *your opponents'* spells, so the controller's own Growth
    // must still be offered the Wurm as a target — the half a reading that
    // stopped at "can't be the target" and never read the rest would lose.
    cast_with_floating(&mut engine, p0, giant_growth());
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    let mine = aim_at(&mut engine, p0, wurm);
    assert!(
        mine.contains(&wurm),
        "\"spells or abilities your opponents control\" is the whole clause, \
         so my own Growth may aim at it: {mine:?}"
    );
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        pt(&engine, wurm),
        (11, 11),
        "+3/+3 from a spell the Wurm was a legal target for"
    );

    // The other seat's turn, where the same Wurm has to be off the menu. The
    // Elves are the control the claim needs: a target list that was empty
    // would satisfy "the Wurm is not on it" for a reason that is not hexproof.
    reach_their_main_phase(&mut engine, p1);
    let elf = on_battlefield(&engine, p0, llanowar_elves()).expect("the Elf is still standing");
    cast_from_hand(&mut engine, p1, swords_to_plowshares());
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    let theirs = aim_at(&mut engine, p1, elf);
    assert_eq!(
        theirs,
        vec![elf],
        "one creature on this board is a legal target for the opponent's \
         Swords, and it is not the Wurm: {theirs:?}"
    );
    assert!(
        !theirs.contains(&wurm),
        "\"This creature can't be the target of spells or abilities your \
         opponents control\""
    );
    pass_until(&mut engine, stack_is_empty);

    assert!(
        on_battlefield(&engine, p0, llanowar_elves()).is_none(),
        "the Swords resolved against the creature it was allowed to name"
    );
    assert!(
        on_battlefield(&engine, p0, plated_slagwurm()).is_some(),
        "and the Wurm it could not name is still on the battlefield"
    );
    assert!(
        in_graveyard(&engine, p0, plated_slagwurm()).is_none(),
        "nothing moved it anywhere: the Swords was aimed at the Elf"
    );
}
