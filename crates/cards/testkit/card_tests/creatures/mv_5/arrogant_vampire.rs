//! `cards/creatures/mv_5/arrogant_vampire.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Arrogant Vampire — {3}{B}{B}, a 4/3 Vampire with flying.
///
/// Flying is a rule about who may block, so the card is played into combat
/// rather than read: two ground bodies stand on p0's side and the engine's own
/// declare-blockers enumeration is asked about each one. The control is what
/// makes the absence say anything — the defending Elf is a legal block for the
/// ground body off the very same offer, so an empty pairing list cannot pass
/// for the keyword. The turn in between is what summoning sickness costs
/// (CR 302.6) and the four damage the defending seat loses is what the printed
/// line buys.
#[test]
#[allow(clippy::too_many_lines)] // one printed card, played end to end: the length is the card's
fn arrogant_vampire_flies_over_a_ground_blocker_for_four() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, swamp())
        .battlefield(
            0,
            &[
                swamp(),
                swamp(),
                swamp(),
                swamp(),
                swamp(),
                quiet_creature(),
            ],
        )
        .hand(0, &[arrogant_vampire()])
        .battlefield(1, &[llanowar_elves()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    // The mana is made first and the spell cast out of what is floating,
    // because `castable` is filtered through the pool: five untapped Swamps
    // pay nothing until they are tapped. The ground body is named as the
    // printing kept back because it is one of the two bodies the combat step
    // is about, and `tap_all_mana` would have spent its own `{T}`.
    tap_all_mana_but(&mut engine, p0, Some(quiet_creature()));
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        5,
        "five Swamps in the pool and the ground body untouched",
    );
    cast_with_floating(&mut engine, p0, arrogant_vampire());
    pass_until(&mut engine, stack_is_empty);

    let vampire = on_battlefield(&engine, p0, arrogant_vampire()).expect("the Vampire resolved");
    let ground = on_battlefield(&engine, p0, quiet_creature()).expect("the ground body is out");
    let elf = on_battlefield(&engine, p1, llanowar_elves()).expect("a blocker is out");
    assert_eq!(pt(&engine, vampire), (4, 3), "the printed 4/3 body");
    assert!(
        keywords(&engine, vampire).contains(KeywordSet::FLYING),
        "and the printed flying reaches the permanent",
    );
    assert!(
        !keywords(&engine, ground).contains(KeywordSet::FLYING),
        "the control has no evasion of its own",
    );

    // A creature that entered this turn cannot attack (CR 302.6), so the turn
    // it lands is spent getting to a combat step the Vampire may take part in.
    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);

    // Both attackers come out of the engine's own attack list, so neither is a
    // body the harness moved into combat by hand.
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    let Pending::ChooseAttackers {
        player, attackers, ..
    } = engine.pending().clone()
    else {
        unreachable!("the pass waited for exactly this")
    };
    assert_eq!(player, p0, "the combat step being read is p0's own");
    assert!(
        attackers.contains(&vampire) && attackers.contains(&ground),
        "both bodies are on the attack offer: {attackers:?}",
    );
    engine
        .apply(
            p0,
            PlayerAction::DeclareAttackers {
                attackers: vec![
                    (vampire, Defender::Player(p1)),
                    (ground, Defender::Player(p1)),
                ],
            },
        )
        .expect("both attackers came out of the list that offered them");

    // The block offer is the only place the keyword can be read: this engine
    // publishes the pairings, so what a 1/1 with no flying may block is that
    // enumeration and nothing else.
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseBlockers { .. })
    });
    let Pending::ChooseBlockers { blockers, .. } = engine.pending().clone() else {
        unreachable!("the pass waited for exactly this")
    };
    let may_block = |attacker: &ObjectId| {
        blockers
            .iter()
            .any(|option| option.blocker == elf && option.attackers.contains(attacker))
    };
    assert!(
        may_block(&ground),
        "the Elf is a legal block for the ground body, so the offer is not \
         simply empty: {blockers:?}",
    );
    assert!(
        !may_block(&vampire),
        "a creature with no flying may not block one that has it: {blockers:?}",
    );

    engine
        .apply(p1, PlayerAction::DeclareBlockers { blockers: vec![] })
        .expect("declaring no blockers is always legal");

    // Not `stack_is_empty`: the stack is already empty the moment the
    // attackers are declared, so that predicate would stop the walk before the
    // damage step and every life total would still read twenty.
    pass_until(&mut engine, |e| {
        matches!(e.state().turn.phase, Phase::Ending)
    });

    assert_eq!(
        engine.state().players[1].life,
        15,
        "four from the flier and one from the ground body — the four is the \
         whole of what the printed keyword earned",
    );
    assert!(
        on_battlefield(&engine, p1, llanowar_elves()).is_some(),
        "the Elf that could not block the flier was never asked to trade",
    );
}
