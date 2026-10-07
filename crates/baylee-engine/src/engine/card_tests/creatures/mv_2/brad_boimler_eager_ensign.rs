//! `cards/creatures/mv_2/brad_boimler_eager_ensign.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Brad Boimler, Eager Ensign ({1}{W}) is a 2/2 legendary Human Officer with
/// lifelink, and the card is `Coverage::Partial` because the replacement
/// effect that would add an extra counter whenever he becomes tapped has no
/// DSL form — so the half there is to play is the keyword. Lifelink exists
/// only while damage is being dealt, so he attacks and the other seat blocks:
/// his two damage land on an Elf, which is worth exactly two life to his
/// controller, while the defending player's life total does not move. A
/// lifegain trigger, or damage that simply connected with a player, would
/// each satisfy only one of those two readings.
#[test]
fn brad_boimler_deals_his_damage_to_a_blocker_and_his_controller_gains_that_much_life() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(97, forest())
        .battlefield(0, &[brad_boimler_eager_ensign()])
        .battlefield(1, &[llanowar_elves()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let brad = on_battlefield(&engine, p0, brad_boimler_eager_ensign()).expect("he is out");
    let blocker = on_battlefield(&engine, p1, llanowar_elves()).expect("the Elf is out");
    assert_eq!(pt(&engine, brad), (2, 2), "the body the card prints");
    assert!(
        keywords(&engine, brad).contains(KeywordSet::LIFELINK),
        "the one line of this card that is implemented"
    );
    let (mine, theirs) = (
        engine.state().players[0].life,
        engine.state().players[1].life,
    );

    // Declare him as the only attacker, against the one defender a duel has.
    pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::ChooseAttackers { player, .. } if *player == p0),
    );
    let Pending::ChooseAttackers {
        attackers,
        defenders,
        ..
    } = engine.pending().clone()
    else {
        unreachable!("pass_until stopped on the declaration it was asked for")
    };
    assert!(
        attackers.contains(&brad),
        "nothing keeps a settled 2/2 out of combat: {attackers:?}"
    );
    assert_eq!(
        defenders.len(),
        1,
        "one surviving opponent, and no planeswalkers"
    );
    engine
        .apply(
            p0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(brad, defenders[0])],
            },
        )
        .unwrap();

    // The other seat blocks, so the damage lands on a creature rather than on
    // a player: lifelink is about damage, not about connecting.
    pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::ChooseBlockers { player, .. } if *player == p1),
    );
    let Pending::ChooseBlockers { blockers, .. } = engine.pending().clone() else {
        unreachable!("pass_until stopped on the declaration it was asked for")
    };
    assert!(
        blockers
            .iter()
            .any(|option| option.blocker == blocker && option.attackers.contains(&brad)),
        "a 1/1 may block a 2/2 with no evasion: {blockers:?}"
    );
    engine
        .apply(
            p1,
            PlayerAction::DeclareBlockers {
                blockers: vec![(blocker, brad)],
            },
        )
        .unwrap();

    pass_until(&mut engine, |e| e.state().players[0].life > mine);
    assert_eq!(
        engine.state().players[0].life,
        mine + 2,
        "lifelink: the two damage he dealt to the Elf are two life"
    );
    assert_eq!(
        engine.state().players[1].life,
        theirs,
        "and none of it reached the player, because the Elf blocked"
    );
}
