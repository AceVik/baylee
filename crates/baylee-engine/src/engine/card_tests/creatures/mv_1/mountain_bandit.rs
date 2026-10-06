//! `cards/creatures/mv_1/mountain_bandit.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Mountain Bandit — {R} — 1/1 Human Soldier Rogue with haste, and nothing
/// else printed on it. Haste (CR 702.10b) is exactly the permission a
/// creature otherwise lacks (CR 302.6): it may attack the turn it arrives.
/// So two creatures land on the same turn off one tapping of a Mountain and
/// a Forest, and the *difference* between them in the attack declaration is
/// the whole proof — an equally untapped, equally new Elf beside it may not
/// attack while the Bandit may, and the 1/1 then connects for one.
#[test]
fn mountain_bandit_attacks_the_turn_it_lands_while_the_elf_beside_it_may_not() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[mountain(), forest()])
        .hand(0, &[mountain_bandit(), llanowar_elves()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    // One pool pays both costs: the Bandit's {R} and the Elf's {G}. Neither
    // creature is a mana source, so both stand untapped when combat comes.
    tap_all_mana(&mut engine, p0);
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        2,
        "the Mountain and the Forest, and nothing else on the board"
    );
    // One at a time: a creature is a sorcery-speed cast (CR 302.1), so the
    // second spell is illegal while the first is still on the stack. The pool
    // survives the resolution — mana empties at the end of a *step*, and
    // resolving a spell is not one.
    cast_with_floating(&mut engine, p0, mountain_bandit());
    pass_until(&mut engine, stack_is_empty);
    cast_with_floating(&mut engine, p0, llanowar_elves());
    pass_until(&mut engine, stack_is_empty);

    let bandit = on_battlefield(&engine, p0, mountain_bandit()).expect("the Bandit resolved");
    let elves = on_battlefield(&engine, p0, llanowar_elves()).expect("the Elf resolved");
    assert_eq!(pt(&engine, bandit), (1, 1), "the body the card prints");
    assert_eq!(pt(&engine, elves), (1, 1), "and the Elf is a 1/1 too");
    assert!(
        keywords(&engine, bandit).contains(KeywordSet::HASTE),
        "haste is the printed line"
    );
    assert!(
        !keywords(&engine, elves).contains(KeywordSet::HASTE),
        "and the Elf beside it has none"
    );

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    let Pending::ChooseAttackers { attackers, .. } = engine.pending().clone() else {
        unreachable!("pass_until stopped on the attack declaration")
    };
    assert!(
        attackers.contains(&bandit),
        "it entered this turn and is still offered: haste, and not a turn \
         that had already passed over it: {attackers:?}"
    );
    assert!(
        !attackers.contains(&elves),
        "the Elf entered in the same breath, is equally untapped, and is \
         still sick (CR 302.6): {attackers:?}"
    );

    engine
        .apply(
            p0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(bandit, Defender::Player(p1))],
            },
        )
        .unwrap();
    // Past the damage step, which an empty stack is not: CR 510.2 is what the
    // life total below is read through.
    pass_until(&mut engine, |e| {
        matches!(e.state().turn.phase, Phase::Ending)
    });
    assert_eq!(engine.state().players[1].life, 19, "the 1/1 connected");
    assert_eq!(engine.state().players[0].life, 20, "and nothing hit back");
}
