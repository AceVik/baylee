//! `cards/creatures/mv_1/tree_monkey.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Tree Monkey is `{G}` for a 1/1 whose entire text is one keyword, so a
/// scenario has to give Reach something to do: a flier attacks and the
/// declare-blockers offer is read as a pairing (CR 509.1). The 1/1 Elf
/// beside it is the control that the same offer has to decline — without it,
/// "the Monkey may block the flier" would read just as true of a board where
/// every creature may block anything. Sphinx of the Final Word is the
/// attacker because it prints flying and no triggered ability of its own, so
/// nothing but the attack is being asked about.
#[test]
fn tree_monkey_casts_for_one_green_and_its_reach_blocks_a_flier() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest(), quiet_creature()])
        .hand(0, &[tree_monkey()])
        .battlefield(1, &[sphinx_of_the_final_word()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    // The Forest is the {G}; the Elf is kept standing, because a creature
    // tapped for mana may not block and it is this test's control in the
    // comparison below.
    tap_all_mana_but(&mut engine, p0, Some(quiet_creature()));
    cast_with_floating(&mut engine, p0, tree_monkey());
    pass_until(&mut engine, stack_is_empty);

    let monkey = on_battlefield(&engine, p0, tree_monkey()).expect("the Monkey resolved");
    let elf = on_battlefield(&engine, p0, quiet_creature()).expect("the Elf is still out");
    assert_eq!(pt(&engine, monkey), (1, 1), "the body the card prints");
    assert!(
        keywords(&engine, monkey).contains(KeywordSet::REACH),
        "the printed keyword reaches the permanent"
    );

    // Across the table and across a turn: the Sphinx has been under p1's
    // control since the game began, so nothing keeps it from attacking.
    pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::ChooseAttackers { player, .. } if *player == p1),
    );
    let sphinx =
        on_battlefield(&engine, p1, sphinx_of_the_final_word()).expect("the Sphinx is out");
    assert!(
        keywords(&engine, sphinx).contains(KeywordSet::FLYING),
        "the attacker has to be a flier or this proves nothing about reach"
    );
    let Pending::ChooseAttackers { attackers, .. } = engine.pending().clone() else {
        unreachable!("pass_until stops on nothing but the attack declaration")
    };
    assert!(
        attackers.contains(&sphinx),
        "an untapped flier may attack: {attackers:?}"
    );
    engine
        .apply(
            p1,
            PlayerAction::DeclareAttackers {
                attackers: vec![(sphinx, Defender::Player(p0))],
            },
        )
        .unwrap();

    pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::ChooseBlockers { player, .. } if *player == p0),
    );
    let Pending::ChooseBlockers { blockers, .. } = engine.pending().clone() else {
        unreachable!("the predicate just matched")
    };
    assert!(
        blockers
            .iter()
            .any(|b| b.blocker == monkey && b.attackers.contains(&sphinx)),
        "reach is the whole of what lets the Monkey block a creature with \
         flying (CR 702.17b): {blockers:?}"
    );
    assert!(
        !blockers
            .iter()
            .any(|b| b.blocker == elf && b.attackers.contains(&sphinx)),
        "while the 1/1 beside it, which prints no reach, is offered no flying \
         attacker at all: {blockers:?}"
    );
}
