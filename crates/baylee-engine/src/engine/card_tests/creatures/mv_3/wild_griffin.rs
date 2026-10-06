//! `cards/creatures/mv_3/wild_griffin.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Wild Griffin is one line of rules text — flying — on a `{2}{W}` 2/2 Griffin,
/// and each half of that is a different reading of the board. Three Plains pay
/// the printed cost, so the permanent that arrives is the one the card buys;
/// a 2/2 is what tells the body from the keyword, since a 1/1 with flying would
/// satisfy "it flies" just as well; and flying is read off the projected
/// characteristics rather than out of the card file, which is the only reading
/// the layer system can be wrong about. The last claim is the round trip: a turn
/// later the Griffin is a live, untapped creature the combat step offers, which
/// the arrival turn could not have shown because a permanent cast this turn is
/// summoning sick (CR 302.6).
#[test]
fn wild_griffin_arrives_as_a_flying_two_two_and_attacks_the_turn_after() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(4177, forest())
        .battlefield(0, &[plains(), plains(), plains()])
        .hand(0, &[wild_griffin()])
        // A creature across the table, so nothing on this side of the board
        // produces mana the cast did not pay for.
        .battlefield(1, &[llanowar_elves()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    cast_from_hand(&mut engine, p0, wild_griffin());
    pass_until(&mut engine, stack_is_empty);

    let griffin = on_battlefield(&engine, p0, wild_griffin()).expect("the Griffin resolved");
    assert!(
        types(&engine, griffin).contains(TypeSet::CREATURE),
        "what arrived is the creature the card prints: {:?}",
        types(&engine, griffin)
    );
    assert_eq!(
        pt(&engine, griffin),
        (2, 2),
        "a 2/2 and not a 1/1 the keyword alone would have passed for"
    );
    assert!(
        keywords(&engine, griffin).contains(KeywordSet::FLYING),
        "the one line of rules text the card prints, read off the layers"
    );

    // Across the opponent's turn and back: the arrival turn is the sick one,
    // so a combat offer on it would be reading a permanent that had only just
    // landed. This is the turn where the body has to be standing on its own.
    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    let Pending::ChooseAttackers { attackers, .. } = engine.pending().clone() else {
        unreachable!("pass_until stops on nothing else here")
    };
    assert!(
        attackers.contains(&griffin),
        "an untapped 2/2 under its controller since the turn began may attack: {attackers:?}"
    );
}

/// Mesa Pegasus attacking alone: flying keeps a creature without flying or
/// reach from blocking it at all, while a flier is offered normally — the
/// half of the card banding never touches.
#[test]
fn mesa_pegasus_flying_keeps_a_grounded_creature_from_blocking_it_alone() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[mesa_pegasus()])
        .battlefield(1, &[gray_ogre(), wild_griffin()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let pegasus = on_battlefield(&engine, p0, mesa_pegasus()).expect("seated");
    let ogre = on_battlefield(&engine, p1, gray_ogre()).expect("seated");
    let griffin = on_battlefield(&engine, p1, wild_griffin()).expect("seated");

    let blockers = attack_and_collect_blocks(&mut engine, pegasus, p1);
    assert!(
        !blockers.iter().any(|b| b.blocker == ogre),
        "no flying, no reach: the ogre is offered nothing at all: {blockers:?}"
    );
    assert!(
        blockers
            .iter()
            .any(|b| b.blocker == griffin && b.attackers.contains(&pegasus)),
        "a flier may still block it: {blockers:?}"
    );
}
