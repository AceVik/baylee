//! `cards/creatures/mv_3/wind_drake.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Wind Drake — {2}{U} — Creature — Drake, 2/2, and the whole of its printed
/// text is "Flying". A one-word card has to be read where the word acts, so
/// this plays it in three places: three Islands pay for it and it arrives as
/// the printed 2/2, the next turn it attacks beside a ground Elf, and the
/// block declaration that follows pairs that Elf with the Elf across the table
/// while leaving the Drake out of every pairing (CR 702.9b). The pair on the
/// ground is the control that keeps the second half honest — the offer is
/// about *this* attack, so the Drake's absence is the evasion rule and not an
/// empty list — and the two Elves, which have no flying of their own, say the
/// keyword comes off the Drake rather than off the projection.
#[allow(clippy::too_many_lines)] // One printed card, played end to end: the length is the card's.
#[test]
fn wind_drake_arrives_as_a_two_two_flier_a_ground_creature_cannot_block() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, island())
        .battlefield(0, &[island(), island(), island(), llanowar_elves()])
        .battlefield(1, &[llanowar_elves()])
        .hand(0, &[wind_drake()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    // Three Islands into the pool, with the Elf named as the source kept back:
    // it is one of the attackers the declaration below is read against, and a
    // creature tapped for mana may not attack.
    tap_all_mana_but(&mut engine, p0, Some(llanowar_elves()));
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        3,
        "three Islands, and the Elf stayed untapped: {{2}}{{U}} is payable"
    );
    cast_with_floating(&mut engine, p0, wind_drake());
    pass_until(&mut engine, |e| at_rest(e, p0));

    let drake = on_battlefield(&engine, p0, wind_drake()).expect("the Drake resolved");
    assert_eq!(pt(&engine, drake), (2, 2), "the body the card prints");
    assert!(
        types(&engine, drake).contains(TypeSet::CREATURE),
        "and the type line it prints"
    );
    assert!(
        keywords(&engine, drake).contains(KeywordSet::FLYING),
        "the one line it prints reaches the permanent through the layers"
    );
    let ours = on_battlefield(&engine, p0, llanowar_elves()).expect("my Elf is out");
    let theirs = on_battlefield(&engine, p1, llanowar_elves()).expect("their Elf is out");
    assert!(
        !keywords(&engine, ours).contains(KeywordSet::FLYING)
            && !keywords(&engine, theirs).contains(KeywordSet::FLYING),
        "the control: neither Elf has flying of its own, so the keyword above \
         is the Drake's"
    );

    // Cast this turn is summoning sick (CR 302.6). The offer holds the Elf
    // under the same seat, so an offer without the Drake is the rule rather
    // than an empty list.
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    let Pending::ChooseAttackers { attackers, .. } = engine.pending().clone() else {
        unreachable!("pass_until stops on the attack declaration")
    };
    assert!(
        attackers.contains(&ours),
        "the Elf has been under its controller's control since the turn began \
         and may attack: {attackers:?}"
    );
    assert!(
        !attackers.contains(&drake),
        "the Drake was cast this turn and may not: {attackers:?}"
    );
    engine
        .apply(p0, PlayerAction::DeclareAttackers { attackers: vec![] })
        .unwrap();

    // A turn later: the untap step has been through the Drake, so nothing on
    // this board is sick any more.
    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);
    assert!(
        !is_tapped(&engine, drake),
        "the untap step ran and the Drake is standing"
    );
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    let Pending::ChooseAttackers { attackers, .. } = engine.pending().clone() else {
        unreachable!("pass_until stops on the attack declaration")
    };
    assert!(
        attackers.contains(&drake) && attackers.contains(&ours),
        "the flier and the ground Elf both attack: {attackers:?}"
    );
    engine
        .apply(
            p0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(drake, Defender::Player(p1)), (ours, Defender::Player(p1))],
            },
        )
        .unwrap();

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseBlockers { .. })
    });
    let Pending::ChooseBlockers { blockers, .. } = engine.pending().clone() else {
        unreachable!("pass_until stops on the block declaration")
    };
    assert!(
        blockers
            .iter()
            .any(|o| o.blocker == theirs && o.attackers.contains(&ours)),
        "the Elf across the table is offered the ground attacker, so the \
         question is about this combat and not an empty offer: {blockers:?}"
    );
    assert!(
        blockers
            .iter()
            .all(|o| o.blocker != theirs || !o.attackers.contains(&drake)),
        "\"flying\": the same untapped Elf is offered no way to block the \
         Drake (CR 702.9b): {blockers:?}"
    );
    engine
        .apply(p1, PlayerAction::DeclareBlockers { blockers: vec![] })
        .unwrap();

    pass_until(&mut engine, |e| {
        matches!(e.state().turn.phase, Phase::Ending)
    });
    assert_eq!(
        engine.state().players[1].life,
        17,
        "two from the unblocked Drake and one from the unblocked Elf (CR 510.2)"
    );
}
