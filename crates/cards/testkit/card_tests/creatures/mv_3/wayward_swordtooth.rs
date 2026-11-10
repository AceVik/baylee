//! `cards/creatures/mv_3/wayward_swordtooth.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// `Wayward Swordtooth`: "Ascend (If you control ten or more permanents, you
/// get the city's blessing for the rest of the game.) You may play an
/// additional land on each of your turns. This creature can't attack or
/// block unless you have the city's blessing."
///
/// Two lands on the first turn (the extra drop) make nine permanents: no
/// blessing, so the Dinosaur is offered neither as an attacker on its own
/// turn nor as a blocker against the opponent's Elves. The land drawn on the
/// next turn is the tenth permanent, the blessing arrives the moment it
/// lands (CR 702.131b), and the attack step offers the Dinosaur.
#[allow(clippy::too_many_lines)] // One printed card, played end to end: the length is the card's.
#[test]
fn wayward_swordtooth_attacks_and_blocks_only_with_the_citys_blessing() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                wayward_swordtooth(),
                forest(),
                forest(),
                forest(),
                forest(),
                forest(),
                forest(),
            ],
        )
        .hand(0, &[forest(), forest()])
        .battlefield(1, &[llanowar_elves()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let dino = on_battlefield(&engine, p0, wayward_swordtooth()).expect("swordtooth deployed");
    let elves = on_battlefield(&engine, p1, llanowar_elves()).expect("their Elves");
    let blessed = |e: &Engine<RegistryLookup>| e.state().players[0].citys_blessing;
    assert_eq!(pt(&engine, dino), (5, 5));

    play_land(&mut engine, p0, forest());
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending());
    };
    assert!(!legal.lands.is_empty(), "the additional land drop");
    play_land(&mut engine, p0, forest());
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending());
    };
    assert!(legal.lands.is_empty(), "and no third");
    assert!(!blessed(&engine), "nine permanents are not ten");
    let kw = keywords(&engine, dino);
    assert!(kw.contains(KeywordSet::CANT_ATTACK) && kw.contains(KeywordSet::CANT_BLOCK));

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    let Pending::ChooseAttackers { attackers, .. } = engine.pending().clone() else {
        unreachable!()
    };
    assert!(
        !attackers.contains(&dino),
        "without the blessing it can't attack"
    );
    engine
        .apply(p0, PlayerAction::DeclareAttackers { attackers: vec![] })
        .expect("an empty attack is always legal");

    pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::ChooseAttackers { player, .. } if *player == p1),
    );
    engine
        .apply(
            p1,
            PlayerAction::DeclareAttackers {
                attackers: vec![(elves, baylee_core::ids::Defender::Player(p0))],
            },
        )
        .expect("their Elves attack");
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseBlockers { .. })
    });
    let Pending::ChooseBlockers { blockers, .. } = engine.pending().clone() else {
        unreachable!()
    };
    assert!(
        blockers.iter().all(|b| b.blocker != dino),
        "without the blessing it can't block: {blockers:?}"
    );
    engine
        .apply(p0, PlayerAction::DeclareBlockers { blockers: vec![] })
        .expect("no block is always legal");

    assert!(walk_to_own_main(&mut engine, p0), "p0's next turn comes");
    play_land(&mut engine, p0, forest());
    assert!(
        blessed(&engine),
        "the tenth permanent brings the city's blessing"
    );
    let kw = keywords(&engine, dino);
    assert!(!kw.contains(KeywordSet::CANT_ATTACK) && !kw.contains(KeywordSet::CANT_BLOCK));
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    let Pending::ChooseAttackers { attackers, .. } = engine.pending().clone() else {
        unreachable!()
    };
    assert!(attackers.contains(&dino), "with the blessing it attacks");
}
