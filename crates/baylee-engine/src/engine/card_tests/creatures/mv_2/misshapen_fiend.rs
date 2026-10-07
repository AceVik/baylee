//! `cards/creatures/mv_2/misshapen_fiend.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Misshapen Fiend is `{1}{B}` for a 1/1 Horror Mercenary whose whole printed
/// text is "Flying", so the only thing a game can hold it to is the combat
/// step: two Swamps pay for it, it arrives as a 1/1 with the keyword on its
/// *projected* characteristics, and a turn later it attacks across a table
/// holding both a grounded Llanowar Elves and a flying Baleful Strix — the
/// one creature that may block it and the one that may not.
///
/// The pairing the engine publishes for the block declaration is the card:
/// the Strix is offered against the Fiend and the Elf is not (CR 702.9b),
/// which is a reading nothing but flying produces — the Elf is untapped,
/// alive, and perfectly able to kill a 1/1, and still may not stand in front
/// of it. p1 falling to 19 is the same claim from the other end, and p0's
/// untouched 20 says the damage is the Fiend's and not a counterattack.
#[allow(clippy::too_many_lines)] // one cast, one full turn cycle, one attack
#[test]
fn misshapen_fiend_flies_over_the_ground_and_only_a_flier_may_block_it() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, swamp())
        .battlefield(0, &[swamp(), swamp()])
        .hand(0, &[misshapen_fiend()])
        // The control: one creature that may block a flier and one that may
        // not, both untapped and both able to kill a 1/1 in a fight.
        .battlefield(1, &[baleful_strix(), llanowar_elves()])
        .life(0, 20)
        .life(1, 20)
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    // `{1}{B}` off the two Swamps — nothing else on this board makes mana, so
    // an emptied pool afterwards is the whole printed cost and nothing more.
    cast_from_hand(&mut engine, p0, misshapen_fiend());
    pass_until(&mut engine, stack_is_empty);
    let fiend = on_battlefield(&engine, p0, misshapen_fiend()).expect("the Fiend resolved");
    assert_eq!(pt(&engine, fiend), (1, 1), "the body the card prints");
    assert!(
        keywords(&engine, fiend).contains(KeywordSet::FLYING),
        "and the one keyword it prints, seen through the layers: {:?}",
        keywords(&engine, fiend)
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "two black paid the {{1}}{{B}}, with nothing left over"
    );

    // A creature cast this turn is summoning sick (CR 302.6), so the attack
    // waits out a whole turn cycle: through p1's turn and back into p0's.
    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    let Pending::ChooseAttackers {
        player, attackers, ..
    } = engine.pending().clone()
    else {
        unreachable!("pass_until stops on nothing else")
    };
    assert_eq!(player, p0, "the active seat declares its attackers");
    assert!(
        attackers.contains(&fiend),
        "untapped and past summoning sickness, it may attack: {attackers:?}"
    );
    engine
        .apply(
            p0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(fiend, Defender::Player(p1))],
            },
        )
        .unwrap();

    // The block declaration, read rather than merely answered: this is the
    // engine's own statement about which creatures flying excludes.
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseBlockers { .. })
    });
    let Pending::ChooseBlockers {
        player, blockers, ..
    } = engine.pending().clone()
    else {
        unreachable!("pass_until stops on nothing else")
    };
    assert_eq!(player, p1, "the defending seat declares blockers");
    let strix = on_battlefield(&engine, p1, baleful_strix()).expect("the Strix is out");
    let elves = on_battlefield(&engine, p1, llanowar_elves()).expect("the Elf is out");
    assert!(
        blockers
            .iter()
            .any(|b| b.blocker == strix && b.attackers.contains(&fiend)),
        "a flier may block a flier: {blockers:?}"
    );
    assert!(
        !blockers
            .iter()
            .any(|b| b.blocker == elves && b.attackers.contains(&fiend)),
        "an untapped 1/1 that could kill the Fiend still may not block it \
         without flying (CR 702.9b): {blockers:?}"
    );
    engine
        .apply(p1, PlayerAction::DeclareBlockers { blockers: vec![] })
        .expect("declining to block is always legal");

    pass_until(&mut engine, |e| {
        matches!(e.state().turn.phase, Phase::Ending)
    });
    assert_eq!(
        engine.state().players[1].life,
        19,
        "the Fiend's one point of combat damage went through (CR 510.2)"
    );
    assert_eq!(
        engine.state().players[0].life,
        20,
        "and nothing came back: the damage is the Fiend's, not a trade"
    );
    assert!(
        on_battlefield(&engine, p0, misshapen_fiend()).is_some(),
        "nobody blocked it, so the Fiend is still on the battlefield"
    );
}
