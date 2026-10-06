//! `cards/creatures/mv_4/harmattan_efreet.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Harmattan Efreet — {2}{U}{U} for a 2/2 flier, and "{1}{U}{U}: Target
/// creature gains flying until end of turn."
///
/// Both halves of that activated sentence need a witness of their own.
/// "Target creature" is *any* creature, so the ground Elf across the table is
/// the one named — the point of the line is handing evasion to a creature that
/// has none — while the Elf standing beside the Efreet under the same seat is
/// what tells a target from a board-wide grant. "Until end of turn" then wants
/// a turn to pass: the same board is read once more on the far side of the
/// cleanup step, where a static grant would still be sitting.
#[test]
#[allow(clippy::too_many_lines)]
fn harmattan_efreet_grants_flying_to_any_target_creature_until_the_turn_ends() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    // Seven Islands: {2}{U}{U} brings the Efreet to the table and {1}{U}{U} is
    // the ability, all in the one main phase a pool survives to (CR 500.5).
    let mut board = vec![island(); 7];
    board.push(llanowar_elves());
    let mut engine = Duel::new(SEED, island())
        .battlefield(0, &board)
        .hand(0, &[harmattan_efreet()])
        .battlefield(1, &[llanowar_elves()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let mine = on_battlefield(&engine, p0, llanowar_elves()).expect("my Elf is out");
    let theirs = on_battlefield(&engine, p1, llanowar_elves()).expect("their Elf is out");
    assert!(
        !keywords(&engine, theirs).contains(KeywordSet::FLYING),
        "a printed 1/1 is no flier before anything is granted"
    );

    // The Elves are named as the printing kept back: they are the creatures
    // the ability is about to choose between, and a mana creature tapped for
    // the cost would leave the pool a count of eight and the Elf read wrong
    // afterwards.
    tap_all_mana_but(&mut engine, p0, Some(llanowar_elves()));
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        7,
        "seven Islands tapped and the Elf contributed nothing"
    );
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Green),
        0,
        "the Elf is a mana creature and it was kept out of the pool"
    );
    cast_with_floating(&mut engine, p0, harmattan_efreet());
    pass_until(&mut engine, stack_is_empty);

    let efreet = on_battlefield(&engine, p0, harmattan_efreet()).expect("the Efreet resolved");
    assert_eq!(pt(&engine, efreet), (2, 2), "the body the card prints");
    assert!(
        keywords(&engine, efreet).contains(KeywordSet::FLYING),
        "and the printed flying reaches the permanent"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        3,
        "the cast's four are spent and the ability's three are still floating"
    );

    // `legal.abilities` is filtered through `can_afford`, which reads the
    // pool and not the untapped lands, so the mana has to be floating before
    // anything is claimed about the offer.
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        legal.abilities.contains(&(efreet, 0)),
        "with three blue floating the one line the card prints is offered: {:?}",
        legal.abilities
    );

    activate(&mut engine, p0, harmattan_efreet(), 0);
    let Pending::ChooseTargets {
        player, options, ..
    } = engine.pending().clone()
    else {
        panic!(
            "\"target creature\" is a target choice, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p0, "the activating seat aims it");
    assert!(
        options.contains(&mine) && options.contains(&theirs),
        "\"target creature\" is any creature, on either side of the table: {options:?}"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        3,
        "CR 601.2c before CR 601.2h: the cost is the last step, so the pool \
         stands untouched while the question is open"
    );

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![theirs],
            },
        )
        .expect("the Elf across the table was one of the options");
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "and the {{1}}{{U}}{{U}} came out of the pool"
    );
    assert!(
        !stack_is_empty(&engine),
        "granting a keyword is no mana ability, so the ability is on the stack"
    );

    pass_until(&mut engine, stack_is_empty);
    assert!(
        keywords(&engine, theirs).contains(KeywordSet::FLYING),
        "the creature the ability named gained flying"
    );
    assert!(
        !keywords(&engine, mine).contains(KeywordSet::FLYING),
        "the Elf nobody named is untouched: the effect targets, it does not \
         sweep the board"
    );
    assert!(
        keywords(&engine, efreet).contains(KeywordSet::FLYING),
        "and the Efreet's own printed flying is not the grant being read"
    );

    // "until end of turn": the cleanup step (CR 514.2) takes the grant away,
    // so the same Elf is a printed 1/1 again on the other side of it.
    reach_their_main_phase(&mut engine, p1);
    assert!(
        !keywords(&engine, theirs).contains(KeywordSet::FLYING),
        "the grant lasted the turn it was made in and no longer"
    );
    assert!(
        on_battlefield(&engine, p1, llanowar_elves()).is_some(),
        "and the creature is still standing, so the keyword left rather than \
         the creature"
    );
}
