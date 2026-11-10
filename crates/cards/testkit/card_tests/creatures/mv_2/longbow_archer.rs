//! `cards/creatures/mv_2/longbow_archer.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Longbow Archer is `{W}{W}` for a 2/2 with reach and first strike, and a
/// keyword is worth nothing as a word in a card file: it is projected from the
/// card def only once the layers have run, so a test reading the def would pass
/// while the permanent on the table carried neither. So the Archer is cast for
/// its printed cost onto a board with a bare Elf beside it, and the pair is read
/// off the objects the layers project — the Archer with both keywords, the Elf
/// with neither. Reach is then exercised where it is a rule rather than a word:
/// the flying Baleful Strix across the table attacks, and the defender's own
/// block declaration is the only place "can block creatures with flying" lives —
/// the Archer is offered against the Strix and the untapped Elf beside it is not.
#[test]
fn longbow_archer_lands_with_reach_and_first_strike_and_blocks_a_flier() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[plains(), plains(), llanowar_elves()])
        .hand(0, &[longbow_archer()])
        .battlefield(1, &[baleful_strix()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    // `{W}{W}` out of the two Plains, with the Elf named as the printing kept
    // back: it is the control in the block declaration below, and a creature
    // tapped for its own mana may not block for the rest of the turn.
    tap_all_mana_but(&mut engine, p0, Some(llanowar_elves()));
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::White),
        2,
        "two Plains, two white, and the Elf tapped for nothing"
    );
    cast_with_floating(&mut engine, p0, longbow_archer());
    pass_until(&mut engine, stack_is_empty);

    let archer = on_battlefield(&engine, p0, longbow_archer()).expect("the Archer resolved");
    assert_eq!(pt(&engine, archer), (2, 2), "the body the card prints");
    let kw = keywords(&engine, archer);
    assert!(kw.contains(KeywordSet::REACH), "reach");
    assert!(kw.contains(KeywordSet::FIRST_STRIKE), "first strike");

    let elf = on_battlefield(&engine, p0, llanowar_elves()).expect("the Elf is out");
    let elf_keywords = keywords(&engine, elf);
    assert!(
        !elf_keywords.contains(KeywordSet::REACH)
            && !elf_keywords.contains(KeywordSet::FIRST_STRIKE),
        "the Elf beside it has neither of the Archer's keywords"
    );

    // Reach decides something only in a block declaration, so the flying Strix
    // attacks and p0's own offer is read: that list is the whole rule, and a
    // keyword that never reached it would leave the Archer unblockable.
    reach_their_main_phase(&mut engine, p1);
    let strix = on_battlefield(&engine, p1, baleful_strix()).expect("the Strix is out");
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    engine
        .apply(
            p1,
            PlayerAction::DeclareAttackers {
                attackers: vec![(strix, Defender::Player(p0))],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseBlockers { .. })
    });
    let Pending::ChooseBlockers {
        player, blockers, ..
    } = engine.pending().clone()
    else {
        unreachable!("pass_until only stops on the block declaration")
    };
    assert_eq!(player, p0, "the defending seat answers");
    let against_the_strix = blockers
        .iter()
        .find(|option| option.blocker == archer)
        .expect("reach lets the Archer block a creature with flying");
    assert!(
        against_the_strix.attackers.contains(&strix),
        "the Archer may block the Strix: {against_the_strix:?}"
    );
    assert!(
        blockers
            .iter()
            .all(|option| option.blocker != elf || !option.attackers.contains(&strix)),
        "an untapped Elf with neither reach nor flying may not: {blockers:?}"
    );
}
