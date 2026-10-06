//! `cards/creatures/mv_2/bay_falcon.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Bay Falcon — {1}{U}, a 1/1 blue Bird whose entire text is "Flying,
/// vigilance", so both words have to be read out of the combat step rather
/// than off the card file. Vigilance is a body that declares itself an
/// attacker and is still standing upright afterwards, with an ordinary
/// Llanowar Elves attacking beside it so that "not tapped" cannot be
/// satisfied by an engine that never taps attackers at all. Flying is read
/// off the block declaration: the opponent's own untapped 1/1 ground
/// creature is offered as a blocker for the ground attacker and still holds
/// no pairing that would put it in front of the Bird (CR 702.9b).
#[allow(clippy::too_many_lines)] // One printed card, played end to end: the length is the card's.
#[test]
fn bay_falcon_flies_over_a_ground_creature_and_attacks_without_tapping() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(4242, island())
        .battlefield(0, &[island(), island(), llanowar_elves()])
        .hand(0, &[bay_falcon()])
        // An untapped ground creature across the table: the blocker flying
        // has to turn away, and the only reason it is on the board at all.
        .battlefield(1, &[llanowar_elves()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let elves = on_battlefield(&engine, p0, llanowar_elves()).expect("my Elves are out");
    let their_elves = on_battlefield(&engine, p1, llanowar_elves()).expect("their Elves are out");
    // Two Islands pay {1}{U}; the Elves is kept back so that this seat has a
    // second attacker next turn, and so that the Bird's mana never came off
    // the very creature the vigilance comparison is about.
    tap_mana_except(&mut engine, p0, elves);
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        2,
        "two Islands, and the Elves is the one source that was kept back"
    );
    cast_with_floating(&mut engine, p0, bay_falcon());
    pass_until(&mut engine, stack_is_empty);

    let falcon = on_battlefield(&engine, p0, bay_falcon()).expect("the Falcon resolved");
    assert_eq!(pt(&engine, falcon), (1, 1), "the body the card prints");
    let printed = keywords(&engine, falcon);
    assert!(printed.contains(KeywordSet::FLYING), "flying");
    assert!(printed.contains(KeywordSet::VIGILANCE), "vigilance");

    // A creature cast this turn has summoning sickness (CR 302.6), so the
    // combat the card is about is the next one this seat takes.
    reach_their_main_phase(&mut engine, p1);
    assert!(
        !is_tapped(&engine, falcon),
        "nothing tapped the Bird on the way around"
    );
    assert!(
        !is_tapped(&engine, their_elves),
        "and the creature across the table is still untapped, ready to be \
         the one flying turns away"
    );
    assert!(
        walk_to_own_main(&mut engine, p0),
        "back around to p0's own main phase"
    );

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    let Pending::ChooseAttackers { attackers, .. } = engine.pending().clone() else {
        unreachable!("the predicate just matched")
    };
    assert!(
        attackers.contains(&falcon) && attackers.contains(&elves),
        "both untapped creatures may attack: {attackers:?}"
    );
    engine
        .apply(
            p0,
            PlayerAction::DeclareAttackers {
                attackers: vec![
                    (falcon, Defender::Player(p1)),
                    (elves, Defender::Player(p1)),
                ],
            },
        )
        .unwrap();

    assert!(
        is_tapped(&engine, elves),
        "attacking taps an ordinary creature (CR 508.1f)"
    );
    assert!(
        !is_tapped(&engine, falcon),
        "and the Falcon is still standing, which is the whole of vigilance \
         (CR 702.14b)"
    );

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseBlockers { .. })
    });
    let Pending::ChooseBlockers {
        player, blockers, ..
    } = engine.pending().clone()
    else {
        unreachable!("the predicate just matched")
    };
    assert_eq!(player, p1, "the defending seat is the one asked");
    assert!(
        blockers
            .iter()
            .any(|option| option.blocker == their_elves && option.attackers.contains(&elves)),
        "the untapped Elf may block the ground attacker, so the refusal \
         below is about flying and not about an empty offer: {blockers:?}"
    );
    assert!(
        !blockers
            .iter()
            .any(|option| option.attackers.contains(&falcon)),
        "and it is offered nothing it could put itself in front of a flier \
         with (CR 702.9b): {blockers:?}"
    );
    engine
        .apply(p1, PlayerAction::DeclareBlockers { blockers: vec![] })
        .unwrap();

    pass_until(&mut engine, |e| {
        matches!(e.state().turn.phase, Phase::Ending)
    });
    assert_eq!(
        engine.state().players[1].life,
        18,
        "two unblocked attackers, past the combat damage step (CR 510.2): \
         the Bird's 1 and the Elf's 1"
    );
}
