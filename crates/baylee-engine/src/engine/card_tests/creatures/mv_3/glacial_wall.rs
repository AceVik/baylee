//! `cards/creatures/mv_3/glacial_wall.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Glacial Wall is a `{2}{U}` 0/7 whose entire printed text is one keyword:
/// "Defender (This creature can't attack.)". The body and the keyword are read
/// on the battlefield, and the keyword is then played out in the combat step
/// that can see it — an untapped Elf beside it is offered in the attack
/// declaration and the Wall is not, which is what separates "Defender" from a
/// board that simply never reached CR 508.1. The Elf carries the second half
/// too: on the following turn the Wall is offered as its blocker, because
/// "can't attack" is not "can't block", and a creature that could do neither
/// would print the same keyword and pass the offer above.
#[test]
fn glacial_wall_is_a_zero_seven_that_cannot_attack_but_may_still_block() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, island())
        .battlefield(0, &[island(), island(), island(), llanowar_elves()])
        .hand(0, &[glacial_wall()])
        .battlefield(1, &[llanowar_elves()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    // Three Islands pay the {2}{U}, and the Elf is named as the printing kept
    // back: it is this test's control in the attack declaration below, and a
    // creature tapped for mana may not attack.
    let elf = on_battlefield(&engine, p0, llanowar_elves()).expect("the Elf is out");
    tap_all_mana_but(&mut engine, p0, Some(llanowar_elves()));
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        3,
        "three Islands and nothing else: the Elf kept its own {{G}} to itself"
    );
    cast_with_floating(&mut engine, p0, glacial_wall());
    pass_until(&mut engine, stack_is_empty);

    let wall = on_battlefield(&engine, p0, glacial_wall()).expect("the Wall resolved");
    assert_eq!(
        pt(&engine, wall),
        (0, 7),
        "the printed 0/7, projected on the battlefield"
    );
    assert!(
        keywords(&engine, wall).contains(KeywordSet::DEFENDER),
        "Defender reaches the permanent it is printed on"
    );

    // CR 508.1a enumerates what may attack, and the control is what makes the
    // absence readable: the untapped 1/1 is on that list and the Wall is not.
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    let Pending::ChooseAttackers { attackers, .. } = engine.pending().clone() else {
        unreachable!("pass_until stops on nothing but the attack declaration")
    };
    assert!(
        attackers.contains(&elf),
        "an untapped 1/1 with no text of its own may attack: {attackers:?}"
    );
    assert!(
        !attackers.contains(&wall),
        "CR 702.3: the 0/7 may not, whatever its body is worth in combat: {attackers:?}"
    );

    // Empty attackers on p0's side, the rest of that turn, and then p1's
    // combat: the same 0/7 is offered as a blocker, where the keyword says
    // nothing at all — and blocking is the only thing a Wall is for.
    pass_until(&mut engine, |e| {
        matches!(e.state().turn.phase, Phase::FirstMain) && e.state().turn.active == p1
    });
    let theirs = on_battlefield(&engine, p1, llanowar_elves()).expect("their Elf is out");
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    engine
        .apply(
            p1,
            PlayerAction::DeclareAttackers {
                attackers: vec![(theirs, Defender::Player(p0))],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseBlockers { .. })
    });
    let Pending::ChooseBlockers { blockers, .. } = engine.pending().clone() else {
        unreachable!("the predicate just matched")
    };
    let option = blockers
        .iter()
        .find(|o| o.blocker == wall)
        .expect("a creature with defender may still block");
    assert!(
        option.attackers.contains(&theirs),
        "and it may block the Elf attacking its controller: {option:?}"
    );
}
