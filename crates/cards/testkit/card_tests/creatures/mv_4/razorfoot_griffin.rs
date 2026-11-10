//! `cards/creatures/mv_4/razorfoot_griffin.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Razorfoot Griffin prints four rules lines: flying, first strike, and the
/// 2/2 body of a `{3}{W}` creature. Neither keyword is visible in the card
/// file once the permanent is on the table, so the Griffin is cast and both
/// are read off the layer projection, with a Llanowar Elves standing beside it
/// as the control that they are the Griffin's own and not a board-wide grant.
///
/// The flying is then played rather than read. On the next turn the Griffin
/// and the Elf attack together, and the untapped Elf across the table is
/// offered as a blocker for the ground attacker and refused for the flier —
/// an empty offer under a lone flier could not have told that apart from a
/// tapped blocker or a combat step that never came.
#[test]
fn razorfoot_griffin_arrives_flying_and_first_striking_and_a_ground_creature_cannot_block_it() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, plains())
        .battlefield(
            0,
            &[plains(), plains(), plains(), plains(), quiet_creature()],
        )
        .hand(0, &[razorfoot_griffin()])
        .battlefield(1, &[quiet_creature()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    // Four Plains pay the {3}{W}. The Elf is named as the printing kept back
    // because it is the ground attacker the block offer below needs standing,
    // and `tap_all_mana` would have tapped it for its own {G}.
    tap_all_mana_but(&mut engine, p0, Some(quiet_creature()));
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        4,
        "four Plains, and the Elf contributed nothing"
    );
    cast_with_floating(&mut engine, p0, razorfoot_griffin());
    pass_until(&mut engine, stack_is_empty);

    let griffin = on_battlefield(&engine, p0, razorfoot_griffin()).expect("the Griffin resolved");
    let mine = on_battlefield(&engine, p0, quiet_creature()).expect("my Elf is out");
    let theirs = on_battlefield(&engine, p1, quiet_creature()).expect("their Elf is out");

    assert_eq!(pt(&engine, griffin), (2, 2), "the printed 2/2 body");
    let granted = keywords(&engine, griffin);
    assert!(granted.contains(KeywordSet::FLYING), "the printed flying");
    assert!(
        granted.contains(KeywordSet::FIRST_STRIKE),
        "the printed first strike"
    );
    let beside = keywords(&engine, mine);
    assert!(
        !beside.contains(KeywordSet::FLYING),
        "the keyword is the Griffin's own and no board-wide grant"
    );
    assert!(
        !beside.contains(KeywordSet::FIRST_STRIKE),
        "and the Elf beside it is the plain 1/1 it prints"
    );

    // CR 302.6: the Griffin cannot attack on the turn it arrived, so the
    // combat below is a turn later.
    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    let Pending::ChooseAttackers {
        player, attackers, ..
    } = engine.pending().clone()
    else {
        unreachable!("pass_until stops on nothing but the attack declaration")
    };
    assert!(
        attackers.contains(&griffin) && attackers.contains(&mine),
        "the Griffin has lost its summoning sickness and the Elf never had \
         any: {attackers:?}"
    );
    engine
        .apply(
            player,
            PlayerAction::DeclareAttackers {
                attackers: vec![
                    (griffin, Defender::Player(p1)),
                    (mine, Defender::Player(p1)),
                ],
            },
        )
        .expect("both came out of the list that offered them");

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseBlockers { .. })
    });
    let Pending::ChooseBlockers { blockers, .. } = engine.pending().clone() else {
        unreachable!("the pass waited for exactly the block declaration")
    };
    let offered = blockers
        .iter()
        .find(|o| o.blocker == theirs)
        .expect("the untapped Elf across the table is on the block offer");
    assert!(
        offered.attackers.contains(&mine),
        "a ground 1/1 may block a ground 1/1: {offered:?}"
    );
    assert!(
        !offered.attackers.contains(&griffin),
        "flying: a creature with neither flying nor reach may not block the \
         Griffin (CR 702.9b): {offered:?}"
    );
}
